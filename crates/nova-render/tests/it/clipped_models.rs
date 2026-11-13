//! A clipped model and a clipped planet cut where a clipped sprite is
//! ([`DrawCommand::Clipped`]): a cube, a 4x4 sprite and a planet, one
//! above another, each crossing the clip's right edge at x = 16, which is
//! target column 32.
//!
//! - On [`RecordingGpu`]: the sprite's quad is cropped to the edge, and
//!   the models and planets batches carry the clip whose scissor ends on
//!   the same column; unclipped, the same batches carry none.
//! - Offscreen: each piece's last lit column is 31, the cube's and the
//!   planet's as the sprite's; unclipped they reach past it. In a world
//!   drawn into a texture at half the window's resolution the cube is cut
//!   in the texture's texels, at the same edge. Needs a GPU adapter;
//!   skips, passing, without one.

use nova_galaxy_media::planet::{Image as Layer, Surface};
use nova_render::fonts::FontFaces;
use nova_render::recording::RecordingGpu;
use nova_render::{
    Batch, Gpu, LoadedPlanet, Mesh, Rect, RenderReport, RenderScale, Renderer, Upscaler, Viewport,
    WorldSpace,
};
use nova_view::geometry::Bounds;
use nova_view::{
    Color, DrawCommand, DrawList, ImageKey, ModelKey, ModelPose, Orientation, PlanetKey,
    PlanetPose, Point,
};

use super::common::pixels::{LOGICAL, SIZE, pixel};
use super::wgpu_offscreen::{Images, gpu_with};

const CUBE: ModelKey = ModelKey(1);
const GLOBE: PlanetKey = PlanetKey(1);

/// The clip: the left half of the logical space, its right edge x = 16.
const CLIP: Bounds = Bounds {
    min: Point { x: 0.0, y: 0.0 },
    max: Point { x: 16.0, y: 24.0 },
};
/// The clip's right edge's target column: two pixels a unit.
const EDGE: u32 = 32;

const BACKGROUND: [u8; 4] = [0, 0, 0, 255];

/// The target rows each piece fills at the clip's edge: the cube's
/// (logical y 2 to 8), the sprite's (11 to 13) and the planet's (16 to
/// 22), each inside its piece by a unit or so.
const CUBE_ROWS: std::ops::Range<u32> = 12..24;
const SPRITE_ROWS: std::ops::Range<u32> = 30..34;
const PLANET_ROWS: std::ops::Range<u32> = 40..52;

/// The cube centred on (16, 5), 8 units a side, facing the camera.
fn cube_pose() -> ModelPose {
    ModelPose {
        center: Point::new(16.0, 5.0),
        orientation: Orientation::IDENTITY,
        // The cube's side is 1: 8 units across.
        scale: 8.0,
    }
}

/// A grey planet of radius 4 centred on (16, 19).
fn planet_pose() -> PlanetPose {
    PlanetPose {
        center: Point::new(16.0, 19.0),
        radius: 4.0,
        time: 0.0,
    }
}

fn grey_planet() -> LoadedPlanet {
    LoadedPlanet::surface(Surface {
        surface: Layer {
            width: 4,
            height: 2,
            rgba: [200, 200, 200, 255].repeat(8),
        },
        clouds: None,
        rings: None,
        atmosphere: None,
        day: 100.0,
        tilt: 0.0,
    })
}

/// The three pieces, each clipped to [`CLIP`] when `clipped`: the cube,
/// the white `rlëD` 2 sprite centred on (16, 12), and the planet.
fn pieces(clipped: bool) -> DrawList {
    let mut list = DrawList::new();
    let commands = [
        DrawCommand::model(CUBE, cube_pose()),
        DrawCommand::Sprite {
            image: ImageKey::sprite(2, 0),
            center: Point::new(16.0, 12.0),
            tint: Color::WHITE,
            blend: nova_view::Blend::Normal,
        },
        DrawCommand::Planet {
            planet: GLOBE,
            pose: planet_pose(),
        },
    ];
    for command in commands {
        if clipped {
            list.clipped(CLIP, command);
        } else {
            list.push(command);
        }
    }
    list
}

fn renderer() -> Renderer<Images> {
    let mut renderer = Renderer::new(Images);
    renderer.add_model(CUBE, Mesh::cube([Color::WHITE; 6]));
    renderer.add_planet(GLOBE, grey_planet());
    renderer
}

fn viewport() -> Viewport {
    Viewport::new(LOGICAL, (SIZE, SIZE), 2.0)
}

/// `batch` with no clip.
fn unclipped(batch: &Batch) -> Batch {
    match batch.clone() {
        Batch::Models { light, models, .. } => Batch::Models {
            light,
            models,
            clip: None,
        },
        Batch::Planets { light, planets, .. } => Batch::Planets {
            light,
            planets,
            clip: None,
        },
        other => other,
    }
}

/// The models and planets batches of `batches`, in order.
fn models_and_planets(batches: &[Batch]) -> Vec<&Batch> {
    (batches.iter())
        .filter(|batch| matches!(batch.kind(), "models" | "planets"))
        .collect()
}

#[test]
fn a_clipped_model_and_planet_carry_the_clip_a_clipped_sprite_is_cropped_to() {
    let mut gpu = RecordingGpu::new();
    let report = renderer().render(&pieces(true), &viewport(), &mut gpu);
    assert_eq!(report, RenderReport::default());
    let frame = gpu.submits()[0].clone();
    let kinds: Vec<&str> = frame.batches.iter().map(Batch::kind).collect();
    assert_eq!(kinds, ["models", "sprites", "planets"]);

    let Batch::Sprites { quads, .. } = &frame.batches[1] else {
        panic!("{kinds:?}");
    };
    let right = quads[0].dest.x + quads[0].dest.w;
    assert!(
        (right - 16.0).abs() < 1e-6,
        "the sprite's crop ends at {right}"
    );

    let clip = Rect {
        x: 0.0,
        y: 0.0,
        w: 16.0,
        h: 24.0,
    };
    assert_eq!(frame.batches[0].clip(), Some(clip));
    assert_eq!(frame.batches[2].clip(), Some(clip));
    let scissor = frame.clip_scissor_in(WorldSpace::Target, clip);
    assert_eq!(scissor.x + scissor.w, frame.viewport.x + 16 * 2);
    assert_eq!(scissor.x + scissor.w, EDGE);

    let mut plain = RecordingGpu::new();
    renderer().render(&pieces(false), &viewport(), &mut plain);
    let plain = plain.submits()[0].clone();
    let cut = models_and_planets(&frame.batches);
    let whole = models_and_planets(&plain.batches);
    assert_eq!(cut.len(), 2);
    assert!(whole.iter().all(|batch| batch.clip().is_none()));
    let cut: Vec<Batch> = cut.into_iter().map(unclipped).collect();
    let whole: Vec<Batch> = whole.into_iter().cloned().collect();
    assert_eq!(cut, whole, "the same batches but for the clip");
}

/// `list` drawn under `upscaler` at `scale`, read back; `None` with no
/// adapter.
fn drawn(list: &DrawList, upscaler: Upscaler, scale: RenderScale) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(SIZE, SIZE, &FontFaces::bundled())?;
    gpu.set_upscaler(upscaler);
    let mut renderer = renderer();
    renderer.set_render_scale(scale);
    let report = renderer.render(list, &viewport(), &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

/// The last column of row `row` of `pixels` that isn't background.
fn last_lit(pixels: &[u8], row: u32) -> Option<u32> {
    (0..SIZE)
        .rev()
        .find(|&column| pixel(pixels, (column, row)) != BACKGROUND)
}

#[test]
fn a_clipped_model_and_planet_are_cut_at_the_column_a_clipped_sprite_is() {
    let Some(pixels) = drawn(&pieces(true), Upscaler::Native, RenderScale::Auto) else {
        return;
    };
    for (piece, rows) in [
        ("the cube", CUBE_ROWS),
        ("the sprite", SPRITE_ROWS),
        ("the planet", PLANET_ROWS),
    ] {
        for row in rows {
            assert_eq!(last_lit(&pixels, row), Some(EDGE - 1), "{piece} row {row}");
        }
    }
}

#[test]
fn unclipped_the_model_and_planet_reach_past_the_edge() {
    let Some(pixels) = drawn(&pieces(false), Upscaler::Native, RenderScale::Auto) else {
        return;
    };
    for (piece, rows) in [
        ("the cube", CUBE_ROWS),
        ("the sprite", SPRITE_ROWS),
        ("the planet", PLANET_ROWS),
    ] {
        for row in rows {
            let last = last_lit(&pixels, row).expect("drawn");
            assert!(last > EDGE, "{piece} row {row} ends at {last}");
        }
    }
}

#[test]
fn a_clipped_world_model_is_cut_at_the_clip_in_a_half_scale_texture_world() {
    let whole = Bounds {
        min: Point::new(0.0, 0.0),
        max: Point::new(32.0, 24.0),
    };
    let mut list = DrawList::new();
    list.world_in(whole, |list| {
        list.clipped(CLIP, DrawCommand::model(CUBE, cube_pose()));
    });
    // Half the 64x48 viewport: a 32x24 texture, a texel a unit, so the
    // clip's edge is texel 16, enlarged onto column 32.
    let Some(pixels) = drawn(&list, Upscaler::SharpBilinear, RenderScale::percent(50)) else {
        return;
    };
    for row in CUBE_ROWS {
        let end = last_lit(&pixels, row).expect("drawn");
        // The upscaler blends a texel across the edge, no more.
        assert!((29..34).contains(&end), "row {row} ends at {end}");
        assert_ne!(pixel(&pixels, (29, row)), BACKGROUND, "row {row}");
    }
}
