//! The galaxy map under fingers (rdm:phase/key-bindings/phase-14-touch-input).
//!
//! One finger is the pointer ([`crate::touch`]), so it drags and clicks as
//! the mouse does with no code here. Two fingers pinch, and only the map
//! acts on a [`Pinch`]:
//!
//! - **The pinch is on the map** when both fingers' points are in the map
//!   area as it begins (above the panel on the viewer, item 3 on an
//!   in-flight map) and Find isn't open. The map then cancels its own
//!   press ([`Screen::cancel_pointer`]): the drag stops and nothing the
//!   primary finger pressed fires.
//! - **Each step is the map's own zoom command**, as the zoom keys give
//!   it, so it zooms about the map's centre within the map's limits, with
//!   the same redraw and borders' coarse pass.
//! - **The steps are clamped at the map's limits.** A step the map refuses
//!   is held back, and each step the other way first takes back one held
//!   step before it zooms. So bringing the fingers back to their starting
//!   spread brings back the starting zoom, however far past a limit they
//!   spread.
//! - **The drag doesn't resume**: from the second finger landing until
//!   the primary lifts, the primary's moves find no press and its release
//!   clicks nothing, as a fresh press could plot a course the player
//!   ending a pinch wouldn't expect.
//! - A pinch with a finger off the map area (on "+", say) or while Find
//!   is open is no pinch of the map's: the primary's press carries on.

use super::map::GalaxyMap;
use super::view::MapView;
use crate::input::Pinch;
use crate::keymap::{Command, MapCommand};
use crate::screen::Screen;

/// A pinch on the map: the steps it refused at a limit, zoom-ins
/// positive, not yet taken back by steps the other way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapPinch {
    refused: i32,
}

impl GalaxyMap {
    /// What a pinch does to the map: see the module docs.
    pub(super) fn pinch_input(&mut self, pinch: Pinch) {
        match pinch {
            Pinch::Began { primary, second } => {
                if self.in_map(primary) && self.in_map(second) {
                    self.cancel_pointer();
                    self.pinch = Some(MapPinch::default());
                }
            }
            Pinch::ZoomIn | Pinch::ZoomOut => {
                let Some(mut held) = self.pinch else {
                    return;
                };
                let (command, way) = if pinch == Pinch::ZoomIn {
                    (MapCommand::ZoomIn, 1)
                } else {
                    (MapCommand::ZoomOut, -1)
                };
                if held.refused * way < 0 {
                    // Takes back a step refused the other way.
                    held.refused += way;
                } else {
                    let before = self.zoom_level();
                    self.command(Command::Map(command), false);
                    if self.zoom_level() == before {
                        held.refused += way;
                    }
                }
                self.pinch = Some(held);
            }
            Pinch::Ended => self.pinch = None,
        }
    }
}

impl GalaxyMap {
    /// The map's zoom now, to tell a step taken from one refused: the
    /// viewer's scale index, or an in-flight map's units per pixel.
    fn zoom_level(&self) -> (Option<usize>, Option<f64>) {
        (self.view().map(MapView::zoom), self.zoom())
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::rc::Rc;

    use super::super::buttons::{MapButton, MapButtons};
    use super::super::catalog::SystemId;
    use super::super::dialog::{DIALOG_ORIGIN, map_area};
    use super::super::testkit::{FakeCatalog, galaxy};
    use super::super::view::MAP_HEIGHT;
    use super::*;
    use crate::draw::{DrawCommand, DrawList};
    use crate::geometry::Point;
    use crate::input::{Input, Key, MouseButton};
    use crate::text::fixture::MonoMetrics;
    use crate::touch::testkit::Fingers;
    use crate::ui::button::ButtonStyle;

    fn p(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// The viewer.
    fn viewer() -> GalaxyMap {
        GalaxyMap::new(&FakeCatalog::new(galaxy()))
    }

    /// The course map in Alpha (128), with metrics and Find, opened.
    fn course_map() -> GalaxyMap {
        let mut map = GalaxyMap::course(&FakeCatalog::new(galaxy()))
            .with_metrics(Rc::new(MonoMetrics))
            .with_find(None, ButtonStyle::STOCK);
        map.show_course(SystemId(128), &[]);
        map.opened();
        map
    }

    fn pointer(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    /// The mouse pressing at `path`'s first point, moving along it and
    /// releasing at its last.
    fn mouse_drag(map: &mut GalaxyMap, path: &[Point]) {
        let (first, last) = (path[0], path[path.len() - 1]);
        map.input(&Input::PointerMoved(first));
        map.input(&pointer(true, first));
        for &at in &path[1..] {
            map.input(&Input::PointerMoved(at));
        }
        map.input(&pointer(false, last));
    }

    /// One finger along the same path.
    fn finger_drag(map: &mut GalaxyMap, path: &[Point]) {
        let mut fingers = Fingers::on(map);
        fingers.down(1, path[0]);
        for &at in &path[1..] {
            fingers.move_to(1, at);
        }
        fingers.up(1, path[path.len() - 1]);
    }

    fn key(map: &mut GalaxyMap, key: Key, times: usize) {
        for _ in 0..times {
            for pressed in [true, false] {
                map.input(&Input::Key {
                    key,
                    pressed,
                    repeat: false,
                });
            }
        }
    }

    fn drawn(map: &GalaxyMap) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        map.draw(&mut list);
        list.iter().cloned().collect()
    }

    /// The map's zoom, whichever map it is.
    fn zoom(map: &GalaxyMap) -> f64 {
        match (map.view(), map.place()) {
            (Some(view), _) => f64::from(view.scale()),
            (None, Some(place)) => place.zoom(),
            (None, None) => panic!("no map"),
        }
    }

    /// `spread` apart along a horizontal line through `centre`: the
    /// primary's point and the second's.
    fn apart(centre: Point, spread: f32) -> (Point, Point) {
        (
            p(centre.x - spread / 2.0, centre.y),
            p(centre.x + spread / 2.0, centre.y),
        )
    }

    /// A point in the viewer's map area with no system near it.
    const VIEWER_EMPTY: Point = Point::new(300.0, 200.0);

    /// A point in item 3 with no system near it, off the course map's
    /// centre (Alpha).
    fn course_empty() -> Point {
        let centre = map_area().center();
        p(centre.x + 100.0, centre.y + 80.0)
    }

    #[test]
    fn a_one_finger_drag_pans_the_viewer_as_a_mouse_drag_does() {
        let path = [VIEWER_EMPTY, p(330.0, 210.0), p(380.0, 240.0)];
        let mut by_mouse = viewer();
        mouse_drag(&mut by_mouse, &path);
        let mut by_finger = viewer();
        finger_drag(&mut by_finger, &path);
        assert_ne!(by_finger.view(), viewer().view(), "it panned");
        assert_eq!(by_finger.view(), by_mouse.view());
        assert_eq!(by_finger.selected(), by_mouse.selected());
    }

    #[test]
    fn a_one_finger_drag_scrolls_the_course_map_as_a_mouse_drag_does() {
        let from = course_empty();
        let path = [
            from,
            p(from.x + 30.0, from.y + 10.0),
            p(from.x + 60.0, from.y + 25.0),
        ];
        let mut by_mouse = course_map();
        mouse_drag(&mut by_mouse, &path);
        let mut by_finger = course_map();
        finger_drag(&mut by_finger, &path);
        assert_ne!(by_finger.place(), course_map().place(), "it scrolled");
        assert_eq!(by_finger.place(), by_mouse.place());
        assert_eq!(drawn(&by_finger), drawn(&by_mouse));
    }

    #[test]
    fn a_one_finger_tap_selects_a_system_on_the_viewer_as_a_click_does() {
        let mut by_mouse = viewer();
        let beta = by_mouse
            .view()
            .expect("the viewer")
            .world_to_screen(p(600.0, 0.0));
        mouse_drag(&mut by_mouse, &[beta]);
        let mut by_finger = viewer();
        crate::touch::testkit::tap(&mut by_finger, beta);
        assert_eq!(by_finger.selected(), Some(SystemId(129)));
        assert_eq!(by_finger.selected(), by_mouse.selected());
    }

    /// Two fingers pinching on `map` about `centre`: they land 90 apart,
    /// then spread to each of `spreads` in turn, the second finger
    /// moving, then lift.
    fn pinch(map: &mut GalaxyMap, centre: Point, spreads: &[f32]) {
        let mut fingers = Fingers::on(map);
        let (primary, second) = apart(centre, 90.0);
        fingers.down(1, primary);
        fingers.down(2, second);
        for &spread in spreads {
            fingers.move_to(2, p(primary.x + spread, primary.y));
        }
        fingers.up(
            2,
            p(
                primary.x + spreads.last().copied().unwrap_or(90.0),
                primary.y,
            ),
        );
        fingers.up(1, primary);
    }

    #[test]
    fn spreading_two_fingers_zooms_the_viewer_in_and_pinching_them_zooms_it_out() {
        let centre = p(512.0, 300.0);
        for (spreads, keys) in [
            (vec![120.0], (Key::Equal, 1)),
            (vec![120.0, 160.0], (Key::Equal, 2)),
            (vec![120.0, 160.0, 90.0], (Key::Equal, 0)),
            (vec![67.5], (Key::Minus, 1)),
            (vec![67.5, 50.625], (Key::Minus, 2)),
        ] {
            let mut pinched = viewer();
            pinch(&mut pinched, centre, &spreads);
            let mut keyed = viewer();
            key(&mut keyed, keys.0, keys.1);
            assert_eq!(pinched.view(), keyed.view(), "{spreads:?}");
        }
        let mut pinched = viewer();
        pinch(&mut pinched, centre, &[120.0]);
        assert_ne!(pinched.view(), viewer().view(), "a step zooms");
    }

    #[test]
    fn a_pinch_steps_the_course_maps_zoom_as_its_keys_do() {
        let centre = map_area().center();
        for (spreads, keys) in [
            (vec![120.0], vec![Key::Equal]),
            (vec![67.5], vec![Key::Minus]),
            (vec![67.5, 50.625], vec![Key::Minus, Key::Minus]),
            (vec![67.5, 90.0], vec![Key::Minus, Key::Equal]),
        ] {
            let mut pinched = course_map();
            pinch(&mut pinched, centre, &spreads);
            let mut keyed = course_map();
            for each in keys {
                key(&mut keyed, each, 1);
            }
            assert_eq!(pinched.place(), keyed.place(), "{spreads:?}");
            assert_eq!(drawn(&pinched), drawn(&keyed), "{spreads:?}: redrawn alike");
            assert_eq!(
                pinched.border_passes(),
                keyed.border_passes(),
                "{spreads:?}: the borders' passes alike"
            );
        }
    }

    /// How many presses of `key` the map takes before its zoom stops.
    fn steps_to_limit(build: fn() -> GalaxyMap, key: Key) -> usize {
        let mut map = build();
        let mut steps = 0;
        loop {
            let before = zoom(&map);
            self::key(&mut map, key, 1);
            if zoom(&map) == before {
                return steps;
            }
            steps += 1;
        }
    }

    #[test]
    fn a_pinch_stops_at_the_maps_zoom_limits_as_its_keys_do() {
        for (build, centre) in [
            (viewer as fn() -> GalaxyMap, p(512.0, 300.0)),
            (course_map, map_area().center()),
        ] {
            for (key, ratio) in [(Key::Equal, 4.0_f32 / 3.0), (Key::Minus, 0.75)] {
                let limit = steps_to_limit(build, key);
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                let past = (limit + 2) as i32;
                let mut pinched = build();
                pinch(&mut pinched, centre, &[90.0 * ratio.powi(past)]);
                let mut keyed = build();
                self::key(&mut keyed, key, limit + 2);
                assert_eq!(zoom(&pinched), zoom(&keyed), "{key:?}: at the limit");
            }
        }
    }

    #[test]
    fn spreading_past_a_limit_and_back_to_the_starting_spread_brings_back_the_starting_zoom() {
        for (build, centre) in [
            (viewer as fn() -> GalaxyMap, p(512.0, 300.0)),
            (course_map, map_area().center()),
        ] {
            for (key, ratio) in [(Key::Equal, 4.0_f32 / 3.0), (Key::Minus, 0.75)] {
                let limit = steps_to_limit(build, key);
                assert!(limit > 0, "{key:?}: the map starts off its limit");
                let start = zoom(&build());
                let mut map = build();
                let mut fingers = Fingers::on(&mut map);
                let (primary, second) = apart(centre, 90.0);
                fingers.down(1, primary);
                fingers.down(2, second);
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                let past = 90.0 * ratio.powi((limit + 3) as i32);
                fingers.move_to(2, p(primary.x + past, primary.y));
                fingers.move_to(2, p(primary.x + 90.0 * ratio, primary.y));
                fingers.move_to(2, second);
                fingers.up(2, second);
                fingers.up(1, primary);
                let back = if key == Key::Equal {
                    Key::Minus
                } else {
                    Key::Equal
                };
                let mut keyed = build();
                self::key(&mut keyed, key, limit);
                self::key(&mut keyed, back, limit);
                assert_eq!(zoom(&map), zoom(&keyed), "{key:?}: back where the keys go");
                assert!(
                    (zoom(&map) - start).abs() < 1e-5,
                    "{key:?}: back at the start"
                );
            }
        }
    }

    #[test]
    fn a_pinch_does_not_pan_the_viewer() {
        let drag = [VIEWER_EMPTY, p(330.0, 210.0)];
        let mut dragged = viewer();
        mouse_drag(&mut dragged, &drag);
        key(&mut dragged, Key::Equal, 1);

        let mut map = viewer();
        let mut fingers = Fingers::on(&mut map);
        fingers.down(1, drag[0]);
        fingers.move_to(1, drag[1]);
        fingers.down(2, p(drag[1].x + 90.0, drag[1].y));
        fingers.move_to(1, p(drag[1].x - 20.0, drag[1].y));
        fingers.move_to(2, p(drag[1].x + 100.0, drag[1].y));
        fingers.up(2, p(drag[1].x + 100.0, drag[1].y));
        fingers.move_to(1, p(drag[1].x + 200.0, drag[1].y + 50.0));
        fingers.up(1, p(drag[1].x + 200.0, drag[1].y + 50.0));
        assert_eq!(map.view(), dragged.view(), "only the zoom changed");
        assert_eq!(map.selected(), None, "the release selected nothing");
    }

    #[test]
    fn a_pinch_does_not_pan_the_course_map() {
        let from = course_empty();
        let drag = [from, p(from.x - 30.0, from.y - 10.0)];
        let mut dragged = course_map();
        mouse_drag(&mut dragged, &drag);
        key(&mut dragged, Key::Equal, 1);

        let mut map = course_map();
        let mut fingers = Fingers::on(&mut map);
        fingers.down(1, drag[0]);
        fingers.move_to(1, drag[1]);
        let second = p(drag[1].x - 90.0, drag[1].y);
        fingers.down(2, second);
        fingers.move_to(1, p(drag[1].x + 20.0, drag[1].y));
        fingers.move_to(2, p(drag[1].x - 100.0, drag[1].y));
        fingers.up(2, p(drag[1].x - 100.0, drag[1].y));
        fingers.move_to(1, p(drag[1].x + 40.0, drag[1].y + 30.0));
        fingers.up(1, p(drag[1].x + 40.0, drag[1].y + 30.0));
        assert_eq!(map.place(), dragged.place(), "only the zoom changed");
        assert_eq!(map.take_press(), None, "the release pressed nothing");
    }

    #[test]
    fn a_pinch_with_a_finger_off_the_map_zooms_nothing() {
        let plus = MapButtons::new(DIALOG_ORIGIN)
            .rect(MapButton::ZoomIn)
            .center();
        let mut clicked = course_map();
        mouse_drag(&mut clicked, &[plus]);
        let mut map = course_map();
        {
            let mut fingers = Fingers::on(&mut map);
            fingers.down(1, plus);
            let second = map_area().center();
            fingers.down(2, second);
            fingers.move_to(2, p(second.x + 300.0, second.y + 200.0));
            fingers.up(2, p(second.x + 300.0, second.y + 200.0));
            fingers.up(1, plus);
        }
        assert_eq!(map.place(), clicked.place(), "the press on + carried on");

        let panel = p(512.0, MAP_HEIGHT + 40.0);
        let mut clicked = viewer();
        mouse_drag(&mut clicked, &[panel]);
        let mut map = viewer();
        {
            let mut fingers = Fingers::on(&mut map);
            fingers.down(1, panel);
            fingers.down(2, p(512.0, 300.0));
            fingers.move_to(2, p(512.0, 100.0));
            fingers.up(2, p(512.0, 100.0));
            fingers.up(1, panel);
        }
        assert_eq!(map.view(), clicked.view(), "on the panel");
    }

    #[test]
    fn a_pinch_while_find_is_open_zooms_nothing() {
        let mut map = course_map();
        key(&mut map, Key::KeyF, 1);
        assert!(map.finding());
        let before = zoom(&map);
        map.input(&Input::Pinch(Pinch::Began {
            primary: course_empty(),
            second: map_area().center(),
        }));
        map.input(&Input::Pinch(Pinch::ZoomIn));
        assert_eq!(zoom(&map), before);
        assert!(map.finding());
    }

    #[test]
    fn cancelling_the_pointer_ends_a_pinch() {
        for build in [viewer as fn() -> GalaxyMap, course_map] {
            let mut map = build();
            let (primary, second) = apart(map_area().center(), 90.0);
            map.input(&Input::Pinch(Pinch::Began { primary, second }));
            map.cancel_pointer();
            let before = zoom(&map);
            map.input(&Input::Pinch(Pinch::ZoomIn));
            assert_eq!(zoom(&map), before);
        }
    }

    #[test]
    fn a_step_with_no_pinch_begun_zooms_nothing() {
        let mut map = viewer();
        let before = zoom(&map);
        map.input(&Input::Pinch(Pinch::ZoomIn));
        map.input(&Input::Pinch(Pinch::ZoomOut));
        assert_eq!(zoom(&map), before);
    }

    #[test]
    fn a_pinch_ended_zooms_no_more() {
        let mut map = viewer();
        let (primary, second) = apart(p(512.0, 300.0), 90.0);
        map.input(&Input::Pinch(Pinch::Began { primary, second }));
        map.input(&Input::Pinch(Pinch::Ended));
        let before = zoom(&map);
        map.input(&Input::Pinch(Pinch::ZoomIn));
        assert_eq!(zoom(&map), before);
    }
}
