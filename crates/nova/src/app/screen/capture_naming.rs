//! [`AppScreen`]'s rename of a ship captured: the Text Input dialog
//! (`DLOG` 3001) `_DoPlunderDialog` opens once the assignment dialog
//! returns Use As My Ship (@0x943a3-0x944ab), asking with
//! `nova_sim::capture_naming`'s prompt and default, which flight answers
//! ([`FlightView::capture_naming`]).
//!
//! OK sends the text typed, unculled, and Cancel none, through flight
//! ([`FlightView::name_capture`]), which takes the ship over or loses the
//! prize and says so. Escape does nothing, as in every Text Input dialog
//! (`_EVTextInputDialog` has no cancel item). The dialog's clicks and its
//! overlong beep are kept.
//!
//! The plunder dialog stays drawn, inert, under the assignment dialog and
//! the rename, as neither hides it (`_DoCapturedShipAssignmentDialog`
//! @0x5b534-0x5b69e, `_EVTextInputDialog`); it closes, keeping its sounds,
//! when the branch exits by the plunder dialog's exit (@0x936fc,
//! `_DisposeDialogNoFlash` @0x949f1): after Use As Escort, or once the
//! rename is answered either way ([`AppScreen::close_plunder`]).

use std::rc::Rc;

use nova_sim::shipyard::SHIP_NAME_CHARS;
use nova_view::flight::FlightView;
use nova_view::ui::text_input::{TEXT_INPUT_DIALOG, TextInputDialog, TextInputOutcome};
use nova_view::ui::{DescriptionSource, DialogResources};
use nova_view::{Input, Screen, ScreenAction};

use super::{AppScreen, ENTERED};

impl AppScreen {
    /// Opens the rename of the ship captured over the plunder dialog:
    /// the interface file's Text Input dialog, or the built-in one without
    /// it (with a warning when the interface file has none), on the prompt
    /// and default flight answers (empty when it answers none). Opened
    /// `by_key`, the text the key that chose Use As My Ship types goes
    /// nowhere, as `_EVTextInputDialog` flushes events as it opens
    /// (@0x55429). With no metrics to lay it out, nothing opens.
    pub(super) fn open_capture_naming(&mut self, by_key: bool) {
        let Some(metrics) = self.metrics() else {
            return;
        };
        let asked = self
            .flight
            .as_mut()
            .and_then(FlightView::capture_naming)
            .unwrap_or_else(|| nova_sim::capture_naming::CaptureNaming {
                prompt: String::new(),
                default: String::new(),
            });
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(TEXT_INPUT_DIALOG)
                .and_then(|template| {
                    TextInputDialog::new(
                        &template,
                        &asked.prompt,
                        &asked.default,
                        SHIP_NAME_CHARS,
                        style,
                        Rc::clone(&metrics),
                    )
                })
        });
        let mut dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings.push(format!(
                        "nova: using the built-in Text Input dialog: {reason}"
                    ));
                }
                TextInputDialog::fallback(
                    &asked.prompt,
                    &asked.default,
                    SHIP_NAME_CHARS,
                    style,
                    metrics,
                )
            }
        };
        if by_key {
            dialog.flush_typed_key();
        }
        self.capture_naming = Some(dialog);
    }

    /// The rename's input: OK sends the text confirmed, Cancel none,
    /// through flight; either closes the rename and the plunder dialog
    /// under it, keeping both dialogs' sounds.
    pub(super) fn capture_naming_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.capture_naming.as_mut().expect("open");
        dialog.input(input);
        let Some(outcome) = dialog.take_outcome() else {
            return ScreenAction::None;
        };
        let mut dialog = self.capture_naming.take().expect("open");
        self.sounds.extend(dialog.take_sounds());
        let typed = match outcome {
            TextInputOutcome::Confirm(typed) => Some(typed),
            TextInputOutcome::Cancel => None,
        };
        self.flight.as_mut().expect(ENTERED).name_capture(typed);
        self.close_plunder();
        ScreenAction::None
    }

    /// Closes the plunder dialog left drawn under the assignment and the
    /// rename, keeping its sounds.
    pub(super) fn close_plunder(&mut self) {
        if let Some(mut dialog) = self.plunder.take() {
            self.sounds.extend(dialog.take_sounds());
        }
    }
}
