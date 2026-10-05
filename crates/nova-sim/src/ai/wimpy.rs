//! Nova's wimpy trader (AI type 1, `_WimpyTraderAI` in the `EV Nova`
//! executable): it goes about its business ([`idle`](super::idle)), and
//! once provoked it flees from its attacker (@0x8b1ac), never fighting
//! back by choice; with the attacker gone it decides again.

use crate::ai::{Behaviour, Goal, Reaction, Surroundings, fire, idle, provoked_by, react};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::traffic::npc::Npc;

/// Nova's wimpy trader (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WimpyTrader;

impl Behaviour for WimpyTrader {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        provoked_by(npc, around).map_or_else(|| idle(npc, around, chance), Goal::Flee)
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        fire::trigger(npc, around)
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        npc.goal.quarry()
    }

    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        react::answer(npc, strike, around)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, TRADERS, around, govts, n, ship, sites};
    use crate::catalog::{StellarId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::weapon::WeaponSpec;
    use crate::testkit::{Draws, weapon};
    use crate::traffic::npc::AiType;

    #[test]
    fn a_wimpy_trader_goes_about_its_business() {
        let govts = govts(0, 0);
        let sites = sites();
        let npcs = [ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0)];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            WimpyTrader.decide(&npcs[0], &around, &mut chance),
            Goal::Land(StellarId(128))
        );
        assert_eq!(WimpyTrader.target(&npcs[0], &around), None);
        assert_eq!(WimpyTrader.trigger(&npcs[0], &around), Trigger::default());
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
            WimpyTrader.decide(&npcs[0], &around, &mut chance),
            Goal::Flee(n(2))
        );
        let fleeing = Npc {
            goal: Goal::Flee(n(2)),
            ..npcs[0].clone()
        };
        assert_eq!(
            WimpyTrader.target(&fleeing, &around),
            Some(n(2)),
            "its attacker"
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
            WimpyTrader.decide(&npcs[0], &around, &mut chance),
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
            WimpyTrader.trigger(&npcs[0], &around).only,
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
            ship: n(1),
            by: n(2),
            damage: 7.0,
            downed: None,
        };
        assert_eq!(
            WimpyTrader.react(&npcs[0], &strike, &around),
            react::answer(&npcs[0], &strike, &around)
        );
        assert_eq!(WimpyTrader.react(&npcs[0], &strike, &around).provoked, 7.0);
    }
}
