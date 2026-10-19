//! Double time: the original's mode that runs a second, partial pass of
//! the game's work on each frame, so the game plays at about twice the
//! speed.
//!
//! **The switch** (`_PlayGame` @0x463b3-0x4643e). Every iteration of the
//! main loop, after the frame counter moves on (@0x45f36) and the mission
//! sound plays (@0x462fa-0x46375), sets `_doubleTime` ([`switch`]):
//!
//! - on while `_warpUpSnd[1]`, `snd ` 129 "Warp up x2", plays
//!   (`ST_IsSoundPlaying` @0x463b3-0x463c1, set @0x46415-0x4641a),
//!   whatever the key says, so a warp begun in double time runs in it to
//!   the end;
//! - else off without Caps Lock's lock state (`_QuickKeyCheck(0x39)`
//!   @0x463c3, reading the `GetKeys` map @0xa0f5; @0x4641f-0x4642c);
//! - else off while `_warpUpSnd[0]`, `snd ` 128, plays and the player
//!   faces within 10° of its desired heading, +0x68
//!   ([`angular_difference`] of the whole-degree facing and +0x68,
//!   @0x463d3-0x4640c), so a warp begun in single time lines up in it;
//! - else on.
//!
//! Ours takes the key's state from its caller
//! ([`Session::set_double_time_key`], which latches the switch at once):
//! the view toggles it on each Caps Lock press, an accepted difference:
//! it can drift from the lock itself when pressed while unfocused or
//! already on at launch, and on macOS, where winit 0.30.13 reports no Caps
//! Lock key, there is no double time. The warp-ups' play state is the
//! session's ([`WarpUps`](crate::warp::WarpUps)). The session latches the
//! switch at the top of each advance pass ([`Session::tick`]), after the
//! warp-ups play on a frame, where the original latches at the end of the
//! iteration before: nothing runs between the two.
//!
//! The desired heading, +0x68, is the session's `aim`, in whole degrees
//! as `_CalcInterceptAngle` (@0x2af60-0x2af90) gives it
//! ([`intercept_angle`](crate::trig::intercept_angle)); 0 as a session
//! starts (pilot init @0x1c315), and kept after a jump ends. It is set:
//!
//! - in a jump's warp stage, a slow ship's, to the bearing of the map
//!   position of the next system from the current one (@0x6b0cf), in the
//!   turn and through the warp;
//! - while the ship brakes for a jump, against its velocity
//!   (`_CalcInterceptAngle(v × 100, 0)` @0x6b761), unless it is
//!   inertialess, whose brake writes none (@0x6b6c3-0x6b71c);
//! - while the face-target key is held out of a jump, at the ship target
//!   or the nav selection's stellar (@0x6af65, @0x6af9e, @0x6afea; see
//!   the `steering` module), disabled or not;
//! - while the reverse key is held out of a jump by a player not
//!   disabled, moving at 0.05 or more on either axis (@0x6c7d8-0x6c822),
//!   against its velocity: `(_CalcInterceptAngle(0, v × 100) + 180) mod
//!   360` (@0x6c88e), over the face key's, whatever the turn keys say;
//!   but an inertialess ship's reverse brakes and writes none
//!   (@0x6c770-0x6c7d3; see the `inertialess` module).
//!
//! Left out: `_gameFrozen`'s clearing (@0x46427, @0x4642f-0x4643e) is a cheat's, and
//! ours has no cheats. The mode is not saved: `_doubleTime` is a global,
//! not a pilot field.
//!
//! **The second pass** (`_PlayGame` @0x45f20-0x45f31). An iteration runs
//! `_DoPlayGameWork(1)` (@0x45bc1-0x45bc8), draws the frame, then, in
//! double time, `_DoPlayGameWork(0)`, and only then moves the frame
//! counter on, once. The second pass ([`Session::tick_again`]) therefore
//! runs on the same counter value as the advance pass.
//!
//! **What the second pass runs** (`_DoPlayGameWork` @0x43de5-0x4453d, its
//! argument saved @0x43dee-0x43df2):
//!
//! - On both passes: `_HandlePlayer` (@0x43e19: the player's controls and
//!   motion, its firing `_FirePlayerWeapon` @0x6ab55, the Red Alert's
//!   check @0x68c79, the tutorial's hint block @0x7093d, the tribbles and
//!   perishables step @0x70827), the collisions and `_CheckShotProximities`
//!   (@0x43e3a-0x44015), `_HandleShot` (@0x44385), `_HandleShip` for every
//!   ship (@0x44483: NPC motion, cloak holds and fades, and their firing
//!   `_FireAIShipWeapon` @0x34ffd), and the sprite routines, explosions
//!   and boxes among them (@0x444d2-0x44513). So here, in that order
//!   ([`Session::tick_again`]): the player's part of the pass runs again
//!   on that frame, its self-repair, point defence, sabotage clock and
//!   trigger before its move (see the `player_repair` and `sabotage`
//!   modules), then its eject check (see the `eject` module); the fight's
//!   collision passes ([`GamePass::Again`]), their hits answered; no
//!   NPC's point defence, which is `_AIDispatch`'s; the shots, their hits
//!   answered; the traffic's motion
//!   ([`Traffic::move_in`](crate::traffic::Traffic::move_in)) with its
//!   departures and fighters docking; the NPCs' fire; the beams and all
//!   that follows from the pass's hits; then the reinforcement clock
//!   (`_HandleReinforcements` @0x44504; see the `reinforcements`
//!   module). A due frame therefore steps the junk and
//!   the hints twice, as the original does, and the sabotage clock, the
//!   escape pod's countdown, the breakup's countdown and its klaxon run
//!   on both passes too, and a called fleet comes twice as fast.
//! - On the advance pass alone (`arg == 0` jumps from @0x4402a to
//!   @0x44350): `_HandleStatus`, `_MissionHandler`, `_HandleVoiceSound`,
//!   `_HandleComm`, `_CalculateTargetedDamage`, `_EnterMoreShips` (the
//!   arrivals and the escort-strength timer), `_AIDispatch` for every ship
//!   (@0x44270-0x442ae: the decisions, the cloak thinks, and the scans,
//!   `_ScanPlayer` running under `_HighLevelAIHandler` @0x8f38a), and
//!   `_HandleAlertSprite` (@0x44518), which steps the Red Alert's blink.
//!   So the second pass plays no mission sound, rolls no arrivals, decides
//!   nothing, takes no scan, hails nothing and does not move the AI's
//!   timer.
//! - The strikes the second pass raises are answered in that pass:
//!   `_HandleShot` (0x35586-0x361ab) takes no pass argument and calls
//!   `_DamageShip` (@0x35855, @0x36106), and neither reads `_doubleTime`,
//!   so `_DamageShip`'s whole reaction block (@0x3b44d-0x3bf5e: target and
//!   provocation, the gate's wait cut @0x3be6e-0x3be7c,
//!   `_AIMakeShipAttackPlayer` @0x3be66 and its quote,
//!   `_ShowPersonHailQuote` @0x89ce1) runs there as on the advance pass.
//!   The pass answers the collision passes' and the shots' hits as they
//!   land, and its caller answers what its close struck after it
//!   ([`Session::answer_strikes`]) and says the attack quotes
//!   ([`Session::tick_quotes_again`]), before the frame is drawn. The
//!   block plays no sound and tells no comm.
//! - `_HandleShip`'s hail-quote block (@0x33f0f-0x33fba) has no pass
//!   test, so each eligible person draws `Rand(140)` on both passes
//!   ([`Session::tick_quotes_again`], after the attack quotes). The
//!   display timer (`_commCount`, the session's
//!   [`CommLine`](crate::CommLine)) is counted down only by `_HandleComm`
//!   (@0x301ce-0x301de), on the advance pass, so after any line on the
//!   advance pass only a quote due now (forced, @0x33f65-0x33f6c) can
//!   follow on the second; otherwise each frame's odds of a random quote
//!   double. The quote clock moves once a frame, on the advance pass.
//! - `_HandleBeams(arg)` (@0x4452f-0x4453d): on the second pass a beam
//!   only ages (see [`GamePass`]).
//! - The warp's clock is `TickCount` less the warp stage's stamp (see the
//!   [`warp`](crate::warp) module), real time, which the second pass
//!   doesn't move on: the pre-jump flies on both passes, and the second
//!   can start the clock, but only the advance pass moves it on, and a
//!   collapse on the second pass reads the frame's clock as its advance
//!   pass left it.
//!
//! **Ordering.** The original draws its frame between the two passes
//! (@0x45d96-0x45e3e); the view draws after both. Every frame shown is
//! still one counter tick apart from the last, so the difference is a
//! constant phase of one pass that no player can see.

use super::Session;
use super::game_work::PassStart;
use crate::ai::Behaviour;
use crate::chance::Chance;
use crate::combat::{GamePass, Rules};
use crate::flight::Controls;
use crate::trig::angular_difference;

/// `_PlayGame`'s double-time switch (@0x463b3-0x4642f; see the module
/// docs), from whether the `key` is on, whether each warp-up is
/// `playing`, `snd ` 128 then 129, and whether the player faces within
/// 10° of its desired heading (`aligned`).
#[must_use]
pub fn switch(key: bool, playing: [bool; 2], aligned: bool) -> bool {
    let [single, double] = playing;
    double || (key && !(aligned && single))
}

/// How far, in whole degrees, the player may face off its desired
/// heading and count as aligned with it (`cmpl $0xa` @0x463fa).
const ALIGNED: i32 = 10;

impl Session {
    /// Takes the double-time key's state: `on` while double time should
    /// run, and latches the switch at once (see the module docs). Off
    /// until it says otherwise.
    pub fn set_double_time_key(&mut self, on: bool) {
        self.double_time_key = on;
        self.latch_double_time();
    }

    /// Sets double time by the switch (see the module docs).
    pub(super) fn latch_double_time(&mut self) {
        #[allow(clippy::cast_possible_truncation)]
        let facing = self.player.heading as i16;
        let off = i32::from(angular_difference(facing, self.aim)).abs();
        self.double_time = switch(
            self.double_time_key,
            [false, true].map(|double| self.warp_ups.playing(double)),
            off <= ALIGNED,
        );
    }

    /// Whether double time is on.
    #[must_use]
    pub fn double_time(&self) -> bool {
        self.double_time
    }

    /// Runs double time's second pass under the player's `controls`, the
    /// fight by `rules`, the NPCs answering its hits as `behaviour` says,
    /// drawing on `chance` (see the module docs), in `_DoPlayGameWork(0)`'s
    /// order: `_HandlePlayer`'s fight before its move
    /// ([`Session::player_fires`]), then its move; the collision passes,
    /// their hits answered; no AI, so no NPC point defence; `_HandleShot`,
    /// its hits answered; the traffic's motion; `_HandleShip`'s fire;
    /// `_HandleBeams` and what the pass leaves; then the asteroids and the
    /// reinforcement clock; from the eject check on, the advance pass's own
    /// routine without its AI ([`Session::pass_after_move`]). Its caller
    /// runs it after the advance pass
    /// ([`Session::game_work`]) while [`Session::double_time`] is on.
    /// Landed, or once a jump has begun, it does nothing, as the advance
    /// pass stands still; a jump that begins on its move stops it there,
    /// and so does the escape pod's countdown running out on it, the
    /// rescue then due ([`Session::rescue_if_due`], which its caller asks
    /// after this pass as after the advance pass). Its caller then answers
    /// what the pass's close struck ([`Session::answer_strikes`]) and runs
    /// the second-pass quote roll, the attack quotes the answers raise
    /// first ([`Session::tick_quotes_again`]).
    pub fn tick_again(
        &mut self,
        controls: Controls,
        behaviour: &(impl Behaviour + ?Sized),
        rules: Rules,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if self.landed.is_some() || self.jumping().is_some() {
            return;
        }
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let pass = GamePass::Again;
        self.before_move(self.pass_frame, pass);
        let start = PassStart::of(self);
        self.player_fires(pass, rules, chance);
        self.player_move(controls, pass);
        if self.jumping().is_some() || self.rescue_due {
            return;
        }
        // `_AIDispatch` runs on the advance pass alone (@0x4402a).
        self.pass_after_move(&start, pass, None, behaviour, rules, chance);
    }

    /// The traffic's part of the second pass, drawing on `chance`: the NPCs
    /// move ([`Traffic::move_in`](crate::traffic::Traffic::move_in)) in the
    /// world the tick flies them in (among the system's stellars, its
    /// gates' widths and the player, at the speed the player leads them
    /// at, which is all their motion reads); each mission ship that
    /// left raises its fate, and fighters dock. The fight that follows
    /// lets go of a target gone and orphans the fighters of a carrier
    /// gone, as it does after any departure, before anything is drawn.
    /// Nothing moves before the system is populated.
    pub(super) fn move_traffic_again(&mut self, chance: &mut (impl Chance + ?Sized)) {
        if self.traffic_due {
            return;
        }
        let marks = self.mission_marks();
        self.step_traffic(|traffic, world| traffic.move_in(world, chance));
        self.mission_ships_left(&marks);
        self.dock_fighters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::combat::armament::Trigger;
    use crate::combat::{CombatEvent, ShipRef};
    use crate::control::SetExpr;
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::session::tests::{OUT, armed, bound_for_131, facing_an_npc};
    use crate::sound::SimSound;
    use crate::testkit::{
        Draws, FakePilotCatalog, Scripted, begin_jump_now, catalog, fly_out, land_now, npc,
    };
    use crate::traffic::npc::NpcId;
    use crate::warp::SoundLength;

    const THRUST: Controls = Controls {
        thrust: true,
        turn: crate::flight::Turn::None,
        reverse: false,
        face: crate::flight::Face::Off,
        afterburner: false,
    };

    fn session(catalog: &FakePilotCatalog) -> Session {
        Session::start(catalog).expect("starts")
    }

    fn again(session: &mut Session, controls: Controls) {
        let behaviour = crate::ai::NovaAi::default();
        session.tick_again(controls, &behaviour, Rules::default(), &mut NeverFires);
    }

    /// [`catalog`] with a double-time warp-up, `snd ` 129, of 44100 frames
    /// at 22050 Hz: 120 Mac ticks, where 128, of which there is none, lasts
    /// 350.
    fn with_warp_up_x2() -> FakePilotCatalog {
        FakePilotCatalog {
            warp_sound_double_time: Some(SoundLength {
                frames: 44_100,
                rate: 22_050,
            }),
            ..catalog()
        }
    }

    #[test]
    fn double_time_ends_the_warps_lead_in_by_the_double_time_warp_up() {
        // w = 1.3: the lead-in lasts while t ≤ 0.35 × d / 1.69, 24.85 Mac
        // ticks for 129's 120 and 72.49 for 128's 350, through engine tick
        // 12 and 36.
        let catalog = with_warp_up_x2();
        for (double_time, last) in [(true, 12), (false, 36)] {
            let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 90.0);
            session.set_double_time_key(double_time);
            session.begin_jump().expect("jumps");
            assert_eq!(session.warp_clock(), Some(0), "at once");
            for _ in 0..last {
                session.tick(Controls::default());
            }
            assert_eq!(session.warp_clock(), Some(last));
            assert!(session.in_warp_lead_in(), "{double_time}");
            session.tick(Controls::default());
            assert!(!session.in_warp_lead_in(), "{double_time}");
        }
    }

    #[test]
    fn a_collapse_in_double_time_reads_the_double_time_warp_up() {
        // Facing up, the turn to east runs 30 ticks; disabled on the 15th,
        // the collapse reads t = 32 Mac ticks.
        let catalog = with_warp_up_x2();
        let w = crate::warp::warp_factor(0);
        let speed = crate::warp::lead_in_speed(32, w, 120);
        assert!(crate::warp::lead_in_speed(32, w, 350) < 0.0, "none by 128");
        for double_time in [true, false] {
            let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 0.0);
            session.set_double_time_key(double_time);
            session.begin_jump().expect("jumps");
            for _ in 0..15 {
                session.tick(Controls::default());
            }
            assert_eq!(session.warp_clock(), Some(15));
            let before = *session.player();
            session.condition = crate::combat::hull::Condition::Disabled;
            session.tick(Controls::default());
            assert_eq!(session.preparing_jump(), None, "collapsed");
            let expected = if double_time {
                assert!(speed > 0.0 && speed < session.top_speed(), "{speed}");
                crate::cargo_box::accel(45, speed)
            } else {
                before.velocity
            };
            assert_eq!(
                session.player().velocity,
                expected * crate::flight::DISABLED_DRAG,
                "{double_time}"
            );
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn the_warp_clock_moves_on_once_a_frame_in_double_time() {
        // The clock is `TickCount`, which the second pass does not move on
        // (F7). Facing up, the turn to east takes 15 frames at two turns a
        // frame.
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 0.0);
        session.set_double_time_key(true);
        session.begin_jump().expect("jumps");
        assert_eq!(session.warp_clock(), Some(0));
        for frame in 1..=5 {
            session.tick(Controls::default());
            assert_eq!(session.warp_clock(), Some(frame), "advance {frame}");
            again(&mut session, Controls::default());
            assert_eq!(session.warp_clock(), Some(frame), "again {frame}");
        }
        assert_eq!(session.player().heading, 30.0, "turned on both passes");
        assert_eq!(
            session.preparing_jump(),
            Some(crate::catalog::SystemId(131))
        );
    }

    #[test]
    fn a_collapse_on_the_second_pass_reads_the_frames_clock() {
        // Three frames into the turn, a collapse on the fourth frame's
        // advance pass reads t = 8 Mac ticks; disabled after it, the
        // second pass's collapse reads the same 8, where 10 would be the
        // next frame's.
        let w = crate::warp::warp_factor(0);
        let d = 36;
        let at_8 = crate::warp::lead_in_speed(8, w, d);
        let at_10 = crate::warp::lead_in_speed(10, w, d);
        let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 0.0);
        let top = session.top_speed();
        assert!(at_8 > 0.0 && at_8 < top && at_10 > at_8, "{at_8} {at_10}");
        session.warp_sounds[1] = d;
        session.set_double_time_key(true);
        session.begin_jump().expect("jumps");
        for _ in 0..3 {
            session.tick(Controls::default());
            again(&mut session, Controls::default());
        }
        let mut advance = session.clone();
        advance.condition = crate::combat::hull::Condition::Disabled;
        advance.tick(Controls::default());
        assert_eq!(advance.preparing_jump(), None, "collapsed");
        assert_eq!(
            advance.player().velocity,
            crate::cargo_box::accel(18, at_8) * crate::flight::DISABLED_DRAG
        );
        session.tick(Controls::default());
        session.condition = crate::combat::hull::Condition::Disabled;
        again(&mut session, Controls::default());
        assert_eq!(session.preparing_jump(), None, "collapsed");
        assert_eq!(
            session.player().velocity,
            crate::cargo_box::accel(21, at_8) * crate::flight::DISABLED_DRAG
        );
    }

    // The warp-up (`_HandlePlayer` @0x6b60d-0x6b690).

    /// The warp-up a jump begun `double_time` sounds.
    fn warp_up(double_time: bool) -> SimSound {
        if double_time {
            SimSound::JumpBeganInDoubleTime
        } else {
            SimSound::JumpBegan
        }
    }

    #[test]
    fn a_jump_begun_in_double_time_sounds_warp_up_x2_and_otherwise_warp_up() {
        for double_time in [true, false] {
            let mut session = bound_for_131(&catalog(), OUT, Vec2::ZERO, 90.0);
            session.set_double_time_key(double_time);
            session.begin_jump().expect("jumps");
            assert_eq!(
                session.take_sounds(),
                [warp_up(double_time)],
                "{double_time}"
            );
        }
        assert_ne!(SimSound::JumpBeganInDoubleTime, SimSound::JumpBegan);
    }

    /// Gives up `session`'s jump as a script's move in flight does (`N130`,
    /// [`Reset::Jump`](super::super::entry::Reset::Jump)): no halt and no
    /// arrival. Then puts the ship back out at rest facing up, bound for
    /// 131 again.
    fn given_up_by_a_move(session: &mut Session, catalog: &FakePilotCatalog) {
        session.run_set(
            &SetExpr::parse("N130").expect("parses"),
            &mut Scripted::default(),
        );
        let _ = session.settle_script(catalog, &mut NeverFires);
        assert_eq!(session.warp_clock(), None, "given up");
        let sounds = session.take_sounds();
        assert!(
            !sounds
                .iter()
                .any(|sound| matches!(sound, SimSound::Halt(_))),
            "no halt: {sounds:?}"
        );
        session
            .plot_course(crate::catalog::SystemId(131))
            .expect("a route");
        session.player = ShipState {
            position: OUT,
            velocity: Vec2::ZERO,
            heading: 0.0,
        };
    }

    /// Runs one frame of `session` with no controls: the advance pass and,
    /// in double time, the second.
    fn frame(session: &mut Session) {
        session.tick(Controls::default());
        if session.double_time() {
            again(session, Controls::default());
        }
    }

    /// The still-playing skip (`_HandlePlayer` @0x6b617-0x6b62b,
    /// @0x6b645-0x6b658): a jump whose warp stage starts while its warp-up
    /// still plays sounds none, though its clock starts. Here a script's
    /// move in flight gives the first jump up, with no halt and no
    /// arrival, which end the warp-up (see the next tests). The skip
    /// happens in the engine's normal play too: a ship knocked back past
    /// the jump speed in its turn brakes again, and the warp stage starts
    /// afresh as it slows (`braking_again_stops_the_warp_clock_...` in
    /// the session's tests). The escape pod's rescue gives a jump up as
    /// well, but the eject before it halts the warp-ups.
    #[test]
    fn a_warp_up_still_playing_is_not_sounded_again() {
        // 129 lasts 120 Mac ticks, 60 frames, and 128, of which there is
        // none, 350, 175 frames, in real time: double time's second pass
        // doesn't shorten it.
        let catalog = with_warp_up_x2();
        for (double_time, frames) in [(true, 60), (false, 175)] {
            let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 0.0);
            session.set_double_time_key(double_time);
            session.begin_jump().expect("jumps");
            assert_eq!(session.take_sounds(), [warp_up(double_time)]);
            for _ in 0..frames - 1 {
                frame(&mut session);
            }
            given_up_by_a_move(&mut session, &catalog);
            session.begin_jump().expect("jumps");
            assert_eq!(session.take_sounds(), [], "{double_time}: still playing");
            assert_eq!(session.warp_clock(), Some(0), "the clock starts");
            frame(&mut session);
            given_up_by_a_move(&mut session, &catalog);
            session.begin_jump().expect("jumps");
            assert_eq!(
                session.take_sounds(),
                [warp_up(double_time)],
                "{double_time}: over"
            );
        }
    }

    #[test]
    fn a_warp_up_of_the_other_speed_plays_on_its_own() {
        // 128, begun in single time, still plays as a jump begins in
        // double time: 129 sounds all the same. The ship faces 90° off its
        // aim, so the switch gives double time while 128 plays.
        let catalog = with_warp_up_x2();
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 0.0);
        session.begin_jump().expect("jumps");
        assert_eq!(session.take_sounds(), [SimSound::JumpBegan]);
        given_up_by_a_move(&mut session, &catalog);
        assert_eq!(session.aim, 0, "no tick has aimed it");
        session.player.heading = 90.0;
        session.set_double_time_key(true);
        assert!(session.double_time());
        session.begin_jump().expect("jumps");
        assert_eq!(session.take_sounds(), [SimSound::JumpBeganInDoubleTime]);
    }

    #[test]
    fn a_collapse_halts_the_warp_up_so_the_next_jump_sounds_it_again() {
        let catalog = with_warp_up_x2();
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 0.0);
        session.set_double_time_key(true);
        session.begin_jump().expect("jumps");
        assert_eq!(session.take_sounds(), [SimSound::JumpBeganInDoubleTime]);
        frame(&mut session);
        session.condition = crate::combat::hull::Condition::Disabled;
        frame(&mut session);
        assert_eq!(session.preparing_jump(), None, "collapsed");
        assert!(
            session
                .take_sounds()
                .contains(&SimSound::Halt(&crate::sound::WARP_UP_HALTS))
        );
        session.condition = crate::combat::hull::Condition::Intact;
        given_up_by_a_move(&mut session, &catalog);
        session.begin_jump().expect("jumps");
        assert_eq!(session.take_sounds(), [SimSound::JumpBeganInDoubleTime]);
    }

    #[test]
    fn the_arrival_ends_the_warp_up() {
        let catalog = with_warp_up_x2();
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 90.0);
        session.set_double_time_key(true);
        session.begin_jump().expect("jumps");
        assert!(session.warp_ups.playing(true));
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(session.warp_ups, crate::warp::WarpUps::default());
    }

    #[test]
    fn landing_ends_the_warp_up() {
        let catalog = with_warp_up_x2();
        let mut session = session(&catalog);
        session.warp_ups.start(true, 120);
        land_now(&mut session).expect("lands");
        assert_eq!(session.warp_ups, crate::warp::WarpUps::default());
    }

    #[test]
    fn double_time_follows_the_key_and_starts_off() {
        let mut session = session(&catalog());
        assert!(!session.double_time());
        session.set_double_time_key(true);
        assert!(session.double_time(), "at once");
        session.set_double_time_key(false);
        assert!(!session.double_time(), "at once");
    }

    // The switch (`_PlayGame` @0x463b3-0x4643e).

    #[test]
    fn the_switch_holds_through_warp_up_x2_and_drops_an_aligned_warp_up() {
        for key in [false, true] {
            for single in [false, true] {
                for aligned in [false, true] {
                    assert!(switch(key, [single, true], aligned), "129 plays");
                }
            }
        }
        for single in [false, true] {
            for aligned in [false, true] {
                assert!(!switch(false, [single, false], aligned), "no key");
            }
        }
        assert!(
            !switch(true, [true, false], true),
            "aligned while 128 plays"
        );
        assert!(switch(true, [true, false], false), "turned away");
        assert!(switch(true, [false, false], true), "128 over");
        assert!(switch(true, [false, false], false));
    }

    /// A session at rest out of the no-jump zone facing east, `snd ` 128
    /// lasting 350 Mac ticks and 129 120.
    fn east(catalog: &FakePilotCatalog) -> Session {
        bound_for_131(catalog, OUT, Vec2::ZERO, 90.0)
    }

    /// The aim a jump from `session`'s system to 131 sets, as the turn's
    /// `_CalcInterceptAngle` of the map positions gives it (@0x6b0cf).
    fn bearing_aim(session: &Session) -> i16 {
        let map = |id| session.galaxy.map().position(id).expect("on the map");
        crate::trig::intercept_angle(map(session.system()), map(crate::catalog::SystemId(131)))
    }

    #[test]
    fn double_time_holds_while_warp_up_x2_plays() {
        // 129 lasts 120 Mac ticks: 60 frames.
        let catalog = with_warp_up_x2();
        let mut session = east(&catalog);
        session.set_double_time_key(true);
        session.begin_jump().expect("jumps");
        assert_eq!(session.take_sounds(), [SimSound::JumpBeganInDoubleTime]);
        session.set_double_time_key(false);
        assert!(session.double_time(), "held by 129");
        for frame in 1..60 {
            session.tick(Controls::default());
            assert!(session.double_time(), "frame {frame}");
        }
        session.tick(Controls::default());
        assert!(!session.double_time(), "129 over");
    }

    #[test]
    fn the_key_gives_no_double_time_facing_the_aim_while_warp_up_plays() {
        let catalog = with_warp_up_x2();
        let mut session = east(&catalog);
        session.begin_jump().expect("jumps");
        assert_eq!(session.take_sounds(), [SimSound::JumpBegan]);
        session.tick(Controls::default());
        let aim = bearing_aim(&session);
        assert_eq!(session.aim, aim, "the jump's bearing");
        assert!((i32::from(aim) - 90).abs() <= 1, "{aim}");
        session.set_double_time_key(true);
        assert!(!session.double_time(), "aligned while 128 plays");
        session.player.heading = f32::from(aim) + 10.9;
        session.set_double_time_key(true);
        assert!(!session.double_time(), "10 degrees off, whole");
        session.player.heading = f32::from(aim) + 11.0;
        session.set_double_time_key(true);
        assert!(session.double_time(), "11 degrees off");
        session.player.heading = f32::from(aim);
        session.set_double_time_key(true);
        assert!(!session.double_time());
        // 128 lasts 350 Mac ticks: 175 frames from the jump.
        for _ in 1..174 {
            session.tick(Controls::default());
        }
        assert!(!session.double_time(), "128 plays on");
        session.tick(Controls::default());
        assert!(session.double_time(), "128 over");
    }

    #[test]
    fn the_brake_aims_against_the_velocity_and_the_turn_at_the_bearing() {
        let mut session = bound_for_131(&catalog(), Vec2::ZERO, Vec2::ZERO, 0.0);
        fly_out(&mut session);
        session.begin_jump().expect("jumps");
        let velocity = session.player().velocity;
        session.tick(Controls::default());
        assert_eq!(session.warp_clock(), None, "braking");
        assert_eq!(
            session.aim,
            crate::trig::intercept_angle(velocity * 100.0, Vec2::ZERO),
            "@0x6b761"
        );
        assert_eq!(session.aim, 180, "racing north: south");
        while session.warp_clock().is_none() {
            session.tick(Controls::default());
        }
        session.tick(Controls::default());
        assert_eq!(session.aim, bearing_aim(&session), "@0x6b0cf");
        while session.jumping().is_none() {
            session.tick(Controls::default());
        }
        session.tick(Controls::default());
        assert_eq!(session.aim, bearing_aim(&session), "on through the warp");
        session
            .arrive(&catalog(), &mut NeverFires)
            .expect("arrives");
        let kept = session.aim;
        session.tick(Controls::default());
        assert_eq!(session.aim, kept, "kept after the jump");
    }

    #[test]
    fn reverse_aims_against_the_velocity() {
        const REVERSE: Controls = Controls {
            thrust: false,
            turn: crate::flight::Turn::Left,
            reverse: true,
            face: crate::flight::Face::Off,
            afterburner: false,
        };
        let mut session = session(&catalog());
        session.player.velocity = Vec2::new(3.0, 1.0);
        session.tick(REVERSE);
        let toward = crate::trig::intercept_angle(Vec2::ZERO, Vec2::new(300.0, 100.0));
        assert_eq!(
            session.aim,
            (toward + 180) % 360,
            "@0x6c88e, turn keys or not"
        );
        assert_eq!(toward, 108);
        // Slower than 0.05 on both axes, it is left alone (@0x6c7d8-0x6c822).
        session.player.velocity = Vec2::new(0.049, -0.049);
        session.tick(REVERSE);
        assert_eq!(session.aim, 288);
        session.player.velocity = Vec2::new(0.0, -0.05);
        session.tick(REVERSE);
        assert_eq!(session.aim, 180);
        // Not without the key.
        session.player.velocity = Vec2::new(0.0, 3.0);
        session.tick(Controls::default());
        assert_eq!(session.aim, 180);
    }

    #[test]
    fn the_second_pass_moves_the_player_again_on_the_same_frame() {
        let catalog = catalog();
        let mut twice = session(&catalog);
        twice.tick(THRUST);
        twice.tick(THRUST);
        let mut doubled = session(&catalog);
        doubled.tick(THRUST);
        again(&mut doubled, THRUST);
        assert_eq!(doubled.player(), twice.player());
        assert_ne!(*doubled.player(), ShipState::default(), "moved");
        assert_eq!((doubled.frame, doubled.pass_frame), (1, 0), "once");
        doubled.tick(THRUST);
        assert_eq!((doubled.frame, doubled.pass_frame), (2, 1));
    }

    #[test]
    fn the_second_pass_moves_the_npcs() {
        let catalog = catalog();
        let mut session = session(&catalog);
        session.tick_traffic(&catalog, &crate::ai::Peaceful, &mut NeverFires);
        session.traffic.add_npc(crate::traffic::npc::Npc {
            state: ShipState {
                velocity: Vec2::new(2.0, 0.0),
                ..ShipState::default()
            },
            ..npc(0, session.stats())
        });
        again(&mut session, Controls::default());
        assert!(session.npcs()[0].state.position.x > 0.0);
    }

    #[test]
    fn a_ship_the_player_leads_moves_at_the_led_top_speed_on_the_second_pass() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        assert!(session.led_speed() > 1.0, "a pilot without Strict Play");
        let escort = &mut session.traffic.npcs_mut()[0];
        escort.mission = Some(crate::traffic::npc::MissionMark {
            slot: 0,
            behav: crate::mission::ships::ShipBehav::EscortPlayer,
            formation: Some(crate::escort::FormationSlot::default()),
            order: None,
        });
        escort.goal = crate::ai::Goal::Formation { guard: None };
        escort.state.position = Vec2::new(0.0, -5000.0);
        let own = escort.stats.handling.max_speed;
        let fastest = (0..200)
            .map(|_| {
                again(&mut session, Controls::default());
                session.npcs()[0].state.velocity.length()
            })
            .fold(0.0, f32::max);
        assert!(fastest > own * 1.01, "{fastest} against its own {own}");
    }

    #[test]
    fn the_second_pass_brings_no_arrivals_and_draws_nothing() {
        let catalog = catalog();
        let mut session = session(&catalog);
        session.tick_traffic(&catalog, &crate::ai::Peaceful, &mut NeverFires);
        session.traffic.table_mut().avg_ships = 3;
        let mut chance = Draws::of(&[]);
        let behaviour = crate::ai::NovaAi::default();
        session.tick_again(
            Controls::default(),
            &behaviour,
            Rules::default(),
            &mut chance,
        );
        assert!(chance.asked.is_empty(), "no arrival rolled");
        assert!(session.npcs().is_empty());
        session.tick_traffic(&catalog, &crate::ai::Peaceful, &mut chance);
        assert!(!chance.asked.is_empty(), "the advance pass rolls");
    }

    #[test]
    fn the_second_pass_fires_and_hits() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.hold_trigger(Trigger {
            primary: true,
            ..Trigger::default()
        });
        session.target = Some(NpcId(0));
        again(&mut session, Controls::default());
        let events = session.take_combat_events();
        assert!(
            events.iter().any(|event| matches!(
                event,
                CombatEvent::Fired {
                    ship: ShipRef::Player,
                    ..
                }
            )),
            "{events:?}"
        );
        let mut ticks = 0;
        while session.npcs()[0].reserves.shield.now >= 30.0 {
            again(&mut session, Controls::default());
            ticks += 1;
            assert!(ticks < 20, "hit");
        }
        // `_HandleShot`'s `_DamageShip` answers it in the pass that hits
        // (@0x35855, @0x36106; see the module docs).
        let npc = &session.npcs()[0];
        assert!(npc.provoked > 0.0, "answered in the pass: {npc:?}");
        assert_eq!(npc.target, Some(ShipRef::Player));
        assert_eq!(session.strikes, [], "none left over");
    }

    #[test]
    fn a_ship_the_player_hits_in_its_gate_on_the_second_pass_is_held_before_that_passs_motion() {
        // `_HandleShot` answers the hit (`_DamageShip` @0x3be6e-0x3be7c) on
        // either pass, before that pass's `_HandleShip` (@0x44483), which
        // counts the hold down.
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.mode = crate::traffic::npc::Mode::Emerging { ticks_left: 60 };
        npc.ai_type = crate::traffic::npc::AiType::Warship;
        // Emerging, it is no target the pass's head keeps (`_HandlePlayer`
        // @0x686a4), so the hit is a stray one: provoked past forgiving
        // through the passes before it.
        npc.stray = 2.0 * crate::ai::answer_flag::STRAY_THRESHOLD;
        session.hold_trigger(Trigger {
            primary: true,
            ..Trigger::default()
        });
        let mut passes = 0;
        while session.npcs()[0].reserves.shield.now >= 30.0 {
            again(&mut session, Controls::default());
            passes += 1;
            assert!(passes < 20, "hit");
        }
        assert_eq!(
            session.npcs()[0].mode,
            crate::traffic::npc::Mode::HeldAtGate { ticks_left: 19 }
        );
    }

    /// Idles, and runs its point defence or not.
    #[derive(Debug)]
    struct Defending(bool);

    impl crate::ai::Behaviour for Defending {
        fn decide(
            &self,
            _npc: &crate::traffic::npc::Npc,
            _around: &crate::ai::Surroundings,
            _chance: &mut dyn Chance,
        ) -> crate::ai::Goal {
            crate::ai::Goal::Idle
        }

        fn trigger(
            &self,
            _npc: &crate::traffic::npc::Npc,
            _around: &crate::ai::Surroundings,
        ) -> Trigger {
            Trigger {
                point_defence: self.0,
                ..Trigger::default()
            }
        }
    }

    #[test]
    fn the_second_pass_runs_no_npc_point_defence() {
        // The NPCs' point defence is `_AIDispatch`'s (`_AIFireTurret`
        // @0x80b0d), which runs on the advance pass alone (@0x4402a).
        let quad = crate::catalog::WeaponRecord {
            id: crate::catalog::WeaponId(133),
            guidance: 9,
            reload: 5,
            count: 12,
            speed: 2000,
            ..crate::session::tests::blaster()
        };
        let missile = crate::catalog::WeaponRecord {
            speed: 500,
            count: 200,
            ..crate::session::tests::homing_missile(134, 1000)
        };
        let catalog = crate::session::tests::arming(missile, quad, 0);
        let mut session = crate::session::tests::with_an_npc_at(&catalog, 750, 550, 180);
        session.target = Some(NpcId(0));
        session.hold_trigger(Trigger {
            primary: true,
            ..Trigger::default()
        });
        // Its point defence off as the player fires.
        session.game_work(
            Controls::default(),
            &catalog,
            &Defending(false),
            Rules::default(),
            &mut NeverFires,
        );
        session.hold_trigger(Trigger::default());
        session.traffic.npcs_mut()[0].trigger.point_defence = true;
        assert!(session.shots().iter().any(|shot| shot.weapon.id.0 == 134));
        session.take_combat_events();
        let quad_fired = |events: &[CombatEvent]| {
            events.iter().any(|event| {
                matches!(
                    event,
                    CombatEvent::Fired {
                        ship: ShipRef::Npc(_),
                        weapon: crate::catalog::WeaponId(133),
                        ..
                    }
                )
            })
        };
        session.tick_again(
            Controls::default(),
            &Defending(true),
            Rules::default(),
            &mut NeverFires,
        );
        assert!(
            !quad_fired(&session.take_combat_events()),
            "the second pass"
        );
        session.game_work(
            Controls::default(),
            &catalog,
            &Defending(true),
            Rules::default(),
            &mut NeverFires,
        );
        assert!(
            quad_fired(&session.take_combat_events()),
            "the advance pass"
        );
    }

    #[test]
    fn the_second_pass_does_nothing_landed_or_once_a_jump_has_begun() {
        let catalog = catalog();
        let mut landed = session(&catalog);
        land_now(&mut landed).expect("lands");
        let before = (*landed.player(), landed.frame, landed.pass_frame);
        again(&mut landed, THRUST);
        assert_eq!((*landed.player(), landed.frame, landed.pass_frame), before);
        let mut jumping = session(&catalog);
        jumping
            .plot_course(crate::catalog::SystemId(131))
            .expect("a route");
        fly_out(&mut jumping);
        begin_jump_now(&mut jumping).expect("jumps");
        let player = *jumping.player();
        again(&mut jumping, THRUST);
        assert_eq!(*jumping.player(), player);
    }

    #[test]
    fn a_jump_that_begins_on_the_second_pass_stops_it_there() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session
            .plot_course(crate::catalog::SystemId(131))
            .expect("a route");
        fly_out(&mut session);
        session.begin_jump().expect("jumps");
        session.traffic.npcs_mut()[0].state.velocity = Vec2::new(2.0, 0.0);
        session.hold_trigger(Trigger {
            primary: true,
            ..Trigger::default()
        });
        session.target = Some(NpcId(0));
        // Brakes and turns on the advance pass until the next pass is
        // ready to jump.
        for steps in 0.. {
            assert!(steps < 1000, "never ready: {:?}", session.player());
            assert_eq!(
                session.preparing_jump(),
                Some(crate::catalog::SystemId(131))
            );
            let mut probe = session.clone();
            again(&mut probe, Controls::default());
            if probe.jumping().is_some() {
                break;
            }
            session.tick(Controls::default());
        }
        session.take_combat_events();
        let npc = session.npcs()[0].state;
        let strikes = session.strikes.clone();
        again(&mut session, Controls::default());
        assert_eq!(session.jumping(), Some(crate::catalog::SystemId(131)));
        assert_eq!(session.npcs()[0].state, npc, "the traffic stood still");
        assert_eq!(session.take_combat_events(), [], "no fight");
        assert_eq!(session.strikes, strikes);
    }
}
