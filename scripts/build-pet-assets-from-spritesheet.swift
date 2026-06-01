import AVFoundation
import CoreGraphics
import Foundation
import ImageIO

let arguments = CommandLine.arguments.dropFirst()
let sourcePath = arguments.first ?? "apps/codex-speak-pet-macos/assets/codex-agent-source-spritesheet.png"
let outputDir = URL(
    fileURLWithPath: arguments.dropFirst().first ?? "apps/codex-speak-pet-macos/assets",
    isDirectory: true
)
let sourceURL = URL(fileURLWithPath: sourcePath)

let outputWidth = 1080
let outputHeight = 1920
let fps: Int32 = 24
let frameCount = 241
let sourceColumns = 6
let spriteFps = 8.0
let outputColorSpace = CGColorSpaceCreateDeviceRGB()
let outputBitmapInfo = CGBitmapInfo(
    rawValue: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue
)

let movieURL = outputDir.appendingPathComponent("codex-agent.mov")
let hitURL = outputDir.appendingPathComponent("codex-agent-hit.png")
let previewURL = outputDir.appendingPathComponent("codex-agent-preview.png")
let noticeURL = outputDir.appendingPathComponent("ASSET-NOTICE.txt")

try FileManager.default.createDirectory(at: outputDir, withIntermediateDirectories: true)

let sheet = try loadImage(sourceURL)
let sourceFrames = try splitSpriteSheet(sheet, columns: sourceColumns).map(preparedSpriteImage)

try? FileManager.default.removeItem(at: movieURL)
try? FileManager.default.removeItem(at: hitURL)
try? FileManager.default.removeItem(at: previewURL)
try? FileManager.default.removeItem(at: noticeURL)

try writeMovie(to: movieURL, frames: sourceFrames)
try writeStill(to: previewURL, image: sourceFrames[min(2, sourceFrames.count - 1)])
try writeStill(to: hitURL, image: sourceFrames[min(2, sourceFrames.count - 1)])
try writeNotice(to: noticeURL, sourceName: sourceURL.lastPathComponent)

print("Built lil-style pet assets from \(sourceURL.path) into \(outputDir.path)")

func loadImage(_ url: URL) throws -> CGImage {
    guard let imageSource = CGImageSourceCreateWithURL(url as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(imageSource, 0, nil) else {
        throw AssetError.message("Could not load image: \(url.path)")
    }
    return image
}

func splitSpriteSheet(_ sheet: CGImage, columns: Int) throws -> [CGImage] {
    guard columns > 0 else {
        throw AssetError.message("Sprite sheet must have at least one column")
    }

    return try (0..<columns).map { index in
        let startX = Int((Double(index) * Double(sheet.width) / Double(columns)).rounded(.toNearestOrAwayFromZero))
        let endX = Int((Double(index + 1) * Double(sheet.width) / Double(columns)).rounded(.toNearestOrAwayFromZero))
        let rect = CGRect(x: startX, y: 0, width: max(1, endX - startX), height: sheet.height)
        guard let frame = sheet.cropping(to: rect) else {
            throw AssetError.message("Could not crop sprite frame \(index)")
        }
        return frame
    }
}

func preparedSpriteImage(_ image: CGImage) throws -> CGImage {
    let width = image.width
    let height = image.height
    let bytesPerRow = width * 4
    var buffer = [UInt8](repeating: 0, count: bytesPerRow * height)
    let inputBitmapInfo = CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue)

    guard let context = CGContext(
        data: &buffer,
        width: width,
        height: height,
        bitsPerComponent: 8,
        bytesPerRow: bytesPerRow,
        space: outputColorSpace,
        bitmapInfo: inputBitmapInfo.rawValue
    ) else {
        throw AssetError.message("Could not create chroma-key context")
    }

    context.interpolationQuality = .high
    context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
    let alreadyHasAlpha = stride(from: 3, to: buffer.count, by: 4).contains { buffer[$0] < 250 }
    if alreadyHasAlpha {
        return try makeImage(width: width, height: height, bytesPerRow: bytesPerRow, buffer: buffer, bitmapInfo: inputBitmapInfo)
    }

    for offset in stride(from: 0, to: buffer.count, by: 4) {
        let r = Double(buffer[offset])
        let g = Double(buffer[offset + 1])
        let b = Double(buffer[offset + 2])
        let keyStrength = greenKeyStrength(red: r, green: g, blue: b)
        guard keyStrength > 0 else { continue }

        let alpha: UInt8
        if keyStrength >= 0.22 {
            alpha = 0
        } else {
            alpha = UInt8(max(0, min(255, round(255.0 * (1.0 - keyStrength * 1.2)))))
        }
        if alpha == 0 {
            buffer[offset] = 0
            buffer[offset + 1] = 0
            buffer[offset + 2] = 0
        } else {
            let greenCap = max(r, b) + 24.0
            buffer[offset + 1] = UInt8(max(0, min(255, round(min(g, greenCap)))))
        }
        buffer[offset + 3] = min(buffer[offset + 3], alpha)
    }
    contractGreenEdge(&buffer, width: width, height: height, bytesPerRow: bytesPerRow)

    return try makeImage(width: width, height: height, bytesPerRow: bytesPerRow, buffer: buffer, bitmapInfo: inputBitmapInfo)
}

func contractGreenEdge(_ buffer: inout [UInt8], width: Int, height: Int, bytesPerRow: Int) {
    let original = buffer
    for y in 1..<(height - 1) {
        for x in 1..<(width - 1) {
            let offset = y * bytesPerRow + x * 4
            guard original[offset + 3] > 0 else { continue }
            var touchesTransparent = false
            for ny in (y - 1)...(y + 1) {
                for nx in (x - 1)...(x + 1) where nx != x || ny != y {
                    let neighbor = ny * bytesPerRow + nx * 4
                    if original[neighbor + 3] == 0 {
                        touchesTransparent = true
                        break
                    }
                }
                if touchesTransparent { break }
            }
            guard touchesTransparent else { continue }

            let r = Int(original[offset])
            let g = Int(original[offset + 1])
            let b = Int(original[offset + 2])
            if g > 100 && g - max(r, b) > 8 {
                buffer[offset] = 0
                buffer[offset + 1] = 0
                buffer[offset + 2] = 0
                buffer[offset + 3] = 0
            }
        }
    }
}

func greenKeyStrength(red r: Double, green g: Double, blue b: Double) -> Double {
    let strongestNonGreen = max(r, b)
    guard g > 130, g - strongestNonGreen > 28 else { return 0 }
    let greenAmount = smoothstep(edge0: 130, edge1: 235, value: g)
    let subjectProtection = 1.0 - smoothstep(edge0: 135, edge1: 205, value: strongestNonGreen)
    let dominance = smoothstep(edge0: 28, edge1: 145, value: g - strongestNonGreen)
    let haloProtection = 1.0 - smoothstep(edge0: 145, edge1: 190, value: strongestNonGreen)
    let haloStrength = smoothstep(edge0: 34, edge1: 110, value: g - strongestNonGreen) * haloProtection
    return max(0, min(1, max(greenAmount * subjectProtection * dominance, haloStrength)))
}

func smoothstep(edge0: Double, edge1: Double, value: Double) -> Double {
    if edge0 == edge1 { return value < edge0 ? 0 : 1 }
    let t = max(0, min(1, (value - edge0) / (edge1 - edge0)))
    return t * t * (3 - 2 * t)
}

func writeMovie(to url: URL, frames: [CGImage]) throws {
    let writer = try AVAssetWriter(outputURL: url, fileType: .mov)
    let outputSettings: [String: Any] = [
        AVVideoCodecKey: AVVideoCodecType.hevcWithAlpha,
        AVVideoWidthKey: outputWidth,
        AVVideoHeightKey: outputHeight
    ]
    let input = AVAssetWriterInput(mediaType: .video, outputSettings: outputSettings)
    input.expectsMediaDataInRealTime = false
    let adaptor = AVAssetWriterInputPixelBufferAdaptor(
        assetWriterInput: input,
        sourcePixelBufferAttributes: [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
            kCVPixelBufferWidthKey as String: outputWidth,
            kCVPixelBufferHeightKey as String: outputHeight,
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

    for frameIndex in 0..<frameCount {
        while !input.isReadyForMoreMediaData {
            Thread.sleep(forTimeInterval: 0.01)
        }
        let spriteIndex = Int(floor((Double(frameIndex) / Double(fps)) * spriteFps)) % frames.count
        let buffer = try renderPixelBuffer(sprite: frames[spriteIndex])
        let time = CMTime(value: CMTimeValue(frameIndex), timescale: fps)
        guard adaptor.append(buffer, withPresentationTime: time) else {
            throw AssetError.message("Failed to append movie frame \(frameIndex)")
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

func writeStill(to url: URL, image: CGImage) throws {
    let buffer = try renderPixelBuffer(sprite: image)
    guard let rendered = cgImage(from: buffer) else {
        throw AssetError.message("Could not create still CGImage")
    }
    try writePNG(rendered, to: url)
}

func renderPixelBuffer(sprite: CGImage) throws -> CVPixelBuffer {
    var pixelBuffer: CVPixelBuffer?
    let status = CVPixelBufferCreate(
        kCFAllocatorDefault,
        outputWidth,
        outputHeight,
        kCVPixelFormatType_32BGRA,
        [
            kCVPixelBufferCGImageCompatibilityKey as String: true,
            kCVPixelBufferCGBitmapContextCompatibilityKey as String: true
        ] as CFDictionary,
        &pixelBuffer
    )
    guard status == kCVReturnSuccess, let buffer = pixelBuffer else {
        throw AssetError.message("Could not allocate output pixel buffer")
    }

    CVPixelBufferLockBaseAddress(buffer, [])
    defer { CVPixelBufferUnlockBaseAddress(buffer, []) }
    guard let base = CVPixelBufferGetBaseAddress(buffer) else {
        throw AssetError.message("Could not lock output pixel buffer")
    }

    let bytesPerRow = CVPixelBufferGetBytesPerRow(buffer)
    memset(base, 0, bytesPerRow * outputHeight)
    guard let context = CGContext(
        data: base,
        width: outputWidth,
        height: outputHeight,
        bitsPerComponent: 8,
        bytesPerRow: bytesPerRow,
        space: outputColorSpace,
        bitmapInfo: outputBitmapInfo.rawValue
    ) else {
        throw AssetError.message("Could not create output drawing context")
    }

    context.interpolationQuality = .high
    let maxHeight = CGFloat(outputHeight) * 0.9
    let maxWidth = CGFloat(outputWidth) * 0.82
    let scale = min(maxWidth / CGFloat(sprite.width), maxHeight / CGFloat(sprite.height))
    let drawWidth = CGFloat(sprite.width) * scale
    let drawHeight = CGFloat(sprite.height) * scale
    let drawRect = CGRect(
        x: (CGFloat(outputWidth) - drawWidth) / 2,
        y: CGFloat(outputHeight) * 0.045,
        width: drawWidth,
        height: drawHeight
    )
    context.draw(sprite, in: drawRect)
    return buffer
}

func cgImage(from buffer: CVPixelBuffer) -> CGImage? {
    CVPixelBufferLockBaseAddress(buffer, .readOnly)
    defer { CVPixelBufferUnlockBaseAddress(buffer, .readOnly) }
    guard let base = CVPixelBufferGetBaseAddress(buffer) else { return nil }
    let bytesPerRow = CVPixelBufferGetBytesPerRow(buffer)
    let data = Data(bytes: base, count: bytesPerRow * outputHeight)
    guard let provider = CGDataProvider(data: data as CFData) else { return nil }
    return CGImage(
        width: outputWidth,
        height: outputHeight,
        bitsPerComponent: 8,
        bitsPerPixel: 32,
        bytesPerRow: bytesPerRow,
        space: outputColorSpace,
        bitmapInfo: outputBitmapInfo,
        provider: provider,
        decode: nil,
        shouldInterpolate: true,
        intent: .defaultIntent
    )
}

func makeImage(width: Int, height: Int, bytesPerRow: Int, buffer: [UInt8], bitmapInfo: CGBitmapInfo) throws -> CGImage {
    let data = Data(buffer)
    guard let provider = CGDataProvider(data: data as CFData),
          let image = CGImage(
              width: width,
              height: height,
              bitsPerComponent: 8,
              bitsPerPixel: 32,
              bytesPerRow: bytesPerRow,
              space: outputColorSpace,
              bitmapInfo: bitmapInfo,
              provider: provider,
              decode: nil,
              shouldInterpolate: true,
              intent: .defaultIntent
          ) else {
        throw AssetError.message("Could not create keyed CGImage")
    }
    return image
}

func writePNG(_ image: CGImage, to url: URL) throws {
    guard let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil) else {
        throw AssetError.message("Could not create PNG destination: \(url.path)")
    }
    CGImageDestinationAddImage(dest, image, nil)
    if !CGImageDestinationFinalize(dest) {
        throw AssetError.message("Could not write PNG: \(url.path)")
    }
}

func writeNotice(to url: URL, sourceName: String) throws {
    try """
    Codex Speak Pet Assets

    These assets are built from an original AI-assisted 2D sprite sheet for the Codex Speak project.
    They are intended for distribution with Codex Speak.

    The display pipeline intentionally matches the lil-agents style of desktop companion:
    - 1080x1920 HEVC-with-alpha .mov.
    - Native transparent AppKit window.
    - AVPlayerLayer playback on macOS.
    - A 2D walking character designed as a transparent video sprite, not a WebView/canvas/3D render.

    Source:
    - \(sourceName): original six-frame 2D sprite sheet, usually preprocessed to alpha before video encoding.

    Files:
    - codex-agent.mov: transparent animated desktop companion.
    - codex-agent-hit.png: alpha mask fallback used by the native macOS helper.
    - codex-agent-preview.png: transparent still frame for quick visual QA.
    """.write(to: url, atomically: true, encoding: .utf8)
}

enum AssetError: Error, CustomStringConvertible {
    case message(String)

    var description: String {
        switch self {
        case let .message(value):
            return value
        }
    }
}
