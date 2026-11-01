use super::faked_screen::{sent_count, sent_since};
use super::*;
use crate::flight::view::dialog_map::MapOpening;
use crate::galaxy::buttons::{MapButton, MapButtons};
use crate::galaxy::dialog::DIALOG_ORIGIN;
use nova_sim::Face;
use nova_sim::flight_frame::StepInput;

// The map key's wait on close in flight (`_DoSystemMap`
// @0x13a2e-0x13a46): every close waits, inert, while M is held, and the
// whole close lands on M's release.

/// A view over a scripted flight in Sol with its course map opened by
/// M, M let go, and its interface sounds taken.
fn faked_m_open() -> Faked {
    let mut view = faked_sol(in_sol());
    view.input(&key(MAP, true));
    view.input(&key(MAP, false));
    assert!(view.map_open);
    ui_sounds(&mut view);
    view
}

/// Where a click lands on the open map's Done button.
fn done_at(view: &Faked) -> Point {
    let at = MapButtons::new(DIALOG_ORIGIN)
        .rect(MapButton::Done)
        .center();
    Point::new(at.x + view.map_offset().x, at.y)
}

/// Clicks the left button at `at` on `view`.
fn click_at(view: &mut Faked, at: Point) {
    for pressed in [true, false] {
        view.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
}

/// Whether `view` sent the course's cue since `from`.
fn cued_since(view: &Faked, from: usize) -> bool {
    sent_since(view, from).contains(&FlightCommand::CueCourse)
}

#[test]
fn m_pressed_on_the_map_waits_drawn_and_silent_until_it_is_let_go() {
    let mut view = faked_m_open();
    let stale = NavDisplay::Stellar(Some("Stale".to_owned()));
    view.nav_scene = Some((view.frame.nav_mode, stale.clone()));
    let from = sent_count(&view);
    view.input(&key(MAP, true));
    view.input(&held(MAP));
    assert!(view.map_open(), "the dialog stays up");
    let mut map = DrawList::new();
    view.course_map().draw(&mut map);
    let list = drawn(&view);
    let over = list.len() - map.len();
    assert_eq!(
        list.iter().skip(over).collect::<Vec<_>>(),
        map.iter().collect::<Vec<_>>(),
        "still drawn"
    );
    assert_eq!(map_sounds(&mut view), [], "no beep yet");
    assert!(!cued_since(&view, from), "no cue yet");
    assert_eq!(
        view.nav_scene.as_ref().map(|(_, nav)| nav),
        Some(&stale),
        "the nav area not redrawn"
    );
    view.input(&key(MAP, false));
    assert!(!view.map_open(), "closed on the release");
    assert_eq!(map_sounds(&mut view), [UiSound::MapClosed]);
    assert!(cued_since(&view, from));
    assert_ne!(view.nav_scene.as_ref().map(|(_, nav)| nav), Some(&stale));
}

#[test]
fn while_waiting_keys_and_clicks_change_nothing() {
    let mut view = faked_m_open();
    view.input(&key(MAP, true));
    let selected = view.map.selected();
    let from = sent_count(&view);
    for k in [Key::Tab, Key::Enter, Key::Char('c'), Key::Space, LAND] {
        tap(&mut view, k);
    }
    view.input(&held(Key::Tab));
    let done = done_at(&view);
    click_at(&mut view, done);
    click_at(&mut view, Point::new(500.0, 400.0));
    assert!(view.map_open(), "still waiting");
    assert_eq!(sent_since(&view, from), [], "no command");
    assert_eq!(ui_sounds(&mut view), [], "no sound");
    assert_eq!(view.map.selected(), selected, "the selection unchanged");
    view.input(&key(MAP, false));
    assert!(!view.map_open());
}

#[test]
fn letting_go_of_every_key_while_waiting_closes_the_map() {
    let mut view = faked_m_open();
    view.input(&key(MAP, true));
    let from = sent_count(&view);
    view.release_keys();
    assert!(!view.map_open(), "focus lost counts as the release");
    assert_eq!(map_sounds(&mut view), [UiSound::MapClosed]);
    assert!(cued_since(&view, from));
}

#[test]
fn with_m_up_return_enter_done_and_the_routers_close_close_at_once() {
    let closes: [fn(&mut Faked); 3] = [
        // Return and Enter are one key here.
        |view| {
            view.input(&key(Key::Enter, true));
        },
        |view| {
            let done = done_at(view);
            click_at(view, done);
        },
        Faked::close_map,
    ];
    for (n, close) in closes.into_iter().enumerate() {
        let mut view = faked_m_open();
        close(&mut view);
        assert!(!view.map_open(), "close {n}");
        assert_eq!(map_sounds(&mut view), [UiSound::MapClosed], "close {n}");
    }
}

#[test]
fn with_m_held_return_enter_done_and_the_routers_close_wait_for_its_release() {
    let closes: [fn(&mut Faked); 3] = [
        // Return and Enter are one key here.
        |view| {
            view.input(&key(Key::Enter, true));
        },
        |view| {
            let done = done_at(view);
            click_at(view, done);
        },
        Faked::close_map,
    ];
    for (n, close) in closes.into_iter().enumerate() {
        let mut view = faked_sol(in_sol());
        view.input(&key(MAP, true));
        ui_sounds(&mut view);
        close(&mut view);
        assert!(view.map_open(), "close {n} waits");
        assert_eq!(map_sounds(&mut view), [], "close {n}");
        view.input(&key(MAP, false));
        assert!(!view.map_open(), "close {n} on the release");
        assert_eq!(map_sounds(&mut view), [UiSound::MapClosed], "close {n}");
    }
}

#[test]
fn a_key_down_the_filter_doesnt_take_with_m_held_waits_for_its_release() {
    let mut view = faked_sol(in_sol());
    view.input(&key(MAP, true));
    tap(&mut view, LAND);
    assert!(view.map_open(), "waits");
    view.input(&key(MAP, false));
    assert!(!view.map_open());
}

#[test]
fn the_hypergate_map_closed_with_m_held_enters_the_gate_on_the_release() {
    let mut view = faked(in_sol());
    let opening = map_opening(&view.frame, Some(130));
    view.sim.queue_screen(opening);
    view.map.offer_gates(vec![SystemId(132), SystemId(133)]);
    view.open_map();
    assert_eq!(view.map.mode(), MapMode::Hypergate);
    let from = sent_count(&view);
    view.input(&key(MAP, true));
    assert!(view.map_open(), "waits");
    let entered = |view: &Faked| {
        sent_since(view, from)
            .iter()
            .any(|command| matches!(command, FlightCommand::EnterHypergate(_)))
    };
    assert!(!entered(&view), "not yet");
    view.input(&key(MAP, false));
    assert!(!view.map_open());
    assert!(entered(&view), "on the release");
}

// What flight reads live as the map closes (`_QuickKeyCheck` @0xa0f5 on
// the snapshot `_DoPlayGameWork` takes): see the `map_wait` module.

/// What the next step read of the keys held on `view`.
fn next_step(view: &mut Faked) -> StepInput {
    view.tick(TICK);
    *view.sim.inputs().last().expect("a step")
}

/// A key and what a step reads it down by.
type KeyRead = (Key, fn(&StepInput) -> bool);

/// Each unlatched key's read in a step, as flight acts on it at once.
fn unlatched_reads() -> [KeyRead; 8] {
    [
        (Key::Up, |step| step.controls.thrust),
        (Key::Left, |step| step.controls.turn == Turn::Left),
        (Key::Right, |step| step.controls.turn == Turn::Right),
        (Key::Down, |step| step.controls.reverse),
        (FACE_TARGET_KEY, |step| step.controls.face == Face::Target),
        (AFTERBURNER_KEY, |step| step.controls.afterburner),
        (FIRE_KEY, |step| step.fire.0),
        (SECONDARY_KEY, |step| step.fire.1),
    ]
}

#[test]
fn a_flight_key_pressed_during_the_wait_and_still_down_acts_after_the_close() {
    for (k, read) in unlatched_reads() {
        let mut view = faked_m_open();
        view.input(&key(MAP, true));
        view.input(&key(k, true));
        view.input(&key(MAP, false));
        assert!(!view.map_open());
        assert!(read(&next_step(&mut view)), "{k:?} reads down");
    }
}

#[test]
fn option_command_and_shift_pressed_during_the_wait_stay_down_after_the_close() {
    for k in [Key::Alt, Key::Command, Key::Shift] {
        let mut view = faked_m_open();
        view.input(&key(MAP, true));
        view.input(&key(k, true));
        view.input(&key(MAP, false));
        assert!(view.held.contains(&k), "{k:?} reads down");
    }
    let mut view = faked_m_open();
    view.input(&key(MAP, true));
    view.input(&key(Key::Alt, true));
    view.input(&key(FACE_TARGET_KEY, true));
    view.input(&key(MAP, false));
    assert_eq!(next_step(&mut view).controls.face, Face::Stellar);
}

#[test]
fn a_flight_key_tapped_during_the_wait_does_nothing_after_the_close() {
    for (k, read) in unlatched_reads() {
        let mut view = faked_m_open();
        view.input(&key(MAP, true));
        tap(&mut view, k);
        view.input(&key(MAP, false));
        assert!(!read(&next_step(&mut view)), "{k:?} reads up");
    }
}

#[test]
fn a_flight_key_down_since_before_the_wait_reads_down_after_the_close() {
    for (k, read) in unlatched_reads() {
        // Pressed with the map open, before any wait.
        let mut view = faked_m_open();
        view.input(&key(k, true));
        tap(&mut view, MAP);
        assert!(read(&next_step(&mut view)), "{k:?} pressed on the map");

        // Held from before the map opened.
        let mut view = faked_sol(in_sol());
        view.input(&key(k, true));
        tap(&mut view, MAP);
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert!(read(&next_step(&mut view)), "{k:?} held across the map");
    }
}

#[test]
fn a_flight_key_let_go_on_the_map_reads_up_after_the_close() {
    for (k, read) in unlatched_reads() {
        let mut view = faked_sol(in_sol());
        view.input(&key(k, true));
        tap(&mut view, MAP);
        view.input(&key(k, false));
        tap(&mut view, MAP);
        assert!(!read(&next_step(&mut view)), "{k:?} let go");
    }
}

#[test]
fn a_latched_key_held_past_the_close_waits_for_its_release() {
    // `_HandlePlayer` sets every latch after M's map (@0x6a898).
    for k in [JUMP_KEY, BOARD_KEY, LAND_KEY, TARGET_KEY] {
        let mut view = faked_m_open();
        view.input(&key(MAP, true));
        view.input(&key(k, true));
        let from = sent_count(&view);
        view.input(&key(MAP, false));
        view.tick(TICK);
        assert_eq!(
            sent_since(&view, from)
                .into_iter()
                .filter(|command| *command != FlightCommand::CueCourse)
                .collect::<Vec<_>>(),
            [],
            "{k:?} sends nothing"
        );
        assert!(!view.held.contains(&k), "{k:?} not held");
    }
}

#[test]
fn a_latched_key_held_as_the_map_opens_is_let_go() {
    let mut view = faked_sol(in_sol());
    view.input(&key(JUMP_KEY, true));
    view.input(&key(MAP, true));
    assert!(!view.held.contains(&JUMP_KEY));
}

#[test]
fn a_click_on_done_during_the_wait_is_dropped_by_the_close() {
    let mut view = faked_m_open();
    view.input(&key(MAP, true));
    let selected = view.map.selected();
    let done = done_at(&view);
    click_at(&mut view, done);
    let from = sent_count(&view);
    view.input(&key(MAP, false));
    assert_eq!(sent_since(&view, from), [FlightCommand::CueCourse]);
    assert_eq!(view.map.selected(), selected);
}

#[test]
fn every_other_map_clears_the_flight_keys_as_it_opens() {
    for opening in [
        MapOpening::Bar,
        MapOpening::Port(crate::flight::PortScreen::Exchange),
    ] {
        let mut view = faked_sol(in_sol());
        view.input(&key(Key::Up, true));
        view.open_map_as(opening);
        assert!(view.held.is_empty(), "{opening:?}");
    }
}

#[test]
fn a_flight_key_lost_with_the_focus_on_the_map_reads_up_after_the_close() {
    let mut view = faked_m_open();
    view.input(&key(Key::Up, true));
    view.release_keys();
    assert!(view.map_open(), "the map stays");
    tap(&mut view, MAP);
    assert!(!next_step(&mut view).controls.thrust);
}

// What a map over a screen leaves for the screen under it
// (`FlightView::take_map_replay`).

/// [`Key::Enter`]'s press and release.
fn enter_tap() -> [Input; 2] {
    [key(Key::Enter, true), key(Key::Enter, false)]
}

/// A view docked at Earth with `opening`'s map open and M let go.
fn docked_map(opening: MapOpening) -> Faked {
    let mut view = faked_sol(FlightFrame {
        landed: Some(StellarId(128)),
        ..in_sol()
    });
    view.open_map_as(opening);
    view.map_key_held = true;
    view.port_map_input(&key(MAP, false));
    view
}

#[test]
fn a_port_screens_map_gives_the_keys_of_its_wait_at_the_close_in_order_once() {
    let mut view = docked_map(MapOpening::Port(crate::flight::PortScreen::Exchange));
    view.port_map_input(&key(MAP, true));
    view.port_map_input(&key(Key::Down, true));
    view.port_map_input(&key(Key::Down, false));
    for input in enter_tap() {
        view.port_map_input(&input);
    }
    assert_eq!(view.take_map_replay(), [], "nothing before the close");
    view.port_map_input(&key(MAP, false));
    assert!(!view.map_open());
    assert_eq!(
        view.take_map_replay(),
        [
            key(Key::Down, true),
            key(Key::Down, false),
            key(Key::Enter, true),
            key(Key::Enter, false)
        ]
    );
    assert_eq!(view.take_map_replay(), [], "given once");
}

#[test]
fn a_port_screens_map_keeps_only_the_keys_of_its_wait() {
    let mut view = docked_map(MapOpening::Bar);
    view.port_map_input(&key(Key::Up, true));
    view.port_map_input(&key(Key::Up, false));
    view.port_map_input(&key(MAP, true));
    view.port_map_input(&key(MAP, false));
    assert_eq!(view.take_map_replay(), []);
}

#[test]
fn m_on_the_map_leaves_nothing_when_nothing_was_typed() {
    let mut view = docked_map(MapOpening::Bar);
    view.port_map_input(&key(MAP, true));
    view.port_map_input(&key(MAP, false));
    assert!(!view.map_open());
    assert_eq!(view.take_map_replay(), []);
}

#[test]
fn a_map_in_flight_leaves_nothing_nothing_reads_the_queue() {
    let mut view = faked_m_open();
    view.input(&key(MAP, true));
    tap(&mut view, Key::Enter);
    view.input(&key(MAP, false));
    assert!(!view.map_open());
    assert_eq!(view.take_map_replay(), []);
}

#[test]
fn letting_go_of_every_key_during_the_wait_drops_what_it_kept() {
    let mut view = docked_map(MapOpening::Bar);
    view.port_map_input(&key(MAP, true));
    view.port_map_input(&key(Key::Enter, true));
    view.release_keys();
    assert!(!view.map_open());
    assert_eq!(view.take_map_replay(), []);
    // Nothing of it outlives the close either.
    view.open_map_as(MapOpening::Bar);
    view.map_key_held = true;
    view.port_map_input(&key(MAP, true));
    view.port_map_input(&key(MAP, false));
    assert_eq!(view.take_map_replay(), []);
}

#[test]
fn a_click_during_the_wait_is_left_out_of_what_the_close_gives() {
    let mut view = docked_map(MapOpening::Bar);
    view.port_map_input(&key(MAP, true));
    let done = done_at(&view);
    for pressed in [true, false] {
        view.port_map_input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        });
    }
    view.port_map_input(&key(MAP, false));
    assert_eq!(view.take_map_replay(), []);
}

#[test]
fn the_wait_is_read_from_its_start_not_from_ms_press() {
    // A port map opens with M held, and isn't waiting yet.
    let mut view = faked_sol(in_sol());
    view.open_dialog_map(MapOpener::Offer(nova_sim::MissionId(130)));
    assert!(!view.map_waiting(), "M held, no close asked");
    view.port_map_input(&key(MAP, true));
    assert!(view.map_waiting());
    view.port_map_input(&key(MAP, false));
    assert!(!view.map_waiting());
}

#[test]
fn a_live_key_handed_over_is_held_only_if_flight_reads_it_with_no_latch() {
    let mut view = faked_sol(in_sol());
    view.hold_live_key(Key::Up);
    view.hold_live_key(JUMP_KEY);
    assert!(view.held.contains(&Key::Up));
    assert!(!view.held.contains(&JUMP_KEY));
}
