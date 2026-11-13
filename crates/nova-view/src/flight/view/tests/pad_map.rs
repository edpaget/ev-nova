//! The course map from a gamepad (rdm:phase/key-bindings/phase-10-gamepad-map):
//! the map told the device whatever opens it, the pad's new map commands
//! taken by the map's filter with the map key held, and the pad's map
//! chord over a screen, read by the pad reader the router feeds
//! ([`FlightView::note_pad`]). The map's own focus rules are the
//! `galaxy::pad` module's, and its cursor's the `galaxy::cursor` module's:
//! here the sticks and ticks reach the cursor over flight and over the
//! port (rdm:phase/key-bindings/phase-20-gamepad-map-cursor).
//!
//! On flight's plain map, flight still holds the keys it reads with no
//! latch (`track_unlatched`), the pad's d-pad and right trigger among
//! them: one still down as the map closes acts in flight, as a key held
//! across the map does in the original. South closes the map on its
//! release, after flight has seen that release, so no shot fires.

use super::*;
use crate::flight::PortScreen;
use crate::input::{ControllerFamily, Device, PadButton, Surroundings};
use crate::keymap::{Bindings, PadBinding, PadEntry, PlayCommand, SharedBindings};
use nova_sim_frame::MissionId;

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

fn told(view: &mut Faked, device: Device) {
    view.input(&Input::Surroundings(Surroundings::of(device)));
}

fn pad_input(button: PadButton, pressed: bool) -> Input {
    Input::PadButton {
        button,
        pressed,
        repeat: false,
    }
}

/// `button` over flight, where flight takes the input.
fn pad(view: &mut Faked, button: PadButton, pressed: bool) {
    view.input(&pad_input(button, pressed));
}

/// `button` as the router gives it over a screen: noted by flight's pad
/// reader, then to the map while it is open over the screen, otherwise to
/// the screen, which flight never sees.
fn over(view: &mut Faked, button: PadButton, pressed: bool) {
    let input = pad_input(button, pressed);
    view.note_pad(&input);
    if view.map_open() {
        view.port_map_input(&input);
    }
}

fn tap_over(view: &mut Faked, button: PadButton) {
    over(view, button, true);
    over(view, button, false);
}

/// A flight docked at Earth.
fn docked() -> Faked {
    faked_sol(FlightFrame {
        landed: Some(StellarId(128)),
        ..in_sol()
    })
}

/// A map's opener, by name.
type Opener = (&'static str, fn(&mut Faked));

/// Each map opened over a screen on the map key: the bar's, a port
/// screen's and a mission dialog's, docked.
const OPENERS: [Opener; 3] = [
    ("bar", Faked::open_bar_map),
    ("port screen", |view| {
        view.open_port_screen_map(PortScreen::Spaceport);
    }),
    ("dialog", |view| {
        view.open_dialog_map(MapOpener::Offer(MissionId(130)));
    }),
];

/// `view` with a map opened over a screen by `open` on the pad's chord:
/// Start, then the right stick up, the screen asking for the map.
fn chord_opened_over(mut view: Faked, open: fn(&mut Faked)) -> Faked {
    over(&mut view, PadButton::Start, true);
    over(&mut view, PadButton::RightStickUp, true);
    open(&mut view);
    assert!(view.map_open());
    view
}

/// The chord's two buttons going up over a screen: Start first, or the
/// stick first.
fn chord_released_over(view: &mut Faked, start_first: bool) {
    let order = if start_first {
        [PadButton::Start, PadButton::RightStickUp]
    } else {
        [PadButton::RightStickUp, PadButton::Start]
    };
    for button in order {
        over(view, button, false);
    }
}

#[test]
fn the_course_map_follows_the_surroundings_open_or_not() {
    let mut view = faked_sol(in_sol());
    told(&mut view, PAD);
    assert_eq!(view.course_map().device(), PAD, "closed");
    view.open_map();
    told(&mut view, Device::Mouse);
    assert_eq!(view.course_map().device(), Device::Mouse, "open");
}

#[test]
fn a_bar_map_opens_told_the_device_flight_was_told() {
    let mut view = docked();
    told(&mut view, PAD);
    view.open_bar_map();
    assert_eq!(view.course_map().device(), PAD);
}

#[test]
fn a_dialog_map_over_flight_opens_told() {
    let mut view = faked_sol(in_sol());
    told(&mut view, PAD);
    view.open_dialog_map(MapOpener::Offer(MissionId(130)));
    assert_eq!(view.course_map().device(), PAD);
}

/// Flight's own map, opened by M and still held, told the pad.
fn m_held_map(frame: FlightFrame) -> Faked {
    let mut view = faked_sol(frame);
    told(&mut view, PAD);
    view.input(&key(MAP, true));
    assert!(view.map_open() && view.map_key_held);
    view
}

#[test]
fn with_m_held_the_dpad_moves_the_focus_and_closes_nothing() {
    let mut view = m_held_map(in_sol());
    for button in [
        PadButton::DPadLeft,
        PadButton::DPadUp,
        PadButton::DPadRight,
        PadButton::DPadDown,
    ] {
        pad(&mut view, button, true);
        pad(&mut view, button, false);
        assert!(!view.map_waiting(), "{button:?} closes nothing");
    }
    assert!(view.map_open());
}

#[test]
fn with_m_held_a_trigger_zooms_and_closes_nothing() {
    let mut view = m_held_map(in_sol());
    let zoom = view.course_map_zoom();
    pad(&mut view, PadButton::RightTrigger, true);
    pad(&mut view, PadButton::RightTrigger, false);
    assert!(!view.map_waiting());
    assert_ne!(view.course_map_zoom(), zoom, "zoomed");
}

#[test]
fn with_m_held_the_left_shoulder_closes_nothing_off_a_planet_and_closes_the_map_docked() {
    let mut view = m_held_map(in_sol());
    pad(&mut view, PadButton::LeftShoulder, true);
    assert!(!view.map_waiting(), "off a planet, the filter takes it");
    let mut view = m_held_map(FlightFrame {
        landed: Some(StellarId(128)),
        ..in_sol()
    });
    pad(&mut view, PadButton::LeftShoulder, true);
    assert!(view.map_waiting(), "docked, as Tab, it closes the map");
}

#[test]
fn over_flight_the_left_stick_pans_the_open_map() {
    let mut view = faked_sol(in_sol());
    told(&mut view, PAD);
    view.open_map();
    let place = |view: &Faked| {
        view.course_map()
            .place()
            .map(crate::galaxy::place::MapPlace::scroll)
    };
    let before = place(&view);
    pad(&mut view, PadButton::LeftStickRight, true);
    pad(&mut view, PadButton::LeftStickRight, false);
    let after = place(&view).expect("in flight");
    assert_eq!(
        after.0,
        before.expect("in flight").0 + 10,
        "an arrow's step"
    );
}

#[test]
fn over_flight_the_triggers_zoom_it() {
    let mut view = faked_sol(in_sol());
    told(&mut view, PAD);
    view.open_map();
    let zoom = view.course_map_zoom().expect("in flight");
    pad(&mut view, PadButton::LeftTrigger, true);
    pad(&mut view, PadButton::LeftTrigger, false);
    assert!(view.course_map_zoom().expect("in flight") > zoom, "out");
    pad(&mut view, PadButton::RightTrigger, true);
    pad(&mut view, PadButton::RightTrigger, false);
    pad(&mut view, PadButton::RightTrigger, true);
    pad(&mut view, PadButton::RightTrigger, false);
    assert!(view.course_map_zoom().expect("in flight") < zoom, "in");
}

#[test]
fn over_flight_south_on_done_closes_it() {
    let mut view = faked_sol(in_sol());
    told(&mut view, PAD);
    view.open_map();
    pad(&mut view, PadButton::South, true);
    assert!(view.map_open(), "on the release");
    pad(&mut view, PadButton::South, false);
    assert!(!view.map_open());
}

#[test]
fn a_bar_map_opened_on_the_chord_lets_go_of_the_map_key_whichever_button_goes_up_first() {
    for (name, open) in OPENERS {
        for start_first in [true, false] {
            let mut view = chord_opened_over(docked(), open);
            assert!(view.map_key_held, "{name}, start first: {start_first}");
            chord_released_over(&mut view, start_first);
            assert!(!view.map_key_held, "{name}, start first: {start_first}");
            // With the map key up, a press the map doesn't take closes
            // nothing.
            over(&mut view, PadButton::West, true);
            assert!(
                view.map_open() && !view.map_waiting(),
                "{name}, start first: {start_first}"
            );
        }
    }
}

#[test]
fn a_bar_map_closes_on_the_chord_and_waits_for_its_release_whichever_button_goes_up_first() {
    for (name, open) in OPENERS {
        for start_first in [true, false] {
            let mut view = chord_opened_over(docked(), open);
            chord_released_over(&mut view, !start_first);
            over(&mut view, PadButton::Start, true);
            over(&mut view, PadButton::RightStickUp, true);
            assert!(view.map_waiting(), "{name}, start first: {start_first}");
            chord_released_over(&mut view, start_first);
            assert!(!view.map_open(), "{name}, start first: {start_first}");
        }
    }
}

#[test]
fn a_dialog_map_over_flight_closes_on_the_chord_in_either_order() {
    let open: fn(&mut Faked) = |view| view.open_dialog_map(MapOpener::Offer(MissionId(130)));
    for start_first in [true, false] {
        let mut view = chord_opened_over(faked_sol(in_sol()), open);
        chord_released_over(&mut view, start_first);
        assert!(!view.map_key_held, "start first: {start_first}");
        over(&mut view, PadButton::Start, true);
        over(&mut view, PadButton::RightStickUp, true);
        assert!(view.map_waiting(), "start first: {start_first}");
        chord_released_over(&mut view, start_first);
        assert!(!view.map_open(), "start first: {start_first}");
    }
}

#[test]
fn done_on_a_chord_opened_bar_map_closes_once_the_chord_is_up() {
    for start_first in [true, false] {
        let mut view = docked();
        told(&mut view, PAD);
        let mut view = chord_opened_over(view, Faked::open_bar_map);
        tap_over(&mut view, PadButton::South);
        assert!(view.map_waiting(), "Done waits for the chord");
        chord_released_over(&mut view, start_first);
        assert!(!view.map_open(), "start first: {start_first}");
    }
    let mut view = docked();
    told(&mut view, PAD);
    let mut view = chord_opened_over(view, Faked::open_bar_map);
    chord_released_over(&mut view, true);
    tap_over(&mut view, PadButton::South);
    assert!(!view.map_open(), "with the chord up, Done closes at once");
}

#[test]
fn focus_lost_forgets_the_pads_map_key_over_the_port() {
    let mut view = chord_opened_over(docked(), Faked::open_bar_map);
    view.release_keys();
    assert!(view.map_open(), "nothing waited, so nothing closes");
    // M closes the map, waiting for M: the stick's release, its press
    // gone with the focus, isn't the map key's.
    view.port_map_input(&key(MAP, true));
    over(&mut view, PadButton::RightStickUp, false);
    assert!(view.map_waiting(), "still waiting for M");
    view.port_map_input(&key(MAP, false));
    assert!(!view.map_open());
}

#[test]
fn a_rebound_map_chord_closes_a_port_map_on_its_new_buttons() {
    let mut bindings = Bindings::default();
    bindings.gamepad.bind(
        Command::Play(PlayCommand::GalaxyMap),
        PadBinding::Entry(PadEntry::on(PadButton::Select, PadButton::North)),
    );
    let mut view = docked();
    view.set_bindings(SharedBindings::new(bindings));
    view.open_bar_map();
    view.port_map_input(&key(MAP, false));
    over(&mut view, PadButton::Start, true);
    over(&mut view, PadButton::RightStickUp, true);
    assert!(!view.map_waiting(), "the old chord is no map key");
    over(&mut view, PadButton::Start, false);
    over(&mut view, PadButton::RightStickUp, false);
    over(&mut view, PadButton::Select, true);
    over(&mut view, PadButton::North, true);
    assert!(view.map_waiting(), "the new chord closes it");
    over(&mut view, PadButton::Select, false);
    assert!(view.map_waiting(), "the layer's release is no map key's");
    over(&mut view, PadButton::North, false);
    assert!(!view.map_open(), "closed on North's release");
}

// The map's pad cursor and North's centre
// (rdm:phase/key-bindings/phase-20-gamepad-map-cursor).

/// A tenth of a second.
const TENTH: std::time::Duration = std::time::Duration::from_millis(100);

/// Barnard (132): neither Sol's preselection nor linked from Sol, so the
/// selection cycle, which steps through Sol's links, never reaches it.
const BARNARD: i16 = 132;

/// Where system `id` draws on `view`'s course map, as a whole pixel in
/// the map's frame.
fn pixel_on_map(view: &Faked, id: i16) -> Point {
    let map = view.course_map();
    let position = map.model().system(SystemId(id)).expect("drawn").position();
    let (h, v) = map.transform().pixel(position);
    #[allow(clippy::cast_precision_loss)] // Screen pixels.
    Point::new(h as f32, v as f32)
}

/// Pans `view`'s open map down with the left stick, by `feed`, until
/// Barnard draws in item 3; the steps taken.
fn pan_to_barnard(view: &mut Faked, feed: fn(&mut Faked, PadButton, bool)) -> usize {
    for steps in 0..100 {
        if crate::galaxy::dialog::in_map_item(pixel_on_map(view, BARNARD)) {
            return steps;
        }
        feed(view, PadButton::LeftStickDown, true);
        feed(view, PadButton::LeftStickDown, false);
    }
    panic!("Barnard never comes into item 3");
}

/// Steers `view`'s map cursor onto system `id` with the right stick, the
/// stick fed by `feed` and the map ticked by `tick`.
fn steer_onto(
    view: &mut Faked,
    id: i16,
    feed: fn(&mut Faked, &Input),
    tick: fn(&mut Faked, std::time::Duration),
) {
    use crate::input::PadAxis;
    let goal = pixel_on_map(view, id);
    for (axis, sign) in [(PadAxis::RightX, 1.0), (PadAxis::RightY, -1.0)] {
        for _ in 0..40 {
            let at = view.course_map().cursor().expect("in flight");
            let gap = if axis == PadAxis::RightX {
                goal.x - at.x
            } else {
                goal.y - at.y
            };
            if gap.abs() < 1.0 {
                break;
            }
            // A full push moves 30 points a tenth; a push p past the dead
            // zone reads (p - 0.15) / 0.85.
            let wanted = (gap.abs() / 30.0).min(1.0);
            let value = (0.15 + wanted * 0.85) * gap.signum() * sign;
            feed(view, &Input::PadAxis { axis, value });
            tick(view, TENTH);
            feed(view, &Input::PadAxis { axis, value: 0.0 });
        }
    }
    let at = view.course_map().cursor().expect("in flight");
    assert!(
        (at.x - goal.x).abs() < 2.0 && (at.y - goal.y).abs() < 2.0,
        "{at:?} near {goal:?}"
    );
}

fn clicked(selected: i16) -> Answer {
    Answer::Clicked(nova_sim_frame::map_click::MapClicked {
        selected: Some(SystemId(selected)),
        gate_pick: None,
        clear_route: None,
        course_changed: false,
        nav_redraw: false,
    })
}

/// The map clicks `view` sent its flight.
fn map_clicks(view: &Faked) -> Vec<FlightCommand> {
    sent(view)
        .into_iter()
        .filter(|command| matches!(command, FlightCommand::MapClick(_)))
        .collect()
}

#[test]
fn over_flight_the_cursor_reaches_a_system_off_the_cycle_and_south_clicks_it() {
    let open = || {
        let mut view = faked_sol(in_sol());
        told(&mut view, PAD);
        view.open_map();
        view
    };
    let mut mouse = open();
    let steps = pan_to_barnard(&mut mouse, pad);
    let at = pixel_on_map(&mouse, BARNARD);
    mouse.sim.queue_answer(clicked(BARNARD));
    faked_map_click(&mut mouse, at);
    let mut view = open();
    assert_eq!(pan_to_barnard(&mut view, pad), steps);
    assert_ne!(view.course_map().selected(), Some(SystemId(BARNARD)));
    steer_onto(
        &mut view,
        BARNARD,
        |view, input| {
            view.input(input);
        },
        Faked::tick,
    );
    view.sim.queue_answer(clicked(BARNARD));
    pad(&mut view, PadButton::South, true);
    pad(&mut view, PadButton::South, false);
    assert!(view.map_open(), "South pressed the map, not Done");
    assert_eq!(map_clicks(&view), map_clicks(&mouse));
    assert_eq!(map_clicks(&view).len(), 1);
    assert_eq!(view.course_map().selected(), Some(SystemId(BARNARD)));
}

#[test]
fn over_the_port_the_cursor_moves_and_clicks() {
    let mut view = docked();
    told(&mut view, PAD);
    view.open_bar_map();
    view.port_map_input(&key(MAP, false));
    pan_to_barnard(&mut view, over);
    steer_onto(&mut view, BARNARD, Faked::port_map_input, Faked::tick);
    view.sim.queue_answer(clicked(BARNARD));
    tap_over(&mut view, PadButton::South);
    assert!(view.map_open());
    assert_eq!(map_clicks(&view).len(), 1);
    assert_eq!(view.course_map().selected(), Some(SystemId(BARNARD)));
}

#[test]
fn over_flight_north_centres_the_open_map() {
    let mut view = faked_sol(in_sol());
    told(&mut view, PAD);
    view.open_map();
    view.map.centre_on_system(SystemId(131));
    let moved = view.course_map().place().copied();
    pad(&mut view, PadButton::North, true);
    pad(&mut view, PadButton::North, false);
    assert!(view.map_open());
    assert_ne!(view.course_map().place().copied(), moved, "centred");
    assert_eq!(view.course_map().selected(), Some(SystemId(130)));
    assert!(
        view.ui_sounds.contains(&crate::sound::UiSound::MapCentred),
        "{:?}",
        view.ui_sounds
    );
}

#[test]
fn a_stick_pushed_over_flight_with_the_map_shut_moves_no_cursor_once_it_opens() {
    use crate::input::PadAxis;
    let mut view = faked_sol(in_sol());
    told(&mut view, PAD);
    view.input(&Input::PadAxis {
        axis: PadAxis::RightX,
        value: 1.0,
    });
    view.open_map();
    let centre = view.course_map().cursor();
    view.tick(TENTH);
    assert_eq!(
        view.course_map().cursor(),
        centre,
        "the shut map read no stick"
    );
}
