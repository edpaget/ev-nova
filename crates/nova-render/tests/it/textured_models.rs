//! A model's base-colour texture and its unlit materials, end to end.
//!
//! - Recorded: a textured, unlit box built by the media fixture, loaded
//!   through [`GalaxyModels`], registered and drawn on [`RecordingGpu`]:
//!   the texture is uploaded decoded, then the mesh naming it, its
//!   vertices carrying the fixture's texture coordinates and the unlit
//!   flag, and the frame draws it.
//! - Offscreen: a quad facing the camera in a 2 by 2 texture of four
//!   colours shows each texel in its quadrant as
//!   [`fragment`](nova_render::camera::fragment) gives it, lit and unlit,
//!   under a white factor and a tinted one; the sampler repeats the
//!   texture for coordinates outside 0 to 1 and filters across the seam;
//!   a fine texture on a small quad shows its mip levels, not its first;
//!   and a cube whose two visible faces the light shades apart draws both
//!   at their own colours when unlit. These need a GPU adapter; they
//!   skip, passing, without one.

use std::f32::consts::FRAC_PI_6;
use std::sync::Arc;

use nova_galaxy_format::fixture::{CC0, galaxy};
use nova_galaxy_format::load::load;
use nova_galaxy_media::fixture::{Builder, ImageAt, Mat, png};
use nova_render::camera::{fragment, shade};
use nova_render::fonts::FontFaces;
use nova_render::model_source::{Material, Part, Texture, vertex_color};
use nova_render::recording::{GpuCall, NoImages, RecordingGpu};
use nova_render::{
    Batch, GalaxyModels, Light, LoadedModel, Mesh, MeshId, ModelRequest, ModelSource, ModelTexture,
    ModelTextureId, RenderReport, Renderer, Section, Vertex, Viewport,
};
use nova_view::{Color, DrawList, ModelKey, ModelPose, Orientation, Point};

use super::common::pixels::{LOGICAL, SIZE, assert_near, color_at};
use super::wgpu_offscreen::{Images, gpu_with};

/// A 2 by 2 image: red and green on the top row, blue and half-clear
/// white on the bottom.
const SKIN: [u8; 16] = [
    230, 40, 40, 255, 40, 200, 60, 255, 50, 70, 240, 255, 255, 255, 255, 128,
];

/// Texel (`column`, `row`) of [`SKIN`], each channel 0 to 1.
fn texel(column: usize, row: usize) -> [f32; 4] {
    let at = (row * 2 + column) * 4;
    [0, 1, 2, 3].map(|channel| f32::from(SKIN[at + channel]) / 255.0)
}

fn skin() -> Texture {
    Texture {
        image: Arc::new(ModelTexture::new(2, 2, SKIN.to_vec()).expect("four texels")),
        path: None,
    }
}

/// A material of the linear `base` colour, drawing texture 0 if
/// `textured`.
fn material(base: [f32; 4], unlit: bool, textured: bool) -> Material {
    Material {
        name: Some("skin".to_owned()),
        paint: None,
        base_color: base,
        metallic: 1.0,
        roughness: 1.0,
        unlit,
        base_color_texture: textured.then_some(0),
    }
}

#[test]
#[allow(
    clippy::float_cmp,
    reason = "the fixture's coordinates are exactly 0 or 1"
)]
fn a_textured_unlit_model_from_data_uploads_its_texture_then_its_mesh_and_draws() {
    const HULL: ModelKey = ModelKey(4);
    // A box 2 wide, 1 tall and 4 long, its nose at −z: 8 m long, so 2 m a
    // unit. The fixture's coordinates are u = 1 on the +x side and v = 1
    // on the +z side, which land to starboard (view +x) and at the tail
    // (view +y).
    let glb = Builder::new([2.0, 1.0, 4.0])
        .texcoords()
        .material(Mat {
            textured: true,
            unlit: true,
            ..Mat::new("hull", [1.0; 4])
        })
        .image(ImageAt::Embedded)
        .image_bytes(&png(2, 2, &SKIN))
        .glb();
    let source = galaxy()
        .with_bytes("media/export/pack/hull.glb", &glb)
        .with(
            "assets/hull.md",
            &format!(
                "---\nkind: model\nplaceholder: false\nfiles: [media/export/pack/hull.glb]\n\
                 front: -z\n{CC0}---\n"
            ),
        );
    let (galaxy, _) = load(&source);
    let loaded = GalaxyModels::new(&galaxy, &source)
        .model(&ModelRequest::new("hull", Some(8.0)))
        .expect("the box loads");

    let mut renderer = Renderer::new(NoImages);
    renderer.add_model(HULL, loaded.mesh());
    let mut list = DrawList::new();
    list.model(
        HULL,
        ModelPose {
            center: Point::new(16.0, 12.0),
            orientation: Orientation::IDENTITY,
            scale: 1.0,
        },
    );
    let mut gpu = RecordingGpu::new();
    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    assert_eq!(report, RenderReport::default());

    let [
        GpuCall::UploadModelTexture { texture, data },
        GpuCall::UploadMesh {
            mesh,
            data: uploaded,
            textures,
        },
        GpuCall::Submit(frame),
    ] = gpu.calls.as_slice()
    else {
        panic!("a texture, a mesh and a frame: {:?}", gpu.calls);
    };
    // The texture, decoded: its first level is the PNG's four colours.
    assert_eq!(*texture, ModelTextureId(0));
    let first = &data.levels()[0];
    assert_eq!((first.width, first.height), (2, 2));
    assert_eq!(first.rgba, SKIN);
    assert_eq!(data.levels().len(), 2, "and its one-texel mip");
    // The mesh names it: one section, all twelve triangles.
    assert_eq!(*mesh, MeshId(0));
    assert_eq!(textures, &[ModelTextureId(0)]);
    assert_eq!(
        uploaded.sections(),
        [Section {
            indices: 0..36,
            texture: Some(0),
        }]
    );
    // Each vertex is unlit and carries the fixture's coordinates.
    assert!(!uploaded.vertices().is_empty());
    let mut corners = Vec::new();
    for vertex in uploaded.vertices() {
        assert!(vertex.unlit, "{vertex:?}");
        let [u, v] = vertex.uv;
        let expected = [vertex.position[0] > 0.0, vertex.position[1] > 0.0].map(f32::from);
        assert_eq!([u, v], expected, "{vertex:?}");
        if !corners.contains(&[u.to_bits(), v.to_bits()]) {
            corners.push([u.to_bits(), v.to_bits()]);
        }
    }
    assert_eq!(corners.len(), 4, "all four corners of the texture");
    // The frame draws the mesh.
    let [Batch::Models { models, .. }] = frame.batches.as_slice() else {
        panic!("one models batch: {:?}", frame.batches);
    };
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].mesh, MeshId(0));
}

/// A quad facing the camera, a unit a side, in a material of the linear
/// `base` colour on `texture`, its texture coordinates running from
/// `from` at its top left to `to` at its bottom right, each way.
fn quad_on(texture: Texture, (from, to): (f32, f32), base: [f32; 4], unlit: bool) -> LoadedModel {
    let color = vertex_color(base);
    let corners = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
    let vertices = corners
        .iter()
        .map(|&(x, y)| Vertex {
            position: [x, y, 0.0],
            normal: [0.0, 0.0, 1.0],
            color,
            uv: [0.0; 2],
            unlit: false,
        })
        .collect();
    let uv = corners
        .iter()
        .map(|&(x, y)| [x + 0.5, y + 0.5].map(|across| from + across * (to - from)))
        .collect();
    LoadedModel {
        parts: vec![Part {
            mesh: Mesh::new(vertices, vec![0, 1, 2, 0, 2, 3]).expect("two triangles"),
            material: Some(0),
            uv: Some(uv),
        }],
        materials: vec![material(base, unlit, true)],
        textures: vec![texture],
    }
}

/// The quad on [`SKIN`], once across it: the texture's top-left texel at
/// its top left.
fn quad(base: [f32; 4], unlit: bool) -> LoadedModel {
    quad_on(skin(), (0.0, 1.0), base, unlit)
}

/// The quads' side, in logical units: 18 pixels, so each quadrant's
/// middle is a pixel's centre, where the sampler reads one texel alone.
const SIDE: f32 = 9.0;
/// The lit and the unlit quad's centres.
const LIT_AT: (f32, f32) = (6.5, 12.5);
const UNLIT_AT: (f32, f32) = (22.5, 12.5);

/// Each of `quads` (a model, its centre and its side in logical units)
/// drawn offscreen facing the camera, and read back; `None` with no
/// adapter.
fn drawn(quads: &[(LoadedModel, (f32, f32), f32)]) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(SIZE, SIZE, &FontFaces::bundled())?;
    let mut renderer = Renderer::new(Images);
    let mut list = DrawList::new();
    for (key, (model, (x, y), side)) in (1..).map(ModelKey).zip(quads) {
        renderer.add_model(key, model.mesh());
        list.model(
            key,
            ModelPose {
                center: Point::new(*x, *y),
                orientation: Orientation::IDENTITY,
                scale: *side,
            },
        );
    }
    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

/// The lit and the unlit quad of `base`, side by side, drawn offscreen
/// and read back; `None` with no adapter.
fn quads_drawn(base: [f32; 4]) -> Option<Vec<u8>> {
    drawn(&[
        (quad(base, false), LIT_AT, SIDE),
        (quad(base, true), UNLIT_AT, SIDE),
    ])
}

/// What a fragment of `color` shows over the black the frame is cleared
/// to: its red, green and blue times its alpha, as bytes.
fn over_black([red, green, blue, alpha]: [f32; 4]) -> [u8; 3] {
    [red, green, blue].map(|c| (c * alpha * 255.0).round() as u8)
}

/// Asserts each quadrant of the lit and the unlit quad in `pixels` shows
/// its texel as the core's fragment function gives it under `base`.
fn assert_quadrants(pixels: &[u8], base: [f32; 4]) {
    let facing = [0.0, 0.0, 1.0];
    for (unlit, (x, y)) in [(false, LIT_AT), (true, UNLIT_AT)] {
        let mut shown = Vec::new();
        for (column, row) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let expected = over_black(fragment(
                facing,
                vertex_color(base),
                texel(column, row),
                unlit,
                Light::STANDARD,
            ));
            // A quadrant's middle: a quarter of the side either way.
            let at = |centre: f32, side: usize| centre + (side as f32 - 0.5) * SIDE / 2.0;
            let middle = (at(x, column), at(y, row));
            let what = format!("texel ({column}, {row}), unlit {unlit}, base {base:?}");
            assert_near(pixels, middle, expected, &what);
            shown.push(expected);
        }
        shown.dedup();
        assert_eq!(shown.len(), 4, "four colours apart: {shown:?}");
    }
    // Lit and unlit show the same texel apart.
    let (lit, unlit) = (
        color_at(pixels, LIT_AT.0 - SIDE / 4.0, LIT_AT.1 - SIDE / 4.0),
        color_at(pixels, UNLIT_AT.0 - SIDE / 4.0, UNLIT_AT.1 - SIDE / 4.0),
    );
    assert_ne!(lit, unlit, "the light dims the lit quad");
}

#[test]
fn a_textured_quad_shows_each_texel_in_its_quadrant_lit_and_unlit() {
    let white = [1.0; 4];
    let Some(pixels) = quads_drawn(white) else {
        return;
    };
    assert_quadrants(&pixels, white);
    // Unlit under a white factor, the texture's own bytes.
    let (x, y) = UNLIT_AT;
    let top_left = (x - SIDE / 4.0, y - SIDE / 4.0);
    assert_near(&pixels, top_left, [230, 40, 40], "the red texel as it is");
    // Outside the quads, nothing.
    assert_eq!(color_at(&pixels, 14.5, 12.5), [0, 0, 0]);
}

#[test]
fn a_tinted_factor_multiplies_the_texture() {
    let tint = [0.5, 1.0, 0.25, 1.0];
    let Some(pixels) = quads_drawn(tint) else {
        return;
    };
    assert_quadrants(&pixels, tint);
    // The tint shows: the red texel's red is dimmer than under white.
    let (x, y) = UNLIT_AT;
    let [red, _, _] = color_at(&pixels, x - SIDE / 4.0, y - SIDE / 4.0);
    assert!(red < 200, "{red}");
}

#[test]
fn texture_coordinates_outside_zero_to_one_repeat_the_texture() {
    let white = [1.0; 4];
    // The lit quad a whole turn past the texture, the unlit one a whole
    // turn before it: each quadrant still shows its own texel, where a
    // clamping sampler would smear an edge texel over the quad.
    let Some(pixels) = drawn(&[
        (quad_on(skin(), (1.0, 2.0), white, false), LIT_AT, SIDE),
        (quad_on(skin(), (-1.0, 0.0), white, true), UNLIT_AT, SIDE),
    ]) else {
        return;
    };
    assert_quadrants(&pixels, white);

    // Across the seam the sampler filters the first texel with the last,
    // wrapped round. Two texels cover the quad's 18 pixels, so the centre
    // of its leftmost pixel is 1/18 of a texel past the seam: 5/9 of the
    // first texel and 4/9 of the last. Read on the top quadrants' middle
    // row, where only the top row of texels is sampled.
    let (first, last) = (texel(0, 0), texel(1, 0));
    let blend = [0, 1, 2, 3].map(|c| (5.0 * first[c] + 4.0 * last[c]) / 9.0);
    let expected = over_black(blend);
    let (x, y) = UNLIT_AT;
    let got = color_at(&pixels, x - SIDE / 2.0 + 0.25, y - SIDE / 4.0);
    assert!(
        within(got, expected, FILTERED),
        "the seam's blend: {got:?}, not {expected:?}"
    );
    for alone in [first, last] {
        assert!(
            !within(got, over_black(alone), 40),
            "one texel alone: {got:?}"
        );
    }
}

/// How far a channel filtered between two texels may be from the blend
/// worked out here: a GPU keeps a filter's weights to a few bits.
const FILTERED: u8 = 4;

/// Whether each channel of `a` is within `tolerance` of `b`'s.
fn within(a: [u8; 3], b: [u8; 3], tolerance: u8) -> bool {
    a.iter().zip(&b).all(|(p, q)| p.abs_diff(*q) <= tolerance)
}

/// The two colours of [`checker`], which average to (120, 40, 120).
const CHECKS: [[u8; 4]; 2] = [[240, 40, 0, 255], [0, 40, 240, 255]];

/// An 8 by 8 texture of one-texel checks in [`CHECKS`].
fn checker() -> Texture {
    let rgba = (0..8)
        .flat_map(|row| (0..8).flat_map(move |column| CHECKS[(row + column) % 2]))
        .collect();
    Texture {
        image: Arc::new(ModelTexture::new(8, 8, rgba).expect("64 texels")),
        path: None,
    }
}

#[test]
fn a_fine_texture_on_a_small_quad_shows_its_mips_average() {
    // Two pixels a side over eight texels: four texels a pixel, which
    // the 2 by 2 mip level answers. Every level under the first is the
    // checks' average all over.
    const SMALL: f32 = 1.0;
    const AT: (f32, f32) = (16.0, 12.0);
    let texture = checker();
    let levels = texture.image.levels();
    assert_eq!(levels.len(), 4);
    let average = [120, 40, 120];
    for level in &levels[1..] {
        for texel in level.rgba.as_chunks::<4>().0 {
            assert_eq!(*texel, [120, 40, 120, 255], "level {}", level.width);
        }
    }
    // Half a texel along, each pixel's centre is a texel's centre in the
    // first level: read from it alone, the pixel would be one check.
    let half = 1.0 / 16.0;
    let quad = quad_on(texture, (half, 1.0 + half), [1.0; 4], true);
    let Some(pixels) = drawn(&[(quad, AT, SMALL)]) else {
        return;
    };
    for (dx, dy) in [(-0.25, -0.25), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)] {
        let at = (AT.0 + dx, AT.1 + dy);
        assert_near(&pixels, at, average, "the checks' average");
        let got = color_at(&pixels, at.0, at.1);
        for [red, green, blue, _] in CHECKS {
            assert!(!within(got, [red, green, blue], 40), "one check: {got:?}");
        }
    }
    // Beside the quad, nothing.
    assert_eq!(color_at(&pixels, AT.0 + 0.75, AT.1), [0, 0, 0]);
}

/// The cube's faces' colours, in `Mesh::cube`'s order: +x, −x, +y, −y,
/// +z, −z.
const FACES: [[u8; 3]; 6] = [
    [0, 255, 255],
    [255, 0, 255],
    [0, 200, 0],
    [255, 255, 0],
    [230, 40, 40],
    [40, 40, 255],
];
const PLUS_Y: usize = 2;
const PLUS_Z: usize = 4;

/// `world_models`' turn: 30° about x, so +z and +y face the camera, lit
/// apart.
fn turn() -> Orientation {
    Orientation::about([1.0, 0.0, 0.0], FRAC_PI_6)
}

/// The cube in [`FACES`], one part in an untextured material, unlit or
/// lit.
fn cube(unlit: bool) -> LoadedModel {
    let faces = FACES.map(|[r, g, b]| Color::rgba(r, g, b, 255));
    LoadedModel {
        parts: vec![Part {
            mesh: Mesh::cube(faces),
            material: Some(0),
            uv: None,
        }],
        materials: vec![material([1.0; 4], unlit, false)],
        textures: Vec::new(),
    }
}

/// The cube at (12, 12), 10 logical units a side, turned: +z covers
/// logical y 5.17 to 13.83 and +y 13.83 to 18.83, x 7 to 17 for each.
/// Drawn offscreen and read back; `None` with no adapter.
fn cube_drawn(unlit: bool) -> Option<Vec<u8>> {
    const CUBE: ModelKey = ModelKey(1);
    let mut gpu = gpu_with(SIZE, SIZE, &FontFaces::bundled())?;
    let mut renderer = Renderer::new(Images);
    renderer.add_model(CUBE, cube(unlit).mesh());
    let mut list = DrawList::new();
    list.model(
        CUBE,
        ModelPose {
            center: Point::new(12.0, 12.0),
            orientation: turn(),
            scale: 10.0,
        },
    );
    let report = renderer.render(&list, &Viewport::new(LOGICAL, (SIZE, SIZE), 2.0), &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

#[test]
fn an_unlit_cube_draws_each_face_at_its_own_colour_whatever_way_it_faces() {
    const ON_PLUS_Z: (f32, f32) = (10.5, 9.5);
    const ON_PLUS_Y: (f32, f32) = (12.5, 16.5);
    let (Some(unlit), Some(lit)) = (cube_drawn(true), cube_drawn(false)) else {
        return;
    };
    assert_near(&unlit, ON_PLUS_Z, FACES[PLUS_Z], "+z unlit");
    assert_near(&unlit, ON_PLUS_Y, FACES[PLUS_Y], "+y unlit");

    // Lit, the same cube is shaded, each face by the way it faces.
    let shaded = |face: usize, normal: [f32; 3]| {
        let normal = turn().rotate(normal).expect("a unit turn");
        let [r, g, b] = FACES[face].map(|c| f32::from(c) / 255.0);
        over_black(shade(normal, [r, g, b, 1.0], Light::STANDARD))
    };
    let (plus_z, plus_y) = (
        shaded(PLUS_Z, [0.0, 0.0, 1.0]),
        shaded(PLUS_Y, [0.0, 1.0, 0.0]),
    );
    assert_near(&lit, ON_PLUS_Z, plus_z, "+z lit");
    assert_near(&lit, ON_PLUS_Y, plus_y, "+y lit");
    assert_ne!(plus_z, FACES[PLUS_Z], "the light dims +z");
    assert_ne!(plus_y, FACES[PLUS_Y], "the light dims +y");
}
