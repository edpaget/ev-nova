//! The fight's tests of a plundering pirate's fire (rdm combat-ai-fidelity
//! phase 9, see [`plunder`]): its shots leave disable-only, its beams at
//! the intact ship it plunders, and its and its escorts' real hits on
//! their targets can't kill.

use super::*;

/// A shell that would destroy a ship of 10 shield and 30 armour: 50 mass
/// and 50 energy, 15 pixels a tick for 13 ticks.
fn shell() -> WeaponRecord {
    WeaponRecord {
        mass_dmg: 50,
        energy_dmg: 50,
        ..blaster()
    }
}

/// NPC 1 at the centre facing right, firing `record` at `target`,
/// plundering as `plundering` says.
fn pirate(record: WeaponRecord, target: ShipRef, plundering: bool) -> Ship {
    Ship {
        plundering,
        ..Ship::at(1, 0.0, 0.0).armed(record).targeting(target)
    }
}

/// `ships` fought for `ticks` ticks, ship 0 firing on its first tick
/// alone, `between` run on them after that first tick: the events.
fn fought(ships: &mut [Ship], ticks: u32, between: impl Fn(&mut [Ship])) -> Vec<CombatEvent> {
    let mut combat = Combat::default();
    let mut events = Vec::new();
    tick(&mut combat, ships, &NovaDisable);
    events.extend(combat.take_events());
    ships[0].trigger = QUIET;
    between(ships);
    for _ in 1..ticks {
        tick(&mut combat, ships, &NovaDisable);
        events.extend(combat.take_events());
    }
    events
}

/// Whether `ship` was spared: left at 1 armour, disabled, never breaking
/// up, among `events`.
fn spared(ship: &Ship, events: &[CombatEvent]) -> bool {
    ship.reserves.armor.now == damage::SPARED_ARMOR
        && ship.condition == Condition::Disabled
        && !broke_up(events, &[ship.id])
}

#[test]
fn a_plunderers_shot_leaves_its_intact_victim_at_1_armour_and_an_attackers_destroys_it() {
    let mut ships = [pirate(shell(), B, true), Ship::at(2, 100.0, 0.0)];
    let events = fought(&mut ships, 20, |_| {});
    assert!(spared(&ships[1], &events), "{:?}", ships[1].reserves);
    let mut ships = [pirate(shell(), B, false), Ship::at(2, 100.0, 0.0)];
    let events = fought(&mut ships, 20, |_| {});
    assert!(broke_up(&events, &[B]), "attacking, it kills");
}

#[test]
fn the_shots_flag_is_set_as_it_leaves_and_holds_when_the_pirate_turns_to_attack() {
    let mut combat = Combat::default();
    let mut ships = [pirate(shell(), B, true), Ship::at(2, 100.0, 0.0)];
    tick(&mut combat, &mut ships, &NovaDisable);
    assert!(combat.shots().iter().all(|shot| shot.disable_only));
    assert_eq!(combat.shots().len(), 1);
    let mut ships = [pirate(shell(), B, true), Ship::at(2, 100.0, 0.0)];
    let events = fought(&mut ships, 20, |ships| ships[0].plundering = false);
    assert!(spared(&ships[1], &events), "the launch flag");
}

#[test]
fn a_shot_fired_attacking_that_lands_on_the_ship_the_pirate_turned_to_plunder_cannot_kill() {
    let mut ships = [pirate(shell(), B, false), Ship::at(2, 100.0, 0.0)];
    let events = fought(&mut ships, 20, |ships| ships[0].plundering = true);
    assert!(spared(&ships[1], &events), "the hit's own test");
    let mut ships = [pirate(shell(), B, false), Ship::at(2, 100.0, 0.0)];
    let events = fought(&mut ships, 20, |ships| {
        ships[0].plundering = true;
        ships[0].target = Some(C);
    });
    assert!(broke_up(&events, &[B]), "plundering another ship");
}

#[test]
fn every_shot_of_a_plunderer_spares_whatever_it_hits_and_its_blast_too() {
    // Its target is C, far off; the shell meets B in its path.
    let mut ships = [
        pirate(shell(), C, true),
        Ship::at(2, 100.0, 0.0),
        Ship::at(3, 0.0, 900.0),
    ];
    let events = fought(&mut ships, 20, |_| {});
    assert!(
        spared(&ships[1], &events),
        "goal 0xd's own branch (@0x3e9fb)"
    );
    let bomb = WeaponRecord {
        blast_radius: 30,
        ..shell()
    };
    let mut ships = [
        pirate(bomb, B, true),
        Ship::at(2, 100.0, 0.0),
        Ship::at(3, 100.0, 25.0),
    ];
    let events = fought(&mut ships, 20, |_| {});
    assert!(spared(&ships[1], &events));
    assert!(spared(&ships[2], &events), "the blast's bystander");
}

#[test]
fn the_players_fire_never_spares_by_the_plunder() {
    let player = Ship {
        id: ShipRef::Player,
        fleet: ShipRef::Player,
        ..pirate(shell(), B, true)
    };
    let mut ships = [player, Ship::at(2, 100.0, 0.0)];
    let events = fought(&mut ships, 20, |_| {});
    assert!(broke_up(&events, &[B]));
}

#[test]
fn an_escort_of_a_plunderer_cannot_kill_its_own_target_and_one_the_player_leads_can() {
    let escorted = |leader: ShipRef, leader_plunders: bool, target: ShipRef| {
        let escort = Ship {
            leader: Some(leader),
            fleet: leader,
            ..Ship::at(3, 0.0, 0.0).armed(shell()).targeting(target)
        };
        let mut lead = Ship::at(1, 0.0, 600.0).targeting(C);
        lead.plundering = leader_plunders;
        let mut ships = [escort, Ship::at(2, 100.0, 0.0), lead];
        let events = fought(&mut ships, 20, |_| {});
        (spared(&ships[1], &events), broke_up(&events, &[B]))
    };
    assert_eq!(escorted(A, true, B), (true, false), "its leader plunders");
    assert_eq!(escorted(A, false, B), (false, true), "its leader attacks");
    assert_eq!(escorted(A, true, C), (false, true), "B is not its target");
    assert_eq!(
        escorted(ShipRef::Player, true, B),
        (false, true),
        "led by the player"
    );
}

/// A beam firing ahead 200 long, of 50 mass and 50 energy a tick.
fn beam() -> WeaponRecord {
    WeaponRecord {
        mass_dmg: 50,
        energy_dmg: 50,
        ..laser()
    }
}

#[test]
fn a_plunderers_beam_is_disable_only_at_the_intact_ship_it_plunders_alone() {
    let mut combat = Combat::default();
    let mut ships = [pirate(beam(), B, true), Ship::at(2, 100.0, 0.0)];
    tick(&mut combat, &mut ships, &NovaDisable);
    assert!(combat.beams().iter().all(|beam| beam.disable_only));
    assert!(!combat.beams().is_empty());
    let mut ships = [pirate(beam(), B, true), Ship::at(2, 100.0, 0.0)];
    let events = fought(&mut ships, 10, |_| {});
    assert!(spared(&ships[1], &events));
    // Its target far off and disabled: the beam carries no flag, and B,
    // which it meets in its path, is no target of the pirate's.
    let far_wreck = || Ship {
        condition: Condition::Disabled,
        ..Ship::at(3, 0.0, 900.0)
    };
    let mut combat = Combat::default();
    let mut ships = [
        pirate(beam(), C, true),
        Ship::at(2, 100.0, 0.0),
        far_wreck(),
    ];
    tick(&mut combat, &mut ships, &NovaDisable);
    assert!(combat.beams().iter().all(|beam| !beam.disable_only));
    assert!(!combat.beams().is_empty());
    let mut ships = [
        pirate(beam(), C, true),
        Ship::at(2, 100.0, 0.0),
        far_wreck(),
    ];
    let events = fought(&mut ships, 10, |_| {});
    assert!(broke_up(&events, &[B]), "no goal-0xd branch for a beam");
}

#[test]
fn a_beam_at_a_victim_disabled_as_it_fires_carries_no_flag_but_its_hit_still_spares_it() {
    let downed = || Ship {
        condition: Condition::Disabled,
        reserves: Reserves::full(0.0, 30.0, 100.0),
        ..Ship::at(2, 100.0, 0.0)
    };
    let mut combat = Combat::default();
    let mut ships = [pirate(beam(), B, true), downed()];
    tick(&mut combat, &mut ships, &NovaDisable);
    assert!(combat.beams().iter().all(|beam| !beam.disable_only));
    assert!(!combat.beams().is_empty());
    let mut ships = [pirate(beam(), B, true), downed()];
    let events = fought(&mut ships, 10, |_| {});
    assert_eq!(ships[1].reserves.armor.now, damage::SPARED_ARMOR);
    assert!(!broke_up(&events, &[B]));
    let mut ships = [pirate(beam(), B, false), downed()];
    let events = fought(&mut ships, 10, |_| {});
    assert!(broke_up(&events, &[B]), "attacking, it finishes it off");
}

#[test]
fn a_sub_munition_reads_its_owners_plunder_as_it_is_released_not_its_parents_flag() {
    let (shell, arsenal) = cluster(0);
    let released = |plundering_at_launch: bool, plundering_after: bool| {
        let mut combat = Combat::default();
        let mut ships = [
            pirate(shell, C, plundering_at_launch),
            Ship::at(3, 500.0, 0.0),
        ];
        tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        ships[0].trigger = QUIET;
        ships[0].plundering = plundering_after;
        for _ in 1..4 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        }
        let subs = sub_shots(&combat);
        assert_eq!(subs.len(), 2);
        subs.iter()
            .map(|shot| shot.disable_only)
            .collect::<Vec<_>>()
    };
    assert_eq!(released(true, true), [true, true]);
    assert_eq!(
        released(true, false),
        [false, false],
        "not the parent's flag"
    );
    assert_eq!(released(false, true), [true, true]);
    assert_eq!(released(false, false), [false, false]);
}

#[test]
fn a_plunderers_point_defence_shot_leaves_disable_only() {
    let defended = |plundering: bool| {
        let mut combat = Combat::default();
        let mut ships = missile_attack(quad());
        ships[0].plundering = plundering;
        missile_in_flight(&mut combat, &mut ships);
        tick(&mut combat, &mut ships, &NovaDisable);
        let defence: Vec<bool> = combat
            .shots()
            .iter()
            .filter(|shot| shot.weapon.guidance == Guidance::PointDefence)
            .map(|shot| shot.disable_only)
            .collect();
        assert_eq!(defence.len(), 1, "{:?}", combat.shots());
        defence[0]
    };
    assert!(defended(true), "`_SpawnShot(.., -1, ..)` in goal 0xd");
    assert!(!defended(false));
}
