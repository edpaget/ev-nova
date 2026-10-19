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
//!   `_HighLevelAIHandler` first makes the target -1 when its slot is
//!   gone (+0xb8 clear, @0x8d4e4-0x8d50d), and then zeroes the provocation
//!   in two blocks, after the type's think in the same dispatch:
//!   - state 3 with no target (@0x8dff6 → @0x8e228-0x8e23d): a flight
//!     from a ship gone, whatever its type ([`flight_ended`], which
//!     `NovaAi` adds after every type's think, as the handler runs after
//!     each, `_AIDispatch` @0x901e5), and an interceptor's
//!     out-of-ammunition retreat with nothing to flee
//!     ([`out_of_ammo::stops`]);
//!   - state 4 on a ship breaking up, still in its slot (@0x8e3e6 →
//!     @0x8e714-0x8e726), which the brave trader's and the interceptor's
//!     thinks leave it in, having no drop of their own
//!     ([`attack_breaking_up`]). The warship and the pirate drop such a
//!     target themselves, writing goal 0 and target -1 but no +0x96
//!     (`_WarshipAI` @0x8bc67-0x8bca9, `_PirateWarshipAI`
//!     @0x8c315-0x8c330), so theirs is kept. An attack on a ship gone
//!     from its slot, as one that has jumped out or landed, has target -1
//!     by then, and its goal-0 drop (@0x8e263 → @0x8e731) zeroes nothing.
//!
//!   A word that provokes the ship (+0x96 = 1, @0x8b7e5, @0x8cb9d) is
//!   written in the think, before the block, which then finds the player
//!   live: [`take`] zeroes first, then sets 1. Two more zeroing blocks are
//!   never reached: state 4 with no leader, target -1 and +0x8c -1
//!   (@0x8e39e-0x8e3c1), as every path there has passed the target -1
//!   test at @0x8e263; and the plunder's (state 0xd) target gone
//!   (@0x8e85d → @0x8eb3c-0x8eb54), as the slot test at @0x8d4e4 has made
//!   such a target -1 and the pirate's head drop clears one breaking up
//!   first. The block's other goal-0 writes zero nothing: an attack with
//!   target -1 (@0x8e731), the ally drop (@0x8e2d2), the fleet drops
//!   (@0x8e381, @0x8e7af, @0x8e820), the flee drop (@0x8e81d) and the
//!   plunder's (@0x8eb5f). The ally and fleet drops come before the
//!   liveness test, but the engine keeps no attack on a ship it spares
//!   ([`spares`](crate::ai::target::spares)), so the order shows nowhere.
//!
//!   One difference is left: a brave trader whose attacker is breaking
//!   up, or has just gone, reads the slot's last place, and attacks it
//!   within 1250 or flees it beyond, so whether it zeroes then turns on
//!   that place; here it decides again at once (its
//!   [`provoked_by`](crate::ai::provoked_by) needs a live ship), and its
//!   flight from a ship gone zeroes as any does. The engine keeps no gone
//!   ship's place, and only a later hit of no damage on such a trader
//!   shows which it did.
//! - **The rest of the traffic**: the count wait's start (@0x8c713,
//!   [`board_wait::enter`]); a ship not intact (`_AIDispatch` @0x8feef,
//!   the tick's third step); a pirate's capture of a ship, for each ship
//!   then targeting it (`_AIPlunderShipContents` @0x84872-0x8489b,
//!   [`plunder`](crate::traffic::plunder)); and -1 for each follower a
//!   lead entering its gate releases (@0x8dd14,
//!   [`gate_entry::RELEASED_PROVOCATION`]).

#[cfg(doc)]
use crate::ai::out_of_ammo;
use crate::ai::{Decision, Goal, Surroundings};
use crate::combat::hull::Condition;
use crate::traffic::npc::Npc;
#[cfg(doc)]
use crate::traffic::{board_wait, gate_entry};

/// Whether the high level, after the think that `decided`, still finds
/// `npc` attacking (state 4, [`Goal::attacking`]) a ship breaking up among
/// `around`, which it tests with `_IsDying` (@0x8e3ef). A think that turns
/// on another ship, or flees, leaves no such attack: its goal is what the
/// high level sees. One that lets the fight go (its
/// [`provoked_by`](crate::ai::provoked_by) needs a live ship) stands for
/// the original's think, which keeps goal 4 on +0x70, so the attack it
/// had before is the one tested. A ship gone from the system is no longer
/// in its slot, its target -1 by then (@0x8d4e4-0x8d50d); a disabled one
/// is there.
#[must_use]
pub(crate) fn attack_breaking_up(npc: &Npc, decided: Goal, around: &Surroundings) -> bool {
    let dying = |ship| matches!(around.condition_of(ship), Some(Condition::Dying { .. }));
    match fought(decided) {
        Some(_) => decided.attacking().is_some_and(dying),
        None => npc.goal.attacking().is_some_and(dying),
    }
}

/// Whether the high level, after the think that `decided`, still finds
/// `npc` fleeing (state 3) a ship gone from its slot among `around`: out
/// of the system or destroyed, not [`active`](Condition::active). It makes
/// that target -1 (@0x8d4e4-0x8d50d) and then state 0, zeroing its
/// provocation (@0x8dff6 → @0x8e228-0x8e23d). A think that turns on
/// another ship leaves no such flight; one that lets the fight go stands
/// for the original's, which keeps goal 3 on +0x70, so the flight it had
/// before is the one tested (see [`attack_breaking_up`]).
#[must_use]
pub(crate) fn flight_ended(npc: &Npc, decided: Goal, around: &Surroundings) -> bool {
    let gone = |ship| !around.condition_of(ship).is_some_and(Condition::active);
    match fought(decided) {
        Some(_) => matches!(decided, Goal::Flee(ship) if gone(ship)),
        None => matches!(npc.goal, Goal::Flee(ship) if gone(ship)),
    }
}

/// The ship `goal` flees or attacks, the fight the high level then sees.
fn fought(goal: Goal) -> Option<crate::combat::ShipRef> {
    match goal {
        Goal::Flee(ship) => Some(ship),
        _ => goal.attacking(),
    }
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

    fn breaking_up(npc: Npc) -> Npc {
        Npc {
            condition: Condition::Dying {
                ticks_left: 10,
                starting: false,
            },
            ..npc
        }
    }

    #[test]
    fn a_fight_given_up_for_a_target_gone_forgets_its_provocation() {
        // A brave trader's and an interceptor's thinks keep goal 4 on a
        // ship breaking up, and the high level ends it (@0x8e3ef →
        // @0x8e714).
        let foe = ship(2, PIRATES, AiType::Warship, 100.0, 0.0);
        let dying = breaking_up(foe.clone());
        let brave = BraveTrader::default();
        let brave_one = attacking(AiType::BraveTrader, TRADERS);
        assert!(
            decided(&brave, &brave_one, std::slice::from_ref(&dying)).forgets_provocation,
            "a brave trader"
        );
        let police = Interceptor::default();
        let police_one = attacking(AiType::Interceptor, POLICE);
        assert!(
            decided(&police, &police_one, std::slice::from_ref(&dying)).forgets_provocation,
            "an interceptor"
        );
        assert!(
            !decided(&brave, &brave_one, std::slice::from_ref(&foe)).forgets_provocation,
            "its attacker still there"
        );
        assert!(
            !decided(&brave, &brave_one, &[]).forgets_provocation,
            "its attacker gone from its slot: target -1, @0x8e731"
        );
        assert!(
            !decided(&police, &police_one, &[]).forgets_provocation,
            "@0x8e731 for an interceptor too"
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
    fn a_flight_from_a_ship_gone_from_its_slot_forgets_its_provocation_whatever_its_type() {
        // The slot test makes the target -1 (@0x8d4e4-0x8d50d), and state
        // 3 with none is state 0, +0x96 0 (@0x8e228-0x8e23d), after every
        // type's think.
        let foe = ship(2, PIRATES, AiType::Warship, 100.0, 0.0);
        for ai_type in [
            AiType::WimpyTrader,
            AiType::BraveTrader,
            AiType::Warship,
            AiType::Interceptor,
        ] {
            let mut fleeing = attacking(ai_type, TRADERS);
            fleeing.goal = Goal::Flee(n(2));
            let ai = crate::ai::NovaAi::default();
            assert!(
                decided(&ai, &fleeing, &[]).forgets_provocation,
                "{ai_type:?}, gone"
            );
            let destroyed = Npc {
                condition: Condition::Destroyed,
                ..foe.clone()
            };
            assert!(
                decided(&ai, &fleeing, &[destroyed]).forgets_provocation,
                "{ai_type:?}, destroyed"
            );
            for there in [foe.clone(), breaking_up(foe.clone())] {
                assert!(
                    !decided(&ai, &fleeing, &[there]).forgets_provocation,
                    "{ai_type:?}, in its slot"
                );
            }
        }
    }

    #[test]
    fn fleeing_a_gone_ship_and_hit_by_another_it_keeps_fleeing_the_other_and_its_provocation() {
        // `_DamageShip` makes Y the target and adds to +0x96; the think
        // writes goal 3 on Y, whose slot is live, so the zero at
        // @0x8e228-0x8e23d never runs.
        for ai_type in [AiType::WimpyTrader, AiType::BraveTrader] {
            let mut fleeing = ship(1, TRADERS, ai_type, 0.0, 0.0);
            fleeing.goal = Goal::Flee(n(2));
            fleeing.target = Some(n(3));
            fleeing.provoked = 10.0;
            let hitter = ship(3, PIRATES, AiType::Warship, 2000.0, 0.0);
            let decision = decided(&crate::ai::NovaAi::default(), &fleeing, &[hitter]);
            assert_eq!(decision.goal, Goal::Flee(n(3)), "{ai_type:?}");
            assert!(!decision.forgets_provocation, "{ai_type:?}");
            let mut npc = fleeing;
            take(&mut npc, &decision);
            assert_eq!(npc.provoked, 10.0, "{ai_type:?}");
        }
    }

    #[test]
    fn attacking_a_ship_breaking_up_and_provoked_by_another_it_turns_on_the_other_and_keeps_it() {
        // The think writes goal 4 on Y, not breaking up, so the zero at
        // @0x8e714-0x8e726 never runs.
        let dying = breaking_up(ship(2, PIRATES, AiType::Warship, 100.0, 0.0));
        let hitter = ship(3, PIRATES, AiType::Warship, 0.0, 100.0);
        let others = [dying, hitter];
        let behaviours: [(&dyn Behaviour, Npc); 2] = [
            (
                &BraveTrader::default(),
                attacking(AiType::BraveTrader, TRADERS),
            ),
            (
                &Interceptor::default(),
                attacking(AiType::Interceptor, POLICE),
            ),
        ];
        for (behaviour, mut npc) in behaviours {
            npc.target = Some(n(3));
            let decision = decided(behaviour, &npc, &others);
            assert_eq!(decision.goal.attacking(), Some(n(3)), "{:?}", npc.ai_type);
            assert!(!decision.forgets_provocation, "{:?}", npc.ai_type);
        }
    }

    #[test]
    fn a_warship_or_pirate_drops_a_target_breaking_up_itself_and_keeps_its_provocation() {
        // `_WarshipAI` @0x8bc67-0x8bca9 and `_PirateWarshipAI`
        // @0x8c315-0x8c330 write goal 0 and target -1, and no +0x96.
        let dying = [breaking_up(ship(2, PIRATES, AiType::Warship, 100.0, 0.0))];
        let warship = decided(
            &Warship::default(),
            &attacking(AiType::Warship, POLICE),
            &dying,
        );
        assert!(!warship.forgets_provocation, "{warship:?}");
        let pirate = decided(
            &PirateWarship::default(),
            &attacking(AiType::Warship, PIRATES),
            &dying,
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
    fn an_attack_is_on_a_ship_breaking_up_and_a_flight_from_one_gone_from_its_slot() {
        let (sites, govts) = (sites(), govts(0, 0));
        let foe = ship(2, PIRATES, AiType::Warship, 100.0, 0.0);
        // `test` of `npc` after a think that `decided`, among `foe`.
        let after = |test: fn(&Npc, Goal, &Surroundings) -> bool,
                     npc: &Npc,
                     decided: Goal,
                     foe: Option<&Npc>| {
            let npcs: Vec<Npc> = std::iter::once(npc.clone()).chain(foe.cloned()).collect();
            test(
                npc,
                decided,
                &around(&sites, &npcs, &govts, (0.0, -500.0), 0),
            )
        };
        let attacking = provoked(Goal::Attack(n(2)));
        // A think that lets the fight go: the goal before is tested.
        let breaking = |npc: &Npc, foe| after(attack_breaking_up, npc, Goal::Idle, foe);
        assert!(!breaking(&attacking, Some(&foe)), "there");
        assert!(!breaking(&attacking, None), "gone: target -1 (@0x8d50d)");
        let dying = breaking_up(foe.clone());
        assert!(breaking(&attacking, Some(&dying)), "@0x8e3ef");
        let disabled = Npc {
            condition: Condition::Disabled,
            ..foe.clone()
        };
        assert!(!breaking(&attacking, Some(&disabled)), "disabled is there");
        assert!(
            breaking(&provoked(Goal::Snipe(n(2))), Some(&dying)),
            "a snipe is goal 4"
        );
        for goal in [
            Goal::Flee(n(2)),
            Goal::Plunder {
                ship: n(2),
                hopeless: false,
            },
            Goal::Inspect(n(2)),
        ] {
            assert!(!breaking(&provoked(goal), Some(&dying)), "{goal:?}");
        }
        let fleeing = provoked(Goal::Flee(n(2)));
        let ended = |npc: &Npc, foe| after(flight_ended, npc, Goal::Idle, foe);
        assert!(ended(&fleeing, None), "gone");
        assert!(!ended(&fleeing, Some(&foe)), "there");
        assert!(
            !ended(&fleeing, Some(&dying)),
            "breaking up, still in its slot"
        );
        assert!(!ended(&attacking, None), "no flight");
        assert!(!ended(&provoked(Goal::Flee(ShipRef::Player)), Some(&foe)));
        // The goal the think leaves is the one the high level sees when it
        // fights: a flight from, or an attack on, another ship ends nothing.
        let player = Goal::Flee(ShipRef::Player);
        assert!(
            !after(flight_ended, &fleeing, player, None),
            "flees another"
        );
        assert!(!after(
            flight_ended,
            &fleeing,
            Goal::Attack(ShipRef::Player),
            None
        ));
        assert!(
            after(flight_ended, &provoked(Goal::Idle), Goal::Flee(n(2)), None),
            "flees one gone"
        );
        assert!(!after(flight_ended, &fleeing, Goal::Flee(n(2)), Some(&foe)));
        assert!(
            !after(attack_breaking_up, &attacking, player, Some(&dying)),
            "flees"
        );
        assert!(!after(
            attack_breaking_up,
            &attacking,
            Goal::Attack(ShipRef::Player),
            Some(&dying)
        ));
        let idle = provoked(Goal::Idle);
        assert!(
            after(attack_breaking_up, &idle, Goal::Attack(n(2)), Some(&dying)),
            "attacks one breaking up"
        );
        assert!(!after(
            attack_breaking_up,
            &idle,
            Goal::Attack(n(2)),
            Some(&foe)
        ));
    }
}
