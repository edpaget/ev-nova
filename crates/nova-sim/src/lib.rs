//! EV Nova's game state and rules, with no rendering, windowing or audio:
//! the simulation takes the player's controls and advances the game state
//! one fixed step at a time.
//!
//! - [`blink`]: how bright a ship's running lights are at a tick,
//!   [`lights_level`], from its `shän`'s [`Blink`] fields, with random
//!   mode rolling on the [`BlinkChance`] port.
//! - [`catalog`]: the [`PilotCatalog`] port, what a flight session starts
//!   from: the first `chär`, its ship's fields and default items, the
//!   `oütf`s and `shïp`s, which systems exist, the star map, and the commodities,
//!   `jünk` and `öops` the exchange trades and is moved by.
//! - [`chance`]: the [`Chance`] port, whether a percentage chance fires,
//!   which the day's planetary events roll on.
//! - [`clock`]: the fixed-step clock, Nova's 1/30 s tick, which turns
//!   display frames of any length into whole simulation steps.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//! - `fixture` (with the `fixture` feature, and in this crate's tests):
//!   `MemoryPilots`, a pilot store in memory.
//! - [`date`]: the in-game [`GameDate`], a Gregorian day that advances
//!   a day for each day a jump takes, and [`date::date_text`], how it is
//!   displayed.
//! - [`flight`]: one tick of a ship's Newtonian flight, [`step`], under
//!   the player's [`Controls`].
//! - [`fuel`]: how much fuel a ship regenerates each tick, from its
//!   `shïp` and its outfits' fuel scoops.
//! - [`gate`]: hypergates and wormholes: which a stellar is
//!   ([`GateKind`]), where each leads, and how a ship comes out of one,
//!   under the [`GateArrivalRule`] and [`WormholeRule`].
//! - [`geometry`]: the simulation's own [`Vec2`], in pixels with y
//!   growing down.
//! - [`glow`]: how bright a ship's engine glow is: its base level, which
//!   thrust ramps up and coasting down ([`ramp_glow`]), and the level
//!   drawn at a tick, flickering on the [`BlinkChance`] port
//!   ([`glow_level`]).
//! - [`handling`]: a ship's [`Handling`], its speed, acceleration and turn
//!   rate in pixels and ticks, and the [`ShipFields`] its stats start from.
//! - [`hyperspace`]: the [`StarMap`] of hyperlinks and the routes along
//!   it, whether the ship can jump ([`check_jump`]), and where it arrives.
//! - [`landing`]: what the land key does, [`land_or_select`]: request
//!   clearance, then land (or, over a hypergate or wormhole, enter it); whether the ship can land, [`check_landing`],
//!   each [`LandingRefusal`] in the order it applies; and the
//!   [`Service`]s a stellar's flags offer.
//! - [`market`]: the commodity exchange: what a stellar trades and at
//!   what price ([`Market`]), cargo space, buying and selling
//!   ([`Order`]), and the planetary events that move prices.
//! - [`message`]: the [`SimMessage`] events a session raises for the
//!   message line, such as arriving from a jump or through a gate; the
//!   view words them.
//! - [`navigation`]: the navigation target Tab selects, [`next_stellar`]:
//!   the system's stellars in their `NavDef` order, wrapping.
//! - [`outfitter`]: the outfitter: which outfits a stellar lists and sells
//!   ([`Outfitter`]), their price and mass, the ship's free mass, and
//!   buying and selling one at a time ([`OutfitOrder`]).
//! - [`pilot`]: the [`Pilot`], everything about the player a save keeps:
//!   ship, location, date, cash, reserves, course, explored systems,
//!   legal records, cargo, the events under way and the outfits owned,
//!   starting from the first `chär`.
//! - [`pre_jump`]: what the ship does between the jump key and the jump:
//!   it brakes until it is [`slow_enough`](pre_jump::slow_enough), then
//!   turns to the bearing of the next system.
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
//!   capacities, fuel regeneration and cargo space come from.
//! - [`wares`]: the rules the outfitter and the shipyard share: tech
//!   levels, `Require` and `Contribute`, the hiding flags, the hide-higher
//!   sweep and the rows' order.

pub mod blink;
pub mod catalog;
pub mod chance;
pub mod clock;
pub mod data;
pub mod date;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod flight;
pub mod fuel;
pub mod gate;
pub mod geometry;
pub mod glow;
pub mod handling;
pub mod hyperspace;
pub mod landing;
pub mod market;
pub mod message;
pub mod navigation;
pub mod outfitter;
pub mod pilot;
pub mod pre_jump;
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
pub mod wares;

pub use blink::{Blink, BlinkChance, HashedRolls, lights_level};
pub use catalog::{
    CharacterStart, CommodityStrings, DateAffixes, DisasterId, DisasterRecord, GateSite, GovtId,
    JunkId, JunkRecord, LandingSite, OutfitId, OutfitRecord, PilotCatalog, ShipId, ShipRecord,
    SoundId, StarSystem, StartDate, StartError, StellarId, SystemId,
};
pub use chance::{Chance, NeverFires};
pub use clock::{FixedStep, MAX_STEPS, Steps, TICK, TICKS_PER_SECOND};
pub use date::GameDate;
pub use flight::{Controls, ShipState, Turn, step};
pub use fuel::{OutfitMod, fuel_regen_per_tick};
pub use gate::{GateArrivalRule, GateKind, GateRefusal, WormholeRule};
pub use geometry::Vec2;
pub use glow::{GLOW_CRUISE, glow_level, ramp_glow};
pub use handling::{Handling, ShipFields};
pub use hyperspace::{
    HyperSelectRule, HyperlinkRule, JumpRefusal, MultiJumpRule, RouteError, StarMap, check_jump,
    hops_per_jump, next_hyper_destination,
};
pub use landing::{
    Clearance, LandOutcome, LandingRefusal, Service, check_landing, is_landable, land_or_select,
    landing_radius, nearest_landable, services,
};
pub use market::{Direction, Good, Lot, Market, MarketRow, Order, TradeRefusal};
pub use message::SimMessage;
pub use navigation::{next_after, next_stellar};
pub use outfitter::{OutfitOrder, OutfitRefusal, OutfitRow, Outfitter};
pub use pilot::Pilot;
pub use recharge::{FUEL_PRICE_PER_UNIT, RechargeRefusal, sells_fuel};
pub use reserves::{Gauge, Reserves};
pub use save::SaveError;
pub use saves::{PilotKeeper, PilotStore, pilot_key};
pub use session::{LandPress, Session};
pub use shipyard::{ShipPurchase, ShipRefusal, ShipRow, ShipSpecs, Shipyard};
pub use sound::SimSound;
pub use stats::ShipStats;
