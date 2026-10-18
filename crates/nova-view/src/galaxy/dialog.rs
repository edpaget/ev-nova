//! The in-flight maps' dialog: `DLOG`/`DITL` 2000 "Map", as
//! `_DoSystemMap` (@0x121a3-0x13c62) opens it and `_MapDialogUpdate`
//! draws it.
//!
//! **Where it sits.** `_CenterWindow` (@0x12534) puts the 601 × 513 dialog
//! ([`DIALOG_SIZE`]) at the window's centre less half its size, each half
//! truncated: [`DIALOG_ORIGIN`], (212, 128) on the 1024 × 768 screen, a
//! pixel right of and below where the Window Manager's centring would
//! ([`center_window`]). Every item is a user item, at its DITL offset from
//! there: the map, item 3 ([`MAP_ITEM`]); the two info panels, items 2
//! and 6 ([`panel`](super::panel)); and the six buttons
//! ([`buttons`](super::buttons)).
//!
//! **The frame** (`_MapDialogUpdate` @0xf609-0xf833), in order
//! ([`draw_frame`]):
//!
//! 1. the dialog painted black (@0xf649);
//! 2. `PICT` [`BACKGROUND`] (8509, "Map") stretched over it (@0xf67c),
//!    which, its frame being the dialog's own size, changes nothing;
//! 3. the six buttons (`_DrawMapButtons(−1)` @0xf6d9);
//! 4. item 3 painted black (@0xf833).
//!
//! Then the map: the borders' cells ([`borders`](super::borders)), its
//! nebulae, each clipped to its rect's overlap with item 3
//! ([`nebulae`](super::nebulae), [`MapDialog::nebula_frame`]), and what
//! `_DrawMap` draws, clipped to item 3 (`CopyBits` @0x100a2-0x10123,
//! [`clip`](super::clip)), the scroll at item 3's centre
//! ([`map_centre`]); then the panels. The second `_DrawMapButtons` after
//! the panels (@0x111dc) draws the same pixels as the first, and is not
//! drawn again.
//!
//! **Pointer.** A press in item 3 (`PtInRect`, half-open:
//! [`in_map_item`]) is the map's (@0x14552); anywhere else is the
//! buttons'. A press in the map that moves drags it ([`MapDialog`]), and
//! while it drags item 3 shows the partial update
//! ([`update`](super::update)): each poll the pointer's whole pixel moved
//! on draws one, and the release draws the full update only when the
//! scroll moved from the drag's start.

use super::buttons::{ButtonState, MapButtons};
use super::nebulae::NebulaFrame;
use super::panel::placed;
use super::place::{ARROW_STEP, MapPlace};
use super::update::MapUpdate;
use super::view::{Bounds, Transform};
use crate::draw::fill_rect;
use crate::flight::radar::IRect;
use crate::text::TextMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::dialog::center_window;
use crate::{Color, DrawList, ImageKey, Key, Point};

/// The map's `DLOG` and `DITL`, "Map" (`_DoSystemMap`'s `GetNewDialog`,
/// @0x121e5).
pub const MAP_DIALOG: i16 = 2000;
/// `DLOG` 2000's size: its bounds (t −150, l 3, b 363, r 604).
pub const DIALOG_SIZE: (f32, f32) = (601.0, 513.0);
/// Where the dialog's top-left corner is on the 1024 × 768 screen
/// ([`SCREEN`](crate::ui::dialog::SCREEN)):
/// [`dialog_origin`]'s, (512 − 300, 384 − 256).
pub const DIALOG_ORIGIN: Point = Point::new(212.0, 128.0);
/// DITL 2000 item 3, the map, in dialog coordinates (t 8, l 9, b 428,
/// r 467).
pub const MAP_ITEM: IRect = IRect::new(9, 8, 467, 428);
/// `_mapBackgroundPic`, `PICT` 8509 "Map" (@0x12502).
pub const BACKGROUND: i16 = 8509;

/// Where `_CenterWindow` puts the dialog's top-left corner in a `window`
/// of that size ([`center_window`]).
#[must_use]
pub fn dialog_origin(window: (f32, f32)) -> Point {
    center_window(window, DIALOG_SIZE)
}

/// The dialog on the screen.
#[must_use]
pub fn dialog_area() -> Bounds {
    Bounds::at(DIALOG_ORIGIN, DIALOG_SIZE.0, DIALOG_SIZE.1)
}

/// Item 3 on the screen: (221, 136)-(679, 556).
#[must_use]
pub fn map_area() -> Bounds {
    placed(MAP_ITEM, DIALOG_ORIGIN)
}

/// Item 3's centre as `_DrawMap` takes it, left + trunc((right − left) /
/// 2), top + trunc((bottom − top) / 2) (@0x12e23-0x12e5f): (450, 346),
/// where the scroll is drawn.
#[allow(clippy::cast_precision_loss)] // Dialog pixels.
#[must_use]
pub fn map_centre() -> Point {
    let (x, y) = MAP_ITEM_CENTRE;
    Point::new(DIALOG_ORIGIN.x + x as f32, DIALOG_ORIGIN.y + y as f32)
}

/// Item 3's centre in dialog coordinates, (238, 218): [`map_centre`]
/// less the dialog's origin.
pub const MAP_ITEM_CENTRE: (i32, i32) = (
    MAP_ITEM.left + (MAP_ITEM.right - MAP_ITEM.left) / 2,
    MAP_ITEM.top + (MAP_ITEM.bottom - MAP_ITEM.top) / 2,
);

/// Whether `at` is in item 3, as `PtInRect` tests it: its left and top
/// edges in, its right and bottom edges out.
#[must_use]
pub fn in_map_item(at: Point) -> bool {
    let area = map_area();
    (area.min.x..area.max.x).contains(&at.x) && (area.min.y..area.max.y).contains(&at.y)
}

/// Draws the frame under the map (see the module docs): the dialog
/// black, [`BACKGROUND`] stretched over it, `buttons` as `state` has
/// them, then item 3 black.
pub fn draw_frame(
    list: &mut DrawList,
    buttons: &MapButtons,
    state: &ButtonState,
    style: &ButtonStyle,
    metrics: Option<&dyn TextMetrics>,
) {
    let dialog = dialog_area();
    fill_rect(list, dialog, Color::BLACK);
    list.stretched_picture(
        ImageKey::picture(BACKGROUND),
        dialog.min,
        dialog.width(),
        dialog.height(),
    );
    buttons.draw(state, style, metrics, list);
    fill_rect(list, map_area(), Color::BLACK);
}

/// A drag in progress: the scroll and the pointer at the press, as
/// `_DoSystemMap` keeps them (@0x137e9-0x1394d), and the whole pixel the
/// pointer was at when last polled.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Drag {
    scroll: (i32, i32),
    from: Point,
    last: (i32, i32),
}

/// `at` as `GetMouse` gives it: the whole pixel it is in.
#[allow(clippy::cast_possible_truncation)] // Screen pixels.
fn whole_pixel(at: Point) -> (i32, i32) {
    (at.x.floor() as i32, at.y.floor() as i32)
}

/// What the in-flight maps keep of the dialog: where they look
/// ([`MapPlace`]), the buttons' trackers, the borders setting their
/// Borders button reads, `showClearRouteButton`, which enables Clear
/// Route, a drag in progress, and which update item 3 shows.
#[derive(Clone, Debug, PartialEq)]
pub struct MapDialog {
    place: MapPlace,
    buttons: MapButtons,
    borders_shown: bool,
    /// `showClearRouteButton`: whether Clear Route is enabled.
    clear_route: bool,
    drag: Option<Drag>,
    /// The update item 3 shows (see [`update`](super::update)).
    update: MapUpdate,
    /// Whether item 3 was redrawn since this was last taken.
    fresh: bool,
}

impl MapDialog {
    /// The dialog looking at `scroll` at the first zoom of a run, with
    /// the borders shown (`ShowMapBorders`' default with no preference,
    /// `_LoadPrefs` @0xced57-0xced93).
    #[must_use]
    pub fn new(scroll: (i32, i32)) -> Self {
        Self {
            place: MapPlace::new(scroll),
            buttons: MapButtons::new(DIALOG_ORIGIN),
            borders_shown: true,
            clear_route: false,
            drag: None,
            update: MapUpdate::Full,
            fresh: false,
        }
    }

    /// Where it looks.
    #[must_use]
    pub fn place(&self) -> &MapPlace {
        &self.place
    }

    /// Where it looks, to change.
    pub fn place_mut(&mut self) -> &mut MapPlace {
        &mut self.place
    }

    /// The buttons' trackers.
    #[must_use]
    pub fn buttons(&self) -> &MapButtons {
        &self.buttons
    }

    /// The buttons' trackers, to track with.
    pub fn buttons_mut(&mut self) -> &mut MapButtons {
        &mut self.buttons
    }

    /// Whether the borders are shown, as the Borders button reads it.
    #[must_use]
    pub fn borders_shown(&self) -> bool {
        self.borders_shown
    }

    /// Sets whether the borders are shown.
    pub fn set_borders_shown(&mut self, shown: bool) {
        self.borders_shown = shown;
    }

    /// Whether Clear Route is enabled (`showClearRouteButton`): off as
    /// the dialog is made.
    #[must_use]
    pub fn clear_route(&self) -> bool {
        self.clear_route
    }

    /// Sets `showClearRouteButton`.
    pub fn set_clear_route(&mut self, enabled: bool) {
        self.clear_route = enabled;
    }

    /// The buttons' flags: the zoom's, `showClearRouteButton`, and the
    /// borders setting.
    #[must_use]
    pub fn state(&self) -> ButtonState {
        ButtonState {
            can_zoom_out: self.place.can_zoom_out(),
            can_zoom_in: self.place.can_zoom_in(),
            clear_route: self.clear_route,
            borders_shown: self.borders_shown,
        }
    }

    /// The map's transform: the scroll at item 3's centre.
    #[must_use]
    pub fn transform(&self) -> Transform {
        self.place.transform(map_centre())
    }

    /// A key press or repeat (`_MapFilter`): the arrows scroll
    /// [`ARROW_STEP`] units (@0x142a3, @0x142c3, @0x14370, @0x14417),
    /// `-` is item 4 and `=` or `+` item 5, each stepping only while its
    /// flag allows (@0x14271-0x14293). Each arrow redraws item 3 (left and
    /// right @0x142fc, up @0x143aa, down @0x1445a), and so does a zoom
    /// step taken (@0x128d9, @0x12b0c); a refused one redraws nothing
    /// (@0x12803-0x12816). Whether it took the key.
    pub fn key(&mut self, key: Key) -> bool {
        let redraw = match key {
            Key::Left => self.scroll_by(-ARROW_STEP, 0),
            Key::Right => self.scroll_by(ARROW_STEP, 0),
            Key::Up => self.scroll_by(0, -ARROW_STEP),
            Key::Down => self.scroll_by(0, ARROW_STEP),
            Key::Char('-') => self.place.zoom_out(),
            Key::Char('=' | '+') => self.place.zoom_in(),
            _ => return false,
        };
        if redraw {
            self.redraw();
        }
        true
    }

    /// Moves the scroll by (`dx`, `dy`): an arrow, which always redraws.
    fn scroll_by(&mut self, dx: i32, dy: i32) -> bool {
        self.place.scroll_by(dx, dy);
        true
    }

    /// Starts a drag from a press at `at`, replacing any whose release
    /// was lost. It redraws nothing.
    pub fn press(&mut self, at: Point) {
        self.drag = Some(Drag {
            scroll: self.place.scroll(),
            from: at,
            last: whole_pixel(at),
        });
    }

    /// The pointer moved to `to`: a drag in progress scrolls from where it
    /// started ([`MapPlace::drag`]), and a poll whose whole pixel moved
    /// draws the partial update, whether or not the scroll followed
    /// (`_ScrollMapDisplay` @0x1128d-0x112fb).
    pub fn pointer_moved(&mut self, to: Point) {
        if let Some(drag) = &mut self.drag {
            let at = whole_pixel(to);
            if at != drag.last {
                drag.last = at;
                self.update = MapUpdate::Partial;
            }
            self.place.drag(drag.scroll, drag.from, to);
        }
    }

    /// Ends any drag, where the release left it: with the full update
    /// when the scroll moved from the drag's start, and otherwise with
    /// item 3 as it was (@0x13934-0x13948). Whether a drag ended, moved
    /// or not: the borders' coarse pass follows every one
    /// (@0x138de-0x1392f).
    pub fn release(&mut self) -> bool {
        let Some(drag) = self.drag.take() else {
            return false;
        };
        if drag.scroll != self.place.scroll() {
            self.redraw();
        }
        true
    }

    /// Abandons any drag, where it got to, as a release ends it (the
    /// original's loop ends on `StillDown` going false), and any press on
    /// a button. Whether a drag ended ([`MapDialog::release`]).
    pub fn cancel_pointer(&mut self) -> bool {
        self.buttons.cancel_pointer();
        self.release()
    }

    /// Redraws item 3 with the full update: an invalidation of item 3 or
    /// a `_MapDialogUpdate` (see [`update`](super::update)).
    pub fn redraw(&mut self) {
        self.update = MapUpdate::Full;
        self.fresh = true;
    }

    /// Whether item 3 was redrawn ([`MapDialog::redraw`]) since this was
    /// last taken; taking it clears it. The borders take their picture
    /// then, as the original's update event draws the grid as the handler
    /// left it.
    pub fn take_fresh(&mut self) -> bool {
        std::mem::take(&mut self.fresh)
    }

    /// The update item 3 shows.
    #[must_use]
    pub fn update(&self) -> MapUpdate {
        self.update
    }

    /// What the nebula pass reads: the dialog's port at
    /// [`DIALOG_ORIGIN`], item 3's centre (238, 218) and rect in it, and
    /// the exact zoom and the scroll.
    #[must_use]
    pub fn nebula_frame(&self) -> NebulaFrame {
        let (x, y) = self.place.scroll();
        NebulaFrame {
            origin: DIALOG_ORIGIN,
            centre: MAP_ITEM_CENTRE,
            zoom: self.place.zoom(),
            scroll: (f64::from(x), f64::from(y)),
            area: MAP_ITEM,
        }
    }

    /// Draws the frame under the map ([`draw_frame`]) with this dialog's
    /// buttons in its state.
    pub fn draw_frame(
        &self,
        list: &mut DrawList,
        style: &ButtonStyle,
        metrics: Option<&dyn TextMetrics>,
    ) {
        draw_frame(list, &self.buttons, &self.state(), style, metrics);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::DrawCommand;
    use crate::galaxy::buttons::{MapButton, ORDER};
    use crate::geometry::intersection;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{Placement, SCREEN};

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
        Bounds {
            min: at(left, top),
            max: at(right, bottom),
        }
    }

    // The layout.

    #[test]
    fn the_dialog_sits_as_center_window_puts_it_a_pixel_off_the_window_managers() {
        assert_eq!(dialog_origin(SCREEN), at(212.0, 128.0));
        assert_eq!(DIALOG_ORIGIN, dialog_origin(SCREEN));
        let bounds = Bounds::at(at(3.0, -150.0), DIALOG_SIZE.0, DIALOG_SIZE.1);
        assert_eq!(Placement::Center.origin(bounds), at(211.0, 127.0));
        assert_eq!(dialog_area(), rect(212.0, 128.0, 813.0, 641.0));
    }

    #[test]
    fn an_odd_window_truncates_as_center_window_does() {
        assert_eq!(dialog_origin((1023.0, 769.0)), at(211.0, 128.0));
        assert_eq!(dialog_origin((640.0, 480.0)), at(20.0, -16.0));
    }

    #[test]
    fn item_3_is_221_136_to_679_556_centred_at_450_346() {
        assert_eq!(MAP_ITEM, IRect::new(9, 8, 467, 428));
        assert_eq!(map_area(), rect(221.0, 136.0, 679.0, 556.0));
        assert_eq!(map_centre(), at(450.0, 346.0));
    }

    #[test]
    fn item_3_holds_its_left_and_top_edges_but_not_its_right_and_bottom() {
        assert!(in_map_item(at(221.0, 136.0)));
        assert!(in_map_item(at(678.9, 555.9)));
        assert!(!in_map_item(at(679.0, 300.0)), "the right edge");
        assert!(!in_map_item(at(300.0, 556.0)), "the bottom edge");
        assert!(!in_map_item(at(220.9, 300.0)));
        assert!(!in_map_item(at(300.0, 135.9)));
    }

    /// Everything enabled, borders shown.
    const ALL: ButtonState = ButtonState {
        can_zoom_out: true,
        can_zoom_in: true,
        clear_route: true,
        borders_shown: true,
    };

    #[test]
    fn the_frame_is_black_then_the_background_then_the_buttons_then_item_3_black() {
        let buttons = MapButtons::new(DIALOG_ORIGIN);
        let mut list = DrawList::new();
        draw_frame(
            &mut list,
            &buttons,
            &ALL,
            &ButtonStyle::STOCK,
            Some(&MonoMetrics),
        );
        let mut expected = DrawList::new();
        fill_rect(
            &mut expected,
            rect(212.0, 128.0, 813.0, 641.0),
            Color::BLACK,
        );
        expected.stretched_picture(ImageKey::picture(8509), at(212.0, 128.0), 601.0, 513.0);
        buttons.draw(&ALL, &ButtonStyle::STOCK, Some(&MonoMetrics), &mut expected);
        fill_rect(
            &mut expected,
            rect(221.0, 136.0, 679.0, 556.0),
            Color::BLACK,
        );
        assert_eq!(list, expected);
        assert_eq!(BACKGROUND, 8509);
    }

    // The dialog's own state.

    #[test]
    fn a_new_dialog_looks_where_it_is_told_at_0_5625_with_the_borders_shown() {
        let dialog = MapDialog::new((10, -20));
        assert_eq!(dialog.place(), &MapPlace::new((10, -20)));
        assert!(dialog.borders_shown());
        assert_eq!(
            dialog.transform(),
            MapPlace::new((10, -20)).transform(at(450.0, 346.0))
        );
    }

    #[test]
    fn the_state_reads_the_zoom_flags_clear_route_and_the_borders() {
        let mut dialog = MapDialog::new((0, 0));
        assert!(!dialog.clear_route(), "off as made");
        dialog.set_clear_route(true);
        assert_eq!(dialog.state(), ALL);
        dialog.place_mut().zoom_in();
        dialog.set_borders_shown(false);
        dialog.set_clear_route(false);
        assert!(!dialog.borders_shown());
        assert_eq!(
            dialog.state(),
            ButtonState {
                can_zoom_out: true,
                can_zoom_in: false,
                clear_route: false,
                borders_shown: false,
            }
        );
    }

    #[test]
    fn its_frame_draws_its_buttons_in_its_state() {
        let mut dialog = MapDialog::new((0, 0));
        dialog.place_mut().zoom_in();
        let mut list = DrawList::new();
        dialog.draw_frame(&mut list, &ButtonStyle::STOCK, None);
        let mut expected = DrawList::new();
        draw_frame(
            &mut expected,
            dialog.buttons(),
            &dialog.state(),
            &ButtonStyle::STOCK,
            None,
        );
        assert_eq!(list, expected);
        let enabled: Vec<bool> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, color, .. } => {
                    Some((text.clone(), *color == ButtonStyle::STOCK.up))
                }
                _ => None,
            })
            .map(|(_, up)| up)
            .collect();
        assert_eq!(enabled, [true, true, false, false, true, true]);
        assert_eq!(ORDER[2], MapButton::ZoomIn);
    }

    #[test]
    fn the_arrows_scroll_10_units_and_the_zoom_keys_step_with_the_limits() {
        let mut dialog = MapDialog::new((0, 0));
        for (key, scroll) in [
            (Key::Right, (10, 0)),
            (Key::Down, (10, 10)),
            (Key::Left, (0, 10)),
            (Key::Up, (0, 0)),
        ] {
            assert!(dialog.key(key));
            assert_eq!(dialog.place().scroll(), scroll, "{key:?}");
        }
        assert!(dialog.key(Key::Char('=')));
        assert_eq!(dialog.place().zoom(), 0.421_875);
        assert!(dialog.key(Key::Char('+')), "taken, though refused");
        assert_eq!(dialog.place().zoom(), 0.421_875);
        let mut zooms = Vec::new();
        for _ in 0..8 {
            dialog.key(Key::Char('-'));
            zooms.push(dialog.place().zoom());
        }
        assert!(zooms[6] > 2.0 && zooms[6] < 3.0, "{zooms:?}");
        assert_eq!(zooms[7], zooms[6], "refused past 2.0");
        assert!(!dialog.key(Key::Char('x')));
        assert!(!dialog.key(Key::Tab));
    }

    #[test]
    fn a_drag_scrolls_from_where_it_started_until_released_or_cancelled() {
        let mut dialog = MapDialog::new((100, 100));
        dialog.pointer_moved(at(500.0, 500.0));
        assert_eq!(dialog.place().scroll(), (100, 100), "no press");
        dialog.press(at(400.0, 300.0));
        dialog.pointer_moved(at(420.0, 300.0));
        dialog.pointer_moved(at(440.0, 284.0));
        assert_eq!(dialog.place().scroll(), (77, 109), "from the start");
        dialog.release();
        dialog.pointer_moved(at(600.0, 300.0));
        assert_eq!(dialog.place().scroll(), (77, 109), "released");
        dialog.press(at(400.0, 300.0));
        dialog.pointer_moved(at(408.0, 300.0));
        dialog.cancel_pointer();
        dialog.pointer_moved(at(500.0, 300.0));
        assert_eq!(dialog.place().scroll(), (72, 109), "cancelled where it got");
    }

    // The update item 3 shows.

    #[test]
    fn a_new_dialog_shows_the_full_update() {
        assert_eq!(MapDialog::new((0, 0)).update(), MapUpdate::Full);
    }

    #[test]
    fn a_move_of_a_whole_pixel_in_a_drag_shows_the_partial_update() {
        let mut dialog = MapDialog::new((100, 100));
        dialog.pointer_moved(at(500.0, 500.0));
        assert_eq!(dialog.update(), MapUpdate::Full, "no press");
        dialog.press(at(400.2, 300.2));
        dialog.pointer_moved(at(400.9, 300.7));
        assert_eq!(dialog.update(), MapUpdate::Full, "the same whole pixel");
        dialog.pointer_moved(at(401.0, 300.7));
        assert_eq!(dialog.update(), MapUpdate::Partial);
    }

    #[test]
    fn a_move_the_scroll_does_not_follow_still_shows_the_partial_update() {
        // 1 px left at zoom 0.5625 from scroll 100: 100 + 0.5625 truncates
        // to 100.
        let mut dialog = MapDialog::new((100, 100));
        dialog.press(at(400.0, 300.0));
        dialog.pointer_moved(at(399.0, 300.0));
        assert_eq!(dialog.place().scroll(), (100, 100));
        assert_eq!(dialog.update(), MapUpdate::Partial);
    }

    #[test]
    fn the_release_shows_the_full_update_only_when_the_scroll_moved() {
        let mut dialog = MapDialog::new((100, 100));
        dialog.press(at(400.0, 300.0));
        dialog.pointer_moved(at(420.0, 300.0));
        dialog.release();
        assert_eq!(dialog.update(), MapUpdate::Full, "moved");

        dialog.press(at(400.0, 300.0));
        dialog.pointer_moved(at(420.0, 300.0));
        dialog.pointer_moved(at(400.0, 300.0));
        dialog.release();
        assert_eq!(dialog.update(), MapUpdate::Partial, "brought back");
        dialog.redraw();
        assert_eq!(dialog.update(), MapUpdate::Full, "until a redraw");
    }

    #[test]
    fn a_cancelled_drag_ends_as_a_release_does() {
        let mut dialog = MapDialog::new((100, 100));
        dialog.press(at(400.0, 300.0));
        dialog.pointer_moved(at(399.0, 300.0));
        dialog.cancel_pointer();
        assert_eq!(dialog.update(), MapUpdate::Partial, "back where it began");
        dialog.press(at(400.0, 300.0));
        dialog.pointer_moved(at(380.0, 300.0));
        dialog.cancel_pointer();
        assert_eq!(dialog.update(), MapUpdate::Full, "moved");
    }

    #[test]
    fn the_arrows_and_a_zoom_step_taken_redraw_and_a_refused_one_does_not() {
        let partial = || {
            let mut dialog = MapDialog::new((100, 100));
            dialog.press(at(400.0, 300.0));
            dialog.pointer_moved(at(399.0, 300.0));
            dialog.release();
            assert_eq!(dialog.update(), MapUpdate::Partial);
            dialog
        };
        for key in [Key::Left, Key::Right, Key::Up, Key::Down, Key::Char('-')] {
            let mut dialog = partial();
            dialog.key(key);
            assert_eq!(dialog.update(), MapUpdate::Full, "{key:?}");
        }
        let mut dialog = partial();
        dialog.key(Key::Char('='));
        assert_eq!(dialog.update(), MapUpdate::Full, "a step in taken");
        let mut dialog = partial();
        dialog.place_mut().zoom_in();
        dialog.key(Key::Char('+'));
        assert_eq!(dialog.update(), MapUpdate::Partial, "refused at the limit");
        dialog.key(Key::Char('x'));
        assert_eq!(dialog.update(), MapUpdate::Partial, "not taken");
    }

    #[test]
    fn a_redraw_is_fresh_until_taken() {
        let mut dialog = MapDialog::new((0, 0));
        assert!(!dialog.take_fresh(), "a new dialog");
        dialog.redraw();
        assert!(dialog.take_fresh());
        assert!(!dialog.take_fresh(), "taken");
        dialog.key(Key::Left);
        assert!(dialog.take_fresh(), "an arrow redraws");
        dialog.press(at(400.0, 300.0));
        dialog.pointer_moved(at(380.0, 300.0));
        assert!(!dialog.take_fresh(), "the partial update isn't a redraw");
    }

    #[test]
    fn a_release_or_a_cancel_says_whether_a_drag_ended_moved_or_not() {
        let mut dialog = MapDialog::new((100, 100));
        assert!(!dialog.release(), "no drag");
        assert!(!dialog.cancel_pointer(), "no drag");
        dialog.press(at(400.0, 300.0));
        assert!(dialog.release(), "a drag that never moved");
        assert!(!dialog.release(), "ended already");
        dialog.press(at(400.0, 300.0));
        dialog.pointer_moved(at(380.0, 300.0));
        assert!(dialog.cancel_pointer(), "moved");
        assert!(!dialog.cancel_pointer());
    }

    #[test]
    fn the_nebulae_are_drawn_in_the_dialogs_port_around_item_3s_centre() {
        let mut dialog = MapDialog::new((10, -20));
        dialog.place_mut().zoom_out();
        assert_eq!(
            dialog.nebula_frame(),
            NebulaFrame {
                origin: at(212.0, 128.0),
                centre: (238, 218),
                zoom: dialog.place().zoom(),
                scroll: (10.0, -20.0),
                area: MAP_ITEM,
            }
        );
        assert_eq!(dialog.place().zoom(), 0.5625 * 1.333_333, "the exact zoom");
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_press_on_a_button() {
        let mut dialog = MapDialog::new((0, 0));
        let done = dialog.buttons().rect(MapButton::Done).center();
        dialog.set_clear_route(true);
        let state = dialog.state();
        let press = |pressed| crate::Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed,
            at: done,
        };
        dialog.buttons_mut().input(&press(true), &state);
        dialog.cancel_pointer();
        assert_eq!(dialog.buttons_mut().input(&press(false), &state), None);
    }

    // The in-flight maps in the dialog.

    use crate::galaxy::buttons::{CLEAR_ROUTE_LABEL, HIDE_BORDERS_LABEL, SHOW_BORDERS_LABEL};
    use crate::galaxy::catalog::SystemId;
    use crate::galaxy::click::PressAnswer;
    use crate::galaxy::map::{BUTTON, ENTER_BUTTON, ENTER_LABEL, GalaxyMap, PANEL};
    use crate::galaxy::testkit::{FakeCatalog, galaxy, system};
    use crate::galaxy::view::{MapView, PAN_STEP, SCALES};
    use crate::{Font, Input, MouseButton, Screen};
    use nova_sim::galaxy_map::{DrawnSystem, MapDrawing};
    use nova_sim::iff::SystemColorCode;
    use nova_sim::map_search::SearchOutcome;

    /// The course map in Alpha (128) at (0, 0), looking at it.
    fn course_map() -> GalaxyMap {
        let mut map = GalaxyMap::course(&FakeCatalog::new(galaxy()));
        map.show_course(SystemId(128), &[]);
        map.recentre();
        map
    }

    fn drawn(map: &GalaxyMap) -> DrawList {
        let mut list = DrawList::new();
        map.draw(&mut list);
        list
    }

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn held(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    fn left(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn click(map: &mut GalaxyMap, at: Point) {
        map.input(&left(true, at));
        map.input(&left(false, at));
    }

    /// Presses at `from`, moves to `to` and releases there.
    fn drag(map: &mut GalaxyMap, from: Point, to: Point) {
        map.input(&left(true, from));
        map.input(&Input::PointerMoved(to));
        map.input(&left(false, to));
    }

    /// Where `button` is.
    fn on(button: MapButton) -> Point {
        MapButtons::new(DIALOG_ORIGIN).rect(button).center()
    }

    /// Each button label drawn, in order, and whether it is drawn
    /// enabled.
    fn labels(list: &DrawList) -> Vec<(String, bool)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    font: Font::Charcoal,
                    color,
                    ..
                } => Some((text.clone(), *color != ButtonStyle::STOCK.grey)),
                _ => None,
            })
            .collect()
    }

    fn label(text: &str, enabled: bool) -> (String, bool) {
        (text.to_owned(), enabled)
    }

    fn scroll(map: &GalaxyMap) -> (i32, i32) {
        map.place().expect("an in-flight map").scroll()
    }

    fn zoom(map: &GalaxyMap) -> f64 {
        map.zoom().expect("an in-flight map")
    }

    #[test]
    fn the_course_map_opens_on_its_scroll_at_0_5625_drawn_at_item_3s_centre() {
        let map = course_map();
        assert_eq!(map.view(), None);
        assert_eq!(map.place(), Some(&MapPlace::new((0, 0))));
        assert_eq!(
            map.transform(),
            MapPlace::new((0, 0)).transform(at(450.0, 346.0))
        );
    }

    #[test]
    fn the_frame_draws_the_background_and_the_six_buttons_by_their_states() {
        let mut map = course_map();
        let list = drawn(&map);
        let mut frame = DrawList::new();
        let state = ButtonState {
            clear_route: false,
            ..ALL
        };
        draw_frame(
            &mut frame,
            &MapButtons::new(DIALOG_ORIGIN),
            &state,
            &ButtonStyle::STOCK,
            None,
        );
        let first: Vec<&DrawCommand> = list.iter().take(frame.len()).collect();
        assert_eq!(first, frame.iter().collect::<Vec<_>>(), "the frame first");
        assert_eq!(
            labels(&list),
            [
                label("Done", true),
                label("-", true),
                label("+", true),
                label(CLEAR_ROUTE_LABEL, false),
                label(HIDE_BORDERS_LABEL, true),
                label("Find", true),
            ],
            "Clear Route off until set; the borders shown by default"
        );
        map.show_course(SystemId(128), &[SystemId(129)]);
        assert_eq!(
            labels(&drawn(&map))[3],
            label(CLEAR_ROUTE_LABEL, false),
            "the course alone doesn't set showClearRouteButton"
        );
        map.set_clear_route(true);
        assert_eq!(labels(&drawn(&map))[3], label(CLEAR_ROUTE_LABEL, true));
        map.show_borders_setting(false);
        assert_eq!(labels(&drawn(&map))[4], label(SHOW_BORDERS_LABEL, true));
        map.set_clear_route(false);
        assert_eq!(labels(&drawn(&map))[3], label(CLEAR_ROUTE_LABEL, false));
    }

    #[test]
    fn the_zoom_buttons_are_disabled_at_their_limits() {
        let mut map = course_map();
        map.input(&press(Key::Char('=')));
        assert_eq!(zoom(&map), 0.421_875);
        let enabled = |map: &GalaxyMap| {
            labels(&drawn(map))
                .into_iter()
                .map(|(_, on)| on)
                .collect::<Vec<_>>()
        };
        assert_eq!(enabled(&map)[1..3], [true, false], "+ at 0.421875");
        for _ in 0..6 {
            map.input(&press(Key::Char('-')));
        }
        assert!(zoom(&map) > 2.0);
        assert_eq!(enabled(&map)[1..3], [false, true], "- past 2.0");
    }

    /// In Alpha, with the course to Beta, every system drawn and named.
    fn drawing() -> MapDrawing {
        let drawn = |id| DrawnSystem {
            id: SystemId(id),
            code: SystemColorCode::Uninhabited,
            red_arrow: false,
            green_arrow: false,
        };
        MapDrawing {
            hypergate: false,
            course: vec![(SystemId(128), SystemId(129))],
            current_links: vec![(SystemId(128), SystemId(129))],
            links: Vec::new(),
            mode3_link: None,
            systems: vec![drawn(128), drawn(129), drawn(130)],
            current: SystemId(128),
            gate_arrows: Vec::new(),
            named: vec![SystemId(128), SystemId(129), SystemId(130)],
        }
    }

    #[test]
    fn nothing_the_map_draws_reaches_past_item_3() {
        let mut map = course_map();
        map.show_drawing(&drawing());
        map.preselect(Some(SystemId(129)));
        let list = drawn(&map);
        let mut frame = DrawList::new();
        draw_frame(
            &mut frame,
            &MapButtons::new(DIALOG_ORIGIN),
            &ALL,
            &ButtonStyle::STOCK,
            None,
        );
        let area = map_area();
        let inside = |p: Point| {
            p.x >= area.min.x - 0.01
                && p.x <= area.max.x + 0.01
                && p.y >= area.min.y - 0.01
                && p.y <= area.max.y + 0.01
        };
        let mut names = Vec::new();
        for command in list.iter().skip(frame.len()) {
            match command {
                DrawCommand::Line {
                    from, to, width, ..
                } if from.y == to.y => {
                    let half = width / 2.0;
                    assert!(
                        inside(at(from.x, from.y - half)) && inside(at(to.x, to.y + half)),
                        "{command:?}"
                    );
                }
                DrawCommand::Line { from, to, .. } => {
                    assert!(inside(*from) && inside(*to), "{command:?}");
                }
                DrawCommand::Dot { center, .. } => assert!(inside(*center), "{command:?}"),
                DrawCommand::Clipped { clip, command } => {
                    // Item 3, or a nebula's rect within it.
                    assert_eq!(intersection(*clip, area), Some(*clip), "{command:?}");
                    if let DrawCommand::Text { text, .. } = &**command {
                        names.push(text.clone());
                    }
                }
                other => panic!("unclipped: {other:?}"),
            }
        }
        assert!(names.contains(&"Beta".to_owned()), "{names:?}");
        assert!(names.contains(&"Alpha".to_owned()), "{names:?}");
    }

    #[test]
    fn the_in_flight_maps_draw_none_of_the_viewers_lines_or_its_panel() {
        let mut map = course_map();
        map.show_course(SystemId(128), &[SystemId(129)]);
        map.preselect(Some(SystemId(128)));
        let list = drawn(&map);
        let texts: Vec<String> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        for viewer in [
            "Arrows",
            "Zoom",
            "Course:",
            "1 of 2",
            "Click a system",
            "Alpha (",
        ] {
            assert!(
                !texts.iter().any(|text| text.starts_with(viewer)),
                "{viewer}: {texts:?}"
            );
        }
        assert!(
            !list.iter().any(|command| matches!(
                command,
                DrawCommand::Line { color, .. } if *color == PANEL || *color == BUTTON
            )),
            "no panel fill, no Enter button"
        );
        assert!(!texts.contains(&ENTER_LABEL.to_owned()));
    }

    #[test]
    fn a_click_on_plus_or_minus_zooms_and_one_released_off_it_does_not() {
        let mut map = course_map();
        click(&mut map, on(MapButton::ZoomIn));
        assert_eq!(zoom(&map), 0.421_875);
        click(&mut map, on(MapButton::ZoomIn));
        assert_eq!(zoom(&map), 0.421_875, "disabled at the limit");
        click(&mut map, on(MapButton::ZoomOut));
        assert_eq!(zoom(&map), 0.421_875 * 1.333_333);
        let before = zoom(&map);
        drag(&mut map, on(MapButton::ZoomOut), on(MapButton::Done));
        drag(&mut map, on(MapButton::ZoomIn), at(10.0, 10.0));
        assert_eq!(zoom(&map), before, "released off them");
        assert!(!map.take_close(), "nor Done");
    }

    #[test]
    fn equals_plus_and_minus_zoom_within_the_limits_their_repeats_too() {
        let mut map = course_map();
        map.input(&press(Key::Char('-')));
        assert_eq!(zoom(&map), 0.749_999_812_5);
        map.input(&held(Key::Char('=')));
        assert_eq!(zoom(&map), 0.749_999_812_5 * 0.75, "a repeat steps again");
        map.input(&press(Key::Char('+')));
        assert!(!map.place().expect("in flight").can_zoom_in());
        let at_limit = zoom(&map);
        map.input(&held(Key::Char('+')));
        assert_eq!(zoom(&map), at_limit, "refused below 0.5");
        for _ in 0..10 {
            map.input(&held(Key::Char('-')));
        }
        assert!(!map.place().expect("in flight").can_zoom_out());
        assert_eq!(scroll(&map), (0, 0), "zooming keeps the scroll");
    }

    #[test]
    fn done_asks_to_close_and_find_opens_find_from_their_ditl_rects() {
        let mut map = course_map().with_metrics(std::rc::Rc::new(MonoMetrics));
        click(&mut map, on(MapButton::Done));
        assert!(map.take_close());
        click(&mut map, on(MapButton::Find));
        assert!(map.finding());
    }

    #[test]
    fn borders_is_taken_flipping_its_label_and_asks_the_owner_nothing() {
        let mut map = course_map().with_metrics(std::rc::Rc::new(MonoMetrics));
        map.show_course(SystemId(128), &[SystemId(129)]);
        map.set_clear_route(true);
        assert!(labels(&drawn(&map)).contains(&label(HIDE_BORDERS_LABEL, true)));
        click(&mut map, on(MapButton::Borders));
        assert!(!map.take_close());
        assert!(!map.finding());
        assert!(!map.take_clear_route());
        assert_eq!(map.route(), [SystemId(129)]);
        assert!(!map.borders_shown());
        assert!(labels(&drawn(&map)).contains(&label(SHOW_BORDERS_LABEL, true)));
    }

    #[test]
    fn clear_route_asks_to_clear_only_while_enabled() {
        let mut map = course_map().with_metrics(std::rc::Rc::new(MonoMetrics));
        map.show_course(SystemId(128), &[SystemId(129)]);
        click(&mut map, on(MapButton::ClearRoute));
        assert!(!map.take_clear_route(), "disabled");
        map.set_clear_route(true);
        click(&mut map, on(MapButton::ClearRoute));
        assert!(map.take_clear_route(), "enabled");
        assert!(!map.take_clear_route(), "taken");
        assert!(!map.take_close());
        drag(&mut map, on(MapButton::ClearRoute), on(MapButton::Done));
        assert!(!map.take_clear_route(), "released off it");
    }

    #[test]
    fn c_asks_to_clear_whatever_the_button_its_repeats_too() {
        for enabled in [false, true] {
            let mut map = course_map();
            map.set_clear_route(enabled);
            for key in [
                press(Key::Char('c')),
                press(Key::Char('C')),
                held(Key::Char('c')),
            ] {
                map.input(&key);
                assert!(map.take_clear_route(), "{key:?}, enabled {enabled}");
            }
            map.input(&Input::Key {
                key: Key::Char('c'),
                pressed: false,
                repeat: false,
            });
            assert!(!map.take_clear_route(), "a release");
        }
        let mut gate = course_map();
        gate.offer_gates(vec![SystemId(129)]);
        gate.input(&press(Key::Char('c')));
        assert!(gate.take_clear_route(), "the hypergate map too");
        let mut viewer = viewer();
        viewer.input(&press(Key::Char('c')));
        assert!(!viewer.take_clear_route(), "never the viewer");
    }

    #[test]
    fn the_latch_is_written_by_an_answer_that_says_and_cleared_by_the_clear() {
        let mut map = course_map();
        let alpha = map.transform().world_to_screen(Point::new(0.0, 0.0));
        let answer = |map: &mut GalaxyMap, clear_route| {
            map.input(&left(true, alpha));
            map.take_press().expect("a hit");
            map.answer_press(Some(PressAnswer {
                selected: None,
                gate_pick: None,
                clear_route,
            }));
            map.input(&left(false, alpha));
            labels(&drawn(map))[3].1
        };
        assert!(!answer(&mut map, None), "nothing written");
        assert!(answer(&mut map, Some(true)), "set");
        assert!(answer(&mut map, None), "kept by a press writing nothing");
        assert!(!answer(&mut map, Some(false)), "turned off");
        assert!(answer(&mut map, Some(true)), "set again");
        map.route_cleared();
        assert!(!labels(&drawn(&map))[3].1, "the clear");
    }

    #[test]
    fn the_clear_drops_the_hypergate_maps_pick() {
        let mut map = course_map();
        map.offer_gates(vec![SystemId(129)]);
        map.select_gate(Some(SystemId(129)));
        map.route_cleared();
        assert_eq!(map.take_gate_choice(), None);
    }

    #[test]
    fn the_arrows_scroll_10_units_at_any_zoom_their_repeats_too() {
        for zoom_in in [false, true] {
            let mut map = course_map();
            if zoom_in {
                map.input(&press(Key::Char('=')));
            }
            map.input(&press(Key::Right));
            map.input(&held(Key::Right));
            map.input(&press(Key::Down));
            assert_eq!(scroll(&map), (20, 10), "zoomed in: {zoom_in}");
            map.input(&held(Key::Left));
            map.input(&press(Key::Up));
            map.input(&held(Key::Up));
            assert_eq!(scroll(&map), (10, -10), "zoomed in: {zoom_in}");
        }
    }

    #[test]
    fn a_drag_in_item_3_scrolls_by_the_travel_times_the_zoom() {
        let mut map = course_map();
        drag(&mut map, at(300.0, 200.0), at(340.0, 184.0));
        assert_eq!(scroll(&map), (-22, 9), "at 0.5625");
        let mut map = course_map().with_zoom(2.0);
        drag(&mut map, at(300.0, 200.0), at(340.0, 184.0));
        assert_eq!(scroll(&map), (-80, 32), "at 2.0");
        assert_eq!(map.selected(), None, "a drag selects nothing");
    }

    #[test]
    fn a_drag_follows_each_move_from_the_presss_start() {
        let mut map = course_map().with_zoom(1.0);
        map.input(&left(true, at(300.0, 200.0)));
        map.input(&Input::PointerMoved(at(310.0, 200.0)));
        assert_eq!(scroll(&map), (-10, 0));
        map.input(&Input::PointerMoved(at(330.0, 205.0)));
        assert_eq!(scroll(&map), (-30, -5));
        map.input(&left(false, at(330.0, 205.0)));
        map.input(&Input::PointerMoved(at(400.0, 205.0)));
        assert_eq!(scroll(&map), (-30, -5), "released");
        map.input(&left(true, at(300.0, 200.0)));
        map.input(&Input::PointerMoved(at(305.0, 200.0)));
        map.cancel_pointer();
        map.input(&Input::PointerMoved(at(400.0, 200.0)));
        assert_eq!(scroll(&map), (-35, -5), "cancelled");
    }

    #[test]
    fn a_press_outside_item_3_neither_drags_nor_selects() {
        let mut map = course_map();
        let alpha = map.transform().world_to_screen(Point::new(0.0, 0.0));
        assert!(in_map_item(alpha));
        for outside in [at(230.0, 600.0), at(700.0, 300.0), at(100.0, 300.0)] {
            drag(&mut map, outside, at(outside.x + 50.0, outside.y - 30.0));
            assert_eq!(scroll(&map), (0, 0), "{outside:?}");
            drag(&mut map, outside, alpha);
            assert_eq!(map.selected(), None, "{outside:?}");
        }
        map.input(&left(true, alpha));
        assert_eq!(
            map.take_press().map(|press| press.hits),
            Some(vec![SystemId(128), SystemId(131)]),
            "a press in item 3"
        );
    }

    #[test]
    fn space_and_find_centre_the_scroll_unclamped_and_recentre_goes_back() {
        let mut map = course_map();
        map.show_search(SearchOutcome::Found(SystemId(129)));
        assert_eq!(scroll(&map), (600, 0));
        map.input(&press(Key::Space));
        assert_eq!(scroll(&map), (0, 0));
        map.take_ui_sounds();
        // Far past every system: nothing clamps.
        for _ in 0..100 {
            map.input(&held(Key::Left));
        }
        assert_eq!(scroll(&map), (-1000, 0));
        map.preselect(Some(SystemId(130)));
        map.recentre();
        assert_eq!(scroll(&map), (0, 0), "on the current system");
        assert_eq!(map.selected(), Some(SystemId(130)), "the selection kept");
        assert_eq!(map.take_ui_sounds(), [], "silent");
        map.centre_on_system(SystemId(130));
        assert_eq!(scroll(&map), (0, 300));
        map.centre_on_system(SystemId(999));
        assert_eq!(scroll(&map), (0, 300), "not on the map");
    }

    #[test]
    fn the_zoom_carries_into_another_map_and_opening_guards_it() {
        let map = course_map().with_zoom(1.777_776);
        assert_eq!(map.zoom(), Some(1.777_776));
        let mut map = course_map().with_zoom(0.0);
        map.opened();
        assert_eq!(map.zoom(), Some(1.0));
        let mut map = course_map().with_zoom(0.75);
        map.opened();
        assert_eq!(map.zoom(), Some(0.75));
    }

    // The viewer keeps its own look.

    fn viewer() -> GalaxyMap {
        GalaxyMap::new(&FakeCatalog::new(galaxy()))
    }

    #[test]
    fn the_viewer_is_full_screen_and_fitted_with_no_dialog() {
        let mut map = viewer();
        assert_eq!(map.view(), Some(&MapView::fit(map.model().bounds())));
        assert_eq!(map.place(), None);
        assert_eq!(map.zoom(), None);
        map.opened();
        map.recentre();
        map.show_borders_setting(false);
        let map = map.with_zoom(2.0);
        assert_eq!(
            map.view(),
            Some(&MapView::fit(map.model().bounds())),
            "untouched"
        );
        let list = drawn(&map);
        assert!(
            !list.iter().any(|command| matches!(
                command,
                DrawCommand::StretchedPicture { image, .. } if *image == ImageKey::picture(BACKGROUND)
            )),
            "no background"
        );
        assert_eq!(labels(&list), [], "no buttons");
        assert!(
            !list
                .iter()
                .any(|command| matches!(command, DrawCommand::Clipped { clip, .. } if *clip == map_area())),
            "nothing clipped to item 3"
        );
    }

    #[test]
    fn the_viewer_pans_and_zooms_its_own_way() {
        let mut map = viewer();
        let fitted = *map.view().expect("the viewer");
        map.input(&press(Key::Right));
        let moved = *map.view().expect("the viewer");
        let step = PAN_STEP / fitted.scale();
        assert!((moved.center().x - fitted.center().x - step).abs() < 1e-3);
        map.input(&press(Key::Char('-')));
        assert_eq!(
            map.view().expect("the viewer").scale(),
            SCALES[fitted.zoom() - 1]
        );
        map.input(&press(Key::Char('=')));
        map.input(&press(Key::Char('=')));
        assert_eq!(
            map.view().expect("the viewer").scale(),
            SCALES[fitted.zoom() + 1]
        );
    }

    #[test]
    fn the_viewer_shows_its_enter_button_and_draws_unclipped_near_the_corner() {
        let galaxy = crate::galaxy::catalog::Galaxy {
            systems: vec![system(128, -488, -280), system(129, 488, 280)],
            ..crate::galaxy::catalog::Galaxy::default()
        };
        let mut map = GalaxyMap::new(&FakeCatalog::new(galaxy));
        assert_eq!(map.view().expect("the viewer").scale(), 1.0);
        let corner = map.transform().world_to_screen(Point::new(488.0, 280.0));
        assert_eq!(corner, at(1000.0, 584.0), "far outside any item 3");
        click(&mut map, corner);
        assert_eq!(map.selected(), Some(SystemId(129)));
        let list = drawn(&map);
        assert!(list.iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, .. } if text == ENTER_LABEL
        )));
        assert!(list.iter().any(|command| matches!(
            command,
            DrawCommand::Line { color, from, .. } if *color == BUTTON && ENTER_BUTTON.contains(*from)
        )));
        assert!(
            list.iter().any(|command| matches!(
                command,
                DrawCommand::Text { text, .. } if text == "System 129"
            )),
            "its name drawn as it is"
        );
    }
}
