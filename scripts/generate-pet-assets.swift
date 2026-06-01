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
let noticeURL = outputDir.appendingPathComponent("ASSET-NOTICE.txt")

try? FileManager.default.removeItem(at: movieURL)
try? FileManager.default.removeItem(at: hitURL)
try? FileManager.default.removeItem(at: noticeURL)

try writeMovie(to: movieURL)
try writeHitMask(to: hitURL)
try """
Codex Speak Pet Assets

These assets are generated from scripts/generate-pet-assets.swift for the Codex Speak project.
They are original project assets and are intended for distribution with Codex Speak.

The display pipeline intentionally matches the lil-agents style of desktop companion:
- 1080x1920 HEVC-with-alpha .mov.
- Native transparent AppKit window.
- AVPlayerLayer playback on macOS.

Files:
- codex-agent.mov: transparent animated desktop companion.
- codex-agent-hit.png: alpha mask fallback used by the native macOS helper.
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
    fillRounded(ctx, CGRect(x: -58, y: 154, width: 116, height: 160), radius: 30, color: mask)
    fillRounded(ctx, CGRect(x: -16, y: 291, width: 32, height: 34), radius: 12, color: mask)
    fillRounded(ctx, CGRect(x: -56, y: 315, width: 112, height: 84), radius: 32, color: mask)
    fillRounded(ctx, CGRect(x: -82, y: 165 + walk * 10, width: 32, height: 122), radius: 16, color: mask)
    fillRounded(ctx, CGRect(x: 50, y: 165 + counterWalk * 10, width: 32, height: 122), radius: 16, color: mask)
    fillRounded(ctx, CGRect(x: -46 + walk * 14, y: 66, width: 34, height: 112), radius: 16, color: mask)
    fillRounded(ctx, CGRect(x: 12 + counterWalk * 14, y: 66, width: 34, height: 112), radius: 16, color: mask)
    fillRounded(ctx, CGRect(x: -68 + walk * 16, y: 42, width: 66, height: 28), radius: 11, color: mask)
    fillRounded(ctx, CGRect(x: 4 + counterWalk * 16, y: 42, width: 66, height: 28), radius: 11, color: mask)
}

func drawAgentArt(in ctx: CGContext, walk: CGFloat, counterWalk: CGFloat, t: CGFloat) {
    let deep = cg(0x172033)
    let visor = cg(0x08111f)
    let teal = cg(0x36c6a7)
    let tealDark = cg(0x1f8f7e)
    let cyan = cg(0x5de7ff)
    let mint = cg(0xb6fff0)
    let coral = cg(0xff6f61)
    let pink = cg(0xffc0df)
    let cream = cg(0xfff7f1)
    let white = cg(0xffffff)
    let sole = cg(0xfafcff)
    let orange = cg(0xff7d30)
    let violet = cg(0x8d7cff)

    fillEllipse(ctx, CGRect(x: -60, y: 34, width: 120, height: 16), color: cg(0x000000, 0.08))

    strokeLine(ctx, [CGPoint(x: -43, y: 188), CGPoint(x: -74, y: 150 + walk * 12), CGPoint(x: -66, y: 113 + walk * 7)], color: tealDark, width: 20)
    strokeLine(ctx, [CGPoint(x: 48, y: 190), CGPoint(x: 78, y: 153 + counterWalk * 12), CGPoint(x: 69, y: 115 + counterWalk * 7)], color: deep, width: 20)
    fillRounded(ctx, CGRect(x: -77, y: 102 + walk * 7, width: 28, height: 24), radius: 11, color: pink)
    fillRounded(ctx, CGRect(x: 54, y: 104 + counterWalk * 7, width: 28, height: 24), radius: 11, color: pink)

    strokeLine(ctx, [CGPoint(x: -28, y: 162), CGPoint(x: -40 + walk * 14, y: 73)], color: cream, width: 26)
    strokeLine(ctx, [CGPoint(x: 26, y: 162), CGPoint(x: 38 + counterWalk * 14, y: 73)], color: deep, width: 26)
    fillRounded(ctx, CGRect(x: -71 + walk * 16, y: 40, width: 68, height: 30), radius: 11, color: sole)
    fillRounded(ctx, CGRect(x: -64 + walk * 16, y: 48, width: 58, height: 14), radius: 7, color: orange)
    fillRounded(ctx, CGRect(x: 2 + counterWalk * 16, y: 40, width: 68, height: 30), radius: 11, color: sole)
    fillRounded(ctx, CGRect(x: 9 + counterWalk * 16, y: 48, width: 58, height: 14), radius: 7, color: cyan)

    fillRounded(ctx, CGRect(x: -60, y: 150, width: 120, height: 166), radius: 31, color: teal)
    fillRounded(ctx, CGRect(x: -45, y: 158, width: 90, height: 146), radius: 25, color: deep)
    fillRounded(ctx, CGRect(x: -35, y: 176, width: 70, height: 100), radius: 22, color: cream)
    fillRounded(ctx, CGRect(x: -28, y: 196, width: 56, height: 18), radius: 9, color: mint)
    strokeLine(ctx, [CGPoint(x: -50, y: 280), CGPoint(x: -18, y: 252)], color: cg(0xffffff, 0.22), width: 4)
    strokeLine(ctx, [CGPoint(x: 50, y: 280), CGPoint(x: 19, y: 252)], color: cg(0xffffff, 0.18), width: 4)
    fillRounded(ctx, CGRect(x: -44, y: 242, width: 25, height: 20), radius: 5, color: cg(0x0b1220, 0.28))
    fillRounded(ctx, CGRect(x: 18, y: 242, width: 25, height: 20), radius: 5, color: cg(0x0b1220, 0.28))
    fillEllipse(ctx, CGRect(x: -8, y: 226, width: 16, height: 16), color: coral)

    fillRounded(ctx, CGRect(x: -13, y: 292, width: 26, height: 30), radius: 10, color: pink)
    fillRounded(ctx, CGRect(x: -54, y: 316, width: 108, height: 84), radius: 32, color: pink)
    fillRounded(ctx, CGRect(x: -49, y: 360, width: 92, height: 28), radius: 13, color: deep)
    fillRounded(ctx, CGRect(x: -37, y: 341, width: 78, height: 30), radius: 14, color: visor)
    fillEllipse(ctx, CGRect(x: -23, y: 350, width: 12, height: 12), color: cyan)
    fillEllipse(ctx, CGRect(x: 11, y: 350, width: 12, height: 12), color: cyan)
    fillEllipse(ctx, CGRect(x: -19, y: 355, width: 4, height: 4), color: white)
    fillEllipse(ctx, CGRect(x: 15, y: 355, width: 4, height: 4), color: white)
    strokeLine(ctx, [CGPoint(x: -16, y: 335), CGPoint(x: -2, y: 330), CGPoint(x: 14, y: 335)], color: visor, width: 4)

    strokeLine(ctx, [CGPoint(x: 0, y: 396), CGPoint(x: 0, y: 415)], color: deep, width: 7)
    fillEllipse(ctx, CGRect(x: -11, y: 408, width: 22, height: 22), color: coral)
    fillEllipse(ctx, CGRect(x: -6, y: 413, width: 12, height: 12), color: cg(0xffd76a))

    let pulse = 0.35 + 0.45 * (sin(t * .pi * 2) + 1) / 2
    strokeLine(ctx, [CGPoint(x: -35, y: 356), CGPoint(x: -16, y: 356)], color: cg(0x5de7ff, pulse), width: 3)
    strokeLine(ctx, [CGPoint(x: 14, y: 356), CGPoint(x: 34, y: 356)], color: cg(0x5de7ff, pulse), width: 3)
    strokeLine(ctx, [CGPoint(x: -44, y: 383), CGPoint(x: -68, y: 397)], color: cg(0x8d7cff, 0.72), width: 5)
    strokeLine(ctx, [CGPoint(x: 40, y: 383), CGPoint(x: 62, y: 398)], color: cg(0x8d7cff, 0.72), width: 5)
    strokeLine(ctx, [CGPoint(x: -21, y: 213), CGPoint(x: 21, y: 213)], color: cg(0xffffff, 0.5), width: 2)
    fillEllipse(ctx, CGRect(x: -4, y: 207, width: 8, height: 8), color: violet)
    fillEllipse(ctx, CGRect(x: -35, y: 272, width: 8, height: 8), color: cg(0xffffff, 0.45))
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
