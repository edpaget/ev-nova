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
//! Landing, taking off and each change made in the spaceport
//! ([`Session::transact`]) make a save due ([`Session::take_save_due`]):
//! whoever saves the pilot takes it after each input and saves then.
//! Arriving in a system explores it.
//!
//! The ship lands on a stellar of its system, on the land key's second
//! press, when the [`landing`](crate::landing) rules allow it, which read
//! the pilot's legal record with the stellar's government, or with its
//! system's on the star map when the stellar has none: docked, it rests
//! at the stellar's centre and ticks move nothing until it takes off
//! again, from the same place. A landed ship cannot jump, nor a jumping
//! one land.
//!
//! The player plots a course to a system on the star map, read once when
//! the session starts: the fewest jumps along the hyperlinks, which the
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
//! days or each hop's. In flight, fuel regenerates each tick at the rate the ship
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
//! target ([`Session::select_next_stellar`]), in the order the
//! [`navigation`](crate::navigation) rule gives, or with the land key
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
//! refused one changes nothing.
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
//! stopping, landing, taking off, a jump beginning and ending), which the
//! audio side drains with [`Session::take_sounds`]. A refused landing or
//! jump emits nothing.
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
//! with it, and answering the fight's strikes since the last traffic
//! tick; it stands still while the player is landed or jumping, as the
//! player's own ship does.
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
//! player is landed or jumping, and its shots and beams are gone once the
//! player lands or arrives elsewhere. An NPC destroyed is taken out of the
//! system. The player's ship, once it is not intact, ignores the controls
//! and drifts, cannot land or jump, and once destroyed stays where it is.
//!
//! The player's crimes change its legal records as the fight's
//! [`LegalCode`] says: disabling an NPC, and breaking one up (which is
//! disabling it too when it was intact), each against the NPC's
//! government. A mere hit is no crime, as in the original.
//!
//! The player boards its target once it is disabled
//! ([`Session::board`]), as the [`board`](crate::board) rules say: the
//! boarding is the [`Crime::Board`] against the target's government, and
//! the NPCs come to its help as for a hit. Then the player plunders it
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
//! entering a system resets, or keeps, as
//! [`Session::with_escort_orders`] says. A ship captured joins the fleet
//! where it is, and so does a person hailed that offers to join
//! (`person_join`'s other reading), flying as itself in every system; an
//! escort disabled or destroyed leaves it, and one hailed may be
//! released. Each change to the fleet makes a save due.
//!
//! The player's fighter bays launch fighters into the fleet, which Return
//! to Hangar brings back to dock, a round of their bay again; an NPC
//! carrier launches its own while it attacks (see [`bay`](crate::bay)).
//! What a fighter launched does first follows
//! [`Session::with_fighter_launch`]; what becomes of the fighters out as
//! the player jumps or lands follows [`Session::with_fighter_recall`],
//! and the fighters abandoned are told ([`Session::take_fighter_notes`]).
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
//! each day of a jump, and for a day at each take-off as
//! [`Session::with_take_off_pay`] says; one left unpaid defects
//! ([`Session::take_pay_notes`]). Which wage it is paid follows
//! [`Session::with_escort_wage`], and hailing it shows it.
//!
//! The pilot's control bits are read with [`Session::control_bit`] and
//! set or cleared with [`Session::set_control_bit`], which makes a save
//! due. A set expression runs on the session ([`Session::run_set`]),
//! writing bits, with its other operators handled as
//! [`Session::with_set_ops`] says: by default [`nova_set_ops`], which
//! grants (`G`) and removes (`D`) outfits and explores systems (`X`);
//! one nothing handles is skipped and told once
//! ([`Session::take_script_notes`]).
//!
//! Buying an outfit, a boarding grant and `G` share one grant path, the
//! original's: a map explores, a clean-record outfit cleans the legal
//! record and a paint paints the ship instead of being added, as
//! [`Session::with_outfit_rules`] says where the rules are disputed (see
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
//! quote as [`Session::with_comm_quote`] says, and in flight its hail
//! quote on its trigger ([`Session::tick_quotes`],
//! [`Session::take_quotes`]).
//!
//! The player targets an NPC ([`Session::select_target`]), the nearest,
//! the nearest threat or the next in turn as the
//! [`targeting`](crate::targeting) rules say, and
//! fires its primary weapons and the secondary selected
//! ([`Session::hold_fire`], [`Session::select_secondary`]): the first the
//! ship carries to start with, kept through a refit while the ship still
//! carries it. The target is let go once it starts breaking up, is
//! destroyed, lands or jumps out, when the system is populated afresh,
//! and when the player lands; it is kept through a jump, and gone on
//! arrival with the last system's traffic. Neither the target nor the
//! secondary is saved.
//!
//! jump emits nothing. The fight's [`CombatEvent`]s are drained with
//! [`Session::take_combat_events`], and the [`SimDiagnostic`]s about game
//! data the simulation does not handle yet, each once a session, with
//! [`Session::take_diagnostics`].

mod control;
mod edit;
mod escorts;
mod fighters;
mod hail;
mod hire;
mod hooks;
mod outfits;
mod persons;

pub use control::nova_set_ops;
pub use hooks::HookRules;

pub use edit::RelocateRefusal;
pub use persons::PersonQuote;

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use self::hooks::ShipHook;
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
    CombatCatalog, DateAffixes, GateSite, GovtId, LandingSite, OutfitId, OutfitRecord, PersonId,
    PilotCatalog, ShipId, ShipRecord, StartError, StellarId, SystemId, TrafficCatalog, WeaponId,
};
use crate::chance::Chance;
use crate::combat::armament::{
    Armament, Arsenal, OutfitRounds, Trigger, next_secondary, outfit_rounds,
};
use crate::combat::beam::Beam;
use crate::combat::hull::{Condition, HullSpec};
use crate::combat::projectile::Shot;
use crate::combat::report::SimDiagnostic;
use crate::combat::weapon::Ammo;
use crate::combat::{Combat, CombatEvent, Downed, Fighter, Rules, ShipRef, Strike};
use crate::control::{ControlBits, NovaBits, ScriptNote, SetOpKind, SetRegistry};
use crate::date::{self, GameDate};
use crate::escort::EscortDuty;
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
    HyperSelectRule, HyperlinkRule, JUMP_FUEL, JumpReadiness, JumpRefusal, JumpZoneRule,
    MultiJumpRule, RouteError, StarMap, arrival, check_jump, hops_per_jump, jump_bearing,
    jump_zone, next_hyper_destination,
};
use crate::landing::{LandOutcome, LandingRefusal, land_or_select};
use crate::legal::{self, Crime, LegalCode};
use crate::market::{self, Direction, Good, Goods, Market, Order, TradeRefusal};
use crate::message::SimMessage;
use crate::navigation::next_stellar;
use crate::outfit_effects::OutfitRules;
use crate::outfitter::{
    self, OutfitFlags, OutfitOrder, OutfitRefusal, Outfitter, Shop, outfit_mods,
};
use crate::person::{NovaPersons, PersonRules, PersonWorld};
use crate::pilot::{self, Escort, Pilot};
use crate::pre_jump::{self, PreJump};
use crate::recharge::{self, RechargeRefusal};
use crate::reserves::{Gauge, Reserves};
use crate::rulebook::RuleSource;
use crate::shipyard::{self, Quote, ShipPurchase, ShipRefusal, Shipyard, Yard};
use crate::sound::SimSound;
use crate::stats::ShipStats;
use crate::targeting::{self, TargetPick};
use crate::traffic::autopilot::Outcome;
use crate::traffic::npc::{AiType, Npc, NpcId};
use crate::traffic::table::SpawnTable;
use crate::traffic::{Traffic, World};

/// A jump, from J being accepted until the ship arrives.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Jump {
    /// The ship brakes and turns towards `bearing`, the map bearing of
    /// `to` (none when there is nothing to turn to), before it jumps.
    PreJump { to: SystemId, bearing: Option<f32> },
    /// The jump to the system has begun.
    Hyperspace(SystemId),
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
    /// Every `oütf`, read when the session starts.
    outfits: Vec<OutfitRecord>,
    /// Every `shïp`, read when the session starts.
    ships: Vec<ShipRecord>,
    /// How the ship performs, with the outfits it carries.
    stats: ShipStats,
    player: ShipState,
    /// The system's stellars, read when the session starts.
    sites: Vec<LandingSite>,
    /// The stellar the ship is docked at, if it has landed.
    landed: Option<StellarId>,
    /// The stellar selected as the navigation target, one of `sites`.
    nav_target: Option<StellarId>,
    /// The star map, read when the session starts.
    star_map: StarMap,
    /// The jump under way, from J being accepted until the ship arrives.
    jump: Option<Jump>,
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
    /// Whether the pilot has changed in a way that should be saved since
    /// this was last taken.
    save_due: bool,
    /// What the date is wrapped in when it is displayed, read when the
    /// session starts.
    date_affixes: DateAffixes,
    /// The NPCs in the system.
    traffic: Traffic,
    /// Whether the system is to be populated on the next traffic tick in
    /// flight: from the session's start, and after each take-off.
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
    /// The fire command the player holds.
    trigger: Trigger,
    /// The shots and beams in flight, and what the fight reports.
    combat: Combat,
    /// The NPC the player targets, if any.
    target: Option<NpcId>,
    /// The secondary weapon the player has selected, if any.
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
    /// Whether the escorts' standing orders are reset on entering a
    /// system (see [`Session::with_escort_orders`]).
    escort_orders: RuleSource,
    /// What a fighter the player launches does first (see
    /// [`Session::with_fighter_launch`]).
    fighter_launch: RuleSource,
    /// What becomes of the player's fighters out as it leaves the system
    /// (see [`Session::with_fighter_recall`]).
    fighter_recall: RuleSource,
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
    /// The fee and wage of a hire (see [`Session::with_hire_terms`]).
    hire_terms: hire::Shared<dyn HireTerms>,
    /// The control-bit test a ship's `Availability` for hire goes through
    /// (see [`Session::with_control_bits`]).
    control_bits: hire::Shared<dyn ControlBits>,
    /// Whether an unmet `Require` refuses a hire (see
    /// [`Session::with_hire_require`]).
    hire_require: RuleSource,
    /// Each ship class's roll for hire since the last landing, drawn the
    /// first time the bar's list asks it.
    hire_rolls: BTreeMap<ShipId, bool>,
    /// Whether each take-off pays the hired escorts a day's wages (see
    /// [`Session::with_take_off_pay`]).
    take_off_pay: RuleSource,
    /// Which wage a hired escort is paid (see
    /// [`Session::with_escort_wage`]).
    escort_wage: RuleSource,
    /// What paying the escorts did since this was last taken.
    pay_notes: Vec<PayNote>,
    /// The handlers of the set operators beyond the bit writes (see
    /// [`Session::with_set_ops`]).
    set_ops: hire::Shared<SetRegistry<Session>>,
    /// The set operator kinds skipped for want of a handler and told
    /// already; never saved.
    unhandled_ops: BTreeSet<SetOpKind>,
    /// What running set expressions had to tell since this was last taken.
    script_notes: Vec<ScriptNote>,
    /// How persons appear (see [`Session::with_person_rules`]).
    person_rules: hire::Shared<dyn PersonRules>,
    /// When a person's comm quote is said (see
    /// [`Session::with_comm_quote`]).
    comm_quote: RuleSource,
    /// The hail quotes' clock.
    quote_clock: persons::QuoteClock,
    /// The hail quotes said since they were last taken.
    quotes: Vec<persons::PersonQuote>,
    /// How granting and removing outfits go where the rules are disputed
    /// (see [`Session::with_outfit_rules`]).
    outfit_rules: OutfitRules,
    /// The order of the set-expression hooks where it is disputed (see
    /// [`Session::with_hook_rules`]).
    hook_rules: HookRules,
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
    /// still carries its default items, from an old save, owns them now,
    /// a ship never named, from an old save, is named after its class,
    /// and the reserves hold no more than the stats allow.
    ///
    /// A pilot last landed on a stellar of its system resumes docked there,
    /// silently, as the original resumes a pilot at its last planet.
    /// Otherwise (or when that stellar is no longer in the system, which
    /// the pilot then forgets) the ship starts at rest at the system's
    /// centre, facing up.
    ///
    /// # Errors
    ///
    /// When the ship cannot be read, or the system no longer exists.
    pub fn fly(
        catalog: &(impl PilotCatalog + CombatCatalog),
        mut pilot: Pilot,
    ) -> Result<Self, StartError> {
        let ship = pilot.ship;
        let fields = catalog
            .ship_fields(ship)
            .map_err(|reason| StartError::Ship(ship, reason))?;
        if !catalog.system_exists(pilot.system) {
            return Err(StartError::NoSystem(pilot.system));
        }
        pilot.explore(pilot.system);
        let sites = catalog.landing_sites(pilot.system);
        let docked = pilot
            .stellar
            .and_then(|stellar| sites.iter().find(|site| site.id == stellar));
        let player = ShipState {
            position: docked.map_or(Vec2::ZERO, |site| site.position),
            ..ShipState::default()
        };
        let landed = docked.map(|site| site.id);
        pilot.stellar = landed;
        let defaults = pilot::default_outfits(catalog, ship);
        let ships = catalog.ships();
        fill_in(&mut pilot, &defaults, &ships);
        let outfits = catalog.outfits();
        let mut session = Self {
            fields,
            defaults,
            ammo_outfits: Arsenal::ammo_outfits(&outfits),
            outfits,
            ships,
            // Refitted below, from the outfits the pilot owns.
            stats: ShipStats::default(),
            player,
            sites,
            landed,
            nav_target: None,
            star_map: StarMap::new(catalog.star_map()),
            jump: None,
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
            save_due: false,
            date_affixes: catalog.date_affixes(),
            traffic: Traffic::new(),
            traffic_due: true,
            arsenal: Arsenal::read(catalog),
            govts: Governments::read(catalog),
            // Refitted below, from the ship and the outfits the pilot owns.
            hull: HullSpec::default(),
            armament: Armament::default(),
            condition: Condition::Intact,
            trigger: Trigger::default(),
            combat: Combat::default(),
            target: None,
            secondary: None,
            strikes: Vec::new(),
            aboard: None,
            granted: None,
            talk: None,
            comm: Vec::new(),
            escort_orders: RuleSource::Engine,
            fighter_launch: RuleSource::Engine,
            fighter_recall: RuleSource::Engine,
            fighter_notes: Vec::new(),
            fleet: Vec::new(),
            restock_persons: false,
            hire_terms: hire::Shared(Rc::new(NovaHire::default())),
            control_bits: hire::Shared(Rc::new(NovaBits)),
            hire_require: RuleSource::Engine,
            hire_rolls: BTreeMap::new(),
            take_off_pay: RuleSource::Engine,
            escort_wage: RuleSource::Engine,
            pay_notes: Vec::new(),
            set_ops: hire::Shared(Rc::new(control::nova_set_ops())),
            unhandled_ops: BTreeSet::new(),
            script_notes: Vec::new(),
            person_rules: hire::Shared(Rc::new(NovaPersons::default())),
            comm_quote: RuleSource::Engine,
            quote_clock: persons::QuoteClock::default(),
            quotes: Vec::new(),
            outfit_rules: OutfitRules::default(),
            hook_rules: HookRules::default(),
            pilot,
        };
        session.refit(false);
        session.restock_fleet();
        Ok(session)
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

    /// The ship's stats with the outfits the pilot owns.
    fn current_stats(&self) -> ShipStats {
        ShipStats::new(
            self.fields,
            &outfit_mods(&self.pilot.outfits, &self.outfits),
        )
    }

    /// Recomputes the stats from the outfits the pilot owns: each gauge
    /// holds up to the stats' most, keeping no more than that, and when
    /// `gain`, one whose most rose gains as much. The hull and the weapons
    /// follow the ship and the outfits, ready to fire.
    fn refit(&mut self, gain: bool) {
        self.hull = self.arsenal.hull(self.pilot.ship);
        self.armament = self
            .arsenal
            .player(self.pilot.ship, &self.pilot.outfits, &self.outfits);
        let carried = self.secondary.filter(|&id| {
            self.armament
                .secondaries()
                .any(|secondary| secondary.id == id)
        });
        self.secondary = carried.or_else(|| self.next_secondary(None, false));
        let stats = self.current_stats();
        let reserves = &mut self.pilot.reserves;
        for (gauge, max) in [
            (&mut reserves.shield, stats.shield),
            (&mut reserves.armor, stats.armor),
            (&mut reserves.fuel, stats.fuel),
        ] {
            refit(gauge, max, gain);
        }
        self.stats = stats;
    }

    /// Advances the session one tick under the player's `controls`, ramps
    /// the engine glow with the thrust, then regenerates fuel. During the
    /// pre-jump stage the controls are ignored: the ship flies the stage
    /// ([`pre_jump::fly`]) instead, and the jump begins on the tick it is
    /// ready. A landed ship, or one in hyperspace, does not move, glows not
    /// at all, and gains no fuel. Any tick ends a gate's pending entry: the
    /// original's hypergate map is modal, so nothing ticks between the
    /// land key and the pick. A ship that is not intact ignores the
    /// controls and drifts, giving up a jump it was turning and braking
    /// for, and a destroyed one stays where it is.
    pub fn tick(&mut self, controls: Controls) {
        self.gate = None;
        if self.landed.is_some() {
            return;
        }
        let controls = match self.condition {
            Condition::Intact => controls,
            Condition::Disabled | Condition::Dying { .. } => {
                if matches!(self.jump, Some(Jump::PreJump { .. })) {
                    self.jump = None;
                }
                Controls::default()
            }
            Condition::Destroyed => {
                self.stop_thrust();
                self.player.velocity = Vec2::ZERO;
                return;
            }
        };
        match self.jump {
            None => {
                self.thrust(controls.thrust);
                step(&mut self.player, &self.stats.handling, controls);
            }
            Some(Jump::PreJump { to, bearing }) => {
                let handling = &self.stats.handling;
                let thrust =
                    pre_jump::fly(&mut self.player, handling, bearing, !self.stats.fast_jump);
                self.thrust(thrust);
                self.start_jump_if_ready(to, bearing);
            }
            Some(Jump::Hyperspace(_)) => return,
        }
        regenerate(&mut self.pilot.reserves.fuel, self.stats.fuel_regen);
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
    /// and rolled on `chance`.
    pub fn populate(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) {
        let system = self.pilot.system;
        let table = SpawnTable::resolve(
            catalog,
            system,
            self.star_map.govt(system),
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
        let facts = control::Facts {
            pilot: &self.pilot,
            ammo_outfits: &self.ammo_outfits,
            armament: &self.armament,
        };
        let world = World {
            persons: PersonWorld {
                rules: &*self.person_rules.0,
                gone: &self.pilot.gone_persons,
                grudges: &self.pilot.grudges,
                control_bits: &*self.control_bits.0,
                pilot: &facts,
                fleet: &self.pilot.escorts,
            },
            ..World::new(&[])
        };
        self.traffic.enter_in(table, world, chance);
        self.traffic_due = false;
        self.enter_escorts();
        self.strikes.clear();
        self.aboard = None;
        self.talk = None;
        self.clear_lost_target();
    }

    /// Advances the NPC traffic one tick, NPCs deciding as `behaviour`
    /// says, rolling on `chance`. While the ship is landed or jumping, it
    /// stands still. The first tick in flight after the session starts,
    /// and after each take-off, instead populates the system
    /// ([`Session::populate`]) from `catalog`, as the original sets a
    /// system up on arrival and on take-off; its NPCs move from the next.
    /// A target that landed or jumped out is let go.
    pub fn tick_traffic(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        behaviour: &(impl Behaviour + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) {
        if self.landed.is_none() && self.jumping().is_none() {
            if self.traffic_due {
                self.populate(catalog, chance);
            } else {
                let system_govt = self.star_map.govt(self.pilot.system);
                let facts = control::Facts {
                    pilot: &self.pilot,
                    ammo_outfits: &self.ammo_outfits,
                    armament: &self.armament,
                };
                let world = World {
                    sites: &self.sites,
                    player: Some(self.player_side()),
                    govts: &self.govts,
                    system_govt,
                    record: system_govt.map_or(0, |govt| self.pilot.legal_record(govt)),
                    persons: PersonWorld {
                        rules: &*self.person_rules.0,
                        gone: &self.pilot.gone_persons,
                        grudges: &self.pilot.grudges,
                        control_bits: &*self.control_bits.0,
                        pilot: &facts,
                        fleet: &self.pilot.escorts,
                    },
                };
                let strikes = std::mem::take(&mut self.strikes);
                self.traffic.tick_in(behaviour, world, &strikes, chance);
                self.dock_fighters();
                self.orphan_fighters();
            }
        }
        self.clear_lost_target();
    }

    /// What the traffic flies among in flight: the system's stellars, the
    /// player, the governments, the system's government and the player's
    /// legal record with it (none in an independent system), and the
    /// persons' world: the session's rules and control bits, and the
    /// pilot's persons gone and grudges.
    fn world(&self) -> World<'_> {
        let system_govt = self.star_map.govt(self.pilot.system);
        World {
            sites: &self.sites,
            player: Some(self.player_side()),
            govts: &self.govts,
            system_govt,
            record: system_govt.map_or(0, |govt| self.pilot.legal_record(govt)),
            persons: PersonWorld {
                rules: &*self.person_rules.0,
                gone: &self.pilot.gone_persons,
                grudges: &self.pilot.grudges,
                control_bits: &*self.control_bits.0,
                pilot: self,
                fleet: &self.pilot.escorts,
            },
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
            only: None,
            turrets_only: false,
            bays: false,
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

    /// The secondary weapon selected, if any: the first the ship carries
    /// when the session starts, and kept through a refit while the ship
    /// still carries it.
    #[must_use]
    pub fn secondary(&self) -> Option<WeaponId> {
        self.secondary
    }

    /// The rounds the selected secondary has left, for one that fires
    /// rounds of ammunition; none for one that fires without, or burns
    /// fuel.
    #[must_use]
    pub fn secondary_rounds(&self) -> Option<u32> {
        let spec = self.arsenal.weapon(self.secondary?)?;
        match spec.ammo {
            Ammo::Rounds(ammo) => {
                Some(outfit_rounds(&self.pilot.outfits, &self.ammo_outfits, ammo))
            }
            _ => None,
        }
    }

    /// Advances the fight a tick among the player and the NPCs, each aimed
    /// at its target, by `rules`, drawing each shot's inaccuracy and
    /// spread on `chance` (see [`Combat::tick`]); each NPC destroyed is taken out of
    /// the system, and the target is let go once it is breaking up or
    /// gone. While the ship is landed or jumping, it stands still.
    pub fn tick_combat(&mut self, rules: Rules, chance: &mut (impl Chance + ?Sized)) {
        if self.landed.is_some() || self.jumping().is_some() {
            return;
        }
        let was: Vec<(NpcId, Condition, Option<GovtId>)> = self
            .traffic
            .npcs()
            .iter()
            .map(|npc| (npc.id, npc.condition, npc.govt))
            .collect();
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
            trigger: self.trigger,
            target: self.target.map(ShipRef::Npc),
            reserves: &mut pilot.reserves,
            condition: &mut self.condition,
            armament: &mut self.armament,
            rounds: &mut rounds,
        }];
        for npc in self.traffic.npcs_mut() {
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
                ..
            } = npc;
            fighters.push(Fighter {
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
                armament,
                rounds,
            });
        }
        self.combat
            .tick(&mut fighters, &self.arsenal, &self.govts, rules, chance);
        let mut strikes = self.combat.take_strikes();
        self.refill_invincible(&mut strikes);
        self.hold_grudges(&strikes);
        self.punish(&strikes, &was, rules.law);
        self.strikes.extend(strikes);
        let sorties = self.combat.take_sorties();
        self.launch_fighters(&sorties);
        self.sync_fleet();
        self.lose_escorts();
        let destroyed: Vec<NpcId> = self
            .traffic
            .npcs()
            .iter()
            .filter(|npc| npc.condition == Condition::Destroyed)
            .map(|npc| npc.id)
            .collect();
        for id in destroyed {
            self.lose_destroyed(id);
        }
        self.orphan_fighters();
        self.clear_lost_target();
    }

    /// Convicts the player of its crimes among `strikes`, as `law` says:
    /// disabling an NPC, and breaking one up, which is disabling it too
    /// when it `was` intact before (as `_DamageShip` slaps both). Each
    /// victim's condition and government are as it `was` before the
    /// fight's tick.
    fn punish(
        &mut self,
        strikes: &[Strike],
        was: &[(NpcId, Condition, Option<GovtId>)],
        law: &dyn LegalCode,
    ) {
        for strike in strikes.iter().filter(|strike| strike.by == ShipRef::Player) {
            let ShipRef::Npc(id) = strike.ship else {
                continue;
            };
            let Some(&(_, condition, govt)) = was.iter().find(|(npc, ..)| *npc == id) else {
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
                legal::convict(&mut self.pilot, &changes);
            }
        }
    }

    /// Lets go of the target once it is no longer in the system (it was
    /// destroyed, landed or jumped out, or the system was populated
    /// afresh) or no longer targetable (it is breaking up), as the
    /// original does every frame.
    fn clear_lost_target(&mut self) {
        if self.target().is_none_or(|npc| !targeting::targetable(npc)) {
            self.target = None;
        }
    }

    /// Picks the player's target as `pick` says (see
    /// [`targeting`](crate::targeting)), and gives it: the nearest NPC, or
    /// the nearest threat, the target unchanged when there is none; or the
    /// next. While the ship is landed or jumping, nothing changes.
    pub fn select_target(&mut self, pick: TargetPick) -> Option<NpcId> {
        if self.landed.is_none() && self.jumping().is_none() {
            let npcs = self.traffic.npcs();
            let from = self.player.position;
            match pick {
                TargetPick::Nearest => {
                    let nearest = targeting::nearest(npcs, from, |_| true);
                    self.target = nearest.or(self.target);
                }
                TargetPick::NearestThreat => {
                    let nearest = targeting::nearest(npcs, from, Npc::threatens_player);
                    self.target = nearest.or(self.target);
                }
                TargetPick::Next => self.target = targeting::next(npcs, self.target),
                TargetPick::NextEscort => {
                    self.target = targeting::next_escort(npcs, self.target);
                }
            }
        }
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
        self.combat.take_diagnostics()
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
                self.arsenal.player(record.id, &defaults, &self.outfits)
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

    /// Ship type `ship`'s name, as its [`ShipRecord`] gives it, if the
    /// session has its record.
    #[must_use]
    pub fn ship_name(&self, ship: ShipId) -> Option<&str> {
        let record = self.ships.iter().find(|record| record.id == ship)?;
        Some(&record.name)
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
        match self.star_map.route(self.pilot.system, to, self.hyperlinks) {
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
    /// still jumps. Until it arrives, ticks in hyperspace move nothing.
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
        let map = |id| self.star_map.position(id);
        let bearing = map(self.pilot.system)
            .zip(map(next))
            .and_then(|(from, to)| jump_bearing(from, to));
        self.jump = Some(Jump::PreJump { to: next, bearing });
        self.start_jump_if_ready(next, bearing);
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
        check_jump(
            &self.player,
            self.pilot.reserves.fuel.now,
            self.pilot.course.first().copied(),
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

    /// Begins the jump to `to`, if the ship is ready to turn no further
    /// towards `bearing` and slow no more: the thrust stops and the glow
    /// goes out.
    fn start_jump_if_ready(&mut self, to: SystemId, bearing: Option<f32>) {
        let handling = &self.stats.handling;
        if pre_jump::stage(&self.player, handling, bearing, !self.stats.fast_jump) == PreJump::Ready
        {
            self.jump = Some(Jump::Hyperspace(to));
            self.stop_thrust();
            self.sounds.push(SimSound::JumpBegan);
        }
    }

    /// The system being jumped to, once the jump itself has begun and
    /// until the ship arrives; `None` during the pre-jump stage.
    #[must_use]
    pub fn jumping(&self) -> Option<SystemId> {
        match self.jump {
            Some(Jump::Hyperspace(to)) => Some(to),
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
    pub fn arrive(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<SystemId> {
        let Some(Jump::Hyperspace(first)) = self.jump else {
            return None;
        };
        self.jump = None;
        self.take_jump_cost(chance);
        let (mut from, mut at) = (self.pilot.system, first);
        if self.pilot.course.first() == Some(&first) {
            self.pilot.course.remove(0);
            for _ in 1..hops_per_jump(self.stats.multi_jump, self.multi_jump) {
                let Some(&next) = self.pilot.course.first() else {
                    break;
                };
                if self.multi_jump == MultiJumpRule::PerHop {
                    if self.pilot.reserves.fuel.now < JUMP_FUEL {
                        break;
                    }
                    self.take_jump_cost(chance);
                }
                self.pilot.course.remove(0);
                (from, at) = (at, next);
            }
        }
        let map = |id| self.star_map.position(id).unwrap_or_default();
        self.player = arrival(map(from), map(at), self.stats.jump_distance);
        let pilot = &mut self.pilot;
        pilot.system = at;
        pilot.stellar = None;
        pilot.explore(at);
        self.leave_with_fighters(false);
        self.sites = catalog.landing_sites(at);
        self.populate(catalog, chance);
        self.combat.clear();
        self.nav_target = None;
        self.sounds.push(SimSound::Arrived);
        self.messages.push(SimMessage::Arrived(at));
        Some(at)
    }

    /// Uses a jump's fuel and lets its days go by ([`Session::pass_jump_days`]).
    fn take_jump_cost(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.pilot.reserves.fuel.now -= JUMP_FUEL;
        self.pass_jump_days(chance);
    }

    /// Lets the days the stats give a jump go by, each stepping the
    /// planetary events, rolled on `chance`, and pays the hired escorts
    /// their wages for them.
    ///
    /// The events step out of the pilot, so that their `ActivateOn` can
    /// read the pilot through the session's control bits: no test reads
    /// the events.
    fn pass_jump_days(&mut self, chance: &mut (impl Chance + ?Sized)) {
        let mut events = std::mem::take(&mut self.pilot.events);
        for _ in 0..self.stats.jump_days {
            self.pilot.date = self.pilot.date.next_day();
            market::step_day(&self.goods, &mut events, self.gate(), chance);
        }
        self.pilot.events = events;
        self.pay_escorts(self.stats.jump_days);
    }

    /// Selects the next of the system's stellars as the navigation target,
    /// as the [`navigation`](crate::navigation) rule says, and gives it;
    /// `None`, and nothing is selected, when the system has no stellars.
    pub fn select_next_stellar(&mut self) -> Option<StellarId> {
        let stellars: Vec<StellarId> = self.sites.iter().map(|site| site.id).collect();
        self.nav_target = next_stellar(&stellars, self.nav_target);
        self.nav_target
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
            &self.star_map,
            self.pilot.system,
            &self.pilot.course,
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

    /// The star map, as read when the session started.
    #[must_use]
    pub fn star_map(&self) -> &StarMap {
        &self.star_map
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

    /// The fuel the ship gains each tick in flight.
    #[must_use]
    pub fn fuel_regen_per_tick(&self) -> f32 {
        self.stats.fuel_regen
    }

    /// Presses the land key, unless a jump is under way, its pre-jump stage
    /// included: as the
    /// [`landing`](crate::landing) rules say, with the navigation target
    /// and the pilot's legal record with the stellar's government, or the
    /// system's on the star map when the stellar has none. A press that
    /// selects a stellar makes it the navigation target and lands on
    /// nothing. A press that lands docks the ship at the stellar's centre,
    /// at rest, its heading and reserves unchanged, and clears the target,
    /// so that once it takes off L requests clearance again. Otherwise the
    /// ship flies on, its target kept, and the refusal says why.
    ///
    /// A press that would land on a hypergate or wormhole
    /// ([`GateKind::of`] its `Flags2`) enters it instead: it does not dock
    /// and makes no sound, and gives [`LandPress::AtGate`]. The entry
    /// then awaits [`Session::enter_hypergate`] or
    /// [`Session::enter_wormhole`], until the next tick.
    pub fn land(&mut self) -> Result<LandPress, LandingRefusal> {
        if self.jump.is_some() {
            return Err(LandingRefusal::Jumping);
        }
        if self.condition != Condition::Intact {
            return Err(LandingRefusal::Disabled);
        }
        let outcome = land_or_select(
            self.nav_target,
            &self.player,
            &self.sites,
            self.star_map.govt(self.pilot.system),
            |govt| self.pilot.legal_record(govt),
        )?;
        match outcome {
            LandOutcome::Selected { stellar, .. } => self.nav_target = Some(stellar),
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
    /// the systems its links lead to ([`gate::hypergate_destinations`]) for
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
            Ok(gate::hypergate_destinations(&links, &sites))
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
        let exit = if has_links(&links) {
            to.and_then(|to| Some((to, *hypergate_exit(&links, &sites, to, &self.star_map)?)))
                .ok_or(GateRefusal::Cancelled)
        } else {
            Err(GateRefusal::NoLinks)
        };
        match exit {
            Ok((system, exit)) => {
                self.come_out(system, &exit, catalog, chance);
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
    /// populated with its traffic ([`Session::populate`]), the fight left
    /// behind and [`SimSound::Arrived`]. It raises [`SimMessage::PassedWormhole`] or
    /// [`SimMessage::ExitedHypergate`].
    pub fn enter_wormhole(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        chance: &mut (impl Chance + ?Sized),
    ) -> Result<SystemId, GateRefusal> {
        let here = self
            .take_gate(GateKind::Wormhole)
            .ok_or(GateRefusal::NotAtGate)?;
        let sites = catalog.gate_sites();
        let exit = *wormhole_exit(here, self.pilot.system, &sites, self.wormholes, chance)
            .ok_or(GateRefusal::NoExit)?;
        self.come_out(exit.system, &exit, catalog, chance);
        self.messages.push(SimMessage::PassedWormhole(exit.system));
        Ok(exit.system)
    }

    /// Brings the ship out of `exit` in `system`: see
    /// [`Session::enter_wormhole`].
    fn come_out(
        &mut self,
        system: SystemId,
        exit: &GateSite,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.player = match self.gate_arrival {
            GateArrivalRule::Engine => emerge(exit, self.stats.handling.max_speed, chance),
            GateArrivalRule::LikeJump => {
                let map = |id| self.star_map.position(id).unwrap_or_default();
                let placed = arrival(
                    map(self.pilot.system),
                    map(system),
                    self.stats.jump_distance,
                );
                self.pass_jump_days(chance);
                placed
            }
        };
        let pilot = &mut self.pilot;
        pilot.system = system;
        pilot.stellar = None;
        pilot.explore(system);
        pilot.course.clear();
        self.leave_with_fighters(false);
        self.sites = catalog.landing_sites(system);
        self.populate(catalog, chance);
        self.combat.clear();
        self.nav_target = None;
        self.stop_thrust();
        self.sounds.push(SimSound::Arrived);
    }

    /// Docks the ship at `stellar`, one of the system's.
    fn dock(&mut self, stellar: StellarId) {
        self.nav_target = None;
        let site = self.sites.iter().find(|site| site.id == stellar);
        if let Some(site) = site {
            self.player.position = site.position;
        }
        let stellar_sound = site.and_then(|site| site.landing_sound);
        self.player.velocity = Vec2::ZERO;
        self.landed = Some(stellar);
        self.pilot.stellar = Some(stellar);
        self.combat.clear();
        self.target = None;
        self.talk = None;
        self.leave_with_fighters(true);
        self.hire_rolls.clear();
        self.save_due = true;
        self.stop_thrust();
        self.sounds.push(SimSound::Landed { stellar_sound });
    }

    /// Takes off from the stellar the ship is docked at, and gives it; the
    /// ship flies again from the stellar's centre, at rest, and the next
    /// traffic tick populates the system afresh
    /// ([`Session::tick_traffic`]). `None`, and nothing changes, when it
    /// has not landed.
    pub fn take_off(&mut self) -> Option<StellarId> {
        let stellar = self.landed.take()?;
        self.traffic_due = true;
        if self.take_off_pay == RuleSource::Engine {
            self.pay_escorts(1);
        }
        self.restock_fleet();
        self.fleet.clear();
        self.sounds.push(SimSound::TookOff);
        self.save_due = true;
        Some(stellar)
    }

    /// Changes the pilot with `change`, while the ship is landed (in the
    /// spaceport), and says whether it did: in flight nothing changes and
    /// `change` is not called. A save is due after a change. `change` must
    /// not change the ship class, which only [`Session::buy_ship`]
    /// changes, nor the outfits, which only [`Session::outfit`] and
    /// [`Session::buy_ship`] change, so the fields and the stats follow.
    pub fn transact(&mut self, change: impl FnOnce(&mut Pilot)) -> bool {
        if self.landed.is_none() {
            return false;
        }
        change(&mut self.pilot);
        self.save_due = true;
        true
    }

    /// The exchange of the stellar the ship is docked at, if it has landed
    /// at a trade center.
    #[must_use]
    pub fn market(&self) -> Option<Market> {
        let stellar = self.landed?;
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        market::market(
            &self.goods,
            stellar,
            site.flags,
            &self.pilot,
            self.stats.capacity,
            self.gate(),
        )
    }

    /// Trades on the exchange as `order` asks, and gives the tons moved: a
    /// change made in the spaceport, so a save is due. When the ship is
    /// not landed at a trade center, or the order would move nothing,
    /// nothing changes and the refusal says why.
    pub fn trade(&mut self, order: Order) -> Result<u32, TradeRefusal> {
        let market = self.market().ok_or(TradeRefusal::NoMarket)?;
        let tons = market.tons(order)?;
        let price = market.row(order.good).map_or(0, |row| row.price);
        self.transact(|pilot| market::settle(pilot, order, tons, price));
        Ok(tons)
    }

    /// The outfitter of the stellar the ship is docked at, if it has
    /// landed at one.
    #[must_use]
    pub fn outfitter(&self) -> Option<Outfitter> {
        let stellar = self.landed?;
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        Shop {
            records: &self.outfits,
            fields: self.fields,
            defaults: &self.defaults,
            site,
            fighter_room: &self.fighter_room(),
            gate: self.gate(),
        }
        .outfitter(&self.pilot)
    }

    /// Buys or sells one outfit as `order` asks: a change made in the
    /// spaceport, so a save is due, and the stats change with it. The
    /// outfit's `OnPurchase` or `OnSell` runs once, drawing on `chance`
    /// (see the `hooks` module). When the ship is not landed at an
    /// outfitter, or the order is refused, nothing changes and the
    /// refusal says why.
    pub fn outfit(
        &mut self,
        order: OutfitOrder,
        chance: &mut dyn Chance,
    ) -> Result<(), OutfitRefusal> {
        let outfitter = self.outfitter().ok_or(OutfitRefusal::NoOutfitter)?;
        outfitter.check(order)?;
        let price = outfitter.row(order.outfit).map_or(0, |row| row.price);
        let record = self
            .outfits
            .iter()
            .find(|record| record.id == order.outfit)
            .cloned()
            .ok_or(OutfitRefusal::NotListed)?;
        self.transact(|pilot| outfitter::settle(pilot, &record, order.direction, price));
        match order.direction {
            Direction::Buy => {
                let added = self.grant_outfit(record.id);
                self.run_script(&record.on_purchase, chance);
                if added && record.flags & OutfitFlags::REMOVE_AFTER_PURCHASE != 0 {
                    self.remove_after_purchase(record.id);
                }
            }
            Direction::Sell => self.run_script(&record.on_sell, chance),
        }
        self.refit(true);
        Ok(())
    }

    /// The shipyard of the stellar the ship is docked at, if it has landed
    /// at one.
    #[must_use]
    pub fn shipyard(&self) -> Option<Shipyard> {
        let stellar = self.landed?;
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        Yard {
            ships: &self.ships,
            outfits: &self.outfits,
            fields: self.fields,
            site,
            gate: self.gate(),
        }
        .shipyard(&self.pilot)
    }

    /// Buys a ship of class `ship`, trading in the one flown, and gives
    /// what the purchase did: a change made in the spaceport, so a save is
    /// due, and the session flies the new ship from then on, its fields,
    /// default items and stats read from its record. The old class's
    /// `OnRetire` runs first and the new class's `OnPurchase` last, each
    /// drawing on `chance`, the paint cleared as
    /// [`HookRules::purchase_paint_order`] says (see the `hooks` module).
    /// When the ship is not landed at a shipyard, or the purchase is
    /// refused, nothing changes and the refusal says why.
    pub fn buy_ship(
        &mut self,
        ship: ShipId,
        chance: &mut dyn Chance,
    ) -> Result<ShipPurchase, ShipRefusal> {
        let shipyard = self.shipyard().ok_or(ShipRefusal::NoShipyard)?;
        shipyard.check(ship)?;
        let record = self
            .ships
            .iter()
            .find(|record| record.id == ship)
            .cloned()
            .ok_or(ShipRefusal::NotListed)?;
        let quote = Quote {
            price: shipyard.row(ship).map_or(0, |row| row.price),
            trade_in: shipyard.trade_in,
        };
        self.ship_hook(self.pilot.ship, ShipHook::Retire, chance);
        let paint_last = self.hook_rules.purchase_paint_order == RuleSource::Engine;
        let old_mass = self.fields.mass;
        self.drop_fighters(|_, _| false);
        let outfits = std::mem::take(&mut self.outfits);
        let mut bought = None;
        self.transact(|pilot| {
            bought = Some(shipyard::purchase(
                pilot, old_mass, &record, quote, &outfits,
            ));
            if !paint_last {
                pilot.paint = None;
            }
        });
        self.outfits = outfits;
        self.fields = record.fields;
        self.defaults = pilot::tally(record.defaults.iter().copied());
        self.refit(false);
        self.ship_hook(record.id, ShipHook::Purchase, chance);
        if paint_last {
            // A new ship is unpainted (`_DoShipyardDialog` @0x5f022), once
            // its `OnPurchase` has run (@0x5f00d); the save is due already.
            self.pilot.paint = None;
        }
        bought.ok_or(ShipRefusal::NoShipyard)
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

    /// The ship's cargo space, in tons.
    #[must_use]
    pub fn capacity(&self) -> u32 {
        self.stats.capacity
    }

    /// How the ship performs, with the outfits it carries.
    #[must_use]
    pub fn stats(&self) -> ShipStats {
        self.stats
    }

    /// Whether the pilot should be saved: it has landed, taken off or
    /// changed in the spaceport since this was last taken. Taking it clears
    /// it.
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

    /// How the player's ship flies.
    #[must_use]
    pub fn handling(&self) -> Handling {
        self.stats.handling
    }

    /// The government the player belongs to, if any. Always `None`: a new
    /// pilot belongs to no government. Nova grants membership later, through
    /// storyline play, and the first `chär` names none (its `Govt1-4` set
    /// starting legal records, not membership).
    #[must_use]
    pub fn government(&self) -> Option<GovtId> {
        None
    }

    /// `good`'s name, as the exchange names it, if it is traded.
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
    /// of the [`Crime::Board`] against the target's government, the
    /// NPCs come to the target's help as for a hit by the player (a
    /// strike of no damage, answered on the next traffic tick), the
    /// target is marked boarded, and every other NPC after it lets it go
    /// and decides again (`_AIShipBoardedByPlayer`). Then, unless `rule`
    /// says its crew repels the boarders, its plunder is rolled on
    /// `chance` and the boarding is under way ([`Session::plunder`]).
    ///
    /// # Errors
    ///
    /// The [`BoardRefusal`] that applies, and nothing changes;
    /// [`BoardRefusal::NoTarget`] while landed or jumping.
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
        self.player.velocity = npc.state.velocity;
        let changes = law.penalties(Crime::Board, npc.govt, &self.govts);
        legal::convict(&mut self.pilot, &changes);
        self.strikes.push(Strike {
            ship: ShipRef::Npc(npc.id),
            by: ShipRef::Player,
            damage: 0.0,
            downed: None,
        });
        for other in self.traffic.npcs_mut() {
            if other.id == npc.id {
                other.boarded = true;
            }
        }
        self.drop_quarry(npc.id);
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

    /// What boarding `npc` grants as `rule` says, drawn on `chance`, added
    /// to the outfits the pilot owns and the ship refitted with them (see
    /// [`grant`](crate::grant)): none, and nothing drawn, for a ship no
    /// person flies or a person of no grant.
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
            self.outfit_rules.grant_max,
            chance,
        )?;
        for _ in 0..granted.count {
            self.grant_outfit(granted.outfit);
        }
        self.refit(true);
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
            self.aboard = None;
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
                let held: u32 = self.pilot.cargo.values().sum();
                let stored = tons.min(self.stats.capacity.saturating_sub(held));
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

    /// Takes up to `rounds` of ammunition outfit `outfit`, one at a time
    /// while the free mass covers it and it is below its `Max`, and gives
    /// how many; the ship is refitted with them.
    fn take_ammo(&mut self, outfit: OutfitId, rounds: u32) -> u32 {
        let Some(record) = self
            .outfits
            .iter()
            .find(|record| record.id == outfit)
            .cloned()
        else {
            return 0;
        };
        let unit = outfitter::unit_mass(&record, self.fields.mass);
        let mut count = 0;
        while count < rounds
            && i32::from(self.pilot.owned(outfit)) < i32::from(record.max)
            && outfitter::free_mass(
                self.fields,
                &self.defaults,
                &self.pilot.outfits,
                &self.outfits,
            ) >= unit
        {
            *self.pilot.outfits.entry(outfit).or_default() += 1;
            count += 1;
        }
        self.refit(true);
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
            self.aboard = None;
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
        self.aboard = None;
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
        self.aboard = None;
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
        let inherent_ai = self
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .and_then(|npc| self.ship_record(npc.ship))
            .map(|record| record.inherent_ai);
        let Some(npc) = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) else {
            return;
        };
        npc.govt = None;
        if let Some(inherent_ai) = inherent_ai {
            npc.ai_type = AiType::from_raw(inherent_ai);
        }
        npc.leader = None;
        npc.goal = Goal::Formation { guard: None };
        npc.target = None;
        npc.trigger = Trigger::default();
        npc.provoked = 0.0;
        npc.escort = Some(EscortDuty {
            slot: 0,
            ships: 0,
            spacing: 0.0,
            order: None,
        });
        let escort = Escort {
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
        self.pilot.escorts.push(escort);
        self.fleet.resize(self.pilot.escorts.len() - 1, None);
        self.fleet.push(Some(id));
        self.reform();
        self.save_due = true;
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
    /// else changes. Either change makes a save due. The captured class's
    /// `OnCapture` runs on `chance` before it joins the fleet; on "Use As
    /// My Ship" the old class's `OnRetire` runs first, both before the
    /// fuel draw and, as [`HookRules::capture_hook_order`] says, before or
    /// after the outfit swap (see the `hooks` module).
    pub fn assign(&mut self, choice: Assignment, chance: &mut dyn Chance) -> Option<Assigned> {
        let aboard = self.aboard.filter(|aboard| aboard.captured)?;
        self.aboard = None;
        let npc = self
            .npcs()
            .iter()
            .find(|npc| npc.id == aboard.npc)
            .cloned()?;
        if choice == Assignment::Escort {
            self.ship_hook(npc.ship, ShipHook::Capture, chance);
            self.join_fleet(npc.id);
            return Some(Assigned::Escort);
        }
        let record = self.ship_record(npc.ship).cloned();
        let Some(record) = record.filter(|_| self.npcs().len() + 1 < MAX_SHIPS_IN_SYSTEM) else {
            return Some(Assigned::Abandoned);
        };
        let hooks_first = self.hook_rules.capture_hook_order == RuleSource::Engine;
        if hooks_first {
            self.retire_for_capture(record.id, chance);
        }
        let stock = ShipStats::new(self.fields, &outfit_mods(&self.defaults, &self.outfits));
        let old = Escort {
            ship: self.pilot.ship,
            reserves: stock.full(),
            order: None,
            carried: false,
            wage: None,
            person: None,
        };
        let defaults = pilot::tally(record.defaults.iter().copied());
        let records = &self.outfits;
        self.pilot.outfits.retain(|id, _| {
            records
                .iter()
                .any(|record| record.id == *id && record.flags & OutfitFlags::PERSISTENT != 0)
        });
        for (&id, &count) in &defaults {
            let owned = self.pilot.outfits.entry(id).or_default();
            *owned = owned.saturating_add(count);
        }
        if !hooks_first {
            self.retire_for_capture(record.id, chance);
        }
        self.pilot.ship = record.id;
        self.fields = record.fields;
        self.defaults = defaults;
        self.player = npc.state;
        let share = if self.arsenal.hull(record.id).tough {
            TOUGH_TAKEOVER_ARMOR_SHARE
        } else {
            TAKEOVER_ARMOR_SHARE
        };
        let reserves = &mut self.pilot.reserves;
        reserves.shield.now = 0.0;
        reserves.armor.now =
            f64::from(record.fields.armor).mul_add(share, TAKEOVER_ARMOR_BASE) as f32;
        reserves.fuel.now = u32::try_from(record.fields.fuel)
            .ok()
            .filter(|&fuel| fuel > 0)
            .map_or(0.0, |fuel| chance.below(fuel) as f32);
        self.refit(false);
        for other in self.traffic.npcs_mut() {
            if other.leader == Some(npc.id) {
                other.leader = None;
            }
        }
        self.lose_captured(npc.id);
        self.leave(npc.id);
        self.pilot.escorts.push(old);
        self.fleet.resize(self.pilot.escorts.len() - 1, None);
        let placed = self.place_escort(self.pilot.escorts.len() - 1);
        self.fleet.push(placed);
        self.reform();
        if let Some(id) = placed {
            self.snap(id);
        }
        self.save_due = true;
        Some(Assigned::MyShip)
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

/// Fills in what a pilot from an old save lacks: the ship's default
/// items, `defaults`, when they are not yet read, and a name after its
/// class among `ships` for a ship never named.
fn fill_in(pilot: &mut Pilot, defaults: &BTreeMap<OutfitId, u16>, ships: &[ShipRecord]) {
    if pilot.default_outfits_pending {
        pilot.outfits.clone_from(defaults);
        pilot.default_outfits_pending = false;
    }
    if pilot.ship_name.is_none() {
        pilot.ship_name = Some(pilot::class_name(ships, pilot.ship));
    }
}

/// Sets `gauge` to hold up to `max`, keeping no more than that; when
/// `gain` and that is more than it held, it gains the difference.
fn refit(gauge: &mut Gauge, max: f32, gain: bool) {
    if gain {
        gauge.now += (max - gauge.max).max(0.0);
    }
    gauge.max = max;
    gauge.now = gauge.now.min(max);
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{CharacterStart, GateSite, LandingSite, SoundId, StellarId};
    use crate::catalog::{CommodityStrings, DisasterId, DisasterRecord, JunkId, JunkRecord};
    use crate::chance::NeverFires;
    use crate::clock::TICKS_PER_SECOND;
    use crate::control::Test;
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
        jump, jump_with, land_now, outfit, planet, star, starting,
    };

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
    fn a_new_pilot_belongs_to_no_government() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.government(), None);
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
        session.take_off();
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
        assert_eq!(session.take_off(), Some(StellarId(128)));
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
        assert_eq!(session.take_off(), None);
        assert_eq!(*session.player(), flying);
        assert_eq!(session.take_off(), None, "nor twice");
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
            crate::landing::check_landing(&flying, &planet(128, 30.0, -40.0), None, |_| 0)
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
        session.take_off();
        assert!(matches!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Selected { .. }))
        ));
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn l_with_a_target_selected_by_tab_lands_on_it_at_once() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.select_next_stellar(), Some(StellarId(128)));
        assert_eq!(
            session.land(),
            Ok(LandOutcome::Landed(StellarId(128)).into())
        );
        // Planet 129 is far away: L refuses and keeps it.
        session.take_off();
        session.select_next_stellar();
        assert_eq!(session.select_next_stellar(), Some(StellarId(129)));
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
    /// there, taking off and landing again.
    fn land_at(
        catalog: &FakePilotCatalog,
        pilot: &Pilot,
        stellar: i16,
    ) -> Result<StellarId, LandingRefusal> {
        let mut pilot = pilot.clone();
        pilot.stellar = Some(StellarId(stellar));
        let mut session = Session::fly(catalog, pilot).expect("flies");
        assert_eq!(session.take_off(), Some(StellarId(stellar)));
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
    fn landing_reads_the_pilots_record_with_each_stellars_government() {
        let catalog = governed([Some(LAWFUL), Some(WANTED)], None);
        let pilot = on_record(&catalog);
        assert_eq!(land_at(&catalog, &pilot, 128), Ok(StellarId(128)));
        assert_eq!(land_at(&catalog, &pilot, 129), denied(129));
    }

    #[test]
    fn landing_on_a_stellar_without_a_government_reads_its_systems_record() {
        let lawful = governed([None, None], Some(LAWFUL));
        let pilot = on_record(&lawful);
        assert_eq!(land_at(&lawful, &pilot, 129), Ok(StellarId(129)));
        let wanted = governed([None, Some(LAWFUL)], Some(WANTED));
        assert_eq!(land_at(&wanted, &pilot, 128), denied(128));
        assert_eq!(land_at(&wanted, &pilot, 129), Ok(StellarId(129)));
    }

    #[test]
    fn a_loaded_pilots_records_decide_landing() {
        let catalog = governed([Some(LAWFUL), Some(WANTED)], None);
        let loaded =
            crate::save::decode(&crate::save::encode(&on_record(&catalog))).expect("loads");
        assert_eq!(loaded.legal_record(LAWFUL), 10);
        assert_eq!(loaded.legal_record(WANTED), 9);
        assert_eq!(land_at(&catalog, &loaded, 128), Ok(StellarId(128)));
        assert_eq!(land_at(&catalog, &loaded, 129), denied(129));
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
        session.take_off();
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
    fn bound_for_131(
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
    const OUT: Vec2 = Vec2 { x: 0.0, y: 1200.0 };

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
        assert_eq!(session.take_sounds(), [SimSound::JumpBegan]);
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
        assert_eq!(sounds.last(), Some(&SimSound::JumpBegan));
        let began = sounds.iter().filter(|s| **s == SimSound::JumpBegan).count();
        assert_eq!(began, 1);
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
        session.take_off();
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
        session.take_off();
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
        session.take_off();
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
        session.take_off();
        assert_eq!(session.pilot().stellar(), Some(StellarId(128)));
        assert!(session.take_save_due());
        session.take_off();
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
        session.take_off();
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

    #[test]
    fn flying_a_pilot_in_a_system_that_no_longer_exists_is_an_error() {
        let pilot = landed_pilot(&catalog());
        let gone = FakePilotCatalog {
            systems: vec![SystemId(131)],
            ..catalog()
        };
        assert_eq!(
            Session::fly(&gone, pilot),
            Err(StartError::NoSystem(SystemId(130)))
        );
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
        session.take_off();
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
        session.take_off();
        assert_eq!(session.take_sounds(), [], "not landed");
        land_now(&mut session).expect("lands");
        session.take_sounds();
        session.take_off();
        assert_eq!(session.take_sounds(), [SimSound::TookOff]);
        session.take_off();
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
            base_prices: ["75", "350", "750", "900", "200"]
                .map(str::to_owned)
                .to_vec(),
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

    fn order(good: Good, direction: Direction, lot: Lot) -> Order {
        Order {
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
        assert_eq!(*catalog.goods_reads.borrow(), 3);
        land_now(&mut session).expect("lands");
        assert!(session.market().is_some());
        session.take_off();
        jump(&mut session, &catalog, 131);
        assert_eq!(*catalog.goods_reads.borrow(), 3);
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
        session.take_off();
        assert_eq!(session.market(), None, "taken off");
        let mut plain = Session::start(&self::catalog()).expect("starts");
        land_now(&mut plain).expect("lands");
        assert_eq!(plain.market(), None, "no trade center");
    }

    #[test]
    fn buying_pays_loads_and_makes_a_save_due() {
        let mut session = Session::start(&exchange()).expect("starts");
        land_now(&mut session).expect("lands");
        session.take_save_due();
        assert_eq!(session.trade(order(FOOD, Direction::Buy, Lot::One)), Ok(1));
        assert_eq!(session.pilot().cash(), 925);
        assert_eq!(session.pilot().held(FOOD), 1);
        assert!(session.take_save_due());
        assert_eq!(session.trade(order(METAL, Direction::Buy, Lot::Max)), Ok(5));
        assert_eq!(session.pilot().cash(), 125, "cash ran out first");
        assert_eq!(session.market().expect("an exchange").free, 14);
    }

    #[test]
    fn buying_stops_when_the_hold_is_full() {
        let catalog = FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { holds: 3, ..FAST }))],
            ..exchange()
        };
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        assert_eq!(session.trade(order(FOOD, Direction::Buy, Lot::Max)), Ok(3));
        session.take_save_due();
        assert_eq!(
            session.trade(order(FOOD, Direction::Buy, Lot::One)),
            Err(TradeRefusal::NoSpace)
        );
        assert_eq!(session.pilot().cash(), 775);
        assert!(!session.take_save_due(), "nothing changed");
    }

    #[test]
    fn selling_pays_the_local_price() {
        let mut session = Session::start(&exchange()).expect("starts");
        land_now(&mut session).expect("lands");
        assert_eq!(session.trade(order(METAL, Direction::Buy, Lot::Max)), Ok(6));
        assert_eq!(session.pilot().cash(), 40);
        session.take_save_due();
        assert_eq!(
            session.trade(order(METAL, Direction::Sell, Lot::One)),
            Ok(1)
        );
        assert_eq!(session.pilot().cash(), 40 + 160);
        assert!(session.take_save_due());
        assert_eq!(
            session.trade(order(METAL, Direction::Sell, Lot::Max)),
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
            session.trade(order(FOOD, Direction::Buy, Lot::One)),
            Err(TradeRefusal::NoMarket)
        );
        assert_eq!(session, flying);
        land_now(&mut session).expect("lands");
        session.take_save_due();
        let landed = session.clone();
        for refused in [
            order(FOOD, Direction::Sell, Lot::One),
            order(Good::Commodity(1), Direction::Buy, Lot::One),
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
            base_price: 100,
            sold_at: vec![StellarId(128)],
            bought_at: vec![StellarId(140)],
            buy_on: Test::default(),
            sell_on: Test::default(),
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
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        assert_eq!(
            session.trade(order(opals, Direction::Buy, Lot::Max)),
            Ok(12)
        );
        assert_eq!(session.pilot().cash(), 1000 - 12 * 80);
        assert_eq!(
            session.trade(order(opals, Direction::Sell, Lot::One)),
            Err(TradeRefusal::NotTraded),
            "not bought here"
        );
        session.take_off();
        jump(&mut session, &catalog, 131);
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        assert_eq!(
            session.trade(order(opals, Direction::Sell, Lot::Max)),
            Ok(12)
        );
        assert_eq!(session.pilot().cash(), 1000 - 12 * 80 + 12 * 125);
    }

    #[test]
    fn the_exchange_tests_junk_through_the_sessions_control_bits() {
        let opals = JunkRecord {
            id: JunkId(146),
            name: "Opals".to_owned(),
            base_price: 100,
            sold_at: vec![StellarId(128)],
            bought_at: Vec::new(),
            buy_on: Test::default(),
            sell_on: Test::parse("b7"),
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
            refusing.trade(order(opals, Direction::Buy, Lot::One)),
            Err(TradeRefusal::NotTraded)
        );
        assert_eq!(session.trade(order(opals, Direction::Buy, Lot::One)), Ok(1));
    }

    /// A food surplus at planet 140 (-15, 30 days, 35 % a day), and planet
    /// 140 a trade center trading food at 75.
    fn surplus() -> FakePilotCatalog {
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

    #[test]
    fn each_day_of_a_jump_ages_the_events() {
        let catalog = surplus();
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.events.insert(DisasterId(128), 5);
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
    fn chained(base: FakePilotCatalog, mod_val: i16) -> FakePilotCatalog {
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
    fn bound_for_134(catalog: &FakePilotCatalog, rule: MultiJumpRule) -> Session {
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
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        begin_jump_now(&mut session).expect("jumps");
        session.plot_course(SystemId(133)).expect("a route");
        assert_eq!(session.course(), ids(&[133]));
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.course(), ids(&[133]));
        assert_eq!(session.reserves().fuel.now, 200.0);
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
        assert_eq!(session.trade(order(METAL, Direction::Buy, Lot::Max)), Ok(6));
        let pilot = session.pilot().clone();
        let mut resumed = Session::fly(&catalog, pilot).expect("flies");
        let market = resumed.market().expect("docked at the exchange");
        assert_eq!(market.free, 14);
        assert_eq!(market.row(METAL).map(|row| row.held), Some(6));
        assert_eq!(
            resumed.trade(order(METAL, Direction::Sell, Lot::Max)),
            Ok(6)
        );
        assert_eq!(resumed.pilot().cash(), 1000);
    }

    // The outfitter.

    use crate::catalog::{GovtId, OutfitRecord};
    use crate::outfitter::{OutfitFlags, OutfitOrder, OutfitRefusal};
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
        assert_eq!(session.outfitter(), None, "in flight");
        assert_eq!(
            session.outfit(buy(SPEED), &mut NeverFires),
            Err(OutfitRefusal::NoOutfitter)
        );
        land_now(&mut session).expect("lands");
        let outfitter = session.outfitter().expect("an outfitter");
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
        assert_eq!(session.outfitter(), None, "no outfitter here");
        assert_eq!(
            session.outfit(buy(SPEED), &mut NeverFires),
            Err(OutfitRefusal::NoOutfitter)
        );
    }

    /// The outfitting catalog with outfit 305 sold only on control bit 7,
    /// and hidden while it is refused.
    fn outfitting_on_bit_7() -> FakePilotCatalog {
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
        let outfitter = session.outfitter().expect("an outfitter");
        assert_eq!(outfitter.row(gated), None);
        session.set_control_bit(crate::control::Bit::new(7).expect("a bit"), true);
        let outfitter = session.outfitter().expect("an outfitter");
        assert_eq!(outfitter.row(gated).map(|row| row.buy), Some(Ok(())));
        assert_eq!(session.outfit(buy(gated), &mut NeverFires), Ok(()));
    }

    #[test]
    fn the_outfitter_tests_availability_through_the_sessions_control_bits() {
        let gated = OutfitId(305);
        let mut session = outfitted(&outfitting_on_bit_7())
            .with_control_bits(Rc::new(crate::testkit::RefuseBits(&[7])));
        session.set_control_bit(crate::control::Bit::new(7).expect("a bit"), true);
        let outfitter = session.outfitter().expect("an outfitter");
        assert_eq!(outfitter.row(gated), None);
        assert_eq!(
            session.outfit(buy(gated), &mut NeverFires),
            Err(OutfitRefusal::NotListed)
        );
    }

    #[test]
    fn a_session_reads_the_outfits_once_when_it_starts() {
        let catalog = outfitting();
        let mut session = outfitted(&catalog);
        let reads = *catalog.outfit_reads.borrow();
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        session.outfitter().expect("an outfitter");
        session.take_off();
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
        assert_eq!(session.pilot().cash(), 24_500, "sold for half");
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
    fn a_cargo_pod_adds_space_to_the_exchange_too() {
        let mut session = outfitted(&outfitting());
        assert_eq!(session.capacity(), 20);
        session.outfit(buy(CARGO), &mut NeverFires).expect("bought");
        assert_eq!(session.capacity(), 30);
        assert_eq!(session.market().expect("an exchange").free, 30);
        session.outfit(sell(CARGO), &mut NeverFires).expect("sold");
        assert_eq!(session.capacity(), 20);
    }

    #[test]
    fn selling_space_below_the_cargo_held_is_allowed_and_none_is_free() {
        let mut session = outfitted(&outfitting());
        session.outfit(buy(CARGO), &mut NeverFires).expect("bought");
        let food = order(FOOD, Direction::Buy, Lot::Max);
        assert_eq!(session.trade(food), Ok(30));
        assert_eq!(session.outfit(sell(CARGO), &mut NeverFires), Ok(()));
        let market = session.market().expect("an exchange");
        assert_eq!((market.capacity, market.free), (20, 0));
        assert_eq!(session.pilot().held(FOOD), 30);
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
        let outfitter = session.outfitter().expect("an outfitter");
        let row = outfitter.row(OutfitId(310)).expect("listed, sell-only");
        assert_eq!(row.buy, Err(OutfitRefusal::NotForSale));
        assert_eq!(row.sell, Ok(()));
        assert_eq!(session.outfit(sell(OutfitId(310)), &mut NeverFires), Ok(()));
        assert_eq!(session.pilot().owned(OutfitId(310)), 1);
        assert_eq!(session.pilot().cash(), 25_000 + 2000);
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
        let outfitter = outfitted(&catalog).outfitter().expect("an outfitter");
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

    // The shipyard.

    use crate::catalog::ShipRecord;
    use crate::shipyard::{ShipPurchase, ShipRefusal};
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
    /// credits, carrying a fuel tank).
    pub(super) fn shipbuying() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![(SystemId(130), vec![shipyard_site()])],
            ships: vec![(ShipId(128), Ok(FAST)), (ShipId(129), Ok(HEAVY))],
            defaults: vec![(ShipId(129), vec![(TANK, 1)])],
            ship_records: vec![
                ship(128, FAST),
                ShipRecord {
                    cost: 17_500,
                    defaults: vec![(TANK, 1)],
                    ..ship(129, HEAVY)
                },
            ],
            ..outfitting()
        }
    }

    pub(super) const NEW: ShipId = ShipId(129);

    #[test]
    fn there_is_a_shipyard_only_while_landed_at_one() {
        let catalog = shipbuying();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.shipyard(), None, "in flight");
        assert_eq!(
            session.buy_ship(NEW, &mut NeverFires),
            Err(ShipRefusal::NoShipyard)
        );
        land_now(&mut session).expect("lands");
        let shipyard = session.shipyard().expect("a shipyard");
        assert_eq!(shipyard.rows.len(), 2);
        assert_eq!((shipyard.cash, shipyard.trade_in), (25_000, 2500));
        assert_eq!(shipyard.current, ShipId(128));
        let plain = FakePilotCatalog {
            sites: vec![(SystemId(130), vec![outfitter_site()])],
            ..shipbuying()
        };
        let mut session = outfitted(&plain);
        assert_eq!(session.shipyard(), None, "no shipyard here");
        assert_eq!(
            session.buy_ship(NEW, &mut NeverFires),
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
            refusing.buy_ship(NEW, &mut NeverFires),
            Err(ShipRefusal::NotForSale)
        );
        session
            .buy_ship(NEW, &mut NeverFires)
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
        session.buy_ship(NEW, &mut NeverFires).expect("bought");
        session.shipyard().expect("a shipyard");
        session.take_off();
        jump(&mut session, &catalog, 131);
        assert_eq!(*catalog.ship_record_reads.borrow(), 2);
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
    fn buying_a_ship_or_using_a_capture_as_my_ship_keeps_the_ships_name() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session.pilot.ship_name = Some("Kestrel".to_owned());
        session.buy_ship(NEW, &mut NeverFires).expect("bought");
        assert_eq!(session.pilot().ship_name(), Some("Kestrel"));
    }

    #[test]
    fn buying_a_ship_makes_a_save_due_and_a_refused_one_changes_nothing() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        let before = session.clone();
        assert_eq!(
            session.buy_ship(ShipId(999), &mut NeverFires),
            Err(ShipRefusal::NotListed)
        );
        let mut poor = session.clone();
        poor.pilot.cash = 14_999;
        let poorer = poor.clone();
        assert_eq!(
            poor.buy_ship(NEW, &mut NeverFires),
            Err(ShipRefusal::CannotAfford)
        );
        assert_eq!(poor, poorer);
        assert!(!poor.take_save_due());
        assert_eq!(session, before);
        assert!(!session.take_save_due());
        assert_eq!(
            session.buy_ship(NEW, &mut NeverFires),
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
    fn buying_a_ship_unpaints_it_and_a_refused_purchase_keeps_the_paint() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        let paint = crate::outfit_effects::Rgb15 { r: 4, g: 5, b: 6 };
        session.pilot.paint = Some(paint);
        assert_eq!(
            session.buy_ship(ShipId(999), &mut NeverFires),
            Err(ShipRefusal::NotListed)
        );
        assert_eq!(session.pilot().paint(), Some(paint));
        session.buy_ship(NEW, &mut NeverFires).expect("bought");
        assert_eq!(session.pilot().paint(), None);
    }

    #[test]
    fn after_a_purchase_the_stats_are_the_new_ships_with_its_outfits() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        session.buy_ship(NEW, &mut NeverFires).expect("bought");
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
        session.buy_ship(NEW, &mut NeverFires).expect("bought");
        let outfitter = session.outfitter().expect("an outfitter");
        assert_eq!(outfitter.free_mass, 12, "its tank is fitted on top");
        let shipyard = session.shipyard().expect("a shipyard");
        assert_eq!(shipyard.current, NEW);
        assert_eq!(shipyard.trade_in, 17_500 / 4 + 500, "the hull and its tank");
    }

    #[test]
    fn after_take_off_the_ship_flies_at_the_new_top_speed() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session.buy_ship(NEW, &mut NeverFires).expect("bought");
        session.take_off().expect("took off");
        let mut expected = *session.player();
        for _ in 0..200 {
            session.tick(THRUST);
            step(&mut expected, &ShipStats::new(HEAVY, &[]).handling, THRUST);
        }
        assert_eq!(*session.player(), expected);
        assert!((session.player().velocity.length() - 9.0).abs() < 1e-3);
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
        let mut session = outfitted(&catalog);
        assert_eq!(session.pilot().owned(PLATE), 1);
        assert_ne!(FAST.mass, HEAVY.mass);
        let bought = session.buy_ship(NEW, &mut NeverFires).expect("bought");
        assert_eq!(bought.sold_back, BTreeMap::from([(PLATE, 1)]));
        assert_eq!(bought.refund, 100 * i64::from(FAST.mass) / 2);
        assert_eq!(bought.trade_in, 2500, "persistent: not in the trade-in");
        assert_eq!(session.pilot().cash(), 25_000 - 17_500 + 2500 + 2000);
        assert_eq!(session.pilot().owned(PLATE), 0);
    }

    #[test]
    fn a_pilot_saved_after_a_purchase_flies_again_as_it_was() {
        let catalog = shipbuying();
        let mut session = outfitted(&catalog);
        session
            .trade(order(FOOD, Direction::Buy, Lot::Max))
            .expect("bought");
        session.buy_ship(NEW, &mut NeverFires).expect("bought");
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
    fn traffic_stands_still_while_landed_or_jumping() {
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
        let before = session.npcs().to_vec();
        let mut chance = Draws::of(&[]);
        session.tick_traffic(&catalog, &Peaceful, &mut chance);
        assert_eq!(session.npcs(), before, "jumping");
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
    fn taking_off_repopulates_the_system_on_the_next_traffic_tick() {
        let catalog = trafficked(130, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        land_now(&mut session).expect("lands on planet 128");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)], "landed");
        session.take_off().expect("took off");
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)], "until it ticks");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(2), NpcId(3)]);
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(2), NpcId(3)], "populated once");
    }

    #[test]
    fn a_ship_landed_from_the_start_populates_once_it_takes_off() {
        let catalog = trafficked(130, 2, 1);
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands on planet 128");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [], "landed");
        session.take_off().expect("took off");
        session.tick_traffic(&catalog, &Peaceful, &mut NeverFires);
        assert_eq!(npc_ids(&session), [NpcId(0), NpcId(1)]);
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
        jump_with(&mut session, &catalog, 131, &mut linked_fleet_draws(3));
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

    /// Ship `id` armed with one of `weapon`, 30 pixels across, breaking
    /// up for 2 ticks before `bööm` 133 destroys it.
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
    /// 129, a trader: 30 shield, 45 armour) each carrying a blaster.
    fn armed() -> FakePilotCatalog {
        FakePilotCatalog {
            weapons: vec![blaster()],
            hulls: vec![armed_hull(128, 128), armed_hull(129, 128)],
            ..trafficked(130, 1, 1)
        }
    }

    /// `catalog`'s session with its one NPC placed 100 pixels above the
    /// player (at the centre, facing up), facing `heading`.
    fn facing_an_npc(catalog: &FakePilotCatalog, heading: u32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.populate(catalog, &mut Draws::of(&[6, 6, 0, 0, 750, 650, heading]));
        assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, -100.0));
        session
    }

    const FIRE: Trigger = Trigger {
        primary: true,
        secondary: None,
        only: None,
        turrets_only: false,
        bays: false,
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
            slot: 2,
            ships: 2,
            spacing: 30.0,
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
    struct Firing;

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
            outfits: vec![outfit(300, &[(MOD_AMMO, 138)])],
            defaults: vec![(ShipId(128), vec![(OutfitId(300), 2)])],
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        for _ in 0..8 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(session.pilot().outfits().count(), 0, "both rockets fired");
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

    const DISABLING: Rules = Rules {
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
        session.pilot.set_legal_record(GovtId(150), -20);
        session.pilot.set_legal_record(GovtId(151), 30);
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
        session.pilot.set_legal_record(GovtId(150), -20);
        session.tick_traffic(&catalog, &spy, &mut NeverFires);
        let (_, system, record, _) = spy.seen.take()[0];
        assert_eq!((system, record), (None, 0), "an independent system");
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
        session.take_off();
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
                (Some(Condition::Dying { ticks_left: 1 }), [-10, -10, 9, 20]),
                (Some(Condition::Dying { ticks_left: 0 }), [-10, -10, 9, 20]),
                (None, [-10, -10, 9, 20]),
            ],
            "hits that leave it intact change nothing; by the engine's law, the neutral gains"
        );
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("reads");
        assert_eq!(&saved, session.pilot(), "the records saved as they are");
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
        // NPC 0 at (0, -100) and NPC 1 at (100, -100), both facing right.
        session.populate(
            &catalog,
            &mut Draws::of(&[6, 6, 0, 0, 750, 650, 90, 2, 6, 6, 0, 0, 850, 650, 90, 2]),
        );
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
        assert_eq!(drifted.velocity, moving.velocity, "no thrust");
        assert_eq!(drifted.heading, moving.heading, "no turn");
        assert_eq!(drifted.position, moving.position + moving.velocity);
        assert_eq!(session.take_sounds(), [SimSound::ThrustStopped]);
        session.hold_trigger(FIRE);
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(session.shots(), [], "nor fires");
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

    /// An NPC one shot of whose disables, breaks up and destroys the
    /// player at once: no shield, armour gone.
    fn deadly() -> FakePilotCatalog {
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
        session.take_off().expect("took off");
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
        session.buy_ship(NEW, &mut NeverFires).expect("bought");
        session.take_off().expect("took off");
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
    fn nothing_is_targeted_while_landed() {
        let catalog = trafficked(130, 2, 3);
        let mut session = two_npcs(&catalog);
        land_now(&mut session).expect("lands on planet 128");
        assert_eq!(session.select_target(TargetPick::Next), None);
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
    /// 129 carries a blaster, a torch and missiles, in that order.
    fn with_secondaries(catalog: FakePilotCatalog) -> FakePilotCatalog {
        let mut outfits = catalog.outfits.clone();
        outfits.push(outfit(310, &[(MOD_AMMO, 140)]));
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
    fn the_first_secondary_is_selected_at_start() {
        let session = Session::start(&with_secondaries(catalog())).expect("starts");
        assert_eq!(session.secondary(), Some(ROCKET));
        let unarmed = Session::start(&catalog()).expect("starts");
        assert_eq!(unarmed.secondary(), None);
        assert_eq!(unarmed.secondary_rounds(), None);
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
                Some(MISSILE),
                Some(TORCH),
                Some(ROCKET),
                Some(TORCH),
                Some(MISSILE)
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
        session.hold_fire(false, true);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            fired_by_the_player(&session.take_combat_events()),
            [MISSILE]
        );
    }

    #[test]
    fn the_secondarys_rounds_follow_its_ammunition() {
        let mut session = Session::start(&with_secondaries(catalog())).expect("starts");
        assert_eq!(session.secondary_rounds(), Some(3));
        session.hold_fire(false, true);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.secondary_rounds(), Some(2));
        session.select_secondary(false);
        assert_eq!(session.secondary_rounds(), None, "missiles: unlimited");
        session.select_secondary(false);
        assert_eq!(session.secondary_rounds(), None, "a torch burns fuel");
    }

    #[test]
    fn an_outfit_bought_keeps_the_secondary_selected() {
        let catalog = with_secondaries(outfitting());
        let mut session = outfitted(&catalog);
        session.select_secondary(false);
        assert_eq!(session.secondary(), Some(MISSILE));
        session.outfit(buy(SPEED), &mut NeverFires).expect("bought");
        assert_eq!(session.secondary(), Some(MISSILE));
    }

    #[test]
    fn a_ship_bought_keeps_the_secondary_it_carries_or_selects_its_first() {
        let catalog = with_secondaries(shipbuying());
        let mut session = outfitted(&catalog);
        session.buy_ship(NEW, &mut NeverFires).expect("bought");
        assert_eq!(session.secondary(), Some(TORCH), "its first");
        let mut session = outfitted(&catalog);
        session.select_secondary(false);
        session.buy_ship(NEW, &mut NeverFires).expect("bought");
        assert_eq!(session.secondary(), Some(MISSILE), "still carried");
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
    /// carrying `npc`, the player's ship type's `Flags` `flags`.
    fn arming(player: WeaponRecord, npc: WeaponRecord, flags: u16) -> FakePilotCatalog {
        FakePilotCatalog {
            weapons: vec![player, npc],
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
    /// (at the centre, facing up), facing `heading`.
    fn with_an_npc_at(catalog: &FakePilotCatalog, x: u32, y: u32, heading: u32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.populate(catalog, &mut Draws::of(&[6, 6, 0, 0, x, y, heading]));
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
    /// 133 and 134 a homing missile and 135 point defence.
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
            session.pilot.set_legal_record(GovtId(143), record);
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
    /// rest idle.
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
                Trigger::default()
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
        session.jump = Some(crate::session::Jump::Hyperspace(SystemId(131)));
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
            base_price: 10,
            sold_at: Vec::new(),
            bought_at: Vec::new(),
            buy_on: Test::default(),
            sell_on: Test::default(),
        }];
        catalog.outfits.push(outfit(310, &[]));
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.good_name(Good::Commodity(0)), Some("Food"));
        assert_eq!(session.good_name(Good::Commodity(4)), Some("Metal"));
        assert_eq!(session.good_name(Good::Commodity(5)), None, "not traded");
        assert_eq!(session.good_name(Good::Junk(JunkId(146))), Some("Ice"));
        assert_eq!(session.good_name(Good::Junk(JunkId(147))), None);
        assert_eq!(session.outfit_name(OutfitId(310)), Some("Outfit 310"));
        assert_eq!(session.outfit_name(OutfitId(311)), None);
    }

    /// [`boardable`] with rockets (weapon 140, firing rounds of their own)
    /// on both ships, the trader holding 9 rounds, and ammunition outfit
    /// 310 for them of `mass` and `max`.
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
        assert_eq!(by_rounds.secondary_rounds(), Some(0));
        assert_eq!(
            take(&mut by_rounds, Take::Ammo, &[]).0,
            Taken::Ammo {
                outfit: OutfitId(310),
                count: 9
            }
        );
        assert_eq!(by_rounds.pilot().owned(OutfitId(310)), 9);
        assert_eq!(by_rounds.secondary_rounds(), Some(9), "refitted");
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
                slot: 2,
                ships: 2,
                spacing: crate::escort::spacing(None, &[None]),
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
        assert_eq!((duty.slot, duty.ships, duty.order), (2, 2, None));
        assert_eq!(
            old.state,
            ShipState {
                position: crate::escort::slot_position(session.player(), 2, 2, duty.spacing),
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

    #[test]
    fn use_as_my_ship_swaps_the_player_into_the_captured_ship_and_keeps_the_old_one() {
        let catalog = kitted();
        let mut session = alongside(&catalog);
        session.pilot.outfits = BTreeMap::from([(OutfitId(400), 1), (OutfitId(401), 2)]);
        session.pilot.cargo.insert(Good::Commodity(2), 4);
        session.pilot.cash = 777;
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
        assert_eq!(session.pilot().cargo().collect::<Vec<_>>(), held);
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

    #[test]
    fn a_grant_weighs_each_outfit_by_its_raw_mass() {
        let mut catalog = granting();
        // 20 tons raw, but 8 by the ship's 40 tons with Flags 0x0400: two
        // would fit the 30 tons free scaled, and only one raw.
        catalog.outfits[0].mass = 20;
        catalog.outfits[0].flags = OutfitFlags::MASS_BY_MASS;
        let mut session = alongside_ace(&catalog);
        let (boarded, _) = board_drawing(&mut session, NovaBoarding::default(), &ACE_DRAWS);
        assert!(boarded.is_ok());
        assert_eq!(session.pilot().owned(OutfitId(200)), 1);
        assert_eq!(session.take_grant().map(|granted| granted.count), Some(1));
    }

    #[test]
    fn one_grant_max_holds_boarding_and_g_to_the_max_alike() {
        // Ace grants two of an outfit of `Max` 1. `GrantMax`, set once on
        // the session's outfit rules, holds the boarding grant (whatever
        // the boarding rule) and `G` to it; by the engine neither is held.
        let mut catalog = granting();
        catalog.outfits[0].max = 1;
        let g = |session: Session| {
            let mut session = session.with_set_ops(std::rc::Rc::new(crate::nova_set_ops()));
            let expr = crate::control::SetExpr::parse("G200 G200").expect("parses");
            session.run_set(&expr, &mut Draws::of(&[]));
            session.pilot().owned(OutfitId(200))
        };
        for (rules, owned) in [
            (OutfitRules::default(), 2),
            (
                OutfitRules {
                    grant_max: RuleSource::Bible,
                    ..OutfitRules::default()
                },
                1,
            ),
        ] {
            let mut boarded = alongside_ace(&catalog).with_outfit_rules(rules);
            board_drawing(&mut boarded, NovaBoarding::default(), &ACE_DRAWS)
                .0
                .expect("boards");
            assert_eq!(boarded.pilot().owned(OutfitId(200)), owned, "boarded");
            assert_eq!(g(alongside(&catalog).with_outfit_rules(rules)), owned, "G");
        }
    }

    #[test]
    fn held_to_the_max_boarding_and_g_weigh_an_outfit_as_the_outfitter_does() {
        // Of a `Mass` scaled by Flags 0x0400 on the ship's 40 tons, with
        // 30 tons free: 20 raw is 8, 75 is 30 and 80 is 32. Held to the
        // `Max`, boarding (granting one) and `G` agree, by the scaled mass.
        let rules = OutfitRules {
            grant_max: RuleSource::Bible,
            ..OutfitRules::default()
        };
        for (mass, fits) in [(20, true), (75, true), (80, false)] {
            let mut catalog = granting();
            catalog.persons[0].grant_count = 1;
            catalog.outfits[0].mass = mass;
            catalog.outfits[0].flags = OutfitFlags::MASS_BY_MASS;
            let mut boarded = alongside_ace(&catalog).with_outfit_rules(rules);
            assert_eq!(boarded.free_mass(), 30);
            board_drawing(&mut boarded, NovaBoarding::default(), &ACE_DRAWS)
                .0
                .expect("boards");
            let mut scripted = alongside(&catalog)
                .with_set_ops(std::rc::Rc::new(crate::nova_set_ops()))
                .with_outfit_rules(rules);
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
        catalog.outfits[0].mods = [(crate::outfit_effects::MAP, 1), (0, 0), (0, 0), (0, 0)];
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
    fn no_fuel_is_sold_in_flight() {
        let mut session = half_empty_at_port();
        session.take_off();
        session.take_save_due();
        assert_refused(session, RechargeRefusal::NoFuel);
    }

    // Navigation.

    /// [`catalog`] with system 130 holding three planets whose navigation
    /// order, 131, 128, 129, is not their order by distance from the
    /// centre, where the ship starts: 128 is nearest, then 131, then 129.
    fn three_stellars() -> FakePilotCatalog {
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
    fn tab_selects_the_systems_stellars_in_navigation_order_and_wraps() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        for expected in [131, 128, 129, 131, 128] {
            assert_eq!(session.select_next_stellar(), Some(StellarId(expected)));
            assert_eq!(session.nav_target(), Some(StellarId(expected)));
        }
    }

    #[test]
    fn a_system_with_no_stellars_has_no_target_to_select() {
        let empty = FakePilotCatalog {
            sites: Vec::new(),
            ..catalog()
        };
        let mut session = Session::start(&empty).expect("starts");
        assert_eq!(session.select_next_stellar(), None);
        assert_eq!(session.nav_target(), None);
    }

    #[test]
    fn the_target_and_the_course_are_independent() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_next_stellar();
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.nav_target(), Some(StellarId(131)));
        session.select_next_stellar();
        assert_eq!(session.course(), ids(&[131, 132]));
        assert_eq!(session.nav_target(), Some(StellarId(128)));
    }

    #[test]
    fn selecting_a_target_makes_no_save_due() {
        let mut session = Session::start(&three_stellars()).expect("starts");
        session.select_next_stellar();
        assert!(!session.take_save_due());
    }

    #[test]
    fn arriving_in_another_system_clears_the_target() {
        let catalog = three_stellars();
        let mut session = Session::start(&catalog).expect("starts");
        session.select_next_stellar();
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
        assert_eq!(session.select_next_stellar(), Some(StellarId(140)));
    }

    // Hyper Select.

    /// [`three_stellars`] with 130 a hub: it lists 134, 131 (twice),
    /// itself, a missing 999 and 135, in that Con order; 136 lists 130
    /// one way; 131 lists 132; 133 lists nothing and exists too.
    fn hub() -> FakePilotCatalog {
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
        session.select_next_stellar();
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

    /// Stellar `id` at (`x`, `y`) in `system`, with `flags2`, its `links`
    /// and its exit angle.
    fn gate_site(
        id: i16,
        system: i16,
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
            system: SystemId(system),
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
    fn gated() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![
                (SystemId(130), vec![gate_landing(300, HYPERGATE)]),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            gates: vec![
                gate_site(300, 130, (0.0, 0.0), 0x1200, &[310, 320, 999], 120),
                gate_site(310, 131, (100.0, 200.0), HYPERGATE, &[300], 90),
                gate_site(320, 132, (-50.0, 0.0), HYPERGATE, &[], -1),
            ],
            ..catalog()
        }
    }

    /// [`gated`]'s session, with L pressed twice over the hypergate: the
    /// entry awaits the player's pick.
    fn at_gate(catalog: &FakePilotCatalog) -> Session {
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
        let half = session.handling().max_speed / 2.0;
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
            gates: vec![gate_site(300, 130, (0.0, 0.0), HYPERGATE, &[0, -2], 0)],
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
            gates: vec![gate_site(300, 130, (0.0, 0.0), HYPERGATE, &[999], 0)],
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
            catalog.gates[0] = gate_site(300, 130, (0.0, 0.0), HYPERGATE, links, 0);
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
            gates: vec![gate_site(300, 130, (0.0, 0.0), HYPERGATE, &[0, -2], 0)],
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
    fn wormholes() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![
                (SystemId(130), vec![gate_landing(400, 0x2200)]),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            gates: vec![
                gate_site(400, 130, (0.0, 0.0), 0x2200, &[], 120),
                gate_site(410, 131, (-300.0, 40.0), WORMHOLE, &[], 0),
                gate_site(420, 132, (5.0, 5.0), WORMHOLE, &[410], 0),
            ],
            ..catalog()
        }
    }

    /// `catalog`'s session with L pressed twice over the wormhole.
    fn at_wormhole(catalog: &FakePilotCatalog) -> Session {
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
            gates: vec![gate_site(400, 130, (0.0, 0.0), WORMHOLE, &[], 0)],
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
}
