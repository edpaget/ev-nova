//! A flight session: a [`Pilot`]'s ship in its system, flown one tick at a
//! time.
//!
//! A session flies a pilot ([`Session::fly`]), a new one from the first
//! `chär` ([`Session::start`]) or a saved one. The ship starts at rest at
//! the system's centre, facing up, or docked at the stellar the pilot last
//! landed on there. The pilot holds everything a save keeps (the ship,
//! system, date, course and reserves among them), and the session changes
//! it as the rules below say; the session itself keeps only what a flight
//! needs on top.
//!
//! Landing, taking off, each change made in the spaceport
//! ([`Session::transact`]) and a Strict Play pilot's rescue from the
//! escape pod ([`Session::rescue_if_due`]) make a save due
//! ([`Session::take_save_due`]): whoever saves the pilot takes it after
//! each input and saves then.
//! Arriving in a system explores it.
//!
//! The ship lands on a stellar of its system, on the land key's second
//! press, when the [`landing`](crate::landing) rules allow it, which read
//! the pilot's legal record in its system (by the Bible's reading of
//! [`RuleKey::LegalRecords`], the record with the stellar's government,
//! or with its system's when the stellar has none): docked, it rests
//! at the stellar's centre and ticks move nothing until it takes off
//! again, from the same place. A landed ship cannot jump, nor a jumping
//! one land.
//!
//! The galaxy is the catalog's, read once when the session starts, with
//! the pilot's galaxy overlay applied ([`Session::galaxy`]): every reader
//! here (the star map, the systems' and stellars' governments, landing,
//! the legal record, traffic and the AI) reads it, and
//! [`Session::change_galaxy`] alone changes it. The player is always in a
//! visible system: a pilot flown, or a player whose system the overlay
//! hides, goes to the visible system at the same place, and so does every
//! arrival (see the [`story`](crate::story) module's "Galaxy overlay").
//!
//! The player plots a course to a system on the star map: the fewest
//! jumps along the hyperlinks, which the
//! session's [`HyperlinkRule`] (the engine's, one way along each system's
//! own links, unless [`Session::with_hyperlinks`] says otherwise) lets it
//! follow. J
//! ([`Session::begin_jump`]) is accepted when the
//! [`hyperspace`](crate::hyperspace) rules allow a jump to the next system
//! on it, with the no-jump zone applying where the session's
//! [`JumpZoneRule`] says (the engine's, only in a system with a stellar
//! that is no gate, unless [`Session::with_jump_zone`] says otherwise).
//! The ship then flies the pre-jump stage on its own, the player's
//! controls ignored: it brakes and turns to the map bearing of that system
//! as [`pre_jump`] says ([`Session::preparing_jump`]), and the jump begins
//! once it is ready ([`Session::jumping`]), at once if it already is. A
//! jump accepted is committed. While the jump lasts ticks move nothing.
//! When it is over ([`Session::arrive`]) the ship is in the next system, at its edge,
//! with a jump's fuel used and the days its stats give a jump gone by, and
//! the rest of the course still ahead. A ship with a multi-jump outfit
//! passes on along the course in the same jump, as many systems as
//! [`hops_per_jump`] gives, and enters only the last; the session's
//! [`MultiJumpRule`] (the engine's unless [`Session::with_multi_jump`]
//! says otherwise) decides whether the chain costs one jump's fuel and
//! days or each hop's. An arrival that made no hop clears the course, or
//! drops its first hop when that stands at the arrival, and each rebuild
//! point rewrites a hidden hop to the system at its place and cuts the
//! course at a hop with none, as
//! [`RuleKey::RouteRewrite`] says (see the [`course`](crate::course)
//! module). In flight, fuel regenerates each tick at the rate the ship
//! and its outfits give.
//!
//! The land key's second press over a hypergate or wormhole, once
//! cleared, enters it rather than docking ([`Session::land`]); the
//! player then picks a hypergate's destination
//! ([`Session::open_hypergate`], [`Session::enter_hypergate`]),
//! while a wormhole picks its own ([`Session::enter_wormhole`]), as the
//! [`gate`](crate::gate) rules say. Neither uses fuel nor minds the
//! no-jump zone, and the ship comes out as the session's
//! [`GateArrivalRule`] says; by the engine's, at the exit gate with no
//! day passing.
//!
//! Everything about how the ship performs (its handling, the most shield,
//! armour and fuel it holds, its fuel regeneration and its cargo space)
//! comes from its [`ShipStats`]: its `shïp`'s fields, read when the session
//! starts (or from the record of a ship bought since), and the outfits the
//! pilot owns. A pilot from a save made
//! before outfits were kept owns its ship's default items.
//!
//! The player selects one of the system's stellars as the navigation
//! target ([`Session::select_stellar`]) by its slot in the system's
//! navigation defaults, as the nearest landable one, or by clicking it,
//! as the [`navigation`](crate::navigation) rules say, and clears it with
//! Nav Off ([`Session::clear_nav_target`]); or with the land key
//! ([`Session::land`]), which selects the nearest landable stellar when
//! there is no target and lands on the target when there is one. Landing
//! clears it. The target is not part of the pilot, so it is not saved,
//! and it is independent of the course.
//! Arriving in another system clears it.
//!
//! Hyper Select ([`Session::select_next_system`], the original's `\`
//! key) cycles the course through the systems linked with the one the
//! ship is in, as the session's [`HyperSelectRule`] says (the engine's
//! unless [`Session::with_hyper_select`] says otherwise), each press
//! leaving a one-jump course and clearing the navigation target.
//!
//! Each day that goes by steps the planetary events (see
//! [`market`](crate::market)), rolling whether each can start on the
//! [`Chance`] the caller passes to [`Session::arrive`].
//!
//! Landed at a trade center, the player trades on its exchange
//! ([`Session::market`], [`Session::trade`]), with the cargo space the ship
//! and its outfits give; the goods traded and the events that move their
//! prices are read when the session starts. A trade makes a save due; a
//! refused one changes nothing. In flight, held tribbles `jünk` grow and
//! perishable `jünk` decay a ton on each due frame, the 15th tick after a
//! take-off and then as the [`market`](crate::market) rules time it, as
//! the session's rule set reads [`RuleKey::JunkFlags`] (the engine's
//! unless [`Session::with_rules`] says otherwise); that makes no save due.
//!
//! Landed at an outfitter, the player buys and sells outfits one at a
//! time ([`Session::outfitter`], [`Session::outfit`]) as the
//! [`outfitter`] rules say, from the `oütf`s read when
//! the session starts. Each changes the stats at once: a gauge whose most
//! rises gains as much (a new tank comes full), and one whose most falls
//! keeps no more than it can hold. A change makes a save due; a refused
//! one changes nothing.
//!
//! Landed at a shipyard, the player buys a new ship
//! ([`Session::shipyard`], [`Session::buy_ship`]) as the [`shipyard`]
//! rules say, from the `shïp`s read when the session starts, trading in
//! the one flown. From then on the session flies the new ship: its fields
//! and default items come from its record, and the stats from them and
//! the outfits the purchase leaves. A purchase makes a save due; a refused
//! one changes nothing.
//!
//! Landed where fuel is sold, the player recharges
//! ([`Session::recharge`]), filling the tank as the
//! [`recharge`](crate::recharge) rules say. A refill makes a save due; a
//! refused one changes nothing.
//!
//! As it goes the session emits [`SimSound`] events (thrust starting and
//! stopping, landing, taking off, a jump beginning and ending, and a set
//! operator's message and sound), which the audio side drains with
//! [`Session::take_sounds`]. A refused landing or
//! jump emits nothing.
//!
//! A change of the player's ship, or of the system or stellar it is at,
//! is told as a [`SessionEvent`], which a view drains with
//! [`Session::take_events`] and redraws from, whatever made the change
//! (see the `events` module). The other outboxes, the sounds among them,
//! keep their own `take_*`; folding them into the events is a possible
//! follow-up.
//!
//! In flight the engine glow's base level ([`Session::engine_glow`])
//! ramps with the thrust each tick, as [`glow`](crate::glow) says, and
//! landing and beginning a jump put it out.
//!
//! NPC traffic flies around the player (see [`traffic`](crate::traffic)):
//! a session starts with none, and its system is filled with its initial
//! population ([`Session::populate`]) on each arrival, replacing the last
//! system's, and on the first traffic tick in flight after the session
//! starts and after each take-off, as the original sets a system up on
//! arrival and on take-off. [`Session::tick_traffic`] advances it a tick,
//! NPCs deciding as a [`Behaviour`] says and rolling on the caller's
//! [`Chance`], seeing the player's ship, the governments read when the
//! session starts, the system's government and the player's legal record
//! in the system (see [`legal`](crate::legal)), and answering the fight's strikes since the last traffic
//! tick; it stands still while the player is landed, and once the
//! player's warp is past its lead-in ([`Session::in_warp_lead_in`]), as
//! the player's own ship does. Through the lead-in the ships the player
//! leads keep their place beside it.
//!
//! Ships fight ([`combat`](crate::combat)): the player holds a fire
//! command ([`Session::hold_trigger`]) and aims at its target, and each
//! NPC holds the fire command and aims at the target its [`Behaviour`]
//! last decided, and [`Session::tick_combat`] advances the fight a tick
//! among the player and the NPCs, with each ship's weapons, read when the
//! session starts, by the fight's [`Rules`]: disabling ships as a
//! [`DisableRule`](crate::combat::hull::DisableRule) says, and every
//! ship's point defence engaging the missiles fired at it, with no target
//! needed, as a [`PointDefenceRule`](crate::combat::defence::PointDefenceRule)
//! says. The player's weapons are its ship's stock weapons
//! and its weapon outfits, firing the rounds of its ammunition outfits and
//! its fuel, so a fight changes the pilot only in its shield, armour and
//! fuel and the ammunition it owns. The fight stands still while the
//! player is landed or jumping, though the scans are still taken through
//! the warp's lead-in, and its shots and beams are gone once the
//! player lands or arrives elsewhere. An NPC destroyed is taken out of the
//! system. The player's ship, once it is not intact, ignores the controls
//! and drifts, slowing a little each tick, cannot land or jump, and once destroyed stays where it is.
//! Disabled or dying, it may eject into an escape pod or an escape ship,
//! leaving a hulk behind (see the `eject` module); the pod flies itself
//! until its countdown ends and the player is rescued (see the `rescue`
//! module).
//!
//! The player's crimes change its legal records as the fight's
//! [`LegalCode`] says: disabling an NPC, and breaking one up (which is
//! disabling it too when it was intact), each against the NPC's
//! government, the record with each government changed and, by that
//! government's change, the record in each of its systems
//! ([`legal::convict`]). Each conviction then takes the ranks its crime
//! reaches (see [`rank`](crate::rank)'s **Crimes**). A mere hit is no
//! crime, as in the original, and a mission's special ship, of any mark,
//! is no victim of any of them (see the [`legal`] module's "Crimes").
//!
//! The player boards its target once it is disabled
//! ([`Session::board`]), as the [`board`](crate::board) rules say: the
//! boarding is the [`Crime::Board`] against the target's government, but
//! for a mission's special ship, and the NPCs come to its help as for a
//! hit. Then the player plunders it
//! ([`Session::plunder`]): its cargo into the hold, its credits, its
//! ammunition as outfits and its energy into the tank, each once, at the
//! risk of its self-destruct; and captures it, with odds from the crews,
//! by a [`BoardingRule`]. A ship captured joins the pilot's fleet or
//! becomes the player's own ship ([`Session::assign`]), which makes a
//! save due. The boarding under way is never saved. Boarding a person
//! may grant outfits of its `GrantClass` as the [`BoardingRule`] says
//! (see [`grant`](crate::grant)), added at once and told once
//! ([`Session::take_grant`]); like plunder, a grant makes no save due.
//!
//! The player hails its target ([`Session::hail`]), as the
//! [`hail`](crate::hail) rules say: the ship answers by its attitude to
//! the player, and the comm dialog lists the [`HailOption`](crate::HailOption)s
//! that apply ([`Session::hailing`]). A press ([`Session::answer`]) says
//! the ship's reply and does its deed: a ship paid to spare the player
//! never targets it again and leaves; one declined attacks; one asked for
//! help flies over to refuel or repair the player
//! ([`Session::tick_assistance`], [`Session::take_comm`]). A price asked is
//! haggled over and paid from the cash ([`Session::haggle`]), which makes
//! no save due. The hail under way, and the help, are never saved.
//!
//! The pilot's fleet flies with it: each escort an NPC
//! beside the player, keeping formation, following it through jumps, and
//! fighting by its standing order, which the player commands
//! ([`Session::command_escorts`], [`Session::escort_menu`]) and which
//! entering a system resets, or keeps, as the rule set reads
//! [`RuleKey::EscortOrders`]. A ship captured joins the fleet
//! where it is, and so does a person hailed that offers to join
//! (`person_join`'s other reading), flying as itself in every system; an
//! escort disabled or destroyed leaves it, and one hailed may be
//! released. Each change to the fleet makes a save due.
//!
//! The player's fighter bays launch fighters into the fleet, which Return
//! to Hangar brings back to dock, a round of their bay again; an NPC
//! carrier launches its own while it attacks (see [`bay`](crate::bay)).
//! What a fighter launched does first follows
//! [`RuleKey::FighterLaunch`]; what becomes of the fighters out as the
//! player jumps or lands follows [`RuleKey::FighterRecall`], and the
//! fighters abandoned are told ([`Session::take_fighter_notes`]).
//! Launching and docking make no save due, as firing ammunition does
//! not; losing or abandoning a fighter makes one. Buying a ship loses
//! every fighter out, and the outfitter sells a fighter only while its
//! bays have room, those out counted.
//!
//! Landed at a stellar with a bar, the player hires escorts
//! ([`Session::escorts_for_hire`], [`Session::hire`]) as the
//! [`hire`](crate::hire) rules say, by the session's [`HireTerms`] and
//! [`ControlBits`]: a hire pays its fee and joins the fleet with its daily
//! wage, which makes a save due. Each hired escort is paid its wage for
//! each day of a jump, and for one day at each take-off, whatever days
//! the stay passes, as the rule set reads [`RuleKey::TakeOffPay`]; one
//! left unpaid defects ([`Session::take_pay_notes`]). Which wage it is
//! paid follows [`RuleKey::EscortWage`], and hailing it shows it.
//!
//! Days pass through one daily step (see the [`story`](crate::story)
//! module's "Daily step"): a jump's days, a gate's under
//! [`GateArrivalRule::LikeJump`], and, as the rule set reads
//! [`RuleKey::StayDays`], the days a spaceport stay passes as it is
//! left: the take-off's own day, a day for an outfit bought or sold, four
//! for a ship bought, and half as many as escorts sold or upgraded
//! ([`Session::take_off`]). [`Session::set_date`] and loading a save
//! write the date directly, with no day passing.
//!
//! The pilot's control bits are read with [`Session::control_bit`] and
//! set or cleared with [`Session::set_control_bit`], which makes a save
//! due. A set expression runs on the session ([`Session::run_set`]),
//! writing bits, with its other operators applied by `Session::apply_op`,
//! one match over the set operators, which
//! grants (`G`) and removes (`D`) outfits, explores systems (`X`),
//! changes the player's ship outside the shipyard (`C`, `E`, `H`, as
//! [`ShipChangeRules`] reads the rule set where the rules are
//! disputed) and renames it (`T`, from the string lists
//! [`Session::with_strings`] gives), moves the player to another
//! system (`M`, `N`) and makes it leave the stellar it is landed on
//! (`Q`, with a message from those string lists), applied when
//! [`Session::settle_script`] settles them, and plays a sound (`P`), as
//! [`ScriptEffectRules`] reads the rule set where the rules are
//! disputed; one nothing handles is skipped and told once
//! ([`Session::take_script_notes`]). See the `ship_change` and
//! `script_effects` modules.
//!
//! Buying an outfit, a boarding grant and `G` share one grant path, the
//! original's: a map explores, a clean-record outfit cleans the legal
//! record and a paint paints the ship instead of being added, as
//! [`OutfitRules`](crate::OutfitRules) reads the rule set where the
//! rules are disputed (see
//! the `outfits` module and [`outfit_effects`](crate::outfit_effects)).
//!
//! For tests and the developer tools, plain edits put the pilot into a
//! given state without flying it there: its credits
//! ([`Session::set_credits`]), shield, armour and fuel within the ship's
//! maxima ([`Session::set_reserve`]), the date ([`Session::set_date`]),
//! and, while landed, a move to a stellar of another system
//! ([`Session::relocate`]). Each keeps the session consistent and makes a
//! save due; see the `edit` module.
//!
//! Persons (see [`person`](crate::person)) appear in the systems their
//! records allow, by the session's [`PersonRules`]
//! ([`Session::with_person_rules`]), named by their records
//! ([`Session::npc_name`]); a destroyed unique person, or a captured one,
//! is gone for good, and one the player hits may hold a grudge, both
//! kept on the pilot with no save due. A person hailed says its comm
//! quote as the rule set reads [`RuleKey::CommQuote`], and in flight its
//! hail
//! quote on its trigger ([`Session::tick_quotes`],
//! [`Session::take_quotes`]). A person's `LinkMission` is resolved as it
//! spawns, and offered in place of the comm dialog when it is hailed
//! ([`Session::open_hail`]), or, with `Flags` 0x0200, in place of the
//! plunder when it is boarded, while the mission is available from a
//! ship; the offer is answered through [`Session::accept_mission`] and
//! [`Session::refuse_mission`] (see the `person_offers` module).
//!
//! The player targets an NPC ([`Session::select_target`]), the nearest,
//! the nearest threat or the next in turn as the
//! [`targeting`](crate::targeting) rules say, and
//! fires its primary weapons and the secondary selected
//! ([`Session::hold_fire`], [`Session::select_secondary`]). No secondary
//! is selected to start with, as `_ResetPlayer` (@0x1d4dd) leaves it, a
//! reload included; it is set by [`Session::select_secondary`] alone, and
//! let go on a ship purchase (`_DoShipyardDialog` @0x5f01c), a capture as
//! the player's ship (`_DoShipCapture` @0x4173e) and the outfitter's sale
//! of the last outfit mounting it (`_DoOutfitDialog` @0x5ce82), even
//! while another outfit still mounts the weapon
//! ([`RuleKey::SecondarySale`]). Every other refit, `G`, `D`, a script's
//! change of ship and boarding among them, leaves it as it is, one the
//! ship no longer carries too: it is still named, and fires nothing
//! ([`RuleKey::StaleSecondary`]). The ship carries a weapon while an
//! outfit the pilot owns mounts it; by the other readings of those keys,
//! a refit and the sale let go of a selection the ship no longer
//! carries, and keep it otherwise. The target is let go once it starts breaking up, is
//! destroyed, lands or jumps out, when the system is populated afresh,
//! and when the player lands; it is kept through a jump, and gone on
//! arrival with the last system's traffic. Neither the target nor the
//! secondary is saved.
//!
//! jump emits nothing. The fight's [`CombatEvent`]s are drained with
//! [`Session::take_combat_events`], and the [`SimDiagnostic`]s about game
//! data the simulation does not handle yet, each once a session, with
//! [`Session::take_diagnostics`].

mod aux_ships;
mod brackets;
mod cargo_boxes;
mod cloaks;
mod control;
mod course;
mod crons;
mod edit;
mod eject;
mod entry;
mod escorts;
mod events;
mod explored;
mod fighters;
mod gate_escorts;
mod gate_follow;
mod hail;
mod hire;
mod hooks;
mod hover;
mod jump_collapse;
mod mission_cargo;
mod mission_escorts;
mod mission_screens;
mod mission_ships;
mod missions;
mod nebulae;
mod news;
mod outfits;
mod person_offers;
mod persons;
mod player_info;
mod player_led;
mod precalced;
mod radar;
mod ranks;
mod red_alert;
mod rescue;
mod sabotage;
mod script_effects;
mod self_repair;
mod ship_change;
mod status_cargo;
mod stellars;
mod story;
mod takeoff_grace;
mod target_panel;

pub use hooks::HookRules;
pub use ship_change::ShipChangeRules;

pub use crate::mission::{AcceptError, OfferPrompt, VenueOffer};
pub use edit::RelocateRefusal;
pub use events::SessionEvent;
pub use mission_screens::{
    ComputerRefusal, ComputerRow, InfoRow, MissionComputer, MissionInfo, MissionText,
    MissionTextKind,
};
#[cfg(any(test, feature = "fixture"))]
pub(crate) use missions::StopRefusal;
pub use missions::{AbortError, NotOffered};
pub use person_offers::Hailed;
pub use persons::PersonQuote;
pub use script_effects::{ScriptEffectRules, Settled};
pub use takeoff_grace::TAKEOFF_GRACE_TICKS;

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use self::entry::{Arrival, Dock, Entry, Placement, ShipPlacement};
use self::hooks::ShipHook;
use self::outfits::{carries, launches};
use self::ship_change::ClassChange;
use crate::ai::{Behaviour, Goal, PlayerSide};
use crate::bay::FighterNote;
use crate::board::{
    AMMO_GROWTH, Assigned, Assignment, BoardRefusal, BoardTarget, Boarding, BoardingRule,
    CAPTURE_TRIP, CARGO_GROWTH, CREDITS_GROWTH, CaptureCrew, ENERGY_GROWTH, ESCORT_ARMOR_SHARE,
    EscortCrew, HeldRounds, MARINES, MAX_ESCORTS, MAX_SHIPS_IN_SYSTEM, Plunder, PlunderView, Prize,
    SELF_DESTRUCT_ROLL, TAKEOVER_ARMOR_BASE, TAKEOVER_ARMOR_SHARE, TOUGH_TAKEOVER_ARMOR_SHARE,
    Take, Taken, check_board,
};
use crate::catalog::{
    CombatCatalog, CronRecord, DateAffixes, DudeId, GateSite, GovtId, JunkId, LandingSite,
    ListedStellar, MissionId, MissionRecord, MissionTexts, NebulaRecord, OutfitId, OutfitRecord,
    PersonId, PilotCatalog, PlaceNames, RankId, RankRecord, ShipId, ShipRecord, StartError,
    StellarId, StellarLife, SystemId, SystemVisibility, TrafficCatalog, WeaponId,
};
use crate::chance::Chance;
use crate::combat::armament::{
    Armament, Arsenal, OutfitRounds, Trigger, fit_stock, next_secondary, outfit_rounds,
};
use crate::combat::beam::Beam;
use crate::combat::hull::{Breakup, Condition, DisableRule, HullSpec};
use crate::combat::projectile::Shot;
use crate::combat::report::{Reports, SimDiagnostic};
use crate::combat::weapon::Ammo;
use crate::combat::{Combat, CombatEvent, Downed, Fighter, Rules, ShipRef, Shove, Strike};
use crate::control::{ControlBits, NovaBits, ScriptNote};
use crate::date::{self, GameDate};
use crate::flight::{Controls, ShipState, step};
use crate::fuel::regenerate;
use crate::gate::{
    self, GateArrivalRule, GateKind, GateRefusal, WormholeRule, emerge, has_links, hypergate_exit,
    wormhole_exit,
};
use crate::geometry::Vec2;
use crate::glow::ramp_glow;
use crate::govt::Governments;
use crate::grant::{GrantStock, Granted, PersonGrant};
use crate::hail::CommNote;
use crate::handling::{Handling, ShipFields};
use crate::hire::{HireTerms, NovaHire, PayNote};
use crate::hyperspace::{
    CoLocated, HyperSelectRule, HyperlinkRule, JUMP_FUEL, JumpReadiness, JumpRefusal, JumpZoneRule,
    MultiJumpRule, RouteError, StarMap, arrival, check_jump, hops_per_jump, jump_bearing,
    jump_zone, next_hyper_destination,
};
use crate::landing::{LandOutcome, LandingRefusal, Standing, land_or_select, nearest_landable};
use crate::legal::{self, Crime, LegalCode};
use crate::market::{
    self, Direction, EscortHolds, Good, Goods, Market, Markup, Order, TradeRefusal,
};
use crate::message::SimMessage;
use crate::mission::{Offers, StellarMarks};
use crate::navigation::stellar_under;
use crate::nebula;
use crate::news::News;
use crate::order_loop::{self, Onward, OrderRun, Pass};
use crate::outfitter::{
    self, FIRST_OUTFIT, Hardpoints, LIST_SLOTS, OutfitFlags, OutfitOrder, OutfitRefusal, OutfitRun,
    OutfitVisit, Outfitter, Reselection, SaleAlert, Shop, outfit_mods,
};
use crate::person::{NovaPersons, PersonRules, PersonWorld};
use crate::pilot::{self, Escort, Pilot};
use crate::player_fate::{Ending, HOLD_TICKS, PlayerCommand, PlayerFate};
use crate::pre_jump::{self, PreJump};
use crate::rating;
use crate::recharge::{self, RechargeRefusal};
use crate::red_alert::RedAlert;
use crate::reserves::{Gauge, Reserves};
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::ship_change::{OutfitCarry, carried_outfits};
use crate::shipyard::{
    self, Purchase, Quote, Sale, ShipNaming, ShipPurchase, ShipRefusal, Shipyard, Yard,
};
use crate::sound::SimSound;
use crate::stats::ShipStats;
use crate::story::day::Stay;
use crate::story::rebuild::Rebuilt;
use crate::story::{Entrance, Fate, OverlaidGalaxy, StoryEvent, listers_of};
use crate::targeting::{self, TargetPick};
use crate::top_speed;
use crate::traffic::autopilot::Outcome;
use crate::traffic::npc::{Npc, NpcId};
use crate::traffic::table::{SpawnDude, SpawnTable};
use crate::traffic::{Traffic, World, strength};
use crate::wares::DayRolls;
use crate::warp;

pub use crate::combat::hull::ESCAPE_POD_SHIP;

/// What [`Session::tick_combat`] works out for a ship's [`Fighter`]
/// beyond what the ship carries.
#[derive(Clone, Copy, Debug)]
struct FightTerms {
    /// Whether it is held disabled.
    held_disabled: bool,
    /// Its class's top speed, its `Speed` alone.
    class_speed: f32,
    /// Its top speed.
    top_speed: f32,
}

impl FightTerms {
    /// `npc` in the fight, on these terms. Its jump is never under way:
    /// an NPC's jump out has no stage here, as it leaves on the tick it
    /// reaches its jump distance.
    fn fighter(self, npc: &mut Npc) -> Fighter<'_> {
        let fleet = npc.fleet();
        let Npc {
            id,
            ship,
            govt,
            stats,
            state,
            hull,
            trigger,
            target,
            reserves,
            condition,
            armament,
            rounds,
            cloak,
            ..
        } = npc;
        Fighter {
            ship: ShipRef::Npc(*id),
            ship_type: *ship,
            fleet,
            govt: *govt,
            state: *state,
            hull: *hull,
            shield_regen: stats.shield_regen,
            armor_regen: stats.armor_regen,
            trigger: *trigger,
            target: *target,
            reserves,
            condition,
            held_disabled: self.held_disabled,
            cloaked: cloak.is_cloaked(),
            class_speed: self.class_speed,
            top_speed: self.top_speed,
            jumping: false,
            armament,
            rounds,
        }
    }
}

/// A jump, from J being accepted until the ship arrives.
///
/// Each carries the warp's clock: the engine's ticks since the warp stage
/// began, the original's +0xa4 stamp (see [`warp`]). The stage begins on
/// the first tick of the jump on which the ship is no longer braking
/// ([`PreJump::Turn`] or [`PreJump::Ready`]), as `_HandlePlayer` stamps
/// +0xa4 once the ship is slow enough or can expedite its jumps
/// (@0x6b5e9-0x6b5f5), and goes back to none whenever the ship brakes
/// again, as its restamp does every braking frame (@0x6b6a2-0x6b6b1).
/// Kept in the jump, it ends with it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Jump {
    /// The ship brakes and turns towards `bearing`, the map bearing of
    /// `to` (none when there is nothing to turn to), before it jumps;
    /// `warp` is the warp's clock, none while it brakes.
    PreJump {
        to: SystemId,
        bearing: Option<f32>,
        warp: Option<u32>,
    },
    /// The jump to the system has begun, `warp` ticks into the warp stage.
    Hyperspace { to: SystemId, warp: u32 },
}

/// Which stellar [`Session::select_stellar`] makes the navigation
/// target, by the original's means (`_HandlePlayer` @0x68390).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StellarPick {
    /// The stellar in this slot of the system's navigation defaults,
    /// from 0: the original's keys 1-4 and F1-F4 (@0x69cf6-0x69f17).
    Slot(usize),
    /// The nearest landable stellar, the one the land key selects with no
    /// target ([`nearest_landable`]): the original's F5, which picks
    /// through the same `_FindNearestStellar` (@0x276c) its land key does.
    Nearest,
    /// The stellar under a click at this point in the system
    /// ([`stellar_under`]), each boxed by the frame of the sheet drawn for
    /// it, so a destroyed one by its `DeadType` or graphic type 0's sheet:
    /// the original's click in space (@0x6a5f9-0x6a6fc).
    At(Vec2),
}

/// What [`Session::land`] did, when it was not refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandPress {
    /// What the [`landing`](crate::landing) rules gave: a stellar
    /// selected, or one landed on and docked at.
    Outcome(LandOutcome),
    /// The ship is cleared and over a hypergate or wormhole, and the
    /// entry awaits [`Session::enter_hypergate`] or
    /// [`Session::enter_wormhole`]: what the landing rules would let it
    /// land on, it enters instead (see [`gate`]).
    AtGate {
        /// The stellar.
        stellar: StellarId,
        /// What it is.
        kind: GateKind,
    },
}

impl From<LandOutcome> for LandPress {
    fn from(outcome: LandOutcome) -> Self {
        Self::Outcome(outcome)
    }
}

/// The player's ship, flying in one system.
// Each flag is its own part of the flight's state, set and read on its
// own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    /// The pilot flying: everything a save keeps.
    pilot: Pilot,
    /// The ship's fields, read when the session starts.
    fields: ShipFields,
    /// The ship's default items, read when the session starts.
    defaults: BTreeMap<OutfitId, u16>,
    /// The ship's stock weapons and their ammunition, as outfits
    /// ([`Arsenal::stock_outfits`]), read when the session starts.
    stock: BTreeMap<OutfitId, u16>,
    /// Every `oütf`, read when the session starts.
    outfits: Vec<OutfitRecord>,
    /// Every `shïp`, read when the session starts.
    ships: Vec<ShipRecord>,
    /// How the ship performs, with the outfits it carries.
    stats: ShipStats,
    player: ShipState,
    /// The stellars shown: those of `listed` that `shown` holds, and the
    /// one docked at, derived whenever [`Session::set_player_system`] says
    /// and at the flight rebuild points (see the `entry` module).
    sites: Vec<LandingSite>,
    /// Every stellar `shown` lists, in `NavDefs` order, through the
    /// galaxy overlay: what `sites` are derived from.
    listed: Vec<LandingSite>,
    /// The system whose stellars `sites` are: the player's, except while
    /// the ship is docked at a stellar of another system.
    shown: SystemId,
    /// The stellar the ship is docked at, if it has landed.
    landed: Option<StellarId>,
    /// The stellar selected as the navigation target, one of `sites`.
    nav_target: Option<StellarId>,
    /// The galaxy every reader sees: the catalog's systems, read when the
    /// session starts, with the pilot's overlay (see the
    /// [`story`](crate::story) module's "Galaxy overlay").
    galaxy: OverlaidGalaxy,
    /// The jump under way, from J being accepted until the ship arrives.
    jump: Option<Jump>,
    /// The warp sound's duration, in Mac ticks, read when the session
    /// starts: the d of the warp's clock ([`warp::sound_duration`]).
    warp_sound: u32,
    /// The rule a multi-jump follows.
    multi_jump: MultiJumpRule,
    /// The rule Hyper Select follows.
    hyper_select: HyperSelectRule,
    /// The rule jumps along the hyperlinks follow.
    hyperlinks: HyperlinkRule,
    /// The rule for where the no-jump zone applies.
    jump_zone: JumpZoneRule,
    /// The hypergate or wormhole the land key has just been pressed over,
    /// cleared: its entry awaits until the next tick.
    gate: Option<StellarId>,
    /// The rule a ship coming out of a gate follows.
    gate_arrival: GateArrivalRule,
    /// The rule an unlinked wormhole follows.
    wormholes: WormholeRule,
    /// The goods traded and the events that move their prices, read when
    /// the session starts.
    goods: Goods,
    /// Whether the ship is thrusting, as the last sounds told it.
    thrusting: bool,
    /// The engine glow's base level (see [`glow`](crate::glow)).
    engine_glow: u8,
    /// The sounds emitted since they were last taken.
    sounds: Vec<SimSound>,
    /// The messages raised since they were last taken.
    messages: Vec<SimMessage>,
    /// The events told since they were last taken.
    events: Vec<SessionEvent>,
    /// Whether the pilot has changed in a way that should be saved since
    /// this was last taken.
    save_due: bool,
    /// What the date is wrapped in when it is displayed, read when the
    /// session starts.
    date_affixes: DateAffixes,
    /// The NPCs in the system.
    traffic: Traffic,
    /// Whether the system is to be populated on the next traffic tick in
    /// flight: from the session's start, and after a relocation that
    /// resets the traffic while landed. A take-off populates at once.
    traffic_due: bool,
    /// Every weapon and ship type's combat fields, read when the session
    /// starts.
    arsenal: Arsenal,
    /// Every government and their relations, read when the session
    /// starts.
    govts: Governments,
    /// Each ammunition outfit, with the weapon it is the rounds of.
    ammo_outfits: Vec<(WeaponId, OutfitId)>,
    /// The player's ship type's hull.
    hull: HullSpec,
    /// The player's weapons.
    armament: Armament,
    /// How the player's ship is holding up.
    condition: Condition,
    /// Whether the player's ship was destroyed on the latest
    /// [`Session::tick_combat`], the tick the fight raised its
    /// `CombatEvent::Destroyed`; read by [`Session::player_fate`]. It stays
    /// set until the next `tick_combat`, so every interpolated view frame
    /// drawn after that tick sees [`PlayerFate::Exploded`]: the original hides
    /// on its own frame boundary, so this crosses no boundary the player
    /// can see.
    player_exploded_this_tick: bool,
    /// How far the game's end has got since the player's ship exploded:
    /// set to the hold's start on the explosion tick, counted down by
    /// [`Session::tick`]'s dead branch, then moved on by
    /// [`Session::begin_fade`] and [`Session::end_play`]. Read only while
    /// the ship is destroyed, and never saved: the session is dropped
    /// when play ends.
    player_end: Option<Ending>,
    /// Whether the hold ran out on a tick since
    /// [`Session::take_hold_expired`] was last asked.
    hold_expiry_due: bool,
    /// The escape pod's countdown, the ticks still to run before the
    /// rescue, while the player is in the pod: the original's +0xc908
    /// while it is above 0, set to [`POD_TICKS`](crate::POD_TICKS) by the
    /// eject (@0x6f669) and counted down by the pod's flight
    /// (@0x68d95-0x68dcd). Never saved: the pilot may not be saved while
    /// it runs ([`crate::FateRow::saves`]).
    pod: Option<u32>,
    /// Whether the pod countdown has run out and the rescue is due (see
    /// the `rescue` module); never saved.
    rescue_due: bool,
    /// Whether the rescue's dësc is still to show (see the `rescue`
    /// module); never saved.
    rescue_desc_due: bool,
    /// Whether the player holds the eject chord, Command and the eject
    /// key together (see the `eject` module); never saved.
    eject_held: bool,
    /// The fire command the player holds.
    trigger: Trigger,
    /// The shots and beams in flight, and what the fight reports.
    combat: Combat,
    /// The cargo boxes jettisons left in space (see the `cargo_boxes`
    /// module); never saved, as the original saves none.
    boxes: crate::cargo_box::CargoBoxes,
    /// What the session reports outside combat.
    reports: Reports,
    /// The NPC the player targets, if any.
    target: Option<NpcId>,
    /// What the view shows of space about the player's ship (see the
    /// `cloaks` module).
    play_area: crate::cloak::Area,
    /// The secondary weapon the player has selected, if any: none to
    /// start with, set by the select key alone, and let go as the module
    /// docs say. By the engine it may name a weapon the ship no longer
    /// carries ([`RuleKey::StaleSecondary`]).
    secondary: Option<WeaponId>,
    /// The fight's strikes since the last traffic tick, for the NPCs to
    /// answer.
    strikes: Vec<Strike>,
    /// The boarding under way, if any.
    aboard: Option<Aboard>,
    /// What the last boarding granted, until it is taken.
    granted: Option<Granted>,
    /// The hail under way, if any.
    talk: Option<hail::Talk>,
    /// What the NPCs assisting the player have done since this was last
    /// taken.
    comm: Vec<CommNote>,
    /// What the player's fighters met as it left systems since this was
    /// last taken.
    fighter_notes: Vec<FighterNote>,
    /// Each escort of the fleet's NPC in the system, lined up with the
    /// pilot's escorts while in flight; none for one not placed.
    fleet: Vec<Option<NpcId>>,
    /// Whether the person escorts are restocked full to their fitted
    /// ships as they are placed next: after a take-off, and when a pilot
    /// is flown.
    restock_persons: bool,
    /// The last order the player gave each escort class, in
    /// [`EscortClass::ALL`](crate::escort::EscortClass::ALL) order: the
    /// original's one order per class (0x3b5394), which a ship joining
    /// the class takes (see [`Session::class_order`]). Not saved, as the
    /// hired escorts keep theirs on their records and mission escorts are
    /// never saved; reset with their orders on entering a system by the
    /// engine's [`RuleKey::EscortOrders`].
    class_orders: [Option<crate::escort::EscortOrder>; 4],
    /// The fee and wage of a hire (see [`Session::with_hire_terms`]).
    hire_terms: hire::Shared<dyn HireTerms>,
    /// The control-bit test a ship's `Availability` for hire goes through
    /// (see [`Session::with_control_bits`]).
    control_bits: hire::Shared<dyn ControlBits>,
    /// Each ship class's roll for hire today: drawn by the daily step, or
    /// else the first time the bar's list asks it (see
    /// [`RuleKey::DayRolls`]). Not saved.
    hire_rolls: DayRolls<ShipId>,
    /// The news of the bar visit under way, drawn as the bar opened (see
    /// the `news` module); forgotten as the docking changes, and never
    /// saved.
    bar_news: Option<News>,
    /// Each outfit's roll for sale today: drawn by the daily step, or else
    /// the first time the outfitter's list asks it (see
    /// [`RuleKey::DayRolls`]). Not saved.
    outfit_rolls: DayRolls<OutfitId>,
    /// The outfitter visit under way ([`OutfitVisit`]): the
    /// once-an-opening flags, which limit what is sold as
    /// [`RuleKey::OutfitLimit`] reads, and the counts owned when it
    /// opened, which a sale refunds against as [`RuleKey::OutfitRefund`]
    /// reads. None of it is saved.
    outfit_visit: OutfitVisit,
    /// The outfit a relisting selected, `oütf` 128, while it may still be
    /// selected: the engine's `*0xded58` after an order that changes the
    /// listing, which Buy and Sell answer for, listed or not (see
    /// [`RuleKey::RelistedSelection`]). The outfitter carries its row even
    /// when it is not listed ([`Outfitter::unlisted`]). An order of
    /// another outfit, and each opening, lets it go. It is not saved.
    outfit_reselected: Option<OutfitId>,
    /// What this spaceport stay has done that passes days as it is left
    /// (see [`Session::take_off`]). A landing starts it afresh. It is not
    /// saved: the original keeps it in flags `_DoPortDialog` clears on
    /// opening (@0x5f93d-0x5f95a), and loading a save passes no time.
    stay: Stay,
    /// Each ship class's roll for sale today: drawn by the daily step, or
    /// else the first time the shipyard's list asks it (see
    /// [`RuleKey::DayRolls`]). Not saved.
    ship_rolls: DayRolls<ShipId>,
    /// The ship classes whose roll is drawn again the next time the
    /// shipyard's list is built: each one bought or declined since it was
    /// last built. The original builds the list once each time the
    /// shipyard opens (`_SetupPortAvailableShipTypes`, called only
    /// @0x5e68f), so a roll drawn again shows only then. A landing with
    /// no day passing forgets their rolls, and a day's draw, which draws
    /// them afresh, empties it.
    ship_redraws: BTreeSet<ShipId>,
    /// The original's frame counter, which times the tribbles and
    /// perishables step (see [`market`]). The original's value when a
    /// flight starts depends on how long the program has run, so 0 here
    /// is a choice; a take-off or a gate's exit sets it anyway. Not
    /// saved, as the
    /// original's pilot file does not keep it.
    frame: i16,
    /// The take-off grace's ticks still to run (see the `takeoff_grace`
    /// module); not saved, as the original's pilot file does not keep its
    /// frame counter.
    takeoff_grace: u32,
    /// The Red Alert's state (see the `red_alert` module); not saved, as
    /// the original's pilot file does not keep it.
    red_alert: RedAlert,
    /// How far out the nav target's brackets are (see the `brackets`
    /// module); not saved, as the original's pilot file does not keep it.
    nav_fly_in: crate::brackets::FlyIn,
    /// How far out the ship target's brackets are; not saved either.
    target_fly_in: crate::brackets::FlyIn,
    /// Whether the player's sound effects are quiet, which by
    /// [`RuleKey::AlertBlink`]'s engine reading blinks the Red Alert's
    /// icon; set by the view from the player's preferences, never saved.
    sound_quiet: bool,
    /// What paying the escorts did since this was last taken.
    pay_notes: Vec<PayNote>,
    /// What sees each set operator `run_set` hands to `apply_op`: none
    /// outside tests (see `control::SetOpObserver`).
    op_observer: Option<hire::Shared<dyn control::SetOpObserver>>,
    /// What running set expressions had to tell since this was last taken.
    script_notes: Vec<ScriptNote>,
    /// How persons appear (see [`Session::with_person_rules`]).
    person_rules: hire::Shared<dyn PersonRules>,
    /// The hail quotes' clock.
    quote_clock: persons::QuoteClock,
    /// The hail quotes said since they were last taken.
    quotes: Vec<persons::PersonQuote>,
    /// When the player's ship is disabled after a change of ship (see
    /// [`Session::with_disable_rule`]).
    disable_rule: hire::Shared<dyn DisableRule>,
    /// The string lists `T` names the ship from (see
    /// [`Session::with_strings`]).
    strings: ship_change::Strings,
    /// What the set expressions left for later: for
    /// [`Session::settle_script`], and the next flight tick; never saved.
    queued: script_effects::Queued,
    /// Every stellar the player's system lists, waiting for the take-off
    /// while the ship is docked at a stellar of another system, `shown`
    /// (see [`Session::set_player_system`]).
    next_sites: Option<Vec<LandingSite>>,
    /// Whether the next take-off keeps the ship where it touched down
    /// (the original's `_dontMovePlayerAfterLanding`); never saved.
    hold_position: bool,
    /// Where the ship was as it last landed, before it was docked at the
    /// stellar's centre.
    touchdown: Vec2,
    /// How each disputed rule the session acts on reads (see
    /// [`Session::with_rules`]); never saved.
    rules: Rulebook,
    /// Each ship slot's draw layer and when it entered it (see
    /// [`Session::draw_order`]), and the frames it last cloned (see
    /// [`Session::framed_by`]): kept from the session's start, across
    /// every system entry and every ship leaving; never saved.
    slot_sprites: crate::ship_layer::SlotSprites,
    /// The story runtime: the events raised and the ships watched (see the
    /// [`story`](crate::story) module); never saved.
    story: story::StoryRuntime,
    /// Every stellar's life, by ID, read when the session starts: what `Y`
    /// and the daily step read of a stellar's destruction and
    /// regeneration (see the `stellars` module).
    lives: BTreeMap<StellarId, StellarLife>,
    /// The current strength of each whole stellar shots have damaged, by
    /// ID; one not here has its full `Strength` (see the `stellars`
    /// module). Never saved, as the original saves none.
    strength: BTreeMap<StellarId, i64>,
    /// The stellars whose `OnDestroy` is running, so that none runs again
    /// inside itself (see the `stellars` module); never saved.
    destroying: BTreeSet<StellarId>,
    /// The stellars shown that have been dead since the last rebuild point
    /// (see the `stellars` module); never saved, as a load is itself a
    /// rebuild point.
    rebuilt: Rebuilt,
    /// Graphic type 0's sheet's frame size, or `None` when it cannot be
    /// read, read from the catalog as the session starts: what a rebuild
    /// point fills an empty sprite slot with, and the box a click on that
    /// slot reads (see the `stellars` module).
    fallback_frame: Option<(u32, u32)>,
    /// Every `mïsn`, read when the session starts.
    missions: Vec<MissionRecord>,
    /// The `düde`s of every `mïsn` of `Flags` 0x0800 with a `ShipDude`
    /// that can be read, read when the session starts: what such a
    /// mission draws its ships' one type from at accept (see the
    /// `mission_ships` module).
    uniform_dudes: BTreeMap<DudeId, SpawnDude>,
    /// Every `crön`, read when the session starts, shared so the daily
    /// pass holds them while their scripts run (see the `crons` module).
    crons: Rc<[CronRecord]>,
    /// Every `ränk` 128-255, read when the session starts (see the
    /// `ranks` module).
    ranks: Rc<[RankRecord]>,
    /// The rank last activated, which `<RRK>` reads, until `L` or a sweep
    /// clears it. Never saved: `None` on every [`Session::fly`], as the
    /// original's `ResetPlayer` (@0x1db44) and `ResetPlayerRecord`
    /// (@0x1dbf1) reset it (see the `ranks` module).
    recent_rank: Option<RankId>,
    /// What the ranks' price multipliers are taken from as the spaceport
    /// opens, `_DoPortDialog`'s inputs (see the `ranks` module). Never
    /// saved, as `_priceFluxMult` lives in memory only.
    port_ranks: ranks::PortRanks,
    /// The active ranks as the last reset took them, which the blessing
    /// table is built from (see the `ranks` and `precalced` modules).
    /// Never saved, as the original's table lives in memory only.
    blessing_ranks: BTreeSet<RankId>,
    /// Whether the resets leave the cargo past the holds untrimmed: set
    /// through an outfit sale by the other reading of
    /// [`RuleKey::OutfitSaleCargo`] (see the `precalced` module). Never
    /// saved.
    sale_keeps_cargo: bool,
    /// `_ScanPlayer`'s contraband latches, armed at each reset
    /// ([`Session::reset_precalced`], the session's stand-in for
    /// `_ResetPlayerPrecalcedValues`), and cleared by a conviction (see
    /// [`scan::Latches`](crate::scan::Latches)). Never saved, as the
    /// original's are globals.
    scan_latches: crate::scan::Latches,
    /// The sabotage clock, `_sabotageCounter` with the cached
    /// `_playerSabotaged`, refreshed at each reset
    /// ([`Session::reset_precalced`]; see the `sabotage` module). Never
    /// saved, as the original's are globals.
    sabotage: crate::sabotage::SabotageClock,
    /// The `jünk` latches the tribbles and perishables step reads, set
    /// only at each reset ([`Session::reset_precalced`]; see
    /// [`market::JunkLatches`]). Both clear until the first. Never saved,
    /// as the original's are globals.
    junk_latches: market::JunkLatches,
    /// What the screens show of each `mïsn`, read when the session
    /// starts (see the `mission_screens` module).
    mission_texts: BTreeMap<MissionId, MissionTexts>,
    /// Every stellar some system lists, read when the session starts:
    /// what a mission's stellars are chosen from, with the galaxy overlay.
    listed_stellars: Vec<ListedStellar>,
    /// Every system's `Visibility`, read when the session starts: what the
    /// galaxy refresh evaluates (see the `story` module).
    visibility: Rc<[SystemVisibility]>,
    /// Every `nëbu` 128-159, read when the session starts: their
    /// `ActiveOn`, which the galaxy refresh evaluates, and their
    /// `OnExplore` (see the `nebulae` module).
    nebulae: Rc<[NebulaRecord]>,
    /// Whether a set expression has changed the pilot since the last
    /// settling: by the other reading of [`RuleKey::VisibilityRefresh`]
    /// the settling refreshes the galaxy then.
    galaxy_stale: bool,
    /// Every stellar's and system's name, read when the session starts:
    /// what the text tags read (see the `missions` module).
    place_names: PlaceNames,
    /// Every stellar's place and name, read when the session starts: what
    /// a mission's stop is matched by (see the `missions` module).
    stellar_marks: StellarMarks,
    /// The offers of the visit under way and the stellar they were rolled
    /// at (see the `missions` module); never saved, as the original saves
    /// none of it.
    offers: Offers,
    /// Whether a mission succeeded on the landing under way, so the
    /// offers are re-offered after its last slot (see the `missions`
    /// module); never saved.
    reoffer_due: bool,
    /// The slots of the missions accepted in flight, whose ship goals are
    /// checked once the settling that accepted them is done (see the
    /// `mission_ships` module); never saved.
    goal_checks_due: Vec<u8>,
    /// The person's NPC an accept on its hail swaps for the ship of the
    /// mission in the slot, once the settling that accepted it begins
    /// (see the `mission_ships` module); never saved.
    swap_due: Option<(NpcId, u8)>,
    /// The slot whose mission's script is running, if any: what a `Q`
    /// fills its mission tags from (see the `missions` module); never
    /// saved.
    script_slot: Option<u8>,
    /// The mission slots whose `OnAbort` is running, so that none ends
    /// again inside itself (see the `missions` module); never saved.
    ending: BTreeSet<u8>,
    /// The mission texts to show, until they are taken (see the
    /// `mission_screens` module); never saved.
    shown_texts: Vec<MissionText>,
    /// The outside offers whose accept was refused, passed over until
    /// another venue is entered (see the `mission_screens` module); never
    /// saved, as the original saves none of it.
    suppressed: BTreeSet<MissionId>,
    /// The venue offers were last entered at, an `AvailLoc`; never saved.
    offer_venue: Option<i16>,
    /// The offers from a ship, by mission, each resolved as a person
    /// linked to it spawned (see the `person_offers` module); never saved,
    /// as the original saves none of it.
    ship_offers: person_offers::ShipOffers,
    /// The offer from a ship the offer dialog shows, until the player
    /// answers it (see the `person_offers` module); never saved.
    offering: Option<person_offers::Offering>,
    /// Whether an offer dialog is open, as the original's `_missionDialog`
    /// is set: while the player's accept or refusal runs, so an `S` in
    /// that `OnAccept` or `OnRefuse` sees it (see the `missions` module);
    /// never saved.
    offer_dialog_open: bool,
    /// Whether the load had to fall back for data the save references
    /// and the catalog lacks ([`Session::load_repaired`]); never saved.
    load_repaired: bool,
}

impl Session {
    /// A new pilot's session, read from `catalog`: an unnamed
    /// [`Pilot::new`], flown.
    pub fn start(catalog: &(impl PilotCatalog + CombatCatalog)) -> Result<Self, StartError> {
        Self::fly(catalog, Pilot::new(catalog, "")?)
    }

    /// `pilot`'s session, its ship's fields and default items, the
    /// outfits, its system's stellars, the star map, the goods, and the
    /// weapons and ship types' combat fields read from `catalog`, with the
    /// system marked explored. A pilot whose ship
    /// still carries its default items, from an old save, owns them now;
    /// one whose ship still mounts its stock weapons beside its outfits,
    /// from a save before they were outfits, owns them and their
    /// `AmmoLoad` on top of its outfits, keeping the armament it flew
    /// with; one whose ship was never named, from a save before ships had
    /// names, names it after its class (empty when the class has no
    /// record); one whose legal records in each system are not yet
    /// derived, a new pilot or a save from before they were kept, derives
    /// them from its records with each government
    /// ([`legal::systems_from_govts`]); one saved with a crön running
    /// or a rank active that the data no longer has forgets it; and the
    /// reserves hold no more than the stats allow. No secondary is
    /// selected, as the original's `_ResetPlayer` leaves it before
    /// `_LoadPilotData` (@0x76502,
    /// @0x7673e), which never sets one.
    ///
    /// Every system's visibility is evaluated from its `Visibility` against
    /// the pilot's bits, the saved flags never trusted, quietly: the load's
    /// galaxy refresh, which moves a pilot whose system it hides as any
    /// refresh does (see the [`story`](crate::story) module's "Visibility
    /// and governments"), and writes every nebula's activity from its
    /// `ActiveOn`. The explored nebulae are forgotten, but for those kept
    /// in the once-fired record, and [`StoryEvent::Explored`] is raised
    /// for every visible explored system, by ascending ID: the load's
    /// sweep (see the `nebulae` module).
    ///
    /// A pilot last landed on a stellar that some system lists resumes in
    /// the system that stellar decides (`resume_system`), not the one it
    /// was saved in, as `_LoadPilotData` sets the system from the stellar
    /// alone (@0x7525b-0x75316); one whose stellar no system lists keeps
    /// its saved system. It resumes docked at that stellar,
    /// silently, as the original resumes a pilot at its last planet, the
    /// spaceport opening with its ranks' price multipliers (see the
    /// `ranks` module); so
    /// does one whose stellar is another system's at its place that the
    /// galaxy overlay hid, its own system's stellars waiting for the
    /// take-off.
    /// Otherwise (or when that stellar is no longer in the system, which
    /// the pilot then forgets) the ship starts at rest at the system's
    /// centre, facing up.
    ///
    /// [`Session::load`] under [`Rulebook::default`]: see there for the
    /// fallbacks a save whose data is missing takes. The session's own
    /// rule set is [`Rulebook::default`] until [`Session::with_rules`].
    ///
    /// # Errors
    ///
    /// When the ship cannot be read, or no system exists at all.
    pub fn fly(
        catalog: &(impl PilotCatalog + CombatCatalog),
        pilot: Pilot,
    ) -> Result<Self, StartError> {
        Self::load(catalog, pilot, &Rulebook::default())
    }

    /// `pilot`'s session as [`Session::fly`] builds it, the load's
    /// fallbacks reading `rules`, and [`Session::load_repaired`] saying
    /// whether one applied, as `_LoadPilotData` returns -0x2e when it set
    /// its warning flag (@0x7648b-0x76498), the load otherwise complete:
    ///
    /// - A pilot whose stellar no system lists, so the loaded data doesn't
    ///   place it, resumes as [`RuleKey::UnlistedStellar`] reads: by the
    ///   engine in the lowest system that exists, as `_LoadPilotData` takes
    ///   the first with +0x1ed set when `_FindSystemFromStellar` finds none
    ///   (@0x752d6-0x75318), by the other reading (the default) in its
    ///   saved system. Either way the stellar is forgotten, as the system
    ///   doesn't list it, and the ship starts at the centre: the original's
    ///   position at the stellar's x/y in a system that doesn't hold it
    ///   (@0x75329-0x75354) isn't copied.
    /// - A pilot whose system no longer exists, its stellar unlisted or
    ///   `None`, resumes in the lowest system that exists, at the centre,
    ///   under either reading. The original has no such case, as it takes
    ///   the system from the stellar alone; the engine's saved system is
    ///   its own, and refusing the pilot would strand one a removed
    ///   plug-in's system held.
    ///
    /// A pilot never landed, its stellar `None`, is not repaired: the
    /// original's stellar -1 fallback (@0x751e9-0x75255) is left out, as
    /// the engine's `None` stellar doesn't mean missing data.
    ///
    /// # Errors
    ///
    /// When the ship cannot be read, or no system exists at all
    /// ([`StartError::NoSystem`] with the pilot's saved system).
    #[allow(clippy::too_many_lines)] // one line for each of the session's fields
    pub fn load(
        catalog: &(impl PilotCatalog + CombatCatalog),
        mut pilot: Pilot,
        rules: &Rulebook,
    ) -> Result<Self, StartError> {
        let ship = pilot.ship;
        let fields = catalog
            .ship_fields(ship)
            .map_err(|reason| StartError::Ship(ship, reason))?;
        // The explored nebulae are never saved: a load forgets them, as
        // `_ResetPlayer` does (@0x1da86), but for those the once-a-pilot
        // reading kept fired (see the `nebulae` module).
        pilot.galaxy.forget_explored_nebulae();
        for kept in nebula::kept(&pilot.story_fired) {
            pilot.galaxy.set_nebula_explored(kept, true);
        }
        // Built first, from the star map and the saved overlay alone: the
        // resume reads the saved visibility, as `leave_hidden` does, both
        // standing in for the load's first refresh (@0x751ca, @0x751cf).
        let mut galaxy = OverlaidGalaxy::new(catalog.star_map(), pilot.galaxy.clone());
        let listed = catalog.listed_stellars();
        let saved = pilot.system;
        let mut load_repaired = false;
        if let Some(stellar) = pilot.stellar {
            if let Some(system) = listed
                .iter()
                .find(|listed| listed.id == stellar)
                .and_then(|listed| resume_system(&galaxy, &listed.systems))
            {
                pilot.system = system;
            } else {
                load_repaired = true;
                if rules.source_for(RuleKey::UnlistedStellar) == RuleSource::Engine {
                    pilot.system = lowest_system(&galaxy, catalog).unwrap_or(pilot.system);
                }
            }
        }
        if !catalog.system_exists(pilot.system) {
            pilot.system = lowest_system(&galaxy, catalog).ok_or(StartError::NoSystem(saved))?;
            load_repaired = true;
        }
        // A load is one of the original's refresh points: a pilot in a
        // hidden system flies from the one standing for it, silently.
        story::leave_hidden(&mut pilot, &mut galaxy);
        pilot.explore(pilot.system);
        let govts = Governments::read(catalog);
        if !pilot.system_records_derived() {
            // A new pilot, or a save from before the records were kept
            // per system (see the `legal` module).
            pilot.system_legal = Some(legal::systems_from_govts(
                galaxy.governed(),
                &pilot.legal,
                &govts,
            ));
        }
        // A crön the data no longer has is inactive (`_LoadPilotData`
        // @0x75fbc-0x76025; see the `crons` module).
        let crons: Rc<[CronRecord]> = catalog.crons().into();
        let missions = catalog.missions();
        pilot
            .crons
            .retain(|id, _| crons.iter().any(|record| record.id == *id));
        // So is a rank (`_LoadPilotData` @0x75e47-0x75eb6; see the
        // `ranks` module).
        let ranks: Rc<[RankRecord]> = catalog.ranks().into();
        pilot
            .ranks
            .retain(|id| ranks.iter().any(|record| record.id == *id));
        let system = pilot.system;
        let here = overlaid_sites(&galaxy, catalog, system);
        let (docked, dock) = docking(&galaxy, catalog, &pilot, &here);
        let player = ShipState {
            position: docked.map_or(Vec2::ZERO, |site| site.position),
            ..ShipState::default()
        };
        let landed = docked.map(|site| site.id);
        pilot.stellar = landed;
        let defaults = pilot::default_outfits(catalog, ship);
        let outfits = catalog.outfits();
        let arsenal = Arsenal::read(catalog);
        let stock = arsenal.stock_outfits(ship, &outfits);
        let ships = catalog.ships();
        fill_in(&mut pilot, &defaults, &stock, &ships);
        let mut session = Self {
            fields,
            defaults,
            stock,
            ammo_outfits: Arsenal::ammo_outfits(&outfits),
            outfits,
            ships,
            // Refitted below, from the outfits the pilot owns.
            stats: ShipStats::default(),
            player,
            sites: Vec::new(),
            listed: Vec::new(),
            shown: system,
            landed,
            nav_target: None,
            galaxy,
            jump: None,
            warp_sound: warp::sound_duration(catalog.warp_sound()),
            multi_jump: MultiJumpRule::default(),
            hyper_select: HyperSelectRule::default(),
            hyperlinks: HyperlinkRule::default(),
            jump_zone: JumpZoneRule::default(),
            gate: None,
            gate_arrival: GateArrivalRule::default(),
            wormholes: WormholeRule::default(),
            goods: Goods::read(catalog),
            thrusting: false,
            engine_glow: 0,
            sounds: Vec::new(),
            messages: Vec::new(),
            events: Vec::new(),
            save_due: false,
            date_affixes: catalog.date_affixes(),
            traffic: Traffic::new(),
            traffic_due: true,
            arsenal,
            govts,
            // Refitted below, from the ship and the outfits the pilot owns.
            hull: HullSpec::default(),
            armament: Armament::default(),
            condition: Condition::Intact,
            player_exploded_this_tick: false,
            player_end: None,
            hold_expiry_due: false,
            pod: None,
            rescue_due: false,
            rescue_desc_due: false,
            eject_held: false,
            trigger: Trigger::default(),
            combat: Combat::default(),
            boxes: crate::cargo_box::CargoBoxes::default(),
            reports: Reports::default(),
            target: None,
            play_area: crate::cloak::DEFAULT_PLAY_AREA,
            secondary: None,
            strikes: Vec::new(),
            aboard: None,
            granted: None,
            talk: None,
            comm: Vec::new(),
            fighter_notes: Vec::new(),
            fleet: Vec::new(),
            restock_persons: false,
            class_orders: [None; 4],
            hire_terms: hire::Shared(Rc::new(NovaHire::default())),
            control_bits: hire::Shared(Rc::new(NovaBits)),
            hire_rolls: DayRolls::default(),
            bar_news: None,
            outfit_rolls: DayRolls::default(),
            outfit_visit: OutfitVisit::default(),
            outfit_reselected: None,
            stay: Stay::default(),
            ship_rolls: DayRolls::default(),
            ship_redraws: BTreeSet::new(),
            frame: 0,
            takeoff_grace: 0,
            red_alert: RedAlert::default(),
            nav_fly_in: crate::brackets::FlyIn::default(),
            target_fly_in: crate::brackets::FlyIn::default(),
            sound_quiet: false,
            pay_notes: Vec::new(),
            op_observer: None,
            script_notes: Vec::new(),
            person_rules: hire::Shared(Rc::new(NovaPersons::default())),
            quote_clock: persons::QuoteClock::default(),
            quotes: Vec::new(),
            disable_rule: Self::nova_disable(),
            strings: ship_change::Strings::none(),
            queued: script_effects::Queued::default(),
            next_sites: None,
            hold_position: false,
            touchdown: player.position,
            rules: Rulebook::default(),
            slot_sprites: crate::ship_layer::SlotSprites::default(),
            story: story::StoryRuntime::default(),
            lives: catalog
                .stellar_lives()
                .into_iter()
                .map(|life| (life.id, life))
                .collect(),
            strength: BTreeMap::new(),
            destroying: BTreeSet::new(),
            rebuilt: Rebuilt::default(),
            fallback_frame: catalog.fallback_frame_size(),
            uniform_dudes: mission_ships::uniform_dudes(&missions, catalog),
            missions,
            crons,
            ranks,
            recent_rank: None,
            port_ranks: ranks::PortRanks::default(),
            blessing_ranks: BTreeSet::new(),
            sale_keeps_cargo: false,
            scan_latches: crate::scan::Latches {
                junk: false,
                outfits: false,
            },
            sabotage: crate::sabotage::SabotageClock::default(),
            junk_latches: market::JunkLatches::default(),
            mission_texts: catalog
                .mission_texts()
                .into_iter()
                .map(|texts| (texts.id, texts))
                .collect(),
            listed_stellars: listed,
            visibility: catalog.system_visibility().into(),
            nebulae: catalog.nebulae().into(),
            galaxy_stale: false,
            place_names: catalog.place_names(),
            stellar_marks: StellarMarks::new(catalog.stellar_marks()),
            offers: Offers::default(),
            reoffer_due: false,
            goal_checks_due: Vec::new(),
            swap_due: None,
            ending: BTreeSet::new(),
            shown_texts: Vec::new(),
            suppressed: BTreeSet::new(),
            offer_venue: None,
            ship_offers: person_offers::ShipOffers::new(),
            offering: None,
            offer_dialog_open: false,
            script_slot: None,
            load_repaired,
            pilot,
        };
        session.register_offer_roll();
        session.register_nebulae();
        session.register_slots();
        session.set_player_system(system, here, dock);
        // A load and the main screen are rebuild points (@0x751cf,
        // @0x75324, @0x1a354, @0x1a8ca, @0x1b003, @0x457ee): the view
        // reads the state as it builds, so nothing is told.
        session.rebuild_quietly();
        session.open_outfit_visit();
        session.refit(false, ReservePolicy::Hold);
        session.restock_fleet();
        // A load's galaxy refresh (`_LoadPilotData` @0x751ca, @0x7531f),
        // with the facts complete; a new pilot's is `_DoNewPilot`'s, before
        // its `OnStart` (@0x1925e).
        session.refresh_galaxy_quietly(catalog);
        // A load's sweep of the visible explored systems' nebulae
        // (@0x765a2-0x765ce); a new pilot's (@0x19118), which by the
        // engine `Session::begin` withdraws (see the `nebulae` module).
        session.sweep_explored();
        // A load forgets the stellar the offers were rolled at (@0x75ef5)
        // and opens the spaceport, so a pilot resumed docked has them
        // rolled afresh (see the `missions` module), and its prices taken
        // as `_DoPortDialog` takes them (see the `ranks` module).
        if let Some(stellar) = session.landed {
            session.open_port_prices();
            session.raise(StoryEvent::Landed { stellar });
        }
        // Game entry's reset (`_EnterGameFromMainScreen` @0x19ffb; see the
        // `precalced` module).
        session.reset_precalced();
        Ok(session)
    }

    /// Whether the load had to fall back because the save references data
    /// the catalog lacks: a stellar no system lists, or a system that no
    /// longer exists (see [`Session::load`]). It stands for
    /// `_LoadPilotData`'s -0x2e result (@0x7648b-0x76498), on which
    /// `_MainScreenChoice` shows `STR#` 140 #52 (@0x1a840-0x1a85b).
    #[must_use]
    pub fn load_repaired(&self) -> bool {
        self.load_repaired
    }

    /// Turns the player's ship to a whole degree from 0 to 359 drawn on
    /// `chance`, as `_LoadPilotData` faces every pilot it loads, docked or
    /// not, with `Rand(0x168)` (@0x7535b-0x75372). For a pilot opened from
    /// a save, after [`Session::fly`]; a new pilot keeps facing up. Taking
    /// off keeps the heading.
    pub fn face_on_load(&mut self, chance: &mut dyn Chance) {
        self.player.heading = f32::from(chance.roll(360));
    }

    /// This session with every disputed rule reading as `rules` says
    /// ([`Rulebook::source_for`]): by default, [`Rulebook::default`], every
    /// rule the engine's. The whole rule set is replaced, so a session is
    /// given one rulebook with every override it needs.
    ///
    /// The session acts on the rules it decides itself through this rule
    /// set, reading each where it decides. The rules a strategy port
    /// decides (its law, boarding, AI, hail options, hire terms and
    /// persons; see [`RuleKey`]) are in the rule set too, but the session
    /// never reads them there: the port it was given reads them, and at
    /// the edge of the program that port is built from the same rulebook.
    /// Setting such a rule here alone changes nothing.
    #[must_use]
    pub fn with_rules(mut self, rules: Rulebook) -> Self {
        self.galaxy.set_colocated(CoLocated::from(
            rules.source_for(RuleKey::ColocatedResolution),
        ));
        self.combat
            .set_breakup(Breakup::from(rules.source_for(RuleKey::BreakupStall)));
        self.rules = rules;
        self
    }

    /// How each disputed rule reads (see [`Session::with_rules`]).
    #[must_use]
    pub fn rules(&self) -> &Rulebook {
        &self.rules
    }

    /// This session with multi-jumps following `rule`; the engine's by
    /// default.
    #[must_use]
    pub fn with_multi_jump(mut self, rule: MultiJumpRule) -> Self {
        self.multi_jump = rule;
        self
    }

    /// This session with Hyper Select following `rule`; the engine's by
    /// default.
    #[must_use]
    pub fn with_hyper_select(mut self, rule: HyperSelectRule) -> Self {
        self.hyper_select = rule;
        self
    }

    /// This session with jumps following the hyperlinks under `rule`; the
    /// engine's by default.
    #[must_use]
    pub fn with_hyperlinks(mut self, rule: HyperlinkRule) -> Self {
        self.hyperlinks = rule;
        self
    }

    /// This session with the no-jump zone applying where `rule` says; the
    /// engine's by default, only in a system with an ordinary stellar.
    #[must_use]
    pub fn with_jump_zone(mut self, rule: JumpZoneRule) -> Self {
        self.jump_zone = rule;
        self
    }

    /// This session with ships coming out of hypergates and wormholes as
    /// `rule` says; the engine's by default.
    #[must_use]
    pub fn with_gate_arrival(mut self, rule: GateArrivalRule) -> Self {
        self.gate_arrival = rule;
        self
    }

    /// This session with unlinked wormholes leading where `rule` says; the
    /// engine's by default.
    #[must_use]
    pub fn with_wormholes(mut self, rule: WormholeRule) -> Self {
        self.wormholes = rule;
        self
    }

    /// The ship's standard equipment: its default items and its stock
    /// weapons and their ammunition, whose mass `FreeMass` leaves out
    /// (`_LoadObjectData` @0x7aaf7, @0x7ac61, @0x7ad8d).
    fn standard(&self) -> BTreeMap<OutfitId, u16> {
        pilot::merged(&self.defaults, &self.stock)
    }

    /// The ship's stats with the outfits the pilot owns.
    fn current_stats(&self) -> ShipStats {
        ShipStats::new(
            self.fields,
            &outfit_mods(&self.pilot.outfits, &self.outfits),
        )
    }

    /// The policy `G`, `D` and boarding refit by: the rule set's reading
    /// of [`RuleKey::RefitReserves`].
    fn script_reserves(&self) -> ReservePolicy {
        ReservePolicy::reading(self.outfit_rules().refit_reserves)
    }

    /// Recomputes the stats from the outfits the pilot owns: each gauge
    /// holds up to the stats' most and, when `gain`, one whose most rose
    /// gains as much. The fuel keeps no more than its most; the shield and
    /// armour keep no more only under [`ReservePolicy::Hold`]. The hull
    /// and the weapons follow the ship and the outfits, ready to fire.
    /// The outfitter, the shipyard, capture and a reload hold; `G`, `D`
    /// and boarding read [`RuleKey::RefitReserves`] and a change of ship
    /// reads [`RuleKey::ShipChangeReserves`] (see
    /// [`ReservePolicy::reading`]). Capture, a change of ship and a
    /// purchase refit through [`Session::become_class`], by the policy
    /// they chose. By the engine, a refit leaves the secondary selected as
    /// it is, one the ship no longer carries too, as the original's
    /// `_SystemInfoToShipStats` never writes it; by the other reading of
    /// [`RuleKey::StaleSecondary`], it lets go of one the ship no longer
    /// carries (see the module docs).
    fn refit(&mut self, gain: bool, reserves: ReservePolicy) {
        let hold = reserves == ReservePolicy::Hold;
        self.hull = self.arsenal.hull(self.pilot.ship);
        self.armament = self.arsenal.player(&self.pilot.outfits, &self.outfits);
        let stats = self.current_stats();
        let reserves = &mut self.pilot.reserves;
        for (gauge, max, hold) in [
            (&mut reserves.shield, stats.shield, hold),
            (&mut reserves.armor, stats.armor, hold),
            (&mut reserves.fuel, stats.fuel, true),
        ] {
            refit(gauge, max, gain, hold);
        }
        self.stats = stats;
        if self.rules.source_for(RuleKey::StaleSecondary) == RuleSource::Bible
            && !carries(&self.pilot, &self.outfits, self.secondary)
        {
            self.secondary = None;
        }
    }

    /// Advances the session one tick under the player's `controls`, ramps
    /// the engine glow with the thrust, then regenerates fuel. During the
    /// pre-jump stage the controls are ignored: the ship flies the stage
    /// ([`pre_jump::fly`]) instead, and the jump begins on the tick it is
    /// ready. The warp's clock starts on the first tick of the jump on
    /// which the ship no longer brakes, sounding the warp-up, stops if it
    /// brakes again, and counts on through hyperspace until the ship
    /// arrives (see the [`warp`](crate::warp) module). A landed ship, or one in hyperspace, does
    /// not move, glows not at all, and gains no fuel. As the jump begins and on every tick in
    /// hyperspace, the ships the player leads let go of their targets
    /// (`_MakeEscortsHyperWithParent` @0x3fdc0-0x3fdc2; see the `escorts`
    /// module). In flight, a mission sound a `P` holds
    /// sounds (see the `script_effects` module). Any tick ends a gate's pending entry: the
    /// original's hypergate map is modal, so nothing ticks between the
    /// land key and the pick. A ship that is not intact ignores the
    /// controls and drifts, slowing by [`crate::flight::DISABLED_DRAG`] a
    /// tick before it moves, its pre-jump collapsing first (see the
    /// `jump_collapse` module), and a destroyed one stays where it is, letting go of its
    /// targets and counting the hold after its explosion down (see
    /// `dead_tick`).
    ///
    /// Each tick in flight, a jump's and the pod's included, counts the
    /// take-off grace down (see the `takeoff_grace` module). Each tick in
    /// flight, a jump's included, moves the frame counter on,
    /// and on a due frame ([`market::junk_step_due`]) steps the held
    /// tribbles and perishables in the space the goods and the mission
    /// cargo aboard leave, unless the player is destroyed. It then
    /// runs the Red Alert on that frame (see the `red_alert` module) and
    /// flies the target brackets in (see the `brackets` module). A
    /// landed tick moves none of them. While the escape pod's countdown
    /// runs, the pod flies itself in place of the controls, the jump and
    /// the fuel (see the `eject` module's `pod_tick`).
    pub fn tick(&mut self, controls: Controls) {
        self.gate = None;
        if self.landed.is_some() {
            return;
        }
        self.takeoff_grace_tick();
        self.sound_script();
        let frame = self.frame;
        self.frame = market::next_frame(frame);
        if market::junk_step_due(frame) && self.condition != Condition::Destroyed {
            let capacity = self.capacity();
            market::step_junk(
                &self.goods,
                &mut self.pilot.cargo,
                capacity,
                self.pilot.missions.tons_aboard(),
                self.rules.source_for(RuleKey::JunkFlags),
                self.junk_latches,
            );
        }
        self.red_alert_tick(frame);
        self.brackets_tick();
        self.boxes_tick();
        if self.pod.is_some() {
            self.pod_tick();
            return;
        }
        let controls = match self.condition {
            Condition::Intact => controls,
            Condition::Disabled | Condition::Dying { .. } => {
                self.collapse_jump();
                Controls::default()
            }
            Condition::Destroyed => {
                self.stop_thrust();
                self.player.velocity = Vec2::ZERO;
                self.dead_tick();
                return;
            }
        };
        match self.jump {
            None => {
                self.thrust(controls.thrust);
                let handling = self.flying_handling();
                if self.player_fate().row().drag {
                    crate::flight::drag(&mut self.player);
                }
                step(&mut self.player, &handling, controls);
            }
            Some(Jump::PreJump { to, bearing, warp }) => {
                let handling = self.flying_handling();
                let thrust =
                    pre_jump::fly(&mut self.player, &handling, bearing, !self.stats.fast_jump);
                self.thrust(thrust);
                self.stage_jump(to, bearing, warp.map(|ticks| ticks.saturating_add(1)));
            }
            Some(Jump::Hyperspace { to, warp }) => {
                self.jump = Some(Jump::Hyperspace {
                    to,
                    warp: warp.saturating_add(1),
                });
                self.let_go_of_led_targets();
                return;
            }
        }
        regenerate(&mut self.pilot.reserves.fuel, self.stats.fuel_regen);
    }

    /// `_HandlePlayer`'s dead branch (0x6870d-0x6888c), on each tick of a
    /// destroyed player: it lets go of the player's ship target and nav
    /// target (+0x70 @0x68712, +0x6c @0x68718), and counts the hold down a
    /// tick (`subss` @0x68727). On the tick after the last, the hold
    /// expires (`jbe` @0x68725): once, which
    /// [`Session::take_hold_expired`] reports. The rest of that tick still
    /// runs, as the original finishes the frame it sets `gPause` in.
    fn dead_tick(&mut self) {
        self.target = None;
        self.nav_target = None;
        let ending = self.player_end.get_or_insert(Ending::Holding {
            ticks_left: HOLD_TICKS,
        });
        match ending {
            Ending::Holding { ticks_left: 0 } => {
                *ending = Ending::Expired;
                self.hold_expiry_due = true;
            }
            Ending::Holding { ticks_left } => *ticks_left -= 1,
            Ending::Expired | Ending::Fading | Ending::GameOver => {}
        }
    }

    /// Whether the hold after the player's explosion has run out, now or
    /// on an earlier tick: true from the expiry's tick on, through the fade
    /// and game over. Asking leaves it as it is; the flight view stops
    /// stepping on it.
    #[must_use]
    pub fn hold_expired(&self) -> bool {
        self.condition == Condition::Destroyed && self.player_end.is_some_and(Ending::expired)
    }

    /// Whether the hold ran out since this was last asked: the seam at the
    /// countdown's expiry, before the original's `gPause = 1` (0x68838),
    /// where the sabotage dialogs show over the flight scene
    /// ([`Session::sabotage_death_descs`]).
    /// True once.
    pub fn take_hold_expired(&mut self) -> bool {
        std::mem::take(&mut self.hold_expiry_due)
    }

    /// Begins the fade to black after an expired hold: `_PlayGame`'s exit
    /// (0x465eb) and `_FadeScreenOut` (0x465f7). Nothing happens at any
    /// other stage.
    pub fn begin_fade(&mut self) {
        if self.condition == Condition::Destroyed && self.player_end == Some(Ending::Expired) {
            self.player_end = Some(Ending::Fading);
        }
    }

    /// Ends play once the fade is over, for the main screen to return
    /// (0x4664a, `_EnterGameFromMainScreen` 0x1a471). Nothing happens at
    /// any other stage.
    pub fn end_play(&mut self) {
        if self.condition == Condition::Destroyed && self.player_end == Some(Ending::Fading) {
            self.player_end = Some(Ending::GameOver);
        }
    }

    /// Whether the pilot may be saved now: only while the player is not
    /// dying or dead ([`crate::FateRow::saves`]).
    #[must_use]
    pub fn may_save(&self) -> bool {
        self.player_fate().row().saves
    }

    /// Whether ending play now deletes the pilot: `_EnterGameFromMainScreen`'s
    /// gate after `_PlayGame` returns (@0x1a4ab-0x1a4e1). A strict-play
    /// pilot whose player is dying (`_IsDying` @0x253d, which the fate
    /// table's [`crate::FateRow::saves`] already answers) and is not in
    /// the [`ESCAPE_POD_SHIP`] has its pilot file deleted
    /// (`_DeletePilotFile` @0x1a4d9). The original also writes a crash log
    /// line there (`_crashlog` @0x1a4cd), a debugging aid the engine has no
    /// use for, so it is left out. The gate makes no sound.
    #[must_use]
    pub fn deletes_pilot_on_ending_play(&self) -> bool {
        self.pilot.strict_play() && !self.may_save() && self.pilot.ship != ESCAPE_POD_SHIP
    }

    /// Sounds the thrust starting or stopping as `thrust` says, and ramps
    /// the engine glow with it.
    fn thrust(&mut self, thrust: bool) {
        if thrust != self.thrusting {
            self.thrusting = thrust;
            self.sounds.push(if thrust {
                SimSound::ThrustStarted
            } else {
                SimSound::ThrustStopped
            });
        }
        self.engine_glow = ramp_glow(self.engine_glow, thrust);
    }

    /// Fills the system with its initial NPC population, replacing any
    /// NPCs there (the target with them), its traffic read from `catalog`
    /// and rolled on `chance`, its stationary warships placed by the
    /// first stellar shown, destroyed or not, and its `LinkSyst` fleets
    /// coming out of a gate shown that the governments allow, or by
    /// hyperspace.
    pub fn populate(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.populate_as(catalog, chance, None);
    }

    /// Fills the system as [`Session::populate`] does, the active
    /// missions' special ships set up first, `entering` it by a way in
    /// whose enter-system hook is still to run, if any (see the
    /// `mission_ships` module). Once the ships are in, each mission of
    /// `Flags` 0x0001 whose slot the setup took auto-aborts, in slot
    /// order.
    fn populate_as(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
        entering: Option<Entrance>,
    ) {
        self.give_back_aux_ships(&BTreeSet::new());
        let table = self.spawn_table(catalog);
        let (missions, set_up) = self.setup_mission_ships(&table, entering, chance);
        let facts = control::Facts {
            pilot: &self.pilot,
            ammo_outfits: &self.ammo_outfits,
            armament: &self.armament,
        };
        let world = World {
            govts: &self.govts,
            persons: PersonWorld {
                rules: &*self.person_rules.0,
                gone: &self.pilot.gone_persons,
                grudges: &self.pilot.grudges,
                control_bits: &*self.control_bits.0,
                pilot: &facts,
                fleet: &self.pilot.escorts,
            },
            ..World::new(&self.sites)
        };
        let last = self.last_npc();
        let berths = self.fleet_berths();
        self.traffic.reserve(berths.iter().copied().collect());
        self.traffic.enter_with(table, world, missions, chance);
        self.resolve_person_offers(last, &mut &mut *chance);
        self.traffic_due = false;
        self.story.watched.clear();
        self.mission_ships_came(last);
        self.abort_set_up(&set_up, &mut &mut *chance);
        self.enter_escorts(&berths);
        if entering.is_none() {
            self.snap_mission_escorts();
        }
        self.strikes.clear();
        self.aboard = None;
        self.talk = None;
        self.clear_lost_target();
    }

    /// What the system's traffic is drawn from, read from `catalog`: its
    /// ships, persons and fleets, the pilot's person escorts aside, and
    /// the `düde`s of the active missions' special ships.
    fn spawn_table(&self, catalog: &(impl TrafficCatalog + ?Sized)) -> SpawnTable {
        let system = self.pilot.system;
        let mut table = SpawnTable::resolve(
            catalog,
            system,
            self.galaxy.govt(system),
            &self.govts,
            &self.ships,
            &self.outfits,
            &self.arsenal,
            &*self.person_rules.0,
            &self
                .pilot
                .escorts
                .iter()
                .filter_map(|escort| escort.person)
                .collect(),
        );
        table.add_mission_dudes(
            catalog,
            &self.mission_dudes(),
            &self.ships,
            &self.outfits,
            &self.arsenal,
        );
        table
    }

    /// Advances the NPC traffic one tick, NPCs deciding as `behaviour`
    /// says, rolling on `chance`. While the ship is landed, or jumping
    /// past the warp's lead-in ([`Session::in_warp_lead_in`]), it stands
    /// still; through the lead-in it flies on, so an interceptor can still
    /// look the player over, but for the ships the player leads and their
    /// fighters, which keep their place beside it. The first tick in flight after the session starts
    /// instead populates the system ([`Session::populate`]) from
    /// `catalog`, as the original sets a system up on arrival; its NPCs
    /// move from the next. A take-off populates the system itself
    /// ([`Session::take_off`]).
    /// A target that landed or jumped out is let go. Each tick that moves
    /// the NPCs raises the first sighting of the ships watched
    /// ([`Session::watch_ship`]), and each mission ship that left the
    /// system raises [`Fate::Departed`] and is counted (see the
    /// `mission_ships` module).
    pub fn tick_traffic(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        behaviour: &(impl Behaviour + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) {
        let flying = self.jumping().is_none() || self.in_warp_lead_in();
        if self.landed.is_none() && flying {
            if self.traffic_due {
                if self.jumping().is_none() {
                    self.populate(catalog, chance);
                }
            } else {
                self.mission_ship_arrivals(catalog, None, chance);
                self.unmark_inactive_mission_ships();
                self.join_mission_escorts();
                let system_govt = self.galaxy.govt(self.pilot.system);
                let facts = control::Facts {
                    pilot: &self.pilot,
                    ammo_outfits: &self.ammo_outfits,
                    armament: &self.armament,
                };
                let targeted = self.combat.targeted_damage();
                let world = World {
                    sites: &self.sites,
                    player: Some(self.player_side()),
                    govts: &self.govts,
                    system_govt,
                    record: self.system_record(),
                    persons: PersonWorld {
                        rules: &*self.person_rules.0,
                        gone: &self.pilot.gone_persons,
                        grudges: &self.pilot.grudges,
                        control_bits: &*self.control_bits.0,
                        pilot: &facts,
                        fleet: &self.pilot.escorts,
                    },
                    escort_tons: strength::escort_tons_in_system(self.traffic.npcs(), &self.ships),
                    led_speed: self.led_speed(),
                    targeted: &targeted,
                };
                let strikes = std::mem::take(&mut self.strikes);
                let last = self.last_npc();
                let marks = self.mission_marks();
                let still = self.held_through_the_warp();
                self.traffic
                    .tick_holding(behaviour, world, &strikes, &still, chance);
                self.mission_ships_left(&marks);
                self.resolve_person_offers(last, &mut &mut *chance);
                self.dock_fighters();
                self.orphan_fighters();
                self.observe_watched();
            }
        }
        self.clear_lost_target();
    }

    /// What the traffic flies among in flight: the system's stellars, the
    /// player, the governments, the system's government and the player's
    /// legal record with it (none in an independent system), and the
    /// persons' world: the session's rules and control bits, and the
    /// pilot's persons gone and grudges. No damage is aimed in it: only
    /// the traffic's tick fires, and it reads the fight's
    /// ([`Session::tick_traffic`]).
    fn world(&self) -> World<'_> {
        let system_govt = self.galaxy.govt(self.pilot.system);
        World {
            sites: &self.sites,
            player: Some(self.player_side()),
            govts: &self.govts,
            system_govt,
            record: self.system_record(),
            persons: PersonWorld {
                rules: &*self.person_rules.0,
                gone: &self.pilot.gone_persons,
                grudges: &self.pilot.grudges,
                control_bits: &*self.control_bits.0,
                pilot: self,
                fleet: &self.pilot.escorts,
            },
            escort_tons: strength::escort_tons_in_system(self.traffic.npcs(), &self.ships),
            led_speed: self.led_speed(),
            targeted: &crate::ai::NO_AIMED_DAMAGE,
        }
    }

    /// The player's ship as an NPC sees it.
    fn player_side(&self) -> PlayerSide {
        PlayerSide {
            state: self.player,
            condition: self.condition,
            reserves: self.pilot.reserves,
            hull: self.hull,
            handling: self.stats.handling,
            rating: self.pilot.combat_rating,
            yardstick: self.arsenal.strength(ShipId(128)),
            untargetable: self
                .ship_record(self.pilot.ship)
                .is_some_and(|record| crate::cloak::untargetable(record.fields.flags2)),
            blessing: self.blessing(),
        }
    }

    /// Holds `trigger`, the player's fire command, until another is held.
    pub fn hold_trigger(&mut self, trigger: Trigger) {
        self.trigger = trigger;
    }

    /// Holds the player's fire keys: the `primary` trigger, and the
    /// `secondary` one on the secondary weapon selected.
    pub fn hold_fire(&mut self, primary: bool, secondary: bool) {
        self.hold_trigger(Trigger {
            primary,
            secondary: self.secondary.filter(|_| secondary),
            ..Trigger::default()
        });
    }

    /// The secondary after `current`, `backwards` or not (see
    /// [`next_secondary`]), skipping one hidden while the pilot owns none
    /// of its rounds.
    fn next_secondary(&self, current: Option<WeaponId>, backwards: bool) -> Option<WeaponId> {
        next_secondary(self.armament.mounts(), current, backwards, |ammo| {
            outfit_rounds(&self.pilot.outfits, &self.ammo_outfits, ammo)
        })
    }

    /// Selects the next secondary weapon, or the one before it when
    /// `backwards`, in the order they were mounted, wrapping (see
    /// [`next_secondary`]).
    pub fn select_secondary(&mut self, backwards: bool) {
        self.secondary = self.next_secondary(self.secondary, backwards);
    }

    /// The secondary weapon selected, if any: none when the session
    /// starts, set by [`select_secondary`](Self::select_secondary) alone,
    /// and let go on a ship purchase, a capture as the player's ship and
    /// the sale of the last outfit mounting it (see the module docs). By
    /// the engine it may name a weapon the ship no longer carries, which
    /// fires nothing, as the original's `_DrawStatusWeap` still names it
    /// (@0x4c16c; [`RuleKey::StaleSecondary`]).
    #[must_use]
    pub fn secondary(&self) -> Option<WeaponId> {
        self.secondary
    }

    /// The count the status bar's secondary weapon line shows after the
    /// selected secondary's name, if any: the rounds of the weapon its
    /// [`line_count`](crate::combat::weapon::line_count) names, as
    /// `_DrawStatusWeap` (@0x4c1ce-0x4c1f4) reads the player's rounds of
    /// it; none when that names none, nothing is selected, or the
    /// session has no record of the weapon.
    #[must_use]
    pub fn secondary_line_count(&self) -> Option<u32> {
        let counted = self.arsenal.weapon(self.secondary?)?.line_count?;
        Some(outfit_rounds(
            &self.pilot.outfits,
            &self.ammo_outfits,
            counted,
        ))
    }

    /// Advances the fight a tick among the player and the NPCs, each aimed
    /// at its target, by `rules`, drawing each shot's inaccuracy and
    /// spread on `chance` (see [`Combat::tick`]); each NPC destroyed is taken out of
    /// the system, and the target is let go once it is breaking up or
    /// gone. While the ship is landed or jumping, it stands still.
    ///
    /// Each NPC intact before the tick and no longer raises
    /// [`Fate::Disabled`], and each destroyed raises [`Fate::Destroyed`]
    /// as it is taken out (see the `story` module's findings). A rescue
    /// mission's ship not yet boarded fights held disabled
    /// ([`Session::mission_ship_held`]; see the `mission_ships` module).
    /// The player is convicted of disabling and destroying an NPC, but
    /// not a mission's special ship (see `punish`). Before the fight,
    /// unless the ship is landed, each NPC that looked the player over in the
    /// traffic tick scans it, convicting by `rules`' law (see the
    /// `mission_cargo` module), jumping or not: past the warp's lead-in
    /// the scan itself is held, as `_ScanPlayer`'s jump-timing check holds
    /// it (@0x7e562-0x7e633). A scan queued in a lead-in step's traffic
    /// tick is taken here in the same step, so it settles before the
    /// frame is drawn and before the arrival. Then the fight stands still
    /// while the ship jumps. The stellars
    /// shown that shots can hit take their damage, and each shot
    /// down is destroyed and runs its `OnDestroy` (see the `stellars`
    /// module). While the player's ship breaks up, the klaxon sounds
    /// first among the tick's sounds (see `sound_the_klaxon`), and on the
    /// player's explosion tick a [`SimSound::Halt`] of the klaxon and the
    /// warp sounds follows it
    /// ([`PLAYER_EXPLOSION_HALTS`](crate::sound::PLAYER_EXPLOSION_HALTS)). A heavy
    /// ship's final explosion, the player's too, sends out a shockwave
    /// ([`shockwave`](crate::combat::shockwave)) that sets the velocity of
    /// each ship it pushes; the player's is not pushed while its jump is
    /// under way, the pre-jump stage included. Before anything else,
    /// unless landed or jumping, the sabotage clock ticks, and what it sets
    /// off goes off (see the `sabotage` module), and the eject is checked
    /// (see the `eject` module), both before the scans.
    pub fn tick_combat(&mut self, rules: Rules, chance: &mut (impl Chance + ?Sized)) {
        self.player_exploded_this_tick = false;
        if self.landed.is_some() || self.jumping().is_some() {
            // The fight stands still, but a scan a lead-in step's traffic
            // tick queued still settles (see `take_scans`).
            self.take_scans(rules.law, &mut &mut *chance);
            return;
        }
        self.sabotage_tick(rules, &mut &mut *chance);
        self.eject_before_fight(&mut &mut *chance);
        let was_destroyed = self.condition == Condition::Destroyed;
        let was_dying = matches!(self.condition, Condition::Dying { .. });
        self.take_scans(rules.law, &mut &mut *chance);
        let was: Vec<NpcBefore> = self.traffic.npcs().iter().map(NpcBefore::of).collect();
        let (player_terms, terms) = self.fight_terms();
        let mut stellars = self.stellar_targets();
        let pilot = &mut self.pilot;
        let mut rounds = OutfitRounds {
            owned: &mut pilot.outfits,
            sources: &self.ammo_outfits,
        };
        let mut fighters = vec![Fighter {
            ship: ShipRef::Player,
            ship_type: pilot.ship,
            fleet: ShipRef::Player,
            govt: None,
            state: self.player,
            hull: self.hull,
            shield_regen: self.stats.shield_regen,
            armor_regen: self.stats.armor_regen,
            trigger: if self.pod.is_some() {
                Trigger::default()
            } else {
                player_trigger(self.trigger)
            },
            target: self.target.map(ShipRef::Npc),
            reserves: &mut pilot.reserves,
            condition: &mut self.condition,
            held_disabled: false,
            cloaked: false,
            class_speed: player_terms.class_speed,
            top_speed: player_terms.top_speed,
            jumping: self.jump.is_some(),
            armament: &mut self.armament,
            rounds: &mut rounds,
        }];
        let npcs = self.traffic.npcs_mut().iter_mut().zip(terms);
        fighters.extend(npcs.map(|(npc, terms)| terms.fighter(npc)));
        self.combat.tick(
            &mut fighters,
            &self.arsenal,
            &self.govts,
            rules,
            &mut stellars,
            chance,
        );
        self.take_shoves();
        self.eject_on_breakup(&mut &mut *chance);
        self.player_exploded_this_tick = !was_destroyed && self.condition == Condition::Destroyed;
        if self.player_exploded_this_tick {
            self.player_end = Some(Ending::Holding {
                ticks_left: HOLD_TICKS,
            });
        }
        self.sound_the_klaxon(was_dying);
        if self.player_exploded_this_tick {
            self.sounds
                .push(SimSound::Halt(&crate::sound::PLAYER_EXPLOSION_HALTS));
        }
        self.raise_disabled(&was, &mut &mut *chance);
        self.answer_strikes(&was, rules.law);
        let sorties = self.combat.take_sorties();
        self.launch_fighters(&sorties);
        self.sync_fleet();
        self.lose_escorts();
        self.lose_dying_mission_escorts();
        self.self_repair(rules, &mut *chance);
        self.take_out_the_destroyed(&mut &mut *chance);
        self.orphan_fighters();
        self.clear_lost_target();
        self.strike_stellars(&stellars, rules.law, chance);
        self.see_stellars();
        self.lay_out_ships();
    }

    /// Raises [`SimSound::Klaxon`] while the player's ship breaks up, as
    /// `_HandlePlayer` plays `_klaxxonSnd` (@0x6f1e4-0x6f227) while the
    /// death timer, after its decrement, is above 1.0: on every tick of
    /// the countdown but the one it is set on (`starting`), and on the
    /// explosion tick of a ship that `was_dying` before it, whose timer
    /// came down to 2.0. The original's `> 1.0` test always holds for a
    /// countdown that is not starting: a timer at
    /// [`EXPLOSION_BAND`](crate::combat::hull::EXPLOSION_BAND) or below
    /// explodes, so one still counting down is at 3 or more. No klaxon
    /// sounds on the breakup tick, nor on a stalled breakup's ticks, each
    /// of which sets the timer again.
    ///
    /// It comes straight after the fight, ahead of every other sound of
    /// the tick, so that on the explosion tick the
    /// [`SimSound::Halt`] of [`PLAYER_EXPLOSION_HALTS`](crate::sound::PLAYER_EXPLOSION_HALTS)
    /// that [`Session::tick_combat`] raises next comes after it and wins,
    /// as `_HandlePlayer` (@0x43e19) runs before `_HandleShipDisplay`
    /// (@0x43e29). The view puts the session's sounds before the fight's,
    /// so the explosion's booms come after it too.
    fn sound_the_klaxon(&mut self, was_dying: bool) {
        let counting_down = matches!(
            self.condition,
            Condition::Dying {
                starting: false,
                ..
            }
        );
        if counting_down || (self.player_exploded_this_tick && was_dying) {
            self.sounds.push(SimSound::Klaxon);
        }
    }

    /// What the fight needs of the player's ship and of each NPC, in
    /// order, that the ship does not carry: whether it is held disabled
    /// ([`Session::mission_ship_held`]; the player never is), and its
    /// class and top speeds, which a shockwave pushes it against.
    fn fight_terms(&self) -> (FightTerms, Vec<FightTerms>) {
        let player = FightTerms {
            held_disabled: false,
            class_speed: self.class_speed(self.pilot.ship),
            top_speed: self.top_speed(),
        };
        let led_speed = self.led_speed();
        let npcs = self
            .traffic
            .npcs()
            .iter()
            .map(|npc| FightTerms {
                held_disabled: self.mission_ship_held(npc),
                class_speed: self.class_speed(npc.ship),
                top_speed: npc.top_speed(led_speed),
            })
            .collect();
        (player, npcs)
    }

    /// Sets the velocity of each ship the fight's shockwaves pushed this
    /// tick ([`Combat::take_shoves`]), which takes effect on its next
    /// movement.
    fn take_shoves(&mut self) {
        for Shove { ship, velocity } in self.combat.take_shoves() {
            match ship {
                ShipRef::Player => self.player.velocity = velocity,
                ShipRef::Npc(id) => {
                    let npc = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id);
                    if let Some(npc) = npc {
                        npc.state.velocity = velocity;
                    }
                }
            }
        }
    }

    /// Ship class `ship`'s top speed, its `Speed` alone
    /// ([`ShipFields::class_speed`](crate::handling::ShipFields::class_speed));
    /// none without a record.
    fn class_speed(&self, ship: ShipId) -> f32 {
        self.ship_record(ship)
            .map_or(0.0, |record| record.fields.class_speed())
    }

    /// Answers the fight's strikes this tick, against what each NPC `was`
    /// before it: refilling the invincible, holding persons' grudges,
    /// convicting the player by `law`, crediting kills and letting each
    /// hit NPC's cloak take its hit; then keeps them for the app.
    fn answer_strikes(&mut self, was: &[NpcBefore], law: &dyn LegalCode) {
        let mut strikes = self.combat.take_strikes();
        self.refill_invincible(&mut strikes);
        self.hold_grudges(&strikes);
        self.punish(&strikes, was, law);
        self.credit_kills(&strikes, was);
        self.cloak_hits(&strikes);
        self.strikes.extend(strikes);
    }

    /// Takes each NPC destroyed out of the system, raising
    /// [`Fate::Destroyed`] for it and counting it lost when it is a
    /// mission's special ship, drawing on `chance` (see the
    /// `mission_ships` module), as it goes. A mission escort among them
    /// has already left the formation, and the rest re-formed, as it
    /// started to break up ([`Session::lose_dying_mission_escorts`]).
    fn take_out_the_destroyed(&mut self, chance: &mut dyn Chance) {
        let destroyed: Vec<NpcId> = self
            .traffic
            .npcs()
            .iter()
            .filter(|npc| npc.condition == Condition::Destroyed)
            .map(|npc| npc.id)
            .collect();
        for id in destroyed {
            self.raise(StoryEvent::ShipFate {
                ship: id,
                fate: Fate::Destroyed,
            });
            self.count_lost_mission_ship(id, chance);
            self.lose_destroyed(id);
        }
    }

    /// Raises [`Fate::Disabled`] for each NPC that `was` intact before the
    /// fight's tick and is no longer: disabled, or broken up at once, which
    /// the original counts as disabled too (`_DamageShip` @0x3ae5b,
    /// @0x3af8b), as [`Session::punish`] convicts it of both. A mission's
    /// special ship is counted disabled, drawing on `chance` (see the
    /// `mission_ships` module).
    fn raise_disabled(&mut self, was: &[NpcBefore], chance: &mut dyn Chance) {
        let downed: Vec<NpcId> = self
            .traffic
            .npcs()
            .iter()
            .filter(|npc| {
                npc.condition != Condition::Intact
                    && was
                        .iter()
                        .any(|before| before.id == npc.id && before.condition == Condition::Intact)
            })
            .map(|npc| npc.id)
            .collect();
        for ship in downed {
            self.raise(StoryEvent::ShipFate {
                ship,
                fate: Fate::Disabled,
            });
            self.count_disabled_mission_ship(ship, chance);
        }
    }

    /// Convicts the player of its crimes among `strikes`, as `law` says:
    /// disabling an NPC, and breaking one up, which is disabling it too
    /// when it `was` intact before (as `_DamageShip` slaps both). Each
    /// victim's condition, government and mission mark are as it `was`
    /// before the fight's tick. A mission's special ship, of any mark,
    /// is convicted of neither: `_DamageShip` passes the ship's mark
    /// (@0x3adb3, @0x3ae08), and `_SlapWithPenalty` slaps only for none
    /// (@0x9c10; see the [`legal`] module's "Crimes"). Each conviction
    /// then takes the ranks its crime reaches (see [`rank`](crate::rank)'s
    /// **Crimes**).
    fn punish(&mut self, strikes: &[Strike], was: &[NpcBefore], law: &dyn LegalCode) {
        for strike in strikes.iter().filter(|strike| strike.by == ShipRef::Player) {
            let ShipRef::Npc(id) = strike.ship else {
                continue;
            };
            let Some(&NpcBefore {
                condition,
                govt,
                marked: false,
                ..
            }) = was.iter().find(|before| before.id == id)
            else {
                continue;
            };
            let crimes: &[Crime] = match (strike.downed, condition) {
                (Some(Downed::Disabled), _) => &[Crime::Disable],
                (Some(Downed::BreakingUp), Condition::Intact) => &[Crime::Disable, Crime::Kill],
                (Some(Downed::BreakingUp), _) => &[Crime::Kill],
                (None, _) => &[],
            };
            for &crime in crimes {
                let changes = law.penalties(crime, govt, &self.govts);
                legal::convict(&mut self.pilot, &changes, self.galaxy.governed());
                self.lose_ranks_to(crime, govt);
            }
        }
    }

    /// Credits the player's combat rating for each NPC among `strikes`
    /// that a hit started breaking up, when the kill counts (see
    /// [`rating`](crate::rating)): the shooter is the player or a ship of
    /// its fleet, its escorts and fighters, and the victim's government,
    /// from `was`, is not derelict. As the original's `_DamageShip`
    /// (@0x3ad4f-0x3ae53), the credit comes with the hit, before a
    /// shooter in the fleet can be lost or the victim taken out.
    fn credit_kills(&mut self, strikes: &[Strike], was: &[NpcBefore]) {
        for strike in strikes {
            let (ShipRef::Npc(id), Some(Downed::BreakingUp)) = (strike.ship, strike.downed) else {
                continue;
            };
            let Some(govt) = was
                .iter()
                .find(|before| before.id == id)
                .map(|before| before.govt)
            else {
                continue;
            };
            let by_players_fleet = match strike.by {
                ShipRef::Player => true,
                ShipRef::Npc(by) => self
                    .traffic
                    .npcs()
                    .iter()
                    .any(|npc| npc.id == by && npc.fleet() == ShipRef::Player),
            };
            if !rating::counts(by_players_fleet, self.govts.derelict(govt)) {
                continue;
            }
            let Some(victim) = self.traffic.npcs().iter().find(|npc| npc.id == id) else {
                continue;
            };
            let strength = i64::from(self.arsenal.strength(victim.ship));
            self.pilot.combat_rating = rating::credit(self.pilot.combat_rating, strength);
        }
    }

    /// Lets go of the target once it is no longer in the system (it was
    /// destroyed, landed or jumped out, or the system was populated
    /// afresh) or no longer targetable (it is breaking up, or cloaked and
    /// the player cannot target cloaked ships), as the original does every
    /// frame. A target of an untargetable type is kept: the original's
    /// check (`_HandlePlayer` @0x686a4-0x686ee) never reads the bit.
    fn clear_lost_target(&mut self) {
        let cloaked_ok = self.scanner().targets;
        if self
            .target()
            .is_none_or(|npc| !targeting::targetable(npc, cloaked_ok))
        {
            self.target = None;
        }
    }

    /// Picks the player's target as `pick` says (see
    /// [`targeting`](crate::targeting)), and gives it: the nearest NPC, or
    /// the nearest threat, the target unchanged when there is none; or the
    /// next or the previous. While the ship is landed or jumping, nothing
    /// changes. A pick that changes the target starts its brackets' fly-in
    /// ([`Session::target_brackets`]).
    pub fn select_target(&mut self, pick: TargetPick) -> Option<NpcId> {
        let before = self.target;
        if self.landed.is_none() && self.jumping().is_none() {
            let scanner = self.scanner();
            let npcs = self.traffic.npcs();
            let from = self.player.position;
            match pick {
                TargetPick::Nearest => {
                    let nearest = targeting::nearest(npcs, from, scanner, |_| true);
                    self.target = nearest.or(self.target);
                }
                TargetPick::NearestThreat => {
                    let threat = |npc: &Npc| npc.threatens_player_among(npcs);
                    let nearest = targeting::nearest(npcs, from, scanner, threat);
                    self.target = nearest.or(self.target);
                }
                TargetPick::Next => self.target = targeting::next(npcs, self.target, scanner),
                TargetPick::NextEscort => {
                    self.target = targeting::next_escort(npcs, self.target, scanner);
                }
                TargetPick::Previous => {
                    self.target = targeting::previous(npcs, self.target, scanner);
                }
                TargetPick::PreviousEscort => {
                    self.target = targeting::previous_escort(npcs, self.target, scanner);
                }
            }
        }
        self.retargeted(before);
        self.target
    }

    /// The NPC the player targets, if any.
    #[must_use]
    pub fn target(&self) -> Option<&Npc> {
        let id = self.target?;
        self.npcs().iter().find(|npc| npc.id == id)
    }

    /// The player's ship type's hull: how it breaks up and dies.
    #[must_use]
    pub fn hull(&self) -> HullSpec {
        self.hull
    }

    /// How the player's ship is holding up.
    #[must_use]
    pub fn player_condition(&self) -> Condition {
        self.condition
    }

    /// Where the player's ship is in its lifecycle: its condition, whether
    /// it exploded on the latest fight tick, how far the game's end has
    /// got since, and the escape pod's countdown.
    #[must_use]
    pub fn player_fate(&self) -> PlayerFate {
        PlayerFate::of(
            self.condition,
            self.player_exploded_this_tick,
            self.player_end,
            self.pod,
        )
    }

    /// Whether the session takes the player's `command` now: its
    /// [`PlayerFate`]'s row decides ([`crate::FateRow::input`]), and for
    /// [`TAKEOFF_GRACE_TICKS`] after a take-off or a gate's exit the map
    /// and the mission info are refused too (see the `takeoff_grace`
    /// module). The flight view asks before it acts on a key, and does
    /// nothing when refused.
    #[must_use]
    pub fn takes(&self, command: PlayerCommand) -> bool {
        self.player_fate().row().input.takes(command) && !self.takeoff_grace_refuses(command)
    }

    /// The shots in flight.
    #[must_use]
    pub fn shots(&self) -> &[Shot] {
        self.combat.shots()
    }

    /// The beams being fired.
    #[must_use]
    pub fn beams(&self) -> &[Beam] {
        self.combat.beams()
    }

    /// What has happened in the fight since this was last taken, in order;
    /// taking it empties the list.
    pub fn take_combat_events(&mut self) -> Vec<CombatEvent> {
        self.combat.take_events()
    }

    /// The diagnostics about game data the simulation does not handle yet,
    /// each made once a session, since they were last taken; taking them
    /// empties the list.
    pub fn take_diagnostics(&mut self) -> Vec<SimDiagnostic> {
        let mut diagnostics = self.combat.take_diagnostics();
        diagnostics.extend(self.reports.take());
        diagnostics
    }

    /// The NPCs in the system, in the order they appeared.
    #[must_use]
    pub fn npcs(&self) -> &[Npc] {
        self.traffic.npcs()
    }

    /// The NPCs that left on the last traffic tick, and how.
    #[must_use]
    pub fn departed(&self) -> &[(NpcId, Outcome)] {
        self.traffic.departed()
    }

    /// The ship types the system's traffic can spawn and the escorts
    /// fly, and the fighters the player's bays and theirs launch, by
    /// ascending ID, so a view can read their sprites up front.
    #[must_use]
    pub fn traffic_ships(&self) -> Vec<ShipId> {
        let mut ships = self.traffic.ship_types();
        ships.extend(self.pilot.escorts.iter().map(|escort| escort.ship));
        let mut armaments: Vec<Armament> = ships
            .iter()
            .filter_map(|&ship| self.ship_record(ship))
            .map(|record| {
                let defaults = pilot::tally(record.defaults.iter().copied());
                self.arsenal.of_class(record.id, &defaults, &self.outfits)
            })
            .collect();
        armaments.push(self.armament.clone());
        let launched: Vec<ShipId> = armaments
            .iter()
            .flat_map(Armament::mounts)
            .filter_map(|mount| mount.spec.carried)
            .collect();
        ships.extend(launched);
        ships.sort_unstable();
        ships.dedup();
        ships
    }

    /// Ship type `ship`'s shipyard long name, as its [`ShipRecord`] gives
    /// it, if the session has its record.
    #[must_use]
    pub fn ship_long_name(&self, ship: ShipId) -> Option<&str> {
        let record = self.ships.iter().find(|record| record.id == ship)?;
        Some(&record.long_name)
    }

    /// Ship type `ship`'s name, as its [`ShipRecord`] gives it, if the
    /// session has its record.
    #[must_use]
    pub fn ship_name(&self, ship: ShipId) -> Option<&str> {
        let record = self.ships.iter().find(|record| record.id == ship)?;
        Some(&record.name)
    }

    /// `jünk` `id`'s `LCName`, raw, if the session has its record: what a
    /// scan that finds it calls it.
    #[must_use]
    pub fn junk_lc_name(&self, id: JunkId) -> Option<&str> {
        let record = self.goods.junk().iter().find(|record| record.id == id)?;
        Some(&record.lc_name)
    }

    /// Stops the thrust, if the ship was thrusting, and puts the engine
    /// glow out, even as it fades after the thrust, as the ship lands or
    /// jumps.
    fn stop_thrust(&mut self) {
        self.engine_glow = 0;
        if self.thrusting {
            self.thrusting = false;
            self.sounds.push(SimSound::ThrustStopped);
        }
    }

    /// Plots a course from the system the ship is in to `to`, replacing any
    /// course, and gives it: the session's [`HyperlinkRule`] decides which
    /// links it follows. When there is no route the course is cleared.
    pub fn plot_course(&mut self, to: SystemId) -> Result<&[SystemId], RouteError> {
        match self
            .galaxy
            .map()
            .route(self.pilot.system, to, self.hyperlinks)
        {
            Ok(route) => {
                self.pilot.course = route;
                Ok(&self.pilot.course)
            }
            Err(error) => {
                self.pilot.course.clear();
                Err(error)
            }
        }
    }

    /// The systems still to jump to, in order, ending at the destination;
    /// none when no course is plotted or the destination has been reached.
    #[must_use]
    pub fn course(&self) -> &[SystemId] {
        &self.pilot.course
    }

    /// Presses the jump key: accepts a jump to the next system on the
    /// course, if the ship has not landed and the
    /// [`hyperspace`](crate::hyperspace) rules allow it, and gives that
    /// system; otherwise the refusal says why.
    ///
    /// An accepted jump starts the pre-jump stage: the ship brakes and
    /// turns to the map bearing of that system as [`pre_jump`] says, and
    /// the jump itself begins on the tick it is ready, or here and now if
    /// it is ready already. Once accepted the jump is committed: J again
    /// during the stage gives the same system and changes nothing, without
    /// asking the rules again, so a ship that brakes into its no-jump zone
    /// still jumps. Once it has stopped braking the warp's clock runs (see
    /// [`Session::tick`]), and the warp-up sounds here when the ship needs
    /// no braking. In hyperspace the ship stays where it is until it
    /// arrives; the traffic flies on, and the scans are taken, only
    /// through the warp's lead-in ([`Session::in_warp_lead_in`]).
    /// Only this first hop of a multi-jump turns and slows: the rest are
    /// made in hyperspace, on arrival.
    pub fn begin_jump(&mut self) -> Result<SystemId, JumpRefusal> {
        if self.landed.is_some() {
            return Err(JumpRefusal::Landed);
        }
        if self.condition != Condition::Intact {
            return Err(JumpRefusal::Disabled);
        }
        if let Some(Jump::PreJump { to, .. }) = self.jump {
            return Ok(to);
        }
        let next = self.check_next_jump()?;
        let map = |id| self.galaxy.map().position(id);
        let bearing = map(self.pilot.system)
            .zip(map(next))
            .and_then(|(from, to)| jump_bearing(from, to));
        self.stage_jump(next, bearing, None);
        Ok(next)
    }

    /// The next system on the course, if J would jump there now, or the
    /// first [`JumpRefusal`] that applies: the ship has landed, or
    /// [`check_jump`]'s refusals, with the no-jump zone the session's
    /// [`JumpZoneRule`] gives the system's stellars ([`jump_zone`]). J and
    /// [`Session::jump_readiness`] both ask this, so the rule lives in one
    /// place.
    fn check_next_jump(&self) -> Result<SystemId, JumpRefusal> {
        if self.landed.is_some() {
            return Err(JumpRefusal::Landed);
        }
        let system = self.pilot.system;
        let next = self
            .pilot
            .course
            .first()
            .and_then(|&hop| self.galaxy.same_place(hop))
            .filter(|next| {
                self.galaxy
                    .map()
                    .jumps(system, self.hyperlinks)
                    .contains(next)
            });
        check_jump(
            &self.player,
            self.pilot.reserves.fuel.now,
            next,
            jump_zone(&self.sites, self.stats.jump_distance, self.jump_zone),
        )
    }

    /// Whether the ship can jump to the next system on its course: under
    /// way once J has been accepted, until the ship arrives; otherwise
    /// clear when J would be accepted now, and blocked when it would be
    /// refused. The nav area draws the destination by this.
    #[must_use]
    pub fn jump_readiness(&self) -> JumpReadiness {
        if self.jump.is_some() {
            JumpReadiness::Underway
        } else if self.check_next_jump().is_ok() {
            JumpReadiness::Clear
        } else {
            JumpReadiness::Blocked
        }
    }

    /// Sets the jump to `to` in the stage the ship is now in, on its way
    /// to `bearing`, `warp` the warp's clock this tick (see [`Jump`]):
    /// none while it brakes, and from 0 on the tick it stops braking,
    /// which sounds the warp-up ([`SimSound::JumpBegan`], `snd ` 128,
    /// @0x6b60d-0x6b690). Once the ship is ready to turn no further and
    /// slow no more, the jump begins: the thrust stops, the glow goes out
    /// and the led ships let go of their targets.
    fn stage_jump(&mut self, to: SystemId, bearing: Option<f32>, warp: Option<u32>) {
        let handling = self.flying_handling();
        let stage = pre_jump::stage(&self.player, &handling, bearing, !self.stats.fast_jump);
        let started = warp.is_none() && stage != PreJump::Brake;
        let warp = match stage {
            PreJump::Brake => None,
            PreJump::Turn | PreJump::Ready => Some(warp.unwrap_or(0)),
        };
        match (stage, warp) {
            (PreJump::Ready, Some(warp)) => {
                self.jump = Some(Jump::Hyperspace { to, warp });
                self.stop_thrust();
            }
            _ => self.jump = Some(Jump::PreJump { to, bearing, warp }),
        }
        if started {
            self.sounds.push(SimSound::JumpBegan);
        }
        if self.jumping().is_some() {
            self.let_go_of_led_targets();
        }
    }

    /// The warp's clock: the engine's ticks since the warp stage began,
    /// if it has (see [`Jump`]).
    fn warp_clock(&self) -> Option<u32> {
        match self.jump? {
            Jump::PreJump { warp, .. } => warp,
            Jump::Hyperspace { warp, .. } => Some(warp),
        }
    }

    /// Whether `_ScanPlayer`'s jump-timing check holds a scan back now
    /// ([`warp::held`]): the warp is past its lead-in, by its clock as Mac
    /// ticks, the warp factor of the player's ship class and the warp
    /// sound's duration, and the player is not disabled (`_IsDisabled`,
    /// whose armour test also holds for a ship breaking up).
    fn warp_holds_scans(&self) -> bool {
        let flags = self
            .ship_record(self.pilot.ship)
            .map_or(0, |record| record.flags);
        warp::held(
            self.warp_clock().map(warp::mac_ticks),
            warp::warp_factor(flags),
            self.warp_sound,
            self.condition != Condition::Intact,
        )
    }

    /// Whether the jump has begun ([`Session::jumping`]) and its warp is
    /// still in its lead-in, where the player is scanned as before the
    /// warp: the traffic flies on and the scans are taken until the
    /// lead-in ends, then both stand still until the ship arrives. A
    /// disabled player's warp never ends its lead-in (see the `warp`
    /// module).
    #[must_use]
    pub fn in_warp_lead_in(&self) -> bool {
        self.jumping().is_some() && !self.warp_holds_scans()
    }

    /// The NPCs the traffic holds still while the player's jump is under
    /// way: none before it begins; once it has, the ships the player leads
    /// ([`Session::player_led`]) and the fighters they lead, which keep
    /// their place beside the player, as the original's led ships time
    /// their warp-out by the player's clock (`_HandleShip`
    /// @0x347c1-0x348bd) and do not accelerate before t > 0.45·d/w²,
    /// after the lead-in.
    fn held_through_the_warp(&self) -> BTreeSet<NpcId> {
        if self.jumping().is_none() {
            return BTreeSet::new();
        }
        let led: BTreeSet<NpcId> = self.player_led().map(|npc| npc.id).collect();
        self.traffic
            .npcs()
            .iter()
            .filter(|npc| {
                led.contains(&npc.id) || npc.leader.is_some_and(|leader| led.contains(&leader))
            })
            .map(|npc| npc.id)
            .collect()
    }

    /// The system being jumped to, once the jump itself has begun and
    /// until the ship arrives; `None` during the pre-jump stage.
    #[must_use]
    pub fn jumping(&self) -> Option<SystemId> {
        match self.jump {
            Some(Jump::Hyperspace { to, .. }) => Some(to),
            _ => None,
        }
    }

    /// The system a jump is bound for while the ship brakes and turns
    /// before it (see [`Session::begin_jump`]); `None` before J and once
    /// the jump has begun.
    #[must_use]
    pub fn preparing_jump(&self) -> Option<SystemId> {
        match self.jump {
            Some(Jump::PreJump { to, .. }) => Some(to),
            _ => None,
        }
    }

    /// Ends the jump under way, if any, and gives the system arrived in.
    /// `None`, and nothing changes, when no jump has begun, during the
    /// pre-jump stage too.
    ///
    /// The jump makes as many hops along the course as
    /// [`hops_per_jump`] gives for the ship's multi-jump total and the
    /// session's [`MultiJumpRule`]: one without a multi-jump outfit. It
    /// chains only when its first hop was to the course's next system (a
    /// course replotted during the jump is not followed), and stops early
    /// where the course ends. Under [`MultiJumpRule::Engine`] the whole
    /// chain uses one jump's fuel and the days the stats give one jump;
    /// under [`MultiJumpRule::PerHop`] every hop does, and the chain stops
    /// at a hop the ship has not the fuel for. Each day steps the planetary
    /// events, rolled on `chance`.
    ///
    /// The systems hopped are taken off the course. Those passed through
    /// on the way are not entered; only the last is: explored, its
    /// stellars read from `catalog`, the navigation target cleared, and
    /// the ship placed just outside its no-jump zone, at rest on the side
    /// facing the system it last left (see [`arrival`]), with its reserves
    /// as they were, so it can jump on at once. It raises
    /// [`SimMessage::Arrived`] for that last system. The system arrived in
    /// is populated with its traffic ([`Session::populate`]), the last
    /// system's gone, and so are the shots and beams in flight; the
    /// fighters out are carried or left behind as the fighter rules say.
    ///
    /// It enters that system as every way in does, by its row of the
    /// table in the `entry` module, which also lets go of the strikes, the
    /// target, the boarding, the hail and a held `P` sound.
    ///
    /// Before its days it raises [`StoryEvent::Explored`] for each system
    /// passed and the one arrived in, as the original explores their
    /// nebulae first; [`StoryEvent::EnteredSystem`] follows the days.
    ///
    /// It refreshes the galaxy (`_PropagateMissionBitEffects` @0x6bf3f):
    /// every system's `Visibility`, and every nebula's `ActiveOn`,
    /// evaluated before the days, so a bit
    /// their scripts set shows only at the next refresh, and written once
    /// the hops and the systems explored are resolved by the flags as they
    /// stood, before the system is entered, telling
    /// [`SessionEvent::GalaxyChanged`] when it changed anything.
    ///
    /// Its first draw on `chance` arms the escort-strength timer (see
    /// [`strength`]), before the days, the roll
    /// table and the population, as the original does (@0x6c013).
    pub fn arrive(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<SystemId> {
        let Some(Jump::Hyperspace { to: first, .. }) = self.jump else {
            return None;
        };
        self.jump = None;
        let mark = self.story_mark();
        // The galaxy refresh evaluated before the days, so a bit their
        // scripts set waits for the next refresh (@0x6bf3f, days
        // @0x6c144).
        let refresh = self.refresh_now();
        // The escort-strength timer armed before the roll table, the days
        // and the setup (@0x6c013).
        self.arm_strength_timer(chance);
        // The Red Alert's icon hidden, its threat kept (@0x6c024-0x6c046).
        self.red_alert.clear_icon();
        self.take_jump_cost(chance);
        let (mut from, mut at) = (self.pilot.system, first);
        let mut hops = 0;
        // The systems passed on the way, each hop the visible system that
        // stands for it, as the original resolves them (@0x6bd67-0x6bdc1).
        let mut passed = Vec::new();
        if self
            .pilot
            .course
            .first()
            .is_some_and(|&hop| hop == first || self.galaxy.same_place(hop) == Some(first))
        {
            hops = 1;
            for _ in 1..hops_per_jump(self.stats.multi_jump, self.multi_jump) {
                let next = self.pilot.course.get(hops);
                let Some(next) = next.and_then(|&hop| self.galaxy.same_place(hop)) else {
                    break;
                };
                if self.multi_jump == MultiJumpRule::PerHop {
                    if self.pilot.reserves.fuel.now < JUMP_FUEL {
                        break;
                    }
                    self.take_jump_cost(chance);
                }
                hops += 1;
                passed.push(at);
                (from, at) = (at, next);
            }
        }
        // The systems passed, then the one arrived in, each reached before
        // the days (@0x6bddb, @0x6be12): the one entering it will resolve
        // to, should the galaxy have hidden it on the way.
        let arrived = self.galaxy.arrival(at).to;
        for (n, system) in passed.into_iter().chain([arrived]).enumerate() {
            self.raise_at(mark + n, StoryEvent::Explored { system });
        }
        let map = |id| self.galaxy.position(id).unwrap_or_default();
        let entered = Arrival {
            system: at,
            sites: self.read_sites(catalog, at),
            placement: Placement {
                ship: ShipPlacement::Arrive(arrival(map(from), map(at), self.stats.jump_distance)),
                stellar: None,
                hold: false,
            },
        };
        // The refresh written, with the hops and the systems explored
        // resolved by the flags as they stood (@0x6bd67-0x6bdd4,
        // @0x6be0b). The player is between systems: its entry resolves the
        // arrival by the same rule as any move out of a hidden system.
        if self
            .write_galaxy(|overlay| {
                refresh.write(overlay);
            })
            .is_some()
        {
            self.emit(SessionEvent::GalaxyChanged);
        }
        self.pilot.jumped_from = Some(from);
        self.enter_system(Entry::Jump { hops }, entered, catalog, catalog, chance);
        // The arrival's reset, after the days, the enter-system hook and
        // the setup (@0x6c51f; see the `precalced` module).
        self.reset_precalced();
        Some(self.pilot.system)
    }

    /// Arms the escort-strength timer on `chance`, as a take-off, a gate
    /// exit and a hyperspace arrival do (see
    /// [`strength`]).
    fn arm_strength_timer(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.traffic.arm_strength_timer(chance);
    }

    /// Uses a jump's fuel and lets its days go by ([`Session::pass_jump_days`]).
    fn take_jump_cost(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.pilot.reserves.fuel.now -= JUMP_FUEL;
        self.pass_jump_days(chance);
    }

    /// Lets the days the stats give a jump go by through the daily step
    /// ([`Session::pass_days`]), the events rolled on `chance`, then pays
    /// the hired escorts their wages for them, once.
    fn pass_jump_days(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.pass_days(self.stats.jump_days, chance);
        self.pay_escorts(self.stats.jump_days);
    }

    /// Makes the stellar `pick` finds the navigation target, as the
    /// original's stellar slot keys, F5 and a click in space do, and
    /// gives it. The course is left alone: the original keeps its route
    /// and only moves its selection. `None`, and nothing changes, during
    /// a jump (pre-jump stage too, as the original skips its nav inputs
    /// while jumping) or when `pick` finds nothing: an empty slot, no
    /// landable stellar, or a click on none. Picking a stellar other than
    /// the target starts its brackets' fly-in ([`Session::nav_brackets`]);
    /// picking the target again doesn't (@0x69ec4-0x69ed6,
    /// @0x6a68f-0x6a69d).
    pub fn select_stellar(&mut self, pick: StellarPick) -> Option<StellarId> {
        if self.jump.is_some() {
            return None;
        }
        let picked = match pick {
            StellarPick::Slot(slot) => self.sites.get(slot).map(|site| site.id),
            StellarPick::Nearest => nearest_landable(self.player.position, &self.sites),
            StellarPick::At(at) => {
                stellar_under(&self.sites, at, |site| self.drawn_frame_size(site))
            }
        }?;
        if self.nav_target != Some(picked) {
            self.nav_fly_in.start();
        }
        self.nav_target = Some(picked);
        Some(picked)
    }

    /// Clears the navigation target, as the original's Nav Off key does
    /// (`_HandlePlayer` @0x69a17-0x69b63), and says whether there was one
    /// to clear. Nothing changes during a jump (pre-jump stage too).
    pub fn clear_nav_target(&mut self) -> bool {
        if self.jump.is_some() {
            return false;
        }
        self.nav_target.take().is_some()
    }

    /// Presses Hyper Select (the original's `\` key): plots a one-jump
    /// course to the next system the session's [`HyperSelectRule`] cycles
    /// to ([`next_hyper_destination`]) and gives it, clearing the stellar
    /// navigation target, as the original switches its nav selection to
    /// hyperspace. `None`, and nothing changes, during a jump (pre-jump
    /// stage too) or when the system has no links to offer.
    ///
    /// The original keeps the plotted route and only moves its selection,
    /// so cycling back to the route's first hop there keeps the whole
    /// route; here the course is the jump target, so a press always
    /// leaves a one-jump course.
    pub fn select_next_system(&mut self) -> Option<SystemId> {
        if self.jump.is_some() {
            return None;
        }
        let next = next_hyper_destination(
            self.galaxy.map(),
            self.pilot.system,
            &self.course_shown(),
            self.nav_target.is_some(),
            self.hyper_select,
            self.hyperlinks,
        )?;
        self.pilot.course = vec![next];
        self.nav_target = None;
        Some(next)
    }

    /// The stellar selected as the navigation target, if any.
    #[must_use]
    pub fn nav_target(&self) -> Option<StellarId> {
        self.nav_target
    }

    /// The sounds emitted since they were last taken, in order; taking
    /// them empties the list.
    pub fn take_sounds(&mut self) -> Vec<SimSound> {
        std::mem::take(&mut self.sounds)
    }

    /// The messages raised since they were last taken, in order; taking
    /// them empties the list.
    pub fn take_messages(&mut self) -> Vec<SimMessage> {
        std::mem::take(&mut self.messages)
    }

    /// The star map every reader sees: the catalog's, read when the
    /// session started, with the galaxy overlay applied (see
    /// [`Session::galaxy`]).
    #[must_use]
    pub fn star_map(&self) -> &StarMap {
        self.galaxy.map()
    }

    /// The galaxy every reader sees: the catalog's, read when the session
    /// started, with the pilot's overlay applied (see the
    /// [`story`](crate::story) module's "Galaxy overlay").
    #[must_use]
    pub fn galaxy(&self) -> &OverlaidGalaxy {
        &self.galaxy
    }

    /// `system`'s stellars, read from `catalog`, each with the galaxy
    /// overlay's government for it: the one way the session reads them.
    fn read_sites(
        &self,
        catalog: &(impl PilotCatalog + ?Sized),
        system: SystemId,
    ) -> Vec<LandingSite> {
        overlaid_sites(&self.galaxy, catalog, system)
    }

    /// Today's date.
    #[must_use]
    pub fn date(&self) -> GameDate {
        self.pilot.date
    }

    /// Today's date as it is displayed, with the first `chär`'s prefix and
    /// suffix: for example "June 23, 1177 NC".
    #[must_use]
    pub fn date_text(&self) -> String {
        date::date_text(self.pilot.date, &self.date_affixes)
    }

    /// Today's date as the mission screens show it, the original's medium
    /// date with the first `chär`'s prefix and suffix
    /// ([`date::med_date_text`]): for example "June 23rd, 1177 NC".
    #[must_use]
    pub fn med_date_text(&self) -> String {
        date::med_date_text(self.pilot.date, &self.date_affixes)
    }

    /// The fuel the ship gains each tick in flight.
    #[must_use]
    pub fn fuel_regen_per_tick(&self) -> f32 {
        self.stats.fuel_regen
    }

    /// Presses the land key, unless a jump is under way, its pre-jump stage
    /// included: as the
    /// [`landing`](crate::landing) rules say, with the navigation target
    /// and traffic control's clearance
    /// ([`landing::cleared`](crate::landing::cleared)) of the pilot's
    /// standing: its legal record, the `Require` its ranks meet, its
    /// missions' stellars and the blessing table (see the `ranks` module),
    /// read as the session's `blessing_landing` rule says. A press that
    /// selects a stellar makes it the navigation target, starting its
    /// brackets' fly-in (@0x6a932), and lands on nothing. A press that
    /// lands docks the ship at the stellar's centre, at rest, its heading
    /// and reserves unchanged, and clears the target, so that once it
    /// takes off L requests clearance again. Otherwise the ship flies on,
    /// its target kept, and the refusal says why.
    ///
    /// A press that would land on a hypergate or wormhole
    /// ([`GateKind::of`] its `Flags2`) enters it instead: it does not dock
    /// and makes no sound, and gives [`LandPress::AtGate`]. The entry
    /// then awaits [`Session::enter_hypergate`] or
    /// [`Session::enter_wormhole`], until the next tick.
    ///
    /// A disabled ship's press goes as far as `_HandlePlayerDockRequest`
    /// does for it and no further: with no target it selects one, starting
    /// its fly-in, as the land key does before the dock request
    /// (@0x6a8fb-0x6a932); a target that can't be landed on, or none, is
    /// refused as an intact ship's is (@0x66977-0x669aa). Then the dock
    /// request's `_IsDisabled` test (@0x66af7) goes to its bare epilogue
    /// (0x67eef): [`LandingRefusal::Disabled`], said nothing of, before
    /// the range and speed checks (@0x67838) and the clearance
    /// (@0x67903-0x67a05), so the selection gives no clearance reply, and
    /// the ship neither docks nor enters a gate. A breaking-up or
    /// destroyed ship's press is refused the same way before it selects
    /// anything, as the land key's breakup-timer gate refuses it
    /// (@0x6a8cb).
    pub fn land(&mut self) -> Result<LandPress, LandingRefusal> {
        if self.jump.is_some() {
            return Err(LandingRefusal::Jumping);
        }
        let disabled = match self.condition {
            Condition::Intact => false,
            Condition::Disabled => true,
            Condition::Dying { .. } | Condition::Destroyed => {
                return Err(LandingRefusal::Disabled);
            }
        };
        let blessing = self.blessing();
        let reading = self.rules.source_for(RuleKey::BlessingLanding);
        let outcome = land_or_select(
            self.nav_target,
            &self.player,
            &self.sites,
            |site| crate::landing::cleared(site, self.standing(site, &blessing), reading),
            |site| self.landing_reach(site),
        );
        if disabled {
            return match outcome {
                Ok(LandOutcome::Selected { stellar, .. }) => {
                    self.nav_target = Some(stellar);
                    self.nav_fly_in.start();
                    Err(LandingRefusal::Disabled)
                }
                Err(
                    refusal @ (LandingRefusal::NotLandable { .. } | LandingRefusal::NoStellars),
                ) => Err(refusal),
                Ok(LandOutcome::Landed(_)) | Err(_) => Err(LandingRefusal::Disabled),
            };
        }
        let outcome = outcome?;
        match outcome {
            LandOutcome::Selected { stellar, .. } => {
                self.nav_target = Some(stellar);
                self.nav_fly_in.start();
            }
            LandOutcome::Landed(stellar) => {
                if let Some(kind) = self.gate_kind(stellar) {
                    self.gate = Some(stellar);
                    return Ok(LandPress::AtGate { stellar, kind });
                }
                self.dock(stellar);
            }
        }
        Ok(outcome.into())
    }

    /// What `stellar`, one of the system's, leads through, if it is a
    /// hypergate or wormhole.
    #[must_use]
    pub fn gate_kind(&self, stellar: StellarId) -> Option<GateKind> {
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        GateKind::of(site.flags2)
    }

    /// The pending entry, if it is through a gate of `kind`; it ends here
    /// whatever it is.
    fn take_gate(&mut self, kind: GateKind) -> Option<StellarId> {
        self.gate
            .take()
            .filter(|&stellar| self.gate_kind(stellar) == Some(kind))
    }

    /// Every system listing `stellar`, by ascending ID ([`listers_of`]):
    /// what its owner and the systems resolving it are derived from.
    fn listers(&self, stellar: StellarId) -> &[SystemId] {
        listers_of(&self.listed_stellars, stellar)
    }

    /// The system that holds `stellar` now, with the player where they
    /// are ([`OverlaidGalaxy::holder`](crate::story::OverlaidGalaxy::holder)):
    /// the stellar's +0x14 as the aux and mission ship readers find it.
    fn holder_of(&self, stellar: StellarId) -> Option<SystemId> {
        self.galaxy.holder(self.listers(stellar), self.pilot.system)
    }

    /// The links of `stellar` among `sites`; none when no system lists it.
    fn links_of(sites: &[GateSite], stellar: StellarId) -> [Option<StellarId>; 8] {
        sites
            .iter()
            .find(|site| site.id == stellar)
            .map_or([None; 8], |site| site.links)
    }

    /// Decides whether the hypergate whose entry is pending opens the
    /// hypergate map, read from `catalog`'s gate sites. A hypergate with
    /// links ([`has_links`], as `_StellarNumHyperLinks` counts them) offers
    /// the systems its links lead to, each as the visible system standing
    /// for it on the overlaid map ([`gate::hypergate_offers`]), for
    /// the player to choose among, and the entry awaits
    /// [`Session::enter_hypergate`]: even when none leads anywhere, the
    /// map opens with nothing to pick, and closing it cancels. One without
    /// links is refused ([`GateRefusal::NoLinks`]), which ends the entry
    /// and clears the navigation target, as [`Session::enter_hypergate`]
    /// refuses. [`GateRefusal::NotAtGate`] when no hypergate entry is
    /// pending.
    pub fn open_hypergate(
        &mut self,
        catalog: &impl PilotCatalog,
    ) -> Result<Vec<SystemId>, GateRefusal> {
        let here = self
            .gate
            .filter(|&stellar| self.gate_kind(stellar) == Some(GateKind::Hypergate))
            .ok_or(GateRefusal::NotAtGate)?;
        let sites = catalog.gate_sites();
        let links = Self::links_of(&sites, here);
        if has_links(&links) {
            let system_of = |id| self.galaxy.system_of(self.listers(id), self.pilot.system);
            Ok(gate::hypergate_offers(
                &links,
                &sites,
                system_of,
                self.galaxy.map(),
            ))
        } else {
            self.gate = None;
            self.nav_target = None;
            Err(GateRefusal::NoLinks)
        }
    }

    /// Enters the hypergate whose entry is pending, for the system `to`
    /// the player picked on the map (`None` for no pick), and gives the
    /// system the ship comes out in: through the first of its links that
    /// leads there ([`hypergate_exit`]), read from `catalog`. Any outcome
    /// ends the entry; a refusal also clears the navigation target, as the
    /// original's does, and otherwise changes nothing. See
    /// [`Session::enter_wormhole`] for the arrival.
    pub fn enter_hypergate(
        &mut self,
        to: Option<SystemId>,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        chance: &mut (impl Chance + ?Sized),
    ) -> Result<SystemId, GateRefusal> {
        let here = self
            .take_gate(GateKind::Hypergate)
            .ok_or(GateRefusal::NotAtGate)?;
        let sites = catalog.gate_sites();
        let links = Self::links_of(&sites, here);
        let system_of = |id| self.galaxy.system_of(self.listers(id), self.pilot.system);
        let exit = if has_links(&links) {
            to.and_then(|to| {
                let exit = hypergate_exit(&links, &sites, to, system_of, self.galaxy.map())?;
                Some((to, *exit))
            })
            .ok_or(GateRefusal::Cancelled)
        } else {
            Err(GateRefusal::NoLinks)
        };
        match exit {
            Ok((system, exit)) => {
                self.come_out(system, &exit, GateKind::Hypergate, catalog, chance);
                let system = self.pilot.system;
                self.messages.push(SimMessage::ExitedHypergate(system));
                Ok(system)
            }
            Err(refusal) => {
                self.nav_target = None;
                Err(refusal)
            }
        }
    }

    /// Enters the wormhole whose entry is pending and gives the system the
    /// ship comes out in: through the wormhole [`wormhole_exit`] rolls on
    /// `chance`, under the session's [`WormholeRule`] (the engine's unless
    /// [`Session::with_wormholes`] says otherwise), read from `catalog`.
    /// Any outcome ends the entry; a refusal changes nothing else.
    ///
    /// Out of a hypergate or wormhole the ship comes out as the session's
    /// [`GateArrivalRule`] says (the engine's unless
    /// [`Session::with_gate_arrival`] says otherwise): by the engine's at
    /// the exit gate, as [`emerge`] says, with no day passing; like a jump,
    /// where a jump between the two systems arrives, with a jump's days
    /// passing, each stepping the planetary events on `chance`. Either way
    /// no fuel is used, the minimum jump distance does not count, and the
    /// ship is in the new system, explored, its stellars read, with the
    /// course and the navigation target cleared, its thrust stopped,
    /// populated with its traffic ([`Session::populate`]), even when it is
    /// the system gone in, the fight left behind and
    /// [`SimSound::Arrived`]. It raises [`SimMessage::PassedWormhole`] or
    /// [`SimMessage::ExitedHypergate`], and no [`SimMessage::Arrived`].
    /// The ship enters the system by the gate's row of the table in the
    /// `entry` module, which keeps a held `P` sound and lets go of the
    /// strikes, the target, the boarding and the hail. Just before, after
    /// the exit's roll or the days, the escort-strength timer is armed on
    /// `chance` (see [`strength`]), as a gate's exit joins the take-off's
    /// path in the original (`_HandlePlayerDockRequest` @0x66f40,
    /// @0x6706f). The special ships following the player are decided
    /// next, as [`RuleKey::GateFollow`] says (see the `gate_follow`
    /// module): those that follow come out of the exit gate after the
    /// player, or where it arrived like a jump. Last, every ship the
    /// player leads comes out of the exit gate after it, as
    /// [`RuleKey::GateEscorts`] says (see the `gate_escorts` module). The
    /// exit then starts the take-off grace, as the take-off's path ends
    /// (see the `takeoff_grace` module).
    pub fn enter_wormhole(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        chance: &mut (impl Chance + ?Sized),
    ) -> Result<SystemId, GateRefusal> {
        let here = self
            .take_gate(GateKind::Wormhole)
            .ok_or(GateRefusal::NotAtGate)?;
        let sites = catalog.gate_sites();
        let player = self.pilot.system;
        let owner_of = |id| self.galaxy.holder(self.listers(id), player);
        let exit = wormhole_exit(here, player, &sites, owner_of, self.wormholes, chance)
            .ok_or(GateRefusal::NoExit)?;
        let (site, system) = (*exit.site, exit.system);
        self.come_out(system, &site, GateKind::Wormhole, catalog, chance);
        let system = self.pilot.system;
        self.messages.push(SimMessage::PassedWormhole(system));
        Ok(system)
    }

    /// Brings the ship out of `exit`, a gate of `kind`, in `system`: see
    /// [`Session::enter_wormhole`]. Before it enters, the exit decides
    /// what becomes of the special ships following the player
    /// ([`Session::exit_gate_slots`]); once it has, those that follow come
    /// out of the gate after it ([`Session::emerge_followers`]), and then
    /// every ship the player leads ([`Session::lead_out_of_gate`]).
    fn come_out(
        &mut self,
        system: SystemId,
        exit: &GateSite,
        kind: GateKind,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        chance: &mut (impl Chance + ?Sized),
    ) {
        let ship = match self.gate_arrival {
            GateArrivalRule::Engine => emerge(exit, self.top_speed(), chance),
            GateArrivalRule::LikeJump => {
                let map = |id| self.galaxy.position(id).unwrap_or_default();
                let placed = arrival(
                    map(self.pilot.system),
                    map(system),
                    self.stats.jump_distance,
                );
                self.pass_jump_days(chance);
                placed
            }
        };
        // A gate's exit joins the take-off's path before its seed
        // (`_HandlePlayerDockRequest` @0x66f40, @0x6706f), and its reset
        // (@0x6709b; see the `precalced` module).
        self.arm_strength_timer(chance);
        self.reset_precalced();
        self.exit_gate_slots(kind, catalog, &mut &mut *chance);
        let entered = Arrival {
            system,
            sites: self.read_sites(catalog, system),
            placement: Placement {
                ship: ShipPlacement::Arrive(ship),
                stellar: None,
                hold: false,
            },
        };
        self.enter_system(Entry::Gate, entered, catalog, catalog, chance);
        self.emerge_followers(exit.position, catalog, chance);
        self.lead_out_of_gate(exit.position, chance);
        // The tail the take-off's path ends in (@0x67673; see the
        // `takeoff_grace` module).
        self.start_takeoff_grace();
    }

    /// Docks the ship at `stellar`, one of the system's, takes the ranks'
    /// price multipliers as the spaceport opens (see the `ranks` module),
    /// and last raises [`StoryEvent::Landed`].
    fn dock(&mut self, stellar: StellarId) {
        self.nav_target = None;
        self.touchdown = self.player.position;
        let site = self.sites.iter().find(|site| site.id == stellar);
        if let Some(site) = site {
            self.player.position = site.position;
        }
        let stellar_sound = site.and_then(|site| site.landing_sound);
        self.player.velocity = Vec2::ZERO;
        self.landed = Some(stellar);
        self.pilot.stellar = Some(stellar);
        self.bar_news = None;
        self.combat.clear();
        // `_HandlePlayerDockRequest` sets `_killBoxes` (@0x66f40).
        self.clear_boxes();
        self.target = None;
        self.talk = None;
        self.leave_with_fighters(true);
        self.land_day_rolls();
        self.open_outfit_visit();
        self.stay = Stay::default();
        self.save_due = true;
        self.stop_thrust();
        // The Red Alert's icon hidden, its threat kept (@0x66d84-0x66da0).
        self.red_alert.clear_icon();
        self.sounds.push(SimSound::Landed { stellar_sound });
        // `_PlayerLandOnStellar`'s reset (@0x63211; see the `precalced`
        // module), before the port opens.
        self.reset_precalced();
        // `_DoPortDialog` takes the price multipliers before its mission
        // land check (@0x5fbeb) and its rebuild point (see the `ranks`
        // module).
        self.open_port_prices();
        self.raise(StoryEvent::Landed { stellar });
        // The port dialog's rebuild point (@0x5fbf0).
        self.rebuild_stellars();
    }

    /// What a landing, or a relocation, does to the day rolls, as
    /// [`RuleKey::DayRolls`] reads. By the engine, it keeps them, as no
    /// landing path in the original writes one, and only the classes
    /// bought or declined since the shipyard was last listed are
    /// forgotten, to be drawn again when next asked: the original draws
    /// them again at once (`_DoShipyardDialog` @0x5f0f8). By the other
    /// reading, it forgets every roll.
    fn land_day_rolls(&mut self) {
        if self.rules.source_for(RuleKey::DayRolls) == RuleSource::Engine {
            for ship in std::mem::take(&mut self.ship_redraws) {
                self.ship_rolls.redraw(&ship);
            }
        } else {
            self.hire_rolls.clear();
            self.outfit_rolls.clear();
            self.ship_rolls.clear();
            self.ship_redraws.clear();
        }
    }

    /// Takes off from the stellar the ship is docked at, and gives it; the
    /// ship flies again from the stellar's centre, at rest, the system is
    /// explored, and the system is populated afresh from `catalog`
    /// ([`Session::populate`]), the pre-landing NPCs gone and the escorts
    /// among the new ones, before `take_off` returns: after the stay's
    /// days and the player's system are set, and before the take-off's
    /// story events dispatch, as the original deletes its computer ships
    /// and sets the system up before the first frame
    /// (`_DeleteAllComputerShips` @0x67096, `_SetupShipsInSystem`
    /// @0x674c2). After a landed move the ship flies in
    /// the system moved to, from where the move left it, and after an `N`
    /// from where it touched down (see the `script_effects` module): the
    /// take-off shows the stellars of the player's system that waited for
    /// it while the ship was docked at another system's stellar
    /// ([`Session::set_player_system`]), telling
    /// [`SessionEvent::SystemChanged`] when they are another system's than
    /// the stellars they replace. `None`, and nothing changes, when it has
    /// not landed.
    ///
    /// It raises a [`StoryEvent::DayPassed`] for each day, then
    /// [`StoryEvent::Explored`] for the system, then
    /// [`StoryEvent::TookOff`], the original's order.
    ///
    /// Leaving passes the stay's days through the daily step, the events
    /// rolled on `chance`, as [`RuleKey::StayDays`] reads: by the engine,
    /// half as many days as escorts sold or upgraded, then the day's wage
    /// [`RuleKey::TakeOffPay`] reads, then a day if an outfit was bought
    /// or sold, four if a ship was bought, and the take-off's own day
    /// (`_DoEscortLand` @0x40849, `_DoPortDialog` @0x601fa-0x6020e,
    /// `_PlayerLandOnStellar` @0x63440). The wage is a day's whatever the
    /// days. After every day each active mission's auxiliary ships are
    /// restarted on `chance` (@0x634df-0x6353c; see the `aux_ships`
    /// module), then the escort-strength timer is armed on it (see
    /// [`strength`], @0x6706f), and the population rolls on it last.
    ///
    /// The take-off starts the take-off grace: for
    /// [`TAKEOFF_GRACE_TICKS`] the map, mission info and player info keys
    /// are refused (see the `takeoff_grace` module).
    pub fn take_off(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<StellarId> {
        let stellar = self.landed.take()?;
        let stay = std::mem::take(&mut self.stay);
        self.bar_news = None;
        let stay_days = self.rules.source_for(RuleKey::StayDays) == RuleSource::Engine;
        if stay_days {
            self.pass_days(stay.escort_days(), chance);
        }
        if self.rules.source_for(RuleKey::TakeOffPay) == RuleSource::Engine {
            self.pay_escorts(1);
        }
        if stay_days {
            self.pass_days(stay.leaving_days(), chance);
        }
        // The dock request's tail (@0x67673; see the `takeoff_grace`
        // module).
        self.start_takeoff_grace();
        if let Some(sites) = self.next_sites.take()
            && self.set_player_system(self.pilot.system, sites, Dock::AsItWas)
        {
            self.emit(SessionEvent::SystemChanged);
        }
        if std::mem::take(&mut self.hold_position) {
            self.player.position = self.touchdown;
        }
        // With its place's head, by the engine (`_PlayerLandOnStellar`
        // @0x63457-0x634a7; see the `explored` module).
        self.explore_place(self.pilot.system);
        self.raise(StoryEvent::Explored {
            system: self.pilot.system,
        });
        self.restock_fleet();
        // The escort-strength timer armed after every day of the stay
        // (@0x6706f), then the old computer ships deleted and the system
        // set up afresh before the first frame (`_DeleteAllComputerShips`
        // @0x67096, `_SetupShipsInSystem` @0x674c2), the escorts among
        // them. The take-off's restart of the auxiliary ships comes
        // before both (@0x634df-0x6353c).
        self.restart_every_aux(&mut &mut *chance);
        self.arm_strength_timer(chance);
        // The dock request's reset, after the stay's days and before the
        // setup (@0x6709b; see the `precalced` module).
        self.reset_precalced();
        self.populate(catalog, chance);
        self.sounds.push(SimSound::TookOff);
        self.save_due = true;
        self.raise(StoryEvent::TookOff { stellar });
        // The dock request's rebuild point (@0x674c7, @0x67653).
        self.rebuild_stellars();
        Some(stellar)
    }

    /// Changes the pilot with `change`, while the ship is landed (in the
    /// spaceport), and says whether it did: in flight nothing changes and
    /// `change` is not called. A save is due after a change. `change` must
    /// not change the ship class, which only [`Session::buy_ship`],
    /// [`Session::assign`] and the ship-change set operators change, nor
    /// the outfits, which only they, [`Session::outfit`] and the other
    /// set operators change, so the fields and the stats follow. A ship
    /// purchase does not go through it: [`Session::buy_ship`] applies its
    /// purchase through `Session::become_class`.
    pub fn transact(&mut self, change: impl FnOnce(&mut Pilot)) -> bool {
        if self.landed.is_none() {
            return false;
        }
        change(&mut self.pilot);
        self.save_due = true;
        true
    }

    /// The exchange of the stellar the ship is docked at, if it has landed
    /// at a trade center, priced by the stellar's [`Markup`].
    #[must_use]
    pub fn market(&self) -> Option<Market> {
        let stellar = self.landed?;
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        market::market(&market::Exchange {
            goods: &self.goods,
            stellar,
            flags: site.flags,
            pilot: &self.pilot,
            capacity: self.capacity(),
            gate: self.gate(),
            rules: self.rules,
            markup: self.markup(site),
        })
    }

    /// The markup at `site`, one of the system's: by whether it has a
    /// government, the player's legal record in the system (with the
    /// system's government, 0 in an independent system), and whether it
    /// is dominated ([`Markup::of`]).
    ///
    /// The port has no domination state yet (task `planet-domination`):
    /// the only dominated stellars are those whose `Flags2` has
    /// [`DOMINATED`](market::DOMINATED), so a stellar the player dominates
    /// by hailing is not yet priced at 1.5. That task feeds its dominated
    /// state in here.
    fn markup(&self, site: &LandingSite) -> Markup {
        Markup::of(
            site.govt.is_some(),
            self.system_record(),
            site.flags2 & market::DOMINATED != 0,
        )
    }

    /// The player's legal record in the current system, as the
    /// [`LegalRecords`](RuleKey::LegalRecords) rule reads it: by the
    /// engine the system's own; by the Bible the record with its
    /// government, 0 in an independent system.
    fn system_record(&self) -> i16 {
        match self.rules.source_for(RuleKey::LegalRecords) {
            RuleSource::Engine => self.pilot.system_record(self.pilot.system),
            RuleSource::Bible => self
                .galaxy
                .govt(self.pilot.system)
                .map_or(0, |govt| self.pilot.legal_record(govt)),
        }
    }

    /// What traffic control weighs of the pilot at `site` (see
    /// [`landing::cleared`](crate::landing::cleared)), its government
    /// blessing as `blessing` says: the legal record
    /// ([`Session::landing_record`]), whether the pilot's ranks contribute
    /// every `Require` bit of `site`'s government, whether `site` is an
    /// active mission's travel or return stellar, compared as raw IDs over
    /// every slot, failed ones included (@0x67996-0x679b4), and whether its
    /// government blesses landing.
    fn standing(&self, site: &LandingSite, blessing: &crate::rank::Blessing) -> Standing {
        let require = self.govts.require(site.govt);
        let mission_stellar = self.pilot.missions().iter().any(|(_, active)| {
            [active.destination, active.return_to]
                .into_iter()
                .flatten()
                .any(|stop| stop.stellar == site.id)
        });
        Standing {
            record: self.landing_record(site),
            meets_require: self.contributed() & require == require,
            mission_stellar,
            blessed: blessing.blesses(site.govt, crate::rank::Blessed::Landing),
        }
    }

    /// The player's legal record that `site`'s `MinStatus` is checked
    /// against: by the engine the record in the current system, whatever
    /// the stellar's government (`_HandlePlayerDockRequest` @0x67903); by
    /// the Bible [`landing_record`](crate::landing::landing_record).
    fn landing_record(&self, site: &LandingSite) -> i16 {
        match self.rules.source_for(RuleKey::LegalRecords) {
            RuleSource::Engine => self.pilot.system_record(self.pilot.system),
            RuleSource::Bible => {
                crate::landing::landing_record(site, self.galaxy.govt(self.pilot.system), |govt| {
                    self.pilot.legal_record(govt)
                })
            }
        }
    }

    /// Trades on the exchange as `order` asks, on its row at that row's
    /// price, and gives the tons moved: a change made in the spaceport, so
    /// a save is due. When the ship is not landed at a trade center, or
    /// the order would move nothing, nothing changes and the refusal says
    /// why.
    pub fn trade(&mut self, order: Order) -> Result<u32, TradeRefusal> {
        let market = self.market().ok_or(TradeRefusal::NoMarket)?;
        let tons = market.tons(order)?;
        let price = market.ordered(order).map_or(0, |row| row.price);
        self.transact(|pilot| market::settle(pilot, order, tons, price));
        Ok(tons)
    }

    /// The player's ship class's `MaxGun` and `MaxTur`; none for a class
    /// with no record, like any class field that cannot be read.
    fn hardpoints(&self) -> Hardpoints {
        self.ship_record(self.pilot.ship)
            .map_or_else(Hardpoints::default, |record| Hardpoints {
                guns: record.max_gun,
                turrets: record.max_tur,
            })
    }

    /// The ship's own free cargo, as the outfitter reads it for a mass
    /// expansion: its cargo space less every ton held, commodities, jünk
    /// and the mission cargo aboard, below zero when it holds more than
    /// that (`_CanBuyOutfitItem` @0x4e980-0x4e9a8 subtracts
    /// `_TotalCargo(player)` from `_ShipTotalHolds()`). The escorts' holds
    /// add nothing.
    fn free_cargo(&self) -> i64 {
        i64::from(self.stats.capacity) - self.pilot.total_cargo()
    }

    /// The outfitter of the stellar the ship is docked at, if it has
    /// landed at one, drawing on `chance` each outfit's roll for the day
    /// not drawn yet (see [`outfitter`]). After an order that changed the
    /// listing it carries the row of the outfit the relisting selected
    /// even when it is not listed ([`Outfitter::unlisted`]; see
    /// [`RuleKey::RelistedSelection`]).
    pub fn outfitter(&mut self, chance: &mut dyn Chance) -> Option<Outfitter> {
        self.outfitter_with(chance, self.outfit_reselected)
    }

    /// The outfitter as [`outfitter`](Self::outfitter) builds it, with
    /// the row of `also` found even when it is not listed (see
    /// `Shop::outfitter_with`).
    fn outfitter_with(
        &mut self,
        chance: &mut dyn Chance,
        also: Option<OutfitId>,
    ) -> Option<Outfitter> {
        let stellar = self.landed?;
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        // Out of the session while the gate reads it.
        let mut rolls = std::mem::take(&mut self.outfit_rolls);
        let outfitter = Shop {
            records: &self.outfits,
            crons: &self.crons,
            ranks: &self.ranks,
            fields: self.fields,
            standard: &self.standard(),
            site,
            fighter_room: &self.fighter_room(),
            ammo_caps: &self.ammo_caps(),
            hardpoints: self.hardpoints(),
            free_cargo: self.free_cargo(),
            launchers: &self.launchers(),
            visit: &self.outfit_visit,
            price_mult: self.outfit_price_mult(),
            rules: self.rules,
            gate: self.gate(),
        }
        .outfitter_with(&self.pilot, &mut rolls, chance, also);
        self.outfit_rolls = rolls;
        outfitter
    }

    /// The outfitter dialog opens: the once-an-opening flags are cleared,
    /// as `_DoOutfitDialog` does on entry (@0x5bb37-0x5bb44), so a map or
    /// clean-record outfit can be bought again (see [`outfitter`]), and
    /// the session resets (@0x5bb57; see the `precalced` module), before
    /// the outfitter's list is built.
    pub fn open_outfitter(&mut self) {
        self.open_outfit_visit();
        self.reset_precalced();
    }

    /// The outfitter dialog closes, by Done, or once an order leaves
    /// nothing listed (@0x5c40c, @0x5d128), every way out joining one tail
    /// (@0x5da33): the session resets (`_SystemInfoToShipStats` @0x5db10;
    /// see the `precalced` module) and forgets the venue offers were last
    /// entered at (`_ResetMissionOfferVenue` @0x5dbd1), so entering one
    /// again forgets the offers whose accept was refused (see the
    /// `mission_screens` module).
    ///
    /// The tail's other parts are modelled elsewhere: its clamp of the
    /// fuel, shield and armour to their most (@0x5da40-0x5dad9) is each
    /// order's refit, and its taking away of the outfits flagged 0x0010
    /// (@0x5dadd-0x5db0e) is the purchase's, at once (see the `outfits`
    /// module).
    pub fn close_outfitter(&mut self) {
        self.reset_precalced();
        self.offer_venue = None;
    }

    /// The bar's hire screen closes, after a hire or none, or the bar says
    /// nothing is for hire: `_DoBarDialog` resets as `_DoShipyardDialog` in
    /// hire mode returns (@0x49235; see the `precalced` module). A hire
    /// ([`Session::hire`]) resets nothing of its own, so a hire's reset
    /// is this one, after it.
    pub fn close_hire(&mut self) {
        self.reset_precalced();
    }

    /// Opens a new outfitter visit: clears the once-an-opening flags and
    /// copies the counts owned, as `_DoOutfitDialog` does on entry
    /// (@0x5badd-0x5bafa, @0x5bb37-0x5bb44). A landing, and a session's
    /// start, do it too, so a sale made without the dialog opened first
    /// (a headless one) refunds against the counts owned since then, and
    /// an outfit owned before never refunds in full.
    fn open_outfit_visit(&mut self) {
        self.outfit_visit = OutfitVisit::open(&self.pilot.outfits);
        self.outfit_reselected = None;
    }

    /// Buys or sells one outfit as `order` asks, the rolls not drawn yet
    /// drawn on `chance`: a change made in the spaceport, so a save is
    /// due, and the stats change with it. The outfit's `OnPurchase` or
    /// `OnSell` runs once, drawing on `chance` too (see the `hooks`
    /// module). When the ship is not landed at an outfitter, or the order
    /// is refused, nothing changes and the refusal says why. It is the
    /// order loop with a count of 1, as a plain click is (@0x5c69f; see
    /// [`outfit_counted`](Self::outfit_counted)).
    pub fn outfit(
        &mut self,
        order: OutfitOrder,
        chance: &mut dyn Chance,
    ) -> Result<(), OutfitRefusal> {
        self.outfit_run(order, 1, chance).counted().map(|_| ())
    }

    /// Runs the order loop for `count` of `order`, each pass an
    /// [`outfit_pass`](Self::outfit_pass), and gives the passes done and
    /// the first refusal (see [`outfit_order`](Self::outfit_order)).
    fn outfit_run(
        &mut self,
        order: OutfitOrder,
        count: u32,
        chance: &mut dyn Chance,
    ) -> OrderRun<OutfitRefusal> {
        self.outfit_order(order, count, chance).run
    }

    /// Buys or sells up to `count` of an outfit as
    /// [`outfit_counted`](Self::outfit_counted) does, and gives the whole
    /// run: the passes done, the first refusal and its pass, and the
    /// alert that refused pass shows, if any ([`Outfitter::sale_alert`],
    /// asked of the outfitter that pass was checked against, so its names
    /// and counts are the click's). A sale runs on past a refusal
    /// ([`order_loop::run_on`]), each later pass refused silently, and a
    /// buy stops at it ([`order_loop::run`]).
    ///
    /// By the engine reading of
    /// [`RuleKey::RelistedSale`](crate::RuleKey::RelistedSale), a sale
    /// that changes the listing makes the passes left check and sell
    /// `oütf` 128 ([`FIRST_OUTFIT`]), listed or not, from index 1, or 513
    /// when nothing is listed, as `_DoOutfitDialog`'s sell loop does
    /// (@0x5d017-0x5d150); `done` counts every sale the order made, so it
    /// can pass the count. By the other reading, the ordered outfit and
    /// the pass index are kept.
    ///
    /// The run's `after` says what the order leaves the outfitter's
    /// selection at, as [`RuleKey::RelistedSelection`] reads it. By the
    /// engine, an order that changes the listing, once after the buy loop
    /// (@0x5c314) or after any sale (@0x5d00a), gives
    /// [`Reselection::Reset`] of `oütf` 128, or [`Reselection::Leave`]
    /// when a rebuild left nothing listed, and the session remembers 128
    /// as selected: the outfitter then carries its row even when it is
    /// not listed ([`Outfitter::unlisted`]), until an order of another
    /// outfit or a new opening. Otherwise, and by the other reading,
    /// [`Reselection::Kept`]. The rebuilds this needs draw no roll the
    /// order would not have drawn.
    ///
    /// A buy resets once after its loop (`_DoOutfitDialog` @0x5c305;
    /// see the `precalced` module) whenever Buy passed its own check
    /// ([`Outfitter::can`]): a loop stopped by a refusal after its first
    /// pass and a Cancel's count of none included, a buy refused at the
    /// button not. A sale resets after each pass (@0x5cffb).
    ///
    /// A `count` of none runs no pass and moves nothing; a buy of none
    /// asks the outfitter whether Buy passes, drawing any roll not drawn
    /// yet, for its reset; [`outfit_counted`](Self::outfit_counted) is the
    /// one that counts a Cancel's outfit day.
    ///
    /// A sale need not lower a count owned: an `OnSell` can grant the
    /// outfit sold back and change the listing again (`^b7 G128`), so
    /// every pass restarts. The original has no guard against that and
    /// would sell forever, or until a count owned saturates. The port
    /// ends the order instead once it has restarted [`RESTART_CEILING`]
    /// times: a deliberate divergence reachable only by plug-ins, as no
    /// stock `OnSell` changes the listing. It keeps no repeat check: a
    /// sale's future also depends on `OnSell`'s `R(...)` draws and on
    /// state no cheap key holds (explored systems, the ship), so an order
    /// that comes back to the counts, listing and bits of an earlier
    /// restart can still end in the original. The ceiling's one cost is
    /// that a plug-in order relisting on more than 512 sales is cut short
    /// where the original would finish.
    pub fn outfit_order(
        &mut self,
        order: OutfitOrder,
        count: u32,
        chance: &mut dyn Chance,
    ) -> OutfitRun {
        let follows = self.rules.source_for(RuleKey::RelistedSelection) == RuleSource::Engine;
        // An order of another outfit names another cell: the selection
        // moved, so the relisting's selection is let go.
        if self.outfit_reselected != Some(order.outfit) {
            self.outfit_reselected = None;
        }
        let asked = self.outfit_reselected;
        let mut alert = None;
        let mut relisting = Relisting::default();
        let run = match order.direction {
            Direction::Sell
                if self.rules.source_for(RuleKey::RelistedSale) == RuleSource::Engine =>
            {
                self.relisted_sale(order, count, chance, &mut alert, asked, &mut relisting)
            }
            Direction::Sell => {
                let mut listing = self.listing_for(follows, count, chance);
                order_loop::run_on(count, |pass| {
                    self.outfit_pass(order, pass, chance, &mut alert, asked)?;
                    if let Some(listing) = listing.as_mut() {
                        let after = listed(self.outfitter(chance).as_ref());
                        relisting.note(listing, &after);
                        *listing = after;
                    }
                    Ok(Onward::Next)
                })
            }
            Direction::Buy => {
                let listing = self.listing_for(follows, count, chance);
                let run = order_loop::run(count, |pass| {
                    self.outfit_pass(order, pass, chance, &mut alert, asked)
                });
                // The buy loop's reset, once after the loop (@0x5c305),
                // whenever Buy passed its own check: a Cancel's count of
                // none and a loop stopped by a refusal included.
                if self.buy_button_passed(order, count, &run, chance, asked) {
                    self.reset_precalced();
                }
                // The buy loop rebuilds the list once, after the loop
                // (@0x5c314).
                if let Some(listing) = listing {
                    relisting.note(&listing, &listed(self.outfitter(chance).as_ref()));
                }
                run
            }
        };
        let after = if follows {
            relisting.reselection()
        } else {
            Reselection::Kept
        };
        if after != Reselection::Kept {
            self.outfit_reselected = Some(FIRST_OUTFIT);
        }
        OutfitRun { run, alert, after }
    }

    /// Whether a buy order's button passed its own check
    /// (`_CanBuyOutfitItem`, [`Outfitter::can`]), so the buy loop's reset
    /// runs after it (@0x5c305): for a `count` of none, asked of the
    /// outfitter now; otherwise unless `run` was refused at its first
    /// pass, whose check is the button's.
    fn buy_button_passed(
        &mut self,
        order: OutfitOrder,
        count: u32,
        run: &OrderRun<OutfitRefusal>,
        chance: &mut dyn Chance,
        asked: Option<OutfitId>,
    ) -> bool {
        if count == 0 {
            return self
                .outfitter_with(chance, asked)
                .is_some_and(|outfitter| outfitter.can(order).is_ok());
        }
        run.stopped.as_ref().is_none_or(|stopped| stopped.pass > 0)
    }

    /// The listing before an order's first pass, when
    /// [`RuleKey::RelistedSelection`] follows the engine (`follows`) and
    /// the order runs a pass: building it draws the rolls that pass
    /// would, so that pass draws none. `None` otherwise, building
    /// nothing.
    fn listing_for(
        &mut self,
        follows: bool,
        count: u32,
        chance: &mut dyn Chance,
    ) -> Option<Vec<OutfitId>> {
        (follows && count > 0).then(|| listed(self.outfitter(chance).as_ref()))
    }

    /// A counted sale by the engine reading of
    /// [`RuleKey::RelistedSale`]: a sale that changes the listing makes
    /// the passes left check and sell `oütf` 128 from index 1, or 513
    /// when nothing is listed (see [`outfit_order`](Self::outfit_order)).
    /// Each relisting is noted in `relisting`. The first passes ask for
    /// `asked`, the outfit a relisting selected before the order.
    fn relisted_sale(
        &mut self,
        order: OutfitOrder,
        count: u32,
        chance: &mut dyn Chance,
        alert: &mut Option<SaleAlert>,
        asked: Option<OutfitId>,
        relisting: &mut Relisting,
    ) -> OrderRun<OutfitRefusal> {
        // The listing as the first pass sees it: building it draws the
        // rolls that pass would, so that pass draws none. A count of none
        // runs no pass, so it builds nothing.
        let mut listing = if count > 0 {
            listed(self.outfitter(chance).as_ref())
        } else {
            Vec::new()
        };
        let mut selected = None;
        let mut restarts = 0;
        order_loop::run_on(count, |pass| {
            let passed = OutfitOrder {
                outfit: selected.unwrap_or(order.outfit),
                ..order
            };
            self.outfit_pass(passed, pass, chance, alert, selected.or(asked))?;
            let after = listed(self.outfitter(chance).as_ref());
            relisting.note(&listing, &after);
            if after == listing {
                return Ok(Onward::Next);
            }
            // The port's guard, the ceiling alone: no repeat check, as
            // `OnSell`'s random draws and unkeyed state can end an order
            // that revisits an earlier state.
            if restarts >= RESTART_CEILING {
                return Ok(Onward::Stop);
            }
            restarts += 1;
            listing = after;
            selected = Some(FIRST_OUTFIT);
            Ok(Onward::From(if listing.is_empty() {
                LIST_SLOTS
            } else {
                0
            }))
        })
    }

    /// One pass of an outfit order: builds the outfitter afresh, drawing
    /// on `chance` any roll not drawn yet, checks `order` against it at
    /// `pass` ([`Outfitter::check_at`]) and, when it goes through, buys or
    /// sells one (see [`outfit`](Self::outfit)). When it is refused and
    /// that outfitter gives an alert for `pass`, `alert` is set to it; a
    /// pass refused silently leaves an earlier pass's alert in place. The
    /// outfitter has the row of `also` even when it is not listed: the
    /// outfit a relisting selects (see
    /// [`outfit_order`](Self::outfit_order)). By the engine, a sale that
    /// leaves none of an outfit mounting the secondary selected lets it
    /// go, before `OnSell` runs (`_DoOutfitDialog` @0x5cf73-0x5cf8c,
    /// @0x5ce82, before @0x5cff6); by the other reading of
    /// [`RuleKey::SecondarySale`], a sale that leaves the ship not
    /// carrying it does, at the same point. A sale resets after its pass
    /// (@0x5cffb); a buy does not, as its reset moved to
    /// [`outfit_order`](Self::outfit_order), once after the loop
    /// (@0x5c305).
    fn outfit_pass(
        &mut self,
        order: OutfitOrder,
        pass: Pass,
        chance: &mut dyn Chance,
        alert: &mut Option<SaleAlert>,
        also: Option<OutfitId>,
    ) -> Result<(), OutfitRefusal> {
        let outfitter = self
            .outfitter_with(chance, also)
            .ok_or(OutfitRefusal::NoOutfitter)?;
        if let Err(refusal) = outfitter.check_at(order, pass) {
            if let Some(shown) = outfitter.sale_alert(order, pass) {
                *alert = Some(shown);
            }
            return Err(refusal);
        }
        let price = outfitter.row(order.outfit).map_or(0, |row| row.buy_price);
        let record = self
            .outfits
            .iter()
            .find(|record| record.id == order.outfit)
            .cloned()
            .ok_or(OutfitRefusal::NotListed)?;
        let amount = match order.direction {
            Direction::Buy => price,
            Direction::Sell => outfitter.refund(order.outfit).unwrap_or(0),
        };
        self.transact(|pilot| outfitter::settle(pilot, &record, order.direction, amount));
        match order.direction {
            Direction::Buy => {
                self.outfit_visit.bought(&record);
                let added = self.grant_outfit(record.id);
                self.run_script(&record.on_purchase, chance);
                if added && record.flags & OutfitFlags::REMOVE_AFTER_PURCHASE != 0 {
                    self.remove_after_purchase(record.id);
                }
            }
            Direction::Sell => {
                self.sale_keeps_cargo = self.sell_keeps_cargo();
                // The sale lets go of the secondary selected, before
                // `OnSell` runs, as `SecondarySale` reads (see [`launches`]
                // and [`carries`]).
                let let_go = match self.rules.source_for(RuleKey::SecondarySale) {
                    RuleSource::Engine => {
                        self.pilot.owned(record.id) == 0 && launches(&record, self.secondary)
                    }
                    RuleSource::Bible => !carries(&self.pilot, &self.outfits, self.secondary),
                };
                if let_go {
                    self.secondary = None;
                }
                self.run_script(&record.on_sell, chance);
            }
        }
        self.stay.outfit_traded = true;
        self.refit(true, ReservePolicy::Hold);
        // A sale's reset, after each pass (`_DoOutfitDialog` @0x5cffb; see
        // the `precalced` module). A buy's runs once after its loop
        // ([`outfit_order`](Self::outfit_order), @0x5c305).
        if order.direction == Direction::Sell {
            self.reset_precalced();
        }
        self.sale_keeps_cargo = false;
        Ok(())
    }

    /// Buys or sells up to `count` of an outfit as `order` asks, through
    /// the order loop one pass at a time, each checked as the engine's
    /// loop checks it (see
    /// [`RuleKey::CountedSale`](crate::RuleKey::CountedSale)), as
    /// `_DoOutfitDialog`'s loops do: the buy loop (@0x5c1bd-0x5c2ff) re-runs `_CanBuyOutfitItem`
    /// after each grant and stops at the first refusal
    /// ([`order_loop::run`]), and the sell loop (@0x5c6ee-0x5d1b1) checks
    /// each sale afresh and runs on past a refusal (@0x5d19c-0x5d1b1),
    /// each later pass refused silently ([`order_loop::run_on`]). It
    /// gives how many went through, or the refusal of the first, with
    /// nothing changed. A `count` of none, which the dialog's Cancel
    /// gives, moves nothing and makes no save due; but when the order's
    /// button passes its own check ([`Outfitter::can`]), the stay still
    /// counts an outfit traded, so leaving passes its day: the original
    /// sets its flag once Buy or Sell passes its own check
    /// (`_DoOutfitDialog` @0x5c087-0x5c0a2 after `_CanBuyOutfitItem`,
    /// @0x5c5d5 after the Sell flag), whatever count follows, a Cancel's 0
    /// (`_DoQuantityDialog` @0x57138) too (@0x5c555, @0x5d317). The count
    /// is not checked against [`Outfitter::count_max`]: whatever it is,
    /// the loop runs as the engine's does. A sale that changes the listing
    /// goes on with `oütf` 128 from index 1 (or 513), as
    /// [`RuleKey::RelistedSale`](crate::RuleKey::RelistedSale) says, and
    /// the count it gives is every sale the order made. What the order
    /// leaves the selection at is [`outfit_order`](Self::outfit_order)'s
    /// `after`.
    pub fn outfit_counted(
        &mut self,
        order: OutfitOrder,
        count: u32,
        chance: &mut dyn Chance,
    ) -> Result<u32, OutfitRefusal> {
        if count == 0
            && self
                .outfitter(chance)
                .is_some_and(|outfitter| outfitter.can(order).is_ok())
        {
            self.stay.outfit_traded = true;
        }
        self.outfit_run(order, count, chance).counted()
    }

    /// The shipyard of the stellar the ship is docked at, if it has landed
    /// at one, drawing on `chance` each class's roll for the day not drawn
    /// yet (see [`shipyard`]). This builds its list
    /// afresh: the roll of each class bought or declined since it was last
    /// built is drawn again first.
    pub fn shipyard(&mut self, chance: &mut dyn Chance) -> Option<Shipyard> {
        for ship in std::mem::take(&mut self.ship_redraws) {
            self.ship_rolls.redraw(&ship);
        }
        self.listed_shipyard(chance)
    }

    /// The shipyard as its list was last built, each class bought or
    /// declined since keeping its roll, the rolls not drawn yet drawn on
    /// `chance`.
    fn listed_shipyard(&mut self, chance: &mut dyn Chance) -> Option<Shipyard> {
        let stellar = self.landed?;
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        // Out of the session while the gate reads it.
        let mut rolls = std::mem::take(&mut self.ship_rolls);
        let shipyard = Yard {
            ships: &self.ships,
            outfits: &self.outfits,
            crons: &self.crons,
            ranks: &self.ranks,
            arsenal: &self.arsenal,
            fields: self.fields,
            site,
            mults: self.price_mults(),
            rules: self.rules,
            gate: self.gate(),
        }
        .shipyard(&self.pilot, &mut rolls, chance);
        self.ship_rolls = rolls;
        shipyard
    }

    /// The shipyard and the record of class `ship`, when it can be bought
    /// there now, the rolls not drawn yet drawn on `chance`; otherwise why
    /// not.
    fn buyable(
        &mut self,
        ship: ShipId,
        chance: &mut dyn Chance,
    ) -> Result<(Shipyard, ShipRecord), ShipRefusal> {
        let shipyard = self
            .listed_shipyard(chance)
            .ok_or(ShipRefusal::NoShipyard)?;
        shipyard.can(ship)?;
        let record = self
            .ships
            .iter()
            .find(|record| record.id == ship)
            .cloned()
            .ok_or(ShipRefusal::NotListed)?;
        Ok((shipyard, record))
    }

    /// The prompt for naming a ship of class `ship` before it is bought,
    /// its default's digits drawn on `chance` ([`shipyard`]'s "Naming the
    /// ship"). It is refused as [`buy_ship`](Self::buy_ship) would refuse
    /// the purchase, and changes nothing about the pilot, so no save is
    /// due. The shipyard's rolls not drawn yet are drawn on `chance`
    /// first.
    pub fn ship_naming(
        &mut self,
        ship: ShipId,
        chance: &mut dyn Chance,
    ) -> Result<ShipNaming, ShipRefusal> {
        let (_, record) = self.buyable(ship, chance)?;
        Ok(shipyard::naming(&record, chance))
    }

    /// Declines to buy a ship of class `ship`, as cancelling its name
    /// prompt does: nothing is bought, but the class's roll is drawn
    /// again, as the original does (`_DoShipyardDialog` @0x5ebc2 to
    /// @0x5f0d5-0x5f0f8), the next time the shipyard's list is built
    /// ([`shipyard`](Self::shipyard)); until then it can be named and
    /// bought again. Nothing about the pilot changes, so no save is due.
    pub fn decline_ship(&mut self, ship: ShipId) {
        self.ship_redraws.insert(ship);
    }

    /// Buys a ship of class `ship`, named `name` ([`shipyard::cull_name`]
    /// drops a leading "the "), trading in the one flown, and gives
    /// what the purchase did: a change made in the spaceport, so a save is
    /// due, and the session flies the new ship from then on, its fields,
    /// default items, stock weapons and stats read from its record. The
    /// old class's `OnRetire` runs first and the new class's `OnPurchase`
    /// last, each drawing on `chance`, the paint cleared as
    /// [`HookRules::purchase_paint_order`] says (see the `hooks` module),
    /// and then the shield, armour and fuel are filled to the most of the
    /// class and outfits `OnPurchase` leaves (`_DoShipyardDialog`
    /// @0x5f051-0x5f07b, after @0x5f00d). No secondary is selected after
    /// `OnPurchase` (@0x5f01c), even one the new ship carries.
    /// When the ship is not landed at a shipyard, or the purchase is
    /// refused, nothing changes and the refusal says why. The rolls not
    /// drawn yet are drawn on
    /// `chance`, and a purchase draws the class's roll again, as the
    /// original does (`_DoShipyardDialog` @0x5f0d5-0x5f0f8), the next time
    /// the shipyard's list is built ([`shipyard`](Self::shipyard)). A
    /// purchase tells [`SessionEvent::ShipChanged`] before `OnPurchase`
    /// runs, which may tell it again; a refused one tells nothing.
    pub fn buy_ship(
        &mut self,
        ship: ShipId,
        name: &str,
        chance: &mut dyn Chance,
    ) -> Result<ShipPurchase, ShipRefusal> {
        let (shipyard, record) = self.buyable(ship, chance)?;
        let quote = Quote {
            price: shipyard.row(ship).map_or(0, |row| row.price),
            trade_in: shipyard.trade_in,
        };
        self.ship_hook(self.pilot.ship, ShipHook::Retire, chance);
        let paint_last = self.hook_rules().purchase_paint_order == RuleSource::Engine;
        let old_mass = self.fields.mass;
        self.drop_fighters(|_, _| false);
        let escorts: Vec<EscortHolds> = self.escort_holds().collect();
        let fits = self.arsenal.stock_fits(ship, &self.outfits);
        let Purchase {
            outfits,
            settlement,
            report,
        } = shipyard::purchase(
            &Sale {
                pilot: &self.pilot,
                old_mass,
                new: &record,
                name,
                fits: &fits,
                records: &self.outfits,
                escorts: &escorts,
                rules: self.rules,
            },
            quote,
        );
        self.become_class(&record, ClassChange::bought(outfits), |session| {
            settlement.settle(&mut session.pilot);
            if !paint_last {
                session.pilot.paint = None;
            }
        });
        // The resets of the new class's stats and of the share's tail
        // (`_DoShipyardDialog` @0x5ee99, @0x5ef9f; see the `precalced`
        // module): their trims are the purchase rule's, already made.
        self.reset_precalced_for(precalced::ResetCause::Purchase);
        self.ship_redraws.insert(ship);
        self.ship_hook(record.id, ShipHook::Purchase, chance);
        // No secondary is selected on the new ship (`_DoShipyardDialog`
        // @0x5f01c), once its `OnPurchase` has run (@0x5f00d) and before
        // the fill (@0x5f051).
        self.secondary = None;
        if paint_last {
            // A new ship is unpainted (`_DoShipyardDialog` @0x5f022), once
            // its `OnPurchase` has run (@0x5f00d); the save is due already.
            self.pilot.paint = None;
        }
        // Filled once `OnPurchase` has run and the paint is cleared, to the
        // class and outfits it leaves (`_DoShipyardDialog`
        // @0x5f051-0x5f07b); the save is due already.
        self.pilot.reserves = shipyard::filled(&self.stats, &self.rules);
        self.stay.ship_bought = true;
        Ok(report)
    }

    /// Fills the tank at the stellar the ship is docked at, as the
    /// [`recharge`] rules say, and gives the price paid: a change made in
    /// the spaceport, so a save is due. When the ship is not landed where
    /// fuel is sold, or the refill is refused, nothing changes and the
    /// refusal says why.
    pub fn recharge(&mut self) -> Result<i64, RechargeRefusal> {
        let stellar = self.landed.ok_or(RechargeRefusal::NoFuel)?;
        let site = self.sites.iter().find(|site| site.id == stellar);
        if !site.is_some_and(|site| recharge::sells_fuel(site.flags)) {
            return Err(RechargeRefusal::NoFuel);
        }
        let price = recharge::quote(self.pilot.reserves.fuel, self.pilot.cash)?;
        self.transact(|pilot| recharge::settle(pilot, price));
        Ok(price)
    }

    /// The fleet's cargo space, in tons (`_TotalFleetHolds`): the ship's
    /// own, with its cargo outfits, plus the `Holds` of each trader escort
    /// ([`market::fleet_holds`]). The exchange and the jünk step measure
    /// free space against it.
    #[must_use]
    pub fn capacity(&self) -> u32 {
        market::fleet_holds(self.stats.capacity, self.escort_holds())
    }

    /// What the fleet's cargo space needs of each escort whose ship class
    /// has a record.
    fn escort_holds(&self) -> impl Iterator<Item = EscortHolds> + '_ {
        self.pilot
            .escorts
            .iter()
            .filter_map(|escort| self.holds_of(escort))
    }

    /// What the fleet's cargo space needs of `escort`: none when its ship
    /// class has no record.
    fn holds_of(&self, escort: &crate::pilot::Escort) -> Option<EscortHolds> {
        let record = self.ship_record(escort.ship)?;
        Some(EscortHolds {
            holds: record.fields.holds,
            inherent_ai: record.inherent_ai,
            carried: escort.carried,
        })
    }

    /// How the ship performs, with the outfits it carries.
    #[must_use]
    pub fn stats(&self) -> ShipStats {
        self.stats
    }

    /// Whether the pilot should be saved: it has landed, taken off,
    /// changed in the spaceport or, playing strictly, been rescued from the
    /// escape pod since this was last taken. Taking it clears it.
    pub fn take_save_due(&mut self) -> bool {
        std::mem::take(&mut self.save_due)
    }

    /// The stellar the ship is docked at, if it has landed.
    #[must_use]
    pub fn landed(&self) -> Option<StellarId> {
        self.landed
    }

    /// Whether the ship flew its last tick under thrust. Landing and
    /// beginning a jump stop it, as they stop the thrust sound.
    #[must_use]
    pub fn thrusting(&self) -> bool {
        self.thrusting
    }

    /// The engine glow's base level, as [`glow`](crate::glow) says: each
    /// tick in flight thrust raises it by 1 up to
    /// [`GLOW_CRUISE`](crate::GLOW_CRUISE) and coasting lowers it by 1 to
    /// 0, so it stays within 0 to 24. It starts at 0, and landing and
    /// beginning a jump put it back to 0.
    #[must_use]
    pub fn engine_glow(&self) -> u8 {
        self.engine_glow
    }

    /// The player's ship as it flies.
    #[must_use]
    pub fn player(&self) -> &ShipState {
        &self.player
    }

    /// The pilot flying.
    #[must_use]
    pub fn pilot(&self) -> &Pilot {
        &self.pilot
    }

    /// The ship's shield, armour and fuel.
    #[must_use]
    pub fn reserves(&self) -> Reserves {
        self.pilot.reserves
    }

    /// The system the player is in.
    #[must_use]
    pub fn system(&self) -> SystemId {
        self.pilot.system
    }

    /// The player's ship class.
    #[must_use]
    pub fn ship(&self) -> ShipId {
        self.pilot.ship
    }

    /// How the player's ship flies, before `_ShipMaxSpeed`'s factor for a
    /// pilot without Strict Play: the ship's own top speed, which the
    /// outfitter and the shipyard show. The player flies at
    /// [`Session::top_speed`].
    #[must_use]
    pub fn handling(&self) -> Handling {
        self.stats.handling
    }

    /// The player's top speed in flight, its `_ShipMaxSpeed`: the ship's
    /// own ([`Session::handling`]) × the led speed, 1.5 for a pilot
    /// without Strict Play by the engine (see
    /// [`top_speed`]).
    #[must_use]
    pub fn top_speed(&self) -> f32 {
        self.stats.handling.max_speed * self.led_speed()
    }

    /// The factor on the top speed of the player and of every ship it
    /// leads (see [`top_speed::led_speed`]), read from the rules and the
    /// pilot at each use, so nothing is deferred.
    pub(crate) fn led_speed(&self) -> f32 {
        top_speed::led_speed(&self.rules, self.pilot.strict_play())
    }

    /// How the player's ship flies in flight: its own handling, at
    /// [`Session::top_speed`].
    fn flying_handling(&self) -> Handling {
        Handling {
            max_speed: self.top_speed(),
            ..self.stats.handling
        }
    }

    /// The government whose `Interface` picks the status bar, as
    /// `_LoadIntfResource` (@0x735f2) reads it from the player's ship type:
    /// its attributes government (runtime +0x9f6, our
    /// [`ShipRecord::inherent_govt`]), or its combat government (+0x9f4,
    /// [`ShipRecord::combat_govt`]) when that is none (@0x7361d-0x73635),
    /// which is so only for a combat-only `InherentGovt` (2128-2383; the
    /// `shïp` loader @0x7a261-0x7a330). `None` when it has neither, or the
    /// ship type has no record (the original's type range check @0x73609);
    /// either way the bar is `ïntf` 128.
    #[must_use]
    pub fn interface_government(&self) -> Option<GovtId> {
        let record = self.ship_record(self.ship())?;
        record.inherent_govt.or(record.combat_govt)
    }

    /// `good`'s name, as the exchange names it, if it is traded
    /// ([`Goods::name`](crate::market::Goods::name)).
    #[must_use]
    pub fn good_name(&self, good: Good) -> Option<&str> {
        self.goods.name(good)
    }

    /// Outfit `outfit`'s name, as its [`OutfitRecord`] gives it, if the
    /// session has its record.
    #[must_use]
    pub fn outfit_name(&self, outfit: OutfitId) -> Option<&str> {
        let record = self.outfits.iter().find(|record| record.id == outfit)?;
        Some(&record.name)
    }

    /// Outfit `outfit`'s lower-case names, its `LCName` and `LCPlural`, if
    /// the session has its record.
    #[must_use]
    pub fn outfit_names(&self, outfit: OutfitId) -> Option<(&str, &str)> {
        let record = self.outfits.iter().find(|record| record.id == outfit)?;
        Some((&record.lc_name, &record.lc_plural))
    }

    /// Ship class `ship`'s record, if the session has it.
    fn ship_record(&self, ship: ShipId) -> Option<&ShipRecord> {
        self.ships.iter().find(|record| record.id == ship)
    }

    /// Ship class `ship`'s `Crew`; none without a record.
    fn crew(&self, ship: ShipId) -> i16 {
        self.ship_record(ship).map_or(0, |record| record.crew)
    }

    /// Boards the player's target, if it is not landed or jumping and the
    /// [`board`](crate::board) checks allow it, and gives what is on
    /// board: the player matches the target's velocity, `law` convicts it
    /// of the [`Crime::Board`] against the target's government and the
    /// boarding takes the ranks it reaches (see [`rank`](crate::rank)'s
    /// **Crimes**; but for a mission's special ship, here one of a
    /// failed mission: the original passes its mark,
    /// `_HandlePlayerBoardAttempt` @0x6582b, and `_SlapWithPenalty`
    /// slaps only for none, @0x9c10), the
    /// NPCs come to the target's help as for a hit by the player (a
    /// strike of no damage, answered on the next traffic tick), the
    /// target is marked boarded, and every other NPC after it lets it go
    /// and decides again (`_AIShipBoardedByPlayer`). Then, unless `rule`
    /// says its crew repels the boarders, its plunder is rolled on
    /// `chance` and the boarding is under way ([`Session::plunder`]). A
    /// board that passes the checks raises [`Fate::Boarded`], repelled
    /// or not. A person of `Flags` 0x0200 whose `LinkMission` is
    /// available from a ship offers it instead, once the crime is
    /// convicted ([`Boarding::Offered`], see the `person_offers` module):
    /// nothing is rolled or granted, and the ship is marked boarded, and
    /// [`Fate::Boarded`] raised, only once the mission is accepted. A
    /// mission's special ship, but one of a failed mission, is only
    /// marked boarded, its velocity matched, and maybe counted, a
    /// `PickupMode` 2 mission's cargo picked up
    /// ([`Boarding::MissionShip`], see the `mission_ships` module); one
    /// of a failed mission is plundered, uncharged.
    ///
    /// # Errors
    ///
    /// The [`BoardRefusal`] that applies, and nothing changes;
    /// [`BoardRefusal::NoTarget`] while landed or jumping. A `PickupMode`
    /// 2 mission's special ship whose cargo has no room is
    /// [`BoardRefusal::CantBoard`], and the pickup's refusal is queued
    /// as a mission text, as `_DoMissionCargoPickup` shows its dialog
    /// (@0x982ff); nothing else changes, the velocity not matched
    /// (@0x65fdb).
    pub fn board(
        &mut self,
        law: &dyn LegalCode,
        rule: &dyn BoardingRule,
        chance: &mut dyn Chance,
    ) -> Result<Boarding, BoardRefusal> {
        if self.landed.is_some() || self.jumping().is_some() {
            return Err(BoardRefusal::NoTarget);
        }
        let target = self.target().map(|npc| BoardTarget {
            state: npc.state,
            condition: npc.condition,
            boarded: npc.boarded,
            crew: self.crew(npc.ship),
            reach: npc.hull.board_reach,
        });
        check_board(&self.player, self.condition, target)?;
        let npc = self.target().cloned().ok_or(BoardRefusal::NoTarget)?;
        if let Some(boarding) = self.board_mission_ship(&npc) {
            let boarding = boarding?;
            self.player.velocity = npc.state.velocity;
            return Ok(boarding);
        }
        self.player.velocity = npc.state.velocity;
        if npc.mission.is_none() {
            let changes = law.penalties(Crime::Board, npc.govt, &self.govts);
            legal::convict(&mut self.pilot, &changes, self.galaxy.governed());
            self.lose_ranks_to(Crime::Board, npc.govt);
        }
        self.strikes.push(Strike {
            ship: ShipRef::Npc(npc.id),
            by: ShipRef::Player,
            damage: 0.0,
            downed: None,
            provokes: true,
        });
        if let Some(offering) = self.person_offering(&npc, person_offers::OfferedBy::Boarding) {
            return Ok(Boarding::Offered(self.offer_from_ship(offering, chance)));
        }
        self.mark_boarded(npc.id);
        if rule.repels(npc.booty) {
            return Ok(Boarding::Repelled);
        }
        let mut plunder = Plunder::roll(&self.prize(&npc), rule, chance);
        plunder.odds = rule.capture_odds(&self.capture_crew(&npc), chance);
        self.granted = self.grant(&npc, rule, chance);
        let aboard = Aboard {
            npc: npc.id,
            ship: npc.ship,
            plunder,
            captured: false,
        };
        self.aboard = Some(aboard);
        Ok(Boarding::Opened(aboard.view()))
    }

    /// NPC `id` is marked boarded, raising [`Fate::Boarded`], and every
    /// other NPC after it lets it go and decides again
    /// (`_AIShipBoardedByPlayer`). A former escort forgets what it was
    /// ([`Npc::former`]), as each of the original's ways of boarding one's
    /// own former escort ends (`_HandlePlayerBoardAttempt` @0x65b2f,
    /// @0x65be8, @0x65e62; that boarding repair is the boarding
    /// follow-up's).
    fn mark_boarded(&mut self, id: NpcId) {
        if let Some(npc) = self.npc_mut(id) {
            npc.boarded = true;
            npc.former = None;
        }
        self.raise(StoryEvent::ShipFate {
            ship: id,
            fate: Fate::Boarded,
        });
        self.drop_quarry(id);
    }

    /// What boarding `npc` grants as `rule` says, drawn on `chance`, added
    /// to the outfits the pilot owns and the ship refitted with them (see
    /// [`grant`](crate::grant)): none, and nothing drawn, for a ship no
    /// person flies or a person of no grant. The session resets once an
    /// outfit is picked (@0x931d9), and again once it is granted
    /// (@0x93226); a pick of which none fits grants nothing.
    fn grant(
        &mut self,
        npc: &Npc,
        rule: &dyn BoardingRule,
        chance: &mut dyn Chance,
    ) -> Option<Granted> {
        let person = self.traffic.person(npc.person?.id)?;
        let grant = PersonGrant::of(&person.record)?;
        let stock: Vec<GrantStock> = self
            .outfits
            .iter()
            .map(|record| self.grant_stock(record))
            .collect();
        let granted = rule.grant(
            &grant,
            &stock,
            self.free_mass(),
            self.outfit_rules().grant_max,
            chance,
        )?;
        // The dialog's reset once an outfit is picked, before its count
        // is fitted to the free mass (`_DoPlunderDialog` @0x931d9; see the
        // `precalced` module): it trims only the cargo, so the free mass
        // the count was fitted to stands.
        self.reset_precalced();
        if granted.count == 0 {
            return None;
        }
        for _ in 0..granted.count {
            self.grant_outfit(granted.outfit);
        }
        self.refit(true, self.script_reserves());
        // And again once it is granted (@0x93226).
        self.reset_precalced();
        Some(granted)
    }

    /// What the last boarding granted, once: taking it leaves none.
    pub fn take_grant(&mut self) -> Option<Granted> {
        self.granted.take()
    }

    /// Every NPC other than `id` that targets it, or fights it, lets it go
    /// and decides again, its provocation gone.
    fn drop_quarry(&mut self, id: NpcId) {
        let quarry = ShipRef::Npc(id);
        for other in self.traffic.npcs_mut() {
            if other.id != id
                && (other.target == Some(quarry) || other.goal.quarry() == Some(quarry))
            {
                other.target = None;
                other.goal = Goal::Idle;
                other.provoked = 0.0;
            }
        }
    }

    /// What `npc` has on board to plunder.
    fn prize(&self, npc: &Npc) -> Prize {
        let record = self.ship_record(npc.ship);
        let fields = record.map(|record| record.fields).unwrap_or_default();
        let fires = |armament: &Armament, ammo: WeaponId, bay: bool| {
            armament
                .mounts()
                .iter()
                .any(|mount| mount.spec.ammo == Ammo::Rounds(ammo) && (!bay || mount.spec.is_bay()))
        };
        Prize {
            booty: npc.booty,
            cost: record.map_or(0, |record| record.cost),
            holds: fields.holds,
            fuel: fields.fuel,
            rounds: npc
                .rounds
                .iter()
                .map(|(&ammo, &rounds)| HeldRounds {
                    ammo,
                    rounds,
                    bay: fires(&npc.armament, ammo, true),
                    outfit: fires(&self.armament, ammo, false)
                        .then(|| {
                            self.ammo_outfits
                                .iter()
                                .find(|&&(of, _)| of == ammo)
                                .map(|&(_, outfit)| outfit)
                        })
                        .flatten(),
                })
                .collect(),
            person_credits: npc
                .person
                .and_then(|person| self.traffic.person(person.id))
                .map(|person| person.record.credits),
        }
    }

    /// The crews the odds of capturing `npc` are worked out from.
    fn capture_crew(&self, npc: &Npc) -> CaptureCrew {
        CaptureCrew {
            crew: i32::from(self.crew(self.pilot.ship)),
            strength: self.hull.strength as i32,
            escorts: self
                .pilot
                .escorts
                .iter()
                .map(|escort| EscortCrew {
                    crew: i32::from(self.crew(escort.ship)),
                    strength: self.arsenal.hull(escort.ship).strength as i32,
                    inherent_ai: self
                        .ship_record(escort.ship)
                        .map_or(0, |record| record.inherent_ai),
                })
                .collect(),
            marines: outfit_mods(&self.pilot.outfits, &self.outfits)
                .into_iter()
                .filter(|outfit| outfit.mod_type == MARINES)
                .map(|outfit| (outfit.mod_val, outfit.count))
                .collect(),
            target_crew: i32::from(self.crew(npc.ship)),
            target_strength: npc.hull.strength as i32,
            derelict: self.govts.derelict(npc.govt),
            fleet_room: self.pilot.escort_count() < MAX_ESCORTS,
        }
    }

    /// What is on board the ship being boarded, while the plunder dialog
    /// is open: none once the boarding is over, or a capture awaits its
    /// assignment.
    #[must_use]
    pub fn boarding(&self) -> Option<PlunderView> {
        self.aboard
            .filter(|aboard| !aboard.captured)
            .map(|aboard| aboard.view())
    }

    /// Presses `take` in the plunder dialog, and gives what it did (see
    /// [`board`](crate::board)): a press whose value is not on board, or
    /// with no boarding under way, does nothing. Any press but Abort right
    /// after a take first rolls the self-destruct on `chance`; a capture
    /// is rolled as `rule` says.
    pub fn plunder(
        &mut self,
        take: Take,
        rule: &dyn BoardingRule,
        chance: &mut dyn Chance,
    ) -> Taken {
        let Some(mut aboard) = self.aboard.filter(|aboard| !aboard.captured) else {
            return Taken::Nothing;
        };
        if take == Take::Abort || !self.npcs().iter().any(|npc| npc.id == aboard.npc) {
            self.close_plunder();
            return Taken::Aborted;
        }
        if !aboard.view().offers(take) {
            return Taken::Nothing;
        }
        let plunder = &mut aboard.plunder;
        if std::mem::take(&mut plunder.armed)
            && chance.below(SELF_DESTRUCT_ROLL) <= plunder.threshold
        {
            self.self_destruct(aboard.npc);
            return Taken::Tripped;
        }
        let (taken, growth) = match take {
            Take::Cargo => {
                let Some((good, tons)) = plunder.cargo.take() else {
                    return Taken::Nothing;
                };
                // The ship's own space, less `_TotalCargo` with the
                // mission cargo aboard, not the fleet's: plunder into
                // pooled holds waits for rdm task `boarding-follow-up`.
                let room = (i64::from(self.stats.capacity) - self.pilot.total_cargo()).max(0);
                let stored = tons.min(u32::try_from(room).unwrap_or(u32::MAX));
                if stored > 0 {
                    *self.pilot.cargo.entry(good).or_default() += stored;
                }
                (Taken::Cargo { good, stored }, CARGO_GROWTH)
            }
            Take::Credits => {
                let credits = std::mem::take(&mut plunder.credits);
                self.pilot.cash = self.pilot.cash.saturating_add(credits);
                (Taken::Credits(credits), CREDITS_GROWTH)
            }
            Take::Ammo => {
                let Some((outfit, rounds)) = plunder.ammo.take() else {
                    return Taken::Nothing;
                };
                // The ammunition button's reset (`_DoPlunderDialog`
                // @0x93cc9; see the `precalced` module).
                self.reset_precalced();
                let count = self.take_ammo(outfit, rounds);
                (Taken::Ammo { outfit, count }, AMMO_GROWTH)
            }
            Take::Energy => {
                let offered = std::mem::take(&mut plunder.fuel);
                let fuel = &mut self.pilot.reserves.fuel;
                let room = (fuel.max - fuel.now).max(0.0).trunc();
                let stored = (offered as f32).min(room);
                fuel.now += stored;
                let taken = Taken::Energy {
                    offered,
                    stored: stored as u32,
                    full: fuel.now >= fuel.max,
                };
                (taken, ENERGY_GROWTH)
            }
            Take::Capture => return self.capture(aboard, rule, chance),
            Take::Abort => return Taken::Nothing,
        };
        plunder.threshold = (f64::from(plunder.threshold) * growth) as u32;
        plunder.armed = true;
        self.aboard = Some(aboard);
        taken
    }

    /// `record`'s `Max` as the `ModType` 27 outfits owned raise it, as
    /// the boarding's calls of `_HasMaxOfItem` read it (`_DoPlunderDialog`
    /// @0x93119, @0x93192, @0x93d9f; see [`outfitter::raised_max`]).
    fn raised_max_of(&self, record: &OutfitRecord) -> i64 {
        outfitter::raised_max(
            record,
            &self.pilot.outfits,
            &self.outfits,
            self.rules.source_for(RuleKey::RaisedMax),
        )
    }

    /// Takes up to `rounds` of ammunition outfit `outfit`, one at a time
    /// while the free mass covers it and it is below its `Max` as
    /// `ModType` 27 raises it (read again for each, as `_HasMaxOfItem` is
    /// at @0x93d9f), and gives how many; the ship is refitted with them.
    ///
    /// Each round is admitted by the outfit's raw `Mass` (`movswl 0x2`
    /// @0x93d6f, `cmpl`/`jl` @0x93d74-0x93d76), not scaled by Flags
    /// 0x0400 (`_AdjustedItemMass` is not called), against the free mass
    /// `_ShipFreeMass` gives again each round (@0x93d62), which does scale
    /// the rounds already taken.
    ///
    /// The session resets after each round taken (@0x93dcb; see the
    /// `precalced` module), as the ammunition button does before the first
    /// (@0x93cc9).
    fn take_ammo(&mut self, outfit: OutfitId, rounds: u32) -> u32 {
        let Some(record) = self
            .outfits
            .iter()
            .find(|record| record.id == outfit)
            .cloned()
        else {
            return 0;
        };
        let unit = i64::from(record.mass);
        let mut count = 0;
        while count < rounds
            && i64::from(self.pilot.owned(outfit)) < self.raised_max_of(&record)
            && outfitter::free_mass(
                self.fields,
                &self.standard(),
                &self.pilot.outfits,
                &self.outfits,
                &self.rules,
            ) >= unit
        {
            *self.pilot.outfits.entry(outfit).or_default() += 1;
            count += 1;
            // The reset after each round (@0x93dcb).
            self.reset_precalced();
        }
        self.refit(true, self.script_reserves());
        count
    }

    /// Rolls the capture of the ship `aboard` boards, at its odds as
    /// `rule` says on `chance`, and gives what came of it. A capture
    /// taken straight into the fleet runs its class's `OnCapture` on
    /// `chance` first.
    fn capture(
        &mut self,
        mut aboard: Aboard,
        rule: &dyn BoardingRule,
        chance: &mut dyn Chance,
    ) -> Taken {
        if !rule.captures(aboard.plunder.odds, chance) {
            self.close_plunder();
            return Taken::CaptureFailed;
        }
        if chance.below(CAPTURE_TRIP) == 0 {
            self.self_destruct(aboard.npc);
            return Taken::Tripped;
        }
        if self.pilot.escort_count() >= MAX_ESCORTS {
            self.aboard = Some(aboard);
            return Taken::FleetFull;
        }
        if self.crew(self.pilot.ship) > 0 {
            aboard.captured = true;
            self.aboard = Some(aboard);
            return Taken::Captured;
        }
        self.close_plunder();
        self.ship_hook(aboard.ship, ShipHook::Capture, chance);
        self.join_fleet(aboard.npc);
        Taken::Escorted
    }

    /// Trips the self-destruct of NPC `id`: its shield and armour are
    /// gone, so the fight breaks it up, and the boarding is over.
    fn self_destruct(&mut self, id: NpcId) {
        for npc in self.traffic.npcs_mut() {
            if npc.id == id {
                npc.reserves.shield.now = 0.0;
                npc.reserves.armor.now = 0.0;
            }
        }
        self.close_plunder();
    }

    /// Closes the plunder dialog: the boarding is over, and the session
    /// resets, as `_DoPlunderDialog`'s only exit does (@0x94a0b; see the
    /// `precalced` module).
    fn close_plunder(&mut self) {
        self.aboard = None;
        self.reset_precalced();
    }

    /// NPC `id`, captured, joins the fleet where it is: no longer its
    /// person, intact, its armour at half its most, and then as
    /// [`Session::fleet_up`] says.
    fn join_fleet(&mut self, id: NpcId) {
        self.lose_captured(id);
        let Some(npc) = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) else {
            return;
        };
        npc.reserves.armor.now = npc.reserves.armor.max * ESCORT_ARMOR_SHARE;
        npc.condition = Condition::Intact;
        self.fleet_up(id, None);
    }

    /// NPC `id`, the person hailed, joins the fleet where it is as
    /// itself, its reserves as they are (see [`Session::fleet_up`]).
    fn enlist_person(&mut self, id: NpcId) {
        let person = self
            .npc(id)
            .and_then(|npc| npc.person)
            .map(|person| person.id);
        self.fleet_up(id, person);
    }

    /// NPC `id` joins the fleet where it is (`_AIMakeEscortFlyInForm`
    /// @0x89b8e), the escort of `person` when it is one: of no
    /// government, its AI type its ship type's `InherentAI`, keeping
    /// formation with no standing order. Every other NPC lets it go, its
    /// own escorts leave it, and so does the player's target; a save is
    /// due.
    fn fleet_up(&mut self, id: NpcId, person: Option<PersonId>) {
        let Some(npc) = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) else {
            return;
        };
        npc.trigger = Trigger::default();
        npc.provoked = 0.0;
        let record = Escort {
            ship: npc.ship,
            reserves: npc.reserves,
            order: None,
            carried: false,
            wage: None,
            person,
        };
        self.drop_quarry(id);
        for other in self.traffic.npcs_mut() {
            if other.leader == Some(id) {
                other.leader = None;
            }
        }
        if self.target == Some(id) {
            self.target = None;
        }
        self.enlist(id, record);
    }

    /// NPC `id` leaves the system: every other NPC after it lets it go,
    /// and so does the player.
    fn leave(&mut self, id: NpcId) {
        self.drop_quarry(id);
        self.traffic.remove(id);
        self.clear_lost_target();
    }

    /// Assigns the ship captured, as `choice` says, and gives what it
    /// did: none when no capture awaits its assignment (see
    /// [`board`](crate::board)). "Use As Escort" adds it to the fleet;
    /// "Use As My Ship" swaps the player into it, the old ship joining the
    /// fleet, its fuel drawn on `chance`, unless no ship slot is free in
    /// the system for the old ship, when the prize is lost and nothing
    /// else changes. The player keeps its persistent outfits and gets the
    /// captured class's default items, then its stock weapons and their
    /// `AmmoLoad`, topped up as after a purchase. Either change makes a
    /// save due. The captured class's `OnCapture` runs on `chance` before
    /// it joins the fleet; on "Use As My Ship" the old class's `OnRetire`
    /// runs first, both before the fuel draw and, as
    /// [`HookRules::capture_hook_order`] says, before or after the outfit
    /// swap (see the `hooks` module). The change of class is applied by
    /// [`Session::become_class`], with the reserves held. "Use As My
    /// Ship" leaves no secondary selected (`_DoShipCapture` @0x4173e);
    /// "Use As Escort" leaves it as it is. By the engine reading of
    /// [`RuleKey::CaptureCargo`], "Use As My Ship" leaves the commodities
    /// with the old ship, the `jünk` and the mission cargo staying with the
    /// player; by the other, the player keeps them. The commodities go,
    /// and the old ship joins the fleet, before the hooks run, as the
    /// original sets both up first (@0x412e6-0x412fe, @0x41188-0x4156f),
    /// so the hooks' resets count its holds when it is a trader; its NPC
    /// is placed once the player has left it. The session then resets
    /// twice with the old ship in the fleet (@0x419fd, @0x41a3e) and once
    /// more as the plunder dialog closes (@0x94a0b), each trim cutting the
    /// commodities to the new fleet's holds while they are past them;
    /// every other assignment resets once, as the dialog closes (see the
    /// `precalced` module).
    pub fn assign(&mut self, choice: Assignment, chance: &mut dyn Chance) -> Option<Assigned> {
        let aboard = self.aboard.filter(|aboard| aboard.captured)?;
        self.aboard = None;
        let Some(npc) = self.npcs().iter().find(|npc| npc.id == aboard.npc).cloned() else {
            self.close_plunder();
            return None;
        };
        if choice == Assignment::Escort {
            self.ship_hook(npc.ship, ShipHook::Capture, chance);
            self.join_fleet(npc.id);
            self.close_plunder();
            return Some(Assigned::Escort);
        }
        let record = self.ship_record(npc.ship).cloned();
        let Some(record) = record.filter(|_| self.npcs().len() + 1 < MAX_SHIPS_IN_SYSTEM) else {
            self.close_plunder();
            return Some(Assigned::Abandoned);
        };
        let hooks_first = self.hook_rules().capture_hook_order == RuleSource::Engine;
        let stock = ShipStats::new(self.fields, &outfit_mods(&self.defaults, &self.outfits));
        let old = Escort {
            ship: self.pilot.ship,
            reserves: stock.full(),
            order: None,
            carried: false,
            wage: None,
            person: None,
        };
        // The commodities stay with the old ship, the player taking the
        // captured ship's, which the engine's ships carry none of
        // (@0x412e6-0x412fe, @0x41762-0x41779), as `capture_cargo` reads:
        // before the hooks, as the original swaps them.
        if self.rules.source_for(RuleKey::CaptureCargo) == RuleSource::Engine {
            self.pilot
                .cargo
                .retain(|good, _| !matches!(good, Good::Commodity(_)));
        }
        // The old ship joins the fleet before the hooks run
        // (@0x41188-0x4156f), so their resets count its holds when it is a
        // trader (@0x41662, @0x416c9); its NPC is placed once the player
        // has left it, below.
        self.pilot.escorts.push(old);
        let old_slot = self.pilot.escorts.len() - 1;
        self.fleet.resize(self.pilot.escorts.len(), None);
        if hooks_first {
            self.retire_for_capture(record.id, chance);
        }
        let defaults = pilot::tally(record.defaults.iter().copied());
        let records = &self.outfits;
        // Capture keeps by 0x0004 alone and holds nothing to its `Max`,
        // whatever the ship-change rules say, and tops the stock weapons
        // up rather than adding them.
        let mut outfits = carried_outfits(
            &self.pilot.outfits,
            OutfitCarry::Persistent {
                mask: OutfitFlags::PERSISTENT,
            },
            &defaults,
            None,
            None,
            records,
        );
        let fits = self.arsenal.stock_fits(record.id, records);
        fit_stock(&mut outfits, &fits, records);
        let share = if self.arsenal.hull(record.id).tough {
            TOUGH_TAKEOVER_ARMOR_SHARE
        } else {
            TAKEOVER_ARMOR_SHARE
        };
        let takeover = ClassChange {
            outfits,
            reserves: ReservePolicy::Hold,
        };
        self.become_class(&record, takeover, |session| {
            if !hooks_first {
                session.retire_for_capture(record.id, chance);
            }
            session.take_over_reserves(&record, share, chance);
        });
        // No secondary is selected on the captured ship (`_DoShipCapture`
        // @0x4173e).
        self.secondary = None;
        self.player = npc.state;
        for other in self.traffic.npcs_mut() {
            if other.leader == Some(npc.id) {
                other.leader = None;
            }
        }
        self.lose_captured(npc.id);
        self.leave(npc.id);
        let placed = self.place_escort(old_slot);
        if let Some(slot) = self.fleet.get_mut(old_slot) {
            *slot = placed;
        }
        self.reform();
        if let Some(id) = placed {
            self.snap(id);
        }
        // `_DoShipCapture`'s two resets (`_ShipStatsToSystemInfo` @0x419fd,
        // `_SystemInfoToShipStats` @0x41a3e), the old ship in the fleet,
        // then the plunder dialog's close (@0x94a0b): each trims again
        // while `jünk` or mission cargo keep the fleet past its holds.
        self.reset_precalced();
        self.reset_precalced();
        self.close_plunder();
        Some(Assigned::MyShip)
    }

    /// The reserves a ship of `record` is taken over with: no shield,
    /// `share` of its armour and [`TAKEOVER_ARMOR_BASE`], and its fuel
    /// drawn on `chance` below its `Fuel` (none, with no draw, for no
    /// tank).
    fn take_over_reserves(&mut self, record: &ShipRecord, share: f64, chance: &mut dyn Chance) {
        let reserves = &mut self.pilot.reserves;
        reserves.shield.now = 0.0;
        reserves.armor.now =
            f64::from(record.fields.armor).mul_add(share, TAKEOVER_ARMOR_BASE) as f32;
        reserves.fuel.now = u32::try_from(record.fields.fuel)
            .ok()
            .filter(|&fuel| fuel > 0)
            .map_or(0.0, |fuel| chance.below(fuel) as f32);
    }
}

/// An NPC as it was before a fight's tick: what the tick's disablings,
/// crimes and kills are read against, as the tick can change each (a
/// mission failed by its ship's loss can lose the ship its mark).
#[derive(Clone, Copy, Debug, PartialEq)]
struct NpcBefore {
    id: NpcId,
    condition: Condition,
    govt: Option<GovtId>,
    /// Whether it carried a mission's mark, of any slot.
    marked: bool,
}

impl NpcBefore {
    fn of(npc: &Npc) -> Self {
        Self {
            id: npc.id,
            condition: npc.condition,
            govt: npc.govt,
            marked: npc.mission.is_some(),
        }
    }
}

/// A boarding under way: the NPC boarded, its ship class, the plunder
/// rolled for it, and whether a capture awaits its assignment.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Aboard {
    npc: NpcId,
    ship: ShipId,
    plunder: Plunder,
    captured: bool,
}

impl Aboard {
    /// What is on board, as the plunder dialog shows it.
    fn view(&self) -> PlunderView {
        let plunder = &self.plunder;
        PlunderView {
            npc: self.npc,
            ship: self.ship,
            credits: plunder.credits,
            cargo: plunder.cargo,
            ammo: plunder.ammo,
            fuel: plunder.fuel,
            odds: plunder.odds,
        }
    }
}

#[cfg(test)]
impl Session {
    /// Puts the scene a system entry lets go of under way, aimed at
    /// `npc`: a strike on it, it targeted, boarded and hailed. The
    /// entry tests set the shots, the navigation target and a gate's
    /// pending entry themselves, as each can.
    fn stage_scene(&mut self, npc: NpcId) {
        self.strikes = vec![Strike {
            ship: ShipRef::Npc(npc),
            by: ShipRef::Player,
            damage: 1.0,
            downed: None,
            provokes: true,
        }];
        self.target = Some(npc);
        self.aboard = Some(Aboard {
            npc,
            ship: ShipId(129),
            plunder: Plunder::default(),
            captured: false,
        });
        self.talk = Some(hail::Talk::opened(npc));
    }

    /// Asserts the scene [`Session::stage_scene`] puts under way is let
    /// go of: no strike, target, boarding or hail.
    fn assert_scene_left(&self, case: &str) {
        assert_eq!(self.strikes, [], "the strikes: {case}");
        assert_eq!(self.target, None, "the target: {case}");
        assert_eq!(self.aboard, None, "the boarding: {case}");
        assert_eq!(self.talk, None, "the hail: {case}");
    }
}

/// The most restarts a counted sale makes before the port ends it (see
/// [`Session::outfit_order`]): the list's [`LIST_SLOTS`]. Without an
/// `OnSell` that changes the listing, a restart needs an outfit's
/// listing to change with its count owned, and after the first only
/// `oütf` 128 is sold, so a stock order restarts at most a few times; a
/// ceiling far above that keeps an order an `OnSell` relists forever, or
/// grows, from hanging. It is the only guard: there is no repeat check,
/// as `OnSell`'s random draws and state outside any cheap key can end an
/// order that revisits an earlier restart's state.
const RESTART_CEILING: usize = LIST_SLOTS as usize;

/// Whether an order changed the listing, and whether it ever left
/// nothing listed: what `_DoOutfitDialog` does after a rebuild that
/// `_SetupPortAvailableItems` says changed the list (@0xc216-0xc23d),
/// after the buy loop (@0x5c314) and after each sale (@0x5d00a), as
/// [`RuleKey::RelistedSelection`] reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Relisting {
    /// The listing changed: the selection is reset to `oütf` 128
    /// (@0x5c32c-0x5c33b, @0x5d01c-0x5d02b).
    changed: bool,
    /// A change left nothing listed: the leave flag -0x63d is set
    /// (@0x5c40c, @0x5d128), and never cleared by a later relisting.
    emptied: bool,
}

impl Relisting {
    /// Notes a rebuild from `before` to `after`.
    fn note(&mut self, before: &[OutfitId], after: &[OutfitId]) {
        if before != after {
            self.changed = true;
            self.emptied |= after.is_empty();
        }
    }

    /// What the order leaves the selection at.
    fn reselection(self) -> Reselection {
        if !self.changed {
            Reselection::Kept
        } else if self.emptied {
            Reselection::Leave
        } else {
            Reselection::Reset(FIRST_OUTFIT)
        }
    }
}

/// The outfits `outfitter` lists, in order; none when there is no
/// outfitter.
fn listed(outfitter: Option<&Outfitter>) -> Vec<OutfitId> {
    outfitter.map_or_else(Vec::new, |outfitter| {
        outfitter.rows.iter().map(|row| row.id).collect()
    })
}

/// The system a pilot resumes in from the stellar it last landed on,
/// which `listers` list (its
/// [`ListedStellar::systems`](crate::catalog::ListedStellar::systems), by
/// ascending ID), as `_LoadPilotData` sets the player's system from the
/// stellar's +0x14 alone (@0x7525b-0x75316); `None` when no system lists
/// it. The first that applies:
///
/// 1. Its owner ([`OverlaidGalaxy::owner`]), as `_PropagateMissionBitEffects`
///    gives it at the load's first refresh (@0x751ca, @0x99684-0x997bd).
/// 2. System 128 when it lists the stellar, hidden or shadowed: the first
///    refresh's `_HandleStellarSystemVisibility` gives an unowned stellar
///    the player's system's ID when that system lists it, before any
///    visibility test (@0x32c58-0x32ca1), and `_ResetPlayer` has just set
///    that system to 128 (@0x1d485). A 128 the catalog lacks lists
///    nothing, so it needs no existence test.
/// 3. The lowest visible lister, shadowed or not, and
/// 4. the lowest of any kind, even hidden: `_FindSystemFromStellar`'s two
///    passes (@0x7edc-0x7f56).
fn resume_system(galaxy: &OverlaidGalaxy, listers: &[SystemId]) -> Option<SystemId> {
    const RESET_SYSTEM: SystemId = SystemId(128);
    galaxy
        .owner(listers)
        .or_else(|| listers.contains(&RESET_SYSTEM).then_some(RESET_SYSTEM))
        .or_else(|| {
            listers
                .iter()
                .copied()
                .find(|&system| galaxy.exists(system) && galaxy.visible(system))
        })
        .or_else(|| listers.first().copied())
}

/// The lowest system on `galaxy`'s map that `catalog` can read: where a
/// load falls back to, as `_LoadPilotData` takes the first system with
/// +0x1ed set (@0x752ea-0x75316).
fn lowest_system(
    galaxy: &OverlaidGalaxy,
    catalog: &(impl PilotCatalog + ?Sized),
) -> Option<SystemId> {
    galaxy
        .map()
        .systems()
        .find(|&system| catalog.system_exists(system))
}

/// Where `pilot`, flown among its system's stellars `here`, resumes
/// docked: the stellar it last landed on, if it is among them; else, if
/// it is another system's at the same place, one the overlay hid and moved
/// it off (the original rewrites only the system numbers,
/// @0x998f1-0x99a36), at that stellar in that system, read from `catalog`
/// through `galaxy`'s overlay; else nowhere.
fn docking(
    galaxy: &OverlaidGalaxy,
    catalog: &(impl PilotCatalog + ?Sized),
    pilot: &Pilot,
    here: &[LandingSite],
) -> (Option<LandingSite>, Dock) {
    let Some(stellar) = pilot.stellar else {
        return (None, Dock::AsItWas);
    };
    let at = |sites: &[LandingSite]| sites.iter().find(|site| site.id == stellar).copied();
    if let Some(site) = at(here) {
        return (Some(site), Dock::AsItWas);
    }
    let system = pilot.system;
    for other in galaxy
        .map()
        .places()
        .at(system)
        .filter(|&other| other != system)
    {
        let sites = overlaid_sites(galaxy, catalog, other);
        if let Some(site) = at(&sites) {
            return (Some(site), Dock::At(other, sites));
        }
    }
    (None, Dock::AsItWas)
}

/// The stellars of `listed`, every one `system` lists, that it shows:
/// each it holds ([`OverlaidGalaxy::shows`]), and `docked`, the stellar
/// the ship is docked at, whatever holds it, in `listed`'s order.
///
/// The original's spaceport never asks whether its stellar is shown, so
/// the engine keeps the stellar docked at among the stellars shown, where
/// its readers find it (the ranks' port, the missions' visit, the dock).
fn shown_sites(
    galaxy: &OverlaidGalaxy,
    listed_stellars: &[ListedStellar],
    system: SystemId,
    listed: &[LandingSite],
    docked: Option<StellarId>,
) -> Vec<LandingSite> {
    listed
        .iter()
        .filter(|site| {
            docked == Some(site.id) || galaxy.shows(listers_of(listed_stellars, site.id), system)
        })
        .copied()
        .collect()
}

/// `system`'s stellars, read from `catalog`, each passed through
/// `galaxy`'s overlay ([`OverlaidGalaxy::site`]).
fn overlaid_sites(
    galaxy: &OverlaidGalaxy,
    catalog: &(impl PilotCatalog + ?Sized),
    system: SystemId,
) -> Vec<LandingSite> {
    catalog
        .landing_sites(system)
        .into_iter()
        .map(|site| galaxy.site(site))
        .collect()
}

/// The trigger the player's ship fights with this tick: `held`, its point
/// defence running every tick (`_HandlePlayer` @0x68c74; a disabled ship's
/// fires nothing,
/// [`Armament::fire_point_defence`](crate::combat::armament::Armament::fire_point_defence)).
fn player_trigger(held: Trigger) -> Trigger {
    Trigger {
        point_defence: true,
        ..held
    }
}

/// `value` held to the range of an `i16`, as a boarding grant reads a
/// raised `Max` (see [`GrantStock::max`]).
fn clamp_i16(value: i64) -> i16 {
    i16::try_from(value.clamp(i64::from(i16::MIN), i64::from(i16::MAX))).unwrap_or_default()
}

/// How a refit treats a shield or armour above its new most: the fuel is
/// always held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReservePolicy {
    /// Each is held to its new most.
    Hold,
    /// Each keeps its surplus, as the original's `_SystemInfoToShipStats`
    /// (@0xca33) clamps no reserve.
    KeepSurplus,
}

impl ReservePolicy {
    /// The policy a reserves rule's `source` reads as: the Bible's holds
    /// each to its most, the engine's keeps the surplus.
    fn reading(source: RuleSource) -> Self {
        match source {
            RuleSource::Bible => Self::Hold,
            RuleSource::Engine => Self::KeepSurplus,
        }
    }
}

/// Sets `gauge` to hold up to `max`, and when `hold`, keeping no more
/// than that; when `gain` and that is more than it held, it gains the
/// difference.
fn refit(gauge: &mut Gauge, max: f32, gain: bool, hold: bool) {
    if gain {
        gauge.now += (max - gauge.max).max(0.0);
    }
    gauge.max = max;
    if hold {
        gauge.now = gauge.now.min(max);
    }
}

/// Fills in what an old save of `pilot` left to the game data: the ship's
/// default items `defaults`, its stock weapons and their `AmmoLoad`
/// `stock` (on top of its outfits), and its name, after its class among
/// `ships`.
fn fill_in(
    pilot: &mut Pilot,
    defaults: &BTreeMap<OutfitId, u16>,
    stock: &BTreeMap<OutfitId, u16>,
    ships: &[ShipRecord],
) {
    if pilot.default_outfits_pending {
        pilot.outfits.clone_from(defaults);
        pilot.default_outfits_pending = false;
    }
    if pilot.stock_weapons_pending {
        pilot.outfits = pilot::merged(&pilot.outfits, stock);
        pilot.stock_weapons_pending = false;
    }
    if pilot.ship_name.is_none() {
        pilot.ship_name = Some(pilot::class_name(ships, pilot.ship));
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
pub(super) mod tests {
    use super::*;
    use crate::catalog::{CharacterStart, GateSite, LandingSite, SoundId, StellarId};
    use crate::catalog::{CommodityStrings, DisasterId, DisasterRecord, JunkId, JunkRecord};
    use crate::chance::NeverFires;
    use crate::clock::TICKS_PER_SECOND;
    use crate::control::{SetExpr, Test};
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::flight::Turn;
    use crate::fuel::FUEL_SCOOP;
    use crate::gate::{GateArrivalRule, GateKind, GateRefusal, HYPERGATE, WORMHOLE, WormholeRule};
    use crate::geometry::Vec2;
    use crate::glow::GLOW_CRUISE;
    use crate::handling::ShipFields;
    use crate::hyperspace::{
        ARRIVAL_DISTANCE, HyperSelectRule, HyperlinkRule, JumpReadiness, JumpRefusal, JumpZoneRule,
        MIN_JUMP_DISTANCE, RouteError, StarMap,
    };
    use crate::landing::StellarFlags;
    use crate::landing::{Clearance, LandOutcome, LandingRefusal};
    use crate::market::{Direction, Good, Lot, Order, TradeRefusal};
    use crate::pre_jump::slow_enough;
    use crate::reserves::{Gauge, Reserves};
    use crate::stats::{
        FAST_JUMP, FAST_JUMP_HULL, HYPERSPACE_DAYS, HYPERSPACE_DISTANCE, MULTI_JUMP,
    };
    use crate::testkit::{
        FAST, FakePilotCatalog, START, Scripted, begin_jump_now, catalog, edge_lander, fly_out,
        hideable, hiding_bit, jump, jump_with, land_now, outfit, planet, star, starting,
    };
    use entry::NoTraffic;

    #[test]
    fn a_session_flies_the_first_chärs_ship_with_its_handling() {
        let catalog = catalog();
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.ship(), ShipId(128));
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(
            session.handling(),
            crate::stats::ShipStats::new(FAST, &[]).handling
        );
        let asked = catalog.ships_asked.borrow();
        assert!(
            !asked.is_empty() && asked.iter().all(|&ship| ship == ShipId(128)),
            "{asked:?}"
        );
    }

    #[test]
    fn the_ship_starts_at_rest_at_the_centre_facing_up() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            *session.player(),
            ShipState {
                position: Vec2::ZERO,
                velocity: Vec2::ZERO,
                heading: 0.0,
            }
        );
        assert_eq!(
            session.reserves(),
            Reserves {
                shield: Gauge::full(30.0),
                armor: Gauge::full(45.0),
                fuel: Gauge::full(300.0),
            }
        );
    }

    #[test]
    fn the_interface_government_is_the_ship_types_then_its_combat_one() {
        let interface = |inherent: Option<i16>, combat: Option<i16>| {
            let mut record = crate::testkit::ship(128, FAST);
            record.inherent_govt = inherent.map(GovtId);
            record.combat_govt = combat.map(GovtId);
            let catalog = FakePilotCatalog {
                ship_records: vec![record],
                ..catalog()
            };
            Session::start(&catalog)
                .expect("starts")
                .interface_government()
        };
        assert_eq!(interface(Some(130), Some(131)), Some(GovtId(130)));
        assert_eq!(interface(Some(130), None), Some(GovtId(130)));
        assert_eq!(interface(None, Some(131)), Some(GovtId(131)));
        assert_eq!(interface(None, None), None);
        // No record for the ship type: no government.
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.interface_government(), None);
    }

    #[test]
    fn every_rule_reaches_the_session() {
        let every = RuleKey::ALL
            .into_iter()
            .fold(Rulebook::default(), |book, key| {
                book.with_override(key, RuleSource::Bible)
            });
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.rules(), &Rulebook::default());
        let session = session.with_rules(every);
        let rules = session.rules();
        assert_eq!(rules, &every);
        for key in RuleKey::ALL {
            assert_eq!(rules.source_for(key), RuleSource::Bible, "{key:?}");
        }
    }

    #[test]
    fn a_tick_leaves_the_reserves_as_they_are() {
        let mut session = Session::start(&catalog()).expect("starts");
        let full = session.reserves();
        for _ in 0..30 {
            session.tick(Controls {
                thrust: true,
                ..Controls::default()
            });
        }
        assert_eq!(session.reserves(), full);
        assert_ne!(session.player().position, Vec2::ZERO);
    }

    #[test]
    fn the_first_system_that_exists_is_the_start() {
        let pick = |slots| Session::start(&starting(slots)).map(|s| s.system());
        assert_eq!(
            pick([None, Some(999), Some(131), Some(130)]),
            Ok(SystemId(131))
        );
        assert_eq!(pick([Some(131), Some(130), None, None]), Ok(SystemId(131)));
        assert_eq!(pick([None, None, None, Some(130)]), Ok(SystemId(130)));
    }

    #[test]
    fn no_system_that_exists_is_an_error_naming_the_slots() {
        let slots = [None, Some(999), None, Some(-5)];
        assert_eq!(
            Session::start(&starting(slots)),
            Err(StartError::NoStartingSystem(slots.map(|s| s.map(SystemId))))
        );
        assert_eq!(
            Session::start(&starting([None; 4])),
            Err(StartError::NoStartingSystem([None; 4]))
        );
    }

    #[test]
    fn a_missing_or_broken_chär_is_its_error() {
        for error in [
            StartError::NoCharacter,
            StartError::Character("chär 128: too short".to_owned()),
        ] {
            let catalog = FakePilotCatalog {
                character: Err(error.clone()),
                ..catalog()
            };
            assert_eq!(Session::start(&catalog), Err(error));
            assert_eq!(*catalog.ships_asked.borrow(), []);
        }
    }

    #[test]
    fn a_chär_without_a_ship_is_an_error() {
        let catalog = FakePilotCatalog {
            character: Ok(CharacterStart {
                ship: None,
                systems: [Some(SystemId(130)), None, None, None],
                start: START,
                ..CharacterStart::default()
            }),
            ..catalog()
        };
        assert_eq!(Session::start(&catalog), Err(StartError::NoShip));
    }

    #[test]
    fn an_unreadable_ship_is_an_error_with_its_reason() {
        let catalog = FakePilotCatalog {
            ships: Vec::new(),
            ..catalog()
        };
        assert_eq!(
            Session::start(&catalog),
            Err(StartError::Ship(ShipId(128), "no shïp 128".to_owned()))
        );
    }

    /// The player's speed after `ticks` of thrust from rest.
    fn speed_after_thrust(session: &mut Session, ticks: usize) -> f32 {
        for _ in 0..ticks {
            session.tick(Controls {
                thrust: true,
                turn: Turn::None,
                reverse: false,
            });
        }
        session.player().velocity.length()
    }

    #[test]
    fn a_pilot_without_strict_play_tops_out_half_as_fast_again() {
        let mut session = Session::start(&catalog()).expect("starts");
        let own = session.handling().max_speed;
        assert!(own > 0.0);
        assert!((session.top_speed() - own * 1.5).abs() < 1e-4);
        let speed = speed_after_thrust(&mut session, 300);
        assert!((speed - own * 1.5).abs() < 1e-3, "{speed} vs {own}");
    }

    #[test]
    fn a_strict_play_pilot_tops_out_at_its_ships_top_speed() {
        let catalog = catalog();
        let pilot = Pilot::new(&catalog, "Ada")
            .expect("starts")
            .with_strict_play(true);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        let own = session.handling().max_speed;
        assert_eq!(session.top_speed().to_bits(), own.to_bits());
        let speed = speed_after_thrust(&mut session, 300);
        assert!((speed - own).abs() < 1e-3, "{speed} vs {own}");
    }

    #[test]
    fn the_other_non_strict_speed_reading_tops_out_at_the_ships_top_speed() {
        let mut session = Session::start(&catalog()).expect("starts").with_rules(
            Rulebook::default().with_override(RuleKey::NonStrictSpeed, RuleSource::Bible),
        );
        let own = session.handling().max_speed;
        assert_eq!(session.top_speed().to_bits(), own.to_bits());
        let speed = speed_after_thrust(&mut session, 300);
        assert!((speed - own).abs() < 1e-3, "{speed} vs {own}");
    }

    #[test]
    fn a_tick_steps_the_player_under_the_controls() {
        let mut session = Session::start(&catalog()).expect("starts");
        let controls = Controls {
            thrust: true,
            turn: Turn::Right,
            reverse: false,
        };
        let mut expected = *session.player();
        for _ in 0..5 {
            session.tick(controls);
            step(&mut expected, &session.handling(), controls);
        }
        assert_eq!(*session.player(), expected);
        assert_ne!(expected, ShipState::default());
    }

    // Landing.

    const THRUST: Controls = Controls {
        thrust: true,
        turn: Turn::None,
        reverse: false,
    };

    #[test]
    fn a_session_reads_its_systems_landing_sites_once() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(*catalog.sites_asked.borrow(), [SystemId(130)]);
        land_now(&mut session).expect("lands");
        session.take_off(&catalog, &mut NeverFires);
        land_now(&mut session).expect("lands again");
        assert_eq!(*catalog.sites_asked.borrow(), [SystemId(130)]);
        let other = starting([Some(131), None, None, None]);
        let mut session = Session::start(&other).expect("starts");
        assert_eq!(*other.sites_asked.borrow(), [SystemId(131)]);
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
    }

    #[test]
    fn a_new_pilots_record_is_clean() {
        let session = Session::start(&catalog()).expect("starts");
        for govt in [128, 140, 150] {
            assert_eq!(session.pilot().legal_record(GovtId(govt)), 0, "{govt}");
        }
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn landing_docks_the_ship_at_the_stellar_at_rest() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(Controls {
            turn: Turn::Right,
            ..THRUST
        });
        let flying = *session.player();
        assert_ne!(flying.velocity, Vec2::ZERO);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(session.landed(), Some(StellarId(128)));
        assert_eq!(
            *session.player(),
            ShipState {
                position: Vec2::new(30.0, -40.0),
                velocity: Vec2::ZERO,
                ..flying
            }
        );
    }

    #[test]
    fn a_tick_while_landed_moves_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        land_now(&mut session).expect("lands");
        let docked = *session.player();
        for _ in 0..10 {
            session.tick(Controls {
                turn: Turn::Left,
                ..THRUST
            });
        }
        assert_eq!(*session.player(), docked);
    }

    #[test]
    fn taking_off_leaves_the_ship_at_the_stellar_at_rest_and_it_flies_again() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(Controls {
            turn: Turn::Right,
            ..THRUST
        });
        land_now(&mut session).expect("lands");
        let docked = *session.player();
        assert_eq!(
            session.take_off(&NoTraffic, &mut NeverFires),
            Some(StellarId(128))
        );
        assert_eq!(session.landed(), None);
        assert_eq!(*session.player(), docked);
        assert_eq!(docked.position, Vec2::new(30.0, -40.0));
        assert_eq!(docked.velocity, Vec2::ZERO);
        session.tick(THRUST);
        let mut expected = docked;
        step(&mut expected, &session.handling(), THRUST);
        assert_eq!(*session.player(), expected);
        assert_ne!(expected, docked);
    }

    #[test]
    fn taking_off_without_landing_does_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(THRUST);
        let flying = *session.player();
        assert_eq!(session.take_off(&NoTraffic, &mut NeverFires), None);
        assert_eq!(*session.player(), flying);
        assert_eq!(
            session.take_off(&NoTraffic, &mut NeverFires),
            None,
            "nor twice"
        );
    }

    #[test]
    fn a_refused_landing_is_its_refusal_and_the_ship_flies_on() {
        let mut session = Session::start(&catalog()).expect("starts");
        for _ in 0..20 {
            session.tick(THRUST);
        }
        let flying = *session.player();
        let refusal = land_now(&mut session);
        assert_eq!(session.nav_target(), Some(StellarId(128)), "kept");
        assert_eq!(
            refusal,
            crate::landing::check_landing(
                &flying,
                &planet(128, 30.0, -40.0),
                |_| true,
                crate::landing::LandingReach::Box(175.0)
            )
            .map(|_| StellarId(128))
        );
        assert!(refusal.is_err(), "{refusal:?}");
        assert_eq!(session.landed(), None);
        assert_eq!(*session.player(), flying);
        session.tick(Controls::default());
        assert_ne!(*session.player(), flying, "still flying");

        let empty = FakePilotCatalog {
            sites: Vec::new(),
            ..catalog()
        };
        let mut session = Session::start(&empty).expect("starts");
        assert_eq!(land_now(&mut session), Err(LandingRefusal::NoStellars));
    }

    #[test]
    fn the_first_l_selects_the_nearest_landable_stellar_and_does_not_land() {
        let mut session = Session::start(&catalog()).expect("starts");
        let before = *session.player();
        assert_eq!(
            session.land(),
            Ok(LandOutcome::Selected {
                stellar: StellarId(128),
                station: false,
                clearance: Clearance::Granted,
            }
            .into())
        );
        assert_eq!(session.nav_target(), Some(StellarId(128)));
        assert_eq!(
            session.landed(),
            None,
            "over it and at rest, yet not landed"
        );
        assert_eq!(session.pilot().stellar(), None);
        assert_eq!(*session.player(), before);
        assert!(!session.take_save_due());
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn the_second_l_lands_on_the_target_and_clears_it() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.land().expect("selects");
        assert_eq!(
            session.land(),
            Ok(LandOutcome::Landed(StellarId(128)).into())
        );
        assert_eq!(session.landed(), Some(StellarId(128)));
        assert_eq!(session.nav_target(), None, "cleared on landing");
        assert!(session.take_save_due());
        assert_eq!(
            session.take_sounds(),
            [SimSound::Landed {
                stellar_sound: None
            }]
        );
        // Once off again, L requests clearance afresh.
        session.take_off(&NoTraffic, &mut NeverFires);
        assert!(matches!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Selected { .. }))
        ));
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn l_with_a_target_picked_by_its_slot_lands_on_it_at_once() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            session.select_stellar(StellarPick::Slot(0)),
            Some(StellarId(128))
        );
        assert_eq!(
            session.land(),
            Ok(LandOutcome::Landed(StellarId(128)).into())
        );
        // Planet 129 is far away: L refuses and keeps it.
        session.take_off(&NoTraffic, &mut NeverFires);
        assert_eq!(
            session.select_stellar(StellarPick::Slot(1)),
            Some(StellarId(129))
        );
        assert_eq!(
            session.land(),
            Err(LandingRefusal::TooFar {
                stellar: StellarId(129),
                station: false,
            })
        );
        assert_eq!(session.nav_target(), Some(StellarId(129)));
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn l_reads_the_pilots_record_for_the_clearance_it_gives() {
        let catalog = governed([Some(WANTED), Some(LAWFUL)], None);
        let mut session = Session::fly(&catalog, on_record(&catalog)).expect("flies");
        assert_eq!(
            session.land(),
            Ok(LandOutcome::Selected {
                stellar: StellarId(128),
                station: false,
                clearance: Clearance::Denied,
            }
            .into())
        );
        assert_eq!(session.nav_target(), Some(StellarId(128)), "still selected");
        let catalog = governed([None, None], Some(LAWFUL));
        let mut session = Session::fly(&catalog, on_record(&catalog)).expect("flies");
        assert!(matches!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Selected {
                clearance: Clearance::Granted,
                ..
            }))
        ));
    }

    /// Two governments, with a pilot's record of 10 and 9.
    const LAWFUL: GovtId = GovtId(140);
    const WANTED: GovtId = GovtId(150);

    /// [`catalog`] with system 130's planets, 128 and 129, each governed
    /// by `stellars` and needing a record of 10, and the system governed
    /// by `system`.
    fn governed(stellars: [Option<GovtId>; 2], system: Option<GovtId>) -> FakePilotCatalog {
        let needs = |site, govt| LandingSite {
            govt,
            min_status: 10,
            ..site
        };
        let mut catalog = catalog();
        catalog.sites[0].1 = vec![
            needs(planet(128, 30.0, -40.0), stellars[0]),
            needs(planet(129, 2000.0, 0.0), stellars[1]),
        ];
        catalog.star_map[0].govt = system;
        catalog
    }

    /// A new pilot of `catalog` whose record is 10 with [`LAWFUL`] and 9
    /// with [`WANTED`].
    fn on_record(catalog: &FakePilotCatalog) -> Pilot {
        let mut pilot = Pilot::new(catalog, "Law").expect("starts");
        pilot.set_legal_record(LAWFUL, 10);
        pilot.set_legal_record(WANTED, 9);
        pilot
    }

    /// Lands `pilot` on `stellar` in system 130 from right over it: docked
    /// there, taking off and landing again, the records read by the Bible.
    fn land_at(
        catalog: &FakePilotCatalog,
        pilot: &Pilot,
        stellar: i16,
    ) -> Result<StellarId, LandingRefusal> {
        let mut pilot = pilot.clone();
        pilot.stellar = Some(StellarId(stellar));
        let mut session = Session::fly(catalog, pilot)
            .expect("flies")
            .with_rules(bible_records());
        assert_eq!(
            session.take_off(&catalog, &mut NeverFires),
            Some(StellarId(stellar))
        );
        land_now(&mut session)
    }

    fn denied(stellar: i16) -> Result<StellarId, LandingRefusal> {
        Err(LandingRefusal::Denied {
            stellar: StellarId(stellar),
            station: false,
            min_status: 10,
        })
    }

    #[test]
    fn by_the_bible_landing_reads_the_pilots_record_with_each_stellars_government() {
        let catalog = governed([Some(LAWFUL), Some(WANTED)], None);
        let pilot = on_record(&catalog);
        assert_eq!(land_at(&catalog, &pilot, 128), Ok(StellarId(128)));
        assert_eq!(land_at(&catalog, &pilot, 129), denied(129));
    }

    #[test]
    fn by_the_bible_landing_on_a_stellar_without_a_government_reads_its_systems_record() {
        let lawful = governed([None, None], Some(LAWFUL));
        let pilot = on_record(&lawful);
        assert_eq!(land_at(&lawful, &pilot, 129), Ok(StellarId(129)));
        let wanted = governed([None, Some(LAWFUL)], Some(WANTED));
        assert_eq!(land_at(&wanted, &pilot, 128), denied(128));
        assert_eq!(land_at(&wanted, &pilot, 129), Ok(StellarId(129)));
    }

    #[test]
    fn by_the_bible_a_loaded_pilots_records_decide_landing() {
        let catalog = governed([Some(LAWFUL), Some(WANTED)], None);
        let loaded =
            crate::save::decode(&crate::save::encode(&on_record(&catalog))).expect("loads");
        assert_eq!(loaded.legal_record(LAWFUL), 10);
        assert_eq!(loaded.legal_record(WANTED), 9);
        assert_eq!(land_at(&catalog, &loaded, 128), Ok(StellarId(128)));
        assert_eq!(land_at(&catalog, &loaded, 129), denied(129));
    }

    /// [`governed`] with system 130 governed by [`WANTED`], 131 by
    /// [`LAWFUL`], 132 by a government with an `InitialRec` of -5 and no
    /// record, and 133 by one with an `InitialRec` but a record of 0.
    fn four_governed() -> FakePilotCatalog {
        let mut catalog = governed([None, None], Some(WANTED));
        catalog.star_map[1].govt = Some(LAWFUL);
        catalog.star_map[2].govt = Some(GovtId(160));
        catalog.star_map[3].govt = Some(GovtId(161));
        catalog.govts = vec![
            crate::catalog::GovtRecord {
                initial_rec: 3,
                ..crate::testkit::govt(140)
            },
            crate::testkit::govt(150),
            crate::catalog::GovtRecord {
                initial_rec: -5,
                ..crate::testkit::govt(160)
            },
            crate::catalog::GovtRecord {
                initial_rec: 7,
                ..crate::testkit::govt(161)
            },
        ];
        catalog
    }

    #[test]
    fn flying_a_pilot_derives_each_systems_record_from_its_government() {
        let catalog = four_governed();
        let mut pilot = on_record(&catalog);
        pilot.set_legal_record(GovtId(161), 0);
        pilot
            .galaxy
            .set_govt(SystemId(131), Some(Some(GovtId(160))));
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().system_records().collect::<Vec<_>>(),
            [(SystemId(130), 9), (SystemId(131), -5), (SystemId(132), -5)],
            "130 WANTED's record; 131 overlaid to 160 and 132, with no record \
             with 160, its InitialRec; 133's government has a record of 0, \
             which wins"
        );
    }

    #[test]
    fn flying_a_pilot_keeps_its_derived_system_records() {
        let catalog = four_governed();
        let mut pilot = on_record(&catalog);
        pilot.set_system_record(SystemId(130), -50);
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().system_records().collect::<Vec<_>>(),
            [(SystemId(130), -50)],
            "derived already, so nothing is derived again"
        );
    }

    /// Sets `session`'s legal record with `govt`, and in each system
    /// `govt` governs, to `record`: the record the session reads by
    /// either reading of [`RuleKey::LegalRecords`].
    pub(super) fn with_record(session: &mut Session, govt: GovtId, record: i16) {
        let systems: Vec<SystemId> = session
            .galaxy
            .governed()
            .filter(|&(_, governing)| governing == Some(govt))
            .map(|(system, _)| system)
            .collect();
        session.pilot.set_legal_record(govt, record);
        for system in systems {
            session.pilot.set_system_record(system, record);
        }
    }

    /// The default rulebook with the legal records read per government,
    /// as the Bible has them.
    fn bible_records() -> Rulebook {
        Rulebook::default().with_override(RuleKey::LegalRecords, RuleSource::Bible)
    }

    #[test]
    fn by_the_engine_the_record_is_the_systems_own() {
        let catalog = governed([None, None], Some(WANTED));
        let mut session = Session::fly(&catalog, on_record(&catalog)).expect("flies");
        session.pilot.set_system_record(SystemId(130), -3);
        assert_eq!(session.system_record(), -3);
        assert_eq!(session.pilot.legal_record(WANTED), 9, "kept");
        let session = session.with_rules(bible_records());
        assert_eq!(session.system_record(), 9, "by the Bible, WANTED's");
    }

    #[test]
    fn by_the_engine_landing_reads_the_systems_record_whatever_the_stellars_government() {
        let catalog = governed([Some(LAWFUL), None], Some(WANTED));
        let pilot = on_record(&catalog);
        let mut session = Session::fly(&catalog, pilot.clone()).expect("flies");
        assert_eq!(land_now(&mut session), denied(128), "system 130's 9");
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(bible_records());
        assert_eq!(
            land_now(&mut session),
            Ok(StellarId(128)),
            "by the Bible, LAWFUL's 10"
        );
    }

    // Hyperspace.

    fn ids(route: &[i16]) -> Vec<SystemId> {
        route.iter().copied().map(SystemId).collect()
    }

    fn dmy(session: &Session) -> (u8, u8, i32) {
        let date = session.date();
        (date.day(), date.month(), date.year())
    }

    #[test]
    fn a_session_reads_the_star_map_and_the_date_once_when_it_starts() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(dmy(&session), (23, 6, 1177));
        assert_eq!(*catalog.star_map_reads.borrow(), 1);
        session.plot_course(SystemId(132)).expect("a route");
        jump(&mut session, &catalog, 132);
        jump(&mut session, &catalog, 132);
        assert_eq!(*catalog.star_map_reads.borrow(), 1);
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.course(), []);
        assert_eq!(session.jumping(), None);
    }

    #[test]
    fn the_date_reads_with_the_chärs_affixes_and_moves_on_with_each_jump() {
        let catalog = FakePilotCatalog {
            date_affixes: DateAffixes {
                prefix: "Year ".to_owned(),
                suffix: " NC".to_owned(),
            },
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.date_text(), "Year June 23, 1177 NC");
        jump(&mut session, &catalog, 131);
        assert_eq!(session.date_text(), "Year June 24, 1177 NC");
        jump(&mut session, &catalog, 132);
        assert_eq!(session.date_text(), "Year June 25, 1177 NC");
        assert_eq!(*catalog.date_affix_reads.borrow(), 1);
    }

    #[test]
    fn a_flown_pilot_reads_its_date_with_the_chärs_affixes() {
        let catalog = catalog();
        let pilot = Pilot::new(&catalog, "Flown").expect("starts");
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.date_text(), "June 23, 1177 NC");
        assert_eq!(*catalog.date_affix_reads.borrow(), 1);
    }

    #[test]
    fn the_star_map_is_the_catalogs() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(*session.star_map(), StarMap::new(catalog().star_map));
        assert_eq!(
            session.star_map().position(SystemId(132)),
            Some(Vec2::new(600.0, 600.0))
        );
    }

    #[test]
    fn plotting_a_course_keeps_the_route_and_a_failed_plot_clears_it() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.course(), []);
        assert_eq!(
            session.plot_course(SystemId(132)),
            Ok(&ids(&[131, 132])[..])
        );
        assert_eq!(session.course(), ids(&[131, 132]));
        assert_eq!(session.plot_course(SystemId(131)), Ok(&ids(&[131])[..]));
        assert_eq!(session.course(), ids(&[131]));
        assert_eq!(
            session.plot_course(SystemId(133)),
            Err(RouteError::Unreachable)
        );
        assert_eq!(session.course(), []);
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(
            session.plot_course(SystemId(130)),
            Err(RouteError::AlreadyThere)
        );
        assert_eq!(session.course(), []);
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.plot_course(SystemId(999)), Err(RouteError::Unknown));
        assert_eq!(session.course(), []);
    }

    #[test]
    fn a_jump_is_refused_without_a_destination_too_close_or_without_fuel() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        session.plot_course(SystemId(131)).expect("a route");
        assert_eq!(
            session.begin_jump(),
            Err(JumpRefusal::TooClose { distance: 0.0 })
        );
        assert_eq!(session.jumping(), None);

        let empty = FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { fuel: 99, ..FAST }))],
            ..catalog()
        };
        let mut session = Session::start(&empty).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        assert_eq!(
            session.begin_jump(),
            Err(JumpRefusal::NoFuel { fuel: 99.0 })
        );
        assert_eq!(session.jumping(), None);
    }

    #[test]
    fn a_plotted_course_is_blocked_until_the_ship_is_out_and_clear_from_then() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            session.jump_readiness(),
            JumpReadiness::Blocked,
            "no course"
        );
        session.plot_course(SystemId(131)).expect("a route");
        assert_eq!(session.jump_readiness(), JumpReadiness::Blocked, "centre");
        session.player.position = Vec2::new(0.0, 999.9);
        assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
        session.player.position = Vec2::new(0.0, MIN_JUMP_DISTANCE);
        assert_eq!(session.jump_readiness(), JumpReadiness::Clear);
        session.player.position = Vec2::new(0.0, 500.0);
        assert_eq!(session.jump_readiness(), JumpReadiness::Blocked, "back in");
    }

    #[test]
    fn without_a_jumps_fuel_the_ship_is_never_clear() {
        let empty = FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { fuel: 99, ..FAST }))],
            ..catalog()
        };
        let mut session = Session::start(&empty).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
    }

    #[test]
    fn once_j_is_accepted_the_jump_is_underway_until_arrival() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        fly_out(&mut session);
        session.begin_jump().expect("pre-jump");
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.jump_readiness(), JumpReadiness::Underway);
        begin_jump_now(&mut session).expect("jumps");
        session.player.position = Vec2::ZERO;
        assert_eq!(session.jump_readiness(), JumpReadiness::Underway, "jumping");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.jump_readiness(), JumpReadiness::Clear, "arrived");
    }

    #[test]
    fn a_raised_jump_distance_moves_where_the_ship_becomes_clear() {
        let raised = owning(catalog(), HYPERSPACE_DISTANCE, 250, 1);
        let mut session = Session::start(&raised).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        session.player.position = Vec2::new(0.0, 1100.0);
        assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
        session.player.position = Vec2::new(0.0, 1250.0);
        assert_eq!(session.jump_readiness(), JumpReadiness::Clear);
    }

    #[test]
    fn a_landed_ship_is_blocked() {
        let catalog = edge_lander();
        let mut session = landed_at_the_edge(&catalog);
        assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.jump_readiness(), JumpReadiness::Clear, "once off");
    }

    /// [`catalog`] with system 130 holding `sites` alone, and 131 none.
    fn holding(sites: Vec<LandingSite>) -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![(SystemId(130), sites), (SystemId(131), vec![])],
            ..catalog()
        }
    }

    /// `catalog`'s session under `rule`, its course plotted to 131 and the
    /// ship at the centre.
    fn at_the_centre(catalog: &FakePilotCatalog, rule: JumpZoneRule) -> Session {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_jump_zone(rule);
        session.plot_course(SystemId(131)).expect("a route");
        session.player.position = Vec2::ZERO;
        session
    }

    #[test]
    fn by_the_engine_a_system_without_an_ordinary_stellar_has_no_zone() {
        let gates_only = holding(vec![
            gate_landing(300, HYPERGATE),
            gate_landing(301, WORMHOLE),
        ]);
        for catalog in [gates_only, holding(vec![])] {
            let mut session = at_the_centre(&catalog, JumpZoneRule::Engine);
            assert_eq!(session.jump_readiness(), JumpReadiness::Clear);
            assert_eq!(session.begin_jump(), Ok(SystemId(131)));
            let mut session = Session::start(&catalog).expect("starts");
            session.plot_course(SystemId(131)).expect("a route");
            assert_eq!(session.begin_jump(), Ok(SystemId(131)), "by default");
        }
    }

    #[test]
    fn under_the_bible_the_zone_applies_in_every_system() {
        let gates_only = holding(vec![gate_landing(300, HYPERGATE)]);
        for catalog in [gates_only, holding(vec![]), catalog()] {
            let mut session = at_the_centre(&catalog, JumpZoneRule::Always);
            assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
            assert_eq!(
                session.begin_jump(),
                Err(JumpRefusal::TooClose { distance: 0.0 })
            );
            session.player.position = Vec2::new(0.0, MIN_JUMP_DISTANCE);
            assert_eq!(session.jump_readiness(), JumpReadiness::Clear);
        }
    }

    #[test]
    fn without_a_zone_a_jump_still_needs_fuel() {
        let catalog = FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { fuel: 99, ..FAST }))],
            ..holding(vec![gate_landing(300, HYPERGATE)])
        };
        let mut session = at_the_centre(&catalog, JumpZoneRule::Engine);
        assert_eq!(
            session.begin_jump(),
            Err(JumpRefusal::NoFuel { fuel: 99.0 })
        );
        assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
    }

    #[test]
    fn the_zone_follows_the_system_the_ship_is_in() {
        // 130 holds a planet, 131 nothing and 132 a planet, and 132 links
        // back to 131.
        let catalog = FakePilotCatalog {
            systems: vec![SystemId(130), SystemId(131), SystemId(132)],
            sites: vec![
                (SystemId(130), vec![planet(128, 30.0, -40.0)]),
                (SystemId(131), vec![]),
                (SystemId(132), vec![planet(150, 0.0, 0.0)]),
            ],
            star_map: vec![
                star(130, (0.0, 0.0), &[131]),
                star(131, (600.0, 0.0), &[132]),
                star(132, (600.0, 600.0), &[131]),
            ],
            ..catalog()
        };
        for (rule, empty) in [
            (JumpZoneRule::Engine, JumpReadiness::Clear),
            (JumpZoneRule::Always, JumpReadiness::Blocked),
        ] {
            let mut session = at_the_centre(&catalog, rule);
            assert_eq!(session.jump_readiness(), JumpReadiness::Blocked, "130");
            session.plot_course(SystemId(132)).expect("a route");
            fly_out(&mut session);
            begin_jump_now(&mut session).expect("jumps");
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(131))
            );
            session.player.position = Vec2::ZERO;
            assert_eq!(session.jump_readiness(), empty, "131 under {rule:?}");
            fly_out(&mut session);
            begin_jump_now(&mut session).expect("jumps");
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(132))
            );
            session.plot_course(SystemId(131)).expect("a route back");
            session.player.position = Vec2::ZERO;
            assert_eq!(session.jump_readiness(), JumpReadiness::Blocked, "132");
        }
    }

    #[test]
    fn while_jumping_ticks_move_nothing() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        assert_eq!(session.jumping(), Some(SystemId(131)));
        let leaving = *session.player();
        for _ in 0..10 {
            session.tick(THRUST);
        }
        assert_eq!(*session.player(), leaving);
        assert_eq!(session.system(), SystemId(130), "not there yet");
    }

    #[test]
    fn arriving_takes_a_jumps_fuel_and_a_day() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert_eq!(
            session.reserves().fuel,
            Gauge {
                now: 200.0,
                max: 300.0
            }
        );
        assert_eq!(dmy(&session), (24, 6, 1177));
        let shield = session.reserves().shield;
        assert_eq!(shield, Gauge::full(30.0), "the other reserves carry over");
    }

    #[test]
    fn arriving_puts_the_ship_at_the_edge_facing_the_system_it_came_from() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.course(), ids(&[132]));
        assert_eq!(session.jumping(), None);
        assert_eq!(
            *session.player(),
            crate::hyperspace::arrival(
                Vec2::ZERO,
                Vec2::new(600.0, 0.0),
                crate::hyperspace::MIN_JUMP_DISTANCE
            )
        );
        assert_eq!(session.player().position, Vec2::new(-1001.0, 0.0));
    }

    #[test]
    fn arriving_reads_the_new_systems_stellars_and_landing_uses_them() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        assert_eq!(
            *catalog.sites_asked.borrow(),
            [SystemId(130), SystemId(131)]
        );
        // Planet 140 at 131's centre: drift there and land.
        let refused = land_now(&mut session);
        assert!(
            matches!(refused, Err(LandingRefusal::TooFar { stellar, .. }) if stellar == StellarId(140)),
            "{refused:?}"
        );
    }

    #[test]
    fn arriving_emits_an_arrived_message_naming_the_system() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        assert_eq!(
            session.take_messages(),
            [SimMessage::Arrived(SystemId(131))]
        );
        assert_eq!(session.take_messages(), [], "taking empties the list");
    }

    #[test]
    fn a_jumps_arrival_lets_go_of_the_strikes_the_boarding_and_the_hail() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        session.stage_scene(NpcId(0));
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        session.assert_scene_left("a jump");
    }

    #[test]
    fn no_message_before_arrival() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        assert_eq!(session.arrive(&catalog, &mut NeverFires), None);
        assert_eq!(session.take_messages(), [], "no jump, no message");
        fly_out(&mut session);
        session.begin_jump().expect("pre-jump");
        session.tick(Controls::default());
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.take_messages(), [], "pre-jump");
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(session.take_messages(), [], "jumping");
    }

    #[test]
    fn arriving_when_not_jumping_does_nothing() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        let before = session.clone();
        assert_eq!(session.arrive(&catalog, &mut NeverFires), None);
        assert_eq!(session, before);
        assert_eq!(*catalog.sites_asked.borrow(), [SystemId(130)]);
    }

    #[test]
    fn a_second_jump_continues_the_route_to_the_destination() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        jump(&mut session, &catalog, 132);
        assert_eq!(session.system(), SystemId(131));
        session.tick(Controls::default());
        assert_eq!(
            session.player().position,
            Vec2::new(-ARRIVAL_DISTANCE, 0.0),
            "resting at the edge"
        );
        assert_eq!(jump(&mut session, &catalog, 132), Some(SystemId(132)));
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.course(), []);
        assert_eq!(session.reserves().fuel.now, 100.0);
        assert_eq!(dmy(&session), (25, 6, 1177));
        // From 131, north of 132 on screen: it arrives at the top edge.
        assert_eq!(session.player().position, Vec2::new(0.0, -1001.0));
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    }

    // The pre-jump stage.

    /// A session on course for 131, due east on the map, with the ship
    /// put at `position`, moving at `velocity` and facing `heading`.
    pub(super) fn bound_for_131(
        catalog: &FakePilotCatalog,
        position: Vec2,
        velocity: Vec2,
        heading: f32,
    ) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        session.player = ShipState {
            position,
            velocity,
            heading,
        };
        session
    }

    /// Far enough out to jump, below the centre.
    pub(super) const OUT: Vec2 = Vec2 { x: 0.0, y: 1200.0 };

    #[test]
    fn a_jump_begun_facing_away_turns_at_the_turn_rate_and_begins_facing_the_bearing() {
        // AC1: at rest facing up, 90 degrees off east, at 3 degrees a tick.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 0.0);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.jumping(), None);
        for k in 1..=29_u8 {
            session.tick(Controls::default());
            assert_eq!(session.player().heading, 3.0 * f32::from(k), "tick {k}");
            assert_eq!(session.jumping(), None, "tick {k}");
            assert_eq!(session.preparing_jump(), Some(SystemId(131)), "tick {k}");
        }
        session.tick(Controls::default());
        assert_eq!(session.player().heading, 90.0);
        assert_eq!(session.jumping(), Some(SystemId(131)), "facing the bearing");
        assert_eq!(session.preparing_jump(), None);
        assert_eq!(session.player().position, OUT, "at rest throughout");
    }

    // The warp's clock.

    /// How many of `sounds` are the warp-up, [`SimSound::JumpBegan`].
    fn warp_ups(sounds: &[SimSound]) -> usize {
        sounds
            .iter()
            .filter(|sound| **sound == SimSound::JumpBegan)
            .count()
    }

    #[test]
    fn the_warp_clock_starts_on_the_turns_first_tick_and_not_while_braking() {
        // fly_out leaves the ship racing north: it brakes facing south,
        // then turns east.
        let mut session = bound_for_131(&catalog(), Vec2::ZERO, Vec2::ZERO, 0.0);
        fly_out(&mut session);
        session.take_sounds();
        session.begin_jump().expect("jumps");
        let mut braked = 0;
        while session.warp_clock().is_none() {
            assert_eq!(warp_ups(&session.take_sounds()), 0, "tick {braked}");
            assert!(braking_for_jump(&session), "tick {braked}");
            session.tick(Controls::default());
            braked += 1;
            assert!(braked < 1000, "never slowed");
        }
        assert!(braked > 1, "it braked first");
        assert_eq!(session.warp_clock(), Some(0), "the stamp (@0x6b5f5)");
        assert!(!braking_for_jump(&session));
        assert_eq!(session.preparing_jump(), Some(SystemId(131)), "turning");
        assert_eq!(
            warp_ups(&session.take_sounds()),
            1,
            "the warp-up (@0x6b690)"
        );
        let mut ticks = 0;
        while session.jumping().is_none() {
            session.tick(Controls::default());
            ticks += 1;
            assert_eq!(session.warp_clock(), Some(ticks));
        }
        assert!(ticks > 1, "it turned");
        session.tick(Controls::default());
        assert_eq!(session.warp_clock(), Some(ticks + 1), "on through the warp");
        assert_eq!(warp_ups(&session.take_sounds()), 0, "not again at the warp");
        session
            .arrive(&catalog(), &mut NeverFires)
            .expect("arrives");
        assert_eq!(session.warp_clock(), None, "it ends with the jump");
    }

    /// Whether `session`'s jump is still in its braking stage.
    fn braking_for_jump(session: &Session) -> bool {
        session.preparing_jump().is_some() && session.warp_clock().is_none()
    }

    #[test]
    fn braking_again_stops_the_warp_clock_and_slowing_again_restarts_it() {
        // At rest facing up: J turns at once.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 0.0);
        session.begin_jump().expect("jumps");
        assert_eq!(session.warp_clock(), Some(0), "on J");
        assert_eq!(warp_ups(&session.take_sounds()), 1);
        for _ in 0..3 {
            session.tick(Controls::default());
        }
        assert_eq!(session.warp_clock(), Some(3));
        // Knocked back past the jump speed, as the restamp reads it
        // (@0x6b6a2-0x6b6b1).
        session.player.velocity = Vec2::new(0.0, 5.0);
        session.tick(Controls::default());
        assert_eq!(session.warp_clock(), None, "braking again");
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        let mut braked = 0;
        while session.warp_clock().is_none() {
            session.tick(Controls::default());
            braked += 1;
            assert!(braked < 1000, "never slowed");
        }
        assert_eq!(session.warp_clock(), Some(0), "stamped afresh");
        assert_eq!(warp_ups(&session.take_sounds()), 1, "the warp-up again");
    }

    #[test]
    fn a_jump_given_up_ends_the_warp_clock() {
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 0.0);
        session.begin_jump().expect("jumps");
        assert_eq!(session.warp_clock(), Some(0));
        session.condition = Condition::Disabled;
        session.tick(Controls::default());
        assert_eq!(session.preparing_jump(), None, "given up");
        assert_eq!(session.warp_clock(), None);
    }

    /// Flies out with `catalog`'s ship, begins a jump and checks that it
    /// turns to the bearing and jumps without ever slowing down.
    fn assert_fast_jumps(catalog: &FakePilotCatalog) {
        let mut session = bound_for_131(catalog, Vec2::ZERO, Vec2::ZERO, 0.0);
        fly_out(&mut session);
        let racing = session.player().velocity.length();
        assert!(racing > 5.0, "{:?}", session.player());
        session.begin_jump().expect("jumps");
        for _ in 0..1000 {
            if session.jumping().is_some() {
                break;
            }
            session.tick(Controls::default());
            let now = *session.player();
            assert!(now.velocity.length() >= racing, "slowed: {now:?}");
        }
        assert_eq!(session.jumping(), Some(SystemId(131)));
        let leaving = *session.player();
        assert_eq!(leaving.heading, 90.0, "it still turned");
        assert!(!slow_enough(leaving.velocity), "{leaving:?}");
    }

    #[test]
    fn a_ship_with_a_fast_jump_outfit_jumps_without_slowing() {
        assert_fast_jumps(&FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(OutfitId(200), 1)])],
            outfits: vec![outfit(200, &[(FAST_JUMP, 1)])],
            ..catalog()
        });
    }

    #[test]
    fn a_fast_jumping_hull_jumps_without_slowing() {
        assert_fast_jumps(&FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    flags2: FAST_JUMP_HULL,
                    ..FAST
                }),
            )],
            ..catalog()
        });
    }

    #[test]
    fn a_jump_begun_while_moving_slows_the_ship_before_it_begins() {
        // AC2: fly_out leaves the ship racing north, facing it. A ship with
        // neither a fast-jump outfit nor a fast-jumping hull slows first.
        let mut session = bound_for_131(&catalog(), Vec2::ZERO, Vec2::ZERO, 0.0);
        fly_out(&mut session);
        let racing = session.player().velocity.length();
        assert!(racing > 5.0, "{:?}", session.player());
        session.begin_jump().expect("jumps");
        let mut slowest = racing;
        for _ in 0..1000 {
            if session.jumping().is_some() {
                break;
            }
            let before = *session.player();
            assert!(
                !(slow_enough(before.velocity) && before.heading == 90.0),
                "ready but still waiting: {before:?}"
            );
            session.tick(Controls::default());
            slowest = slowest.min(session.player().velocity.length());
        }
        assert_eq!(session.jumping(), Some(SystemId(131)));
        let leaving = *session.player();
        assert!(slow_enough(leaving.velocity), "{leaving:?}");
        assert!(leaving.velocity.length() < racing - 3.0, "{leaving:?}");
        assert_eq!(leaving.velocity.length(), slowest, "{leaving:?}");
        assert_eq!(leaving.heading, 90.0);
    }

    #[test]
    fn a_jump_begun_at_rest_facing_the_bearing_begins_at_once() {
        // AC3.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 90.0);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(session.jumping(), Some(SystemId(131)));
        assert_eq!(session.preparing_jump(), None);
        assert_eq!(session.take_sounds(), [SimSound::JumpBegan], "one warp-up");
        assert_eq!(
            session.warp_clock(),
            Some(0),
            "the clock starts with the warp"
        );
    }

    #[test]
    fn a_jump_begun_drifting_slowly_facing_the_bearing_begins_at_once() {
        let drifting = Vec2::new(1.9, -1.9);
        let mut session = bound_for_131(&catalog(), OUT, drifting, 90.0);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(session.jumping(), Some(SystemId(131)));
    }

    #[test]
    fn the_players_controls_are_ignored_during_the_pre_jump_stage() {
        let mut session = bound_for_131(&catalog(), Vec2::ZERO, Vec2::ZERO, 0.0);
        fly_out(&mut session);
        session.begin_jump().expect("jumps");
        let mut steered = session.clone();
        let fighting = Controls {
            thrust: true,
            turn: Turn::Left,
            reverse: false,
        };
        while session.jumping().is_none() {
            session.tick(Controls::default());
            steered.tick(fighting);
            assert_eq!(steered, session);
        }
    }

    #[test]
    fn braking_sounds_its_thrust_and_glows_and_the_jump_puts_both_out() {
        let mut session = bound_for_131(&catalog(), Vec2::ZERO, Vec2::ZERO, 0.0);
        fly_out(&mut session);
        session.tick(Controls::default());
        session.take_sounds();
        session.begin_jump().expect("jumps");
        assert_eq!(session.take_sounds(), [], "nothing until it begins");
        let mut sounds = Vec::new();
        let mut brightest = 0;
        while session.jumping().is_none() {
            session.tick(Controls::default());
            brightest = brightest.max(session.engine_glow());
            sounds.extend(session.take_sounds());
        }
        assert_eq!(sounds.first(), Some(&SimSound::ThrustStarted), "braking");
        // The warp-up sounds as the brake ends and the warp stage begins
        // (@0x6b690), and the thrust stops as the turn takes over.
        assert_eq!(warp_ups(&sounds), 1);
        let warp_up = sounds.iter().position(|s| *s == SimSound::JumpBegan);
        assert!(
            warp_up.is_some_and(|at| at > 0),
            "after the brake: {sounds:?}"
        );
        assert_eq!(sounds.last(), Some(&SimSound::ThrustStopped));
        assert!(brightest > 0, "the brake glows");
        assert_eq!(session.engine_glow(), 0, "jumping");
        assert!(!session.thrusting());
    }

    #[test]
    fn a_jump_that_begins_mid_thrust_stops_the_thrust_first() {
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 90.0);
        session.tick(THRUST);
        session.take_sounds();
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(
            session.take_sounds(),
            [SimSound::ThrustStopped, SimSound::JumpBegan]
        );
        assert_eq!(session.engine_glow(), 0);
    }

    #[test]
    fn fuel_regenerates_during_the_pre_jump_stage() {
        let catalog = regenerating(2);
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 0.0);
        session.pilot.reserves.fuel.now = 150.0;
        session.begin_jump().expect("jumps");
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.reserves().fuel.now, 155.0);
    }

    #[test]
    fn j_again_during_the_pre_jump_stage_changes_nothing() {
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 0.0);
        session.begin_jump().expect("jumps");
        session.tick(Controls::default());
        session.take_sounds();
        let turning = session.clone();
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(session, turning, "no restart, no sound");
    }

    #[test]
    fn a_jump_is_committed_even_once_braking_carries_the_ship_inside_the_no_jump_zone() {
        // Just outside the zone, racing inward and facing along its motion:
        // it coasts a long way in while it turns to brake.
        let edge = Vec2::new(0.0, -ARRIVAL_DISTANCE);
        let mut session = bound_for_131(&catalog(), edge, Vec2::new(0.0, 6.0), 180.0);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        for _ in 0..30 {
            session.tick(Controls::default());
        }
        let inside = session.player().position.length();
        assert!(inside < MIN_JUMP_DISTANCE, "inside: {inside}");
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        session.take_sounds();
        let braking = session.clone();
        assert_eq!(session.begin_jump(), Ok(SystemId(131)), "J again");
        assert_eq!(session, braking, "no restart, no sound, not refused");
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(session.jumping(), Some(SystemId(131)), "still goes ahead");
    }

    #[test]
    fn landing_is_refused_during_the_pre_jump_stage() {
        let mut session = bound_for_131(&catalog(), Vec2::new(30.0, -40.0), Vec2::ZERO, 0.0);
        session.stats.jump_distance = 0.0;
        session.begin_jump().expect("jumps from over planet 128");
        session.take_sounds();
        let turning = session.clone();
        assert_eq!(land_now(&mut session), Err(LandingRefusal::Jumping));
        assert_eq!(session, turning, "nothing changes");
    }

    #[test]
    fn arriving_during_the_pre_jump_stage_does_nothing() {
        let catalog = catalog();
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 0.0);
        session.begin_jump().expect("jumps");
        let turning = session.clone();
        assert_eq!(session.arrive(&catalog, &mut NeverFires), None);
        assert_eq!(session, turning);
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
    }

    #[test]
    fn the_jump_goes_where_it_was_bound_when_j_was_pressed() {
        let catalog = catalog();
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 0.0);
        session.begin_jump().expect("jumps");
        assert_eq!(
            session.plot_course(SystemId(133)),
            Err(RouteError::Unreachable)
        );
        assert_eq!(session.course(), [], "the course is gone");
        session.tick(Controls::default());
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(session.jumping(), Some(SystemId(131)));
        assert_eq!(session.player().heading, 90.0, "the bearing to 131");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
    }

    /// The catalog with ship 128 regenerating a unit of fuel every
    /// `regen` ticks.
    fn regenerating(regen: i16) -> FakePilotCatalog {
        FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    fuel_regen: regen,
                    ..FAST
                }),
            )],
            ..catalog()
        }
    }

    #[test]
    fn fuel_regenerates_each_tick_at_the_ships_rate_up_to_full() {
        let catalog = regenerating(2);
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.fuel_regen_per_tick(), 0.5);
        jump(&mut session, &catalog, 131);
        assert_eq!(session.reserves().fuel.now, 200.0);
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, 205.0);
        for _ in 0..1000 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel, Gauge::full(300.0));
        let still = Session::start(&self::catalog()).expect("starts");
        assert_eq!(still.fuel_regen_per_tick(), 0.0);
    }

    #[test]
    fn the_ships_default_outfits_add_to_its_fuel_regeneration() {
        // A unit every 8 ticks from the ship, and two scoops each giving a
        // unit every 4 ticks; a mod of another type gives nothing.
        let catalog = FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(OutfitId(200), 2), (OutfitId(201), 1)])],
            outfits: vec![outfit(200, &[(FUEL_SCOOP, 4)]), outfit(201, &[(1, 1)])],
            ..regenerating(8)
        };
        let mut session = Session::start(&catalog).expect("starts");
        let read = (
            catalog.defaults_asked.borrow().clone(),
            *catalog.outfit_reads.borrow(),
        );
        assert!(read.0.iter().all(|&ship| ship == ShipId(128)), "{read:?}");
        assert_eq!(session.fuel_regen_per_tick(), 0.125 + 0.5);
        jump(&mut session, &catalog, 131);
        for _ in 0..8 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, 205.0);
        assert_eq!(
            (
                catalog.defaults_asked.borrow().clone(),
                *catalog.outfit_reads.borrow()
            ),
            read,
            "read only when it starts"
        );
    }

    /// A session landed on planet 140 at 131's edge, far enough out to
    /// jump, with fuel and 132 still to go.
    fn landed_at_the_edge(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        jump(&mut session, catalog, 132);
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        session
    }

    #[test]
    fn a_jump_is_refused_while_landed() {
        let catalog = edge_lander();
        let mut session = landed_at_the_edge(&catalog);
        let docked = session.clone();
        assert_eq!(session.begin_jump(), Err(JumpRefusal::Landed));
        assert_eq!(session, docked, "nothing changes");
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(132)), "once off");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn landing_is_refused_while_jumping() {
        let catalog = edge_lander();
        let mut session = landed_at_the_edge(&catalog);
        session.take_off(&catalog, &mut NeverFires);
        begin_jump_now(&mut session).expect("jumps from over planet 140");
        let jumping = session.clone();
        assert_eq!(land_now(&mut session), Err(LandingRefusal::Jumping));
        assert_eq!(session, jumping, "nothing changes");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn fuel_does_not_regenerate_while_landed_or_jumping() {
        let catalog = edge_lander();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        jump(&mut session, &catalog, 132);
        assert_eq!(session.reserves().fuel.now, 200.0);
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, 200.0);
        session.take_off(&catalog, &mut NeverFires);
        session.tick(Controls::default());
        assert_eq!(session.reserves().fuel.now, 201.0);

        begin_jump_now(&mut session).expect("jumps from the edge");
        let leaving = session.reserves().fuel.now;
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, leaving, "in hyperspace");
    }

    // The pilot.

    #[test]
    fn landing_records_the_stellar_on_the_pilot_and_a_save_is_due() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert!(!session.take_save_due(), "nothing to save yet");
        assert_eq!(session.pilot().stellar(), None);
        land_now(&mut session).expect("lands");
        assert_eq!(session.pilot().stellar(), Some(StellarId(128)));
        assert!(session.take_save_due());
        assert!(!session.take_save_due(), "taking it clears it");
    }

    #[test]
    fn a_refused_landing_records_nothing_and_no_save_is_due() {
        let mut session = Session::start(&catalog()).expect("starts");
        for _ in 0..20 {
            session.tick(THRUST);
        }
        land_now(&mut session).expect_err("refused");
        assert_eq!(session.pilot().stellar(), None);
        assert!(!session.take_save_due());
    }

    #[test]
    fn taking_off_keeps_the_stellar_and_a_save_is_due() {
        let mut session = Session::start(&catalog()).expect("starts");
        land_now(&mut session).expect("lands");
        session.take_save_due();
        session.take_off(&NoTraffic, &mut NeverFires);
        assert_eq!(session.pilot().stellar(), Some(StellarId(128)));
        assert!(session.take_save_due());
        session.take_off(&NoTraffic, &mut NeverFires);
        assert!(!session.take_save_due(), "not landed: nothing happens");
    }

    #[test]
    fn a_transaction_while_landed_changes_the_pilot_and_a_save_is_due() {
        let mut session = Session::start(&catalog()).expect("starts");
        land_now(&mut session).expect("lands");
        session.take_save_due();
        assert!(session.transact(|pilot| pilot.set_cash(500)));
        assert_eq!(session.pilot().cash(), 500);
        assert!(session.take_save_due());
    }

    #[test]
    fn a_transaction_in_flight_is_refused_and_changes_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        let before = session.clone();
        let mut called = false;
        assert!(!session.transact(|pilot| {
            called = true;
            pilot.set_cash(500);
        }));
        assert!(!called);
        assert_eq!(session, before);
        assert!(!session.take_save_due());
    }

    #[test]
    fn arriving_explores_the_system_and_forgets_the_stellar() {
        let catalog = edge_lander();
        let mut session = landed_at_the_edge(&catalog);
        assert_eq!(session.pilot().stellar(), Some(StellarId(140)));
        assert_eq!(
            session.pilot().explored().collect::<Vec<_>>(),
            [SystemId(130), SystemId(131)]
        );
        session.take_off(&catalog, &mut NeverFires);
        session.take_save_due();
        begin_jump_now(&mut session).expect("jumps from the edge");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        let pilot = session.pilot();
        assert_eq!(pilot.system(), SystemId(132));
        assert_eq!(pilot.stellar(), None);
        assert_eq!(
            pilot.explored().collect::<Vec<_>>(),
            [SystemId(130), SystemId(131), SystemId(132)]
        );
        assert!(!session.take_save_due(), "a jump alone saves nothing");
    }

    #[test]
    fn a_strict_play_pilots_jump_makes_no_save_due() {
        let catalog = edge_lander();
        let mut session = landed_at_the_edge(&catalog);
        session.pilot = session.pilot.clone().with_strict_play(true);
        session.take_off(&catalog, &mut NeverFires);
        session.take_save_due();
        begin_jump_now(&mut session).expect("jumps from the edge");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.pilot().system(), SystemId(132));
        assert!(
            !session.take_save_due(),
            "the original saves on no jump arrival, strict or not"
        );
    }

    /// A pilot landed on planet 128 in system 130, with 7 credits and a
    /// day gone by, taken from a session.
    fn landed_pilot(catalog: &FakePilotCatalog) -> Pilot {
        let mut session = Session::start(catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session.transact(|pilot| pilot.set_cash(7));
        session.pilot().clone()
    }

    #[test]
    fn flying_a_pilot_landed_at_a_stellar_resumes_docked_there() {
        let catalog = catalog();
        let pilot = landed_pilot(&catalog);
        let session = Session::fly(&catalog, pilot.clone()).expect("flies");
        session.assert_docked_among_shown();
        assert_eq!(session.landed(), Some(StellarId(128)));
        assert_eq!(session.player().position, Vec2::new(30.0, -40.0));
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(*session.pilot(), pilot);
        let mut session = session;
        assert_eq!(session.take_sounds(), [], "no landing sound");
        assert!(!session.take_save_due());
    }

    #[test]
    fn flying_a_pilot_whose_stellar_is_gone_starts_at_the_centre() {
        let pilot = landed_pilot(&catalog());
        let moved = FakePilotCatalog {
            sites: vec![(SystemId(130), vec![planet(129, 2000.0, 0.0)])],
            ..catalog()
        };
        let session = Session::fly(&moved, pilot).expect("flies");
        assert_eq!(session.landed(), None);
        assert_eq!(*session.player(), ShipState::default());
        assert_eq!(session.pilot().stellar(), None);
    }

    /// [`landed_pilot`] moved to system 131 and last landed on stellar
    /// 999, which no system lists.
    fn unlisted_pilot(catalog: &FakePilotCatalog) -> Pilot {
        let mut pilot = landed_pilot(catalog);
        pilot.system = SystemId(131);
        pilot.stellar = Some(StellarId(999));
        pilot
    }

    fn engine_unlisted() -> Rulebook {
        Rulebook::default().with_override(RuleKey::UnlistedStellar, RuleSource::Engine)
    }

    /// Asserts `session` flies undocked at its system's centre, having
    /// forgotten its stellar.
    fn assert_undocked_at_centre(session: &Session) {
        assert_eq!(session.landed(), None);
        assert_eq!(*session.player(), ShipState::default());
        assert_eq!(session.pilot().stellar(), None);
    }

    #[test]
    fn loading_a_pilot_whose_stellar_no_system_lists_keeps_its_system_by_default_and_says_so() {
        let catalog = catalog();
        let session =
            Session::load(&catalog, unlisted_pilot(&catalog), &Rulebook::default()).expect("loads");
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.pilot().system(), SystemId(131));
        assert_undocked_at_centre(&session);
        assert!(session.load_repaired());
    }

    #[test]
    fn loading_an_unlisted_stellar_by_the_engines_reading_moves_to_the_lowest_system() {
        let catalog = catalog();
        let session =
            Session::load(&catalog, unlisted_pilot(&catalog), &engine_unlisted()).expect("loads");
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.pilot().system(), SystemId(130));
        assert_undocked_at_centre(&session);
        assert!(session.load_repaired());
    }

    #[test]
    fn loading_a_never_landed_pilot_repairs_nothing_under_either_reading() {
        let catalog = catalog();
        let mut pilot = Session::start(&catalog).expect("starts").pilot().clone();
        pilot.system = SystemId(131);
        assert_eq!(pilot.stellar(), None);
        for rules in [Rulebook::default(), engine_unlisted()] {
            let session = Session::load(&catalog, pilot.clone(), &rules).expect("loads");
            assert_eq!(session.system(), SystemId(131), "{rules:?}");
            assert!(!session.load_repaired(), "{rules:?}");
        }
    }

    #[test]
    fn loading_a_pilot_whose_system_is_gone_moves_to_the_lowest_that_exists() {
        // 130 is gone with its stellars, so no system lists 128 either;
        // the star map still has 130, 132 and 133, none of which exists.
        let gone = FakePilotCatalog {
            systems: vec![SystemId(131)],
            sites: vec![(SystemId(131), vec![planet(140, 0.0, 0.0)])],
            ..catalog()
        };
        let mut never_landed = Session::start(&catalog()).expect("starts").pilot().clone();
        never_landed.stellar = None;
        let mut unlisted = landed_pilot(&catalog());
        unlisted.stellar = Some(StellarId(999));
        for pilot in [never_landed, unlisted] {
            assert_eq!(pilot.system(), SystemId(130));
            for rules in [Rulebook::default(), engine_unlisted()] {
                let session = Session::load(&gone, pilot.clone(), &rules).expect("loads");
                assert_eq!(session.system(), SystemId(131), "{rules:?}");
                assert_eq!(session.pilot().system(), SystemId(131), "{rules:?}");
                assert_undocked_at_centre(&session);
                assert!(session.load_repaired(), "{rules:?}");
            }
        }
    }

    #[test]
    fn loading_with_no_system_at_all_is_still_no_system() {
        let pilot = landed_pilot(&catalog());
        let none = FakePilotCatalog {
            systems: vec![],
            ..catalog()
        };
        for rules in [Rulebook::default(), engine_unlisted()] {
            assert_eq!(
                Session::load(&none, pilot.clone(), &rules),
                Err(StartError::NoSystem(SystemId(130)))
            );
        }
        let mut moved = pilot;
        moved.stellar = Some(StellarId(999));
        assert_eq!(
            Session::load(&none, moved, &engine_unlisted()),
            Err(StartError::NoSystem(SystemId(130))),
            "the saved system, not one a fallback chose"
        );
    }

    #[test]
    fn loading_a_pilot_whose_system_is_gone_but_whose_stellar_is_listed_resumes_as_phase_13_places_it()
     {
        let catalog = catalog();
        let mut pilot = landed_pilot(&catalog);
        pilot.system = SystemId(133);
        pilot.stellar = Some(StellarId(140));
        for rules in [Rulebook::default(), engine_unlisted()] {
            let session = Session::load(&catalog, pilot.clone(), &rules).expect("loads");
            assert_eq!(session.system(), SystemId(131), "{rules:?}");
            assert_eq!(session.landed(), Some(StellarId(140)), "{rules:?}");
            assert!(!session.load_repaired(), "{rules:?}");
        }
    }

    #[test]
    fn a_normal_load_repairs_nothing() {
        let catalog = catalog();
        let pilot = landed_pilot(&catalog);
        for rules in [Rulebook::default(), engine_unlisted()] {
            let session = Session::load(&catalog, pilot.clone(), &rules).expect("loads");
            assert_eq!(session.landed(), Some(StellarId(128)));
            assert!(!session.load_repaired());
        }
        assert!(!Session::start(&catalog).expect("starts").load_repaired());
    }

    #[test]
    fn fly_reads_the_default_rulebook() {
        let catalog = catalog();
        let session = Session::fly(&catalog, unlisted_pilot(&catalog)).expect("flies");
        assert_eq!(session.system(), SystemId(131));
        assert!(session.load_repaired());
    }

    /// [`catalog`] with 131's planet 140 at (50, 60), off its centre.
    fn off_centre() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![
                (
                    SystemId(130),
                    vec![planet(128, 30.0, -40.0), planet(129, 2000.0, 0.0)],
                ),
                (SystemId(131), vec![planet(140, 50.0, 60.0)]),
            ],
            ..catalog()
        }
    }

    #[test]
    fn flying_a_pilot_whose_stellar_another_system_owns_resumes_docked_there() {
        let catalog = off_centre();
        let mut pilot = landed_pilot(&catalog);
        pilot.stellar = Some(StellarId(140));
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.pilot().system(), SystemId(131));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.landed(), Some(StellarId(140)));
        assert_eq!(session.player().position, Vec2::new(50.0, 60.0));
        session.assert_docked_among_shown();
    }

    #[test]
    fn flying_a_pilot_whose_system_is_gone_resumes_in_its_stellars_owner() {
        let mut catalog = off_centre();
        let mut pilot = landed_pilot(&catalog);
        pilot.stellar = Some(StellarId(140));
        catalog.systems = vec![SystemId(131)];
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.pilot().system(), SystemId(131));
        assert_eq!(session.landed(), Some(StellarId(140)));
        assert_eq!(session.player().position, Vec2::new(50.0, 60.0));
        session.assert_docked_among_shown();
    }

    #[test]
    fn a_stellar_its_saved_system_lists_but_another_owns_resumes_at_the_owner() {
        // 130 lists 140 too, at its own place, lower than 131.
        let catalog = FakePilotCatalog {
            sites: vec![
                (
                    SystemId(130),
                    vec![planet(128, 30.0, -40.0), planet(140, 70.0, 80.0)],
                ),
                (SystemId(131), vec![planet(140, 50.0, 60.0)]),
            ],
            ..catalog()
        };
        let mut pilot = landed_pilot(&catalog);
        pilot.system = SystemId(131);
        pilot.stellar = Some(StellarId(140));
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.pilot().system(), SystemId(130));
        assert_eq!(session.landed(), Some(StellarId(140)));
        assert_eq!(session.player().position, Vec2::new(70.0, 80.0));
        session.assert_docked_among_shown();
    }

    /// `systems`, each `(id, place)`, with every system the overlay
    /// hides in `hidden`.
    fn galaxy_of(systems: &[(i16, (f32, f32))], hidden: &[i16]) -> OverlaidGalaxy {
        let mut overlay = crate::story::GalaxyOverlay::default();
        for &system in hidden {
            overlay.set_visible(SystemId(system), Some(false));
        }
        OverlaidGalaxy::new(
            systems
                .iter()
                .map(|&(id, place)| star(id, place, &[]))
                .collect(),
            overlay,
        )
    }

    fn systems(ids: &[i16]) -> Vec<SystemId> {
        ids.iter().copied().map(SystemId).collect()
    }

    #[test]
    fn resume_system_takes_the_owner_first() {
        let both = galaxy_of(&[(128, (0.0, 0.0)), (200, (100.0, 0.0))], &[]);
        assert_eq!(
            resume_system(&both, &systems(&[128, 200])),
            Some(SystemId(128))
        );
        let hidden_128 = galaxy_of(&[(128, (0.0, 0.0)), (129, (50.0, 0.0))], &[128]);
        assert_eq!(
            resume_system(&hidden_128, &systems(&[128, 129])),
            Some(SystemId(129)),
            "the owner over a 128 that doesn't own it"
        );
        let shadowed_128 = galaxy_of(
            &[(127, (0.0, 0.0)), (128, (0.0, 0.0)), (129, (50.0, 0.0))],
            &[],
        );
        assert_eq!(
            resume_system(&shadowed_128, &systems(&[128, 129])),
            Some(SystemId(129))
        );
    }

    #[test]
    fn resume_system_takes_128_for_an_unowned_stellar_hidden_or_shadowed() {
        // 200 is visible but shadowed by 199 at its place: no owner.
        let hidden_128 = galaxy_of(
            &[(128, (0.0, 0.0)), (199, (100.0, 0.0)), (200, (100.0, 0.0))],
            &[128],
        );
        assert_eq!(
            resume_system(&hidden_128, &systems(&[128, 200])),
            Some(SystemId(128))
        );
        let shadowed_128 = galaxy_of(&[(127, (0.0, 0.0)), (128, (0.0, 0.0))], &[]);
        assert_eq!(
            resume_system(&shadowed_128, &systems(&[128])),
            Some(SystemId(128))
        );
    }

    #[test]
    fn resume_system_takes_the_lowest_visible_lister_next() {
        let galaxy = galaxy_of(
            &[(150, (0.0, 0.0)), (200, (100.0, 0.0)), (201, (100.0, 0.0))],
            &[150],
        );
        assert_eq!(
            resume_system(&galaxy, &systems(&[150, 201])),
            Some(SystemId(201)),
            "the shadowed 201 over the lower hidden 150"
        );
        let gone = galaxy_of(&[(200, (100.0, 0.0)), (201, (100.0, 0.0))], &[]);
        assert_eq!(
            resume_system(&gone, &systems(&[150, 201])),
            Some(SystemId(201)),
            "a lister not in the galaxy is not visible"
        );
    }

    #[test]
    fn resume_system_takes_the_lowest_lister_of_any_kind_last() {
        let galaxy = galaxy_of(&[(150, (0.0, 0.0)), (160, (100.0, 0.0))], &[150, 160]);
        assert_eq!(
            resume_system(&galaxy, &systems(&[150, 160])),
            Some(SystemId(150))
        );
    }

    #[test]
    fn resume_system_has_none_without_listers() {
        let galaxy = galaxy_of(&[(128, (0.0, 0.0))], &[]);
        assert_eq!(resume_system(&galaxy, &[]), None);
    }

    #[test]
    fn facing_on_load_turns_a_docked_pilot_to_the_drawn_heading() {
        let catalog = catalog();
        let mut session = Session::fly(&catalog, landed_pilot(&catalog)).expect("flies");
        let mut chance = Scripted::rolling(&[359]);
        session.face_on_load(&mut chance);
        assert_eq!(session.player().heading, 359.0);
        assert_eq!(chance.sides_asked, [360], "a whole degree, 0 to 359");
        assert_eq!(session.landed(), Some(StellarId(128)), "still docked");
    }

    #[test]
    fn facing_on_load_turns_an_undocked_pilot_too() {
        let catalog = catalog();
        for draw in [0, 123] {
            let mut session = Session::start(&catalog).expect("starts");
            let mut chance = Scripted::rolling(&[draw]);
            session.face_on_load(&mut chance);
            assert_eq!(session.player().heading, f32::from(draw));
            assert_eq!(chance.sides_asked, [360]);
        }
    }

    #[test]
    fn flying_alone_leaves_the_heading_at_0() {
        let catalog = catalog();
        let session = Session::fly(&catalog, landed_pilot(&catalog)).expect("flies");
        assert_eq!(session.player().heading, 0.0);
    }

    #[test]
    fn flying_a_pilot_explores_its_system() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        let mut pilot = session.pilot().clone();
        pilot.explored.clear();
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().explored().collect::<Vec<_>>(),
            [SystemId(131)]
        );
    }

    // Sounds.

    #[test]
    fn holding_thrust_starts_it_once_and_letting_go_stops_it_once() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(Controls::default());
        assert_eq!(session.take_sounds(), []);
        for _ in 0..5 {
            session.tick(THRUST);
        }
        assert_eq!(session.take_sounds(), [SimSound::ThrustStarted]);
        for _ in 0..5 {
            session.tick(Controls {
                turn: Turn::Left,
                ..Controls::default()
            });
        }
        assert_eq!(session.take_sounds(), [SimSound::ThrustStopped]);
        session.tick(THRUST);
        session.tick(Controls::default());
        session.tick(THRUST);
        assert_eq!(
            session.take_sounds(),
            [
                SimSound::ThrustStarted,
                SimSound::ThrustStopped,
                SimSound::ThrustStarted
            ]
        );
    }

    #[test]
    fn the_session_is_thrusting_while_the_last_tick_flew_thrust() {
        let mut session = Session::start(&sounding()).expect("starts");
        assert!(!session.thrusting(), "at rest");
        session.tick(THRUST);
        assert!(session.thrusting());
        session.tick(Controls::default());
        assert!(!session.thrusting(), "coasting");
        session.tick(THRUST);
        session.land().expect("selects");
        assert!(matches!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Landed(_)))
        ));
        assert!(!session.thrusting(), "landed");
        session.tick(THRUST);
        assert!(!session.thrusting(), "docked, nothing thrusts");
    }

    #[test]
    fn beginning_a_jump_stops_the_session_thrusting() {
        // At rest facing east, the bearing to 131: the jump begins on J.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 90.0);
        session.tick(THRUST);
        assert!(session.thrusting());
        session.begin_jump().expect("jumps");
        assert_eq!(session.jumping(), Some(SystemId(131)));
        assert!(!session.thrusting());
        session.tick(THRUST);
        assert!(!session.thrusting(), "jumping, nothing thrusts");
    }

    // The engine glow.

    #[test]
    fn the_engine_glow_ramps_up_with_thrust_and_down_after_it() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.engine_glow(), 0, "a new session");
        for k in 1..=30 {
            session.tick(THRUST);
            assert_eq!(session.engine_glow(), k.min(GLOW_CRUISE), "{k} ticks");
        }
        for k in 1..=30 {
            session.tick(Controls::default());
            assert_eq!(
                session.engine_glow(),
                GLOW_CRUISE.saturating_sub(k),
                "{k} ticks coasting"
            );
        }
    }

    /// A slow ship, at most half a pixel a tick, over a landable planet at
    /// the centre: it can thrust and still land.
    fn slow_lander() -> FakePilotCatalog {
        FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { speed: 50, ..FAST }))],
            sites: vec![(SystemId(130), vec![planet(128, 0.0, 0.0)])],
            ..catalog()
        }
    }

    #[test]
    fn landing_puts_the_engine_glow_out_even_while_coasting() {
        let mut session = Session::start(&slow_lander()).expect("starts");
        for _ in 0..10 {
            session.tick(THRUST);
        }
        for _ in 0..2 {
            session.tick(Controls::default());
        }
        assert!(!session.thrusting());
        assert_eq!(session.engine_glow(), 8, "still glowing as it coasts");
        land_now(&mut session).expect("lands");
        assert_eq!(session.engine_glow(), 0, "landed");
        session.tick(THRUST);
        assert_eq!(session.engine_glow(), 0, "docked, nothing glows");
    }

    #[test]
    fn beginning_a_jump_puts_the_engine_glow_out() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        for _ in 0..5 {
            session.tick(THRUST);
        }
        session.tick(Controls::default());
        assert!(!session.thrusting());
        assert!(session.engine_glow() > 0, "glowing as it coasts");
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(session.engine_glow(), 0, "jumping");
        session.tick(THRUST);
        assert_eq!(session.engine_glow(), 0, "jumping, nothing glows");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.engine_glow(), 0, "arrived");
    }

    #[test]
    fn a_refused_jump_leaves_the_engine_glow_as_it_was() {
        let mut session = Session::start(&catalog()).expect("starts");
        for _ in 0..5 {
            session.tick(THRUST);
        }
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        assert_eq!(session.engine_glow(), 5);
    }

    #[test]
    fn taking_the_sounds_empties_them() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(THRUST);
        assert_eq!(session.take_sounds(), [SimSound::ThrustStarted]);
        assert_eq!(session.take_sounds(), []);
    }

    /// The catalog with planet 128 playing `snd ` 10032 when landed on.
    fn sounding() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![(
                SystemId(130),
                vec![LandingSite {
                    landing_sound: Some(SoundId(10_032)),
                    ..planet(128, 30.0, -40.0)
                }],
            )],
            ..catalog()
        }
    }

    #[test]
    fn landing_emits_landed_with_the_stellars_sound() {
        let mut session = Session::start(&sounding()).expect("starts");
        land_now(&mut session).expect("lands");
        assert_eq!(
            session.take_sounds(),
            [SimSound::Landed {
                stellar_sound: Some(SoundId(10_032))
            }]
        );
        let mut silent = Session::start(&catalog()).expect("starts");
        land_now(&mut silent).expect("lands");
        assert_eq!(
            silent.take_sounds(),
            [SimSound::Landed {
                stellar_sound: None
            }]
        );
    }

    #[test]
    fn landing_while_thrusting_stops_the_thrust_first() {
        let mut session = Session::start(&sounding()).expect("starts");
        session.tick(Controls {
            turn: Turn::Right,
            ..THRUST
        });
        assert_eq!(session.take_sounds(), [SimSound::ThrustStarted]);
        land_now(&mut session).expect("lands");
        assert_eq!(
            session.take_sounds(),
            [
                SimSound::ThrustStopped,
                SimSound::Landed {
                    stellar_sound: Some(SoundId(10_032))
                }
            ]
        );
        session.tick(THRUST);
        assert_eq!(session.take_sounds(), [], "docked, nothing thrusts");
        session.take_off(&NoTraffic, &mut NeverFires);
        assert_eq!(session.take_sounds(), [SimSound::TookOff]);
        session.tick(THRUST);
        assert_eq!(
            session.take_sounds(),
            [SimSound::ThrustStarted],
            "off again"
        );
    }

    #[test]
    fn taking_off_emits_took_off_and_not_taking_off_emits_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.take_off(&NoTraffic, &mut NeverFires);
        assert_eq!(session.take_sounds(), [], "not landed");
        land_now(&mut session).expect("lands");
        session.take_sounds();
        session.take_off(&NoTraffic, &mut NeverFires);
        assert_eq!(session.take_sounds(), [SimSound::TookOff]);
        session.take_off(&NoTraffic, &mut NeverFires);
        assert_eq!(session.take_sounds(), [], "nor twice");
    }

    #[test]
    fn a_refused_landing_emits_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        for _ in 0..20 {
            session.tick(THRUST);
        }
        session.take_sounds();
        land_now(&mut session).expect_err("refused");
        assert_eq!(session.take_sounds(), []);
        let mut jumping = Session::start(&edge_lander()).expect("starts");
        jumping.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut jumping);
        jumping.begin_jump().expect("jumps");
        jumping.take_sounds();
        assert_eq!(jumping.land(), Err(LandingRefusal::Jumping));
        assert_eq!(jumping.take_sounds(), []);
    }

    #[test]
    fn a_jump_emits_jump_began_then_arrived_stopping_the_thrust_first() {
        let catalog = catalog();
        // At rest facing east, the bearing to 131: the jump begins on J.
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 90.0);
        session.tick(THRUST);
        let thrusting = session.take_sounds();
        assert_eq!(thrusting.last(), Some(&SimSound::ThrustStarted));
        session.begin_jump().expect("jumps");
        assert_eq!(
            session.take_sounds(),
            [SimSound::ThrustStopped, SimSound::JumpBegan]
        );
        session.tick(THRUST);
        assert_eq!(session.take_sounds(), [], "jumping, nothing thrusts");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.take_sounds(), [SimSound::Arrived]);
        session.arrive(&catalog, &mut NeverFires);
        assert_eq!(session.take_sounds(), [], "no jump under way");
    }

    #[test]
    fn a_jump_without_thrust_emits_only_jump_began() {
        // Turning at rest to the bearing, it never thrusts.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 0.0);
        session.tick(Controls::default());
        session.take_sounds();
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(session.take_sounds(), [SimSound::JumpBegan]);
    }

    #[test]
    fn a_refused_jump_emits_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(THRUST);
        session.take_sounds();
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        assert_eq!(session.take_sounds(), []);
        let catalog = edge_lander();
        let mut landed = landed_at_the_edge(&catalog);
        landed.take_sounds();
        assert_eq!(landed.begin_jump(), Err(JumpRefusal::Landed));
        assert_eq!(landed.take_sounds(), []);
    }

    // The exchange.

    /// Food at 75 and metal at 200, as `STR#` 4000 and 4004 have them.
    fn food_and_metal() -> CommodityStrings {
        CommodityStrings {
            names: ["Food", "Industrial", "Medical", "Luxury", "Metal"]
                .map(str::to_owned)
                .to_vec(),
            name_patches: Default::default(),
            base_prices: ["75", "350", "750", "900", "200"]
                .map(str::to_owned)
                .to_vec(),
            price_patches: Default::default(),
        }
    }

    /// Food at medium and metal at low.
    const TRADES: u32 = StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | 2 << 28 | 1 << 12;

    /// The catalog with planet 128 at the centre, which the ship starts
    /// over, a trade center trading food (75) and metal (160), and the
    /// first `chär` holding 1000 credits; ship 128 holds 20 tons.
    fn exchange() -> FakePilotCatalog {
        FakePilotCatalog {
            character: Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [Some(SystemId(130)), None, None, None],
                start: START,
                cash: 1000,
                ..CharacterStart::default()
            }),
            sites: vec![(
                SystemId(130),
                vec![LandingSite {
                    flags: TRADES,
                    ..planet(128, 0.0, 0.0)
                }],
            )],
            commodities: food_and_metal(),
            ..catalog()
        }
    }

    /// An order for `lot` of `good`, `direction`, on its first row in
    /// `session`'s exchange, or on no row without one.
    fn order(session: &Session, good: Good, direction: Direction, lot: Lot) -> Order {
        let row = session
            .market()
            .and_then(|market| market.rows.iter().position(|row| row.good == good))
            .unwrap_or(usize::MAX);
        Order {
            row,
            good,
            direction,
            lot,
        }
    }

    const FOOD: Good = Good::Commodity(0);
    const METAL: Good = Good::Commodity(4);

    #[test]
    fn a_session_reads_the_goods_once_when_it_starts() {
        let catalog = exchange();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(*catalog.goods_reads.borrow(), 4);
        land_now(&mut session).expect("lands");
        assert!(session.market().is_some());
        session.take_off(&catalog, &mut NeverFires);
        jump(&mut session, &catalog, 131);
        assert_eq!(*catalog.goods_reads.borrow(), 4);
    }

    #[test]
    fn the_exchange_names_a_commodity_from_its_str_patch() {
        let mut catalog = exchange();
        catalog.commodities.name_patches[0] = crate::StringPatch::Text("Grain".into());
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        assert_eq!(session.good_name(FOOD), Some("Grain"));
        assert_eq!(session.good_name(METAL), Some("Metal"));
        let market = session.market().expect("an exchange");
        let food: Vec<_> = market
            .rows
            .iter()
            .filter(|r| r.good == FOOD)
            .map(|r| r.price)
            .collect();
        assert_eq!(food, [75]);
    }

    #[test]
    fn the_cargo_space_is_the_ships_holds_and_its_cargo_pods() {
        let session = Session::start(&exchange()).expect("starts");
        assert_eq!(session.capacity(), 20);
        let catalog = FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { holds: -3, ..FAST }))],
            defaults: vec![(ShipId(128), vec![(OutfitId(200), 2)])],
            outfits: vec![outfit(200, &[(crate::market::MORE_CARGO, 5)])],
            ..exchange()
        };
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.capacity(), 13);
    }

    #[test]
    fn there_is_an_exchange_only_while_landed_at_a_trade_center() {
        let catalog = exchange();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.market(), None, "in flight");
        land_now(&mut session).expect("lands");
        let market = session.market().expect("an exchange");
        let prices: Vec<_> = market.rows.iter().map(|r| (r.good, r.price)).collect();
        assert_eq!(prices, [(FOOD, 75), (METAL, 160)]);
        assert_eq!((market.cash, market.capacity, market.free), (1000, 20, 20));
        session.take_off(&catalog, &mut NeverFires);
        assert_eq!(session.market(), None, "taken off");
        let mut plain = Session::start(&self::catalog()).expect("starts");
        land_now(&mut plain).expect("lands");
        assert_eq!(plain.market(), None, "no trade center");
    }

    /// The exchange's catalog with food at high and metal at low, at a
    /// trade center of government `site_govt` with `flags2`, in a system
    /// of government `system_govt`.
    fn marked_up(
        site_govt: Option<i16>,
        system_govt: Option<i16>,
        flags2: u16,
    ) -> FakePilotCatalog {
        let mut catalog = exchange();
        catalog.sites[0].1[0] = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | 4 << 28 | 1 << 12,
            govt: site_govt.map(GovtId),
            flags2,
            ..planet(128, 0.0, 0.0)
        };
        catalog.star_map[0].govt = system_govt.map(GovtId);
        catalog
    }

    /// The food and metal prices at `catalog`'s trade center with these
    /// legal records, set once landed.
    fn marked_up_prices(catalog: &FakePilotCatalog, records: &[(i16, i16)]) -> [i64; 2] {
        let mut session = Session::start(catalog).expect("starts");
        land_now(&mut session).expect("lands");
        for &(govt, record) in records {
            with_record(&mut session, GovtId(govt), record);
        }
        let market = session.market().expect("an exchange");
        [FOOD, METAL].map(|good| market.row(good).expect("traded").price)
    }

    #[test]
    fn by_the_engine_the_markup_reads_the_systems_record() {
        let catalog = marked_up(Some(150), Some(150), 0);
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session.pilot.set_legal_record(GovtId(150), 5);
        session.pilot.set_system_record(SystemId(130), -1);
        let prices = |session: &Session| {
            let market = session.market().expect("an exchange");
            [FOOD, METAL].map(|good| market.row(good).expect("traded").price)
        };
        assert_eq!(prices(&session), [82, 181], "1.1, by the system's -1");
        let session = session.with_rules(bible_records());
        assert_eq!(prices(&session), [93, 160], "1.25, by the Bible 150's 5");
    }

    #[test]
    fn a_negative_record_in_a_governed_stellars_system_marks_its_prices_up_by_1_1() {
        let catalog = marked_up(Some(150), Some(150), 0);
        assert_eq!(marked_up_prices(&catalog, &[]), [93, 160], "1.25");
        assert_eq!(marked_up_prices(&catalog, &[(150, 0)]), [93, 160], "1.25");
        assert_eq!(marked_up_prices(&catalog, &[(150, -1)]), [82, 181], "1.1");
        let elsewhere = marked_up(Some(150), Some(151), 0);
        assert_eq!(
            marked_up_prices(&elsewhere, &[(150, -1)]),
            [93, 160],
            "the record with the system's government counts"
        );
        assert_eq!(
            marked_up_prices(&elsewhere, &[(150, 5), (151, -1)]),
            [82, 181]
        );
    }

    #[test]
    fn a_stellar_without_a_government_or_in_an_independent_system_keeps_1_25() {
        let ungoverned = marked_up(None, Some(150), 0);
        assert_eq!(marked_up_prices(&ungoverned, &[(150, -1)]), [93, 160]);
        let independent = marked_up(Some(150), None, 0);
        assert_eq!(marked_up_prices(&independent, &[(150, -1)]), [93, 160]);
    }

    #[test]
    fn a_dominated_stellar_marks_its_prices_up_by_1_5_whatever_the_record() {
        use crate::market::DOMINATED;
        assert_eq!(DOMINATED, 0x0020);
        for (govt, records) in [(Some(150), &[(150, -1)][..]), (Some(150), &[]), (None, &[])] {
            let catalog = marked_up(govt, Some(150), DOMINATED);
            assert_eq!(marked_up_prices(&catalog, records), [112, 133], "{govt:?}");
        }
        // Every other bit but the gates', which are not landed at.
        let others = !(DOMINATED | crate::gate::HYPERGATE | crate::gate::WORMHOLE);
        let undominated = marked_up(Some(150), Some(150), others);
        assert_eq!(
            marked_up_prices(&undominated, &[(150, -1)]),
            [82, 181],
            "only the dominated bit"
        );
    }

    /// An escort of ship class `ship`, out of no bay.
    fn fleet_escort(ship: i16) -> Escort {
        Escort {
            ship: ShipId(ship),
            reserves: Reserves::default(),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }
    }

    /// Ship class `id`'s record, of `Holds` `holds` and `InherentAI`
    /// `inherent_ai`.
    fn holds_record(id: i16, holds: i16, inherent_ai: i16) -> ShipRecord {
        ShipRecord {
            inherent_ai,
            ..ship(id, ShipFields { holds, ..FAST })
        }
    }

    /// [`exchange`] with ship classes 129 a trader of 15 tons, 130 a
    /// warship of 50 and 131 a trader of 5; 132 has no record.
    fn fleet_exchange() -> FakePilotCatalog {
        FakePilotCatalog {
            ship_records: vec![
                holds_record(129, 15, 1),
                holds_record(130, 50, 3),
                holds_record(131, 5, 1),
            ],
            ..exchange()
        }
    }

    /// A session of [`fleet_exchange`], landed, whose fleet is a trader
    /// escort (129), a warship (130), a launched fighter (131) and an
    /// escort with no record (132).
    fn landed_with_a_fleet() -> Session {
        let mut session = Session::start(&fleet_exchange()).expect("starts");
        session.pilot.escorts = vec![
            fleet_escort(129),
            fleet_escort(130),
            Escort {
                carried: true,
                ..fleet_escort(131)
            },
            fleet_escort(132),
        ];
        land_now(&mut session).expect("lands");
        session
    }

    #[test]
    fn the_exchange_counts_trader_escorts_holds() {
        let session = landed_with_a_fleet();
        assert_eq!(session.capacity(), 35);
        let market = session.market().expect("an exchange");
        assert_eq!((market.capacity, market.free), (35, 35));
    }

    #[test]
    fn the_exchange_counts_a_trader_escorts_negative_holds_by_its_size() {
        let catalog = FakePilotCatalog {
            ship_records: vec![holds_record(129, -5, 1)],
            ..exchange()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.escorts = vec![fleet_escort(129)];
        land_now(&mut session).expect("lands");
        assert_eq!(session.capacity(), 25);
        let market = session.market().expect("an exchange");
        assert_eq!((market.capacity, market.free), (25, 25));
    }

    #[test]
    fn the_exchanges_free_space_is_the_fleets_less_everything_held() {
        let mut session = landed_with_a_fleet();
        session.pilot.set_cash(100_000);
        assert_eq!(
            session.trade(order(&session, FOOD, Direction::Buy, Lot::Max)),
            Ok(35)
        );
        assert_eq!(session.market().expect("an exchange").free, 0);
        assert_eq!(
            session.trade(order(&session, FOOD, Direction::Buy, Lot::Click)),
            Err(TradeRefusal::NoSpace)
        );
    }

    #[test]
    fn the_exchanges_cargo_space_stops_at_32000() {
        let catalog = FakePilotCatalog {
            ship_records: vec![holds_record(129, 20_000, 2)],
            ..exchange()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.escorts = vec![fleet_escort(129), fleet_escort(129)];
        land_now(&mut session).expect("lands");
        assert_eq!(session.market().expect("an exchange").capacity, 32_000);
    }

    #[test]
    fn buying_pays_loads_and_makes_a_save_due() {
        let mut session = Session::start(&exchange()).expect("starts");
        land_now(&mut session).expect("lands");
        session.take_save_due();
        assert_eq!(
            session.trade(order(&session, FOOD, Direction::Buy, Lot::Click)),
            Ok(10),
            "a click buys 10 tons"
        );
        assert_eq!(session.pilot().cash(), 1000 - 10 * 75);
        assert_eq!(session.pilot().held(FOOD), 10);
        assert!(session.take_save_due());
        assert_eq!(
            session.trade(order(&session, METAL, Direction::Buy, Lot::Max)),
            Ok(1)
        );
        assert_eq!(session.pilot().cash(), 90, "cash ran out first");
        assert_eq!(session.market().expect("an exchange").free, 9);
    }

    #[test]
    fn buying_stops_when_the_hold_is_full() {
        let catalog = FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { holds: 3, ..FAST }))],
            ..exchange()
        };
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        assert_eq!(
            session.trade(order(&session, FOOD, Direction::Buy, Lot::Max)),
            Ok(3)
        );
        session.take_save_due();
        assert_eq!(
            session.trade(order(&session, FOOD, Direction::Buy, Lot::Click)),
            Err(TradeRefusal::NoSpace)
        );
        assert_eq!(session.pilot().cash(), 775);
        assert!(!session.take_save_due(), "nothing changed");
    }

    #[test]
    fn selling_pays_the_local_price() {
        let mut session = Session::start(&exchange())
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::TradeLot, RuleSource::Bible));
        land_now(&mut session).expect("lands");
        assert_eq!(
            session.trade(order(&session, METAL, Direction::Buy, Lot::Max)),
            Ok(6)
        );
        assert_eq!(session.pilot().cash(), 40);
        session.take_save_due();
        assert_eq!(
            session.trade(order(&session, METAL, Direction::Sell, Lot::Click)),
            Ok(1)
        );
        assert_eq!(session.pilot().cash(), 40 + 160);
        assert!(session.take_save_due());
        assert_eq!(
            session.trade(order(&session, METAL, Direction::Sell, Lot::Max)),
            Ok(5)
        );
        assert_eq!(session.pilot().cash(), 1000);
        assert_eq!(session.pilot().held(METAL), 0);
    }

    #[test]
    fn a_refused_trade_changes_nothing_and_makes_no_save_due() {
        let mut session = Session::start(&exchange()).expect("starts");
        let flying = session.clone();
        assert_eq!(
            session.trade(order(&session, FOOD, Direction::Buy, Lot::Click)),
            Err(TradeRefusal::NoMarket)
        );
        assert_eq!(session, flying);
        land_now(&mut session).expect("lands");
        session.take_save_due();
        let landed = session.clone();
        for refused in [
            order(&session, FOOD, Direction::Sell, Lot::Click),
            order(&session, Good::Commodity(1), Direction::Buy, Lot::Click),
        ] {
            assert!(session.trade(refused).is_err(), "{refused:?}");
        }
        assert_eq!(session, landed);
        assert!(!session.take_save_due());
    }

    #[test]
    fn junk_is_bought_and_sold_like_a_commodity() {
        let opals = JunkRecord {
            id: JunkId(146),
            name: "Opals".to_owned(),
            abbrev: String::new(),
            base_price: 100,
            sold_at: vec![StellarId(128)],
            bought_at: vec![StellarId(140)],
            buy_on: Test::default(),
            sell_on: Test::default(),
            flags: 0,
            scan_mask: 0,
            lc_name: String::new(),
        };
        let catalog = edge_lander();
        let catalog = FakePilotCatalog {
            character: exchange().character,
            sites: vec![
                (
                    SystemId(130),
                    vec![LandingSite {
                        flags: TRADES,
                        ..planet(128, 0.0, 0.0)
                    }],
                ),
                (
                    SystemId(131),
                    vec![LandingSite {
                        flags: TRADES,
                        ..planet(140, -1000.0, 0.0)
                    }],
                ),
            ],
            junk: vec![opals],
            ..catalog
        };
        let opals = Good::Junk(JunkId(146));
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::JunkTrade, RuleSource::Bible));
        land_now(&mut session).expect("lands");
        assert_eq!(
            session.trade(order(&session, opals, Direction::Buy, Lot::Max)),
            Ok(12)
        );
        assert_eq!(session.pilot().cash(), 1000 - 12 * 80);
        assert_eq!(
            session.trade(order(&session, opals, Direction::Sell, Lot::Click)),
            Err(TradeRefusal::NotTraded),
            "by the Bible, not bought here"
        );
        session.take_off(&catalog, &mut NeverFires);
        jump(&mut session, &catalog, 131);
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        assert_eq!(
            session.trade(order(&session, opals, Direction::Sell, Lot::Max)),
            Ok(12)
        );
        assert_eq!(session.pilot().cash(), 1000 - 12 * 80 + 12 * 125);
    }

    #[test]
    fn junk_listed_both_ways_is_bought_low_and_sold_high() {
        let both = JunkRecord {
            id: JunkId(146),
            name: "Opals".to_owned(),
            abbrev: String::new(),
            base_price: 100,
            sold_at: vec![StellarId(128)],
            bought_at: vec![StellarId(128)],
            buy_on: Test::default(),
            sell_on: Test::default(),
            flags: 0,
            scan_mask: 0,
            lc_name: String::new(),
        };
        let catalog = FakePilotCatalog {
            junk: vec![both],
            ..exchange()
        };
        let opals = Good::Junk(JunkId(146));
        let mut session = Session::start(&catalog).expect("starts").with_rules(
            Rulebook::default()
                .with_override(RuleKey::JunkTrade, RuleSource::Bible)
                .with_override(RuleKey::TradeLot, RuleSource::Bible),
        );
        land_now(&mut session).expect("lands");
        let (high, low) = (2, 3);
        let on = |row, direction| Order {
            row,
            good: opals,
            direction,
            lot: Lot::Click,
        };
        assert_eq!(session.trade(on(low, Direction::Buy)), Ok(1));
        assert_eq!(session.pilot().cash(), 1000 - 80, "100 / 1.25");
        assert_eq!(session.trade(on(high, Direction::Sell)), Ok(1));
        assert_eq!(session.pilot().cash(), 1000 - 80 + 125, "100 × 1.25");
        assert_eq!(
            session.trade(on(high, Direction::Buy)),
            Err(TradeRefusal::NotTraded)
        );
        assert_eq!(
            session.trade(on(low, Direction::Sell)),
            Err(TradeRefusal::NotTraded)
        );
    }

    #[test]
    fn by_the_engine_junk_listed_both_ways_is_bought_and_sold_on_each_row_at_its_price() {
        let both = JunkRecord {
            id: JunkId(146),
            name: "Opals".to_owned(),
            abbrev: String::new(),
            base_price: 100,
            sold_at: vec![StellarId(128)],
            bought_at: vec![StellarId(128)],
            buy_on: Test::default(),
            sell_on: Test::default(),
            flags: 0,
            scan_mask: 0,
            lc_name: String::new(),
        };
        let catalog = FakePilotCatalog {
            junk: vec![both],
            ..exchange()
        };
        let opals = Good::Junk(JunkId(146));
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::TradeLot, RuleSource::Bible));
        land_now(&mut session).expect("lands");
        let prices: Vec<_> = session
            .market()
            .expect("an exchange")
            .rows
            .iter()
            .map(|row| (row.good, row.price))
            .collect();
        assert_eq!(prices[2..], [(opals, 125), (opals, 80)]);
        let (high, low) = (2, 3);
        let on = |row, direction| Order {
            row,
            good: opals,
            direction,
            lot: Lot::Click,
        };
        assert_eq!(session.trade(on(low, Direction::Buy)), Ok(1));
        assert_eq!(session.pilot().cash(), 1000 - 80, "the low row's price");
        assert_eq!(session.trade(on(high, Direction::Buy)), Ok(1));
        assert_eq!(session.pilot().cash(), 1000 - 80 - 125, "the high row's");
        assert_eq!(session.pilot().held(opals), 2, "one held count");
        assert_eq!(session.trade(on(low, Direction::Sell)), Ok(1));
        assert_eq!(session.pilot().cash(), 1000 - 125, "sold at the low price");
        assert_eq!(session.trade(on(high, Direction::Sell)), Ok(1));
        assert_eq!(session.pilot().cash(), 1000, "and at the high");
        assert_eq!(session.pilot().held(opals), 0);
    }

    #[test]
    fn the_session_trades_junk_both_ways_as_its_rule_says() {
        let opals = JunkRecord {
            id: JunkId(146),
            name: "Opals".to_owned(),
            abbrev: String::new(),
            base_price: 100,
            sold_at: vec![StellarId(128)],
            bought_at: Vec::new(),
            buy_on: Test::default(),
            sell_on: Test::default(),
            flags: 0,
            scan_mask: 0,
            lc_name: String::new(),
        };
        let catalog = FakePilotCatalog {
            junk: vec![opals],
            ..exchange()
        };
        let opals = Good::Junk(JunkId(146));
        for (source, sold) in [
            (None, Ok(1)),
            (Some(RuleSource::Engine), Ok(1)),
            (Some(RuleSource::Bible), Err(TradeRefusal::NotTraded)),
        ] {
            let rules = Rulebook::default().with_override(RuleKey::TradeLot, RuleSource::Bible);
            let rules = match source {
                Some(source) => rules.with_override(RuleKey::JunkTrade, source),
                None => rules,
            };
            let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
            assert_eq!(
                session.rules().source_for(RuleKey::JunkTrade),
                source.unwrap_or_default()
            );
            land_now(&mut session).expect("lands");
            assert!(session.transact(|pilot| {
                pilot.cargo.insert(opals, 3);
            }));
            let sale = order(&session, opals, Direction::Sell, Lot::Click);
            assert_eq!(session.trade(sale), sold, "{source:?}");
            let paid = if sold.is_ok() { 80 } else { 0 };
            assert_eq!(session.pilot().cash(), 1000 + paid, "{source:?}");
        }
    }

    /// [`exchange`] with 100 credits, its planet 128 buying `jünk` 146
    /// at `base`, the player holding 3 tons of it.
    fn junk_session(base: i16, source: Option<RuleSource>) -> Session {
        let mut catalog = FakePilotCatalog {
            junk: vec![JunkRecord {
                id: JunkId(146),
                name: "Waste".to_owned(),
                abbrev: String::new(),
                base_price: base,
                sold_at: Vec::new(),
                bought_at: vec![StellarId(128)],
                buy_on: Test::default(),
                sell_on: Test::default(),
                flags: 0,
                scan_mask: 0,
                lc_name: String::new(),
            }],
            ..exchange()
        };
        if let Ok(start) = &mut catalog.character {
            start.cash = 100;
        }
        let session = Session::start(&catalog).expect("starts");
        let mut session = match source {
            Some(source) => {
                session.with_rules(Rulebook::default().with_override(RuleKey::JunkPrice, source))
            }
            None => session,
        };
        land_now(&mut session).expect("lands");
        assert!(session.transact(|pilot| {
            pilot.cargo.insert(Good::Junk(JunkId(146)), 3);
        }));
        session
    }

    #[test]
    fn the_session_trades_junk_of_negative_price_as_its_rule_says() {
        let waste = Good::Junk(JunkId(146));
        let mut engine = junk_session(-100, None);
        assert_eq!(
            engine.rules().source_for(RuleKey::JunkPrice),
            RuleSource::Engine,
            "the default"
        );
        assert_eq!(
            engine
                .market()
                .and_then(|market| market.row(waste).map(|row| row.price)),
            Some(-125)
        );
        assert_eq!(
            engine.trade(order(&engine, waste, Direction::Sell, Lot::Max)),
            Ok(3)
        );
        assert_eq!(engine.pilot().cash(), 100 - 3 * 125, "below 0");
        assert_eq!(engine.pilot().held(waste), 0);
        let mut floor = junk_session(-100, Some(RuleSource::Bible));
        assert_eq!(
            floor.rules().source_for(RuleKey::JunkPrice),
            RuleSource::Bible
        );
        assert_eq!(
            floor.trade(order(&floor, waste, Direction::Sell, Lot::Max)),
            Ok(3)
        );
        assert_eq!(floor.pilot().cash(), 100, "sold for nothing");
        let engine = junk_session(0, Some(RuleSource::Engine));
        assert_eq!(
            engine.rules().source_for(RuleKey::JunkPrice),
            RuleSource::Engine
        );
        assert_eq!(
            engine.market().map(|market| market.rows.len()),
            Some(2),
            "food and metal, and no row priced 0"
        );
    }

    #[test]
    fn by_the_engine_a_buy_on_a_negative_price_is_offered_and_changes_nothing() {
        let waste = Good::Junk(JunkId(146));
        let mut session = junk_session(-100, None);
        session.take_save_due();
        let market = session.market().expect("an exchange");
        let row = market
            .rows
            .iter()
            .position(|row| row.good == waste)
            .expect("listed");
        assert!(market.row_allows(row, Direction::Buy), "100 >= -125");
        let before = session.clone();
        for lot in [Lot::Click, Lot::Max] {
            assert_eq!(
                session.trade(order(&session, waste, Direction::Buy, lot)),
                Err(TradeRefusal::CannotAfford),
                "{lot:?}"
            );
        }
        assert_eq!(session, before);
        assert_eq!(session.pilot().cash(), 100);
        assert_eq!(session.pilot().held(waste), 3);
        assert!(!session.take_save_due());
    }

    #[test]
    fn the_session_trades_a_click_as_its_rule_says() {
        for (source, tons) in [
            (None, 10),
            (Some(RuleSource::Engine), 10),
            (Some(RuleSource::Bible), 1),
        ] {
            let session = Session::start(&exchange()).expect("starts");
            let mut session = match source {
                Some(source) => {
                    session.with_rules(Rulebook::default().with_override(RuleKey::TradeLot, source))
                }
                None => session,
            };
            let source = source.unwrap_or_default();
            assert_eq!(session.rules().source_for(RuleKey::TradeLot), source);
            land_now(&mut session).expect("lands");
            assert_eq!(
                session
                    .market()
                    .map(|market| market.rules.source_for(RuleKey::TradeLot)),
                Some(source)
            );
            session.take_save_due();
            assert_eq!(
                session.trade(order(&session, FOOD, Direction::Buy, Lot::Click)),
                Ok(tons),
                "{source:?}"
            );
            assert_eq!(session.pilot().cash(), 1000 - 75 * i64::from(tons));
            assert_eq!(session.pilot().held(FOOD), tons);
            assert!(session.take_save_due());
            assert_eq!(
                session.trade(order(&session, FOOD, Direction::Sell, Lot::Click)),
                Ok(tons),
                "{source:?}"
            );
            assert_eq!(session.pilot().cash(), 1000);
        }
    }

    #[test]
    fn the_session_trades_a_counted_lot_at_the_rows_price() {
        let mut session = Session::start(&exchange()).expect("starts");
        land_now(&mut session).expect("lands");
        session.take_save_due();
        assert_eq!(
            session.trade(order(&session, FOOD, Direction::Buy, Lot::Count(7))),
            Ok(7)
        );
        assert_eq!(session.pilot().cash(), 1000 - 7 * 75);
        assert_eq!(session.pilot().held(FOOD), 7);
        assert!(session.take_save_due());
        assert_eq!(
            session.trade(order(&session, FOOD, Direction::Sell, Lot::Count(3))),
            Ok(3)
        );
        assert_eq!(session.pilot().cash(), 1000 - 4 * 75);
        assert_eq!(session.pilot().held(FOOD), 4);
        let before = session.clone();
        assert_eq!(
            session.trade(order(&session, FOOD, Direction::Sell, Lot::Count(5))),
            Err(TradeRefusal::OutOfRange)
        );
        assert_eq!(session, before);
    }

    /// [`exchange`] with food at 32767 a ton and ship 128 holding 2000
    /// tons.
    fn dear_exchange() -> FakePilotCatalog {
        let mut commodities = food_and_metal();
        commodities.base_prices[0] = "32767".to_owned();
        FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    holds: 2000,
                    ..FAST
                }),
            )],
            commodities,
            ..exchange()
        }
    }

    #[test]
    fn the_session_buys_the_most_by_its_quotient_rule() {
        for (source, tons, cash) in [
            (None, 1000, -1),
            (Some(RuleSource::Engine), 1000, -1),
            (Some(RuleSource::Bible), 999, 32_766),
        ] {
            for lot in [Lot::Max, Lot::Count(tons)] {
                let session = Session::start(&dear_exchange()).expect("starts");
                let mut session = match source {
                    Some(source) => session.with_rules(
                        Rulebook::default().with_override(RuleKey::TradeQuotient, source),
                    ),
                    None => session,
                };
                let source = source.unwrap_or_default();
                assert_eq!(session.rules().source_for(RuleKey::TradeQuotient), source);
                session.pilot.set_cash(32_766_999);
                land_now(&mut session).expect("lands");
                let market = session.market().expect("an exchange");
                assert_eq!(market.rules.source_for(RuleKey::TradeQuotient), source);
                assert_eq!(market.row_max(0, Direction::Buy), Some(i64::from(tons)));
                assert_eq!(
                    session.trade(order(&session, FOOD, Direction::Buy, lot)),
                    Ok(tons),
                    "{source:?} {lot:?}"
                );
                assert_eq!(session.pilot().cash(), cash, "{source:?} {lot:?}");
                assert_eq!(session.pilot().held(FOOD), tons);
            }
        }
    }

    #[test]
    fn the_sessions_exchange_carries_its_count_rule() {
        for source in [None, Some(RuleSource::Engine), Some(RuleSource::Bible)] {
            let session = Session::start(&exchange()).expect("starts");
            let mut session = match source {
                Some(source) => session
                    .with_rules(Rulebook::default().with_override(RuleKey::TradeCount, source)),
                None => session,
            };
            let source = source.unwrap_or_default();
            assert_eq!(session.rules().source_for(RuleKey::TradeCount), source);
            assert_eq!(
                session.rules().source_for(RuleKey::TradeQuotient),
                RuleSource::Engine
            );
            land_now(&mut session).expect("lands");
            let market = session.market().expect("an exchange");
            assert_eq!(
                market.rules.source_for(RuleKey::TradeCount),
                source,
                "{source:?}"
            );
        }
    }

    #[test]
    fn the_sessions_exchange_carries_its_debt_rule() {
        for source in [None, Some(RuleSource::Engine), Some(RuleSource::Bible)] {
            let session = Session::start(&exchange()).expect("starts");
            let mut session = match source {
                Some(source) => session
                    .with_rules(Rulebook::default().with_override(RuleKey::TradeDebt, source)),
                None => session,
            };
            let source = source.unwrap_or_default();
            assert_eq!(session.rules().source_for(RuleKey::TradeDebt), source);
            assert_eq!(
                session.rules().source_for(RuleKey::TradeCount),
                RuleSource::Engine
            );
            land_now(&mut session).expect("lands");
            let market = session.market().expect("an exchange");
            assert_eq!(
                market.rules.source_for(RuleKey::TradeDebt),
                source,
                "{source:?}"
            );
        }
    }

    /// [`exchange`] landed at its planet 128, which sells `jünk` 146 at
    /// base -100 (its low price, -80), the player holding none of it and
    /// with cash of -80, exactly that price, read as `source` says.
    fn debt_session(source: Option<RuleSource>) -> Session {
        let catalog = FakePilotCatalog {
            junk: vec![JunkRecord {
                id: JunkId(146),
                name: "Waste".to_owned(),
                abbrev: String::new(),
                base_price: -100,
                sold_at: vec![StellarId(128)],
                bought_at: Vec::new(),
                buy_on: Test::default(),
                sell_on: Test::default(),
                flags: 0,
                scan_mask: 0,
                lc_name: String::new(),
            }],
            ..exchange()
        };
        let session = Session::start(&catalog).expect("starts");
        let mut session = match source {
            Some(source) => {
                session.with_rules(Rulebook::default().with_override(RuleKey::TradeDebt, source))
            }
            None => session,
        };
        session.pilot.set_cash(-80);
        land_now(&mut session).expect("lands");
        session.take_save_due();
        session
    }

    #[test]
    fn by_the_engine_negative_cash_buys_a_junk_priced_at_it_and_leaves_none() {
        let waste = Good::Junk(JunkId(146));
        for lot in [Lot::Click, Lot::Count(1)] {
            let mut session = debt_session(None);
            let market = session.market().expect("an exchange");
            assert_eq!(market.row(waste).map(|row| row.price), Some(-80));
            assert_eq!(session.pilot().held(waste), 0);
            assert_eq!(
                session.trade(order(&session, waste, Direction::Buy, lot)),
                Ok(1),
                "{lot:?}"
            );
            assert_eq!(session.pilot().cash(), 0, "{lot:?}");
            assert_eq!(session.pilot().held(waste), 1, "{lot:?}");
            assert!(session.take_save_due(), "{lot:?}");
        }
    }

    #[test]
    fn by_the_other_debt_reading_negative_cash_buys_nothing() {
        let waste = Good::Junk(JunkId(146));
        let mut session = debt_session(Some(RuleSource::Bible));
        let before = session.clone();
        for lot in [Lot::Click, Lot::Count(1)] {
            assert_eq!(
                session.trade(order(&session, waste, Direction::Buy, lot)),
                Err(TradeRefusal::CannotAfford),
                "{lot:?}"
            );
        }
        assert_eq!(session, before);
        assert_eq!(session.pilot().cash(), -80);
        assert!(!session.take_save_due());
    }

    // Tribbles and perishables.

    const FURBALLS: Good = Good::Junk(JunkId(128));
    const FRUIT: Good = Good::Junk(JunkId(129));
    const MORE_FURBALLS: Good = Good::Junk(JunkId(130));
    const ROTTING_FURBALLS: Good = Good::Junk(JunkId(131));

    /// [`exchange`], its planet 128 selling `jünk` 128 and 130, tribbles,
    /// 129, perishable, and 131, both, at 8 a ton.
    fn breeding() -> FakePilotCatalog {
        let sold = |id, flags| JunkRecord {
            id: JunkId(id),
            name: format!("Junk {id}"),
            abbrev: String::new(),
            base_price: 10,
            sold_at: vec![StellarId(128)],
            bought_at: Vec::new(),
            buy_on: Test::default(),
            sell_on: Test::default(),
            flags,
            scan_mask: 0,
            lc_name: String::new(),
        };
        FakePilotCatalog {
            junk: vec![
                sold(128, market::TRIBBLES),
                sold(129, market::PERISHABLE),
                sold(130, market::TRIBBLES),
                sold(131, market::TRIBBLES | market::PERISHABLE),
            ],
            ..exchange()
        }
    }

    /// `session`, landed on planet 128 of [`breeding`], buys `tons` of
    /// each good, a ton a click: it is left on the other reading of
    /// [`RuleKey::TradeLot`](crate::RuleKey::TradeLot).
    fn buy_goods(session: &mut Session, tons: &[(Good, u32)]) {
        session.rules = session
            .rules
            .with_override(RuleKey::TradeLot, RuleSource::Bible);
        for &(good, tons) in tons {
            for _ in 0..tons {
                let bought = session.trade(order(session, good, Direction::Buy, Lot::Click));
                assert_eq!(bought, Ok(1), "{good:?}");
            }
        }
    }

    /// `session`, landed on planet 128 of [`breeding`], buys `tons` of
    /// each good and takes off.
    fn take_off_with(session: &mut Session, tons: &[(Good, u32)]) {
        buy_goods(session, tons);
        assert_eq!(
            session.take_off(&NoTraffic, &mut NeverFires),
            Some(StellarId(128))
        );
    }

    /// A session of [`breeding`] that has bought `tons` of each good at
    /// planet 128 and taken off.
    fn taken_off_with(tons: &[(Good, u32)]) -> Session {
        let mut session = Session::start(&breeding()).expect("starts");
        land_now(&mut session).expect("lands");
        take_off_with(&mut session, tons);
        session
    }

    /// The tons of furballs and of fruit held.
    fn furballs_and_fruit(session: &Session) -> (u32, u32) {
        (session.pilot().held(FURBALLS), session.pilot().held(FRUIT))
    }

    /// The ticks, from 1, on which `session`'s cargo changes in `ticks`
    /// ticks with no keys held.
    fn steps_in(session: &mut Session, ticks: u32) -> Vec<u32> {
        let mut steps = Vec::new();
        for tick in 1..=ticks {
            let before = session.pilot().cargo().collect::<Vec<_>>();
            session.tick(Controls::default());
            if session.pilot().cargo().collect::<Vec<_>>() != before {
                steps.push(tick);
            }
        }
        steps
    }

    #[test]
    fn after_a_take_off_tribbles_grow_and_perishables_decay_on_the_15th_tick_and_every_due_frame() {
        let mut session = taken_off_with(&[(FURBALLS, 2), (FRUIT, 10)]);
        assert_eq!(steps_in(&mut session, 14), Vec::<u32>::new());
        assert_eq!(furballs_and_fruit(&session), (2, 10));
        assert_eq!(steps_in(&mut session, 1), [1], "the 15th tick");
        assert_eq!(furballs_and_fruit(&session), (3, 9));
        assert_eq!(steps_in(&mut session, 249), Vec::<u32>::new());
        assert_eq!(steps_in(&mut session, 1), [1], "the 265th tick");
        assert_eq!(furballs_and_fruit(&session), (4, 8));
        assert_eq!(
            steps_in(&mut session, 1040 - 265),
            [515, 765, 1015, 1040].map(|tick| tick - 265),
            "four 250 ticks apart, then the next 25 ticks later"
        );
        assert_eq!(furballs_and_fruit(&session), (8, 4));
    }

    #[test]
    fn a_both_flags_good_decays_after_the_perishable_only_good_is_gone_until_the_next_reset() {
        let mut session = taken_off_with(&[(FRUIT, 1), (ROTTING_FURBALLS, 5)]);
        let held = |session: &Session| {
            (
                session.pilot().held(FRUIT),
                session.pilot().held(ROTTING_FURBALLS),
            )
        };
        assert_eq!(steps_in(&mut session, 15), [15]);
        assert_eq!(
            held(&session),
            (0, 5),
            "the fruit gone; 5 grown to 6, decayed to 5"
        );
        assert_eq!(steps_in(&mut session, 250), Vec::<u32>::new());
        assert_eq!(
            held(&session),
            (0, 5),
            "the take-off's perishable latch still holds (@0x708c3): 6 if read live"
        );
        session.run_script(&crate::control::Script::parse("b1"), &mut NeverFires);
        assert_eq!(steps_in(&mut session, 250), [250]);
        assert_eq!(
            held(&session),
            (0, 6),
            "a reset with no perishable-only good aboard"
        );
    }

    #[test]
    fn landed_ticks_move_neither_the_cargo_nor_the_frame_counter() {
        let mut session = Session::start(&breeding()).expect("starts");
        land_now(&mut session).expect("lands");
        buy_goods(&mut session, &[(FURBALLS, 19)]);
        let frame = session.frame;
        assert_eq!(steps_in(&mut session, 300), Vec::<u32>::new());
        assert_eq!(session.frame, frame, "the counter stands still");
        take_off_with(&mut session, &[]);
        assert_eq!(steps_in(&mut session, 15), [15], "still the 15th tick");
        assert_eq!(session.pilot().held(FURBALLS), 20, "the hold filled");
    }

    /// A session of [`breeding`] by `source`, whose fleet is a trader
    /// escort of `holds` tons (ship class 129), that has filled the ship's
    /// own 20 tons with furballs at planet 128 and taken off.
    fn furballs_in_a_full_ship(source: RuleSource, holds: Option<i16>) -> Session {
        let catalog = FakePilotCatalog {
            ship_records: vec![holds_record(129, holds.unwrap_or(0), 1)],
            ..breeding()
        };
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::JunkFlags, source));
        if holds.is_some() {
            session.pilot.escorts = vec![fleet_escort(129)];
        }
        land_now(&mut session).expect("lands");
        take_off_with(&mut session, &[(FURBALLS, 20)]);
        session
    }

    #[test]
    fn tribbles_grow_into_a_trader_escorts_holds() {
        let mut session = furballs_in_a_full_ship(RuleSource::Engine, Some(5));
        assert_eq!(steps_in(&mut session, 15), [15]);
        assert_eq!(session.pilot().held(FURBALLS), 21);
        let mut alone = furballs_in_a_full_ship(RuleSource::Engine, None);
        assert_eq!(steps_in(&mut alone, 15), Vec::<u32>::new());
        assert_eq!(alone.pilot().held(FURBALLS), 20, "no free space");
    }

    #[test]
    fn the_mission_cargo_aboard_takes_the_tribbles_space() {
        let mut session = taken_off_with(&[(FURBALLS, 2), (FRUIT, 3)]);
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 15));
        assert_eq!(steps_in(&mut session, 300), Vec::<u32>::new());
        assert_eq!(furballs_and_fruit(&session), (2, 3), "20 tons of 20");
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 14));
        assert_eq!(steps_in(&mut session, 250), [250 - 35]);
        assert_eq!(furballs_and_fruit(&session), (3, 2), "a ton free");
    }

    #[test]
    fn by_the_bible_tribbles_grow_only_into_the_fleets_free_space() {
        let mut session = furballs_in_a_full_ship(RuleSource::Bible, Some(2));
        assert_eq!(steps_in(&mut session, 600), [15, 265]);
        assert_eq!(session.pilot().held(FURBALLS), 22);
    }

    #[test]
    fn a_step_falls_due_in_hyperspace_too() {
        let catalog = breeding();
        let mut session = taken_off_with(&[(FURBALLS, 1), (FRUIT, 15)]);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        let held = furballs_and_fruit(&session);
        session.frame = market::JUNK_STEP_FRAMES - 1;
        assert_eq!(steps_in(&mut session, 2), [2]);
        assert_eq!(session.jumping(), Some(SystemId(131)), "still jumping");
        assert_eq!(furballs_and_fruit(&session), (held.0 + 1, held.1 - 1));
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
    }

    #[test]
    fn a_destroyed_players_ticks_do_not_step_but_the_counter_moves_on() {
        let mut session = taken_off_with(&[(FURBALLS, 2), (FRUIT, 2)]);
        session.condition = Condition::Destroyed;
        session.frame = 0;
        assert_eq!(steps_in(&mut session, 1), Vec::<u32>::new());
        assert_eq!(furballs_and_fruit(&session), (2, 2));
        assert_eq!(session.frame, 1);
    }

    #[test]
    fn a_step_makes_no_save_due() {
        let mut session = taken_off_with(&[(FURBALLS, 2)]);
        session.take_save_due();
        assert_eq!(steps_in(&mut session, 15), [15]);
        assert!(!session.take_save_due());
    }

    #[test]
    fn the_session_reads_the_junk_flags_as_its_rule_says() {
        let tons = [(FURBALLS, 10), (MORE_FURBALLS, 9)];
        let mut engine = taken_off_with(&tons);
        assert_eq!(
            engine.rules().source_for(RuleKey::JunkFlags),
            RuleSource::Engine
        );
        steps_in(&mut engine, 15);
        let held =
            |session: &Session| [FURBALLS, MORE_FURBALLS].map(|good| session.pilot().held(good));
        assert_eq!(held(&engine), [11, 10], "21 tons of 20");
        let mut bible = Session::start(&breeding())
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::JunkFlags, RuleSource::Bible));
        assert_eq!(
            bible.rules().source_for(RuleKey::JunkFlags),
            RuleSource::Bible
        );
        land_now(&mut bible).expect("lands");
        take_off_with(&mut bible, &tons);
        steps_in(&mut bible, 15);
        assert_eq!(held(&bible), [11, 9], "only the lower ID, into the space");
    }

    #[test]
    fn the_exchange_tests_junk_through_the_sessions_control_bits() {
        let opals = JunkRecord {
            id: JunkId(146),
            name: "Opals".to_owned(),
            abbrev: String::new(),
            base_price: 100,
            sold_at: vec![StellarId(128)],
            bought_at: Vec::new(),
            buy_on: Test::default(),
            sell_on: Test::parse("b7"),
            flags: 0,
            scan_mask: 0,
            lc_name: String::new(),
        };
        let catalog = FakePilotCatalog {
            character: exchange().character,
            sites: vec![(
                SystemId(130),
                vec![LandingSite {
                    flags: TRADES,
                    ..planet(128, 0.0, 0.0)
                }],
            )],
            junk: vec![opals],
            ..catalog()
        };
        let opals = Good::Junk(JunkId(146));
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session.set_control_bit(crate::control::Bit::new(7).expect("a bit"), true);
        let mut refusing = session
            .clone()
            .with_control_bits(Rc::new(crate::testkit::RefuseBits(&[7])));
        assert_eq!(
            refusing
                .market()
                .and_then(|market| market.row(opals).cloned()),
            None
        );
        assert_eq!(
            refusing.trade(order(&refusing, opals, Direction::Buy, Lot::Count(1))),
            Err(TradeRefusal::NotTraded)
        );
        let buy = order(&session, opals, Direction::Buy, Lot::Count(1));
        assert_eq!(session.trade(buy), Ok(1));
    }

    /// A food surplus at planet 140 (-15, 30 days, 35 % a day), and planet
    /// 140 a trade center trading food at 75.
    pub(super) fn surplus() -> FakePilotCatalog {
        let landers = edge_lander();
        FakePilotCatalog {
            sites: vec![(
                SystemId(131),
                vec![LandingSite {
                    flags: TRADES,
                    ..planet(140, -1000.0, 0.0)
                }],
            )],
            commodities: food_and_metal(),
            disasters: vec![DisasterRecord {
                id: DisasterId(128),
                name: "An enormous food surplus".to_owned(),
                stellar: 140,
                commodity: 0,
                price_delta: -15,
                duration: 30,
                freq: 35,
                activate_on: Test::default(),
            }],
            ..landers
        }
    }

    #[test]
    fn an_event_starts_on_arrival_when_the_chance_fires_and_moves_the_price() {
        let catalog = surplus();
        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = Scripted::answering(&[true]);
        jump_with(&mut session, &catalog, 131, &mut chance);
        assert_eq!(chance.asked, [35], "one roll for the one day");
        assert_eq!(
            session.pilot().events().collect::<Vec<_>>(),
            [(DisasterId(128), 30)]
        );
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        let market = session.market().expect("an exchange");
        assert_eq!(market.row(FOOD).map(|row| row.price), Some(60));
        assert_eq!(market.events, ["An enormous food surplus"]);
    }

    #[test]
    fn a_jump_tests_activate_on_through_the_sessions_control_bits() {
        let mut catalog = surplus();
        catalog.disasters[0].freq = 100;
        catalog.disasters[0].activate_on = Test::parse("b7");
        let mut session = Session::start(&catalog).expect("starts");
        session.set_control_bit(crate::control::Bit::new(7).expect("a bit"), true);
        let mut refusing = session
            .clone()
            .with_control_bits(Rc::new(crate::testkit::RefuseBits(&[7])));
        let mut chance = Scripted::answering(&[true]);
        jump_with(&mut refusing, &catalog, 131, &mut chance);
        assert_eq!(refusing.pilot().events().count(), 0);
        assert_eq!(chance.asked, Vec::<u8>::new(), "never rolled");
        jump_with(&mut session, &catalog, 131, &mut chance);
        assert_eq!(
            session.pilot().events().collect::<Vec<_>>(),
            [(DisasterId(128), 30)]
        );
    }

    #[test]
    fn the_session_prices_events_as_its_rule_says() {
        let mut catalog = surplus();
        catalog.sites[0].1[0].flags = TRADES & !(0xF << 28) | 4 << 28;
        if let Ok(start) = &mut catalog.character {
            start.cash = 1000;
        }
        assert_eq!(
            Session::start(&catalog)
                .expect("starts")
                .rules()
                .source_for(RuleKey::EventPrice),
            RuleSource::Engine
        );
        for (source, price) in [(RuleSource::Engine, 60), (RuleSource::Bible, 78)] {
            let mut session = Session::start(&catalog).expect("starts").with_rules(
                Rulebook::default()
                    .with_override(RuleKey::EventPrice, source)
                    .with_override(RuleKey::TradeLot, RuleSource::Bible),
            );
            assert_eq!(session.rules().source_for(RuleKey::EventPrice), source);
            jump_with(
                &mut session,
                &catalog,
                131,
                &mut Scripted::answering(&[true]),
            );
            land_now(&mut session).expect("lands");
            let market = session.market().expect("an exchange");
            assert_eq!(
                market.row(FOOD).map(|row| row.price),
                Some(price),
                "{source:?}"
            );
            let cash = session.pilot().cash();
            assert_eq!(
                session.trade(order(&session, FOOD, Direction::Buy, Lot::Click)),
                Ok(1)
            );
            assert_eq!(cash - session.pilot().cash(), price, "{source:?}");
        }
    }

    #[test]
    fn no_event_starts_when_the_chance_does_not_fire() {
        let catalog = surplus();
        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = Scripted::default();
        jump_with(&mut session, &catalog, 131, &mut chance);
        assert_eq!(chance.asked, [35]);
        assert_eq!(session.pilot().events().count(), 0);
        land_now(&mut session).expect("lands");
        let market = session.market().expect("an exchange");
        assert_eq!(market.row(FOOD).map(|row| row.price), Some(75));
        assert_eq!(market.events, Vec::<String>::new());
    }

    /// [`surplus`], its event at any stellar: 139 or 140.
    fn drifting() -> FakePilotCatalog {
        let mut catalog = surplus();
        catalog.disasters[0].stellar = -1;
        catalog.stellars = vec![(StellarId(139), TRADES), (StellarId(140), TRADES)];
        catalog
    }

    /// The stellar each event under way is at.
    fn places(session: &Session) -> Vec<(DisasterId, Option<StellarId>)> {
        session
            .pilot
            .events
            .iter()
            .map(|(&id, active)| (id, active.stellar))
            .collect()
    }

    #[test]
    fn an_any_stellar_event_starts_at_a_drawn_stellar_and_moves_the_price_there_only() {
        let catalog = drifting();
        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = Scripted::answering(&[true]).and_rolling(&[1]);
        jump_with(&mut session, &catalog, 131, &mut chance);
        assert_eq!(
            chance.sides_asked,
            [vec![2], vec![100; crate::mission::ROLLED]].concat(),
            "the stellar, then the arrival's table"
        );
        assert_eq!(places(&session), [(DisasterId(128), Some(StellarId(140)))]);
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        let market = session.market().expect("an exchange");
        assert_eq!(market.row(FOOD).map(|row| row.price), Some(60));
        assert_eq!(market.events, ["An enormous food surplus"]);

        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = Scripted::answering(&[true]).and_rolling(&[0]);
        jump_with(&mut session, &catalog, 131, &mut chance);
        assert_eq!(places(&session), [(DisasterId(128), Some(StellarId(139)))]);
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        let market = session.market().expect("an exchange");
        assert_eq!(market.row(FOOD).map(|row| row.price), Some(75));
        assert_eq!(market.events, Vec::<String>::new());
    }

    #[test]
    fn an_any_stellar_event_keeps_its_stellar_across_a_save_and_reload() {
        let catalog = drifting();
        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = Scripted::answering(&[true]).and_rolling(&[1]);
        jump_with(&mut session, &catalog, 131, &mut chance);
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        let saved = crate::save::encode(session.pilot());
        let pilot = crate::save::decode(&saved).expect("loads");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(places(&session), [(DisasterId(128), Some(StellarId(140)))]);
        let market = session.market().expect("docked at the exchange");
        assert_eq!(market.events, ["An enormous food surplus"]);
        session.take_off(&catalog, &mut NeverFires);
        let mut chance = Scripted::default();
        jump_with(&mut session, &catalog, 132, &mut chance);
        // The take-off's day ages it too (`_PlayerLandOnStellar` @0x63440).
        assert_eq!(
            session.pilot().events().collect::<Vec<_>>(),
            [(DisasterId(128), 28)]
        );
        assert_eq!(places(&session), [(DisasterId(128), Some(StellarId(140)))]);
        assert_eq!(
            chance.sides_asked,
            vec![100; crate::mission::ROLLED],
            "the arrival's table alone: not drawn again"
        );
    }

    #[test]
    fn each_day_of_a_jump_ages_the_events() {
        let catalog = surplus();
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.events.insert(
            DisasterId(128),
            crate::market::ActiveEvent {
                days: 5,
                stellar: Some(StellarId(140)),
            },
        );
        jump(&mut session, &catalog, 131);
        assert_eq!(
            session.pilot().events().collect::<Vec<_>>(),
            [(DisasterId(128), 4)],
            "a light hull's one day"
        );
    }

    /// `catalog` with ship 128 carrying `count` of outfit 320, which has
    /// `mod_type` at `mod_val`, as a default item: a new pilot owns them.
    fn owning(
        catalog: FakePilotCatalog,
        mod_type: i16,
        mod_val: i16,
        count: u16,
    ) -> FakePilotCatalog {
        FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(OutfitId(320), count)])],
            outfits: vec![outfit(320, &[(mod_type, mod_val)])],
            ..catalog
        }
    }

    #[test]
    fn a_hyperspace_distance_outfit_lets_the_ship_jump_nearer_the_centre() {
        // The stock Horizontal Booster: -500, halving the no-jump zone.
        let booster = owning(catalog(), HYPERSPACE_DISTANCE, -500, 1);
        let mut session = Session::start(&booster).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        session.player.position = Vec2::new(0.0, 600.0);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        assert_eq!(
            session.arrive(&booster, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(
            session.player().position.length(),
            ARRIVAL_DISTANCE,
            "it still drops out at the standard edge"
        );

        let catalog = catalog();
        let mut plain = Session::start(&catalog).expect("starts");
        plain.plot_course(SystemId(131)).expect("a route");
        plain.player.position = Vec2::new(0.0, 600.0);
        assert_eq!(
            plain.begin_jump(),
            Err(JumpRefusal::TooClose { distance: 600.0 })
        );
        assert_eq!(plain.jumping(), None);
    }

    #[test]
    fn after_arriving_and_a_second_of_flight_the_ship_can_jump_on() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(jump(&mut session, &catalog, 132), Some(SystemId(131)));
        for _ in 0..TICKS_PER_SECOND {
            session.tick(Controls::default());
        }
        assert_eq!(session.begin_jump(), Ok(SystemId(132)));
    }

    #[test]
    fn with_a_raised_jump_distance_the_ship_arrives_outside_it_and_can_jump_on() {
        let raised = owning(catalog(), HYPERSPACE_DISTANCE, 250, 1);
        let mut session = Session::start(&raised).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        session.player.position = Vec2::new(0.0, 1300.0);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        assert_eq!(
            session.arrive(&raised, &mut NeverFires),
            Some(SystemId(131))
        );
        let distance = session.player().position.length();
        assert!(distance >= 1250.0, "outside the raised zone: {distance}");
        for _ in 0..TICKS_PER_SECOND {
            session.tick(Controls::default());
        }
        assert_eq!(session.begin_jump(), Ok(SystemId(132)));
    }

    #[test]
    fn a_hyperspace_speed_outfit_makes_each_jump_take_more_days() {
        let slower = owning(surplus(), HYPERSPACE_DAYS, 1, 1);
        let mut session = Session::start(&slower).expect("starts");
        let mut chance = Scripted::default();
        jump_with(&mut session, &slower, 131, &mut chance);
        assert_eq!(chance.asked, [35, 35], "one roll for each of two days");
        assert_eq!(dmy(&session), (25, 6, 1177));
        assert_eq!(session.reserves().fuel.now, 200.0, "still one jump's fuel");

        // The stock -1 cannot take a jump below a day.
        let quicker = owning(surplus(), HYPERSPACE_DAYS, -1, 1);
        let mut session = Session::start(&quicker).expect("starts");
        let mut chance = Scripted::default();
        jump_with(&mut session, &quicker, 131, &mut chance);
        assert_eq!(chance.asked, [35]);
        assert_eq!(dmy(&session), (24, 6, 1177));
    }

    #[test]
    fn a_heavy_hull_jumps_for_three_days_and_the_dampener_takes_one_off() {
        let heavy = |catalog: FakePilotCatalog| FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { mass: 250, ..FAST }))],
            ..catalog
        };
        let catalog = heavy(surplus());
        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = Scripted::default();
        jump_with(&mut session, &catalog, 131, &mut chance);
        assert_eq!(
            chance.asked,
            [35, 35, 35],
            "one roll for each of three days"
        );
        assert_eq!(dmy(&session), (26, 6, 1177));
        let fuel = session.reserves().fuel;
        assert_eq!(fuel.max - fuel.now, 100.0, "one jump's fuel");

        // The stock Sutherland Alluvial Dampener: -1.
        let dampened = heavy(owning(surplus(), HYPERSPACE_DAYS, -1, 1));
        let mut session = Session::start(&dampened).expect("starts");
        let mut chance = Scripted::default();
        jump_with(&mut session, &dampened, 131, &mut chance);
        assert_eq!(chance.asked, [35, 35]);
        assert_eq!(dmy(&session), (25, 6, 1177));
    }

    // Multi-jump.

    /// `base` on a longer map: 130 (0, 0) links to 131 (600, 0) and 133
    /// (-600, 0), 131 to 132 (600, 600) and 132 to 134 (1200, 600), so the
    /// course to 134 is 131, 132, 134; ship 128 carries one outfit of
    /// multi-jump `mod_val`.
    pub(super) fn chained(base: FakePilotCatalog, mod_val: i16) -> FakePilotCatalog {
        FakePilotCatalog {
            star_map: vec![
                star(130, (0.0, 0.0), &[131, 133]),
                star(131, (600.0, 0.0), &[132]),
                star(132, (600.0, 600.0), &[134]),
                star(133, (-600.0, 0.0), &[]),
                star(134, (1200.0, 600.0), &[]),
            ],
            ..owning(base, MULTI_JUMP, mod_val, 1)
        }
    }

    /// Plots a course to 134, flies out and begins the jump.
    pub(super) fn bound_for_134(catalog: &FakePilotCatalog, rule: MultiJumpRule) -> Session {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_multi_jump(rule);
        session.plot_course(SystemId(134)).expect("a route");
        assert_eq!(session.course(), ids(&[131, 132, 134]));
        fly_out(&mut session);
        session
    }

    fn arrivals(session: &mut Session) -> usize {
        session
            .take_sounds()
            .into_iter()
            .filter(|sound| *sound == SimSound::Arrived)
            .count()
    }

    #[test]
    fn a_multi_jump_passes_modval_systems_along_the_course_for_one_jumps_fuel_and_days() {
        let catalog = chained(catalog(), 2);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        assert_eq!(session.jumping(), Some(SystemId(131)), "the first hop");
        session.take_sounds();
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.course(), ids(&[134]));
        assert_eq!(session.jumping(), None);
        assert_eq!(session.reserves().fuel.now, 200.0, "one jump's fuel");
        assert_eq!(dmy(&session), (24, 6, 1177), "one jump's day");
        assert_eq!(
            session.pilot().explored().collect::<Vec<_>>(),
            [SystemId(130), SystemId(132)],
            "131 passed through"
        );
        assert_eq!(
            *catalog.sites_asked.borrow(),
            [SystemId(130), SystemId(132)]
        );
        assert_eq!(arrivals(&mut session), 1);
        assert_eq!(
            *session.player(),
            crate::hyperspace::arrival(
                Vec2::new(600.0, 0.0),
                Vec2::new(600.0, 600.0),
                MIN_JUMP_DISTANCE
            ),
            "facing the system it last left"
        );
        assert_eq!(session.player().position, Vec2::new(0.0, -1001.0));
    }

    #[test]
    fn a_multi_jump_emits_one_arrived_message_for_the_final_system() {
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        begin_jump_now(&mut session).expect("jumps");
        session.arrive(&catalog, &mut NeverFires);
        assert_eq!(
            session.take_messages(),
            [SimMessage::Arrived(SystemId(134))]
        );
    }

    #[test]
    fn a_multi_jump_rolls_the_events_for_one_jumps_days() {
        let catalog = chained(surplus(), 2);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        begin_jump_now(&mut session).expect("jumps");
        let mut chance = Scripted::default();
        assert_eq!(session.arrive(&catalog, &mut chance), Some(SystemId(132)));
        assert_eq!(chance.asked, [35], "one roll for the one day");
    }

    #[test]
    fn the_chain_stops_where_the_course_ends() {
        // The stock Multi-Jumping Organ: 10.
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(134))
        );
        assert_eq!(session.course(), []);
        assert_eq!(session.reserves().fuel.now, 200.0);
        assert_eq!(dmy(&session), (24, 6, 1177));
        assert_eq!(
            session.player().position,
            Vec2::new(-1001.0, 0.0),
            "facing 132, west of 134"
        );
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    }

    #[test]
    fn an_engine_multi_jump_chains_on_with_no_fuel_left_after_the_first_hop() {
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        session.pilot.reserves.fuel.now = 100.0;
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(134))
        );
        assert_eq!(session.course(), []);
        assert_eq!(session.reserves().fuel.now, 0.0, "one jump's fuel");
        assert_eq!(dmy(&session), (24, 6, 1177), "one jump's day");
    }

    #[test]
    fn a_multi_jump_modval_of_one_makes_a_single_hop() {
        let catalog = chained(catalog(), 1);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.course(), ids(&[132, 134]));
    }

    #[test]
    fn a_course_replotted_during_the_jump_is_not_chained() {
        let catalog = chained(catalog(), 10);
        for source in RuleSource::ALL {
            let mut session = bound_for_134(&catalog, MultiJumpRule::default())
                .with_rules(Rulebook::default().with_override(RuleKey::RouteRewrite, source));
            begin_jump_now(&mut session).expect("jumps");
            session.plot_course(SystemId(133)).expect("a route");
            assert_eq!(session.course(), ids(&[133]));
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(131))
            );
            // An arrival off the course clears it by the engine
            // (@0x6c472-0x6c4c6).
            let kept = match source {
                RuleSource::Engine => ids(&[]),
                RuleSource::Bible => ids(&[133]),
            };
            assert_eq!(session.course(), kept, "{source:?}");
            assert_eq!(session.reserves().fuel.now, 200.0);
        }
    }

    #[test]
    fn per_hop_makes_one_plus_modval_hops_each_taking_fuel_and_days() {
        let catalog = chained(catalog(), 1);
        let mut session = bound_for_134(&catalog, MultiJumpRule::PerHop);
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.course(), ids(&[134]));
        assert_eq!(session.reserves().fuel.now, 100.0, "two jumps' fuel");
        assert_eq!(dmy(&session), (25, 6, 1177), "two jumps' days");

        let catalog = chained(surplus(), 1);
        let mut session = bound_for_134(&catalog, MultiJumpRule::PerHop);
        begin_jump_now(&mut session).expect("jumps");
        let mut chance = Scripted::default();
        session.arrive(&catalog, &mut chance);
        assert_eq!(chance.asked, [35, 35], "one roll for each hop's day");
    }

    #[test]
    fn per_hop_stops_cleanly_when_a_hop_lacks_fuel() {
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::PerHop);
        session.pilot.reserves.fuel.now = 250.0;
        begin_jump_now(&mut session).expect("jumps");
        session.take_sounds();
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.reserves().fuel.now, 50.0);
        assert_eq!(dmy(&session), (25, 6, 1177));
        assert_eq!(session.course(), ids(&[134]));
        assert_eq!(
            session.pilot().explored().collect::<Vec<_>>(),
            [SystemId(130), SystemId(132)]
        );
        assert_eq!(arrivals(&mut session), 1);
    }

    #[test]
    fn per_hop_takes_a_hop_with_exactly_a_jumps_fuel() {
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::PerHop);
        session.pilot.reserves.fuel.now = 200.0;
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.reserves().fuel.now, 0.0);
    }

    #[test]
    fn per_hop_stops_where_the_course_ends() {
        let catalog = chained(catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::PerHop);
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(134))
        );
        assert_eq!(session.course(), []);
        assert_eq!(session.reserves().fuel.now, 0.0);
        assert_eq!(dmy(&session), (26, 6, 1177));
    }

    #[test]
    fn a_pilot_flown_with_cargo_has_less_space_and_can_sell_it() {
        let catalog = exchange();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        assert_eq!(
            session.trade(order(&session, METAL, Direction::Buy, Lot::Max)),
            Ok(6)
        );
        let pilot = session.pilot().clone();
        let mut resumed = Session::fly(&catalog, pilot).expect("flies");
        let market = resumed.market().expect("docked at the exchange");
        assert_eq!(market.free, 14);
        assert_eq!(market.row(METAL).map(|row| row.held), Some(6));
        assert_eq!(
            resumed.trade(order(&resumed, METAL, Direction::Sell, Lot::Max)),
            Ok(6)
        );
        assert_eq!(resumed.pilot().cash(), 1000);
    }

    // The outfitter.

    use crate::catalog::{GovtId, OutfitRecord};
    use crate::order_loop::{OrderRun, Stopped};
    use crate::outfitter::{OutfitFlags, OutfitOrder, OutfitRefusal, OutfitVisit};
    use crate::stats::{MORE_FUEL, MORE_SHIELD, MORE_SPEED};

    const SPEED: OutfitId = OutfitId(300);
    const CARGO: OutfitId = OutfitId(301);
    const SHIELD: OutfitId = OutfitId(302);
    const TANK: OutfitId = OutfitId(303);
    const SCOOP: OutfitId = OutfitId(304);

    /// Planet 128 at the centre, which the ship starts over: an outfitter
    /// of tech level 5, of government 128, and a trade center as
    /// [`TRADES`] says.
    fn outfitter_site() -> LandingSite {
        LandingSite {
            flags: TRADES | StellarFlags::OUTFITTER,
            tech_level: 5,
            govt: Some(GovtId(128)),
            ..planet(128, 0.0, 0.0)
        }
    }

    /// The catalog with planet 128 an outfitter selling a speed booster
    /// (+300), a cargo pod (+10 tons), a shield (+50), a fuel tank (+100)
    /// and a fuel scoop (a unit every 10 ticks), each a ton and 1000
    /// credits; the first `chär` holds 25,000 credits.
    pub(super) fn outfitting() -> FakePilotCatalog {
        FakePilotCatalog {
            character: Ok(CharacterStart {
                cash: 25_000,
                ..exchange().character.expect("a chär")
            }),
            sites: vec![(SystemId(130), vec![outfitter_site()])],
            commodities: food_and_metal(),
            outfits: vec![
                outfit(300, &[(MORE_SPEED, 300)]),
                outfit(301, &[(crate::market::MORE_CARGO, 10)]),
                outfit(302, &[(MORE_SHIELD, 50)]),
                outfit(303, &[(MORE_FUEL, 100)]),
                outfit(304, &[(FUEL_SCOOP, 10)]),
            ],
            ..catalog()
        }
    }

    pub(super) fn buy(outfit: OutfitId) -> OutfitOrder {
        OutfitOrder {
            outfit,
            direction: Direction::Buy,
        }
    }

    pub(super) fn sell(outfit: OutfitId) -> OutfitOrder {
        OutfitOrder {
            outfit,
            direction: Direction::Sell,
        }
    }

    pub(super) fn outfitted(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session.take_save_due();
        session
    }

    #[test]
    fn there_is_an_outfitter_only_while_landed_at_one() {
        let catalog = outfitting();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.outfitter(&mut NeverFires), None, "in flight");
        assert_eq!(
            session.outfit(buy(SPEED), &mut NeverFires),
            Err(OutfitRefusal::NoOutfitter)
        );
        land_now(&mut session).expect("lands");
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        assert_eq!(outfitter.rows.len(), 5);
        assert_eq!((outfitter.cash, outfitter.free_mass), (25_000, 30));
        let plain = FakePilotCatalog {
            sites: vec![(
                SystemId(130),
                vec![LandingSite {
                    flags: TRADES,
                    ..outfitter_site()
                }],
            )],
            ..outfitting()
        };
        let mut session = outfitted(&plain);
        assert_eq!(
            session.outfitter(&mut NeverFires),
            None,
            "no outfitter here"
        );
        assert_eq!(
            session.outfit(buy(SPEED), &mut NeverFires),
            Err(OutfitRefusal::NoOutfitter)
        );
    }

    #[test]
    fn the_sessions_outfitter_charges_an_outfit_priced_by_mass_as_its_rules_say() {
        // Armour 100 credits a ton of ship, on a start class of mass 0.
        const ARMOUR: OutfitId = OutfitId(320);
        let mut catalog = outfitting();
        catalog.outfits.push(OutfitRecord {
            cost: 100,
            flags: OutfitFlags::PRICE_BY_MASS,
            ..outfit(320, &[])
        });
        catalog.ships = vec![(ShipId(128), Ok(ShipFields { mass: 0, ..FAST }))];
        let charged = |source| {
            let rules = Rulebook::default().with_override(RuleKey::MassPrice, source);
            let mut session = outfitted(&catalog).with_rules(rules);
            let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
            let price = outfitter.row(ARMOUR).expect("listed").price;
            let cash = session.pilot().cash();
            session
                .outfit(buy(ARMOUR), &mut NeverFires)
                .expect("bought");
            assert_eq!(session.pilot().owned(ARMOUR), 1);
            (price, cash - session.pilot().cash())
        };
        assert_eq!(charged(RuleSource::Engine), (100, 100), "the cost");
        assert_eq!(charged(RuleSource::Bible), (0, 0), "100 x 0");
    }

    #[test]
    fn a_session_reads_the_outfits_once_when_it_starts() {
        let catalog = outfitting();
        let mut session = outfitted(&catalog);
        let reads = *catalog.outfit_reads.borrow();
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        session.outfitter(&mut NeverFires).expect("an outfitter");
        session.take_off(&catalog, &mut NeverFires);
        jump(&mut session, &catalog, 131);
        assert_eq!(*catalog.outfit_reads.borrow(), reads);
    }

    #[test]
    fn a_speed_booster_raises_the_top_speed_and_selling_it_lowers_it_again() {
        let mut session = outfitted(&outfitting());
        let before = session.handling();
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        assert_eq!(session.handling().max_speed, before.max_speed + 3.0);
        assert_eq!(session.handling().accel, before.accel);
        assert_eq!(session.pilot().owned(SPEED), 1);
        assert_eq!(session.pilot().cash(), 24_000);
        assert_eq!(session.outfit(sell(SPEED), &mut NeverFires), Ok(()));
        assert_eq!(session.handling(), before);
        assert_eq!(
            session.pilot().cash(),
            25_000,
            "bought this opening: sold back in full"
        );
    }

    #[test]
    fn a_cargo_pod_adds_space_to_the_exchange_too() {
        let mut session = outfitted(&outfitting());
        assert_eq!(session.capacity(), 20);
        session.outfit(buy(CARGO), &mut NeverFires).expect("bought");
        assert_eq!(session.capacity(), 30);
        assert_eq!(session.market().expect("an exchange").free, 30);
        session.outfit(sell(CARGO), &mut NeverFires).expect("sold");
        assert_eq!(session.capacity(), 20);
    }

    // Counted outfit orders.

    #[test]
    fn a_counted_buy_stops_when_the_cash_runs_out() {
        let mut session = outfitted(&outfitting());
        session.pilot.cash = 3500;
        assert_eq!(
            session.outfit_counted(buy(SPEED), 5, &mut NeverFires),
            Ok(3)
        );
        assert_eq!(session.pilot().owned(SPEED), 3);
        assert_eq!(session.pilot().cash(), 500);
        assert!(session.take_save_due());
    }

    #[test]
    fn a_counted_buy_stops_at_the_max() {
        let mut session = outfitted(&outfitting());
        assert_eq!(
            session.outfit_counted(buy(SPEED), 12, &mut NeverFires),
            Ok(10)
        );
        assert_eq!(session.pilot().owned(SPEED), 10);
        assert_eq!(session.pilot().cash(), 15_000);
    }

    #[test]
    fn a_counted_buy_stops_when_the_free_mass_runs_out() {
        let mut catalog = outfitting();
        catalog.outfits[0].mass = 10;
        let mut session = outfitted(&catalog);
        assert_eq!(
            session.outfit_counted(buy(SPEED), 5, &mut NeverFires),
            Ok(3),
            "30 tons free"
        );
    }

    #[test]
    fn a_counted_buy_of_maps_stops_after_one_as_the_rule_says() {
        let mut session = outfitted(&limiting());
        assert_eq!(session.outfit_counted(buy(MAP), 3, &mut NeverFires), Ok(1));
        let mut session = outfitted(&limiting())
            .with_rules(Rulebook::default().with_override(RuleKey::OutfitLimit, RuleSource::Bible));
        assert_eq!(session.outfit_counted(buy(MAP), 3, &mut NeverFires), Ok(3));
    }

    #[test]
    fn a_counted_sale_stops_when_none_are_left() {
        let mut session = outfitted(&outfitting());
        session
            .outfit_counted(buy(SPEED), 3, &mut NeverFires)
            .expect("bought");
        assert_eq!(
            session.outfit_counted(sell(SPEED), 5, &mut NeverFires),
            Ok(3)
        );
        assert_eq!(session.pilot().owned(SPEED), 0);
        assert_eq!(session.pilot().cash(), 25_000, "bought this opening");
    }

    #[test]
    fn a_counted_sale_stops_before_the_free_mass_goes_below_nothing() {
        // Two pods adding 10 tons each, and seven 5-ton boosters: 15
        // tons free, so one pod can be sold and the second cannot.
        let mut catalog = outfitting();
        catalog.outfits[0].mass = 5;
        catalog.outfits.push(OutfitRecord {
            mass: -10,
            ..outfit(310, &[])
        });
        let mut session = outfitted(&catalog);
        let pod = OutfitId(310);
        session
            .outfit_counted(buy(pod), 2, &mut NeverFires)
            .expect("bought");
        session
            .outfit_counted(buy(SPEED), 7, &mut NeverFires)
            .expect("bought");
        let outfitter = session.outfitter(&mut NeverFires).expect("open");
        assert_eq!(outfitter.count_max(pod, Direction::Sell), Some(2));
        assert_eq!(session.outfit_counted(sell(pod), 2, &mut NeverFires), Ok(1));
        assert_eq!(
            session.outfit(sell(pod), &mut NeverFires),
            Err(OutfitRefusal::NegativeFreeMass)
        );
    }

    #[test]
    fn a_counted_sale_stops_at_a_raised_max() {
        // Two racks and two widgets: the first rack sold leaves a `Max`
        // of 2, the second would leave none, each pass checked as a single
        // sale by the other reading of `RuleKey::CountedSale`.
        let mut session = outfitted(&raising())
            .with_rules(Rulebook::default().with_override(RuleKey::CountedSale, RuleSource::Bible));
        session
            .outfit_counted(buy(WIDGET_RACK), 2, &mut NeverFires)
            .expect("bought");
        session
            .outfit_counted(buy(WIDGET), 2, &mut NeverFires)
            .expect("bought");
        assert_eq!(
            session
                .outfitter(&mut NeverFires)
                .expect("open")
                .count_max(WIDGET_RACK, Direction::Sell),
            Some(2)
        );
        assert_eq!(
            session.outfit_counted(sell(WIDGET_RACK), 2, &mut NeverFires),
            Ok(1)
        );
        assert_eq!(session.pilot().owned(WIDGET_RACK), 1);
    }

    // A counted sale whose sale changes the listing
    // (`RuleKey::RelistedSale`).

    pub(super) const BLASTER: OutfitId = OutfitId(128);
    const CONTRABAND: OutfitId = OutfitId(312);

    /// [`outfitting`] also selling a blaster (128, tech 1), with
    /// contraband (312) that sells anywhere and is not for sale here
    /// (tech 9). Each is a ton and 1000 credits.
    fn relisting_catalog() -> FakePilotCatalog {
        let mut catalog = outfitting();
        catalog.outfits.extend([
            outfit(128, &[]),
            OutfitRecord {
                flags: OutfitFlags::SELL_ANYWHERE,
                tech_level: 9,
                ..outfit(312, &[])
            },
        ]);
        catalog
    }

    /// A session landed at `catalog`'s outfitter by `rules`, owning
    /// `owned` since the outfitter opened, so each refunds half its
    /// price; no roll is drawn yet.
    fn landed_owning(
        catalog: &FakePilotCatalog,
        rules: Rulebook,
        owned: &[(OutfitId, u16)],
    ) -> Session {
        let mut session = outfitted(catalog).with_rules(rules);
        session.pilot.outfits = owned.iter().copied().collect();
        session.open_outfitter();
        session
    }

    /// [`relisting_catalog`] landed by `rules`, owning 3 blasters and 2
    /// contraband.
    fn relisting(rules: Rulebook) -> Session {
        landed_owning(
            &relisting_catalog(),
            rules,
            &[(BLASTER, 3), (CONTRABAND, 2)],
        )
    }

    /// The rule set with `RelistedSale` read by `source`.
    fn relisted_by(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::RelistedSale, source)
    }

    #[test]
    fn by_the_engine_a_sale_that_unlists_the_ordered_outfit_goes_on_with_outfit_128() {
        let mut session = relisting(Rulebook::default());
        let cash = session.pilot().cash();
        let order = session.outfit_order(sell(CONTRABAND), 2, &mut NeverFires);
        assert_eq!(
            order,
            OutfitRun {
                run: OrderRun {
                    done: 3,
                    stopped: None
                },
                alert: None,
                after: Reselection::Reset(FIRST_OUTFIT)
            }
        );
        assert_eq!(session.pilot().owned(CONTRABAND), 0);
        assert_eq!(session.pilot().owned(BLASTER), 2);
        assert_eq!(session.pilot().cash() - cash, 500 + 500 + 500);
    }

    /// [`relisting_catalog`] with the outfitter's `Flags2` `flags2` and
    /// outfit 312 not selling anywhere: only a stellar of `Flags2`
    /// [`BUYS_OWNED_OUTFITS`](crate::outfitter::BUYS_OWNED_OUTFITS) lists
    /// it, owned, as it is not for sale by tech (9).
    fn buying_back_catalog(flags2: u16) -> FakePilotCatalog {
        let mut catalog = relisting_catalog();
        catalog.sites = vec![(
            SystemId(130),
            vec![LandingSite {
                flags2,
                ..outfitter_site()
            }],
        )];
        for record in &mut catalog.outfits {
            if record.id == CONTRABAND {
                record.flags = 0;
            }
        }
        catalog
    }

    #[test]
    fn a_flags2_0x0400_stellar_buys_back_an_owned_outfit_off_tech() {
        use crate::outfitter::BUYS_OWNED_OUTFITS;
        let mut session = landed_owning(
            &buying_back_catalog(BUYS_OWNED_OUTFITS),
            Rulebook::default(),
            &[(CONTRABAND, 1)],
        );
        let cash = session.pilot().cash();
        assert_eq!(session.outfit(sell(CONTRABAND), &mut NeverFires), Ok(()));
        assert_eq!(session.pilot().owned(CONTRABAND), 0);
        assert_eq!(session.pilot().cash() - cash, 500);
        let mut unflagged = landed_owning(
            &buying_back_catalog(0),
            Rulebook::default(),
            &[(CONTRABAND, 1)],
        );
        assert_eq!(
            unflagged.outfit(sell(CONTRABAND), &mut NeverFires),
            Err(OutfitRefusal::NotListed)
        );
        assert_eq!(unflagged.pilot().owned(CONTRABAND), 1);
    }

    #[test]
    fn by_the_engine_selling_the_last_flags2_0x0400_row_relists_and_goes_on_with_outfit_128() {
        use crate::outfitter::BUYS_OWNED_OUTFITS;
        let mut session = landed_owning(
            &buying_back_catalog(BUYS_OWNED_OUTFITS),
            Rulebook::default(),
            &[(BLASTER, 3), (CONTRABAND, 2)],
        );
        let cash = session.pilot().cash();
        let order = session.outfit_order(sell(CONTRABAND), 2, &mut NeverFires);
        assert_eq!(
            order,
            OutfitRun {
                run: OrderRun {
                    done: 3,
                    stopped: None
                },
                alert: None,
                after: Reselection::Reset(FIRST_OUTFIT)
            }
        );
        assert_eq!(session.pilot().owned(CONTRABAND), 0);
        assert_eq!(session.pilot().owned(BLASTER), 2);
        assert_eq!(session.pilot().cash() - cash, 500 + 500 + 500);
    }

    #[test]
    fn by_the_engine_a_counted_sale_of_all_three_reruns_its_later_passes_on_outfit_128() {
        let mut session = landed_owning(
            &relisting_catalog(),
            Rulebook::default(),
            &[(BLASTER, 3), (CONTRABAND, 3)],
        );
        let order = session.outfit_order(sell(CONTRABAND), 3, &mut NeverFires);
        assert_eq!(
            order.run,
            OrderRun {
                done: 5,
                stopped: None
            }
        );
        assert_eq!(session.pilot().owned(CONTRABAND), 0);
        assert_eq!(session.pilot().owned(BLASTER), 1);
    }

    #[test]
    fn by_the_engine_the_passes_after_a_relisting_do_nothing_without_outfit_128() {
        let mut session = landed_owning(
            &relisting_catalog(),
            Rulebook::default(),
            &[(CONTRABAND, 2)],
        );
        let mut counted = session.clone();
        let cash = session.pilot().cash();
        let order = session.outfit_order(sell(CONTRABAND), 2, &mut NeverFires);
        assert_eq!(
            order,
            OutfitRun {
                run: OrderRun {
                    done: 2,
                    stopped: Some(Stopped {
                        pass: 1,
                        refusal: OutfitRefusal::NoneOwned
                    })
                },
                alert: None,
                after: Reselection::Reset(FIRST_OUTFIT)
            }
        );
        assert_eq!(session.pilot().owned(CONTRABAND), 0);
        assert_eq!(session.pilot().cash() - cash, 500 + 500);
        assert_eq!(
            counted.outfit_counted(sell(CONTRABAND), 2, &mut NeverFires),
            Ok(2)
        );
    }

    #[test]
    fn by_the_engine_an_unlisted_outfit_128_is_sold_after_a_relisting() {
        let mut catalog = relisting_catalog();
        for record in &mut catalog.outfits {
            if record.id == BLASTER {
                record.tech_level = 9;
            }
        }
        let mut session = landed_owning(
            &catalog,
            Rulebook::default(),
            &[(BLASTER, 1), (CONTRABAND, 2)],
        );
        let outfitter = session.outfitter(&mut NeverFires).expect("open");
        assert_eq!(outfitter.row(BLASTER), None, "not listed");
        assert!(outfitter.row(SPEED).is_some(), "the booster is");
        let order = session.outfit_order(sell(CONTRABAND), 2, &mut NeverFires);
        assert_eq!(
            order.run,
            OrderRun {
                done: 3,
                stopped: None
            }
        );
        assert_eq!(session.pilot().owned(BLASTER), 0);
        assert_eq!(session.pilot().owned(CONTRABAND), 0);
    }

    #[test]
    fn by_the_engine_a_relisting_with_nothing_listed_runs_on_only_past_the_list_s_512_slots() {
        // Only the contraband is listed: the blaster is off sale (tech 9)
        // and nothing else is in the catalog.
        let catalog = FakePilotCatalog {
            outfits: vec![
                OutfitRecord {
                    tech_level: 9,
                    ..outfit(128, &[])
                },
                OutfitRecord {
                    flags: OutfitFlags::SELL_ANYWHERE,
                    tech_level: 9,
                    mass: 0,
                    ..outfit(312, &[])
                },
            ],
            ..outfitting()
        };
        let sold = |contraband, count| {
            let mut session = landed_owning(
                &catalog,
                Rulebook::default(),
                &[(BLASTER, 2), (CONTRABAND, contraband)],
            );
            let order = session.outfit_order(sell(CONTRABAND), count, &mut NeverFires);
            assert_eq!(session.pilot().owned(CONTRABAND), 0);
            (order.run, session.pilot().owned(BLASTER))
        };
        assert_eq!(
            sold(2, 2),
            (
                OrderRun {
                    done: 2,
                    stopped: None
                },
                2
            )
        );
        assert_eq!(
            sold(515, 515),
            (
                OrderRun {
                    done: 517,
                    stopped: None
                },
                0
            ),
            "passes 513 and 514 sell the blasters"
        );
    }

    #[test]
    fn by_the_engine_a_refusal_after_a_relisting_words_outfit_128_in_the_plural_while_passes_remain()
     {
        // A -10-ton blaster with 31 tons of ballast: 30 - 31 + 10 = 9
        // tons free, and the massless contraband sold leaves it 9, so the
        // blaster's sale would leave -1.
        let mut catalog = relisting_catalog();
        for record in &mut catalog.outfits {
            match record.id {
                BLASTER => record.mass = -10,
                CONTRABAND => record.mass = 0,
                _ => {}
            }
        }
        catalog.outfits.push(OutfitRecord {
            mass: 31,
            ..outfit(311, &[])
        });
        let mut session = landed_owning(
            &catalog,
            Rulebook::default(),
            &[(BLASTER, 1), (CONTRABAND, 3), (OutfitId(311), 1)],
        );
        assert_eq!(
            session.outfitter(&mut NeverFires).expect("open").free_mass,
            9
        );
        let order = session.outfit_order(sell(CONTRABAND), 3, &mut NeverFires);
        assert_eq!(
            order.run,
            OrderRun {
                done: 3,
                stopped: Some(Stopped {
                    pass: 1,
                    refusal: OutfitRefusal::NegativeFreeMass
                })
            }
        );
        let alert = order.alert.expect("an alert");
        assert_eq!(
            (alert.outfit, alert.refusal, alert.plural),
            (BLASTER, OutfitRefusal::NegativeFreeMass, true),
            "pass 1 < 3 - 1"
        );
        assert_eq!(session.pilot().owned(BLASTER), 1);
    }

    #[test]
    fn by_the_other_reading_a_sale_that_relists_keeps_the_ordered_outfit_and_its_passes() {
        let mut session = relisting(relisted_by(RuleSource::Bible));
        let order = session.outfit_order(sell(CONTRABAND), 2, &mut NeverFires);
        assert_eq!(
            order,
            OutfitRun {
                run: OrderRun {
                    done: 2,
                    stopped: None
                },
                alert: None,
                after: Reselection::Reset(FIRST_OUTFIT)
            }
        );
        assert_eq!(session.pilot().owned(CONTRABAND), 0);
        assert_eq!(session.pilot().owned(BLASTER), 3);
    }

    #[test]
    fn a_sale_that_leaves_the_listing_unchanged_is_the_same_by_both_readings() {
        let sold = |source| {
            let mut session = landed_owning(
                &relisting_catalog(),
                relisted_by(source),
                &[(BLASTER, 3), (CONTRABAND, 2), (SPEED, 3)],
            );
            let order = session.outfit_order(sell(SPEED), 3, &mut NeverFires);
            (order, session.pilot().clone())
        };
        let (order, pilot) = sold(RuleSource::Engine);
        assert_eq!(
            order.run,
            OrderRun {
                done: 3,
                stopped: None
            }
        );
        assert_eq!(pilot.owned(SPEED), 0);
        assert_eq!((order, pilot), sold(RuleSource::Bible));
    }

    #[test]
    fn a_sale_of_none_draws_no_roll_and_changes_nothing_by_either_reading() {
        // A booster of `BuyRandom` 50, not owned and not rolled yet.
        let mut catalog = relisting_catalog();
        catalog.outfits.push(OutfitRecord {
            buy_random: 50,
            ..outfit(313, &[])
        });
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let mut session = landed_owning(
                &catalog,
                relisted_by(source),
                &[(BLASTER, 3), (CONTRABAND, 2)],
            );
            let before = session.clone();
            let mut chance = Scripted::default();
            let order = session.outfit_order(sell(CONTRABAND), 0, &mut chance);
            assert_eq!(
                order.run,
                OrderRun {
                    done: 0,
                    stopped: None
                },
                "{source:?}"
            );
            assert_eq!(
                (chance.asked, chance.sides_asked),
                (vec![], vec![]),
                "{source:?}"
            );
            assert!(session == before, "{source:?}: unchanged");
        }
    }

    #[test]
    fn the_engine_reading_draws_no_roll_the_other_does_not() {
        // A booster of `BuyRandom` 50, not owned, rolled once at the
        // first build.
        let mut catalog = relisting_catalog();
        catalog.outfits.push(OutfitRecord {
            buy_random: 50,
            ..outfit(313, &[])
        });
        let drawn = |source| {
            let mut session = landed_owning(
                &catalog,
                relisted_by(source),
                &[(BLASTER, 3), (CONTRABAND, 2)],
            );
            let mut chance = Scripted::default();
            let order = session.outfit_order(sell(CONTRABAND), 2, &mut chance);
            (order.run.done, (chance.asked, chance.sides_asked))
        };
        let (done, engine) = drawn(RuleSource::Engine);
        assert_eq!(done, 3, "relisted");
        assert_eq!(engine, (vec![50], vec![]), "the first build rolls 313");
        assert_eq!(engine, drawn(RuleSource::Bible).1);
    }

    // What an order leaves the outfitter's selection at
    // (`RuleKey::RelistedSelection`).

    /// The rule set with `RelistedSelection` read by `source`.
    fn relisted_selection_by(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::RelistedSelection, source)
    }

    const LICENSE: OutfitId = OutfitId(306);
    const LICENSED: OutfitId = OutfitId(307);

    /// [`outfitting`] also selling a license (306, tech 1, contributing
    /// bit 0x2), an outfit (307, tech 1) hidden until bit 0x2 is
    /// contributed, and a blaster (128).
    fn licensing_catalog() -> FakePilotCatalog {
        let mut catalog = outfitting();
        catalog.outfits.extend([
            OutfitRecord {
                tech_level: 1,
                contribute: 0x2,
                ..outfit(306, &[])
            },
            OutfitRecord {
                flags: OutfitFlags::HIDE_UNLESS_REQUIRED,
                require: 0x2,
                require_govt: -1,
                tech_level: 1,
                ..outfit(307, &[])
            },
            outfit(128, &[]),
        ]);
        catalog
    }

    /// [`relisting_catalog`] with the blaster off sale by tech (9), so it
    /// is not listed, owned or not.
    fn unlisted_blaster_catalog() -> FakePilotCatalog {
        let mut catalog = relisting_catalog();
        for record in &mut catalog.outfits {
            if record.id == BLASTER {
                record.tech_level = 9;
            }
        }
        catalog
    }

    #[test]
    fn relisting_notes_a_change_and_keeps_an_emptying() {
        let some = [SPEED];
        let more = [SPEED, BLASTER];
        let mut unchanged = Relisting::default();
        unchanged.note(&some, &some);
        unchanged.note(&[], &[]);
        assert_eq!(unchanged.reselection(), Reselection::Kept);
        let mut changed = Relisting::default();
        changed.note(&some, &more);
        assert_eq!(changed.reselection(), Reselection::Reset(FIRST_OUTFIT));
        changed.note(&more, &more);
        assert_eq!(changed.reselection(), Reselection::Reset(FIRST_OUTFIT));
        let mut emptied = Relisting::default();
        emptied.note(&some, &[]);
        assert_eq!(emptied.reselection(), Reselection::Leave);
        emptied.note(&[], &more);
        assert_eq!(emptied.reselection(), Reselection::Leave, "refilled");
    }

    #[test]
    fn by_the_engine_a_sale_that_relists_resets_the_selection_to_outfit_128() {
        let mut session = relisting(Rulebook::default());
        let order = session.outfit_order(sell(CONTRABAND), 2, &mut NeverFires);
        assert_eq!(order.after, Reselection::Reset(FIRST_OUTFIT));
        let outfitter = session.outfitter(&mut NeverFires).expect("open");
        assert!(outfitter.rows.iter().any(|row| row.id == BLASTER), "listed");
        assert_eq!(outfitter.unlisted, None);
    }

    #[test]
    fn by_the_engine_a_buy_that_meets_a_require_resets_the_selection() {
        let mut session = outfitted(&licensing_catalog());
        let before = session.outfitter(&mut NeverFires).expect("open");
        assert_eq!(before.row(LICENSED), None, "hidden until required");
        let order = session.outfit_order(buy(LICENSE), 1, &mut NeverFires);
        assert_eq!(order.run.done, 1);
        assert_eq!(order.after, Reselection::Reset(FIRST_OUTFIT));
        let after = session.outfitter(&mut NeverFires).expect("open");
        assert!(after.rows.iter().any(|row| row.id == LICENSED), "listed");
    }

    #[test]
    fn by_the_engine_after_a_reset_the_outfitter_answers_for_an_unlisted_outfit_128() {
        let mut session = landed_owning(
            &unlisted_blaster_catalog(),
            Rulebook::default(),
            &[(BLASTER, 2), (CONTRABAND, 1)],
        );
        let order = session.outfit_order(sell(CONTRABAND), 1, &mut NeverFires);
        assert_eq!(order.after, Reselection::Reset(FIRST_OUTFIT));
        let outfitter = session.outfitter(&mut NeverFires).expect("open");
        assert!(outfitter.rows.iter().all(|row| row.id != BLASTER));
        assert_eq!(outfitter.unlisted.as_ref().map(|row| row.id), Some(BLASTER));
        assert_eq!(outfitter.can(sell(BLASTER)), Ok(()));
        let sold = session.outfit_order(sell(BLASTER), 1, &mut NeverFires);
        assert_eq!(sold.run.done, 1);
        assert_eq!(session.pilot().owned(BLASTER), 1);
    }

    #[test]
    fn by_the_engine_the_reset_outfit_128_off_today_is_bought() {
        let mut catalog = relisting_catalog();
        for record in &mut catalog.outfits {
            if record.id == BLASTER {
                record.buy_random = 50;
            }
        }
        let mut session = landed_owning(&catalog, Rulebook::default(), &[(CONTRABAND, 1)]);
        let order = session.outfit_order(sell(CONTRABAND), 1, &mut NeverFires);
        assert_eq!(order.after, Reselection::Reset(FIRST_OUTFIT));
        let outfitter = session.outfitter(&mut NeverFires).expect("open");
        assert!(
            outfitter.rows.iter().all(|row| row.id != BLASTER),
            "off today"
        );
        assert_eq!(session.outfit(buy(BLASTER), &mut NeverFires), Ok(()));
        assert_eq!(session.pilot().owned(BLASTER), 1);
        let owned = session.outfitter(&mut NeverFires).expect("open");
        assert!(
            owned.rows.iter().any(|row| row.id == BLASTER),
            "listed, owned"
        );
    }

    #[test]
    fn by_the_engine_a_sale_that_leaves_nothing_listed_leaves() {
        let catalog = FakePilotCatalog {
            outfits: vec![
                OutfitRecord {
                    tech_level: 9,
                    ..outfit(128, &[])
                },
                OutfitRecord {
                    flags: OutfitFlags::SELL_ANYWHERE,
                    tech_level: 9,
                    ..outfit(312, &[])
                },
            ],
            ..outfitting()
        };
        let mut session = landed_owning(&catalog, Rulebook::default(), &[(CONTRABAND, 2)]);
        let order = session.outfit_order(sell(CONTRABAND), 2, &mut NeverFires);
        assert_eq!(order.run.done, 2);
        assert_eq!(order.after, Reselection::Leave);
    }

    #[test]
    fn by_the_other_reading_an_order_that_relists_keeps_the_selection() {
        let mut selling = relisting(relisted_selection_by(RuleSource::Bible));
        let sold = selling.outfit_order(sell(CONTRABAND), 2, &mut NeverFires);
        assert_eq!(sold.run.done, 3, "the sale still relists");
        assert_eq!(sold.after, Reselection::Kept);
        let outfitter = selling.outfitter(&mut NeverFires).expect("open");
        assert_eq!(outfitter.unlisted, None);
        let mut buying =
            outfitted(&licensing_catalog()).with_rules(relisted_selection_by(RuleSource::Bible));
        let bought = buying.outfit_order(buy(LICENSE), 1, &mut NeverFires);
        assert_eq!(bought.run.done, 1);
        assert_eq!(bought.after, Reselection::Kept);
        let outfitter = buying.outfitter(&mut NeverFires).expect("open");
        assert!(
            outfitter.rows.iter().any(|row| row.id == LICENSED),
            "relisted"
        );
        assert_eq!(outfitter.unlisted, None);
        let mut unlisted = landed_owning(
            &unlisted_blaster_catalog(),
            relisted_selection_by(RuleSource::Bible),
            &[(BLASTER, 2), (CONTRABAND, 1)],
        );
        let order = unlisted.outfit_order(sell(CONTRABAND), 1, &mut NeverFires);
        assert_eq!(order.after, Reselection::Kept);
        let outfitter = unlisted.outfitter(&mut NeverFires).expect("open");
        assert_eq!(outfitter.unlisted, None);
    }

    #[test]
    fn the_relisted_sale_and_selection_keys_read_apart() {
        let mut kept_sale = relisting(
            relisted_by(RuleSource::Bible)
                .with_override(RuleKey::RelistedSelection, RuleSource::Engine),
        );
        let order = kept_sale.outfit_order(sell(CONTRABAND), 2, &mut NeverFires);
        assert_eq!(order.run.done, 2);
        assert_eq!(order.after, Reselection::Reset(FIRST_OUTFIT));
        assert_eq!(kept_sale.pilot().owned(BLASTER), 3);
        let mut kept_selection = relisting(
            relisted_by(RuleSource::Engine)
                .with_override(RuleKey::RelistedSelection, RuleSource::Bible),
        );
        let order = kept_selection.outfit_order(sell(CONTRABAND), 2, &mut NeverFires);
        assert_eq!(order.run.done, 3);
        assert_eq!(order.after, Reselection::Kept);
        // The kept sale's passes sell the reset outfit 128 though it is
        // not listed.
        let mut unlisted = landed_owning(
            &unlisted_blaster_catalog(),
            relisted_by(RuleSource::Bible)
                .with_override(RuleKey::RelistedSelection, RuleSource::Engine),
            &[(BLASTER, 2), (CONTRABAND, 1)],
        );
        let order = unlisted.outfit_order(sell(CONTRABAND), 1, &mut NeverFires);
        assert_eq!(order.after, Reselection::Reset(FIRST_OUTFIT));
        let sold = unlisted.outfit_order(sell(BLASTER), 1, &mut NeverFires);
        assert_eq!(sold.run.done, 1);
        assert_eq!(unlisted.pilot().owned(BLASTER), 1);
    }

    #[test]
    fn by_the_other_sale_reading_an_emptying_sale_leaves_unless_the_selection_is_kept() {
        let catalog = FakePilotCatalog {
            outfits: vec![OutfitRecord {
                flags: OutfitFlags::SELL_ANYWHERE,
                tech_level: 9,
                ..outfit(312, &[])
            }],
            ..outfitting()
        };
        let ordered = |rules| {
            let mut session = landed_owning(&catalog, rules, &[(CONTRABAND, 2)]);
            session
                .outfit_order(sell(CONTRABAND), 2, &mut NeverFires)
                .after
        };
        assert_eq!(
            ordered(relisted_by(RuleSource::Bible)),
            Reselection::Leave,
            "the kept sale's passes note the emptying"
        );
        assert_eq!(
            ordered(
                relisted_by(RuleSource::Bible)
                    .with_override(RuleKey::RelistedSelection, RuleSource::Bible)
            ),
            Reselection::Kept
        );
    }

    #[test]
    fn an_order_that_leaves_the_listing_unchanged_keeps_the_selection_by_both_readings() {
        let ordered = |source| {
            let mut session = landed_owning(
                &relisting_catalog(),
                relisted_selection_by(source),
                &[(BLASTER, 3), (CONTRABAND, 2), (SPEED, 3)],
            );
            let sold = session.outfit_order(sell(SPEED), 3, &mut NeverFires);
            let bought = session.outfit_order(buy(SPEED), 1, &mut NeverFires);
            (sold, bought, session.pilot().clone())
        };
        let (sold, bought, pilot) = ordered(RuleSource::Engine);
        assert_eq!((sold.run.done, sold.after), (3, Reselection::Kept));
        assert_eq!((bought.run.done, bought.after), (1, Reselection::Kept));
        assert_eq!(pilot.owned(SPEED), 1);
        assert_eq!((sold, bought, pilot), ordered(RuleSource::Bible));
    }

    #[test]
    fn an_order_of_another_outfit_or_a_new_opening_lets_the_reset_go() {
        let mut session = landed_owning(
            &unlisted_blaster_catalog(),
            Rulebook::default(),
            &[(BLASTER, 2), (CONTRABAND, 1), (SPEED, 1)],
        );
        let order = session.outfit_order(sell(CONTRABAND), 1, &mut NeverFires);
        assert_eq!(order.after, Reselection::Reset(FIRST_OUTFIT));
        let unlisted = |session: &mut Session| {
            session
                .outfitter(&mut NeverFires)
                .expect("open")
                .unlisted
                .map(|row| row.id)
        };
        assert_eq!(unlisted(&mut session), Some(BLASTER));
        let mut reopened = session.clone();
        reopened.open_outfitter();
        assert_eq!(unlisted(&mut reopened), None, "a new opening");
        let mut kept = session.clone();
        let again = kept.outfit_order(sell(BLASTER), 1, &mut NeverFires);
        assert_eq!(again.after, Reselection::Kept);
        assert_eq!(unlisted(&mut kept), Some(BLASTER), "still selected");
        assert_eq!(session.outfit(sell(SPEED), &mut NeverFires), Ok(()));
        assert_eq!(unlisted(&mut session), None, "another outfit");
    }

    #[test]
    fn building_for_the_selection_draws_no_extra_roll() {
        // A booster of `BuyRandom` 50, not owned, rolled once at the
        // first build.
        let mut catalog = relisting_catalog();
        catalog.outfits.push(OutfitRecord {
            buy_random: 50,
            ..outfit(313, &[])
        });
        let drawn = |source, order: OutfitOrder, count| {
            let mut session = landed_owning(
                &catalog,
                relisted_by(RuleSource::Bible).with_override(RuleKey::RelistedSelection, source),
                &[(BLASTER, 3), (CONTRABAND, 2)],
            );
            let mut chance = Scripted::default();
            let run = session.outfit_order(order, count, &mut chance);
            (run.run.done, (chance.asked, chance.sides_asked))
        };
        for (order, count) in [(sell(CONTRABAND), 2), (buy(SPEED), 2)] {
            let (done, engine) = drawn(RuleSource::Engine, order, count);
            assert_eq!(done, 2, "{order:?}");
            assert_eq!(engine, (vec![50], vec![]), "{order:?}: the first build");
            assert_eq!(
                (done, engine),
                drawn(RuleSource::Bible, order, count),
                "{order:?}"
            );
        }
    }

    // The visit.

    /// `session` after a booster and a map bought: the map flag set, and
    /// a booster more than the visit opened with.
    fn dirty_the_visit(session: &mut Session) {
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        session.outfit(buy(MAP), &mut NeverFires).expect("bought");
        assert_ne!(
            session.outfit_visit,
            OutfitVisit::open(&session.pilot.outfits)
        );
    }

    /// Whether `session`'s visit is a new one, opened over its outfits.
    fn opened_anew(session: &Session) -> bool {
        session.outfit_visit == OutfitVisit::open(&session.pilot.outfits)
    }

    #[test]
    fn opening_the_outfitter_opens_a_new_visit() {
        let mut session = outfitted(&limiting());
        dirty_the_visit(&mut session);
        session.open_outfitter();
        assert!(opened_anew(&session));
    }

    #[test]
    fn landing_opens_a_new_visit() {
        let mut session = outfitted(&limiting());
        dirty_the_visit(&mut session);
        session
            .take_off(&NoTraffic, &mut NeverFires)
            .expect("takes off");
        land_now(&mut session).expect("lands");
        assert!(opened_anew(&session));
    }

    #[test]
    fn a_session_starts_with_a_visit_opened() {
        let catalog = limiting();
        let mut session = outfitted(&catalog);
        dirty_the_visit(&mut session);
        let resumed = Session::fly(&catalog, session.pilot().clone()).expect("flies");
        assert_eq!(resumed.pilot().owned(SPEED), 1);
        assert!(opened_anew(&resumed));
    }

    #[test]
    fn a_sale_leaves_the_visit_as_it_is() {
        let mut session = outfitted(&limiting());
        dirty_the_visit(&mut session);
        let visit = session.outfit_visit.clone();
        session.outfit(sell(SPEED), &mut NeverFires).expect("sold");
        assert_eq!(session.outfit_visit, visit);
    }

    #[test]
    fn the_session_hands_its_visit_to_the_outfitter() {
        let mut session = outfitted(&limiting());
        session.outfit(buy(MAP), &mut NeverFires).expect("bought");
        let outfitter = session.outfitter(&mut NeverFires).expect("open");
        assert_eq!(outfitter.visit, session.outfit_visit);
    }

    // Refunds.

    #[test]
    fn an_outfit_bought_this_opening_sells_back_in_full() {
        let mut session = outfitted(&outfitting());
        session.open_outfitter();
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        session.outfit(sell(SPEED), &mut NeverFires).expect("sold");
        assert_eq!(session.pilot().cash(), 25_000);
    }

    #[test]
    fn an_outfit_owned_before_the_opening_sells_back_at_half() {
        let mut session = outfitted(&outfitting());
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        session.open_outfitter();
        session.outfit(sell(SPEED), &mut NeverFires).expect("sold");
        assert_eq!(session.pilot().cash(), 24_000 + 500);
    }

    #[test]
    fn a_counted_sale_refunds_in_full_only_above_the_opening_count() {
        let mut session = outfitted(&outfitting());
        session
            .outfit_counted(buy(SPEED), 2, &mut NeverFires)
            .expect("bought");
        session.open_outfitter();
        session
            .outfit_counted(buy(SPEED), 2, &mut NeverFires)
            .expect("bought");
        assert_eq!(session.pilot().cash(), 21_000);
        assert_eq!(
            session.outfit_counted(sell(SPEED), 3, &mut NeverFires),
            Ok(3)
        );
        assert_eq!(session.pilot().cash(), 21_000 + 2 * 1000 + 500);
        assert_eq!(session.pilot().owned(SPEED), 1);
    }

    #[test]
    fn reopening_the_outfitter_takes_a_new_copy() {
        let mut session = outfitted(&outfitting());
        session.open_outfitter();
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        session.open_outfitter();
        session.outfit(sell(SPEED), &mut NeverFires).expect("sold");
        assert_eq!(session.pilot().cash(), 24_000 + 500);
    }

    #[test]
    fn landing_takes_the_copy() {
        let mut session = outfitted(&outfitting());
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        session.take_off(&NoTraffic, &mut NeverFires);
        land_now(&mut session).expect("lands");
        session.outfit(sell(SPEED), &mut NeverFires).expect("sold");
        assert_eq!(session.pilot().cash(), 24_000 + 500);
    }

    #[test]
    fn a_session_resumed_docked_takes_the_copy() {
        let catalog = outfitting();
        let mut session = outfitted(&catalog);
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        let pilot = session.pilot().clone();
        let mut resumed = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(resumed.landed(), Some(StellarId(128)));
        resumed.outfit(sell(SPEED), &mut NeverFires).expect("sold");
        assert_eq!(resumed.pilot().cash(), 24_000 + 500, "owned on resuming");
    }

    #[test]
    fn the_bible_reading_refunds_half_whenever_bought() {
        let mut session = outfitted(&outfitting()).with_rules(
            Rulebook::default().with_override(RuleKey::OutfitRefund, RuleSource::Bible),
        );
        assert_eq!(
            session.rules().source_for(RuleKey::OutfitRefund),
            RuleSource::Bible
        );
        session.open_outfitter();
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        session.outfit(sell(SPEED), &mut NeverFires).expect("sold");
        assert_eq!(session.pilot().cash(), 24_000 + 500);
        let session = session.with_rules(
            Rulebook::default().with_override(RuleKey::OutfitRefund, RuleSource::Engine),
        );
        assert_eq!(
            session.rules().source_for(RuleKey::OutfitRefund),
            RuleSource::Engine
        );
    }

    #[test]
    fn the_engine_reading_is_the_default() {
        let session = outfitted(&outfitting());
        assert_eq!(
            session.rules().source_for(RuleKey::OutfitRefund),
            RuleSource::Engine
        );
    }

    #[test]
    fn the_outfit_count_rule_reaches_the_outfitter() {
        for source in [None, Some(RuleSource::Engine), Some(RuleSource::Bible)] {
            let session = outfitted(&outfitting());
            let mut session = match source {
                Some(source) => session
                    .with_rules(Rulebook::default().with_override(RuleKey::OutfitCount, source)),
                None => session,
            };
            let source = source.unwrap_or_default();
            assert_eq!(session.rules().source_for(RuleKey::OutfitCount), source);
            assert_eq!(
                session.rules().source_for(RuleKey::OutfitRefund),
                RuleSource::Engine
            );
            let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
            assert_eq!(
                outfitter.rules.source_for(RuleKey::OutfitCount),
                source,
                "{source:?}"
            );
        }
    }

    /// A session landed at the outfitter over its mass limit: of its 30
    /// tons free, a 50-ton outfit (311) and four 5-ton boosters take 70
    /// and three pods (310) give back 30, so -10 tons are free.
    fn over_the_mass_limit(source: RuleSource) -> Session {
        over_the_mass_limit_by(Rulebook::default().with_override(RuleKey::SaleMass, source))
    }

    /// [`over_the_mass_limit`] by `rules`.
    fn over_the_mass_limit_by(rules: Rulebook) -> Session {
        let mut catalog = outfitting();
        catalog.outfits[0].mass = 5;
        catalog.outfits.push(OutfitRecord {
            mass: -10,
            ..outfit(310, &[])
        });
        catalog.outfits.push(OutfitRecord {
            mass: 50,
            ..outfit(311, &[])
        });
        let mut session = outfitted(&catalog).with_rules(rules);
        session.pilot.outfits =
            BTreeMap::from([(OutfitId(311), 1), (SPEED, 4), (OutfitId(310), 3)]);
        let outfitter = session.outfitter(&mut NeverFires).expect("open");
        assert_eq!(outfitter.free_mass, -10);
        session
    }

    #[test]
    fn by_the_engine_a_counted_sale_of_mass_0_or_more_sells_in_full_below_0_free() {
        let mut session = over_the_mass_limit(RuleSource::Engine);
        assert_eq!(
            session.outfit_counted(sell(SPEED), 4, &mut NeverFires),
            Ok(4),
            "-10 free, each 5-ton booster sold regardless"
        );
        // Now 10 free: one pod leaves 0, and the next would leave -10.
        let pod = OutfitId(310);
        assert_eq!(session.outfit_counted(sell(pod), 3, &mut NeverFires), Ok(1));
        assert_eq!(session.pilot().owned(pod), 2);
    }

    #[test]
    fn by_the_engine_a_pod_is_refused_while_the_free_mass_is_below_0() {
        let mut session = over_the_mass_limit(RuleSource::Engine);
        assert_eq!(
            session.outfit_counted(sell(OutfitId(310)), 3, &mut NeverFires),
            Err(OutfitRefusal::NegativeFreeMass),
            "max(-10, 0) - 10 < 0"
        );
    }

    #[test]
    fn by_the_other_reading_a_counted_sale_below_0_free_is_refused_at_once() {
        let mut session = over_the_mass_limit(RuleSource::Bible);
        let before = session.pilot().clone();
        assert_eq!(
            session.outfit_counted(sell(SPEED), 4, &mut NeverFires),
            Err(OutfitRefusal::NegativeFreeMass),
            "-10 + 5 < 0"
        );
        assert_eq!(session.pilot(), &before);
    }

    #[test]
    fn a_counted_order_refused_at_once_changes_nothing() {
        let mut session = outfitted(&outfitting());
        session.pilot.cash = 500;
        let before = session.pilot().clone();
        assert_eq!(
            session.outfit_counted(buy(SPEED), 3, &mut NeverFires),
            Err(OutfitRefusal::CannotAfford)
        );
        assert_eq!(
            session.outfit_counted(sell(SPEED), 3, &mut NeverFires),
            Err(OutfitRefusal::NoneOwned)
        );
        assert_eq!(session.pilot(), &before);
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_count_of_none_buys_nothing_and_makes_no_save_due() {
        let mut session = outfitted(&outfitting());
        let before = session.pilot().clone();
        assert_eq!(
            session.outfit_counted(buy(SPEED), 0, &mut NeverFires),
            Ok(0)
        );
        assert_eq!(session.pilot(), &before);
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_single_order_and_a_counted_order_of_one_take_the_same_loop() {
        let mut mapped = outfitted(&limiting());
        mapped.outfit(buy(MAP), &mut NeverFires).expect("bought");
        let mut owning = outfitted(&outfitting());
        owning.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        let cases = vec![
            (outfitted(&outfitting()), buy(SPEED), Ok(1)),
            (
                mapped,
                buy(OTHER_MAP),
                Err(OutfitRefusal::BoughtThisOpening),
            ),
            (owning, sell(SPEED), Ok(1)),
        ];
        for (session, order, expected) in cases {
            let (mut single, mut counted, mut run) = (session.clone(), session.clone(), session);
            let one = single.outfit(order, &mut NeverFires);
            assert_eq!(one.map(|()| 1), expected, "{order:?}");
            assert_eq!(
                counted.outfit_counted(order, 1, &mut NeverFires),
                expected,
                "{order:?}"
            );
            assert_eq!(
                run.outfit_run(order, 1, &mut NeverFires).counted(),
                expected,
                "{order:?}"
            );
            assert!(single == counted && counted == run, "{order:?}");
        }
    }

    #[test]
    fn a_counted_order_refused_part_way_keeps_its_refusal_in_the_run() {
        let mut session = outfitted(&outfitting());
        let mut counted = session.clone();
        let run = session.outfit_run(buy(SPEED), 12, &mut NeverFires);
        let refusal = session
            .outfitter(&mut NeverFires)
            .expect("open")
            .can(buy(SPEED))
            .expect_err("refused at the max");
        assert_eq!(
            run,
            OrderRun {
                done: 10,
                stopped: Some(Stopped { pass: 10, refusal }),
            }
        );
        assert_eq!(
            counted.outfit_counted(buy(SPEED), 12, &mut NeverFires),
            Ok(10)
        );
    }

    const POD: OutfitId = OutfitId(310);

    #[test]
    fn a_plain_sale_refused_in_the_loop_carries_its_alert_and_sells_nothing() {
        let mut session = over_the_mass_limit(RuleSource::Engine);
        session.take_save_due();
        let before = session.pilot().clone();
        let order = session.outfit_order(sell(POD), 1, &mut NeverFires);
        assert_eq!(
            order.run,
            OrderRun {
                done: 0,
                stopped: Some(Stopped {
                    pass: 0,
                    refusal: OutfitRefusal::NegativeFreeMass,
                }),
            }
        );
        let alert = order.alert.expect("an alert");
        assert_eq!(alert.outfit, POD);
        assert_eq!(alert.refusal, OutfitRefusal::NegativeFreeMass);
        assert!(!alert.plural, "a plain click names it in the singular");
        assert_eq!(session.pilot(), &before);
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_counted_sale_refused_at_its_first_pass_carries_a_plural_alert() {
        let mut session = over_the_mass_limit(RuleSource::Engine);
        let before = session.pilot().clone();
        let order = session.outfit_order(sell(POD), 3, &mut NeverFires);
        assert_eq!(order.run.done, 0);
        let alert = order.alert.expect("an alert");
        assert_eq!(alert.refusal, OutfitRefusal::NegativeFreeMass);
        assert!(alert.plural, "passes remain");
        assert_eq!(session.pilot(), &before);
    }

    /// [`over_the_mass_limit_by`] with its four boosters sold: ten tons
    /// free, so one pod leaves none and the next is refused.
    fn ten_tons_free_by(rules: Rulebook) -> Session {
        let mut session = over_the_mass_limit_by(rules);
        session
            .outfit_counted(sell(SPEED), 4, &mut NeverFires)
            .expect("sold");
        session.take_save_due();
        session
    }

    fn part_way_refused() -> OrderRun<OutfitRefusal> {
        OrderRun {
            done: 1,
            stopped: Some(Stopped {
                pass: 1,
                refusal: OutfitRefusal::NegativeFreeMass,
            }),
        }
    }

    #[test]
    fn a_counted_sale_refused_part_way_carries_its_alert_and_keeps_its_sales() {
        let mut session = ten_tons_free_by(Rulebook::default());
        let cash = session.pilot().cash();
        let refund = session
            .outfitter(&mut NeverFires)
            .expect("open")
            .refund(POD)
            .expect("a refund");
        let order = session.outfit_order(sell(POD), 3, &mut NeverFires);
        assert_eq!(order.run, part_way_refused());
        let alert = order.alert.expect("an alert");
        assert_eq!(alert.outfit, POD);
        assert_eq!(alert.refusal, OutfitRefusal::NegativeFreeMass);
        assert!(alert.plural, "refused at pass 1 of 3: a pass remains");
        assert_eq!(session.pilot().owned(POD), 2);
        assert_eq!(session.pilot().cash(), cash + refund);
        assert!(session.take_save_due());
    }

    #[test]
    fn a_counted_sale_refused_at_its_last_pass_names_the_outfit_singular() {
        let mut session = ten_tons_free_by(Rulebook::default());
        let order = session.outfit_order(sell(POD), 2, &mut NeverFires);
        assert_eq!(order.run, part_way_refused());
        let alert = order.alert.expect("an alert");
        assert_eq!(alert.refusal, OutfitRefusal::NegativeFreeMass);
        assert!(!alert.plural, "no pass remains after pass 1 of 2");
    }

    #[test]
    fn by_the_other_reading_a_part_way_sale_keeps_its_sales_and_carries_no_alert() {
        let mut session = ten_tons_free_by(
            Rulebook::default().with_override(RuleKey::SaleRefusal, RuleSource::Bible),
        );
        let order = session.outfit_order(sell(POD), 3, &mut NeverFires);
        assert_eq!(order.run, part_way_refused());
        assert_eq!(order.alert, None);
        assert_eq!(session.pilot().owned(POD), 2);
    }

    #[test]
    fn a_counted_buy_refused_part_way_keeps_todays_result() {
        let mut session = outfitted(&outfitting());
        let mut counted = session.clone();
        let order = session.outfit_order(buy(SPEED), 12, &mut NeverFires);
        let refusal = session
            .outfitter(&mut NeverFires)
            .expect("open")
            .can(buy(SPEED))
            .expect_err("refused at the max");
        assert_eq!(
            order,
            OutfitRun {
                run: OrderRun {
                    done: 10,
                    stopped: Some(Stopped { pass: 10, refusal }),
                },
                alert: None,
                after: Reselection::Kept,
            }
        );
        assert_eq!(session.pilot().owned(SPEED), 10);
        assert_eq!(session.pilot().cash(), 15_000);
        assert_eq!(
            counted.outfit_counted(buy(SPEED), 12, &mut NeverFires),
            Ok(10)
        );
    }

    #[test]
    fn by_the_other_reading_an_order_carries_no_alert() {
        let mut session = over_the_mass_limit_by(
            Rulebook::default().with_override(RuleKey::SaleRefusal, RuleSource::Bible),
        );
        for count in [1, 3] {
            let order = session.outfit_order(sell(POD), count, &mut NeverFires);
            assert_eq!(
                order.run.stopped.map(|stopped| stopped.refusal),
                Some(OutfitRefusal::NegativeFreeMass)
            );
            assert_eq!(order.alert, None, "{count}");
        }
    }

    #[test]
    fn outfit_and_outfit_counted_agree_with_outfit_order() {
        let mut owning = outfitted(&outfitting());
        owning
            .outfit_counted(buy(SPEED), 3, &mut NeverFires)
            .expect("bought");
        let cases = vec![
            (over_the_mass_limit(RuleSource::Engine), sell(POD), 3),
            (over_the_mass_limit(RuleSource::Engine), sell(SPEED), 2),
            (outfitted(&outfitting()), buy(SPEED), 12),
            (owning, sell(SPEED), 5),
        ];
        for (session, order, count) in cases {
            let (mut single, mut by_order) = (session.clone(), session.clone());
            assert_eq!(
                single.outfit(order, &mut NeverFires),
                by_order
                    .outfit_order(order, 1, &mut NeverFires)
                    .run
                    .counted()
                    .map(|_| ()),
                "{order:?}"
            );
            assert!(single == by_order, "{order:?}");
            let (mut counted, mut by_order) = (session.clone(), session);
            assert_eq!(
                counted.outfit_counted(order, count, &mut NeverFires),
                by_order
                    .outfit_order(order, count, &mut NeverFires)
                    .run
                    .counted(),
                "{order:?}"
            );
            assert!(counted == by_order, "{order:?}");
        }
    }

    const MAP: OutfitId = OutfitId(306);
    const OTHER_MAP: OutfitId = OutfitId(307);
    const RECORD: OutfitId = OutfitId(308);
    const OTHER_RECORD: OutfitId = OutfitId(309);

    /// [`outfitting`] also selling two maps (306, 307) and two clean
    /// records (308, 309).
    fn limiting() -> FakePilotCatalog {
        let mut catalog = outfitting();
        catalog.outfits.extend([
            outfit(306, &[(outfitter::EXPLORES_MAP, 5)]),
            outfit(307, &[(outfitter::EXPLORES_MAP, 7)]),
            outfit(308, &[(outfitter::CLEAN_RECORD, 128)]),
            outfit(309, &[(outfitter::CLEAN_RECORD, 129)]),
        ]);
        catalog
    }

    /// For each of the map pair and the record pair, a session that has
    /// just bought the first of the pair, with the pair.
    fn each_limited(source: RuleSource) -> Vec<(Session, OutfitId, OutfitId)> {
        [(MAP, OTHER_MAP), (RECORD, OTHER_RECORD)]
            .into_iter()
            .map(|(first, other)| {
                let mut session = outfitted(&limiting())
                    .with_rules(Rulebook::default().with_override(RuleKey::OutfitLimit, source));
                session.outfit(buy(first), &mut NeverFires).expect("bought");
                (session, first, other)
            })
            .collect()
    }

    #[test]
    fn a_map_or_clean_record_is_refused_again_until_another_outfit_is_bought() {
        for (mut session, first, other) in each_limited(RuleSource::Engine) {
            let (cash, owned) = (session.pilot().cash(), session.pilot().owned(first));
            session.take_save_due();
            for again in [first, other] {
                assert_eq!(
                    session.outfit(buy(again), &mut NeverFires),
                    Err(OutfitRefusal::BoughtThisOpening),
                    "{again:?} after {first:?}"
                );
            }
            assert_eq!(session.pilot().cash(), cash);
            assert_eq!(session.pilot().owned(first), owned);
            assert!(!session.take_save_due(), "a refusal changes nothing");
            session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
            assert_eq!(session.outfit(buy(first), &mut NeverFires), Ok(()));
        }
    }

    #[test]
    fn buying_a_map_raises_explored_for_each_system_it_reaches() {
        let mut session = outfitted(&limiting());
        session.drop_raised();
        session.outfit(buy(MAP), &mut NeverFires).expect("bought");
        assert_eq!(
            session.story_raised(),
            [130, 131, 132].map(|system| StoryEvent::Explored {
                system: SystemId(system)
            })
        );
    }

    #[test]
    fn a_map_and_a_clean_record_do_not_limit_each_other() {
        let mut session = outfitted(&limiting());
        session.outfit(buy(MAP), &mut NeverFires).expect("bought");
        assert_eq!(session.outfit(buy(RECORD), &mut NeverFires), Ok(()));
        assert_eq!(session.outfit(buy(MAP), &mut NeverFires), Ok(()), "lifted");
    }

    #[test]
    fn opening_the_outfitter_again_lifts_the_limit() {
        for (mut session, first, _) in each_limited(RuleSource::Engine) {
            session.open_outfitter();
            assert_eq!(session.outfit(buy(first), &mut NeverFires), Ok(()));
        }
    }

    #[test]
    fn the_next_landing_lifts_the_limit() {
        for (mut session, first, _) in each_limited(RuleSource::Engine) {
            relanded(&mut session);
            assert_eq!(session.outfit(buy(first), &mut NeverFires), Ok(()));
        }
    }

    #[test]
    fn a_sale_leaves_the_limit_as_it_is() {
        // A plain outfit is sold, not the limited one: were a sale to
        // reset the flags, selling the map or record would set them again.
        for first in [MAP, RECORD] {
            let mut session = outfitted(&limiting());
            session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
            session.outfit(buy(first), &mut NeverFires).expect("bought");
            session.outfit(sell(SPEED), &mut NeverFires).expect("sold");
            assert_eq!(
                session.outfit(buy(first), &mut NeverFires),
                Err(OutfitRefusal::BoughtThisOpening),
                "{first:?} after selling {SPEED:?}"
            );
        }
    }

    #[test]
    fn the_outfitter_greys_the_buy_but_keeps_the_words() {
        for (mut session, first, _) in each_limited(RuleSource::Engine) {
            let outfitter = session.outfitter(&mut NeverFires).expect("open");
            assert!(outfitter.row(first).is_some(), "listed");
            assert_eq!(
                outfitter.can(buy(first)),
                Err(OutfitRefusal::BoughtThisOpening)
            );
            assert_eq!(outfitter.words(first), None);
        }
    }

    #[test]
    fn by_the_other_reading_nothing_is_limited() {
        assert_eq!(
            outfitted(&limiting())
                .rules()
                .source_for(RuleKey::OutfitLimit),
            RuleSource::Engine,
            "the engine's by default"
        );
        for (mut session, first, other) in each_limited(RuleSource::Bible) {
            assert_eq!(
                session.rules().source_for(RuleKey::OutfitLimit),
                RuleSource::Bible
            );
            assert_eq!(session.outfit(buy(first), &mut NeverFires), Ok(()));
            assert_eq!(session.outfit(buy(other), &mut NeverFires), Ok(()));
        }
        // The flags kept under the other reading apply once it changes.
        let mut session = outfitted(&limiting())
            .with_rules(Rulebook::default().with_override(RuleKey::OutfitLimit, RuleSource::Bible));
        session.outfit(buy(MAP), &mut NeverFires).expect("bought");
        let mut session = session.with_rules(
            Rulebook::default().with_override(RuleKey::OutfitLimit, RuleSource::Engine),
        );
        assert_eq!(
            session.outfit(buy(MAP), &mut NeverFires),
            Err(OutfitRefusal::BoughtThisOpening)
        );
    }

    /// The outfitting catalog with outfit 305 sold only on control bit 7,
    /// and hidden while it is refused.
    pub(super) fn outfitting_on_bit_7() -> FakePilotCatalog {
        let mut catalog = outfitting();
        catalog.outfits.push(OutfitRecord {
            flags: crate::outfitter::OutfitFlags::HIDE_UNLESS_AVAILABLE,
            availability: crate::control::Test::parse("b7"),
            ..outfit(305, &[])
        });
        catalog
    }

    #[test]
    fn an_outfit_is_listed_once_the_bit_its_availability_tests_is_set() {
        let gated = OutfitId(305);
        let mut session = outfitted(&outfitting_on_bit_7());
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        assert_eq!(outfitter.row(gated), None);
        session.set_control_bit(crate::control::Bit::new(7).expect("a bit"), true);
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        assert_eq!(
            outfitter.row(gated).map(|_| outfitter.can(buy(gated))),
            Some(Ok(()))
        );
        assert_eq!(session.outfit(buy(gated), &mut NeverFires), Ok(()));
    }

    #[test]
    fn the_outfitter_tests_availability_through_the_sessions_control_bits() {
        let gated = OutfitId(305);
        let mut session = outfitted(&outfitting_on_bit_7())
            .with_control_bits(Rc::new(crate::testkit::RefuseBits(&[7])));
        session.set_control_bit(crate::control::Bit::new(7).expect("a bit"), true);
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        assert_eq!(outfitter.row(gated), None);
        assert_eq!(
            session.outfit(buy(gated), &mut NeverFires),
            Err(OutfitRefusal::NotListed)
        );
    }

    const EXPANSION: OutfitId = OutfitId(305);

    /// [`outfitting`] also selling a mass expansion of 12 tons (outfit
    /// 305, massless), with ship class 129 a trader of 15 tons.
    fn expanding() -> FakePilotCatalog {
        let mut catalog = outfitting();
        catalog.outfits.push(OutfitRecord {
            mass: 0,
            ..outfit(305, &[(crate::market::MORE_CARGO, -12)])
        });
        catalog.ship_records.push(holds_record(129, 15, 1));
        catalog
    }

    /// Whether the outfitter would sell `session` a mass expansion.
    fn expansion_buy(session: &mut Session) -> Result<(), OutfitRefusal> {
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        outfitter.check(buy(EXPANSION))
    }

    #[test]
    fn a_mass_expansion_needs_the_ships_own_free_cargo() {
        // `_CanBuyOutfitItem` @0x4e980-0x4e9a8: `_ShipTotalHolds`, not
        // `_TotalFleetHolds`, less `_TotalCargo` (commodities and jünk).
        let mut session = outfitted(&expanding());
        session.pilot.escorts = vec![fleet_escort(129)];
        assert_eq!(session.capacity(), 35, "the fleet's space");
        session.pilot.cargo.insert(FOOD, 8);
        assert_eq!(expansion_buy(&mut session), Ok(()), "exactly 12 free");
        session.pilot.cargo.insert(FOOD, 9);
        assert_eq!(
            expansion_buy(&mut session),
            Err(OutfitRefusal::NoCargoRoom),
            "a commodity; the escort's holds add nothing"
        );
        session.pilot.cargo.insert(FOOD, 8);
        session.pilot.cargo.insert(Good::Junk(JunkId(146)), 1);
        assert_eq!(
            expansion_buy(&mut session),
            Err(OutfitRefusal::NoCargoRoom),
            "a jünk"
        );
        session.pilot.cargo.remove(&Good::Junk(JunkId(146)));
        session.pilot.cargo.insert(FOOD, 9);
        session
            .trade(order(&session, FOOD, Direction::Sell, Lot::Click))
            .expect("sold");
        assert_eq!(
            session.outfit(buy(EXPANSION), &mut NeverFires),
            Ok(()),
            "room once a ton is sold"
        );
        assert_eq!(session.pilot().owned(EXPANSION), 1);
    }

    #[test]
    fn a_mass_expansion_counts_the_mission_cargo_aboard() {
        let mut session = outfitted(&expanding());
        session.pilot.cargo.insert(FOOD, 7);
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 1));
        assert_eq!(expansion_buy(&mut session), Ok(()), "exactly 12 free");
        session
            .pilot
            .missions
            .put(1, crate::testkit::carrying(141, 1));
        assert_eq!(
            expansion_buy(&mut session),
            Err(OutfitRefusal::NoCargoRoom),
            "a ton of a mission's cargo"
        );
    }

    /// The outfitting catalog with a map of 1 jump (306), an outfit
    /// cleaning the record with government 140 (307), a paint (308), a
    /// plain outfit removed after purchase (309) and a map removed after
    /// purchase (310) for sale too.
    pub(super) fn outfitting_effects() -> FakePilotCatalog {
        use crate::outfit_effects::{CLEAN_RECORD, MAP, PAINT};
        let mut catalog = outfitting();
        catalog.outfits.extend([
            outfit(306, &[(MAP, 1)]),
            outfit(307, &[(CLEAN_RECORD, 140)]),
            outfit(308, &[(PAINT, 0x7C00)]),
            OutfitRecord {
                flags: OutfitFlags::REMOVE_AFTER_PURCHASE,
                ..outfit(309, &[(MORE_SHIELD, 50)])
            },
            OutfitRecord {
                flags: OutfitFlags::REMOVE_AFTER_PURCHASE,
                ..outfit(310, &[(MAP, 1)])
            },
        ]);
        catalog
    }

    #[test]
    fn buying_a_map_pays_and_explores_and_adds_nothing() {
        let mut session = outfitted(&outfitting_effects());
        assert_eq!(session.outfit(buy(OutfitId(306)), &mut NeverFires), Ok(()));
        assert_eq!(session.pilot().cash(), 24_000);
        assert_eq!(
            session.pilot().explored().collect::<Vec<_>>(),
            [SystemId(130), SystemId(131)]
        );
        assert_eq!(session.pilot().owned(OutfitId(306)), 0);
        assert!(session.take_save_due());
    }

    #[test]
    fn buying_a_clean_record_outfit_clears_the_record_and_a_paint_paints() {
        let mut session = outfitted(&outfitting_effects());
        session.pilot.legal.insert(GovtId(140), -300);
        session
            .outfit(buy(OutfitId(307)), &mut NeverFires)
            .expect("bought");
        assert_eq!(session.pilot().legal_record(GovtId(140)), 0);
        assert_eq!(session.pilot().owned(OutfitId(307)), 0);
        session
            .outfit(buy(OutfitId(308)), &mut NeverFires)
            .expect("bought");
        assert_eq!(
            session.pilot().paint(),
            Some(crate::outfit_effects::Rgb15 { r: 31, g: 0, b: 0 })
        );
        assert_eq!(session.pilot().owned(OutfitId(308)), 0);
        assert_eq!(session.pilot().cash(), 23_000);
    }

    #[test]
    fn an_outfit_removed_after_purchase_still_acts_but_is_not_kept() {
        let mut session = outfitted(&outfitting_effects());
        let shield = session.stats().shield;
        session
            .outfit(buy(OutfitId(309)), &mut NeverFires)
            .expect("bought");
        assert_eq!(session.pilot().owned(OutfitId(309)), 0);
        assert_eq!(session.stats().shield, shield, "not kept");
        assert_eq!(session.pilot().cash(), 24_000, "paid for");
        session
            .outfit(buy(OutfitId(310)), &mut NeverFires)
            .expect("bought");
        assert!(session.pilot().has_explored(SystemId(131)), "it explores");
        assert_eq!(session.pilot().owned(OutfitId(310)), 0);
    }

    #[test]
    fn the_outfitter_holds_a_surplus_shield_and_armour_by_either_reading() {
        // `_DoOutfitDialog` clamps the reserves when it closes
        // (@0x5da4b-0x5dad4), whatever `refit_reserves` says of `G`, `D`
        // and boarding.
        for source in [RuleSource::Engine, RuleSource::Bible] {
            for order in [buy(OutfitId(302)), sell(OutfitId(302))] {
                let mut session = outfitted(&outfitting())
                    .with_rules(Rulebook::default().with_override(RuleKey::RefitReserves, source));
                if order.direction == Direction::Sell {
                    session.pilot.outfits.insert(OutfitId(302), 1);
                    session.refit(false, ReservePolicy::Hold);
                }
                let (shield, armor) = (session.stats().shield, session.stats().armor);
                session.pilot.reserves.shield.now = shield + 5.0;
                session.pilot.reserves.armor.now = armor + 5.0;
                session.outfit(order, &mut NeverFires).expect("traded");
                let (shield, armor) = (session.stats().shield, session.stats().armor);
                let reserves = session.reserves();
                assert_eq!(
                    reserves.shield,
                    Gauge::full(shield),
                    "{source:?}, {order:?}"
                );
                assert_eq!(reserves.armor, Gauge::full(armor), "{source:?}, {order:?}");
            }
        }
    }

    #[test]
    fn an_owned_mass_expansion_lowers_the_free_cargo_the_next_needs() {
        let mut session = outfitted(&expanding());
        assert_eq!(session.outfit(buy(EXPANSION), &mut NeverFires), Ok(()));
        assert_eq!(session.capacity(), 8);
        assert_eq!(
            session.outfit(buy(EXPANSION), &mut NeverFires),
            Err(OutfitRefusal::NoCargoRoom),
            "8 tons left, 12 needed"
        );
    }

    #[test]
    fn selling_space_below_the_cargo_held_is_allowed_and_trims_it_by_the_engine() {
        for (source, food) in [(RuleSource::Engine, 20), (RuleSource::Bible, 30)] {
            let mut session = outfitted(&outfitting())
                .with_rules(Rulebook::default().with_override(RuleKey::OutfitSaleCargo, source));
            session.outfit(buy(CARGO), &mut NeverFires).expect("bought");
            let bought = order(&session, FOOD, Direction::Buy, Lot::Max);
            assert_eq!(session.trade(bought), Ok(30));
            assert_eq!(session.outfit(sell(CARGO), &mut NeverFires), Ok(()));
            let market = session.market().expect("an exchange");
            assert_eq!((market.capacity, market.free), (20, 0), "{source:?}");
            assert_eq!(session.pilot().held(FOOD), food, "{source:?}");
        }
    }

    #[test]
    fn a_shield_or_tank_comes_full_and_selling_it_keeps_no_more_than_fits() {
        let mut session = outfitted(&outfitting());
        session.pilot.reserves.fuel.now = 200.0;
        session.pilot.reserves.shield.now = 10.0;
        session.outfit(buy(TANK), &mut NeverFires).expect("bought");
        assert_eq!(
            session.reserves().fuel,
            Gauge {
                now: 300.0,
                max: 400.0
            }
        );
        session
            .outfit(buy(SHIELD), &mut NeverFires)
            .expect("bought");
        assert_eq!(
            session.reserves().shield,
            Gauge {
                now: 60.0,
                max: 80.0
            }
        );
        assert_eq!(session.reserves().armor, Gauge::full(45.0), "unchanged");
        session.outfit(sell(TANK), &mut NeverFires).expect("sold");
        assert_eq!(
            session.reserves().fuel,
            Gauge {
                now: 300.0,
                max: 300.0
            }
        );
        session.outfit(sell(SHIELD), &mut NeverFires).expect("sold");
        assert_eq!(
            session.reserves().shield,
            Gauge {
                now: 30.0,
                max: 30.0
            }
        );
        session.pilot.reserves.fuel.now = 50.0;
        session.outfit(buy(TANK), &mut NeverFires).expect("bought");
        session.outfit(sell(TANK), &mut NeverFires).expect("sold");
        assert_eq!(
            session.reserves().fuel,
            Gauge {
                now: 150.0,
                max: 300.0
            }
        );
    }

    #[test]
    fn a_fuel_scoop_adds_to_the_regeneration_and_selling_it_takes_it_away() {
        let mut session = outfitted(&outfitting());
        assert_eq!(session.fuel_regen_per_tick(), 0.0);
        session.outfit(buy(SCOOP), &mut NeverFires).expect("bought");
        assert_eq!(session.fuel_regen_per_tick(), 0.1);
        assert_eq!(session.stats().fuel_regen, 0.1);
        session.outfit(sell(SCOOP), &mut NeverFires).expect("sold");
        assert_eq!(session.fuel_regen_per_tick(), 0.0);
    }

    #[test]
    fn an_outfit_bought_or_sold_makes_a_save_due_and_a_refused_one_changes_nothing() {
        let mut session = outfitted(&outfitting());
        let before = session.clone();
        assert_eq!(
            session.outfit(sell(SPEED), &mut NeverFires),
            Err(OutfitRefusal::NoneOwned)
        );
        assert_eq!(
            session.outfit(buy(OutfitId(999)), &mut NeverFires),
            Err(OutfitRefusal::NotListed)
        );
        assert_eq!(session, before);
        assert!(!session.take_save_due());
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        assert!(session.take_save_due());
        session.outfit(sell(SPEED), &mut NeverFires).expect("sold");
        assert!(session.take_save_due());
    }

    #[test]
    fn buying_is_refused_by_cash_free_mass_or_max() {
        let heavy = OutfitRecord {
            mass: 20,
            max: 3,
            ..outfit(305, &[])
        };
        let dear = OutfitRecord {
            cost: 30_000,
            ..outfit(306, &[])
        };
        let catalog = FakePilotCatalog {
            outfits: vec![heavy, dear],
            ..outfitting()
        };
        let mut session = outfitted(&catalog);
        assert_eq!(
            session.outfit(buy(OutfitId(306)), &mut NeverFires),
            Err(OutfitRefusal::CannotAfford)
        );
        assert_eq!(session.outfit(buy(OutfitId(305)), &mut NeverFires), Ok(()));
        // The list built, the day's rolls are as the refusal leaves them.
        session.outfitter(&mut NeverFires);
        let before = session.clone();
        assert_eq!(
            session.outfit(buy(OutfitId(305)), &mut NeverFires),
            Err(OutfitRefusal::NoSpace)
        );
        assert_eq!(session, before);
        let roomy = FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    free_mass: 100,
                    ..FAST
                }),
            )],
            ..catalog
        };
        let mut session = outfitted(&roomy);
        for _ in 0..3 {
            session
                .outfit(buy(OutfitId(305)), &mut NeverFires)
                .expect("bought");
        }
        assert_eq!(
            session.outfit(buy(OutfitId(305)), &mut NeverFires),
            Err(OutfitRefusal::MaxOwned)
        );
        assert_eq!(session.pilot().owned(OutfitId(305)), 3);
    }

    #[test]
    fn an_owned_outfit_that_sells_anywhere_sells_where_it_is_not_for_sale() {
        let map = OutfitRecord {
            tech_level: 9,
            cost: 4001,
            flags: OutfitFlags::SELL_ANYWHERE,
            ..outfit(310, &[])
        };
        let catalog = FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(OutfitId(310), 2)])],
            outfits: vec![map],
            ..outfitting()
        };
        let mut session = outfitted(&catalog);
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        assert!(outfitter.row(OutfitId(310)).is_some(), "listed, sell-only");
        assert_eq!(
            outfitter.can(buy(OutfitId(310))),
            Err(OutfitRefusal::NotForSale)
        );
        assert_eq!(outfitter.can(sell(OutfitId(310))), Ok(()));
        assert_eq!(session.outfit(sell(OutfitId(310)), &mut NeverFires), Ok(()));
        assert_eq!(session.pilot().owned(OutfitId(310)), 1);
        assert_eq!(session.pilot().cash(), 25_000 + 2000);
    }

    #[test]
    fn an_owned_sell_anywhere_outfit_of_tech_minus_1_is_bought_through_the_session() {
        let map = OutfitRecord {
            tech_level: -1,
            cost: 4001,
            flags: OutfitFlags::SELL_ANYWHERE,
            ..outfit(310, &[])
        };
        let catalog = FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(OutfitId(310), 2)])],
            outfits: vec![map],
            ..outfitting()
        };
        let mut session = outfitted(&catalog);
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        assert!(outfitter.row(OutfitId(310)).is_some(), "listed, off tech");
        assert_eq!(outfitter.can(buy(OutfitId(310))), Ok(()));
        assert_eq!(session.outfit(buy(OutfitId(310)), &mut NeverFires), Ok(()));
        assert_eq!(session.pilot().owned(OutfitId(310)), 3);
        assert_eq!(session.pilot().cash(), 25_000 - 4001);
    }

    #[test]
    fn a_new_pilots_default_items_are_owned_and_its_reserves_full_with_them() {
        let catalog = FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(TANK, 1), (SHIELD, 2)])],
            ..outfitting()
        };
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session.pilot().outfits().collect::<Vec<_>>(),
            [(SHIELD, 2), (TANK, 1)]
        );
        assert_eq!(
            session.reserves(),
            Reserves {
                shield: Gauge::full(130.0),
                armor: Gauge::full(45.0),
                fuel: Gauge::full(400.0),
            }
        );
        let outfitter = outfitted(&catalog)
            .outfitter(&mut NeverFires)
            .expect("an outfitter");
        assert_eq!(outfitter.free_mass, 30, "free mass is on top of them");
    }

    #[test]
    fn flying_a_pilot_with_outfits_gives_their_stats() {
        let catalog = outfitting();
        let mut session = outfitted(&catalog);
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        session.outfit(buy(TANK), &mut NeverFires).expect("bought");
        let text = crate::save::encode(session.pilot());
        let pilot = crate::save::decode(&text).expect("a pilot");
        let resumed = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(resumed.stats(), session.stats());
        assert_eq!(resumed.reserves(), session.reserves());
        assert_eq!(resumed.pilot().owned(SPEED), 1);
    }

    #[test]
    fn flying_a_pilot_from_before_outfits_gives_it_the_ships_default_items() {
        let catalog = FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(TANK, 1), (SCOOP, 1)])],
            ..outfitting()
        };
        let mut pilot = Pilot::new(&catalog, "Old").expect("starts");
        pilot.outfits.clear();
        pilot.default_outfits_pending = true;
        pilot.reserves = Reserves::full(30.0, 45.0, 300.0);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().outfits().collect::<Vec<_>>(),
            [(TANK, 1), (SCOOP, 1)]
        );
        assert!(!session.pilot().default_outfits_pending);
        assert_eq!(session.fuel_regen_per_tick(), 0.1);
        assert_eq!(
            session.reserves().fuel,
            Gauge {
                now: 300.0,
                max: 400.0
            },
            "as saved, with room for more"
        );
        assert!(!session.take_save_due());
        let saved: serde_json::Value =
            serde_json::from_str(&crate::save::encode(session.pilot())).expect("JSON");
        assert_eq!(
            saved["outfits"],
            serde_json::json!([{"outfit": 303, "count": 1}, {"outfit": 304, "count": 1}])
        );
    }

    #[test]
    fn flying_a_pilot_keeps_no_more_in_its_reserves_than_its_stats_hold() {
        let catalog = outfitting();
        let mut pilot = Pilot::new(&catalog, "").expect("starts");
        pilot.reserves = Reserves {
            shield: Gauge {
                now: 90.0,
                max: 100.0,
            },
            armor: Gauge {
                now: 20.0,
                max: 45.0,
            },
            fuel: Gauge {
                now: 350.0,
                max: 500.0,
            },
        };
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.reserves(),
            Reserves {
                shield: Gauge {
                    now: 30.0,
                    max: 30.0
                },
                armor: Gauge {
                    now: 20.0,
                    max: 45.0
                },
                fuel: Gauge {
                    now: 300.0,
                    max: 300.0
                },
            }
        );
    }

    /// A widget (305, `Max` 2) and a widget rack (306) that raises it.
    const WIDGET: OutfitId = OutfitId(305);
    const WIDGET_RACK: OutfitId = OutfitId(306);

    /// [`outfitting`] with the widget and its rack for sale too.
    fn raising() -> FakePilotCatalog {
        let mut catalog = outfitting();
        catalog.outfits.push(OutfitRecord {
            max: 2,
            ..outfit(305, &[])
        });
        catalog
            .outfits
            .push(outfit(306, &[(crate::outfitter::RAISES_MAX, WIDGET.0)]));
        catalog
    }

    #[test]
    fn a_session_buys_past_max_with_two_raisers_owned() {
        let mut session = outfitted(&raising());
        for _ in 0..2 {
            assert_eq!(session.outfit(buy(WIDGET), &mut NeverFires), Ok(()));
        }
        assert_eq!(
            session.outfit(buy(WIDGET), &mut NeverFires),
            Err(OutfitRefusal::MaxOwned)
        );
        session
            .outfit(buy(WIDGET_RACK), &mut NeverFires)
            .expect("one");
        assert_eq!(
            session.outfit(buy(WIDGET), &mut NeverFires),
            Err(OutfitRefusal::MaxOwned),
            "one rack leaves it"
        );
        session
            .outfit(buy(WIDGET_RACK), &mut NeverFires)
            .expect("two");
        for _ in 0..2 {
            assert_eq!(session.outfit(buy(WIDGET), &mut NeverFires), Ok(()));
        }
        assert_eq!(session.pilot().owned(WIDGET), 4);
        assert_eq!(
            session.outfit(buy(WIDGET), &mut NeverFires),
            Err(OutfitRefusal::MaxOwned)
        );
    }

    /// `session` owning 2 widget racks and 2 widgets, of `Max` 2 each.
    fn two_racks_and_widgets(session: &mut Session) {
        session
            .outfit_counted(buy(WIDGET_RACK), 2, &mut NeverFires)
            .expect("bought");
        session
            .outfit_counted(buy(WIDGET), 2, &mut NeverFires)
            .expect("bought");
    }

    #[test]
    fn by_the_engine_a_counted_sale_of_raisers_is_refused_for_the_whole_count() {
        // Two racks and two widgets of `Max` 2: a counted sale of both
        // leaves 2 - (2 - 2) x 2 = 2 widgets over at its first pass.
        let mut session = outfitted(&raising());
        two_racks_and_widgets(&mut session);
        let mut clicked = session.clone();
        let refused = OutfitRefusal::RaisedFirst {
            count: 2,
            target: WIDGET,
        };
        let ran = session.outfit_order(sell(WIDGET_RACK), 2, &mut NeverFires);
        assert_eq!(
            ran.run,
            crate::order_loop::OrderRun {
                done: 0,
                stopped: Some(crate::order_loop::Stopped {
                    pass: 0,
                    refusal: refused,
                }),
            }
        );
        let alert = ran.alert.expect("alerts");
        assert_eq!(alert.refusal, refused);
        assert!(alert.plural);
        assert!(alert.names.contains_key(&WIDGET));
        assert_eq!(session.pilot().owned(WIDGET_RACK), 2);
        // Single clicks sell one first.
        assert_eq!(clicked.outfit(sell(WIDGET_RACK), &mut NeverFires), Ok(()));
        assert_eq!(
            clicked.outfit(sell(WIDGET_RACK), &mut NeverFires),
            Err(refused)
        );
    }

    #[test]
    fn a_plain_sale_is_the_same_by_either_counted_reading() {
        for source in RuleSource::ALL {
            let mut session = outfitted(&raising())
                .with_rules(Rulebook::default().with_override(RuleKey::CountedSale, source));
            two_racks_and_widgets(&mut session);
            assert_eq!(
                session.outfit(sell(WIDGET_RACK), &mut NeverFires),
                Ok(()),
                "{source:?}"
            );
            assert_eq!(
                session.outfit(sell(WIDGET_RACK), &mut NeverFires),
                Err(OutfitRefusal::RaisedFirst {
                    count: 2,
                    target: WIDGET,
                }),
                "{source:?}"
            );
        }
    }

    #[test]
    fn the_session_reads_the_raised_max_as_its_rule_says() {
        let catalog = raising();
        assert_eq!(
            outfitted(&catalog).rules().source_for(RuleKey::RaisedMax),
            RuleSource::Engine
        );
        for (source, sold) in [
            (
                RuleSource::Engine,
                Err(OutfitRefusal::RaisedFirst {
                    count: 1,
                    target: WIDGET,
                }),
            ),
            (RuleSource::Bible, Ok(())),
        ] {
            let mut session = outfitted(&catalog)
                .with_rules(Rulebook::default().with_override(RuleKey::RaisedMax, source));
            assert_eq!(session.rules().source_for(RuleKey::RaisedMax), source);
            session
                .outfit(buy(WIDGET_RACK), &mut NeverFires)
                .expect("a rack");
            session
                .outfit(buy(WIDGET), &mut NeverFires)
                .expect("a widget");
            assert_eq!(
                session.outfit(sell(WIDGET_RACK), &mut NeverFires),
                sold,
                "{source:?}"
            );
        }
    }

    // The shipyard.

    use crate::catalog::ShipRecord;
    use crate::shipyard::{ShipNaming, ShipPurchase, ShipRefusal};
    use crate::testkit::ship;

    /// Ship 129: faster, with more shield, 15 tons of cargo space and 12
    /// free, of mass 25, regenerating a unit of fuel every 5 ticks.
    const HEAVY: ShipFields = ShipFields {
        speed: 900,
        shield: 80,
        fuel_regen: 5,
        holds: 15,
        free_mass: 12,
        mass: 25,
        ..FAST
    };

    /// Planet 128 at the centre, which the ship starts over: a shipyard
    /// and outfitter of tech level 5, and a trade center.
    fn shipyard_site() -> LandingSite {
        LandingSite {
            flags: TRADES | StellarFlags::OUTFITTER | StellarFlags::SHIPYARD,
            ..outfitter_site()
        }
    }

    /// [`outfitting`], where planet 128 is a shipyard too, selling ship
    /// 128 (FAST, 10,000 credits) and ship 129 ([`HEAVY`], 17,500
    /// credits, carrying a fuel tank). Both are of tech level 5, the
    /// planet's, so the shipyard's flux takes no low-tech discount from
    /// their prices.
    pub(super) fn shipbuying() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![(SystemId(130), vec![shipyard_site()])],
            ships: vec![(ShipId(128), Ok(FAST)), (ShipId(129), Ok(HEAVY))],
            defaults: vec![(ShipId(129), vec![(TANK, 1)])],
            ship_records: vec![
                ShipRecord {
                    tech_level: 5,
                    ..ship(128, FAST)
                },
                ShipRecord {
                    cost: 17_500,
                    defaults: vec![(TANK, 1)],
                    tech_level: 5,
                    ..ship(129, HEAVY)
                },
            ],
            ..outfitting()
        }
    }

    pub(super) const NEW: ShipId = ShipId(129);

    /// [`shipbuying`] with ship 130 too: `FreeMass` -10, a stock gun 130
    /// held by outfit 310 of 6 tons. Its Space is -10 + 6 = -4 to start,
    /// and the gun leaves 0.
    fn stock_gunned() -> FakePilotCatalog {
        let mut catalog = shipbuying();
        catalog.ship_records.push(ship(
            130,
            ShipFields {
                free_mass: -10,
                ..FAST
            },
        ));
        catalog.outfits.push(crate::catalog::OutfitRecord {
            mass: 6,
            ..outfit(310, &[(crate::combat::armament::MOD_WEAPON, 130)])
        });
        catalog.hulls.push(crate::catalog::HullRecord {
            weapons: vec![crate::catalog::StockWeapon {
                weapon: WeaponId(130),
                count: 1,
                ammo: 0,
            }],
            ..crate::testkit::hull(130)
        });
        catalog
    }

    #[test]
    fn the_shipyard_walks_each_space_over_the_sessions_stock() {
        let catalog = stock_gunned();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        let shipyard = session.shipyard(&mut NeverFires).expect("a shipyard");
        let row = shipyard.row(ShipId(130)).expect("listed");
        assert_eq!(row.specs.space, 0, "not its FreeMass of -10");
    }

    #[test]
    fn there_is_a_shipyard_only_while_landed_at_one() {
        let catalog = shipbuying();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.shipyard(&mut NeverFires), None, "in flight");
        assert_eq!(
            session.buy_ship(NEW, "Kestrel", &mut NeverFires),
            Err(ShipRefusal::NoShipyard)
        );
        land_now(&mut session).expect("lands");
        let shipyard = session.shipyard(&mut NeverFires).expect("a shipyard");
        assert_eq!(shipyard.rows.len(), 2);
        assert_eq!((shipyard.cash, shipyard.trade_in), (25_000, 2500));
        assert_eq!(shipyard.current, ShipId(128));
        let plain = FakePilotCatalog {
            sites: vec![(SystemId(130), vec![outfitter_site()])],
            ..shipbuying()
        };
        let mut session = outfitted(&plain);
        assert_eq!(session.shipyard(&mut NeverFires), None, "no shipyard here");
        assert_eq!(
            session.buy_ship(NEW, "Kestrel", &mut NeverFires),
            Err(ShipRefusal::NoShipyard)
        );
    }

    #[test]
    fn the_shipyard_tests_availability_through_the_sessions_control_bits() {
        let mut catalog = shipbuying();
        catalog.ship_records[1].availability = crate::control::Test::parse("b7");
        let mut session = outfitted(&catalog);
        session.set_control_bit(crate::control::Bit::new(7).expect("a bit"), true);
        let mut refusing = session
            .clone()
            .with_control_bits(Rc::new(crate::testkit::RefuseBits(&[7])));
        assert_eq!(
            refusing.buy_ship(NEW, "Kestrel", &mut NeverFires),
            Err(ShipRefusal::NotForSale)
        );
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought while bit 7 is set");
    }

    #[test]
    fn a_ship_type_is_named_by_its_ship_record() {
        let session = Session::start(&shipbuying()).expect("starts");
        assert_eq!(session.ship_name(ShipId(129)), Some("Ship 129"));
        assert_eq!(session.ship_name(ShipId(128)), Some("Ship 128"));
        assert_eq!(session.ship_name(ShipId(130)), None, "no such record");
    }

    #[test]
    fn a_session_reads_the_ship_records_once_when_it_flies() {
        let catalog = shipbuying();
        let pilot = Pilot::new(&catalog, "").expect("starts");
        assert_eq!(*catalog.ship_record_reads.borrow(), 1, "to name its ship");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        land_now(&mut session).expect("lands");
        assert_eq!(*catalog.ship_record_reads.borrow(), 2);
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        session.shipyard(&mut NeverFires).expect("a shipyard");
        session.take_off(&catalog, &mut NeverFires);
        jump(&mut session, &catalog, 131);
        assert_eq!(*catalog.ship_record_reads.borrow(), 2);
    }

    #[test]
    fn buying_a_ship_makes_a_save_due_and_a_refused_one_changes_nothing() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        let before = session.clone();
        assert_eq!(
            session.buy_ship(ShipId(999), "Kestrel", &mut NeverFires),
            Err(ShipRefusal::NotListed)
        );
        let mut poor = session.clone();
        poor.pilot.cash = 14_999;
        let poorer = poor.clone();
        assert_eq!(
            poor.buy_ship(NEW, "Kestrel", &mut NeverFires),
            Err(ShipRefusal::CannotAfford)
        );
        assert_eq!(poor, poorer);
        assert!(!poor.take_save_due());
        assert_eq!(session, before);
        assert!(!session.take_save_due());
        assert_eq!(
            session.buy_ship(NEW, "Kestrel", &mut NeverFires),
            Ok(ShipPurchase {
                price: 17_500,
                trade_in: 2500,
                sold_back: BTreeMap::new(),
                refund: 0,
                left_behind: BTreeMap::new(),
            })
        );
        assert!(session.take_save_due());
        assert_eq!(session.ship(), NEW);
        assert_eq!(session.pilot().cash(), 25_000 - 17_500 + 2500);
        assert_eq!(session.pilot().outfits().collect::<Vec<_>>(), [(TANK, 1)]);
    }

    #[test]
    fn a_pilot_in_debt_is_refused_a_ship_and_nothing_changes() {
        let catalog = shipbuying();
        let mut poor = outfitted(&catalog);
        poor.pilot.cash = -1;
        let before = poor.clone();
        assert_eq!(
            poor.buy_ship(NEW, "Kestrel", &mut NeverFires),
            Err(ShipRefusal::CannotAfford)
        );
        assert_eq!(
            poor.ship_naming(NEW, &mut NeverFires).map(|_| ()),
            Err(ShipRefusal::CannotAfford)
        );
        assert_eq!(poor, before);
        assert!(!poor.take_save_due());
        let mut cheap = shipbuying();
        cheap.ship_records[1].cost = 1000;
        let mut indebted = outfitted(&cheap);
        indebted.pilot.cash = -1;
        let unchanged = indebted.clone();
        assert_eq!(
            indebted.buy_ship(NEW, "Kestrel", &mut NeverFires),
            Err(ShipRefusal::CannotAfford),
            "the 2,500 trade-in covers the 1,000 price and the debt"
        );
        assert_eq!(indebted, unchanged);
        let mut even = outfitted(&cheap);
        even.pilot.cash = 0;
        let bought = even
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("cash 0 buys a ship whose trade-in exceeds its price");
        assert_eq!((bought.price, bought.trade_in), (1000, 2500));
        assert_eq!(even.pilot().cash(), 2500 - 1000, "paid the difference");
    }

    #[test]
    fn buying_the_class_flown_is_refused_and_changes_nothing() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session.take_events();
        let before = session.clone();
        assert_eq!(
            session.buy_ship(ShipId(128), "Kestrel", &mut NeverFires),
            Err(ShipRefusal::AlreadyFlown)
        );
        assert_eq!(
            session.ship_naming(ShipId(128), &mut NeverFires),
            Err(ShipRefusal::AlreadyFlown)
        );
        assert_eq!(session, before);
        assert!(!session.take_save_due());
        assert_eq!(session.take_events(), []);
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(
            session.buy_ship(NEW, "Other", &mut NeverFires),
            Err(ShipRefusal::AlreadyFlown),
            "the new class, under its new name"
        );
        assert!(
            session.ship_naming(ShipId(128), &mut NeverFires).is_ok(),
            "the refusal follows the class flown now"
        );
    }

    #[test]
    fn a_purchase_tells_the_ship_changed_and_a_refused_one_tells_nothing() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session.take_events();
        let _ = session.buy_ship(ShipId(999), "Kestrel", &mut NeverFires);
        assert_eq!(session.take_events(), [], "refused");
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(session.take_events(), [SessionEvent::ShipChanged]);
    }

    #[test]
    fn an_on_purchase_change_of_ship_tells_the_ship_changed_again() {
        let mut catalog = shipbuying();
        catalog.ship_records[1].on_purchase = crate::control::Script::parse("H128");
        let mut session = outfitted(&catalog);
        session.take_events();
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(session.ship(), ShipId(128));
        assert_eq!(
            session.take_events(),
            [SessionEvent::ShipChanged, SessionEvent::ShipChanged]
        );
    }

    #[test]
    fn buying_a_ship_keeps_the_cargo_as_its_rule_says_with_the_trader_escorts() {
        let catalog = FakePilotCatalog {
            ship_records: [shipbuying().ship_records, vec![holds_record(130, 15, 1)]].concat(),
            ..shipbuying()
        };
        let opals = Good::Junk(JunkId(146));
        assert_eq!(
            Session::start(&catalog)
                .expect("starts")
                .rules()
                .source_for(RuleKey::PurchaseCargo),
            RuleSource::Engine
        );
        // The new ship's 15 tons of 30 with the trader: f = 0.5.
        for (source, kept, left_behind) in [
            (
                RuleSource::Engine,
                vec![(FOOD, 5), (opals, 2)],
                BTreeMap::from([(FOOD, 5), (opals, 2)]),
            ),
            (
                RuleSource::Bible,
                vec![(FOOD, 10), (opals, 4)],
                BTreeMap::new(),
            ),
        ] {
            let mut session = outfitted(&catalog)
                .with_rules(Rulebook::default().with_override(RuleKey::PurchaseCargo, source));
            assert_eq!(session.rules().source_for(RuleKey::PurchaseCargo), source);
            session.pilot.escorts = vec![
                fleet_escort(130),
                Escort {
                    carried: true,
                    ..fleet_escort(130)
                },
            ];
            session.pilot.cargo = BTreeMap::from([(FOOD, 10), (opals, 4)]);
            let bought = session
                .buy_ship(NEW, "Kestrel", &mut NeverFires)
                .expect("bought");
            assert_eq!(
                session.pilot().cargo().collect::<Vec<_>>(),
                kept,
                "{source:?}: the launched fighter takes no share"
            );
            assert_eq!(bought.left_behind, left_behind, "{source:?}");
        }
    }

    #[test]
    fn buying_a_ship_counts_the_mission_cargo_aboard_and_never_leaves_it() {
        // The new ship's 15 tons: by the engine 14 x 15/20, 10, and then,
        // as 16 tons are still past them, 10 x 15/16; by the other reading
        // the 9 tons the mission cargo leaves.
        for (source, kept, left_behind) in [(RuleSource::Engine, 9, 5), (RuleSource::Bible, 9, 5)] {
            let mut session = outfitted(&shipbuying())
                .with_rules(Rulebook::default().with_override(RuleKey::PurchaseCargo, source));
            session.pilot.cargo = BTreeMap::from([(FOOD, 14)]);
            session
                .pilot
                .missions
                .put(0, crate::testkit::carrying(140, 6));
            let bought = session
                .buy_ship(NEW, "Kestrel", &mut NeverFires)
                .expect("bought");
            assert_eq!(session.capacity(), 15, "{source:?}");
            assert_eq!(
                session.pilot().cargo().collect::<Vec<_>>(),
                [(FOOD, kept)],
                "{source:?}"
            );
            assert_eq!(
                bought.left_behind,
                BTreeMap::from([(FOOD, left_behind)]),
                "{source:?}"
            );
            let held = session.pilot().missions.get(0).expect("still held");
            assert_eq!((held.cargo_tons, held.aboard), (6, true), "{source:?}");
            assert_eq!(session.pilot().total_cargo(), i64::from(kept) + 6);
        }
    }

    #[test]
    fn flying_an_unnamed_ship_names_it_after_its_class() {
        let catalog = shipbuying();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.ship_name = None;
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.pilot().ship_name(), Some("Ship 128"));
        assert!(!session.take_save_due());
        let mut named = Pilot::new(&catalog, "Ada").expect("starts");
        named.ship_name = Some("Kestrel".to_owned());
        let session = Session::fly(&catalog, named).expect("flies");
        assert_eq!(session.pilot().ship_name(), Some("Kestrel"), "kept");
        let mut classless = Pilot::new(&catalog, "Ada").expect("starts");
        classless.ship_name = None;
        let bare = FakePilotCatalog {
            ship_records: Vec::new(),
            ..shipbuying()
        };
        let session = Session::fly(&bare, classless).expect("flies");
        assert_eq!(session.pilot().ship_name(), Some(""), "no record");
    }

    #[test]
    fn the_naming_is_refused_as_the_purchase_would_be() {
        let catalog = shipbuying();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session.ship_naming(NEW, &mut NeverFires),
            Err(ShipRefusal::NoShipyard)
        );
        land_now(&mut session).expect("lands");
        session.take_save_due();
        assert_eq!(
            session.ship_naming(ShipId(999), &mut NeverFires),
            Err(ShipRefusal::NotListed)
        );
        let mut poor = session.clone();
        poor.pilot.cash = 14_999;
        assert_eq!(
            poor.ship_naming(NEW, &mut NeverFires),
            Err(ShipRefusal::CannotAfford)
        );
        let before = session.clone();
        let mut chance = Scripted::rolling(&[0, 0, 0]);
        assert_eq!(
            session.ship_naming(NEW, &mut chance),
            Ok(ShipNaming {
                ship: NEW,
                prompt: "Please name your new The Ship 129: ".to_owned(),
                default: "Ship 129 111".to_owned(),
            })
        );
        assert_eq!(chance.sides_asked, [9, 9, 9]);
        assert_eq!(session, before, "naming changes nothing");
        assert!(!session.take_save_due());
    }

    #[test]
    fn buying_names_the_ship() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session
            .buy_ship(NEW, "The Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(session.pilot().ship_name(), Some("Kestrel"));
        assert!(session.take_save_due());
    }

    #[test]
    fn after_a_purchase_the_stats_are_the_new_ships_with_its_outfits() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        let stats = ShipStats::new(
            HEAVY,
            &[crate::fuel::OutfitMod {
                mod_type: MORE_FUEL,
                mod_val: 100,
                count: 1,
            }],
        );
        assert_eq!(session.stats(), stats, "the booster went with the old ship");
        assert_eq!(session.handling(), stats.handling);
        assert_eq!(session.handling().max_speed, 9.0);
        assert_eq!(session.capacity(), 15);
        assert_eq!(session.reserves(), stats.full());
        assert_eq!(session.reserves().fuel, Gauge::full(400.0));
        assert_eq!(session.fuel_regen_per_tick(), 0.2);
        assert_eq!(session.market().expect("an exchange").capacity, 15);
    }

    #[test]
    fn after_a_purchase_the_outfitter_reads_the_new_ships_free_mass_and_defaults() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        assert_eq!(outfitter.free_mass, 12, "its tank is fitted on top");
        let shipyard = session.shipyard(&mut NeverFires).expect("a shipyard");
        assert_eq!(shipyard.current, NEW);
        assert_eq!(
            shipyard.trade_in, 4870,
            "the hull and its tank, 4375 + 500, rounded down to ten by the flux"
        );
    }

    // Stock weapons.

    /// A 5-ton gun (weapon 150), 1000 credits.
    const GUN: OutfitId = OutfitId(305);
    /// A rocket launcher (weapon 138), 2000 credits.
    const LAUNCHER: OutfitId = OutfitId(306);
    /// A rocket (weapon 138's rounds), 20 credits and massless.
    const ROCKETS: OutfitId = OutfitId(307);

    /// `catalog` with ship 128 stocking two guns, and ship 129 a gun and
    /// a rocket launcher with 20 rockets, each held by its outfit.
    fn stock_weapons(catalog: FakePilotCatalog) -> FakePilotCatalog {
        let stock = |id, count, ammo| StockWeapon {
            weapon: WeaponId(id),
            count,
            ammo,
        };
        let mut catalog = catalog;
        catalog.weapons.extend([
            weapon(150),
            WeaponRecord {
                ammo_type: 10,
                flags: SECONDARY,
                ..weapon(138)
            },
        ]);
        catalog.hulls.extend([
            HullRecord {
                weapons: vec![stock(150, 2, 0)],
                ..hull(128)
            },
            HullRecord {
                weapons: vec![stock(150, 1, 0), stock(138, 1, 20)],
                ..hull(129)
            },
        ]);
        catalog.outfits.extend([
            OutfitRecord {
                mass: 5,
                ..outfit(305, &[(MOD_WEAPON, 150)])
            },
            OutfitRecord {
                cost: 2000,
                ..outfit(306, &[(MOD_WEAPON, 138)])
            },
            OutfitRecord {
                cost: 20,
                mass: 0,
                max: 100,
                ..outfit(307, &[(MOD_AMMO, 138)])
            },
        ]);
        catalog
    }

    #[test]
    fn the_stock_weapons_mass_is_room_the_hull_already_has() {
        let catalog = stock_weapons(outfitting());
        let mut session = outfitted(&catalog);
        assert_eq!(session.pilot().owned(GUN), 2);
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        assert_eq!(
            outfitter.free_mass,
            i64::from(FAST.free_mass),
            "FreeMass leaves out the 10 tons of guns"
        );
    }

    #[test]
    fn the_classes_max_gun_and_a_stock_gun_limit_the_outfitter() {
        let gun_buy = |max_gun: Option<i16>| {
            let mut catalog = stock_weapons(outfitting());
            for record in &mut catalog.outfits {
                if record.id == GUN {
                    record.flags = OutfitFlags::GUN;
                }
            }
            if let Some(max_gun) = max_gun {
                catalog.ship_records = vec![ShipRecord {
                    max_gun,
                    ..ship(128, FAST)
                }];
            }
            let mut session = outfitted(&catalog);
            assert_eq!(session.pilot().owned(GUN), 2, "the stock guns");
            let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
            (outfitter.check(buy(GUN)), outfitter.check(buy(SPEED)))
        };
        assert_eq!(gun_buy(Some(3)), (Ok(()), Ok(())));
        assert_eq!(gun_buy(Some(2)), (Err(OutfitRefusal::GunLimit), Ok(())));
        assert_eq!(
            gun_buy(None),
            (Err(OutfitRefusal::GunLimit), Ok(())),
            "a class with no record has room for none"
        );
    }

    /// Each weapon `session` mounts, with how many.
    fn mounted(session: &Session) -> Vec<(i16, u32)> {
        session
            .armament
            .mounts()
            .iter()
            .map(|mount| (mount.spec.id.0, mount.count))
            .collect()
    }

    #[test]
    fn a_new_pilots_armament_mounts_each_stock_weapon_once() {
        let catalog = stock_weapons(shipbuying());
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(mounted(&session), [(150, 2)]);
        assert_eq!(session.stock, BTreeMap::from([(GUN, 2)]));
    }

    #[test]
    fn buying_a_ship_takes_the_name_given_not_its_classs() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session.pilot.ship_name = Some("Old Name".to_owned());
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(session.pilot().ship_name(), Some("Kestrel"));
    }

    #[test]
    fn a_bought_ships_armament_mounts_each_stock_weapon_once() {
        let catalog = stock_weapons(shipbuying());
        let mut session = outfitted(&catalog);
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(mounted(&session), [(150, 1), (138, 1)]);
        assert_eq!(
            session.pilot().outfits().collect::<Vec<_>>(),
            [(TANK, 1), (GUN, 1), (LAUNCHER, 1), (ROCKETS, 20)]
        );
        assert_eq!(
            session.stock,
            BTreeMap::from([(GUN, 1), (LAUNCHER, 1), (ROCKETS, 20)])
        );
        assert_eq!(session.secondary(), None);
        session.select_secondary(false);
        assert_eq!(session.secondary(), Some(WeaponId(138)));
        assert_eq!(session.secondary_line_count(), Some(20));
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        assert_eq!(
            outfitter.free_mass,
            i64::from(HEAVY.free_mass),
            "its tank, gun and launcher are fitted on top"
        );
    }

    #[test]
    fn buying_a_ship_unpaints_it_and_a_refused_purchase_keeps_the_paint() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        let paint = crate::outfit_effects::Rgb15 { r: 4, g: 5, b: 6 };
        session.pilot.paint = Some(paint);
        assert_eq!(
            session.buy_ship(ShipId(999), "Kestrel", &mut NeverFires),
            Err(ShipRefusal::NotListed)
        );
        assert_eq!(session.pilot().paint(), Some(paint));
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(session.pilot().paint(), None);
    }

    #[test]
    fn the_trade_in_counts_stock_weapons_and_rounds_as_the_original_does() {
        let catalog = stock_weapons(shipbuying());
        let mut session = outfitted(&catalog);
        let shipyard = session.shipyard(&mut NeverFires).expect("a shipyard");
        assert_eq!(shipyard.trade_in, 2500 + 2 * 500, "two guns");
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        let shipyard = session.shipyard(&mut NeverFires).expect("a shipyard");
        assert_eq!(
            shipyard.trade_in, 6570,
            "a quarter of the hull, and half its tank, gun, launcher and rockets, \
             4375 + 500 + 500 + 1000 + 200 = 6575, rounded down to ten by the flux"
        );
    }

    #[test]
    fn by_the_engine_the_trade_in_counts_an_unsellable_outfit_but_not_a_persistent_one() {
        // An unsellable 3000-credit map and a persistent 1000-credit
        // licence on ship 128, whose hull trades in for 2500.
        let mut catalog = shipbuying();
        catalog.outfits.push(OutfitRecord {
            cost: 3000,
            flags: OutfitFlags::CANNOT_SELL,
            ..outfit(330, &[])
        });
        catalog.outfits.push(OutfitRecord {
            cost: 1000,
            flags: OutfitFlags::PERSISTENT,
            ..outfit(331, &[])
        });
        let trade_in = |source| {
            let mut session = outfitted(&catalog)
                .with_rules(Rulebook::default().with_override(RuleKey::TradeInOutfits, source));
            session.pilot.outfits = BTreeMap::from([(OutfitId(330), 1), (OutfitId(331), 1)]);
            session
                .shipyard(&mut NeverFires)
                .expect("a shipyard")
                .trade_in
        };
        assert_eq!(trade_in(RuleSource::Engine), 2500 + 1500);
        assert_eq!(trade_in(RuleSource::Bible), 2500);
    }

    #[test]
    fn flying_an_older_save_fits_its_stock_weapons_on_top_of_its_outfits() {
        let catalog = stock_weapons(outfitting());
        let mut pilot = Pilot::new(&catalog, "Old").expect("starts");
        pilot.outfits = BTreeMap::from([(GUN, 1)]);
        pilot.stock_weapons_pending = true;
        let pilot = crate::save::decode(&crate::save::encode(&pilot)).expect("a pilot");
        assert!(pilot.stock_weapons_pending);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().outfits().collect::<Vec<_>>(),
            [(GUN, 3)],
            "the one bought and the two it mounted"
        );
        assert_eq!(mounted(&session), [(150, 3)]);
        assert!(!session.pilot().stock_weapons_pending);
        assert!(!session.take_save_due());
        let saved: serde_json::Value =
            serde_json::from_str(&crate::save::encode(session.pilot())).expect("JSON");
        assert_eq!(saved["stock_weapons_fitted"], true);
        assert_eq!(
            saved["outfits"],
            serde_json::json!([{"outfit": 305, "count": 3}])
        );
        land_now(&mut session).expect("lands");
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        assert_eq!(outfitter.free_mass, 30 - 5, "the gun bought takes room");
        // A save from before outfits: the default items, then the stock.
        let catalog = FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(TANK, 1)])],
            ..stock_weapons(outfitting())
        };
        let mut pilot = Pilot::new(&catalog, "Older").expect("starts");
        pilot.outfits.clear();
        pilot.default_outfits_pending = true;
        pilot.stock_weapons_pending = true;
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().outfits().collect::<Vec<_>>(),
            [(TANK, 1), (GUN, 2)]
        );
        assert!(!session.pilot().default_outfits_pending);
    }

    #[test]
    fn after_take_off_the_ship_flies_at_the_new_top_speed() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("took off");
        let mut expected = *session.player();
        let handling = Handling {
            max_speed: 9.0 * top_speed::NON_STRICT_SPEED,
            ..ShipStats::new(HEAVY, &[]).handling
        };
        for _ in 0..200 {
            session.tick(THRUST);
            step(&mut expected, &handling, THRUST);
        }
        assert_eq!(*session.player(), expected);
        assert!((session.player().velocity.length() - 13.5).abs() < 1e-3);
    }

    #[test]
    fn a_purchase_sells_back_a_persistent_outfit_priced_on_the_ship_flown() {
        // A persistent armour plate, 100 credits a ton of ship and 20 tons,
        // fitted to ship 128 (mass 40): it does not fit ship 129's 12 tons
        // free, and sells back at half of 100 x 40, not of 100 x 25.
        const PLATE: OutfitId = OutfitId(320);
        let plate = OutfitRecord {
            mass: 20,
            cost: 100,
            max: 1,
            flags: OutfitFlags::PERSISTENT | OutfitFlags::PRICE_BY_MASS,
            ..outfit(320, &[])
        };
        let mut catalog = shipbuying();
        catalog.outfits.push(plate);
        catalog.defaults.push((ShipId(128), vec![(PLATE, 1)]));
        let mut session = outfitted(&catalog).with_rules(
            Rulebook::default().with_override(RuleKey::PurchaseCarry, RuleSource::Bible),
        );
        assert_eq!(session.pilot().owned(PLATE), 1);
        assert_ne!(FAST.mass, HEAVY.mass);
        let bought = session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(bought.sold_back, BTreeMap::from([(PLATE, 1)]));
        assert_eq!(bought.refund, 100 * i64::from(FAST.mass) / 2);
        assert_eq!(bought.trade_in, 2500, "persistent: not in the trade-in");
        assert_eq!(session.pilot().cash(), 25_000 - 17_500 + 2500 + 2000);
        assert_eq!(session.pilot().owned(PLATE), 0);
    }

    #[test]
    fn by_the_engine_a_purchase_keeps_a_persistent_outfit_past_the_new_free_mass() {
        // The plate of the test above, 20 tons: by the engine it carries
        // over though ship 129 has 12 tons free, and nothing is sold back.
        const PLATE: OutfitId = OutfitId(320);
        let plate = OutfitRecord {
            mass: 20,
            cost: 100,
            max: 1,
            flags: OutfitFlags::PERSISTENT | OutfitFlags::PRICE_BY_MASS,
            ..outfit(320, &[])
        };
        let mut catalog = shipbuying();
        catalog.outfits.push(plate);
        catalog.defaults.push((ShipId(128), vec![(PLATE, 1)]));
        let mut session = outfitted(&catalog);
        let bought = session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(bought.sold_back, BTreeMap::new());
        assert_eq!(bought.refund, 0);
        assert_eq!(bought.trade_in, 2500);
        assert_eq!(session.pilot().cash(), 25_000 - 17_500 + 2500);
        assert_eq!(session.pilot().owned(PLATE), 1);
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        assert_eq!(outfitter.free_mass, 12 - 20, "its tank is fitted on top");
    }

    #[test]
    fn a_pilot_saved_after_a_purchase_flies_again_as_it_was() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session
            .trade(order(&session, FOOD, Direction::Buy, Lot::Max))
            .expect("bought");
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        let text = crate::save::encode(session.pilot());
        let pilot = crate::save::decode(&text).expect("a pilot");
        assert_eq!(pilot, *session.pilot());
        let resumed = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(resumed.ship(), NEW);
        assert_eq!(resumed.pilot().outfits().collect::<Vec<_>>(), [(TANK, 1)]);
        assert_eq!(resumed.pilot().cash(), session.pilot().cash());
        assert_eq!(resumed.pilot().held(FOOD), 15);
        assert_eq!(resumed.reserves(), session.reserves());
        assert_eq!(resumed.stats(), session.stats());
        assert_eq!(*resumed.pilot(), *session.pilot(), "no default added twice");
    }

    // BuyRandom.

    /// [`shipbuying`], with the speed booster for sale on a `BuyRandom` of
    /// `outfit_random` and ship 129 on one of `ship_random`.
    fn rolling(outfit_random: i16, ship_random: i16) -> FakePilotCatalog {
        let mut catalog = shipbuying();
        catalog.outfits[0].buy_random = outfit_random;
        catalog.ship_records[1].buy_random = ship_random;
        catalog
    }

    /// Takes off and lands on planet 128 again.
    fn relanded(session: &mut Session) {
        session
            .take_off(&NoTraffic, &mut NeverFires)
            .expect("takes off");
        land_now(session).expect("lands again");
    }

    fn lists_speed(session: &mut Session, chance: &mut dyn Chance) -> bool {
        let outfitter = session.outfitter(chance).expect("an outfitter");
        outfitter.row(SPEED).is_some()
    }

    fn lists_new(session: &mut Session, chance: &mut dyn Chance) -> bool {
        let shipyard = session.shipyard(chance).expect("a shipyard");
        shipyard.row(NEW).is_some()
    }

    /// The rule set reading [`RuleKey::DayRolls`] the other way: a
    /// landing forgets the rolls.
    fn landing_rolls() -> Rulebook {
        Rulebook::default().with_override(RuleKey::DayRolls, RuleSource::Bible)
    }

    /// The rule set whose stay passes no day ([`RuleKey::StayDays`]).
    fn no_stay_days() -> Rulebook {
        Rulebook::default().with_override(RuleKey::StayDays, RuleSource::Bible)
    }

    /// The original redraws the rolls only in its daily step
    /// (`_IncrementGameTime` @0xb7c4-0xb833), and no landing path writes
    /// them: the take-off's day draws the roll, and the landing keeps it.
    #[test]
    fn the_outfitters_rolls_are_drawn_once_a_day() {
        let mut session = outfitted(&rolling(50, 100));
        let mut missing = Scripted::answering(&[false]);
        assert!(!lists_speed(&mut session, &mut missing), "drawn as asked");
        assert_eq!(missing.asked, [50]);
        let mut firing = Scripted::answering(&[true, true]);
        assert!(!lists_speed(&mut session, &mut firing), "kept");
        assert!(firing.asked.is_empty());
        session
            .take_off(&NoTraffic, &mut firing)
            .expect("takes off");
        assert_eq!(firing.asked, [50], "the take-off's day draws it");
        land_now(&mut session).expect("lands again");
        assert!(lists_speed(&mut session, &mut firing), "the day's roll");
        assert_eq!(firing.asked, [50], "the landing keeps it");
    }

    /// The original redraws the rolls only in its daily step
    /// (`_IncrementGameTime` @0xb7c4-0xb833), and no landing path writes
    /// them: the take-off's day draws the roll, and the landing keeps it.
    #[test]
    fn the_shipyards_rolls_are_drawn_once_a_day() {
        let mut session = outfitted(&rolling(100, 50));
        let mut missing = Scripted::answering(&[false]);
        assert!(!lists_new(&mut session, &mut missing), "drawn as asked");
        assert_eq!(missing.asked, [50]);
        let mut firing = Scripted::answering(&[true, true]);
        assert!(!lists_new(&mut session, &mut firing), "kept");
        assert!(firing.asked.is_empty());
        session
            .take_off(&NoTraffic, &mut firing)
            .expect("takes off");
        assert_eq!(firing.asked, [50], "the take-off's day draws it");
        land_now(&mut session).expect("lands again");
        assert!(lists_new(&mut session, &mut firing), "the day's roll");
        assert_eq!(firing.asked, [50], "the landing keeps it");
    }

    #[test]
    fn a_take_offs_day_draws_the_rolls_the_next_landing_reads() {
        let mut session = outfitted(&rolling(50, 50));
        let mut day = Scripted::answering(&[true, false]);
        session.take_off(&NoTraffic, &mut day).expect("takes off");
        assert_eq!(day.asked, [50, 50], "the ship class, then the outfit");
        land_now(&mut session).expect("lands again");
        let mut none = Scripted::default();
        assert!(lists_new(&mut session, &mut none));
        assert!(!lists_speed(&mut session, &mut none));
        assert!(none.asked.is_empty(), "the landing reads the day's rolls");
    }

    #[test]
    fn a_landing_with_no_day_passing_keeps_the_rolls() {
        let mut session = outfitted(&rolling(50, 50)).with_rules(no_stay_days());
        let mut firing = Scripted::answering(&[true, false]);
        assert!(lists_speed(&mut session, &mut firing));
        assert!(!lists_new(&mut session, &mut firing));
        relanded(&mut session);
        let mut none = Scripted::answering(&[false, true]);
        assert!(lists_speed(&mut session, &mut none));
        assert!(!lists_new(&mut session, &mut none));
        assert!(none.asked.is_empty(), "kept");
    }

    #[test]
    fn a_declined_class_is_drawn_again_after_a_landing_with_no_day() {
        let mut session = outfitted(&rolling(50, 50)).with_rules(no_stay_days());
        let mut firing = Scripted::answering(&[true, true]);
        assert!(lists_speed(&mut session, &mut firing));
        assert!(lists_new(&mut session, &mut firing));
        session.decline_ship(NEW);
        relanded(&mut session);
        let mut missing = Scripted::answering(&[false]);
        assert!(lists_speed(&mut session, &mut missing), "kept");
        assert!(!lists_new(&mut session, &mut missing), "drawn again");
        assert_eq!(missing.asked, [50]);
    }

    #[test]
    fn a_bought_class_is_drawn_again_after_a_landing_with_no_day() {
        let mut session = outfitted(&rolling(50, 50)).with_rules(no_stay_days());
        let mut firing = Scripted::answering(&[true, true]);
        assert!(lists_speed(&mut session, &mut firing));
        session
            .buy_ship(NEW, "Kestrel", &mut firing)
            .expect("bought");
        assert_eq!(firing.asked, [50, 50]);
        relanded(&mut session);
        let mut missing = Scripted::answering(&[false]);
        assert!(!lists_new(&mut session, &mut missing), "drawn again");
        assert_eq!(missing.asked, [50]);
    }

    /// The daily step draws every roll afresh (`_IncrementGameTime`
    /// @0xb7c4-0xb833), and no landing path writes one: only the redraw on
    /// landing changes, the day's draw settling the declined class.
    #[test]
    fn a_days_draw_supersedes_the_redraws_due() {
        let mut session = outfitted(&rolling(100, 50));
        let mut firing = Scripted::answering(&[true, true]);
        assert!(lists_new(&mut session, &mut firing));
        session.decline_ship(NEW);
        session
            .take_off(&NoTraffic, &mut firing)
            .expect("takes off");
        land_now(&mut session).expect("lands again");
        assert!(lists_new(&mut session, &mut firing));
        assert_eq!(firing.asked, [50, 50], "the listing's draw, then the day's");
        assert!(lists_new(&mut session, &mut firing), "kept");
        assert_eq!(firing.asked, [50, 50]);
    }

    #[test]
    fn an_owned_outfits_hold_lasts_until_the_next_day() {
        let mut session = outfitted(&rolling(50, 100)).with_rules(no_stay_days());
        let mut firing = Scripted::answering(&[true]);
        assert_eq!(session.outfit(buy(SPEED), &mut firing), Ok(()));
        assert_eq!(session.outfit(sell(SPEED), &mut firing), Ok(()));
        relanded(&mut session);
        let mut missing = Scripted::default();
        assert!(lists_speed(&mut session, &mut missing), "still held");
        assert!(missing.asked.is_empty());
        session.pass_days(1, &mut missing);
        assert_eq!(missing.asked, [50], "the day draws it again");
        assert!(!lists_speed(&mut session, &mut missing));
        assert_eq!(missing.asked, [50]);
    }

    #[test]
    fn by_the_other_reading_the_outfitters_rolls_are_drawn_once_a_landing_and_again_at_the_next() {
        let mut session = outfitted(&rolling(50, 100)).with_rules(landing_rolls());
        let mut missing = Scripted::answering(&[false]);
        assert!(!lists_speed(&mut session, &mut missing));
        assert_eq!(missing.asked, [50]);
        let mut firing = Scripted::answering(&[true, true]);
        assert!(!lists_speed(&mut session, &mut firing), "kept");
        assert!(firing.asked.is_empty());
        relanded(&mut session);
        assert!(lists_speed(&mut session, &mut firing));
        assert_eq!(firing.asked, [50]);
    }

    #[test]
    fn outfit_trades_draw_no_roll_again() {
        let mut session = outfitted(&rolling(50, 100));
        let mut chance = Scripted::answering(&[true]);
        assert_eq!(session.outfit(buy(SPEED), &mut chance), Ok(()));
        assert_eq!(session.outfit(sell(SPEED), &mut chance), Ok(()));
        assert!(lists_speed(&mut session, &mut chance));
        assert_eq!(chance.asked, [50]);
    }

    #[test]
    fn by_the_other_reading_the_shipyards_rolls_are_drawn_once_a_landing_and_again_at_the_next() {
        let mut session = outfitted(&rolling(100, 50)).with_rules(landing_rolls());
        let mut missing = Scripted::answering(&[false]);
        assert!(!lists_new(&mut session, &mut missing));
        assert_eq!(missing.asked, [50]);
        let mut firing = Scripted::answering(&[true, true]);
        assert!(!lists_new(&mut session, &mut firing), "kept");
        assert!(firing.asked.is_empty());
        relanded(&mut session);
        assert!(lists_new(&mut session, &mut firing));
        assert_eq!(firing.asked, [50]);
    }

    #[test]
    fn buying_a_ship_draws_its_class_roll_again() {
        let mut session = outfitted(&rolling(100, 50));
        let mut firing = Scripted::answering(&[true]);
        session
            .buy_ship(NEW, "Kestrel", &mut firing)
            .expect("bought");
        assert_eq!(firing.asked, [50]);
        let mut missing = Scripted::answering(&[false]);
        assert!(!lists_new(&mut session, &mut missing), "drawn again");
        assert_eq!(missing.asked, [50]);
    }

    #[test]
    fn declining_draws_the_classs_roll_again_and_changes_nothing() {
        let mut session = outfitted(&rolling(100, 50));
        let mut firing = Scripted::answering(&[true]);
        assert!(lists_new(&mut session, &mut firing));
        let before = session.pilot().clone();
        session.decline_ship(NEW);
        assert_eq!(session.pilot(), &before);
        assert!(!session.take_save_due());
        let mut missing = Scripted::answering(&[false]);
        assert!(!lists_new(&mut session, &mut missing), "drawn again");
        assert_eq!(missing.asked, [50]);
    }

    #[test]
    fn a_declined_class_keeps_its_roll_until_the_shipyard_is_listed_again() {
        let mut session = outfitted(&rolling(100, 50));
        let mut firing = Scripted::answering(&[true]);
        assert!(lists_new(&mut session, &mut firing));
        session.decline_ship(NEW);
        let mut missing = Scripted::answering(&[false]);
        assert!(
            session.ship_naming(NEW, &mut missing).is_ok(),
            "named again"
        );
        assert!(missing.asked.is_empty(), "the list's roll is kept");
        assert!(!lists_new(&mut session, &mut missing), "drawn again");
        assert_eq!(missing.asked, [50]);
    }

    #[test]
    fn a_bought_class_keeps_its_roll_until_the_shipyard_is_listed_again() {
        let mut session = outfitted(&rolling(100, 50));
        let mut firing = Scripted::answering(&[true]);
        session
            .buy_ship(NEW, "Kestrel", &mut firing)
            .expect("bought");
        let mut missing = Scripted::answering(&[false]);
        assert_ne!(
            session.ship_naming(NEW, &mut missing).err(),
            Some(ShipRefusal::NotListed),
            "still listed"
        );
        assert!(missing.asked.is_empty(), "the list's roll is kept");
    }

    #[test]
    fn by_the_other_reading_a_landing_forgets_the_redraws_due() {
        let mut session = outfitted(&rolling(100, 50)).with_rules(landing_rolls());
        let mut firing = Scripted::answering(&[true, true]);
        assert!(lists_new(&mut session, &mut firing));
        session.decline_ship(NEW);
        relanded(&mut session);
        assert!(lists_new(&mut session, &mut firing));
        assert_eq!(firing.asked, [50, 50]);
        assert!(lists_new(&mut session, &mut firing), "kept");
        assert_eq!(firing.asked, [50, 50]);
    }

    #[test]
    fn an_item_off_today_is_refused_as_not_listed() {
        let mut session = outfitted(&rolling(50, 50));
        let before = session.clone();
        assert_eq!(
            session.outfit(buy(SPEED), &mut NeverFires),
            Err(OutfitRefusal::NotListed)
        );
        assert_eq!(
            session.buy_ship(NEW, "Kestrel", &mut NeverFires),
            Err(ShipRefusal::NotListed)
        );
        assert_eq!(session.pilot(), before.pilot());
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_pilot_loaded_docked_rolls_afresh() {
        let catalog = rolling(50, 50);
        let mut session = outfitted(&catalog);
        let mut firing = Scripted::answering(&[true, true]);
        assert!(lists_speed(&mut session, &mut firing));
        assert!(lists_new(&mut session, &mut firing));
        let text = crate::save::encode(session.pilot());
        assert!(!text.contains("roll"), "no roll is saved: {text}");
        let pilot = crate::save::decode(&text).expect("a pilot");
        let mut resumed = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(resumed.landed(), Some(StellarId(128)));
        let mut missing = Scripted::answering(&[false, false]);
        assert!(!lists_speed(&mut resumed, &mut missing));
        assert!(!lists_new(&mut resumed, &mut missing));
        assert_eq!(missing.asked, [50, 50]);
    }

    /// Fires its `k`-th question, `k` counting from 0, when `k % 100` is
    /// below the percent: over 100 questions of one percent, it fires
    /// exactly that many.
    #[derive(Default)]
    struct Cycle {
        k: u32,
    }

    impl Chance for Cycle {
        fn fires(&mut self, percent: u8) -> bool {
            let fired = self.k % 100 < u32::from(percent);
            self.k += 1;
            fired
        }

        fn below(&mut self, n: u32) -> u32 {
            NeverFires.below(n)
        }
    }

    /// The original draws the rolls once a day (`_IncrementGameTime`
    /// @0xb7c4-0xb833), and no landing path writes them: only when the
    /// draw happens changes, at the take-off's day, not the listing.
    #[test]
    fn about_n_percent_of_days_sell_an_item_of_buy_random_n() {
        for (catalog, expected) in [(rolling(37, 100), (37, 100)), (rolling(100, 62), (100, 62))] {
            let mut session = outfitted(&catalog);
            let mut days = Cycle::default();
            let (mut speeds, mut news) = (0, 0);
            for _ in 0..100 {
                session.take_off(&NoTraffic, &mut days).expect("takes off");
                land_now(&mut session).expect("lands again");
                let mut none = Scripted::default();
                speeds += u32::from(lists_speed(&mut session, &mut none));
                news += u32::from(lists_new(&mut session, &mut none));
                assert!(none.asked.is_empty(), "the day drew it");
            }
            assert_eq!((speeds, news), expected);
            assert_eq!(days.k, 100, "one roll a day");
        }
    }

    #[test]
    fn by_the_other_reading_about_n_percent_of_landings_sell_an_item_of_buy_random_n() {
        let mut session = outfitted(&rolling(37, 62)).with_rules(landing_rolls());
        let (mut outfits, mut ships) = (Cycle::default(), Cycle::default());
        let (mut speeds, mut news) = (0, 0);
        for _ in 0..100 {
            speeds += u32::from(lists_speed(&mut session, &mut outfits));
            news += u32::from(lists_new(&mut session, &mut ships));
            relanded(&mut session);
        }
        assert_eq!((speeds, news), (37, 62));
        assert_eq!((outfits.k, ships.k), (100, 100), "one roll a landing");
    }

    #[test]
    fn the_session_reads_buy_random_as_its_rule_says() {
        let catalog = rolling(0, -1);
        let mut engine = outfitted(&catalog);
        assert_eq!(
            engine.rules().source_for(RuleKey::BuyRandom),
            RuleSource::Engine
        );
        assert!(!lists_speed(&mut engine, &mut NeverFires));
        assert!(lists_new(&mut engine, &mut NeverFires));
        let mut bible = outfitted(&catalog)
            .with_rules(Rulebook::default().with_override(RuleKey::BuyRandom, RuleSource::Bible));
        assert_eq!(
            bible.rules().source_for(RuleKey::BuyRandom),
            RuleSource::Bible
        );
        assert!(lists_speed(&mut bible, &mut NeverFires));
        assert!(!lists_new(&mut bible, &mut NeverFires));
    }

    // Recharging.

    /// A session landed at the exchange's inhabited planet 128 with 1000
    /// credits and half a tank (150 of 300), no save due.
    fn half_empty_at_port() -> Session {
        let mut session = Session::start(&exchange()).expect("starts");
        land_now(&mut session).expect("lands");
        session.pilot.reserves.fuel.now = 150.0;
        session.take_save_due();
        session
    }

    #[test]
    fn recharging_fills_the_tank_charges_the_price_and_makes_a_save_due() {
        let mut session = half_empty_at_port();
        assert_eq!(session.recharge(), Ok(150));
        assert_eq!(session.reserves().fuel, Gauge::full(300.0));
        assert_eq!(session.pilot().cash(), 1000 - 150);
        assert!(session.take_save_due());
    }

    /// Asserts `session.recharge()` is refused with `refusal`, changing
    /// nothing and making no save due.
    fn assert_refused(mut session: Session, refusal: RechargeRefusal) {
        let before = session.clone();
        assert_eq!(session.recharge(), Err(refusal));
        assert_eq!(session, before, "nothing changes");
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_full_tank_is_not_recharged() {
        let mut session = half_empty_at_port();
        session.pilot.reserves.fuel.now = 300.0;
        assert_refused(session, RechargeRefusal::Full);
    }

    #[test]
    fn a_refill_the_pilot_cannot_pay_for_is_refused() {
        let mut session = half_empty_at_port();
        session.pilot.cash = 149;
        assert_refused(session, RechargeRefusal::CannotAfford);
    }

    #[test]
    fn an_uninhabited_stellar_sells_no_fuel() {
        let catalog = FakePilotCatalog {
            sites: vec![(
                SystemId(130),
                vec![LandingSite {
                    flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
                    ..planet(128, 0.0, 0.0)
                }],
            )],
            ..exchange()
        };
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session.pilot.reserves.fuel.now = 150.0;
        session.take_save_due();
        assert_refused(session, RechargeRefusal::NoFuel);
    }
    // Traffic.

    use crate::ai::Peaceful;
    use crate::catalog::{DudeId, DudeRecord, SystemTraffic};
    use crate::testkit::Draws;
    use crate::traffic::autopilot::Outcome;
    use crate::traffic::npc::AiType;

    /// [`catalog`] with `avg` ships on average in `system`, all of düde
    /// 128: ship 129 (fast) with AI `ai_type`, for govt 140.
    fn trafficked(system: i16, avg: i16, ai_type: i16) -> FakePilotCatalog {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        FakePilotCatalog {
            traffic: vec![(
                SystemId(system),
                SystemTraffic {
                    dude_types,
                    avg_ships: avg,
                    persons: Default::default(),
                },
            )],
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type,
                    govt: Some(GovtId(140)),
                    ships: vec![(ShipId(129), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )],
            ship_records: vec![ship(129, FAST)],
            ..catalog()
        }
    }

    #[test]
    fn a_session_starts_with_no_traffic_until_it_is_populated() {
        let catalog = trafficked(130, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.npcs(), []);
        assert_eq!(session.traffic_ships(), []);
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(session.npcs().len(), 2);
        assert_eq!(session.traffic_ships(), [ShipId(129)]);
    }

    #[test]
    fn a_warship_of_no_speed_is_placed_by_the_first_stellar_shown_destroyed_or_not() {
        // Ship 129 of no speed, flown as a warship; stellar 128, the
        // system's first, at (30, -40).
        let mut catalog = trafficked(130, 1, 3);
        catalog.ship_records = vec![ship(129, ShipFields { speed: 0, ..FAST })];
        let catalog = catalog.living([crate::testkit::life(128, 2)]);
        for destroyed in [false, true] {
            let mut session = Session::start(&catalog).expect("starts");
            if destroyed {
                session.run_set(
                    &crate::control::SetExpr::parse("Y128").expect("parses"),
                    &mut NeverFires,
                );
            }
            assert_eq!(
                (session.sites[0].id, session.sites[0].destroyed),
                (StellarId(128), destroyed)
            );
            // At angle 90 from it, facing 45.
            session.populate(&catalog, &mut Draws::of(&[6, 6, 0, 0, 90, 45, 0]));
            let state = session.npcs()[0].state;
            assert!(
                (state.position - Vec2::new(130.0, -40.0)).length() < 1e-3,
                "{destroyed}: {state:?}"
            );
            assert_eq!((state.velocity, state.heading), (Vec2::ZERO, 45.0));
        }
    }

    #[test]
    fn arriving_populates_the_new_system_from_its_dudes() {
        let catalog = trafficked(131, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        let npcs = session.npcs();
        assert_eq!(npcs.len(), 2);
        for npc in npcs {
            assert_eq!(
                (npc.ship, npc.govt, npc.ai_type),
                (ShipId(129), Some(GovtId(140)), AiType::WimpyTrader)
            );
            assert_eq!(npc.stats, ShipStats::new(FAST, &[]));
        }
        assert_eq!(session.traffic_ships(), [ShipId(129)]);
    }

    #[test]
    fn arriving_clears_the_last_systems_traffic() {
        let catalog = trafficked(130, 2, 3);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(session.npcs().len(), 2);
        jump(&mut session, &catalog, 131);
        assert_eq!(session.npcs(), []);
        assert_eq!(session.traffic_ships(), []);
    }

    #[test]
    fn populating_replaces_the_npcs() {
        let catalog = trafficked(130, 3, 3);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        let first: Vec<_> = session.npcs().iter().map(|npc| npc.id).collect();
        session.populate(&catalog, &mut NeverFires);
        let second: Vec<_> = session.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(second.len(), 3);
        assert!(
            first.iter().all(|id| !second.contains(id)),
            "{first:?} {second:?}"
        );
        let mut persons = Draws::of(&[0, 0, 0]);
        session.populate(&catalog, &mut persons);
        assert_eq!(session.npcs(), []);
        assert_eq!(persons.asked, [7, 7, 7]);
    }

    #[test]
    fn traffic_arrives_over_time() {
        let catalog = trafficked(130, 1, 3);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut Draws::of(&[0]));
        assert_eq!(session.npcs(), []);
        session.tick_traffic(&catalog, &Peaceful, &mut Draws::of(&[2]));
        assert_eq!(session.npcs(), []);
        session.tick_traffic(&catalog, &Peaceful, &mut Draws::of(&[0, 6, 6, 0, 0, 0]));
        assert_eq!(session.npcs().len(), 1, "jumping in");
    }

    /// Ticks the traffic until an NPC leaves, and gives how.
    fn first_departure(session: &mut Session, catalog: &FakePilotCatalog) -> Outcome {
        for _ in 0..3000 {
            session.tick_traffic(catalog, &Peaceful, &mut NeverFires);
            if let Some(&(_, outcome)) = session.departed().first() {
                return outcome;
            }
        }
        panic!("nobody left: {:?}", session.npcs());
    }

    #[test]
    fn every_ai_type_lands_peacefully() {
        for ai_type in 1..=4 {
            let how = Outcome::Landed(StellarId(129));
            let catalog = trafficked(130, 1, ai_type);
            let mut session = Session::start(&catalog).expect("starts");
            session.populate(&catalog, &mut NeverFires);
            assert_eq!(first_departure(&mut session, &catalog), how, "AI {ai_type}");
            assert_eq!(session.npcs(), []);
        }
    }

    #[test]
    fn traffic_stands_still_while_landed_or_jumping_past_the_warps_lead_in() {
        let mut catalog = trafficked(130, 1, 1);
        let elsewhere = (SystemId(131), catalog.traffic[0].1);
        catalog.traffic.push(elsewhere);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        land_now(&mut session).expect("lands on planet 128");
        let before = session.npcs().to_vec();
        session.tick_traffic(&catalog, &Peaceful, &mut Draws::of(&[0]));
        assert_eq!(session.npcs(), before, "landed");
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        while session.in_warp_lead_in() {
            session.tick(Controls::default());
        }
        let before = session.npcs().to_vec();
        let mut chance = Draws::of(&[]);
        session.tick_traffic(&catalog, &Peaceful, &mut chance);
        assert_eq!(session.npcs(), before, "jumping, past the lead-in");
        assert!(chance.asked.is_empty());
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        let before = session.npcs().to_vec();
        session.tick_traffic(&catalog, &Peaceful, &mut chance);
        assert_eq!(session.npcs().len(), 1);
        assert_ne!(session.npcs()[0].state, before[0].state, "flying again");
    }

    fn npc_ids(session: &Session) -> Vec<NpcId> {
        session.npcs().iter().map(|npc| npc.id).collect()
    }

    #[test]
    fn a_session_populates_its_system_on_its_first_traffic_tick() {
        let catalog = trafficked(130, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)], "populated once");
    }

    #[test]
    fn the_tick_that_populates_the_system_moves_no_one() {
        let catalog = trafficked(130, 2, 1);
        let mut ticked = Session::start(&catalog).expect("starts");
        ticked.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        let mut populated = Session::start(&catalog).expect("starts");
        populated.populate(&catalog, &mut NeverFires);
        assert_eq!(ticked.npcs(), populated.npcs());
    }

    #[test]
    fn taking_off_repopulates_the_system_before_any_tick() {
        let catalog = trafficked(130, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        land_now(&mut session).expect("lands on planet 128");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)], "landed");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("took off");
        assert_eq!(npc_ids(&session), [NpcId(2), NpcId(3)], "as it takes off");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(2), NpcId(3)], "populated once");
    }

    /// A draw a [`QueuedLog`] chance was asked for.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Asked {
        Fires(u8),
        Below(u32),
    }

    /// A [`Chance`] that takes every draw, percentages too, from one
    /// queue (each `fires` a draw below 100), wrapped into range, the last
    /// outcome once it runs out, and logs each draw asked in order.
    #[derive(Debug, Default)]
    struct QueuedLog {
        queue: std::collections::VecDeque<u32>,
        log: Vec<Asked>,
    }

    impl QueuedLog {
        fn of(draws: &[u32]) -> Self {
            Self {
                queue: draws.iter().copied().collect(),
                log: Vec::new(),
            }
        }

        fn draw(&mut self, n: u32) -> u32 {
            self.queue.pop_front().map_or(n - 1, |draw| draw % n)
        }
    }

    impl Chance for QueuedLog {
        fn fires(&mut self, percent: u8) -> bool {
            self.log.push(Asked::Fires(percent));
            self.draw(100) < u32::from(percent)
        }

        fn below(&mut self, n: u32) -> u32 {
            self.log.push(Asked::Below(n));
            self.draw(n)
        }
    }

    #[test]
    fn taking_off_rolls_the_days_then_populates() {
        let mut catalog = trafficked(130, 2, 1);
        catalog.ship_records[0].hire_random = 30;
        catalog.ship_records[0].buy_random = 40;
        let spawns = [5, 11, 3, 17, 7, 23, 2, 13, 19, 29, 31, 37, 41];
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands on planet 128");
        // The day's two rolls, the escort-strength seed, then the spawns.
        let mut chance = QueuedLog::of(&[[99, 99, 0].as_slice(), &spawns].concat());
        session.take_off(&catalog, &mut chance).expect("took off");
        assert_eq!(
            chance.log[..3],
            [Asked::Fires(30), Asked::Fires(40), Asked::Below(30)],
            "the day's hire then buy roll first, then the seed"
        );
        assert!(chance.log.len() > 2, "then the population draws");
        assert!(
            chance.log[2..]
                .iter()
                .all(|asked| matches!(asked, Asked::Below(_))),
            "{:?}",
            chance.log
        );

        let mut populated = Session::start(&catalog).expect("starts");
        populated.populate(&catalog, &mut QueuedLog::of(&spawns));
        let at = |session: &Session| {
            session
                .npcs()
                .iter()
                .map(|npc| npc.state.position)
                .collect::<Vec<_>>()
        };
        assert!(!at(&session).is_empty());
        assert_eq!(at(&session), at(&populated), "placed by the later draws");
    }

    #[test]
    fn a_ship_landed_from_the_start_populates_once_it_takes_off() {
        let catalog = trafficked(130, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands on planet 128");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [], "landed");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("took off");
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)], "as it takes off");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)], "populated once");
    }

    #[test]
    fn a_system_populated_on_arrival_is_not_populated_again_on_its_first_tick() {
        let catalog = trafficked(131, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
    }

    #[test]
    fn a_populated_system_is_not_populated_again_on_its_first_tick() {
        let catalog = trafficked(130, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
    }

    // Fleets.

    use crate::catalog::{EscortRecord, FleetId, FleetRecord};

    /// `flët` `id`, linked by `link_syst`: a ship 129 lead and 1-3 ship
    /// 130 escorts, for govt 150.
    fn fleet(id: i16, link_syst: i16) -> FleetRecord {
        FleetRecord {
            id: FleetId(id),
            lead: Some(ShipId(129)),
            escorts: vec![EscortRecord {
                ship: ShipId(130),
                min: 1,
                max: 3,
            }],
            govt: Some(GovtId(150)),
            link_syst,
            appear_on: Test::default(),
        }
    }

    /// The `LinkSyst` of the systems government `govt` governs.
    fn govt_link(govt: i16) -> i16 {
        10_000 + (govt - 128)
    }

    /// [`trafficked`] with one ship on average in 130 and 131, governed
    /// on the star map by 140 and 141, and `flët`s 130 (slot 2), linked to
    /// govt 140, and 131 (slot 3), linked to govt 141.
    fn fleeted() -> FakePilotCatalog {
        let mut catalog = trafficked(130, 1, 3);
        let elsewhere = (SystemId(131), catalog.traffic[0].1);
        catalog.traffic.push(elsewhere);
        catalog.star_map[0].govt = Some(GovtId(140));
        catalog.star_map[1].govt = Some(GovtId(141));
        catalog.fleets = vec![fleet(130, govt_link(140)), fleet(131, govt_link(141))];
        catalog.ship_records.push(ship(130, FAST));
        catalog
    }

    /// A setup pass that draws the `LinkSyst` fleet in `slot`, with three
    /// escorts.
    fn linked_fleet_draws(slot: u32) -> Draws {
        Draws::of(&[1, 0, slot, 0, 2])
    }

    /// Each NPC's ship, government and leader.
    fn fleet_of(session: &Session) -> Vec<(ShipId, Option<GovtId>, Option<NpcId>)> {
        session
            .npcs()
            .iter()
            .map(|npc| (npc.ship, npc.govt, npc.leader))
            .collect()
    }

    /// A [`fleet`]'s lead, as NPC `lead`, and its three escorts.
    fn a_fleet_led_by(lead: u32) -> Vec<(ShipId, Option<GovtId>, Option<NpcId>)> {
        let escort = (ShipId(130), Some(GovtId(150)), Some(NpcId(lead)));
        vec![
            (ShipId(129), Some(GovtId(150)), None),
            escort,
            escort,
            escort,
        ]
    }

    #[test]
    fn populating_brings_in_the_fleet_linked_to_the_systems_government() {
        let catalog = fleeted();
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut linked_fleet_draws(2));
        assert_eq!(fleet_of(&session), a_fleet_led_by(0));
        session.populate(&catalog, &mut linked_fleet_draws(3));
        assert_eq!(fleet_of(&session), [], "linked to another government");
    }

    #[test]
    fn populating_tests_a_fleets_appear_on_through_the_sessions_control_bits() {
        let mut catalog = fleeted();
        catalog.fleets[0].appear_on = Test::parse("b7");
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut linked_fleet_draws(2));
        assert_eq!(fleet_of(&session), [], "bit 7 is clear");
        session.set_control_bit(crate::control::Bit::new(7).expect("a bit"), true);
        session.populate(&catalog, &mut linked_fleet_draws(2));
        assert_eq!(fleet_of(&session), a_fleet_led_by(0));
        let mut refusing = session.with_control_bits(Rc::new(crate::testkit::RefuseBits(&[7])));
        refusing.populate(&catalog, &mut linked_fleet_draws(2));
        assert_eq!(fleet_of(&refusing), []);
    }

    #[test]
    fn arriving_brings_in_the_fleet_linked_to_the_new_systems_government() {
        let catalog = fleeted();
        let mut session = Session::start(&catalog).expect("starts");
        // The escort-strength seed, then the roll table and the fleet.
        let mut seeded: Vec<u32> = vec![0];
        seeded.extend(std::iter::repeat_n(99, crate::mission::ROLLED));
        seeded.extend([1, 0, 3, 0, 2]);
        jump_with(&mut session, &catalog, 131, &mut Draws::of(&seeded));
        assert_eq!(fleet_of(&session), a_fleet_led_by(0));
        session.populate(&catalog, &mut linked_fleet_draws(2));
        assert_eq!(fleet_of(&session), [], "linked to another government");
    }

    #[test]
    fn a_fleet_the_systems_dude_types_name_arrives_when_its_roll_fires() {
        let mut catalog = trafficked(130, 1, 3);
        catalog.traffic[0].1.dude_types[1] = (-132, 30);
        catalog.fleets = vec![fleet(132, 0)];
        catalog.ship_records.push(ship(130, FAST));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut Draws::of(&[0]));
        assert_eq!(fleet_of(&session), [], "a person");
        session.tick_traffic(&catalog, &Peaceful, &mut Draws::of(&[1, 30, 0]));
        assert_eq!(fleet_of(&session), [], "31 is above its 30%");
        session.tick_traffic(&catalog, &Peaceful, &mut Draws::of(&[1, 29, 0, 0, 2]));
        assert_eq!(fleet_of(&session), a_fleet_led_by(0));
    }

    /// `catalog` with a hypergate of `govt` at (300, -400) in system 130,
    /// heading ships out on 90°, and govt 150 (class 1) at war with 160
    /// (class 2).
    fn with_hypergate(mut catalog: FakePilotCatalog, govt: Option<GovtId>) -> FakePilotCatalog {
        catalog.sites[0].1.push(LandingSite {
            flags2: crate::gate::HYPERGATE,
            exit_angle: 90,
            govt,
            ..planet(141, 300.0, -400.0)
        });
        catalog.govts = vec![
            crate::catalog::GovtRecord {
                classes: [1, -1, -1, -1],
                enemies: [2, -1, -1, -1],
                ..crate::testkit::govt(150)
            },
            crate::catalog::GovtRecord {
                classes: [2, -1, -1, -1],
                ..crate::testkit::govt(160)
            },
        ];
        catalog
    }

    #[test]
    fn traffic_arriving_among_a_hypergate_may_come_out_of_it() {
        let catalog = with_hypergate(trafficked(130, 1, 3), None);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut Draws::of(&[0]));
        assert_eq!(session.npcs(), []);
        // An arrival, then the gate: 1 of planet 128 and the gate, planet
        // 129 being 2000 out.
        let mut chance = Draws::of(&[0, 6, 6, 0, 0, 0, 1]);
        session.tick_traffic(&catalog, &Peaceful, &mut chance);
        assert_eq!(chance.asked, [500, 7, 7, 100, 1, 360, 2, 3]);
        let npc = &session.npcs()[0];
        assert!(npc.emerging(), "{:?}", npc.mode);
        assert_eq!(npc.state.position, Vec2::new(300.0, -400.0));
    }

    #[test]
    fn populating_judges_a_fleets_gate_by_the_sessions_governments() {
        // Fleet 130 (govt 150) at setup: by a gate of independents it may
        // come out of the gate; by govt 160's, at war with it, it never
        // does, and nothing is drawn for the pick.
        let catalog = with_hypergate(fleeted(), None);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut Draws::of(&[1, 0, 2, 1, 2]));
        assert_eq!(fleet_of(&session), a_fleet_led_by(0));
        assert!(session.npcs().iter().all(Npc::emerging));
        let catalog = with_hypergate(fleeted(), Some(GovtId(160)));
        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = linked_fleet_draws(2);
        session.populate(&catalog, &mut chance);
        assert_eq!(fleet_of(&session), a_fleet_led_by(0));
        assert_eq!(&chance.asked[..4], [7, 7, 256, 360], "jumping in");
        assert!(session.npcs().iter().all(|npc| !npc.emerging()));
    }

    // Combat.

    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::catalog::{GovtRecord, HullRecord, StockWeapon, WeaponRecord};
    use crate::combat::armament::MOD_AMMO;
    use crate::combat::defence::Allegiance;
    use crate::combat::hull::DisableRule;
    use crate::combat::weapon::Explosion;
    use crate::legal::{Crime, LegalCode, NovaLaw};
    use crate::rulebook::RuleSource;
    use crate::testkit::{hull, weapon};
    use crate::traffic::npc::Npc;

    /// A blaster firing every other tick, 20 pixels a tick for 30 ticks,
    /// doing 5 mass and 10 energy damage.
    fn blaster() -> WeaponRecord {
        WeaponRecord {
            reload: 2,
            count: 30,
            speed: 2000,
            mass_dmg: 5,
            energy_dmg: 10,
            ..weapon(128)
        }
    }

    /// Ship `id` armed with one of `weapon`, 30 pixels across, of
    /// `DeathDelay` 2: an NPC breaks up for a tick before `bööm` 133
    /// destroys it, the player for 4.
    fn armed_hull(id: i16, weapon: i16) -> HullRecord {
        HullRecord {
            weapons: vec![StockWeapon {
                weapon: WeaponId(weapon),
                count: 1,
                ammo: 0,
            }],
            death_delay: 2,
            explode2: 5,
            size: Some(30),
            ..hull(id)
        }
    }

    /// [`trafficked`] with the player (ship 128) and its traffic (ship
    /// 129, a trader: 30 shield, 45 armour) each carrying a blaster, held
    /// by outfit 250.
    pub(super) fn armed() -> FakePilotCatalog {
        FakePilotCatalog {
            weapons: vec![blaster()],
            hulls: vec![armed_hull(128, 128), armed_hull(129, 128)],
            outfits: vec![outfit(250, &[(MOD_WEAPON, 128)])],
            ..trafficked(130, 1, 1)
        }
    }

    /// `catalog`'s session with its one NPC placed 100 pixels above the
    /// player (at the centre, facing up), facing `heading`, at rest.
    pub(super) fn facing_an_npc(catalog: &FakePilotCatalog, heading: u32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.populate(catalog, &mut Draws::of(&[6, 6, 0, 0, 750, 650, heading]));
        assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, -100.0));
        stop_npcs(&mut session);
        session
    }

    const FIRE: Trigger = Trigger {
        primary: true,
        secondary: None,
        only: None,
        turrets_only: false,
        bays: false,
        point_defence: false,
    };

    /// The fight's events about NPC 0, other than its firing.
    fn about_the_npc(events: &[CombatEvent]) -> Vec<CombatEvent> {
        events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    CombatEvent::Disabled { .. }
                        | CombatEvent::BreakingUp { .. }
                        | CombatEvent::Destroyed { .. }
                )
            })
            .copied()
            .collect()
    }

    #[test]
    fn the_players_shots_pass_through_its_escort_and_the_escorts_through_the_player() {
        // The escort 100 above the player, facing it: each fires at the
        // other point-blank.
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let escort = &mut session.traffic.npcs_mut()[0];
        escort.escort = Some(crate::escort::EscortDuty {
            formation: FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            },
            order: None,
        });
        escort.trigger = FIRE;
        escort.target = Some(ShipRef::Player);
        session.hold_trigger(FIRE);
        session.target = Some(NpcId(0));
        let mut fired = 0;
        for _ in 0..60 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            fired += session
                .take_combat_events()
                .iter()
                .filter(|event| matches!(event, CombatEvent::Fired { .. }))
                .count();
        }
        assert!(fired > 20, "both fired: {fired}");
        assert_eq!(
            session.npcs()[0].reserves,
            Reserves::full(30.0, 45.0, 300.0)
        );
        assert_eq!(session.reserves().shield.now, 30.0, "untouched");
        assert_eq!(session.reserves().armor.now, 45.0);
    }

    /// The fight's 60 ticks of the player and NPC 0, 100 above it, firing
    /// at each other point-blank, NPC 0 marked by slot 0 as `formation`
    /// says: the NPC's shield and the player's after them.
    fn exchange_with_a_mission_ship(formation: Option<FormationSlot>) -> (f32, f32) {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let ship = &mut session.traffic.npcs_mut()[0];
        ship.mission = Some(crate::traffic::npc::MissionMark {
            order: None,
            slot: 0,
            behav: crate::mission::ships::ShipBehav::EscortPlayer,
            formation,
        });
        ship.trigger = FIRE;
        ship.target = Some(ShipRef::Player);
        session.hold_trigger(FIRE);
        session.target = Some(NpcId(0));
        for _ in 0..60 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        let npc = session
            .npcs()
            .first()
            .map_or(0.0, |npc| npc.reserves.shield.now);
        (npc, session.reserves().shield.now)
    }

    #[test]
    fn the_players_shots_pass_through_a_mission_escort_but_not_a_mission_ship_yet_to_join() {
        let joined = FormationSlot {
            slot: 2,
            ships: 2,
            spacing: 30.0,
        };
        assert_eq!(
            exchange_with_a_mission_ship(Some(joined)),
            (30.0, 30.0),
            "untouched"
        );
        let (npc, player) = exchange_with_a_mission_ship(None);
        assert!(npc < 30.0 && player < 30.0, "{npc} {player}");
    }

    #[test]
    fn the_players_trigger_fires_its_stock_weapon_and_disables_then_destroys_an_npc() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let mut gauges = Vec::new();
        let mut events = Vec::new();
        for _ in 0..60 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
            let Some(npc) = session.npcs().first() else {
                break;
            };
            gauges.push((
                npc.reserves.shield.now,
                npc.reserves.armor.now,
                npc.condition,
            ));
            if npc.condition == Condition::Disabled {
                break;
            }
        }
        let last = *gauges.last().expect("ticks");
        assert_eq!(last, (-3.0, 10.0, Condition::Disabled), "{gauges:?}");
        let first_armour = gauges.iter().position(|g| g.1 < 45.0).expect("hit");
        assert!(gauges[first_armour].0 <= 0.0, "shields first: {gauges:?}");
        assert!(
            gauges[..first_armour]
                .iter()
                .any(|g| g.0 > 0.0 && g.0 < 30.0)
        );
        assert_eq!(
            about_the_npc(&events),
            [CombatEvent::Disabled {
                ship: ShipRef::Npc(NpcId(0))
            }]
        );
        assert!(events.contains(&CombatEvent::Fired {
            ship: ShipRef::Player,
            weapon: WeaponId(128),
            at: Vec2::ZERO
        }));
        assert_eq!(session.npcs().len(), 1, "disabled, and still alive");
        for _ in 0..60 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        let ending: Vec<_> = about_the_npc(&events);
        assert!(
            matches!(
                ending[..],
                [
                    CombatEvent::Disabled { .. },
                    CombatEvent::BreakingUp { .. },
                    CombatEvent::Destroyed {
                        ship: ShipRef::Npc(NpcId(0)),
                        ship_type: ShipId(129),
                        explosion: Some(Explosion { .. }),
                        size: 0.0,
                        ..
                    }
                ]
            ),
            "{ending:?}"
        );
        assert_eq!(session.npcs(), [], "gone once destroyed");
        assert_eq!(session.player_condition(), Condition::Intact);
    }

    /// Every NPC idles and holds its trigger.
    #[derive(Debug)]
    pub(super) struct Firing;

    impl Behaviour for Firing {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }

        fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
            FIRE
        }
    }

    #[test]
    fn an_npcs_trigger_fires_at_the_player_through_the_same_fight() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let mut events = Vec::new();
        for _ in 0..12 {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        assert!(events.contains(&CombatEvent::Fired {
            ship: ShipRef::Npc(NpcId(0)),
            weapon: WeaponId(128),
            at: Vec2::new(0.0, -100.0)
        }));
        assert!(
            session.reserves().shield.now < 30.0,
            "{:?}",
            session.reserves()
        );
        assert!(
            session
                .shots()
                .iter()
                .all(|shot| shot.firer == ShipRef::Npc(NpcId(0))),
            "the player held no trigger"
        );
    }

    #[test]
    fn the_players_ammunition_outfits_and_fuel_pay_for_its_shots() {
        let rocket = WeaponRecord {
            ammo_type: 10,
            ..blaster()
        };
        let mut catalog = FakePilotCatalog {
            weapons: vec![WeaponRecord {
                id: WeaponId(138),
                ..rocket
            }],
            hulls: vec![armed_hull(128, 138)],
            outfits: vec![
                outfit(300, &[(MOD_AMMO, 138)]),
                outfit(250, &[(MOD_WEAPON, 138)]),
                outfit(251, &[(MOD_WEAPON, 128)]),
            ],
            defaults: vec![(ShipId(128), vec![(OutfitId(300), 2)])],
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        for _ in 0..8 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(
            session.pilot().outfits().collect::<Vec<_>>(),
            [(OutfitId(250), 1)],
            "both rockets fired"
        );
        assert_eq!(session.shots().len(), 2);
        let fuelled = WeaponRecord {
            ammo_type: -1100,
            ..blaster()
        };
        catalog.weapons = vec![fuelled];
        catalog.hulls = vec![armed_hull(128, 128)];
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.reserves().fuel.now, 290.0, "10 units a shot");
    }

    #[test]
    fn a_beam_weapon_fires_beams() {
        let laser = WeaponRecord {
            guidance: 0,
            count: 5,
            beam_length: 100,
            ..blaster()
        };
        let catalog = FakePilotCatalog {
            weapons: vec![laser],
            ..armed()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.beams().len(), 1);
        assert_eq!(session.beams()[0].firer, ShipRef::Player);
        assert_eq!(session.shots(), []);
    }

    #[test]
    fn the_fight_stands_still_while_landed_or_jumping() {
        let mut catalog = armed();
        catalog.traffic.push((SystemId(131), catalog.traffic[0].1));
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.shots().len(), 1);
        land_now(&mut session).expect("lands on planet 128");
        assert_eq!(session.shots(), [], "gone on landing");
        session.take_combat_events();
        let mut chance = Draws::of(&[]);
        for _ in 0..3 {
            session.tick_combat(Rules::default(), &mut chance);
        }
        assert_eq!(session.take_combat_events(), [], "landed, though reloaded");
        assert_eq!(session.shots(), []);
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut chance);
        assert_eq!(session.shots().len(), 1);
        begin_jump_now(&mut session).expect("jumps");
        session.take_combat_events();
        for _ in 0..3 {
            session.tick_combat(Rules::default(), &mut chance);
        }
        assert_eq!(session.take_combat_events(), [], "jumping, though reloaded");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.shots(), [], "gone on arriving");
        assert!(chance.asked.is_empty());
    }

    /// Disables every ship.
    #[derive(Debug)]
    struct Disabling;

    impl DisableRule for Disabling {
        fn disabled(&self, _armor: Gauge, _hull: &HullSpec) -> bool {
            true
        }
    }

    pub(super) const DISABLING: Rules = Rules {
        disable: &Disabling,
        defence: &Allegiance,
        law: &NovaLaw {
            crime_gains: RuleSource::Engine,
        },
    };

    // What NPCs see.

    /// What an NPC sees: the player, the system's government, the record
    /// there, and how many governments there are.
    type Seen = (Option<crate::ai::PlayerSide>, Option<GovtId>, i16, usize);

    /// Records what each NPC sees as it decides, and each strike it
    /// answers; decides nothing new.
    #[derive(Debug, Default)]
    struct Spy {
        seen: std::cell::RefCell<Vec<Seen>>,
        struck: std::cell::RefCell<Vec<crate::combat::Strike>>,
    }

    impl Behaviour for Spy {
        fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            self.seen.borrow_mut().push((
                around.player,
                around.system_govt,
                around.record,
                around.govts.ids().count(),
            ));
            npc.goal
        }

        fn react(
            &self,
            _npc: &Npc,
            strike: &crate::combat::Strike,
            _around: &Surroundings,
        ) -> crate::ai::Reaction {
            self.struck.borrow_mut().push(*strike);
            crate::ai::Reaction::default()
        }
    }

    #[test]
    fn npcs_see_the_player_the_governments_and_the_record_in_the_system() {
        let mut catalog = armed();
        catalog.star_map[0].govt = Some(GovtId(150));
        catalog.govts = vec![crate::testkit::govt(150), crate::testkit::govt(151)];
        let mut session = facing_an_npc(&catalog, 180);
        with_record(&mut session, GovtId(150), -20);
        with_record(&mut session, GovtId(151), 30);
        let spy = Spy::default();
        session.tick_traffic(&catalog, &spy, &mut NeverFires);
        let seen = spy.seen.take();
        assert_eq!(seen.len(), 1);
        let (player, system, record, govts) = seen[0];
        let player = player.expect("the player flies here");
        assert_eq!(player.state, *session.player());
        assert_eq!(player.condition, Condition::Intact);
        assert_eq!(player.reserves, session.reserves());
        assert_eq!(player.hull, session.hull());
        assert_eq!(player.handling, session.handling());
        assert_eq!((system, record, govts), (Some(GovtId(150)), -20, 2));
        catalog.star_map[0].govt = None;
        let mut session = facing_an_npc(&catalog, 180);
        with_record(&mut session, GovtId(150), -20);
        session.tick_traffic(&catalog, &spy, &mut NeverFires);
        let (_, system, record, _) = spy.seen.take()[0];
        assert_eq!((system, record), (None, 0), "an independent system");
    }

    #[test]
    fn by_the_engine_npcs_see_the_record_in_the_system() {
        let mut catalog = armed();
        catalog.star_map[0].govt = Some(GovtId(150));
        catalog.govts = vec![crate::testkit::govt(150)];
        let mut session = facing_an_npc(&catalog, 180);
        session.pilot.set_legal_record(GovtId(150), 30);
        session.pilot.set_system_record(SystemId(130), -20);
        let spy = Spy::default();
        session.tick_traffic(&catalog, &spy, &mut NeverFires);
        let (_, _, record, _) = spy.seen.take()[0];
        assert_eq!(record, -20, "the system's own");
        let mut session = session.with_rules(bible_records());
        session.tick_traffic(&catalog, &spy, &mut NeverFires);
        let (_, _, record, _) = spy.seen.take()[0];
        assert_eq!(record, 30, "by the Bible, 150's");
    }

    #[test]
    fn npcs_answer_the_fights_strikes_on_the_next_traffic_tick_once() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let spy = Spy::default();
        let mut hits = 0;
        for _ in 0..30 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            session.tick_traffic(&catalog, &spy, &mut NeverFires);
            let struck = spy.struck.take();
            hits += struck.len();
            assert!(struck.len() <= 1, "each strike once: {struck:?}");
            for strike in struck {
                assert_eq!(
                    (strike.ship, strike.by),
                    (ShipRef::Npc(NpcId(0)), ShipRef::Player)
                );
            }
        }
        assert!(hits > 3, "{hits}");
        session.tick_combat(Rules::default(), &mut NeverFires);
        land_now(&mut session).expect("lands");
        session.take_off(&catalog, &mut NeverFires);
        session.tick_traffic(&catalog, &spy, &mut NeverFires);
        assert_eq!(spy.struck.take(), [], "gone with the system's traffic");
    }

    // Crimes.

    /// The NPCs' government 140 (class 1: disabling costs 3, killing 7),
    /// its ally 141, its enemy 142 (disabling 8, killing 10) and a
    /// neutral, 143.
    fn lawful_govts() -> Vec<GovtRecord> {
        let penalised = |id, disable, kill| GovtRecord {
            penalties: crate::catalog::Penalties {
                disable,
                kill,
                ..crate::catalog::Penalties::default()
            },
            ..crate::testkit::govt(id)
        };
        vec![
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..penalised(140, 3, 7)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                ..penalised(141, 20, 20)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..penalised(142, 8, 10)
            },
            penalised(143, 20, 20),
        ]
    }

    /// [`armed`], with [`lawful_govts`].
    fn lawful() -> FakePilotCatalog {
        FakePilotCatalog {
            govts: lawful_govts(),
            ..armed()
        }
    }

    /// The pilot's records with governments 140-143.
    fn records(session: &Session) -> [i16; 4] {
        [140, 141, 142, 143].map(|govt| session.pilot().legal_record(GovtId(govt)))
    }

    /// Records each crime it is asked about, and the victim's
    /// government, and changes nothing.
    #[derive(Debug, Default)]
    struct Witness {
        seen: std::cell::RefCell<Vec<(Crime, Option<GovtId>)>>,
    }

    impl LegalCode for Witness {
        fn penalties(
            &self,
            crime: Crime,
            victim: Option<GovtId>,
            _govts: &Governments,
        ) -> Vec<(GovtId, i32)> {
            self.seen.borrow_mut().push((crime, victim));
            Vec::new()
        }
    }

    #[test]
    fn disabling_an_npc_lowers_the_records_as_the_law_says_once_and_breaking_it_up_again() {
        let catalog = lawful();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let mut seen = Vec::new();
        for _ in 0..80 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            let condition = session.npcs().first().map(|npc| npc.condition);
            if seen.last().is_none_or(|(last, _)| *last != condition) {
                seen.push((condition, records(&session)));
            }
        }
        assert_eq!(
            seen,
            [
                (Some(Condition::Intact), [0, 0, 0, 0]),
                (Some(Condition::Disabled), [-3, -3, 4, 10]),
                (
                    Some(Condition::Dying {
                        ticks_left: 2,
                        starting: true
                    }),
                    [-10, -10, 9, 20]
                ),
                (None, [-10, -10, 9, 20]),
            ],
            "hits that leave it intact change nothing; by the engine's law, the neutral gains"
        );
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("reads");
        assert_eq!(&saved, session.pilot(), "the records saved as they are");
    }

    #[test]
    fn disabling_an_npc_changes_the_records_in_each_system_of_a_changed_government() {
        let mut catalog = lawful();
        for (system, govt) in catalog
            .star_map
            .iter_mut()
            .zip([Some(140), Some(142), None])
        {
            system.govt = govt.map(GovtId);
        }
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let systems = |session: &Session| {
            [130, 131, 132].map(|system| session.pilot().system_record(SystemId(system)))
        };
        for _ in 0..80 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            if session.npcs().first().map(|npc| npc.condition) == Some(Condition::Disabled) {
                break;
            }
        }
        assert_eq!(
            systems(&session),
            [-3, 4, 0],
            "the victim's system and its enemy's, each its government's change; \
             the independent untouched"
        );
    }

    #[test]
    fn the_law_hears_of_a_disabling_then_a_killing_against_the_victims_government() {
        let catalog = lawful();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let witness = Witness::default();
        let rules = Rules {
            law: &witness,
            ..Rules::default()
        };
        for _ in 0..80 {
            session.tick_combat(rules, &mut NeverFires);
        }
        assert_eq!(session.npcs(), [], "destroyed");
        assert_eq!(
            witness.seen.take(),
            [
                (Crime::Disable, Some(GovtId(140))),
                (Crime::Kill, Some(GovtId(140)))
            ]
        );
        assert_eq!(records(&session), [0; 4], "as the witness says");
    }

    #[test]
    fn breaking_up_an_intact_ship_at_once_is_disabling_it_and_killing_it() {
        let catalog = FakePilotCatalog {
            weapons: vec![WeaponRecord {
                mass_dmg: 500,
                energy_dmg: 500,
                ..blaster()
            }],
            ..lawful()
        };
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let witness = Witness::default();
        let rules = Rules {
            law: &witness,
            ..Rules::default()
        };
        for _ in 0..20 {
            session.tick_combat(rules, &mut NeverFires);
            session.hold_trigger(Trigger::default());
        }
        assert_eq!(
            witness.seen.take(),
            [
                (Crime::Disable, Some(GovtId(140))),
                (Crime::Kill, Some(GovtId(140)))
            ],
            "as the original's _DamageShip slaps both"
        );
    }

    /// How NPC 0 is a mission's special ship, if it is: a mark of slot 0
    /// holding a mission, unfailed or failed, or a stale mark of slot 5,
    /// free.
    #[derive(Clone, Copy, Debug)]
    enum Marked {
        Active,
        Failed,
        Stale,
    }

    /// Marks NPC 0 of `session` as `marked` says, filling slot 0 with
    /// mission 140 for a mark of it.
    fn mark_the_npc(session: &mut Session, marked: Marked) {
        let slot = match marked {
            Marked::Active | Marked::Failed => {
                let mut active = crate::testkit::carrying(140, 0);
                if matches!(marked, Marked::Failed) {
                    active.failed = Some(crate::mission::FailedBy::Marked);
                }
                assert!(session.pilot.missions.put(0, active));
                0
            }
            Marked::Stale => 5,
        };
        session.traffic.npcs_mut()[0].mission = Some(crate::traffic::npc::MissionMark {
            order: None,
            slot,
            behav: crate::mission::ships::ShipBehav::Own,
            formation: None,
        });
    }

    const MARKS: [Marked; 3] = [Marked::Active, Marked::Failed, Marked::Stale];

    /// [`lawful`]'s NPC, marked as `marked` says, shot at until it is
    /// destroyed, by the weapons of `catalog`, fire held for the first
    /// tick only when `once`: the crimes a [`Witness`] heard, then the
    /// records by the engine's law.
    fn crimes_against(
        catalog: &FakePilotCatalog,
        marked: Option<Marked>,
        once: bool,
    ) -> (Vec<(Crime, Option<GovtId>)>, [i16; 4]) {
        let fight = |law: &dyn LegalCode| {
            let mut session = facing_an_npc(catalog, 180);
            if let Some(marked) = marked {
                mark_the_npc(&mut session, marked);
            }
            session.hold_trigger(FIRE);
            let rules = Rules {
                law,
                ..Rules::default()
            };
            for _ in 0..80 {
                session.tick_combat(rules, &mut NeverFires);
                if once {
                    session.hold_trigger(Trigger::default());
                }
            }
            assert_eq!(session.npcs(), [], "destroyed");
            records(&session)
        };
        let witness = Witness::default();
        fight(&witness);
        (
            witness.seen.take(),
            fight(&crate::legal::NovaLaw::default()),
        )
    }

    #[test]
    fn disabling_and_destroying_a_mission_ship_changes_no_record() {
        let catalog = lawful();
        assert_eq!(
            crimes_against(&catalog, None, false),
            (
                vec![
                    (Crime::Disable, Some(GovtId(140))),
                    (Crime::Kill, Some(GovtId(140)))
                ],
                [-10, -10, 9, 20]
            ),
            "the same ship of no mission"
        );
        for marked in MARKS {
            assert_eq!(
                crimes_against(&catalog, Some(marked), false),
                (vec![], [0; 4]),
                "{marked:?}: any mark escapes the penalty, as _SlapWithPenalty \
                 slaps only for a mission slot of -1 (@0x9c10)"
            );
        }
    }

    #[test]
    fn breaking_up_an_intact_mission_ship_at_once_changes_no_record() {
        let catalog = FakePilotCatalog {
            weapons: vec![WeaponRecord {
                mass_dmg: 500,
                energy_dmg: 500,
                ..blaster()
            }],
            ..lawful()
        };
        assert_eq!(crimes_against(&catalog, None, true).0.len(), 2);
        for marked in MARKS {
            assert_eq!(
                crimes_against(&catalog, Some(marked), true),
                (vec![], [0; 4]),
                "{marked:?}"
            );
        }
    }

    /// [`lawful`] with ranks 200 (of 141, allied with the NPC's 140) and
    /// 201 (of 143, neutral), each lost to disabling or destroying, and
    /// 202 (of 141), lost to any crime.
    fn ranked_lawful() -> FakePilotCatalog {
        let lost = |id, govt, flags| RankRecord {
            affil_govt: Some(GovtId(govt)),
            flags,
            ..crate::testkit::rank(id)
        };
        FakePilotCatalog {
            ranks: vec![
                lost(200, 141, crate::rank::LOST_TO_VIOLENCE),
                lost(201, 143, crate::rank::LOST_TO_VIOLENCE),
                lost(202, 141, crate::rank::LOST_TO_CRIME),
            ],
            ..lawful()
        }
    }

    /// The session's active ranks.
    fn ranks_held(session: &Session) -> Vec<i16> {
        session.pilot().ranks().iter().map(|id| id.0).collect()
    }

    /// [`ranked_lawful`]'s session by `rules`, holding all its ranks,
    /// its NPC marked as `marked` says, the player firing at it.
    fn fighting_ranked(marked: Option<Marked>, rules: Rulebook) -> Session {
        let catalog = ranked_lawful();
        let mut session = facing_an_npc(&catalog, 180).with_rules(rules);
        if let Some(marked) = marked {
            mark_the_npc(&mut session, marked);
        }
        session
            .pilot
            .ranks
            .extend([RankId(200), RankId(201), RankId(202)]);
        session.hold_trigger(FIRE);
        session
    }

    /// Fights until NPC 0 is disabled, at most 80 ticks.
    fn fight_until_disabled(session: &mut Session) {
        for _ in 0..80 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            if session.npcs().first().map(|npc| npc.condition) == Some(Condition::Disabled) {
                return;
            }
        }
        panic!("never disabled");
    }

    #[test]
    fn disabling_and_destroying_an_npc_takes_the_ranks_they_reach() {
        let mut session = fighting_ranked(None, Rulebook::default());
        fight_until_disabled(&mut session);
        assert_eq!(
            ranks_held(&session),
            [201],
            "a disabling reaches its ally's ranks, not a neutral's"
        );
        session.pilot.ranks.insert(RankId(200));
        for _ in 0..80 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(session.npcs(), [], "destroyed");
        assert_eq!(ranks_held(&session), [201], "and so does a killing");
    }

    #[test]
    fn downing_a_mission_ship_takes_no_rank() {
        for marked in MARKS {
            let mut session = fighting_ranked(Some(marked), Rulebook::default());
            for _ in 0..80 {
                session.tick_combat(Rules::default(), &mut NeverFires);
            }
            assert_eq!(session.npcs(), [], "{marked:?}: destroyed");
            assert_eq!(ranks_held(&session), [200, 201, 202], "{marked:?}: @0x9c10");
        }
    }

    #[test]
    fn by_the_other_reading_of_rank_crime_reach_an_allied_victim_keeps_a_0x40_rank() {
        let mut session = fighting_ranked(
            None,
            Rulebook::default().with_override(RuleKey::RankCrimeReach, RuleSource::Bible),
        );
        fight_until_disabled(&mut session);
        assert_eq!(
            ranks_held(&session),
            [201, 202],
            "0x0004 reaches the allies, 0x0040 only its own government"
        );
    }

    /// What downing [`crimes_against_a_failing_escort`]'s ship did.
    #[derive(Debug, PartialEq)]
    struct EscortDowned {
        /// The crimes a [`Witness`] heard.
        crimes: Vec<(Crime, Option<GovtId>)>,
        /// The records by the engine's law.
        records: [i16; 4],
        /// Whether slot 0 still holds the mission after the tick.
        held: bool,
        /// Whether the NPC is still marked after the tick.
        marked: bool,
    }

    /// [`lawful`]'s NPC marked as an escort of mission 140 in slot 0,
    /// abortable, so its first loss fails the mission at once and
    /// releases the ship, shot at by `catalog`'s weapons until the tick
    /// it is downed.
    fn crimes_against_a_failing_escort(mut catalog: FakePilotCatalog) -> EscortDowned {
        catalog.missions = vec![MissionRecord {
            ships: crate::catalog::MissionShip {
                goal: 3,
                ..crate::catalog::MissionShip::NONE
            },
            ..crate::testkit::mission(140)
        }];
        let fight = |law: &dyn LegalCode| {
            let mut session = facing_an_npc(&catalog, 180);
            mark_the_npc(&mut session, Marked::Active);
            session.hold_trigger(FIRE);
            let rules = Rules {
                law,
                ..Rules::default()
            };
            for _ in 0..80 {
                session.tick_combat(rules, &mut NeverFires);
                if session.npcs()[0].condition != Condition::Intact {
                    break;
                }
            }
            assert_ne!(session.npcs()[0].condition, Condition::Intact, "downed");
            (
                records(&session),
                session.pilot().missions().get(0).is_some(),
                session.npcs()[0].mission.is_some(),
            )
        };
        let witness = Witness::default();
        let (_, held, marked) = fight(&witness);
        let (records, ..) = fight(&crate::legal::NovaLaw::default());
        EscortDowned {
            crimes: witness.seen.take(),
            records,
            held,
            marked,
        }
    }

    /// A loss that fails the ship's mission clears its mark within the
    /// tick (`raise_disabled` before `punish`), so `punish` must read the
    /// mark from before the tick.
    #[test]
    fn downing_a_mission_ship_whose_loss_fails_its_mission_changes_no_record() {
        assert_eq!(
            crimes_against_a_failing_escort(lawful()),
            EscortDowned {
                crimes: vec![],
                records: [0; 4],
                held: false,
                marked: false,
            },
            "disabled: the mission ended and the ship unmarked in the tick"
        );
        let at_once = FakePilotCatalog {
            weapons: vec![WeaponRecord {
                mass_dmg: 500,
                energy_dmg: 500,
                ..blaster()
            }],
            ..lawful()
        };
        assert_eq!(
            crimes_against_a_failing_escort(at_once),
            EscortDowned {
                crimes: vec![],
                records: [0; 4],
                held: false,
                marked: false,
            },
            "broken up at once: neither Disable nor Kill"
        );
    }

    // The combat rating.

    /// [`lawful`], its traffic's ship 129 of `strength`.
    fn rated(strength: i16) -> FakePilotCatalog {
        let mut catalog = lawful();
        for hull in &mut catalog.hulls {
            if hull.id == ShipId(129) {
                hull.strength = strength;
            }
        }
        catalog
    }

    /// The player shooting `catalog`'s NPC for 80 ticks: the NPC's
    /// condition and the player's combat rating each time either changes.
    fn rating_while_shooting(catalog: &FakePilotCatalog) -> Vec<(Option<Condition>, i64)> {
        let mut session = facing_an_npc(catalog, 180);
        session.hold_trigger(FIRE);
        let mut seen = Vec::new();
        for _ in 0..80 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            let condition = session.npcs().first().map(|npc| npc.condition);
            let now = (condition, session.pilot().combat_rating());
            if seen.last() != Some(&now) {
                seen.push(now);
            }
        }
        seen
    }

    #[test]
    fn breaking_up_a_disabled_ship_raises_the_rating_by_a_fifth_of_its_strength_once() {
        assert_eq!(
            rating_while_shooting(&rated(50)),
            [
                (Some(Condition::Intact), 0),
                (Some(Condition::Disabled), 0),
                (
                    Some(Condition::Dying {
                        ticks_left: 2,
                        starting: true
                    }),
                    10
                ),
                (None, 10),
            ],
            "disabling credits nothing, the break-up 50 / 5, its destruction nothing"
        );
    }

    #[test]
    fn breaking_up_a_ship_of_strength_4_or_less_raises_the_rating_by_1() {
        let seen = rating_while_shooting(&rated(3));
        assert_eq!(seen.last(), Some(&(None, 1)), "{seen:?}");
    }

    #[test]
    fn the_rating_starts_from_the_chärs_kills() {
        let mut catalog = rated(50);
        if let Ok(character) = &mut catalog.character {
            character.kills = 7;
        }
        let seen = rating_while_shooting(&catalog);
        assert_eq!(seen.last(), Some(&(None, 17)), "{seen:?}");
    }

    #[test]
    fn breaking_up_a_derelict_ship_leaves_the_rating() {
        let mut catalog = rated(50);
        catalog.govts[0].flags |= crate::govt::DERELICT;
        let seen = rating_while_shooting(&catalog);
        assert_eq!(seen.last(), Some(&(None, 0)), "{seen:?}");
    }

    #[test]
    fn an_npc_sees_the_players_rating_and_the_strength_of_shïp_128() {
        let mut catalog = rated(50);
        for hull in &mut catalog.hulls {
            if hull.id == ShipId(128) {
                hull.strength = 25;
            }
        }
        let mut session = facing_an_npc(&catalog, 180);
        session.pilot.combat_rating = 320_000;
        let seen = session.player_side();
        assert_eq!((seen.rating, seen.yardstick), (320_000, 25));
        session.arsenal = crate::combat::armament::Arsenal::new(&[], Vec::new());
        assert_eq!(session.player_side().yardstick, 0, "no shïp 128");
    }

    #[test]
    fn an_npc_sees_whether_the_players_ship_type_is_untargetable() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let flags2 = |session: &mut Session, flags2: u16| {
            session.ships = vec![ship(session.pilot.ship.0, ShipFields { flags2, ..FAST })];
            session.player_side().untargetable
        };
        assert!(!flags2(&mut session, 0x0003), "other bits");
        assert!(flags2(&mut session, crate::cloak::UNTARGETABLE | 0x0001));
        session.ships.clear();
        assert!(!session.player_side().untargetable, "no record");
    }

    #[test]
    fn boarding_a_disabled_ship_leaves_the_rating() {
        let session = aboard(&boardable());
        assert_eq!(session.pilot().combat_rating(), 0);
    }

    /// [`alongside`]'s trader (NPC 0, ship 129 of strength 10, government
    /// 140) broken up by NPC 1, the police, made the player's escort when
    /// `escorting`, then by NPC 0 itself: the rating after each.
    fn rating_after_npc_kills(escorting: bool) -> (i64, i64) {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        assert_eq!(session.arsenal.strength(ShipId(129)), 10);
        if escorting {
            session.traffic.npcs_mut()[1].escort = Some(EscortDuty {
                formation: FormationSlot {
                    slot: 2,
                    ships: 2,
                    spacing: 100.0,
                },
                order: None,
            });
        }
        let was = [NpcBefore {
            id: NpcId(0),
            condition: Condition::Disabled,
            govt: Some(GovtId(140)),
            marked: false,
        }];
        let broken_up_by = |by| Strike {
            ship: ShipRef::Npc(NpcId(0)),
            by,
            damage: 9.0,
            downed: Some(Downed::BreakingUp),
            provokes: true,
        };
        session.credit_kills(&[broken_up_by(ShipRef::Npc(NpcId(1)))], &was);
        let by_the_police = session.pilot().combat_rating();
        session.credit_kills(&[broken_up_by(ShipRef::Npc(NpcId(0)))], &was);
        (by_the_police, session.pilot().combat_rating())
    }

    #[test]
    fn a_kill_by_the_players_escort_raises_the_rating() {
        assert_eq!(
            rating_after_npc_kills(true),
            (2, 2),
            "10 / 5; NPC 0 is no escort"
        );
    }

    #[test]
    fn a_kill_by_a_fighter_the_player_launched_raises_the_rating() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        let before: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        session.launch_fighters(&[crate::combat::Sortie {
            carrier: ShipRef::Player,
            bay: WeaponId(149),
            ship: ShipId(130),
            from: *session.player(),
            heading: 0.0,
            target: None,
        }]);
        let fighter = session
            .npcs()
            .iter()
            .map(|npc| npc.id)
            .find(|id| !before.contains(id))
            .expect("launched");
        let was = [NpcBefore {
            id: NpcId(0),
            condition: Condition::Disabled,
            govt: Some(GovtId(140)),
            marked: false,
        }];
        session.credit_kills(
            &[Strike {
                ship: ShipRef::Npc(NpcId(0)),
                by: ShipRef::Npc(fighter),
                damage: 9.0,
                downed: Some(Downed::BreakingUp),
                provokes: true,
            }],
            &was,
        );
        assert_eq!(session.pilot().combat_rating(), 2, "10 / 5");
    }

    #[test]
    fn a_kill_by_an_npc_outside_the_players_fleet_leaves_the_rating() {
        assert_eq!(rating_after_npc_kills(false), (0, 0));
    }

    #[test]
    fn only_a_break_up_of_an_npc_credits_the_rating() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        let was = [NpcBefore {
            id: NpcId(0),
            condition: Condition::Intact,
            govt: Some(GovtId(140)),
            marked: false,
        }];
        let strike = |ship, downed| Strike {
            ship,
            by: ShipRef::Player,
            damage: 9.0,
            downed,
            provokes: true,
        };
        session.credit_kills(
            &[
                strike(ShipRef::Npc(NpcId(0)), None),
                strike(ShipRef::Npc(NpcId(0)), Some(Downed::Disabled)),
                strike(ShipRef::Player, Some(Downed::BreakingUp)),
                strike(ShipRef::Npc(NpcId(9)), Some(Downed::BreakingUp)),
            ],
            &was,
        );
        assert_eq!(session.pilot().combat_rating(), 0);
        session.credit_kills(
            &[strike(ShipRef::Npc(NpcId(0)), Some(Downed::BreakingUp))],
            &was,
        );
        assert_eq!(session.pilot().combat_rating(), 2);
    }

    /// The ship fates raised and not yet settled.
    fn fates(session: &Session) -> Vec<StoryEvent> {
        session
            .story_raised()
            .into_iter()
            .filter(|event| matches!(event, StoryEvent::ShipFate { .. }))
            .collect()
    }

    fn fate(ship: u32, fate: Fate) -> StoryEvent {
        StoryEvent::ShipFate {
            ship: NpcId(ship),
            fate,
        }
    }

    #[test]
    fn an_npc_disabled_then_destroyed_raises_each_fate_once_as_it_happens() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let mut seen = Vec::new();
        for _ in 0..80 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            let condition = session.npcs().first().map(|npc| npc.condition);
            let now = (condition, fates(&session).len());
            if seen.last() != Some(&now) {
                seen.push(now);
            }
        }
        assert_eq!(
            seen,
            [
                (Some(Condition::Intact), 0),
                (Some(Condition::Disabled), 1),
                (
                    Some(Condition::Dying {
                        ticks_left: 2,
                        starting: true
                    }),
                    1
                ),
                (None, 2),
            ]
        );
        assert_eq!(
            fates(&session),
            [fate(0, Fate::Disabled), fate(0, Fate::Destroyed)]
        );
    }

    #[test]
    fn breaking_up_an_intact_ship_at_once_raises_disabled_then_destroyed() {
        let catalog = FakePilotCatalog {
            weapons: vec![WeaponRecord {
                mass_dmg: 500,
                energy_dmg: 500,
                ..blaster()
            }],
            ..armed()
        };
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        for _ in 0..20 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            session.hold_trigger(Trigger::default());
        }
        assert_eq!(session.npcs(), [], "destroyed");
        assert_eq!(
            fates(&session),
            [fate(0, Fate::Disabled), fate(0, Fate::Destroyed)],
            "as the original's _DamageShip counts it disabled"
        );
    }

    #[test]
    fn a_watched_ship_is_observed_once_by_the_traffic_in_flight() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(fates(&session), [], "unwatched");
        session.watch_ship(NpcId(0));
        for _ in 0..3 {
            session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        }
        assert_eq!(fates(&session), [fate(0, Fate::Observed)]);
        assert!(session.story.watched.is_empty(), "forgotten once seen");
    }

    /// A full cloak: fading out from 32, it reads cloaked for 32 more
    /// ticks.
    pub(super) const CLOAKED: crate::cloak::Cloak = crate::cloak::Cloak {
        fade: crate::cloak::FADE_FULL,
        direction: 0,
    };

    /// A cloak and fuel: a ship of it can cloak, and none of its AI-cloak
    /// conditions fire.
    const CAN_CLOAK: crate::cloak::CloakGear = crate::cloak::CloakGear {
        device: true,
        decloaks_when_hit: false,
        on_radar: false,
        flags2: 0,
        scanner: crate::cloak::Scanner::NONE,
    };

    /// [`armed`], with outfit 260, a cloak scanner of `ModVal` `scan`, the
    /// player owns.
    pub(super) fn scanning(scan: i16) -> FakePilotCatalog {
        let mut catalog = armed();
        catalog
            .outfits
            .push(outfit(260, &[(crate::cloak::MOD_SCANNER, scan)]));
        catalog
    }

    /// `catalog`'s [`facing_an_npc`], the player owning outfit 260.
    pub(super) fn holding_260(catalog: &FakePilotCatalog) -> Session {
        let mut session = facing_an_npc(catalog, 180);
        session.pilot.outfits.insert(OutfitId(260), 1);
        session
    }

    #[test]
    fn a_watched_cloaked_ship_is_not_observed_and_stays_watched_until_it_decloaks() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.traffic.npcs_mut()[0].cloak = CLOAKED;
        session.watch_ship(NpcId(0));
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(fates(&session), [], "cloaked, and no scanner");
        assert_eq!(session.story.watched, BTreeSet::from([NpcId(0)]), "kept");
        let mut ticks = 1;
        while fates(&session).is_empty() {
            assert!(
                session.npcs()[0].cloak.is_cloaked(),
                "seen at once uncloaked"
            );
            session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
            ticks += 1;
        }
        assert_eq!(
            ticks, 32,
            "fading out from 32 at 0.75 a tick, cloaked above 8"
        );
        assert_eq!(fates(&session), [fate(0, Fate::Observed)]);
        assert!(session.story.watched.is_empty(), "forgotten once seen");
    }

    #[test]
    fn a_cloak_scanner_sees_a_watched_cloaked_ship_within_200_on_each_axis() {
        let catalog = scanning(crate::cloak::SCAN_SCREEN);
        let mut session = holding_260(&catalog);
        assert!(session.scanner().on_screen);
        let player = session.player().position;
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.cloak = CLOAKED;
        npc.state.position = player + Vec2::new(0.0, -250.0);
        session.watch_ship(NpcId(0));
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(fates(&session), [], "beyond 200");
        session.traffic.npcs_mut()[0].state.position = player + Vec2::new(150.0, -150.0);
        session.traffic.npcs_mut()[0].cloak = CLOAKED;
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert!(session.npcs()[0].cloak.is_cloaked());
        assert_eq!(fates(&session), [fate(0, Fate::Observed)], "within 200");
        let other = scanning(crate::cloak::SCAN_RADAR | crate::cloak::SCAN_TARGET);
        let mut session = holding_260(&other);
        session.traffic.npcs_mut()[0].cloak = CLOAKED;
        session.watch_ship(NpcId(0));
        session.tick_traffic(&other, &Peaceful, &mut NeverFires);
        assert_eq!(fates(&session), [], "no on-screen bit");
    }

    #[test]
    fn a_watched_ship_that_can_cloak_is_observed_only_once_it_is_also_on_screen() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let player = session.player().position;
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.cloak_gear = CAN_CLOAK;
        npc.state.position = player + Vec2::new(0.0, -400.0);
        session.watch_ship(NpcId(0));
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert!(!session.npcs()[0].cloak.is_cloaked());
        assert_eq!(fates(&session), [], "seen, but off screen");
        assert_eq!(session.story.watched, BTreeSet::from([NpcId(0)]), "kept");
        session.traffic.npcs_mut()[0].state.position = player + Vec2::new(0.0, -200.0);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(fates(&session), [fate(0, Fate::Observed)]);
    }

    #[test]
    fn a_watched_ship_that_cannot_cloak_is_observed_off_screen() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let player = session.player().position;
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.state.position = player + Vec2::new(0.0, -2000.0);
        session.watch_ship(NpcId(0));
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(fates(&session), [fate(0, Fate::Observed)]);
        let mut disabled = facing_an_npc(&catalog, 180);
        let npc = &mut disabled.traffic.npcs_mut()[0];
        npc.cloak_gear = CAN_CLOAK;
        npc.condition = Condition::Disabled;
        npc.state.position = player + Vec2::new(0.0, -2000.0);
        disabled.watch_ship(NpcId(0));
        disabled.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(
            fates(&disabled),
            [fate(0, Fate::Observed)],
            "disabled: cannot cloak"
        );
    }

    #[test]
    fn the_play_area_the_view_gives_decides_what_is_on_screen() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let player = session.player().position;
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.cloak_gear = CAN_CLOAK;
        npc.state.position = player + Vec2::new(0.0, -400.0);
        assert!(
            !session.on_screen(&session.npcs()[0]),
            "the default play area"
        );
        session.set_play_area(crate::cloak::Area::centred(Vec2::ZERO, 1024.0, 1024.0));
        assert!(session.on_screen(&session.npcs()[0]));
        session.set_play_area(crate::cloak::Area {
            left: -100.0,
            top: 0.0,
            right: 100.0,
            bottom: 500.0,
        });
        assert!(
            !session.on_screen(&session.npcs()[0]),
            "the area is below the player"
        );
        session.watch_ship(NpcId(0));
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(fates(&session), []);
    }

    #[test]
    fn the_play_area_is_the_originals_smallest_until_the_view_says() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        assert_eq!(session.play_area(), crate::cloak::DEFAULT_PLAY_AREA);
        let wide = crate::cloak::Area::centred(Vec2::ZERO, 2000.0, 100.0);
        session.set_play_area(wide);
        assert_eq!(session.play_area(), wide);
    }

    #[test]
    fn a_ships_sprite_width_and_height_inset_the_play_area() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let player = session.player().position;
        session.set_play_area(crate::cloak::Area::centred(Vec2::ZERO, 600.0, 600.0));
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.hull.sprite_size = Some(40.0);
        npc.hull.sprite_height = Some(80.0);
        // A quarter of the sprite inside: 10 across, 20 down.
        npc.state.position = player + Vec2::new(289.0, 0.0);
        assert!(session.on_screen(&session.npcs()[0]));
        session.traffic.npcs_mut()[0].state.position = player + Vec2::new(290.0, 0.0);
        assert!(!session.on_screen(&session.npcs()[0]));
        session.traffic.npcs_mut()[0].state.position = player + Vec2::new(0.0, 279.0);
        assert!(session.on_screen(&session.npcs()[0]));
        session.traffic.npcs_mut()[0].state.position = player + Vec2::new(0.0, 280.0);
        assert!(!session.on_screen(&session.npcs()[0]));
    }

    #[test]
    fn the_player_sees_a_ship_uncloaked_and_not_emerging() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        assert!(session.sees(&session.npcs()[0]));
        session.traffic.npcs_mut()[0].cloak = CLOAKED;
        assert!(!session.sees(&session.npcs()[0]));
        session.traffic.npcs_mut()[0].cloak = crate::cloak::Cloak::default();
        session.traffic.npcs_mut()[0].mode = crate::traffic::npc::Mode::Emerging { ticks_left: 5 };
        assert!(!session.sees(&session.npcs()[0]));
    }

    #[test]
    fn the_player_sees_past_a_ships_cloak_whether_or_not_it_waits_at_its_gate() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.traffic.npcs_mut()[0].mode = crate::traffic::npc::Mode::Emerging { ticks_left: 5 };
        assert!(session.sees_past_cloak(&session.npcs()[0]), "emerging");
        session.traffic.npcs_mut()[0].cloak = CLOAKED;
        assert!(!session.sees_past_cloak(&session.npcs()[0]), "cloaked");
        let scans = scanning(crate::cloak::SCAN_SCREEN);
        let mut session = holding_260(&scans);
        let player = session.player().position;
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.cloak = CLOAKED;
        npc.state.position = player + Vec2::new(0.0, -150.0);
        assert!(
            session.sees_past_cloak(&session.npcs()[0]),
            "a scanner within 200"
        );
    }

    #[test]
    fn a_cloaked_ship_is_on_the_radar_only_when_its_cloak_shows_there_or_the_player_scans_for_it() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        assert!(session.on_radar(&session.npcs()[0]), "uncloaked");
        session.traffic.npcs_mut()[0].cloak = CLOAKED;
        assert!(!session.on_radar(&session.npcs()[0]));
        session.traffic.npcs_mut()[0].cloak_gear.on_radar = true;
        assert!(session.on_radar(&session.npcs()[0]), "its cloak shows");
        let scans = scanning(crate::cloak::SCAN_RADAR);
        let mut session = holding_260(&scans);
        session.traffic.npcs_mut()[0].cloak = CLOAKED;
        assert!(session.on_radar(&session.npcs()[0]), "the player scans");
    }

    #[test]
    fn a_cloaked_target_is_let_go_unless_the_player_can_target_cloaked_ships() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.target = Some(NpcId(0));
        session.traffic.npcs_mut()[0].cloak = CLOAKED;
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(session.target, None);
        assert_eq!(session.select_target(TargetPick::Nearest), None, "unpicked");
        let targets = scanning(crate::cloak::SCAN_TARGET);
        let mut session = holding_260(&targets);
        session.target = Some(NpcId(0));
        session.traffic.npcs_mut()[0].cloak = CLOAKED;
        session.tick_traffic(&targets, &Peaceful, &mut NeverFires);
        assert_eq!(session.target, Some(NpcId(0)), "it can");
        session.target = None;
        assert_eq!(session.select_target(TargetPick::Next), Some(NpcId(0)));
    }

    /// `session`'s NPC 0 made of an untargetable type, attacking the
    /// player.
    fn untargetable_attacker(session: &mut Session) {
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.cloak_gear.flags2 = crate::cloak::UNTARGETABLE;
        npc.goal = Goal::Attack(ShipRef::Player);
    }

    #[test]
    fn the_target_keys_pass_over_an_untargetable_ship_until_the_player_owns_the_scanner() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        untargetable_attacker(&mut session);
        for pick in [
            TargetPick::Nearest,
            TargetPick::NearestThreat,
            TargetPick::Next,
        ] {
            session.target = None;
            assert_eq!(session.select_target(pick), None, "{pick:?}");
        }
        let scans = scanning(crate::cloak::SCAN_UNTARGETABLE);
        let mut session = holding_260(&scans);
        untargetable_attacker(&mut session);
        for pick in [
            TargetPick::Nearest,
            TargetPick::NearestThreat,
            TargetPick::Next,
        ] {
            session.target = None;
            assert_eq!(session.select_target(pick), Some(NpcId(0)), "{pick:?}");
        }
    }

    #[test]
    fn a_held_target_of_an_untargetable_type_is_not_let_go() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.target = Some(NpcId(0));
        untargetable_attacker(&mut session);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(session.target, Some(NpcId(0)));
    }

    #[test]
    fn a_hit_by_the_player_cloaks_a_ship_that_cloaks_when_hit_by_a_ship_it_is_not_attacking() {
        let catalog = FakePilotCatalog {
            weapons: vec![WeaponRecord {
                mass_dmg: 1,
                energy_dmg: 1,
                ..blaster()
            }],
            ..armed()
        };
        let mut session = facing_an_npc(&catalog, 180);
        session.traffic.npcs_mut()[0].cloak_gear = crate::cloak::CloakGear {
            flags2: crate::cloak::CLOAK_WHEN_HIT,
            ..CAN_CLOAK
        };
        session.hold_trigger(FIRE);
        let shield = session.npcs()[0].reserves.shield.now;
        for _ in 0..20 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            if session.npcs()[0].reserves.shield.now < shield {
                break;
            }
        }
        assert!(session.npcs()[0].reserves.shield.now < shield, "hit");
        assert_eq!(session.npcs()[0].cloak.direction, 1, "fading in");
        let mut fighting = facing_an_npc(&catalog, 180);
        let npc = &mut fighting.traffic.npcs_mut()[0];
        npc.cloak_gear = crate::cloak::CloakGear {
            flags2: crate::cloak::CLOAK_WHEN_HIT,
            ..CAN_CLOAK
        };
        npc.goal = Goal::Attack(ShipRef::Player);
        fighting.hold_trigger(FIRE);
        for _ in 0..20 {
            fighting.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(
            fighting.npcs()[0].cloak,
            crate::cloak::Cloak::default(),
            "it attacks the player"
        );
    }

    #[test]
    fn a_hit_uncloaks_a_cloaked_ship_whose_cloak_decloaks_when_hit() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.cloak_gear = crate::cloak::CloakGear {
            decloaks_when_hit: true,
            ..CAN_CLOAK
        };
        npc.cloak = CLOAKED;
        session.hold_trigger(FIRE);
        for _ in 0..20 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            if session.npcs()[0].cloak.direction != 0 {
                break;
            }
        }
        assert_eq!(session.npcs()[0].cloak.direction, -1);
    }

    #[test]
    fn a_shockwave_neither_cloaks_nor_uncloaks_the_ship_it_hits() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let shockwave = Strike {
            ship: ShipRef::Npc(NpcId(0)),
            by: ShipRef::Player,
            damage: 32.0,
            downed: None,
            provokes: false,
        };
        session.traffic.npcs_mut()[0].cloak_gear = crate::cloak::CloakGear {
            flags2: crate::cloak::CLOAK_WHEN_HIT,
            ..CAN_CLOAK
        };
        session.cloak_hits(&[shockwave]);
        assert_eq!(session.npcs()[0].cloak, crate::cloak::Cloak::default());
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.cloak_gear = crate::cloak::CloakGear {
            decloaks_when_hit: true,
            ..CAN_CLOAK
        };
        npc.cloak = CLOAKED;
        session.cloak_hits(&[shockwave]);
        assert_eq!(session.npcs()[0].cloak, CLOAKED);
    }

    // The death shockwave.

    /// [`lawful`], with the player's ship 128 a fast one.
    fn shockwave_catalog() -> FakePilotCatalog {
        let mut catalog = lawful();
        catalog.ship_records.push(crate::testkit::ship(128, FAST));
        catalog
    }

    /// `catalog`'s session with the player at rest at the centre and
    /// NPC 0 at rest 40 pixels above it, both of `Mass` 200 and
    /// regenerating nothing.
    fn beside_a_heavy_npc(catalog: &FakePilotCatalog) -> Session {
        let mut session = facing_an_npc(catalog, 180);
        session.player.position = Vec2::ZERO;
        session.player.velocity = Vec2::ZERO;
        session.hull.mass = 200.0;
        session.stats.shield_regen = 0.0;
        session.stats.armor_regen = 0.0;
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.state.position = Vec2::new(0.0, -40.0);
        npc.hull.mass = 200.0;
        npc.hull.tough = false;
        npc.stats.shield_regen = 0.0;
        npc.stats.armor_regen = 0.0;
        session
    }

    /// Breaks up a ship of `reserves` and `condition` to the last tick
    /// before its final explosion, which the next fight tick sets off.
    fn on_its_last_tick(reserves: &mut Reserves, condition: &mut Condition) {
        reserves.armor.now = -1.0;
        *condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
    }

    /// Shields down and `armor` of `armor_max` armour.
    fn bare(reserves: &mut Reserves, armor: f32, armor_max: f32) {
        reserves.shield = crate::reserves::Gauge { now: 0.0, max: 0.0 };
        reserves.armor = crate::reserves::Gauge {
            now: armor,
            max: armor_max,
        };
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn the_players_heavy_hull_exploding_damages_and_pushes_npcs_but_not_itself() {
        let catalog = shockwave_catalog();
        let mut session = beside_a_heavy_npc(&catalog);
        session.traffic.npcs_mut()[0].reserves = Reserves::full(100.0, 300.0, 100.0);
        on_its_last_tick(&mut session.pilot.reserves, &mut session.condition);
        let player = session.pilot.reserves;
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.player_condition(), Condition::Destroyed);
        let npc = &session.npcs()[0];
        assert_eq!(
            (npc.reserves.shield.now, npc.reserves.armor.now),
            (68.0, 300.0)
        );
        assert!(
            near(npc.state.velocity, Vec2::new(0.0, -3.75)),
            "{:?}",
            npc.state.velocity
        );
        assert_eq!(
            session.pilot.reserves, player,
            "its own shockwave spares it"
        );
        assert_eq!(session.player().velocity, Vec2::ZERO);
    }

    #[test]
    fn a_ship_drifting_away_is_pushed_short_of_its_class_speed_and_not_past_it() {
        let catalog = shockwave_catalog();
        for (drift, after) in [(-2.0, -5.75), (-6.0, -6.0)] {
            let mut session = beside_a_heavy_npc(&catalog);
            session.traffic.npcs_mut()[0].state.velocity = Vec2::new(0.0, drift);
            on_its_last_tick(&mut session.pilot.reserves, &mut session.condition);
            session.tick_combat(Rules::default(), &mut NeverFires);
            let velocity = session.npcs()[0].state.velocity;
            assert!(
                near(velocity, Vec2::new(0.0, after)),
                "class speed 6, from {drift}: {velocity:?}"
            );
        }
    }

    #[test]
    fn an_exploding_heavy_npc_damages_and_pushes_the_player() {
        let catalog = shockwave_catalog();
        let mut session = beside_a_heavy_npc(&catalog);
        session.pilot.reserves.shield = crate::reserves::Gauge::full(100.0);
        session.pilot.reserves.armor = crate::reserves::Gauge::full(300.0);
        let npc = &mut session.traffic.npcs_mut()[0];
        on_its_last_tick(&mut npc.reserves, &mut npc.condition);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.npcs(), [], "the wreck is taken out");
        let reserves = session.pilot.reserves;
        assert_eq!((reserves.shield.now, reserves.armor.now), (68.0, 300.0));
        assert!(
            near(session.player().velocity, Vec2::new(0.0, 3.75)),
            "{:?}",
            session.player().velocity
        );
    }

    #[test]
    fn a_player_braking_for_its_jump_is_hit_by_a_shockwave_but_not_pushed() {
        let catalog = shockwave_catalog();
        let mut session = beside_a_heavy_npc(&catalog);
        session.pilot.reserves.shield = crate::reserves::Gauge::full(100.0);
        session.jump = Some(Jump::PreJump {
            to: SystemId(131),
            bearing: None,
            warp: None,
        });
        let npc = &mut session.traffic.npcs_mut()[0];
        on_its_last_tick(&mut npc.reserves, &mut npc.condition);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.pilot.reserves.shield.now, 68.0, "hit");
        assert_eq!(session.player().velocity, Vec2::ZERO, "not pushed");
    }

    #[test]
    fn the_players_shockwave_on_a_300_armour_ship_convicts_of_nothing_and_credits_no_kill() {
        let catalog = shockwave_catalog();
        for (armor, before, after) in [
            (110.0, Condition::Intact, 100.99),
            (20.0, Condition::Disabled, 1.0),
        ] {
            let mut session = beside_a_heavy_npc(&catalog);
            let npc = &mut session.traffic.npcs_mut()[0];
            bare(&mut npc.reserves, armor, 300.0);
            npc.condition = before;
            on_its_last_tick(&mut session.pilot.reserves, &mut session.condition);
            let rating = session.pilot.combat_rating;
            let witness = Witness::default();
            let rules = Rules {
                law: &witness,
                ..Rules::default()
            };
            session.tick_combat(rules, &mut NeverFires);
            let npc = &session.npcs()[0];
            assert!(
                (npc.reserves.armor.now - after).abs() < 1e-4,
                "{armor}: {:?}",
                npc.reserves
            );
            assert_eq!(npc.condition, before, "{armor}");
            assert_eq!(witness.seen.take(), [], "{armor}");
            assert_eq!(session.pilot.combat_rating, rating, "{armor}");
        }
    }

    #[test]
    fn the_players_shockwave_disabling_a_40000_armour_ship_convicts_once() {
        let catalog = shockwave_catalog();
        let mut session = beside_a_heavy_npc(&catalog);
        bare(
            &mut session.traffic.npcs_mut()[0].reserves,
            13_350.0,
            40_000.0,
        );
        on_its_last_tick(&mut session.pilot.reserves, &mut session.condition);
        session.tick_combat(Rules::default(), &mut NeverFires);
        let npc = &session.npcs()[0];
        assert_eq!(npc.reserves.armor.now, 13_333.0, "floored, still disabled");
        assert_eq!(npc.condition, Condition::Disabled);
        assert_eq!(
            records(&session),
            [-3, -3, 4, 10],
            "convicted of disabling it"
        );
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(records(&session), [-3, -3, 4, 10], "once");
    }

    #[test]
    fn an_npcs_shockwave_convicts_the_player_of_nothing() {
        let catalog = shockwave_catalog();
        let mut session = beside_a_heavy_npc(&catalog);
        let mut victim = session.npcs()[0].clone();
        victim.state.position = Vec2::new(30.0, -40.0);
        bare(&mut victim.reserves, 13_350.0, 40_000.0);
        let victim = session.traffic.add_npc(victim);
        let npc = &mut session.traffic.npcs_mut()[0];
        on_its_last_tick(&mut npc.reserves, &mut npc.condition);
        session.tick_combat(Rules::default(), &mut NeverFires);
        let victim = session
            .npcs()
            .iter()
            .find(|npc| npc.id == victim)
            .expect("still here");
        assert_eq!(victim.condition, Condition::Disabled);
        assert_eq!(victim.reserves.armor.now, 13_333.0);
        assert_eq!(records(&session), [0, 0, 0, 0]);
    }

    #[test]
    fn a_watched_ship_coming_out_of_a_gate_is_observed_once_it_is_out() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.traffic.npcs_mut()[0].mode = crate::traffic::npc::Mode::Emerging { ticks_left: 3 };
        session.watch_ship(NpcId(0));
        for left in [2, 1] {
            session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
            assert!(session.npcs()[0].emerging(), "{left} to go");
            assert_eq!(fates(&session), [], "not seen while it waits");
            assert_eq!(session.story.watched, BTreeSet::from([NpcId(0)]), "kept");
        }
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert!(!session.npcs()[0].emerging(), "bursting out");
        assert_eq!(fates(&session), [fate(0, Fate::Observed)]);
        assert!(session.story.watched.is_empty(), "forgotten once seen");
    }

    #[test]
    fn a_watched_ship_is_not_observed_while_landed_or_jumping_past_the_warps_lead_in() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.landed = Some(StellarId(128));
        session.watch_ship(NpcId(0));
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(fates(&session), [], "landed");
        session.landed = None;
        session.jump = Some(Jump::Hyperspace {
            to: SystemId(131),
            warp: 1000,
        });
        assert!(!session.in_warp_lead_in());
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(fates(&session), [], "jumping");
        assert_eq!(session.story.watched, BTreeSet::from([NpcId(0)]), "kept");
    }

    #[test]
    fn repopulating_or_losing_a_watched_ship_forgets_it() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.watch_ship(NpcId(0));
        session.populate(&catalog, &mut Draws::of(&[6, 6, 0, 0, 750, 650, 180]));
        assert!(session.story.watched.is_empty(), "the populate forgot it");

        session.watch_ship(NpcId(99));
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(fates(&session), [], "not in the system");
        assert!(session.story.watched.is_empty(), "forgotten");

        session.watch_ship(NpcId(0));
        session.traffic_due = true;
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(fates(&session), [], "the populating tick");
        assert!(session.story.watched.is_empty(), "forgotten");

        let watched = session.npcs()[0].id;
        session.watch_ship(watched);
        session.landed = Some(StellarId(128));
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        assert!(session.story.watched.is_empty(), "the traffic reset");
    }

    #[test]
    fn an_npc_downing_another_is_no_crime() {
        let catalog = FakePilotCatalog {
            traffic: lawful()
                .traffic
                .into_iter()
                .map(|(system, traffic)| {
                    (
                        system,
                        SystemTraffic {
                            avg_ships: 2,
                            persons: Default::default(),
                            ..traffic
                        },
                    )
                })
                .collect(),
            ..lawful()
        };
        let mut session = Session::start(&catalog).expect("starts");
        // NPC 0 at (0, -100) and NPC 1 at (100, -100), both facing right,
        // at rest.
        session.populate(
            &catalog,
            &mut Draws::of(&[6, 6, 0, 0, 750, 650, 90, 2, 6, 6, 0, 0, 850, 650, 90, 2]),
        );
        stop_npcs(&mut session);
        let witness = Witness::default();
        let rules = Rules {
            law: &witness,
            ..Rules::default()
        };
        for _ in 0..80 {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(rules, &mut NeverFires);
        }
        assert_eq!(session.npcs().len(), 1, "NPC 0 destroyed NPC 1");
        assert_eq!(witness.seen.take(), []);
    }

    #[test]
    fn a_disabled_player_drifts_ignoring_the_controls() {
        let catalog = armed();
        let mut session = Session::start(&catalog).expect("starts");
        session.tick(Controls {
            thrust: true,
            ..Controls::default()
        });
        session.take_sounds();
        let moving = *session.player();
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(session.player_condition(), Condition::Disabled);
        session.tick(Controls {
            thrust: true,
            turn: Turn::Left,
            reverse: false,
        });
        let drifted = *session.player();
        assert_eq!(
            drifted.velocity,
            moving.velocity * crate::flight::DISABLED_DRAG,
            "no thrust, only drag"
        );
        assert_eq!(drifted.heading, moving.heading, "no turn");
        assert_eq!(
            drifted.position,
            moving.position + moving.velocity * crate::flight::DISABLED_DRAG,
            "moved by the damped velocity"
        );
        assert_eq!(session.take_sounds(), [SimSound::ThrustStopped]);
        session.hold_trigger(FIRE);
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(session.shots(), [], "nor fires");
    }

    /// A session whose player has thrust once and is then disabled.
    fn disabled_moving_player(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.tick(Controls {
            thrust: true,
            ..Controls::default()
        });
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(session.player_condition(), Condition::Disabled);
        session
    }

    #[test]
    fn a_disabled_player_slows_by_0_995_a_tick() {
        let catalog = armed();
        let mut session = disabled_moving_player(&catalog);
        let start = session.player().velocity;
        let mut expected = start;
        for _ in 0..10 {
            session.tick(Controls::default());
            expected = expected * crate::flight::DISABLED_DRAG;
        }
        assert_eq!(session.player().velocity, expected);
        assert!(expected.length() < start.length() * 0.96);
    }

    #[test]
    fn a_breaking_up_player_slows_by_0_995_a_tick() {
        let catalog = armed();
        let mut session = disabled_moving_player(&catalog);
        session.condition = Condition::Dying {
            ticks_left: 10,
            starting: false,
        };
        let before = *session.player();
        session.tick(Controls::default());
        let damped = before.velocity * crate::flight::DISABLED_DRAG;
        assert_eq!(session.player().velocity, damped);
        assert_eq!(session.player().position, before.position + damped);
    }

    #[test]
    fn an_intact_player_coasting_keeps_its_velocity() {
        let catalog = armed();
        let mut session = Session::start(&catalog).expect("starts");
        session.tick(Controls {
            thrust: true,
            ..Controls::default()
        });
        let moving = *session.player();
        session.tick(Controls::default());
        assert_eq!(session.player().velocity, moving.velocity);
    }

    #[test]
    fn a_disabled_player_drags_once_a_tick_across_a_full_session_tick() {
        let catalog = armed();
        let mut session = disabled_moving_player(&catalog);
        let start = session.player().velocity;
        session.tick(Controls::default());
        session.tick_traffic(&catalog, &Firing, &mut NeverFires);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            session.player().velocity,
            start * crate::flight::DISABLED_DRAG
        );
    }

    #[test]
    fn a_ship_that_is_not_intact_can_neither_land_nor_jump() {
        let catalog = armed();
        let mut session = Session::start(&catalog).expect("starts");
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(
            session.land(),
            Err(LandingRefusal::Disabled),
            "over planet 128"
        );
        assert_eq!(session.landed(), None);
        assert!(!session.take_save_due(), "nothing to save");
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.take_sounds();
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::Disabled));
        assert_eq!(session.jumping(), None);
        assert_eq!(session.take_sounds(), [], "a refusal emits nothing");
    }

    // A disabled player's land key (`_HandlePlayerDockRequest`'s silent
    // refusal at 0x66af7).

    /// [`catalog`]'s session with the player disabled, its sounds and
    /// save taken.
    fn disabled(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.condition = Condition::Disabled;
        let _ = session.take_sounds();
        session.take_save_due();
        session
    }

    #[test]
    fn a_disabled_players_l_with_no_target_selects_the_nearest_stellar_and_says_nothing() {
        let mut session = disabled(&catalog());
        assert_eq!(session.land(), Err(LandingRefusal::Disabled));
        assert_eq!(session.nav_target(), Some(StellarId(128)), "@0x6a901");
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn a_disabled_players_l_over_its_cleared_target_neither_lands_nor_says_why() {
        let mut session = disabled(&catalog());
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(session.land(), Err(LandingRefusal::Disabled));
        assert_eq!(session.landed(), None);
        assert_eq!(session.nav_target(), Some(StellarId(128)), "kept");
        assert_eq!(session.take_sounds(), [], "no landing sound");
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_disabled_players_l_too_far_or_denied_says_nothing_either() {
        let mut session = disabled(&catalog());
        session.select_stellar(StellarPick::Slot(1));
        assert_eq!(
            session.land(),
            Err(LandingRefusal::Disabled),
            "too far: the range test comes after (@0x67838)"
        );
        assert_eq!(session.nav_target(), Some(StellarId(129)), "kept");
        let catalog = governed([Some(WANTED), Some(LAWFUL)], None);
        let mut session = Session::fly(&catalog, on_record(&catalog)).expect("flies");
        session.condition = Condition::Disabled;
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(
            session.land(),
            Err(LandingRefusal::Disabled),
            "denied: the clearance comes after (@0x67903)"
        );
    }

    #[test]
    fn a_disabled_players_l_on_an_unlandable_target_says_why_as_an_intact_one_does() {
        let mut catalog = catalog();
        catalog.sites[0].1[0].flags = 0;
        let mut session = disabled(&catalog);
        session.select_stellar(StellarPick::Slot(0));
        let refusal = Err(LandingRefusal::NotLandable {
            stellar: StellarId(128),
            station: false,
        });
        assert_eq!(session.land(), refusal, "@0x66977");
        session.condition = Condition::Intact;
        assert_eq!(session.land(), refusal, "the same refusal");
    }

    #[test]
    fn a_disabled_players_l_in_a_system_with_no_stellars_says_so_as_an_intact_one_does() {
        let empty = FakePilotCatalog {
            sites: Vec::new(),
            ..catalog()
        };
        let mut session = disabled(&empty);
        assert_eq!(session.land(), Err(LandingRefusal::NoStellars), "@0x669aa");
        session.condition = Condition::Intact;
        assert_eq!(
            session.land(),
            Err(LandingRefusal::NoStellars),
            "the same refusal"
        );
    }

    #[test]
    fn a_disabled_players_l_over_a_hypergate_does_not_enter_it() {
        let catalog = gated();
        let mut session = disabled(&catalog);
        assert_eq!(session.land(), Err(LandingRefusal::Disabled));
        assert_eq!(session.nav_target(), Some(StellarId(300)));
        assert_eq!(session.land(), Err(LandingRefusal::Disabled));
        assert_eq!(
            session.open_hypergate(&catalog),
            Err(GateRefusal::NotAtGate),
            "no entry"
        );
    }

    #[test]
    fn a_breaking_up_players_l_selects_nothing() {
        let mut session = disabled(&catalog());
        session.condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
        assert_eq!(session.land(), Err(LandingRefusal::Disabled));
        assert_eq!(session.nav_target(), None, "the breakup gate @0x6a8cb");
    }

    /// An NPC one shot of whose disables, breaks up and destroys the
    /// player at once: no shield, armour gone.
    pub(super) fn deadly() -> FakePilotCatalog {
        let mut catalog = armed();
        catalog.weapons = vec![WeaponRecord {
            mass_dmg: 100,
            flags: 0x0020,
            ..blaster()
        }];
        catalog
    }

    #[test]
    fn a_destroyed_player_stays_destroyed_and_stops_moving() {
        let catalog = deadly();
        let mut session = facing_an_npc(&catalog, 180);
        let mut events = Vec::new();
        for _ in 0..20 {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        assert_eq!(session.player_condition(), Condition::Destroyed);
        assert!(events.iter().any(|event| matches!(
            event,
            CombatEvent::Destroyed {
                ship: ShipRef::Player,
                ship_type: ShipId(128),
                ..
            }
        )));
        let at = *session.player();
        session.tick(Controls {
            thrust: true,
            ..Controls::default()
        });
        assert_eq!(session.player().position, at.position);
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(session.land(), Err(LandingRefusal::Disabled));
        assert!(!session.take_save_due());
    }

    #[test]
    fn the_players_fate_is_exploded_only_on_the_tick_it_is_destroyed_then_holding() {
        let catalog = deadly();
        let mut session = facing_an_npc(&catalog, 180);
        let mut broke_up = false;
        let mut exploded = false;
        for _ in 0..40 {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            let events = session.take_combat_events();
            let destroyed = events.iter().any(|event| {
                matches!(
                    event,
                    CombatEvent::Destroyed {
                        ship: ShipRef::Player,
                        ..
                    }
                )
            });
            if destroyed {
                assert_eq!(session.player_fate(), PlayerFate::Exploded);
                exploded = true;
                break;
            }
            if session.player_condition() != Condition::Intact {
                assert!(
                    matches!(session.player_fate(), PlayerFate::BreakingUp { .. }),
                    "{:?}",
                    session.player_fate()
                );
                broke_up = true;
            }
        }
        assert!(
            broke_up && exploded,
            "broke up {broke_up}, exploded {exploded}"
        );
        for _ in 0..3 {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            assert!(
                matches!(session.player_fate(), PlayerFate::Holding { .. }),
                "{:?}",
                session.player_fate()
            );
        }
    }

    /// One whole tick of `session`, as the flight view steps it: flight,
    /// then the traffic under `catalog`'s firing NPC, then the fight.
    fn full_tick(session: &mut Session, catalog: &FakePilotCatalog) {
        session.tick(Controls::default());
        session.tick_traffic(catalog, &Firing, &mut NeverFires);
        session.tick_combat(Rules::default(), &mut NeverFires);
    }

    /// Steps `session` under fire from `catalog`'s NPC until the player's
    /// ship explodes, and returns with it on the explosion's tick.
    fn until_exploded(session: &mut Session, catalog: &FakePilotCatalog) {
        for _ in 0..60 {
            full_tick(session, catalog);
            if session.player_fate() == PlayerFate::Exploded {
                return;
            }
        }
        panic!("the player's ship never exploded");
    }

    #[test]
    fn the_hold_runs_242_ticks_after_the_explosion_and_expires_on_the_243rd() {
        let catalog = deadly();
        let mut session = facing_an_npc(&catalog, 180);
        until_exploded(&mut session, &catalog);
        assert!(!session.hold_expired());
        for tick in 1..=HOLD_TICKS {
            full_tick(&mut session, &catalog);
            assert_eq!(
                session.player_fate(),
                PlayerFate::Holding {
                    ticks_left: HOLD_TICKS - tick
                },
                "dead tick {tick}"
            );
            assert!(!session.hold_expired(), "dead tick {tick}");
            assert!(!session.take_hold_expired(), "dead tick {tick}");
        }
        full_tick(&mut session, &catalog);
        assert_eq!(session.player_fate(), PlayerFate::Holding { ticks_left: 0 });
        assert!(session.hold_expired(), "on the 243rd dead tick");
        assert!(session.take_hold_expired());
        assert!(!session.take_hold_expired(), "taken once");
        assert!(session.hold_expired(), "taking it leaves the stage alone");
        full_tick(&mut session, &catalog);
        assert!(!session.take_hold_expired(), "it expires once");
        assert!(session.hold_expired());
    }

    #[test]
    fn every_dead_tick_lets_go_of_the_players_target_and_nav_target() {
        let catalog = deadly();
        let mut session = facing_an_npc(&catalog, 180);
        session.condition = Condition::Destroyed;
        for ticks_left in [5, 1, 0] {
            session.player_end = Some(Ending::Holding { ticks_left });
            session.target = Some(NpcId(0));
            session.nav_target = Some(StellarId(128));
            session.tick(Controls::default());
            assert_eq!(session.target, None, "{ticks_left} left");
            assert_eq!(session.nav_target(), None, "{ticks_left} left");
        }
        assert!(session.hold_expired(), "the last was the expiry's tick");
        session.target = Some(NpcId(0));
        session.nav_target = Some(StellarId(128));
        session.tick(Controls::default());
        assert_eq!(
            (session.target, session.nav_target()),
            (None, None),
            "and after it"
        );
    }

    #[test]
    fn a_destroyed_player_with_no_ending_recorded_starts_the_hold() {
        let catalog = deadly();
        let mut session = facing_an_npc(&catalog, 180);
        session.condition = Condition::Destroyed;
        session.tick(Controls::default());
        assert_eq!(
            session.player_fate(),
            PlayerFate::Holding {
                ticks_left: HOLD_TICKS - 1
            }
        );
    }

    #[test]
    fn the_scene_runs_on_during_the_hold() {
        let catalog = deadly();
        let mut session = facing_an_npc(&catalog, 180);
        until_exploded(&mut session, &catalog);
        session.take_combat_events();
        session.npc_mut(NpcId(0)).expect("the NPC").state.velocity = Vec2::new(1.0, 0.0);
        let before = session.npcs()[0].state.position;
        let mut fired = false;
        for _ in 0..40 {
            full_tick(&mut session, &catalog);
            assert!(matches!(session.player_fate(), PlayerFate::Holding { .. }));
            fired |= session.take_combat_events().iter().any(|event| {
                matches!(
                    event,
                    CombatEvent::Fired {
                        ship: ShipRef::Npc(NpcId(0)),
                        ..
                    }
                )
            });
        }
        assert_ne!(session.npcs()[0].state.position, before, "the NPC moves");
        assert!(fired, "the NPC fires");
    }

    /// A session whose player has exploded and whose hold has run out.
    fn expired_hold() -> Session {
        let catalog = deadly();
        let mut session = facing_an_npc(&catalog, 180);
        session.condition = Condition::Destroyed;
        session.player_end = Some(Ending::Holding { ticks_left: 0 });
        session.tick(Controls::default());
        assert!(session.hold_expired());
        session
    }

    #[test]
    fn begin_fade_moves_only_an_expired_hold_to_fading() {
        let catalog = deadly();
        let mut flying = facing_an_npc(&catalog, 180);
        flying.begin_fade();
        assert_eq!(flying.player_fate(), PlayerFate::Flying);
        let mut holding = facing_an_npc(&catalog, 180);
        holding.condition = Condition::Destroyed;
        holding.player_end = Some(Ending::Holding { ticks_left: 3 });
        holding.begin_fade();
        assert_eq!(holding.player_fate(), PlayerFate::Holding { ticks_left: 3 });
        let mut session = expired_hold();
        session.begin_fade();
        assert_eq!(session.player_fate(), PlayerFate::Fading);
        session.tick(Controls::default());
        assert_eq!(session.player_fate(), PlayerFate::Fading, "ticks keep it");
        session.end_play();
        session.begin_fade();
        assert_eq!(session.player_fate(), PlayerFate::GameOver, "not back");
    }

    #[test]
    fn end_play_moves_fading_to_game_over() {
        let mut session = expired_hold();
        session.end_play();
        assert_eq!(
            session.player_fate(),
            PlayerFate::Holding { ticks_left: 0 },
            "not before the fade"
        );
        session.begin_fade();
        session.end_play();
        assert_eq!(session.player_fate(), PlayerFate::GameOver);
        assert!(session.hold_expired());
        session.tick(Controls::default());
        assert_eq!(session.player_fate(), PlayerFate::GameOver);
    }

    #[test]
    fn a_dying_or_dead_player_may_not_be_saved() {
        let catalog = deadly();
        let mut session = facing_an_npc(&catalog, 180);
        assert!(session.may_save(), "flying");
        session.condition = Condition::Disabled;
        assert!(session.may_save(), "disabled");
        session.condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
        assert!(!session.may_save(), "breaking up");
        session.condition = Condition::Destroyed;
        session.player_exploded_this_tick = true;
        assert!(!session.may_save(), "exploded");
        session.player_exploded_this_tick = false;
        for ending in [
            Ending::Holding { ticks_left: 9 },
            Ending::Expired,
            Ending::Fading,
            Ending::GameOver,
        ] {
            session.player_end = Some(ending);
            assert!(!session.may_save(), "{ending:?}");
        }
    }

    #[test]
    fn only_a_strict_pilot_ending_play_dying_outside_the_pod_is_deleted() {
        let catalog = deadly();
        let mut session = facing_an_npc(&catalog, 180);
        let breaking_up = |starting| Condition::Dying {
            ticks_left: 9,
            starting,
        };
        // Each fate, set up directly, and whether the player is dying in
        // it (`_IsDying`).
        let fates = [
            (Condition::Intact, false, None, PlayerFate::Flying, false),
            (
                Condition::Disabled,
                false,
                None,
                PlayerFate::Disabled,
                false,
            ),
            (
                breaking_up(true),
                false,
                None,
                PlayerFate::BreakingUp {
                    ticks_left: 9,
                    starting: true,
                },
                true,
            ),
            (
                breaking_up(false),
                false,
                None,
                PlayerFate::BreakingUp {
                    ticks_left: 9,
                    starting: false,
                },
                true,
            ),
            (Condition::Destroyed, true, None, PlayerFate::Exploded, true),
            (
                Condition::Destroyed,
                false,
                Some(Ending::Holding { ticks_left: 9 }),
                PlayerFate::Holding { ticks_left: 9 },
                true,
            ),
            (
                Condition::Destroyed,
                false,
                Some(Ending::Expired),
                PlayerFate::Holding { ticks_left: 0 },
                true,
            ),
            (
                Condition::Destroyed,
                false,
                Some(Ending::Fading),
                PlayerFate::Fading,
                true,
            ),
            (
                Condition::Destroyed,
                false,
                Some(Ending::GameOver),
                PlayerFate::GameOver,
                true,
            ),
        ];
        // The escape ship keeps a real class (`_EscapeShipType` @0x6f4bb);
        // only the pod is spared.
        for (condition, exploded, ending, fate, dying) in fates {
            for strict in [false, true] {
                for (ship, pod) in [
                    (ShipId(128), false),
                    (ShipId(144), false),
                    (ESCAPE_POD_SHIP, true),
                ] {
                    session.condition = condition;
                    session.player_exploded_this_tick = exploded;
                    session.player_end = ending;
                    session.pilot.strict_play = strict;
                    session.pilot.ship = ship;
                    assert_eq!(session.player_fate(), fate);
                    assert_eq!(
                        session.deletes_pilot_on_ending_play(),
                        strict && dying && !pod,
                        "{fate:?} strict {strict} {ship:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_escape_pod_is_ship_895() {
        assert_eq!(ESCAPE_POD_SHIP, ShipId(895));
    }

    #[test]
    fn a_strict_pilot_ending_play_in_the_pod_is_not_deleted_nor_saved() {
        let mut session = Session::start(&armed()).expect("starts");
        session.pilot.strict_play = true;
        session.pilot.ship = ESCAPE_POD_SHIP;
        session.pod = Some(crate::POD_TICKS);
        assert_eq!(
            session.player_fate(),
            PlayerFate::Ejected {
                ticks_left: crate::POD_TICKS
            }
        );
        assert!(!session.may_save());
        assert!(!session.deletes_pilot_on_ending_play());
    }

    #[test]
    fn the_players_explosion_halts_the_klaxon_and_warp_sounds_after_the_klaxon() {
        // DeathDelay 2: a timer of 6, exploding 4 ticks after the breakup.
        let catalog = deadly_in(2);
        let mut session = facing_an_npc(&catalog, 180);
        let seen = from_the_breakup(&mut session, &catalog, 9);
        let halt = SimSound::Halt(&crate::sound::PLAYER_EXPLOSION_HALTS);
        let explosion = seen
            .iter()
            .position(|(condition, _)| *condition == Condition::Destroyed)
            .expect("it explodes");
        assert_eq!(
            seen[explosion].1,
            [SimSound::Klaxon, halt],
            "the halt follows the klaxon, once"
        );
        for (tick, (_, sounds)) in seen.iter().enumerate() {
            if tick != explosion {
                assert!(!sounds.contains(&halt), "tick {tick}: {sounds:?}");
            }
        }
        assert!(seen.len() > explosion + 3, "the hold is watched too");
    }

    #[test]
    fn the_player_explosions_halts_name_the_klaxon_and_the_warp_sounds() {
        assert_eq!(
            crate::sound::PLAYER_EXPLOSION_HALTS,
            [SoundId(371), SoundId(128), SoundId(129), SoundId(130)]
        );
    }

    #[test]
    fn an_npcs_explosion_raises_no_halt() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let mut sounds = Vec::new();
        for _ in 0..200 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            sounds.extend(session.take_sounds());
            if session.npcs().is_empty() {
                break;
            }
        }
        assert_eq!(session.npcs(), [], "the NPC is destroyed");
        assert!(
            !sounds
                .iter()
                .any(|sound| matches!(sound, SimSound::Halt(_))),
            "{sounds:?}"
        );
    }

    #[test]
    fn the_session_takes_the_commands_of_the_players_fate() {
        let catalog = deadly_in(2);
        let mut session = facing_an_npc(&catalog, 180);
        assert!(session.takes(PlayerCommand::Map));
        from_the_breakup(&mut session, &catalog, 1);
        assert!(matches!(
            session.player_fate(),
            PlayerFate::BreakingUp { .. }
        ));
        assert!(!session.takes(PlayerCommand::Map));
        assert!(session.takes(PlayerCommand::Target));
        while session.player_condition() != Condition::Destroyed {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert!(!session.takes(PlayerCommand::Target));
    }

    #[test]
    fn a_new_session_flies() {
        let session = Session::start(&armed()).expect("starts");
        assert_eq!(session.player_fate(), PlayerFate::Flying);
    }

    /// [`deadly`] with the player's ship of `DeathDelay` `death_delay`.
    pub(super) fn deadly_in(death_delay: i16) -> FakePilotCatalog {
        let mut catalog = deadly();
        for hull in &mut catalog.hulls {
            if hull.id == ShipId(128) {
                hull.death_delay = death_delay;
            }
        }
        catalog
    }

    /// The player's condition and the sounds of each fight tick from the
    /// one its armour goes on, under fire from `catalog`'s NPC, for up to
    /// `ticks` ticks; the traffic's own sounds are dropped.
    pub(super) fn from_the_breakup(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        ticks: usize,
    ) -> Vec<(Condition, Vec<SimSound>)> {
        let mut seen = Vec::new();
        for _ in 0..200 {
            session.tick_traffic(catalog, &Firing, &mut NeverFires);
            session.take_sounds();
            session.tick_combat(Rules::default(), &mut NeverFires);
            let sounds = session.take_sounds();
            let condition = session.player_condition();
            if !seen.is_empty() || !matches!(condition, Condition::Intact | Condition::Disabled) {
                seen.push((condition, sounds));
            }
            if seen.len() == ticks {
                break;
            }
        }
        seen
    }

    #[test]
    fn the_player_breaking_up_raises_the_klaxon_from_the_tick_after_to_the_explosion() {
        // DeathDelay 2: a timer of 6, exploding 4 ticks after the breakup.
        let catalog = deadly_in(2);
        let mut session = facing_an_npc(&catalog, 180);
        let seen = from_the_breakup(&mut session, &catalog, 7);
        let klaxon = || vec![SimSound::Klaxon];
        let dying = |ticks_left| Condition::Dying {
            ticks_left,
            starting: false,
        };
        assert_eq!(
            seen[..4],
            [
                (
                    Condition::Dying {
                        ticks_left: 6,
                        starting: true
                    },
                    vec![]
                ),
                (dying(5), klaxon()),
                (dying(4), klaxon()),
                (dying(3), klaxon()),
            ],
            "none on the breakup tick, then one each tick"
        );
        let (exploded, sounds) = &seen[4];
        assert_eq!(*exploded, Condition::Destroyed);
        assert_eq!(
            sounds.first(),
            Some(&SimSound::Klaxon),
            "first on the explosion's tick"
        );
        assert_eq!(
            seen[5..],
            [
                (Condition::Destroyed, vec![]),
                (Condition::Destroyed, vec![])
            ],
            "none after"
        );
    }

    #[test]
    fn an_npc_breaking_up_raises_no_klaxon() {
        let mut catalog = armed();
        for hull in &mut catalog.hulls {
            hull.death_delay = 10;
        }
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(FIRE);
        let mut sounds = Vec::new();
        let mut broke_up = false;
        for _ in 0..80 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            sounds.extend(session.take_sounds());
            broke_up |= session
                .npcs()
                .first()
                .is_some_and(|npc| matches!(npc.condition, Condition::Dying { .. }));
        }
        assert!(broke_up);
        assert_eq!(session.npcs(), [], "destroyed");
        assert_eq!(session.player_condition(), Condition::Intact);
        assert!(!sounds.contains(&SimSound::Klaxon), "{sounds:?}");
    }

    #[test]
    fn a_session_given_the_originals_breakup_rule_never_explodes_a_player_without_a_death_delay() {
        let catalog = deadly_in(0);
        let by_default = facing_an_npc(&catalog, 180);
        let mut session = by_default.clone();
        assert_eq!(
            from_the_breakup(&mut session, &catalog, 1),
            [(
                Condition::Destroyed,
                vec![SimSound::Halt(&crate::sound::PLAYER_EXPLOSION_HALTS)]
            )],
            "by default, destroyed as it breaks up, halting its sounds"
        );
        let mut session = by_default.with_rules(
            crate::rulebook::Rulebook::default()
                .with_override(RuleKey::BreakupStall, RuleSource::Engine),
        );
        let seen = from_the_breakup(&mut session, &catalog, 100);
        assert_eq!(seen.len(), 100);
        assert!(
            seen.iter().all(|(condition, sounds)| *condition
                == Condition::Dying {
                    ticks_left: 0,
                    starting: true
                }
                && sounds.is_empty()),
            "{seen:?}"
        );
    }

    #[test]
    fn diagnostics_are_drained_once() {
        let mut catalog = armed();
        catalog.weapons[0].flags2 = 0x8000;
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        for _ in 0..5 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(
            session.take_diagnostics(),
            [SimDiagnostic::UnimplementedWeaponFlag {
                weapon: WeaponId(128),
                field: crate::combat::flags::FlagField::Flags2,
                bit: 0x8000
            }]
        );
        assert_eq!(session.take_diagnostics(), []);
    }

    #[test]
    fn shields_regenerate_in_the_fight_at_the_ships_rate() {
        let mut catalog = armed();
        catalog.ships = vec![(
            ShipId(128),
            Ok(ShipFields {
                shield_rech: 1000,
                ..FAST
            }),
        )];
        let mut session = facing_an_npc(&catalog, 180);
        for _ in 0..12 {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        let hurt = session.reserves().shield.now;
        assert!(hurt < 30.0);
        session.traffic.remove(NpcId(0));
        session.combat.clear();
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.reserves().shield.now, hurt + 1.0);
    }

    // Refitting for the fight.

    use crate::catalog::BoomId;
    use crate::combat::armament::MOD_WEAPON;
    use crate::combat::weapon::PASSES_SHIELDS;

    /// The weapons the player fired in `events`.
    fn fired_by_the_player(events: &[CombatEvent]) -> Vec<WeaponId> {
        events
            .iter()
            .filter_map(|event| match event {
                CombatEvent::Fired {
                    ship: ShipRef::Player,
                    weapon,
                    ..
                } => Some(*weapon),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_weapon_outfit_bought_fires_once_the_ship_takes_off() {
        let mut catalog = FakePilotCatalog {
            weapons: vec![blaster()],
            hulls: vec![hull(128)],
            ..outfitting()
        };
        catalog.outfits.push(outfit(305, &[(MOD_WEAPON, 128)]));
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.take_combat_events(), [], "no weapon yet");
        land_now(&mut session).expect("lands at the outfitter");
        session
            .outfit(buy(OutfitId(305)), &mut NeverFires)
            .expect("bought");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("took off");
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            fired_by_the_player(&session.take_combat_events()),
            [WeaponId(128)]
        );
        assert_eq!(session.shots().len(), 1);
    }

    /// [`shipbuying`] where the player's ship 128 carries a blaster
    /// (weapon 128), 30 pixels across with `bööm` 133 destroying it, and
    /// ship 129 ([`NEW`]) a cannon (weapon 129), 120 pixels across with
    /// `bööm` 135; the system's traffic is a trader, ship 130, whose gun
    /// (weapon 130) passes the shields and destroys at one shot.
    fn armed_shipyard() -> FakePilotCatalog {
        let traffic = trafficked(130, 1, 1);
        let mut catalog = FakePilotCatalog {
            weapons: vec![
                blaster(),
                WeaponRecord {
                    id: WeaponId(129),
                    ..blaster()
                },
                WeaponRecord {
                    id: WeaponId(130),
                    mass_dmg: 1000,
                    flags: PASSES_SHIELDS,
                    ..blaster()
                },
            ],
            hulls: vec![
                armed_hull(128, 128),
                HullRecord {
                    size: Some(120),
                    explode2: 7,
                    ..armed_hull(129, 129)
                },
                armed_hull(130, 130),
            ],
            traffic: traffic.traffic,
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type: 1,
                    govt: Some(GovtId(140)),
                    ships: vec![(ShipId(130), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )],
            ..shipbuying()
        };
        catalog.ship_records.push(ship(130, FAST));
        catalog.outfits.extend([
            outfit(250, &[(MOD_WEAPON, 128)]),
            outfit(251, &[(MOD_WEAPON, 129)]),
            outfit(252, &[(MOD_WEAPON, 130)]),
        ]);
        catalog
    }

    #[test]
    fn the_players_hull_is_its_ship_types() {
        let session = Session::start(&armed()).expect("starts");
        assert_eq!(session.hull(), HullSpec::new(&armed_hull(128, 128)));
        assert_eq!(session.hull().death_delay, 2);
    }

    #[test]
    fn a_ship_bought_fights_with_its_own_weapons_size_and_explosion() {
        let catalog = armed_shipyard();
        let mut session = outfitted(&catalog);
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("took off");
        session.hold_trigger(FIRE);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            fired_by_the_player(&session.take_combat_events()),
            [WeaponId(129)],
            "the new ship's cannon, not the old one's blaster"
        );
        session.hold_trigger(Trigger::default());
        session.combat.clear();
        session.populate(&catalog, &mut Draws::of(&[6, 6, 0, 0, 750, 650, 180]));
        assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, -100.0));
        // The NPC fires straight down, passing 20 pixels from the player's
        // centre: beyond the old ship's hit radius (9.9), within the new
        // one's (39.6).
        session.player.position = Vec2::new(20.0, 0.0);
        let mut events = Vec::new();
        for _ in 0..20 {
            session.tick_traffic(&catalog, &Firing, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        assert_eq!(session.player_condition(), Condition::Destroyed);
        assert!(
            events.iter().any(|event| matches!(
                event,
                CombatEvent::Destroyed {
                    ship: ShipRef::Player,
                    ship_type: NEW,
                    explosion: Some(Explosion {
                        boom: BoomId(135),
                        extra: false
                    }),
                    ..
                }
            )),
            "{events:?}"
        );
    }

    // Targeting.

    use crate::targeting::TargetPick;

    /// Every NPC idles.
    #[derive(Debug)]
    struct Idling;

    impl Behaviour for Idling {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }
    }

    /// `catalog`'s session with two NPCs: 0 at (100, -100) and 1 at
    /// (-50, 0), nearer the player at the centre.
    fn two_npcs(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        let draws = [6, 6, 0, 0, 850, 650, 0, 2, 6, 6, 0, 0, 700, 750, 0, 2];
        session.populate(catalog, &mut Draws::of(&draws));
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
        session
    }

    fn target_id(session: &Session) -> Option<NpcId> {
        session.target().map(|npc| npc.id)
    }

    #[test]
    fn selecting_the_next_target_walks_the_npcs_then_none() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        assert_eq!(target_id(&session), None);
        let picks: Vec<_> = (0..4)
            .map(|_| session.select_target(TargetPick::Next))
            .collect();
        assert_eq!(
            picks,
            [Some(NpcId(0)), Some(NpcId(1)), None, Some(NpcId(0))]
        );
        assert_eq!(target_id(&session), Some(NpcId(0)));
        assert_eq!(session.target().map(|npc| npc.ship), Some(ShipId(129)));
    }

    #[test]
    fn selecting_the_nearest_target_picks_the_nearest_npc() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        assert_eq!(session.select_target(TargetPick::Nearest), Some(NpcId(1)));
        assert_eq!(target_id(&session), Some(NpcId(1)));
        session.player.position = Vec2::new(90.0, -90.0);
        assert_eq!(session.select_target(TargetPick::Nearest), Some(NpcId(0)));
    }

    #[test]
    fn selecting_the_nearest_threat_picks_the_nearest_npc_fighting_the_player() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        // NPC 1 is nearer; NPC 0 attacks the player.
        let threatened = |session: &mut Session, goal: Goal| {
            session.traffic.npcs_mut()[0].goal = goal;
            session.target = None;
            session.select_target(TargetPick::NearestThreat)
        };
        assert_eq!(
            threatened(&mut session, Goal::Attack(ShipRef::Player)),
            Some(NpcId(0))
        );
        assert_eq!(
            threatened(&mut session, Goal::Flee(ShipRef::Player)),
            Some(NpcId(0)),
            "fleeing from it is a threat, as _IsThreatToPlayer says"
        );
        assert_eq!(
            threatened(&mut session, Goal::Inspect(ShipRef::Player)),
            None
        );
        session.target = Some(NpcId(1));
        assert_eq!(
            session.select_target(TargetPick::NearestThreat),
            Some(NpcId(1)),
            "none: the target unchanged"
        );
        session.traffic.npcs_mut()[0].goal = Goal::Attack(ShipRef::Player);
        session.traffic.npcs_mut()[0].condition = Condition::Disabled;
        session.target = None;
        assert_eq!(
            session.select_target(TargetPick::NearestThreat),
            None,
            "a disabled ship is no threat"
        );
        assert_eq!(session.select_target(TargetPick::Nearest), Some(NpcId(1)));
    }

    #[test]
    fn the_nearest_with_nothing_to_pick_leaves_the_target_as_it_was() {
        let catalog = trafficked(130, 2, 3);
        let mut session = Session::start(&catalog).expect("starts");
        session.target = Some(NpcId(7));
        assert_eq!(session.select_target(TargetPick::Nearest), Some(NpcId(7)));
        assert_eq!(session.target, Some(NpcId(7)));
    }

    #[test]
    fn target_select_reaches_a_ship_that_filled_a_freed_berth_first() {
        let catalog = trafficked(130, 2, 3);
        let mut session = Session::start(&catalog).expect("starts");
        let ship = || crate::testkit::npc(0, crate::stats::ShipStats::default());
        for _ in 0..3 {
            session.traffic.add_npc(ship());
        }
        session.traffic.remove(NpcId(0));
        assert_eq!(session.traffic.add_npc(ship()), NpcId(3));
        assert_eq!(session.npc(NpcId(3)).map(|npc| npc.berth), Some(1));
        let walk = |session: &mut Session, pick| -> Vec<Option<NpcId>> {
            session.target = None;
            (0..4).map(|_| session.select_target(pick)).collect()
        };
        let up = [Some(NpcId(3)), Some(NpcId(1)), Some(NpcId(2)), None];
        let down = [Some(NpcId(2)), Some(NpcId(1)), Some(NpcId(3)), None];
        assert_eq!(walk(&mut session, TargetPick::Next), up);
        assert_eq!(walk(&mut session, TargetPick::Previous), down);
        for npc in session.traffic.npcs_mut() {
            npc.escort = Some(EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            });
        }
        assert_eq!(walk(&mut session, TargetPick::Next), [None; 4]);
        assert_eq!(walk(&mut session, TargetPick::NextEscort), up);
        assert_eq!(walk(&mut session, TargetPick::PreviousEscort), down);
    }

    #[test]
    fn nothing_is_targeted_while_landed() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        land_now(&mut session).expect("lands on planet 128");
        assert_eq!(session.select_target(TargetPick::Next), None);
        assert_eq!(session.select_target(TargetPick::Previous), None);
        assert_eq!(session.select_target(TargetPick::Nearest), None);
        assert_eq!(target_id(&session), None);
    }

    #[test]
    fn the_target_is_kept_through_a_jump_and_cleared_on_arrival() {
        let mut catalog = trafficked(130, 2, 3);
        catalog.traffic.push((SystemId(131), catalog.traffic[0].1));
        let mut session = two_npcs(&catalog);
        session.select_target(TargetPick::Next);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(session.select_target(TargetPick::Next), Some(NpcId(0)));
        assert_eq!(session.select_target(TargetPick::Nearest), Some(NpcId(0)));
        session.tick_traffic(&catalog, &Idling, &mut NeverFires);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(target_id(&session), Some(NpcId(0)), "kept while jumping");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert!(!session.npcs().is_empty());
        assert_eq!(target_id(&session), None, "cleared on arrival");
        assert_eq!(session.target, None);
    }

    #[test]
    fn the_target_is_kept_through_ticks_and_while_it_is_disabled() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.select_target(TargetPick::Next);
        for _ in 0..5 {
            session.tick(Controls::default());
            session.tick_traffic(&catalog, &Idling, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(target_id(&session), Some(NpcId(0)));
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(session.npcs()[0].condition, Condition::Disabled);
        assert_eq!(target_id(&session), Some(NpcId(0)), "a disabled target");
        assert_eq!(session.select_target(TargetPick::Nearest), Some(NpcId(0)));
    }

    #[test]
    fn the_target_is_cleared_as_soon_as_it_starts_breaking_up() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.select_target(TargetPick::Next);
        session.hold_trigger(FIRE);
        for _ in 0..200 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            let events = session.take_combat_events();
            if events
                .iter()
                .any(|event| matches!(event, CombatEvent::BreakingUp { .. }))
            {
                assert_eq!(npc_ids(&session), [NpcId(0)], "still breaking up");
                assert_eq!(target_id(&session), None);
                assert_eq!(session.target, None);
                return;
            }
            assert_eq!(target_id(&session), Some(NpcId(0)));
        }
        panic!("never broke up: {:?}", session.npcs());
    }

    #[test]
    fn the_target_is_cleared_when_it_is_destroyed_and_gone() {
        let mut catalog = armed();
        catalog.hulls[1].death_delay = 0;
        let mut session = facing_an_npc(&catalog, 180);
        session.select_target(TargetPick::Next);
        session.traffic.npcs_mut()[0].reserves.armor.now = 0.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(npc_ids(&session), [], "destroyed at once");
        assert_eq!(session.target, None);
    }

    #[test]
    fn the_target_is_cleared_when_it_lands_or_jumps_out() {
        for ai_type in [1, 3] {
            let catalog = trafficked(130, 1, ai_type);
            let mut session = Session::start(&catalog).expect("starts");
            session.populate(&catalog, &mut NeverFires);
            session.select_target(TargetPick::Next);
            assert_eq!(target_id(&session), Some(NpcId(0)));
            first_departure(&mut session, &catalog);
            assert_eq!(session.departed()[0].0, NpcId(0), "AI {ai_type}");
            assert_eq!(session.target, None, "AI {ai_type}");
        }
    }

    #[test]
    fn the_target_is_cleared_when_the_system_is_populated_afresh() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        session.select_target(TargetPick::Next);
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(session.target, None);
    }

    #[test]
    fn the_target_is_cleared_on_landing() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        session.select_target(TargetPick::Next);
        land_now(&mut session).expect("lands on planet 128");
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
        assert_eq!(session.target, None);
    }

    // The secondary weapon.

    use crate::combat::weapon::SECONDARY;

    const ROCKET: WeaponId = WeaponId(140);
    const MISSILE: WeaponId = WeaponId(141);
    const TORCH: WeaponId = WeaponId(142);

    /// Secondary weapon `id`, a [`blaster`] spending `ammo_type`.
    fn secondary(id: i16, ammo_type: i16) -> WeaponRecord {
        WeaponRecord {
            id: WeaponId(id),
            flags: SECONDARY,
            ammo_type,
            ..blaster()
        }
    }

    /// One of each of `weapons`.
    fn carrying(id: i16, weapons: &[i16]) -> HullRecord {
        HullRecord {
            weapons: weapons
                .iter()
                .map(|&weapon| StockWeapon {
                    weapon: WeaponId(weapon),
                    count: 1,
                    ammo: 0,
                })
                .collect(),
            ..hull(id)
        }
    }

    /// `catalog` with ship 128 carrying a blaster (128) and three
    /// secondaries: rockets ([`ROCKET`]) firing rounds of their own, of
    /// which its default ammunition outfit 310 brings 3; missiles
    /// ([`MISSILE`]), unlimited; and a torch ([`TORCH`]) burning fuel. Ship
    /// 129 carries a blaster, a torch and missiles. Outfits 311-314 hold
    /// the blaster and the three secondaries, in that order.
    fn with_secondaries(catalog: FakePilotCatalog) -> FakePilotCatalog {
        let mut outfits = catalog.outfits.clone();
        outfits.push(outfit(310, &[(MOD_AMMO, 140)]));
        outfits.extend(
            [(311, 128), (312, 140), (313, 141), (314, 142)]
                .map(|(id, weapon)| outfit(id, &[(MOD_WEAPON, weapon)])),
        );
        FakePilotCatalog {
            weapons: vec![
                blaster(),
                secondary(140, 12),
                secondary(141, -1),
                secondary(142, -1100),
            ],
            hulls: vec![
                carrying(128, &[128, 140, 141, 142]),
                carrying(129, &[128, 142, 141]),
            ],
            outfits,
            defaults: vec![(ShipId(128), vec![(OutfitId(310), 3)])],
            ..catalog
        }
    }

    #[test]
    fn no_secondary_is_selected_at_start_or_on_a_reload() {
        let armed = with_secondaries(catalog());
        let mut session = Session::start(&armed).expect("starts");
        assert_eq!(session.secondary(), None, "as _ResetPlayer leaves it");
        assert_eq!(session.secondary_line_count(), None);
        session.select_secondary(false);
        assert_eq!(session.secondary(), Some(ROCKET));
        let reloaded = Session::fly(&armed, session.pilot().clone()).expect("flies");
        assert_eq!(reloaded.secondary(), None, "not saved");
        assert_eq!(reloaded.secondary_line_count(), None);
        let unarmed = Session::start(&catalog()).expect("starts");
        assert_eq!(unarmed.secondary(), None);
        assert_eq!(unarmed.secondary_line_count(), None);
    }

    #[test]
    fn selecting_a_secondary_cycles_through_them_either_way() {
        let mut session = Session::start(&with_secondaries(catalog())).expect("starts");
        let mut picked = Vec::new();
        for backwards in [false, false, false, true, true] {
            session.select_secondary(backwards);
            picked.push(session.secondary());
        }
        assert_eq!(
            picked,
            [
                Some(ROCKET),
                Some(MISSILE),
                Some(TORCH),
                Some(MISSILE),
                Some(ROCKET)
            ]
        );
    }

    #[test]
    fn holding_fire_fires_the_selected_secondary_and_the_primaries_as_asked() {
        let catalog = with_secondaries(catalog());
        for (primary, secondary, fired) in [
            (false, true, vec![ROCKET]),
            (true, false, vec![WeaponId(128)]),
            (true, true, vec![WeaponId(128), ROCKET]),
            (false, false, vec![]),
        ] {
            let mut session = Session::start(&catalog).expect("starts");
            session.select_secondary(false);
            session.hold_fire(primary, secondary);
            session.tick_combat(Rules::default(), &mut NeverFires);
            assert_eq!(
                fired_by_the_player(&session.take_combat_events()),
                fired,
                "{primary} {secondary}"
            );
        }
        let mut session = Session::start(&catalog).expect("starts");
        session.select_secondary(false);
        session.select_secondary(false);
        session.hold_fire(false, true);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            fired_by_the_player(&session.take_combat_events()),
            [MISSILE]
        );
        let mut none = Session::start(&catalog).expect("starts");
        none.hold_fire(false, true);
        none.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            fired_by_the_player(&none.take_combat_events()),
            [],
            "none selected"
        );
    }

    #[test]
    fn the_secondary_lines_count_is_its_ammunitions_rounds_unless_hidden() {
        let mut session = Session::start(&with_secondaries(catalog())).expect("starts");
        assert_eq!(session.secondary_line_count(), None, "none selected");
        session.select_secondary(false);
        assert_eq!(session.secondary_line_count(), Some(3));
        session.hold_fire(false, true);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.secondary_line_count(), Some(2));
        session.select_secondary(false);
        assert_eq!(session.secondary_line_count(), None, "unlimited");
        session.select_secondary(false);
        assert_eq!(session.secondary_line_count(), None, "a torch burns fuel");
        let mut hidden = with_secondaries(catalog());
        hidden.weapons[1].flags2 = crate::combat::weapon::HIDES_COUNT;
        let mut session = Session::start(&hidden).expect("starts");
        session.select_secondary(false);
        assert_eq!(session.secondary(), Some(ROCKET));
        assert_eq!(session.secondary_line_count(), None, "Flags2 0x0040");
    }

    #[test]
    fn an_outfit_bought_keeps_the_secondary_selected() {
        let catalog = with_secondaries(outfitting());
        let mut session = outfitted(&catalog);
        session.select_secondary(false);
        session.select_secondary(false);
        assert_eq!(session.secondary(), Some(MISSILE));
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        assert_eq!(session.secondary(), Some(MISSILE));
    }

    /// Selects the `nth` secondary (from 1), stepping on from the one
    /// selected.
    fn selecting(session: &mut Session, nth: usize) {
        for _ in 0..nth {
            session.select_secondary(false);
        }
    }

    /// Runs `script` on `session` as a hook would.
    fn scripted(session: &mut Session, script: &str) {
        session.run_script(&crate::control::Script::parse(script), &mut NeverFires);
    }

    #[test]
    fn a_ship_bought_leaves_no_secondary_selected() {
        let catalog = with_secondaries(shipbuying());
        let mut session = outfitted(&catalog);
        selecting(&mut session, 3);
        assert_eq!(session.secondary(), Some(TORCH), "which ship 129 carries");
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(session.secondary(), None, "_DoShipyardDialog @0x5f01c");
        let mut session = outfitted(&catalog);
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(session.secondary(), None, "none before");
        let mut granting = with_secondaries(shipbuying());
        granting.ship_records[1].on_purchase = crate::control::Script::parse("G312");
        let mut session = outfitted(&granting);
        selecting(&mut session, 1);
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(session.pilot().owned(OutfitId(312)), 1, "granted");
        assert_eq!(session.secondary(), None, "cleared after OnPurchase");
    }

    #[test]
    fn use_as_my_ship_leaves_no_secondary_selected() {
        let catalog = ammo_aboard(1, 20);
        let assigned = |choice| {
            let mut session = ammo_session(&catalog);
            selecting(&mut session, 1);
            assert_eq!(session.secondary(), Some(ROCKET));
            assert_eq!(
                take(&mut session, Take::Capture, &[43, 1]).0,
                Taken::Captured
            );
            session
                .assign(choice, &mut Draws::of(&[0]))
                .expect("assigned");
            session.secondary()
        };
        assert_eq!(
            assigned(Assignment::MyShip),
            None,
            "_DoShipCapture @0x4173e"
        );
        assert_eq!(
            assigned(Assignment::Escort),
            Some(ROCKET),
            "the old ship kept"
        );
    }

    /// `catalog`'s session landed, owning `outfits` alone, refitted, and
    /// with the `nth` secondary selected.
    fn selling_from(catalog: &FakePilotCatalog, outfits: &[(i16, u16)], nth: usize) -> Session {
        let mut session = outfitted(catalog);
        session.pilot.outfits = outfits
            .iter()
            .map(|&(id, count)| (OutfitId(id), count))
            .collect();
        session.refit(false, ReservePolicy::Hold);
        selecting(&mut session, nth);
        session
    }

    #[test]
    fn selling_the_last_launcher_of_the_selected_secondary_lets_it_go() {
        let catalog = with_secondaries(outfitting());
        let rockets = OutfitId(312);
        let mut session = selling_from(&catalog, &[(312, 2), (313, 1)], 1);
        assert_eq!(session.secondary(), Some(ROCKET));
        session
            .outfit(sell(rockets), &mut NeverFires)
            .expect("sold");
        assert_eq!(session.secondary(), Some(ROCKET), "one left");
        session
            .outfit(sell(rockets), &mut NeverFires)
            .expect("sold");
        assert_eq!(session.secondary(), None, "_DoOutfitDialog @0x5ce82");
        let mut other = selling_from(&catalog, &[(312, 1), (313, 1)], 1);
        other
            .outfit(sell(OutfitId(313)), &mut NeverFires)
            .expect("sold");
        assert_eq!(other.secondary(), Some(ROCKET), "another secondary sold");
        let mut ammo = selling_from(&catalog, &[(310, 1), (312, 1)], 1);
        ammo.outfit(sell(OutfitId(310)), &mut NeverFires)
            .expect("sold");
        assert_eq!(ammo.pilot().owned(OutfitId(310)), 0);
        assert_eq!(
            ammo.secondary(),
            Some(ROCKET),
            "its ammunition mounts no weapon"
        );
        let mut counted = selling_from(&catalog, &[(312, 2)], 1);
        assert_eq!(
            counted.outfit_counted(sell(rockets), 2, &mut NeverFires),
            Ok(2)
        );
        assert_eq!(counted.secondary(), None, "a counted sale");
        let mut on_sell = with_secondaries(outfitting());
        for record in &mut on_sell.outfits {
            if record.id == rockets {
                record.on_sell = crate::control::Script::parse("G312");
            }
        }
        let mut regranted = selling_from(&on_sell, &[(312, 1)], 1);
        regranted
            .outfit(sell(rockets), &mut NeverFires)
            .expect("sold");
        assert_eq!(regranted.pilot().owned(rockets), 1, "granted back");
        assert_eq!(regranted.secondary(), None, "cleared before OnSell");
    }

    #[test]
    fn selling_one_of_two_outfits_mounting_the_selected_weapon_lets_it_go() {
        let mut catalog = with_secondaries(outfitting());
        catalog.outfits.push(outfit(315, &[(MOD_WEAPON, 140)]));
        let mut session = selling_from(&catalog, &[(312, 1), (315, 1)], 1);
        assert_eq!(session.secondary(), Some(ROCKET));
        session
            .outfit(sell(OutfitId(312)), &mut NeverFires)
            .expect("sold");
        assert_eq!(
            session.secondary(),
            None,
            "@0x5cf78 checks the sold outfit alone"
        );
    }

    #[test]
    fn buying_an_outfit_leaves_the_selection_alone() {
        let catalog = with_secondaries(outfitting());
        let mut session = selling_from(&catalog, &[], 0);
        session
            .outfit(buy(OutfitId(312)), &mut NeverFires)
            .expect("bought");
        assert_eq!(session.pilot().owned(OutfitId(312)), 1);
        assert_eq!(session.secondary(), None);
    }

    #[test]
    fn g_and_d_leave_the_secondary_selection_alone() {
        let armed = with_secondaries(catalog());
        let mut session = Session::start(&armed).expect("starts");
        scripted(&mut session, "G312");
        assert_eq!(session.pilot().owned(OutfitId(312)), 2);
        assert_eq!(session.secondary(), None, "G selects nothing");
        let mut session = Session::start(&armed).expect("starts");
        selecting(&mut session, 1);
        scripted(&mut session, "D312");
        assert_eq!(session.pilot().owned(OutfitId(312)), 0);
        assert_eq!(session.secondary(), Some(ROCKET), "still named");
        session.hold_fire(false, true);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            fired_by_the_player(&session.take_combat_events()),
            [],
            "nothing mounted to fire"
        );
        let mut session = Session::start(&armed).expect("starts");
        selecting(&mut session, 2);
        scripted(&mut session, "D312");
        assert_eq!(session.secondary(), Some(MISSILE), "still carried");
    }

    #[test]
    fn a_change_of_ship_by_script_leaves_the_secondary_selected() {
        let catalog = with_secondaries(shipbuying());
        let mut session = outfitted(&catalog);
        selecting(&mut session, 3);
        scripted(&mut session, "C129");
        assert_eq!(session.ship(), NEW);
        assert_eq!(session.secondary(), Some(TORCH));
        let mut session = outfitted(&catalog);
        selecting(&mut session, 1);
        scripted(&mut session, "H129");
        assert_eq!(session.ship(), NEW);
        assert_eq!(session.pilot().owned(OutfitId(312)), 0, "not carried");
        assert_eq!(session.secondary(), Some(ROCKET), "still named");
    }

    /// The rule set reading the secondary selection's `rule` by `source`,
    /// every other rule by the engine.
    fn secondary_rules(rule: RuleKey, source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(rule, source)
    }

    /// [`with_secondaries`] over `catalog`, with outfit 315 mounting the
    /// rockets too.
    fn with_two_rocket_outfits(catalog: FakePilotCatalog) -> FakePilotCatalog {
        let mut catalog = with_secondaries(catalog);
        catalog.outfits.push(outfit(315, &[(MOD_WEAPON, 140)]));
        catalog
    }

    #[test]
    fn d_of_the_selected_weapons_last_launcher_by_each_reading() {
        let armed = with_two_rocket_outfits(catalog());
        for (source, last) in [
            (RuleSource::Engine, Some(ROCKET)),
            (RuleSource::Bible, None),
        ] {
            let rules = secondary_rules(RuleKey::StaleSecondary, source);
            let run = |nth: usize, scripts: &[&str]| {
                let mut session = Session::start(&armed).expect("starts").with_rules(rules);
                assert_eq!(session.pilot().owned(OutfitId(312)), 1, "stock");
                assert_eq!(session.pilot().owned(OutfitId(315)), 0);
                selecting(&mut session, nth);
                for script in scripts {
                    scripted(&mut session, script);
                }
                session.secondary()
            };
            assert_eq!(run(1, &["D312"]), last, "{source:?}: the last launcher");
            assert_eq!(
                run(1, &["G312", "D312"]),
                Some(ROCKET),
                "{source:?}: one of two left"
            );
            assert_eq!(
                run(1, &["G315", "D312"]),
                Some(ROCKET),
                "{source:?}: 315 still carries it"
            );
            assert_eq!(
                run(1, &["D310"]),
                Some(ROCKET),
                "{source:?}: its ammunition mounts nothing"
            );
            assert_eq!(
                run(2, &["D312"]),
                Some(MISSILE),
                "{source:?}: another secondary"
            );
            assert_eq!(run(0, &["G312"]), None, "{source:?}: G selects nothing");
            assert_eq!(run(1, &["G312"]), Some(ROCKET), "{source:?}: G keeps it");
        }
    }

    #[test]
    fn a_change_of_ship_by_script_by_each_reading() {
        let catalog = with_secondaries(shipbuying());
        let mut persistent = with_secondaries(shipbuying());
        for record in &mut persistent.outfits {
            if record.id == OutfitId(312) {
                record.flags = OutfitFlags::PERSISTENT;
            }
        }
        for (source, stripped) in [
            (RuleSource::Engine, Some(ROCKET)),
            (RuleSource::Bible, None),
        ] {
            let rules = secondary_rules(RuleKey::StaleSecondary, source);
            let run = |catalog: &FakePilotCatalog, nth: usize, script: &str| {
                let mut session = outfitted(catalog).with_rules(rules);
                selecting(&mut session, nth);
                scripted(&mut session, script);
                assert_eq!(session.ship(), NEW);
                (session.pilot().owned(OutfitId(312)), session.secondary())
            };
            assert_eq!(
                run(&catalog, 1, "C129"),
                (1, Some(ROCKET)),
                "{source:?}: C keeps every outfit"
            );
            assert_eq!(
                run(&catalog, 1, "E129"),
                (1, Some(ROCKET)),
                "{source:?}: E keeps every outfit"
            );
            assert_eq!(
                run(&catalog, 1, "H129"),
                (0, stripped),
                "{source:?}: H strips the launcher"
            );
            assert_eq!(
                run(&catalog, 3, "H129").1,
                Some(TORCH),
                "{source:?}: ship 129 stocks the torch"
            );
            assert_eq!(
                run(&persistent, 1, "H129"),
                (1, Some(ROCKET)),
                "{source:?}: a persistent launcher is kept"
            );
        }
    }

    #[test]
    fn selling_one_of_two_outfits_mounting_the_selected_weapon_by_each_reading() {
        let catalog = with_two_rocket_outfits(outfitting());
        for (source, kept) in [
            (RuleSource::Engine, None),
            (RuleSource::Bible, Some(ROCKET)),
        ] {
            let mut session = selling_from(&catalog, &[(312, 1), (315, 1)], 1)
                .with_rules(secondary_rules(RuleKey::SecondarySale, source));
            assert_eq!(session.secondary(), Some(ROCKET));
            session
                .outfit(sell(OutfitId(312)), &mut NeverFires)
                .expect("sold");
            assert_eq!(session.secondary(), kept, "{source:?}");
        }
        let mut session = selling_from(&catalog, &[(312, 1), (315, 1)], 1)
            .with_rules(secondary_rules(RuleKey::SecondarySale, RuleSource::Bible));
        for outfit in [312, 315] {
            session
                .outfit(sell(OutfitId(outfit)), &mut NeverFires)
                .expect("sold");
        }
        assert_eq!(session.secondary(), None, "neither left");
    }

    #[test]
    fn selling_the_last_launcher_by_the_other_reading() {
        let catalog = with_two_rocket_outfits(outfitting());
        let rules = secondary_rules(RuleKey::SecondarySale, RuleSource::Bible);
        let rockets = OutfitId(312);
        let from = |catalog: &FakePilotCatalog, outfits: &[(i16, u16)]| {
            selling_from(catalog, outfits, 1).with_rules(rules)
        };
        let mut session = from(&catalog, &[(312, 2), (313, 1)]);
        session
            .outfit(sell(rockets), &mut NeverFires)
            .expect("sold");
        assert_eq!(session.secondary(), Some(ROCKET), "one left");
        session
            .outfit(sell(rockets), &mut NeverFires)
            .expect("sold");
        assert_eq!(session.secondary(), None, "none left");
        let mut other = from(&catalog, &[(312, 1), (313, 1)]);
        other
            .outfit(sell(OutfitId(313)), &mut NeverFires)
            .expect("sold");
        assert_eq!(other.secondary(), Some(ROCKET), "another secondary sold");
        let mut ammo = from(&catalog, &[(310, 1), (312, 1)]);
        ammo.outfit(sell(OutfitId(310)), &mut NeverFires)
            .expect("sold");
        assert_eq!(ammo.secondary(), Some(ROCKET), "its ammunition sold");
        let mut counted = from(&catalog, &[(312, 2)]);
        assert_eq!(
            counted.outfit_counted(sell(rockets), 2, &mut NeverFires),
            Ok(2)
        );
        assert_eq!(counted.secondary(), None, "a counted sale");
        let mut on_sell = with_two_rocket_outfits(outfitting());
        for record in &mut on_sell.outfits {
            if record.id == rockets {
                record.on_sell = crate::control::Script::parse("G312");
            }
        }
        let mut regranted = from(&on_sell, &[(312, 1)]);
        regranted
            .outfit(sell(rockets), &mut NeverFires)
            .expect("sold");
        assert_eq!(regranted.pilot().owned(rockets), 1, "granted back");
        assert_eq!(regranted.secondary(), None, "checked before OnSell");
        let mut unowned = from(&catalog, &[(312, 1), (315, 0)]);
        unowned
            .outfit(sell(rockets), &mut NeverFires)
            .expect("sold");
        assert_eq!(
            unowned.secondary(),
            None,
            "315 owned 0 times carries nothing"
        );
    }

    #[test]
    fn the_two_secondary_rules_are_read_apart() {
        let outfitting = with_two_rocket_outfits(outfitting());
        let armed = with_secondaries(catalog());
        for (rule, sold, dropped) in [
            (RuleKey::StaleSecondary, None, None),
            (RuleKey::SecondarySale, Some(ROCKET), Some(ROCKET)),
        ] {
            let rules = secondary_rules(rule, RuleSource::Bible);
            let mut session = selling_from(&outfitting, &[(312, 1), (315, 1)], 1).with_rules(rules);
            session
                .outfit(sell(OutfitId(312)), &mut NeverFires)
                .expect("sold");
            assert_eq!(session.secondary(), sold, "{rule:?}: the sale");
            let mut session = Session::start(&armed).expect("starts").with_rules(rules);
            selecting(&mut session, 1);
            scripted(&mut session, "D312");
            assert_eq!(session.secondary(), dropped, "{rule:?}: D of the last");
        }
    }

    #[test]
    fn a_ship_bought_or_captured_leaves_no_secondary_by_either_reading() {
        let both = Rulebook::default()
            .with_override(RuleKey::StaleSecondary, RuleSource::Bible)
            .with_override(RuleKey::SecondarySale, RuleSource::Bible);
        let catalog = with_secondaries(shipbuying());
        let mut session = outfitted(&catalog).with_rules(both);
        selecting(&mut session, 3);
        assert_eq!(session.secondary(), Some(TORCH), "which ship 129 carries");
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(session.secondary(), None, "a ship bought");
        let catalog = ammo_aboard(1, 20);
        let mut session = ammo_session(&catalog);
        session.rules = both;
        selecting(&mut session, 1);
        assert_eq!(session.secondary(), Some(ROCKET));
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 1]).0,
            Taken::Captured
        );
        session
            .assign(Assignment::MyShip, &mut Draws::of(&[0]))
            .expect("assigned");
        assert_eq!(session.secondary(), None, "a ship captured");
    }

    // Guided weapons, turrets and point defence.

    /// `weapon` 130 (with blaster damage) of `guidance`, firing every
    /// `reload` ticks.
    fn guided(guidance: i16, reload: i16) -> WeaponRecord {
        WeaponRecord {
            id: WeaponId(130),
            reload,
            guidance,
            ..blaster()
        }
    }

    /// [`trafficked`] with the player carrying `player` and its traffic
    /// carrying `npc`, the player's ship type's `Flags` `flags`; outfit
    /// 250 holds the player's weapon.
    fn arming(player: WeaponRecord, npc: WeaponRecord, flags: u16) -> FakePilotCatalog {
        FakePilotCatalog {
            weapons: vec![player, npc],
            outfits: vec![outfit(250, &[(MOD_WEAPON, player.id.0)])],
            hulls: vec![
                HullRecord {
                    flags,
                    ..armed_hull(128, player.id.0)
                },
                armed_hull(129, npc.id.0),
            ],
            ..trafficked(130, 1, 1)
        }
    }

    /// `catalog`'s session with its one NPC at (`x`, `y`) from the player
    /// (at the centre, facing up), facing `heading`, at rest.
    fn with_an_npc_at(catalog: &FakePilotCatalog, x: u32, y: u32, heading: u32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.populate(catalog, &mut Draws::of(&[6, 6, 0, 0, x, y, heading]));
        stop_npcs(&mut session);
        session
    }

    #[test]
    fn the_player_fires_a_turret_at_its_target_astern_unless_its_ship_is_blind_there() {
        for (flags, fires) in [(0, true), (0x4000, false)] {
            let catalog = arming(guided(4, 2), blaster(), flags);
            let mut session = with_an_npc_at(&catalog, 750, 850, 0);
            assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, 100.0));
            session.select_target(TargetPick::Nearest);
            session.hold_fire(true, false);
            for _ in 0..12 {
                session.tick_combat(Rules::default(), &mut NeverFires);
            }
            let shield = session.npcs()[0].reserves.shield.now;
            assert_eq!(shield < 30.0, fires, "{flags:#x}: {shield}");
        }
        let catalog = arming(guided(4, 2), blaster(), 0);
        let mut session = with_an_npc_at(&catalog, 750, 850, 0);
        session.hold_fire(true, false);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.shots(), [], "no target, no fire");
    }

    /// A homing missile: 10 pixels a tick for 60 ticks, turning 7 degrees
    /// a tick, standing 4 of point defence.
    fn homing_missile(id: i16, reload: i16) -> WeaponRecord {
        WeaponRecord {
            id: WeaponId(id),
            guidance: 1,
            reload,
            count: 60,
            speed: 1000,
            guided_turn: 70,
            durability: 4,
            ..blaster()
        }
    }

    #[test]
    fn the_players_homing_missile_hits_its_target_and_without_one_flies_through_it() {
        for (targeted, hit) in [(true, true), (false, false)] {
            let catalog = arming(homing_missile(134, 1000), blaster(), 0);
            let mut session = facing_an_npc(&catalog, 180);
            if targeted {
                session.select_target(TargetPick::Nearest);
            }
            session.hold_fire(true, false);
            for _ in 0..30 {
                session.tick_combat(Rules::default(), &mut NeverFires);
                session.hold_fire(false, false);
            }
            let shield = session.npcs()[0].reserves.shield.now;
            assert_eq!(shield < 30.0, hit, "{targeted}: {shield}");
            assert_eq!(
                session.shots().is_empty(),
                hit,
                "{targeted}: spent or flying on"
            );
        }
    }

    /// Every NPC idles, targets the player and holds its trigger.
    #[derive(Debug)]
    struct Attacking;

    impl Behaviour for Attacking {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }

        fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
            FIRE
        }

        fn target(&self, _npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            Some(ShipRef::Player)
        }
    }

    /// Every NPC attacks the player, firing as Nova's AI picks
    /// ([`crate::ai::fire::trigger`]).
    #[derive(Debug)]
    struct Missileer;

    impl Behaviour for Missileer {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Attack(ShipRef::Player)
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            crate::ai::fire::trigger(npc, around, crate::ai::fire::FireRules::default())
        }

        fn target(&self, _npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            Some(ShipRef::Player)
        }
    }

    #[test]
    fn an_npc_holds_its_next_missile_while_the_one_in_flight_covers_its_target() {
        // The player holds 30 shield and 45 armour: 1.05 x 75 = 78.75. A
        // missile of 79 mass and 79 energy aims 79 at it, one of 78 aims
        // 78. It flies 1 pixel a tick, so the first is still in flight.
        // The NPC's short gun (52 pixels' reach) keeps the missile from
        // being its gun.
        let launches = |damage: i16| {
            let missile = WeaponRecord {
                speed: 100,
                count: 400,
                reload: 0,
                mass_dmg: damage,
                energy_dmg: damage,
                ..homing_missile(134, 0)
            };
            let short = WeaponRecord {
                id: WeaponId(135),
                count: 1,
                ..blaster()
            };
            let mut catalog = arming(blaster(), missile, 0);
            catalog.weapons.push(short);
            catalog.hulls[1].weapons.insert(
                0,
                StockWeapon {
                    weapon: WeaponId(135),
                    count: 1,
                    ammo: 0,
                },
            );
            let mut session = with_an_npc_at(&catalog, 750, 550, 180);
            let mut fired = 0;
            for _ in 0..4 {
                session.tick_traffic(&catalog, &Missileer, &mut NeverFires);
                session.tick_combat(Rules::default(), &mut NeverFires);
                fired += session
                    .take_combat_events()
                    .iter()
                    .filter(|event| {
                        matches!(event, CombatEvent::Fired { weapon, .. } if *weapon == WeaponId(134))
                    })
                    .count();
            }
            fired
        };
        assert_eq!(launches(79), 1, "the first suffices");
        assert!(launches(78) > 1, "the first falls short");
    }

    /// Every NPC idles, targets the player and fires weapon 131 alone,
    /// skipping Nova's AI pick.
    #[derive(Debug)]
    struct Picking;

    impl Behaviour for Picking {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }

        fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
            Trigger {
                only: Some(WeaponId(131)),
                ..Trigger::default()
            }
        }

        fn target(&self, _npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            Some(ShipRef::Player)
        }
    }

    #[test]
    fn a_cloaked_npc_fires_only_a_weapon_that_fires_cloaked() {
        for (flags2, cloaked, fires) in [
            (0, false, true),
            (0, true, false),
            (crate::combat::weapon::FIRES_CLOAKED, true, true),
        ] {
            let gun = WeaponRecord {
                id: WeaponId(131),
                flags2,
                ..blaster()
            };
            let catalog = arming(blaster(), gun, 0);
            let mut session = with_an_npc_at(&catalog, 750, 550, 180);
            session.tick_traffic(&catalog, &Picking, &mut NeverFires);
            if cloaked {
                session.traffic.npcs_mut()[0].cloak = CLOAKED;
            }
            session.tick_combat(Rules::default(), &mut NeverFires);
            let fired = session.take_combat_events().iter().any(|event| {
                matches!(event, CombatEvent::Fired { weapon, .. } if *weapon == WeaponId(131))
            });
            assert_eq!(fired, fires, "{flags2:#x} cloaked {cloaked}");
        }
    }

    #[test]
    fn the_players_point_defence_shoots_down_an_npcs_missile_with_no_target_of_its_own() {
        let quad = WeaponRecord {
            id: WeaponId(133),
            guidance: 9,
            reload: 5,
            count: 12,
            speed: 2000,
            mass_dmg: 1,
            energy_dmg: 4,
            ..blaster()
        };
        let slow = WeaponRecord {
            speed: 500,
            count: 200,
            ..homing_missile(134, 1000)
        };
        let catalog = arming(quad, slow, 0);
        let mut session = with_an_npc_at(&catalog, 750, 550, 180);
        assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, -200.0));
        let mut events = Vec::new();
        for _ in 0..40 {
            session.tick_traffic(&catalog, &Attacking, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        assert_eq!(session.target(), None, "the player targets nothing");
        assert!(
            events.contains(&CombatEvent::Fired {
                ship: ShipRef::Npc(NpcId(0)),
                weapon: WeaponId(134),
                at: Vec2::new(0.0, -200.0)
            }),
            "{events:?}"
        );
        let downed = events
            .iter()
            .filter(|event| matches!(event, CombatEvent::ShotDown { .. }))
            .count();
        assert_eq!(downed, 1, "{events:?}");
        assert_eq!(session.reserves().shield.now, 30.0, "untouched");
        assert!(
            session
                .shots()
                .iter()
                .all(|shot| shot.weapon.id != WeaponId(134)),
            "the missile is gone"
        );
    }

    /// Every NPC takes `goal`, fires as Nova's AI picks
    /// ([`crate::ai::fire::trigger`]) and targets what `goal` is about.
    #[derive(Debug)]
    struct Minding {
        goal: Goal,
    }

    impl Behaviour for Minding {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            self.goal
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            crate::ai::fire::trigger(npc, around, crate::ai::fire::FireRules::default())
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.goal.quarry()
        }
    }

    #[test]
    fn an_npc_not_fighting_lets_a_players_missile_through_and_one_fighting_shoots_it_down() {
        // The NPC's point defence runs only when its AI asks for a turret
        // (`_AIFireTurret` @0x80b0d).
        let quad = WeaponRecord {
            id: WeaponId(133),
            guidance: 9,
            reload: 5,
            count: 12,
            speed: 2000,
            mass_dmg: 1,
            energy_dmg: 4,
            ..blaster()
        };
        let slow = WeaponRecord {
            speed: 500,
            count: 200,
            ..homing_missile(134, 1000)
        };
        let catalog = arming(slow, quad, 0);
        for (goal, downed) in [(Goal::Idle, false), (Goal::Attack(ShipRef::Player), true)] {
            let mut session = with_an_npc_at(&catalog, 750, 550, 180);
            session.select_target(TargetPick::Nearest);
            session.hold_fire(true, false);
            let mut events = Vec::new();
            for _ in 0..60 {
                session.tick_traffic(&catalog, &Minding { goal }, &mut NeverFires);
                session.tick_combat(Rules::default(), &mut NeverFires);
                session.hold_fire(false, false);
                events.extend(session.take_combat_events());
            }
            let shot_down = events
                .iter()
                .any(|event| matches!(event, CombatEvent::ShotDown { .. }));
            assert_eq!(shot_down, downed, "{goal:?}: {events:?}");
            let shield = session.npcs()[0].reserves.shield.now;
            assert_eq!(shield < 30.0, !downed, "{goal:?}: {shield}");
        }
    }

    // Fights, by Nova's AI.

    /// Traders (140, class 1: disabling costs 3), police allied with them
    /// (141, retreating by `Flags` 0x0010, `MaxOdds` 200), xenophobes
    /// (142) and neutrals (143, `CrimeTol` 6).
    fn skirmish_govts() -> Vec<GovtRecord> {
        let tolerant = |id| GovtRecord {
            crime_tol: 6,
            penalties: crate::catalog::Penalties {
                disable: 3,
                kill: 7,
                ..crate::catalog::Penalties::default()
            },
            ..crate::testkit::govt(id)
        };
        vec![
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..tolerant(140)
            },
            GovtRecord {
                flags: crate::govt::WARSHIPS_RETREAT,
                allies: [1, -1, -1, -1],
                classes: [2, -1, -1, -1],
                max_odds: 200,
                ..tolerant(141)
            },
            GovtRecord {
                flags: crate::govt::XENOPHOBIC,
                ..tolerant(142)
            },
            tolerant(143),
        ]
    }

    /// A point-defence turret firing every 5 ticks at missiles within
    /// 360 pixels.
    fn point_defence() -> WeaponRecord {
        WeaponRecord {
            guidance: 9,
            reload: 5,
            count: 12,
            speed: 2000,
            mass_dmg: 1,
            energy_dmg: 4,
            ..weapon(133)
        }
    }

    /// A homing missile, 5 pixels a tick for 100 ticks, fired every 30.
    fn homing() -> WeaponRecord {
        WeaponRecord {
            guidance: 1,
            reload: 30,
            count: 100,
            speed: 500,
            guided_turn: 70,
            mass_dmg: 20,
            energy_dmg: 10,
            ..weapon(134)
        }
    }

    /// Ship `id` of strength `strength` carrying one of each of `weapons`.
    fn fitted(id: i16, strength: i16, weapons: &[i16]) -> HullRecord {
        HullRecord {
            strength,
            weapons: weapons
                .iter()
                .map(|&weapon| StockWeapon {
                    weapon: WeaponId(weapon),
                    count: 1,
                    ammo: 0,
                })
                .collect(),
            ..hull(id)
        }
    }

    /// System 130 (governed by `system_govt`) trafficked by `dudes`, each
    /// (AI type, government, ship) at an equal share; the player's tough
    /// ship 128 (300 shield, 450 armour) carries a blaster and point
    /// defence. Ship 129 is an unarmed trader, 130-132 carry a blaster,
    /// 133 and 134 a homing missile and 135 point defence. Outfits 250,
    /// 251 and 252 hold the blaster, the point defence and the missile.
    fn skirmish(dudes: &[(i16, i16, i16)], system_govt: Option<i16>) -> FakePilotCatalog {
        let mut dude_types = [(-1, 0); 8];
        for (slot, _) in dudes.iter().enumerate() {
            dude_types[slot] = (128 + slot as i16, 100 / dudes.len() as i16);
        }
        let mut catalog = FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    shield: 300,
                    armor: 450,
                    ..FAST
                }),
            )],
            weapons: vec![blaster(), point_defence(), homing()],
            outfits: vec![
                outfit(250, &[(MOD_WEAPON, 128)]),
                outfit(251, &[(MOD_WEAPON, 133)]),
                outfit(252, &[(MOD_WEAPON, 134)]),
            ],
            hulls: vec![
                fitted(128, 100, &[128, 133]),
                fitted(129, 10, &[]),
                fitted(130, 100, &[128]),
                fitted(131, 100, &[128]),
                fitted(132, 100, &[128]),
                fitted(133, 100, &[134]),
                fitted(134, 10, &[134]),
                fitted(135, 100, &[133]),
            ],
            govts: skirmish_govts(),
            traffic: vec![(
                SystemId(130),
                SystemTraffic {
                    dude_types,
                    avg_ships: dudes.len() as i16,
                    persons: Default::default(),
                },
            )],
            dudes: dudes
                .iter()
                .enumerate()
                .map(|(slot, &(ai_type, govt, ship))| {
                    (
                        DudeId(128 + slot as i16),
                        DudeRecord {
                            ai_type,
                            govt: Some(GovtId(govt)),
                            ships: vec![(ShipId(ship), 1)],
                            booty: 0,
                            info_types: 0,
                        },
                    )
                })
                .collect(),
            ship_records: (129..=135).map(|id| ship(id, FAST)).collect(),
            ..catalog()
        };
        catalog.star_map[0].govt = system_govt.map(GovtId);
        catalog
    }

    /// Brings every NPC of `session` to rest, for a scene that starts
    /// with them still: setup starts each at its top speed.
    fn stop_npcs(session: &mut Session) {
        for npc in session.traffic.npcs_mut() {
            npc.state.velocity = Vec2::ZERO;
        }
    }

    /// Setup draws placing, in order, düde `slot` of `of` at (`x`, `y`)
    /// facing `heading`, each of aggression 2.
    fn placing(of: usize, ships: &[(usize, i32, i32, u32)]) -> Draws {
        let share = 100 / of as u32;
        let draws: Vec<u32> = ships
            .iter()
            .flat_map(|&(slot, x, y, heading)| {
                [
                    6,
                    6,
                    slot as u32 * share,
                    0,
                    (x + 750) as u32,
                    (y + 750) as u32,
                    heading,
                    0,
                ]
            })
            .collect();
        Draws::of(&draws)
    }

    /// A tick of the fight, then of Nova's traffic.
    fn fight(session: &mut Session, catalog: &FakePilotCatalog) {
        session.tick_combat(Rules::default(), &mut NeverFires);
        session.tick_traffic(catalog, &crate::ai::NovaAi::default(), &mut NeverFires);
    }

    #[test]
    fn the_players_attack_on_a_trader_puts_it_to_flight_and_brings_the_police() {
        // The trader 100 ahead of the player; the police far behind.
        let catalog = skirmish(&[(1, 140, 129), (4, 141, 130)], Some(140));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(
            &catalog,
            &mut placing(2, &[(0, 0, -100, 0), (1, 0, 740, 0)]),
        );
        let (trader, police) = (NpcId(0), NpcId(1));
        let goal_of = |session: &Session, id| {
            session
                .npcs()
                .iter()
                .find(|npc| npc.id == id)
                .map(|npc| npc.goal)
        };
        session.hold_trigger(FIRE);
        let mut fled = false;
        let mut picked_while_fleeing = None;
        let mut disabled_at = None;
        for tick in 0..400 {
            fight(&mut session, &catalog);
            if goal_of(&session, trader) == Some(Goal::Flee(ShipRef::Player)) && !fled {
                fled = true;
                picked_while_fleeing = session.select_target(TargetPick::NearestThreat);
            }
            let downed = session
                .npcs()
                .iter()
                .any(|npc| npc.id == trader && npc.condition == Condition::Disabled);
            if downed && disabled_at.is_none() {
                disabled_at = Some(tick);
                session.hold_trigger(Trigger::default());
                assert_eq!(
                    [140, 141, 142, 143].map(|govt| session.pilot().legal_record(GovtId(govt))),
                    [-3, -3, 1, 1],
                    "the traders and their allies the police lower; the xenophobes and \
                     the neutrals, not allied with them, pleased by half theirs"
                );
                let npc_at = |id| {
                    session
                        .npcs()
                        .iter()
                        .find(|npc| npc.id == id)
                        .map(|npc| npc.state.position.length())
                };
                assert!(npc_at(trader) < npc_at(police), "the trader is nearer");
                assert_eq!(
                    session.select_target(TargetPick::NearestThreat),
                    Some(police),
                    "the disabled trader is no threat; the police are"
                );
            }
            if disabled_at.is_some() && session.reserves().shield.now < 300.0 {
                break;
            }
        }
        assert!(fled, "the trader fled from the player");
        assert_eq!(
            picked_while_fleeing,
            Some(trader),
            "a ship fleeing from the player is a threat to it"
        );
        assert!(disabled_at.is_some(), "the trader was disabled");
        assert_eq!(
            goal_of(&session, police),
            Some(Goal::Attack(ShipRef::Player))
        );
        assert!(
            session.reserves().shield.now < 300.0,
            "the police's shots reached the player: {:?}",
            session.reserves()
        );
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("reads");
        assert_eq!(&saved, session.pilot());
    }

    #[test]
    fn a_neutral_warship_hunts_a_player_wanted_below_its_tolerance() {
        let catalog = skirmish(&[(3, 143, 132)], Some(143));
        for (record, hunted) in [(-6, false), (-7, true)] {
            let mut session = Session::start(&catalog).expect("starts");
            with_record(&mut session, GovtId(143), record);
            session.populate(&catalog, &mut placing(1, &[(0, 300, 0, 0)]));
            fight(&mut session, &catalog);
            assert_eq!(
                session.npcs()[0].goal == Goal::Attack(ShipRef::Player),
                hunted,
                "{record}: {:?}",
                session.npcs()[0].goal
            );
        }
    }

    #[test]
    fn a_xenophobes_warship_attacks_the_player_and_the_police_whichever_is_nearer() {
        let catalog = skirmish(&[(3, 142, 131), (4, 141, 130)], Some(140));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(
            &catalog,
            &mut placing(2, &[(0, 0, -300, 0), (1, 700, 700, 0)]),
        );
        fight(&mut session, &catalog);
        assert_eq!(session.npcs()[0].goal, Goal::Attack(ShipRef::Player));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(
            &catalog,
            &mut placing(2, &[(0, 0, -300, 0), (1, 0, -350, 0)]),
        );
        fight(&mut session, &catalog);
        assert_eq!(session.npcs()[0].goal, Goal::Attack(ShipRef::Npc(NpcId(1))));
    }

    /// Every NPC of AI type 1 attacks NPC `at`, firing its missile; the
    /// rest idle, their point defence running.
    #[derive(Debug)]
    struct Volley {
        at: ShipRef,
    }

    impl Behaviour for Volley {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            if npc.ai_type == crate::traffic::npc::AiType::WimpyTrader {
                Goal::Attack(self.at)
            } else {
                Goal::Idle
            }
        }

        fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
            if npc.ai_type == crate::traffic::npc::AiType::WimpyTrader {
                Trigger {
                    only: Some(WeaponId(134)),
                    ..Trigger::default()
                }
            } else {
                Trigger {
                    point_defence: true,
                    ..Trigger::default()
                }
            }
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.goal.quarry()
        }
    }

    /// Whether point defence shot a missile down over `ticks` of `session`
    /// under `volley`.
    fn shot_down(session: &mut Session, catalog: &FakePilotCatalog, volley: &Volley) -> bool {
        let mut downed = false;
        for _ in 0..60 {
            session.tick_traffic(catalog, volley, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            downed |= session
                .take_combat_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::ShotDown { .. }));
        }
        downed
    }

    #[test]
    fn point_defence_engages_a_xenophobes_missile_but_not_an_allys() {
        // A xenophobe's missile boat (AI 1 here, so the volley fires it)
        // 300 above the player.
        let catalog = skirmish(&[(1, 142, 133)], Some(140));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut placing(1, &[(0, 0, -300, 180)]));
        let at_player = Volley {
            at: ShipRef::Player,
        };
        assert!(shot_down(&mut session, &catalog, &at_player));
        // A trader's missile at the police, allied: their point defence
        // lets it be, and it hits.
        let catalog = skirmish(&[(1, 140, 134), (4, 141, 135)], Some(140));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(
            &catalog,
            &mut placing(2, &[(0, 0, -600, 180), (1, 0, -300, 0)]),
        );
        let at_police = Volley {
            at: ShipRef::Npc(NpcId(1)),
        };
        assert!(!shot_down(&mut session, &catalog, &at_police));
        assert!(session.npcs()[1].reserves.shield.now < 30.0, "it hit");
    }

    /// `catalog`'s session with a xenophobe's missile boat (NPC 0) 600
    /// above the player facing down, and NPC 1, 300 above, made the
    /// player's escort on standing order `order`; and Nova's AI with the
    /// boat firing its missile at the escort.
    fn missile_at_an_escort(
        catalog: &FakePilotCatalog,
        order: Option<crate::escort::EscortOrder>,
    ) -> (Session, crate::ai::NovaAi) {
        let mut session = Session::start(catalog).expect("starts");
        session.populate(
            catalog,
            &mut placing(2, &[(0, 0, -600, 180), (1, 0, -300, 0)]),
        );
        stop_npcs(&mut session);
        let escort = &mut session.traffic.npcs_mut()[1];
        escort.govt = None;
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            },
            order,
        });
        let ai = crate::ai::NovaAi::default().with(
            crate::traffic::npc::AiType::WimpyTrader,
            Rc::new(Volley {
                at: ShipRef::Npc(NpcId(1)),
            }),
        );
        (session, ai)
    }

    /// Whether point defence shot a missile down over 60 ticks of
    /// `session` under `ai`.
    fn shot_down_by(session: &mut Session, catalog: &FakePilotCatalog, ai: &dyn Behaviour) -> bool {
        let mut downed = false;
        for _ in 0..60 {
            session.tick_traffic(catalog, ai, &mut NeverFires);
            session.tick_combat(Rules::default(), &mut NeverFires);
            downed |= session
                .take_combat_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::ShotDown { .. }));
        }
        downed
    }

    #[test]
    fn a_holding_escort_shoots_down_a_missile_fired_at_it() {
        // The escort flies ship 135, which carries point defence alone.
        let catalog = skirmish(&[(1, 142, 133), (4, 140, 135)], Some(140));
        let (mut session, ai) =
            missile_at_an_escort(&catalog, Some(crate::escort::EscortOrder::Hold));
        assert!(session.npcs()[1].escorts_player());
        assert!(shot_down_by(&mut session, &catalog, &ai));
        assert_eq!(session.npcs()[1].goal, Goal::Idle, "holding");
    }

    // Boarding and capture.

    use crate::board::{
        Assigned, Assignment, BoardRefusal, Boarding, MAX_ESCORTS, NovaBoarding, PlunderView, Take,
        Taken,
    };
    use crate::pilot::Escort;

    /// [`skirmish`]'s traders (140, `BoardPenalty` 5), whose wimpy trader
    /// ship 129 (crew 3, `Cost` 150,000, 20 holds, 300 fuel, strength 10)
    /// carries food and money (`Booty` 0x0041), and the police allied with
    /// them (141) flying interceptor 130; the player's ship 128 (strength
    /// 100, 20 holds, 300 fuel, 30 free mass) has a crew of 10.
    pub(super) fn boardable() -> FakePilotCatalog {
        let mut catalog = skirmish(&[(1, 140, 129), (4, 141, 130)], Some(140));
        catalog.dudes[0].1.booty = 0x0041;
        catalog.govts[0].penalties.board = 5;
        for record in &mut catalog.ship_records {
            if record.id == ShipId(129) {
                record.cost = 150_000;
            }
        }
        catalog.ship_records.push(ShipRecord {
            crew: 10,
            ..ship(128, FAST)
        });
        catalog
    }

    /// `catalog`'s session with the trader (NPC 0) disabled (armour 10 of
    /// 45) under the player at the centre, both at rest facing up, and
    /// targeted, and the police (NPC 1) 740 below.
    fn alongside(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.populate(catalog, &mut placing(2, &[(0, 0, 0, 0), (1, 0, 740, 0)]));
        let trader = &mut session.traffic.npcs_mut()[0];
        assert_eq!(trader.state.position, Vec2::ZERO);
        // Setup starts it at its top speed: brought to rest under the
        // player.
        trader.state.velocity = Vec2::ZERO;
        trader.condition = Condition::Disabled;
        trader.reserves.armor.now = 10.0;
        session.target = Some(NpcId(0));
        session
    }

    /// The boarding's draws: the threshold 15, credits 3.75 + 2 thousand,
    /// food, 10 + 3 tons, 170 energy and no jitter.
    const BOARD_DRAWS: [u32; 6] = [0, 2, 0, 3, 17, 5];

    /// What [`BOARD_DRAWS`] put on board the trader: odds of 10 / 30
    /// (33), and 10 for the player's strength over five times the
    /// trader's.
    const ON_BOARD: PlunderView = PlunderView {
        npc: NpcId(0),
        ship: ShipId(129),
        credits: 5750,
        cargo: Some((Good::Commodity(0), 13)),
        ammo: None,
        fuel: 170,
        odds: 43,
    };

    /// Boards by `law` and Nova's boarding rules, on [`BOARD_DRAWS`].
    fn board_by(session: &mut Session, law: &dyn LegalCode) -> Result<Boarding, BoardRefusal> {
        session.board(law, &NovaBoarding::default(), &mut Draws::of(&BOARD_DRAWS))
    }

    /// [`alongside`], boarded by Nova's law.
    pub(super) fn aboard(catalog: &FakePilotCatalog) -> Session {
        let mut session = alongside(catalog);
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Opened(ON_BOARD))
        );
        session
    }

    pub(super) fn take(session: &mut Session, take: Take, draws: &[u32]) -> (Taken, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let taken = session.plunder(take, &NovaBoarding::default(), &mut chance);
        (taken, chance.asked)
    }

    #[test]
    fn boarding_matches_the_targets_velocity_raises_the_crime_once_and_opens_its_plunder() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.traffic.npcs_mut()[0].state.velocity = Vec2::new(0.3, -0.2);
        let witness = Witness::default();
        assert_eq!(
            board_by(&mut session, &witness),
            Ok(Boarding::Opened(ON_BOARD))
        );
        assert_eq!(*witness.seen.borrow(), [(Crime::Board, Some(GovtId(140)))]);
        assert_eq!(session.player().velocity, Vec2::new(0.3, -0.2));
        assert!(session.npcs()[0].boarded);
        assert!(!session.npcs()[1].boarded);
        assert_eq!(session.boarding(), Some(ON_BOARD));
        assert!(!session.take_save_due(), "nothing to save yet");
    }

    #[test]
    fn boarding_changes_the_record_in_each_system_of_the_victims_government() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        board_by(&mut session, &NovaLaw::default()).expect("boards");
        assert_eq!(session.pilot().system_record(SystemId(130)), -5);
        assert_eq!(session.pilot().legal_record(GovtId(140)), -5);
    }

    /// [`boardable`] with ranks 200 (of the traders, 140), lost to any
    /// crime, and 201 (of 140), lost to disabling or destroying.
    fn boardable_ranked() -> FakePilotCatalog {
        let lost = |id, flags| RankRecord {
            affil_govt: Some(GovtId(140)),
            flags,
            ..crate::testkit::rank(id)
        };
        FakePilotCatalog {
            ranks: vec![
                lost(200, crate::rank::LOST_TO_CRIME),
                lost(201, crate::rank::LOST_TO_VIOLENCE),
            ],
            ..boardable()
        }
    }

    #[test]
    fn boarding_takes_a_0x40_rank_but_not_a_0x04_rank() {
        let catalog = boardable_ranked();
        let mut session = alongside(&catalog);
        session.pilot.ranks.extend([RankId(200), RankId(201)]);
        board_by(&mut session, &NovaLaw::default()).expect("boards");
        assert_eq!(ranks_held(&session), [201]);
    }

    #[test]
    fn boarding_a_mission_ship_takes_no_rank() {
        let catalog = boardable_ranked();
        for marked in MARKS {
            let mut session = alongside(&catalog);
            mark_the_npc(&mut session, marked);
            session.pilot.ranks.extend([RankId(200), RankId(201)]);
            board_by(&mut session, &NovaLaw::default()).expect("boards");
            assert!(session.npcs()[0].boarded, "{marked:?}");
            assert_eq!(ranks_held(&session), [200, 201], "{marked:?}: @0x9c10");
        }
    }

    #[test]
    fn a_refused_boarding_changes_nothing() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.traffic.npcs_mut()[0].condition = Condition::Intact;
        let witness = Witness::default();
        assert_eq!(
            board_by(&mut session, &witness),
            Err(BoardRefusal::CantBoard)
        );
        let mut session = alongside(&catalog);
        session.player.velocity = Vec2::new(0.0, 0.6);
        assert_eq!(board_by(&mut session, &witness), Err(BoardRefusal::TooFast));
        assert_eq!(session.player().velocity, Vec2::new(0.0, 0.6));
        assert_eq!(*witness.seen.borrow(), [], "no crime");
        assert!(!session.npcs()[0].boarded);
        assert_eq!(session.boarding(), None);
        assert_eq!(session.strikes, [], "no one is called");
        session.target = None;
        assert_eq!(
            board_by(&mut session, &witness),
            Err(BoardRefusal::NoTarget)
        );
    }

    #[test]
    fn a_ship_with_no_crew_cannot_be_boarded() {
        let mut catalog = boardable();
        for record in &mut catalog.ship_records {
            if record.id == ShipId(129) {
                record.crew = 0;
            }
        }
        let mut session = alongside(&catalog);
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Err(BoardRefusal::CantBoard)
        );
    }

    #[test]
    fn nothing_is_boarded_while_landed_or_jumping() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.jump = Some(crate::session::Jump::Hyperspace {
            to: SystemId(131),
            warp: 0,
        });
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Err(BoardRefusal::NoTarget)
        );
        session.jump = None;
        session.landed = Some(StellarId(128));
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Err(BoardRefusal::NoTarget)
        );
    }

    #[test]
    fn a_ship_boarded_cannot_be_boarded_again() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        assert_eq!(
            take(&mut session, Take::Abort, &[]),
            (Taken::Aborted, vec![])
        );
        assert_eq!(session.boarding(), None);
        let witness = Witness::default();
        assert_eq!(
            board_by(&mut session, &witness),
            Err(BoardRefusal::CantBoard)
        );
        assert_eq!(*witness.seen.borrow(), []);
    }

    #[test]
    fn boarding_brings_the_police_as_an_attack_does() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        assert_eq!(
            session.strikes,
            [Strike {
                ship: ShipRef::Npc(NpcId(0)),
                by: ShipRef::Player,
                damage: 0.0,
                downed: None,
                provokes: true,
            }]
        );
        session.tick_traffic(&catalog, &crate::ai::NovaAi::default(), &mut NeverFires);
        assert_eq!(session.npcs()[1].goal, Goal::Attack(ShipRef::Player));
        assert_eq!(session.npcs()[1].target, Some(ShipRef::Player));
    }

    #[test]
    fn boarding_a_ship_turns_every_other_ship_off_it() {
        let catalog = boardable();
        let boarded = ShipRef::Npc(NpcId(0));
        let after = |target, goal| {
            let mut session = alongside(&catalog);
            let other = &mut session.traffic.npcs_mut()[1];
            other.target = Some(target);
            other.goal = goal;
            other.provoked = 5.0;
            board_by(&mut session, &NovaLaw::default()).expect("boards");
            let other = &session.npcs()[1];
            (other.target, other.goal, other.provoked)
        };
        let dropped = (None, Goal::Idle, 0.0);
        assert_eq!(after(boarded, Goal::Attack(boarded)), dropped);
        assert_eq!(
            after(boarded, Goal::Attack(ShipRef::Player)),
            dropped,
            "targeting it"
        );
        assert_eq!(
            after(ShipRef::Player, Goal::Flee(boarded)),
            dropped,
            "fighting it"
        );
        let player = ShipRef::Player;
        assert_eq!(
            after(player, Goal::Attack(player)),
            (Some(player), Goal::Attack(player), 5.0),
            "a ship after another is left be"
        );
    }

    #[test]
    fn a_boarding_ends_when_the_ship_boarded_is_gone() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        session.traffic.remove(NpcId(0));
        assert_eq!(
            take(&mut session, Take::Credits, &[]),
            (Taken::Aborted, vec![])
        );
        assert_eq!(session.boarding(), None);
    }

    #[test]
    fn a_board_raises_boarded_once_repelled_or_not_and_a_refused_one_nothing() {
        let boarded = [StoryEvent::ShipFate {
            ship: NpcId(0),
            fate: Fate::Boarded,
        }];
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.drop_raised();
        session.target = None;
        assert!(board_by(&mut session, &NovaLaw::default()).is_err());
        assert_eq!(session.story_raised(), [], "no target");
        session.target = Some(NpcId(0));
        board_by(&mut session, &NovaLaw::default()).expect("boards");
        assert_eq!(session.story_raised(), boarded);
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Err(BoardRefusal::CantBoard),
            "boarded already"
        );
        assert_eq!(session.story_raised(), boarded);

        let mut catalog = boardable();
        catalog.dudes[0].1.booty = 0;
        let bible = NovaBoarding {
            empty_booty: RuleSource::Bible,
            ..NovaBoarding::default()
        };
        let mut session = alongside(&catalog);
        session.drop_raised();
        assert_eq!(
            session.board(&NovaLaw::default(), &bible, &mut Draws::of(&[])),
            Ok(Boarding::Repelled)
        );
        assert_eq!(session.story_raised(), boarded, "repelled");
    }

    #[test]
    fn by_the_bible_an_empty_booty_repels_the_boarders_but_the_crime_stands() {
        let mut catalog = boardable();
        catalog.dudes[0].1.booty = 0;
        let bible = NovaBoarding {
            empty_booty: RuleSource::Bible,
            ..NovaBoarding::default()
        };
        let mut session = alongside(&catalog);
        let witness = Witness::default();
        let mut chance = Draws::of(&[]);
        assert_eq!(
            session.board(&witness, &bible, &mut chance),
            Ok(Boarding::Repelled)
        );
        assert_eq!(*witness.seen.borrow(), [(Crime::Board, Some(GovtId(140)))]);
        assert!(chance.asked.is_empty(), "no plunder rolled");
        assert_eq!(session.boarding(), None);
        assert!(session.npcs()[0].boarded);
        assert_eq!(session.strikes.len(), 1, "the police still come");
        // By the engine, the dialog opens with no credits and no cargo.
        let mut session = alongside(&catalog);
        let opened = board_by(&mut session, &NovaLaw::default());
        let Ok(Boarding::Opened(view)) = opened else {
            panic!("{opened:?}");
        };
        assert_eq!((view.credits, view.cargo), (0, None));
    }

    #[test]
    fn boarding_a_trader_by_novas_law_costs_its_board_penalty_with_it_and_its_allies() {
        let catalog = boardable();
        let session = aboard(&catalog);
        assert_eq!(
            [140, 141, 142, 143].map(|govt| session.pilot().legal_record(GovtId(govt))),
            [-5, -5, 0, 0]
        );
    }

    #[test]
    fn the_capture_odds_count_the_fleet_and_the_marines() {
        let mut catalog = boardable();
        catalog.outfits.push(outfit(320, &[(25, 1)]));
        for record in &mut catalog.ship_records {
            if record.id == ShipId(130) {
                record.crew = 30;
                record.inherent_ai = 4;
            }
            if record.id == ShipId(131) {
                record.crew = 300;
            }
        }
        let mut session = alongside(&catalog);
        session.pilot.outfits.insert(OutfitId(320), 2);
        let escort = |ship| Escort {
            ship: ShipId(ship),
            reserves: Reserves::default(),
            order: None,
            carried: false,
            wage: None,
            person: None,
        };
        session.pilot.escorts = vec![escort(130), escort(130), escort(131)];
        // Crew 10, 3 from each interceptor escort (ship 130), none from
        // the trader escort (131, `InherentAI` 1) and 2 marines: 18
        // against 3 is 60, and 10 for the strength.
        let opened = board_by(&mut session, &NovaLaw::default());
        let Ok(Boarding::Opened(view)) = opened else {
            panic!("{opened:?}");
        };
        assert_eq!(view.odds, 70);
        // A derelict government, or a full fleet, gives none.
        let mut session = alongside(&catalog);
        session.pilot.escorts = vec![
            Escort {
                ship: ShipId(130),
                reserves: Reserves::default(),
                order: None,
                carried: false,
                wage: None,
                person: None,
            };
            MAX_ESCORTS
        ];
        let opened = board_by(&mut session, &NovaLaw::default());
        assert!(matches!(
            opened,
            Ok(Boarding::Opened(PlunderView { odds: 0, .. }))
        ));
        let mut catalog = boardable();
        catalog.govts[0].flags = crate::govt::DERELICT;
        let mut session = alongside(&catalog);
        let opened = board_by(&mut session, &NovaLaw::default());
        assert!(matches!(
            opened,
            Ok(Boarding::Opened(PlunderView { odds: 0, .. }))
        ));
    }

    // The plunder.

    #[test]
    fn the_mission_cargo_aboard_takes_the_plunders_room() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 10));
        assert_eq!(
            take(&mut session, Take::Cargo, &[]).0,
            Taken::Cargo {
                good: Good::Commodity(0),
                stored: 10
            },
            "20 holds, 10 tons aboard"
        );
        let mut session = aboard(&catalog);
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 25));
        assert_eq!(
            take(&mut session, Take::Cargo, &[]).0,
            Taken::Cargo {
                good: Good::Commodity(0),
                stored: 0
            },
            "more aboard than the holds"
        );
    }

    #[test]
    fn the_cargo_is_stored_up_to_the_free_space_once() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        assert_eq!(
            take(&mut session, Take::Cargo, &[]),
            (
                Taken::Cargo {
                    good: Good::Commodity(0),
                    stored: 13
                },
                vec![]
            ),
            "the first take is never rolled"
        );
        assert_eq!(session.pilot().held(Good::Commodity(0)), 13);
        assert_eq!(session.boarding().map(|view| view.cargo), Some(None));
        assert_eq!(
            take(&mut session, Take::Cargo, &[]),
            (Taken::Nothing, vec![]),
            "taken once; an ignored press rolls nothing"
        );
        let mut session = aboard(&catalog);
        session.pilot.cargo.insert(Good::Commodity(3), 10);
        assert_eq!(
            take(&mut session, Take::Cargo, &[]).0,
            Taken::Cargo {
                good: Good::Commodity(0),
                stored: 10
            }
        );
        let mut session = aboard(&catalog);
        session.pilot.cargo.insert(Good::Commodity(3), 20);
        assert_eq!(
            take(&mut session, Take::Cargo, &[]).0,
            Taken::Cargo {
                good: Good::Commodity(0),
                stored: 0
            }
        );
        assert_eq!(session.pilot().held(Good::Commodity(0)), 0);
        assert_eq!(
            session.pilot().cargo().collect::<Vec<_>>(),
            [(Good::Commodity(3), 20)],
            "no empty entry for the good not stored"
        );
        assert_eq!(session.boarding().map(|view| view.cargo), Some(None));
    }

    #[test]
    fn the_credits_are_added_to_the_cash() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        session.pilot.cash = 100;
        assert_eq!(
            take(&mut session, Take::Credits, &[]).0,
            Taken::Credits(5750)
        );
        assert_eq!(session.pilot().cash(), 5850);
        assert_eq!(session.boarding().map(|view| view.credits), Some(0));
        assert_eq!(take(&mut session, Take::Credits, &[]).0, Taken::Nothing);
        assert_eq!(session.pilot().cash(), 5850);
    }

    #[test]
    fn the_energy_fills_the_tank_up_to_its_room() {
        let catalog = boardable();
        for (now, stored, full) in [
            (300.0, 0, true),
            (200.0, 100, true),
            (0.0, 170, false),
            (130.0, 170, true),
        ] {
            let mut session = aboard(&catalog);
            session.pilot.reserves.fuel.now = now;
            assert_eq!(
                take(&mut session, Take::Energy, &[]).0,
                Taken::Energy {
                    offered: 170,
                    stored,
                    full
                },
                "{now}"
            );
            assert_eq!(session.reserves().fuel.now, now + stored as f32);
            assert_eq!(session.boarding().map(|view| view.fuel), Some(0));
        }
        // Room for 99.5 stores 99, as the original truncates the room.
        let mut session = aboard(&catalog);
        session.pilot.reserves.fuel.now = 200.5;
        assert_eq!(
            take(&mut session, Take::Energy, &[]).0,
            Taken::Energy {
                offered: 170,
                stored: 99,
                full: false
            }
        );
        assert_eq!(session.reserves().fuel.now, 299.5);
    }

    #[test]
    fn the_plunder_is_named_as_the_exchange_and_the_outfitter_name_it() {
        let mut catalog = boardable();
        catalog.commodities = food_and_metal();
        catalog.junk = vec![JunkRecord {
            id: JunkId(146),
            name: "Ice".to_owned(),
            abbrev: String::new(),
            base_price: 10,
            sold_at: Vec::new(),
            bought_at: Vec::new(),
            buy_on: Test::default(),
            sell_on: Test::default(),
            flags: 0,
            scan_mask: 0,
            lc_name: String::new(),
        }];
        catalog.outfits.push(outfit(310, &[]));
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.good_name(Good::Commodity(0)), Some("Food"));
        assert_eq!(session.good_name(Good::Commodity(4)), Some("Metal"));
        assert_eq!(session.good_name(Good::Commodity(5)), Some(""), "no name");
        assert_eq!(session.good_name(Good::Commodity(6)), None, "not standard");
        assert_eq!(session.good_name(Good::Junk(JunkId(146))), Some("Ice"));
        assert_eq!(session.good_name(Good::Junk(JunkId(147))), None);
        assert_eq!(session.outfit_name(OutfitId(310)), Some("Outfit 310"));
        assert_eq!(session.outfit_name(OutfitId(311)), None);
    }

    /// [`boardable`] with rockets (weapon 140, firing rounds of their own,
    /// held by outfit 253) on both ships, the trader holding 9 rounds, and
    /// ammunition outfit 310 for them of `mass` and `max`.
    fn ammo_aboard(mass: i16, max: i16) -> FakePilotCatalog {
        let mut catalog = boardable();
        catalog.weapons.push(secondary(140, 12));
        for hull in &mut catalog.hulls {
            if hull.id == ShipId(128) {
                hull.weapons.push(StockWeapon {
                    weapon: ROCKET,
                    count: 1,
                    ammo: 0,
                });
            }
            if hull.id == ShipId(129) {
                hull.weapons.push(StockWeapon {
                    weapon: ROCKET,
                    count: 1,
                    ammo: 9,
                });
            }
        }
        catalog.outfits.push(OutfitRecord {
            mass,
            max,
            ..outfit(310, &[(MOD_AMMO, 140)])
        });
        catalog.outfits.push(outfit(253, &[(MOD_WEAPON, 140)]));
        catalog
    }

    /// The ammunition boarding's draws: [`BOARD_DRAWS`] with the rockets
    /// drawn before the energy.
    const AMMO_DRAWS: [u32; 7] = [0, 2, 0, 3, 0, 17, 5];

    fn ammo_session(catalog: &FakePilotCatalog) -> Session {
        let mut session = alongside(catalog);
        let opened = session.board(
            &NovaLaw::default(),
            &NovaBoarding::default(),
            &mut Draws::of(&AMMO_DRAWS),
        );
        assert_eq!(
            opened,
            Ok(Boarding::Opened(PlunderView {
                ammo: Some((OutfitId(310), 9)),
                ..ON_BOARD
            }))
        );
        session
    }

    #[test]
    fn the_ammo_is_taken_an_outfit_at_a_time_up_to_the_rounds_the_mass_and_the_max() {
        let session = ammo_session(&ammo_aboard(1, 20));
        let mut by_rounds = session.clone();
        by_rounds.select_secondary(false);
        assert_eq!(by_rounds.secondary_line_count(), Some(0));
        assert_eq!(
            take(&mut by_rounds, Take::Ammo, &[]).0,
            Taken::Ammo {
                outfit: OutfitId(310),
                count: 9
            }
        );
        assert_eq!(by_rounds.pilot().owned(OutfitId(310)), 9);
        assert_eq!(by_rounds.secondary_line_count(), Some(9), "refitted");
        assert_eq!(by_rounds.boarding().map(|view| view.ammo), Some(None));
        // 30 tons free: seven of 4 tons, not eight.
        let mut by_mass = ammo_session(&ammo_aboard(4, 20));
        assert_eq!(
            take(&mut by_mass, Take::Ammo, &[]).0,
            Taken::Ammo {
                outfit: OutfitId(310),
                count: 7
            }
        );
        // Up to a `Max` of 12 with 5 owned.
        let mut by_max = ammo_session(&ammo_aboard(1, 12));
        by_max.pilot.outfits.insert(OutfitId(310), 5);
        assert_eq!(
            take(&mut by_max, Take::Ammo, &[]).0,
            Taken::Ammo {
                outfit: OutfitId(310),
                count: 7
            }
        );
        assert_eq!(by_max.pilot().owned(OutfitId(310)), 12);
        let mut full = ammo_session(&ammo_aboard(1, 12));
        full.pilot.outfits.insert(OutfitId(310), 12);
        assert_eq!(
            take(&mut full, Take::Ammo, &[]).0,
            Taken::Ammo {
                outfit: OutfitId(310),
                count: 0
            }
        );
        assert_eq!(full.boarding().map(|view| view.ammo), Some(None));
    }

    #[test]
    fn an_ammunition_take_resets_as_it_is_pressed_and_after_each_round() {
        let mut session = ammo_session(&ammo_aboard(1, 12));
        session.pilot.outfits.insert(OutfitId(310), 9);
        trimmed_by_each_reset(&mut session);
        crate::session::precalced::tests::convicted(&mut session);
        assert_eq!(
            take(&mut session, Take::Ammo, &[]).0,
            Taken::Ammo {
                outfit: OutfitId(310),
                count: 3
            }
        );
        assert_eq!(
            commodity_left(&session),
            1,
            "four trims: @0x93cc9, then @0x93dcb for each of the three rounds"
        );
        assert!(session.boarding().is_some(), "the dialog still open");
        assert_eq!(session.scan_latches, crate::scan::Latches::ARMED);
    }

    #[test]
    fn taking_ammo_admits_each_round_by_its_raw_mass() {
        // Rounds of `Mass` 4 with Flags 0x0400 on a class of `Mass` 200:
        // 8 tons scaled, 4 raw. `_DoPlunderDialog` tests the raw `Mass`
        // (@0x93d6f-0x93d76) against the free mass, which charges each at
        // 8: four from 30 tons free (at 30, 22, 14 and 6), not three.
        // Either reading of the scaled mass takes the same.
        let mut catalog = ammo_aboard(4, 20);
        for record in &mut catalog.outfits {
            if record.id == OutfitId(310) {
                record.flags = OutfitFlags::MASS_BY_MASS;
            }
        }
        catalog.ships = vec![(ShipId(128), Ok(ShipFields { mass: 200, ..FAST }))];
        for record in &mut catalog.ship_records {
            if record.id == ShipId(128) {
                record.fields.mass = 200;
            }
        }
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let mut session = ammo_session(&catalog)
                .with_rules(Rulebook::default().with_override(RuleKey::ScaledMass, source));
            assert_eq!(session.free_mass(), 30, "{source:?}");
            assert_eq!(
                take(&mut session, Take::Ammo, &[]).0,
                Taken::Ammo {
                    outfit: OutfitId(310),
                    count: 4
                },
                "{source:?}"
            );
            assert_eq!(session.free_mass(), -2, "charged 8 a round: {source:?}");
        }
        // Exactly on the raw `Mass`: rounds of 6 (12 scaled) at 30 and 18
        // free, and the third at 6 free, as the raw test admits `>=`.
        for record in &mut catalog.outfits {
            if record.id == OutfitId(310) {
                record.mass = 6;
            }
        }
        let mut session = ammo_session(&catalog);
        assert_eq!(
            take(&mut session, Take::Ammo, &[]).0,
            Taken::Ammo {
                outfit: OutfitId(310),
                count: 3
            }
        );
        assert_eq!(session.free_mass(), -6);
    }

    #[test]
    fn taking_ammo_stops_at_the_max_a_mod_type_27_raises() {
        let mut catalog = ammo_aboard(1, 4);
        catalog
            .outfits
            .push(outfit(320, &[(crate::outfitter::RAISES_MAX, 310)]));
        for (raisers, count) in [(0, 4), (1, 4), (2, 8)] {
            let mut session = ammo_session(&catalog);
            if raisers > 0 {
                session.pilot.outfits.insert(OutfitId(320), raisers);
            }
            assert_eq!(
                take(&mut session, Take::Ammo, &[]).0,
                Taken::Ammo {
                    outfit: OutfitId(310),
                    count
                },
                "{raisers}"
            );
        }
    }

    #[test]
    fn the_ammo_take_keeps_a_surplus_shield_and_armour_by_the_engine_and_holds_them_by_the_other_reading()
     {
        for (source, surplus) in [(RuleSource::Engine, 5.0), (RuleSource::Bible, 0.0)] {
            let mut session = ammo_session(&ammo_aboard(1, 20))
                .with_rules(Rulebook::default().with_override(RuleKey::RefitReserves, source));
            let (shield, armor) = (session.stats().shield, session.stats().armor);
            session.pilot.reserves.shield.now = shield + 5.0;
            session.pilot.reserves.armor.now = armor + 5.0;
            take(&mut session, Take::Ammo, &[]);
            let reserves = session.reserves();
            assert_eq!(
                reserves.shield,
                Gauge {
                    now: shield + surplus,
                    max: shield
                },
                "{source:?}"
            );
            assert_eq!(
                reserves.armor,
                Gauge {
                    now: armor + surplus,
                    max: armor
                },
                "{source:?}"
            );
        }
    }

    #[test]
    fn each_take_grows_the_self_destruct_threshold_truncated() {
        let catalog = ammo_aboard(1, 20);
        let mut session = ammo_session(&catalog);
        let threshold = |session: &Session| session.aboard.map(|aboard| aboard.plunder.threshold);
        assert_eq!(threshold(&session), Some(15));
        for (press, after) in [
            (Take::Cargo, 30),
            (Take::Credits, 37),
            (Take::Energy, 55),
            (Take::Ammo, 110),
        ] {
            take(&mut session, press, &[99]);
            assert_eq!(threshold(&session), Some(after), "{press:?}");
        }
    }

    #[test]
    fn the_press_after_a_take_rolls_the_self_destruct_before_it_is_handled() {
        let catalog = boardable();
        for (draw, taken) in [(31, Taken::Credits(5750)), (30, Taken::Tripped)] {
            let mut session = aboard(&catalog);
            take(&mut session, Take::Cargo, &[]);
            let cash = session.pilot().cash();
            let (outcome, asked) = take(&mut session, Take::Credits, &[draw]);
            assert_eq!(
                (outcome, asked),
                (taken, vec![100]),
                "threshold 30, drew {draw}"
            );
            if taken == Taken::Tripped {
                assert_eq!(session.pilot().cash(), cash, "not handled");
                assert_eq!(session.boarding(), None);
            }
        }
    }

    #[test]
    fn abort_never_rolls() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        take(&mut session, Take::Cargo, &[]);
        assert_eq!(
            take(&mut session, Take::Abort, &[0]),
            (Taken::Aborted, vec![])
        );
        assert_eq!(session.boarding(), None);
        assert_eq!(
            take(&mut session, Take::Credits, &[]),
            (Taken::Nothing, vec![]),
            "no boarding under way"
        );
    }

    #[test]
    fn a_tripped_self_destruct_breaks_the_ship_up_through_the_fight_and_is_no_crime() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        let witness = Witness::default();
        board_by(&mut session, &witness).expect("boards");
        take(&mut session, Take::Cargo, &[]);
        assert_eq!(take(&mut session, Take::Credits, &[0]).0, Taken::Tripped);
        let trader = &session.npcs()[0];
        assert_eq!(
            (trader.reserves.shield.now, trader.reserves.armor.now),
            (0.0, 0.0)
        );
        let rules = Rules {
            law: &witness,
            ..Rules::default()
        };
        session.tick_combat(rules, &mut NeverFires);
        assert!(
            session
                .take_combat_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::BreakingUp { ship, .. } if *ship == ShipRef::Npc(NpcId(0)))),
            "it breaks up"
        );
        assert_eq!(
            *witness.seen.borrow(),
            [(Crime::Board, Some(GovtId(140)))],
            "the boarding alone"
        );
    }

    // Capture.

    #[test]
    fn a_failed_capture_ends_the_boarding_and_leaves_the_ship_boarded() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        assert_eq!(
            take(&mut session, Take::Capture, &[44]),
            (Taken::CaptureFailed, vec![100])
        );
        assert_eq!(session.boarding(), None);
        assert_eq!(session.npcs().len(), 2, "still there");
        assert!(session.npcs()[0].boarded);
        assert_eq!(session.pilot().escorts(), []);
    }

    #[test]
    fn a_capture_at_the_odds_trips_on_a_tenth() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 0]),
            (Taken::Tripped, vec![100, 10])
        );
        let trader = &session.npcs()[0];
        assert_eq!(trader.reserves.armor.now, 0.0);
        assert_eq!(session.boarding(), None);
    }

    #[test]
    fn a_capture_with_the_fleet_full_changes_nothing_and_the_boarding_goes_on() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        session.pilot.escorts = vec![
            Escort {
                ship: ShipId(130),
                reserves: Reserves::default(),
                order: None,
                carried: false,
                wage: None,
                person: None,
            };
            MAX_ESCORTS
        ];
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 1]).0,
            Taken::FleetFull
        );
        assert_eq!(session.boarding(), Some(ON_BOARD));
        assert_eq!(session.pilot().escorts().len(), MAX_ESCORTS);
    }

    #[test]
    fn carried_fighters_do_not_fill_the_fleet_for_a_capture() {
        let catalog = boardable();
        let escort = |carried| Escort {
            ship: ShipId(130),
            reserves: Reserves::default(),
            order: None,
            carried,
            wage: None,
            person: None,
        };
        let fleet = |escorts: usize, fighters: usize| {
            let mut fleet = vec![escort(false); escorts];
            fleet.extend(vec![escort(true); fighters]);
            fleet
        };
        let mut session = aboard(&catalog);
        session.pilot.escorts = fleet(MAX_ESCORTS, 2);
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 1]).0,
            Taken::FleetFull,
            "6 escorts and 2 fighters out"
        );
        let mut session = aboard(&catalog);
        session.pilot.escorts = fleet(MAX_ESCORTS - 1, 3);
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 1]).0,
            Taken::Captured,
            "5 escorts and 3 fighters out"
        );
        let mut session = alongside(&catalog);
        session.pilot.escorts = fleet(MAX_ESCORTS - 1, 3);
        let opened = board_by(&mut session, &NovaLaw::default());
        assert!(
            !matches!(opened, Ok(Boarding::Opened(PlunderView { odds: 0, .. }))),
            "the fleet has room: {opened:?}"
        );
    }

    #[test]
    fn a_capture_awaits_its_assignment_and_ignores_any_other_press() {
        let catalog = boardable();
        let mut session = aboard(&catalog);
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 1]).0,
            Taken::Captured
        );
        assert_eq!(
            take(&mut session, Take::Credits, &[]),
            (Taken::Nothing, vec![])
        );
        assert_eq!(
            take(&mut session, Take::Abort, &[]),
            (Taken::Nothing, vec![])
        );
        assert_eq!(session.pilot().escorts(), []);
        assert!(!session.take_save_due(), "nothing changed yet");
    }

    /// The trader's reserves once the boarding set its armour at 10.
    fn trader_reserves() -> Reserves {
        Reserves {
            armor: Gauge {
                now: 10.0,
                max: 45.0,
            },
            ..Reserves::full(30.0, 45.0, 300.0)
        }
    }

    /// The trader as an escort: armour at half its most.
    fn trader_escort() -> Escort {
        Escort {
            ship: ShipId(129),
            reserves: Reserves {
                armor: Gauge {
                    now: 22.5,
                    max: 45.0,
                },
                ..trader_reserves()
            },
            order: None,
            carried: false,
            wage: None,
            person: None,
        }
    }

    #[test]
    fn a_crewless_player_captures_straight_into_its_fleet() {
        let mut catalog = boardable();
        if let Some(record) = catalog
            .ship_records
            .iter_mut()
            .find(|r| r.id == ShipId(128))
        {
            record.crew = 0;
        }
        let mut session = alongside(&catalog);
        let opened = board_by(&mut session, &NovaLaw::default());
        assert_eq!(
            opened,
            Ok(Boarding::Opened(PlunderView {
                odds: 10,
                ..ON_BOARD
            })),
            "none for the crew, 10 for the strength"
        );
        session.traffic.npcs_mut()[1].leader = Some(NpcId(0));
        assert_eq!(
            take(&mut session, Take::Capture, &[10, 1]).0,
            Taken::Escorted
        );
        assert_eq!(session.pilot().escorts(), [trader_escort()]);
        assert_joined_in_place(&session);
        assert_eq!(session.npcs()[1].leader, None, "its own escort let go");
        assert_eq!(session.target(), None);
        assert_eq!(session.boarding(), None);
        assert!(session.take_save_due());
    }

    /// The trader, NPC 0, has joined the fleet where it is: intact, its
    /// armour at half its most, of no government, its AI type its ship
    /// type's, keeping formation in slot 2 with no standing order, and
    /// let go by the police.
    fn assert_joined_in_place(session: &Session) {
        assert_eq!(
            session.npcs().iter().map(|npc| npc.id).collect::<Vec<_>>(),
            [NpcId(0), NpcId(1)],
            "it stays in the system"
        );
        let escort = &session.npcs()[0];
        assert_eq!(escort.condition, Condition::Intact);
        assert_eq!(escort.reserves, trader_escort().reserves);
        assert_eq!(escort.govt, None);
        assert_eq!(escort.ai_type, crate::traffic::npc::AiType::WimpyTrader);
        assert_eq!(escort.goal, Goal::Formation { guard: None });
        assert_eq!(escort.target, None);
        assert_eq!(
            escort.escort,
            Some(EscortDuty {
                formation: FormationSlot {
                    slot: 2,
                    ships: 2,
                    spacing: crate::escort::spacing(None, &[None]),
                },
                order: None,
            })
        );
        assert_eq!(escort.fleet(), ShipRef::Player);
        assert!(session.is_escort(NpcId(0)));
        assert!(!session.is_escort(NpcId(1)));
        assert_ne!(
            session.npcs()[1].target,
            Some(ShipRef::Npc(NpcId(0))),
            "the police let it go"
        );
    }

    /// [`aboard`], captured and awaiting its assignment.
    pub(super) fn captured(catalog: &FakePilotCatalog) -> Session {
        let mut session = aboard(catalog);
        assert_eq!(
            take(&mut session, Take::Capture, &[43, 1]).0,
            Taken::Captured
        );
        session
    }

    #[test]
    fn use_as_escort_adds_the_ship_to_the_fleet_at_half_its_armour() {
        let catalog = boardable();
        let mut session = captured(&catalog);
        let records = session.pilot().legal_records().collect::<Vec<_>>();
        session.take_save_due();
        session.traffic.npcs_mut()[1].leader = Some(NpcId(7));
        assert_eq!(
            session.assign(Assignment::Escort, &mut Draws::of(&[])),
            Some(Assigned::Escort)
        );
        assert_eq!(session.npcs()[1].leader, Some(NpcId(7)), "another's escort");
        session.traffic.npcs_mut()[1].leader = None;
        assert_eq!(session.pilot().escorts(), [trader_escort()]);
        assert_joined_in_place(&session);
        assert_eq!(session.target(), None);
        assert_eq!(session.boarding(), None);
        assert_eq!(
            session.pilot().legal_records().collect::<Vec<_>>(),
            records,
            "no crime"
        );
        assert!(session.take_save_due());
        assert_eq!(
            session.assign(Assignment::Escort, &mut Draws::of(&[])),
            None,
            "nothing awaits"
        );
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("reads");
        assert_eq!(&saved, session.pilot());
    }

    /// A former escort's memory: the trader, hired at 30 a day.
    fn former_trader() -> crate::self_repair::FormerEscort {
        crate::self_repair::FormerEscort {
            record: Escort {
                wage: Some(30),
                ..trader_escort()
            },
        }
    }

    #[test]
    fn a_former_escort_boarded_forgets_what_it_was() {
        let catalog = boardable();
        let mut session = alongside(&catalog);
        session.traffic.npcs_mut()[0].former = Some(former_trader());
        session.traffic.npcs_mut()[1].former = Some(former_trader());
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Opened(ON_BOARD))
        );
        assert_eq!(session.npcs()[0].former, None);
        assert_eq!(
            session.npcs()[1].former,
            Some(former_trader()),
            "not boarded"
        );
    }

    #[test]
    fn a_former_escort_taken_into_the_fleet_forgets_what_it_was() {
        let catalog = boardable();
        let mut session = captured(&catalog);
        session.traffic.npcs_mut()[0].former = Some(former_trader());
        assert_eq!(
            session.assign(Assignment::Escort, &mut Draws::of(&[])),
            Some(Assigned::Escort)
        );
        assert_eq!(session.npcs()[0].former, None);
    }

    /// [`boardable`] with outfits: 400 persistent, 401 not, and 402 the
    /// trader's default item.
    pub(super) fn kitted() -> FakePilotCatalog {
        let mut catalog = boardable();
        catalog.outfits.extend([
            OutfitRecord {
                flags: OutfitFlags::PERSISTENT,
                ..outfit(400, &[])
            },
            outfit(401, &[]),
            outfit(402, &[]),
        ]);
        for record in &mut catalog.ship_records {
            if record.id == ShipId(129) {
                record.defaults = vec![(OutfitId(402), 1)];
            }
        }
        catalog
    }

    /// The player's old ship, the second NPC, flies as its escort on its
    /// slot beside the player, stock and full, its record and NPC lined up.
    fn assert_old_ship_escorts(session: &mut Session) {
        let old = &session.npcs()[1];
        assert!(session.is_escort(old.id));
        let duty = old.escort.expect("an escort");
        assert_eq!(
            (duty.formation.slot, duty.formation.ships, duty.order),
            (2, 2, None)
        );
        assert_eq!(
            old.state,
            ShipState {
                position: crate::escort::slot_position(
                    session.player(),
                    2,
                    2,
                    duty.formation.spacing
                ),
                ..*session.player()
            },
            "on its slot"
        );
        assert_eq!(old.reserves, Reserves::full(300.0, 450.0, 300.0));
        assert_eq!(old.govt, None);
        let old = old.id;
        assert!(
            session
                .command_escorts(
                    crate::escort::EscortGroup::All,
                    crate::escort::EscortCommand::Hold
                )
                .is_some()
        );
        let held = session.npcs().iter().find(|npc| npc.id == old);
        assert_eq!(
            held.and_then(|npc| npc.escort).and_then(|duty| duty.order),
            Some(crate::escort::EscortOrder::Hold),
            "its record and its NPC lined up"
        );
    }

    /// 50 tons of a commodity, 20 of `jünk` and 3 of mission cargo in 40
    /// holds: the `jünk` and the mission cargo keep the fleet past its
    /// holds through each of the three trims, so each one cuts and the
    /// tons kept by the other reading pin how many run.
    #[test]
    fn use_as_my_ship_leaves_the_commodities_with_the_old_ship_unless_the_other_reading_keeps_them()
    {
        use crate::scan::Latches;
        use crate::session::precalced::tests::{JUNK, convicted};
        for (source, kept) in [(RuleSource::Engine, None), (RuleSource::Bible, Some(19))] {
            let catalog = kitted();
            let mut session = alongside(&catalog)
                .with_rules(Rulebook::default().with_override(RuleKey::CaptureCargo, source));
            board_by(&mut session, &NovaLaw::default()).expect("boards");
            assert_eq!(
                take(&mut session, Take::Capture, &[0, 1]).0,
                Taken::Captured
            );
            session.pilot.cargo = BTreeMap::from([(Good::Commodity(2), 50), (JUNK, 20)]);
            session
                .pilot
                .missions
                .put(0, crate::testkit::carrying(140, 3));
            convicted(&mut session);
            assert_eq!(
                session.assign(Assignment::MyShip, &mut Draws::of(&[0])),
                Some(Assigned::MyShip)
            );
            assert_eq!(session.capacity(), 40, "the new 20 and the old ship's 20");
            let mut want = BTreeMap::from([(JUNK, 20)]);
            if let Some(tons) = kept {
                want.insert(Good::Commodity(2), tons);
            }
            assert_eq!(
                session.pilot().cargo().collect::<BTreeMap<_, _>>(),
                want,
                "{source:?}: the jünk kept; by the other reading 50 x 40/73, 27, \
                 then 27 x 40/50, 21, then 21 x 40/44, 19, as the three resets \
                 trim with the old ship in the fleet (@0x419fd, @0x41a3e, \
                 @0x94a0b)"
            );
            assert_eq!(
                session.pilot().missions().get(0).map(|m| m.cargo_tons),
                Some(3)
            );
            assert_eq!(session.scan_latches, Latches::ARMED, "{source:?}");
        }
    }

    /// Use As My Ship's `OnRetire` (`b9`, of the flown class 128) resets
    /// with the old ship already in the fleet (`_DoShipCapture` @0x41662,
    /// the escort set up @0x41188-0x4156f), by the other reading of
    /// [`RuleKey::CaptureCargo`], with 50 tons of a commodity, 20 of
    /// `jünk` and 3 of mission cargo, in either hook order. A trader, its
    /// 20 tons count, so `OnRetire`'s trim has 40: 50 x 40/73, 27, then
    /// 27 x 40/50, 21, 21 x 40/44, 19, and 19 x 40/42, 18 (@0x419fd,
    /// @0x41a3e, @0x94a0b). Without the old ship, 50 x 20/73 would leave
    /// 13, which the 40 tons after never cut. A class of `InherentAI` 4
    /// is no trader, and its holds never count: 13, then 13 x 20/36, 7,
    /// 7 x 20/30, 4, and 4 x 20/27, 2.
    #[test]
    fn use_as_my_ships_on_retire_reset_counts_the_old_ships_holds() {
        use crate::session::precalced::tests::JUNK;
        for (inherent_ai, holds, kept) in [(1, 40, 18), (4, 20, 2)] {
            for order in [RuleSource::Engine, RuleSource::Bible] {
                let mut catalog = kitted();
                for record in &mut catalog.ship_records {
                    if record.id == ShipId(128) {
                        record.on_retire = crate::control::Script::parse("b9");
                        record.inherent_ai = inherent_ai;
                    }
                }
                let rules = Rulebook::default()
                    .with_override(RuleKey::CaptureCargo, RuleSource::Bible)
                    .with_override(RuleKey::CaptureHookOrder, order);
                let mut session = alongside(&catalog).with_rules(rules);
                board_by(&mut session, &NovaLaw::default()).expect("boards");
                assert_eq!(
                    take(&mut session, Take::Capture, &[0, 1]).0,
                    Taken::Captured
                );
                session.pilot.cargo = BTreeMap::from([(Good::Commodity(2), 50), (JUNK, 20)]);
                session
                    .pilot
                    .missions
                    .put(0, crate::testkit::carrying(140, 3));
                assert_eq!(
                    session.assign(Assignment::MyShip, &mut Draws::of(&[0])),
                    Some(Assigned::MyShip)
                );
                assert!(
                    session.control_bit(crate::control::Bit::new(9).expect("a bit")),
                    "OnRetire ran"
                );
                assert_eq!(session.capacity(), holds, "AI {inherent_ai}");
                assert_eq!(
                    session.pilot().cargo().collect::<BTreeMap<_, _>>(),
                    BTreeMap::from([(Good::Commodity(2), kept), (JUNK, 20)]),
                    "AI {inherent_ai}, {order:?}"
                );
            }
        }
    }

    /// [`captured`] with `commodity` tons of commodity 2, `junk` of
    /// `jünk` and 3 of mission cargo aboard, assigned Use As Escort with
    /// the trader's `OnCapture` blank. Gives the cargo after.
    fn escorted_with(commodity: u32, junk: u32) -> BTreeMap<Good, u32> {
        use crate::session::precalced::tests::JUNK;
        let catalog = boardable();
        let mut session = captured(&catalog);
        session.pilot.cargo = BTreeMap::from([(Good::Commodity(2), commodity), (JUNK, junk)]);
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 3));
        assert_eq!(session.capacity(), 20, "the player's 20 alone");
        assert_eq!(
            session.assign(Assignment::Escort, &mut Draws::of(&[])),
            Some(Assigned::Escort)
        );
        assert_eq!(
            session.capacity(),
            40,
            "the player's 20 and the trader's 20"
        );
        session.pilot().cargo().collect()
    }

    /// `_DoPlunderDialog` runs a blank `OnCapture` (@0x9460e) and takes
    /// the trader into the fleet (@0x947a5) before the close resets
    /// (@0x94a0b), so the trim measures the trader's holds too: 38 tons
    /// in 40 holds is not cut, where 20 holds would cut 25 to 13.
    #[test]
    fn use_as_escort_trims_with_the_captured_ship_in_the_fleet() {
        use crate::session::precalced::tests::JUNK;
        assert_eq!(
            escorted_with(25, 10),
            BTreeMap::from([(Good::Commodity(2), 25), (JUNK, 10)]),
            "25 + 10 + 3 within the fleet's 40: untouched"
        );
    }

    /// Past even the fleet's 40 holds, Use As Escort's one reset cuts to
    /// them: 50 x 40/73, 27, where the player's 20 would leave 13.
    #[test]
    fn use_as_escort_past_the_new_fleets_holds_trims_to_them() {
        use crate::session::precalced::tests::JUNK;
        assert_eq!(
            escorted_with(50, 20),
            BTreeMap::from([(Good::Commodity(2), 27), (JUNK, 20)]),
            "50 x 40/73 once, the jünk kept"
        );
    }

    /// Use As My Ship with no ship slot free loses the prize, and the
    /// plunder dialog's close (@0x94a0b) still resets: the commodities
    /// past the player's 20 holds are cut, 50 x 20/73, 13, and a
    /// conviction's latches re-armed.
    #[test]
    fn an_abandoned_capture_still_trims_as_the_plunder_dialog_closes() {
        use crate::scan::Latches;
        use crate::session::precalced::tests::{JUNK, convicted};
        let mut full = crowded(62);
        full.pilot.cargo = BTreeMap::from([(Good::Commodity(2), 50), (JUNK, 20)]);
        full.pilot.missions.put(0, crate::testkit::carrying(140, 3));
        convicted(&mut full);
        assert_eq!(
            full.assign(Assignment::MyShip, &mut Draws::of(&[])),
            Some(Assigned::Abandoned)
        );
        assert_eq!(full.capacity(), 20);
        assert_eq!(
            full.pilot().cargo().collect::<BTreeMap<_, _>>(),
            BTreeMap::from([(Good::Commodity(2), 13), (JUNK, 20)]),
            "50 x 20/73, the jünk kept"
        );
        assert_eq!(full.scan_latches, Latches::ARMED);
    }

    #[test]
    fn use_as_my_ship_swaps_the_player_into_the_captured_ship_and_keeps_the_old_one() {
        let catalog = kitted();
        let mut session = alongside(&catalog);
        session.pilot.outfits = BTreeMap::from([(OutfitId(400), 1), (OutfitId(401), 2)]);
        session.pilot.cargo.insert(Good::Commodity(2), 4);
        session.pilot.cash = 777;
        session.pilot.ship_name = Some("Kestrel".to_owned());
        let trader = &mut session.traffic.npcs_mut()[0];
        trader.state.position = Vec2::new(5.0, -3.0);
        trader.state.velocity = Vec2::new(0.25, 0.0);
        trader.state.heading = 180.0;
        session.traffic.npcs_mut()[1].leader = Some(NpcId(0));
        board_by(&mut session, &NovaLaw::default()).expect("boards");
        // Cargo first: the hold holds 20, 4 of them used.
        take(&mut session, Take::Cargo, &[]);
        assert_eq!(
            take(&mut session, Take::Capture, &[99, 0, 1]).0,
            Taken::Captured
        );
        let held = session.pilot().cargo().collect::<Vec<_>>();
        let mut chance = Draws::of(&[123]);
        assert_eq!(
            session.assign(Assignment::MyShip, &mut chance),
            Some(Assigned::MyShip)
        );
        assert_eq!(chance.asked, [300], "the fuel drawn below its Fuel");
        assert_eq!(session.ship(), ShipId(129));
        assert_eq!(session.pilot().ship_name(), Some("Kestrel"), "unrenamed");
        assert_eq!(
            *session.player(),
            ShipState {
                position: Vec2::new(5.0, -3.0),
                velocity: Vec2::new(0.25, 0.0),
                heading: 180.0,
            }
        );
        let reserves = session.reserves();
        assert_eq!(
            reserves.shield,
            Gauge {
                now: 0.0,
                max: 30.0
            }
        );
        assert!((reserves.armor.now - 16.9985).abs() < 1e-4, "{reserves:?}");
        assert_eq!(reserves.armor.max, 45.0);
        assert_eq!(
            reserves.fuel,
            Gauge {
                now: 123.0,
                max: 300.0
            }
        );
        assert_eq!(
            session.pilot().outfits().collect::<Vec<_>>(),
            [(OutfitId(400), 1), (OutfitId(402), 1)],
            "the persistent kept, the rest gone, the new ship's default items fitted"
        );
        assert!(!held.is_empty());
        assert_eq!(
            session.pilot().cargo().collect::<Vec<_>>(),
            [],
            "the commodities stay with the old ship (@0x41762-0x41779)"
        );
        assert_eq!(session.pilot().cash(), 777);
        assert_eq!(session.stats(), ShipStats::new(FAST, &[]));
        assert_eq!(session.hull(), session.arsenal.hull(ShipId(129)));
        assert_eq!(
            session.pilot().escorts(),
            [Escort {
                ship: ShipId(128),
                reserves: Reserves::full(300.0, 450.0, 300.0),
                order: None,
                carried: false,
                wage: None,
                person: None,
            }],
            "the old ship, stock and full"
        );
        assert_eq!(
            session
                .npcs()
                .iter()
                .map(|npc| (npc.id, npc.ship))
                .collect::<Vec<_>>(),
            [(NpcId(1), ShipId(130)), (NpcId(2), ShipId(128))],
            "the captured ship left, and the old one flies beside the player"
        );
        assert_eq!(session.npcs()[0].leader, None, "its escort let go");
        assert_old_ship_escorts(&mut session);
        assert_eq!(session.player_condition(), Condition::Intact);
        assert_eq!(session.boarding(), None);
        assert!(session.take_save_due());
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("reads");
        assert_eq!(&saved, session.pilot());
    }

    #[test]
    fn a_captured_ship_keeps_its_stock_weapons() {
        // The trader stocks two blasters (held by 250) and a homing
        // missile (252) with 6 rounds of its own (403).
        let mut catalog = kitted();
        for hull in &mut catalog.hulls {
            if hull.id == ShipId(129) {
                hull.weapons = vec![
                    StockWeapon {
                        weapon: WeaponId(128),
                        count: 2,
                        ammo: 0,
                    },
                    StockWeapon {
                        weapon: WeaponId(134),
                        count: 1,
                        ammo: 6,
                    },
                ];
            }
        }
        catalog.outfits.push(outfit(403, &[(MOD_AMMO, 134)]));
        let mut session = captured(&catalog);
        assert_eq!(mounted(&session), [(128, 1), (133, 1)], "its own");
        assert_eq!(
            session.assign(Assignment::MyShip, &mut Draws::of(&[0])),
            Some(Assigned::MyShip)
        );
        assert_eq!(mounted(&session), [(128, 2), (134, 1)]);
        assert_eq!(
            session.pilot().outfits().collect::<Vec<_>>(),
            [
                (OutfitId(250), 2),
                (OutfitId(252), 1),
                (OutfitId(402), 1),
                (OutfitId(403), 6)
            ],
            "its default item, then its stock weapons and rounds"
        );
        assert_eq!(
            session.stock,
            BTreeMap::from([(OutfitId(250), 2), (OutfitId(252), 1), (OutfitId(403), 6)])
        );
    }

    #[test]
    fn a_ship_taken_over_with_no_fuel_tank_draws_no_fuel() {
        let mut catalog = boardable();
        for record in &mut catalog.ship_records {
            if record.id == ShipId(129) {
                record.fields.fuel = 0;
            }
        }
        let mut session = alongside(&catalog);
        // No energy on board, so no draw for it.
        let opened = session.board(
            &NovaLaw::default(),
            &NovaBoarding::default(),
            &mut Draws::of(&[0, 2, 0, 3, 5]),
        );
        assert!(matches!(
            opened,
            Ok(Boarding::Opened(PlunderView { fuel: 0, .. }))
        ));
        take(&mut session, Take::Capture, &[43, 1]);
        let mut chance = Draws::of(&[]);
        assert_eq!(
            session.assign(Assignment::MyShip, &mut chance),
            Some(Assigned::MyShip)
        );
        assert!(chance.asked.is_empty(), "{:?}", chance.asked);
        assert_eq!(session.reserves().fuel.now, 0.0);
    }

    #[test]
    fn ammo_the_player_cannot_fire_or_a_bays_fighters_are_not_plundered() {
        let mut unarmed = ammo_aboard(1, 20);
        for hull in &mut unarmed.hulls {
            if hull.id == ShipId(128) {
                hull.weapons.retain(|stock| stock.weapon != ROCKET);
            }
        }
        let mut session = alongside(&unarmed);
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Opened(ON_BOARD)),
            "no rockets to fire them: no ammo, and no draw for it"
        );
        // Both ships carry a bay of ship 144, and the player's fighter
        // outfit is its rounds.
        let mut bay = ammo_aboard(1, 20);
        bay.weapons.push(WeaponRecord {
            guidance: crate::board::FIGHTER_BAY,
            ..secondary(150, 144)
        });
        bay.outfits.push(outfit(311, &[(MOD_AMMO, 150)]));
        bay.outfits.push(outfit(254, &[(MOD_WEAPON, 150)]));
        let bays = |ammo| StockWeapon {
            weapon: WeaponId(150),
            count: 1,
            ammo,
        };
        for hull in &mut bay.hulls {
            if hull.id == ShipId(128) {
                hull.weapons.push(bays(0));
            }
            if hull.id == ShipId(129) {
                hull.weapons = vec![bays(9)];
            }
        }
        let mut session = alongside(&bay);
        assert!(
            session
                .armament
                .mounts()
                .iter()
                .any(|mount| mount.spec.is_bay()),
            "the player can launch them"
        );
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Ok(Boarding::Opened(ON_BOARD)),
            "a bay's rounds are its fighters"
        );
    }

    #[test]
    fn a_tough_ship_taken_over_holds_a_tenth_of_its_armour_and_2() {
        let mut catalog = boardable();
        for hull in &mut catalog.hulls {
            if hull.id == ShipId(129) {
                hull.flags = crate::combat::hull::TOUGH;
            }
        }
        let mut session = alongside(&catalog);
        session.traffic.npcs_mut()[0].hull.tough = true;
        board_by(&mut session, &NovaLaw::default()).expect("boards");
        take(&mut session, Take::Capture, &[43, 1]);
        session.assign(Assignment::MyShip, &mut Draws::of(&[0]));
        assert!((session.reserves().armor.now - 6.5).abs() < 1e-4);
        assert_eq!(session.reserves().fuel.now, 0.0);
    }

    /// The outfits the player owns, as pairs of raw IDs and counts.
    fn owned_ids(session: &Session) -> Vec<(i16, u16)> {
        session
            .pilot()
            .outfits()
            .map(|(id, count)| (id.0, count))
            .collect()
    }

    #[test]
    fn a_capture_keeps_only_0x0004_whatever_ship_change_persistence_says() {
        // 404 is persistent through a mission's change of ship (0x0020)
        // alone: `H` keeps it by either reading, a capture by neither.
        let mut catalog = kitted();
        catalog.outfits.push(OutfitRecord {
            flags: OutfitFlags::MISSION_PERSISTENT,
            ..outfit(404, &[])
        });
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let rules = Rulebook::default().with_override(RuleKey::ShipChangePersistence, source);
            let mut session = captured(&catalog).with_rules(rules);
            session.pilot.outfits = BTreeMap::from([(OutfitId(400), 1), (OutfitId(404), 1)]);
            session.assign(Assignment::MyShip, &mut Draws::of(&[0]));
            assert_eq!(owned_ids(&session), [(400, 1), (402, 1)], "{source:?}");
        }
    }

    #[test]
    fn a_capture_holds_no_outfit_to_its_max_whatever_ship_change_max_says() {
        // 400 (persistent, owned once) and 402 are both up to 1, and the
        // trader's default items are one more 400 and three 402.
        let mut catalog = kitted();
        for record in &mut catalog.outfits {
            if matches!(record.id.0, 400 | 402) {
                record.max = 1;
            }
        }
        for record in &mut catalog.ship_records {
            if record.id == ShipId(129) {
                record.defaults = vec![(OutfitId(400), 1), (OutfitId(402), 3)];
            }
        }
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let rules = Rulebook::default().with_override(RuleKey::ShipChangeMax, source);
            let mut session = captured(&catalog).with_rules(rules);
            session.pilot.outfits = BTreeMap::from([(OutfitId(400), 1)]);
            session.assign(Assignment::MyShip, &mut Draws::of(&[0]));
            assert_eq!(owned_ids(&session), [(400, 2), (402, 3)], "{source:?}");
        }
    }

    #[test]
    fn a_capture_tops_the_stock_weapons_up_rather_than_adding_them() {
        // The trader stocks two blasters (held by 250, here persistent)
        // and a homing missile (252) with 6 rounds (403); the player owns
        // a blaster, and the trader's default items hold 4 rounds.
        let mut catalog = kitted();
        for hull in &mut catalog.hulls {
            if hull.id == ShipId(129) {
                hull.weapons = vec![
                    StockWeapon {
                        weapon: WeaponId(128),
                        count: 2,
                        ammo: 0,
                    },
                    StockWeapon {
                        weapon: WeaponId(134),
                        count: 1,
                        ammo: 6,
                    },
                ];
            }
        }
        catalog.outfits.push(outfit(403, &[(MOD_AMMO, 134)]));
        for record in &mut catalog.outfits {
            if record.id == OutfitId(250) {
                record.flags = OutfitFlags::PERSISTENT;
            }
        }
        for record in &mut catalog.ship_records {
            if record.id == ShipId(129) {
                record.defaults = vec![(OutfitId(402), 1), (OutfitId(403), 4)];
            }
        }
        let mut session = captured(&catalog);
        session.pilot.outfits = BTreeMap::from([(OutfitId(250), 1)]);
        session.assign(Assignment::MyShip, &mut Draws::of(&[0]));
        assert_eq!(
            owned_ids(&session),
            [(250, 2), (252, 1), (402, 1), (403, 6)],
            "what is held already counts: no blaster or round doubled"
        );
    }

    #[test]
    fn a_capture_holds_its_reserves_whatever_ship_change_reserves_says() {
        // 404, persistent, takes 35 armour: the trader's 45 falls to 10,
        // below the 17 a take-over leaves.
        let mut catalog = kitted();
        catalog.outfits.push(OutfitRecord {
            flags: OutfitFlags::PERSISTENT,
            ..outfit(404, &[(crate::stats::MORE_ARMOR, -35)])
        });
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let rules = Rulebook::default().with_override(RuleKey::ShipChangeReserves, source);
            let mut session = captured(&catalog).with_rules(rules);
            session.pilot.outfits = BTreeMap::from([(OutfitId(404), 1)]);
            session.assign(Assignment::MyShip, &mut Draws::of(&[0]));
            assert_eq!(
                session.reserves().armor,
                Gauge::full(10.0),
                "held to its most: {source:?}"
            );
        }
    }

    // Boarding grants.

    use crate::grant::Granted;
    use crate::traffic::npc::NpcPerson;

    /// [`boardable`] with "Ace" (`përs` 600, flying ship 129 anywhere)
    /// granting up to 2 outfits of class 7 at odds of 40, and the shield
    /// boosters of class 7 (`oütf` 200, 10 more shield each, a ton, up to
    /// 10), "shield booster" and "shield boosters".
    fn granting() -> FakePilotCatalog {
        let mut catalog = boardable();
        catalog.persons = vec![crate::catalog::PersonRecord {
            grant_class: 7,
            grant_prob: 40,
            grant_count: 2,
            ..crate::testkit::person(600, 129)
        }];
        catalog.outfits.push(OutfitRecord {
            item_class: 7,
            lc_name: "shield booster".to_owned(),
            lc_plural: "shield boosters".to_owned(),
            ..outfit(200, &[(crate::stats::MORE_SHIELD, 10)])
        });
        catalog
    }

    /// The shield booster (outfit 200) of [`granting`]'s `catalog`.
    fn booster(catalog: &mut FakePilotCatalog) -> &mut OutfitRecord {
        catalog
            .outfits
            .iter_mut()
            .find(|record| record.id == OutfitId(200))
            .expect("the booster")
    }

    /// Person `id` as the NPC flying it carries it, with nothing of its
    /// own.
    fn flown_by(id: i16) -> NpcPerson {
        NpcPerson {
            id: crate::catalog::PersonId(id),
            flags: 0,
            coward: 0,
            comm_quote: -1,
            hail_quote: -1,
            mission: false,
            portrait: None,
            invincible: false,
            grudge: false,
            quoted: false,
            quoted_at: None,
        }
    }

    /// [`alongside`], the trader flown by Ace, so of no booty.
    fn alongside_ace(catalog: &FakePilotCatalog) -> Session {
        let mut session = alongside(catalog);
        let trader = &mut session.traffic.npcs_mut()[0];
        trader.booty = 0;
        trader.person = Some(flown_by(600));
        session
    }

    /// Boards by Nova's law and `rule`, drawing `draws`: what it did, and
    /// the bounds asked.
    fn board_drawing(
        session: &mut Session,
        rule: NovaBoarding,
        draws: &[u32],
    ) -> (Result<Boarding, BoardRefusal>, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let boarded = session.board(&NovaLaw::default(), &rule, &mut chance);
        (boarded, chance.asked)
    }

    /// Ace's boarding's draws: the threshold, 170 energy, no jitter, the
    /// grant's odds at 39 (40, which grants) and its count at 50 (2).
    const ACE_DRAWS: [u32; 5] = [0, 17, 5, 39, 50];

    #[test]
    fn boarding_a_granting_person_adds_its_grant_after_the_plunder_and_the_odds() {
        let catalog = granting();
        let mut session = alongside_ace(&catalog);
        let shield = session.stats().shield;
        let (boarded, asked) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
        assert!(matches!(boarded, Ok(Boarding::Opened(_))), "{boarded:?}");
        assert_eq!(
            asked,
            [26, 30, 11, 100, 51],
            "the threshold, the energy and the jitter, then the grant"
        );
        assert_eq!(session.pilot().owned(OutfitId(200)), 2);
        assert_eq!(session.stats().shield, shield + 20.0, "refitted");
        assert_eq!(session.reserves().shield.now, shield + 20.0, "full");
        let granted = Granted {
            outfit: OutfitId(200),
            count: 2,
        };
        assert_eq!(session.take_grant(), Some(granted));
        assert_eq!(session.take_grant(), None, "taken once");
        assert!(!session.take_save_due(), "as plunder, a grant makes none");
        assert!(session.boarding().is_some(), "the plunder dialog opens");
    }

    #[test]
    fn a_grant_may_pick_an_outfit_whose_max_a_raiser_raises() {
        let mut catalog = granting();
        catalog
            .outfits
            .iter_mut()
            .find(|record| record.id == OutfitId(200))
            .expect("the booster")
            .max = 2;
        catalog
            .outfits
            .push(outfit(205, &[(crate::outfitter::RAISES_MAX, 200)]));
        for (raisers, granted) in [(0, None), (1, None), (2, Some(2))] {
            let mut session = alongside_ace(&catalog);
            session.pilot.outfits.insert(OutfitId(200), 2);
            if raisers > 0 {
                session.pilot.outfits.insert(OutfitId(205), raisers);
            }
            let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
            assert!(boarded.is_ok(), "{raisers}");
            assert_eq!(
                session.take_grant().map(|granted| granted.count),
                granted,
                "{raisers}"
            );
        }
    }

    #[test]
    fn a_raised_max_past_an_i16_is_held_to_its_range() {
        for (value, held) in [
            (i64::MIN, i16::MIN),
            (i64::from(i16::MIN) - 1, i16::MIN),
            (-5, -5),
            (0, 0),
            (i64::from(i16::MAX), i16::MAX),
            (i64::from(i16::MAX) + 1, i16::MAX),
        ] {
            assert_eq!(clamp_i16(value), held, "{value}");
        }
    }

    #[test]
    fn a_person_that_grants_nothing_or_misses_its_odds_draws_nothing_more() {
        let mut catalog = granting();
        let (_, asked) = board_drawing(
            &mut alongside_ace(&catalog),
            NovaBoarding::default(),
            &[0, 17, 5, 40],
        );
        assert_eq!(asked, [26, 30, 11, 100], "41 misses the odds of 40");
        catalog.persons[0].grant_class = 0;
        let mut session = alongside_ace(&catalog);
        let (_, asked) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
        assert_eq!(asked, [26, 30, 11], "no grant of class 0");
        assert_eq!(session.pilot().owned(OutfitId(200)), 0);
        assert_eq!(session.take_grant(), None);
    }

    #[test]
    fn a_ship_no_person_flies_draws_no_grant() {
        let catalog = granting();
        let mut session = alongside(&catalog);
        let (_, asked) = board_drawing(&mut session, NovaBoarding::default(), &BOARD_DRAWS);
        assert_eq!(asked, [26, 3, 1, 10, 30, 11]);
        assert_eq!(session.take_grant(), None);
    }

    #[test]
    fn boarders_repelled_are_granted_nothing() {
        let catalog = granting();
        let mut session = alongside_ace(&catalog);
        let bible = NovaBoarding {
            empty_booty: RuleSource::Bible,
            ..NovaBoarding::default()
        };
        let (boarded, asked) = board_drawing(&mut session, bible, &ACE_DRAWS);
        assert_eq!(boarded, Ok(Boarding::Repelled));
        assert!(asked.is_empty(), "{asked:?}");
        assert_eq!(session.pilot().owned(OutfitId(200)), 0);
        assert_eq!(session.take_grant(), None);
    }

    /// Loads `session` with 1000 tons of commodity 0 and 30 of `jünk`,
    /// past its 20 tons of holds, for the plunder dialog's resets: each
    /// trim cuts the commodity by 20 over everything held, to 19, 7, 3, 1
    /// and then none (1000 x 20/1030, 19 x 20/49, 7 x 20/37, 3 x 20/33,
    /// 1 x 20/31).
    fn trimmed_by_each_reset(session: &mut Session) {
        assert_eq!(session.capacity(), 20);
        session.pilot.cargo = [
            (Good::Commodity(0), 1000),
            (crate::session::precalced::tests::JUNK, 30),
        ]
        .into();
    }

    /// The commodity left by [`trimmed_by_each_reset`]'s cargo.
    fn commodity_left(session: &Session) -> u32 {
        session.pilot().held(Good::Commodity(0))
    }

    #[test]
    fn a_persons_grant_resets_before_and_after_it() {
        let catalog = granting();
        let mut session = alongside_ace(&catalog);
        trimmed_by_each_reset(&mut session);
        crate::session::precalced::tests::convicted(&mut session);
        let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
        assert!(matches!(boarded, Ok(Boarding::Opened(_))), "{boarded:?}");
        assert_eq!(session.pilot().owned(OutfitId(200)), 2);
        assert_eq!(
            commodity_left(&session),
            7,
            "trimmed twice, as the dialog opens: @0x931d9 and @0x93226"
        );
        assert_eq!(session.scan_latches, crate::scan::Latches::ARMED);
    }

    #[test]
    fn a_grant_picked_that_fits_none_resets_once_and_no_grant_resets_nothing() {
        let mut catalog = granting();
        booster(&mut catalog).mass = 1000;
        let mut session = alongside_ace(&catalog);
        trimmed_by_each_reset(&mut session);
        let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
        assert!(matches!(boarded, Ok(Boarding::Opened(_))), "{boarded:?}");
        assert_eq!(session.take_grant(), None, "nothing fits");
        assert_eq!(commodity_left(&session), 19, "@0x931d9 alone");
        for draws in [&[0, 17, 5, 40][..], &BOARD_DRAWS] {
            let mut session = if draws.len() == 4 {
                alongside_ace(&granting())
            } else {
                alongside(&granting())
            };
            trimmed_by_each_reset(&mut session);
            let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), draws);
            assert!(matches!(boarded, Ok(Boarding::Opened(_))), "{boarded:?}");
            assert_eq!(
                commodity_left(&session),
                1000,
                "no outfit picked: {draws:?}"
            );
        }
    }

    #[test]
    fn a_grant_weighs_each_outfit_by_its_raw_mass() {
        let mut catalog = granting();
        // 20 tons raw, but 8 by the ship's 40 tons with Flags 0x0400: two
        // would fit the 30 tons free scaled (by the scaled mass's other
        // reading, the bare product), and only one raw.
        let booster = catalog
            .outfits
            .iter_mut()
            .find(|record| record.id == OutfitId(200))
            .expect("the booster");
        booster.mass = 20;
        booster.flags = OutfitFlags::MASS_BY_MASS;
        let other = Rulebook::default().with_override(RuleKey::ScaledMass, RuleSource::Bible);
        let mut session = alongside_ace(&catalog).with_rules(other);
        let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
        assert!(boarded.is_ok());
        assert_eq!(session.pilot().owned(OutfitId(200)), 1);
        assert_eq!(session.take_grant().map(|granted| granted.count), Some(1));
    }

    #[test]
    fn a_grant_credits_back_the_mass_of_the_ships_stock_weapons() {
        let mut catalog = granting();
        // 15 tons each: two fit the 30 tons free only with the ton each of
        // the stock blaster and point defence (outfits 250 and 251) the
        // player owns credited back; without it, 28 tons fit one.
        let booster = catalog
            .outfits
            .iter_mut()
            .find(|record| record.id == OutfitId(200))
            .expect("the booster");
        booster.mass = 15;
        let mut session = alongside_ace(&catalog);
        assert_eq!(session.pilot().owned(OutfitId(250)), 1, "stock blaster");
        assert_eq!(session.pilot().owned(OutfitId(251)), 1, "stock defence");
        let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
        assert!(boarded.is_ok());
        assert_eq!(session.pilot().owned(OutfitId(200)), 2);
        assert_eq!(session.take_grant().map(|granted| granted.count), Some(2));
    }

    #[test]
    fn one_grant_max_holds_boarding_and_g_to_the_max_alike() {
        // Ace grants two of an outfit of `Max` 1. `GrantMax`, set once on
        // the session's outfit rules, holds the boarding grant (whatever
        // the boarding rule) and `G` to it; by the engine neither is held.
        let mut catalog = granting();
        booster(&mut catalog).max = 1;
        let g = |mut session: Session| {
            let expr = crate::control::SetExpr::parse("G200 G200").expect("parses");
            session.run_set(&expr, &mut Draws::of(&[]));
            session.pilot().owned(OutfitId(200))
        };
        for (rules, owned) in [
            (Rulebook::default(), 2),
            (
                Rulebook::default().with_override(RuleKey::GrantMax, RuleSource::Bible),
                1,
            ),
        ] {
            let mut boarded = alongside_ace(&catalog).with_rules(rules);
            board_drawing(&mut boarded, NovaBoarding::default(), &ACE_DRAWS)
                .0
                .expect("boards");
            assert_eq!(boarded.pilot().owned(OutfitId(200)), owned, "boarded");
            assert_eq!(g(alongside(&catalog).with_rules(rules)), owned, "G");
        }
    }

    #[test]
    fn a_boarding_grant_keeps_a_surplus_shield_and_armour_by_the_engine_and_holds_them_by_the_other_reading()
     {
        let catalog = granting();
        for (source, surplus) in [(RuleSource::Engine, 5.0), (RuleSource::Bible, 0.0)] {
            let mut session = alongside_ace(&catalog)
                .with_rules(Rulebook::default().with_override(RuleKey::RefitReserves, source));
            let (shield, armor) = (session.stats().shield, session.stats().armor);
            session.pilot.reserves.shield.now = shield + 5.0;
            session.pilot.reserves.armor.now = armor + 5.0;
            board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS)
                .0
                .expect("boards");
            assert_eq!(session.pilot().owned(OutfitId(200)), 2);
            let reserves = session.reserves();
            assert_eq!(
                reserves.shield,
                Gauge {
                    now: shield + 20.0 + surplus,
                    max: shield + 20.0
                },
                "{source:?}: the grant's shield gained"
            );
            assert_eq!(
                reserves.armor,
                Gauge {
                    now: armor + surplus,
                    max: armor
                },
                "{source:?}"
            );
        }
    }

    #[test]
    fn held_to_the_max_boarding_and_g_weigh_an_outfit_as_the_outfitter_does() {
        // Of a `Mass` scaled by Flags 0x0400 on the ship's 40 tons, with
        // 30 tons free: 20 raw is 8, 75 is 30 and 80 is 32. Held to the
        // `Max`, boarding (granting one) and `G` agree, by the scaled mass
        // (its other reading, the bare product).
        let rules = Rulebook::default()
            .with_override(RuleKey::GrantMax, RuleSource::Bible)
            .with_override(RuleKey::ScaledMass, RuleSource::Bible);
        for (mass, fits) in [(20, true), (75, true), (80, false)] {
            let mut catalog = granting();
            catalog.persons[0].grant_count = 1;
            booster(&mut catalog).mass = mass;
            booster(&mut catalog).flags = OutfitFlags::MASS_BY_MASS;
            let mut boarded = alongside_ace(&catalog).with_rules(rules);
            assert_eq!(boarded.free_mass(), 30);
            board_drawing(&mut boarded, NovaBoarding::default(), &ACE_DRAWS)
                .0
                .expect("boards");
            let mut scripted = alongside(&catalog).with_rules(rules);
            let expr = crate::control::SetExpr::parse("G200").expect("parses");
            scripted.run_set(&expr, &mut Draws::of(&[]));
            let owned = u16::from(fits);
            assert_eq!(
                boarded.pilot().owned(OutfitId(200)),
                owned,
                "boarded {mass}"
            );
            assert_eq!(scripted.pilot().owned(OutfitId(200)), owned, "G {mass}");
        }
    }

    #[test]
    fn boarding_a_person_granting_a_map_explores_and_adds_nothing() {
        let mut catalog = granting();
        booster(&mut catalog).mods = [(crate::outfit_effects::MAP, 1), (0, 0), (0, 0), (0, 0)];
        let mut session = alongside_ace(&catalog);
        let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
        assert!(boarded.is_ok());
        assert_eq!(session.pilot().owned(OutfitId(200)), 0);
        assert!(session.pilot().has_explored(SystemId(131)), "a jump away");
        assert_eq!(
            session.take_grant(),
            Some(Granted {
                outfit: OutfitId(200),
                count: 2,
            }),
            "it is still told"
        );
    }

    #[test]
    fn an_outfits_lower_case_names_are_its_records() {
        let catalog = granting();
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session.outfit_names(OutfitId(200)),
            Some(("shield booster", "shield boosters"))
        );
        assert_eq!(session.outfit_names(OutfitId(999)), None);
    }

    /// [`boardable`] with `others` police besides the trader in the
    /// system.
    fn crowded(others: usize) -> Session {
        let mut catalog = boardable();
        catalog.traffic[0].1.avg_ships = (others + 1) as i16;
        let mut placed = vec![(0, 0, 0, 0)];
        placed.extend(std::iter::repeat_n((1, 0, 740, 0), others));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut placing(2, &placed));
        assert_eq!(session.npcs().len(), others + 1);
        stop_npcs(&mut session);
        let trader = &mut session.traffic.npcs_mut()[0];
        trader.condition = Condition::Disabled;
        session.target = Some(NpcId(0));
        board_by(&mut session, &NovaLaw::default()).expect("boards");
        take(&mut session, Take::Capture, &[43, 1]);
        session
    }

    #[test]
    fn with_no_ship_slot_free_for_the_old_ship_the_swap_is_dropped() {
        let mut full = crowded(62);
        let pilot = full.pilot().clone();
        assert_eq!(
            full.assign(Assignment::MyShip, &mut Draws::of(&[])),
            Some(Assigned::Abandoned),
            "63 NPCs and the player: no slot"
        );
        assert_eq!(full.pilot(), &pilot, "nothing changes");
        assert_eq!(full.npcs().len(), 63);
        assert_eq!(full.boarding(), None, "the prize is lost");
        assert!(!full.take_save_due());
        let mut roomy = crowded(61);
        assert_eq!(
            roomy.assign(Assignment::MyShip, &mut Draws::of(&[])),
            Some(Assigned::MyShip),
            "62 NPCs and the player: one slot"
        );
    }

    #[test]
    fn only_use_as_my_ship_tells_the_ship_changed() {
        let catalog = boardable();
        let mut escort = captured(&catalog);
        escort.take_events();
        escort.assign(Assignment::Escort, &mut Draws::of(&[]));
        assert_eq!(escort.take_events(), [], "an escort");
        let mut mine = captured(&catalog);
        mine.take_events();
        assert_eq!(
            mine.assign(Assignment::MyShip, &mut Draws::of(&[])),
            Some(Assigned::MyShip)
        );
        assert_eq!(mine.take_events(), [SessionEvent::ShipChanged], "my ship");
        let mut full = crowded(62);
        full.take_events();
        assert_eq!(
            full.assign(Assignment::MyShip, &mut Draws::of(&[])),
            Some(Assigned::Abandoned)
        );
        assert_eq!(full.take_events(), [], "abandoned");
    }

    #[test]
    fn no_fuel_is_sold_in_flight() {
        let mut session = half_empty_at_port();
        session.take_off(&NoTraffic, &mut NeverFires);
        session.take_save_due();
        assert_refused(session, RechargeRefusal::NoFuel);
    }

    // Navigation.

    /// [`catalog`] with system 130 holding three planets whose navigation
    /// order, 131, 128, 129, is not their order by distance from the
    /// centre, where the ship starts: 128 is nearest, then 131, then 129.
    pub(super) fn three_stellars() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.sites[0].1 = vec![
            planet(131, 900.0, 0.0),
            planet(128, 30.0, -40.0),
            planet(129, 2000.0, 0.0),
        ];
        catalog
    }

    #[test]
    fn a_session_starts_with_no_navigation_target() {
        let session = Session::start(&three_stellars()).expect("starts");
        assert_eq!(session.nav_target(), None);
    }

    #[test]
    fn the_target_and_the_course_are_independent() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_stellar(StellarPick::Slot(0));
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.nav_target(), Some(StellarId(131)));
        session.select_stellar(StellarPick::Slot(1));
        assert_eq!(session.course(), ids(&[131, 132]));
        assert_eq!(session.nav_target(), Some(StellarId(128)));
    }

    #[test]
    fn arriving_in_another_system_clears_the_target() {
        let catalog = three_stellars();
        let mut session = Session::start(&catalog).expect("starts");
        session.select_stellar(StellarPick::Slot(0));
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(
            session.nav_target(),
            Some(StellarId(131)),
            "kept while jumping"
        );
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.nav_target(), None);
        assert_eq!(
            session.select_stellar(StellarPick::Slot(0)),
            Some(StellarId(140))
        );
    }

    #[test]
    fn a_slot_pick_selects_the_stellar_the_system_lists_there() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        for (slot, expected) in [(0, 131), (2, 129), (1, 128)] {
            assert_eq!(
                session.select_stellar(StellarPick::Slot(slot)),
                Some(StellarId(expected))
            );
            assert_eq!(session.nav_target(), Some(StellarId(expected)));
        }
    }

    #[test]
    fn a_slot_past_the_last_stellar_selects_nothing_and_keeps_the_target() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_stellar(StellarPick::Slot(2));
        assert_eq!(session.select_stellar(StellarPick::Slot(3)), None);
        assert_eq!(session.nav_target(), Some(StellarId(129)));
    }

    #[test]
    fn the_nearest_pick_selects_what_l_would_from_no_target() {
        let mut catalog = three_stellars();
        // 128, the nearest, cannot be landed on: L passes over it.
        catalog.sites[0].1[1].flags = 0;
        let mut session = Session::start(&catalog).expect("starts");
        session.select_stellar(StellarPick::Slot(2));
        assert_eq!(
            session.select_stellar(StellarPick::Nearest),
            Some(StellarId(131))
        );
        assert_eq!(session.nav_target(), Some(StellarId(131)));

        let mut session = Session::start(&catalog).expect("starts");
        let LandPress::Outcome(LandOutcome::Selected { stellar, .. }) =
            session.land().expect("selects")
        else {
            panic!("L selects");
        };
        assert_eq!(stellar, StellarId(131), "L agrees");
    }

    #[test]
    fn a_click_pick_selects_the_stellar_under_it_or_nothing() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        assert_eq!(
            session.select_stellar(StellarPick::At(Vec2::new(940.0, 30.0))),
            Some(StellarId(131))
        );
        assert_eq!(
            session.select_stellar(StellarPick::At(Vec2::new(500.0, 500.0))),
            None
        );
        assert_eq!(session.nav_target(), Some(StellarId(131)), "kept");
        assert_eq!(
            session.select_stellar(StellarPick::At(Vec2::new(2000.0, 0.0))),
            Some(StellarId(129))
        );
    }

    #[test]
    fn a_system_with_no_stellars_has_none_to_pick() {
        let empty = FakePilotCatalog {
            sites: Vec::new(),
            ..catalog()
        };
        let mut session = Session::start(&empty).expect("starts");
        for pick in [
            StellarPick::Slot(0),
            StellarPick::Nearest,
            StellarPick::At(Vec2::ZERO),
        ] {
            assert_eq!(session.select_stellar(pick), None, "{pick:?}");
        }
        assert_eq!(session.nav_target(), None);
    }

    #[test]
    fn picking_a_stellar_leaves_the_course_alone_and_makes_no_save_due() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        session.select_stellar(StellarPick::Slot(0));
        session.select_stellar(StellarPick::Nearest);
        assert_eq!(session.course(), ids(&[131, 132]));
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_new_pick_replaces_the_target_and_l_lands_on_it() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(session.nav_target(), Some(StellarId(131)));
        assert_eq!(
            session.select_stellar(StellarPick::Slot(1)),
            Some(StellarId(128))
        );
        // The ship starts over 128, at rest.
        assert_eq!(
            session.land(),
            Ok(LandOutcome::Landed(StellarId(128)).into())
        );
    }

    #[test]
    fn clearing_the_target_leaves_none() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        session.select_stellar(StellarPick::Slot(0));
        assert!(session.clear_nav_target());
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.course(), ids(&[131, 132]), "course kept");
        assert!(!session.clear_nav_target(), "nothing to clear");
        assert_eq!(session.nav_target(), None);
        assert!(!session.take_save_due());
    }

    #[test]
    fn no_stellar_is_picked_or_cleared_during_a_jump() {
        let mut session = Session::start(&hub()).expect("starts");
        session.select_next_system();
        session.select_stellar(StellarPick::Slot(0));
        fly_out(&mut session);
        session.player.heading = 90.0;
        assert_eq!(session.begin_jump(), Ok(SystemId(134)));
        for stage in ["pre-jump", "in hyperspace"] {
            for pick in [
                StellarPick::Slot(1),
                StellarPick::Nearest,
                StellarPick::At(Vec2::new(30.0, -40.0)),
            ] {
                assert_eq!(session.select_stellar(pick), None, "{stage} {pick:?}");
            }
            assert!(!session.clear_nav_target(), "{stage}");
            assert_eq!(session.nav_target(), Some(StellarId(131)), "{stage}");
            if session.jumping().is_none() {
                begin_jump_now(&mut session).expect("jumps");
            }
        }
        assert_eq!(session.jumping(), Some(SystemId(134)));
    }

    // Hyper Select.

    /// [`three_stellars`] with 130 a hub: it lists 134, 131 (twice),
    /// itself, a missing 999 and 135, in that Con order; 136 lists 130
    /// one way; 131 lists 132; 133 lists nothing and exists too.
    pub(super) fn hub() -> FakePilotCatalog {
        let mut catalog = three_stellars();
        catalog.star_map = vec![
            star(130, (0.0, 0.0), &[134, 131, 131, 130, 999, 135]),
            star(131, (600.0, 0.0), &[132]),
            star(132, (600.0, 600.0), &[]),
            star(133, (-600.0, 0.0), &[]),
            star(134, (0.0, -600.0), &[]),
            star(135, (0.0, 600.0), &[]),
            star(136, (-600.0, -600.0), &[130]),
        ];
        catalog.systems.push(SystemId(133));
        catalog
    }

    #[test]
    fn the_hyper_select_key_cycles_the_listed_systems_and_wraps() {
        let mut session = Session::start(&hub()).expect("starts");
        for expected in [134, 131, 135, 134] {
            assert_eq!(session.select_next_system(), Some(SystemId(expected)));
            assert_eq!(session.course(), ids(&[expected]));
        }
    }

    #[test]
    fn a_hyper_select_replaces_a_multi_jump_course_with_one_jump() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.course(), ids(&[131, 132]));
        assert_eq!(session.select_next_system(), Some(SystemId(135)));
        assert_eq!(session.course(), ids(&[135]));
    }

    #[test]
    fn a_hyper_select_clears_the_stellar_nav_target() {
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(
            session.select_next_system(),
            Some(SystemId(134)),
            "a stellar targeted: the first listed, not the one after 131"
        );
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.select_next_system(), Some(SystemId(131)));
    }

    #[test]
    fn a_hyper_select_makes_no_save_due() {
        let mut session = Session::start(&hub()).expect("starts");
        session.select_next_system();
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_jump_goes_to_the_selected_system() {
        let catalog = hub();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.select_next_system(), Some(SystemId(134)));
        assert_eq!(session.select_next_system(), Some(SystemId(131)));
        fly_out(&mut session);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.course(), []);
    }

    #[test]
    fn no_hyper_select_during_a_jump() {
        let mut session = Session::start(&hub()).expect("starts");
        session.select_next_system();
        fly_out(&mut session);
        session.player.heading = 90.0;
        assert_eq!(session.begin_jump(), Ok(SystemId(134)));
        assert_eq!(session.preparing_jump(), Some(SystemId(134)));
        assert_eq!(session.select_next_system(), None, "pre-jump");
        assert_eq!(session.course(), ids(&[134]));
        begin_jump_now(&mut session).expect("jumps");
        assert_eq!(session.jumping(), Some(SystemId(134)));
        assert_eq!(session.select_next_system(), None, "in hyperspace");
        assert_eq!(session.course(), ids(&[134]));
    }

    #[test]
    fn a_system_with_no_listed_links_selects_nothing() {
        let mut catalog = hub();
        catalog.character = starting([Some(133), None, None, None]).character;
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.system(), SystemId(133));
        assert_eq!(session.select_next_system(), None);
        assert_eq!(session.course(), []);
    }

    #[test]
    fn plotting_a_course_follows_one_way_links_by_the_engines_reading() {
        // 136 lists 130 one way: by the engine, 130 cannot jump to it.
        let mut session = Session::start(&hub()).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        assert_eq!(
            session.plot_course(SystemId(136)),
            Err(RouteError::Unreachable)
        );
        assert_eq!(session.course(), [], "cleared");
        let mut session = Session::start(&hub())
            .expect("starts")
            .with_hyperlinks(HyperlinkRule::BothWays);
        assert_eq!(session.plot_course(SystemId(136)), Ok(&ids(&[136])[..]));
    }

    #[test]
    fn hyper_select_by_the_one_jump_course_reading_never_jumps_against_a_one_way_link() {
        let cycle = |hyperlinks| {
            let mut session = Session::start(&hub())
                .expect("starts")
                .with_hyper_select(HyperSelectRule::OneJumpCourse)
                .with_hyperlinks(hyperlinks);
            (0..4)
                .map(|_| session.select_next_system().expect("one"))
                .collect::<Vec<_>>()
        };
        assert_eq!(cycle(HyperlinkRule::Engine), ids(&[131, 134, 135, 131]));
        assert_eq!(cycle(HyperlinkRule::BothWays), ids(&[131, 134, 135, 136]));
    }

    #[test]
    fn hyper_select_by_the_one_jump_course_reading_cycles_every_jump() {
        let mut session = Session::start(&hub())
            .expect("starts")
            .with_hyper_select(HyperSelectRule::OneJumpCourse);
        assert_eq!(session.select_next_system(), Some(SystemId(131)));
        assert_eq!(session.select_next_system(), Some(SystemId(134)));
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(
            session.select_next_system(),
            Some(SystemId(131)),
            "a multi-jump course: the first"
        );
    }

    // Hypergates and wormholes.

    /// Stellar `id` at (`x`, `y`), with `flags2`, its `links` and its exit
    /// angle.
    fn gate_site(
        id: i16,
        (x, y): (f32, f32),
        flags2: u16,
        links: &[i16],
        exit_angle: i16,
    ) -> GateSite {
        let mut slots = [None; 8];
        for (slot, &link) in slots.iter_mut().zip(links) {
            *slot = Some(StellarId(link));
        }
        GateSite {
            id: StellarId(id),
            position: Vec2::new(x, y),
            flags2,
            links: slots,
            exit_angle,
        }
    }

    /// A landable stellar `id` at the centre with `flags2`.
    fn gate_landing(id: i16, flags2: u16) -> LandingSite {
        LandingSite {
            flags2,
            ..planet(id, 0.0, 0.0)
        }
    }

    /// The ship starts over hypergate 300, at system 130's centre, linked
    /// to 310 in 131 at (100, 200), heading ships out on 90°, then to 320
    /// in 132 (a random heading), then to 999, which no system lists.
    pub(super) fn gated() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![
                (SystemId(130), vec![gate_landing(300, HYPERGATE)]),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            gates: vec![
                gate_site(300, (0.0, 0.0), 0x1200, &[310, 320, 999], 120),
                gate_site(310, (100.0, 200.0), HYPERGATE, &[300], 90),
                gate_site(320, (-50.0, 0.0), HYPERGATE, &[], -1),
            ],
            gate_listers: vec![
                (SystemId(131), StellarId(310)),
                (SystemId(132), StellarId(320)),
            ],
            ..catalog()
        }
    }

    /// [`gated`]'s session, with L pressed twice over the hypergate: the
    /// entry awaits the player's pick.
    pub(super) fn at_gate(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert!(matches!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Selected { .. }))
        ));
        assert_eq!(
            session.land(),
            Ok(LandPress::AtGate {
                stellar: StellarId(300),
                kind: GateKind::Hypergate,
            })
        );
        session
    }

    #[test]
    fn l_twice_over_a_cleared_hypergate_awaits_the_entry_and_does_not_dock() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        assert_eq!(session.landed(), None);
        assert_eq!(session.pilot().stellar(), None);
        assert!(!session.take_save_due());
        assert!(
            !session
                .take_sounds()
                .iter()
                .any(|sound| matches!(sound, SimSound::Landed { .. })),
            "no landing sound"
        );
        assert_eq!(
            session.open_hypergate(&catalog),
            Ok(ids(&[131, 132])),
            "each linked gate's system, in slot order"
        );
        assert_eq!(session.gate_kind(StellarId(300)), Some(GateKind::Hypergate));
        assert_eq!(session.gate_kind(StellarId(310)), None, "not here");
    }

    #[test]
    fn no_destinations_are_offered_without_an_entry_pending() {
        let catalog = gated();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session.open_hypergate(&catalog),
            Err(GateRefusal::NotAtGate)
        );
        assert_eq!(*catalog.gate_reads.borrow(), 0, "not read at the start");
    }

    #[test]
    fn a_tick_ends_a_pending_entry() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        session.tick(Controls::default());
        assert_eq!(
            session.open_hypergate(&catalog),
            Err(GateRefusal::NotAtGate)
        );
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Err(GateRefusal::NotAtGate)
        );
        assert_eq!(session.system(), SystemId(130));
    }

    #[test]
    fn entering_a_hypergate_comes_out_of_the_gate_picked_at_half_speed() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        session.plot_course(SystemId(132)).expect("a route");
        session.take_sounds();
        let mut chance = Scripted::default();
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut chance),
            Ok(SystemId(131))
        );
        assert_eq!(session.system(), SystemId(131));
        let player = *session.player();
        assert_eq!(player.position, Vec2::new(100.0, 200.0));
        assert_eq!(player.heading, 90.0);
        let half = session.top_speed() / 2.0;
        assert!((player.velocity - Vec2::new(half, 0.0)).length() < 1e-4);
        assert!(session.pilot().has_explored(SystemId(131)));
        assert_eq!(session.pilot().stellar(), None);
        assert_eq!(session.course(), [], "the course cleared");
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.reserves().fuel, Gauge::full(300.0), "no fuel");
        assert_eq!(dmy(&session), (23, 6, 1177), "no day passes");
        assert!(chance.asked.is_empty() && chance.sides_asked.is_empty());
        assert_eq!(session.take_sounds(), [SimSound::Arrived]);
        assert_eq!(
            session.take_messages(),
            [SimMessage::ExitedHypergate(SystemId(131))]
        );
        assert_eq!(
            catalog.sites_asked.borrow().last(),
            Some(&SystemId(131)),
            "the new system's stellars read"
        );
        assert_eq!(
            session.enter_hypergate(Some(SystemId(130)), &catalog, &mut chance),
            Err(GateRefusal::NotAtGate),
            "the entry is over"
        );
    }

    #[test]
    fn a_gate_with_an_angle_outside_0_to_359_heads_the_ship_out_at_random() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        let mut chance = Scripted::rolling(&[270]);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(132)), &catalog, &mut chance),
            Ok(SystemId(132))
        );
        assert_eq!(chance.sides_asked, [360]);
        assert_eq!(session.player().heading, 270.0);
        assert_eq!(session.player().position, Vec2::new(-50.0, 0.0));
    }

    #[test]
    fn no_pick_or_one_no_link_leads_to_cancels_and_clears_the_target() {
        let catalog = gated();
        for to in [None, Some(SystemId(133))] {
            let mut session = at_gate(&catalog);
            assert_eq!(session.nav_target(), Some(StellarId(300)));
            assert_eq!(
                session.enter_hypergate(to, &catalog, &mut NeverFires),
                Err(GateRefusal::Cancelled),
                "{to:?}"
            );
            assert_eq!(session.nav_target(), None);
            assert_eq!(session.system(), SystemId(130));
            assert_eq!(session.player().position, Vec2::ZERO);
            assert_eq!(session.take_messages(), []);
            assert_eq!(
                session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
                Err(GateRefusal::NotAtGate),
                "the entry is over"
            );
        }
    }

    #[test]
    fn a_hypergate_without_links_leads_nowhere() {
        let catalog = FakePilotCatalog {
            gates: vec![gate_site(300, (0.0, 0.0), HYPERGATE, &[0, -2], 0)],
            ..gated()
        };
        let mut session = at_gate(&catalog);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Err(GateRefusal::NoLinks)
        );
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.system(), SystemId(130));
    }

    #[test]
    fn a_hypergate_whose_links_lead_nowhere_cancels() {
        let catalog = FakePilotCatalog {
            gates: vec![gate_site(300, (0.0, 0.0), HYPERGATE, &[999], 0)],
            ..gated()
        };
        let mut session = at_gate(&catalog);
        assert_eq!(
            session.enter_hypergate(None, &catalog, &mut NeverFires),
            Err(GateRefusal::Cancelled)
        );
    }

    #[test]
    fn a_hypergate_with_links_offers_its_map_even_when_they_lead_nowhere() {
        for (links, offered) in [(&[310, 320][..], ids(&[131, 132])), (&[999][..], vec![])] {
            let mut catalog = gated();
            catalog.gates[0] = gate_site(300, (0.0, 0.0), HYPERGATE, links, 0);
            let mut session = at_gate(&catalog);
            assert_eq!(
                session.open_hypergate(&catalog),
                Ok(offered),
                "choose among them: {links:?}"
            );
            assert_eq!(session.nav_target(), Some(StellarId(300)), "{links:?}");
            assert_eq!(
                session.enter_hypergate(None, &catalog, &mut NeverFires),
                Err(GateRefusal::Cancelled),
                "the entry still awaits the pick: {links:?}"
            );
        }
    }

    #[test]
    fn a_hypergate_without_links_refuses_to_offer_its_map_and_ends_the_entry() {
        let catalog = FakePilotCatalog {
            gates: vec![gate_site(300, (0.0, 0.0), HYPERGATE, &[0, -2], 0)],
            ..gated()
        };
        let mut session = at_gate(&catalog);
        assert_eq!(session.open_hypergate(&catalog), Err(GateRefusal::NoLinks));
        assert_eq!(session.nav_target(), None, "the target cleared");
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(
            session.enter_hypergate(None, &catalog, &mut NeverFires),
            Err(GateRefusal::NotAtGate),
            "the entry is over"
        );
    }

    /// The ship starts over unlinked wormhole 400, at system 130's centre;
    /// unlinked wormhole 410 is in 131 at (-300, 40), and wormhole 420,
    /// linked to 410, in 132.
    pub(super) fn wormholes() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![
                (SystemId(130), vec![gate_landing(400, 0x2200)]),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            gates: vec![
                gate_site(400, (0.0, 0.0), 0x2200, &[], 120),
                gate_site(410, (-300.0, 40.0), WORMHOLE, &[], 0),
                gate_site(420, (5.0, 5.0), WORMHOLE, &[410], 0),
            ],
            gate_listers: vec![
                (SystemId(131), StellarId(410)),
                (SystemId(132), StellarId(420)),
            ],
            ..catalog()
        }
    }

    /// `catalog`'s session with L pressed twice over the wormhole.
    pub(super) fn at_wormhole(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.land().expect("clearance");
        assert_eq!(
            session.land(),
            Ok(LandPress::AtGate {
                stellar: StellarId(400),
                kind: GateKind::Wormhole,
            })
        );
        session
    }

    #[test]
    fn entering_a_wormhole_comes_out_of_the_one_rolled() {
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog);
        assert_eq!(
            session.open_hypergate(&catalog),
            Err(GateRefusal::NotAtGate),
            "not a hypergate"
        );
        session.take_sounds();
        let mut chance = Scripted::rolling(&[0]);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut chance),
            Ok(SystemId(131))
        );
        assert_eq!(chance.sides_asked, [2]);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.player().position, Vec2::new(-300.0, 40.0));
        assert_eq!(session.player().heading, 0.0);
        assert_eq!(session.reserves().fuel, Gauge::full(300.0));
        assert_eq!(dmy(&session), (23, 6, 1177));
        assert_eq!(session.take_sounds(), [SimSound::Arrived]);
        assert_eq!(
            session.take_messages(),
            [SimMessage::PassedWormhole(SystemId(131))]
        );
        assert_eq!(
            session.enter_wormhole(&catalog, &mut chance),
            Err(GateRefusal::NotAtGate)
        );
    }

    /// Holds `P300` in `session`, by the engine's `script_sound` reading.
    fn hold_p300(session: &mut Session) {
        session.run_set(
            &crate::control::SetExpr::parse("P300").expect("parses"),
            &mut Scripted::default(),
        );
    }

    /// The first flight tick after a gate exit sounds the held `P300`, as
    /// the original's gate exits (`_PlayerEnterHypergate` @0x637bf,
    /// `_PlayerEnterWormhole` @0x64005) leave `_missionSoundID` alone.
    fn assert_held_sound_plays_next_tick(session: &mut Session) {
        assert_eq!(session.take_sounds(), [SimSound::Arrived], "not yet");
        session.tick(Controls::default());
        assert_eq!(
            session.take_sounds(),
            [SimSound::Script {
                sound: SoundId(300),
                exclusive: true,
            }]
        );
    }

    #[test]
    fn by_the_engine_a_held_sound_outlives_a_wormhole_exit() {
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog);
        session.take_sounds();
        hold_p300(&mut session);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut Scripted::rolling(&[0])),
            Ok(SystemId(131))
        );
        assert_held_sound_plays_next_tick(&mut session);
    }

    #[test]
    fn by_the_engine_a_held_sound_outlives_a_hypergate_exit() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        session.take_sounds();
        hold_p300(&mut session);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut Scripted::default()),
            Ok(SystemId(131))
        );
        assert_held_sound_plays_next_tick(&mut session);
    }

    #[test]
    fn an_arrival_tells_the_system_changed() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert_eq!(session.take_events(), [SessionEvent::SystemChanged]);
    }

    #[test]
    fn a_gates_exit_tells_the_system_changed_even_into_the_system_gone_in() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        assert_eq!(
            session.enter_hypergate(None, &catalog, &mut Scripted::default()),
            Err(GateRefusal::Cancelled)
        );
        assert_eq!(session.take_events(), [], "cancelled");
        let mut session = at_gate(&catalog);
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut Scripted::default())
            .expect("through");
        assert_eq!(session.take_events(), [SessionEvent::SystemChanged]);
        let catalog = wormholes();
        for (roll, system) in [(0, 131), (1, 130)] {
            let mut session = at_wormhole(&catalog);
            assert_eq!(
                session.enter_wormhole(&catalog, &mut Scripted::rolling(&[roll])),
                Ok(SystemId(system))
            );
            assert_eq!(
                session.take_events(),
                [SessionEvent::SystemChanged],
                "{system}"
            );
        }
    }

    #[test]
    fn a_wormhole_rolled_onto_a_linked_one_comes_out_where_it_went_in() {
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog);
        let mut chance = Scripted::rolling(&[1]);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut chance),
            Ok(SystemId(130))
        );
        assert_eq!(session.player().position, Vec2::ZERO);
        assert_eq!(session.player().heading, 120.0);
        assert_eq!(
            session.take_messages(),
            [SimMessage::PassedWormhole(SystemId(130))]
        );
    }

    #[test]
    fn by_the_bible_a_wormhole_rolls_over_the_unlinked_ones_only() {
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog).with_wormholes(WormholeRule::UnlinkedOnly);
        let mut chance = Scripted::default();
        assert_eq!(
            session.enter_wormhole(&catalog, &mut chance),
            Ok(SystemId(131))
        );
        assert_eq!(chance.sides_asked, [1]);
    }

    #[test]
    fn a_wormhole_with_nowhere_to_lead_refuses() {
        let catalog = FakePilotCatalog {
            gates: vec![gate_site(400, (0.0, 0.0), WORMHOLE, &[], 0)],
            ..wormholes()
        };
        let mut session = at_wormhole(&catalog);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut NeverFires),
            Err(GateRefusal::NoExit)
        );
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.take_messages(), []);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut NeverFires),
            Err(GateRefusal::NotAtGate),
            "the entry is over"
        );
    }

    #[test]
    fn each_gate_is_entered_only_its_own_way() {
        let hypergates = gated();
        let mut session = at_gate(&hypergates);
        assert_eq!(
            session.enter_wormhole(&hypergates, &mut NeverFires),
            Err(GateRefusal::NotAtGate)
        );
        assert_eq!(session.system(), SystemId(130));
        let holes = wormholes();
        let mut session = at_wormhole(&holes);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &holes, &mut NeverFires),
            Err(GateRefusal::NotAtGate)
        );
        assert_eq!(session.system(), SystemId(130));
    }

    #[test]
    fn like_a_jump_a_gate_places_the_ship_as_a_jump_and_lets_its_days_pass() {
        let catalog = FakePilotCatalog {
            disasters: surplus().disasters,
            ..gated()
        };
        let mut session = at_gate(&catalog).with_gate_arrival(GateArrivalRule::LikeJump);
        let mut chance = Scripted::answering(&[true]);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut chance),
            Ok(SystemId(131))
        );
        assert_eq!(
            *session.player(),
            crate::hyperspace::arrival(Vec2::ZERO, Vec2::new(600.0, 0.0), MIN_JUMP_DISTANCE)
        );
        assert_eq!(dmy(&session), (24, 6, 1177), "a jump's day");
        assert_eq!(chance.asked, [35], "the day's events rolled");
        assert_eq!(session.pilot().events().count(), 1);
        assert_eq!(session.reserves().fuel, Gauge::full(300.0), "still no fuel");
    }

    #[test]
    fn by_the_engine_a_gate_lets_no_day_pass_even_with_events_to_roll() {
        let catalog = FakePilotCatalog {
            disasters: surplus().disasters,
            ..gated()
        };
        let mut session = at_gate(&catalog);
        let mut chance = Scripted::answering(&[true]);
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut chance)
            .expect("enters");
        assert!(chance.asked.is_empty());
        assert_eq!(session.pilot().events().count(), 0);
    }

    #[test]
    fn a_gate_entered_mid_thrust_stops_the_thrust() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        session.thrusting = true;
        session.engine_glow = 10;
        session.take_sounds();
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
            .expect("enters");
        assert_eq!(
            session.take_sounds(),
            [SimSound::ThrustStopped, SimSound::Arrived]
        );
        assert_eq!(session.engine_glow(), 0);
        assert!(!session.thrusting());
    }

    // Entering a system: what each way in lets go of, and the order of
    // its draws.

    /// `base` with traffic: two ships in 130 and three in 131 and 132,
    /// all of düde 128 (ship 129, for govt 140); the player's ship
    /// carries a blaster, outfit 250 holding it.
    fn busy(base: FakePilotCatalog) -> FakePilotCatalog {
        let dudes = trafficked(130, 2, 1);
        let traffic = |system, avg| trafficked(system, avg, 1).traffic[0];
        FakePilotCatalog {
            traffic: vec![traffic(130, 2), traffic(131, 3), traffic(132, 3)],
            dudes: dudes.dudes,
            ship_records: dudes.ship_records,
            weapons: vec![WeaponRecord {
                reload: 2,
                count: 30,
                speed: 2000,
                ..weapon(128)
            }],
            hulls: vec![HullRecord {
                weapons: vec![StockWeapon {
                    weapon: WeaponId(128),
                    count: 1,
                    ammo: 0,
                }],
                ..hull(128)
            }],
            outfits: vec![outfit(250, &[(MOD_WEAPON, 128)])],
            ..base
        }
    }

    /// `session`, entering a gate, with an escort of ship 129 among the
    /// system's traffic, a shot in flight, and the scene under way
    /// ([`Session::stage_scene`]) aimed at its first NPC.
    fn in_the_thick_of_it(mut session: Session, catalog: &FakePilotCatalog) -> Session {
        session.pilot.escorts = vec![Escort {
            ship: ShipId(129),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }];
        session.populate(catalog, &mut NeverFires);
        session.hold_fire(true, false);
        session.tick_combat(Rules::default(), &mut NeverFires);
        session.hold_fire(false, false);
        assert_eq!(session.shots().len(), 1);
        session.stage_scene(session.npcs()[0].id);
        session
    }

    /// Asserts `session` came out into a system of three düde ships,
    /// the fight and the scene left behind and its escort coming out of
    /// the exit gate after it.
    fn assert_came_out_afresh(session: &Session, case: &str) {
        assert_eq!(session.shots(), [], "{case}");
        session.assert_scene_left(case);
        let fleet: Vec<NpcId> = session.fleet.iter().flatten().copied().collect();
        assert_eq!(fleet.len(), 1, "{case}");
        let escort = session
            .npcs()
            .iter()
            .find(|npc| npc.id == fleet[0])
            .expect("the escort is in");
        assert!(escort.escort.is_some(), "an escort: {case}");
        assert!(
            escort.emerging() && escort.led_out,
            "led out of the exit gate: {case}"
        );
        assert_eq!(escort.state.velocity, Vec2::ZERO, "{case}");
        assert_eq!(escort.state.heading, session.player().heading, "{case}");
        let dudes: Vec<&Npc> = session
            .npcs()
            .iter()
            .filter(|npc| npc.id != fleet[0])
            .collect();
        assert_eq!(dudes.len(), 3, "{case}");
        assert!(
            dudes
                .iter()
                .all(|npc| (npc.ship, npc.govt) == (ShipId(129), Some(GovtId(140)))),
            "{case}"
        );
    }

    #[test]
    fn a_gate_exit_leaves_the_fight_and_the_scene_and_populates_the_system_afresh() {
        let catalog = busy(gated());
        let mut session = in_the_thick_of_it(at_gate(&catalog), &catalog);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Ok(SystemId(131))
        );
        assert_came_out_afresh(&session, "a hypergate");
        let catalog = busy(wormholes());
        let mut session = in_the_thick_of_it(at_wormhole(&catalog), &catalog);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut Scripted::rolling(&[0])),
            Ok(SystemId(131))
        );
        assert_came_out_afresh(&session, "a wormhole");
    }

    #[test]
    fn a_wormhole_back_into_the_system_it_went_in_still_populates_it_afresh() {
        let catalog = busy(wormholes());
        let mut session = at_wormhole(&catalog);
        session.populate(&catalog, &mut NeverFires);
        let before: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(before.len(), 2);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut Scripted::rolling(&[1])),
            Ok(SystemId(130))
        );
        let after: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(after.len(), 2);
        assert!(
            after.iter().all(|id| !before.contains(id)),
            "{before:?} {after:?}"
        );
    }

    /// One question asked of a [`Logged`] chance.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Draw {
        Fires(u8),
        Below(u32),
        Roll(u16),
    }

    /// A [`Chance`] that answers as [`NeverFires`] does and logs every
    /// question, in the order asked.
    #[derive(Debug, Default)]
    struct Logged(Vec<Draw>);

    impl Chance for Logged {
        fn fires(&mut self, percent: u8) -> bool {
            self.0.push(Draw::Fires(percent));
            NeverFires.fires(percent)
        }

        fn below(&mut self, n: u32) -> u32 {
            self.0.push(Draw::Below(n));
            NeverFires.below(n)
        }

        fn roll(&mut self, sides: u16) -> u16 {
            self.0.push(Draw::Roll(sides));
            NeverFires.roll(sides)
        }
    }

    /// The draws populating `system` of `catalog` asks, from a new
    /// pilot's session.
    fn population_draws(catalog: &FakePilotCatalog, system: i16) -> Vec<Draw> {
        let mut session = Session::start(catalog).expect("starts");
        session.pilot.system = SystemId(system);
        let mut chance = Logged::default();
        session.populate(catalog, &mut chance);
        assert!(!chance.0.is_empty(), "the population draws");
        chance.0
    }

    /// The surplus event (35 %, rolled each day) as `base`'s only one.
    fn with_event(base: FakePilotCatalog) -> FakePilotCatalog {
        FakePilotCatalog {
            disasters: surplus().disasters,
            ..base
        }
    }

    /// The escort-strength timer's seed, `Rand(30)`.
    const SEED: Draw = Draw::Below(30);

    #[test]
    fn a_jumps_days_draw_before_the_population_of_the_system_arrived_in() {
        let catalog = with_event(busy(catalog()));
        let mut session = Session::start(&catalog).expect("starts");
        let mut chance = Logged::default();
        jump_with(&mut session, &catalog, 131, &mut chance);
        // The seed first (@0x6c013), then the days, the roll table and the
        // population.
        let expected = [
            vec![SEED, Draw::Fires(35)],
            vec![Draw::Roll(100); crate::mission::ROLLED],
            population_draws(&catalog, 131),
        ]
        .concat();
        assert_eq!(chance.0, expected);
    }

    #[test]
    fn a_gate_like_a_jump_draws_its_days_before_the_population() {
        let catalog = with_event(busy(gated()));
        let mut session = at_gate(&catalog).with_gate_arrival(GateArrivalRule::LikeJump);
        let mut chance = Logged::default();
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut chance)
            .expect("enters");
        let expected = [vec![Draw::Fires(35), SEED], population_draws(&catalog, 131)].concat();
        assert_eq!(chance.0, expected);
    }

    #[test]
    fn a_gate_rolls_its_heading_before_the_population() {
        let catalog = busy(gated());
        let mut session = at_gate(&catalog);
        let mut chance = Logged::default();
        session
            .enter_hypergate(Some(SystemId(132)), &catalog, &mut chance)
            .expect("enters");
        let expected = [vec![Draw::Roll(360), SEED], population_draws(&catalog, 132)].concat();
        assert_eq!(chance.0, expected);
    }

    #[test]
    fn a_wormhole_rolls_its_exit_before_the_population() {
        let catalog = busy(wormholes());
        let mut session = at_wormhole(&catalog);
        let mut chance = Logged::default();
        session
            .enter_wormhole(&catalog, &mut chance)
            .expect("enters");
        let expected = [vec![Draw::Roll(2), SEED], population_draws(&catalog, 131)].concat();
        assert_eq!(chance.0, expected);
    }

    #[test]
    fn a_take_off_draws_the_seed_after_the_stays_days_and_before_the_population() {
        let catalog = with_event(busy(catalog()));
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        // Two escorts changed (a day before the wage), an outfit traded
        // and a ship bought (five days after it), and the take-off's day.
        session.stay.escorts_changed = 2;
        session.stay.outfit_traded = true;
        session.stay.ship_bought = true;
        let mut chance = Logged::default();
        session.take_off(&catalog, &mut chance).expect("takes off");
        let expected = [
            vec![Draw::Fires(35); 7],
            vec![SEED],
            population_draws(&catalog, 130),
        ]
        .concat();
        assert_eq!(chance.0, expected);
    }

    // The escort-strength timer.

    /// [`busy`] [`catalog`] with escort classes 150 (a wimpy trader of 60
    /// tons), 151 (a brave trader of 250) and 152 (a warship of 300), and
    /// `flët` 382, led by ship 160, and 383, led by ship 161, which link
    /// to no system.
    fn strong() -> FakePilotCatalog {
        let mut catalog = busy(catalog());
        catalog.ship_records.extend([
            holds_record(150, 60, 1),
            holds_record(151, 250, 2),
            holds_record(152, 300, 3),
            ship(160, FAST),
            ship(161, FAST),
        ]);
        let fleet = |id, lead| FleetRecord {
            id: FleetId(id),
            lead: Some(ShipId(lead)),
            escorts: Vec::new(),
            govt: None,
            link_syst: 0,
            appear_on: Test::default(),
        };
        catalog.fleets = vec![fleet(382, 160), fleet(383, 161)];
        catalog
    }

    /// [`strong`]'s session with an escort of each of `classes`, landed on
    /// planet 128 and taken off, the timer armed to 59 ([`NeverFires`]).
    fn escorted_take_off(catalog: &FakePilotCatalog, classes: &[i16]) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.pilot.escorts = classes.iter().map(|&id| fleet_escort(id)).collect();
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        session
            .take_off(catalog, &mut NeverFires)
            .expect("takes off");
        session
    }

    /// The AI types of `session`'s NPCs of ship `id`.
    fn flying_as(session: &Session, id: i16) -> Vec<AiType> {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.ship == ShipId(id))
            .map(|npc| npc.ai_type)
            .collect()
    }

    #[test]
    fn escorts_above_49_tons_bring_fleet_382_on_the_60th_tick_after_a_take_off() {
        let catalog = strong();
        let mut session = escorted_take_off(&catalog, &[150]);
        assert_eq!(flying_as(&session, 150).len(), 1, "the escort is out");
        for _ in 0..59 {
            session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        }
        assert_eq!(flying_as(&session, 160), []);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(flying_as(&session, 160), [AiType::Interceptor]);
        assert_eq!(flying_as(&session, 161), []);
    }

    #[test]
    fn escorts_above_199_tons_bring_fleet_383() {
        let catalog = strong();
        for classes in [&[151][..], &[150, 150, 150, 150]] {
            let mut session = escorted_take_off(&catalog, classes);
            for _ in 0..60 {
                session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
            }
            assert_eq!(
                flying_as(&session, 161),
                [AiType::Interceptor],
                "{classes:?}"
            );
            assert_eq!(flying_as(&session, 160), []);
        }
    }

    #[test]
    fn a_warship_escort_or_none_brings_nothing() {
        let catalog = strong();
        for classes in [&[152][..], &[]] {
            let mut session = escorted_take_off(&catalog, classes);
            for _ in 0..100 {
                session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
            }
            assert_eq!(flying_as(&session, 160), [], "{classes:?}");
            assert_eq!(flying_as(&session, 161), [], "{classes:?}");
        }
    }

    #[test]
    fn a_flight_never_taken_off_or_arrived_in_is_never_timed() {
        let catalog = strong();
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.escorts = vec![fleet_escort(151)];
        session.populate(&catalog, &mut NeverFires);
        for _ in 0..100 {
            session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        }
        assert_eq!(flying_as(&session, 161), []);
    }

    // The galaxy overlay.

    /// [`catalog`] with 134 at 131's place, (600, 0), listing 130 and 132
    /// and holding planet 141 at its centre: 131 heads the place.
    pub(super) fn alternates() -> FakePilotCatalog {
        let mut catalog = catalog();
        with_alternate(&mut catalog);
        catalog
    }

    /// Adds [`alternates`]' 134 to `catalog`, and makes every system on
    /// its map [`hideable`].
    pub(super) fn with_alternate(catalog: &mut FakePilotCatalog) {
        catalog.star_map.push(star(134, (600.0, 0.0), &[130, 132]));
        catalog.systems.push(SystemId(134));
        catalog
            .sites
            .push((SystemId(134), vec![planet(141, 0.0, 0.0)]));
        hideable(catalog);
    }

    /// Hides each of `systems` of a [`hideable`] `catalog` as play does: its
    /// hiding bit set, then the galaxy refreshed, as at a refresh point, so
    /// every later refresh keeps it hidden. Whether that changed anything.
    pub(super) fn hide(session: &mut Session, catalog: &FakePilotCatalog, systems: &[i16]) -> bool {
        set_hiding(session, catalog, systems, true)
    }

    /// Shows each of `systems` of a [`hideable`] `catalog` again, as
    /// [`hide`] hides it.
    pub(super) fn unhide(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        systems: &[i16],
    ) -> bool {
        set_hiding(session, catalog, systems, false)
    }

    /// Sets or clears each of `systems`' hiding bit, then refreshes the
    /// galaxy: whether that changed anything.
    fn set_hiding(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        systems: &[i16],
        hidden: bool,
    ) -> bool {
        for &id in systems {
            assert!(
                catalog
                    .visibility
                    .iter()
                    .any(|test| test.id == SystemId(id)),
                "sÿst {id} is hideable"
            );
            let bits = session.pilot.bits_mut();
            if hidden {
                bits.set(hiding_bit(id));
            } else {
                bits.clear(hiding_bit(id));
            }
        }
        session.refresh_galaxy(catalog)
    }

    /// Whether the player is in a system on the overlaid map.
    fn on_the_map(session: &Session) -> bool {
        session.star_map().position(session.system()).is_some()
    }

    #[test]
    fn with_no_override_the_galaxy_is_the_catalogs() {
        let catalog = alternates();
        let session = Session::start(&catalog).expect("starts");
        assert!(session.galaxy().overlay().is_empty());
        assert_eq!(*session.star_map(), StarMap::new(catalog.star_map.clone()));
        assert_eq!(session.course_shown(), []);
    }

    /// The session over `catalog` with `colocated_resolution` reading
    /// `source`.
    fn resolving(catalog: &FakePilotCatalog, source: RuleSource) -> Session {
        Session::start(catalog)
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::ColocatedResolution, source))
    }

    #[test]
    fn by_the_engine_a_place_of_two_visible_systems_is_the_lowest_one() {
        let catalog = alternates();
        let mut session = resolving(&catalog, RuleSource::Engine);
        assert_eq!(
            session
                .star_map()
                .jumps(SystemId(130), HyperlinkRule::Engine),
            ids(&[131])
        );
        assert_eq!(
            session.plot_course(SystemId(134)),
            Err(RouteError::Unreachable),
            "134 is shadowed by 131"
        );
        land_now(&mut session).expect("lands");
        session
            .relocate(&catalog, SystemId(134), StellarId(141))
            .expect("relocates");
        assert_eq!(session.system(), SystemId(131), "resolved to the lowest");
        assert!(session.galaxy().visible(SystemId(134)), "still visible");
    }

    #[test]
    fn by_the_other_reading_a_visible_system_at_a_shared_place_stands_for_itself() {
        let catalog = alternates();
        let mut session = resolving(&catalog, RuleSource::Bible);
        assert_eq!(
            session
                .star_map()
                .jumps(SystemId(130), HyperlinkRule::Engine),
            ids(&[131, 134])
        );
        assert_eq!(jump(&mut session, &catalog, 134), Some(SystemId(134)));
        assert_eq!(session.system(), SystemId(134), "and stays");
        let mut session = resolving(&catalog, RuleSource::Bible);
        land_now(&mut session).expect("lands");
        session
            .relocate(&catalog, SystemId(134), StellarId(141))
            .expect("relocates");
        assert_eq!(session.system(), SystemId(134));
    }

    #[test]
    fn a_hidden_system_leaves_the_hyperlinks_for_the_one_at_its_place() {
        let catalog = alternates();
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let mut session = resolving(&catalog, source);
            let jumps = session
                .star_map()
                .jumps(SystemId(130), HyperlinkRule::Engine);
            let both = ids(&[131, 134]);
            let lowest = ids(&[131]);
            assert_eq!(
                jumps,
                if source == RuleSource::Engine {
                    lowest
                } else {
                    both
                },
                "{source:?}"
            );
            assert!(hide(&mut session, &catalog, &[131]));
            assert_eq!(
                session
                    .star_map()
                    .jumps(SystemId(130), HyperlinkRule::Engine),
                ids(&[134]),
                "{source:?}"
            );
        }
        let mut session = resolving(&catalog, RuleSource::Bible);
        let jumps = |session: &Session| {
            session
                .star_map()
                .jumps(SystemId(130), HyperlinkRule::Engine)
        };
        assert_eq!(jumps(&session), ids(&[131, 134]));
        assert!(hide(&mut session, &catalog, &[131]));
        assert_eq!(jumps(&session), ids(&[134]), "on the next query");
        assert_eq!(
            session.star_map().links(),
            [
                (SystemId(130), SystemId(134)),
                (SystemId(132), SystemId(134))
            ]
        );
        assert!(!session.galaxy().visible(SystemId(131)));
    }

    #[test]
    fn a_hidden_system_cannot_be_plotted_to_or_hyper_selected() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        hide(&mut session, &catalog, &[131]);
        assert_eq!(session.plot_course(SystemId(131)), Err(RouteError::Unknown));
        assert_eq!(
            session.plot_course(SystemId(132)).map(<[_]>::to_vec),
            Ok(ids(&[134, 132]))
        );
        assert_eq!(session.select_next_system(), Some(SystemId(134)));
    }

    #[test]
    fn a_course_through_a_hidden_system_jumps_to_the_one_at_its_place() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.course(), ids(&[131, 132]));
        hide(&mut session, &catalog, &[131]);
        assert_eq!(session.course(), ids(&[131, 132]), "kept as saved");
        assert_eq!(session.course_shown(), ids(&[134, 132]));
        fly_out(&mut session);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(134)));
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(134))
        );
        assert_eq!(session.course(), ids(&[132]));
    }

    #[test]
    fn a_course_hop_with_no_system_at_its_place_is_no_destination_until_one_shows() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        hide(&mut session, &catalog, &[131, 134]);
        assert_eq!(session.course_shown(), [], "cut at the first hop");
        fly_out(&mut session);
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
        unhide(&mut session, &catalog, &[131]);
        assert_eq!(session.course_shown(), ids(&[131, 132]), "followed again");
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
    }

    /// [`alternates`] with two ships of düde 128 flying in 131, and none
    /// in 134.
    fn trafficked_alternates() -> FakePilotCatalog {
        let mut catalog = trafficked(131, 2, 1);
        with_alternate(&mut catalog);
        catalog
    }

    #[test]
    fn hiding_the_players_system_keeps_its_ships_and_draws_arrivals_from_the_new_one() {
        let catalog = trafficked_alternates();
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        assert_eq!(session.npcs().len(), 2);
        let npcs = session.npcs().to_vec();
        assert_eq!(session.traffic_ships(), [ShipId(129)]);
        hide(&mut session, &catalog, &[131]);
        assert_eq!(session.system(), SystemId(134));
        assert_eq!(session.npcs(), npcs, "every ship in the system stays");
        assert_eq!(session.traffic_ships(), [], "134's table");
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(session.npcs(), [], "the next entry is 134's");
    }

    #[test]
    fn by_the_bible_a_government_override_changes_the_record_and_landing_in_the_system() {
        let catalog = governed([None, None], Some(WANTED));
        let mut pilot = on_record(&catalog);
        pilot.stellar = Some(StellarId(128));
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(bible_records());
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.system_record(), 9);
        assert_eq!(land_now(&mut session), denied(128));
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_govt(SystemId(130), Some(Some(LAWFUL)));
        });
        assert_eq!(session.system_record(), 10);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_govt(SystemId(130), Some(None));
        });
        assert_eq!(session.system_record(), 0, "made independent");
    }

    #[test]
    fn by_the_engine_a_government_override_keeps_the_systems_record() {
        let catalog = governed([None, None], Some(WANTED));
        let mut pilot = on_record(&catalog);
        pilot.stellar = Some(StellarId(128));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.system_record(), 9);
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_govt(SystemId(130), Some(Some(LAWFUL)));
        });
        assert_eq!(session.system_record(), 9, "the system's own record");
        assert_eq!(land_now(&mut session), denied(128));
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_govt(SystemId(130), Some(None));
        });
        assert_eq!(session.system_record(), 9, "made independent, kept");
    }

    #[test]
    fn by_the_bible_a_stellars_government_override_changes_landing_there() {
        let catalog = governed([None, None], Some(WANTED));
        let mut pilot = on_record(&catalog);
        pilot.stellar = Some(StellarId(128));
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(bible_records());
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(land_now(&mut session), denied(128));
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_stellar_govt(StellarId(128), Some(Some(LAWFUL)));
        });
        assert_eq!(session.sites[0].govt, Some(LAWFUL));
        assert_eq!(session.sites[1].govt, None, "only that stellar");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_stellar_govt(StellarId(128), Some(Some(WANTED)));
        });
        assert_eq!(land_now(&mut session), denied(128), "the record with it");
    }

    #[test]
    fn a_pilot_flown_with_a_stellar_override_reads_its_stellars_through_it() {
        let catalog = governed([Some(LAWFUL), None], None);
        let mut pilot = on_record(&catalog);
        pilot
            .galaxy
            .set_stellar_govt(StellarId(128), Some(Some(WANTED)));
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.sites[0].govt, Some(WANTED));
    }

    #[test]
    fn a_stellars_government_override_changes_what_its_outfitter_requires() {
        let mut catalog = outfitting();
        catalog.outfits[0].require = 0x2;
        catalog.outfits[0].require_govt = 128;
        let mut session = outfitted(&catalog);
        let buyable = |session: &mut Session| {
            session
                .outfitter(&mut NeverFires)
                .expect("an outfitter")
                .can(buy(SPEED))
                .is_ok()
        };
        assert!(!buyable(&mut session), "required by its government");
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_stellar_govt(StellarId(128), Some(None));
        });
        assert!(buyable(&mut session), "independent now");
    }

    #[test]
    fn a_government_override_brings_in_the_fleet_linked_to_it_at_the_next_population() {
        let catalog = fleeted();
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut linked_fleet_draws(3));
        assert_eq!(fleet_of(&session), [], "linked to 141");
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_govt(SystemId(130), Some(Some(GovtId(141))));
        });
        session.populate(&catalog, &mut linked_fleet_draws(3));
        let lead = session.npcs()[0].id.0;
        assert_eq!(fleet_of(&session), a_fleet_led_by(lead));
    }

    #[test]
    fn a_government_override_replaces_the_table_arrivals_are_drawn_from_at_once() {
        let mut catalog = fleeted();
        catalog.fleets[1].escorts[0].ship = ShipId(131);
        catalog.ship_records.push(ship(131, FAST));
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut linked_fleet_draws(2));
        let npcs = session.npcs().to_vec();
        assert!(!session.traffic_ships().contains(&ShipId(131)));
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_govt(SystemId(130), Some(Some(GovtId(141))));
        });
        assert!(
            session.traffic_ships().contains(&ShipId(131)),
            "141's fleet"
        );
        assert!(!session.traffic_ships().contains(&ShipId(130)));
        assert_eq!(session.npcs(), npcs, "the ships present stay");
    }

    #[test]
    fn a_landed_move_into_a_system_then_hidden_takes_off_into_the_one_at_its_place() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
        hide(&mut session, &catalog, &[131]);
        assert_eq!(session.system(), SystemId(134));
        assert_eq!(session.sites.len(), 2, "130's, until the take-off");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.sites, [planet(141, 0.0, 0.0)]);
    }

    /// [`alternates`]' session landed on 128 in 130, then moved by a
    /// landed `M131`: 131's stellars wait for the take-off.
    fn moved_while_landed(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        let _ = session.settle_script(catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
        assert!(session.next_sites.is_some(), "deferred");
        session.assert_docked_among_shown();
        session
    }

    #[test]
    fn a_stellar_override_in_the_system_shown_after_a_landed_move_shows_at_once() {
        let catalog = alternates();
        let mut session = moved_while_landed(&catalog);
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_stellar_govt(StellarId(128), Some(Some(LAWFUL)));
        });
        session.assert_docked_among_shown();
        assert_eq!(session.sites[0].id, StellarId(128), "130's, still shown");
        assert_eq!(session.sites[0].govt, Some(LAWFUL));
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.sites, [planet(140, 0.0, 0.0)], "131's own");
    }

    #[test]
    fn a_stellar_override_in_the_system_moved_to_while_landed_holds_at_the_take_off() {
        let catalog = alternates();
        let mut session = moved_while_landed(&catalog);
        let shown = session.sites.clone();
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_stellar_govt(StellarId(140), Some(Some(LAWFUL)));
        });
        assert_eq!(session.sites, shown, "130's, untouched");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(
            session.sites,
            [LandingSite {
                govt: Some(LAWFUL),
                ..planet(140, 0.0, 0.0)
            }]
        );
    }

    #[test]
    fn the_npcs_see_a_government_override_from_the_next_traffic_tick() {
        let mut catalog = armed();
        catalog.star_map[0].govt = Some(GovtId(150));
        catalog.govts = vec![crate::testkit::govt(150), crate::testkit::govt(151)];
        let mut session = facing_an_npc(&catalog, 180);
        let spy = Spy::default();
        session.tick_traffic(&catalog, &spy, &mut NeverFires);
        assert_eq!(spy.seen.take()[0].1, Some(GovtId(150)));
        let npcs = session.npcs().len();
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_govt(SystemId(130), Some(Some(GovtId(151))));
        });
        assert_eq!(session.npcs().len(), npcs, "the ships present stay");
        session.tick_traffic(&catalog, &spy, &mut NeverFires);
        assert_eq!(spy.seen.take()[0].1, Some(GovtId(151)));
    }

    #[test]
    fn a_change_to_the_galaxy_is_told_once_and_makes_a_save_due_and_a_no_op_neither() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        session.take_events();
        assert!(hide(&mut session, &catalog, &[133]));
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
        assert!(session.take_save_due());
        assert!(!hide(&mut session, &catalog, &[133]), "already hidden");
        assert!(!session.change_galaxy(&catalog, |_| {}));
        assert_eq!(session.take_events(), []);
        assert!(!session.take_save_due());
        assert!(!session.pilot().galaxy.is_empty(), "saved on the pilot");
        assert_eq!(*session.galaxy().overlay(), session.pilot().galaxy);
        assert!(session.change_galaxy(&catalog, |overlay| {
            overlay.set_govt(SystemId(130), Some(Some(GovtId(140))));
        }));
        assert_eq!(
            session.take_events(),
            [SessionEvent::GalaxyChanged],
            "a government alone"
        );
        assert!(session.take_save_due());
    }

    #[test]
    fn a_player_whose_system_is_hidden_moves_to_the_one_at_its_place_as_it_is() {
        let catalog = alternates();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.system = SystemId(131);
        pilot.stellar = Some(StellarId(140));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.landed(), Some(StellarId(140)));
        session.plot_course(SystemId(132)).expect("a route");
        let (player, course, escorts, date) = (
            *session.player(),
            session.course().to_vec(),
            session.pilot().escorts().to_vec(),
            session.date(),
        );
        // The load's rebuild shared 131's exploration with 134, its place's
        // other system, by the engine (see the `explored` module); forgotten
        // here, so the move is seen to explore nothing.
        assert!(session.pilot.explored.remove(&SystemId(134)));
        let explored: Vec<SystemId> = session.pilot().explored().collect();
        session.take_events();
        session.drop_raised();
        session.take_messages();
        assert!(hide(&mut session, &catalog, &[131]));
        session.assert_docked_among_shown();
        assert_eq!(session.system(), SystemId(134));
        assert_eq!(*session.player(), player);
        assert_eq!(session.landed(), Some(StellarId(140)), "still landed");
        assert_eq!(session.pilot().stellar, Some(StellarId(140)));
        assert_eq!(session.course(), course);
        assert_eq!(session.pilot().escorts(), escorts);
        assert_eq!(session.date(), date, "no day");
        assert!(
            session.pilot().explored().eq(explored.iter().copied()),
            "explores nothing"
        );
        assert_eq!(session.sites, [planet(140, 0.0, 0.0)], "its spaceport's");
        assert_eq!(
            session.take_events(),
            [SessionEvent::GalaxyChanged],
            "the scene waits for the take-off"
        );
        assert_eq!(session.story_raised(), [], "no story event");
        assert_eq!(session.take_messages(), [], "no arrival");
        assert!(on_the_map(&session));
    }

    /// [`alternates`]' pilot landed at 140 in 131.
    fn landed_at_140() -> Pilot {
        let mut pilot = Pilot::new(&alternates(), "Ada").expect("starts");
        pilot.system = SystemId(131);
        pilot.stellar = Some(StellarId(140));
        pilot
    }

    /// Takes off from `session`, and checks it flies in 134 among its
    /// stellars, the scene told changed.
    fn takes_off_into_134(session: &mut Session) {
        session.assert_docked_among_shown();
        session.take_events();
        assert_eq!(
            session.take_off(&NoTraffic, &mut NeverFires),
            Some(StellarId(140))
        );
        assert_eq!(session.system(), SystemId(134));
        assert_eq!(session.sites, [planet(141, 0.0, 0.0)]);
        assert_eq!(session.take_events(), [SessionEvent::SystemChanged]);
        assert!(on_the_map(session));
    }

    #[test]
    fn a_landed_player_moved_off_a_hidden_system_keeps_its_spaceport_until_the_take_off() {
        let catalog = alternates();
        let mut session = Session::fly(&catalog, landed_at_140()).expect("flies");
        assert_eq!(session.recharge(), Err(RechargeRefusal::Full));
        hide(&mut session, &catalog, &[131]);
        session.assert_docked_among_shown();
        assert_eq!(session.landed(), Some(StellarId(140)));
        assert_eq!(
            session.recharge(),
            Err(RechargeRefusal::Full),
            "fuel is still sold here"
        );
        assert!(session.transact(|_| {}), "still in the spaceport");
        takes_off_into_134(&mut session);
    }

    #[test]
    fn a_save_made_landed_after_the_system_is_hidden_reloads_landed() {
        let catalog = alternates();
        let mut session = Session::fly(&catalog, landed_at_140()).expect("flies");
        hide(&mut session, &catalog, &[131]);
        let pilot = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.assert_docked_among_shown();
        assert_eq!(session.system(), SystemId(134));
        assert_eq!(session.landed(), Some(StellarId(140)));
        assert_eq!(session.sites, [planet(140, 0.0, 0.0)]);
        assert_eq!(session.recharge(), Err(RechargeRefusal::Full));
        takes_off_into_134(&mut session);
    }

    #[test]
    fn a_pilot_flown_landed_in_a_hidden_system_stays_landed_until_the_take_off() {
        let catalog = alternates();
        let mut pilot = landed_at_140();
        // Saved while 131 is hidden by its `Visibility`: the load's refresh
        // keeps it hidden.
        pilot.bits_mut().set(hiding_bit(131));
        pilot.galaxy.set_visible(SystemId(131), Some(false));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert!(!session.galaxy().visible(SystemId(131)));
        session.assert_docked_among_shown();
        assert_eq!(session.system(), SystemId(134));
        assert_eq!(session.landed(), Some(StellarId(140)));
        assert_eq!(session.pilot().stellar, Some(StellarId(140)));
        assert_eq!(session.sites, [planet(140, 0.0, 0.0)]);
        assert_eq!(session.take_events(), [], "silently");
        takes_off_into_134(&mut session);
    }

    #[test]
    fn a_pilot_saved_landed_in_a_system_its_load_hides_stays_landed_until_the_take_off() {
        let catalog = alternates();
        let mut pilot = landed_at_140();
        // Saved while 131 was shown: only the load's refresh hides it, so
        // the refresh, not the load's first look at the saved flags, moves
        // the player.
        pilot.bits_mut().set(hiding_bit(131));
        assert_eq!(pilot.galaxy.system(SystemId(131)).visible, None);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert!(!session.galaxy().visible(SystemId(131)));
        session.assert_docked_among_shown();
        assert_eq!(session.system(), SystemId(134), "the one at its place");
        assert_eq!(session.landed(), Some(StellarId(140)), "still landed");
        assert_eq!(session.pilot().stellar, Some(StellarId(140)));
        assert_eq!(session.sites, [planet(140, 0.0, 0.0)], "its spaceport's");
        assert_eq!(session.take_events(), [], "silently");
        assert!(!session.take_save_due(), "nothing to save");
        takes_off_into_134(&mut session);
    }

    #[test]
    fn with_no_system_visible_at_its_place_the_player_goes_to_the_head_made_visible() {
        let catalog = alternates();
        let mut session = resolving(&catalog, RuleSource::Bible);
        jump(&mut session, &catalog, 134);
        session.take_events();
        assert!(hide(&mut session, &catalog, &[131, 134]));
        assert_eq!(session.system(), SystemId(131));
        assert!(session.galaxy().visible(SystemId(131)));
        assert!(!session.galaxy().visible(SystemId(134)));
        assert_eq!(
            session.pilot().galaxy.system(SystemId(131)).visible,
            Some(true)
        );
        assert_eq!(
            session.take_events(),
            [SessionEvent::SystemChanged, SessionEvent::GalaxyChanged]
        );
        assert!(on_the_map(&session));
    }

    #[test]
    fn by_the_engine_a_player_in_a_shadowed_system_goes_to_the_head_made_visible_once_both_hide() {
        let catalog = alternates();
        let mut pilot = Pilot::new(&catalog, "").expect("starts");
        pilot.system = SystemId(134);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.system(), SystemId(134), "shadowed, not hidden");
        session.take_events();
        assert!(hide(&mut session, &catalog, &[131, 134]));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(
            session.pilot().galaxy.system(SystemId(131)).visible,
            Some(true)
        );
        assert_eq!(
            session.take_events(),
            [SessionEvent::SystemChanged, SessionEvent::GalaxyChanged]
        );
    }

    #[test]
    fn a_player_in_the_head_with_none_visible_stays_and_its_system_is_shown() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        session.take_events();
        assert!(hide(&mut session, &catalog, &[131, 134]));
        assert_eq!(session.system(), SystemId(131));
        assert!(session.galaxy().visible(SystemId(131)));
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
    }

    #[test]
    fn a_pilot_loaded_in_a_hidden_system_flies_from_the_one_at_its_place() {
        let catalog = alternates();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.system = SystemId(131);
        // Saved while 131 is hidden by its `Visibility`: the load's refresh
        // keeps it hidden.
        pilot.bits_mut().set(hiding_bit(131));
        pilot.galaxy.set_visible(SystemId(131), Some(false));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert!(!session.galaxy().visible(SystemId(131)));
        assert_eq!(session.system(), SystemId(134));
        assert_eq!(session.sites, [planet(141, 0.0, 0.0)]);
        assert_eq!(session.take_events(), [], "silently");
        assert!(on_the_map(&session));
    }

    #[test]
    fn a_jump_whose_destination_is_hidden_on_the_way_arrives_at_the_one_at_its_place() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
        hide(&mut session, &catalog, &[131]);
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(134))
        );
        assert_eq!(session.system(), SystemId(134));
        assert_eq!(session.sites, [planet(141, 0.0, 0.0)]);
        assert_eq!(session.course(), [], "the hop made");
        assert!(on_the_map(&session));
    }

    #[test]
    fn a_jump_into_a_system_hidden_on_the_way_explores_the_one_it_arrives_in() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        hide(&mut session, &catalog, &[131]);
        session.drop_raised();
        session.arrive(&catalog, &mut NeverFires);
        let explored: Vec<SystemId> = session
            .story_raised()
            .into_iter()
            .filter_map(|event| match event {
                StoryEvent::Explored { system } => Some(system),
                _ => None,
            })
            .collect();
        assert_eq!(explored, [SystemId(134)], "no event for the head");
        assert!(session.pilot().has_explored(SystemId(134)));
        assert!(
            session.pilot().has_explored(SystemId(131)),
            "131, its place's head, by the engine (see the `explored` module)"
        );
    }

    /// The `Explored` events raised and not yet settled.
    fn explored_raised(session: &Session) -> Vec<SystemId> {
        session
            .story_raised()
            .into_iter()
            .filter_map(|event| match event {
                StoryEvent::Explored { system } => Some(system),
                _ => None,
            })
            .collect()
    }

    /// [`chained`] of `mod_val` with 135 at 131's place, listing 132,
    /// every system [`hideable`].
    fn chained_alternates(mod_val: i16) -> FakePilotCatalog {
        let mut catalog = chained(catalog(), mod_val);
        catalog.star_map.push(star(135, (600.0, 0.0), &[132]));
        hideable(&mut catalog);
        catalog
    }

    #[test]
    fn a_multi_jump_passes_a_hidden_hop_by_the_system_at_its_place() {
        let catalog = chained_alternates(2);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        hide(&mut session, &catalog, &[131]);
        assert_eq!(begin_jump_now(&mut session), Ok(SystemId(135)));
        session.drop_raised();
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        assert_eq!(session.course(), ids(&[134]));
        assert_eq!(explored_raised(&session), ids(&[135, 132]), "135 passed");
        assert!(on_the_map(&session));
    }

    #[test]
    fn a_multi_jump_stops_before_a_hop_with_no_system_visible_at_its_place() {
        let catalog = chained_alternates(3);
        for source in RuleSource::ALL {
            let mut session = bound_for_134(&catalog, MultiJumpRule::default())
                .with_rules(Rulebook::default().with_override(RuleKey::RouteRewrite, source));
            hide(&mut session, &catalog, &[132]);
            assert_eq!(begin_jump_now(&mut session), Ok(SystemId(131)));
            session.drop_raised();
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(131))
            );
            // The arrival is a rebuild point, whose rewrite cuts the course
            // at 132, with no system visible at its place, by the engine
            // (@0x6c203, @0x32aed-0x32b02).
            let kept = match source {
                RuleSource::Engine => ids(&[]),
                RuleSource::Bible => ids(&[132, 134]),
            };
            assert_eq!(session.course(), kept, "{source:?}");
            assert_eq!(explored_raised(&session), ids(&[131]));
        }
    }

    #[test]
    fn a_hypergate_into_a_hidden_system_comes_out_at_the_one_at_its_place() {
        let mut catalog = gated();
        with_alternate(&mut catalog);
        let mut session = at_gate(&catalog);
        hide(&mut session, &catalog, &[131]);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Err(GateRefusal::Cancelled),
            "hidden 131 is never what a link resolves to by place"
        );
        let mut session = at_gate(&catalog);
        hide(&mut session, &catalog, &[131]);
        assert_eq!(
            session.enter_hypergate(Some(SystemId(134)), &catalog, &mut NeverFires),
            Ok(SystemId(134))
        );
        assert_eq!(session.system(), SystemId(134));
        assert_eq!(session.sites, [planet(141, 0.0, 0.0)]);
        assert!(on_the_map(&session));
    }

    #[test]
    fn a_hypergate_offers_and_takes_the_system_standing_for_a_hidden_listers_gate() {
        let mut catalog = gated();
        with_alternate(&mut catalog);
        let mut session = at_gate(&catalog);
        hide(&mut session, &catalog, &[131]);
        assert_eq!(
            session.open_hypergate(&catalog),
            Ok(ids(&[134, 132])),
            "134 stands for 131"
        );
        assert_eq!(
            session.enter_hypergate(Some(SystemId(134)), &catalog, &mut NeverFires),
            Ok(SystemId(134))
        );
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0), "at 310");
        assert!(on_the_map(&session));
    }

    #[test]
    fn a_hypergate_neither_offers_nor_takes_a_link_with_no_system_visible_at_its_place() {
        let mut catalog = gated();
        with_alternate(&mut catalog);
        let mut session = at_gate(&catalog);
        hide(&mut session, &catalog, &[131, 134]);
        assert_eq!(session.open_hypergate(&catalog), Ok(ids(&[132])));
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Err(GateRefusal::Cancelled)
        );
    }

    #[test]
    fn a_wormhole_listed_only_by_a_hidden_system_is_no_exit() {
        let mut catalog = wormholes();
        with_alternate(&mut catalog);
        let mut session = at_wormhole(&catalog);
        hide(&mut session, &catalog, &[131]);
        session.take_messages();
        assert_eq!(
            session.enter_wormhole(&catalog, &mut NeverFires),
            Err(GateRefusal::NoExit),
            "410 has no owner, and 420 is linked (@0x6410e-0x64176)"
        );
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.take_messages(), []);
    }

    #[test]
    fn a_hypergate_offers_a_links_owner_not_a_hidden_lower_lister() {
        let mut catalog = gated();
        with_alternate(&mut catalog);
        catalog.gate_listers = vec![
            (SystemId(131), StellarId(310)),
            (SystemId(133), StellarId(310)),
            (SystemId(132), StellarId(320)),
        ];
        let mut session = at_gate(&catalog);
        hide(&mut session, &catalog, &[131]);
        assert_eq!(
            session.open_hypergate(&catalog),
            Ok(ids(&[133, 132])),
            "310's owner is 133, not 131 by 134"
        );
        assert_eq!(
            session.enter_hypergate(Some(SystemId(133)), &catalog, &mut NeverFires),
            Ok(SystemId(133))
        );
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0), "at 310");
    }

    #[test]
    fn a_hypergate_offers_a_links_owner_not_a_shadowed_lower_lister() {
        let mut catalog = gated();
        with_alternate(&mut catalog);
        catalog.star_map.push(star(135, (-600.0, 600.0), &[]));
        catalog.gate_listers = vec![
            (SystemId(134), StellarId(310)),
            (SystemId(135), StellarId(310)),
            (SystemId(132), StellarId(320)),
        ];
        let mut session = at_gate(&catalog);
        assert_eq!(
            session.open_hypergate(&catalog),
            Ok(ids(&[135, 132])),
            "310's owner is 135, not 134, visible but shadowed by 131"
        );
        assert_eq!(
            session.enter_hypergate(Some(SystemId(135)), &catalog, &mut NeverFires),
            Ok(SystemId(135))
        );
        assert_eq!(session.system(), SystemId(135));
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0), "at 310");
    }

    #[test]
    fn a_hypergate_link_with_no_owner_resolves_through_its_lowest_visible_lister_by_place() {
        let mut catalog = gated();
        with_alternate(&mut catalog);
        catalog.gate_listers = vec![
            (SystemId(134), StellarId(310)),
            (SystemId(132), StellarId(320)),
        ];
        let mut session = at_gate(&catalog);
        assert_eq!(
            session.open_hypergate(&catalog),
            Ok(ids(&[131, 132])),
            "310 is listed only by 134, shadowed by 131: unowned, found in 134, offered as 131"
        );
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires),
            Ok(SystemId(131))
        );
        assert_eq!(session.player().position, Vec2::new(100.0, 200.0), "at 310");
    }

    #[test]
    fn a_wormhole_comes_out_in_its_exits_owner() {
        let mut catalog = wormholes();
        with_alternate(&mut catalog);
        catalog.gate_listers = vec![
            (SystemId(131), StellarId(410)),
            (SystemId(133), StellarId(410)),
            (SystemId(132), StellarId(420)),
        ];
        let mut session = at_wormhole(&catalog);
        hide(&mut session, &catalog, &[131]);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut Scripted::rolling(&[0])),
            Ok(SystemId(133)),
            "410's owner, not 134 standing for its lowest lister"
        );
        assert_eq!(session.player().position, Vec2::new(-300.0, 40.0));
    }

    /// [`wormholes`] over [`alternates`]' 134, with the ship starting in
    /// 131 rather than 130, over wormhole 400 of links `links`, and
    /// wormhole 430 at (70, -90) listed by 131 alone; 410 is 130's.
    fn rescued_wormholes(links: &[i16]) -> FakePilotCatalog {
        let mut catalog = FakePilotCatalog {
            character: starting([Some(131), None, None, None]).character,
            ..wormholes()
        };
        with_alternate(&mut catalog);
        catalog.sites = vec![
            (SystemId(130), vec![planet(128, 30.0, -40.0)]),
            (SystemId(131), vec![gate_landing(400, 0x2200)]),
        ];
        catalog.gates[0] = gate_site(400, (0.0, 0.0), 0x2200, links, 120);
        catalog
            .gates
            .push(gate_site(430, (70.0, -90.0), WORMHOLE, &[], 0));
        catalog.gate_listers = vec![
            (SystemId(130), StellarId(410)),
            (SystemId(132), StellarId(420)),
            (SystemId(131), StellarId(430)),
        ];
        catalog
    }

    /// `catalog`'s session with 131 and 134 hidden, so the refresh moves
    /// the player into 131 made visible, the chain head at their place,
    /// which owns nothing (@0x998f1-0x99a36): 430 is held only by the
    /// rescue ([`OverlaidGalaxy::holder`]). Then L pressed twice over the
    /// wormhole.
    fn at_rescued_wormhole(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        hide(&mut session, catalog, &[131, 134]);
        assert_eq!(session.system(), SystemId(131));
        let listers = [SystemId(131)];
        assert_eq!(session.galaxy().owner(&listers), None, "made visible");
        assert_eq!(
            session.galaxy().holder(&listers, SystemId(131)),
            Some(SystemId(131)),
            "rescued"
        );
        session.land().expect("clearance");
        assert_eq!(
            session.land(),
            Ok(LandPress::AtGate {
                stellar: StellarId(400),
                kind: GateKind::Wormhole,
            })
        );
        session
    }

    #[test]
    fn a_wormhole_linked_to_one_only_the_players_ownerless_system_lists_comes_out_there() {
        let catalog = rescued_wormholes(&[430]);
        let mut session = at_rescued_wormhole(&catalog);
        let mut chance = Scripted::rolling(&[0]);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut chance),
            Ok(SystemId(131)),
            "430 has no owner: the rescue holds it in the player's 131"
        );
        assert_eq!(chance.sides_asked, [1]);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.player().position, Vec2::new(70.0, -90.0), "at 430");
    }

    #[test]
    fn an_unlinked_wormhole_held_only_by_the_rescue_is_no_candidate() {
        let catalog = rescued_wormholes(&[]);
        let mut session = at_rescued_wormhole(&catalog);
        let mut chance = Scripted::rolling(&[0]);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut chance),
            Ok(SystemId(130))
        );
        assert_eq!(
            chance.sides_asked,
            [2],
            "410 in 130 and 420 in 132: 430, held by the player's 131, is dropped (@0x6410e-0x64176)"
        );
        assert_eq!(session.player().position, Vec2::new(-300.0, 40.0), "at 410");
    }

    #[test]
    fn m_into_a_hidden_system_arrives_at_the_one_at_its_place_and_reads_its_stellars() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        hide(&mut session, &catalog, &[131]);
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        let settled = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(settled.moved, Some(SystemId(134)));
        assert_eq!(session.system(), SystemId(134));
        assert_eq!(session.sites, [planet(141, 0.0, 0.0)]);
        assert!(on_the_map(&session));
    }

    #[test]
    fn a_relocation_into_a_hidden_system_arrives_at_the_one_at_its_place() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        hide(&mut session, &catalog, &[131]);
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("relocates");
        session.assert_docked_among_shown();
        assert_eq!(session.system(), SystemId(134));
        assert_eq!(session.landed(), Some(StellarId(140)), "docked as asked");
        assert_eq!(session.pilot().stellar, Some(StellarId(140)));
        assert_eq!(session.sites, [planet(140, 0.0, 0.0)], "its spaceport's");
        assert_eq!(session.recharge(), Err(RechargeRefusal::Full));
        assert!(on_the_map(&session));
        takes_off_into_134(&mut session);
    }

    #[test]
    fn an_arrival_where_none_is_visible_shows_the_head_and_tells_it_once() {
        let catalog = alternates();
        let mut session = Session::start(&catalog).expect("starts");
        hide(&mut session, &catalog, &[131, 134]);
        session.take_events();
        session.take_save_due();
        session.run_set(&SetExpr::parse("M134").expect("parses"), &mut NeverFires);
        let settled = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(settled.moved, Some(SystemId(131)));
        assert_eq!(session.system(), SystemId(131));
        assert!(session.galaxy().visible(SystemId(131)));
        let events = session.take_events();
        assert_eq!(
            events
                .iter()
                .filter(|&&event| event == SessionEvent::GalaxyChanged)
                .count(),
            1,
            "{events:?}"
        );
        assert!(session.take_save_due());
        assert!(on_the_map(&session));
    }
}
