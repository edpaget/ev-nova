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

use crate::geometry::Bounds;

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
    const fn label(self) -> &'static str {
        match self {
            Self::Up => "^",
            Self::Down => "&",
        }
    }

    /// The button this arrow draws at `rect` while `text` is as it is, or
    /// none with both of its flags clear.
    pub(super) fn button(self, rect: Bounds, text: &ScrollText) -> Option<Button> {
        let (up, down) = (text.can_scroll_up(), text.can_scroll_down());
        (up || down).then(|| Button {
            enabled: match self {
                Self::Up => up,
                Self::Down => down,
            },
            ..Button::new(rect, self.label())
        })
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
    use crate::input::{Input, Key};
    use crate::text::fixture::MonoMetrics;
    use crate::ui::button::{Button, ButtonSkin, ButtonStyle};
    use crate::ui::dialog::{Dialog, DialogTemplate, ItemSpec, ItemTemplate, Placement, Role};

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
}
