//! The new-game fixture's hull numbers reaching the ship that flies.
//!
//! - **A kit's acceleration.** Bought at Hearth Dock, the racked hauler
//!   picks up speed slower than the hauler it replaces, by its cargo
//!   racks' `acceleration` effect alone: the two hulls share their own.
//!
//! What each format word maps to is nova-data's (`nova_data::new_game`),
//! and how a ship's stats work is nova-sim's, each pinned
//! there. This pins only that the two meet over the fixture as the app
//! opens it.

use nova_data::new_game::fixture::folder;
use nova_sim::ShipId;

use crate::new_game_play::Play;
use crate::new_game_travel::nova_of;

#[test]
fn the_racked_hauler_s_racks_take_their_acceleration_off_the_hull_s() {
    let mut play = Play::new(&folder(), "hearth", |_| {});
    play.land("hearth-dock");
    let hauler = play.session.stats().handling.accel;
    let racked = ShipId(nova_of(&play.game.ids.ships, "racked-hauler"));
    play.session
        .buy_ship(racked, "Rack", &mut play.chance)
        .expect("bought");
    let racks = play.session.stats().handling.accel;
    let expected = hauler * 310.0 / 360.0;
    assert!(
        (racks - expected).abs() < expected * 1e-4,
        "the hauler's 360 less the racks' 50: {hauler} then {racks}, not {expected}"
    );
}
