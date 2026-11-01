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
//! slot below the 8 of the
//! `nova_sim::board::MAX_SHIPS_IN_SYSTEM` it leaves
//! free (@0x3d648), so none once 55 NPCs are in the system by the engine
//! ([`RuleKey::SpawnReserve`](crate::RuleKey::SpawnReserve)): with none, a
//! copy is not attempted, spending, counting and reloading nothing
//! (`_FirePlayerWeapon` @0x62c61-0x62c72, `_AILaunchFighter`
//! @0x81421-0x81423). It leaves from the
//! carrier's position along the carrier's heading, the bay's
//! `Inaccuracy` drawn as a shot's, at the carrier's velocity plus the
//! bay's `Speed`/100 pixels a tick along that heading
//! ([`launch_velocity`]: `_AdjustedAccel` @0x3dc70-0x3dca3, @0x2ae4b),
//! with its class's stock weapons and rounds, its shield and armour full
//! at a fighter's capacities ([`fighter_capacity`]: 1.333 × its class's,
//! as `_ShipShieldCapacity` and `_ShipArmorCapacity` read AI type 5,
//! @0x2a91-0x2aa7, @0x2cb9-0x2cd2; its shield recharge likewise,
//! @0x2b96-0x2bac), led by its carrier (@0x3d866-0x3d870), and gliding
//! the bay's `Count` (+0x4c, @0x3d843-0x3d85b) until a think ends it.
//! An NPC launches from the first bay (lowest weapon ID) holding
//! rounds, while it attacks (`_AILaunchFighter` @0x81372); a later bay
//! waits until that one is empty. An NPC with no target holds every bay
//! at its full `Reload` (`_HandleShip` @0x3390a-0x3394e), so its first
//! launch comes `Reload` ticks after it takes one.
//!
//! **Docking** (`_HighLevelAIHandler` state 5 @0x8ebd7,
//! `_LowLevelAIHandler` sub 8 @0x87e06-0x880fa): a fighter returning
//! flies at its carrier. Outside its dock window ([`dock_window`]: r =
//! trunc(([`DOCK_WINDOW_BASE`] − `Maneuver` × [`DOCK_MANEUVER_SHARE`]) ×
//! [`DOCK_WINDOW_SCALE`]), raised to [`DOCK_WINDOW_MIN`]) on either axis
//! it closes as an escort closes on its slot; inside, it steers at the
//! carrier, thrusting within its turn rate + [`DOCK_TURN_SLACK`] degrees
//! of it, and is pulled on each axis by
//! `nova_sim::escort::FORMATION_NUDGE` ticks of its
//! acceleration (`Accel` × 10.0 @0xdd0b0). Within the carrier's sprite
//! width on both axes ([`Carrier::reach`], which the original reads live,
//! @0x87f0b-0x87f21) it docks (`_AIAddFighterToParent` @0x802a5): a round
//! goes to the carrier's first bay launching its type, else its first bay
//! launching its sprite base ([`Armament::dock_bay`](crate::combat::armament::Armament::dock_bay),
//! ship class +0xa08, the lowest-ID ship of its `shän`'s `BaseImage`), with
//! no cap (the dock's `_CanBuyFighter` test @0x87fbb-0x87fe3 is for slot 0,
//! where no fighter sits), an empty bay restarting its reload; the
//! player's round goes to the first ammunition outfit of that bay by
//! ascending outfit ID. A fighter with no bay writes nothing (@0x80384):
//! it stays out at its carrier, steering and thrusting at it unpulled
//! (@0x88111), and retries, or is lost by the other reading of
//! [`RuleKey::FighterNoBay`](crate::RuleKey::FighterNoBay).
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
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::speed_cap::adjusted_accel;

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

/// What a fighter's shield and armour capacities and its shield recharge
/// rate are scaled by (1.333, a double @0xdd0a0): `_ShipShieldCapacity`,
/// `_ShipArmorCapacity` and `_ShipShieldRechargeRate` for AI type 5
/// (@0x2a91-0x2aa7, @0x2cb9-0x2cd2, @0x2b96-0x2bac).
pub const FIGHTER_CAPACITY: f64 = 1.333;

/// A fighter's shield or armour capacity, or its shield recharge rate, of
/// its class's `amount`, by `reading` of the rulebook's
/// [`RuleKey::FighterCapacity`](crate::RuleKey::FighterCapacity): by the
/// engine, `amount` × [`FIGHTER_CAPACITY`], taken in doubles and stored
/// as a float, as the original's products are; by the other reading,
/// `amount` itself.
#[must_use]
pub fn fighter_capacity(amount: f32, reading: RuleSource) -> f32 {
    match reading {
        RuleSource::Engine => (f64::from(amount) * FIGHTER_CAPACITY) as f32,
        RuleSource::Bible => amount,
    }
}

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
/// moving at `carrier`, pushed `push` pixels a tick, of `top_speed`:
/// `_AdjustedAccel` (@0x2ae4b-0x2af43, [`adjusted_accel`]).
#[must_use]
pub fn launch_velocity(carrier: Vec2, heading: f32, push: f32, top_speed: f32) -> Vec2 {
    adjusted_accel(carrier, heading, push, top_speed)
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
    fn a_launch_is_adjusted_accel() {
        for (carrier, heading, push, top) in [
            (Vec2::ZERO, 0.0, 4.0, 10.0),
            (Vec2::new(8.0, 0.0), 45.0, 4.0, 10.0),
            (Vec2::new(0.0, 30.0), 0.0, -4.0, 10.0),
        ] {
            assert_eq!(
                launch_velocity(carrier, heading, push, top),
                adjusted_accel(carrier, heading, push, top)
            );
        }
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
    fn a_fighter_holds_1_333_times_by_the_engine_taken_in_doubles() {
        // 30 × 1.333 in doubles, stored as a float, is the float nearest
        // 39.99, where a float product falls a float short of it.
        assert_eq!(fighter_capacity(30.0, RuleSource::Engine), 39.99);
        assert_ne!(fighter_capacity(30.0, RuleSource::Engine), 30.0 * 1.333_f32);
        assert_eq!(fighter_capacity(1000.0, RuleSource::Engine), 1333.0);
        assert_eq!(fighter_capacity(0.0, RuleSource::Engine), 0.0);
        assert_eq!(fighter_capacity(30.0, RuleSource::Bible), 30.0);
        assert_eq!(FIGHTER_CAPACITY, 1.333);
    }

    #[test]
    fn the_strings_are_str_2002s() {
        assert_eq!(ABANDONED_ONE, "fighter abandoned");
        assert_eq!(ABANDONED_MANY, "fighters abandoned");
        assert_eq!((FIGHTER_BAY, FIRST_SHIP), (99, 128));
    }
}
