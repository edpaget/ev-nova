//! The message line's, the jump's, the landing's, the gates', the take-offs'
//! and the death's component tests on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-9-travel-on-port): a view over a
//! scripted frame and scripted answers, the commands it sends, what it says
//! and what it draws. Kept apart from `tests/mod.rs`, which is large.

use super::faked_world::{in_sol_with, the_player_at, world};
use super::*;
use crate::flight::view::words::Words;
use nova_sim::flight_frame::FlightEvent;

/// How a test words the flight's messages: each by its `Debug` text, but
/// an arrival's lead, which waits for its arrival, and the long date as
/// [`DATE`].
pub(super) fn faked_words() -> Words<Faked> {
    Words {
        text: |_, message| match message {
            SimMessage::Arrived(_)
            | SimMessage::Buoy(_)
            | SimMessage::ExitedHypergate(_)
            | SimMessage::PassedWormhole(_) => None,
            _ => Some(format!("{message:?}")),
        },
        date: |_| DATE.to_owned(),
    }
}

/// One tick of flight time on `view`'s clock, as a frame's tick moves it
/// ([`FlightView::step_stellars`]): the message line counts it.
pub(super) fn faked_tick(view: &mut Faked) {
    view.step_stellars(TICK);
}

/// Asserts `view`'s message line says `text` now, for `n` ticks in all,
/// and not after.
#[track_caller]
pub(super) fn faked_lasts(view: &mut Faked, text: &str, n: u16) {
    assert_eq!(view.message(), Some(text));
    for _ in 1..n {
        faked_tick(view);
    }
    assert_eq!(
        view.message(),
        Some(text),
        "still shown after {} ticks",
        n - 1
    );
    faked_tick(view);
    assert_eq!(view.message(), None, "gone after {n} ticks");
}

/// The message lines `view` sent its flight, by their ticks
/// ([`FlightCommand::CommShown`]).
pub(super) fn shown_to_flight(view: &Faked) -> Vec<u16> {
    sent(view)
        .iter()
        .filter_map(|command| match command {
            FlightCommand::CommShown(ticks) => Some(*ticks),
            _ => None,
        })
        .collect()
}

// The message line.

#[test]
fn a_line_said_shows_for_its_ticks_and_sets_the_flights_line() {
    let mut view = faked(in_sol());
    view.say_for("A", 240);
    assert_eq!(shown_to_flight(&view), [240], "the session's line follows");
    faked_lasts(&mut view, "A", 240);
}

#[test]
fn a_new_message_replaces_the_old_and_its_time() {
    let mut view = faked(in_sol());
    view.say_for("A", 420);
    for _ in 0..10 {
        faked_tick(&mut view);
    }
    view.say_for("B", 240);
    faked_lasts(&mut view, "B", 240);
    assert_eq!(shown_to_flight(&view), [420, 240]);
}

#[test]
fn a_line_cleared_by_name_goes_for_a_tick_of_the_flights_line_and_another_stays() {
    let mut view = faked(in_sol());
    view.say_for(TOO_CLOSE, 360);
    view.clear_message(NO_FUEL);
    assert_eq!(view.message(), Some(TOO_CLOSE), "another line stays");
    assert_eq!(shown_to_flight(&view), [360], "and the flight's");
    view.clear_message(TOO_CLOSE);
    assert_eq!(view.message(), None);
    assert_eq!(shown_to_flight(&view), [360, 1], "an empty line for a tick");
}

#[test]
fn an_emptied_slot_shows_nothing_and_leaves_the_flights_line() {
    let mut view = faked(in_sol());
    view.say_for("A", 240);
    view.blank_message();
    assert_eq!(view.message(), None);
    assert_eq!(shown_to_flight(&view), [240], "the session settles its own");
}

#[test]
fn the_flights_messages_said_after_a_press_are_worded_each_in_turn_setting_its_line() {
    let mut view = faked(in_sol());
    for message in [SimMessage::NoResponse, SimMessage::MissionFailed] {
        view.sim.queue_event(FlightEvent::Message(message));
    }
    view.say_messages(faked_words());
    let last = format!("{:?}", SimMessage::MissionFailed);
    assert_eq!(view.message(), Some(last.as_str()), "the last replaces");
    assert_eq!(
        shown_to_flight(&view),
        [
            SimMessage::NoResponse.ticks(),
            SimMessage::MissionFailed.ticks()
        ]
    );
}

#[test]
fn the_steps_messages_are_relayed_leaving_the_flights_line() {
    let mut view = faked(in_sol());
    view.sim
        .queue_event(FlightEvent::Message(SimMessage::ShipDestroyed));
    view.relay_messages(faked_words());
    let said = format!("{:?}", SimMessage::ShipDestroyed);
    assert_eq!(view.message(), Some(said.as_str()));
    assert_eq!(shown_to_flight(&view), [0_u16; 0], "the session set it");
}

#[test]
fn an_arrivals_lead_waits_for_its_arrival() {
    let mut view = faked(in_sol());
    let said = view.word(
        vec![SimMessage::Arrived(SystemId(131)), SimMessage::NoResponse],
        faked_words(),
    );
    assert_eq!(
        said,
        [(
            format!("{:?}", SimMessage::NoResponse),
            SimMessage::NoResponse.ticks()
        )]
    );
    assert_eq!(view.pending_leads, [SimMessage::Arrived(SystemId(131))]);
}

#[test]
fn a_flight_that_never_started_makes_no_sounds() {
    let mut view = faked(FlightFrame::default());
    view.say_messages(faked_words());
    assert_eq!(view.drained_sounds(), []);
    assert_eq!(view.message(), None);
}

// The message box, drawn over a scripted world.

/// A world in Sol over the starry catalog, the player alone at the
/// centre, with `text` said.
fn saying(catalog: FakeCatalog, text: &str) -> Faked {
    let mut view = world(catalog, in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]));
    view.say_for(text, MESSAGE_TICKS);
    view
}

/// What `view` draws of its flight.
fn faked_drawn(view: &Faked) -> DrawList {
    super::faked_hud::flown(view)
}

#[test]
fn the_messages_box_draws_over_the_starfield_under_the_ships_and_its_text_over_them() {
    let mut view = saying(starry(), TOO_FAR_PLANET);
    view.reset_stars();
    let list = faked_drawn(&view);
    let commands: Vec<&DrawCommand> = list.iter().collect();
    let stars = commands.iter().take_while(|c| is_star(c)).count();
    assert!(stars > 0, "the starfield first");
    assert_eq!(
        commands[stars],
        &fill_command(hud::message_box(VIEW_SIZE), Color::BLACK),
        "then the message's box, black in a black system"
    );
    let sprite = commands
        .iter()
        .position(|c| matches!(c, DrawCommand::Sprite { .. }) && !is_star(c))
        .expect("the player's sprite");
    let text = commands
        .iter()
        .position(|c| **c == message_drawn(TOO_FAR_PLANET))
        .expect("the message");
    assert!(stars < sprite, "the ship over the box");
    assert!(sprite < text, "the text over the ship");
}

#[test]
fn a_long_message_wraps_in_the_box_by_the_metrics() {
    let text = vec!["abcd"; 60].join(" ");
    let mut view = saying(catalog(), &text);
    view.metrics = Some(super::super::Metrics(Rc::new(
        crate::text::fixture::MonoMetrics,
    )));
    let mut expected = DrawList::new();
    hud::draw_message_box(&mut expected, VIEW_SIZE, Color::BLACK);
    hud::draw_message_text(
        &mut expected,
        &text,
        VIEW_SIZE,
        Some(&crate::text::fixture::MonoMetrics),
    );
    let list = faked_drawn(&view);
    let commands: Vec<&DrawCommand> = list.iter().collect();
    let first = commands
        .iter()
        .position(|c| matches!(c, DrawCommand::Text { origin, .. } if *origin == message_at()))
        .expect("the message");
    let expected: Vec<&DrawCommand> = expected.iter().collect();
    assert_eq!(
        expected.len(),
        4,
        "the box, two whole lines and a clipped third"
    );
    assert!(commands[..first].contains(&expected[0]), "the box under");
    assert_eq!(commands[first..first + 3], expected[1..]);
}

#[test]
fn the_message_box_is_filled_in_the_systems_background_at_rgb555() {
    // `_DisplayComm` erases the rect in the system's `BkgndColor`
    // (@0x902cf, @0x6e06), which the thousands-of-colours display holds
    // as 5 bits a channel: 0x07 is 0, 0x0F is 0x08 and 0x12 is 0x10.
    let mut view = saying(catalog(), TOO_FAR_PLANET);
    view.sim.frame_mut().system_background = Some(0x0007_0F12);
    view.refresh_frame();
    let list = faked_drawn(&view);
    let commands: Vec<&DrawCommand> = list.iter().collect();
    let message = commands
        .iter()
        .position(|c| **c == message_drawn(TOO_FAR_PLANET))
        .expect("the message");
    let filled = fill_command(
        hud::message_box(VIEW_SIZE),
        Color::rgba(0x00, 0x08, 0x10, 255),
    );
    assert!(
        commands[..message].contains(&&filled),
        "the box, under the text"
    );
}

// Death.

/// [`faked`] in Sol with the flight's fate `fate` and its hold expired
/// as `expired` says.
fn dying(fate: PlayerFate, expired: bool) -> Faked {
    faked(FlightFrame {
        fate,
        hold_expired: expired,
        ..in_sol()
    })
}

/// The alpha of the black `view` draws over everything, if it does.
fn black_of(view: &Faked) -> Option<u8> {
    let mut list = DrawList::new();
    view.draw_fades(&mut list);
    list.iter().find_map(|command| match command {
        DrawCommand::Line { width, color, .. }
            if *width == VIEW_SIZE.1 && (color.r, color.g, color.b) == (0, 0, 0) =>
        {
            Some(color.a)
        }
        _ => None,
    })
}

#[test]
fn until_the_hold_has_expired_the_death_frame_leaves_the_tick_to_step() {
    for fate in [
        PlayerFate::Flying,
        PlayerFate::Exploded,
        PlayerFate::Holding { ticks_left: 5 },
        PlayerFate::Holding { ticks_left: 0 },
    ] {
        let mut view = dying(fate, false);
        assert!(!view.death_frame(TICK), "{fate:?}: the steps run");
        assert_eq!(black_of(&view), None, "{fate:?}");
        assert!(!view.game_over(), "{fate:?}");
    }
}

#[test]
fn at_the_holds_expiry_flight_stops_and_nothing_fades_until_the_fade_begins() {
    let mut view = dying(PlayerFate::Holding { ticks_left: 0 }, true);
    for _ in 0..30 {
        assert!(view.death_frame(TICK), "nothing steps");
        assert_eq!(black_of(&view), None, "nothing fades");
        assert!(!view.game_over());
    }
    assert_eq!(sent(&view), [], "no end of play asked for");
    view.sim.queue_answer(Answer::Done);
    view.sim.frame_mut().fate = PlayerFate::Fading;
    view.begin_fade();
    assert_eq!(sent(&view), [FlightCommand::BeginFade]);
    assert!(view.death_fade.is_some(), "the fade begun");
}

#[test]
fn the_death_fade_goes_to_black_over_a_second_then_ends_play() {
    let mut view = dying(PlayerFate::Fading, true);
    view.begin_fade();
    view.death_frame(Duration::from_millis(500));
    assert_eq!(black_of(&view), Some(128), "half way");
    view.death_frame(Duration::from_millis(499));
    assert_eq!(
        sent(&view),
        [FlightCommand::BeginFade],
        "not over: no end yet"
    );
    view.sim.queue_answer(Answer::Done);
    view.sim.frame_mut().fate = PlayerFate::GameOver;
    view.death_frame(Duration::from_millis(1));
    assert_eq!(black_of(&view), Some(255));
    assert_eq!(
        sent(&view),
        [FlightCommand::BeginFade, FlightCommand::EndPlay],
        "play ended at the fade's end"
    );
    assert!(view.game_over(), "the frame's fate");
}

#[test]
fn beginning_the_fade_before_the_hold_expires_does_nothing() {
    let mut view = dying(PlayerFate::Holding { ticks_left: 3 }, false);
    view.begin_fade();
    assert_eq!(sent(&view), [FlightCommand::BeginFade], "refused");
    assert!(!view.death_frame(Duration::from_millis(1500)));
    assert_eq!(black_of(&view), None);
    assert!(!view.game_over());
}

#[test]
fn the_sabotaged_players_death_descs_are_the_flights() {
    let mut view = faked(in_sol());
    assert_eq!(view.sabotage_death_descs(), Vec::<i16>::new());
    view.sim.set_death_descs(vec![5000]);
    assert_eq!(view.sabotage_death_descs(), [5000]);
}

#[test]
fn the_flight_keys_held_are_the_views() {
    let mut view = faked(in_sol());
    assert!(!view.holds_keys());
    view.track_held(&key(Key::Up, true));
    assert!(view.holds_keys());
}
