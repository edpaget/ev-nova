//! A ship assisting the player through its type's think: whether the
//! think returns at once for goals 9 and 0xf ([`type_think_returns`]),
//! as every engine AI type's does in the `EV Nova` executable:
//!
//! - `_WimpyTraderAI` @0x8b045-0x8b05c, `_BraveTraderAI` @0x8b4af-0x8b4c6
//!   and `_WarshipAI` @0x8b745-0x8b75c, after their `_IsDisabled` test;
//! - `_InterceptorAI` @0x8caf7-0x8cb07, after its `_IsDisabled` and goal
//!   0x16 tests and its nag-ship block (+0xc8d0 = 0x3ff,
//!   @0x8c8c8-0x8caf2), which the engine has no ship for.
//!
//! `_MinerAI` (@0x8b202-0x8b492) has no such test: a leaderless miner
//! ([`miner::thinks_for`]) assisting the player thinks as ever on its next
//! decision and drops the help (it flees, mines, waits or lands), as the
//! engine's [`MinerAi`](crate::ai::miner::MinerAi) decides. Request
//! Assistance helps any AI type of 4 or below (`_DoCommDialog` @0x96564),
//! so a miner can hold the help. A pirate warship (a plunderer, refused
//! "In your dreams") and an escort (AI type above 4, refused 0x19,
//! @0x9663a) never hold goal 9 or 0xf: `_AIMakeShipRefuelPlayer` and
//! `_AIMakeShipRepairPlayer` are called from Request Assistance alone
//! (@0x96618, @0x96606).
//!
//! Provoked or not, an assister whose think returns keeps its goal,
//! target and trigger, and `_AIDispatch` runs its high level all the same
//! (@0x901e5): the traffic's `high_levels_due`.

use crate::ai::Goal;
use crate::ai::miner;
use crate::traffic::npc::Npc;

/// Whether `npc` assists the player ([`Goal::Assist`], goals 9 and 0xf)
/// and its type's think returns at once for it: any ship but a leaderless
/// miner (see the module docs).
#[must_use]
pub fn type_think_returns(npc: &Npc) -> bool {
    matches!(npc.goal, Goal::Assist(_)) && !miner::thinks_for(npc)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::miner::{MINES, SCOOPS};
    use crate::combat::ShipRef;
    use crate::combat::armament::Trigger;
    use crate::combat::hull::Condition;
    use crate::geometry::Vec2;
    use crate::hail::Help;
    use crate::reserves::Gauge;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::assisting::{Recording, player_in};
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{AiType, NpcId};
    use crate::traffic::{Traffic, World};
    use nova_sim_base::fixture::{FAST, Scripted};

    const TYPES: [AiType; 4] = [
        AiType::WimpyTrader,
        AiType::BraveTrader,
        AiType::Warship,
        AiType::Interceptor,
    ];

    /// NPC 1 of `ai_type` giving `help` to the player from (`x`, 0) at
    /// rest, its class turn 3, provoked by `provoked`.
    fn assister(ai_type: AiType, help: Help, x: f32, provoked: f32) -> Npc {
        let mut ship = npc(1, ShipStats::new(FAST, &[]));
        ship.ai_type = ai_type;
        ship.goal = Goal::Assist(help);
        ship.target = Some(ShipRef::Player);
        ship.hull.class_turn = 3.0;
        ship.state.position = Vec2::new(x, 0.0);
        ship.provoked = provoked;
        ship
    }

    /// The traffic of `ship` alone, deciding every tick.
    fn traffic_of(ship: Npc) -> Traffic {
        let mut traffic = Traffic::with_interval(1);
        traffic.npcs = vec![ship];
        traffic
    }

    /// A world holding the player at the centre in `condition` with
    /// `fuel` of 300.
    fn world(condition: Condition, fuel: f32) -> World<'static> {
        let mut player = player_in(condition);
        player.reserves.fuel = Gauge {
            now: fuel,
            max: 300.0,
        };
        World {
            player: Some(player),
            ..World::new(&[])
        }
    }

    #[test]
    fn every_engine_ai_types_think_returns_for_an_assister_but_a_leaderless_miners() {
        for ai_type in TYPES {
            for help in [Help::Refuel, Help::Repair] {
                assert!(type_think_returns(&assister(ai_type, help, 0.0, 0.0)));
                let mut miner = assister(ai_type, help, 0.0, 0.0);
                miner.miner = MINES;
                assert!(!type_think_returns(&miner), "{ai_type:?} miner");
                miner.leader = Some(NpcId(9));
                assert!(type_think_returns(&miner), "a led miner isn't one");
            }
            let mut idle = assister(ai_type, Help::Refuel, 0.0, 0.0);
            idle.goal = Goal::Idle;
            assert!(!type_think_returns(&idle), "not assisting");
        }
    }

    #[test]
    fn an_assister_of_any_type_keeps_its_help_through_its_think_provoked_or_not() {
        for ai_type in TYPES {
            for help in [Help::Refuel, Help::Repair] {
                for provoked in [0.0, 5.0] {
                    let mut ship = assister(ai_type, help, 5000.0, provoked);
                    let trigger = Trigger {
                        primary: true,
                        ..Trigger::default()
                    };
                    ship.trigger = trigger;
                    let mut traffic = traffic_of(ship);
                    let recording = Recording::default();
                    for _ in 0..3 {
                        traffic.tick_in(
                            &recording,
                            world(Condition::Disabled, 40.0),
                            &[],
                            &mut Scripted::default(),
                        );
                    }
                    let kept = &traffic.npcs()[0];
                    let case = format!("{ai_type:?} {help:?} {provoked}");
                    assert_eq!(*recording.decided.borrow(), [], "{case}");
                    assert_eq!(kept.goal, Goal::Assist(help), "{case}");
                    assert_eq!(kept.target, Some(ShipRef::Player), "{case}");
                    assert_eq!(kept.trigger, trigger, "{case}");
                    assert_eq!(kept.provoked, provoked, "{case}");
                }
            }
        }
    }

    #[test]
    fn an_assisters_high_level_runs_provoked_or_not() {
        for provoked in [0.0, 5.0] {
            let mut traffic =
                traffic_of(assister(AiType::WimpyTrader, Help::Refuel, 10.0, provoked));
            traffic.tick_in(
                &Recording::default(),
                world(Condition::Intact, 40.0),
                &[],
                &mut Scripted::default(),
            );
            assert_eq!(traffic.take_fuel_given(), 1.0, "provoked {provoked}");
        }
    }

    #[test]
    fn a_leaderless_miner_assisting_the_player_thinks() {
        for bits in [MINES, SCOOPS] {
            let mut ship = assister(AiType::WimpyTrader, Help::Refuel, 5000.0, 0.0);
            ship.miner = bits;
            let mut traffic = traffic_of(ship);
            let recording = Recording::default();
            traffic.tick_in(
                &recording,
                world(Condition::Intact, 40.0),
                &[],
                &mut Scripted::default(),
            );
            assert_eq!(*recording.decided.borrow(), [NpcId(1)], "{bits}");
        }
    }
}
