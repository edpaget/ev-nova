//! The take-off grace: for [`TAKEOFF_GRACE_TICKS`] after the ship takes
//! off, the map, mission info and player info keys are refused, as
//! `_HandlePlayer` refuses them while the original's frame counter
//! `_frameCounter` is below 0.
//!
//! **The counter.** `_frameCounter` (0x188332, through the pointer at
//! 0x3b5538) has two writers: `_PlayGame` adds 1 once a frame, after
//! `_DoPlayGameWork` (@0x45f36-0x45f52), and wraps it to 0 past 0x400;
//! and the tail of `_HandlePlayerDockRequest` sets it to -15 (0xfff1,
//! @0x67673-0x67678), after `_RedrawStatus` and `_AdjustSoundVolume`.
//! That tail follows the spaceport's return (`_PlayerLandOnStellar`
//! @0x66f29, its one caller) and a hypergate's or wormhole's exit alike
//! (`_PlayerEnterHypergate` @0x66edb and `_PlayerEnterWormhole` @0x66f12
//! join it at 0x66f40), so both start the grace here: [`Session::take_off`]
//! and a gate's exit. Only a hypergate's -1 (no exit, @0x66ef2-0x66f08)
//! skips the tail, as the engine's refused hypergate starts nothing; a
//! wormhole's -1 is never tested (@0x66f17-0x66f20), so a wormhole that
//! leads nowhere runs the tail and starts the grace too (see the
//! `wormhole_no_exit` module). No jump arrival, pod rescue or flight
//! start writes it. The take-off frame's
//! own key reads see -15, and frame k after it -15 + k, so the keys are
//! refused on 15 frames and taken from the 16th.
//!
//! The engine counts the grace down at each flight tick instead, from 15
//! at the take-off: the key reads after 0 to 14 ticks are refused and
//! those after 15 taken, the same 15 reads, with no frame drawn or
//! dialog opened in between. Pausing stops both: the original's pause
//! (`_gPause`, @0x45f13) leaves `_PlayGame`'s loop (@0x465dd), which no
//! frame runs outside, and its entry (0x457ba-0x45a90) leaves the counter
//! as it was; the engine ticks nothing while paused. The same write also
//! restarts the tribbles and perishables step's frame (see
//! [`market`](crate::market)).
//!
//! **The keys behind it.** Only three keys test the counter's sign:
//!
//! - the map, `mapKey` (slot 0x12): `js` @0x6a7de, silently. Its latch is
//!   set on the refusal (0x6a89f), so a map key held through the grace
//!   does not open later, as the engine's press model has it.
//! - the player info, `playerInfoKey` (slot 0x32): @0x7030f-0x70329, with
//!   the shared refusal beep ([`Session::press_player_info`]).
//! - the mission info, `missionInfoKey` (slot 0x50): `js` @0x7045e,
//!   silently ([`Session::press_mission_info`]). Its refusal clears its
//!   latch (0x705af), so in the original an I key held through the grace
//!   opens the dialog on the first frame after it. The view acts on
//!   presses for its dialog keys, so a held key doesn't retry; that is the
//!   general held-key model, not this grace.
//!
//! The counter's other sign tests (0x2e6a1, 0x32637, 0x3529c, 0x45b7d,
//! 0x68c79, 0x6fc4d, 0x706e4-0x7094d, 0x8fdbc) are periodic modulos, not
//! gates. The map key's `_fadePending` gate (@0x6a7bd-0x6a7cf) is screen
//! state the session doesn't hold, as [`Session::open_player_info`]
//! records for its dialog.

use super::{Port, Session};
use crate::market;
use crate::player_fate::PlayerCommand;

/// The key reads refused after a take-off: `_HandlePlayerDockRequest`
/// sets `_frameCounter` to -15 (@0x67678), and `_PlayGame` adds 1 a frame
/// (@0x45f36), so the reads see it below 0 for 15 frames.
pub const TAKEOFF_GRACE_TICKS: u32 = 15;

impl Session {
    /// Starts the take-off grace, and the tribbles and perishables step's
    /// frame with it: the dock request's tail (@0x67673-0x67678), run as
    /// the ship takes off or comes out of a gate.
    pub(super) fn start_takeoff_grace(&mut self) {
        self.space.frame = market::AFTER_TAKE_OFF_FRAME;
        self.port.takeoff_grace = TAKEOFF_GRACE_TICKS;
    }
}

impl Port {
    /// Counts the take-off grace down a flight tick.
    pub(super) fn takeoff_grace_tick(&mut self) {
        self.takeoff_grace = self.takeoff_grace.saturating_sub(1);
    }

    /// Whether the take-off grace is still running.
    pub(super) fn in_takeoff_grace(&self) -> bool {
        self.takeoff_grace > 0
    }

    /// Whether the take-off grace refuses `command`: the map and the
    /// mission info, silently, while it runs. The player info is refused
    /// by [`Session::press_player_info`], with its beep.
    pub(super) fn takeoff_grace_refuses(&self, command: PlayerCommand) -> bool {
        self.in_takeoff_grace()
            && matches!(command, PlayerCommand::Map | PlayerCommand::MissionInfo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SystemId;
    use crate::chance::NeverFires;

    use crate::flight::Controls;
    use crate::session::tests::{at_gate, at_wormhole, gated, wormholes};
    use crate::sound::SimSound;
    use crate::testkit::work;
    use crate::testkit::{catalog, jump, land_now};

    /// A session that has landed and taken off, its sounds taken.
    fn taken_off() -> Session {
        let mut session = Session::start(&catalog()).expect("starts");
        land_now(&mut session).expect("lands");
        session
            .take_off(&catalog(), &mut NeverFires)
            .expect("took off");
        let _ = session.take_sounds();
        session
    }

    /// What the keys behind the grace do in `session` now.
    #[derive(Debug, PartialEq)]
    struct Keys {
        /// Whether the map is taken.
        map: bool,
        /// Whether the mission info is taken.
        mission_info: bool,
        /// Whether a press of the player info is taken.
        player_info: bool,
        /// The sounds that press made.
        sounds: Vec<SimSound>,
    }

    fn keys(session: &mut Session) -> Keys {
        Keys {
            map: session.takes(PlayerCommand::Map),
            mission_info: session.takes(PlayerCommand::MissionInfo),
            player_info: session.press_player_info(),
            sounds: session.take_sounds(),
        }
    }

    /// All three refused, the player info with its beep.
    fn refused() -> Keys {
        Keys {
            map: false,
            mission_info: false,
            player_info: false,
            sounds: vec![SimSound::Refused],
        }
    }

    /// All three taken, in silence.
    fn taken() -> Keys {
        Keys {
            map: true,
            mission_info: true,
            player_info: true,
            sounds: Vec::new(),
        }
    }

    #[test]
    fn the_grace_is_15_ticks() {
        assert_eq!(TAKEOFF_GRACE_TICKS, 15);
    }

    #[test]
    fn the_keys_are_refused_for_15_ticks_after_a_take_off_and_taken_after() {
        let mut session = taken_off();
        for tick in 0..15 {
            assert_eq!(keys(&mut session), refused(), "after {tick} ticks");
            work(
                &mut session,
                &crate::testkit::catalog(),
                Controls::default(),
            );
        }
        assert_eq!(keys(&mut session), taken(), "after 15 ticks");
        assert!(session.takes(PlayerCommand::Jump), "J is not behind it");
    }

    #[test]
    fn only_the_map_mission_info_and_player_info_are_behind_the_grace() {
        let session = taken_off();
        for command in PlayerCommand::ALL {
            let behind = matches!(
                command,
                PlayerCommand::Map | PlayerCommand::MissionInfo | PlayerCommand::PlayerInfo
            );
            assert_eq!(
                session.takes(command),
                !behind || command == PlayerCommand::PlayerInfo,
                "{command:?}: the player info is refused by its press"
            );
        }
    }

    #[test]
    fn no_grace_in_a_session_that_starts_in_flight() {
        let mut session = Session::start(&catalog()).expect("starts");
        let _ = session.take_sounds();
        assert_eq!(keys(&mut session), taken());
    }

    #[test]
    fn no_grace_after_a_jumps_arrival() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let _ = session.take_sounds();
        assert_eq!(keys(&mut session), taken());
    }

    #[test]
    fn a_hypergates_exit_starts_the_grace_and_a_cancelled_entry_does_not() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        session.open_hypergate(&catalog).expect("opens");
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
            .expect("enters");
        let _ = session.take_sounds();
        assert_eq!(keys(&mut session), refused(), "out of the gate");
        for _ in 0..15 {
            work(&mut session, &catalog, Controls::default());
        }
        assert_eq!(keys(&mut session), taken(), "after 15 ticks");
        let mut session = at_gate(&catalog);
        session.open_hypergate(&catalog).expect("opens");
        let _ = session.enter_hypergate(None, &catalog, &mut NeverFires);
        let _ = session.take_sounds();
        assert_eq!(keys(&mut session), taken(), "cancelled (@0x66ef2)");
    }

    #[test]
    fn a_wormholes_exit_starts_the_grace() {
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog);
        session
            .enter_wormhole(&catalog, &mut NeverFires)
            .expect("enters");
        let _ = session.take_sounds();
        assert_eq!(keys(&mut session), refused());
    }

    #[test]
    fn a_wormhole_with_no_exit_starts_the_grace_too() {
        // `_HandlePlayerDockRequest` doesn't test the wormhole's -1
        // (@0x66f17-0x66f20), so the tail's -15 is written (@0x67673).
        let mut catalog = wormholes();
        catalog
            .gates
            .retain(|site| site.id == crate::catalog::StellarId(400));
        let mut session = at_wormhole(&catalog);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut NeverFires),
            Err(crate::gate::GateRefusal::NoExit)
        );
        let _ = session.take_sounds();
        for tick in 0..15 {
            assert_eq!(keys(&mut session), refused(), "after {tick} ticks");
            work(&mut session, &catalog, Controls::default());
        }
        assert_eq!(keys(&mut session), taken(), "after 15 ticks");
    }

    #[test]
    fn a_gates_exit_restarts_the_junk_steps_frame_as_a_take_off_does() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        for _ in 0..2 {
            session.space.frame = market::next_frame(session.space.frame);
        }
        session.open_hypergate(&catalog).expect("opens");
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
            .expect("enters");
        assert_eq!(session.space.frame, market::AFTER_TAKE_OFF_FRAME);
        assert_eq!(taken_off().space.frame, market::AFTER_TAKE_OFF_FRAME);
    }
}
