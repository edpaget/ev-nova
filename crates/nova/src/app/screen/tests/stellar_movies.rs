//! The router over flight's stellar movies
//! (rdm:phase/movies/phase-3-stellar-desc-movies): a landing at a stellar
//! whose `dësc` names a movie shows the movie, then the spaceport; with
//! movies off it goes straight to the spaceport; the movie holds
//! flight's input, Escape among it; a key pressed under it reaches the
//! spaceport once it runs out; and a cinematic one hides the pointer once
//! its fade-out is over. The gate's counterpart, a flight key
//! reaching flight after a hypergate's come-out, is in the `nova` crate's
//! `tests/it/hyperspace.rs`, beside the hypergate fixture.

use super::*;

/// The router over Alpha Prime (128), whose `dësc` names "Race 1.mov"
/// with `Flags` `flags`, with `prefs`, movies played through a new [`MockMoviePlayer`], and
/// Ada flying.
fn filmed_port(flags: u16, prefs: Prefs) -> (AppScreen, Rc<MockMoviePlayer>) {
    let fork = game_data_fork(true, true, true, 100, b"", b"", None).resource(
        Desc::TYPE,
        128,
        Some(b"Alpha Prime"),
        &filmed_desc_of("A port.", "Race 1.mov", flags),
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
    let (mut screen, reel) = filmed_port(0, Prefs::default());
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
    let (mut screen, reel) = filmed_port(
        0,
        Prefs {
            quicktime_movies: false,
            ..Prefs::default()
        },
    );
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert!(reeled(&reel).is_empty());
}

#[test]
fn escape_during_the_landings_movie_leaves_neither_flight_nor_the_movie() {
    let (mut screen, _) = filmed_port(0, Prefs::default());
    land_twice(&mut screen);
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::Movie);
    screen.input(&key(Key::Space, true));
    assert_eq!(screen.showing(), Showing::Spaceport, "Space skips it");
}

#[test]
fn a_key_pressed_under_the_landings_movie_reaches_the_spaceport_once_it_runs_out() {
    let (mut screen, _) = filmed_port(0, Prefs::default());
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

#[test]
fn a_cinematic_landing_movie_hides_the_pointer_once_its_fade_out_is_over() {
    let (mut screen, _) = filmed_port(0x0004, Prefs::default());
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Movie);
    assert!(!screen.hides_pointer(), "during the fade out");
    screen.tick(nova_view::movie::MOVIE_FADE_FOR);
    assert_eq!(screen.showing(), Showing::Movie, "still playing");
    assert!(screen.hides_pointer(), "_HideTheCursor (@0x55a73)");
}

#[test]
fn a_cinematic_landing_movie_over_widened_flight_covers_the_whole_view() {
    use super::widescreen::{WIDE, black_across};
    let (mut screen, _) = filmed_port(0x0004, Prefs::default());
    screen.set_widened_size(WIDE);
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Movie);
    screen.tick(nova_view::movie::MOVIE_FADE_FOR);
    screen.tick(nova_view::movie::MOVIE_FADE_FOR);
    assert_eq!(screen.showing(), Showing::Movie, "still playing");
    assert!(
        black_across(&screen, WIDE.0).contains(&255),
        "the black window over the whole widened view"
    );
}
