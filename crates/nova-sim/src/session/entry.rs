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

use std::collections::BTreeSet;

use super::Session;
use super::script_effects::ScriptEffectRules;
use crate::catalog::{
    DudeId, DudeRecord, FleetRecord, LandingSite, PersonRecord, StellarId, SystemId, SystemTraffic,
    TrafficCatalog,
};
use crate::chance::Chance;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::message::SimMessage;
use crate::rulebook::{RuleSource, Rulebook};
use crate::sound::SimSound;
use crate::traffic::Traffic;

/// How the player enters a system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Entry {
    /// A hyperspace jump's arrival, having made `hops` hops along the
    /// course.
    #[cfg_attr(not(test), expect(dead_code))]
    Jump { hops: usize },
    /// Out of a hypergate or wormhole.
    #[cfg_attr(not(test), expect(dead_code))]
    Gate,
    /// `M` or `N`, landed or in flight.
    ScriptMove { landed: bool },
    /// The developer tools' move, landed.
    Relocate,
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
    #[cfg_attr(not(test), expect(dead_code))]
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
    /// The stellars a landed move left for the take-off are dropped.
    DeferredSites,
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
    /// The bar's hire rolls and the shops' rolls are drawn afresh, and
    /// the outfitter's opening starts anew, as a landing does.
    ShopRolls,
    /// A save is due.
    SaveDue,
    /// [`SimSound::Arrived`] sounds.
    ArrivalSound,
    /// [`SimMessage::Arrived`] is raised.
    ArrivalMessage,
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

/// When the stellars read become the system's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SitesStep {
    /// At once.
    Now,
    /// At the next take-off: the open spaceport keeps serving the stellar
    /// landed on until then.
    Defer,
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
    sites: SitesStep,
    traffic: TrafficStep,
}

impl Resets {
    fn of(set: &[Reset], course: CourseStep, sites: SitesStep, traffic: TrafficStep) -> Self {
        Self {
            set: set.iter().copied().collect(),
            course,
            sites,
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
/// (@0x6c4ff). It never clears the course, only drops the hops made; the
/// jump's start already stopped the thrust, and every tick ends a gate's
/// pending entry. Populating lets go of the strikes, the boarding, the
/// hail and the target too.
fn jump(hops: usize) -> Resets {
    Resets::of(
        &[
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
        ],
        CourseStep::Drop(hops),
        SitesStep::Now,
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
        ],
        CourseStep::Clear,
        SitesStep::Now,
        TrafficStep::Populate,
    )
}

/// A landed `M` or `N` (`_EvalSetExp` @0x1570e-0x158d4 and
/// @0x158d9-0x159ad): landed, the original fills `_dockedPortMissions`
/// and nothing more, so only the pilot's system and stellar change and
/// the stellars wait for the take-off. As an `arrival` (the Bible's
/// reading of `MoveArrival`) the system is explored and the course
/// cleared at once.
fn script_move_landed(arrival: bool) -> Resets {
    let mut set = vec![Reset::Stellar, Reset::SaveDue];
    let course = if arrival {
        set.push(Reset::Explore);
        CourseStep::Clear
    } else {
        CourseStep::Keep
    };
    Resets::of(&set, course, SitesStep::Defer, TrafficStep::None)
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
    Resets::of(&set, course, SitesStep::Now, traffic)
}

/// The developer tools' move of a landed pilot: no original does it. It
/// docks the ship as a landing does, the shops' rolls drawn afresh, and
/// leaves the last system's traffic and scene behind; it keeps a held
/// sound and the fighters out, and sounds nothing.
fn relocate() -> Resets {
    Resets::of(
        &[
            Reset::Stellar,
            Reset::Explore,
            Reset::Touchdown,
            Reset::Dock,
            Reset::DeferredSites,
            Reset::NavTarget,
            Reset::GateEntry,
            Reset::Combat,
            Reset::Strikes,
            Reset::Target,
            Reset::Boarding,
            Reset::Hail,
            Reset::ShopRolls,
            Reset::SaveDue,
        ],
        CourseStep::Clear,
        SitesStep::Now,
        TrafficStep::Reset,
    )
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
    /// Enters `arrival`'s system `how`: places the ship as the caller
    /// chose, makes the arrival writes, and resets what the row of
    /// [`resets`] for `how` says, populating from `catalog` on `chance`
    /// where it says so (see the module docs).
    pub(super) fn enter_system<C, R>(
        &mut self,
        how: Entry,
        arrival: Arrival,
        catalog: &C,
        chance: &mut R,
    ) where
        C: TrafficCatalog + ?Sized,
        R: Chance + ?Sized,
    {
        let Arrival {
            system,
            sites,
            placement,
        } = arrival;
        let facts = EntryFacts {
            system_changed: system != self.pilot.system,
            jump_pending: self.jump.is_some(),
        };
        let resets = resets(how, &self.rules, facts);
        self.place(placement);
        self.pilot.system = system;
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
        match resets.sites {
            SitesStep::Now => self.sites = sites,
            SitesStep::Defer => self.next_sites = Some(sites),
        }
        if resets.has(Reset::DeferredSites) {
            self.next_sites = None;
        }
        self.leave_behind(&resets);
        self.bring_in_traffic(resets.traffic, catalog, chance);
        if resets.has(Reset::ShopRolls) {
            self.hire_rolls.clear();
            self.outfit_rolls.clear();
            self.open_opening();
            self.ship_rolls.clear();
            self.ship_redraws.clear();
        }
        if resets.has(Reset::SaveDue) {
            self.save_due = true;
        }
        self.announce(&resets, system);
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

    /// Does `step` to the traffic, reading `catalog` and drawing on
    /// `chance` to populate: the only draw entering a system makes.
    fn bring_in_traffic<C, R>(&mut self, step: TrafficStep, catalog: &C, chance: &mut R)
    where
        C: TrafficCatalog + ?Sized,
        R: Chance + ?Sized,
    {
        match step {
            TrafficStep::None => {}
            TrafficStep::Populate => self.populate(catalog, chance),
            TrafficStep::Retarget => {
                let table = self.spawn_table(catalog);
                let fleet: BTreeSet<_> = self.fleet.iter().flatten().copied().collect();
                self.traffic.retarget(table, &fleet);
            }
            TrafficStep::Reset => {
                self.traffic = Traffic::new();
                self.traffic_due = true;
                self.fleet.clear();
            }
        }
    }

    /// Sounds and raises the arrival in `system`, as `resets` says.
    fn announce(&mut self, resets: &Resets, system: SystemId) {
        if resets.has(Reset::ArrivalSound) {
            self.sounds.push(SimSound::Arrived);
        }
        if resets.has(Reset::ArrivalMessage) {
            self.messages.push(SimMessage::Arrived(system));
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
    /// pending entry aside.
    const SCENE: [Reset; 6] = [
        Reset::NavTarget,
        Reset::Combat,
        Reset::Strikes,
        Reset::Target,
        Reset::Boarding,
        Reset::Hail,
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
                    ],
                ),
                CourseStep::Drop(hops),
                SitesStep::Now,
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
            SitesStep::Now,
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
                        Reset::DeferredSites,
                        Reset::GateEntry,
                        Reset::ShopRolls,
                        Reset::SaveDue,
                    ],
                ),
                CourseStep::Clear,
                SitesStep::Now,
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
                &[Reset::Stellar, Reset::SaveDue],
                CourseStep::Keep,
                SitesStep::Defer,
                TrafficStep::None,
            ),
            "by the engine"
        );
        assert_eq!(
            resets(landed, &arrival(RuleSource::Bible), facts(true, false)),
            Resets::of(
                &[Reset::Stellar, Reset::Explore, Reset::SaveDue],
                CourseStep::Clear,
                SitesStep::Defer,
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
                        Resets::of(&set, course, SitesStep::Now, traffic),
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
