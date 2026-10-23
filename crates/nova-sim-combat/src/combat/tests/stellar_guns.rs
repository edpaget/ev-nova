//! The fight's tests of the stellar guns
//! ([`stellar_guns`](crate::combat::stellar_guns)): a stellar's shot,
//! launched by no ship (source -1), hits only the ship it is aimed at
//! (`_ShotCanHitShip` @0x44a71-0x44a88), as the fight's later passes fly
//! and test it.

use super::*;
use crate::catalog::{GovtRecord, StellarId};
use crate::combat::phases::Arena;
use crate::combat::stellar_guns::{GunTimer, Standing, StellarGun};
use nova_sim_catalog::fixture::govt;

const FEDERATION: GovtId = GovtId(128);
const PIRATES: GovtId = GovtId(129);

/// The Federation (class 1) is the pirates' (class 2) enemy.
fn govts() -> Governments {
    let federation = GovtRecord {
        classes: [1, -1, -1, -1],
        enemies: [2, -1, -1, -1],
        ..govt(128)
    };
    let pirates = GovtRecord {
        classes: [2, -1, -1, -1],
        ..govt(129)
    };
    Governments::new([federation, pirates])
}

/// A turret of 15 pixels a tick for 40 ticks, doing 4 mass and 10
/// energy damage.
fn turret() -> WeaponRecord {
    WeaponRecord {
        reload: 100,
        count: 40,
        speed: 1500,
        mass_dmg: 4,
        energy_dmg: 10,
        ..weapon(196)
    }
}

/// A Federation stellar at the origin, armed with [`turret`], the player
/// in good standing.
fn stellar() -> StellarGun {
    StellarGun {
        stellar: StellarId(128),
        position: Vec2::ZERO,
        weapon: WeaponId(196),
        govt: Some(FEDERATION),
        flags2: 0,
        dominated: false,
        defended: false,
        standing: Standing {
            govt: Some(FEDERATION),
            ..Standing::default()
        },
        timer: GunTimer::default(),
    }
}

#[test]
fn a_stellars_shot_passes_through_a_ship_in_its_way_and_strikes_its_target_by_no_one() {
    let arsenal = Arsenal::new(&[turret()], Vec::new());
    let govts = govts();
    let mut ships = vec![
        Ship {
            govt: Some(PIRATES),
            ..Ship::at(1, 0.0, -200.0)
        },
        Ship::at(2, 0.0, -100.0),
    ];
    let mut combat = Combat::default();
    let mut guns = [stellar()];
    {
        let fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        let arena = Arena {
            pass: GamePass::Advance { double_time: false },
            arsenal: &arsenal,
            govts: &govts,
            rules: Rules::default(),
            field: Field::default(),
        };
        combat.fire_stellar_guns(&fighters, arena, &mut guns, &mut NeverFires);
    }
    assert_eq!(combat.shots().len(), 1);
    assert_eq!(combat.shots()[0].aimed_at, Some(A), "at the pirate, not B");
    let mut strikes = Vec::new();
    for _ in 0..20 {
        tick_among(&mut combat, &mut ships, &arsenal, &govts, Rules::default());
        strikes.extend(combat.take_strikes());
    }
    assert!(
        strikes.iter().all(|strike| strike.ship != B),
        "passes through B: {strikes:?}"
    );
    let on_a: Vec<&Strike> = strikes.iter().filter(|strike| strike.ship == A).collect();
    assert_eq!(on_a.len(), 1, "{strikes:?}");
    assert_eq!(on_a[0].by, None, "by no one");
    assert!(combat.shots().is_empty(), "spent on A");
}
