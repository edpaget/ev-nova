//! Nova's brave trader (AI type 2, `_BraveTraderAI` in the `EV Nova`
//! executable): it goes about its business ([`idle`](super::idle)),
//! stopping to wait ([`Goal::Wait`], state 6) where it can't leave until
//! provoked, and once provoked it attacks its attacker while it is within
//! [`STAND_AND_FIGHT`] pixels on both axes (@0x8b616), and flees from it
//! beyond, or when the chase is hopeless; with the attacker gone it
//! decides again. Fleeing, or fighting a warship or an interceptor, it
//! calls for help on every think (`_AICallForHelp` @0x8b6cc, @0x8b71c;
//! see [`call_for_help`](super::call_for_help)), and fleeing it calls
//! for reinforcements too, after the call for help (@0x8b6dc; see
//! [`reinforcements`](super::reinforcements)). A stand-off ship (`shïp`
//! `Flags2` 0x0002) never asks
//! whether the chase is hopeless: `_HighLevelAIHandler` tests the bit
//! first and jumps past `_AIEvalHopelessChase` (@0x8e48e), keeping its
//! band instead (see [`stand_off`](super::stand_off)).

use crate::ai::call_for_help::brave_call;
use crate::ai::fire::FireRules;
use crate::ai::odds::hopeless_chase;
use crate::ai::{
    Behaviour, Decision, Goal, Reaction, Surroundings, fire, idle, lost_sight, provoked_by, react,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;
use crate::traffic::provocation::attack_breaking_up;

/// How near, in pixels on either axis, a brave trader's attacker must be
/// for it to fight back (`cmpw $0x4e2`).
pub const STAND_AND_FIGHT: f32 = 1250.0;

/// Nova's brave trader (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BraveTrader {
    /// Who comes to a ship's help ([`react::answer`]).
    pub piracy_police: RuleSource,
    /// The rulebook's entries its weapon pick follows.
    pub fire: FireRules,
}

/// What `_BraveTraderAI` writes, before the high level: provoked, it
/// attacks its attacker within [`STAND_AND_FIGHT`] on both axes
/// (`setle`, @0x8b616-0x8b68d) unless the chase is hopeless, and flees
/// it beyond; unprovoked, it goes about its business. An attacker
/// breaking up is still in its slot, so the think reads its place and
/// attacks it near, whatever the chase (the high level's `_IsDying`
/// test, @0x8e3ef, comes before the hopeless chase's, @0x8e48e), or
/// flees it far. One gone from the system is not read: the engine
/// keeps no place for a ship that has left.
fn think(npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
    let near_to = |attacker| {
        around.state_of(attacker).is_some_and(|state| {
            let off = state.position - npc.state.position;
            off.x.abs() <= STAND_AND_FIGHT && off.y.abs() <= STAND_AND_FIGHT
        })
    };
    if let Some(attacker) = provoked_by(npc, around) {
        let near = near_to(attacker);
        return if near && (npc.hull.stands_off || !hopeless_chase(npc, attacker, around)) {
            Goal::Attack(attacker)
        } else {
            Goal::Flee(attacker)
        };
    }
    let breaking_up = npc.target.filter(|&attacker| {
        npc.provoked > 0.0 && matches!(around.condition_of(attacker), Some(Condition::Dying { .. }))
    });
    match breaking_up {
        Some(attacker) if near_to(attacker) => Goal::Attack(attacker),
        Some(attacker) => Goal::Flee(attacker),
        None => idle(npc, around, chance),
    }
}

impl Behaviour for BraveTrader {
    /// Its think's goal ([`think`]), with a distress call
    /// fleeing or fighting a warship or interceptor ([`brave_call`]), and a
    /// call for reinforcements fleeing (state 3, @0x8b6dc). An attack on a
    /// ship breaking up ends with its provocation and its target
    /// ([`attack_breaking_up`]): `_BraveTraderAI` has no drop of its own,
    /// so the high level ends it (@0x8e3ef → @0x8e714-0x8e726: goal 0,
    /// target -1, +0x96 0), after the think's call.
    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        let thought = think(npc, around, chance);
        let ended = thought
            .attacking()
            .is_some_and(|ship| matches!(around.condition_of(ship), Some(Condition::Dying { .. })));
        Decision {
            goal: if ended { Goal::Idle } else { thought },
            comm_call: brave_call(thought, around),
            calls_reinforcements: matches!(thought, Goal::Flee(_)),
            forgets_provocation: attack_breaking_up(npc, thought, around),
            clears_target: ended,
            ..Decision::default()
        }
    }

    /// Its [`decision`](Self::decision)'s goal.
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
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
    use crate::catalog::{StellarId, WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::weapon::WeaponSpec;
    use crate::geometry::Vec2;
    use crate::traffic::npc::AiType;
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::weapon;

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
        BraveTrader::default().decide(&npcs[0], &around, &mut Draws::of(&[0]))
    }

    #[test]
    fn a_visited_brave_trader_leaves_waits_when_it_cannot_jump_and_fights_or_flees_provoked() {
        let mut trader = ship(1, TRADERS, AiType::BraveTrader, 200.0, 100.0);
        trader.visited = Some(StellarId(128));
        assert_eq!(decided(&[trader.clone()]), Goal::JumpOut);
        trader.class_fuel = 99;
        assert_eq!(decided(&[trader.clone()]), Goal::Wait);
        trader.goal = Goal::Wait;
        assert_eq!(decided(&[trader]), Goal::Wait, "state 6 is sticky");
        let waiting = |x, y| {
            let mut npcs = provoked(x, y);
            npcs[0].goal = Goal::Wait;
            npcs
        };
        assert_eq!(decided(&waiting(0.0, 0.0)), Goal::Attack(n(2)));
        assert_eq!(decided(&waiting(1251.0, 0.0)), Goal::Flee(n(2)));
    }

    #[test]
    fn a_provoked_held_brave_trader_fights_within_1250_or_flees() {
        let held = |x, y| {
            let mut npcs = provoked(x, y);
            npcs[0].goal = Goal::Hold;
            npcs
        };
        assert_eq!(decided(&held(1250.0, -1250.0)), Goal::Attack(n(2)));
        assert_eq!(decided(&held(1251.0, 0.0)), Goal::Flee(n(2)));
        let mut calm = held(0.0, 0.0);
        calm[0].provoked = 0.0;
        assert_eq!(decided(&calm), Goal::Hold, "unprovoked, it holds");
    }

    #[test]
    fn a_brave_trader_fights_back_within_1250_on_both_axes_and_flees_beyond() {
        assert_eq!(decided(&provoked(1250.0, -1250.0)), Goal::Attack(n(2)));
        assert_eq!(decided(&provoked(-1250.0, 1250.0)), Goal::Attack(n(2)));
        assert_eq!(decided(&provoked(1251.0, 0.0)), Goal::Flee(n(2)));
        assert_eq!(decided(&provoked(0.0, -1251.0)), Goal::Flee(n(2)));
        assert_eq!(STAND_AND_FIGHT, 1250.0);
    }

    /// The brave trader's call, among `npcs`.
    fn call(npcs: &[Npc]) -> Option<CommCall> {
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -5000.0), 0);
        BraveTrader::default()
            .decision(&npcs[0], &around, &mut Draws::of(&[0]))
            .comm_call
    }

    #[test]
    fn a_brave_trader_calls_for_help_fleeing_or_fighting_a_warship_or_interceptor() {
        assert_eq!(
            call(&provoked(1251.0, 0.0)),
            Some(CommCall::Distress),
            "fleeing"
        );
        assert_eq!(
            call(&provoked(100.0, 0.0)),
            Some(CommCall::Distress),
            "a warship"
        );
        let mut police = provoked(100.0, 0.0);
        police[1].ai_type = AiType::Interceptor;
        assert_eq!(call(&police), Some(CommCall::Distress), "an interceptor");
        for ai_type in [AiType::BraveTrader, AiType::WimpyTrader] {
            let mut trader = provoked(100.0, 0.0);
            trader[1].ai_type = ai_type;
            assert_eq!(call(&trader), None, "{ai_type:?}");
        }
        let mut calm = provoked(100.0, 0.0);
        calm[0].provoked = 0.0;
        assert_eq!(call(&calm), None, "idle");
    }

    /// Whether the brave trader's think calls for reinforcements, among
    /// `npcs`.
    fn calls_reinforcements(npcs: &[Npc]) -> bool {
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -5000.0), 0);
        BraveTrader::default()
            .decision(&npcs[0], &around, &mut Draws::of(&[0]))
            .calls_reinforcements
    }

    #[test]
    fn a_fleeing_brave_trader_calls_for_reinforcements_too_and_a_fighting_one_does_not() {
        assert!(
            calls_reinforcements(&provoked(1251.0, 0.0)),
            "fleeing (@0x8b6dc)"
        );
        assert!(
            !calls_reinforcements(&provoked(100.0, 0.0)),
            "fighting a warship, only for help"
        );
        let mut calm = provoked(100.0, 0.0);
        calm[0].provoked = 0.0;
        assert!(!calls_reinforcements(&calm), "idle");
    }

    #[test]
    fn a_brave_trader_fighting_the_player_does_not_call() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::BraveTrader, 0.0, 0.0);
        trader.provoked = 10.0;
        trader.target = Some(ShipRef::Player);
        let npcs = [trader];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        let decision = BraveTrader::default().decision(&npcs[0], &around, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, Goal::Attack(ShipRef::Player));
        assert_eq!(decision.comm_call, None);
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
    fn a_stand_off_brave_trader_never_asks_whether_a_chase_is_hopeless() {
        let mut npcs = provoked(0.0, -300.0);
        npcs[0].hull.mass = 100.0;
        npcs[0].hull.stands_off = true;
        npcs[1].goal = Goal::Flee(n(1));
        npcs[1].state.velocity = Vec2::new(0.0, -1.0);
        assert_eq!(decided(&npcs), Goal::Attack(n(2)));
        let mut far = provoked(1251.0, 0.0);
        far[0].hull.stands_off = true;
        assert_eq!(decided(&far), Goal::Flee(n(2)), "still flees beyond 1250");
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

    /// The brave trader's decision among `npcs`, its attacker breaking up.
    fn decided_on_dying(mut npcs: [Npc; 2]) -> Decision {
        npcs[1].condition = crate::combat::hull::Condition::Dying {
            ticks_left: 10,
            starting: false,
        };
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (0.0, -5000.0), 0);
        BraveTrader::default().decision(&npcs[0], &around, &mut Draws::of(&[0]))
    }

    #[test]
    fn an_attacker_breaking_up_within_1250_ends_the_attack_and_its_provocation() {
        // The think reads the slot's place and writes goal 4 within 1250
        // on both axes, `setle` (@0x8b616-0x8b68d), calling for help on a
        // warship (@0x8b6e3-0x8b71c); the high level finds it `_IsDying`
        // and writes goal 0, target -1, +0x96 0 (@0x8e3ef → @0x8e714).
        for (x, y) in [(100.0, 0.0), (1250.0, -1250.0), (-1250.0, 1250.0)] {
            let decision = decided_on_dying(provoked(x, y));
            assert_eq!(decision.goal, Goal::Idle, "({x}, {y})");
            assert!(decision.forgets_provocation, "({x}, {y})");
            assert!(!decision.calls_reinforcements, "({x}, {y})");
            assert_eq!(decision.comm_call, Some(CommCall::Distress), "a warship");
        }
        let mut hopeless = provoked(0.0, -300.0);
        hopeless[0].hull.mass = 100.0;
        hopeless[1].goal = Goal::Flee(n(1));
        hopeless[1].state.velocity = Vec2::new(0.0, -1.0);
        let decision = decided_on_dying(hopeless);
        assert_eq!(
            (decision.goal, decision.forgets_provocation),
            (Goal::Idle, true),
            "the dying test comes before the hopeless chase's (@0x8e48e)"
        );
    }

    #[test]
    fn an_attacker_breaking_up_beyond_1250_is_fled_and_the_provocation_kept() {
        // Goal 3 on a ship still in its slot: no zero (@0x8e228 needs
        // target -1).
        for (x, y) in [(1251.0, 0.0), (0.0, -1251.0)] {
            let decision = decided_on_dying(provoked(x, y));
            assert_eq!(decision.goal, Goal::Flee(n(2)), "({x}, {y})");
            assert!(!decision.forgets_provocation, "({x}, {y})");
            assert!(decision.calls_reinforcements, "@0x8b6dc");
            assert_eq!(decision.comm_call, Some(CommCall::Distress), "@0x8b6cc");
        }
        let mut calm = provoked(1251.0, 0.0);
        calm[0].provoked = 0.0;
        assert_eq!(
            decided_on_dying(calm).goal,
            Goal::Land(StellarId(128)),
            "unprovoked"
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
        assert_eq!(BraveTrader::default().target(&npcs[0], &around), Some(n(2)));
        assert_eq!(
            BraveTrader::default().trigger(&npcs[0], &around).only,
            Some(WeaponId(128))
        );
        let strike = Strike {
            damage: 7.0,
            ..crate::combat::fixture::strike(n(1), n(2))
        };
        assert_eq!(
            BraveTrader::default()
                .react(&npcs[0], &strike, &around)
                .target,
            Some(n(2))
        );
    }

    #[test]
    fn a_trader_that_stopped_on_losing_sight_keeps_its_target() {
        // State 6 keeps +0x70 (@0x8d803-0x8d80f).
        let govts = govts(0, 0);
        let sites = sites();
        let mut trader = ship(1, TRADERS, AiType::BraveTrader, 0.0, 0.0);
        trader.goal = Goal::Wait;
        trader.target = Some(ShipRef::Player);
        let npcs = [trader, ship(2, PIRATES, AiType::Warship, 50.0, 0.0)];
        let on_player = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(
            BraveTrader::default().target(&npcs[0], &on_player),
            Some(ShipRef::Player)
        );
        let mut on_npc = npcs.clone();
        on_npc[0].target = Some(n(2));
        let seen = around(&sites, &on_npc, &govts, (0.0, -100.0), 0);
        assert_eq!(BraveTrader::default().target(&on_npc[0], &seen), Some(n(2)));
        on_npc[0].goal = Goal::Idle;
        let idle = around(&sites, &on_npc, &govts, (0.0, -100.0), 0);
        assert_eq!(
            BraveTrader::default().target(&on_npc[0], &idle),
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
        let mut trader = ship(1, TRADERS, AiType::BraveTrader, 0.0, 0.0);
        trader.goal = Goal::Wait;
        trader.target = Some(n(2));
        let alone = [trader.clone()];
        let around_alone = around(&sites, &alone, &govts, (0.0, -100.0), 0);
        assert_eq!(
            BraveTrader::default().target(&alone[0], &around_alone),
            None
        );
        let mut pirate = ship(2, PIRATES, AiType::Warship, 50.0, 0.0);
        pirate.condition = crate::combat::hull::Condition::Destroyed;
        let npcs = [trader.clone(), pirate.clone()];
        let around_gone = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(BraveTrader::default().target(&npcs[0], &around_gone), None);
        pirate.condition = crate::combat::hull::Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let npcs = [trader, pirate];
        let around_dying = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(
            BraveTrader::default().target(&npcs[0], &around_dying),
            Some(n(2)),
            "breaking up, still active"
        );
    }
}
