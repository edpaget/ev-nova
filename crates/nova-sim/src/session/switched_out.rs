//! The app switched out: the warp-ups' halt of `_SwitchedOut`
//! (@0x4f381-0x4f420), run as the app loses focus.
//!
//! `_SwitchedOut` halts `_warpUpSnd[0]`, `snd ` 128, then
//! `_warpUpSnd[1]`, 129, each if playing (`ST_IsSoundPlaying` @0x4f3b2,
//! `ST_HaltSound` @0x4f3c1; @0x4f3d0, @0x4f3e0). Ours raises one
//! [`SimSound::Halt`](crate::sound::SimSound::Halt) of
//! [`WARP_UP_HALTS`](crate::sound::WARP_UP_HALTS), which also ends the
//! session's count of them, so a warp's end takes the natural end
//! (`natural_end` in `Session::arrive`) as the original does with the
//! warp-up halted. The intro music's stop (@0x4f390-0x4f3a0) is the
//! audio core's (`nova_audio`'s `UiSound::SwitchedOut`); the rest of the
//! function is recorded in rdm phase play-feel-5 19's findings.

use super::Session;

impl Session {
    /// The app was switched out: halt both warp-ups, as `_SwitchedOut`
    /// does (@0x4f3a3-0x4f3e0).
    ///
    /// The halt is raised every time, not only while the session counts
    /// a warp-up as playing: `ST_IsSoundPlaying` asks the sound itself,
    /// halting a sound that isn't playing does nothing, and the session's
    /// count can be wrong either way (a warp-up still sounding as the ship
    /// lands is taken as over), so gating on it could leave one playing.
    pub fn switched_out(&mut self) {
        self.ship
            .halt_sounds(&mut self.out, &crate::sound::WARP_UP_HALTS);
    }
}

#[cfg(test)]
mod tests {
    use crate::flight_frame::{Answer, FlightCommand};
    use crate::session::ApplyCollaborators;
    use crate::session::Session;
    use crate::sound::{SimSound, WARP_UP_HALTS};
    use crate::testkit::catalog;

    /// Runs `command` on `session` with the default collaborators.
    fn apply(session: &mut Session, command: FlightCommand) -> Answer {
        session.apply(
            command,
            &mut ApplyCollaborators {
                catalog: None,
                chance: &mut crate::chance::NeverFires,
                law: &crate::legal::NovaLaw::default(),
                boarding_rule: &crate::board::NovaBoarding::default(),
                hail_options: &crate::hail::HailOptions::default(),
            },
        )
    }

    #[test]
    fn switching_out_halts_both_warp_ups_and_ends_them() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.ship.warp.warp_ups.start(false, 100);
        session.ship.warp.warp_ups.start(true, 100);
        session.take_sounds();
        session.switched_out();
        assert_eq!(session.take_sounds(), [SimSound::Halt(&WARP_UP_HALTS)]);
        assert!(!session.ship.warp.warp_ups.playing(false));
        assert!(!session.ship.warp.warp_ups.playing(true));
    }

    #[test]
    fn switching_out_halts_even_with_no_warp_up_counted() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.take_sounds();
        session.switched_out();
        assert_eq!(session.take_sounds(), [SimSound::Halt(&WARP_UP_HALTS)]);
    }

    #[test]
    fn the_switched_out_command_halts_the_warp_ups() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.ship.warp.warp_ups.start(false, 100);
        session.take_sounds();
        assert_eq!(
            apply(&mut session, FlightCommand::SwitchedOut),
            Answer::Done
        );
        assert_eq!(session.take_sounds(), [SimSound::Halt(&WARP_UP_HALTS)]);
        assert!(!session.ship.warp.warp_ups.playing(false));
    }
}
