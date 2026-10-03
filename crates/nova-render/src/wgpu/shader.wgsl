// Sprites and solids in the logical space, projected onto the viewport.

struct Globals {
    // The logical space's width and height.
    logical: vec2<f32>,
    pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;

// Logical units (y down) to clip space (y up).
fn to_clip(p: vec2<f32>) -> vec4<f32> {
    let unit = p / globals.logical;
    return vec4<f32>(unit.x * 2.0 - 1.0, 1.0 - unit.y * 2.0, 0.0, 1.0);
}

@group(1) @binding(0) var page: texture_2d<f32>;
@group(1) @binding(1) var page_sampler: sampler;

struct SpriteOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

// Six vertices per instance: two triangles over the unit square.
@vertex
fn sprite_vs(
    @builtin(vertex_index) index: u32,
    @location(0) dest: vec4<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) tint: vec4<f32>,
) -> SpriteOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let corner = corners[index];
    var out: SpriteOut;
    out.position = to_clip(dest.xy + corner * dest.zw);
    out.uv = mix(uv.xy, uv.zw, corner);
    out.tint = tint;
    return out;
}

@fragment
fn sprite_fs(in: SpriteOut) -> @location(0) vec4<f32> {
    return textureSample(page, page_sampler, in.uv) * in.tint;
}

struct SolidOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn solid_vs(@location(0) position: vec2<f32>, @location(1) color: vec4<f32>) -> SolidOut {
    var out: SolidOut;
    out.position = to_clip(position);
    out.color = color;
    return out;
}

@fragment
fn solid_fs(in: SolidOut) -> @location(0) vec4<f32> {
    return in.color;
}
