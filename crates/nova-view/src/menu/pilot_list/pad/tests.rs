use std::rc::Rc;

use super::*;
use crate::input::{Device, Input, Key, PadButton};
use crate::screen::Screen;
use crate::text::fixture::MonoMetrics;
use crate::ui::ButtonStyle;
use crate::ui::pad_focus::every_screen::{PAD, PadProbe, pad, pointer, told};

fn list(count: usize) -> PilotList {
    PilotList::new(
        (0..count).map(|n| format!("Pilot {n}")).collect(),
        ButtonStyle::STOCK,
        Rc::new(MonoMetrics),
    )
}

fn padded(count: usize) -> PilotList {
    let mut list = list(count);
    list.input(&told(PAD));
    list
}

fn tap(list: &mut PilotList, button: PadButton) {
    list.input(&pad(button, true));
    list.input(&pad(button, false));
}

fn opened(n: usize) -> PilotListOutcome {
    PilotListOutcome::Open(format!("Pilot {n}"))
}

#[test]
fn told_the_pad_the_focus_starts_on_the_selected_row_or_on_cancel_when_empty() {
    assert_eq!(padded(3).pad_highlight(), Some(PadTarget::Row(0)));
    assert_eq!(padded(0).pad_highlight(), Some(PadTarget::Cancel));
    assert_eq!(list(3).pad_highlight(), None, "untold");
    assert_eq!(padded(3).device(), PAD);
}

#[test]
fn the_dpad_reaches_each_shown_row_cancel_and_open() {
    let mut list = padded(3);
    let mut reached = vec![list.pad_highlight()];
    for button in [
        PadButton::DPadDown,
        PadButton::DPadDown,
        PadButton::DPadDown,
        PadButton::DPadRight,
    ] {
        tap(&mut list, button);
        reached.push(list.pad_highlight());
    }
    assert_eq!(
        reached,
        [
            Some(PadTarget::Row(0)),
            Some(PadTarget::Row(1)),
            Some(PadTarget::Row(2)),
            Some(PadTarget::Cancel),
            Some(PadTarget::Open),
        ]
    );
    tap(&mut list, PadButton::RightShoulder);
    assert_eq!(
        list.pad_highlight(),
        Some(PadTarget::Row(0)),
        "Tab order wraps"
    );
}

#[test]
fn an_empty_lists_open_is_no_target() {
    let mut list = padded(0);
    tap(&mut list, PadButton::DPadRight);
    assert_eq!(list.pad_highlight(), Some(PadTarget::Cancel));
}

#[test]
fn south_on_a_row_selects_and_opens_it_as_a_click_does() {
    let mut list = padded(3);
    tap(&mut list, PadButton::DPadDown);
    tap(&mut list, PadButton::DPadDown);
    tap(&mut list, PadButton::South);
    assert_eq!(list.selected(), Some(2));
    assert_eq!(list.take_outcome(), Some(opened(2)));
}

#[test]
fn south_on_open_opens_the_selection_and_on_cancel_cancels() {
    let mut list = padded(3);
    tap(&mut list, PadButton::LeftShoulder);
    assert_eq!(list.pad_highlight(), Some(PadTarget::Open));
    tap(&mut list, PadButton::South);
    assert_eq!(list.take_outcome(), Some(opened(0)));
    tap(&mut list, PadButton::LeftShoulder);
    tap(&mut list, PadButton::South);
    assert_eq!(list.take_outcome(), Some(PilotListOutcome::Cancel));
}

#[test]
fn south_is_silent_and_acts_on_its_release() {
    let mut list = padded(3);
    list.input(&pad(PadButton::South, true));
    assert_eq!(list.take_outcome(), None, "not on the press");
    assert_eq!(list.take_sounds(), []);
    list.input(&pad(PadButton::South, false));
    assert_eq!(list.take_sounds(), []);
    assert_eq!(list.take_outcome(), Some(opened(0)));
}

#[test]
fn a_focused_row_keeps_its_slot_while_the_left_stick_scrolls() {
    let mut list = padded(14);
    for _ in 0..9 {
        tap(&mut list, PadButton::DPadDown);
    }
    assert_eq!(list.pad_highlight(), Some(PadTarget::Row(9)));
    for _ in 0..12 {
        tap(&mut list, PadButton::LeftStickDown);
    }
    assert_eq!(list.selected(), Some(12), "the stick moved the selection");
    assert_eq!(list.pad_highlight(), Some(PadTarget::Row(9)), "on its slot");
    tap(&mut list, PadButton::South);
    assert_eq!(
        list.take_outcome(),
        Some(opened(12)),
        "the slot's pilot now"
    );
}

#[test]
fn a_move_drops_a_held_south_silently() {
    let mut list = padded(3);
    list.input(&pad(PadButton::South, true));
    tap(&mut list, PadButton::DPadDown);
    list.input(&pad(PadButton::South, false));
    assert_eq!(list.take_outcome(), None);
    assert_eq!(list.take_sounds(), []);
}

#[test]
fn a_release_the_list_never_saw_pressed_does_nothing() {
    let mut list = padded(3);
    list.input(&pad(PadButton::South, false));
    assert_eq!(list.take_outcome(), None);
    let open = list.open_button().rect.center();
    list.input(&pointer(true, open));
    list.input(&pad(PadButton::South, false));
    assert_eq!(list.take_outcome(), None, "the pointer's press is its own");
}

#[test]
fn keyboard_space_still_does_nothing() {
    let mut list = padded(3);
    for pressed in [true, false] {
        list.input(&Input::Key {
            key: Key::Space,
            pressed,
            repeat: false,
        });
    }
    assert_eq!(list.take_outcome(), None);
    assert_eq!(list.pad_highlight(), Some(PadTarget::Row(0)));
}

#[test]
fn east_cancels_as_escape_does() {
    let mut list = padded(3);
    tap(&mut list, PadButton::East);
    assert_eq!(list.take_outcome(), Some(PilotListOutcome::Cancel));
}

#[test]
fn the_highlight_leaves_with_the_pad() {
    let mut list = padded(3);
    list.input(&told(Device::Mouse));
    assert_eq!(list.pad_highlight(), None);
}

/// The pilot list under the pad's probe.
struct Probe(PilotList);

impl PadProbe for Probe {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.0
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        let mut controls: Vec<_> = ["Pilot 0", "Pilot 1", "Pilot 2"]
            .into_iter()
            .zip(0..)
            .filter_map(|(name, index)| self.0.row(index).map(|rect| (name, rect)))
            .collect();
        controls.push(("Cancel", self.0.cancel_button().rect));
        if self.0.open_button().enabled {
            controls.push(("Open", self.0.open_button().rect));
        }
        controls
    }

    fn outcome(&mut self) -> String {
        let outcome = self.0.take_outcome();
        format!("{:?} {outcome:?}", self.0)
    }

    fn default_control(&self) -> Option<&'static str> {
        self.0.open_button().enabled.then_some("Open")
    }
}

/// Three saved pilots.
pub(crate) fn pad_probe() -> Box<dyn PadProbe> {
    Box::new(Probe(list(3)))
}

/// No saved pilot: Cancel alone.
pub(crate) fn empty_pad_probe() -> Box<dyn PadProbe> {
    Box::new(Probe(list(0)))
}

#[test]
fn told_the_pad_after_a_scroll_the_focus_starts_on_the_selected_rows_slot() {
    let mut list = list(14);
    for _ in 0..12 {
        tap(&mut list, PadButton::LeftStickDown);
    }
    assert_eq!(list.selected(), Some(12));
    list.input(&told(PAD));
    assert_eq!(
        list.pad_highlight(),
        Some(PadTarget::Row(9)),
        "pilot 12, scrolled by 3"
    );
    tap(&mut list, PadButton::South);
    assert_eq!(list.take_outcome(), Some(opened(12)));
}
