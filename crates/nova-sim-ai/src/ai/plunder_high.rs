//! Goal 0xd's high level for any ship that holds it
//! (`_HighLevelAIHandler` @0x8e740-0x8eb6b in the `EV Nova` executable),
//! as the engine folds it after the think: [`fold`].
//!
//! `_AIDispatch` runs the high level after every type's think (@0x901e5),
//! so a plunder a pirate's think wrote and one a trader or interceptor
//! keeps (a lost lead's, copied by the succession) are flown alike. The
//! goal-0xd block reads the ship +0x70 names:
//!
//! - **Under its AI wait** the handler returns first (@0x8d4b9-0x8d4d2),
//!   so nothing is written: the plunder stays unsniped, its +0x6c kept,
//!   whatever sub the wait started with (none of the writers that start a
//!   wait leaves sub 0xe: the dock's sub 0xf, @0x86fc7; a spawn's or the
//!   emerge's sub 0, @0x8953b; a hit only cuts a running wait,
//!   @0x3be6e-0x3be7c).
//! - **Its ship gone**, its target cleared by the slot test (@0x8d4e4),
//!   the block writes goal 0 alone (@0x8e761 → @0x8eb5f): the traffic's
//!   [`HighLevel::LetsGo`](crate::traffic::plunder::HighLevel::LetsGo).
//!   Here the plunder is kept for it, its +0x6c too.
//! - **Its ship disabled**, it docks (subs 0xf, 0xb or 9, the traffic's
//!   [`plunder::high_level`](crate::traffic::plunder::high_level)), its
//!   +0x6c the ship's slot (@0x8e8c9), unsniped.
//! - **Its ship intact**, +0x6c -1 (@0x8ea50). Outside [`DOGFIGHT_BOX`]
//!   on either axis, `_AIEvalHopelessChase` decides (@0x8eace): a hopeless
//!   chase snipes (sub 0xe) for AI type above 2 and flees (goal 3, sub 5)
//!   for 2 or below (`cmpw $0x2, 0x88` @0x8ead7-0x8eafb), a trader that
//!   keeps a copied plunder, then jumps past every call (@0x8eb6b): that
//!   frame's sub 5 flies flee close whatever the reach
//!   ([`hopeless_flee`](super::hopeless_flee)). Otherwise it approaches or
//!   dogfights, the pursuit's.
//!
//! The fleet drops (@0x8e771-0x8e838) need a leader (+0x9a set): a
//! plunderer has none, as a led ship thinks as `FleetEscortAi`, which
//! never holds goal 0xd. The block plays no sound and draws nothing.

use crate::ai::fire::DOGFIGHT_BOX;
use crate::ai::hopeless_flee::HopelessFlee;
use crate::ai::odds::hopeless_chase;
use crate::ai::travel::{self, TravelWrite};
use crate::ai::{Decision, Goal, Surroundings};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::traffic::npc::Npc;

/// Goal 0xd's high level for `npc` plundering `ship` among `around` (see
/// the module docs): the goal it flies, and how it writes its travel
/// stellar.
#[must_use]
pub fn fold(npc: &Npc, ship: ShipRef, around: &Surroundings) -> (Goal, TravelWrite) {
    let plunder = Goal::Plunder {
        ship,
        hopeless: false,
    };
    if npc.high_level_waits_under(plunder) || around.state_of(ship).is_none() {
        return (plunder, TravelWrite::Keep);
    }
    if around.condition_of(ship).is_some_and(Condition::disabled) {
        let slot = travel::slot_of(ship, around).map_or(TravelWrite::Clear, TravelWrite::Set);
        return (plunder, slot);
    }
    let goal = if outside_box(npc, ship, around) && hopeless_chase(npc, ship, around) {
        if npc.original_ai_type() > 2 {
            Goal::Plunder {
                ship,
                hopeless: true,
            }
        } else {
            Goal::Flee(ship)
        }
    } else {
        plunder
    };
    (goal, TravelWrite::Clear)
}

/// `decision`, `npc`'s think among `around`, with goal 0xd's high level
/// folded in when it holds a plunder ([`fold`]): its goal the fold's, and
/// its travel stellar the fold's write over the think's, the think's when
/// the fold writes none; a flight the fold wrote is goal 0xd's first frame
/// of a hopeless flee ([`HopelessFlee::Plunder`]), which makes no call.
/// Any other decision stands.
#[must_use]
pub fn after_think(npc: &Npc, decision: Decision, around: &Surroundings) -> Decision {
    let Goal::Plunder { ship, .. } = decision.goal else {
        return decision;
    };
    let (goal, travel) = fold(npc, ship, around);
    Decision {
        goal,
        travel: match travel {
            TravelWrite::Keep => decision.travel,
            write => write,
        },
        // The fold flees only a hopeless chase (@0x8eaef-0x8eafb).
        hopeless_flee: matches!(goal, Goal::Flee(_)).then_some(HopelessFlee::Plunder),
        ..decision
    }
}

/// Whether `target` is outside [`DOGFIGHT_BOX`] of `npc` on either axis,
/// by float abs (`_HighLevelAIHandler` @0x8ea49-0x8eac6); a ship not in
/// the system is not.
fn outside_box(npc: &Npc, target: ShipRef, around: &Surroundings) -> bool {
    around.state_of(target).is_some_and(|at| {
        let off = at.position - npc.state.position;
        off.x.abs() > DOGFIGHT_BOX || off.y.abs() > DOGFIGHT_BOX
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, TRADERS, around, govts, n, ship, sites};
    use crate::ai::hopeless_flee::HopelessFlee;
    use crate::ai::{Behaviour, CommCall, NovaAi};
    use crate::catalog::StellarId;
    use crate::geometry::Vec2;
    use crate::traffic::npc::{AiType, NpcId};
    use nova_sim_base::fixture::Draws;

    /// A heavy plunderer (NPC 1, berth 1) of `ai_type` at the centre,
    /// holding a plunder of the trader (NPC 2, berth 4) at (0, `y`),
    /// which opens the gap downwards at 1 a tick when `opening`.
    fn scene(ai_type: AiType, y: f32, opening: bool) -> [Npc; 2] {
        let mut plunderer = ship(1, PIRATES, ai_type, 0.0, 0.0);
        plunderer.berth = 1;
        plunderer.hull.mass = 100.0;
        plunderer.goal = Goal::Plunder {
            ship: n(2),
            hopeless: false,
        };
        plunderer.target = Some(n(2));
        let mut trader = ship(2, TRADERS, AiType::WimpyTrader, 0.0, y);
        trader.berth = 4;
        trader.hull.crew = 1;
        trader.hull.inherent_ai = 1;
        if opening {
            trader.goal = Goal::Flee(n(1));
            trader.state.velocity = Vec2::new(0.0, 1.0);
        }
        [plunderer, trader]
    }

    fn folded(npcs: &[Npc]) -> (Goal, TravelWrite) {
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -1000.0), 0);
        fold(&npcs[0], n(2), &around)
    }

    const PLAIN: Goal = Goal::Plunder {
        ship: ShipRef::Npc(NpcId(2)),
        hopeless: false,
    };

    #[test]
    fn an_intact_ships_plunder_clears_the_travel_stellar_and_snipes_or_flees_a_hopeless_chase() {
        // +0x6c -1 (@0x8ea50); sub 0xe above AI type 2, goal 3 sub 5 at
        // or below (@0x8ead7-0x8eafb).
        assert_eq!(
            folded(&scene(AiType::Warship, 300.0, true)),
            (
                Goal::Plunder {
                    ship: n(2),
                    hopeless: true
                },
                TravelWrite::Clear
            )
        );
        assert_eq!(
            folded(&scene(AiType::Interceptor, 300.0, true)).0,
            Goal::Plunder {
                ship: n(2),
                hopeless: true
            }
        );
        for ai_type in [AiType::WimpyTrader, AiType::BraveTrader] {
            assert_eq!(
                folded(&scene(ai_type, 300.0, true)),
                (Goal::Flee(n(2)), TravelWrite::Clear),
                "{ai_type:?}"
            );
        }
        assert_eq!(
            folded(&scene(AiType::WimpyTrader, 300.0, false)),
            (PLAIN, TravelWrite::Clear),
            "a chase that isn't hopeless"
        );
        assert_eq!(
            folded(&scene(AiType::WimpyTrader, 160.0, true)),
            (PLAIN, TravelWrite::Clear),
            "inside the box no chase is asked (@0x8ea49-0x8eac6)"
        );
    }

    #[test]
    fn a_traders_hopeless_plunder_fled_is_goal_0xds_and_a_snipe_is_none() {
        // Goal 3 with sub 5 (@0x8eaef-0x8eafb), then @0x8eb6b past every
        // call.
        let govts = govts(0, 0);
        let sites = sites();
        let thought = Decision {
            goal: PLAIN,
            ..Decision::default()
        };
        for (ai_type, marked) in [
            (AiType::WimpyTrader, Some(HopelessFlee::Plunder)),
            (AiType::Warship, None),
        ] {
            let npcs = scene(ai_type, 300.0, true);
            let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
            assert_eq!(
                after_think(&npcs[0], thought, &around).hopeless_flee,
                marked,
                "{ai_type:?}"
            );
        }
        let npcs = scene(AiType::WimpyTrader, 300.0, false);
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        assert_eq!(after_think(&npcs[0], thought, &around).hopeless_flee, None);
    }

    #[test]
    fn a_disabled_ships_plunder_holds_its_slot_and_is_never_hopeless() {
        // @0x8e8c9.
        let mut npcs = scene(AiType::WimpyTrader, 300.0, true);
        npcs[1].condition = Condition::Disabled;
        assert_eq!(folded(&npcs), (PLAIN, TravelWrite::Set(travel::of_slot(4))));
    }

    #[test]
    fn a_gone_ships_plunder_or_one_under_the_wait_writes_nothing_and_is_unsniped() {
        let npcs = scene(AiType::Warship, 300.0, true);
        assert_eq!(folded(&npcs[..1]), (PLAIN, TravelWrite::Keep), "gone");
        let mut waiting = npcs;
        waiting[0].ai_wait = 30;
        assert_eq!(
            folded(&waiting),
            (PLAIN, TravelWrite::Keep),
            "the high level returns first (@0x8d4b9-0x8d4d2)"
        );
    }

    #[test]
    fn after_a_think_the_fold_writes_over_the_thinks_travel_only_when_it_writes() {
        let govts = govts(0, 0);
        let sites = sites();
        let npcs = scene(AiType::WimpyTrader, 300.0, false);
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        let thought = Decision {
            goal: PLAIN,
            travel: TravelWrite::Set(StellarId(128)),
            ..Decision::default()
        };
        assert_eq!(
            after_think(&npcs[0], thought, &around).travel,
            TravelWrite::Clear
        );
        let alone = [npcs[0].clone()];
        let around = crate::ai::fixture::around(&sites, &alone, &govts, (0.0, -1000.0), 0);
        assert_eq!(after_think(&alone[0], thought, &around), thought);
        let other = Decision {
            goal: Goal::JumpOut,
            ..thought
        };
        assert_eq!(after_think(&npcs[0], other, &around), other);
    }

    #[test]
    fn a_wimpy_successors_hopeless_plunder_flees_with_no_call_and_calls_from_its_next_think() {
        // `_WimpyTraderAI` keeps goal 0xd unprovoked and calls for help in
        // goal 3 alone (@0x8b1e1); goal 0xd's high level then writes goal
        // 3 (@0x8eaef-0x8eafb), which the next think keeps and calls in.
        let govts = govts(0, 0);
        let sites = sites();
        let mut npcs = scene(AiType::WimpyTrader, 300.0, true);
        npcs[0].govt = Some(TRADERS);
        npcs[0].travel = Some(StellarId(128));
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        let decision = NovaAi::default().decision(&npcs[0], &around, &mut Draws::of(&[0; 4]));
        assert_eq!(decision.goal, Goal::Flee(n(2)));
        assert_eq!(decision.travel, TravelWrite::Clear);
        assert_eq!(decision.comm_call, None);
        assert_eq!(decision.hopeless_flee, Some(HopelessFlee::Plunder));
        npcs[0].goal = decision.goal;
        let around = crate::ai::fixture::around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        let next = NovaAi::default().decision(&npcs[0], &around, &mut Draws::of(&[0; 4]));
        assert_eq!(next.goal, Goal::Flee(n(2)));
        assert_eq!(next.comm_call, Some(CommCall::Distress));
    }
}
