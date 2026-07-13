import Metal
import simd
import UIKit

// Per-instance glyph data. Must match GlyphInstance in Shaders.metal exactly
// (48 bytes, no implicit padding — float4 color comes first so nothing
// requires 16-byte alignment after a non-16-byte-multiple offset).
struct GlyphInstance {
    var color: SIMD4<Float>          // offset  0, 16 bytes
    var cellOriginPx: SIMD2<Float>   // offset 16,  8 bytes
    var uvOrigin: SIMD2<Float>       // offset 24,  8 bytes
    var uvSize: SIMD2<Float>         // offset 32,  8 bytes
    var useColor: UInt32             // offset 40,  4 bytes
    var pad: Float                   // offset 44,  4 bytes
    // total: 48 bytes
}

struct Uniforms {
    var viewportSize: SIMD2<Float>
    var bgColor: SIMD4<Float>
    var fgColor: SIMD4<Float>
}

// Converts an aa-ffi frame buffer (8 bytes/cell: u32 codepoint LE + RGB +
// has_color flag) into a GlyphInstance buffer. `into` must have capacity for
// at least width*height instances. Returns the number of non-space instances
// written. Shared by the live MTKView renderer and the offscreen Live Photo
// exporter so the two can't visually drift from each other.
func buildGlyphInstances(
    buffer: UnsafePointer<UInt8>,
    width: Int,
    height: Int,
    atlas: GlyphAtlas,
    cellSizePx: SIMD2<Float>,
    originPx: SIMD2<Float>,
    into ptr: UnsafeMutablePointer<GlyphInstance>
) -> Int {
    var count = 0
    for row in 0..<height {
        for col in 0..<width {
            let i = (row * width + col) * 8
            let cp = UInt32(buffer[i])
                | UInt32(buffer[i+1]) << 8
                | UInt32(buffer[i+2]) << 16
                | UInt32(buffer[i+3]) << 24
            guard cp != 0x20, cp != 0,
                  let scalar = Unicode.Scalar(cp),
                  let glyphIdx = atlas.index(for: scalar) else { continue }

            let uv = atlas.uvRect(for: glyphIdx)
            let useColor = buffer[i + 7] == 1
            let color: SIMD4<Float> = useColor
                ? SIMD4<Float>(Float(buffer[i+4]) / 255, Float(buffer[i+5]) / 255, Float(buffer[i+6]) / 255, 1)
                : SIMD4<Float>(0, 0, 0, 1)

            ptr[count] = GlyphInstance(
                color: color,
                cellOriginPx: SIMD2<Float>(originPx.x + Float(col) * cellSizePx.x, originPx.y + Float(row) * cellSizePx.y),
                uvOrigin: SIMD2<Float>(uv.x, uv.y),
                uvSize:   SIMD2<Float>(uv.w, uv.h),
                useColor: useColor ? 1 : 0,
                pad: 0
            )
            count += 1
        }
    }
    return count
}

// Encodes one full frame (background quad + glyph instances) into the given
// render pass. Shared by the live MTKView renderer and the offscreen Live
// Photo exporter.
func encodeFrame(
    commandBuffer: MTLCommandBuffer,
    passDescriptor: MTLRenderPassDescriptor,
    pipelines: GlyphPipelines,
    atlas: GlyphAtlas,
    instanceBuffer: MTLBuffer?,
    instanceCount: Int,
    cellSizePx: SIMD2<Float>,
    uniforms: Uniforms
) {
    guard let enc = commandBuffer.makeRenderCommandEncoder(descriptor: passDescriptor) else { return }
    var u = uniforms

    // Background — fullscreen quad (triangle strip, 4 verts).
    enc.setRenderPipelineState(pipelines.bg)
    enc.setVertexBytes(&u, length: MemoryLayout<Uniforms>.stride, index: 0)
    enc.setFragmentBytes(&u, length: MemoryLayout<Uniforms>.stride, index: 0)
    enc.drawPrimitives(type: .triangleStrip, vertexStart: 0, vertexCount: 4)

    // Glyphs — one instanced call for all non-space cells.
    if instanceCount > 0, let ibuf = instanceBuffer {
        enc.setRenderPipelineState(pipelines.glyph)
        enc.setVertexBuffer(ibuf, offset: 0, index: 0)
        enc.setVertexBytes(&u, length: MemoryLayout<Uniforms>.stride, index: 1)
        var cs = cellSizePx
        enc.setVertexBytes(&cs, length: MemoryLayout<SIMD2<Float>>.stride, index: 2)
        enc.setFragmentTexture(atlas.texture, index: 0)
        enc.setFragmentSamplerState(pipelines.sampler, index: 0)
        enc.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 6, instanceCount: instanceCount)
    }

    enc.endEncoding()
}

// Reads a theme UIColor into an RGBA SIMD4<Float> for the Uniforms struct.
func rgba(_ color: UIColor) -> SIMD4<Float> {
    var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
    color.getRed(&r, green: &g, blue: &b, alpha: &a)
    return SIMD4<Float>(Float(r), Float(g), Float(b), Float(a))
}
