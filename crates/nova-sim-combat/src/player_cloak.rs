//! The player's cloaking device: the cloak key and the upkeep in
//! `_HandlePlayer` (@0x705b7-0x70827), as plain functions over the
//! player's [`Cloak`], the key's debounce latch and its reserves. What
//! each returns is for the session to sound (see
//! `nova_sim::Session::hold_cloak`).
//!
//! # The key (@0x705b7-0x70673)
//!
//! `_QuickKeyCheck(cloakKey)` (keymap +0x52) is taken only while the
//! player is not breaking up (+0x3c ≤ 0, @0x705d6), not jumping (+0x50 ≤
//! 0, @0x705e0) and not disabled (`_IsDisabled`, @0x705e7); the session
//! decides that ([`press`]'s `taken`). A key not taken clears the latch
//! `_cloakFlag` (`*0xdedfc`, @0x7066b-0x70670). A taken key does nothing
//! while the latch is set (@0x705ff), and otherwise sets it (@0x70601).
//! Then:
//!
//! - with `_ShipCanCloak`, a fade above 0 or a direction above 0
//!   (@0x70610-0x70625) calls `_DoShipUncloak` (@0x7062d), which acts only
//!   on a cloak that reads cloaked, and anything else `_DoShipCloak`
//!   (@0x7063a, [`cloak_ship`], so a zeroing cloak zeroes the shield);
//! - without it, `_beepSnd[3]` (`snd ` 153) plays at priority 1
//!   (@0x70641-0x70664): [`Pressed::Refused`].
//!
//! So a press while the cloak still fades in (not yet cloaked), drifts or
//! fades out below 8 does nothing, and the player can't cloak again until
//! the fade is back at 0.
//!
//! # The upkeep (@0x70673-0x70827)
//!
//! Only while the cloak reads cloaked (`_ShipIsCloaked` @0x70679), as the
//! key left it this frame ([`upkeep`]):
//!
//! 1. a cloak that can no longer cloak, or a disabled player (-0x2cb),
//!    uncloaks (`_DoShipUncloak` @0x7068c-0x706a4);
//! 2. a fuel usage above 0 costs usage × [`CLOAK_UPKEEP_RATE`] fuel,
//!    floored at 0 (@0x706af-0x70731);
//! 3. a zeroing cloak stores a shield of 0 every frame (@0x7073c-0x7074d);
//!    any other drains its shield usage while the shield holds it
//!    ([`drain_shield`], @0x70792-0x70822).
//!
//! The test was made before step 1, so the frame the cloak drops pays
//! too.
//!
//! ## Left out
//!
//! - `_keyPressedThisFrame` (`*0x3b5550`, @0x705ef-0x705f4), which a taken
//!   press sets: it has no engine counterpart, as the `self_destruct`
//!   module also leaves it out.
//! - The status-redraw flags set every 10 frames (@0x706f6-0x70721,
//!   @0x70748-0x70783, @0x707e7-0x70812): the HUD is drawn every frame.
//! - The frame-time factor (`*0x3b511c`, @0x706ce-0x706e0,
//!   @0x707c0-0x707d1): the engine's tick models frame time.

use crate::cloak::{CLOAK_UPKEEP_RATE, Cloak, CloakGear, cloak_ship, drain_shield};

/// What a press of the cloak key did, for the session to sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pressed {
    /// `_DoShipCloak`: `sounds` when its sound plays (`snd ` 381).
    Cloaked {
        /// [`Cloak::cloak`]'s sound condition.
        sounds: bool,
    },
    /// `_DoShipUncloak`: `sounds` when its sound plays (`snd ` 380).
    Uncloaked {
        /// [`Cloak::uncloak`]'s sound condition.
        sounds: bool,
    },
    /// The player can't cloak: `_beepSnd[3]` (`snd ` 153) beeps.
    Refused,
}

/// One frame of the cloak key on the player's `cloak` of `gear`, `taken`
/// or not (see the module docs), debounced by `latch`; whether the player
/// `can_cloak` is `_ShipCanCloak` as the session asks it. Returns what a
/// press did, or `None` when nothing was pressed or the latch held it.
pub fn press(
    cloak: &mut Cloak,
    latch: &mut bool,
    taken: bool,
    can_cloak: bool,
    gear: CloakGear,
    shield: &mut f32,
) -> Option<Pressed> {
    if !taken {
        *latch = false;
        return None;
    }
    if *latch {
        return None;
    }
    *latch = true;
    if !can_cloak {
        return Some(Pressed::Refused);
    }
    if cloak.fade > 0.0 || cloak.direction > 0 {
        Some(Pressed::Uncloaked {
            sounds: cloak.uncloak(),
        })
    } else {
        Some(Pressed::Cloaked {
            sounds: cloak_ship(cloak, gear, shield),
        })
    }
}

/// One frame of the player's cloak upkeep on `cloak` of `gear` (see the
/// module docs), the player able to cloak (`can_cloak`) or not and
/// `disabled` or not, paid from `fuel` and `shield`. Runs only while the
/// cloak reads cloaked; returns `Some` with [`Cloak::uncloak`]'s sound
/// condition when it uncloaked.
pub fn upkeep(
    cloak: &mut Cloak,
    gear: CloakGear,
    can_cloak: bool,
    disabled: bool,
    fuel: &mut f32,
    shield: &mut f32,
) -> Option<bool> {
    if !cloak.is_cloaked() {
        return None;
    }
    let uncloaked = (!can_cloak || disabled).then(|| cloak.uncloak());
    if gear.fuel_usage > 0 {
        *fuel = (*fuel - f32::from(gear.fuel_usage) * CLOAK_UPKEEP_RATE).max(0.0);
    }
    if gear.zeroes_shields {
        *shield = 0.0;
    } else {
        drain_shield(shield, gear.shield_usage, true);
    }
    uncloaked
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::cloak::FADE_FULL;

    fn at(fade: f32, direction: i8) -> Cloak {
        Cloak { fade, direction }
    }

    const DEVICE: CloakGear = CloakGear {
        device: true,
        decloaks_when_hit: false,
        on_radar: false,
        flags2: 0,
        shield_usage: 0,
        fuel_usage: 0,
        zeroes_shields: false,
        can_cloak_mod: Some(0),
        scanner: crate::cloak::Scanner::NONE,
    };

    const ZEROING: CloakGear = CloakGear {
        zeroes_shields: true,
        ..DEVICE
    };

    const FULL: Cloak = Cloak {
        fade: FADE_FULL,
        direction: 0,
    };

    /// A fresh [`FULL`] cloak to spend.
    fn full() -> Cloak {
        FULL
    }

    /// One taken press with a clear latch on `cloak`, able to cloak.
    fn pressed(cloak: &mut Cloak) -> Option<Pressed> {
        press(cloak, &mut false, true, true, DEVICE, &mut 50.0)
    }

    #[test]
    fn a_taken_press_cloaks_an_uncloaked_player_with_its_sound_and_zeroes_a_zeroing_cloaks_shield()
    {
        let mut cloak = Cloak::default();
        let mut latch = false;
        let mut shield = 50.0;
        assert_eq!(
            press(&mut cloak, &mut latch, true, true, ZEROING, &mut shield),
            Some(Pressed::Cloaked { sounds: true })
        );
        assert_eq!((cloak, latch, shield), (at(0.0, 1), true, 0.0));
        let mut shield = 50.0;
        let mut cloak = Cloak::default();
        press(&mut cloak, &mut false, true, true, DEVICE, &mut shield);
        assert_eq!((cloak, shield), (at(0.0, 1), 50.0), "no zeroing bit");
    }

    #[test]
    fn a_press_uncloaks_a_cloaked_fade_and_does_nothing_until_it_reads_cloaked() {
        let mut rising = at(4.0, 1);
        assert_eq!(
            pressed(&mut rising),
            Some(Pressed::Uncloaked { sounds: false })
        );
        assert_eq!(rising, at(4.0, 1), "not yet cloaked: nothing");
        let mut cloaked = at(20.0, 1);
        assert_eq!(
            pressed(&mut cloaked),
            Some(Pressed::Uncloaked { sounds: true })
        );
        assert_eq!(cloaked, at(20.0, -1));
        let mut drifting = at(5.0, 0);
        assert_eq!(
            pressed(&mut drifting),
            Some(Pressed::Uncloaked { sounds: false })
        );
        assert_eq!(drifting, at(5.0, 0), "a drifting fade can't cloak again");
        let mut full = FULL;
        assert_eq!(
            pressed(&mut full),
            Some(Pressed::Uncloaked { sounds: true })
        );
        assert_eq!(full, at(FADE_FULL, -1));
        let mut starting = at(0.0, 1);
        assert_eq!(
            pressed(&mut starting),
            Some(Pressed::Uncloaked { sounds: false })
        );
        assert_eq!(starting, at(0.0, 1), "a direction above 0 at fade 0");
    }

    #[test]
    fn a_held_key_toggles_once_until_it_is_let_go_or_a_gate_fails() {
        let mut cloak = Cloak::default();
        let mut latch = false;
        let mut shield = 50.0;
        let mut frame = |cloak: &mut Cloak, latch: &mut bool, taken| {
            press(cloak, latch, taken, true, DEVICE, &mut shield)
        };
        assert!(frame(&mut cloak, &mut latch, true).is_some());
        let mut held = FULL;
        assert_eq!(frame(&mut held, &mut latch, true), None, "held");
        assert_eq!(held, FULL);
        assert_eq!(frame(&mut held, &mut latch, false), None, "let go");
        assert!(!latch);
        assert_eq!(
            frame(&mut held, &mut latch, true),
            Some(Pressed::Uncloaked { sounds: true }),
            "pressed again"
        );
        // A gate failing (`taken` false) clears the latch as letting go does.
        assert_eq!(frame(&mut held, &mut latch, false), None);
        assert!(!latch);
    }

    #[test]
    fn a_press_that_cannot_cloak_is_refused_once_a_press_and_changes_nothing() {
        let mut cloak = FULL;
        let mut latch = false;
        let mut shield = 50.0;
        assert_eq!(
            press(&mut cloak, &mut latch, true, false, ZEROING, &mut shield),
            Some(Pressed::Refused)
        );
        assert_eq!((cloak, latch, shield), (FULL, true, 50.0));
        assert_eq!(
            press(&mut cloak, &mut latch, true, false, ZEROING, &mut shield),
            None,
            "held: refused once"
        );
        let mut uncloaked = Cloak::default();
        assert_eq!(
            press(
                &mut uncloaked,
                &mut false,
                true,
                false,
                ZEROING,
                &mut shield
            ),
            Some(Pressed::Refused)
        );
        assert_eq!((uncloaked, shield), (Cloak::default(), 50.0));
    }

    #[test]
    fn the_upkeep_runs_only_while_cloaked() {
        let gear = CloakGear {
            fuel_usage: 3,
            shield_usage: 2,
            ..DEVICE
        };
        for start in [Cloak::default(), at(16.0, 1), at(8.0, -1), at(20.0, 0)] {
            let mut cloak = start;
            let (mut fuel, mut shield) = (100.0, 100.0);
            let cloaked = start.is_cloaked();
            assert_eq!(
                upkeep(&mut cloak, gear, false, true, &mut fuel, &mut shield),
                cloaked.then_some(true),
                "{start:?}"
            );
            assert_eq!(fuel < 100.0, cloaked, "{start:?}");
            assert_eq!(shield < 100.0, cloaked, "{start:?}");
        }
    }

    #[test]
    fn the_upkeep_drains_fuel_by_its_usage_and_floors_it_at_0() {
        let gear = CloakGear {
            fuel_usage: 3,
            ..DEVICE
        };
        let (mut fuel, mut shield) = (100.0, 100.0);
        assert_eq!(
            upkeep(&mut full(), gear, true, false, &mut fuel, &mut shield),
            None
        );
        assert_eq!(fuel, 100.0 - 3.0 * CLOAK_UPKEEP_RATE);
        assert_eq!(shield, 100.0, "no shield usage");
        let mut low = 0.05;
        upkeep(&mut full(), gear, true, false, &mut low, &mut shield);
        assert_eq!(low, 0.0, "floored");
        let mut overdrawn = -1.0;
        upkeep(
            &mut full(),
            DEVICE,
            true,
            false,
            &mut overdrawn,
            &mut shield,
        );
        assert_eq!(overdrawn, -1.0, "no usage: the fuel is left alone");
    }

    #[test]
    fn the_upkeep_zeroes_a_zeroing_cloaks_shield_every_frame_and_otherwise_drains_it_while_it_holds_its_usage()
     {
        let mut fuel = 100.0;
        for start in [30.0, -2.0, 0.0] {
            let mut shield = start;
            upkeep(&mut full(), ZEROING, true, false, &mut fuel, &mut shield);
            assert_eq!(shield, 0.0, "{start}");
        }
        let draining = CloakGear {
            shield_usage: 8,
            ..DEVICE
        };
        let mut shield = 30.0;
        upkeep(&mut full(), draining, true, false, &mut fuel, &mut shield);
        assert_eq!(shield, 30.0 - 8.0 * CLOAK_UPKEEP_RATE);
        let mut shield = 7.0;
        upkeep(&mut full(), draining, true, false, &mut fuel, &mut shield);
        assert_eq!(shield, 7.0, "below its usage");
    }

    #[test]
    fn the_upkeep_uncloaks_a_cloak_that_can_no_longer_cloak_or_a_disabled_ship_and_still_takes_that_frames_upkeep()
     {
        let gear = CloakGear {
            fuel_usage: 3,
            ..ZEROING
        };
        for (can_cloak, disabled) in [(false, false), (true, true), (false, true)] {
            let mut cloak = FULL;
            let (mut fuel, mut shield) = (10.0, 30.0);
            assert_eq!(
                upkeep(
                    &mut cloak,
                    gear,
                    can_cloak,
                    disabled,
                    &mut fuel,
                    &mut shield
                ),
                Some(true),
                "{can_cloak} {disabled}"
            );
            assert_eq!(cloak, at(FADE_FULL, -1));
            assert_eq!(fuel, 10.0 - 3.0 * CLOAK_UPKEEP_RATE, "paid that frame");
            assert_eq!(shield, 0.0, "zeroed that frame");
        }
        let mut leaving = at(20.0, -1);
        assert_eq!(
            upkeep(&mut leaving, gear, false, false, &mut 10.0, &mut 30.0),
            Some(false),
            "already fading out: no sound"
        );
        let mut kept = FULL;
        assert_eq!(
            upkeep(&mut kept, gear, true, false, &mut 10.0, &mut 30.0),
            None
        );
        assert_eq!(kept, FULL);
    }
}
