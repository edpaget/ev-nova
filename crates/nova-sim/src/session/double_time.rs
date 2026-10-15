//! Double time: the original's mode that runs a second, partial pass of
//! the game's work on each frame, so the game plays at about twice the
//! speed.
//!
//! **The switch** (`_PlayGame` @0x463b3-0x4643e). Every iteration of the
//! main loop, after the frame counter moves on (@0x45f36) and the mission
//! sound plays (@0x462fa-0x46375), sets `_doubleTime` from Caps Lock's
//! lock state (`_QuickKeyCheck(0x39)` @0x463c3, reading the `GetKeys`
//! map @0xa0f5). Ours takes the key's state from its caller
//! ([`Session::set_double_time_key`]): the view toggles it on each Caps
//! Lock press, an accepted difference: it can drift from the lock itself
//! when pressed while unfocused or already on at launch, and on macOS,
//! where winit 0.30.13 reports no Caps Lock key, there is no double time.
//! The switch's hold through a warp begun at double speed, and its
//! release through an aligned warp begun at single speed (@0x463b3-0x463c1,
//! @0x463d3-0x4640c), read the warp-up sounds' play state, which ours does
//! not model (rdm:phase/hud-fidelity/phase-20-audio-double-speed-warp).
//! `_gameFrozen`'s clearing (@0x46427, @0x4642f-0x4643e) is a cheat's, and
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
//!   and boxes among them (@0x444d2-0x44513). So here the player's part of
//!   [`Session::tick`] runs again on that frame, then the traffic's
//!   motion ([`Traffic::move_in`](crate::traffic::Traffic::move_in)) with
//!   its departures, fighters docking and targets lost, then
//!   `_HandlePlayer`'s sabotage clock and eject check (see the `sabotage`
//!   and `eject` modules), then the fight ([`GamePass::Again`]) with all
//!   that follows from its hits. A due frame therefore steps the junk and
//!   the hints twice, as the original does, and the sabotage clock, the
//!   escape pod's countdown, the breakup's countdown and its klaxon run
//!   on both passes too.
//! - On the advance pass alone (`arg == 0` jumps from @0x4402a to
//!   @0x44350): `_HandleStatus`, `_MissionHandler`, `_HandleVoiceSound`,
//!   `_HandleComm`, `_CalculateTargetedDamage`, `_EnterMoreShips` (the
//!   arrivals and the escort-strength timer), `_AIDispatch` for every ship
//!   (@0x44270-0x442ae: the decisions, the cloak thinks, and the scans,
//!   `_ScanPlayer` running under `_HighLevelAIHandler` @0x8f38a), and
//!   `_HandleAlertSprite` (@0x44518), which steps the Red Alert's blink.
//!   So the second pass plays no mission sound, rolls no arrivals, decides
//!   nothing, takes no scan, hails nothing and does not move the AI's
//!   timer; strikes it raises wait for the next advance pass's answers.
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
use crate::chance::Chance;
use crate::combat::{GamePass, Rules};
use crate::flight::Controls;

impl Session {
    /// Takes the double-time key's state: `on` while double time should
    /// run (see the module docs). Off until it says otherwise.
    pub fn set_double_time_key(&mut self, on: bool) {
        self.double_time = on;
    }

    /// Whether double time is on.
    #[must_use]
    pub fn double_time(&self) -> bool {
        self.double_time
    }

    /// Runs double time's second pass under the player's `controls`, the
    /// fight by `rules`, drawing on `chance` (see the module docs). Its
    /// caller runs it after the advance pass's calls ([`Session::tick`]
    /// to [`Session::tick_combat`]) while [`Session::double_time`] is on.
    /// Landed, or once a jump has begun, it does nothing, as the advance
    /// pass's calls stand still; a jump that begins on it stops it there,
    /// and so does the escape pod's countdown running out on it, the
    /// rescue then due ([`Session::rescue_if_due`], which its caller asks
    /// after this pass as after the advance pass's tick).
    pub fn tick_again(
        &mut self,
        controls: Controls,
        rules: Rules,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if self.landed.is_some() || self.jumping().is_some() {
            return;
        }
        self.fly_player(self.pass_frame, controls, GamePass::Again);
        if self.jumping().is_some() || self.rescue_due {
            return;
        }
        self.move_traffic_again(&mut *chance);
        self.before_fight(rules, &mut &mut *chance);
        self.fight(GamePass::Again, rules, chance);
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
    fn move_traffic_again(&mut self, chance: &mut (impl Chance + ?Sized)) {
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
    };

    fn session(catalog: &FakePilotCatalog) -> Session {
        Session::start(catalog).expect("starts")
    }

    fn again(session: &mut Session, controls: Controls) {
        session.tick_again(controls, Rules::default(), &mut NeverFires);
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
        // double time: 129 sounds all the same.
        let catalog = with_warp_up_x2();
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 0.0);
        session.begin_jump().expect("jumps");
        assert_eq!(session.take_sounds(), [SimSound::JumpBegan]);
        given_up_by_a_move(&mut session, &catalog);
        session.set_double_time_key(true);
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
        assert!(session.double_time());
        session.set_double_time_key(false);
        assert!(!session.double_time());
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
        session.tick_again(Controls::default(), Rules::default(), &mut chance);
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
        assert!(
            session
                .strikes
                .iter()
                .any(|strike| strike.by == ShipRef::Player && strike.ship == ShipRef::Npc(NpcId(0))),
            "kept for the advance pass's answers: {:?}",
            session.strikes
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
