#![allow(clippy::float_cmp)]

use std::rc::Rc;

use super::super::{DialogTemplate, ItemSpec, ItemTemplate, Placement, Role};
use super::*;
use crate::color::Color;
use crate::draw::{DrawCommand, DrawList};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{ControllerFamily, Key, MouseButton, PadButton};
use crate::text::fixture::MonoMetrics;
use crate::ui::scroll_arrows::ScrollArrow;

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
    Bounds::at(Point::new(x, y), w, h)
}

fn item(bounds: Bounds, enabled: bool, kind: ItemSpec) -> ItemTemplate {
    ItemTemplate {
        bounds,
        enabled,
        kind,
    }
}

fn button(x: f32, y: f32, title: &str) -> ItemTemplate {
    item(rect(x, y, 70.0, 20.0), true, ItemSpec::Button(title.into()))
}

/// Three buttons in a row, right to left: OK (1, the default), Cancel
/// (2) and Other (3); then a static text (4) above them.
fn row() -> DialogTemplate {
    DialogTemplate {
        bounds: rect(0.0, 0.0, 300.0, 100.0),
        placement: Placement::Fixed,
        items: vec![
            button(220.0, 70.0, "OK"),
            button(140.0, 70.0, "Cancel"),
            button(60.0, 70.0, "Other"),
            item(
                rect(10.0, 10.0, 280.0, 40.0),
                false,
                ItemSpec::StaticText("Hi".into()),
            ),
        ],
    }
}

fn dialog(template: &DialogTemplate) -> Dialog {
    Dialog::new(template, &[], Rc::new(MonoMetrics)).with_cancel(Some(2))
}

fn pad(button: PadButton, pressed: bool) -> Input {
    Input::PadButton { button, pressed }
}

fn tap(dialog: &mut Dialog, button: PadButton) -> Option<DialogEvent> {
    let down = dialog.input(&pad(button, true));
    let up = dialog.input(&pad(button, false));
    down.or(up)
}

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

fn key(key: Key) -> Input {
    Input::Key {
        key,
        pressed: true,
        repeat: false,
    }
}

fn drawn(dialog: &Dialog) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    dialog.draw(&mut list);
    list.iter().cloned().collect()
}

/// The pad's highlight round `bounds`, as it draws.
fn highlight(bounds: Bounds) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    pad_focus::draw_highlight(&mut list, bounds, Color::WHITE);
    list.iter().cloned().collect()
}

fn ends_with_highlight(dialog: &Dialog, item: usize) -> bool {
    let commands = drawn(dialog);
    let ring = highlight(dialog.item_bounds(item).expect("item"));
    commands.ends_with(&ring)
}

#[test]
fn the_dpad_moves_the_focus_between_buttons_under_either_ring() {
    for ring in [FocusRing::Tab, FocusRing::Off] {
        let mut dialog = dialog(&row()).with_focus_ring(ring);
        dialog.input(&told(PAD));
        assert_eq!(dialog.focus(), Some(1), "{ring:?}");
        tap(&mut dialog, PadButton::DPadLeft);
        assert_eq!(dialog.focus(), Some(2), "{ring:?}");
        tap(&mut dialog, PadButton::DPadLeft);
        assert_eq!(dialog.focus(), Some(3), "{ring:?}");
        tap(&mut dialog, PadButton::DPadLeft);
        assert_eq!(dialog.focus(), Some(3), "{ring:?}: no wrap");
        tap(&mut dialog, PadButton::DPadUp);
        assert_eq!(dialog.focus(), Some(3), "{ring:?}: the text is no target");
        tap(&mut dialog, PadButton::DPadRight);
        assert_eq!(dialog.focus(), Some(2), "{ring:?}");
    }
}

#[test]
fn south_presses_the_focused_button_on_its_release_and_draws_it_pressed_meanwhile() {
    let mut dialog = dialog(&row());
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::DPadLeft);
    assert_eq!(dialog.input(&pad(PadButton::South, true)), None);
    assert_eq!(dialog.pressed_item(), Some(2), "drawn pressed while held");
    assert_eq!(
        dialog.input(&pad(PadButton::South, false)),
        Some(DialogEvent::Item(2))
    );
    assert_eq!(dialog.pressed_item(), None);
}

#[test]
fn before_the_pad_shows_its_focus_moves_and_south_do_nothing() {
    for ring in [FocusRing::Tab, FocusRing::Off] {
        let mut dialog = dialog(&row()).with_focus_ring(ring);
        for button in [
            PadButton::DPadLeft,
            PadButton::RightShoulder,
            PadButton::LeftShoulder,
        ] {
            assert_eq!(tap(&mut dialog, button), None, "{ring:?}: {button:?}");
            assert_eq!(dialog.focus(), None, "{ring:?}: {button:?}");
        }
        assert_eq!(tap(&mut dialog, PadButton::South), None, "{ring:?}");
        assert_eq!(dialog.pressed_item(), None, "{ring:?}");
        dialog.input(&told(PAD));
        tap(&mut dialog, PadButton::DPadLeft);
        assert_eq!(dialog.focus(), Some(2), "{ring:?}: works once told");
        assert_eq!(
            tap(&mut dialog, PadButton::South),
            Some(DialogEvent::Item(2)),
            "{ring:?}"
        );
    }
}

#[test]
fn before_the_pad_shows_south_presses_not_even_a_focus_the_keyboard_placed() {
    let mut dialog = dialog(&row()).with_focus_ring(FocusRing::Tab);
    dialog.input(&key(Key::Tab));
    let keyed = dialog.focus();
    assert!(keyed.is_some());
    assert_eq!(tap(&mut dialog, PadButton::South), None);
    assert_eq!(dialog.pressed_item(), None);
    assert_eq!(tap(&mut dialog, PadButton::DPadLeft), None);
    assert_eq!(dialog.focus(), keyed);
}

#[test]
fn before_the_pad_shows_east_still_does_what_escape_does() {
    let mut dialog = dialog(&row());
    assert_eq!(
        tap(&mut dialog, PadButton::East),
        Some(DialogEvent::Item(2))
    );
}

#[test]
fn east_activates_the_cancel_item_as_escape_does() {
    let mut keyed = dialog(&row());
    assert_eq!(keyed.input(&key(Key::Escape)), Some(DialogEvent::Item(2)));
    let mut padded = dialog(&row());
    padded.input(&told(PAD));
    assert_eq!(
        tap(&mut padded, PadButton::East),
        Some(DialogEvent::Item(2))
    );
    let mut no_cancel = dialog(&row()).with_cancel(None);
    no_cancel.input(&told(PAD));
    assert_eq!(tap(&mut no_cancel, PadButton::East), None);
}

#[test]
fn the_shoulders_step_the_focus_in_item_order_under_either_ring() {
    for ring in [FocusRing::Tab, FocusRing::Off] {
        let mut dialog = dialog(&row()).with_focus_ring(ring);
        dialog.input(&told(PAD));
        let mut order = Vec::new();
        for _ in 0..3 {
            tap(&mut dialog, PadButton::RightShoulder);
            order.push(dialog.focus());
        }
        assert_eq!(order, [Some(2), Some(3), Some(1)], "{ring:?}");
        tap(&mut dialog, PadButton::LeftShoulder);
        assert_eq!(dialog.focus(), Some(3), "{ring:?}");
    }
}

#[test]
fn a_gamepad_surroundings_focuses_the_default_button_first() {
    let mut dialog = dialog(&row()).with_default(Some(2));
    assert_eq!(dialog.focus(), None);
    dialog.input(&told(PAD));
    assert_eq!(dialog.focus(), Some(2));
    assert_eq!(dialog.device(), PAD);
    assert_eq!(
        tap(&mut dialog, PadButton::South),
        Some(DialogEvent::Item(2))
    );
}

#[test]
fn the_first_target_without_a_default() {
    for default in [None, Some(4)] {
        let mut dialog = dialog(&row()).with_default(default);
        dialog.input(&told(PAD));
        assert_eq!(dialog.focus(), Some(1), "{default:?}");
    }
    let mut greyed = dialog(&row());
    greyed.set_greyed(1, true);
    greyed.input(&told(PAD));
    assert_eq!(greyed.focus(), Some(2), "a greyed default is no target");
}

#[test]
fn the_highlight_shows_only_while_the_pad_was_used_last() {
    let mut dialog = dialog(&row());
    let plain = drawn(&dialog);
    dialog.input(&told(PAD));
    assert_eq!(dialog.pad_highlight(), Some(1));
    assert!(ends_with_highlight(&dialog, 1));
    assert_eq!(drawn(&dialog).len(), plain.len() + 8);
    for device in [Device::Keyboard, Device::Mouse] {
        let mut left = dialog.clone();
        left.input(&told(device));
        assert_eq!(left.pad_highlight(), None, "{device:?}");
        assert_eq!(drawn(&left), plain, "{device:?}");
    }
}

#[test]
fn return_presses_the_default_after_the_pad_moved_the_focus() {
    let mut dialog = dialog(&row());
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::DPadLeft);
    tap(&mut dialog, PadButton::DPadLeft);
    assert_eq!(dialog.focus(), Some(3));
    assert_eq!(dialog.input(&key(Key::Enter)), Some(DialogEvent::Item(1)));
}

#[test]
fn under_focus_ring_off_keyboard_tab_and_space_stay_inert_after_the_pad() {
    let mut dialog = dialog(&row()).with_focus_ring(FocusRing::Off);
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::DPadLeft);
    assert_eq!(
        dialog.input(&key(Key::Space)),
        None,
        "Space presses nothing"
    );
    assert_eq!(dialog.input(&key(Key::Tab)), None);
    assert_eq!(dialog.focus(), Some(2), "Tab moves nothing");
    dialog.input(&told(Device::Keyboard));
    assert_eq!(dialog.focus(), None, "the pad's focus is dropped");
    assert_eq!(dialog.input(&key(Key::Tab)), None);
    assert_eq!(dialog.focus(), None);
}

#[test]
fn under_focus_ring_tab_keyboard_tab_moves_on_from_the_pads_focus() {
    let mut dialog = dialog(&row());
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::DPadLeft);
    assert_eq!(dialog.input(&key(Key::Tab)), None);
    assert_eq!(dialog.focus(), Some(3));
    let ok = dialog.item_bounds(3).expect("Other");
    assert!(
        drawn(&dialog).contains(&DrawCommand::Line {
            from: ok.min,
            to: Point::new(ok.max.x, ok.min.y),
            width: 1.0,
            color: Color::WHITE,
        }),
        "the keyboard's ring"
    );
    assert_eq!(dialog.pad_highlight(), None);
    assert_eq!(dialog.input(&key(Key::Space)), Some(DialogEvent::Item(3)));
}

#[test]
fn a_key_bound_to_a_focus_move_moves_the_keyboards_focus_under_tab_alone() {
    use crate::keymap::{Bindings, KeyCombo, SharedBindings};
    let bindings = SharedBindings::new(
        Bindings::default().rebound(UiCommand::FocusLeft, KeyCombo::key(Key::KeyH)),
    );
    let mut dialog = dialog(&row()).with_bindings(bindings.clone());
    dialog.input(&key(Key::KeyH));
    assert_eq!(dialog.focus(), Some(1), "lands on the initial target");
    dialog.input(&key(Key::KeyH));
    assert_eq!(dialog.focus(), Some(2));
    assert_eq!(dialog.pad_highlight(), None, "the keyboard's");
    let mut off = dialog_with(bindings).with_focus_ring(FocusRing::Off);
    off.input(&key(Key::KeyH));
    assert_eq!(off.focus(), None);
}

fn dialog_with(bindings: crate::keymap::SharedBindings) -> Dialog {
    dialog(&row()).with_bindings(bindings)
}

#[test]
fn a_dpad_press_while_south_is_held_drops_the_press_silently() {
    let mut dialog = dialog(&row());
    dialog.input(&told(PAD));
    dialog.input(&pad(PadButton::South, true));
    assert_eq!(dialog.pressed_item(), Some(1));
    tap(&mut dialog, PadButton::DPadLeft);
    assert_eq!(dialog.pressed_item(), None);
    assert_eq!(dialog.input(&pad(PadButton::South, false)), None);
    assert_eq!(dialog.tracker.take_sound(), None);
}

#[test]
fn a_south_release_whose_press_it_never_saw_activates_nothing() {
    let mut dialog = dialog(&row());
    dialog.input(&told(PAD));
    assert_eq!(dialog.input(&pad(PadButton::South, false)), None);
    let at = dialog.item_bounds(1).expect("OK").center();
    dialog.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at,
    });
    assert_eq!(
        dialog.input(&pad(PadButton::South, false)),
        None,
        "a click's press is not South's"
    );
    assert_eq!(dialog.pressed_item(), Some(1), "the click goes on");
}

/// "Desc Dialog"-shaped with its arrows: Done (1), the text (2), a
/// role-less user item (3), a greyed button (4), an undrawn button (5),
/// then the up (6) and down (7) arrows.
fn desc_like() -> DialogTemplate {
    DialogTemplate {
        bounds: rect(0.0, 0.0, 200.0, 120.0),
        placement: Placement::Fixed,
        items: vec![
            item(rect(100.0, 90.0, 90.0, 25.0), true, ItemSpec::User),
            item(rect(10.0, 10.0, 120.0, 30.0), false, ItemSpec::User),
            item(rect(150.0, 10.0, 23.0, 23.0), true, ItemSpec::User),
            button(10.0, 90.0, "Grey"),
            button(10.0, 60.0, "Undrawn"),
            item(rect(135.0, 10.0, 10.0, 10.0), false, ItemSpec::User),
            item(rect(135.0, 30.0, 10.0, 10.0), false, ItemSpec::User),
        ],
    }
}

fn desc_dialog(text: &str) -> Dialog {
    let roles = [
        (1, Role::Button("Done".into())),
        (
            2,
            Role::ScrollText {
                text: text.into(),
                font: Font::Geneva,
                size: 10.0,
                color: Color::WHITE,
                backing: None,
            },
        ),
        (4, Role::Greyed),
        (6, Role::ScrollArrow(ScrollArrow::Up)),
        (7, Role::ScrollArrow(ScrollArrow::Down)),
    ];
    let mut dialog = Dialog::new(&desc_like(), &roles, Rc::new(MonoMetrics));
    dialog.set_drawn(5, false);
    dialog
}

const LONG: &str = "l0\rl1\rl2\rl3\rl4\rl5\rl6\rl7\rl8\rl9";

fn ids(dialog: &Dialog) -> Vec<usize> {
    dialog
        .pad_targets()
        .iter()
        .map(|target| target.id)
        .collect()
}

#[test]
fn scroll_arrows_and_clickable_user_items_are_targets_and_undrawn_or_greyed_buttons_are_not() {
    assert_eq!(ids(&desc_dialog(LONG)), [1, 3, 6, 7]);
    assert_eq!(
        ids(&desc_dialog("short")),
        [1, 3],
        "no arrows while it fits"
    );
    let mut overflowing = desc_dialog(LONG).with_pixel_scroll();
    overflowing.input(&told(PAD));
    overflowing.input(&pad(PadButton::RightShoulder, true));
    overflowing.input(&pad(PadButton::RightShoulder, true));
    overflowing.input(&pad(PadButton::RightShoulder, true));
    assert_eq!(overflowing.focus(), Some(7));
    assert_eq!(
        overflowing.input(&pad(PadButton::South, true)),
        None,
        "an arrow reports nothing"
    );
    assert!(overflowing.holds_arrow(), "a hold, as a left press makes");
    overflowing.tick(std::time::Duration::from_nanos(16_666_666 * 3));
    assert_eq!(overflowing.input(&pad(PadButton::South, false)), None);
    assert!(!overflowing.holds_arrow());
    overflowing.tick(std::time::Duration::from_nanos(16_666_666 * 3));
    let mut clicked = desc_dialog(LONG).with_pixel_scroll();
    let at = clicked.item_bounds(7).expect("down").center();
    clicked.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at,
    });
    clicked.tick(std::time::Duration::from_nanos(16_666_666 * 3));
    clicked.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: false,
        at,
    });
    let offset = |dialog: &Dialog| dialog.scroll_text().expect("text").offset();
    assert_eq!(offset(&overflowing), offset(&clicked));
    assert_eq!(offset(&overflowing), 4.0, "a step, then three ticks'");
    let mut user = desc_dialog(LONG);
    user.input(&told(PAD));
    tap(&mut user, PadButton::RightShoulder);
    assert_eq!(user.focus(), Some(3));
    assert_eq!(tap(&mut user, PadButton::South), Some(DialogEvent::Item(3)));
}

#[test]
fn the_left_stick_scrolls_the_text_as_the_arrows_do() {
    let mut keyed = desc_dialog(LONG);
    keyed.input(&key(Key::ArrowDown));
    let mut padded = desc_dialog(LONG);
    padded.input(&told(PAD));
    tap(&mut padded, PadButton::LeftStickDown);
    let scroll = |dialog: &Dialog| dialog.scroll_text().expect("text").first();
    assert!(scroll(&padded) > 0);
    assert_eq!(scroll(&padded), scroll(&keyed));
    tap(&mut padded, PadButton::LeftStickUp);
    assert_eq!(scroll(&padded), 0);
}

#[test]
fn a_dialog_laid_out_again_keeps_the_pads_focused_item_and_device() {
    let mut dialog = self::dialog(&row());
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::DPadLeft);
    dialog.lay_out_again(self::dialog(&row()));
    assert_eq!(dialog.device(), PAD);
    assert_eq!(dialog.focus(), Some(2));
    assert_eq!(dialog.pad_highlight(), Some(2));
    assert_eq!(dialog.keys.device(), PAD, "its reader names the pad too");
    tap(&mut dialog, PadButton::DPadLeft);
    assert_eq!(
        tap(&mut dialog, PadButton::South),
        Some(DialogEvent::Item(3))
    );
    let mut keyboard = self::dialog(&row());
    keyboard.lay_out_again(self::dialog(&row()));
    assert_eq!(
        (keyboard.device(), keyboard.focus()),
        (Device::Keyboard, None)
    );
}

#[test]
fn a_dialog_laid_out_again_keeps_its_focus_ring_and_bindings() {
    use crate::keymap::{Bindings, KeyCombo, SharedBindings};
    let bindings = SharedBindings::new(
        Bindings::default().rebound(UiCommand::Confirm, KeyCombo::key(Key::KeyT)),
    );
    let mut dialog = dialog_with(bindings).with_focus_ring(FocusRing::Off);
    dialog.lay_out_again(self::dialog(&row()));
    assert_eq!(dialog.focus_ring(), FocusRing::Off);
    assert_eq!(dialog.input(&key(Key::Enter)), None, "Return is rebound");
    assert_eq!(dialog.input(&key(Key::KeyT)), Some(DialogEvent::Item(1)));
}

#[test]
fn the_pads_focus_on_an_item_gone_from_the_new_layout_starts_again() {
    let mut dialog = self::dialog(&row());
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::DPadLeft);
    tap(&mut dialog, PadButton::DPadLeft);
    assert_eq!(dialog.focus(), Some(3));
    let without_other = Dialog::new(&row(), &[(3, Role::Hidden)], Rc::new(MonoMetrics));
    dialog.lay_out_again(without_other);
    assert_eq!(dialog.focus(), Some(1), "the initial target, the default");
    assert_eq!(dialog.pad_highlight(), Some(1));
}

#[test]
fn a_dialog_told_the_pad_with_nothing_focused_shows_the_pads_focus_once_laid_out_again() {
    let mut dialog = Dialog::new(&row(), &[(1, Role::Hidden)], Rc::new(MonoMetrics));
    dialog.input(&told(PAD));
    assert_eq!(dialog.focus(), Some(2), "the first target");
    dialog.lay_out_again(self::dialog(&row()));
    assert_eq!(dialog.pad_highlight(), Some(2), "still a target");
    let mut empty = Dialog::new(
        &row(),
        &[(1, Role::Hidden), (2, Role::Hidden), (3, Role::Hidden)],
        Rc::new(MonoMetrics),
    );
    empty.input(&told(PAD));
    assert_eq!(empty.focus(), None, "nothing to focus");
    empty.lay_out_again(self::dialog(&row()));
    assert_eq!(
        empty.pad_highlight(),
        Some(1),
        "lands on the new initial target"
    );
}

#[test]
fn a_held_south_holds_back_keys_as_a_held_click_does() {
    let mut dialog = dialog(&row()).with_held_keys();
    dialog.input(&told(PAD));
    dialog.input(&pad(PadButton::South, true));
    assert!(dialog.holds());
    assert_eq!(dialog.input(&key(Key::Escape)), None, "held back");
    assert_eq!(
        dialog.input(&pad(PadButton::South, false)),
        Some(DialogEvent::Item(1))
    );
    assert!(!dialog.holds());
    assert_eq!(dialog.take_replay(), vec![key(Key::Escape)]);
}

impl Dialog {
    fn holds_arrow(&self) -> bool {
        self.hold.is_some()
    }
}

#[test]
fn a_dpad_press_ends_a_pads_hold_on_an_arrow() {
    let mut dialog = desc_dialog(LONG).with_pixel_scroll();
    dialog.input(&told(PAD));
    for _ in 0..3 {
        tap(&mut dialog, PadButton::RightShoulder);
    }
    dialog.input(&pad(PadButton::South, true));
    tap(&mut dialog, PadButton::DPadLeft);
    assert!(!dialog.holds_arrow());
    let at = dialog.item_bounds(7).expect("down").center();
    dialog.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at,
    });
    assert_eq!(dialog.input(&pad(PadButton::South, false)), None);
    assert!(
        dialog.holds_arrow(),
        "South's release never ends a click's hold"
    );
}

#[test]
fn turning_the_ring_off_keeps_the_pads_focus() {
    let mut dialog = dialog(&row());
    dialog.input(&told(PAD));
    dialog.set_focus_ring(FocusRing::Off);
    assert_eq!(dialog.focus(), Some(1));
    assert_eq!(dialog.pad_highlight(), Some(1));
}

#[test]
fn a_focus_greyed_out_shows_no_highlight_and_south_presses_nothing() {
    let mut dialog = dialog(&row());
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::DPadLeft);
    assert_eq!(dialog.pad_highlight(), Some(2));
    dialog.set_greyed(2, true);
    assert_eq!(dialog.focus(), Some(2), "kept, for when it comes back");
    assert_eq!(dialog.pad_highlight(), None);
    let before = drawn(&dialog);
    dialog.input(&pad(PadButton::South, true));
    assert_eq!(dialog.pressed_item(), None);
    assert_eq!(drawn(&dialog), before);
    assert_eq!(dialog.input(&pad(PadButton::South, false)), None);
}

/// OK (1, the default) and Cancel (2) at the bottom, a user item (3)
/// across the top that a host draws two cells in, and a dead user item
/// (4) at the bottom left.
fn celled() -> Dialog {
    let template = DialogTemplate {
        bounds: rect(0.0, 0.0, 300.0, 140.0),
        placement: Placement::Fixed,
        items: vec![
            button(220.0, 110.0, "OK"),
            button(140.0, 110.0, "Cancel"),
            item(rect(10.0, 10.0, 280.0, 80.0), true, ItemSpec::User),
            item(rect(10.0, 110.0, 40.0, 20.0), true, ItemSpec::User),
        ],
    };
    let mut dialog = self::dialog(&template);
    dialog.set_pad_cells(3, cells());
    dialog
}

fn cells() -> Vec<Bounds> {
    vec![rect(120.0, 20.0, 60.0, 60.0), rect(220.0, 20.0, 60.0, 60.0)]
}

fn spots(dialog: &Dialog) -> Vec<PadSpot> {
    dialog.pad_spots().iter().map(|target| target.id).collect()
}

#[test]
fn cells_stand_in_for_their_item_in_tab_order() {
    let dialog = celled();
    assert_eq!(
        spots(&dialog),
        [
            PadSpot::Item(1),
            PadSpot::Item(2),
            PadSpot::Cell(0),
            PadSpot::Cell(1),
            PadSpot::Item(4),
        ]
    );
    let mut none = celled();
    none.set_pad_cells(3, Vec::new());
    assert!(
        !spots(&none).contains(&PadSpot::Item(3)),
        "no cells, no item"
    );
}

#[test]
fn the_dpad_moves_between_cells_and_buttons() {
    let mut dialog = celled();
    dialog.input(&told(PAD));
    assert_eq!(dialog.focus(), Some(1), "on the default");
    tap(&mut dialog, PadButton::DPadUp);
    assert_eq!(dialog.pad_highlight_cell(), Some(1));
    assert_eq!(dialog.focus(), None, "a cell is no item");
    tap(&mut dialog, PadButton::DPadLeft);
    assert_eq!(dialog.pad_highlight_cell(), Some(0));
    assert!(drawn(&dialog).ends_with(&highlight(cells()[0])));
    tap(&mut dialog, PadButton::DPadDown);
    assert_eq!(dialog.pad_highlight_cell(), None);
    assert_eq!(dialog.pad_highlight(), Some(2));
}

#[test]
fn south_on_a_cell_reports_it_on_the_press_silently() {
    let mut dialog = celled();
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::DPadUp);
    assert_eq!(
        dialog.input(&pad(PadButton::South, true)),
        Some(DialogEvent::Cell(1))
    );
    assert_eq!(dialog.input(&pad(PadButton::South, false)), None);
    assert_eq!(dialog.tracker.take_sound(), None);
    assert_eq!(dialog.tracked, None, "nothing armed");
}

#[test]
fn a_cell_gone_draws_no_highlight_and_the_next_move_lands_on_the_initial_target() {
    let mut dialog = celled();
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::DPadUp);
    dialog.set_pad_cells(3, vec![cells()[0]]);
    assert_eq!(dialog.pad_highlight_cell(), None);
    assert_eq!(dialog.input(&pad(PadButton::South, true)), None);
    tap(&mut dialog, PadButton::DPadLeft);
    assert_eq!(dialog.pad_highlight(), Some(1), "the initial target");
}

#[test]
fn keyboard_tab_never_visits_a_cell_and_goes_on_from_the_cells_item() {
    let mut keyed = celled();
    let mut order = Vec::new();
    for _ in 0..3 {
        keyed.input(&key(Key::Tab));
        order.push(keyed.focus());
    }
    assert_eq!(order, [Some(1), Some(2), Some(1)], "buttons only");
    let mut dialog = celled();
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::DPadUp);
    dialog.input(&told(Device::Keyboard));
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::DPadUp);
    assert_eq!(dialog.pad_highlight_cell(), Some(1));
    dialog.input(&key(Key::Tab));
    assert_eq!(dialog.focus(), Some(1), "on from item 3, wrapping to OK");
}

#[test]
fn focus_names_items_only() {
    let mut dialog = celled();
    dialog.input(&told(PAD));
    assert_eq!(dialog.focus(), Some(1));
    tap(&mut dialog, PadButton::DPadUp);
    assert_eq!(dialog.focus(), None);
    assert_eq!(dialog.pad_highlight(), None);
    assert!(format!("{dialog:?}").contains("focus: None"));
}

#[test]
fn items_left_out_are_no_targets() {
    let template = row();
    let mut dialog = self::dialog(&template);
    dialog.set_pad_left_out(vec![2]);
    assert_eq!(ids(&dialog), [1, 3]);
    let mut dialog = celled();
    dialog.set_pad_left_out(vec![4]);
    assert!(!spots(&dialog).contains(&PadSpot::Item(4)));
    assert!(spots(&celled()).contains(&PadSpot::Item(4)));
}

#[test]
fn items_left_out_again_replace_those_left_out_before() {
    let template = row();
    let mut dialog = self::dialog(&template);
    dialog.set_pad_left_out(vec![2]);
    dialog.set_pad_left_out(vec![3]);
    assert_eq!(ids(&dialog), [1, 2], "item 2 a target again");
    dialog.set_pad_left_out(Vec::new());
    assert_eq!(ids(&dialog), [1, 2, 3]);
}

#[test]
fn a_dialog_laid_out_again_keeps_a_pad_focused_cell() {
    let mut dialog = celled();
    dialog.set_pad_left_out(vec![4]);
    dialog.input(&told(PAD));
    tap(&mut dialog, PadButton::DPadUp);
    let template = DialogTemplate {
        bounds: rect(0.0, 0.0, 300.0, 140.0),
        placement: Placement::Fixed,
        items: vec![
            button(220.0, 110.0, "OK"),
            button(140.0, 110.0, "Cancel"),
            item(rect(10.0, 10.0, 280.0, 80.0), true, ItemSpec::User),
            item(rect(10.0, 110.0, 40.0, 20.0), true, ItemSpec::User),
        ],
    };
    dialog.lay_out_again(self::dialog(&template));
    assert_eq!(dialog.pad_highlight_cell(), Some(1));
    assert!(
        !spots(&dialog).contains(&PadSpot::Item(4)),
        "still left out"
    );
}
