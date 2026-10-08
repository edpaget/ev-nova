//! EV Nova's game state and rules, with no rendering, windowing or audio:
//! the simulation takes the player's controls and advances the game state
//! one fixed step at a time.
//!
//! - [`ai`]: NPC decisions: the [`Behaviour`] port that sets each NPC's
//!   [`Goal`] from its [`Surroundings`] and answers hits; [`Peaceful`],
//!   traffic that never fights; and [`NovaAi`], Nova's combat AI, routing
//!   each NPC by its AI type to [`WimpyTrader`], [`BraveTrader`],
//!   [`Warship`] or [`Interceptor`]: whom they attack by governments'
//!   relations and the player's legal record, when they flee, how they
//!   come to each other's help, and which weapon they fire; the
//!   player's escorts to [`EscortAi`], which flies them by their standing
//!   orders; and an NPC carrier's fighters to [`CarriedAi`].
//! - [`bay`]: fighter bays: the fighters a bay holds ([`capacity`]), how
//!   one is launched, the window it docks in ([`dock_window`]) with its
//!   [`Carrier`], and what the flight is told of fighters abandoned
//!   ([`FighterNote`]).
//! - [`board`]: boarding a disabled ship: whether the player can
//!   ([`BoardRefusal`]), the [`Plunder`] rolled for it, the plunder
//!   dialog's [`Take`]s and what each [`Taken`] did, capturing it, and
//!   assigning a ship captured ([`Assignment`]); the [`BoardingRule`]
//!   port with Nova's [`NovaBoarding`]: who repels boarders, the capture
//!   odds and the capture roll, a person's credits and its grant.
//! - [`blink`]: how bright a ship's running lights are at a tick,
//!   [`lights_level`], from its `shän`'s [`Blink`] fields, with random
//!   mode rolling on the [`BlinkChance`] port.
//! - [`catalog`]: the [`PilotCatalog`] port, what a flight session starts
//!   from: the first `chär`, its ship's fields and default items, the
//!   `oütf`s and `shïp`s, which systems exist, the star map, and the commodities,
//!   `jünk` and `öops` the exchange trades and is moved by; the
//!   [`TrafficCatalog`] port, the `sÿst` traffic, `düde`s and `flët`s NPC
//!   traffic is spawned from; the [`CombatCatalog`] port, the `wëap`s,
//!   each `shïp`'s combat fields and the `gövt`s ships fight by; and the
//!   [`CommCatalog`] port, the string lists a hailed ship's words come
//!   from.
//! - [`chance`]: the [`Chance`] port, whether a percentage chance fires,
//!   which the day's planetary events roll on, and uniform draws, which
//!   the NPC traffic rolls on.
//! - [`combat`]: ships fighting: firing their [`Armament`] on a
//!   [`Trigger`] at their target, shots and beams flying and hitting,
//!   homing missiles steering, turrets aiming in their arcs, point defence
//!   shooting missiles down, sub-munitions, damage to shields then armour,
//!   the fight's [`Rules`]: the [`DisableRule`] port with Nova's
//!   [`NovaDisable`] and the [`PointDefenceRule`] port with Nova's
//!   [`Allegiance`], and destruction, reported as [`CombatEvent`]s, with
//!   each weapon feature not done yet reported once as a
//!   [`SimDiagnostic`].
//! - [`clock`]: the fixed-step clock, Nova's 1/30 s tick, which turns
//!   display frames of any length into whole simulation steps.
//! - [`data`]: the catalog ports' adapters over `nova_data`'s `GameData`.
//! - `fixture` (with the `fixture` feature, and in this crate's tests):
//!   `MemoryPilots`, a pilot store in memory.
//! - [`date`]: the in-game [`GameDate`], a Gregorian day that advances
//!   a day for each day a jump takes, and [`date::date_text`], how it is
//!   displayed.
//! - [`escort`]: the player's escorts: their [`EscortClass`], the
//!   [`EscortOrder`] each follows, the [`EscortCommand`]s the player gives
//!   an [`EscortGroup`] and what one changed ([`Commanded`]), the escort
//!   menu's [`ClassRow`]s, an escort NPC's [`EscortDuty`], the formation
//!   it keeps beside the player, and how it scores the threats to the
//!   player.
//! - [`flight`]: one tick of a ship's Newtonian flight, [`step`], under
//!   the player's [`Controls`].
//! - [`fuel`]: how much fuel a ship regenerates each tick, from its
//!   `shïp` and its outfits' fuel scoops.
//! - [`gate`]: hypergates and wormholes: which a stellar is
//!   ([`GateKind`]), where each leads, and how a ship comes out of one,
//!   under the [`GateArrivalRule`] and [`WormholeRule`].
//! - [`geometry`]: the simulation's own [`Vec2`], in pixels with y
//!   growing down.
//! - [`govt`]: the [`Governments`] and their relations: allies, enemies,
//!   xenophobes and the flags that say how their ships behave.
//! - [`grant`]: boarding grants: what a person's record grants
//!   ([`PersonGrant`]), the outfits a grant sees ([`GrantStock`]), and
//!   what it gave ([`Granted`]).
//! - [`hail`]: hailing a ship: whether it likes the player
//!   ([`Attitude`]), the [`Conversation`] a hail rolls (its variant, mood,
//!   price and advice) and haggling over a price ([`Haggle`]), the
//!   [`Reply`] it says, and the [`HailOption`] port the comm dialog lists
//!   ([`HailOptions`]), with Nova's Greetings, Request Assistance and Beg
//!   For Mercy, [`Release`] for the player's escort, and Use As Escort
//!   ([`JoinFleet`]) for a person who may join, which answer each
//!   press with an [`Answer`]: a reply, a [`Deed`] and a price asked
//!   ([`Ask`]); the [`Help`] a ship gives the player once asked; and what
//!   the comm dialog shows of the player's own escort ([`EscortStatus`]),
//!   a hired one's daily pay among it.
//! - [`glow`]: how bright a ship's engine glow is: its base level, which
//!   thrust ramps up and coasting down ([`ramp_glow`]), and the level
//!   drawn at a tick, flickering on the [`BlinkChance`] port
//!   ([`glow_level`]).
//! - [`handling`]: a ship's [`Handling`], its speed, acceleration and turn
//!   rate in pixels and ticks, and the [`ShipFields`] its stats start from.
//! - [`hire`]: hiring escorts in the bar: which ships are for hire today
//!   ([`HireList`], [`HireRow`], [`HireRefusal`]), the [`HireTerms`] port
//!   with Nova's [`NovaHire`] fee and daily wage ([`price_flux`]), the
//!   [`ControlBits`] port a ship's `Availability` goes through
//!   ([`NoControlBits`] until control bits exist), what a hire did
//!   ([`Hired`]), and the escorts who defect unpaid ([`PayNote`]).
//! - [`hyperspace`]: the [`StarMap`] of hyperlinks and the routes along
//!   it, whether the ship can jump ([`check_jump`]), and where it arrives.
//! - [`landing`]: what the land key does, [`land_or_select`]: request
//!   clearance, then land (or, over a hypergate or wormhole, enter it);
//!   whether the ship can land, [`check_landing`],
//!   each [`LandingRefusal`] in the order it applies; and the
//!   [`Service`]s a stellar's flags offer.
//! - [`legal`]: the player's legal record: the [`LegalCode`] port that
//!   says what a [`Crime`] against a ship does to it, Nova's
//!   [`NovaLaw`], and [`legal::convict`], which applies it.
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
//! - [`person`]: persons (`përs`): where they may appear ([`PersonLink`](person::PersonLink)),
//!   how often ([`PersonRules`], Nova's [`NovaPersons`]), the ship each
//!   flies ([`person::fit`]), and when it says its hail quote
//!   ([`person::quote_eligible`], [`expand_tags`]).
//! - [`pilot`]: the [`Pilot`], everything about the player a save keeps:
//!   ship, location, date, cash, reserves, course, explored systems,
//!   legal records, cargo, the events under way, the outfits owned and
//!   the fleet of [`Escort`]s, starting from the first `chär`.
//! - [`pre_jump`]: what the ship does between the jump key and the jump:
//!   it brakes until it is [`slow_enough`](pre_jump::slow_enough), then
//!   turns to the bearing of the next system.
//! - [`recharge`]: recharging in the spaceport: which stellars sell
//!   fuel ([`sells_fuel`]), what filling the tank costs, and each
//!   [`RechargeRefusal`].
//! - [`reserves`]: a ship's [`Reserves`], its shield, armour and fuel
//!   [`Gauge`]s, full when it starts.
//! - [`rulebook`]: the rules where the Nova Bible and the original engine
//!   disagree: each follows a [`RuleSource`], the engine's by default, as
//!   the [`Rulebook`] says, by its [`RuleKey`].
//! - [`save`]: the save schema: a pilot as versioned JSON and back,
//!   upgrading older saves.
//! - [`saves`]: the [`PilotStore`] port pilots are saved through, the key
//!   each is saved under, and the [`PilotKeeper`] that saves, lists and
//!   opens them.
//! - [`session`]: a flight [`Session`], a pilot's ship flying
//!   from its starting system, landing, jumping along a plotted course,
//!   fighting, boarding and capturing ships, hailing them
//!   ([`HailView`], [`HailRefusal`], [`CommNote`]), flying and
//!   commanding its escorts, launching and docking the fighters of its
//!   bays and the NPC carriers' ([`Sortie`]), and hiring escorts in the
//!   bar and paying their wages.
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
//! - [`targeting`]: which NPC the player's target command picks
//!   ([`TargetPick`]): the nearest, the nearest threat, the next in turn,
//!   or the next of the player's escorts.
//! - [`traffic`]: NPC [`Traffic`]: the ships spawned from a system's
//!   `düde`s and fleets on arrival and over time, each [`Npc`] flown by an
//!   autopilot with the player's flight physics and stats.
//! - [`wares`]: the rules the outfitter and the shipyard share: tech
//!   levels, `Require` and `Contribute`, the hiding flags, the hide-higher
//!   sweep and the rows' order.

pub mod ai;
pub mod bay;
pub mod blink;
pub mod board;
pub mod catalog;
pub mod chance;
pub mod clock;
pub mod combat;
pub mod data;
pub mod date;
pub mod escort;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod flight;
pub mod fuel;
pub mod gate;
pub mod geometry;
pub mod glow;
pub mod govt;
pub mod grant;
pub mod hail;
pub mod handling;
pub mod hire;
pub mod hyperspace;
pub mod landing;
pub mod legal;
pub mod market;
pub mod message;
pub mod navigation;
pub mod outfitter;
pub mod person;
pub mod pilot;
pub mod pre_jump;
pub mod recharge;
pub mod reserves;
pub mod rulebook;
pub mod save;
pub mod saves;
pub mod session;
pub mod shipyard;
pub mod sound;
pub mod stats;
pub mod targeting;
#[cfg(test)]
mod testkit;
pub mod traffic;
pub mod wares;

pub use ai::{
    Behaviour, BraveTrader, CarriedAi, EscortAi, Goal, Interceptor, NovaAi, Peaceful, PlayerSide,
    Reaction, Surroundings, Warship, WimpyTrader,
};
pub use bay::{Carrier, FighterNote, capacity, dock_window};
pub use blink::{Blink, BlinkChance, HashedRolls, lights_level};
pub use board::{
    Assigned, Assignment, BoardRefusal, Boarding, BoardingRule, NovaBoarding, Plunder, PlunderView,
    Take, Taken,
};
pub use catalog::{
    BoomId, CharacterStart, CombatCatalog, CommCatalog, CommodityStrings, DateAffixes, DisasterId,
    DisasterRecord, DudeId, DudeRecord, EscortRecord, FleetId, FleetRecord, GateSite, GovtId,
    GovtRecord, HullRecord, JunkId, JunkRecord, LandingSite, MissionShip, OutfitId, OutfitRecord,
    Penalties, PersonId, PersonRecord, PersonWeapon, PilotCatalog, ShipId, ShipRecord, SoundId,
    StarSystem, StartDate, StartError, StellarId, StockWeapon, StringPatch, SystemId,
    SystemTraffic, TrafficCatalog, WeaponId, WeaponRecord,
};
pub use chance::{Chance, NeverFires};
pub use clock::{FixedStep, MAX_STEPS, Steps, TICK, TICKS_PER_SECOND};
pub use combat::armament::{Armament, Trigger};
pub use combat::defence::{Allegiance, PointDefenceRule};
pub use combat::hull::{Condition, DisableRule, HullSpec, NovaDisable};
pub use combat::report::SimDiagnostic;
pub use combat::{CombatEvent, Downed, Rules, ShipRef, Sortie, Strike};
pub use date::GameDate;
pub use escort::{
    ClassRow, Commanded, EscortClass, EscortCommand, EscortDuty, EscortGroup, EscortOrder,
};
pub use flight::{Controls, ShipState, Turn, step};
pub use fuel::{OutfitMod, fuel_regen_per_tick};
pub use gate::{GateArrivalRule, GateKind, GateRefusal, WormholeRule};
pub use geometry::Vec2;
pub use glow::{GLOW_CRUISE, glow_level, ramp_glow};
pub use govt::Governments;
pub use grant::{GrantStock, Granted, PersonGrant};
pub use hail::{
    Answer, Ask, Attitude, CommNote, Conversation, Deed, EscortStatus, Haggle, Hail, HailButton,
    HailOption, HailOptions, HailRefusal, HailView, Help, JoinFleet, Release, Reply,
};
pub use handling::{Handling, ShipFields};
pub use hire::{
    ControlBits, HireList, HireRefusal, HireRow, HireTerms, Hired, NoControlBits, NovaHire,
    PayNote, price_flux,
};
pub use hyperspace::{
    HyperSelectRule, HyperlinkRule, JumpReadiness, JumpRefusal, JumpZoneRule, MultiJumpRule,
    RouteError, StarMap, check_jump, hops_per_jump, jump_zone, next_hyper_destination,
};
pub use landing::{
    Clearance, LandOutcome, LandingRefusal, Service, check_landing, is_landable, land_or_select,
    landing_radius, nearest_landable, services,
};
pub use legal::{Crime, LegalCode, NovaLaw};
pub use market::{Direction, Good, Lot, Market, MarketRow, Order, TradeRefusal};
pub use message::SimMessage;
pub use navigation::{next_after, next_stellar};
pub use outfitter::{OutfitOrder, OutfitRefusal, OutfitRow, Outfitter};
pub use person::{
    COMM_QUOTES, ESCAPE_POD, GRUDGE, HAIL_QUOTES, NovaPersons, PersonRoll, PersonRules,
    PersonWorld, QuoteTags, expand_tags,
};
pub use pilot::{Escort, Pilot};
pub use recharge::{FUEL_PRICE_PER_UNIT, RechargeRefusal, sells_fuel};
pub use reserves::{Gauge, Reserves};
pub use rulebook::{RuleKey, RuleSource, Rulebook};
pub use save::SaveError;
pub use saves::{PilotKeeper, PilotStore, pilot_key};
pub use session::{LandPress, PersonQuote, Session};
pub use shipyard::{ShipNaming, ShipPurchase, ShipRefusal, ShipRow, ShipSpecs, Shipyard};
pub use sound::SimSound;
pub use stats::ShipStats;
pub use targeting::TargetPick;
pub use traffic::npc::{AiType, Npc, NpcId, NpcPerson};
pub use traffic::{Traffic, World};
