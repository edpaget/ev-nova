//! An ionized ship's glow, as `_HandleShipDisplay` (@0x2beb9-0x2bf94)
//! works it out each frame for every ship in the player's system.
//!
//! - **The charge gone** (ion ≤ 0): the ship's ion colour is cleared
//!   (`movl $0,0xb0` @0x2bf8a). As the display runs for every ship each
//!   frame, and before the first frame of every arrival, take-off and
//!   gate exit (`_PrepareAllSprites` @0x3a75a-0x3a78e), a colour never
//!   outlives its charge by more than a frame.
//! - **Too faint** (factor below 0.33 as a double, @0x2beed): nothing is
//!   written; the colour is kept.
//! - **Otherwise** the tint's level is [`tint_level`] (16 to 24) plus
//!   `Rand(5)` − 2 (@0x2bf24-0x2bf39), a flicker of 14 to 26 each frame,
//!   in the ship's ion colour ([`IonTint`]). Whether it beats the murk the
//!   hull already has is the view's to work out, with the murk
//!   (@0x2bf3d-0x2bf82,
//!   [`RuleKey::IonTintMurkColour`](crate::RuleKey::IonTintMurkColour)).
//!
//! The factor here is the ion over the capacity, not capped at 0.7. The
//! original compares the level as a 16-bit word (`cmpw` @0x2bf0b); the
//! engine keeps it wide. None of this plays a sound.

use super::ion::Ion;
use crate::chance::Chance;
use crate::ionization::factor;

/// The factor below which an ionized ship shows no tint (the double 0.33
/// @0xdd568).
pub const TINT_FROM: f64 = 0.33;
/// The tint's least base level (`cmpw $0xf`, raised to 16 @0x2bf21).
pub const TINT_LEAST: i64 = 16;
/// The tint's most base level (`cmpw $0x18` @0x2bf0b).
pub const TINT_MOST: i64 = 24;
/// The factor's scale before the hundredth (the float 24.0 @0xdd040).
pub const TINT_SCALE: f32 = 24.0;
/// The flicker's spread, `Rand(5)` (@0x2bf24).
pub const FLICKER: u32 = 5;

/// The colorize an ionized ship's hull takes this frame: its level, out
/// of 32, and its ion colour, `00RRGGBB`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IonTint {
    /// The level, 14 to 26 (the original's +0xb0).
    pub level: u8,
    /// The ship's ion colour (its +0xb0, the sprite's +0xb4).
    pub colour: u32,
}

/// The tint's base level at ionization factor `factor`, none below
/// [`TINT_FROM`]: trunc(factor × 24.0 as a float, × 0.01 as a double),
/// [`TINT_MOST`] at most and [`TINT_LEAST`] at least (@0x2bef3-0x2bf21).
/// So it is 16 until the factor reaches 66.7.
#[must_use]
pub fn tint_level(factor: f32) -> Option<i64> {
    if f64::from(factor) < TINT_FROM {
        return None;
    }
    let level = (f64::from(factor * TINT_SCALE) * 0.01).trunc();
    // Clamped to 16..=24 at once, so any size casts.
    #[allow(clippy::cast_possible_truncation)]
    let level = level.clamp(0.0, 1.0e9) as i64;
    Some(level.clamp(TINT_LEAST, TINT_MOST))
}

/// The tint a ship of ionization `ion` and capacity `capacity` shows this
/// frame, rolling its flicker on `chance` (see the module docs), its
/// colour cleared when its charge is gone.
pub fn display(
    ion: &mut Ion,
    capacity: f32,
    chance: &mut (impl Chance + ?Sized),
) -> Option<IonTint> {
    if ion.charge <= 0.0 {
        ion.colour = 0;
        return None;
    }
    let base = tint_level(factor(ion.charge, capacity))?;
    let flicker = i64::from(chance.below(FLICKER)) - 2;
    let level = u8::try_from(base + flicker).unwrap_or(u8::MAX);
    Some(IonTint {
        level,
        colour: ion.colour,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use nova_sim_base::fixture::Draws;

    #[test]
    fn below_0_33_there_is_no_tint() {
        assert_eq!(tint_level(0.329), None);
        assert_eq!(tint_level(0.0), None);
        assert_eq!(tint_level(-1.0), None);
        assert_eq!(tint_level(0.33), Some(16), "0.33 as a float is past it");
        assert_eq!(TINT_FROM, 0.33);
    }

    #[test]
    fn the_base_level_is_16_until_a_factor_of_66_7() {
        for factor in [0.5, 1.0, 3.0, 66.0] {
            assert_eq!(tint_level(factor), Some(16), "{factor}");
        }
    }

    #[test]
    fn past_66_7_the_level_is_trunc_of_a_quarter_less_a_hundredth() {
        assert_eq!(tint_level(83.3), Some(19), "19.99");
        assert_eq!(tint_level(84.0), Some(20), "20.16");
        assert_eq!(tint_level(100.0), Some(24));
        assert_eq!(tint_level(200.0), Some(24), "held at 24");
        assert_eq!(tint_level(f32::MAX), Some(24));
        assert_eq!((TINT_LEAST, TINT_MOST, TINT_SCALE), (16, 24, 24.0));
    }

    #[test]
    fn the_flicker_spreads_the_level_2_either_way() {
        for (roll, level) in [(0, 14), (1, 15), (2, 16), (3, 17), (4, 18)] {
            let mut ion = Ion {
                charge: 200.0,
                colour: 0x0000_80ff,
                ..Ion::default()
            };
            let tint = display(&mut ion, 400.0, &mut Draws::of(&[roll]));
            assert_eq!(
                tint,
                Some(IonTint {
                    level,
                    colour: 0x0000_80ff
                }),
                "{roll}"
            );
        }
        let mut ion = Ion {
            charge: 100.0,
            colour: 1,
            ..Ion::default()
        };
        assert_eq!(
            display(&mut ion, 1.0, &mut Draws::of(&[4])).map(|t| t.level),
            Some(26)
        );
    }

    #[test]
    fn the_flicker_rolls_rand_5() {
        let mut draws = Draws::of(&[0]);
        let mut ion = Ion {
            charge: 200.0,
            colour: 1,
            ..Ion::default()
        };
        let _ = display(&mut ion, 400.0, &mut draws);
        assert_eq!(draws.asked, [FLICKER]);
    }

    #[test]
    fn with_no_charge_the_colour_is_cleared() {
        for charge in [0.0, -0.5] {
            let mut ion = Ion {
                charge,
                colour: 0x00ff_0000,
                ..Ion::default()
            };
            assert_eq!(display(&mut ion, 400.0, &mut Draws::of(&[])), None);
            assert_eq!(
                ion,
                Ion {
                    charge,
                    colour: 0,
                    ..Ion::default()
                }
            );
        }
    }

    #[test]
    fn a_faint_charge_keeps_its_colour_and_shows_no_tint() {
        let mut ion = Ion {
            charge: 100.0,
            colour: 0x00ff_0000,
            ..Ion::default()
        };
        let mut draws = Draws::of(&[]);
        assert_eq!(display(&mut ion, 400.0, &mut draws), None);
        assert_eq!(ion.colour, 0x00ff_0000);
        assert!(draws.asked.is_empty(), "no roll");
    }

    #[test]
    fn no_capacity_shows_no_tint() {
        let mut ion = Ion {
            charge: 100.0,
            colour: 0x00ff_0000,
            ..Ion::default()
        };
        assert_eq!(display(&mut ion, 0.0, &mut Draws::of(&[])), None);
        assert_eq!(ion.colour, 0x00ff_0000);
    }
}
