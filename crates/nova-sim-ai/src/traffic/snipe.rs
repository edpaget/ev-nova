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
//!   moving. It wants its retro heading ([`retro`]), the way its velocity
//!   points turned about in the table's whole degrees
//!   (`_CalcInterceptAngle((0,0), v × 100)` + 180, mod 360,
//!   @0x871b2-0x871eb); or, with a last gun (+0xc8da,
//!   [`Armament::last_gun`](crate::combat::armament::Armament::last_gun))
//!   and by the engine, the lead angle at its target for that gun
//!   (`_CalcLeadAngle` @0x8716a-0x871b0), so it turns toward its target
//!   and thrusts: not a brake at all
//!   ([`RuleKey::SnipeBrakeLead`](crate::RuleKey::SnipeBrakeLead)). It
//!   thrusts while its turn rate plus [`THRUST_SLACK`] is more than how
//!   far it is off, its heading truncated to whole degrees
//!   (`_AngularDifference`, [`fire::off_by`]; strict, @0x871ef-0x87255),
//!   with no reverse key. Its
//!   passes run nothing, and it asks for no point defence: it goes
//!   straight to `_AILaunchFighter` (@0x87329).
//! - **At rest**: its velocity slows by [`DAMPING`] each tick
//!   (@0x8725a-0x8728e), it turns to the plain bearing of its target in
//!   the table's whole degrees (`_CalcInterceptAngle` @0x872aa, not a
//!   lead), and never thrusts. It fires while its heading, truncated to
//!   whole degrees, faces that bearing within [`GUN_FACING`] turns,
//!   strictly ([`facing`], @0x872c3-0x872fe).
//!
//! Left out: an inertialess ship's brake (thrust reversed,
//! @0x8710f-0x8713a): a moving inertialess sniper brakes as substate 1
//! does instead ([`inertialess::brake`](crate::traffic::inertialess::brake),
//! see the autopilot), until
//! rdm:phase/ship-handling-fidelity/phase-19-inertialess-snipe-and-pursuit.
//! The block calls no `PlaySound` or `_beepSnd`.

use crate::ai::fire::{self, GUN_FACING};
use crate::flight::{Controls, ShipState};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::rulebook::RuleSource;
use crate::traffic::autopilot::steer;
use crate::traffic::npc::Npc;
use crate::trig;

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

/// The retro heading of a ship moving at `velocity`, the way it moves
/// turned about, in whole degrees: the table heading of the velocity
/// scaled by the float 100.0 (`_CalcInterceptAngle((0,0), v × 100)`,
/// @0xdd060), plus 180, modulo 360 in integers (@0x871b2-0x871eb). The
/// table heading is 0..359, so the sum is 180..539 and the result 0..359.
#[must_use]
pub fn retro(velocity: Vec2) -> i16 {
    let heading = i32::from(trig::intercept_angle(Vec2::ZERO, velocity * 100.0));
    i16::try_from((heading + 180) % 360).expect("0..360 fits an i16")
}

/// Whether `npc` faces its target at `at` closely enough to fire: its
/// heading truncated to whole degrees within [`GUN_FACING`] turns of the
/// table's plain bearing (`_CalcInterceptAngle`), strictly
/// (@0x872aa-0x872fe).
#[must_use]
pub fn facing(npc: &Npc, at: &ShipState) -> bool {
    fire::faces(
        npc,
        trig::intercept_angle(npc.state.position, at.position),
        GUN_FACING,
    )
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
            // +0x68, the table's whole-degree bearing (@0x872aa, @0x872b7).
            turn: steer(
                state.heading,
                f32::from(trig::intercept_angle(state.position, target.position)),
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
    // +0x68, a whole degree either way (@0x871ac, @0x871eb).
    let wanted = match (last_gun, lead) {
        (Some(gun), RuleSource::Engine) => fire::lead(npc, target, Some(&gun.spec)),
        _ => retro(state.velocity),
    };
    Controls {
        thrust: fire::off_by(state.heading, wanted) < handling.turn_rate + THRUST_SLACK,
        turn: steer(state.heading, f32::from(wanted), handling),
        reverse: false,
        ..Controls::default()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::catalog::{WeaponId, WeaponRecord};
    use crate::combat::ShipRef;
    use crate::combat::aim::bearing;
    use crate::combat::armament::{Armament, LauncherRounds, Shooter};
    use crate::combat::hull::Condition;
    use crate::combat::report::Reports;
    use crate::combat::weapon::WeaponSpec;
    use crate::flight::{Turn, heading_of};
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
        // Its heading is truncated to whole degrees (`cvttss2si` @0x871f2).
        assert!(thrusting(3.99).thrust);
        assert!(thrusting(3.5).thrust);
        assert!(!thrusting(4.0).thrust, "strictly");
        assert!(!thrusting(4.5).thrust, "4, not less than 4");
        // 356.01 and 356.5 are 356, 4 off: they hold, where their
        // fractional 3.99 and 3.5 would thrust (@0x871f2).
        assert!(!thrusting(356.01).thrust);
        assert!(!thrusting(356.5).thrust);
        assert_eq!(thrusting(10.0).turn, Turn::Left);
        assert_eq!(thrusting(350.0).turn, Turn::Right);
    }

    /// [`FAST_HANDLING`] turning 3.6 degrees a tick: a window of 4.6.
    const FRACTIONAL_TURN: Handling = Handling {
        turn_rate: 3.6,
        ..FAST_HANDLING
    };

    #[test]
    fn moving_it_thrusts_by_its_heading_in_whole_degrees_against_a_fractional_turn_rate() {
        // Moving down at 2: its retro heading is 0. Heading 4.7 is 4, less
        // than 4.6, so it thrusts; the fractional 4.7 would not.
        let target = at(-300.0, 0.0, 0.0, 0.0, 0.0);
        let mut npc = sniper(at(0.0, 0.0, 0.0, 2.0, 4.7));
        let controls = fly(
            &mut npc,
            Some(&target),
            &FRACTIONAL_TURN,
            RuleSource::Engine,
        );
        assert!(controls.thrust);
        let mut beyond = sniper(at(0.0, 0.0, 0.0, 2.0, 5.0));
        let controls = fly(
            &mut beyond,
            Some(&target),
            &FRACTIONAL_TURN,
            RuleSource::Engine,
        );
        assert!(!controls.thrust, "5 is not less than 4.6");
    }

    #[test]
    fn moving_it_thrusts_by_its_retro_heading_in_the_tables_whole_degrees() {
        // Moving at (1, 1): the table's retro heading is 314, the float's 315.
        let target = at(-300.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(retro(Vec2::new(1.0, 1.0)), 314);
        let thrusting = |heading: f32| {
            let mut npc = sniper(at(0.0, 0.0, 1.0, 1.0, heading));
            fly(&mut npc, Some(&target), &FAST_HANDLING, RuleSource::Engine).thrust
        };
        // 318 is 4 off 314: it holds, where the float's 3.5 off 315 would
        // thrust.
        assert!(!thrusting(318.5));
        assert!(thrusting(317.9), "317 is 3 off");
        assert!(!thrusting(310.0), "4 off");
        assert!(thrusting(311.0), "3 off");
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
        // At 3.6 a tick, 83.9 is 83, 4 off the lead, less than 4.6: it
        // thrusts, where the fractional 4.9 would not (@0x871f2).
        let mut whole = having_fired(sniper(at(0.0, 0.0, 0.0, 2.0, 83.9)));
        let controls = fly(
            &mut whole,
            Some(&target),
            &FRACTIONAL_TURN,
            RuleSource::Engine,
        );
        assert!(controls.thrust);
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
    fn retro_is_the_tables_heading_of_the_velocity_turned_about_in_whole_degrees() {
        // (_CalcInterceptAngle((0,0), v x 100) + 180) mod 360, in integers
        // (@0x871b2-0x871eb).
        assert_eq!(retro(Vec2::new(0.0, 2.0)), 0, "180 + 180 wraps");
        assert_eq!(retro(Vec2::new(0.0, -2.0)), 180);
        assert_eq!(retro(Vec2::new(-2.0, 0.0)), 90, "270 + 180 wraps");
        // The table puts (1, 1) at 134, so its retro heading is 314, where
        // the float's is 315.
        assert_eq!(heading_of(Vec2::new(-1.0, -1.0)), 315.0);
        assert_eq!(retro(Vec2::new(1.0, 1.0)), 314);
        assert_eq!(retro(Vec2::new(1.0, 2.0)), 334, "the float's 333.43");
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
        // 3 degrees a tick: within 9 of the table bearing, 90, strictly, its
        // heading truncated to whole degrees (@0x872aa-0x872fe).
        let target = at(300.0, 0.0, 0.0, 5.0, 0.0);
        let faces = |heading: f32| facing(&sniper(at(0.0, 0.0, 0.0, 0.0, heading)), &target);
        // 81.01 and 81.99 are 81, 9 off: neither faces, where their
        // fractional 8.99 and 8.01 would.
        assert!(!faces(81.01));
        assert!(!faces(81.99));
        assert!(!faces(81.0));
        assert!(faces(82.0));
        assert!(faces(98.99), "98, 8 off");
        assert!(!faces(99.0));
        assert!(!faces(99.5));
    }

    #[test]
    fn it_faces_the_tables_bearing_of_its_target() {
        // The target at (300, 300): the table bearing is 134, the float's 135.
        let target = at(300.0, 300.0, 0.0, 0.0, 0.0);
        assert_eq!(trig::intercept_angle(Vec2::ZERO, target.position), 134);
        assert_eq!(bearing(Vec2::ZERO, target.position), 135.0);
        let faces = |heading: f32| facing(&sniper(at(0.0, 0.0, 0.0, 0.0, heading)), &target);
        assert!(
            !faces(143.5),
            "143 is 9 off 134; the float's 8.5 would face"
        );
        assert!(!faces(125.5), "125 is 9 off");
        assert!(faces(142.9), "142, 8 off");
    }

    #[test]
    fn at_rest_it_turns_to_the_tables_bearing_of_its_target() {
        // The target at (300, 300), table bearing 134: at 136.4 it is 2.4
        // off, beyond half a turn (1.5), so it turns left, where the
        // float's 135 is 1.4 off and it would hold (+0x68 @0x872b7).
        let target = at(300.0, 300.0, 0.0, 0.0, 0.0);
        let mut npc = sniper(at(0.0, 0.0, 0.0, 0.0, 136.4));
        let controls = fly(&mut npc, Some(&target), &FAST_HANDLING, RuleSource::Engine);
        assert_eq!(controls.turn, Turn::Left);
        assert!(!controls.thrust);
    }
}
