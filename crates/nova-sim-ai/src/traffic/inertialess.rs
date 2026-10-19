//! An inertialess NPC's flight (see
//! [`inertialess`](crate::inertialess)): how the autopilot flies a ship
//! whose hull's `Flags2` holds 0x0040 (or, by the Bible's reading of
//! [`RuleKey::InertialDampener`](crate::RuleKey::InertialDampener), that
//! carries an inertial dampener) and that is not keeping close formation
//! ([`Npc::flies_inertialess`]).
//!
//! The ship holds a speed along its heading ([`Npc::speed`], the
//! original's +0x48), and its velocity follows heading x speed
//! (`_HandleShip` @0x34250-0x3425f, before it moves the ship). Its thrust
//! word moves the speed, not the velocity (@0x34ae9-0x34b6d), so the
//! autopilot's controls fly it through
//! [`flight::inertialess_step`](crate::flight::inertialess_step): thrust
//! adds a tick's acceleration, and its brake takes half of one off.
//!
//! - **The brake** ([`brake`]) is `_LowLevelAIHandler` substate 1's
//!   (@0x85384-0x85403): while the ship moves [`MOVING`] or more on
//!   either axis, a thrust of -0.5 x `_ShipAccelRate` (@0x853f6), and no
//!   turn, where an inertial ship turns against its motion.
//! - **The engine's own planning** (the autopilot's stopping distance,
//!   braking speed and velocity match, which no port of the original's
//!   reaches yet) gains an inertialess arm: it faces the velocity it
//!   wants and thrusts or brakes the speed toward that velocity's length
//!   ([`match_speed`]), and stops in the distance half a tick's thrust a
//!   tick takes ([`stopping_distance`], [`braking_speed`]). Without it an
//!   inertialess ship would turn away to brake and speed up instead. A
//!   moving sniper doesn't: it brakes as substate 0xe does, a whole tick's
//!   acceleration off its speed
//!   ([`snipe::inertialess_brake`](crate::traffic::snipe::inertialess_brake)).
//! - **Holding still.** Where the original only stops reading the
//!   ship's controls, while it waits on its course after its glide in or
//!   burst out (@0x34a54) and while it fades into a gate (@0x3501f), its
//!   velocity still follows its speed and it moves ([`drift`]); the gate
//!   zeroes its velocity once, as it reaches the box (@0x8dc65), and
//!   leaves its speed.
//! - **The glide in and the burst out of a gate**, `_HandleShip`'s
//!   negative-cap path (@0x34bc0-0x34d5c), set the speed to the burst's
//!   speed (@0x34bdf-0x34bf0) where an inertial ship's velocity is set,
//!   so the velocity follows it at its rate; as the burst ends, the
//!   velocity is zeroed (@0x34d46-0x34d59) and the speed kept.
//! - **Disabled**, its speed slows by
//!   [`DISABLED_DRAG`](crate::flight::DISABLED_DRAG) with its velocity
//!   (@0x3398f-0x339a1), as every disabled ship's does.
//!
//! None of these blocks plays a sound.

use crate::flight::{Controls, ShipState, heading_of, shortest_turn};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::inertialess::{ADJUST_RATE, BRAKE, adjust_velocity};
use crate::traffic::autopilot::{ALIGNED, steer};
use crate::traffic::npc::Npc;

/// How fast, in pixels a tick on either axis, a ship braking must still
/// move to brake (`_LowLevelAIHandler` substate 1, the double 0.35 at
/// 0xdda68, @0x8539d, @0x853c3).
pub const MOVING: f64 = 0.35;

/// How a ship flies this tick: inertially, or inertialess at its speed
/// along its heading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Inertia {
    /// With inertia: thrust pushes its velocity.
    Inertial,
    /// Inertialess, holding `speed` along its heading.
    Inertialess {
        /// Its speed along its heading.
        speed: f32,
    },
}

impl Inertia {
    /// How `npc` flies this tick, its leader at `leader` and whose
    /// dampener counts as `flight` says ([`Npc::flies_inertialess`]).
    #[must_use]
    pub fn of(npc: &Npc, leader: Option<&ShipState>, flight: crate::flight::FlightRules) -> Self {
        if npc.flies_inertialess(leader, flight.inertial_dampener) {
            Self::Inertialess { speed: npc.speed }
        } else {
            Self::Inertial
        }
    }
}

/// Whether a ship moving at `velocity` moves [`MOVING`] or more on either
/// axis: substate 1 brakes it until it doesn't. Like the snipe's test
/// ([`snipe::moving`](crate::traffic::snipe::moving)), it takes each
/// axis's `_FloatAbs`, widens it (`cvtss2sd`) and compares it with the
/// double (@0x8539d, @0x853c3), so the float 0.35, which widens to
/// 0.34999999403953552, is not moving.
#[must_use]
pub fn moving(velocity: Vec2) -> bool {
    f64::from(velocity.x.abs()) >= MOVING || f64::from(velocity.y.abs()) >= MOVING
}

/// Substate 1's brake for an inertialess ship at `state`: reverse, which
/// an inertialess ship's step takes as [`BRAKE`], while it is
/// [`moving`]; nothing once it isn't. It never turns.
#[must_use]
pub fn brake(state: &ShipState) -> Controls {
    Controls {
        reverse: moving(state.velocity),
        ..Controls::default()
    }
}

/// How far an inertialess ship at `speed` travels while it brakes to a
/// stop at half a tick's thrust a tick ([`BRAKE`]), a whole tick's speed
/// more as the margin; none at or below
/// [`AT_REST_SPEED`](crate::flight::AT_REST_SPEED), for ever when it
/// cannot thrust.
#[must_use]
pub fn stopping_distance(speed: f32, handling: &Handling) -> f32 {
    if speed <= crate::flight::AT_REST_SPEED {
        return 0.0;
    }
    if handling.accel <= 0.0 {
        return f32::INFINITY;
    }
    speed + speed * speed / (2.0 * -BRAKE * handling.accel)
}

/// The fastest an inertialess ship can go and still stop within
/// `distance` ([`stopping_distance`]'s inverse): the `v` with
/// `v + v² / a = distance`. None when it cannot thrust.
#[must_use]
pub fn braking_speed(distance: f32, handling: &Handling) -> f32 {
    let a = handling.accel;
    if a <= 0.0 {
        return 0.0;
    }
    a * ((1.0 + 4.0 * distance.max(0.0) / a).sqrt() - 1.0) / 2.0
}

/// An inertialess ship at `state`, holding `speed`, matching its velocity
/// to `desired`: it turns to face `desired` ([`steer`]) and thrusts while
/// it faces it within [`ALIGNED`] (or its turn rate, if wider) and its
/// speed is more than half a tick's thrust short of `desired`'s length;
/// it brakes (reverse) while its speed is more than that past it, or while
/// it faces more than a quarter turn away. Wanting to be at rest, it
/// brakes and does not turn.
#[must_use]
pub fn match_speed(state: &ShipState, speed: f32, handling: &Handling, desired: Vec2) -> Controls {
    let target = desired.length();
    let slack = handling.accel / 2.0;
    if target <= slack {
        return Controls {
            reverse: speed > slack,
            ..Controls::default()
        };
    }
    let wanted = heading_of(desired);
    let off = shortest_turn(state.heading, wanted).abs();
    let aligned = off <= ALIGNED.max(handling.turn_rate);
    Controls {
        thrust: aligned && speed < target - slack,
        turn: steer(state.heading, wanted, handling),
        reverse: speed > target + slack || (off > 90.0 && speed > slack),
        ..Controls::default()
    }
}

/// One tick of an inertialess `npc` whose controls nothing reads: its
/// velocity follows its speed at `handling`'s rate
/// ([`adjust_velocity`]), and it moves.
pub fn drift(npc: &mut Npc, handling: &Handling) {
    let state = &mut npc.state;
    state.velocity = adjust_velocity(
        state.velocity,
        state.heading,
        npc.speed,
        handling.accel * ADJUST_RATE,
    );
    state.position = state.position + state.velocity;
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::catalog::{LandingSite, StellarId};
    use crate::combat::hull::Condition;
    use crate::escort::{EscortDuty, FormationSlot, slot_position};
    use crate::flight::{FlightRules, Turn, facing};
    use crate::handling::ShipFields;
    use crate::inertialess::InertialDampener;
    use crate::landing::LANDING_SPEED;
    use crate::rulebook::RuleSource;
    use crate::stats::{INERTIAL_DAMPENER, INERTIALESS_HULL, ShipStats};
    use crate::traffic::autopilot::{Outcome, TopSpeeds};
    use crate::traffic::npc::Mode;
    use crate::traffic::spawn::glide_speed;
    use crate::{RuleKey, Rulebook};
    use nova_sim_base::fixture::{Draws, FAST};
    use nova_sim_catalog::fixture::planet;
    use std::collections::BTreeMap;

    /// [`FAST`] with an inertialess hull: 6 pixels a tick at most, 0.18
    /// more a tick, 3° a tick.
    const RAVEN: ShipFields = ShipFields {
        flags2: INERTIALESS_HULL,
        ..FAST
    };

    fn at(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading,
        }
    }

    fn npc(fields: ShipFields, goal: Goal, start: ShipState, speed: f32) -> Npc {
        Npc {
            state: start,
            goal,
            speed,
            ..crate::traffic::fixture::npc(1, ShipStats::new(fields, &[]))
        }
    }

    fn fly_at(
        npc: &mut Npc,
        sites: &[LandingSite],
        other: Option<&ShipState>,
        speeds: TopSpeeds,
        rules: FlightRules,
    ) -> Outcome {
        crate::traffic::autopilot::fly(
            npc,
            sites,
            &BTreeMap::new(),
            other,
            false,
            speeds,
            rules,
            crate::rulebook::RuleSource::Engine,
            &mut Draws::of(&[]),
        )
    }

    fn fly(npc: &mut Npc, sites: &[LandingSite], other: Option<&ShipState>) -> Outcome {
        let speeds = TopSpeeds::own_of(npc);
        fly_at(npc, sites, other, speeds, FlightRules::default())
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn the_named_values_are_pinned() {
        assert_eq!(MOVING, 0.35_f64);
    }

    #[test]
    fn a_velocity_of_the_float_0_35_on_each_axis_is_not_moving_as_substate_1_widens_it() {
        // The float 0.35 widens to 0.34999999403953552, below the double
        // 0.35; the next float up is above it.
        let above = f32::from_bits(0.35_f32.to_bits() + 1);
        assert!(!moving(Vec2::new(0.35, 0.0)));
        assert!(!moving(Vec2::new(0.0, -0.35)));
        assert!(!moving(Vec2::new(-0.35, 0.35)));
        assert!(moving(Vec2::new(above, 0.0)));
        assert!(moving(Vec2::new(0.0, -above)));
    }

    // Who flies inertialess.

    #[test]
    fn an_inertialess_hull_flies_inertialess_and_a_dampener_only_by_the_bible() {
        let plain = npc(FAST, Goal::Hold, ShipState::default(), 0.0);
        let rules = FlightRules::default();
        assert_eq!(Inertia::of(&plain, None, rules), Inertia::Inertial);
        let raven = npc(RAVEN, Goal::Hold, ShipState::default(), 2.0);
        assert_eq!(
            Inertia::of(&raven, None, rules),
            Inertia::Inertialess { speed: 2.0 }
        );
        let dampened = Npc {
            stats: ShipStats::new(
                FAST,
                &[crate::fuel::OutfitMod {
                    mod_type: INERTIAL_DAMPENER,
                    mod_val: 0,
                    count: 1,
                }],
            ),
            ..plain
        };
        assert!(!dampened.flies_inertialess(None, InertialDampener::PlayerOnly));
        let bible = FlightRules::from_rulebook(
            &Rulebook::default().with_override(RuleKey::InertialDampener, RuleSource::Bible),
        );
        assert_eq!(
            Inertia::of(&dampened, None, bible),
            Inertia::Inertialess { speed: 0.0 }
        );
    }

    /// An inertialess escort in slot 2 of 3, spaced 30 apart, from
    /// `start`, keeping formation.
    fn escort(start: ShipState, speed: f32) -> Npc {
        Npc {
            escort: Some(EscortDuty {
                formation: FormationSlot {
                    slot: 2,
                    ships: 3,
                    spacing: 30.0,
                },
                order: None,
            }),
            ..npc(RAVEN, Goal::Formation { guard: None }, start, speed)
        }
    }

    #[test]
    fn an_escort_within_300_of_its_slot_keeps_close_formation_and_is_not_inertialess() {
        let leader = at(400.0, -200.0, 1.0, -2.0, 30.0);
        let slot = slot_position(&leader, 2, 3, 30.0);
        let rule = InertialDampener::PlayerOnly;
        let on = escort(at(slot.x + 300.0, slot.y - 300.0, 0.0, 0.0, 0.0), 0.0);
        assert!(on.keeps_close_formation(Some(&leader)));
        assert!(!on.flies_inertialess(Some(&leader), rule));
        let off = escort(at(slot.x + 301.0, slot.y, 0.0, 0.0, 0.0), 0.0);
        assert!(!off.keeps_close_formation(Some(&leader)));
        assert!(off.flies_inertialess(Some(&leader), rule));
        assert!(!on.keeps_close_formation(None), "with its leader gone");
        let idle = Npc {
            goal: Goal::Idle,
            ..on.clone()
        };
        assert!(
            !idle.keeps_close_formation(Some(&leader)),
            "not in formation"
        );
    }

    #[test]
    fn an_inertialess_escort_on_its_slot_is_flown_with_its_leader_as_an_inertial_one() {
        let leader = at(400.0, -200.0, 1.0, -2.0, 30.0);
        let slot = slot_position(&leader, 2, 3, 30.0);
        let mut ship = escort(at(slot.x, slot.y, 0.0, 0.0, 200.0), 4.0);
        assert_eq!(fly(&mut ship, &[], Some(&leader)), Outcome::Flying);
        assert_eq!(ship.state.velocity, leader.velocity);
        assert_eq!(ship.state.heading, 30.0);
        assert_eq!(ship.speed, 4.0, "its speed left as it was");
    }

    #[test]
    fn an_inertialess_escort_off_its_slot_flies_along_its_heading_and_rejoins() {
        let leader = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let slot = slot_position(&leader, 2, 3, 30.0);
        // Far off, moving sideways at full speed and facing away.
        let mut ship = escort(at(slot.x + 2000.0, slot.y, 0.0, 6.0, 180.0), 6.0);
        fly(&mut ship, &[], Some(&leader));
        let along = facing(ship.state.heading) * ship.speed;
        assert!(
            (ship.state.velocity - along).length() < 6.0,
            "pulled toward its heading: {:?}",
            ship.state
        );
        for _ in 0..2000 {
            if ship.keeps_close_formation(Some(&leader)) {
                return;
            }
            fly(&mut ship, &[], Some(&leader));
        }
        panic!("never rejoined: {:?} at {}", ship.state, ship.speed);
    }

    // Flying.

    #[test]
    fn an_inertialess_npc_holding_its_course_flies_along_its_heading_without_drift() {
        // Moving east at 3, facing north: the velocity swings round.
        let mut ship = npc(RAVEN, Goal::Hold, at(0.0, 0.0, 3.0, 0.0, 0.0), 3.0);
        fly(&mut ship, &[], None);
        assert!(
            near(ship.state.velocity, Vec2::new(3.0 - 0.72, -0.72)),
            "{:?}",
            ship.state
        );
        for _ in 0..10 {
            fly(&mut ship, &[], None);
        }
        assert!(
            near(ship.state.velocity, Vec2::new(0.0, -3.0)),
            "{:?}",
            ship.state
        );
        assert_eq!(ship.speed, 3.0);
    }

    #[test]
    fn an_inertialess_npcs_brake_takes_half_a_ticks_thrust_off_with_no_turn() {
        let mut ship = npc(RAVEN, Goal::Idle, at(0.0, 0.0, 0.0, -3.0, 0.0), 3.0);
        fly(&mut ship, &[], None);
        assert!((ship.speed - (3.0 - 0.09)).abs() < 1e-5, "{}", ship.speed);
        assert_eq!(ship.state.heading, 0.0, "no turn");
        let mut ticks = 1;
        while moving(ship.state.velocity) {
            fly(&mut ship, &[], None);
            assert_eq!(ship.state.heading, 0.0);
            ticks += 1;
            assert!(ticks < 100);
        }
        // Down from 3 to under 0.35 at 0.09 a tick.
        assert_eq!(ticks, 30);
        let left = ship.speed;
        fly(&mut ship, &[], None);
        assert_eq!(ship.speed, left, "it brakes no more");
    }

    #[test]
    fn an_inertialess_npc_thrusting_adds_to_its_speed_toward_its_top_speed() {
        let mut ship = npc(RAVEN, Goal::JumpOut, at(0.0, -100.0, 0.0, 0.0, 0.0), 0.0);
        fly(&mut ship, &[], None);
        assert!((ship.speed - 0.18).abs() < 1e-6, "{}", ship.speed);
        assert!(near(ship.state.velocity, Vec2::new(0.0, -0.18)));
        for _ in 0..100 {
            fly(&mut ship, &[], None);
        }
        // Its plan stops within half a tick's thrust of the top speed.
        assert!((5.91..=6.0).contains(&ship.speed), "{}", ship.speed);
    }

    #[test]
    fn a_disabled_npcs_speed_falls_by_0_995_a_tick_with_its_velocity() {
        for fields in [RAVEN, FAST] {
            let mut ship = npc(fields, Goal::Idle, at(0.0, 0.0, 0.0, -4.0, 0.0), 4.0);
            ship.condition = Condition::Disabled;
            fly(&mut ship, &[], None);
            assert!((ship.speed - 4.0 * 0.995).abs() < 1e-5, "{fields:?}");
            assert!(near(ship.state.velocity, Vec2::new(0.0, -4.0 * 0.995)));
        }
    }

    #[test]
    fn an_inertialess_npc_lands_on_its_stellar() {
        let sites = [planet(140, 500.0, 300.0)];
        for heading in [0.0, 90.0, 225.0] {
            let mut ship = npc(
                RAVEN,
                Goal::Land(StellarId(140)),
                at(0.0, 0.0, 0.0, 0.0, heading),
                0.0,
            );
            let mut landed = None;
            for tick in 0..2000 {
                let outcome = fly(&mut ship, &sites, None);
                if outcome != Outcome::Flying {
                    landed = Some((outcome, tick));
                    break;
                }
            }
            let (outcome, ticks) = landed.expect("lands");
            assert_eq!(outcome, Outcome::Landed(StellarId(140)), "{heading}");
            assert!(ticks < 1200, "{ticks} ticks");
            assert!(ship.state.velocity.length() <= LANDING_SPEED);
        }
    }

    /// An inertialess fighter returning from `start` at `speed` to the
    /// player, its dock window 100 and the player's sprite 40 across.
    fn docking(start: ShipState, speed: f32) -> Npc {
        Npc {
            ship: crate::catalog::ShipId(144),
            carrier: Some(crate::bay::Carrier {
                ship: crate::combat::ShipRef::Player,
                window: 100.0,
                reach: 40.0,
            }),
            ..npc(
                RAVEN,
                Goal::Dock(crate::combat::ShipRef::Player),
                start,
                speed,
            )
        }
    }

    #[test]
    fn inside_its_dock_window_an_inertialess_fighter_thrusts_its_speed() {
        let carrier = at(0.0, 0.0, 0.0, 0.0, 0.0);
        // 60 above the carrier, at rest and facing it.
        let mut ship = docking(at(0.0, -60.0, 0.0, 0.0, 180.0), 0.0);
        assert_eq!(fly(&mut ship, &[], Some(&carrier)), Outcome::Flying);
        assert!((ship.speed - 0.18).abs() < 1e-6, "{}", ship.speed);
        assert!(near(ship.state.velocity, Vec2::new(0.0, 0.18)), "{ship:?}");
    }

    #[test]
    fn an_inertialess_fighter_docks_with_its_carrier() {
        for (carrier, heading) in [
            (at(0.0, 0.0, 0.0, 0.0, 0.0), 0.0),
            (at(0.0, 0.0, 0.0, 0.0, 0.0), 90.0),
            (at(0.0, 0.0, 0.0, 0.0, 0.0), 225.0),
            (at(0.0, 0.0, 1.0, -0.5, 30.0), 0.0),
            (at(0.0, 0.0, 1.0, -0.5, 30.0), 270.0),
        ] {
            let mut carrier = carrier;
            let mut ship = docking(at(300.0, -400.0, 0.0, 0.0, heading), 0.0);
            let mut docked = None;
            for tick in 0..2000 {
                let outcome = fly(&mut ship, &[], Some(&carrier));
                if outcome != Outcome::Flying {
                    docked = Some((outcome, tick));
                    break;
                }
                carrier.position = carrier.position + carrier.velocity;
            }
            let (outcome, ticks) = docked.expect("docks");
            assert_eq!(
                outcome,
                Outcome::Docked {
                    carrier: crate::combat::ShipRef::Player,
                    ship: crate::catalog::ShipId(144),
                },
                "{heading}"
            );
            assert!(ticks < 600, "{heading}: {ticks} ticks");
        }
    }

    #[test]
    fn a_sniper_braked_below_0_35_on_each_axis_turns_to_its_target() {
        let target = at(500.0, 0.0, 0.0, 0.0, 0.0);
        let mut ship = npc(
            RAVEN,
            Goal::Snipe(crate::combat::ShipRef::Player),
            at(0.0, 0.0, 0.0, -0.3, 0.0),
            0.3,
        );
        fly(&mut ship, &[], Some(&target));
        assert_eq!(ship.state.heading, 3.0, "turning right to face east");
    }

    #[test]
    fn an_inertialess_sniper_moving_brakes_a_whole_tick_with_no_turn_until_it_rests_then_turns_to_its_target()
     {
        use crate::traffic::ship_display::Thrust;
        let target = at(500.0, 0.0, 0.0, 0.0, 0.0);
        let mut ship = npc(
            RAVEN,
            Goal::Snipe(crate::combat::ShipRef::Player),
            at(0.0, 0.0, 0.0, -3.0, 0.0),
            3.0,
        );
        fly(&mut ship, &[], Some(&target));
        // A whole tick's 0.18, not substate 1's half.
        assert!((ship.speed - 2.82).abs() < 1e-5, "{}", ship.speed);
        assert_eq!(ship.state.heading, 0.0, "no turn");
        assert_eq!(ship.display.drive.thrust, Thrust::None);
        let mut ticks = 1;
        while crate::traffic::snipe::moving(&ship.state) {
            fly(&mut ship, &[], Some(&target));
            assert_eq!(ship.state.heading, 0.0, "no turn while it moves");
            ticks += 1;
            assert!(ticks < 100);
        }
        // Down from 3 to under 0.35 at 0.18 a tick.
        assert_eq!(ticks, 15);
        let rested = ship.speed;
        fly(&mut ship, &[], Some(&target));
        assert_eq!(ship.state.heading, 3.0, "turning right to face east");
        assert_eq!(ship.speed, rested, "at rest its speed is left alone");
    }

    // The glide in and the burst out.

    #[test]
    fn an_inertialess_ship_gliding_in_takes_the_glides_speed_and_its_velocity_follows() {
        let mut ship = npc(RAVEN, Goal::Idle, at(0.0, -2000.0, 0.0, 50.0, 180.0), 0.0);
        ship.mode = Mode::JumpingIn { tick: 0 };
        let speeds = TopSpeeds {
            own: 6.0,
            lead: Some(9.0),
        };
        fly_at(&mut ship, &[], None, speeds, FlightRules::default());
        assert_eq!(ship.speed, glide_speed(0));
        assert!(
            near(ship.state.velocity, Vec2::new(0.0, 50.0 - 0.72)),
            "pulled toward last tick's speed of none: {:?}",
            ship.state
        );
        fly_at(&mut ship, &[], None, speeds, FlightRules::default());
        assert!(
            near(ship.state.velocity, Vec2::new(0.0, 50.0)),
            "{:?}",
            ship.state
        );
        while matches!(ship.mode, Mode::JumpingIn { .. }) {
            fly_at(&mut ship, &[], None, speeds, FlightRules::default());
        }
        assert_eq!(ship.state.velocity, Vec2::ZERO, "zeroed as the glide ends");
        assert!(
            (ship.speed - glide_speed(37)).abs() < 1e-4,
            "{}",
            ship.speed
        );
        fly_at(&mut ship, &[], None, speeds, FlightRules::default());
        assert!(
            near(ship.state.velocity, Vec2::new(0.0, 0.72)),
            "its velocity follows its speed again: {:?}",
            ship.state
        );
    }

    #[test]
    fn an_inertialess_ship_entering_a_gate_follows_its_speed_while_it_fades() {
        let mut ship = npc(RAVEN, Goal::Idle, at(0.0, 0.0, 0.0, 0.0, 90.0), 2.0);
        ship.mode = Mode::EnteringGate {
            gate: StellarId(300),
            ticks_left: 10,
        };
        fly(&mut ship, &[], None);
        assert!(
            near(ship.state.velocity, Vec2::new(0.72, 0.0)),
            "{:?}",
            ship.state
        );
        assert!(near(ship.state.position, Vec2::new(0.72, 0.0)));
        let mut inertial = npc(FAST, Goal::Idle, at(0.0, 0.0, 0.0, 0.0, 90.0), 2.0);
        inertial.mode = Mode::EnteringGate {
            gate: StellarId(300),
            ticks_left: 10,
        };
        fly(&mut inertial, &[], None);
        assert_eq!(
            inertial.state.position,
            Vec2::ZERO,
            "an inertial one holds still"
        );
    }

    #[test]
    fn an_inertialess_ship_holding_its_course_after_its_burst_follows_its_speed() {
        let mut ship = npc(RAVEN, Goal::Idle, at(0.0, 0.0, 0.0, 0.0, 90.0), 3.0);
        ship.mode = Mode::Coasting { ticks_left: 5 };
        fly(&mut ship, &[], None);
        assert!(
            near(ship.state.velocity, Vec2::new(0.72, 0.0)),
            "{:?}",
            ship.state
        );
        assert!(near(ship.state.position, Vec2::new(0.72, 0.0)));
        assert_eq!(ship.mode, Mode::Coasting { ticks_left: 4 });
        assert_eq!(ship.speed, 3.0, "no brake while it waits");
    }

    // The planning helpers.

    #[test]
    fn the_stopping_distance_and_braking_speed_are_inverse() {
        let handling = Handling {
            max_speed: 6.0,
            accel: 0.2,
            turn_rate: 3.0,
        };
        assert_eq!(stopping_distance(0.0, &handling), 0.0);
        // 2 + 4 / 0.2.
        assert!((stopping_distance(2.0, &handling) - 22.0).abs() < 1e-4);
        assert!((braking_speed(22.0, &handling) - 2.0).abs() < 1e-4);
        assert_eq!(braking_speed(0.0, &handling), 0.0);
        let engineless = Handling {
            accel: 0.0,
            ..handling
        };
        assert_eq!(stopping_distance(2.0, &engineless), f32::INFINITY);
        assert_eq!(braking_speed(22.0, &engineless), 0.0);
    }

    #[test]
    fn matching_a_velocity_holds_within_half_a_ticks_thrust_inclusive() {
        // Half of 0.25 is 0.125: every bound below is exact.
        let handling = Handling {
            max_speed: 6.0,
            accel: 0.25,
            turn_rate: 3.0,
        };
        let east = Vec2::new(4.0, 0.0);
        let facing_east = at(0.0, 0.0, 0.0, 0.0, 90.0);
        let controls = |speed: f32| match_speed(&facing_east, speed, &handling, east);
        assert!(!controls(3.875).thrust, "exactly half a thrust short");
        assert!(controls(3.874).thrust);
        assert!(!controls(4.125).reverse, "exactly half a thrust past");
        assert!(controls(4.126).reverse);
        assert!(
            !controls(4.2).thrust && controls(4.2).reverse,
            "past by more than half"
        );
        let still = at(0.0, 0.0, 0.0, 0.0, 0.0);
        assert!(!match_speed(&still, 0.125, &handling, Vec2::ZERO).reverse);
        assert!(match_speed(&still, 0.126, &handling, Vec2::ZERO).reverse);
        // A quarter turn off is not yet facing away; more is.
        let facing_north = at(0.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(shortest_turn(0.0, heading_of(east)), 90.0);
        assert!(!match_speed(&facing_north, 1.0, &handling, east).reverse);
        let facing_west = at(0.0, 0.0, 0.0, 0.0, 270.0);
        assert!(!match_speed(&facing_west, 0.125, &handling, east).reverse);
        assert!(match_speed(&facing_west, 0.126, &handling, east).reverse);
    }

    #[test]
    fn matching_a_velocity_faces_it_and_thrusts_or_brakes_the_speed_toward_its_length() {
        let handling = Handling {
            max_speed: 6.0,
            accel: 0.2,
            turn_rate: 3.0,
        };
        let east = Vec2::new(4.0, 0.0);
        let facing_east = at(0.0, 0.0, 0.0, 0.0, 90.0);
        let slow = match_speed(&facing_east, 1.0, &handling, east);
        assert!(slow.thrust && !slow.reverse && slow.turn == Turn::None);
        let fast = match_speed(&facing_east, 5.0, &handling, east);
        assert!(!fast.thrust && fast.reverse);
        let matched = match_speed(&facing_east, 4.05, &handling, east);
        assert_eq!(matched, Controls::default(), "within half a tick's thrust");
        let facing_north = at(0.0, 0.0, 0.0, 0.0, 0.0);
        let turning = match_speed(&facing_north, 1.0, &handling, east);
        assert_eq!(turning.turn, Turn::Right);
        assert!(!turning.thrust, "not until it faces it");
        let facing_west = at(0.0, 0.0, 0.0, 0.0, 270.0);
        assert!(
            match_speed(&facing_west, 1.0, &handling, east).reverse,
            "facing away"
        );
        let stop = match_speed(&facing_north, 1.0, &handling, Vec2::ZERO);
        assert!(stop.reverse && stop.turn == Turn::None);
    }
}
