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

/// [`turret`] doing no mass damage, which passes over a disabled ship.
fn massless() -> WeaponRecord {
    WeaponRecord {
        mass_dmg: 0,
        ..turret()
    }
}

/// A pirate, NPC `id`, `distance` above the stellar.
fn pirate(id: u32, distance: f32) -> Ship {
    Ship {
        govt: Some(PIRATES),
        ..Ship::at(id, 0.0, -distance)
    }
}

/// The ship [`stellar`]'s gun, armed with `record`, aims its shot at among
/// `ships`, if it fires.
fn aimed_at(ships: &mut [Ship], record: WeaponRecord) -> Option<ShipRef> {
    let arsenal = Arsenal::new(&[record], Vec::new());
    let govts = govts();
    let mut combat = Combat::default();
    let fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    combat.fire_stellar_guns(&fighters, arena, &mut [stellar()], &mut NeverFires);
    combat.shots().first().and_then(|shot| shot.aimed_at)
}

#[test]
fn the_gun_passes_over_a_fighter_cloaked_disabled_jumping_in_or_destroyed() {
    let near = || pirate(1, 100.0);
    let far = || pirate(2, 300.0);
    assert_eq!(
        aimed_at(&mut [near(), far()], massless()),
        Some(A),
        "nearer"
    );
    let breaking_up = || Ship {
        condition: Condition::Dying {
            ticks_left: 0,
            starting: true,
        },
        ..near()
    };
    let held_disabled = || Ship {
        held_disabled: true,
        ..near()
    };
    for (state, decoy) in [("breaking up", breaking_up()), ("held", held_disabled())] {
        assert_eq!(
            aimed_at(&mut [decoy, far()], turret()),
            Some(A),
            "{state}: an ordinary gun fires at a disabled ship"
        );
    }
    let passed_over = [
        (
            "cloaked",
            Ship {
                cloaked: true,
                ..near()
            },
            turret(),
        ),
        ("breaking up", breaking_up(), massless()),
        ("held disabled", held_disabled(), massless()),
        (
            "jumping in",
            Ship {
                jumping_in: true,
                ..near()
            },
            turret(),
        ),
        (
            "destroyed",
            Ship {
                condition: Condition::Destroyed,
                ..near()
            },
            turret(),
        ),
    ];
    for (state, decoy, record) in passed_over {
        assert_eq!(aimed_at(&mut [decoy, far()], record), Some(B), "{state}");
    }
}
