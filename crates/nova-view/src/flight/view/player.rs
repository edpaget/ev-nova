//! [`FlightView`]'s player info screen and the main screen's pilot panel,
//! over the [`FlightSim`] port: the player info key's press and the
//! screen's opening are commands ([`FlightCommand::PressPlayerInfo`],
//! [`FlightCommand::OpenPlayerInfo`], whose answer carries the screen),
//! and the view marks the screen with its own cheats setting; the pilot
//! panel is the flight's answer to its screen ask
//! ([`ScreenAsk::PilotPanel`]), kept until the flight next changes
//! ([`KeptPanel`]).

use nova_sim::PlayerCommand;
use nova_sim::flight_frame::{Answer, FlightCommand, ScreenAnswer, ScreenAsk};
use nova_sim::{PlayerInfo, pilot_panel::PilotPanel};

use crate::ScreenAction;
use crate::flight::sim_port::FlightSim;

use super::FlightView;

/// The pilot panel the view keeps between two refills of its frame
/// ([`FlightView::pilot_panel`]), so the menu, which reads the panel after
/// every input and tick, asks the flight once per change. The deferral
/// never crosses a boundary the player sees: every change of the flight
/// refills the frame ([`FlightView::refresh_frame`]: a command, a frame's
/// steps, a catch-up, a change behind the port), which forgets the panel
/// before the next frame reads it. Unlike the floating map's, the panel
/// can't change between two menu frames with no such change.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum KeptPanel {
    /// Not asked since the frame was last refilled.
    #[default]
    Unasked,
    /// Asked, and what the flight answered: none for nothing to answer.
    Answered(Option<PilotPanel>),
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY)
    /// asked for the player info screen since this was last taken, once:
    /// the router opens it on [`FlightView::open_player_info`] over the
    /// paused flight.
    pub fn take_player_info_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_player_info)
    }

    /// What the main screen's pilot panel says about the pilot, as the
    /// flight gives it ([`ScreenAsk::PilotPanel`]), changing nothing;
    /// `None` for a flight with nothing to answer. The menu reads it after
    /// every input and tick, but the panel changes only with the flight,
    /// and every change refills the kept frame, which forgets the panel
    /// kept ([`KeptPanel`]): so it is asked once per change, and read from
    /// what was kept until then, whatever the answer was.
    pub fn pilot_panel(&mut self) -> Option<PilotPanel> {
        if let KeptPanel::Answered(panel) = &self.kept_panel {
            return panel.clone();
        }
        let panel = match self.sim.screen(ScreenAsk::PilotPanel) {
            ScreenAnswer::PilotPanel(panel) => Some(*panel),
            _ => None,
        };
        self.kept_panel = KeptPanel::Answered(panel.clone());
        panel
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
