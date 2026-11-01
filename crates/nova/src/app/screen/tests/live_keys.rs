use super::encounters::{boardable, boarding_data, hailable};
use super::map_wait_keys::{game_data_fork_loaded, job, offered_over_flight};
use super::*;
use nova_view::ui::player_info::PLAYER_INFO_KEY;

// The keys and the button down under a dialog over flight, handed back as
// the last one closes (the `live_input` module): `_HandlePlayer` polls its
// unlatched keys past `_SetAllKeyCmdFlags` (@0x61e6d), which latches the
// rest. Wiring through the real router and flight: which keys flight
// reads live, and how, is nova-view's (`flight::view::tests::live_keys`).

/// Presses and lets go of `k`.
fn tap(screen: &mut AppScreen, k: Key) {
    screen.input(&key(k, true));
    screen.input(&key(k, false));
}

/// How fast the player's ship goes.
fn speed(screen: &AppScreen) -> f32 {
    ship(screen).velocity.length()
}

/// Flight in Sol with Up held, then a person's offer opened over it as
/// its hail opens it: flight lets go of its keys first, as their releases
/// go to the offer.
fn offered_thrusting() -> AppScreen {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(game_data_fork_loaded())
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    screen.input(&key(Key::Up, true));
    offer_as_a_hail_would(&mut screen);
    screen
}

/// Opens a person's offer over flight as its hail does: flight lets go
/// of its keys and the button (`FlightView::hold_offer`), then the router
/// opens it; a tick over it follows, as the original's next frame does.
fn offer_as_a_hail_would(screen: &mut AppScreen) {
    flight_mut(screen).cancel_pointer();
    flight_mut(screen).release_keys();
    screen.open_offer(job());
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::Offer);
}

/// Refuses the offer open over flight, which closes back to flight.
fn refuse(screen: &mut AppScreen) {
    tap(screen, Key::Char('n'));
    assert_eq!(screen.showing(), Showing::Flight, "closed");
}

#[test]
fn thrust_held_through_a_persons_offer_thrusts_after_its_close() {
    let mut screen = offered_thrusting();
    refuse(&mut screen);
    let closed = speed(&screen);
    screen.tick(TICK * 3);
    assert!(speed(&screen) > closed, "thrust: {:?}", ship(&screen));
}

#[test]
fn thrust_pressed_during_the_offer_and_still_down_at_its_close_thrusts() {
    let mut screen = offered_over_flight();
    let still = ship(&screen);
    screen.input(&key(Key::Up, true));
    refuse(&mut screen);
    screen.tick(TICK * 3);
    assert_ne!(ship(&screen), still, "thrust");
}

#[test]
fn thrust_let_go_during_the_offer_doesnt() {
    let mut screen = offered_thrusting();
    screen.input(&key(Key::Up, false));
    refuse(&mut screen);
    let closed = speed(&screen);
    screen.tick(TICK * 3);
    assert!(speed(&screen) <= closed, "no thrust: {:?}", ship(&screen));
}

#[test]
fn a_thrust_key_let_go_after_the_close_stops_thrusting() {
    let mut screen = offered_thrusting();
    refuse(&mut screen);
    screen.tick(TICK);
    screen.input(&key(Key::Up, false));
    let let_go = speed(&screen);
    screen.tick(TICK * 3);
    assert!(speed(&screen) <= let_go, "its release came live");
}

#[test]
fn a_focus_loss_under_a_dialog_forgets_the_keys_down() {
    // Ours can't read the keys unfocused (`Screen::release_keys`).
    let mut screen = offered_thrusting();
    screen.cancel_pointer();
    screen.release_keys();
    screen.focus_lost();
    screen.focus_gained();
    screen.tick(TICK);
    refuse(&mut screen);
    let closed = speed(&screen);
    screen.tick(TICK * 3);
    assert!(speed(&screen) <= closed, "forgotten: {:?}", ship(&screen));
}

#[test]
fn an_internal_let_go_under_a_dialog_keeps_the_keys_down() {
    // P over the offer opens the player info, the router letting go of
    // what is below it, whose releases go to the screen.
    let mut screen = offered_thrusting();
    tap(&mut screen, PLAYER_INFO_KEY);
    assert_eq!(screen.showing(), Showing::PlayerInfo, "over the offer");
    tap(&mut screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Offer, "back to the offer");
    refuse(&mut screen);
    let closed = speed(&screen);
    screen.tick(TICK * 3);
    assert!(speed(&screen) > closed, "thrust: {:?}", ship(&screen));
}

#[test]
fn a_replayed_press_doesnt_bring_back_a_key_let_go() {
    // The map's wait over the offer queues Up's tap and N, which the
    // router replays to the offer at the wait's end, after Up's live
    // release.
    let mut screen = offered_thrusting();
    screen.input(&key(MAP, true));
    tap(&mut screen, Key::Escape);
    assert!(flight(&screen).map_waiting(), "waiting");
    screen.input(&key(Key::Up, false));
    tap(&mut screen, Key::Up);
    tap(&mut screen, Key::Char('n'));
    screen.input(&key(MAP, false));
    assert_eq!(screen.showing(), Showing::Flight, "refused");
    let closed = speed(&screen);
    screen.tick(TICK * 3);
    assert!(speed(&screen) <= closed, "no thrust: {:?}", ship(&screen));
}

#[test]
fn a_click_held_past_the_offers_close_keeps_flights_cursor_whole() {
    for held in [true, false] {
        let mut screen = offered_over_flight();
        let at = Point::new(300.0, 300.0);
        screen.input(&Input::PointerMoved(at));
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        });
        if !held {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed: false,
                at,
            });
        }
        refuse(&mut screen);
        // The app tells flight where the pointer is as it shows again.
        screen.input(&Input::PointerMoved(at));
        for _ in 0..90 {
            screen.tick(TICK);
        }
        assert_eq!(
            flight(&screen).cursor_at().is_some(),
            held,
            "whole while held, faded once let go: held {held}"
        );
    }
}

#[test]
fn thrust_held_through_the_comm_dialog_thrusts_after_its_close() {
    // `_HandlePlayerCommunication` @0x625ee.
    let mut screen = hailable(with_dialogs(boarding_data()));
    fly(&mut screen);
    screen.input(&key(Key::Up, true));
    hail(&mut screen);
    assert_eq!(screen.showing(), Showing::Comm);
    tap(&mut screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight, "the channel closed");
    let closed = speed(&screen);
    screen.tick(TICK * 3);
    assert!(speed(&screen) > closed, "thrust: {:?}", ship(&screen));
}

#[test]
fn thrust_held_through_the_plunder_dialog_thrusts_after_its_close() {
    // `_HandlePlayerBoardAttempt` @0x659f1.
    let mut screen = boardable(with_dialogs(boarding_data()));
    fly(&mut screen);
    screen.tick(TICK);
    tap(&mut screen, Key::Tab);
    screen.input(&key(Key::Up, true));
    tap(&mut screen, Key::Char('b'));
    assert_eq!(screen.showing(), Showing::Plunder);
    tap(&mut screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight, "aborted");
    let closed = speed(&screen);
    screen.tick(TICK * 3);
    assert!(speed(&screen) > closed, "thrust: {:?}", ship(&screen));
}

#[test]
fn a_latched_key_held_across_the_offers_close_acts_only_once_pressed_again() {
    // The stellar slot key 1 selects on a frame it is held with its
    // latch clear, and `_SetAllKeyCmdFlags` (@0x622ac) sets the latch at
    // the close.
    let mut screen = offered_over_flight();
    let target = |screen: &AppScreen| flight(screen).session().expect("flying").nav_target();
    assert_eq!(target(&screen), None);
    screen.input(&key(Key::Char('1'), true));
    refuse(&mut screen);
    screen.tick(TICK * 8);
    assert_eq!(target(&screen), None, "held across the close: latched");
    screen.input(&key(Key::Char('1'), false));
    screen.input(&key(Key::Char('1'), true));
    screen.tick(TICK * 8);
    assert_ne!(target(&screen), None, "pressed afresh");
}

#[test]
fn a_latched_key_held_across_the_offers_close_ignores_its_repeats_until_let_go() {
    // `_HandlePlayer` polls `GetKeys` (`_QuickKeyCheckHandler` @0x43e01)
    // and drops queued keys (`_IterateInGameEventLoop` @0x56a6b), so an
    // OS repeat never clears the latch `_SetAllKeyCmdFlags` (@0x622ac)
    // set.
    let mut screen = offered_over_flight();
    let target = |screen: &AppScreen| flight(screen).session().expect("flying").nav_target();
    screen.input(&key(Key::Char('1'), true));
    refuse(&mut screen);
    for _ in 0..4 {
        screen.tick(TICK * 2);
        screen.input(&Input::Key {
            key: Key::Char('1'),
            pressed: true,
            repeat: true,
        });
    }
    screen.tick(TICK * 8);
    assert_eq!(target(&screen), None, "repeats: still latched");
    screen.input(&key(Key::Char('1'), false));
    screen.input(&key(Key::Char('1'), true));
    screen.tick(TICK * 8);
    assert_ne!(target(&screen), None, "pressed afresh");
}

#[test]
fn thrust_held_through_the_player_info_thrusts_after_its_close() {
    // `_HandlePlayer` @0x70393.
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(Key::Up, true));
    tap(&mut screen, PLAYER_INFO_KEY);
    assert_eq!(screen.showing(), Showing::PlayerInfo);
    tap(&mut screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight);
    let closed = speed(&screen);
    screen.tick(TICK * 3);
    assert!(speed(&screen) > closed, "thrust: {:?}", ship(&screen));
}

#[test]
fn thrust_held_through_the_spaceport_thrusts_after_the_take_off() {
    // `_PlayerLandOnStellar` @0x63583: the take-off grace holds off only
    // M, P and I.
    let mut screen = with_dialogs(data());
    land(&mut screen);
    screen.input(&key(Key::Up, true));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight, "took off");
    screen.tick(TICK * 3);
    assert_ne!(
        ship(&screen).velocity,
        nova_sim::ShipState::default().velocity
    );
}

// Keys a dialog holds back while a press is held in it (nova-view's
// `ui::held_keys`) run at the release below the router, whose own
// handlers ran on them live: wiring through the real router, views and
// flight. A click held on Okay or a button holds them as a held arrow
// does.

/// Where item `item` of `dialog`, drawn over flight, is on the screen.
fn item_on_screen(screen: &AppScreen, dialog: &nova_view::ui::Dialog, item: usize) -> Point {
    let at = dialog.item_bounds(item).expect("the item").center();
    let offset = screen.overlay_offset();
    Point::new(at.x + offset.x, at.y + offset.y)
}

fn left(pressed: bool, at: Point) -> Input {
    Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    }
}

/// Flight in Sol with a mission text over it, and a click held on its
/// Okay.
fn texted_and_held() -> AppScreen {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(game_data_fork_loaded())
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    screen.show_over_flight(vec![nova_view::ui::mission_dialogs::DescShown::str_text(
        "Well done.",
        None,
    )]);
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::MissionText);
    let text = screen
        .flight_mission_text()
        .expect("shown")
        .dialog()
        .clone();
    let okay = item_on_screen(&screen, &text, nova_view::ui::desc::DONE_ITEM);
    screen.input(&left(true, okay));
    screen
}

#[test]
fn a_return_held_back_by_a_click_closes_the_text_at_the_release_and_flight_thrusts() {
    let mut screen = texted_and_held();
    screen.input(&key(Key::Up, true));
    tap(&mut screen, Key::Enter);
    assert_eq!(screen.showing(), Showing::MissionText, "it waits");
    screen.input(&left(false, Point::new(1.0, 1.0)));
    assert_eq!(screen.showing(), Showing::Flight, "closed by the release");
    let closed = speed(&screen);
    screen.tick(TICK * 3);
    assert!(speed(&screen) > closed, "thrust: {:?}", ship(&screen));
}

#[test]
fn caps_lock_pressed_during_a_held_click_toggles_double_time_once() {
    let mut screen = texted_and_held();
    let before = flight(&screen).double_time();
    tap(&mut screen, Key::CapsLock);
    assert_ne!(flight(&screen).double_time(), before, "live, as it came");
    tap(&mut screen, Key::Enter);
    screen.input(&left(false, Point::new(1.0, 1.0)));
    assert_eq!(screen.showing(), Showing::Flight);
    assert_ne!(
        flight(&screen).double_time(),
        before,
        "not toggled back by the replay"
    );
}

#[test]
fn the_flight_offer_closes_from_a_ticks_replay_after_a_release_outside() {
    let mut screen = offered_over_flight();
    let offer = screen
        .offer()
        .and_then(OfferDialog::dialog)
        .expect("laid")
        .clone();
    let accept = item_on_screen(&screen, &offer, nova_view::spaceport::offer::ACCEPT_ITEM);
    screen.input(&left(true, accept));
    tap(&mut screen, Key::Enter);
    assert_eq!(screen.showing(), Showing::Offer, "it waits");
    // The window's adapter cancels a press released outside the content.
    screen.cancel_pointer();
    assert_eq!(screen.showing(), Showing::Offer, "until the tick");
    screen.tick(TICK);
    assert_ne!(screen.showing(), Showing::Offer, "answered by the tick");
}

/// `_ShowTextDialog` flushes nothing as it closes (@0x5b353-0x5b507), so
/// an M queued after the closing Return reaches flight's loop, past the
/// router's own handlers, which ran on it and on Caps Lock live.
#[test]
fn an_m_queued_after_the_closing_return_opens_flights_map_and_double_time_toggles_once() {
    let mut screen = texted_and_held();
    let before = flight(&screen).double_time();
    tap(&mut screen, Key::CapsLock);
    tap(&mut screen, Key::Enter);
    tap(&mut screen, MAP);
    assert_eq!(screen.showing(), Showing::MissionText, "it waits");
    screen.input(&left(false, Point::new(1.0, 1.0)));
    assert_eq!(screen.showing(), Showing::FlightMap, "the M reached flight");
    assert_ne!(flight(&screen).double_time(), before, "toggled once");
}

/// The answer's `FlushEvents` pass event codes for masks and flush only
/// mouse events (@0xa2da2-0xa2dde), and a refusal flushes nothing, so an
/// M queued during the click on Refuse reaches flight after it.
#[test]
fn an_m_queued_during_a_click_that_refuses_the_flight_offer_opens_flights_map() {
    let mut screen = offered_over_flight();
    let offer = screen
        .offer()
        .and_then(OfferDialog::dialog)
        .expect("laid")
        .clone();
    let refuse = item_on_screen(&screen, &offer, nova_view::spaceport::offer::REFUSE_ITEM);
    screen.input(&left(true, refuse));
    tap(&mut screen, MAP);
    assert_eq!(screen.showing(), Showing::Offer, "it waits");
    screen.input(&left(false, refuse));
    assert!(screen.offer().is_none(), "refused");
    assert_eq!(screen.showing(), Showing::FlightMap, "the M reached flight");
}

/// With no mission active, `_OfferOneMission`'s I arm does nothing
/// (@0xa32c1-0xa32ee): an I held back by a click on Refuse, let go
/// outside it, opens nothing and flushes nothing, so the Return after it
/// answers the offer at the release.
#[test]
fn an_i_and_a_return_held_back_on_the_flight_offer_answer_it_with_no_mission_active() {
    let mut screen = offered_over_flight();
    let offer = screen
        .offer()
        .and_then(OfferDialog::dialog)
        .expect("laid")
        .clone();
    let refuse = item_on_screen(&screen, &offer, nova_view::spaceport::offer::REFUSE_ITEM);
    screen.input(&left(true, refuse));
    tap(&mut screen, nova_view::ui::mission_info::MISSION_INFO_KEY);
    tap(&mut screen, Key::Enter);
    assert_eq!(screen.showing(), Showing::Offer, "it waits");
    screen.input(&left(false, Point::new(1.0, 1.0)));
    assert!(screen.flight_mission_info().is_none(), "nothing to list");
    assert!(screen.offer().is_none(), "the Return answered it");
}
