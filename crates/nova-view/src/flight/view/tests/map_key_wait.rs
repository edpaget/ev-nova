use super::faked_screen::{sent_count, sent_since};
use super::*;
use crate::galaxy::buttons::{MapButton, MapButtons};
use crate::galaxy::dialog::DIALOG_ORIGIN;

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
