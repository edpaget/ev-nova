//! [`FlightView`]'s player info screen and the main screen's pilot panel,
//! over the [`FlightSim`] port: what each shows is the flight's
//! ([`PilotSource`](crate::flight::sim_port::PilotSource)), the player info key's press and the screen's opening
//! its commands ([`FlightCommand::PressPlayerInfo`],
//! [`FlightCommand::OpenPlayerInfo`]), and the view marks the screen
//! with its own cheats setting.

use nova_sim::PlayerCommand;
use nova_sim::flight_frame::{Answer, FlightCommand};
use nova_sim::{PlayerInfo, pilot_panel::PilotPanel};

use crate::ScreenAction;
use crate::flight::sim_port::FlightSim;

use super::FlightView;

impl<C, S: FlightSim> FlightView<C, S> {
    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY)
    /// asked for the player info screen since this was last taken, once:
    /// the router opens it on [`FlightView::player_info`] over the paused
    /// flight.
    pub fn take_player_info_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_player_info)
    }

    /// What the player info screen shows, as the flight gives it
    /// ([`PilotSource::player_info`](crate::flight::sim_port::PilotSource::player_info)), marked with the cheats setting,
    /// changing nothing; `None` for a flight with no pilot to show, one
    /// that failed. An opening of the screen is
    /// [`FlightView::open_player_info`].
    #[must_use]
    pub fn player_info(&self) -> Option<PlayerInfo> {
        Some(self.marked(self.sim.player_info()?))
    }

    /// What the main screen's pilot panel says about the pilot, as the
    /// flight gives it ([`PilotSource::pilot_panel`](crate::flight::sim_port::PilotSource::pilot_panel)), changing nothing;
    /// `None` for a flight that failed.
    #[must_use]
    pub fn pilot_panel(&self) -> Option<PilotPanel> {
        self.sim.pilot_panel()
    }

    /// The player info dialog opens ([`FlightCommand::OpenPlayerInfo`],
    /// `nova_sim::Session::open_player_info`): the flight resets first, so
    /// the screen it gives shows the cargo trimmed; none in a jump or
    /// while the ship breaks up, when nothing opens, or for a flight that
    /// refuses it, one that failed. It is marked with the cheats setting.
    /// The router opens the screen on it, from flight and from the
    /// spaceport alike.
    pub fn open_player_info(&mut self) -> Option<PlayerInfo> {
        match self.send(FlightCommand::OpenPlayerInfo) {
            Answer::PlayerInfo(Some(info)) => Some(self.marked(*info)),
            _ => None,
        }
    }

    /// `info` marked with the cheats setting ([`FlightOverlays::cheats`](super::FlightOverlays::cheats)),
    /// which the flight leaves off.
    fn marked(&self, info: PlayerInfo) -> PlayerInfo {
        PlayerInfo {
            cheats: self.overlays.cheats,
            ..info
        }
    }

    /// Presses [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY)
    /// outside a jump, its [`PlayerCommand`] taken: when the flight takes
    /// the press too ([`FlightView::press_player_info`]), asks the router
    /// for the player info screen, letting go of the flight keys
    /// (`_HandlePlayer` @0x702bf-0x703c9); when it refuses, with its beep,
    /// nothing more.
    pub(super) fn ask_player_info(&mut self) -> ScreenAction {
        if self.press_player_info() {
            self.pending_player_info = true;
            self.held.clear();
        }
        ScreenAction::None
    }

    /// Whether the flight takes a press of
    /// [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY)
    /// ([`FlightCommand::PressPlayerInfo`],
    /// `nova_sim::Session::press_player_info`, which beeps a refusal).
    /// Sent with no gate of its own: `Screen::input`'s press filter has
    /// asked the key's first, so the flight's own refusal is the one
    /// left. A flight that refuses the command, one that failed, takes
    /// it, and the router opens nothing for it, as a failed session
    /// always has: the decision is the view's, not the adapter's.
    fn press_player_info(&mut self) -> bool {
        !matches!(
            self.send(FlightCommand::PressPlayerInfo),
            Answer::PlayerInfoPressed(false)
        )
    }

    /// A press of [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY)
    /// during a jump: outside the map, the flight refuses it with its beep
    /// ([`FlightView::press_player_info`], @0x702f7, @0x7039a-0x703bd),
    /// when the player's fate takes it.
    pub(super) fn refuse_player_info(&mut self) -> ScreenAction {
        if !self.map_open && self.takes(PlayerCommand::PlayerInfo) {
            let _ = self.press_player_info();
        }
        ScreenAction::None
    }
}
