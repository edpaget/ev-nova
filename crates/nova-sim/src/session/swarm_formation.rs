//! The swarm's formation parent applied to the player's formation
//! (`_DoPlayGameWork` @0x4405e-0x441f7 in the `EV Nova` executable; see
//! `nova_sim::ai::swarm`): an escort, fighter or mission escort the player
//! leads that attacks or snipes with a target and a swarm leader forms on
//! its swarm leader, not the player, so it leaves the player's formation
//! and the rest close up ([`Session::formation_slots`]). It keeps the slot
//! it held, which its attack doesn't read, and takes a new one as it
//! rejoins.
//!
//! The original marks every formation parent each frame before the AI
//! runs (`_DoPlayGameWork` @0x4405e, `_AIDispatch` @0x4429d), so a frame's
//! formation is by the goals, targets and swarm leaders the last frame
//! left. The engine re-forms at the same point, just before the traffic's
//! tick ([`Session::reform_on_swarm_change`]), whenever a slot the
//! formation would give differs from the one its ship holds.
//!
//! A ship that left takes a slot in its swarm leader's formation from the
//! traffic's formation pass
//! ([`fleet_formation`](crate::traffic::fleet_formation),
//! [`Npc::fleet_slot`](crate::traffic::npc::Npc::fleet_slot)), and is
//! nudged toward it in substates 5, 0x11 and 7 (`nova_sim_ai`'s
//! `swarm_low_level` module).
//!
//! The formation numbers every ship whose parent is the player, not only
//! the ships it leads ([`Session::formation_slots`]): `_HandlePlayer`'s
//! `_AICalcFormationPositions` reads +0xc92e alone (@0x6f1cb-0x6f1df). So
//! a ship the player lost keeps its number until it's gone, its AI type
//! down and its parent the last pass's: an escort or fighter a hit took
//! (`_DamageShip` @0x3b0d7), a mission escort dying or sent off
//! (`_HandleShip` @0x3512b, `_AbortMission` @0x99af8). The check here sees
//! it go, and the rest close up. A release writes -1 (`_AISetup`
//! @0x7f41e), so it leaves no gap. A fleet escort attacking on a word of 0
//! counts here too, whoever leads it.

use super::Session;

impl Session {
    /// Re-forms the player's formation ([`Session::reform`]) when a slot
    /// it would give ([`Session::formation_slots`]) is not the one its
    /// ship holds: a ship left it or rejoined it as its formation parent
    /// changed (see the module docs).
    pub(super) fn reform_on_swarm_change(&mut self) {
        let stale = self.formation_slots().into_iter().any(|(id, slot)| {
            self.space
                .npc(id)
                .and_then(|npc| npc.formation().or(npc.fleet_slot))
                != Some(slot)
        });
        if stale {
            self.reform();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{Behaviour, Goal, Peaceful, Surroundings};
    use crate::catalog::{HullRecord, ShipId, ShipRecord};
    use crate::chance::{Chance, NeverFires};
    use crate::combat::Rules;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort::FormationSlot;
    use crate::flight::Controls;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::testkit::work_with;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, hull, ship};
    use crate::traffic::npc::{Npc, NpcId};

    /// The escorts' ship type, 40 pixels across.
    const ESCORT: ShipId = ShipId(150);

    /// [`catalog`] with the escorts' ship type, as fast as the player.
    fn fleeted() -> FakePilotCatalog {
        FakePilotCatalog {
            ship_records: vec![
                ship(128, FAST),
                ShipRecord {
                    escort_type: 2,
                    inherent_ai: 3,
                    ..ship(150, FAST)
                },
            ],
            hulls: vec![
                HullRecord {
                    size: Some(40),
                    ..hull(128)
                },
                HullRecord {
                    size: Some(40),
                    ..hull(150)
                },
            ],
            ..catalog()
        }
    }

    /// A session flying three hired escorts, at berths 1-3, its first
    /// traffic tick done.
    fn escorted(catalog: &FakePilotCatalog) -> Session {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.escorts = vec![
            Escort {
                ship: ESCORT,
                reserves: Reserves::full(30.0, 45.0, 300.0),
                order: None,
                carried: false,
                wage: None,
                person: None,
                mark: None,
                voice: None,
                berth: None,
            };
            3
        ];
        let mut session = Session::fly(catalog, pilot).expect("flies");
        work_with(
            &mut session,
            catalog,
            Controls::default(),
            &Peaceful,
            Rules::default(),
            &mut NeverFires,
        );
        session
    }

    /// The escorts in berth order.
    fn escorts(session: &Session) -> Vec<Npc> {
        let mut npcs: Vec<Npc> = session.player_led().cloned().collect();
        npcs.sort_by_key(|npc| npc.berth);
        npcs
    }

    /// The (slot, ships) each escort holds, in berth order.
    fn slots(session: &Session) -> Vec<(u8, u8)> {
        escorts(session)
            .iter()
            .map(|npc| {
                npc.formation()
                    .map_or((0, 0), |slot| (slot.slot, slot.ships))
            })
            .collect()
    }

    /// The quarry the swarming escorts attack.
    const QUARRY: ShipRef = ShipRef::Npc(NpcId(99));

    /// The escort at `berth` attacking [`QUARRY`], of a swarming class,
    /// naming `leader` as its swarm leader.
    fn attacking(session: &mut Session, berth: u32, leader: Option<u32>) {
        let id = escorts(session)
            .into_iter()
            .find(|npc| npc.berth == berth)
            .map(|npc| npc.id)
            .expect("an escort there");
        let npc = session.space.npc_mut(id).expect("in the system");
        npc.hull.swarms = true;
        npc.goal = Goal::Attack(QUARRY);
        npc.target = Some(QUARRY);
        npc.swarm_leader = leader;
    }

    #[test]
    fn an_escort_attacking_with_a_swarm_leader_leaves_the_players_formation_and_the_rest_re_slot() {
        let catalog = fleeted();
        let mut session = escorted(&catalog);
        assert_eq!(
            escorts(&session)
                .iter()
                .map(|npc| npc.berth)
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(slots(&session), [(2, 4), (3, 4), (4, 4)]);
        attacking(&mut session, 1, None);
        attacking(&mut session, 2, Some(1));
        session.reform_on_swarm_change();
        // Berth 2 leaves (its slot as it was); berth 3 goes from 4 of 4 to
        // 3 of 3, counting the player.
        assert_eq!(slots(&session), [(2, 3), (3, 4), (3, 3)]);
        let spacing = escorts(&session)[0].formation().map(|slot| slot.spacing);
        assert_eq!(
            escorts(&session)[2].formation(),
            spacing.map(|spacing| FormationSlot {
                slot: 3,
                ships: 3,
                spacing,
            })
        );
    }

    #[test]
    fn it_rejoins_once_it_stops_attacking() {
        let catalog = fleeted();
        let mut session = escorted(&catalog);
        attacking(&mut session, 1, None);
        attacking(&mut session, 2, Some(1));
        session.reform_on_swarm_change();
        let id = escorts(&session)[1].id;
        if let Some(npc) = session.space.npc_mut(id) {
            npc.goal = Goal::Formation { guard: None };
        }
        session.reform_on_swarm_change();
        assert_eq!(slots(&session), [(2, 4), (3, 4), (4, 4)]);
    }

    #[test]
    fn an_escort_attacking_with_no_swarm_leader_stays() {
        let catalog = fleeted();
        let mut session = escorted(&catalog);
        attacking(&mut session, 1, None);
        attacking(&mut session, 2, None);
        session.reform_on_swarm_change();
        assert_eq!(slots(&session), [(2, 4), (3, 4), (4, 4)]);
    }

    #[test]
    fn a_rejoined_escort_with_a_leftover_fleet_slot_flies_the_slot_it_holds() {
        let catalog = fleeted();
        let mut session = escorted(&catalog);
        let escort = escorts(&session)[0].clone();
        let held = escort.formation().expect("in formation");
        let leftover = FormationSlot {
            slot: 7,
            ships: 8,
            spacing: 60.0,
        };
        if let Some(npc) = session.space.npc_mut(escort.id) {
            npc.fleet_slot = Some(leftover);
        }
        session.reform_on_swarm_change();
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &Peaceful,
            Rules::default(),
            &mut NeverFires,
        );
        let npc = session.space.npc(escort.id).expect("there");
        assert_eq!(npc.formation(), Some(held));
        assert_eq!(npc.slot_place, Some(held.position(&session.ship.player)));
    }

    #[test]
    fn a_released_escort_leaves_no_gap() {
        // `_AISetup` writes -1 to +0xc906 and +0xc92e (@0x7f415-0x7f41e), so
        // no pass numbers it, as the ship flies off.
        let catalog = fleeted();
        let mut session = escorted(&catalog);
        let id = escorts(&session)[0].id;
        session.space.npc_mut(id).expect("there").swarm_leader = Some(2);
        session.release(id);
        let released = session.space.npc(id).expect("there");
        assert_eq!(released.form_parent, None);
        assert_eq!(released.swarm_leader, None);
        assert_eq!(slots(&session), [(2, 3), (3, 3)], "straight after");
    }

    /// The (slot, ships) of the ship `id`, a lost one, and of the others
    /// in berth order, after a marking pass and the fleet losing it.
    fn lost(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        id: NpcId,
    ) -> ((u8, u8), Vec<(u8, u8)>) {
        work_with(
            session,
            catalog,
            Controls::default(),
            &Peaceful,
            Rules::default(),
            &mut NeverFires,
        );
        session.space.npc_mut(id).expect("there").condition = Condition::Disabled;
        assert!(session.unfleet(id));
        let lost = session.space.npc(id).expect("there");
        assert_eq!(lost.escort, None);
        let slot = lost.fleet_slot.expect("numbered");
        ((slot.slot, slot.ships), slots(session))
    }

    #[test]
    fn a_lost_escort_keeps_its_number_while_its_ship_stays() {
        let catalog = fleeted();
        let mut session = escorted(&catalog);
        let id = escorts(&session)[0].id;
        assert_eq!(
            lost(&mut session, &catalog, id),
            ((2, 4), vec![(3, 4), (4, 4)])
        );
        // It reads its class's type and holds the player, the store.
        let npc = session.space.npc(id).expect("there");
        assert_eq!(npc.original_ai_type(), 3);
        assert_eq!(npc.form_parent, Some(0));
        // A tick on it stays, and so does the rest's formation.
        session.reform_on_swarm_change();
        assert_eq!(slots(&session), [(3, 4), (4, 4)]);
        // The rest close up once it's gone.
        session.space.traffic.remove(id);
        session.reform_on_swarm_change();
        assert_eq!(slots(&session), [(2, 3), (3, 3)]);
    }

    #[test]
    fn a_lost_fighter_keeps_its_number_while_its_ship_stays() {
        let catalog = fleeted();
        let mut session = escorted(&catalog);
        let id = escorts(&session)[0].id;
        let carrier = session.player_carrier(5);
        let fighter = session.space.npc_mut(id).expect("there");
        fighter.carrier = Some(carrier);
        assert_eq!(
            lost(&mut session, &catalog, id),
            ((2, 4), vec![(3, 4), (4, 4)])
        );
        let npc = session.space.npc(id).expect("there");
        assert!(npc.carrier.is_some(), "kept for its capacities");
        assert_eq!(npc.original_ai_type(), 3, "its class's, not AI 5");
        session.space.traffic.remove(id);
        session.reform_on_swarm_change();
        assert_eq!(slots(&session), [(2, 3), (3, 3)]);
    }

    #[test]
    fn a_fleet_escort_attacking_on_word_0_counts_in_the_players_formation() {
        // @0x44195: parent 0 whoever leads it, and @0x6f1cb-0x6f1df numbers
        // it with the player's own. By berth, so it moves them on.
        let catalog = fleeted();
        let mut session = escorted(&catalog);
        let mut fleet = crate::traffic::fixture::npc(0, crate::stats::ShipStats::default());
        fleet.leader = Some(NpcId(77));
        fleet.goal = Goal::Attack(QUARRY);
        fleet.target = Some(QUARRY);
        fleet.swarm_leader = Some(0);
        fleet.hull.sprite_size = Some(40.0);
        let berth = escorts(&session)[1].berth;
        let lowered: Vec<NpcId> = escorts(&session).iter().skip(1).map(|npc| npc.id).collect();
        for id in lowered {
            let npc = session.space.npc_mut(id).expect("there");
            npc.berth += 10;
        }
        let id = session.space.traffic.add_npc_at(fleet, berth);
        session.reform_on_swarm_change();
        let npc = session.space.npc(id).expect("there");
        assert_eq!(
            npc.fleet_slot.map(|slot| (slot.slot, slot.ships)),
            Some((3, 5))
        );
        assert_eq!(npc.formation(), None, "no duty");
        assert_eq!(slots(&session), [(2, 5), (4, 5), (5, 5)]);
        // A second pass finds the same slots and leaves them.
        let number = session.space.npc(id).expect("there").fleet_slot;
        session.reform_on_swarm_change();
        assert_eq!(slots(&session), [(2, 5), (4, 5), (5, 5)]);
        assert_eq!(session.space.npc(id).expect("there").fleet_slot, number);
        // Its number alone differing re-forms: the duties all still match.
        session.space.npc_mut(id).expect("there").fleet_slot = Some(FormationSlot {
            slot: 9,
            ships: 9,
            spacing: 60.0,
        });
        session.reform_on_swarm_change();
        assert_eq!(session.space.npc(id).expect("there").fleet_slot, number);
        assert_eq!(slots(&session), [(2, 5), (4, 5), (5, 5)]);
    }

    /// Each escort but the one at berth 3 attacks its ship, which keeps
    /// out of it.
    #[derive(Debug)]
    struct TwoAttack(ShipRef);

    impl TwoAttack {
        fn attacks(&self, npc: &Npc) -> bool {
            npc.berth != 3 && ShipRef::Npc(npc.id) != self.0
        }
    }

    impl Behaviour for TwoAttack {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            if self.attacks(npc) {
                Goal::Attack(self.0)
            } else {
                Goal::Formation { guard: None }
            }
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            self.attacks(npc).then_some(self.0)
        }
    }

    #[test]
    fn the_traffic_tick_re_forms_on_the_swarm_leaders_its_pass_named() {
        let catalog = fleeted();
        let mut session = escorted(&catalog);
        for id in escorts(&session)
            .iter()
            .map(|npc| npc.id)
            .collect::<Vec<_>>()
        {
            if let Some(npc) = session.space.npc_mut(id) {
                npc.hull.swarms = true;
            }
        }
        // A ship in the system, so no slot test lets go of it.
        let quarry = crate::testkit::npc(0, crate::stats::ShipStats::default());
        let two_attack = TwoAttack(ShipRef::Npc(session.space.traffic.add_npc_at(quarry, 20)));
        // The ticks take the attacks and the targets, the pass names berth
        // 1 for berth 2 alone, and each tick forms by the last's before its
        // AI runs, as `_DoPlayGameWork` marks the parents before
        // `_AIDispatch`.
        for _ in 0..3 {
            work_with(
                &mut session,
                &catalog,
                Controls::default(),
                &two_attack,
                Rules::default(),
                &mut NeverFires,
            );
        }
        let named: Vec<Option<u32>> = escorts(&session)
            .iter()
            .map(|npc| npc.swarm_leader)
            .collect();
        assert_eq!(named, [None, Some(1), None]);
        assert_eq!(slots(&session)[2], (3, 3));
    }
}
