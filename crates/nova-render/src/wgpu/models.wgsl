// 3D models: each vertex through its instance's clip_from_model, each
// fragment lit as `nova_render::camera::shade` defines, from the
// interpolated normal on the view's axes.

struct ModelOut {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    // The batch's light: toward it, then the ambient.
    @location(2) @interpolate(flat) light: vec4<f32>,
};

@vertex
fn model_vs(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) c0: vec4<f32>,
    @location(4) c1: vec4<f32>,
    @location(5) c2: vec4<f32>,
    @location(6) c3: vec4<f32>,
    @location(7) n0: vec4<f32>,
    @location(8) n1: vec4<f32>,
    @location(9) n2: vec4<f32>,
    @location(10) light: vec4<f32>,
) -> ModelOut {
    let clip_from_model = mat4x4<f32>(c0, c1, c2, c3);
    let normal_from_model = mat3x3<f32>(n0.xyz, n1.xyz, n2.xyz);
    var out: ModelOut;
    out.position = clip_from_model * vec4<f32>(position, 1.0);
    out.normal = normal_from_model * normal;
    out.color = color;
    out.light = light;
    return out;
}

// `v` as a unit vector, or zero when it has no length.
fn unit(v: vec3<f32>) -> vec3<f32> {
    let length = length(v);
    return select(vec3<f32>(0.0), v / length, length > 0.0);
}

// The colour times `ambient + (1 − ambient) · max(0, n · l)`, alpha kept.
@fragment
fn model_fs(in: ModelOut) -> @location(0) vec4<f32> {
    let facing = max(dot(unit(in.normal), unit(in.light.xyz)), 0.0);
    let lit = in.light.w + (1.0 - in.light.w) * facing;
    return vec4<f32>(in.color.rgb * lit, in.color.a);
}
