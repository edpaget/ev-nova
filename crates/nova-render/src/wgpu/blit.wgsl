// Copies the scene a frame was drawn into onto the target, pixel for pixel.

@group(0) @binding(0) var scene: texture_2d<f32>;

// One triangle covering the whole target: clip (-1, -1), (3, -1), (-1, 3).
@vertex
fn blit_vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
}

// The scene's texel under this fragment's pixel centre.
@fragment
fn blit_fs(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return textureLoad(scene, vec2<u32>(position.xy), 0);
}
