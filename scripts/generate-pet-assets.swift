import AVFoundation
import CoreGraphics
import Foundation
import ImageIO

let outputDir = URL(fileURLWithPath: CommandLine.arguments.dropFirst().first ?? "apps/codex-speak-pet-macos/assets", isDirectory: true)
try FileManager.default.createDirectory(at: outputDir, withIntermediateDirectories: true)

let width = 1080
let height = 1920
let logicalWidth: CGFloat = 288
let logicalHeight: CGFloat = 512
let fps: Int32 = 24
let frameCount = 241
let colorSpace = CGColorSpaceCreateDeviceRGB()
let bitmapInfo = CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue)

let movieURL = outputDir.appendingPathComponent("codex-agent.mov")
let hitURL = outputDir.appendingPathComponent("codex-agent-hit.png")
let previewURL = outputDir.appendingPathComponent("codex-agent-preview.png")
let noticeURL = outputDir.appendingPathComponent("ASSET-NOTICE.txt")

try? FileManager.default.removeItem(at: movieURL)
try? FileManager.default.removeItem(at: hitURL)
try? FileManager.default.removeItem(at: previewURL)
try? FileManager.default.removeItem(at: noticeURL)

try writeMovie(to: movieURL)
try writeHitMask(to: hitURL)
try writePreview(to: previewURL)
try """
Codex Speak Pet Assets

These assets are generated from scripts/generate-pet-assets.swift for the Codex Speak project.
They are original project assets and are intended for distribution with Codex Speak.

The display pipeline intentionally matches the lil-agents style of desktop companion:
- 1080x1920 HEVC-with-alpha .mov.
- Native transparent AppKit window.
- AVPlayerLayer playback on macOS.
- A small walking character designed as a transparent video sprite, not a WebView/canvas.

Files:
- codex-agent.mov: transparent animated desktop companion.
- codex-agent-hit.png: alpha mask fallback used by the native macOS helper.
- codex-agent-preview.png: transparent still frame for quick visual QA.
""".write(to: noticeURL, atomically: true, encoding: .utf8)

print("Generated pet assets in \(outputDir.path)")

func writeMovie(to url: URL) throws {
    let writer = try AVAssetWriter(outputURL: url, fileType: .mov)
    let outputSettings: [String: Any] = [
        AVVideoCodecKey: AVVideoCodecType.hevcWithAlpha,
        AVVideoWidthKey: width,
        AVVideoHeightKey: height
    ]
    let input = AVAssetWriterInput(mediaType: .video, outputSettings: outputSettings)
    input.expectsMediaDataInRealTime = false
    let adaptor = AVAssetWriterInputPixelBufferAdaptor(
        assetWriterInput: input,
        sourcePixelBufferAttributes: [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
            kCVPixelBufferWidthKey as String: width,
            kCVPixelBufferHeightKey as String: height,
            kCVPixelBufferCGImageCompatibilityKey as String: true,
            kCVPixelBufferCGBitmapContextCompatibilityKey as String: true
        ]
    )

    guard writer.canAdd(input) else {
        throw AssetError.message("AVAssetWriter cannot add video input")
    }
    writer.add(input)
    writer.startWriting()
    writer.startSession(atSourceTime: .zero)

    for frame in 0..<frameCount {
        while !input.isReadyForMoreMediaData {
            Thread.sleep(forTimeInterval: 0.01)
        }
        let buffer = try makeFrame(frame: frame, maskOnly: false)
        let time = CMTime(value: CMTimeValue(frame), timescale: fps)
        guard adaptor.append(buffer, withPresentationTime: time) else {
            throw AssetError.message("Failed to append frame \(frame)")
        }
    }

    input.markAsFinished()
    let sema = DispatchSemaphore(value: 0)
    writer.finishWriting {
        sema.signal()
    }
    sema.wait()
    if writer.status != .completed {
        throw writer.error ?? AssetError.message("AVAssetWriter finished with status \(writer.status.rawValue)")
    }
}

func writeHitMask(to url: URL) throws {
    let buffer = try makeFrame(frame: 72, maskOnly: true)
    guard let image = cgImage(from: buffer) else {
        throw AssetError.message("Could not create CGImage for hit mask")
    }
    guard let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil) else {
        throw AssetError.message("Could not create PNG destination")
    }
    CGImageDestinationAddImage(dest, image, nil)
    if !CGImageDestinationFinalize(dest) {
        throw AssetError.message("Could not write \(url.path)")
    }
}

func writePreview(to url: URL) throws {
    let buffer = try makeFrame(frame: 96, maskOnly: false)
    guard let image = cgImage(from: buffer) else {
        throw AssetError.message("Could not create CGImage for preview")
    }
    guard let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil) else {
        throw AssetError.message("Could not create PNG destination")
    }
    CGImageDestinationAddImage(dest, image, nil)
    if !CGImageDestinationFinalize(dest) {
        throw AssetError.message("Could not write \(url.path)")
    }
}

func makeFrame(frame: Int, maskOnly: Bool) throws -> CVPixelBuffer {
    var pixelBuffer: CVPixelBuffer?
    let status = CVPixelBufferCreate(
        kCFAllocatorDefault,
        width,
        height,
        kCVPixelFormatType_32BGRA,
        [
            kCVPixelBufferCGImageCompatibilityKey as String: true,
            kCVPixelBufferCGBitmapContextCompatibilityKey as String: true
        ] as CFDictionary,
        &pixelBuffer
    )
    guard status == kCVReturnSuccess, let buffer = pixelBuffer else {
        throw AssetError.message("Could not allocate pixel buffer")
    }

    CVPixelBufferLockBaseAddress(buffer, [])
    defer { CVPixelBufferUnlockBaseAddress(buffer, []) }
    guard let base = CVPixelBufferGetBaseAddress(buffer) else {
        throw AssetError.message("Could not lock pixel buffer")
    }
    let bytesPerRow = CVPixelBufferGetBytesPerRow(buffer)
    memset(base, 0, bytesPerRow * height)
    guard let context = CGContext(
        data: base,
        width: width,
        height: height,
        bitsPerComponent: 8,
        bytesPerRow: bytesPerRow,
        space: colorSpace,
        bitmapInfo: bitmapInfo.rawValue
    ) else {
        throw AssetError.message("Could not create drawing context")
    }

    drawCodexAgent(in: context, frame: frame, maskOnly: maskOnly)
    return buffer
}

func cgImage(from buffer: CVPixelBuffer) -> CGImage? {
    CVPixelBufferLockBaseAddress(buffer, .readOnly)
    defer { CVPixelBufferUnlockBaseAddress(buffer, .readOnly) }
    guard let base = CVPixelBufferGetBaseAddress(buffer) else { return nil }
    let bytesPerRow = CVPixelBufferGetBytesPerRow(buffer)
    let data = Data(bytes: base, count: bytesPerRow * height)
    guard let provider = CGDataProvider(data: data as CFData) else { return nil }
    return CGImage(
        width: width,
        height: height,
        bitsPerComponent: 8,
        bitsPerPixel: 32,
        bytesPerRow: bytesPerRow,
        space: colorSpace,
        bitmapInfo: bitmapInfo,
        provider: provider,
        decode: nil,
        shouldInterpolate: true,
        intent: .defaultIntent
    )
}

func drawCodexAgent(in ctx: CGContext, frame: Int, maskOnly: Bool) {
    let t = CGFloat(frame) / CGFloat(frameCount - 1)
    let cycle = t * .pi * 2
    let walk = sin(cycle)
    let counterWalk = sin(cycle + .pi)
    let bob = sin(cycle * 2) * 5
    let lean = sin(cycle) * 0.035

    ctx.clear(CGRect(x: 0, y: 0, width: width, height: height))
    ctx.saveGState()
    ctx.scaleBy(x: CGFloat(width) / logicalWidth, y: CGFloat(height) / logicalHeight)
    ctx.translateBy(x: 144, y: 32 + bob)
    ctx.rotate(by: lean)

    if maskOnly {
        drawAgentMask(in: ctx, walk: walk, counterWalk: counterWalk)
    } else {
        drawAgentArt(in: ctx, walk: walk, counterWalk: counterWalk, t: t)
    }

    ctx.restoreGState()
}

func drawAgentMask(in ctx: CGContext, walk: CGFloat, counterWalk: CGFloat) {
    let mask = cg(0xffffff, 1)
    fillRounded(ctx, CGRect(x: -78, y: 205, width: 56, height: 96), radius: 22, color: mask)
    strokeLine(ctx, [CGPoint(x: -28, y: 262), CGPoint(x: -58, y: 212 + walk * 8), CGPoint(x: -50, y: 172 + walk * 5)], color: mask, width: 24)
    strokeLine(ctx, [CGPoint(x: 37, y: 260), CGPoint(x: 63, y: 208 + counterWalk * 8), CGPoint(x: 58, y: 170 + counterWalk * 5)], color: mask, width: 24)
    strokeLine(ctx, [CGPoint(x: -18, y: 188), CGPoint(x: -43 + walk * 16, y: 112 + walk * 8), CGPoint(x: -42 + walk * 20, y: 68)], color: mask, width: 29)
    strokeLine(ctx, [CGPoint(x: 25, y: 188), CGPoint(x: 43 + counterWalk * 15, y: 113 + counterWalk * 8), CGPoint(x: 51 + counterWalk * 20, y: 69)], color: mask, width: 29)
    fillRounded(ctx, CGRect(x: -74 + walk * 20, y: 42, width: 72, height: 32), radius: 12, color: mask)
    fillRounded(ctx, CGRect(x: 7 + counterWalk * 20, y: 42, width: 74, height: 32), radius: 12, color: mask)
    fillRounded(ctx, CGRect(x: -52, y: 176, width: 108, height: 148), radius: 28, color: mask)
    fillRounded(ctx, CGRect(x: -11, y: 302, width: 28, height: 35), radius: 11, color: mask)
    fillRounded(ctx, CGRect(x: -35, y: 327, width: 94, height: 72), radius: 29, color: mask)
    fillRounded(ctx, CGRect(x: -3, y: 348, width: 58, height: 31), radius: 14, color: mask)
    strokeLine(ctx, [CGPoint(x: -4, y: 396), CGPoint(x: -16, y: 421)], color: mask, width: 7)
    strokeLine(ctx, [CGPoint(x: 33, y: 394), CGPoint(x: 52, y: 414)], color: mask, width: 7)
    fillEllipse(ctx, CGRect(x: -14, y: 411, width: 18, height: 18), color: mask)
}

func drawAgentArt(in ctx: CGContext, walk: CGFloat, counterWalk: CGFloat, t: CGFloat) {
    let ink = cg(0x101522)
    let visor = cg(0x07111f)
    let jacket = cg(0x4ac79b)
    let jacketDark = cg(0x1c8f78)
    let blue = cg(0x4a9be8)
    let orange = cg(0xff6a32)
    let cream = cg(0xfff4ef)
    let skin = cg(0xffb7d0)
    let sole = cg(0xf8fbff)
    let glow = cg(0x63e6ff)
    let white = cg(0xffffff)

    fillEllipse(ctx, CGRect(x: -67, y: 38, width: 134, height: 17), color: cg(0x000000, 0.09))

    fillRounded(ctx, CGRect(x: -79, y: 207, width: 56, height: 95), radius: 22, color: orange)
    fillRounded(ctx, CGRect(x: -73, y: 219, width: 43, height: 68), radius: 17, color: cg(0x1a2233, 0.92))
    fillRounded(ctx, CGRect(x: -67, y: 238, width: 30, height: 11), radius: 6, color: glow)

    strokeLine(ctx, [CGPoint(x: 32, y: 266), CGPoint(x: 64, y: 212 + counterWalk * 8), CGPoint(x: 58, y: 172 + counterWalk * 5)], color: blue, width: 23)
    fillRounded(ctx, CGRect(x: 44, y: 160 + counterWalk * 5, width: 27, height: 25), radius: 11, color: ink)
    strokeLine(ctx, [CGPoint(x: 23, y: 190), CGPoint(x: 43 + counterWalk * 15, y: 112 + counterWalk * 8), CGPoint(x: 51 + counterWalk * 20, y: 68)], color: ink, width: 27)
    fillRounded(ctx, CGRect(x: 6 + counterWalk * 20, y: 41, width: 74, height: 32), radius: 12, color: sole)
    fillRounded(ctx, CGRect(x: 15 + counterWalk * 20, y: 49, width: 57, height: 13), radius: 7, color: blue)

    strokeLine(ctx, [CGPoint(x: -20, y: 190), CGPoint(x: -44 + walk * 16, y: 112 + walk * 8), CGPoint(x: -43 + walk * 20, y: 68)], color: cream, width: 28)
    fillRounded(ctx, CGRect(x: -75 + walk * 20, y: 41, width: 74, height: 32), radius: 12, color: sole)
    fillRounded(ctx, CGRect(x: -67 + walk * 20, y: 49, width: 58, height: 13), radius: 7, color: orange)

    strokeLine(ctx, [CGPoint(x: -29, y: 266), CGPoint(x: -58, y: 212 + walk * 8), CGPoint(x: -50, y: 173 + walk * 5)], color: jacketDark, width: 23)
    fillRounded(ctx, CGRect(x: -64, y: 161 + walk * 5, width: 27, height: 25), radius: 11, color: skin)

    fillRounded(ctx, CGRect(x: -52, y: 176, width: 108, height: 148), radius: 28, color: jacket)
    fillRounded(ctx, CGRect(x: -42, y: 185, width: 38, height: 128), radius: 20, color: jacketDark)
    fillRounded(ctx, CGRect(x: -3, y: 186, width: 53, height: 126), radius: 23, color: ink)
    fillRounded(ctx, CGRect(x: -4, y: 201, width: 41, height: 91), radius: 18, color: cream)
    fillRounded(ctx, CGRect(x: 1, y: 221, width: 34, height: 15), radius: 8, color: cg(0xbdf8e8))
    strokeLine(ctx, [CGPoint(x: -42, y: 289), CGPoint(x: -20, y: 262)], color: cg(0xffffff, 0.26), width: 4)
    strokeLine(ctx, [CGPoint(x: 43, y: 286), CGPoint(x: 21, y: 260)], color: cg(0xffffff, 0.2), width: 4)
    fillEllipse(ctx, CGRect(x: 9, y: 245, width: 14, height: 14), color: orange)

    fillRounded(ctx, CGRect(x: -11, y: 305, width: 28, height: 35), radius: 10, color: skin)
    fillRounded(ctx, CGRect(x: -35, y: 329, width: 94, height: 70), radius: 29, color: skin)
    fillRounded(ctx, CGRect(x: -30, y: 366, width: 80, height: 22), radius: 12, color: ink)
    fillRounded(ctx, CGRect(x: -3, y: 347, width: 58, height: 32), radius: 15, color: visor)
    fillEllipse(ctx, CGRect(x: 31, y: 356, width: 12, height: 12), color: glow)
    fillEllipse(ctx, CGRect(x: 35, y: 361, width: 4, height: 4), color: white)
    strokeLine(ctx, [CGPoint(x: 15, y: 342), CGPoint(x: 31, y: 340), CGPoint(x: 43, y: 345)], color: visor, width: 4)

    strokeLine(ctx, [CGPoint(x: -4, y: 396), CGPoint(x: -16, y: 421)], color: jacketDark, width: 7)
    strokeLine(ctx, [CGPoint(x: 32, y: 394), CGPoint(x: 52, y: 414)], color: blue, width: 7)
    fillEllipse(ctx, CGRect(x: -15, y: 411, width: 20, height: 20), color: orange)
    fillEllipse(ctx, CGRect(x: -9, y: 417, width: 9, height: 9), color: cg(0xffd66e))

    let pulse = 0.28 + 0.52 * (sin(t * .pi * 2) + 1) / 2
    strokeLine(ctx, [CGPoint(x: 6, y: 360), CGPoint(x: 25, y: 360)], color: cg(0x63e6ff, pulse), width: 3)
    fillEllipse(ctx, CGRect(x: -33, y: 274, width: 8, height: 8), color: cg(0xffffff, 0.45))
}

func fillRounded(_ ctx: CGContext, _ rect: CGRect, radius: CGFloat, color: CGColor) {
    ctx.setFillColor(color)
    ctx.addPath(CGPath(roundedRect: rect, cornerWidth: radius, cornerHeight: radius, transform: nil))
    ctx.fillPath()
}

func fillEllipse(_ ctx: CGContext, _ rect: CGRect, color: CGColor) {
    ctx.setFillColor(color)
    ctx.fillEllipse(in: rect)
}

func strokeLine(_ ctx: CGContext, _ points: [CGPoint], color: CGColor, width: CGFloat) {
    guard let first = points.first else { return }
    ctx.setStrokeColor(color)
    ctx.setLineWidth(width)
    ctx.setLineCap(.round)
    ctx.setLineJoin(.round)
    ctx.beginPath()
    ctx.move(to: first)
    for point in points.dropFirst() {
        ctx.addLine(to: point)
    }
    ctx.strokePath()
}

func cg(_ hex: UInt32, _ alpha: CGFloat = 1) -> CGColor {
    CGColor(
        red: CGFloat((hex >> 16) & 0xff) / 255,
        green: CGFloat((hex >> 8) & 0xff) / 255,
        blue: CGFloat(hex & 0xff) / 255,
        alpha: alpha
    )
}

enum AssetError: Error {
    case message(String)
}
