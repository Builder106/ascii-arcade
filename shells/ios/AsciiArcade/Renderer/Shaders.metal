#include <metal_stdlib>
using namespace metal;

// ── Shared uniforms ───────────────────────────────────────────────────────────

struct Uniforms {
    float2 viewportSize;  // drawable size in pixels
    float4 bgColor;
    float4 fgColor;       // theme default text colour
};

// Per-instance glyph data.
// Field order is chosen so float4 comes first → no implicit padding on either
// the Swift or Metal side. Both sides must match exactly.
//   offset 0:  color        16 bytes
//   offset 16: cellOriginPx  8 bytes
//   offset 24: uvOrigin       8 bytes
//   offset 32: uvSize         8 bytes
//   offset 40: useColor       4 bytes
//   offset 44: _pad           4 bytes
//   total: 48 bytes
struct GlyphInstance {
    float4  color;
    float2  cellOriginPx;
    float2  uvOrigin;
    float2  uvSize;
    uint    useColor;
    float   _pad;
};

// Convert drawable-pixel coordinate to Metal NDC (y axis flipped).
static inline float4 toNDC(float2 px, float2 vpSize) {
    return float4(
         px.x / vpSize.x * 2.0f - 1.0f,
        -px.y / vpSize.y * 2.0f + 1.0f,
        0.0f, 1.0f
    );
}

// ── Background (fullscreen quad, triangle strip) ──────────────────────────────

struct BgVOut { float4 position [[position]]; };

vertex BgVOut bg_vert(uint vid [[vertex_id]], constant Uniforms &u [[buffer(0)]]) {
    float2 vp = u.viewportSize;
    const float2 corners[4] = { float2(0,0), float2(vp.x,0), float2(0,vp.y), float2(vp.x,vp.y) };
    BgVOut out;
    out.position = toNDC(corners[vid], vp);
    return out;
}

fragment float4 bg_frag(BgVOut in [[stage_in]], constant Uniforms &u [[buffer(0)]]) {
    return u.bgColor;
}

// ── Glyphs (instanced, 6 vertices per quad) ───────────────────────────────────

struct GlyphVOut { float4 position [[position]]; float2 uv; float4 color; };

vertex GlyphVOut glyph_vert(
    uint vid [[vertex_id]],
    uint iid [[instance_id]],
    constant GlyphInstance *instances [[buffer(0)]],
    constant Uniforms &u [[buffer(1)]],
    constant float2 &cellSizePx [[buffer(2)]]
) {
    const float2 corners[6] = {
        float2(0,0), float2(1,0), float2(0,1),
        float2(1,0), float2(1,1), float2(0,1)
    };
    float2 corner = corners[vid];
    GlyphInstance inst = instances[iid];

    GlyphVOut out;
    out.position = toNDC(inst.cellOriginPx + corner * cellSizePx, u.viewportSize);
    out.uv = inst.uvOrigin + corner * inst.uvSize;
    out.color = inst.useColor > 0u ? inst.color : u.fgColor;
    return out;
}

fragment float4 glyph_frag(
    GlyphVOut in [[stage_in]],
    texture2d<float> atlas [[texture(0)]],
    sampler samp [[sampler(0)]]
) {
    float coverage = atlas.sample(samp, in.uv).r;
    if (coverage < 0.02f) discard_fragment();
    return float4(in.color.rgb, in.color.a * coverage);
}
