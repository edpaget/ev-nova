//! The fight's tests of `_AIDispatch`'s slot loop (rdm weapons-fidelity
//! phase 36): each NPC's goal-4 launch (@0x8e6fa), point defence
//! (`_AIFireTurret` @0x80b0d) and the snipe's launch (@0x8732f), slot by
//! slot (@0x44270-0x442ae), before `_HandleShot` and every pick.

use super::*;

/// A ship launching from the carrier's bay ([`carrier_bay`]) as `trigger`
/// says, holding 2 fighters, point defence of the Quad Light Blaster
/// Turret too.
fn carrying(id: u32, x: f32, trigger: Trigger) -> Ship {
    let mut ship = Ship::at(id, x, 0.0).facing(0.0);
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

/// Ticks `ships` once with every trigger but the missile firer's held
/// back, so its missile is in flight; then gives each its own again, the
/// firer's none.
fn missile_flying(combat: &mut Combat, ships: &mut [Ship], firer: ShipRef) {
    let held: Vec<Trigger> = ships.iter().map(|ship| ship.trigger).collect();
    for ship in ships.iter_mut().filter(|ship| ship.id != firer) {
        ship.trigger = Trigger::default();
    }
    tick(combat, ships, &NovaDisable);
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
    tick(&mut combat, &mut ships, &NovaDisable);
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
        tick(&mut combat, &mut ships, &NovaDisable);
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
        combat.defend_npcs(&mut fighters, on, &mut NeverFires);
    }
    assert_eq!(combat.take_sorties().len(), 1, "one: the bay then reloads");
    assert_eq!(fired(&combat.take_events()), [(A, BAY, Some(5))]);
    assert_eq!(ships[0].rounds.slot(BAY), 1);
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
    tick_on(&mut combat, &mut ships, GamePass::Again);
    assert_eq!(combat.take_sorties(), []);
    assert_eq!(ships[0].rounds.slot(BAY), 2);
    tick_on(
        &mut combat,
        &mut ships,
        GamePass::Advance { double_time: true },
    );
    assert_eq!(combat.take_sorties().len(), 1);
}

#[test]
fn within_a_tick_every_launch_is_heard_before_every_pick() {
    // Every `_AILaunchFighter` comes in `_AIDispatch`, before
    // `_HandleShot` (@0x44385) and `_HandleShip`'s picks (@0x44483),
    // whatever the slots.
    let launching = Trigger {
        bays: true,
        ..Trigger::default()
    };
    let mut ships = [
        carrying(2, 0.0, launching),
        Ship::at(1, 0.0, 100.0).armed(blaster()).picking(128),
    ];
    let mut combat = Combat::default();
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        fired(&combat.take_events()),
        [(B, BAY, Some(5)), (A, WeaponId(128), Some(4))]
    );
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
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        fired(&combat.take_events()),
        [(A, BAY, Some(5)), (A, BAY, Some(5))]
    );
    assert_eq!(combat.take_sorties().len(), 2);
    assert_eq!(ships[0].rounds.slot(BAY), 0, "two rounds spent");
    let mut ships = [carrying(1, 0.0, launching)];
    let mut combat = Combat::default();
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(combat.take_sorties().len(), 1, "the stock bay then reloads");
    assert_eq!(ships[0].rounds.slot(BAY), 1);
}
