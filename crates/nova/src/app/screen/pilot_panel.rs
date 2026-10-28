//! [`AppScreen`]'s side of the main menu's pilot panel: the menu shows
//! the game held ([`AppScreen::game_in_progress`]) as it stands after the
//! last input or tick.
//!
//! - With no game held, the panel shows no pilot ("No Pilot File Loaded"
//!   once slide 2 has finished).
//! - With a held game that isn't dying, the alive panel
//!   ([`FlightView::pilot_panel`]); with a dying one
//!   ([`SessionFlight::is_dying`](nova_view::flight::SessionFlight::is_dying), `_IsDying`), the killed message.
//! - A held game whose session failed draws no panel and no message: that
//!   state is the engine's alone, as the original has no game in progress
//!   without its player globals.
//!
//! **Under the #52 note** (`_MainScreenChoice` @0x1a82f-0x1a83b): the
//! original redraws the main screen before the note, once `_OpenPilotFile`
//! has loaded the new pilot into the player globals but while
//! `_gameInProgress` still has its old value (it is set @0x1a879). So with
//! a game held before the Open the newly opened pilot's panel shows under
//! the note, and with none held before, "No Pilot File Loaded" does
//! (slide 2 has always finished by then). The version note (#34, a failed
//! open) changes nothing.

use std::rc::Rc;

use nova_data::GameData;
use nova_view::flight::FlightView;
use nova_view::menu::PanelPilot;

use super::AppScreen;

impl AppScreen {
    /// Shows the game held on the main menu's pilot panel (see the module
    /// docs), reading a new ship type's picture from the game data. The
    /// router calls it from `settle_script`, after every input and tick,
    /// and at launch.
    pub(super) fn show_game_on_menu(&mut self) {
        let Some(menu) = &mut self.menu else {
            return;
        };
        let pilot = match &self.game {
            _ if self.note_hides_game => PanelPilot::None,
            None => PanelPilot::None,
            Some(game) => shown(game),
        };
        menu.screen.show_pilot(pilot, self.data.as_ref());
    }
}

/// What the panel shows of the held `game`.
fn shown(game: &FlightView<Rc<GameData>>) -> PanelPilot {
    if game.sim().is_dying() {
        return game.sim().pilot().map_or(PanelPilot::Blank, |pilot| {
            PanelPilot::Dying(pilot.name().to_owned())
        });
    }
    game.pilot_panel().map_or(PanelPilot::Blank, |panel| {
        PanelPilot::Alive(Box::new(panel))
    })
}
