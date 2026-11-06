//! The map's pad cursor: alone, moved by the map's ticks, and pressing
//! as a click does.

#![allow(clippy::float_cmp)]

use std::time::Duration;

use nova_sim::story::{GalaxyOverlay, OverlaidGalaxy};

use super::*;
use crate::draw::{DrawCommand, DrawList};
use crate::galaxy::catalog::Galaxy;
use crate::galaxy::click::PressAnswer;
use crate::galaxy::dialog::in_map_item;
use crate::galaxy::map::MapMode;
use crate::galaxy::model::star_system;
use crate::galaxy::place::MapPlace;
use crate::galaxy::testkit::{FakeCatalog, galaxy, system};
use crate::input::{Input, Key, MouseButton, PadAxis, PadButton};
use crate::screen::Screen;
use crate::sound::UiSound;
use crate::ui::pad_focus::every_screen::{PAD, pad, told};

/// The map item's centre, where the scroll draws: (450, 346).
const CENTRE: Point = Point::new(450.0, 346.0);

/// A tenth of a second.
const TENTH: Duration = Duration::from_millis(100);

/// The course map's transform drawing (0, 0) at (450, 346), at `zoom`.
fn transform(zoom: f64) -> Transform {
    MapPlace::new((0, 0))
        .with_zoom(zoom)
        .transform((212, 128), (238, 218))
}

fn model(systems: &[(i16, i16, i16)]) -> GalaxyModel {
    GalaxyModel::new(Galaxy {
        systems: systems.iter().map(|&(id, x, y)| system(id, x, y)).collect(),
        ..Galaxy::default()
    })
}

fn at(x: f32, y: f32) -> MapCursor {
    MapCursor::at_point(Point::new(x, y))
}

fn ids(target: Option<(SystemId, Point)>) -> Option<i16> {
    target.map(|(id, _)| id.0)
}

#[test]
fn full_deflection_moves_300_points_a_second() {
    let mut cursor = MapCursor::default();
    cursor.advance((1.0, 0.0), TENTH);
    assert!(
        (cursor.at().x - (CENTRE.x + 30.0)).abs() < 1e-3,
        "{cursor:?}"
    );
    assert_eq!(cursor.at().y, CENTRE.y);
    cursor.advance((-0.5, 0.0), TENTH);
    assert!(
        (cursor.at().x - (CENTRE.x + 15.0)).abs() < 1e-3,
        "half a push"
    );
}

#[test]
fn pushed_up_moves_up_the_screen() {
    let mut cursor = MapCursor::default();
    cursor.advance((0.0, 1.0), TENTH);
    assert!(
        (cursor.at().y - (CENTRE.y - 30.0)).abs() < 1e-3,
        "{cursor:?}"
    );
    assert_eq!(cursor.at().x, CENTRE.x);
}

#[test]
fn no_amount_no_move() {
    let mut cursor = MapCursor::default();
    cursor.advance((0.0, 0.0), Duration::from_secs(5));
    assert_eq!(cursor, MapCursor::default());
}

#[test]
fn it_stays_inside_item_3() {
    for push in [(1.0, 1.0), (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)] {
        let mut cursor = MapCursor::default();
        cursor.advance(push, Duration::from_secs(10));
        assert!(in_map_item(cursor.at()), "{push:?}: {cursor:?}");
    }
    let mut cursor = MapCursor::default();
    cursor.advance((-1.0, 1.0), Duration::from_secs(10));
    assert_eq!(cursor.at(), map_area().min, "the top-left corner");
    cursor.advance((1.0, -1.0), Duration::from_secs(10));
    let max = map_area().max;
    assert_eq!(
        cursor.at(),
        Point::new(max.x - 1.0, max.y - 1.0),
        "the bottom-right pixel"
    );
}

#[test]
fn it_targets_the_nearest_system_within_16_points() {
    let model = model(&[(128, 0, 0), (129, 12, 0)]);
    let transform = transform(1.0);
    assert_eq!(ids(at(458.0, 346.0).target(&model, &transform)), Some(129));
    assert_eq!(ids(at(455.0, 346.0).target(&model, &transform)), Some(128));
    assert_eq!(
        ids(at(450.0, 362.0).target(&model, &transform)),
        Some(128),
        "16 down"
    );
    assert_eq!(
        at(458.0, 346.0).target(&model, &transform).map(|(_, p)| p),
        Some(Point::new(462.0, 346.0)),
        "its whole-pixel point"
    );
}

#[test]
fn beyond_16_points_it_targets_nothing() {
    let model = model(&[(128, 0, 0)]);
    let transform = transform(1.0);
    assert_eq!(at(450.0, 362.5).target(&model, &transform), None);
    assert_eq!(at(462.0, 358.0).target(&model, &transform), None);
}

#[test]
fn a_tie_goes_to_the_lower_id() {
    let model = model(&[(129, 10, 0), (130, -10, 0)]);
    assert_eq!(
        ids(MapCursor::default().target(&model, &transform(1.0))),
        Some(129)
    );
    let model = self::model(&[(129, -10, 0), (130, 10, 0)]);
    assert_eq!(
        ids(MapCursor::default().target(&model, &transform(1.0))),
        Some(129)
    );
}

#[test]
fn a_hidden_system_is_never_a_target() {
    let mut model = GalaxyModel::new(galaxy());
    let transform = transform(1.0);
    assert_eq!(
        ids(MapCursor::default().target(&model, &transform)),
        Some(128)
    );
    let mut overlay = GalaxyOverlay::default();
    overlay.set_visible(SystemId(128), Some(false));
    overlay.set_visible(SystemId(131), Some(false));
    let systems = galaxy().systems.iter().map(star_system).collect();
    model.apply(&nova_sim::galaxy_map::GalaxyShown::of(
        &OverlaidGalaxy::new(systems, overlay),
    ));
    assert_eq!(MapCursor::default().target(&model, &transform), None);
}

#[test]
fn inside_any_hit_box_it_has_a_target() {
    for zoom in [1.0, 0.5625] {
        let model = model(&[(128, 40, 20)]);
        let transform = transform(zoom);
        let (h, v) = transform.pixel(Point::new(40.0, 20.0));
        #[allow(clippy::cast_precision_loss)] // Screen pixels.
        let (h, v) = (h as f32, v as f32);
        for (x, y) in [
            (h - 10.0, v - 10.0),
            (h + 9.99, v - 10.0),
            (h - 10.0, v + 9.99),
            (h + 9.99, v + 9.99),
        ] {
            assert_eq!(
                ids(at(x, y).target(&model, &transform)),
                Some(128),
                "{zoom}: ({x}, {y})"
            );
        }
    }
}

#[test]
fn the_press_point_is_the_targets_pixel_point_or_the_free_point() {
    let model = model(&[(128, 0, 0)]);
    let transform = transform(1.0);
    assert_eq!(at(455.5, 349.5).press_point(&model, &transform), CENTRE);
    let free = Point::new(500.5, 400.5);
    assert_eq!(
        MapCursor::at_point(free).press_point(&model, &transform),
        free
    );
}

#[test]
fn reset_puts_it_at_item_3s_centre() {
    let mut cursor = at(300.0, 200.0);
    cursor.reset();
    assert_eq!(cursor.at(), CENTRE);
    assert_eq!(MapCursor::default().at(), CENTRE);
}

// The map moving its cursor.

/// Systems the map tests share: 128 at (0, 0), drawn at item 3's centre
/// at any zoom; 129 at (100, 0); 130 at (0, 100), linked from nothing.
fn spread() -> Galaxy {
    Galaxy {
        systems: vec![system(128, 0, 0), system(129, 100, 0), system(130, 0, 100)],
        ..Galaxy::default()
    }
}

/// The course map of [`spread`] in 128, opened and told the pad.
fn course() -> GalaxyMap {
    let mut map = GalaxyMap::course(&FakeCatalog::new(spread()));
    map.show_course(SystemId(128), &[]);
    map.opened();
    map.input(&told(PAD));
    map
}

fn stick(axis: PadAxis, value: f32) -> Input {
    Input::PadAxis { axis, value }
}

/// Pushes the right stick to (`x`, `y`), ticks the map `ticks` tenths of
/// a second, then lets the stick go; the ticks.
fn push(map: &mut GalaxyMap, (x, y): (f32, f32), ticks: u32) -> u32 {
    map.input(&stick(PadAxis::RightX, x));
    map.input(&stick(PadAxis::RightY, y));
    for _ in 0..ticks {
        map.tick(TENTH);
    }
    map.input(&stick(PadAxis::RightX, 0.0));
    map.input(&stick(PadAxis::RightY, 0.0));
    ticks
}

/// Ticks `map` `ticks` tenths of a second, as its twin was while its
/// cursor moved: the borders' null events run on each.
fn idle(map: &mut GalaxyMap, ticks: u32) {
    for _ in 0..ticks {
        map.tick(TENTH);
    }
}

/// Where system `id` draws on `map`, as a whole pixel.
fn pixel_of(map: &GalaxyMap, id: i16) -> Point {
    let position = map.model().system(SystemId(id)).expect("drawn").position();
    let (h, v) = map.transform().pixel(position);
    #[allow(clippy::cast_precision_loss)] // Screen pixels.
    Point::new(h as f32, v as f32)
}

/// Moves `map`'s cursor onto system `id` with the stick: across, then
/// down or up, a full push for whole tenths and the rest at a fraction;
/// the ticks it took.
fn steer_onto(map: &mut GalaxyMap, id: i16) -> u32 {
    let mut ticks = 0;
    let goal = pixel_of(map, id);
    let from = map.cursor().expect("in flight");
    let (dx, dy) = (goal.x - from.x, goal.y - from.y);
    for (amount, axis) in [(dx, 0), (dy, 1)] {
        let tenths = amount.abs() / 30.0;
        let whole = tenths.floor();
        let sign = amount.signum();
        let pushed = |value: f32| {
            if axis == 0 {
                (value * sign, 0.0)
            } else {
                (0.0, -value * sign)
            }
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let full = whole as u32;
        ticks += push(map, pushed(1.0), full);
        // The rest, past the dead zone's rescale: a push p reads
        // (p - 0.15) / 0.85.
        let rest = tenths - whole;
        if rest > 0.0 {
            ticks += push(map, pushed(0.15 + rest * 0.85), 1);
        }
    }
    let at = map.cursor().expect("in flight");
    assert!(
        (at.x - goal.x).abs() < 2.0 && (at.y - goal.y).abs() < 2.0,
        "{at:?} near {goal:?}"
    );
    ticks
}

fn tap(map: &mut GalaxyMap, button: PadButton) {
    map.input(&pad(button, true));
    map.input(&pad(button, false));
}

#[test]
fn pad_axis_inputs_then_a_tick_move_the_cursor_by_amount_speed_and_time() {
    let mut map = course();
    assert_eq!(map.cursor(), Some(CENTRE));
    push(&mut map, (1.0, 0.0), 1);
    let at = map.cursor().expect("in flight");
    assert!((at.x - (CENTRE.x + 30.0)).abs() < 1e-3, "{at:?}");
    push(&mut map, (0.0, 1.0), 2);
    let at = map.cursor().expect("in flight");
    assert!((at.y - (CENTRE.y - 60.0)).abs() < 1e-3, "up: {at:?}");
}

#[test]
fn inside_the_dead_zone_it_holds_still() {
    let mut map = course();
    push(&mut map, (0.1, -0.1), 10);
    assert_eq!(map.cursor(), Some(CENTRE));
}

#[test]
fn while_find_is_open_it_holds_still() {
    let mut map = GalaxyMap::course(&FakeCatalog::new(spread()))
        .with_metrics(std::rc::Rc::new(crate::text::fixture::MonoMetrics));
    map.opened();
    map.input(&told(PAD));
    map.input(&Input::Key {
        key: Key::KeyF,
        pressed: true,
        repeat: false,
    });
    assert!(map.finding());
    push(&mut map, (1.0, 1.0), 5);
    assert_eq!(map.cursor(), Some(CENTRE));
}

#[test]
fn let_go_stops_it() {
    let mut map = course();
    map.input(&stick(PadAxis::RightX, 1.0));
    map.tick(TENTH);
    let moved = map.cursor();
    map.release_keys();
    map.tick(TENTH);
    assert_eq!(map.cursor(), moved);
}

#[test]
fn each_open_puts_it_back_at_the_centre() {
    let mut map = course();
    push(&mut map, (1.0, 1.0), 3);
    assert_ne!(map.cursor(), Some(CENTRE));
    map.opened();
    assert_eq!(map.cursor(), Some(CENTRE));
}

#[test]
fn the_viewer_has_no_cursor() {
    let mut map = GalaxyMap::new(&FakeCatalog::new(spread()));
    map.input(&told(PAD));
    push(&mut map, (1.0, 1.0), 3);
    assert_eq!(map.cursor(), None);
}

// South over the map, as a click.

fn left(map: &mut GalaxyMap, pressed: bool, at: Point) {
    map.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    });
}

/// The course map of [`spread`] in 128 in `mode`, opened and told the pad.
fn twin(mode: MapMode) -> GalaxyMap {
    let mut map = course();
    if mode == MapMode::Hypergate {
        map.offer_gates(vec![SystemId(129), SystemId(130)]);
        assert_eq!(map.mode(), MapMode::Hypergate);
    }
    map
}

/// Twin maps in `mode`: the first clicked on 130 with the mouse, the
/// second steered onto it with the stick and pressed with South, `west`
/// held on both (Shift on the mouse's). Then each press is answered
/// selecting 130, and released.
fn clicked_and_south(mode: MapMode, west: bool) -> (GalaxyMap, GalaxyMap) {
    let mut mouse = twin(mode);
    let mut pad_map = twin(mode);
    let at = pixel_of(&mouse, 130);
    if west {
        mouse.input(&Input::Key {
            key: Key::ShiftLeft,
            pressed: true,
            repeat: false,
        });
        pad_map.input(&pad(PadButton::West, true));
    }
    let ticks = steer_onto(&mut pad_map, 130);
    idle(&mut mouse, ticks);
    left(&mut mouse, true, at);
    pad_map.input(&pad(PadButton::South, true));
    let clicked = mouse.take_press().expect("the click's press");
    let south = pad_map.take_press().expect("South's press");
    assert_eq!(south, clicked);
    assert_eq!(south.hits, vec![SystemId(130)]);
    assert_eq!(south.shift, west);
    let answer = Some(PressAnswer {
        selected: Some(SystemId(130)),
        gate_pick: (mode == MapMode::Hypergate).then_some(Some(SystemId(130))),
        clear_route: None,
    });
    mouse.answer_press(answer);
    pad_map.answer_press(answer);
    left(&mut mouse, false, at);
    pad_map.input(&pad(PadButton::South, false));
    (mouse, pad_map)
}

#[test]
fn south_on_a_system_presses_as_a_left_click_on_it_does() {
    let (mut mouse, mut pad_map) = clicked_and_south(MapMode::Course, false);
    assert_eq!(pad_map.selected(), Some(SystemId(130)));
    assert_eq!(pad_map.selected(), mouse.selected());
    assert_eq!(pad_map.take_ui_sounds(), Vec::<UiSound>::new(), "silent");
    assert_eq!(mouse.take_ui_sounds(), Vec::<UiSound>::new());
    assert_eq!(pad_map.border_passes(), mouse.border_passes());
    assert_eq!(pad_map.place(), mouse.place());
}

#[test]
fn an_unaccepted_south_drags_nothing_and_runs_the_borders_pass_as_the_click() {
    let mut mouse = course();
    let mut pad_map = course();
    let at = pixel_of(&mouse, 130);
    let ticks = steer_onto(&mut pad_map, 130);
    idle(&mut mouse, ticks);
    left(&mut mouse, true, at);
    pad_map.input(&pad(PadButton::South, true));
    assert_eq!(pad_map.take_press(), mouse.take_press());
    mouse.answer_press(None);
    pad_map.answer_press(None);
    left(&mut mouse, false, at);
    pad_map.input(&pad(PadButton::South, false));
    assert_eq!(pad_map.border_passes(), mouse.border_passes());
    assert_eq!(pad_map.place(), mouse.place(), "nothing dragged");
    assert_eq!(pad_map.selected(), None);
}

#[test]
fn west_held_makes_it_a_plot_course_press() {
    clicked_and_south(MapMode::Course, true);
}

#[test]
fn on_the_hypergate_map_it_presses_the_same() {
    let (mouse, pad_map) = clicked_and_south(MapMode::Hypergate, false);
    assert_eq!(pad_map.hyper_selection(), mouse.hyper_selection());
}

#[test]
fn south_off_every_system_presses_at_the_free_point() {
    let mut mouse = course();
    let mut pad_map = course();
    idle(&mut mouse, push(&mut pad_map, (1.0, 1.0), 2));
    let free = pad_map.cursor().expect("in flight");
    left(&mut mouse, true, free);
    pad_map.input(&pad(PadButton::South, true));
    assert_eq!(pad_map.take_press(), None, "nothing hit");
    assert_eq!(mouse.take_press(), None);
    left(&mut mouse, false, free);
    pad_map.input(&pad(PadButton::South, false));
    assert_eq!(
        pad_map.border_passes(),
        mouse.border_passes(),
        "the drag's end"
    );
    assert_eq!(pad_map.place(), mouse.place());
    // A pointer moved after the release drags nothing: the press ended.
    let place = pad_map.place().copied();
    pad_map.input(&Input::PointerMoved(Point::new(300.0, 300.0)));
    assert_eq!(pad_map.place().copied(), place);
}

#[test]
fn focus_lost_ends_souths_press_on_the_map() {
    let mut map = course();
    push(&mut map, (1.0, 1.0), 2);
    map.input(&pad(PadButton::South, true));
    map.release_keys();
    let place = map.place().copied();
    map.input(&Input::PointerMoved(Point::new(300.0, 300.0)));
    assert_eq!(map.place().copied(), place, "no drag left behind");
}

// North: the centre.

/// [`course`], scrolled away from 128 and with 129 selected by a click.
fn moved_course() -> GalaxyMap {
    let mut map = course();
    map.centre_on_system(SystemId(129));
    let at = pixel_of(&map, 129);
    left(&mut map, true, at);
    map.take_press().expect("a press");
    map.answer_press(Some(PressAnswer {
        selected: Some(SystemId(129)),
        gate_pick: None,
        clear_route: None,
    }));
    left(&mut map, false, at);
    assert_eq!(map.selected(), Some(SystemId(129)));
    map.take_ui_sounds();
    map
}

#[test]
fn north_centres_the_map_as_space_does() {
    let mut space = moved_course();
    let mut north = moved_course();
    space.input(&Input::Key {
        key: Key::Space,
        pressed: true,
        repeat: false,
    });
    tap(&mut north, PadButton::North);
    assert_eq!(north.place(), space.place());
    assert_eq!(north.selected(), Some(SystemId(128)));
    assert_eq!(north.selected(), space.selected());
    assert_eq!(north.take_ui_sounds(), vec![UiSound::MapCentred]);
    assert_eq!(space.take_ui_sounds(), vec![UiSound::MapCentred]);
    assert_eq!(north.border_passes(), space.border_passes());
}

#[test]
fn north_repeats_nothing() {
    let mut map = moved_course();
    map.input(&pad(PadButton::North, true));
    assert_eq!(map.take_ui_sounds(), vec![UiSound::MapCentred]);
    map.input(&Input::PadButton {
        button: PadButton::North,
        pressed: true,
        repeat: true,
    });
    map.input(&pad(PadButton::North, false));
    assert_eq!(map.take_ui_sounds(), Vec::<UiSound>::new());
}

#[test]
fn north_does_nothing_on_the_viewer() {
    let mut map = GalaxyMap::new(&FakeCatalog::new(spread()));
    map.input(&told(PAD));
    let view = *map.view().expect("the viewer");
    tap(&mut map, PadButton::North);
    assert_eq!(*map.view().expect("the viewer"), view);
    assert_eq!(map.take_ui_sounds(), Vec::<UiSound>::new());
}

#[test]
fn centre_puts_the_cursor_back_at_the_centre() {
    let mut map = moved_course();
    push(&mut map, (1.0, 1.0), 2);
    assert_ne!(map.cursor(), Some(CENTRE));
    tap(&mut map, PadButton::North);
    assert_eq!(map.cursor(), Some(CENTRE));
}

// Drawing.

fn drawn(map: &GalaxyMap) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    map.draw(&mut list);
    list.iter().cloned().collect()
}

/// What `cursor` alone draws on `map`, in the pad highlight's colour.
fn cursor_alone(map: &GalaxyMap, cursor: MapCursor) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    cursor.draw(
        &mut list,
        map.model(),
        &map.transform(),
        crate::ui::button::ButtonStyle::STOCK.up,
    );
    list.iter().cloned().collect()
}

/// Whether `list` holds `part` as a run of commands, in order.
fn holds(list: &[DrawCommand], part: &[DrawCommand]) -> bool {
    !part.is_empty() && list.windows(part.len()).any(|window| window == part)
}

/// [`course`], its cursor moved off every system.
fn free_course(device: Option<crate::input::Device>) -> GalaxyMap {
    let mut map = GalaxyMap::course(&FakeCatalog::new(spread()));
    map.show_course(SystemId(128), &[]);
    map.opened();
    if let Some(device) = device {
        map.input(&told(device));
    }
    push(&mut map, (1.0, 1.0), 2);
    map
}

#[test]
fn the_cursor_draws_only_while_the_pad_was_used_last() {
    use crate::input::Device;
    let map = free_course(Some(PAD));
    let crosshair = cursor_alone(&map, MapCursor::at_point(map.cursor().expect("in flight")));
    assert_eq!(crosshair.len(), 4, "four arms");
    assert!(holds(&drawn(&map), &crosshair), "drawn told the pad");
    for device in [
        Some(Device::Keyboard),
        Some(Device::Touch),
        Some(Device::Mouse),
        None,
    ] {
        let map = free_course(device);
        assert!(!holds(&drawn(&map), &crosshair), "{device:?}");
    }
}

#[test]
fn a_target_is_ringed_round_its_hit_box() {
    let map = course();
    let ring = cursor_alone(&map, MapCursor::default());
    let mut expected = DrawList::new();
    draw_highlight(
        &mut expected,
        hit_box(450, 346),
        crate::ui::button::ButtonStyle::STOCK.up,
    );
    assert_eq!(ring, expected.iter().cloned().collect::<Vec<_>>());
    assert!(holds(&drawn(&map), &ring));
}

#[test]
fn the_crosshair_is_clipped_to_item_3() {
    let mut map = free_course(Some(PAD));
    push(&mut map, (1.0, -1.0), 30);
    let corner = map.cursor().expect("in flight");
    let max = map_area().max;
    assert_eq!(corner, Point::new(max.x - 1.0, max.y - 1.0), "the corner");
    // Right, left, down, up: the right and down arms lie past item 3.
    let arms = cursor_alone(&map, MapCursor::at_point(corner));
    let drawn = drawn(&map);
    let shown: Vec<bool> = arms.iter().map(|arm| drawn.contains(arm)).collect();
    assert_eq!(shown, vec![false, true, false, true]);
}

#[test]
fn the_viewer_draws_no_cursor() {
    let mut map = GalaxyMap::new(&FakeCatalog::new(spread()));
    map.input(&told(PAD));
    let crosshair = cursor_alone(&map, MapCursor::default());
    assert!(!holds(&drawn(&map), &crosshair));
}
