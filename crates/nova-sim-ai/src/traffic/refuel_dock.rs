//! A ship refuelling another, the player as a hail asks it to: goal 9's
//! block of `_HighLevelAIHandler` (@0x8f3e5-0x8f6d4 in the `EV Nova`
//! executable, [`high_level`] and [`Traffic::refuel`]), flown through the
//! docking's substate 0xb ([`plunder_dock`]) and substate 1, the stop
//! ([`leave_flight::stop`](crate::traffic::leave_flight)).
//!
//! The setter, `_AIMakeShipRefuelPlayer` (@0x82dbb-0x82df1), sets goal 9,
//! sub 0, the target the player, the provocation and +0x50 none and the AI
//! wait -1.0 (the session's hail). Each think while its AI wait is out
//! (@0x8d4b9-0x8d4d2) and it holds a target (@0x8f3f3), whatever that
//! ship's state (goal 0xf's test of it, @0x8f702, has no twin here):
//!
//! - it flies sub 0xb first (@0x8f408);
//! - beyond [`refuel_box`] of the **target** on either axis, strictly
//!   (`ja` @0x8f47d, `jbe` @0x8f4bb), it closes: sub 0xb
//!   ([`RefuelStep::Close`]). The box has no ×4 for an inertialess ship,
//!   where goal 0xf's has (@0x8f781);
//! - inside it, with either speed at [`STOP_SPEED`] or more as a double
//!   (`jae` @0x8f4f2, `jb` @0x8f518), it stops: sub 1 ([`RefuelStep::Stop`]),
//!   with goal 9's damping off its retro heading (@0x85592-0x855e1);
//! - inside and slow, its velocity is zeroed (@0x8f52b-0x8f533), and the
//!   target's fuel read ([`fuel_step`]): while it holds at most
//!   [`REFUEL_UNTIL`] it gains [`REFUEL_STEP`], with no cap at its tank
//!   (@0x8f548-0x8f563), and the sub stays 0xb, so the low level closes on
//!   the target from rest the same think ([`RefuelStep::Refuel`]); above
//!   it, or NaN (`jp` @0x8f556), the refuel is done
//!   ([`RefuelStep::Done`]): with the target the player, `_beepSnd[1]`
//!   (`snd ` 151) and "`CommName`:  Energy transfer complete, `ship
//!   name`." (@0x8f585-0x8f6a1), the session's from
//!   [`OnLine::RefuelDone`]; with an NPC, nothing said. Either way goal 0,
//!   sub 0, target and nav -1 (@0x8f6b3-0x8f6ce). On that think the low
//!   level runs sub 0, no thrust (@0x851da-0x85236): it coasts.
//!
//! Done is tested before the add, so a refuel ends on the think after the
//! fuel passes 100. The player's flight step clamps its fuel to its tank
//! before the traffic's tick, as `_HandlePlayer` does each frame
//! (@0x6d9c7-0x6d9f2), and an NPC's flight after the thinks, as
//! `_HandleShip` does (@0x35347-0x3536f): so a tank of 100 or less never
//! passes 100 and is refuelled for ever, until something else ends it.
//! The [`World::refuel_cap`] reading caps the add at the tank instead, and
//! a full tank is done too
//! ([`RuleKey::RefuelCap`](crate::RuleKey::RefuelCap)).
//!
//! The refuel's high level runs on each of its thinks though an
//! assister decides nothing, provoked or not, as the repair's does
//! ([`repair_dock`](crate::traffic::repair_dock)). The ships refuelling run
//! in berth order, each reading the fuel the last left, as the original's
//! thinks run slot by slot. The player's fuel given is tallied
//! ([`Traffic::take_fuel_given`]) and lands in the session after the
//! traffic's tick, before the frame is drawn, so no boundary is crossed.
//!
//! Recorded only: the `_statusRedraw` writes (@0x8f56e-0x8f573,
//! @0x8f6a6-0x8f6af), as the status display reads live state; the box
//! taken as a 16-bit word (@0x8f44d), a width; and `_AIEscortFireUnprovoked`
//! after subs 1 and 0xb, nothing for AI type 4 or below. Sub 1's cut of
//! +0x48 is rdm:task/ai-drags-scale-the-npc-speed's.

use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::hail::assist::REFUEL_UNTIL;
use crate::reserves::Gauge;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;
use crate::traffic::on_line::OnLine;
use crate::traffic::plunder::{self, HighLevel};
#[cfg(doc)]
use crate::traffic::plunder_dock;
use crate::traffic::plunder_dock::{BOX_TURNS, DockSub, Docking, SPEED_SLACK};
use crate::traffic::{Traffic, World};

/// The pixels of refuel box per turn (the float 15.0 @0xdda30).
pub const REFUEL_BOX_PER_TURN: f32 = 15.0;
/// The speed on an axis, as a double, from which a refueller inside its
/// box stops (the double 0.35 @0xdda68, the docking's [`SPEED_SLACK`]).
pub const STOP_SPEED: f64 = SPEED_SLACK;
/// The fuel a refuel gives a think (the float 1.0 @0xdd068).
pub const REFUEL_STEP: f32 = 1.0;

/// The refuel box of a class whose turn (+0x38) is `class_turn`:
/// trunc((10 − turn) × 15), in floats (@0x8f414-0x8f444), with no guard.
#[must_use]
pub fn refuel_box(class_turn: f32) -> i32 {
    // Within an i32 for any class's turn.
    #[allow(clippy::cast_possible_truncation)]
    let side = ((BOX_TURNS - class_turn) * REFUEL_BOX_PER_TURN) as i32;
    side
}

/// What a refueller's high level does this think (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefuelStep {
    /// It closes on its target: sub 0xb.
    Close,
    /// It stops: sub 1.
    Stop,
    /// It gives a think's fuel, its velocity zeroed, and closes from rest.
    Refuel,
    /// The refuel is done.
    Done,
}

/// The high level of `npc`'s refuel of a ship at `at`, but for the fuel
/// (see the module docs): [`RefuelStep::Close`] strictly beyond the box
/// on either axis, [`RefuelStep::Stop`] inside it with either speed at
/// [`STOP_SPEED`] or more, else [`RefuelStep::Refuel`].
#[must_use]
pub fn high_level(npc: &Npc, at: &ShipState) -> RefuelStep {
    // f32: the box is converted back to a float for the test.
    #[allow(clippy::cast_precision_loss)]
    let side = refuel_box(npc.hull.class_turn) as f32;
    let off = npc.state.position - at.position;
    let fast = |axis: f32| f64::from(axis.abs()) >= STOP_SPEED;
    if off.x.abs() > side || off.y.abs() > side {
        RefuelStep::Close
    } else if fast(npc.state.velocity.x) || fast(npc.state.velocity.y) {
        RefuelStep::Stop
    } else {
        RefuelStep::Refuel
    }
}

/// The fuel a ship holding `fuel` has after a refuelling think, as `cap`
/// reads it, or none when the refuel is done (see the module docs): by
/// the engine, [`REFUEL_STEP`] more while it holds at most
/// [`REFUEL_UNTIL`], with no cap; by the other reading, no more than its
/// tank, and a full tank done too.
#[must_use]
pub fn fuel_step(fuel: Gauge, cap: RuleSource) -> Option<f32> {
    // Above 100 or NaN is done (`ja`, `jp` @0x8f554-0x8f556).
    if fuel.now <= REFUEL_UNTIL {
        match cap {
            RuleSource::Engine => Some(fuel.now + REFUEL_STEP),
            RuleSource::Bible => {
                (fuel.now < fuel.max).then(|| (fuel.now + REFUEL_STEP).min(fuel.max))
            }
        }
    } else {
        None
    }
}

/// Applies `step` to `npc` refuelling `ship` (see the module docs): its
/// docking at `ship` closing or stopping; refuelling, its velocity zeroed
/// and closing; done, its velocity zeroed, idle, its target, docking and
/// travel stellar (nav, +0x6c) dropped.
pub fn take(npc: &mut Npc, ship: ShipRef, step: RefuelStep) {
    let sub = match step {
        RefuelStep::Close => DockSub::Close,
        RefuelStep::Stop => DockSub::Stop,
        RefuelStep::Refuel => {
            npc.state.velocity = Vec2::ZERO;
            DockSub::Close
        }
        RefuelStep::Done => {
            npc.state.velocity = Vec2::ZERO;
            npc.goal = Goal::Idle;
            npc.target = None;
            npc.docking = None;
            npc.travel_set = false;
            return;
        }
    };
    npc.docking = Some(Docking { ship, sub });
}

impl Traffic {
    /// The refuel of each NPC whose high level is `due` and refuels
    /// ([`HighLevel::Refuels`]), in berth order, in `world` (see the module
    /// docs): the fuel it gives the player tallied, an NPC target's raised
    /// in place, the note of a refuel of the player done queued on the
    /// line at its berth.
    pub(super) fn refuel(&mut self, world: &World, due: &[bool]) {
        let around = world.around(&self.npcs);
        let mut order: Vec<usize> = (0..self.npcs.len())
            .filter(|&at| {
                due.get(at).copied().unwrap_or(false)
                    && plunder::high_level(&self.npcs[at], &around) == HighLevel::Refuels
            })
            .collect();
        order.sort_by_key(|&at| self.npcs[at].berth);
        let steps: Vec<(usize, ShipRef, RefuelStep)> = order
            .into_iter()
            .filter_map(|at| {
                let npc = &self.npcs[at];
                let ship = npc.target?;
                let target = around.state_of(ship)?;
                Some((at, ship, high_level(npc, &target)))
            })
            .collect();
        let mut player_fuel = world.player.map(|player| player.reserves.fuel);
        for (at, ship, step) in steps {
            let step = match step {
                RefuelStep::Refuel => self.give_fuel(ship, &mut player_fuel, world.refuel_cap),
                step => step,
            };
            if step == RefuelStep::Done && ship == ShipRef::Player {
                let (id, kind) = (self.npcs[at].id, self.npcs[at].ship);
                self.queue_on_line(id, OnLine::RefuelDone(id, kind));
            }
            take(&mut self.npcs[at], ship, step);
        }
    }

    /// A think's fuel into `ship`, the player's read from and kept in
    /// `player_fuel`, as `cap` reads it ([`fuel_step`]): what the refuel
    /// does, [`RefuelStep::Refuel`] or [`RefuelStep::Done`].
    fn give_fuel(
        &mut self,
        ship: ShipRef,
        player_fuel: &mut Option<Gauge>,
        cap: RuleSource,
    ) -> RefuelStep {
        let fuel = match ship {
            ShipRef::Player => player_fuel.as_mut(),
            ShipRef::Npc(id) => self
                .npcs
                .iter_mut()
                .find(|npc| npc.id == id)
                .map(|npc| &mut npc.reserves.fuel),
        };
        let Some(fuel) = fuel else {
            return RefuelStep::Done;
        };
        let Some(now) = fuel_step(*fuel, cap) else {
            return RefuelStep::Done;
        };
        if ship == ShipRef::Player {
            self.fuel_given += now - fuel.now;
        }
        fuel.now = now;
        RefuelStep::Refuel
    }

    /// The fuel the ships refuelling the player have given it since this
    /// was last called (see the module docs); taking it empties the tally.
    pub fn take_fuel_given(&mut self) -> f32 {
        std::mem::take(&mut self.fuel_given)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::combat::hull::class_turn;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use nova_sim_base::fixture::FAST;

    /// A ship refuelling the player from (`x`, `y`) at (`vx`, `vy`), its
    /// class turn 3 (a box of 105).
    fn refueller(x: f32, y: f32, vx: f32, vy: f32) -> Npc {
        let mut ship = npc(1, ShipStats::new(FAST, &[]));
        ship.goal = Goal::Assist(crate::hail::Help::Refuel);
        ship.target = Some(ShipRef::Player);
        ship.hull.class_turn = 3.0;
        ship.state.position = Vec2::new(x, y);
        ship.state.velocity = Vec2::new(vx, vy);
        ship
    }

    fn at_centre() -> ShipState {
        ShipState::default()
    }

    fn gauge(now: f32, max: f32) -> Gauge {
        Gauge { now, max }
    }

    #[test]
    fn the_values_are_the_originals() {
        assert_eq!([REFUEL_BOX_PER_TURN, REFUEL_STEP], [15.0, 1.0]);
        assert_eq!(STOP_SPEED, 0.35);
    }

    #[test]
    fn the_refuel_box_is_ten_less_the_class_turn_times_fifteen_in_floats_with_no_guard() {
        assert_eq!(refuel_box(3.0), 105);
        assert_eq!(refuel_box(0.0), 150);
        // In floats (10 - 3.3) × 15 is 100.49999..., truncated to 100.
        assert_eq!(
            refuel_box(class_turn(33)),
            ((10.0 - class_turn(33)) * 15.0) as i32
        );
        assert_eq!(refuel_box(10.5), -7);
        assert_eq!(refuel_box(12.0), -30, "no guard");
    }

    #[test]
    fn the_refuel_high_level_closes_strictly_beyond_the_box_on_either_axis() {
        let step = |x: f32, y: f32| high_level(&refueller(x, y, 0.0, 0.0), &at_centre());
        assert_eq!(step(105.0, -105.0), RefuelStep::Refuel);
        assert_eq!(step(-105.0, 105.0), RefuelStep::Refuel);
        assert_eq!(step(105.5, 0.0), RefuelStep::Close);
        assert_eq!(step(0.0, -105.5), RefuelStep::Close);
        let target = ShipState {
            position: Vec2::new(1000.0, -1000.0),
            ..ShipState::default()
        };
        assert_eq!(
            high_level(&refueller(1105.0, -895.0, 0.0, 0.0), &target),
            RefuelStep::Refuel,
            "measured from the target"
        );
    }

    #[test]
    fn the_refuel_box_is_the_same_for_an_inertialess_ship() {
        let mut ship = refueller(105.0, 0.0, 0.0, 0.0);
        ship.stats.inertialess_hull = true;
        assert!(ship.flies_inertialess(None, crate::inertialess::InertialDampener::PlayerOnly));
        assert_eq!(high_level(&ship, &at_centre()), RefuelStep::Refuel);
        ship.state.position.x = 106.0;
        assert_eq!(high_level(&ship, &at_centre()), RefuelStep::Close, "no ×4");
    }

    #[test]
    fn inside_the_box_it_stops_while_either_speed_is_0_35_as_a_double() {
        let float = 0.35_f32;
        assert!(
            f64::from(float) < 0.35,
            "the float 0.35 is below the double"
        );
        let above = f32::from_bits(float.to_bits() + 1);
        for (vx, vy, step) in [
            (float, 0.0, RefuelStep::Refuel),
            (0.0, -float, RefuelStep::Refuel),
            (above, 0.0, RefuelStep::Stop),
            (0.0, -above, RefuelStep::Stop),
            (-1.0, 3.0, RefuelStep::Stop),
        ] {
            assert_eq!(
                high_level(&refueller(0.0, 0.0, vx, vy), &at_centre()),
                step,
                "({vx}, {vy})"
            );
        }
    }

    #[test]
    fn a_refuelling_think_zeroes_the_velocity_and_keeps_closing() {
        let mut ship = refueller(0.0, 0.0, 0.2, -0.1);
        take(&mut ship, ShipRef::Player, RefuelStep::Refuel);
        assert_eq!(ship.state.velocity, Vec2::ZERO);
        assert_eq!(
            ship.docking,
            Some(Docking {
                ship: ShipRef::Player,
                sub: DockSub::Close
            })
        );
        assert_eq!(ship.goal, Goal::Assist(crate::hail::Help::Refuel));
        let mut far = refueller(500.0, 0.0, 2.0, 0.0);
        take(&mut far, ShipRef::Player, RefuelStep::Close);
        assert_eq!(far.state.velocity, Vec2::new(2.0, 0.0));
        assert_eq!(far.docking.map(|docking| docking.sub), Some(DockSub::Close));
        take(&mut far, ShipRef::Player, RefuelStep::Stop);
        assert_eq!(far.state.velocity, Vec2::new(2.0, 0.0));
        assert_eq!(far.docking.map(|docking| docking.sub), Some(DockSub::Stop));
    }

    #[test]
    fn the_fuel_gains_a_unit_while_100_or_less_with_no_cap_and_is_done_above() {
        let step = |now: f32, max: f32| fuel_step(gauge(now, max), RuleSource::Engine);
        assert_eq!(step(40.0, 300.0), Some(41.0));
        assert_eq!(step(100.0, 300.0), Some(101.0));
        assert_eq!(step(100.5, 300.0), None);
        assert_eq!(step(80.0, 80.0), Some(81.0), "no cap");
        assert_eq!(step(f32::NAN, 300.0), None, "NaN is done");
    }

    #[test]
    fn under_the_cap_reading_the_fuel_stops_at_the_tank_and_a_full_tank_is_done() {
        let step = |now: f32, max: f32| fuel_step(gauge(now, max), RuleSource::Bible);
        assert_eq!(step(79.5, 80.0), Some(80.0));
        assert_eq!(step(80.0, 80.0), None);
        assert_eq!(step(100.0, 300.0), Some(101.0));
        assert_eq!(step(101.0, 300.0), None);
    }

    #[test]
    fn a_finished_refuel_drops_its_goal_target_docking_and_travel_stellar() {
        let mut ship = refueller(0.0, 0.0, 0.1, 0.0);
        ship.docking = Some(Docking {
            ship: ShipRef::Player,
            sub: DockSub::Close,
        });
        ship.travel_set = true;
        take(&mut ship, ShipRef::Player, RefuelStep::Done);
        assert_eq!(ship.goal, Goal::Idle);
        assert_eq!((ship.target, ship.docking), (None, None));
        assert!(!ship.travel_set, "nav -1, @0x8f6ce");
        assert_eq!(ship.state.velocity, Vec2::ZERO);
    }
}
