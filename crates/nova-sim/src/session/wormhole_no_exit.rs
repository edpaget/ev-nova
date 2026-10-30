//! A wormhole that leads nowhere still runs the dock request's tail
//! (play-feel-4 phase 16). All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! **The original.** When `_PlayerEnterWormhole` finds no active exit
//! (@0x6409a-0x6409d → 0x6461c), it says #84 + #86 (`_DisplayComm`
//! @0x646a3, 250 ticks), plays `_beepSnd[3]`, `snd ` 153 (`PlaySound`
//! @0x646cb, priority 1), and returns -1 (@0x646d0), before its
//! starfield, particles, course restart, cloak fade and placement.
//! `_HandlePlayerDockRequest` stores -1 in both its exit locals
//! (@0x66f17-0x66f20) and, unlike a hypergate's -1 (@0x66ef2-0x66f08),
//! jumps to the tail at 0x66f40 untested. In the system the player stays
//! in, the tail:
//!
//! - kills the shots, boxes, pending explosions, smoke and asteroids
//!   (`_killShots`, `_killBoxes`, `_killExplods`, `_killSmoke`,
//!   `_killAsteroids` @0x66f40-0x66f6f) and every beam (`_beamData` +0x10
//!   = -1, @0x66fc4-0x66fe6);
//! - folds the ship as a take-off does and clears its bank, display
//!   timers, ionization, glow and boarded flag (@0x66f72-0x67020;
//!   [`Session::begin_dock_tail`], shared with the take-off and a gate's
//!   exit in the `dock_tail` module);
//! - shuts the escort menu, drops the dock request and lets go of the
//!   target (@0x6705a-0x67069), and arms the escort-strength timer
//!   (@0x6706f-0x67084);
//! - deletes the computer ships and resets the player
//!   (`_DeleteAllComputerShips(0, 1)` @0x67096, @0x6709b), skips the
//!   gate-exit block (ebp-0x2a3, -0x2a2 and -0x2a1 all 0, @0x67122-0x67153),
//!   sets the system up as a take-off does (`_SetupShipsInSystem(+0x74, 1)`
//!   @0x674c2) and rebuilds the stellars (@0x674c7);
//! - lets go of the nav selection, keeping the mode (@0x674d2), cues the
//!   course (@0x674d8), resets every stellar's frames with no exit gate
//!   (@0x675ce-0x675fa), prepares the sprites around a rebuild
//!   (@0x6764e-0x67658) and starts the take-off grace (`_frameCounter` =
//!   -15, @0x67673).
//!
//! The ship keeps its place, velocity and thrust; the course isn't
//! restarted, the system isn't explored, no day passes, nothing is saved,
//! and no arrival sound or message plays.
//!
//! **Here.** [`Session::enter_wormhole`] beeps [`SimSound::Refused`] and
//! runs [`Session::refuse_no_exit`], which runs the tail's two shared
//! halves ([`Session::begin_dock_tail`], [`Session::end_dock_tail`]; see
//! the `dock_tail` module) around its own population, telling
//! [`SessionEvent::SystemChanged`] for the ships set up afresh (no
//! [`SessionEvent::StarfieldReset`]) and
//! [`SessionEvent::StellarFramesReset`]. The view says the message, shuts
//! the escort menu and lays the scene out afresh from those events, which
//! lets go of the smoke, the effects and the display timers.
//!
//! **Left out.** `_SpawnBountyHunter` (@0x674dd): the engine models no
//! bounty hunter at any of its call sites (task `planet-domination`).
//! The screen's redraws (`_statusRedraw`, `_LoadIntfResource`, the
//! background, `_RedrawComm`, `_RefreshSAT`, `_RedrawStatus`,
//! `_CheckLoMem`; @0x6702a-0x6703d, @0x670c8-0x6711d, @0x6765d-0x6767d):
//! the view draws every frame. `_ClearStarfield` (@0x670a0-0x670c3), run
//! only in a system of negative Murk, whose stars are already hidden.
//! `_AdjustSoundVolume` (@0x6766e) plays nothing.

use super::{Session, SessionEvent};
use crate::catalog::{PilotCatalog, TrafficCatalog};
use crate::chance::Chance;
use crate::sound::SimSound;

impl Session {
    /// Refuses a wormhole that leads nowhere, beeping, and runs the dock
    /// request's tail in the system the player stays in, populated afresh
    /// from `catalog` on `chance` (see the module docs).
    ///
    /// The tail runs through [`Session::begin_dock_tail`] and
    /// [`Session::end_dock_tail`], as [`Session::take_off`]'s does; see
    /// the `dock_tail` module for the order.
    pub(super) fn refuse_no_exit(
        &mut self,
        catalog: &(impl PilotCatalog + TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) {
        // `PlaySound` @0x646cb, before the -1.
        self.out.sounds.push(SimSound::Refused);
        // The kill flags (@0x66f40-0x66f6f) and the beams
        // (@0x66fc4-0x66fe6).
        self.space.combat.clear();
        self.kill_explosions();
        self.space.clear_boxes();
        self.space.kill_asteroids();
        // Player +0x70 = -1 (@0x67069), which neither the fold nor the
        // reset reads.
        self.chart.target = None;
        // The fold, the player block, the strength timer and the reset
        // (@0x66f72-0x6709b; see the `dock_tail` module).
        self.begin_dock_tail(chance);
        // `_DeleteAllComputerShips(0, 1)` (@0x67096) and
        // `_SetupShipsInSystem(+0x74, 1)` (@0x674c2), as a take-off.
        self.populate(catalog, chance);
        self.out.emit(SessionEvent::SystemChanged);
        // The frames' reset, no exit gate (@0x675ce-0x675fa).
        self.reset_stellar_frames(None);
        // The rebuild, the sprites, the dock request dropped and the
        // course cued (see the `dock_tail` module).
        self.end_dock_tail();
        // `_frameCounter` = -15 (@0x67673).
        self.start_takeoff_grace();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::SimMessage;
    use crate::catalog::{SystemId, WeaponRecord};
    use crate::chance::NeverFires;
    use crate::combat::CombatEvent;
    use crate::combat::armament::{Fold, Trigger};
    use crate::combat::ion::Ion;
    use crate::combat::weapon::Explosion;
    use crate::flight::Controls;
    use crate::gate::{GateKind, GateRefusal, WORMHOLE};
    use crate::geometry::Vec2;
    use crate::landing::LandOutcome;
    use crate::navigation::NavMode;
    use crate::session::tests::{
        FIRE, armed, at_gate, at_wormhole, blaster, fleet_escort, gate_site, gated, wormholes,
    };
    use crate::session::{LandPress, Session, SessionEvent};
    use crate::sound::SimSound;
    use crate::stats::ShipStats;
    use crate::story::StoryEvent;
    use crate::testkit::{FakePilotCatalog, Scripted, npc, work};

    /// [`wormholes`] with wormhole 400, the one the ship starts over,
    /// the only gate: it leads nowhere.
    fn nowhere() -> FakePilotCatalog {
        FakePilotCatalog {
            gates: vec![gate_site(400, (0.0, 0.0), WORMHOLE, &[], 0)],
            ..wormholes()
        }
    }

    /// `catalog`'s session over its wormhole, its sounds taken.
    fn over(catalog: &FakePilotCatalog) -> Session {
        let mut session = at_wormhole(catalog);
        session.take_sounds();
        session
    }

    #[test]
    fn a_wormhole_with_no_exit_beeps_the_refusal_once() {
        // `_PlayerEnterWormhole` plays `_beepSnd[3]` (`snd ` 153) before
        // its -1 (@0x646cb).
        let catalog = nowhere();
        let mut session = over(&catalog);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut NeverFires),
            Err(GateRefusal::NoExit)
        );
        assert_eq!(session.take_sounds(), [SimSound::Refused]);
    }

    #[test]
    fn no_entry_pending_beeps_nothing() {
        let catalog = nowhere();
        let mut session = Session::start(&catalog).expect("starts");
        session.take_sounds();
        assert_eq!(
            session.enter_wormhole(&catalog, &mut NeverFires),
            Err(GateRefusal::NotAtGate)
        );
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn a_refused_hypergate_beeps_nothing() {
        // `_PlayerEnterHypergate` plays no beep around its map (@0x6385b).
        let catalog = gated();
        let mut session = at_gate(&catalog);
        session.take_sounds();
        assert_eq!(
            session.enter_hypergate(None, &catalog, &mut NeverFires),
            Err(GateRefusal::Cancelled)
        );
        assert_eq!(session.take_sounds(), [], "cancelled");
        let catalog = FakePilotCatalog {
            gates: vec![gate_site(
                300,
                (0.0, 0.0),
                crate::gate::HYPERGATE,
                &[0, -2],
                0,
            )],
            ..gated()
        };
        let mut session = at_gate(&catalog);
        session.take_sounds();
        assert_eq!(session.open_hypergate(&catalog), Err(GateRefusal::NoLinks));
        assert_eq!(session.take_sounds(), [], "no links");
        assert_eq!(session.system(), SystemId(130));
    }

    // The tail.

    /// [`nowhere`] laid over [`armed`], the ship and its traffic each
    /// carrying `weapon`: system 130 has traffic of ship 129.
    fn fought(weapon: WeaponRecord) -> FakePilotCatalog {
        let holes = nowhere();
        FakePilotCatalog {
            sites: holes.sites,
            gates: holes.gates,
            gate_listers: holes.gate_listers,
            weapons: vec![weapon],
            ..armed()
        }
    }

    /// Presses L over the wormhole, runs the dock approach and presses L
    /// again, as [`at_wormhole`] does, then takes the sounds and events.
    fn press_over(session: &mut Session) {
        assert!(matches!(
            session.land(),
            Ok(LandPress::Outcome(LandOutcome::Requested { .. }))
        ));
        session.dock_approach();
        assert!(matches!(
            session.land(),
            Ok(LandPress::AtGate {
                kind: GateKind::Wormhole,
                ..
            })
        ));
        session.take_sounds();
        session.take_events();
    }

    /// `catalog`'s session having fired its weapon once, over the
    /// wormhole.
    fn fired_over(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.hold_trigger(FIRE);
        work(&mut session, catalog, Controls::default());
        session.hold_trigger(Trigger::default());
        press_over(&mut session);
        session
    }

    fn refuse(session: &mut Session, catalog: &FakePilotCatalog) {
        assert_eq!(
            session.enter_wormhole(catalog, &mut NeverFires),
            Err(GateRefusal::NoExit)
        );
    }

    #[test]
    fn the_tail_kills_the_shots_and_beams() {
        // `_killShots` (@0x66f40-0x66f6f) and every `_beamData` record's
        // +0x10 = -1 (@0x66fc4-0x66fe6).
        let laser = WeaponRecord {
            guidance: 0,
            count: 5,
            beam_length: 100,
            ..blaster()
        };
        for weapon in [blaster(), laser] {
            let catalog = fought(weapon);
            let mut session = fired_over(&catalog);
            assert!(
                !session.shots().is_empty() || !session.beams().is_empty(),
                "fired"
            );
            refuse(&mut session, &catalog);
            assert_eq!(session.shots(), []);
            assert!(session.beams().is_empty());
        }
    }

    #[test]
    fn the_tail_kills_the_boxes_the_asteroids_and_the_explosions_pending() {
        // `_killBoxes`, `_killAsteroids`, `_killExplods` (@0x66f40-0x66f6f).
        let catalog = nowhere();
        let mut session = over(&catalog);
        session.drop_cargo_boxes(10, 10, &mut NeverFires);
        assert!(!session.cargo_boxes().is_empty(), "dropped");
        session
            .space
            .combat
            .explode(Vec2::ZERO, Explosion::decode(4).expect("bööm 132"));
        assert_eq!(session.space.asteroid_kill, None);
        refuse(&mut session, &catalog);
        assert!(session.cargo_boxes().is_empty());
        assert!(session.space.asteroid_kill.is_some(), "the field killed");
        assert!(
            !session
                .take_combat_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::Exploded { .. })),
            "the explosion dropped"
        );
    }

    #[test]
    fn the_tail_lets_go_of_the_target_and_the_dock_request_and_arms_the_strength_timer() {
        // `_dockPermission` = 0 (@0x6705f), player +0x70 = -1 (@0x67069),
        // `_antiEscortDelay` = Rand(30) + 30 (@0x6706f-0x67084).
        let catalog = nowhere();
        let mut session = over(&catalog);
        let target = session.space.traffic.add_npc(npc(0, ShipStats::default()));
        session.chart.target = Some(target);
        assert!(!session.space.traffic.strength_armed());
        refuse(&mut session, &catalog);
        assert_eq!(session.chart.target, None);
        assert_eq!(session.port.dock, None);
        assert!(session.space.traffic.strength_armed());
    }

    #[test]
    fn the_tail_sets_the_system_up_afresh_with_the_escorts_beside_the_player() {
        // `_DeleteAllComputerShips(0, 1)` (@0x67096), then
        // `_SetupShipsInSystem(+0x74, 1)` (@0x674c2), as a take-off.
        let catalog = fought(blaster());
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.escorts = vec![fleet_escort(129)];
        session.populate(&catalog, &mut NeverFires);
        press_over(&mut session);
        let stray = Vec2::new(777.0, 777.0);
        let mut ship = npc(0, ShipStats::default());
        ship.state.position = stray;
        session.space.traffic.add_npc(ship);
        let escort = session.space.escorts.fleet[0].expect("placed");
        session
            .space
            .traffic
            .npcs_mut()
            .iter_mut()
            .find(|npc| npc.id == escort)
            .expect("there")
            .state
            .position = Vec2::new(-500.0, 500.0);
        refuse(&mut session, &catalog);
        assert!(
            session.npcs().iter().all(|npc| npc.state.position != stray),
            "the old ships deleted"
        );
        let escort = session.space.escorts.fleet[0].expect("kept");
        let escort = session.space.npc(escort).expect("in the system");
        let duty = escort.escort.expect("an escort");
        assert_eq!(
            escort.state.position,
            duty.formation.position(session.player()),
            "re-snapped beside the player"
        );
        assert_eq!(session.system(), SystemId(130));
    }

    #[test]
    fn the_tail_tells_the_scene_set_up_afresh_and_the_frames_reset_and_no_starfield() {
        let catalog = nowhere();
        let mut session = over(&catalog);
        session.take_events();
        refuse(&mut session, &catalog);
        assert_eq!(
            session.take_events(),
            [
                SessionEvent::SystemChanged,
                SessionEvent::StellarFramesReset
            ]
        );
    }

    #[test]
    fn the_tail_leaves_the_ship_where_it_is_and_clears_its_block() {
        // Fold (@0x66f72-0x66fc4); bank +0xc8e4, ion charge +0x5c, glow
        // +0xc8d4, boarded +0xb9 and ion colour +0xb0 = 0
        // (@0x66fe8-0x67020). No placement, no velocity, no thrust stop.
        let catalog = nowhere();
        let mut session = over(&catalog);
        session.ship.player.velocity = Vec2::new(3.0, -4.0);
        session.ship.player.position = Vec2::new(10.0, 20.0);
        session.ship.thrusting = true;
        session.ship.engine_glow = 20;
        session.ship.ship_display.frames.bank = 5.0;
        session.ship.fate.player_boarded = true;
        session.ship.ion = Ion {
            charge: 50.0,
            colour: 0x00ff_0000,
            ..Ion::default()
        };
        *session.ship.fit.armament.fold_mut() = Fold {
            set: 3,
            dir: 0,
            ..session.ship.fit.armament.fold()
        };
        let before = *session.player();
        refuse(&mut session, &catalog);
        assert_eq!(*session.player(), before);
        assert!(session.ship.thrusting, "still thrusting");
        assert_eq!(session.ship.engine_glow, 0);
        assert_eq!(session.ship.ship_display.frames.bank, 0.0);
        assert!(!session.ship.fate.player_boarded);
        assert_eq!(session.ship.ion, Ion::default());
        let fold = session.ship.fit.armament.fold();
        assert_eq!((fold.set, fold.dir), (0, 1), "the take-off's fold");
    }

    #[test]
    fn the_tail_keeps_the_system_and_the_course_and_cues_it_again() {
        // +0x6c = -1 (@0x674d2), then `_CueNextHyperRouteDest` (@0x674d8).
        let catalog = nowhere();
        let mut session = over(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        session.chart.nav = NavMode::Stellar(Some(crate::catalog::StellarId(400)));
        refuse(&mut session, &catalog);
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.course(), [SystemId(131)]);
        assert_eq!(
            session.nav_mode(),
            NavMode::Hyperspace(Some(SystemId(131))),
            "mode 3 cued after the deselect"
        );
    }

    #[test]
    fn the_tail_explores_nothing_says_nothing_passes_no_day_and_saves_nothing() {
        let catalog = nowhere();
        let mut session = over(&catalog);
        session.take_messages();
        session.take_save_due();
        let date = session.date();
        let raised = session.story_raised();
        refuse(&mut session, &catalog);
        assert_eq!(session.take_messages(), Vec::<SimMessage>::new());
        assert_eq!(session.take_sounds(), [SimSound::Refused], "no arrival");
        assert!(!session.take_save_due());
        assert_eq!(session.date(), date);
        assert_eq!(session.story_raised(), raised, "no explore, no entry");
        assert!(
            !session
                .story_raised()
                .iter()
                .any(|event| matches!(event, StoryEvent::EnteredSystem { .. }))
        );
    }

    // The gate exits' share of the dock request's tail.

    /// `session` with its ion charged, its fold at set 3 and an explosion
    /// pending.
    fn charged(session: &mut Session) {
        session.ship.ion = Ion {
            charge: 50.0,
            colour: 0x00ff_0000,
            ..Ion::default()
        };
        *session.ship.fit.armament.fold_mut() = Fold {
            set: 3,
            dir: 0,
            ..session.ship.fit.armament.fold()
        };
        session
            .space
            .combat
            .explode(Vec2::ZERO, Explosion::decode(4).expect("bööm 132"));
    }

    fn assert_cleared(session: &mut Session, kind: GateKind) {
        assert_eq!(session.ship.ion, Ion::default(), "{kind:?}");
        let fold = session.ship.fit.armament.fold();
        assert_eq!((fold.set, fold.dir), (0, 1), "{kind:?}");
        assert!(
            !session
                .take_combat_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::Exploded { .. })),
            "{kind:?}: `_killExplods` (@0x66f63)"
        );
    }

    #[test]
    fn a_gates_exit_clears_the_ion_folds_as_a_take_off_and_drops_the_explosions() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        charged(&mut session);
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
            .expect("enters");
        assert_cleared(&mut session, GateKind::Hypergate);
        let catalog = wormholes();
        let mut session = at_wormhole(&catalog);
        charged(&mut session);
        session
            .enter_wormhole(&catalog, &mut Scripted::rolling(&[0]))
            .expect("enters");
        assert_cleared(&mut session, GateKind::Wormhole);
    }
}
