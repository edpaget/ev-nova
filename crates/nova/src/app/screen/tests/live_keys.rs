use super::encounters::{boardable, boarding_data, hailable};
use super::map_wait_keys::{game_data_fork_loaded, job, offered_over_flight};
use super::*;
use nova_view::flight::view::HAIL_KEY;
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
    screen.input(&key(Key::ArrowUp, true));
    offer_as_a_hail_would(&mut screen);
    screen
}

/// Opens a person's offer over flight as its hail does: flight lets go
/// of its keys and the button and marks the site (`FlightView::hold_offer`,
/// @0x622ac), then the router opens it; a tick over it follows, as the
/// original's next frame does.
fn offer_as_a_hail_would(screen: &mut AppScreen) {
    flight_mut(screen).cancel_pointer();
    flight_mut(screen).release_keys();
    flight_mut(screen).mark_site();
    screen.open_offer(job());
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::Offer);
}

/// Refuses the offer open over flight, which closes back to flight.
fn refuse(screen: &mut AppScreen) {
    tap(screen, Key::KeyN);
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
    screen.input(&key(Key::ArrowUp, true));
    refuse(&mut screen);
    screen.tick(TICK * 3);
    assert_ne!(ship(&screen), still, "thrust");
}

#[test]
fn thrust_let_go_during_the_offer_doesnt() {
    let mut screen = offered_thrusting();
    screen.input(&key(Key::ArrowUp, false));
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
    screen.input(&key(Key::ArrowUp, false));
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
    screen.input(&key(Key::ArrowUp, false));
    tap(&mut screen, Key::ArrowUp);
    tap(&mut screen, Key::KeyN);
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
    screen.input(&key(Key::ArrowUp, true));
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
    screen.input(&key(Key::ArrowUp, true));
    tap(&mut screen, Key::KeyB);
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
    screen.input(&key(Key::Digit1, true));
    refuse(&mut screen);
    screen.tick(TICK * 8);
    assert_eq!(target(&screen), None, "held across the close: latched");
    screen.input(&key(Key::Digit1, false));
    screen.input(&key(Key::Digit1, true));
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
    screen.input(&key(Key::Digit1, true));
    refuse(&mut screen);
    for _ in 0..4 {
        screen.tick(TICK * 2);
        screen.input(&Input::Key {
            key: Key::Digit1,
            pressed: true,
            repeat: true,
        });
    }
    screen.tick(TICK * 8);
    assert_eq!(target(&screen), None, "repeats: still latched");
    screen.input(&key(Key::Digit1, false));
    screen.input(&key(Key::Digit1, true));
    screen.tick(TICK * 8);
    assert_ne!(target(&screen), None, "pressed afresh");
}

#[test]
fn thrust_held_through_the_player_info_thrusts_after_its_close() {
    // `_HandlePlayer` @0x70393.
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(Key::ArrowUp, true));
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
    screen.input(&key(Key::ArrowUp, true));
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

/// Flight in Sol, `before` run on it, then a mission text queued over it
/// as the session's texts are (`AppScreen::over_flight`), shown by a
/// tick: no `_SetAllKeyCmdFlags` follows it (`_MissionHandler`,
/// `_DoPlayGameWork` @0x44214).
fn texted_after(before: impl FnOnce(&mut AppScreen)) -> AppScreen {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(game_data_fork_loaded())
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    before(&mut screen);
    screen.show_over_flight(vec![nova_view::ui::mission_dialogs::DescShown::str_text(
        "Well done.",
        None,
    )]);
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::MissionText);
    screen
}

/// [`texted_after`] nothing.
fn texted() -> AppScreen {
    texted_after(|_| {})
}

/// Flight in Sol with a mission text over it, and a click held on its
/// Okay.
pub(super) fn texted_and_held() -> AppScreen {
    let mut screen = texted();
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
    screen.input(&key(Key::ArrowUp, true));
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

// A latched key pressed under a dialog no `_SetAllKeyCmdFlags` follows,
// and still down at its close, acts on the first step after it: the
// router replays what flight gives back (the `live_input` module).

/// The navigation target `screen`'s flight has selected.
fn nav(screen: &AppScreen) -> Option<nova_sim::StellarId> {
    flight(screen).session().expect("flying").nav_target()
}

/// Closes the mission text in front with Return, let go after.
fn close_text(screen: &mut AppScreen) {
    tap(screen, Key::Enter);
    assert_eq!(screen.showing(), Showing::Flight, "closed");
}

const SLOT_1: Key = Key::Digit1;

#[test]
fn a_slot_key_pressed_under_a_mission_text_and_still_down_selects_after_its_close() {
    let mut screen = texted();
    screen.input(&key(SLOT_1, true));
    close_text(&mut screen);
    screen.tick(TICK);
    assert_ne!(nav(&screen), None, "selected on the first step");
}

#[test]
fn a_slot_key_pressed_and_let_go_under_a_mission_text_selects_nothing() {
    let mut screen = texted();
    tap(&mut screen, SLOT_1);
    close_text(&mut screen);
    screen.tick(TICK * 8);
    assert_eq!(nav(&screen), None);
}

/// Whether `sounds` hold the flight's refusal beep, as a hail with
/// nothing to hail beeps.
fn refused(sounds: &[Sound]) -> bool {
    sounds.contains(&Sound::Sim(nova_sim::SimSound::Refused))
}

#[test]
fn a_hail_key_held_from_before_a_mission_text_waits_for_its_release() {
    // Y pressed before the text: the original's poll before it read Y
    // down, and none sees its release.
    let mut screen = texted_after(|screen| {
        screen.input(&key(HAIL_KEY, true));
        assert!(refused(&screen.take_sounds()), "nothing to hail");
    });
    close_text(&mut screen);
    screen.tick(TICK * 4);
    let sounds = screen.take_sounds();
    assert!(!refused(&sounds), "no second hail: {sounds:?}");
    screen.input(&key(HAIL_KEY, false));
    screen.input(&key(HAIL_KEY, true));
    assert!(refused(&screen.take_sounds()), "pressed afresh, it hails");
}

#[test]
fn a_hail_key_down_at_the_poll_before_a_mission_text_waits_though_let_go_and_pressed_again() {
    // The poll read Y down; its release, before the text opened, and its
    // press under it, no poll saw.
    let mut screen = texted_after(|screen| {
        screen.input(&key(HAIL_KEY, true));
        screen.tick(TICK);
        screen.input(&key(HAIL_KEY, false));
        let _ = screen.take_sounds();
    });
    screen.input(&key(HAIL_KEY, true));
    close_text(&mut screen);
    screen.tick(TICK * 4);
    assert!(!refused(&screen.take_sounds()), "no second hail");
}

#[test]
fn p_and_y_pressed_under_a_mission_text_open_the_player_info_and_y_waits() {
    // P's dialog runs `_SetAllKeyCmdFlags` (@0x70393) before Y's read
    // (@0x703e9).
    let mut screen = texted_after(|screen| {
        tap(screen, SLOT_1);
        screen.tick(TICK);
        assert_ne!(nav(screen), None, "Earth to hail");
    });
    screen.input(&key(PLAYER_INFO_KEY, true));
    screen.input(&key(HAIL_KEY, true));
    tap(&mut screen, Key::Enter);
    assert_eq!(
        screen.showing(),
        Showing::PlayerInfo,
        "P acted at the close"
    );
    screen.input(&key(PLAYER_INFO_KEY, false));
    tap(&mut screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight, "Y latched");
    screen.tick(TICK * 4);
    assert_eq!(screen.showing(), Showing::Flight, "Y still latched");
    screen.input(&key(HAIL_KEY, false));
    screen.input(&key(HAIL_KEY, true));
    assert_ne!(
        screen.showing(),
        Showing::Flight,
        "pressed afresh, it hails"
    );
}

#[test]
fn a_slot_key_pressed_under_the_preferences_and_still_down_selects_after_its_close() {
    let mut screen = texted();
    close_text(&mut screen);
    command_comma(&mut screen);
    assert_eq!(screen.showing(), Showing::Preferences);
    screen.input(&key(SLOT_1, true));
    tap(&mut screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight);
    screen.tick(TICK);
    assert_ne!(nav(&screen), None, "selected on the first step");
}

#[test]
fn a_slot_key_pressed_under_the_about_text_and_still_down_selects_after_its_close() {
    let mut screen = texted();
    close_text(&mut screen);
    screen.menu_command(MenuCommand::About);
    assert_eq!(screen.showing(), Showing::About);
    screen.input(&key(SLOT_1, true));
    tap(&mut screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight);
    screen.tick(TICK);
    assert_ne!(nav(&screen), None, "selected on the first step");
}

#[test]
fn a_slot_key_pressed_in_the_spaceport_and_held_past_the_take_off_waits_for_its_release() {
    // `_PlayerLandOnStellar` @0x63583.
    let mut screen = with_dialogs(game_data_fork_loaded());
    land(&mut screen);
    // L, which landed, let go: held, it would pick a stellar of its own.
    screen.input(&key(Key::KeyL, false));
    screen.input(&key(SLOT_1, true));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight, "took off");
    screen.tick(TICK * 8);
    assert_eq!(nav(&screen), None, "latched");
    screen.input(&key(SLOT_1, false));
    screen.input(&key(SLOT_1, true));
    screen.tick(TICK * 8);
    assert_ne!(nav(&screen), None, "pressed afresh");
}

/// A mission text shown over `screen`'s flight, as the session's are.
fn text_over(screen: &mut AppScreen) {
    screen.show_over_flight(vec![nova_view::ui::mission_dialogs::DescShown::str_text(
        "Well done.",
        None,
    )]);
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::MissionText);
}

#[test]
fn b_and_p_pressed_under_a_mission_text_board_and_p_waits_on_the_plunders_latch() {
    // B @0x6a98b before P @0x702cb: B, held, boards at the next pass on
    // its own latch (0xdee18), and the board opens the plunder dialog,
    // whose `_SetAllKeyCmdFlags` (@0x659f1) latches P.
    let mut screen = boardable(with_dialogs(boarding_data()));
    fly(&mut screen);
    screen.tick(TICK);
    tap(&mut screen, Key::Tab);
    text_over(&mut screen);
    screen.input(&key(Key::KeyB, true));
    screen.input(&key(PLAYER_INFO_KEY, true));
    tap(&mut screen, Key::Enter);
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::Plunder, "B boarded at the pass");
    tap(&mut screen, Key::Escape);
    assert_eq!(screen.showing(), Showing::Flight, "aborted");
    screen.tick(TICK * 4);
    assert_eq!(screen.showing(), Showing::Flight, "P latched");
}

#[test]
fn a_slot_key_and_p_pressed_under_a_mission_text_select_then_open_the_player_info() {
    // The slot block @0x69d3f picks before P @0x702cb opens its dialog.
    let mut screen = texted();
    screen.input(&key(SLOT_1, true));
    screen.input(&key(PLAYER_INFO_KEY, true));
    close_text(&mut screen);
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::PlayerInfo, "P on the first step");
    assert_ne!(nav(&screen), None, "picked first");
}

// B's and L's latches are no site's (0xdee18, 0xdee34): each is read on
// every pass it is down, across every close.

#[test]
fn l_held_across_the_take_off_with_no_stellar_targeted_targets_the_nearest() {
    // The take-off lets the selection go and is a site (@0x63583), but L's
    // block picks the nearest on every pass L is down (@0x6a8c5-0x6a932),
    // its latch set by the landing's request (@0x6a953) holding the
    // request back.
    let mut screen = with_dialogs(data());
    land(&mut screen);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight, "took off");
    assert_eq!(nav(&screen), None, "the take-off let it go");
    let said = flight(&screen).message().map(str::to_owned);
    screen.tick(TICK);
    assert_ne!(nav(&screen), None, "the nearest");
    assert_eq!(
        flight(&screen).message().map(str::to_owned),
        said,
        "no request said"
    );
}

#[test]
fn b_pressed_under_a_dialog_boards_at_its_close() {
    // A site's dialog latches what is down at its close, but not B's own
    // latch (0xdee18), which the last pass before it read up.
    let mut screen = boardable(with_dialogs(boarding_data()));
    fly(&mut screen);
    screen.tick(TICK);
    tap(&mut screen, Key::Tab);
    offer_as_a_hail_would(&mut screen);
    screen.input(&key(Key::KeyB, true));
    refuse(&mut screen);
    screen.tick(TICK);
    assert_eq!(screen.showing(), Showing::Plunder, "B boarded");
}
