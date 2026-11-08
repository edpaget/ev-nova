//! Text entry as data (rdm:phase/key-bindings/phase-17-text-entry-port):
//! what a screen with a focused text field asks of the platform, and who
//! does the entry for the device used last.
//!
//! - **[`EntryAsk`]** is a field of the frame a screen fills
//!   ([`DrawList::ask_text_entry`](crate::draw::DrawList::ask_text_entry)),
//!   not a [`Screen`](crate::screen::Screen) method: the field's rect and
//!   how many times a field took the focus. Its presence is "entry is
//!   active": `nova`'s app starts the platform's entry (IME, the soft
//!   keyboard) on a frame that asks, starts it again when the ask
//!   changes, and stops it on a frame that asks nothing.
//! - **[`EntryMode`]** says who enters the text: the system (the keys,
//!   the IME and a soft keyboard) or the game's on-screen keyboard
//!   (`ui::entry_keyboard`). A host asks only in [`EntryMode::System`]:
//!   under a pad the game's keyboard types, and the system's keyboard
//!   would pop up over it.
//!
//! The field's text isn't in the ask. The port would only pass it on as
//! winit's surrounding text, which of winit 0.31's backends only Wayland
//! reads, and which would oblige the game to handle the IME's
//! delete-surrounding requests.
//!
//! IME, the soft keyboard and the on-screen keyboard are new features, so
//! none of this has a `Rulebook` key.

use crate::geometry::Bounds;
use crate::input::Device;

/// A screen's ask for text entry, as its frame reports it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntryAsk {
    /// The focused field's rect, in the logical space of the shape shown.
    pub field: Bounds,
    /// How many times a field of the screen has taken the focus: the
    /// dialog opening, Tab, a click, a tap, the pad's South. The app
    /// starts entry again when it changes, so the soft keyboard comes back
    /// after the player hid it, which no event reports and which leaves
    /// the rect as it was.
    pub focused: u32,
}

impl EntryAsk {
    /// The ask moved by `by`, as a frame drawn moved by `by` reports it.
    #[must_use]
    pub fn offset(self, by: crate::geometry::Point) -> Self {
        Self {
            field: self.field.offset(by),
            ..self
        }
    }
}

/// Who enters the text for the device used last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryMode {
    /// The system: the keys and the IME, and the platform's own soft
    /// keyboard where it has one. A host asks for it ([`EntryAsk`]).
    System,
    /// The game's on-screen keyboard, worked by the pad or by taps.
    OnScreen,
}

impl EntryMode {
    /// Who enters the text for `device`, on a platform that has a soft
    /// keyboard or not: the on-screen keyboard for a pad, and for touch
    /// without a soft keyboard; the system otherwise.
    #[must_use]
    pub const fn of(device: Device, soft_keyboard: bool) -> Self {
        match device {
            Device::Gamepad(_) => Self::OnScreen,
            Device::Touch if !soft_keyboard => Self::OnScreen,
            Device::Touch | Device::Keyboard | Device::Mouse => Self::System,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;
    use crate::input::ControllerFamily;

    #[test]
    fn the_mode_is_on_screen_for_a_pad_and_for_touch_without_a_soft_keyboard() {
        let pad = Device::Gamepad(ControllerFamily::PlayStation);
        for (device, soft, mode) in [
            (Device::Keyboard, false, EntryMode::System),
            (Device::Keyboard, true, EntryMode::System),
            (Device::Mouse, false, EntryMode::System),
            (Device::Mouse, true, EntryMode::System),
            (Device::Touch, false, EntryMode::OnScreen),
            (Device::Touch, true, EntryMode::System),
            (pad, false, EntryMode::OnScreen),
            (pad, true, EntryMode::OnScreen),
        ] {
            assert_eq!(EntryMode::of(device, soft), mode, "{device:?} {soft}");
        }
    }

    #[test]
    fn an_ask_moves_its_field_and_keeps_its_count() {
        let ask = EntryAsk {
            field: Bounds::at(Point::new(10.0, 20.0), 30.0, 16.0),
            focused: 3,
        };
        assert_eq!(
            ask.offset(Point::new(-170.0, 2.0)),
            EntryAsk {
                field: Bounds::at(Point::new(-160.0, 22.0), 30.0, 16.0),
                focused: 3,
            }
        );
    }
}
