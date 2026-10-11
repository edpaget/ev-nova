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
//! placed, the system explored and the fighters out dealt with.
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
//! another, and for a pilot flown docked at a stellar of a hidden system at
//! its place. A mover says only where the ship is docked ([`Dock`]).
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
use crate::rulebook::{RuleSource, Rulebook};
use crate::sound::SimSound;
#[cfg(doc)]
use crate::story::OverlaidGalaxy;
use crate::story::{Entrance, StoryEvent};
use crate::traffic::Traffic;

/// How the player enters a system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Entry {
    /// A hyperspace jump's arrival, having made `hops` hops along the
    /// course.
    Jump { hops: usize },
    /// Out of a hypergate or wormhole.
    Gate,
    /// `M` or `N`, landed or in flight.
    ScriptMove { landed: bool },
    /// The developer tools' move, landed, or a walker's landing
    /// ([`Session::land_at`]), landed or in flight.
    Relocate,
}

/// The way in a [`StoryEvent::EnteredSystem`](crate::story::StoryEvent)
/// names: a jump's hops left out.
impl From<Entry> for Entrance {
    fn from(how: Entry) -> Self {
        match how {
            Entry::Jump { .. } => Self::Jump,
            Entry::Gate => Self::Gate,
            Entry::ScriptMove { landed } => Self::ScriptMove { landed },
            Entry::Relocate => Self::Relocate,
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
    /// The ship is docked at the placement's stellar.
    Dock,
    /// A held `P` sound is dropped.
    HeldSound,
    /// A jump under way is given up.
    Jump,
    /// The thrust stops and the engine glow goes out.
    Thrust,
    /// The navigation target is let go.
    NavTarget,
    /// A gate's pending entry ends.
    GateEntry,
    /// The shots and beams in flight are left behind.
    Combat,
    /// The fight's strikes are forgotten.
    Strikes,
    /// The player's target is let go.
    Target,
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
    /// [`SimMessage::Arrived`] is raised.
    ArrivalMessage,
    /// The stellars shown are rebuilt, last: a rebuild point (see the
    /// `stellars` module).
    Rebuild,
}

/// What becomes of the course.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CourseStep {
    /// It is kept.
    Keep,
    /// Its first so many systems, the hops made, are taken off.
    Drop(usize),
    /// It is cleared.
    Clear,
}

/// What becomes of the traffic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TrafficStep {
    /// It is left as it is.
    None,
    /// The system is populated afresh ([`Session::populate`]).
    Populate,
    /// It goes on in the new system with the fleet's NPCs only, the new
    /// system's ships coming with its arrivals over time
    /// ([`Traffic::retarget`]).
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
    traffic: TrafficStep,
}

impl Resets {
    fn of(set: &[Reset], course: CourseStep, traffic: TrafficStep) -> Self {
        Self {
            set: set.iter().copied().collect(),
            course,
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
    match how {
        Entry::Jump { hops } => jump(hops),
        Entry::Gate => gate(),
        Entry::ScriptMove { landed: true } => script_move_landed(arrival),
        Entry::ScriptMove { landed: false } => script_move_in_flight(arrival, facts),
        Entry::Relocate => relocate(),
    }
}

/// A jump's arrival, after its `hops`: as `_HandlePlayer`'s hyperspace
/// arrival, which sets the system up (`_SetupShipsInSystem`,
/// `_AutoSetExploration` @0x6c1f9), abandons the fighters out without a
/// jump's fuel (@0x6c0b9-0x6c0f9) and resets the mission sound ID
/// (@0x6c4ff), and forgets the stellar the offers were rolled at
/// (@0x6bfce), drawing the roll table afresh before the system is set up
/// (@0x6c040-0x6c066). It never clears the course, only drops the hops made; the
/// jump's start already stopped the thrust, and every tick ends a gate's
/// pending entry. Populating lets go of the strikes, the boarding, the
/// hail and the target too.
fn jump(hops: usize) -> Resets {
    Resets::of(
        &[
            Reset::RedrawOffers,
            Reset::Stellar,
            Reset::Explore,
            Reset::HeldSound,
            Reset::NavTarget,
            Reset::Combat,
            Reset::Strikes,
            Reset::Target,
            Reset::Boarding,
            Reset::Hail,
            Reset::FightersOut,
            Reset::ArrivalSound,
            Reset::ArrivalMessage,
            Reset::Rebuild,
        ],
        CourseStep::Drop(hops),
        TrafficStep::Populate,
    )
}

/// Out of a hypergate or wormhole: `_PlayerEnterHypergate` (@0x637bf)
/// and `_PlayerEnterWormhole` (@0x64005) bypass the hyperspace arrival,
/// so a held sound is kept. The course is cleared and the thrust
/// stopped; the system is populated even when it is the one gone in,
/// and the arrival sounds but raises no message of its own (the caller
/// raises the gate's).
fn gate() -> Resets {
    Resets::of(
        &[
            Reset::Stellar,
            Reset::Explore,
            Reset::Thrust,
            Reset::NavTarget,
            Reset::Combat,
            Reset::Strikes,
            Reset::Target,
            Reset::Boarding,
            Reset::Hail,
            Reset::FightersOut,
            Reset::ArrivalSound,
            Reset::Rebuild,
        ],
        CourseStep::Clear,
        TrafficStep::Populate,
    )
}

/// A landed `M` or `N` (`_EvalSetExp` @0x1570e-0x158d4 and
/// @0x158d9-0x159ad): landed, the original empties `_dockedPortMissions`
/// (@0x1584b, @0x15949) and does nothing more, so only the pilot's
/// system and stellar change, the offers are emptied (the stellar they
/// were rolled at kept) and the stellars wait for the take-off. As an
/// `arrival` (the Bible's reading of `MoveArrival`) the system is
/// explored and the course cleared at once.
fn script_move_landed(arrival: bool) -> Resets {
    let mut set = vec![Reset::Stellar, Reset::SaveDue, Reset::EmptyOffers];
    let course = if arrival {
        set.push(Reset::Explore);
        CourseStep::Clear
    } else {
        CourseStep::Keep
    };
    Resets::of(&set, course, TrafficStep::None)
}

/// An `M` or `N` in flight (`_EvalSetExp` @0x158a4): the shots are
/// killed and the stellars rebuilt; here the whole scene is let go of
/// with them, and a jump given up, the thrust stopping only when there
/// was one. The pilot's stellar is forgotten only in another system. As
/// an `arrival` the system is explored, the course cleared, the fighters
/// out dealt with and the system populated, even within the same
/// system; otherwise only into another system does the traffic go on,
/// with the fleet's NPCs alone.
fn script_move_in_flight(arrival: bool, facts: EntryFacts) -> Resets {
    let mut set = vec![
        Reset::Jump,
        Reset::NavTarget,
        Reset::GateEntry,
        Reset::Combat,
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
    Resets::of(&set, course, traffic)
}

/// The developer tools' move of a landed pilot, and the walker's landing
/// from anywhere ([`Session::land_at`], which then docks as a landing
/// does): no original does either. It docks the ship as a landing does,
/// the shops' rolls drawn afresh, and leaves the last system's traffic
/// and scene behind; it keeps a held sound and the fighters out, and
/// sounds nothing. The offers are emptied and their stellar forgotten, so
/// the next landing rolls them.
fn relocate() -> Resets {
    Resets::of(
        &[
            Reset::ForgetOffers,
            Reset::EmptyOffers,
            Reset::Stellar,
            Reset::Explore,
            Reset::Touchdown,
            Reset::Dock,
            Reset::NavTarget,
            Reset::GateEntry,
            Reset::Combat,
            Reset::Strikes,
            Reset::Target,
            Reset::Boarding,
            Reset::Hail,
            Reset::ShopRolls,
            Reset::SaveDue,
            Reset::Rebuild,
        ],
        CourseStep::Clear,
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
        Entry::Jump { .. } | Entry::Gate | Entry::ScriptMove { landed: false } => {
            Some(SessionEvent::SystemChanged)
        }
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
            self.pilot.explore(system);
        }
        match resets.course {
            CourseStep::Keep => {}
            CourseStep::Drop(hops) => {
                let hops = hops.min(self.pilot.course.len());
                self.pilot.course.drain(..hops);
            }
            CourseStep::Clear => self.pilot.course.clear(),
        }
        if resets.has(Reset::Touchdown) {
            self.touchdown = self.player.position;
        }
        if resets.has(Reset::Dock) {
            self.landed = placement.stellar;
        }
        let scene_changed = self.set_player_system(system, sites, dock);
        self.leave_behind(&resets);
        if resets.has(Reset::RedrawOffers) {
            self.offers.redraw(&mut &mut *chance);
        }
        self.bring_in_traffic(resets.traffic, how.into(), catalog, chance);
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
        self.announce(how, &resets, system, scene_event(how, scene_changed));
        // A jump's arrival raised its `Explored` before its days, at the
        // original's first nebula exploration (@0x6be12).
        if resets.has(Reset::Explore) && !matches!(how, Entry::Jump { .. }) {
            self.raise(StoryEvent::Explored { system });
        }
        if resets.has(Reset::Rebuild) {
            self.rebuild_stellars();
        }
    }

    /// Makes `system` the player's system and `sites` its stellars: the one
    /// owner of which stellars the session shows (see the module docs).
    /// In flight, or docked at a stellar of `system` itself, `sites` are
    /// shown at once and nothing waits for the take-off. Docked at a
    /// stellar of another system (where the ship was, or `dock`'s system),
    /// that system's stellars stay shown, or `dock`'s become so, and
    /// `sites` wait for the take-off. Whether the system whose stellars are
    /// shown changed.
    pub(super) fn set_player_system(
        &mut self,
        system: SystemId,
        sites: Vec<LandingSite>,
        dock: Dock,
    ) -> bool {
        let before = self.shown;
        self.pilot.system = system;
        let (docked_in, docked_sites) = match dock {
            Dock::AsItWas => (self.shown, None),
            Dock::At(at, at_sites) => (at, Some(at_sites)),
        };
        if self.landed.is_none() || docked_in == system {
            self.sites = sites;
            self.shown = system;
            self.next_sites = None;
        } else {
            if let Some(docked_sites) = docked_sites {
                self.sites = docked_sites;
            }
            self.shown = docked_in;
            self.next_sites = Some(sites);
        }
        self.shown != before
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
    /// a jump, the thrust, the scene, and the fighters out.
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
        if resets.has(Reset::NavTarget) {
            self.nav_target = None;
        }
        if resets.has(Reset::GateEntry) {
            self.gate = None;
        }
        if resets.has(Reset::Combat) {
            self.combat.clear();
        }
        if resets.has(Reset::Strikes) {
            self.strikes.clear();
        }
        if resets.has(Reset::Target) {
            self.target = None;
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

    /// Does `step` to the traffic of a system entered `how`, reading
    /// `catalog` and drawing on `chance` to populate: the only draw
    /// entering a system makes.
    fn bring_in_traffic<C, R>(
        &mut self,
        step: TrafficStep,
        how: Entrance,
        catalog: &C,
        chance: &mut R,
    ) where
        C: TrafficCatalog + ?Sized,
        R: Chance + ?Sized,
    {
        match step {
            TrafficStep::None => {}
            TrafficStep::Populate => self.populate_entering(catalog, how, chance),
            TrafficStep::Retarget => {
                let table = self.spawn_table(catalog);
                let fleet: BTreeSet<_> = self.fleet.iter().flatten().copied().collect();
                self.traffic.retarget(table, &fleet);
            }
            TrafficStep::Reset => {
                self.traffic = Traffic::new();
                self.traffic_due = true;
                self.fleet.clear();
                self.story.watched.clear();
            }
        }
    }

    /// Sounds and raises the arrival in `system`, as `resets` says, tells
    /// `event`, if any ([`scene_event`]), and raises
    /// [`StoryEvent::EnteredSystem`], entered `how`.
    fn announce(
        &mut self,
        how: Entry,
        resets: &Resets,
        system: SystemId,
        event: Option<SessionEvent>,
    ) {
        if resets.has(Reset::ArrivalSound) {
            self.sounds.push(SimSound::Arrived);
        }
        if resets.has(Reset::ArrivalMessage) {
            self.messages.push(SimMessage::Arrived(system));
        }
        if let Some(event) = event {
            self.emit(event);
        }
        self.raise(StoryEvent::EnteredSystem {
            system,
            how: how.into(),
        });
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
    const SCENE: [Reset; 7] = [
        Reset::NavTarget,
        Reset::Combat,
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
                        Reset::FightersOut,
                        Reset::ArrivalSound,
                        Reset::ArrivalMessage,
                        Reset::RedrawOffers,
                    ],
                ),
                CourseStep::Drop(hops),
                TrafficStep::Populate,
            );
            for f in [facts(true, false), facts(false, true)] {
                assert_eq!(
                    resets(Entry::Jump { hops }, &Rulebook::default(), f),
                    expected,
                    "{hops} {f:?}"
                );
            }
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
                TrafficStep::Reset,
            )
        );
    }

    #[test]
    fn a_landed_move_changes_where_the_pilot_is_and_defers_the_stellars() {
        let landed = Entry::ScriptMove { landed: true };
        assert_eq!(
            resets(landed, &arrival(RuleSource::Engine), facts(true, false)),
            Resets::of(
                &[Reset::Stellar, Reset::SaveDue, Reset::EmptyOffers],
                CourseStep::Keep,
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
                        Resets::of(&set, course, traffic),
                        "{reading:?} changed {changed} pending {pending}"
                    );
                }
            }
        }
    }

    #[test]
    fn only_a_moves_rows_read_move_arrival_from_the_rulebook() {
        let rows = |reading| {
            let rules = arrival(reading);
            [
                Entry::Jump { hops: 1 },
                Entry::Gate,
                Entry::Relocate,
                Entry::ScriptMove { landed: true },
                Entry::ScriptMove { landed: false },
            ]
            .map(|how| resets(how, &rules, facts(true, false)))
        };
        let (engine, bible) = (rows(RuleSource::Engine), rows(RuleSource::Bible));
        assert_eq!(engine[..3], bible[..3], "a jump, a gate and a relocation");
        assert_ne!(engine[3], bible[3], "a landed move");
        assert_ne!(engine[4], bible[4], "a move in flight");
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
            (Entry::Jump { hops: 0 }, Entrance::Jump),
            (Entry::Jump { hops: 3 }, Entrance::Jump),
            (Entry::Gate, Entrance::Gate),
            (
                Entry::ScriptMove { landed: true },
                Entrance::ScriptMove { landed: true },
            ),
            (
                Entry::ScriptMove { landed: false },
                Entrance::ScriptMove { landed: false },
            ),
            (Entry::Relocate, Entrance::Relocate),
        ] {
            assert_eq!(Entrance::from(how), entrance, "{how:?}");
        }
    }

    #[test]
    fn every_way_in_but_a_landed_move_tells_the_scene_changed_and_a_relocation_tells_which() {
        use SessionEvent::{StellarChanged, SystemChanged};
        for changed in [false, true] {
            for (how, event) in [
                (Entry::Jump { hops: 1 }, Some(SystemChanged)),
                (Entry::Gate, Some(SystemChanged)),
                (Entry::ScriptMove { landed: false }, Some(SystemChanged)),
                (Entry::ScriptMove { landed: true }, None),
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
