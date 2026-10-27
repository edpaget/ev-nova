//! The fight's tests of its phases' order (rdm weapons-fidelity phase 28):
//! `_DoPlayGameWork` runs the collision passes, then `_AIDispatch` (the
//! NPCs' point defence), then `_HandleShot` (the shots' ends of life,
//! moves, locks and holds), then `_HandleShip` (each NPC's reload and
//! pick), then `_HandleBeams`. Each test here runs the phase it pins:
//! the order between them is the session's
//! (`nova_sim::Session::pass_after_move`), and so are its tests (nova-sim's
//! `session::fight_order`).

use super::*;

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
fn the_players_reload_counts_down_after_its_trigger() {
    // `_HandlePlayer` counts its reloads down (@0x6ae25-0x6ae61) after
    // its trigger (@0x6ab55): a weapon of Reload 3 fires on every third
    // trigger. A one-at-a-time weapon's, held by `_HandleShot` between
    // the two, is the session's (nova-sim's
    // `session::fight_order::the_players_one_at_a_time_weapon_fires_again_reload_ticks_after_its_shots_last_move`).
    let every = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        ..Ship::at(1, 0.0, 0.0).armed(WeaponRecord {
            flags3: 0,
            ..single()
        })
    };
    let mut ships = [every, Ship::at(2, 900.0, 0.0)];
    let mut combat = Combat::default();
    let fired_on: Vec<u32> = (0..7)
        .filter(|_| {
            phase::fire_player(&mut combat, &mut ships, &NovaDisable);
            fired(&combat.take_events(), ShipRef::Player) > 0
        })
        .collect();
    assert_eq!(fired_on, [0, 3, 6], "every Reload ticks");
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

#[test]
fn a_lock_lands_in_the_shot_step() {
    // `_HandleShot` makes the lock (@0x35855) as it moves the missile:
    // the player's, fired once, moves in each `fly_shots` and locks on
    // the move its age passes 30, its 31st.
    let mut ships = [player_with(missile()), far_b()];
    let mut combat = Combat::default();
    phase::fire_player(&mut combat, &mut ships, &NovaDisable);
    ships[0].trigger = QUIET;
    assert_eq!(combat.take_strikes(), [], "fired");
    let mut locked = Vec::new();
    for flown in 1..=40 {
        phase::fly_shots(&mut combat, &mut ships, &NovaDisable);
        let strikes = combat.take_strikes();
        locked.extend(strikes.iter().filter(|strike| strike.lock).map(|_| flown));
    }
    assert_eq!(locked, [31]);
}

#[test]
fn a_breakup_an_expiry_begins_takes_no_tick_off_on_its_first() {
    // A fires the shell, its three moves spend it, and its blast at its
    // next visit takes B's last armour in `_HandleShot`; the countdown
    // starts at B's display, set and marked starting (@0x2d834-0x2d850),
    // and its next step counts it down.
    let mut b = Ship::at(2, 30.0, 35.0);
    b.reserves.shield.now = 0.0;
    b.reserves.armor.now = 1.0;
    b.hull.death_delay = 5;
    let mut ships = [Ship::at(1, 0.0, 0.0).armed(flak()), b];
    let mut combat = Combat::default();
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    ships[0].trigger = QUIET;
    for _ in 1..=4 {
        phase::fly_shots(&mut combat, &mut ships, &NovaDisable);
    }
    assert!(ships[1].reserves.armor.now < 1.0, "blasted");
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    let starting = Condition::Dying {
        ticks_left: 5,
        starting: true,
    };
    assert_eq!(ships[1].condition, starting);
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
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
        phase::test_hits(&mut combat, &mut ships, &NovaDisable);
        assert!(ships[1].reserves.shield.now < 10.0, "{capped}: B struck");
        phase::defend_npcs(&mut combat, &mut ships, &NovaDisable);
        let (pd, _) = in_flight_by(&combat, A);
        assert_eq!(pd, usize::from(!capped), "{capped}");
    }
}

#[test]
fn under_the_caps_an_npcs_pick_after_the_shots_fly_finds_them_freed() {
    // `_HandleShot` frees the shots the collision passes ended (@0x356c1)
    // and the spent ones whose end of life it runs (@0x3617c), and
    // `_HandleShip`'s picks (@0x44483), after it in the session's order
    // (`nova_sim::Session::pass_after_move`), find their slots free.
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
    let pick = Trigger {
        only: Some(WeaponId(128)),
        point_defence: false,
        ..QUIET
    };
    for capped in [false, true] {
        // One ended on B by the collision passes, freed at its visit.
        let (mut combat, mut ships) = last_slot_left(A, capped);
        combat.put_in_flight(ending_on_b());
        phase::test_hits(&mut combat, &mut ships, &NovaDisable);
        assert!(ships[1].reserves.shield.now < 10.0, "{capped}: B struck");
        phase::fly_shots(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = pick;
        phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(in_flight_by(&combat, A), (0, 1), "{capped}: ended");
        // One whose move spends it, ended at its next visit.
        let (mut combat, mut ships) = last_slot_left(A, capped);
        combat.put_in_flight(spent);
        phase::fly_shots(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.shots().len(), MAX_SHOTS, "{capped}: spent");
        phase::fly_shots(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.shots().len(), MAX_SHOTS - 1, "{capped}: freed");
        ships[0].trigger = pick;
        phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(in_flight_by(&combat, A), (0, 1), "{capped}: spent");
    }
}

#[test]
fn under_the_caps_an_npcs_point_defence_finds_the_missiles_last_ticks_beams_downed_still_held() {
    // `_HandleBeams` downs a missile last in the frame (@0x4453d, @0x30ab1),
    // and the next frame's `_HandleShot` frees it, after `_AIDispatch`
    // (the session's order, `nova_sim::Session::pass_after_move`).
    let fragile = drawn(
        WeaponRecord {
            speed: 500,
            durability: 0,
            ..missile()
        },
        18,
    );
    for capped in [false, true] {
        // 126 others; B's missile at A and F's at E, fired in the ship
        // loop, the last two slots.
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
        let f = Ship::at(6, 300.0, -200.0)
            .armed(fragile)
            .facing(180.0)
            .targeting(e_ref);
        let mut ships = [a, b, e, f];
        phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.shots().len(), MAX_SHOTS, "{capped}: both in");
        ships[0].trigger = QUIET;
        ships[1].trigger = QUIET;
        ships[3].trigger = QUIET;
        // A's point-defence beam, fired in the NPCs' point defence, downs
        // B's missile at `_HandleBeams`, its slot still held.
        phase::defend_npcs(&mut combat, &mut ships, &NovaDisable);
        phase::close_tick(&mut combat, &mut ships, &NovaDisable);
        let downed = combat
            .take_events()
            .iter()
            .any(|event| matches!(event, CombatEvent::ShotDown { .. }));
        assert!(downed, "{capped}");
        assert_eq!(combat.shots().len(), MAX_SHOTS - 1, "{capped}: F's left");
        ships[2].trigger = QUIET;
        phase::defend_npcs(&mut combat, &mut ships, &NovaDisable);
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
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    for ship in &mut ships {
        ship.trigger = QUIET;
    }
    combat.take_events();
    let player_defends = |combat: &mut Combat, ships: &mut [Ship], pass: GamePass| {
        phase::run_on(combat, ships, pass, |combat, fighters, arena, chance| {
            combat.defend_player(fighters, arena, chance);
        });
        quad_firers(&combat.take_events())
    };
    let npcs_defend = |combat: &mut Combat, ships: &mut [Ship], pass: GamePass| {
        phase::run_on(combat, ships, pass, |combat, fighters, arena, chance| {
            combat.defend_npcs(fighters, arena, None, false, chance);
        });
        quad_firers(&combat.take_events())
    };
    let again = GamePass::Again;
    let advance = GamePass::Advance { double_time: true };
    assert_eq!(
        player_defends(&mut combat, &mut ships, again),
        [ShipRef::Player]
    );
    assert_eq!(npcs_defend(&mut combat, &mut ships, again), []);
    assert_eq!(
        player_defends(&mut combat, &mut ships, advance),
        [],
        "the player's reloading"
    );
    assert_eq!(
        npcs_defend(&mut combat, &mut ships, advance),
        [A],
        "the advance pass's"
    );
}

#[test]
fn a_lock_meets_its_target_where_it_is() {
    // `_HandleShot`'s lock (@0x35855) reads the target as it is when the
    // missile moves.
    let mut ships = [player_with(missile()), far_b()];
    let mut combat = Combat::default();
    phase::fire_player(&mut combat, &mut ships, &NovaDisable);
    ships[0].trigger = QUIET;
    combat.take_events();
    for _ in 1..=40 {
        // B moves between the moves, as a session's traffic would.
        ships[1].state.position.y += 10.0;
        let now = ships[1].state.position;
        phase::fly_shots(&mut combat, &mut ships, &NovaDisable);
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

/// A's laser of `laser_mass` mass and 5 energy damage on `struck`, 100 to
/// its right, the struck ship's shield at `shield` of 100 regenerating 1 a
/// tick and its armour at `armor` of 30 regenerating 1 a tick.
fn beamed(struck: ShipRef, laser_mass: i16, shield: f32, armor: f32) -> [Ship; 2] {
    let mut b = Ship::at(2, 100.0, 0.0);
    b.id = struck;
    b.fleet = struck;
    b.reserves.shield = Gauge {
        now: shield,
        max: 100.0,
    };
    b.reserves.armor.now = armor;
    b.shield_regen = 1.0;
    b.armor_regen = 1.0;
    let laser = WeaponRecord {
        mass_dmg: laser_mass,
        ..laser()
    };
    [Ship::at(1, 0.0, 0.0).armed(laser), b]
}

#[test]
fn an_npc_intact_at_its_step_regenerates_though_a_beam_disables_it_after() {
    // B is intact at its `_HandleShip` (the frame-start `_IsDisabled`
    // @0x34424), so its armour regenerates, 25 + 1; the beam A fires there
    // takes 20 at `_HandleBeams` and disables it. That the regeneration
    // comes first is the session's order
    // (`nova_sim::Session::pass_after_move`).
    let mut ships = beamed(B, 20, 0.0, 25.0);
    let mut combat = Combat::default();
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[1].condition, Condition::Intact);
    assert_eq!(ships[1].reserves.armor.now, 26.0, "regenerated");
    phase::close_tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[1].condition, Condition::Disabled);
    assert_eq!(ships[1].reserves.armor.now, 6.0);
}

#[test]
fn a_player_whose_armour_is_gone_before_its_regeneration_regenerates_its_shield_alone() {
    // `_HandlePlayer` gates the shield on its frame-start flags alone
    // (@0x6cfa2) but the armour on a live `_IsDisabled` too (@0x6d035):
    // armour taken to none since the frame began (a sabotage kill, a
    // self-destruct) regenerates nothing, and the display step breaks it
    // up.
    let mut ships = beamed(ShipRef::Player, 0, 50.0, 0.0);
    ships[0].trigger = QUIET;
    phase::regenerate_player(&mut Combat::default(), &mut ships, &NovaDisable);
    assert_eq!(ships[1].reserves.shield.now, 51.0, "its shield");
    assert_eq!(ships[1].reserves.armor.now, 0.0, "no armour");
}

#[test]
fn a_player_disabled_as_its_frame_began_regenerates_no_shield() {
    // `_HandlePlayer` gates the shield on its frame-start flags
    // (@0x6cfa2): a disabled player regenerates none.
    let mut ships = beamed(ShipRef::Player, 0, 50.0, 5.0);
    ships[0].trigger = QUIET;
    ships[1].condition = Condition::Disabled;
    phase::regenerate_player(&mut Combat::default(), &mut ships, &NovaDisable);
    assert_eq!(ships[1].reserves.shield.now, 50.0);
    assert_eq!(ships[1].reserves.armor.now, 5.0);
}
