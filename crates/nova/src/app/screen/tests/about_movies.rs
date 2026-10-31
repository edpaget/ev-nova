//! The router over the About text's and the X text's own movies
//! (rdm:phase/movies/phase-4-desc-movies-outside-missions): `_DoAboutBox`
//! and `_MainScreenKey`'s X load their own `dësc` and show it with
//! `_ShowTextDialog(0, 0, 1)` (@0x16062, @0x1acde), so its movie plays
//! before the text (`Flags` 0x0001 clear) or after it (set), under either
//! reading of `desc_movie_globals`, and a `Graphic` above 127 shows in
//! the picture variant.

use super::*;
use nova_sim::{RuleKey, RuleSource, Rulebook};

/// The router over a fork whose About text (32767) and X text (32766)
/// are `about` and `x`, with dialogs, under `rules`, movies played
/// through a new [`MockMoviePlayer`], on the main menu.
fn texts(about: &[u8], x: &[u8], rules: Rulebook) -> (AppScreen, Rc<MockMoviePlayer>) {
    texts_with(about, x, rules, Prefs::default())
}

/// [`texts`], with `prefs`.
fn texts_with(
    about: &[u8],
    x: &[u8],
    rules: Rulebook,
    prefs: Prefs,
) -> (AppScreen, Rc<MockMoviePlayer>) {
    let fork = ForkBuilder::new()
        .resource(Desc::TYPE, ABOUT_TEXT, None, about)
        .resource(Desc::TYPE, MENU_X_TEXT, None, x);
    let reel = MockMoviePlayer::playing((160, 120), 2);
    let store = MemoryPilots::new();
    let mut screen = with_dialogs(loaded(&fork))
        .with_prefs(prefs)
        .with_rules(rules)
        .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    screen.take_warnings();
    (screen, reel)
}

/// [`texts`] with the About text "About." naming "about.mov" of
/// `Flags` `flags`, and a plain X text.
fn filmed_about(flags: u16, rules: Rulebook) -> (AppScreen, Rc<MockMoviePlayer>) {
    texts(
        &filmed_desc_of("About.", "about.mov", flags),
        &desc_of("X."),
        rules,
    )
}

fn about_words(screen: &AppScreen) -> Option<String> {
    screen.about().map(|about| about.lines().join(" "))
}

fn tick_out(screen: &mut AppScreen) {
    for _ in 0..2 {
        screen.tick(Duration::from_millis(16));
    }
}

#[test]
fn the_about_texts_movie_plays_before_it_then_the_text_shows() {
    let (mut screen, reel) = filmed_about(0, Rulebook::default());
    open_about(&mut screen);
    assert_eq!(reeled(&reel), [Call::Start("about.mov".into())]);
    assert_eq!(about_words(&screen), None, "not while the movie plays");
    tick_out(&mut screen);
    assert_eq!(about_words(&screen).as_deref(), Some("About."));
    assert_eq!(screen.showing(), Showing::About);
    screen.input(&key(Key::Enter, true));
    assert!(screen.about().is_none());
    assert_eq!(screen.showing(), Showing::MainMenu);
}

#[test]
fn a_skip_of_the_about_texts_movie_shows_the_text() {
    let (mut screen, _) = filmed_about(0, Rulebook::default());
    open_about(&mut screen);
    screen.input(&key(Key::Space, true));
    assert_eq!(about_words(&screen).as_deref(), Some("About."));
}

#[test]
fn the_about_texts_after_movie_plays_once_done_closes_it() {
    let (mut screen, reel) = filmed_about(0x0001, Rulebook::default());
    open_about(&mut screen);
    assert_eq!(about_words(&screen).as_deref(), Some("About."));
    assert_eq!(reeled(&reel), [], "not before it");
    screen.input(&key(Key::Enter, true));
    assert_eq!(reeled(&reel), [Call::Start("about.mov".into())]);
    assert_eq!(screen.showing(), Showing::About, "open while it plays");
    tick_out(&mut screen);
    assert_eq!(screen.showing(), Showing::MainMenu, "closed as it ends");
}

#[test]
fn the_x_texts_movie_plays_as_the_about_texts_does() {
    for (flags, before) in [(0, true), (0x0001, false)] {
        let (mut screen, reel) = texts(
            &desc_of("About."),
            &filmed_desc_of("X.", "x.mov", flags),
            Rulebook::default(),
        );
        screen.input(&key(Key::Char('x'), true));
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), Showing::Note, "{flags:#x}");
        if before {
            assert_eq!(reeled(&reel), [Call::Start("x.mov".into())]);
            tick_out(&mut screen);
        }
        assert_eq!(about_words(&screen).as_deref(), Some("X."), "{flags:#x}");
        screen.input(&key(Key::Char('x'), false));
        screen.input(&key(Key::Enter, true));
        if !before {
            assert_eq!(reeled(&reel), [Call::Start("x.mov".into())]);
            tick_out(&mut screen);
        }
        assert_eq!(screen.showing(), Showing::MainMenu, "{flags:#x}");
    }
}

#[test]
fn the_own_movies_play_under_either_reading_of_the_desc_globals() {
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let rules = Rulebook::default().with_override(RuleKey::DescMovieGlobals, source);
        let (mut screen, reel) = filmed_about(0, rules);
        open_about(&mut screen);
        assert_eq!(
            reeled(&reel),
            [Call::Start("about.mov".into())],
            "{source:?}"
        );
    }
}

#[test]
fn an_about_text_with_a_graphic_above_127_shows_in_the_picture_variant() {
    let (mut screen, _) = texts(
        &pictured_desc_of("About.", 5005),
        &desc_of("X."),
        Rulebook::default(),
    );
    open_about(&mut screen);
    let about = screen.about().expect("open");
    assert_eq!(about.picture(), Some(5005), "DLOG 3004 (@0x5a89a-0x5a8cd)");
    assert!((about.dialog().bounds().width() - 300.0).abs() < f32::EPSILON);
}

#[test]
fn with_movies_off_the_about_text_shows_at_once() {
    let (mut screen, reel) = texts_with(
        &filmed_desc_of("About.", "about.mov", 0),
        &desc_of("X."),
        Rulebook::default(),
        Prefs {
            quicktime_movies: false,
            ..Prefs::default()
        },
    );
    open_about(&mut screen);
    assert_eq!(about_words(&screen).as_deref(), Some("About."));
    assert_eq!(reeled(&reel), []);
}

/// [`game_data_fork`]'s data, its About text naming "about.mov" as its
/// after-movie.
fn filmed_about_in_play() -> Rc<GameData> {
    let mut built = game_data_fork(false, false, false, 100, b"", b"", None).build();
    let desc = built.layout.data_entry(Desc::TYPE, ABOUT_TEXT) + 4;
    let text = about_text()
        .iter()
        .position(|&byte| byte == 0)
        .expect("a C string");
    let at = desc + text + 3;
    built.bytes[at..at + 9].copy_from_slice(b"about.mov");
    built.put_u16(at + 32, 0x0001);
    loaded_bytes(built.bytes)
}

#[test]
fn about_over_flight_loads_through_flights_session_as_the_last_desc() {
    let reel = MockMoviePlayer::playing((160, 120), 2);
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(filmed_about_in_play())
        .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(flight(&screen).sim().loaded_after_movie(), None);
    screen.menu_command(MenuCommand::About);
    assert_eq!(screen.showing(), Showing::About, "from the menu bar");
    assert_eq!(
        about_words(&screen).map(|words| words.contains("Ada")),
        Some(true),
        "munged for the pilot in play (@0x16035)"
    );
    assert_eq!(
        flight(&screen)
            .sim()
            .loaded_after_movie()
            .map(|movie| movie.file),
        Some("about.mov".to_owned()),
        "_DoAboutBox's load overwrites the globals (@0x16021)"
    );
    screen.input(&key(Key::Enter, true));
    assert_eq!(reeled(&reel), [Call::Start("about.mov".into())]);
}
