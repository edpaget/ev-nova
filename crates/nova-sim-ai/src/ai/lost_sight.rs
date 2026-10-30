//! The lost-sight search: what a fighting ship does when it can no longer
//! see its target (`_HighLevelAIHandler` @0x8d6ef-0x8d8ae in the `EV Nova`
//! executable), and an interceptor's drop of a ship it inspects but can no
//! longer see (`_InterceptorAI` @0x8d39a-0x8d3ee).
//!
//! # `_HighLevelAIHandler`'s block ([`after_think`])
//!
//! It runs after the type's think (`_AIDispatch` @0x901e5) and the
//! `ShipBehav` 0 mission hunt (@0x8d628-0x8d6e9), on the goal they leave:
//! an attack or snipe (state 4) or a plunder (0xd), with a target. Like
//! the rest of the handler it is skipped while the ship's AI wait runs
//! ([`Npc::ai_wait`], @0x8d4b9-0x8d4d2). It asks whether the ship sees
//! its target ([`Surroundings::sees`], `_ShipVisibleToShip` @0x8d738).
//!
//! - Seen, the search timer (+0xc90c, [`Npc::lost_sight`]) is cleared
//!   (@0x8d8ab-0x8d8ae) and the handler goes on.
//! - Unseen by a trader (AI type +0x88 of 2 or below, @0x8d748,
//!   [`Npc::original_ai_type`]: not a led ship or a fighter, but one the
//!   player has paid off), it
//!   leaves by hyperspace (state 2, target -1, @0x8d7cd-0x8d7fe) if it can
//!   jump ([`leave::can_jump`], the inlined `_AIShipHasFuelForJump`
//!   @0x8d756-0x8d7c9), and otherwise stops and waits (state 6,
//!   @0x8d803-0x8d80f), keeping its target ([`trader_target`]), which the
//!   threat tests then read ([`Npc::fought`]). The timer is not touched. The leave's +0x50 clamp
//!   and +0xa4 stamp are the warp stage's
//!   (rdm:phase/ship-handling-fidelity/phase-8-warp-out), as the mission
//!   hunt's give-up's are.
//! - Unseen by any other ship (@0x8d814), it stops (sub 1, +0x50 0) and
//!   searches, its travel stellar cleared on each frame (+0x6c -1,
//!   @0x8d833, [`TravelWrite::Clear`]; see [`travel`](crate::ai::travel)):
//!   an unset timer is set to `Rand(100)+100` (@0x8d83b-0x8d852),
//!   a set one counts down a tick (@0x8d85f-0x8d876), and one run out drops
//!   the target (target -1, state and sub 0, timer -1, @0x8d884-0x8d8a6).
//!   A timer the ship took from its slot ([`SearchTimer::Carried`]: no
//!   spawn writes +0xc90c, see
//!   [`berth_search`](crate::traffic::berth_search)) counts down the same
//!   way with no roll, so a never-used slot's 0 drops the target on the
//!   first tick out of sight.
//!   Each path ends the handler for the frame (the jump to @0x8fb47,
//!   [`ends_high_level`]), so the state-4 block's call for reinforcements
//!   and the flee drop do not run; `_LowLevelAIHandler` still flies sub
//!   1, a stop (see [`searching`]). It holds N = 100..=199 ticks: set on
//!   tick 0, N-k after tick k, dropped on tick N.
//!
//! The type's other words in its think (its turn on the player, its call
//! on the message line, its own call for reinforcements) stand: the think
//! ran before the block.
//!
//! # `_InterceptorAI`'s lost-sight tail ([`after_inspection`])
//!
//! An interceptor inspecting a ship (state 7) it no longer sees drops it:
//! target and nav -1, state and sub 0 (@0x8d39a-0x8d3ee). It comes after
//! the out-of-ammo retreat (@0x8d343-0x8d391), so an inspection that
//! retreat turned into a flight is left alone.
//!
//! # Not modelled
//!
//! - What a hit by an NPC does to an inspection: the original writes the
//!   attacker into +0x70 (`_DamageShip` @0x3bcf6), so the interceptor
//!   inspects and re-picks over its attacker (`_InterceptorAI` spares
//!   state 7 its provocation, @0x8cc0d), where the engine's turns on it:
//!   rdm:phase/combat-ai-fidelity/phase-27-left-out-of-phase-21.
//! - The warp-stage writes of the trader's leave (+0x50, +0xa4), the
//!   warp stage's (rdm:phase/ship-handling-fidelity/phase-8-warp-out).
//! - The NPC target picks' own sight of a cloaked ship
//!   (rdm:phase/cloaking-fidelity/phase-4-npc-cloak-visibility): until
//!   then a warship that gives up its search re-picks the cloaked ship on
//!   its next think and searches again.

use crate::ai::travel::TravelWrite;
use crate::ai::{Decision, Goal, Surroundings, leave};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::traffic::npc::Npc;

/// The fewest ticks a search lasts (`Rand(100)+100`, @0x8d84b).
pub const SEARCH_TICKS: u32 = 100;

/// A ship's search timer, the original's +0xc90c ([`Npc::lost_sight`]).
///
/// Only the lost-sight block writes it, and no spawn does: a ship takes the
/// timer its slot's last ship left (see
/// [`berth_search`](crate::traffic::berth_search)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchTimer {
    /// No timer, the original's -1: a loss of sight rolls a search.
    #[default]
    Unset,
    /// A count the ship took from its slot without searching: a never-used
    /// slot's 0 (the zero fill of `_NewPtr`, `_InitArrays` @0x1b7a1), or
    /// the ticks left of the search its last ship broke off. A loss of
    /// sight counts it down with no roll, so 0 gives up at once.
    Carried(u32),
    /// A search under way, with this many ticks left.
    Searching(u32),
}

impl SearchTimer {
    /// The timer a new ship brought into the slot holds: the same count,
    /// but no search under way.
    #[must_use]
    pub fn carried(self) -> Self {
        match self {
            Self::Searching(left) => Self::Carried(left),
            timer => timer,
        }
    }
}

/// The lost-sight block's write of the search timer +0xc90c, when it ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LostSight {
    /// The target is seen: the timer is cleared (-1, @0x8d8ae).
    Seen,
    /// The target is unseen: the search goes on, with this many ticks
    /// left after this one.
    Searching(u32),
    /// The search ran out: the target is dropped and the timer cleared
    /// (-1, @0x8d89c).
    GaveUp,
}

/// `decision`, `npc`'s type's think, through `_HighLevelAIHandler`'s
/// lost-sight block among `around`, its search's length rolled on
/// `chance` (@0x8d6ef-0x8d8ae; see the module docs).
#[must_use]
pub fn after_think(
    npc: &Npc,
    decision: Decision,
    around: &Surroundings,
    chance: &mut dyn Chance,
) -> Decision {
    if npc.ai_wait > 0 {
        return decision;
    }
    let Some(target) = decision
        .goal
        .attacking()
        .or_else(|| decision.goal.plundering())
    else {
        return decision;
    };
    if around.sees(npc, target) {
        return Decision {
            lost_sight: Some(LostSight::Seen),
            ..decision
        };
    }
    if npc.original_ai_type() <= 2 {
        return if leave::can_jump(npc, around.jump_fuel) {
            Decision {
                goal: Goal::JumpOut,
                clears_target: true,
                ..decision
            }
        } else {
            Decision {
                goal: Goal::Wait,
                ..decision
            }
        };
    }
    // Each frame of the search writes +0x6c = -1 (@0x8d833).
    match npc.lost_sight {
        SearchTimer::Unset => Decision {
            lost_sight: Some(LostSight::Searching(
                SEARCH_TICKS + chance.below(SEARCH_TICKS),
            )),
            travel: TravelWrite::Clear,
            ..decision
        },
        SearchTimer::Carried(left) | SearchTimer::Searching(left) if left > 1 => Decision {
            lost_sight: Some(LostSight::Searching(left - 1)),
            travel: TravelWrite::Clear,
            ..decision
        },
        SearchTimer::Carried(_) | SearchTimer::Searching(_) => Decision {
            goal: Goal::Idle,
            clears_target: true,
            lost_sight: Some(LostSight::GaveUp),
            travel: TravelWrite::Clear,
            ..decision
        },
    }
}

/// Whether the block ended the high level this think: searching or
/// given up (the jump to @0x8fb47).
#[must_use]
pub fn ends_high_level(decision: &Decision) -> bool {
    matches!(
        decision.lost_sight,
        Some(LostSight::Searching(_) | LostSight::GaveUp)
    )
}

/// Whether `npc` searches now: its goal an attack, snipe or plunder
/// (state 4 or 0xd) and a search under way. It flies sub 1, a stop. A
/// timer carried from its slot is no search: the original's ship flies
/// its fight until a loss of sight runs the block.
#[must_use]
pub fn searching(npc: &Npc) -> bool {
    matches!(
        npc.goal,
        Goal::Attack(_) | Goal::Snipe(_) | Goal::Plunder { .. }
    ) && matches!(npc.lost_sight, SearchTimer::Searching(_))
}

/// The ship a trader (AI type 2 or below) targets among `around`: its
/// goal's quarry, but stopped ([`Goal::Wait`], state 6) the target it
/// holds, which the lost-sight stop keeps (@0x8d803-0x8d80f) and the
/// idle blocks' stop never writes (`_WimpyTraderAI` @0x8b1a3,
/// `_BraveTraderAI` @0x8b60d), until its slot is gone
/// (`_HighLevelAIHandler` @0x8d4e4-0x8d50d: one breaking up is still
/// active). A stopped trader fires nothing at it (state 6 flies sub 1,
/// @0x8f101), but the threat tests read it
/// ([`Npc::threatens_player_or_led`], [`Npc::threatens_npc_among`],
/// [`Npc::threatens_player_extended`]).
///
/// A stop keeps the trader's provocation as well (see
/// [`provocation`](crate::traffic::provocation)), and
/// [`provoked_by`](crate::ai::provoked_by) reads this target, so a stopped
/// trader with a kept target and provocation turns on it again, as the
/// original's provoked branch does from state 6 (@0x8b1ac, @0x8b616).
#[must_use]
pub fn trader_target(npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
    match npc.goal {
        Goal::Wait => npc
            .target
            .filter(|&ship| around.condition_of(ship).is_some_and(Condition::active)),
        goal => goal.quarry(),
    }
}

/// `decision`, an interceptor's think, through `_InterceptorAI`'s
/// lost-sight tail among `around` (@0x8d39a-0x8d3ee): an inspection of a
/// ship `npc` doesn't see is dropped.
#[must_use]
pub fn after_inspection(npc: &Npc, decision: Decision, around: &Surroundings) -> Decision {
    match decision.goal {
        Goal::Inspect(ship) if !around.sees(npc, ship) => Decision {
            goal: Goal::Idle,
            clears_target: true,
            ..decision
        },
        _ => decision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{TRADERS, around, cloaked, govts, n, ship, sites};
    use crate::bay::Carrier;
    use crate::combat::ShipRef;
    use crate::rulebook::RuleSource;
    use crate::traffic::npc::{AiType, NpcId};
    use nova_sim_base::fixture::Draws;

    /// NPC 1 of `ai_type` at the centre, its timer `timer`.
    fn hunter(ai_type: AiType, timer: SearchTimer) -> Npc {
        let mut npc = ship(1, TRADERS, ai_type, 0.0, 0.0);
        npc.lost_sight = timer;
        npc
    }

    /// NPC 2, a trader 300 off, cloaked as `hidden` says.
    fn quarry(hidden: bool) -> Npc {
        let npc = ship(2, TRADERS, AiType::WimpyTrader, 300.0, 0.0);
        if hidden { cloaked(npc) } else { npc }
    }

    fn of(goal: Goal) -> Decision {
        Decision {
            goal,
            ..Decision::default()
        }
    }

    /// `decision` of `npc` through the block with `quarry`, drawing
    /// `chance`, the jump fuel read as `jump_fuel`.
    fn through(
        npc: &Npc,
        quarry: Npc,
        decision: Decision,
        jump_fuel: RuleSource,
        chance: &mut Draws,
    ) -> Decision {
        let (sites, govts) = (sites(), govts(0, 0));
        let npcs = [npc.clone(), quarry];
        let around = Surroundings {
            jump_fuel,
            ..around(&sites, &npcs, &govts, (0.0, 2000.0), 0)
        };
        after_think(&npcs[0], decision, &around, chance)
    }

    fn lost(npc: &Npc, decision: Decision, chance: &mut Draws) -> Decision {
        through(npc, quarry(true), decision, RuleSource::Engine, chance)
    }

    #[test]
    fn a_warship_that_loses_sight_starts_a_100_to_199_tick_search() {
        let warship = hunter(AiType::Warship, SearchTimer::Unset);
        let mut low = Draws::of(&[0]);
        let decision = lost(&warship, of(Goal::Attack(n(2))), &mut low);
        assert_eq!(decision.lost_sight, Some(LostSight::Searching(100)));
        assert_eq!(decision.goal, Goal::Attack(n(2)), "it holds its attack");
        assert!(!decision.clears_target);
        assert_eq!(decision.travel, TravelWrite::Clear, "+0x6c -1, @0x8d833");
        assert_eq!(low.asked, [100]);
        let mut high = Draws::of(&[99]);
        let decision = lost(&warship, of(Goal::Attack(n(2))), &mut high);
        assert_eq!(decision.lost_sight, Some(LostSight::Searching(199)));
    }

    #[test]
    fn a_search_under_way_counts_down_without_a_draw() {
        let warship = hunter(AiType::Warship, SearchTimer::Searching(5));
        let mut chance = Draws::of(&[]);
        let decision = lost(&warship, of(Goal::Attack(n(2))), &mut chance);
        assert_eq!(decision.lost_sight, Some(LostSight::Searching(4)));
        assert_eq!(decision.goal, Goal::Attack(n(2)));
        assert_eq!(decision.travel, TravelWrite::Clear);
        assert!(chance.asked.is_empty());
        let warship = hunter(AiType::Warship, SearchTimer::Searching(2));
        let decision = lost(&warship, of(Goal::Attack(n(2))), &mut chance);
        assert_eq!(decision.lost_sight, Some(LostSight::Searching(1)));
    }

    #[test]
    fn the_search_gives_up_when_the_timer_runs_out() {
        for timer in [1, 0] {
            let warship = hunter(AiType::Warship, SearchTimer::Searching(timer));
            let mut chance = Draws::of(&[]);
            let decision = lost(&warship, of(Goal::Attack(n(2))), &mut chance);
            assert_eq!(decision.goal, Goal::Idle, "timer {timer}");
            assert!(decision.clears_target);
            assert_eq!(decision.lost_sight, Some(LostSight::GaveUp));
            assert_eq!(decision.travel, TravelWrite::Clear);
            assert!(chance.asked.is_empty());
        }
    }

    #[test]
    fn a_target_seen_again_clears_the_search() {
        let warship = hunter(AiType::Warship, SearchTimer::Searching(50));
        let mut chance = Draws::of(&[]);
        let decision = through(
            &warship,
            quarry(false),
            of(Goal::Attack(n(2))),
            RuleSource::Engine,
            &mut chance,
        );
        assert_eq!(decision.lost_sight, Some(LostSight::Seen));
        assert_eq!(decision.goal, Goal::Attack(n(2)));
        assert!(!decision.clears_target);
        let player = through(
            &warship,
            quarry(true),
            of(Goal::Attack(ShipRef::Player)),
            RuleSource::Engine,
            &mut chance,
        );
        assert_eq!(player.lost_sight, Some(LostSight::Seen), "the player");
    }

    #[test]
    fn snipes_and_plunders_search_too_and_flights_and_inspections_do_not() {
        let warship = hunter(AiType::Warship, SearchTimer::Searching(5));
        let plunder = Goal::Plunder {
            ship: n(2),
            hopeless: true,
        };
        for goal in [Goal::Snipe(n(2)), plunder] {
            let decision = lost(&warship, of(goal), &mut Draws::of(&[]));
            assert_eq!(
                decision.lost_sight,
                Some(LostSight::Searching(4)),
                "{goal:?}"
            );
            assert_eq!(decision.goal, goal);
        }
        for goal in [Goal::Flee(n(2)), Goal::Inspect(n(2)), Goal::Idle] {
            let decision = lost(&warship, of(goal), &mut Draws::of(&[]));
            assert_eq!(decision, of(goal), "{goal:?}");
        }
    }

    #[test]
    fn a_ship_whose_ai_wait_runs_skips_the_block() {
        let mut warship = hunter(AiType::Warship, SearchTimer::Searching(5));
        warship.ai_wait = 3;
        let decision = lost(&warship, of(Goal::Attack(n(2))), &mut Draws::of(&[]));
        assert_eq!(decision, of(Goal::Attack(n(2))), "@0x8d4b9-0x8d4d2");
    }

    #[test]
    fn the_think_s_other_words_stand_through_the_search() {
        let warship = hunter(AiType::Warship, SearchTimer::Searching(5));
        let decision = Decision {
            turns_on_player: true,
            calls_reinforcements: true,
            comm_call: Some(crate::ai::CommCall::PreScan),
            ..of(Goal::Attack(n(2)))
        };
        let searched = lost(&warship, decision, &mut Draws::of(&[]));
        assert_eq!(
            searched,
            Decision {
                lost_sight: Some(LostSight::Searching(4)),
                travel: TravelWrite::Clear,
                ..decision
            }
        );
    }

    #[test]
    fn a_trader_that_loses_sight_leaves_when_it_can_jump_and_waits_when_it_cannot() {
        for ai_type in [AiType::WimpyTrader, AiType::BraveTrader] {
            let mut trader = hunter(ai_type, SearchTimer::Searching(5));
            trader.class_fuel = 100;
            let mut chance = Draws::of(&[]);
            let decision = lost(&trader, of(Goal::Attack(n(2))), &mut chance);
            assert_eq!(decision.goal, Goal::JumpOut, "{ai_type:?}");
            assert!(decision.clears_target);
            assert_eq!(decision.lost_sight, None, "the timer is untouched");
            assert_eq!(decision.travel, TravelWrite::Keep, "no +0x6c write");
            assert!(chance.asked.is_empty());
            trader.class_fuel = 99;
            let decision = lost(&trader, of(Goal::Attack(n(2))), &mut chance);
            assert_eq!(decision.goal, Goal::Wait, "{ai_type:?} at the floor");
            assert!(!decision.clears_target, "state 6 keeps its target");
            assert_eq!(decision.lost_sight, None);
            assert_eq!(decision.travel, TravelWrite::Keep);
            trader.class_fuel = 100;
            trader.reserves.fuel.now = 0.0;
            let bible = through(
                &trader,
                quarry(true),
                of(Goal::Attack(n(2))),
                RuleSource::Bible,
                &mut chance,
            );
            assert_eq!(bible.goal, Goal::Wait, "{ai_type:?} with an empty tank");
            let engine = lost(&trader, of(Goal::Attack(n(2))), &mut chance);
            assert_eq!(engine.goal, Goal::JumpOut, "the engine's jump fuel");
        }
    }

    #[test]
    fn a_trader_type_the_player_leads_or_a_carrier_launched_searches_instead() {
        let mut led = hunter(AiType::WimpyTrader, SearchTimer::Unset);
        led.leader = Some(NpcId(3));
        let decision = lost(&led, of(Goal::Attack(n(2))), &mut Draws::of(&[0]));
        assert_eq!(decision.lost_sight, Some(LostSight::Searching(100)), "led");
        let mut fighter = hunter(AiType::BraveTrader, SearchTimer::Unset);
        fighter.carrier = Some(Carrier {
            ship: n(3),
            window: 100.0,
            reach: 40.0,
        });
        let decision = lost(&fighter, of(Goal::Attack(n(2))), &mut Draws::of(&[0]));
        assert_eq!(
            decision.lost_sight,
            Some(LostSight::Searching(100)),
            "carried"
        );
        let interceptor = hunter(AiType::Interceptor, SearchTimer::Unset);
        let decision = lost(&interceptor, of(Goal::Attack(n(2))), &mut Draws::of(&[0]));
        assert_eq!(decision.lost_sight, Some(LostSight::Searching(100)));
    }

    #[test]
    fn a_led_ship_paid_off_leaves_as_a_trader() {
        // The bribe writes AI type 1 (@0x961f8), parent kept (@0x8d748).
        let mut spared = hunter(AiType::Warship, SearchTimer::Unset);
        spared.leader = Some(NpcId(3));
        spared.spared = true;
        spared.class_fuel = 100;
        let mut chance = Draws::of(&[]);
        let decision = lost(&spared, of(Goal::Attack(n(2))), &mut chance);
        assert_eq!(decision.goal, Goal::JumpOut);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn only_a_search_or_its_give_up_ends_the_high_level() {
        assert!(!ends_high_level(&of(Goal::Attack(n(2)))));
        let with = |lost_sight| Decision {
            lost_sight: Some(lost_sight),
            ..of(Goal::Attack(n(2)))
        };
        assert!(!ends_high_level(&with(LostSight::Seen)));
        assert!(ends_high_level(&with(LostSight::Searching(4))));
        assert!(ends_high_level(&with(LostSight::GaveUp)));
    }

    #[test]
    fn searching_holds_only_in_state_4_or_0xd_with_the_timer_set() {
        let mut npc = hunter(AiType::Warship, SearchTimer::Searching(3));
        let plunder = Goal::Plunder {
            ship: n(2),
            hopeless: false,
        };
        for goal in [Goal::Attack(n(2)), Goal::Snipe(n(2)), plunder] {
            npc.goal = goal;
            assert!(searching(&npc), "{goal:?}");
        }
        for goal in [
            Goal::Flee(n(2)),
            Goal::Inspect(n(2)),
            Goal::Idle,
            Goal::Wait,
        ] {
            npc.goal = goal;
            assert!(!searching(&npc), "{goal:?}");
        }
        npc.goal = Goal::Attack(n(2));
        npc.lost_sight = SearchTimer::Unset;
        assert!(!searching(&npc), "no timer");
    }

    #[test]
    fn a_carried_timer_counts_down_as_a_search_does() {
        let warship = hunter(AiType::Warship, SearchTimer::Carried(5));
        let mut chance = Draws::of(&[]);
        let decision = lost(&warship, of(Goal::Attack(n(2))), &mut chance);
        assert_eq!(decision.lost_sight, Some(LostSight::Searching(4)));
        assert_eq!(decision.goal, Goal::Attack(n(2)));
        assert_eq!(decision.travel, TravelWrite::Clear);
        assert!(chance.asked.is_empty(), "no roll");
    }

    #[test]
    fn a_fresh_slots_zero_timer_gives_up_at_once() {
        // A never-used slot's +0xc90c reads 0.0 (`_NewPtr` @0x1b7a1), not
        // below 0 at @0x8d830: counted down, it drops the target.
        let warship = hunter(AiType::Warship, SearchTimer::Carried(0));
        let mut chance = Draws::of(&[]);
        let decision = lost(&warship, of(Goal::Attack(n(2))), &mut chance);
        assert_eq!(decision.goal, Goal::Idle);
        assert!(decision.clears_target);
        assert_eq!(decision.lost_sight, Some(LostSight::GaveUp));
        assert!(chance.asked.is_empty(), "no roll");
    }

    #[test]
    fn a_carried_timer_is_no_search() {
        let mut npc = hunter(AiType::Warship, SearchTimer::Carried(30));
        npc.goal = Goal::Attack(n(2));
        assert!(!searching(&npc), "a carried timer");
        npc.lost_sight = SearchTimer::Searching(30);
        assert!(searching(&npc));
    }

    #[test]
    fn a_timer_carried_to_a_new_ship_is_no_search_under_way() {
        assert_eq!(
            SearchTimer::Searching(40).carried(),
            SearchTimer::Carried(40)
        );
        assert_eq!(SearchTimer::Carried(7).carried(), SearchTimer::Carried(7));
        assert_eq!(SearchTimer::Unset.carried(), SearchTimer::Unset);
        assert_eq!(SearchTimer::default(), SearchTimer::Unset);
    }

    /// `decision` of an interceptor at the centre through its tail, with
    /// `quarry`.
    fn inspected(quarry: Npc, decision: Decision) -> Decision {
        let (sites, govts) = (sites(), govts(0, 0));
        let npcs = [hunter(AiType::Interceptor, SearchTimer::Unset), quarry];
        let around = around(&sites, &npcs, &govts, (0.0, 2000.0), 0);
        after_inspection(&npcs[0], decision, &around)
    }

    #[test]
    fn an_inspection_of_a_ship_out_of_sight_is_dropped() {
        let dropped = inspected(quarry(true), of(Goal::Inspect(n(2))));
        assert_eq!(dropped.goal, Goal::Idle);
        assert!(dropped.clears_target);
        let kept = inspected(quarry(false), of(Goal::Inspect(n(2))));
        assert_eq!(kept, of(Goal::Inspect(n(2))));
        let fled = inspected(quarry(true), of(Goal::Flee(n(2))));
        assert_eq!(fled, of(Goal::Flee(n(2))), "a flight is left alone");
    }

    #[test]
    fn a_ship_attacking_a_cloaked_player_it_cannot_see_searches_for_it() {
        let search = |npc: Npc, at: (f32, f32)| {
            let (sites, govts) = (sites(), govts(0, 0));
            let npcs = [npc];
            let mut around = around(&sites, &npcs, &govts, at, 0);
            around.player = around.player.map(crate::ai::fixture::cloaked_player);
            let mut chance = Draws::of(&[0]);
            after_think(
                &npcs[0],
                of(Goal::Attack(ShipRef::Player)),
                &around,
                &mut chance,
            )
        };
        let warship = hunter(AiType::Warship, SearchTimer::Unset);
        let unseen = search(warship.clone(), (0.0, 2000.0));
        assert_eq!(
            unseen.lost_sight,
            Some(LostSight::Searching(100)),
            "@0x8d738"
        );
        assert_eq!(unseen.goal, Goal::Attack(ShipRef::Player));
        let scanned = search(crate::ai::fixture::scanning(warship), (0.0, 150.0));
        assert_eq!(
            scanned.lost_sight,
            Some(LostSight::Seen),
            "scanned within 200"
        );
    }
}
