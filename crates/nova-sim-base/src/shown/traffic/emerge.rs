//! How opaque a ship coming out of a gate is drawn ([`emerge_level`], in
//! 32nds of [`FULL_LEVEL`], over its last [`FADE_TICKS`]), from `nova_sim_ai::traffic::emerge`, which
//! re-exports it and keeps the emerge itself: the gate an arrival comes
//! out of and its burst.

/// For how many of its last ticks a ship coming out of a gate fades in
/// (the 16.0 at 0xdd048).
pub const FADE_TICKS: f32 = 16.0;

/// The opacity [`emerge_level`] counts in: 32nds.
pub const FULL_LEVEL: u8 = 32;

/// How opaque a ship coming out of a gate with `ticks_left` of its wait
/// is drawn, in 32nds: `None` while it is hidden, above [`FADE_TICKS`] (or
/// a NaN wait, `ucomiss`/`jb` @0x2c0aa-0x2c0ad), then twice its frames in,
/// clamp(trunc(16.0 - +0x4c), 0, 16) in floats (`subss`, `cvttss2si`,
/// @0x2c0c8-0x2c0e2), as `_HandleShipDisplay` writes its fade as 32 less
/// twice those (@0x2c0e5-0x2c0f1): `32 - 2 * ticks_left` at a whole wait, rising to
/// full; a fractional one truncates its frames in (7.3 and 7.5 both draw
/// at 16).
#[must_use]
pub fn emerge_level(ticks_left: f32) -> Option<u8> {
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if !(ticks_left <= FADE_TICKS) {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    let frames_in = ((FADE_TICKS - ticks_left) as i32).clamp(0, FADE_FRAMES);
    u8::try_from(2 * frames_in).ok()
}

/// The most frames in its fade a ship coming out of a gate counts, 16
/// (`cmpw $0x11`, @0x2c0d9-0x2c0e2).
const FADE_FRAMES: i32 = 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_emerging_ship_is_hidden_above_16_ticks_then_fades_in() {
        assert_eq!(emerge_level(60.0), None);
        assert_eq!(emerge_level(17.0), None);
        assert_eq!(emerge_level(16.0), Some(0));
        assert_eq!(emerge_level(8.0), Some(16));
        assert_eq!(emerge_level(1.0), Some(30));
        assert_eq!(emerge_level(0.0), Some(FULL_LEVEL));
    }

    #[test]
    fn a_fractional_wait_emerging_truncates_its_frames_in_and_hides_past_16() {
        // 16 - 7.3 = 8.7 and 16 - 7.5 = 8.5: both 8 frames in.
        assert_eq!(emerge_level(7.3), Some(16));
        assert_eq!(emerge_level(7.5), Some(16));
        assert_eq!(emerge_level(16.0), Some(0), "at 16: shown, unfaded in");
        assert_eq!(emerge_level(16.25), None, "past 16: hidden");
        assert_eq!(emerge_level(0.25), Some(30), "15.75: 15 frames in");
        assert_eq!(emerge_level(f32::NAN), None, "a NaN wait hides it");
    }
}
