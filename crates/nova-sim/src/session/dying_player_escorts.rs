//! The player's death frees its escorts
//! ([`Session::free_dying_players_escorts`]).
//!
//! `_EscortAI`'s lost parent (@0x83917-0x8397b) takes a ship whose parent
//! is `_IsDying` (@0x253d: its death timer set, or its armour at or below
//! none) out of its fleet: parent -1, goal 0, sub 0, its class's
//! `InherentAI` (@0x83c4a), its target kept. For the player (parent 0,
//! whose slot is never inactive) that frees, at each one's think, every
//! ship it leads: its hired and captured escorts and its launched
//! fighters, which drop AI type 5 and so their 1.333 capacities. No other
//! routine frees them at its death.
//!
//! Here the session frees them before the traffic's AI
//! (`Session::before_traffic_ai`, after the mission escorts join), so the
//! advance pass's think half sees it after the collision passes, as the
//! original's AI loop (@0x44270) follows them (@0x43e70-0x44015): a hit
//! there frees the escorts before the next frame's eject (`_HandlePlayer`
//! @0x6f22c), and a player who then ejects into a pod has lost them. Each
//! leaves the fleet as the pod releases it
//! ([`Session::release_for_the_pod`]). An escort's own fighters are led by
//! the escort, not the player, and stay. No rulebook key: no reading
//! disagrees with the original's.
//!
//! **Who `_EscortAI` reaches** ([`dispatch_gate::reaches_escort_think`]).
//! It doesn't run for a ship in goal 0x15, gliding or bursting
//! (@0x8fd38-0x8fd7e), on the jump path, or in goal 0x16 (@0x838ef), so a
//! ship the player leads in any of those is freed at the think it next
//! makes. A ship on the player's jump is cancelled (@0x900e3-0x9011e) and
//! freed the frame after, which the gate gives for free.
//!
//! **A mission escort let go**
//! ([`Npc::mission_let_go`](crate::traffic::npc::Npc::mission_let_go)).
//! `_AIDispatch` writes AI 6 and parent 0 for a `ShipBehav` 1 ship not
//! disabled (@0x8fc03-0x8fc22, [`dispatch_gate::led_again`]), and
//! `_EscortAI`'s lost parent then writes parent -1, goal 0 and its class's
//! `InherentAI`. The re-lead's parent 0 lives only inside its own dispatch,
//! where nothing reads it, so at every other reader, from its first think
//! after the player starts dying, it has parent -1 and that AI type: it is
//! drawn with the other ships (@0x2bd35)
//! and faintly when cloaked (@0x2d1b4), the radar, brackets, hit rules and
//! threat tests read a stranger, it flies at its own top speed, the pod's
//! walk fails `+0x9a == 0` (@0x6f71d) and leaves it to its next dispatch,
//! its mission's end only clears its mark (@0x8fc3a-0x8fc4b), and its own
//! break-up takes no cargo share (@0x350ab). It idles under its own think
//! ([`EscortAi`](crate::ai::EscortAi)). Each traffic tick here is that
//! dispatch: once the player is whole, it is led again and the formation
//! re-forms, the hired ships perhaps gone. A ship not intact isn't led
//! again (@0x8fc0c). Its +0xc92e and the player's +0xc2 have no reader
//! here (@0x440d8, @0x6f1cb): it flies sub 0, and no other ship keeps the
//! player's formation.
//!
//! **A captured mission ship.** One of another `ShipBehav` is no re-lead's
//! (@0x8fc07 → @0x8fc2d), so `_EscortAI` frees it for good
//! ([`dispatch_gate::not_led_again`]). One of `ShipBehav` 2 runs
//! `_DeathStarAI` in `_EscortAI`'s place (@0x8ff3a), and stays led.
//!
//! Nothing here plays a sound or draws a random number.

use super::{Session, Space};
use crate::ai::{Goal, dispatch_gate};
use crate::combat::hull::Condition;
use crate::traffic::npc::NpcId;

impl Session {
    /// While the player breaks up or is destroyed, every ship it leads
    /// whose think reaches `_EscortAI` leaves its fleet, but a `ShipBehav`
    /// 1 ship, which is let go until the next dispatch; a ship that thinks
    /// as `_DeathStarAI` stays led (see the module docs).
    pub(super) fn free_dying_players_escorts(&mut self) {
        let dying = self
            .ship
            .condition
            .dying_or_destroyed(self.pilot.reserves.armor);
        self.space.let_go_of_mission_escorts(dying);
        if !dying {
            return;
        }
        let led: Vec<NpcId> = self
            .space
            .traffic
            .npcs()
            .iter()
            .filter(|npc| dispatch_gate::reaches_escort_think(npc))
            .filter(|npc| npc.is_players() || dispatch_gate::not_led_again(npc))
            .map(|npc| npc.id)
            .collect();
        for id in led {
            self.release_for_the_pod(id);
        }
    }

    /// NPC `id`, a ship the player leads, is released (+0x9a = -1): out of
    /// the fleet or the formation ([`Session::unfleet`]), out of its fleet
    /// with its class's `InherentAI` and no carrier
    /// ([`Npc::leave_fleet`](crate::traffic::npc::Npc::leave_fleet)), and
    /// deciding afresh (goal 0); the rest re-form. The pod's release and
    /// the player's death share it.
    pub(super) fn release_led(&mut self, id: NpcId) {
        self.unfleet(id);
        if let Some(npc) = self.space.npc_mut(id) {
            npc.leave_fleet();
            npc.goal = Goal::Idle;
        }
        self.reform();
    }
}

impl Space {
    /// Each intact `ShipBehav` 1 ship is led again, as the dispatch leads
    /// it ([`dispatch_gate::led_again`]), and let go again if the player is
    /// `dying` and its think reaches `_EscortAI`. One not intact isn't led
    /// again (@0x8fc0c), so a let go stands. One that rejoins takes its slot
    /// from the same pass's [`Session::reform_on_swarm_change`], the hired
    /// ships perhaps gone meanwhile.
    pub(super) fn let_go_of_mission_escorts(&mut self, dying: bool) {
        for npc in self
            .traffic
            .npcs_mut()
            .iter_mut()
            .filter(|npc| npc.condition == Condition::Intact)
        {
            npc.mission_let_go =
                dying && dispatch_gate::led_again(npc) && dispatch_gate::reaches_escort_think(npc);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::{Behaviour, Surroundings};
    use crate::bay::Carrier;
    use crate::chance::{Chance, NeverFires};
    use crate::combat::hull::Condition;
    use crate::combat::{Rules, ShipRef};
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::flight::Controls;
    use crate::mission::ships::ShipBehav;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::stats::ShipStats;
    use crate::testkit::work_with;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, npc};
    use crate::traffic::npc::{AiType, MissionMark, Mode, Npc};
    use crate::traffic::warp_out::WarpGoal;

    /// Every ship keeps its goal and target.
    #[derive(Debug)]
    struct Keeping;

    impl Behaviour for Keeping {
        fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    /// A ship of a warship's class (`InherentAI` 3) flying as a trader,
    /// as a captured ship keeps its own type, targeting the player.
    fn ship() -> Npc {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.ai_type = AiType::WimpyTrader;
        ship.hull.inherent_ai = 3;
        ship.target = Some(ShipRef::Player);
        ship
    }

    /// The player's escort record, `carried` for a fighter.
    fn record(carried: bool) -> Escort {
        Escort {
            ship: crate::catalog::ShipId(129),
            reserves: Reserves::full(1.0, 1.0, 1.0),
            order: None,
            carried,
            wage: None,
            person: None,
            mark: None,
        }
    }

    /// The ships of the player's fleet: a hired escort, a fighter it
    /// launched, a mission escort, and the escort's own fighter.
    struct Fleet {
        session: Session,
        escort: NpcId,
        fighter: NpcId,
        mission: NpcId,
        escorts_fighter: NpcId,
    }

    fn fleet() -> (Fleet, FakePilotCatalog) {
        let catalog = catalog();
        let mut session =
            Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies");
        // Its first traffic tick enters the system, before the fleet
        // arrives.
        traffic_tick(&mut session, &catalog);
        let duty = EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        };
        let escort = session.space.traffic.add_npc(Npc {
            escort: Some(duty),
            goal: Goal::Formation { guard: None },
            ..ship()
        });
        session.pilot.escorts.push(record(false));
        session.space.escorts.fleet.push(Some(escort));
        let mut fighter = Npc {
            escort: Some(duty),
            goal: Goal::Formation { guard: None },
            ..ship()
        };
        fighter.take_carrier(session.player_carrier(30), RuleSource::Engine);
        let fighter = session.space.traffic.add_npc(fighter);
        session.pilot.escorts.push(record(true));
        session.space.escorts.fleet.push(Some(fighter));
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 4));
        let mission = session.space.traffic.add_npc(Npc {
            mission: Some(MissionMark {
                slot: 0,
                behav: ShipBehav::EscortPlayer,
                formation: Some(FormationSlot::default()),
                order: None,
            }),
            goal: Goal::Formation { guard: None },
            ..ship()
        });
        let mut escorts_fighter = Npc {
            leader: Some(escort),
            goal: Goal::Formation { guard: None },
            ..ship()
        };
        escorts_fighter.take_carrier(
            Carrier {
                ship: ShipRef::Npc(escort),
                window: 100.0,
                reach: 40.0,
            },
            RuleSource::Engine,
        );
        let escorts_fighter = session.space.traffic.add_npc(escorts_fighter);
        session.reform();
        (
            Fleet {
                session,
                escort,
                fighter,
                mission,
                escorts_fighter,
            },
            catalog,
        )
    }

    /// A traffic tick, its work before the AI among it.
    fn traffic_tick(session: &mut Session, catalog: &FakePilotCatalog) {
        work_with(
            session,
            catalog,
            Controls::default(),
            &Keeping,
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// An advance pass.
    fn advance_pass(session: &mut Session, catalog: &FakePilotCatalog) {
        session.game_work(
            crate::flight::Controls::default(),
            catalog,
            &Keeping,
            Rules::default(),
            &mut NeverFires,
        );
    }

    type Tick = fn(&mut Session, &FakePilotCatalog);

    const TICKS: [(&str, Tick); 2] = [("traffic", traffic_tick), ("advance", advance_pass)];

    /// A traffic tick by Nova's AI.
    fn nova_traffic_tick(session: &mut Session, catalog: &FakePilotCatalog) {
        work_with(
            session,
            catalog,
            Controls::default(),
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// An advance pass by Nova's AI.
    fn nova_advance_pass(session: &mut Session, catalog: &FakePilotCatalog) {
        session.game_work(
            crate::flight::Controls::default(),
            catalog,
            &crate::ai::NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// The player breaking up, as `_IsDying` reads it each way: its death
    /// timer set, or its armour gone.
    fn dying(session: &mut Session, timer: bool) {
        if timer {
            session.ship.condition = Condition::Dying {
                ticks_left: 30,
                starting: false,
            };
        } else {
            session.pilot.reserves.armor.now = 0.0;
        }
    }

    #[test]
    fn while_the_player_breaks_up_its_escorts_and_fighters_leave_its_fleet() {
        for (name, tick) in TICKS {
            for timer in [true, false] {
                let (mut fleet, catalog) = fleet();
                dying(&mut fleet.session, timer);
                tick(&mut fleet.session, &catalog);
                let session = &fleet.session;
                for id in [fleet.escort, fleet.fighter] {
                    let freed = session.space.npc(id).expect("here");
                    assert!(!freed.escorts_player(), "{name} {timer}: {id:?}");
                    assert_eq!(freed.ai_type, AiType::Warship, "{name} {timer}");
                    assert_eq!(freed.goal, Goal::Idle, "{name} {timer}");
                    assert_eq!(freed.carrier, None, "{name} {timer}");
                    assert_eq!(freed.target, Some(ShipRef::Player), "{name} {timer}");
                }
                let fighter = session.space.npc(fleet.fighter).expect("here");
                assert_eq!(
                    (fighter.reserves.shield.max, fighter.reserves.armor.max),
                    (30.0, 45.0),
                    "{name} {timer}: its class's capacities"
                );
                assert_eq!(session.pilot.escorts, [], "{name} {timer}");
                assert_eq!(session.space.escorts.fleet, [], "{name} {timer}");
                let own = session.space.npc(fleet.escorts_fighter).expect("here");
                assert_eq!(
                    (own.leader, own.carrier.map(|bay| bay.ship), own.ai_type),
                    (
                        Some(fleet.escort),
                        Some(ShipRef::Npc(fleet.escort)),
                        AiType::WimpyTrader
                    ),
                    "{name} {timer}: the escort's own fighter stays its"
                );
            }
        }
    }

    const NOVA_TICKS: [(&str, Tick); 2] = [
        ("traffic", nova_traffic_tick),
        ("advance", nova_advance_pass),
    ];

    /// A mission mark of `behav`, in the formation.
    fn mark(behav: ShipBehav) -> MissionMark {
        MissionMark {
            slot: 0,
            behav,
            formation: Some(FormationSlot::default()),
            order: None,
        }
    }

    fn let_go(session: &Session, id: NpcId) -> bool {
        session.space.npc(id).expect("here").mission_let_go
    }

    #[test]
    fn a_mission_escort_is_let_go_idling_keeping_its_target_and_its_mark() {
        for (name, tick) in NOVA_TICKS {
            let (mut fleet, catalog) = fleet();
            dying(&mut fleet.session, true);
            tick(&mut fleet.session, &catalog);
            let mission = fleet.session.space.npc(fleet.mission).expect("here");
            assert!(mission.mission_let_go, "{name}");
            assert!(!mission.escorts_player(), "{name}");
            assert!(
                mission.mission.is_some_and(|mark| mark.formation.is_some()),
                "{name}: its mark stays, as +0xc92e does"
            );
            assert_eq!(mission.goal, Goal::Idle, "{name}");
            assert_eq!(mission.target, Some(ShipRef::Player), "{name}");
            assert_eq!(mission.ai_type, AiType::WimpyTrader, "{name}");
            assert_eq!(mission.original_ai_type(), 3, "{name}: its class's");
        }
    }

    #[test]
    fn a_mission_escort_let_go_is_drawn_with_the_other_ships() {
        // `_HandleShipDisplay`'s layer reads +0x9a (@0x2bd35).
        for overlap in RuleSource::ALL {
            let (mut fleet, catalog) = fleet();
            let session = &mut fleet.session;
            session.setup.rules = Rulebook::default().with_override(RuleKey::ShipOverlap, overlap);
            let stranger = session.space.traffic.add_npc(ship());
            let late = session.space.traffic.add_npc(Npc {
                mission: Some(mark(ShipBehav::EscortPlayer)),
                goal: Goal::Formation { guard: None },
                ..ship()
            });
            session.reform();
            traffic_tick(session, &catalog);
            let at = |session: &Session, id: NpcId| {
                let drawn = ShipRef::Npc(id);
                session.draw_order().iter().position(|&ship| ship == drawn)
            };
            assert!(at(session, late) < at(session, stranger), "{overlap:?}");
            dying(session, true);
            traffic_tick(session, &catalog);
            traffic_tick(session, &catalog);
            assert!(at(session, stranger) < at(session, late), "{overlap:?}");
        }
    }

    #[test]
    fn a_mission_escort_let_go_flies_at_its_own_top_speed() {
        // `_ShipMaxSpeed` reads +0x9a.
        let (mut fleet, catalog) = fleet();
        let flat_out = |fleet: &Fleet| {
            let npc = fleet.session.space.npc(fleet.mission).expect("here");
            (npc.top_speed(2.0), npc.own_handling().max_speed)
        };
        let (led, own) = flat_out(&fleet);
        assert_eq!(led, own * 2.0);
        dying(&mut fleet.session, true);
        traffic_tick(&mut fleet.session, &catalog);
        let (free, own) = flat_out(&fleet);
        assert_eq!(free, own);
    }

    #[test]
    fn the_dispatch_leads_a_mission_escort_again_once_the_player_is_whole() {
        for (name, tick) in NOVA_TICKS {
            let (mut fleet, catalog) = fleet();
            dying(&mut fleet.session, true);
            tick(&mut fleet.session, &catalog);
            fleet.session.ship.condition = Condition::Intact;
            tick(&mut fleet.session, &catalog);
            let mission = fleet.session.space.npc(fleet.mission).expect("here");
            assert!(!mission.mission_let_go, "{name}");
            assert!(mission.mission_escort(), "{name}");
            assert_eq!(
                mission.goal,
                Goal::Formation { guard: None },
                "{name}: from its think"
            );
            let slot = mission.formation().expect("in the formation");
            assert_eq!((slot.slot, slot.ships), (2, 2), "{name}: alone in it");
        }
    }

    #[test]
    fn a_disabled_mission_escort_is_not_led_again_until_it_is_whole() {
        // @0x8fc0c.
        let (mut fleet, catalog) = fleet();
        dying(&mut fleet.session, true);
        traffic_tick(&mut fleet.session, &catalog);
        let session = &mut fleet.session;
        let mission = session.space.npc_mut(fleet.mission).expect("here");
        mission.condition = Condition::Disabled;
        session.ship.condition = Condition::Intact;
        traffic_tick(session, &catalog);
        assert!(let_go(session, fleet.mission));
        let mission = session.space.npc_mut(fleet.mission).expect("here");
        mission.condition = Condition::Intact;
        traffic_tick(session, &catalog);
        assert!(!let_go(session, fleet.mission));
    }

    #[test]
    fn a_disabled_mission_escort_stays_led_through_the_players_death() {
        // Not re-led, so not let go here: its +0x9a is
        // rdm:phase/escort-fleet-fidelity-followups/phase-16-disabled-mission-escort-parent's.
        let (mut fleet, catalog) = fleet();
        let session = &mut fleet.session;
        let mission = session.space.npc_mut(fleet.mission).expect("here");
        mission.condition = Condition::Disabled;
        dying(session, true);
        traffic_tick(session, &catalog);
        assert!(!let_go(session, fleet.mission));
    }

    #[test]
    fn a_mission_ending_while_let_go_clears_only_the_mark() {
        // With +0x9a -1 the slot-freed branch leaves the ship (@0x8fc3a-0x8fc4b).
        let (mut fleet, catalog) = fleet();
        dying(&mut fleet.session, true);
        nova_traffic_tick(&mut fleet.session, &catalog);
        let session = &mut fleet.session;
        assert!(session.pilot.missions.free(0).is_some(), "finished");
        traffic_tick(session, &catalog);
        let mission = session.space.npc(fleet.mission).expect("here");
        assert_eq!(mission.mission, None);
        assert!(!mission.mission_let_go);
        assert_eq!(mission.goal, Goal::Idle, "no leave");
        assert_eq!(mission.ai_type, AiType::Warship);
        assert_eq!(mission.original_ai_type(), 3);
    }

    /// The fleet with a ship of the mission marked `behav` beside it,
    /// which the player's death reaches.
    fn with_captured(behav: ShipBehav) -> (Fleet, FakePilotCatalog, NpcId) {
        let (mut fleet, catalog) = fleet();
        let captured = fleet.session.space.traffic.add_npc(Npc {
            mission: Some(mark(behav)),
            goal: Goal::Formation { guard: None },
            ..ship()
        });
        fleet.session.reform();
        (fleet, catalog, captured)
    }

    #[test]
    fn a_captured_mission_ship_of_another_behav_is_freed_for_good() {
        // No re-lead's (@0x8fc07 → @0x8fc2d).
        for behav in [ShipBehav::AttackPlayer, ShipBehav::Own] {
            let (mut fleet, catalog, captured) = with_captured(behav);
            dying(&mut fleet.session, true);
            traffic_tick(&mut fleet.session, &catalog);
            let freed = fleet.session.space.npc(captured).expect("here");
            assert_eq!(
                freed.mission.and_then(|mark| mark.formation),
                None,
                "{behav:?}"
            );
            assert!(freed.mission.is_some(), "{behav:?}: its mark stays");
            assert!(!freed.mission_let_go, "{behav:?}");
            assert_eq!(freed.ai_type, AiType::Warship, "{behav:?}");
            fleet.session.ship.condition = Condition::Intact;
            traffic_tick(&mut fleet.session, &catalog);
            let freed = fleet.session.space.npc(captured).expect("here");
            assert!(!freed.mission_escort(), "{behav:?}: not led again");
        }
    }

    #[test]
    fn a_captured_behav_2_ship_stays_led() {
        // `_DeathStarAI` runs in `_EscortAI`'s place (@0x8ff3a).
        let (mut fleet, catalog, captured) = with_captured(ShipBehav::AttackStellars);
        dying(&mut fleet.session, true);
        traffic_tick(&mut fleet.session, &catalog);
        let led = fleet.session.space.npc(captured).expect("here");
        assert!(led.mission_escort());
        assert!(!led.mission_let_go);
        assert_eq!(led.ai_type, AiType::WimpyTrader);
    }

    #[test]
    fn a_ship_in_its_gate_or_on_the_jump_path_or_in_a_count_wait_is_freed_at_its_think() {
        type Change = fn(&mut Npc);
        let cases: [(&str, Change, Change); 3] = [
            (
                "gate",
                |npc| npc.mode = Mode::Emerging { ticks_left: 30 },
                |npc| npc.mode = Mode::Flying,
            ),
            (
                "jump",
                |npc| npc.warp.goal = Some(WarpGoal::Own),
                |npc| npc.warp.goal = None,
            ),
            (
                "count wait",
                |npc| npc.goal = Goal::CountWait,
                |npc| npc.goal = Goal::Idle,
            ),
        ];
        for (name, waits, thinks) in cases {
            let (mut fleet, catalog) = fleet();
            let session = &mut fleet.session;
            waits(session.space.npc_mut(fleet.escort).expect("here"));
            waits(session.space.npc_mut(fleet.mission).expect("here"));
            dying(session, true);
            traffic_tick(session, &catalog);
            for id in [fleet.escort, fleet.mission] {
                let npc = session.space.npc(id).expect("here");
                assert!(npc.escorts_player(), "{name}: {id:?}");
            }
            thinks(session.space.npc_mut(fleet.escort).expect("here"));
            thinks(session.space.npc_mut(fleet.mission).expect("here"));
            traffic_tick(session, &catalog);
            let hired = session.space.npc(fleet.escort).expect("here");
            assert!(!hired.escorts_player(), "{name}");
            assert_eq!(hired.ai_type, AiType::Warship, "{name}");
            assert!(let_go(session, fleet.mission), "{name}");
        }
    }

    #[test]
    fn an_intact_or_disabled_player_frees_no_one() {
        for (name, tick) in TICKS {
            for condition in [Condition::Intact, Condition::Disabled] {
                let (mut fleet, catalog) = fleet();
                fleet.session.ship.condition = condition;
                fleet.session.pilot.reserves.armor.now = 1.0;
                tick(&mut fleet.session, &catalog);
                for id in [fleet.escort, fleet.fighter] {
                    let kept = fleet.session.space.npc(id).expect("here");
                    assert!(kept.escorts_player(), "{name} {condition:?}");
                }
                assert_eq!(fleet.session.pilot.escorts.len(), 2, "{name} {condition:?}");
            }
        }
    }
}
