//! The flight's `dësc` movies' component tests on [`FakeFlight`] (see the
//! `movies` module): a landing's, a hypergate's and a wormhole's movie,
//! what is loaded and asked for them, what waits behind them, and what
//! movies off leaves out. Kept apart from `tests/mod.rs`, which is large.

use super::faked_landing::{arrived, at_gate, came_out_in_alpha};
use super::*;
use crate::Color;
use crate::draw::fill_rect;
use crate::flight::jump::{JumpEffect, JumpPhase};
use crate::movie::fixture::{Call, MockMoviePlayer};
use crate::movie::{MOVIE_FADE_FOR, Movies};
use crate::view_shape::classic_offset;
use nova_sim::flight_frame::{FlightEvent, GateEntry};
use nova_sim::landing::LandOutcome;
use nova_sim::{DescMovie, GateKind, LandPress};

/// How many advances each movie plays for.
const ADVANCES: u32 = 3;

/// The movie each filmed `dësc` names: plain, `Flags` 0x0001.
fn film() -> DescMovie {
    DescMovie::from_desc("Port.mov", 0x0001).expect("named")
}

/// The catalog's `dësc`s `ids`, each naming [`film`], and `dësc`s
/// `plain`, naming none.
fn filmed(ids: &[i16], plain: &[i16]) -> FakeCatalog {
    FakeCatalog {
        descriptions: ids
            .iter()
            .chain(plain)
            .map(|&id| (id, format!("dësc {id}")))
            .collect(),
        desc_movies: ids.iter().map(|&id| (id, film())).collect(),
        ..catalog()
    }
}

/// The load of filmed `dësc` `id` ([`filmed`]).
fn load(id: i16) -> FlightCommand {
    FlightCommand::LoadDescription {
        raw: Some(format!("dësc {id}")),
        movie: Some(film()),
    }
}

/// A view in Sol over `catalog`, its movies played through a mock player
/// while `on`.
fn filming(catalog: FakeCatalog, on: bool) -> (Faked, Rc<MockMoviePlayer>) {
    let player = MockMoviePlayer::playing((100, 80), ADVANCES);
    let view = faked_in(catalog, sol()).with_movies(Movies::new(player.clone(), on));
    (view, player)
}

/// Lands on Mars (129), the press's beep raised with it.
fn land_on_mars(view: &mut Faked) {
    view.sim
        .queue_event(FlightEvent::Sound(nova_sim::SimSound::Landed));
    view.sim
        .queue_answer(Answer::LandPress(LandPress::Outcome(LandOutcome::Landed(
            StellarId(129),
        ))));
    view.land();
}

/// Ticks `view` until its movie has run out: [`ADVANCES`] ticks.
fn run_out(view: &mut Faked) {
    for _ in 0..ADVANCES {
        assert!(view.playing(), "still playing");
        view.tick(TICK);
    }
    assert!(!view.playing(), "ended");
}

/// Whether `view` has sent `command`.
fn has_sent(view: &Faked, command: &FlightCommand) -> bool {
    sent(view).contains(command)
}

// The landing.

#[test]
fn a_landing_plays_its_desc_movie_before_it_lands_beeps_and_redraws_the_bar() {
    let (mut view, player) = filming(filmed(&[129], &[]), true);
    land_on_mars(&mut view);
    assert_eq!(view.sim.asks(), [load(129)], "the stellar's own dësc");
    assert_eq!(player.calls(), [Call::Start("Port.mov".to_owned())]);
    assert!(view.playing());
    assert_eq!(view.take_landing(), None, "held behind the movie");
    assert_eq!(
        view.take_sounds(),
        [],
        "the beep comes after the movie (@0x632ef)"
    );
    assert_eq!(view.reserves_scene, None, "the bar as the press found it");
    run_out(&mut view);
    assert_eq!(view.sim.steps(), 0, "flight frozen under it");
    assert_eq!(view.take_landing(), Some(StellarId(129)));
    assert_eq!(view.take_sounds(), [Sound::Sim(nova_sim::SimSound::Landed)]);
    assert!(view.reserves_scene.is_some(), "redrawn (@0x632fb)");
}

#[test]
fn a_landing_movie_holds_back_only_the_beep_the_voice_kill_comes_out_on_the_press() {
    let (mut view, _player) = filming(filmed(&[129], &[]), true);
    let voice = nova_sim::SoundId(1010);
    view.sim
        .queue_event(FlightEvent::Sound(nova_sim::SimSound::Voice {
            sound: voice,
        }));
    view.tick(TICK);
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(nova_sim::SimSound::Voice { sound: voice })],
        "a voice playing at the landing"
    );
    for sound in [
        nova_sim::SimSound::HaltVoice { sound: voice },
        nova_sim::SimSound::ThrustStopped,
    ] {
        view.sim.queue_event(FlightEvent::Sound(sound));
    }
    land_on_mars(&mut view);
    assert!(view.playing());
    assert_eq!(
        view.take_sounds(),
        [
            Sound::Sim(nova_sim::SimSound::HaltVoice { sound: voice }),
            Sound::Sim(nova_sim::SimSound::ThrustStopped),
        ],
        "_KillVoiceSound (@0x66daf) comes before the movie (@0x632a7)"
    );
    assert_eq!(view.take_sounds(), [], "the beep still held");
    run_out(&mut view);
    assert_eq!(view.take_sounds(), [Sound::Sim(nova_sim::SimSound::Landed)]);
}

#[test]
fn a_landing_at_a_desc_naming_no_movie_or_none_at_all_loads_it_and_lands_at_once() {
    for (catalog, raw) in [
        (filmed(&[], &[129]), Some("dësc 129".to_owned())),
        (filmed(&[], &[]), None),
    ] {
        let (mut view, player) = filming(catalog, true);
        land_on_mars(&mut view);
        assert_eq!(
            view.sim.asks(),
            [FlightCommand::LoadDescription { raw, movie: None }],
            "a missing one loaded as missing (@0x71a5e)"
        );
        assert_eq!(player.calls(), [], "MovieFile empty: no movie (@0x55f39)");
        assert!(!view.playing());
        assert_eq!(view.take_landing(), Some(StellarId(129)));
        assert_eq!(view.take_sounds(), [Sound::Sim(nova_sim::SimSound::Landed)]);
    }
}

#[test]
fn with_movies_off_a_landing_loads_nothing_and_lands_at_once() {
    let (mut view, player) = filming(filmed(&[129], &[]), false);
    land_on_mars(&mut view);
    assert_eq!(view.sim.asks(), [], "nothing loaded (@0x6328a)");
    assert_eq!(player.calls(), []);
    assert_eq!(view.take_landing(), Some(StellarId(129)));
}

#[test]
fn a_movie_that_cannot_start_lands_at_once() {
    let player = MockMoviePlayer::refusing();
    let mut view =
        faked_in(filmed(&[129], &[]), sol()).with_movies(Movies::new(player.clone(), true));
    land_on_mars(&mut view);
    assert_eq!(view.sim.asks(), [load(129)]);
    assert!(!view.playing());
    assert_eq!(view.take_landing(), Some(StellarId(129)));
}

#[test]
fn under_the_landing_movie_the_play_area_is_bare_and_the_movie_is_centred_in_a_widened_view() {
    let (mut view, player) = filming(filmed(&[129], &[]), true);
    view.set_view_size((1366.0, 768.0));
    land_on_mars(&mut view);
    let mut list = DrawList::new();
    view.draw(&mut list);
    let mut bare = DrawList::new();
    fill_rect(&mut bare, view.play_rect(), Color::BLACK);
    assert_eq!(
        list.iter().next(),
        bare.iter().next(),
        "every sprite hidden, the stars too (_SWHideAllSprites @0xb8c09)"
    );
    assert!(
        matches!(list.iter().last(), Some(DrawCommand::MovieFrame { .. })),
        "the movie over it"
    );
    let drawn = player.drawn();
    let offset = classic_offset(1366.0);
    assert_eq!(drawn.len(), 1);
    let frames: Vec<_> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::MovieFrame { area, .. } => Some(*area),
            _ => None,
        })
        .collect();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].min.x, drawn[0].min.x + offset, "centred");
}

#[test]
fn while_a_movie_plays_flight_takes_no_key_and_steps_not() {
    let (mut view, _) = filming(filmed(&[129], &[]), true);
    land_on_mars(&mut view);
    let sent_before = sent(&view).len();
    view.input(&key(LAND_KEY, true));
    view.input(&key(LAND_KEY, false));
    view.tick(TICK);
    assert_eq!(sent(&view).len(), sent_before, "nothing sent");
    assert_eq!(view.sim.steps(), 0, "no step");
    assert!(view.playing());
}

#[test]
fn a_skip_ends_the_landing_movie_and_what_else_was_pressed_waits_for_the_spaceport() {
    let (mut view, player) = filming(filmed(&[129], &[]), true);
    land_on_mars(&mut view);
    view.input(&key(Key::Char('b'), true));
    assert!(view.playing());
    view.input(&key(Key::Space, true));
    assert!(!view.playing(), "skipped");
    assert_eq!(player.calls().last(), Some(&Call::Stop));
    assert_eq!(view.take_landing(), Some(StellarId(129)));
    assert_eq!(view.take_movie_replay(), [], "a skip flushes the queue");
    let (mut view, _) = filming(filmed(&[129], &[]), true);
    land_on_mars(&mut view);
    view.input(&key(Key::Char('b'), true));
    run_out(&mut view);
    assert_eq!(view.take_movie_replay(), [key(Key::Char('b'), true)]);
    assert_eq!(view.take_movie_replay(), [], "taken");
}

#[test]
fn a_cinematic_movie_hides_the_pointer_once_its_fade_out_is_over() {
    let cinematic = DescMovie::from_desc("Port.mov", 0x0004).expect("named");
    let catalog = FakeCatalog {
        desc_movies: vec![(129, cinematic)],
        ..filmed(&[], &[129])
    };
    let (mut view, _) = filming(catalog, true);
    land_on_mars(&mut view);
    assert!(!view.hides_pointer(), "fading out");
    view.tick(MOVIE_FADE_FOR);
    assert!(view.movie_hides_pointer());
    assert!(view.hides_pointer());
}

// The hypergate.

/// [`filming`] over `catalog` with L pressed over Sol's cleared
/// hypergate (300), its map open offering Alpha Centauri (131) and
/// Barnard (132), Alpha Centauri picked.
fn at_the_filmed_hypergate(catalog: FakeCatalog, on: bool) -> (Faked, Rc<MockMoviePlayer>) {
    let (mut view, player) = filming(catalog, on);
    let opening = map_opening(&view.frame, Some(130));
    view.sim.queue_screen(opening);
    view.sim.queue_answer(at_gate(GateKind::Hypergate));
    view.sim
        .queue_answer(Answer::Offered(vec![SystemId(131), SystemId(132)]));
    view.land();
    view.map.select_gate(Some(SystemId(131)));
    (view, player)
}

/// The hypergate's entry for Alpha Centauri.
const ENTER_ALPHA: FlightCommand = FlightCommand::EnterHypergate(Some(SystemId(131)));

#[test]
fn a_hypergate_entry_loads_the_desc_the_flight_names_and_plays_its_movie_before_entering() {
    let (mut view, player) = at_the_filmed_hypergate(filmed(&[131], &[]), true);
    view.sim
        .queue_screen(ScreenAnswer::GateEntryDesc(Some(131)));
    view.close_map();
    assert_eq!(
        view.sim.screens_asked().last(),
        Some(&ScreenAsk::GateEntryDesc(GateEntry::Hypergate(Some(
            SystemId(131)
        ))))
    );
    assert_eq!(
        view.sim.asks(),
        [load(131)],
        "the picked system's dësc, as the flight names it"
    );
    assert!(!has_sent(&view, &ENTER_ALPHA), "held behind the movie");
    assert_eq!(player.calls(), [Call::Start("Port.mov".to_owned())]);
    came_out_in_alpha(&mut view, SimMessage::ExitedHypergate(SystemId(131)));
    view.sim.queue_answer(arrived());
    run_out(&mut view);
    assert_eq!(sent_but_lines_last(&view), Some(ENTER_ALPHA));
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeIn(0.0)),
        "the come-out's white fade, after the movie"
    );
}

/// The last command `view` sent but the message line's.
fn sent_but_lines_last(view: &Faked) -> Option<FlightCommand> {
    super::faked_travel::sent_but_lines(view).last().cloned()
}

#[test]
fn a_skip_ends_the_hypergates_movie_and_enters_it() {
    let (mut view, _) = at_the_filmed_hypergate(filmed(&[131], &[]), true);
    view.sim
        .queue_screen(ScreenAnswer::GateEntryDesc(Some(131)));
    view.close_map();
    came_out_in_alpha(&mut view, SimMessage::ExitedHypergate(SystemId(131)));
    view.sim.queue_answer(arrived());
    view.input(&key(Key::Enter, true));
    assert!(!view.playing());
    assert_eq!(sent_but_lines_last(&view), Some(ENTER_ALPHA));
}

#[test]
fn a_hypergate_entry_the_flight_would_refuse_loads_nothing_and_enters_at_once() {
    let (mut view, player) = at_the_filmed_hypergate(filmed(&[131], &[]), true);
    view.sim.queue_screen(ScreenAnswer::GateEntryDesc(None));
    view.sim.queue_answer(arrived());
    view.close_map();
    assert_eq!(view.sim.asks(), [], "no movie past a refusal (@0x6392c)");
    assert_eq!(player.calls(), []);
    assert!(!view.playing());
    assert_eq!(sent_but_lines_last(&view), Some(ENTER_ALPHA));
}

#[test]
fn with_movies_off_a_hypergate_entry_asks_and_loads_nothing() {
    let (mut view, player) = at_the_filmed_hypergate(filmed(&[131], &[]), false);
    view.sim.queue_answer(arrived());
    view.close_map();
    assert!(
        !view
            .sim
            .screens_asked()
            .iter()
            .any(|ask| matches!(ask, ScreenAsk::GateEntryDesc(_))),
        "nothing asked (@0x639bb)"
    );
    assert_eq!(view.sim.asks(), []);
    assert_eq!(player.calls(), []);
    assert_eq!(sent_but_lines_last(&view), Some(ENTER_ALPHA));
}

#[test]
fn under_a_gates_movie_the_old_systems_flight_is_drawn_frozen() {
    let (mut view, _) = at_the_filmed_hypergate(filmed(&[131], &[]), true);
    view.sim
        .queue_screen(ScreenAnswer::GateEntryDesc(Some(131)));
    view.close_map();
    let mut list = DrawList::new();
    view.draw(&mut list);
    let mut flight = DrawList::new();
    assert!(view.draw_flight(&mut flight, String::new));
    let flight: Vec<_> = flight.iter().cloned().collect();
    let shown: Vec<_> = list.iter().cloned().collect();
    assert_eq!(shown[..flight.len()], flight[..], "the flight under it");
    assert!(matches!(shown.last(), Some(DrawCommand::MovieFrame { .. })));
}

// The wormhole.

/// L pressed over Sol's cleared wormhole (300) on `view`, the flight
/// answering `desc` for its entry's `dësc` when movies are on, and the
/// entry, when it is sent, `entry`.
fn into_the_wormhole(view: &mut Faked, desc: Option<ScreenAnswer>, entry: Answer) {
    if let Some(desc) = desc {
        view.sim.queue_screen(desc);
    }
    view.sim.queue_answer(at_gate(GateKind::Wormhole));
    view.sim.queue_answer(entry);
    view.land();
}

#[test]
fn a_wormhole_entry_plays_its_own_desc_movie_before_passing_through() {
    let (mut view, player) = filming(filmed(&[300], &[]), true);
    into_the_wormhole(
        &mut view,
        Some(ScreenAnswer::GateEntryDesc(Some(300))),
        arrived(),
    );
    assert_eq!(
        view.sim.screens_asked().last(),
        Some(&ScreenAsk::GateEntryDesc(GateEntry::Wormhole))
    );
    assert_eq!(view.sim.asks(), [load(300)]);
    assert!(!has_sent(&view, &FlightCommand::EnterWormhole));
    assert_eq!(player.calls(), [Call::Start("Port.mov".to_owned())]);
    came_out_in_alpha(&mut view, SimMessage::PassedWormhole(SystemId(131)));
    run_out(&mut view);
    assert_eq!(
        sent_but_lines_last(&view),
        Some(FlightCommand::EnterWormhole)
    );
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Flash(0.0))
    );
}

#[test]
fn a_wormhole_leading_nowhere_loads_nothing_and_is_refused_at_once() {
    let (mut view, player) = filming(filmed(&[300], &[]), true);
    into_the_wormhole(
        &mut view,
        Some(ScreenAnswer::GateEntryDesc(None)),
        Answer::GateRefused(nova_sim::GateRefusal::NoExit),
    );
    assert_eq!(view.sim.asks(), [], "no movie past the refusal (@0x6461c)");
    assert_eq!(player.calls(), []);
    assert_eq!(
        sent_but_lines_last(&view),
        Some(FlightCommand::EnterWormhole)
    );
}

#[test]
fn with_movies_off_a_wormhole_entry_asks_and_loads_nothing() {
    let (mut view, player) = filming(filmed(&[300], &[]), false);
    into_the_wormhole(&mut view, None, arrived());
    assert!(
        view.sim.screens_asked().is_empty(),
        "nothing asked (@0x64260)"
    );
    assert_eq!(view.sim.asks(), []);
    assert_eq!(player.calls(), []);
    assert_eq!(
        sent_but_lines_last(&view),
        Some(FlightCommand::EnterWormhole)
    );
}
