//! Which NPCs appear, and where: a system's initial population, the
//! arrivals over time, and the fleets, rolled on the [`Chance`] port.
//!
//! The rolls are the original's (`_SetupShipsInSystem`, `_EnterMoreShips`,
//! `_HyperShipSpawn`, `_SpawnFleet`, `_HyperSpawnFleet` and
//! `_RandomShipSpawn` in the `EV Nova` executable), each written as
//! "`below(n) < k` fires":
//!
//! - **Setup** ([`initial`]) makes `AvgShips` passes. Each first rolls
//!   for a person ([`PersonRules::roll`],
//!   see [`person`](crate::person)): a person who may appear is placed as
//!   a `düde` ship is, an empty roll brings nothing that pass, and no
//!   person goes on to a `LinkSyst` fleet (`Rand(7) == 0`, [`link_fleet`]),
//!   else one `düde` ship, placed in the system already: x and y each
//!   `Rand(1500) - 750`, heading `Rand(360)`, at its top speed along its
//!   heading ([`at_speed`]). Then each of the system's Person slots naming
//!   a person alive whose `ActiveOn` holds is rolled
//!   ([`PersonRules::listed`]), and
//!   brings its person, placed so, unless one of its name is there.
//!
//! # Setup speed and stationary warships
//!
//! - **Speed.** `_SetupShipsInSystem` (@0x42b61) gives every pass's ship
//!   (@0x432eb) and every slot's person (@0x433fe)
//!   `_AdjustedAccel(heading, max, max, &vel)` (@0x2ae4b), which adds to
//!   the velocity while it is short of the top speed: from rest, exactly
//!   the top speed along the heading. The top speed is ship-class +0x40,
//!   `shïp` `Speed` / 100 (`_LoadObjectData` @0x7a0db-0x7a0ef), here the
//!   ship's [`Handling::max_speed`](crate::handling::Handling::max_speed)
//!   with the outfits it flies with: a `düde` ship's default items, or a
//!   person's fitted ship. The original reads the bare `Speed`; each ship
//!   here still starts at exactly its own top speed. A `LinkSyst` fleet
//!   gets none: it comes by hyperspace.
//! - **Derelicts.** A person of a derelict government (`gövt` `Flags`
//!   0x0800) brought by a pass has its velocity zeroed (@0x43245-0x4325b),
//!   but the code jumps on to the pass's shared `_AdjustedAccel`
//!   (@0x432b0), so it starts at its top speed. One brought by its slot is
//!   zeroed after its `_AdjustedAccel` (@0x43418-0x43443), and starts at
//!   rest. The other reading of
//!   [`RuleKey::DerelictSetupSpeed`](crate::RuleKey::DerelictSetupSpeed)
//!   starts both at rest.
//! - **Stationary warships** ([`dude_setup_state`]). A `düde` ship flown
//!   as a warship (AI type 3) whose ship has no speed starts, when the
//!   system has a first stellar, at that stellar plus
//!   [`STATIONARY_DISTANCE`] at angle `Rand(360)`, at rest, then heading
//!   `Rand(360)` (`_RandomShipSpawn` @0x3c254-0x3c324, @0x3c410). The
//!   stellar is the first the galaxy overlay shows for the system,
//!   destroyed or not, as the original reads its first stellar slot with
//!   no other check. Here that is the first readable `spöb` of its
//!   `nav_def`s: where slot 0 is empty or unreadable, the original falls
//!   back to the box and this takes the next stellar. A person never is:
//!   `_SpawnPerson` places it through `_GenericRandomShipSpawn`'s box
//!   alone (@0x3ccf0-0x3cd1a).
//! - **Arrivals** ([`arrivals`]), every tick while the system holds fewer
//!   NPCs than `AvgShips`, draw `Rand(500)`. The count leaves out the
//!   player's escorts and fighters but takes in an NPC carrier's fighters
//!   (`_EnterMoreShips` @0x43b14-0x43b5c). On 1, when the system's
//!   `DudeTypes` name fleets, `Rand(100) + 1` at or below the sum of their
//!   `% Prob` brings one in, picked by `% Prob`; otherwise it falls
//!   through to 0's case. On 0, when the system has a `düde`, the person
//!   roll (a person who may appear, its government not derelict, jumps
//!   in; an empty roll brings nothing), else a `LinkSyst` fleet
//!   (`Rand(7) == 0`), else one `düde` ship jumps in.
//! - **A `LinkSyst` fleet** ([`link_fleet`]) draws `Rand(256)`, `flët`
//!   `128 + slot`, and comes only when that fleet's `LinkSyst` matches.
//! - **A fleet** ([`fleet`]) is its lead, with the fleet's government and
//!   its ship's `InherentAI` (or the AI type its caller gives: the
//!   escort-strength timer's fleet is led by an interceptor, see
//!   [`strength`](crate::traffic::strength)), and `Min + Rand(Max - Min + 1)` of each
//!   escort type, each at the lead's place plus `Rand(300) - 150` each way,
//!   with its heading and velocity, its own ship's `InherentAI`, escorting
//!   the lead. Fleets always arrive, at setup too: by hyperspace, or out
//!   of a gate. An announced fleet's lead carries its `flët`'s `Quote`
//!   ([`NewShip::quote`], see [`quote`](crate::traffic::quote)): every
//!   arriving fleet is announced, setup's fleets never
//!   ([`FleetCall::announced`]). A `flët` with `Flags` 0x0001
//!   ([`RANDOM_CARGO`]) gives each of its ships whose class's
//!   `InherentAI` is at most 2, the lead and each escort as it is placed,
//!   one commodity `Rand(6)` of `Rand(holds) + 1` tons in its own hold
//!   ([`NewShip::cargo`]).
//! - **A `düde` ship** is a `düde` picked by weight, then a ship by its
//!   `Probability`; its AI is the `düde`'s `AIType` when above 0, or else
//!   its ship's `InherentAI`, and its government the `düde`'s.
//! - **By hyperspace** ([`hyperspace_entry`]) a ship starts at angle
//!   `Rand(360)`, [`jump_in_distance`] from the centre, heading in at
//!   [`JUMP_IN_SPEED`]. A ship with no fuel capacity is not brought in,
//!   by hyperspace or gate; nor is a fleet whose lead has none.
//! - **By hypergate or wormhole** (see [`emerge`](crate::traffic::emerge)):
//!   every arrival, a `düde` ship, a person or a fleet's lead, the
//!   `LinkSyst` fleets at setup among them, may come out of one of the
//!   system's [`Stellars`] instead ([`emerge_stellar`]). A lone ship draws
//!   its jump-in angle first and then the pick (`_HyperShipSpawn` @0x42a26,
//!   @0x42ac0); a fleet's lead draws the pick first and its jump-in angle
//!   only when it jumps in (`_HyperSpawnFleet` @0x41f0b). Out of a gate,
//!   it starts at the gate's centre, at rest, on the gate's
//!   [`exit_heading`], waiting [`EMERGE_TICKS`]. Each escort of a lead
//!   that came out of a gate draws its offsets still (@0x4234a, @0x42391),
//!   then is placed at the gate's centre, at rest, on its lead's heading,
//!   waiting [`EMERGE_TICKS`] + [`ESCORT_EMERGE_EXTRA`] +
//!   `Rand(`[`ESCORT_EMERGE_SPREAD`]`)` (@0x42534-0x4255e). With no gate
//!   to come out of, nothing more is drawn.
//!
//! - **Aggression** ([`aggression`]): every ship spawned draws
//!   `Rand(3) ^ 2` last, after its place (@0x3c55b, @0x3cb04): 2, 3 or 0.
//!   It decides how far a warship hunts a player wanted by its
//!   government, and when it retreats (see [`ai`](crate::ai)). A person
//!   draws none: its `Aggress` gives it ([`aggression_of`]).
//!
//! A ship type with no record spawns nothing, a `Max` below `Min` gives
//! `Min`, a negative count none, and a draw is never asked over 0.
//!
//! # Mission ships
//!
//! An active mission's special ships ([`SpecialShips`](crate::mission::SpecialShips))
//! are resolved as it is accepted: their system from `ShipSyst`
//! ([`select_ship_system`](crate::mission::ships::select_ship_system)),
//! their name and subtitle, and, for a `ShipStart` above 0, their appear
//! timer. Each ship carries its mission's mark ([`MissionMark`], the
//! original's ship +0xc8d2: the slot and its `ShipBehav`). They come in
//! three ways, each through
//! `_MissionDudeSpawn` (@0x3cd3b, [`mission_ships`],
//! [`arriving_mission_ships`], [`swapped_mission_ship`]): a ship drawn from the mission's `düde`,
//! its government, `Booty`, `InfoTypes` and AI, aimed at the player for
//! `ShipBehav` 0 (`_AIMakeShipAttackPlayer` @0x3d099-0x3d0bb).
//!
//! - **At setup** (`_SetupShipsInSystem` @0x42dc4-0x43194), before the
//!   system's own passes, none of which they use up: each slot in order
//!   whose ships are in the system (or follow the player, `ShipSyst` -6),
//!   with ships left and no appear timer running, brings ships left of
//!   them. `ShipStart` 0 places them at random, as any ship at setup; -1
//!   to -16 at that nav slot's stellar, or at random when the slot is
//!   empty (@0x4300a-0x43091); 2 cloaked (`_DoShipCloak`
//!   @0x43114-0x43121, [`NewShip::cloaked`]): it starts fading in, and
//!   its think uncloaks it once it reads cloaked unless one of its ship
//!   type's AI-cloak conditions holds (see [`cloak`](crate::cloak)). An
//!   escort goal's ship (`ShipGoal` 3), after its own draws, is moved to
//!   `Rand(512) - 256` each way about the centre (@0x42ed8-0x42f33), and
//!   then to its nav slot's stellar for -1 to -16. A rescue goal's ship
//!   (`ShipGoal` 5) first draws a heading `Rand(360)`, after its own
//!   draws, and starts still and derelict: shields 0 and its armour cut
//!   just below where it is disabled (@0x42f35-0x42fdc). The other two
//!   paths give it no such placement.
//! - **On the appear timer** (`_EnterMoreShips` @0x43757-0x43ad9): every
//!   tick in flight a slot whose ships are in the system with any to come
//!   counts its timer down, and at 0 they come in by hyperspace, uncloaked,
//!   around an entry point toward the system the player last jumped from.
//!   `ShipStart` 1 and 2 start the timer at accept, so they come this way
//!   first; once it has run out, a later setup brings them in place.
//!   The enter-system hook restarts the timer for `ShipStart` 1 (see the
//!   session's `mission_ships` module and the [`story`](crate::story)
//!   module's "One arrival").
//! - **In a person's place** (`_HandlePlayerCommunication`
//!   @0x62421-0x6252f): a person of `Flags` 0x0040 whose mission is
//!   accepted on its hail, with one of its ships left, is swapped for that
//!   one ship, which takes the person's position, velocity and heading.
//!   With mission `Flags` 0x0800 it flies the person's own type when the
//!   `düde` lists it, drawing no type. No count changes: it is the slot's
//!   one ship left (see the session's `mission_ships` module).
//!
//! A `ShipBehav` 1 ship joins the player's formation as a mission escort
//! as it comes by any of the three paths (see the session's
//! `mission_escorts` module).
//!
//! A ship destroyed lowers its slot's ships left, so it does not come again. An
//! abort unmarks its ships, or takes them out while docked; a freed slot's
//! ships are unmarked on the next tick, as `_AIDispatch` does; either way
//! they fly on as ordinary traffic, a mission escort taking its ship
//! type's `InherentAI` and leaving the system.
//!
//! # Auxiliary ships
//!
//! A mission's auxiliary ships (`AuxShipCount` of its `AuxShipDude`, see
//! [`mission::aux_ships`](crate::mission::aux_ships)) are ordinary traffic: each
//! comes through `_SpecificDudeSpawn` (@0x3d0eb, [`arriving_aux_ships`]),
//! not `_MissionDudeSpawn`, with no mission mark, so it counts toward no
//! goal and gets none of a special ship's handling. It carries only its
//! slot's aux mark (ship +0x8a), which gives it back to its slot's ships
//! left to come if it is still in the system as the system's ships are
//! dropped. Once their countdown runs out, in a system their
//! `AuxShipSyst` allows, all those left to come and not yet arrived come
//! in one frame by hyperspace around one entry point at an angle drawn,
//! never toward the system the player jumped from (`_EnterMoreShips`
//! @0x434c9-0x43734). A ship type is drawn among the `düde`'s whose
//! `AppearOn` holds, with no fallback: a `düde` none of whose types may
//! appear brings none. When the session brings them in is its
//! `mission_ships` module's.
//!
//! # `AppearOn`
//!
//! The `flët` and `shïp` `AppearOn` tests are asked through the
//! [`ControlBits`](crate::ControlBits) port for the player's pilot, the
//! [`Gate`] of the draw's [`PersonWorld`], when a ship spawns, so a bit
//! set in flight counts from the next spawn. One that did not parse never
//! holds. They are checked where the executable checks them, against the
//! results `_PropagateMissionBitEffects` keeps (`shïp` @0x997e0, `flët`
//! @0x9985a):
//!
//! - **A `düde`'s ship types** whose `AppearOn` does not hold are left out
//!   of its `Probability` before the draw, as `_SelectShipFieldFromDude`
//!   does (@0x662b, @0x6662, @0x66c7): the draw is over the rest's
//!   weights, and a `düde` none of whose ship types may appear brings
//!   nothing, with no ship drawn. This is the Bible's "will not show up in
//!   dude resources". A ship type with no record stays in the weights.
//! - **A fleet's escorts** of a ship type whose `AppearOn` does not hold
//!   do not come; their count is drawn first (`_HyperSpawnFleet`
//!   @0x42143, @0x4217f). Its lead comes whatever its ship type's
//!   `AppearOn`, and a person's ship never reads it.
//! - **A fleet** whose own `AppearOn` does not hold never comes, and draws
//!   nothing ([`fleet`]). The `DudeTypes` fleets are picked among those
//!   with a record whose `AppearOn` holds, against the odds of them all, as
//!   `_SelectFleetFromSystem` does (@0x6884): when none may appear, the
//!   arrival falls through to 0's case, as on a roll above the odds. A
//!   `LinkSyst` slot whose fleet's `AppearOn` does not hold brings nothing
//!   (`_SpawnFleet` @0x4275b, @0x428a5). The executable leaves such a
//!   fleet out of the slots, and draws no slot when none is left; here the
//!   slot is always drawn, whether or not any fleet links to the system.

use std::collections::BTreeSet;

use crate::catalog::{DudeId, FleetId, GovtId, LandingSite, PersonId, ShipId, StellarId};
use crate::chance::Chance;
use crate::control::Gate;
use crate::flight::{ShipState, facing, heading_of};
use crate::gate::exit_heading;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::market::Good;
use crate::mission::ShipGoal;
use crate::mission::ships::{ShipBehav, ShipStart};
use crate::person::{PersonRoll, PersonRules, PersonWorld, aggression_of};
use crate::rulebook::RuleSource;
use crate::traffic::emerge::{
    EMERGE_TICKS, ESCORT_EMERGE_EXTRA, ESCORT_EMERGE_SPREAD, emerge_stellar,
};
use crate::traffic::npc::{AiType, MissionMark};
use crate::traffic::table::{ShipKind, SpawnDude, SpawnPerson, SpawnTable, pick};

/// `Rand(7) == 0`: a person, rather than anything else, by the engine
/// (see [`person`](crate::person)).
pub const PERSON_ODDS: u32 = 7;
/// `Rand(7) == 0`: a `LinkSyst` fleet, rather than a `düde` ship.
pub const FLEET_ODDS: u32 = 7;
/// `Rand(500)`: 0 or 1 brings an arrival, about one every 8 s.
pub const ARRIVAL_ODDS: u32 = 500;
/// `Rand(100) + 1`: a percentage roll.
pub const PERCENT: u32 = 100;
/// `Rand(256)`: the fleet slot a `LinkSyst` roll lands on.
pub const FLEET_SLOTS: u32 = 256;
/// The `flët` ID of slot 0.
pub const FIRST_FLEET_ID: i16 = 128;
/// `Rand(1500) - 750`: where in the system a ship is placed at setup.
pub const SPAWN_SPREAD: u32 = 1500;
/// How far from the system's first stellar a stationary warship starts
/// (100.0 @0x3c305).
pub const STATIONARY_DISTANCE: f32 = 100.0;
/// `Rand(300) - 150`: how far from its lead an escort starts.
pub const ESCORT_SPREAD: u32 = 300;
/// `Rand(360)`: a heading, or an angle round the centre.
pub const DEGREES: u32 = 360;
/// How many ticks of the glide a ship arriving by hyperspace is placed
/// back from [`JUMP_IN_END`] (`0x2b`, `_HyperShipSpawn`): where it starts,
/// not how long it glides, which its cap decides
/// ([`TopSpeeds::burst_cap`](crate::traffic::autopilot::TopSpeeds::burst_cap)).
pub const JUMP_IN_TICKS: u32 = 43;
/// Its speed on the glide's first tick, in pixels a tick.
pub const JUMP_IN_SPEED: f32 = 50.0;
/// How much slower each tick of the glide is than the last.
pub const JUMP_IN_SLOWING: f32 = 1.165;
/// How far from the centre a glide of [`JUMP_IN_TICKS`] would end, the
/// distance a ship's start is reckoned from.
pub const JUMP_IN_END: f32 = 1000.0;
/// The `flët` `Flags` bit that gives its trader ships random cargo
/// (@0x41e56-0x41e5e).
pub const RANDOM_CARGO: u16 = 0x0001;
/// The most `InherentAI` a ship class may have, as a signed comparison,
/// for a [`RANDOM_CARGO`] fleet to give it cargo: the traders'
/// (@0x41e7b-0x41e81).
const CARGO_AI_MAX: i16 = 2;
/// `Rand(6)`: which of the six standard commodities its cargo is.
const CARGO_COMMODITIES: u32 = 6;
/// `Rand(3)`: the draw a ship's aggression is made from.
pub const AGGRESSION_DRAW: u32 = 3;
/// What the aggression draw is combined with, bit by bit, by exclusive
/// or.
const AGGRESSION_XOR: u32 = 2;

/// How a ship added to the system comes into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    /// Placed in the system already, flying.
    Placed,
    /// Gliding in from hyperspace.
    JumpingIn,
    /// Coming out of a hypergate or wormhole, after a wait of this many
    /// ticks (see [`emerge`](crate::traffic::emerge)).
    Emerging {
        /// The ticks of the wait.
        ticks_left: u32,
        /// The gate it comes out of.
        gate: StellarId,
    },
}

/// The system's stellars, which an arrival may come out of (see
/// [`emerge`](crate::traffic::emerge)), and the governments that judge
/// whether a ship may.
#[derive(Clone, Copy, Debug)]
pub struct Stellars<'a> {
    /// The system's stellars, as the galaxy overlay shows them.
    pub sites: &'a [LandingSite],
    /// Every government and their relations.
    pub govts: &'a Governments,
}

/// No governments at all.
static NO_GOVERNMENTS: Governments = Governments::empty();

impl Stellars<'static> {
    /// No stellars: every arrival comes by hyperspace.
    pub const NONE: Self = Self {
        sites: &[],
        govts: &NO_GOVERNMENTS,
    };
}

/// A ship to add to the system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NewShip {
    /// Its ship type.
    pub ship: ShipId,
    /// Its government.
    pub govt: Option<GovtId>,
    /// How it behaves.
    pub ai_type: AiType,
    /// The index, among the ships added with it, of the lead it escorts.
    pub lead: Option<usize>,
    /// Where it starts.
    pub state: ShipState,
    /// How it comes into the system.
    pub entry: Entry,
    /// How aggressive it is: 0, 2 or 3.
    pub aggression: u8,
    /// Its `düde`'s `Booty` flags; none for a fleet's ship, which has no
    /// `düde`.
    pub booty: u16,
    /// Its `düde`'s `InfoTypes` flags, what it says when hailed; none for
    /// a fleet's ship.
    pub info_types: u16,
    /// The person flying it, if any.
    pub person: Option<PersonId>,
    /// The mission whose special ship it is, if any.
    pub mission: Option<MissionMark>,
    /// The slot of the mission whose auxiliary ship it is, if any.
    pub aux: Option<u8>,
    /// Whether it starts as a derelict does: its shield gone, its armour
    /// cut just below where it is disabled, and disabled. A rescue
    /// mission's ship set up in the system starts so ([`mission_ships`]).
    pub derelict: bool,
    /// Whether it starts cloaking (`_DoShipCloak`): a `ShipStart` 2
    /// mission's ship set up in the system does ([`mission_ships`]).
    pub cloaked: bool,
    /// The `STR#` its arrival shows a line of: only an announced fleet's
    /// lead has one, and only when its `flët` has a `Quote`
    /// ([`FleetCall::announced`], see [`quote`](crate::traffic::quote)).
    pub quote: Option<i16>,
    /// The commodity in its own hold and its tons, if any: the original's
    /// six commodity words at ship +0x7a, all cleared as it spawns
    /// (`_GenericRandomShipSpawn` @0x3ccce-0x3cce5), of which a `flët`
    /// with `Flags` 0x0001 fills one for its trader ships
    /// ([`fleet`]). The plunder never
    /// offers it: boarding reads the `düde`'s `Booty` alone
    /// (`_SetPlunderValues` @0x92222-0x92256).
    pub cargo: Option<(Good, u32)>,
}

/// What a caller of [`fleet`] asks of the fleet: `_HyperSpawnFleet`'s
/// third and fourth arguments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FleetCall {
    /// The AI type its lead flies as, in place of its ship's `InherentAI`
    /// (@0x41d5d-0x41d69); its escorts always fly as their own.
    pub lead_ai: Option<AiType>,
    /// Whether its arrival shows its `flët`'s `Quote` (`0x14(%ebp)`,
    /// @0x42607-0x42629). Setup's fleets come silently
    /// (`_SetupShipsInSystem` @0x4327c-0x4328d); every arriving one is
    /// announced: a `LinkSyst` fleet (`_HyperShipSpawn` @0x42973), a
    /// `DudeTypes` fleet (`_EnterMoreShips` @0x43bbc-0x43bd3) and the
    /// escort-strength timer's (@0x43d90-0x43dcf).
    pub announced: bool,
}

/// The persons' side of a system's spawning: their world, and the names
/// of the persons in the system, which grow as persons spawn.
#[derive(Clone, Debug)]
pub struct PersonDraw<'a> {
    /// The rules, the persons gone and the control bits.
    pub world: PersonWorld<'a>,
    /// The names of the persons in the system.
    pub here: BTreeSet<String>,
}

impl<'a> PersonDraw<'a> {
    /// The draw in `world`, with no person in the system.
    #[must_use]
    pub fn new(world: PersonWorld<'a>) -> Self {
        Self {
            world,
            here: BTreeSet::new(),
        }
    }

    /// The persons of `table` who may appear, `arriving` by hyperspace or
    /// not, in ascending ID (see [`person`](crate::person)).
    #[must_use]
    pub fn eligible(&self, table: &SpawnTable, arriving: bool) -> Vec<PersonId> {
        table
            .persons
            .iter()
            .filter(|(id, person)| {
                let record = &person.record;
                person.linked
                    && record.ai_type > 0
                    && !self.world.gone.contains(id)
                    && self.world.allows(&record.active_on)
                    && !(arriving && person.derelict)
                    && !self.world.in_fleet(**id)
                    && !self.named_here(table, &record.name)
            })
            .map(|(&id, _)| id)
            .collect()
    }

    /// Whether a person named `name` is in the system already, the
    /// persons of the player's fleet, known to `table`, among them.
    fn named_here(&self, table: &SpawnTable, name: &str) -> bool {
        self.here.contains(name)
            || self
                .world
                .fleet
                .iter()
                .filter_map(|escort| table.persons.get(&escort.person?))
                .any(|person| person.record.name == name)
    }

    /// The person roll among those of `table` who may appear.
    fn roll(
        &self,
        table: &SpawnTable,
        arriving: bool,
        chance: &mut (impl Chance + ?Sized),
    ) -> PersonRoll {
        let eligible = self.eligible(table, arriving);
        self.world.rules.roll(&eligible, &mut &mut *chance)
    }

    /// Person `id` of `table`, now in the system, starting at `state`.
    fn spawn(&mut self, table: &SpawnTable, id: PersonId, state: ShipState) -> Option<NewShip> {
        let person = table.persons.get(&id)?;
        let record = &person.record;
        self.here.insert(record.name.clone());
        Some(NewShip {
            ship: record.ship?,
            govt: record.govt,
            ai_type: AiType::from_raw(record.ai_type),
            lead: None,
            state,
            entry: Entry::Placed,
            aggression: aggression_of(record.aggress),
            booty: 0,
            info_types: 0,
            person: Some(id),
            mission: None,
            derelict: false,
            cloaked: false,
            aux: None,
            quote: None,
            cargo: None,
        })
    }
}

/// How fast a ship glides on tick `k` (from 0) of its jump in.
#[must_use]
pub fn glide_speed(k: u32) -> f32 {
    JUMP_IN_SLOWING.mul_add(-(k as f32), JUMP_IN_SPEED)
}

/// How far from the centre a ship starts its jump in: the glide's end
/// plus the whole glide, about 2098 pixels.
#[must_use]
pub fn jump_in_distance() -> f32 {
    JUMP_IN_END + (0..JUMP_IN_TICKS).map(glide_speed).sum::<f32>()
}

/// A system's initial population, its persons drawn as `draw` says, its
/// stationary warships placed by its `first_stellar`, when it has one,
/// and its `LinkSyst` fleets coming out of its `stellars` or by
/// hyperspace (see the module docs).
pub fn initial(
    table: &SpawnTable,
    draw: &mut PersonDraw,
    first_stellar: Option<Vec2>,
    stellars: Stellars,
    chance: &mut (impl Chance + ?Sized),
) -> Vec<NewShip> {
    let gate = draw.world.gate();
    let mut out = Vec::new();
    for _ in 0..table.avg_ships {
        match draw.roll(table, false, chance) {
            PersonRoll::Person(id) => {
                let state = in_system(chance);
                if let Some(person) = table.persons.get(&id) {
                    let speed = person_setup_speed(person, true, draw.world.rules);
                    out.extend(draw.spawn(table, id, at_speed(state, speed)));
                }
                continue;
            }
            PersonRoll::Empty => continue,
            PersonRoll::None => {}
        }
        if chance.below(FLEET_ODDS) < 1 {
            link_fleet(table, gate, stellars, false, chance, &mut out);
            continue;
        }
        if let Some((ship, dude, ai_type, kind)) = dude_ship(table, gate, chance) {
            let state = dude_setup_state(ai_type, kind, first_stellar, chance);
            out.push(NewShip {
                ship,
                govt: dude.govt,
                ai_type,
                lead: None,
                state,
                entry: Entry::Placed,
                aggression: aggression(chance),
                booty: dude.booty,
                info_types: dude.info_types,
                person: None,
                mission: None,
                derelict: false,
                cloaked: false,
                aux: None,
                quote: None,
                cargo: None,
            });
        }
    }
    for &(id, prob) in &table.person_slots {
        let Some(person) = table.persons.get(&id) else {
            continue;
        };
        let world = draw.world;
        if world.gone.contains(&id) || world.in_fleet(id) || !world.allows(&person.record.active_on)
        {
            continue;
        }
        if world.rules.listed(prob, &mut &mut *chance)
            && !draw.named_here(table, &person.record.name)
        {
            let speed = person_setup_speed(person, false, world.rules);
            out.extend(draw.spawn(table, id, at_speed(in_system(chance), speed)));
        }
    }
    out
}

/// The ships that arrive this tick in a system holding `present` ships
/// that count toward `AvgShips` (not the player's), its persons drawn as
/// `draw` says, each coming out of one of its `stellars` or by
/// hyperspace (see the module docs).
pub fn arrivals(
    table: &SpawnTable,
    present: usize,
    draw: &mut PersonDraw,
    stellars: Stellars,
    chance: &mut (impl Chance + ?Sized),
) -> Vec<NewShip> {
    let mut out = Vec::new();
    if present >= table.avg_ships as usize {
        return out;
    }
    match chance.below(ARRIVAL_ODDS) {
        1 if named_fleet(table, draw.world.gate(), stellars, chance, &mut out) => {}
        0 | 1 => hyper_ship(table, draw, stellars, chance, &mut out),
        _ => {}
    }
    out
}

/// Rolls for one of the fleets the system's `DudeTypes` name, picked
/// among those with a record whose `AppearOn` holds by `gate`, and brings
/// it in among `stellars`, added to `out`; whether a fleet was picked.
fn named_fleet(
    table: &SpawnTable,
    gate: Gate,
    stellars: Stellars,
    chance: &mut (impl Chance + ?Sized),
    out: &mut Vec<NewShip>,
) -> bool {
    if table.dude_fleets.is_empty() {
        return false;
    }
    let odds: u32 = table.dude_fleets.iter().map(|&(_, prob)| prob).sum();
    if chance.below(PERCENT) + 1 > odds {
        return false;
    }
    let appearing: Vec<(FleetId, u32)> = table
        .dude_fleets
        .iter()
        .copied()
        .filter(|(id, _)| {
            table
                .fleets
                .get(id)
                .is_some_and(|record| gate.allows(&record.appear_on))
        })
        .collect();
    let Some(id) = pick(&appearing, chance) else {
        return false;
    };
    let call = FleetCall {
        lead_ai: None,
        announced: true,
    };
    fleet(table, id, gate, stellars, call, chance, out);
    true
}

/// `_HyperShipSpawn`: when the system has a `düde`, a person (drawn as
/// `draw` says), a `LinkSyst` fleet or a `düde` ship comes in, out of one
/// of `stellars` or by hyperspace, added to `out`.
fn hyper_ship(
    table: &SpawnTable,
    draw: &mut PersonDraw,
    stellars: Stellars,
    chance: &mut (impl Chance + ?Sized),
    out: &mut Vec<NewShip>,
) {
    if table.dudes.is_empty() {
        return;
    }
    match draw.roll(table, true, chance) {
        PersonRoll::Person(id) => {
            if let Some(person) = table
                .persons
                .get(&id)
                .filter(|person| can_jump(&person.kind))
            {
                let jump_in = hyperspace_entry(chance);
                let (state, entry) = arrival(person.record.govt, jump_in, stellars, chance);
                out.extend(
                    draw.spawn(table, id, state)
                        .map(|ship| NewShip { entry, ..ship }),
                );
            }
            return;
        }
        PersonRoll::Empty => return,
        PersonRoll::None => {}
    }
    let gate = draw.world.gate();
    if chance.below(FLEET_ODDS) < 1 {
        link_fleet(table, gate, stellars, true, chance, out);
        return;
    }
    if let Some((ship, dude, ai_type, kind)) = dude_ship(table, gate, chance)
        && can_jump(kind)
    {
        let jump_in = hyperspace_entry(chance);
        let (state, entry) = arrival(dude.govt, jump_in, stellars, chance);
        out.push(NewShip {
            ship,
            govt: dude.govt,
            ai_type,
            lead: None,
            state,
            entry,
            aggression: aggression(chance),
            booty: dude.booty,
            info_types: dude.info_types,
            person: None,
            mission: None,
            derelict: false,
            cloaked: false,
            aux: None,
            quote: None,
            cargo: None,
        });
    }
}

/// `Rand(512) - 256`: how far from their entry point each mission ship
/// coming in later is placed, each way (@0x43992-0x43a10).
pub const MISSION_ARRIVAL_SPREAD: u32 = 512;

/// `Rand(512) - 256`: where about the system's centre each ship of a
/// `ShipGoal` 3 mission set up in the system is placed, each way
/// (`_SetupShipsInSystem` @0x42ed8-0x42f33).
pub const ESCORTED_SETUP_SPREAD: u32 = 512;

/// A mission's special ships to bring in, as its slot holds them (see
/// the module docs' "Mission ships").
#[derive(Clone, Copy, Debug)]
pub struct MissionSpawn<'a> {
    /// The mission's slot.
    pub slot: u8,
    /// Its `ShipDude`.
    pub dude: DudeId,
    /// How many to bring in.
    pub count: i16,
    /// Its `ShipGoal`: only the setup reads it ([`mission_ships`]).
    pub goal: ShipGoal,
    /// Its `ShipStart`.
    pub start: ShipStart,
    /// Its `ShipBehav`.
    pub behav: ShipBehav,
    /// The system's `NavDefs` slots, in order: each its stellar's
    /// position, none for an empty slot.
    pub nav: &'a [Option<Vec2>],
}

/// One of `spawn`'s ships as `_MissionDudeSpawn` (@0x3cd3b) brings it in:
/// placed as any ship at setup, its aggression drawn first, then x and y
/// (`_GenericRandomShipSpawn` @0x3c89f), with no heading drawn; then its
/// ship type: `preset` when given, drawing nothing (the slot's +0x56,
/// @0x3cdea-0x3cdf3); else drawn from the `düde` by `Probability` among
/// the ship types whose `AppearOn` holds by `gate`, or, with none, among
/// them all (`_SelectShipFieldFromDude` with 0, then 1, @0x3cdf5-0x3ce36). It
/// carries the `düde`'s government, `Booty` and `InfoTypes`, the `düde`'s
/// `AIType` above 0 or else its ship's `InherentAI`, and the mission's
/// mark. `None`, after the place's draws, when the `düde` has no record
/// or no ship.
fn mission_dude_ship(
    table: &SpawnTable,
    spawn: &MissionSpawn,
    preset: Option<ShipId>,
    gate: Gate,
    chance: &mut (impl Chance + ?Sized),
) -> Option<NewShip> {
    let ship = specific_dude_ship(table, spawn.dude, preset, Fallback::AllShips, gate, chance)?;
    Some(NewShip {
        mission: Some(MissionMark {
            order: None,
            slot: spawn.slot,
            behav: spawn.behav,
            formation: None,
        }),
        ..ship
    })
}

/// Where a ship type is drawn from when none of a `düde`'s may appear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fallback {
    /// From all of them (`_MissionDudeSpawn`'s second draw, @0x3ce1b).
    AllShips,
    /// None: the spawn fails (`_SpecificDudeSpawn` @0x3d146-0x3d14c).
    Fails,
}

/// A ship of `düde` `dude`, as `_GenericRandomShipSpawn` and
/// `_SelectShipFieldFromDude` bring it in: its aggression drawn, then x
/// and y (@0x3c89f), with no heading drawn; then its ship type: `preset`
/// when given, drawing nothing; else drawn by `Probability` among the
/// ship types whose `AppearOn` holds by `gate`, or with none as
/// `fallback` says. It carries the `düde`'s government, `Booty` and
/// `InfoTypes`, and the `düde`'s `AIType` above 0 or else its ship's
/// `InherentAI`, and no mark. `None`, after the place's draws, when the
/// `düde` has no record or no ship comes of it.
fn specific_dude_ship(
    table: &SpawnTable,
    dude: DudeId,
    preset: Option<ShipId>,
    fallback: Fallback,
    gate: Gate,
    chance: &mut (impl Chance + ?Sized),
) -> Option<NewShip> {
    let aggression = aggression(chance);
    let x = spread(SPAWN_SPREAD, chance);
    let y = spread(SPAWN_SPREAD, chance);
    let dude = table.dude_records.get(&dude)?;
    let ship = if let Some(ship) = preset {
        ship
    } else {
        let appearing: Vec<(ShipId, u32)> = dude
            .ships
            .iter()
            .copied()
            .filter(|(ship, _)| appears(table, *ship, gate))
            .collect();
        match fallback {
            Fallback::AllShips => pick(&appearing, chance).or_else(|| pick(&dude.ships, chance))?,
            Fallback::Fails => pick(&appearing, chance)?,
        }
    };
    let inherent_ai = table.ships.get(&ship).map_or(0, |kind| kind.inherent_ai);
    let ai_type = if dude.ai_type > 0 {
        dude.ai_type
    } else {
        inherent_ai
    };
    Some(NewShip {
        ship,
        govt: dude.govt,
        ai_type: AiType::from_raw(ai_type),
        lead: None,
        state: ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            heading: 0.0,
        },
        entry: Entry::Placed,
        aggression,
        booty: dude.booty,
        info_types: dude.info_types,
        person: None,
        mission: None,
        derelict: false,
        cloaked: false,
        aux: None,
        quote: None,
        cargo: None,
    })
}

/// `spawn`'s ships as the system is set up (`_SetupShipsInSystem`
/// @0x42e89-0x43121): `count` of them, each placed as
/// `mission_dude_ship` places it; for an escort goal (`ShipGoal` 3) then
/// moved to `Rand(512) - 256` each way about the centre, x drawn first
/// ([`ESCORTED_SETUP_SPREAD`], @0x42ed8-0x42f33); for a rescue goal
/// (`ShipGoal` 5) then
/// facing a heading `Rand(360)`, still, and set up derelict
/// ([`NewShip::derelict`]: shields 0, armour cut, @0x42f35-0x42fdc);
/// with a `ShipStart` of -1 to -16 then moved to that nav slot's stellar,
/// unless the slot is empty (@0x4300a-0x43091); with `ShipStart` 2,
/// cloaking ([`NewShip::cloaked`], `_DoShipCloak` @0x43114-0x43121). The
/// original also zeroes a
/// rescue ship's +0x48, an inertialess ship's speed along its heading
/// (`_AdjustInertialessShipVelocity` @0x334c9-0x334d9), which the engine
/// does not keep: the zero velocity covers it.
pub fn mission_ships(
    table: &SpawnTable,
    spawn: &MissionSpawn,
    gate: Gate,
    chance: &mut (impl Chance + ?Sized),
) -> Vec<NewShip> {
    let nav = match spawn.start {
        ShipStart::Nav(slot) => spawn.nav.get(usize::from(slot)).copied().flatten(),
        _ => None,
    };
    let rescue = spawn.goal == ShipGoal::Rescue;
    let escorted = spawn.goal == ShipGoal::Escort;
    (0..spawn.count.max(0))
        .filter_map(|_| {
            let mut ship = mission_dude_ship(table, spawn, None, gate, chance)?;
            if escorted {
                let x = spread(ESCORTED_SETUP_SPREAD, chance);
                let y = spread(ESCORTED_SETUP_SPREAD, chance);
                ship.state.position = Vec2::new(x, y);
            }
            if rescue {
                ship.state.heading = chance.below(DEGREES) as f32;
                ship.state.velocity = Vec2::ZERO;
                ship.derelict = true;
            }
            Some(ship)
        })
        .map(|mut ship| {
            if let Some(at) = nav {
                ship.state.position = at;
            }
            ship.cloaked = spawn.start == ShipStart::Cloaked;
            ship
        })
        .collect()
}

/// `spawn`'s ships coming in later, as `_EnterMoreShips` brings them when
/// their appear timer runs out (@0x43821-0x43aa5): an entry point
/// [`jump_in_distance`] out, toward `from` (the way to the system the
/// player last jumped from) or else at an angle `Rand(360)`, drawn first;
/// then `count` ships, each placed as `mission_dude_ship` places it and
/// moved to the entry point plus `Rand(512) - 256` each way, facing the
/// centre and gliding in from hyperspace. None is cloaked, and a ship
/// with no fuel comes all the same.
pub fn arriving_mission_ships(
    table: &SpawnTable,
    spawn: &MissionSpawn,
    from: Option<Vec2>,
    gate: Gate,
    chance: &mut (impl Chance + ?Sized),
) -> Vec<NewShip> {
    let toward = from.filter(|way| way.length() > 0.0).map_or_else(
        || facing(chance.below(DEGREES) as f32),
        |way| way * (1.0 / way.length()),
    );
    let entry = toward * jump_in_distance();
    (0..spawn.count.max(0))
        .filter_map(|_| {
            let ship = mission_dude_ship(table, spawn, None, gate, chance)?;
            Some(glide_in(ship, entry, chance))
        })
        .collect()
}

/// `ship` moved to `entry` plus `Rand(512) - 256` each way, x drawn
/// first, facing the centre and gliding in from hyperspace at
/// [`JUMP_IN_SPEED`] (@0x43992-0x43a10, @0x43617-0x4372a).
fn glide_in(ship: NewShip, entry: Vec2, chance: &mut (impl Chance + ?Sized)) -> NewShip {
    let x = spread(MISSION_ARRIVAL_SPREAD, chance);
    let y = spread(MISSION_ARRIVAL_SPREAD, chance);
    let position = entry + Vec2::new(x, y);
    let heading = heading_of(Vec2::ZERO - position);
    NewShip {
        state: ShipState {
            position,
            velocity: facing(heading) * JUMP_IN_SPEED,
            heading,
        },
        entry: Entry::JumpingIn,
        ..ship
    }
}

/// A mission's auxiliary ships to bring in (see the module docs'
/// "Auxiliary ships").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuxSpawn {
    /// The mission's slot.
    pub slot: u8,
    /// Its `AuxShipDude`.
    pub dude: DudeId,
    /// How many to bring in.
    pub count: i16,
}

/// `spawn`'s auxiliary ships, as `_EnterMoreShips` brings them in once
/// their countdown runs out (@0x43525-0x43734): an entry point
/// [`jump_in_distance`] out at an angle `Rand(360)`, drawn first, never
/// toward the system the player jumped from; then `count` ships, each
/// through `_SpecificDudeSpawn` (@0x3d0eb): placed as any ship at setup,
/// its aggression, x and y drawn, then its ship type among those of the
/// `düde` whose `AppearOn` holds, with no fallback, so a `düde` none of
/// whose types may appear brings that ship nowhere, drawing nothing more
/// for it; then moved to the entry point plus `Rand(512) - 256` each way,
/// facing the centre and gliding in. Each is ordinary traffic, of no
/// mission, with its slot's aux mark (ship +0x8a, @0x435e3). A ship with
/// no fuel comes all the same.
pub fn arriving_aux_ships(
    table: &SpawnTable,
    spawn: &AuxSpawn,
    gate: Gate,
    chance: &mut (impl Chance + ?Sized),
) -> Vec<NewShip> {
    let entry = facing(chance.below(DEGREES) as f32) * jump_in_distance();
    (0..spawn.count.max(0))
        .filter_map(|_| {
            let ship = specific_dude_ship(table, spawn.dude, None, Fallback::Fails, gate, chance)?;
            Some(glide_in(
                NewShip {
                    aux: Some(spawn.slot),
                    ..ship
                },
                entry,
                chance,
            ))
        })
        .collect()
}

/// `spawn`'s one ship swapped in for a person whose mission was just
/// accepted on its hail (`_HandlePlayerCommunication` @0x6244b-0x624e5):
/// brought in as `mission_dude_ship` brings it, its type `preset` when
/// given ([`swapped_ship_type`](crate::mission::ships::swapped_ship_type)),
/// then given the person's `state` (position +0x18, velocity +0x20,
/// heading +0x44), not gliding in and never cloaked. `spawn`'s count and
/// start are not read. `None`, after the place's draws, when the `düde`
/// has no record or no ship.
pub fn swapped_mission_ship(
    table: &SpawnTable,
    spawn: &MissionSpawn,
    state: ShipState,
    preset: Option<ShipId>,
    gate: Gate,
    chance: &mut (impl Chance + ?Sized),
) -> Option<NewShip> {
    let ship = mission_dude_ship(table, spawn, preset, gate, chance)?;
    Some(NewShip { state, ..ship })
}

/// A `düde` ship: a `düde` by weight, a ship of it by `Probability`
/// among those whose `AppearOn` holds by `gate`, and the `düde`, its AI
/// and the ship's kind; `None` when either has no record, or no ship of
/// the `düde` may appear.
fn dude_ship<'a>(
    table: &'a SpawnTable,
    gate: Gate,
    chance: &mut (impl Chance + ?Sized),
) -> Option<(ShipId, &'a SpawnDude, AiType, &'a ShipKind)> {
    let dude = table.dude_records.get(&pick(&table.dudes, chance)?)?;
    let appearing: Vec<(ShipId, u32)> = dude
        .ships
        .iter()
        .copied()
        .filter(|(ship, _)| appears(table, *ship, gate))
        .collect();
    let ship = pick(&appearing, chance)?;
    let kind = table.ships.get(&ship)?;
    let ai_type = if dude.ai_type > 0 {
        dude.ai_type
    } else {
        kind.inherent_ai
    };
    Some((ship, dude, AiType::from_raw(ai_type), kind))
}

/// The `LinkSyst` fleet roll: one of 256 slots, and the fleet there, if
/// its `LinkSyst` matches and its `AppearOn` holds by `gate`, added to
/// `out`, its arrival `announced` or not ([`FleetCall::announced`]).
pub fn link_fleet(
    table: &SpawnTable,
    gate: Gate,
    stellars: Stellars,
    announced: bool,
    chance: &mut (impl Chance + ?Sized),
    out: &mut Vec<NewShip>,
) {
    let slot = chance.below(FLEET_SLOTS) as i16;
    let id = FleetId(FIRST_FLEET_ID + slot);
    if table.link_fleets.contains(&id) {
        let call = FleetCall {
            lead_ai: None,
            announced,
        };
        fleet(table, id, gate, stellars, call, chance, out);
    }
}

/// Fleet `id` coming in, added to `out`: its lead, out of one of
/// `stellars` or by hyperspace, then its escorts whose ship type's
/// `AppearOn` holds by `gate`, each near the lead by hyperspace or at the
/// lead's gate; nothing, and no draw, when the fleet's own `AppearOn`
/// does not. The lead flies as the `call`'s AI type when it gives one,
/// and carries the fleet's `Quote` when the `call` announces it (see
/// [`FleetCall`]).
///
/// The whole fleet is rolled, even the ships that will find no ship-table
/// slot: the cap (`_GenericRandomShipSpawn` giving -1 for the lead
/// @0x41d02 or an escort @0x421b9) is applied as the traffic places them
/// ([`Traffic`](crate::Traffic)'s `add`), in the same call and before any
/// frame, so the draws it makes for them change nothing the player sees.
pub fn fleet(
    table: &SpawnTable,
    id: FleetId,
    gate: Gate,
    stellars: Stellars,
    call: FleetCall,
    chance: &mut (impl Chance + ?Sized),
    out: &mut Vec<NewShip>,
) {
    let Some(record) = table
        .fleets
        .get(&id)
        .filter(|record| gate.allows(&record.appear_on))
    else {
        return;
    };
    let Some((lead_ship, kind)) = record
        .lead
        .and_then(|ship| Some((ship, table.ships.get(&ship)?)))
    else {
        return;
    };
    if !can_jump(kind) {
        return;
    }
    let lead = out.len();
    let (state, entry) = match emerge_stellar(record.govt, stellars.sites, stellars.govts, chance) {
        Some(site) => emerging(site, chance),
        None => (hyperspace_entry(chance), Entry::JumpingIn),
    };
    out.push(NewShip {
        ship: lead_ship,
        govt: record.govt,
        ai_type: call
            .lead_ai
            .unwrap_or_else(|| AiType::from_raw(kind.inherent_ai)),
        lead: None,
        state,
        entry,
        aggression: aggression(chance),
        booty: 0,
        info_types: 0,
        person: None,
        mission: None,
        derelict: false,
        cloaked: false,
        aux: None,
        quote: record.quote.filter(|_| call.announced),
        cargo: random_cargo(record.flags, kind, chance),
    });
    for escort in &record.escorts {
        let count = escort_count(escort.min, escort.max, chance);
        let Some(kind) = table
            .ships
            .get(&escort.ship)
            .filter(|kind| can_jump(kind) && gate.allows(&kind.appear_on))
        else {
            continue;
        };
        for _ in 0..count {
            let placed = escort_offset(&state, chance);
            let (placed, escort_entry) = match entry {
                Entry::Emerging { gate, .. } => (
                    state,
                    Entry::Emerging {
                        ticks_left: EMERGE_TICKS
                            + ESCORT_EMERGE_EXTRA
                            + chance.below(ESCORT_EMERGE_SPREAD),
                        gate,
                    },
                ),
                Entry::Placed | Entry::JumpingIn => (placed, entry),
            };
            out.push(NewShip {
                ship: escort.ship,
                govt: record.govt,
                ai_type: AiType::from_raw(kind.inherent_ai),
                lead: Some(lead),
                state: placed,
                entry: escort_entry,
                aggression: aggression(chance),
                booty: 0,
                info_types: 0,
                person: None,
                mission: None,
                derelict: false,
                cloaked: false,
                aux: None,
                quote: None,
                cargo: random_cargo(record.flags, kind, chance),
            });
        }
    }
}

/// The random cargo a ship of `kind` in a fleet of `flags` starts with
/// (`_HyperSpawnFleet` @0x41e49-0x41ec1 for the lead, @0x42430-0x424a5
/// for each escort): with [`RANDOM_CARGO`], when its class's own
/// `InherentAI` (not the AI type it flies as) is at most 2, a trader's,
/// one commodity `Rand(6)` of `Rand(holds) + 1` tons; otherwise none, and
/// no draw. A class of no holds gets none: the original would draw
/// `Rand(0)`, which reseeds its generator and returns the new seed's low
/// 16 bits (@0xa4c82-0xa4ca2), an arbitrary tonnage. Data that broken
/// is not worth copying, so no rule chooses it.
fn random_cargo(
    flags: u16,
    kind: &ShipKind,
    chance: &mut (impl Chance + ?Sized),
) -> Option<(Good, u32)> {
    if flags & RANDOM_CARGO == 0 || kind.inherent_ai > CARGO_AI_MAX || kind.holds == 0 {
        return None;
    }
    let commodity = u8::try_from(chance.below(CARGO_COMMODITIES)).unwrap_or(0);
    let tons = chance.below(u32::from(kind.holds)) + 1;
    Some((Good::Commodity(commodity), tons))
}

/// How many of an escort type with `min` and `max` come:
/// `Min + Rand(Max - Min + 1)`, `min` when `max` is not above it, and
/// none below none.
pub fn escort_count(min: i16, max: i16, chance: &mut (impl Chance + ?Sized)) -> u32 {
    let min = u32::try_from(min).unwrap_or(0);
    let max = u32::try_from(max).unwrap_or(0);
    if max <= min {
        return min;
    }
    min + chance.below(max - min + 1)
}

/// Where in the system a ship placed at setup starts: somewhere within
/// 750 pixels of the centre each way, facing anywhere, still at rest
/// until [`at_speed`] sets it moving.
pub fn in_system(chance: &mut (impl Chance + ?Sized)) -> ShipState {
    let x = spread(SPAWN_SPREAD, chance);
    let y = spread(SPAWN_SPREAD, chance);
    ShipState {
        position: Vec2::new(x, y),
        velocity: Vec2::ZERO,
        heading: chance.below(DEGREES) as f32,
    }
}

/// A ship placed at setup, `state`, moving at `max_speed` along its
/// heading, as `_AdjustedAccel(heading, max, max, &vel)` sets it from rest
/// (@0x2ae4b): at rest when `max_speed` is 0.
#[must_use]
pub fn at_speed(state: ShipState, max_speed: f32) -> ShipState {
    ShipState {
        velocity: facing(state.heading) * max_speed,
        ..state
    }
}

/// Where a `düde` ship of `kind`, flying as `ai_type`, starts at setup
/// (`_RandomShipSpawn` @0x3c0f3): a warship of no speed, when the system
/// has a `first_stellar`, [`STATIONARY_DISTANCE`] from it at an angle
/// drawn, at rest, facing anywhere (@0x3c254-0x3c324); any other ship
/// [`in_system`], at its top speed ([`at_speed`]).
pub fn dude_setup_state(
    ai_type: AiType,
    kind: &ShipKind,
    first_stellar: Option<Vec2>,
    chance: &mut (impl Chance + ?Sized),
) -> ShipState {
    let max_speed = kind.stats.handling.max_speed;
    if ai_type == AiType::Warship
        && max_speed == 0.0
        && let Some(stellar) = first_stellar
    {
        let position = stellar + facing(chance.below(DEGREES) as f32) * STATIONARY_DISTANCE;
        return ShipState {
            position,
            velocity: Vec2::ZERO,
            heading: chance.below(DEGREES) as f32,
        };
    }
    at_speed(in_system(chance), max_speed)
}

/// How fast `person` starts when setup places it, brought by a setup pass
/// (`by_pass`) or by its Person slot: its top speed, but at rest when it
/// is derelict and comes by its slot, or by a pass when `rules` take the
/// other reading of [`RuleKey::DerelictSetupSpeed`](crate::RuleKey::DerelictSetupSpeed).
fn person_setup_speed(person: &SpawnPerson, by_pass: bool, rules: &dyn PersonRules) -> f32 {
    let at_rest =
        person.derelict && (!by_pass || rules.derelict_setup_speed() == RuleSource::Bible);
    if at_rest {
        0.0
    } else {
        person.kind.stats.handling.max_speed
    }
}

/// Where a ship jumping in starts: [`jump_in_distance`] from the centre
/// at an angle drawn, facing the centre and moving in at
/// [`JUMP_IN_SPEED`].
pub fn hyperspace_entry(chance: &mut (impl Chance + ?Sized)) -> ShipState {
    let out = facing(chance.below(DEGREES) as f32);
    let heading = heading_of(Vec2::ZERO - out);
    ShipState {
        position: out * jump_in_distance(),
        velocity: facing(heading) * JUMP_IN_SPEED,
        heading,
    }
}

/// Where and how a ship of `govt` arriving alone comes in, its jump-in
/// place `jump_in` drawn already, as `_HyperShipSpawn` draws it before
/// the pick (@0x42a26, @0x42ac0): out of the gate among `stellars` that
/// [`emerge_stellar`] picks, after [`EMERGE_TICKS`], or else from
/// hyperspace at `jump_in`.
fn arrival(
    govt: Option<GovtId>,
    jump_in: ShipState,
    stellars: Stellars,
    chance: &mut (impl Chance + ?Sized),
) -> (ShipState, Entry) {
    match emerge_stellar(govt, stellars.sites, stellars.govts, chance) {
        Some(site) => emerging(site, chance),
        None => (jump_in, Entry::JumpingIn),
    }
}

/// A ship coming out of `site`: at its centre, at rest (the callers of
/// `_AIMakeShipEmergeFromHyperGate`, @0x42ae4-0x42b0a, @0x41f34-0x41f64),
/// heading its [`exit_heading`] rolled on `chance`, waiting
/// [`EMERGE_TICKS`], with `site` its travel stellar (+0x6c, @0x8954a).
fn emerging(site: &LandingSite, chance: &mut (impl Chance + ?Sized)) -> (ShipState, Entry) {
    let state = ShipState {
        position: site.position,
        velocity: Vec2::ZERO,
        heading: exit_heading(site.exit_angle, chance),
    };
    let entry = Entry::Emerging {
        ticks_left: EMERGE_TICKS,
        gate: site.id,
    };
    (state, entry)
}

/// Where an escort of a lead at `lead` starts: within 150 pixels of it
/// each way, with its heading and velocity.
pub fn escort_offset(lead: &ShipState, chance: &mut (impl Chance + ?Sized)) -> ShipState {
    let x = spread(ESCORT_SPREAD, chance);
    let y = spread(ESCORT_SPREAD, chance);
    ShipState {
        position: lead.position + Vec2::new(x, y),
        ..*lead
    }
}

/// A ship's aggression: `Rand(3) ^ 2`.
pub fn aggression(chance: &mut (impl Chance + ?Sized)) -> u8 {
    u8::try_from(chance.below(AGGRESSION_DRAW) ^ AGGRESSION_XOR).unwrap_or(0)
}

/// A draw of `Rand(spread) - spread / 2`.
fn spread(spread: u32, chance: &mut (impl Chance + ?Sized)) -> f32 {
    chance.below(spread) as f32 - (spread / 2) as f32
}

/// Whether ship type `ship` may appear by `gate`: its `AppearOn` holds,
/// or it has no record (and so spawns nothing).
fn appears(table: &SpawnTable, ship: ShipId, gate: Gate) -> bool {
    table
        .ships
        .get(&ship)
        .is_none_or(|kind| gate.allows(&kind.appear_on))
}

/// Whether a ship of `kind` can come by hyperspace: it has fuel capacity.
fn can_jump(kind: &ShipKind) -> bool {
    kind.stats.fuel > 0.0
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::catalog::{DudeId, EscortRecord, FleetRecord};
    use crate::catalog::{PersonId, PersonRecord};
    use crate::combat::hull::Condition;
    use crate::control::{ControlBits, PilotFacts};
    use crate::control::{Test, TestExpr};
    use crate::handling::ShipFields;
    use crate::market::Good;
    use crate::person::{NovaPersons, PersonWorld};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::stats::ShipStats;
    use crate::testkit::{AllowAll, Draws, FAST, RefuseBits, person};
    use crate::traffic::table::SpawnPerson;

    /// The person draw of a world without persons gone or grudging, by
    /// the engine.
    fn nobody() -> PersonDraw<'static> {
        PersonDraw::new(PersonWorld::NONE)
    }

    fn kind(inherent_ai: i16, fields: ShipFields) -> ShipKind {
        ShipKind {
            stats: ShipStats::new(fields, &[]),
            inherent_ai,
            ..ShipKind::default()
        }
    }

    fn fleet_record(id: i16, lead: i16, escorts: &[(i16, i16, i16)]) -> FleetRecord {
        FleetRecord {
            id: FleetId(id),
            lead: Some(ShipId(lead)),
            escorts: escorts
                .iter()
                .map(|&(ship, min, max)| EscortRecord {
                    ship: ShipId(ship),
                    min,
                    max,
                })
                .collect(),
            govt: Some(GovtId(131)),
            link_syst: -1,
            appear_on: Test::default(),
            quote: None,
            flags: 0,
        }
    }

    /// Two ships on average. Düde 128 (weight 100, no `AIType`, govt 130)
    /// flies ship 200 (`InherentAI` 2) or 201 (3), half and half. Fleet
    /// 140, named at 30 %, is led by ship 201 with one to three of ship 202
    /// (4), its arrival quote `STR#` 7600; fleet 141 (slot 13) links
    /// here, led by ship 200 with exactly two of ship 202, quote 7601.
    /// Ship 203 has no fuel tank.
    fn table() -> SpawnTable {
        let no_fuel = ShipFields { fuel: 0, ..FAST };
        SpawnTable {
            avg_ships: 2,
            dudes: vec![(DudeId(128), 100)],
            dude_records: BTreeMap::from([(
                DudeId(128),
                SpawnDude {
                    ai_type: 0,
                    govt: Some(GovtId(130)),
                    ships: vec![(ShipId(200), 50), (ShipId(201), 50)],
                    booty: 0x0041,
                    info_types: 0x4005,
                },
            )]),
            dude_fleets: vec![(FleetId(140), 30)],
            link_fleets: BTreeSet::from([FleetId(141)]),
            fleets: BTreeMap::from([
                (
                    FleetId(140),
                    FleetRecord {
                        quote: Some(7600),
                        ..fleet_record(140, 201, &[(202, 1, 3)])
                    },
                ),
                (
                    FleetId(141),
                    FleetRecord {
                        quote: Some(7601),
                        ..fleet_record(141, 200, &[(202, 2, 2)])
                    },
                ),
            ]),
            ships: BTreeMap::from([
                (ShipId(200), kind(2, FAST)),
                (ShipId(201), kind(3, FAST)),
                (ShipId(202), kind(4, FAST)),
                (ShipId(203), kind(1, no_fuel)),
            ]),
            persons: BTreeMap::new(),
            person_slots: Vec::new(),
        }
    }

    fn one_pass() -> SpawnTable {
        SpawnTable {
            avg_ships: 1,
            ..table()
        }
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn the_named_values() {
        assert_eq!(
            [
                PERSON_ODDS,
                FLEET_ODDS,
                ARRIVAL_ODDS,
                PERCENT,
                FLEET_SLOTS,
                SPAWN_SPREAD,
                ESCORT_SPREAD,
                DEGREES,
                JUMP_IN_TICKS
            ],
            [7, 7, 500, 100, 256, 1500, 300, 360, 43]
        );
        assert_eq!(FIRST_FLEET_ID, 128);
        assert_eq!(
            (JUMP_IN_SPEED, JUMP_IN_SLOWING, JUMP_IN_END),
            (50.0, 1.165, 1000.0)
        );
    }

    #[test]
    fn the_glide_slows_each_tick_and_starts_about_2098_out() {
        assert_eq!(glide_speed(0), 50.0);
        assert_eq!(glide_speed(1), 50.0 - 1.165);
        assert!((glide_speed(42) - (50.0 - 1.165 * 42.0)).abs() < 1e-4);
        // 1000 + 43 x 50 - 1.165 x (0 + 1 + ... + 42).
        assert!(
            (jump_in_distance() - 2098.005).abs() < 0.01,
            "{}",
            jump_in_distance()
        );
    }

    // Setup.

    #[test]
    fn a_setup_pass_drawing_a_person_spawns_nothing() {
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            initial(
                &one_pass(),
                &mut nobody(),
                None,
                Stellars::NONE,
                &mut chance
            ),
            []
        );
        assert_eq!(chance.asked, [7]);
    }

    #[test]
    fn a_setup_pass_drawing_a_fleet_rolls_the_linksyst_slot() {
        // Not a person, a fleet, slot 13 (fleet 141), at angle 0, its two
        // escorts at (-150, -150) and (149, 149) from the lead, each ship's
        // aggression drawn after its place.
        let mut chance = Draws::of(&[6, 0, 13, 0, 2, 0, 0, 2, 299, 299, 2]);
        let ships = initial(
            &one_pass(),
            &mut nobody(),
            None,
            Stellars::NONE,
            &mut chance,
        );
        assert_eq!(chance.asked, [7, 7, 256, 360, 3, 300, 300, 3, 300, 300, 3]);
        let lead = ships[0];
        assert_eq!(
            (lead.ship, lead.lead, lead.entry),
            (ShipId(200), None, Entry::JumpingIn)
        );
        assert!(near(
            lead.state.position,
            Vec2::new(0.0, -jump_in_distance())
        ));
        let escorts: Vec<_> = ships[1..]
            .iter()
            .map(|escort| {
                (
                    escort.ship,
                    escort.lead,
                    escort.state.position - lead.state.position,
                )
            })
            .collect();
        assert_eq!(escorts.len(), 2);
        assert_eq!((escorts[0].0, escorts[0].1), (ShipId(202), Some(0)));
        assert!(near(escorts[0].2, Vec2::new(-150.0, -150.0)), "{escorts:?}");
        assert!(near(escorts[1].2, Vec2::new(149.0, 149.0)), "{escorts:?}");
    }

    #[test]
    fn only_a_draw_of_0_in_7_is_a_person_or_a_fleet() {
        // 1 is neither: a düde ship.
        let mut chance = Draws::of(&[1, 1, 0, 0, 0, 0, 0]);
        assert_eq!(
            initial(
                &one_pass(),
                &mut nobody(),
                None,
                Stellars::NONE,
                &mut chance
            )
            .len(),
            1
        );
        assert_eq!(chance.asked, [7, 7, 100, 100, 1500, 1500, 360, 3]);
        let mut chance = Draws::of(&[0, 1, 1, 0, 0, 0]);
        assert_eq!(
            arrivals(&table(), 0, &mut nobody(), Stellars::NONE, &mut chance).len(),
            1
        );
        assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360, 3]);
    }

    #[test]
    fn a_setup_pass_drawing_a_dude_ship_starts_it_at_its_max_speed_along_its_heading() {
        // Not a person, not a fleet: düde 128, ship 200 (FAST: 6 pixels a
        // tick), at (-750, 749) facing 90.
        let mut chance = Draws::of(&[6, 6, 0, 0, 0, 1499, 90]);
        let ships = initial(
            &one_pass(),
            &mut nobody(),
            None,
            Stellars::NONE,
            &mut chance,
        );
        assert_eq!(chance.asked, [7, 7, 100, 100, 1500, 1500, 360, 3]);
        assert!(
            near(ships[0].state.velocity, Vec2::new(6.0, 0.0)),
            "{ships:?}"
        );
        assert_eq!(
            ships,
            [NewShip {
                ship: ShipId(200),
                govt: Some(GovtId(130)),
                ai_type: AiType::BraveTrader,
                lead: None,
                state: ShipState {
                    position: Vec2::new(-750.0, 749.0),
                    velocity: ships[0].state.velocity,
                    heading: 90.0,
                },
                entry: Entry::Placed,
                aggression: 0,
                booty: 0x0041,
                info_types: 0x4005,
                person: None,
                mission: None,
                derelict: false,
                cloaked: false,
                aux: None,
                quote: None,
                cargo: None,
            }]
        );
    }

    #[test]
    fn setup_makes_avg_ships_passes() {
        let mut persons = Draws::of(&[0, 0, 0]);
        let three = SpawnTable {
            avg_ships: 3,
            ..table()
        };
        assert_eq!(
            initial(&three, &mut nobody(), None, Stellars::NONE, &mut persons),
            []
        );
        assert_eq!(persons.asked, [7, 7, 7]);
        assert_eq!(
            initial(
                &three,
                &mut nobody(),
                None,
                Stellars::NONE,
                &mut crate::NeverFires
            )
            .len(),
            3
        );
        let none = SpawnTable {
            avg_ships: 0,
            ..table()
        };
        let mut nothing = Draws::of(&[]);
        assert_eq!(
            initial(&none, &mut nobody(), None, Stellars::NONE, &mut nothing),
            []
        );
        assert!(nothing.asked.is_empty());
    }

    #[test]
    fn a_dude_with_an_ai_type_gives_it_and_one_without_gives_the_ships() {
        let mut table = one_pass();
        let ship_201 = [6, 6, 0, 99, 0, 0, 0];
        let ships = initial(
            &table,
            &mut nobody(),
            None,
            Stellars::NONE,
            &mut Draws::of(&ship_201),
        );
        assert_eq!(ships[0].ai_type, AiType::Warship, "201's InherentAI");
        table
            .dude_records
            .get_mut(&DudeId(128))
            .expect("düde")
            .ai_type = 4;
        let ships = initial(
            &table,
            &mut nobody(),
            None,
            Stellars::NONE,
            &mut Draws::of(&ship_201),
        );
        assert_eq!(ships[0].ai_type, AiType::Interceptor, "the düde's AIType");
        table
            .dude_records
            .get_mut(&DudeId(128))
            .expect("düde")
            .ai_type = 1;
        let ships = initial(
            &table,
            &mut nobody(),
            None,
            Stellars::NONE,
            &mut Draws::of(&ship_201),
        );
        assert_eq!(ships[0].ai_type, AiType::WimpyTrader, "1 is above 0");
    }

    #[test]
    fn a_dude_ship_carries_its_dudes_booty() {
        let ship_201 = [6, 6, 0, 99, 0, 0, 0];
        let ships = initial(
            &one_pass(),
            &mut nobody(),
            None,
            Stellars::NONE,
            &mut Draws::of(&ship_201),
        );
        assert_eq!(ships[0].booty, 0x0041);
        let arrived = arrivals(
            &table(),
            1,
            &mut nobody(),
            Stellars::NONE,
            &mut Draws::of(&[0, 6, 6, 0, 99, 90]),
        );
        assert_eq!(arrived[0].booty, 0x0041, "a düde ship jumping in too");
    }

    #[test]
    fn a_fleet_ship_has_no_booty() {
        let ships = arrivals(
            &table(),
            0,
            &mut nobody(),
            Stellars::NONE,
            &mut Draws::of(&[1, 29, 0, 0, 0, 2]),
        );
        assert_eq!(ships.len(), 4);
        assert!(ships.iter().all(|ship| ship.booty == 0), "{ships:?}");
    }

    #[test]
    fn a_dude_ship_carries_its_dudes_info_types() {
        let ship_201 = [6, 6, 0, 99, 0, 0, 0];
        let ships = initial(
            &one_pass(),
            &mut nobody(),
            None,
            Stellars::NONE,
            &mut Draws::of(&ship_201),
        );
        assert_eq!(ships[0].info_types, 0x4005);
        let arrived = arrivals(
            &table(),
            1,
            &mut nobody(),
            Stellars::NONE,
            &mut Draws::of(&[0, 6, 6, 0, 99, 90]),
        );
        assert_eq!(arrived[0].info_types, 0x4005, "a düde ship jumping in too");
    }

    #[test]
    fn a_fleet_ship_has_no_info_types() {
        let ships = arrivals(
            &table(),
            0,
            &mut nobody(),
            Stellars::NONE,
            &mut Draws::of(&[1, 29, 0, 0, 0, 2]),
        );
        assert_eq!(ships.len(), 4);
        assert!(ships.iter().all(|ship| ship.info_types == 0), "{ships:?}");
    }

    #[test]
    fn a_ship_type_without_a_record_or_a_dude_without_one_spawns_nothing() {
        let mut table = one_pass();
        table.ships.remove(&ShipId(200));
        let mut chance = Draws::of(&[6, 6, 0, 0]);
        assert_eq!(
            initial(&table, &mut nobody(), None, Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [7, 7, 100, 100], "no placement drawn");
        table.dude_records.clear();
        let mut chance = Draws::of(&[6, 6, 0]);
        assert_eq!(
            initial(&table, &mut nobody(), None, Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [7, 7, 100]);
    }

    // Arrivals.

    #[test]
    fn no_ship_arrives_in_a_system_holding_avg_ships() {
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            arrivals(&table(), 2, &mut nobody(), Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(
            arrivals(&table(), 3, &mut nobody(), Stellars::NONE, &mut chance),
            []
        );
        assert!(chance.asked.is_empty(), "nothing drawn");
    }

    #[test]
    fn two_in_500_ticks_bring_an_arrival() {
        let mut chance = Draws::of(&[2]);
        assert_eq!(
            arrivals(&table(), 1, &mut nobody(), Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [500]);
        let mut chance = Draws::of(&[499]);
        assert_eq!(
            arrivals(&table(), 0, &mut nobody(), Stellars::NONE, &mut chance),
            []
        );
    }

    #[test]
    fn on_0_a_dude_ship_jumps_in() {
        // 0, not a person, not a fleet, düde 128, ship 201, at angle 90.
        let mut chance = Draws::of(&[0, 6, 6, 0, 99, 90]);
        let ships = arrivals(&table(), 1, &mut nobody(), Stellars::NONE, &mut chance);
        assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360, 3]);
        assert_eq!(ships.len(), 1);
        let ship = ships[0];
        assert_eq!(
            (ship.ship, ship.govt, ship.ai_type, ship.lead, ship.entry),
            (
                ShipId(201),
                Some(GovtId(130)),
                AiType::Warship,
                None,
                Entry::JumpingIn
            )
        );
        assert!(near(
            ship.state.position,
            Vec2::new(jump_in_distance(), 0.0)
        ));
    }

    #[test]
    fn on_0_a_person_or_a_linksyst_fleet_may_come_instead() {
        let mut person = Draws::of(&[0, 0]);
        assert_eq!(
            arrivals(&table(), 0, &mut nobody(), Stellars::NONE, &mut person),
            []
        );
        assert_eq!(person.asked, [500, 7]);
        let mut linked = Draws::of(&[0, 6, 0, 13]);
        let ships = arrivals(&table(), 0, &mut nobody(), Stellars::NONE, &mut linked);
        assert_eq!(&linked.asked[..4], [500, 7, 7, 256]);
        assert_eq!(ships.len(), 3, "fleet 141: its lead and two escorts");
        let mut missed = Draws::of(&[0, 6, 0, 14]);
        assert_eq!(
            arrivals(&table(), 0, &mut nobody(), Stellars::NONE, &mut missed),
            []
        );
        assert_eq!(missed.asked, [500, 7, 7, 256]);
    }

    #[test]
    fn on_1_a_named_fleet_comes_when_the_percentage_roll_is_within_its_odds() {
        // 1, then Rand(100) + 1 = 30, at the fleets' 30: fleet 140, at
        // angle 0, its lead's aggression, then Rand(3) = 2, so three
        // escorts.
        let mut chance = Draws::of(&[1, 29, 0, 0, 0, 2]);
        let ships = arrivals(&table(), 0, &mut nobody(), Stellars::NONE, &mut chance);
        assert_eq!(&chance.asked[..6], [500, 100, 30, 360, 3, 3]);
        assert_eq!(
            chance.asked.len(),
            6 + 3 * 3,
            "each escort's offset and aggression"
        );
        assert_eq!(ships.len(), 4);
        assert_eq!(
            (ships[0].ship, ships[0].govt, ships[0].ai_type),
            (ShipId(201), Some(GovtId(131)), AiType::Warship),
            "the lead's own InherentAI, the fleet's govt"
        );
        for escort in &ships[1..] {
            assert_eq!(
                (escort.ship, escort.govt, escort.ai_type, escort.lead),
                (ShipId(202), Some(GovtId(131)), AiType::Interceptor, Some(0))
            );
            assert_eq!(escort.entry, Entry::JumpingIn);
        }
    }

    #[test]
    fn on_1_a_percentage_roll_above_the_odds_falls_through_to_0s_case() {
        // Rand(100) + 1 = 31, above 30: falls through, and a person comes.
        let mut chance = Draws::of(&[1, 30, 0]);
        assert_eq!(
            arrivals(&table(), 0, &mut nobody(), Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [500, 100, 7]);
        // Through to a düde ship.
        let mut chance = Draws::of(&[1, 30, 6, 6, 0, 0, 0]);
        let ships = arrivals(&table(), 0, &mut nobody(), Stellars::NONE, &mut chance);
        assert_eq!(chance.asked, [500, 100, 7, 7, 100, 100, 360, 3]);
        assert_eq!(ships.len(), 1);
    }

    #[test]
    fn on_1_without_named_fleets_it_goes_straight_to_0s_case() {
        let table = SpawnTable {
            dude_fleets: Vec::new(),
            ..table()
        };
        let mut chance = Draws::of(&[1, 0]);
        assert_eq!(
            arrivals(&table, 0, &mut nobody(), Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [500, 7]);
    }

    #[test]
    fn without_a_dude_nothing_comes_on_0() {
        let table = SpawnTable {
            dudes: Vec::new(),
            dude_records: BTreeMap::new(),
            ..table()
        };
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            arrivals(&table, 0, &mut nobody(), Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [500]);
        let mut chance = Draws::of(&[1, 30]);
        assert_eq!(
            arrivals(&table, 0, &mut nobody(), Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [500, 100]);
    }

    #[test]
    fn a_ship_without_fuel_never_jumps_in() {
        let mut table = table();
        table
            .dude_records
            .get_mut(&DudeId(128))
            .expect("düde")
            .ships = vec![(ShipId(203), 1)];
        let mut chance = Draws::of(&[0, 6, 6, 0, 0]);
        assert_eq!(
            arrivals(&table, 0, &mut nobody(), Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [500, 7, 7, 100, 1], "no placement drawn");
        // Nor a fleet it leads.
        table
            .fleets
            .insert(FleetId(141), fleet_record(141, 203, &[(202, 2, 2)]));
        let mut out = Vec::new();
        let mut chance = Draws::of(&[]);
        fleet(
            &table,
            FleetId(141),
            Gate::FRESH,
            Stellars::NONE,
            FleetCall::default(),
            &mut chance,
            &mut out,
        );
        assert_eq!(out, []);
        assert!(chance.asked.is_empty());
        // Nor such an escort, though its lead comes.
        table.fleets.insert(
            FleetId(141),
            fleet_record(141, 200, &[(203, 2, 2), (999, 1, 1)]),
        );
        let mut chance = Draws::of(&[]);
        fleet(
            &table,
            FleetId(141),
            Gate::FRESH,
            Stellars::NONE,
            FleetCall::default(),
            &mut chance,
            &mut out,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(chance.asked, [360, 3]);
    }

    #[test]
    fn a_fleet_without_a_lead_or_a_record_does_not_come() {
        let mut table = table();
        let mut out = Vec::new();
        fleet(
            &table,
            FleetId(150),
            Gate::FRESH,
            Stellars::NONE,
            FleetCall::default(),
            &mut Draws::of(&[]),
            &mut out,
        );
        let mut leaderless = fleet_record(141, 200, &[(202, 2, 2)]);
        leaderless.lead = None;
        table.fleets.insert(FleetId(141), leaderless);
        fleet(
            &table,
            FleetId(141),
            Gate::FRESH,
            Stellars::NONE,
            FleetCall::default(),
            &mut Draws::of(&[]),
            &mut out,
        );
        assert_eq!(out, []);
    }

    // Fleets.

    #[test]
    fn the_linksyst_roll_hits_one_slot_in_256() {
        let mut out = Vec::new();
        let mut hit = Draws::of(&[13]);
        link_fleet(
            &table(),
            Gate::FRESH,
            Stellars::NONE,
            false,
            &mut hit,
            &mut out,
        );
        assert_eq!(out.len(), 3, "fleet 141");
        assert_eq!(hit.asked[0], 256);
        let mut out = Vec::new();
        for slot in [12, 0, 255] {
            let mut miss = Draws::of(&[slot]);
            link_fleet(
                &table(),
                Gate::FRESH,
                Stellars::NONE,
                false,
                &mut miss,
                &mut out,
            );
            assert_eq!(miss.asked, [256], "{slot}");
        }
        assert_eq!(out, [], "fleet 140 is named, not linked");
    }

    #[test]
    fn a_fleets_escorts_follow_the_index_of_their_lead_among_the_ships_added() {
        let mut out = vec![
            initial(
                &one_pass(),
                &mut nobody(),
                None,
                Stellars::NONE,
                &mut crate::NeverFires,
            )[0],
        ];
        fleet(
            &table(),
            FleetId(141),
            Gate::FRESH,
            Stellars::NONE,
            FleetCall::default(),
            &mut crate::NeverFires,
            &mut out,
        );
        assert_eq!(out.len(), 4);
        assert_eq!(
            out.iter().map(|ship| ship.lead).collect::<Vec<_>>(),
            [None, None, Some(1), Some(1)]
        );
    }

    #[test]
    fn a_lead_ai_given_flies_the_lead_so_and_leaves_its_escorts_their_own() {
        let flown = |lead_ai| {
            let mut out = Vec::new();
            let mut chance = Draws::of(&[]);
            fleet(
                &table(),
                FleetId(141),
                Gate::FRESH,
                Stellars::NONE,
                FleetCall {
                    lead_ai,
                    announced: false,
                },
                &mut chance,
                &mut out,
            );
            (
                out.iter().map(|ship| ship.ai_type).collect::<Vec<_>>(),
                chance.asked,
            )
        };
        let (own, own_draws) = flown(None);
        assert_eq!(
            own,
            [
                AiType::BraveTrader,
                AiType::Interceptor,
                AiType::Interceptor
            ]
        );
        let (led, led_draws) = flown(Some(AiType::Warship));
        assert_eq!(
            led,
            [AiType::Warship, AiType::Interceptor, AiType::Interceptor]
        );
        assert_eq!(led_draws, own_draws, "the same draws");
    }

    /// The quotes of the ships fleet `id` of `table` brings, `announced`
    /// or not.
    fn quotes_of(table: &SpawnTable, id: i16, announced: bool) -> Vec<Option<i16>> {
        let mut out = Vec::new();
        fleet(
            table,
            FleetId(id),
            Gate::FRESH,
            Stellars::NONE,
            FleetCall {
                lead_ai: None,
                announced,
            },
            &mut crate::NeverFires,
            &mut out,
        );
        out.iter().map(|ship| ship.quote).collect()
    }

    #[test]
    fn an_announced_fleet_puts_its_quote_on_its_lead_alone() {
        assert_eq!(quotes_of(&table(), 141, true), [Some(7601), None, None]);
    }

    #[test]
    fn an_unannounced_fleet_or_one_without_a_quote_puts_none() {
        assert_eq!(quotes_of(&table(), 141, false), [None; 3]);
        let mut table = table();
        table
            .fleets
            .insert(FleetId(141), fleet_record(141, 200, &[(202, 2, 2)]));
        assert_eq!(quotes_of(&table, 141, true), [None; 3]);
    }

    #[test]
    fn a_setup_linksyst_fleet_is_unannounced_and_an_arriving_one_announced() {
        let quotes = |ships: Vec<NewShip>| ships.iter().map(|ship| ship.quote).collect::<Vec<_>>();
        // Not a person, a fleet, slot 13 (fleet 141).
        let mut chance = Draws::of(&[6, 0, 13]);
        let set_up = initial(
            &one_pass(),
            &mut nobody(),
            None,
            Stellars::NONE,
            &mut chance,
        );
        assert_eq!(quotes(set_up), [None; 3], "_SetupShipsInSystem passes 0");
        let mut chance = Draws::of(&[0, 6, 0, 13]);
        let linked = arrivals(&table(), 0, &mut nobody(), Stellars::NONE, &mut chance);
        assert_eq!(
            quotes(linked),
            [Some(7601), None, None],
            "_HyperShipSpawn passes 1"
        );
        let mut chance = Draws::of(&[1, 29, 0, 0, 0, 0]);
        let named = arrivals(&table(), 0, &mut nobody(), Stellars::NONE, &mut chance);
        assert_eq!(
            quotes(named),
            [Some(7600), None],
            "_EnterMoreShips passes 1"
        );
    }

    // Random cargo.

    /// A ship kind of `inherent_ai` and `holds` tons.
    fn holding(inherent_ai: i16, holds: u16) -> ShipKind {
        ShipKind {
            holds,
            ..kind(inherent_ai, FAST)
        }
    }

    /// `flët` 150 of `flags`, led by ship 210 (`InherentAI` `lead_ai`, 20
    /// tons), with one ship 211 (`InherentAI` 2, 10 tons) and one ship 212
    /// (`InherentAI` 3, 30 tons).
    fn cargo_table(flags: u16, lead_ai: i16) -> SpawnTable {
        SpawnTable {
            fleets: BTreeMap::from([(
                FleetId(150),
                FleetRecord {
                    flags,
                    ..fleet_record(150, 210, &[(211, 1, 1), (212, 1, 1)])
                },
            )]),
            ships: BTreeMap::from([
                (ShipId(210), holding(lead_ai, 20)),
                (ShipId(211), holding(2, 10)),
                (ShipId(212), holding(3, 30)),
            ]),
            ..SpawnTable::default()
        }
    }

    /// The cargo of each ship fleet 150 of `table` brings, called as
    /// `call`, drawn on `chance`.
    fn cargoes(
        table: &SpawnTable,
        call: FleetCall,
        chance: &mut Draws,
    ) -> Vec<Option<(Good, u32)>> {
        let mut out = Vec::new();
        fleet(
            table,
            FleetId(150),
            Gate::FRESH,
            Stellars::NONE,
            call,
            chance,
            &mut out,
        );
        out.iter().map(|ship| ship.cargo).collect()
    }

    #[test]
    fn a_random_cargo_fleets_trader_ships_each_carry_one_commodity() {
        let table = cargo_table(RANDOM_CARGO, 1);
        let mut last = Draws::of(&[]);
        assert_eq!(
            cargoes(&table, FleetCall::default(), &mut last),
            [
                Some((Good::Commodity(5), 20)),
                Some((Good::Commodity(5), 10)),
                None
            ],
            "Rand(6) and Rand(holds) + 1, for InherentAI 1 and 2, not 3"
        );
        assert_eq!(last.asked.iter().filter(|&&n| n == 6).count(), 2);
        assert!(last.asked.contains(&20) && last.asked.contains(&10));
        assert!(!last.asked.contains(&30), "the warship draws no tons");
        let mut first = Draws::of(&[0; 32]);
        assert_eq!(
            cargoes(&table, FleetCall::default(), &mut first),
            [
                Some((Good::Commodity(0), 1)),
                Some((Good::Commodity(0), 1)),
                None
            ]
        );
    }

    #[test]
    fn a_class_of_inherent_ai_0_or_below_carries_cargo() {
        for lead_ai in [0, -1] {
            let cargo = cargoes(
                &cargo_table(RANDOM_CARGO, lead_ai),
                FleetCall::default(),
                &mut Draws::of(&[]),
            );
            assert_eq!(cargo[0], Some((Good::Commodity(5), 20)), "{lead_ai}");
        }
    }

    #[test]
    fn a_lead_carries_cargo_by_its_classs_inherent_ai_not_the_ai_it_flies_as() {
        let flown = |lead_ai: i16, flies: AiType| {
            let call = FleetCall {
                lead_ai: Some(flies),
                announced: false,
            };
            cargoes(
                &cargo_table(RANDOM_CARGO, lead_ai),
                call,
                &mut Draws::of(&[]),
            )[0]
        };
        assert_eq!(
            flown(1, AiType::Interceptor),
            Some((Good::Commodity(5), 20))
        );
        assert_eq!(flown(3, AiType::WimpyTrader), None);
    }

    #[test]
    fn a_class_without_holds_carries_none() {
        let mut table = cargo_table(RANDOM_CARGO, 1);
        table.ships.insert(ShipId(210), holding(1, 0));
        let mut chance = Draws::of(&[]);
        assert_eq!(cargoes(&table, FleetCall::default(), &mut chance)[0], None);
        assert_eq!(
            chance.asked.iter().filter(|&&n| n == 6).count(),
            1,
            "only the escort's"
        );
    }

    #[test]
    fn without_the_flag_nobody_carries_cargo_and_no_cargo_is_drawn() {
        let mut chance = Draws::of(&[]);
        assert_eq!(
            cargoes(&cargo_table(0, 1), FleetCall::default(), &mut chance),
            [None; 3]
        );
        assert!(!chance.asked.contains(&6));
        let mut other_flags = Draws::of(&[]);
        assert_eq!(
            cargoes(
                &cargo_table(0xFFFE, 1),
                FleetCall::default(),
                &mut other_flags
            ),
            [None; 3]
        );
    }

    #[test]
    fn escort_counts_run_from_min_to_max() {
        for (draw, count) in [(0, 1), (1, 2), (2, 3)] {
            let mut chance = Draws::of(&[draw]);
            assert_eq!(escort_count(1, 3, &mut chance), count);
            assert_eq!(chance.asked, [3]);
        }
        let mut chance = Draws::of(&[]);
        assert_eq!(escort_count(2, 2, &mut chance), 2);
        assert_eq!(escort_count(3, 1, &mut chance), 3, "Max below Min");
        assert_eq!(escort_count(-1, -1, &mut chance), 0);
        assert_eq!(escort_count(-3, -5, &mut chance), 0);
        assert!(chance.asked.is_empty(), "no draw over one outcome or none");
        let mut chance = Draws::of(&[1]);
        assert_eq!(escort_count(-2, 1, &mut chance), 1, "none below none");
        assert_eq!(chance.asked, [2]);
    }

    // AppearOn.

    /// The person draw of a world whose control bits refuse bit 7.
    fn refusing_7() -> PersonDraw<'static> {
        PersonDraw::new(PersonWorld {
            control_bits: &RefuseBits(&[7]),
            ..PersonWorld::NONE
        })
    }

    /// The person draw of a world whose control bits hold every test.
    fn allowing() -> PersonDraw<'static> {
        PersonDraw::new(PersonWorld {
            control_bits: &AllowAll,
            ..PersonWorld::NONE
        })
    }

    /// [`one_pass`] with ship `id`'s `AppearOn` `appear_on`.
    fn appearing(mut table: SpawnTable, id: i16, appear_on: &str) -> SpawnTable {
        table.ships.get_mut(&ShipId(id)).expect("a kind").appear_on = Test::parse(appear_on);
        table
    }

    #[test]
    fn a_dude_ship_type_whose_appear_on_is_refused_is_dropped_before_the_draw() {
        // `_SelectShipFieldFromDude` sums only the ship types whose
        // AppearOn holds: Rand(50), and ship 200 (@0x662b, @0x66c7).
        let gated = appearing(one_pass(), 201, "b7");
        let mut chance = Draws::of(&[6, 6, 0, 49, 0, 0, 0]);
        let ships = initial(&gated, &mut refusing_7(), None, Stellars::NONE, &mut chance);
        assert_eq!(chance.asked, [7, 7, 100, 50, 1500, 1500, 360, 3]);
        assert_eq!(
            ships.iter().map(|ship| ship.ship).collect::<Vec<_>>(),
            [ShipId(200)]
        );
        // On arrival too.
        let gated = appearing(table(), 201, "b7");
        let mut chance = Draws::of(&[0, 6, 6, 0, 49, 0, 0]);
        let ships = arrivals(&gated, 0, &mut refusing_7(), Stellars::NONE, &mut chance);
        assert_eq!(chance.asked, [500, 7, 7, 100, 50, 360, 3]);
        assert_eq!(
            ships.iter().map(|ship| ship.ship).collect::<Vec<_>>(),
            [ShipId(200)]
        );
        // Under a new pilot's bits, bit 7 is clear too.
        let gated = appearing(one_pass(), 201, "b7");
        let mut chance = Draws::of(&[6, 6, 0, 49, 0, 0, 0]);
        initial(&gated, &mut nobody(), None, Stellars::NONE, &mut chance);
        assert_eq!(chance.asked[3], 50);
    }

    #[test]
    fn a_dude_whose_every_ship_type_is_refused_brings_nothing() {
        let gated = appearing(appearing(one_pass(), 200, "b7"), 201, "!b7");
        let mut chance = Draws::of(&[6, 6, 0]);
        assert_eq!(
            initial(&gated, &mut refusing_7(), None, Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [7, 7, 100], "no ship drawn");
        let gated = appearing(appearing(table(), 200, "b7"), 201, "b7");
        let mut chance = Draws::of(&[0, 6, 6, 0]);
        assert_eq!(
            arrivals(&gated, 0, &mut refusing_7(), Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [500, 7, 7, 100]);
    }

    #[test]
    fn a_dude_ship_type_with_a_malformed_appear_on_never_comes() {
        let gated = appearing(one_pass(), 200, "b1 &");
        let mut chance = Draws::of(&[6, 6, 0, 0, 0, 0, 0]);
        let ships = initial(&gated, &mut allowing(), None, Stellars::NONE, &mut chance);
        assert_eq!(chance.asked[3], 50);
        assert_eq!(ships[0].ship, ShipId(201));
    }

    #[test]
    fn a_fleets_lead_comes_whatever_its_ship_types_appear_on() {
        // `_HyperSpawnFleet` tests no AppearOn of the lead's ship type.
        let gated = appearing(table(), 200, "b7");
        let mut out = Vec::new();
        let gate = refusing_7().world.gate();
        fleet(
            &gated,
            FleetId(141),
            gate,
            Stellars::NONE,
            FleetCall::default(),
            &mut crate::NeverFires,
            &mut out,
        );
        assert_eq!(out.len(), 3, "the lead and its two escorts");
    }

    #[test]
    fn a_fleet_escort_type_whose_appear_on_is_refused_does_not_come() {
        // Its count is drawn, then its ship type's AppearOn tested
        // (`_HyperSpawnFleet` @0x42143, @0x4217f).
        let gated = appearing(table(), 202, "b7");
        let gate = refusing_7().world.gate();
        let mut out = Vec::new();
        let mut chance = Draws::of(&[0, 0, 2]);
        fleet(
            &gated,
            FleetId(140),
            gate,
            Stellars::NONE,
            FleetCall::default(),
            &mut chance,
            &mut out,
        );
        assert_eq!(chance.asked, [360, 3, 3], "the lead, then the count");
        assert_eq!(
            out.iter().map(|ship| ship.ship).collect::<Vec<_>>(),
            [ShipId(201)]
        );
        let malformed = appearing(gated, 202, "b1 &");
        let mut out = Vec::new();
        fleet(
            &malformed,
            FleetId(140),
            allowing().world.gate(),
            Stellars::NONE,
            FleetCall::default(),
            &mut crate::NeverFires,
            &mut out,
        );
        assert_eq!(out.len(), 1);
    }

    /// [`table`] with fleet `id`'s `AppearOn` `appear_on`.
    fn fleet_appearing(mut table: SpawnTable, id: i16, appear_on: &str) -> SpawnTable {
        table
            .fleets
            .get_mut(&FleetId(id))
            .expect("a fleet")
            .appear_on = Test::parse(appear_on);
        table
    }

    #[test]
    fn a_fleet_whose_appear_on_is_refused_brings_nothing_and_draws_nothing() {
        let gated = fleet_appearing(table(), 141, "b7");
        let mut out = Vec::new();
        let mut chance = Draws::of(&[]);
        fleet(
            &gated,
            FleetId(141),
            refusing_7().world.gate(),
            Stellars::NONE,
            FleetCall::default(),
            &mut chance,
            &mut out,
        );
        assert_eq!(out, []);
        assert!(chance.asked.is_empty());
        fleet(
            &gated,
            FleetId(141),
            Gate::FRESH,
            Stellars::NONE,
            FleetCall::default(),
            &mut chance,
            &mut out,
        );
        assert_eq!(out, [], "bit 7 is clear");
        let malformed = fleet_appearing(gated, 141, "b1 &");
        fleet(
            &malformed,
            FleetId(141),
            allowing().world.gate(),
            Stellars::NONE,
            FleetCall::default(),
            &mut chance,
            &mut out,
        );
        assert_eq!(out, []);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn a_linksyst_slot_whose_fleets_appear_on_is_refused_brings_nothing() {
        let gated = fleet_appearing(table(), 141, "b7");
        let mut out = Vec::new();
        let mut hit = Draws::of(&[13]);
        link_fleet(
            &gated,
            refusing_7().world.gate(),
            Stellars::NONE,
            false,
            &mut hit,
            &mut out,
        );
        assert_eq!(hit.asked, [256]);
        assert_eq!(out, []);
        // At setup, the pass brings nothing more.
        let mut chance = Draws::of(&[6, 0, 13]);
        assert_eq!(
            initial(
                &SpawnTable {
                    avg_ships: 1,
                    ..gated
                },
                &mut refusing_7(),
                None,
                Stellars::NONE,
                &mut chance
            ),
            []
        );
        assert_eq!(chance.asked, [7, 7, 256]);
    }

    #[test]
    fn named_fleets_whose_appear_on_is_refused_are_dropped_before_the_pick() {
        // `_SelectFleetFromSystem` sums only the fleets whose AppearOn holds
        // (@0x6884), against the odds of them all: 30 + 20 = 50, Rand(20).
        let mut gated = fleet_appearing(table(), 140, "b7");
        gated.dude_fleets.push((FleetId(141), 20));
        let mut chance = Draws::of(&[1, 49, 19, 0, 0]);
        let ships = arrivals(&gated, 0, &mut refusing_7(), Stellars::NONE, &mut chance);
        assert_eq!(&chance.asked[..4], [500, 100, 20, 360]);
        assert_eq!(ships[0].ship, ShipId(200), "fleet 141's lead");
    }

    #[test]
    fn when_every_named_fleet_is_refused_the_arrival_falls_through_to_0s_case() {
        // `_EnterMoreShips` goes on to a düde when no fleet is picked.
        let gated = fleet_appearing(table(), 140, "b7");
        let mut chance = Draws::of(&[1, 0, 6, 6, 0, 0, 0]);
        let ships = arrivals(&gated, 0, &mut refusing_7(), Stellars::NONE, &mut chance);
        assert_eq!(chance.asked, [500, 100, 7, 7, 100, 100, 360, 3]);
        assert_eq!(ships.len(), 1, "a düde ship");
        let malformed = fleet_appearing(gated, 140, "b1 &");
        let mut chance = Draws::of(&[1, 0, 0]);
        assert_eq!(
            arrivals(&malformed, 0, &mut allowing(), Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [500, 100, 7]);
        // A named fleet with no record is not picked either.
        let unread = SpawnTable {
            dude_fleets: vec![(FleetId(150), 30)],
            ..table()
        };
        let mut chance = Draws::of(&[1, 0, 0]);
        assert_eq!(
            arrivals(&unread, 0, &mut nobody(), Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [500, 100, 7]);
    }

    // Aggression.

    #[test]
    fn a_ships_aggression_is_its_last_draw_of_3_xor_2() {
        for (draw, aggression) in [(0, 2), (1, 3), (2, 0)] {
            // A düde ship placed at setup.
            let mut chance = Draws::of(&[6, 6, 0, 0, 0, 0, 0, draw]);
            let ships = initial(
                &one_pass(),
                &mut nobody(),
                None,
                Stellars::NONE,
                &mut chance,
            );
            assert_eq!(chance.asked, [7, 7, 100, 100, 1500, 1500, 360, 3]);
            assert_eq!(ships[0].aggression, aggression, "{draw}");
            // One jumping in.
            let mut chance = Draws::of(&[0, 6, 6, 0, 0, 0, draw]);
            let ships = arrivals(&table(), 0, &mut nobody(), Stellars::NONE, &mut chance);
            assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360, 3]);
            assert_eq!(ships[0].aggression, aggression, "{draw}");
        }
        assert_eq!(AGGRESSION_DRAW, 3);
    }

    #[test]
    fn each_ship_of_a_fleet_draws_its_aggression_after_its_place() {
        // Fleet 141: its lead at angle 0 (aggression 3), then its two
        // escorts, each placed and then its aggression drawn (0, then 2).
        let mut out = Vec::new();
        let mut chance = Draws::of(&[0, 1, 150, 150, 2, 150, 150, 0]);
        fleet(
            &table(),
            FleetId(141),
            Gate::FRESH,
            Stellars::NONE,
            FleetCall::default(),
            &mut chance,
            &mut out,
        );
        assert_eq!(chance.asked, [360, 3, 300, 300, 3, 300, 300, 3]);
        assert_eq!(
            out.iter().map(|ship| ship.aggression).collect::<Vec<_>>(),
            [3, 0, 2]
        );
    }

    // Stationary warships.

    /// The fields of a ship of no speed.
    const STILL: ShipFields = ShipFields { speed: 0, ..FAST };

    /// Where the system's first stellar is in these tests.
    const FIRST_STELLAR: Vec2 = Vec2::new(300.0, -200.0);

    /// [`one_pass`] with ship 201 (`InherentAI` 3, a warship) of no speed.
    fn still_warship() -> SpawnTable {
        let mut table = one_pass();
        table.ships.insert(ShipId(201), kind(3, STILL));
        table
    }

    #[test]
    fn a_warship_of_no_speed_is_placed_100_from_the_first_stellar_at_rest() {
        // Not a person, not a fleet: düde 128, ship 201, at angle 90 from
        // the stellar, facing 45, of aggression 3.
        let mut chance = Draws::of(&[6, 6, 0, 99, 90, 45, 1]);
        let ships = initial(
            &still_warship(),
            &mut nobody(),
            Some(FIRST_STELLAR),
            Stellars::NONE,
            &mut chance,
        );
        assert_eq!(chance.asked, [7, 7, 100, 100, 360, 360, 3]);
        let ship = ships[0];
        assert_eq!(
            (ship.ship, ship.ai_type, ship.aggression),
            (ShipId(201), AiType::Warship, 3)
        );
        assert!(
            near(ship.state.position, FIRST_STELLAR + Vec2::new(100.0, 0.0)),
            "{ship:?}"
        );
        assert!(((ship.state.position - FIRST_STELLAR).length() - 100.0).abs() < 1e-3);
        assert_eq!(
            (ship.state.velocity, ship.state.heading),
            (Vec2::ZERO, 45.0)
        );
        let mut chance = Draws::of(&[6, 6, 0, 99, 180, 0, 0]);
        let ships = initial(
            &still_warship(),
            &mut nobody(),
            Some(FIRST_STELLAR),
            Stellars::NONE,
            &mut chance,
        );
        assert!(
            near(
                ships[0].state.position,
                FIRST_STELLAR + Vec2::new(0.0, 100.0)
            ),
            "{ships:?}"
        );
    }

    #[test]
    fn with_no_first_stellar_a_warship_of_no_speed_is_placed_in_the_box() {
        let mut chance = Draws::of(&[6, 6, 0, 99, 0, 1499, 45, 0]);
        let ships = initial(
            &still_warship(),
            &mut nobody(),
            None,
            Stellars::NONE,
            &mut chance,
        );
        assert_eq!(chance.asked, [7, 7, 100, 100, 1500, 1500, 360, 3]);
        assert_eq!(ships[0].state.position, Vec2::new(-750.0, 749.0));
        assert_eq!(ships[0].state.velocity, Vec2::ZERO);
    }

    #[test]
    fn a_moving_warship_or_a_still_trader_is_placed_in_the_box() {
        // Ship 201 at its top speed, facing 90.
        let mut chance = Draws::of(&[6, 6, 0, 99, 0, 1499, 90, 0]);
        let ships = initial(
            &one_pass(),
            &mut nobody(),
            Some(FIRST_STELLAR),
            Stellars::NONE,
            &mut chance,
        );
        assert_eq!(chance.asked, [7, 7, 100, 100, 1500, 1500, 360, 3]);
        assert_eq!(ships[0].state.position, Vec2::new(-750.0, 749.0));
        assert!(near(ships[0].state.velocity, Vec2::new(6.0, 0.0)));
        // Ship 200, a brave trader (`InherentAI` 2) of no speed.
        let mut table = one_pass();
        table.ships.insert(ShipId(200), kind(2, STILL));
        let mut chance = Draws::of(&[6, 6, 0, 0, 0, 1499, 90, 0]);
        let ships = initial(
            &table,
            &mut nobody(),
            Some(FIRST_STELLAR),
            Stellars::NONE,
            &mut chance,
        );
        assert_eq!(chance.asked, [7, 7, 100, 100, 1500, 1500, 360, 3]);
        assert_eq!(ships[0].state.position, Vec2::new(-750.0, 749.0));
    }

    #[test]
    fn a_dude_of_ai_type_3_makes_a_still_ship_a_stationary_warship() {
        let mut table = one_pass();
        table.ships.insert(ShipId(200), kind(2, STILL));
        table
            .dude_records
            .get_mut(&DudeId(128))
            .expect("there")
            .ai_type = 3;
        let mut chance = Draws::of(&[6, 6, 0, 0, 90, 45, 0]);
        let ships = initial(
            &table,
            &mut nobody(),
            Some(FIRST_STELLAR),
            Stellars::NONE,
            &mut chance,
        );
        assert_eq!(chance.asked, [7, 7, 100, 100, 360, 360, 3]);
        assert!(near(
            ships[0].state.position,
            FIRST_STELLAR + Vec2::new(100.0, 0.0)
        ));
    }

    #[test]
    fn a_person_is_never_placed_by_the_first_stellar() {
        let mut table = peopled();
        for person in table.persons.values_mut() {
            person.kind = kind(3, STILL);
        }
        // Rolled in a pass.
        let mut chance = Draws::of(&[0, 382, 0, 1499, 90, 99]);
        let ships = initial(
            &table,
            &mut nobody(),
            Some(FIRST_STELLAR),
            Stellars::NONE,
            &mut chance,
        );
        assert_eq!(chance.asked, [7, 1022, 1500, 1500, 360, 100]);
        assert_eq!(ships[0].ai_type, AiType::Warship);
        assert_eq!(ships[0].state.position, Vec2::new(-750.0, 749.0));
        // In its slot.
        let slots = SpawnTable {
            avg_ships: 0,
            ..table
        };
        let mut chance = Draws::of(&[49, 0, 1499, 90]);
        let ships = initial(
            &slots,
            &mut nobody(),
            Some(FIRST_STELLAR),
            Stellars::NONE,
            &mut chance,
        );
        assert_eq!(chance.asked, [100, 1500, 1500, 360]);
        assert_eq!(ships[0].state.position, Vec2::new(-750.0, 749.0));
    }

    // Placement.

    #[test]
    fn a_ship_placed_at_setup_is_within_750_each_way_at_rest() {
        let state = in_system(&mut Draws::of(&[0, 0, 0]));
        assert_eq!(state.position, Vec2::new(-750.0, -750.0));
        assert_eq!((state.velocity, state.heading), (Vec2::ZERO, 0.0));
        let state = in_system(&mut Draws::of(&[1499, 750, 359]));
        assert_eq!(state.position, Vec2::new(749.0, 0.0));
        assert_eq!(state.heading, 359.0);
    }

    #[test]
    fn a_setup_ship_moves_at_its_top_speed_along_its_heading() {
        let placed = |heading| ShipState {
            position: Vec2::new(10.0, -20.0),
            velocity: Vec2::ZERO,
            heading,
        };
        let east = at_speed(placed(90.0), 6.0);
        assert!(near(east.velocity, Vec2::new(6.0, 0.0)), "{east:?}");
        assert_eq!(
            (east.position, east.heading),
            (Vec2::new(10.0, -20.0), 90.0)
        );
        let south = at_speed(placed(180.0), 2.5);
        assert!(near(south.velocity, Vec2::new(0.0, 2.5)), "{south:?}");
        assert_eq!(at_speed(placed(37.0), 0.0).velocity, Vec2::ZERO);
    }

    #[test]
    fn a_ship_jumping_in_starts_about_2098_out_heading_in_at_50() {
        for angle in [0, 90, 135, 359] {
            let state = hyperspace_entry(&mut Draws::of(&[angle]));
            let out = facing(angle as f32);
            assert!(
                near(state.position, out * jump_in_distance()),
                "{angle}: {state:?}"
            );
            assert!(
                near(state.velocity, out * -JUMP_IN_SPEED),
                "{angle}: {state:?}"
            );
            assert!(
                near(facing(state.heading), out * -1.0),
                "{angle}: {state:?}"
            );
            assert!((0.0..360.0).contains(&state.heading));
        }
        let mut chance = Draws::of(&[]);
        hyperspace_entry(&mut chance);
        assert_eq!(chance.asked, [360]);
    }

    #[test]
    fn an_escort_starts_within_150_of_its_lead_with_its_heading_and_velocity() {
        let lead = ShipState {
            position: Vec2::new(100.0, -200.0),
            velocity: Vec2::new(3.0, 4.0),
            heading: 37.0,
        };
        let state = escort_offset(&lead, &mut Draws::of(&[0, 299]));
        assert_eq!(state.position, Vec2::new(-50.0, -51.0));
        assert_eq!(
            (state.velocity, state.heading),
            (lead.velocity, lead.heading)
        );
        let state = escort_offset(&lead, &mut Draws::of(&[150, 150]));
        assert_eq!(state.position, lead.position);
    }

    // Persons.

    /// Person `id` flying ship 201, linked here or not, of `aggress` and
    /// government 128.
    fn spawn_person(id: i16, linked: bool, aggress: i16) -> SpawnPerson {
        let kind = kind(3, FAST);
        SpawnPerson {
            record: PersonRecord {
                govt: Some(GovtId(128)),
                aggress,
                ..person(id, 201)
            },
            linked,
            reserves: kind.stats.full(),
            kind,
            condition: Condition::Intact,
            derelict: false,
        }
    }

    /// [`one_pass`] with person 510 (`Aggress` 4) linked here, and 600,
    /// unlinked, in the first Person slot at 50 %.
    fn peopled() -> SpawnTable {
        SpawnTable {
            persons: BTreeMap::from([
                (PersonId(510), spawn_person(510, true, 4)),
                (PersonId(600), spawn_person(600, false, 1)),
            ]),
            person_slots: vec![(PersonId(600), 50)],
            ..one_pass()
        }
    }

    /// Refuses every test that reads its bit.
    #[derive(Debug)]
    struct Refusing(u16);

    impl ControlBits for Refusing {
        fn allows(&self, test: &TestExpr, _pilot: &dyn PilotFacts) -> bool {
            !test.reads().iter().any(|bit| bit.get() == self.0)
        }
    }

    /// The setup of `table` in `world`, drawn from `draws`, and the
    /// bounds asked.
    fn set_up(table: &SpawnTable, world: PersonWorld, draws: &[u32]) -> (Vec<NewShip>, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let ships = initial(
            table,
            &mut PersonDraw::new(world),
            None,
            Stellars::NONE,
            &mut chance,
        );
        (ships, chance.asked)
    }

    fn persons_of(ships: &[NewShip]) -> Vec<Option<i16>> {
        ships
            .iter()
            .map(|ship| ship.person.map(|id| id.0))
            .collect()
    }

    #[test]
    fn a_setup_pass_rolling_a_person_places_it_with_its_own_aggression() {
        let (ships, asked) = set_up(&peopled(), PersonWorld::NONE, &[0, 382, 0, 1499, 90, 99]);
        assert_eq!(
            asked,
            [7, 1022, 1500, 1500, 360, 100],
            "no aggression drawn"
        );
        assert!(
            near(ships[0].state.velocity, Vec2::new(6.0, 0.0)),
            "{ships:?}"
        );
        assert_eq!(
            ships,
            [NewShip {
                ship: ShipId(201),
                govt: Some(GovtId(128)),
                ai_type: AiType::Warship,
                lead: None,
                state: ShipState {
                    position: Vec2::new(-750.0, 749.0),
                    velocity: ships[0].state.velocity,
                    heading: 90.0,
                },
                entry: Entry::Placed,
                aggression: 4,
                booty: 0,
                info_types: 0,
                person: Some(PersonId(510)),
                mission: None,
                derelict: false,
                cloaked: false,
                aux: None,
                quote: None,
                cargo: None,
            }]
        );
    }

    #[test]
    fn a_person_roll_that_misses_brings_nothing_that_pass() {
        let two = SpawnTable {
            avg_ships: 2,
            ..peopled()
        };
        let (ships, asked) = set_up(
            &two,
            PersonWorld::NONE,
            &[0, 381, 6, 6, 0, 0, 0, 0, 0, 0, 99],
        );
        assert_eq!(&asked[..2], [7, 1022], "the first pass: nothing");
        assert_eq!(&asked[2..4], [7, 7], "the next pass rolls as usual");
        assert_eq!(persons_of(&ships), [None]);
    }

    #[test]
    fn a_person_gone_refused_by_its_active_on_or_of_no_ai_type_does_not_come() {
        let gone = BTreeSet::from([PersonId(510)]);
        let world = PersonWorld {
            gone: &gone,
            ..PersonWorld::NONE
        };
        assert_eq!(set_up(&peopled(), world, &[0, 99]).1, [7, 100], "empty");
        let mut table = peopled();
        table
            .persons
            .get_mut(&PersonId(510))
            .expect("there")
            .record
            .active_on = Test::parse("b8");
        let refusing = Refusing(8);
        let world = PersonWorld {
            control_bits: &refusing,
            ..PersonWorld::NONE
        };
        assert_eq!(set_up(&table, world, &[0, 99]).1, [7, 100]);
        let mut table = peopled();
        table
            .persons
            .get_mut(&PersonId(510))
            .expect("there")
            .record
            .ai_type = 0;
        assert_eq!(set_up(&table, PersonWorld::NONE, &[0, 99]).1, [7, 100]);
    }

    #[test]
    fn a_person_whose_name_is_in_the_system_does_not_come_again() {
        let mut draw = PersonDraw::new(PersonWorld::NONE);
        draw.here.insert("Person 510".to_owned());
        assert_eq!(draw.eligible(&peopled(), false), []);
        let two = SpawnTable {
            avg_ships: 2,
            ..peopled()
        };
        // The first pass brings 510; the second's roll lands on it again.
        let (ships, asked) = set_up(&two, PersonWorld::NONE, &[0, 382, 0, 0, 0, 0, 99]);
        assert_eq!(persons_of(&ships), [Some(510)]);
        assert_eq!(&asked[5..], [7, 100], "nobody may come: no second draw");
    }

    #[test]
    fn a_derelict_person_may_come_at_setup_but_never_jumps_in() {
        let mut table = peopled();
        table
            .persons
            .get_mut(&PersonId(510))
            .expect("there")
            .derelict = true;
        let draw = PersonDraw::new(PersonWorld::NONE);
        assert_eq!(draw.eligible(&table, false), [PersonId(510)]);
        assert_eq!(draw.eligible(&table, true), []);
        assert_eq!(draw.eligible(&peopled(), true), [PersonId(510)]);
    }

    #[test]
    fn after_the_passes_each_person_slot_is_rolled_and_brings_its_person() {
        let none = SpawnTable {
            avg_ships: 0,
            ..peopled()
        };
        let (ships, asked) = set_up(&none, PersonWorld::NONE, &[49, 0, 0, 0]);
        assert_eq!(asked, [100, 1500, 1500, 360]);
        assert_eq!(persons_of(&ships), [Some(600)], "linked or not");
        assert_eq!(ships[0].aggression, 1);
        assert_eq!(ships[0].entry, Entry::Placed);
        let (ships, asked) = set_up(&none, PersonWorld::NONE, &[50]);
        assert_eq!((ships.len(), asked), (0, vec![100]));
    }

    #[test]
    fn a_person_at_setup_starts_at_its_own_ships_top_speed_along_its_heading() {
        let mut table = peopled();
        for id in [510, 600] {
            let slower = ShipFields { speed: 250, ..FAST };
            table.persons.get_mut(&PersonId(id)).expect("there").kind = kind(3, slower);
        }
        // Rolled in a pass, facing 90.
        let (ships, _) = set_up(&table, PersonWorld::NONE, &[0, 382, 0, 1499, 90, 99]);
        assert_eq!(persons_of(&ships), [Some(510)]);
        assert!(
            near(ships[0].state.velocity, Vec2::new(2.5, 0.0)),
            "{ships:?}"
        );
        // In its slot, facing 180.
        let none = SpawnTable {
            avg_ships: 0,
            ..table
        };
        let (ships, _) = set_up(&none, PersonWorld::NONE, &[49, 0, 0, 180]);
        assert_eq!(persons_of(&ships), [Some(600)]);
        assert!(
            near(ships[0].state.velocity, Vec2::new(0.0, 2.5)),
            "{ships:?}"
        );
    }

    /// The velocity [`peopled`]'s persons, derelict or not, start at in
    /// `world`: 510 brought by a pass facing 90, and 600 by its slot
    /// facing 180.
    fn setup_velocities(derelict: bool, world: PersonWorld) -> (Vec2, Vec2) {
        let mut table = peopled();
        for person in table.persons.values_mut() {
            person.derelict = derelict;
        }
        let (pass, _) = set_up(&table, world, &[0, 382, 0, 1499, 90, 99]);
        assert_eq!(persons_of(&pass), [Some(510)]);
        let slots = SpawnTable {
            avg_ships: 0,
            ..table
        };
        let (slot, _) = set_up(&slots, world, &[49, 0, 0, 180]);
        assert_eq!(persons_of(&slot), [Some(600)]);
        (pass[0].state.velocity, slot[0].state.velocity)
    }

    #[test]
    fn by_the_engine_a_derelict_starts_at_speed_from_a_pass_and_at_rest_from_its_slot() {
        let (pass, slot) = setup_velocities(true, PersonWorld::NONE);
        assert!(near(pass, Vec2::new(6.0, 0.0)), "{pass:?}");
        assert_eq!(slot, Vec2::ZERO);
    }

    #[test]
    fn by_the_other_reading_a_derelict_starts_at_rest_either_way() {
        let rules = by(RuleKey::DerelictSetupSpeed);
        let world = PersonWorld {
            rules: &rules,
            ..PersonWorld::NONE
        };
        assert_eq!(setup_velocities(true, world), (Vec2::ZERO, Vec2::ZERO));
        let (pass, slot) = setup_velocities(false, world);
        assert!(near(pass, Vec2::new(6.0, 0.0)), "{pass:?}");
        assert!(near(slot, Vec2::new(0.0, 6.0)), "not derelict: {slot:?}");
    }

    #[test]
    fn a_slot_naming_a_person_gone_refused_or_without_a_ship_draws_nothing() {
        let none = SpawnTable {
            avg_ships: 0,
            ..peopled()
        };
        let gone = BTreeSet::from([PersonId(600)]);
        let world = PersonWorld {
            gone: &gone,
            ..PersonWorld::NONE
        };
        assert_eq!(set_up(&none, world, &[]), (vec![], vec![]));
        let mut table = none.clone();
        table
            .persons
            .get_mut(&PersonId(600))
            .expect("there")
            .record
            .active_on = Test::parse("b9");
        let refusing = Refusing(9);
        let world = PersonWorld {
            control_bits: &refusing,
            ..PersonWorld::NONE
        };
        assert_eq!(set_up(&table, world, &[]), (vec![], vec![]));
        let shipless = SpawnTable {
            person_slots: vec![(PersonId(605), 100)],
            ..none
        };
        assert_eq!(set_up(&shipless, PersonWorld::NONE, &[]), (vec![], vec![]));
    }

    #[test]
    fn a_slot_person_already_present_draws_and_does_not_come() {
        let none = SpawnTable {
            avg_ships: 0,
            ..peopled()
        };
        let mut draw = PersonDraw::new(PersonWorld::NONE);
        draw.here.insert("Person 600".to_owned());
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            initial(&none, &mut draw, None, Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [100]);
    }

    #[test]
    fn a_person_arriving_jumps_in_from_hyperspace() {
        let mut draw = PersonDraw::new(PersonWorld::NONE);
        let mut chance = Draws::of(&[0, 0, 382, 90]);
        let ships = arrivals(&peopled(), 0, &mut draw, Stellars::NONE, &mut chance);
        assert_eq!(chance.asked, [500, 7, 1022, 360], "no aggression drawn");
        assert_eq!(persons_of(&ships), [Some(510)]);
        assert_eq!(ships[0].entry, Entry::JumpingIn);
        assert_eq!(ships[0].aggression, 4);
        assert!(near(
            ships[0].state.position,
            Vec2::new(jump_in_distance(), 0.0)
        ));
        assert!(draw.here.contains("Person 510"), "now here");
        let mut chance = Draws::of(&[0, 0, 381]);
        assert_eq!(
            arrivals(&peopled(), 0, &mut nobody(), Stellars::NONE, &mut chance),
            []
        );
        assert_eq!(chance.asked, [500, 7, 1022], "nothing arrives");
    }

    fn by(key: RuleKey) -> NovaPersons {
        NovaPersons::from_rulebook(&Rulebook::default().with_override(key, RuleSource::Bible))
    }

    #[test]
    fn by_the_bibles_odds_a_pass_brings_a_person_5_times_in_100_or_goes_on() {
        let rules = by(RuleKey::PersonOdds);
        let world = PersonWorld {
            rules: &rules,
            ..PersonWorld::NONE
        };
        let (ships, asked) = set_up(&peopled(), world, &[4, 0, 0, 0, 99]);
        assert_eq!(asked, [100, 1500, 1500, 360, 100]);
        assert_eq!(persons_of(&ships), [Some(510)]);
        let (ships, asked) = set_up(&one_pass(), world, &[4, 6, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            asked,
            [100, 7, 100, 100, 1500, 1500, 360, 3],
            "on to a düde ship"
        );
        assert_eq!(persons_of(&ships), [None]);
    }

    #[test]
    fn by_the_bibles_slots_a_slot_brings_its_person_without_a_draw() {
        let rules = by(RuleKey::SystemPersons);
        let world = PersonWorld {
            rules: &rules,
            ..PersonWorld::NONE
        };
        let table = SpawnTable {
            avg_ships: 0,
            person_slots: vec![(PersonId(600), 1)],
            ..peopled()
        };
        let (ships, asked) = set_up(&table, world, &[0, 0, 0]);
        assert_eq!(asked, [1500, 1500, 360]);
        assert_eq!(persons_of(&ships), [Some(600)]);
    }

    /// The player's escorts that are persons `ids`.
    fn fleet_of(ids: &[i16]) -> Vec<crate::pilot::Escort> {
        ids.iter()
            .map(|&id| crate::pilot::Escort {
                ship: ShipId(201),
                reserves: kind(3, FAST).stats.full(),
                order: None,
                carried: false,
                wage: None,
                person: Some(PersonId(id)),
            })
            .collect()
    }

    #[test]
    fn a_person_in_the_players_fleet_is_never_rolled_nor_slotted() {
        let mut table = peopled();
        table
            .persons
            .insert(PersonId(511), spawn_person(511, true, 2));
        let fleet = fleet_of(&[510]);
        let world = PersonWorld {
            fleet: &fleet,
            ..PersonWorld::NONE
        };
        assert_eq!(
            PersonDraw::new(world).eligible(&table, false),
            [PersonId(511)]
        );
        let (ships, asked) = set_up(&table, world, &[0, 382, 99]);
        assert_eq!(asked, [7, 1022, 100], "landing on it is empty");
        assert_eq!(ships, []);
        let (ships, _) = set_up(&table, world, &[0, 383, 0, 0, 0, 99]);
        assert_eq!(persons_of(&ships), [Some(511)], "another comes");
        let slotted = fleet_of(&[600]);
        let world = PersonWorld {
            fleet: &slotted,
            ..PersonWorld::NONE
        };
        let none = SpawnTable {
            avg_ships: 0,
            ..peopled()
        };
        assert_eq!(set_up(&none, world, &[0]), (vec![], vec![]), "no draw");
    }

    #[test]
    fn a_person_of_the_name_of_one_in_the_players_fleet_does_not_come() {
        let mut table = peopled();
        let mut namesake = spawn_person(512, true, 2);
        namesake.record.name = "Person 600".to_owned();
        table.persons.insert(PersonId(512), namesake);
        let fleet = fleet_of(&[600]);
        let world = PersonWorld {
            fleet: &fleet,
            ..PersonWorld::NONE
        };
        assert_eq!(
            PersonDraw::new(world).eligible(&table, false),
            [PersonId(510)]
        );
        assert_eq!(
            PersonDraw::new(PersonWorld::NONE).eligible(&table, false),
            [PersonId(510), PersonId(512)],
            "with no fleet, as before"
        );
        let slot = SpawnTable {
            avg_ships: 0,
            person_slots: vec![(PersonId(512), 100)],
            ..table
        };
        let (ships, asked) = set_up(&slot, world, &[0]);
        assert_eq!((persons_of(&ships), asked), (vec![], vec![100]));
    }

    // Mission ships.

    /// Düde 150, a mission's and not the system's: `AIType` 4, govt 131,
    /// flying ship 202 or 203 half and half.
    fn with_mission_dude() -> SpawnTable {
        let mut table = table();
        table.dude_records.insert(
            DudeId(150),
            SpawnDude {
                ai_type: 4,
                govt: Some(GovtId(131)),
                ships: vec![(ShipId(202), 50), (ShipId(203), 50)],
                booty: 0x0002,
                info_types: 0x0001,
            },
        );
        table
    }

    /// Nav slot 0 empty, slot 1 at (100, 200), slot 2 at (-300, 0).
    const NAV: [Option<Vec2>; 3] = [
        None,
        Some(Vec2::new(100.0, 200.0)),
        Some(Vec2::new(-300.0, 0.0)),
    ];

    /// `count` ships of düde 150 for slot 3, of `ShipStart` `start` and
    /// `ShipBehav` 0.
    fn mission_spawn(count: i16, start: i16) -> MissionSpawn<'static> {
        MissionSpawn {
            slot: 3,
            dude: DudeId(150),
            count,
            goal: ShipGoal::Destroy,
            start: ShipStart::from_code(start),
            behav: ShipBehav::AttackPlayer,
            nav: &NAV,
        }
    }

    fn mission_gate() -> Gate<'static> {
        PersonWorld::NONE.gate()
    }

    #[test]
    fn a_mission_ship_draws_its_aggression_its_place_then_its_ship_from_its_dude() {
        let mut chance = Draws::of(&[0, 0, 1499, 50]);
        let ships = mission_ships(
            &with_mission_dude(),
            &mission_spawn(1, 0),
            mission_gate(),
            &mut chance,
        );
        assert_eq!(chance.asked, [3, 1500, 1500, 100]);
        assert_eq!(
            ships,
            [NewShip {
                ship: ShipId(203),
                govt: Some(GovtId(131)),
                ai_type: AiType::Interceptor,
                lead: None,
                state: ShipState {
                    position: Vec2::new(-750.0, 749.0),
                    velocity: Vec2::ZERO,
                    heading: 0.0,
                },
                entry: Entry::Placed,
                aggression: 2,
                booty: 0x0002,
                info_types: 0x0001,
                person: None,
                mission: Some(MissionMark {
                    order: None,
                    slot: 3,
                    behav: ShipBehav::AttackPlayer,
                    formation: None,
                }),
                derelict: false,
                cloaked: false,
                aux: None,
                quote: None,
                cargo: None,
            }]
        );
    }

    #[test]
    fn as_many_mission_ships_come_as_asked_and_none_for_none_or_below() {
        let table = with_mission_dude();
        for (count, ships) in [(3, 3), (1, 1), (0, 0), (-1, 0)] {
            let spawned = mission_ships(
                &table,
                &mission_spawn(count, 0),
                mission_gate(),
                &mut crate::NeverFires,
            );
            assert_eq!(spawned.len(), ships, "{count}");
            assert!(
                spawned
                    .iter()
                    .all(|ship| ship.mission.map(|mark| mark.slot) == Some(3))
            );
        }
    }

    #[test]
    fn a_mission_ships_ai_is_its_dudes_above_0_else_its_ships_inherent_ai() {
        let mut table = with_mission_dude();
        table
            .dude_records
            .get_mut(&DudeId(150))
            .expect("150")
            .ai_type = 0;
        let mut chance = Draws::of(&[0, 0, 0, 0]);
        let ships = mission_ships(&table, &mission_spawn(1, 0), mission_gate(), &mut chance);
        assert_eq!(
            (ships[0].ship, ships[0].ai_type),
            (ShipId(202), AiType::Interceptor),
            "ship 202's InherentAI 4"
        );
    }

    #[test]
    fn a_mission_dude_with_no_record_or_no_ship_brings_none_after_the_places_draws() {
        let table = with_mission_dude();
        let unknown = MissionSpawn {
            dude: DudeId(151),
            ..mission_spawn(1, 0)
        };
        let mut chance = Draws::of(&[]);
        assert_eq!(
            mission_ships(&table, &unknown, mission_gate(), &mut chance),
            []
        );
        assert_eq!(chance.asked, [3, 1500, 1500]);
        let mut shipless = with_mission_dude();
        shipless
            .dude_records
            .get_mut(&DudeId(150))
            .expect("150")
            .ships = vec![(ShipId(202), 0)];
        let mut chance = Draws::of(&[]);
        assert_eq!(
            mission_ships(&shipless, &mission_spawn(1, 0), mission_gate(), &mut chance),
            []
        );
        assert_eq!(chance.asked, [3, 1500, 1500]);
    }

    #[test]
    fn with_none_of_its_ships_appearing_a_mission_dude_draws_among_them_all() {
        let gated = appearing(appearing(with_mission_dude(), 202, "b7"), 203, "b7");
        let refusing = refusing_7();
        let mut chance = Draws::of(&[0, 0, 0, 99]);
        let ships = mission_ships(
            &gated,
            &mission_spawn(1, 0),
            refusing.world.gate(),
            &mut chance,
        );
        assert_eq!(chance.asked, [3, 1500, 1500, 100]);
        assert_eq!(ships[0].ship, ShipId(203));
        let half = appearing(with_mission_dude(), 203, "b7");
        let mut chance = Draws::of(&[0, 0, 0, 49]);
        let ships = mission_ships(
            &half,
            &mission_spawn(1, 0),
            refusing.world.gate(),
            &mut chance,
        );
        assert_eq!(chance.asked, [3, 1500, 1500, 50]);
        assert_eq!(ships[0].ship, ShipId(202), "only 202 appears");
    }

    #[test]
    fn a_ship_start_of_minus_1_to_minus_16_puts_the_ships_at_that_nav_slots_stellar() {
        let table = with_mission_dude();
        let at = |start| {
            mission_ships(
                &table,
                &mission_spawn(2, start),
                mission_gate(),
                &mut crate::NeverFires,
            )
            .iter()
            .map(|ship| ship.state.position)
            .collect::<Vec<_>>()
        };
        assert_eq!(at(-2), [Vec2::new(100.0, 200.0); 2], "slot 1");
        assert_eq!(at(-3), [Vec2::new(-300.0, 0.0); 2], "slot 2");
        let random = Vec2::new(749.0, 749.0);
        assert_eq!(at(-1), [random; 2], "slot 0 is empty: placed at random");
        assert_eq!(at(-16), [random; 2], "no slot 15");
        assert_eq!(at(0), [random; 2], "0 places at random");
        assert_eq!(at(-17), [random; 2], "out of range");
    }

    #[test]
    fn only_a_ship_start_of_2_spawns_cloaked() {
        let table = with_mission_dude();
        for (start, cloaked) in [(2, true), (0, false), (1, false), (-1, false)] {
            let ships = mission_ships(
                &table,
                &mission_spawn(1, start),
                mission_gate(),
                &mut crate::NeverFires,
            );
            assert_eq!(ships[0].cloaked, cloaked, "{start}");
        }
    }

    /// [`mission_spawn`] of `ShipGoal` `goal`.
    fn goal_spawn(count: i16, start: i16, goal: ShipGoal) -> MissionSpawn<'static> {
        MissionSpawn {
            goal,
            ..mission_spawn(count, start)
        }
    }

    #[test]
    fn a_rescue_ship_set_up_draws_its_heading_after_its_ship_and_starts_still_and_derelict() {
        let mut chance = Draws::of(&[0, 0, 0, 0, 90, 0, 0, 0, 99, 270]);
        let ships = mission_ships(
            &with_mission_dude(),
            &goal_spawn(2, -2, ShipGoal::Rescue),
            mission_gate(),
            &mut chance,
        );
        assert_eq!(
            chance.asked,
            [3, 1500, 1500, 100, 360, 3, 1500, 1500, 100, 360]
        );
        assert_eq!(
            ships
                .iter()
                .map(|ship| (ship.ship, ship.state, ship.derelict))
                .collect::<Vec<_>>(),
            [
                (
                    ShipId(202),
                    ShipState {
                        position: Vec2::new(100.0, 200.0),
                        velocity: Vec2::ZERO,
                        heading: 90.0,
                    },
                    true
                ),
                (
                    ShipId(203),
                    ShipState {
                        position: Vec2::new(100.0, 200.0),
                        velocity: Vec2::ZERO,
                        heading: 270.0,
                    },
                    true
                ),
            ],
            "at nav slot 1 after the heading"
        );
    }

    #[test]
    fn an_escorted_ship_set_up_is_moved_near_the_centre_by_two_draws_after_its_ship() {
        // Ship 1: aggression, place, ship, then x and y; ship 2 the same.
        let mut chance = Draws::of(&[0, 0, 0, 0, 0, 511, 0, 0, 0, 99, 300, 256]);
        let ships = mission_ships(
            &with_mission_dude(),
            &goal_spawn(2, 0, ShipGoal::Escort),
            mission_gate(),
            &mut chance,
        );
        assert_eq!(
            chance.asked,
            [3, 1500, 1500, 100, 512, 512, 3, 1500, 1500, 100, 512, 512]
        );
        assert_eq!(
            ships
                .iter()
                .map(|ship| (ship.ship, ship.state))
                .collect::<Vec<_>>(),
            [
                (
                    ShipId(202),
                    ShipState {
                        position: Vec2::new(-256.0, 255.0),
                        ..ShipState::default()
                    }
                ),
                (
                    ShipId(203),
                    ShipState {
                        position: Vec2::new(44.0, 0.0),
                        ..ShipState::default()
                    }
                ),
            ]
        );
        assert!(ships.iter().all(|ship| !ship.derelict));
    }

    #[test]
    fn an_escorted_ship_set_up_at_a_nav_slot_ends_at_its_stellar() {
        let mut chance = Draws::of(&[0, 0, 0, 0, 7, 9]);
        let ships = mission_ships(
            &with_mission_dude(),
            &goal_spawn(1, -2, ShipGoal::Escort),
            mission_gate(),
            &mut chance,
        );
        assert_eq!(chance.asked, [3, 1500, 1500, 100, 512, 512]);
        assert_eq!(ships[0].state.position, Vec2::new(100.0, 200.0));
    }

    #[test]
    fn an_escorted_ship_coming_later_or_swapped_in_draws_no_setup_offset() {
        let table = with_mission_dude();
        let escorted = goal_spawn(1, 1, ShipGoal::Escort);
        let mut chance = Draws::of(&[0, 0, 0, 0, 0, 0, 0]);
        arriving_mission_ships(&table, &escorted, None, mission_gate(), &mut chance);
        assert_eq!(chance.asked, [360, 3, 1500, 1500, 100, 512, 512]);
        let mut chance = Draws::of(&[0, 0, 0, 0]);
        let at = ShipState {
            position: Vec2::new(900.0, 900.0),
            ..ShipState::default()
        };
        let swapped =
            swapped_mission_ship(&table, &escorted, at, None, mission_gate(), &mut chance)
                .expect("a ship");
        assert_eq!(chance.asked, [3, 1500, 1500, 100]);
        assert_eq!(swapped.state.position, at.position);
    }

    #[test]
    fn a_mission_ship_of_another_goal_set_up_draws_no_heading_and_is_not_derelict() {
        for goal in [
            ShipGoal::None,
            ShipGoal::Destroy,
            ShipGoal::Disable,
            ShipGoal::Board,
            ShipGoal::Observe,
            ShipGoal::ChaseOff,
            ShipGoal::Other,
        ] {
            let mut chance = Draws::of(&[0, 0, 0, 0]);
            let ships = mission_ships(
                &with_mission_dude(),
                &goal_spawn(1, 0, goal),
                mission_gate(),
                &mut chance,
            );
            assert_eq!(chance.asked, [3, 1500, 1500, 100], "{goal:?}");
            assert_eq!(ships[0].state.heading, 0.0, "{goal:?}");
            assert!(!ships[0].derelict, "{goal:?}");
        }
    }

    #[test]
    fn a_rescue_ship_coming_later_or_swapped_in_gets_no_setup_placement() {
        let table = with_mission_dude();
        let rescue = goal_spawn(1, 1, ShipGoal::Rescue);
        let mut chance = Draws::of(&[0, 0, 0, 0, 0, 0, 0]);
        let ships = arriving_mission_ships(&table, &rescue, None, mission_gate(), &mut chance);
        assert_eq!(chance.asked, [360, 3, 1500, 1500, 100, 512, 512]);
        assert!(!ships[0].derelict);
        let mut chance = Draws::of(&[0, 0, 0, 0]);
        let swapped = swapped_mission_ship(
            &table,
            &rescue,
            ShipState::default(),
            None,
            mission_gate(),
            &mut chance,
        )
        .expect("a ship");
        assert_eq!(chance.asked, [3, 1500, 1500, 100]);
        assert!(!swapped.derelict);
    }

    #[test]
    fn mission_ships_coming_later_glide_in_around_an_entry_point_drawn_first() {
        // Angle 0 (straight up), then each ship: aggression, place, ship,
        // and its spread from the entry point.
        let mut chance = Draws::of(&[0, 0, 0, 0, 0, 0, 511, 2, 0, 0, 50, 256, 256]);
        let ships = arriving_mission_ships(
            &with_mission_dude(),
            &mission_spawn(2, 1),
            None,
            mission_gate(),
            &mut chance,
        );
        assert_eq!(
            chance.asked,
            [
                360, 3, 1500, 1500, 100, 512, 512, 3, 1500, 1500, 100, 512, 512
            ]
        );
        let entry = Vec2::new(0.0, -jump_in_distance());
        assert_eq!(ships.len(), 2);
        assert!(near(
            ships[0].state.position,
            entry + Vec2::new(-256.0, 255.0)
        ));
        assert!(near(ships[1].state.position, entry));
        assert_eq!((ships[0].ship, ships[1].ship), (ShipId(202), ShipId(203)));
        for ship in &ships {
            assert_eq!(ship.entry, Entry::JumpingIn);
            let inward = Vec2::ZERO - ship.state.position;
            assert!(near(
                ship.state.velocity,
                inward * (JUMP_IN_SPEED / inward.length())
            ));
            assert_eq!(ship.state.heading, heading_of(inward));
            assert!(!ship.cloaked);
        }
    }

    #[test]
    fn mission_ships_coming_later_enter_from_the_way_the_player_came_drawing_no_angle() {
        let mut chance = Draws::of(&[]);
        let ships = arriving_mission_ships(
            &with_mission_dude(),
            &mission_spawn(1, 2),
            Some(Vec2::new(30.0, 40.0)),
            mission_gate(),
            &mut chance,
        );
        assert_eq!(chance.asked, [3, 1500, 1500, 100, 512, 512]);
        let entry = Vec2::new(0.6, 0.8) * jump_in_distance();
        assert!(near(
            ships[0].state.position,
            entry + Vec2::new(255.0, 255.0)
        ));
        assert!(!ships[0].cloaked, "a ShipStart of 2 comes in uncloaked");
        let mut chance = Draws::of(&[90]);
        let _ = arriving_mission_ships(
            &with_mission_dude(),
            &mission_spawn(1, 1),
            Some(Vec2::ZERO),
            mission_gate(),
            &mut chance,
        );
        assert_eq!(chance.asked[0], 360, "no way back is drawn");
    }

    #[test]
    fn a_mission_ship_with_no_fuel_comes_in_later_all_the_same() {
        let mut chance = Draws::of(&[0, 0, 0, 0, 99]);
        let ships = arriving_mission_ships(
            &with_mission_dude(),
            &mission_spawn(1, 1),
            None,
            mission_gate(),
            &mut chance,
        );
        assert_eq!(ships[0].ship, ShipId(203), "no fuel tank");
    }
    // Auxiliary ships.

    /// `count` auxiliary ships of düde 150 for slot 4.
    fn aux_spawn(count: i16) -> AuxSpawn {
        AuxSpawn {
            slot: 4,
            dude: DudeId(150),
            count,
        }
    }

    #[test]
    fn aux_ships_glide_in_around_one_entry_point_drawn_first_unmarked_but_for_their_slot() {
        // Angle 90 (right), then each ship: aggression, place, ship, and
        // its spread from the entry point.
        let mut chance = Draws::of(&[90, 0, 0, 0, 0, 0, 511, 2, 0, 0, 50, 256, 256]);
        let ships = arriving_aux_ships(
            &with_mission_dude(),
            &aux_spawn(2),
            mission_gate(),
            &mut chance,
        );
        assert_eq!(
            chance.asked,
            [
                360, 3, 1500, 1500, 100, 512, 512, 3, 1500, 1500, 100, 512, 512
            ]
        );
        let entry = facing(90.0) * jump_in_distance();
        assert_eq!(ships.len(), 2);
        assert!(near(
            ships[0].state.position,
            entry + Vec2::new(-256.0, 255.0)
        ));
        assert!(near(ships[1].state.position, entry));
        assert_eq!((ships[0].ship, ships[1].ship), (ShipId(202), ShipId(203)));
        for ship in &ships {
            assert_eq!(ship.entry, Entry::JumpingIn);
            let inward = Vec2::ZERO - ship.state.position;
            assert!(near(
                ship.state.velocity,
                inward * (JUMP_IN_SPEED / inward.length())
            ));
            assert_eq!(ship.state.heading, heading_of(inward));
            assert_eq!(ship.mission, None, "ordinary traffic");
            assert_eq!(ship.aux, Some(4));
            assert_eq!(
                (ship.govt, ship.ai_type, ship.booty, ship.info_types),
                (Some(GovtId(131)), AiType::Interceptor, 0x0002, 0x0001),
                "the düde's"
            );
            assert!(!ship.derelict);
        }
        assert_eq!((ships[0].aggression, ships[1].aggression), (2, 0));
    }

    #[test]
    fn as_many_aux_ships_come_as_asked_and_none_for_none_or_below() {
        let table = with_mission_dude();
        for (count, ships) in [(3, 3), (1, 1), (0, 0), (-1, 0)] {
            let spawned = arriving_aux_ships(
                &table,
                &aux_spawn(count),
                mission_gate(),
                &mut crate::NeverFires,
            );
            assert_eq!(spawned.len(), ships, "{count}");
        }
        let mut chance = Draws::of(&[]);
        let _ = arriving_aux_ships(&table, &aux_spawn(0), mission_gate(), &mut chance);
        assert_eq!(chance.asked, [360], "the angle is drawn all the same");
    }

    #[test]
    fn an_aux_ship_of_no_type_that_may_appear_fails_drawing_no_spread() {
        let gated = appearing(appearing(with_mission_dude(), 202, "b7"), 203, "b7");
        let mut chance = Draws::of(&[]);
        assert_eq!(
            arriving_aux_ships(&gated, &aux_spawn(2), mission_gate(), &mut chance),
            []
        );
        assert_eq!(
            chance.asked,
            [360, 3, 1500, 1500, 3, 1500, 1500],
            "no fallback to every type, and no spread"
        );
        // One type refused: the other is drawn alone.
        let gated = appearing(with_mission_dude(), 202, "b7");
        let mut chance = Draws::of(&[]);
        let ships = arriving_aux_ships(&gated, &aux_spawn(1), mission_gate(), &mut chance);
        assert_eq!(chance.asked, [360, 3, 1500, 1500, 50, 512, 512]);
        assert_eq!(ships[0].ship, ShipId(203), "no fuel tank, and it comes");
    }

    #[test]
    fn an_aux_dude_with_no_record_brings_none_after_the_places_draws() {
        let unknown = AuxSpawn {
            dude: DudeId(151),
            ..aux_spawn(1)
        };
        let mut chance = Draws::of(&[]);
        assert_eq!(
            arriving_aux_ships(&with_mission_dude(), &unknown, mission_gate(), &mut chance),
            []
        );
        assert_eq!(chance.asked, [360, 3, 1500, 1500]);
    }

    #[test]
    fn an_aux_ships_ai_is_its_dudes_above_0_else_its_ships_inherent_ai() {
        let mut table = with_mission_dude();
        table
            .dude_records
            .get_mut(&DudeId(150))
            .expect("150")
            .ai_type = 0;
        let ships = arriving_aux_ships(
            &table,
            &aux_spawn(1),
            mission_gate(),
            &mut Draws::of(&[0, 0, 0, 0, 0]),
        );
        assert_eq!(
            (ships[0].ship, ships[0].ai_type),
            (ShipId(202), AiType::Interceptor),
            "ship 202's InherentAI 4"
        );
    }

    /// A person's place: at (40, -60), moving (1, 2), heading 90.
    const PERSON_AT: ShipState = ShipState {
        position: Vec2::new(40.0, -60.0),
        velocity: Vec2::new(1.0, 2.0),
        heading: 90.0,
    };

    #[test]
    fn a_swapped_mission_ship_takes_the_persons_place_its_ship_drawn_from_its_dude() {
        let mut chance = Draws::of(&[0, 0, 1499, 50]);
        let ship = swapped_mission_ship(
            &with_mission_dude(),
            &mission_spawn(1, 0),
            PERSON_AT,
            None,
            mission_gate(),
            &mut chance,
        );
        assert_eq!(chance.asked, [3, 1500, 1500, 100]);
        assert_eq!(
            ship,
            Some(NewShip {
                ship: ShipId(203),
                govt: Some(GovtId(131)),
                ai_type: AiType::Interceptor,
                lead: None,
                state: PERSON_AT,
                entry: Entry::Placed,
                aggression: 2,
                booty: 0x0002,
                info_types: 0x0001,
                person: None,
                mission: Some(MissionMark {
                    order: None,
                    slot: 3,
                    behav: ShipBehav::AttackPlayer,
                    formation: None,
                }),
                derelict: false,
                cloaked: false,
                aux: None,
                quote: None,
                cargo: None,
            })
        );
    }

    #[test]
    fn a_swapped_mission_ship_of_a_preset_type_flies_it_drawing_no_pick() {
        let gated = appearing(with_mission_dude(), 202, "b7");
        let refusing = refusing_7();
        let mut chance = Draws::of(&[]);
        let ship = swapped_mission_ship(
            &gated,
            &MissionSpawn {
                behav: ShipBehav::EscortPlayer,
                start: ShipStart::from_code(2),
                ..mission_spawn(1, 0)
            },
            PERSON_AT,
            Some(ShipId(202)),
            refusing.world.gate(),
            &mut chance,
        )
        .expect("a ship");
        assert_eq!(chance.asked, [3, 1500, 1500], "no pick, AppearOn unasked");
        assert_eq!(ship.ship, ShipId(202));
        assert_eq!((ship.state, ship.entry), (PERSON_AT, Entry::Placed));
        assert_eq!(
            ship.mission,
            Some(MissionMark {
                order: None,
                slot: 3,
                behav: ShipBehav::EscortPlayer,
                formation: None,
            }),
        );
        assert!(!ship.cloaked, "never cloaked");
        let mut inherent = with_mission_dude();
        inherent
            .dude_records
            .get_mut(&DudeId(150))
            .expect("150")
            .ai_type = 0;
        let ship = swapped_mission_ship(
            &inherent,
            &mission_spawn(1, 0),
            PERSON_AT,
            Some(ShipId(202)),
            mission_gate(),
            &mut crate::NeverFires,
        );
        assert_eq!(
            ship.map(|ship| ship.ai_type),
            Some(AiType::Interceptor),
            "ship 202's InherentAI 4"
        );
    }

    #[test]
    fn a_swap_whose_dude_has_no_record_brings_no_ship() {
        let unknown = MissionSpawn {
            dude: DudeId(151),
            ..mission_spawn(1, 0)
        };
        let mut chance = Draws::of(&[]);
        let ship = swapped_mission_ship(
            &with_mission_dude(),
            &unknown,
            PERSON_AT,
            Some(ShipId(202)),
            mission_gate(),
            &mut chance,
        );
        assert_eq!((ship, chance.asked), (None, vec![3, 1500, 1500]));
    }

    /// A planet at the centre, then a hypergate at (300, -400) heading
    /// ships out on `exit_angle`.
    fn gated(exit_angle: i16) -> [LandingSite; 2] {
        [
            crate::testkit::planet(128, 0.0, 0.0),
            LandingSite {
                flags2: crate::gate::HYPERGATE,
                exit_angle,
                ..crate::testkit::planet(129, 300.0, -400.0)
            },
        ]
    }

    /// No government with a `Flags2` switch, and none at war.
    static PLAIN: Governments = Governments::empty();

    fn among(sites: &[LandingSite]) -> Stellars<'_> {
        Stellars {
            sites,
            govts: &PLAIN,
        }
    }

    /// At the hypergate of [`gated`], heading `heading`, at rest.
    fn at_gate(heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(300.0, -400.0),
            velocity: Vec2::ZERO,
            heading,
        }
    }

    #[test]
    fn a_dude_ship_arriving_may_come_out_of_a_hypergate_at_rest_on_its_angle() {
        // On 0, not a person, not a fleet, düde 128's ship 201, the
        // jump-in angle drawn, then the gate (1 of the planet and the
        // gate), and its aggression last.
        let sites = gated(90);
        let mut chance = Draws::of(&[0, 6, 6, 0, 99, 90, 1, 0]);
        let ships = arrivals(&table(), 0, &mut nobody(), among(&sites), &mut chance);
        assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360, 2, 3]);
        assert_eq!(ships.len(), 1);
        assert_eq!(ships[0].ship, ShipId(201));
        assert_eq!(ships[0].state, at_gate(90.0));
        assert_eq!(
            ships[0].entry,
            Entry::Emerging {
                ticks_left: EMERGE_TICKS,
                gate: StellarId(129),
            }
        );
        assert_eq!(ships[0].aggression, 2);
        // A gate with no angle rolls one after the pick.
        let sites = gated(-1);
        let mut chance = Draws::of(&[0, 6, 6, 0, 99, 90, 1, 45]);
        let ships = arrivals(&table(), 0, &mut nobody(), among(&sites), &mut chance);
        assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360, 2, 360, 3]);
        assert_eq!(ships[0].state, at_gate(45.0));
        // The planet drawn: it jumps in as before.
        let mut chance = Draws::of(&[0, 6, 6, 0, 99, 90, 0]);
        let ships = arrivals(&table(), 0, &mut nobody(), among(&gated(90)), &mut chance);
        assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360, 2, 3]);
        assert_eq!(ships[0].entry, Entry::JumpingIn);
        assert!(near(
            ships[0].state.position,
            facing(90.0) * jump_in_distance()
        ));
    }

    #[test]
    fn a_ship_of_a_government_preferring_wormholes_comes_out_of_one() {
        let govts = Governments::new([crate::catalog::GovtRecord {
            flags2: crate::govt::PREFERS_WORMHOLES,
            ..crate::testkit::govt(130)
        }]);
        let sites = [
            crate::testkit::planet(128, 0.0, 0.0),
            LandingSite {
                flags2: crate::gate::WORMHOLE,
                exit_angle: 180,
                ..crate::testkit::planet(129, 300.0, -400.0)
            },
        ];
        let stellars = Stellars {
            sites: &sites,
            govts: &govts,
        };
        let mut chance = Draws::of(&[0, 6, 6, 0, 99, 90, 0, 0]);
        let ships = arrivals(&table(), 0, &mut nobody(), stellars, &mut chance);
        assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360, 1, 3]);
        assert_eq!(ships[0].state, at_gate(180.0));
        assert_eq!(
            ships[0].entry,
            Entry::Emerging {
                ticks_left: EMERGE_TICKS,
                gate: StellarId(129),
            }
        );
        // Any other ship never takes the wormhole, and draws no pick.
        let mut chance = Draws::of(&[0, 6, 6, 0, 99, 90]);
        let ships = arrivals(&table(), 0, &mut nobody(), among(&sites), &mut chance);
        assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360, 3]);
        assert_eq!(ships[0].entry, Entry::JumpingIn);
    }

    #[test]
    fn a_person_arriving_may_come_out_of_a_hypergate() {
        let mut draw = PersonDraw::new(PersonWorld::NONE);
        let mut chance = Draws::of(&[0, 0, 382, 90, 1]);
        let sites = gated(270);
        let ships = arrivals(&peopled(), 0, &mut draw, among(&sites), &mut chance);
        assert_eq!(chance.asked, [500, 7, 1022, 360, 2], "no aggression drawn");
        assert_eq!(persons_of(&ships), [Some(510)]);
        assert_eq!(ships[0].state, at_gate(270.0));
        assert_eq!(
            ships[0].entry,
            Entry::Emerging {
                ticks_left: EMERGE_TICKS,
                gate: StellarId(129),
            }
        );
    }

    #[test]
    fn a_fleet_whose_lead_comes_out_of_a_gate_brings_its_escorts_out_of_it_later() {
        // The gate first, no jump-in angle; the lead's aggression; then
        // each escort's offsets, its extra wait and its aggression.
        let mut chance = Draws::of(&[1, 0, 0, 0, 7, 0, 299, 299, 14, 0]);
        let mut out = Vec::new();
        let sites = gated(90);
        fleet(
            &table(),
            FleetId(141),
            Gate::FRESH,
            among(&sites),
            FleetCall::default(),
            &mut chance,
            &mut out,
        );
        assert_eq!(chance.asked, [2, 3, 300, 300, 15, 3, 300, 300, 15, 3]);
        assert_eq!(out.len(), 3);
        assert_eq!(
            out.iter().map(|ship| ship.state).collect::<Vec<_>>(),
            [at_gate(90.0); 3],
            "every one at the gate's centre, on the lead's heading"
        );
        assert_eq!(
            out.iter().map(|ship| ship.entry).collect::<Vec<_>>(),
            [
                Entry::Emerging {
                    ticks_left: 60,
                    gate: StellarId(129)
                },
                Entry::Emerging {
                    ticks_left: 72,
                    gate: StellarId(129)
                },
                Entry::Emerging {
                    ticks_left: 79,
                    gate: StellarId(129)
                },
            ]
        );
        assert_eq!(
            out.iter().map(|ship| ship.lead).collect::<Vec<_>>(),
            [None, Some(0), Some(0)]
        );
        // A gate rolling its heading gives the escorts the lead's.
        let mut chance = Draws::of(&[1, 200]);
        let mut out = Vec::new();
        let sites = gated(-1);
        fleet(
            &table(),
            FleetId(141),
            Gate::FRESH,
            among(&sites),
            FleetCall::default(),
            &mut chance,
            &mut out,
        );
        assert_eq!(&chance.asked[..2], [2, 360]);
        assert!(out.iter().all(|ship| ship.state == at_gate(200.0)));
    }

    #[test]
    fn a_linksyst_fleet_at_setup_may_come_out_of_a_gate() {
        // Not a person, a fleet, slot 13 (fleet 141), the gate.
        let mut chance = Draws::of(&[6, 0, 13, 1, 2, 0, 0, 5, 2, 299, 299, 9, 2]);
        let sites = gated(90);
        let ships = initial(&one_pass(), &mut nobody(), None, among(&sites), &mut chance);
        assert_eq!(
            chance.asked,
            [7, 7, 256, 2, 3, 300, 300, 15, 3, 300, 300, 15, 3]
        );
        assert_eq!(
            ships.iter().map(|ship| ship.entry).collect::<Vec<_>>(),
            [
                Entry::Emerging {
                    ticks_left: 60,
                    gate: StellarId(129)
                },
                Entry::Emerging {
                    ticks_left: 70,
                    gate: StellarId(129)
                },
                Entry::Emerging {
                    ticks_left: 74,
                    gate: StellarId(129)
                },
            ]
        );
        assert!(ships.iter().all(|ship| ship.state == at_gate(90.0)));
    }

    /// Ships brought in among some stellars, drawn from a source.
    type Scenario = fn(Stellars, &mut Draws) -> Vec<NewShip>;

    /// The scenarios of these tests, each with the draws it is drawn
    /// from: a düde ship (govt 130) arriving, a person arriving, a setup
    /// pass bringing `LinkSyst` fleet 141 (govt 131), a named fleet 140
    /// arriving, fleet 141 itself, and a setup pass of last outcomes.
    fn scenarios() -> [(Scenario, &'static [u32]); 6] {
        [
            (
                |stellars, chance| arrivals(&table(), 0, &mut nobody(), stellars, chance),
                &[0, 6, 6, 0, 99, 90],
            ),
            (
                |stellars, chance| {
                    let mut draw = PersonDraw::new(PersonWorld::NONE);
                    arrivals(&peopled(), 0, &mut draw, stellars, chance)
                },
                &[0, 0, 382, 90],
            ),
            (
                |stellars, chance| initial(&one_pass(), &mut nobody(), None, stellars, chance),
                &[6, 0, 13, 0, 2, 0, 0, 2, 299, 299, 2],
            ),
            (
                |stellars, chance| arrivals(&table(), 0, &mut nobody(), stellars, chance),
                &[1, 29, 0, 0, 0, 2],
            ),
            (
                |stellars, chance| {
                    let mut out = Vec::new();
                    fleet(
                        &table(),
                        FleetId(141),
                        Gate::FRESH,
                        stellars,
                        FleetCall::default(),
                        chance,
                        &mut out,
                    );
                    out
                },
                &[],
            ),
            (
                |stellars, chance| initial(&one_pass(), &mut nobody(), None, stellars, chance),
                &[],
            ),
        ]
    }

    /// `scenario` drawn from `draws` among `stellars`: the ships and the
    /// bounds asked.
    fn run(scenario: Scenario, stellars: Stellars, draws: &[u32]) -> (Vec<NewShip>, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let ships = scenario(stellars, &mut chance);
        (ships, chance.asked)
    }

    #[test]
    fn with_no_gate_to_come_out_of_arrivals_are_as_they_were() {
        let destroyed = {
            let [planet, gate] = gated(90);
            [
                planet,
                LandingSite {
                    destroyed: true,
                    ..gate
                },
            ]
        };
        let hostile = {
            let [planet, gate] = gated(90);
            [
                planet,
                LandingSite {
                    govt: Some(GovtId(129)),
                    ..gate
                },
            ]
        };
        let govts = Governments::new([
            crate::catalog::GovtRecord {
                classes: [1, -1, -1, -1],
                enemies: [2, -1, -1, -1],
                ..crate::testkit::govt(130)
            },
            crate::catalog::GovtRecord {
                classes: [2, -1, -1, -1],
                ..crate::testkit::govt(129)
            },
        ]);
        let planets = [
            crate::testkit::planet(128, 0.0, 0.0),
            crate::testkit::planet(130, 500.0, 0.0),
        ];
        for (index, (scenario, draws)) in scenarios().into_iter().enumerate() {
            let before = run(scenario, Stellars::NONE, draws);
            assert!(!before.0.is_empty(), "{index}: something comes");
            assert_eq!(run(scenario, among(&planets), draws), before, "{index}");
            assert_eq!(
                run(scenario, among(&destroyed), draws),
                before,
                "{index}: a destroyed gate"
            );
        }
        let at_war = Stellars {
            sites: &hostile,
            govts: &govts,
        };
        let (dude_ship, draws) = scenarios()[0];
        assert_eq!(
            run(dude_ship, at_war, draws),
            run(dude_ship, Stellars::NONE, draws),
            "an enemy's gate, for the düde ship of govt 130"
        );
    }

    #[test]
    fn a_ship_arriving_skips_a_destroyed_gate_and_jumps_in() {
        let [planet, gate] = gated(90);
        let sites = [
            planet,
            LandingSite {
                destroyed: true,
                ..gate
            },
        ];
        let mut chance = Draws::of(&[0, 6, 6, 0, 99, 90]);
        let ships = arrivals(&table(), 0, &mut nobody(), among(&sites), &mut chance);
        assert_eq!(chance.asked, [500, 7, 7, 100, 100, 360, 3]);
        assert_eq!(ships[0].entry, Entry::JumpingIn);
    }
}
