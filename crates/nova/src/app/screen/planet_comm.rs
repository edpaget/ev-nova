//! The planet comm dialog over flight (see `nova_sim::planet_comm`).
//!
//! A navigation stellar hailed opens the planet comm dialog over flight,
//! which pauses under it as under the comm dialog. Each press goes through
//! flight (`FlightView::press_planet_comm`): its reply and buttons are
//! shown afresh, Offer Bribe opens the haggle dialog over it on the price
//! asked ("Pay us"), each choice there goes through flight too
//! (`FlightView::haggle_bribe`), and Close Channel, or a bribe paid,
//! closes it, resuming flight and keeping the dialog's sounds. The
//! dialog draws the stellar's frame as the flight shows it as it opens
//! (`FlightView::stellar_frame`).

use std::rc::Rc;

use nova_sim::Haggle;
use nova_sim::planet_comm::{PlanetCommItem, PlanetCommView};
use nova_view::ui::DescriptionSource;
use nova_view::ui::planet_comm::{PLANET_COMM_DIALOG, PlanetCommDialog};
use nova_view::{Input, Screen, ScreenAction};

use super::{AppScreen, ENTERED};

impl AppScreen {
    /// Opens the planet comm dialog on `view` over flight, which pauses:
    /// the interface file's, or the built-in one without it (with a
    /// warning when the interface file has none). With no metrics to lay
    /// it out, the dialog is closed at once, with a warning.
    pub(super) fn open_planet_comm(&mut self, view: &PlanetCommView) {
        let Some(metrics) = self.metrics() else {
            if let Some(flight) = &mut self.flight {
                flight.press_planet_comm(PlanetCommItem::Close);
            }
            self.warnings.push(
                "nova: cannot show the planet comm dialog: no metrics to lay it out".to_owned(),
            );
            return;
        };
        let sprite = self
            .flight
            .as_ref()
            .and_then(|flight| flight.stellar_frame(view.stellar));
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(PLANET_COMM_DIALOG)
                .and_then(|template| {
                    PlanetCommDialog::new(&template, view, sprite, style, Rc::clone(&metrics))
                })
        });
        let dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings.push(format!(
                        "nova: using the built-in planet comm dialog: {reason}"
                    ));
                }
                PlanetCommDialog::fallback(view, sprite, style, metrics)
            }
        };
        self.planet_comm = Some(dialog);
    }

    /// The planet comm dialog's input: each press goes through flight, and
    /// the dialog shows what came of it, opening the haggle dialog over it
    /// when a price is asked; a press that closes it resumes flight,
    /// keeping the dialog's sounds.
    pub(super) fn planet_comm_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.planet_comm.as_mut().expect("open");
        dialog.input(input);
        let Some(press) = dialog.take_press() else {
            return ScreenAction::None;
        };
        let flight = self.flight.as_mut().expect(ENTERED);
        match flight.press_planet_comm(press) {
            Some(view) => {
                if let Some(dialog) = &mut self.planet_comm {
                    dialog.set_view(&view);
                }
                if let Some(price) = view.asking {
                    self.open_haggle(price, false);
                }
            }
            None => self.close_planet_comm_dialog(),
        }
        ScreenAction::None
    }

    /// The bribe haggle's `choice`, made over the planet comm dialog: a
    /// price lowered is shown; otherwise the haggle dialog closes, keeping
    /// its sounds, and the planet comm dialog shows what came of it, or
    /// closes too once the bribe is paid.
    pub(super) fn bribe_haggled(&mut self, choice: Haggle) {
        let flight = self.flight.as_mut().expect(ENTERED);
        let shown = flight.haggle_bribe(choice);
        if let Some(price) = shown.as_ref().and_then(|view| view.asking) {
            if let Some(dialog) = &mut self.haggle {
                dialog.set_asking(price, false);
            }
            return;
        }
        if let Some(mut dialog) = self.haggle.take() {
            self.sounds.extend(dialog.take_sounds());
        }
        match shown {
            Some(view) => {
                if let Some(dialog) = &mut self.planet_comm {
                    dialog.set_view(&view);
                }
            }
            None => self.close_planet_comm_dialog(),
        }
    }

    /// Takes the planet comm dialog down, keeping its sounds.
    fn close_planet_comm_dialog(&mut self) {
        if let Some(mut dialog) = self.planet_comm.take() {
            self.sounds.extend(dialog.take_sounds());
        }
    }
}
