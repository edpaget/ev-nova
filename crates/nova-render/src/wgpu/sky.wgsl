// The layered sky: one rectangle per star, covering its streak and its
// radius round it, each fragment adding the star's colour times
// `nova_render::sky::streak_coverage` there. It decides nothing the core
// hasn't defined.

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

// `sky::streak_coverage`: `coverage` of the distance from the segment
// from the centre along `trail`, in radii.
fn streak_coverage(offset: vec2<f32>, trail: vec2<f32>, radius: f32) -> f32 {
    let span = dot(trail, trail);
    var along = 0.0;
    if span > 0.0 {
        along = clamp(dot(offset, trail) / span, 0.0, 1.0);
    }
    return coverage(length(offset - trail * along) / radius);
}

struct StarOut {
    @builtin(position) position: vec4<f32>,
    // Where the fragment is from the centre, in logical units.
    @location(0) offset: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
    // The trail, then the radius.
    @location(2) @interpolate(flat) streak: vec4<f32>,
};

@vertex
fn star_vs(
    @builtin(vertex_index) index: u32,
    // The centre, the radius, then the trail's x.
    @location(0) place: vec4<f32>,
    // The colour, then the trail's y.
    @location(1) color: vec4<f32>,
) -> StarOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let centre = place.xy;
    let radius = place.z;
    let trail = vec2<f32>(place.w, color.w);
    let lo = min(centre, centre + trail) - vec2<f32>(radius);
    let hi = max(centre, centre + trail) + vec2<f32>(radius);
    let at = mix(lo, hi, corners[index]);
    var out: StarOut;
    out.position = to_clip(at);
    out.offset = at - centre;
    out.color = vec4<f32>(color.rgb, 0.0);
    out.streak = vec4<f32>(trail, radius, 0.0);
    return out;
}

@fragment
fn star_fs(in: StarOut) -> @location(0) vec4<f32> {
    let lit = streak_coverage(in.offset, in.streak.xy, in.streak.z);
    return vec4<f32>(in.color.rgb * lit, 0.0);
}
