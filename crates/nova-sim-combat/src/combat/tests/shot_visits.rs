//! `_HandleShot` once per shot slot, in slot order (`_DoPlayGameWork`
//! @0x4435e-0x44393; rdm weapons-fidelity phase 47): each spent shot's
//! end of life releases its sub-munitions (@0x35f49) and then lands its
//! blast (@0x36106) at its own visit, and each live shot moves and makes
//! its missile lock (@0x35855) at its own. So a later slot's visit reads
//! what an earlier slot's landing left, an escort's cleared leader
//! included, and the strikes land in slot order.

use super::escort_lost_in_place::{
    E, X, cluster_seeking, escort, mission_escort, player_at, seekers_of,
};
use super::*;

/// A flak shell of the player's, detonating at the end of its life with
/// a 40-pixel blast of the blaster's 4 mass and 10 energy, exploding.
fn flak() -> WeaponRecord {
    WeaponRecord {
        count: 3,
        speed: 1000,
        blast_radius: 40,
        flags: 0x8000,
        explod_type: 1,
        ..blaster()
    }
}

/// A shot of the player's of `record` whose last move, ending at `at`,
/// spent it, aimed at nothing.
fn spent(record: WeaponRecord, at: Vec2) -> Shot {
    let mut shot = Shot::launch(
        WeaponSpec::new(&record),
        ShipRef::Player,
        ShipRef::Player,
        &ShipState::default(),
        90.0,
    );
    shot.position = at;
    shot.moved_from = Some(at - Vec2::new(10.0, 0.0));
    shot.age = shot.weapon.lifetime;
    shot
}

/// The player's homing missile at (-500, -100), aimed at `at`, whose next
/// move takes its age past `LockAge`.
fn locking_on(at: ShipRef) -> Shot {
    let mut shot = Shot::launch(
        WeaponSpec::new(&missile()),
        ShipRef::Player,
        ShipRef::Player,
        &ShipState {
            position: Vec2::new(-500.0, -100.0),
            ..ShipState::default()
        },
        90.0,
    );
    shot.moved_from = Some(shot.position);
    shot.age = projectile::LOCK_AGE;
    shot.aimed_at = Some(at);
    shot.target = Some(at);
    shot
}

/// Where the flak goes off: 30 pixels left of E at (0, -100), 130 from X.
const BESIDE_E: Vec2 = Vec2::new(-30.0, -100.0);

/// Where the seeking shell ends: 40 pixels right of E, 60 from X.
const NEARER_E: Vec2 = Vec2::new(40.0, -100.0);

/// The player at (0, 300), X at (100, -100) and `e`.
fn among(combat: &mut Combat, e: Ship) -> [Ship; 3] {
    [player_at(combat, 0.0, 300.0), Ship::at(2, 100.0, -100.0), e]
}

/// `_HandleShot`'s pass alone among `ships`, sub-munitions read from
/// `arsenal`: the strikes it made, as (ship, lock).
fn handle(combat: &mut Combat, ships: &mut [Ship], arsenal: &Arsenal) -> Vec<(ShipRef, bool)> {
    let govts = Governments::default();
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.fly_shots(&mut fighters, arena, &mut NeverFires);
    combat
        .take_strikes()
        .iter()
        .map(|strike| (strike.ship, strike.lock))
        .collect()
}

/// The targets of the seeking shell's sub-munitions (weapon 148).
fn sub_targets(combat: &Combat) -> Vec<Option<ShipRef>> {
    combat
        .shots()
        .iter()
        .filter(|shot| shot.weapon.id == WeaponId(148))
        .map(|shot| shot.target)
        .collect()
}

/// The flak in slot 0 going off beside `e`, then the seeking shell in
/// slot 1 ending nearer `e` than X: what its sub-munitions took.
fn after_the_flak(e: Ship) -> Vec<Option<ShipRef>> {
    let (shell, arsenal) = cluster_seeking();
    let mut combat = Combat::default();
    let mut ships = among(&mut combat, e);
    combat.put_in_flight(spent(flak(), BESIDE_E));
    combat.put_in_flight(spent(shell, NEARER_E));
    let struck = handle(&mut combat, &mut ships, &arsenal);
    assert_eq!(struck, [(E, false)], "the flak's blast");
    assert_eq!(ships[2].condition, Condition::Disabled);
    sub_targets(&combat)
}

#[test]
fn a_later_slots_sub_munitions_read_an_escort_an_earlier_slots_blast_unled() {
    // Slot 0's blast lands at its own visit (@0x36106) and clears E's
    // leader (@0x3afdd), before slot 1's end of life seeks (@0x35f49).
    assert_eq!(after_the_flak(escort()), [Some(E), Some(E)], "unled");
    assert_eq!(
        after_the_flak(mission_escort()),
        [Some(X), Some(X)],
        "a mission escort stays led"
    );
}

#[test]
fn a_shots_own_sub_munitions_seek_before_its_blast_lands() {
    // One shell both blasts and releases: `_SpawnShotSubmunitions`
    // (@0x35f49) comes before the blast's `_DamageShip` (@0x36106), so its
    // own sub-munitions read E still led.
    let (shell, _) = cluster_seeking();
    let shell = WeaponRecord {
        blast_radius: 40,
        flags: 0x8000,
        mass_dmg: 4,
        ..shell
    };
    let arsenal = seekers_of(shell);
    let mut combat = Combat::default();
    let mut ships = among(&mut combat, escort());
    combat.put_in_flight(spent(shell, BESIDE_E));
    let struck = handle(&mut combat, &mut ships, &arsenal);
    assert_eq!(struck, [(E, false)]);
    assert_eq!(ships[2].condition, Condition::Disabled, "the blast lost E");
    assert_eq!(sub_targets(&combat), [Some(X), Some(X)]);
}

/// The player's missile locking on `e` and the flak going off beside
/// it, the missile in the lower slot when `lock_first`: the strikes.
fn lock_and_blast(e: Ship, lock_first: bool) -> Vec<(ShipRef, bool)> {
    let mut combat = Combat::default();
    let mut ships = among(&mut combat, e);
    let (lower, upper) = if lock_first {
        (locking_on(E), spent(flak(), BESIDE_E))
    } else {
        (spent(flak(), BESIDE_E), locking_on(E))
    };
    combat.put_in_flight(lower);
    combat.put_in_flight(upper);
    handle(&mut combat, &mut ships, &Arsenal::default())
}

#[test]
fn a_lock_reads_the_leader_before_a_later_slots_blast_clears_it() {
    // The lock tests the target's leader at its own visit (@0x35773).
    assert_eq!(lock_and_blast(escort(), true), [(E, false)], "still led");
    assert_eq!(
        lock_and_blast(escort(), false),
        [(E, false), (E, true)],
        "the blast lost E first"
    );
}

#[test]
fn the_shots_strikes_land_in_slot_order() {
    // Each lock lands at its own visit (@0x35855), between the slots'
    // ends of life, so the strikes come in slot order.
    let npc = || Ship::at(3, 0.0, -100.0);
    assert_eq!(lock_and_blast(npc(), true), [(E, true), (E, false)]);
    assert_eq!(lock_and_blast(npc(), false), [(E, false), (E, true)]);
}

#[test]
fn a_release_is_heard_before_its_shells_explosion() {
    // `_SpawnShotSubmunitions` (@0x35f49, its sound @0x3f2ad) comes before
    // `_CreateExplosion` (@0x35f97).
    let (shell, _) = cluster_seeking();
    let shell = WeaponRecord {
        blast_radius: 40,
        flags: 0x8000,
        explod_type: 1,
        ..shell
    };
    let arsenal = seekers_of(shell);
    let mut combat = Combat::default();
    let mut ships = among(&mut combat, escort());
    combat.put_in_flight(spent(shell, BESIDE_E));
    handle(&mut combat, &mut ships, &arsenal);
    let heard: Vec<&str> = combat
        .take_events()
        .iter()
        .filter_map(|event| match event {
            CombatEvent::Fired { .. } => Some("fired"),
            CombatEvent::Exploded { .. } => Some("exploded"),
            _ => None,
        })
        .collect();
    assert_eq!(heard, ["fired", "exploded"]);
}

#[test]
fn every_visits_blast_meets_a_struck_ship_where_it_stood() {
    // `_HandleShot`'s blasts read each ship where its last `_HandleShip`
    // left it (@0x35fd5-0x36141): N stood at (300, -100) as the last tick
    // ran and has since moved to (0, -100). Slot 0's blast strikes it
    // where it stood, and so does slot 1's, after N is read again.
    let mut combat = Combat::default();
    let n = ShipRef::Npc(NpcId(3));
    combat.stood.insert(n, Vec2::new(300.0, -100.0));
    let mut ships = [player_at(&mut combat, 0.0, 300.0), Ship::at(3, 0.0, -100.0)];
    combat.put_in_flight(spent(flak(), Vec2::new(270.0, -100.0)));
    combat.put_in_flight(spent(flak(), Vec2::new(330.0, -100.0)));
    let struck = handle(&mut combat, &mut ships, &Arsenal::default());
    assert_eq!(struck, [(n, false), (n, false)]);
}
