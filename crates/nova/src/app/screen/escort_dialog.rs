//! The escort dialog over flight, and the report of the escorts sold and
//! upgraded over the spaceport (see `nova_sim::escort_dialog`).
//!
//! A fleet escort hailed opens the escort dialog over flight, which
//! pauses under it as under the comm dialog. Each press goes through
//! flight ([`FlightView::press_escort`]): Upgrade and Sell show the
//! marks changed, and Close Channel and Release close it, resuming
//! flight and keeping the dialog's sounds.
//!
//! Leaving the spaceport, by its Leave or a `Q`'s, first settles the
//! escorts' marks, and passes the escort days and pays a day's wages
//! ([`FlightView::land_escorts`]). A report of what was
//! sold and upgraded shows over the spaceport in "Desc Dialog", as
//! `_DoEscortLand` shows it with `_ShowTextDialog` before the leave's
//! days, pay and take-off (@0x405bb-0x407e8), and the leave waits for it
//! to close: the take-off is a boundary the player sees, so the report
//! never shows over flight. Once it closes it plays the last `dësc`
//! loaded's after-movie, as `_ShowTextDialog`'s after-check reads it
//! (@0x5b40b-0x5b44f; `SessionFlight::loaded_after_movie`), and the leave
//! waits through that too. An escort that defects for want of the pay
//! shows after the report, in the same way (see the `defection` module).

use std::rc::Rc;

use nova_sim::escort_dialog::EscortDialogView;
#[cfg(doc)]
use nova_view::flight::FlightView;
use nova_view::spaceport::SpaceportView;
use nova_view::ui::DescriptionSource;
use nova_view::ui::escort_dialog::{ESCORT_DIALOG, EscortDialog};
use nova_view::{Input, Screen, ScreenAction};

use super::{AppScreen, ENTERED};

impl AppScreen {
    /// Opens the escort dialog on `view` over flight, which pauses: the
    /// interface file's, or the built-in one without it (with a warning
    /// when the interface file has none). With no metrics to lay it out,
    /// the dialog is closed at once, with a warning. Flight's cursor hides
    /// first: `_DoEscortDialog` runs inside `_DoCommDialog` (@0x95742),
    /// after `_HandlePlayerCommunication` hid its sprite (@0x62561).
    pub(super) fn open_escort_dialog(&mut self, view: &EscortDialogView) {
        self.hide_flight_cursor();
        let Some(metrics) = self.metrics() else {
            if let Some(flight) = &mut self.flight {
                flight.press_escort(nova_sim::escort_dialog::EscortButton::Close);
            }
            self.warnings
                .push("nova: cannot show the escort dialog: no metrics to lay it out".to_owned());
            return;
        };
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(ESCORT_DIALOG)
                .and_then(|template| EscortDialog::new(&template, view, style, Rc::clone(&metrics)))
        });
        let dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings
                        .push(format!("nova: using the built-in escort dialog: {reason}"));
                }
                EscortDialog::fallback(view, style, metrics)
            }
        };
        self.escort_dialog = Some(
            dialog
                .with_focus_ring(self.dialog_focus_ring())
                .with_bindings(self.bindings.clone()),
        );
    }

    /// The escort dialog's input: each press goes through flight, and the
    /// dialog shows what came of it; a press that closes it resumes
    /// flight, keeping the dialog's sounds.
    pub(super) fn escort_dialog_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.escort_dialog.as_mut().expect("open");
        dialog.input(input);
        let Some(press) = dialog.take_press() else {
            return ScreenAction::None;
        };
        let flight = self.flight.as_mut().expect(ENTERED);
        match flight.press_escort(press) {
            Some(view) => {
                if let Some(dialog) = &mut self.escort_dialog {
                    dialog.set_view(&view);
                }
            }
            None => {
                if let Some(mut dialog) = self.escort_dialog.take() {
                    self.sounds.extend(dialog.take_sounds());
                }
            }
        }
        ScreenAction::None
    }

    /// Settles the escorts' marks as the ship leaves the spaceport, and
    /// the escort days and their pay with them, and shows over it the
    /// report of what was sold and upgraded, then each escort that
    /// defected for want of that pay (`AppScreen::show_defections`), as
    /// `_DoEscortLand` shows them (@0x407e8, @0x4085a); whether they hold
    /// the leave. Without the spaceport's text dialog nothing is shown
    /// and nothing holds the leave.
    pub(super) fn report_escorts_landed(&mut self) -> bool {
        let Some(flight) = self.flight.as_mut() else {
            return false;
        };
        let report = flight.land_escorts();
        let after = flight.sim().loaded_after_movie();
        let Some(spaceport) = &mut self.spaceport else {
            return false;
        };
        if let Some(text) = report {
            spaceport.show_text(&text, after);
        }
        self.show_defections();
        self.spaceport
            .as_ref()
            .is_some_and(SpaceportView::holds_leave)
    }
}
