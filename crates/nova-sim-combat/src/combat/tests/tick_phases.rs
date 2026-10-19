//! The fight's tests of its phases' order (rdm weapons-fidelity phase 28):
//! `_DoPlayGameWork` runs the collision passes, then `_AIDispatch` (the
//! NPCs' point defence), then `_HandleShot` (the shots' ends of life,
//! moves, locks and holds), then `_HandleShip` (each NPC's reload and
//! pick), then `_HandleBeams`.

use super::*;

/// The tick numbers, from 1, on which `ship` was struck over `ticks`
/// ticks of `ships`.
fn struck_on(ships: &mut [Ship], ship: ShipRef, ticks: u32) -> Vec<u32> {
    let mut combat = Combat::default();
    let mut on = Vec::new();
    for tick_no in 1..=ticks {
        tick(&mut combat, ships, &NovaDisable);
        if combat
            .take_strikes()
            .iter()
            .any(|strike| strike.ship == ship)
        {
            on.push(tick_no);
        }
    }
    on
}

#[test]
fn an_npcs_shot_first_moves_in_the_tick_after_it_is_fired() {
    // `_HandleShip` fires the pick (@0x34ffd) after `_HandleShot` has
    // moved the shots (@0x44385), so its shot first moves in the next
    // frame's `_HandleShot`.
    let mut combat = Combat::default();
    let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster())];
    tick(&mut combat, &mut ships, &NovaDisable);
    let fired = combat.shots()[0];
    assert_eq!(fired.position, Vec2::ZERO, "at its launch point");
    assert_eq!(fired.moved_from, None, "not moved");
    ships[0].trigger = QUIET;
    tick(&mut combat, &mut ships, &NovaDisable);
    let moved = combat.shots()[0];
    assert!((moved.position.x - 15.0).abs() < 1e-4, "{moved:?}");
    assert_eq!(moved.moved_from, Some(Vec2::ZERO), "moved once");
}

#[test]
fn an_npcs_shot_is_first_tested_two_ticks_after_it_is_fired() {
    // Fired on tick 1, moved on tick 2, its move tested by tick 3's
    // collision passes.
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(blaster()),
        Ship::at(2, 15.0, 0.0),
    ];
    let on = struck_on(&mut ships, B, 3);
    assert_eq!(on.first(), Some(&3), "{on:?}");
}

/// A one-at-a-time weapon (`Flags3` 0x0004) of Reload 3, 10 pixels a tick
/// for 10 ticks.
fn single() -> WeaponRecord {
    WeaponRecord {
        reload: 3,
        count: 10,
        speed: 1000,
        flags3: 0x0004,
        ..weapon(128)
    }
}

#[test]
fn an_npcs_one_at_a_time_weapon_fires_again_reload_ticks_after_its_shots_last_move() {
    // `_HandleShot` holds the reload at 3 on the shot's every move
    // (@0x35c09), then `_HandleShip` counts it down (@0x33896) before the
    // pick (@0x34ffd): fired on tick 0, the shot moves on ticks 1 to 10,
    // the 10th move spending it, and the reload reads 2, 1 then 0 on
    // ticks 10 to 12.
    let firer = || Ship::at(1, 0.0, 0.0).armed(single());
    assert_eq!(
        fire_ticks(&mut [firer(), Ship::at(2, 900.0, 0.0)], 25),
        [0, 12, 24],
        "Reload ticks after its last move"
    );
    // It moves onto B, 60 pixels off, on tick 5, and the collision passes
    // take it on tick 6, before `_HandleShot` could hold the reload again.
    assert_eq!(
        fire_ticks(&mut [firer(), Ship::at(2, 60.0, 0.0)], 9),
        [0, 7],
        "Reload ticks after its last move, onto its victim"
    );
}

#[test]
fn the_players_reload_counts_down_after_its_trigger() {
    // `_HandlePlayer` counts its reloads down (@0x6ae25-0x6ae61) after
    // its trigger (@0x6ab55) and before `_HandleShot` holds them: its
    // shot moves on the tick it is fired, ticks 0 to 9, and the reload
    // reads 2, 1 then 0 after the triggers of ticks 10 to 12.
    let player = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        ..Ship::at(1, 0.0, 0.0).armed(single())
    };
    assert_eq!(
        fire_ticks(&mut [player, Ship::at(2, 900.0, 0.0)], 14),
        [0, 13]
    );
    let every = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        ..Ship::at(1, 0.0, 0.0).armed(WeaponRecord {
            flags3: 0,
            ..single()
        })
    };
    assert_eq!(
        fire_ticks(&mut [every, Ship::at(2, 900.0, 0.0)], 7),
        [0, 3, 6],
        "every Reload ticks"
    );
}

/// A flak shell, 10 pixels a tick for 3 ticks, detonating at the end of
/// its life with a 40-pixel blast of the blaster's 4 mass and 10 energy.
fn flak() -> WeaponRecord {
    WeaponRecord {
        count: 3,
        speed: 1000,
        blast_radius: 40,
        flags: 0x8000,
        ..blaster()
    }
}

/// The advance pass at single speed, by the default rules, among no
/// sub-munitions, no governments' relations, on frame 0.
fn arena<'a>(arsenal: &'a Arsenal, govts: &'a Governments) -> Arena<'a> {
    Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal,
        govts,
        rules: Rules::default(),
        field: Field::default(),
    }
}

#[test]
fn a_spent_shots_blast_lands_after_the_npcs_point_defence_and_before_their_picks() {
    // `_AIDispatch` runs the NPCs' point defence (@0x4429d), then
    // `_HandleShot` the spent shell's end of life (@0x36195), then
    // `_HandleShip` the picks (@0x44483). A, frail, carries the Quad and a
    // pick; B's missile closes on it, and C's shell, fired on tick 1 to
    // pass 35 below A, is spent on tick 4 and blasts A on tick 5.
    let [mut a, b] = missile_attack(quad());
    a.armament = Armament::new([
        (WeaponSpec::new(&quad()), 1),
        (WeaponSpec::new(&slow_blaster()), 1),
    ]);
    a.trigger = Trigger {
        point_defence: false,
        ..QUIET
    };
    a.reserves.shield.now = 0.0;
    a.reserves.armor.now = 10.5;
    let mut ships = [a, b, Ship::at(3, -30.0, 35.0).armed(flak())];
    let mut combat = Combat::default();
    tick(&mut combat, &mut ships, &NovaDisable);
    ships[1].trigger = QUIET;
    ships[2].trigger = QUIET;
    for _ in 2..=4 {
        tick(&mut combat, &mut ships, &NovaDisable);
    }
    assert_eq!(ships[0].condition, Condition::Intact);
    combat.take_events();
    ships[0].trigger = Trigger {
        primary: true,
        ..QUIET
    };
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[0].condition, Condition::Disabled, "blasted");
    let fired: Vec<WeaponId> = combat
        .take_events()
        .iter()
        .filter_map(|event| match *event {
            CombatEvent::Fired { ship, weapon, .. } if ship == A => Some(weapon),
            _ => None,
        })
        .collect();
    assert_eq!(fired, [WeaponId(133)], "its point defence, not its pick");
}

#[test]
fn a_lock_lands_in_the_shot_step() {
    // `_HandleShot` makes the lock (@0x35855) as it moves the missile,
    // before `_HandleShip` and `_HandleBeams`.
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = arena(&arsenal, &govts);
    let mut ships = [player_with(missile()), far_b()];
    let mut combat = Combat::default();
    let mut locked = Vec::new();
    for tick_no in 1..=40 {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        combat.defend_player(&mut fighters, arena, &mut NeverFires);
        combat.fire_player(&mut fighters, arena, &mut NeverFires);
        combat.test_hits(&mut fighters, arena, Scenery::default(), &mut NeverFires);
        combat.defend_npcs(&mut fighters, arena, &mut NeverFires);
        combat.fly_shots(&mut fighters, arena, &mut NeverFires);
        let shot_step = combat.take_strikes();
        locked.extend(
            shot_step
                .iter()
                .filter(|strike| strike.lock)
                .map(|_| tick_no),
        );
        combat.fire_npcs(&mut fighters, arena, &mut NeverFires);
        combat.close_tick(&mut fighters, arena, &mut NeverFires);
        let closed = combat.take_strikes();
        assert!(
            closed.iter().all(|strike| !strike.lock),
            "{tick_no}: {closed:?}"
        );
        drop(fighters);
        ships[0].trigger = QUIET;
    }
    assert_eq!(locked.len(), 1, "{locked:?}");
}

#[test]
fn a_breakup_an_expiry_begins_takes_no_tick_off_on_its_first() {
    // Fired on tick 1, the shell is spent on tick 4 and its blast takes
    // B's last armour on tick 5, in `_HandleShot`; the countdown starts
    // there, set and marked starting (@0x2d834-0x2d850).
    let mut b = Ship::at(2, 30.0, 35.0);
    b.reserves.shield.now = 0.0;
    b.reserves.armor.now = 1.0;
    b.hull.death_delay = 5;
    let mut ships = [Ship::at(1, 0.0, 0.0).armed(flak()), b];
    let mut combat = Combat::default();
    for _ in 1..=5 {
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
    }
    let starting = Condition::Dying {
        ticks_left: 5,
        starting: true,
    };
    assert_eq!(ships[1].condition, starting);
    tick(&mut combat, &mut ships, &NovaDisable);
    let counting = Condition::Dying {
        ticks_left: 4,
        starting: false,
    };
    assert_eq!(ships[1].condition, counting);
}

/// A shot of C's that the collision passes end on B, its move from 10 to
/// the left of B's place in [`missile_attack`] onto it.
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

#[test]
fn under_the_caps_an_npcs_point_defence_finds_the_slots_the_hit_pass_ended_still_held() {
    // `_HandleShot` frees a shot the collision passes ended (life -1) only
    // at its visit (@0x356c1), after `_AIDispatch` runs the point defence
    // (@0x4429d): 127 in flight, one of them ended on B, leave A's point
    // defence no slot.
    for capped in [false, true] {
        let (mut combat, mut ships) = last_slot_left(A, capped);
        combat.put_in_flight(ending_on_b());
        ships[0].trigger = QUIET;
        tick(&mut combat, &mut ships, &NovaDisable);
        assert!(ships[1].reserves.shield.now < 10.0, "{capped}: B struck");
        let (pd, _) = in_flight_by(&combat, A);
        assert_eq!(pd, usize::from(!capped), "{capped}");
    }
}

#[test]
fn under_the_caps_an_npcs_pick_after_the_shots_fly_finds_them_freed() {
    // `_HandleShip`'s picks (@0x44483) come after `_HandleShot` has freed
    // the shots the collision passes ended (@0x356c1) and the spent ones
    // whose end of life it ran (@0x3617c).
    let spent = Shot::launch(
        WeaponSpec::new(&WeaponRecord {
            count: 1,
            speed: 0,
            ..weapon(141)
        }),
        C,
        C,
        &ShipState {
            position: Vec2::new(-50_000.0, 0.0),
            ..ShipState::default()
        },
        0.0,
    );
    for capped in [false, true] {
        // One ended on B by the collision passes.
        let (mut combat, mut ships) = last_slot_left(A, capped);
        combat.put_in_flight(ending_on_b());
        ships[0].trigger = Trigger {
            only: Some(WeaponId(128)),
            point_defence: false,
            ..QUIET
        };
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(in_flight_by(&combat, A), (0, 1), "{capped}: ended");
        // One whose move spends it on the tick before, ended in
        // `_HandleShot`.
        let (mut combat, mut ships) = last_slot_left(A, capped);
        combat.put_in_flight(spent);
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.shots().len(), MAX_SHOTS, "{capped}: spent");
        ships[0].trigger = Trigger {
            only: Some(WeaponId(128)),
            point_defence: false,
            ..QUIET
        };
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(in_flight_by(&combat, A), (0, 1), "{capped}: spent");
    }
}

#[test]
fn under_the_caps_an_npcs_point_defence_finds_the_missiles_last_ticks_beams_downed_still_held() {
    // `_HandleBeams` downs a missile last in the frame (@0x4453d, @0x30ab1),
    // and the next frame's `_HandleShot` frees it, after `_AIDispatch`.
    let fragile = drawn(
        WeaponRecord {
            speed: 500,
            durability: 0,
            ..missile()
        },
        18,
    );
    for capped in [false, true] {
        // 126 others; B's missile, which A's beam downs the tick after it
        // is fired; and F's at E, fired that tick.
        let mut combat = crowded(MAX_SHOTS - 2, capped);
        let mut a = Ship::at(1, 0.0, 0.0).armed(pd_beam());
        a.trigger = Trigger {
            point_defence: false,
            ..QUIET
        };
        let b = Ship::at(2, 0.0, -200.0)
            .armed(fragile)
            .facing(180.0)
            .targeting(A);
        let e_ref = ShipRef::Npc(NpcId(5));
        let mut e = Ship::at(5, 300.0, 0.0).armed(quad());
        e.trigger = Trigger {
            point_defence: false,
            ..QUIET
        };
        let mut f = Ship::at(6, 300.0, -200.0)
            .armed(fragile)
            .facing(180.0)
            .targeting(e_ref);
        f.trigger = QUIET;
        let mut ships = [a, b, e, f];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
        ships[1].trigger = QUIET;
        ships[3] = Ship::at(6, 300.0, -200.0)
            .armed(fragile)
            .facing(180.0)
            .targeting(e_ref);
        tick(&mut combat, &mut ships, &NovaDisable);
        let downed = combat
            .take_events()
            .iter()
            .any(|event| matches!(event, CombatEvent::ShotDown { .. }));
        assert!(downed, "{capped}");
        assert_eq!(combat.shots().len(), MAX_SHOTS - 1, "{capped}: F's in");
        ships[2].trigger = QUIET;
        ships[3].trigger = QUIET;
        tick(&mut combat, &mut ships, &NovaDisable);
        let (pd, _) = in_flight_by(&combat, e_ref);
        assert_eq!(pd, usize::from(!capped), "{capped}");
    }
}

/// The ships whose Quad fired among `events`, in order.
fn quad_firers(events: &[CombatEvent]) -> Vec<ShipRef> {
    events
        .iter()
        .filter_map(|event| match *event {
            CombatEvent::Fired {
                ship,
                weapon: WeaponId(133),
                ..
            } => Some(ship),
            _ => None,
        })
        .collect()
}

#[test]
fn double_times_second_pass_runs_the_players_point_defence_and_no_npcs() {
    // `_HandlePlayer` runs on both passes, but `_AIDispatch`, which runs
    // the NPCs' point defence, on the advance pass alone (@0x4402a).
    let attacker = |id: u32, x: f32, at: ShipRef| {
        Ship::at(id, x, -200.0)
            .armed(missile())
            .facing(180.0)
            .targeting(at)
    };
    let guard = |ship: Ship| Ship {
        trigger: Trigger {
            point_defence: false,
            ..QUIET
        },
        ..ship.armed(quad()).facing(0.0)
    };
    let mut player = guard(Ship::at(0, 0.0, 0.0));
    player.id = ShipRef::Player;
    player.fleet = ShipRef::Player;
    let mut ships = [
        player,
        attacker(2, 0.0, ShipRef::Player),
        guard(Ship::at(1, 400.0, 0.0)),
        attacker(3, 400.0, A),
    ];
    let mut combat = Combat::default();
    tick(&mut combat, &mut ships, &NovaDisable);
    for ship in &mut ships {
        ship.trigger = QUIET;
    }
    combat.take_events();
    tick_on(&mut combat, &mut ships, GamePass::Again);
    assert_eq!(quad_firers(&combat.take_events()), [ShipRef::Player]);
    tick_on(
        &mut combat,
        &mut ships,
        GamePass::Advance { double_time: true },
    );
    assert_eq!(
        quad_firers(&combat.take_events()),
        [A],
        "the advance pass's, the player's reloading"
    );
}

#[test]
fn a_lock_meets_its_target_where_it_is_not_where_it_stood() {
    // `_HandleShot`'s lock (@0x35855) reads the target as it is when the
    // missile moves, where the collision passes and the ends of life meet
    // an NPC where it stood as the last tick ran.
    let mut ships = [player_with(missile()), far_b()];
    let mut combat = Combat::default();
    for _ in 1..=40 {
        // B moves between ticks, as a session's traffic would.
        ships[1].state.position.y += 10.0;
        let now = ships[1].state.position;
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = QUIET;
        let locked = combat.take_strikes().iter().any(|strike| strike.lock);
        if locked {
            let at: Vec<Vec2> = combat
                .take_events()
                .iter()
                .filter_map(|event| match *event {
                    CombatEvent::Hit { ship, at, .. } if ship == B => Some(at),
                    _ => None,
                })
                .collect();
            assert_eq!(at, [now]);
            return;
        }
        combat.take_events();
    }
    panic!("never locked");
}
