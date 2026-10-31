//! The player info's and the pilot panel's component tests on
//! [`FakeFlight`] (rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks):
//! the commands the player info key and the screen's opening send, what
//! the view holds for the router and how it marks the screen. Whether the
//! session takes the press, what it opens and what each screen says are
//! nova-sim's (`session::player_info`, `session::pilot_panel`,
//! `session::apply::player_info`).

use super::faked_travel::sent_but_lines;
use super::*;
use nova_sim::PlayerInfo;
use nova_sim::flight_frame::FlightEvent;
use nova_sim::pilot_panel::PilotPanel;
use nova_sim::player_info::{CheatIds, Words};

/// A player info screen for Ada, flying the Bitsy, cheats off.
fn screen() -> PlayerInfo {
    PlayerInfo {
        pilot_name: "Ada".to_owned(),
        date: "Jun 23, 1177".to_owned(),
        system: "Sol".to_owned(),
        legal_status: "Citizen".to_owned(),
        combat_rating: "Average".to_owned(),
        ship_name: "Bitsy".to_owned(),
        ship_class: "Shuttle".to_owned(),
        turn_rate: 90,
        accel: 450,
        max_speed: 600,
        credits: 12_345,
        shield: Some(100),
        armor: None,
        energy: None,
        income: None,
        fleet: false,
        cargo: None,
        extras: None,
        honors: None,
        jettisonable: false,
        words: Words::STOCK,
        cheat_ids: CheatIds::default(),
        cheats: false,
    }
}

/// A view over a flight in Sol with the cheats setting `cheats`.
fn faked_cheats(cheats: bool) -> Faked {
    faked(in_sol()).with_overlays(FlightOverlays {
        cheats,
        ..FlightOverlays::default()
    })
}

#[test]
fn p_asks_for_the_player_info_once_letting_go_of_the_flight_keys() {
    let mut view = faked(in_sol());
    assert!(!view.take_player_info_request());
    view.held.insert(Key::Up);
    view.sim.queue_answer(Answer::PlayerInfoPressed(true));
    let _ = view.ask_player_info();
    assert_eq!(sent_but_lines(&view), [FlightCommand::PressPlayerInfo]);
    assert!(view.take_player_info_request());
    assert!(!view.take_player_info_request(), "once");
    assert!(view.held.is_empty(), "the flight keys let go");
    assert_eq!(view.message(), None, "nothing said");
    assert_eq!(view.drained_sounds(), [], "silent");
}

#[test]
fn a_press_the_flight_refuses_asks_nothing_and_beeps_through_the_drain() {
    let mut view = faked(in_sol());
    view.held.insert(Key::Up);
    view.sim
        .queue_event(FlightEvent::Sound(nova_sim::SimSound::Refused));
    view.sim.queue_answer(Answer::PlayerInfoPressed(false));
    let _ = view.ask_player_info();
    assert_eq!(sent_but_lines(&view), [FlightCommand::PressPlayerInfo]);
    assert!(!view.take_player_info_request());
    assert!(!view.held.is_empty(), "the flight keys held");
    assert_eq!(
        view.drained_sounds(),
        [Sound::Sim(nova_sim::SimSound::Refused)],
        "the flight's beep"
    );
}

#[test]
fn in_a_flight_that_never_started_p_asks_the_router_which_opens_nothing() {
    let mut view = faked(in_sol());
    let _ = view.ask_player_info();
    assert_eq!(sent_but_lines(&view), [FlightCommand::PressPlayerInfo]);
    assert!(
        view.take_player_info_request(),
        "a refused press is taken, as a failed session's always was"
    );
    assert_eq!(view.open_player_info(), None, "and nothing opens");
}

#[test]
fn p_in_a_jump_presses_for_the_flights_refusal_only_when_its_gate_is_taken() {
    let mut view = faked(in_sol());
    view.change_sim(|sim| sim.refuse(PlayerCommand::PlayerInfo));
    view.refuse_player_info();
    assert_eq!(sent_but_lines(&view), [], "its fate refuses it: nothing");
    view.change_sim(|sim| sim.take(PlayerCommand::PlayerInfo));
    view.sim.queue_answer(Answer::PlayerInfoPressed(false));
    view.refuse_player_info();
    assert_eq!(sent_but_lines(&view), [FlightCommand::PressPlayerInfo]);
    assert!(!view.take_player_info_request(), "refused in the jump");
}

#[test]
fn opening_the_player_info_gives_the_flights_screen_or_none() {
    let mut view = faked(in_sol());
    view.sim
        .queue_answer(Answer::PlayerInfo(Some(Box::new(screen()))));
    assert_eq!(view.open_player_info(), Some(screen()));
    assert_eq!(sent_but_lines(&view), [FlightCommand::OpenPlayerInfo]);
    view.sim.queue_answer(Answer::PlayerInfo(None));
    assert_eq!(view.open_player_info(), None, "nothing opens in a jump");
}

#[test]
fn open_player_info_carries_the_cheats_setting() {
    for cheats in [false, true] {
        let mut view = faked_cheats(cheats);
        view.sim
            .queue_answer(Answer::PlayerInfo(Some(Box::new(screen()))));
        let opened = view.open_player_info().expect("opens");
        assert_eq!(opened.cheats, cheats, "opened");
    }
}

/// Ada's pilot panel, flying a shuttle named Bitsy.
fn panel() -> PilotPanel {
    PilotPanel {
        pilot_name: "Ada".to_owned(),
        ship_name: "Bitsy".to_owned(),
        ship_class: "Shuttle".to_owned(),
        ship: ShipId(128),
        legal_status: "Citizen".to_owned(),
        combat_rating: "Average".to_owned(),
        date: "Jun 23, 1177".to_owned(),
    }
}

#[test]
fn the_pilot_panel_is_the_flights() {
    let mut view = faked_cheats(true);
    assert_eq!(view.pilot_panel(), None, "nothing answered");
    view.change_sim(|_| {});
    view.sim
        .queue_screen(ScreenAnswer::PilotPanel(Box::new(panel())));
    assert_eq!(view.pilot_panel(), Some(panel()));
    assert_eq!(sent_but_lines(&view), [], "reading it sends nothing");
}

#[test]
fn the_menus_pilot_panel_is_asked_once_until_the_flight_changes() {
    let mut view = faked(in_sol());
    view.sim
        .queue_screen(ScreenAnswer::PilotPanel(Box::new(panel())));
    for _ in 0..3 {
        assert_eq!(view.pilot_panel(), Some(panel()));
    }
    assert_eq!(view.sim.screens_asked(), [ScreenAsk::PilotPanel], "once");
    view.send(FlightCommand::CueCourse);
    assert_eq!(view.pilot_panel(), None, "asked afresh: nothing answered");
    for _ in 0..3 {
        assert_eq!(view.pilot_panel(), None, "nothing kept, not asked again");
    }
    assert_eq!(
        view.sim.screens_asked(),
        [ScreenAsk::PilotPanel, ScreenAsk::PilotPanel],
        "once more after the command, the answer of nothing kept too"
    );
    view.tick(TICK);
    view.sim
        .queue_screen(ScreenAnswer::PilotPanel(Box::new(panel())));
    assert_eq!(view.pilot_panel(), Some(panel()), "after the frame's steps");
    assert_eq!(view.sim.screens_asked().len(), 3);
}
