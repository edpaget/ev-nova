//! An escort's defection shown as a text dialog, as `_DoEscortPayment`
//! shows it (@0x40111-0x4031e): `_ShowTextDialog(0, 0, 0)` (@0x40301)
//! holding `STR#` 2002 #302 or #303, with no picture and no
//! before-movie, then the last `dësc` loaded's after-movie once it closes
//! (@0x5b40b-0x5b44f; `SessionFlight::loaded_after_movie`). Flight keeps
//! the defections off its message line ([`FlightView::take_defections`]);
//! this glue shows each where the original's dialog comes up:
//!
//! - **Over the spaceport**, when it is open: the take-off's pay runs in
//!   `_DoEscortLand` (@0x4085a), with the port dialog still up, after the
//!   escorts' report (`AppScreen::report_escorts_landed`), so its dialog
//!   queues behind the report, unfaded, and holds the leave
//!   ([`SpaceportView::holds_leave`]).
//! - **Over flight** otherwise, through the mission dialogs over flight
//!   (`AppScreen::show_over_flight`): at a jump's arrival the pay is the
//!   arrival's last call (@0x6c616-0x6c620), so the arrival's own texts
//!   come first, and the first text shown on the screen the warp lit
//!   white takes `_ShowTextDialog`'s faded branch (@0x5af96-0x5b038),
//!   fading it in from black (see the `jump_white` module). That ends the
//!   jump's effect, and the session has put the white out
//!   (@0x40298-0x402aa), so none fades back (@0x6840b-0x6841f).
//!
//! No `_SetAllKeyCmdFlags` follows either dialog (`_DoEscortLand` runs
//! inside the port's loop, and @0x6c620 has no site after it in
//! `_HandlePlayer`'s pass), so over flight a latched key pressed under the
//! defection and still down at its close acts on the next step (see
//! `live_input`); over the spaceport the take-off's own site latches it.
//!
//! On any other side (a map or info screen over flight) they stay kept
//! until flight shows again. Without the dialogs' "Desc Dialog" they are
//! dropped, as every text without it is.
//!
//! The bar needs no redraw once one closes (`_RefreshSAT` @0x40306,
//! `_DrawStatus` @0x40312): at an arrival the pay comes before flight
//! lays the system out and redraws the bar from the frame, and at a
//! take-off before the take-off's own redraw, so the bar already shows
//! the fleet and cash after the pay.

#[cfg(doc)]
use nova_view::flight::FlightView;
#[cfg(doc)]
use nova_view::spaceport::SpaceportView;
use nova_view::ui::mission_dialogs::DescShown;

use super::{AppScreen, Side};

impl AppScreen {
    /// Shows the escorts' defections flight kept, each in "Desc Dialog"
    /// with the last `dësc` loaded's after-movie (see the module docs):
    /// over the spaceport if it is open, else over flight, when it shows.
    pub(super) fn show_defections(&mut self) {
        if self.spaceport.is_none() && self.side != Side::Flight {
            return;
        }
        let Some(flight) = self.flight.as_mut() else {
            return;
        };
        let defections = flight.take_defections();
        if defections.is_empty() {
            return;
        }
        let after = flight.sim().loaded_after_movie();
        if let Some(spaceport) = &mut self.spaceport {
            for text in &defections {
                spaceport.show_text(text, after.clone());
            }
            return;
        }
        self.show_over_flight(
            defections
                .into_iter()
                .map(|text| DescShown::str_text(text, after.clone()))
                .collect(),
        );
    }
}
