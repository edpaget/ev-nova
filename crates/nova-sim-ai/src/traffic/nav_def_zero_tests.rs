//! Where a stationary warship is placed: `_RandomShipSpawn` reads slot 0
//! of the system's `NavDefs` (@0x3c274-0x3c27d) and places the ship by that
//! `spöb`'s record (+4/+6, @0x3c2a9-0x3c2f8), owner-, shown- and
//! destroyed-blind; with slot 0 empty it falls to the box (@0x3c32b),
//! whatever later slots hold. The traffic reads the slot from
//! [`World::nav_def_0`] and the place from [`World::listed`].

use super::World;
use crate::catalog::{DudeId, LandingSite, ShipId, StellarId};
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::stats::ShipStats;
use crate::traffic::Traffic;
use crate::traffic::spawn::STATIONARY_DISTANCE;
use crate::traffic::table::{ShipKind, SpawnDude, SpawnTable};
use nova_sim_base::fixture::{FAST, Ranged};
use nova_sim_catalog::fixture::planet;
use std::collections::BTreeMap;

/// One ship on average: düde 128 flying ship 201, a warship (`InherentAI`
/// 3) of no speed.
fn still_warships() -> SpawnTable {
    SpawnTable {
        avg_ships: 1,
        dudes: vec![(DudeId(128), 100)],
        dude_records: BTreeMap::from([(
            DudeId(128),
            SpawnDude {
                ai_type: 0,
                govt: None,
                ships: vec![(ShipId(201), 100)],
                booty: 0,
                info_types: 0,
            },
        )]),
        ships: BTreeMap::from([(
            ShipId(201),
            ShipKind {
                stats: ShipStats::new(ShipFields { speed: 0, ..FAST }, &[]),
                inherent_ai: 3,
                ..ShipKind::default()
            },
        )]),
        ..SpawnTable::default()
    }
}

/// The ship a system set up in `world` holds, each angle drawn 90 and each
/// box draw 1499.
fn placed(world: &World) -> crate::traffic::npc::Npc {
    let mut traffic = Traffic::new();
    traffic.enter_in(still_warships(), *world, &mut Ranged::of(&[(360, 90)]));
    let [ship] = traffic.npcs() else {
        panic!("one ship: {:?}", traffic.npcs());
    };
    ship.clone()
}

/// A, shown, 600 below the centre, and B, 600 above it.
fn stellars() -> (LandingSite, LandingSite) {
    (planet(128, 0.0, 600.0), planet(140, 0.0, -600.0))
}

#[test]
fn a_stationary_warship_is_placed_by_nav_def_0_though_it_is_not_shown() {
    let (a, b) = stellars();
    let (sites, listed) = ([a], [b, a]);
    let ship = placed(&World {
        listed: &listed,
        nav_def_0: Some(StellarId(140)),
        ..World::new(&sites)
    });
    // Angle 90 is along +x (`spawn`'s own tests pin the angle).
    let off = ship.state.position - (b.position + Vec2::new(STATIONARY_DISTANCE, 0.0));
    assert!(off.length() < 1e-3, "{:?}", ship.state);
    assert_eq!(ship.state.velocity, Vec2::ZERO, "at rest");
}

#[test]
fn with_slot_0_empty_a_stationary_warship_is_placed_in_the_box() {
    let (a, b) = stellars();
    let (sites, listed) = ([a], [a, b]);
    let ship = placed(&World {
        listed: &listed,
        nav_def_0: None,
        ..World::new(&sites)
    });
    // `Rand(1500) - 750` on each axis, at its top speed of none.
    assert_eq!(ship.state.position, Vec2::new(749.0, 749.0));
    assert_eq!(ship.state.velocity, Vec2::ZERO);
}
