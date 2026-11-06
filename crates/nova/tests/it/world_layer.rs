//! Flight's world layer through the whole app, over the persons' game
//! ([`crate::persons`]): flight's frame reaches the Gpu as its world layer
//! then an overlay, in a 4:3 and a 16:9 window, a hail's comm dialog is
//! drawn over flight's world, under the sharp-bilinear upscaler the message
//! line's box goes into the world texture and its text stays overlay, and
//! the spaceport draws no world. What goes in which layer, and where, is
//! nova-view's, pinned there.

use nova::app::Showing;
use nova_render::{Batch, Frame, QuadInstance, Upscaler, WorldSpace};
use nova_sim::fixture::MemoryPilots;
use nova_view::flight::hud;
use nova_view::geometry::Bounds;
use nova_view::{DrawCommand, DrawLayer, DrawList, Key, Point, Screen};

use crate::common::Frames;
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
fn flights_frame_reaches_the_gpu_as_its_world_then_an_overlay_in_both_windows() {
    for window in WINDOWS {
        let store = MemoryPilots::new();
        let mut game = flying(window, &store);
        let frame = game.frame();
        assert_eq!(frame.layer(0), WORLD, "{window:?}");
        assert!(!quads_in(&frame, WORLD).is_empty(), "{window:?}");
        assert!(!texts_in(&frame, OVERLAY).is_empty(), "{window:?}");
    }
}

#[test]
fn a_hails_comm_dialog_over_widened_flight_is_overlay() {
    let store = MemoryPilots::new();
    let mut game = flying((1920, 1080), &store);
    for key in [Key::Tab, Key::Tab, Key::Char('y')] {
        game.tap(key);
        game.settle();
    }
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

/// The areas the solid quads drawn in `layer` cover: each one's corners'
/// bounds.
fn solids_in(frame: &Frame, layer: DrawLayer) -> Vec<Bounds> {
    frame
        .in_layer(layer)
        .flat_map(|batch| match batch {
            Batch::Solid(quads) => quads
                .iter()
                .map(|quad| {
                    let [first, rest @ ..] = quad.corners;
                    rest.iter().fold(
                        Bounds {
                            min: first,
                            max: first,
                        },
                        |area, corner| Bounds {
                            min: Point::new(area.min.x.min(corner.x), area.min.y.min(corner.y)),
                            max: Point::new(area.max.x.max(corner.x), area.max.y.max(corner.y)),
                        },
                    )
                })
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

#[test]
fn under_sharp_bilinear_flights_message_text_is_overlay_and_its_box_world() {
    let store = MemoryPilots::new();
    let mut game = flying((1024, 768), &store);
    game.upscale_with(Upscaler::SharpBilinear);
    game.tap(Key::Char('l'));
    game.settle_until("a message on the line", |game| {
        game.app
            .screen()
            .flight_view()
            .expect("flying")
            .message()
            .is_some()
    });
    let frame = game.frame();
    let flight = game.app.screen().flight_view().expect("flying");
    let message = flight.message().expect("a message on the line").to_owned();
    assert!(
        matches!(frame.world_space, WorldSpace::Texture { .. }),
        "{:?}",
        frame.world_space
    );
    assert!(
        texts_in(&frame, OVERLAY).contains(&message),
        "{message}: {:?}",
        texts_in(&frame, OVERLAY)
    );
    assert!(texts_in(&frame, WORLD).is_empty());
    let view = (frame.logical.w as f32, frame.logical.h as f32);
    let message_box = hud::message_box(view);
    assert!(
        solids_in(&frame, WORLD).contains(&message_box),
        "the box in the world texture: {message_box:?}"
    );
}

#[test]
fn the_spaceport_draws_no_world() {
    let store = MemoryPilots::new();
    let mut game = flying((1920, 1080), &store);
    game.tap(Key::Char('l'));
    game.settle_until("the dock approach's clearance", |game| {
        game.session().dock_cleared()
    });
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Spaceport);
    let frame = game.frame();
    assert_eq!(frame.world, [0_usize; 0]);
    assert!(!frame.batches.is_empty());
}
