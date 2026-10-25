//! Test helpers for the [`FlightSim`] port: [`FakeFlight`], a scripted
//! sim that records what it's asked, and [`check_contract`], what every
//! sim must honour.
//!
//! Available to this crate's tests and, through the `fixture` feature, to
//! other crates' tests. A component test of the view runs on a
//! [`FakeFlight`]; `tests/it/flight_seam.rs` runs [`check_contract`] on a
//! real [`SessionFlight`](crate::flight::SessionFlight).

use std::collections::VecDeque;

use nova_sim::JumpReadiness;
use nova_sim::StellarPick;
use nova_sim::catalog::SystemId;
use nova_sim::flight_frame::{
    Answer, FlightCommand, FlightEvent, FlightFrame, MapKey, StepInput, StepReport,
};
use nova_sim::navigation::NavMode;

use super::{CommSource, FlightSim, MapSource, PortSource};

/// A scripted [`FlightSim`]: it shows the frame it was given, records
/// each command in the order sent, answers each from a queue
/// ([`Answer::Refused`] once the queue is empty), hands out the events
/// queued for it on [`FlightSim::drain`], and reports each step from a
/// queue (the default once the queue is empty), counting the steps. One
/// with nothing scripted meets [`check_contract`].
#[derive(Clone, Debug, Default)]
pub struct FakeFlight {
    frame: FlightFrame,
    applied: Vec<FlightCommand>,
    answers: VecDeque<Answer>,
    events: Vec<FlightEvent>,
    reports: VecDeque<StepReport>,
    steps: usize,
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
}

impl MapSource for FakeFlight {}

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
    use nova_sim::catalog::StellarId;

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
        }
    }
}
