// 3D models: each vertex through its instance's clip_from_model, each
// fragment as `nova_render::camera::fragment` defines: the vertex colour
// times the section's texel, lit as `nova_render::camera::shade` defines
// from the interpolated normal on the view's axes, unless it is unlit;
// then, as `nova_render::camera::colorized` defines, its colour mixed
// toward the instance's colorize when that mixes, as a sprite's texel is;
// then, as `nova_render::camera::faded` defines, its alpha times the
// instance's.

// The section's base-colour texture, or one opaque white texel for a
// section with none.
@group(0) @binding(0) var base_color: texture_2d<f32>;
@group(0) @binding(1) var base_sampler: sampler;

struct ModelOut {
    // Invariant, so a faded model's colour pass lands on exactly the depths
    // its depth pass wrote, through another pipeline.
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    // The batch's light: toward it, then the ambient.
    @location(2) @interpolate(flat) light: vec4<f32>,
    @location(3) uv: vec2<f32>,
    // 1 when the vertex is unlit, 0 when it is lit: the same for every
    // vertex of a triangle.
    @location(4) @interpolate(flat) unlit: f32,
    // The instance's alpha.
    @location(5) @interpolate(flat) alpha: f32,
    // The instance's colorize: the colour's 5-bit red, green and blue,
    // then the level; all zero for none.
    @location(6) @interpolate(flat) colorize: vec4<f32>,
};

@vertex
fn model_vs(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) unlit: f32,
    @location(5) c0: vec4<f32>,
    @location(6) c1: vec4<f32>,
    @location(7) c2: vec4<f32>,
    @location(8) c3: vec4<f32>,
    @location(9) n0: vec4<f32>,
    @location(10) n1: vec4<f32>,
    @location(11) n2: vec4<f32>,
    @location(12) light: vec4<f32>,
    @location(13) alpha: f32,
    @location(14) colorize: vec4<f32>,
) -> ModelOut {
    let clip_from_model = mat4x4<f32>(c0, c1, c2, c3);
    let normal_from_model = mat3x3<f32>(n0.xyz, n1.xyz, n2.xyz);
    var out: ModelOut;
    out.position = clip_from_model * vec4<f32>(position, 1.0);
    out.normal = normal_from_model * normal;
    out.color = color;
    out.light = light;
    out.uv = uv;
    out.unlit = unlit;
    out.alpha = alpha;
    out.colorize = colorize;
    return out;
}

// `v` as a unit vector, or zero when it has no length.
fn unit(v: vec3<f32>) -> vec3<f32> {
    let length = length(v);
    return select(vec3<f32>(0.0), v / length, length > 0.0);
}

// The next three are `shader.wgsl`'s, which mixes a sprite's texel with
// them: each shader file is a module of its own, so they are written out
// again here. `nova_render::camera::colorized` is what both mirror, and
// the offscreen tests pin each against it.

// Whether an instance's colorize mixes: a level of 1 to 32.
fn mixes(colorize: vec4<f32>) -> bool {
    return colorize.w >= 1.0 && colorize.w <= 32.0;
}

// The 5-bit `s5` mixed toward the colorize's colour, as
// `_BlitPixieTranslucentCopy` does per channel:
// `((s × (32 − m)) >> 5) | ((c × m) >> 5)`, the halves ORed.
fn colorized5(s5: vec3<u32>, colorize: vec4<f32>) -> vec3<u32> {
    let m = u32(colorize.w);
    let c5 = vec3<u32>(colorize.rgb);
    return ((s5 * (32u - m)) >> vec3<u32>(5u)) | ((c5 * m) >> vec3<u32>(5u));
}

// A 5-bit colour widened to 8 bits by bit replication, as 0 to 1.
fn widened(c5: vec3<u32>) -> vec3<f32> {
    return vec3<f32>((c5 << vec3<u32>(3u)) | (c5 >> vec3<u32>(2u))) / 255.0;
}

// The colour times the texel, all four channels; then, unless unlit, its
// red, green and blue times `ambient + (1 − ambient) · max(0, n · l)`;
// then, when the instance's colorize mixes, that colour held to 0 to 1,
// narrowed to 5 bits a channel, mixed and widened; and its alpha times the
// instance's.
@fragment
fn model_fs(in: ModelOut) -> @location(0) vec4<f32> {
    let base = in.color * textureSample(base_color, base_sampler, in.uv);
    let facing = max(dot(unit(in.normal), unit(in.light.xyz)), 0.0);
    let lit = in.light.w + (1.0 - in.light.w) * facing;
    let shown = select(lit, 1.0, in.unlit > 0.5);
    var rgb = base.rgb * shown;
    if mixes(in.colorize) {
        let s5 = vec3<u32>(round(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)) * 255.0)) >> vec3<u32>(3u);
        rgb = widened(colorized5(s5, in.colorize));
    }
    return vec4<f32>(rgb, base.a * in.alpha);
}
