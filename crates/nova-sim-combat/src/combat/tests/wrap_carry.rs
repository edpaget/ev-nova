//! The player's wrap in the fight (see the `wrap_carry` module): what it
//! moves, and how its pass's contact walks meet what it left.

use super::collision_walk::{D, fly, walk, walk_amid};
use super::*;
use crate::catalog::StellarId;
use crate::combat::stellar::{StellarStrike, StellarTarget};
use crate::player_wrap::{PlayerWrap, WRAP_EDGE, WRAP_SHIFT};

/// Where the player stood as it wrapped, east of the edge.
const STOOD: Vec2 = Vec2::new(WRAP_EDGE + 10.0, 0.0);

const SHIFT: Vec2 = Vec2::new(-WRAP_SHIFT, 0.0);

/// Well beyond the wrap's reach of [`STOOD`], on the way back.
const FAR: f32 = -WRAP_EDGE * 2.0;

/// What the wrap at [`STOOD`] reaches, and what it leaves: `x` from the
/// player, on the way.
const fn off(x: f32) -> Vec2 {
    Vec2::new(STOOD.x + x, 0.0)
}

fn wrap() -> PlayerWrap {
    PlayerWrap::of(STOOD).expect("past the edge")
}

/// A shot from `A` still at `to`, having just moved 10 pixels on.
fn shot_to(combat: &mut Combat, to: Vec2) {
    let from = to - Vec2::new(10.0, 0.0);
    let flown = (blaster(), A, ShotLayer::Shots);
    fly(combat, flown, (from, to), None);
}

fn shot_with_a_fuse_to(combat: &mut Combat, to: Vec2, reach: i16) {
    let record = WeaponRecord {
        prox_radius: reach,
        ..blaster()
    };
    fly(combat, (record, A, ShotLayer::Shots), (to, to), None);
}

fn none_left() -> Vec<ShipRef> {
    Vec::new()
}

#[test]
fn a_shot_within_reach_moves_with_its_last_move_and_keeps_its_velocity() {
    let mut combat = Combat::default();
    shot_to(&mut combat, off(-100.0));
    let velocity = combat.shots[0].velocity;
    combat.carry(&wrap(), none_left());
    let shot = &combat.shots[0];
    assert_eq!(shot.position, off(-100.0) + SHIFT);
    assert_eq!(shot.moved_from, Some(off(-110.0) + SHIFT));
    assert_eq!(shot.velocity, velocity);
}

#[test]
fn a_shot_that_has_not_moved_moves_with_nothing_to_carry_of_its_move() {
    let mut combat = Combat::default();
    shot_to(&mut combat, off(-100.0));
    combat.shots[0].moved_from = None;
    combat.carry(&wrap(), none_left());
    assert_eq!(combat.shots[0].moved_from, None);
}

#[test]
fn a_shot_beyond_reach_stays() {
    let mut combat = Combat::default();
    shot_to(&mut combat, off(FAR));
    combat.carry(&wrap(), none_left());
    let shot = &combat.shots[0];
    assert_eq!(shot.position, off(FAR));
    assert_eq!(shot.moved_from, Some(off(FAR - 10.0)));
}

#[test]
fn a_shot_point_defence_ended_moves_by_the_same_test() {
    let mut combat = Combat::default();
    shot_to(&mut combat, off(-100.0));
    shot_to(&mut combat, off(FAR));
    combat.hidden = std::mem::take(&mut combat.shots);
    combat.carry(&wrap(), none_left());
    assert_eq!(combat.hidden[0].position, off(-100.0) + SHIFT);
    assert_eq!(combat.hidden[1].position, off(FAR));
}

/// The beam of a ship at `x` on the axis, fired in `combat`.
fn beam_fired_at(combat: &mut Combat, x: f32) {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let mut ships = [Ship::at(3, x, 0.0).armed(laser())];
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.handle_npcs(&mut fighters, arena, &mut NeverFires);
}

#[test]
fn every_beam_moves_with_no_test() {
    let mut combat = Combat::default();
    beam_fired_at(&mut combat, off(-100.0).x);
    beam_fired_at(&mut combat, off(FAR).x);
    let before: Vec<(Vec2, Vec2)> = combat.beams.iter().map(|b| (b.start, b.end)).collect();
    assert_eq!(before.len(), 2);
    combat.carry(&wrap(), none_left());
    for (beam, (start, end)) in combat.beams.iter().zip(before) {
        assert_eq!(beam.start, start + SHIFT);
        assert_eq!(beam.end, end + SHIFT);
    }
}

#[test]
fn where_the_player_stood_moves_whatever_it_is_near() {
    let mut combat = Combat {
        player_stood: Some(off(FAR)),
        ..Combat::default()
    };
    combat.carry(&wrap(), none_left());
    assert_eq!(combat.player_stood, Some(off(FAR) + SHIFT));
    let mut combat = Combat::default();
    combat.carry(&wrap(), none_left());
    assert_eq!(combat.player_stood, None, "not in the last tick's fight");
}

#[test]
fn a_slots_kept_position_moves_by_the_test_empty_or_not() {
    let mut combat = Combat::default();
    let mut left = [
        Ship::at(1, off(-100.0).x, 0.0),
        Ship::at(2, off(FAR).x, 0.0),
    ];
    combat.note_slots(&left.iter_mut().map(Ship::fighter).collect::<Vec<_>>());
    combat.carry(&wrap(), none_left());
    let kept = |berth| combat.slot_ships[&Slot::Berth(berth)].position;
    assert_eq!(kept(1), off(-100.0) + SHIFT);
    assert_eq!(kept(2), off(FAR));
}

// The wrap's pass: the contact walks.

/// The strikes `shot` makes on a ship at `ship`, the wrap having left it
/// behind or not: shot and ship placed as the last pass placed them.
fn struck_on_the_wraps_pass(shot: Vec2, ship: Vec2, ship_left: bool) -> bool {
    let mut combat = Combat::default();
    shot_to(&mut combat, shot);
    let left = if ship_left { vec![B] } else { none_left() };
    let ship_at = if ship_left { ship } else { ship + SHIFT };
    combat.carry(&wrap(), left);
    let mut ships = [Ship::at(2, ship_at.x, ship_at.y)];
    !walk(&mut combat, &mut ships).is_empty()
}

#[test]
fn a_shot_carried_meets_a_ship_carried_where_their_sprites_stood() {
    assert!(struck_on_the_wraps_pass(off(-100.0), off(-100.0), false));
}

#[test]
fn a_shot_carried_meets_a_ship_left_behind_where_its_sprite_stood() {
    // Each on its own side of the reach, their sprites touching.
    let (shot, ship) = (off(-WRAP_EDGE + 5.0), off(-WRAP_EDGE - 5.0));
    assert!(struck_on_the_wraps_pass(shot, ship, true));
}

#[test]
fn a_shot_left_behind_meets_a_ship_left_behind_where_it_is() {
    let at = off(FAR);
    assert!(struck_on_the_wraps_pass(at, at, true));
}

#[test]
fn a_shot_left_behind_meets_a_ship_carried_where_their_sprites_stood() {
    let (shot, ship) = (off(-WRAP_EDGE - 5.0), off(-WRAP_EDGE + 5.0));
    assert!(struck_on_the_wraps_pass(shot, ship, false));
}

#[test]
fn a_ship_left_behind_is_not_met_where_it_is_by_a_shot_that_was_carried_past_it() {
    // The shot comes down on the ship's world place, but its sprite stood a
    // whole shift from the ship's.
    let ship = off(-100.0) + SHIFT;
    let mut combat = Combat::default();
    shot_to(&mut combat, off(-100.0));
    combat.carry(&wrap(), vec![B]);
    let mut ships = [Ship::at(2, ship.x, ship.y)];
    assert_eq!(walk(&mut combat, &mut ships), []);
}

/// Whether a shot at `shot` is spent on an asteroid standing at `rock`.
fn rock_struck_on_the_wraps_pass(shot: Vec2, rock_at: Vec2) -> bool {
    let mut combat = Combat::default();
    shot_to(&mut combat, shot);
    combat.carry(&wrap(), none_left());
    let mut rocks = Rocks::of([rock(0, rock_at.x, rock_at.y)]);
    walk_amid(&mut combat, &mut [], &mut rocks, &Arsenal::default());
    combat.shots.is_empty()
}

#[test]
fn a_shot_carried_meets_an_asteroid_where_its_sprite_stood() {
    assert!(rock_struck_on_the_wraps_pass(off(-100.0), off(-100.0)));
}

#[test]
fn a_shot_carried_is_not_spent_on_an_asteroid_it_lands_on_in_the_world() {
    assert!(!rock_struck_on_the_wraps_pass(
        off(-100.0),
        off(-100.0) + SHIFT
    ));
}

#[test]
fn a_shot_left_behind_meets_an_asteroid_where_it_is() {
    let at = off(FAR);
    assert!(rock_struck_on_the_wraps_pass(at, at));
}

#[test]
fn point_defence_meets_a_missile_each_side_of_the_reach_where_their_sprites_stood() {
    let mut combat = Combat {
        player_stood: Some(STOOD),
        ..Combat::default()
    };
    let (missile_at, defender_at) = (off(-WRAP_EDGE - 2.0), off(-WRAP_EDGE + 2.0));
    fly(
        &mut combat,
        (still_missile(), A, ShotLayer::Homing),
        (missile_at, missile_at),
        None,
    );
    fly(
        &mut combat,
        (pd_shot(), D, ShotLayer::PointDefence),
        (defender_at, defender_at),
        None,
    );
    combat.carry(&wrap(), none_left());
    walk(&mut combat, &mut []);
    assert_eq!(combat.shots.len(), 0, "both spent: {:?}", combat.shots);
}

/// Whether a fused shot carried to where it is, with an asteroid `off`
/// from the shot's place before the wrap, `wraps` or not, is set off.
fn fuse_set_off(rock_at: Vec2, wraps: bool) -> bool {
    let mut combat = Combat::default();
    shot_with_a_fuse_to(&mut combat, off(-100.0), 30);
    combat.shots[0].age = 3;
    if wraps {
        combat.carry(&wrap(), none_left());
    }
    let mut rocks = Rocks::of([rock(0, rock_at.x, rock_at.y)]);
    walk_amid(&mut combat, &mut [], &mut rocks, &Arsenal::default());
    combat.shots.is_empty()
}

#[test]
fn a_fuse_reads_world_positions_on_the_wraps_pass() {
    let beside = Vec2::new(0.0, 25.0);
    assert!(fuse_set_off(off(-100.0) + beside, false), "with no wrap");
    assert!(
        !fuse_set_off(off(-100.0) + beside, true),
        "where its sprite stood"
    );
    assert!(
        fuse_set_off(off(-100.0) + SHIFT + beside, true),
        "in the world"
    );
}

/// The strikes a shot carried from beside the stellar at `stellar` makes.
fn stellars_struck(stellar: Vec2) -> Vec<StellarStrike> {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let mut combat = Combat::default();
    let planet_type = WeaponRecord {
        flags2: 0x0400,
        ..blaster()
    };
    let at = off(-100.0);
    fly(
        &mut combat,
        (planet_type, A, ShotLayer::Shots),
        (at, at),
        None,
    );
    combat.shots[0].age = 3;
    combat.carry(&wrap(), none_left());
    let mut stellars = [StellarTarget {
        stellar: StellarId(128),
        position: stellar,
        radius: 40.0,
        strength: 100,
    }];
    let scenery = Scenery {
        stellars: &mut stellars,
        ..Scenery::default()
    };
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    combat.test_hits(&mut [], arena, scenery, &mut NeverFires);
    combat.take_stellar_strikes()
}

#[test]
fn a_stellar_is_met_at_its_world_position_on_the_wraps_pass() {
    assert_eq!(stellars_struck(off(-100.0)), [], "where its sprite stood");
    assert_eq!(
        stellars_struck(off(-100.0) + SHIFT).len(),
        1,
        "in the world"
    );
}

#[test]
fn the_next_pass_meets_nothing_where_its_sprite_stood() {
    let mut combat = Combat::default();
    shot_to(&mut combat, off(-100.0));
    combat.carry(&wrap(), none_left());
    walk(&mut combat, &mut []);
    assert_eq!(combat.wrap_pass, None);
    let mut rocks = Rocks::of([rock(0, off(-100.0).x, 0.0)]);
    walk_amid(&mut combat, &mut [], &mut rocks, &Arsenal::default());
    assert_eq!(combat.shots.len(), 1, "unspent");
}

#[test]
fn clearing_the_fight_drops_the_pass() {
    let mut combat = Combat::default();
    combat.carry(&wrap(), none_left());
    combat.clear();
    assert_eq!(combat.wrap_pass, None);
}
