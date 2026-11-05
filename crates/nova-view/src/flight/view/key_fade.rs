//! Which held command fades the cursor, against what the kept frame says
//! fades it ([`FlightFrame::key_fades`](nova_sim::flight_frame::FlightFrame::key_fades)).
//! The gates are the session's (`nova_sim::key_fade`, which records where
//! each of `_HandlePlayer`'s writes of the byte sits); the keys are the
//! view's, in [`fade_key`]. Every latched block writes the byte before
//! its latch test, so a latched key held past a close fades the cursor as
//! a held one does ([`FlightView::commands_down`]).

use nova_sim::flight_frame::FadeKey;

use crate::Input;
use crate::flight::sim_port::FlightSim;
use crate::keymap::{Command, FlightCommand as F, Kind, PlayCommand};

use super::FlightView;

/// The gate `command` sets the byte behind, or `None` for one that never
/// sets it: Command, Option and Shift alone hold no command, and the
/// escort menu's keys and Show Framerate are polled with no write. The
/// stellar slots and the menu's groups are
/// [`FlightView::slot_polled`]'s.
fn fade_key(command: Command) -> Option<FadeKey> {
    let gate = match command {
        Command::Flight(
            F::TargetSelect
            | F::TargetSelectBack
            | F::TargetEscort
            | F::TargetEscortBack
            | F::ClosestTarget
            | F::ClosestShip
            | F::HyperSelect
            | F::SelectSecondary
            | F::SelectSecondaryBack
            | F::Board
            | F::Communicate
            | F::HailStellar
            | F::Acknowledge
            | F::HyperspaceMode
            | F::NavOff
            | F::ClearTarget
            | F::Autopilot
            | F::FaceStellar
            | F::JettisonCargo
            | F::JettisonNonMission,
        )
        | Command::Play(PlayCommand::GalaxyMap | PlayCommand::PlayerInfo) => FadeKey::Free,
        Command::Flight(
            F::Accelerate
            | F::ReverseCourse
            | F::RotateLeft
            | F::RotateRight
            | F::Afterburner
            | F::FirePrimary
            | F::HyperJump
            | F::EngageCloak,
        ) => FadeKey::Helm,
        Command::Flight(F::FireSecondary) => FadeKey::Secondary,
        Command::Flight(F::WeaponSafety) => FadeKey::WeaponSafety,
        Command::Flight(F::LandDock) => FadeKey::Land,
        Command::Play(PlayCommand::MissionInfo) => FadeKey::MissionInfo,
        Command::Flight(F::Eject) => FadeKey::Eject,
        Command::Flight(F::SelfDestruct) => FadeKey::SelfDestruct,
        _ => return None,
    };
    Some(gate)
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// Whether `command`, pressed or held, fades the cursor on this frame:
    /// a stellar slot or a group while its block polls it, as it writes the
    /// byte behind no gate of its own (@0x69e0d), any other by
    /// [`fade_key`]. The jettison, eject and self-destruct chords hold
    /// their commands only with Command down, as `_HandlePlayer` reads
    /// their keys only then.
    pub(super) fn key_fades_cursor(&self, command: Command) -> bool {
        let fades = self.frame.key_fades;
        match self.slot_polled(command, false) {
            Some(polled) => polled && fades.fades(FadeKey::Free),
            None => fade_key(command).is_some_and(|gate| fades.fades(gate)),
        }
    }

    /// Whether the key `input` presses or repeats fades the cursor: a
    /// command it would hold does ([`FlightView::key_fades_cursor`]).
    pub(super) fn press_fades_cursor(&self, input: &Input) -> bool {
        self.keys
            .meanings(input)
            .into_iter()
            .any(|command| command.kind() == Kind::Held && self.key_fades_cursor(command))
    }

    /// Whether this frame's pass sets the byte with what is read down: a
    /// jump or an auto-eject with no key at all, or any held command that
    /// fades, a latched one held past a close included.
    pub(super) fn cursor_keyed(&self) -> bool {
        self.frame.key_fades.unbidden()
            || self
                .commands_down()
                .any(|command| self.key_fades_cursor(command))
    }
}
