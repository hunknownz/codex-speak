import AVFoundation
import AppKit
import Darwin
import Foundation
import QuartzCore

private struct PetState {
    var state: String
    var message: String?
    var updatedAtMs: Double
}

private struct AlphaMask {
    let width: Int
    let height: Int
    let bytesPerRow: Int
    let pixels: [UInt8]

    init?(url: URL) {
        guard let image = NSImage(contentsOf: url),
              let cgImage = Self.cgImage(from: image) else {
            return nil
        }
        let imageWidth = cgImage.width
        let imageHeight = cgImage.height
        let rowBytes = imageWidth * 4
        var buffer = [UInt8](repeating: 0, count: rowBytes * imageHeight)
        let ok = buffer.withUnsafeMutableBytes { pointer -> Bool in
            guard let base = pointer.baseAddress,
                  let context = CGContext(
                      data: base,
                      width: imageWidth,
                      height: imageHeight,
                      bitsPerComponent: 8,
                      bytesPerRow: rowBytes,
                      space: CGColorSpaceCreateDeviceRGB(),
                      bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
                  ) else {
                return false
            }
            context.draw(cgImage, in: CGRect(x: 0, y: 0, width: imageWidth, height: imageHeight))
            return true
        }
        if !ok {
            return nil
        }
        width = imageWidth
        height = imageHeight
        bytesPerRow = rowBytes
        pixels = buffer
    }

    private static func cgImage(from image: NSImage) -> CGImage? {
        var rect = NSRect(origin: .zero, size: image.size)
        return image.cgImage(forProposedRect: &rect, context: nil, hints: nil)
    }

    func alpha(at point: NSPoint, in bounds: NSRect) -> UInt8 {
        let fitted = aspectFitRect(aspectRatio: CGFloat(width) / CGFloat(height), in: bounds)
        guard fitted.contains(point) else { return 0 }
        let unitX = max(0, min(0.999, (point.x - fitted.minX) / fitted.width))
        let unitY = max(0, min(0.999, (point.y - fitted.minY) / fitted.height))
        let x = Int(unitX * CGFloat(width))
        let y = Int(unitY * CGFloat(height))
        let index = y * bytesPerRow + x * 4 + 3
        return pixels.indices.contains(index) ? pixels[index] : 0
    }
}

private func aspectFitRect(aspectRatio: CGFloat, in rect: NSRect) -> NSRect {
    let rectRatio = rect.width / max(rect.height, 1)
    if rectRatio > aspectRatio {
        let width = rect.height * aspectRatio
        return NSRect(x: rect.midX - width / 2, y: rect.minY, width: width, height: rect.height)
    }
    let height = rect.width / max(aspectRatio, 0.01)
    return NSRect(x: rect.minX, y: rect.midY - height / 2, width: rect.width, height: height)
}

private enum WindowAlphaSampler {
    private typealias CreateImageFn = @convention(c) (CGRect, UInt32, CGWindowID, UInt32) -> Unmanaged<CGImage>?

    private static let createImage: CreateImageFn? = {
        guard let handle = dlopen("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics", RTLD_LAZY),
              let symbol = dlsym(handle, "CGWindowListCreateImage") else {
            return nil
        }
        return unsafeBitCast(symbol, to: CreateImageFn.self)
    }()

    static func image(captureRect: CGRect, windowID: CGWindowID) -> CGImage? {
        guard let createImage else { return nil }
        let listOption = CGWindowListOption.optionIncludingWindow.rawValue
        let imageOption = CGWindowImageOption.boundsIgnoreFraming.rawValue
            | CGWindowImageOption.bestResolution.rawValue
        return createImage(captureRect, listOption, windowID, imageOption)?.takeRetainedValue()
    }
}

private final class PetController: NSObject, NSApplicationDelegate {
    private let statePath: String
    private let cliPath: String
    private let assetDir: String
    private let parentPid: Int32?
    private let lockPath: String

    private var window: NSWindow!
    private var bubbleWindow: NSWindow?
    private var petView: PetView!
    private var displayLink: CADisplayLink?
    private var fallbackMovementTimer: Timer?
    private var state = PetState(state: "idle", message: nil, updatedAtMs: 0)

    private var isWalking = false
    private var goingRight = true
    private var pauseUntil = CACurrentMediaTime() + 1.2
    private var walkStartTime: CFTimeInterval = 0
    private var walkDuration: CFTimeInterval = 9.2
    private var walkStartProgress: CGFloat = 0.78
    private var walkEndProgress: CGFloat = 0.78
    private var positionProgress: CGFloat = 0.78
    private var manualHoldUntil: CFTimeInterval = 0
    private let videoDuration: CFTimeInterval = 10.0
    private let accelStart: CFTimeInterval = 3.0
    private let fullSpeedStart: CFTimeInterval = 3.75
    private let decelStart: CFTimeInterval = 8.0
    private let walkStop: CFTimeInterval = 8.5

    init(arguments: [String]) {
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        statePath = Self.value(after: "--state", in: arguments)
            ?? "\(home)/.codex/codex-speak/state/pet-state.json"
        cliPath = Self.value(after: "--cli", in: arguments)
            ?? "\(home)/.codex/codex-speak/bin/codex-speak"
        assetDir = Self.value(after: "--asset-dir", in: arguments)
            ?? "\(home)/.codex/codex-speak/assets/pet"
        parentPid = Self.value(after: "--parent", in: arguments).flatMap { Int32($0) }
        lockPath = "\(home)/.codex/codex-speak/state/pet-helper.pid"
        super.init()
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        guard acquireLock() else {
            NSApp.terminate(nil)
            return
        }
        createWindow()
        refreshState()
        Timer.scheduledTimer(withTimeInterval: 0.7, repeats: true) { [weak self] _ in
            self?.refreshState()
        }
        startDisplayLink()
    }

    func applicationWillTerminate(_ notification: Notification) {
        displayLink?.invalidate()
        fallbackMovementTimer?.invalidate()
        try? FileManager.default.removeItem(atPath: lockPath)
    }

    private static func value(after flag: String, in arguments: [String]) -> String? {
        guard let index = arguments.firstIndex(of: flag), arguments.indices.contains(index + 1) else {
            return nil
        }
        return arguments[index + 1]
    }

    private func acquireLock() -> Bool {
        if let raw = try? String(contentsOfFile: lockPath, encoding: .utf8),
           let pid = Int32(raw.trimmingCharacters(in: .whitespacesAndNewlines)),
           pid != getpid(),
           isProcessAlive(pid) {
            return false
        }
        try? FileManager.default.createDirectory(
            atPath: (lockPath as NSString).deletingLastPathComponent,
            withIntermediateDirectories: true
        )
        try? "\(getpid())\n".write(toFile: lockPath, atomically: true, encoding: .utf8)
        return true
    }

    private func isProcessAlive(_ pid: Int32) -> Bool {
        kill(pid, 0) == 0 || errno == EPERM
    }

    private func startDisplayLink() {
        if #available(macOS 14.0, *) {
            let link = window.displayLink(target: self, selector: #selector(displayLinkTick(_:)))
            link.add(to: .main, forMode: .common)
            displayLink = link
        } else {
            fallbackMovementTimer = Timer.scheduledTimer(withTimeInterval: 1.0 / 30.0, repeats: true) { [weak self] _ in
                self?.tick()
            }
        }
    }

    @objc private func displayLinkTick(_ sender: CADisplayLink) {
        tick()
    }

    private func createWindow() {
        let size = PetView.preferredSize
        let frame = initialFrame(size: size)
        window = NSWindow(
            contentRect: frame,
            styleMask: [.borderless],
            backing: .buffered,
            defer: false
        )
        window.isOpaque = false
        window.backgroundColor = .clear
        window.hasShadow = false
        window.level = .statusBar
        window.ignoresMouseEvents = false
        window.collectionBehavior = [.moveToActiveSpace, .stationary]
        window.animationBehavior = .none

        petView = PetView(frame: NSRect(origin: .zero, size: size), assetDir: assetDir)
        petView.controller = self
        window.contentView = petView
        window.orderFrontRegardless()
    }

    private func initialFrame(size: NSSize) -> NSRect {
        let screen = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
        let y = screen.minY - size.height * 0.15
        return NSRect(x: screen.maxX - size.width - 24, y: y, width: size.width, height: size.height)
    }

    private func refreshState() {
        state = effectiveState(readState())
        petView.setState(state.state)
        renderBubble()
    }

    private func readState() -> PetState {
        guard let data = try? Data(contentsOf: URL(fileURLWithPath: statePath)),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            return PetState(state: "idle", message: nil, updatedAtMs: 0)
        }
        return PetState(
            state: object["state"] as? String ?? "idle",
            message: object["message"] as? String,
            updatedAtMs: object["updated_at_ms"] as? Double ?? 0
        )
    }

    private func effectiveState(_ raw: PetState) -> PetState {
        let age = Date().timeIntervalSince1970 * 1000 - raw.updatedAtMs
        if raw.state == "done" && age > 4500 {
            return PetState(state: "idle", message: nil, updatedAtMs: raw.updatedAtMs)
        }
        if raw.state == "ready" && age > 18000 {
            return PetState(state: "idle", message: nil, updatedAtMs: raw.updatedAtMs)
        }
        if raw.state == "error" && age > 9000 {
            return PetState(state: "idle", message: nil, updatedAtMs: raw.updatedAtMs)
        }
        return raw
    }

    private func tick() {
        checkParent()
        updateMovement()
        updateBubblePosition()
    }

    private func checkParent() {
        guard let parentPid else { return }
        if !isProcessAlive(parentPid) {
            NSApp.terminate(nil)
        }
    }

    private func updateMovement() {
        guard let window else { return }
        let now = CACurrentMediaTime()
        if now < manualHoldUntil {
            petView.setWalking(false, facingRight: goingRight)
            return
        }

        if !isWalking {
            if now >= pauseUntil {
                startWalk()
            } else {
                petView.setWalking(false, facingRight: goingRight)
                return
            }
        }

        let elapsed = now - walkStartTime
        if elapsed >= videoDuration {
            positionProgress = walkEndProgress
            enterPause()
            return
        }

        let progress = movementPosition(atVideoTime: elapsed)
        positionProgress = walkStartProgress + (walkEndProgress - walkStartProgress) * progress
        let frame = frameForProgress(positionProgress, size: window.frame.size)
        window.setFrameOrigin(frame.origin)
        petView.setWalking(true, facingRight: goingRight)
    }

    private func startWalk() {
        syncProgressFromWindow()
        isWalking = true
        walkStartTime = CACurrentMediaTime()
        walkDuration = videoDuration
        if positionProgress < 0.12 {
            goingRight = true
        } else if positionProgress > 0.88 {
            goingRight = false
        } else {
            goingRight = Bool.random()
        }
        walkStartProgress = positionProgress
        let distance = CGFloat.random(in: 0.12...0.28)
        walkEndProgress = goingRight
            ? min(0.98, walkStartProgress + distance)
            : max(0.02, walkStartProgress - distance)
        petView.setWalking(true, facingRight: goingRight)
    }

    private func enterPause() {
        isWalking = false
        pauseUntil = CACurrentMediaTime() + Double.random(in: 3.5...8.0)
        petView.setWalking(false, facingRight: goingRight)
    }

    private func movementPosition(atVideoTime videoTime: CFTimeInterval) -> CGFloat {
        let dIn = fullSpeedStart - accelStart
        let dLinear = decelStart - fullSpeedStart
        let dOut = walkStop - decelStart
        let velocity = 1.0 / (dIn / 2.0 + dLinear + dOut / 2.0)

        if videoTime <= accelStart {
            return 0.0
        } else if videoTime <= fullSpeedStart {
            let t = videoTime - accelStart
            return CGFloat(velocity * t * t / (2.0 * dIn))
        } else if videoTime <= decelStart {
            let easeInDistance = velocity * dIn / 2.0
            let t = videoTime - fullSpeedStart
            return CGFloat(easeInDistance + velocity * t)
        } else if videoTime <= walkStop {
            let easeInDistance = velocity * dIn / 2.0
            let linearDistance = velocity * dLinear
            let t = videoTime - decelStart
            return CGFloat(easeInDistance + linearDistance + velocity * (t - t * t / (2.0 * dOut)))
        }

        return 1.0
    }

    private func frameForProgress(_ progress: CGFloat, size: NSSize) -> NSRect {
        let screen = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
        let minX = screen.minX + 16
        let maxX = max(minX, screen.maxX - size.width - 16)
        let x = minX + (maxX - minX) * max(0, min(1, progress))
        let y = screen.minY - size.height * 0.15
        return NSRect(x: x, y: y, width: size.width, height: size.height)
    }

    private func syncProgressFromWindow() {
        guard let window else { return }
        let screen = NSScreen.main?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1440, height: 900)
        let minX = screen.minX + 16
        let maxX = max(minX, screen.maxX - window.frame.width - 16)
        let travel = max(maxX - minX, 1)
        positionProgress = max(0, min(1, (window.frame.minX - minX) / travel))
    }

    private func bubbleText() -> String? {
        switch state.state {
        case "ready":
            return "整理好了"
        case "speaking":
            return "正在朗读"
        case "done":
            return "完成"
        case "error":
            return "朗读出错"
        default:
            return nil
        }
    }

    private func renderBubble() {
        guard let text = bubbleText() else {
            bubbleWindow?.orderOut(nil)
            return
        }
        if bubbleWindow == nil {
            bubbleWindow = createBubbleWindow()
        }
        let bubbleView = bubbleWindow?.contentView as? BubbleView
        bubbleView?.text = text
        bubbleView?.isError = state.state == "error"
        bubbleView?.needsDisplay = true
        updateBubblePosition()
        bubbleWindow?.orderFrontRegardless()
    }

    private func createBubbleWindow() -> NSWindow {
        let win = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 94, height: 30),
            styleMask: [.borderless],
            backing: .buffered,
            defer: false
        )
        win.isOpaque = false
        win.backgroundColor = .clear
        win.hasShadow = true
        win.level = NSWindow.Level(rawValue: NSWindow.Level.statusBar.rawValue + 5)
        win.ignoresMouseEvents = true
        win.collectionBehavior = [.moveToActiveSpace, .stationary]
        win.contentView = BubbleView(frame: NSRect(x: 0, y: 0, width: 94, height: 30))
        return win
    }

    private func updateBubblePosition() {
        guard let bubbleWindow, let bubbleView = bubbleWindow.contentView as? BubbleView else { return }
        let size = bubbleView.fittingSize()
        let charFrame = window.frame
        bubbleWindow.setFrame(
            NSRect(x: charFrame.midX - size.width / 2, y: charFrame.maxY - 30, width: size.width, height: size.height),
            display: false
        )
    }

    func holdManualMovement(for seconds: CFTimeInterval = 8) {
        manualHoldUntil = CACurrentMediaTime() + seconds
        isWalking = false
        syncProgressFromWindow()
    }

    func stopSpeech() {
        _ = Process.launchedProcess(launchPath: cliPath, arguments: ["stop"])
    }

    func openControlApp() {
        _ = Process.launchedProcess(launchPath: cliPath, arguments: ["app", "open"])
    }

    func currentState() -> String {
        state.state
    }

    func petMoved() {
        holdManualMovement()
        updateBubblePosition()
    }
}

private final class BubbleView: NSView {
    var text: String = ""
    var isError = false

    override var isOpaque: Bool { false }

    func fittingSize() -> NSSize {
        let font = NSFont.systemFont(ofSize: 13, weight: .semibold)
        let textSize = (text as NSString).size(withAttributes: [.font: font])
        return NSSize(width: max(64, ceil(textSize.width) + 24), height: 30)
    }

    override func draw(_ dirtyRect: NSRect) {
        NSColor.clear.setFill()
        dirtyRect.fill()
        let rect = bounds.insetBy(dx: 1, dy: 1)
        let path = NSBezierPath(roundedRect: rect, xRadius: 15, yRadius: 15)
        (isError ? NSColor(calibratedRed: 1, green: 0.91, blue: 0.9, alpha: 0.94) : NSColor(calibratedWhite: 1, alpha: 0.92)).setFill()
        path.fill()
        NSColor(calibratedWhite: 0.1, alpha: 0.2).setStroke()
        path.lineWidth = 1
        path.stroke()

        let font = NSFont.systemFont(ofSize: 13, weight: .semibold)
        let attrs: [NSAttributedString.Key: Any] = [
            .font: font,
            .foregroundColor: NSColor(calibratedRed: 0.08, green: 0.09, blue: 0.13, alpha: 0.92)
        ]
        let size = (text as NSString).size(withAttributes: attrs)
        (text as NSString).draw(
            at: NSPoint(x: (bounds.width - size.width) / 2, y: (bounds.height - size.height) / 2 - 1),
            withAttributes: attrs
        )
    }
}

private final class PetView: NSView {
    private static let videoWidth: CGFloat = 1080
    private static let videoHeight: CGFloat = 1920
    private static let displayHeight: CGFloat = 200
    static let preferredSize = NSSize(
        width: round(displayHeight * (videoWidth / videoHeight)),
        height: displayHeight
    )

    weak var controller: PetController?

    private var playerLayer: AVPlayerLayer?
    private var queuePlayer: AVQueuePlayer?
    private var looper: AVPlayerLooper?
    private var hitMask: AlphaMask?
    private var state = "idle"
    private var isWalking = false
    private var facingRight = true
    private var dragStartScreen: NSPoint?
    private var dragStartOrigin: NSPoint?

    init(frame frameRect: NSRect, assetDir: String) {
        super.init(frame: frameRect)
        wantsLayer = true
        layer?.backgroundColor = NSColor.clear.cgColor
        loadVisual(from: assetDir)
        syncLayout()
        syncPlayback()
    }

    required init?(coder: NSCoder) {
        nil
    }

    override var isOpaque: Bool { false }

    override func layout() {
        super.layout()
        syncLayout()
    }

    private func syncLayout() {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        playerLayer?.frame = bounds
        CATransaction.commit()
    }

    private func loadVisual(from assetDir: String) {
        let dir = URL(fileURLWithPath: assetDir, isDirectory: true)
        if let url = firstExistingURL(in: dir, names: ["codex-agent.mov", "codex-agent.hevc.mov"]) {
            let asset = AVURLAsset(url: url)
            let player = AVQueuePlayer()
            let item = AVPlayerItem(asset: asset)
            looper = AVPlayerLooper(player: player, templateItem: item)
            queuePlayer = player
            let videoLayer = AVPlayerLayer(player: player)
            videoLayer.videoGravity = .resizeAspect
            videoLayer.backgroundColor = NSColor.clear.cgColor
            videoLayer.frame = bounds
            layer?.addSublayer(videoLayer)
            playerLayer = videoLayer
            loadHitMask(from: dir)
            return
        }

        if let url = firstExistingURL(in: dir, names: ["codex-agent.png", "codex-agent@2x.png"]) ,
           let image = NSImage(contentsOf: url),
           let cgImage = cgImage(from: image) {
            layer?.contentsGravity = .resizeAspect
            layer?.contents = cgImage
            loadHitMask(from: dir)
            return
        }

        layer?.contentsGravity = .resizeAspect
        layer?.contents = cgImage(from: renderFallbackImage())
    }

    private func loadHitMask(from dir: URL) {
        if let url = firstExistingURL(in: dir, names: ["codex-agent-hit.png", "codex-agent.png", "codex-agent@2x.png"]) {
            hitMask = AlphaMask(url: url)
        }
    }

    private func firstExistingURL(in dir: URL, names: [String]) -> URL? {
        for name in names {
            let url = dir.appendingPathComponent(name)
            if FileManager.default.fileExists(atPath: url.path) {
                return url
            }
        }
        return nil
    }

    private func cgImage(from image: NSImage) -> CGImage? {
        var rect = NSRect(origin: .zero, size: image.size)
        return image.cgImage(forProposedRect: &rect, context: nil, hints: nil)
    }

    func setState(_ nextState: String) {
        guard nextState != state else { return }
        state = nextState
        configureMotion()
        syncPlayback()
    }

    func setWalking(_ walking: Bool, facingRight nextFacingRight: Bool) {
        guard walking != isWalking || nextFacingRight != facingRight else {
            syncPlayback()
            return
        }
        isWalking = walking
        facingRight = nextFacingRight
        configureFlip()
        configureMotion()
        if walking {
            queuePlayer?.seek(to: .zero)
        }
        syncPlayback()
    }

    private func syncPlayback() {
        let shouldPlay = isWalking || state == "speaking" || state == "ready"
        if shouldPlay {
            queuePlayer?.play()
        } else {
            queuePlayer?.pause()
            queuePlayer?.seek(to: .zero)
        }
    }

    private func configureFlip() {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        let scaleX: CGFloat = facingRight ? 1 : -1
        playerLayer?.transform = CATransform3DMakeScale(scaleX, 1, 1)
        playerLayer?.frame = bounds
        CATransaction.commit()
    }

    private func configureMotion() {
        layer?.removeAllAnimations()
        switch state {
        case "speaking":
            addBobAnimation(distance: 5, duration: 0.45)
        case "ready":
            addBobAnimation(distance: 4, duration: 0.7)
        case "done":
            addBobAnimation(distance: 3, duration: 0.8)
        case "error":
            let shake = CABasicAnimation(keyPath: "transform.translation.x")
            shake.fromValue = -5
            shake.toValue = 5
            shake.duration = 0.08
            shake.autoreverses = true
            shake.repeatCount = 6
            layer?.add(shake, forKey: "shake")
        default:
            if !isWalking {
                addBobAnimation(distance: 2, duration: 2.8)
            }
        }
    }

    private func addBobAnimation(distance: CGFloat, duration: CFTimeInterval) {
        let bob = CABasicAnimation(keyPath: "transform.translation.y")
        bob.fromValue = 0
        bob.toValue = distance
        bob.duration = duration
        bob.autoreverses = true
        bob.repeatCount = .infinity
        bob.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
        layer?.add(bob, forKey: "bob")
    }

    override func hitTest(_ point: NSPoint) -> NSView? {
        let localPoint = convert(point, from: superview)
        guard bounds.contains(localPoint) else { return nil }

        switch visiblePixelHit(at: localPoint) {
        case .visible:
            return self
        case .transparent:
            return nil
        case .unavailable:
            break
        }

        if hitMask?.alpha(at: localPoint, in: bounds) ?? 0 > 30 {
            return self
        }

        let fallbackHitRect = bounds.insetBy(dx: bounds.width * 0.2, dy: bounds.height * 0.15)
        return fallbackHitRect.contains(localPoint) ? self : nil
    }

    private enum PixelHit {
        case visible
        case transparent
        case unavailable
    }

    private func visiblePixelHit(at localPoint: NSPoint) -> PixelHit {
        guard let window,
              let primaryScreen = NSScreen.screens.first,
              window.windowNumber > 0 else {
            return .unavailable
        }

        let windowPoint = convert(localPoint, to: nil)
        let screenPoint = window.convertPoint(toScreen: windowPoint)
        let flippedY = primaryScreen.frame.height - screenPoint.y
        let captureRect = CGRect(x: screenPoint.x - 0.5, y: flippedY - 0.5, width: 1, height: 1)

        guard let image = WindowAlphaSampler.image(
            captureRect: captureRect,
            windowID: CGWindowID(window.windowNumber)
        ) else {
            return .unavailable
        }

        var pixel: [UInt8] = [0, 0, 0, 0]
        guard let context = CGContext(
            data: &pixel,
            width: 1,
            height: 1,
            bitsPerComponent: 8,
            bytesPerRow: 4,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        ) else {
            return .unavailable
        }

        context.draw(image, in: CGRect(x: 0, y: 0, width: 1, height: 1))
        return pixel[3] > 30 ? .visible : .transparent
    }

    override func mouseDown(with event: NSEvent) {
        controller?.holdManualMovement(for: 12)
        dragStartScreen = window?.convertPoint(toScreen: event.locationInWindow)
        dragStartOrigin = window?.frame.origin
        if event.clickCount >= 2 {
            controller?.openControlApp()
        } else if controller?.currentState() == "speaking" {
            controller?.stopSpeech()
        }
    }

    override func mouseDragged(with event: NSEvent) {
        guard let window, let dragStartScreen, let dragStartOrigin else { return }
        let current = window.convertPoint(toScreen: event.locationInWindow)
        window.setFrameOrigin(NSPoint(
            x: dragStartOrigin.x + current.x - dragStartScreen.x,
            y: dragStartOrigin.y + current.y - dragStartScreen.y
        ))
        controller?.petMoved()
    }

    private func renderFallbackImage() -> NSImage {
        let image = NSImage(size: Self.preferredSize)
        image.lockFocus()
        NSColor.clear.setFill()
        NSRect(origin: .zero, size: Self.preferredSize).fill()

        let body = NSBezierPath(roundedRect: NSRect(x: 20, y: 34, width: 78, height: 126), xRadius: 28, yRadius: 28)
        NSColor(calibratedRed: 0.33, green: 0.78, blue: 0.96, alpha: 1).setFill()
        body.fill()

        let face = NSBezierPath(roundedRect: NSRect(x: 28, y: 106, width: 62, height: 35), xRadius: 16, yRadius: 16)
        NSColor(calibratedRed: 0.06, green: 0.08, blue: 0.17, alpha: 1).setFill()
        face.fill()

        NSColor.white.setFill()
        NSBezierPath(ovalIn: NSRect(x: 42, y: 119, width: 8, height: 8)).fill()
        NSBezierPath(ovalIn: NSRect(x: 68, y: 119, width: 8, height: 8)).fill()

        NSColor(calibratedRed: 0.13, green: 0.19, blue: 0.36, alpha: 1).setFill()
        NSBezierPath(roundedRect: NSRect(x: 26, y: 18, width: 24, height: 18), xRadius: 7, yRadius: 7).fill()
        NSBezierPath(roundedRect: NSRect(x: 66, y: 18, width: 24, height: 18), xRadius: 7, yRadius: 7).fill()

        image.unlockFocus()
        return image
    }
}

private let delegate = PetController(arguments: CommandLine.arguments)
private let app = NSApplication.shared
app.delegate = delegate
app.run()
