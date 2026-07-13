import Metal

// Shared bg/glyph render pipeline state, used by both the live MTKView
// renderer and the offscreen Live Photo exporter so the two never drift.
struct GlyphPipelines {
    let bg: MTLRenderPipelineState
    let glyph: MTLRenderPipelineState
    let sampler: MTLSamplerState

    static func build(device: MTLDevice, pixelFormat: MTLPixelFormat) -> GlyphPipelines? {
        guard let lib = device.makeDefaultLibrary() else {
            print("[GlyphPipelines] Metal library not found — check that Shaders.metal is in the target")
            return nil
        }

        let bgDesc = MTLRenderPipelineDescriptor()
        bgDesc.vertexFunction   = lib.makeFunction(name: "bg_vert")
        bgDesc.fragmentFunction = lib.makeFunction(name: "bg_frag")
        bgDesc.colorAttachments[0].pixelFormat = pixelFormat
        guard let bgPipeline = try? device.makeRenderPipelineState(descriptor: bgDesc) else { return nil }

        let gd = MTLRenderPipelineDescriptor()
        gd.vertexFunction   = lib.makeFunction(name: "glyph_vert")
        gd.fragmentFunction = lib.makeFunction(name: "glyph_frag")
        gd.colorAttachments[0].pixelFormat = pixelFormat
        let ca = gd.colorAttachments[0]!
        ca.isBlendingEnabled           = true
        ca.sourceRGBBlendFactor        = .sourceAlpha
        ca.destinationRGBBlendFactor   = .oneMinusSourceAlpha
        ca.sourceAlphaBlendFactor      = .one
        ca.destinationAlphaBlendFactor = .oneMinusSourceAlpha
        guard let glyphPipeline = try? device.makeRenderPipelineState(descriptor: gd) else { return nil }

        let sd = MTLSamplerDescriptor()
        sd.minFilter = .linear
        sd.magFilter = .linear
        sd.sAddressMode = .clampToEdge
        sd.tAddressMode = .clampToEdge
        guard let sampler = device.makeSamplerState(descriptor: sd) else { return nil }

        return GlyphPipelines(bg: bgPipeline, glyph: glyphPipeline, sampler: sampler)
    }
}
