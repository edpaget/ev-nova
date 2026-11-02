//! A fighter docking inside its dock window: `_LowLevelAIHandler`'s sub 8
//! (@0x87e09-0x88111) as the `EV Nova` executable flies it, with what
//! `_HandleShip` does with the burst word +0x34 it leaves. Goal 5's block
//! picks the sub ([`return_flight::sub`]), and the
//! [`autopilot`]'s dock tests its reach before it flies ([`fly`]).
//!
//! - **The thrust window** (@0x87e74-0x87f02): it turns to the carrier's
//!   table bearing (+0x68, `_CalcInterceptAngle` @0x87e81-0x87e9d) as
//!   `_HandleShip` turns it ([`flight::turn_to`]). Off it by less than its
//!   turn rate and [`DOCK_TURN_SLACK`] (1.0 @0xdd068), whole degrees,
//!   strictly, by its heading before the turn ([`fire::off_by`],
//!   @0x87ea9-0x87ee4), it writes +0x30 = `_ShipAccelRate` and +0x34 = 0
//!   (@0x87ef8-0x87eff).
//! - **The dock** (@0x87f0b-0x87ff4): within the carrier's sprite width
//!   on both axes, inclusive, `_AIAddFighterToParent` (@0x87fef). The
//!   autopilot's dock tests it ([`Carrier::reach`](crate::bay::Carrier::reach))
//!   before this flight, so a fighter that docks is gone before its word
//!   matters.
//!   A fighter with no bay for it there gets nothing written (@0x80384)
//!   and exits to @0x88111, skipping the pull and the word below
//!   ([`By::unpulled`]).
//! - **The pull** (@0x87ff9-0x880b8): pull = `_ShipAccelRate` × 10.0
//!   (double @0xdd0b0, [`FORMATION_NUDGE`]) × `_gSpeedMult` (1.0 a tick
//!   here). On each axis, towards the carrier: x += pull when carrier.x -
//!   pull >= x (@0x8805b-0x88068), else x -= pull when x >= carrier.x +
//!   pull (@0x8806f-0x8807f); y the same (@0x8808c-0x880b8). Both tests
//!   are inclusive, so an axis exactly the pull off is pulled onto the
//!   carrier ([`pulled`]). It pulls under both readings below.
//! - **The word** (@0x880bd-0x880c9): +0x34 -= pull, every frame past the
//!   dock test. By the engine's reading of
//!   [`RuleKey::HeldGateBurst`](crate::RuleKey::HeldGateBurst)
//!   ([`Npc::owed_burst`], |+0x34|):
//!   - Inside the window the word is 0 - pull, and +0x30 is not 0, so
//!     `_HandleShip` bursts (@0x34a7b-0x34a84, @0x34b00-0x34b03 →
//!     @0x34bc0) whatever it owed before: it moves along its heading at
//!     the pull, an inertialess one's +0x48 set to it, in place of its
//!     thrust, and owes pull - accel, 9 × its acceleration
//!     ([`held_burst::step`]). It never thrusts plainly there.
//!   - Outside it +0x30 = 0, and the word is the preamble's less the
//!     pull (@0x851e9-0x8520d): a negative word kept owes a pull more, and
//!     `_ShipMaxSpeed` less the pull owes pull - top speed for a ship
//!     whose pull exceeds its top speed, and nothing otherwise. No stock
//!     fighter's pull reaches its top speed.
//!   - At or within its cap ([`TopSpeeds::burst_cap`],
//!     @0x34d10-0x34d2a), `_AIMakeShipStopJumping` ends the burst in the
//!     same frame ([`held_burst::settle`]): goal 0, sub 0, +0x6c = -1 and
//!     +0x50 = +0x30 = +0x34 = 0. A fighter has a lead, so it waits no
//!     Rand(30) + 30 (@0x34d5c-0x34d5f); an inertialess one's velocity is
//!     zeroed (@0x34d46-0x34d54). Goal 0 is never throttled, and the
//!     writer of its goal 5 (`_EscortAI`'s Return @0x83e79-0x83ea3, or
//!     `_HighLevelAIHandler`'s turn-back @0x8d575-0x8d625) writes it again
//!     at the next dispatch, so a fighter facing its carrier bursts and
//!     drops to goal 0 every frame until it docks.
//!   - Above its cap (a Fed Viper's 1.53 against a Fed Carrier's 1.3) the
//!     burst goes on, goal 5 kept, and the next frame's sub 8 writes the
//!     word again, or lowers it by another pull outside the window.
//!   - Double time's second pass runs no `_LowLevelAIHandler`
//!     (`_DoPlayGameWork(0)`), so no word is written: `_HandleShip` bursts
//!     on what the advance pass left, as the kept word
//!     ([`slot_word`](crate::traffic::slot_word)) does.
//!
//!   By the other reading it thrusts as the port flew it before
//!   ([`Word::Writes`]), owing nothing.
//!
//! Sub 8 calls no `PlaySound`, `_ST_PlaySound` or `_beepSnd`, and draws no
//! random number.
//!
//! # Left out
//!
//! - **Sub 8's exits** (@0x87e17-0x87e3f, @0x880fc-0x88108) and **its
//!   carrier test** (@0x87e45-0x87e6e, @0x880d0-0x880f3): reached only
//!   behind an earlier goal 0 that frame, or the one-tick lag the
//!   autopilot's dock records.
//! - **The player's slot branch** (@0x87fbb-0x87fe3, @0x886f6-0x88711):
//!   no fighter is slot 0.

use crate::ai::fire;
use crate::bay::DOCK_TURN_SLACK;
use crate::escort::FORMATION_NUDGE;
#[cfg(doc)]
use crate::flight;
use crate::flight::{Controls, ShipState};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::rulebook::RuleSource;
#[cfg(doc)]
use crate::traffic::autopilot::{self, TopSpeeds};
#[cfg(doc)]
use crate::traffic::held_burst;
use crate::traffic::held_burst::{Flight, Word};
use crate::traffic::npc::Npc;
#[cfg(doc)]
use crate::traffic::return_flight;
use crate::traffic::slot_word::Cap;
use crate::trig;

/// What sub 8's flight flies by beside its handling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct By {
    /// Its own top speed, `_ShipMaxSpeed`
    /// ([`TopSpeeds::own`](crate::traffic::autopilot::TopSpeeds::own)),
    /// which the preamble writes as its word.
    pub top: f32,
    /// The reading of
    /// [`RuleKey::HeldGateBurst`](crate::RuleKey::HeldGateBurst): by the
    /// engine's it flies the word (see the module docs).
    pub word: RuleSource,
    /// Whether it is within its carrier's reach with no bay for it:
    /// `_AIAddFighterToParent` writes nothing (@0x80384) and sub 8 exits
    /// to @0x88111, past the pull and the word.
    pub unpulled: bool,
}

/// One tick of `npc`'s sub 8 docking with a carrier at `carrier`, inside
/// its dock window, on `handling`, at its slot (see the module docs): it
/// wants the carrier's table bearing, thrusting within its window by its
/// heading at the slot, its burst word written as `by` says, and is
/// [`pulled`] towards the carrier. Its flight, which `_HandleShip` flies
/// at its step ([`held_burst::step`]): by the engine's reading a burst
/// along its heading at its pull within the window ([`Word::Keeps`]), by
/// the other a plain thrust ([`Word::Writes`]). Double time's second pass
/// flies the advance pass's word again, writing none.
pub(crate) fn fly(npc: &mut Npc, carrier: &ShipState, handling: &Handling, by: By) -> Flight {
    // +0x68, the table's bearing of the carrier (@0x87e81-0x87e9d).
    let wanted = trig::intercept_angle(npc.state.position, carrier.position);
    // Off it by less than a turn and the slack, strictly, by its heading
    // before `_HandleShip` turns it (@0x87ea9-0x87ee4), its turn rate read
    // at the slot (@0x87eb6).
    let window = fire::off_by(npc.state.heading, wanted) < handling.turn_rate + DOCK_TURN_SLACK;
    let controls = Controls {
        thrust: window,
        ..Controls::default()
    };
    let pull = handling.accel * FORMATION_NUDGE;
    let thrust = if window { handling.accel } else { 0.0 };
    let word = match by.word {
        RuleSource::Engine if window => {
            // +0x34 = 0, then less the pull (@0x87ef8, @0x880c0), unless
            // it exits before the pull.
            npc.owed_burst = (pull > 0.0 && !by.unpulled).then_some(pull);
            Word::Keeps { thrust }
        }
        RuleSource::Engine => {
            // The preamble's word, less the pull (@0x851e9-0x8520d,
            // @0x880c0), unless it exits before the pull.
            if !by.unpulled {
                npc.owed_burst = match npc.owed_burst {
                    Some(owed) => Some(owed + pull),
                    None => (pull > by.top).then_some(pull - by.top),
                };
            }
            Word::Keeps { thrust }
        }
        RuleSource::Bible => Word::Writes,
    };
    if !by.unpulled {
        npc.state.position = pulled(npc.state.position, carrier.position, pull);
    }
    Flight::writes(controls, wanted, Cap::Step).doing(word)
}

/// `at` pulled by `pull` towards `to` on each axis, as sub 8 tests it
/// (@0x8805b-0x880b8): towards it when it is at least the pull off,
/// inclusive, else left.
#[must_use]
pub(crate) fn pulled(at: Vec2, to: Vec2, pull: f32) -> Vec2 {
    let towards = |at: f32, to: f32| {
        if to - pull >= at {
            at + pull
        } else if at >= to + pull {
            at - pull
        } else {
            at
        }
    };
    Vec2::new(towards(at.x, to.x), towards(at.y, to.y))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ai::Goal;
    use crate::combat::ShipRef;
    use crate::escort::FORMATION_NUDGE;
    use crate::flight::{FlightRules, ShipState, facing};
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::rulebook::RuleSource;
    use crate::stats::{INERTIALESS_HULL, ShipStats};
    use crate::traffic::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
    use crate::traffic::npc::Npc;
    use crate::traffic::ship_display::Thrust;
    use nova_sim_base::fixture::{Draws, FAST};
    use std::collections::BTreeMap;

    /// An average ship: 3 pixels a tick at most, 0.1 more a tick, 1° a
    /// tick.
    const AVERAGE: ShipFields = ShipFields {
        speed: 300,
        accel: 300,
        maneuver: 10,
        ..FAST
    };

    /// [`FAST`] with no inertia.
    const RAVEN: ShipFields = ShipFields {
        flags2: INERTIALESS_HULL,
        ..FAST
    };

    /// [`FAST`]'s pull of 1.8 a tick at a top speed of 1.0: `Accel` above
    /// 5 × `Speed`.
    const SLOW_TOP: ShipFields = ShipFields { speed: 100, ..FAST };

    /// A pull of 10 a tick at a top speed of 10.
    const EVEN: ShipFields = ShipFields {
        speed: 1000,
        accel: 5000,
        ..FAST
    };

    /// No acceleration, so no pull.
    const DRIFTER: ShipFields = ShipFields { accel: 0, ..FAST };

    fn at(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading,
        }
    }

    /// The carrier, at rest at the centre.
    fn carrier() -> ShipState {
        at(0.0, 0.0, 0.0, 0.0, 0.0)
    }

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    fn near(a: Option<f32>, b: f32) -> bool {
        a.is_some_and(|a| (a - b).abs() < 1e-5)
    }

    /// A fighter of `fields` returning from `start` to the player, its
    /// dock window 100 and the player's sprite 40 across, owing `owed`.
    fn docking(fields: ShipFields, start: ShipState, owed: Option<f32>) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(fields, &[]));
        npc.ship = crate::catalog::ShipId(144);
        npc.goal = Goal::Dock(ShipRef::Player);
        npc.carrier = Some(crate::bay::Carrier {
            ship: ShipRef::Player,
            window: 100.0,
            reach: 40.0,
        });
        npc.state = start;
        npc.owed_burst = owed;
        npc
    }

    /// Flies `npc` a tick at the carrier ([`carrier`]) by `readings`,
    /// beside `beside`, under `rules`, at its own top speed.
    fn fly_by(npc: &mut Npc, readings: Readings, beside: Beside, rules: FlightRules) -> Outcome {
        let speeds = TopSpeeds::own_of(npc);
        fly_at(npc, speeds, readings, beside, rules)
    }

    /// [`fly_by`] at `speeds`.
    fn fly_at(
        npc: &mut Npc,
        speeds: TopSpeeds,
        readings: Readings,
        beside: Beside,
        rules: FlightRules,
    ) -> Outcome {
        autopilot::fly(
            npc,
            &[],
            &BTreeMap::new(),
            Some(&carrier()),
            false,
            beside,
            speeds,
            rules,
            readings,
            &crate::ai::NO_WEAPONS,
            &mut Draws::of(&[]),
        )
    }

    /// Flies `npc` a tick at the carrier by the engine's readings, on the
    /// advance pass.
    fn fly(npc: &mut Npc) -> Outcome {
        fly_by(
            npc,
            Readings::default(),
            Beside::default(),
            FlightRules::default(),
        )
    }

    /// The engine's readings but `held_burst`'s other.
    fn by_the_other() -> Readings {
        Readings {
            held_burst: RuleSource::Bible,
            ..Readings::default()
        }
    }

    fn pull_of(npc: &Npc) -> f32 {
        npc.stats.handling.accel * FORMATION_NUDGE
    }

    // Inside the window, by the engine.

    #[test]
    fn facing_its_carrier_it_bursts_along_its_heading_at_its_pull_whatever_it_owed() {
        // 60 above the carrier, 2° off its bearing (180): inside a turn (3)
        // and 1.0. +0x34 = 0, then less the pull (@0x87ef8, @0x880c0), so
        // `_HandleShip` bursts (@0x34bc0).
        for owed in [None, Some(30.0), Some(0.5)] {
            let mut ship = docking(FAST, at(0.0, -60.0, 0.0, 0.0, 182.0), owed);
            let accel = ship.stats.handling.accel;
            let pull = pull_of(&ship);
            assert_eq!(fly(&mut ship), Outcome::Flying);
            assert_eq!(ship.state.heading, 180.0, "{owed:?}: turned first");
            let along = facing(180.0) * pull;
            assert!(close(ship.state.velocity, along), "{owed:?}: {ship:?}");
            // Moved by the burst, then pulled on y; x is within the pull.
            let moved = Vec2::new(0.0, -60.0) + along + Vec2::new(0.0, pull);
            assert!(close(ship.state.position, moved), "{owed:?}: {ship:?}");
            assert!(
                near(ship.owed_burst, pull - accel),
                "{owed:?}: owes 9 × its acceleration, {:?}",
                ship.owed_burst
            );
            assert_eq!(ship.display.drive.thrust, Thrust::Plain, "{owed:?}");
        }
    }

    #[test]
    fn an_inertialess_fighter_facing_its_carrier_takes_its_pull_as_its_speed() {
        // +0x48 = |+0x34| (@0x34bdf-0x34bf0), its velocity following the
        // speed it had, none.
        let mut ship = docking(RAVEN, at(0.0, -60.0, 0.0, 0.0, 180.0), None);
        let accel = ship.stats.handling.accel;
        let pull = pull_of(&ship);
        fly(&mut ship);
        assert!((ship.speed - pull).abs() < 1e-6, "{}", ship.speed);
        assert!(near(ship.owed_burst, pull - accel), "{:?}", ship.owed_burst);
        assert!(
            close(ship.state.position, Vec2::new(0.0, -60.0 + pull)),
            "only pulled: {:?}",
            ship.state
        );
        // By the other reading it thrusts its speed up by its acceleration.
        let mut ship = docking(RAVEN, at(0.0, -60.0, 0.0, 0.0, 180.0), None);
        let rules = FlightRules::default();
        fly_by(&mut ship, by_the_other(), Beside::default(), rules);
        assert!((ship.speed - accel).abs() < 1e-6, "{}", ship.speed);
        assert!(
            close(ship.state.velocity, Vec2::new(0.0, accel)),
            "{ship:?}"
        );
    }

    // Outside the window, by the engine.

    #[test]
    fn facing_away_it_turns_with_no_thrust_and_its_word_falls_by_its_pull() {
        let away = at(0.0, -60.0, 0.0, 0.0, 0.0);
        let mut ship = docking(FAST, away, Some(5.0));
        let pull = pull_of(&ship);
        fly(&mut ship);
        assert_ne!(ship.state.heading, 0.0, "turning");
        assert_eq!(ship.state.velocity, Vec2::ZERO, "no thrust");
        assert_eq!(ship.display.drive.thrust, Thrust::None);
        assert!(
            near(ship.owed_burst, 5.0 + pull),
            "a negative word kept, less the pull: {:?}",
            ship.owed_burst
        );
        // `_ShipMaxSpeed` (6) less the pull (1.8) is not negative.
        let mut ship = docking(FAST, away, None);
        fly(&mut ship);
        assert_eq!(ship.owed_burst, None);
        // A pull of 1.8 beyond a top speed of 1.0 owes the 0.8 between.
        let mut ship = docking(SLOW_TOP, away, None);
        fly(&mut ship);
        assert!(near(ship.owed_burst, 0.8), "{:?}", ship.owed_burst);
        // A pull of exactly its top speed (1.0 × 10 against 10) leaves the
        // word at 0, owing nothing.
        let mut ship = docking(EVEN, away, None);
        assert_eq!(pull_of(&ship), ship.stats.handling.max_speed);
        fly(&mut ship);
        assert_eq!(ship.owed_burst, None);
    }

    #[test]
    fn outside_its_window_the_word_reads_its_own_top_speed_not_its_leads() {
        // The preamble's word is `_ShipMaxSpeed` (@0x851e9-0x8520d), its own:
        // a pull of 1.8 under its own 6.0 owes nothing, though its lead's
        // 1.0, and so its burst cap, is below the pull.
        let mut ship = docking(FAST, at(0.0, -60.0, 0.0, 0.0, 0.0), None);
        let speeds = TopSpeeds {
            own: ship.stats.handling.max_speed,
            lead: Some(1.0),
        };
        let pull = pull_of(&ship);
        assert!(speeds.burst_cap() < pull && pull < speeds.own);
        let rules = FlightRules::default();
        fly_at(
            &mut ship,
            speeds,
            Readings::default(),
            Beside::default(),
            rules,
        );
        assert_eq!(ship.owed_burst, None);
    }

    #[test]
    fn with_no_acceleration_its_window_writes_the_word_0_and_it_owes_nothing() {
        // +0x34 = 0 less a pull of 0 (@0x87ef8, @0x880c0): no word, so no
        // burst, over the word it owed.
        let start = at(0.0, -60.0, 0.0, 0.0, 180.0);
        let mut ship = docking(DRIFTER, start, Some(30.0));
        assert_eq!(pull_of(&ship), 0.0);
        fly(&mut ship);
        assert_eq!(ship.owed_burst, None);
        assert_eq!(ship.state.velocity, Vec2::ZERO);
        assert_eq!(ship.state.position, start.position);
    }

    // Double time's second pass.

    #[test]
    fn by_the_other_reading_it_thrusts_dropping_what_it_owed_and_lowers_nothing() {
        let rules = FlightRules::default();
        let mut ship = docking(FAST, at(0.0, -60.0, 0.0, 0.0, 180.0), Some(30.0));
        let accel = ship.stats.handling.accel;
        fly_by(&mut ship, by_the_other(), Beside::default(), rules);
        assert!(
            close(ship.state.velocity, facing(180.0) * accel),
            "{:?}",
            ship.state
        );
        assert_eq!(ship.owed_burst, None, "its thrust writes +0x34");
        let away = at(0.0, -60.0, 0.0, 0.0, 0.0);
        let mut ship = docking(FAST, away, Some(5.0));
        fly_by(&mut ship, by_the_other(), Beside::default(), rules);
        assert_eq!(ship.owed_burst, Some(5.0));
        let mut ship = docking(SLOW_TOP, away, None);
        fly_by(&mut ship, by_the_other(), Beside::default(), rules);
        assert_eq!(ship.owed_burst, None);
    }

    // Under both readings.

    #[test]
    fn inside_its_window_a_fighter_turns_at_the_carrier_and_thrusts_within_a_turn_and_a_degree() {
        for readings in [Readings::default(), by_the_other()] {
            let flown = |heading: f32| {
                let mut ship = docking(AVERAGE, at(0.0, -60.0, 0.0, 0.0, heading), None);
                fly_by(
                    &mut ship,
                    readings,
                    Beside::default(),
                    FlightRules::default(),
                );
                ship
            };
            // 60 above the carrier, facing away: it turns, and only turns.
            let away = flown(0.0);
            assert_ne!(away.state.heading, 0.0, "turning");
            assert_eq!(away.state.velocity, Vec2::ZERO, "no thrust");
            // Less than a turn (1) and 1.0 off the bearing (180), in whole
            // degrees, strictly: it thrusts.
            for heading in [179.0, 181.0, 180.0] {
                let aimed = flown(heading);
                assert!(
                    aimed.state.velocity.length() > 0.0,
                    "{readings:?} {heading}"
                );
            }
            for heading in [178.0, 182.0, 177.0, 183.0] {
                let aside = flown(heading);
                assert_eq!(aside.state.velocity, Vec2::ZERO, "{readings:?} {heading}");
            }
        }
        assert_eq!(crate::bay::DOCK_TURN_SLACK, 1.0);
    }

    #[test]
    fn inside_its_window_a_fighter_is_pulled_by_ten_ticks_thrust_on_each_axis_reaching_it() {
        for readings in [Readings::default(), by_the_other()] {
            let pulled = |start: ShipState| {
                let mut ship = docking(FAST, start, None);
                fly_by(
                    &mut ship,
                    readings,
                    Beside::default(),
                    FlightRules::default(),
                );
                ship.state.position
            };
            // At rest, facing away from the carrier at (-70, 1.5): FAST
            // pulls 0.18 x 10 = 1.8 a tick, and y is within that.
            let one = pulled(at(-70.0, 1.5, 0.0, 0.0, 270.0));
            assert!(close(one, Vec2::new(-68.2, 1.5)), "{readings:?}: {one:?}");
            let both = pulled(at(50.0, 60.0, 0.0, 0.0, 135.0));
            assert!(close(both, Vec2::new(48.2, 58.2)), "{readings:?}: {both:?}");
            // An axis exactly the pull off is pulled onto the carrier: both
            // tests are inclusive (@0x8805b-0x880b8).
            let pull = ShipStats::new(FAST, &[]).handling.accel * FORMATION_NUDGE;
            let edge = pulled(at(-pull, 60.0, 0.0, 0.0, 135.0));
            assert_eq!(edge, Vec2::new(0.0, 60.0 - pull), "{readings:?}");
            let edge = pulled(at(60.0, pull, 0.0, 0.0, 135.0));
            assert_eq!(edge, Vec2::new(60.0 - pull, 0.0), "{readings:?}");
        }
    }

    #[test]
    fn inside_its_window_a_fighter_is_pulled_by_ten_ticks_of_its_rules_thrust() {
        let thrust_over = FlightRules::from_rulebook(&crate::Rulebook::default().with_value(
            crate::rulebook::RuleValue::ThrustDivisor,
            std::num::NonZeroU32::new(3000).unwrap(),
        ));
        let pulled = |start: ShipState, rules: FlightRules| {
            let mut ship = docking(FAST, start, None);
            fly_by(&mut ship, Readings::default(), Beside::default(), rules);
            ship.state.position
        };
        // FAST's 900 / 3000 is 0.3 a tick, ten ticks' 3; y's 1.5 within it.
        let quicker = pulled(at(-70.0, 1.5, 0.0, 0.0, 270.0), thrust_over);
        assert!(close(quicker, Vec2::new(-67.0, 1.5)), "{quicker:?}");
        let both = pulled(at(50.0, 60.0, 0.0, 0.0, 135.0), thrust_over);
        assert!(close(both, Vec2::new(47.0, 57.0)), "{both:?}");
        let original = pulled(at(50.0, 60.0, 0.0, 0.0, 135.0), FlightRules::default());
        assert!(close(original, Vec2::new(48.2, 58.2)), "{original:?}");
    }
}
