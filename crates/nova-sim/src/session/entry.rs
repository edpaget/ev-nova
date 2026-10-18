//! Entering a system: the one path that a jump's arrival, a hypergate or
//! wormhole exit, a set expression's move (`M` and `N`) and the developer
//! tools' relocation all take into a system ([`Session::enter_system`]).
//!
//! Every way in makes the same arrival writes: the system, the pilot's
//! stellar, the ship's placement, the stellars read, and so on. What each
//! one lets go of or keeps differs, and [`resets`] gives that as one
//! table, a row for each way in ([`Entry`]), read with the session's
//! rules and the facts of the moment ([`EntryFacts`]). The callers keep
//! only their own work: a jump's fuel and days, a gate's roll, and
//! choosing where the ship goes ([`Placement`]).
//!
//! Every draw a caller makes on its chance comes before it enters the
//! system; inside, only populating the system draws, after the ship is
//! placed, the system explored and the fighters out dealt with, and then,
//! by the other reading of
//! [`RuleKey::AsteroidEntryField`](crate::RuleKey::AsteroidEntryField), a
//! jump's arrival placing its asteroid field about the ship, so the frame
//! drawn after the arrival shows it.
//!
//! Before any write, the system entered is resolved through the galaxy
//! overlay, so the player is always in a visible system: the system
//! itself when it is visible, else the visible system at its place, else
//! the chain head, made visible ([`OverlaidGalaxy::arrival`]; see the
//! `story` module's "Galaxy overlay"). The stellars of a system resolved
//! to another are read again; the caller's placement is kept, a system at
//! the same place being at the same map position.
//!
//! **The stellars shown.** [`Session::set_player_system`] is the one owner
//! of which stellars the session shows, and every move of the player's
//! system goes through it: a way in, the galaxy overlay's move off a
//! hidden system, a pilot flown, and the take-off. In flight the new
//! system's stellars are shown at once. Docked, the stellars shown stay
//! those of the system whose stellar the ship is docked at, so the
//! spaceport keeps serving it, and the new system's wait for the
//! take-off: after a landed `M` or `N`, after the overlay moves a landed
//! player, after a relocation to a stellar of a system resolved to
//! another. A mover says only where the ship is docked ([`Dock`]).
//!
//! Which of a system's stellars are shown is the galaxy's: those it holds
//! by ownership or by the rescue ([`OverlaidGalaxy::shows`]), as
//! `_HandleStellarSystemVisibility` shows only the active stellars the
//! player's system owns, so a stellar another visible system owns is not
//! shown though the system lists it. The stellar the ship is docked at
//! is always among those shown. The session keeps every stellar the
//! system lists, and derives the stellars shown from them as the system
//! changes and at each flight rebuild point ([`Session::reshow`]); a
//! galaxy change between rebuild points reads the stellars shown again
//! but never changes which are shown.
//!
//! **The ships left behind.** An in-flight `M` or `N` into another
//! system takes along only the ships the player leads. By the engine
//! reading of [`RuleKey::MoveLeftBehind`](crate::RuleKey::MoveLeftBehind)
//! the rest stay in the ship table, out of the system, until the end of
//! the next traffic tick ([`Traffic::retarget`]): the original's display
//! pass frees them later in the frame (`_HandleShipDisplay`
//! @0x2ded6-0x2dee7), so the one arrivals pass before it still sees
//! their slots and persons. A move back before then brings them back;
//! none of their aux ships is given back. They are never drawn, saved or
//! counted toward `AvgShips`, and every setup and reset drops them. The
//! engine settles every script after the frame's traffic tick, so a move
//! the original makes after its arrivals pass (in `_AIDispatch`,
//! `_HandleShot` or the ship loop) holds them through one pass more than
//! the original would; none of it is drawn.
//!
//! The last step announces the arrival, and tells a view what to redraw
//! ([`scene_event`], see the `events` module): measured against the
//! system whose stellars are shown, which after a landed move is still
//! the one landed in. A landed move's own event waits, with its stellars,
//! for the take-off ([`Session::take_off`]).

use std::collections::BTreeSet;

use super::script_effects::ScriptEffectRules;
use super::{Session, SessionEvent, story};
use crate::catalog::{
    DudeId, DudeRecord, FleetRecord, LandingSite, PersonRecord, PilotCatalog, StellarId, SystemId,
    SystemTraffic, TrafficCatalog,
};
use crate::chance::Chance;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::message::SimMessage;
use crate::navigation::NavMode;
use crate::pilot::ExploredLevel;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::sound::SimSound;
#[cfg(doc)]
use crate::story::OverlaidGalaxy;
use crate::story::{Entrance, StoryEvent};
use crate::traffic::{Leave, Traffic};

/// How the player enters a system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Entry {
    /// A hyperspace jump's arrival, having made `hops` hops along the
    /// course, at its natural end, both warp-ups finished, or not, at its
    /// fallback end (see [`Session::arrive`]).
    Jump { hops: usize, natural_end: bool },
    /// Out of a hypergate or wormhole.
    Gate,
    /// `M` or `N`, landed or in flight.
    ScriptMove { landed: bool },
    /// An `N` out of range in flight, into the system the player is in:
    /// the in-flight move's tail with no move (`_EvalSetExp` @0x1593f,
    /// @0x1596e-0x159ad). It is never an arrival, whatever
    /// [`RuleKey::MoveArrival`](crate::RuleKey::MoveArrival) says, as it
    /// moves nowhere: its row is a move's within the system by the
    /// engine's reading, and it names the entrance and tells the view as
    /// an in-flight move does.
    ScriptStay,
    /// The developer tools' move, landed, or a walker's landing
    /// ([`Session::land_at`]), landed or in flight.
    Relocate,
    /// The escape pod's rescue (see the `rescue` module).
    Rescue,
}

impl Entry {
    /// The way in a [`StoryEvent::EnteredSystem`](crate::story::StoryEvent)
    /// names, a jump's hops left out: none for the escape pod's rescue,
    /// which calls no `_MissionHandlePlayerEnteredNewSystem` and so raises
    /// no enter-system event and runs no enter-system hook.
    fn entrance(self) -> Option<Entrance> {
        match self {
            Self::Jump { .. } => Some(Entrance::Jump),
            Self::Gate => Some(Entrance::Gate),
            Self::ScriptMove { landed } => Some(Entrance::ScriptMove { landed }),
            Self::ScriptStay => Some(Entrance::ScriptMove { landed: false }),
            Self::Relocate => Some(Entrance::Relocate),
            Self::Rescue => None,
        }
    }
}

/// The runtime conditions the table reads, evaluated by
/// [`Session::enter_system`] before any write.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct EntryFacts {
    /// Whether the system entered is another than the one left.
    pub(super) system_changed: bool,
    /// Whether a jump was under way, being prepared or in hyperspace.
    pub(super) jump_pending: bool,
}

/// Where the caller chose to put the ship; [`Session::enter_system`]
/// makes the only write.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Placement {
    /// Where the ship goes.
    pub(super) ship: ShipPlacement,
    /// The pilot's stellar, where the row sets it ([`Reset::Stellar`]),
    /// and the stellar docked at, where it docks ([`Reset::Dock`]).
    pub(super) stellar: Option<StellarId>,
    /// Whether the next take-off keeps the ship where it touched down:
    /// added to what it was, never taken away.
    pub(super) hold: bool,
}

/// Where the ship goes as it enters a system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ShipPlacement {
    /// Exactly here, moving and heading as this says.
    Arrive(ShipState),
    /// At rest at this position, its heading kept.
    AtRest(Vec2),
    /// Where it is, as it is.
    Keep,
}

/// The system entered, its stellars as the caller read them (once), and
/// where the ship goes.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Arrival {
    /// The system.
    pub(super) system: SystemId,
    /// Its stellars.
    pub(super) sites: Vec<LandingSite>,
    /// Where the ship goes.
    pub(super) placement: Placement,
}

/// Where the ship is docked as the player's system changes, for
/// [`Session::set_player_system`].
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Dock {
    /// As it was: in flight, or docked still at the stellar it was, whose
    /// system's stellars stay shown.
    AsItWas,
    /// Anew, at a stellar of this system, whose stellars these are.
    At(SystemId, Vec<LandingSite>),
}

/// One reset or write a row may do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Reset {
    /// The pilot's stellar becomes the placement's.
    Stellar,
    /// The system is marked explored.
    Explore,
    /// Where the ship last touched down becomes where it is placed.
    Touchdown,
    /// The ship is docked at the placement's stellar. The warp-ups end
    /// for the session, raising no halt: the relocation has no
    /// counterpart in the original, and no landing there halts them
    /// either (every `_warpUpSnd` reference: `_LoadSounds`, `_Bye`,
    /// `_HandleShipDisplay`, `_PlayGame`, `_SwitchedOut` @0x4f3a3, the
    /// jump collapse @0x68a42, sabotage @0x695d0, the warp stage
    /// @0x6b617-0x6c6d2, the cheat key @0x6e4db and the eject @0x6f925).
    Dock,
    /// A held `P` sound is dropped.
    HeldSound,
    /// A jump under way is given up.
    Jump,
    /// The thrust stops and the engine glow goes out.
    Thrust,
    /// A gate's pending entry ends.
    GateEntry,
    /// The shots and beams in flight are left behind.
    Combat,
    /// The cargo boxes are cleared, as `_killBoxes` kills them
    /// (see the `cargo_boxes` module).
    Boxes,
    /// The asteroid field is killed, as `_killAsteroids` kills it (see the
    /// `asteroids` module).
    Asteroids,
    /// The asteroid field is placed afresh about the ship, as
    /// `_CreateAsteroids` places it, and kept: a jump's arrival by the
    /// other reading of
    /// [`RuleKey::AsteroidEntryField`](crate::RuleKey::AsteroidEntryField)
    /// (see the `asteroids` module).
    PlaceAsteroids,
    /// The fight's strikes are forgotten.
    Strikes,
    /// The player's target is let go.
    Target,
    /// The dock request is dropped (see the `clearance` module).
    DockRequest,
    /// The boarding under way ends.
    Boarding,
    /// The hail under way ends.
    Hail,
    /// The fighters out are carried or left behind, as the fighter rules
    /// say for an arrival.
    FightersOut,
    /// The bar's hire rolls and the shops' rolls land as a landing's do:
    /// kept, the classes bought or declined to be drawn again, or, by the
    /// other reading of [`RuleKey::DayRolls`](crate::RuleKey::DayRolls),
    /// all drawn afresh; and the outfitter's opening starts anew.
    ShopRolls,
    /// A save is due.
    SaveDue,
    /// The stellar the offers were rolled at is forgotten, so the next
    /// landing rolls them afresh (see the `missions` module).
    ForgetOffers,
    /// The stellar the offers were rolled at is forgotten, and the
    /// visit's roll table drawn afresh before the system is populated,
    /// for the offers from a ship (see the `missions` module).
    RedrawOffers,
    /// The offers are emptied, as a landed `M` or `N` empties them
    /// (@0x1584b, @0x15949).
    EmptyOffers,
    /// [`SimSound::Arrived`] sounds.
    ArrivalSound,
    /// [`SimMessage::Arrived`] is raised, or the system's message buoy,
    /// [`SimMessage::Buoy`], in its place (see the `buoys` module).
    ArrivalMessage,
    /// The stellars shown are rebuilt, last: a rebuild point (see the
    /// `stellars` module).
    Rebuild,
}

/// What an entry tells as it announces the arrival: the system entered,
/// the scene's event, if any ([`scene_event`]), and how many fighter notes
/// stood before the entry queued its own.
#[derive(Clone, Copy)]
struct Heard {
    system: SystemId,
    event: Option<SessionEvent>,
    notes: usize,
}

/// What becomes of the course.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CourseStep {
    /// It is kept.
    Keep,
    /// A jump's arrival: the hops made dropped, and an arrival off the
    /// course cleared or its first hop dropped as
    /// [`RuleKey::RouteRewrite`](crate::RuleKey::RouteRewrite) says (see
    /// the `course` module).
    Arrival { hops: usize },
    /// It is cleared.
    Clear,
}

/// What becomes of the player's nav mode and its selection (see the
/// `nav` module), after the course's step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NavStep {
    /// Neither is written.
    Keep,
    /// The selection is let go and the mode kept.
    Deselect,
    /// The nav system is turned off, then the course's next hop cued: a
    /// hyperspace arrival (@0x6be25-0x6be2b, @0x6c4cd).
    OffThenCue,
}

/// What becomes of the traffic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TrafficStep {
    /// It is left as it is.
    None,
    /// The system is populated afresh ([`Session::populate`]).
    Populate,
    /// It goes on in the new system with the NPCs the player leads only
    /// (`Session::player_led`), the new
    /// system's ships coming with its arrivals over time
    /// ([`Traffic::retarget`]). The rest stay in the ship table through
    /// the next traffic tick, nothing given back, or are dropped at once,
    /// their aux ships given back, as
    /// [`RuleKey::MoveLeftBehind`](crate::RuleKey::MoveLeftBehind) says.
    Retarget,
    /// It is emptied, the fleet with it, and the next traffic tick in
    /// flight populates the system.
    Reset,
}

/// What a way in resets and writes: one row of the table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Resets {
    set: BTreeSet<Reset>,
    course: CourseStep,
    nav: NavStep,
    traffic: TrafficStep,
}

impl Resets {
    fn of(set: &[Reset], course: CourseStep, nav: NavStep, traffic: TrafficStep) -> Self {
        Self {
            set: set.iter().copied().collect(),
            course,
            nav,
            traffic,
        }
    }

    /// Whether the row does `reset`.
    fn has(&self, reset: Reset) -> bool {
        self.set.contains(&reset)
    }
}

/// What entering a system `how` resets and writes, with `rules` and
/// `facts`: the table, a row for each way in.
pub(super) fn resets(how: Entry, rules: &Rulebook, facts: EntryFacts) -> Resets {
    let arrival = ScriptEffectRules::from_rulebook(rules).arrival == RuleSource::Bible;
    let placed = rules.source_for(RuleKey::AsteroidEntryField) == RuleSource::Bible;
    match how {
        Entry::Jump { hops, .. } => jump(hops, placed),
        Entry::Gate => gate(),
        Entry::ScriptMove { landed: true } => script_move_landed(arrival),
        Entry::ScriptMove { landed: false } => script_move_in_flight(arrival, facts),
        Entry::ScriptStay => script_move_in_flight(false, facts),
        Entry::Relocate => relocate(),
        Entry::Rescue => rescue(),
    }
}

/// The escape pod's rescue (`_HandlePlayer` 0x68f24-0x69135): every way
/// `_ResetPlayer(0)` and the rescue let go of the scene, a held `P`
/// sound (`_missionSoundID` = -1) and a jump; the offers' stellar is
/// forgotten (`_lastMissionStellar` = -1) and their roll table drawn
/// afresh (`_missionRandom`, @0x1d7e3, @0x1d940); the system is explored
/// (`_AutoSetExploration(system, 0, 1)` @0x690cb) and set up afresh
/// (`_DeleteAllComputerShips` @0x68f69, `_SetupShipsInSystem` @0x69114);
/// the course is cleared (`_hyperRoute` @0x1d6bb-0x1d6cd). The nav mode is
/// left to the rescue, which turns it off as `_ResetPlayer` does
/// (@0x1d4cd-0x1d4d5). It sounds and
/// says nothing, and its stellars are rebuilt by the rescue itself, after
/// its days (`_HandleStellarSystemVisibility` @0x69135). It keeps the
/// asteroid field, as `_ResetPlayer` sets no `_killAsteroids`
/// (@0x1d8cd-0x1d8e2).
fn rescue() -> Resets {
    Resets::of(
        &[
            Reset::RedrawOffers,
            Reset::Stellar,
            Reset::Explore,
            Reset::HeldSound,
            Reset::Jump,
            Reset::Thrust,
            Reset::GateEntry,
            Reset::Combat,
            Reset::Boxes,
            Reset::Strikes,
            Reset::Target,
            Reset::Boarding,
            Reset::Hail,
        ],
        CourseStep::Clear,
        NavStep::Keep,
        TrafficStep::Populate,
    )
}

/// A jump's arrival, after its `hops`: as `_HandlePlayer`'s hyperspace
/// arrival, which sets the system up (`_SetupShipsInSystem`,
/// `_AutoSetExploration` @0x6c1f9), abandons the fighters out without a
/// jump's fuel (@0x6c0b9-0x6c0f9) and resets the mission sound ID
/// (@0x6c4ff), and forgets the stellar the offers were rolled at
/// (@0x6bfce), drawing the roll table afresh before the system is set up
/// (@0x6c040-0x6c066). It drops the hops made, and an arrival that made
/// none clears the course or drops its first hop as
/// [`RuleKey::RouteRewrite`](crate::RuleKey::RouteRewrite) says
/// (@0x6c472-0x6c4c6; see the `course` module); the jump's start
/// already stopped the thrust, and every tick ends a gate's pending
/// entry. The nav system is turned off as the arrival begins
/// (@0x6be25-0x6be2b) and the course's next hop cued after its step
/// (@0x6c4cd), so mode 3 comes back while hops remain. The asteroid
/// field is killed (@0x6c4fc), or, `placed` (the other reading of
/// [`RuleKey::AsteroidEntryField`](crate::RuleKey::AsteroidEntryField)),
/// placed afresh as `_CreateAsteroids` places it (@0x6c51a). The arrival
/// message is the system's buoy in place of the arrival line when it has
/// one (@0x6c25b-0x6c27a; see the `buoys` module). The dock request is
/// dropped (@0x6bfb5-0x6bfc4; see the `clearance` module). Populating
/// lets go of the strikes, the boarding, the hail and the target too. The caller arms the escort-strength timer
/// before entering, ahead of the jump's days ([`Session::arrive`]).
fn jump(hops: usize, placed: bool) -> Resets {
    let asteroids = if placed {
        Reset::PlaceAsteroids
    } else {
        Reset::Asteroids
    };
    Resets::of(
        &[
            Reset::RedrawOffers,
            Reset::Stellar,
            Reset::Explore,
            Reset::HeldSound,
            Reset::DockRequest,
            Reset::Combat,
            Reset::Boxes,
            asteroids,
            Reset::Strikes,
            Reset::Target,
            Reset::Boarding,
            Reset::Hail,
            Reset::FightersOut,
            Reset::ArrivalSound,
            Reset::ArrivalMessage,
            Reset::Rebuild,
        ],
        CourseStep::Arrival { hops },
        NavStep::OffThenCue,
        TrafficStep::Populate,
    )
}

/// Out of a hypergate or wormhole: `_PlayerEnterHypergate` (@0x637bf)
/// and `_PlayerEnterWormhole` (@0x64005) bypass the hyperspace arrival,
/// so a held sound is kept. The course is cleared and the thrust
/// stopped; the system is populated even when it is the one gone in,
/// and the arrival sounds but raises no message of its own (the caller
/// raises the gate's). The caller arms the escort-strength timer before
/// entering, as a take-off does ([`Session::enter_wormhole`]). The nav
/// selection is let go and the mode kept, the take-off's tail that the
/// gates join (@0x66edb-0x66f40, @0x674d2); the hypergate then turns the
/// nav system off itself ([`Session::enter_hypergate`]). The asteroid
/// field is killed (@0x63f47, @0x645f6).
fn gate() -> Resets {
    Resets::of(
        &[
            Reset::Stellar,
            Reset::Explore,
            Reset::Thrust,
            Reset::Combat,
            Reset::Boxes,
            Reset::Asteroids,
            Reset::Strikes,
            Reset::Target,
            Reset::Boarding,
            Reset::Hail,
            Reset::FightersOut,
            Reset::ArrivalSound,
            Reset::Rebuild,
        ],
        CourseStep::Clear,
        NavStep::Deselect,
        TrafficStep::Populate,
    )
}

/// A landed `M` or `N` (`_EvalSetExp` @0x1570e-0x158d4 and
/// @0x158d9-0x159ad): landed, the original empties `_dockedPortMissions`
/// (@0x1584b, @0x15949) and does nothing more, so only the pilot's
/// system and stellar change, the offers are emptied (the stellar they
/// were rolled at kept) and the stellars wait for the take-off. As an
/// `arrival` (the Bible's reading of `MoveArrival`) the system is
/// explored and the course cleared at once. The nav is kept: the
/// operator already wrote it, as `_EvalSetExp` does (`M` in mode 2 its
/// destination's first stellar, @0x156f2-0x15705; otherwise the selection
/// let go, @0x15733, @0x158fd; see the `nav` module), and the take-off
/// lets go of it (@0x674d2).
fn script_move_landed(arrival: bool) -> Resets {
    let mut set = vec![Reset::Stellar, Reset::SaveDue, Reset::EmptyOffers];
    let course = if arrival {
        set.push(Reset::Explore);
        CourseStep::Clear
    } else {
        CourseStep::Keep
    };
    Resets::of(&set, course, NavStep::Keep, TrafficStep::None)
}

/// An `M` or `N` in flight (`_EvalSetExp` @0x158a4): the shots are
/// killed and the stellars rebuilt; here the whole scene is let go of
/// with them, and a jump given up, the thrust stopping only when there
/// was one. The pilot's stellar is forgotten only in another system. As
/// an `arrival` the system is explored, the course cleared, the fighters
/// out dealt with and the system populated, even within the same
/// system; otherwise only into another system does the traffic go on,
/// with the ships the player leads alone (`_EvalSetExp` @0x15807,
/// @0x1590b), the rest left behind as
/// [`RuleKey::MoveLeftBehind`](crate::RuleKey::MoveLeftBehind) says. The
/// nav selection is let go and the mode kept (@0x157e3, @0x158fd). The
/// asteroid field is killed (@0x158ca, @0x1598e).
fn script_move_in_flight(arrival: bool, facts: EntryFacts) -> Resets {
    let mut set = vec![
        Reset::Jump,
        Reset::GateEntry,
        Reset::Combat,
        Reset::Boxes,
        Reset::Asteroids,
        Reset::Strikes,
        Reset::Target,
        Reset::Boarding,
        Reset::Hail,
        Reset::SaveDue,
        Reset::Rebuild,
    ];
    if facts.system_changed {
        set.push(Reset::Stellar);
    }
    if facts.jump_pending {
        set.push(Reset::Thrust);
    }
    let (course, traffic) = if arrival {
        set.extend([Reset::Explore, Reset::FightersOut]);
        (CourseStep::Clear, TrafficStep::Populate)
    } else if facts.system_changed {
        (CourseStep::Keep, TrafficStep::Retarget)
    } else {
        (CourseStep::Keep, TrafficStep::None)
    };
    Resets::of(&set, course, NavStep::Deselect, traffic)
}

/// The developer tools' move of a landed pilot, and the walker's landing
/// from anywhere ([`Session::land_at`], which then docks as a landing
/// does): no original does either. It docks the ship as a landing does,
/// the shops' rolls drawn afresh, and leaves the last system's traffic
/// and scene behind; it keeps a held sound and the fighters out, and
/// sounds nothing. The offers are emptied and their stellar forgotten, so
/// the next landing rolls them, and the nav selection is let go, the mode
/// kept, as the other ways in let go of it. The asteroid field is killed,
/// as every other move kills it.
fn relocate() -> Resets {
    Resets::of(
        &[
            Reset::ForgetOffers,
            Reset::EmptyOffers,
            Reset::Stellar,
            Reset::Explore,
            Reset::Touchdown,
            Reset::Dock,
            Reset::GateEntry,
            Reset::Combat,
            Reset::Boxes,
            Reset::Asteroids,
            Reset::Strikes,
            Reset::Target,
            Reset::Boarding,
            Reset::Hail,
            Reset::ShopRolls,
            Reset::SaveDue,
            Reset::Rebuild,
        ],
        CourseStep::Clear,
        NavStep::Deselect,
        TrafficStep::Reset,
    )
}

/// What entering a system `how` tells a view (see the `events` module),
/// with `scene_changed` whether the system entered is another than the one
/// whose stellars are shown. A jump, a gate and a move in flight rebuild
/// the scene even within the same system, so each tells
/// [`SessionEvent::SystemChanged`]; a landed move tells nothing, its
/// stellars and its event waiting for the take-off; a relocation tells
/// `SystemChanged` into another system and otherwise
/// [`SessionEvent::StellarChanged`].
pub(super) fn scene_event(how: Entry, scene_changed: bool) -> Option<SessionEvent> {
    match how {
        Entry::Jump { .. }
        | Entry::Gate
        | Entry::ScriptMove { landed: false }
        | Entry::ScriptStay
        | Entry::Rescue => Some(SessionEvent::SystemChanged),
        Entry::ScriptMove { landed: true } => None,
        Entry::Relocate if scene_changed => Some(SessionEvent::SystemChanged),
        Entry::Relocate => Some(SessionEvent::StellarChanged),
    }
}

/// No traffic to read: what [`Session::relocate`] enters a system with,
/// as its row resets the traffic without reading any.
#[derive(Clone, Copy, Debug)]
pub(super) struct NoTraffic;

impl TrafficCatalog for NoTraffic {
    fn system_traffic(&self, _id: SystemId) -> Option<SystemTraffic> {
        None
    }

    fn dude(&self, _id: DudeId) -> Option<DudeRecord> {
        None
    }

    fn fleets(&self) -> Vec<FleetRecord> {
        Vec::new()
    }

    fn persons(&self) -> Vec<PersonRecord> {
        Vec::new()
    }
}

impl Session {
    /// Enters `arrival`'s system `how`, resolved through the galaxy
    /// overlay, its stellars read again from `stellars` when it resolves to
    /// another: places the ship as the caller chose, makes the arrival
    /// writes, and resets what the row of [`resets`] for `how` says,
    /// populating from `catalog` on `chance` where it says so (see the
    /// module docs). A chain head made visible makes a save due and tells
    /// [`SessionEvent::GalaxyChanged`].
    pub(super) fn enter_system<S, C, R>(
        &mut self,
        how: Entry,
        arrival: Arrival,
        stellars: &S,
        catalog: &C,
        chance: &mut R,
    ) where
        S: PilotCatalog + ?Sized,
        C: TrafficCatalog + ?Sized,
        R: Chance + ?Sized,
    {
        let Arrival {
            system: asked,
            sites,
            placement,
        } = arrival;
        let resolved = self.galaxy.arrival(asked);
        if story::show(&mut self.pilot, &mut self.galaxy, resolved) {
            self.save_due = true;
            self.emit(SessionEvent::GalaxyChanged);
        }
        let system = resolved.to;
        let left = self.pilot.system;
        let facts = EntryFacts {
            system_changed: system != self.pilot.system,
            jump_pending: self.jump.is_some(),
        };
        let resets = resets(how, &self.rules, facts);
        // A way in that docks docks at a stellar of the system asked for.
        let dock = if resets.has(Reset::Dock) {
            Dock::At(asked, sites.clone())
        } else {
            Dock::AsItWas
        };
        let sites = if system == asked {
            sites
        } else {
            self.read_sites(stellars, system)
        };
        self.place(placement);
        if resets.has(Reset::Stellar) {
            self.pilot.stellar = placement.stellar;
        }
        if resets.has(Reset::Explore) {
            // At level 1, with its place's head, by the engine (see the
            // `explored` module).
            self.explore_place(system, ExploredLevel::Explored);
        }
        match resets.course {
            CourseStep::Keep => {}
            CourseStep::Arrival { hops } => self.course_after_arrival(hops, system),
            CourseStep::Clear => self.pilot.course.clear(),
        }
        if resets.has(Reset::Touchdown) {
            self.touchdown = self.player.position;
        }
        if resets.has(Reset::Dock) {
            self.landed = placement.stellar;
            self.bar_news = None;
            self.bar_nav = None;
            self.warp_ups.clear();
            // The warp-out needs no clearing here: a relocation from
            // flight docks after (`Session::land_at`), and one already
            // landed has none playing, as landing ended it.
        }
        let scene_changed = self.set_player_system(system, sites, dock);
        if resets.has(Reset::Dock) {
            // A relocation docks, opening the spaceport (see the `ranks`
            // module); undocked, it takes no government, so no rank's
            // price.
            self.open_port_prices();
        }
        // The fighter notes this entry queues, which a buoy drops.
        let notes = self.fighter_notes.len();
        self.leave_behind(&resets);
        if resets.has(Reset::RedrawOffers) {
            self.offers.redraw(&mut &mut *chance);
        }
        self.bring_in_traffic(
            resets.traffic,
            how.entrance(),
            (left, system),
            catalog,
            chance,
        );
        if resets.has(Reset::PlaceAsteroids) {
            self.place_asteroids(chance);
        }
        if resets.has(Reset::ShopRolls) {
            self.land_day_rolls();
            self.open_outfit_visit();
        }
        if resets.has(Reset::SaveDue) {
            self.save_due = true;
        }
        if resets.has(Reset::ForgetOffers) {
            self.forget_offers();
        }
        if resets.has(Reset::EmptyOffers) {
            self.empty_offers();
        }
        let heard = Heard {
            system,
            event: scene_event(how, scene_changed),
            notes,
        };
        self.announce(how, &resets, heard, stellars);
        // A jump's arrival raised its `Explored` before its days, at the
        // original's first nebula exploration (@0x6be12); a gate's or
        // wormhole's exit explores no nebula (`_PlayerEnterHypergate`
        // 0x637bf, `_PlayerEnterWormhole` 0x64005; see the `nebulae`
        // module).
        if resets.has(Reset::Explore) && !matches!(how, Entry::Jump { .. } | Entry::Gate) {
            self.raise(StoryEvent::Explored { system });
        }
        if resets.has(Reset::Rebuild) {
            self.rebuild_stellars();
        }
    }

    /// Makes `system` the player's system and `listed` every stellar it
    /// lists: the one owner of which stellars the session shows (see the
    /// module docs). In flight, or docked at a stellar of `system` itself,
    /// those of `listed` it shows are shown at once and nothing waits for
    /// the take-off. Docked at a stellar of another system (where the ship
    /// was, or `dock`'s system), that system's stellars stay shown, or
    /// `dock`'s become so, and `listed` waits for the take-off. Either
    /// way the stellars shown are those their system holds, and the one
    /// docked at ([`shown_sites`](super::shown_sites)). Whether the system
    /// whose stellars are shown changed.
    pub(super) fn set_player_system(
        &mut self,
        system: SystemId,
        listed: Vec<LandingSite>,
        dock: Dock,
    ) -> bool {
        let before = self.shown;
        self.pilot.system = system;
        let (docked_in, docked_listed) = match dock {
            Dock::AsItWas => (self.shown, None),
            Dock::At(at, at_listed) => (at, Some(at_listed)),
        };
        if self.landed.is_none() || docked_in == system {
            self.show(system, listed);
            self.next_sites = None;
        } else {
            if let Some(docked_listed) = docked_listed {
                self.show(docked_in, docked_listed);
            }
            self.shown = docked_in;
            self.next_sites = Some(listed);
        }
        self.shown != before
    }

    /// Shows the stellars of `listed`, every one `system` lists, that it
    /// shows ([`shown_sites`](super::shown_sites)).
    fn show(&mut self, system: SystemId, listed: Vec<LandingSite>) {
        self.sites = super::shown_sites(
            &self.galaxy,
            &self.listed_stellars,
            system,
            &listed,
            self.landed,
        );
        self.listed = listed;
        self.shown = system;
        self.keep_gate_sprites();
    }

    /// Derives the stellars shown afresh from those their system lists, in
    /// flight only, and says whether that changed which are shown: a
    /// flight rebuild point's part (see the `stellars` module), as
    /// `_HandleStellarSystemVisibility` sets which stellars are active
    /// and owned by the player's system at each rebuild point.
    ///
    /// Docked, nothing is derived: the landing's rebuild (@0x5fbf0) would
    /// change only the sprites the flight shows, and the take-off's
    /// (@0x674c7) derives them again before the first frame, so the
    /// difference crosses no boundary the player sees. Every flight
    /// rebuild point lets go of the navigation target first, so it never
    /// names a stellar no longer shown.
    pub(super) fn reshow(&mut self) -> bool {
        if self.landed.is_some() {
            return false;
        }
        let listed = std::mem::take(&mut self.listed);
        let before: Vec<StellarId> = self.stellars_shown().collect();
        self.show(self.shown, listed);
        !self.stellars_shown().eq(before)
    }

    /// The stellars shown, in navigation order (their system's `NavDefs`
    /// order): what the flight view draws.
    pub fn stellars_shown(&self) -> impl Iterator<Item = StellarId> + '_ {
        self.sites.iter().map(|site| site.id)
    }

    /// Checks the invariant [`Session::set_player_system`] keeps: docked,
    /// the stellar the ship is docked at is among the stellars shown.
    #[cfg(test)]
    pub(crate) fn assert_docked_among_shown(&self) {
        if let Some(stellar) = self.landed {
            let shown: Vec<StellarId> = self.sites.iter().map(|site| site.id).collect();
            assert!(
                shown.contains(&stellar),
                "docked at {stellar:?}, showing {shown:?} of {:?}",
                self.shown
            );
        }
    }

    /// Puts the ship where `placement` says.
    fn place(&mut self, placement: Placement) {
        match placement.ship {
            ShipPlacement::Arrive(state) => self.player = state,
            ShipPlacement::AtRest(position) => {
                self.player.position = position;
                self.player.velocity = Vec2::ZERO;
            }
            ShipPlacement::Keep => {}
        }
        self.hold_position |= placement.hold;
    }

    /// Lets go of what `resets` says of the system left: the held sound,
    /// a jump, the thrust, the nav selection (after the course's step and
    /// in the system entered, which the cue reads), the scene, and the
    /// fighters out.
    fn leave_behind(&mut self, resets: &Resets) {
        if resets.has(Reset::HeldSound) {
            self.queued.sound = None;
        }
        if resets.has(Reset::Jump) {
            self.jump = None;
        }
        if resets.has(Reset::Thrust) {
            self.stop_thrust();
        }
        match resets.nav {
            NavStep::Keep => {}
            NavStep::Deselect => self.deselect(),
            NavStep::OffThenCue => {
                self.set_nav(NavMode::Off);
                self.cue_course();
            }
        }
        if resets.has(Reset::GateEntry) {
            self.gate = None;
        }
        if resets.has(Reset::Combat) {
            self.combat.clear();
        }
        if resets.has(Reset::Boxes) {
            self.clear_boxes();
        }
        if resets.has(Reset::Asteroids) {
            self.kill_asteroids();
        }
        if resets.has(Reset::Strikes) {
            self.strikes.clear();
        }
        if resets.has(Reset::Target) {
            self.target = None;
        }
        if resets.has(Reset::DockRequest) {
            self.dock = None;
        }
        if resets.has(Reset::Boarding) {
            self.aboard = None;
        }
        if resets.has(Reset::Hail) {
            self.talk = None;
        }
        if resets.has(Reset::FightersOut) {
            self.leave_with_fighters(false);
        }
    }

    /// Does `step` to the traffic of a system entered `how`, by a way in
    /// whose enter-system hook is still to run, if any, going `from` one
    /// system `to` another, reading `catalog` and drawing on `chance` to
    /// populate: the only draw entering a system makes.
    fn bring_in_traffic<C, R>(
        &mut self,
        step: TrafficStep,
        how: Option<Entrance>,
        (from, to): (SystemId, SystemId),
        catalog: &C,
        chance: &mut R,
    ) where
        C: TrafficCatalog + ?Sized,
        R: Chance + ?Sized,
    {
        match step {
            TrafficStep::None => {}
            TrafficStep::Populate => self.populate_as(catalog, chance, how),
            TrafficStep::Retarget => {
                let table = self.spawn_table(catalog);
                // Every ship the player leads, as `_EvalSetExp`
                // (@0x15807-0x1583e, @0x1590b-0x1593d) carries.
                let led: BTreeSet<_> = self.player_led().map(|npc| npc.id).collect();
                // By the engine the rest stay in the ship table until the
                // display pass frees them, nothing given back
                // (`_HandleShipDisplay` @0x2ded6-0x2dee7).
                let leave = if self.rules.source_for(RuleKey::MoveLeftBehind) == RuleSource::Engine
                {
                    Leave::Held { from, to }
                } else {
                    self.give_back_aux_ships(&led);
                    Leave::Dropped
                };
                self.traffic.retarget(table, &led, leave);
            }
            TrafficStep::Reset => {
                self.give_back_aux_ships(&BTreeSet::new());
                self.traffic = Traffic::new();
                self.traffic_due = true;
                self.fleet.clear();
                self.story.watched.clear();
            }
        }
    }

    /// Whether an arrival `how` plays the warp-out, `snd ` 130: a jump's
    /// natural end only while it is not still playing (`_HandlePlayer`
    /// @0x6bbe1-0x6bbe9); a jump's fallback end and a gate's or wormhole's
    /// exit always (@0x6c706, @0x63e57, @0x6443d).
    fn sounds_warp_out(&self, how: Entry) -> bool {
        !matches!(
            how,
            Entry::Jump {
                natural_end: true,
                ..
            }
        ) || !self.warp_out.playing()
    }

    /// Sounds and raises the arrival in `heard`'s system, as `resets`
    /// says, tells its event, if any ([`scene_event`]), and raises
    /// [`StoryEvent::EnteredSystem`], entered `how`, for a way in that
    /// names an entrance ([`Entry::entrance`]). The arrival message is the
    /// system's buoy, its strings read from `catalog`, or the arrival line
    /// ([`Session::arrival_message`]); a buoy replaces the
    /// abandoned-fighters suffix too, so it drops the fighter notes this
    /// entry queued (see the `buoys` module).
    fn announce<S>(&mut self, how: Entry, resets: &Resets, heard: Heard, catalog: &S)
    where
        S: PilotCatalog + ?Sized,
    {
        let Heard {
            system,
            event,
            notes,
        } = heard;
        if resets.has(Reset::ArrivalSound) && self.sounds_warp_out(how) {
            self.warp_out.start(self.warp_out_sound);
            self.sounds.push(SimSound::Arrived);
        }
        if resets.has(Reset::ArrivalMessage) {
            let message = self.arrival_message(system, catalog);
            if matches!(message, SimMessage::Buoy(_)) {
                self.fighter_notes.truncate(notes);
            }
            self.say(message);
            // The hyperspace arrival leaves the pilot past the tutorial
            // (`_HandlePlayer` @0x6c002; see the `tutorial` module).
            self.tutorial.jumped();
        }
        if let Some(event) = event {
            self.emit(event);
        }
        if let Some(how) = how.entrance() {
            self.raise(StoryEvent::EnteredSystem { system, how });
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::rulebook::RuleKey;
    use crate::testkit::catalog;

    const READINGS: [RuleSource; 2] = [RuleSource::Engine, RuleSource::Bible];

    /// The rules with `MoveArrival` read as `reading`.
    fn arrival(reading: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::MoveArrival, reading)
    }

    fn facts(system_changed: bool, jump_pending: bool) -> EntryFacts {
        EntryFacts {
            system_changed,
            jump_pending,
        }
    }

    /// The scene every way in but a landed move lets go of, a gate's
    /// pending entry aside, and its stellars rebuilt.
    const SCENE: [Reset; 8] = [
        Reset::Combat,
        Reset::Boxes,
        Reset::Asteroids,
        Reset::Strikes,
        Reset::Target,
        Reset::Boarding,
        Reset::Hail,
        Reset::Rebuild,
    ];

    fn with(base: &[Reset], more: &[Reset]) -> Vec<Reset> {
        [base, more].concat()
    }

    #[test]
    fn a_jump_drops_its_hops_explores_lets_go_of_the_scene_and_a_held_sound_and_populates() {
        for hops in [0, 2] {
            let expected = Resets::of(
                &with(
                    &SCENE,
                    &[
                        Reset::Stellar,
                        Reset::Explore,
                        Reset::HeldSound,
                        Reset::DockRequest,
                        Reset::FightersOut,
                        Reset::ArrivalSound,
                        Reset::ArrivalMessage,
                        Reset::RedrawOffers,
                    ],
                ),
                CourseStep::Arrival { hops },
                NavStep::OffThenCue,
                TrafficStep::Populate,
            );
            for f in [facts(true, false), facts(false, true)] {
                assert_eq!(
                    resets(
                        Entry::Jump {
                            hops,
                            natural_end: false
                        },
                        &Rulebook::default(),
                        f
                    ),
                    expected,
                    "{hops} {f:?}"
                );
            }
        }
    }

    #[test]
    fn placed_a_jump_places_the_asteroid_field_in_place_of_killing_it() {
        let placed =
            Rulebook::default().with_override(RuleKey::AsteroidEntryField, RuleSource::Bible);
        let engine = resets(
            Entry::Jump {
                hops: 1,
                natural_end: true,
            },
            &Rulebook::default(),
            facts(true, false),
        );
        let row = resets(
            Entry::Jump {
                hops: 1,
                natural_end: true,
            },
            &placed,
            facts(true, false),
        );
        assert!(engine.has(Reset::Asteroids) && !engine.has(Reset::PlaceAsteroids));
        assert!(row.has(Reset::PlaceAsteroids) && !row.has(Reset::Asteroids));
        let mut swapped = engine.clone();
        swapped.set.remove(&Reset::Asteroids);
        swapped.set.insert(Reset::PlaceAsteroids);
        assert_eq!(row, swapped, "nothing else changes");
    }

    #[test]
    fn only_a_jumps_row_reads_asteroid_entry_field() {
        let placed =
            Rulebook::default().with_override(RuleKey::AsteroidEntryField, RuleSource::Bible);
        for how in [
            Entry::Gate,
            Entry::Relocate,
            Entry::Rescue,
            Entry::ScriptMove { landed: true },
            Entry::ScriptMove { landed: false },
        ] {
            assert_eq!(
                resets(how, &placed, facts(true, false)),
                resets(how, &Rulebook::default(), facts(true, false)),
                "{how:?}"
            );
        }
    }

    #[test]
    fn a_gate_clears_the_course_stops_the_thrust_keeps_a_held_sound_and_populates_into_any_system()
    {
        let expected = Resets::of(
            &with(
                &SCENE,
                &[
                    Reset::Stellar,
                    Reset::Explore,
                    Reset::Thrust,
                    Reset::FightersOut,
                    Reset::ArrivalSound,
                ],
            ),
            CourseStep::Clear,
            NavStep::Deselect,
            TrafficStep::Populate,
        );
        for changed in [true, false] {
            assert_eq!(
                resets(Entry::Gate, &Rulebook::default(), facts(changed, false)),
                expected,
                "{changed}"
            );
        }
    }

    #[test]
    fn a_relocation_docks_lets_go_of_the_scene_and_resets_the_traffic_and_shops() {
        assert_eq!(
            resets(Entry::Relocate, &Rulebook::default(), facts(true, false)),
            Resets::of(
                &with(
                    &SCENE,
                    &[
                        Reset::Stellar,
                        Reset::Explore,
                        Reset::Touchdown,
                        Reset::Dock,
                        Reset::GateEntry,
                        Reset::ShopRolls,
                        Reset::SaveDue,
                        Reset::ForgetOffers,
                        Reset::EmptyOffers,
                    ],
                ),
                CourseStep::Clear,
                NavStep::Deselect,
                TrafficStep::Reset,
            )
        );
    }

    #[test]
    fn a_rescue_clears_the_course_lets_go_of_the_scene_a_jump_and_a_held_sound_and_populates() {
        // It keeps the asteroid field (@0x1d8cd-0x1d8e2), and rebuilds its
        // stellars itself.
        let scene: Vec<Reset> = SCENE
            .into_iter()
            .filter(|&reset| reset != Reset::Rebuild && reset != Reset::Asteroids)
            .collect();
        let expected = Resets::of(
            &with(
                &scene,
                &[
                    Reset::Stellar,
                    Reset::Explore,
                    Reset::HeldSound,
                    Reset::Jump,
                    Reset::Thrust,
                    Reset::GateEntry,
                    Reset::RedrawOffers,
                ],
            ),
            CourseStep::Clear,
            NavStep::Keep,
            TrafficStep::Populate,
        );
        for f in [facts(true, false), facts(false, true)] {
            assert_eq!(
                resets(Entry::Rescue, &Rulebook::default(), f),
                expected,
                "{f:?}"
            );
        }
    }

    #[test]
    fn a_landed_move_changes_where_the_pilot_is_and_defers_the_stellars() {
        let landed = Entry::ScriptMove { landed: true };
        assert_eq!(
            resets(landed, &arrival(RuleSource::Engine), facts(true, false)),
            Resets::of(
                &[Reset::Stellar, Reset::SaveDue, Reset::EmptyOffers],
                CourseStep::Keep,
                NavStep::Keep,
                TrafficStep::None,
            ),
            "by the engine"
        );
        assert_eq!(
            resets(landed, &arrival(RuleSource::Bible), facts(true, false)),
            Resets::of(
                &[
                    Reset::Stellar,
                    Reset::Explore,
                    Reset::SaveDue,
                    Reset::EmptyOffers,
                ],
                CourseStep::Clear,
                NavStep::Keep,
                TrafficStep::None,
            ),
            "as an arrival"
        );
    }

    #[test]
    fn a_move_in_flight_lets_go_of_the_scene_and_does_the_rest_as_its_reading_and_the_facts_say() {
        let in_flight = Entry::ScriptMove { landed: false };
        let always = with(&SCENE, &[Reset::Jump, Reset::GateEntry, Reset::SaveDue]);
        for reading in READINGS {
            for changed in [false, true] {
                for pending in [false, true] {
                    let mut set = always.clone();
                    if changed {
                        set.push(Reset::Stellar);
                    }
                    if pending {
                        set.push(Reset::Thrust);
                    }
                    let (course, traffic) = match (reading, changed) {
                        (RuleSource::Bible, _) => {
                            set.extend([Reset::Explore, Reset::FightersOut]);
                            (CourseStep::Clear, TrafficStep::Populate)
                        }
                        (RuleSource::Engine, true) => (CourseStep::Keep, TrafficStep::Retarget),
                        (RuleSource::Engine, false) => (CourseStep::Keep, TrafficStep::None),
                    };
                    assert_eq!(
                        resets(in_flight, &arrival(reading), facts(changed, pending)),
                        Resets::of(&set, course, NavStep::Deselect, traffic),
                        "{reading:?} changed {changed} pending {pending}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_stay_is_a_move_within_the_system_by_the_engine_whatever_the_reading() {
        let always = with(&SCENE, &[Reset::Jump, Reset::GateEntry, Reset::SaveDue]);
        for reading in READINGS {
            for pending in [false, true] {
                let mut set = always.clone();
                if pending {
                    set.push(Reset::Thrust);
                }
                assert_eq!(
                    resets(Entry::ScriptStay, &arrival(reading), facts(false, pending)),
                    Resets::of(&set, CourseStep::Keep, NavStep::Deselect, TrafficStep::None),
                    "{reading:?} pending {pending}"
                );
            }
        }
    }

    #[test]
    fn only_a_moves_rows_read_move_arrival_from_the_rulebook() {
        let rows = |reading| {
            let rules = arrival(reading);
            [
                Entry::Jump {
                    hops: 1,
                    natural_end: true,
                },
                Entry::Gate,
                Entry::Relocate,
                Entry::Rescue,
                Entry::ScriptMove { landed: true },
                Entry::ScriptMove { landed: false },
            ]
            .map(|how| resets(how, &rules, facts(true, false)))
        };
        let (engine, bible) = (rows(RuleSource::Engine), rows(RuleSource::Bible));
        assert_eq!(
            engine[..4],
            bible[..4],
            "a jump, a gate, a relocation and a rescue"
        );
        assert_ne!(engine[4], bible[4], "a landed move");
        assert_ne!(engine[5], bible[5], "a move in flight");
        let every_other = RuleKey::ALL
            .into_iter()
            .filter(|&key| key != RuleKey::MoveArrival)
            .fold(Rulebook::default(), |book, key| {
                book.with_override(key, RuleSource::Bible)
            });
        let in_flight = Entry::ScriptMove { landed: false };
        assert_eq!(
            resets(in_flight, &every_other, facts(true, false)),
            resets(in_flight, &Rulebook::default(), facts(true, false)),
            "no other rule"
        );
    }

    #[test]
    fn each_way_in_is_the_entrance_its_story_event_names() {
        for (how, entrance) in [
            (
                Entry::Jump {
                    hops: 0,
                    natural_end: true,
                },
                Entrance::Jump,
            ),
            (
                Entry::Jump {
                    hops: 3,
                    natural_end: false,
                },
                Entrance::Jump,
            ),
            (Entry::Gate, Entrance::Gate),
            (
                Entry::ScriptMove { landed: true },
                Entrance::ScriptMove { landed: true },
            ),
            (
                Entry::ScriptMove { landed: false },
                Entrance::ScriptMove { landed: false },
            ),
            (Entry::ScriptStay, Entrance::ScriptMove { landed: false }),
            (Entry::Relocate, Entrance::Relocate),
        ] {
            assert_eq!(how.entrance(), Some(entrance), "{how:?}");
        }
        assert_eq!(Entry::Rescue.entrance(), None, "the rescue names none");
    }

    #[test]
    fn every_way_in_but_a_landed_move_tells_the_scene_changed_and_a_relocation_tells_which() {
        use SessionEvent::{StellarChanged, SystemChanged};
        for changed in [false, true] {
            for (how, event) in [
                (
                    Entry::Jump {
                        hops: 1,
                        natural_end: false,
                    },
                    Some(SystemChanged),
                ),
                (Entry::Gate, Some(SystemChanged)),
                (Entry::ScriptMove { landed: false }, Some(SystemChanged)),
                (Entry::ScriptStay, Some(SystemChanged)),
                (Entry::ScriptMove { landed: true }, None),
                (Entry::Rescue, Some(SystemChanged)),
            ] {
                assert_eq!(scene_event(how, changed), event, "{how:?} {changed}");
            }
        }
        assert_eq!(scene_event(Entry::Relocate, true), Some(SystemChanged));
        assert_eq!(scene_event(Entry::Relocate, false), Some(StellarChanged));
    }

    /// Enters system 130, the one the session is in, by a move in flight
    /// that resets nothing more, placed as `ship` and `hold` say.
    fn placed(session: &mut Session, ship: ShipPlacement, hold: bool) {
        let arrival = Arrival {
            system: session.pilot.system,
            sites: session.sites.clone(),
            placement: Placement {
                ship,
                stellar: None,
                hold,
            },
        };
        session.enter_system(
            Entry::ScriptMove { landed: false },
            arrival,
            &catalog(),
            &catalog(),
            &mut NeverFires,
        );
    }

    #[test]
    fn the_ship_is_placed_as_the_caller_chose() {
        let mut session = Session::start(&catalog()).expect("starts");
        let moving = ShipState {
            position: Vec2::new(10.0, 20.0),
            velocity: Vec2::new(3.0, -4.0),
            heading: 1.5,
        };
        placed(&mut session, ShipPlacement::Arrive(moving), false);
        assert_eq!(*session.player(), moving, "exactly so");
        placed(&mut session, ShipPlacement::Keep, false);
        assert_eq!(*session.player(), moving, "as it was");
        placed(
            &mut session,
            ShipPlacement::AtRest(Vec2::new(-5.0, 6.0)),
            false,
        );
        assert_eq!(
            *session.player(),
            ShipState {
                position: Vec2::new(-5.0, 6.0),
                velocity: Vec2::ZERO,
                heading: 1.5,
            },
            "at rest, its heading kept"
        );
    }

    #[test]
    fn a_placements_hold_is_added_to_the_take_offs_never_taken_away() {
        let mut session = Session::start(&catalog()).expect("starts");
        placed(&mut session, ShipPlacement::Keep, false);
        assert!(!session.hold_position);
        placed(&mut session, ShipPlacement::Keep, true);
        assert!(session.hold_position);
        placed(&mut session, ShipPlacement::Keep, false);
        assert!(session.hold_position, "kept");
    }
}
