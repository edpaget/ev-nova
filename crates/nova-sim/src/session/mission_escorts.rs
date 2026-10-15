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
//! ([`Npc::fleet`]), flown by the escorts' AI by the standing order the
//! player's escort commands give it, its class's order as it joins (see
//! [`ai::escort`](crate::ai::escort) and [`Session::command_escorts`]),
//! and never one of the hired escorts:
//! it carries no [`EscortDuty`](crate::escort::EscortDuty), so saving,
//! pay, the escort landing screen, the escort comm dialog, jettison,
//! fighter recall and the hiring room, which read the pilot's escorts or
//! the session's fleet, all leave it out, as the original leaves out
//! every marked ship there. A ship disabled before it joins does not join
//! until it is intact again; one disabled once joined keeps its place, as
//! the original leaves a disabled ship's leader as it was (@0x8fc0c), and
//! its `_HandleShip` releases only an unmarked disabled escort
//! (@0x339a6). One that repairs itself (see
//! [`self_repair`](super::self_repair)) resumes as a mission escort: it
//! never left, so it never rejoins as a hired one. A person swapped for a ship of a `ShipGoal` 3 mission of
//! another `ShipBehav` never joins: the swap's `_AIMakeEscortFlyInForm`
//! (@0x62509) follows its leader, which `_GenericRandomShipSpawn` left at
//! -1 (@0x3c9cd), so it never forms on the player.
//! Because the join comes before its type's AI decides, a `ShipBehav` 1
//! ship never thinks as a warship or interceptor while its mission is
//! active, so `_SelectWarshipTarget`'s `ShipBehav` 1 branch
//! (@0x89f86-0x89fef: clear a player target, else fight a threat to the
//! player, else fly with it) never runs and is not modelled.
//!
//! **One formation.** The hired escorts and the mission escorts share one
//! formation with the player, as the original numbers every ship whose
//! formation parent is the player in one count and spacing
//! (`_AICalcFormationPositions` @0x850bc-0x85192). Both take slots 2, 3,
//! … together in ship-table order, by their berths
//! ([`Npc::berth`](crate::traffic::npc::Npc::berth)): the hired escorts
//! keep theirs across each setup, and the mission escorts, set up again,
//! fill the first free ones. The formation re-forms whenever one joins or
//! leaves. Set up at take-off or as a pilot is flown, a mission escort
//! sits on its slot with the player's heading and velocity
//! (`_AICalcFormationPositions(player, 1)` @0x4319a-0x431b2); set up on a
//! jump's arrival it stays where it was set up and flies to its slot; at
//! a gate's exit it comes out of the exit gate (see
//! [`gate_escorts`](super::gate_escorts)).
//!
//! **Following.** The original's setup first deletes every mission escort
//! (`_DeleteAllComputerShips` @0x3bf7e-0x3bfb6), so its "already with the
//! player" count (@0x42e3f-0x42e83) always finds none, and a `ShipSyst`
//! -6 slot's ships are set up again in each system, at full strength. So
//! it is here: the traffic of the system left goes with it, and the new
//! system's setup brings the ships left, never a second set; a slot of
//! `ShipStart` 1 brings none at setup, and its ships come in on the
//! restarted timer. A hypergate's or wormhole's exit is the exception:
//! there the original's loop A stops a -6 escort slot's timer before the
//! setup (@0x671a1-0x671b0), so the exit's setup brings its ships at
//! once, `ShipStart` 1 or not, as the engine reads
//! [`RuleKey::GateFollow`](crate::RuleKey::GateFollow) (see the
//! [`gate_follow`](super::gate_follow) module).
//! An in-flight `M`/`N` carries them along as they are (`_EvalSetExp`
//! @0x15807-0x1583e), and the next setup deletes them and sets them up
//! again as usual.
//!
//! **Release.** A mission escort whose mark is cleared, its mission
//! aborted in flight ([`Session::release_mission_ships`], `_AbortMission`
//! @0x99ab5-0x99b24) or its slot freed (the next traffic tick,
//! `_AIDispatch` @0x8fc3a-0x8fc96), leaves the formation: its AI type
//! becomes its ship type's `InherentAI` (shïp +0x12), not its `düde`'s,
//! and it leaves the system (`_AIMakeShipLeave`), jumping out with a
//! jump's fuel. The rest re-form. A mission ship that never joined only
//! loses its mark. A mission escort breaking up leaves the formation
//! without losing its mark (see Death).
//!
//! **Death.** A mission escort that starts to break up (or is destroyed
//! outright) leaves the formation on that tick, before its wreck is
//! taken out ([`Session::lose_dying_mission_escorts`]), as
//! `_HandleShip`'s dying branch (@0x350ab-0x3512b) clears its leader and
//! sets its AI type to its ship type's `InherentAI`. Its mark stays, so
//! it is still its mission's ship lost when it goes, and the rest
//! re-form at once. A trader (`InherentAI` 2 or less) first takes its
//! share of the fleet's cargo (`_DestroyPartialFleetCargo`,
//! [`Session::lose_cargo_share`]): as its `Holds` are never in the
//! fleet's, it takes its `Holds` over the player's ship's and the hired
//! traders', at most all of it, by the engine, and over those with its
//! own by the other reading of
//! [`RuleKey::LostCargoShare`](crate::RuleKey::LostCargoShare). A
//! disabled mission escort keeps its place and takes nothing.

use super::Session;
use crate::ai::Goal;
use crate::catalog::ShipId;
use crate::combat::hull::Condition;
use crate::escort::{EscortClass, FormationSlot};
use crate::mission::ships::ShipBehav;
use crate::traffic::npc::{AiType, Npc, NpcId};

impl Session {
    /// Every mission's special ship that may join the player's formation
    /// joins it (see the module docs): of [`ShipBehav::EscortPlayer`],
    /// intact, not yet in it, its slot holding its mission. Each keeps
    /// formation with no target, taking its class's standing order
    /// ([`Session::class_order`], `_EscortAI` @0x83ad8-0x83b2b). Whether any joined; the formation
    /// re-forms when one did.
    pub(super) fn join_mission_escorts(&mut self) -> bool {
        let orders = EscortClass::ALL.map(|class| (class, self.class_order(class)));
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
                mark.order = orders
                    .iter()
                    .find(|(class, _)| *class == npc.class)
                    .and_then(|(_, order)| *order);
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
    /// player's heading and velocity: the ships the player leads
    /// ([`Session::player_led`]) narrowed to the mission escorts.
    pub(super) fn snap_mission_escorts(&mut self) {
        let escorts: Vec<NpcId> = self
            .player_led()
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

    /// Every mission escort breaking up or destroyed leaves the formation
    /// at once, as `_HandleShip`'s dying branch (@0x350ab-0x3512b) clears
    /// its leader and sets its AI type to its ship type's `InherentAI`
    /// (@0x35118-0x3512b) on the first tick it is dying. Its mark stays,
    /// so it is still counted as its mission's ship lost when it goes, and
    /// it does not leave the system. A trader (`InherentAI` 2 or less)
    /// first takes its share of the fleet's cargo
    /// ([`Session::lose_cargo_share`], @0x350d9-0x350f4): the original
    /// tests its AI type 6 and no mark, and its `Holds` are never in the
    /// fleet's, a marked ship's never being (@0xced9). The rest re-form.
    pub(super) fn lose_dying_mission_escorts(&mut self) {
        let dying: Vec<(NpcId, ShipId)> = self
            .traffic
            .npcs()
            .iter()
            .filter(|npc| {
                npc.mission_escort()
                    && matches!(
                        npc.condition,
                        Condition::Dying { .. } | Condition::Destroyed
                    )
            })
            .map(|npc| (npc.id, npc.ship))
            .collect();
        for &(id, ship) in &dying {
            let inherent = self
                .ship_record(ship)
                .map(|record| AiType::from_raw(record.inherent_ai));
            if let Some(npc) = self.npc_mut(id) {
                if let Some(mark) = &mut npc.mission {
                    mark.formation = None;
                }
                if let Some(ai_type) = inherent {
                    npc.ai_type = ai_type;
                }
            }
            self.lose_cargo_share(ship);
        }
        if !dying.is_empty() {
            self.reform();
        }
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
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::{JunkId, MissionShip, ShipId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::escort::{
        self, Commanded, EscortClass, EscortCommand, EscortGroup, EscortOrder, KEEP_FORMATION,
    };
    use crate::flight::ShipState;
    use crate::hail::nova::{RELEASE, REQUEST_ASSISTANCE};
    use crate::hail::reply::{ESCORT_OPENING, IN_YOUR_DREAMS, JUST_AN_ESCORT};
    use crate::hail::{HailOptions, HailRefusal};
    use crate::market::Good;
    use crate::pilot::Escort;
    use crate::reserves::Reserves;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::mission_ships::tests::{
        accepted, jump_to, settle, shipping, ships_catalog, ships_of, take_off, tick, ticks,
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

    /// Records the NPCs a type's behaviour is asked to decide about.
    #[derive(Debug, Default)]
    struct Asked(std::cell::RefCell<Vec<NpcId>>);

    impl crate::ai::Behaviour for Asked {
        fn decide(
            &self,
            npc: &Npc,
            _around: &crate::ai::Surroundings,
            _chance: &mut dyn crate::chance::Chance,
        ) -> Goal {
            self.0.borrow_mut().push(npc.id);
            Goal::default()
        }

        fn trigger(
            &self,
            _npc: &Npc,
            _around: &crate::ai::Surroundings,
        ) -> crate::combat::armament::Trigger {
            crate::combat::armament::Trigger::default()
        }

        fn target(
            &self,
            _npc: &Npc,
            _around: &crate::ai::Surroundings,
        ) -> Option<crate::combat::ShipRef> {
            None
        }

        fn react(
            &self,
            _npc: &Npc,
            _strike: &crate::combat::Strike,
            _around: &crate::ai::Surroundings,
        ) -> crate::ai::Reaction {
            crate::ai::Reaction::default()
        }
    }

    /// `_SelectWarshipTarget`'s `ShipBehav` 1 branch (@0x89f86-0x89fef)
    /// never runs: a ship out of formation joins before its type's AI
    /// decides, so the warship's or interceptor's behaviour is never
    /// asked about it.
    #[test]
    fn a_ship_out_of_formation_joins_before_its_types_ai_decides() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let npc = &mut session.traffic.npcs_mut()[0];
        let id = npc.id;
        if let Some(mark) = &mut npc.mission {
            mark.formation = None;
        }
        npc.condition = Condition::Disabled;
        session.reform();
        tick(&mut session, &catalog);
        session.traffic.npcs_mut()[0].condition = Condition::Intact;
        assert!(!session.npcs()[0].mission_escort(), "out of formation");
        let asked = std::rc::Rc::new(Asked::default());
        let ai = crate::ai::NovaAi::default()
            .with(AiType::Warship, asked.clone())
            .with(AiType::Interceptor, asked.clone())
            .with(AiType::WimpyTrader, asked.clone())
            .with(AiType::BraveTrader, asked.clone());
        for _ in 0..16 {
            session.tick_traffic(&catalog, &ai, &mut NeverFires);
            let npc = &session.npcs()[0];
            assert!(npc.mission_escort());
            assert!(matches!(npc.goal, Goal::Formation { .. }), "{:?}", npc.goal);
        }
        assert!(!asked.0.borrow().contains(&id), "{:?}", asked.0.borrow());
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

    /// Each ship the player leads, by its id, and where it is from the
    /// player.
    fn led_offsets(session: &Session) -> Vec<(NpcId, crate::geometry::Vec2)> {
        let player = session.player().position;
        session
            .player_led()
            .map(|npc| (npc.id, npc.state.position - player))
            .collect()
    }

    #[test]
    fn every_ship_the_player_leads_leaves_on_the_players_clock_and_arrives_with_it() {
        let catalog = ships_catalog(escorting(-6, 0));
        let mut session = accepted(&catalog);
        session.pilot.escorts.push(hired());
        take_off(&mut session, &catalog);
        session.plot_course(SystemId(131)).expect("a route");
        session.player.position = crate::geometry::Vec2::new(5000.0, 0.0);
        let led: Vec<NpcId> = session.player_led().map(|npc| npc.id).collect();
        assert_eq!(led.len(), 3, "the hired escort and both mission escorts");
        for &id in &led {
            session.snap(id);
        }
        session.begin_jump().expect("jumps");
        let ai = crate::ai::NovaAi::default();
        let mut braking = 0;
        while session.jumping().is_none() {
            session.tick(crate::flight::Controls::default());
            session.tick_traffic(&catalog, &ai, &mut NeverFires);
            session.tick_combat(crate::combat::Rules::default(), &mut NeverFires);
            assert!(session.traffic.departed().is_empty(), "none leaves alone");
            braking += 1;
            assert!(braking < 1000, "the jump never began");
        }
        let frozen = led_offsets(&session);
        assert_eq!(frozen.len(), 3, "all still led");
        for _ in 0..30 {
            session.tick(crate::flight::Controls::default());
            session.tick_traffic(&catalog, &ai, &mut NeverFires);
            session.tick_combat(crate::combat::Rules::default(), &mut NeverFires);
            assert_eq!(led_offsets(&session), frozen, "held beside the player");
            assert!(session.traffic.departed().is_empty());
        }
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        settle(&mut session, &catalog);
        assert_eq!(session.pilot().escorts().len(), 1);
        assert!(session.fleet[0].is_some_and(|id| session.npc(id).is_some()));
        assert_eq!(slots(&session).len(), 2, "both mission escorts came");
        assert_eq!(session.player_led().count(), 3);
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

    // Death.

    /// Food, a commodity.
    const FOOD: Good = Good::Commodity(0);
    /// Opals, a `jünk`.
    const OPALS: Good = Good::Junk(JunkId(146));

    /// A session in flight with mission 140's two escorts of ship 200, of
    /// `InherentAI` `inherent` and `Holds` 10 (the player's ship's are
    /// 20), by `source` of `LostCargoShare`, holding 9 food and 3 opals.
    fn laden(inherent: i16, source: RuleSource) -> Session {
        let mut catalog = ships_catalog(escorting(-1, 0));
        catalog.ship_records[1].inherent_ai = inherent;
        catalog.ship_records[1].fields.holds = 10;
        let mut session = accepted(&catalog);
        session.rules = Rulebook::default().with_override(RuleKey::LostCargoShare, source);
        take_off(&mut session, &catalog);
        assert_eq!(mission_escorts(&session).len(), 2);
        session.pilot.cargo = BTreeMap::from([(FOOD, 9), (OPALS, 3)]);
        session
    }

    /// The first mission escort's armour goes to 0, and the fight ticks:
    /// it starts to break up, for 5 ticks. Its NPC.
    fn break_up(session: &mut Session) -> NpcId {
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.reserves.armor.now = 0.0;
        npc.hull.death_delay = 5;
        let id = npc.id;
        session.tick_combat(crate::combat::Rules::default(), &mut NeverFires);
        let npc = session.npc(id).expect("breaking up");
        assert!(matches!(npc.condition, Condition::Dying { .. }));
        id
    }

    #[test]
    fn a_trader_mission_escorts_death_destroys_its_share_of_the_cargo() {
        let mut session = laden(1, RuleSource::Engine);
        break_up(&mut session);
        assert_eq!(
            session.pilot.cargo,
            BTreeMap::from([(FOOD, 5), (OPALS, 2)]),
            "its 10 of the player's 20"
        );
        let mut session = laden(1, RuleSource::Bible);
        break_up(&mut session);
        assert_eq!(
            session.pilot.cargo,
            BTreeMap::from([(FOOD, 6), (OPALS, 2)]),
            "its 10 of the player's 20 and its own 10"
        );
    }

    #[test]
    fn a_dying_trader_mission_escort_resets_as_it_takes_its_share() {
        use crate::scan::Latches;
        use crate::session::precalced::tests::convicted;
        let mut session = laden(1, RuleSource::Engine);
        convicted(&mut session);
        break_up(&mut session);
        assert_eq!(session.scan_latches, Latches::ARMED, "@0x350e6, @0xcff6");
    }

    #[test]
    fn a_dying_trader_mission_escort_trims_the_cargo_past_the_holds_after_its_share() {
        let mut session = laden(1, RuleSource::Engine);
        assert_eq!(session.capacity(), 20);
        session.pilot.cargo = BTreeMap::from([(FOOD, 40), (OPALS, 6)]);
        break_up(&mut session);
        assert_eq!(
            session.pilot.cargo,
            BTreeMap::from([(FOOD, 17), (OPALS, 3)]),
            "half of each taken, then 20 x 20/23 of the food (@0x350e6, @0xcff6)"
        );
    }

    #[test]
    fn a_mission_escort_of_inherent_ai_2_dying_destroys_its_share_of_the_cargo() {
        let mut session = laden(2, RuleSource::Engine);
        break_up(&mut session);
        assert_eq!(
            session.pilot.cargo,
            BTreeMap::from([(FOOD, 5), (OPALS, 2)]),
            "InherentAI 2 is a trader's"
        );
    }

    #[test]
    fn a_dying_mission_escort_leaves_the_formation_at_once() {
        let mut session = laden(1, RuleSource::Engine);
        let id = break_up(&mut session);
        let npc = session.npc(id).expect("breaking up");
        assert!(!npc.mission_escort());
        assert!(npc.mission.is_some(), "still its mission's ship");
        assert_eq!(npc.ai_type, AiType::WimpyTrader, "its ship's InherentAI");
        assert_eq!(slots(&session), [(2, 2)], "the other re-forms");
        for _ in 0..100 {
            if session.npc(id).is_none() {
                break;
            }
            session.tick_combat(crate::combat::Rules::default(), &mut NeverFires);
        }
        assert_eq!(session.npc(id), None, "destroyed and gone");
        assert_eq!(ships_of(&session).destroyed, 1, "a lost mission ship");
        assert_eq!(
            session.pilot.cargo,
            BTreeMap::from([(FOOD, 5), (OPALS, 2)]),
            "only once"
        );
    }

    #[test]
    fn a_warship_mission_escorts_death_destroys_no_cargo() {
        let mut session = laden(3, RuleSource::Engine);
        let id = break_up(&mut session);
        assert_eq!(session.pilot.cargo, BTreeMap::from([(FOOD, 9), (OPALS, 3)]));
        let npc = session.npc(id).expect("breaking up");
        assert!(!npc.mission_escort(), "it still leaves the formation");
        assert_eq!(npc.ai_type, AiType::Warship);
    }

    #[test]
    fn a_disabled_mission_escort_destroys_no_cargo() {
        let mut session = laden(1, RuleSource::Engine);
        session.traffic.npcs_mut()[0].condition = Condition::Disabled;
        session.tick_combat(crate::combat::Rules::default(), &mut NeverFires);
        assert_eq!(session.pilot.cargo, BTreeMap::from([(FOOD, 9), (OPALS, 3)]));
        assert!(session.npcs()[0].mission_escort());
    }

    #[test]
    fn a_mission_escort_disabled_or_breaking_up_remembers_no_former_escort() {
        let mut session = laden(1, RuleSource::Engine);
        session.traffic.npcs_mut()[1].reserves.armor.now = 5.0;
        let id = break_up(&mut session);
        let disabled = &session.npcs()[1];
        assert_eq!(disabled.condition, Condition::Disabled);
        assert_eq!(disabled.former, None);
        assert_eq!(session.npc(id).expect("breaking up").former, None);
    }

    #[test]
    fn a_disabled_mission_escort_that_repairs_itself_resumes_and_never_joins_the_fleet() {
        let mut session = laden(1, RuleSource::Engine);
        let _ = (session.take_messages(), session.take_sounds());
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.self_repair = true;
        npc.reserves.armor.now = 1.0;
        let id = npc.id;
        session.tick_combat(crate::combat::Rules::default(), &mut NeverFires);
        assert_eq!(
            session.npc(id).expect("here").condition,
            Condition::Disabled
        );
        let mut chance = crate::testkit::Draws::of(&[0]);
        session.tick_combat(crate::combat::Rules::default(), &mut chance);
        assert_eq!(chance.asked, [crate::self_repair::REPAIR_ODDS]);
        let npc = session.npc(id).expect("here");
        assert_eq!(npc.condition, Condition::Intact);
        let max = npc.reserves.armor.max;
        assert!(
            (npc.reserves.armor.now - crate::self_repair::repaired_armor(max, false)).abs() < 1e-4
        );
        assert!(npc.mission_escort(), "resumes as a mission escort");
        assert_eq!(npc.escort, None);
        assert!(session.pilot().escorts().is_empty(), "never a hired escort");
        assert_eq!(session.take_messages(), []);
        assert_eq!(session.take_sounds(), []);
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
    fn a_move_in_flight_carries_the_hired_and_the_mission_escorts() {
        // The original's in-flight `M`/`N` rewrites only the system (+0x74)
        // of every active ship the player leads (`_EvalSetExp`
        // @0x15807-0x1583e, @0x1590b-0x1593d): they come as they are, in
        // the same formation and where they were.
        for expr in ["M131", "N131"] {
            let catalog = ships_catalog(escorting(-1, 0));
            let (mut session, ids) = flying_with_escorts(&catalog);
            let hired = session.fleet[0].expect("placed");
            let at = |session: &Session| -> Vec<_> {
                session
                    .player_led()
                    .map(|npc| (npc.id, npc.state.position))
                    .collect()
            };
            let before = at(&session);
            assert_eq!(hired_slot(&session), (2, 4));
            assert_eq!(slots(&session), [(3, 4), (4, 4)]);
            session.run_set(&SetExpr::parse(expr).expect("parses"), &mut NeverFires);
            settle(&mut session, &catalog);
            assert_eq!(session.system(), SystemId(131), "{expr}");
            assert!(
                session.npc(hired).is_some(),
                "{expr}: the hired escort comes"
            );
            assert_eq!(
                of(&session, &ids).len(),
                2,
                "{expr}: the mission escorts come"
            );
            for npc in of(&session, &ids) {
                assert!(npc.mission_escort(), "{expr}: still escorting");
            }
            assert_eq!(session.player_led().count(), 3, "{expr}");
            assert_eq!(hired_slot(&session), (2, 4), "{expr}: same formation");
            assert_eq!(slots(&session), [(3, 4), (4, 4)], "{expr}: same formation");
            assert_eq!(at(&session), before, "{expr}: where they were");
            ticks(&mut session, &catalog, 5);
            assert_eq!(of(&session, &ids).len(), 2, "{expr}: they stay");
            for npc in of(&session, &ids) {
                assert!(npc.mission_escort(), "{expr}: still escorting");
            }
        }
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
        assert_eq!(
            view.escort,
            Some(crate::hail::EscortStatus { wage: None }),
            "\"Status: Escort\" with no pay, as _CommDialogUpdate shows any ship the player leads"
        );
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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

    #[test]
    fn plain_r_picks_a_ship_attacking_a_mission_escort() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let escort = mission_escorts(&session)[0].id;
        let player = session.player.position;
        let mut hunter = crate::testkit::npc(0, crate::stats::ShipStats::default());
        hunter.state.position = player + crate::geometry::Vec2::new(0.0, 400.0);
        hunter.goal = Goal::Attack(crate::combat::ShipRef::Npc(escort));
        let attacker = session.traffic.add_npc(hunter);
        assert_eq!(
            session.select_target(TargetPick::NearestThreat),
            Some(attacker)
        );
        let led = session.npc_mut(escort).expect("there");
        led.goal = Goal::Attack(crate::combat::ShipRef::Player);
        led.state.position = player + crate::geometry::Vec2::new(5.0, 0.0);
        assert_eq!(
            session.select_target(TargetPick::NearestThreat),
            Some(attacker),
            "the nearer escort attacking the player is passed over"
        );
        let unled = session.npc_mut(escort).expect("there");
        unled.goal = Goal::Idle;
        unled.mission = None;
        session.target = None;
        assert_eq!(session.select_target(TargetPick::NearestThreat), None);
    }

    // Not a hired escort.

    #[test]
    fn a_mission_escort_is_no_hired_escort_and_the_nearest_target_passes_it_over() {
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
        assert_eq!(session.pilot.system, SystemId(130));
        // Closest Target skips every ship led by the player, mission
        // escorts too (`_FindNearestShipToPlayer` @0x92a6-0x92ae,
        // `_FindNearestThreatToPlayer` @0x94db), even one attacking him.
        for npc in session.traffic.npcs_mut() {
            npc.goal = Goal::Attack(crate::combat::ShipRef::Player);
        }
        assert_eq!(session.select_target(TargetPick::Nearest), None);
        assert_eq!(session.select_target(TargetPick::NearestThreat), None);
        let player = session.player.position;
        let npcs = session.traffic.npcs_mut();
        npcs[0].state.position = player + crate::geometry::Vec2::new(5.0, 0.0);
        npcs[1].state.position = player + crate::geometry::Vec2::new(500.0, 0.0);
        npcs[1].mission = None;
        let ordinary = npcs[1].id;
        assert_eq!(session.select_target(TargetPick::Nearest), Some(ordinary));
        assert_eq!(
            session.select_target(TargetPick::NearestThreat),
            Some(ordinary)
        );
        session.traffic.npcs_mut()[1].condition = Condition::Destroyed;
        assert_eq!(
            session.select_target(TargetPick::Nearest),
            Some(ordinary),
            "the target is left as it was"
        );
    }

    // Escort commands (`_IssueNewEscortCommand` @0x660d5).

    /// Each mission escort's standing order, in order.
    fn orders(session: &Session) -> Vec<Option<EscortOrder>> {
        mission_escorts(session)
            .iter()
            .map(|npc| npc.escort_order())
            .collect()
    }

    /// A session in flight with only mission 140's two escorts.
    fn flying(catalog: &FakePilotCatalog) -> Session {
        let mut session = accepted(catalog);
        take_off(&mut session, catalog);
        assert_eq!(mission_escorts(&session).len(), 2);
        session
    }

    #[test]
    fn an_escort_command_reaches_a_mission_escort_as_the_original_does() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Hold),
            Some(Commanded {
                group: EscortGroup::All,
                command: EscortCommand::Hold,
                targeted: false,
                underway: false,
            })
        );
        assert_eq!(orders(&session), [Some(EscortOrder::Hold); 2]);
        tick(&mut session, &catalog);
        for npc in mission_escorts(&session) {
            assert_eq!(npc.goal, Goal::Idle);
        }
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Hold),
            None,
            "nothing changed"
        );
    }

    #[test]
    fn a_class_command_reaches_a_mission_escort_of_its_class_only() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        let class = mission_escorts(&session)[0].class;
        let other = EscortClass::ALL
            .into_iter()
            .find(|&other| other != class)
            .expect("another class");
        assert_eq!(
            session.command_escorts(EscortGroup::Class(other), EscortCommand::Defend),
            None
        );
        assert_eq!(orders(&session), [None; 2]);
        assert!(
            session
                .command_escorts(EscortGroup::Class(class), EscortCommand::Defend)
                .is_some()
        );
        assert_eq!(orders(&session), [Some(EscortOrder::Defend); 2]);
    }

    #[test]
    fn return_to_hangar_sends_a_mission_escort_to_formation() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Dock),
            None,
            "already in formation"
        );
        assert_eq!(orders(&session), [None; 2]);
        session.command_escorts(EscortGroup::All, EscortCommand::Defend);
        assert_eq!(
            session
                .command_escorts(EscortGroup::All, EscortCommand::Dock)
                .map(|done| done.command),
            Some(EscortCommand::Recall)
        );
        assert_eq!(orders(&session), [None; 2]);
    }

    #[test]
    fn attack_never_copies_a_targeted_mission_escort_as_a_target() {
        let catalog = ships_catalog(escorting(-1, 0));
        let (mut session, ids) = flying_with_escorts(&catalog);
        session.target = Some(ids[0]);
        let done = session
            .command_escorts(EscortGroup::All, EscortCommand::Attack)
            .expect("the orders changed");
        assert!(!done.targeted);
        for npc in session.player_led() {
            assert_eq!(npc.target, None, "{:?}", npc.id);
            assert_eq!(npc.escort_order(), Some(EscortOrder::Attack));
        }
    }

    #[test]
    fn attack_copies_a_strangers_target_to_mission_escorts() {
        let catalog = ships_catalog(escorting(-1, 0));
        let (mut session, ids) = flying_with_escorts(&catalog);
        let stranger = session.place_escort(0).expect("a ship");
        session.npc_mut(stranger).expect("here").escort = None;
        session.target = Some(stranger);
        let done = session
            .command_escorts(EscortGroup::All, EscortCommand::Attack)
            .expect("changed");
        assert!(done.targeted);
        for npc in of(&session, &ids) {
            assert_eq!(npc.target, Some(crate::combat::ShipRef::Npc(stranger)));
        }
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Attack),
            None,
            "the same order and target again"
        );
    }

    // The escort menu (`_HandlePlayer` @0x6dda9, `_DrawEscortMenuInFrame`
    // @0x56392).

    /// The menu row of `class`.
    fn row(session: &Session, class: EscortClass) -> crate::escort::ClassRow {
        session
            .escort_menu()
            .into_iter()
            .find(|row| row.class == class)
            .expect("every class has a row")
    }

    #[test]
    fn the_escort_menu_counts_mission_escorts() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        let class = mission_escorts(&session)[0].class;
        for other in session.escort_menu() {
            assert_eq!(other.present, other.class == class, "{:?}", other.class);
            assert_eq!(other.order, None);
        }
        session.command_escorts(EscortGroup::All, EscortCommand::Hold);
        assert_eq!(row(&session, class).order, Some(EscortOrder::Hold));
        assert!(session.escorts_flying());
    }

    #[test]
    fn a_hired_escorts_order_heads_its_class_row_over_a_mission_escorts() {
        let catalog = ships_catalog(escorting(-1, 0));
        let (mut session, ids) = flying_with_escorts(&catalog);
        let class = session.npc(ids[0]).expect("here").class;
        assert_eq!(session.escort_class(0), class, "one class");
        session.pilot.escorts[0].order = Some(EscortOrder::Defend);
        assert_eq!(row(&session, class).order, Some(EscortOrder::Defend));
    }

    #[test]
    fn a_disabled_mission_escort_keeps_its_row_but_no_escort_is_flying() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        let class = mission_escorts(&session)[0].class;
        for npc in session.traffic.npcs_mut() {
            npc.condition = Condition::Disabled;
        }
        assert!(row(&session, class).present);
        assert!(!session.escorts_flying());
        session.traffic.npcs_mut()[1].condition = Condition::Intact;
        assert!(session.escorts_flying());
    }

    #[test]
    fn a_hired_escort_alone_is_flying() {
        let catalog = ships_catalog(escorting(-1, 0));
        let (mut session, ids) = flying_with_escorts(&catalog);
        for id in &ids {
            session.npc_mut(*id).expect("here").condition = Condition::Disabled;
        }
        assert!(session.escorts_flying());
        session.pilot.escorts.clear();
        assert!(!session.escorts_flying());
    }

    #[test]
    fn a_command_that_changes_something_beeps_and_one_that_changes_nothing_is_silent() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        session.take_sounds();
        session.command_escorts(EscortGroup::All, EscortCommand::Recall);
        assert_eq!(session.take_sounds(), []);
        session.command_escorts(EscortGroup::All, EscortCommand::Hold);
        assert_eq!(
            session.take_sounds(),
            [crate::SimSound::EscortsCommanded],
            "_beepSnd[1]"
        );
    }

    // Joining with the class order (`_EscortAI` @0x83ad8-0x83b2b).

    /// A session in flight with a hired escort of the mission escorts'
    /// class given `command`, mission 140's escorts of `ships` yet to
    /// come on their timer.
    fn commanded_before_they_come(catalog: &FakePilotCatalog, command: EscortCommand) -> Session {
        let mut session = accepted(catalog);
        session.pilot.escorts.push(hired());
        take_off(&mut session, catalog);
        assert!(mission_escorts(&session).is_empty());
        session.command_escorts(EscortGroup::All, command);
        ticks(&mut session, catalog, 199);
        assert_eq!(mission_escorts(&session).len(), 2);
        session
    }

    #[test]
    fn a_mission_escort_joining_takes_its_classs_order() {
        let catalog = ships_catalog(escorting(-1, 1));
        let session = commanded_before_they_come(&catalog, EscortCommand::Attack);
        assert_eq!(orders(&session), [Some(EscortOrder::Attack); 2]);
        let session = commanded_before_they_come(&catalog, EscortCommand::Defend);
        assert_eq!(orders(&session), [Some(EscortOrder::Defend); 2]);
    }

    #[test]
    fn a_mission_escort_joining_after_return_to_hangar_keeps_formation() {
        let catalog = ships_catalog(escorting(-1, 1));
        let mut session = accepted(&catalog);
        session.pilot.escorts.push(Escort {
            carried: true,
            ..hired()
        });
        take_off(&mut session, &catalog);
        session.command_escorts(EscortGroup::All, EscortCommand::Dock);
        assert_eq!(session.pilot.escorts[0].order, Some(EscortOrder::Dock));
        ticks(&mut session, &catalog, 199);
        assert_eq!(orders(&session), [None; 2]);
    }

    #[test]
    fn a_jump_resets_mission_escorts_orders_by_the_engine_and_keeps_the_classs_by_the_bible() {
        for (source, kept) in [
            (RuleSource::Engine, None),
            (RuleSource::Bible, Some(EscortOrder::Attack)),
        ] {
            let catalog = ships_catalog(escorting(-6, 0));
            let mut session = accepted(&catalog);
            session.rules = Rulebook::default().with_override(RuleKey::EscortOrders, source);
            session.pilot.escorts.push(hired());
            take_off(&mut session, &catalog);
            session.command_escorts(EscortGroup::All, EscortCommand::Attack);
            assert_eq!(orders(&session), [Some(EscortOrder::Attack); 2]);
            jump_to(&mut session, &catalog, 131);
            assert_eq!(orders(&session), [kept; 2], "{source:?}");
            assert_eq!(session.pilot.escorts[0].order, kept, "{source:?}");
        }
    }

    #[test]
    fn with_no_hired_escort_of_a_class_its_order_is_its_first_mission_escorts() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        let ids: Vec<NpcId> = mission_escorts(&session).iter().map(|npc| npc.id).collect();
        let class = session.npc(ids[1]).expect("here").class;
        let other = EscortClass::ALL
            .into_iter()
            .find(|&other| other != class)
            .expect("another class");
        for (id, class, order) in [
            (ids[0], other, EscortOrder::Attack),
            (ids[1], class, EscortOrder::Hold),
        ] {
            let npc = session.npc_mut(id).expect("here");
            npc.class = class;
            npc.mission.as_mut().expect("marked").order = Some(order);
        }
        assert_eq!(session.class_order(class), Some(EscortOrder::Hold));
        assert_eq!(session.class_order(other), Some(EscortOrder::Attack));
    }

    #[test]
    fn with_no_hired_escort_a_jump_keeps_the_class_order_by_the_bible_only() {
        for (source, kept) in [
            (RuleSource::Engine, None),
            (RuleSource::Bible, Some(EscortOrder::Attack)),
        ] {
            let catalog = ships_catalog(escorting(-6, 0));
            let mut session = accepted(&catalog);
            session.rules = Rulebook::default().with_override(RuleKey::EscortOrders, source);
            take_off(&mut session, &catalog);
            session.command_escorts(EscortGroup::All, EscortCommand::Attack);
            jump_to(&mut session, &catalog, 131);
            assert_eq!(orders(&session), [kept; 2], "{source:?}");
        }
    }

    #[test]
    fn a_class_command_keeps_only_its_classs_order_across_a_jump() {
        let catalog = ships_catalog(escorting(-6, 0));
        let mut session = accepted(&catalog);
        session.rules = Rulebook::default().with_override(RuleKey::EscortOrders, RuleSource::Bible);
        take_off(&mut session, &catalog);
        let class = mission_escorts(&session)[0].class;
        let other = EscortClass::ALL
            .into_iter()
            .find(|&other| other != class)
            .expect("another class");
        session.command_escorts(EscortGroup::Class(other), EscortCommand::Hold);
        jump_to(&mut session, &catalog, 131);
        assert_eq!(orders(&session), [None; 2]);
        session.command_escorts(EscortGroup::Class(class), EscortCommand::Hold);
        jump_to(&mut session, &catalog, 130);
        assert_eq!(orders(&session), [Some(EscortOrder::Hold); 2]);
    }

    #[test]
    fn the_fixture_disables_a_ship_the_player_cannot_shoot() {
        let catalog = ships_catalog(escorting(-1, 0));
        let mut session = flying(&catalog);
        let id = mission_escorts(&session)[0].id;
        crate::fixture::disable(&mut session, id);
        let npc = session.npc(id).expect("here");
        assert_eq!(npc.condition, Condition::Disabled);
        assert_eq!(
            (npc.reserves.shield.now, npc.reserves.armor.now),
            (0.0, 1.0)
        );
        assert!(session.escorts_flying(), "the other is intact");
    }
}
