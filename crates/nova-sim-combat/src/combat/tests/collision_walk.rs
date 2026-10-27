//! The collision passes as `_DoPlayGameWork` walks them (rdm
//! weapons-fidelity phase 33): each hit lands as the walk finds it
//! (`_HandleShipHit` @0x36a45-0x36f86), so a turret shot's miss rule
//! (`_ShotCanHitShip` @0x448ad-0x448d5) reads the armour the pass's
//! earlier hits left.

use super::*;

#[test]
fn a_fighter_is_disabled_now_as_is_disabled_reads_it() {
    let rule = NovaDisable;
    let mut under = Ship::at(1, 0.0, 0.0);
    under.reserves.armor.now = 9.0;
    assert!(
        under.fighter().disabled_now(&rule),
        "intact, under the line"
    );
    let mut held = Ship {
        held_disabled: true,
        ..Ship::at(2, 0.0, 0.0)
    };
    assert!(held.fighter().disabled_now(&rule), "held disabled");
    let mut down = Ship {
        condition: Condition::Disabled,
        ..Ship::at(3, 0.0, 0.0)
    };
    assert!(down.fighter().disabled_now(&rule), "not intact");
    let mut above = Ship::at(4, 0.0, 0.0);
    above.reserves.armor.now = 10.0;
    assert!(!above.fighter().disabled_now(&rule), "intact, at the line");
}

pub(super) const D: ShipRef = ShipRef::Npc(NpcId(4));
const X: ShipRef = ShipRef::Npc(NpcId(5));

/// The collision passes alone among `ships`, with no asteroids or
/// stellars, by the default rules: the strikes they made, as (ship hit,
/// by).
pub(super) fn walk(combat: &mut Combat, ships: &mut [Ship]) -> Vec<(ShipRef, Option<ShipRef>)> {
    walk_amid(combat, ships, &mut Rocks::of([]), &Arsenal::default())
}

/// [`walk`] among `rocks` too, sub-munitions read from `arsenal`.
pub(super) fn walk_amid(
    combat: &mut Combat,
    ships: &mut [Ship],
    rocks: &mut Rocks,
    arsenal: &Arsenal,
) -> Vec<(ShipRef, Option<ShipRef>)> {
    let govts = Governments::default();
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let scenery = Scenery {
        asteroids: Some(rocks.lend()),
        ..Scenery::default()
    };
    combat.test_hits(&mut fighters, arena, scenery, &mut NeverFires);
    combat
        .take_strikes()
        .iter()
        .map(|strike| (strike.ship, strike.by))
        .collect()
}

/// A shot of `record` that `firer` fired, in `layer`, its last move from
/// `from` to `to`, rolling `roll`: put in flight in `combat` after the
/// shots already there, so later in firing order.
pub(super) fn fly(
    combat: &mut Combat,
    (record, firer, layer): (WeaponRecord, ShipRef, ShotLayer),
    (from, to): (Vec2, Vec2),
    roll: Option<u32>,
) {
    let state = ShipState {
        position: to,
        velocity: Vec2::ZERO,
        heading: 90.0,
    };
    let mut shot = Shot::launch(WeaponSpec::new(&record), firer, firer, &state, 90.0);
    shot.id = combat.next_id();
    shot.moved_from = Some(from);
    shot.age = 1;
    shot.miss_roll = roll;
    shot.layer = layer;
    shot.firer_slot = Some(match firer {
        ShipRef::Player => Slot::Player,
        ShipRef::Npc(NpcId(berth)) => Slot::Berth(berth),
    });
    combat.put_in_flight(shot);
}

/// A homing missile, aimed at C.
pub(super) fn missile() -> WeaponRecord {
    WeaponRecord {
        guidance: 1,
        guided_turn: 70,
        ..blaster()
    }
}

/// A fighter-class C at (0, -100), its shield down and its armour 11 of
/// 30, one point above the line a blaster's 4 mass takes it under; a
/// blaster's A at (-300, 0) and a heavy turret ship D at (300, 0), far
/// enough that no blast reaches them.
pub(super) fn three() -> Vec<Ship> {
    let mut fighter = Ship::at(3, 0.0, -100.0).of(of_class(80));
    fighter.reserves.shield.now = 0.0;
    fighter.reserves.armor.now = 11.0;
    vec![
        Ship::at(1, -300.0, 0.0).armed(blaster()),
        fighter,
        Ship::at(4, 300.0, 0.0).of(of_class(100)),
    ]
}

/// A move ending inside C, from 20 below its centre.
pub(super) const INTO_C: (Vec2, Vec2) = (Vec2::new(0.0, -80.0), Vec2::new(0.0, -95.0));

/// A turret shot of D's, rolling 81, over a fighter's class.
pub(super) fn rolled_over(layer: ShotLayer) -> (WeaponRecord, ShipRef, ShotLayer) {
    (turret(4), D, layer)
}

#[test]
fn a_turret_shot_rolling_over_a_fighter_an_earlier_hit_this_pass_disabled_hits_it() {
    let mut ships = three();
    let mut combat = Combat::default();
    fly(&mut combat, (blaster(), A, ShotLayer::Shots), INTO_C, None);
    fly(&mut combat, rolled_over(ShotLayer::Shots), INTO_C, Some(81));
    assert_eq!(walk(&mut combat, &mut ships), [(C, Some(A)), (C, Some(D))]);
    assert_eq!(ships[1].condition, Condition::Disabled);
}

#[test]
fn a_turret_shot_rolling_over_an_intact_fighter_passes_it_when_its_test_comes_first() {
    let mut ships = three();
    let mut combat = Combat::default();
    fly(&mut combat, rolled_over(ShotLayer::Shots), INTO_C, Some(81));
    fly(&mut combat, (blaster(), A, ShotLayer::Shots), INTO_C, None);
    assert_eq!(walk(&mut combat, &mut ships), [(C, Some(A))]);
    let left: Vec<ShipRef> = combat.shots().iter().map(|shot| shot.firer).collect();
    assert_eq!(left, [D], "D's shot flies on");
}

#[test]
fn a_shots_layer_turret_is_tested_before_a_homing_missile_disables_the_fighter() {
    let mut ships = three();
    let mut combat = Combat::default();
    fly(&mut combat, (missile(), A, ShotLayer::Homing), INTO_C, None);
    combat.shots[0].target = Some(C);
    fly(&mut combat, rolled_over(ShotLayer::Shots), INTO_C, Some(81));
    assert_eq!(walk(&mut combat, &mut ships), [(C, Some(A))]);
}

#[test]
fn a_turret_over_ships_is_tested_after_a_homing_missile_disables_the_fighter() {
    let mut ships = three();
    let mut combat = Combat::default();
    fly(
        &mut combat,
        rolled_over(ShotLayer::OverShips),
        INTO_C,
        Some(81),
    );
    fly(&mut combat, (missile(), A, ShotLayer::Homing), INTO_C, None);
    combat.shots[1].target = Some(C);
    assert_eq!(walk(&mut combat, &mut ships), [(C, Some(A)), (C, Some(D))]);
}

#[test]
fn a_blast_from_an_earlier_hit_on_another_ship_lets_a_turret_hit_the_fighter_it_disabled() {
    let mut ships = three();
    // X, beside C and walked before it (a lower entry: a higher berth).
    ships.push(Ship::at(5, 25.0, -100.0));
    let shell = WeaponRecord {
        blast_radius: 50,
        ..blaster()
    };
    let mut combat = Combat::default();
    fly(&mut combat, rolled_over(ShotLayer::Shots), INTO_C, Some(81));
    let into_x = (Vec2::new(60.0, -100.0), Vec2::new(40.0, -100.0));
    fly(&mut combat, (shell, A, ShotLayer::Shots), into_x, None);
    assert_eq!(
        walk(&mut combat, &mut ships),
        [(X, Some(A)), (C, Some(A)), (C, Some(D))]
    );
}

/// A move passing 5 pixels outside C's hit radius, ending beside it.
pub(super) const BESIDE_C: (Vec2, Vec2) = (Vec2::new(-10.0, -121.0), Vec2::new(0.0, -121.0));

/// `record` with a proximity fuse reaching 20 pixels.
pub(super) fn fused(record: WeaponRecord) -> WeaponRecord {
    WeaponRecord {
        prox_radius: 20,
        ..record
    }
}

#[test]
fn a_fused_turret_shot_hits_a_fighter_a_contact_disabled_this_pass() {
    let mut ships = three();
    let mut combat = Combat::default();
    let (record, firer, layer) = rolled_over(ShotLayer::Shots);
    fly(
        &mut combat,
        (fused(record), firer, layer),
        BESIDE_C,
        Some(81),
    );
    fly(&mut combat, (blaster(), A, ShotLayer::Shots), INTO_C, None);
    assert_eq!(walk(&mut combat, &mut ships), [(C, Some(A)), (C, Some(D))]);
}

#[test]
fn of_two_fused_turret_shots_the_second_hits_a_fighter_the_first_disabled() {
    let mut ships = three();
    let mut combat = Combat::default();
    fly(
        &mut combat,
        (fused(blaster()), A, ShotLayer::Shots),
        BESIDE_C,
        None,
    );
    let (record, firer, layer) = rolled_over(ShotLayer::Shots);
    fly(
        &mut combat,
        (fused(record), firer, layer),
        BESIDE_C,
        Some(81),
    );
    assert_eq!(walk(&mut combat, &mut ships), [(C, Some(A)), (C, Some(D))]);
}

/// `ship` standing in `layer` at `entry`.
fn ranked(ship: Ship, layer: ShipLayer, entry: i64) -> Ship {
    Ship {
        sprite: Some(SpriteRank { layer, entry }),
        ..ship
    }
}

/// A move from (0, -50) to (0, -150), through ships at (0, -70) and
/// (0, -130).
const THROUGH_TWO: (Vec2, Vec2) = (Vec2::new(0.0, -50.0), Vec2::new(0.0, -150.0));

/// Which of NPC 2, at (0, -70) in `near`, and NPC 3, at (0, -130) in
/// `far`, a shot through both takes.
fn taken_of_two(near: (ShipLayer, i64), far: (ShipLayer, i64)) -> Vec<(ShipRef, Option<ShipRef>)> {
    let mut ships = [
        Ship::at(1, -300.0, 0.0),
        ranked(Ship::at(2, 0.0, -70.0), near.0, near.1),
        ranked(Ship::at(3, 0.0, -130.0), far.0, far.1),
    ];
    let mut combat = Combat::default();
    fly(
        &mut combat,
        (blaster(), A, ShotLayer::Shots),
        THROUGH_TWO,
        None,
    );
    walk(&mut combat, &mut ships)
}

#[test]
fn a_shot_touching_two_ships_hits_the_one_walked_first() {
    use ShipLayer::{Disabled, Led, Ships};
    assert_eq!(taken_of_two((Led, 0), (Ships, 0)), [(C, Some(A))]);
    assert_eq!(taken_of_two((Disabled, 0), (Led, 0)), [(C, Some(A))]);
    assert_eq!(taken_of_two((Ships, 0), (Disabled, 0)), [(B, Some(A))]);
    assert_eq!(
        taken_of_two((Ships, 1), (Ships, 0)),
        [(C, Some(A))],
        "lower entry"
    );
    assert_eq!(taken_of_two((Led, -3), (Led, 2)), [(B, Some(A))]);
}

#[test]
fn a_ship_an_earlier_hit_disabled_is_still_walked_in_its_old_layer() {
    // C, in the top layer, is disabled by A's first shot, and still taken
    // before B, nearer along the second's move, in the escorts' layer.
    let mut far = ranked(Ship::at(3, 0.0, -130.0), ShipLayer::Ships, 0);
    far.reserves.shield.now = 0.0;
    far.reserves.armor.now = 11.0;
    let mut ships = [
        Ship::at(1, -300.0, 0.0),
        ranked(Ship::at(2, 0.0, -70.0), ShipLayer::Led, 0),
        far,
    ];
    let mut combat = Combat::default();
    let into_far = (Vec2::new(0.0, -160.0), Vec2::new(0.0, -140.0));
    fly(
        &mut combat,
        (blaster(), A, ShotLayer::Shots),
        into_far,
        None,
    );
    fly(
        &mut combat,
        (blaster(), A, ShotLayer::Shots),
        THROUGH_TWO,
        None,
    );
    assert_eq!(walk(&mut combat, &mut ships), [(C, Some(A)), (C, Some(A))]);
    assert_eq!(ships[2].condition, Condition::Disabled);
}

#[test]
fn a_hidden_ship_takes_no_contact_but_sets_off_a_fuse() {
    let hidden = |prox| {
        let mut ships = [
            Ship::at(1, -300.0, 0.0),
            Ship {
                sprite: None,
                ..Ship::at(3, 0.0, -100.0)
            },
        ];
        let mut combat = Combat::default();
        let record = WeaponRecord {
            prox_radius: prox,
            ..blaster()
        };
        fly(&mut combat, (record, A, ShotLayer::Shots), INTO_C, None);
        walk(&mut combat, &mut ships)
    };
    assert_eq!(hidden(0), [], "no contact");
    assert_eq!(hidden(20), [(C, Some(A))], "its fuse");
}

#[test]
fn strikes_come_out_in_the_walks_order() {
    let mut ships = [
        Ship::at(1, -300.0, 0.0),
        ranked(Ship::at(2, 100.0, -100.0), ShipLayer::Led, 0),
        ranked(Ship::at(3, -100.0, -100.0), ShipLayer::Ships, 0),
    ];
    let mut combat = Combat::default();
    let into = |x| (Vec2::new(x, -80.0), Vec2::new(x, -95.0));
    fly(
        &mut combat,
        (blaster(), A, ShotLayer::Shots),
        into(100.0),
        None,
    );
    fly(
        &mut combat,
        (blaster(), A, ShotLayer::Shots),
        into(-100.0),
        None,
    );
    assert_eq!(walk(&mut combat, &mut ships), [(C, Some(A)), (B, Some(A))]);
}

#[test]
fn a_missile_meets_the_ship_it_touched_before_an_asteroid_on_its_move() {
    // The homing layer's ship passes come before its asteroid pass
    // (@0x43ef1-0x43f62), and the shot layer's asteroid pass before
    // both.
    let mut ships = three();
    let mut rocks = Rocks::of([rock(0, 0.0, -82.0)]);
    let mut combat = Combat::default();
    fly(&mut combat, (missile(), A, ShotLayer::Homing), INTO_C, None);
    combat.shots[0].target = Some(C);
    let arsenal = Arsenal::default();
    assert_eq!(
        walk_amid(&mut combat, &mut ships, &mut rocks, &arsenal),
        [(C, Some(A))]
    );
    assert_eq!(rocks.at(0).strength, 10, "the asteroid untouched");
}

#[test]
fn point_defence_passes_a_missile_that_touched_the_player_where_it_stood() {
    // Contact meets the player where it stood as the shots moved, so the
    // missile's contact comes first and point defence's layer passes it.
    let player = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        ..Ship::at(0, 0.0, 300.0)
    };
    let mut ships = [Ship::at(1, -300.0, 0.0), player, Ship::at(4, 300.0, 0.0)];
    let mut combat = Combat {
        player_stood: Some(Vec2::new(0.0, -100.0)),
        ..Combat::default()
    };
    fly(
        &mut combat,
        (still_missile(), A, ShotLayer::Homing),
        INTO_C,
        None,
    );
    combat.shots[0].target = Some(ShipRef::Player);
    let at_the_missile = (Vec2::new(0.0, -90.0), INTO_C.1);
    fly(
        &mut combat,
        (pd_shot(), D, ShotLayer::PointDefence),
        at_the_missile,
        None,
    );
    assert_eq!(walk(&mut combat, &mut ships), [(ShipRef::Player, Some(A))]);
    let left: Vec<ShipRef> = combat.shots().iter().map(|shot| shot.firer).collect();
    assert_eq!(left, [D], "point defence's shot unspent");
}

/// A homing missile of A's into C, a point-defence shot of D's over it,
/// with C hidden or not: the strikes the walk made and whose shots are
/// left in flight.
fn missile_into_c_under_point_defence(
    sprite: Option<SpriteRank>,
) -> (Vec<(ShipRef, Option<ShipRef>)>, Vec<ShipRef>) {
    let mut ships = [
        Ship::at(1, -300.0, 0.0),
        Ship {
            sprite,
            ..Ship::at(3, 0.0, -100.0)
        },
        Ship::at(4, 300.0, 0.0),
    ];
    let mut combat = Combat::default();
    fly(
        &mut combat,
        (still_missile(), A, ShotLayer::Homing),
        INTO_C,
        None,
    );
    combat.shots[0].target = Some(C);
    let at_the_missile = (Vec2::new(0.0, -90.0), INTO_C.1);
    fly(
        &mut combat,
        (pd_shot(), D, ShotLayer::PointDefence),
        at_the_missile,
        None,
    );
    let struck = walk(&mut combat, &mut ships);
    let left = combat.shots().iter().map(|shot| shot.firer).collect();
    (struck, left)
}

#[test]
fn point_defence_meets_a_missile_that_touches_only_a_hidden_ship() {
    // The contact passes never walk a hidden ship's sprite (@0xacba3), so
    // `_PointDefenseCollisionHandler` (pass 6, @0x43fff) meets the missile.
    let (struck, left) = missile_into_c_under_point_defence(None);
    assert_eq!(struck, []);
    assert_eq!(left, [], "point defence's shot spent, the missile down");
}

#[test]
fn point_defence_passes_a_missile_that_touches_a_walked_ship() {
    let walked = Some(SpriteRank {
        layer: ShipLayer::Ships,
        entry: 0,
    });
    let (struck, left) = missile_into_c_under_point_defence(walked);
    assert_eq!(struck, [(C, Some(A))]);
    assert_eq!(left, [D], "point defence's shot unspent");
}

#[test]
fn a_turret_shot_hits_a_fighter_an_asteroids_blast_disabled_this_pass() {
    // The player's shot in the shots layer meets a rock beside C, whose
    // blast (only a player's shot's, @0x3670d) takes C under the line;
    // D's turret shot over ships, rolling over C's class, is tested after
    // it.
    let mut ships = three();
    ships[0] = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        ..Ship::at(0, -300.0, 0.0)
    };
    let mut rocks = Rocks::of([rock(0, 0.0, -130.0)]);
    let shell = WeaponRecord {
        blast_radius: 50,
        ..blaster()
    };
    let mut combat = Combat::default();
    let into_rock = (Vec2::new(0.0, -150.0), Vec2::new(0.0, -135.0));
    fly(
        &mut combat,
        (shell, ShipRef::Player, ShotLayer::Shots),
        into_rock,
        None,
    );
    fly(
        &mut combat,
        rolled_over(ShotLayer::OverShips),
        INTO_C,
        Some(81),
    );
    let arsenal = Arsenal::default();
    let struck = walk_amid(&mut combat, &mut ships, &mut rocks, &arsenal);
    assert_eq!(struck, [(C, Some(ShipRef::Player)), (C, Some(D))]);
}

#[test]
fn a_fused_shells_sub_munitions_read_their_owners_plunder_as_it_hits() {
    let released = |plundering: bool| {
        let (shell, arsenal) = cluster(0);
        let mut ships = three();
        ships[0].plundering = plundering;
        ships[0].target = Some(C);
        let mut combat = Combat::default();
        fly(
            &mut combat,
            (fused(shell), A, ShotLayer::Shots),
            BESIDE_C,
            None,
        );
        let struck = walk_amid(&mut combat, &mut ships, &mut Rocks::of([]), &arsenal);
        assert_eq!(struck, [(C, Some(A))]);
        let subs = sub_shots(&combat);
        assert_eq!(subs.len(), 2);
        subs.iter()
            .map(|shot| shot.disable_only)
            .collect::<Vec<_>>()
    };
    assert_eq!(released(true), [true, true]);
    assert_eq!(released(false), [false, false]);
}
