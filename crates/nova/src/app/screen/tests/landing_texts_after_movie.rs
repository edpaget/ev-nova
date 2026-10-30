//! The router over a landing's movie and its land checks' texts
//! (rdm:phase/movies/phase-9-landing-movie-before-land-texts):
//! `_PlayerLandOnStellar` plays the movie to its end (@0x632a7) before
//! `_DoPortDialog` (@0x63381), which shows the spaceport (@0x5faff) and
//! only then the land checks' texts (`_MissionLandCheck` @0x5fbeb), each
//! text's movies inside its own dialog. Keys pressed under the movie
//! reach the first text's loop. Over the fixture of `desc_load_order`.

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
