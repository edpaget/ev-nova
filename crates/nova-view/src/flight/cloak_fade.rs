//! A cloaking ship as `_HandleShipDisplay` draws it, in the `EV Nova`
//! executable at 16-bit depth, from how its cloak shows it
//! (`nova_sim::cloak::CloakShown`).
//!
//! - **Fading** (0 < fade < 32, @0x2d2c0-0x2d648), every ship, whoever
//!   leads it: the hull, the alt layer, the glow, the lights and the
//!   weapon glow are moved by the same jitter, each axis
//!   n − `Rand`(2n + 1) for n = trunc(fade / 10) ([`jitter_reach`],
//!   @0x2d2ca-0x2d31e); the shield is not. The hull's translucency is
//!   0x20 and each channel factor trunc(paint − fade), or, at 1 or
//!   less, 2 for a channel of paint above 15 and 0 otherwise
//!   ([`hull_factor`], @0x2d323-0x2d440). Through
//!   `_BlitPixieRLETranslucent` (@0xc1568) that is `((src × n) >> 5) |
//!   dst`, [`Blend::Or`] at [`lights_tint`]`(n)`. The alt layer copies
//!   the hull's (@0x2d447-0x2d4b5). The glow and lights are held to
//!   trunc(32 − fade) ([`layer_cap`], @0x2d4ed-0x2d641), the weapon glow
//!   to trunc(40 − fade) ([`weapon_cap`], @0x2d6a3-0x2d72e).
//! - **Fully faded** (@0x2d187-0x2d2bb): the hull at translucency 0x1e
//!   with its channel factors at [`full_fade_factor`] (2, unpainted)
//!   draws `((src × 4) >> 5) | ((dst × 30) >> 5)`
//!   ([`Blend::OrTranslucent`]); at 0x40 it is not drawn. The alt layer
//!   copies the hull's. The glow, lights and shield are hidden
//!   (@0x2d24f-0x2d2ad). The weapon glow, unjittered, is held to
//!   trunc(40 − 32) = 8, even with the hull not drawn
//!   (@0x2d287-0x2d290).
//!
//! The view rolls the jitter from hashed rolls of its own, per ship and
//! tick ([`cloak_rolls`]), not the original's `Rand` draws. A ship's paint
//! is taken as unpainted ([`UNPAINTED`], `_GetShipPaintColor` @0x7d0c's
//! default), as the engine draws no paint.

use nova_sim::HashedRolls;
use nova_sim::blink::BlinkChance;
use nova_sim::cloak::{CloakShown, FADE_FULL, TRANSLUCENCY_STEPS};
use nova_sim::traffic::npc::NpcId;

use crate::draw::{lights_tint, translucent_tint};
use crate::{Blend, Color, Point};

/// An unpainted ship's paint, each channel in 32nds
/// (`_GetShipPaintColor` @0x7d0c).
pub const UNPAINTED: u8 = 32;

/// The fade a step of the jitter's reach takes (the float 10.0 @0xdd65c).
pub const JITTER_STEP: f32 = 10.0;

/// The fade the weapon glow's cap is taken from (the float 40.0 @0xdd660).
pub const WEAPON_CAP_FROM: f32 = 40.0;

/// How far, in screen pixels on each axis, a ship at `fade` is jittered:
/// trunc(fade / 10).
#[must_use]
pub fn jitter_reach(fade: f32) -> u16 {
    // Within 0..=3 for a fade below 32.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let reach = (fade / JITTER_STEP).trunc().max(0.0) as u16;
    reach
}

/// A channel's factor below a full fade: trunc(`paint` − `fade`), or, at 1
/// or less, 2 for a `paint` above 15 and 0 otherwise.
#[must_use]
pub fn hull_factor(paint: u8, fade: f32) -> u8 {
    let factor = (f32::from(paint) - fade).trunc();
    if factor <= 1.0 {
        full_fade_factor(paint)
    } else {
        // Within 2..=32 here.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let factor = factor as u8;
        factor
    }
}

/// A channel's factor at a full fade: 2 for a `paint` above 15, else 0.
#[must_use]
pub fn full_fade_factor(paint: u8) -> u8 {
    if paint > 15 { 2 } else { 0 }
}

/// trunc(`from` − `fade`), held to 0..=32.
fn cap(from: f32, fade: f32) -> u8 {
    // Held to 0..=32.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let level = (from - fade).trunc().clamp(0.0, FADE_FULL) as u8;
    level
}

/// The level the glow and lights are held to at `fade`: trunc(32 − fade).
#[must_use]
pub fn layer_cap(fade: f32) -> u8 {
    cap(FADE_FULL, fade)
}

/// The level the weapon glow is held to at `fade`: trunc(40 − fade), 8 at
/// a full fade.
#[must_use]
pub fn weapon_cap(fade: f32) -> u8 {
    cap(WEAPON_CAP_FROM, fade)
}

/// The rolls NPC `id`'s cloak jitters on: its own, apart from its glow's.
#[must_use]
pub fn cloak_rolls(id: NpcId) -> HashedRolls {
    HashedRolls::new(0x434c_4f41_4b00_0000 ^ u64::from(id.0))
}

/// The jitter of reach `reach` at `tick`, rolled on `rolls`: n −
/// roll(2n + 1) on each axis, each within ±`reach`.
#[must_use]
pub fn jitter(reach: u16, tick: u64, rolls: &impl BlinkChance) -> Point {
    let sides = 2 * reach + 1;
    let axis = |change| f32::from(reach) - f32::from(rolls.roll(change, sides));
    Point::new(axis(tick.wrapping_mul(2)), axis(tick.wrapping_mul(2) + 1))
}

/// How a cloak changes a ship's drawing (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloakDraw {
    /// Its hull's tint and blend in place of its own, murk and all, when
    /// drawn: the alt layer takes it too.
    pub hull: Option<(Color, Blend)>,
    /// How far its hull, alt, glow, lights and weapon glow are moved.
    pub offset: Point,
    /// The level its glow and lights are held to, or `None` when hidden.
    pub layers: Option<u8>,
    /// The level its weapon glow is held to.
    pub weapon: u8,
    /// Whether its shield layer is drawn.
    pub shield: bool,
}

impl CloakDraw {
    /// How a ship `shown` so is drawn, its jitter at `tick` rolled on
    /// `rolls`, or `None` when its cloak changes nothing.
    #[must_use]
    pub fn of(shown: CloakShown, tick: u64, rolls: &impl BlinkChance) -> Option<Self> {
        let full = |hull| Self {
            hull,
            offset: Point::new(0.0, 0.0),
            layers: None,
            weapon: weapon_cap(FADE_FULL),
            shield: false,
        };
        match shown {
            CloakShown::Opaque => None,
            CloakShown::Hidden => Some(full(None)),
            CloakShown::Translucent(level) => {
                let level = level.min(TRANSLUCENCY_STEPS);
                // The source factor is the channel's − +0xa8 + 32, capped
                // at 32 (`_BlitPixieRLETranslucent` @0xc1568).
                let source = (full_fade_factor(UNPAINTED) + 32)
                    .saturating_sub(level)
                    .min(32);
                let tint = translucent_tint(source, level);
                Some(full(Some((tint, Blend::OrTranslucent))))
            }
            CloakShown::Fading(fade) => Some(Self {
                hull: Some((lights_tint(hull_factor(UNPAINTED, fade)), Blend::Or)),
                offset: jitter(jitter_reach(fade), tick, rolls),
                layers: Some(layer_cap(fade)),
                weapon: weapon_cap(fade),
                shield: true,
            }),
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn the_jitter_reaches_a_pixel_for_each_10_of_fade() {
        let reaches = [0.0, 9.9, 10.0, 19.9, 20.0, 31.9].map(jitter_reach);
        assert_eq!(reaches, [0, 0, 1, 1, 2, 3]);
    }

    #[test]
    fn a_hull_channels_factor_is_its_paint_less_the_fade_or_2_or_0_at_1_or_less() {
        let cases = [
            (32, 0.5, 31),
            (32, 10.9, 21),
            (32, 30.0, 2),
            (32, 30.5, 2),
            (32, 31.0, 2),
            (32, 31.9, 2),
            (15, 5.0, 10),
            (15, 13.5, 0),
            (15, 14.0, 0),
            (16, 15.0, 2),
        ];
        for (paint, fade, factor) in cases {
            assert_eq!(hull_factor(paint, fade), factor, "{paint} {fade}");
        }
    }

    #[test]
    fn a_full_fades_channel_factor_is_2_above_15_and_0_otherwise() {
        assert_eq!([32, 16, 15, 0].map(full_fade_factor), [2, 2, 0, 0]);
    }

    #[test]
    fn the_glow_and_lights_are_held_to_32_less_the_fade_and_the_weapon_glow_to_40() {
        assert_eq!(
            [0.0, 0.5, 10.9, 31.9, 32.0].map(layer_cap),
            [32, 31, 21, 0, 0]
        );
        assert_eq!([0.0, 10.9, 31.9, 32.0].map(weapon_cap), [32, 29, 8, 8]);
    }

    /// Rolls that always come up `value`, held below the sides.
    struct Always(u16);

    impl BlinkChance for Always {
        fn roll(&self, _change: u64, sides: u16) -> u16 {
            self.0.min(sides - 1)
        }
    }

    #[test]
    fn the_jitter_is_reach_less_a_roll_of_twice_it_and_one_each_way() {
        assert_eq!(jitter(3, 5, &Always(0)), Point::new(3.0, 3.0));
        assert_eq!(jitter(3, 5, &Always(6)), Point::new(-3.0, -3.0));
        assert_eq!(jitter(0, 5, &Always(6)), Point::new(0.0, 0.0));
        let rolls = cloak_rolls(NpcId(4));
        for tick in 0..200 {
            let at = jitter(2, tick, &rolls);
            assert!(at.x.abs() <= 2.0 && at.y.abs() <= 2.0, "{at:?}");
        }
        let spread: std::collections::BTreeSet<_> = (0..200)
            .map(|tick| jitter(2, tick, &rolls))
            .map(|at| (at.x as i8, at.y as i8))
            .collect();
        assert_eq!(spread.len(), 25, "every offset comes up");
    }

    #[test]
    fn each_ship_jitters_on_its_own_rolls() {
        let a = (0..20).map(|tick| jitter(3, tick, &cloak_rolls(NpcId(0))));
        let b = (0..20).map(|tick| jitter(3, tick, &cloak_rolls(NpcId(1))));
        assert!(a.ne(b));
    }

    #[test]
    fn an_opaque_ship_is_drawn_as_it_is() {
        assert_eq!(CloakDraw::of(CloakShown::Opaque, 0, &Always(0)), None);
    }

    #[test]
    fn a_fading_ship_draws_its_hull_ored_jittered_and_its_layers_capped() {
        let draw = CloakDraw::of(CloakShown::Fading(21.5), 0, &Always(0)).expect("fading");
        assert_eq!(
            draw,
            CloakDraw {
                hull: Some((lights_tint(10), Blend::Or)),
                offset: Point::new(2.0, 2.0),
                layers: Some(10),
                weapon: 18,
                shield: true,
            }
        );
        let low = CloakDraw::of(CloakShown::Fading(31.0), 0, &Always(0)).expect("fading");
        assert_eq!(low.hull, Some((lights_tint(2), Blend::Or)), "held at 2");
    }

    #[test]
    fn a_full_fade_draws_its_hull_at_4_over_30_or_not_and_only_its_weapon_glow_at_8() {
        let faint = CloakDraw::of(CloakShown::Translucent(0x1e), 9, &Always(0)).expect("faint");
        assert_eq!(
            faint,
            CloakDraw {
                hull: Some((translucent_tint(4, 30), Blend::OrTranslucent)),
                offset: Point::new(0.0, 0.0),
                layers: None,
                weapon: 8,
                shield: false,
            }
        );
        let hidden = CloakDraw::of(CloakShown::Hidden, 9, &Always(0)).expect("hidden");
        assert_eq!(
            hidden,
            CloakDraw {
                hull: None,
                ..faint
            }
        );
    }

    mod drawing {
        use std::num::NonZeroU16;

        use nova_sim::combat::ship_frames::{Alt, Drawn, FrameSpec};

        use super::super::*;
        use super::Always;
        use crate::draw::lights_tint;
        use crate::flight::catalog::{LayerSheet, ShipSheet};
        use crate::flight::ship_layers::ShipDraw;
        use crate::{DrawCommand, DrawList};

        const AT: Point = Point::new(100.0, 50.0);

        fn layer(image_id: i16) -> LayerSheet {
            LayerSheet {
                image_id,
                frames: NonZeroU16::new(36).expect("non-zero"),
                solid: false,
            }
        }

        /// A hull 1000 with a glow 1100, lights 1200, weapon glow 1930,
        /// alt 1400 and shield 1500.
        fn sheet() -> ShipSheet {
            ShipSheet {
                image_id: 1000,
                rotations: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 10,
                frame_height: 10,
                glow: Some(layer(1100)),
                lights: Some(layer(1200)),
                weapon: Some(layer(1930)),
                weap_decay: 0,
                alt: Some(layer(1400)),
                shield: Some(layer(1500)),
                base_transp: 0,
                solid: false,
            }
        }

        /// Each sprite `cloak` leaves of a ship with every layer at 20: its
        /// image, place, tint and blend.
        fn drawn(cloak: Option<CloakDraw>) -> Vec<(i16, Point, Color, Blend)> {
            let sheet = sheet();
            let spec = FrameSpec::NONE;
            let ship = ShipDraw {
                sheet: &sheet,
                spec: &spec,
                drawn: Drawn {
                    alt: Alt::Drawn(0),
                    ..Drawn::default()
                },
                heading: 0.0,
                tint: Color::WHITE,
                cloak,
                hull: cloak.is_none_or(|cloak| cloak.hull.is_some()),
                murk: 0,
                held: true,
                glow: Some(20),
                lights: Some(20),
                weapon: Some(20),
                shield: Some(20),
                colorize: None,
            };
            let mut list = DrawList::new();
            ship.draw(&mut list, AT, ship.live_alt(AT));
            list.iter()
                .filter_map(|command| match *command {
                    DrawCommand::Sprite {
                        image,
                        center,
                        tint,
                        blend,
                    } => Some((image.id, center, tint, blend)),
                    _ => None,
                })
                .collect()
        }

        #[test]
        fn fading_jitters_all_but_the_shield_ors_the_hull_and_alt_and_caps_the_layers() {
            let cloak = CloakDraw::of(CloakShown::Fading(21.5), 0, &Always(0));
            let moved = Point::new(102.0, 52.0);
            let hull = lights_tint(10);
            assert_eq!(
                drawn(cloak),
                [
                    (1000, moved, hull, Blend::Or),
                    (1100, moved, lights_tint(10), Blend::Or),
                    (1200, moved, lights_tint(10), Blend::Or),
                    (1930, moved, lights_tint(18), Blend::Or),
                    (1400, moved, hull, Blend::Or),
                    (1500, AT, lights_tint(20), Blend::Or),
                ]
            );
        }

        #[test]
        fn a_low_fade_holds_no_layer_below_its_own_level() {
            let cloak = CloakDraw::of(CloakShown::Fading(2.5), 0, &Always(0));
            let levels: Vec<_> = drawn(cloak)
                .iter()
                .map(|&(id, _, tint, _)| (id, tint))
                .collect();
            assert_eq!(
                levels,
                [
                    (1000, lights_tint(29)),
                    (1100, lights_tint(20)),
                    (1200, lights_tint(20)),
                    (1930, lights_tint(20)),
                    (1400, lights_tint(29)),
                    (1500, lights_tint(20)),
                ]
            );
        }

        #[test]
        fn a_full_fade_draws_the_hull_and_alt_faint_and_the_weapon_glow_at_8_alone() {
            let faint = translucent_tint(4, 30);
            let cloak = CloakDraw::of(CloakShown::Translucent(0x1e), 0, &Always(0));
            assert_eq!(
                drawn(cloak),
                [
                    (1000, AT, faint, Blend::OrTranslucent),
                    (1930, AT, lights_tint(8), Blend::Or),
                    (1400, AT, faint, Blend::OrTranslucent),
                ]
            );
            let hidden = CloakDraw::of(CloakShown::Hidden, 0, &Always(0));
            assert_eq!(drawn(hidden), [(1930, AT, lights_tint(8), Blend::Or)]);
        }

        #[test]
        fn with_no_cloak_the_hull_is_drawn_plainly_and_every_layer_as_it_is() {
            let plain = drawn(None);
            assert_eq!(plain[0], (1000, AT, Color::WHITE, Blend::Normal));
            assert_eq!(plain.len(), 6);
            assert!(plain.iter().all(|&(_, at, ..)| at == AT));
        }
    }
}
