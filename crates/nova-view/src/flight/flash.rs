//! A ship's shield flash: the timer a hit sets and the level its shän
//! shield layer is drawn at, as the original keeps it (`_DamageShip`
//! @0x3bef9-0x3bf0c, `_HandleShipDisplay` @0x2c831-0x2c97d in the
//! `EV Nova` executable).
//!
//! - A hit that reaches `_DamageShip`'s flash write
//!   ([`CombatEvent::Hit`](nova_sim::CombatEvent::Hit)) sets the ship's
//!   timer (+0xc8f0) to [`FLASH_TICKS`], unless its weapon passes shields
//!   ([`ShieldFlash::hit`]).
//! - Each frame, while the timer is above none, the ship's shield above
//!   none and its class has a shield layer, the timer is first held down
//!   to `L = trunc(shield × 32 / base)`, where `base` is the class's own
//!   `Shield` as loaded (the class record's +0x5c @0x2c8aa, not the
//!   ship's capacity with its outfits, so a ship with shield outfits holds
//!   it at 32). `L` above 32 is taken as 32 and `L` of 3 or below as 4, so
//!   a weak shield flashes faintly and briefly and a recharging one never
//!   raises it again; a quotient out of an `i32`'s range, as a `base` of
//!   none gives, truncates to `i32::MIN` (`cvttss2si`) and so to 4. The
//!   layer is drawn at `max(trunc(timer), 1)`. Then the timer drops by a
//!   frame while it is above none, drawn or not ([`ShieldFlash::step`]):
//!   the original's `_gSpeedMult`, 1 at the fixed step the engine runs.
//!
//! An NPC's shot hits land before its `_HandleShipDisplay` in
//! `_DoPlayGameWork`, so they show on the frame they land; a hit on the
//! player, a beam's, and some shockwaves land after their ship's display
//! and show a frame later
//! ([`CombatEvent::Hit::late`](nova_sim::CombatEvent::Hit)).
//!
//! No ship's layer is drawn yet: the shän shield layer is
//! rdm:phase/ship-handling-fidelity/phase-10's, which only has to supply
//! whether a class has it and draw it at the level [`ShieldFlash::step`]
//! gives. Until then the flight view steps every flash with no layer, so
//! the timer runs down unseen.

/// The timer a hit sets: 32 frames (the float 32.0 @0x3bf02).
pub const FLASH_TICKS: f32 = 32.0;

/// The most level a flash shows, and what a shield's share is scaled to.
pub const FLASH_LEVELS: f32 = 32.0;

/// The least level a weak shield holds the timer to: a share of 3 or
/// below is taken as 4.
pub const FLASH_FLOOR: f32 = 4.0;

/// The level `shield` of a class's own `base` holds a flash to:
/// `trunc(shield × 32 / base)` as `cvttss2si` takes it, `i32::MIN` out of
/// an `i32`'s range, then at most 32 and, at 3 or below, 4.
fn share(shield: f32, base: f32) -> f32 {
    let quotient = shield * FLASH_LEVELS / base;
    // Within an `i32`'s range by the test, whole once truncated.
    #[allow(clippy::cast_possible_truncation)]
    let level = if quotient.is_finite() && quotient.abs() < 2_147_483_648.0 {
        quotient.trunc() as i32
    } else {
        i32::MIN
    };
    match level {
        33.. => FLASH_LEVELS,
        ..=3 => FLASH_FLOOR,
        // 4-32: exact in an `f32`.
        #[allow(clippy::cast_precision_loss)]
        level => level as f32,
    }
}

/// One ship's shield flash.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShieldFlash {
    /// The timer, +0xc8f0: what it has left, in frames.
    timer: f32,
    /// The timer as the last frame showed it: after its clamp, before
    /// its drop.
    shown: f32,
}

impl ShieldFlash {
    /// A hit: the timer set to [`FLASH_TICKS`], unless it
    /// `passes_shields`.
    pub fn hit(&mut self, passes_shields: bool) {
        if !passes_shields {
            self.timer = FLASH_TICKS;
        }
    }

    /// A frame, for a ship of `shield` whose class's own `Shield` is
    /// `base` and which has a shield layer when `has_layer`: the level its
    /// layer is drawn at, if it is drawn (see the module docs). The timer
    /// then drops by 1 while it is above none.
    pub fn step(&mut self, shield: f32, base: f32, has_layer: bool) -> Option<u8> {
        let level = if self.timer > 0.0 && shield > 0.0 && has_layer {
            self.timer = self.timer.min(share(shield, base));
            // At most 32 and at least 1: within a `u8`.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Some(self.timer.trunc().max(1.0) as u8)
        } else {
            None
        };
        self.shown = self.timer;
        if self.timer > 0.0 {
            self.timer -= 1.0;
        }
        level
    }

    /// The timer as the last frame showed it, after its clamp and before
    /// its drop: [`FLASH_TICKS`] on the frame of a hit.
    #[must_use]
    pub fn shown(&self) -> f32 {
        self.shown
    }

    /// Whether the timer has run out.
    #[must_use]
    pub fn spent(&self) -> bool {
        self.timer <= 0.0
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// The levels shown over `frames` frames of a flash just hit, at
    /// `shield` of 100.
    fn levels(shield: f32, frames: usize) -> Vec<Option<u8>> {
        let mut flash = ShieldFlash::default();
        flash.hit(false);
        (0..frames)
            .map(|_| flash.step(shield, 100.0, true))
            .collect()
    }

    #[test]
    fn a_full_shield_flashes_from_32_down_a_level_a_frame() {
        let seen = levels(100.0, 34);
        let expected: Vec<Option<u8>> = (1..=32).rev().map(Some).chain([None, None]).collect();
        assert_eq!(seen, expected);
    }

    #[test]
    fn a_shield_above_its_classs_own_is_held_to_32() {
        assert_eq!(levels(250.0, 1), [Some(32)]);
    }

    #[test]
    fn a_class_of_no_shield_of_its_own_holds_the_flash_to_4() {
        let mut flash = ShieldFlash::default();
        flash.hit(false);
        assert_eq!(flash.step(50.0, 0.0, true), Some(4), "inf: i32::MIN");
        let mut flash = ShieldFlash::default();
        flash.hit(false);
        assert_eq!(flash.step(50.0, -10.0, true), Some(4), "negative");
        assert_eq!(share(1.0e9, 1.0), 4.0, "out of an i32's range");
        assert_eq!(share(100.0, 100.0), 32.0);
        assert_eq!(share(33.0, 32.0), 32.0, "33 held to 32");
        assert_eq!(share(15.0, 120.0), 4.0, "exactly 4");
        assert_eq!(share(16.0, 120.0), 4.0);
        assert_eq!(share(19.0, 120.0), 5.0);
    }

    #[test]
    fn a_weak_shield_holds_the_flash_to_its_share_and_at_least_4() {
        // trunc(5 × 32 / 100) = 1, taken as 4.
        assert_eq!(
            levels(5.0, 6),
            [Some(4), Some(3), Some(2), Some(1), None, None]
        );
        // trunc(12 × 32 / 100) = 3, taken as 4; trunc(13 × 32 / 100) = 4.
        assert_eq!(levels(12.0, 1), [Some(4)]);
        assert_eq!(levels(13.0, 1), [Some(4)]);
        // trunc(50 × 32 / 100) = 16.
        assert_eq!(levels(50.0, 2), [Some(16), Some(15)]);
    }

    #[test]
    fn the_last_frames_fraction_shows_at_least_level_1() {
        let mut flash = ShieldFlash {
            timer: 0.5,
            shown: 0.0,
        };
        assert_eq!(flash.step(100.0, 100.0, true), Some(1));
        assert!(flash.spent());
    }

    #[test]
    fn a_shield_of_none_shows_nothing_but_the_timer_still_runs_down() {
        let mut flash = ShieldFlash::default();
        flash.hit(false);
        assert_eq!(flash.step(0.0, 100.0, true), None);
        assert_eq!(flash.shown(), 32.0, "not held down");
        assert_eq!(flash.step(100.0, 100.0, true), Some(31));
    }

    #[test]
    fn a_recharging_shield_never_raises_the_flash_again() {
        let mut flash = ShieldFlash::default();
        flash.hit(false);
        assert_eq!(flash.step(25.0, 100.0, true), Some(8));
        assert_eq!(flash.step(100.0, 100.0, true), Some(7));
    }

    #[test]
    fn without_a_layer_nothing_is_shown_and_the_timer_still_runs_down() {
        let mut flash = ShieldFlash::default();
        flash.hit(false);
        assert_eq!(flash.step(5.0, 100.0, false), None);
        assert_eq!(flash.shown(), 32.0, "no clamp without a layer");
        assert_eq!(flash.step(5.0, 100.0, false), None);
        assert_eq!(flash.shown(), 31.0);
        for _ in 0..31 {
            flash.step(5.0, 100.0, false);
        }
        assert!(flash.spent());
        assert_eq!(flash.step(5.0, 100.0, false), None);
        assert_eq!(flash.shown(), 0.0, "it stops at none");
    }

    #[test]
    fn a_hit_that_passes_shields_leaves_the_timer_as_it_was() {
        let mut flash = ShieldFlash::default();
        flash.hit(true);
        assert_eq!(flash.step(100.0, 100.0, true), None);
        assert!(flash.spent());
        flash.hit(false);
        flash.step(100.0, 100.0, true);
        flash.step(100.0, 100.0, true);
        flash.hit(true);
        assert_eq!(flash.step(100.0, 100.0, true), Some(30), "not set again");
    }

    #[test]
    fn a_new_hit_sets_the_timer_back_to_32() {
        let mut flash = ShieldFlash::default();
        flash.hit(false);
        flash.step(100.0, 100.0, true);
        flash.hit(false);
        assert_eq!(flash.step(100.0, 100.0, true), Some(32));
        assert_eq!((FLASH_TICKS, FLASH_LEVELS, FLASH_FLOOR), (32.0, 32.0, 4.0));
    }
}
