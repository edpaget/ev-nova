//! The colorize an ionized ship's hull is drawn with, as
//! `_HandleShipDisplay` (@0x2bf3d-0x2bf82) writes it over the murk the
//! hull already has (`nova_sim::combat::ion_tint` rolls the tint).
//!
//! With m the hull's sprite murk ([`sprite_murk`](super::murk::sprite_murk),
//! 0 for the player) and v the tint's level (14 to 26):
//!
//! - **v > m** (@0x2bf48-0x2bf57): the hull is colorized at level v toward
//!   the ship's ion colour.
//! - **m ≥ v**: by the engine (@0x2bf78-0x2bf82) the level stays m and the
//!   colour becomes `32 − m` as an RGB555 pixel, a dark blue of 32 − m out
//!   of 31; by the other reading of
//!   [`RuleKey::IonTintMurkColour`](nova_sim::RuleKey::IonTintMurkColour),
//!   at level m toward the ion colour, as the 8-bit path (@0x2bf70) has it.
//!
//! The colorize takes the murk's one slot (sprite +0xb0/+0xb4), so a hull
//! drawn colorized is drawn at its unmurked tint. The layers that copy the
//! hull's colorize take it too (see the
//! [`ship_layers`](super::ship_layers) module).

use nova_sim::RuleSource;
use nova_sim::combat::ion_tint::IonTint;

use crate::Colorize;
use crate::colorize::COLORIZE_STEPS;

/// The colorize a hull of sprite murk `murk` takes from the ion `tint` it
/// shows this frame, the m ≥ v colour as `murk_colour` reads (see the
/// module docs); none without a tint, when the hull keeps its murk.
#[must_use]
pub fn hull_colorize(murk: u8, tint: Option<IonTint>, murk_colour: RuleSource) -> Option<Colorize> {
    let tint = tint?;
    if tint.level > murk {
        return Some(Colorize::toward(tint.level, tint.colour));
    }
    Some(match murk_colour {
        RuleSource::Engine => {
            Colorize::toward_pixel(murk, u16::from(COLORIZE_STEPS.saturating_sub(murk)))
        }
        RuleSource::Bible => Colorize::toward(murk, tint.colour),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TINT: IonTint = IonTint {
        level: 16,
        colour: 0x0034_c2ff,
    };

    #[test]
    fn no_tint_leaves_the_hull_its_murk() {
        for rule in RuleSource::ALL {
            assert_eq!(hull_colorize(20, None, rule), None);
            assert_eq!(hull_colorize(0, None, rule), None);
        }
    }

    #[test]
    fn a_tint_deeper_than_the_murk_colorizes_at_its_level_in_the_ion_colour() {
        for murk in [0, 7, 15] {
            for rule in RuleSource::ALL {
                assert_eq!(
                    hull_colorize(murk, Some(TINT), rule),
                    Some(Colorize::toward(16, 0x0034_c2ff)),
                    "{murk} {rule:?}"
                );
            }
        }
    }

    #[test]
    fn by_the_engine_a_murk_as_deep_takes_32_less_it_as_a_pixel_at_its_level() {
        assert_eq!(
            hull_colorize(16, Some(TINT), RuleSource::Engine),
            Some(Colorize {
                level: 16,
                rgb5: [0, 0, 16]
            })
        );
        assert_eq!(
            hull_colorize(25, Some(TINT), RuleSource::Engine),
            Some(Colorize {
                level: 25,
                rgb5: [0, 0, 7]
            })
        );
    }

    #[test]
    fn by_the_other_reading_a_murk_as_deep_takes_the_ion_colour_at_its_level() {
        assert_eq!(
            hull_colorize(25, Some(TINT), RuleSource::Bible),
            Some(Colorize::toward(25, 0x0034_c2ff))
        );
        assert_eq!(
            hull_colorize(16, Some(TINT), RuleSource::Bible),
            Some(Colorize::toward(16, 0x0034_c2ff))
        );
    }
}
