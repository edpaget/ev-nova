//! A shot's fade, the original's +0x24, as `_HandleShot` steps it each
//! live visit at the frame's gain (@0x359ac-0x35c07).
//!
//! F is the weapon's `Falloff` as loaded
//! ([`WeaponSpec::falloff`](super::weapon::WeaponSpec::falloff), the
//! shot fade's +0xb8: no beam flies as a shot, so it is the view's shot
//! fade `Falloff` too) and f the shot's fade, 0 at its spawn (@0x3e81c).
//! Each visit reads f to shade the shot before it steps it, and the view
//! shades it by what that visit read
//! ([`Shot::fade_shown`](super::projectile::Shot::fade_shown)) and the
//! life after the visit's count-down:
//!
//! - **Fade-in, F < 0**, while f < 32.0 (0xdd654): f =
//!   (float)((double)f − (double)F × g) (@0x35a5b-0x35aa4).
//! - **Fade-out, F > 0**, while the life is below 32.0 (@0x35ac6-0x35ad2)
//!   and f below 32.0: f = (float)((double)f + F × g) (@0x35b60-0x35b9f).
//! - **No `Falloff`**: f never steps.
//!
//! The original doesn't step a translucent shot's fade-in at 16-bit (the
//! jump @0x35a59 skips @0x35a72-0x35aa4). The view draws such a shot at
//! level 0 under the engine's reading of
//! [`RuleKey::TranslucentShotFade`](crate::RuleKey::TranslucentShotFade)
//! whatever f is, and the other reading steps it as a plain shot's fade,
//! so f is stepped for every shot here, which shows the same under both.
//!
//! None of this plays a sound.

use std::cmp::Ordering;

use crate::speed_mult::SpeedMult;

/// The fade a shot's fade steps to and its window's life, 32.0 (0xdd654).
pub const FADE_FULL: f32 = 32.0;

/// A shot's fade `fade` after a visit at the frame's `gain`, of a weapon
/// of `Falloff` `falloff`, its life after the count-down `life` (see the
/// module docs).
#[must_use]
pub fn step(fade: f32, falloff: i16, life: f32, gain: SpeedMult) -> f32 {
    let by = f32::from(falloff);
    match falloff.cmp(&0) {
        Ordering::Less if fade < FADE_FULL => gain.add_times(fade, -by),
        Ordering::Greater if life < FADE_FULL && fade < FADE_FULL => gain.add_times(fade, by),
        _ => fade,
    }
}

/// The life and the fade its last visit read of a shot of `Count` `count`
/// and `Falloff` `falloff`, `frames` full frames old: the life counted
/// down a frame a visit, spent ([`SPENT`](super::projectile::SPENT)) once
/// not above 0, and [`step`] folded at full gain. A builder, the state a
/// test or a crate downstream puts a shot's fade in; a shot of no visits
/// has its full life and a fade of 0.
#[must_use]
pub fn at_full_gain(count: u32, falloff: i16, frames: u32) -> (f32, f32) {
    let mut life = super::projectile::whole_frames(count);
    let (mut fade, mut shown) = (0.0, 0.0);
    for _ in 0..frames {
        life = SpeedMult::FULL.add_times(life, -1.0);
        if !super::projectile::above(life, 0.0) {
            life = super::projectile::SPENT;
        }
        shown = fade;
        fade = step(fade, falloff, life, SpeedMult::FULL);
    }
    (life, shown)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const HALF: SpeedMult = SpeedMult::new(0.5);

    /// The fades read on each of `visits` visits at `gain` of a shot of
    /// `Count` `count` and `Falloff` `falloff`.
    fn read(count: u32, falloff: i16, visits: u32, gain: SpeedMult) -> Vec<f32> {
        let mut life = super::super::projectile::whole_frames(count);
        let mut fade = 0.0;
        (0..visits)
            .map(|_| {
                life = gain.add_times(life, -1.0);
                let shown = fade;
                fade = step(fade, falloff, life, gain);
                shown
            })
            .collect()
    }

    #[test]
    fn at_half_gain_a_fade_in_rises_half_its_falloff_a_visit_and_stops_at_32() {
        let fades = read(100, -10, 9, HALF);
        assert_eq!(fades, [0.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 35.0]);
        assert_eq!(step(31.0, -3, 100.0, HALF), 32.5, "below 32: steps");
        assert_eq!(step(32.0, -3, 100.0, HALF), 32.0, "at 32: done");
    }

    #[test]
    fn at_half_gain_a_fade_out_waits_for_a_life_below_32_then_rises_half_its_falloff() {
        assert_eq!(step(0.0, 4, 32.0, HALF), 0.0, "life 32: not yet");
        assert_eq!(step(0.0, 4, 31.9, HALF), 2.0, "31.9: in the window");
        assert_eq!(step(31.0, 4, 10.0, HALF), 33.0);
        assert_eq!(step(33.0, 4, 10.0, HALF), 33.0, "stops at 32 or more");
        assert_eq!(step(32.0, 4, 10.0, HALF), 32.0, "at 32: done");
        // Count 33: the life drops below 32 on its 3rd visit (31.5).
        assert_eq!(read(33, 4, 5, HALF), [0.0, 0.0, 0.0, 2.0, 4.0]);
    }

    #[test]
    fn at_no_gain_nothing_moves() {
        assert_eq!(step(3.0, -5, 100.0, SpeedMult::ZERO), 3.0);
        assert_eq!(step(3.0, 5, 10.0, SpeedMult::ZERO), 3.0);
        assert_eq!(step(3.0, 0, 10.0, HALF), 3.0, "nor with no Falloff");
    }

    #[test]
    fn at_full_gain_the_fades_step_a_whole_falloff_a_visit() {
        assert_eq!(
            read(100, -5, 9, SpeedMult::FULL)[..8],
            [0.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0]
        );
        // Count 40, F 2: the window opens on the 9th visit, life 31.
        let out = read(40, 2, 12, SpeedMult::FULL);
        assert_eq!(out[8..], [0.0, 2.0, 4.0, 6.0]);
        assert_eq!(out[..8], [0.0; 8]);
    }

    #[test]
    fn the_full_gain_builder_agrees_with_folding_step() {
        for (count, falloff) in [(100, -5), (40, 2), (5, 4), (32, 13), (110, 2)] {
            for frames in 1..=count + 1 {
                let fades = read(count, falloff, frames, SpeedMult::FULL);
                let (life, shown) = at_full_gain(count, falloff, frames);
                assert_eq!(
                    shown,
                    *fades.last().expect("a visit"),
                    "{count} {falloff} {frames}"
                );
                let expected = if frames >= count {
                    super::super::projectile::SPENT
                } else {
                    super::super::projectile::whole_frames(count - frames)
                };
                assert_eq!(life, expected);
            }
        }
        assert_eq!(at_full_gain(32, 4, 0), (32.0, 0.0), "unvisited");
    }
}
