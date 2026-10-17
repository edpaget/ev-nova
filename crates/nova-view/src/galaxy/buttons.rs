//! The in-flight maps' Done and Find buttons, until the map is laid out
//! as DITL 2000 (galaxy-map-fidelity phase 5, which takes them over with
//! the rest of `_DrawMapButtons`).
//!
//! Done is DITL 2000's item 1 and Find its item 10, each placed from the
//! panels' temporary [`PANEL_ORIGIN`], so both sit in the bottom panel
//! under the info strip and right of the map's own lines. Their labels
//! are stock `STR#` 150's `[4]` "Done" and `[59]` "Find" (0-based), and both
//! are always enabled (`_DrawMapButtons` @0x2a990-0x2a9d5).
//!
//! `_TrackMapButtons` (@0x2aafa) arms the enabled button under a press,
//! shows it pressed while the pointer stays on it, and gives it only for
//! a release over it (@0x2ada3). It plays no sound, so the trackers'
//! button sounds are dropped. `_MapFilter` turns Done into item 1, as
//! Return is (@0x1457a-0x14587), and Find into item 10, as F and S are
//! (@0x145d8-0x145e6).

use super::panel::{PANEL_ORIGIN, placed};
use crate::draw::DrawList;
use crate::flight::radar::IRect;
use crate::font::Font;
use crate::input::Input;
use crate::text::TextMetrics;
use crate::ui::button::{Button, ButtonSkin, ButtonStyle, ButtonTracker};

/// DITL 2000 item 1, Done, in dialog coordinates (t 483, l 483, b 508,
/// r 582).
pub const DONE_ITEM: IRect = IRect::new(483, 483, 582, 508);
/// DITL 2000 item 10, Find, in dialog coordinates (t 483, l 288, b 508,
/// r 387).
pub const FIND_ITEM: IRect = IRect::new(288, 483, 387, 508);
/// Done's label, stock `STR#` 150 `[4]`.
pub const DONE_LABEL: &str = "Done";
/// Find's label, stock `STR#` 150 `[59]`.
pub const FIND_LABEL: &str = "Find";

/// Which button a click activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapButton {
    /// Done: close the map, as Return does.
    Done,
    /// Find: open the Find dialog, as F and S do.
    Find,
}

/// The two buttons, each with its tracker.
#[derive(Clone, Debug, PartialEq)]
pub struct MapButtons {
    done: Button,
    find: Button,
    done_tracker: ButtonTracker,
    find_tracker: ButtonTracker,
}

impl Default for MapButtons {
    fn default() -> Self {
        Self::new()
    }
}

impl MapButtons {
    /// Done and Find, placed from [`PANEL_ORIGIN`], enabled, unpressed.
    #[must_use]
    pub fn new() -> Self {
        Self {
            done: Button::new(placed(DONE_ITEM, PANEL_ORIGIN), DONE_LABEL),
            find: Button::new(placed(FIND_ITEM, PANEL_ORIGIN), FIND_LABEL),
            done_tracker: ButtonTracker::default(),
            find_tracker: ButtonTracker::default(),
        }
    }

    /// Done.
    #[must_use]
    pub fn done(&self) -> &Button {
        &self.done
    }

    /// Find.
    #[must_use]
    pub fn find(&self) -> &Button {
        &self.find
    }

    /// Tracks `input` over both buttons: the button a release over it
    /// activates, if any. Silent.
    pub fn input(&mut self, input: &Input) -> Option<MapButton> {
        let done = self.done_tracker.input(&self.done, input);
        let find = self.find_tracker.input(&self.find, input);
        self.done_tracker.take_sound();
        self.find_tracker.take_sound();
        if done {
            Some(MapButton::Done)
        } else if find {
            Some(MapButton::Find)
        } else {
            None
        }
    }

    /// Abandons a press on either, silently.
    pub fn cancel_pointer(&mut self) {
        self.done_tracker.cancel_pointer();
        self.find_tracker.cancel_pointer();
    }

    /// Draws both with Nova's button pictures, their labels set in `style`
    /// and measured by `metrics`, or centred on their point unmeasured
    /// without them; each pressed while armed with the pointer on it.
    pub fn draw(
        &self,
        style: &ButtonStyle,
        metrics: Option<&dyn TextMetrics>,
        list: &mut DrawList,
    ) {
        let metrics: &dyn TextMetrics = metrics.unwrap_or(&Unmeasured);
        for (button, tracker) in [
            (&self.done, &self.done_tracker),
            (&self.find, &self.find_tracker),
        ] {
            button.draw(tracker.pressed(), &ButtonSkin::NOVA, style, &metrics, list);
        }
    }
}

/// Text measured as taking no room, so a label starts at its button's
/// centre.
struct Unmeasured;

impl TextMetrics for Unmeasured {
    fn width(&self, _font: Font, _size: f32, _text: &str) -> f32 {
        0.0
    }

    fn line_height(&self, _font: Font, _size: f32) -> f32 {
        0.0
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::Point;
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;

    fn button(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn click(buttons: &mut MapButtons, from: Point, to: Point) -> Option<MapButton> {
        assert_eq!(buttons.input(&button(true, from)), None, "a press");
        buttons.input(&Input::PointerMoved(to));
        buttons.input(&button(false, to))
    }

    fn labels(list: &DrawList) -> Vec<(String, Point)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } => Some((text.clone(), *origin)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn they_are_done_and_find_both_enabled() {
        let buttons = MapButtons::new();
        assert_eq!(buttons.done().label, DONE_LABEL);
        assert_eq!(buttons.find().label, FIND_LABEL);
        assert!(buttons.done().enabled && buttons.find().enabled);
        assert_eq!(MapButtons::default(), buttons);
    }

    #[test]
    fn a_release_over_a_button_pressed_activates_it() {
        let mut buttons = MapButtons::new();
        let (done, find) = (buttons.done().rect.center(), buttons.find().rect.center());
        assert_eq!(click(&mut buttons, done, done), Some(MapButton::Done));
        assert_eq!(click(&mut buttons, find, find), Some(MapButton::Find));
    }

    #[test]
    fn a_release_elsewhere_or_over_the_other_button_activates_nothing() {
        let mut buttons = MapButtons::new();
        let (done, find) = (buttons.done().rect.center(), buttons.find().rect.center());
        let away = Point::new(10.0, 10.0);
        assert_eq!(click(&mut buttons, done, away), None);
        assert_eq!(click(&mut buttons, done, find), None);
        assert_eq!(click(&mut buttons, find, done), None);
        assert_eq!(click(&mut buttons, away, done), None);
        assert_eq!(buttons.input(&button(true, done)), None);
        buttons.cancel_pointer();
        assert_eq!(buttons.input(&button(false, done)), None, "cancelled");
    }

    #[test]
    fn they_draw_novas_pictures_with_their_labels_pressed_while_armed() {
        let mut buttons = MapButtons::new();
        let mut list = DrawList::new();
        buttons.draw(&ButtonStyle::STOCK, Some(&MonoMetrics), &mut list);
        let mut expected = DrawList::new();
        for button in [buttons.done(), buttons.find()] {
            button.draw(
                false,
                &ButtonSkin::NOVA,
                &ButtonStyle::STOCK,
                &MonoMetrics,
                &mut expected,
            );
        }
        assert_eq!(list, expected);
        let done = buttons.done().rect.center();
        buttons.input(&button(true, done));
        let mut list = DrawList::new();
        buttons.draw(&ButtonStyle::STOCK, Some(&MonoMetrics), &mut list);
        let mut expected = DrawList::new();
        buttons.done().draw(
            true,
            &ButtonSkin::NOVA,
            &ButtonStyle::STOCK,
            &MonoMetrics,
            &mut expected,
        );
        buttons.find().draw(
            false,
            &ButtonSkin::NOVA,
            &ButtonStyle::STOCK,
            &MonoMetrics,
            &mut expected,
        );
        assert_eq!(list, expected, "Done pressed");
    }

    #[test]
    fn without_metrics_a_label_starts_at_its_buttons_centre() {
        let buttons = MapButtons::new();
        let mut list = DrawList::new();
        buttons.draw(&ButtonStyle::STOCK, None, &mut list);
        assert_eq!(
            labels(&list),
            [
                (DONE_LABEL.to_owned(), buttons.done().rect.center()),
                (FIND_LABEL.to_owned(), buttons.find().rect.center()),
            ]
        );
    }
}
