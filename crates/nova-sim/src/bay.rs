//! Fighter bays: a weapon whose rounds are the fighters aboard.
//!
//! The values are the original's (`_LaunchFighter`, `_AILaunchFighter`,
//! `_EscortAI`, `_LowLevelAIHandler`, `_AIAddFighterToParent`,
//! `_CanBuyFighter` and `_HandlePlayer` in the `EV Nova` executable),
//! the Bible's `wëap` and `shïp` sections and the help page
//! `escorts_fighters.html`.
//!
//! **A bay** is `wëap` `Guidance` [`FIGHTER_BAY`] (99) whose `AmmoType`
//! is a `shïp` ID, [`FIRST_SHIP`] or more: it launches that ship
//! ([`WeaponSpec::carried`](crate::combat::weapon::WeaponSpec::carried)).
//! Its rounds are its own (the engine spends the bay's own slot), the
//! fighters aboard: for the player, the ammunition outfits of the bay
//! (`ModType` 3, `ModVal` the bay); for an NPC, its rounds, each stock
//! bay's `AmmoLoad`. Guidance 99 with any other `AmmoType` fires nothing.
//!
//! **Launching** (`_LaunchFighter` @0x3d62d): a launch spends a round and
//! reloads `Reload` / bays, as any weapon. The fighter needs a free ship
//! slot (none when the system holds
//! [`MAX_SHIPS_IN_SYSTEM`](crate::board::MAX_SHIPS_IN_SYSTEM) ships, the
//! player's counted), or the round is put back. It leaves from the
//! carrier's position along the carrier's heading, the bay's
//! `Inaccuracy` drawn as a shot's, at the carrier's velocity plus the
//! bay's `Speed`/100 pixels a tick along that heading
//! ([`launch_velocity`]: `_AdjustedAccel` @0x3dc70-0x3dca3, @0x2ae4b),
//! with its shield and armour full and its class's stock weapons and
//! rounds. An NPC launches from the first bay (lowest weapon ID) holding
//! rounds, while it attacks (`_AILaunchFighter` @0x81372); a later bay
//! waits until that one is empty.
//!
//! **Docking** (`_HighLevelAIHandler` state 5 @0x8ebd7,
//! `_LowLevelAIHandler` sub 8 @0x87e06-0x880fa): a fighter returning
//! flies at its carrier. Outside its dock window ([`dock_window`]: r =
//! trunc(([`DOCK_WINDOW_BASE`] − `Maneuver` × [`DOCK_MANEUVER_SHARE`]) ×
//! [`DOCK_WINDOW_SCALE`]), raised to [`DOCK_WINDOW_MIN`]) on either axis
//! it closes as an escort closes on its slot; inside, it steers at the
//! carrier, thrusting within its turn rate + [`DOCK_TURN_SLACK`] degrees
//! of it, and is pulled on each axis by
//! [`FORMATION_NUDGE`](crate::escort::FORMATION_NUDGE) ticks of its
//! acceleration (`Accel` × 10.0 @0xdd0b0). Within the carrier's sprite
//! width on both axes ([`Carrier::reach`]) it docks
//! (`_AIAddFighterToParent` @0x802a5): a round goes to the carrier's
//! first bay launching its type, with no cap, an empty bay restarting
//! its reload; the player's round goes to the first ammunition outfit of
//! that bay by ascending outfit ID. A fighter no bay launches is lost.
//!
//! **Capacity** ([`capacity`], `_CanBuyFighter` @0x5a82): `MaxAmmo` ×
//! the bays carried when `MaxAmmo` is above none, else the fighter
//! outfit's `Max`; the fighters aboard and out count against it.
//!
//! **Leaving the system** (`_HandlePlayer` @0x6c0b9-0x6c0f9): each of the
//! player's fighters whose ship type lacks a jump's fuel (`Fuel` 99 or
//! less, `_AIShipHasFuelForJump` @0x7f48f) is abandoned on arrival, and
//! the arrival message counts them ([`FighterNote::Abandoned`],
//! [`ABANDONED_ONE`], [`ABANDONED_MANY`]); the others follow. Landed,
//! fighters stay out. The rulebook's
//! [`RuleKey::FighterRecall`](crate::RuleKey::FighterRecall) chooses this
//! or the uncalled `_InstantFighterRecall` @0x5df3, which puts every
//! fighter back aboard.

use crate::combat::ShipRef;
use crate::flight::facing;
use crate::geometry::Vec2;

/// The `wëap` `Guidance` of a fighter bay.
pub const FIGHTER_BAY: i16 = 99;
/// The first `shïp` ID: a bay's `AmmoType` at or above it is the ship it
/// carries.
pub const FIRST_SHIP: i16 = 128;
/// The least dock window, in pixels.
pub const DOCK_WINDOW_MIN: f32 = 100.0;
/// The dock window's base, before the fighter's `Maneuver` (10.0
/// @0xdd65c).
pub const DOCK_WINDOW_BASE: f64 = 10.0;
/// The share of its `Maneuver` taken off the base.
pub const DOCK_MANEUVER_SHARE: f64 = 0.1;
/// What the dock window is scaled by (50.0 @0xdd064).
pub const DOCK_WINDOW_SCALE: f64 = 50.0;
/// How far beyond its turn rate, in degrees, a docking fighter still
/// thrusts at its carrier (1.0 @0xdd068).
pub const DOCK_TURN_SLACK: f32 = 1.0;
/// `STR#` 2002 #164: one fighter abandoned.
pub const ABANDONED_ONE: &str = "fighter abandoned";
/// `STR#` 2002 #165: several fighters abandoned.
pub const ABANDONED_MANY: &str = "fighters abandoned";

/// The ship that launched a fighter, which it docks with, and the reach
/// of the docking.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Carrier {
    /// The carrier.
    pub ship: ShipRef,
    /// The fighter's dock window, in pixels on each axis ([`dock_window`]).
    pub window: f32,
    /// How near the carrier, in pixels on each axis, the fighter docks:
    /// the carrier's sprite width.
    pub reach: f32,
}

/// What became of the player's fighters, for the flight to tell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FighterNote {
    /// This many fighters were abandoned as the player left the system.
    Abandoned(u32),
}

/// The dock window of a fighter of `Maneuver` `maneuver`.
#[must_use]
pub fn dock_window(maneuver: i16) -> f32 {
    let window = (DOCK_WINDOW_BASE - f64::from(maneuver) * DOCK_MANEUVER_SHARE) * DOCK_WINDOW_SCALE;
    (window.trunc() as f32).max(DOCK_WINDOW_MIN)
}

/// The velocity of a fighter launched along `heading` from a carrier
/// moving at `carrier`, pushed `push` pixels a tick, of `top_speed`.
#[must_use]
pub fn launch_velocity(carrier: Vec2, heading: f32, push: f32, top_speed: f32) -> Vec2 {
    let along = facing(heading);
    let gain = |velocity: f32, share: f32| {
        let push = share * push;
        let top = share * top_speed;
        let past = if push > 0.0 {
            velocity >= top
        } else {
            velocity <= top
        };
        if past { velocity } else { velocity + push }
    };
    Vec2::new(gain(carrier.x, along.x), gain(carrier.y, along.y))
}

/// How many fighters `bays` bays of `MaxAmmo` `max_ammo` hold, with
/// their fighter outfit's `Max` `outfit_max`.
#[must_use]
pub fn capacity(max_ammo: u32, bays: u32, outfit_max: i16) -> u32 {
    if bays == 0 {
        0
    } else if max_ammo > 0 {
        max_ammo.saturating_mul(bays)
    } else {
        u32::try_from(outfit_max).unwrap_or(0)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn the_dock_window_narrows_with_the_maneuver_and_is_at_least_100() {
        assert_eq!(dock_window(80), 100.0, "the Fed Viper");
        assert_eq!(dock_window(90), 100.0, "raised");
        assert_eq!(dock_window(81), 100.0, "99, raised");
        assert_eq!(dock_window(50), 250.0, "the Thunderhead");
        assert_eq!(dock_window(0), 500.0);
        assert_eq!(
            dock_window(79),
            104.0,
            "79 x 0.1 is a hair over 7.9 in doubles, and the window truncates"
        );
        assert_eq!(dock_window(70), 150.0);
        assert_eq!(dock_window(-20), 600.0);
        assert_eq!(DOCK_WINDOW_MIN, 100.0);
    }

    #[test]
    fn a_carrier_at_rest_gives_the_fighter_the_push_along_its_heading() {
        let up = launch_velocity(Vec2::ZERO, 0.0, 4.0, 10.0);
        assert_eq!(up, facing(0.0) * 4.0);
        let right = launch_velocity(Vec2::ZERO, 90.0, 4.0, 10.0);
        assert!((right - Vec2::new(4.0, 0.0)).length() < 1e-5, "{right:?}");
    }

    #[test]
    fn an_axis_at_or_past_the_top_speed_gains_nothing_and_the_other_still_gains() {
        // Heading 45: the push is (2.83, -2.83), the top speed's share
        // (7.07, -7.07).
        let push = facing(45.0) * 4.0;
        let top = facing(45.0) * 10.0;
        let fast_x = launch_velocity(Vec2::new(8.0, 0.0), 45.0, 4.0, 10.0);
        assert_eq!(fast_x, Vec2::new(8.0, push.y), "x past its share");
        let at_y = launch_velocity(Vec2::new(0.0, top.y), 45.0, 4.0, 10.0);
        assert_eq!(at_y, Vec2::new(push.x, top.y), "y at its share");
        let backwards = launch_velocity(Vec2::new(-20.0, 20.0), 45.0, 4.0, 10.0);
        assert_eq!(
            backwards,
            Vec2::new(-20.0 + push.x, 20.0 + push.y),
            "moving the other way, both gain"
        );
        let down = launch_velocity(Vec2::new(0.0, 12.0), 180.0, 4.0, 10.0);
        assert_eq!(down.y, 12.0, "downwards past its share");
        let down = launch_velocity(Vec2::new(0.0, 9.0), 180.0, 4.0, 10.0);
        assert!((down.y - 13.0).abs() < 1e-5, "{down:?}");
    }

    #[test]
    fn a_push_that_crosses_the_top_speed_is_not_clamped() {
        let v = launch_velocity(Vec2::new(0.0, -8.0), 0.0, 4.0, 10.0);
        assert!((v.y + 12.0).abs() < 1e-5, "{v:?}");
        assert!(v.x.abs() < 1e-5);
    }

    #[test]
    fn the_capacity_is_max_ammo_a_bay_else_the_outfits_max() {
        assert_eq!(capacity(4, 2, 9999), 8);
        assert_eq!(capacity(4, 1, 9999), 4);
        assert_eq!(capacity(0, 1, 3), 3, "MaxAmmo none: the outfit's Max");
        assert_eq!(capacity(0, 2, 3), 3, "not a bay");
        assert_eq!(capacity(0, 1, 0), 0);
        assert_eq!(capacity(0, 1, -1), 0, "Max below none");
        assert_eq!(capacity(4, 0, 9999), 0, "no bays");
        assert_eq!(capacity(0, 0, 9999), 0, "no bays");
    }

    #[test]
    fn the_strings_are_str_2002s() {
        assert_eq!(ABANDONED_ONE, "fighter abandoned");
        assert_eq!(ABANDONED_MANY, "fighters abandoned");
        assert_eq!((FIGHTER_BAY, FIRST_SHIP), (99, 128));
    }
}
