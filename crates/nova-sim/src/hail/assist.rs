//! A ship's help to the player once asked: refuelling or repairing it.
//!
//! The rules are the original's (`_AIMakeShipRefuelPlayer` @0x82dbb,
//! `_AIMakeShipRepairPlayer` @0x82df2 and their AI states in
//! `_HighLevelAIHandler` in the `EV Nova` executable). The help the
//! player [`need`]s is a repair while it is disabled, else fuel while it
//! holds less than [`REFUEL_UNTIL`] and has a tank at all.
//!
//! An assisting ship flies to the player, and is inside its reach
//! ([`within`]) once it is no further than [`reach`] on each axis, which
//! depends on its turn rate (its `Maneuver` / 10, class+0x38 in
//! `_LoadObjectData` @0x7a0fb):
//!
//! - **Refuel**: (10 - turn rate) x 15 ([`REACH_TURNS`] @0xdd65c,
//!   [`REFUEL_REACH`] @0xdda30), at least [`MIN_REFUEL_REACH`], a stated
//!   guard. Inside it, it brakes until both its velocity's components are
//!   below [`DOCKED_SPEED`] (@0xdda68) ([`docked`]); then the player
//!   gains [`REFUEL_STEP`] a tick while it holds no more than
//!   [`REFUEL_UNTIL`] (@0xdd060), no further than its tank holds.
//! - **Repair**: (10 - turn rate) x 30 + 50 ([`REPAIR_REACH`] @0xdd084,
//!   [`REPAIR_REACH_BASE`] @0xdd064), the turns no fewer than none, a
//!   stated guard. Once it has been inside for [`REPAIR_TICKS`] (the
//!   timer @0x86d54), the player's armour is raised [`REPAIR_STEP`] at a
//!   time until it is no longer disabled. The original's 3-pixel docking
//!   manoeuvre is simplified to the reach.

use super::Help;
use crate::combat::hull::Condition;
use crate::flight::ShipState;
use crate::reserves::Gauge;

/// The turns a reach is worked out from, less the ship's turn rate.
pub const REACH_TURNS: f32 = 10.0;
/// A refuel's reach per turn.
pub const REFUEL_REACH: f32 = 15.0;
/// The least a refuel's reach is.
pub const MIN_REFUEL_REACH: f32 = 15.0;
/// A repair's reach per turn.
pub const REPAIR_REACH: f32 = 30.0;
/// What a repair's reach adds.
pub const REPAIR_REACH_BASE: f32 = 50.0;
/// How slow, in pixels a tick on each axis, a ship refuelling must be.
pub const DOCKED_SPEED: f32 = 0.35;
/// The fuel a refuel tops the player up past: one jump's.
pub const REFUEL_UNTIL: f32 = 100.0;
/// The fuel a refuel gives a tick.
pub const REFUEL_STEP: f32 = 1.0;
/// The ticks a repairing ship spends inside its reach before it repairs.
pub const REPAIR_TICKS: u32 = 100;
/// The armour a repair raises at a time.
pub const REPAIR_STEP: f32 = 1.0;

/// The help the player needs, its ship in `condition` with `fuel` (see
/// the module docs).
#[must_use]
pub fn need(condition: Condition, fuel: Gauge) -> Option<Help> {
    if condition == Condition::Disabled {
        Some(Help::Repair)
    } else if fuel.now < REFUEL_UNTIL && fuel.max > 0.0 {
        Some(Help::Refuel)
    } else {
        None
    }
}

/// How far, in pixels on each axis, a ship of `turn_rate` (degrees a
/// tick) giving `help` reaches.
#[must_use]
pub fn reach(help: Help, turn_rate: f32) -> f32 {
    let turns = REACH_TURNS - turn_rate;
    match help {
        Help::Refuel => (turns * REFUEL_REACH).max(MIN_REFUEL_REACH),
        Help::Repair => turns.max(0.0).mul_add(REPAIR_REACH, REPAIR_REACH_BASE),
    }
}

/// Whether a ship at `helper` is within `reach` of the player at
/// `player` on each axis.
#[must_use]
pub fn within(helper: &ShipState, player: &ShipState, reach: f32) -> bool {
    let off = helper.position - player.position;
    off.x.abs() <= reach && off.y.abs() <= reach
}

/// Whether a ship at `state` is slow enough to refuel: both its
/// velocity's components below [`DOCKED_SPEED`].
#[must_use]
pub fn docked(state: &ShipState) -> bool {
    state.velocity.x.abs() < DOCKED_SPEED && state.velocity.y.abs() < DOCKED_SPEED
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;

    fn fuel(now: f32, max: f32) -> Gauge {
        Gauge { now, max }
    }

    #[test]
    fn a_disabled_player_needs_a_repair_even_with_little_fuel() {
        assert_eq!(
            need(Condition::Disabled, fuel(0.0, 300.0)),
            Some(Help::Repair)
        );
        assert_eq!(
            need(Condition::Disabled, fuel(300.0, 300.0)),
            Some(Help::Repair)
        );
    }

    #[test]
    fn a_player_with_less_than_a_jumps_fuel_needs_a_refuel() {
        assert_eq!(
            need(Condition::Intact, fuel(99.0, 300.0)),
            Some(Help::Refuel)
        );
        assert_eq!(
            need(Condition::Intact, fuel(99.9, 100.0)),
            Some(Help::Refuel)
        );
        assert_eq!(need(Condition::Intact, fuel(100.0, 300.0)), None);
        assert_eq!(need(Condition::Intact, fuel(0.0, 0.0)), None, "no tank");
    }

    #[test]
    fn a_refuels_reach_shrinks_with_the_turn_rate_down_to_fifteen() {
        assert_eq!(reach(Help::Refuel, 2.0), 120.0, "Maneuver 20");
        assert_eq!(reach(Help::Refuel, 0.0), 150.0);
        assert_eq!(reach(Help::Refuel, 9.0), 15.0);
        assert_eq!(reach(Help::Refuel, 9.5), 15.0, "the guard");
        assert_eq!(reach(Help::Refuel, 30.0), 15.0, "the guard");
    }

    #[test]
    fn a_repairs_reach_is_thirty_a_turn_on_fifty() {
        assert_eq!(reach(Help::Repair, 2.0), 290.0);
        assert_eq!(reach(Help::Repair, 10.0), 50.0);
        assert_eq!(reach(Help::Repair, 12.0), 50.0, "the guard");
    }

    fn at(x: f32, y: f32, vx: f32, vy: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            heading: 0.0,
        }
    }

    #[test]
    fn within_reach_is_on_each_axis_edges_included() {
        let player = at(10.0, -10.0, 0.0, 0.0);
        assert!(within(&at(130.0, 110.0, 0.0, 0.0), &player, 120.0));
        assert!(within(&at(-110.0, -130.0, 0.0, 0.0), &player, 120.0));
        assert!(!within(&at(131.0, -10.0, 0.0, 0.0), &player, 120.0));
        assert!(!within(&at(10.0, -131.0, 0.0, 0.0), &player, 120.0));
        assert!(!within(&at(10.0, 111.0, 0.0, 0.0), &player, 120.0));
        assert!(!within(&at(-111.0, 0.0, 0.0, 0.0), &player, 120.0));
    }

    #[test]
    fn docked_is_slower_than_a_third_of_a_pixel_on_each_axis() {
        assert!(docked(&at(0.0, 0.0, 0.34, -0.34)));
        assert!(!docked(&at(0.0, 0.0, 0.35, 0.0)));
        assert!(!docked(&at(0.0, 0.0, 0.0, -0.35)));
        assert!(!docked(&at(0.0, 0.0, -0.35, 0.0)));
    }

    #[test]
    fn the_values_are_the_originals() {
        assert_eq!(
            [
                REACH_TURNS,
                REFUEL_REACH,
                REPAIR_REACH,
                REPAIR_REACH_BASE,
                DOCKED_SPEED,
                REFUEL_UNTIL
            ],
            [10.0, 15.0, 30.0, 50.0, 0.35, 100.0]
        );
        assert_eq!(REPAIR_TICKS, 100);
    }
}
