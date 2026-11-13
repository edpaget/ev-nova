//! A world model the flight draws with nothing registered for it is
//! reported: over the 3D sample
//! (`nova_data::new_game::fixture::modelled_sample_3d`), wired as `main`
//! wires it but with Luyten Exchange's mesh and Kestrel Reach's planet
//! withheld from the app while the table still names their keys, the app
//! keeps one warning for each key over the frames the flight draws, and
//! with the assets whole it keeps none.
//!
//! Wiring only: that the keys are listed once is nova-render's, and the
//! lines' wording and that the app keeps them the app's own (its
//! `missing_keys` module), each pinned there. This pins that the flight's
//! frames, the renderer's report and the app's warnings meet.

use std::rc::Rc;

use nova::ruleset::Ruleset;
use nova_sim::StellarId;
use nova_sim::fixture::MemoryPilots;
use nova_view::flight::StellarModel;

use crate::common::draw;
use crate::new_game_models::{flying_over, nova_id, sample};
use crate::pilots::Harness;

/// The warnings `game` kept that name a key drawn but not registered.
fn missing_key_lines(game: &mut Harness) -> Vec<String> {
    game.app
        .take_warnings()
        .into_iter()
        .filter(|line| {
            line.starts_with("nova: model key ") || line.starts_with("nova: planet key ")
        })
        .collect()
}

#[test]
fn a_withheld_mesh_and_planet_are_each_reported_once_over_the_flight_s_frames() {
    let (game, mut assets) = sample();
    let exchange = StellarId(nova_id(&game.ids.stellars, "luyten-exchange"));
    let reach = StellarId(nova_id(&game.ids.stellars, "kestrel-reach"));
    let Some(StellarModel::Model { key: hub, .. }) = assets.models.stellar(exchange) else {
        panic!("the exchange's model");
    };
    let Some(StellarModel::Planet { key: giant, .. }) = assets.models.stellar(reach) else {
        panic!("Kestrel Reach's planet");
    };
    assets.meshes.retain(|(key, _)| *key != hub);
    assets.planets.retain(|(key, _)| *key != giant);
    let pilots = MemoryPilots::new();
    let mut harness = flying_over(Ruleset::NewGame, &Rc::new(game.data), &assets, &pilots);
    for _ in 0..5 {
        draw(&mut harness);
    }

    assert_eq!(
        missing_key_lines(&mut harness),
        [
            format!(
                "nova: model key {} is drawn but the renderer was given no mesh for it: it draws nothing",
                hub.0
            ),
            format!(
                "nova: planet key {} is drawn but the renderer was given no planet for it: it draws nothing",
                giant.0
            ),
        ]
    );
    for _ in 0..3 {
        draw(&mut harness);
    }
    assert_eq!(missing_key_lines(&mut harness), Vec::<String>::new());
}

#[test]
fn a_flight_whose_models_are_all_registered_reports_no_missing_key() {
    let (game, assets) = sample();
    let pilots = MemoryPilots::new();
    let mut harness = flying_over(Ruleset::NewGame, &Rc::new(game.data), &assets, &pilots);
    for _ in 0..5 {
        draw(&mut harness);
    }
    assert_eq!(missing_key_lines(&mut harness), Vec::<String>::new());
}
