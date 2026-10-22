//! The flight of a stellar attacker, [`Goal::AttackStellar`]: the
//! original's goal 0x12 as `_HighLevelAIHandler` (@0x8f906-0x8faeb in the
//! `EV Nova` executable) picks its substate and `_LowLevelAIHandler` flies
//! it, each tick in this order:
//!
//! 1. **The band** ([`death_star::band`]), read from the velocity before
//!    the drag: beyond 0.75 r of its stellar it approaches (sub 2), inside
//!    0.25 r it runs out (sub 3), in between it brakes (sub 1) while
//!    moving 0.35 or more on an axis and aims (sub 0x16) otherwise.
//! 2. **The drag**, in band (@0x8faaf-0x8faeb): its velocity and its speed
//!    along its heading (+0x48, [`Npc::speed`]) each cut to [`IN_BAND_DRAG`]
//!    of themselves, in doubles.
//! 3. **The substate**: sub 2 by [`approach_flight::toward`], writing +0x34
//!    as it thrusts; sub 3 by [`leave_flight`]'s run out, from the system's
//!    centre (0,0), not from the stellar; sub 1 by [`leave_flight`]'s stop,
//!    not refuelling, which reads the cut velocity, so a ship just above
//!    0.35 settles; sub 0x16 turning to the stellar's bearing
//!    (`_CalcInterceptAngle`) with no thrust (+0x30 the preamble's 0,
//!    +0x34 untouched, so an owed gate burst is kept). The fire is
//!    [`death_star::trigger`]'s.
//!
//! An inertialess ship flies the same substates through the inertialess
//! paths of each. While its AI wait runs it coasts, as every goal does
//! ([`autopilot`](crate::traffic::autopilot)). Nothing here plays a sound.

#[cfg(doc)]
use crate::ai::Goal;
use crate::ai::death_star::{self, Band};
use crate::ai::leave::Leg;
use crate::catalog::LandingSite;
use crate::combat::armament::Arsenal;
use crate::flight::{Controls, FlightRules};
use crate::handling::Handling;
use crate::traffic::autopilot::{Outcome, steer};
use crate::traffic::held_burst::{self, Word};
use crate::traffic::inertialess::Inertia;
use crate::traffic::npc::Npc;
use crate::traffic::{approach_flight, leave_flight};
use crate::trig;

/// What a stellar attacker in band keeps of its velocity and its speed
/// each tick (the double 0.98 @0xdd988).
pub const IN_BAND_DRAG: f64 = 0.98;

/// Flies `npc`, attacking a stellar among `sites`, one tick on `handling`
/// under `flight`, as `inertia` says, its planet weapon's range read from
/// `weapons` (see the module docs). A stellar no longer among `sites`
/// brakes it.
pub fn fly(
    npc: &mut Npc,
    sites: &[LandingSite],
    weapons: &Arsenal,
    handling: &Handling,
    (flight, inertia): (FlightRules, Inertia),
) -> Outcome {
    let Some((band, site)) = death_star::attack_band(npc, sites, weapons) else {
        return leave_flight::fly(npc, Leg::Brake, handling, flight, inertia);
    };
    if matches!(band, Band::Brake | Band::Aim) {
        // The in-band drag (@0x8faaf-0x8faeb).
        npc.drag_in_doubles(IN_BAND_DRAG);
    }
    match band {
        Band::Approach => {
            let (controls, flown) = approach_flight::toward(&npc.state, site.position, handling);
            held_burst::step(npc, &flown, controls, flight, inertia, Word::Writes);
            Outcome::Flying
        }
        Band::RunOut => leave_flight::fly(npc, Leg::RunOut, handling, flight, inertia),
        Band::Brake => leave_flight::fly(npc, Leg::Brake, handling, flight, inertia),
        Band::Aim => {
            let bearing = trig::intercept_angle(npc.state.position, site.position);
            let controls = Controls {
                turn: steer(npc.state.heading, f32::from(bearing), handling),
                ..Controls::default()
            };
            let word = Word::Keeps { thrust: 0.0 };
            held_burst::step(npc, handling, controls, flight, inertia, word);
            Outcome::Flying
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::catalog::{HullRecord, ShipId, StellarId, StockWeapon, WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::weapon::{PLANET_TYPE, WeaponSpec};
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::npc::Mode;
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::{hull, planet, weapon};

    const HANDLING: Handling = Handling {
        max_speed: 8.0,
        accel: 0.2,
        turn_rate: 4.0,
    };

    /// The planet gun, `wëap` 130: 2 pixels a tick for 100 ticks, so a
    /// range of 200.
    fn planet_gun() -> WeaponRecord {
        WeaponRecord {
            guidance: -1,
            count: 100,
            speed: 200,
            flags2: PLANET_TYPE,
            ..weapon(130)
        }
    }

    /// Class 128 carrying the planet gun in stock.
    fn weapons() -> Arsenal {
        let hull = HullRecord {
            weapons: vec![StockWeapon {
                weapon: WeaponId(130),
                count: 1,
                ammo: 0,
            }],
            ..hull(128)
        };
        Arsenal::new(&[planet_gun()], vec![hull])
    }

    /// Stellar 131 at (1000, 0).
    fn sites() -> Vec<LandingSite> {
        vec![planet(131, 1000.0, 0.0)]
    }

    /// A ship of class 128 attacking 131, at `state`.
    fn attacker(state: ShipState) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
        npc.ship = ShipId(128);
        npc.armament = Armament::new([(WeaponSpec::new(&planet_gun()), 1)]);
        npc.goal = Goal::AttackStellar(StellarId(131));
        npc.state = state;
        npc
    }

    fn state(x: f32, y: f32, (vx, vy): (f32, f32), heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading,
        }
    }

    /// Flies `npc` a tick, inertial.
    fn flown(npc: &mut Npc) -> Outcome {
        fly(
            npc,
            &sites(),
            &weapons(),
            &HANDLING,
            (FlightRules::default(), Inertia::Inertial),
        )
    }

    /// The bearing from `from` to `to`.
    fn bearing(from: Vec2, to: Vec2) -> f32 {
        f32::from(trig::intercept_angle(from, to))
    }

    #[test]
    fn beyond_three_quarters_of_its_range_it_closes_in_and_drops_an_owed_burst() {
        let range = weapons()
            .weapon(WeaponId(130))
            .map_or(0.0, WeaponSpec::range);
        assert_eq!(range, 200.0);
        let from = Vec2::new(0.0, 0.0);
        let to = bearing(from, Vec2::new(1000.0, 0.0));
        let mut npc = attacker(state(0.0, 0.0, (0.0, 0.0), to));
        npc.owed_burst = Some(-30.0);
        assert_eq!(flown(&mut npc), Outcome::Flying);
        assert!(npc.state.velocity.x > 0.0, "{:?}", npc.state);
        assert_eq!(npc.owed_burst, None, "sub 2 writes +0x34");
        // Moving 2.05 at it, past a quarter of 8, within 500 its thrust
        // adds nothing; beyond, it speeds up.
        let mut near = attacker(state(600.0, 0.0, (2.05, 0.0), to));
        flown(&mut near);
        assert!(
            near.state.velocity.x <= 2.05 + 1e-4,
            "a quarter of 8 within 500: {:?}",
            near.state
        );
        let mut far = attacker(state(0.0, 0.0, (2.05, 0.0), to));
        flown(&mut far);
        assert!(far.state.velocity.x > 2.2, "{:?}", far.state);
    }

    #[test]
    fn inside_a_quarter_of_its_range_it_runs_out_from_the_centre_not_the_stellar() {
        // 30 below 131: out from the centre is about right (90), away
        // from the stellar is up (0).
        let out = bearing(Vec2::ZERO, Vec2::new(1000.0, 30.0));
        let mut facing_out = attacker(state(1000.0, 30.0, (0.0, 0.0), out));
        flown(&mut facing_out);
        assert!(
            facing_out.state.velocity.x > 0.0,
            "it thrusts out from the centre"
        );
        let away = bearing(Vec2::new(1000.0, 0.0), Vec2::new(1000.0, 30.0));
        let mut facing_away = attacker(state(1000.0, 30.0, (0.0, 0.0), away));
        flown(&mut facing_away);
        assert_eq!(
            facing_away.state.velocity,
            Vec2::ZERO,
            "not away from the stellar"
        );
    }

    #[test]
    fn in_band_and_moving_it_is_dragged_then_brakes_and_just_above_0_35_settles() {
        // 120 left of 131: in band of 200's 50 to 150.
        let mut npc = attacker(state(880.0, 0.0, (0.351, 0.0), 0.0));
        flown(&mut npc);
        let dragged = (f64::from(0.351_f32) * IN_BAND_DRAG) as f32;
        let settled = (f64::from(dragged) * leave_flight::SETTLE) as f32;
        assert_eq!(npc.state.velocity.x, settled);
        let mut fast = attacker(state(880.0, 0.0, (1.0, 0.0), 270.0));
        flown(&mut fast);
        assert!(fast.state.velocity.x < 0.98, "dragged, then braked");
        assert!(fast.state.velocity.x > 0.0);
        assert_eq!(IN_BAND_DRAG, 0.98);
    }

    #[test]
    fn in_band_at_rest_it_is_dragged_and_turns_to_the_stellar_keeping_an_owed_burst() {
        let mut npc = attacker(state(880.0, 0.0, (0.1, 0.2), 0.0));
        npc.owed_burst = Some(-30.0);
        flown(&mut npc);
        let dragged = |axis: f32| (f64::from(axis) * IN_BAND_DRAG) as f32;
        assert_eq!(npc.state.velocity, Vec2::new(dragged(0.1), dragged(0.2)));
        assert_eq!(npc.owed_burst, Some(-30.0), "no thrust, +0x34 untouched");
        assert!(
            npc.state.heading > 0.0,
            "turning right to the stellar at 90"
        );
    }

    #[test]
    fn an_inertialess_ship_in_band_has_its_speed_dragged_too() {
        let mut npc = attacker(state(880.0, 0.0, (0.0, 0.1), 0.0));
        npc.speed = 0.1;
        fly(
            &mut npc,
            &sites(),
            &weapons(),
            &HANDLING,
            (FlightRules::default(), Inertia::Inertialess { speed: 0.1 }),
        );
        assert_eq!(npc.speed, (f64::from(0.1_f32) * IN_BAND_DRAG) as f32);
    }

    #[test]
    fn a_stellar_gone_from_the_system_brakes_it() {
        let mut npc = attacker(state(0.0, 0.0, (0.1, 0.0), 0.0));
        fly(
            &mut npc,
            &[],
            &weapons(),
            &HANDLING,
            (FlightRules::default(), Inertia::Inertial),
        );
        assert_eq!(
            npc.state.velocity.x,
            (f64::from(0.1_f32) * leave_flight::SETTLE) as f32,
            "it settles, undragged"
        );
    }

    #[test]
    fn the_autopilot_flies_a_stellar_attacker_by_its_band_and_coasts_it_through_its_wait() {
        use crate::traffic::autopilot::{Beside, Readings, TopSpeeds};
        let to = bearing(Vec2::ZERO, Vec2::new(1000.0, 0.0));
        let fly_one = |npc: &mut Npc| {
            crate::traffic::autopilot::fly(
                npc,
                &sites(),
                &std::collections::BTreeMap::new(),
                None,
                false,
                Beside::default(),
                TopSpeeds::own_of(npc),
                FlightRules::default(),
                Readings::default(),
                &weapons(),
                &mut nova_sim_base::fixture::Draws::of(&[]),
            )
        };
        let mut npc = attacker(state(0.0, 0.0, (0.0, 0.0), to));
        fly_one(&mut npc);
        assert!(npc.state.velocity.x > 0.0, "it closes in");
        let mut waiting = attacker(state(0.0, 0.0, (1.0, 0.0), to));
        waiting.ai_wait = 5;
        waiting.mode = Mode::Flying;
        fly_one(&mut waiting);
        assert_eq!(waiting.state.velocity, Vec2::new(1.0, 0.0), "it coasts");
    }
}
