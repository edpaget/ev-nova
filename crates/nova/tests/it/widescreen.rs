//! The whole app in a 16:9 window, over the persons' game
//! ([`crate::persons`]): flight widens to fill the window, a hail's comm
//! dialog shows centred over it, and the spaceport lands pillarboxed;
//! in a 4:3 window flight is Nova's own 1024 x 768.

use nova::app::Showing;
use nova_render::{Batch, Frame, LogicalSize, PixelRect};
use nova_sim::fixture::MemoryPilots;
use nova_view::{DrawCommand, DrawList, Key, Screen};

use crate::persons::{BOTH, Game, idle};

/// A 16:9 window.
const WINDOW: (u32, u32) = (1920, 1080);

fn whole(window: (u32, u32)) -> PixelRect {
    PixelRect {
        x: 0,
        y: 0,
        w: window.0,
        h: window.1,
    }
}

/// Every text run's text and its origin in window pixels.
fn runs(frame: &Frame) -> Vec<(String, (f32, f32))> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Text(runs) => runs
                .iter()
                .map(|run| (run.text.clone(), run.origin_px))
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

#[test]
fn flight_in_a_16_by_9_window_fills_it_768_high() {
    let store = MemoryPilots::new();
    let mut game = Game::opened_in(WINDOW, &store, &BOTH, Showing::Flight, idle);
    let frame = game.frame();
    assert_eq!(frame.logical, LogicalSize { w: 1365, h: 768 });
    assert_eq!(frame.viewport, whole(WINDOW), "no side bars");
}

#[test]
fn a_hail_opens_the_comm_dialog_centred_over_widened_flight() {
    let store = MemoryPilots::new();
    let mut game = Game::opened_in(WINDOW, &store, &BOTH, Showing::Flight, idle);
    game.tap(Key::Tab);
    game.tap(Key::Tab);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Comm);
    let frame = game.frame();
    assert_eq!(frame.logical, LogicalSize { w: 1365, h: 768 });
    let mut comm = DrawList::new();
    game.app.screen().comm().expect("open").draw(&mut comm);
    let (text, origin) = comm
        .iter()
        .find_map(|command| match command {
            DrawCommand::Text { text, origin, .. } => Some((text.clone(), *origin)),
            _ => None,
        })
        .expect("the dialog's text");
    // 170 right of its 1024 x 768 place, in window pixels.
    let across = 1920.0 / 1365.0;
    let down = 1080.0 / 768.0;
    let at = ((origin.x + 170.0) * across, origin.y * down);
    assert!(
        runs(&frame).iter().any(|(drawn, px)| *drawn == text
            && (px.0 - at.0).abs() < 0.01
            && (px.1 - at.1).abs() < 0.01),
        "{text} at {at:?}: {:?}",
        runs(&frame)
    );
}

#[test]
fn landing_in_a_16_by_9_window_shows_the_spaceport_pillarboxed() {
    let store = MemoryPilots::new();
    let mut game = Game::opened_in(WINDOW, &store, &BOTH, Showing::Flight, idle);
    game.tap(Key::Char('l'));
    game.approach();
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Spaceport);
    let frame = game.frame();
    assert_eq!(frame.logical, LogicalSize { w: 1024, h: 768 });
    assert_eq!(
        frame.viewport,
        PixelRect {
            x: 240,
            y: 0,
            w: 1440,
            h: 1080
        }
    );
}

#[test]
fn flight_in_a_4_by_3_window_is_novas_own_1024_by_768() {
    let store = MemoryPilots::new();
    let mut game = Game::opened_in((1024, 768), &store, &BOTH, Showing::Flight, idle);
    let frame = game.frame();
    assert_eq!(frame.logical, LogicalSize { w: 1024, h: 768 });
    assert_eq!(frame.viewport, whole((1024, 768)));
}
