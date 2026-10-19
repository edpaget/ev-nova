//! A ship's provocation ([`Npc::provoked`], the original's +0x96, a
//! 16-bit word kept here as an `f32`, its width not replicated), zeroed
//! only where the original writes it, in the `EV Nova` executable.
//!
//! `_DamageShip` adds a hit's damage to it, for damage above 0 alone
//! (@0x3bcc0-0x3bcf0), though any hit by a ship sets the target
//! (@0x3bcf6; see [`react`](crate::ai::react)). The type thinks read it
//! beside a target that is not -1: `_WimpyTraderAI` @0x8b1ac, `_MinerAI`
//! @0x8b254, `_BraveTraderAI` @0x8b616, `_WarshipAI` @0x8bb0c and
//! `_InterceptorAI` @0x8cbf6 and @0x8ccc1, each a `cmpw $0` and a signed
//! `jle`, so -1 reads as unprovoked (see
//! [`provoked_by`](crate::ai::provoked_by)).
//!
//! It is kept through every goal a think takes but where a routine
//! writes it: a leave, a landing, a stop, an inspection, an idle think, a
//! follow. So a ship provoked, then bribed, that takes a hit of no
//! damage from a ship flees it (`_AIMakeShipLeave` @0x7e2b0-0x7e2e7
//! writes no +0x96).
//!
//! - **The high level** ([`Decision::forgets_provocation`], [`take`]):
//!   `_HighLevelAIHandler` zeroes it in two blocks, after the type's think
//!   in the same dispatch:
//!   - state 3 with no target (@0x8dff6 → @0x8e228-0x8e23d), which an
//!     interceptor's out-of-ammunition retreat with nothing to flee makes
//!     ([`out_of_ammo::stops`]);
//!   - state 4 on a ship gone or breaking up (@0x8e3e6 → @0x8e714-0x8e726),
//!     which the brave trader's and the interceptor's thinks leave it in,
//!     having no drop of their own ([`attack_gone`]). The warship and the
//!     pirate drop such a target themselves, writing goal 0 and target -1
//!     but no +0x96 (`_WarshipAI` @0x8bc67-0x8bca9, `_PirateWarshipAI`
//!     @0x8c315-0x8c330), so theirs is kept.
//!
//!   A word that provokes the ship (+0x96 = 1, @0x8b7e5, @0x8cb9d) is
//!   written in the think, before the block, which then finds the player
//!   live: [`take`] zeroes first, then sets 1. Two more zeroing blocks are
//!   never reached: state 4 with no leader, target -1 and +0x8c -1
//!   (@0x8e39e-0x8e3c1), as every path there has passed the target -1
//!   test at @0x8e263; and the plunder's (state 0xd) target gone
//!   (@0x8e85d → @0x8eb3c-0x8eb54), as the pirate's head drop clears such
//!   a target first. The block's other goal-0 writes zero nothing: an
//!   attack with target -1 (@0x8e731), the ally drop (@0x8e2d2), the fleet
//!   drops (@0x8e381, @0x8e7af, @0x8e820), the flee drop (@0x8e81d) and
//!   the plunder's (@0x8eb5f). The ally and fleet drops come before the
//!   liveness test, but the engine keeps no attack on a ship it spares
//!   ([`spares`](crate::ai::target::spares)), so the order shows nowhere.
//!   A brave trader whose attacker has gone reads the gone slot's last
//!   place and attacks it within 1250 (then zeroed) or flees it beyond
//!   (kept); here it decides again, its provocation zeroed either way, as
//!   the engine keeps no gone ship's place. Only a later hit of no damage
//!   on a trader that fled a ship already gone shows it.
//! - **The rest of the traffic**: the count wait's start (@0x8c713,
//!   [`board_wait::enter`]); a ship not intact (`_AIDispatch` @0x8feef,
//!   the tick's third step); and a pirate's capture of a ship, for each
//!   ship then targeting it (`_AIPlunderShipContents` @0x84872-0x8489b,
//!   [`plunder`](crate::traffic::plunder)).

#[cfg(doc)]
use crate::ai::out_of_ammo;
use crate::ai::{Decision, Surroundings};
#[cfg(doc)]
use crate::traffic::board_wait;
use crate::traffic::npc::Npc;

/// Whether `npc`'s attack (state 4, [`Goal::attacking`]) is on a ship gone
/// or breaking up among `around`: not [`live`](Surroundings::live), as
/// the high level tests the slot's +0xb8 and `_IsDying` (@0x8e3df-0x8e3f6).
/// A disabled ship is there.
///
/// [`Goal::attacking`]: crate::ai::Goal::attacking
#[must_use]
pub(crate) fn attack_gone(npc: &Npc, around: &Surroundings) -> bool {
    npc.goal.attacking().is_some_and(|ship| !around.live(ship))
}

/// `npc` takes `decision`'s writes of its provocation: zeroed where the
/// high level ends its fight ([`Decision::forgets_provocation`]), then 1
/// where the think's word provokes it ([`Decision::provokes`]), and
/// otherwise kept, whatever the goal.
pub(super) fn take(npc: &mut Npc, decision: &Decision) {
    if decision.forgets_provocation {
        npc.provoked = 0.0;
    }
    if decision.provokes {
        npc.provoked = 1.0;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::brave::BraveTrader;
    use crate::ai::fire::AmmoLeft;
    use crate::ai::fixture::{
        PIRATES, POLICE, TRADERS, around, govts, n, ship, short_of_ammo, sites,
    };
    use crate::ai::interceptor::Interceptor;
    use crate::ai::pirate::PirateWarship;
    use crate::ai::warship::Warship;
    use crate::ai::{Behaviour, Goal, Help};
    use crate::catalog::StellarId;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::traffic::npc::AiType;
    use nova_sim_base::fixture::Draws;

    fn provoked(goal: Goal) -> Npc {
        let mut npc = ship(1, TRADERS, AiType::BraveTrader, 0.0, 0.0);
        npc.provoked = 5.0;
        npc.goal = goal;
        npc
    }

    fn taking(decision: Decision) -> f32 {
        let mut npc = provoked(Goal::Attack(n(2)));
        take(&mut npc, &decision);
        npc.provoked
    }

    #[test]
    fn a_ship_keeps_its_provocation_as_it_leaves_lands_stops_inspects_or_idles() {
        for goal in [
            Goal::JumpOut,
            Goal::Land(StellarId(128)),
            Goal::EnterGate(StellarId(128)),
            Goal::Wait,
            Goal::Inspect(ShipRef::Player),
            Goal::Idle,
            Goal::Follow(crate::traffic::npc::NpcId(2)),
            Goal::Assist(Help::Refuel),
        ] {
            let kept = taking(Decision {
                goal,
                ..Decision::default()
            });
            assert_eq!(kept, 5.0, "{goal:?}");
        }
    }

    #[test]
    fn a_fight_the_high_level_ends_forgets_its_provocation_and_a_word_then_sets_1() {
        let forgets = Decision {
            goal: Goal::Idle,
            forgets_provocation: true,
            ..Decision::default()
        };
        assert_eq!(taking(forgets), 0.0, "@0x8e23d, @0x8e71d");
        let worded = Decision {
            goal: Goal::Attack(ShipRef::Player),
            forgets_provocation: true,
            provokes: true,
            ..Decision::default()
        };
        assert_eq!(taking(worded), 1.0, "the word's 1 after (@0x8b7e5)");
    }

    /// `npc`'s decision by `behaviour` among `others`, the player far off.
    fn decided(behaviour: &dyn Behaviour, npc: &Npc, others: &[Npc]) -> Decision {
        let (sites, govts) = (sites(), govts(0, 0));
        let npcs: Vec<Npc> = std::iter::once(npc.clone())
            .chain(others.iter().cloned())
            .collect();
        let around = around(&sites, &npcs, &govts, (0.0, -5000.0), 0);
        behaviour.decision(npc, &around, &mut Draws::of(&[0; 16]))
    }

    fn attacking(ai_type: AiType, govt: crate::catalog::GovtId) -> Npc {
        let mut npc = ship(1, govt, ai_type, 0.0, 0.0);
        npc.provoked = 5.0;
        npc.goal = Goal::Attack(n(2));
        npc.target = Some(n(2));
        npc
    }

    #[test]
    fn a_fight_given_up_for_a_target_gone_forgets_its_provocation() {
        // A brave trader's and an interceptor's thinks keep goal 4 on a
        // ship gone, and the high level ends it (@0x8e3e6 → @0x8e714).
        let brave = BraveTrader::default();
        assert!(
            decided(&brave, &attacking(AiType::BraveTrader, TRADERS), &[]).forgets_provocation,
            "a brave trader"
        );
        let police = Interceptor::default();
        assert!(
            decided(&police, &attacking(AiType::Interceptor, POLICE), &[]).forgets_provocation,
            "an interceptor"
        );
        let foe = ship(2, PIRATES, AiType::Warship, 100.0, 0.0);
        assert!(
            !decided(&brave, &attacking(AiType::BraveTrader, TRADERS), &[foe]).forgets_provocation,
            "its attacker still there"
        );
        // Short of ammunition with no target: state 3, which the high level
        // makes 0 at once (@0x8dff6 → @0x8e228-0x8e23d).
        let mut waiting = short_of_ammo(
            ship(1, POLICE, AiType::Interceptor, 0.0, 0.0),
            AmmoLeft::Dry,
            true,
        );
        waiting.goal = Goal::Wait;
        waiting.provoked = 5.0;
        let stopped = decided(&police, &waiting, &[]);
        assert_eq!(stopped.goal, Goal::Idle);
        assert!(stopped.forgets_provocation, "@0x8e23d");
        waiting.hull.retreats_dry = false;
        assert!(
            !decided(&police, &waiting, &[]).forgets_provocation,
            "no retreat, no flight"
        );
    }

    #[test]
    fn a_warship_or_pirate_drops_a_target_gone_itself_and_keeps_its_provocation() {
        // `_WarshipAI` @0x8bc67-0x8bca9 and `_PirateWarshipAI`
        // @0x8c315-0x8c330 write goal 0 and target -1, and no +0x96.
        let warship = decided(
            &Warship::default(),
            &attacking(AiType::Warship, POLICE),
            &[],
        );
        assert!(!warship.forgets_provocation, "{warship:?}");
        let pirate = decided(
            &PirateWarship::default(),
            &attacking(AiType::Warship, PIRATES),
            &[],
        );
        assert!(!pirate.forgets_provocation, "{pirate:?}");
    }

    #[test]
    fn a_fight_dropped_for_an_ally_or_a_flee_drop_keeps_it() {
        // The flee drop (@0x8e81d) and the ally drop (@0x8e2d2) write goal
        // 0 and no +0x96.
        let (sites, govts) = (sites(), govts(0, 0));
        let mut fleeing = ship(1, POLICE, AiType::BraveTrader, 0.0, 0.0);
        fleeing.provoked = 5.0;
        let ally = ship(2, TRADERS, AiType::BraveTrader, 100.0, 0.0);
        let npcs = [fleeing.clone(), ally];
        let around = around(&sites, &npcs, &govts, (0.0, -5000.0), 0);
        let dropped = crate::ai::flee_drop::with_flee_drop(
            &fleeing,
            Decision {
                goal: Goal::Flee(n(2)),
                ..Decision::default()
            },
            &around,
            crate::rulebook::RuleSource::Engine,
        );
        assert_eq!(dropped.goal, Goal::Idle, "dropped");
        let mut npc = fleeing;
        take(&mut npc, &dropped);
        assert_eq!(npc.provoked, 5.0);
        let mut police = attacking(AiType::Warship, POLICE);
        police.goal = Goal::Attack(n(2));
        let spared = decided(
            &Warship::default(),
            &police,
            &[ship(2, TRADERS, AiType::BraveTrader, 100.0, 0.0)],
        );
        assert!(!spared.forgets_provocation, "{spared:?}");
    }

    #[test]
    fn an_attack_is_on_a_ship_gone_when_it_is_out_of_the_system_or_breaking_up() {
        let (sites, govts) = (sites(), govts(0, 0));
        let foe = ship(2, PIRATES, AiType::Warship, 100.0, 0.0);
        let gone = |npc: &Npc, foe: Option<&Npc>| {
            let npcs: Vec<Npc> = std::iter::once(npc.clone()).chain(foe.cloned()).collect();
            attack_gone(npc, &around(&sites, &npcs, &govts, (0.0, -500.0), 0))
        };
        let attacking = provoked(Goal::Attack(n(2)));
        assert!(!gone(&attacking, Some(&foe)), "there");
        assert!(gone(&attacking, None), "gone (+0xb8 0, @0x8e3e6)");
        let dying = Npc {
            condition: Condition::Dying {
                ticks_left: 10,
                starting: false,
            },
            ..foe.clone()
        };
        assert!(gone(&attacking, Some(&dying)), "breaking up (@0x8e3ef)");
        let disabled = Npc {
            condition: Condition::Disabled,
            ..foe.clone()
        };
        assert!(!gone(&attacking, Some(&disabled)), "disabled is there");
        assert!(
            gone(&provoked(Goal::Snipe(n(2))), None),
            "a snipe is goal 4"
        );
        for goal in [
            Goal::Flee(n(2)),
            Goal::Plunder {
                ship: n(2),
                hopeless: false,
            },
            Goal::Inspect(n(2)),
            Goal::Idle,
        ] {
            assert!(!gone(&provoked(goal), None), "{goal:?} is no attack");
        }
    }
}
