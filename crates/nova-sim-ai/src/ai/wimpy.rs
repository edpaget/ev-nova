//! Nova's wimpy trader (AI type 1, `_WimpyTraderAI` in the `EV Nova`
//! executable): it goes about its business ([`idle`](super::idle)),
//! stopping to wait ([`Goal::Wait`], state 6) where it can't leave until
//! provoked, and once provoked it flees from its attacker (@0x8b1ac), never fighting
//! back by choice, calling for help on every think it flees
//! (`_AICallForHelp` @0x8b1f5; see
//! [`call_for_help`](super::call_for_help)); with the attacker gone it
//! decides again. In its count wait (goal 0x16) it returns at once
//! (@0x8b062), provoked or not. Unprovoked, it keeps any goal it holds
//! but 0, as its idle block runs on goal 0 alone (@0x8b06c): a plunder, a
//! fight on a ship still here, a mine, a scoop or a stellar attack, a lost
//! lead's that the succession copied included ([`trader_keep`]). Goal 4's
//! high level then flies a kept attack in the same frame: one on a ship
//! breaking up ends, and a hopeless chase is fled
//! ([`trader_keep::high_level`]).

use crate::ai::call_for_help::when_fleeing;
use crate::ai::fire::FireRules;
use crate::ai::{
    Behaviour, Decision, Goal, Reaction, Surroundings, fire, idle_block, lost_sight, provoked_by,
    react, trader_keep,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;
use crate::traffic::provocation::attack_breaking_up;

/// Nova's wimpy trader (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WimpyTrader {
    /// Who comes to a ship's help ([`react::answer`]).
    pub piracy_police: RuleSource,
    /// The rulebook's entries its weapon pick follows.
    pub fire: FireRules,
}

/// What `_WimpyTraderAI` writes, before the high level: its count wait
/// kept (@0x8b062), a flight from its attacker provoked (@0x8b1ac), and
/// otherwise a goal it keeps or its idle block's
/// ([`trader_keep::unprovoked`]).
fn think(npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
    if npc.goal == Goal::CountWait {
        return npc.goal;
    }
    provoked_by(npc, around)
        .map_or_else(|| trader_keep::unprovoked(npc, around, chance), Goal::Flee)
}

impl Behaviour for WimpyTrader {
    /// Its think's goal ([`think`]) as goal 4's high level takes it
    /// ([`trader_keep::high_level`]), an attack on a ship breaking up
    /// ended with its target and provocation ([`trader_keep::ends_attack`],
    /// [`attack_breaking_up`]), with a distress call when the think flees
    /// ([`when_fleeing`]), and its idle block's pick written to its travel
    /// stellar ([`idle_block::pick_write`], @0x8b0d1, @0x8b0ef).
    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        let thought = think(npc, around, chance);
        let ended = trader_keep::ends_attack(thought, around);
        Decision {
            goal: if ended {
                Goal::Idle
            } else {
                trader_keep::high_level(npc, thought, around)
            },
            comm_call: when_fleeing(thought),
            clears_target: ended,
            hopeless_flee: trader_keep::hopeless_flee(npc, thought, around).filter(|_| !ended),
            forgets_provocation: attack_breaking_up(npc, thought, around),
            travel: idle_block::pick_write(npc, around, thought),
            ..Decision::default()
        }
    }

    /// Its [`decision`](Self::decision)'s goal.
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    /// Its weapon pick, but nothing while it docks or drifts
    /// ([`fire::quiet`]).
    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        if fire::quiet(npc) {
            return Trigger::default();
        }
        fire::trigger(npc, around, self.fire)
    }

    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        lost_sight::trader_target(npc, around)
    }

    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        react::answer(npc, strike, around, self.piracy_police)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::CommCall;
    use crate::ai::fixture::{PIRATES, TRADERS, around, govts, n, ship, sites};
    use crate::catalog::{StellarId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::weapon::WeaponSpec;
    use crate::traffic::npc::AiType;
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::weapon;

    #[test]
    fn a_wimpy_trader_goes_about_its_business() {
        let govts = govts(0, 0);
        let sites = sites();
        let npcs = [ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0)];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            WimpyTrader::default().decide(&npcs[0], &around, &mut chance),
            Goal::Land(StellarId(128))
        );
        assert_eq!(WimpyTrader::default().target(&npcs[0], &around), None);
        assert_eq!(
            WimpyTrader::default().trigger(&npcs[0], &around),
            Trigger::default()
        );
    }

    #[test]
    fn a_visited_wimpy_trader_leaves_waits_when_it_cannot_jump_and_flees_provoked() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.visited = Some(StellarId(128));
        let decided = |trader: &Npc, npcs: &[Npc]| {
            let around = around(&sites, npcs, &govts, (0.0, -100.0), 0);
            WimpyTrader::default().decide(trader, &around, &mut Draws::of(&[]))
        };
        assert_eq!(decided(&trader, &[trader.clone()]), Goal::JumpOut);
        trader.class_fuel = 99;
        assert_eq!(decided(&trader, &[trader.clone()]), Goal::Wait);
        trader.goal = Goal::Wait;
        assert_eq!(
            decided(&trader, &[trader.clone()]),
            Goal::Wait,
            "state 6 is sticky"
        );
        trader.provoked = 10.0;
        trader.target = Some(n(2));
        let npcs = [trader.clone(), ship(2, PIRATES, AiType::Warship, 50.0, 0.0)];
        assert_eq!(decided(&trader, &npcs), Goal::Flee(n(2)), "@0x8b1ac");
    }

    #[test]
    fn its_decision_clears_the_travel_stellar_when_its_pick_fails() {
        // @0x8b0d1, @0x8b0ef: the pick's -1.
        let govts = govts(0, 0);
        let unlandable: Vec<_> = sites()
            .into_iter()
            .map(|site| crate::catalog::LandingSite { flags: 0, ..site })
            .collect();
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.travel = Some(StellarId(150));
        let npcs = [trader];
        let around = around(&unlandable, &npcs, &govts, (0.0, -100.0), 0);
        let decision = WimpyTrader::default().decision(&npcs[0], &around, &mut Draws::of(&[]));
        assert_eq!(decision.goal, Goal::JumpOut);
        assert_eq!(decision.travel, crate::ai::travel::TravelWrite::Clear);
    }

    #[test]
    fn a_provoked_wimpy_trader_flees_from_its_attacker_and_never_attacks() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.provoked = 10.0;
        trader.target = Some(n(2));
        let npcs = [trader, ship(2, PIRATES, AiType::Warship, 50.0, 0.0)];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        let mut chance = Draws::of(&[]);
        assert_eq!(
            WimpyTrader::default().decide(&npcs[0], &around, &mut chance),
            Goal::Flee(n(2))
        );
        let fleeing = Npc {
            goal: Goal::Flee(n(2)),
            ..npcs[0].clone()
        };
        assert_eq!(
            WimpyTrader::default().target(&fleeing, &around),
            Some(n(2)),
            "its attacker"
        );
    }

    #[test]
    fn a_fleeing_wimpy_trader_calls_for_help_and_an_idle_one_does_not() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.provoked = 10.0;
        trader.target = Some(n(2));
        let npcs = [trader, ship(2, PIRATES, AiType::Warship, 50.0, 0.0)];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        let decision = WimpyTrader::default().decision(&npcs[0], &around, &mut Draws::of(&[]));
        assert_eq!(decision.goal, Goal::Flee(n(2)));
        assert_eq!(decision.comm_call, Some(CommCall::Distress));
        let calm = [ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0)];
        let around = crate::ai::fixture::around(&sites, &calm, &govts, (0.0, -100.0), 0);
        let decision = WimpyTrader::default().decision(&calm[0], &around, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, Goal::Land(StellarId(128)));
        assert_eq!(decision.comm_call, None);
    }

    #[test]
    fn an_unprovoked_wimpy_trader_keeps_an_inspection_and_a_provoked_one_flees() {
        // Any goal but 0 is kept unprovoked (@0x8b06c, @0x8b1ac), goal 7
        // included, its state-7 block type-free (`inspect_high`).
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.goal = Goal::Inspect(ShipRef::Player);
        let npcs = [trader.clone(), ship(2, PIRATES, AiType::Warship, 50.0, 0.0)];
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        let decision = WimpyTrader::default().decision(&npcs[0], &around, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, Goal::Inspect(ShipRef::Player));
        assert_eq!(decision.travel, crate::ai::travel::TravelWrite::Keep);
        trader.provoked = 10.0;
        trader.target = Some(n(2));
        let npcs = [trader, ship(2, PIRATES, AiType::Warship, 50.0, 0.0)];
        let around = crate::ai::fixture::around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        assert_eq!(
            WimpyTrader::default().decide(&npcs[0], &around, &mut Draws::of(&[0])),
            Goal::Flee(n(2))
        );
    }

    #[test]
    fn a_provoked_held_wimpy_trader_flees() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.goal = Goal::Hold;
        let calm = [trader.clone()];
        let around_calm = around(&sites, &calm, &govts, (0.0, -100.0), 0);
        assert_eq!(
            WimpyTrader::default().decide(&calm[0], &around_calm, &mut Draws::of(&[])),
            Goal::Hold
        );
        trader.provoked = 10.0;
        trader.target = Some(n(2));
        let npcs = [trader, ship(2, PIRATES, AiType::Warship, 50.0, 0.0)];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(
            WimpyTrader::default().decide(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Flee(n(2))
        );
    }

    #[test]
    fn once_its_attacker_is_gone_it_decides_again() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.provoked = 10.0;
        trader.target = Some(n(2));
        trader.goal = Goal::Flee(n(2));
        let npcs = [trader];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            WimpyTrader::default().decide(&npcs[0], &around, &mut chance),
            Goal::Land(StellarId(128))
        );
    }

    #[test]
    fn a_fleeing_wimpy_trader_fires_its_turrets_at_its_attacker() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.goal = Goal::Flee(n(2));
        trader.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                guidance: 4,
                speed: 1000,
                count: 20,
                ..weapon(130)
            }),
            1,
        )]);
        let npcs = [trader, ship(2, PIRATES, AiType::Warship, 0.0, 100.0)];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(
            WimpyTrader::default().trigger(&npcs[0], &around).only,
            Some(crate::catalog::WeaponId(130))
        );
    }

    #[test]
    fn a_wimpy_trader_docking_or_drifting_fires_nothing() {
        // Subs 9 and 0xb end in `_AIEscortFireUnprovoked`, which returns
        // for AI type 4 or below (@0x80e59-0x80e61); sub 0xf and the
        // drift's sub 0 fire nothing.
        use crate::traffic::plunder_dock::{DockSub, Docking};
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.goal = Goal::Flee(n(2));
        trader.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                guidance: 4,
                speed: 1000,
                count: 20,
                ..weapon(130)
            }),
            1,
        )]);
        trader.docking = Some(Docking {
            ship: n(2),
            sub: DockSub::Approach,
        });
        let npcs = [trader, ship(2, PIRATES, AiType::Warship, 0.0, 100.0)];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(
            WimpyTrader::default().trigger(&npcs[0], &around),
            Trigger::default(),
            "docking"
        );
        let mut drifting = npcs.clone();
        drifting[0].docking = None;
        drifting[0].goal = Goal::Drift;
        let around = crate::ai::fixture::around(&sites, &drifting, &govts, (0.0, -100.0), 0);
        assert_eq!(
            WimpyTrader::default().trigger(&drifting[0], &around),
            Trigger::default(),
            "drifting"
        );
    }

    #[test]
    fn a_wimpy_trader_answers_a_hit_as_nova_does() {
        let govts = govts(0, 0);
        let sites = sites();
        let npcs = [
            ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0),
            ship(2, PIRATES, AiType::Warship, 50.0, 0.0),
        ];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        let strike = Strike {
            nominal: crate::combat::Nominal {
                mass: 3.0,
                energy: 4.0,
            },
            ..crate::combat::fixture::strike(n(1), n(2))
        };
        assert_eq!(
            WimpyTrader::default().react(&npcs[0], &strike, &around),
            react::answer(&npcs[0], &strike, &around, RuleSource::Engine)
        );
        assert_eq!(
            WimpyTrader::default()
                .react(&npcs[0], &strike, &around)
                .provoked,
            7.0
        );
    }

    /// A wimpy trader (NPC 1) at the centre, unprovoked, holding `goal` on
    /// its quarry, and a pirate (NPC 2) 300 below it in `condition`.
    fn keeping(goal: Goal, condition: crate::combat::hull::Condition) -> [Npc; 2] {
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.goal = goal;
        trader.target = goal.quarry();
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.condition = condition;
        [trader, pirate]
    }

    /// The wimpy trader's decision among `npcs`, drawing 0.
    fn decision_among(npcs: &[Npc]) -> Decision {
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -1000.0), 0);
        WimpyTrader::default().decision(&npcs[0], &around, &mut Draws::of(&[0]))
    }

    #[test]
    fn an_unprovoked_wimpy_trader_keeps_a_plunder_of_a_disabled_or_intact_ship_or_one_gone() {
        // Its idle block runs on goal 0 alone (@0x8b06c), and unprovoked
        // the think writes nothing (@0x8b1ac-0x8b1fa): goal 0xd's high
        // level flies it.
        use crate::combat::hull::Condition;
        let plunder = Goal::Plunder {
            ship: n(2),
            hopeless: false,
        };
        for condition in [Condition::Disabled, Condition::Intact] {
            let decision = decision_among(&keeping(plunder, condition));
            assert_eq!(decision.goal, plunder, "{condition:?}");
            assert_eq!(decision.comm_call, None, "{condition:?}");
            assert_eq!(
                decision.travel,
                crate::ai::travel::TravelWrite::Keep,
                "{condition:?}: no idle block, so no +0x6c"
            );
        }
        let npcs = keeping(plunder, Condition::Intact);
        assert_eq!(decision_among(&npcs[..1]).goal, plunder, "gone");
    }

    #[test]
    fn an_unprovoked_wimpy_trader_keeps_a_count_wait_or_a_fight_on_a_ship_still_here() {
        use crate::combat::hull::Condition;
        for goal in [
            Goal::CountWait,
            Goal::Attack(n(2)),
            Goal::Snipe(n(2)),
            Goal::Flee(n(2)),
        ] {
            let goal_now = if goal == Goal::Snipe(n(2)) {
                // Goal 4's high level: a chase that isn't hopeless is
                // attacked (sub 0xe only while hopeless, @0x8e5f8).
                Goal::Attack(n(2))
            } else {
                goal
            };
            assert_eq!(
                decision_among(&keeping(goal, Condition::Intact)).goal,
                goal_now,
                "{goal:?}"
            );
        }
        let npcs = keeping(Goal::Attack(n(2)), Condition::Intact);
        assert_eq!(
            decision_among(&npcs[..1]).goal,
            Goal::Land(StellarId(128)),
            "a fight on a ship gone idles (@0x8e263 → @0x8e731)"
        );
    }

    #[test]
    fn a_wimpy_trader_keeping_a_flight_calls_for_help() {
        // `_AICallForHelp` reads the goal the think leaves (@0x8b1e1-0x8b1f5).
        let npcs = keeping(Goal::Flee(n(2)), crate::combat::hull::Condition::Intact);
        assert_eq!(decision_among(&npcs).comm_call, Some(CommCall::Distress));
    }

    #[test]
    fn a_provoked_wimpy_trader_still_flees_whatever_it_held_but_its_count_wait() {
        use crate::combat::hull::Condition;
        let plunder = Goal::Plunder {
            ship: ShipRef::Player,
            hopeless: false,
        };
        let mut npcs = keeping(plunder, Condition::Intact);
        npcs[0].provoked = 5.0;
        npcs[0].target = Some(n(2));
        assert_eq!(decision_among(&npcs).goal, Goal::Flee(n(2)), "@0x8b1ac");
        npcs[0].goal = Goal::CountWait;
        let decision = decision_among(&npcs);
        assert_eq!(decision.goal, Goal::CountWait, "@0x8b062: it returns first");
        assert_eq!(decision.comm_call, None);
    }

    #[test]
    fn a_wimpy_trader_fights_the_player_whose_hit_of_no_damage_turned_it() {
        // `react::victim` writes goal 4 on the player with no provocation;
        // `_WimpyTraderAI` keeps it unprovoked (@0x8b1ac → @0x8b1fa), and
        // goal 4's high level flies it.
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.goal = Goal::Attack(ShipRef::Player);
        trader.target = Some(ShipRef::Player);
        let npcs = [trader];
        let around = around(&sites, &npcs, &govts, (0.0, -300.0), 0);
        let decision = WimpyTrader::default().decision(&npcs[0], &around, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, Goal::Attack(ShipRef::Player));
        assert_eq!(decision.comm_call, None);
        assert_eq!(
            WimpyTrader::default().target(&npcs[0], &around),
            Some(ShipRef::Player)
        );
    }

    #[test]
    fn a_wimpy_traders_kept_attack_on_a_ship_breaking_up_ends_with_its_target_and_provocation() {
        // @0x8e3ef → @0x8e714-0x8e726: goal 0, target -1, +0x96 0.
        let dying = crate::combat::hull::Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let decision = decision_among(&keeping(Goal::Attack(n(2)), dying));
        assert_eq!(decision.goal, Goal::Idle);
        assert!(decision.clears_target);
        assert!(decision.forgets_provocation);
        assert_eq!(decision.travel, crate::ai::travel::TravelWrite::Keep);
    }

    #[test]
    fn a_wimpy_traders_kept_hopeless_attack_flees_and_calls_from_the_next_think() {
        // Goal 3 with sub 5 (@0x8e622-0x8e62e), written by the high level
        // after the think's call for help read goal 4.
        let mut npcs = keeping(Goal::Attack(n(2)), crate::combat::hull::Condition::Intact);
        npcs[0].hull.mass = 100.0;
        npcs[1].goal = Goal::Flee(n(1));
        npcs[1].state.velocity = crate::geometry::Vec2::new(0.0, 1.0);
        let decision = decision_among(&npcs);
        assert_eq!(decision.goal, Goal::Flee(n(2)));
        assert_eq!(
            decision.hopeless_flee,
            Some(crate::ai::hopeless_flee::HopelessFlee::Attack),
            "goal 4's high level wrote it"
        );
        assert_eq!(decision.comm_call, None, "the think saw goal 4");
        npcs[0].goal = decision.goal;
        assert_eq!(
            decision_among(&npcs).hopeless_flee,
            None,
            "a flight kept is no hopeless chase"
        );
        assert_eq!(
            decision_among(&npcs).comm_call,
            Some(CommCall::Distress),
            "the next think keeps the flight"
        );
    }

    #[test]
    fn a_trader_that_stopped_on_losing_sight_keeps_its_target() {
        // State 6 keeps +0x70 (@0x8d803-0x8d80f).
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.goal = Goal::Wait;
        trader.target = Some(ShipRef::Player);
        let npcs = [trader, ship(2, PIRATES, AiType::Warship, 50.0, 0.0)];
        let on_player = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(
            WimpyTrader::default().target(&npcs[0], &on_player),
            Some(ShipRef::Player)
        );
        let mut on_npc = npcs.clone();
        on_npc[0].target = Some(n(2));
        let seen = around(&sites, &on_npc, &govts, (0.0, -100.0), 0);
        assert_eq!(WimpyTrader::default().target(&on_npc[0], &seen), Some(n(2)));
        on_npc[0].goal = Goal::Idle;
        let idle = around(&sites, &on_npc, &govts, (0.0, -100.0), 0);
        assert_eq!(
            WimpyTrader::default().target(&on_npc[0], &idle),
            None,
            "any other goal targets its quarry"
        );
    }

    #[test]
    fn a_stopped_traders_kept_target_is_dropped_once_gone() {
        // `_HighLevelAIHandler` drops a target whose slot is inactive
        // (@0x8d4e4-0x8d50d); one breaking up is still active.
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.goal = Goal::Wait;
        trader.target = Some(n(2));
        let alone = [trader.clone()];
        let around_alone = around(&sites, &alone, &govts, (0.0, -100.0), 0);
        assert_eq!(
            WimpyTrader::default().target(&alone[0], &around_alone),
            None
        );
        let mut pirate = ship(2, PIRATES, AiType::Warship, 50.0, 0.0);
        pirate.condition = crate::combat::hull::Condition::Destroyed;
        let npcs = [trader.clone(), pirate.clone()];
        let around_gone = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(WimpyTrader::default().target(&npcs[0], &around_gone), None);
        pirate.condition = crate::combat::hull::Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let npcs = [trader, pirate];
        let around_dying = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(
            WimpyTrader::default().target(&npcs[0], &around_dying),
            Some(n(2)),
            "breaking up, still active"
        );
    }
}
