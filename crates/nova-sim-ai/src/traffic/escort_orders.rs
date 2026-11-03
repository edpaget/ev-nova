//! The traffic's pass of fleet leaders' orders: each NPC that leads
//! another issues, on its frame, the order words of
//! [`ai::escort_orders`](crate::ai::escort_orders) to every NPC it leads,
//! by that ship's class
//! ([`Npc::order_word`](super::npc::Npc::order_word)), whatever the led
//! ship's condition or mode, as `_AIIssueEscortOrders` tests neither
//! (@0x89b10-0x89b84). Each order sets the ship's mark too
//! ([`Npc::ordered_by_lead`](super::npc::Npc::ordered_by_lead), +0xc4,
//! @0x89b71). Each leader issues at the head of its own slot's run, before
//! its decision ([`dispatch_head`](super::dispatch_head)).
//!
//! **Ordering.** The original issues at each leader's own `_AIDispatch`,
//! in slot order, so a ship in a lower slot than its leader thinks on the
//! old word that frame and flies the new one the next, and so does the
//! engine. A fleeing leader's leg is read from its state as its slot
//! comes, as the original reads its last high level's substate.

use super::{Traffic, World};
use crate::ai::Surroundings;
use crate::ai::candidates::has_escorts;
use crate::ai::escort_orders::{for_class, issues, issues_on, orders};

impl Traffic {
    /// Each leader `picked` picks (one flag an NPC) that is due on
    /// `world`'s frame issues its escorts' orders (see the module docs). A
    /// leader reads a gone target by its slot's record while its target
    /// still names the slot ([`Surroundings::held_gone`]); a record stands
    /// until a ship takes its berth, so the leaders only read the records
    /// and leave them for the tick's thinks.
    ///
    /// Every due leader's table is worked out, then written: the leaders
    /// read the NPCs while the writes need them mutably. No table reads an
    /// order word, so the order of the two makes no difference.
    pub(crate) fn issue_escort_orders(&mut self, world: &World, picked: &[bool]) {
        let around = Surroundings {
            gone: self.gone.ships(),
            ..world.around(&self.npcs)
        };
        let tables: Vec<_> = self
            .npcs
            .iter()
            .zip(picked)
            .filter(|&(_, &picked)| picked)
            .map(|(leader, _)| leader)
            .filter(|leader| {
                issues_on(world.frame, leader.berth)
                    && issues(leader)
                    && has_escorts(leader, &around)
            })
            .map(|leader| (leader.id, orders(leader, &around, world.escort_order_reach)))
            .collect();
        for (id, table) in tables {
            for npc in self
                .npcs
                .iter_mut()
                .filter(|npc| npc.leader == Some(id) && !npc.escorts_player())
            {
                npc.order_word = for_class(table, npc.class);
                npc.ordered_by_lead = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{Behaviour, Decision, FleetEscortAi, Goal};
    use crate::bay::Carrier;
    use crate::catalog::EscortClass;
    use crate::chance::Chance;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::fleet::OrderWord;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{AiType, Npc, NpcId};
    use nova_sim_base::fixture::{Draws, FAST};

    /// NPC 0, at berth 1: a warship leader at the centre attacking NPC 9,
    /// its shield full; its odds stale once placed, so it orders 2, 2, 0,
    /// 0.
    fn leader() -> Npc {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.ai_type = AiType::Warship;
        ship.goal = Goal::Attack(ShipRef::Npc(NpcId(9)));
        ship
    }

    /// A ship of `class` led by NPC 0, at (0, 100).
    fn led(class: EscortClass) -> Npc {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.ai_type = AiType::Warship;
        ship.class = class;
        ship.leader = Some(NpcId(0));
        ship.state.position = Vec2::new(0.0, 100.0);
        ship
    }

    /// Traffic deciding every tick holding `ships`, added in turn at
    /// berths 1, 2, ...
    fn traffic(ships: impl IntoIterator<Item = Npc>) -> Traffic {
        let mut traffic = Traffic::with_interval(1);
        for ship in ships {
            traffic.add_npc(ship);
        }
        traffic
    }

    /// The raw order words of `traffic`'s NPCs, in order.
    fn words(traffic: &Traffic) -> Vec<i16> {
        traffic.npcs().iter().map(|npc| npc.order_word.0).collect()
    }

    /// `traffic` issuing on `frame`, every NPC picked.
    fn issue(traffic: &mut Traffic, frame: i16) {
        let picked = vec![true; traffic.npcs().len()];
        let world = World {
            frame,
            ..World::new(&[])
        };
        traffic.issue_escort_orders(&world, &picked);
    }

    #[test]
    fn a_leader_not_picked_issues_nothing() {
        let mut traffic = traffic([leader(), led(EscortClass::Fighter)]);
        traffic.issue_escort_orders(&World::new(&[]), &[false, true]);
        assert_eq!(words(&traffic), [-1, -1]);
    }

    #[test]
    fn on_its_frame_a_leader_writes_each_ship_it_leads_its_classs_word() {
        let mut down = led(EscortClass::Fighter);
        down.condition = Condition::Disabled;
        let mut traffic = traffic([
            leader(),
            led(EscortClass::Fighter),
            led(EscortClass::Medium),
            led(EscortClass::Warship),
            led(EscortClass::Freighter),
            down,
        ]);
        let mut waiting = traffic.npcs[4].clone();
        waiting.mode = crate::traffic::npc::Mode::Emerging { ticks_left: 3.0 };
        traffic.npcs[4] = waiting;
        assert_eq!(words(&traffic), [-1; 6], "every spawn writes -1");
        issue(&mut traffic, 0);
        assert_eq!(words(&traffic), [-1, 2, 2, 0, 0, 2]);
    }

    #[test]
    fn off_its_frame_a_leader_writes_nothing() {
        let mut traffic = traffic([leader(), led(EscortClass::Fighter)]);
        for frame in [1, 7, -3] {
            issue(&mut traffic, frame);
            assert_eq!(words(&traffic), [-1, -1], "{frame}");
        }
        // A leader in berth 8 issues a frame later.
        traffic.npcs[0].berth = 8;
        issue(&mut traffic, 0);
        assert_eq!(words(&traffic), [-1, -1]);
        issue(&mut traffic, 1);
        assert_eq!(words(&traffic), [-1, 2]);
    }

    #[test]
    fn a_ship_another_leads_or_none_is_untouched() {
        let mut other = led(EscortClass::Fighter);
        other.leader = Some(NpcId(5));
        let mut alone = led(EscortClass::Fighter);
        alone.leader = None;
        let mut traffic = traffic([leader(), other, alone]);
        issue(&mut traffic, 0);
        assert_eq!(words(&traffic), [-1, -1, -1]);
    }

    #[test]
    fn an_order_issued_sets_the_ships_mark_and_a_ship_no_order_reached_stays_clear() {
        // @0x89b71, beside the word, in the same loop.
        let mut other = led(EscortClass::Fighter);
        other.leader = Some(NpcId(5));
        let mut players = led(EscortClass::Fighter);
        players.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        let mut traffic = traffic([leader(), led(EscortClass::Freighter), other, players]);
        let marks = |traffic: &Traffic| -> Vec<bool> {
            traffic
                .npcs()
                .iter()
                .map(|npc| npc.ordered_by_lead)
                .collect()
        };
        assert_eq!(marks(&traffic), [false; 4], "no spawn sets it");
        issue(&mut traffic, 1);
        assert_eq!(marks(&traffic), [false; 4], "off its frame");
        issue(&mut traffic, 0);
        assert_eq!(marks(&traffic), [false, true, false, false]);
    }

    #[test]
    fn a_leader_gliding_in_issues_nothing() {
        let mut gliding = leader();
        gliding.mode = crate::traffic::npc::Mode::JumpingIn { tick: 0 };
        let mut traffic = traffic([gliding, led(EscortClass::Fighter)]);
        issue(&mut traffic, 0);
        assert_eq!(words(&traffic), [-1, -1]);
    }

    #[test]
    fn the_players_escort_carrier_writes_its_fighters_words() {
        let mut carrier = leader();
        carrier.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        let mut fighter = led(EscortClass::Fighter);
        fighter.carrier = Some(Carrier {
            ship: ShipRef::Npc(NpcId(0)),
            window: 100.0,
            reach: 40.0,
        });
        let mut traffic = traffic([carrier, fighter]);
        issue(&mut traffic, 0);
        assert_eq!(words(&traffic), [-1, 2]);
    }

    #[test]
    fn a_leader_reads_a_target_gone_since_the_last_think_closed_by_its_record() {
        // Its shield low, a disabled target sends fighters in (2).
        let mut low = leader();
        low.reserves.shield.now = 0.0;
        low.target = Some(ShipRef::Npc(NpcId(2)));
        let mut wreck = led(EscortClass::Medium);
        wreck.leader = None;
        wreck.condition = Condition::Disabled;
        let mut traffic = traffic([low, led(EscortClass::Fighter), wreck]);
        // Freed in the ship pass after the leader's slot: still read.
        traffic.remove_destroyed(NpcId(2));
        issue(&mut traffic, 0);
        assert_eq!(words(&traffic), [-1, 2]);
        assert_eq!(
            traffic.gone.ships().len(),
            1,
            "left for the tick's thinks to read"
        );
    }

    /// The leader keeps its goal; a ship it leads flies
    /// [`FleetEscortAi`].
    #[derive(Debug)]
    struct Fleet;

    impl Behaviour for Fleet {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.decision(npc, around, chance).goal
        }

        fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
            if npc.leader.is_some() {
                FleetEscortAi::default().decision(npc, around, chance)
            } else {
                Decision {
                    goal: npc.goal,
                    ..Decision::default()
                }
            }
        }
    }

    #[test]
    fn a_fighter_of_the_players_escort_takes_the_word_its_think_writes() {
        // `_EscortAI` @0x83b76, @0x83b7f, off its carrier's frame.
        let mut carrier = leader();
        carrier.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: Some(crate::escort::EscortOrder::Defend),
        });
        carrier.order_word = OrderWord(1);
        let mut fighter = led(EscortClass::Fighter);
        fighter.carrier = Some(Carrier {
            ship: ShipRef::Npc(NpcId(0)),
            window: 100.0,
            reach: 40.0,
        });
        let mut traffic = traffic([carrier, fighter]);
        let world = World {
            frame: 1,
            ..World::new(&[])
        };
        traffic.tick_in(&Fleet, world, &[], &mut Draws::of(&[0]));
        assert_eq!(traffic.npcs[1].order_word, OrderWord(1));
        // A decision that writes none leaves the word.
        traffic.npcs[1].order_word = OrderWord(4);
        traffic.npcs[1].leader = None;
        traffic.tick_in(&Fleet, world, &[], &mut Draws::of(&[0]));
        assert_eq!(traffic.npcs[1].order_word, OrderWord(4));
    }

    #[test]
    fn a_tick_on_the_leaders_frame_flies_its_escort_by_the_new_order_in_the_same_tick() {
        // The leader attacks NPC 2, which threatens it: Attack (2) for a
        // medium escort, which takes the threat.
        let mut lead = leader();
        lead.goal = Goal::Attack(ShipRef::Npc(NpcId(2)));
        let mut threat = led(EscortClass::Medium);
        threat.leader = None;
        threat.state.position = Vec2::new(0.0, 3000.0);
        threat.goal = Goal::Attack(ShipRef::Npc(NpcId(0)));
        let mut traffic = traffic([lead, led(EscortClass::Medium), threat]);
        let world = World {
            frame: 1,
            ..World::new(&[])
        };
        traffic.tick_in(&Fleet, world, &[], &mut Draws::of(&[0]));
        assert_eq!(traffic.npcs[1].order_word, OrderWord::NONE);
        assert!(
            matches!(traffic.npcs[1].goal, Goal::Follow { .. }),
            "off its frame it forms: {:?}",
            traffic.npcs[1].goal
        );
        let world = World {
            frame: 8,
            ..World::new(&[])
        };
        traffic.tick_in(&Fleet, world, &[], &mut Draws::of(&[0]));
        assert_eq!(traffic.npcs[1].order_word, OrderWord(2));
        assert_eq!(traffic.npcs[1].goal, Goal::Attack(ShipRef::Npc(NpcId(2))));
    }
}
