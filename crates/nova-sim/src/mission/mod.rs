//! Mission offers: which missions a stellar offers on a visit
//! ([`availability`]), the values each offer is rolled with ([`resolve`])
//! and the per-visit state that keeps them ([`Offers`]).
//!
//! These are the original `EV Nova` engine's rules, read from its
//! executable (each address below is in `EV Nova.app/Contents/MacOS/EV
//! Nova`). Where an offer goes in a mission's life, and what forgets the
//! offers, is the [`story`](crate::story) module's "Mission lifecycle".
//!
//! # An offer
//!
//! An offer is a `mïsn` a stellar puts before the player on one visit,
//! with the values the visit rolled for it: its destination and return
//! stellar, its cargo and tons, its pay and its deadline ([`Offer`]).
//! Each is offered at one venue ([`Venue`]): the mission computer
//! (`AvailLoc` 0), or outside it, at the bar or a shop (every other
//! `AvailLoc` but 2, which is offered only from a ship, missions phase
//! 6). Every mission available is offered: there is no cap.
//!
//! # Availability (`_IsMissionAvailable` @0x9b152)
//!
//! A mission is available at the docked stellar when every check of
//! [`availability`] passes, in this order (the original evaluates the
//! middle ten all, then ANDs them @0x9b81d; the first refusal is the
//! same):
//!
//! 1. offered at all: `AvailStel` above -32000, as an empty `mïsn` slot is
//!    not (@0x9b179);
//! 2. `AvailRandom` above 0 (@0x9b184);
//! 3. the venue: `AvailLoc` 0 at the mission computer, any other but -1
//!    and 2 outside it (@0x9b195-0x9b1d5);
//! 4. when `AvailStel` is no stellar (128-2175), a `TravelStel` or
//!    `ReturnStel` naming a stellar (128-2175) whose system is at the
//!    player's system's place refuses (@0x9b1db-0x9b2c0), a rule the
//!    Bible does not give;
//! 5. `Flags` 0x0008 needs a jump's fuel, 100 (@0x9b2c6-0x9b2ec);
//! 6. `AvailStel` (@0x9b307-0x9b4ab): -1 anywhere; 128-4999 that stellar;
//!    5000-9998 a stellar of a system whose own `sÿst` lists system
//!    `AvailStel` - 5000 + 128; then by the docked stellar's own
//!    government, `gövt` index g = `AvailStel` less the range's base:
//!    9999-14999 governed by g (9999 independent), 15000-19999 an ally of
//!    g, 20000-24999 not g, 25000-29999 an enemy of g, 30000-30999
//!    sharing a class with g, 31000-31999 sharing none; the ally, enemy
//!    and no-shared-class ranges refuse an independent stellar. Any other
//!    value refuses;
//! 7. `AvailBits`, through [`ControlBits`](crate::ControlBits): one that
//!    did not parse never holds;
//! 8. `Require`: every bit the ship and its outfits contribute
//!    ([`wares::requirement_met`]);
//! 9. `AvailRecord` (@0x9b519-0x9b5ce): 0 passes, above 0 needs the
//!    legal record that high, below 0 needs it that low; -32000 (the
//!    docked stellar dominated) and -32001 (any stellar dominated) refuse
//!    until domination is built (rdm task planet-domination), and so does
//!    any value below them. The record is the pilot's with the docked
//!    stellar's system's government, as the original keeps one per
//!    system;
//! 10. `AvailRating`: 0 or below passes, else the combat rating that high
//!     (@0x9b5d6-0x9b608);
//! 11. `AvailRandom`: above 99 passes, else the visit's roll for the
//!     mission no higher (@0x9b60b-0x9b63e; see "Draws");
//! 12. cargo space: with `Flags2` 0x0001, room for a mission's cargo
//!     (`_TotalMissionCargoSpace` @0xcffb: the ship's own holds less the
//!     cargo the trader escorts' holds do not take) of at least `CargoQty`,
//!     raw (@0x9b641-0x9b687), so a range below 0 passes;
//! 13. `AvailShipType` (@0x9b6ae-0x9b77f): 128-896 that ship; 1128-1896
//!     not ship `AvailShipType` - 1000; 2128-2384 a ship whose combat or
//!     attribute government is `gövt` `AvailShipType` - 2000; 3128-3384
//!     neither; any other value passes;
//! 14. `Flags` 0x2000 needs the ship's `InherentAI` above 2, 0x4000 below
//!     3 (@0x9b783-0x9b7ee);
//! 15. a `PayVal` below -50000 takes cash, so needs -50000 - `PayVal` of
//!     it (@0x9b7f1-0x9b819);
//! 16. a `TravelStel`, then a `ReturnStel`, of -2, -3 or above 0 must find
//!     a stellar with no origin ([`select::selectable`], @0x9b832-0x9b8b5):
//!     a fixed stellar, or a random code with a candidate anywhere; 1-127
//!     and 2176-9998 name the origin, which availability has none of;
//! 17. not active (@0x9b8b7-0x9b8df).
//!
//! # Rolling and resolving (`_RandomizeMissionData` @0xa1756)
//!
//! A landing at a stellar other than the one the offers were last
//! rolled at rolls them (`_HandlePlayerDockRequest` @0x66d2b-0x66d77;
//! [`Offers::roll`]): it draws the visit's table, then lists the mission
//! computer's offers and then the others', each in ascending ID order,
//! every mission not active that is available at that venue
//! (@0xa17c4-0xa17ec), and resolves each listed one in that order
//! (`_RandomizeOneMission` @0xa14d1, [`resolve`]):
//!
//! 1. its destination: none for `TravelStel` -1, else the stellar the
//!    code chooses from the docked stellar ([`select_stellar`]);
//! 2. its return: the destination for `ReturnStel` -1, else the stellar
//!    the code chooses, never the destination;
//! 3. its cargo: `CargoType` 0-999 as it is, 1000 a `Rand(6)` commodity,
//!    any other none;
//! 4. its tons: `CargoQty` 0 or more as it is, -1 none, and q of -2 or
//!    below `Rand(|q|) + |q| / 2`, drawn whatever the cargo
//!    (`_SelectMissionCargoQty` @0x98191);
//! 5. its pay, `PayVal` raw;
//! 6. its deadline: today plus `TimeLimit` days, when that is above 0.
//!
//! A mission's success re-offers without drawing the table again
//! ([`Offers::reoffer`], missions phase 2); a jump's arrival, an abort,
//! a failure or a success forgets the stellar rolled at
//! ([`Offers::forget`]), so the next landing rolls afresh; a landed `M`
//! or `N` empties the offers ([`Offers::empty`]). None of it is saved.
//!
//! # Draws
//!
//! The visit's table is 1000 `Rand(100) + 1` rolls, drawn in index order
//! as the offers are rolled ([`RollTable::draw`]), and kept for the visit:
//! `AvailRandom` reads it as a fact, so checking a mission's availability
//! again draws nothing, and every check is plain. The original's own
//! destination check draws a stellar and throws it away; here it only
//! asks whether one exists. Resolving draws, in its order: one
//! `below(n)` over the candidates for each random stellar, a `roll(6)`
//! for a random cargo and a `below(|q|)` for a ranged quantity. The
//! engine keeps no other draw of the original's.

pub mod active;
pub mod offer;
pub mod select;

use crate::catalog::{GovtId, LandingSite, MissionId, MissionRecord, ShipId, StellarId, SystemId};
use crate::control::Gate;
use crate::date::GameDate;
use crate::rulebook::Rulebook;
use crate::rules::{Check, Checks, Context};
use crate::wares;

pub use self::active::{
    AbortFacts, AbortRefusal, AcceptFacts, AcceptRefusal, ActiveMission, ActiveMissions, Among,
    FailedBy, PayEffect, SLOTS, abort_checks, abort_penalty, accept_charge, accept_checks,
    auto_aborts_at_accept, failure_records, fined, goals_met_at_accept, pay_effect,
    success_records,
};
pub use self::offer::{Offer, Offers, ROLLED, RollTable, Stop, resolve};
pub use self::select::{Kind, Stellar, StellarCode, World, select_stellar, stellars_of};

/// Where a mission is offered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Venue {
    /// The mission computer: `AvailLoc` 0.
    Computer,
    /// Outside it, at the bar or a shop: every other `AvailLoc` but -1,
    /// and 2, which is offered only from a ship.
    Outside,
}

impl Venue {
    /// Both venues, in the order a roll lists them: the mission computer's
    /// pass, then the others' (@0xa1784).
    pub const ORDER: [Self; 2] = [Self::Computer, Self::Outside];

    /// The venue a mission of `avail_loc` is offered at.
    #[must_use]
    pub fn of(avail_loc: i16) -> Self {
        if avail_loc == COMPUTER {
            Self::Computer
        } else {
            Self::Outside
        }
    }

    /// Whether this venue offers a mission of `avail_loc`.
    fn offers(self, avail_loc: i16) -> bool {
        match self {
            Self::Computer => avail_loc == COMPUTER,
            Self::Outside => !matches!(avail_loc, COMPUTER | NOWHERE | SHIP),
        }
    }
}

/// `AvailLoc`: the mission computer.
const COMPUTER: i16 = 0;
/// `AvailLoc`: offered from a ship (missions phase 6).
const SHIP: i16 = 2;
/// `AvailLoc`: never offered.
const NOWHERE: i16 = -1;

/// `AvailStel` at or below which a mission is never offered: an empty
/// `mïsn` slot's.
const UNOFFERED: i16 = -32000;
/// `AvailRecord`: the docked stellar dominated.
const DOMINATED_HERE: i16 = -32000;
/// `AvailRandom` above which no roll is read.
const ALWAYS: i16 = 99;
/// `Flags`: needs a jump's fuel.
const NEEDS_FUEL: u16 = 0x0008;
/// A jump's fuel.
const JUMP_FUEL: f32 = 100.0;
/// `Flags`: needs a warship (`InherentAI` above 2).
const WARSHIPS_ONLY: u16 = 0x2000;
/// `Flags`: needs a trader (`InherentAI` below 3).
const TRADERS_ONLY: u16 = 0x4000;
/// The `InherentAI` a warship is above.
const TRADER_AI: i16 = 2;
/// `Flags2`: needs room for the cargo.
const NEEDS_ROOM: u16 = 0x0001;
/// `PayVal` below which the mission takes cash.
const TAKES_CASH: i32 = -50000;

/// Why a mission is not offered: one reason for each check of
/// [`availability`], named by the field it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// It is never offered: an `AvailStel` of -32000 or below.
    Unoffered,
    /// Its `AvailLoc` is not this venue.
    AvailLoc,
    /// Its fixed destination or return is in the player's system's place.
    GoesHere,
    /// `Flags` 0x0008: the ship has less than a jump's fuel.
    Fuel,
    /// Its `AvailStel` does not take the docked stellar.
    AvailStel,
    /// Its `AvailBits` does not hold.
    AvailBits,
    /// Its `Require` is not met.
    Require,
    /// Its `AvailRecord` does not take the legal record.
    AvailRecord,
    /// The combat rating is below its `AvailRating`.
    AvailRating,
    /// Its `AvailRandom` is 0 or below, or the visit's roll is above it.
    AvailRandom,
    /// `Flags2` 0x0001: there is no room for its cargo.
    CargoSpace,
    /// Its `AvailShipType` does not take the ship.
    AvailShipType,
    /// `Flags` 0x2000 or 0x4000: the ship's `InherentAI` is not the one
    /// asked.
    InherentAi,
    /// Its `PayVal` takes more cash than the pilot has.
    Cash,
    /// Its `TravelStel` finds no stellar.
    TravelStel,
    /// Its `ReturnStel` finds no stellar.
    ReturnStel,
    /// It is active already.
    Active,
}

/// What the checks read about the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Player {
    /// The legal record with the docked stellar's system's government, 0
    /// in an independent system.
    pub record: i16,
    /// The combat rating.
    pub rating: i64,
    /// The ship's class.
    pub ship: ShipId,
    /// Its class's `InherentAI`.
    pub inherent_ai: i16,
    /// The government whose attributes its class takes on.
    pub inherent_govt: Option<GovtId>,
    /// The government its class fights as.
    pub combat_govt: Option<GovtId>,
    /// The `Contribute` bits of the ship and its outfits.
    pub contributed: u64,
    /// Its fuel.
    pub fuel: f32,
    /// The room for a mission's cargo, in tons, as
    /// `_TotalMissionCargoSpace` (@0xcffb) reckons it: the ship's own
    /// holds less the cargo the escorts do not take; below 0 when it holds
    /// more.
    pub free_cargo: i64,
    /// The pilot's credits.
    pub cash: i64,
}

/// One visit to a docked stellar: everything availability and the
/// resolution read but the mission, its venue and the roll table.
#[derive(Clone, Copy, Debug)]
pub struct Visit<'a> {
    /// The stellar docked at, overlaid.
    pub docked: LandingSite,
    /// The player's system, the one the stellar is in.
    pub system: SystemId,
    /// The galaxy the stellars are chosen from.
    pub world: World<'a>,
    /// The player.
    pub player: Player,
    /// The missions active, in slot order.
    pub active: &'a [MissionId],
    /// The control-bit test `AvailBits` goes through.
    pub gate: Gate<'a>,
    /// The rule set.
    pub rules: &'a Rulebook,
    /// Today.
    pub today: GameDate,
}

/// The context [`availability`] reads: one mission at one venue, on one
/// visit, with its roll table.
#[derive(Clone, Copy, Debug)]
pub struct AvailFacts<'a> {
    /// The visit.
    pub visit: &'a Visit<'a>,
    /// The mission.
    pub mission: &'a MissionRecord,
    /// The venue asked about.
    pub venue: Venue,
    /// The visit's rolls.
    pub rolls: &'a RollTable,
}

impl Context for AvailFacts<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.visit.rules
    }
}

/// `Err(refusal)` unless `ok`.
fn pass(ok: bool, refusal: Refusal) -> Result<(), Refusal> {
    if ok { Ok(()) } else { Err(refusal) }
}

/// Whether `code` names a stellar: 128-2175.
fn names_stellar(code: i16) -> bool {
    (128..=2175).contains(&code)
}

/// Whether availability asks `code` to find a stellar: -2, -3 or above 0.
fn finds_stellar(code: i16) -> bool {
    matches!(code, -3 | -2) || code > 0
}

/// The government a code's government index from `base` names: `gövt`
/// index + 128, independent below 0.
fn govt_at(code: i16, base: i16) -> Option<GovtId> {
    let index = code - base;
    (index >= 0).then(|| GovtId(index + 128))
}

/// The checks that say whether a mission is offered (see the module
/// docs): every one plain, so none draws.
#[must_use]
pub fn availability<'a>() -> Checks<AvailFacts<'a>, Refusal> {
    [
        Check::plain("offered", "@0x9b179", offered),
        Check::plain("random above 0", "@0x9b184", random_above_0),
        Check::plain("venue", "@0x9b195-0x9b1d5", venue),
        Check::plain("not going here", "@0x9b1db-0x9b2c0", not_going_here),
        Check::plain("fuel", "@0x9b2c6-0x9b2ec", fuel),
        Check::plain("AvailStel", "@0x9b307-0x9b4ab", avail_stel),
        Check::plain("AvailBits", "@0x9b4af-0x9b4f5", avail_bits),
        Check::plain("Require", "_PlayerMeetsRequirements @0x9b50b", require),
        Check::plain("AvailRecord", "@0x9b519-0x9b5ce", avail_record),
        Check::plain("AvailRating", "@0x9b5d6-0x9b608", avail_rating),
        Check::plain("AvailRandom", "@0x9b60b-0x9b63e", avail_random),
        Check::plain("cargo space", "@0x9b641-0x9b687", cargo_space),
        Check::plain("AvailShipType", "@0x9b6ae-0x9b77f", avail_ship_type),
        Check::plain("InherentAI", "@0x9b783-0x9b7ee", inherent_ai),
        Check::plain("PayVal cash", "@0x9b7f1-0x9b819", pay_cash),
        Check::plain("stellars found", "@0x9b832-0x9b8b5", stellars_found),
        Check::plain("not active", "@0x9b8b7-0x9b8df", not_active),
    ]
    .into()
}

fn offered(facts: &AvailFacts) -> Result<(), Refusal> {
    pass(facts.mission.avail_stel > UNOFFERED, Refusal::Unoffered)
}

fn random_above_0(facts: &AvailFacts) -> Result<(), Refusal> {
    pass(facts.mission.avail_random > 0, Refusal::AvailRandom)
}

fn venue(facts: &AvailFacts) -> Result<(), Refusal> {
    pass(
        facts.venue.offers(facts.mission.avail_loc),
        Refusal::AvailLoc,
    )
}

fn not_going_here(facts: &AvailFacts) -> Result<(), Refusal> {
    let mission = facts.mission;
    if names_stellar(mission.avail_stel) {
        return Ok(());
    }
    let visit = facts.visit;
    let here = |code: i16| {
        names_stellar(code)
            && visit
                .world
                .system_of(StellarId(code))
                .is_some_and(|system| visit.world.map.places().together(system, visit.system))
    };
    pass(
        !here(mission.travel_stel) && !here(mission.return_stel),
        Refusal::GoesHere,
    )
}

fn fuel(facts: &AvailFacts) -> Result<(), Refusal> {
    pass(
        facts.mission.flags & NEEDS_FUEL == 0 || facts.visit.player.fuel >= JUMP_FUEL,
        Refusal::Fuel,
    )
}

fn avail_stel(facts: &AvailFacts) -> Result<(), Refusal> {
    let visit = facts.visit;
    let code = facts.mission.avail_stel;
    let govts = visit.world.govts;
    let here = visit.docked.govt;
    let takes = match code {
        -1 => true,
        128..=4999 => visit.docked.id == StellarId(code),
        5000..=9998 => visit
            .world
            .map
            .listed_links(visit.system)
            .contains(&SystemId(code - 5000 + 128)),
        9999..=14999 => here == govt_at(code, 10000),
        15000..=19999 => here.is_some() && govts.allies(govt_at(code, 15000), here),
        20000..=24999 => here != govt_at(code, 20000),
        25000..=29999 => here.is_some() && govts.enemies(govt_at(code, 25000), here),
        30000..=30999 => govts.shared_class(govt_at(code, 30000), here),
        31000..=31999 => here.is_some() && !govts.shared_class(govt_at(code, 31000), here),
        _ => false,
    };
    pass(takes, Refusal::AvailStel)
}

fn avail_bits(facts: &AvailFacts) -> Result<(), Refusal> {
    pass(
        facts.visit.gate.allows(&facts.mission.avail_bits),
        Refusal::AvailBits,
    )
}

fn require(facts: &AvailFacts) -> Result<(), Refusal> {
    pass(
        wares::requirement_met(facts.mission.require, facts.visit.player.contributed),
        Refusal::Require,
    )
}

fn avail_record(facts: &AvailFacts) -> Result<(), Refusal> {
    let wanted = facts.mission.avail_record;
    let record = facts.visit.player.record;
    let takes = match wanted {
        0 => true,
        1.. => record >= wanted,
        // -32000, -32001 and below: domination, which is not built.
        ..=DOMINATED_HERE => false,
        _ => record <= wanted,
    };
    pass(takes, Refusal::AvailRecord)
}

fn avail_rating(facts: &AvailFacts) -> Result<(), Refusal> {
    let wanted = facts.mission.avail_rating;
    pass(
        wanted <= 0 || facts.visit.player.rating >= i64::from(wanted),
        Refusal::AvailRating,
    )
}

fn avail_random(facts: &AvailFacts) -> Result<(), Refusal> {
    let percent = facts.mission.avail_random;
    let rolled = || {
        facts
            .rolls
            .roll(facts.mission.id)
            .is_some_and(|roll| i32::from(roll) <= i32::from(percent))
    };
    pass(percent > ALWAYS || rolled(), Refusal::AvailRandom)
}

fn cargo_space(facts: &AvailFacts) -> Result<(), Refusal> {
    let mission = facts.mission;
    pass(
        mission.flags2 & NEEDS_ROOM == 0
            || facts.visit.player.free_cargo >= i64::from(mission.cargo_qty),
        Refusal::CargoSpace,
    )
}

fn avail_ship_type(facts: &AvailFacts) -> Result<(), Refusal> {
    let player = facts.visit.player;
    let code = facts.mission.avail_ship_type;
    let flies_as = |base: i16| {
        let govt = Some(GovtId(code - base + 128));
        player.combat_govt == govt || player.inherent_govt == govt
    };
    let takes = match code {
        128..=896 => player.ship == ShipId(code),
        1128..=1896 => player.ship != ShipId(code - 1000),
        2128..=2384 => flies_as(2128),
        3128..=3384 => !flies_as(3128),
        _ => true,
    };
    pass(takes, Refusal::AvailShipType)
}

fn inherent_ai(facts: &AvailFacts) -> Result<(), Refusal> {
    let flags = facts.mission.flags;
    let ai = facts.visit.player.inherent_ai;
    let warship = flags & WARSHIPS_ONLY == 0 || ai > TRADER_AI;
    let trader = flags & TRADERS_ONLY == 0 || ai <= TRADER_AI;
    pass(warship && trader, Refusal::InherentAi)
}

fn pay_cash(facts: &AvailFacts) -> Result<(), Refusal> {
    let pay = facts.mission.pay_val;
    pass(
        pay >= TAKES_CASH || facts.visit.player.cash >= i64::from(TAKES_CASH) - i64::from(pay),
        Refusal::Cash,
    )
}

fn stellars_found(facts: &AvailFacts) -> Result<(), Refusal> {
    let mission = facts.mission;
    let world = &facts.visit.world;
    let found = |code: i16| !finds_stellar(code) || select::selectable(code, world);
    pass(found(mission.travel_stel), Refusal::TravelStel)?;
    pass(found(mission.return_stel), Refusal::ReturnStel)
}

fn not_active(facts: &AvailFacts) -> Result<(), Refusal> {
    pass(
        !facts.visit.active.contains(&facts.mission.id),
        Refusal::Active,
    )
}

#[cfg(test)]
mod tests;
