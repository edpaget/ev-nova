//! The app over game data with no records, as the new game starts before
//! its loader reads anything: frames draw, with only missing pictures to
//! report, and New Pilot doesn't panic.

use std::rc::Rc;

use nova::app::{Control, Showing};
use nova_audio::AudioSettings;
use nova_data::GameData;
use nova_render::ImageError;
use nova_sim::fixture::MemoryPilots;
use nova_view::menu::MenuChoice;
use winit::keyboard::KeyCode;

use crate::pilots::Harness;

/// Draws a frame, whose only failures are pictures the data doesn't hold.
fn draw(game: &mut Harness) {
    assert_eq!(game.redraw(), Control::Continue);
    for (key, error) in game.app.take_failures() {
        assert!(matches!(error, ImageError::Missing), "{key:?}: {error:?}");
    }
}

#[test]
fn the_app_draws_and_survives_new_pilot_over_empty_data() {
    let store = MemoryPilots::new();
    let mut game = Harness::built_on(
        Rc::new(GameData::default()),
        &store,
        AudioSettings::default(),
        true,
    );
    assert_eq!(game.showing(), Showing::MainMenu);
    draw(&mut game);
    draw(&mut game);

    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    assert_eq!(game.showing(), Showing::NewPilot);
    draw(&mut game);
    game.type_name("Ada");
    game.press(KeyCode::Enter);
    game.press(KeyCode::Enter);
    draw(&mut game);
    draw(&mut game);
    assert_ne!(game.showing(), Showing::Flight, "no chär to fly from");
    assert_eq!(store.keys(), Vec::<String>::new(), "no pilot saved");
}
