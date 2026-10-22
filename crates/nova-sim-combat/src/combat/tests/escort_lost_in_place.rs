//! A ship of the player's fleet leaves it as the hit that leaves it
//! disabled lands (`_DamageShip` @0x3af8f-0x3afdd, rdm weapons-fidelity
//! phase 42), so every later test of the tick reads it unled: the rest of
//! the collision walk (`_ShotCanHitShip`'s rules 6a and 10), a seeking
//! sub-munition's pick (`_FindNearestValidShotTarget` @0x6a3a), and a later
//! beam's filter (`_HandleBeams` @0x30ba0-0x30c05, its hit landed in its
//! loop @0x316a1).

use super::collision_walk::{INTO_C, fly, missile, walk, walk_amid};
use super::*;
use crate::combat::escort_loss::leave_fleet_if_disabled;
use crate::combat::weapon::SUBS_SEEK_NEAREST;

pub(super) const E: ShipRef = ShipRef::Npc(NpcId(3));
pub(super) const X: ShipRef = ShipRef::Npc(NpcId(2));

/// E, a ship of the player's fleet in the Led layer at (0, -100): its
/// shield down and its armour 11 of 30, one point above the line a
/// blaster's 4 mass takes it under.
pub(super) fn escort() -> Ship {
    let mut ship = Ship::at(3, 0.0, -100.0);
    ship.leader = Some(ShipRef::Player);
    ship.fleet = ShipRef::Player;
    ship.fleet_escort = true;
    ship.reserves.shield.now = 0.0;
    ship.reserves.armor.now = 11.0;
    ship.sprite = Some(SpriteRank {
        layer: ShipLayer::Led,
        entry: 0,
    });
    ship
}

/// The player at (`x`, `y`), standing there for the contact passes.
pub(super) fn player_at(combat: &mut Combat, x: f32, y: f32) -> Ship {
    combat.stood.insert(ShipRef::Player, Vec2::new(x, y));
    Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        ..Ship::at(0, x, y)
    }
}

#[test]
fn an_escort_a_hit_leaves_disabled_leaves_its_leader() {
    let mut ship = escort();
    ship.reserves.armor.now = 9.0;
    let mut fighter = ship.fighter();
    assert!(leave_fleet_if_disabled(&mut fighter, &NovaDisable));
    assert_eq!((fighter.leader, fighter.fleet), (None, E));
}

#[test]
fn an_escort_still_intact_keeps_its_leader() {
    let mut ship = escort();
    let mut fighter = ship.fighter();
    assert!(!leave_fleet_if_disabled(&mut fighter, &NovaDisable));
    assert_eq!(
        (fighter.leader, fighter.fleet),
        (Some(ShipRef::Player), ShipRef::Player)
    );
}

#[test]
fn a_mission_escort_disabled_keeps_its_leader() {
    let mut ship = escort();
    ship.fleet_escort = false;
    ship.reserves.armor.now = 9.0;
    let mut fighter = ship.fighter();
    assert!(!leave_fleet_if_disabled(&mut fighter, &NovaDisable));
    assert_eq!(fighter.leader, Some(ShipRef::Player));
}

#[test]
fn an_escort_already_disabled_hit_again_leaves_its_leader() {
    // The escort block runs on any hit that leaves the ship disabled
    // (@0x3ae73, @0x3ae90), not only the one that disabled it.
    let mut ship = escort();
    ship.condition = Condition::Disabled;
    let mut fighter = ship.fighter();
    assert!(leave_fleet_if_disabled(&mut fighter, &NovaDisable));
    assert_eq!(fighter.leader, None);
}

#[test]
fn a_ship_led_by_another_npc_keeps_its_leader() {
    let mut ship = escort();
    ship.leader = Some(X);
    ship.fleet = X;
    ship.reserves.armor.now = 9.0;
    let mut fighter = ship.fighter();
    assert!(!leave_fleet_if_disabled(&mut fighter, &NovaDisable));
    assert_eq!((fighter.leader, fighter.fleet), (Some(X), X));
}

/// A, an enemy at (-300, 0), the player at (0, 300) and `e` among them,
/// with A's blaster shot in the shots layer moving into E and `homing`'s
/// missile in the homing layer, aimed at its `target`, moving `into`:
/// the strikes the walk made.
fn walked(
    e: Ship,
    (homing, target): (ShipRef, ShipRef),
    into: (Vec2, Vec2),
) -> Vec<(ShipRef, Option<ShipRef>)> {
    let mut combat = Combat::default();
    let player = player_at(&mut combat, 0.0, 300.0);
    let mut ships = [player, Ship::at(1, -300.0, 0.0), e];
    fly(&mut combat, (blaster(), A, ShotLayer::Shots), INTO_C, None);
    fly(
        &mut combat,
        (missile(), homing, ShotLayer::Homing),
        into,
        None,
    );
    let missile = combat.shots.len() - 1;
    combat.shots[missile].target = Some(target);
    walk(&mut combat, &mut ships)
}

/// A move ending inside the player at (0, 300).
const INTO_PLAYER: (Vec2, Vec2) = (Vec2::new(0.0, 280.0), Vec2::new(0.0, 295.0));

#[test]
fn a_players_shot_later_in_the_walk_hits_an_escort_an_earlier_hit_disabled() {
    let struck = walked(escort(), (ShipRef::Player, E), INTO_C);
    assert_eq!(struck, [(E, Some(A)), (E, Some(ShipRef::Player))]);
}

#[test]
fn a_disabled_escorts_shot_in_flight_hits_the_player_later_in_the_walk() {
    let struck = walked(escort(), (E, ShipRef::Player), INTO_PLAYER);
    assert_eq!(struck, [(E, Some(A)), (ShipRef::Player, Some(E))]);
}

#[test]
fn an_intact_escorts_shot_still_passes_the_player() {
    let mut whole = escort();
    whole.reserves.armor.now = 30.0;
    let struck = walked(whole, (E, ShipRef::Player), INTO_PLAYER);
    assert_eq!(struck, [(E, Some(A))]);
}

#[test]
fn a_mission_escort_disabled_in_the_walk_still_lets_the_players_shots_pass() {
    let mut mission = escort();
    mission.fleet_escort = false;
    let struck = walked(mission, (ShipRef::Player, E), INTO_C);
    assert_eq!(struck, [(E, Some(A))]);
    let struck = walked(mission_escort(), (E, ShipRef::Player), INTO_PLAYER);
    assert_eq!(struck, [(E, Some(A))]);
}

pub(super) fn mission_escort() -> Ship {
    Ship {
        fleet_escort: false,
        ..escort()
    }
}

#[test]
fn a_seeking_sub_munition_of_a_proximity_hit_reads_the_escort_unled() {
    // The player's seeking shell ends 28 pixels from X and 22 from E, its
    // fuse reaching both: it is set off by X, the first in slot order, and
    // its sub-munitions take the nearest ship they can hit.
    let picked = |e: Ship| {
        let (shell, arsenal) = cluster_seeking();
        let mut combat = Combat::default();
        let player = player_at(&mut combat, 0.0, 300.0);
        let mut ships = [
            player,
            Ship::at(1, -300.0, 0.0),
            Ship::at(2, 0.0, -150.0),
            e,
        ];
        fly(&mut combat, (blaster(), A, ShotLayer::Shots), INTO_C, None);
        let beside = (Vec2::new(-10.0, -122.0), Vec2::new(0.0, -122.0));
        fly(
            &mut combat,
            (shell, ShipRef::Player, ShotLayer::Shots),
            beside,
            None,
        );
        let struck = walk_amid(&mut combat, &mut ships, &mut [], &arsenal);
        assert_eq!(struck[..2], [(E, Some(A)), (X, Some(ShipRef::Player))]);
        let subs: Vec<Option<ShipRef>> = combat
            .shots()
            .iter()
            .filter(|shot| shot.weapon.id == WeaponId(148))
            .map(|shot| shot.target)
            .collect();
        subs
    };
    assert_eq!(picked(escort()), [Some(E), Some(E)], "unled, E is nearest");
    assert_eq!(picked(mission_escort()), [Some(X), Some(X)], "led still");
}

/// A sub-munition's slot, its target and whether it has moved.
type SubSeated = (usize, Option<ShipRef>, bool);

/// The player's seeking shell S1 (slot 0) ending 28 pixels from X and 22
/// from `e`, then A's proximity shell S2 (slot 1) of 4 mass ending 30 below
/// `e`, out of X's reach: the proximity pass's strikes, as (ship hit, by),
/// and S1's sub-munitions' slots, targets and whether they moved.
fn s1_then_s2(e: Ship) -> (Vec<(ShipRef, Option<ShipRef>)>, Vec<SubSeated>) {
    let (shell, arsenal) = cluster_seeking();
    let fused_on_e = WeaponRecord {
        reload: 1000,
        count: 3,
        speed: 1000,
        mass_dmg: 4,
        prox_radius: 40,
        ..weapon(183)
    };
    let mut combat = Combat::default();
    let player = player_at(&mut combat, 0.0, 300.0);
    let mut ships = [
        player,
        Ship::at(1, -300.0, 0.0),
        Ship::at(2, 0.0, -150.0),
        e,
    ];
    let beside = (Vec2::new(-10.0, -122.0), Vec2::new(0.0, -122.0));
    fly(
        &mut combat,
        (shell, ShipRef::Player, ShotLayer::Shots),
        beside,
        None,
    );
    let below_e = (Vec2::new(-10.0, -70.0), Vec2::new(0.0, -70.0));
    fly(
        &mut combat,
        (fused_on_e, A, ShotLayer::Shots),
        below_e,
        None,
    );
    let struck = walk_amid(&mut combat, &mut ships, &mut [], &arsenal);
    let subs = combat
        .shots()
        .iter()
        .filter(|shot| shot.weapon.id == WeaponId(148))
        .map(|shot| (shot.slot, shot.target, shot.moved_from.is_some()))
        .collect();
    (struck, subs)
}

#[test]
fn a_proximity_releases_seek_before_a_later_proximity_hit_unleds_an_escort() {
    // `_CheckShotProximities` walks the shots by slot and
    // `_HandleShipHit(shot, ship, 1)` releases each hit's sub-munitions in
    // place (@0x3768a → @0x36f47): S1's seek before S2's hit on E lands,
    // and read E still led.
    let (struck, subs) = s1_then_s2(escort());
    assert_eq!(struck, [(X, Some(ShipRef::Player)), (E, Some(A))]);
    let targets: Vec<Option<ShipRef>> = subs.iter().map(|&(_, target, _)| target).collect();
    assert_eq!(targets, [Some(X), Some(X)]);
}

#[test]
fn a_proximity_hits_own_sub_munitions_read_its_own_hit() {
    // `_HandleShipHit` lands the hit and its blast (@0x36bc5-0x36f05)
    // before it releases (@0x36f12-0x36f47): the player's shell, set off
    // by X, blasts E out of the fleet, and its own sub-munitions take E.
    let (shell, _) = cluster_seeking();
    let blasting = WeaponRecord {
        blast_radius: 40,
        mass_dmg: 4,
        ..shell
    };
    let arsenal = seekers_of(blasting);
    let mut combat = Combat::default();
    let player = player_at(&mut combat, 0.0, 300.0);
    let mut ships = [player, Ship::at(2, 0.0, -150.0), escort()];
    let beside = (Vec2::new(-10.0, -122.0), Vec2::new(0.0, -122.0));
    fly(
        &mut combat,
        (blasting, ShipRef::Player, ShotLayer::Shots),
        beside,
        None,
    );
    walk_amid(&mut combat, &mut ships, &mut [], &arsenal);
    let targets: Vec<Option<ShipRef>> = combat
        .shots()
        .iter()
        .filter(|shot| shot.weapon.id == WeaponId(148))
        .map(|shot| shot.target)
        .collect();
    assert_eq!(targets, [Some(E), Some(E)]);
}

#[test]
fn a_sub_munition_released_into_a_later_slot_sets_off_no_fuse_that_pass() {
    // Released at S1's hit, they take the first slots free with S1's still
    // held and S2 in flight (slots 2 and 3), and the pass does not fuse
    // them: their life is still their `Count` (@0x3720e-0x3721e).
    let (struck, subs) = s1_then_s2(mission_escort());
    assert_eq!(struck, [(X, Some(ShipRef::Player)), (E, Some(A))]);
    assert_eq!(subs, [(2, Some(X), false), (3, Some(X), false)]);
}

/// A shell of 40-pixel fuse releasing two unguided sub-munitions 148
/// that seek the nearest ship (`Flags2` 0x0010).
pub(super) fn cluster_seeking() -> (WeaponRecord, Arsenal) {
    let shell = WeaponRecord {
        reload: 1000,
        count: 3,
        speed: 1000,
        energy_dmg: 1,
        prox_radius: 40,
        sub_count: 2,
        sub_type: Some(WeaponId(148)),
        flags2: SUBS_SEEK_NEAREST,
        ..weapon(182)
    };
    (shell, seekers_of(shell))
}

/// The arsenal of `shell` and the unguided sub-munition 148 it releases.
pub(super) fn seekers_of(shell: WeaponRecord) -> Arsenal {
    let sub = WeaponRecord {
        guidance: 0,
        count: 50,
        speed: 500,
        ..weapon(148)
    };
    Arsenal::new(&[shell, sub], Vec::new())
}

/// A beam of 200 pixels doing 4 mass damage, lasting 3 ticks.
fn ray() -> WeaponSpec {
    WeaponSpec::new(&WeaponRecord {
        guidance: 0,
        count: 3,
        reload: 10,
        beam_length: 200,
        mass_dmg: 4,
        ..weapon(146)
    })
}

/// `firer`'s beam from (`x`, `y`) along `heading`.
fn beam_of(firer: ShipRef, x: f32, y: f32, heading: f32) -> Beam {
    let from = ShipState {
        position: Vec2::new(x, y),
        velocity: Vec2::ZERO,
        heading,
    };
    Beam::launch(ray(), firer, firer, &from, Aiming::Ahead)
}

/// The beams' strikes in a tick's close among A at (-100, -100) facing
/// right, the player at (0, 0) facing up and `e`, with A's beam held
/// first, each along E's centre.
fn beamed(e: Ship) -> Vec<(ShipRef, Option<ShipRef>)> {
    let mut combat = Combat::default();
    let player = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        state: ShipState {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            heading: 0.0,
        },
        ..Ship::at(0, 0.0, 0.0)
    };
    let mut ships = [player, Ship::at(1, -100.0, -100.0), e];
    combat.beams.push(beam_of(A, -100.0, -100.0, 90.0));
    combat.beams.push(beam_of(ShipRef::Player, 0.0, 0.0, 0.0));
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.close_tick(&mut fighters, arena, &mut NeverFires);
    combat
        .take_strikes()
        .iter()
        .map(|strike| (strike.ship, strike.by))
        .collect()
}

#[test]
fn a_later_beam_hits_an_escort_an_earlier_beam_disabled() {
    assert_eq!(beamed(escort()), [(E, Some(A)), (E, Some(ShipRef::Player))]);
}

#[test]
fn a_later_beam_passes_an_escort_an_earlier_beam_left_intact() {
    let mut whole = escort();
    whole.reserves.armor.now = 30.0;
    assert_eq!(beamed(whole), [(E, Some(A))]);
}

#[test]
fn each_beams_hit_lands_before_the_next_beam_is_tested() {
    // `_HandleBeams` explodes, sprays and damages for each beam in turn
    // (@0x31094, @0x31152, @0x316a1): the first beam's hit comes before
    // the second beam's explosion.
    let mut combat = Combat::default();
    let flashing = WeaponSpec::new(&WeaponRecord {
        guidance: 0,
        count: 3,
        energy_dmg: 5,
        explod_type: 1,
        ..weapon(146)
    });
    let mut target = Ship::at(2, 0.0, -100.0);
    target.reserves.shield = Gauge::full(100.0);
    let mut facing_left = Ship::at(4, 100.0, -100.0);
    facing_left.state.heading = 270.0;
    let mut ships = [Ship::at(1, -100.0, -100.0), target, facing_left];
    for (firer, x, heading) in [(A, -100.0, 90.0), (D_SHIP, 100.0, 270.0)] {
        let from = ShipState {
            position: Vec2::new(x, -100.0),
            velocity: Vec2::ZERO,
            heading,
        };
        let spec = WeaponSpec {
            beam_length: 200.0,
            ..flashing
        };
        combat
            .beams
            .push(Beam::launch(spec, firer, firer, &from, Aiming::Ahead));
    }
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.close_tick(&mut fighters, arena, &mut NeverFires);
    let kinds: Vec<&str> = combat
        .take_events()
        .iter()
        .filter_map(|event| match event {
            CombatEvent::Exploded { .. } => Some("exploded"),
            CombatEvent::Hit { .. } => Some("hit"),
            _ => None,
        })
        .collect();
    assert_eq!(kinds, ["exploded", "hit", "exploded", "hit"]);
}

const D_SHIP: ShipRef = ShipRef::Npc(NpcId(4));

/// The strikes and escort losses of a tick's close among A at
/// (-100, -100) facing right, D at (100, -100) facing left and `e`, with
/// A's beam held first and D's second, each along E's centre.
fn beamed_from_both_sides(e: Ship) -> (Vec<(ShipRef, Option<ShipRef>)>, Vec<EscortLoss>) {
    let mut combat = Combat::default();
    let mut facing_left = Ship::at(4, 100.0, -100.0);
    facing_left.state.heading = 270.0;
    let mut ships = [Ship::at(1, -100.0, -100.0), e, facing_left];
    combat.beams.push(beam_of(A, -100.0, -100.0, 90.0));
    combat.beams.push(beam_of(D_SHIP, 100.0, -100.0, 270.0));
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.close_tick(&mut fighters, arena, &mut NeverFires);
    let (strikes, losses, _) = combat.take_landed();
    let struck = strikes
        .iter()
        .map(|strike| (strike.ship, strike.by))
        .collect();
    (struck, losses)
}

#[test]
fn an_escort_lost_is_taken_with_the_index_of_the_strike_that_lost_it() {
    // `_DamageShip` reads a shooter's leader as each kill lands
    // (@0x3ad72), so the session needs where in the strikes the escort
    // stopped being the player's.
    let (struck, losses) = beamed_from_both_sides(escort());
    assert_eq!(struck, [(E, Some(A)), (E, Some(D_SHIP))]);
    assert_eq!(
        losses,
        [EscortLoss { ship: E, strike: 0 }],
        "once, as A's hit"
    );
    let mut sturdier = escort();
    sturdier.reserves.armor.now = 15.0;
    let (struck, losses) = beamed_from_both_sides(sturdier);
    assert_eq!(struck, [(E, Some(A)), (E, Some(D_SHIP))]);
    assert_eq!(losses, [EscortLoss { ship: E, strike: 1 }], "as D's hit");
}

#[test]
fn an_escort_left_intact_or_a_mission_escort_is_never_lost() {
    let mut whole = escort();
    whole.reserves.armor.now = 30.0;
    assert_eq!(beamed_from_both_sides(whole).1, []);
    assert_eq!(beamed_from_both_sides(mission_escort()).1, []);
}

#[test]
fn taking_the_strikes_alone_drops_the_escort_losses_with_them() {
    let mut combat = Combat::default();
    let mut ships = [Ship::at(1, -100.0, -100.0), escort()];
    combat.beams.push(beam_of(A, -100.0, -100.0, 90.0));
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.close_tick(&mut fighters, arena, &mut NeverFires);
    assert_eq!(combat.take_strikes().len(), 1);
    assert_eq!(combat.take_landed(), (Vec::new(), Vec::new(), Vec::new()));
}
