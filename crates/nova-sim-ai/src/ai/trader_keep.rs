//! What Nova's traders keep of a goal other than 0, unprovoked, and goal
//! 4's high level for a fight they keep: `_WimpyTraderAI`
//! (@0x8b029-0x8b201 in the `EV Nova` executable) and `_BraveTraderAI`
//! (@0x8b493-0x8b728), read for the goals a think holds that it didn't
//! write itself, such as a lost lead's that the succession copies onto its
//! successor (`_AICheckParentExists` @0x893b0-0x893e1, see
//! [`succession`](crate::traffic::succession)).
//!
//! Each think returns when disabled (@0x8b038, @0x8b49f), for goal 9 or
//! 0xf (@0x8b045-0x8b05c, @0x8b4af-0x8b4c6; the traffic's `assist_think`)
//! and for goal 0x16 ([`Goal::CountWait`], @0x8b062, @0x8b4cc), before it
//! reads its provocation: the count wait is
//! [`board_wait::lapse`](crate::traffic::board_wait::lapse)'s to end. Its
//! idle block runs on goal 0 alone (@0x8b06c, @0x8b4d6), and every other
//! goal reaches the provoked test (@0x8b1ac, @0x8b616), which writes
//! nothing unprovoked or with +0x70 -1. So such a trader keeps the goal it
//! holds, and the high level flies it ([`kept`]):
//!
//! - a plunder ([`Goal::Plunder`]), whether its ship is there or gone:
//!   goal 0xd's high level docks with a disabled ship, chases an intact
//!   one, fleeing a hopeless chase, and lets a gone one go
//!   ([`plunder_high`](super::plunder_high) and the traffic's
//!   [`plunder::high_level`](crate::traffic::plunder::high_level));
//! - a count wait;
//! - an attack, snipe or flight, on the ship +0x70 names ([`held_target`]):
//!   one in its slot (breaking up or disabled included), or one gone whose
//!   freed slot +0x70 still names ([`Surroundings::held_gone`]). Neither
//!   the think nor the high level reads a goal's quarry, only +0x70, so a
//!   fight whose +0x70 a hit moved (`_DamageShip` @0x3bcf6) goes on
//!   against the ship it names now. With +0x70 -1 the fight is kept on
//!   its quarry for the think's calls, and then ends (see "The last
//!   think" below);
//! - a mine, a scoop and a stellar attack (goals 0x10, 0x11 and 0x12),
//!   whose high-level blocks are type-free and end none of them
//!   (@0x8f832-0x8f903, @0x8faf3-0x8fafd, @0x8f906-0x8faeb), as the
//!   traffic flies them by their goal;
//! - an inspection (goal 7), whether its ship is there or gone: the
//!   state-7 block (@0x8f11e-0x8f3c5) is type-free too, and ends it with
//!   its ship gone or within reach, scanning the player there
//!   ([`inspect_high`](super::inspect_high)).
//!
//! The goals the idle block's [`kept`](super::kept) keeps already (a
//! stellar, a gate, a jump out, a hold, a wait, a drift) stay its own.
//! A fighter's return (goal 5) is never copied: no ship leads a fighter.
//!
//! **The last think** of a fight whose +0x70 went: with +0x70 -1 (the
//! slot test cleared it, or the trader let go of a gone slot at its step,
//! `_HandleShip` @0x33865-0x33888), the kept goal 3 or 4 makes the think's
//! calls on the goal it holds: a flight calls for help
//! (`_WimpyTraderAI` @0x8b1e1-0x8b1f5, `_BraveTraderAI` @0x8b6cc) and,
//! for the brave trader, for reinforcements (@0x8b6dc); an attack or a
//! snipe makes none (goal 4's call needs +0x70 not -1, @0x8b6ed). The
//! high level then ends it in the same dispatch ([`last_think`]): a flight
//! goes to goal 0 with +0x96 0 (@0x8dff6 → @0x8e228-0x8e23d), an attack
//! or snipe to goal 0 with +0x96 kept (@0x8e263 → @0x8e731). A fight on a
//! gone ship +0x70 still names is ended by the high level's slot test
//! instead, after calls made on its record
//! ([`with_flight_end`](crate::traffic::provocation::with_flight_end),
//! [`handler_head::slot_test`]). A flight under way (+0x50 above 0)
//! whose +0x70 went is taken to goal 2 by the hold before goal 3's block
//! in the original (@0x8d535-0x8d56b), keeping +0x96; the engine zeroes
//! it, which rdm:phase/npc-ai-2 records.
//!
//! **Goal 4's high level** then takes a kept attack or snipe, or a brave
//! trader's own attack, as it takes every type's
//! ([`attack_drop`](super::attack_drop)): an attack on a ship breaking up
//! ends (@0x8e3ef → @0x8e714: goal 0, target -1, +0x96 0), and a hopeless
//! chase is fled, as a trader is of AI type 2 or below (@0x8e60a-0x8e62e),
//! that flight's first frame goal 4's
//! ([`hopeless_flee`](super::hopeless_flee)). The think's calls read the
//! goal it wrote, before the high level. Goal 4's ally and fleet drops
//! (@0x8e277-0x8e399) never meet a kept attack: a copied target was the
//! lead's own, under the same government and fleet, so the lead's high
//! level spared it already; a bribed follower never takes over
//! ([`succession`](crate::traffic::succession)); and the player's
//! government is never allied.
//!
//! Nothing here plays a sound or draws a random number.

use crate::ai::{Decision, Goal, Surroundings, handler_head, idle};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::traffic::npc::Npc;

/// The goal trader `npc`'s think keeps among `around`, unprovoked, of
/// those its idle block's [`kept`](super::kept) doesn't (see the module
/// docs): a plunder, a count wait, a mine, a scoop, a stellar attack or
/// an inspection, and an attack, snipe or flight, on the ship its target
/// names ([`held_target`]), or on its quarry when that went, for the
/// think's calls before [`last_think`] ends it. None for any other goal.
#[must_use]
pub fn kept(npc: &Npc, around: &Surroundings) -> Option<Goal> {
    match npc.goal {
        Goal::Plunder { .. }
        | Goal::CountWait
        | Goal::Mine
        | Goal::Scoop
        | Goal::AttackStellar(_)
        | Goal::Inspect(_) => Some(npc.goal),
        Goal::Attack(quarry) | Goal::Snipe(quarry) | Goal::Flee(quarry) => {
            let on = held_target(npc, around).unwrap_or(quarry);
            Some(match npc.goal {
                Goal::Attack(_) => Goal::Attack(on),
                Goal::Snipe(_) => Goal::Snipe(on),
                _ => Goal::Flee(on),
            })
        }
        _ => None,
    }
}

/// What trader `npc`'s think reads as its +0x70 among `around`: its target
/// ([`Npc::slot_target`]) while its slot is
/// [`active`](Surroundings::active), or a gone ship whose freed slot it
/// still names ([`Surroundings::held_gone`]). None otherwise: a target
/// naming a gone slot not held was let go at the ship's step
/// (`_HandleShip` @0x33865-0x33888).
#[must_use]
pub fn held_target(npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
    npc.slot_target()
        .filter(|&ship| around.active(ship))
        .or_else(|| around.held_gone(npc).map(|gone| ShipRef::Npc(gone.id)))
}

/// Trader `npc`'s `decision` among `around`, its calls made, through the
/// high level's end of a fight its think kept with +0x70 -1
/// ([`held_target`] none; see the module docs): a flight becomes goal 0,
/// target -1, +0x96 0, its calls kept (@0x8dff6 → @0x8e228-0x8e23d); an
/// attack or snipe becomes goal 0, target -1, +0x96 kept, and makes no
/// call, as goal 4's call needs +0x70 not -1 (@0x8b6ed; @0x8e263 →
/// @0x8e731). A decision the wait stops at the handler's head is left
/// whole ([`handler_head::skips`], @0x8d4b9-0x8d4d2), as is one whose
/// target stands: a flight from a gone ship still held is ended by
/// [`with_flight_end`](crate::traffic::provocation::with_flight_end), an
/// attack on one by [`handler_head::slot_test`].
#[must_use]
pub fn last_think(npc: &Npc, around: &Surroundings, decision: Decision) -> Decision {
    if handler_head::skips(npc, &decision) || held_target(npc, around).is_some() {
        return decision;
    }
    match decision.goal {
        Goal::Flee(_) => Decision {
            goal: Goal::Idle,
            clears_target: true,
            forgets_provocation: true,
            ..decision
        },
        Goal::Attack(_) | Goal::Snipe(_) => Decision {
            goal: Goal::Idle,
            clears_target: true,
            comm_call: None,
            ..decision
        },
        _ => decision,
    }
}

/// What trader `npc`'s think gives among `around` when its provoked test
/// writes nothing: a goal it keeps ([`kept`](super::kept), then
/// [`kept`]), else its idle block's ([`idle`]), rolling on `chance`.
pub fn unprovoked(npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
    super::kept(npc, around)
        .or_else(|| kept(npc, around))
        .unwrap_or_else(|| idle(npc, around, chance))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::CommCall;
    use crate::ai::fixture::{PIRATES, TRADERS, around, govts, n, ship, sites};
    use crate::catalog::StellarId;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::traffic::gone::{Freed, GoneShip};
    use crate::traffic::npc::AiType;
    use nova_sim_base::fixture::Draws;

    /// A wimpy trader (NPC 1) at the centre holding `goal`, unprovoked,
    /// and a pirate (NPC 2) 300 below it, in `condition`.
    fn holding(goal: Goal, condition: Condition) -> [Npc; 2] {
        let mut trader = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        trader.goal = goal;
        trader.target = goal.quarry();
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.condition = condition;
        [trader, pirate]
    }

    fn kept_among(npcs: &[Npc]) -> Option<Goal> {
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -1000.0), 0);
        kept(&npcs[0], &around)
    }

    #[test]
    fn a_plunder_is_kept_whether_its_ship_is_intact_disabled_or_gone() {
        for (condition, alone) in [
            (Condition::Intact, false),
            (Condition::Disabled, false),
            (Condition::Intact, true),
        ] {
            let goal = Goal::Plunder {
                ship: n(2),
                hopeless: false,
            };
            let npcs = holding(goal, condition);
            let npcs = if alone { &npcs[..1] } else { &npcs[..] };
            assert_eq!(kept_among(npcs), Some(goal), "{condition:?}, gone {alone}");
        }
    }

    /// The three fights a trader keeps, on `ship`.
    fn fights(ship: ShipRef) -> [Goal; 3] {
        [Goal::Attack(ship), Goal::Snipe(ship), Goal::Flee(ship)]
    }

    #[test]
    fn a_fight_is_kept_on_the_ship_its_target_names_live_breaking_up_disabled_or_held_gone() {
        // Any goal but 0 skips the idle block (@0x8b06c, @0x8b4d6), and the
        // provoked test writes nothing unprovoked (@0x8b1ac, @0x8b616).
        for goal in fights(n(2)) {
            for condition in [
                Condition::Intact,
                Condition::Disabled,
                Condition::Dying {
                    ticks_left: 3,
                    starting: false,
                },
            ] {
                let npcs = holding(goal, condition);
                assert_eq!(kept_among(&npcs), Some(goal), "{goal:?} {condition:?}");
            }
            let [trader, mut pirate] = holding(goal, Condition::Intact);
            pirate.berth = 2;
            let gone = [GoneShip {
                ship: pirate,
                freed: Freed::InDispatch,
            }];
            let mut reader = trader;
            reader.berth = 3;
            assert_eq!(
                kept_with_gone(&reader, &gone),
                Some(goal),
                "{goal:?}: held gone"
            );
        }
    }

    /// [`kept`] for `trader`, alone in the system but for the `gone`.
    fn kept_with_gone(trader: &Npc, gone: &[GoneShip]) -> Option<Goal> {
        let govts = govts(0, 0);
        let sites = sites();
        let npcs = [trader.clone()];
        let around = Surroundings {
            gone,
            ..around(&sites, &npcs, &govts, (0.0, -1000.0), 0)
        };
        kept(&npcs[0], &around)
    }

    #[test]
    fn a_fight_is_kept_on_a_ship_a_hit_moved_its_target_to() {
        // Neither the think nor the high level reads the goal's quarry,
        // only +0x70, which `_DamageShip` moved (@0x3bcf6).
        for goal in fights(n(2)) {
            let mut npcs = holding(goal, Condition::Intact);
            npcs[0].target = Some(n(3));
            let hitter = ship(3, PIRATES, AiType::Warship, 300.0, 0.0);
            let npcs = [npcs[0].clone(), npcs[1].clone(), hitter];
            let moved = match goal {
                Goal::Attack(_) => Goal::Attack(n(3)),
                Goal::Snipe(_) => Goal::Snipe(n(3)),
                _ => Goal::Flee(n(3)),
            };
            assert_eq!(kept_among(&npcs), Some(moved), "{goal:?}");
        }
    }

    #[test]
    fn a_fight_whose_target_went_is_kept_on_its_quarry_for_the_last_think_to_end() {
        // +0x70 -1, or a gone slot let go at the trader's step: the goal
        // stands into the think's calls (@0x8b1e1-0x8b1f5, @0x8b6bc-0x8b71c).
        for goal in fights(n(2)) {
            let mut npcs = holding(goal, Condition::Intact);
            npcs[0].target = None;
            assert_eq!(kept_among(&npcs), Some(goal), "{goal:?}: no target");
            let npcs = holding(goal, Condition::Intact);
            assert_eq!(kept_among(&npcs[..1]), Some(goal), "{goal:?}: let go");
        }
    }

    /// A trader's decision of `goal` with a distress call.
    fn calling(goal: Goal) -> Decision {
        Decision {
            goal,
            comm_call: Some(CommCall::Distress),
            calls_reinforcements: matches!(goal, Goal::Flee(_)),
            ..Decision::default()
        }
    }

    /// [`last_think`] of a provoked trader holding `goal` with `target`,
    /// among the pirate (NPC 2) and the `gone`, of its decision `decision`.
    fn ended(
        goal: Goal,
        target: Option<ShipRef>,
        gone: &[GoneShip],
        decision: Decision,
    ) -> Decision {
        let [mut trader, pirate] = holding(goal, Condition::Intact);
        trader.target = target;
        trader.provoked = 10.0;
        trader.berth = 3;
        let govts = govts(0, 0);
        let sites = sites();
        let npcs = [trader, pirate];
        let around = Surroundings {
            gone,
            ..around(&sites, &npcs, &govts, (0.0, -1000.0), 0)
        };
        last_think(&npcs[0], &around, decision)
    }

    #[test]
    fn a_kept_flight_whose_target_went_ends_zeroed_with_its_calls() {
        // Goal 3 on +0x70 -1: goal 0, sub 0, +0x96 0 (@0x8dff6 →
        // @0x8e228-0x8e23d), after the think's calls.
        let flight = calling(Goal::Flee(n(2)));
        assert_eq!(
            ended(Goal::Flee(n(2)), None, &[], flight),
            Decision {
                goal: Goal::Idle,
                clears_target: true,
                forgets_provocation: true,
                ..flight
            }
        );
    }

    #[test]
    fn a_kept_attack_or_snipe_whose_target_went_ends_keeping_its_provocation_with_no_call() {
        // Goal 4 on +0x70 -1: goal 0, +0x96 kept (@0x8e263 → @0x8e731), and
        // goal 4's call needs +0x70 not -1 (@0x8b6ed).
        for goal in [Goal::Attack(n(2)), Goal::Snipe(n(2))] {
            assert_eq!(
                ended(goal, None, &[], calling(goal)),
                Decision {
                    goal: Goal::Idle,
                    clears_target: true,
                    ..Decision::default()
                },
                "{goal:?}"
            );
        }
    }

    #[test]
    fn a_fight_whose_target_stands_or_is_held_gone_is_left_to_the_high_level() {
        for goal in fights(n(2)) {
            let decision = calling(goal);
            assert_eq!(ended(goal, Some(n(2)), &[], decision), decision, "{goal:?}");
            let mut away = ship(9, PIRATES, AiType::Warship, 0.0, 0.0);
            away.berth = 2;
            let gone = [GoneShip {
                ship: away,
                freed: Freed::InDispatch,
            }];
            let on_gone = calling(Goal::Flee(n(9)));
            assert_eq!(
                ended(goal, Some(n(9)), &gone, on_gone),
                on_gone,
                "{goal:?}: held gone"
            );
        }
        let idle = calling(Goal::Land(StellarId(128)));
        assert_eq!(ended(Goal::Idle, None, &[], idle), idle, "no fight");
    }

    #[test]
    fn a_kept_fight_whose_target_went_stands_while_the_wait_stops_the_high_level() {
        // The wait exit comes before the slot test and the blocks
        // (@0x8d4b9-0x8d4d2), so goal 3 or 4 stands with its calls.
        let [mut trader, pirate] = holding(Goal::Flee(n(2)), Condition::Intact);
        trader.target = None;
        trader.ai_wait = 3.0;
        let govts = govts(0, 0);
        let sites = sites();
        let npcs = [trader, pirate];
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        let flight = calling(Goal::Flee(n(2)));
        assert_eq!(last_think(&npcs[0], &around, flight), flight);
    }

    #[test]
    fn a_count_wait_a_mine_a_scoop_and_a_stellar_attack_are_kept() {
        for goal in [
            Goal::CountWait,
            Goal::Mine,
            Goal::Scoop,
            Goal::AttackStellar(StellarId(128)),
        ] {
            assert_eq!(
                kept_among(&holding(goal, Condition::Intact)),
                Some(goal),
                "{goal:?}"
            );
        }
    }

    #[test]
    fn an_inspection_is_kept_whether_its_ship_is_there_or_gone() {
        // The state-7 block ends it whatever the type (`inspect_high`).
        let goal = Goal::Inspect(n(2));
        let npcs = holding(goal, Condition::Intact);
        assert_eq!(kept_among(&npcs), Some(goal), "there");
        assert_eq!(kept_among(&npcs[..1]), Some(goal), "gone");
        let player = Goal::Inspect(ShipRef::Player);
        assert_eq!(
            kept_among(&holding(player, Condition::Intact)),
            Some(player)
        );
    }

    #[test]
    fn an_idle_or_the_idle_blocks_own_goals_are_not_kept_here() {
        for goal in [
            Goal::Idle,
            Goal::Land(StellarId(128)),
            Goal::JumpOut,
            Goal::Hold,
            Goal::Wait,
            Goal::Drift,
        ] {
            assert_eq!(
                kept_among(&holding(goal, Condition::Intact)),
                None,
                "{goal:?}"
            );
        }
    }

    #[test]
    fn unprovoked_a_trader_keeps_its_goal_or_goes_about_its_business() {
        let govts = govts(0, 0);
        let sites = sites();
        let goal = Goal::Plunder {
            ship: n(2),
            hopeless: false,
        };
        let npcs = holding(goal, Condition::Intact);
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        let mut chance = Draws::of(&[0]);
        assert_eq!(unprovoked(&npcs[0], &around, &mut chance), goal);
        let npcs = holding(Goal::Idle, Condition::Intact);
        let around = crate::ai::fixture::around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        assert_eq!(
            unprovoked(&npcs[0], &around, &mut chance),
            Goal::Land(StellarId(128))
        );
    }

    #[test]
    fn a_led_trader_follows_its_lead_before_any_goal_it_keeps() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut npcs = holding(Goal::Attack(n(2)), Condition::Intact);
        npcs[0].leader = Some(crate::traffic::npc::NpcId(2));
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        assert_eq!(
            unprovoked(&npcs[0], &around, &mut Draws::of(&[])),
            Goal::Follow {
                lead: crate::traffic::npc::NpcId(2),
                guard: None
            }
        );
    }
}
