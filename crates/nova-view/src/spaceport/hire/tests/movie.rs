//! The selected ship's movie looping in item 8 on the hire screen, as on
//! the shipyard: the movie code reads no `_shipBuyMode` (see
//! [`ship_movie`](crate::spaceport::ship_movie)).

use super::*;
use crate::movie::fixture::{Call, MockMoviePlayer};
use crate::movie::{MoviePlayer, Movies};
use crate::spaceport::grid::{NO_PICTURE, PICTURE_AVAILABLE};

/// [`art`], where ship 128 (`PICT` 5000) names "Ship.mov"; ship 140 (no
/// picture) and ship 141 (128's base, so its picture) name none.
fn filmed() -> Rc<FakeArt> {
    let art = Rc::try_unwrap(art()).ok().expect("unshared");
    Rc::new(FakeArt {
        movies: vec![(ShipId(128), "Ship.mov".to_owned())],
        ..art
    })
}

/// Ships 128, 140 and 141 for hire.
fn three() -> HireList {
    HireList {
        rows: vec![
            row(128, 970, 100, Ok(())),
            row(140, 970, 100, Ok(())),
            row(141, 970, 100, Ok(())),
        ],
        ..list()
    }
}

/// [`three`] over `art` as it opens, movies played on a main player and
/// the loops on `reel`, `on` or off.
fn opened_on(art: &Rc<FakeArt>, reel: &Rc<MockMoviePlayer>, on: bool) -> HireScreen {
    let main = MockMoviePlayer::playing((640, 480), 5);
    let mut screen = opened_of(art, three());
    screen.set_movies(
        Movies::new(Rc::clone(&main) as Rc<dyn MoviePlayer>, on)
            .with_looping_player(Rc::clone(reel) as Rc<dyn MoviePlayer>),
    );
    screen
}

/// [`opened_on`] [`filmed`], movies on, ship 128 selected by Right.
fn first_on(reel: &Rc<MockMoviePlayer>) -> HireScreen {
    let mut screen = opened_on(&filmed(), reel, true);
    screen.input(&key(Key::Right, false));
    assert_eq!(screen.selected(), Some(0));
    screen
}

fn start(file: &str) -> Call {
    Call::Start(file.to_owned())
}

fn frames(screen: &HireScreen) -> Vec<Bounds> {
    drawn(screen)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::MovieFrame { area, .. } => Some(*area),
            _ => None,
        })
        .collect()
}

fn pictured(screen: &HireScreen) -> bool {
    let picture = item(screen, PICTURE_ITEM);
    drawn(screen).iter().any(|command| {
        matches!(command, DrawCommand::StretchedPicture { top_left, .. } if *top_left == picture.min)
    })
}

fn no_picture(screen: &HireScreen) -> bool {
    let words: Vec<String> = texts(screen).into_iter().map(|(text, _, _)| text).collect();
    words.contains(&NO_PICTURE.to_owned()) && words.contains(&PICTURE_AVAILABLE.to_owned())
}

#[test]
fn a_ship_with_a_movie_shows_its_frames_in_item_8() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let screen = first_on(&reel);
    assert_eq!(reel.calls(), [start("Ship.mov")]);
    assert_eq!(frames(&screen), [item(&screen, PICTURE_ITEM)]);
    assert!(!pictured(&screen));
    assert!(!no_picture(&screen));
    assert!(
        texts(&screen)
            .iter()
            .any(|(text, _, _)| text == "dësc 14000."),
        "the pilot's description still"
    );
}

#[test]
fn a_ship_without_one_keeps_its_picture_or_no_picture() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let mut screen = opened_on(&filmed(), &reel, true);
    click_cell(&mut screen, 2);
    assert_eq!(screen.selected(), Some(2), "ship 141");
    assert!(pictured(&screen));
    click_cell(&mut screen, 1);
    assert_eq!(screen.selected(), Some(1), "ship 140");
    assert!(no_picture(&screen));
    assert!(reel.calls().is_empty());
    assert_eq!(frames(&screen), []);
}

#[test]
fn the_movie_restarts_when_it_ends() {
    let reel = MockMoviePlayer::playing((32, 32), 2);
    let mut screen = first_on(&reel);
    screen.tick(Duration::from_millis(100));
    screen.tick(Duration::from_millis(100));
    assert_eq!(
        reel.calls(),
        [
            start("Ship.mov"),
            Call::Advance,
            Call::Advance,
            start("Ship.mov")
        ]
    );
}

#[test]
fn a_new_selection_an_empty_cell_or_done_stops_it() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let mut screen = first_on(&reel);
    screen.input(&key(Key::Right, false));
    assert_eq!(reel.calls(), [start("Ship.mov"), Call::Stop], "ship 140");
    click_cell(&mut screen, 0);
    click_cell(&mut screen, 5);
    assert_eq!(screen.selected(), None);
    assert_eq!(
        reel.calls()[2..],
        [start("Ship.mov"), Call::Stop],
        "an empty cell"
    );
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let mut screen = first_on(&reel);
    screen.input(&key(Key::Escape, false));
    assert!(screen.closed());
    assert_eq!(reel.calls(), [start("Ship.mov"), Call::Stop], "Done");
    screen.end_movie();
    assert_eq!(reel.calls().len(), 2, "once");
}

#[test]
fn the_info_panel_does_not_stop_it_and_it_holds_its_frame() {
    let reel = MockMoviePlayer::playing((32, 32), 100);
    let mut screen = first_on(&reel);
    screen.input(&key(INFO_KEY, false));
    assert!(screen.info_panel().is_some());
    screen.tick(Duration::from_millis(100));
    screen.tick_covered(Duration::from_millis(100));
    assert_eq!(
        reel.calls(),
        [start("Ship.mov")],
        "neither advanced nor stopped"
    );
    assert_eq!(
        frames(&screen),
        [item(&screen, PICTURE_ITEM)],
        "its frame holds"
    );
    screen.input(&key(Key::Escape, false));
    assert!(screen.info_panel().is_none());
    screen.tick(Duration::from_millis(16));
    assert_eq!(reel.advanced_by(), [Duration::from_millis(216)]);
}

#[test]
fn with_movies_off_or_a_refusing_player_the_picture_is_drawn() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let mut screen = opened_on(&filmed(), &reel, false);
    screen.input(&key(Key::Right, false));
    assert!(reel.calls().is_empty());
    assert!(pictured(&screen));
    let refusing = MockMoviePlayer::refusing();
    let screen = first_on(&refusing);
    assert_eq!(refusing.calls(), [start("Ship.mov")]);
    assert_eq!(frames(&screen), []);
    assert!(pictured(&screen));
}
