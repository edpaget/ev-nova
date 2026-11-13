//! A sky's decorations (`nova_render::sky::SkyDecoration`) drawn offscreen
//! through the draw list, in the world layer, from a system's list
//! ([`SkyLooks::set_decorations`]). Every pixel checked is predicted by
//! the core: the sky beneath from its own instances
//! (`nova_render::sky::instances`), then each decoration's light and
//! cover ([`lit`]) laid over it ([`over`]), never retyped. Pixels within a
//! pixel of a quad's edge are left out, as the rasteriser decides those.
//!
//! Logical 256 × 256 in a 256 × 256 target: a logical unit is a pixel.
//! Needs a GPU adapter; skips, passing, without one.

use std::sync::Arc;

use nova_render::fonts::FontFaces;
use nova_render::sky::{
    BackdropLook, Placement, SkyBlend, SkyDecoration, SkyLook, SkyLooks, StarInstance, backdrop,
    instances, lit, over, placed, streak_coverage,
};
use nova_render::{LogicalSize, RenderReport, RenderScale, Renderer, SkyImage, Viewport};
use nova_view::{DrawList, Point, SkyPose};

use super::wgpu_offscreen::{Images, gpu_with};

const SIDE: u32 = 256;
const LOGICAL: LogicalSize = LogicalSize { w: SIDE, h: SIDE };
const SEED: u64 = 12;

/// The sky seen from over the system's centre at `level`.
fn pose(level: u8) -> SkyPose {
    SkyPose::new(SEED, Point::new(0.0, 0.0), Point::new(128.0, 128.0), level)
}

/// A look that draws no star and no backdrop.
fn dark() -> SkyLook {
    SkyLook {
        stars: 0.0,
        tint: None,
        backdrop: BackdropLook {
            color: None,
            amount: Some(0.0),
        },
    }
}

/// A busy sky: twice the stars, a cloud in every cell.
fn busy() -> SkyLook {
    SkyLook {
        stars: 2.0,
        tint: None,
        backdrop: BackdropLook {
            color: Some([0.4, 0.6, 1.0]),
            amount: Some(1.0),
        },
    }
}

/// A `side` by `side` image, each texel its own colour, of alpha `alpha`
/// but for a clear top-left quarter when `cut`.
fn picture(side: u32, alpha: u8, cut: bool) -> Arc<SkyImage> {
    let mut rgba = Vec::new();
    for y in 0..side {
        for x in 0..side {
            let clear = cut && x < side / 2 && y < side / 2;
            let (r, g, b) = (
                40 + (x * 200 / side) as u8,
                230 - (y * 180 / side) as u8,
                90 + ((x + y) % 4) as u8 * 40,
            );
            rgba.extend([r, g, b, if clear { 0 } else { alpha }]);
        }
    }
    Arc::new(SkyImage::new(side, side, rgba).expect("whole rows"))
}

/// `sky` drawn alone in the world layer at `scale`, in `look` with the
/// decorations `list`, read back; `None` with no adapter.
fn drawn(
    sky: SkyPose,
    look: SkyLook,
    list: &[SkyDecoration],
    scale: RenderScale,
) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(SIDE, SIDE, &FontFaces::bundled())?;
    let mut renderer = Renderer::new(Images);
    let mut looks = SkyLooks::new();
    looks.insert(sky.seed, look);
    looks.set_decorations(sky.seed, list.to_vec());
    renderer.set_sky_looks(looks);
    renderer.set_render_scale(scale);
    let mut draws = DrawList::new();
    draws.world(|draws| {
        draws.sky(sky);
    });
    let viewport = Viewport::new(LOGICAL, (SIDE, SIDE), 1.0);
    let report = renderer.render(&draws, &viewport, &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

fn rgb(pixels: &[u8], (x, y): (u32, u32)) -> [u8; 3] {
    let at = ((y * SIDE + x) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// The centre of pixel `at`, in logical units.
fn centre((x, y): (u32, u32)) -> Point {
    Point::new(x as f32 + 0.5, y as f32 + 0.5)
}

/// The sky the core predicts beneath the decorations at pixel `at`'s
/// centre, 0 to 1: the sum of each of `stars`' (the sky's stars and
/// puffs) colour by its falloff.
fn sky_at(stars: &[StarInstance], at: (u32, u32)) -> [f32; 3] {
    let point = centre(at);
    let mut sum = [0.0; 3];
    for one in stars {
        let offset = Point::new(point.x - one.centre.x, point.y - one.centre.y);
        let light = streak_coverage(offset, one.trail, one.radius);
        for (channel, c) in sum.iter_mut().zip(one.color) {
            *channel += c * light;
        }
    }
    sum.map(|c| c.min(1.0))
}

/// A sky at `level` whose busy look shows a cloud behind the middle of
/// the view: the first seed with a large puff centred there. It is seen
/// from an eye that puts a backdrop cell's corner at the view's, as no
/// cloud reaches a corner of its cell, which an eye over the system's
/// centre draws at the middle.
fn cloudy_pose(level: u8) -> SkyPose {
    let eye = Point::new(5120.0, 5120.0);
    (0..2000)
        .map(|seed| SkyPose::new(seed, eye, Point::new(128.0, 128.0), level))
        .find(|sky| {
            backdrop(sky, &busy(), LOGICAL).iter().any(|puff| {
                puff.radius > 90.0 && (puff.centre.x - 128.0).hypot(puff.centre.y - 128.0) < 28.0
            })
        })
        .expect("a seed with a cloud behind the middle of the view")
}

/// `placement` with each edge `by` units further from its centre.
fn grown(placement: &Placement, by: f32) -> Placement {
    let longer = |edge: Point| {
        let length = edge.x.hypot(edge.y);
        let scale = (length + by) / length;
        Point::new(edge.x * scale, edge.y * scale)
    };
    Placement {
        right: longer(placement.right),
        down: longer(placement.down),
        ..*placement
    }
}

/// What the core predicts at pixel `at`, 0 to 255: the sky of `stars`,
/// then each of `list`'s decorations as `drawn` places them, laid over
/// it in draw order; and whether a decoration lit or covered it. `None`
/// for a pixel within a pixel of a quad's edge.
fn predicted(
    stars: &[StarInstance],
    list: &[SkyDecoration],
    drawn: &[(usize, Placement)],
    at: (u32, u32),
) -> Option<([f32; 3], bool)> {
    let point = centre(at);
    let mut pixel = sky_at(stars, at);
    let mut touched = false;
    for (index, placement) in drawn {
        let image = &list[*index].image;
        let inside = lit(image, &grown(placement, -1.0), point).is_some();
        let near = lit(image, &grown(placement, 1.0), point).is_some();
        if inside != near {
            return None;
        }
        if let Some(light) = lit(image, placement, point) {
            pixel = over(pixel, light).map(|c| c.min(1.0));
            touched = true;
        }
    }
    Some((pixel.map(|c| c * 255.0), touched))
}

/// Every pixel of `pixels` away from a quad's edge is the core's
/// prediction, within 3 of 255 a channel. Gives how many pixels a
/// decoration lit or covered.
fn assert_as_predicted(
    pixels: &[u8],
    sky: &SkyPose,
    look: &SkyLook,
    list: &[SkyDecoration],
    what: &str,
) -> usize {
    let stars = instances(sky, look, LOGICAL, 1.0);
    let drawn = placed(sky, list, LOGICAL, 1.0);
    let mut touched = 0;
    for at in (0..SIDE).flat_map(|y| (0..SIDE).map(move |x| (x, y))) {
        let Some((expected, by_one)) = predicted(&stars, list, &drawn, at) else {
            continue;
        };
        let seen = rgb(pixels, at);
        for (s, e) in seen.iter().zip(expected) {
            assert!(
                (f32::from(*s) - e).abs() <= 3.0,
                "{what} at {at:?}: {seen:?}, predicted {expected:?}"
            );
        }
        touched += usize::from(by_one);
    }
    touched
}

/// `image` drawn at the middle of the view as `sky` sees it, `size` units
/// long, with every default: placed where its parallax of the eye's
/// move brings it back to the anchor.
fn centred(sky: &SkyPose, image: &Arc<SkyImage>, size: f32) -> SkyDecoration {
    let plain = SkyDecoration::new(Arc::clone(image), Point::new(0.0, 0.0));
    SkyDecoration {
        size,
        at: Point::new(sky.eye.x * plain.parallax, sky.eye.y * plain.parallax),
        ..plain
    }
}

#[test]
fn an_image_at_one_texel_a_pixel_shows_its_texels_where_the_core_places_them() {
    let image = picture(32, 255, false);
    let list = [SkyDecoration {
        size: 32.0,
        parallax: 0.2,
        ..SkyDecoration::new(Arc::clone(&image), Point::new(40.0, -30.0))
    }];
    // An eye off the centre, so its parallax places it.
    let sky = SkyPose::new(SEED, Point::new(100.0, 50.0), Point::new(128.0, 128.0), 32);
    let Some(pixels) = drawn(sky, dark(), &list, RenderScale::Auto) else {
        return;
    };
    let touched = assert_as_predicted(&pixels, &sky, &dark(), &list, "one texel a pixel");
    assert!(touched > 800, "{touched}");
    // Its centre is at 128 + 40 − 20, 128 − 30 − 10; the texel there is
    // the image's own, premultiplied by its whole alpha.
    let texel = &image.texture().levels()[0];
    let at = ((16 * 32 + 16) * 4) as usize;
    let seen = rgb(&pixels, (148, 88));
    for (s, e) in seen.iter().zip(&texel.rgba[at..at + 3]) {
        assert!((i32::from(*s) - i32::from(*e)).abs() <= 3, "{seen:?}");
    }
    // Outside it the dark sky is black.
    assert_eq!(rgb(&pixels, (20, 200)), [0, 0, 0]);
}

#[test]
fn a_turned_tinted_half_bright_image_shows_the_references_pixels() {
    let image = picture(16, 255, true);
    let list = [SkyDecoration {
        size: 96.0,
        turn: 30.0,
        tint: Some([1.0, 0.6, 0.3]),
        brightness: 0.5,
        ..SkyDecoration::new(image, Point::new(10.0, -6.0))
    }];
    let Some(pixels) = drawn(pose(32), dark(), &list, RenderScale::Auto) else {
        return;
    };
    let touched = assert_as_predicted(&pixels, &pose(32), &dark(), &list, "turned and tinted");
    assert!(touched > 6000, "{touched}");
    // Unturned, it draws other pixels.
    let straight = [SkyDecoration {
        turn: 0.0,
        ..list[0].clone()
    }];
    let Some(unturned) = drawn(pose(32), dark(), &straight, RenderScale::Auto) else {
        return;
    };
    assert_ne!(pixels, unturned);
}

#[test]
fn a_spinning_image_is_drawn_turned_by_the_poses_time() {
    let image = picture(16, 255, true);
    let spinning = [SkyDecoration {
        size: 96.0,
        spin: 6.0,
        ..SkyDecoration::new(image, Point::new(10.0, -6.0))
    }];
    // Five seconds in it has turned 30 degrees: the reference's pixels
    // then, which are those of the same image set at that turn.
    let later = pose(32).with_time(5.0);
    let Some(pixels) = drawn(later, dark(), &spinning, RenderScale::Auto) else {
        return;
    };
    let touched = assert_as_predicted(&pixels, &later, &dark(), &spinning, "spun");
    assert!(touched > 6000, "{touched}");
    let turned = [SkyDecoration {
        turn: 30.0,
        spin: 0.0,
        ..spinning[0].clone()
    }];
    let Some(set) = drawn(pose(32), dark(), &turned, RenderScale::Auto) else {
        return;
    };
    assert_eq!(pixels, set);
    // At time zero it has not turned.
    let Some(start) = drawn(pose(32), dark(), &spinning, RenderScale::Auto) else {
        return;
    };
    assert_ne!(pixels, start);
}

#[test]
fn drawn_larger_it_blends_its_texels_and_smaller_it_reads_its_mips() {
    let image = picture(32, 255, false);
    for (size, what) in [
        (64.0, "twice its size"),
        (16.0, "half its size"),
        (8.0, "a quarter of its size"),
        (24.0, "between two levels"),
    ] {
        let list = [centred(&pose(32), &image, size)];
        let Some(pixels) = drawn(pose(32), dark(), &list, RenderScale::Auto) else {
            return;
        };
        let touched = assert_as_predicted(&pixels, &pose(32), &dark(), &list, what);
        assert!(touched as f32 >= (size - 2.0) * (size - 2.0), "{what}");
    }
}

#[test]
fn murk_level_16_halves_its_light() {
    let image = picture(32, 255, false);
    let list = [centred(&pose(32), &image, 64.0)];
    let Some(clear) = drawn(pose(32), dark(), &list, RenderScale::Auto) else {
        return;
    };
    let Some(half) = drawn(pose(16), dark(), &list, RenderScale::Auto) else {
        return;
    };
    assert_as_predicted(&half, &pose(16), &dark(), &list, "level 16");
    let mut checked = 0;
    for at in (100..156).flat_map(|y| (100..156).map(move |x| (x, y))) {
        for (c, h) in rgb(&clear, at).into_iter().zip(rgb(&half, at)) {
            assert!(
                (f32::from(h) - f32::from(c) / 2.0).abs() <= 1.5,
                "at {at:?}: {c} then {h}"
            );
            checked += usize::from(c > 10);
        }
    }
    assert!(checked > 5000, "{checked}");
}

#[test]
fn an_opaque_image_painted_over_a_busy_sky_hides_it_and_the_sky_shows_outside() {
    let image = picture(32, 255, false);
    let sky = cloudy_pose(32);
    let list = [centred(&sky, &image, 96.0)];
    let Some(pixels) = drawn(sky, busy(), &list, RenderScale::Auto) else {
        return;
    };
    let touched = assert_as_predicted(&pixels, &sky, &busy(), &list, "over a busy sky");
    assert!(touched > 8000, "{touched}");
    // Inside, the pixels are the image's alone, whatever is beneath:
    // the same as over a dark sky.
    let Some(alone) = drawn(sky, dark(), &list, RenderScale::Auto) else {
        return;
    };
    let stars = instances(&sky, &busy(), LOGICAL, 1.0);
    let mut lit_beneath = 0;
    for at in (84..172).flat_map(|y| (84..172).map(move |x| (x, y))) {
        assert_eq!(rgb(&pixels, at), rgb(&alone, at), "at {at:?}");
        lit_beneath += usize::from(sky_at(&stars, at).iter().any(|&c| c > 0.02));
    }
    assert!(
        lit_beneath > 1500,
        "the sky is lit behind it: {lit_beneath}"
    );
    // Outside it, the sky shows as it does with no decoration.
    let Some(bare) = drawn(sky, busy(), &[], RenderScale::Auto) else {
        return;
    };
    let mut outside = 0;
    let clear_of_it = |c: u32| !(70..186).contains(&c);
    for at in (0..SIDE).flat_map(|y| (0..SIDE).map(move |x| (x, y))) {
        if clear_of_it(at.0) || clear_of_it(at.1) {
            assert_eq!(rgb(&pixels, at), rgb(&bare, at), "at {at:?}");
            outside += usize::from(rgb(&pixels, at).iter().any(|&c| c > 2));
        }
    }
    assert!(outside > 2000, "{outside}");
}

#[test]
fn a_half_clear_image_shows_half_the_sky_beneath_plus_its_light() {
    let image = picture(32, 128, true);
    let sky = cloudy_pose(32);
    let list = [centred(&sky, &image, 96.0)];
    let Some(pixels) = drawn(sky, busy(), &list, RenderScale::Auto) else {
        return;
    };
    assert_as_predicted(&pixels, &sky, &busy(), &list, "half clear");
    // In its clear quarter the sky shows whole; in the rest, dimmed and
    // lit: both differ from the image over a dark sky.
    let Some(alone) = drawn(sky, dark(), &list, RenderScale::Auto) else {
        return;
    };
    let differs = (84..172)
        .flat_map(|y| (84..172).map(move |x| (x, y)))
        .filter(|&at| rgb(&pixels, at) != rgb(&alone, at))
        .count();
    assert!(differs > 2000, "{differs}");
}

#[test]
fn a_glow_adds_its_light_to_the_backdrop_beneath() {
    let image = picture(32, 255, false);
    let sky = cloudy_pose(32);
    let list = [SkyDecoration {
        blend: SkyBlend::Glow,
        brightness: 0.4,
        ..centred(&sky, &image, 96.0)
    }];
    let look = SkyLook {
        stars: 0.0,
        ..busy()
    };
    let Some(pixels) = drawn(sky, look, &list, RenderScale::Auto) else {
        return;
    };
    assert_as_predicted(&pixels, &sky, &look, &list, "a glow");
    // No pixel under it is darker than the backdrop alone.
    let Some(bare) = drawn(sky, look, &[], RenderScale::Auto) else {
        return;
    };
    let mut brighter = 0;
    for at in (84..172).flat_map(|y| (84..172).map(move |x| (x, y))) {
        let (with, without) = (rgb(&pixels, at), rgb(&bare, at));
        assert!(
            with.iter().zip(without).all(|(w, b)| *w >= b),
            "at {at:?}: {with:?} under {without:?}"
        );
        brighter += usize::from(with != without);
    }
    assert!(brighter > 5000, "{brighter}");
}

#[test]
fn of_two_overlapping_images_painted_over_the_nearer_shows() {
    let (far, near) = (picture(16, 255, false), picture(32, 255, false));
    // Listed nearer first: the order drawn is by parallax, not the list.
    let list = [
        SkyDecoration {
            parallax: 0.4,
            ..centred(&pose(32), &near, 64.0)
        },
        SkyDecoration {
            parallax: 0.05,
            tint: Some([1.0, 0.2, 0.2]),
            ..SkyDecoration::new(Arc::clone(&far), Point::new(30.0, 20.0))
        },
    ];
    let Some(pixels) = drawn(pose(32), dark(), &list, RenderScale::Auto) else {
        return;
    };
    assert_as_predicted(&pixels, &pose(32), &dark(), &list, "two overlapping");
    // Where they overlap the pixels are the nearer one's alone.
    let Some(alone) = drawn(pose(32), dark(), &list[..1], RenderScale::Auto) else {
        return;
    };
    for at in (100..156).flat_map(|y| (100..156).map(move |x| (x, y))) {
        assert_eq!(rgb(&pixels, at), rgb(&alone, at), "at {at:?}");
    }
    // The farther one shows where the nearer doesn't reach.
    assert_ne!(rgb(&pixels, (200, 180)), [0, 0, 0]);
    assert_eq!(rgb(&alone, (200, 180)), [0, 0, 0]);
}

#[test]
fn through_a_rush_the_stars_streak_and_the_image_is_as_it_was() {
    let image = picture(32, 255, false);
    let still = cloudy_pose(32);
    let list = [centred(&still, &image, 96.0)];
    let rushing = still.with_rush(Point::new(900.0, -400.0));
    let Some(before) = drawn(still, busy(), &list, RenderScale::Auto) else {
        return;
    };
    let Some(after) = drawn(rushing, busy(), &list, RenderScale::Auto) else {
        return;
    };
    assert_as_predicted(&after, &rushing, &busy(), &list, "rushing");
    for at in (84..172).flat_map(|y| (84..172).map(move |x| (x, y))) {
        assert_eq!(rgb(&after, at), rgb(&before, at), "at {at:?}");
    }
    assert_ne!(before, after, "the stars streak round it");
}

#[test]
fn at_the_lowest_render_scale_an_image_still_lights_its_pixels() {
    let image = picture(32, 255, false);
    let list = [centred(&pose(32), &image, 64.0)];
    let lowest = RenderScale::Percent(RenderScale::MIN);
    let Some(pixels) = drawn(pose(32), dark(), &list, lowest) else {
        return;
    };
    let lit_pixels = (100..156)
        .flat_map(|y| (100..156).map(move |x| (x, y)))
        .filter(|&at| rgb(&pixels, at).iter().any(|&c| c > 20))
        .count();
    assert_eq!(lit_pixels, 56 * 56);
    assert_eq!(rgb(&pixels, (10, 10)), [0, 0, 0]);
}
