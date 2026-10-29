//! The fight's tests of each ship's display step (rdm weapons-fidelity
//! phase 39): `_HandleShipDisplay` starts a breakup's countdown and makes
//! its final explosion, for the player before the collision passes
//! (@0x43e29) and for each NPC right after its own `_HandleShip`, slot by
//! slot (@0x444ad), before `_HandleBeams` (@0x4453d).

use super::*;

/// Starting, its timer set this pass or unset.
fn starting(ticks_left: u32) -> Condition {
    Condition::Dying {
        ticks_left,
        starting: true,
    }
}

/// Counting down, its timer at `ticks_left`.
fn counting(ticks_left: u32) -> Condition {
    Condition::Dying {
        ticks_left,
        starting: false,
    }
}

/// The advance pass at single speed, by the default rules.
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
fn a_breakup_a_beam_begins_starts_its_countdown_at_the_next_display() {
    // `_HandleBeams` (@0x4453d) runs after B's display (@0x444ad), the
    // session's order (`nova_sim::Session::pass_after_move`): the breakup
    // the beam A fires begins is unset until B's next display sets it
    // (@0x2d834), and B explodes at 2 (@0x2d85a), 5 - 2 of its own steps
    // later.
    let mut b = Ship::at(2, 100.0, 0.0);
    b.reserves = Reserves::full(0.0, 30.0, 100.0);
    b.reserves.armor.now = 1.0;
    b.hull.death_delay = 5;
    let laser = WeaponRecord {
        mass_dmg: 20,
        ..laser()
    };
    let mut ships = [Ship::at(1, 0.0, 0.0).armed(laser), b];
    let mut combat = Combat::default();
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    ships[0].trigger = QUIET;
    phase::close_tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[1].condition, starting(0), "unset by the beam");
    let mut seen = Vec::new();
    for _ in 2..=6 {
        phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
        seen.push(ships[1].condition);
    }
    assert_eq!(
        seen,
        [
            starting(5),
            counting(4),
            counting(3),
            Condition::Destroyed,
            Condition::Destroyed
        ]
    );
}

#[test]
fn a_breakup_the_collision_passes_begin_starts_at_that_ticks_display() {
    // The collision passes (@0x43e70) leave the breakup unset; B's own
    // display that frame (@0x444ad) sets it.
    let mut hulk = Ship::at(2, 0.0, 0.0);
    hulk.reserves = Reserves::full(0.0, 1.0, 100.0);
    hulk.hull.death_delay = 5;
    let mut ships = [hulk];
    let mut combat = Combat::default();
    combat.put_in_flight(flown_by_c(blaster(), Vec2::new(5.0, 0.0)));
    phase::test_hits(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[0].condition, starting(0), "unset by the passes");
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[0].condition, starting(5), "set at its display");
}

#[test]
fn the_players_breakup_starts_at_its_next_display_before_the_collision_passes() {
    // The player's display (@0x43e29) comes before the collision passes
    // (the session's order, `nova_sim::Session::pass_after_move`), so a
    // breakup they begin is unset until its next display sets it, at
    // three times its `DeathDelay` (@0x2d842-0x2d850).
    let mut player = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        slot: Slot::Player,
        ..Ship::at(0, 0.0, 0.0)
    };
    player.reserves = Reserves::full(0.0, 1.0, 100.0);
    player.hull.death_delay = 4;
    let mut ships = [player];
    let mut combat = Combat::default();
    combat.put_in_flight(flown_by_c(blaster(), Vec2::new(5.0, 0.0)));
    phase::test_hits(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[0].condition, starting(0), "unset by the passes");
    phase::display_player(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[0].condition, starting(12), "set at its display");
}

#[test]
fn a_final_explosions_shockwave_lands_at_the_wrecks_display_before_a_later_slots_regeneration() {
    // The wreck in slot 1 explodes at its display (@0x2d86d), its
    // shockwave striking at once (@0x2da89), before slot 2's `_HandleShip`
    // regenerates it (@0x34424): 100 less 32, then 1 back.
    let mut wreck = Ship::at(1, 0.0, 0.0);
    wreck.hull.mass = 200.0;
    wreck.hull.death_delay = 5;
    wreck.condition = counting(3);
    let mut near = holding(2, 40.0, 0.0, 100.0, 300.0, 300.0);
    near.shield_regen = 1.0;
    let mut ships = [wreck, near];
    phase::handle_npcs(&mut Combat::default(), &mut ships, &NovaDisable);
    assert_eq!(ships[0].condition, Condition::Destroyed);
    assert_eq!(ships[1].reserves.shield.now, 69.0);
}

#[test]
fn a_shockwave_is_late_on_a_ship_displayed_before_its_wreck_by_slot() {
    // The ship loop displays slot by slot (@0x4445c-0x444c4): a ship in a
    // lower slot than the wreck was shown before the shockwave, so its
    // flash shows the next frame; one in a higher slot shows it that
    // frame. The order of the fight's list says nothing.
    let ships = |wreck_first: bool| {
        let mut wreck = wreck(WRECK, 200.0, 0.0, 0.0);
        wreck.slot = Slot::Berth(5);
        let lower = holding(1, 40.0, 0.0, 100.0, 300.0, 300.0);
        let higher = holding(7, -40.0, 0.0, 100.0, 300.0, 300.0);
        if wreck_first {
            [wreck, lower, higher]
        } else {
            [higher, lower, wreck]
        }
    };
    let seven = ShipRef::Npc(NpcId(7));
    for wreck_first in [true, false] {
        let mut ships = ships(wreck_first);
        let mut combat = Combat::default();
        phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
        let mut late = lateness(&combat.take_events());
        late.sort();
        assert_eq!(late, [(A, true), (seven, false)], "{wreck_first}");
    }
}

#[test]
fn the_players_wreck_strikes_the_npcs_not_yet_shown_and_spares_the_player() {
    // The player's display (@0x43e29) comes before every NPC's (the
    // session's order, `nova_sim::Session::pass_after_move`), so its
    // shockwave meets none shown yet: each one's flash shows that frame.
    let mut player = wreck(ShipRef::Player, 200.0, 0.0, 0.0);
    player.slot = Slot::Player;
    let mut ships = [player, holding(1, 40.0, 0.0, 100.0, 300.0, 300.0)];
    let mut combat = Combat::default();
    phase::display_player(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[0].condition, Condition::Destroyed);
    assert_eq!(lateness(&combat.take_events()), [(A, false)]);
}

#[test]
fn each_display_is_recorded_in_display_order_with_its_disabled_read() {
    // The player's display (@0x43e29), then, in the ship loop, each NPC's
    // by slot (@0x444ad), each with the `_IsDisabled` its layer block
    // reads (@0x2b61f). The session runs the player's before the ship
    // loop (`nova_sim::Session::pass_after_move`) and takes each step's
    // records as it ends (`nova_sim::Session::lay_out_displayed`).
    let player = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        slot: Slot::Player,
        ..Ship::at(0, 0.0, 0.0)
    };
    let mut downed = Ship::at(3, 500.0, 0.0);
    downed.reserves.armor.now = 1.0;
    let mut ships = [Ship::at(4, -500.0, 0.0), downed, player];
    let mut combat = Combat::default();
    phase::display_player(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        combat.take_displays(),
        [Displayed {
            ship: ShipRef::Player,
            slot: Slot::Player,
            disabled: false
        }],
        "the player's"
    );
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        combat.take_displays(),
        [
            Displayed {
                ship: C,
                slot: Slot::Berth(3),
                disabled: true
            },
            Displayed {
                ship: ShipRef::Npc(NpcId(4)),
                slot: Slot::Berth(4),
                disabled: false
            },
        ],
        "the ship loop's, by slot"
    );
    assert_eq!(combat.take_displays(), [], "taken");
}

/// NPC 7 at (-40, 0) regenerating a point of shield, A at (40, 0), and a
/// wreck in slot 5 at the centre going off on its display.
fn by_slot() -> [Ship; 3] {
    let mut wreck = wreck(WRECK, 200.0, 0.0, 0.0);
    wreck.slot = Slot::Berth(5);
    let lower = holding(1, 40.0, 0.0, 100.0, 300.0, 300.0);
    let mut higher = holding(7, -40.0, 0.0, 100.0, 300.0, 300.0);
    higher.shield_regen = 1.0;
    [higher, lower, wreck]
}

/// Each of `ships`' condition, reserves and state.
fn read(ships: &[Ship]) -> Vec<(Condition, Reserves, ShipState)> {
    ships
        .iter()
        .map(|ship| (ship.condition, ship.reserves, ship.state))
        .collect()
}

#[test]
fn stepping_each_npc_by_slot_matches_the_whole_loop() {
    // The session steps each NPC at its own slot, after its flight
    // (`nova_sim::Session::ship_loop`), keeping the ships shown: the same
    // fight as the loop over every NPC did before it was split NPC by NPC
    // (rdm weapons-fidelity phase 46, recorded from the parent of
    // 5db4172e). By slot: A (1) regenerates and is shown; the wreck (5)
    // goes off, striking A late and 7 not yet shown; 7 then regenerates
    // its point back.
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = arena(&arsenal, &govts);
    let ships = by_slot;
    let left = |ships: &[Ship]| {
        ships
            .iter()
            .map(|ship| {
                (
                    ship.id,
                    ship.condition,
                    ship.reserves.shield.now,
                    ship.reserves.armor.now,
                )
            })
            .collect::<Vec<_>>()
    };
    let seven = ShipRef::Npc(NpcId(7));
    let origin = Vec2::new(0.0, 0.0);
    let hit = |ship, late| CombatEvent::Hit {
        ship,
        weapon: None,
        at: origin,
        passes_shields: false,
        late,
    };
    let events = [
        CombatEvent::BreakingUp {
            ship: WRECK,
            at: origin,
            explosion: None,
        },
        CombatEvent::Destroyed {
            ship: WRECK,
            ship_type: ShipId(128),
            at: origin,
            velocity: origin,
            explosion: None,
            size: 65.0,
        },
        hit(seven, false),
        hit(A, true),
    ];
    let displays = [
        Displayed {
            ship: A,
            slot: Slot::Berth(1),
            disabled: false,
        },
        Displayed {
            ship: WRECK,
            slot: Slot::Berth(5),
            disabled: true,
        },
        Displayed {
            ship: seven,
            slot: Slot::Berth(7),
            disabled: false,
        },
    ];
    let mut whole = ships();
    let mut loop_combat = Combat::default();
    let mut fighters: Vec<Fighter> = whole.iter_mut().map(Ship::fighter).collect();
    loop_combat.handle_npcs(&mut fighters, arena, &mut NeverFires);
    drop(fighters);
    let mut each = ships();
    let mut each_combat = Combat::default();
    let mut shown = vec![ShipRef::Player];
    for ship in [A, WRECK, seven] {
        let mut fighters: Vec<Fighter> = each.iter_mut().map(Ship::fighter).collect();
        each_combat.handle_npc(&mut fighters, arena, ship, &mut shown, &mut NeverFires);
    }
    assert_eq!(read(&each), read(&whole));
    assert_eq!(each_combat, loop_combat);
    assert_eq!(shown, [ShipRef::Player, A, WRECK, seven]);
    for (ships, combat) in [(&each, &mut each_combat), (&whole, &mut loop_combat)] {
        assert_eq!(
            left(ships),
            [
                (seven, Condition::Intact, 69.0, 300.0),
                (A, Condition::Intact, 68.0, 300.0),
                (WRECK, Condition::Destroyed, 10.0, 0.0),
            ]
        );
        assert_eq!(combat.take_events(), events);
        assert_eq!(combat.take_displays(), displays);
    }
}

#[test]
fn an_npcs_step_then_its_display_match_handling_it() {
    // The ship loop runs an NPC's step and its display apart when its
    // pods may come out between the two (`_HandleShip`'s dying block
    // @0x35080-0x3557b, before `_HandleShipDisplay` @0x444ad).
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = arena(&arsenal, &govts);
    let order = [A, WRECK, ShipRef::Npc(NpcId(7))];
    let mut whole = by_slot();
    let mut whole_combat = Combat::default();
    let mut whole_shown = vec![ShipRef::Player];
    for ship in order {
        let mut fighters: Vec<Fighter> = whole.iter_mut().map(Ship::fighter).collect();
        whole_combat.handle_npc(
            &mut fighters,
            arena,
            ship,
            &mut whole_shown,
            &mut NeverFires,
        );
    }
    let mut split = by_slot();
    let mut split_combat = Combat::default();
    let mut shown = vec![ShipRef::Player];
    for ship in order {
        let mut fighters: Vec<Fighter> = split.iter_mut().map(Ship::fighter).collect();
        split_combat.step_npc(&mut fighters, arena, ship, &mut NeverFires);
        drop(fighters);
        let mut fighters: Vec<Fighter> = split.iter_mut().map(Ship::fighter).collect();
        split_combat.display_npc(&mut fighters, arena, ship, &mut shown);
    }
    assert_eq!(read(&split), read(&whole));
    assert_eq!(split_combat, whole_combat);
    assert_eq!(shown, whole_shown);
}
