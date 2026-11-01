//! The session's frame gain, the original's `_gSpeedMult`, and where the
//! session writes it (see `nova_sim_base::speed_mult` for the ramp
//! itself, `_HandleTimeAdjustment` @0x33332-0x3349b).
//!
//! - **Each step** in flight runs one frame of the ramp before its passes
//!   ([`Session::adjust_time`]), as `_PlayGame`'s loop calls
//!   `_HandleTimeAdjustment` (@0x45b28) before `_DoPlayGameWork(1)`
//!   (@0x45bc8), so both of double time's passes run at one gain. A landed
//!   step doesn't: the spaceport runs no `_PlayGame` loop.
//! - **The zeroes** ([`Session::zero_speed_mult`]): the take-off, as the
//!   stay's tail (`_PlayerLandOnStellar` @0x6375e-0x63769, after
//!   `_DoPortDialog` returns) runs on every way out of the spaceport; the
//!   hail's, the planet comm's and the escort dialog's closes (@0x9690e,
//!   @0x97af2, @0x95693); and the player and mission info dialogs
//!   (`_HandlePlayer` @0x70357, @0x704e8), at the press the dialog's own
//!   guard refuses as well as one it opens, as the write follows the call
//!   whatever it did. The session zeroes at the opening, not the close:
//!   no step runs under the dialog, so the next frame sees the same. A
//!   press `_HandlePlayer` refuses before the call (in a jump, in the
//!   take-off's grace, breaking up, or no active missions) leaves the
//!   gain.
//! - **The switch-in** ([`Session::switch_in`]): `_SwitchedIn`'s 1.0
//!   (@0x513e1-0x513ec).
//! - **A gap** ([`Session::hold_time`]): the view's 334 ms or more without
//!   a step, the original's frame of `new` ≥ 10.0, which keeps the gain.
//! - **Game entry** ([`Session::enter_game`]): the gain the last game left,
//!   carried by the app ([`Session::carry_speed_mult`]): 0.0 after a game
//!   that ended landed, as the stay's tail ran on the way out.
//!
//! By the other reading of [`RuleKey::SpeedMultRamp`] neither a zero nor
//! a game entry moves the gain from full, the port's earlier feel.
//!
//! The frame a hail or an info dialog opens on runs its rest at the gain
//! it had: the engine's key read that opens a dialog ends the tick, so no
//! pass of it runs after the open.

use crate::SpeedMult;
use crate::rulebook::{RuleKey, RuleSource};

use super::Session;

impl Session {
    /// Runs one frame of the ramp before a step's passes, unless landed
    /// (see the module docs).
    pub(super) fn adjust_time(&mut self) {
        if self.mode.landed().is_none() {
            self.time.frame();
        }
    }

    /// The frame's gain as it stands, `_gSpeedMult`: the last step's, or a
    /// writer's since.
    #[must_use]
    pub fn speed_mult(&self) -> SpeedMult {
        self.time.gain()
    }

    /// The gain the next step will run at, which the view steps the
    /// stellars by between steps.
    #[must_use]
    pub(super) fn next_speed_mult(&self) -> SpeedMult {
        self.time.next()
    }

    /// Whether the ramp follows the original (see [`RuleKey::SpeedMultRamp`]).
    fn ramps(&self) -> bool {
        self.setup.rules.source_for(RuleKey::SpeedMultRamp) == RuleSource::Engine
    }

    /// Zeroes the gain and holds the next frame, by the engine's reading:
    /// a take-off, a dialog's close (see the module docs).
    pub(super) fn zero_speed_mult(&mut self) {
        if self.ramps() {
            self.time.zero();
        }
    }

    /// The app switched back in: the gain full, the next frame held, as
    /// `_SwitchedIn` sets it (@0x513e1-0x513ec).
    pub fn switch_in(&mut self) {
        self.time.switched_in();
    }

    /// Holds the next frame: the view went 334 ms or more without a step.
    pub(super) fn hold_time(&mut self) {
        self.time.hold();
    }

    /// Game entry at `gain`, the one the last game left: by the engine's
    /// reading the passes and the first frame run at it; by the other,
    /// at full gain.
    pub(super) fn enter_time(&mut self, gain: SpeedMult) {
        let gain = if self.ramps() { gain } else { SpeedMult::FULL };
        self.time.entered(gain);
    }

    /// The gain the next game of the run enters at: 0.0 while landed, as
    /// the stay's tail zeroes it on the way out (@0x6375e-0x63769), and the
    /// gain as it stands otherwise.
    #[must_use]
    pub(super) fn carry_speed_mult(&self) -> SpeedMult {
        if self.mode.landed().is_some() {
            SpeedMult::ZERO
        } else {
            self.time.gain()
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::flight_frame::{FadedPress, FlightCommand, StepInput, StepReport};
    use crate::rulebook::Rulebook;
    use crate::session::StepCollaborators;
    use crate::session::apply::tests::{apply, jumping};
    use crate::testkit::{FakePilotCatalog, catalog, land_now};

    /// The ramp after a zero, frame by frame.
    const RAMP: [f64; 4] = [0.0, 0.25, 0.4375, 0.578_125];

    /// The gains of `n` frames, each a step's adjustment alone.
    pub(in crate::session) fn frame_gains(session: &mut Session, n: usize) -> Vec<f64> {
        (0..n)
            .map(|_| {
                session.adjust_time();
                session.speed_mult().get()
            })
            .collect()
    }

    /// `count` steps of `session` under `input`, on `catalog`'s
    /// collaborators.
    fn steps(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        input: &StepInput,
        count: u32,
    ) -> StepReport {
        let behaviour = crate::ai::NovaAi::default();
        let mut chance = NeverFires;
        let mut collab = StepCollaborators {
            catalog,
            behaviour: &behaviour,
            rules: Rules::default(),
            chance: &mut chance,
        };
        let mut report = StepReport::default();
        session.step(input, count, &mut collab, &mut report);
        report
    }

    /// The gain each pass of `count` steps ran at.
    pub(in crate::session) fn pass_gains(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        count: u32,
    ) -> Vec<f64> {
        steps(session, catalog, &StepInput::default(), count)
            .steps
            .iter()
            .flat_map(|run| run.passes.iter().map(|pass| pass.speed_mult.get()))
            .collect()
    }

    fn took_off(rules: Rulebook) -> (FakePilotCatalog, Session) {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
        land_now(&mut session).expect("lands");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        (catalog, session)
    }

    #[test]
    fn a_take_off_runs_its_tail_passes_and_first_frame_at_zero_then_ramps() {
        let (catalog, mut session) = took_off(Rulebook::default());
        let tail: Vec<f64> = session
            .take_stellar_passes()
            .iter()
            .map(|pass| pass.gain().get())
            .collect();
        assert_eq!(tail, [0.0, 0.0]);
        assert_eq!(pass_gains(&mut session, &catalog, 4), RAMP);
    }

    #[test]
    fn by_the_earlier_reading_a_take_off_runs_at_full_gain() {
        let rules = Rulebook::default().with_override(RuleKey::SpeedMultRamp, RuleSource::Bible);
        let (catalog, mut session) = took_off(rules);
        let tail: Vec<f64> = session
            .take_stellar_passes()
            .iter()
            .map(|pass| pass.gain().get())
            .collect();
        assert_eq!(tail, [1.0, 1.0]);
        assert_eq!(pass_gains(&mut session, &catalog, 2), [1.0, 1.0]);
    }

    #[test]
    fn the_frame_shows_the_next_steps_gain() {
        let (catalog, mut session) = took_off(Rulebook::default());
        let mut frame = crate::flight_frame::FlightFrame::default();
        let mut nexts = Vec::new();
        for _ in 0..3 {
            session.fill_frame(&mut frame);
            nexts.push(frame.speed_mult.get());
            pass_gains(&mut session, &catalog, 1);
        }
        assert_eq!(nexts, RAMP[..3]);
    }

    #[test]
    fn double_time_runs_both_passes_at_one_gain() {
        let (catalog, mut session) = took_off(Rulebook::default());
        let input = StepInput {
            double_time_key: true,
            ..StepInput::default()
        };
        let report = steps(&mut session, &catalog, &input, 3);
        let gains: Vec<Vec<f64>> = report
            .steps
            .iter()
            .map(|run| {
                run.passes
                    .iter()
                    .map(|pass| pass.speed_mult.get())
                    .collect()
            })
            .collect();
        assert_eq!(
            gains,
            [vec![0.0, 0.0], vec![0.25, 0.25], vec![0.4375, 0.4375]]
        );
    }

    #[test]
    fn landed_steps_leave_the_ramp() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.time.zero();
        session.adjust_time();
        land_now(&mut session).expect("lands");
        steps(&mut session, &catalog, &StepInput::default(), 3);
        assert_eq!(session.speed_mult(), SpeedMult::ZERO);
    }

    #[test]
    fn switched_in_restores_full_gain() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.time.zero();
        frame_gains(&mut session, 2);
        assert_eq!(
            apply(&mut session, FlightCommand::SwitchedIn),
            crate::flight_frame::Answer::Done
        );
        assert_eq!(session.speed_mult(), SpeedMult::FULL);
        assert_eq!(frame_gains(&mut session, 2), [1.0, 1.0]);
    }

    #[test]
    fn a_held_step_input_keeps_the_ramp() {
        let (catalog, mut session) = took_off(Rulebook::default());
        pass_gains(&mut session, &catalog, 2);
        let held = StepInput {
            held: true,
            ..StepInput::default()
        };
        let report = steps(&mut session, &catalog, &held, 2);
        let gains: Vec<f64> = report
            .steps
            .iter()
            .map(|run| run.passes[0].speed_mult.get())
            .collect();
        assert_eq!(gains, [0.25, 0.4375], "the first held, then on");
    }

    #[test]
    fn a_faded_info_press_zeroes_the_gain_too() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        apply(
            &mut session,
            FlightCommand::PressFaded(FadedPress::PlayerInfo),
        );
        assert_eq!(frame_gains(&mut session, 2), RAMP[..2]);
    }

    #[test]
    fn an_opened_player_info_zeroes_the_gain_in_flight_only() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.open_player_info(&catalog);
        assert_eq!(frame_gains(&mut session, 2), RAMP[..2]);
        let mut landed = Session::start(&catalog).expect("starts");
        land_now(&mut landed).expect("lands");
        landed.open_player_info(&catalog);
        assert_eq!(landed.speed_mult(), SpeedMult::FULL, "the port's own");
    }

    #[test]
    fn an_info_press_refused_in_a_jump_leaves_the_gain() {
        for command in [
            FlightCommand::PressPlayerInfo,
            FlightCommand::PressMissionInfo,
            FlightCommand::PressFaded(FadedPress::PlayerInfo),
        ] {
            let mut session = jumping();
            apply(&mut session, command.clone());
            assert_eq!(session.speed_mult(), SpeedMult::FULL, "{command:?}");
            assert_eq!(frame_gains(&mut session, 1), [1.0], "{command:?}");
        }
    }

    #[test]
    fn a_mission_info_press_with_no_missions_leaves_the_gain() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        apply(&mut session, FlightCommand::PressMissionInfo);
        assert_eq!(frame_gains(&mut session, 1), [1.0]);
    }

    #[test]
    fn a_landed_session_carries_zero_and_a_flying_one_its_gain() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.time.zero();
        frame_gains(&mut session, 2);
        assert_eq!(session.carry_over().speed_mult, Some(SpeedMult::new(0.25)));
        let mut landed = Session::start(&catalog).expect("starts");
        land_now(&mut landed).expect("lands");
        assert_eq!(landed.carry_over().speed_mult, Some(SpeedMult::ZERO));
    }
}
