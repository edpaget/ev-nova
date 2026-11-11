//! The new game's dock scan read on the message line, through the app over
//! the sample cut (`nova_data::new_game::fixture::sample_folder`), wired
//! as `main` wires it: a pilot carrying the gun mount kit, a lawless kit
//! (`sample_kits` buys it at Marrow Yard; here it is set on the saved
//! pilot), asks Luyten Exchange to land and is found, fined and denied.
//! The line keeps the find and the fine over the denial. The scan's rule
//! is pinned in nova-sim, the line's order in nova-view; this pins that
//! they meet.

use nova_data::new_game::fixture::sample_folder;
use nova_data::records::stellar::Stellar;
use nova_sim::fixture::MemoryPilots;
use nova_sim::{OutfitId, PilotStore, Vec2};
use winit::keyboard::KeyCode;

use crate::new_game_conversation::{app_over, new_pilot_flying, resumed_in};
use crate::new_game_play;
use crate::new_game_travel::nova_of;
use crate::pilots::{Harness, saved};

/// Ada, saved in the sample's start, Luyten Crossing, carrying the gun
/// mount kit, resumed in a new app over the sample.
fn carrying_the_kit() -> Harness {
    let store = MemoryPilots::new();
    drop(new_pilot_flying(app_over(&sample_folder(), &store)));
    let (_, game) = new_game_play::opened(&sample_folder());
    let mount = OutfitId(nova_of(&game.ids.kits, "gun-mount"));
    let mut pilot = saved(&store, "Ada");
    pilot.outfits.insert(mount, 1);
    store
        .write("Ada", &nova_sim::save::encode(&pilot))
        .expect("writes");
    resumed_in(app_over(&sample_folder(), &store))
}

/// Where the sample's stellar `id` sits.
fn position_of(id: &str) -> Vec2 {
    let (data, game) = new_game_play::opened(&sample_folder());
    let nova = nova_of(&game.ids.stellars, id);
    let stellar = data
        .get::<Stellar>(nova)
        .expect("present")
        .expect("decoded")
        .record
        .clone();
    Vec2::new(f32::from(stellar.x_pos), f32::from(stellar.y_pos))
}

fn cash(game: &Harness) -> i64 {
    game.app
        .screen()
        .flight_view()
        .expect("flying")
        .sim()
        .pilot()
        .expect("a pilot")
        .cash()
}

fn message(game: &Harness) -> Option<String> {
    game.app
        .screen()
        .flight_view()
        .expect("flying")
        .message()
        .map(str::to_owned)
}

#[test]
fn asking_luyten_exchange_to_land_with_the_kit_reads_the_find_and_the_fine() {
    let mut game = carrying_the_kit();
    let before = cash(&game);
    let at = position_of("luyten-exchange");
    game.app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            nova_sim::fixture::put_player_at(sim.session_mut().expect("a session"), at);
        });
    // The words are nova-view's, and the kit's name in them nova-data's
    // (its `LCName`); the find and the fine are named in them.
    let reads_find = |line: &Option<String>| {
        line.as_deref()
            .is_some_and(|line| line.to_lowercase().contains("gun mount") && line.contains("500"))
    };
    let mut presses = 0;
    while !reads_find(&message(&game)) {
        presses += 1;
        assert!(presses <= 3, "never scanned: {:?}", message(&game));
        game.press(KeyCode::KeyL);
        game.frame();
    }
    assert_eq!(cash(&game), before - 500, "fined");
    game.frame();
    assert!(
        reads_find(&message(&game)),
        "still read a frame on: {:?}",
        message(&game)
    );
}
