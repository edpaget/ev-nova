//! The layered sky's distant backdrop (`nova_render::sky::backdrop`)
//! drawn offscreen through the draw list, in the world layer, in a
//! system's look ([`SkyLooks`]). Every pixel checked is predicted from the
//! core's own instances (`nova_render::sky::instances`), lit by its
//! falloff, never retyped: the default look draws its seeded backdrop
//! under its stars, a look's colour colours the clouds, the backdrop moves
//! with the eye, the murk level dims it, a look with no backdrop draws the
//! stars alone, and the clouds still show at the lowest render scale.
//!
//! Logical 512 × 512 in a 512 × 512 target: a logical unit is a pixel.
//! Needs a GPU adapter; skips, passing, without one.

use nova_render::fonts::FontFaces;
use nova_render::sky::{
    BackdropLook, SkyLook, SkyLooks, StarInstance, backdrop, instances, stars, streak_coverage,
};
use nova_render::{LogicalSize, RenderReport, RenderScale, Renderer, Viewport};
use nova_view::{DrawList, Point, SkyPose};

use super::wgpu_offscreen::{Images, gpu_with};

const SIDE: u32 = 512;
const LOGICAL: LogicalSize = LogicalSize { w: SIDE, h: SIDE };

/// A sky whose seeded backdrop shows clouds in the view from [`EYE`].
const SEED: u64 = 12;

const EYE: (f32, f32) = (1000.0, -500.0);

/// The sky seen from `eye` at `level`.
fn pose(eye: (f32, f32), level: u8) -> SkyPose {
    SkyPose::new(
        SEED,
        Point::new(eye.0, eye.1),
        Point::new(256.0, 256.0),
        level,
    )
}

/// `sky` drawn alone in the world layer at `scale`, in `look` (none: the
/// default), read back; `None` with no adapter.
fn drawn(sky: SkyPose, look: Option<SkyLook>, scale: RenderScale) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(SIDE, SIDE, &FontFaces::bundled())?;
    let mut renderer = Renderer::new(Images);
    if let Some(look) = look {
        let mut looks = SkyLooks::new();
        looks.insert(sky.seed, look);
        renderer.set_sky_looks(looks);
    }
    renderer.set_render_scale(scale);
    let mut list = DrawList::new();
    list.world(|list| {
        list.sky(sky);
    });
    let viewport = Viewport::new(LOGICAL, (SIDE, SIDE), 1.0);
    let report = renderer.render(&list, &viewport, &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

fn rgb(pixels: &[u8], (x, y): (u32, u32)) -> [u8; 3] {
    let at = ((y * SIDE + x) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// The colour the core predicts at pixel `at`'s centre from `drawn`, 0 to
/// 255: the sum of each instance's colour × its falloff there.
fn predicted(drawn: &[StarInstance], (x, y): (u32, u32)) -> [f32; 3] {
    let centre = (x as f32 + 0.5, y as f32 + 0.5);
    let mut sum = [0.0; 3];
    for one in drawn {
        let offset = Point::new(centre.0 - one.centre.x, centre.1 - one.centre.y);
        let lit = streak_coverage(offset, one.trail, one.radius);
        for (channel, c) in sum.iter_mut().zip(one.color) {
            *channel += c * lit;
        }
    }
    sum.map(|c| (c * 255.0).min(255.0))
}

/// The pixels these tests sample: every eighth each way, offset so some
/// land off the grid of the cells.
fn samples() -> impl Iterator<Item = (u32, u32)> {
    (3..SIDE)
        .step_by(8)
        .flat_map(|y| (5..SIDE).step_by(8).map(move |x| (x, y)))
}

/// Every sampled pixel of `pixels` is `drawn`'s prediction, within 3 of
/// 255 a channel.
fn assert_as_predicted(pixels: &[u8], drawn: &[StarInstance], what: &str) {
    for at in samples() {
        let seen = rgb(pixels, at);
        let expected = predicted(drawn, at);
        for (s, e) in seen.iter().zip(expected) {
            assert!(
                (f32::from(*s) - e).abs() <= 3.0,
                "{what} at {at:?}: {seen:?}, predicted {expected:?}"
            );
        }
    }
}

/// How much light `puffs` add at pixel `at`, its brightest channel, 0 to
/// 255.
fn cloud_at(puffs: &[StarInstance], at: (u32, u32)) -> f32 {
    predicted(puffs, at).into_iter().fold(0.0, f32::max)
}

#[test]
fn the_default_look_draws_its_seeded_backdrop_under_its_stars() {
    let sky = pose(EYE, 32);
    let Some(pixels) = drawn(sky, None, RenderScale::Auto) else {
        return;
    };
    let all = instances(&sky, &SkyLook::default(), LOGICAL, 1.0);
    let puffs = backdrop(&sky, &SkyLook::default(), LOGICAL);
    assert!(!puffs.is_empty(), "the seed shows a cloud from here");
    assert_as_predicted(&pixels, &all, "the default sky");
    // Sampled pixels inside clouds and between them.
    let inside = samples().filter(|&at| cloud_at(&puffs, at) > 10.0).count();
    let between = samples().filter(|&at| cloud_at(&puffs, at) == 0.0).count();
    assert!(
        inside > 20 && between > 20,
        "{inside} inside, {between} between"
    );
}

/// A look of `color` clouds in every cell and no stars.
fn clouds_only(color: Option<[f32; 3]>) -> SkyLook {
    SkyLook {
        stars: 0.0,
        tint: None,
        backdrop: BackdropLook {
            color,
            amount: Some(1.0),
        },
    }
}

#[test]
fn a_looks_colour_draws_its_clouds_in_that_colours_channels() {
    let sky = pose(EYE, 32);
    let look = clouds_only(Some([0.8, 0.0, 0.4]));
    let Some(pixels) = drawn(sky, Some(look), RenderScale::Auto) else {
        return;
    };
    let puffs = backdrop(&sky, &look, LOGICAL);
    assert!(!puffs.is_empty());
    assert_as_predicted(&pixels, &puffs, "magenta clouds");
    let mut lit = 0;
    for at in samples() {
        let [r, g, b] = rgb(&pixels, at);
        assert_eq!(g, 0, "at {at:?}");
        assert!(b <= r.div_ceil(2) + 1, "at {at:?}: {r} {b}");
        lit += usize::from(r > 0);
    }
    assert!(lit > 50, "{lit}");
}

#[test]
fn the_moved_eyes_backdrop_is_the_moved_prediction() {
    // Far enough to move the backdrop a hundred units.
    let moved = (EYE.0 + 4000.0, EYE.1 - 2400.0);
    let look = clouds_only(None);
    let Some(before) = drawn(pose(EYE, 32), Some(look), RenderScale::Auto) else {
        return;
    };
    let Some(after) = drawn(pose(moved, 32), Some(look), RenderScale::Auto) else {
        return;
    };
    let puffs = backdrop(&pose(moved, 32), &look, LOGICAL);
    assert_as_predicted(&after, &puffs, "the moved eye");
    assert_ne!(before, after);
}

#[test]
fn murk_level_16_halves_the_backdrops_light() {
    let look = clouds_only(None);
    let Some(clear) = drawn(pose(EYE, 32), Some(look), RenderScale::Auto) else {
        return;
    };
    let Some(half) = drawn(pose(EYE, 16), Some(look), RenderScale::Auto) else {
        return;
    };
    let puffs = backdrop(&pose(EYE, 16), &look, LOGICAL);
    assert_as_predicted(&half, &puffs, "level 16");
    let mut checked = 0;
    for at in samples() {
        for (c, h) in rgb(&clear, at).into_iter().zip(rgb(&half, at)) {
            assert!(
                (f32::from(h) - f32::from(c) / 2.0).abs() <= 1.5,
                "at {at:?}: {c} then {h}"
            );
            checked += usize::from(c > 10);
        }
    }
    assert!(checked > 20, "{checked}");
}

#[test]
fn a_look_with_no_backdrop_draws_the_stars_alone() {
    let sky = pose(EYE, 32);
    let look = SkyLook {
        backdrop: BackdropLook {
            color: None,
            amount: Some(0.0),
        },
        ..SkyLook::default()
    };
    let Some(starry) = drawn(sky, Some(look), RenderScale::Auto) else {
        return;
    };
    assert_as_predicted(&starry, &stars(&sky, LOGICAL, 1.0), "stars alone");
    let Some(seeded) = drawn(sky, None, RenderScale::Auto) else {
        return;
    };
    assert_ne!(starry, seeded, "the default look draws its backdrop too");
}

#[test]
fn at_the_lowest_render_scale_the_backdrop_still_lights_pixels() {
    let sky = pose(EYE, 32);
    let look = clouds_only(None);
    let Some(pixels) = drawn(sky, Some(look), RenderScale::Percent(RenderScale::MIN)) else {
        return;
    };
    let puffs = backdrop(&sky, &look, LOGICAL);
    let inside: Vec<&StarInstance> = puffs
        .iter()
        .filter(|puff| {
            (0.0..SIDE as f32).contains(&puff.centre.x)
                && (0.0..SIDE as f32).contains(&puff.centre.y)
        })
        .collect();
    assert!(!inside.is_empty());
    for puff in inside {
        let at = (puff.centre.x as u32, puff.centre.y as u32);
        assert!(rgb(&pixels, at).iter().any(|&c| c > 0), "{puff:?}");
    }
}
