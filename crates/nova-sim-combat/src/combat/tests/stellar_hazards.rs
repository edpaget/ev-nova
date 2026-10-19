//! The fight's tests of the stellar hazards inside its tick
//! ([`stellar_hazards`](crate::combat::stellar_hazards)): gravity's pull and
//! the deadly stellars' kills land after the shots and before the beams,
//! as `_DoPlayGameWork` calls `_HandleGravity` (@0x44509) and
//! `_HandleDeadlyStellars` (@0x4450e) before it jumps to `_HandleBeams`
//! (@0x4453d).

use super::*;
use crate::combat::stellar_hazards::{
    DeadlyDisc, GravitySource, HazardResistance, StellarHazards, StellarKill,
};

/// A tick among `ships` on `pass` with `hazards`, by the default rules,
/// each shove set on its ship as the session sets it: every shove made.
fn tick_in(
    combat: &mut Combat,
    ships: &mut [Ship],
    hazards: &StellarHazards,
    pass: GamePass,
) -> Vec<Shove> {
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    combat.tick_on(
        pass,
        &mut fighters,
        &Arsenal::default(),
        &Governments::default(),
        Rules::default(),
        Field::default(),
        Scenery {
            hazards: Some(hazards),
            ..Scenery::default()
        },
        &mut NeverFires,
    );
    drop(fighters);
    let shoves = combat.take_shoves();
    for shove in &shoves {
        if let Some(ship) = ships.iter_mut().find(|ship| ship.id == shove.ship) {
            ship.state.velocity = shove.velocity;
        }
    }
    shoves
}

const ADVANCE: GamePass = GamePass::Advance { double_time: false };

/// Gravity alone: `gravity` at `at`.
fn pulling(at: Vec2, gravity: i16) -> StellarHazards {
    StellarHazards {
        gravity: vec![GravitySource { at, gravity }],
        deadly: Vec::new(),
    }
}

/// Deadly discs alone, each at its place of `radius`.
fn deadly(discs: &[(Vec2, f32)]) -> StellarHazards {
    StellarHazards {
        gravity: Vec::new(),
        deadly: discs
            .iter()
            .map(|&(at, radius)| DeadlyDisc { at, radius })
            .collect(),
    }
}

/// An explosion to go off as a ship dies.
const BOOM: Explosion = Explosion {
    boom: crate::catalog::BoomId(130),
    extra: false,
};

/// NPC `id` at (`x`, `y`) with an `Explode2`.
fn exploding(id: u32, x: f32, y: f32) -> Ship {
    let mut ship = Ship::at(id, x, y);
    ship.hull.explosion = Some(BOOM);
    ship
}

fn exploded(events: &[CombatEvent]) -> Vec<CombatEvent> {
    events
        .iter()
        .filter(|event| matches!(event, CombatEvent::Exploded { .. }))
        .copied()
        .collect()
}

#[test]
fn the_pull_lands_on_the_velocity_as_a_shove() {
    let mut combat = Combat::default();
    let mut ships = [Ship::at(1, 0.0, 0.0), Ship::at(2, 0.0, 300.0)];
    let stellar = Vec2::new(2000.0, 0.0);
    let shoves = tick_in(&mut combat, &mut ships, &pulling(stellar, 40), ADVANCE);
    let a = crate::gravity::pull(Vec2::ZERO, stellar, 40, 1.0);
    let b = crate::gravity::pull(Vec2::new(0.0, 300.0), stellar, 40, 1.0);
    assert!(a.x > 0.09, "{a:?}");
    assert_eq!(
        shoves,
        [
            Shove {
                ship: A,
                velocity: a
            },
            Shove {
                ship: B,
                velocity: b
            },
        ]
    );
    assert_eq!(ships[0].state.velocity, a);
}

#[test]
fn each_source_pulls_in_turn_adding_to_the_velocity() {
    let mut combat = Combat::default();
    let mut ships = [Ship::at(1, 0.0, 0.0).moving(Vec2::new(1.0, 2.0))];
    let east = GravitySource {
        at: Vec2::new(2000.0, 0.0),
        gravity: 40,
    };
    let west = GravitySource {
        at: Vec2::new(-1000.0, 0.0),
        gravity: 30,
    };
    let hazards = StellarHazards {
        gravity: vec![east, west],
        deadly: Vec::new(),
    };
    let shoves = tick_in(&mut combat, &mut ships, &hazards, ADVANCE);
    let once = Vec2::new(1.0, 2.0) + crate::gravity::pull(Vec2::ZERO, east.at, 40, 1.0);
    let twice = once + crate::gravity::pull(Vec2::ZERO, west.at, 30, 1.0);
    assert_eq!(shoves_on(&shoves, A), [once, twice]);
}

#[test]
fn a_resisting_fighter_is_neither_pulled_nor_killed() {
    let mut combat = Combat::default();
    let mut resisting = exploding(1, 0.0, 0.0);
    resisting.resists = HazardResistance {
        gravity: true,
        deadly: true,
    };
    let mut ships = [resisting];
    let hazards = StellarHazards {
        gravity: vec![GravitySource {
            at: Vec2::new(500.0, 0.0),
            gravity: 300,
        }],
        deadly: vec![DeadlyDisc {
            at: Vec2::ZERO,
            radius: 50.0,
        }],
    };
    let shoves = tick_in(&mut combat, &mut ships, &hazards, ADVANCE);
    assert_eq!(shoves, []);
    assert_eq!(ships[0].condition, Condition::Intact);
    assert_eq!(combat.take_stellar_kills(), []);
    // Each resistance is its own.
    let mut pulled_only = exploding(1, 0.0, 0.0);
    pulled_only.resists.deadly = true;
    let mut killed_only = exploding(2, 0.0, 0.0);
    killed_only.resists.gravity = true;
    let mut ships = [pulled_only, killed_only];
    let shoves = tick_in(&mut combat, &mut ships, &hazards, ADVANCE);
    assert_eq!(shoves_on(&shoves, A).len(), 1, "{shoves:?}");
    assert_eq!(shoves_on(&shoves, B), []);
    assert_eq!(
        (ships[0].condition, ships[1].condition),
        (Condition::Intact, Condition::Destroyed)
    );
}

#[test]
fn a_destroyed_fighter_is_not_pulled_and_a_dying_one_is_pulled_but_not_killed() {
    let mut combat = Combat::default();
    let mut destroyed = exploding(1, 0.0, 0.0);
    destroyed.condition = Condition::Destroyed;
    let mut dying = exploding(2, 0.0, 0.0);
    dying.reserves.armor.now = -1.0;
    dying.condition = Condition::Dying {
        ticks_left: 50,
        starting: false,
    };
    let mut ships = [destroyed, dying];
    let hazards = StellarHazards {
        gravity: vec![GravitySource {
            at: Vec2::new(2000.0, 0.0),
            gravity: 40,
        }],
        deadly: vec![DeadlyDisc {
            at: Vec2::ZERO,
            radius: 50.0,
        }],
    };
    let shoves = tick_in(&mut combat, &mut ships, &hazards, ADVANCE);
    assert_eq!(shoves_on(&shoves, A), [], "destroyed");
    assert_eq!(shoves_on(&shoves, B).len(), 1, "dying: {shoves:?}");
    assert!(matches!(ships[1].condition, Condition::Dying { .. }));
    assert_eq!(ships[1].reserves.armor.now, -1.0);
    assert_eq!(combat.take_stellar_kills(), []);
    assert_eq!(exploded(&combat.take_events()), []);
}

#[test]
fn a_kill_destroys_at_minus_1000_armour_in_one_explode2_once_however_many_discs() {
    let mut combat = Combat::default();
    let mut ships = [
        exploding(1, 0.0, 0.0),
        exploding(2, 0.0, 100.0),
        exploding(3, 500.0, 0.0),
    ];
    // Disabled by its armour, so its display step before the hazards
    // keeps it so.
    ships[0].condition = Condition::Disabled;
    ships[0].reserves.armor.now = 1.0;
    let overlapping = deadly(&[(Vec2::new(0.0, 50.0), 80.0), (Vec2::ZERO, 80.0)]);
    tick_in(&mut combat, &mut ships, &overlapping, ADVANCE);
    let seen = combat.stellar_kills().to_vec();
    assert_eq!(seen.len(), 2, "seen in place");
    for ship in &ships[..2] {
        assert_eq!(ship.condition, Condition::Destroyed, "{:?}", ship.id);
        assert_eq!(ship.reserves.armor.now, -1000.0);
    }
    assert_eq!(ships[2].condition, Condition::Intact, "clear of both");
    assert_eq!(combat.take_stellar_kills(), seen, "and left there");
    assert_eq!(
        seen,
        [
            StellarKill {
                ship: A,
                was: Condition::Disabled
            },
            StellarKill {
                ship: B,
                was: Condition::Intact
            }
        ]
    );
    let events = combat.take_events();
    assert_eq!(
        exploded(&events),
        [
            CombatEvent::Exploded {
                at: Vec2::ZERO,
                explosion: BOOM,
                size: 0.0,
            },
            CombatEvent::Exploded {
                at: Vec2::new(0.0, 100.0),
                explosion: BOOM,
                size: 0.0,
            },
        ]
    );
    assert!(
        !events.iter().any(|event| matches!(
            event,
            CombatEvent::BreakingUp { .. } | CombatEvent::Destroyed { .. }
        )),
        "{events:?}"
    );
    assert_eq!(combat.take_stellar_kills(), [], "taken");
}

#[test]
fn a_ship_with_no_explode2_is_killed_with_no_explosion() {
    let mut combat = Combat::default();
    let mut ships = [Ship::at(1, 0.0, 0.0)];
    tick_in(
        &mut combat,
        &mut ships,
        &deadly(&[(Vec2::ZERO, 10.0)]),
        ADVANCE,
    );
    assert_eq!(ships[0].condition, Condition::Destroyed);
    assert_eq!(exploded(&combat.take_events()), []);
}

#[test]
fn the_second_pass_pulls_and_kills_too() {
    let mut combat = Combat::default();
    let mut ships = [Ship::at(1, 0.0, 0.0), exploding(2, 400.0, 0.0)];
    let hazards = StellarHazards {
        gravity: vec![GravitySource {
            at: Vec2::new(-2000.0, 0.0),
            gravity: 40,
        }],
        deadly: vec![DeadlyDisc {
            at: Vec2::new(400.0, 0.0),
            radius: 10.0,
        }],
    };
    let shoves = tick_in(&mut combat, &mut ships, &hazards, GamePass::Again);
    assert_eq!(shoves_on(&shoves, A).len(), 1);
    assert_eq!(
        combat.take_stellar_kills(),
        [StellarKill {
            ship: B,
            was: Condition::Intact
        }]
    );
}

#[test]
fn a_pulled_ships_beam_knockback_is_clamped_on_the_velocity_the_pull_left() {
    let mut combat = Combat::default();
    let mut target = of_mass(Ship::at(2, 100.0, 0.0), 1.0);
    target.top_speed = 10.0;
    let mut ships = [Ship::at(1, 0.0, 0.0).armed(beam_of(50)), target];
    // Within 548 pixels the pull is Gravity / 30 a tick: 5 east.
    let hazards = pulling(Vec2::new(400.0, 0.0), 150);
    let shoves = tick_in(&mut combat, &mut ships, &hazards, ADVANCE);
    let pushed = shoves_on(&shoves, B);
    assert_eq!(pushed.len(), 2, "the pull, then the beam: {shoves:?}");
    assert!(close_by(pushed[0], Vec2::new(5.0, 0.0)), "{pushed:?}");
    // 5 + 50 clamped to 10, not 10 + 5.
    assert!(close_by(pushed[1], Vec2::new(10.0, 0.0)), "{pushed:?}");
    assert!(close_by(ships[1].state.velocity, Vec2::new(10.0, 0.0)));
}

#[test]
fn a_ship_a_deadly_stellar_kills_takes_no_beam_strike_that_tick() {
    let mut combat = Combat::default();
    let mut ships = laser_and_target();
    ships[1].hull.explosion = Some(BOOM);
    let hazards = deadly(&[(Vec2::new(100.0, 0.0), 10.0)]);
    tick_in(&mut combat, &mut ships, &hazards, ADVANCE);
    assert_eq!(ships[1].condition, Condition::Destroyed);
    assert_eq!(ships[1].reserves.shield.now, 100.0, "unhurt by the beam");
    assert_eq!(combat.take_strikes(), []);
}

#[test]
fn a_beam_whose_firer_a_deadly_stellar_kills_stays_where_its_firer_was() {
    let mut combat = Combat::default();
    let mut ships = laser_and_target();
    tick_in(&mut combat, &mut ships, &StellarHazards::default(), ADVANCE);
    assert_eq!(combat.beams()[0].start, Vec2::ZERO);
    ships[0].state.position = Vec2::new(5.0, 6.0);
    let hazards = deadly(&[(Vec2::new(5.0, 6.0), 10.0)]);
    tick_in(&mut combat, &mut ships, &hazards, ADVANCE);
    assert_eq!(ships[0].condition, Condition::Destroyed);
    let beam = combat.beams()[0];
    assert_eq!(beam.start, Vec2::ZERO, "not moved onto its dead firer");
    // As a beam whose firer a hit destroyed.
    let mut combat = Combat::default();
    let mut ships = laser_and_target();
    tick_in(&mut combat, &mut ships, &StellarHazards::default(), ADVANCE);
    ships[0].state.position = Vec2::new(5.0, 6.0);
    ships[0].condition = Condition::Destroyed;
    tick_in(&mut combat, &mut ships, &StellarHazards::default(), ADVANCE);
    assert_eq!(combat.beams()[0].start, beam.start);
}

#[test]
fn a_stellar_kill_makes_no_shockwave() {
    let mut combat = Combat::default();
    let mut heavy = exploding(9, 0.0, 0.0);
    heavy.id = WRECK;
    heavy.fleet = WRECK;
    heavy.hull.mass = 200.0;
    let mut ships = [heavy, holding(1, 30.0, 0.0, 100.0, 300.0, 300.0)];
    tick_in(
        &mut combat,
        &mut ships,
        &deadly(&[(Vec2::ZERO, 5.0)]),
        ADVANCE,
    );
    assert_eq!(ships[0].condition, Condition::Destroyed);
    assert_eq!(left(&ships[1]), (100.0, 300.0), "no shockwave");
    assert_eq!(combat.take_strikes(), []);
    assert_eq!(combat.take_shoves(), []);
    // Nor a death timer on later ticks.
    tick_in(&mut combat, &mut ships, &StellarHazards::default(), ADVANCE);
    assert_eq!(left(&ships[1]), (100.0, 300.0));
    assert_eq!(ships[0].condition, Condition::Destroyed);
}
