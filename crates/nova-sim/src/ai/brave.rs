//! Nova's brave trader (AI type 2, `_BraveTraderAI` in the `EV Nova`
//! executable): it goes about its business ([`idle`](super::idle)), and
//! once provoked it attacks its attacker while it is within
//! [`STAND_AND_FIGHT`] pixels on both axes (@0x8b616), and flees from it
//! beyond, or when the chase is hopeless; with the attacker gone it
//! decides again.

use crate::ai::odds::hopeless_chase;
use crate::ai::{Behaviour, Goal, Reaction, Surroundings, fire, idle, provoked_by, react};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::traffic::npc::Npc;

/// How near, in pixels on either axis, a brave trader's attacker must be
/// for it to fight back (`cmpw $0x4e2`).
pub const STAND_AND_FIGHT: f32 = 1250.0;

/// Nova's brave trader (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BraveTrader;

impl Behaviour for BraveTrader {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        let Some(attacker) = provoked_by(npc, around) else {
            return idle(npc, around, chance);
        };
        let near = around.state_of(attacker).is_some_and(|state| {
            let off = state.position - npc.state.position;
            off.x.abs() <= STAND_AND_FIGHT && off.y.abs() <= STAND_AND_FIGHT
        });
        if near && !hopeless_chase(npc, attacker, around) {
            Goal::Attack(attacker)
        } else {
            Goal::Flee(attacker)
        }
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
    use crate::catalog::{StellarId, WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::weapon::WeaponSpec;
    use crate::geometry::Vec2;
    use crate::testkit::{Draws, weapon};
    use crate::traffic::npc::AiType;

    /// A provoked brave trader at (200, 100) and its attacker (`x`, `y`)
    /// from it.
    fn provoked(x: f32, y: f32) -> [Npc; 2] {
        let mut trader = ship(1, TRADERS, AiType::BraveTrader, 200.0, 100.0);
        trader.provoked = 10.0;
        trader.target = Some(n(2));
        [
            trader,
            ship(2, PIRATES, AiType::Warship, 200.0 + x, 100.0 + y),
        ]
    }

    fn decided(npcs: &[Npc]) -> Goal {
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -5000.0), 0);
        BraveTrader.decide(&npcs[0], &around, &mut Draws::of(&[0]))
    }

    #[test]
    fn a_brave_trader_fights_back_within_1250_on_both_axes_and_flees_beyond() {
        assert_eq!(decided(&provoked(1250.0, -1250.0)), Goal::Attack(n(2)));
        assert_eq!(decided(&provoked(-1250.0, 1250.0)), Goal::Attack(n(2)));
        assert_eq!(decided(&provoked(1251.0, 0.0)), Goal::Flee(n(2)));
        assert_eq!(decided(&provoked(0.0, -1251.0)), Goal::Flee(n(2)));
        assert_eq!(STAND_AND_FIGHT, 1250.0);
    }

    #[test]
    fn a_brave_trader_flees_a_hopeless_chase() {
        let mut npcs = provoked(0.0, -300.0);
        npcs[0].hull.mass = 100.0;
        npcs[1].goal = Goal::Flee(n(1));
        npcs[1].state.velocity = Vec2::new(0.0, -1.0);
        assert_eq!(decided(&npcs), Goal::Flee(n(2)));
    }

    #[test]
    fn unprovoked_or_with_its_attacker_gone_it_goes_about_its_business() {
        let mut npcs = provoked(100.0, 0.0);
        npcs[0].provoked = 0.0;
        assert_eq!(decided(&npcs), Goal::Land(StellarId(128)));
        assert_eq!(
            decided(&provoked(100.0, 0.0)[..1]),
            Goal::Land(StellarId(128))
        );
    }

    #[test]
    fn a_brave_trader_targets_and_fires_at_whom_it_fights() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut npcs = provoked(0.0, -100.0);
        npcs[0].goal = Goal::Attack(n(2));
        npcs[0].armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                speed: 1000,
                count: 20,
                ..weapon(128)
            }),
            1,
        )]);
        let around = around(&sites, &npcs, &govts, (0.0, -5000.0), 0);
        assert_eq!(BraveTrader.target(&npcs[0], &around), Some(n(2)));
        assert_eq!(
            BraveTrader.trigger(&npcs[0], &around).only,
            Some(WeaponId(128))
        );
        let strike = Strike {
            ship: n(1),
            by: n(2),
            damage: 7.0,
            downed: None,
        };
        assert_eq!(
            BraveTrader.react(&npcs[0], &strike, &around).target,
            Some(n(2))
        );
    }
}
