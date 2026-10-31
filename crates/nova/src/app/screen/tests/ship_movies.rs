//! The router's wiring of the shipyard's and the hire screen's looping
//! ship movie (rdm:phase/movies/phase-7-shipyard-looping-ship-movie): the
//! real data's `shïp` `MovieFile` (offset 0x6AE, `_LoadObjectData`
//! @0x7b0ce-0x7b136) reaches the looping player given with
//! [`AppScreen::with_looping_movie_player`], never the texts' player. No
//! stock ship names a movie, so this runs on fixture data.

use super::*;
use nova_view::spaceport::BarScreen;

/// Alpha Prime, of tech level 1, whose shipyard sells `shïp` 129, naming
/// "Sold.mov", and whose bar hires out `shïp` 128, naming "Hired.mov";
/// Ada landed there, the texts' movies played on the first player and the
/// loops on the second, the two set up in either order.
fn filmed_port(looping_first: bool) -> (AppScreen, Rc<MockMoviePlayer>, Rc<MockMoviePlayer>) {
    let mut built = game_data_fork(false, true, true, 100, b"", b"", None).build();
    for (id, file) in [(128, &b"Hired.mov"[..]), (129, &b"Sold.mov"[..])] {
        let at = built.layout.data_entry(Ship::TYPE, id) + 4 + 0x6AE;
        built.bytes[at..at + file.len()].copy_from_slice(file);
    }
    let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
    built.put_u16(first + 0x38A, 100);
    let main = MockMoviePlayer::playing((160, 120), 2);
    let reel = MockMoviePlayer::playing((32, 32), 100);
    let screen = AppScreen::new(loaded_bytes(built.bytes));
    let main_player = Rc::clone(&main) as Rc<dyn MoviePlayer>;
    let reel_player = Rc::clone(&reel) as Rc<dyn MoviePlayer>;
    let screen = if looping_first {
        screen
            .with_looping_movie_player(reel_player)
            .with_movie_player(main_player)
    } else {
        screen
            .with_movie_player(main_player)
            .with_looping_movie_player(reel_player)
    };
    let store = MemoryPilots::new();
    let mut screen = screen
        .with_dialogs(Rc::new(PastTheHoldsDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    (screen, main, reel)
}

#[test]
fn the_shipyards_selected_ship_loops_its_movie_file_on_the_looping_player() {
    for looping_first in [false, true] {
        let (mut screen, main, reel) = filmed_port(looping_first);
        let landing = reeled(&main);
        click_port_item(&mut screen, 9);
        assert!(spaceport(&screen).open_shipyard().is_some(), "shipbuying");
        assert_eq!(reeled(&reel), [], "nothing selected yet");
        screen.input(&key(Key::Right, true));
        screen.input(&key(Key::Right, false));
        assert_eq!(
            reeled(&reel),
            [Call::Start("Sold.mov".into())],
            "{looping_first}"
        );
        screen.tick(Duration::from_millis(16));
        assert!(reel.calls().contains(&Call::Advance), "serviced");
        screen.input(&key(Key::Escape, true));
        assert!(spaceport(&screen).open_shipyard().is_none(), "closed");
        assert_eq!(reeled(&reel), [Call::Start("Sold.mov".into()), Call::Stop]);
        assert_eq!(reeled(&main), landing, "the texts' player untouched");
    }
}

#[test]
fn the_hire_screens_selected_ship_loops_its_movie_file_on_the_looping_player() {
    let (mut screen, main, reel) = filmed_port(false);
    let landing = reeled(&main);
    click_port_item(&mut screen, 11);
    screen.input(&key(Key::Char('h'), true));
    assert!(
        spaceport(&screen)
            .open_bar()
            .and_then(BarScreen::hire_screen)
            .is_some(),
        "for hire"
    );
    screen.input(&key(Key::Right, true));
    screen.input(&key(Key::Right, false));
    assert_eq!(reeled(&reel), [Call::Start("Hired.mov".into())]);
    screen.input(&key(Key::Escape, true));
    assert!(
        spaceport(&screen)
            .open_bar()
            .is_some_and(|bar| bar.hire_screen().is_none()),
        "back in the bar"
    );
    assert_eq!(reeled(&reel), [Call::Start("Hired.mov".into()), Call::Stop]);
    assert_eq!(reeled(&main), landing, "the texts' player untouched");
}
