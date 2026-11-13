//! A model's base-colour sampler, coordinate set and texture transform,
//! end to end.
//!
//! - Recorded: fixture boxes loaded through [`GalaxyModels`], registered
//!   and drawn on [`RecordingGpu`]. A base colour that names `TEXCOORD_1`
//!   uploads a mesh carrying that set; one with a `KHR_texture_transform`
//!   uploads the transformed coordinates; a sampler of its own reaches
//!   the mesh's section; two materials on one image through two samplers
//!   upload one texture and two sections; and a second frame uploads
//!   nothing, so an adapter has nothing to make on a frame.
//! - Offscreen: each pixel is compared with the core's reference,
//!   [`sample`] times [`fragment`]. A clamped axis shows the edge texel
//!   past the image and a mirrored one the image reversed, each axis
//!   apart; nearest magnification shows one texel either side of a seam
//!   the default sampler blends across; a sampler with no mips shows the
//!   first level's check where a mipped one shows the average; a box
//!   sampled by its second set shows the image transposed, and a
//!   transformed one the transformed image. These need a GPU adapter;
//!   they skip, passing, without one.
//!
//! The helpers that draw a quad and read it back are
//! [`textured_models`](super::textured_models)'.

use std::f32::consts::FRAC_PI_2;

use nova_galaxy_format::fixture::{CC0, galaxy};
use nova_galaxy_format::load::load;
use nova_galaxy_media::fixture::{Builder, ImageAt, Mat, SamplerCodes, TransformJson, code, png};
use nova_galaxy_media::sampling::Transform;
use nova_render::camera::fragment;
use nova_render::model_sampler::{Filter, Mip, Wrap, sample};
use nova_render::model_source::Texture;
use nova_render::recording::{GpuCall, NoImages, RecordingGpu};
use nova_render::{
    GalaxyModels, Light, LoadedModel, Mesh, ModelRequest, ModelSampler, ModelSource,
    ModelTextureId, RenderReport, Renderer, Section, Viewport,
};
use nova_view::{DrawList, ModelKey, ModelPose, Orientation, Point};

use super::common::pixels::{LOGICAL, SIZE, color_at};
use super::textured_models::{
    CHECKS, FILTERED, LIT_AT, SIDE, SKIN, UNLIT_AT, checker, drawn, over_black, quad_on, skin,
    texel, within,
};

/// A box 2 wide, 1 tall and 4 long with both coordinate sets, in one
/// unlit white material on its image, sampled as `material` says.
fn boxed(material: Mat) -> Builder {
    Builder::new([2.0, 1.0, 4.0])
        .texcoords()
        .texcoords1()
        .material(Mat {
            textured: true,
            unlit: true,
            ..material
        })
}

fn plain() -> Mat {
    Mat::new("hull", [1.0; 4])
}

/// The boxes' image: [`SKIN`] with its last texel opaque. A box's bottom
/// face is drawn under its top one, so a half-clear texel on the top
/// would show the bottom's through it.
fn paint() -> [u8; 16] {
    let mut rgba = SKIN;
    rgba[15] = 255;
    rgba
}

/// The boxes' image as a texture.
fn painted() -> Texture {
    Texture {
        image: std::sync::Arc::new(
            nova_render::ModelTexture::new(2, 2, paint().to_vec()).expect("four texels"),
        ),
        path: None,
    }
}

/// What texel (`column`, `row`) of the boxes' image shows alone on an
/// unlit white box.
fn paint_alone(column: usize, row: usize) -> [u8; 3] {
    let at = (row * 2 + column) * 4;
    [0, 1, 2].map(|channel| paint()[at + channel])
}

/// `builder`'s model with [`paint`] its image, loaded through
/// [`GalaxyModels`] as asset `hull`, its nose at −z and 8 m long: 2 m a
/// unit, so 4 wide and 8 long, its top toward the camera.
fn hull(builder: Builder) -> LoadedModel {
    let glb = builder
        .image(ImageAt::Embedded)
        .image_bytes(&png(2, 2, &paint()))
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
    GalaxyModels::new(&galaxy, &source)
        .model(&ModelRequest::new("hull", Some(8.0)))
        .expect("the box loads")
}

/// The calls each of `frames` frames drawing `model` makes on a
/// recording GPU, frame by frame.
fn recorded(model: &LoadedModel, frames: usize) -> Vec<Vec<GpuCall>> {
    const HULL: ModelKey = ModelKey(4);
    let mut renderer = Renderer::new(NoImages);
    renderer.add_model(HULL, model.mesh());
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
    (0..frames)
        .map(|_| {
            let viewport = Viewport::new(LOGICAL, (SIZE, SIZE), 2.0);
            let report = renderer.render(&list, &viewport, &mut gpu);
            assert_eq!(report, RenderReport::default());
            std::mem::take(&mut gpu.calls)
        })
        .collect()
}

/// The one mesh `calls` upload, and how many textures they upload.
fn uploaded(calls: &[GpuCall]) -> (&Mesh, usize) {
    let meshes: Vec<&Mesh> = calls
        .iter()
        .filter_map(|call| match call {
            GpuCall::UploadMesh { data, .. } => Some(data),
            _ => None,
        })
        .collect();
    let textures = calls
        .iter()
        .filter(|call| matches!(call, GpuCall::UploadModelTexture { .. }))
        .count();
    assert_eq!(meshes.len(), 1, "one mesh: {calls:?}");
    (meshes[0], textures)
}

/// The fixture's first coordinate set at a vertex of the loaded box: u
/// is 1 on the +x side, which lands to starboard (view +x), and v is 1
/// on the +z side, which lands at the tail (view +y).
fn first_set(position: [f32; 3]) -> [f32; 2] {
    [position[0] > 0.0, position[1] > 0.0].map(f32::from)
}

#[test]
#[allow(
    clippy::float_cmp,
    reason = "the fixture's coordinates are exactly 0 or 1"
)]
fn a_base_colour_naming_the_second_set_uploads_a_mesh_carrying_it() {
    let second = hull(boxed(Mat {
        tex_coord: 1,
        ..plain()
    }));
    let frames = recorded(&second, 1);
    let (mesh, textures) = uploaded(&frames[0]);
    assert_eq!(textures, 1);
    assert_eq!(mesh.sections(), [Section::new(0..36, Some(0))]);
    // The second set is the first with u and v swapped.
    let mut differ = 0;
    for vertex in mesh.vertices() {
        let [u, v] = first_set(vertex.position);
        assert_eq!(vertex.uv, [v, u], "{vertex:?}");
        differ += usize::from(u != v);
    }
    assert!(differ > 0, "the sets differ at some vertex");
    // The same box sampled by its first set carries that one.
    let first = hull(boxed(plain()));
    let frames = recorded(&first, 1);
    for vertex in uploaded(&frames[0]).0.vertices() {
        assert_eq!(vertex.uv, first_set(vertex.position), "{vertex:?}");
    }
}

/// A transform that moves every corner of the unit square: it reads
/// `u' = 0.5 + 3v`, `v' = 1 − u`.
const TURNED: Transform = Transform {
    offset: [0.5, 1.0],
    rotation: FRAC_PI_2,
    scale: [1.0, 3.0],
};

/// The box whose base colour is transformed by [`TURNED`].
fn turned() -> Builder {
    boxed(Mat {
        transform: Some(TransformJson {
            offset: Some(TURNED.offset),
            rotation: Some(TURNED.rotation),
            scale: Some(TURNED.scale),
            tex_coord: None,
        }),
        ..plain()
    })
}

#[test]
fn a_texture_transform_uploads_a_mesh_carrying_the_transformed_coordinates() {
    let frames = recorded(&hull(turned()), 1);
    let (mesh, _) = uploaded(&frames[0]);
    assert!(!mesh.vertices().is_empty());
    for vertex in mesh.vertices() {
        let [u, v] = first_set(vertex.position);
        let expected = [0.5 + 3.0 * v, 1.0 - u];
        let close = (0..2).all(|k| (vertex.uv[k] - expected[k]).abs() < 1e-5);
        assert!(close, "{vertex:?}, not {expected:?}");
        assert!(
            (vertex.uv[0] - u).abs() > 0.4,
            "not the coordinate as written: {vertex:?}"
        );
    }
}

/// A sampler that clamps along u, mirrors along v and reads the nearest
/// texel, as the fixture writes it and as the port says it.
fn sharp() -> (SamplerCodes, ModelSampler) {
    let codes = SamplerCodes {
        wrap_s: Some(code::CLAMP_TO_EDGE),
        wrap_t: Some(code::MIRRORED_REPEAT),
        mag: Some(code::NEAREST),
        min: Some(code::NEAREST),
    };
    let sampler = ModelSampler {
        wrap_s: Wrap::Clamp,
        wrap_t: Wrap::Mirror,
        mag: Filter::Nearest,
        min: Filter::Nearest,
        mip: Mip::None,
    };
    (codes, sampler)
}

#[test]
fn a_sampler_of_its_own_reaches_the_mesh_s_section_with_one_texture_uploaded() {
    let (codes, sampler) = sharp();
    let one = hull(boxed(Mat {
        sampler: Some(codes),
        ..plain()
    }));
    let frames = recorded(&one, 1);
    let (mesh, textures) = uploaded(&frames[0]);
    assert_eq!(textures, 1);
    assert_eq!(
        mesh.sections(),
        [Section::new(0..36, Some(0)).sampled(sampler)]
    );

    // Two materials on the one image, one sampled the default way: one
    // texture uploaded, and a section for each naming it.
    let two = hull(
        boxed(Mat {
            sampler: Some(codes),
            ..plain()
        })
        .material(Mat {
            textured: true,
            ..Mat::new("again", [1.0; 4])
        }),
    );
    let frames = recorded(&two, 1);
    let [
        GpuCall::UploadModelTexture { texture, .. },
        GpuCall::UploadMesh { data, textures, .. },
        GpuCall::Submit(_),
    ] = frames[0].as_slice()
    else {
        panic!("a texture, a mesh and a frame: {:?}", frames[0]);
    };
    assert_eq!(*texture, ModelTextureId(0));
    assert_eq!(textures, &[ModelTextureId(0)]);
    assert_eq!(
        data.sections(),
        [
            Section::new(0..18, Some(0)).sampled(sampler),
            Section::new(18..36, Some(0)),
        ]
    );
}

#[test]
fn a_second_frame_of_a_model_with_two_samplers_uploads_nothing() {
    let two = hull(
        boxed(Mat {
            sampler: Some(sharp().0),
            ..plain()
        })
        .material(Mat {
            textured: true,
            ..Mat::new("again", [1.0; 4])
        }),
    );
    let frames = recorded(&two, 3);
    assert_eq!(frames[0].len(), 3, "a texture, a mesh and a frame");
    for later in &frames[1..] {
        // The samplers came with the mesh: a frame has nothing to hand
        // an adapter but itself.
        assert!(
            matches!(later.as_slice(), [GpuCall::Submit(_)]),
            "{later:?}"
        );
    }
}

/// What a fragment of an unlit white model shows over black where
/// `sampler` reads `texture` at `uv` and the level of detail `lod`: the
/// core's reference.
fn reference(texture: &Texture, sampler: ModelSampler, uv: [f32; 2], lod: f32) -> [u8; 3] {
    let texel = sample(&texture.image, sampler, uv, lod);
    over_black(fragment(
        [0.0, 0.0, 1.0],
        [1.0; 4],
        texel,
        true,
        Light::STANDARD,
    ))
}

/// What texel (`column`, `row`) of [`SKIN`] shows alone on an unlit
/// white model.
fn alone(column: usize, row: usize) -> [u8; 3] {
    over_black(texel(column, row))
}

/// The unlit white quad on `texture`, its coordinates running `range`
/// each way, sampled through `sampler`.
fn quad(texture: Texture, range: (f32, f32), sampler: ModelSampler) -> LoadedModel {
    let mut model = quad_on(texture, range, [1.0; 4], true);
    model.materials[0].base_color_sampler = sampler;
    model
}

/// A level of detail well under 0: the 2 by 2 [`SKIN`] over 18 pixels is
/// magnified.
const MAGNIFIED: f32 = -3.0;

/// Asserts each quadrant's middle of the [`SIDE`] quad at `at`, on
/// [`SKIN`] over `range` through `sampler`, shows what the reference
/// gives there, and that this is the texel `expect` names for its
/// (column, row), alone. Where that isn't the quadrant's own texel, the
/// pixel isn't what repeating would show.
#[track_caller]
fn assert_quadrants(
    pixels: &[u8],
    at: (f32, f32),
    sampler: ModelSampler,
    (from, to): (f32, f32),
    expect: impl Fn(usize, usize) -> (usize, usize),
) {
    for (column, row) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        // A quadrant's middle: a quarter of the side either way.
        let across = |side: usize| 0.25 + 0.5 * side as f32;
        let uv = [across(column), across(row)].map(|a| from + a * (to - from));
        let middle = |centre: f32, side: usize| centre + (across(side) - 0.5) * SIDE;
        let got = color_at(pixels, middle(at.0, column), middle(at.1, row));
        let what = format!("quadrant ({column}, {row}) through {sampler:?} over {from} to {to}");
        let expected = reference(&skin(), sampler, uv, MAGNIFIED);
        assert!(
            within(got, expected, 2),
            "{what}: {got:?}, not {expected:?}"
        );
        let (texel_column, texel_row) = expect(column, row);
        assert_eq!(
            expected,
            alone(texel_column, texel_row),
            "{what}: the reference reads texel ({texel_column}, {texel_row})"
        );
        if (texel_column, texel_row) != (column, row) {
            assert!(
                !within(got, alone(column, row), 40),
                "{what}: {got:?} is what repeating shows"
            );
        }
    }
}

/// The default sampler with `wrap_s` and `wrap_t`.
fn wrapping(wrap_s: Wrap, wrap_t: Wrap) -> ModelSampler {
    ModelSampler {
        wrap_s,
        wrap_t,
        ..ModelSampler::DEFAULT
    }
}

#[test]
fn a_clamped_axis_shows_the_edge_texel_past_the_image_and_the_other_axis_repeats() {
    let along_u = wrapping(Wrap::Clamp, Wrap::Repeat);
    let along_v = wrapping(Wrap::Repeat, Wrap::Clamp);
    // A whole turn past the image: the right-hand column in both halves,
    // top over bottom; and the bottom row, left beside right.
    let past = (1.0, 2.0);
    let Some(pixels) = drawn(&[
        (quad(skin(), past, along_u), LIT_AT, SIDE),
        (quad(skin(), past, along_v), UNLIT_AT, SIDE),
    ]) else {
        return;
    };
    assert_quadrants(&pixels, LIT_AT, along_u, past, |_, row| (1, row));
    assert_quadrants(&pixels, UNLIT_AT, along_v, past, |column, _| (column, 1));
    // A whole turn before it: the opposite edges.
    let before = (-1.0, 0.0);
    let Some(pixels) = drawn(&[
        (quad(skin(), before, along_u), LIT_AT, SIDE),
        (quad(skin(), before, along_v), UNLIT_AT, SIDE),
    ]) else {
        return;
    };
    assert_quadrants(&pixels, LIT_AT, along_u, before, |_, row| (0, row));
    assert_quadrants(&pixels, UNLIT_AT, along_v, before, |column, _| (column, 0));
}

#[test]
fn a_mirrored_axis_shows_the_image_reversed_past_its_edge() {
    let both = wrapping(Wrap::Mirror, Wrap::Mirror);
    let along_u = wrapping(Wrap::Mirror, Wrap::Repeat);
    let past = (1.0, 2.0);
    let Some(pixels) = drawn(&[
        (quad(skin(), past, both), LIT_AT, SIDE),
        (quad(skin(), past, along_u), UNLIT_AT, SIDE),
    ]) else {
        return;
    };
    assert_quadrants(&pixels, LIT_AT, both, past, |column, row| {
        (1 - column, 1 - row)
    });
    assert_quadrants(&pixels, UNLIT_AT, along_u, past, |column, row| {
        (1 - column, row)
    });
    // Along v alone, and a second turn past, where the image is the
    // right way round again.
    let along_v = wrapping(Wrap::Repeat, Wrap::Mirror);
    let further = (2.0, 3.0);
    let Some(pixels) = drawn(&[
        (quad(skin(), past, along_v), LIT_AT, SIDE),
        (quad(skin(), further, both), UNLIT_AT, SIDE),
    ]) else {
        return;
    };
    assert_quadrants(&pixels, LIT_AT, along_v, past, |column, row| {
        (column, 1 - row)
    });
    assert_quadrants(&pixels, UNLIT_AT, both, further, |column, row| {
        (column, row)
    });
}

#[test]
fn nearest_magnification_shows_one_texel_either_side_of_a_seam() {
    let nearest = ModelSampler {
        mag: Filter::Nearest,
        ..ModelSampler::DEFAULT
    };
    let once = (0.0, 1.0);
    let Some(pixels) = drawn(&[
        (quad(skin(), once, nearest), LIT_AT, SIDE),
        (quad(skin(), once, ModelSampler::DEFAULT), UNLIT_AT, SIDE),
    ]) else {
        return;
    };
    // The two pixels either side of the quad's middle, on the top
    // quadrants' middle row: their centres are 1/18 of a texel from the
    // seam between the first texel and the second.
    for (side, column) in [(-0.25, 0), (0.25, 1)] {
        let uv = [0.5 + side / SIDE, 0.25];
        let at = |(x, y): (f32, f32)| color_at(&pixels, x + side, y - SIDE / 4.0);
        let sharp = at(LIT_AT);
        assert_eq!(reference(&skin(), nearest, uv, MAGNIFIED), alone(column, 0));
        assert!(
            within(sharp, alone(column, 0), 2),
            "nearest, column {column}: {sharp:?}"
        );
        // The default sampler blends the two there.
        let soft = at(UNLIT_AT);
        let blend = reference(&skin(), ModelSampler::DEFAULT, uv, MAGNIFIED);
        assert!(
            within(soft, blend, FILTERED),
            "linear, column {column}: {soft:?}, not {blend:?}"
        );
        for texel in [alone(0, 0), alone(1, 0)] {
            assert!(!within(soft, texel, 40), "one texel alone: {soft:?}");
        }
    }

    // Across the repeating seam, where the default sampler blends the
    // first texel with the last wrapped round, nearest shows the first.
    let before = (-1.0, 0.0);
    let Some(pixels) = drawn(&[(quad(skin(), before, nearest), UNLIT_AT, SIDE)]) else {
        return;
    };
    let (x, y) = UNLIT_AT;
    let leftmost = color_at(&pixels, x - SIDE / 2.0 + 0.25, y - SIDE / 4.0);
    assert!(within(leftmost, alone(0, 0), 2), "{leftmost:?}");
}

/// A sampler that reads `min` within a level and `mip` between them.
fn minifying(min: Filter, mip: Mip) -> ModelSampler {
    ModelSampler {
        min,
        mip,
        ..ModelSampler::DEFAULT
    }
}

/// The level of detail of the 8 by 8 checker on a quad two pixels a
/// side: four texels a pixel.
const FOUR_TEXELS_A_PIXEL: f32 = 2.0;

/// The small quads' side in logical units (two pixels), and their
/// centres.
const SMALL: f32 = 1.0;
const SMALL_AT: [(f32, f32); 3] = [(8.0, 12.0), (16.0, 12.0), (24.0, 12.0)];

/// Asserts the four pixels of the small quad at `at`, on the checker
/// with its coordinates starting `start` of the way along, show what
/// the reference gives through `sampler`, which is `expected`.
#[track_caller]
fn assert_small(
    pixels: &[u8],
    at: (f32, f32),
    sampler: ModelSampler,
    start: f32,
    expected: [u8; 3],
) {
    for (dx, dy) in [(-0.25, -0.25), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)] {
        let uv = [start + 0.5 + dx, start + 0.5 + dy];
        let core = reference(&checker(), sampler, uv, FOUR_TEXELS_A_PIXEL);
        assert!(
            within(core, expected, 1),
            "the reference through {sampler:?}: {core:?}, not {expected:?}"
        );
        let got = color_at(pixels, at.0 + dx, at.1 + dy);
        assert!(
            within(got, core, FILTERED),
            "{sampler:?} at ({dx}, {dy}): {got:?}, not {core:?}"
        );
    }
}

#[test]
fn a_sampler_with_no_mips_shows_the_first_level_where_a_mipped_one_shows_the_average() {
    let [check, _] = CHECKS.map(|[red, green, blue, _]| [red, green, blue]);
    let average = [120, 40, 120];
    // Half a texel along, each pixel's centre is a texel's centre in the
    // first level, the same check under all four pixels.
    let half = 1.0 / 16.0;
    let range = (half, 1.0 + half);
    let samplers = [
        minifying(Filter::Nearest, Mip::None),
        minifying(Filter::Linear, Mip::None),
        minifying(Filter::Linear, Mip::Nearest),
    ];
    let quads: Vec<_> = samplers
        .iter()
        .zip(SMALL_AT)
        .map(|(sampler, at)| (quad(checker(), range, *sampler), at, SMALL))
        .collect();
    let Some(pixels) = drawn(&quads) else {
        return;
    };
    assert_small(&pixels, SMALL_AT[0], samplers[0], half, check);
    assert_small(&pixels, SMALL_AT[1], samplers[1], half, check);
    assert_small(&pixels, SMALL_AT[2], samplers[2], half, average);
    assert!(!within(check, average, 40));

    // A quarter of a texel further, the nearest filter still reads one
    // check, and the linear one blends it with its neighbours: 5/8 of the
    // one and 3/8 of the other.
    let off = half + 1.0 / 32.0;
    let range = (off, 1.0 + off);
    let quads: Vec<_> = samplers[..2]
        .iter()
        .zip(SMALL_AT)
        .map(|(sampler, at)| (quad(checker(), range, *sampler), at, SMALL))
        .collect();
    let Some(pixels) = drawn(&quads) else {
        return;
    };
    assert_small(&pixels, SMALL_AT[0], samplers[0], off, check);
    assert_small(&pixels, SMALL_AT[1], samplers[1], off, [150, 40, 90]);
}

/// Where two boxes are drawn, side by side: each quadrant of a box's top
/// face (4 by 8 logical units) has a pixel's centre at its middle.
const LEFT: (f32, f32) = (8.25, 12.25);
const RIGHT: (f32, f32) = (24.25, 12.25);

/// The colour at the middle of quadrant (`column`, `row`) of the top
/// face of the box drawn at `at`.
fn on_top(pixels: &[u8], at: (f32, f32), column: usize, row: usize) -> [u8; 3] {
    let side = |side: usize| side as f32 - 0.5;
    color_at(pixels, at.0 + 2.0 * side(column), at.1 + 4.0 * side(row))
}

/// The first set's coordinates at the middle of quadrant (`column`,
/// `row`) of the box's top face: u across, v toward the tail.
fn top_uv(column: usize, row: usize) -> [f32; 2] {
    [column, row].map(|side| 0.25 + 0.5 * side as f32)
}

#[test]
fn a_box_sampled_by_its_second_set_shows_the_image_transposed() {
    let first = hull(boxed(plain()));
    let second = hull(boxed(Mat {
        tex_coord: 1,
        ..plain()
    }));
    let Some(pixels) = drawn(&[(first, LEFT, 1.0), (second, RIGHT, 1.0)]) else {
        return;
    };
    for (column, row) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let what = format!("quadrant ({column}, {row})");
        let upright = on_top(&pixels, LEFT, column, row);
        assert!(
            within(upright, paint_alone(column, row), 2),
            "{what} by the first set: {upright:?}"
        );
        let transposed = on_top(&pixels, RIGHT, column, row);
        assert!(
            within(transposed, paint_alone(row, column), 2),
            "{what} by the second set: {transposed:?}"
        );
        let [u, v] = top_uv(column, row);
        let core = reference(&painted(), ModelSampler::DEFAULT, [v, u], MAGNIFIED);
        assert_eq!(core, paint_alone(row, column), "{what}");
    }
    // Green and blue change places.
    assert!(within(on_top(&pixels, RIGHT, 1, 0), paint_alone(0, 1), 2));
    assert!(!within(paint_alone(1, 0), paint_alone(0, 1), 40));
}

#[test]
fn a_transformed_box_shows_the_image_at_the_transformed_coordinates() {
    let Some(pixels) = drawn(&[
        (hull(boxed(plain())), LEFT, 1.0),
        (hull(turned()), RIGHT, 1.0),
    ]) else {
        return;
    };
    for (column, row) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let what = format!("quadrant ({column}, {row})");
        let uv = top_uv(column, row);
        let moved = TURNED.apply(uv);
        let core = reference(&painted(), ModelSampler::DEFAULT, moved, MAGNIFIED);
        // `u' = 0.5 + 3v`, `v' = 1 − u`: the texel in column `row` and
        // row `1 − column`.
        assert_eq!(core, paint_alone(row, 1 - column), "{what}");
        let got = on_top(&pixels, RIGHT, column, row);
        assert!(within(got, core, 2), "{what}: {got:?}, not {core:?}");
        // Not what the coordinates as written show, which the box
        // beside it does.
        let untransformed = reference(&painted(), ModelSampler::DEFAULT, uv, MAGNIFIED);
        assert!(!within(got, untransformed, 40), "{what}: {got:?}");
        let beside = on_top(&pixels, LEFT, column, row);
        assert!(within(beside, untransformed, 2), "{what}: {beside:?}");
    }
}
