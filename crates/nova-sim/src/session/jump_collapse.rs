//! A disabled player's pre-jump collapses (`_HandlePlayer`
//! 0x688aa-0x68ab4).
//!
//! **When.** The original checks, early in each frame of `_HandlePlayer`,
//! whether the player is disabled (`_IsDisabled` @0x68897, kept in
//! -0x2cb) and its jump state +0x50 is above 0 (@0x688aa-0x688b5).
//! `_IsDisabled` is an armour test (@0x2d95-0x2dcc), so a dying player,
//! its armour gone, collapses too. The engine runs the collapse
//! ([`Session::collapse_jump`]) in [`Session::tick`]'s arm for a disabled
//! or dying player, and only for a [`Jump::PreJump`]: the engine never
//! ticks the world through [`Jump::Hyperspace`] (see Left out).
//!
//! **The speed.** el = `TickCount` − +0xa4 (@0x688bb-0x688c0), the Mac
//! ticks of the warp clock. While the ship brakes the original restamps
//! +0xa4 every frame (@0x6b6a2-0x6b6b1), so a player disabled then
//! measures one frame; from the warp start it stamps it once (@0x6b5f5),
//! the engine's `warp` clock ([`collapse_ticks`]). s is the warp's
//! lead-in speed ([`warp::lead_in_speed`], @0x688f2-0x6895e) with:
//!
//! - **C**, the class's +0x44, the warp factor `_LoadObjectData` sets
//!   from the `shïp` `Flags` (+0x9e8) at 0x7ae9d-0x7aef1: 0x1 gives 0.7,
//!   else 0x2 1.3, else 0x4 1.6, else 1.0, each × 1.3 (f64 @0xdd5e8) and
//!   stored as an f32: 0.91, 1.69, 2.08 or 1.3 ([`warp::warp_factor`]).
//!   A negative factor becomes 0.5 (@0x7aefe-0x7af07), which no `Flags`
//!   reaches, so data never makes C 0: the C = 0 branch (the −35/C term
//!   skipped, s 0) is reached only through [`warp::lead_in_speed`] itself.
//!   C is read from the player's class when the collapse runs.
//! - **W**, `_WarpSoundDuration` (@0x8203-0x8224), entry [0] of
//!   `_jumpSoundLength` (0x1b8be6, through the pointer slot 0x3b5070):
//!   `_LoadSounds` (@0x1c0b5-0x1c12d) stores 350 (0x15e @0x1c0d2) and
//!   then, when `snd ` 128 exists, its frames × 60 / the whole part of
//!   its rate (`_ParseSndHeader`, `imull $0x3c`, `divl`), in Mac ticks;
//!   stock 128 gives 364 ([`warp::sound_duration`]). In double time it
//!   is entry [1], from `snd ` 129 (@0x8210-0x8219; stock 252). The
//!   session's `warp_sound` ([`warp::warp_sound_duration`]). The term
//!   stays −35/C in double time: only the move-off's is −52.5/C
//!   ([`warp::move_off_speed`]).
//!
//! When s > 0 (@0x68976) the velocity is zeroed (@0x689b6, 0x689bf) and
//! `_Accel` gives it min(s, `_ShipMaxSpeed`) (@0x6897e/0x6899d) along the
//! heading cut to whole degrees (`cvttss2si` @0x689d3); otherwise it is
//! left (`jbe` 0x689e1) ([`collapsed_velocity`]). The disabled drag
//! (0.995 @0x68ace) runs after it in the same frame, as the engine's
//! `None` arm of [`Session::tick`] drags and steps once the jump is gone.
//!
//! **The rest.** +0x50 = −1.0 (@0x689ea) ends the jump. The comm line
//! says [`SimMessage::JumpCollapsed`], `STR#` 2002 #35 (`_GetIndString`
//! @0x68a04), for 250 frames (`_DisplayComm` @0x68a3d). The warp-up
//! sounds 128 and 129 are halted if playing (@0x68a5c, @0x68a7f;
//! [`WARP_UP_HALTS`]), nothing else, and `_beepSnd[1]`, `snd ` 151, plays
//! ([`SimSound::JumpCollapsed`], `_PlaySound` @0x68aaf). `begin_jump`
//! refuses a disabled player, so the jump does not restart.
//!
//! **Ordering against the original.** `_PlayGame` calls `_HandlePlayer`
//! (@0x43e19) before `_HandleShot` (@0x44385) and `_HandleShip`
//! (@0x44483), so a hit that disables the player collapses the jump in
//! the next frame's `_HandlePlayer`. The engine does the same: the fight
//! in [`Session::tick_combat`] disables, and the next [`Session::tick`]
//! collapses. A sabotage bomb that disables (in `tick_combat`) does
//! likewise, a frame after the original's (@0x69a05, after 0x688aa in
//! the same `_HandlePlayer`): a one-tick lag that crosses no boundary the
//! player sees. The eject (@0x6f22c) comes after the collapse in
//! `_HandlePlayer`, so [`Session::eject`] collapses a pre-jump first,
//! before the hulk takes the player's state and before the class
//! changes: `eject_on_breakup` runs a later frame's eject early, and can
//! meet a player still in its pre-jump.
//!
//! **Left out.**
//!
//! - The collapse during [`Jump::Hyperspace`]. While the streak runs,
//!   the flight view advances only the jump effect and never calls
//!   [`Session::tick`], `tick_traffic` or `tick_combat`, so nothing can
//!   disable the player there and no warp speed builds up. It needs the
//!   world to run through the player's warp, the player warp-out motion
//!   of rdm:phase/ship-handling-fidelity/phase-8-warp-out, and reuses
//!   this module unchanged.
//! - The `_warpSoundDone` clears (@0x68a61, 0x68a84). The flag gates
//!   only the warp stage's warp-out sound (read @0x6bbc5 and 0x6c6dc),
//!   and the next warp start clears it again (@0x6b695), so nothing reads
//!   it after the collapse.

use super::{Jump, Session};
use crate::cargo_box::accel;
use crate::geometry::Vec2;
use crate::message::SimMessage;
use crate::sound::{SimSound, WARP_UP_HALTS};
use crate::warp;

/// The velocity a collapse leaves a ship moving at `velocity`, facing
/// `heading`, given the lead-in `speed` and its `top` speed: min(speed,
/// top) along the heading cut to whole degrees when the speed is above
/// 0, else `velocity` as it is (a NaN speed included).
pub(crate) fn collapsed_velocity(velocity: Vec2, heading: f32, speed: f32, top: f32) -> Vec2 {
    if speed > 0.0 {
        #[allow(clippy::cast_possible_truncation)]
        let heading = heading.trunc() as i32;
        accel(heading, speed.min(top))
    } else {
        velocity
    }
}

/// The Mac ticks of the warp clock as the collapse reads them, from the
/// pre-jump's `warp` at the start of the pass. On the advance pass: one
/// tick while braking (the stamp is the previous frame's), and k ticks on
/// the k-th tick after the warp start, whose own advance comes later in
/// the tick. On double time's second pass (`again`), the frame's advance
/// pass has already moved the clock on: k ticks for a `warp` of k, and
/// none while braking, the stamp the same frame's.
fn collapse_ticks(warp: Option<u32>, again: bool) -> u32 {
    let ticks = if again {
        warp.unwrap_or(0)
    } else {
        warp.map_or(1, |ticks| ticks.saturating_add(1))
    };
    warp::mac_ticks(ticks)
}

impl Session {
    /// Collapses a pre-jump, as a disabled or dying player's does (see
    /// the module docs), on double time's second pass when `again`:
    /// nothing without one.
    pub(super) fn collapse_jump(&mut self, again: bool) {
        let Some(Jump::PreJump { warp, .. }) = self.jump else {
            return;
        };
        let flags = self
            .ship_record(self.pilot.ship)
            .map_or(0, |record| record.flags);
        let speed = warp::lead_in_speed(
            collapse_ticks(warp, again),
            warp::warp_factor(flags),
            self.warp_sound(),
        );
        self.player.velocity = collapsed_velocity(
            self.player.velocity,
            self.player.heading,
            speed,
            self.top_speed(),
        );
        self.jump = None;
        self.messages.push(SimMessage::JumpCollapsed);
        self.sounds.push(SimSound::Halt(&WARP_UP_HALTS));
        self.sounds.push(SimSound::JumpCollapsed);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::SystemId;
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::flight::{Controls, DISABLED_DRAG};
    use crate::session::JumpRefusal;
    use crate::session::tests::{DISABLING, OUT, bound_for_131};
    use crate::sound::PLAYER_EXPLOSION_HALTS;
    use crate::testkit::{catalog, fly_out};

    // The pure parts.

    #[test]
    fn a_collapse_sets_the_velocity_along_the_whole_degree_heading() {
        let old = Vec2::new(3.0, -4.0);
        assert_eq!(
            collapsed_velocity(old, 45.7, 2.0, 10.0),
            accel(45, 2.0),
            "the old velocity discarded"
        );
        assert_ne!(accel(45, 2.0), accel(46, 2.0));
        assert_eq!(collapsed_velocity(old, 90.0, 2.0, 10.0), accel(90, 2.0));
        assert!((accel(90, 2.0) - Vec2::new(2.0, 0.0)).length() < 1e-6);
    }

    #[test]
    fn a_collapse_is_capped_at_the_top_speed() {
        let old = Vec2::new(1.0, 1.0);
        assert_eq!(collapsed_velocity(old, 0.0, 12.0, 9.0), accel(0, 9.0));
        assert_eq!(collapsed_velocity(old, 0.0, 9.0, 9.0), accel(0, 9.0));
        assert_eq!(collapsed_velocity(old, 0.0, 8.0, 9.0), accel(0, 8.0));
    }

    #[test]
    fn no_speed_a_negative_one_or_nan_leaves_the_velocity() {
        let old = Vec2::new(3.0, -4.0);
        for speed in [0.0, -0.5, -30.0, f32::NAN] {
            assert_eq!(collapsed_velocity(old, 45.0, speed, 9.0), old, "{speed}");
        }
        assert_eq!(collapsed_velocity(old, 45.0, 0.001, 9.0), accel(45, 0.001));
    }

    #[test]
    fn the_clock_is_one_tick_while_braking_and_counts_from_the_warp_start() {
        assert_eq!(collapse_ticks(None, false), 2, "the previous frame's stamp");
        assert_eq!(collapse_ticks(Some(0), false), 2);
        assert_eq!(collapse_ticks(Some(4), false), 10);
        assert_eq!(collapse_ticks(Some(u32::MAX), false), u32::MAX, "saturates");
    }

    #[test]
    fn on_the_second_pass_the_clock_is_the_frames_already() {
        assert_eq!(collapse_ticks(None, true), 0, "this frame's stamp");
        assert_eq!(collapse_ticks(Some(0), true), 0);
        assert_eq!(collapse_ticks(Some(4), true), 8);
        assert_eq!(collapse_ticks(Some(u32::MAX), true), u32::MAX, "saturates");
    }

    // Through the session.

    /// A session flown out and braking for its jump to 131, with a warp
    /// sound of `duration` Mac ticks.
    fn braking(duration: u32) -> Session {
        let mut session = bound_for_131(&catalog(), Vec2::ZERO, Vec2::ZERO, 0.0);
        fly_out(&mut session);
        session.begin_jump().expect("jumps");
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.warp_clock(), None, "braking");
        session.warp_sounds[0] = duration;
        session.take_sounds();
        session.take_messages();
        session
    }

    /// Disables the player as a fight does, then ticks.
    fn disable_and_tick(session: &mut Session) {
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(session.condition, Condition::Disabled);
        session.tick(Controls::default());
    }

    /// The session's sounds, less the thrust's stopping, which the braking
    /// ship's thrust going out raises as the drift takes over.
    fn sounds(session: &mut Session) -> Vec<SimSound> {
        let mut sounds = session.take_sounds();
        sounds.retain(|sound| *sound != SimSound::ThrustStopped);
        sounds
    }

    /// The warp factor of the session's player, its class with no record.
    fn factor() -> f32 {
        warp::warp_factor(0)
    }

    #[test]
    fn a_player_disabled_while_braking_collapses_on_one_ticks_clock() {
        let mut session = braking(9);
        let before = *session.player();
        disable_and_tick(&mut session);
        assert_eq!(session.preparing_jump(), None);
        assert_eq!(session.jumping(), None);
        let speed = warp::lead_in_speed(2, factor(), 9);
        assert!(speed > 0.0 && speed < session.top_speed(), "{speed}");
        let expected =
            collapsed_velocity(before.velocity, before.heading, speed, session.top_speed())
                * DISABLED_DRAG;
        assert_eq!(session.player().velocity, expected);
        assert_ne!(expected, before.velocity * DISABLED_DRAG);
    }

    #[test]
    fn a_player_disabled_while_turning_collapses_on_the_clock_from_the_turns_start() {
        // At rest facing up, 90 degrees off east: J turns at once.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 0.0);
        session.warp_sounds[0] = 25;
        session.begin_jump().expect("jumps");
        assert_eq!(
            session.take_sounds(),
            [SimSound::JumpBegan],
            "the warp-up the halt halts"
        );
        for _ in 0..2 {
            session.tick(Controls::default());
        }
        assert_eq!(session.warp_clock(), Some(2));
        assert_eq!(session.player().heading, 6.0);
        disable_and_tick(&mut session);
        assert_eq!(session.preparing_jump(), None);
        let speed = warp::lead_in_speed(warp::mac_ticks(3), factor(), 25);
        assert!(speed > 0.0 && speed < session.top_speed(), "{speed}");
        assert_ne!(speed, warp::lead_in_speed(2, factor(), 25));
        assert_eq!(session.player().velocity, accel(6, speed) * DISABLED_DRAG);
    }

    #[test]
    fn a_collapse_beyond_the_top_speed_is_capped() {
        let mut session = braking(1);
        let heading = session.player().heading;
        let top = session.top_speed();
        assert!(warp::lead_in_speed(2, factor(), 1) > top);
        disable_and_tick(&mut session);
        #[allow(clippy::cast_possible_truncation)]
        let expected = accel(heading.trunc() as i32, top) * DISABLED_DRAG;
        assert_eq!(session.player().velocity, expected);
    }

    #[test]
    fn a_collapse_with_no_speed_leaves_the_velocity() {
        let mut session = braking(364);
        assert!(warp::lead_in_speed(2, factor(), 364) < 0.0);
        let before = session.player().velocity;
        disable_and_tick(&mut session);
        assert_eq!(session.preparing_jump(), None, "still dropped");
        assert_eq!(session.player().velocity, before * DISABLED_DRAG);
        assert_eq!(session.take_messages(), [SimMessage::JumpCollapsed]);
        assert_eq!(
            sounds(&mut session),
            [SimSound::Halt(&WARP_UP_HALTS), SimSound::JumpCollapsed]
        );
    }

    #[test]
    fn a_collapse_posts_the_message_halts_128_and_129_then_beeps() {
        let mut session = braking(9);
        disable_and_tick(&mut session);
        assert_eq!(session.take_messages(), [SimMessage::JumpCollapsed]);
        let sounds = sounds(&mut session);
        assert_eq!(
            sounds,
            [SimSound::Halt(&WARP_UP_HALTS), SimSound::JumpCollapsed],
            "the halt, then the beep"
        );
        assert!(!sounds.contains(&SimSound::Halt(&PLAYER_EXPLOSION_HALTS)));
        assert_eq!(WARP_UP_HALTS.map(|sound| sound.0), [128, 129]);
    }

    #[test]
    fn a_dying_player_collapses_too() {
        let mut session = braking(9);
        let before = *session.player();
        session.condition = Condition::Dying {
            ticks_left: 10,
            starting: false,
        };
        session.tick(Controls::default());
        assert_eq!(session.preparing_jump(), None);
        assert_eq!(session.take_messages(), [SimMessage::JumpCollapsed]);
        let speed = warp::lead_in_speed(2, factor(), 9);
        let collapsed =
            collapsed_velocity(before.velocity, before.heading, speed, session.top_speed());
        assert_eq!(session.player().velocity, collapsed * DISABLED_DRAG);
    }

    #[test]
    fn a_collapsed_jump_is_not_begun_again() {
        let mut session = braking(9);
        disable_and_tick(&mut session);
        session.take_messages();
        session.take_sounds();
        assert_eq!(session.begin_jump(), Err(JumpRefusal::Disabled));
        for _ in 0..5 {
            session.tick(Controls::default());
        }
        assert_eq!(session.take_messages(), [], "once");
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.preparing_jump(), None);
    }

    #[test]
    fn a_disabled_player_with_no_jump_collapses_nothing() {
        let mut session = bound_for_131(&catalog(), OUT, Vec2::new(1.0, 0.0), 0.0);
        session.warp_sounds[0] = 9;
        disable_and_tick(&mut session);
        assert_eq!(session.take_messages(), []);
        assert_eq!(session.take_sounds(), []);
        assert_eq!(
            session.player().velocity,
            Vec2::new(1.0, 0.0) * DISABLED_DRAG
        );
    }

    #[test]
    fn an_intact_player_in_its_pre_jump_never_collapses() {
        let mut session = braking(9);
        for _ in 0..5 {
            session.tick(Controls::default());
        }
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.take_messages(), []);
        assert!(!session.take_sounds().contains(&SimSound::JumpCollapsed));
    }
}
