//! The scroll arrows of a text dialog: items 5 and 6 of `DLOG` 3003 and
//! 3004, drawn by `_DrawTextButtons` @0x269ea.
//!
//! An arrow is a button built afresh at each draw, from the flags of the
//! text it scrolls. `_DrawTextButtons` gives `_DrawButton` the states
//! (@0x26a89-0x26afb): both flags clear, −2 and −2, so neither arrow shows;
//! one flag set, 0 for its arrow and −1, disabled, for the other. The
//! original's arrows are up to date one event pass after any key:
//! `_ShowTextDialog`'s loop invalidates an arrow whose flag changed
//! (@0x5b273-0x5b31f). Reading the flags at each draw matches that.
//!
//! A press on an arrow is [`Dialog`](super::Dialog)'s: an [`ArrowHold`]
//! while its flag is set, and otherwise tracked as `_TrackTextButtons`
//! @0x26bb0 tracks it. Nothing here plays a sound, as `_DrawButton`,
//! `_DrawTextButtons`, `_TrackTextButtons`, `_ScrollDialogText` and the
//! filter's arms play none.

use std::time::Duration;

use crate::geometry::Bounds;
use crate::time::sixtieths;

use super::button::Button;
use super::scroll_text::ScrollText;

/// One of a text's two scroll arrows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollArrow {
    /// Toward the text's start: item 5.
    Up,
    /// Toward the text's end: item 6.
    Down,
}

impl ScrollArrow {
    /// Its `_DrawButton` label, `STR#` 150 #19 or #20, a pen glyph
    /// (`_textButtonTable`, @0x26a78-0x26a83).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Up => "^",
            Self::Down => "&",
        }
    }

    /// Whether this arrow's flag is set for `text`.
    pub(super) fn can_scroll(self, text: &ScrollText) -> bool {
        match self {
            Self::Up => text.can_scroll_up(),
            Self::Down => text.can_scroll_down(),
        }
    }

    /// The button this arrow draws at `rect` while `text` is as it is, or
    /// none with both of its flags clear.
    pub(super) fn button(self, rect: Bounds, text: &ScrollText) -> Option<Button> {
        (text.can_scroll_up() || text.can_scroll_down()).then(|| Button {
            enabled: self.can_scroll(text),
            ..Button::new(rect, self.label())
        })
    }

    /// One step of a held press, in units toward the text's end:
    /// `_ScrollDialogText(±1, immediate)` (@0x52172-0x5237f).
    pub(super) const fn step(self) -> f32 {
        match self {
            Self::Up => -1.0,
            Self::Down => 1.0,
        }
    }
}

/// A press held on a scroll arrow, as `_TextDialogFilter`'s loops hold one
/// (@0x52172-0x5237f): a step at once, then one each Mac tick, 60 px/s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ArrowHold {
    arrow: ScrollArrow,
    elapsed: Duration,
}

impl ArrowHold {
    /// A press just made on `arrow`, whose first step is the caller's.
    pub(super) const fn new(arrow: ScrollArrow) -> Self {
        Self {
            arrow,
            elapsed: Duration::ZERO,
        }
    }

    /// The arrow held.
    pub(super) const fn arrow(self) -> ScrollArrow {
        self.arrow
    }

    /// How many steps fall due after `dt` more: one for each whole Mac tick
    /// the hold has passed.
    pub(super) fn advance(&mut self, dt: Duration) -> u128 {
        let before = sixtieths(self.elapsed);
        self.elapsed += dt;
        sixtieths(self.elapsed) - before
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::color::Color;
    use crate::draw::{DrawCommand, DrawList};
    use crate::font::Font;
    use crate::geometry::{Bounds, Point};
    use crate::input::{Input, Key, MouseButton, ScrollDelta};
    use crate::text::fixture::MonoMetrics;
    use crate::ui::button::{Button, ButtonSkin, ButtonStyle};
    use crate::ui::dialog::{
        Dialog, DialogEvent, DialogTemplate, ItemSpec, ItemTemplate, Placement, Role,
    };
    use crate::ui::scroll_text::ARROW_UNITS;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// The dialog's corner, which the items are placed from.
    const ORIGIN: Point = Point::new(40.0, 20.0);

    const UP_ITEM: usize = 5;
    const DOWN_ITEM: usize = 6;

    /// "Desc Dialog"'s shape, smaller: Done (1), a 60 x 36 text box (3) and
    /// the arrows (5, 6), disabled user items as in stock `DITL` 3003.
    fn template() -> DialogTemplate {
        let item = |bounds, enabled| ItemTemplate {
            bounds,
            enabled,
            kind: ItemSpec::User,
        };
        DialogTemplate {
            bounds: Bounds::at(ORIGIN, 200.0, 120.0),
            placement: Placement::Fixed,
            items: vec![
                item(rect(10.0, 90.0, 60.0, 25.0), true),
                item(rect(10.0, 400.0, 20.0, 20.0), false),
                item(rect(10.0, 10.0, 60.0, 36.0), false),
                item(rect(10.0, 400.0, 20.0, 20.0), false),
                item(rect(100.0, 90.0, 23.0, 23.0), false),
                item(rect(133.0, 90.0, 23.0, 23.0), false),
            ],
        }
    }

    fn roles(text: &str) -> Vec<(usize, Role)> {
        vec![
            (1, Role::Button("Okay".into())),
            (
                3,
                Role::ScrollText {
                    text: text.into(),
                    font: Font::Geneva,
                    size: 10.0,
                    color: Color::WHITE,
                    backing: None,
                },
            ),
            (UP_ITEM, Role::ScrollArrow(ScrollArrow::Up)),
            (DOWN_ITEM, Role::ScrollArrow(ScrollArrow::Down)),
        ]
    }

    fn dialog(text: &str) -> Dialog {
        Dialog::new(&template(), &roles(text), Rc::new(MonoMetrics)).with_pixel_scroll()
    }

    /// Five lines, 60 high in the 36 box: 24 is as far as they scroll.
    fn long() -> Dialog {
        dialog("l0\rl1\rl2\rl3\rl4")
    }

    const END: f32 = 24.0;

    fn at_offset(offset: f32) -> Dialog {
        let mut dialog = long();
        dialog
            .scroll_text_mut()
            .expect("the text")
            .scroll_units(offset);
        dialog
    }

    /// What the dialog draws after its last text line: the arrows, if any.
    fn after_the_text(dialog: &Dialog) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        let commands: Vec<DrawCommand> = list.iter().cloned().collect();
        let is_body = |command: &DrawCommand| match command {
            DrawCommand::Text { font, .. } => *font == Font::Geneva,
            DrawCommand::Clipped { .. } => true,
            _ => false,
        };
        let end = commands.iter().rposition(is_body).expect("the text");
        commands[end + 1..].to_vec()
    }

    /// `item` drawn as `_DrawButton` draws an arrow.
    fn arrow(item: usize, label: &str, enabled: bool, pressed: bool) -> Vec<DrawCommand> {
        let rect = template().items[item - 1].bounds.offset(ORIGIN);
        let button = Button {
            rect,
            label: label.to_owned(),
            enabled,
        };
        let mut list = DrawList::new();
        button.draw(
            pressed,
            &ButtonSkin::NOVA,
            &ButtonStyle::STOCK,
            &MonoMetrics,
            &mut list,
        );
        list.iter().cloned().collect()
    }

    fn up(enabled: bool, pressed: bool) -> Vec<DrawCommand> {
        arrow(UP_ITEM, "^", enabled, pressed)
    }

    fn down(enabled: bool, pressed: bool) -> Vec<DrawCommand> {
        arrow(DOWN_ITEM, "&", enabled, pressed)
    }

    /// `_DrawTextButtons` gives both arrows state −2 with both flags clear
    /// (@0x26a89-0x26afb), which draws neither face nor label.
    #[test]
    fn a_text_that_fits_draws_neither_arrow() {
        assert_eq!(after_the_text(&dialog("l0\rl1\rl2")), []);
    }

    #[test]
    fn at_the_top_up_is_disabled_and_down_normal() {
        assert_eq!(
            after_the_text(&long()),
            [up(false, false), down(true, false)].concat()
        );
    }

    #[test]
    fn in_the_middle_both_are_normal() {
        assert_eq!(
            after_the_text(&at_offset(5.0)),
            [up(true, false), down(true, false)].concat()
        );
    }

    #[test]
    fn at_the_end_down_is_disabled_only_after_the_failed_try() {
        let mut dialog = at_offset(END);
        assert_eq!(
            after_the_text(&dialog),
            [up(true, false), down(true, false)].concat(),
            "reaching the end moved it"
        );
        dialog.input(&Input::Key {
            key: Key::Down,
            pressed: true,
            repeat: false,
        });
        assert_eq!(
            after_the_text(&dialog),
            [up(true, false), down(false, false)].concat()
        );
    }

    #[test]
    fn with_no_scrolling_text_an_arrow_draws_nothing() {
        let roles = [(UP_ITEM, Role::ScrollArrow(ScrollArrow::Up))];
        let dialog = Dialog::new(&template(), &roles, Rc::new(MonoMetrics));
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        assert!(list.iter().all(|command| !matches!(
            command,
            DrawCommand::StretchedPicture { top_left, .. }
                if *top_left == dialog.item_bounds(UP_ITEM).expect("up").min
        )));
    }

    #[test]
    fn tab_skips_the_arrows() {
        let mut dialog = long();
        for _ in 0..3 {
            dialog.input(&Input::Key {
                key: Key::Tab,
                pressed: true,
                repeat: false,
            });
            assert_eq!(dialog.focus(), Some(1));
        }
    }

    /// The filter hit-tests them itself, so the `DITL`'s disabled flag means
    /// nothing to them (@0x52172-0x522d1).
    #[test]
    fn an_arrow_is_enabled_whatever_the_ditl_says_only_as_a_user_item() {
        assert!(!template().items[UP_ITEM - 1].enabled);
        let mut template = template();
        template.items[UP_ITEM - 1].kind = ItemSpec::Other;
        let dialog = Dialog::new(&template, &roles("l0"), Rc::new(MonoMetrics));
        let at = dialog.item_bounds(UP_ITEM).expect("up").center();
        let mut dialog = dialog;
        for pressed in [true, false] {
            let event = dialog.input(&Input::PointerButton {
                button: crate::input::MouseButton::Left,
                pressed,
                at,
            });
            assert_eq!(event, None, "a disabled item of another kind");
        }
    }

    // The held press, `_TextDialogFilter` @0x52172-0x5237f.

    fn left(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn right(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Right,
            pressed,
            at,
        }
    }

    fn up_at(dialog: &Dialog) -> Point {
        dialog.item_bounds(UP_ITEM).expect("up").center()
    }

    fn down_at(dialog: &Dialog) -> Point {
        dialog.item_bounds(DOWN_ITEM).expect("down").center()
    }

    fn offset(dialog: &Dialog) -> f32 {
        dialog.scroll_text().expect("the text").offset()
    }

    /// One Mac tick.
    const TICK: Duration = Duration::from_nanos(16_666_666);

    /// Five hundred pixels of lines, for holds that don't reach an end.
    fn tall() -> Dialog {
        let lines: Vec<String> = (0..50).map(|n| format!("l{n}")).collect();
        dialog(&lines.join("\r"))
    }

    #[test]
    fn a_press_on_the_down_arrow_moves_a_pixel_at_once_and_draws_it_pressed() {
        let mut dialog = long();
        let at = down_at(&dialog);
        assert_eq!(dialog.input(&left(true, at)), None);
        assert_eq!(offset(&dialog), 1.0);
        assert_eq!(
            after_the_text(&dialog),
            [up(true, false), down(true, true)].concat()
        );
    }

    #[test]
    fn a_press_on_the_up_arrow_moves_a_pixel_back_at_once_and_draws_it_pressed() {
        let mut dialog = at_offset(10.0);
        let at = up_at(&dialog);
        assert_eq!(dialog.input(&left(true, at)), None);
        assert_eq!(offset(&dialog), 9.0);
        assert_eq!(
            after_the_text(&dialog),
            [up(true, true), down(true, false)].concat()
        );
    }

    #[test]
    fn a_press_on_an_arrows_left_and_top_edges_holds_and_on_its_right_and_bottom_doesnt() {
        for (dx, dy, held) in [
            (0.0, 0.0, true),
            (22.0, 22.0, true),
            (23.0, 10.0, false),
            (10.0, 23.0, false),
        ] {
            let mut dialog = long();
            let corner = dialog.item_bounds(DOWN_ITEM).expect("down").min;
            let at = Point::new(corner.x + dx, corner.y + dy);
            dialog.input(&left(true, at));
            assert_eq!(offset(&dialog) > 0.0, held, "{dx} {dy}");
        }
    }

    #[test]
    fn a_second_button_on_an_arrow_holds_nothing() {
        let mut dialog = long();
        let at = down_at(&dialog);
        dialog.input(&right(true, at));
        dialog.tick(TICK);
        assert_eq!(offset(&dialog), 0.0);
    }

    #[test]
    fn each_mac_tick_of_a_hold_moves_a_pixel_and_a_frame_of_a_thirtieth_two() {
        let mut dialog = tall();
        let at = down_at(&dialog);
        dialog.input(&left(true, at));
        dialog.tick(TICK);
        assert_eq!(offset(&dialog), 2.0);
        dialog.tick(TICK);
        assert_eq!(offset(&dialog), 3.0);
        dialog.tick(Duration::from_secs(1) / 30);
        assert_eq!(offset(&dialog), 5.0);
    }

    #[test]
    fn frames_shorter_than_a_tick_add_up_to_one() {
        let mut dialog = tall();
        let at = down_at(&dialog);
        dialog.input(&left(true, at));
        for expected in [1.0, 2.0, 2.0, 3.0] {
            dialog.tick(TICK / 2 + Duration::from_nanos(1));
            assert_eq!(offset(&dialog), expected);
        }
    }

    #[test]
    fn with_no_hold_a_tick_moves_nothing() {
        let mut dialog = tall();
        dialog.tick(TICK * 10);
        assert_eq!(offset(&dialog), 0.0);
    }

    /// The loop tests only `StillDown`.
    #[test]
    fn dragging_off_the_arrow_keeps_it_scrolling_and_drawn_pressed() {
        let mut dialog = tall();
        let at = down_at(&dialog);
        dialog.input(&left(true, at));
        dialog.input(&Input::PointerMoved(Point::new(0.0, 0.0)));
        dialog.input(&Input::PointerLeft);
        dialog.tick(TICK);
        assert_eq!(offset(&dialog), 2.0);
        assert_eq!(
            after_the_text(&dialog),
            [up(true, false), down(true, true)].concat()
        );
    }

    #[test]
    fn the_release_stops_the_hold_on_the_arrow_or_off_it_and_it_draws_up_again() {
        for at in [None, Some(Point::new(0.0, 0.0))] {
            let mut dialog = tall();
            let arrow = down_at(&dialog);
            dialog.input(&left(true, arrow));
            assert_eq!(dialog.input(&left(false, at.unwrap_or(arrow))), None);
            dialog.tick(TICK * 5);
            assert_eq!(offset(&dialog), 1.0, "{at:?}");
            assert_eq!(
                after_the_text(&dialog),
                [up(true, false), down(true, false)].concat()
            );
        }
    }

    #[test]
    fn held_to_the_end_it_turns_disabled_after_the_failed_step_and_runs_on() {
        let mut dialog = long();
        let at = down_at(&dialog);
        dialog.input(&left(true, at));
        dialog.tick(TICK * 23);
        assert_eq!(offset(&dialog), END);
        assert_eq!(
            after_the_text(&dialog),
            [up(true, false), down(true, true)].concat(),
            "it moved on the way there"
        );
        dialog.tick(TICK);
        assert_eq!(offset(&dialog), END);
        assert_eq!(
            after_the_text(&dialog),
            [up(true, false), down(false, true)].concat(),
            "the failed step clears it"
        );
        dialog.tick(TICK * 5);
        dialog.input(&left(false, at));
        let up_arrow = up_at(&dialog);
        dialog.input(&left(true, up_arrow));
        assert_eq!(offset(&dialog), END - 1.0, "scrolling back is free");
        assert!(dialog.scroll_text().expect("the text").can_scroll_down());
    }

    #[test]
    fn held_past_the_top_it_runs_on_and_keeps_up_disabled() {
        let mut dialog = at_offset(2.0);
        let at = up_at(&dialog);
        dialog.input(&left(true, at));
        dialog.tick(TICK);
        assert_eq!(offset(&dialog), 0.0);
        dialog.tick(TICK);
        assert_eq!(offset(&dialog), 0.0);
        assert_eq!(
            after_the_text(&dialog),
            [up(false, true), down(true, false)].concat()
        );
    }

    #[test]
    fn cancelling_the_pointer_ends_the_hold() {
        let mut dialog = tall();
        let at = down_at(&dialog);
        dialog.input(&left(true, at));
        dialog.cancel_pointer();
        dialog.tick(TICK * 5);
        assert_eq!(offset(&dialog), 1.0);
        assert_eq!(
            after_the_text(&dialog),
            [up(true, false), down(true, false)].concat()
        );
    }

    /// A press during a hold recovers a release that never came.
    #[test]
    fn a_left_press_during_a_hold_ends_it_and_acts_as_any_press() {
        let mut dialog = tall();
        let arrow = down_at(&dialog);
        let done = dialog.item_bounds(1).expect("Okay").center();
        dialog.input(&left(true, arrow));
        assert_eq!(dialog.input(&left(true, done)), None);
        dialog.tick(TICK * 5);
        assert_eq!(offset(&dialog), 1.0, "the hold is over");
        assert_eq!(dialog.pressed_item(), Some(1));
        assert_eq!(dialog.input(&left(false, done)), Some(DialogEvent::Item(1)));
    }

    #[test]
    fn a_left_press_on_the_other_arrow_during_a_hold_holds_that_one() {
        let mut dialog = tall();
        let down_arrow = down_at(&dialog);
        let up_arrow = up_at(&dialog);
        dialog.input(&left(true, down_arrow));
        dialog.tick(TICK * 4);
        dialog.input(&left(true, up_arrow));
        dialog.tick(TICK);
        assert_eq!(offset(&dialog), 3.0, "5, then two back");
    }

    #[test]
    fn a_second_buttons_press_and_release_leave_a_hold_alone() {
        let mut dialog = tall();
        let at = down_at(&dialog);
        dialog.input(&left(true, at));
        dialog.input(&right(true, at));
        dialog.input(&right(false, at));
        dialog.tick(TICK);
        assert_eq!(offset(&dialog), 2.0);
    }

    /// `_TrackTextButtons` takes a press on an arrow whose flag is clear
    /// (@0x5238c-0x523f7), and `_ShowTextDialog` tests the flag again
    /// (@0x5b0b5-0x5b0bd, @0x5b0f7-0x5b0ff).
    #[test]
    fn a_press_on_an_arrow_with_its_flag_clear_scrolls_nothing_and_activates_nothing() {
        let mut dialog = long();
        let at = up_at(&dialog);
        assert_eq!(dialog.input(&left(true, at)), None);
        assert_eq!(offset(&dialog), 0.0);
        dialog.tick(TICK * 5);
        assert_eq!(offset(&dialog), 0.0);
        assert_eq!(dialog.input(&left(false, at)), None);
        assert_eq!(offset(&dialog), 0.0);
        assert_eq!(
            after_the_text(&dialog),
            [up(false, false), down(true, false)].concat()
        );
    }

    /// Only the engine's wheel can set a flag between the press and the
    /// release.
    #[test]
    fn a_click_on_an_arrow_whose_flag_a_wheel_set_steps_ten_pixels_on_the_release() {
        let mut dialog = long();
        let at = up_at(&dialog);
        dialog.input(&left(true, at));
        dialog.input(&Input::Scroll {
            delta: ScrollDelta::Units(20.0),
            at: dialog.item_bounds(3).expect("the text").center(),
        });
        assert_eq!(offset(&dialog), 20.0);
        assert_eq!(dialog.input(&left(false, at)), None);
        assert_eq!(offset(&dialog), 20.0 - ARROW_UNITS);
    }

    #[test]
    fn a_click_on_the_down_arrow_whose_flag_a_wheel_set_steps_ten_pixels_too() {
        let mut dialog = long();
        dialog
            .scroll_text_mut()
            .expect("the text")
            .clear_can_scroll_down();
        let at = down_at(&dialog);
        dialog.input(&left(true, at));
        assert_eq!(offset(&dialog), 0.0, "tracked, not held");
        dialog.input(&Input::Scroll {
            delta: ScrollDelta::Units(5.0),
            at: dialog.item_bounds(3).expect("the text").center(),
        });
        assert_eq!(dialog.input(&left(false, at)), None);
        assert_eq!(offset(&dialog), 5.0 + ARROW_UNITS);
    }

    #[test]
    fn a_tracked_arrow_released_off_it_does_nothing() {
        let mut dialog = at_offset(5.0);
        let at = up_at(&dialog);
        dialog
            .scroll_text_mut()
            .expect("the text")
            .clear_can_scroll_down();
        let down_arrow = down_at(&dialog);
        dialog.input(&left(true, down_arrow));
        dialog.input(&Input::Scroll {
            delta: ScrollDelta::Units(1.0),
            at: dialog.item_bounds(3).expect("the text").center(),
        });
        assert_eq!(dialog.input(&left(false, at)), None);
        assert_eq!(offset(&dialog), 6.0);
    }
}
