//! A mission's special ship captured: Use As Escort keeps its mark, so it
//! flies as a mission escort, not a hired one. All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386); the findings are on
//! rdm:phase/ship-handling-fidelity/phase-21-captured-mission-ship-keeps-its-mark.
//!
//! **The original.** The escort path (`_DoPlunderDialog`
//! @0x9459b-0x947cf, `_AIShipCapturedByPlayer` @0x89693-0x89780) makes the
//! captured ship AI type 6 (@0x9461f), led by the player (+0x9a = 0,
//! @0x94635), hired by nobody (+0xbb, @0x9464b) and of no government
//! (@0x94690), and never writes its mission mark +0xc8d2. A marked AI
//! type 6 ship the player leads is what a `ShipBehav` 1 ship becomes
//! (`_AIDispatch` @0x8fc03-0x8fc22), so the captured ship is a **mission
//! escort** ([`Npc::mission_escort`](crate::traffic::npc::Npc::mission_escort),
//! see the [`mission_escorts`](super::mission_escorts) module) whatever
//! its mission's `ShipBehav`.
//!
//! **What follows.** Every site the original keys on the mark the engine
//! keys on [`MissionMark`] alone, so keeping the mark is enough:
//! - it takes a slot in the one formation by berth
//!   (`_AICalcFormationPositions` @0x850bc-0x85192, [`Session::reform`])
//!   and its class's standing order (`_EscortAI` @0x83ad8-0x83b2b,
//!   [`Space::class_order`](super::Space::class_order));
//! - it carries no [`EscortDuty`](crate::escort::EscortDuty), has no
//!   escort record and takes no fleet slot, so the hire count
//!   (`_CanHireEscorts` @0x57c2), the plunder sum (@0x92666), the save
//!   (`_WritePilotData` @0x72683), pay, the landing's sell and upgrade, the
//!   escort comm dialog, jettison and fighter recall all leave it out;
//! - its later loss counts as its mission's ship lost (`_HandleShipDisplay`
//!   @0x2daa2-0x2dbf1), though a failed mission, the only one whose ship
//!   reaches the plunder, never runs `OnShipDone` (@0x9ec70-0x9ec8c);
//! - the next system entry deletes it with no count given back
//!   (`_DeleteAllComputerShips` @0x3bf7e-0x3bfb6, @0x3c076-0x3c082);
//! - an abort in flight releases it (`_AbortMission` @0x99ab5-0x99b24);
//! - a `ShipBehav` 0 one still hunts the player
//!   (`_HighLevelAIHandler` @0x8d628-0x8d6ea, after `_EscortAI`
//!   @0x901e5).
//!
//! Use As My Ship leaves the captured slot inactive with its mark set
//! (`_DoShipCapture` @0x41a46); every reader of the mark tests the active
//! flag first, so the stale mark is unobservable, and the engine takes the
//! NPC out of the system.
//!
//! A captured `ShipBehav` 2 ship thinks as `_DeathStarAI`, then
//! `_WarshipAI`, not as `_EscortAI` (`_AIDispatch` @0x8fc2d-0x8fc38,
//! @0x8ff3a-0x8ff43): built by stellar-fidelity phase 2, keyed on the mark
//! ([`death_star`](crate::ai::death_star), the `stellar_attackers`
//! module). Its gate wait and answer to hits stay the escorts'. No stock
//! mission has `ShipBehav` 2.

use super::Session;
use crate::escort::FormationSlot;
use crate::traffic::npc::{AiType, MissionMark, NpcId};

impl Session {
    /// Marked NPC `id`, captured, falls in as a mission escort (see the
    /// module docs): as a hired escort falls in
    /// ([`super::escorts::fall_in`]), keeping its mark, which joins the
    /// formation with its class's standing order, with no escort record
    /// or fleet slot. The formation re-forms, and a save is due.
    pub(super) fn join_as_mission_escort(&mut self, id: NpcId) {
        let Some(npc) = self.space.npc(id) else {
            return;
        };
        let inherent = self
            .statics
            .ship_record(npc.ship)
            .map(|record| AiType::from_raw(record.inherent_ai));
        let order = self
            .space
            .class_order(&self.pilot, &self.statics, npc.class);
        let Some(npc) = self.space.npc_mut(id) else {
            return;
        };
        super::escorts::fall_in(npc, inherent);
        if let Some(mark) = &mut npc.mission {
            *mark = MissionMark {
                formation: Some(FormationSlot::default()),
                order,
                ..*mark
            };
        }
        self.reform();
        self.out.save_due = true;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ai::{Goal, NovaAi};
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::escort::EscortOrder;
    use crate::mission::FailedBy;
    use crate::mission::ships::ShipBehav;
    use crate::pilot::Escort;
    use crate::reserves::Reserves;
    use crate::session::Session;
    use crate::session::mission_ships::tests::{
        accepted, destroy, goal_catalog, in_flight, jump_to, marked, settle, ships_of, take_off,
    };
    use crate::testkit::FakePilotCatalog;
    use crate::traffic::npc::{AiType, NpcId};

    /// Fails slot 0's mission, as only a failed mission's ship reaches
    /// the plunder (`Session::board`).
    fn fail(session: &mut Session) {
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.failed = Some(FailedBy::Quick);
        }
    }

    /// The first marked NPC.
    fn first_marked(session: &Session) -> NpcId {
        session
            .npcs()
            .iter()
            .find(|npc| npc.mission.is_some())
            .expect("a mission ship")
            .id
    }

    /// `_DoPlunderDialog` zeroes the captured ship's fade and direction
    /// (@0x946e3, @0x946f9) before `_AISetup` and
    /// `_AIShipCapturedByPlayer`, so a mission's special ship joining as a
    /// mission escort is zeroed too.
    #[test]
    fn a_special_ship_captured_while_its_cloak_still_fades_joins_at_fade_0() {
        let catalog = goal_catalog(0, 0);
        let mut session = in_flight(&catalog);
        fail(&mut session);
        let id = first_marked(&session);
        session.space.npc_mut(id).expect("here").cloak = crate::cloak::Cloak {
            fade: 12.0,
            direction: -1,
        };
        session.join_fleet(id, &mut NeverFires);
        let npc = session.space.npc(id).expect("here");
        assert!(npc.mission_escort());
        assert_eq!(npc.cloak, crate::cloak::Cloak::default());
    }

    /// Mission 140 of `goal`, in flight, failed, its first ship captured
    /// for an escort: the session and that ship.
    fn captured(catalog: &FakePilotCatalog) -> (Session, NpcId) {
        let mut session = in_flight(catalog);
        fail(&mut session);
        let id = first_marked(&session);
        session.join_fleet(id, &mut NeverFires);
        (session, id)
    }

    /// A hired escort of ship 200.
    fn hired() -> Escort {
        Escort {
            ship: crate::catalog::ShipId(200),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried: false,
            wage: None,
            person: None,
            mark: None,
        }
    }

    #[test]
    fn use_as_escort_keeps_a_mission_ships_mark_and_flies_it_as_a_mission_escort() {
        let catalog = goal_catalog(0, 0);
        let mut session = in_flight(&catalog);
        fail(&mut session);
        session.space.escorts.class_orders = [Some(EscortOrder::Hold); 4];
        let id = first_marked(&session);
        let before = session
            .space
            .npc(id)
            .and_then(|npc| npc.mission)
            .expect("marked");
        session.take_save_due();
        session.join_fleet(id, &mut NeverFires);
        let npc = session.space.npc(id).expect("still here");
        let mark = npc.mission.expect("keeps its mark (@0x9459b-0x947cf)");
        assert_eq!((mark.slot, mark.behav), (before.slot, before.behav));
        assert!(npc.mission_escort());
        assert_eq!(npc.escort, None, "no hired duty");
        assert!(session.pilot().escorts().is_empty(), "no escort record");
        assert!(session.space.escorts.fleet.is_empty(), "no fleet slot");
        assert_eq!(npc.condition, Condition::Intact);
        assert_eq!(npc.reserves.armor.now, npc.reserves.armor.max * 0.5);
        assert_eq!(npc.goal, Goal::Formation { guard: None });
        assert_eq!(npc.target, None);
        assert_eq!(npc.leader, None);
        assert_eq!(npc.govt, None);
        assert_eq!(npc.ai_type, AiType::WimpyTrader, "ship 200's InherentAI");
        let slot = mark.formation.expect("in formation");
        assert_eq!((slot.slot, slot.ships), (2, 2));
        assert_eq!(
            mark.order,
            session
                .space
                .class_order(&session.pilot, &session.statics, npc.class)
        );
        assert_eq!(mark.order, Some(EscortOrder::Hold), "_EscortAI @0x83ad8");
        assert_eq!(marked(&session), [0, 0]);
        assert!(session.take_save_due());
    }

    #[test]
    fn a_captured_mission_escort_shares_the_formation_and_is_not_counted_or_saved() {
        let catalog = goal_catalog(0, 0);
        let mut session = accepted(&catalog);
        session.pilot.escorts.push(hired());
        take_off(&mut session, &catalog);
        fail(&mut session);
        let hired_id = session.space.escorts.fleet[0].expect("placed");
        let counted = session.space.led_escorts(&session.pilot);
        let can_hire = session.can_hire_escorts();
        let id = first_marked(&session);
        session.join_fleet(id, &mut NeverFires);
        let mut led: Vec<(u32, u8, u8)> = session
            .player_led()
            .filter_map(|npc| {
                let place = npc.formation()?;
                Some((npc.berth, place.slot, place.ships))
            })
            .collect();
        led.sort_unstable();
        assert_eq!(led.len(), 2, "the hired escort and the captured one");
        let berths: Vec<u32> = led.iter().map(|&(berth, _, _)| berth).collect();
        assert_eq!(
            led.iter()
                .map(|&(_, slot, ships)| (slot, ships))
                .collect::<Vec<_>>(),
            [(2, 3), (3, 3)],
            "slots by berth: {berths:?}"
        );
        assert!(
            session
                .space
                .npc(hired_id)
                .and_then(|npc| npc.escort)
                .is_some()
        );
        assert_eq!(
            session.space.led_escorts(&session.pilot),
            counted,
            "_CanHireEscorts @0x57c2"
        );
        assert_eq!(session.can_hire_escorts(), can_hire);
        let saved = crate::save::encode(session.pilot());
        let loaded = crate::save::decode(&saved).expect("loads");
        assert_eq!(loaded.escorts(), [hired()], "_WritePilotData @0x72683");
    }

    #[test]
    fn a_captured_mission_ship_lost_later_is_counted_as_its_missions_ship() {
        for goal in [0, 1] {
            let catalog = goal_catalog(goal, 0);
            let (mut session, id) = captured(&catalog);
            let ships = ships_of(&session);
            let _ = session.take_sounds();
            let index = session
                .npcs()
                .iter()
                .position(|npc| npc.id == id)
                .expect("here");
            destroy(&mut session, index);
            let after = ships_of(&session);
            assert_eq!(after.left, ships.left - 1, "goal {goal}: @0x2dbe7");
            assert_eq!(
                after.destroyed,
                ships.destroyed + 1,
                "goal {goal}: @0x2dbd4"
            );
        }
    }

    /// Mission 140 of a destroy goal, in flight, its first ship captured
    /// for an escort, its mission failed first when `failed`; its other
    /// ship is destroyed, and then the captured one, the last kill the
    /// goal needs. Whether `OnShipDone` (b6) ran.
    fn ship_done_after_losing_the_captured(failed: bool) -> bool {
        let catalog = goal_catalog(0, 0);
        let mut session = in_flight(&catalog);
        if failed {
            fail(&mut session);
        }
        let id = first_marked(&session);
        session.join_fleet(id, &mut NeverFires);
        let other = session
            .npcs()
            .iter()
            .position(|npc| npc.mission.is_some() && npc.id != id)
            .expect("the other ship");
        destroy(&mut session, other);
        settle(&mut session, &catalog);
        assert_eq!(ships_of(&session).left, 1, "failed: {failed}");
        let index = session
            .npcs()
            .iter()
            .position(|npc| npc.id == id)
            .expect("here");
        destroy(&mut session, index);
        settle(&mut session, &catalog);
        assert_eq!(ships_of(&session).left, 0, "failed: {failed}");
        session.pilot().bits.iter().any(|bit| bit.get() == 6)
    }

    #[test]
    fn a_captured_mission_ships_loss_completing_a_failed_missions_goal_runs_no_ship_done() {
        assert!(
            !ship_done_after_losing_the_captured(true),
            "no OnShipDone for a failed mission (@0x9ec70-0x9ec8c)"
        );
        assert!(
            ship_done_after_losing_the_captured(false),
            "the control: the same losses on a mission not failed run it"
        );
    }

    #[test]
    fn a_captured_mission_escort_is_gone_at_the_next_system_entry() {
        let catalog = goal_catalog(0, 0);
        let mut session = in_flight(&catalog);
        let left = ships_of(&session).left;
        fail(&mut session);
        let id = first_marked(&session);
        session.join_fleet(id, &mut NeverFires);
        jump_to(&mut session, &catalog, 131);
        assert!(
            session.npcs().iter().all(|npc| !npc.mission_escort()),
            "_DeleteAllComputerShips @0x3bf9c"
        );
        assert!(session.pilot().escorts().is_empty());
        assert!(session.space.escorts.fleet.iter().all(Option::is_none));
        assert_eq!(ships_of(&session).left, left, "no count changed (@0x3c082)");
    }

    #[test]
    fn an_abort_in_flight_releases_a_captured_mission_escort() {
        let catalog = goal_catalog(0, 0);
        let (mut session, id) = captured(&catalog);
        session.release_mission_ships(0);
        let npc = session.space.npc(id).expect("still here, leaving");
        assert_eq!(npc.mission, None, "_AbortMission @0x99ab5-0x99b24");
        assert_eq!(npc.formation(), None);
        assert!(!npc.escorts_player());
        assert_eq!(npc.ai_type, AiType::WimpyTrader);
        assert!(
            matches!(npc.goal, Goal::JumpOut | Goal::Idle),
            "it leaves: {:?}",
            npc.goal
        );
    }

    #[test]
    fn a_captured_attack_player_mission_escort_still_hunts_the_player() {
        let mut catalog = goal_catalog(0, 0);
        catalog.missions[0].ships.behav = 0;
        let (mut session, id) = captured(&catalog);
        assert_eq!(
            session
                .space
                .npc(id)
                .and_then(|npc| npc.mission)
                .map(|mark| mark.behav),
            Some(ShipBehav::AttackPlayer)
        );
        assert!(
            session
                .space
                .npc(id)
                .is_some_and(crate::traffic::npc::Npc::mission_escort)
        );
        let ai = NovaAi::default();
        for _ in 0..40 {
            session.tick_traffic(&catalog, &ai, &mut NeverFires);
        }
        let npc = session.space.npc(id).expect("here");
        assert_eq!(
            npc.goal,
            Goal::Attack(crate::combat::ShipRef::Player),
            "_HighLevelAIHandler @0x8d628-0x8d6ea"
        );
    }

    #[test]
    fn use_as_my_ship_of_a_mission_ship_changes_no_count() {
        let catalog = goal_catalog(0, 0);
        let mut session = in_flight(&catalog);
        fail(&mut session);
        let id = first_marked(&session);
        let ship = session.space.npc(id).map(|npc| npc.ship).expect("here");
        let ships = ships_of(&session);
        session.space.escorts.aboard = Some(crate::session::Aboard {
            npc: id,
            ship,
            plunder: crate::board::Plunder::default(),
            captured: true,
        });
        let _ = session.take_sounds();
        let _ = session.take_messages();
        assert_eq!(
            session.assign(crate::board::Assignment::MyShip, &mut NeverFires),
            Some(crate::board::Assigned::MyShip)
        );
        assert!(session.space.npc(id).is_none(), "taken out (@0x41a46)");
        let after = ships_of(&session);
        assert_eq!(
            (after.left, after.destroyed, after.disabled, after.boarded),
            (ships.left, ships.destroyed, ships.disabled, ships.boarded)
        );
        assert!(
            !session.take_sounds().iter().any(|sound| matches!(
                sound,
                crate::SimSound::MissionShipLost | crate::SimSound::MissionShipBoarded
            )),
            "no mission-ship fate"
        );
        assert!(session.take_messages().is_empty(), "no loss shown");
    }
}
