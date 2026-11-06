//! New Pilot's text entry (rdm:phase/key-bindings/phase-17-text-entry-port):
//! the dialog hosts the on-screen keyboard
//! ([`entry_keyboard`](crate::ui::entry_keyboard)) over its focused
//! field, Full Name or Nickname.
//!
//! - It opens with the name focused, and asks for the system's entry
//!   there under the keyboard, the mouse and touch with a soft keyboard.
//! - Tab (the right shoulder too) moves the ask to the other field and
//!   counts a focus, so the system's entry starts again there.
//! - Under the pad it asks for nothing as it opens, so one South on OK
//!   still creates the pilot with the drawn default names; South on a
//!   field raises the on-screen keyboard. Under touch without a soft
//!   keyboard it shows at once.
//! - The keyboard's keys type through the dialog's own input, so the
//!   name's rules (OK's limit, the refusal) are the ones typing meets.

use crate::geometry::Bounds;
use crate::ui::entry_keyboard::{EntryHost, EntryKeyboard};
use crate::ui::text_field::TextField;

use super::{Focus, NewPilotDialog};

impl EntryHost for NewPilotDialog {
    fn keyboard(&self) -> &EntryKeyboard {
        &self.keyboard
    }

    fn keyboard_mut(&mut self) -> &mut EntryKeyboard {
        &mut self.keyboard
    }

    /// The focused field: the name, or the nickname.
    fn entry_field(&self) -> Option<&TextField> {
        match self.focus {
            Focus::Name => Some(&self.field),
            Focus::Nickname => self.nickname.as_ref(),
        }
    }

    /// Empties the focused field; emptying the name clears a refusal,
    /// as any change of it does.
    fn clear_entry_field(&mut self) {
        if let (Focus::Nickname, Some(field)) = (self.focus, &mut self.nickname) {
            field.clear();
        } else {
            self.field.clear();
            self.refusal = None;
        }
    }

    fn dialog_bounds(&self) -> Bounds {
        self.dialog.bounds()
    }

    fn pad_target(&self) -> Option<usize> {
        self.dialog.pad_highlight()
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use nova_sim::Gender;

    use super::super::tests::stock;
    use super::super::{NAME_ITEM, NICKNAME_ITEM, NewPilotOutcome};
    use super::*;
    use crate::draw::DrawList;
    use crate::input::{Device, Input, Key, PadButton, Surroundings};
    use crate::screen::Screen;
    use crate::text::fixture::MonoMetrics;
    use crate::text_entry::EntryAsk;
    use crate::touch::testkit::Fingers;
    use crate::ui::button::ButtonStyle;
    use crate::ui::entry_keyboard::KeyId;
    use crate::ui::pad_focus::every_screen::{PAD, pad, told};

    fn dialog() -> NewPilotDialog {
        NewPilotDialog::new(&stock(), ButtonStyle::STOCK, Rc::new(MonoMetrics))
            .expect("builds")
            .with_defaults("Ada Lovelace", "Ada")
    }

    fn asked(dialog: &NewPilotDialog) -> Option<EntryAsk> {
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        list.text_entry()
    }

    fn keyboard_drawn(dialog: &NewPilotDialog) -> bool {
        !crate::ui::edit_dialog::keyboard_drawn(dialog).is_empty()
    }

    fn tap(dialog: &mut NewPilotDialog, button: PadButton) {
        dialog.input(&pad(button, true));
        dialog.input(&pad(button, false));
    }

    fn rect(dialog: &NewPilotDialog, item: usize) -> Bounds {
        dialog.dialog().item_bounds(item).expect("an item")
    }

    fn created(name: &str, nickname: &str) -> NewPilotOutcome {
        NewPilotOutcome::Create {
            name: name.into(),
            nickname: nickname.into(),
            gender: Gender::Male,
            strict_play: false,
        }
    }

    /// The keys that clear a field and type `zoe`: Shift is on as the
    /// field takes the focus, so it types "Zoe".
    const ZOE: [KeyId; 4] = [
        KeyId::Clear,
        KeyId::Char('z'),
        KeyId::Char('o'),
        KeyId::Char('e'),
    ];

    /// Types `keys` on `dialog`'s raised keyboard from pad inputs alone.
    fn pad_types(dialog: &mut NewPilotDialog, keys: &[KeyId]) {
        let keyboard = dialog.keyboard();
        let from = keyboard.pad_focus().expect("the pad's focus");
        for input in keyboard
            .layout(dialog.dialog_bounds())
            .pad_presses(from, keys)
        {
            dialog.input(&input);
        }
    }

    #[test]
    fn opening_asks_for_entry_at_the_name_field() {
        for device in [Device::Keyboard, Device::Mouse] {
            let mut dialog = dialog();
            dialog.input(&told(device));
            assert_eq!(
                asked(&dialog),
                Some(EntryAsk {
                    field: rect(&dialog, NAME_ITEM),
                    focused: 1,
                }),
                "{device:?}"
            );
            assert!(!keyboard_drawn(&dialog), "{device:?}");
        }
    }

    #[test]
    fn tab_moves_the_ask_to_the_nickname_and_counts_a_focus() {
        let mut dialog = dialog();
        dialog.input(&Input::Key {
            key: Key::Tab,
            pressed: true,
            repeat: false,
        });
        assert_eq!(
            asked(&dialog),
            Some(EntryAsk {
                field: rect(&dialog, NICKNAME_ITEM),
                focused: 2,
            })
        );
        let at = rect(&dialog, NAME_ITEM).center();
        crate::touch::testkit::tap(&mut dialog, at);
        assert_eq!(
            asked(&dialog),
            Some(EntryAsk {
                field: rect(&dialog, NAME_ITEM),
                focused: 3,
            }),
            "a click on the name"
        );
    }

    /// AC 2: from pad inputs alone, a pilot is named on the on-screen
    /// keyboard and created. Under the pad the dialog opens asking for
    /// nothing (its at-open ask is the keyboard's,
    /// `opening_asks_for_entry_at_the_name_field`).
    #[test]
    fn the_pad_alone_names_a_pilot_and_creates_it() {
        let mut dialog = dialog();
        dialog.input(&told(PAD));
        assert_eq!(asked(&dialog), None, "under the pad it asks nothing");
        assert!(!keyboard_drawn(&dialog), "nor shows the keyboard");
        for _ in 0..8 {
            if dialog.dialog().pad_highlight() == Some(NAME_ITEM) {
                break;
            }
            tap(&mut dialog, PadButton::LeftShoulder);
        }
        tap(&mut dialog, PadButton::South);
        assert!(keyboard_drawn(&dialog), "South on the field raises it");
        pad_types(&mut dialog, &ZOE);
        assert_eq!(dialog.name(), "Zoe");
        tap(&mut dialog, PadButton::RightShoulder);
        assert!(keyboard_drawn(&dialog), "Tab keeps it up");
        pad_types(
            &mut dialog,
            &[KeyId::Char('z'), KeyId::Char('o'), KeyId::Done],
        );
        assert!(!keyboard_drawn(&dialog), "Done lowers it");
        assert_eq!(dialog.take_outcome(), Some(created("Zoe", "Zo")));
    }

    /// Types a name over the limit and presses Return: OK refuses it,
    /// its first characters selected.
    fn refuse_a_long_name(dialog: &mut NewPilotDialog) {
        for _ in 0..30 {
            dialog.input(&Input::Text('x'));
        }
        for pressed in [true, false] {
            dialog.input(&Input::Key {
                key: Key::Enter,
                pressed,
                repeat: false,
            });
        }
        assert_eq!(dialog.take_outcome(), None, "refused");
        assert!(dialog.field.selected(), "its first characters selected");
    }

    /// Clear empties the field even after OK refused a name and selected
    /// its first characters, by a tap and by the pad.
    #[test]
    fn clear_empties_a_refused_name() {
        let mut tapped = dialog();
        tapped.input(&touch(false));
        refuse_a_long_name(&mut tapped);
        let at = tapped
            .keyboard()
            .layout(tapped.dialog_bounds())
            .key(KeyId::Clear)
            .expect("Clear")
            .center();
        crate::touch::testkit::tap(&mut tapped, at);
        assert_eq!(tapped.name(), "", "by a tap");
        let mut padded = dialog();
        padded.input(&told(PAD));
        refuse_a_long_name(&mut padded);
        for _ in 0..8 {
            if padded.dialog().pad_highlight() == Some(NAME_ITEM) {
                break;
            }
            tap(&mut padded, PadButton::LeftShoulder);
        }
        tap(&mut padded, PadButton::South);
        pad_types(&mut padded, &[KeyId::Clear]);
        assert_eq!(padded.name(), "", "by the pad");
    }

    /// Clear empties only the focused field, and emptying the name
    /// clears a refusal as typing does.
    #[test]
    fn clear_empties_the_focused_field_and_ends_a_refusal_of_the_name() {
        let mut dialog = dialog();
        dialog.input(&touch(false));
        let clear = |dialog: &mut NewPilotDialog| {
            let at = dialog
                .keyboard()
                .layout(dialog.dialog_bounds())
                .key(KeyId::Clear)
                .expect("Clear")
                .center();
            crate::touch::testkit::tap(dialog, at);
        };
        dialog.refuse("taken");
        dialog.input(&Input::Key {
            key: Key::Tab,
            pressed: true,
            repeat: false,
        });
        clear(&mut dialog);
        assert_eq!((dialog.name(), dialog.nickname()), ("Ada Lovelace", ""));
        assert_eq!(dialog.refusal(), Some("taken"), "the nickname's emptied");
        dialog.input(&Input::Key {
            key: Key::Tab,
            pressed: true,
            repeat: false,
        });
        clear(&mut dialog);
        assert_eq!(dialog.name(), "");
        assert_eq!(dialog.refusal(), None);
    }

    #[test]
    fn east_lowers_the_keyboard_and_the_dialog_stays() {
        let mut dialog = dialog();
        dialog.input(&told(PAD));
        for _ in 0..8 {
            if dialog.dialog().pad_highlight() == Some(NAME_ITEM) {
                break;
            }
            tap(&mut dialog, PadButton::LeftShoulder);
        }
        tap(&mut dialog, PadButton::South);
        tap(&mut dialog, PadButton::East);
        assert!(!keyboard_drawn(&dialog));
        assert_eq!(dialog.take_outcome(), None);
        assert_eq!(
            dialog.dialog().pad_highlight(),
            Some(NAME_ITEM),
            "back on the field"
        );
    }

    fn touch(soft_keyboard: bool) -> Input {
        Input::Surroundings(Surroundings {
            soft_keyboard,
            ..Surroundings::of(Device::Touch)
        })
    }

    /// AC 4: with fake touches and no soft keyboard, the on-screen
    /// keyboard names a pilot by taps.
    #[test]
    fn the_on_screen_keyboard_names_a_pilot_by_taps() {
        let mut dialog = dialog();
        dialog.input(&touch(false));
        assert!(keyboard_drawn(&dialog), "at once");
        assert_eq!(asked(&dialog), None);
        let layout = dialog.keyboard().layout(dialog.dialog_bounds());
        let mut fingers = Fingers::on(&mut dialog);
        for key in ZOE.into_iter().chain([KeyId::Done]) {
            let at = layout.key(key).expect("a key").center();
            fingers.down(1, at);
            fingers.up(1, at);
        }
        assert_eq!(dialog.take_outcome(), Some(created("Zoe", "Ada")));
    }

    /// AC 3: on touch, with a soft keyboard, the dialog asks for the
    /// system's entry at the focused field and draws no keyboard of its
    /// own; a tap on the field asks afresh, so the soft keyboard comes
    /// back after the player hid it.
    #[test]
    fn with_a_soft_keyboard_a_touch_asks_for_entry_and_draws_no_keyboard() {
        let mut dialog = dialog();
        dialog.input(&touch(true));
        assert!(!keyboard_drawn(&dialog));
        assert_eq!(
            asked(&dialog),
            Some(EntryAsk {
                field: rect(&dialog, NAME_ITEM),
                focused: 1,
            })
        );
        let at = rect(&dialog, NICKNAME_ITEM).center();
        crate::touch::testkit::tap(&mut dialog, at);
        assert_eq!(
            asked(&dialog),
            Some(EntryAsk {
                field: rect(&dialog, NICKNAME_ITEM),
                focused: 2,
            })
        );
        assert!(!keyboard_drawn(&dialog));
    }

    #[test]
    fn covered_it_asks_nothing_and_draws_no_keyboard() {
        let mut dialog = dialog();
        dialog.set_covered(true);
        assert_eq!(asked(&dialog), None);
        let mut touched = self::dialog();
        touched.input(&touch(false));
        touched.set_covered(true);
        assert!(!keyboard_drawn(&touched));
        touched.set_covered(false);
        assert!(keyboard_drawn(&touched));
    }

    #[test]
    fn the_built_in_dialog_asks_at_its_own_name_field() {
        let dialog = NewPilotDialog::fallback(ButtonStyle::STOCK, Rc::new(MonoMetrics));
        assert_eq!(
            asked(&dialog),
            Some(EntryAsk {
                field: rect(&dialog, NAME_ITEM),
                focused: 1,
            })
        );
    }
}
