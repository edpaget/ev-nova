//! [`CarriedAi`]: how an NPC carrier's fighters fly and fight, while
//! their carrier is in the system (see [`bay`](crate::bay)).
//!
//! The original drives them through its escort AI with the carrier as
//! their parent, and the carrier's orders (`_AIIssueEscortOrders`
//! @0x8986d): Attack or Defend while it fights, Return to Hangar in any
//! state but attacking. Here:
//!
//! - While its carrier attacks or snipes at a ship, or plunders one still
//!   intact (goal 0xd, which `_AIIssueEscortOrders` orders as goal 4
//!   until the target is disabled, @0x89968-0x8997e), a fighter attacks
//!   its own target while that ship is live in the system, else the
//!   carrier's.
//! - When the carrier does neither, it flies back and docks
//!   ([`Goal::Dock`]).
//! - With its carrier not in the system it idles; the session leaves
//!   such a fighter on its own AI type.
//! - It fires as any ship does at what it attacks ([`fire::trigger`]),
//!   and targets that ship.
//! - A hit takes its attacker as its target, as any ship does
//!   ([`react::victim`]); it never comes to another ship's help.

use crate::ai::fire::FireRules;
use crate::ai::{Behaviour, GateWait, Goal, Reaction, Surroundings, escort, fire, react};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::traffic::npc::Npc;

/// An NPC carrier's fighters' AI (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CarriedAi {
    /// The rulebook's entries its weapon pick follows.
    pub fire: FireRules,
}

impl Behaviour for CarriedAi {
    fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        let carrier = match npc.carrier.map(|carrier| carrier.ship) {
            Some(ShipRef::Npc(id)) => around.npc(id),
            _ => None,
        };
        let Some(carrier) = carrier else {
            return Goal::Idle;
        };
        let plundered = carrier.goal.plundering().filter(|&ship| {
            !matches!(
                around.condition_of(ship),
                Some(Condition::Disabled | Condition::Dying { .. })
            )
        });
        match carrier.goal.attacking().or(plundered) {
            Some(quarry) => {
                Goal::Attack(npc.target.filter(|&own| around.live(own)).unwrap_or(quarry))
            }
            None => Goal::Dock(ShipRef::Npc(carrier.id)),
        }
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        fire::trigger(npc, around, self.fire)
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        npc.goal.attacking()
    }

    /// The escorts' [`fire_unprovoked`](escort::fire_unprovoked): the
    /// original flies a carried fighter as an escort (AI type above 4).
    fn gate_wait(&self, npc: &Npc, around: &Surroundings) -> GateWait {
        escort::fire_unprovoked(npc, around, self.fire)
    }

    /// [`react::victim`], as for the player's escorts: `_DamageShip`'s
    /// reaction block reads no AI type, and `_DoGoodSamaritan` takes AI 3
    /// and 4 alone (@0x8289f-0x828bb).
    ///
    /// Its next think attacks that target while its carrier attacks, as
    /// [`CarriedAi::decide`] reads its own target first, and docks
    /// otherwise. Keeping the hit's attacker by the carrier's orders, as
    /// `_EscortAI` does, is
    /// rdm:phase/escort-fleet-fidelity/phase-9-escorts-answer-hits-and-run-dry's.
    /// Until then a docking fighter's target can hold for as little as
    /// one tick, until that think. Nothing the player sees reads it in
    /// that time.
    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        react::victim(npc, strike, around)
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
    use crate::traffic::npc::{AiType, NpcId};
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::weapon;

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
        let goal = CarriedAi::default().decide(&npcs[1], &around, &mut chance);
        assert_eq!(chance.asked, [0_u32; 0], "no draws");
        goal
    }

    #[test]
    fn with_its_carrier_plundering_an_intact_ship_a_fighter_attacks_it_and_a_disabled_one_docks() {
        // `_AIIssueEscortOrders` orders a plundering leader's escorts as an
        // attacking one's, but to return (order 3) once its target is
        // disabled (@0x89968-0x8997e).
        let plunder = Goal::Plunder {
            ship: n(3),
            hopeless: true,
        };
        let npcs = [carrier(plunder), fighter(Goal::Idle, None), trader()];
        assert_eq!(decide(&npcs), Goal::Attack(n(3)));
        let mut wreck = trader();
        wreck.condition = crate::combat::hull::Condition::Disabled;
        let npcs = [carrier(plunder), fighter(Goal::Idle, None), wreck.clone()];
        assert_eq!(decide(&npcs), Goal::Dock(n(1)));
        wreck.condition = crate::combat::hull::Condition::Dying {
            ticks_left: 2,
            starting: false,
        };
        let npcs = [carrier(plunder), fighter(Goal::Idle, None), wreck];
        assert_eq!(decide(&npcs), Goal::Dock(n(1)), "`_IsDisabled` holds");
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
        wreck.condition = crate::combat::hull::Condition::Dying {
            ticks_left: 2,
            starting: false,
        };
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
    fn a_fighter_waiting_in_its_gate_runs_the_escorts_point_defence() {
        // AI type above 4 (@0x80e59): `_AIEscortFireUnprovoked` runs.
        let mut waiting = fighter(Goal::Idle, Some(n(3)));
        waiting.mode = crate::traffic::npc::Mode::Emerging { ticks_left: 30 };
        let mut down = trader();
        down.condition = crate::combat::hull::Condition::Disabled;
        for npcs in [
            [carrier(Goal::Idle), waiting.clone(), trader()],
            [carrier(Goal::Idle), waiting.clone(), down],
        ] {
            let around = Surroundings::new(&[], &npcs);
            assert_eq!(
                CarriedAi::default().gate_wait(&waiting, &around),
                crate::ai::escort::fire_unprovoked(&waiting, &around, FireRules::default())
            );
        }
        let npcs = [carrier(Goal::Idle), waiting.clone(), trader()];
        let around = Surroundings::new(&[], &npcs);
        assert!(
            CarriedAi::default()
                .gate_wait(&waiting, &around)
                .trigger
                .point_defence
        );
    }

    #[test]
    fn a_fighter_fires_as_any_ship_does_at_what_it_attacks() {
        let attacking = fighter(Goal::Attack(ShipRef::Player), None);
        let docking = fighter(Goal::Dock(n(1)), Some(ShipRef::Player));
        let npcs = [carrier(Goal::Idle), attacking.clone(), docking.clone()];
        let around = Surroundings {
            player: Some(player(50.0, -100.0)),
            ..Surroundings::new(&[], &npcs)
        };
        let fired = CarriedAi::default().trigger(&attacking, &around);
        assert_eq!(
            fired,
            fire::trigger(&attacking, &around, FireRules::default())
        );
        assert_eq!(fired.only, Some(WeaponId(141)), "the gun ahead");
        assert_eq!(
            CarriedAi::default().target(&attacking, &around),
            Some(ShipRef::Player)
        );
        assert_eq!(
            CarriedAi::default().trigger(&docking, &around),
            Trigger::default()
        );
        assert_eq!(CarriedAi::default().target(&docking, &around), None);
    }

    #[test]
    fn a_fighter_the_player_hits_turns_on_the_player_and_one_an_npc_hits_takes_it() {
        // `_DamageShip`'s reaction block reads no AI type (@0x3bb7a-0x3bed2),
        // and `_AIMakeShipAttackPlayer` sets the victim's goal 4 (@0x89d3e);
        // `_DoGoodSamaritan` never takes AI 5 or 6 (@0x8289f-0x828bb).
        let attacking = fighter(Goal::Attack(n(3)), Some(n(3)));
        let npcs = [carrier(Goal::Attack(n(3))), attacking.clone(), trader()];
        let around = Surroundings {
            player: Some(player(50.0, -100.0)),
            ..Surroundings::new(&[], &npcs)
        };
        let react = |strike| CarriedAi::default().react(&attacking, &strike, &around);
        let by_player = crate::combat::Strike {
            damage: 10.0,
            ..crate::combat::fixture::strike(n(2), ShipRef::Player)
        };
        assert_eq!(
            react(by_player),
            crate::ai::Reaction {
                target: Some(ShipRef::Player),
                provoked: 10.0,
                goal: Some(Goal::Attack(ShipRef::Player)),
            }
        );
        assert_eq!(
            react(crate::combat::fixture::strike(n(2), n(3))),
            crate::ai::Reaction {
                target: Some(n(3)),
                provoked: 12.0,
                goal: None,
            }
        );
        assert_eq!(
            react(crate::combat::fixture::strike(n(3), ShipRef::Player)),
            crate::ai::Reaction::default(),
            "no Good Samaritan"
        );
    }
}
