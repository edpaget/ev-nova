//! Flight's world layer through the whole app, over the persons' game
//! ([`crate::persons`]): flight draws its scene into the world layer, and
//! its HUD, every text and the dialogs over it into the overlay, in a 4:3
//! and a 16:9 window; the spaceport draws no world.

use nova::app::Showing;
use nova_render::{Batch, Frame, QuadInstance};
use nova_sim::fixture::MemoryPilots;
use nova_view::{DrawCommand, DrawLayer, DrawList, Key, Screen};

use crate::persons::{BOTH, Game, idle};

const WORLD: DrawLayer = DrawLayer::World;
const OVERLAY: DrawLayer = DrawLayer::Overlay;

/// A 4:3 window and a 16:9 one.
const WINDOWS: [(u32, u32); 2] = [(1024, 768), (1920, 1080)];

/// The text drawn in `layer`.
fn texts_in(frame: &Frame, layer: DrawLayer) -> Vec<String> {
    frame
        .in_layer(layer)
        .flat_map(|batch| match batch {
            Batch::Text(runs) => runs.iter().map(|run| run.text.clone()).collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// The sprite and picture quads drawn in `layer`.
fn quads_in(frame: &Frame, layer: DrawLayer) -> Vec<QuadInstance> {
    frame
        .in_layer(layer)
        .flat_map(|batch| match batch {
            Batch::Sprites { quads, .. } => quads.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// The game in flight in a `window` window.
fn flying(window: (u32, u32), store: &MemoryPilots) -> Game {
    Game::opened_in(window, store, &BOTH, Showing::Flight, idle)
}

#[test]
fn flight_draws_its_scene_in_the_world_and_its_hud_and_text_over_it() {
    for window in WINDOWS {
        let store = MemoryPilots::new();
        let mut game = flying(window, &store);
        let frame = game.frame();
        let (w, h) = (frame.logical.w as f32, frame.logical.h as f32);
        // The scene first, in the world (the fixture has no stars' sheet,
        // so no stars).
        assert_eq!(frame.layer(0), WORLD, "{window:?}");
        // The player's ship, at the play area's centre left of the 194-wide
        // status bar, and the two NPCs above it, in the world.
        let world = quads_in(&frame, WORLD);
        let centred = |quad: &QuadInstance| {
            let dest = quad.dest;
            let (x, y) = (dest.x + dest.w / 2.0, dest.y + dest.h / 2.0);
            (x - ((w - 194.0) / 2.0).trunc()).abs() <= 1.0 && (y - (h / 2.0).trunc()).abs() <= 1.0
        };
        assert!(world.iter().any(centred), "{window:?}: {world:?}");
        assert!(world.len() >= 3, "{window:?}: {world:?}");
        // The status bar, at the view's right edge, over the world, which
        // has nothing under it.
        let bar = quads_in(&frame, OVERLAY)
            .into_iter()
            .find(|quad| (quad.dest.x + quad.dest.w - w).abs() < 0.5)
            .expect("the status bar");
        assert!(
            world
                .iter()
                .all(|quad| quad.dest.x + quad.dest.w <= bar.dest.x),
            "{window:?}: {world:?}"
        );
        // Every text over the world.
        assert!(texts_in(&frame, WORLD).is_empty(), "{window:?}");
        assert!(
            texts_in(&frame, OVERLAY).contains(&"No Target".to_owned()),
            "{window:?}"
        );
    }
}

#[test]
fn a_hails_comm_dialog_over_widened_flight_is_overlay() {
    let store = MemoryPilots::new();
    let mut game = flying((1920, 1080), &store);
    game.tap(Key::Tab);
    game.tap(Key::Tab);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Comm);
    let frame = game.frame();
    let mut comm = DrawList::new();
    game.app.screen().comm().expect("open").draw(&mut comm);
    let dialog: Vec<String> = comm
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(!dialog.is_empty());
    let overlay = texts_in(&frame, OVERLAY);
    for text in &dialog {
        assert!(overlay.contains(text), "{text}: {overlay:?}");
    }
    assert!(texts_in(&frame, WORLD).is_empty());
    assert!(
        frame.in_layer(WORLD).next().is_some(),
        "flight's world beneath"
    );
}

#[test]
fn the_spaceport_draws_no_world() {
    let store = MemoryPilots::new();
    let mut game = flying((1920, 1080), &store);
    game.tap(Key::Char('l'));
    game.approach();
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Spaceport);
    let frame = game.frame();
    assert_eq!(frame.world, [0_usize; 0]);
    assert!(!frame.batches.is_empty());
}
