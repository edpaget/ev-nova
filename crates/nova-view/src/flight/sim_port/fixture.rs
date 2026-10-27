//! Test helpers for the [`FlightSim`] port: [`FakeFlight`], a scripted
//! sim that records what it's asked, and [`check_contract`], what every
//! sim must honour.
//!
//! Available to this crate's tests and, through the `fixture` feature, to
//! other crates' tests. A component test of the view runs on a
//! [`FakeFlight`]; `tests/it/flight_seam.rs` runs [`check_contract`] on a
//! real [`SessionFlight`](crate::flight::SessionFlight).

use std::cell::RefCell;
use std::collections::VecDeque;

use nova_sim::JumpReadiness;
use nova_sim::catalog::SystemId;
use nova_sim::flight_frame::{
    Answer, DrawnBoxes, FlightCommand, FlightEvent, FlightFrame, FloatingLinks, FrameAsteroid,
    FrameBeam, FrameBox, FrameShip, MapKey, StepInput, StepReport, StepRun,
};
use nova_sim::galaxy_map::MapDrawing;
use nova_sim::map_borders::BorderSources;
use nova_sim::map_panel::MapPanel;
use nova_sim::map_search::SearchOutcome;
use nova_sim::navigation::NavMode;
use nova_sim::story::OverlaidGalaxy;
use nova_sim::{
    Hover, MapMarks, MissionId, PlayerCommand, SavedNav, ShipId, ShipRef, StellarId, StellarPick,
    Vec2,
};

use super::{CommSource, FlightSim, MapSource, PortSource};

/// A scripted [`FlightSim`]: it shows the frame it was given, records
/// each command in the order sent, answers each from a queue
/// ([`Answer::Refused`] once the queue is empty), hands out the events
/// queued for it on [`FlightSim::drain`], and reports each frame's steps
/// from a queue (no step run once the queue is empty), recording each
/// call's input and step count. It
/// takes every gate but those it is told to refuse
/// ([`FakeFlight::refuse`]), answers [`MapSource`] from [`FakeMap`]
/// ([`FakeFlight::map_mut`]), and records the map's questions whose
/// arguments matter ([`FakeFlight::asked`]). It answers the hover as told
/// ([`FakeFlight::set_hover`]), recording each ask
/// ([`FakeFlight::hovers`]), and the coming traffic from its list
/// ([`FakeFlight::set_coming_traffic`]). One with nothing scripted meets
/// [`check_contract`].
#[derive(Clone, Debug, Default)]
pub struct FakeFlight {
    frame: FlightFrame,
    applied: Vec<FlightCommand>,
    answers: VecDeque<Answer>,
    events: Vec<FlightEvent>,
    reports: VecDeque<StepReport>,
    stepped: Vec<(StepInput, u32)>,
    refused: Vec<PlayerCommand>,
    map: FakeMap,
    asked: RefCell<Vec<MapAsk>>,
    hover: Option<Hover>,
    hovers: RefCell<Vec<(Vec2, DrawnBoxes)>>,
    coming: Vec<ShipId>,
}

/// What a [`FakeFlight`] answers the [`MapSource`] questions with, each
/// field the answer to the question of its name. The default answers
/// nothing, as a [`MapSource`]'s defaults do.
#[derive(Clone, Debug, Default)]
pub struct FakeMap {
    /// [`MapSource::map_drawing`]'s answer, whatever it is asked.
    pub drawing: Option<MapDrawing>,
    /// [`MapSource::map_panel`]'s answer, whatever it is asked.
    pub panel: Option<MapPanel>,
    /// [`MapSource::map_border_sources`]'s answer.
    pub border_sources: Option<BorderSources>,
    /// [`MapSource::find_map_system`]'s answer, whatever the query.
    pub search: Option<SearchOutcome>,
    /// [`MapSource::map_mission_systems`]'s answer.
    pub mission_systems: Vec<SystemId>,
    /// [`MapSource::map_preselection`]'s answer.
    pub map_preselection: Option<SystemId>,
    /// [`MapSource::floating_preselection`]'s answer.
    pub floating_preselection: Option<SystemId>,
    /// [`MapSource::hypergate_cycle`]'s answer.
    pub hypergate_cycle: Vec<Option<SystemId>>,
    /// The marks each of [`MapSource::offer_map_marks`],
    /// [`MapSource::computer_map_marks`] and
    /// [`MapSource::mission_info_map_marks`] answers, whatever it is
    /// asked; [`MapMarks::NONE`] when `None`.
    pub marks: Option<MapMarks>,
    /// [`MapSource::map_nav`]'s answer.
    pub map_nav: Option<SavedNav>,
    /// [`MapSource::floating_map_places`]'s answer.
    pub floating: FloatingLinks,
    /// The stellars [`MapSource::stellar_name`] names, with their names.
    pub stellar_names: Vec<(StellarId, String)>,
    /// The systems [`MapSource::has_explored`] says are explored.
    pub explored: Vec<SystemId>,
    /// [`MapSource::galaxy`]'s answer.
    pub galaxy: Option<OverlaidGalaxy>,
}

/// A [`MapSource`] question a [`FakeFlight`] was asked whose arguments a
/// test may check, in the order asked ([`FakeFlight::asked`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapAsk {
    /// [`MapSource::map_drawing`], with its arguments.
    Drawing {
        /// The hypergate map's offers, or `None` on the course map.
        hypergate: Option<Vec<SystemId>>,
        /// The mission systems.
        missions: Vec<SystemId>,
        /// The opener's marks.
        marks: MapMarks,
    },
    /// [`MapSource::map_panel`], with its arguments.
    Panel {
        /// The system selected.
        selected: Option<SystemId>,
        /// The hypergate map's offers, or `None` on the course map.
        hypergate: Option<Vec<SystemId>>,
    },
    /// [`MapSource::find_map_system`], with its query.
    Find(String),
    /// [`MapSource::offer_map_marks`], for its mission.
    OfferMarks(MissionId),
    /// [`MapSource::computer_map_marks`], for its row.
    ComputerMarks(Option<MissionId>),
    /// [`MapSource::mission_info_map_marks`], for its slot.
    MissionInfoMarks(Option<u8>),
}

impl FakeFlight {
    /// A sim with nothing scripted: the default frame, every command
    /// refused, nothing to drain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows `frame` from now on.
    pub fn set_frame(&mut self, frame: FlightFrame) {
        self.frame = frame;
    }

    /// The frame it shows, to change in place.
    pub fn frame_mut(&mut self) -> &mut FlightFrame {
        &mut self.frame
    }

    /// Answers every hover ask with `hover` from now on.
    pub fn set_hover(&mut self, hover: Option<Hover>) {
        self.hover = hover;
    }

    /// Each hover ask's point and boxes, in the order asked.
    #[must_use]
    pub fn hovers(&self) -> Vec<(Vec2, DrawnBoxes)> {
        self.hovers.borrow().clone()
    }

    /// Answers the coming traffic with `ships` from now on.
    pub fn set_coming_traffic(&mut self, ships: Vec<ShipId>) {
        self.coming = ships;
    }

    /// Answers the next command not yet answered with `answer`.
    pub fn queue_answer(&mut self, answer: Answer) {
        self.answers.push_back(answer);
    }

    /// Hands out `event` on the next drain.
    pub fn queue_event(&mut self, event: FlightEvent) {
        self.events.push(event);
    }

    /// Reports `report` for the next call of [`FlightSim::step`] not yet
    /// reported, whatever its step count.
    pub fn queue_report(&mut self, report: StepReport) {
        self.reports.push_back(report);
    }

    /// The commands applied, in the order sent.
    #[must_use]
    pub fn applied(&self) -> &[FlightCommand] {
        &self.applied
    }

    /// How many times [`FlightSim::step`] was called: once a frame.
    #[must_use]
    pub fn steps(&self) -> usize {
        self.stepped.len()
    }

    /// How many steps the calls of [`FlightSim::step`] asked for, all
    /// told.
    #[must_use]
    pub fn steps_asked(&self) -> u32 {
        self.stepped.iter().map(|&(_, steps)| steps).sum()
    }

    /// Each call's input to [`FlightSim::step`], in order.
    #[must_use]
    pub fn inputs(&self) -> Vec<StepInput> {
        self.stepped.iter().map(|&(input, _)| input).collect()
    }

    /// Refuses the gate `gate` from now on ([`FlightSim::takes`]).
    pub fn refuse(&mut self, gate: PlayerCommand) {
        self.refused.push(gate);
    }

    /// Takes the gate `gate` again ([`FlightSim::takes`]).
    pub fn take(&mut self, gate: PlayerCommand) {
        self.refused.retain(|&refused| refused != gate);
    }

    /// What the map's questions are answered with, to script.
    pub fn map_mut(&mut self) -> &mut FakeMap {
        &mut self.map
    }

    /// The map's questions asked whose arguments matter, in the order
    /// asked ([`MapAsk`]).
    #[must_use]
    pub fn asked(&self) -> Vec<MapAsk> {
        self.asked.borrow().clone()
    }

    /// Notes `ask`.
    fn ask(&self, ask: MapAsk) {
        self.asked.borrow_mut().push(ask);
    }
}

impl FlightSim for FakeFlight {
    fn frame(&self, into: &mut FlightFrame) {
        into.clone_from(&self.frame);
    }

    fn step(&mut self, input: &StepInput, steps: u32, into: &mut StepReport) {
        self.stepped.push((*input, steps));
        match self.reports.pop_front() {
            Some(report) => into.clone_from(&report),
            None => into.clear(),
        }
    }

    fn apply(&mut self, command: FlightCommand) -> Answer {
        self.applied.push(command);
        self.answers.pop_front().unwrap_or(Answer::Refused)
    }

    fn drain(&mut self, into: &mut Vec<FlightEvent>) {
        into.append(&mut self.events);
    }

    fn takes(&self, gate: PlayerCommand) -> bool {
        !self.refused.contains(&gate)
    }

    fn hover(&self, at: Vec2, drawn: &DrawnBoxes) -> Option<Hover> {
        self.hovers.borrow_mut().push((at, drawn.clone()));
        self.hover
    }

    fn coming_traffic(&self) -> Vec<ShipId> {
        self.coming.clone()
    }
}

impl MapSource for FakeFlight {
    fn map_drawing(
        &self,
        hypergate: Option<&[SystemId]>,
        missions: &[SystemId],
        marks: MapMarks,
    ) -> Option<MapDrawing> {
        self.ask(MapAsk::Drawing {
            hypergate: hypergate.map(<[SystemId]>::to_vec),
            missions: missions.to_vec(),
            marks,
        });
        self.map.drawing.clone()
    }

    fn map_panel(
        &self,
        selected: Option<SystemId>,
        hypergate: Option<&[SystemId]>,
    ) -> Option<MapPanel> {
        self.ask(MapAsk::Panel {
            selected,
            hypergate: hypergate.map(<[SystemId]>::to_vec),
        });
        self.map.panel.clone()
    }

    fn map_border_sources(&self) -> Option<BorderSources> {
        self.map.border_sources.clone()
    }

    fn find_map_system(&self, query: &str) -> Option<SearchOutcome> {
        self.ask(MapAsk::Find(query.to_owned()));
        self.map.search
    }

    fn map_mission_systems(&self) -> Vec<SystemId> {
        self.map.mission_systems.clone()
    }

    fn map_preselection(&self) -> Option<SystemId> {
        self.map.map_preselection
    }

    fn floating_preselection(&self) -> Option<SystemId> {
        self.map.floating_preselection
    }

    fn hypergate_cycle(&self) -> Vec<Option<SystemId>> {
        self.map.hypergate_cycle.clone()
    }

    fn offer_map_marks(&self, mission: MissionId) -> MapMarks {
        self.ask(MapAsk::OfferMarks(mission));
        self.map.marks.unwrap_or(MapMarks::NONE)
    }

    fn computer_map_marks(&self, selected: Option<MissionId>) -> MapMarks {
        self.ask(MapAsk::ComputerMarks(selected));
        self.map.marks.unwrap_or(MapMarks::NONE)
    }

    fn mission_info_map_marks(&self, slot: Option<u8>) -> MapMarks {
        self.ask(MapAsk::MissionInfoMarks(slot));
        self.map.marks.unwrap_or(MapMarks::NONE)
    }

    fn map_nav(&self) -> Option<SavedNav> {
        self.map.map_nav
    }

    fn floating_map_places(&self) -> FloatingLinks {
        self.map.floating.clone()
    }

    fn stellar_name(&self, stellar: StellarId) -> Option<String> {
        self.map
            .stellar_names
            .iter()
            .find(|(id, _)| *id == stellar)
            .map(|(_, name)| name.clone())
    }

    fn has_explored(&self, system: SystemId) -> bool {
        self.map.explored.contains(&system)
    }

    fn galaxy(&self) -> Option<&OverlaidGalaxy> {
        self.map.galaxy.as_ref()
    }
}

impl CommSource for FakeFlight {}

impl PortSource for FakeFlight {}

/// A command every sim refuses: a stellar key for a slot no system has.
#[must_use]
pub fn refused_command() -> FlightCommand {
    FlightCommand::SelectStellar(StellarPick::Slot(usize::MAX))
}

/// A frame unlike `frame` in every field, to fill over: a fill that
/// leaves a field unwritten then shows in the comparison. The
/// destructuring names every field, so a field added to [`FlightFrame`]
/// doesn't compile until it is changed here.
pub(crate) fn unlike(frame: &FlightFrame) -> FlightFrame {
    let FlightFrame {
        nav_mode,
        system,
        course_shown,
        course_plotted,
        docked,
        jump_under_way,
        jump_readiness,
        texts_queued,
        map_key,
        ships,
        drawn,
        shots,
        beams,
        cargo_boxes,
        asteroids,
        stellars,
        effective_murk,
        system_murk,
        system_background,
        double_time,
        hyper_gamma,
        traffic_ships,
    } = frame;
    let other_system = SystemId(if system.0 == i16::MAX { 0 } else { i16::MAX });
    FlightFrame {
        nav_mode: unlike_nav(*nav_mode),
        system: other_system,
        course_shown: if course_shown.is_empty() {
            vec![other_system]
        } else {
            Vec::new()
        },
        course_plotted: !course_plotted,
        docked: !docked,
        jump_under_way: !jump_under_way,
        jump_readiness: if *jump_readiness == JumpReadiness::Blocked {
            JumpReadiness::Clear
        } else {
            JumpReadiness::Blocked
        },
        texts_queued: texts_queued.wrapping_add(1),
        map_key: MapKey {
            system: if map_key.system == other_system {
                SystemId(other_system.0 - 1)
            } else {
                other_system
            },
            nav: unlike_nav(map_key.nav),
            ..map_key.clone()
        },
        ships: one_or_none(ships, FrameShip::default),
        drawn: one_or_none(drawn, || (ShipRef::Player, nova_sim::ShipLayer::Ships)),
        shots: one_or_none(shots, record::shot),
        beams: one_or_none(beams, || FrameBeam {
            firer: ShipRef::Player,
            start: Vec2::ZERO,
            end: Vec2::ZERO,
            weapon: nova_sim::WeaponId(128),
            fade: 0,
        }),
        cargo_boxes: one_or_none(cargo_boxes, || FrameBox {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            frame: 0,
            fade: 0,
            graphic: 0,
        }),
        asteroids: one_or_none(asteroids, || FrameAsteroid {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            frame: 0.0,
            kind: 0,
        }),
        stellars: one_or_none(stellars, || record::stellar(StellarId(128))),
        effective_murk: effective_murk.wrapping_add(1),
        system_murk: system_murk.wrapping_add(1),
        system_background: if system_background.is_some() {
            None
        } else {
            Some(0)
        },
        double_time: !double_time,
        hyper_gamma: hyper_gamma + 1.0,
        traffic_ships: one_or_none(traffic_ships, || ShipId(128)),
    }
}

/// None when `records` holds any, else one of `record`'s.
fn one_or_none<T>(records: &[T], record: impl FnOnce() -> T) -> Vec<T> {
    if records.is_empty() {
        vec![record()]
    } else {
        Vec::new()
    }
}

/// Small record builders for frames a test scripts: a ship, a shot, a
/// stellar.
pub mod record {
    use nova_sim::combat::projectile::ShotId;
    use nova_sim::flight_frame::{FrameShip, FrameShot, FrameStellar};
    use nova_sim::{DrawnSheet, NpcId, ShipId, ShipRef, StellarId, Vec2, WeaponId};

    /// The player's ship, of class `class`, at `at` facing `heading`,
    /// drawn in full and boxed by its own frames.
    #[must_use]
    pub fn player(class: i16, at: Vec2, heading: f32) -> FrameShip {
        FrameShip {
            class: ShipId(class),
            position: at,
            heading,
            framed: Some(ShipId(class)),
            ..FrameShip::default()
        }
    }

    /// NPC `id`, of class `class`, at `at` facing `heading`, drawn in
    /// full, boxed by its own frames, on the radar.
    #[must_use]
    pub fn npc(id: u32, class: i16, at: Vec2, heading: f32) -> FrameShip {
        FrameShip {
            ship: ShipRef::Npc(NpcId(id)),
            class: ShipId(class),
            position: at,
            heading,
            framed: Some(ShipId(class)),
            on_radar: true,
            ..FrameShip::default()
        }
    }

    /// A shot of weapon 128 at the system's centre, at rest.
    #[must_use]
    pub fn shot() -> FrameShot {
        FrameShot {
            id: ShotId(0),
            weapon: WeaponId(128),
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            heading: 0.0,
            age: 0,
            spin_timer: 0,
            layer: nova_sim::combat::shot_layer::ShotLayer::Shots,
        }
    }

    /// Stellar `id`, shown, whole, on its `Type` sheet, no hypergate.
    #[must_use]
    pub fn stellar(id: StellarId) -> FrameStellar {
        FrameStellar {
            id,
            shown: true,
            destroyed: false,
            drawn: DrawnSheet::Type,
            counted: DrawnSheet::Type,
            frozen: false,
            counts_none: false,
            gate_frame: None,
        }
    }
}

/// A nav mode other than `nav`.
fn unlike_nav(nav: NavMode) -> NavMode {
    if nav == NavMode::Off {
        NavMode::Hyperspace(Some(SystemId(i16::MAX)))
    } else {
        NavMode::Off
    }
}

/// Checks the port's contract against `sim`:
///
/// - **drain empties**: after one drain, a second adds nothing;
/// - **a frame fills the same twice**: two fills with no step between,
///   the second over a frame unlike the first in every field, give equal
///   frames, so a fill that skips a field fails;
/// - **a refusal changes nothing**: [`refused_command`] answers a
///   refusal, and the frame after it equals the frame before;
/// - **no step asked, none run**: a frame's steps asked for none fill the
///   report, over one holding a step, with no step and no stop;
/// - **the player's record first, once**: a frame's ships begin with the
///   player's, and hold no other;
/// - **every ship drawn has a record**: each of a frame's ships drawn is
///   among its ships;
/// - **no ship hovered without boxes**: a hover with no ship's box finds
///   no ship and not the player's;
/// - **a click on nothing is refused**: a click in space far from
///   anything, with no boxes, is refused, and the frame after it equals
///   the frame before.
///
/// The error says which clause failed.
pub fn check_contract(sim: &mut impl FlightSim) -> Result<(), String> {
    let mut drained = Vec::new();
    sim.drain(&mut drained);
    let first = drained.len();
    sim.drain(&mut drained);
    if drained.len() != first {
        return Err(format!(
            "drain empties: a second drain added {:?}",
            &drained[first..]
        ));
    }

    let mut once = FlightFrame::default();
    sim.frame(&mut once);
    let mut twice = unlike(&once);
    sim.frame(&mut twice);
    if twice != once {
        return Err(format!(
            "a frame fills the same twice: {once:?}, then {twice:?}"
        ));
    }

    let answer = sim.apply(refused_command());
    if !answer.is_refused() {
        return Err(format!(
            "a refusal changes nothing: {:?} answered {answer:?}",
            refused_command()
        ));
    }
    let mut after = unlike(&once);
    sim.frame(&mut after);
    if after != once {
        return Err(format!(
            "a refusal changes nothing: the frame was {once:?}, then {after:?}"
        ));
    }

    let mut report = StepReport {
        steps: vec![StepRun::default()],
        stopped: true,
    };
    sim.step(&StepInput::default(), 0, &mut report);
    if report != StepReport::default() {
        return Err(format!(
            "no step asked, none run: the report was {report:?}"
        ));
    }
    check_world(sim, &once)
}

/// [`check_contract`]'s clauses on the world: the ships' records and
/// those drawn, the hover and a click in space, `once` the frame filled
/// before.
fn check_world(sim: &mut impl FlightSim, once: &FlightFrame) -> Result<(), String> {
    let players = once
        .ships
        .iter()
        .filter(|record| record.ship == ShipRef::Player)
        .count();
    let first = once.ships.first().map(|record| record.ship);
    if players != 1 || first != Some(ShipRef::Player) {
        return Err(format!(
            "the player's record first, once: the ships were {:?}",
            once.ships
                .iter()
                .map(|record| record.ship)
                .collect::<Vec<_>>()
        ));
    }
    if let Some((stray, _)) = once
        .drawn
        .iter()
        .find(|(ship, _)| once.ship(*ship).is_none())
    {
        return Err(format!(
            "every ship drawn has a record: {stray:?} is drawn with none"
        ));
    }
    let at = once.player().position;
    if let Some(hover @ (Hover::Ship(_) | Hover::Player)) = sim.hover(at, &DrawnBoxes::default()) {
        return Err(format!(
            "no ship hovered without boxes: {hover:?} at {at:?}"
        ));
    }
    let far = Vec2::new(1.0e9, 1.0e9);
    let click = FlightCommand::ClickInSpace {
        at: far,
        drawn: DrawnBoxes::default(),
    };
    let answer = sim.apply(click);
    if !answer.is_refused() {
        return Err(format!(
            "a click on nothing is refused: a click at {far:?} answered {answer:?}"
        ));
    }
    let mut after = unlike(once);
    sim.frame(&mut after);
    if after != *once {
        return Err(format!(
            "a click on nothing is refused: the frame was {once:?}, then {after:?}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use nova_sim::SimSound;

    use super::*;

    fn stellar(id: i16) -> FlightFrame {
        FlightFrame {
            nav_mode: NavMode::Stellar(Some(StellarId(id))),
            ..FlightFrame::default()
        }
    }

    #[test]
    fn a_fake_with_nothing_scripted_meets_the_contract() {
        assert_eq!(check_contract(&mut FakeFlight::new()), Ok(()));
    }

    #[test]
    fn a_fake_showing_a_frame_meets_the_contract() {
        let mut fake = FakeFlight::new();
        fake.set_frame(stellar(128));
        fake.queue_event(FlightEvent::Sound(SimSound::NavSelected));
        assert_eq!(check_contract(&mut fake), Ok(()));
    }

    #[test]
    fn the_fake_shows_the_frame_it_was_given() {
        let mut fake = FakeFlight::new();
        fake.set_frame(stellar(128));
        let mut frame = FlightFrame::default();
        fake.frame(&mut frame);
        assert_eq!(frame, stellar(128));
    }

    #[test]
    fn the_fake_records_commands_in_order_and_answers_from_its_queue() {
        let mut fake = FakeFlight::new();
        fake.queue_answer(Answer::Stellar(StellarId(130)));
        fake.queue_answer(Answer::Stellar(StellarId(129)));
        let commands = [
            FlightCommand::SelectStellar(StellarPick::Slot(1)),
            FlightCommand::SelectStellar(StellarPick::Nearest),
            FlightCommand::SelectStellar(StellarPick::Slot(0)),
        ];
        let answers: Vec<_> = commands
            .iter()
            .map(|command| fake.apply(command.clone()))
            .collect();
        assert_eq!(
            answers,
            [
                Answer::Stellar(StellarId(130)),
                Answer::Stellar(StellarId(129)),
                Answer::Refused,
            ]
        );
        assert_eq!(fake.applied(), commands);
    }

    #[test]
    fn the_fake_hands_out_its_events_once_after_what_was_drained() {
        let mut fake = FakeFlight::new();
        let sounds = [SimSound::NavSelected, SimSound::HyperSelected];
        for sound in sounds {
            fake.queue_event(FlightEvent::Sound(sound));
        }
        let kept = FlightEvent::Sound(SimSound::NavSelected);
        let mut drained = vec![kept.clone()];
        fake.drain(&mut drained);
        assert_eq!(
            drained,
            [
                kept,
                FlightEvent::Sound(SimSound::NavSelected),
                FlightEvent::Sound(SimSound::HyperSelected),
            ]
        );
        let mut again = Vec::new();
        fake.drain(&mut again);
        assert_eq!(again, []);
    }

    #[test]
    fn the_fake_reports_steps_from_its_queue_and_counts_them() {
        let mut fake = FakeFlight::new();
        let scripted = StepReport {
            steps: vec![StepRun::default(); 2],
            stopped: true,
        };
        fake.queue_report(scripted.clone());
        assert_eq!(fake.steps(), 0);
        let held = StepInput {
            cloak: true,
            ..StepInput::default()
        };
        let mut report = StepReport::default();
        fake.step(&held, 2, &mut report);
        assert_eq!(report, scripted, "copied into the caller's report");
        for steps in [3, 0] {
            fake.step(&StepInput::default(), steps, &mut report);
            assert_eq!(report, StepReport::default(), "none run once empty");
        }
        assert_eq!(fake.steps(), 3, "the calls");
        assert_eq!(fake.steps_asked(), 5, "the steps asked");
        assert_eq!(
            fake.inputs(),
            [held, StepInput::default(), StepInput::default()]
        );
    }

    #[test]
    fn the_fake_takes_every_gate_but_those_refused() {
        let mut fake = FakeFlight::new();
        assert!(fake.takes(PlayerCommand::Nav));
        fake.refuse(PlayerCommand::Nav);
        assert!(!fake.takes(PlayerCommand::Nav));
        assert!(fake.takes(PlayerCommand::SelectSystem));
        fake.take(PlayerCommand::Nav);
        assert!(fake.takes(PlayerCommand::Nav));
    }

    #[test]
    fn the_fake_answers_nothing_on_the_map_until_scripted() {
        let fake = FakeFlight::new();
        assert_eq!(fake.map_drawing(None, &[], MapMarks::NONE), None);
        assert_eq!(fake.map_panel(None, None), None);
        assert_eq!(fake.map_border_sources(), None);
        assert_eq!(fake.find_map_system("sol"), None);
        assert_eq!(fake.offer_map_marks(MissionId(128)), MapMarks::NONE);
        assert_eq!(fake.map_nav(), None);
        assert_eq!(fake.stellar_name(StellarId(128)), None);
        assert!(!fake.has_explored(SystemId(130)));
        assert_eq!(fake.galaxy(), None);
    }

    #[test]
    fn the_fake_answers_the_map_as_scripted() {
        let mut fake = FakeFlight::new();
        let marks = MapMarks {
            green: Some(SystemId(131)),
            multi_arrows: false,
        };
        let saved = SavedNav::of(NavMode::Off);
        let floating = FloatingLinks {
            current: nova_sim::Vec2::new(1.0, 2.0),
            links: vec![(SystemId(131), nova_sim::Vec2::new(3.0, 4.0))],
        };
        let map = fake.map_mut();
        map.search = Some(SearchOutcome::Found(SystemId(131)));
        map.border_sources = Some(BorderSources::default());
        map.mission_systems = vec![SystemId(132)];
        map.map_preselection = Some(SystemId(131));
        map.floating_preselection = Some(SystemId(132));
        map.hypergate_cycle = vec![None, Some(SystemId(131))];
        map.marks = Some(marks);
        map.map_nav = Some(saved);
        map.floating = floating.clone();
        map.stellar_names = vec![(StellarId(128), "Earth".to_owned())];
        map.explored = vec![SystemId(130)];
        let galaxy = OverlaidGalaxy::new(Vec::new(), nova_sim::story::GalaxyOverlay::default());
        map.galaxy = Some(galaxy.clone());
        assert_eq!(
            fake.find_map_system("x"),
            Some(SearchOutcome::Found(SystemId(131)))
        );
        assert_eq!(fake.map_border_sources(), Some(BorderSources::default()));
        assert_eq!(fake.map_mission_systems(), [SystemId(132)]);
        assert_eq!(fake.map_preselection(), Some(SystemId(131)));
        assert_eq!(fake.floating_preselection(), Some(SystemId(132)));
        assert_eq!(fake.hypergate_cycle(), [None, Some(SystemId(131))]);
        assert_eq!(fake.offer_map_marks(MissionId(1)), marks);
        assert_eq!(fake.computer_map_marks(None), marks);
        assert_eq!(fake.mission_info_map_marks(Some(0)), marks);
        assert_eq!(fake.map_nav(), Some(saved));
        assert_eq!(fake.floating_map_places(), floating);
        assert_eq!(fake.stellar_name(StellarId(128)).as_deref(), Some("Earth"));
        assert_eq!(fake.stellar_name(StellarId(129)), None);
        assert!(fake.has_explored(SystemId(130)));
        assert!(!fake.has_explored(SystemId(131)));
        assert_eq!(fake.galaxy(), Some(&galaxy));
    }

    #[test]
    fn the_fake_records_the_maps_questions_whose_arguments_matter() {
        let fake = FakeFlight::new();
        let marks = MapMarks {
            green: Some(SystemId(131)),
            multi_arrows: true,
        };
        fake.map_drawing(Some(&[SystemId(131)]), &[SystemId(132)], marks);
        fake.map_panel(Some(SystemId(130)), None);
        fake.find_map_system("alpha");
        fake.offer_map_marks(MissionId(128));
        fake.computer_map_marks(Some(MissionId(129)));
        fake.mission_info_map_marks(Some(2));
        fake.map_preselection();
        assert_eq!(
            fake.asked(),
            [
                MapAsk::Drawing {
                    hypergate: Some(vec![SystemId(131)]),
                    missions: vec![SystemId(132)],
                    marks,
                },
                MapAsk::Panel {
                    selected: Some(SystemId(130)),
                    hypergate: None,
                },
                MapAsk::Find("alpha".to_owned()),
                MapAsk::OfferMarks(MissionId(128)),
                MapAsk::ComputerMarks(Some(MissionId(129))),
                MapAsk::MissionInfoMarks(Some(2)),
            ]
        );
    }

    /// What a broken sim gets wrong.
    #[derive(Clone, Copy, Debug)]
    enum Fault {
        /// Every drain hands out another event.
        NeverEmpties,
        /// A fill leaves the nav mode as it was.
        SkipsAField,
        /// The refused command is answered.
        AnswersTheRefusal,
        /// The refused command changes the frame.
        RefusalChangesTheFrame,
        /// A frame asked for no step reports one.
        StepsUnasked,
        /// The frame's ships begin with an NPC's.
        NpcFirst,
        /// The frame draws a ship it has no record of.
        DrawsAStray,
        /// The hover finds the player's ship with no boxes.
        HoversWithoutBoxes,
        /// The click on nothing is answered.
        AnswersTheClick,
        /// The click on nothing changes the frame.
        ClickChangesTheFrame,
    }

    /// A [`FakeFlight`] with one [`Fault`].
    struct Broken {
        fake: FakeFlight,
        fault: Fault,
    }

    impl Broken {
        fn new(fault: Fault) -> Self {
            let mut fake = FakeFlight::new();
            fake.set_frame(stellar(128));
            Self { fake, fault }
        }
    }

    impl FlightSim for Broken {
        fn frame(&self, into: &mut FlightFrame) {
            if !matches!(self.fault, Fault::SkipsAField) {
                self.fake.frame(into);
            }
            let npc = ShipRef::Npc(nova_sim::NpcId(7));
            match self.fault {
                Fault::NpcFirst => into.ships.insert(0, record::npc(7, 129, Vec2::ZERO, 0.0)),
                Fault::DrawsAStray => into.drawn.push((npc, nova_sim::ShipLayer::Ships)),
                _ => {}
            }
        }

        fn hover(&self, at: Vec2, drawn: &DrawnBoxes) -> Option<Hover> {
            if matches!(self.fault, Fault::HoversWithoutBoxes) {
                return Some(Hover::Player);
            }
            self.fake.hover(at, drawn)
        }

        fn step(&mut self, input: &StepInput, steps: u32, into: &mut StepReport) {
            self.fake.step(input, steps, into);
            if matches!(self.fault, Fault::StepsUnasked) {
                into.steps.push(StepRun::default());
            }
        }

        fn apply(&mut self, command: FlightCommand) -> Answer {
            let click = matches!(command, FlightCommand::ClickInSpace { .. });
            match self.fault {
                Fault::AnswersTheRefusal => Answer::Stellar(StellarId(128)),
                Fault::RefusalChangesTheFrame => {
                    self.fake.set_frame(stellar(129));
                    self.fake.apply(command)
                }
                Fault::AnswersTheClick if click => Answer::InSpace(Hover::Player),
                Fault::ClickChangesTheFrame if click => {
                    self.fake.set_frame(stellar(129));
                    self.fake.apply(command)
                }
                _ => self.fake.apply(command),
            }
        }

        fn takes(&self, gate: PlayerCommand) -> bool {
            self.fake.takes(gate)
        }

        fn drain(&mut self, into: &mut Vec<FlightEvent>) {
            if matches!(self.fault, Fault::NeverEmpties) {
                self.fake
                    .queue_event(FlightEvent::Sound(SimSound::NavSelected));
            }
            self.fake.drain(into);
        }
    }

    impl MapSource for Broken {}

    impl CommSource for Broken {}

    impl PortSource for Broken {}

    /// The clause `fault` breaks, as `check_contract` names it.
    fn clause(fault: Fault) -> String {
        check_contract(&mut Broken::new(fault)).expect_err("broken")
    }

    #[test]
    fn a_drain_that_never_empties_breaks_the_contract() {
        assert!(clause(Fault::NeverEmpties).starts_with("drain empties"));
    }

    #[test]
    fn a_fill_that_skips_a_field_breaks_the_contract() {
        assert!(clause(Fault::SkipsAField).starts_with("a frame fills the same twice"));
    }

    #[test]
    fn an_answered_refusal_breaks_the_contract() {
        let clause = clause(Fault::AnswersTheRefusal);
        assert!(clause.starts_with("a refusal changes nothing"), "{clause}");
        assert!(clause.contains("answered"), "{clause}");
    }

    #[test]
    fn a_refusal_that_changes_the_frame_breaks_the_contract() {
        let clause = clause(Fault::RefusalChangesTheFrame);
        assert!(clause.starts_with("a refusal changes nothing"), "{clause}");
        assert!(clause.contains("the frame was"), "{clause}");
    }

    #[test]
    fn a_step_unasked_breaks_the_contract() {
        assert!(clause(Fault::StepsUnasked).starts_with("no step asked, none run"));
    }

    #[test]
    fn an_npc_first_breaks_the_contract() {
        assert!(clause(Fault::NpcFirst).starts_with("the player's record first, once"));
    }

    #[test]
    fn a_second_player_record_breaks_the_contract() {
        let mut fake = FakeFlight::new();
        fake.frame_mut().ships.push(FrameShip::default());
        let clause = check_contract(&mut fake).expect_err("broken");
        assert!(
            clause.starts_with("the player's record first, once"),
            "{clause}"
        );
    }

    #[test]
    fn a_ship_drawn_with_no_record_breaks_the_contract() {
        assert!(clause(Fault::DrawsAStray).starts_with("every ship drawn has a record"));
    }

    #[test]
    fn a_hover_without_boxes_breaks_the_contract() {
        assert!(clause(Fault::HoversWithoutBoxes).starts_with("no ship hovered without boxes"));
    }

    #[test]
    fn an_answered_click_on_nothing_breaks_the_contract() {
        let clause = clause(Fault::AnswersTheClick);
        assert!(
            clause.starts_with("a click on nothing is refused"),
            "{clause}"
        );
        assert!(clause.contains("answered"), "{clause}");
    }

    #[test]
    fn a_click_on_nothing_that_changes_the_frame_breaks_the_contract() {
        let clause = clause(Fault::ClickChangesTheFrame);
        assert!(
            clause.starts_with("a click on nothing is refused"),
            "{clause}"
        );
        assert!(clause.contains("the frame was"), "{clause}");
    }

    #[test]
    fn the_fake_answers_the_hover_as_told_and_records_each_ask() {
        let mut fake = FakeFlight::new();
        let boxes = DrawnBoxes {
            player: Some(nova_sim::DrawnBox {
                centre: Vec2::ZERO,
                width: 10,
                height: 10,
            }),
            npcs: Vec::new(),
        };
        assert_eq!(fake.hover(Vec2::new(1.0, 2.0), &boxes), None);
        fake.set_hover(Some(Hover::Stellar(StellarId(128))));
        assert_eq!(
            fake.hover(Vec2::ZERO, &DrawnBoxes::default()),
            Some(Hover::Stellar(StellarId(128)))
        );
        assert_eq!(
            fake.hovers(),
            [
                (Vec2::new(1.0, 2.0), boxes),
                (Vec2::ZERO, DrawnBoxes::default())
            ]
        );
    }

    #[test]
    fn the_fake_brings_the_coming_traffic_it_is_given() {
        let mut fake = FakeFlight::new();
        assert_eq!(fake.coming_traffic(), []);
        fake.set_coming_traffic(vec![ShipId(129)]);
        assert_eq!(fake.coming_traffic(), [ShipId(129)]);
    }

    #[test]
    fn a_fill_over_an_unlike_frame_changes_every_field() {
        let busy = FlightFrame {
            ships: Vec::new(),
            drawn: Vec::new(),
            shots: vec![record::shot()],
            stellars: vec![record::stellar(StellarId(130))],
            effective_murk: u8::MAX,
            system_background: Some(0x10),
            double_time: true,
            traffic_ships: vec![ShipId(130)],
            system: SystemId(i16::MAX),
            course_shown: vec![SystemId(131)],
            course_plotted: true,
            docked: true,
            jump_under_way: true,
            jump_readiness: JumpReadiness::Underway,
            texts_queued: u64::MAX,
            map_key: MapKey {
                system: SystemId(i16::MAX),
                nav: NavMode::Stellar(None),
                ..MapKey::default()
            },
            ..stellar(128)
        };
        for frame in [FlightFrame::default(), stellar(128), busy] {
            let other = unlike(&frame);
            let FlightFrame {
                nav_mode,
                system,
                course_shown,
                course_plotted,
                docked,
                jump_under_way,
                jump_readiness,
                texts_queued,
                map_key,
                ships,
                drawn,
                shots,
                beams,
                cargo_boxes,
                asteroids,
                stellars,
                effective_murk,
                system_murk,
                system_background,
                double_time,
                hyper_gamma,
                traffic_ships,
            } = &frame;
            assert_ne!(other.ships, *ships);
            assert_ne!(other.drawn, *drawn);
            assert_ne!(other.shots, *shots);
            assert_ne!(other.beams, *beams);
            assert_ne!(other.cargo_boxes, *cargo_boxes);
            assert_ne!(other.asteroids, *asteroids);
            assert_ne!(other.stellars, *stellars);
            assert_ne!(other.effective_murk, *effective_murk);
            assert_ne!(other.system_murk, *system_murk);
            assert_ne!(other.system_background, *system_background);
            assert_ne!(other.double_time, *double_time);
            assert_ne!(other.hyper_gamma.to_bits(), hyper_gamma.to_bits());
            assert_ne!(other.traffic_ships, *traffic_ships);
            assert_ne!(other.nav_mode, *nav_mode);
            assert_ne!(other.system, *system);
            assert_ne!(other.course_shown, *course_shown);
            assert_ne!(other.course_plotted, *course_plotted);
            assert_ne!(other.docked, *docked);
            assert_ne!(other.jump_under_way, *jump_under_way);
            assert_ne!(other.jump_readiness, *jump_readiness);
            assert_ne!(other.texts_queued, *texts_queued);
            assert_ne!(other.map_key, *map_key);
            assert_ne!(other.map_key.system, map_key.system);
            assert_ne!(other.map_key.nav, map_key.nav);
        }
    }

    #[test]
    fn an_unlike_maps_key_moves_off_the_system_unlike_takes() {
        // The key already on the system the frame's unlike moves to steps
        // one below it.
        for (system, key_system, moved) in [(0, i16::MAX, i16::MAX - 1), (i16::MAX, 0, -1)] {
            let frame = FlightFrame {
                system: SystemId(system),
                map_key: MapKey {
                    system: SystemId(key_system),
                    ..MapKey::default()
                },
                ..FlightFrame::default()
            };
            let other = unlike(&frame);
            assert_eq!(other.map_key.system, SystemId(moved));
            assert_ne!(other.map_key.system, frame.map_key.system);
        }
    }
}
