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
//! escorts' marks ([`FlightView::land_escorts`]). A report of what was
//! sold and upgraded shows over the spaceport in "Desc Dialog", as
//! `_DoEscortLand` shows it with `_ShowTextDialog` before the leave's
//! days, pay and take-off (@0x405bb-0x407e8), and the leave waits for it
//! to close: the take-off is a boundary the player sees, so the report
//! never shows over flight.

use std::rc::Rc;

use nova_sim::escort_dialog::EscortDialogView;
use nova_view::flight::FlightView;
use nova_view::ui::DescriptionSource;
use nova_view::ui::escort_dialog::{ESCORT_DIALOG, EscortDialog};
use nova_view::{Input, Screen, ScreenAction};

use super::{AppScreen, ENTERED};

impl AppScreen {
    /// Opens the escort dialog on `view` over flight, which pauses: the
    /// interface file's, or the built-in one without it (with a warning
    /// when the interface file has none). With no metrics to lay it out,
    /// the dialog is closed at once, with a warning.
    pub(super) fn open_escort_dialog(&mut self, view: &EscortDialogView) {
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
        self.escort_dialog = Some(dialog);
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
    /// shows the report of what was sold and upgraded over it; whether the
    /// report holds the leave. Without the spaceport's text dialog the
    /// report is not shown and nothing holds the leave.
    pub(super) fn report_escorts_landed(&mut self) -> bool {
        let Some(text) = self.flight.as_mut().and_then(FlightView::land_escorts) else {
            return false;
        };
        let Some(spaceport) = &mut self.spaceport else {
            return false;
        };
        spaceport.show_text(&text);
        spaceport.holds_leave()
    }
}
