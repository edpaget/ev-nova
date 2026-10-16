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
//! - [`asteroid`]: a system's asteroid field: the 16 slots
//!   ([`Asteroids`](asteroid::Asteroids)), how an asteroid spawns at the
//!   field's edge or in place, how it drifts and spins, and when it is
//!   culled.
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
//! - [`brackets`]: the corner brackets round the nav target and the ship
//!   target: their [`FlyIn`](brackets::FlyIn) from a selection and a ship
//!   target's [`Standing`](brackets::Standing).
//! - [`cargo_box`]: the cargo boxes a jettison drops behind the player
//!   and its trader escorts: how many ([`box_count`](cargo_box::box_count)),
//!   where each starts, and how it drifts, spins and fades
//!   ([`CargoBoxes`](cargo_box::CargoBoxes)).
//! - [`catalog`]: the [`PilotCatalog`] port, what a flight session starts
//!   from: the first `chär`, its ship's fields and default items, the
//!   `oütf`s and `shïp`s, which systems exist, the star map, and the commodities,
//!   `jünk` and `öops` the exchange trades and is moved by, the
//!   `crön`s the day's cron pass runs and the `ränk`s the pilot can hold; the
//!   [`TrafficCatalog`] port, the `sÿst` traffic, `düde`s and `flët`s NPC
//!   traffic is spawned from; the [`CombatCatalog`] port, the `wëap`s,
//!   each `shïp`'s combat fields and the `gövt`s ships fight by; the
//!   [`AsteroidCatalog`] port, the `röid`s and each `sÿst`'s asteroid
//!   field; and the [`CommCatalog`] port, the string lists a hailed ship's words come
//!   from.
//! - [`chance`]: the [`Chance`] port, whether a percentage chance fires,
//!   which the day's planetary events roll on, and uniform draws, which
//!   the NPC traffic rolls on.
//! - [`clearance`]: the dock request the land key makes
//!   ([`DockRequest`]), its counter, and the dock approach that counts it
//!   on every frame and greets the pilot ([`approach`]).
//! - [`control`]: the control bits missions and storylines script with,
//!   held as a [`ControlBitSet`] indexed by [`Bit`]; a record's parsed
//!   control-bit [`Test`], and evaluating one against the [`PilotFacts`]
//!   it reads ([`control::holds`]), through the [`ControlBits`] port with
//!   Nova's [`NovaBits`]; and running a set expression
//!   ([`control::execute`]), its operators beyond the bit writes handed to
//!   the caller's dispatch (the session's: `Session::apply_op`, one match
//!   over the set operators).
//! - [`cron`]: the `crön`s the day's second participant runs: whether an
//!   inactive one starts ([`cron::eligibility`], with its disputed
//!   `Random` and date range), the daily pass over their holdoffs and
//!   durations, each one's [`CronState`](cron::CronState), and whose
//!   `Contribute` bits count ([`cron::contributes`]).
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
//!   `MemoryPilots`, a pilot store in memory, and `StoryWalker`, which
//!   walks a session through a mission chain from a script.
//! - [`date`]: the in-game [`GameDate`], a Gregorian day that advances
//!   a day for each day a jump takes, and [`date::date_text`], how it is
//!   displayed.
//! - [`escape`]: the player's escape: the escape pod and auto-eject
//!   outfits, the escape ship a fighter bay offers, when auto-eject arms,
//!   and where the escape pod is taken.
//! - [`escort`]: the player's escorts: their [`EscortClass`], the
//!   [`EscortOrder`] each follows, the [`EscortCommand`]s the player gives
//!   an [`EscortGroup`] and what one changed ([`Commanded`]), the escort
//!   menu's [`ClassRow`]s, an escort NPC's [`EscortDuty`], the formation
//!   it keeps beside the player, and how it scores the threats to the
//!   player.
//! - [`exploration`]: which systems a map outfit explores
//!   ([`exploration::map_reveals`]) and which are inhabited.
//! - [`flight`]: one tick of a ship's Newtonian flight, [`step`], under
//!   the player's [`Controls`].
//! - [`fuel`]: how much fuel a ship regenerates each tick, from its
//!   `shïp` and its outfits' fuel scoops.
//! - [`gate`]: hypergates and wormholes: which a stellar is
//!   ([`GateKind`]), where each leads, and how a ship comes out of one,
//!   under the [`GateArrivalRule`] and [`WormholeRule`].
//! - [`gate_sprite`]: which frame a hypergate's sprite shows, opening
//!   while a ship comes out of it and closing once none does.
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
//!   the comm dialog shows of a ship the player leads ([`EscortStatus`]),
//!   a hired escort's daily pay among it.
//! - [`glow`]: how bright a ship's engine glow is: its base level, which
//!   thrust ramps up and coasting down ([`ramp_glow`]), and the level
//!   drawn at a tick, flickering on the [`BlinkChance`] port
//!   ([`glow_level`]).
//! - [`handling`]: a ship's [`Handling`], its speed, acceleration and turn
//!   rate in pixels and ticks, and the [`ShipFields`] its stats start from.
//! - [`hire`]: hiring escorts in the bar: which ships are for hire today
//!   ([`HireList`], [`HireRow`], [`HireRefusal`]), the [`HireTerms`] port
//!   with Nova's [`NovaHire`] fee and daily wage, [`price_flux`] (the
//!   original's price flux under the ranks' multiplier, which the
//!   shipyard shares), the [`ControlBits`] port a ship's `Availability` goes through, what a
//!   hire did
//!   ([`Hired`]), and the escorts who defect unpaid ([`PayNote`]).
//! - [`hyperspace`]: the [`StarMap`] of hyperlinks and the routes along
//!   it, whether the ship can jump ([`check_jump`]), and where it arrives.
//! - [`landing`]: what the land key does, [`land_or_select`]: request
//!   clearance, then, once the dock approach has cleared the request,
//!   land (or, over a hypergate or wormhole, enter it);
//!   whether traffic control clears the pilot, [`cleared`]; whether the ship
//!   can land, [`check_landing`],
//!   each [`LandingRefusal`] in the order it applies; and the
//!   [`Service`]s a stellar's flags offer.
//! - [`legal`]: the player's legal record, kept with each government and
//!   in each system: the [`LegalCode`] port that says what a [`Crime`]
//!   against a ship does to it, with each government and along its walk
//!   through the [`CrimeMap`] from system to system, Nova's [`NovaLaw`],
//!   [`legal::convict`], which applies it, and
//!   [`legal::systems_from_govts`], which derives the records in each
//!   system, and [`legal::spread_statuses`], which spreads a new pilot's
//!   `chär` statuses.
//! - [`market`]: the commodity exchange: what a stellar trades and at
//!   what price ([`Market`]), cargo space, buying and selling
//!   ([`Order`]), and the planetary events that move prices.
//! - [`message`]: the [`SimMessage`] events a session raises for the
//!   message line, such as arriving from a jump or through a gate; the
//!   view words them.
//! - [`mission`]: mission offers: which missions a docked stellar offers
//!   at its mission computer and outside it ([`mission::availability`],
//!   each field a plain [`Check`](rules::Check) refusing with a
//!   [`mission::Refusal`]), the stellars a mission's codes choose
//!   ([`mission::select_stellar`]), each [`mission::Offer`]'s resolved
//!   values ([`mission::resolve`]), and the per-visit [`mission::Offers`]
//!   with the roll table `AvailRandom` reads.
//! - [`navigation`]: how the player selects the stellar navigation
//!   target: by its `NavDef` slot, the nearest, or a click
//!   ([`stellar_under`]), as the original does.
//! - [`news`]: the bar's news, a commercial line and a story ([`News`]),
//!   and which running `crön`s have a story for the stellar docked at
//!   ([`news::candidates`], with the disputed [`news::news_govt_match`]),
//!   and the `öops` price story ([`news::oops_story`]), drawn by the
//!   session as the bar opens.
//! - [`order_loop`]: an order of a count run one pass at a time: each
//!   pass handed its [`order_loop::Pass`], the buy loop
//!   ([`order_loop::run`]) stopping at the first refusal and the sell
//!   loop ([`order_loop::run_on`]) running on past it, with the
//!   [`order_loop::OrderRun`] keeping the first refusal and its pass.
//! - [`outfit_effects`]: what granting an outfit does: a map explores, a
//!   clean-record outfit cleans the legal record and a paint paints the
//!   ship ([`Rgb15`]) instead of being added
//!   ([`outfit_effects::GrantEffect`]), and the disputed [`OutfitRules`].
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
//!   the fleet of [`Escort`]s, the control bits and the player's
//!   [`Gender`] and its ship's paint and name, starting from the first
//!   `chär`.
//! - [`player_fate`]: the player's lifecycle, a [`PlayerFate`] read off
//!   its [`Condition`], and each fate's [`FateRow`]: how much of the ship
//!   is [`Drawn`].
//! - [`pre_jump`]: what the ship does between the jump key and the jump:
//!   it brakes until it is [`slow_enough`](pre_jump::slow_enough), then
//!   turns to the bearing of the next system.
//! - [`rank`]: the ranks, as `_ActivateRank` and `_DeactivateRank` change
//!   them, their salaries, `Contribute` bits and names in the text tags.
//! - [`rating`]: the player's combat rating: what a kill adds
//!   ([`rating::credit`]) and which kills count ([`rating::counts`]).
//! - [`recharge`]: recharging in the spaceport: which stellars sell
//!   fuel ([`sells_fuel`]), what filling the tank costs, and each
//!   [`RechargeRefusal`].
//! - [`red_alert`]: the Red Alert: when a threat to the player sounds it
//!   ([`RedAlert::check`], every [`ALERT_CHECK_FRAMES`]) and the icon's
//!   blink ([`AlertIcon`]).
//! - [`reserves`]: a ship's [`Reserves`], its shield, armour and fuel
//!   [`Gauge`]s, full when it starts.
//! - [`rulebook`]: the rules where the Nova Bible and the original engine
//!   disagree: each follows a [`RuleSource`], the engine's by default, as
//!   the [`Rulebook`] says, by its [`RuleKey`].
//! - [`rules`]: the rules core over any context: ordered
//!   [`Checks`](rules::Checks) refusing with a reason, [`Value`](rules::Value)s
//!   giving data, each plain, disputed (its reading chosen by the
//!   [`Rulebook`]) or drawing on the chance in rule order, and each
//!   described by a [`Descriptor`](rules::Descriptor) tools list without
//!   running it.
//! - [`save`]: the save schema: a pilot as versioned JSON and back,
//!   upgrading older saves.
//! - [`saves`]: the [`PilotStore`] port pilots are saved through, the key
//!   each is saved under, and the [`PilotKeeper`] that saves, lists and
//!   opens them.
//! - [`scan`]: a warship's or an interceptor's scan of the player: whether
//!   it goes on ([`scan::goes_on`]) and the illegal `jünk` or outfit it
//!   finds after the mission cargo ([`scan::contraband`]).
//! - [`session`]: a flight [`Session`], a pilot's ship flying
//!   from its starting system, landing, jumping along a plotted course,
//!   fighting, boarding and capturing ships, hailing them
//!   ([`HailView`], [`HailRefusal`], [`CommNote`]), flying and
//!   commanding its escorts, launching and docking the fighters of its
//!   bays and the NPC carriers' ([`Sortie`]), hiring escorts in the
//!   bar and paying their wages, and the mission offers rolled as it lands
//!   ([`Session::mission_offers`]).
//! - [`ship_change`]: changing the player's ship outside the shipyard,
//!   as the `C`, `E` and `H` set operators do: which outfits carry over.
//! - [`ship_layer`]: the layers ships are drawn in ([`ShipLayer`]): the
//!   disabled at the bottom, then the ships the player leads, then the
//!   rest with the player.
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
//! - [`story`]: the story runtime's design, with the original's event
//!   sources and orderings, and the pure types it names: the
//!   [`StoryEvent`](story::StoryEvent)s, the
//!   [`Subscription`](story::Subscription)s consumers register and the
//!   order they run in ([`dispatch_order`](story::dispatch_order)), and
//!   the [`GalaxyOverlay`](story::GalaxyOverlay) play changes over the
//!   static galaxy, which every reader sees through the
//!   [`OverlaidGalaxy`](story::OverlaidGalaxy).
//! - [`target_panel`]: what the status bar's target panel says of the
//!   target at its bottom: its [`TargetStatus`] and its [`TargetTag`].
//! - [`targeting`]: which NPC the player's target command picks
//!   ([`TargetPick`]): the nearest, the nearest threat, the next in turn,
//!   or the next of the player's escorts.
//! - [`text_parse`]: the conditional text (`{G "he" "she"}`, `{bnnn …}`,
//!   `{Pnnn …}`) every loaded text is resolved by, as the original's
//!   `_ParseTextDescription` ([`text_parse::conditional_text`]).
//! - [`text_tags`]: the one substitution path for the text tags, as the
//!   original's `_MungeBriefing`: a [`TagContext`] of the pilot, the
//!   person, the ranks and the mission's resolved [`MissionValues`], its
//!   names read through the [`TagNames`] port ([`text_tags::expand`]).
//! - [`top_speed`]: a ship's top speed in flight, the original's
//!   `_ShipMaxSpeed` branch by branch: the 1.5 the player and the ships it
//!   leads fly at without Strict Play ([`top_speed::led_speed`]).
//! - [`traffic`]: NPC [`Traffic`]: the ships spawned from a system's
//!   `düde`s and fleets on arrival and over time, each [`Npc`] flown by an
//!   autopilot with the player's flight physics and stats.
//! - [`tutorial`]: the new pilot's first-flight [`Tutorial`]: its hints
//!   ([`TutorialHint`]), the take-off greeting and the newbie-only "No
//!   destination".
//! - [`wares`]: the rules the outfitter and the shipyard share: tech
//!   levels, `Require` and `Contribute`, the hiding flags, the hide-higher
//!   sweep and the rows' order.
//! - [`warp`]: the player's warp clock: the warp factor, the warp sound's
//!   duration, and when the warp's lead-in ends and scans are held.

pub mod ai;
pub mod asteroid;
pub mod bay;
pub mod blink;
pub mod board;
pub mod brackets;
pub mod buoy;
pub mod cargo_box;
pub mod catalog;
pub mod chance;
pub mod clearance;
pub mod cloak;
pub mod clock;
pub mod combat;
pub mod control;
pub mod course;
pub mod cron;
pub mod data;
pub mod date;
pub mod escape;
pub mod escort;
pub mod exploration;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod flight;
pub mod fuel;
pub mod gate;
pub mod gate_sprite;
pub mod geometry;
pub mod glow;
pub mod govt;
pub mod grant;
pub mod hail;
pub mod handling;
pub mod hire;
pub mod hover;
pub mod hyperspace;
pub mod iff;
pub mod landing;
pub mod legal;
pub mod market;
pub mod message;
pub mod mission;
pub mod navigation;
pub mod nebula;
pub mod news;
pub mod order_loop;
pub mod outfit_effects;
pub mod outfitter;
pub mod person;
pub mod pilot;
pub mod player_fate;
pub mod player_info;
pub mod pre_jump;
pub mod rank;
pub mod rating;
pub mod recharge;
pub mod red_alert;
pub mod reserves;
pub mod rulebook;
pub mod rules;
pub mod sabotage;
pub mod save;
pub mod saves;
pub mod scan;
pub mod scoop;
pub mod self_repair;
pub mod session;
pub mod ship_change;
pub mod ship_layer;
pub mod shipyard;
pub mod sound;
pub mod stats;
pub mod status_cargo;
pub mod story;
pub mod strings;
pub mod target_panel;
pub mod targeting;
#[cfg(test)]
mod testkit;
pub mod text_parse;
pub mod text_tags;
pub mod top_speed;
pub mod traffic;
pub mod trig;
pub mod tutorial;
pub mod wares;
pub mod warp;

pub use ai::{
    Behaviour, BraveTrader, CarriedAi, EscortAi, Goal, Interceptor, NovaAi, Peaceful, PlayerSide,
    Reaction, Surroundings, Warship, WimpyTrader,
};
pub use bay::{Carrier, FighterNote, capacity, dock_window};
pub use blink::{Blink, BlinkChance, HashedRolls, lights_level};
pub use board::{
    Assigned, Assignment, BoardRefusal, Boarding, BoardingRule, MissionBoardLine, NovaBoarding,
    Plunder, PlunderView, Take, Taken,
};
pub use catalog::{
    AsteroidCatalog, AsteroidFieldRecord, AsteroidTypeRecord, BOX_GRAPHICS, BoomId, CharacterStart,
    CombatCatalog, CommCatalog, CommodityStrings, CronId, CronRecord, DateAffixes, DescMovie,
    DisasterId, DisasterRecord, DudeId, DudeRecord, EscortRecord, FleetId, FleetRecord, GateSite,
    GovtId, GovtRecord, HullRecord, JunkId, JunkRecord, LandingSite, ListedStellar, MissionId,
    MissionRecord, MissionShip, MissionTexts, NebulaId, NebulaRecord, OutfitId, OutfitRecord,
    Penalties, PersonId, PersonRecord, PersonWeapon, PilotCatalog, PlaceNames, RankId, RankRecord,
    ShipId, ShipRecord, SoundId, StarSystem, StartDate, StartError, StellarId, StellarLife,
    StockWeapon, StringPatch, SystemId, SystemTraffic, TextGraphics, TextMovies, TrafficCatalog,
    WeaponId, WeaponRecord,
};
pub use chance::{Chance, NeverFires};
pub use clearance::{Approach, DockKind, DockRequest, approach, approach_clears};
pub use clock::{FixedStep, MAX_STEPS, Steps, TICK, TICKS_PER_SECOND};
pub use combat::armament::{Armament, Trigger};
pub use combat::defence::{Allegiance, PointDefenceRule};
pub use combat::hull::{Condition, DisableRule, HullSpec, NovaDisable};
pub use combat::report::SimDiagnostic;
pub use combat::{CombatEvent, Downed, GamePass, Rules, ShipRef, Sortie, Strike};
pub use control::{
    Bit, BitStore, BitWrite, ControlBitSet, ControlBits, NovaBits, PilotFacts, Script, ScriptNote,
    SetExpr, SetOp, SetOpKind, Test, TestExpr,
};
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
pub use hire::{HireList, HireRefusal, HireRow, HireTerms, Hired, NovaHire, PayNote, price_flux};
pub use hover::{DrawnBox, DrawnShips, Hover, hover_gate};
pub use hyperspace::{
    CoLocated, HyperSelectRule, HyperlinkRule, JumpReadiness, JumpRefusal, JumpZoneRule,
    MultiJumpRule, Places, RouteError, StarMap, check_jump, hops_per_jump, jump_zone,
    next_hyper_destination,
};
pub use iff::{RadarGear, ShipIff, StellarIff};
pub use landing::{
    DockPress, DockSaid, LandOutcome, LandingReach, LandingRefusal, NO_FRAME_REACH,
    OUT_OF_RANGE_FRAME_WIDTH, REACH_PER_WIDTH, Service, Standing, StellarShown, TypeWidth,
    UNLOADED_TYPE_WIDTH, check_landing, cleared, is_landable, land_or_select, landing_radius,
    landing_reach, nearest_stellar, nearest_stellar_station, services, stellar_width,
};
pub use legal::{Crime, CrimeMap, LegalCode, NovaLaw};
pub use market::{Direction, Good, Lot, Market, MarketRow, Order, TradeRefusal};
pub use message::SimMessage;
pub use navigation::{NavMode, next_after, stellar_under};
pub use news::News;
pub use outfit_effects::{OutfitRules, Rgb15};
pub use outfitter::{
    Armed, Launcher, LcNames, OpeningLimit, OutfitFacts, OutfitOrder, OutfitRefusal, OutfitRow,
    OutfitRun, OutfitVisit, Outfitter, Raise, Reselection, SaleAlert,
};
pub use person::{
    COMM_QUOTES, ESCAPE_POD, GRUDGE, HAIL_QUOTES, NovaPersons, PersonRoll, PersonRules,
    PersonWorld, QuoteTags, expand_tags,
};
pub use pilot::{Escort, Gender, Pilot};
pub use player_fate::{
    Drawn, Ending, FateRow, HOLD_TICKS, POD_TICKS, PlayerCommand, PlayerFate, Takes,
};
pub use player_info::PlayerInfo;
pub use recharge::{FUEL_PRICE_PER_UNIT, RechargeRefusal, sells_fuel};
pub use red_alert::{
    ALERT_CHECK_FRAMES, ALERT_CICN, ALERT_STEP_FRAMES, ALERT_STEPS, AlertIcon, RedAlert,
    alert_check_due, alert_step_due,
};
pub use reserves::{Gauge, Reserve, Reserves};
pub use rulebook::{RuleKey, RuleSource, Rulebook};
pub use sabotage::{SABOTAGE_BOMB, SABOTAGE_KILL, Sabotage, death_descs, sabotage_flag};
pub use save::SaveError;
pub use saves::{PilotKeeper, PilotStore, pilot_key};
pub use session::{
    AbortError, AcceptError, ComputerRefusal, ComputerRow, Departure, ESCAPE_POD_SHIP, Hailed,
    HookRules, InfoRow, InterfaceLoad, LandPress, MissionComputer, MissionInfo, MissionInfoPress,
    MissionShown, MissionText, MissionTextKind, NotOffered, OfferPrompt, PersonQuote,
    RelocateRefusal, ScriptEffectRules, Session, SessionEvent, Settled, ShipChangeRules,
    StellarPick, TAKEOFF_GRACE_TICKS, VenueOffer,
};
pub use ship_layer::ShipLayer;
pub use shipyard::{
    Rating, ShipFacts, ShipNaming, ShipPurchase, ShipRefusal, ShipRow, ShipSpecs, Shipyard,
    StockLine,
};
pub use sound::{PLAYER_EXPLOSION_HALTS, SimSound};
pub use stats::ShipStats;
pub use status_cargo::{CargoFacts, CommodityRow, MissionCargo, Special, StatusCargo};
pub use target_panel::{StatusFacts, TagFacts, TargetStatus, TargetTag};
pub use targeting::TargetPick;
pub use text_tags::{MissionTags, MissionValues, NoNames, TagContext, TagNames};
pub use traffic::npc::{AiType, Npc, NpcId, NpcPerson};
pub use traffic::{Leave, Traffic, World};
pub use tutorial::{Tutorial, TutorialHint};
pub use wares::{ShownTons, TonUnit};
