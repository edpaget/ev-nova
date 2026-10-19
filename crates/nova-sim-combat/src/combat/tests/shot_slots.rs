//! The original's shot slots in the fight (rdm weapons-fidelity phase
//! 34): which slot each shot takes (`_SpawnShot` @0x3e5ab-0x3e5ce), when
//! `_HandleShot` frees it (@0x3617c), and the order each layer keeps
//! (`_PutSpriteInLayer` @0x766c-0x76b1).

use super::collision_walk::{BESIDE_C, D, INTO_C, fly, fused, missile, rolled_over, three, walk};
use super::*;

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
        tick(&mut combat, &mut ships, &NovaDisable);
        assert!(ships[1].reserves.shield.now < 10.0, "B struck");
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
    combat.defend_npcs(&mut fighters, arena, &mut NeverFires);
    assert_eq!(slots(&combat), [0], "spent");
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
        tick(&mut combat, &mut ships, &NovaDisable);
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
fn spent_shell(record: WeaponRecord) -> Shot {
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
