//! [`AppScreen`]'s christening of a new pilot's ship: the Text Input
//! dialog (`DLOG` 3001) `_DoNewPilot` opens once the New Pilot dialog is
//! confirmed (@0x18cca-0x18d76), asking the ship's name with
//! `nova_sim::christening`'s prompt and default.
//!
//! OK christens the ship, culled (@0x19189-0x191ae), then begins the
//! pilot's game and saves it ([`AppScreen::hold_new_pilot`]), so the
//! `chär`'s `OnStart` (@0x19345) and the first save (@0x1935f) come after
//! the name, as in the original. Cancel abandons the new pilot (@0x18d76):
//! nothing is saved, and any game held before stays held.
//!
//! A new pilot that replaces one already saved (its name matched ignoring
//! case; see `menu`'s replace question) is saved in its place
//! (`nova_sim::saves::PilotKeeper::save_replacing`, which decides what
//! becomes of the old save), and a failed delete of the old save is a
//! warning. Cancel leaves the old save as it was, as `_DoNewPilot` writes
//! the file only at `_SavePilotFile` (@0x1935f).

use std::rc::Rc;

use nova_sim::Pilot;
use nova_sim::christening::{christen, christening};
use nova_sim::shipyard::SHIP_NAME_CHARS;
use nova_view::ui::text_input::{TEXT_INPUT_DIALOG, TextInputDialog, TextInputOutcome};
use nova_view::ui::{DescriptionSource, DialogResources};
use nova_view::{Input, Screen, ScreenAction};

use super::{AppScreen, MENU};

/// The christening open over the main menu: its dialog, and the new
/// pilot it names the ship of.
#[derive(Clone, Debug)]
pub(super) struct Christening {
    /// The Text Input dialog asking the name.
    pub(super) dialog: TextInputDialog,
    /// The new pilot, its ship not yet christened.
    pilot: Pilot,
    /// The key of the saved pilot the new one replaces, if any.
    replaces: Option<String>,
}

impl AppScreen {
    /// Opens the christening of `pilot`'s ship: the interface file's Text
    /// Input dialog, or the built-in one without it (with a warning when
    /// the interface file has none), its default rolled on the router's
    /// chance. Opened `by_key`, the text the key that confirmed the New
    /// Pilot dialog types (Return's carriage return) goes nowhere, as
    /// `_EVTextInputDialog` flushes events as it opens (@0x55429); opened
    /// by a click, nothing is pending, so nothing is dropped. A pilot that
    /// `replaces` a saved one replaces it once saved (see the module docs).
    pub(super) fn open_christening(
        &mut self,
        pilot: Pilot,
        by_key: bool,
        replaces: Option<String>,
    ) {
        let asked = christening(
            self.data.as_ref(),
            &nova_sim::PilotCatalog::ships(self.data.as_ref()),
            pilot.ship(),
            &mut self.chance,
        );
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
                        Rc::clone(&dialogs.metrics),
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
                let metrics = Rc::clone(&self.menu.as_ref().expect(MENU).metrics);
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
        self.christening = Some(Christening {
            dialog,
            pilot,
            replaces,
        });
    }

    /// The christening's input. OK christens the ship with the name
    /// confirmed, culled, and holds the new pilot, its game begun and
    /// saved in place of the pilot it replaces ([`AppScreen::hold_new_pilot`]);
    /// Cancel abandons it. Either
    /// keeps the dialog's sounds.
    pub(super) fn christening_input(&mut self, input: &Input) -> ScreenAction {
        let open = self.christening.as_mut().expect("open");
        open.dialog.input(input);
        let Some(outcome) = open.dialog.take_outcome() else {
            return ScreenAction::None;
        };
        let Christening {
            mut dialog,
            pilot,
            replaces,
        } = self.christening.take().expect("open");
        self.sounds.extend(dialog.take_sounds());
        if let TextInputOutcome::Confirm(typed) = outcome {
            let flight = self.pilot_flight(christen(pilot, &typed), false);
            self.hold_new_pilot(flight, replaces.as_deref());
        }
        ScreenAction::None
    }
}
