// A sky's decorations: one quad per image, turned as the core placed
// it, each fragment sampling the image at the core's level of detail and
// leaving `beneath * (1 - cover) + light`, `nova_render::sky::lit` and
// `over`. It decides nothing the core hasn't defined.

struct Globals {
    // The logical space's width and height.
    logical: vec2<f32>,
    pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
// The image, premultiplied, with its mip chain, and the one sampler every
// sky image is read through.
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;

// Logical units (y down) to clip space (y up).
fn to_clip(p: vec2<f32>) -> vec4<f32> {
    let unit = p / globals.logical;
    return vec4<f32>(unit.x * 2.0 - 1.0, 1.0 - unit.y * 2.0, 0.0, 1.0);
}

struct ImageOut {
    @builtin(position) position: vec4<f32>,
    // Across the image from its left, and down it from its top, 0 to 1.
    @location(0) uv: vec2<f32>,
    // What the texel's colour is multiplied by, then what its alpha is.
    @location(1) @interpolate(flat) color: vec4<f32>,
    @location(2) @interpolate(flat) lod: f32,
};

@vertex
fn image_vs(
    @builtin(vertex_index) index: u32,
    // The centre, then the vector to the middle of the right edge.
    @location(0) place: vec4<f32>,
    // The vector to the middle of the bottom edge, the cover, then the
    // level of detail.
    @location(1) shape: vec4<f32>,
    // The colour, then nothing.
    @location(2) color: vec4<f32>,
) -> ImageOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let corner = corners[index];
    let along = corner * 2.0 - vec2<f32>(1.0);
    let at = place.xy + place.zw * along.x + shape.xy * along.y;
    var out: ImageOut;
    out.position = to_clip(at);
    out.uv = corner;
    out.color = vec4<f32>(color.rgb, shape.z);
    out.lod = shape.w;
    return out;
}

@fragment
fn image_fs(in: ImageOut) -> @location(0) vec4<f32> {
    let texel = textureSampleLevel(image, image_sampler, in.uv, in.lod);
    return vec4<f32>(texel.rgb * in.color.rgb, texel.a * in.color.a);
}
