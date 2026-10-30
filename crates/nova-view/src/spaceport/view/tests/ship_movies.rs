//! The shipyard's and the hire screen's looping ship movie (see
//! [`ship_movie`](crate::spaceport::ship_movie)) across the spaceport:
//! each way the spaceport closes them stops it, as their dialog's close
//! does (@0x5f73f-0x5f750), and a text's or an offer's movie plays on the
//! other player, leaving the loop alone.

use super::*;

fn start(file: &str) -> Call {
    Call::Start(file.to_owned())
}

/// Movies played on `main`, the loops on `reel`.
fn looping(main: &Rc<MockMoviePlayer>, reel: &Rc<MockMoviePlayer>) -> Movies {
    movies_of(main).with_looping_player(Rc::clone(reel) as Rc<dyn MoviePlayer>)
}

/// [`venues`] playing movies on `main` and loops on `reel`, its shipyard
/// open and listed, and ship 129, naming "Ship.mov", selected.
fn in_the_shipyard(main: &Rc<MockMoviePlayer>, reel: &Rc<MockMoviePlayer>) -> SpaceportView {
    let mut view = venues().with_movies(looping(main, reel));
    assert_eq!(view.take_offer_request(), Some(SPACEPORT));
    click_item(&mut view, 9);
    serve_openings(&mut view);
    view.input(&key(Key::Right));
    assert_eq!(reel.calls(), [start("Ship.mov")]);
    view
}

#[test]
fn a_purchase_closing_the_shipyard_stops_the_loop() {
    let (main, reel) = (
        MockMoviePlayer::playing((160, 120), 2),
        MockMoviePlayer::playing((32, 32), 100),
    );
    let mut view = in_the_shipyard(&main, &reel);
    view.input(&key(Key::Char('b')));
    assert_eq!(view.take_ship_request(), Some(ShipId(129)));
    view.open_ship_naming(&naming());
    view.input(&key(Key::Enter));
    assert!(view.take_ship().is_some(), "ordered");
    assert_eq!(reel.calls(), [start("Ship.mov")], "not before it closes");
    view.show_mission_texts(Vec::<MissionText>::new());
    assert!(view.open_shipyard().is_none(), "closed");
    assert_eq!(reel.calls(), [start("Ship.mov"), Call::Stop]);
    assert!(main.calls().is_empty());
}

#[test]
fn done_closing_the_shipyard_stops_the_loop_once() {
    let (main, reel) = (
        MockMoviePlayer::playing((160, 120), 2),
        MockMoviePlayer::playing((32, 32), 100),
    );
    let mut view = in_the_shipyard(&main, &reel);
    view.input(&key(Key::Escape));
    assert!(view.open_shipyard().is_none());
    assert_eq!(reel.calls(), [start("Ship.mov"), Call::Stop]);
}

#[test]
fn a_leave_with_the_shipyard_open_stops_the_loop() {
    let (main, reel) = (
        MockMoviePlayer::playing((160, 120), 2),
        MockMoviePlayer::playing((32, 32), 100),
    );
    let mut view = in_the_shipyard(&main, &reel);
    let _ = view.close_sounds();
    assert_eq!(reel.calls(), [start("Ship.mov"), Call::Stop]);
}

#[test]
fn a_leave_with_the_hire_screen_open_stops_its_loop() {
    let (main, reel) = (
        MockMoviePlayer::playing((160, 120), 2),
        MockMoviePlayer::playing((32, 32), 100),
    );
    let mut view = venues().with_movies(looping(&main, &reel));
    assert_eq!(view.take_offer_request(), Some(SPACEPORT));
    click_item(&mut view, 11);
    view.open_hire(hirelings(2, true), None);
    view.input(&key(Key::Right));
    view.input(&key(Key::Right));
    assert_eq!(reel.calls(), [start("Ship.mov")], "129 names it");
    let _ = view.close_sounds();
    assert_eq!(reel.calls(), [start("Ship.mov"), Call::Stop]);
}

#[test]
fn an_offers_movie_over_the_shipyard_plays_on_the_main_player_and_leaves_the_loop() {
    let (main, reel) = (
        MockMoviePlayer::playing((160, 120), 2),
        MockMoviePlayer::playing((32, 32), 100),
    );
    let mut view = in_the_shipyard(&main, &reel);
    assert_eq!(view.take_offer_request(), Some(SHIPYARD));
    view.open_offer(filmed_offer(0));
    for _ in 0..3 {
        view.tick(ms(100));
    }
    assert_eq!(started(&main), ["Offer.mov"]);
    assert!(offer_shown(&view), "its movie ended");
    assert_eq!(
        reel.calls(),
        [start("Ship.mov")],
        "neither stopped nor advanced under the offer"
    );
    view.input(&key(Key::Char('n')));
    assert!(view.offer_dialog().is_none(), "refused");
    view.tick(ms(16));
    assert_eq!(reel.calls(), [start("Ship.mov"), Call::Advance]);
    assert_eq!(reel.advanced_by(), [ms(316)], "the time it was covered");
}

#[test]
fn a_mission_text_over_the_shipyard_holds_the_loop() {
    let (main, reel) = (
        MockMoviePlayer::playing((160, 120), 2),
        MockMoviePlayer::playing((32, 32), 100),
    );
    let mut view = in_the_shipyard(&main, &reel);
    view.show_mission_texts([MissionText {
        kind: MissionTextKind::Comp,
        mission: MissionId(130),
        text: "Well done.".to_owned(),
        graphic: None,
        movie: None,
        map: None,
    }]);
    view.tick(ms(100));
    assert_eq!(reel.calls(), [start("Ship.mov")], "covered");
    view.input(&key(Key::Enter));
    assert!(!view.mission_dialogs_open());
    view.tick(ms(16));
    assert_eq!(reel.advanced_by(), [ms(116)]);
}
