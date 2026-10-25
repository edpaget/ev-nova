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
    Answer, FlightCommand, FlightEvent, FlightFrame, FloatingLinks, MapKey, StepInput, StepReport,
};
use nova_sim::galaxy_map::MapDrawing;
use nova_sim::map_borders::BorderSources;
use nova_sim::map_panel::MapPanel;
use nova_sim::map_search::SearchOutcome;
use nova_sim::navigation::NavMode;
use nova_sim::{MapMarks, MissionId, PlayerCommand, SavedNav, StellarId, StellarPick};

use super::{CommSource, FlightSim, MapSource, PortSource};

/// A scripted [`FlightSim`]: it shows the frame it was given, records
/// each command in the order sent, answers each from a queue
/// ([`Answer::Refused`] once the queue is empty), hands out the events
/// queued for it on [`FlightSim::drain`], and reports each step from a
/// queue (the default once the queue is empty), counting the steps. It
/// takes every gate but those it is told to refuse
/// ([`FakeFlight::refuse`]), answers [`MapSource`] from [`FakeMap`]
/// ([`FakeFlight::map_mut`]), and records the map's questions whose
/// arguments matter ([`FakeFlight::asked`]). One with nothing scripted
/// meets [`check_contract`].
#[derive(Clone, Debug, Default)]
pub struct FakeFlight {
    frame: FlightFrame,
    applied: Vec<FlightCommand>,
    answers: VecDeque<Answer>,
    events: Vec<FlightEvent>,
    reports: VecDeque<StepReport>,
    steps: usize,
    refused: Vec<PlayerCommand>,
    map: FakeMap,
    asked: RefCell<Vec<MapAsk>>,
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
    /// [`MapSource::text_nav`]'s answer.
    pub text_nav: Option<NavMode>,
    /// [`MapSource::floating_map_places`]'s answer.
    pub floating: FloatingLinks,
    /// The stellars [`MapSource::stellar_name`] names, with their names.
    pub stellar_names: Vec<(StellarId, String)>,
    /// The systems [`MapSource::has_explored`] says are explored.
    pub explored: Vec<SystemId>,
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

    /// Answers the next command not yet answered with `answer`.
    pub fn queue_answer(&mut self, answer: Answer) {
        self.answers.push_back(answer);
    }

    /// Hands out `event` on the next drain.
    pub fn queue_event(&mut self, event: FlightEvent) {
        self.events.push(event);
    }

    /// Reports `report` for the next step not yet reported.
    pub fn queue_report(&mut self, report: StepReport) {
        self.reports.push_back(report);
    }

    /// The commands applied, in the order sent.
    #[must_use]
    pub fn applied(&self) -> &[FlightCommand] {
        &self.applied
    }

    /// How many steps were taken.
    #[must_use]
    pub fn steps(&self) -> usize {
        self.steps
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

    fn step(&mut self, _input: &StepInput) -> StepReport {
        self.steps += 1;
        self.reports.pop_front().unwrap_or_default()
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

    fn text_nav(&self) -> Option<NavMode> {
        self.map.text_nav
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
///   refusal, and the frame after it equals the frame before.
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
        fake.queue_report(StepReport::default());
        assert_eq!(fake.steps(), 0);
        for _ in 0..3 {
            assert_eq!(fake.step(&StepInput::default()), StepReport::default());
        }
        assert_eq!(fake.steps(), 3);
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
        assert_eq!(fake.text_nav(), None);
        assert_eq!(fake.stellar_name(StellarId(128)), None);
        assert!(!fake.has_explored(SystemId(130)));
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
        map.text_nav = Some(NavMode::Off);
        map.floating = floating.clone();
        map.stellar_names = vec![(StellarId(128), "Earth".to_owned())];
        map.explored = vec![SystemId(130)];
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
        assert_eq!(fake.text_nav(), Some(NavMode::Off));
        assert_eq!(fake.floating_map_places(), floating);
        assert_eq!(fake.stellar_name(StellarId(128)).as_deref(), Some("Earth"));
        assert_eq!(fake.stellar_name(StellarId(129)), None);
        assert!(fake.has_explored(SystemId(130)));
        assert!(!fake.has_explored(SystemId(131)));
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
        }

        fn step(&mut self, input: &StepInput) -> StepReport {
            self.fake.step(input)
        }

        fn apply(&mut self, command: FlightCommand) -> Answer {
            match self.fault {
                Fault::AnswersTheRefusal => Answer::Stellar(StellarId(128)),
                Fault::RefusalChangesTheFrame => {
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
    fn a_fill_over_an_unlike_frame_changes_every_field() {
        let busy = FlightFrame {
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
            } = &frame;
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
