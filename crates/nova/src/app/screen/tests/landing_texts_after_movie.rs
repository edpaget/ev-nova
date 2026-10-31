//! The router over a landing's movie and its land checks' texts
//! (rdm:phase/movies/phase-9-landing-movie-before-land-texts):
//! `_PlayerLandOnStellar` plays the movie to its end (@0x632a7) before
//! `_DoPortDialog` (@0x63381), which shows the spaceport (@0x5faff) and
//! only then the land checks' texts (`_MissionLandCheck` @0x5fbeb), each
//! text's movies inside its own dialog. Keys pressed under the movie
//! reach the first text's loop. Over the fixture of `desc_load_order`,
//! and over a port whose spaceport offers a mission
//! (`_OfferMissionFromPort` @0x5fc37), which shows once the movie ends
//! and takes the keys pressed under it.

use super::desc_load_order::{Held, landed, text_shown, through_to_the_spaceport};
use super::*;

/// Ticks `screen` through a movie, which ends on its second advance.
fn run_out(screen: &mut AppScreen) {
    for _ in 0..2 {
        screen.tick(Duration::from_millis(16));
    }
}

#[test]
fn with_movies_on_the_comp_text_waits_for_the_landings_movie() {
    let (mut screen, reel) = landed(Held::Completed, Prefs::default());
    assert_eq!(screen.showing(), Showing::Movie);
    assert_eq!(reeled(&reel), [Call::Start("Port.mov".into())]);
    assert!(screen.spaceport_view().is_none(), "no spaceport yet");
    run_out(&mut screen);
    assert_eq!(
        reeled(&reel),
        [Call::Start("Port.mov".into()), Call::Stop],
        "the landing's movie ran to its end"
    );
    assert_eq!(screen.showing(), Showing::Spaceport, "not over flight");
    assert_eq!(text_shown(&screen).as_deref(), Some("Done."));
}

#[test]
fn with_movies_on_the_comp_texts_after_movie_starts_after_the_landings_stop() {
    let (mut screen, reel) = landed(Held::Completed, Prefs::default());
    run_out(&mut screen);
    screen.input(&key(Key::Enter, true));
    assert_eq!(
        reeled(&reel),
        [
            Call::Start("Port.mov".into()),
            Call::Stop,
            Call::Start("Comp.mov".into())
        ]
    );
    through_to_the_spaceport(&mut screen);
    run_out(&mut screen);
    assert_eq!(
        reeled(&reel),
        [
            Call::Start("Port.mov".into()),
            Call::Stop,
            Call::Start("Comp.mov".into()),
            Call::Stop
        ]
    );
}

#[test]
fn with_movies_on_the_no_space_text_shows_over_the_spaceport_after_the_movie() {
    let (mut screen, reel) = landed(Held::NoSpace, Prefs::default());
    assert!(screen.spaceport_view().is_none(), "no spaceport yet");
    run_out(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport, "not over flight");
    assert!(text_shown(&screen).is_some(), "the no-space text over it");
    through_to_the_spaceport(&mut screen);
    run_out(&mut screen);
    assert_eq!(
        reeled(&reel),
        [
            Call::Start("Port.mov".into()),
            Call::Stop,
            Call::Start("Port.mov".into()),
            Call::Stop
        ],
        "the landing's movie, then the no-space text's after-movie"
    );
}

#[test]
fn a_skip_of_the_landings_movie_shows_the_text_over_the_spaceport() {
    let (mut screen, reel) = landed(Held::Completed, Prefs::default());
    screen.input(&key(Key::Space, true));
    assert_eq!(reeled(&reel), [Call::Start("Port.mov".into()), Call::Stop]);
    assert_eq!(screen.showing(), Showing::Spaceport, "not over flight");
    assert_eq!(text_shown(&screen).as_deref(), Some("Done."));
}

#[test]
fn escape_pressed_under_the_landings_movie_reaches_the_text_not_the_spaceport() {
    let (mut screen, _) = landed(Held::Completed, Prefs::default());
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::Movie, "queued, not acted on");
    run_out(&mut screen);
    assert!(
        flight(&screen)
            .session()
            .expect("flying")
            .landed()
            .is_some(),
        "the Escape went to the text, not the spaceport's leave"
    );
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert_eq!(text_shown(&screen), None, "the text closed by it");
}

/// The router over Alpha Prime (128), whose `dësc` names "Port.mov" as
/// its after-movie and whose spaceport offers mission 130 ("Job 130.",
/// as `venue_data` makes it), with movies on through a new
/// [`MockMoviePlayer`]; Ada flying, landed with two L presses, so the
/// landing's movie plays.
fn landed_offering_under_the_movie() -> (AppScreen, Rc<MockMoviePlayer>) {
    let fork = game_data_fork(true, true, true, 100, b"", b"", None)
        .resource(
            Mission::TYPE,
            130,
            Some(b"Venue job"),
            &venue_mission(SPACEPORT),
        )
        .resource(Desc::TYPE, 130 + 3872, Some(b"Offer"), &desc_of("Job 130."))
        .resource(
            Desc::TYPE,
            128,
            None,
            &filmed_desc_of("A port.", "Port.mov", 0x0001),
        );
    let reel = MockMoviePlayer::playing((160, 120), 2);
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(loaded(&fork))
        .with_prefs(Prefs::default())
        .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Movie);
    assert_eq!(reeled(&reel), [Call::Start("Port.mov".into())]);
    (screen, reel)
}

#[test]
fn with_movies_on_the_spaceport_offer_shows_once_the_landings_movie_ends() {
    let (mut screen, reel) = landed_offering_under_the_movie();
    assert!(
        screen.spaceport_view().is_none(),
        "no offer under the movie"
    );
    run_out(&mut screen);
    assert_eq!(reeled(&reel), [Call::Start("Port.mov".into()), Call::Stop]);
    assert_eq!(screen.showing(), Showing::Spaceport, "not over flight");
    assert_eq!(
        offered_mission(spaceport(&screen).offer_dialog()),
        Some(MissionId(130)),
        "offered over the spaceport (@0x5fc37) with no further input"
    );
    assert!(active(&screen).is_empty(), "not yet accepted");
}

/// Return would skip the movie (`KeyCheck` 0x24 @0x55de0-0x55e9f) and
/// its queue be flushed, so the offer's other accept key, its label's
/// first letter ("Yes"), stands for a key pressed under it.
#[test]
fn a_key_pressed_under_the_landings_movie_accepts_the_spaceport_offer() {
    let (mut screen, _) = landed_offering_under_the_movie();
    screen.input(&key(Key::Char('y'), true));
    screen.input(&key(Key::Char('y'), false));
    assert_eq!(screen.showing(), Showing::Movie, "queued, not acted on");
    assert!(
        active(&screen).is_empty(),
        "nothing accepted under the movie"
    );
    run_out(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    assert_eq!(
        active(&screen),
        [MissionId(130)],
        "the Y reached the offer, not the bare spaceport"
    );
    assert!(spaceport(&screen).offer_dialog().is_none(), "closed by it");
}
