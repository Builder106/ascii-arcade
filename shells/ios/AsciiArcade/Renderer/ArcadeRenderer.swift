import Metal
import MetalKit
import simd

// Owns all Metal state and drives the render loop as the MTKViewDelegate.
// All methods are called exclusively from the MTKView render thread.
final class ArcadeRenderer: NSObject, MTKViewDelegate, @unchecked Sendable {

    // MARK: - Metal objects

    private var device: MTLDevice?
    private var commandQueue: MTLCommandQueue?
    private var pipelines: GlyphPipelines?
    private var glyphAtlas: GlyphAtlas?
    private var instanceBuffer: MTLBuffer?

    // MARK: - Scene / theme (written from SwiftUI, read on render thread)

    var engine: AaEngine?
    var currentSceneId: String = ""
    var theme: Theme = .hacker

    // MARK: - Layout (drawable pixels)

    private var storedTopInsetPt: CGFloat = 0
    private var storedBottomInsetPt: CGFloat = 0
    private var storedTabBarHeightPt: CGFloat = 49
    private var topInsetPx: Float = 0
    private var cellSizePx: SIMD2<Float> = .zero
    private var cols: Int = 0
    private var rows: Int = 0
    private var instanceCount: Int = 0

    // MARK: - Lifecycle

    override init() { super.init() }

    @MainActor func attach(to view: MTKView) {
        guard let dev = MTLCreateSystemDefaultDevice(),
              let queue = dev.makeCommandQueue() else { return }
        device = dev
        commandQueue = queue

        view.device = dev
        view.colorPixelFormat = .bgra8Unorm
        view.framebufferOnly = false
        view.delegate = self

        pipelines = GlyphPipelines.build(device: dev, pixelFormat: view.colorPixelFormat)
    }

    // MARK: - Layout

    func updateLayout(drawableSize: CGSize, contentScale: CGFloat, topInsetPt: CGFloat, bottomInsetPt: CGFloat, tabBarHeightPt: CGFloat) {
        guard drawableSize.width > 0, drawableSize.height > 0 else { return }
        storedTopInsetPt = topInsetPt
        storedBottomInsetPt = bottomInsetPt
        storedTabBarHeightPt = tabBarHeightPt

        guard let dev = device else { return }
        let scale = Float(contentScale)
        topInsetPx = Float(topInsetPt) * scale
        let bottomPx = (Float(bottomInsetPt) + Float(tabBarHeightPt)) * scale

        if glyphAtlas == nil {
            let font = UIFont(name: "Menlo-Regular", size: 13)
                    ?? UIFont.monospacedSystemFont(ofSize: 13, weight: .regular)
            glyphAtlas = GlyphAtlas(device: dev, font: font, scale: contentScale)
        }
        guard let atlas = glyphAtlas else { return }

        cellSizePx = SIMD2<Float>(Float(atlas.cellWidthPx), Float(atlas.cellHeightPx))
        let usableH = Float(drawableSize.height) - topInsetPx - bottomPx
        cols = max(1, Int(Float(drawableSize.width) / cellSizePx.x))
        rows = max(1, Int(usableH / cellSizePx.y))
        engine?.setGrid(width: cols, height: rows)
    }

    // MARK: - Instance buffer

    private func rebuildInstances(buffer: UnsafePointer<UInt8>, width: Int, height: Int, device: MTLDevice) {
        guard let atlas = glyphAtlas else { return }
        let capacity = width * height
        let stride = MemoryLayout<GlyphInstance>.stride
        if instanceBuffer == nil || instanceBuffer!.length < capacity * stride {
            instanceBuffer = device.makeBuffer(length: max(capacity, 1) * stride, options: .storageModeShared)
        }
        guard let ibuf = instanceBuffer else { return }
        let ptr = ibuf.contents().assumingMemoryBound(to: GlyphInstance.self)
        instanceCount = buildGlyphInstances(
            buffer: buffer, width: width, height: height,
            atlas: atlas, cellSizePx: cellSizePx,
            originPx: SIMD2<Float>(0, topInsetPx),
            into: ptr
        )
    }

    // MARK: - MTKViewDelegate

    func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {
        updateLayout(
            drawableSize: size,
            contentScale: view.contentScaleFactor,
            topInsetPt: storedTopInsetPt,
            bottomInsetPt: storedBottomInsetPt,
            tabBarHeightPt: storedTabBarHeightPt
        )
    }

    func draw(in view: MTKView) {
        guard let dev = device,
              let queue = commandQueue,
              let drawable = view.currentDrawable,
              let passDesc = view.currentRenderPassDescriptor,
              let cmdBuf = queue.makeCommandBuffer(),
              let pipelines,
              let atlas = glyphAtlas else { return }

        let t = CACurrentMediaTime()
        if let result = engine?.nextFrame(t: t) {
            rebuildInstances(buffer: result.buffer, width: result.width, height: result.height, device: dev)
        }

        let uniforms = Uniforms(
            viewportSize: SIMD2<Float>(Float(view.drawableSize.width), Float(view.drawableSize.height)),
            bgColor: SIMD4<Float>(0, 0, 0, 1),
            fgColor: rgba(theme.text)
        )

        encodeFrame(
            commandBuffer: cmdBuf,
            passDescriptor: passDesc,
            pipelines: pipelines,
            atlas: atlas,
            instanceBuffer: instanceBuffer,
            instanceCount: instanceCount,
            cellSizePx: cellSizePx,
            uniforms: uniforms
        )

        cmdBuf.present(drawable)
        cmdBuf.commit()
    }
}
