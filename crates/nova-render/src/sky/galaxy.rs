//! The far galaxy a sky's seed may give it ([`seeded`]): the engine's
//! default decoration for a system whose data lists none, so a game with
//! no art still has skies of their own.
//!
//! # Which skies have one
//!
//! [`GALAXY_CHANCE`] of the seeds, 3 in 10, have one galaxy; the rest
//! have none, and none has two. The seed picks its kind (a spiral two
//! times in three, else an elliptical), where it sits (within the seeded
//! reach, [`super::SEEDED_REACH`]), its size ([`GALAXY_SIZE`]), how far
//! it is tipped from face-on ([`GALAXY_TILT`], the ratio of its short
//! axis to its long), its heading, its hue (one of [`GALAXY_HUES`]), the
//! light of its core ([`GALAXY_LIGHT`]) and which way its arms wind. The
//! draws are salted past the stars', the backdrop's and the decorations'
//! places, so no star, cloud or place changes.
//!
//! # How it is drawn
//!
//! As a decoration like any other ([`decoration`]): at
//! [`GALAXY_PARALLAX`], half the backdrop's, so it barely moves; blended
//! as a glow, so it adds to the stars and the clouds and never hides
//! them; unspun, untinted and at full brightness, its light being in its
//! image.
//!
//! # Its image
//!
//! [`bake`] paints it into a [`GALAXY_SIDE`]-texel square from round
//! blobs of the stars' own falloff ([`super::coverage`]), at most
//! [`MOST_BLOBS`] of them: a bright core and a wide faint disc, and for
//! a spiral two arms, each a chain of blobs laid by turning a step at a
//! time through an angle whose sine and cosine are fractions (13/85 and
//! 84/85) while stepping outward. The disc and the arms are flattened by
//! the tilt, the core less so. The blobs' lights are scaled so the
//! middle of the image is exactly the core's light, which is the
//! brightest it gets, and every blob lies inside the square, so its edge
//! texels are clear.
//!
//! The bake uses integer hashing and IEEE `+ − × ÷` and `sqrt` alone, as
//! the planet bake does, so a seed bakes the same bytes on every
//! platform; a digest test pins it. The renderer bakes a galaxy the
//! first frame that draws its sky (`crate::sky_images`), so it costs
//! nothing at start-up.

use std::sync::Arc;

use nova_view::Point;

use super::decoration::{SEEDED_REACH, SkyBlend, SkyDecoration};
use super::image::SkyImage;
use super::{Draws, LAYERS, coverage};

/// How many seeds in how many have a galaxy.
pub const GALAXY_CHANCE: (u64, u64) = (3, 10);

/// The least and most length a galaxy is drawn at, in logical units.
pub const GALAXY_SIZE: (f32, f32) = (48.0, 112.0);

/// The least and most a galaxy's short axis is of its long: nearly
/// edge-on, to face-on.
pub const GALAXY_TILT: (f32, f32) = (0.32, 1.0);

/// The least and most light of a galaxy's core, of full.
pub const GALAXY_LIGHT: (f32, f32) = (0.35, 0.55);

/// The share of the eye's move a galaxy moves by: half the backdrop's.
pub const GALAXY_PARALLAX: f32 = 0.0125;

/// The side of a galaxy's image, in texels.
pub const GALAXY_SIDE: u32 = 128;

/// The most blobs a galaxy is painted from.
pub const MOST_BLOBS: usize = 3 + 2 * ARM_BLOBS;

/// The blobs in one arm of a spiral.
const ARM_BLOBS: usize = 36;

/// The hues a seed picks a galaxy's from, RGB 0 to 1 at full brightness:
/// pale blue-white, white, pale gold and pale rose.
pub const GALAXY_HUES: [[f32; 3]; 4] = [
    [0.78, 0.87, 1.0],
    [1.0, 1.0, 1.0],
    [1.0, 0.93, 0.78],
    [1.0, 0.84, 0.82],
];

/// What a galaxy is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GalaxyKind {
    /// A disc with two arms winding out from its core.
    Spiral,
    /// A smooth oval, brightest in the middle.
    Elliptical,
}

/// The far galaxy of a sky.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Galaxy {
    /// What it is.
    pub kind: GalaxyKind,
    /// Where it sits on its layer, in logical units from the layer's
    /// centre.
    pub at: Point,
    /// The drawn length of its image's side, in logical units.
    pub size: f32,
    /// Its short axis over its long, within [`GALAXY_TILT`].
    pub tilt: f32,
    /// Its heading, in degrees clockwise as drawn, 0 to 360.
    pub heading: f32,
    /// Its hue, one of [`GALAXY_HUES`].
    pub hue: [f32; 3],
    /// The light of its core, of full, within [`GALAXY_LIGHT`].
    pub light: f32,
    /// Whether its arms wind clockwise going out.
    pub clockwise: bool,
    /// What its arms' blobs are varied by.
    pub grain: u64,
}

/// The galaxy sky `seed` has when its data lists no decorations, if it
/// has one (see the [module docs](self)).
#[must_use]
pub fn seeded(seed: u64) -> Option<Galaxy> {
    // Salted past the star layers, the backdrop's two salts and the
    // decorations' places.
    let mut draws = Draws::new(seed, LAYERS.len() + 3, 0, 0);
    if draws.next() % GALAXY_CHANCE.1 >= GALAXY_CHANCE.0 {
        return None;
    }
    let kind = if draws.next().is_multiple_of(3) {
        GalaxyKind::Elliptical
    } else {
        GalaxyKind::Spiral
    };
    let x = (2.0 * draws.unit() - 1.0) * SEEDED_REACH.0;
    let y = (2.0 * draws.unit() - 1.0) * SEEDED_REACH.1;
    let size = between(GALAXY_SIZE, draws.unit());
    let tilt = between(GALAXY_TILT, draws.unit());
    let heading = draws.unit() * 360.0;
    let hue = GALAXY_HUES[(draws.next() % GALAXY_HUES.len() as u64) as usize];
    let light = between(GALAXY_LIGHT, draws.unit());
    let clockwise = draws.next().is_multiple_of(2);
    Some(Galaxy {
        kind,
        at: Point::new(x, y),
        size,
        tilt,
        heading,
        hue,
        light,
        clockwise,
        grain: draws.next(),
    })
}

/// `range`'s value at `t` of the way from its least to its most.
fn between((lo, hi): (f32, f32), t: f32) -> f32 {
    lo + (hi - lo) * t
}

/// `galaxy` as the decoration its sky draws, showing `image` (its
/// [`bake`]).
#[must_use]
pub fn decoration(galaxy: &Galaxy, image: Arc<SkyImage>) -> SkyDecoration {
    SkyDecoration {
        parallax: GALAXY_PARALLAX,
        size: galaxy.size,
        turn: galaxy.heading,
        blend: SkyBlend::Glow,
        ..SkyDecoration::new(image, galaxy.at)
    }
}

/// One round blob of a galaxy's image, on its disc: the image's square
/// runs from −1 to 1 each way, and the disc is the square with its down
/// axis stretched by the blob's `flat`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Blob {
    /// Its centre on the disc.
    centre: Point,
    /// Its radius on the disc.
    radius: f32,
    /// How much shorter it is drawn down the image than across: the
    /// galaxy's tilt for its disc and arms, nearer 1 for its core.
    flat: f32,
    /// The light it adds at its centre, RGB.
    light: [f32; 3],
}

/// The blobs `galaxy` is painted from, their lights not yet scaled to
/// its core's.
fn blobs(galaxy: &Galaxy) -> Vec<Blob> {
    let (hue, tilt) = (galaxy.hue, galaxy.tilt);
    // The hue moved `share` of the way toward `toward`, at `light`.
    let lit = |toward: [f32; 3], share: f32, light: f32| {
        [0, 1, 2].map(|c| (hue[c] + (toward[c] - hue[c]) * share) * light)
    };
    // A blob at the middle.
    let round = |radius: f32, flat: f32, light: [f32; 3]| Blob {
        centre: Point::new(0.0, 0.0),
        radius,
        flat,
        light,
    };
    // A core is a ball, so it is tipped half as far as the disc.
    let ball = f32::midpoint(1.0, tilt);
    if galaxy.kind == GalaxyKind::Elliptical {
        return vec![
            round(0.3, ball, lit(CORE, 0.5, 1.0)),
            round(0.62, tilt, lit(CORE, 0.3, 0.6)),
            round(0.95, tilt, lit(CORE, 0.0, 0.28)),
        ];
    }
    let mut blobs = vec![
        round(0.2, ball, lit(CORE, 0.6, 1.0)),
        round(0.42, ball, lit(CORE, 0.3, 0.3)),
        round(0.93, tilt, lit(CORE, 0.0, 0.22)),
    ];
    // Each step turns the arm by the angle whose cosine and sine are
    // these fractions, about 9 degrees: close enough that an arm's
    // blobs run together.
    let (cos, sin) = (84.0 / 85.0, 13.0 / 85.0);
    let sin = if galaxy.clockwise { sin } else { -sin };
    let mut draws = Draws::new(galaxy.grain, 0, 0, 0);
    for start in [1.0, -1.0] {
        let (mut x, mut y) = (start, 0.0_f32);
        for step in 0..ARM_BLOBS {
            // How far along the arm, 0 to 1.
            let out = (step as f32 + 0.5) / ARM_BLOBS as f32;
            let reach = 0.14 + 0.64 * out;
            let radius = (0.07 + 0.08 * out) * (0.85 + 0.3 * draws.unit());
            // It fades to nothing at its tip, so an arm has no cut end.
            let fade = (1.0 - out) * (1.0 - 0.3 * out);
            // Blobs closer together than they are wide add up, so each
            // gives less by as much: an arm is as bright along its
            // length whatever its blobs' spacing.
            let spacing = reach * (13.0 / 85.0) + 0.64 / ARM_BLOBS as f32;
            let light = 0.5 * fade * (0.7 + 0.6 * draws.unit()) * spacing / radius;
            blobs.push(Blob {
                centre: Point::new(x * reach, y * reach),
                radius,
                flat: tilt,
                light: lit(ARM, 0.25, light),
            });
            (x, y) = (x * cos - y * sin, x * sin + y * cos);
        }
    }
    blobs
}

/// The colour a galaxy's core leans toward: the warm white of old stars.
const CORE: [f32; 3] = [1.0, 0.9, 0.72];

/// The colour a spiral's arms lean toward: the blue-white of young ones.
const ARM: [f32; 3] = [0.72, 0.84, 1.0];

/// The light `blobs` add at (`x`, `y`) of the image's square, RGB.
fn light_at(blobs: &[Blob], x: f32, y: f32) -> [f32; 3] {
    let mut sum = [0.0; 3];
    for blob in blobs {
        let (dx, dy) = (x - blob.centre.x, y / blob.flat - blob.centre.y);
        // Past its radius across, it adds nothing: no need to measure.
        if dx.abs() >= blob.radius {
            continue;
        }
        let share = coverage((dx * dx + dy * dy).sqrt() / blob.radius);
        for (channel, light) in sum.iter_mut().zip(blob.light) {
            *channel += light * share;
        }
    }
    sum
}

/// `galaxy`'s image: [`GALAXY_SIDE`] texels square, premultiplied (see
/// the [module docs](self)).
///
/// # Panics
///
/// Never: the image is always whole.
#[must_use]
pub fn bake(galaxy: &Galaxy) -> SkyImage {
    let blobs = blobs(galaxy);
    // The blobs' lights are scaled so the middle's brightest channel is
    // the core's light.
    let middle = light_at(&blobs, 0.0, 0.0).into_iter().fold(0.0, f32::max);
    let scale = galaxy.light / middle * 255.0;
    let side = GALAXY_SIDE;
    let half = side as f32 / 2.0;
    let mut rgba = Vec::with_capacity((side * side * 4) as usize);
    for row in 0..side {
        for column in 0..side {
            // The texel's centre, −1 to 1 across the square.
            let across = (column as f32 + 0.5) / half - 1.0;
            let down = (row as f32 + 0.5) / half - 1.0;
            let light = light_at(&blobs, across, down);
            let bytes = light.map(|c| (c * scale).round().min(255.0) as u8);
            // Premultiplied: its alpha is its brightest channel.
            let alpha = bytes.into_iter().fold(0, u8::max);
            rgba.extend(bytes);
            rgba.push(alpha);
        }
    }
    SkyImage::premultiplied(side, side, rgba).expect("a whole square of texels")
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::{DECORATION_PARALLAX, DECORATION_SIZE, decoration_area_bound};
    use super::*;
    use crate::viewport::LOGICAL;

    /// The galaxies of the first `count` seeds that have one.
    fn galaxies(count: u64) -> Vec<(u64, Galaxy)> {
        (0..count)
            .filter_map(|seed| seeded(seed).map(|galaxy| (seed, galaxy)))
            .collect()
    }

    /// The first seed whose galaxy is of `kind`.
    fn first(kind: GalaxyKind) -> Galaxy {
        (0..10_000)
            .find_map(|seed| seeded(seed).filter(|galaxy| galaxy.kind == kind))
            .expect("a galaxy of the kind")
    }

    #[test]
    fn about_three_seeds_in_ten_have_a_galaxy() {
        assert_eq!(GALAXY_CHANCE, (3, 10));
        let with = galaxies(2000).len();
        assert!((540..=660).contains(&with), "{with} of 2000");
        // The same seed always answers the same.
        for seed in 0..50 {
            assert_eq!(seeded(seed), seeded(seed));
        }
    }

    #[test]
    fn a_seeds_galaxy_is_pinned() {
        // The sample's three systems (their Nova IDs are their seeds):
        // Brightwater has an elliptical, Luyten Crossing none and Marrow
        // a spiral. Pinned so a change to the draws, their order or
        // their salt shows.
        assert_eq!(
            seeded(128),
            Some(Galaxy {
                kind: GalaxyKind::Elliptical,
                at: Point::new(276.9146, -58.866_737),
                size: 64.485_886,
                tilt: 0.662_933_9,
                heading: 214.897_87,
                hue: [1.0, 0.93, 0.78],
                light: 0.422_753_84,
                clockwise: false,
                grain: 7_568_530_573_109_403_012,
            })
        );
        assert_eq!(seeded(129), None);
        assert_eq!(
            seeded(130),
            Some(Galaxy {
                kind: GalaxyKind::Spiral,
                at: Point::new(296.214_94, -112.661_98),
                size: 107.897_255,
                tilt: 0.671_297_9,
                heading: 94.861_73,
                hue: [1.0, 0.84, 0.82],
                light: 0.455_286_7,
                clockwise: true,
                grain: 3_548_471_114_448_882_018,
            })
        );
    }

    #[test]
    fn every_galaxy_is_within_its_ranges() {
        for (seed, galaxy) in galaxies(1500) {
            assert!(
                galaxy.at.x.abs() <= SEEDED_REACH.0 && galaxy.at.y.abs() <= SEEDED_REACH.1,
                "{seed}: {galaxy:?}"
            );
            assert!((48.0..=112.0).contains(&galaxy.size), "{seed}");
            assert!((0.32..=1.0).contains(&galaxy.tilt), "{seed}");
            assert!((0.0..360.0).contains(&galaxy.heading), "{seed}");
            assert!((0.35..=0.55).contains(&galaxy.light), "{seed}");
            assert!(GALAXY_HUES.contains(&galaxy.hue), "{seed}");
        }
        assert_eq!(GALAXY_SIZE, (48.0, 112.0));
        assert_eq!(GALAXY_LIGHT, (0.35, 0.55));
        assert_eq!(GALAXY_TILT, (0.32, 1.0));
    }

    #[test]
    fn the_galaxies_vary_in_every_way() {
        let all = galaxies(1500);
        let count = |what: &dyn Fn(&Galaxy) -> bool| all.iter().filter(|(_, g)| what(g)).count();
        // Two spirals to an elliptical.
        let spirals = count(&|g| g.kind == GalaxyKind::Spiral);
        let ellipticals = count(&|g| g.kind == GalaxyKind::Elliptical);
        assert_eq!(spirals + ellipticals, all.len());
        assert!(
            spirals > ellipticals * 3 / 2 && spirals < ellipticals * 3,
            "{spirals} spirals, {ellipticals} ellipticals"
        );
        // Every hue, each a fair share.
        for hue in GALAXY_HUES {
            let of = count(&|g| g.hue == hue);
            assert!(of > all.len() / 6, "{hue:?}: {of} of {}", all.len());
        }
        // Both ways round.
        let clockwise = count(&|g| g.clockwise);
        assert!(clockwise > all.len() / 3 && clockwise < all.len() * 2 / 3);
        // Each range is used from end to end.
        let spread = |of: &dyn Fn(&Galaxy) -> f32, (least, most): (f32, f32)| {
            let values: Vec<f32> = all.iter().map(|(_, g)| of(g)).collect();
            let low = values.iter().copied().fold(f32::MAX, f32::min);
            let high = values.iter().copied().fold(f32::MIN, f32::max);
            let span = most - least;
            assert!(
                low < least + span * 0.05 && high > most - span * 0.05,
                "{low} to {high} of {least} to {most}"
            );
        };
        spread(&|g| g.size, GALAXY_SIZE);
        spread(&|g| g.tilt, GALAXY_TILT);
        spread(&|g| g.light, GALAXY_LIGHT);
        spread(&|g| g.heading, (0.0, 360.0));
        spread(&|g| g.at.x, (-SEEDED_REACH.0, SEEDED_REACH.0));
        spread(&|g| g.at.y, (-SEEDED_REACH.1, SEEDED_REACH.1));
        // No two alike.
        let grains: std::collections::HashSet<u64> = all.iter().map(|(_, g)| g.grain).collect();
        assert_eq!(grains.len(), all.len());
    }

    #[test]
    fn a_galaxy_is_a_far_glow_at_its_place_size_and_heading() {
        let galaxy = first(GalaxyKind::Spiral);
        let image = Arc::new(bake(&galaxy));
        let drawn = decoration(&galaxy, Arc::clone(&image));
        assert!(Arc::ptr_eq(&drawn.image, &image));
        assert_eq!(drawn.at, galaxy.at);
        assert_eq!(drawn.parallax, 0.0125);
        assert_eq!(
            drawn.parallax, DECORATION_PARALLAX.0,
            "the farthest there is"
        );
        assert_eq!(drawn.size, galaxy.size);
        assert_eq!(drawn.turn, galaxy.heading);
        assert_eq!(drawn.spin, 0.0);
        assert_eq!(drawn.tint, None);
        assert_eq!(drawn.brightness, 1.0);
        assert_eq!(drawn.blend, SkyBlend::Glow);
        // It is well inside a decoration's bounds.
        assert!(GALAXY_SIZE.1 <= DECORATION_SIZE.1 && GALAXY_SIZE.0 >= DECORATION_SIZE.0);
        assert!(GALAXY_SIZE.1 * GALAXY_SIZE.1 <= decoration_area_bound(LOGICAL) / 6.0);
    }

    /// Texel (`x`, `y`) of `image`'s whole level.
    fn texel(image: &SkyImage, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * GALAXY_SIDE + x) * 4) as usize;
        image.texture().levels()[0].rgba[at..at + 4]
            .try_into()
            .expect("four bytes")
    }

    /// The brightest channel of each texel of `image`, row by row.
    fn lights(image: &SkyImage) -> Vec<u8> {
        image.texture().levels()[0]
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|t| t[0].max(t[1]).max(t[2]))
            .collect()
    }

    #[test]
    fn a_bake_is_brightest_in_its_middle_clear_at_its_edge_and_never_over_its_light() {
        let some: Vec<Galaxy> = galaxies(120).into_iter().map(|(_, g)| g).collect();
        assert!(some.len() > 20);
        assert!(some.iter().any(|g| g.kind == GalaxyKind::Elliptical));
        for galaxy in some {
            let image = bake(&galaxy);
            assert_eq!((image.width(), image.height()), (128, 128));
            let lights = lights(&image);
            let brightest = lights.iter().copied().max().expect("texels");
            // The core's light, to the byte, and it is at the four
            // middle texels.
            let core = (galaxy.light * 255.0).round() as u8;
            assert!(brightest.abs_diff(core) <= 1, "{galaxy:?}: {brightest}");
            assert!(brightest <= 141, "never over 0.55 of full");
            for (x, y) in [(63, 63), (64, 63), (63, 64), (64, 64)] {
                let middle = lights[(y * 128 + x) as usize];
                assert!(brightest - middle <= 2, "{galaxy:?}: {middle}");
            }
            // Every texel of its edge is clear, alpha too.
            for along in 0..128 {
                for (x, y) in [(along, 0), (along, 127), (0, along), (127, along)] {
                    assert_eq!(texel(&image, x, y), [0; 4], "{galaxy:?} at ({x}, {y})");
                }
            }
            // It is premultiplied: no colour is over its alpha.
            for t in image.texture().levels()[0].rgba.as_chunks::<4>().0 {
                assert!(t[0] <= t[3] && t[1] <= t[3] && t[2] <= t[3], "{t:?}");
            }
            // It has a body: a fair share of its texels are lit.
            let lit = lights.iter().filter(|&&l| l > 0).count();
            assert!(lit > 128 * 128 / 12, "{galaxy:?}: {lit} lit");
        }
    }

    #[test]
    fn a_tipped_galaxy_is_narrower_down_than_across() {
        for kind in [GalaxyKind::Spiral, GalaxyKind::Elliptical] {
            let flat = Galaxy {
                tilt: 0.35,
                ..first(kind)
            };
            let image = bake(&flat);
            let lights = lights(&image);
            let lit = |x: u32, y: u32| lights[(y * 128 + x) as usize] > 0;
            // Out along its long axis it is lit; as far out along its
            // short one it is not.
            assert!(lit(64 + 40, 64) && lit(64 - 40, 64), "{kind:?}");
            assert!(!lit(64, 64 + 40) && !lit(64, 64 - 40), "{kind:?}");
            let round = Galaxy { tilt: 1.0, ..flat };
            let lights = super::tests::lights(&bake(&round));
            assert!(lights[((64 + 40) * 128 + 64) as usize] > 0, "{kind:?}");
        }
    }

    #[test]
    fn a_spiral_has_arms_and_an_elliptical_none() {
        // Round a ring half way out, a face-on spiral's light rises and
        // falls as its arms cross; an elliptical's is even.
        let swing = |kind: GalaxyKind| {
            let face_on = Galaxy {
                tilt: 1.0,
                ..first(kind)
            };
            let lights = lights(&bake(&face_on));
            // Sixteen points round the ring, by a turn of fractions.
            let (mut x, mut y) = (36.0_f32, 0.0_f32);
            let mut seen = Vec::new();
            for _ in 0..16 {
                let at = ((64.0 + y) as u32) * 128 + (64.0 + x) as u32;
                seen.push(i32::from(lights[at as usize]));
                (x, y) = (
                    x * (12.0 / 13.0) - y * (5.0 / 13.0),
                    x * (5.0 / 13.0) + y * (12.0 / 13.0),
                );
            }
            seen.iter().max().expect("points") - seen.iter().min().expect("points")
        };
        let (spiral, elliptical) = (swing(GalaxyKind::Spiral), swing(GalaxyKind::Elliptical));
        assert!(spiral >= 8, "a spiral's ring swings by {spiral}");
        assert!(elliptical <= 2, "an elliptical's by {elliptical}");
    }

    #[test]
    fn its_arms_wind_the_way_it_says() {
        let one = first(GalaxyKind::Spiral);
        let other = Galaxy {
            clockwise: !one.clockwise,
            tilt: 1.0,
            ..one
        };
        let one = Galaxy { tilt: 1.0, ..one };
        let (a, b) = (bake(&one), bake(&other));
        assert_ne!(a, b);
        // One is the other's mirror, top to bottom.
        let (a, b) = (lights(&a), lights(&b));
        for (x, y) in [(40_u32, 50_u32), (90, 70), (64, 30), (20, 64)] {
            let mirrored = b[((127 - y) * 128 + x) as usize];
            assert!(
                a[(y * 128 + x) as usize].abs_diff(mirrored) <= 1,
                "({x}, {y})"
            );
        }
    }

    #[test]
    fn its_hue_colours_it_and_its_light_scales_it() {
        let base = Galaxy {
            hue: [1.0, 1.0, 1.0],
            light: 0.5,
            ..first(GalaxyKind::Spiral)
        };
        let rose = Galaxy {
            hue: [1.0, 0.84, 0.82],
            ..base
        };
        // Out in the disc, the white one's blue is no less than the
        // rose one's, whose red leads its blue.
        let (white, pink) = (bake(&base), bake(&rose));
        let out = |image: &SkyImage| texel(image, 64 + 30, 64 + 6);
        assert!(out(&pink)[0] > out(&pink)[2], "{:?}", out(&pink));
        assert!(out(&white)[2] > out(&pink)[2], "{:?}", out(&white));
        // Half the light is half the bytes, within rounding.
        let dim = bake(&Galaxy {
            light: 0.25,
            ..base
        });
        for (x, y) in [(64, 64), (80, 70), (40, 60)] {
            let (full, half) = (texel(&white, x, y), texel(&dim, x, y));
            for c in 0..3 {
                assert!(
                    (i32::from(full[c]) - 2 * i32::from(half[c])).abs() <= 2,
                    "({x}, {y}): {full:?} and {half:?}"
                );
            }
        }
    }

    fn fnv(hash: u64, bytes: &[u8]) -> u64 {
        bytes.iter().fold(hash, |h, &b| {
            (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01B3)
        })
    }

    /// A digest of every level of `image`.
    fn digest(image: &SkyImage) -> u64 {
        (image.texture().levels().iter())
            .fold(0xCBF2_9CE4_8422_2325, |h, level| fnv(h, &level.rgba))
    }

    #[test]
    fn a_seed_bakes_the_same_bytes_and_another_seed_others() {
        let spiral = first(GalaxyKind::Spiral);
        let elliptical = first(GalaxyKind::Elliptical);
        assert_eq!(bake(&spiral), bake(&spiral));
        assert_ne!(bake(&spiral), bake(&elliptical));
        let next = (0..10_000)
            .filter_map(seeded)
            .filter(|g| g.kind == GalaxyKind::Spiral)
            .nth(1)
            .expect("a second spiral");
        assert_ne!(bake(&spiral), bake(&next));
        // Pinned, so a change to the bake, or a platform that works it
        // out differently, shows.
        assert_eq!(
            (digest(&bake(&spiral)), digest(&bake(&elliptical))),
            (SPIRAL_DIGEST, ELLIPTICAL_DIGEST)
        );
    }

    const SPIRAL_DIGEST: u64 = 11_659_944_440_295_647_996;
    const ELLIPTICAL_DIGEST: u64 = 8_206_709_616_014_913_309;

    #[test]
    fn a_spiral_is_painted_from_no_more_blobs_than_its_bound() {
        assert_eq!(MOST_BLOBS, 75);
        for (_, galaxy) in galaxies(60) {
            let count = blobs(&galaxy).len();
            assert!(count <= MOST_BLOBS, "{galaxy:?}: {count}");
            match galaxy.kind {
                GalaxyKind::Spiral => assert_eq!(count, MOST_BLOBS),
                GalaxyKind::Elliptical => assert_eq!(count, 3),
            }
            // Every blob lies inside the image, so its edge is clear.
            for blob in blobs(&galaxy) {
                let reach = blob.centre.x.hypot(blob.centre.y) + blob.radius;
                assert!(reach <= 0.97, "{galaxy:?}: {blob:?} reaches {reach}");
            }
        }
    }
}
