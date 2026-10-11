//! Mission escorts: the special ships of a `ShipBehav` 1 mission flying
//! with the player (see the [`mission_ships`](super::mission_ships)
//! module for their lifetime, and [`escorts`](super::escorts) for the
//! hired escorts).
//!
//! **Joining.** The original makes a `ShipBehav` 1 ship an escort led by
//! the player, AI type 6 and leader 0: as the system is set up
//! (`_SetupShipsInSystem` @0x4309f-0x430ef), and on every think while its
//! mission is in its slot and it is not disabled (`_AIDispatch`
//! @0x8fc03-0x8fc22), which is how a ship that comes in on the appear
//! timer or is swapped in for a person joins. Here a mission's special
//! ship of [`ShipBehav::EscortPlayer`], intact, its slot holding its
//! mission, joins the player's formation
//! ([`MissionMark::formation`](crate::traffic::npc::MissionMark)) as it
//! comes by any of the three paths ([`Session::mission_ships_came`]) and
//! on every traffic tick, keeping formation ([`Goal::Formation`]). It is
//! a **mission escort** ([`Npc::mission_escort`]): of the player's fleet
//! ([`Npc::fleet`]), flown by the escorts' AI with no standing order (see
//! [`ai::escort`](crate::ai::escort)), and never one of the hired escorts:
//! it carries no [`EscortDuty`](crate::escort::EscortDuty), so saving,
//! pay, the escort landing screen, the escort comm dialog, jettison,
//! fighter recall and the hiring room, which read the pilot's escorts or
//! the session's fleet, all leave it out, as the original leaves out
//! every marked ship there. A ship disabled before it joins does not join
//! until it is intact again; one disabled once joined keeps its place, as
//! the original leaves a disabled ship's leader as it was (@0x8fc0c), and
//! its `_HandleShip` releases only an unmarked disabled escort
//! (@0x339a6). A person swapped for a ship of a `ShipGoal` 3 mission of
//! another `ShipBehav` never joins: the swap's `_AIMakeEscortFlyInForm`
//! (@0x62509) follows its leader, which `_GenericRandomShipSpawn` left at
//! -1 (@0x3c9cd), so it never forms on the player.
//!
//! **One formation.** The hired escorts and the mission escorts share one
//! formation with the player, as the original numbers every ship whose
//! formation parent is the player in one count and spacing
//! (`_AICalcFormationPositions` @0x850bc-0x85192). Here the hired escorts
//! take slots 2, 3, … in fleet order, then the mission escorts in the
//! order they came (by [`NpcId`]); the original's order is ship-table
//! order across both. The formation re-forms whenever one joins or
//! leaves. Set up at take-off or as a pilot is flown, a mission escort
//! sits on its slot with the player's heading and velocity
//! (`_AICalcFormationPositions(player, 1)` @0x4319a-0x431b2); set up on a
//! jump's arrival or a gate's exit it stays where it was set up and flies
//! to its slot.
//!
//! **Following.** The original's setup first deletes every mission escort
//! (`_DeleteAllComputerShips` @0x3bf7e-0x3bfb6), so its "already with the
//! player" count (@0x42e3f-0x42e83) always finds none, and a `ShipSyst`
//! -6 slot's ships are set up again in each system, at full strength. So
//! it is here: the traffic of the system left goes with it, and the new
//! system's setup brings the ships left, never a second set; a slot of
//! `ShipStart` 1 brings none at setup, and its ships come in on the
//! restarted timer.
//!
//! **Release.** A mission escort whose mark is cleared, its mission
//! aborted in flight ([`Session::release_mission_ships`], `_AbortMission`
//! @0x99ab5-0x99b24) or its slot freed (the next traffic tick,
//! `_AIDispatch` @0x8fc3a-0x8fc96), leaves the formation: its AI type
//! becomes its ship type's `InherentAI` (shïp +0x12), not its `düde`'s,
//! and it leaves the system (`_AIMakeShipLeave`), jumping out with a
//! jump's fuel. The rest re-form. A mission ship that never joined only
//! loses its mark.

use super::Session;
use crate::ai::Goal;
use crate::combat::hull::Condition;
use crate::escort::FormationSlot;
use crate::mission::ships::ShipBehav;
use crate::traffic::npc::{AiType, Npc, NpcId};

impl Session {
    /// Every mission's special ship that may join the player's formation
    /// joins it (see the module docs): of [`ShipBehav::EscortPlayer`],
    /// intact, not yet in it, its slot holding its mission. Each keeps
    /// formation with no target. Whether any joined; the formation
    /// re-forms when one did.
    pub(super) fn join_mission_escorts(&mut self) -> bool {
        let missions = &self.pilot.missions;
        let mut joined = false;
        for npc in self.traffic.npcs_mut() {
            let Some(mark) = &mut npc.mission else {
                continue;
            };
            let joins = mark.behav == ShipBehav::EscortPlayer
                && mark.formation.is_none()
                && npc.condition == Condition::Intact
                && missions.get(mark.slot).is_some();
            if joins {
                mark.formation = Some(FormationSlot::default());
                npc.goal = Goal::Formation { guard: None };
                npc.target = None;
                joined = true;
            }
        }
        if joined {
            self.reform();
        }
        joined
    }

    /// Puts every mission escort on its slot beside the player, with the
    /// player's heading and velocity.
    pub(super) fn snap_mission_escorts(&mut self) {
        let escorts: Vec<NpcId> = self
            .npcs()
            .iter()
            .filter(|npc| npc.mission_escort())
            .map(|npc| npc.id)
            .collect();
        for id in escorts {
            self.snap(id);
        }
    }

    /// NPC `id` loses its mission's mark; a mission escort also leaves
    /// the formation, takes its ship type's `InherentAI` and leaves the
    /// system (see the module docs). Whether it was a mission escort.
    pub(super) fn unmark_mission_ship(&mut self, id: NpcId) -> bool {
        let Some(npc) = self.npc(id) else {
            return false;
        };
        let escort = npc.mission_escort();
        let inherent = self
            .ship_record(npc.ship)
            .map(|record| AiType::from_raw(record.inherent_ai));
        let Some(npc) = self.npc_mut(id) else {
            return false;
        };
        npc.mission = None;
        if escort {
            leave_formation(npc, inherent);
        }
        escort
    }
}

/// Mission escort `npc` leaves the formation and the system: its AI type
/// its ship type's `inherent` one (kept as it was when its ship type has
/// no record), and it leaves ([`super::hail::make_leave`]).
fn leave_formation(npc: &mut Npc, inherent: Option<AiType>) {
    if let Some(ai_type) = inherent {
        npc.ai_type = ai_type;
    }
    super::hail::make_leave(npc);
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{MissionShip, ShipId, SystemId};
    use crate::chance::NeverFires;
    use crate::escort::{self, KEEP_FORMATION};
    use crate::flight::ShipState;
    use crate::hail::nova::{RELEASE, REQUEST_ASSISTANCE};
    use crate::hail::reply::{ESCORT_OPENING, IN_YOUR_DREAMS, JUST_AN_ESCORT};
    use crate::hail::{HailOptions, HailRefusal};
    use crate::pilot::Escort;
    use crate::reserves::Reserves;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::mission_ships::tests::{
        accepted, jump_to, shipping, ships_catalog, ships_of, take_off, tick, ticks,
    };
    use crate::targeting::TargetPick;
    use crate::testkit::FakePilotCatalog;
    use crate::testkit::land_now;

    /// Two special ships of düde 150 escorting the player (`ShipBehav`
    /// 1), of `ShipSyst` `syst` and `ShipStart` `start`.
    fn escorting(syst: i16, start: i16) -> MissionShip {
        MissionShip {
            behav: 1,
            ..shipping(syst, start)
        }
    }

    /// The mission escorts in the system, in order.
    fn mission_escorts(session: &Session) -> Vec<&Npc> {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.mission_escort())
            .collect()
    }

    /// The slot and formation size of each mission escort, in order.
    fn slots(session: &Session) -> Vec<(u8, u8)> {
        mission_escorts(session)
            .iter()
            .filter_map(|npc| npc.formation())
            .map(|place| (place.slot, place.ships))
            .collect()
    }

    /// A session of `catalog` with mission 140 accepted into slot 0, and
    /// the player turned to 90 degrees and moving, so a ship on its slot
    /// shows the player's heading and velocity.
    fn turned(catalog: &FakePilotCatalog) -> Session {
        let mut session = accepted(catalog);
        session.player.heading = 90.0;
        session
    }

    // Joining.

    #[test]
    fn ships_escorting_the_player_join_its_formation_as_they_are_set_up_at_take_off() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = turned(&catalog);
        take_off(&mut session, &catalog);
        let player = session.player;
        assert_eq!(slots(&session), [(2, 3), (3, 3)]);
        for npc in mission_escorts(&session) {
            let place = npc.formation().expect("in formation");
            assert_eq!(
                npc.state,
                ShipState {
                    position: place.position(&player),
                    ..player
                },
                "on its slot with the player's heading and velocity"
            );
            assert_eq!(npc.escort, None, "no hired escort");
            assert_eq!(npc.goal, Goal::Formation { guard: None });
        }
        assert_ne!(player.heading, 0.0, "{player:?}");
    }

    #[test]
    fn ships_of_another_behaviour_never_join() {
        for behav in [-1, 0, 2] {
            let catalog = ships_catalog(MissionShip {
                behav,
                ..shipping(-1, 0)
            });
            let mut session = accepted(&catalog);
            take_off(&mut session, &catalog);
            ticks(&mut session, &catalog, 3);
            assert_eq!(session.npcs().len(), 2, "{behav}");
            assert!(mission_escorts(&session).is_empty(), "{behav}");
        }
    }

    #[test]
    fn set_up_on_a_jumps_arrival_they_fly_to_their_slots() {
        let catalog = ships_catalog(escorting(-6, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        jump_to(&mut session, &catalog, 131);
        assert_eq!(slots(&session), [(2, 3), (3, 3)]);
        let off_slot = |session: &Session| {
            let player = session.player;
            mission_escorts(session)
                .iter()
                .map(|npc| {
                    let off = npc.formation().expect("in formation").position(&player)
                        - npc.state.position;
                    off.x.abs().max(off.y.abs())
                })
                .fold(0.0, f32::max)
        };
        assert!(
            off_slot(&session) > KEEP_FORMATION,
            "set up where they were"
        );
        ticks(&mut session, &catalog, 600);
        assert!(
            off_slot(&session) <= KEEP_FORMATION,
            "{}",
            off_slot(&session)
        );
    }

    #[test]
    fn ships_coming_in_on_their_timer_join_as_they_come() {
        let catalog = ships_catalog(escorting(-1, 1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 198);
        assert!(session.npcs().is_empty());
        tick(&mut session, &catalog);
        assert_eq!(slots(&session), [(2, 3), (3, 3)]);
        assert_eq!(ships_of(&session).arrived, 2);
    }

    #[test]
    fn a_mission_escort_stays_joined_while_marked_and_while_disabled() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 5);
        assert_eq!(slots(&session), [(2, 3), (3, 3)]);
        session.traffic.npcs_mut()[0].condition = Condition::Disabled;
        ticks(&mut session, &catalog, 5);
        assert_eq!(slots(&session), [(2, 3), (3, 3)], "keeps its place");
    }

    #[test]
    fn a_ship_disabled_before_it_joins_joins_once_intact() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let npc = &mut session.traffic.npcs_mut()[0];
        if let Some(mark) = &mut npc.mission {
            mark.formation = None;
        }
        npc.condition = Condition::Disabled;
        session.reform();
        tick(&mut session, &catalog);
        assert_eq!(slots(&session), [(2, 2)], "only the other");
        session.traffic.npcs_mut()[0].condition = Condition::Intact;
        tick(&mut session, &catalog);
        assert_eq!(slots(&session), [(2, 3), (3, 3)]);
    }

    // Following.

    /// The marked ships of slot 0 in the system.
    fn of_slot_0(session: &Session) -> usize {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.mission.is_some_and(|mark| mark.slot == 0))
            .count()
    }

    #[test]
    fn ships_following_the_player_are_set_up_again_after_a_jump_never_doubled() {
        let catalog = ships_catalog(escorting(-6, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(slots(&session), [(2, 3), (3, 3)]);
        let before: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        jump_to(&mut session, &catalog, 131);
        assert_eq!(of_slot_0(&session), 2, "exactly the two left");
        assert!(
            session.npcs().iter().all(|npc| !before.contains(&npc.id)),
            "the old ones left behind"
        );
        assert_eq!(slots(&session), [(2, 3), (3, 3)], "both join");
        ticks(&mut session, &catalog, 5);
        assert_eq!(of_slot_0(&session), 2);
    }

    #[test]
    fn ships_coming_later_come_in_on_the_restarted_timer_after_a_jump_and_join() {
        // ShipGoal 3 restarts the timer at 30 on a jump's arrival.
        let catalog = ships_catalog(MissionShip {
            goal: 3,
            ..escorting(-6, 1)
        });
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 300);
        assert_eq!(slots(&session), [(2, 3), (3, 3)]);
        jump_to(&mut session, &catalog, 131);
        assert_eq!(of_slot_0(&session), 0, "setup brings none");
        assert_eq!(ships_of(&session).appear, 30);
        for _ in 0..29 {
            tick(&mut session, &catalog);
            assert_eq!(of_slot_0(&session), 0);
        }
        tick(&mut session, &catalog);
        assert_eq!(slots(&session), [(2, 3), (3, 3)], "come in and join");
        for _ in 0..100 {
            tick(&mut session, &catalog);
            assert!(of_slot_0(&session) <= 2);
        }
    }

    #[test]
    fn ships_of_a_fixed_system_stay_behind_and_are_set_up_again_on_return() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(slots(&session), [(2, 3), (3, 3)]);
        jump_to(&mut session, &catalog, 131);
        assert_eq!(of_slot_0(&session), 0, "none with the player");
        jump_to(&mut session, &catalog, 130);
        assert_eq!(slots(&session), [(2, 3), (3, 3)]);
    }

    // One formation.

    /// A hired escort of ship 200.
    fn hired() -> Escort {
        Escort {
            ship: ShipId(200),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }
    }

    #[test]
    fn hired_and_mission_escorts_hold_distinct_slots_of_one_formation_and_re_form() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = accepted(&catalog);
        session.pilot.escorts.push(hired());
        take_off(&mut session, &catalog);
        let hired_id = session.fleet[0].expect("placed");
        let duty = session
            .npc(hired_id)
            .and_then(|npc| npc.escort)
            .expect("hired");
        assert_eq!((duty.formation.slot, duty.formation.ships), (2, 4));
        assert_eq!(slots(&session), [(3, 4), (4, 4)]);
        let widths: Vec<Option<f32>> = session
            .npcs()
            .iter()
            .map(|npc| npc.hull.sprite_size)
            .collect();
        let spacing = escort::spacing(session.hull.sprite_size, &widths);
        assert_eq!(duty.formation.spacing, spacing);
        for npc in mission_escorts(&session) {
            assert_eq!(npc.formation().map(|place| place.spacing), Some(spacing));
        }
        session.dismiss(hired_id);
        assert_eq!(slots(&session), [(2, 3), (3, 3)], "the rest re-form");
    }

    #[test]
    fn a_destroyed_mission_escort_leaves_the_rest_to_re_form() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        session.traffic.npcs_mut()[0].condition = Condition::Destroyed;
        session.tick_combat(crate::combat::Rules::default(), &mut NeverFires);
        assert_eq!(session.npcs().len(), 1);
        assert_eq!(slots(&session), [(2, 2)]);
        assert_eq!(ships_of(&session).destroyed, 1, "a lost mission ship");
    }

    // Release.

    /// [`ships_catalog`] of `ships`, its ship 200 of `InherentAI` 1, a
    /// wimpy trader, where the düde's `AIType` is 3, a warship.
    fn inherent_catalog(ships: MissionShip) -> FakePilotCatalog {
        let mut catalog = ships_catalog(ships);
        catalog.ship_records[1].inherent_ai = 1;
        catalog
    }

    /// The NPCs of `ids` still in the system.
    fn of<'a>(session: &'a Session, ids: &[NpcId]) -> Vec<&'a Npc> {
        session
            .npcs()
            .iter()
            .filter(|npc| ids.contains(&npc.id))
            .collect()
    }

    /// Each released mission escort among `ids`: unmarked, its ship type's
    /// AI, leaving by a jump.
    fn assert_released(session: &Session, ids: &[NpcId]) {
        let released = of(session, ids);
        assert_eq!(released.len(), ids.len(), "still here as they leave");
        for npc in released {
            assert_eq!(npc.mission, None);
            assert!(!npc.escorts_player());
            assert_eq!(
                npc.ai_type,
                AiType::WimpyTrader,
                "the ship's, not the düde's"
            );
            assert_eq!(npc.goal, Goal::JumpOut);
            assert_eq!(npc.target, None);
        }
    }

    /// A session of `catalog` in flight with mission 140's two escorts
    /// and a hired escort in formation: the mission escorts' NPCs.
    fn flying_with_escorts(catalog: &FakePilotCatalog) -> (Session, Vec<NpcId>) {
        let mut session = accepted(catalog);
        session.pilot.escorts.push(hired());
        take_off(&mut session, catalog);
        let ids: Vec<NpcId> = mission_escorts(&session).iter().map(|npc| npc.id).collect();
        assert_eq!(ids.len(), 2);
        for npc in of(&session, &ids) {
            assert_eq!(npc.ai_type, AiType::Warship, "the düde's");
        }
        (session, ids)
    }

    /// The hired escort's slot and formation size.
    fn hired_slot(session: &Session) -> (u8, u8) {
        let id = session.fleet[0].expect("placed");
        let duty = session.npc(id).and_then(|npc| npc.escort).expect("hired");
        (duty.formation.slot, duty.formation.ships)
    }

    #[test]
    fn aborted_in_flight_a_mission_escort_takes_its_ships_ai_and_leaves() {
        let catalog = inherent_catalog(escorting(-1, 0));
        let (mut session, ids) = flying_with_escorts(&catalog);
        assert_eq!(hired_slot(&session), (2, 4));
        assert_eq!(session.abort_mission(0, &mut NeverFires), Ok(()));
        assert_released(&session, &ids);
        assert_eq!(hired_slot(&session), (2, 2), "the hired escort re-forms");
        ticks(&mut session, &catalog, 600);
        assert!(of(&session, &ids).is_empty(), "they jumped out");
    }

    #[test]
    fn a_freed_slot_releases_its_mission_escorts_on_the_next_tick() {
        let catalog = inherent_catalog(escorting(-1, 0));
        let (mut session, ids) = flying_with_escorts(&catalog);
        assert!(session.pilot.missions.free(0).is_some(), "finished");
        assert_eq!(slots(&session), [(3, 4), (4, 4)], "until the next tick");
        tick(&mut session, &catalog);
        assert_released(&session, &ids);
        assert_eq!(hired_slot(&session), (2, 2));
    }

    #[test]
    fn an_auto_abort_releases_its_mission_escorts_on_the_next_tick() {
        let catalog = inherent_catalog(escorting(-1, 0));
        let (mut session, ids) = flying_with_escorts(&catalog);
        let record = catalog.missions[0].clone();
        session.auto_abort(0, &record, &mut NeverFires);
        tick(&mut session, &catalog);
        assert_released(&session, &ids);
    }

    #[test]
    fn a_mission_ship_yet_to_join_only_loses_its_mark_as_its_mission_is_aborted() {
        let catalog = inherent_catalog(escorting(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        for npc in session.traffic.npcs_mut() {
            if let Some(mark) = &mut npc.mission {
                mark.formation = None;
            }
            npc.condition = Condition::Disabled;
        }
        assert_eq!(session.abort_mission(0, &mut NeverFires), Ok(()));
        for npc in session.npcs() {
            assert_eq!(npc.mission, None);
            assert_eq!(npc.ai_type, AiType::Warship, "its düde's still");
            assert_ne!(npc.goal, Goal::JumpOut);
        }
    }

    // Hails.

    /// [`ships_catalog`] of `ships`, the ship comm strings "c<n>" in
    /// `STR#` 3000, and government 131, the düde's, of `flags`.
    fn hailing(ships: MissionShip, flags: u16) -> FakePilotCatalog {
        let mut catalog = ships_catalog(ships);
        catalog.govts.push(crate::catalog::GovtRecord {
            flags,
            ..crate::testkit::govt(131)
        });
        catalog
            .strings
            .push((3000, (1..=200).map(|n| format!("c{n}")).collect()));
        catalog
    }

    /// Group `group`'s line in variant 4, the one [`NeverFires`] draws.
    fn line(group: u8) -> String {
        format!("c{}", crate::hail::Reply::locate(group, 4).1)
    }

    /// The comm dialog of `catalog`'s session in flight, NPC 0 targeted
    /// and hailed.
    fn hailed(catalog: &FakePilotCatalog) -> (Session, crate::hail::HailView) {
        let mut session = accepted(catalog);
        take_off(&mut session, catalog);
        session.target = Some(session.npcs()[0].id);
        let view = session
            .hail(catalog, &HailOptions::default(), &mut NeverFires)
            .expect("answered");
        (session, view)
    }

    /// The labels of `view`'s options.
    fn labels(view: &crate::hail::HailView) -> Vec<&str> {
        view.options
            .iter()
            .map(|button| button.label.as_str())
            .collect()
    }

    /// What the hailed ship says to Request Assistance.
    fn request_assistance(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        view: &crate::hail::HailView,
    ) -> String {
        let pick = labels(view)
            .iter()
            .position(|&label| label == REQUEST_ASSISTANCE)
            .expect("listed");
        session
            .answer(pick, catalog, &HailOptions::default(), &mut NeverFires)
            .expect("answered")
            .reply
    }

    #[test]
    fn a_mission_escort_opens_with_what_can_i_do_for_you_whatever_its_goal() {
        for goal in [-1, 3, 0] {
            let catalog = hailing(
                MissionShip {
                    goal,
                    ..escorting(-1, 0)
                },
                0,
            );
            let (session, view) = hailed(&catalog);
            assert!(session.npcs()[0].mission_escort());
            assert_eq!(view.reply, line(ESCORT_OPENING), "{goal}");
        }
    }

    #[test]
    fn a_mission_escort_is_just_an_escort_to_request_assistance_with_no_escort_dialog() {
        let catalog = hailing(escorting(-1, 0), 0);
        let (mut session, view) = hailed(&catalog);
        assert!(labels(&view).contains(&REQUEST_ASSISTANCE));
        assert!(!labels(&view).contains(&RELEASE), "{:?}", labels(&view));
        assert_eq!(view.escort, None, "not the hired escort's dialog");
        assert!(!view.hostile);
        assert_eq!(
            request_assistance(&mut session, &catalog, &view),
            line(JUST_AN_ESCORT)
        );
    }

    #[test]
    fn a_marked_ship_not_escorting_the_player_is_no_friend_whatever_its_government() {
        // A ShipGoal 3 ship of ShipBehav -1, as a swapped person's: it
        // never forms on the player, and its mark keeps it from liking it.
        let ships = MissionShip {
            goal: 3,
            ..shipping(-1, 0)
        };
        let catalog = hailing(ships, 0);
        let (mut session, view) = hailed(&catalog);
        assert!(!session.npcs()[0].escorts_player());
        assert_eq!(
            request_assistance(&mut session, &catalog, &view),
            line(IN_YOUR_DREAMS)
        );
        session.traffic.npcs_mut()[0].mission = None;
        let view = session
            .hail(&catalog, &HailOptions::default(), &mut NeverFires)
            .expect("answered");
        assert_ne!(
            request_assistance(&mut session, &catalog, &view),
            line(IN_YOUR_DREAMS),
            "unmarked, its government likes the player"
        );
    }

    #[test]
    fn a_mission_escort_of_a_mute_government_answers_no_hail_while_a_hired_escort_does() {
        let mut catalog = hailing(escorting(-1, 0), crate::hail::MUTE);
        catalog.ship_records[1].inherent_govt = Some(crate::catalog::GovtId(131));
        let mut session = accepted(&catalog);
        session.pilot.escorts.push(hired());
        take_off(&mut session, &catalog);
        let mission_escort = mission_escorts(&session)[0].id;
        session.target = Some(mission_escort);
        assert_eq!(
            session
                .hail(&catalog, &HailOptions::default(), &mut NeverFires)
                .map(|view| view.reply),
            Err(HailRefusal::NoResponse)
        );
        session.target = session.fleet[0];
        let view = session
            .hail(&catalog, &HailOptions::default(), &mut NeverFires)
            .expect("the hired escort answers");
        assert!(labels(&view).contains(&RELEASE));
    }

    // Left out of the hired escorts' paths.

    #[test]
    fn a_save_holds_no_mission_escort() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(mission_escorts(&session).len(), 2);
        let saved = crate::save::encode(session.pilot());
        let loaded = crate::save::decode(&saved).expect("loads");
        assert!(loaded.escorts().is_empty());
        assert!(
            session.pilot().escorts().is_empty(),
            "joining makes no record"
        );
    }

    #[test]
    fn a_jump_and_a_take_off_pay_mission_escorts_nothing_and_none_defects() {
        let catalog = ships_catalog(escorting(-6, 0));
        let mut session = accepted(&catalog);
        session.pilot.cash = 0;
        take_off(&mut session, &catalog);
        jump_to(&mut session, &catalog, 131);
        assert!(session.take_pay_notes().is_empty());
        assert_eq!(slots(&session), [(2, 3), (3, 3)], "none defected");
        session.player.position = session.sites[0].position;
        land_now(&mut session).expect("lands");
        take_off(&mut session, &catalog);
        assert!(session.take_pay_notes().is_empty());
        assert_eq!(session.pilot().cash, 0, "nothing paid");
        assert_eq!(slots(&session), [(2, 3), (3, 3)]);
    }

    #[test]
    fn landing_sells_upgrades_and_keeps_no_mission_escort() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        land_now(&mut session).expect("lands");
        assert!(session.pilot().escorts().is_empty());
        assert_eq!(session.stay.escorts_changed, 0, "none sold or upgraded");
        session.take_off(&mut NeverFires).expect("takes off");
        assert!(session.fleet.is_empty(), "none waits in the record");
    }

    #[test]
    fn a_trader_mission_escort_adds_no_holds_and_a_jettison_spreads_over_none() {
        let mut catalog = ships_catalog(escorting(-1, 0));
        catalog.ship_records[1].inherent_ai = 1;
        catalog.ship_records[1].escort_type = 3;
        catalog.ship_records[1].fields.holds = 50;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert!(
            mission_escorts(&session)
                .iter()
                .all(|npc| npc.class == crate::escort::EscortClass::Freighter)
        );
        assert_eq!(session.capacity(), session.stats().capacity);
        session.jettison_cargo(true, &mut NeverFires);
        assert!(session.pilot().escorts().is_empty());
        assert_eq!(session.capacity(), session.stats().capacity);
    }

    #[test]
    fn a_fighter_mission_escort_is_never_recalled_into_a_bay() {
        for source in RuleSource::ALL {
            let mut catalog = ships_catalog(escorting(-6, 0));
            catalog.ship_records[1].escort_type = 0;
            let mut session = accepted(&catalog);
            session.rules = Rulebook::default().with_override(RuleKey::FighterRecall, source);
            take_off(&mut session, &catalog);
            assert!(
                mission_escorts(&session)
                    .iter()
                    .all(|npc| npc.class == crate::escort::EscortClass::Fighter)
            );
            let outfits = session.pilot().outfits.clone();
            jump_to(&mut session, &catalog, 131);
            assert_eq!(session.pilot().outfits, outfits, "{source:?}");
            assert!(session.take_fighter_notes().is_empty(), "{source:?}");
            assert_eq!(slots(&session), [(2, 3), (3, 3)], "{source:?}");
        }
    }

    #[test]
    fn a_mission_escort_destroyed_or_disabled_is_a_lost_mission_ship_not_a_lost_escort() {
        // Its mission cannot be aborted, so it fails in its slot.
        let mut catalog = ships_catalog(MissionShip {
            goal: 3,
            ..escorting(-1, 0)
        });
        catalog.missions[0].can_abort = 0;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        session.traffic.npcs_mut()[1].condition = Condition::Disabled;
        session.traffic.npcs_mut()[0].condition = Condition::Destroyed;
        session.tick_combat(crate::combat::Rules::default(), &mut NeverFires);
        assert!(session.pilot().escorts().is_empty());
        assert!(session.fleet.is_empty());
        assert_eq!(ships_of(&session).destroyed, 1);
        let failed = session
            .pilot()
            .missions()
            .get(0)
            .and_then(|active| active.failed);
        assert!(failed.is_some(), "an escort goal fails");
        assert!(session.npcs()[0].mission_escort(), "the disabled one stays");
    }

    // Not a hired escort.

    #[test]
    fn a_mission_escort_is_no_hired_escort_and_the_nearest_target_can_pick_it() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(mission_escorts(&session).len(), 2);
        for npc in session.npcs() {
            assert!(!session.is_escort(npc.id));
        }
        assert!(session.pilot().escorts().is_empty());
        assert_eq!(session.pilot.escort_count(), 0, "the hiring room's count");
        assert_eq!(session.select_target(TargetPick::NextEscort), None);
        // The original's Closest Target skips every ship led by the player
        // (`_FindNearestShipToPlayer` @0x92a6-0x92ae), mission escorts
        // too; here only the hired escorts are skipped.
        assert!(session.select_target(TargetPick::Nearest).is_some());
        assert_eq!(session.pilot.system, SystemId(130));
    }
}
