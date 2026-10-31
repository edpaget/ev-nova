//! A fighter flying back to its carrier, goal 5, as the `EV Nova`
//! executable flies it: `_HighLevelAIHandler`'s goal 5 (@0x8ebd7-0x8ed0c),
//! which picks the substate ([`sub`]), and sub 0xb of `_LowLevelAIHandler`
//! outside its dock window ([`close`]). It flies by the engine's reading
//! of [`RuleKey::FormationFlight`](crate::RuleKey::FormationFlight)
//! ([`FormationFlight::Original`](crate::flight::FormationFlight::Original));
//! the other reading is the port's earlier approach, which the
//! [`autopilot`](crate::traffic::autopilot) keeps.
//!
//! - **The substate** ([`sub`]): measured from the carrier, nav = +0x9a.
//!   Within its dock window ([`dock_window`](crate::bay::dock_window)) on
//!   both axes, inclusive (`jb` @0x8ec84, @0x8ecc5), it takes sub 8,
//!   the [`autopilot`](crate::traffic::autopilot)'s dock, otherwise sub
//!   0xb. Then, not seeing its carrier (`_ShipVisibleToShip(carrier,
//!   fighter)` @0x8ed00), it takes sub 1 **over both**, so an unseen
//!   carrier stops a fighter inside its window too, and it never docks
//!   while it can't see the carrier. The sight is the fly pass's to give
//!   ([`Beside::leader_seen`](crate::traffic::autopilot::Beside::leader_seen)).
//! - **Close** (sub 0xb, @0x87337-0x87692): as
//!   [`formation_flight::steer`] flies it. Every writer of goal 5 clears
//!   +0x70 (@0x82e36, @0x83e92, @0x89c10, @0x8d611), so the sub's steer,
//!   +0x70 if set and nav otherwise (@0x873a6-0x873cf), reads nav, the
//!   carrier. `_EscortAI` runs in that same dispatch, so a hit's +0x70 is
//!   cleared before the low level reads it, as at full speed every frame
//!   is a think.
//! - **The swarm branch** (@0x87364-0x873a3): a returning fighter of a
//!   `Flags2` 0x0001 class has it too. Its formation parent in goal 5 is
//!   its leader (`_DoPlayGameWork` @0x44160-0x441c0), so its slot is its
//!   place in the player's formation ([`Npc::formation`]) or in its NPC
//!   parent's ([`Npc::fleet_slot`]), from the carrier.
//! - **Stop** (sub 1): as any stop flies, with the settle
//!   ([`leave_flight::stop_flight`](crate::traffic::leave_flight::stop_flight)).
//! - With no target, `_AIEscortFireUnprovoked` returns (@0x80e6b), so subs
//!   1 and 0xb fire nothing. The block plays no sound and draws no random
//!   number.
//!
//! # Not built
//!
//! The block's +0x8c gate (@0x8ebd7): +0x8c marks a defence ship, never a
//! fighter.

use crate::ai::swarm::SwarmLead;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::traffic::formation_flight;
use crate::traffic::held_burst::{Flight, Word};
use crate::traffic::npc::Npc;

/// The substate goal 5 flies (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sub {
    /// Sub 1: it stops, its carrier unseen.
    Stop,
    /// Sub 0xb: it closes in, outside its dock window.
    Close,
    /// Sub 8: it docks, inside its dock window.
    Dock,
}

/// The substate a fighter at `at` flies beside its carrier at `carrier`,
/// its dock window `window`, seeing the carrier when `seen` (goal 5, see
/// the module docs).
#[must_use]
pub fn sub(at: Vec2, carrier: Vec2, window: f32, seen: bool) -> Sub {
    let off = at - carrier;
    if !seen {
        Sub::Stop
    } else if off.x.abs() <= window && off.y.abs() <= window {
        Sub::Dock
    } else {
        Sub::Close
    }
}

/// Sub 0xb of `npc` closing on its carrier at `carrier` on `handling`,
/// with `swarm` its swarm lead (see the module docs).
pub(crate) fn close(
    npc: &mut Npc,
    carrier: &ShipState,
    swarm: Option<SwarmLead>,
    handling: &Handling,
) -> Flight {
    let slot = npc
        .formation()
        .or(npc.fleet_slot)
        .map(|place| place.position(carrier));
    let (controls, flown) = formation_flight::steer(npc, carrier, true, swarm, slot, handling);
    Flight {
        controls,
        word: Word::Writes,
        flown: Some(flown),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::bay::Carrier;
    use crate::combat::ShipRef;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::flight::{FlightRules, FormationFlight};
    use crate::handling::ShipFields;
    use crate::stats::ShipStats;
    use crate::traffic::autopilot::{self, Beside, Outcome, Readings, TopSpeeds};
    use crate::traffic::fixture::npc;
    use crate::traffic::ship_display::GlowCopy;
    use crate::trig;
    use nova_sim_base::fixture::{Draws, FAST};
    use std::collections::BTreeMap;

    fn at(x: f32, y: f32, vx: f32, vy: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading,
        }
    }

    /// The carrier, at rest at the centre.
    const CARRIER: ShipState = ShipState {
        position: Vec2::ZERO,
        velocity: Vec2::ZERO,
        heading: 0.0,
    };

    /// The earlier formation's rules.
    fn earlier() -> FlightRules {
        FlightRules {
            formation_flight: FormationFlight::Earlier,
            ..FlightRules::default()
        }
    }

    /// A [`FAST`] ship (top speed 6, thrust 0.18, 3 degrees a tick) from
    /// `start`, with `goal`.
    fn ship(goal: Goal, start: ShipState) -> Npc {
        Npc {
            state: start,
            goal,
            ..npc(1, ShipStats::new(FAST, &[]))
        }
    }

    /// A [`FAST`] fighter returning from `start` to the player, its dock
    /// window `window` and the player 30 across.
    fn returning(window: f32, start: ShipState) -> Npc {
        Npc {
            carrier: Some(Carrier {
                ship: ShipRef::Player,
                window,
                reach: 30.0,
            }),
            ..ship(Goal::Dock(ShipRef::Player), start)
        }
    }

    /// `npc` on a slower hull: top speed 3, thrust 0.1, a degree a tick, so
    /// it brakes from 400 off.
    fn sluggish(npc: Npc) -> Npc {
        const FIELDS: ShipFields = ShipFields {
            speed: 300,
            accel: 300,
            maneuver: 10,
            ..FAST
        };
        Npc {
            stats: ShipStats::new(FIELDS, &[]),
            ..npc
        }
    }

    /// `npc` flown a tick beside the ship at `other`, with `beside`, under
    /// `rules`.
    fn flown_under(
        npc: &mut Npc,
        other: Option<&ShipState>,
        beside: Beside,
        rules: FlightRules,
    ) -> Outcome {
        autopilot::fly(
            npc,
            &[],
            &BTreeMap::new(),
            other,
            false,
            beside,
            TopSpeeds::own_of(npc),
            rules,
            Readings::default(),
            &crate::ai::NO_WEAPONS,
            &mut Draws::of(&[]),
        )
    }

    /// `npc` flown a tick beside the carrier, seeing it, by the original.
    fn flown(npc: &mut Npc) -> Outcome {
        flown_under(
            npc,
            Some(&CARRIER),
            Beside::default(),
            FlightRules::default(),
        )
    }

    /// Its carrier unseen.
    fn unseen() -> Beside {
        Beside {
            leader_seen: false,
            ..Beside::default()
        }
    }

    #[test]
    fn the_substate_goes_by_its_window_on_each_axis_inclusive_and_sight_comes_first() {
        let carrier = Vec2::new(100.0, -50.0);
        let off = |x: f32, y: f32| carrier + Vec2::new(x, y);
        for (at, seen, expected) in [
            (off(100.0, 100.0), true, Sub::Dock),
            (off(-100.0, -100.0), true, Sub::Dock),
            (off(100.5, 0.0), true, Sub::Close),
            (off(-100.5, 0.0), true, Sub::Close),
            (off(0.0, 100.5), true, Sub::Close),
            (off(0.0, -100.5), true, Sub::Close),
            (off(0.0, 0.0), false, Sub::Stop),
            (off(100.0, 100.0), false, Sub::Stop),
            (off(500.0, 0.0), false, Sub::Stop),
        ] {
            assert_eq!(sub(at, carrier, 100.0, seen), expected, "{at:?} {seen}");
        }
    }

    #[test]
    fn outside_its_window_it_steers_at_its_carrier_at_full_thrust() {
        // 400 above, moving down at the carrier, it holds its heading and
        // thrusts.
        let mut near = returning(100.0, at(0.0, -400.0, 0.0, 3.0, 180.0));
        assert_eq!(flown(&mut near), Outcome::Flying);
        assert_eq!(near.state.heading, 180.0);
        assert!(near.state.velocity.y > 3.0, "{:?}", near.state);
        // At its top speed too, where the earlier approach turns to brake.
        let mut top = sluggish(returning(100.0, at(0.0, -400.0, 0.0, 3.0, 180.0)));
        flown(&mut top);
        assert_eq!(top.state.heading, 180.0, "{:?}", top.state);
        // Beyond the 200 box it turns to the bearing, whatever it is moving
        // at: 270, from 3 degrees a tick off 0.
        let mut aside = returning(100.0, at(500.0, 0.0, 0.0, 1.0, 0.0));
        flown(&mut aside);
        assert_eq!(aside.state.heading, 357.0, "{:?}", aside.state);
    }

    #[test]
    fn within_200_it_corrects_its_velocity_beyond_15_degrees() {
        // 150 right of it, moving down at 3: its velocity is 90 off the
        // bearing (270), so it wants the gap to its top speed along the
        // bearing.
        let gap = flight_gap(Vec2::new(0.0, 3.0));
        let wanted = trig::intercept_angle(Vec2::ZERO, gap);
        assert_ne!(wanted, 270);
        let start = at(150.0, 0.0, 0.0, 3.0, f32::from(wanted) + 2.0);
        let mut ship = returning(100.0, start);
        flown(&mut ship);
        assert_eq!(ship.state.heading, f32::from(wanted), "{:?}", ship.state);
        // Moving along the bearing, it keeps its own heading.
        let mut true_course = returning(100.0, at(150.0, 0.0, -3.0, 0.0, 250.0));
        flown(&mut true_course);
        assert_eq!(true_course.state.heading, 250.0, "{:?}", true_course.state);
    }

    /// The gap from `velocity` to [`FAST`]'s top speed along bearing 270.
    fn flight_gap(velocity: Vec2) -> Vec2 {
        crate::flight::facing(270.0) * 6.0 - velocity
    }

    #[test]
    fn within_100_closing_it_is_capped_at_half_its_top_speed() {
        // Its window 50, 90 below it moving up at 3 of its 6, facing up.
        let mut capped = returning(50.0, at(0.0, 90.0, 0.0, -3.0, 0.0));
        flown(&mut capped);
        assert_eq!(capped.state.velocity, Vec2::new(0.0, -3.0), "capped");
        // Beyond 100, no cap.
        let mut free = returning(50.0, at(0.0, 110.0, 0.0, -3.0, 0.0));
        flown(&mut free);
        assert!(free.state.velocity.y < -3.0, "{:?}", free.state);
    }

    #[test]
    fn a_swarmer_returning_is_nudged_to_its_slot_and_takes_its_swarm_leaders_glow() {
        let place = FormationSlot {
            slot: 2,
            ships: 3,
            spacing: 30.0,
        };
        let slot = place.position(&CARRIER);
        let swarm = Beside {
            swarm: Some(SwarmLead {
                aim: Vec2::ZERO,
                leads_it: false,
                glow: 12,
            }),
            ..Beside::default()
        };
        // At rest, facing away from its carrier: no thrust.
        let start = at(450.0, 0.0, 0.0, 0.0, 90.0);
        let mut own = returning(100.0, start);
        own.escort = Some(EscortDuty {
            formation: place,
            order: None,
        });
        let mut fleet = returning(100.0, start);
        fleet.fleet_slot = Some(place);
        for (name, mut ship) in [("the player's", own), ("a fleet's", fleet)] {
            ship.swarm_leader = Some(5);
            let rules = FlightRules::default();
            flown_under(&mut ship, Some(&CARRIER), swarm, rules);
            // 0.18 a tick, a nudge of 1.8 toward the slot on each axis.
            let nudge = 0.18_f32 * 10.0;
            let toward = |at: f32, to: f32| at + nudge.copysign(to - at);
            let nudged = Vec2::new(toward(450.0, slot.x), toward(0.0, slot.y));
            assert!(
                (ship.state.position - nudged).length() < 1e-5,
                "{name}: {:?}",
                ship.state
            );
            assert_eq!(ship.display.drive.glow, GlowCopy::Swarm(12), "{name}");
        }
    }

    #[test]
    fn an_unseen_carrier_stops_a_fighter_with_sub_1_then_settles() {
        let rules = FlightRules::default();
        // Outside its window and inside it, moving down, facing up.
        for y in [-400.0, -60.0] {
            let mut ship = returning(100.0, at(0.0, y, 0.0, 2.0, 0.0));
            flown_under(&mut ship, Some(&CARRIER), unseen(), rules);
            assert!(ship.state.velocity.y < 2.0, "{y}: {:?}", ship.state);
        }
        let mut ship = returning(100.0, at(0.0, -400.0, 0.2, -0.1, 45.0));
        flown_under(&mut ship, Some(&CARRIER), unseen(), rules);
        #[allow(clippy::cast_possible_truncation)]
        let settled = |axis: f32| (f64::from(axis) * 0.95) as f32;
        assert_eq!(ship.state.velocity, Vec2::new(settled(0.2), settled(-0.1)));
        assert_eq!(ship.state.heading, 45.0);
    }

    #[test]
    fn an_unseen_carrier_within_reach_is_not_docked_with() {
        let start = at(10.0, 10.0, 0.0, 0.0, 0.0);
        let rules = FlightRules::default();
        let mut blind = returning(100.0, start);
        assert_eq!(
            flown_under(&mut blind, Some(&CARRIER), unseen(), rules),
            Outcome::Flying
        );
        let mut sighted = returning(100.0, start);
        assert_eq!(
            flown(&mut sighted),
            Outcome::Docked {
                carrier: ShipRef::Player,
                ship: sighted.ship
            }
        );
    }

    #[test]
    fn by_the_earlier_formation_a_fighter_rejoins_and_reads_no_sight() {
        let start = at(0.0, -400.0, 0.0, 3.0, 180.0);
        let mut seen = sluggish(returning(100.0, start));
        flown_under(&mut seen, Some(&CARRIER), Beside::default(), earlier());
        assert_ne!(seen.state.heading, 180.0, "it turns to slow down");
        let mut blind = sluggish(returning(100.0, start));
        flown_under(&mut blind, Some(&CARRIER), unseen(), earlier());
        assert_eq!(blind.state, seen.state, "sight unread");
        let mut there = returning(100.0, at(10.0, 10.0, 0.0, 0.0, 0.0));
        assert_eq!(
            flown_under(&mut there, Some(&CARRIER), unseen(), earlier()),
            Outcome::Docked {
                carrier: ShipRef::Player,
                ship: there.ship
            }
        );
    }

    #[test]
    fn inspecting_it_flies_sub_9_and_by_the_earlier_formation_it_follows() {
        let inspecting = |start: ShipState| ship(Goal::Inspect(ShipRef::Player), start);
        let rules = FlightRules::default();
        // 400 above, moving down at 3: sub 9 thrusts on; at its top speed
        // the earlier follow turns to slow down.
        let start = at(0.0, -400.0, 0.0, 3.0, 180.0);
        let mut original = inspecting(start);
        flown_under(&mut original, Some(&CARRIER), Beside::default(), rules);
        assert_eq!(original.state.heading, 180.0);
        assert!(original.state.velocity.y > 3.0, "{:?}", original.state);
        let mut top = sluggish(inspecting(start));
        flown_under(&mut top, Some(&CARRIER), Beside::default(), rules);
        assert_eq!(top.state.heading, 180.0, "{:?}", top.state);
        let mut follows = sluggish(inspecting(start));
        flown_under(&mut follows, Some(&CARRIER), Beside::default(), earlier());
        assert_ne!(follows.state.heading, 180.0, "{:?}", follows.state);
        // Sub 9 has no cap within 100, and thrusts on within it.
        let mut close = inspecting(at(0.0, -90.0, 0.0, 3.0, 180.0));
        flown_under(&mut close, Some(&CARRIER), Beside::default(), rules);
        assert!(close.state.velocity.y > 3.0, "{:?}", close.state);
        // With nothing there it brakes, by either reading.
        for reading in [rules, earlier()] {
            let mut lost = inspecting(at(0.0, 0.0, 0.0, 2.0, 0.0));
            flown_under(&mut lost, None, Beside::default(), reading);
            assert!(lost.state.velocity.y < 2.0, "{:?}", lost.state);
        }
    }

    #[test]
    fn inspecting_and_returning_their_thrust_drops_an_owed_burst() {
        // Subs 9 and 0xb write +0x34 as they thrust (@0x878e4): flying at a
        // ship 400 off, a burst owed from its hold goes unburst.
        for goal in [Goal::Inspect(ShipRef::Player), Goal::Dock(ShipRef::Player)] {
            let mut flier = returning(100.0, at(0.0, -400.0, 0.0, 3.0, 180.0));
            flier.goal = goal;
            flier.owed_burst = Some(30.0);
            flown(&mut flier);
            assert_eq!(flier.owed_burst, None, "{goal:?}");
            assert!(flier.state.velocity.y < 4.0, "{goal:?}: {:?}", flier.state);
        }
    }
}
