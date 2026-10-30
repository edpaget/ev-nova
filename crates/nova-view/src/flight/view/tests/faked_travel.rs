//! The message line's, the jump's, the landing's, the gates', the take-offs'
//! and the death's component tests on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-9-travel-on-port): a view over a
//! scripted frame and scripted answers, the commands it sends, what it says
//! and what it draws. Kept apart from `tests/mod.rs`, which is large.

use super::faked_world::world;
use super::*;
use crate::flight::escorts::fighters_abandoned_message;
use crate::flight::jump::{FADE_IN_FOR, FADE_OUT_FOR, JumpEffect, JumpPhase};
use nova_sim::flight_frame::FlightEvent;
use nova_sim::{SessionEvent, SimSound};

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

/// The commands `view` sent its flight, the message line's left out.
pub(super) fn sent_but_lines(view: &Faked) -> Vec<FlightCommand> {
    sent(view)
        .iter()
        .filter(|command| !matches!(command, FlightCommand::CommShown(_)))
        .cloned()
        .collect()
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
    view.say_messages();
    assert_eq!(view.message(), Some(MISSION_FAILED), "the last replaces");
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
    view.relay_messages();
    assert_eq!(view.message(), Some(SHIP_DESTROYED));
    assert_eq!(shown_to_flight(&view), [0_u16; 0], "the session set it");
}

#[test]
fn an_arrivals_lead_waits_for_its_arrival() {
    let mut view = faked(in_sol());
    let said = view.word(vec![
        SimMessage::Arrived(SystemId(131)),
        SimMessage::NoResponse,
    ]);
    assert_eq!(
        said,
        [(NO_RESPONSE.to_owned(), SimMessage::NoResponse.ticks())]
    );
    assert_eq!(view.pending_leads, [SimMessage::Arrived(SystemId(131))]);
}

#[test]
fn a_flight_that_never_started_makes_no_sounds() {
    let mut view = faked(FlightFrame::default());
    view.say_messages();
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
    assert!(view.death_fade.is_none(), "no fade begun");
    assert!(!view.death_frame(Duration::from_millis(1500)));
    assert_eq!(black_of(&view), None);
    assert!(!view.game_over());
}

#[test]
fn the_sabotaged_players_death_descs_are_the_frames() {
    let mut view = faked(in_sol());
    assert_eq!(view.sabotage_death_descs(), Vec::<i16>::new());
    view.change_sim(|sim| sim.frame_mut().death_descs = vec![5000]);
    assert_eq!(view.sabotage_death_descs(), [5000]);
}

#[test]
fn the_flight_keys_held_are_the_views() {
    let mut view = faked(in_sol());
    assert!(!view.holds_keys());
    view.track_held(&key(Key::Up, true));
    assert!(view.holds_keys());
}

// The jump.

/// Shows `frame` from now on, the kept frame filled from it, as a step
/// leaves it.
pub(super) fn now_shows(view: &mut Faked, frame: FlightFrame) {
    view.sim.set_frame(frame);
    view.refresh_frame();
}

#[test]
fn j_says_a_refusal_the_player_is_told_in_its_words_for_its_ticks_and_beeps_as_drained() {
    use nova_sim::JumpRefusal;
    for (refusal, text, ticks) in [
        (JumpRefusal::NoDestination, NO_DESTINATION, 360),
        (JumpRefusal::TooClose { distance: 0.0 }, TOO_CLOSE, 360),
        (JumpRefusal::Tractored, TRACTORED, 360),
        (JumpRefusal::NoFuel { fuel: 50.0 }, NO_FUEL, 240),
        (JumpRefusal::Landed, TAKE_OFF_FIRST, 240),
    ] {
        let mut view = faked(in_sol());
        view.sim.queue_answer(Answer::JumpRefused {
            refusal,
            told: true,
        });
        view.sim.queue_event(FlightEvent::Sound(SimSound::Refused));
        view.jump_frame(true);
        assert_eq!(sent_but_lines(&view), [FlightCommand::Jump], "{refusal:?}");
        assert_eq!(view.jump_effect(), None, "{refusal:?}");
        assert_eq!(
            view.drained_sounds(),
            [Sound::Sim(SimSound::Refused)],
            "{refusal:?}: the flight's beep"
        );
        faked_lasts(&mut view, text, ticks);
    }
    let mut untold = faked(in_sol());
    untold.sim.queue_answer(Answer::JumpRefused {
        refusal: JumpRefusal::NoDestination,
        told: false,
    });
    untold.jump_frame(true);
    assert_eq!(untold.message(), None, "a refusal not told says nothing");
    assert_eq!(shown_to_flight(&untold), [0_u16; 0]);
}

#[test]
fn j_acts_only_on_an_intact_frame() {
    for condition in [
        Condition::Disabled,
        Condition::Dying {
            ticks_left: 100,
            starting: false,
        },
    ] {
        let mut view = faked(in_sol());
        view.change_sim(|sim| sim.frame_mut().ships[0].condition = condition);
        view.jump_frame(true);
        assert_eq!(sent(&view), [], "{condition:?}: no J");
        view.change_sim(|sim| sim.frame_mut().ships[0].condition = Condition::Intact);
        view.jump_frame(true);
        assert_eq!(sent(&view), [FlightCommand::Jump], "{condition:?}: intact");
    }
}

#[test]
fn starting_a_jump_clears_the_hints_and_too_close_and_keeps_any_other_line() {
    for text in [FAR_ENOUGH, NOTHING_OUT_HERE, TOO_CLOSE, TRACTORED] {
        let mut view = faked(in_sol());
        view.say_for(text, 512);
        view.sim.queue_answer(Answer::Jumped(SystemId(131)));
        view.jump_frame(true);
        assert_eq!(view.message(), None, "{text}");
        assert_eq!(shown_to_flight(&view), [512, 1], "{text}: a tick of none");
    }
    for text in [NO_FUEL, MISSION_FAILED, NO_DESTINATION] {
        let mut view = faked(in_sol());
        view.say_for(text, 240);
        view.sim.queue_answer(Answer::Jumped(SystemId(131)));
        view.jump_frame(true);
        assert_eq!(view.message(), Some(text), "{text} stays");
    }
}

#[test]
fn the_jump_effect_waits_for_the_jump_itself_and_starts_at_once_when_it_has() {
    let mut view = faked(sol());
    view.sim
        .queue_answer_showing(Answer::Jumped(SystemId(131)), under_way(false));
    view.jump_frame(true);
    assert_eq!(view.jump_effect(), None, "braking and turning first");
    for _ in 0..3 {
        view.start_jump_effect();
        assert_eq!(view.jump_effect(), None);
    }
    now_shows(&mut view, under_way(true));
    view.start_jump_effect();
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Warp),
        "on the frame the jump itself begins"
    );
    let mut ready = faked(sol());
    ready
        .sim
        .queue_answer_showing(Answer::Jumped(SystemId(131)), under_way(true));
    ready.jump_frame(true);
    assert_eq!(
        ready.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Warp),
        "a ship ready to jump warps on J"
    );
}

/// A view whose flight warps to Alpha Centauri, its effect begun.
pub(super) fn warping() -> Faked {
    let mut view = faked(sol());
    jump_now(&mut view);
    view
}

/// Presses J on `view`'s flight, which begins its jump at once.
pub(super) fn jump_now(view: &mut Faked) {
    view.sim
        .queue_answer_showing(Answer::Jumped(SystemId(131)), under_way(true));
    view.jump_frame(true);
    assert!(view.jump_effect().is_some(), "warping");
}

/// [`warping`]'s view with the frame's `_hyperGamma` at `gamma`.
pub(super) fn lit(view: &mut Faked, gamma: f32) {
    now_shows(
        view,
        FlightFrame {
            hyper_gamma: gamma,
            ..under_way(true)
        },
    );
    view.start_jump_effect();
}

/// The frame of the flight arrived in Alpha Centauri (131), its jump
/// over, and the session's event of the system entered drained next.
pub(super) fn arrived_in_alpha(view: &mut Faked) {
    view.sim.set_frame(FlightFrame {
        system: SystemId(131),
        stellars: Vec::new(),
        ..sol()
    });
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
    view.sim
        .queue_event(FlightEvent::Message(SimMessage::Arrived(SystemId(131))));
}

/// The warp's end: the frame says it is due, and the flight answers the
/// arrival with `combat`.
pub(super) fn at_the_warps_end(view: &mut Faked, combat: Vec<CombatEvent>) {
    now_shows(
        view,
        FlightFrame {
            warp_end_due: true,
            ..under_way(true)
        },
    );
    view.sim.queue_answer(Answer::Arrived {
        system: SystemId(131),
        combat,
    });
    arrived_in_alpha(view);
}

#[test]
fn the_fade_to_white_starts_as_the_frame_lights_the_screen_and_the_arrival_waits_for_the_warps_end()
{
    let mut view = warping();
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Warp)
    );
    view.arrival_frame();
    assert_eq!(
        sent_but_lines(&view),
        [FlightCommand::Jump],
        "no arrival yet"
    );
    lit(&mut view, 0.5);
    assert!(matches!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeOut(_))
    ));
    view.fade_frame(TICK);
    assert!(view.jump.is_some(), "a jump under way is no jump given up");
    if let Some(effect) = &mut view.jump {
        effect.advance(ms(750));
    }
    let mut list = DrawList::new();
    view.draw_fades(&mut list);
    assert!(fade(&list).is_some(), "fading to white");
    view.arrival_frame();
    assert!(matches!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeOut(_))
    ));
    at_the_warps_end(&mut view, Vec::new());
    view.arrival_frame();
    assert_eq!(
        sent_but_lines(&view),
        [FlightCommand::Jump, FlightCommand::Arrive]
    );
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeIn(0.0)),
        "white at arrival"
    );
    assert!(view.jump.is_none(), "the warp's effect is the fade-in now");
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(view.course_map().current(), Some(SystemId(131)));
    if let Some(effect) = &mut view.fade_in {
        effect.advance(FADE_IN_FOR);
    }
    view.fade_frame(TICK);
    assert_eq!(view.jump_effect(), None, "then plain flight");
}

#[test]
fn a_jump_given_up_before_the_screen_is_lit_drops_its_effect() {
    let mut view = warping();
    now_shows(&mut view, in_sol());
    view.fade_frame(TICK);
    assert_eq!(view.jump_effect(), None);
    let mut list = DrawList::new();
    view.draw_fades(&mut list);
    assert_eq!(fade(&list), None, "nothing white");
}

#[test]
fn a_jump_given_up_once_the_screen_is_lit_fades_back_from_white() {
    let mut view = warping();
    lit(&mut view, 0.5);
    if let Some(effect) = &mut view.jump {
        effect.advance(FADE_OUT_FOR);
    }
    now_shows(&mut view, in_sol());
    view.fade_frame(TICK);
    assert!(
        matches!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::FadeIn(_))
        ),
        "{:?}",
        view.jump_effect()
    );
    view.fade_frame(FADE_IN_FOR);
    view.fade_frame(TICK);
    assert_eq!(view.jump_effect(), None, "the white gone");
}

#[test]
fn with_effects_off_the_arrival_flashes_white_once() {
    let mut view = faked(sol()).with_hyperspace_effects(false);
    assert!(!view.hyperspace_effects());
    jump_now(&mut view);
    at_the_warps_end(&mut view, Vec::new());
    view.arrival_frame();
    let mut list = DrawList::new();
    view.draw_fades(&mut list);
    assert_eq!(
        fade(&list).map(|(_, color)| color),
        Some(Color::WHITE),
        "opaque white"
    );
    view.fade_frame(ms(33));
    assert_eq!(view.jump_effect(), None, "no fade in");
    let mut on = faked(sol()).with_hyperspace_effects(false);
    on.set_hyperspace_effects(true);
    assert!(on.hyperspace_effects(), "set back on");
    jump_now(&mut on);
    lit(&mut on, 0.5);
    assert!(
        matches!(
            on.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::FadeOut(_))
        ),
        "set back on, the next jump fades"
    );
}

#[test]
fn arriving_shows_the_arrival_line_with_the_date_and_lets_a_refusal_go() {
    let mut view = warping();
    view.say_for(NO_DESTINATION, 360);
    at_the_warps_end(&mut view, Vec::new());
    view.arrival_frame();
    let text = arrival_message("Alpha Centauri", DATE, false);
    faked_lasts(&mut view, &text, 240);
}

#[test]
fn j_during_the_arrival_fade_in_begins_the_next_jump_and_the_fade_plays_on() {
    let mut view = warping();
    at_the_warps_end(&mut view, Vec::new());
    view.arrival_frame();
    let arrival = view.message().expect("the arrival line").to_owned();
    view.sim.queue_answer_showing(
        Answer::Jumped(SystemId(132)),
        FlightFrame {
            system: SystemId(131),
            ..under_way(false)
        },
    );
    // A fresh press follows a release, which clears the latch.
    view.jump_latch = false;
    view.jump_frame(true);
    assert_eq!(view.message(), Some(arrival.as_str()), "not refused");
    assert!(view.fade_in.is_some(), "the fade-in plays on");
    assert_eq!(sent_but_lines(&view).last(), Some(&FlightCommand::Jump));
}

#[test]
fn j_in_a_flight_that_never_started_is_refused_and_does_nothing() {
    let mut view = faked(FlightFrame::default());
    view.jump_frame(true);
    assert_eq!(sent(&view), [FlightCommand::Jump]);
    assert_eq!(view.jump_effect(), None);
    assert_eq!(view.message(), None);
}

#[test]
fn the_course_map_starts_on_the_current_system_with_no_course() {
    let view = faked(in_sol());
    assert!(!view.map_open());
    assert_eq!(view.course_map().mode(), MapMode::Course);
    assert_eq!(view.course_map().current(), Some(SystemId(130)));
    assert_eq!(view.course_map().route(), []);
    assert_eq!(view.jump_effect(), None);
    assert_eq!((MAP_KEY, JUMP_KEY), (MAP, JUMP));
}

#[test]
fn a_failed_flights_catch_up_redraws_nothing() {
    let mut view = faked(FlightFrame::default());
    let scene = view.scene().map(SystemScene::id);
    assert_eq!(view.catch_up(), Redrawn::default());
    assert_eq!(view.frame, FlightFrame::default(), "the default frame");
    assert_eq!(
        view.scene().map(SystemScene::id),
        scene,
        "laid out as it was"
    );
}

// J's frame (controls.rs's).

/// J held through `frames` frames of `view` with the frame's condition
/// `condition`, a jump answered `answer` at each J sent.
fn j_frames(view: &mut Faked, condition: Condition, frames: usize) {
    view.change_sim(|sim| sim.frame_mut().ships[0].condition = condition);
    for _ in 0..frames {
        view.jump_frame(true);
    }
}

#[test]
fn j_held_into_a_disable_acts_again_on_the_first_intact_tick() {
    let mut view = faked(in_sol());
    j_frames(&mut view, Condition::Intact, 1);
    assert_eq!(sent(&view), [FlightCommand::Jump], "refused once");
    j_frames(&mut view, Condition::Disabled, 3);
    assert_eq!(sent(&view), [FlightCommand::Jump], "the latch cleared");
    j_frames(&mut view, Condition::Intact, 2);
    assert_eq!(
        sent(&view),
        [FlightCommand::Jump, FlightCommand::Jump],
        "again on the first intact frame, once"
    );
}

#[test]
fn j_first_pressed_while_disabled_and_held_jumps_on_the_first_intact_tick() {
    let mut view = faked(in_sol());
    j_frames(&mut view, Condition::Disabled, 2);
    assert_eq!(sent(&view), []);
    j_frames(&mut view, Condition::Intact, 1);
    assert_eq!(sent(&view), [FlightCommand::Jump]);
}

#[test]
fn j_let_go_while_disabled_does_nothing_on_the_repair() {
    let mut view = faked(in_sol());
    j_frames(&mut view, Condition::Disabled, 2);
    view.jump_frame(false);
    view.change_sim(|sim| sim.frame_mut().ships[0].condition = Condition::Intact);
    view.jump_frame(false);
    assert_eq!(sent(&view), []);
}

#[test]
fn j_held_while_intact_is_refused_once() {
    let mut view = faked(in_sol());
    j_frames(&mut view, Condition::Intact, 5);
    assert_eq!(sent(&view), [FlightCommand::Jump]);
}

#[test]
fn j_held_through_a_jump_does_nothing_after_it() {
    let mut view = warping();
    for _ in 0..3 {
        view.jump_frame(true);
    }
    now_shows(&mut view, in_sol());
    for _ in 0..3 {
        view.jump_frame(true);
    }
    assert_eq!(sent(&view), [FlightCommand::Jump], "latched through it");
    view.jump_frame(false);
    view.jump_frame(true);
    assert_eq!(
        sent(&view),
        [FlightCommand::Jump, FlightCommand::Jump],
        "a fresh press after the let-go"
    );
}

// The arrival's world.

#[test]
fn an_arrivals_pending_explosions_are_drawn_on_its_first_frame_and_unheard_from_the_view() {
    let boom = nova_sim::catalog::Explosion::decode(0).expect("type 0");
    let mut view = world(
        armed(0, &[BLASTER]),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]),
    );
    jump_now(&mut view);
    at_the_warps_end(
        &mut view,
        vec![CombatEvent::ExplodedHeard {
            at: Vec2::ZERO,
            explosion: boom,
        }],
    );
    view.drained_sounds();
    view.arrival_frame();
    let explosions = view.effects().explosions();
    assert_eq!(explosions.len(), 1, "{explosions:?}");
    assert_eq!(explosions[0].boom, boom.boom);
    assert_eq!(explosions[0].at, Point::new(0.0, 0.0));
    assert_eq!(explosions[0].frame, 1.0, "advanced once: {explosions:?}");
    assert!(
        !view
            .drained_sounds()
            .iter()
            .any(|sound| matches!(sound, Sound::Combat(_))),
        "a `Y`'s: the session raised its sound"
    );
}

#[test]
fn arriving_lays_out_the_new_system_letting_go_of_the_effects_pods_and_smoke() {
    let mut view = world(
        armed(0, &[BLASTER]),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]),
    );
    super::faked_world::raise(
        &mut view,
        &[CombatEvent::Exploded {
            at: Vec2::ZERO,
            explosion: nova_sim::catalog::Explosion::decode(0).expect("type 0"),
            size: 0.0,
        }],
    );
    assert_ne!(*view.effects(), Effects::default());
    jump_now(&mut view);
    at_the_warps_end(&mut view, Vec::new());
    view.arrival_frame();
    assert_eq!(*view.effects(), Effects::default());
    assert!(view.effects().pods().is_empty());
    assert_eq!(view.shot_looks.smoke().puffs(), []);
}

#[test]
fn arriving_reads_the_new_systems_traffic_sheets() {
    let mut view = world(trafficked(&[131], 2, 129, 1), sol());
    jump_now(&mut view);
    let read = view.catalog().sheets_asked.borrow().clone();
    at_the_warps_end(&mut view, Vec::new());
    view.sim.frame_mut().traffic_ships = vec![ShipId(129)];
    view.arrival_frame();
    let asked = view.catalog().sheets_asked.borrow().clone();
    assert_eq!(asked[read.len()..], [ShipId(129)], "read on arrival");
}

#[test]
fn a_jumps_arrival_puts_the_maps_scroll_back_on_the_current_system() {
    let mut view = warping();
    view.map.centre_on_system(SystemId(132));
    at_the_warps_end(&mut view, Vec::new());
    view.arrival_frame();
    let mut recentred = faked(FlightFrame {
        system: SystemId(131),
        ..FlightFrame::default()
    });
    recentred.map.recentre();
    assert_eq!(
        view.course_map()
            .place()
            .map(crate::galaxy::place::MapPlace::scroll),
        recentred
            .course_map()
            .place()
            .map(crate::galaxy::place::MapPlace::scroll)
    );
}

#[test]
fn an_arrival_shows_the_new_systems_stellars_on_the_radar_at_once_leaving_the_cadence() {
    let mut view = warping();
    let cadence = view.hud_cadence;
    at_the_warps_end(&mut view, Vec::new());
    view.sim.frame_mut().stellars = vec![record::stellar(StellarId(140))];
    view.arrival_frame();
    assert_eq!(view.hud_cadence, cadence, "neither timed nor flipped");
    let stellars: Vec<Point> = view
        .radar_shown(&view.frame_now())
        .stellars
        .iter()
        .map(|s| s.position)
        .collect();
    assert_eq!(stellars, [at(0.0, 0.0)], "Alpha Centauri's Proxima");
}

#[test]
fn the_escape_pods_rescue_lays_the_view_out_where_it_put_the_ship_and_gives_its_desc() {
    use nova_sim::flight_frame::{ShipPlace, StepReport, StepRun};
    let mut view = faked(sol());
    view.map.centre_on_system(SystemId(132));
    let there = ShipPlace {
        position: Vec2::new(50.0, 50.0),
        heading: 90.0,
    };
    view.sim.queue_report(StepReport {
        steps: vec![StepRun {
            rescued: Some(there),
            ..StepRun::default()
        }],
        stopped: true,
    });
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
    view.sim
        .queue_event(FlightEvent::RescueDesc(rescued(13999)));
    view.sim.set_frame(FlightFrame {
        system: SystemId(131),
        ships: vec![the_player_at(Vec2::new(50.0, 50.0), 90.0)],
        ..sol()
    });
    view.run_steps(1);
    assert_eq!(
        view.previous.position,
        Vec2::new(50.0, 50.0),
        "laid out there"
    );
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(view.take_rescue_desc(), Some(rescued(13999)));
    assert_eq!(view.take_rescue_desc(), None, "once");
    let mut recentred = faked(FlightFrame {
        system: SystemId(131),
        ..FlightFrame::default()
    });
    recentred.map.recentre();
    assert_eq!(
        view.course_map()
            .place()
            .map(crate::galaxy::place::MapPlace::scroll),
        recentred
            .course_map()
            .place()
            .map(crate::galaxy::place::MapPlace::scroll),
        "the map's scroll on the system the pod reached"
    );
}

#[test]
fn a_change_of_ship_during_the_streak_reads_its_sheet() {
    let mut view = world(shipbuying(), sol());
    jump_now(&mut view);
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::ShipChanged));
    view.sim.set_frame(FlightFrame {
        ships: vec![record::player(129, Vec2::ZERO, 0.0)],
        ..under_way(true)
    });
    view.catch_up();
    assert!(view.jump_effect().is_some(), "still streaking");
    assert_eq!(view.sheet_ship, Some(ShipId(129)), "the hull redrawn");
}

#[test]
fn the_cursor_draws_under_the_jumps_fade() {
    let mut view = world(catalog(), sol());
    view.looks = Looks::read(&*view.catalog, std::iter::empty());
    jump_now(&mut view);
    lit(&mut view, 0.5);
    if let Some(effect) = &mut view.jump {
        effect.advance(ms(750));
    }
    view.cursor_input(&Input::PointerMoved(Point::new(40.0, 400.0)));
    let mut list = super::faked_hud::flown(&view);
    view.draw_fades(&mut list);
    let cursor = list
        .iter()
        .position(|command| {
            matches!(unclipped(command), DrawCommand::Sprite { image, .. } if image.id == 651)
        })
        .expect("drawn");
    let (at_fade, _) = fade(&list).expect("fading out");
    assert!(cursor < at_fade, "under the fade");
}

// The stars' scattering at an arrival.

/// A view over a starry world, its effects rolled on a counting chance,
/// caught up with once, so its stars are scattered.
fn starry_world() -> Faked {
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(super::stars::Counting::default()));
    let mut view = world(starry(), sol()).with_effects_chance(SharedChance::new(chance));
    view.catch_up();
    view
}

/// An arrival's scattering: the flight tells the stars to scatter afresh
/// ([`SessionEvent::StarfieldReset`]) as it lays the new system out.
fn scattered_on_arrival(view: &mut Faked) {
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::StarfieldReset));
    view.catch_up();
}

/// Whether every star's factor is a `Rand(35)` roll × 0.01, some above 0.
fn rolled(view: &Faked) -> bool {
    let stars = view.stars.stars();
    stars.iter().all(|star| {
        let hundredths = (star.factor * 100.0).round();
        (0.0..35.0).contains(&hundredths) && (star.factor - hundredths * 0.01).abs() < 1e-4
    }) && stars.iter().any(|star| star.factor > 0.0)
}

/// Whether every star's factor is 0.
fn fixed(view: &Faked) -> bool {
    view.stars.stars().iter().all(|star| star.factor == 0.0)
}

#[test]
fn the_parallax_starfield_is_on_by_default_and_each_scattering_rolls_each_stars_factor() {
    let mut view = starry_world();
    assert!(view.parallax_starfield(), "on by default");
    assert!(rolled(&view));
    scattered_on_arrival(&mut view);
    assert!(rolled(&view));
}

#[test]
fn with_the_parallax_starfield_off_every_factor_is_0() {
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(super::stars::Counting::default()));
    let mut view = world(starry(), sol())
        .with_effects_chance(SharedChance::new(chance))
        .with_parallax_starfield(false);
    assert!(!view.parallax_starfield());
    view.catch_up();
    assert!(view.stars.stars().iter().all(|star| star.shown));
    assert!(fixed(&view));
    scattered_on_arrival(&mut view);
    assert!(fixed(&view));
}

#[test]
fn a_parallax_starfield_change_waits_for_the_next_scattering() {
    let mut view = starry_world();
    let before = view.stars.stars();
    view.set_parallax_starfield(false);
    assert_eq!(
        view.stars.stars(),
        before,
        "the stars scattered keep theirs"
    );
    scattered_on_arrival(&mut view);
    assert!(fixed(&view));
    view.set_parallax_starfield(true);
    assert!(fixed(&view), "until the next scattering");
    scattered_on_arrival(&mut view);
    assert!(rolled(&view), "back on, the next scattering rolls");
}

// The arrival's line, over the flight's names and date
// (rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks). Which notes the
// flight raises, the buoy's string and their ticks are nova-sim's.

/// [`warping`]'s view at the warp's end, the flight's events of its
/// arrival in Alpha Centauri `events` drained after the system's change.
fn arriving_with(events: Vec<FlightEvent>) -> Faked {
    let mut view = warping();
    at_the_warps_end(&mut view, Vec::new());
    for event in events {
        view.sim.queue_event(event);
    }
    view
}

/// The arrival line in Alpha Centauri on the flight's date.
fn alpha_arrival() -> String {
    arrival_message("Alpha Centauri", DATE, false)
}

#[test]
fn a_jump_into_a_buoyed_system_shows_the_buoy_for_480_ticks_in_place_of_the_arrival() {
    let buoy = SimMessage::Buoy(HELLO.to_owned());
    let mut view = arriving_with(vec![FlightEvent::Message(buoy.clone())]);
    view.arrival_frame();
    assert_eq!(buoy.ticks(), 480);
    faked_lasts(&mut view, HELLO, buoy.ticks());
}

#[test]
fn an_empty_buoy_replaces_the_line_shown_before_the_jump() {
    let mut view = warping();
    view.say_for("Before the jump.", 10_000);
    at_the_warps_end(&mut view, Vec::new());
    view.sim
        .queue_event(FlightEvent::Message(SimMessage::Buoy(String::new())));
    view.arrival_frame();
    assert_eq!(view.message(), Some(""));
}

#[test]
fn fighters_abandoned_in_a_jump_are_told_on_arrival() {
    let mut view = arriving_with(vec![FlightEvent::FighterNote(
        nova_sim::FighterNote::Abandoned(2),
    )]);
    view.arrival_frame();
    let said = format!("{}  {}", alpha_arrival(), fighters_abandoned_message(2));
    assert_eq!(view.message(), Some(said.as_str()));
}

#[test]
fn an_escort_left_unpaid_by_a_jump_defects_and_the_flight_says_so() {
    let mut view = arriving_with(vec![FlightEvent::PayNote(nova_sim::PayNote::Defected(1))]);
    view.arrival_frame();
    let said = format!("{}  {DEFECTED_ONE}", alpha_arrival());
    assert_eq!(view.message(), Some(said.as_str()), "after the arrival");
}

#[test]
fn fighters_abandoned_and_escorts_defected_on_one_arrival_are_said_together() {
    let mut view = arriving_with(vec![
        FlightEvent::PayNote(nova_sim::PayNote::Defected(1)),
        FlightEvent::FighterNote(nova_sim::FighterNote::Abandoned(2)),
    ]);
    view.arrival_frame();
    let said = format!(
        "{}  {}  {DEFECTED_ONE}",
        alpha_arrival(),
        fighters_abandoned_message(2)
    );
    assert_eq!(view.message(), Some(said.as_str()), "the fighters first");
}

// The tutorial's hints, relayed from a frame's steps.

/// A view over a flight in Sol showing the Moon (129), the steps'
/// `message` drained after them and relayed.
fn relaying(message: SimMessage) -> Faked {
    let mut view = faked(FlightFrame {
        stellars: vec![record::stellar(StellarId(129))],
        ..in_sol()
    });
    view.sim.queue_event(FlightEvent::Message(message));
    view.relay_messages();
    view
}

#[test]
fn the_land_hint_names_the_stellar_and_l() {
    let hint = SimMessage::Tutorial(nova_sim::TutorialHint::Land {
        stellar: StellarId(129),
        station: false,
    });
    let mut view = relaying(hint.clone());
    assert_eq!(hint.ticks(), 512);
    faked_lasts(&mut view, &land_hint(false, "Moon", "L"), hint.ticks());
}

#[test]
fn the_hyperspace_hint_names_m_and_j() {
    let hint = "Now you should probably hyperspace to another system. Hit 'M' to access the map, select a nearby system, then move outward and hit 'J' to begin your jump.";
    assert_eq!(hyperspace_hint("M", "J"), hint);
    let message = SimMessage::Tutorial(nova_sim::TutorialHint::Hyperspace);
    let mut view = relaying(message.clone());
    faked_lasts(&mut view, hint, message.ticks());
}

#[test]
fn far_enough_and_nothing_out_here_last_512() {
    let far = SimMessage::Tutorial(nova_sim::TutorialHint::FarEnough);
    let mut view = relaying(far.clone());
    assert_eq!(view.message(), Some(FAR_ENOUGH));
    let nothing = SimMessage::Tutorial(nova_sim::TutorialHint::NothingOutHere);
    view.sim.queue_event(FlightEvent::Message(nothing.clone()));
    view.relay_messages();
    assert_eq!(nothing.ticks(), 512);
    faked_lasts(&mut view, NOTHING_OUT_HERE, nothing.ticks());
}

// The jump's white as a faded screen (rdm:phase/movies/phase-4-desc-movies-outside-missions).

#[test]
fn the_lit_warp_leaves_the_screen_faded_until_a_text_fades_it_in_then_the_next_light_fades_again() {
    let mut view = warping();
    assert!(!view.screen_faded(), "unlit");
    lit(&mut view, 0.5);
    assert!(view.screen_faded());
    view.advance_white(FADE_OUT_FOR / 2);
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeOut(0.5)),
        "the white runs on under a dialog (`CGDisplayFade`, asynchronous)"
    );
    view.screen_faded_in();
    assert!(!view.screen_faded());
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Warp),
        "the white is gone"
    );
    lit(&mut view, 0.5);
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeOut(0.0)),
        "relit on the next pass, 1.5 s again (@0x683f1-0x68404)"
    );
}

#[test]
fn a_text_on_the_arrivals_tick_finds_the_white_and_its_fade_in_ends_the_effect() {
    let mut view = warping();
    lit(&mut view, 0.5);
    at_the_warps_end(&mut view, Vec::new());
    view.arrival_frame();
    assert!(view.screen_faded(), "until the next pass's _FadeWhiteOut");
    view.screen_faded_in();
    assert_eq!(view.jump_effect(), None, "nothing left to fade from white");

    let mut view = warping();
    lit(&mut view, 0.5);
    at_the_warps_end(&mut view, Vec::new());
    view.arrival_frame();
    view.fade_frame(TICK);
    assert!(!view.screen_faded(), "the next pass cleared it");
    view.advance_white(FADE_IN_FOR);
    assert!(
        view.jump_effect().is_some(),
        "the arrival's fade-in is flight's own to run"
    );
}
