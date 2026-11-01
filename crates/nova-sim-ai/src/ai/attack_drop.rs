//! `_HighLevelAIHandler`'s state-4 block (@0x8e246-0x8e731 in the `EV Nova`
//! executable), which every type's think reaches (`_AIDispatch` @0x901e5):
//! [`attack_block`].
//!
//! It runs on the goal the think, the slot test and the mission hunt leave
//! (@0x8d6b7): goal 4 ([`Goal::attacking`], an attack or a snipe, whatever
//! the ship's type), after the handler's head
//! ([`handler_head`](crate::ai::handler_head)), the lost-sight block and
//! the flee block. Its +0x50 0 (@0x8e25e) is the warp stage's, which
//! `traffic::warp_out` zeroes for any goal outside 2, 3 and 0xb. Target -1,
//! the slot test's, has ended at @0x8e263 → @0x8e731 already
//! ([`handler_head::slot_test`](crate::ai::handler_head::slot_test)).
//! Then, in order, each writing goal 0 and reaching the handler's tail,
//! which writes sub 0 (@0x8fb09-0x8fb13), so no pursuit carries over:
//!
//! 1. **The wait** (+0x4c above 0, @0x8e26d → @0x8e731, [`Npc::wait_running`]),
//!    reached only by a ship of goal 0xa or 0xe the hunt turned, as any
//!    other has left at the head. The think's +0x4c write is read
//!    ([`Decision::ends_wait`]): the traffic ends a wait later. The hunt's
//!    target 0 stays ([`Decision::holds_target`]).
//! 2. **The ally drop** (@0x8e277-0x8e2e4): an NPC target it spares as an
//!    ally ([`spares_ally`], by [`RuleKey::LedAllyAttack`]): its own
//!    government not -1, neither it nor the target led by the player, and
//!    `_GovtAllies`. Goal and sub 0, then @0x8fb47: +0x70 and +0x96 stay,
//!    so a dying ally keeps both. A warship's or an interceptor's next
//!    think reads the target held and takes it again
//!    ([`held_target`](crate::ai::held_target)), as their thinks spare no
//!    ally, so it stays on goal 0 while the ally lasts. The player is never an ally: its +0x98 is
//!    -1 (`_ResetPlayer` @0x1d46f), which `_GovtAllies` turns down
//!    (@0x4e5e-0x4e69).
//! 3. **The fleet drops** (@0x8e2e9-0x8e37c, [`fleet_drop`]), with P the
//!    ship's parent (+0x9a, [`Npc::leader_ref`]; the player's is -1,
//!    `_ResetPlayer` @0x1d4e5), T its target, and -1 at any step going on:
//!    P -1 drops nothing (@0x8e2f7); P is T (@0x8e306 → @0x8e381); T's
//!    parent is P (@0x8e322 → @0x8e81d); P's parent's parent is T's
//!    parent's parent (@0x8e374 → @0x8e381). Goal and sub 0, target -1,
//!    then @0x8fb47: +0x96 stays, dying or not. So an escort of the player
//!    drops an attack on the player (its +0x9a 0) or on a fellow escort,
//!    every frame. The walk is narrower than "same root"
//!    ([`Npc::fleet_among`]): a leader on its own follower, or a ship on
//!    its parent's parent, is not dropped.
//! 4. **Gone or breaking up** (+0xb8 clear @0x8e3e6, `_IsDying` @0x8e3f6 →
//!    @0x8e714): target -1, +0x96 0 (@0x8e71d), goal 0. Its +0x96 0 comes
//!    after the think, so it stands over a word's 1 (@0x8cb9d). A ship
//!    reaches it on a destroyed player only through the hunt, whose target
//!    0 comes after the slot test.
//!
//! The ship it keeps is then flown. These two rows write no goal 0:
//!
//! 5. **A hopeless chase**: a hull that doesn't stand off (`shïp` `Flags2`
//!    0x0002, @0x8e48e, which goes to its band instead, see
//!    [`stand_off`](crate::ai::stand_off)), its target outside the
//!    dogfight box on either axis ([`outside_box`], @0x8e41b-0x8e473; inside
//!    goes to @0x8e6a2 and never asks), and `_AIEvalHopelessChase`
//!    ([`hopeless_chase`], @0x8e5fe). A ship of AI type above 2
//!    ([`Npc::original_ai_type`], @0x8e60a) keeps goal 4 on sub 0xe
//!    ([`Goal::Snipe`], @0x8e614); any other flees, goal 3 and sub 5
//!    (@0x8e625-0x8e62e, [`Decision::flees_hopeless`]), which flies close
//!    at any distance. Both go on to the launch and the call, unless the
//!    ship is disabled (@0x8e6e5-0x8e70d), so the flight calls for
//!    reinforcements here, beyond the flee block's reach.
//! 6. **Anything else** is goal 4 on sub 7, 6 or 0x12
//!    (@0x8e63c-0x8e6dc): [`Goal::Attack`], over a think's
//!    [`Goal::Snipe`], as a chase can stop being hopeless.
//!
//! The think's other words stand in every row, as they came first. A drop
//! skips the fighter launch (@0x8e6fa) and the call for reinforcements
//! (@0x8e70d), and `_LowLevelAIHandler`'s passes then find no target. The
//! cleared target stands to the end of the frame for every thinker
//! ([`Decision::clears_target`]), and so does a held one.
//!
//! **Left out**: the fleet walk's step 4, T's parent's parent being T's
//! parent (@0x8e350 → @0x8e7af), never true, as every register write of
//! +0x9a (@0x3d870, @0x421ef, @0x8496e, @0x89488) names another ship; and
//! the never-run zero of @0x8e39e-0x8e3c1, which needs target -1 after
//! @0x8e263 has passed. A plunder (goal 0xd) has its own block. No sound
//! and no draw.

use crate::ai::odds::{hopeless_chase, outside_box};
use crate::ai::target::spares_ally;
use crate::ai::{Decision, Goal, Surroundings};
use crate::combat::ShipRef;
#[cfg(doc)]
use crate::rulebook::RuleKey;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;

/// `decision`, `npc`'s, through the state-4 block among `around`, an ally
/// spared as `led_ally` reads [`RuleKey::LedAllyAttack`] (see the module
/// docs).
#[must_use]
pub fn attack_block(
    npc: &Npc,
    decision: Decision,
    around: &Surroundings,
    led_ally: RuleSource,
) -> Decision {
    let Some(ship) = decision.goal.attacking() else {
        return decision;
    };
    let held = Decision {
        goal: Goal::Idle,
        holds_target: Some(ship),
        ..decision
    };
    if npc.wait_running() && !decision.ends_wait {
        return held;
    }
    let ally = match ship {
        ShipRef::Npc(id) => around
            .npc(id)
            .is_some_and(|other| spares_ally(npc, other, around, led_ally)),
        ShipRef::Player => false,
    };
    if ally {
        return held;
    }
    if fleet_drop(npc, ship, around) {
        return Decision {
            goal: Goal::Idle,
            clears_target: true,
            ..decision
        };
    }
    if !around.standing(ship) {
        return Decision {
            goal: Goal::Idle,
            clears_target: true,
            forgets_provocation: true,
            provokes: false,
            ..decision
        };
    }
    if !npc.hull.stands_off && outside_box(npc, ship, around) && hopeless_chase(npc, ship, around) {
        return if npc.original_ai_type() > 2 {
            Decision {
                goal: Goal::Snipe(ship),
                ..decision
            }
        } else {
            Decision {
                goal: Goal::Flee(ship),
                flees_hopeless: true,
                calls_reinforcements: decision.calls_reinforcements || npc.active_not_disabled(),
                ..decision
            }
        };
    }
    Decision {
        goal: Goal::Attack(ship),
        ..decision
    }
}

/// The ship `ship` follows among `around`, its +0x9a: none for the player
/// (`_ResetPlayer` @0x1d4e5) or a ship not in the system, otherwise its
/// [`leader_ref`](Npc::leader_ref).
fn parent(ship: ShipRef, around: &Surroundings) -> Option<ShipRef> {
    match ship {
        ShipRef::Player => None,
        ShipRef::Npc(id) => around.npc(id).and_then(Npc::leader_ref),
    }
}

/// Whether the fleet walk (@0x8e2e9-0x8e37c) drops `npc`'s attack on
/// `target` among `around` (see the module docs).
fn fleet_drop(npc: &Npc, target: ShipRef, around: &Surroundings) -> bool {
    let Some(own) = npc.leader_ref() else {
        return false;
    };
    if own == target {
        return true;
    }
    let Some(its) = parent(target, around) else {
        return false;
    };
    if its == own {
        return true;
    }
    // Step 4, the target's grandparent its own parent (@0x8e350), is
    // never true: no write of +0x9a names the ship itself.
    let Some(its_root) = parent(its, around) else {
        return false;
    };
    parent(own, around).and_then(|up| parent(up, around)) == Some(its_root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::CommCall;
    use crate::ai::fixture::{PIRATES, POLICE, TRADERS, around, govts, n, ship, sites};
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::traffic::npc::{AiType, Mode, NpcId};

    const DYING: Condition = Condition::Dying {
        ticks_left: 3,
        starting: false,
    };

    fn of(goal: Goal) -> Decision {
        Decision {
            goal,
            ..Decision::default()
        }
    }

    /// NPC 1, a traders' warship at the centre.
    fn attacker() -> Npc {
        ship(1, TRADERS, AiType::Warship, 0.0, 0.0)
    }

    /// NPC `id` of `govt`, 100 to the right, led by `leader`.
    fn other(id: u32, govt: crate::catalog::GovtId, leader: Option<u32>) -> Npc {
        Npc {
            leader: leader.map(NpcId),
            ..ship(id, govt, AiType::Warship, 100.0, 0.0)
        }
    }

    fn escorting(npc: Npc) -> Npc {
        Npc {
            escort: Some(EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            }),
            ..npc
        }
    }

    /// `decision` of `npcs[0]` through the block among `npcs`, the player
    /// 500 up and as `player` says, by `led_ally`.
    fn blocked(
        npcs: &[Npc],
        decision: Decision,
        player: Condition,
        led_ally: RuleSource,
    ) -> Decision {
        let govts = govts(0, 0);
        let sites = sites();
        let mut around = around(&sites, npcs, &govts, (0.0, -500.0), 0);
        if let Some(side) = around.player.as_mut() {
            side.condition = player;
        }
        attack_block(&npcs[0], decision, &around, led_ally)
    }

    /// [`blocked`] by the engine, the player intact.
    fn by_engine(npcs: &[Npc], decision: Decision) -> Decision {
        blocked(npcs, decision, Condition::Intact, RuleSource::Engine)
    }

    /// Row 4's drop of `decision`.
    fn dropped_dying(decision: Decision) -> Decision {
        Decision {
            goal: Goal::Idle,
            clears_target: true,
            forgets_provocation: true,
            provokes: false,
            ..decision
        }
    }

    /// A drop that keeps the target and the provocation (rows 1 and 2).
    fn held(decision: Decision, ship: ShipRef) -> Decision {
        Decision {
            goal: Goal::Idle,
            holds_target: Some(ship),
            ..decision
        }
    }

    /// A fleet drop (row 3): target -1, +0x96 kept.
    fn fleet_dropped(decision: Decision) -> Decision {
        Decision {
            goal: Goal::Idle,
            clears_target: true,
            ..decision
        }
    }

    #[test]
    fn row_1_holds_a_hunted_ships_target_under_its_wait() {
        // Only a goal 0xa or 0xe ship the hunt turned reaches it: +0x4c
        // above 0 (@0x8e26d) goes to @0x8e731, goal 0, the target 0 kept.
        let hunt = Decision {
            turns_on_player: true,
            ..of(Goal::Attack(ShipRef::Player))
        };
        for waiting in [
            Npc {
                ai_wait: 3,
                ..attacker()
            },
            Npc {
                mode: Mode::HeldAtGate { ticks_left: 20 },
                ..attacker()
            },
        ] {
            assert_eq!(
                by_engine(&[waiting], hunt),
                held(hunt, ShipRef::Player),
                "@0x8e26d → @0x8e731"
            );
        }
    }

    #[test]
    fn a_think_that_ended_its_wait_passes_row_1() {
        // The think wrote +0x4c first (@0x83d62); the traffic ends it later.
        let waiting = Npc {
            ai_wait: 3,
            ..attacker()
        };
        let dying = Npc {
            condition: DYING,
            ..other(2, PIRATES, None)
        };
        let ended = Decision {
            ends_wait: true,
            ..of(Goal::Attack(n(2)))
        };
        assert_eq!(
            by_engine(&[waiting, dying], ended),
            dropped_dying(ended),
            "on to @0x8e3ef"
        );
    }

    #[test]
    fn an_ally_is_dropped_keeping_the_target_and_the_provocation_dying_or_not() {
        // @0x8e277-0x8e2e4: goal and sub 0, then @0x8fb47; no +0x96 or
        // +0x70 write, before the dying test.
        let attack = Decision {
            provokes: true,
            ..of(Goal::Attack(n(2)))
        };
        for condition in [Condition::Intact, Condition::Disabled, DYING] {
            let ally = Npc {
                condition,
                ..other(2, POLICE, None)
            };
            assert_eq!(
                by_engine(&[attacker(), ally], attack),
                held(attack, n(2)),
                "{condition:?}"
            );
        }
        let snipe = of(Goal::Snipe(n(2)));
        assert_eq!(
            by_engine(&[attacker(), other(2, POLICE, None)], snipe),
            held(snipe, n(2)),
            "goal 4 either way"
        );
    }

    #[test]
    fn led_ally_attack_is_read_both_ways() {
        // By the engine, a ship of no government, a ship the player leads
        // or an ally it leads spares no ally (@0x8e277-0x8e2d0).
        let attack = of(Goal::Attack(n(2)));
        let led = escorting(other(2, POLICE, None));
        let npcs = [attacker(), led];
        assert_eq!(by_engine(&npcs, attack), attack, "the engine fights on");
        assert_eq!(
            blocked(&npcs, attack, Condition::Intact, RuleSource::Bible),
            held(attack, n(2)),
            "the other reading spares it"
        );
        let stateless = Npc {
            govt: None,
            ..attacker()
        };
        let none = Npc {
            govt: None,
            ..other(2, POLICE, None)
        };
        assert_eq!(
            by_engine(&[stateless, none], attack),
            attack,
            "@0x8e27e: no government, no ally drop"
        );
    }

    #[test]
    fn a_player_target_is_never_an_ally() {
        // The player's +0x98 is -1 (`_ResetPlayer` @0x1d46f), which
        // `_GovtAllies` turns down at once (@0x4e5e-0x4e69).
        let attack = of(Goal::Attack(ShipRef::Player));
        let stateless = Npc {
            govt: None,
            ..attacker()
        };
        for npc in [attacker(), stateless] {
            for led_ally in [RuleSource::Engine, RuleSource::Bible] {
                assert_eq!(
                    blocked(
                        std::slice::from_ref(&npc),
                        attack,
                        Condition::Intact,
                        led_ally
                    ),
                    attack,
                    "{led_ally:?}"
                );
            }
        }
    }

    #[test]
    fn a_ship_attacking_its_parent_is_dropped() {
        // Step 2: +0x9a is +0x70 (@0x8e306 → @0x8e381).
        let led = Npc {
            leader: Some(NpcId(5)),
            ..attacker()
        };
        let attack = of(Goal::Attack(n(5)));
        assert_eq!(
            by_engine(&[led, other(5, PIRATES, None)], attack),
            fleet_dropped(attack)
        );
        let escort = escorting(attacker());
        let on_player = Decision {
            turns_on_player: true,
            ..of(Goal::Attack(ShipRef::Player))
        };
        assert_eq!(
            by_engine(&[escort], on_player),
            fleet_dropped(on_player),
            "an escort on the player: +0x9a 0 is +0x70 0"
        );
    }

    #[test]
    fn a_ship_attacking_a_fellow_follower_is_dropped_dying_or_not() {
        // Step 3: the target's +0x9a is its own (@0x8e322 → @0x8e81d), and
        // +0x96 stays.
        let attack = Decision {
            provokes: true,
            ..of(Goal::Attack(n(6)))
        };
        let led = Npc {
            leader: Some(NpcId(5)),
            ..attacker()
        };
        for condition in [Condition::Intact, DYING] {
            let fellow = Npc {
                condition,
                ..other(6, PIRATES, Some(5))
            };
            assert_eq!(
                by_engine(&[led.clone(), fellow], attack),
                fleet_dropped(attack),
                "{condition:?}"
            );
        }
        let escorts = [escorting(attacker()), escorting(other(6, PIRATES, None))];
        assert_eq!(
            by_engine(&escorts, attack),
            fleet_dropped(attack),
            "a fellow escort of the player"
        );
    }

    #[test]
    fn a_ship_whose_grandparent_is_its_targets_grandparent_is_dropped() {
        // Step 5: +0x9a's +0x9a's +0x9a is the target's +0x9a's +0x9a
        // (@0x8e356-0x8e37c → @0x8e381).
        let attack = of(Goal::Attack(n(6)));
        let led = Npc {
            leader: Some(NpcId(5)),
            ..attacker()
        };
        let chain = |top: Option<u32>| {
            vec![
                led.clone(),
                other(5, PIRATES, Some(7)),
                other(7, PIRATES, top),
                other(6, PIRATES, Some(9)),
                other(9, PIRATES, Some(4)),
            ]
        };
        assert_eq!(by_engine(&chain(Some(4)), attack), fleet_dropped(attack));
        assert_eq!(by_engine(&chain(Some(3)), attack), attack, "other roots");
        assert_eq!(by_engine(&chain(None), attack), attack, "@0x8e36b");
    }

    #[test]
    fn the_walks_that_find_no_fleet_drop_nothing() {
        let attack = of(Goal::Attack(n(6)));
        // Step 1: a ship with no parent (@0x8e2f7), its own follower.
        let follower = other(6, PIRATES, Some(1));
        assert_eq!(by_engine(&[attacker(), follower], attack), attack);
        // Its parent's parent, which has none (@0x8e32c).
        let led = Npc {
            leader: Some(NpcId(5)),
            ..attacker()
        };
        let npcs = [
            led.clone(),
            other(5, PIRATES, Some(6)),
            other(6, PIRATES, None),
        ];
        assert_eq!(by_engine(&npcs, attack), attack, "its parent's parent");
        // A target with no parent's parent (@0x8e347).
        let npcs = [
            led,
            other(5, PIRATES, None),
            other(6, PIRATES, Some(9)),
            other(9, PIRATES, None),
        ];
        assert_eq!(by_engine(&npcs, attack), attack, "no grandparent");
    }

    #[test]
    fn a_ship_breaking_up_or_destroyed_is_dropped_with_its_provocation() {
        // +0xb8 clear (@0x8e3e6) or `_IsDying` (@0x8e3f6) → @0x8e714:
        // target -1, +0x96 0, goal 0; its 0 after a word's 1 (@0x8e71d).
        let worded = Decision {
            provokes: true,
            turns_on_player: true,
            comm_call: Some(CommCall::Distress),
            calls_reinforcements: true,
            ..of(Goal::Attack(n(2)))
        };
        for condition in [DYING, Condition::Destroyed] {
            let foe = Npc {
                condition,
                ..other(2, PIRATES, None)
            };
            assert_eq!(
                by_engine(&[attacker(), foe.clone()], worded),
                dropped_dying(worded),
                "{condition:?}"
            );
            let snipe = of(Goal::Snipe(n(2)));
            assert_eq!(
                by_engine(&[attacker(), foe], snipe),
                dropped_dying(snipe),
                "a snipe, {condition:?}"
            );
        }
        let hunt = of(Goal::Attack(ShipRef::Player));
        for player in [DYING, Condition::Destroyed] {
            assert_eq!(
                blocked(&[attacker()], hunt, player, RuleSource::Engine),
                dropped_dying(hunt),
                "the player {player:?}"
            );
        }
    }

    #[test]
    fn a_target_standing_is_kept() {
        for condition in [Condition::Intact, Condition::Disabled] {
            let attack = of(Goal::Attack(n(2)));
            let foe = Npc {
                condition,
                ..other(2, PIRATES, None)
            };
            assert_eq!(by_engine(&[attacker(), foe], attack), attack);
        }
        let player = of(Goal::Attack(ShipRef::Player));
        assert_eq!(by_engine(&[attacker()], player), player);
    }

    /// `chaser`, heavy at the centre, and NPC 2, a pirate fleeing it at
    /// (0, `y`), its gap opening: a hopeless chase (`_AIEvalHopelessChase`).
    fn chase(chaser: Npc, y: f32) -> [Npc; 2] {
        let heavy = Npc {
            hull: crate::combat::hull::HullSpec {
                mass: 100.0,
                ..chaser.hull
            },
            ..chaser
        };
        let mut runner = ship(2, PIRATES, AiType::Warship, 0.0, y);
        runner.goal = Goal::Flee(n(1));
        runner.state.velocity = crate::geometry::Vec2::new(0.0, -1.0);
        [heavy, runner]
    }

    #[test]
    fn a_hopeless_chase_outside_the_dogfight_box_is_sniped_by_any_type_above_2() {
        // Either axis beyond 165 (@0x8e41b-0x8e473), no stand-off hull
        // (@0x8e48e) and `_AIEvalHopelessChase` (@0x8e5fe): +0x88 above 2
        // keeps goal 4 on sub 0xe (@0x8e614).
        let worded = Decision {
            comm_call: Some(CommCall::Distress),
            ..of(Goal::Attack(n(2)))
        };
        for ai_type in [AiType::Warship, AiType::Interceptor] {
            let warship = Npc {
                ai_type,
                ..attacker()
            };
            assert!(crate::ai::odds::hopeless_chase(
                &chase(warship.clone(), -300.0)[0],
                n(2),
                &Surroundings::new(&[], &chase(warship.clone(), -300.0))
            ));
            assert_eq!(
                by_engine(&chase(warship.clone(), -300.0), worded),
                Decision {
                    goal: Goal::Snipe(n(2)),
                    ..worded
                },
                "{ai_type:?}"
            );
            assert_eq!(
                by_engine(&chase(warship, -100.0), worded),
                worded,
                "{ai_type:?}: inside the box goes to @0x8e6a2 and never asks"
            );
        }
        let led = Npc {
            leader: Some(NpcId(9)),
            ..attacker()
        };
        let mut npcs = chase(led, -300.0).to_vec();
        npcs.push(other(9, PIRATES, None));
        assert_eq!(
            by_engine(&npcs, worded).goal,
            Goal::Snipe(n(2)),
            "a fleet escort, +0x88 6"
        );
    }

    #[test]
    fn a_stand_off_hull_never_asks_whether_a_chase_is_hopeless() {
        // @0x8e48e jumps past `_AIEvalHopelessChase` to the band.
        let stand_off = Npc {
            hull: crate::combat::hull::HullSpec {
                stands_off: true,
                ..attacker().hull
            },
            ..attacker()
        };
        let attack = of(Goal::Attack(n(2)));
        assert_eq!(by_engine(&chase(stand_off, -300.0), attack), attack);
    }

    #[test]
    fn a_trader_in_a_hopeless_chase_flees_close_launching_and_calling() {
        // +0x88 2 or below: goal 3, sub 5 (@0x8e625-0x8e62e), then the
        // launch and the call (@0x8e6eb-0x8e70d) unless it is disabled.
        let attack = Decision {
            comm_call: Some(CommCall::Distress),
            ..of(Goal::Attack(n(2)))
        };
        for ai_type in [AiType::WimpyTrader, AiType::BraveTrader] {
            let trader = Npc {
                ai_type,
                ..attacker()
            };
            assert_eq!(
                by_engine(&chase(trader.clone(), -300.0), attack),
                Decision {
                    goal: Goal::Flee(n(2)),
                    flees_hopeless: true,
                    calls_reinforcements: true,
                    ..attack
                },
                "{ai_type:?}"
            );
            let downed = Npc {
                condition: Condition::Disabled,
                ..trader.clone()
            };
            assert_eq!(
                by_engine(&chase(downed, -300.0), attack),
                Decision {
                    goal: Goal::Flee(n(2)),
                    flees_hopeless: true,
                    ..attack
                },
                "{ai_type:?}: disabled, no call (@0x8e6eb)"
            );
            assert_eq!(
                by_engine(&chase(trader, -100.0), attack),
                attack,
                "{ai_type:?}: inside the box"
            );
        }
    }

    #[test]
    fn a_thinks_snipe_that_isnt_hopeless_is_an_attack() {
        // Every other goal 4 flies sub 7, 6 or 0x12 (@0x8e63c-0x8e6dc), the
        // engine's `Goal::Attack`, whatever the think wrote.
        for at in [-300.0, -100.0] {
            let npcs = [attacker(), other(2, PIRATES, None)];
            let mut npcs = npcs;
            npcs[1].state.position = crate::geometry::Vec2::new(0.0, at);
            assert_eq!(
                by_engine(&npcs, of(Goal::Snipe(n(2)))),
                of(Goal::Attack(n(2))),
                "{at}"
            );
        }
        let stand_off = Npc {
            hull: crate::combat::hull::HullSpec {
                stands_off: true,
                ..attacker().hull
            },
            ..attacker()
        };
        assert_eq!(
            by_engine(&chase(stand_off, -300.0), of(Goal::Snipe(n(2)))),
            of(Goal::Attack(n(2))),
            "a stand-off hull's band is its own"
        );
    }

    #[test]
    fn only_goal_4_reaches_the_block() {
        let dying = Npc {
            condition: DYING,
            ..other(2, POLICE, None)
        };
        let waiting = Npc {
            ai_wait: 3,
            ..attacker()
        };
        for goal in [
            Goal::Idle,
            Goal::Flee(n(2)),
            Goal::Inspect(n(2)),
            Goal::Plunder {
                ship: n(2),
                hopeless: false,
            },
        ] {
            assert_eq!(
                by_engine(&[waiting.clone(), dying.clone()], of(goal)),
                of(goal),
                "{goal:?}"
            );
        }
    }
}
