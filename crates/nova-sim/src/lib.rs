//! EV Nova's game state and rules, with no rendering, windowing or audio:
//! the simulation takes the player's controls and advances the game state
//! one fixed step at a time.
//!
//! - [`ai`]: NPC decisions: the [`Behaviour`] port that sets each NPC's
//!   [`Goal`], and [`Peaceful`], Nova's default: traders land, others jump
//!   out, escorts follow their lead.
//! - [`catalog`]: the [`PilotCatalog`] port, what a flight session starts
//!   from: the first `chär`, its ship's fields and default items, the
//!   `oütf`s and `shïp`s, which systems exist, the star map, and the commodities,
//!   `jünk` and `öops` the exchange trades and is moved by; the
//!   [`TrafficCatalog`] port, the `sÿst` traffic, `düde`s and `flët`s NPC
//!   traffic is spawned from; and the [`CombatCatalog`] port, the `wëap`s
//!   and each `shïp`'s combat fields ships fight with.
//! - [`chance`]: the [`Chance`] port, whether a percentage chance fires,
//!   which the day's planetary events roll on, and uniform draws, which
//!   the NPC traffic rolls on.
//! - [`clock`]: the fixed-step clock, Nova's 1/30 s tick, which turns
//!   display frames of any length into whole simulation steps.
//! - [`data`]: the catalog ports' adapters over `nova_data`'s `GameData`.
//! - `fixture` (with the `fixture` feature, and in this crate's tests):
//!   `MemoryPilots`, a pilot store in memory.
//! - [`date`]: the in-game [`GameDate`], a Gregorian day that advances
//!   one day a jump.
//! - [`flight`]: one tick of a ship's Newtonian flight, [`step`], under
//!   the player's [`Controls`].
//! - [`fuel`]: how much fuel a ship regenerates each tick, from its
//!   `shïp` and its outfits' fuel scoops.
//! - [`geometry`]: the simulation's own [`Vec2`], in pixels with y
//!   growing down.
//! - [`handling`]: a ship's [`Handling`], its speed, acceleration and turn
//!   rate in pixels and ticks, and the [`ShipFields`] its stats start from.
//! - [`hyperspace`]: the [`StarMap`] of hyperlinks and the routes along
//!   it, whether the ship can jump ([`check_jump`]), and where it arrives.
//! - [`landing`]: whether the ship can land, [`check_landing`], each
//!   [`LandingRefusal`] in the order it applies, and the [`Service`]s a
//!   stellar's flags offer.
//! - [`market`]: the commodity exchange: what a stellar trades and at
//!   what price ([`Market`]), cargo space, buying and selling
//!   ([`Order`]), and the planetary events that move prices.
//! - [`outfitter`]: the outfitter: which outfits a stellar lists and sells
//!   ([`Outfitter`]), their price and mass, the ship's free mass, and
//!   buying and selling one at a time ([`OutfitOrder`]).
//! - [`pilot`]: the [`Pilot`], everything about the player a save keeps:
//!   ship, location, date, cash, reserves, course, explored systems,
//!   legal records, cargo, the events under way and the outfits owned,
//!   starting from the first `chär`.
//! - [`recharge`]: recharging in the spaceport: which stellars sell
//!   fuel ([`sells_fuel`]), what filling the tank costs, and each
//!   [`RechargeRefusal`].
//! - [`reserves`]: a ship's [`Reserves`], its shield, armour and fuel
//!   [`Gauge`]s, full when it starts.
//! - [`save`]: the save schema: a pilot as versioned JSON and back,
//!   upgrading older saves.
//! - [`saves`]: the [`PilotStore`] port pilots are saved through, the key
//!   each is saved under, and the [`PilotKeeper`] that saves, lists and
//!   opens them.
//! - [`session`]: a flight [`Session`], a pilot's ship flying
//!   from its starting system, landing, and jumping along a plotted course.
//! - [`shipyard`]: the shipyard: which ships a stellar lists and sells
//!   ([`Shipyard`]), their price, what the ship flown trades in for, and
//!   buying a new one ([`ShipPurchase`]): which outfits carry over, the
//!   cargo kept and the default items fitted.
//! - [`sound`]: the [`SimSound`] events a session emits as it thrusts,
//!   lands, takes off and jumps; the audio side decides what they play.
//! - [`stats`]: a ship's computed [`ShipStats`], from its `shïp`'s fields
//!   and the outfits it carries: the one place its handling, reserve
//!   capacities, fuel, shield and armour regeneration and cargo space come
//!   from.
//! - [`traffic`]: NPC [`Traffic`]: the ships spawned from a system's
//!   `düde`s and fleets on arrival and over time, each [`Npc`] flown by an
//!   autopilot with the player's flight physics and stats.
//! - [`wares`]: the rules the outfitter and the shipyard share: tech
//!   levels, `Require` and `Contribute`, the hiding flags, the hide-higher
//!   sweep and the rows' order.

pub mod ai;
pub mod catalog;
pub mod chance;
pub mod clock;
pub mod data;
pub mod date;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod flight;
pub mod fuel;
pub mod geometry;
pub mod handling;
pub mod hyperspace;
pub mod landing;
pub mod market;
pub mod outfitter;
pub mod pilot;
pub mod recharge;
pub mod reserves;
pub mod save;
pub mod saves;
pub mod session;
pub mod shipyard;
pub mod sound;
pub mod stats;
#[cfg(test)]
mod testkit;
pub mod traffic;
pub mod wares;

pub use ai::{Behaviour, Goal, Peaceful, Surroundings};
pub use catalog::{
    BoomId, CharacterStart, CombatCatalog, CommodityStrings, DisasterId, DisasterRecord, DudeId,
    DudeRecord, EscortRecord, FleetId, FleetRecord, GovtId, HullRecord, JunkId, JunkRecord,
    LandingSite, OutfitId, OutfitRecord, PilotCatalog, ShipId, ShipRecord, SoundId, StarSystem,
    StartDate, StartError, StellarId, StockWeapon, SystemId, SystemTraffic, TrafficCatalog,
    WeaponId, WeaponRecord,
};
pub use chance::{Chance, NeverFires};
pub use clock::{FixedStep, MAX_STEPS, Steps, TICK, TICKS_PER_SECOND};
pub use date::GameDate;
pub use flight::{Controls, ShipState, Turn, step};
pub use fuel::{OutfitMod, fuel_regen_per_tick};
pub use geometry::Vec2;
pub use handling::{Handling, ShipFields};
pub use hyperspace::{JumpRefusal, RouteError, StarMap, check_jump};
pub use landing::{LandingRefusal, Service, check_landing, landing_radius, services};
pub use market::{Direction, Good, Lot, Market, MarketRow, Order, TradeRefusal};
pub use outfitter::{OutfitOrder, OutfitRefusal, OutfitRow, Outfitter};
pub use pilot::Pilot;
pub use recharge::{FUEL_PRICE_PER_UNIT, RechargeRefusal, sells_fuel};
pub use reserves::{Gauge, Reserves};
pub use save::SaveError;
pub use saves::{PilotKeeper, PilotStore, pilot_key};
pub use session::Session;
pub use shipyard::{ShipPurchase, ShipRefusal, ShipRow, ShipSpecs, Shipyard};
pub use sound::SimSound;
pub use stats::ShipStats;
pub use traffic::Traffic;
pub use traffic::npc::{AiType, Npc, NpcId};
