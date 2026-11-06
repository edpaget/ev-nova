//! Text entry in a dialog (rdm:phase/key-bindings/phase-17-text-entry-port):
//! the game's own on-screen keyboard, and the ask for the system's entry.
//!
//! A text-entry host (the edit dialog, `EditDialog`, and New Pilot,
//! `NewPilotDialog`) keeps an [`EntryKeyboard`] and does one of two
//! things each frame while a field of it has the focus and nothing its
//! holder shows covers it:
//!
//! - **asks** for the system's entry, by the field's rect in its frame
//!   ([`EntryAsk`](crate::text_entry::EntryAsk), a field of the
//!   [`DrawList`]), when the system enters the text
//!   ([`EntryMode::System`](crate::text_entry::EntryMode): the keyboard
//!   and the mouse, touch with a soft keyboard); `nova`'s app turns the
//!   IME and the soft keyboard on for it;
//! - or **draws the keyboard** ([`keyboard`]), when the game enters it (a
//!   pad, once South pressed the field; touch without a soft keyboard,
//!   at once). Its keys give back what real keys give, run through the
//!   host's own input, so the fields don't change.
//!
//! It isn't a touch aid (`ui::touch_aids`): it shows for the pad as well
//! as for touch. It reuses their parts: a key acts on the release inside
//! it (`touch_aids::press`), is sized in points inside the safe area
//! (`touch_aids::place`'s minimum and margin) and drawn as a touch control
//! is (`touch_aids::look`); the pad's focus among the keys is phase 8's
//! (`ui::pad_focus::FocusState`).
//!
//! # Where additions go
//!
//! - A new key is an entry of a row in [`layout`], and what it gives back
//!   is [`keyboard`]'s.
//! - A new host implements [`EntryHost`] and calls the four hooks from
//!   its [`Screen`] methods, one line each: [`intercept`] at the head of
//!   `input`, [`cancel`] in `cancel_pointer`, [`cover`] in `set_covered`
//!   and [`draw`] at the end of `draw`. A field that takes the focus
//!   (opening, Tab) calls [`EntryKeyboard::focus_field`], and a click, a
//!   tap or South on a field [`EntryKeyboard::field_pressed`].
//! - Whoever holds a host tells it whether it is covered
//!   ([`Screen::set_covered`]), as for a touch aid, so a covered dialog
//!   neither asks nor draws a keyboard.
//! - Each host pins its own rows of the acceptance criteria: pad-only
//!   entry, and the ask as it opens, in its own tests.

pub mod keyboard;
pub mod layout;

pub use keyboard::EntryKeyboard;
pub use layout::{KeyId, Layout};

use crate::draw::DrawList;
use crate::geometry::Bounds;
use crate::input::Input;
use crate::screen::{Screen, ScreenAction};
use crate::text_entry::EntryAsk;
use crate::ui::text_field::TextField;

/// A screen with text fields that hosts an [`EntryKeyboard`].
pub trait EntryHost: Screen {
    /// Its keyboard.
    fn keyboard(&self) -> &EntryKeyboard;

    /// Its keyboard, to change.
    fn keyboard_mut(&mut self) -> &mut EntryKeyboard;

    /// The field with the focus, if one has it: entry is active.
    fn entry_field(&self) -> Option<&TextField>;

    /// Its dialog's bounds, which the keyboard keeps clear of.
    fn dialog_bounds(&self) -> Bounds;

    /// The control the pad's focus is on (its dialog's item), if the
    /// highlight shows. The pad's raise lasts while it stays on the field
    /// South raised it on: a shoulder that moves the pad's focus off it
    /// lowers the keyboard, so South on another control presses that
    /// control, as with no keyboard up. Tab, which moves the text focus
    /// and not the pad's, keeps it up.
    fn pad_target(&self) -> Option<usize>;
}

/// The keyboard's layout while it shows over `host`, if it shows.
#[must_use]
pub fn shown(host: &impl EntryHost) -> Option<Layout> {
    let keyboard = host.keyboard();
    let pad = matches!(keyboard.told().device, crate::input::Device::Gamepad(_));
    (keyboard.shows(host.entry_field().is_some()) && (!pad || stays_raised(host)))
        .then(|| keyboard.layout(host.dialog_bounds()))
}

/// Whether the pad's focus is still where the pad raised the keyboard.
fn stays_raised(host: &impl EntryHost) -> bool {
    host.keyboard().raised_on() == host.pad_target()
}

/// Runs `input` through `host`'s keyboard at the head of the host's
/// [`Screen::input`]: `None` when the host takes `input` as it is, or
/// else the action of the inputs the keyboard gave back, each run through
/// the host's input past the keyboard (the last that isn't
/// [`ScreenAction::None`]).
pub fn intercept(host: &mut impl EntryHost, input: &Input) -> Option<ScreenAction> {
    if host.keyboard().replaying() {
        return None;
    }
    if !stays_raised(host) {
        host.keyboard_mut().lower();
    }
    let layout = shown(host);
    let chars = host
        .entry_field()
        .map_or(0, |field| field.text().chars().count());
    let given = host.keyboard_mut().input(input, layout.as_ref(), chars);
    if given == [*input] {
        return None;
    }
    host.keyboard_mut().set_replaying(true);
    let mut action = ScreenAction::None;
    for each in &given {
        let got = host.input(each);
        if got != ScreenAction::None {
            action = got;
        }
    }
    host.keyboard_mut().set_replaying(false);
    Some(action)
}

/// In the host's pointer cancel: a press on a key is forgotten.
pub fn cancel(host: &mut impl EntryHost) {
    host.keyboard_mut().cancel();
}

/// In the host's [`Screen::set_covered`]: covered, the keyboard hides
/// and lets go of a press, and the host asks nothing.
pub fn cover(host: &mut impl EntryHost, covered: bool) {
    host.keyboard_mut().set_covered(covered);
}

/// At the end of the host's draw: the keyboard while it shows, or else
/// the ask for the system's entry while the host asks.
pub fn draw(host: &impl EntryHost, list: &mut DrawList) {
    let keyboard = host.keyboard();
    if let Some(layout) = shown(host) {
        keyboard.draw(&layout, list);
    } else if let Some(field) = host.entry_field()
        && keyboard.asks(true)
    {
        list.ask_text_entry(EntryAsk {
            field: field.rect(),
            focused: keyboard.focused(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;
    use crate::input::{Device, Key, MouseButton, PadButton, Surroundings};
    use crate::ui::pad_focus::every_screen::{PAD, pad, told};

    /// A host that records what reaches it, with one field it types into,
    /// and that quits on Return.
    struct Recorder {
        keyboard: EntryKeyboard,
        field: Option<TextField>,
        got: Vec<Input>,
        /// Whether the pad's focus is on the field.
        on_field: bool,
    }

    /// The field's item.
    const FIELD: usize = 3;

    const DIALOG: Bounds = Bounds {
        min: Point::new(350.0, 280.0),
        max: Point::new(676.0, 490.0),
    };

    impl EntryHost for Recorder {
        fn keyboard(&self) -> &EntryKeyboard {
            &self.keyboard
        }

        fn keyboard_mut(&mut self) -> &mut EntryKeyboard {
            &mut self.keyboard
        }

        fn entry_field(&self) -> Option<&TextField> {
            self.field.as_ref()
        }

        fn dialog_bounds(&self) -> Bounds {
            DIALOG
        }

        fn pad_target(&self) -> Option<usize> {
            self.on_field.then_some(FIELD)
        }
    }

    impl Screen for Recorder {
        fn input(&mut self, input: &Input) -> ScreenAction {
            if let Some(action) = intercept(self, input) {
                return action;
            }
            self.got.push(*input);
            if let Some(field) = &mut self.field {
                field.input(input);
            }
            match *input {
                Input::Key {
                    key: Key::Enter,
                    pressed: true,
                    ..
                } => ScreenAction::Quit,
                _ => ScreenAction::None,
            }
        }

        fn tick(&mut self, _dt: std::time::Duration) {}

        fn draw(&self, list: &mut DrawList) {
            draw(self, list);
        }

        fn cancel_pointer(&mut self) {
            cancel(self);
        }

        fn set_covered(&mut self, covered: bool) {
            cover(self, covered);
        }
    }

    fn host(device: Device) -> Recorder {
        let mut host = Recorder {
            keyboard: EntryKeyboard::new(),
            field: Some(TextField::with_text(
                Bounds::at(Point::new(400.0, 300.0), 120.0, 16.0),
                usize::MAX,
                "abc",
            )),
            got: Vec::new(),
            on_field: true,
        };
        host.keyboard.focus_field();
        host.input(&told(device));
        host.got.clear();
        host
    }

    fn click(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn key_at(host: &Recorder, key: KeyId) -> Point {
        host.keyboard
            .layout(DIALOG)
            .key(key)
            .expect("a key")
            .center()
    }

    fn drawn(host: &Recorder) -> DrawList {
        let mut list = DrawList::new();
        host.draw(&mut list);
        list
    }

    #[test]
    fn inputs_it_gives_back_run_through_the_host_past_the_keyboard() {
        let mut host = host(Device::Touch);
        for key in [KeyId::Clear, KeyId::Char('x')] {
            let at = key_at(&host, key);
            host.input(&click(true, at));
            host.input(&click(false, at));
        }
        assert_eq!(host.field.as_ref().expect("a field").text(), "X");
        let at = key_at(&host, KeyId::Done);
        host.input(&click(true, at));
        assert_eq!(
            host.input(&click(false, at)),
            ScreenAction::Quit,
            "Done is Return"
        );
        let backspace = |pressed| Input::Key {
            key: Key::Backspace,
            pressed,
            repeat: false,
        };
        assert_eq!(
            host.got[..2],
            [backspace(true), backspace(false)],
            "the keys the field reads, not the taps"
        );
        assert!(
            !host
                .got
                .iter()
                .any(|input| matches!(input, Input::PointerButton { .. }))
        );
    }

    #[test]
    fn the_host_asks_for_text_entry_in_system_mode_with_its_field_and_count() {
        let mut host = host(Device::Keyboard);
        let field = host.field.as_ref().expect("a field").rect();
        assert_eq!(
            drawn(&host).text_entry(),
            Some(EntryAsk { field, focused: 1 })
        );
        assert!(drawn(&host).is_empty(), "no keyboard drawn");
        host.keyboard.field_pressed(Some(FIELD));
        assert_eq!(
            drawn(&host).text_entry(),
            Some(EntryAsk { field, focused: 2 })
        );
        let mut soft = self::host(Device::Touch);
        soft.input(&Input::Surroundings(Surroundings {
            soft_keyboard: true,
            ..Surroundings::of(Device::Touch)
        }));
        assert_eq!(
            drawn(&soft).text_entry(),
            Some(EntryAsk { field, focused: 1 })
        );
    }

    #[test]
    fn the_host_asks_nothing_in_on_screen_mode_or_covered_or_with_no_field() {
        let touch = host(Device::Touch);
        assert_eq!(drawn(&touch).text_entry(), None);
        assert!(!drawn(&touch).is_empty(), "the keyboard instead");
        let mut padded = host(PAD);
        assert_eq!(drawn(&padded).text_entry(), None);
        assert!(drawn(&padded).is_empty(), "nor a keyboard until raised");
        padded.keyboard.field_pressed(Some(FIELD));
        assert!(!drawn(&padded).is_empty(), "raised");
        let mut covered = host(Device::Keyboard);
        covered.set_covered(true);
        assert_eq!(drawn(&covered).text_entry(), None);
        let mut covered = host(Device::Touch);
        covered.set_covered(true);
        assert!(drawn(&covered).is_empty(), "no keyboard covered");
        covered.set_covered(false);
        assert!(!drawn(&covered).is_empty());
        let mut fieldless = host(Device::Keyboard);
        fieldless.field = None;
        assert_eq!(drawn(&fieldless).text_entry(), None);
        let mut fieldless = host(Device::Touch);
        fieldless.field = None;
        assert!(drawn(&fieldless).is_empty());
    }

    #[test]
    fn cancel_forgets_a_press() {
        let mut host = host(Device::Touch);
        let at = key_at(&host, KeyId::Char('q'));
        host.input(&click(true, at));
        host.cancel_pointer();
        host.input(&click(false, at));
        assert_eq!(host.field.as_ref().expect("a field").text(), "abc");
        assert_eq!(host.got, [click(false, at)], "the release, the host's");
    }

    #[test]
    fn the_pads_focus_leaving_the_field_lowers_it() {
        let mut host = host(PAD);
        host.keyboard.field_pressed(Some(FIELD));
        assert!(shown(&host).is_some());
        host.on_field = false;
        assert!(shown(&host).is_none(), "hidden at once");
        host.on_field = true;
        assert!(shown(&host).is_some(), "until an input lowers it");
        host.on_field = false;
        host.input(&pad(PadButton::South, true));
        host.on_field = true;
        assert!(shown(&host).is_none(), "lowered: South raises it again");
        assert_eq!(
            host.got,
            [pad(PadButton::South, true)],
            "South was the host's"
        );
        let mut touched = self::host(Device::Touch);
        touched.on_field = false;
        touched.input(&Input::PointerMoved(Point::new(1.0, 1.0)));
        assert!(shown(&touched).is_some(), "touch has no pad focus to leave");
    }

    #[test]
    fn what_it_does_not_take_reaches_the_host_once() {
        let mut host = host(PAD);
        host.keyboard.field_pressed(Some(FIELD));
        host.input(&pad(PadButton::RightShoulder, true));
        host.input(&Input::Text('z'));
        assert_eq!(
            host.got,
            [pad(PadButton::RightShoulder, true), Input::Text('z')]
        );
    }
}
