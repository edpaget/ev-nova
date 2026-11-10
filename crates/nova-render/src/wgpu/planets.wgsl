// Planets: one square per planet, each fragment mirroring
// `nova_render::planet`: where the view ray meets the globe, the ring
// plane and the atmosphere, which surface texel it shows, and the
// shadows. It decides nothing the core hasn't defined.

struct Globals {
    // The logical space's width and height.
    logical: vec2<f32>,
    pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;

@group(1) @binding(0) var surface_tex: texture_2d<f32>;
@group(1) @binding(1) var clouds_tex: texture_2d<f32>;
@group(1) @binding(2) var rings_tex: texture_2d<f32>;
// Repeats in u, clamps in v, trilinear.
@group(1) @binding(3) var wrap: sampler;
// Clamps, linear.
@group(1) @binding(4) var clamp_linear: sampler;

const PI: f32 = 3.14159265358979;
const TAU: f32 = 6.28318530717959;
// `planet::RIM`.
const RIM: f32 = 0.6;

// Logical units (y down) to clip space (y up).
fn to_clip(p: vec2<f32>) -> vec4<f32> {
    let unit = p / globals.logical;
    return vec4<f32>(unit.x * 2.0 - 1.0, 1.0 - unit.y * 2.0, 0.0, 1.0);
}

struct PlanetOut {
    @builtin(position) position: vec4<f32>,
    // Where the fragment is, in planet radii from the centre.
    @location(0) local: vec2<f32>,
    // North, then the surface's turn.
    @location(1) @interpolate(flat) north: vec4<f32>,
    // The equator's point facing the camera, then the clouds' turn.
    @location(2) @interpolate(flat) facing: vec4<f32>,
    // East.
    @location(3) @interpolate(flat) east: vec4<f32>,
    // The rings' inner and outer radii.
    @location(4) @interpolate(flat) rings: vec4<f32>,
    @location(5) @interpolate(flat) ring_color: vec4<f32>,
    // The atmosphere's colour and thickness.
    @location(6) @interpolate(flat) atmosphere: vec4<f32>,
    // Toward the light, then the ambient.
    @location(7) @interpolate(flat) light: vec4<f32>,
};

@vertex
fn planet_vs(
    @builtin(vertex_index) index: u32,
    @location(0) dest: vec4<f32>,
    @location(1) north: vec4<f32>,
    @location(2) facing: vec4<f32>,
    @location(3) east: vec4<f32>,
    @location(4) rings: vec4<f32>,
    @location(5) ring_color: vec4<f32>,
    @location(6) atmosphere: vec4<f32>,
    @location(7) light: vec4<f32>,
) -> PlanetOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let corner = corners[index];
    var out: PlanetOut;
    out.position = to_clip(dest.xy + corner * dest.zw);
    // `rings.z` is the reach.
    out.local = (corner * 2.0 - 1.0) * rings.z;
    out.north = north;
    out.facing = facing;
    out.east = east;
    out.rings = rings;
    out.ring_color = ring_color;
    out.atmosphere = atmosphere;
    out.light = light;
    return out;
}

// `planet::lit`.
fn lit(n: vec3<f32>, light: vec4<f32>) -> f32 {
    return light.w + (1.0 - light.w) * max(dot(n, light.xyz), 0.0);
}

// `planet::rim`.
fn rim(d: f32, thickness: f32) -> f32 {
    if (thickness <= 0.0 || d >= 1.0 + thickness) {
        return 0.0;
    }
    if (d <= 1.0) {
        let d2 = d * d;
        return RIM * d2 * d2;
    }
    let fade = 1.0 - (d - 1.0) / thickness;
    return RIM * fade * fade;
}

// `RingBand::opacity`: the profile read linearly between its texels, the
// first at `inner` and the last at `outer`; 0 outside.
fn ring_opacity(r: f32, rings: vec4<f32>) -> f32 {
    if (rings.y <= 0.0 || r < rings.x || r > rings.y) {
        return 0.0;
    }
    let n = f32(textureDimensions(rings_tex).x);
    let at = (r - rings.x) / (rings.y - rings.x) * (n - 1.0);
    return textureSampleLevel(rings_tex, clamp_linear, vec2<f32>((at + 0.5) / n, 0.5), 0.0).r;
}

// `planet::in_planet_shadow`.
fn in_planet_shadow(p: vec3<f32>, l: vec3<f32>) -> bool {
    let along = dot(p, l);
    return along < 0.0 && along * along - (dot(p, p) - 1.0) >= 0.0;
}

// `a` over `under`, premultiplied.
fn over(under: vec4<f32>, color: vec3<f32>, alpha: f32) -> vec4<f32> {
    return vec4<f32>(color * alpha + under.rgb * (1.0 - alpha), alpha + under.a * (1.0 - alpha));
}

@fragment
fn planet_fs(in: PlanetOut) -> @location(0) vec4<f32> {
    let x = in.local.x;
    let y = in.local.y;
    let d = length(in.local);
    let north = in.north.xyz;
    let l = normalize(in.light.xyz);
    let light = vec4<f32>(l, in.light.w);

    // `planet::globe`, clamped so its derivatives exist everywhere.
    let n = vec3<f32>(x, y, sqrt(max(1.0 - x * x - y * y, 0.0)));
    let on_disc = 1.0 - x * x - y * y >= 0.0;
    // `planet::surface_uv`, with gradients taken from a continuous
    // longitude, so no mip seam shows where u wraps. `turned` (in
    // [-0.5, 0.5]) jumps on the meridian opposite `facing`, `wrapped` (in
    // [0, 1)) on `facing`'s own, half a turn away; in each 2 by 2 quad the
    // smaller gradient of the two is the one that doesn't cross a jump.
    let latitude = asin(clamp(dot(n, north), -1.0, 1.0));
    let turned = atan2(dot(n, in.east.xyz), dot(n, in.facing.xyz)) / TAU;
    let v = 0.5 - latitude / PI;
    let u = fract(turned - in.north.w);
    let cloud_u = fract(turned - in.facing.w);
    let wrapped = fract(turned);
    let du_plain = vec2<f32>(dpdx(turned), dpdy(turned));
    let du_wrapped = vec2<f32>(dpdx(wrapped), dpdy(wrapped));
    let du = select(du_plain, du_wrapped, abs(du_wrapped) < abs(du_plain));
    let dv = vec2<f32>(dpdx(v), dpdy(v));
    let ddx = vec2<f32>(du.x, dv.x);
    let ddy = vec2<f32>(du.y, dv.y);
    let ground = textureSampleGrad(surface_tex, wrap, vec2<f32>(u, v), ddx, ddy);
    let cloud = textureSampleGrad(clouds_tex, wrap, vec2<f32>(cloud_u, v), ddx, ddy);
    let edge = max(fwidth(d), 1e-6);

    var acc = vec4<f32>(0.0);

    // The halo, outside the limb, lit as the limb under it is.
    let thickness = in.atmosphere.w;
    if (!on_disc && d > 0.0) {
        let limb = vec3<f32>(x / d, y / d, 0.0);
        acc = over(acc, in.atmosphere.rgb * lit(limb, light), rim(d, thickness));
    }

    // The globe: lit, its glow unlit, its clouds over it, its rim tinted.
    if (on_disc || d < 1.0 + edge) {
        var shade = lit(n, light);
        // `planet::ring_shadow`: the rings shade where its sunward ray
        // crosses them.
        let toward = dot(l, north);
        if (abs(toward) > 1e-6) {
            let t = -dot(n, north) / toward;
            if (t > 0.0) {
                let q = n + t * l;
                shade = light.w + (shade - light.w) * (1.0 - ring_opacity(length(q), in.rings));
            }
        }
        let surface = mix(ground.rgb * shade, ground.rgb, ground.a);
        let clouded = mix(surface, vec3<f32>(shade), cloud.a);
        let tinted = mix(clouded, in.atmosphere.rgb * shade, rim(min(d, 1.0), thickness));
        let coverage = clamp((1.0 - d) / edge + 0.5, 0.0, 1.0);
        acc = over(acc, tinted, coverage);
    }

    // `planet::ring_hit`: the rings where they are in front of the globe.
    if (abs(north.z) > 1e-6) {
        let z = -(x * north.x + y * north.y) / north.z;
        let p = vec3<f32>(x, y, z);
        let in_front = !on_disc || z > n.z;
        let opacity = ring_opacity(length(p), in.rings);
        if (in_front && opacity > 0.0) {
            let shade = select(1.0, light.w, in_planet_shadow(p, l));
            acc = over(acc, in.ring_color.rgb * shade, opacity);
        }
    }

    if (acc.a <= 0.0) {
        discard;
    }
    return vec4<f32>(acc.rgb / acc.a, acc.a);
}
