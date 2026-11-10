// The layered sky: one square per star, each fragment adding the star's
// colour times `nova_render::sky::coverage` of its distance from the
// centre, in radii. It decides nothing the core hasn't defined.

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

// `sky::coverage`: 1 at the centre, falling smoothly to 0 at the edge.
fn coverage(t: f32) -> f32 {
    if t >= 1.0 {
        return 0.0;
    }
    let inside = 1.0 - t * t;
    return inside * inside;
}

struct StarOut {
    @builtin(position) position: vec4<f32>,
    // Where the fragment is, in radii from the centre.
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
};

@vertex
fn star_vs(
    @builtin(vertex_index) index: u32,
    // The centre, the radius, then 0.
    @location(0) place: vec4<f32>,
    @location(1) color: vec4<f32>,
) -> StarOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let corner = corners[index];
    var out: StarOut;
    out.position = to_clip(place.xy + corner * place.z);
    out.local = corner;
    out.color = color;
    return out;
}

@fragment
fn star_fs(in: StarOut) -> @location(0) vec4<f32> {
    let lit = coverage(length(in.local));
    return vec4<f32>(in.color.rgb * lit, 0.0);
}
