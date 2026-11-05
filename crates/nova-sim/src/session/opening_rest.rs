//! The rest of the frame a comm, escort or info dialog opens on, run at
//! gain 0 as the first step after the dialog's zero.
//!
//! **The original.** `_DoPlayGameWork(1)` runs `_HandlePlayer` first
//! (@0x43e19), and `_HandlePlayer` reads the player info (@0x702cb), comm
//! (@0x703d8) and mission info (@0x70411) keys late, after the
//! self-destruct's block (ending @0x702b2) and before the cloak key
//! (@0x705b7). The dialog runs modally there and its close zeroes
//! `_gSpeedMult` (the comm dialog @0x9690e, the planet comm @0x97af2, the
//! escort dialog @0x95693, the player info @0x70357, the mission info
//! @0x704e8), so the rest of that frame runs at gain 0: the rest of
//! `_HandlePlayer` (the cloak key and upkeep, the tribbles and
//! perishables @0x70827, the hints @0x7093d, the clamps, the zoom keys),
//! the player's `_HandleShipDisplay` (@0x43e29) and everything after it
//! in the pass (the collision passes, the AI, `_HandleShot`, the ship
//! loop, the world handlers), then double time's second pass (@0x45f31),
//! whole, and the frame counter's move (@0x45f36). Nothing scaled moves,
//! but the unscaled steps run: an NPC's breakup countdown and its think,
//! a hit, an arrival, a hint or cargo count on a due frame.
//!
//! **Here.** The view reads the key between steps, so the dialog opens
//! after a whole step and the session runs the opening frame's rest as
//! the next step ([`Session::rest_of_opening_pass`]), in place of a whole
//! one, owed where a key-block site writes the zero:
//!
//! - [`Session::zero_at_key_block`]: an opened dialog's close, the comm
//!   dialog's hang-up, the planet comm's two closes, the escort dialog's
//!   close and an info dialog's ([`Session::close_info_dialog`]). The next
//!   frame is held, as the player spent 334 ms or more in the dialog.
//! - [`Session::zero_at_key_block_unheld`]: an info press the dialog's own
//!   guard refused (a jump or a breakup for the player info, the screen
//!   faded for either). The opening frame's `_HandleTimeAdjustment`
//!   (@0x45b28) ran before the key block and spent any pending hold, so
//!   only a frame's time passes and the frame after the rest ramps from 0.
//!
//! Both act only in flight and by [`RuleKey::SpeedMultRamp`]'s engine
//! reading; by the other there is no zero and so no rest. The take-off's
//! zero owes none, nor does a press refused before any dialog (a comm
//! refusal, no active missions): their next step is a whole frame.
//!
//! The rest step runs no `_HandleTimeAdjustment`, so it runs at the gain
//! the zero left, 0, and a held zero's hold waits for the next step; the
//! view steps the stellars by that 0 between ([`Session::next_speed_mult`]).
//! Its pass is the advance pass from the key block on: the pass's head
//! ([`Session::pass_head`]), so the frame counter moves once, the work
//! from the key block ([`Session::work_from_key_block`]), then the pass
//! from the key block on ([`Session::pass_from_key_block`]). Double time's
//! second pass runs after it, whole, at 0. The player's head isn't run:
//! the tractor's hold, the gamma, the wrap, the lost target's let-go, the
//! Red Alert's check (@0x68c79), the trigger and the move, the warning on
//! a new jump (@0x6b3de), the escape pod's rescue (@0x68ddd) and the
//! display step's head (the death timer @0x6d7e6 through the
//! self-destruct @0x6fdb4, the escort menu's block @0x6df59 among it, so
//! the rest's report carries none). The pod and a breakup aren't reachable
//! there: every one of these dialogs refuses in the pod and while dying.
//!
//! The opening frame's head (the last iteration's tail @0x45f36,
//! @0x462fa-0x46375, @0x463b3-0x4643e, `_HandleTimeAdjustment` @0x45b28
//! and `_HandlePlayer` @0x68390-0x702bf) isn't run before the dialog: the
//! engine's last step before the press stands in for the frame before,
//! and the rest step's [`Session::pass_head`] runs that tail after the
//! dialog, not before it.
//!
//! The rest waits for the next step, which crosses no boundary the
//! player sees: no frame is drawn before that step runs, an in-flight
//! save is held by [`RuleKey::InFlightSave`]'s default, and ending play
//! in flight saves nothing by default.

use super::Session;
use super::game_work::PassStart;
use crate::ai::Behaviour;
use crate::catalog::{PilotCatalog, TrafficCatalog};
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::combat::{GamePass, Rules};
use crate::market;
use crate::rulebook::RuleKey;

impl Session {
    /// Zeroes the gain at a dialog's close at `_HandlePlayer`'s key block,
    /// holding the next frame, and owes the opening frame's rest to the
    /// next step: in flight, by [`RuleKey::SpeedMultRamp`]'s engine
    /// reading (see the module docs).
    pub(super) fn zero_at_key_block(&mut self) {
        if self.key_block_zeroes() {
            self.time.zero();
            self.opening_rest_owed = true;
        }
    }

    /// Zeroes the gain at an info press the dialog's own guard refused,
    /// after the opening frame's ramp spent any hold, so the next frame
    /// isn't held, and owes the opening frame's rest to the next step: in
    /// flight, by [`RuleKey::SpeedMultRamp`]'s engine reading (see the
    /// module docs).
    pub(super) fn zero_at_key_block_unheld(&mut self) {
        if self.key_block_zeroes() {
            // `_HandleTimeAdjustment` (@0x45b28), before the key block.
            self.time.frame();
            self.time.zero_unheld();
            self.opening_rest_owed = true;
        }
    }

    /// Whether a key-block site zeroes the gain: in flight, by the ramp's
    /// engine reading.
    fn key_block_zeroes(&self) -> bool {
        self.mode.landed().is_none() && self.ramps()
    }

    /// Takes the opening frame's rest owed, if the session is still in
    /// flight to run it.
    pub(super) fn take_opening_rest(&mut self) -> bool {
        std::mem::take(&mut self.opening_rest_owed) && self.mode.landed().is_none()
    }

    /// Whether the next step runs the opening frame's rest.
    pub(super) fn opening_rest_owed(&self) -> bool {
        self.opening_rest_owed
    }

    /// The rest of the frame a dialog opened on, as an advance pass at the
    /// gain as it stands, the traffic drawn from `catalog` and deciding as
    /// `behaviour` says, the fight by `rules`, drawing on `chance`: the
    /// pass's head, the work from the key block, then the pass from the
    /// key block on, the player's head left out (see the module docs).
    pub(super) fn rest_of_opening_pass(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog),
        behaviour: &(impl Behaviour + ?Sized),
        rules: Rules,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        self.ship.warp.gate = None;
        let frame = self.pass_head();
        let pass = GamePass::Advance {
            double_time: self.ship.warp.double_time,
        };
        self.work_from_key_block(frame, pass);
        self.ship.fate.player_exploded_this_tick = false;
        // The player stood where it is, and its head is read at the top
        // of the opening frame, which is now.
        let start = PassStart::of(self);
        self.load_sprite_masks(catalog);
        self.pass_from_key_block(
            &start,
            pass,
            Some(catalog),
            Some(catalog),
            behaviour,
            rules,
            chance,
        );
    }

    /// The work on `frame` that `_HandlePlayer` and `_DoPlayGameWork` run
    /// after the key block, on `pass`, before the move here: the
    /// tribbles and perishables on a due frame (@0x70827), the Red
    /// Alert's blink on the advance pass alone (`_HandleAlertSprite`
    /// @0x4451e), the tutorial's hints (@0x7093d) and the target brackets
    /// (`_HandleNavTargetSprites`, `_HandleWeapTargetSprites` @0x444fa,
    /// @0x444ff). A pass runs it from `Session::before_move`; the rest of
    /// the frame a dialog opened on runs it alone.
    pub(super) fn work_from_key_block(&mut self, frame: i16, pass: GamePass) {
        if market::junk_step_due(frame) && self.ship.condition != Condition::Destroyed {
            let capacity = self.capacity();
            market::step_junk(
                &self.statics.goods,
                &mut self.pilot.cargo,
                capacity,
                self.pilot.missions.tons_aboard(),
                self.setup.rules.source_for(RuleKey::JunkFlags),
                self.ship.latches.junk_latches,
            );
        }
        if pass != GamePass::Again {
            self.red_alert.step(frame);
        }
        self.tutorial_tick(frame);
        self.chart.brackets_tick(self.speed_mult());
    }
}

#[cfg(test)]
mod tests {
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort_dialog::EscortButton;
    use crate::flight_frame::StepInput;
    use crate::flight_frame::{Answer, FadedPress, FlightCommand};
    use crate::hail::Haggle;
    use crate::market;
    use crate::planet_comm::PlanetCommItem;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::apply::tests::{apply, jumping};
    use crate::session::escort_dialog::tests::{escorting, opened, upgradable};
    use crate::session::hail::tests::{hail, hailable, targeting};
    use crate::session::mission_screens::tests::{docked, screens, to_mars};
    use crate::session::planet_comm::tests::{at_earth, planet_catalog};
    use crate::session::tests::{armed, facing_an_npc};
    use crate::session::time_adjustment::tests::pass_gains;
    use crate::session::time_adjustment::tests::steps;
    use crate::session::{MissionInfoPress, Session};
    use crate::testkit::{FakePilotCatalog, land_now};
    use crate::{MissionId, SpeedMult, TimeAdjustment};

    /// The gains of `n` frames ramping from a zero no frame holds.
    fn ramp_from_zero(n: usize) -> Vec<f64> {
        let mut time = TimeAdjustment::at(SpeedMult::ZERO);
        (0..n).map(|_| time.frame().get()).collect()
    }

    /// Each key-block site that closes an opened dialog, with what it
    /// leaves: the catalog and the session just after the close.
    fn closes() -> Vec<(&'static str, FakePilotCatalog, Session)> {
        closes_by(Rulebook::default())
    }

    /// [`closes`], each session flown by `rules`.
    fn closes_by(rules: Rulebook) -> Vec<(&'static str, FakePilotCatalog, Session)> {
        let hang_up = {
            let catalog = hailable();
            let mut session = targeting(&catalog).with_rules(rules);
            hail(&mut session, &catalog);
            session.hang_up();
            ("comm hang-up", catalog, session)
        };
        let planet_close = {
            let catalog = planet_catalog();
            let mut session = at_earth(&catalog, 1_000_000, 0).with_rules(rules);
            crate::session::planet_comm::tests::hail(&mut session, &catalog, 0, 0);
            crate::session::planet_comm::tests::press(
                &mut session,
                &catalog,
                PlanetCommItem::Close,
                0,
            );
            ("planet comm Close", catalog, session)
        };
        let bribe_close = {
            let catalog = planet_catalog();
            let mut session = at_earth(&catalog, 1_000_000, 1).with_rules(rules);
            crate::session::planet_comm::tests::hail(&mut session, &catalog, 0, 31);
            crate::session::planet_comm::tests::press(
                &mut session,
                &catalog,
                PlanetCommItem::Second,
                99,
            );
            assert_eq!(session.haggle_bribe(Haggle::Accept, &catalog), None);
            ("planet comm bribe", catalog, session)
        };
        let escort = |button, name| {
            let catalog = upgradable();
            let mut session = escorting(&catalog).with_rules(rules);
            opened(&mut session, &catalog);
            session.press_escort(button);
            (name, catalog, session)
        };
        let player_info = {
            let catalog = crate::testkit::catalog();
            let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
            assert!(session.open_player_info(&catalog).is_some());
            apply(&mut session, FlightCommand::CloseInfoDialog);
            ("player info", catalog, session)
        };
        let mission_info = {
            let catalog = screens(vec![to_mars(130)], Vec::new());
            let mut session = flying_a_mission(&catalog).with_rules(rules);
            apply(&mut session, FlightCommand::PressMissionInfo);
            apply(&mut session, FlightCommand::CloseInfoDialog);
            ("mission info", catalog, session)
        };
        vec![
            hang_up,
            planet_close,
            bribe_close,
            escort(EscortButton::Close, "escort Close"),
            escort(EscortButton::Release, "escort Release"),
            player_info,
            mission_info,
        ]
    }

    /// `catalog`'s session in flight past the take-off's grace and at
    /// full gain, carrying mission 130.
    fn flying_a_mission(catalog: &FakePilotCatalog) -> Session {
        let mut session = docked(catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        session
            .take_off(catalog, &mut NeverFires)
            .expect("takes off");
        session.port.takeoff_grace = 0;
        session.at_gain(SpeedMult::FULL);
        session
    }

    #[test]
    fn each_key_block_close_runs_the_opening_frames_rest_at_zero_before_the_held_frame() {
        let mut expected = vec![0.0, 0.0];
        expected.extend(ramp_from_zero(1));
        for (name, catalog, mut session) in closes() {
            assert_eq!(pass_gains(&mut session, &catalog, 3), expected, "{name}");
        }
    }

    /// Each press an info dialog's own guard refuses, after `before` if
    /// any, with what it leaves: the catalog and the session just after
    /// it.
    fn refusals(before: Option<&FlightCommand>) -> Vec<(&'static str, FakePilotCatalog, Session)> {
        refusals_by(Rulebook::default(), before)
    }

    /// [`refusals`], each session flown by `rules`.
    fn refusals_by(
        rules: Rulebook,
        before: Option<&FlightCommand>,
    ) -> Vec<(&'static str, FakePilotCatalog, Session)> {
        let before = |session: &mut Session| {
            if let Some(command) = before {
                apply(session, command.clone());
            }
        };
        let jump_refused = {
            let catalog = crate::testkit::catalog();
            let mut session = jumping().with_rules(rules);
            before(&mut session);
            assert_eq!(session.open_player_info(&catalog), None);
            ("player info in a jump", catalog, session)
        };
        let faded_player_info = {
            let catalog = crate::testkit::catalog();
            let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
            before(&mut session);
            assert_eq!(
                apply(
                    &mut session,
                    FlightCommand::PressFaded(FadedPress::PlayerInfo)
                ),
                Answer::PlayerInfoFaded
            );
            ("faded player info", catalog, session)
        };
        let faded_mission_info = {
            let catalog = screens(vec![to_mars(130)], Vec::new());
            let mut session = flying_a_mission(&catalog).with_rules(rules);
            before(&mut session);
            assert_eq!(
                apply(
                    &mut session,
                    FlightCommand::PressFaded(FadedPress::MissionInfo)
                ),
                Answer::MissionInfo(MissionInfoPress::Faded)
            );
            ("faded mission info", catalog, session)
        };
        vec![jump_refused, faded_player_info, faded_mission_info]
    }

    #[test]
    fn a_press_an_info_guard_refuses_runs_the_rest_at_zero_then_ramps() {
        let mut expected = vec![0.0];
        expected.extend(ramp_from_zero(2));
        for before in [None, Some(FlightCommand::SwitchedIn)] {
            for (name, catalog, mut session) in refusals(before.as_ref()) {
                assert_eq!(
                    pass_gains(&mut session, &catalog, 3),
                    expected,
                    "{name} after {before:?}"
                );
            }
        }
    }

    /// Opens the player info over `session`'s flight and closes it.
    fn closed_player_info(session: &mut Session, catalog: &FakePilotCatalog) {
        assert!(session.open_player_info(catalog).is_some());
        apply(session, FlightCommand::CloseInfoDialog);
    }

    /// The player's shots in flight.
    fn player_shots(session: &Session) -> usize {
        session
            .shots()
            .iter()
            .filter(|shot| shot.firer == ShipRef::Player)
            .count()
    }

    #[test]
    fn the_rest_skips_the_players_head() {
        let catalog = armed();
        let mut session = Session::start(&catalog).expect("starts");
        closed_player_info(&mut session, &catalog);
        let firing = StepInput {
            fire: (true, false),
            ..StepInput::default()
        };
        let mut shots = Vec::new();
        for _ in 0..2 {
            steps(&mut session, &catalog, &firing, 1);
            shots.push(player_shots(&session));
        }
        assert_eq!(shots, [0, 1], "the trigger is the head's");
    }

    #[test]
    fn the_rest_runs_the_npcs_unscaled_steps() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let dying = |ticks_left| Condition::Dying {
            ticks_left,
            starting: false,
        };
        session.space.traffic.npcs_mut()[0].condition = dying(5);
        closed_player_info(&mut session, &catalog);
        steps(&mut session, &catalog, &StepInput::default(), 1);
        assert_eq!(
            session.npcs()[0].condition,
            dying(4),
            "its breakup's countdown, unscaled (@0x35083-0x350a3)"
        );
    }

    #[test]
    fn the_rest_moves_the_frame_counter_once() {
        let catalog = crate::testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        closed_player_info(&mut session, &catalog);
        let mut expected = Vec::new();
        let mut frames = Vec::new();
        let mut frame = session.space.frame;
        for _ in 0..2 {
            frame = market::next_frame(frame);
            expected.push(frame);
            steps(&mut session, &catalog, &StepInput::default(), 1);
            frames.push(session.space.frame);
        }
        assert_eq!(frames, expected, "@0x45f36, once a step");
    }

    #[test]
    fn double_time_runs_its_second_pass_after_the_rest_at_zero() {
        let catalog = crate::testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        closed_player_info(&mut session, &catalog);
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
        let ramp = ramp_from_zero(1)[0];
        assert_eq!(gains, [vec![0.0, 0.0], vec![0.0, 0.0], vec![ramp, ramp]]);
        assert!(report.steps[0].between_passes.is_some(), "drawn between");
    }

    #[test]
    fn by_the_earlier_reading_no_rest_is_owed() {
        let bible = Rulebook::default().with_override(RuleKey::SpeedMultRamp, RuleSource::Bible);
        for (name, catalog, mut session) in closes_by(bible) {
            assert_eq!(pass_gains(&mut session, &catalog, 2), [1.0, 1.0], "{name}");
        }
        for (name, catalog, mut session) in refusals_by(bible, None) {
            assert_eq!(pass_gains(&mut session, &catalog, 2), [1.0, 1.0], "{name}");
        }
        let catalog = armed();
        let mut session = Session::start(&catalog).expect("starts").with_rules(bible);
        closed_player_info(&mut session, &catalog);
        let firing = StepInput {
            fire: (true, false),
            ..StepInput::default()
        };
        steps(&mut session, &catalog, &firing, 1);
        assert_eq!(player_shots(&session), 1, "a whole frame, its trigger read");
    }

    #[test]
    fn the_frame_shows_the_rests_gain_next() {
        let mut frame = crate::flight_frame::FlightFrame::default();
        for (name, catalog, mut session) in refusals(None).into_iter().chain(closes()) {
            let mut nexts = Vec::new();
            for _ in 0..2 {
                session.fill_frame(&mut frame);
                nexts.push(frame.speed_mult);
                steps(&mut session, &catalog, &StepInput::default(), 1);
            }
            assert_eq!(nexts, [SpeedMult::ZERO, session.speed_mult()], "{name}");
        }
    }

    #[test]
    fn a_key_block_site_owes_nothing_landed_and_a_landed_step_drops_the_rest() {
        let catalog = crate::testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        let gain = session.speed_mult();
        apply(
            &mut session,
            FlightCommand::PressFaded(FadedPress::PlayerInfo),
        );
        assert_eq!(
            (session.speed_mult(), session.opening_rest_owed()),
            (gain, false),
            "no `_HandlePlayer` runs landed"
        );
        session.opening_rest_owed = true;
        let frame = session.space.frame;
        steps(&mut session, &catalog, &StepInput::default(), 1);
        assert_eq!(session.space.frame, frame, "no pass ran");
        assert!(!session.opening_rest_owed());
    }
}
