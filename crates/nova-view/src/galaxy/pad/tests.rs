//! The in-flight maps' pad focus, fed pad inputs; and the map's row in
//! the pad's every-screen table.

#![allow(clippy::float_cmp)]

use std::rc::Rc;

use super::super::buttons::MapButtons;
use super::super::buttons::{MapButton, ORDER};
use super::super::catalog::SystemId;
use super::super::dialog::DIALOG_ORIGIN;
use super::super::map::{GalaxyMap, MapCycle};
use super::super::testkit::{FakeCatalog, galaxy};
use crate::draw::{DrawCommand, DrawList};
use crate::geometry::Bounds;
use crate::input::{Device, Input, Key, PadButton};
use crate::screen::Screen;
use crate::sound::Sound;
use crate::text::fixture::MonoMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::pad_focus::every_screen::{PAD, PadProbe, highlights, pad, pointer, told};

/// The course map in Alpha (128), with metrics and Find, Clear Route
/// enabled, and its zoom between its limits: all six buttons enabled.
fn map() -> GalaxyMap {
    let mut map = GalaxyMap::course(&FakeCatalog::new(galaxy()))
        .with_metrics(Rc::new(MonoMetrics))
        .with_find(None, ButtonStyle::STOCK);
    map.show_course(SystemId(128), &[]);
    map.set_clear_route(true);
    map.opened();
    map
}

fn rect(button: MapButton) -> Bounds {
    MapButtons::new(DIALOG_ORIGIN).rect(button)
}

fn tap(map: &mut GalaxyMap, button: PadButton) {
    map.input(&pad(button, true));
    map.input(&pad(button, false));
}

fn key(map: &mut GalaxyMap, key: Key) {
    for pressed in [true, false] {
        map.input(&Input::Key {
            key,
            pressed,
            repeat: false,
        });
    }
}

fn click(map: &mut GalaxyMap, button: MapButton) {
    let at = rect(button).center();
    map.input(&pointer(true, at));
    map.input(&pointer(false, at));
}

fn drawn(map: &GalaxyMap) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    map.draw(&mut list);
    list.iter().cloned().collect()
}

/// The button the pad's highlight surrounds, if any.
fn highlighted(map: &GalaxyMap) -> Option<MapButton> {
    let drawn = drawn(map);
    ORDER
        .into_iter()
        .find(|&button| highlights(&drawn, rect(button)))
}

/// Moves the pad's focus onto `button` with the d-pad, left from Done.
fn focus_on(map: &mut GalaxyMap, button: MapButton) {
    for _ in 0..ORDER.len() {
        if highlighted(map) == Some(button) {
            return;
        }
        tap(map, PadButton::DPadLeft);
    }
    assert_eq!(highlighted(map), Some(button), "the d-pad reaches it");
}

/// What the map has asked for or changed: the close, the route's clear,
/// the zoom, the borders and Find.
fn outcome(map: &mut GalaxyMap) -> (bool, bool, Option<f64>, bool, bool) {
    (
        map.take_close(),
        map.take_clear_route(),
        map.zoom(),
        map.borders_shown(),
        map.finding(),
    )
}

fn scroll(map: &GalaxyMap) -> (i32, i32) {
    map.place().expect("in flight").scroll()
}

#[test]
fn untold_the_dpad_and_south_do_nothing() {
    let mut map = map();
    let before = (scroll(&map), map.zoom());
    for button in [
        PadButton::DPadLeft,
        PadButton::DPadUp,
        PadButton::South,
        PadButton::DPadRight,
        PadButton::DPadDown,
        PadButton::South,
    ] {
        tap(&mut map, button);
    }
    assert_eq!(highlighted(&map), None);
    assert_eq!((scroll(&map), map.zoom()), before, "nothing moves");
    assert_eq!(map.selected(), None, "South doesn't centre");
    assert_eq!(map.take_sounds(), Vec::<Sound>::new());
    assert!(!map.take_close(), "nothing pressed");
}

#[test]
fn told_the_pad_the_focus_starts_on_done_and_south_closes_as_return_does() {
    let mut map = map();
    map.input(&told(PAD));
    assert_eq!(highlighted(&map), Some(MapButton::Done));
    map.input(&pad(PadButton::South, true));
    assert!(!map.take_close(), "on the release");
    map.input(&pad(PadButton::South, false));
    assert!(map.take_close());
    assert_eq!(map.take_sounds(), Vec::<Sound>::new(), "silent");
}

#[test]
fn the_dpad_moves_the_focus_among_the_buttons_and_never_pans() {
    let mut map = map();
    map.input(&told(PAD));
    let before = (scroll(&map), map.zoom());
    // Left to right: Borders, Clear Route, Find, "-", "+", Done.
    let mut seen = vec![highlighted(&map)];
    for _ in 0..5 {
        tap(&mut map, PadButton::DPadLeft);
        seen.push(highlighted(&map));
    }
    assert_eq!(
        seen,
        [
            MapButton::Done,
            MapButton::ZoomIn,
            MapButton::ZoomOut,
            MapButton::Find,
            MapButton::ClearRoute,
            MapButton::Borders,
        ]
        .map(Some)
    );
    tap(&mut map, PadButton::DPadLeft);
    tap(&mut map, PadButton::DPadUp);
    tap(&mut map, PadButton::DPadDown);
    assert_eq!(
        highlighted(&map),
        Some(MapButton::Borders),
        "nothing that way"
    );
    tap(&mut map, PadButton::DPadRight);
    assert_eq!(highlighted(&map), Some(MapButton::ClearRoute));
    assert_eq!((scroll(&map), map.zoom()), before, "no pan, no zoom");
}

#[test]
fn the_left_stick_pans_as_the_arrows_do() {
    for (stick, arrow) in [
        (PadButton::LeftStickLeft, Key::ArrowLeft),
        (PadButton::LeftStickRight, Key::ArrowRight),
        (PadButton::LeftStickUp, Key::ArrowUp),
        (PadButton::LeftStickDown, Key::ArrowDown),
    ] {
        let mut padded = map();
        padded.input(&told(PAD));
        let before = scroll(&padded);
        tap(&mut padded, stick);
        let mut keyed = map();
        key(&mut keyed, arrow);
        assert_ne!(scroll(&padded), before, "{stick:?} pans");
        assert_eq!(scroll(&padded), scroll(&keyed), "{stick:?}");
        assert_eq!(
            highlighted(&padded),
            Some(MapButton::Done),
            "the focus stays"
        );
    }
}

#[test]
fn the_triggers_zoom_as_minus_and_equals_do() {
    for (trigger, zoom_key) in [
        (PadButton::RightTrigger, Key::Equal),
        (PadButton::LeftTrigger, Key::Minus),
    ] {
        let mut padded = map();
        padded.input(&told(PAD));
        let mut keyed = map();
        let mut changed = false;
        // Past the limit: the last pulls change nothing, as the keys don't.
        for _ in 0..8 {
            let before = padded.zoom();
            tap(&mut padded, trigger);
            key(&mut keyed, zoom_key);
            changed |= padded.zoom() != before;
            assert_eq!(padded.zoom(), keyed.zoom(), "{trigger:?}");
            assert_eq!(scroll(&padded), scroll(&keyed), "{trigger:?}");
        }
        assert!(changed, "{trigger:?} zooms");
    }
}

#[test]
fn south_on_each_button_does_what_a_click_on_it_does() {
    for button in ORDER {
        let mut padded = map();
        padded.input(&told(PAD));
        focus_on(&mut padded, button);
        tap(&mut padded, PadButton::South);
        let mut clicked = map();
        click(&mut clicked, button);
        let clicked = outcome(&mut clicked);
        assert_eq!(outcome(&mut padded), clicked, "{button:?}");
        assert_ne!(outcome(&mut map()), clicked, "{button:?} acts");
    }
}

#[test]
fn a_disabled_button_is_no_target_and_a_focus_left_on_one_draws_nothing() {
    let mut map = map();
    map.input(&told(PAD));
    focus_on(&mut map, MapButton::ZoomIn);
    tap(&mut map, PadButton::South);
    assert!(
        !map.place().expect("in flight").can_zoom_in(),
        "at the limit"
    );
    assert_eq!(highlighted(&map), None, "+ is disabled");
    let zoom = map.zoom();
    tap(&mut map, PadButton::South);
    assert_eq!(map.zoom(), zoom, "South presses nothing");
    tap(&mut map, PadButton::DPadLeft);
    assert_eq!(highlighted(&map), Some(MapButton::Done), "back on Done");
    tap(&mut map, PadButton::DPadLeft);
    assert_eq!(highlighted(&map), Some(MapButton::ZoomOut), "+ is skipped");
}

#[test]
fn the_right_shoulder_still_cycles_and_the_left_steps_back_as_tab_back_does() {
    for tell in [false, true] {
        let mut map = map();
        if tell {
            map.input(&told(PAD));
        }
        let focus = highlighted(&map);
        tap(&mut map, PadButton::RightShoulder);
        assert_eq!(map.take_cycle(), Some(MapCycle { back: false }), "{tell}");
        tap(&mut map, PadButton::LeftShoulder);
        assert_eq!(map.take_cycle(), Some(MapCycle { back: true }), "{tell}");
        assert_eq!(highlighted(&map), focus, "no focus moves");
    }
}

#[test]
fn keyboard_space_still_centres_and_a_pads_south_does_not() {
    let mut keyed = map();
    keyed.input(&told(PAD));
    key(&mut keyed, Key::Space);
    assert_eq!(keyed.selected(), Some(SystemId(128)), "centred");
    assert_eq!(
        keyed.take_sounds(),
        vec![Sound::Ui(crate::sound::UiSound::MapCentred)]
    );
    let mut padded = map();
    padded.input(&told(PAD));
    tap(&mut padded, PadButton::South);
    assert_eq!(padded.selected(), None, "not centred");
    assert_eq!(padded.take_sounds(), Vec::<Sound>::new());
    assert!(padded.take_close(), "Done pressed");
}

#[test]
fn the_surroundings_are_no_player_input() {
    let mut map = map();
    let at = rect(MapButton::Find).center();
    map.input(&pointer(true, at));
    let update = map.update();
    map.input(&told(PAD));
    map.input(&told(Device::Keyboard));
    assert_eq!(map.update(), update, "no redraw");
    assert!(!map.take_close() && !map.take_clear_route(), "no request");
    assert!(map.borders_shown(), "the borders untouched");
    map.input(&pointer(false, at));
    assert!(map.finding(), "the press wasn't abandoned");
}

#[test]
fn find_opens_told_the_device_the_map_was_told() {
    let mut map = map();
    map.input(&told(PAD));
    focus_on(&mut map, MapButton::Find);
    tap(&mut map, PadButton::South);
    assert!(map.finding());
    tap(&mut map, PadButton::South);
    assert!(!map.finding(), "Find's own focus pressed Find");
    assert_eq!(map.take_find(), None, "an empty text searches nothing");

    let mut untold = self::map();
    key(&mut untold, Key::KeyF);
    assert!(untold.finding());
    tap(&mut untold, PadButton::South);
    assert!(untold.finding(), "untold, Find holds its focus back");
}

#[test]
fn an_open_find_follows_the_surroundings() {
    let mut map = map();
    key(&mut map, Key::KeyF);
    assert!(map.finding());
    map.input(&told(PAD));
    tap(&mut map, PadButton::South);
    assert!(!map.finding(), "told while open");
}

#[test]
fn each_open_starts_the_focus_on_done() {
    let mut map = map();
    map.input(&told(PAD));
    focus_on(&mut map, MapButton::Borders);
    map.input(&pad(PadButton::South, true));
    map.opened();
    assert_eq!(highlighted(&map), Some(MapButton::Done));
    map.input(&pad(PadButton::South, false));
    assert!(map.borders_shown(), "the press before the open is dropped");
    assert!(!map.take_close(), "and no other button takes its release");
}

#[test]
fn letting_go_of_the_keys_or_the_pointer_drops_south_s_press() {
    for drop in [<GalaxyMap as Screen>::release_keys, Screen::cancel_pointer] {
        let mut map = map();
        map.input(&told(PAD));
        map.input(&pad(PadButton::South, true));
        drop(&mut map);
        map.input(&pad(PadButton::South, false));
        assert!(!map.take_close());
    }
}

#[test]
fn the_viewer_has_no_pad_focus() {
    let mut viewer = GalaxyMap::new(&FakeCatalog::new(galaxy()));
    viewer.input(&told(PAD));
    assert_eq!(viewer.device(), PAD);
    let before = drawn(&viewer);
    for button in [PadButton::DPadLeft, PadButton::DPadUp, PadButton::South] {
        tap(&mut viewer, button);
    }
    assert_eq!(drawn(&viewer), before, "no highlight, nothing moved");
    assert_eq!(viewer.take_entry(), None, "South enters nothing");
}

#[test]
fn the_map_keeps_the_device_it_was_told() {
    let mut map = map();
    assert_eq!(map.device(), Device::Keyboard);
    map.input(&told(PAD));
    assert_eq!(map.device(), PAD);
    map.input(&told(Device::Mouse));
    assert_eq!(map.device(), Device::Mouse);
}

/// The map's row in the pad's every-screen table: its six buttons.
struct MapProbe(GalaxyMap);

impl PadProbe for MapProbe {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.0
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        ORDER
            .into_iter()
            .map(|button| (name(button), rect(button)))
            .collect()
    }

    fn outcome(&mut self) -> String {
        format!("{:?}", outcome(&mut self.0))
    }

    fn default_control(&self) -> Option<&'static str> {
        Some(name(MapButton::Done))
    }
}

const fn name(button: MapButton) -> &'static str {
    match button {
        MapButton::Done => "Done",
        MapButton::ZoomOut => "-",
        MapButton::ZoomIn => "+",
        MapButton::ClearRoute => "Clear Route",
        MapButton::Borders => "Borders",
        MapButton::Find => "Find",
    }
}

/// The course map with all six buttons enabled.
pub(crate) fn pad_probe() -> Box<dyn PadProbe> {
    Box::new(MapProbe(map()))
}
