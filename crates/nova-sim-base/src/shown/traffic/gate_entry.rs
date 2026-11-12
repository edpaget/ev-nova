//! How opaque a ship going into a gate is drawn ([`enter_level`]), from
//! `nova_sim_ai::traffic::gate_entry`, which re-exports it and keeps the
//! entry itself: the gate's reach and the approach.

use super::emerge::FULL_LEVEL;

/// How opaque a ship entering a gate with `ticks_left` of its wait is
/// drawn, in 32nds ([`FULL_LEVEL`]): full less the fade
/// `_HandleShipDisplay` writes, clamp(trunc(-2.0 × +0x4c + 32.0), 0, 32)
/// in floats (`mulss` 0xdd650, `addss` 0xdd654, `cvttss2si`,
/// @0x2bfb2-0x2c00e). At a whole wait it is 2 × `ticks_left`, at most
/// full; a fractional one truncates the fade (7.3 and 7.5 both fade 17,
/// so draw at 15).
#[must_use]
pub fn enter_level(ticks_left: f32) -> u8 {
    #[allow(clippy::cast_possible_truncation)]
    let fade = (FADE_SLOPE * ticks_left + FADE_TOP) as i32;
    let fade = fade.clamp(0, i32::from(FULL_LEVEL));
    FULL_LEVEL - u8::try_from(fade).unwrap_or(FULL_LEVEL)
}

/// The fade's slope, the float -2.0 at 0xdd650.
const FADE_SLOPE: f32 = -2.0;

/// The fade's top, the float 32.0 at 0xdd654.
const FADE_TOP: f32 = 32.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ship_entering_fades_out_by_two_32nds_a_tick() {
        assert_eq!(enter_level(16.0), 32);
        assert_eq!(enter_level(17.0), 32);
        assert_eq!(enter_level(15.0), 30);
        assert_eq!(enter_level(1.0), 2);
        assert_eq!(enter_level(0.0), 0);
    }

    #[test]
    fn a_fractional_wait_entering_truncates_its_fade() {
        // -2 × 7.3 + 32 = 17.4 and -2 × 7.5 + 32 = 17.0: both fade 17.
        assert_eq!(enter_level(7.3), 15);
        assert_eq!(enter_level(7.5), 15);
        assert_eq!(enter_level(0.75), 2, "30.5 faded 30");
        assert_eq!(enter_level(-0.25), 0, "faded all the way");
    }
}
