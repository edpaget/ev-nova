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
//! case; see `menu`'s replace question) replaces it only once it is saved:
//! the old save is deleted then, when the store lists both it and the new
//! save under different spellings, so one pilot is listed, as the Mac's
//! file system keeps one file. A store that ignores case lists one, which
//! the save already replaced, and a failed save deletes nothing. Cancel leaves the old save as it was, as `_DoNewPilot`
//! writes the file only at `_SavePilotFile` (@0x1935f).

use std::rc::Rc;

use nova_sim::christening::{christen, christening};
use nova_sim::shipyard::SHIP_NAME_CHARS;
use nova_sim::{Pilot, pilot_key};
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

    /// Deletes the save under `old` that a new pilot saved under `new`
    /// replaces, when the store lists both under different spellings (see
    /// the module docs); a failed delete is a warning.
    fn drop_replaced(&mut self, old: &str, new: Option<&str>) {
        let Some(keeper) = self.keeper() else {
            return;
        };
        let Some(new) = new.filter(|&new| new != old) else {
            return;
        };
        let both = keeper.list().is_ok_and(|keys| {
            keys.iter().any(|key| key == old) && keys.iter().any(|key| key == new)
        });
        if both && let Err(warning) = keeper.delete(old) {
            self.warnings.push(warning);
        }
    }

    /// The christening's input. OK christens the ship with the name
    /// confirmed, culled, and holds the new pilot, its game begun and
    /// saved ([`AppScreen::hold_new_pilot`]), the pilot it replaces deleted
    /// when it is saved under another spelling; Cancel abandons it. Either
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
            let key = pilot_key(pilot.name());
            let flight = self.pilot_flight(christen(pilot, &typed), false);
            self.hold_new_pilot(flight);
            if let Some(old) = replaces {
                self.drop_replaced(&old, key.as_deref());
            }
        }
        ScreenAction::None
    }
}
