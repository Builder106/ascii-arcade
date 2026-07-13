import Metal
import AVFoundation
import CoreImage
import CoreVideo
import UIKit
import simd

enum LivePhotoExportError: LocalizedError {
    case metalUnavailable
    case engineUnavailable
    case writerSetupFailed
    case stillFrameMissing
    case photoLibraryDenied
    case saveFailed(Error)

    var errorDescription: String? {
        switch self {
        case .metalUnavailable:   return "Metal is unavailable on this device."
        case .engineUnavailable:  return "Could not start the scene engine."
        case .writerSetupFailed:  return "Could not set up video export."
        case .stillFrameMissing:  return "Could not capture the still frame."
        case .photoLibraryDenied: return "Photos access was denied. Enable it in Settings to save Live Photos."
        case .saveFailed(let error): return "Could not save to Photos: \(error.localizedDescription)"
        }
    }
}

// Renders a fixed-length clip of a scene entirely offscreen (own device,
// pipelines, glyph atlas and engine instance — fully decoupled from whatever
// ArcadeRenderer currently has on screen) and encodes it straight into an
// AVAssetWriter as a Live-Photo-tagged .mov, plus a matching still JPEG.
// Runs synchronously start to finish; call from a background thread.
enum LivePhotoRenderJob {
    static let fps: Int32 = 30
    static let durationSeconds: Double = 3.0

    static func run(
        sceneId: String,
        pixelSize: CGSize,
        displayScale: CGFloat,
        bgColor: SIMD4<Float>,
        fgColor: SIMD4<Float>
    ) throws -> (photoURL: URL, videoURL: URL) {
        let totalFrames = Int(Double(fps) * durationSeconds)
        let stillFrameIndex = totalFrames / 2
        // H.264 needs even macroblock-aligned dimensions.
        let width  = Int(pixelSize.width.rounded(.down))  & ~1
        let height = Int(pixelSize.height.rounded(.down)) & ~1
        guard width > 0, height > 0 else { throw LivePhotoExportError.writerSetupFailed }

        guard let device = MTLCreateSystemDefaultDevice(),
              let queue = device.makeCommandQueue() else {
            throw LivePhotoExportError.metalUnavailable
        }
        guard let pipelines = GlyphPipelines.build(device: device, pixelFormat: .bgra8Unorm) else {
            throw LivePhotoExportError.metalUnavailable
        }
        let font = UIFont(name: "Menlo-Regular", size: 13)
                ?? UIFont.monospacedSystemFont(ofSize: 13, weight: .regular)
        guard let atlas = GlyphAtlas(device: device, font: font, scale: displayScale) else {
            throw LivePhotoExportError.metalUnavailable
        }
        guard let engine = AaEngine(sceneId: sceneId) else {
            throw LivePhotoExportError.engineUnavailable
        }

        let cellSizePx = SIMD2<Float>(Float(atlas.cellWidthPx), Float(atlas.cellHeightPx))
        let cols = max(1, Int(Float(width) / cellSizePx.x))
        let rows = max(1, Int(Float(height) / cellSizePx.y))
        engine.setGrid(width: cols, height: rows)

        let assetIdentifier = UUID().uuidString
        let tmpDir = FileManager.default.temporaryDirectory
        let videoURL = tmpDir.appendingPathComponent("\(assetIdentifier).mov")
        let photoURL = tmpDir.appendingPathComponent("\(assetIdentifier).jpg")
        try? FileManager.default.removeItem(at: videoURL)
        try? FileManager.default.removeItem(at: photoURL)

        let writer = try AVAssetWriter(outputURL: videoURL, fileType: .mov)
        writer.metadata = [LivePhotoMetadata.contentIdentifierItem(assetIdentifier: assetIdentifier)]

        let videoSettings: [String: Any] = [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: width,
            AVVideoHeightKey: height
        ]
        let videoInput = AVAssetWriterInput(mediaType: .video, outputSettings: videoSettings)
        videoInput.expectsMediaDataInRealTime = false

        let pixelBufferAttributes: [String: Any] = [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
            kCVPixelBufferWidthKey as String: width,
            kCVPixelBufferHeightKey as String: height,
            kCVPixelBufferMetalCompatibilityKey as String: true
        ]
        let pixelBufferAdaptor = AVAssetWriterInputPixelBufferAdaptor(
            assetWriterInput: videoInput,
            sourcePixelBufferAttributes: pixelBufferAttributes
        )

        guard writer.canAdd(videoInput) else { throw LivePhotoExportError.writerSetupFailed }
        writer.add(videoInput)

        let stillTimeAdaptor = LivePhotoMetadata.makeStillImageTimeAdaptor()
        guard writer.canAdd(stillTimeAdaptor.assetWriterInput) else { throw LivePhotoExportError.writerSetupFailed }
        writer.add(stillTimeAdaptor.assetWriterInput)

        guard writer.startWriting() else {
            throw writer.error ?? LivePhotoExportError.writerSetupFailed
        }
        writer.startSession(atSourceTime: .zero)

        var textureCache: CVMetalTextureCache?
        CVMetalTextureCacheCreate(kCFAllocatorDefault, nil, device, nil, &textureCache)
        guard let textureCache else { throw LivePhotoExportError.metalUnavailable }

        let frameDuration = CMTime(value: 1, timescale: fps)
        var stillImagePixelBuffer: CVPixelBuffer?

        for frameIndex in 0..<totalFrames {
            while !videoInput.isReadyForMoreMediaData {
                if writer.status == .failed { throw writer.error ?? LivePhotoExportError.writerSetupFailed }
                Thread.sleep(forTimeInterval: 0.005)
            }

            guard let pool = pixelBufferAdaptor.pixelBufferPool else { throw LivePhotoExportError.writerSetupFailed }
            var pixelBufferOut: CVPixelBuffer?
            CVPixelBufferPoolCreatePixelBuffer(kCFAllocatorDefault, pool, &pixelBufferOut)
            guard let pixelBuffer = pixelBufferOut else { throw LivePhotoExportError.writerSetupFailed }

            var cvTextureOut: CVMetalTexture?
            CVMetalTextureCacheCreateTextureFromImage(
                kCFAllocatorDefault, textureCache, pixelBuffer, nil,
                .bgra8Unorm, width, height, 0, &cvTextureOut
            )
            guard let cvTexture = cvTextureOut, let texture = CVMetalTextureGetTexture(cvTexture) else {
                throw LivePhotoExportError.writerSetupFailed
            }

            let t = Double(frameIndex) / Double(fps)
            guard let frame = engine.nextFrame(t: t) else { continue }

            let capacity = frame.width * frame.height
            let stride = MemoryLayout<GlyphInstance>.stride
            guard let instanceBuffer = device.makeBuffer(length: max(capacity, 1) * stride, options: .storageModeShared) else {
                throw LivePhotoExportError.writerSetupFailed
            }
            let ptr = instanceBuffer.contents().assumingMemoryBound(to: GlyphInstance.self)
            let instanceCount = buildGlyphInstances(
                buffer: frame.buffer, width: frame.width, height: frame.height,
                atlas: atlas, cellSizePx: cellSizePx, originPx: .zero, into: ptr
            )

            let passDesc = MTLRenderPassDescriptor()
            passDesc.colorAttachments[0].texture = texture
            passDesc.colorAttachments[0].loadAction = .clear
            passDesc.colorAttachments[0].clearColor = MTLClearColor(red: 0, green: 0, blue: 0, alpha: 1)
            passDesc.colorAttachments[0].storeAction = .store

            guard let cmdBuf = queue.makeCommandBuffer() else { throw LivePhotoExportError.writerSetupFailed }
            let uniforms = Uniforms(
                viewportSize: SIMD2<Float>(Float(width), Float(height)),
                bgColor: bgColor,
                fgColor: fgColor
            )
            encodeFrame(
                commandBuffer: cmdBuf, passDescriptor: passDesc, pipelines: pipelines,
                atlas: atlas, instanceBuffer: instanceBuffer, instanceCount: instanceCount,
                cellSizePx: cellSizePx, uniforms: uniforms
            )
            cmdBuf.commit()
            cmdBuf.waitUntilCompleted()

            let presentationTime = CMTime(value: CMTimeValue(frameIndex), timescale: fps)
            pixelBufferAdaptor.append(pixelBuffer, withPresentationTime: presentationTime)

            if frameIndex == stillFrameIndex {
                stillImagePixelBuffer = pixelBuffer
                LivePhotoMetadata.markStillImageTime(stillTimeAdaptor, at: presentationTime, frameDuration: frameDuration)
            }
        }

        videoInput.markAsFinished()
        stillTimeAdaptor.assetWriterInput.markAsFinished()

        let writerFinished = DispatchSemaphore(value: 0)
        writer.finishWriting { writerFinished.signal() }
        writerFinished.wait()

        guard writer.status == .completed else {
            throw writer.error ?? LivePhotoExportError.writerSetupFailed
        }

        guard let stillPixelBuffer = stillImagePixelBuffer,
              let jpegData = jpegData(from: stillPixelBuffer),
              let taggedJpeg = LivePhotoMetadata.taggedJPEGData(from: jpegData, assetIdentifier: assetIdentifier) else {
            throw LivePhotoExportError.stillFrameMissing
        }
        try taggedJpeg.write(to: photoURL)

        return (photoURL, videoURL)
    }

    private static func jpegData(from pixelBuffer: CVPixelBuffer) -> Data? {
        let ciImage = CIImage(cvPixelBuffer: pixelBuffer)
        let context = CIContext()
        guard let colorSpace = CGColorSpace(name: CGColorSpace.sRGB) else { return nil }
        return context.jpegRepresentation(of: ciImage, colorSpace: colorSpace, options: [:])
    }
}
