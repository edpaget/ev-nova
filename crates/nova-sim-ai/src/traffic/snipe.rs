//! A sniper's flight: `_LowLevelAIHandler`'s sub-goal 0xe
//! (@0x8709d-0x87334 in the `EV Nova` executable), the flying half; the
//! passes it runs are [`fire`]'s.
//!
//! - **Its target** (+0x70) gone, or itself disabled (@0x870ab-0x870b9),
//!   it does nothing: its thrust is already none from the handler's entry
//!   (@0x851f1) and nothing slows it, so it coasts.
//! - **Moving** ([`moving`]): 0.35 or more on either axis ([`MOVING`],
//!   @0x870bf-0x87109), each axis's float speed widened and tested
//!   against the double 0.35, so a speed of the float 0.35 is not
//!   moving. It wants its retro heading, the way its velocity points
//!   turned about (`_CalcInterceptAngle((0,0), v × 100)` + 180,
//!   @0x871b2-0x871eb); or, with a last gun (+0xc8da,
//!   [`Armament::last_gun`](crate::combat::armament::Armament::last_gun))
//!   and by the engine, the lead angle at its target for that gun
//!   (`_CalcLeadAngle` @0x8716a-0x871b0), so it turns toward its target
//!   and thrusts: not a brake at all
//!   ([`RuleKey::SnipeBrakeLead`](crate::RuleKey::SnipeBrakeLead)). It
//!   thrusts while its turn rate plus [`THRUST_SLACK`] is more than how
//!   far it is off (strict, @0x871ef-0x87255), with no reverse key. Its
//!   passes run nothing, and it asks for no point defence: it goes
//!   straight to `_AILaunchFighter` (@0x87329).
//! - **At rest**: its velocity slows by [`DAMPING`] each tick
//!   (@0x8725a-0x8728e), it turns to the plain bearing of its target
//!   (`_CalcInterceptAngle` @0x872aa, not a lead), and never thrusts. It
//!   fires while it faces that bearing within [`GUN_FACING`] turns,
//!   strictly ([`facing`], @0x872c3-0x872fe).
//!
//! Left out (rdm phase weapons-fidelity/phase-30-snipe-left-out-of-phase-17):
//! the whole-degree headings and `_AngularDifference` the original tests
//! by (@0x871ef-0x87255, @0x872aa-0x872fe), and the whole-degree modulo
//! of its retro heading. An inertialess ship's brake (thrust reversed,
//! @0x8710f-0x8713a) waits for inertialess flight
//! (rdm phase ship-handling-fidelity/phase-5-inertialess-flight). The
//! block calls no `PlaySound` or `_beepSnd`.

use crate::ai::fire::{self, GUN_FACING};
use crate::combat::aim::{angle_off, bearing};
use crate::flight::{Controls, ShipState, heading_of, shortest_turn};
use crate::handling::Handling;
use crate::rulebook::RuleSource;
use crate::traffic::autopilot::steer;
use crate::traffic::npc::Npc;

/// How fast, in pixels a tick on either axis, a sniper is moving (the
/// double 0.35 @0xdda68).
pub const MOVING: f64 = 0.35;
/// What a sniper at rest keeps of its velocity each tick (0.95 @0xdd618).
pub const DAMPING: f32 = 0.95;
/// How far beyond a turn, in degrees, a moving sniper still thrusts (1.0
/// @0xdd068).
pub const THRUST_SLACK: f32 = 1.0;

/// Whether a sniper in `state` is moving: 0.35 or more on either axis
/// (`jae` @0x870e3, `jb` @0x87109). The original takes `_FloatAbs` of
/// each axis's float speed, widens it (`cvtss2sd`) and compares it with
/// the double [`MOVING`] (`ucomisd`), so the float 0.35, which widens to
/// 0.34999999403953552, is not moving.
#[must_use]
pub fn moving(state: &ShipState) -> bool {
    f64::from(state.velocity.x.abs()) >= MOVING || f64::from(state.velocity.y.abs()) >= MOVING
}

/// Whether `npc` faces its target at `at` closely enough to fire: within
/// [`GUN_FACING`] turns of the plain bearing, strictly (@0x872c3-0x872fe).
#[must_use]
pub fn facing(npc: &Npc, at: &ShipState) -> bool {
    angle_off(npc.state.heading, bearing(npc.state.position, at.position))
        < GUN_FACING * npc.stats.handling.turn_rate
}

/// The controls `npc` snipes by this tick at its target at `target`, on
/// `handling`, slowing it first when it is at rest; the lead angle a
/// moving one with a last gun turns to by `lead`'s reading (see the
/// module docs). With no target it coasts.
pub fn fly(
    npc: &mut Npc,
    target: Option<&ShipState>,
    handling: &Handling,
    lead: RuleSource,
) -> Controls {
    let Some(target) = target else {
        return Controls::default();
    };
    let state = npc.state;
    if !moving(&state) {
        npc.state.velocity = state.velocity * DAMPING;
        return Controls {
            turn: steer(
                state.heading,
                bearing(state.position, target.position),
                handling,
            ),
            ..Controls::default()
        };
    }
    let last_gun = npc.armament.last_gun().and_then(|gun| {
        npc.armament
            .mounts()
            .iter()
            .find(|mount| mount.spec.id == gun)
    });
    let wanted = match (last_gun, lead) {
        (Some(gun), RuleSource::Engine) => fire::lead_with(npc, target, Some(&gun.spec)),
        _ => heading_of(state.velocity * -1.0),
    };
    Controls {
        thrust: handling.turn_rate + THRUST_SLACK > shortest_turn(state.heading, wanted).abs(),
        turn: steer(state.heading, wanted, handling),
        reverse: false,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::catalog::{WeaponId, WeaponRecord};
    use crate::combat::ShipRef;
    use crate::combat::armament::{Armament, LauncherRounds, Shooter};
    use crate::combat::hull::Condition;
    use crate::combat::report::Reports;
    use crate::combat::weapon::WeaponSpec;
    use crate::flight::Turn;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::weapon;

    fn at(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading,
        }
    }

    /// A sniper (3 degrees a tick, 0.3 thrust) in `state`, carrying a gun
    /// of 128, 10 pixels a tick.
    fn sniper(state: ShipState) -> Npc {
        let gun = WeaponRecord {
            speed: 1000,
            count: 20,
            ..weapon(128)
        };
        Npc {
            state,
            goal: Goal::Snipe(ShipRef::Player),
            armament: Armament::new([(WeaponSpec::new(&gun), 1)]),
            ..crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]))
        }
    }

    /// `npc` having fired its gun once, its last gun.
    fn having_fired(mut npc: Npc) -> Npc {
        let mut fuel = npc.reserves.fuel;
        npc.armament.fire_pick(
            WeaponId(128),
            Condition::Intact,
            Shooter::PLAYER,
            &mut LauncherRounds::default(),
            &mut fuel,
            &mut crate::chance::NeverFires,
            &mut Reports::default(),
            RuleSource::Engine,
            &mut |_| Some(crate::combat::aim::Aim::along(0.0, None)),
        );
        assert_eq!(npc.armament.last_gun(), Some(WeaponId(128)));
        npc
    }

    /// The handling of [`FAST`]: 3 degrees a tick, 0.3 thrust.
    const FAST_HANDLING: Handling = Handling {
        max_speed: 6.0,
        accel: 0.3,
        turn_rate: 3.0,
    };

    #[test]
    fn a_sniper_moving_a_double_0_35_on_either_axis_is_moving_and_the_float_0_35_on_both_rests() {
        // The float 0.35 widens to 0.34999999403953552, below the double
        // 0.35 it is tested against; the next float up is above it.
        let above = f32::from_bits(0.35_f32.to_bits() + 1);
        assert!(f64::from(above) > 0.35_f64);
        assert!(moving(&at(0.0, 0.0, above, 0.0, 0.0)));
        assert!(moving(&at(0.0, 0.0, 0.0, -above, 0.0)));
        assert!(!moving(&at(0.0, 0.0, 0.35, 0.35, 0.0)));
        assert!(!moving(&at(0.0, 0.0, -0.35, 0.0, 0.0)));
        assert!(!moving(&at(0.0, 0.0, 0.0, -0.35, 0.0)));
    }

    #[test]
    fn at_rest_it_slows_by_0_95_a_tick_turns_to_the_plain_bearing_and_never_thrusts() {
        // The target 300 right, crossing down at 5 a tick: its lead lies
        // below the bearing of 90.
        let target = at(300.0, 0.0, 0.0, 5.0, 0.0);
        let mut npc = sniper(at(0.0, 0.0, 0.2, -0.3, 80.0));
        let lead = fire::lead_with(&npc, &target, Some(&npc.armament.mounts()[0].spec));
        assert!(lead > 95.0, "{lead}");
        let controls = fly(&mut npc, Some(&target), &FAST_HANDLING, RuleSource::Engine);
        assert_eq!(npc.state.velocity, Vec2::new(0.2 * 0.95, -0.3 * 0.95));
        assert_eq!(controls.turn, Turn::Right, "to 90");
        assert!(!controls.thrust && !controls.reverse);
        // Within half a turn of the bearing, not of the lead: it holds.
        let mut on_bearing = sniper(at(0.0, 0.0, 0.0, 0.0, 89.0));
        let held = fly(
            &mut on_bearing,
            Some(&target),
            &FAST_HANDLING,
            RuleSource::Engine,
        );
        assert_eq!(held, Controls::default());
    }

    #[test]
    fn moving_with_no_last_gun_it_turns_to_its_retro_heading_and_thrusts_only_within_a_turn_and_one()
     {
        // Moving down at 2: its retro heading is up, 0. A turn and one is
        // 4 degrees, strictly.
        let target = at(-300.0, 0.0, 0.0, 0.0, 0.0);
        let thrusting = |heading: f32| {
            let mut npc = sniper(at(0.0, 0.0, 0.0, 2.0, heading));
            let controls = fly(&mut npc, Some(&target), &FAST_HANDLING, RuleSource::Engine);
            assert_eq!(npc.state.velocity, Vec2::new(0.0, 2.0), "no damping");
            assert!(!controls.reverse);
            controls
        };
        assert!(thrusting(3.99).thrust);
        assert!(!thrusting(4.0).thrust, "strictly");
        assert!(thrusting(356.01).thrust);
        assert_eq!(thrusting(10.0).turn, Turn::Left);
        assert_eq!(thrusting(350.0).turn, Turn::Right);
    }

    #[test]
    fn moving_with_a_last_gun_it_turns_to_the_lead_and_thrusts_by_the_engine_and_to_its_retro_heading_by_the_other_reading()
     {
        // Moving down at 2, its target 300 to the right at rest: its gun
        // (10 a tick) leads it at (300, -60), 79 by the table (78.69 by
        // float); its retro heading is up, 0.
        let target = at(300.0, 0.0, 0.0, 0.0, 0.0);
        let npc = having_fired(sniper(at(0.0, 0.0, 0.0, 2.0, 76.0)));
        let gun = npc.armament.mounts()[0].spec;
        assert_eq!(gun.speed, 10.0);
        assert_eq!(fire::lead_with(&npc, &target, Some(&gun)), 79.0);
        let mut engine = npc.clone();
        let controls = fly(
            &mut engine,
            Some(&target),
            &FAST_HANDLING,
            RuleSource::Engine,
        );
        assert!(controls.thrust, "toward its target");
        assert_eq!(controls.turn, Turn::Right);
        let mut other = npc.clone();
        let controls = fly(&mut other, Some(&target), &FAST_HANDLING, RuleSource::Bible);
        assert!(!controls.thrust, "76 off its retro heading");
        assert_eq!(controls.turn, Turn::Left);
        // With no last gun, the engine too turns to its retro heading.
        let mut fresh = sniper(at(0.0, 0.0, 0.0, 2.0, 76.0));
        let controls = fly(
            &mut fresh,
            Some(&target),
            &FAST_HANDLING,
            RuleSource::Engine,
        );
        assert_eq!((controls.thrust, controls.turn), (false, Turn::Left));
    }

    #[test]
    fn with_its_target_gone_it_coasts() {
        let mut npc = sniper(at(0.0, 0.0, 0.1, 2.0, 0.0));
        let controls = fly(&mut npc, None, &FAST_HANDLING, RuleSource::Engine);
        assert_eq!(controls, Controls::default());
        assert_eq!(npc.state.velocity, Vec2::new(0.1, 2.0), "not slowed");
    }

    #[test]
    fn it_faces_its_target_strictly_within_three_turns_of_the_plain_bearing() {
        // 3 degrees a tick: within 9 of the bearing, 90, strictly.
        let target = at(300.0, 0.0, 0.0, 5.0, 0.0);
        assert!(facing(&sniper(at(0.0, 0.0, 0.0, 0.0, 81.01)), &target));
        assert!(!facing(&sniper(at(0.0, 0.0, 0.0, 0.0, 81.0)), &target));
        assert!(facing(&sniper(at(0.0, 0.0, 0.0, 0.0, 98.99)), &target));
        assert!(!facing(&sniper(at(0.0, 0.0, 0.0, 0.0, 99.0)), &target));
    }
}
