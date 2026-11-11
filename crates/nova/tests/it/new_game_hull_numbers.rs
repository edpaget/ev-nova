//! The new-game fixture's hull numbers reaching the ship that flies.
//!
//! - **A kit's acceleration.** Bought at Hearth Dock, the racked hauler
//!   picks up speed slower than the hauler it replaces, by its cargo
//!   racks' `acceleration` effect alone: the flown acceleration scales by
//!   the racked hull's `Accel` plus the racks' `MORE_ACCEL` over the
//!   hauler's `Accel`, each read from the data the app opened, so the
//!   fixture's numbers stay nova-data's.
//! - **A hull's shield recharge.** A new pilot's hauler, its shields put
//!   low, recharges them by its `shield_recharge: 300` a tick as the app
//!   runs frames, up to full and not past it. No ship in the fixture
//!   shoots the player, and a loaded pilot's shields are refilled as
//!   `_LoadPilotData` refills them, so nova-sim's
//!   `fixture::set_player_shield` puts them low. The fixture's hulls that
//!   name no recharge (the warden picket) aren't sold or started in, so
//!   the player can't fly one.
//!
//! What each format word maps to is nova-data's (`nova_data::new_game`),
//! and how a ship's stats work is nova-sim's, each pinned
//! there. This pins only that the two meet over the fixture as the app
//! opens it.

use nova_data::new_game::fixture::folder;
use nova_data::new_game::kits::mod_type;
use nova_data::records::outfit::Outfit;
use nova_data::records::ship::Ship;
use nova_data::{GameData, Registered};
use nova_sim::{Gauge, ShipId};

use crate::new_game_conversation::flying;
use crate::new_game_play::Play;
use crate::new_game_travel::nova_of;
use crate::pilots::Harness;

/// `id`'s record in the data the app opened.
fn record<T: Registered + Clone>(data: &GameData, id: i16) -> T {
    data.get::<T>(id)
        .expect("present")
        .expect("decoded")
        .record
        .clone()
}

/// The acceleration `kit`'s effects add, from its `MORE_ACCEL` slots.
fn accel_delta(kit: &Outfit) -> f32 {
    [
        (kit.mod_type, kit.mod_val),
        (kit.mod_type2, kit.mod_val2),
        (kit.mod_type3, kit.mod_val3),
        (kit.mod_type4, kit.mod_val4),
    ]
    .into_iter()
    .filter(|&(kind, _)| kind == mod_type::MORE_ACCEL)
    .map(|(_, val)| f32::from(val))
    .sum()
}

#[test]
fn the_racked_hauler_s_racks_take_their_acceleration_off_the_hull_s() {
    let mut play = Play::new(&folder(), "hearth", |_| {});
    play.land("hearth-dock");
    let hauler = play.session.stats().handling.accel;
    let ids = &play.game.ids;
    let hull = f32::from(record::<Ship>(&play.data, nova_of(&ids.ships, "hauler")).accel);
    let racked_hull =
        f32::from(record::<Ship>(&play.data, nova_of(&ids.ships, "racked-hauler")).accel);
    let delta = accel_delta(&record::<Outfit>(
        &play.data,
        nova_of(&ids.kits, "cargo-racks"),
    ));
    assert!(delta != 0.0, "the racks change the acceleration");
    let racked = ShipId(nova_of(&ids.ships, "racked-hauler"));
    play.session
        .buy_ship(racked, "Rack", &mut play.chance)
        .expect("bought");
    let racks = play.session.stats().handling.accel;
    let expected = hauler * (racked_hull + delta) / hull;
    assert!(
        (racks - expected).abs() < expected.abs() * 1e-4,
        "the hauler's {hull}, the racked hull's {racked_hull} and the racks' {delta}: \
         {hauler} then {racks}, not {expected}"
    );
}

/// Sets the flying ship's shield points to `now` through nova-sim's
/// published helper: no ship in the fixture shoots the player, and a
/// loaded pilot's shields are refilled, as `_LoadPilotData` refills them.
fn set_shield(game: &mut Harness, now: f32) {
    game.app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            nova_sim::fixture::set_player_shield(sim.session_mut().expect("a session"), now);
        });
}

/// The flying ship's shield gauge.
fn shield(game: &Harness) -> Gauge {
    game.ship_session().reserves().shield
}

#[test]
#[allow(clippy::float_cmp)] // a frame with no tick leaves the gauge exactly as it was
fn the_hauler_s_shields_recharge_by_its_hull_s_rate_tick_over_tick_up_to_full() {
    let mut game = flying();
    let full = shield(&game);
    assert_eq!(full.now, full.max, "a new pilot's hauler flies full");
    let session = game.ship_session();
    // The hauler's `shield_recharge: 300`, as the ship flies it, at the
    // frame's gain: what each engine tick adds.
    let step = session.stats().shield_regen * session.speed_mult().as_f32();
    set_shield(&mut game, 10.0);
    let mut last = shield(&game).now;
    let mut ticks = 0;
    for _ in 0..60 {
        game.frame();
        let now = shield(&game).now;
        if now == last {
            continue;
        }
        let risen = now - last;
        let whole = (risen / step).round();
        assert!(
            whole >= 1.0 && (risen - whole * step).abs() < 1e-3,
            "{last} to {now}: whole ticks of the hull's {step}"
        );
        ticks += whole as u32;
        last = now;
    }
    assert!(ticks > 0, "the shields recharge: still at {last}");
    assert!(
        (last - (10.0 + ticks as f32 * step)).abs() < 1e-3,
        "{ticks} ticks of {step} from 10: {last}"
    );

    set_shield(&mut game, full.max - step / 2.0);
    for _ in 0..10 {
        game.frame();
        assert!(shield(&game).now <= full.max, "{:?}", shield(&game));
    }
    assert_eq!(shield(&game), full, "full, and not past it");
}
