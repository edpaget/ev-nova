//! An NPC's AI wait, the original's float +0x4c: [`Npc::ai_wait`] while
//! it flies, and the `ticks_left` of [`Mode::Emerging`],
//! [`Mode::HeldAtGate`], [`Mode::Coasting`] and [`Mode::EnteringGate`].
//!
//! - **It runs while above none.** Every reader tests `+0x4c > 0.0` as a
//!   float (`ucomiss`): `_HandleShip`'s turn skip (@0x34442) and its
//!   steering, thrust and fire skip (@0x34a57), its count-down
//!   (@0x3502a), `_HighLevelAIHandler`'s count wait (@0x8d474) and the
//!   drift's end (@0x8ebc5), and `_IsThreatToPlayer` (@0x7f51b)
//!   ([`running`]). The pirate's dock compares it against 100.0 as well
//!   (@0x86d54), the gate fade against 16.0, 32.0 and 1.0
//!   (`_HandleShipDisplay` @0x2bfb2-0x2c12b), and a hit cuts it to 20.0
//!   (`_DamageShip` @0x3be6e-0x3be7c): each reads it as a float.
//! - **It counts down by the frame's gain.** While it runs and the ship
//!   is not an escape pod (class 0x2ff, @0x3502f), `_HandleShip` takes
//!   the gain off it, `(float)((double)+0x4c - g)` with no floor
//!   (@0x35037-0x35048, [`counted_down`]). So a wait of 0.25 at a gain of
//!   0.5 reads -0.25 and is out. The glow's -1 that follows
//!   (@0x35069-0x35079) and the breakup timer's -1.0 (+0x3c, @0x3509b)
//!   read no gain.
//! - **Its writes are whole floats.** Every write the engine makes from
//!   a roll or a ship's data is a whole number (`cvtsi2ss`, as the dock's
//!   `Rand(80) + 100` @0x86fdf), which [`ticks`] gives.
//!
//! A pod's wait is [`Mode::Adrift`]'s, which `_AIDispatch` counts
//! (@0x8fb8b), not this; no other writer gives a pod a wait.
//!
//! No `PlaySound` or `_beepSnd` is made.
//!
//! [`Npc::ai_wait`]: crate::traffic::npc::Npc::ai_wait
//! [`Mode::Emerging`]: crate::traffic::npc::Mode::Emerging
//! [`Mode::HeldAtGate`]: crate::traffic::npc::Mode::HeldAtGate
//! [`Mode::Coasting`]: crate::traffic::npc::Mode::Coasting
//! [`Mode::EnteringGate`]: crate::traffic::npc::Mode::EnteringGate
//! [`Mode::Adrift`]: crate::traffic::npc::Mode::Adrift

use crate::speed_mult::SpeedMult;

/// Whether a wait of `wait` runs: above none, as every reader tests it
/// (`ucomiss`, see the module docs). A NaN wait does not run.
#[must_use]
pub fn running(wait: f32) -> bool {
    wait > 0.0
}

/// `wait` after a frame at the frame's `gain`: while it runs, the gain
/// taken off it in doubles and narrowed, unfloored (@0x35037-0x35048);
/// otherwise as it was.
#[must_use]
pub fn counted_down(wait: f32, gain: SpeedMult) -> f32 {
    if running(wait) {
        gain.add_times(wait, -1.0)
    } else {
        wait
    }
}

/// A wait of `n` whole frames, as the original converts a roll or a
/// ship's datum to its float (`cvtsi2ss`).
#[must_use]
pub fn ticks(n: u32) -> f32 {
    // Exact below 2^24: every wait the data and the rolls give is far
    // below it.
    #[allow(clippy::cast_precision_loss)]
    let wait = n as f32;
    wait
}

/// The whole frames of `wait`, a whole wait: the tests' count of a wait's
/// frames.
#[cfg(test)]
#[must_use]
pub(crate) fn frames(wait: f32) -> u32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let frames = wait as u32;
    frames
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const HALF: SpeedMult = SpeedMult::new(0.5);

    #[test]
    fn a_wait_runs_only_above_none() {
        assert!(running(0.25));
        assert!(running(120.0));
        assert!(!running(0.0));
        assert!(!running(-0.25));
        assert!(!running(f32::NAN));
    }

    #[test]
    fn at_half_gain_a_wait_counts_down_half_a_frame_unfloored() {
        assert_eq!(counted_down(5.0, HALF), 4.5);
        let left = counted_down(0.25, HALF);
        assert_eq!(left, -0.25, "no floor");
        assert!(!running(left), "and out");
        assert_eq!(counted_down(0.0, HALF), 0.0, "an ended wait stays");
        assert_eq!(counted_down(-1.0, HALF), -1.0);
        assert_eq!(counted_down(5.0, SpeedMult::ZERO), 5.0, "no gain: still");
        assert_eq!(counted_down(5.0, SpeedMult::FULL), 4.0);
    }

    #[test]
    fn a_whole_wait_is_its_frames() {
        assert_eq!(ticks(0), 0.0);
        assert_eq!(ticks(180), 180.0);
    }
}
