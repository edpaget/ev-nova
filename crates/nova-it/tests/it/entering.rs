//! Entering play through the whole app (`nova::app::screen`'s `entry`):
//! the music's order against the main screen's fade and flight's first
//! frame, with the app driving the audio core, and the intro seen once,
//! across a restart. Built on the pilots tests' app ([`Harness`]).

use nova::app::{Showing, WindowEvent};
use nova_audio::{AudioCommand, AudioSettings};
use nova_sim::fixture::MemoryPilots;
use nova_view::fade::DEATH_FADE_FOR;
use nova_view::menu::MenuChoice;
use winit::keyboard::KeyCode;

use crate::pilots::{Harness, intro_game_data, saved};

/// The redraws' pace, as [`Harness::frame`] sends them.
const FRAME_MS: u64 = 16;

/// A new pilot named Ada made on `game`'s main menu, held there.
fn new_pilot(game: &mut Harness) {
    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    game.type_name("Ada");
    game.press(KeyCode::Enter);
    // The ship's christening, confirmed with its default.
    game.press(KeyCode::Enter);
    assert_eq!(game.showing(), Showing::MainMenu);
}

#[test]
fn the_music_fades_out_after_the_main_screens_fade_by_flights_first_frame() {
    let store = MemoryPilots::new();
    let mut game = Harness::opening(&store);
    game.frame();
    assert!(matches!(
        game.music()[..],
        [AudioCommand::StartMusic { .. }]
    ));
    new_pilot(&mut game);
    game.frame();
    game.music();
    let at = crate::common::enter_ship_at(game.app.screen());
    game.click(at);
    assert!(game.app.screen().entering());
    let mut frames = 0;
    let faded = loop {
        frames += 1;
        assert!(frames < 200, "flight never shows");
        game.frame();
        let music = game.music();
        if game.showing() == Showing::Flight {
            break music;
        }
        assert_eq!(
            music,
            [],
            "frame {frames}: nothing before flight's first frame"
        );
    };
    let fade_frames = u64::try_from(DEATH_FADE_FOR.as_millis()).expect("short") / FRAME_MS;
    assert!(
        frames >= fade_frames,
        "flight showed after {frames} frames, before the fade's second"
    );
    assert!(
        matches!(faded[..], [AudioCommand::FadeOutMusic(_)]),
        "{faded:?}: told to fade by the redraw that first draws flight"
    );
    while game.app.screen().entering() {
        game.frame();
    }
    game.press(KeyCode::Escape);
    assert_eq!(game.showing(), Showing::MainMenu);
    for _ in 0..100 {
        game.frame();
    }
    assert_eq!(game.music(), [], "never starts again");
}

#[test]
fn a_pilot_that_saw_the_intro_and_landed_enters_with_none_after_a_restart() {
    let store = MemoryPilots::new();
    let mut game = Harness::built_on(intro_game_data(), &store, AudioSettings::default(), false);
    new_pilot(&mut game);
    let at = crate::common::enter_ship_at(game.app.screen());
    game.click(at);
    let mut intro = false;
    while game.app.screen().entering() {
        intro |= game.showing() == Showing::Intro;
        game.frame();
    }
    assert!(intro, "the first entry shows the intro");
    game.frame();
    game.land();
    assert_eq!(game.showing(), Showing::Spaceport);
    assert!(saved(&store, "Ada").seen_intro(), "the landing saved it");
    assert_eq!(
        game.handle(WindowEvent::CloseRequested),
        nova::app::Control::Exit
    );

    let mut game = Harness::built_on(intro_game_data(), &store, AudioSettings::default(), true);
    let held = game
        .app
        .screen()
        .game_in_progress()
        .expect("held at launch");
    assert!(held.seen_intro());
    let at = crate::common::enter_ship_at(game.app.screen());
    game.click(at);
    while game.app.screen().entering() {
        assert_ne!(game.showing(), Showing::Intro, "no intro again");
        game.frame();
    }
    assert_eq!(game.showing(), Showing::Flight);
}
