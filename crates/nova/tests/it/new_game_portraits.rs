//! The new game's ships and stellars drawn as their models outside the
//! flight's world (rdm:phase/rendering-3d/phase-8-models-in-other-screens):
//! over the 3D sample (`nova_data::new_game::fixture::modelled_sample_3d`),
//! wired as `main` wires it (the world models' table on the router, their
//! meshes on the app), the status bar's target panel draws the target's
//! class's model in its picture's place, in the overlay.
//!
//! Wiring only: what each screen draws, where and how big, is
//! nova-view's (its `model_picture` module and each screen's), the table's
//! keys and radii nova-render's (`new_game_world`), each pinned there.
//! This pins that the data, the edge, the app and the screens meet.

use std::path::Path;

use nova::chance::SplitMix;
use nova::ruleset::Ruleset;
use nova_data::new_game::{self, NewGame};
use nova_render::{WorldAssets, world_assets};
use nova_sim::TargetPick;
use nova_sim::fixture::{MemoryPilots, place, put_player_at};
use nova_sim::{ShipRef, Vec2};
use nova_view::{DrawCommand, DrawLayer, DrawList, ModelKey, Screen};

use crate::common::draw;
use crate::new_game_models::{flying_over, frame};
use crate::pilots::Harness;

/// The 3D sample, loaded, and its world models.
fn sample() -> (NewGame, WorldAssets) {
    let source = new_game::fixture::modelled_sample_3d();
    let game = new_game::load(&source, Path::new("sample"));
    let assets = world_assets(&game, &source);
    assert_eq!(assets.problems, Vec::<String>::new());
    (game, assets)
}

/// The models the screen drawn now draws in the overlay, in order.
fn overlay_models(game: &Harness) -> Vec<ModelKey> {
    let mut list = DrawList::new();
    game.app.screen().draw(&mut list);
    list.layered()
        .filter_map(|(layer, command)| match command {
            DrawCommand::Model { model, .. } if layer == DrawLayer::Overlay => Some(*model),
            _ => None,
        })
        .collect()
}

#[test]
fn the_target_panel_draws_the_target_s_class_as_its_model() {
    let (game, assets) = sample();
    let data = std::rc::Rc::new(game.data);
    let pilots = MemoryPilots::new();
    let mut harness = flying_over(Ruleset::NewGame, &data, &assets, &pilots);
    let target = harness
        .app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            let session = sim.session_mut().expect("a session");
            let npc = (0..200)
                .find_map(|seed| {
                    if session.npcs().is_empty() {
                        session.populate(&data, &mut SplitMix::new(seed));
                    }
                    session.npcs().first().map(|npc| npc.id)
                })
                .expect("a ship to target");
            put_player_at(session, Vec2::ZERO);
            place(session, npc, Vec2::new(60.0, -100.0));
            assert_eq!(session.select_target(TargetPick::Nearest), Some(npc));
            npc
        });
    draw(&mut harness);
    let class = frame(&harness)
        .ship(ShipRef::Npc(target))
        .expect("the target")
        .class;
    let key = assets.models.ship(class).expect("the class's model").key;
    let drawn = overlay_models(&harness);
    assert!(drawn.contains(&key), "the target's portrait: {drawn:?}");
}
