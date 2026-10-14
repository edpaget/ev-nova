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
//! Lock press, which can drift from the lock itself
//! (rdm:phase/hud-fidelity/phase-21-double-time-follows-the-caps-lock-state).
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
//!   its departures, fighters docking and targets lost, then the fight
//!   ([`GamePass::Again`]) with all that follows from its hits. A due frame
//!   therefore steps the junk and the hints twice, as the original does.
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
//!
//! **Ordering.** The original draws its frame between the two passes
//! (@0x45d96-0x45e3e); the view draws after both. Every frame shown is
//! still one counter tick apart from the last, so the difference is a
//! constant phase of one pass that no player can see.

use super::Session;
use crate::chance::Chance;
use crate::combat::{GamePass, Rules};
use crate::flight::Controls;
use crate::traffic::World;

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
    /// pass's calls stand still; a jump that begins on it stops it there.
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
        if self.jumping().is_some() {
            return;
        }
        self.move_traffic_again();
        self.fight(GamePass::Again, rules, chance);
    }

    /// The traffic's part of the second pass: the NPCs move
    /// ([`Traffic::move_in`](crate::traffic::Traffic::move_in)) among the
    /// system's stellars and the player, at the speed the player leads
    /// them at, which is all their motion reads; each mission ship that
    /// left raises its fate, and fighters dock. The fight that follows
    /// lets go of a target gone and orphans the fighters of a carrier
    /// gone, as it does after any departure, before anything is drawn.
    /// Nothing moves before the system is populated.
    fn move_traffic_again(&mut self) {
        if self.traffic_due {
            return;
        }
        let world = World {
            player: Some(self.player_side()),
            led_speed: self.led_speed(),
            ..World::new(&self.sites)
        };
        let marks = self.mission_marks();
        self.traffic.move_in(world);
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
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::{
        Draws, FakePilotCatalog, begin_jump_now, catalog, fly_out, land_now, npc,
    };
    use crate::traffic::npc::NpcId;

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
