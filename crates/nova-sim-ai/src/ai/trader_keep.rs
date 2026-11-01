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
//! nothing unprovoked. So an unprovoked trader keeps the goal it holds,
//! and the high level flies it ([`kept`]):
//!
//! - a plunder ([`Goal::Plunder`]), whether its ship is there or gone:
//!   goal 0xd's high level docks with a disabled ship, chases an intact
//!   one and lets a gone one go (the traffic's
//!   [`plunder::high_level`](crate::traffic::plunder::high_level));
//! - a count wait;
//! - an attack, snipe or flight on a ship still in the system (dying or
//!   disabled included);
//! - a mine, a scoop and a stellar attack (goals 0x10, 0x11 and 0x12),
//!   whose high-level blocks are type-free and end none of them
//!   (@0x8f832-0x8f903, @0x8faf3-0x8fafd, @0x8f906-0x8faeb), as the
//!   traffic flies them by their goal.
//!
//! The goals the idle block's [`kept`](super::kept) keeps already (a
//! stellar, a gate, a jump out, a hold, a wait, a drift) stay its own. A
//! fight on a ship gone from the system is not kept: the think idles,
//! standing for the high level's drop that frame (goal 4 or 3 with target
//! -1, @0x8e263 → @0x8e731, @0x8dff6 → @0x8e228), and a brave trader reads
//! a gone attacker it still names ([`fight_on_gone`](crate::traffic::provocation::fight_on_gone)).
//! Nor is an inspection (goal 7) kept: the original's state-7 block
//! (@0x8f11e-0x8f391), its end within reach and its scan of the player, is
//! type-free, but here it is the interceptor's think's
//! ([`inspection_holds`](super::interceptor::inspection_holds)), so a
//! trader keeping it would never end it: filed as
//! rdm:phase/combat-ai-fidelity-cross-roadmap/phase-18-left-out-of-phase-16.
//! A fighter's return (goal 5) is never copied: no ship leads a fighter.
//!
//! **Goal 4's high level** ([`high_level`]): a kept attack or snipe, or a
//! brave trader's own attack, is flown as `_HighLevelAIHandler`'s goal-4
//! block takes it that frame. An attack on a ship breaking up ends
//! ([`ends_attack`], @0x8e3ef → @0x8e714: goal 0, target -1, +0x96 0), and
//! a hopeless chase is fled, as a trader is of AI type 2 or below
//! ([`state4_attack`], @0x8e60a-0x8e62e). The think's calls read the goal
//! it wrote, before the high level. Goal 4's ally and fleet drops
//! (@0x8e277-0x8e399) never meet a kept attack: a copied target was the
//! lead's own, under the same government and fleet, so the lead's high
//! level spared it already; a bribed follower never takes over
//! ([`succession`](crate::traffic::succession)); and the player's
//! government is never allied.
//!
//! Nothing here plays a sound or draws a random number.

use crate::ai::odds::state4_attack;
use crate::ai::{Goal, Surroundings, idle};
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::traffic::npc::Npc;

/// The goal trader `npc`'s think keeps among `around`, unprovoked, of
/// those its idle block's [`kept`](super::kept) doesn't (see the module
/// docs): a plunder, a count wait, a mine, a scoop or a stellar attack,
/// and an attack, snipe or flight on a ship still in the system. None for
/// any other goal.
#[must_use]
pub fn kept(npc: &Npc, around: &Surroundings) -> Option<Goal> {
    match npc.goal {
        Goal::Plunder { .. }
        | Goal::CountWait
        | Goal::Mine
        | Goal::Scoop
        | Goal::AttackStellar(_) => Some(npc.goal),
        Goal::Attack(ship) | Goal::Snipe(ship) | Goal::Flee(ship)
            if around.state_of(ship).is_some() =>
        {
            Some(npc.goal)
        }
        _ => None,
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

/// Whether goal 4's high level ends `thought`, a trader's think, among
/// `around`: an attack or snipe on a ship breaking up (`_IsDying`
/// @0x8e3ef → @0x8e714-0x8e726: goal 0, target -1, +0x96 0).
#[must_use]
pub fn ends_attack(thought: Goal, around: &Surroundings) -> bool {
    thought
        .attacking()
        .is_some_and(|ship| matches!(around.condition_of(ship), Some(Condition::Dying { .. })))
}

/// The goal trader `npc` flies once goal 4's high level has taken
/// `thought`, its think's, among `around`, the attack not ended
/// ([`ends_attack`]): an attack or snipe is [`state4_attack`]'s, a
/// hopeless chase fled (goal 3, sub 5, @0x8e622-0x8e62e); any other goal
/// stands.
#[must_use]
pub fn high_level(npc: &Npc, thought: Goal, around: &Surroundings) -> Goal {
    thought
        .attacking()
        .map_or(thought, |ship| state4_attack(npc, ship, around))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, TRADERS, around, govts, n, ship, sites};
    use crate::catalog::StellarId;
    use crate::combat::ShipRef;
    use crate::geometry::Vec2;
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

    #[test]
    fn a_fight_on_a_ship_in_the_system_is_kept_and_one_on_a_ship_gone_is_not() {
        for goal in [Goal::Attack(n(2)), Goal::Snipe(n(2)), Goal::Flee(n(2))] {
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
            let npcs = holding(goal, Condition::Intact);
            assert_eq!(kept_among(&npcs[..1]), None, "{goal:?}: gone");
        }
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
    fn an_inspection_an_idle_or_the_idle_blocks_own_goals_are_not_kept_here() {
        for goal in [
            Goal::Inspect(n(2)),
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

    #[test]
    fn goal_4s_high_level_ends_an_attack_on_a_ship_breaking_up() {
        let govts = govts(0, 0);
        let sites = sites();
        let dying = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        for goal in [Goal::Attack(n(2)), Goal::Snipe(n(2))] {
            let npcs = holding(goal, dying);
            let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
            assert!(ends_attack(goal, &around), "{goal:?}");
            let npcs = holding(goal, Condition::Disabled);
            let around = crate::ai::fixture::around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
            assert!(!ends_attack(goal, &around), "{goal:?}: disabled");
        }
        let npcs = holding(Goal::Flee(n(2)), dying);
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        assert!(!ends_attack(Goal::Flee(n(2)), &around), "no attack");
    }

    #[test]
    fn goal_4s_high_level_flees_a_traders_hopeless_chase_and_keeps_any_other_goal() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut npcs = holding(Goal::Snipe(n(2)), Condition::Intact);
        npcs[0].hull.mass = 100.0;
        npcs[1].goal = Goal::Flee(n(1));
        npcs[1].state.velocity = Vec2::new(0.0, 1.0);
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        for goal in [Goal::Attack(n(2)), Goal::Snipe(n(2))] {
            assert_eq!(
                high_level(&npcs[0], goal, &around),
                Goal::Flee(n(2)),
                "{goal:?}"
            );
        }
        npcs[0].hull.mass = 10.0;
        let around = crate::ai::fixture::around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        assert_eq!(
            high_level(&npcs[0], Goal::Snipe(n(2)), &around),
            Goal::Attack(n(2)),
            "a chase that isn't hopeless is attacked"
        );
        for goal in [
            Goal::Flee(n(2)),
            Goal::CountWait,
            Goal::Plunder {
                ship: ShipRef::Player,
                hopeless: false,
            },
        ] {
            assert_eq!(high_level(&npcs[0], goal, &around), goal, "{goal:?}");
        }
    }
}
