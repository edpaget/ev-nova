//! [`CarriedAi`]: how an NPC carrier's fighters fly and fight, while
//! their carrier is in the system (see [`bay`](crate::bay)).
//!
//! The original drives them through its escort AI with the carrier as
//! their parent, and the carrier's orders (`_AIIssueEscortOrders`
//! @0x8986d): Attack or Defend while it fights, Return to Hangar in any
//! state but attacking. Here:
//!
//! - While its carrier attacks or snipes at a ship, a fighter attacks
//!   its own target while that ship is live in the system, else the
//!   carrier's.
//! - When the carrier does neither, it flies back and docks
//!   ([`Goal::Dock`]).
//! - With its carrier not in the system it idles; the session leaves
//!   such a fighter on its own AI type.
//! - It fires as any ship does at what it attacks ([`fire::trigger`]),
//!   targets that ship, and reacts to no hit.

use crate::ai::{Behaviour, Goal, Surroundings, fire};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::armament::Trigger;
use crate::traffic::npc::Npc;

/// An NPC carrier's fighters' AI (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CarriedAi;

impl Behaviour for CarriedAi {
    fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        let carrier = match npc.carrier.map(|carrier| carrier.ship) {
            Some(ShipRef::Npc(id)) => around.npc(id),
            _ => None,
        };
        let Some(carrier) = carrier else {
            return Goal::Idle;
        };
        match carrier.goal.attacking() {
            Some(quarry) => {
                Goal::Attack(npc.target.filter(|&own| around.live(own)).unwrap_or(quarry))
            }
            None => Goal::Dock(ShipRef::Npc(carrier.id)),
        }
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        fire::trigger(npc, around)
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        npc.goal.attacking()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, TRADERS, n, player, ship};
    use crate::bay::Carrier;
    use crate::catalog::{StellarId, WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::weapon::WeaponSpec;
    use crate::testkit::{Draws, weapon};
    use crate::traffic::npc::{AiType, NpcId};

    /// The carrier, NPC 1, a pirate warship at the centre with `goal`.
    fn carrier(goal: Goal) -> Npc {
        let mut npc = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        npc.goal = goal;
        npc
    }

    /// NPC 2, its fighter, an interceptor 50 to the right carrying a gun,
    /// with `goal` and `target`.
    fn fighter(goal: Goal, target: Option<ShipRef>) -> Npc {
        let mut npc = ship(2, PIRATES, AiType::Interceptor, 50.0, 0.0);
        npc.carrier = Some(Carrier {
            ship: n(1),
            window: 100.0,
            reach: 40.0,
        });
        npc.leader = Some(NpcId(1));
        npc.goal = goal;
        npc.target = target;
        npc.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                speed: 1000,
                count: 30,
                energy_dmg: 5,
                ..weapon(141)
            }),
            1,
        )]);
        npc
    }

    /// A trader, NPC 3, 100 above.
    fn trader() -> Npc {
        ship(3, TRADERS, AiType::WimpyTrader, 0.0, -100.0)
    }

    /// What the fighter, `npcs[1]`, decides among `npcs`, the player 200
    /// below.
    fn decide(npcs: &[Npc]) -> Goal {
        let around = Surroundings {
            player: Some(player(0.0, 200.0)),
            ..Surroundings::new(&[], npcs)
        };
        let mut chance = Draws::of(&[]);
        let goal = CarriedAi.decide(&npcs[1], &around, &mut chance);
        assert_eq!(chance.asked, [0_u32; 0], "no draws");
        goal
    }

    #[test]
    fn with_its_carrier_attacking_a_fighter_with_no_target_attacks_the_carriers() {
        for goal in [Goal::Attack(ShipRef::Player), Goal::Snipe(ShipRef::Player)] {
            let npcs = [carrier(goal), fighter(Goal::Idle, None)];
            assert_eq!(decide(&npcs), Goal::Attack(ShipRef::Player), "{goal:?}");
        }
        let npcs = [
            carrier(Goal::Attack(n(3))),
            fighter(Goal::Idle, Some(ShipRef::Player)),
            trader(),
        ];
        assert_eq!(
            decide(&npcs),
            Goal::Attack(ShipRef::Player),
            "its own target, while it is live"
        );
        let npcs = [
            carrier(Goal::Attack(n(3))),
            fighter(Goal::Attack(n(9)), Some(n(9))),
            trader(),
        ];
        assert_eq!(decide(&npcs), Goal::Attack(n(3)), "its own is gone");
        let mut wreck = trader();
        wreck.id = NpcId(4);
        wreck.condition = crate::combat::hull::Condition::Dying { ticks_left: 2 };
        let npcs = [
            carrier(Goal::Attack(n(3))),
            fighter(Goal::Attack(n(4)), Some(n(4))),
            trader(),
            wreck,
        ];
        assert_eq!(decide(&npcs), Goal::Attack(n(3)), "its own is breaking up");
    }

    #[test]
    fn with_its_carrier_not_attacking_a_fighter_docks() {
        for goal in [
            Goal::Idle,
            Goal::Flee(ShipRef::Player),
            Goal::Land(StellarId(128)),
            Goal::JumpOut,
        ] {
            let npcs = [
                carrier(goal),
                fighter(Goal::Attack(ShipRef::Player), Some(ShipRef::Player)),
            ];
            assert_eq!(decide(&npcs), Goal::Dock(n(1)), "{goal:?}");
        }
    }

    #[test]
    fn a_fighter_whose_carrier_is_not_there_idles() {
        let npcs = [trader(), fighter(Goal::Attack(ShipRef::Player), None)];
        assert_eq!(decide(&npcs), Goal::Idle);
        let mut lost = fighter(Goal::Idle, None);
        lost.carrier = None;
        let npcs = [carrier(Goal::Attack(ShipRef::Player)), lost];
        assert_eq!(decide(&npcs), Goal::Idle, "no carrier at all");
    }

    #[test]
    fn a_fighter_fires_as_any_ship_does_at_what_it_attacks_and_reacts_to_nothing() {
        let attacking = fighter(Goal::Attack(ShipRef::Player), None);
        let docking = fighter(Goal::Dock(n(1)), Some(ShipRef::Player));
        let npcs = [carrier(Goal::Idle), attacking.clone(), docking.clone()];
        let around = Surroundings {
            player: Some(player(50.0, -100.0)),
            ..Surroundings::new(&[], &npcs)
        };
        let fired = CarriedAi.trigger(&attacking, &around);
        assert_eq!(fired, fire::trigger(&attacking, &around));
        assert_eq!(fired.only, Some(WeaponId(141)), "the gun ahead");
        assert_eq!(CarriedAi.target(&attacking, &around), Some(ShipRef::Player));
        assert_eq!(CarriedAi.trigger(&docking, &around), Trigger::default());
        assert_eq!(CarriedAi.target(&docking, &around), None);
        let strike = crate::combat::Strike {
            ship: n(2),
            by: ShipRef::Player,
            damage: 10.0,
            downed: None,
        };
        assert_eq!(
            CarriedAi.react(&attacking, &strike, &around),
            crate::ai::Reaction::default()
        );
    }
}
