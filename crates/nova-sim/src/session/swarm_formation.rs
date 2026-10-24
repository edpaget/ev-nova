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
//! [`Npc::fleet_slot`]). Its `_AIMaintainFormation` calls in substates 5,
//! 0x11 and 7 are
//! rdm:phase/combat-ai-fidelity-cross-roadmap/phase-1-left-out-of-combat-ai-fidelity-14's
//! item 2.

use super::Session;
use crate::traffic::npc::Npc;

impl Session {
    /// Re-forms the player's formation ([`Session::reform`]) when a slot
    /// it would give ([`Session::formation_slots`]) is not the one its
    /// ship holds: a ship left it or rejoined it as its formation parent
    /// changed (see the module docs).
    pub(super) fn reform_on_swarm_change(&mut self) {
        let stale = self
            .formation_slots()
            .into_iter()
            .any(|(id, slot)| self.space.npc(id).and_then(Npc::formation) != Some(slot));
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
    use crate::combat::ShipRef;
    use crate::escort::FormationSlot;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, hull, ship};
    use crate::traffic::npc::NpcId;

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
            };
            3
        ];
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.tick_traffic(catalog, &Peaceful, &mut NeverFires);
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

    /// Each escort but the one at berth 3 attacks [`QUARRY`].
    #[derive(Debug)]
    struct TwoAttack;

    impl Behaviour for TwoAttack {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            if npc.berth == 3 {
                Goal::Formation { guard: None }
            } else {
                Goal::Attack(QUARRY)
            }
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            (npc.berth != 3).then_some(QUARRY)
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
        // The ticks take the attacks and the targets, the pass names berth
        // 1 for berth 2 alone, and each tick forms by the last's before its
        // AI runs, as `_DoPlayGameWork` marks the parents before
        // `_AIDispatch`.
        for _ in 0..3 {
            session.tick_traffic(&catalog, &TwoAttack, &mut NeverFires);
        }
        let named: Vec<Option<u32>> = escorts(&session)
            .iter()
            .map(|npc| npc.swarm_leader)
            .collect();
        assert_eq!(named, [None, Some(1), None]);
        assert_eq!(slots(&session)[2], (3, 3));
    }
}
