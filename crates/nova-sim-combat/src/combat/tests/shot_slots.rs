//! The original's shot slots in the fight (rdm weapons-fidelity phase
//! 34): which slot each shot takes (`_SpawnShot` @0x3e5ab-0x3e5ce), when
//! `_HandleShot` frees it (@0x3617c), and the order each layer keeps
//! (`_PutSpriteInLayer` @0x766c-0x76b1).

use super::collision_walk::{BESIDE_C, D, INTO_C, fly, fused, missile, rolled_over, three, walk};
use super::*;
use crate::combat::weapon::{PROXIMITY_BY_OTHERS, SPINS, SUBS_SEEK_NEAREST};

/// A long-lived shot of C's, far from every ship, never moving.
fn idle() -> Shot {
    let weapon = WeaponSpec::new(&WeaponRecord {
        count: 1000,
        speed: 0,
        ..weapon(140)
    });
    let far = ShipState {
        position: Vec2::new(50_000.0, 50_000.0),
        ..ShipState::default()
    };
    Shot::launch(weapon, C, C, &far, 0.0)
}

/// A shot of C's whose last move, from 10 to the left, ended on B at
/// (0, -200), for the collision passes to end.
fn ending_on_b() -> Shot {
    let at = Vec2::new(0.0, -200.0);
    Shot {
        position: at,
        moved_from: Some(at - Vec2::new(10.0, 0.0)),
        age: 1,
        ..Shot::launch(
            WeaponSpec::new(&blaster()),
            C,
            C,
            &ShipState::default(),
            90.0,
        )
    }
}

/// The slots of the shots in flight, as kept.
fn slots(combat: &Combat) -> Vec<usize> {
    combat.shots().iter().map(|shot| shot.slot).collect()
}

#[test]
fn a_shot_takes_the_first_free_slot_and_the_shots_are_kept_in_slot_order() {
    let mut combat = Combat::default();
    for _ in 0..3 {
        assert!(combat.put_in_flight(idle()).is_some());
    }
    assert_eq!(slots(&combat), [0, 1, 2]);
    let ended = combat.shots.remove(1);
    combat.slots.hold(ended.slot);
    let mut next = idle();
    next.id = ShotId(7);
    combat.put_in_flight(next);
    assert_eq!(slots(&combat), [0, 2, 3], "1 still held");
    combat.slots.free(1);
    next.id = ShotId(8);
    combat.put_in_flight(next);
    assert_eq!(slots(&combat), [0, 1, 2, 3], "the freed lower slot");
    assert_eq!(combat.shots()[1].id, ShotId(8));
}

#[test]
fn a_shot_fired_after_a_lower_slot_is_freed_takes_it() {
    // C's shot in slot 0 ends on B in the collision passes; `_HandleShot`
    // frees it (@0x3617c) before A's pick (`_HandleShip` @0x44483), whose
    // shot takes slot 0, under the two idle shots, not slot 3.
    for capped in [false, true] {
        let mut combat = Combat::default();
        if capped {
            combat.set_combat_caps(RuleSource::Engine);
        }
        combat.put_in_flight(ending_on_b());
        combat.put_in_flight(idle());
        combat.put_in_flight(idle());
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()),
            Ship::at(2, 0.0, -200.0),
        ];
        phase::test_hits(&mut combat, &mut ships, &NovaDisable);
        assert!(ships[1].reserves.shield.now < 10.0, "B struck");
        assert!(combat.slots.is_held(0), "{capped}: held till `_HandleShot`");
        phase::fly_shots(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(slots(&combat), [1, 2], "{capped}: freed");
        phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(slots(&combat), [0, 1, 2], "{capped}");
        assert_eq!(combat.shots()[0].firer, A, "{capped}");
    }
}

#[test]
fn a_spent_shots_slot_is_held_through_the_npcs_point_defence_and_freed_by_handle_shot() {
    let mut combat = Combat::default();
    combat.put_in_flight(idle());
    combat.put_in_flight(ending_on_b());
    let mut ships = [Ship::at(1, 0.0, 0.0), Ship::at(2, 0.0, -200.0)];
    let govts = Governments::default();
    let arsenal = Arsenal::default();
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.test_hits(&mut fighters, arena, Scenery::default(), &mut NeverFires);
    combat.defend_npcs(&mut fighters, arena, None, false, &mut NeverFires);
    assert_eq!(slots(&combat), [0], "spent");
    assert!(combat.slots.is_held(1), "held through point defence");
    assert_eq!(combat.first_free_slot(), Some(2));
    combat.fly_shots(&mut fighters, arena, &mut NeverFires);
    assert!(!combat.slots.is_held(1), "freed at its visit");
    assert_eq!(combat.first_free_slot(), Some(1));
}

#[test]
fn a_shot_spent_on_a_stellar_holds_its_slot_through_the_npcs_point_defence() {
    // A planet-type shot of C's whose last move, from 10 to the left,
    // ended inside the stellar at (0, 500): the hit pass spends it
    // (`Combat::proximity_pass`) and its slot is held until `_HandleShot`
    // visits it (@0x36195 → @0x3617c), as a shot spent on a ship's is.
    let at = Vec2::new(0.0, 500.0);
    let on_the_stellar = Shot {
        position: at,
        moved_from: Some(at - Vec2::new(10.0, 0.0)),
        age: 1,
        ..Shot::launch(
            WeaponSpec::new(&planet_gun()),
            C,
            C,
            &ShipState::default(),
            90.0,
        )
    };
    let mut combat = Combat::default();
    combat.put_in_flight(idle());
    combat.put_in_flight(on_the_stellar);
    let mut ships = [Ship::at(1, 0.0, 0.0), Ship::at(2, 0.0, -200.0)];
    let mut stellars = [stellar_at(0.0, 500.0, 20.0, 100)];
    let govts = Governments::default();
    let arsenal = Arsenal::default();
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let scenery = Scenery {
        stellars: &mut stellars,
        ..Scenery::default()
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.test_hits(&mut fighters, arena, scenery, &mut NeverFires);
    combat.defend_npcs(&mut fighters, arena, None, false, &mut NeverFires);
    assert_eq!(slots(&combat), [0], "spent on the stellar");
    assert!(combat.slots.is_held(1), "held through point defence");
    assert_eq!(combat.first_free_slot(), Some(2));
    combat.fly_shots(&mut fighters, arena, &mut NeverFires);
    assert!(!combat.slots.is_held(1), "freed at its visit");
    assert_eq!(combat.first_free_slot(), Some(1));
}

/// A shot of C's in `layer`, numbered `id` in firing order.
fn in_layer(layer: ShotLayer, id: u32) -> Shot {
    Shot {
        layer,
        id: ShotId(id),
        ..idle()
    }
}

/// A combat whose shot layers overlap as `source` reads them.
fn overlapping(source: RuleSource) -> Combat {
    let mut combat = Combat::default();
    combat.set_shot_overlap(source);
    combat
}

/// The numbers of the shots in the order they are drawn.
fn drawn(combat: &Combat) -> Vec<u32> {
    combat.shots_drawn().iter().map(|shot| shot.id.0).collect()
}

/// Ends and frees the shot in `slot`, as `_HandleShot`'s visit does.
fn end_and_free(combat: &mut Combat, slot: usize) {
    combat.shots.retain(|shot| shot.slot != slot);
}

#[test]
fn a_reused_slot_whose_sprite_is_in_the_layer_keeps_its_place() {
    // X (0) takes slot 0 and Y (1) slot 1, both moved to the homing
    // layer's tail; X ends, and Z (2) takes slot 0, already in the layer
    // (@0x768c): drawn under Y, though fired after it.
    for (source, order) in [(RuleSource::Engine, [2, 1]), (RuleSource::Bible, [1, 2])] {
        let mut combat = overlapping(source);
        combat.put_in_flight(in_layer(ShotLayer::Homing, 0));
        combat.put_in_flight(in_layer(ShotLayer::Homing, 1));
        end_and_free(&mut combat, 0);
        combat.put_in_flight(in_layer(ShotLayer::Homing, 2));
        assert_eq!(combat.shots()[0].id, ShotId(2), "slot 0");
        assert_eq!(drawn(&combat), order, "{source:?}");
    }
}

#[test]
fn a_reused_slot_from_another_layer_goes_to_the_tail() {
    for source in RuleSource::ALL {
        let mut combat = overlapping(source);
        combat.put_in_flight(in_layer(ShotLayer::Shots, 0));
        combat.put_in_flight(in_layer(ShotLayer::Homing, 1));
        end_and_free(&mut combat, 0);
        combat.put_in_flight(in_layer(ShotLayer::Homing, 2));
        assert_eq!(drawn(&combat), [1, 2], "{source:?}");
    }
}

#[test]
fn the_layers_are_drawn_bottom_first() {
    for source in RuleSource::ALL {
        let mut combat = overlapping(source);
        combat.put_in_flight(in_layer(ShotLayer::OverShips, 0));
        combat.put_in_flight(in_layer(ShotLayer::Shots, 1));
        combat.put_in_flight(in_layer(ShotLayer::PointDefence, 2));
        combat.put_in_flight(in_layer(ShotLayer::Homing, 3));
        assert_eq!(drawn(&combat), [2, 3, 1, 0], "{source:?}");
    }
}

#[test]
fn a_point_defence_shot_fired_at_a_ship_goes_to_the_shot_layers_tail() {
    // Guns in slots 0..2; slot 0's ends. The point-defence shot at the
    // fighter takes slot 0, is put in point defence's layer and then moved
    // to the shot layer (`_HandleShipPointDefense` @0x398bd): at its tail,
    // over slots 1 and 2, where a gun in slot 0 would be under them.
    for source in RuleSource::ALL {
        let mut combat = overlapping(source);
        for id in 0..3 {
            combat.put_in_flight(in_layer(ShotLayer::Shots, id));
        }
        end_and_free(&mut combat, 0);
        combat.next_shot = 10;
        let mut ships = fighter_attack(quad(), 0.0, -200.0);
        phase::defend_npcs(&mut combat, &mut ships, &NovaDisable);
        let fired = pd_shots(&combat);
        assert_eq!(fired.len(), 1, "{:?}", combat.shots());
        assert_eq!((fired[0].slot, fired[0].layer), (0, ShotLayer::Shots));
        let pd = fired[0].id.0;
        assert_eq!(drawn(&combat), [1, 2, pd], "{source:?}");
    }
    let mut combat = Combat::default();
    for id in 0..3 {
        combat.put_in_flight(in_layer(ShotLayer::Shots, id));
    }
    end_and_free(&mut combat, 0);
    combat.put_in_flight(in_layer(ShotLayer::Shots, 3));
    assert_eq!(drawn(&combat), [3, 1, 2], "a gun keeps slot 0's place");
}

/// Moves slot 0's sprite out of the homing layer and back: a shot in the
/// shot layer takes it and ends, so the next homing shot in slot 0 enters
/// the homing layer at its tail.
fn cycle_slot_0(combat: &mut Combat) {
    end_and_free(combat, 0);
    combat.put_in_flight(in_layer(ShotLayer::Shots, 99));
    end_and_free(combat, 0);
}

/// Aims every shot in flight at C, which a missile hits alone.
fn all_at_c(combat: &mut Combat) {
    for shot in &mut combat.shots {
        shot.target = Some(C);
        shot.aimed_at = Some(C);
    }
}

#[test]
fn the_contact_walk_takes_a_reused_slot_in_its_old_place() {
    // A's gun G1 takes slot 0 and D's turret T slot 1, both in the shot
    // layer; G1 ends, and A's gun G2 takes slot 0, which keeps its place
    // under T. By entry G2 lands first and disables C, and T, rolling 81
    // over C's class, hits it disabled; by firing order T is tested first
    // and passes the intact C.
    for (source, strikes) in [
        (RuleSource::Engine, vec![(C, Some(A)), (C, Some(D))]),
        (RuleSource::Bible, vec![(C, Some(A))]),
    ] {
        let mut ships = three();
        let mut combat = overlapping(source);
        fly(&mut combat, (blaster(), A, ShotLayer::Shots), INTO_C, None);
        fly(&mut combat, rolled_over(ShotLayer::Shots), INTO_C, Some(81));
        end_and_free(&mut combat, 0);
        fly(&mut combat, (blaster(), A, ShotLayer::Shots), INTO_C, None);
        assert_eq!(walk(&mut combat, &mut ships), strikes, "{source:?}");
    }
}

#[test]
fn the_contact_walk_goes_by_entry_not_by_slot() {
    // A's missile in slot 1 entered the homing layer before D's in slot 0,
    // whose sprite went out and back: the walk meets A's first, which
    // disables C, so D's, rolling 81, hits. By slot D's would pass C.
    for source in RuleSource::ALL {
        let mut ships = three();
        let mut combat = overlapping(source);
        fly(&mut combat, (missile(), A, ShotLayer::Homing), INTO_C, None);
        fly(&mut combat, (missile(), A, ShotLayer::Homing), INTO_C, None);
        cycle_slot_0(&mut combat);
        fly(
            &mut combat,
            (missile(), D, ShotLayer::Homing),
            INTO_C,
            Some(81),
        );
        all_at_c(&mut combat);
        assert_eq!(combat.shots()[0].firer, D, "slot 0");
        let strikes = walk(&mut combat, &mut ships);
        assert_eq!(strikes, [(C, Some(A)), (C, Some(D))], "{source:?}");
    }
}

#[test]
fn the_proximity_pass_goes_by_slot_not_by_entry() {
    // The same two missiles, fused, beside C: `_CheckShotProximities`
    // walks the slots (@0x371e0-0x37765), so D's in slot 0 lands first,
    // though it entered the layer and was fired after A's.
    for source in RuleSource::ALL {
        let mut ships = three();
        let mut combat = overlapping(source);
        let fused_missile = fused(missile());
        fly(
            &mut combat,
            (fused_missile, A, ShotLayer::Homing),
            BESIDE_C,
            None,
        );
        fly(
            &mut combat,
            (fused_missile, A, ShotLayer::Homing),
            BESIDE_C,
            None,
        );
        cycle_slot_0(&mut combat);
        fly(
            &mut combat,
            (fused_missile, D, ShotLayer::Homing),
            BESIDE_C,
            None,
        );
        all_at_c(&mut combat);
        let strikes = walk(&mut combat, &mut ships);
        assert_eq!(strikes, [(C, Some(D)), (C, Some(A))], "{source:?}");
    }
}

#[test]
fn point_defence_meets_a_missile_with_its_first_shot_in_entry_order() {
    // X takes slot 0 and Y slot 1 in point defence's layer; X ends, and Z
    // takes slot 0, keeping its place under Y. By entry Z meets B's
    // missile first and downs it, and Y flies on; by firing order Y does.
    for (source, spent, kept) in [(RuleSource::Engine, 2, 1), (RuleSource::Bible, 1, 2)] {
        let mut combat = overlapping(source);
        let quad = pd_shot();
        put(&mut combat, quad, A, Vec2::ZERO, Vec2::ZERO);
        let y = put(&mut combat, quad, A, Vec2::ZERO, Vec2::ZERO);
        end_and_free(&mut combat, 0);
        let z = put(&mut combat, quad, A, Vec2::ZERO, Vec2::ZERO);
        let missile = put(
            &mut combat,
            still_missile(),
            B,
            Vec2::new(3.0, 0.0),
            Vec2::ZERO,
        );
        assert_eq!((y, z), (ShotId(1), ShotId(2)));
        let mut ships = [Ship::at(1, 1000.0, 1000.0), Ship::at(2, -1000.0, 1000.0)];
        walk(&mut combat, &mut ships);
        let left: Vec<u32> = combat.shots().iter().map(|shot| shot.id.0).collect();
        assert_eq!(left, [kept], "{source:?}: {spent} spent, {missile:?} gone");
    }
}

/// A shell releasing one of weapon 148 at the end of its life, and that
/// weapon.
fn one_sub() -> (WeaponRecord, Arsenal) {
    let (shell, _) = cluster(0);
    let shell = WeaponRecord {
        sub_count: 1,
        ..shell
    };
    let sub = WeaponRecord {
        count: 50,
        speed: 500,
        ..weapon(148)
    };
    (shell, Arsenal::new(&[shell, sub], Vec::new()))
}

/// A shell of `record` whose last move, its third, spent it, at (0, 300).
pub(super) fn spent_shell(record: WeaponRecord) -> Shot {
    let at = Vec2::new(0.0, 300.0);
    Shot {
        position: at,
        moved_from: Some(at - Vec2::new(10.0, 0.0)),
        age: 3,
        ..Shot::launch(WeaponSpec::new(&record), A, A, &ShipState::default(), 90.0)
    }
}

/// `_HandleShot`'s pass alone, among two ships far off, its
/// sub-munitions read from `arsenal`.
fn handle_shots(combat: &mut Combat, arsenal: &Arsenal) {
    let mut ships = [Ship::at(1, -5000.0, 0.0), Ship::at(2, 5000.0, 0.0)];
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
}

/// Idle shots in slots `0..n` but the shell spent in slot 5.
fn shell_in_slot_5(shell: WeaponRecord, n: usize) -> Combat {
    let mut combat = Combat::default();
    for slot in 0..n {
        if slot == 5 {
            combat.put_in_flight(spent_shell(shell));
        } else {
            combat.put_in_flight(idle());
        }
    }
    combat
}

#[test]
fn a_sub_munition_released_behind_the_visit_sits_out_the_tick() {
    // `_HandleShot` visits slot 5, whose shell releases its sub-munition
    // into slot 2, which the loop has passed (@0x4435e-0x44393): it is not
    // moved this frame. Into slot 7, it is visited and moved.
    let (shell, arsenal) = one_sub();
    let mut behind = shell_in_slot_5(shell, 6);
    end_and_free(&mut behind, 2);
    handle_shots(&mut behind, &arsenal);
    let subs = sub_shots(&behind);
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].slot, 2);
    assert_eq!(subs[0].moved_from, None, "not moved");
    assert_eq!(
        subs[0].position,
        Vec2::new(0.0, 300.0),
        "where it was released"
    );
    let mut ahead = shell_in_slot_5(shell, 10);
    end_and_free(&mut ahead, 7);
    handle_shots(&mut ahead, &arsenal);
    let subs = sub_shots(&ahead);
    assert_eq!(subs[0].slot, 7);
    assert!(subs[0].moved_from.is_some(), "moved");
    assert!(
        !ahead.shots().iter().any(|shot| shot.slot == 5),
        "the shell freed"
    );
}

#[test]
fn a_held_slot_is_freed_at_its_own_visit() {
    // Slots 2 and 7 hold shots the collision passes ended. Slot 2's visit
    // frees it before slot 5's shell releases, so the sub-munition takes
    // it; slot 7, still held then, is passed over for slot 10.
    let (shell, arsenal) = one_sub();
    let mut both = shell_in_slot_5(shell, 10);
    for slot in [2, 7] {
        end_and_free(&mut both, slot);
        both.slots.hold(slot);
    }
    handle_shots(&mut both, &arsenal);
    assert_eq!(sub_shots(&both)[0].slot, 2, "freed at its visit");
    assert_eq!(both.slots.held().count(), 0, "both freed");
    let mut above = shell_in_slot_5(shell, 10);
    end_and_free(&mut above, 7);
    above.slots.hold(7);
    handle_shots(&mut above, &arsenal);
    let subs = sub_shots(&above);
    assert_eq!(subs[0].slot, 10, "7 still held at slot 5's visit");
    assert!(subs[0].moved_from.is_some(), "visited after slot 5");
    assert!(!above.slots.is_held(7), "freed at its own visit");
}

/// A ship's shot rolling `roll` (none for a shot that rolls no miss) put
/// in slot 0 and ended there, then a shell in slot 1 whose sub-munition
/// takes slot 0 at slot 1's visit: the sub-munition's roll, by
/// `source`'s reading of the leftover roll, and the combat.
fn sub_after(roll: Option<u32>, source: RuleSource) -> (Option<u32>, Combat) {
    let (shell, arsenal) = one_sub();
    let mut combat = Combat::default();
    combat.set_reused_shot_slot_roll(source);
    combat.put_in_flight(Shot {
        miss_roll: roll,
        ..idle()
    });
    combat.put_in_flight(spent_shell(shell));
    end_and_free(&mut combat, 0);
    handle_shots(&mut combat, &arsenal);
    let subs = sub_shots(&combat);
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].slot, 0);
    (subs[0].miss_roll, combat)
}

#[test]
fn a_sub_munition_carries_the_roll_its_slots_last_ship_fired_shot_left() {
    // A turret's roll of 90 left in slot 0 (`_SpawnShot` writes +0x42
    // only for a ship's own shot, @0x3e89e): the sub-munition carries it,
    // and misses the intact fighter C of class 80 but hits it disabled
    // (`_ShotCanHitShip` @0x448bd-0x448d5). By the other reading it
    // carries none and hits both.
    for (source, roll) in [(RuleSource::Engine, Some(90)), (RuleSource::Bible, None)] {
        for disabled in [false, true] {
            let (carried, mut combat) = sub_after(Some(90), source);
            assert_eq!(carried, roll, "{source:?}");
            combat.shots.retain(|shot| shot.weapon.id == WeaponId(148));
            let sub = &mut combat.shots[0];
            (sub.moved_from, sub.position) = (Some(INTO_C.0), INTO_C.1);
            let mut ships = three();
            if disabled {
                ships[1].condition = Condition::Disabled;
            }
            let hit = !walk(&mut combat, &mut ships).is_empty();
            let misses = roll.is_some() && !disabled;
            assert_eq!(hit, !misses, "{source:?}, disabled {disabled}");
        }
    }
}

#[test]
fn a_sub_munition_after_a_shot_of_no_roll_or_in_a_slot_never_used_carries_none() {
    assert_eq!(sub_after(None, RuleSource::Engine).0, None, "a gun's -1");
    let (shell, arsenal) = one_sub();
    let mut combat = Combat::default();
    combat.put_in_flight(spent_shell(shell));
    handle_shots(&mut combat, &arsenal);
    let subs = sub_shots(&combat);
    assert_eq!((subs[0].slot, subs[0].miss_roll), (1, None), "never used");
}

/// A shell whose `count` sub-munitions of `sub` (weapon 148) seek the
/// nearest ship (`Flags2` 0x0010) unspread, and the arsenal with them.
pub(super) fn seeking_shell(count: i16, sub: WeaponRecord) -> (WeaponRecord, Arsenal) {
    let (shell, _) = one_sub();
    let shell = WeaponRecord {
        sub_count: count,
        sub_theta: 0,
        flags2: SUBS_SEEK_NEAREST,
        ..shell
    };
    (shell, Arsenal::new(&[shell, sub], Vec::new()))
}

/// An unguided sub-munition, 148.
pub(super) fn unguided_sub() -> WeaponRecord {
    WeaponRecord {
        guidance: -1,
        count: 50,
        speed: 500,
        ..weapon(148)
    }
}

/// B, 50 below the shell spent at (0, 300), of turret miss class 80, as
/// `condition`; and C, 200 to its right, of class 100.
fn near_and_far(condition: Condition) -> [Ship; 2] {
    let near = Ship {
        hull: HullSpec {
            miss_class: 80,
            ..HullSpec::default()
        },
        condition,
        ..Ship::at(2, 0.0, 350.0)
    };
    [near, Ship::at(3, 200.0, 300.0)]
}

/// A turret's roll of 90 left in slot 0, then `shell` spent in slot 1,
/// aimed at C, releasing at slot 1's visit among `ships`, by `source`'s
/// reading of the leftover roll, in a system of `interference`: each
/// sub-munition's slot, target and heading.
fn seeks(
    (shell, arsenal): (WeaponRecord, Arsenal),
    ships: &mut [Ship],
    source: RuleSource,
    interference: i16,
) -> Vec<(usize, Option<ShipRef>, f32)> {
    let mut combat = Combat::default();
    combat.set_reused_shot_slot_roll(source);
    combat.put_in_flight(Shot {
        miss_roll: Some(90),
        ..idle()
    });
    combat.put_in_flight(Shot {
        aimed_at: Some(C),
        ..spent_shell(shell)
    });
    end_and_free(&mut combat, 0);
    let field = Field {
        interference,
        ..Field::default()
    };
    handle_in(&mut combat, ships, &arsenal, field);
    sub_shots(&combat)
        .iter()
        .map(|shot| (shot.slot, shot.target, shot.heading))
        .collect()
}

#[test]
fn two_sub_munitions_of_one_release_seek_different_ships_by_their_slots_rolls() {
    // The first takes slot 0 and its leftover roll of 90, over B's class
    // of 80: seeking after `_SpawnShot` seats it (@0x3f328, @0x3f3cf), it
    // passes the intact B (`_ShotCanHitShip` @0x448bd-0x448d5) and heads
    // at C. The second takes slot 2, never used, and heads at B. By the
    // other reading neither carries a roll, and both head at B.
    let seeking = || seeking_shell(2, unguided_sub());
    let mut ships = near_and_far(Condition::Intact);
    assert_eq!(
        seeks(seeking(), &mut ships, RuleSource::Engine, 0),
        [(0, Some(C), 90.0), (2, Some(B), 180.0)]
    );
    assert_eq!(
        seeks(seeking(), &mut ships, RuleSource::Bible, 0),
        [(0, Some(B), 180.0), (2, Some(B), 180.0)]
    );
}

#[test]
fn a_disabled_near_ship_draws_every_sub_munition_whatever_its_roll() {
    for source in RuleSource::ALL {
        let mut ships = near_and_far(Condition::Disabled);
        let seen = seeks(seeking_shell(2, unguided_sub()), &mut ships, source, 0);
        assert_eq!(
            seen,
            [(0, Some(B), 180.0), (2, Some(B), 180.0)],
            "{source:?}"
        );
    }
}

#[test]
fn an_interfered_homing_sub_munition_seeks_as_a_clear_one() {
    // A homing `Seeker` 0x0008 sub-munition rolls interference as
    // `_SpawnShot` seats it (@0x3eab2-0x3eb43). Jammed or not, it seeks
    // as a clear one, as `_ShotCanHitShip` rejects only jam 998 (@0x6a75 ->
    // @0x447f7): of `Flags2` 0x0008, it takes the nearest, B, here of a
    // class its slot's leftover roll of 90 is under. Slot 0, behind the
    // visit, sits out the tick.
    let homing = WeaponRecord {
        guidance: 1,
        seeker: SENSOR_INTERFERENCE,
        flags2: PROXIMITY_BY_OTHERS,
        ..unguided_sub()
    };
    for (interference, seen) in [(100, (0, Some(B), 180.0)), (0, (0, Some(B), 180.0))] {
        let mut ships = near_and_far(Condition::Intact);
        ships[0].hull.miss_class = 100;
        let shell = seeking_shell(1, homing);
        let subs = seeks(shell, &mut ships, RuleSource::Engine, interference);
        assert_eq!(subs, [seen], "interference {interference}");
    }
}

/// A shell releasing one turret (4) of weapon 148, and the arsenal with
/// it, class 128 without `Flags3` 0x0040 and class 129 with it.
fn turret_cluster() -> (WeaponRecord, Arsenal) {
    let (shell, _) = one_sub();
    let sub = WeaponRecord {
        guidance: 4,
        count: 50,
        speed: 500,
        ..weapon(148)
    };
    let class = |id: i16, flags3: u16| crate::catalog::HullRecord {
        flags3,
        ..nova_sim_catalog::fixture::hull(id)
    };
    let classes = vec![class(128, 0), class(129, 0x0040)];
    (shell, Arsenal::new(&[shell, sub], classes))
}

/// NPC `id` of `class` at berth 3, far off.
fn at_berth_3(id: u32, class: i16) -> Ship {
    Ship {
        class: ShipId(class),
        slot: Slot::Berth(3),
        ..Ship::at(id, 4000.0, 4000.0)
    }
}

/// C's spent `shell` from berth 3, carrying `over` and aimed at B.
fn shell_from_berth_3(shell: WeaponRecord, over: bool) -> Shot {
    Shot {
        firer_slot: Some(Slot::Berth(3)),
        turret_over: over,
        aimed_at: Some(B),
        firer: C,
        fleet: C,
        ..spent_shell(shell)
    }
}

/// `_HandleShot`'s pass among `ships`, sub-munitions read from `arsenal`.
fn handle_among(combat: &mut Combat, ships: &mut [Ship], arsenal: &Arsenal) {
    handle_in(combat, ships, arsenal, Field::default());
}

/// `_HandleShot`'s pass among `ships` in `field`, sub-munitions read from
/// `arsenal`.
pub(super) fn handle_in(combat: &mut Combat, ships: &mut [Ship], arsenal: &Arsenal, field: Field) {
    let govts = Governments::default();
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal,
        govts: &govts,
        rules: Rules::default(),
        field,
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.fly_shots(&mut fighters, arena, &mut NeverFires);
}

/// The one sub-munition of a shell C fired from berth 3 as a ship of
/// `firer` class, carrying its bit, released once `now` holds berth 3
/// (none: empty), with B far off, by `source`'s reading.
fn released_from_berth_3(firer: i16, now: Option<Ship>, source: RuleSource) -> (Shot, Combat) {
    let (shell, arsenal) = turret_cluster();
    let mut combat = Combat::default();
    combat.set_submunition_source_slot(source);
    let mut fired_by = [at_berth_3(3, firer)];
    let over = arsenal.hull(ShipId(firer)).turret_shots_over;
    combat.note_slots(&fired_by.iter_mut().map(Ship::fighter).collect::<Vec<_>>());
    combat.put_in_flight(shell_from_berth_3(shell, over));
    let mut ships: Vec<Ship> = now.into_iter().collect();
    ships.push(Ship::at(2, -4000.0, 4000.0));
    handle_among(&mut combat, &mut ships, &arsenal);
    let sub = *sub_shots(&combat)[0];
    (sub, combat)
}

#[test]
fn a_sub_munition_takes_its_layer_from_the_ship_now_in_its_source_slot() {
    // `_SpawnShot` reads class +0x9ec of whatever ship the source slot
    // holds as the sub-munition spawns (@0x3f0d6-0x3f109).
    for (firer, now, engine, other) in [
        (128, 129, ShotLayer::OverShips, ShotLayer::Shots),
        (129, 128, ShotLayer::Shots, ShotLayer::OverShips),
    ] {
        let taken = || Some(at_berth_3(9, now));
        let (sub, _) = released_from_berth_3(firer, taken(), RuleSource::Engine);
        assert_eq!(sub.layer, engine, "{firer} then {now}");
        let (sub, _) = released_from_berth_3(firer, taken(), RuleSource::Bible);
        assert_eq!(sub.layer, other, "{firer} then {now}: the parent's bit");
    }
}

#[test]
fn a_sub_munition_is_disable_only_when_the_ship_now_in_its_source_slot_plunders() {
    // The ship now at berth 3 plunders B (`_AIShipIsPlunderingShip`
    // @0x3e826-0x3e899); C, which fired, did not.
    let plunderer = || {
        Some(Ship {
            plundering: true,
            target: Some(B),
            ..at_berth_3(9, 128)
        })
    };
    let (sub, _) = released_from_berth_3(128, plunderer(), RuleSource::Engine);
    assert!(sub.disable_only, "the slot's ship plunders");
    let (sub, _) = released_from_berth_3(128, plunderer(), RuleSource::Bible);
    assert!(!sub.disable_only, "the firer, gone, did not");
}

#[test]
fn a_sub_munition_from_an_empty_slot_reads_its_last_ship() {
    // C fired from berth 3 as a class-128 ship; a class-129 ship came and
    // went; the empty slot keeps its record (`_HandleShip` @0x3359a).
    let (shell, arsenal) = turret_cluster();
    for (source, layer) in [
        (RuleSource::Engine, ShotLayer::OverShips),
        (RuleSource::Bible, ShotLayer::Shots),
    ] {
        let mut combat = Combat::default();
        combat.set_submunition_source_slot(source);
        for (id, class) in [(4, 128), (5, 129)] {
            let mut ship = [at_berth_3(id, class)];
            combat.note_slots(&ship.iter_mut().map(Ship::fighter).collect::<Vec<_>>());
        }
        combat.put_in_flight(shell_from_berth_3(shell, false));
        handle_among(&mut combat, &mut [Ship::at(2, -4000.0, 4000.0)], &arsenal);
        assert_eq!(sub_shots(&combat)[0].layer, layer, "{source:?}");
    }
}

#[test]
fn a_sub_munitions_own_sub_munitions_read_the_same_slot() {
    // The new shot's source is its parent's (+0x2c, @0x3e7c4-0x3e7c8).
    let (sub, _) = released_from_berth_3(128, None, RuleSource::Engine);
    assert_eq!(sub.firer_slot, Some(Slot::Berth(3)));
    let (shell, _) = turret_cluster();
    let parent = shell_from_berth_3(shell, false);
    let arsenal_sub = WeaponSpec::new(&WeaponRecord {
        count: 50,
        ..weapon(148)
    });
    let grandchildren = submunition::release(&sub_of(&parent), &arsenal_sub, (None, false));
    assert!(
        grandchildren
            .iter()
            .all(|shot| shot.firer_slot == Some(Slot::Berth(3)))
    );
}

/// `parent` as one of its own sub-munitions, a generation on, spent and
/// releasing in turn.
fn sub_of(parent: &Shot) -> Shot {
    Shot {
        generation: parent.generation + 1,
        ..*parent
    }
}

#[test]
fn the_shot_slots_keep_their_places_and_rolls_through_a_system_change() {
    // `_InitSprites` lays the shot sprites out once (@0x1fa90-0x1fb83);
    // a system change only frees the slots, and a homing shot in slot 0
    // keeps the place slot 0 took in the homing layer.
    let mut combat = Combat::default();
    combat.put_in_flight(in_layer(ShotLayer::Homing, 0));
    combat.put_in_flight(Shot {
        miss_roll: Some(40),
        ..in_layer(ShotLayer::Homing, 1)
    });
    combat.slots.hold(2);
    let history = combat.shot_slots();
    combat.clear();
    assert_eq!(combat.slots.held().count(), 0, "every slot free");
    assert_eq!(combat.shot_slots(), history, "the places and rolls kept");
    combat.put_in_flight(in_layer(ShotLayer::Homing, 2));
    assert_eq!(drawn(&combat), [2]);
    assert!(
        combat.slots.rank(0) < combat.slots.rank(1),
        "slot 0's place"
    );
    assert_eq!(combat.slots.roll(1), Some(40));
}

#[test]
fn a_fight_starts_from_the_shot_slots_an_earlier_one_left_none_held() {
    let mut earlier = Combat::default();
    earlier.put_in_flight(in_layer(ShotLayer::Homing, 0));
    earlier.slots.hold(1);
    let history = earlier.shot_slots();
    assert!(!history.is_held(1), "the history holds none");
    assert_eq!(
        history.rank(0).map(|(layer, _)| layer),
        Some(ShotLayer::Homing)
    );
    let mut held = history.clone();
    held.hold(5);
    let mut later = Combat::default();
    later.set_shot_slots(held);
    assert_eq!(later.shot_slots(), history);
    assert_eq!(later.first_free_slot(), Some(0), "none held");
    assert_eq!(later.slots.rank(0), history.rank(0));
}

/// [`idle`] of `Flags` `flags` and `BeamWidth` 4.
fn idle_of(flags: u16) -> Shot {
    let mut shot = idle();
    shot.weapon.flags = flags;
    shot.weapon.beam_width = 4;
    shot
}

/// `n` of `_HandleShot`'s passes among two ships far off.
fn visits(combat: &mut Combat, n: usize) {
    let arsenal = Arsenal::default();
    for _ in 0..n {
        handle_shots(combat, &arsenal);
    }
}

#[test]
fn a_spinning_shot_runs_its_slots_spin_timer_and_the_next_shot_there_starts_on_it() {
    // `_HandleShot` runs +0x1c each live visit of a `Flags` 0x0001 shot,
    // stepping at `BeamWidth` (@0x35884-0x358c4); `_SpawnShot` never
    // writes it (@0x3e550-0x3f143).
    let mut combat = Combat::default();
    combat.put_in_flight(idle_of(SPINS));
    assert_eq!(combat.shots()[0].spin_timer, 0, "a slot never used");
    visits(&mut combat, 6);
    assert_eq!(combat.slots.spin(0), 2, "6 passes at 4: stepped at 4");
    end_and_free(&mut combat, 0);
    combat.put_in_flight(idle_of(SPINS));
    assert_eq!(combat.shots()[0].spin_timer, 2, "the slot's leftover");
    visits(&mut combat, 3);
    assert_eq!(combat.slots.spin(0), 1, "(2 + 3) mod 4");
    assert_eq!(combat.slots.spin(1), 0, "another slot's own");
}

#[test]
fn a_shot_that_does_not_spin_leaves_its_slots_spin_timer_as_it_found_it() {
    let mut combat = Combat::default();
    combat.put_in_flight(idle_of(SPINS));
    visits(&mut combat, 3);
    end_and_free(&mut combat, 0);
    combat.put_in_flight(idle_of(0));
    assert_eq!(combat.shots()[0].spin_timer, 3, "it starts on it too");
    visits(&mut combat, 5);
    assert_eq!(combat.slots.spin(0), 3, "untouched");
    end_and_free(&mut combat, 0);
    combat.put_in_flight(idle_of(SPINS));
    assert_eq!(combat.shots()[0].spin_timer, 3);
}

#[test]
fn a_sub_munition_put_in_a_slot_starts_on_its_spin_timer() {
    let (shell, arsenal) = one_sub();
    let mut combat = Combat::default();
    combat.put_in_flight(idle_of(SPINS));
    visits(&mut combat, 3);
    combat.put_in_flight(spent_shell(shell));
    end_and_free(&mut combat, 0);
    handle_shots(&mut combat, &arsenal);
    let subs = sub_shots(&combat);
    assert_eq!((subs[0].slot, subs[0].spin_timer), (0, 3));
}

#[test]
fn by_the_other_reading_every_shot_starts_its_spin_timer_on_0() {
    let mut combat = Combat::default();
    combat.set_reused_shot_slot_spin(RuleSource::Bible);
    combat.put_in_flight(idle_of(SPINS));
    visits(&mut combat, 3);
    end_and_free(&mut combat, 0);
    combat.put_in_flight(idle_of(SPINS));
    assert_eq!(combat.shots()[0].spin_timer, 0);
    assert_eq!(combat.slots.spin(0), 0, "the slot's own put back too");
    visits(&mut combat, 1);
    assert_eq!(combat.slots.spin(0), 1, "and running from there");
}

#[test]
fn the_shot_slots_keep_their_spin_timers_through_a_system_change() {
    let mut combat = Combat::default();
    combat.put_in_flight(idle_of(SPINS));
    visits(&mut combat, 2);
    let history = combat.shot_slots();
    assert_eq!(history.spin(0), 2);
    combat.clear();
    let mut later = Combat::default();
    later.set_shot_slots(history);
    later.put_in_flight(idle_of(SPINS));
    assert_eq!(later.shots()[0].spin_timer, 2);
}
