//! Flight's world layer through the whole app, over the persons' game
//! ([`crate::persons`]): flight's frame reaches the Gpu as its world layer
//! then an overlay, in a 4:3 and a 16:9 window, a hail's comm dialog is
//! drawn over flight's world (natively and under FSR 1), under each
//! texture upscaler (sharp-bilinear, FSR 1) the message
//! line's box goes into the world texture and its text stays overlay, the
//! world's tail starts between them so the text draws in the target, and
//! natively the tail moves nothing out of it, and the spaceport draws no
//! world. What goes in which layer, and where the tail starts, is
//! nova-view's, pinned there.

use nova::app::Showing;
use nova_render::{Batch, Frame, QuadInstance, SolidQuad, Upscaler, WorldSpace};
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
    // Natively, and under FSR 1, which draws the world into a texture.
    for upscaler in [None, Some(Upscaler::Fsr1)] {
        let store = MemoryPilots::new();
        let mut game = flying((1920, 1080), &store);
        if let Some(upscaler) = upscaler {
            game.upscale_with(upscaler);
        }
        for key in [Key::Tab, Key::Tab, Key::KeyY] {
            game.tap(key);
            game.settle();
        }
        assert_eq!(game.showing(), Showing::Comm, "{upscaler:?}");
        let frame = game.frame();
        if upscaler.is_some() {
            assert!(
                matches!(frame.world_space, WorldSpace::Texture { .. }),
                "{:?}",
                frame.world_space
            );
        }
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
            assert!(overlay.contains(text), "{upscaler:?} {text}: {overlay:?}");
        }
        assert!(texts_in(&frame, WORLD).is_empty(), "{upscaler:?}");
        assert!(
            frame.in_layer(WORLD).next().is_some(),
            "flight's world beneath"
        );
    }
}

/// The area a solid quad covers: its corners' bounds.
fn solid_area(quad: &SolidQuad) -> Bounds {
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
}

/// The areas the solid quads drawn in `layer` cover.
fn solids_in(frame: &Frame, layer: DrawLayer) -> Vec<Bounds> {
    frame
        .in_layer(layer)
        .flat_map(|batch| match batch {
            Batch::Solid(quads) => quads.iter().map(solid_area).collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// The game in flight in a 1024x768 window, upscaled with `upscaler`
/// (none: native) and with a message on the line, and that message.
fn with_a_message(store: &MemoryPilots, upscaler: Option<Upscaler>) -> (Game, String) {
    let mut game = flying((1024, 768), store);
    if let Some(upscaler) = upscaler {
        game.upscale_with(upscaler);
    }
    game.tap(Key::KeyL);
    game.settle_until("a message on the line", |game| {
        game.app
            .screen()
            .flight_view()
            .expect("flying")
            .message()
            .is_some()
    });
    let message = game
        .app
        .screen()
        .flight_view()
        .expect("flying")
        .message()
        .expect("a message on the line")
        .to_owned();
    (game, message)
}

/// The upscalers that draw flight's world into a texture on every device.
const TEXTURE_UPSCALERS: [Upscaler; 2] = [Upscaler::SharpBilinear, Upscaler::Fsr1];

#[test]
fn under_a_texture_upscaler_flights_message_text_is_overlay_and_its_box_world() {
    for upscaler in TEXTURE_UPSCALERS {
        let store = MemoryPilots::new();
        let (mut game, message) = with_a_message(&store, Some(upscaler));
        let frame = game.frame();
        assert!(
            matches!(frame.world_space, WorldSpace::Texture { .. }),
            "{upscaler:?}: {:?}",
            frame.world_space
        );
        assert!(
            texts_in(&frame, OVERLAY).contains(&message),
            "{upscaler:?} {message}: {:?}",
            texts_in(&frame, OVERLAY)
        );
        assert!(texts_in(&frame, WORLD).is_empty(), "{upscaler:?}");
        let view = (frame.logical.w as f32, frame.logical.h as f32);
        let message_box = hud::message_box(view);
        assert!(
            solids_in(&frame, WORLD).contains(&message_box),
            "{upscaler:?}: the box in the world texture: {message_box:?}"
        );
    }
}

#[test]
fn the_spaceport_draws_no_world() {
    let store = MemoryPilots::new();
    let mut game = flying((1920, 1080), &store);
    game.tap(Key::KeyL);
    game.settle_until("the dock approach's clearance", |game| {
        game.session().dock_cleared()
    });
    game.tap(Key::KeyL);
    assert_eq!(game.showing(), Showing::Spaceport);
    let frame = game.frame();
    assert_eq!(frame.world, [0_usize; 0]);
    assert!(!frame.batches.is_empty());
}

#[test]
fn under_a_texture_upscaler_flights_floaters_draw_in_the_target_from_the_world_tail() {
    for upscaler in TEXTURE_UPSCALERS {
        let store = MemoryPilots::new();
        let (mut game, message) = with_a_message(&store, Some(upscaler));
        let frame = game.frame();
        let view = (frame.logical.w as f32, frame.logical.h as f32);
        let message_box = hud::message_box(view);
        let box_at = (0..frame.batches.len())
            .find(|&at| {
                frame.layer(at) == WORLD
                    && matches!(&frame.batches[at], Batch::Solid(quads)
                        if quads.iter().any(|quad| solid_area(quad) == message_box))
            })
            .expect("the message's box");
        let text_at = (0..frame.batches.len())
            .find(|&at| {
                matches!(&frame.batches[at], Batch::Text(runs)
                    if runs.iter().any(|run| run.text == message))
            })
            .expect("the message's text");
        assert!(
            frame
                .world_tail
                .is_some_and(|tail| box_at < tail && tail <= text_at),
            "box {box_at}, text {text_at}, tail {:?}",
            frame.world_tail
        );
        assert!(
            matches!(frame.space_of(box_at), WorldSpace::Texture { .. }),
            "{:?}",
            frame.space_of(box_at)
        );
        assert_eq!(frame.space_of(text_at), WorldSpace::Target);
    }
}

#[test]
fn natively_flights_world_tail_moves_no_batch_out_of_the_target() {
    let store = MemoryPilots::new();
    let (mut game, _) = with_a_message(&store, None);
    let frame = game.frame();
    assert_eq!(frame.world_space, WorldSpace::Target);
    assert!(frame.world_tail.is_some());
    for at in 0..frame.batches.len() {
        assert_eq!(frame.space_of(at), WorldSpace::Target, "batch {at}");
    }
}
