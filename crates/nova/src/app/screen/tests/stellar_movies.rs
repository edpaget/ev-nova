//! The router over flight's stellar movies
//! (rdm:phase/movies/phase-3-stellar-desc-movies): a landing at a stellar
//! whose `dësc` names a movie shows the movie, then the spaceport; with
//! movies off it goes straight to the spaceport; the movie holds
//! flight's input, Escape among it; and a key pressed under it reaches the
//! spaceport once it runs out. The gate's counterpart, a flight key
//! reaching flight after a hypergate's come-out, is in the `nova` crate's
//! `tests/it/hyperspace.rs`, beside the hypergate fixture.

use super::*;

/// The router over Alpha Prime (128), whose `dësc` names "Race 1.mov",
/// with `prefs`, movies played through a new [`MockMoviePlayer`], and
/// Ada flying.
fn filmed_port(prefs: Prefs) -> (AppScreen, Rc<MockMoviePlayer>) {
    let fork = game_data_fork(true, true, true, 100, b"", b"", None).resource(
        Desc::TYPE,
        128,
        Some(b"Alpha Prime"),
        &filmed_desc_of("A port.", "Race 1.mov", 0),
    );
    let reel = MockMoviePlayer::playing((160, 120), 2);
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(loaded(&fork))
        .with_prefs(prefs)
        .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    (screen, reel)
}

#[test]
fn a_landing_with_movies_on_shows_the_stellars_movie_then_the_spaceport() {
    let (mut screen, reel) = filmed_port(Prefs::default());
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Movie);
    assert_eq!(reeled(&reel), [Call::Start("Race 1.mov".into())]);
    for _ in 0..2 {
        screen.tick(Duration::from_millis(16));
    }
    assert_eq!(
        reeled(&reel),
        [Call::Start("Race 1.mov".into()), Call::Stop]
    );
    assert_eq!(screen.showing(), Showing::Spaceport, "on the tick it ends");
}

#[test]
fn a_landing_with_movies_off_goes_straight_to_the_spaceport() {
    let (mut screen, reel) = filmed_port(Prefs {
        quicktime_movies: false,
        ..Prefs::default()
    });
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert!(reeled(&reel).is_empty());
}

#[test]
fn escape_during_the_landings_movie_leaves_neither_flight_nor_the_movie() {
    let (mut screen, _) = filmed_port(Prefs::default());
    land_twice(&mut screen);
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::Movie);
    screen.input(&key(Key::Space, true));
    assert_eq!(screen.showing(), Showing::Spaceport, "Space skips it");
}

#[test]
fn a_key_pressed_under_the_landings_movie_reaches_the_spaceport_once_it_runs_out() {
    let (mut screen, _) = filmed_port(Prefs::default());
    land_twice(&mut screen);
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::Movie, "queued, not acted on");
    for _ in 0..2 {
        screen.tick(Duration::from_millis(16));
    }
    assert_eq!(
        screen.showing(),
        Showing::Flight,
        "the spaceport is shown, then its Escape leaves it"
    );
    assert_eq!(
        flight(&screen).session().expect("flying").landed(),
        None,
        "taken off"
    );
}
