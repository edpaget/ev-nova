//! The fight's tests of `_AIDispatch`'s slot loop (rdm weapons-fidelity
//! phase 36): each NPC's goal-4 launch (@0x8e6fa), point defence
//! (`_AIFireTurret` @0x80b0d) and the snipe's launch (@0x8732f), slot by
//! slot (@0x44270-0x442ae), before `_HandleShot` and every pick. That
//! every launch comes before every pick is the session's order
//! (`nova_sim::Session::pass_after_move`), which its fighters' tests pin.

use super::*;

/// A ship launching from the carrier's bay ([`carrier_bay`]) as `trigger`
/// says, holding 2 fighters, point defence of the Quad Light Blaster
/// Turret too. It targets ship A, itself when it is A, as a launching
/// ship has a target in the fight: with none, `_HandleShip` holds its bay
/// at its full `Reload` (@0x3390a-0x3394e).
fn carrying(id: u32, x: f32, trigger: Trigger) -> Ship {
    let mut ship = Ship::at(id, x, 0.0).facing(0.0).targeting(A);
    ship.armament = Armament::new([
        (WeaponSpec::new(&carrier_bay()), 1),
        (WeaponSpec::new(&quad()), 1),
    ]);
    ship.rounds = LauncherRounds::from([(WeaponId(149), 2)]);
    ship.trigger = trigger;
    ship
}

/// A defender `id` at (`x`, 0) with the Quad Light Blaster Turret, its
/// point defence asked for.
fn defender(id: u32, x: f32) -> Ship {
    let mut ship = Ship::at(id, x, 0.0).armed(quad()).facing(0.0);
    ship.trigger = QUIET;
    ship
}

/// Ship `id` 200 pixels above the centre, firing a missile at A on its
/// first tick, on the IR Missile's 18-pixel sprite.
fn missile_firer(id: u32) -> Ship {
    Ship::at(id, 0.0, -200.0)
        .armed(drawn(
            WeaponRecord {
                speed: 500,
                ..missile()
            },
            18,
        ))
        .facing(180.0)
        .targeting(A)
}

/// Runs the ship loop once with every trigger but the missile firer's
/// held back, so its missile is in flight; then gives each its own again,
/// the firer's none.
fn missile_flying(combat: &mut Combat, ships: &mut [Ship], firer: ShipRef) {
    let held: Vec<Trigger> = ships.iter().map(|ship| ship.trigger).collect();
    for ship in ships.iter_mut().filter(|ship| ship.id != firer) {
        ship.trigger = Trigger::default();
    }
    phase::handle_npcs(combat, ships, &NovaDisable);
    for (ship, trigger) in ships.iter_mut().zip(held) {
        ship.trigger = if ship.id == firer {
            Trigger::default()
        } else {
            trigger
        };
    }
    combat.take_events();
}

/// The `Fired` events among `events`: each firer, weapon and priority.
fn fired(events: &[CombatEvent]) -> Vec<(ShipRef, WeaponId, Option<u8>)> {
    events
        .iter()
        .filter_map(|event| match *event {
            CombatEvent::Fired {
                ship,
                weapon,
                sound,
                ..
            } => Some((ship, weapon, sound.map(|sound| sound.priority))),
            _ => None,
        })
        .collect()
}

const BAY: WeaponId = WeaponId(149);
const QUAD: WeaponId = WeaponId(133);
const D: ShipRef = ShipRef::Npc(NpcId(4));

#[test]
fn an_npcs_launch_comes_after_a_lower_slots_point_defence_and_before_a_higher_slots() {
    // `_AIDispatch` runs the slots in turn (@0x44270-0x442ae): slot 1's
    // `_AIFireTurret`, slot 2's `_AILaunchFighter` (@0x8e6fa), then slot
    // 3's `_AIFireTurret`, whatever order the fight holds them in.
    let launching = Trigger {
        bays: true,
        ..Trigger::default()
    };
    // C flies in A's fleet, so defends against a missile aimed at A.
    let mut wing = defender(3, -30.0);
    wing.fleet = A;
    let mut ships = [
        carrying(2, 30.0, launching),
        wing,
        missile_firer(4),
        defender(1, 0.0),
    ];
    let mut combat = Combat::default();
    missile_flying(&mut combat, &mut ships, D);
    phase::defend_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        fired(&combat.take_events()),
        [(A, QUAD, Some(3)), (B, BAY, Some(5)), (C, QUAD, Some(3))]
    );
}

#[test]
fn a_goal_four_launch_comes_before_its_own_point_defence_and_the_snipes_after_it() {
    // The high level's launch (@0x8e6fa) before the low level's
    // `_AIFireTurret` (@0x87324), and the snipe's own after it (@0x8732f).
    let heard = |trigger: Trigger| {
        let mut ships = [
            carrying(
                1,
                0.0,
                Trigger {
                    point_defence: true,
                    ..trigger
                },
            ),
            missile_firer(2),
        ];
        let mut combat = Combat::default();
        missile_flying(&mut combat, &mut ships, B);
        phase::defend_npcs(&mut combat, &mut ships, &NovaDisable);
        fired(&combat.take_events())
    };
    let high = heard(Trigger {
        bays: true,
        ..Trigger::default()
    });
    assert_eq!(high, [(A, BAY, Some(5)), (A, QUAD, Some(3))]);
    let snipe = heard(Trigger {
        bays_after: true,
        ..Trigger::default()
    });
    assert_eq!(snipe, [(A, QUAD, Some(3)), (A, BAY, Some(5))]);
}

#[test]
fn the_npcs_launch_in_their_point_defence_phase_and_never_with_their_picks() {
    let launching = Trigger {
        bays: true,
        bays_after: true,
        ..Trigger::default()
    };
    let mut ships = [carrying(1, 0.0, launching)];
    let mut combat = Combat::default();
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let on = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        combat.handle_npcs(&mut fighters, on, &mut NeverFires);
    }
    assert_eq!(combat.take_sorties(), [], "no launch with the picks");
    assert_eq!(fired(&combat.take_events()), []);
    assert_eq!(ships[0].rounds.slot(BAY), 2);
    {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        combat.defend_npcs(&mut fighters, on, None, None, &mut NeverFires);
    }
    assert_eq!(combat.take_sorties().len(), 1, "one: the bay then reloads");
    assert_eq!(fired(&combat.take_events()), [(A, BAY, Some(5))]);
    assert_eq!(ships[0].rounds.slot(BAY), 1);
}

/// The NPCs' whole step-4 loop, on `arena`'s pass.
fn defend(combat: &mut Combat, fighters: &mut [Fighter], arena: Arena, chance: &mut dyn Chance) {
    combat.defend_npcs(fighters, arena, None, None, chance);
}

#[test]
fn on_double_times_second_pass_no_npc_launches() {
    // `_AIDispatch` runs on the advance pass alone (@0x44026-0x4402a).
    let launching = Trigger {
        bays: true,
        ..Trigger::default()
    };
    let mut ships = [carrying(1, 0.0, launching)];
    let mut combat = Combat::default();
    phase::run_on(&mut combat, &mut ships, GamePass::Again, defend);
    assert_eq!(combat.take_sorties(), []);
    assert_eq!(ships[0].rounds.slot(BAY), 2);
    let pass = GamePass::Advance { double_time: true };
    phase::run_on(&mut combat, &mut ships, pass, defend);
    assert_eq!(combat.take_sorties().len(), 1);
}

#[test]
fn a_reload_zero_bay_launches_on_both_calls_of_one_tick() {
    // `_AILaunchFighter` sets the reload to `Reload` / count (@0x81473-0x81491),
    // so a bay of `Reload` 0 is ready again for the snipe's own call
    // (@0x8732f) after the high level's (@0x8e6fa). The fight already
    // launches on both calls, from the one armament (rdm weapons-fidelity
    // phase 41 pins it): this test was green when written.
    let launching = Trigger {
        bays: true,
        bays_after: true,
        ..Trigger::default()
    };
    let mut zero = carrying(1, 0.0, launching);
    zero.armament = Armament::new([(
        WeaponSpec::new(&WeaponRecord {
            reload: 0,
            ..carrier_bay()
        }),
        1,
    )]);
    let mut ships = [zero];
    let mut combat = Combat::default();
    phase::defend_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        fired(&combat.take_events()),
        [(A, BAY, Some(5)), (A, BAY, Some(5))]
    );
    assert_eq!(combat.take_sorties().len(), 2);
    assert_eq!(ships[0].rounds.slot(BAY), 0, "two rounds spent");
    let mut ships = [carrying(1, 0.0, launching)];
    let mut combat = Combat::default();
    phase::defend_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.take_sorties().len(), 1, "the stock bay then reloads");
    assert_eq!(ships[0].rounds.slot(BAY), 1);
}

// A loop run in bounded runs (rdm combat-ai-fidelity-cross-roadmap phase
// 6): `_LaunchFighter` marks the fighter's slot active at once (@0x3c90b),
// and the loop rereads each slot as it comes (@0x44270-0x442ae), so the
// session runs each slot's turn apart, with what it left in the system
// before the next.

/// One run of the NPCs' step-4 loop among `ships` on the advance pass: the
/// slots above `after`, up to `through`.
fn dispatch(combat: &mut Combat, ships: &mut [Ship], after: Option<Slot>, through: Option<Slot>) {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.defend_npcs(&mut fighters, arena, after, through, &mut NeverFires);
}

/// A's defender in slot 1, B's carrier launching in slot 2, C's wing of
/// A's fleet defending in slot 3, D's missile at A in flight.
fn launch_between_defenders(combat: &mut Combat) -> [Ship; 4] {
    let launching = Trigger {
        bays: true,
        ..Trigger::default()
    };
    let mut wing = defender(3, -30.0);
    wing.fleet = A;
    let mut ships = [
        defender(1, 0.0),
        carrying(2, 30.0, launching),
        wing,
        missile_firer(4),
    ];
    missile_flying(combat, &mut ships, D);
    ships
}

#[test]
fn a_run_takes_only_the_slots_up_to_its_last() {
    let mut combat = Combat::default();
    let mut ships = launch_between_defenders(&mut combat);
    dispatch(&mut combat, &mut ships, None, Some(Slot::Berth(2)));
    assert_eq!(
        fired(&combat.take_events()),
        [(A, QUAD, Some(3)), (B, BAY, Some(5))],
        "no higher slot's turn"
    );
    assert_eq!(combat.take_sorties().len(), 1);
}

#[test]
fn a_later_run_takes_only_the_slots_after_the_last_one() {
    let mut combat = Combat::default();
    let mut ships = launch_between_defenders(&mut combat);
    // A bay of `Reload` 0, ready again at once: a second turn would
    // launch again.
    ships[1].armament = Armament::new([(
        WeaponSpec::new(&WeaponRecord {
            reload: 0,
            ..carrier_bay()
        }),
        1,
    )]);
    dispatch(&mut combat, &mut ships, None, Some(Slot::Berth(2)));
    combat.take_events();
    combat.take_sorties();
    dispatch(&mut combat, &mut ships, Some(Slot::Berth(2)), None);
    assert_eq!(
        fired(&combat.take_events()),
        [(C, QUAD, Some(3))],
        "neither A's point defence again nor B's launch"
    );
    assert_eq!(combat.take_sorties(), []);
}

#[test]
fn a_loop_that_does_not_stop_runs_every_slot_once() {
    let mut combat = Combat::default();
    let mut ships = launch_between_defenders(&mut combat);
    dispatch(&mut combat, &mut ships, None, None);
    assert_eq!(
        fired(&combat.take_events()),
        [(A, QUAD, Some(3)), (B, BAY, Some(5)), (C, QUAD, Some(3))]
    );
}

#[test]
fn a_later_slots_point_defence_can_pick_a_fighter_placed_since() {
    // `_HandleShipPointDefense`'s ship branch walks the live table
    // (@0x394ce-0x3969d): the fighter in slot 5, put in after slot 2's
    // launch, attacks slot 3's defender, whose turn comes after.
    let mut combat = Combat::default();
    let mut ships = [
        carrying(2, 30.0, Trigger::default()),
        defender(3, 0.0),
        Ship::at(5, 0.0, 150.0).attacking(C),
    ];
    dispatch(&mut combat, &mut ships, Some(Slot::Berth(2)), None);
    assert_eq!(fired(&combat.take_events()), [(C, QUAD, Some(3))]);
    let mut combat = Combat::default();
    let mut alone = [carrying(2, 30.0, Trigger::default()), defender(3, 0.0)];
    dispatch(&mut combat, &mut alone, Some(Slot::Berth(2)), None);
    assert_eq!(fired(&combat.take_events()), [], "nothing else to fire at");
}

#[test]
fn on_double_times_second_pass_a_run_does_nothing() {
    let launching = Trigger {
        bays: true,
        ..Trigger::default()
    };
    let mut ships = [carrying(1, 0.0, launching)];
    let mut combat = Combat::default();
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass: GamePass::Again,
        arsenal: &arsenal,
        govts: &govts,
        rules: Rules::default(),
        field: Field::default(),
    };
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.defend_npcs(&mut fighters, arena, None, None, &mut NeverFires);
    assert_eq!(combat.take_sorties(), []);
}

#[test]
fn a_run_in_which_no_npc_acts_notes_what_the_loop_leaves_of_it() {
    // A run of ships whose triggers ask nothing of the loop writes only the
    // field and each ship's slot record (`Combat::defend_npcs`), so the
    // session can skip the fight's assembly and note them from its ships.
    let idle = |ship: Ship| Ship {
        trigger: Trigger::default(),
        ..ship
    };
    let mut ships = [
        idle(Ship::at(1, 0.0, 0.0).targeting(B)),
        Ship {
            plundering: true,
            docked: true,
            ..idle(Ship::at(2, 50.0, 20.0).targeting(A))
        },
    ];
    let field = Field {
        frame: 7,
        interference: 3,
        ..Field::default()
    };
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    for pass in [GamePass::Advance { double_time: false }, GamePass::Again] {
        let arena = Arena {
            pass,
            arsenal: &arsenal,
            govts: &govts,
            rules: Rules::default(),
            field,
        };
        let mut looped = Combat::default();
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        looped.defend_npcs(&mut fighters, arena, None, None, &mut NeverFires);
        let records: Vec<(Slot, SlotShip)> = fighters
            .iter()
            .map(|fighter| {
                let ship = SlotShip {
                    class: fighter.ship_type,
                    plunderer: plunder::Plunderer::of(fighter),
                    position: fighter.state.position,
                };
                (fighter.slot, ship)
            })
            .collect();
        let mut noted = Combat::default();
        noted.note_idle_run(pass, field, records);
        assert_eq!(noted, looped, "{pass:?}");
        let wrote = pass != GamePass::Again;
        assert_eq!(looped != Combat::default(), wrote, "{pass:?}");
    }
}
