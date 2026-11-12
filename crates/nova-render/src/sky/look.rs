//! A system's own sky look ([`SkyLook`]): how dense its stars are, the
//! colour they lean toward, and its distant backdrop
//! ([`super::backdrop`](mod@super::backdrop)), set by the system's data
//! or left to the engine's defaults.
//!
//! The renderer holds a table of looks keyed by the sky's seed
//! ([`SkyLooks`]), filled once from the game's data, and looks each
//! [`SkyPose::seed`](nova_view::SkyPose::seed) up in it: the pose carries no
//! look. A seed it holds none for draws [`SkyLook::default`]: the engine's
//! star field and a backdrop picked by the seed.
//!
//! # The stars
//!
//! [`SkyLook::stars`] multiplies every layer's density, from 0 (an empty
//! star field) to 2 (twice as busy); a value outside that is taken to its
//! nearer end, and one that is not finite as 1. A cell's star count is the
//! number of `k` with `k + u < density`, `u` its first draw, which rises
//! with the density, and each star's draws follow the count, so a busier
//! sky keeps every star a sparser one shows, where it was.
//!
//! [`SkyLook::tint`] moves each star's palette tint [`TINT_SHARE`] of the
//! way toward the tint, after the tint is scaled so its brightest channel
//! is 1 ([`normalised`]): a dark tint gives the colours its bright
//! equivalent does, so it doesn't dim the sky, and black, with no
//! brightest channel, leaves the palette. The tint is mixed after the
//! palette is drawn, so no draw changes.

use std::collections::HashMap;

/// How far each star's palette tint moves toward a system's tint.
pub const TINT_SHARE: f32 = 0.35;

/// A system's own sky look. The default is the engine's: stars at their
/// density, the palette's tints and a backdrop picked by the seed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyLook {
    /// Every star layer's density times this, 0 to 2.
    pub stars: f32,
    /// The colour the stars lean toward, RGB 0 to 1; `None` for the
    /// palette alone.
    pub tint: Option<[f32; 3]>,
    /// The distant backdrop.
    pub backdrop: BackdropLook,
}

impl Default for SkyLook {
    fn default() -> Self {
        Self {
            stars: 1.0,
            tint: None,
            backdrop: BackdropLook::default(),
        }
    }
}

/// A system's distant backdrop, each field left `None` coming from the
/// seed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BackdropLook {
    /// The clouds' colour, RGB 0 to 1, taken at full brightness.
    pub color: Option<[f32; 3]>,
    /// The chance that each backdrop cell holds a cloud, 0 to 1; 0 is no
    /// backdrop.
    pub amount: Option<f32>,
}

impl SkyLook {
    /// What every star layer's density is multiplied by: [`Self::stars`]
    /// taken into 0 to 2, and 1 when it is not finite.
    #[must_use]
    pub fn star_density(&self) -> f32 {
        if self.stars.is_finite() {
            self.stars.clamp(0.0, 2.0)
        } else {
            1.0
        }
    }

    /// A star's palette tint `palette`, moved [`TINT_SHARE`] of the way
    /// toward the look's tint at full brightness.
    #[must_use]
    pub fn tinted(&self, palette: [f32; 3]) -> [f32; 3] {
        let Some(tint) = self.tint.and_then(normalised) else {
            return palette;
        };
        [0, 1, 2].map(|c| palette[c] + TINT_SHARE * (tint[c] - palette[c]))
    }
}

/// `color` scaled so its brightest channel is 1, a channel below 0 or not
/// a number taken as 0; `None` when no channel is above 0 or one is
/// infinite.
#[must_use]
pub fn normalised(color: [f32; 3]) -> Option<[f32; 3]> {
    let top = color.iter().copied().fold(0.0, f32::max);
    (top.is_finite() && top > 0.0).then(|| color.map(|c| c.max(0.0) / top))
}

/// The looks of the systems whose data sets one, keyed by their skies'
/// seeds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkyLooks(HashMap<u64, SkyLook>);

impl SkyLooks {
    /// No looks: every sky draws the default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sky `seed` draws `look`, in place of any it drew before.
    pub fn insert(&mut self, seed: u64, look: SkyLook) {
        self.0.insert(seed, look);
    }

    /// The look sky `seed` draws: its own, or the default.
    #[must_use]
    pub fn look(&self, seed: u64) -> SkyLook {
        self.0.get(&seed).copied().unwrap_or_default()
    }

    /// How many skies have a look of their own.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no sky has a look of its own.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_view::{Point, SkyPose};

    use super::super::{
        CELL, DENSITY_AREA, LAYERS, StarInstance, cell_bound, layer_stars_of, rushing_cell_bound,
        stars, stars_of,
    };
    use super::*;
    use crate::viewport::{LOGICAL, LogicalSize};

    /// Plenty of texels per unit: no star is grown.
    const FINE: f32 = 8.0;

    fn pose(seed: u64, eye: (f32, f32)) -> SkyPose {
        SkyPose::new(seed, Point::new(eye.0, eye.1), Point::new(512.0, 384.0), 32)
    }

    fn with_stars(stars: f32) -> SkyLook {
        SkyLook {
            stars,
            ..SkyLook::default()
        }
    }

    fn with_tint(tint: [f32; 3]) -> SkyLook {
        SkyLook {
            tint: Some(tint),
            ..SkyLook::default()
        }
    }

    #[test]
    fn the_default_look_gives_the_engines_stars() {
        let seen = pose(3, (120.0, -45.0));
        assert_eq!(
            stars_of(&seen, &SkyLook::default(), LOGICAL, FINE),
            stars(&seen, LOGICAL, FINE)
        );
        assert_eq!(SkyLook::default().stars, 1.0);
        assert_eq!(SkyLook::default().tint, None);
        assert_eq!(SkyLook::default().backdrop, BackdropLook::default());
        assert_eq!(BackdropLook::default().color, None);
        assert_eq!(BackdropLook::default().amount, None);
    }

    #[test]
    fn no_stars_gives_an_empty_star_field() {
        assert!(stars_of(&pose(3, (0.0, 0.0)), &with_stars(0.0), LOGICAL, FINE).is_empty());
    }

    #[test]
    fn twice_the_stars_shows_each_layers_density_twice_on_average() {
        let big = LogicalSize { w: 4096, h: 4096 };
        let area = 4096.0 * 4096.0;
        for (layer, sky_layer) in LAYERS.iter().enumerate() {
            let count = layer_stars_of(&pose(12, (0.0, 0.0)), layer, &with_stars(2.0), big, FINE)
                .len() as f32;
            let expected = 2.0 * sky_layer.density * area / DENSITY_AREA;
            assert!(
                (count - expected).abs() < expected * 0.06,
                "layer {layer}: {count} for {expected}"
            );
        }
    }

    /// Whether `all` holds a star at `star`'s place, of its size and colour.
    fn holds(all: &[StarInstance], star: &StarInstance) -> bool {
        all.iter().any(|other| {
            other.centre == star.centre && other.radius == star.radius && other.color == star.color
        })
    }

    #[test]
    fn a_busier_sky_keeps_every_star_a_sparser_one_shows() {
        for (seed, eye) in [(5, (0.0, 0.0)), (9, (-3000.0, 777.0)), (21, (64.5, 1e5))] {
            let seen = pose(seed, eye);
            for (sparse, busy) in [(0.3, 0.7), (0.7, 1.0), (1.0, 1.6), (1.6, 2.0)] {
                let fewer = stars_of(&seen, &with_stars(sparse), LOGICAL, FINE);
                let more = stars_of(&seen, &with_stars(busy), LOGICAL, FINE);
                assert!(
                    !fewer.is_empty() && more.len() > fewer.len(),
                    "{sparse} {busy}"
                );
                for star in &fewer {
                    assert!(holds(&more, star), "{sparse} → {busy}: {star:?} lost");
                }
            }
        }
    }

    #[test]
    fn with_no_tint_the_colours_are_the_palettes() {
        let seen = pose(8, (40.0, 90.0));
        let look = SkyLook {
            stars: 1.0,
            tint: None,
            backdrop: BackdropLook {
                color: Some([1.0, 0.0, 0.0]),
                amount: Some(1.0),
            },
        };
        assert_eq!(
            stars_of(&seen, &look, LOGICAL, FINE),
            stars(&seen, LOGICAL, FINE)
        );
    }

    #[test]
    fn a_tint_moves_each_stars_palette_tint_its_share_toward_the_bright_tint() {
        let seen = pose(8, (40.0, 90.0));
        let plain = stars(&seen, LOGICAL, FINE);
        // `#ff8040` at full brightness.
        let tint = [1.0, 0.5, 0.25];
        let tinted = stars_of(&seen, &with_tint(tint), LOGICAL, FINE);
        assert_eq!(plain.len(), tinted.len());
        let mut moved = 0;
        for (plain, tinted) in plain.iter().zip(&tinted) {
            assert_eq!((plain.centre, plain.radius), (tinted.centre, tinted.radius));
            // The palette's tints all have a channel at 1, so the
            // brightest channel is the star's light.
            let light = plain.color.iter().copied().fold(0.0, f32::max);
            for ((&from, &to), &got) in plain.color.iter().zip(&tint).zip(&tinted.color) {
                let expected = from + TINT_SHARE * (to * light - from);
                assert!((got - expected).abs() < 1e-5, "{plain:?} {tinted:?}");
            }
            moved += usize::from(tinted.color != plain.color);
        }
        assert!(moved > plain.len() / 2, "{moved} of {}", plain.len());
    }

    #[test]
    fn a_dark_tint_gives_the_colours_of_its_bright_equivalent() {
        let seen = pose(8, (40.0, 90.0));
        let bright = stars_of(&seen, &with_tint([1.0, 0.5, 0.25]), LOGICAL, FINE);
        let dark = stars_of(&seen, &with_tint([0.2, 0.1, 0.05]), LOGICAL, FINE);
        assert_eq!(bright.len(), dark.len());
        for (bright, dark) in bright.iter().zip(&dark) {
            for (a, b) in bright.color.iter().zip(&dark.color) {
                assert!((a - b).abs() < 1e-6, "{bright:?} {dark:?}");
            }
        }
        // `#101010` is white at full brightness.
        let grey = 16.0 / 255.0;
        assert_eq!(
            stars_of(&seen, &with_tint([grey; 3]), LOGICAL, FINE),
            stars_of(&seen, &with_tint([1.0; 3]), LOGICAL, FINE)
        );
    }

    #[test]
    fn a_black_tint_leaves_the_palette() {
        let seen = pose(8, (40.0, 90.0));
        assert_eq!(
            stars_of(&seen, &with_tint([0.0; 3]), LOGICAL, FINE),
            stars(&seen, LOGICAL, FINE)
        );
    }

    #[test]
    fn a_colour_is_normalised_by_its_brightest_channel() {
        assert_eq!(normalised([0.5, 0.25, 0.125]), Some([1.0, 0.5, 0.25]));
        assert_eq!(normalised([0.0, 0.0, 0.2]), Some([0.0, 0.0, 1.0]));
        assert_eq!(normalised([0.0, 0.4, 0.0]), Some([0.0, 1.0, 0.0]));
        assert_eq!(normalised([2.0, 1.0, 0.0]), Some([1.0, 0.5, 0.0]));
        assert_eq!(normalised([-1.0, 0.5, f32::NAN]), Some([0.0, 1.0, 0.0]));
        assert_eq!(normalised([0.0; 3]), None);
        assert_eq!(normalised([-0.5, -1.0, 0.0]), None);
        assert_eq!(normalised([f32::INFINITY, 0.5, 0.0]), None);
    }

    #[test]
    fn a_star_density_out_of_range_is_taken_to_its_end_and_one_not_finite_to_one() {
        assert_eq!(with_stars(0.5).star_density(), 0.5);
        assert_eq!(with_stars(5.0).star_density(), 2.0);
        assert_eq!(with_stars(-1.0).star_density(), 0.0);
        for odd in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(with_stars(odd).star_density(), 1.0, "{odd}");
        }
        let seen = pose(4, (10.0, 10.0));
        assert_eq!(
            stars_of(&seen, &with_stars(9.0), LOGICAL, FINE),
            stars_of(&seen, &with_stars(2.0), LOGICAL, FINE)
        );
        assert!(stars_of(&seen, &with_stars(-3.0), LOGICAL, FINE).is_empty());
        assert_eq!(
            stars_of(&seen, &with_stars(f32::NAN), LOGICAL, FINE),
            stars(&seen, LOGICAL, FINE)
        );
    }

    #[test]
    fn at_twice_the_stars_each_layer_stays_within_its_cells_bound() {
        let still = Point::new(0.0, 0.0);
        for logical in [LOGICAL, LogicalSize { w: 2560, h: 1080 }] {
            for (layer, sky_layer) in LAYERS.iter().enumerate() {
                let per_cell = 2.0 * sky_layer.density * CELL * CELL / DENSITY_AREA;
                let most = per_cell.floor() as usize + 1;
                for (seed, eye) in [(2, (63.9, 64.1)), (7, (-1e5, 3e4))] {
                    for rush in [still, Point::new(30.0, 18.0), Point::new(-1e6, 3e5)] {
                        let seen = pose(seed, eye).with_rush(rush);
                        let count =
                            layer_stars_of(&seen, layer, &with_stars(2.0), logical, FINE).len();
                        let cells = if rush == still {
                            cell_bound(logical)
                        } else {
                            rushing_cell_bound(logical)
                        };
                        assert!(count <= cells * most, "{layer} {rush:?}: {count}");
                        assert!(count > 0, "{layer} {rush:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_seed_with_a_look_draws_it_and_another_the_default() {
        let mut looks = SkyLooks::new();
        assert!(looks.is_empty());
        assert_eq!(looks.look(77), SkyLook::default());
        let sparse = with_stars(0.25);
        looks.insert(77, sparse);
        assert_eq!((looks.len(), looks.is_empty()), (1, false));
        assert_eq!(looks.look(77), sparse);
        assert_eq!(looks.look(78), SkyLook::default());
        looks.insert(77, with_stars(1.5));
        assert_eq!(looks.look(77), with_stars(1.5));
        assert_eq!(looks.len(), 1);
    }
}

/// How the [`Renderer`](crate::Renderer) draws a sky in its look.
#[cfg(test)]
mod batch_tests {
    use nova_view::{DrawList, Point, SkyPose};

    use super::super::instances;
    use super::*;
    use crate::Renderer;
    use crate::gpu::{Batch, Frame};
    use crate::recording::{NoImages, RecordingGpu};
    use crate::viewport::{LogicalSize, Viewport};

    const SMALL: LogicalSize = LogicalSize { w: 256, h: 192 };

    /// Half a pixel a unit, so the target draws at 0.5 texels a unit.
    fn viewport() -> Viewport {
        Viewport::new(SMALL, (128, 96), 0.5)
    }

    fn sky(seed: u64) -> SkyPose {
        SkyPose::new(seed, Point::new(40.0, -70.0), Point::new(128.0, 96.0), 32)
    }

    fn frame_of(renderer: &mut Renderer<NoImages>, seed: u64) -> Frame {
        let mut list = DrawList::new();
        list.sky(sky(seed));
        let mut gpu = RecordingGpu::new();
        renderer.render(&list, &viewport(), &mut gpu);
        gpu.submits()
            .last()
            .map(|frame| (*frame).clone())
            .expect("a frame")
    }

    /// A look unlike the default in every field.
    fn red_and_sparse() -> SkyLook {
        SkyLook {
            stars: 0.5,
            tint: Some([0.0, 0.0, 1.0]),
            backdrop: BackdropLook {
                color: Some([1.0, 0.0, 0.0]),
                amount: Some(1.0),
            },
        }
    }

    #[test]
    fn a_look_registered_for_the_poses_seed_is_the_one_drawn() {
        let mut renderer = Renderer::new(NoImages);
        let mut looks = SkyLooks::new();
        looks.insert(21, red_and_sparse());
        renderer.set_sky_looks(looks);
        let expected = instances(&sky(21), &red_and_sparse(), SMALL, 0.5);
        assert_ne!(
            expected,
            instances(&sky(21), &SkyLook::default(), SMALL, 0.5)
        );
        assert_eq!(frame_of(&mut renderer, 21).batches, [Batch::Sky(expected)]);
    }

    #[test]
    fn another_seeds_look_is_not_drawn() {
        let mut renderer = Renderer::new(NoImages);
        let mut looks = SkyLooks::new();
        looks.insert(22, red_and_sparse());
        renderer.set_sky_looks(looks);
        let expected = instances(&sky(21), &SkyLook::default(), SMALL, 0.5);
        assert_eq!(frame_of(&mut renderer, 21).batches, [Batch::Sky(expected)]);
    }

    #[test]
    fn a_look_that_draws_nothing_is_no_batch() {
        let mut renderer = Renderer::new(NoImages);
        let mut looks = SkyLooks::new();
        let nothing = SkyLook {
            stars: 0.0,
            tint: None,
            backdrop: BackdropLook {
                color: None,
                amount: Some(0.0),
            },
        };
        looks.insert(21, nothing);
        renderer.set_sky_looks(looks);
        assert_eq!(frame_of(&mut renderer, 21).batches, []);
    }
}
