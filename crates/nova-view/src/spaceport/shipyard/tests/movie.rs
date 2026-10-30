//! The selected ship's movie looping in item 8 (see
//! [`ship_movie`](crate::spaceport::ship_movie)), on the shipyard.

use super::*;
use crate::movie::MoviePlayer;
use crate::movie::fixture::{Call, MockMoviePlayer};
use crate::spaceport::grid::{NO_PICTURE, PICTURE_AVAILABLE};

/// [`art`], where the Shuttle (128) names "Ship.mov" and the Courier
/// (130) "Courier.mov"; the Heavy Shuttle (129, no picture) and the
/// Viper (131, its base's picture) name none. Of [`long`]'s hulls, 200
/// and 201 name "Hull 0.mov" and "Hull 1.mov".
fn filmed() -> Rc<FakeArt> {
    let art = Rc::try_unwrap(art()).ok().expect("unshared");
    Rc::new(FakeArt {
        movies: vec![
            (ShipId(128), "Ship.mov".to_owned()),
            (ShipId(130), "Courier.mov".to_owned()),
            (ShipId(200), "Hull 0.mov".to_owned()),
            (ShipId(201), "Hull 1.mov".to_owned()),
        ],
        ..art
    })
}

/// Movies played on `main`, the loops on `reel`, `on` or off.
fn movies(main: &Rc<MockMoviePlayer>, reel: &Rc<MockMoviePlayer>, on: bool) -> Movies {
    Movies::new(Rc::clone(main) as Rc<dyn MoviePlayer>, on)
        .with_looping_player(Rc::clone(reel) as Rc<dyn MoviePlayer>)
}

/// `shipyard` over [`filmed`] as it opens, movies given with the loops
/// on `reel` and `on` or off; and the main player, which the loop never
/// touches.
fn opened_on(
    shipyard: Shipyard,
    reel: &Rc<MockMoviePlayer>,
    on: bool,
) -> (ShipyardScreen, Rc<MockMoviePlayer>) {
    let main = MockMoviePlayer::playing((640, 480), 5);
    let mut screen = opened_with(shipyard, &filmed());
    screen.set_offer_movies(movies(&main, reel, on));
    (screen, main)
}

/// [`shipyard`] opened over [`filmed`], movies on, the Shuttle selected
/// by Right.
fn shuttle_on(reel: &Rc<MockMoviePlayer>) -> (ShipyardScreen, Rc<MockMoviePlayer>) {
    let (mut screen, main) = opened_on(shipyard(), reel, true);
    press(&mut screen, Key::Right);
    assert_eq!(screen.selected(), Some(0));
    (screen, main)
}

fn start(file: &str) -> Call {
    Call::Start(file.to_owned())
}

/// The areas of the movie frames drawn.
fn frames(screen: &ShipyardScreen) -> Vec<Bounds> {
    drawn(screen)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::MovieFrame { area, .. } => Some(*area),
            _ => None,
        })
        .collect()
}

/// Whether a picture is stretched into item 8.
fn pictured(screen: &ShipyardScreen) -> bool {
    let picture = item(screen, PICTURE_ITEM);
    drawn(screen).iter().any(|command| {
        matches!(command, DrawCommand::StretchedPicture { top_left, .. } if *top_left == picture.min)
    })
}

/// Whether "No Picture" and "Available" are drawn.
fn no_picture(screen: &ShipyardScreen) -> bool {
    let words = words(screen);
    words.contains(&NO_PICTURE.to_owned()) && words.contains(&PICTURE_AVAILABLE.to_owned())
}

/// Whether the description is drawn, as it is with or without a movie.
fn described(screen: &ShipyardScreen) -> bool {
    words(screen).iter().any(|word| word.starts_with("dësc"))
}

#[test]
fn a_ship_with_a_movie_shows_its_frames_in_item_8() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let (screen, main) = shuttle_on(&reel);
    assert_eq!(reel.calls(), [start("Ship.mov")]);
    assert_eq!(frames(&screen), [item(&screen, PICTURE_ITEM)]);
    assert!(!pictured(&screen), "not the picture (@0x59238)");
    assert!(!no_picture(&screen));
    assert!(described(&screen), "the description still");
    assert!(main.calls().is_empty(), "the loop has its own player");
}

#[test]
fn a_ship_without_one_keeps_its_picture() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let (mut screen, _) = opened_on(shipyard(), &reel, true);
    click_cell(&mut screen, 3);
    assert_eq!(screen.selected(), Some(3), "the Viper");
    assert!(reel.calls().is_empty());
    assert_eq!(frames(&screen), []);
    assert!(pictured(&screen));
}

#[test]
fn a_ship_without_one_or_a_picture_keeps_no_picture() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let (mut screen, _) = opened_on(shipyard(), &reel, true);
    click_cell(&mut screen, 1);
    assert_eq!(screen.selected(), Some(1), "the Heavy Shuttle");
    assert!(reel.calls().is_empty());
    assert_eq!(frames(&screen), []);
    assert!(no_picture(&screen));
}

#[test]
fn the_movie_restarts_when_it_ends() {
    let reel = MockMoviePlayer::playing((32, 32), 2);
    let (mut screen, _) = shuttle_on(&reel);
    screen.tick(ms(100));
    screen.tick(ms(100));
    assert_eq!(
        reel.calls(),
        [
            start("Ship.mov"),
            Call::Advance,
            Call::Advance,
            start("Ship.mov")
        ]
    );
    assert_eq!(frames(&screen), [item(&screen, PICTURE_ITEM)]);
}

#[test]
fn a_new_selection_stops_it_and_starts_the_next_by_click_and_by_arrow() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let (mut screen, _) = shuttle_on(&reel);
    click_cell(&mut screen, 2);
    assert_eq!(
        reel.calls(),
        [start("Ship.mov"), Call::Stop, start("Courier.mov")],
        "@0x60c4d-0x60cd2"
    );
    press(&mut screen, Key::Right);
    assert_eq!(screen.selected(), Some(3), "the Viper");
    assert_eq!(reel.calls()[3..], [Call::Stop], "names none: only stopped");
    assert!(pictured(&screen));
    press(&mut screen, Key::Left);
    press(&mut screen, Key::Left);
    press(&mut screen, Key::Left);
    assert_eq!(screen.selected(), Some(0));
    assert_eq!(
        reel.calls()[4..],
        [start("Courier.mov"), Call::Stop, start("Ship.mov")],
        "@0x6082e-0x608b0; the Heavy Shuttle names none"
    );
}

#[test]
fn the_same_ship_again_does_not_restart_it() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let (mut screen, _) = shuttle_on(&reel);
    click_cell(&mut screen, 0);
    press(&mut screen, Key::Up);
    assert_eq!(screen.selected(), Some(0), "Up on the first row stays");
    assert_eq!(reel.calls(), [start("Ship.mov")]);
}

#[test]
fn an_empty_cell_or_a_scroll_stops_it_by_the_engine() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let (mut screen, _) = shuttle_on(&reel);
    click_cell(&mut screen, 5);
    assert_eq!(screen.selected(), None);
    assert_eq!(reel.calls(), [start("Ship.mov"), Call::Stop]);
    assert_eq!(frames(&screen), []);
    let scrolled = MockMoviePlayer::playing((32, 32), 3);
    let (mut screen, _) = opened_on(long(30), &scrolled, true);
    click_cell(&mut screen, 0);
    click_item(&mut screen, SCROLL_DOWN_ITEM);
    assert_eq!(screen.selected(), None);
    assert_eq!(
        scrolled.calls(),
        [start("Hull 0.mov"), Call::Stop],
        "@0x5f5c2-0x5f5d3"
    );
}

#[test]
fn by_the_other_reading_a_scroll_keeps_the_ship_and_its_movie() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let (mut screen, _) = opened_on(
        Shipyard {
            rules: bible(),
            ..long(30)
        },
        &reel,
        true,
    );
    assert_eq!(screen.selected(), Some(0));
    click_item(&mut screen, SCROLL_DOWN_ITEM);
    assert_eq!(screen.selected(), Some(0), "kept");
    assert_eq!(reel.calls(), [start("Hull 0.mov")]);
}

#[test]
fn by_the_other_reading_the_first_cells_movie_starts_once_movies_are_given() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let main = MockMoviePlayer::playing((640, 480), 5);
    let mut screen = opened_with(
        Shipyard {
            rules: bible(),
            ..shipyard()
        },
        &filmed(),
    );
    assert_eq!(screen.selected(), Some(0), "selected as it opens");
    assert!(reel.calls().is_empty(), "no movies yet");
    assert!(pictured(&screen));
    screen.set_offer_movies(movies(&main, &reel, true));
    assert_eq!(reel.calls(), [start("Ship.mov")]);
    assert_eq!(frames(&screen), [item(&screen, PICTURE_ITEM)]);
}

#[test]
fn done_stops_it() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let (mut screen, _) = shuttle_on(&reel);
    click_item(&mut screen, DONE_ITEM);
    assert!(screen.closed());
    assert_eq!(
        reel.calls(),
        [start("Ship.mov"), Call::Stop],
        "@0x5f73f-0x5f750"
    );
    screen.end_movie();
    assert_eq!(reel.calls().len(), 2, "stopped once");
}

#[test]
fn end_movie_stops_it_for_the_spaceports_other_closes() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let (mut screen, _) = shuttle_on(&reel);
    screen.end_movie();
    assert_eq!(reel.calls(), [start("Ship.mov"), Call::Stop]);
    assert!(pictured(&screen), "the picture again");
}

/// Ticks `screen` twice while something covers it, checks the loop is
/// neither advanced nor stopped and its frame still drawn; then, once
/// `uncover` has run, that the next tick advances it by the time covered
/// and the tick.
fn holds_while_covered(
    reel: &MockMoviePlayer,
    screen: &mut ShipyardScreen,
    uncover: impl FnOnce(&mut ShipyardScreen),
) {
    let before = reel.calls().len();
    screen.tick(ms(100));
    screen.tick(ms(100));
    assert_eq!(reel.calls().len(), before, "neither advanced nor stopped");
    assert_eq!(
        frames(screen),
        [item(screen, PICTURE_ITEM)],
        "its frame holds"
    );
    uncover(screen);
    screen.tick(ms(16));
    assert_eq!(
        reel.advanced_by().last(),
        Some(&ms(216)),
        "the time base ran on"
    );
}

#[test]
fn the_info_panel_does_not_stop_it_and_it_holds_its_frame() {
    let reel = MockMoviePlayer::playing((32, 32), 100);
    let (mut screen, _) = shuttle_on(&reel);
    press(&mut screen, INFO_KEY);
    assert!(screen.info_panel().is_some());
    holds_while_covered(&reel, &mut screen, |screen| {
        press(screen, Key::Escape);
        assert!(screen.info_panel().is_none());
    });
    assert!(!reel.calls().contains(&Call::Stop));
}

#[test]
fn a_double_clicks_info_does_not_stop_it() {
    let reel = MockMoviePlayer::playing((32, 32), 100);
    let (mut screen, _) = opened_on(shipyard(), &reel, true);
    click_cell(&mut screen, 0);
    click_cell(&mut screen, 0);
    assert!(screen.info_panel().is_some(), "@0x60cd7-0x60d12");
    assert_eq!(reel.calls(), [start("Ship.mov")]);
    holds_while_covered(&reel, &mut screen, |screen| press(screen, Key::Escape));
}

#[test]
fn the_name_prompt_does_not_stop_it_and_it_holds_its_frame() {
    let reel = MockMoviePlayer::playing((32, 32), 100);
    let (mut screen, _) = shuttle_on(&reel);
    screen.open_naming(&ShipNaming {
        ship: ShipId(128),
        ..naming()
    });
    assert!(screen.naming().is_some());
    holds_while_covered(&reel, &mut screen, |screen| {
        naming_click(screen, crate::ui::text_input::CANCEL_ITEM);
        assert!(screen.naming().is_none(), "cancelled");
    });
    assert!(!reel.calls().contains(&Call::Stop));
}

#[test]
fn an_offer_over_it_holds_it_and_so_does_a_covered_tick() {
    let reel = MockMoviePlayer::playing((32, 32), 100);
    let main = MockMoviePlayer::playing((640, 480), 5);
    let mut screen = opened_with(shipyard(), &filmed()).with_offers(offer_templates());
    screen.set_offer_movies(movies(&main, &reel, true));
    press(&mut screen, Key::Right);
    screen.open_offer(prompt());
    assert!(screen.offer_dialog().is_some());
    holds_while_covered(&reel, &mut screen, |screen| press(screen, Key::Enter));
    screen.tick_covered(ms(100));
    screen.tick(ms(16));
    assert_eq!(reel.advanced_by().last(), Some(&ms(116)), "a covered tick");
    assert!(main.calls().is_empty(), "the offer names no movie");
}

#[test]
fn with_movies_off_no_movie_starts_and_the_picture_is_drawn() {
    let reel = MockMoviePlayer::playing((32, 32), 3);
    let (mut screen, _) = opened_on(shipyard(), &reel, false);
    press(&mut screen, Key::Right);
    assert!(reel.calls().is_empty(), "@0x60282-0x6028b");
    assert_eq!(frames(&screen), []);
    assert!(pictured(&screen));
}

#[test]
fn a_player_that_cannot_start_leaves_the_picture_or_no_picture() {
    let reel = MockMoviePlayer::refusing();
    let (screen, _) = shuttle_on(&reel);
    assert_eq!(reel.calls(), [start("Ship.mov")]);
    assert_eq!(frames(&screen), []);
    assert!(pictured(&screen), "the Shuttle's picture");
    let films = vec![(ShipId(129), "Heavy.mov".to_owned())];
    let art = Rc::try_unwrap(art()).ok().expect("unshared");
    let heavy = Rc::new(FakeArt {
        movies: films,
        ..art
    });
    let main = MockMoviePlayer::playing((640, 480), 5);
    let mut screen = opened_with(shipyard(), &heavy);
    screen.set_offer_movies(movies(&main, &reel, true));
    click_cell(&mut screen, 1);
    assert_eq!(reel.calls()[1..], [start("Heavy.mov")]);
    assert!(no_picture(&screen), "the Heavy Shuttle's No Picture");
}
