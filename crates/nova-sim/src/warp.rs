//! The player's warp clock: the pieces of the original's warp timing
//! that the session's jump and `_ScanPlayer`'s jump-timing check read.
//!
//! The original (`EV Nova.app/Contents/MacOS/EV Nova`, i386) times the
//! player's warp by three values:
//!
//! - **t**, the Mac ticks (60 a second, `TickCount`) since the warp
//!   stage began: `_HandlePlayer` stamps the player's +0xa4 once as the
//!   stage starts (@0x6b5f0-0x6b5f5), on the first frame of the jump on
//!   which the ship is slow enough or can expedite its jumps, and
//!   restamps it every frame while the ship still brakes
//!   (@0x6b6a2-0x6b6b1). The engine ticks at 30 a second, so t is
//!   [`MAC_TICKS_PER_TICK`] × the engine's ticks since the stamp
//!   ([`mac_ticks`]).
//! - **w**, the ship class's warp factor (class +0x44), which
//!   `_LoadObjectData` sets from the `shïp`'s `Flags` (@0x7ae9d-0x7aef1;
//!   [`warp_factor`]).
//! - **d**, the warp sound's duration in Mac ticks (`_WarpSoundDuration`
//!   @0x8203-0x8224), which `_LoadSounds` works out from `snd ` 128
//!   (@0x1c0b5-0x1c12d; [`sound_duration`]).
//!
//! `_ScanPlayer` (@0x7e562-0x7e633) and the player's own acceleration away
//! (`_HandlePlayer` @0x6d1c1-0x6d296) both read v = t·w / (d·0.01) − 35/w:
//! the scan is held once v > 0 ([`held`]), which is where the player
//! starts to move off. Until then the warp is in its lead-in.
//!
//! Left out: `_WarpSoundDuration`'s second value, `snd ` 129's duration,
//! read in double-speed mode (@0x8210-0x8219), which the engine does not
//! have (rdm:phase/hud-fidelity/phase-20-audio-double-speed-warp); and
//! `_LoadObjectData`'s guard that sets a negative factor to 0.5
//! (@0x7aefe-0x7af07), which no `Flags` reaches.

use crate::clock::TICKS_PER_SECOND;
use crate::mission::screens::MAC_TICKS_PER_SECOND;

/// The Mac's ticks in one of the engine's: 2.
pub const MAC_TICKS_PER_TICK: u32 = MAC_TICKS_PER_SECOND / TICKS_PER_SECOND;

/// The warp sound's duration, in Mac ticks, when there is no `snd ` 128
/// (`_LoadSounds` @0x1c0d2, 0x15e).
pub const NO_SOUND_DURATION: u32 = 350;

/// What the warp factor of every ship class is multiplied by (1.3, f64
/// @0xdd5e8).
const FACTOR_SCALE: f64 = 1.3;

/// What d is multiplied by before t·w is divided by it (0.01, f64
/// @0xdd098).
const DURATION_SCALE: f64 = 0.01;

/// The lead-in's term, over w (−35.0, f64 @0xdd630).
const LEAD_IN: f64 = -35.0;

/// A sound's length as `_ParseSndHeader` gives it: its frames, and the
/// whole part of its 16.16 sample rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundLength {
    /// The frames: samples per channel.
    pub frames: u32,
    /// The whole samples a second: the high half of the 16.16 rate.
    pub rate: u32,
}

/// The engine's ticks since the warp stage began, as the original's
/// Mac ticks.
#[must_use]
pub fn mac_ticks(ticks: u32) -> u32 {
    ticks.saturating_mul(MAC_TICKS_PER_TICK)
}

/// A ship class's warp factor, from its `shïp` `Flags` (`_LoadObjectData`
/// @0x7ae9d-0x7aef1): 0.7 for bit 0x0001, else 1.3 for 0x0002, else 1.6
/// for 0x0004, else 1.0, each an f32, × 1.3 in doubles and stored as an
/// f32 again: 0.91, 1.69, 2.08 or 1.3.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn warp_factor(flags: u16) -> f32 {
    let base: f32 = if flags & 0x0001 != 0 {
        0.7
    } else if flags & 0x0002 != 0 {
        1.3
    } else if flags & 0x0004 != 0 {
        1.6
    } else {
        1.0
    };
    (f64::from(base) * FACTOR_SCALE) as f32
}

/// The warp sound's duration, in Mac ticks, from `sound`, the length of
/// `snd ` 128 (`_LoadSounds` @0x1c0b5-0x1c12d): its frames × 60 over the
/// whole part of its rate, rounded down; [`NO_SOUND_DURATION`] without
/// one.
///
/// The original works it in 32 bits and keeps 16 (`imull`, `divl`,
/// `movw`); the engine keeps it wide. A rate under 1 Hz would trap the
/// original's divide: the engine takes it as no sound.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn sound_duration(sound: Option<SoundLength>) -> u32 {
    match sound {
        Some(SoundLength { frames, rate }) if rate > 0 => {
            let ticks = u64::from(frames) * u64::from(MAC_TICKS_PER_SECOND) / u64::from(rate);
            ticks.min(u64::from(u32::MAX)) as u32
        }
        _ => NO_SOUND_DURATION,
    }
}

/// Whether `_ScanPlayer`'s jump-timing check (@0x7e562-0x7e633) holds a
/// scan back: `since` the Mac ticks since the warp stage began (none
/// before it, or while the ship still brakes), `factor` the ship class's
/// warp factor (w), `duration` the warp sound's (d) and `disabled`
/// whether the player is disabled.
///
/// The check applies only to a player in the warp stage (+0x50 > 0) that
/// is not disabled (`_IsDisabled`). It works v = f32(f64(f32(t) × w) /
/// (f64(d) × 0.01)), then, when w ≠ 0, v = f32(f64(v) + (−35.0 / f64(w))),
/// in the original's widths, and holds the scan when v > 0.0. With w = 0
/// the −35/w term is skipped, so v is 0 or NaN and nothing is held. A d
/// of 0 gives ±∞ or NaN, as in the original.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn held(since: Option<u32>, factor: f32, duration: u32, disabled: bool) -> bool {
    let Some(since) = since else {
        return false;
    };
    if disabled {
        return false;
    }
    // The original makes t exact in f32 by its halves (@0x7e598-0x7e5ba):
    // each half is exact, and their sum rounds once, as this does.
    let t = since as f32;
    let mut v = (f64::from(t * factor) / (f64::from(duration) * DURATION_SCALE)) as f32;
    if factor != 0.0 {
        v = (f64::from(v) + LEAD_IN / f64::from(factor)) as f32;
    }
    v > 0.0
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn the_named_values_are_pinned() {
        assert_eq!(MAC_TICKS_PER_TICK, 2);
        assert_eq!(NO_SOUND_DURATION, 350);
        assert_eq!(mac_ticks(0), 0);
        assert_eq!(mac_ticks(37), 74);
        assert_eq!(mac_ticks(u32::MAX), u32::MAX, "saturates");
    }

    // The warp factor.

    /// The original's f32 of the f64 product of `base` and 1.3.
    #[allow(clippy::cast_possible_truncation)]
    fn scaled(base: f32) -> f32 {
        (f64::from(base) * 1.3) as f32
    }

    #[test]
    fn each_jump_speed_flag_gives_its_warp_factor_bit_exact() {
        assert_eq!(warp_factor(0x0001).to_bits(), scaled(0.7).to_bits());
        assert_eq!(warp_factor(0x0002).to_bits(), scaled(1.3).to_bits());
        assert_eq!(warp_factor(0x0004).to_bits(), scaled(1.6).to_bits());
        assert_eq!(warp_factor(0).to_bits(), scaled(1.0).to_bits());
        assert!((warp_factor(0x0001) - 0.91).abs() < 1e-6);
        assert!((warp_factor(0x0002) - 1.69).abs() < 1e-6);
        assert!((warp_factor(0x0004) - 2.08).abs() < 1e-6);
        assert_eq!(warp_factor(0), 1.3);
    }

    #[test]
    fn the_lowest_jump_speed_bit_wins_and_other_bits_are_ignored() {
        assert_eq!(warp_factor(0x0007), warp_factor(0x0001));
        assert_eq!(warp_factor(0x0005), warp_factor(0x0001));
        assert_eq!(warp_factor(0x0006), warp_factor(0x0002));
        assert_eq!(warp_factor(0xfff8), warp_factor(0), "no jump speed bit");
        assert_eq!(warp_factor(0x8004), warp_factor(0x0004));
    }

    // The warp sound's duration.

    #[test]
    fn the_duration_is_the_frames_times_60_over_the_rate() {
        let warp_up = SoundLength {
            frames: 134_016,
            rate: 22_050,
        };
        assert_eq!(sound_duration(Some(warp_up)), 364, "stock snd 128");
        let exact = SoundLength {
            frames: 22_050,
            rate: 22_050,
        };
        assert_eq!(sound_duration(Some(exact)), 60);
    }

    #[test]
    fn the_duration_divides_by_the_whole_part_of_the_rate() {
        // 22254.54 Hz divides by 22254: 22254 × 100 frames is 6000 ticks
        // exactly, where 22255 would give 5999.
        let sound = SoundLength {
            frames: 2_225_400,
            rate: 22_254,
        };
        assert_eq!(sound_duration(Some(sound)), 6000);
    }

    #[test]
    fn no_sound_or_no_rate_gives_350() {
        assert_eq!(sound_duration(None), 350);
        let still = SoundLength {
            frames: 1000,
            rate: 0,
        };
        assert_eq!(sound_duration(Some(still)), 350);
        let empty = SoundLength {
            frames: 0,
            rate: 22_050,
        };
        assert_eq!(sound_duration(Some(empty)), 0, "a sound of no frames");
    }

    #[test]
    fn a_long_sound_keeps_its_duration_wide() {
        let long = SoundLength {
            frames: u32::MAX,
            rate: 1,
        };
        assert_eq!(sound_duration(Some(long)), u32::MAX, "capped");
        let wide = SoundLength {
            frames: 1_000_000,
            rate: 22_050,
        };
        assert_eq!(sound_duration(Some(wide)), 2721, "past an i16's 16 bits");
    }

    // The hold.

    #[test]
    fn no_clock_holds_nothing_and_neither_does_a_disabled_player() {
        let w = warp_factor(0);
        assert!(!held(None, w, 350, false));
        assert!(held(Some(1000), w, 350, false));
        assert!(!held(Some(1000), w, 350, true), "disabled");
    }

    #[test]
    fn the_lead_in_ends_past_0_35_d_over_w_squared() {
        // 0.35 × 350 / 1.69 = 72.49 Mac ticks.
        let w = warp_factor(0);
        assert!(!held(Some(0), w, 350, false));
        assert!(!held(Some(72), w, 350, false));
        assert!(held(Some(74), w, 350, false));
    }

    #[test]
    fn the_warp_factor_moves_the_end_of_the_lead_in() {
        // 0.91: 0.35 × 350 / 0.8281 = 147.9.
        let slow = warp_factor(0x0001);
        assert!(!held(Some(146), slow, 350, false));
        assert!(held(Some(148), slow, 350, false));
        // 2.08: 0.35 × 350 / 4.3264 = 28.3.
        let fast = warp_factor(0x0004);
        assert!(!held(Some(28), fast, 350, false));
        assert!(held(Some(30), fast, 350, false));
        // The duration moves it too: 0.35 × 364 / 1.69 = 75.4.
        let w = warp_factor(0);
        assert!(!held(Some(74), w, 364, false));
        assert!(held(Some(76), w, 364, false));
    }

    #[test]
    fn a_warp_factor_of_0_never_holds() {
        for since in [0, 2, 74, 1000, u32::MAX] {
            for duration in [0, 350] {
                assert!(
                    !held(Some(since), 0.0, duration, false),
                    "{since} {duration}"
                );
            }
        }
    }

    #[test]
    fn a_duration_of_0_holds_once_any_time_has_passed() {
        // t·w / 0 is +∞ once t > 0, and +∞ − 35/w is still +∞; at t = 0 it
        // is 0 / 0, NaN, which is never above 0.
        let w = warp_factor(0);
        assert!(!held(Some(0), w, 0, false), "NaN");
        assert!(held(Some(2), w, 0, false), "+∞");
    }

    #[test]
    fn a_nan_warp_factor_never_holds() {
        // The original's `jp` takes the −35/w branch for a NaN factor
        // (@0x7e60c), which leaves v NaN.
        assert!(!held(Some(1000), f32::NAN, 350, false));
    }
}
