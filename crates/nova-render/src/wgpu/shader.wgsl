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
    // The same for every vertex of a quad; flat, so no interpolation can
    // nudge an OR level off its exact 32nd.
    @location(1) @interpolate(flat) tint: vec4<f32>,
    // The colorize: 5-bit red, green, blue and the level, 0 for none.
    @location(2) @interpolate(flat) colorize: vec4<f32>,
};

// Six vertices per instance: two triangles over the unit square.
@vertex
fn sprite_vs(
    @builtin(vertex_index) index: u32,
    @location(0) dest: vec4<f32>,
    @location(1) uv: vec4<f32>,
    @location(2) tint: vec4<f32>,
    @location(3) colorize: vec4<f32>,
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
    out.colorize = colorize;
    return out;
}

// Whether a quad's colorize mixes: a level of 1 to 32.
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

// A texel, its colour mixed by the colorize when it mixes, its alpha kept.
fn colorized(t: vec4<f32>, colorize: vec4<f32>) -> vec4<f32> {
    let s5 = vec3<u32>(round(t.rgb * 255.0)) >> vec3<u32>(3u);
    return vec4<f32>(widened(colorized5(s5, colorize)), t.a);
}

@fragment
fn sprite_fs(in: SpriteOut) -> @location(0) vec4<f32> {
    let t = textureSample(page, page_sampler, in.uv);
    if mixes(in.colorize) {
        return colorized(t, in.colorize) * in.tint;
    }
    return t * in.tint;
}

// A max batch's colour, premultiplied by its alpha so a transparent texel
// is black and leaves what is beneath as it is: the blend state takes the
// larger of each channel (`Blend::Max`, QuickDraw's `addMax`).
@fragment
fn max_fs(in: SpriteOut) -> @location(0) vec4<f32> {
    let t = textureSample(page, page_sampler, in.uv) * in.tint;
    return vec4<f32>(t.rgb * t.a, t.a);
}

// What an OR batch reads from beneath it: a copy of the scene, at the
// fragment's own pixel.
@group(2) @binding(0) var backdrop: texture_2d<f32>;

// ORs the texel, scaled by the tint's level (n/32 per channel), into the
// backdrop: the original's `((src_c * n) >> 5) | dst_c` with a 5-bit source
// and an 8-bit destination. The arithmetic and why are on `Blend::Or`.
@fragment
fn or_fs(in: SpriteOut) -> @location(0) vec4<f32> {
    let t = textureSampleLevel(page, page_sampler, in.uv, 0.0);
    let d = textureLoad(backdrop, vec2<u32>(in.position.xy), 0);
    // Premultiplied, so a transparent texel contributes nothing; a
    // colorized one is mixed first, still nothing where transparent.
    var s5 = vec3<u32>(round(t.rgb * t.a * 255.0)) >> vec3<u32>(3u);
    if mixes(in.colorize) && t.a > 0.0 {
        let c = colorized(t, in.colorize);
        s5 = vec3<u32>(round(c.rgb * c.a * 255.0)) >> vec3<u32>(3u);
    }
    // n/32 and s5 * n/32 are exact in f32, so this is the original's floor.
    let t5 = vec3<u32>(floor(vec3<f32>(s5) * in.tint.rgb));
    let t8 = (t5 << vec3<u32>(3u)) | (t5 >> vec3<u32>(2u));
    let d8 = vec3<u32>(round(d.rgb * 255.0));
    return vec4<f32>(vec3<f32>(d8 | t8) / 255.0, d.a);
}

// Crossfades the texel with the backdrop by OR, in 5 bits: the original's
// `((src_c * k) >> 5) | ((dst_c * (32 - k)) >> 5)`, k the tint's alpha in
// 32nds and the tint's colour a multiplier on the source. A transparent
// texel or k = 0 leaves the backdrop as it is. The arithmetic and why are
// on `Blend::OrCrossfade`.
@fragment
fn crossfade_fs(in: SpriteOut) -> @location(0) vec4<f32> {
    let t = textureSampleLevel(page, page_sampler, in.uv, 0.0);
    let d = textureLoad(backdrop, vec2<u32>(in.position.xy), 0);
    let k = u32(round(in.tint.a * 32.0));
    if (t.a == 0.0 || k == 0u) {
        return d;
    }
    let s5 = vec3<u32>(round(t.rgb * in.tint.rgb * 255.0)) >> vec3<u32>(3u);
    let d5 = vec3<u32>(round(d.rgb * 255.0)) >> vec3<u32>(3u);
    let c5 = ((s5 * k) >> vec3<u32>(5u)) | ((d5 * (32u - k)) >> vec3<u32>(5u));
    let c8 = (c5 << vec3<u32>(3u)) | (c5 >> vec3<u32>(2u));
    return vec4<f32>(mix(d.rgb, vec3<f32>(c8) / 255.0, t.a), d.a);
}

// The original's translucent copy with its factors set apart:
// `((src_c * n_c) >> 5) | ((dst_c * m) >> 5)`, the tint's colour the source
// factors n_c and its alpha the destination factor m, each in 32nds. A
// transparent texel leaves the backdrop as it is. The arithmetic and why
// are on `Blend::OrTranslucent`.
@fragment
fn translucent_fs(in: SpriteOut) -> @location(0) vec4<f32> {
    let t = textureSampleLevel(page, page_sampler, in.uv, 0.0);
    let d = textureLoad(backdrop, vec2<u32>(in.position.xy), 0);
    if (t.a == 0.0) {
        return d;
    }
    let n = vec3<u32>(round(in.tint.rgb * 32.0));
    let m = u32(round(in.tint.a * 32.0));
    let s5 = vec3<u32>(round(t.rgb * 255.0)) >> vec3<u32>(3u);
    let d5 = vec3<u32>(round(d.rgb * 255.0)) >> vec3<u32>(3u);
    let c5 = ((s5 * n) >> vec3<u32>(5u)) | ((d5 * m) >> vec3<u32>(5u));
    let c8 = (c5 << vec3<u32>(3u)) | (c5 >> vec3<u32>(2u));
    return vec4<f32>(mix(d.rgb, vec3<f32>(c8) / 255.0, t.a), d.a);
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
