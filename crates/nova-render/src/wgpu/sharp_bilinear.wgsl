// Enlarges the world texture onto the viewport with sharp-bilinear: each
// target pixel samples the texel under it at its centre (nearest), except
// within half a target pixel of a seam between texels, where it blends
// the two linearly.
//
// The maths is `sharp-bilinear-simple` from the libretro slang-shaders
// (interpolation/shaders/sharp-bilinear-simple.slang), which is in the
// public domain, applied on each axis with that axis's own scale.

struct Params {
    // The world texture's size in texels.
    source: vec2<f32>,
    // The top-left corner of the rectangle it fills, in target pixels.
    origin: vec2<f32>,
    // That rectangle's size, in target pixels.
    size: vec2<f32>,
    // Target pixels a texel on each axis, at least 1.
    scale: vec2<f32>,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var world: texture_2d<f32>;
@group(0) @binding(2) var world_sampler: sampler;

// One triangle covering the whole viewport: clip (-1, -1), (3, -1), (-1, 3).
@vertex
fn sharp_vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn sharp_fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = (position.xy - params.origin) / params.size;
    let texel = uv * params.source;
    let region = 0.5 - 0.5 / params.scale;
    let d = fract(texel) - 0.5;
    let f = (d - clamp(d, -region, region)) * params.scale + 0.5;
    let at = (floor(texel) + f) / params.source;
    return vec4<f32>(textureSampleLevel(world, world_sampler, at, 0.0).rgb, 1.0);
}
