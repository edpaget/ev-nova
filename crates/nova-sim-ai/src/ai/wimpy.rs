//! Nova's wimpy trader (AI type 1, `_WimpyTraderAI` in the `EV Nova`
//! executable): it goes about its business ([`idle`](super::idle)), and
//! once provoked it flees from its attacker (@0x8b1ac), never fighting
//! back by choice, calling for help on every think it flees
//! (`_AICallForHelp` @0x8b1f5; see
//! [`call_for_help`](super::call_for_help)); with the attacker gone it
//! decides again.

use crate::ai::call_for_help::when_fleeing;
use crate::ai::fire::FireRules;
use crate::ai::{
    Behaviour, Decision, Goal, Reaction, Surroundings, fire, idle, provoked_by, react,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;

/// Nova's wimpy trader (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WimpyTrader {
    /// Who comes to a ship's help ([`react::answer`]).
    pub piracy_police: RuleSource,
    /// The rulebook's entries its weapon pick follows.
    pub fire: FireRules,
}

impl Behaviour for WimpyTrader {
    /// Its [`decide`](Self::decide) goal, with a distress call fleeing
    /// ([`when_fleeing`]).
    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        let goal = self.decide(npc, around, chance);
        Decision {
            goal,
            comm_call: when_fleeing(goal),
            ..Decision::default()
        }
    }

    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        provoked_by(npc, around).map_or_else(|| idle(npc, around, chance), Goal::Flee)
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        fire::trigger(npc, around, self.fire)
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        npc.goal.quarry()
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
}
