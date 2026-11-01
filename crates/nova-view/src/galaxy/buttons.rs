//! The in-flight maps' six buttons, as `_DrawMapButtons` (@0x2a86e-0x2aaf9)
//! draws them and `_TrackMapButtons` (@0x2aafa-0x2ade5) tracks them.
//!
//! Each is a DITL 2000 user item, placed from the dialog's top-left corner
//! ([`MapButtons::new`]). In `_DrawMapButtons`' order (@0x2a981-0x2a9d5):
//!
//! - Done, item 1, `STR#` 150 `[4]` "Done" (0-based): always enabled;
//! - zoom out, item 4, `[16]` "-", an ASCII hyphen-minus, drawn as
//!   `_DrawButton`'s pen glyph: while `_canZoomOut`;
//! - zoom in, item 5, `[17]` "+", the same: while `_canZoomIn`;
//! - Clear Route, item 8, `[48]` "Clear Route": while
//!   `showClearRouteButton`;
//! - Borders, item 9, `[55]` "Show Borders" while the borders are off and
//!   `[56]` "Hide Borders" while they are on (@0x2a9d8-0x2a9e8): always;
//! - Find, item 10, `[59]` "Find": always.
//!
//! The original reads the flags at each draw and each track, so ours
//! builds the buttons afresh from a [`ButtonState`] each time. A disabled
//! button is drawn with the disabled pictures and the grey label
//! (`DrawButton` state −1, @0x2aa0a-0x2aadd).
//!
//! `_TrackMapButtons` arms the enabled button under a press, shows it
//! pressed while the pointer stays on it, and gives it only for a release
//! over it (@0x2ada3). A press on a disabled button, or on none, arms
//! nothing (@0x2acb5-0x2acb9), so nothing it is released over acts. The
//! press, the pointer while held and the release are tested with
//! `PtInRect` (@0x2ac03-0x2acac, @0x2acf0-0x2ad94), half-open as
//! [`ButtonTracker`] tests them: a button's right and bottom edge pixels
//! are off it. It plays no sound, so the trackers' button sounds are
//! dropped.
//! `_MapFilter` maps the six to items 1, 4, 5, 8, 9 and 10
//! (@0x1457a-0x145e6). What each does is the map's
//! ([`GalaxyMap`](super::GalaxyMap)): Clear Route, like C, asks the
//! owner to clear the route (item 8, `_DoSystemMap` @0x12c69-0x12dbf).

use super::panel::placed;
use super::view::Bounds;
use crate::draw::DrawList;
use crate::flight::radar::IRect;
use crate::font::Font;
use crate::geometry::Point;
use crate::input::Input;
use crate::text::TextMetrics;
use crate::ui::button::{Button, ButtonSkin, ButtonStyle, ButtonTracker};

/// DITL 2000 item 1, Done, in dialog coordinates (t 483, l 483, b 508,
/// r 582).
pub const DONE_ITEM: IRect = IRect::new(483, 483, 582, 508);
/// DITL 2000 item 4, zoom out "-" (t 483, l 408, b 508, r 433).
pub const ZOOM_OUT_ITEM: IRect = IRect::new(408, 483, 433, 508);
/// DITL 2000 item 5, zoom in "+" (t 483, l 438, b 508, r 463).
pub const ZOOM_IN_ITEM: IRect = IRect::new(438, 483, 463, 508);
/// DITL 2000 item 8, Clear Route (t 483, l 155, b 508, r 275).
pub const CLEAR_ROUTE_ITEM: IRect = IRect::new(155, 483, 275, 508);
/// DITL 2000 item 9, Show/Hide Borders (t 483, l 11, b 508, r 141).
pub const BORDERS_ITEM: IRect = IRect::new(11, 483, 141, 508);
/// DITL 2000 item 10, Find, in dialog coordinates (t 483, l 288, b 508,
/// r 387).
pub const FIND_ITEM: IRect = IRect::new(288, 483, 387, 508);
/// Done's label, stock `STR#` 150 `[4]`.
pub const DONE_LABEL: &str = "Done";
/// Zoom out's label, `[16]`: an ASCII hyphen-minus.
pub const ZOOM_OUT_LABEL: &str = "-";
/// Zoom in's label, `[17]`.
pub const ZOOM_IN_LABEL: &str = "+";
/// Clear Route's label, `[48]`.
pub const CLEAR_ROUTE_LABEL: &str = "Clear Route";
/// Borders' label while the borders are off, `[55]`.
pub const SHOW_BORDERS_LABEL: &str = "Show Borders";
/// Borders' label while the borders are on, `[56]`.
pub const HIDE_BORDERS_LABEL: &str = "Hide Borders";
/// Find's label, stock `STR#` 150 `[59]`.
pub const FIND_LABEL: &str = "Find";

/// Which button a click activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapButton {
    /// Done, item 1: close the map, as Return does.
    Done,
    /// "-", item 4: one zoom step out.
    ZoomOut,
    /// "+", item 5: one zoom step in.
    ZoomIn,
    /// Clear Route, item 8.
    ClearRoute,
    /// Show/Hide Borders, item 9.
    Borders,
    /// Find, item 10: open the Find dialog, as F and S do.
    Find,
}

/// The six in `_DrawMapButtons`' order.
pub const ORDER: [MapButton; 6] = [
    MapButton::Done,
    MapButton::ZoomOut,
    MapButton::ZoomIn,
    MapButton::ClearRoute,
    MapButton::Borders,
    MapButton::Find,
];

impl MapButton {
    /// Its DITL 2000 item's rect, in dialog coordinates.
    #[must_use]
    pub fn item(self) -> IRect {
        match self {
            Self::Done => DONE_ITEM,
            Self::ZoomOut => ZOOM_OUT_ITEM,
            Self::ZoomIn => ZOOM_IN_ITEM,
            Self::ClearRoute => CLEAR_ROUTE_ITEM,
            Self::Borders => BORDERS_ITEM,
            Self::Find => FIND_ITEM,
        }
    }
}

/// The flags `_DrawMapButtons` and `_TrackMapButtons` read.
#[allow(clippy::struct_excessive_bools)] // The original's four flags, as it reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonState {
    /// `_canZoomOut`: "-" is enabled.
    pub can_zoom_out: bool,
    /// `_canZoomIn`: "+" is enabled.
    pub can_zoom_in: bool,
    /// `showClearRouteButton`: Clear Route is enabled.
    pub clear_route: bool,
    /// Whether the borders are shown (`_settings`+0x13): Borders reads
    /// "Hide Borders".
    pub borders_shown: bool,
}

impl ButtonState {
    /// Whether `button` is enabled in this state.
    #[must_use]
    pub fn enabled(&self, button: MapButton) -> bool {
        match button {
            MapButton::Done | MapButton::Borders | MapButton::Find => true,
            MapButton::ZoomOut => self.can_zoom_out,
            MapButton::ZoomIn => self.can_zoom_in,
            MapButton::ClearRoute => self.clear_route,
        }
    }

    /// `button`'s label in this state.
    #[must_use]
    pub fn label(&self, button: MapButton) -> &'static str {
        match button {
            MapButton::Done => DONE_LABEL,
            MapButton::ZoomOut => ZOOM_OUT_LABEL,
            MapButton::ZoomIn => ZOOM_IN_LABEL,
            MapButton::ClearRoute => CLEAR_ROUTE_LABEL,
            MapButton::Borders if self.borders_shown => HIDE_BORDERS_LABEL,
            MapButton::Borders => SHOW_BORDERS_LABEL,
            MapButton::Find => FIND_LABEL,
        }
    }
}

/// The six buttons' trackers, placed from the dialog's top-left corner.
#[derive(Clone, Debug, PartialEq)]
pub struct MapButtons {
    origin: Point,
    /// One for each button, in [`ORDER`].
    trackers: [ButtonTracker; 6],
}

impl MapButtons {
    /// The six, placed with the dialog's top-left corner at `origin`,
    /// unpressed.
    #[must_use]
    pub fn new(origin: Point) -> Self {
        Self {
            origin,
            trackers: [ButtonTracker::default(); 6],
        }
    }

    /// Where `button` is on the screen.
    #[must_use]
    pub fn rect(&self, button: MapButton) -> Bounds {
        placed(button.item(), self.origin)
    }

    /// `button` as `state` has it: placed, labelled and enabled.
    #[must_use]
    pub fn button(&self, button: MapButton, state: &ButtonState) -> Button {
        Button {
            enabled: state.enabled(button),
            ..Button::new(self.rect(button), state.label(button))
        }
    }

    /// All six in `_DrawMapButtons`' order, as `state` has them.
    #[must_use]
    pub fn buttons(&self, state: &ButtonState) -> [Button; 6] {
        ORDER.map(|button| self.button(button, state))
    }

    /// Tracks `input` over the six as `state` has them: the button a
    /// release over it activates, if any. Silent.
    pub fn input(&mut self, input: &Input, state: &ButtonState) -> Option<MapButton> {
        let buttons = self.buttons(state);
        let mut activated = None;
        for ((which, button), tracker) in ORDER.into_iter().zip(&buttons).zip(&mut self.trackers) {
            if tracker.input(button, input) {
                activated = activated.or(Some(which));
            }
            tracker.take_sound();
        }
        activated
    }

    /// Abandons a press on any, silently.
    pub fn cancel_pointer(&mut self) {
        for tracker in &mut self.trackers {
            tracker.cancel_pointer();
        }
    }

    /// Draws the six as `state` has them with Nova's button pictures,
    /// their labels set in `style` and measured by `metrics`, or centred
    /// on their point unmeasured without them; each pressed while armed
    /// with the pointer on it.
    pub fn draw(
        &self,
        state: &ButtonState,
        style: &ButtonStyle,
        metrics: Option<&dyn TextMetrics>,
        list: &mut DrawList,
    ) {
        let metrics: &dyn TextMetrics = metrics.unwrap_or(&Unmeasured);
        for (button, tracker) in self.buttons(state).iter().zip(&self.trackers) {
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
    use crate::color::Color;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::button::ButtonImages;

    /// The map's dialog's top-left corner, (212, 128).
    const ORIGIN: Point = Point::new(212.0, 128.0);

    /// Everything enabled, borders shown.
    const ALL: ButtonState = ButtonState {
        can_zoom_out: true,
        can_zoom_in: true,
        clear_route: true,
        borders_shown: true,
    };

    /// Zoom and Clear Route disabled, borders hidden.
    const NONE: ButtonState = ButtonState {
        can_zoom_out: false,
        can_zoom_in: false,
        clear_route: false,
        borders_shown: false,
    };

    fn button(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn click(
        buttons: &mut MapButtons,
        state: ButtonState,
        from: Point,
        to: Point,
    ) -> Option<MapButton> {
        assert_eq!(buttons.input(&button(true, from), &state), None, "a press");
        buttons.input(&Input::PointerMoved(to), &state);
        buttons.input(&button(false, to), &state)
    }

    fn labels(list: &DrawList) -> Vec<(String, Point)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } => Some((text.clone(), *origin)),
                _ => None,
            })
            .collect()
    }

    fn bounds(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
        Bounds {
            min: Point::new(left, top),
            max: Point::new(right, bottom),
        }
    }

    #[test]
    fn each_button_is_its_ditl_item_placed_from_the_dialogs_corner() {
        let buttons = MapButtons::new(ORIGIN);
        let rects = ORDER.map(|which| buttons.rect(which));
        assert_eq!(
            rects,
            [
                bounds(695.0, 611.0, 794.0, 636.0),
                bounds(620.0, 611.0, 645.0, 636.0),
                bounds(650.0, 611.0, 675.0, 636.0),
                bounds(367.0, 611.0, 487.0, 636.0),
                bounds(223.0, 611.0, 353.0, 636.0),
                bounds(500.0, 611.0, 599.0, 636.0),
            ],
            "Done, -, +, Clear Route, Borders, Find"
        );
        let items = ORDER.map(MapButton::item);
        assert_eq!(
            items,
            [
                IRect::new(483, 483, 582, 508),
                IRect::new(408, 483, 433, 508),
                IRect::new(438, 483, 463, 508),
                IRect::new(155, 483, 275, 508),
                IRect::new(11, 483, 141, 508),
                IRect::new(288, 483, 387, 508),
            ]
        );
        let elsewhere = MapButtons::new(Point::new(0.0, 0.0));
        assert_eq!(
            elsewhere.rect(MapButton::Done),
            bounds(483.0, 483.0, 582.0, 508.0)
        );
    }

    #[test]
    fn the_labels_are_str_150s_with_borders_by_whether_they_are_shown() {
        let buttons = MapButtons::new(ORIGIN);
        let labels = |state: &ButtonState| buttons.buttons(state).map(|button| button.label);
        assert_eq!(
            labels(&ALL),
            ["Done", "-", "+", "Clear Route", "Hide Borders", "Find"]
        );
        assert_eq!(labels(&NONE)[4], "Show Borders");
    }

    #[test]
    fn done_borders_and_find_are_always_enabled_and_the_rest_follow_their_flags() {
        let buttons = MapButtons::new(ORIGIN);
        let enabled = |state: &ButtonState| buttons.buttons(state).map(|button| button.enabled);
        assert_eq!(enabled(&ALL), [true; 6]);
        assert_eq!(enabled(&NONE), [true, false, false, false, true, true]);
        for (state, expected) in [
            (
                ButtonState {
                    can_zoom_out: true,
                    ..NONE
                },
                [true, true, false, false, true, true],
            ),
            (
                ButtonState {
                    can_zoom_in: true,
                    ..NONE
                },
                [true, false, true, false, true, true],
            ),
            (
                ButtonState {
                    clear_route: true,
                    ..NONE
                },
                [true, false, false, true, true, true],
            ),
        ] {
            assert_eq!(enabled(&state), expected, "{state:?}");
        }
    }

    /// The left caps `list` stretches, in order: one a button, whatever
    /// its width (the 25-wide "-" and "+" have no middle).
    fn pictures(list: &DrawList) -> Vec<i16> {
        let skin = ButtonSkin::NOVA;
        let lefts = [skin.normal, skin.pressed, skin.disabled].map(|images| images.left.id);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::StretchedPicture { image, .. } if lefts.contains(&image.id) => {
                    Some(image.id)
                }
                _ => None,
            })
            .collect()
    }

    fn ids(images: &ButtonImages) -> i16 {
        images.left.id
    }

    #[test]
    fn a_disabled_button_draws_its_disabled_pictures_and_grey_label() {
        let buttons = MapButtons::new(ORIGIN);
        let mut list = DrawList::new();
        buttons.draw(&NONE, &ButtonStyle::STOCK, Some(&MonoMetrics), &mut list);
        let normal = ids(&ButtonSkin::NOVA.normal);
        let disabled = ids(&ButtonSkin::NOVA.disabled);
        assert_eq!(
            pictures(&list),
            [normal, disabled, disabled, disabled, normal, normal]
        );
        let (up, grey) = (ButtonStyle::STOCK.up, ButtonStyle::STOCK.grey);
        let colours = |wanted: fn(&DrawCommand) -> Option<Color>| {
            list.iter().filter_map(wanted).collect::<Vec<_>>()
        };
        let text = colours(|command| match command {
            DrawCommand::Text { color, .. } => Some(*color),
            _ => None,
        });
        assert_eq!(text, [up, grey, up, up], "Done, Clear Route, Borders, Find");
        let glyphs = colours(|command| match command {
            DrawCommand::Line { color, .. } => Some(*color),
            _ => None,
        });
        assert_eq!(glyphs, [grey, grey, grey], "- is one stroke and + two");
    }

    #[test]
    fn a_press_and_release_over_an_enabled_button_activates_it() {
        let mut buttons = MapButtons::new(ORIGIN);
        for which in ORDER {
            let at = buttons.rect(which).center();
            assert_eq!(click(&mut buttons, ALL, at, at), Some(which), "{which:?}");
        }
    }

    #[test]
    fn a_release_off_it_over_another_or_after_a_cancel_activates_nothing() {
        let mut buttons = MapButtons::new(ORIGIN);
        let away = Point::new(10.0, 10.0);
        for which in ORDER {
            let at = buttons.rect(which).center();
            assert_eq!(click(&mut buttons, ALL, at, away), None, "{which:?} off");
            for other in ORDER.into_iter().filter(|&other| other != which) {
                let there = buttons.rect(other).center();
                assert_eq!(
                    click(&mut buttons, ALL, at, there),
                    None,
                    "{which:?} over {other:?}"
                );
            }
            assert_eq!(buttons.input(&button(true, at), &ALL), None);
            buttons.cancel_pointer();
            assert_eq!(
                buttons.input(&button(false, at), &ALL),
                None,
                "{which:?} cancelled"
            );
        }
        let done = buttons.rect(MapButton::Done).center();
        assert_eq!(click(&mut buttons, ALL, away, done), None, "pressed off");
    }

    #[test]
    fn a_press_on_a_disabled_button_arms_nothing() {
        let mut buttons = MapButtons::new(ORIGIN);
        for which in [MapButton::ZoomOut, MapButton::ZoomIn, MapButton::ClearRoute] {
            let at = buttons.rect(which).center();
            assert_eq!(click(&mut buttons, NONE, at, at), None, "{which:?}");
            let done = buttons.rect(MapButton::Done).center();
            assert_eq!(
                click(&mut buttons, NONE, at, done),
                None,
                "{which:?} released over Done"
            );
        }
        let plus = buttons.rect(MapButton::ZoomIn).center();
        buttons.input(&button(true, plus), &NONE);
        assert_eq!(
            buttons.input(&button(false, plus), &ALL),
            None,
            "enabled only by the release"
        );
    }

    #[test]
    fn an_armed_button_draws_pressed_only_while_the_pointer_is_over_it() {
        let mut buttons = MapButtons::new(ORIGIN);
        let find = buttons.rect(MapButton::Find).center();
        buttons.input(&button(true, find), &ALL);
        let drawn = |buttons: &MapButtons| {
            let mut list = DrawList::new();
            buttons.draw(&ALL, &ButtonStyle::STOCK, Some(&MonoMetrics), &mut list);
            pictures(&list)
        };
        let normal = ids(&ButtonSkin::NOVA.normal);
        let pressed = ids(&ButtonSkin::NOVA.pressed);
        let with_find = |find: i16| vec![normal, normal, normal, normal, normal, find];
        assert_eq!(drawn(&buttons), with_find(pressed));
        buttons.input(&Input::PointerMoved(Point::new(10.0, 10.0)), &ALL);
        assert_eq!(drawn(&buttons), with_find(normal), "off it");
        buttons.input(&Input::PointerMoved(find), &ALL);
        assert_eq!(drawn(&buttons), with_find(pressed), "back on it");
    }

    /// `_TrackMapButtons` tests the press, the pointer while held and the
    /// release with `PtInRect` (@0x2ac03-0x2acac, @0x2acf0-0x2ad94): the
    /// left and top edges are on a button.
    #[test]
    fn a_press_and_release_on_a_buttons_left_or_top_edge_pixel_acts() {
        let mut buttons = MapButtons::new(ORIGIN);
        for which in ORDER {
            let r = buttons.rect(which);
            let c = r.center();
            for at in [Point::new(r.min.x, c.y), Point::new(c.x, r.min.y)] {
                assert_eq!(
                    click(&mut buttons, ALL, at, at),
                    Some(which),
                    "{which:?} at {at:?}"
                );
            }
        }
    }

    /// ... and the right and bottom edges are off it. None of the six
    /// share an edge.
    #[test]
    fn a_press_and_release_on_a_buttons_right_or_bottom_edge_pixel_does_not_act() {
        let mut buttons = MapButtons::new(ORIGIN);
        for which in ORDER {
            let r = buttons.rect(which);
            let c = r.center();
            for at in [Point::new(r.max.x, c.y), Point::new(c.x, r.max.y)] {
                assert_eq!(
                    click(&mut buttons, ALL, at, at),
                    None,
                    "{which:?} at {at:?}"
                );
            }
            for at in [
                Point::new(r.max.x - 1.0, c.y),
                Point::new(c.x, r.max.y - 1.0),
            ] {
                assert_eq!(
                    click(&mut buttons, ALL, at, at),
                    Some(which),
                    "{which:?} a pixel inside, at {at:?}"
                );
            }
        }
    }

    #[test]
    fn dragging_onto_a_buttons_right_or_bottom_edge_is_off_it() {
        let mut buttons = MapButtons::new(ORIGIN);
        let r = buttons.rect(MapButton::Find);
        let c = r.center();
        let drawn = |buttons: &MapButtons| {
            let mut list = DrawList::new();
            buttons.draw(&ALL, &ButtonStyle::STOCK, Some(&MonoMetrics), &mut list);
            pictures(&list)
        };
        let normal = ids(&ButtonSkin::NOVA.normal);
        for edge in [Point::new(r.max.x, c.y), Point::new(c.x, r.max.y)] {
            buttons.input(&button(true, c), &ALL);
            buttons.input(&Input::PointerMoved(edge), &ALL);
            assert_eq!(drawn(&buttons), vec![normal; 6], "on {edge:?}");
            assert_eq!(
                buttons.input(&button(false, edge), &ALL),
                None,
                "released on {edge:?}"
            );
        }
    }

    #[test]
    fn they_draw_each_button_as_the_buttons_own_draw_does() {
        let buttons = MapButtons::new(ORIGIN);
        let mut list = DrawList::new();
        buttons.draw(&ALL, &ButtonStyle::STOCK, Some(&MonoMetrics), &mut list);
        let mut expected = DrawList::new();
        for button in buttons.buttons(&ALL) {
            button.draw(
                false,
                &ButtonSkin::NOVA,
                &ButtonStyle::STOCK,
                &MonoMetrics,
                &mut expected,
            );
        }
        assert_eq!(list, expected);
    }

    #[test]
    fn without_metrics_a_label_starts_at_its_buttons_centre() {
        let buttons = MapButtons::new(ORIGIN);
        let mut list = DrawList::new();
        buttons.draw(&ALL, &ButtonStyle::STOCK, None, &mut list);
        let expected: Vec<(String, Point)> = ORDER
            .iter()
            .filter(|&&which| !matches!(which, MapButton::ZoomOut | MapButton::ZoomIn))
            .map(|&which| (ALL.label(which).to_owned(), buttons.rect(which).center()))
            .collect();
        assert_eq!(labels(&list), expected);
    }
}
