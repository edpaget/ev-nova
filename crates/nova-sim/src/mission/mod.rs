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
//! or a person in flight, with the values rolled for it: its destination
//! and return stellar, its cargo and tons, its pay and its deadline
//! ([`Offer`]).
//! Each is offered at one venue ([`Venue`]): the mission computer
//! (`AvailLoc` 0); outside it, at the bar or a shop (every other
//! `AvailLoc` but -1 and 2); or from a ship, by a person hailed or
//! boarded in flight (`AvailLoc` 2; see "From a ship"). Every mission
//! available is offered: there is no cap.
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
//!    and 2 outside it, and 2 alone from a ship (@0x9b195-0x9b1d5);
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
//!    legal record that high (@0x9b592-0x9b5a6), below 0 needs it that
//!    low (@0x9b5b4-0x9b5c8); -32000 needs the docked stellar dominated
//!    (its dominated byte +0x46, @0x9b539-0x9b560), -32001 any stellar of
//!    a visible system dominated, destroyed or not (+0x44 and +0x46 over
//!    every slot, @0x9b562-0x9b58b), both read through the
//!    always-dominated bit until rdm task planet-domination builds more;
//!    any value below them refuses (@0x9b567). The record is the one in
//!    the player's system, as the original reads `_playerRecord`; by the
//!    Bible's reading of [`RuleKey::LegalRecords`](crate::RuleKey::LegalRecords),
//!    the pilot's with that system's government, 0 in an independent
//!    system (see [`legal`](crate::legal));
//! 10. `AvailRating`: 0 or below passes, else the combat rating that high
//!     (@0x9b5d6-0x9b608);
//! 11. `AvailRandom`: above 99 passes, else the visit's roll for the
//!     mission no higher (@0x9b60b-0x9b63e; see "Draws");
//! 12. cargo space: with `Flags2` 0x0001, room for a mission's cargo
//!     (`_TotalMissionCargoSpace` @0xcffb: the ship's own holds less the
//!     mission cargo aboard and the goods the trader escorts' holds do not
//!     take) of at least `CargoQty`,
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
//!     a fixed stellar, or a random code whose draw has a stellar to take,
//!     the draw as
//!     [`mission_stellar_draw`](crate::RuleKey::MissionStellarDraw) reads
//!     it; with no origin, a stellar is distant by the engine's reading of
//!     [`mission_stellar_distance`](crate::RuleKey::MissionStellarDistance)
//!     only when every system at its place lists it
//!     (`_StellarsAreDistant` @0x9e20-0x9ee4), and by the other reading
//!     always; 1-127 and 2176-9998 name the origin, which availability
//!     has none of;
//! 17. not active (@0x9b8b7-0x9b8df).
//!
//! **From a ship** (`_offeringFromShip` set): `AvailStel` is not read
//! (@0x9b321), so the venue needs no stellar docked; an `AvailRecord` of
//! -32000 or below is read as any value below 0, the player's system's
//! record at or below it (@0x9b52e-0x9b537, @0x9b5b4-0x9b5c8),
//! domination left out; and a
//! `CargoQty` above 0 needs the room for it whatever `Flags2` says
//! (@0x9b641-0x9b687). Every other check runs as at a stellar, against
//! the player's system. At the other venues a visit with no stellar
//! docked refuses `AvailStel`.
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
//!    code chooses from the docked stellar ([`select_stellar`]), a
//!    random -2 drawn as
//!    [`mission_stellar_draw`](crate::RuleKey::MissionStellarDraw) reads:
//!    by the engine at any distance once the original's scan finds a
//!    distant stellar (`_SelectMissionStellar` @0x9a3ce-0x9a3db), by the
//!    other reading only a distant one, distance told as
//!    [`mission_stellar_distance`](crate::RuleKey::MissionStellarDistance)
//!    reads it;
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
//! ([`Offers::reoffer`], missions phase 2); an abort, a failure or a
//! success forgets the stellar rolled at ([`Offers::forget`]), so the
//! next landing rolls afresh; a jump's arrival forgets it too and draws
//! the table afresh before the system is set up ([`Offers::redraw`],
//! @0x6bfce, @0x6c040-0x6c066), for the offers from a ship to read until
//! the next landing; a landed `M` or `N` empties the offers
//! ([`Offers::empty`]). None of it is saved.
//!
//! A person's `LinkMission` is resolved as the person spawns, with no
//! availability check (`_SpawnPerson` @0x4104b-0x41072): from the
//! player's navigation target, or no stellar, as
//! [`person_mission_stellar`](crate::RuleKey::PersonMissionStellar)
//! reads. With no origin a random code draws among the candidates
//! distant from no origin, as
//! [`mission_stellar_distance`](crate::RuleKey::MissionStellarDistance)
//! reads it: by the engine those every system at their place lists (the
//! -1 reaches `_StellarsAreDistant`'s chain walk, @0xa151f-0xa157e,
//! @0x9cf4-0x9cf9), by the other reading every candidate; a -2 by the
//! engine's [`mission_stellar_draw`](crate::RuleKey::MissionStellarDraw)
//! tests only its gate's stellar so. Its values are kept by mission until another person linked to it spawns
//! (`_briefMissionInfo`); the landing roll never lists a mission offered
//! from a ship.
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
//!
//! # Active missions
//!
//! A mission accepted goes into the lowest free of 16 slots
//! ([`ActiveMissions`], `_currentMissions` 0x184688), holding its offer's
//! values, its days left and deadline (from the accept, not the roll),
//! whether its destination is reached, its ship goals met and its cargo
//! aboard, and how it failed, if it did ([`ActiveMission`], [`FailedBy`]). Its own fields
//! (scripts, `CanAbort`, `CompGovt`, `CompReward`, `DatePostInc`,
//! `Flags`) are read from its record when used. The slot is the record of
//! its story subscriptions (see the [`story`](crate::story) module's
//! "Mission lifecycle", which says what moves a mission). In order:
//!
//! - **Accept** (`_DoMissionAccept` @0xa1b11): the checks
//!   ([`accept_checks`]: a free slot, then the room for its cargo; see
//!   "Cargo");
//!   the slot filled; `OnAccept` (@0xa1da2); a `PayVal` below -50000 takes
//!   `-50000 - PayVal`, the cash floored at none ([`accept_charge`]); the
//!   destination reached when it has none, or, docked, when it is the
//!   stellar docked at, or, an offer from a ship accepted in flight, when
//!   it is the navigation target as
//!   [`person_mission_stellar`](crate::RuleKey::PersonMissionStellar)
//!   reads (@0xa1fc5-0xa1ff8); docked, the mission's offer
//!   dropped and the others kept only while available (@0xa1ffd-0xa2136);
//!   the auto-abort when docked, with `Flags` 0x0001, the destination
//!   reached, and no return stellar nor ship ([`auto_aborts_at_accept`],
//!   @0xa2139-0xa217b); its roll made 0 (@0xa2196).
//! - **Refuse**: `OnRefuse` (@0xa2f41), and the offer dropped; nothing
//!   forgotten.
//! - **The player's abort** (`_DoMissionInfoDialog` @0x9e56d): the checks
//!   ([`abort_checks`]: `CanAbort`); with `Flags` 0x0040, the record with
//!   `CompGovt` less 5 × `CompReward` ([`abort_penalty`]); then the end.
//! - **The end** (`_AbortMission` @0x99a92), shared by the abort, `A` and
//!   the failures: the ships leave (mission-ships phase 2); `OnAbort` for
//!   the abort and `A` only; the slot freed; the offers' stellar
//!   forgotten (@0x99bcf).
//! - **The auto-abort** (`_AutoAbortMission` @0x99bdc): `OnAbort`; the
//!   `DatePostInc` days; `Flags` 0x0008, a jump's fuel taken; `Flags2`
//!   0x0002, the pay; then the end, with no second `OnAbort`. A mission
//!   of `Flags` 0x0001 reaches it at accept (above), as its ship goals
//!   are met (@0x9ed87), and once per slot as its special ships spawn: at
//!   the system's setup (@0x4317e) and as its appear timer brings them in
//!   (@0x43ad4).
//! - **Success**, landing at the return with the destination reached and
//!   the goals met (`_DoMissionSuccess` @0xa03fc): the slot freed before
//!   `OnSuccess`; the `DatePostInc` days; the records ([`success_records`]:
//!   `CompGovt` + `CompReward`, its enemies less half and its allies plus
//!   half, each record and half summed and truncated); the pay
//!   ([`pay_effect`]); the offers' stellar forgotten; and, after the
//!   landing's last slot, the offers again, the table kept. No end: its
//!   ships stay.
//! - **Failure**, landing at the return of a failed mission
//!   (`_DoMissionFailure` @0xa0285): `OnFailure`; the record with
//!   `CompGovt` less half `CompReward`, toward none ([`failure_records`]);
//!   the end, with no `OnAbort`.
//! - **The deadline** (`_MissionObjectivesCheck` @0x9eba5-0x9ec61): with
//!   none of its days left, not failed, and the ship not docked, a mission
//!   fails at once (`_QuickMissionFailure` @0x99d1b), as its cargo
//!   jettisoned or found by a scan does (see "Cargo"): `OnFailure`,
//!   marked failed ([`FailedBy::Quick`]), the end when it is abortable,
//!   and the offers' stellar forgotten. No record changes. An unabortable one
//!   stays active and failed; at its return the original fails it again,
//!   running `OnFailure` a second time and applying the failure records.
//!   The Rulebook key [`repeat_mission_failure`](crate::RuleKey::RepeatMissionFailure)
//!   defaults to that; its other reading frees it there without running
//!   `OnFailure` again, the failure records still applied once. A mission
//!   failed by `F` ran nothing, so it runs `OnFailure` once at its return
//!   under both.
//! - **`S`, `A` and `F`** (`_EvalSetExp` @0x152f3-0x15418): `S` resolves the
//!   mission from the stellar docked at, or in flight the first stellar
//!   the player's system lists, and accepts it, checking neither its
//!   availability nor whether it is active: a mission active already
//!   takes another slot, unless the Rulebook key
//!   [`start_active_mission`](crate::RuleKey::StartActiveMission) reads
//!   otherwise. `A` ends every slot holding the mission, running
//!   `OnAbort`, whatever its `CanAbort`. `F` marks each failed, running
//!   nothing.
//!
//! A pay (`_ApplyMissionPay` @0x98326) above 0 is cash; -10000 to -39999
//! clean the negative records of a government, its allies or those
//! sharing a class with it; -40001 to -40099 fine a percent of the cash
//! ([`fined`]). Each record is the pilot's with a government, as the
//! [`legal`](crate::legal) module keeps them, held within
//! [`RECORD_LIMIT`](crate::legal::RECORD_LIMIT), and wider than the
//! original's 16 bits.
//!
//! # Cargo
//!
//! A mission carries `CargoType` 0-999 or nothing, of the tons its offer
//! resolved: a commodity, a `jünk`, or another of `STR#` 4000's cargoes,
//! passengers among them (type 6, "*passengers"). Its cargo is aboard or
//! not ([`ActiveMission::aboard`], the slot's +0x35), and a freed slot's
//! cargo goes with it. It is never a good held, so the exchange never
//! lists it and it cannot be sold ([`cargo`]).
//!
//! - **Room.** An accept, whatever its `PickupMode`, and a pickup each
//!   need, for tons above none, the ship's own holds (`_ShipTotalHolds`
//!   @0x5cb3) and then the room for a mission's cargo
//!   (`_TotalMissionCargoSpace` @0xcffb) to take the tons
//!   ([`accept_checks`], [`pickup_checks`]; `STR#` 2002 #355-#358).
//!   Passengers are cargo like any other: nothing counts berths, so the
//!   berths are the cargo space.
//! - **The hold.** The cargo aboard counts as `_TotalCargo` (@0x4a0b)
//!   counts it, whatever its type: against the exchange's free space, the
//!   outfitter's for a mass expansion and the plunder's room. The room for
//!   a mission's cargo keeps it in the ship alone: the goods held spill to
//!   the trader escorts' holds, the mission cargo never does
//!   (@0xd0c7-0xd0eb).
//! - **Loading**, by `PickupMode`: 0 as it is accepted, after `BriefText`,
//!   showing `LoadCargText` (@0xa1f28-0xa1fbf); 1 at the destination
//!   ([`picks_up_at_destination`], `_MissionLandCargoCheck`
//!   @0x9ee0e-0x9ef11), a refused pickup leaving the destination
//!   unreached so that the next landing there tries again; 2 on boarding
//!   its special ship (`_HandlePlayerBoardAttempt` @0x652ee-0x653ed,
//!   mission-ships phase 2); any other never. Any other landing at the
//!   destination reaches it.
//! - **Unloading**, by `DropOffMode` ([`drops_off`]): 0 at the
//!   destination, 1 at the return once its goals are met or it has no
//!   `ShipGoal`, each showing `DumpCargoText`, before a success's
//!   `CompText`; any other never, the cargo going with its slot. A mission
//!   with no cargo type is loaded too, for its `DumpCargoText`, and one
//!   loaded and unloaded at its destination (1 and 0) is loaded and
//!   unloaded on the same landing.
//! - **Jettison** ([`jettison`], `_JettisonCargo` @0x3dffd): the player's
//!   Command+K jettisons every good and, in slot order, the typed cargo
//!   aboard of every abortable mission, which fails at once
//!   (`_QuickMissionFailure` @0x99d1b); an unabortable mission keeps its
//!   cargo. Command+Option+K keeps every mission's, so the original
//!   already leaves keeping mission cargo to the player, and no Rulebook
//!   key does. In flight the player is told "Mission failed." unless the
//!   mission has `Flags` 0x0400, else "Cargo jettisoned." or "Non-mission
//!   cargo jettisoned."; landed, nothing. `Flags` 0x0080 is never read,
//!   as the Bible says. In flight it also sounds `snd ` 154
//!   (@0x3e31a-0x3e33d) and drops cargo boxes behind the player and its
//!   trader escorts (see [`cargo_box`](crate::cargo_box)).
//! - **Scan** ([`scan`], `_ScanPlayer` @0x7e46e): a warship or an
//!   interceptor that has looked the player over scans it, when its
//!   government has a `SmugPenalty`, within reach, on a draw of
//!   `Rand(100)` no higher than 75 (see [`scan`](crate::scan) for the
//!   gates and the jump-timing check). The first slot whose typed cargo
//!   aboard shares a `ScanMask` bit with the government's decides: with
//!   `Flags` 0x0020, and not failed already, the mission fails at once,
//!   sounding `snd ` 151 unless it is hidden; otherwise the player is
//!   caught smuggling and fined the government's `ScanFine` (below none,
//!   that percent of the cash in hundreds, at least 1), the cash floored
//!   at none, sounding `snd ` 154. Either way the scanner attacks the
//!   player. No legal record changes. Only interceptors inspect here,
//!   where the original's warships may scan too. The scan then goes on
//!   to the player's illegal `jünk` and outfits (see
//!   [`scan`](crate::scan)'s "Contraband").
//!
//! Not built here: the player info dialog's Jettison button, and the
//! plunder of the player's cargo, which waits for NPC boarding.
//!
//! # Screens
//!
//! What the mission computer, the offer dialog and the mission info
//! screen show is decided here and in the session ([`screens`]); the
//! view draws it. Each mission's name, offer text (`dësc` 4000 + its
//! index), `BriefText`, `QuickBrief`, `CompText`, `FailText`,
//! `RefuseText` and button labels are read once, as the session starts
//! ([`MissionTexts`](crate::MissionTexts)); a missing `dësc` is no text.
//!
//! Once both venues' offers are resolved, each list is put in the order
//! it shows ([`display_order`], `_RandomizeMissionData`
//! @0xa1842-0xa197d). The original means to list them by descending
//! `DispWeight`, but its first threshold reads the weight of index -1,
//! the zero padding before `_missionAvailData` (@0xa18bb): so every offer
//! weighing 0 or more keeps its index order and comes first, and the
//! rest follow by descending weight, ties in index order. Every stock
//! weight is 0 or more, so the stock lists keep index order. The Rulebook
//! key [`mission_order`](crate::RuleKey::MissionOrder) defaults to that;
//! its other reading is the Bible's full descending sort.
//!
//! The mission computer lists every offer at the computer, each name and
//! offer text filled for its offer, and opens only with a slot free and
//! an offer still available (`_DoMissionDialog` @0xa37e9-0xa3956, saying
//! why otherwise: [`screens`]' strings). Outside it, each venue offers
//! the first offer at its `AvailLoc` that is not suppressed and is still
//! available, on a timer of [`screens::FIRST_OFFER_TICKS`] and then
//! [`offer_gap`] (`_OfferMissionFromPort` @0xa369e); one that cannot be
//! refused (`Flags` 0x0004) with no offer text is accepted at once, and
//! one whose accept is refused is suppressed until another venue but the
//! spaceport is entered. The offer dialog's labels are the mission's own
//! when each starts with a letter ([`offer_labels`]). The mission info
//! screen lists the active missions lacking `Flags` 0x0400, each with its
//! `QuickBrief`, which shows a deadline through `<DL>`; its abort is
//! [`abort_checks`]'. Each text shown is filled through the session.
//!
//! A landing reaches a mission's destination or return at the stellar
//! of its ID and, by [`RuleKey::MissionStellarIdentity`](crate::RuleKey::MissionStellarIdentity)'s
//! engine reading, at any other stellar with the same `xPos`/`yPos` and
//! exactly the same name, in any system (`_StellarsAreIdentical`
//! @0xab35-0xac90, [`StellarMarks::identical`]): a storyline's variant
//! of a stellar stands in for it, as Tre'ar Zalom 448 does for 227 once
//! b1502 swaps Tri'niphu 212 for 657. Only the cargo check
//! (@0x9edfd, @0x9f01d) and the success and failure at the return
//! (@0xa1a37) match so; the accept's "reached here" (@0xa1fd8) and the
//! success check's "the destination is the return" (@0xa1a4e) compare
//! raw IDs.

pub mod active;
pub mod aux;
pub mod cargo;
pub mod goals;
pub mod identity;
pub mod offer;
pub mod screens;
pub mod select;
pub mod ships;

use crate::catalog::{GovtId, LandingSite, MissionId, MissionRecord, ShipId, StellarId, SystemId};
use crate::control::Gate;
use crate::date::GameDate;
use crate::rulebook::Rulebook;
use crate::rules::{Check, Checks, Context};
use crate::wares;

pub use self::active::{
    AbortFacts, AbortRefusal, AcceptError, AcceptFacts, AcceptRefusal, ActiveMission,
    ActiveMissions, Among, FailedBy, PayEffect, SLOTS, abort_checks, abort_penalty, accept_charge,
    accept_checks, auto_aborts_at_accept, cleaned, failure_records, fined, pay_effect,
    success_records,
};
pub use self::cargo::{
    JettisonMessage, Jettisoned, PickupFacts, PickupRefusal, Room, ScanOutcome, drops_off,
    jettison, picks_up_at_destination, pickup_checks, scan,
};
pub use self::goals::{
    AssistLine, Departure, GoalCheck, GoalFacts, LossFacts, ShipGoal, assists, check_goals,
    departure, fails_on_destroyed, fails_on_disabled, goals_met_moment, held_disabled, met_before,
};
pub use self::identity::StellarMarks;
pub use self::offer::{Offer, Offers, ROLLED, RollTable, Stop, resolve};
pub use self::screens::{
    OfferLabels, OfferPrompt, VenueOffer, display_order, offer_gap, offer_labels,
};
pub use self::select::{
    Kind, Placed, Stellar, StellarCode, World, placements, select_stellar, stellars_of,
};
pub use self::ships::{ShipSystem, SpecialShips};

/// Where a mission is offered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Venue {
    /// The mission computer: `AvailLoc` 0.
    Computer,
    /// Outside it, at the bar or a shop: every other `AvailLoc` but -1,
    /// and 2, which is offered only from a ship.
    Outside,
    /// From a ship in flight, a person hailed or boarded: `AvailLoc` 2
    /// (`_offeringFromShip`, @0x9b19f-0x9b1d5).
    Ship,
}

impl Venue {
    /// Both venues, in the order a roll lists them: the mission computer's
    /// pass, then the others' (@0xa1784).
    pub const ORDER: [Self; 2] = [Self::Computer, Self::Outside];

    /// The venue a mission of `avail_loc` is offered at.
    #[must_use]
    pub fn of(avail_loc: i16) -> Self {
        match avail_loc {
            COMPUTER => Self::Computer,
            SHIP => Self::Ship,
            _ => Self::Outside,
        }
    }

    /// Whether this venue offers a mission of `avail_loc`.
    fn offers(self, avail_loc: i16) -> bool {
        match self {
            Self::Computer => avail_loc == COMPUTER,
            Self::Outside => !matches!(avail_loc, COMPUTER | NOWHERE | SHIP),
            Self::Ship => avail_loc == SHIP,
        }
    }
}

/// `AvailLoc`: the mission computer.
const COMPUTER: i16 = 0;
/// `AvailLoc`: offered from a ship.
const SHIP: i16 = 2;
/// `AvailLoc`: never offered.
const NOWHERE: i16 = -1;

/// `AvailStel` at or below which a mission is never offered: an empty
/// `mïsn` slot's.
const UNOFFERED: i16 = -32000;
/// `AvailRecord`: the docked stellar dominated (@0x9b539-0x9b560).
const DOMINATED_HERE: i16 = -32000;
/// `AvailRecord`: any stellar of a visible system dominated
/// (@0x9b562-0x9b58b).
const DOMINATED_ANY: i16 = -32001;
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
    /// The legal record in the player's system (@0x9b592-0x9b5c4), as
    /// [`RuleKey::LegalRecords`](crate::RuleKey::LegalRecords) reads it:
    /// by the Bible, the record with its government, 0 in an independent
    /// system.
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
    /// holds less the mission cargo aboard and the goods the escorts do
    /// not take; below 0 when it holds more.
    pub free_cargo: i64,
    /// The pilot's credits.
    pub cash: i64,
}

/// One visit to a docked stellar, or the flight in a system: everything
/// availability and the resolution read but the mission, its venue and
/// the roll table.
#[derive(Clone, Copy, Debug)]
pub struct Visit<'a> {
    /// The stellar docked at, overlaid: none in flight.
    pub docked: Option<LandingSite>,
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
    // From a ship it is not read (@0x9b321).
    if facts.venue == Venue::Ship {
        return Ok(());
    }
    let visit = facts.visit;
    let Some(docked) = visit.docked else {
        return Err(Refusal::AvailStel);
    };
    let code = facts.mission.avail_stel;
    let govts = visit.world.govts;
    let here = docked.govt;
    let takes = match code {
        -1 => true,
        128..=4999 => docked.id == StellarId(code),
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
        // From a ship, any value below 0 needs the record that low
        // (@0x9b52e-0x9b537, @0x9b5b4).
        ..=DOMINATED_HERE if facts.venue == Venue::Ship => record <= wanted,
        DOMINATED_HERE => facts
            .visit
            .docked
            .is_some_and(|site| dominated(site.flags2)),
        DOMINATED_ANY => facts
            .visit
            .world
            .stellars
            .iter()
            .any(|stellar| dominated(stellar.flags2)),
        // Below -32001 at a stellar: never (@0x9b567).
        ..DOMINATED_ANY => false,
        _ => record <= wanted,
    };
    pass(takes, Refusal::AvailRecord)
}

/// Whether a stellar of `flags2` is dominated: its always-dominated bit
/// ([`DOMINATED`](crate::market::DOMINATED)), from which the original
/// sets the stellar's dominated byte (+0x46) at `_ResetPlayerRecord`
/// (@0x1dbdb-0x1dbdf) and `_HandleStellarSystemVisibility`
/// (@0x32ce5-0x32ceb). That bit is the engine's whole domination state
/// until rdm task planet-domination feeds the state it builds through
/// here.
fn dominated(flags2: u16) -> bool {
    flags2 & crate::market::DOMINATED != 0
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
    // From a ship, any tons above none ask for the room, whatever
    // `Flags2` says (@0x9b641-0x9b687).
    let asks = if facts.venue == Venue::Ship {
        mission.cargo_qty > 0
    } else {
        mission.flags2 & NEEDS_ROOM != 0
    };
    pass(
        !asks || facts.visit.player.free_cargo >= i64::from(mission.cargo_qty),
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
