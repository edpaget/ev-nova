//! Keys that wait for a held press's release, in the dialogs that ask for
//! it ([`Dialog::with_held_keys`](super::Dialog::with_held_keys)): "Desc
//! Dialog" and the offer dialog.
//!
//! The original reads no events while the mouse button is held in those
//! dialogs. `_TextDialogFilter`'s and `_SingleMissionFilter`'s held loops
//! on an arrow (@0x52172-0x5237f, @0x99060-0x991e6) and their trackers for
//! a click on a button, `_TrackTextButtons` @0x26bb0 and
//! `_TrackSingleMissionButtons` @0x28188, loop on `StillDown` alone. So a
//! key pressed meanwhile waits in the event queue and reaches the filter
//! after the release.
//!
//! The rules:
//!
//! - While a left press is held on an arrow, or a click is tracked on any
//!   of the dialog's buttons or arrows, the dialog queues each key input
//!   and each typed character, in the order they come: presses, repeats
//!   and releases. A press off every item holds nothing, as the trackers
//!   return at once there.
//! - The wheel stays live: it is the engine's own addition, and the
//!   original has none. Pointer events keep the hold's rules
//!   ([`Dialog::input`](super::Dialog::input)).
//! - The release that ends the press turns the queue into the replay
//!   ([`Dialog::take_replay`](super::Dialog::take_replay)). A tracked
//!   click's release acts first, so its item comes before the replay. A
//!   left press during the press (a release the engine never saw) ends it
//!   without acting, and the replay is the queue followed by that press.
//! - A press abandoned through
//!   [`Dialog::cancel_pointer`](super::Dialog::cancel_pointer) keeps its
//!   queue for the next [`Dialog::tick`](super::Dialog::tick), which makes
//!   it the replay. The router cancels a press released outside the
//!   content area. The replay settles before the frame is drawn, so no
//!   boundary the player sees is crossed.
//!
//! The host runs the replay through its own input path, in order, within
//! the same input call or tick: its own keys too, which it skips while
//! the dialog [`holds`](super::Dialog::holds) (the map, player info and
//! mission info keys, the labels' letters, the page keys). A replay holds
//! only keys and that one press, and a key can't start a hold, so a replay
//! never nests.
//!
//! The menu chords wait too. The original's Command-Q and Command-comma
//! are key-downs that wait in the queue with the rest, until
//! `_MyModalDialog`'s `WaitNextEvent` (@0x5999a) hands them to the menu.
//! The router (`nova::app`) takes them before the dialog sees them, so it
//! holds its own chords while the screen in front reads
//! [`Screen::holds_back_keys`](crate::Screen::holds_back_keys) true: from
//! the press until the replay of what it held back has run
//! ([`Dialog::holds_back_keys`](super::Dialog::holds_back_keys)). Then it
//! acts on them, and what was typed after them, in order.
//!
//! Its rest after a key that closes the dialog or opens a screen is the
//! host's to place, as the original's queued events reach whatever reads
//! events next. The map, player info and mission info each flush the queue
//! as they open (`FlushEvents(0xff7f, 0)`: `_DoSystemMap` @0x1257b,
//! `_DoPlayerInfoDialog` @0x5a544, `_DoMissionInfoDialog` @0x9e287), and
//! so does a text dialog (@0x5ae67). Closing a text dialog flushes
//! nothing. The offer's answers call `FlushEvents` with event codes where
//! it takes masks (@0xa2da2-0xa2dde and the like), which flushes only
//! mouse events. So the rest after a close goes on to what shows next,
//! unless that is a text dialog.

use crate::input::Input;

/// A dialog's keys held back while a press is held, and what they become.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct HeldKeys {
    /// Keys and characters held back from the press in progress.
    queued: Vec<Input>,
    /// What an abandoned press held back, until the next tick.
    pending: Vec<Input>,
    /// What the host is to run through its input path, until taken.
    replay: Vec<Input>,
}

impl HeldKeys {
    /// Whether `input` waits while a press is held: a key's press, repeat
    /// or release, or a typed character.
    pub(super) const fn waits(input: &Input) -> bool {
        matches!(input, Input::Key { .. } | Input::Text(_))
    }

    /// Holds `input` back until the press is over.
    pub(super) fn queue(&mut self, input: Input) {
        self.queued.push(input);
    }

    /// The press is over: what it held back, then `then` if any, is the
    /// replay.
    pub(super) fn release(&mut self, then: Option<Input>) {
        self.replay.append(&mut self.queued);
        self.replay.extend(then);
    }

    /// The press was abandoned: what it held back waits for the next tick.
    pub(super) fn abandon(&mut self) {
        self.pending.append(&mut self.queued);
    }

    /// A tick: what an abandoned press held back is the replay.
    pub(super) fn tick(&mut self) {
        self.replay.append(&mut self.pending);
    }

    /// Whether some held-back keys are still to replay: an abandoned press's,
    /// until the tick, or a replay not yet taken.
    pub(super) fn waiting(&self) -> bool {
        !(self.pending.is_empty() && self.replay.is_empty())
    }

    /// The replay, once.
    pub(super) fn take_replay(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.replay)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::color::Color;
    use crate::font::Font;
    use crate::geometry::{Bounds, Point};
    use crate::input::{Key, MouseButton};
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{
        Dialog, DialogEvent, DialogTemplate, ItemSpec, ItemTemplate, Placement, Role,
    };

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    fn left(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    /// Okay (1) and a text box (3), its keys waiting for a held press.
    fn dialog() -> Dialog {
        let item = |bounds, enabled| ItemTemplate {
            bounds,
            enabled,
            kind: ItemSpec::User,
        };
        let template = DialogTemplate {
            bounds: rect(0.0, 0.0, 200.0, 120.0),
            placement: Placement::Fixed,
            items: vec![
                item(rect(10.0, 90.0, 60.0, 25.0), true),
                item(rect(10.0, 400.0, 20.0, 20.0), false),
                item(rect(10.0, 10.0, 60.0, 36.0), false),
            ],
        };
        let roles = [
            (1, Role::Button("Okay".into())),
            (
                3,
                Role::ScrollText {
                    text: "l0".into(),
                    font: Font::Geneva,
                    size: 10.0,
                    color: Color::WHITE,
                    backing: None,
                },
            ),
        ];
        Dialog::new(&template, &roles, Rc::new(MonoMetrics)).with_held_keys()
    }

    fn okay(dialog: &Dialog) -> Point {
        dialog.item_bounds(1).expect("Okay").center()
    }

    #[test]
    fn keys_wait_during_a_tracked_click_on_a_button_and_replay_after_its_item() {
        let mut dialog = dialog();
        let at = okay(&dialog);
        dialog.input(&left(true, at));
        assert!(dialog.holds());
        assert_eq!(dialog.input(&key(Key::Enter, true)), None, "it waits");
        assert_eq!(dialog.input(&left(false, at)), Some(DialogEvent::Item(1)));
        assert!(!dialog.holds());
        assert_eq!(dialog.take_replay(), [key(Key::Enter, true)]);
    }

    #[test]
    fn a_click_released_off_its_button_still_replays_its_keys() {
        let mut dialog = dialog();
        dialog.input(&left(true, okay(&dialog)));
        dialog.input(&key(Key::Escape, true));
        assert_eq!(dialog.input(&left(false, Point::new(150.0, 10.0))), None);
        assert_eq!(dialog.take_replay(), [key(Key::Escape, true)]);
    }

    /// The trackers return at once when no button is under the press.
    #[test]
    fn a_press_off_every_button_holds_no_key() {
        let mut dialog = dialog();
        dialog.input(&left(true, Point::new(150.0, 10.0)));
        assert!(!dialog.holds());
        assert_eq!(
            dialog.input(&key(Key::Enter, true)),
            Some(DialogEvent::Item(1))
        );
    }

    #[test]
    fn a_second_left_press_during_a_click_ends_it_and_replays_after_the_keys() {
        let mut dialog = dialog();
        let at = okay(&dialog);
        dialog.input(&left(true, at));
        dialog.input(&Input::Text('q'));
        assert_eq!(dialog.input(&left(true, at)), None);
        assert!(!dialog.holds());
        assert_eq!(dialog.pressed_item(), None, "the click is over");
        assert_eq!(dialog.take_replay(), [Input::Text('q'), left(true, at)]);
    }

    #[test]
    fn a_cancelled_click_replays_its_keys_on_the_next_tick() {
        let mut dialog = dialog();
        dialog.input(&left(true, okay(&dialog)));
        dialog.input(&key(Key::Enter, false));
        dialog.cancel_pointer();
        assert_eq!(dialog.take_replay(), []);
        dialog.tick(std::time::Duration::ZERO);
        assert_eq!(dialog.take_replay(), [key(Key::Enter, false)]);
        assert_eq!(dialog.take_replay(), [], "once");
    }

    #[test]
    fn the_queue_releases_in_order_and_an_abandon_waits_for_a_tick() {
        let mut keys = HeldKeys::default();
        keys.queue(key(Key::Up, true));
        keys.queue(key(Key::Up, false));
        keys.release(Some(Input::Text('a')));
        assert_eq!(
            keys.take_replay(),
            [key(Key::Up, true), key(Key::Up, false), Input::Text('a')]
        );
        keys.queue(key(Key::End, true));
        keys.abandon();
        assert_eq!(keys.take_replay(), []);
        keys.tick();
        assert_eq!(keys.take_replay(), [key(Key::End, true)]);
    }

    #[test]
    fn a_click_holds_back_keys_from_its_press_until_its_replay_is_taken() {
        let mut dialog = dialog();
        let at = okay(&dialog);
        assert!(!dialog.holds_back_keys(), "before the press");
        dialog.input(&left(true, at));
        assert!(dialog.holds_back_keys(), "during the press");
        dialog.input(&key(Key::Escape, true));
        dialog.input(&left(false, at));
        assert!(dialog.holds_back_keys(), "until the replay is taken");
        dialog.take_replay();
        assert!(!dialog.holds_back_keys(), "after it");
    }

    #[test]
    fn a_click_that_held_nothing_holds_back_nothing_once_released() {
        let mut dialog = dialog();
        let at = okay(&dialog);
        dialog.input(&left(true, at));
        dialog.input(&left(false, at));
        assert!(!dialog.holds_back_keys());
    }

    #[test]
    fn a_cancelled_click_holds_back_keys_until_its_tick_and_the_replay() {
        let mut dialog = dialog();
        dialog.input(&left(true, okay(&dialog)));
        dialog.input(&key(Key::Enter, true));
        dialog.cancel_pointer();
        assert!(dialog.holds_back_keys(), "until the tick");
        dialog.tick(std::time::Duration::ZERO);
        assert!(dialog.holds_back_keys(), "until the replay is taken");
        dialog.take_replay();
        assert!(!dialog.holds_back_keys());
    }

    #[test]
    fn a_dialog_whose_keys_act_at_once_never_holds_them_back() {
        let mut dialog = Dialog::new(
            &DialogTemplate {
                bounds: rect(0.0, 0.0, 200.0, 120.0),
                placement: Placement::Fixed,
                items: vec![ItemTemplate {
                    bounds: rect(10.0, 90.0, 60.0, 25.0),
                    enabled: true,
                    kind: ItemSpec::User,
                }],
            },
            &[(1, Role::Button("Okay".into()))],
            Rc::new(MonoMetrics),
        );
        let at = dialog.item_bounds(1).expect("Okay").center();
        dialog.input(&left(true, at));
        assert!(!dialog.holds_back_keys());
        dialog.input(&key(Key::Escape, true));
        assert!(!dialog.holds_back_keys());
    }

    #[test]
    fn only_keys_and_characters_wait() {
        assert!(HeldKeys::waits(&key(Key::Enter, true)));
        assert!(HeldKeys::waits(&Input::Text('a')));
        assert!(!HeldKeys::waits(&Input::PointerLeft));
        assert!(!HeldKeys::waits(&Input::PointerMovedOutside(Point::new(
            -5.0, 0.0
        ))));
        assert!(!HeldKeys::waits(&left(true, Point::new(0.0, 0.0))));
    }
}
