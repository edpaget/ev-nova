//! A pirate's wait while another ship boards the ship it would plunder:
//! `_PirateWarshipAI`'s wait loop (@0x8c68c-0x8c730 in the `EV Nova`
//! executable) and its count wait, goal 0x16 ([`Goal::CountWait`]).
//!
//! - **The wait loop** ([`resolve`]): a pirate whose think found its victim
//!   boarded ([`Decision::board_wait`], see [`pirate`](crate::ai::pirate))
//!   looks over slots 1-63 for a ship boarding it: in the system and not
//!   disabled or breaking up (`_IsDisabled` @0x8c6c5), not the pirate, not
//!   the victim, not led by the player (+0x9a ≠ 0, so a ship led by no one
//!   counts), targeting the victim, and plundering (goal 0xd, any target
//!   of its own) or sniping (sub-goal 0xe, whatever its goal: a hopeless
//!   attack or plunder, or a stand-off ship keeping its band at 0xe). The
//!   first found makes it wait: goal 0x16, sub 0, target, nav and
//!   provocation cleared, its +0x4c [`BOARD_WAIT`] (@0x8c6f5-0x8c71c,
//!   [`enter`]), and
//!   a plunder its think decided is dropped, as the loop runs before the
//!   plunder (@0x8c74c, see [`plunder`](crate::traffic::plunder)). With
//!   none it attacks the victim, or drops it fully dry (the think's goal).
//!   The original's loop runs inside each pirate's think, slot by slot,
//!   so it reads the ships of lower slots as their thinks this frame left
//!   them and the higher ones as last frame's: of two pirates on one
//!   boarded victim, the lower waits and the higher attacks. The traffic
//!   decides every NPC from one view, so [`resolve`] replays that order
//!   as a pass over the decisions, by berth: a lower berth's new goal, a
//!   higher berth's goal as the tick found it. A lower berth's sub-goal
//!   0xe is read from its goal and the frame it flew last tick, its new
//!   frame being worked out after the decisions
//!   ([`pursuit`](crate::ai::pursuit)): a stand-off ship that took up its
//!   band this tick is read as it flew last tick, a lag the player never
//!   sees.
//! - **The count wait**: `_HighLevelAIHandler` holds goal 0x16 while +0x4c
//!   is above 0, firing nothing (sub 1, a stop), and at 0 sets goal 0
//!   (@0x8d462-0x8d4ac), the per-type think having returned on 0x16 that
//!   frame (@0x8c2ee); `_HandleShip` takes a frame off +0x4c after the
//!   thinks (@0x35037-0x35048), on each of double time's passes. So the
//!   frame that sets 120 holds and leaves 119, the 120th frame after ends
//!   the wait, and the pirate searches again on the 121st. The wait is the
//!   ship's AI wait ([`Npc::ai_wait`]), apart from its goal, as the
//!   original keeps it in +0x4c: the traffic counts it down as every AI
//!   wait is ([`plunder::count_down`](crate::traffic::plunder::count_down),
//!   as it flies, every pass) and runs the end ([`lapse`]) after its goals,
//!   on a think's tick alone. A hit's answer cuts the wait to 20 as it cuts
//!   a gate's ([`HIT_WAIT`](crate::traffic::autopilot::HIT_WAIT):
//!   `_DamageShip` reads no goal, @0x3be6e-0x3be7c).
//! - **Turned out of it**: a goal that replaces 0x16 writes no +0x4c, so
//!   the ship keeps what is left of its wait: the player's hit
//!   (`_AIMakeShipAttackPlayer` @0x3be69, then the cut), a Good
//!   Samaritan's call (`_DoGoodSamaritan` @0x82b82-0x82b97, uncut), and a
//!   hail's attack or bribe (`_DoCommDialog` @0x9621f, @0x961eb). Goal 4
//!   is not exempt from the wait as 0x16 is, so the high level returns
//!   (@0x8d4b9-0x8d4d2) and `_HandleShip` holds its turn, thrust and fire
//!   (@0x3443f-0x34454, @0x34a54-0x34a72): it sits still for what is left,
//!   then attacks.
//! - **The provocation**: the wait loop zeroes it as the wait starts
//!   (@0x8c713, [`enter`]). Goal 0x16's block never writes +0x96, so a hit
//!   during the wait adds to it and it stays, here too (see
//!   [`provocation`](crate::traffic::provocation)).
//!
//! Nothing here plays a sound.

use crate::ai::fire::Manoeuvre;
use crate::ai::travel::TravelWrite;
use crate::ai::{Decision, Goal};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::traffic::npc::Npc;

/// The ticks a pirate waits while another ship boards its victim (+0x4c
/// = 120.0, 0x42f00000 @0x8c71c).
pub const BOARD_WAIT: u32 = 120;

/// The wait loop over `decisions`, one for each of `npcs` in order (see
/// the module docs): each pirate whose decision waits on a boarded ship
/// ([`Decision::board_wait`]) and finds a ship boarding it, in berth order,
/// waits ([`Goal::CountWait`]), its travel stellar cleared (@0x8c70d),
/// calling for no reinforcements, as
/// `_HighLevelAIHandler` returns while its wait runs (@0x8d4b9-0x8d4d2).
pub fn resolve(npcs: &[Npc], decisions: &mut [Option<Decision>]) {
    let mut order: Vec<usize> = (0..npcs.len()).collect();
    order.sort_by_key(|&at| npcs[at].berth);
    for (rank, &at) in order.iter().enumerate() {
        let Some(victim) = decisions[at].and_then(|decision| decision.board_wait) else {
            continue;
        };
        let (lower, higher) = order.split_at(rank);
        let boarded_by = |other: usize, (goal, target): (Goal, Option<ShipRef>)| {
            boards(&npcs[other], goal, target, victim)
        };
        let boarder = lower
            .iter()
            .any(|&other| boarded_by(other, decided(&npcs[other], decisions[other])))
            || higher
                .iter()
                .skip(1)
                .any(|&other| boarded_by(other, (npcs[other].goal, npcs[other].target)));
        if boarder && let Some(decision) = decisions[at].as_mut() {
            *decision = Decision {
                goal: Goal::CountWait,
                calls_reinforcements: false,
                clears_target: true,
                keeps_pursuit: false,
                plunders: None,
                // +0x6c -1 (@0x8c70d).
                travel: TravelWrite::Clear,
                ..*decision
            };
        }
    }
}

/// The goal and target `npc` holds once its think this tick, if any, set
/// `decision`: its new goal and the ship that goal fights. A goal that
/// fights nothing boards no ship whatever target it keeps, as boarding is
/// plundering or sniping, so its target is not read.
fn decided(npc: &Npc, decision: Option<Decision>) -> (Goal, Option<ShipRef>) {
    decision.map_or((npc.goal, npc.target), |decision| {
        (decision.goal, decision.goal.quarry())
    })
}

/// Whether `other`, of `goal` and `target`, boards `victim` as the wait
/// loop reads it (@0x8c6a0-0x8c6f3; see the module docs).
fn boards(other: &Npc, goal: Goal, target: Option<ShipRef>, victim: ShipRef) -> bool {
    ShipRef::Npc(other.id) != victim
        && other.leader_ref() != Some(ShipRef::Player)
        && other.condition == Condition::Intact
        && target == Some(victim)
        && (goal.plundering().is_some() || sniping(other, goal))
}

/// Whether `npc` of `goal` is in sub-goal 0xe: sniping
/// ([`Goal::Snipe`]), plundering hopelessly, or flying a snipe by the
/// frame it flew for that goal (a stand-off ship in its band).
fn sniping(npc: &Npc, goal: Goal) -> bool {
    let snipe_frame = npc.pursuit.frame.is_some_and(|frame| {
        frame.goal == goal
            && frame
                .last()
                .is_some_and(|body| body.manoeuvre == Manoeuvre::Snipe)
    });
    matches!(goal, Goal::Snipe(_) | Goal::Plunder { hopeless: true, .. }) || snipe_frame
}

/// `npc` taking `goal`: one whose goal becomes 0x16 from another takes
/// [`BOARD_WAIT`] ticks of AI wait ([`Npc::ai_wait`], +0x4c = 120.0
/// @0x8c71c) and loses its provocation (+0x96 = 0, @0x8c713); one
/// already waiting keeps what is left of its wait, and its provocation.
pub(crate) fn enter(npc: &mut Npc, goal: Goal) {
    if goal == Goal::CountWait && npc.goal != Goal::CountWait {
        npc.ai_wait = BOARD_WAIT;
        npc.provoked = 0.0;
    }
}

/// The end of each count wait among `npcs` whose think was `due` this
/// tick: at none of its AI wait left, goal 0 (`_HighLevelAIHandler`
/// @0x8d497-0x8d4a3), [`Goal::Idle`].
pub fn lapse(npcs: &mut [Npc], due: &[bool]) {
    for (npc, _) in npcs.iter_mut().zip(due).filter(|(_, due)| **due) {
        if npc.goal == Goal::CountWait && npc.ai_wait == 0 {
            npc.goal = Goal::Idle;
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::pursuit::{Body, Frame, Kept, Pursuit};
    use crate::ai::{Behaviour, Surroundings};
    use crate::chance::Chance;
    use crate::combat::NpcId;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::stats::ShipStats;
    use crate::traffic::fixture::{count_waiting, npc};
    use crate::traffic::{CommCall, Traffic, World};
    use nova_sim_base::fixture::{Draws, FAST};

    fn n(id: u32) -> ShipRef {
        ShipRef::Npc(NpcId(id))
    }

    /// The victim, NPC 9 in berth 9, disabled and boarded.
    fn victim() -> Npc {
        let mut ship = npc(9, ShipStats::new(FAST, &[]));
        ship.berth = 9;
        ship.condition = Condition::Disabled;
        ship.boarded = true;
        ship
    }

    /// NPC `id` in berth `berth`, of `goal`, targeting its quarry.
    fn ship(id: u32, berth: u32, goal: Goal) -> Npc {
        let mut ship = npc(id, ShipStats::new(FAST, &[]));
        ship.berth = berth;
        ship.goal = goal;
        ship.target = goal.quarry();
        ship
    }

    fn plunder(ship: ShipRef) -> Goal {
        Goal::Plunder {
            ship,
            hopeless: false,
        }
    }

    /// A pirate's decision waiting on the victim: attacking it otherwise.
    fn waiting_on(victim: ShipRef) -> Decision {
        Decision {
            goal: Goal::Attack(victim),
            board_wait: Some(victim),
            calls_reinforcements: true,
            turns_on_player: true,
            ..Decision::default()
        }
    }

    const WAIT: Goal = Goal::CountWait;

    /// The goals `decisions` hold for `npcs` once resolved.
    fn resolved(npcs: &[Npc], mut decisions: Vec<Option<Decision>>) -> Vec<Option<Goal>> {
        resolve(npcs, &mut decisions);
        decisions
            .into_iter()
            .map(|decision| decision.map(|decision| decision.goal))
            .collect()
    }

    #[test]
    fn of_two_pirates_on_one_boarded_victim_the_lower_berth_waits_and_the_higher_attacks() {
        let victim_ref = n(9);
        for (low, high) in [(1, 2), (2, 1)] {
            let npcs = [
                ship(1, low, plunder(victim_ref)),
                ship(2, high, plunder(victim_ref)),
                victim(),
            ];
            let decisions = vec![
                Some(waiting_on(victim_ref)),
                Some(waiting_on(victim_ref)),
                None,
            ];
            let goals = resolved(&npcs, decisions);
            let (lower, higher) = if low == 1 { (0, 1) } else { (1, 0) };
            assert_eq!(goals[lower], Some(WAIT), "berth {low} waits");
            assert_eq!(
                goals[higher],
                Some(Goal::Attack(victim_ref)),
                "berth {high} reads the lower's new goal 0x16"
            );
        }
        assert_eq!(BOARD_WAIT, 120);
    }

    #[test]
    fn a_wait_keeps_the_thinks_call_on_the_player_and_calls_for_no_reinforcements() {
        let npcs = [ship(1, 1, Goal::Idle), ship(2, 2, plunder(n(9))), victim()];
        let kept = Decision {
            keeps_pursuit: true,
            ..waiting_on(n(9))
        };
        let mut decisions = vec![Some(kept), None, None];
        resolve(&npcs, &mut decisions);
        let waited = decisions[0].expect("decided");
        assert_eq!(waited.goal, WAIT);
        assert!(!waited.calls_reinforcements, "@0x8d4b9-0x8d4d2");
        assert!(waited.turns_on_player, "made before the loop");
        assert!(
            waited.clears_target && !waited.keeps_pursuit,
            "target -1, sub 0"
        );
        assert_eq!(waited.travel, TravelWrite::Clear, "+0x6c -1, @0x8c70d");
        let lone = [ship(1, 1, Goal::Idle), victim()];
        let mut decisions = vec![Some(waiting_on(n(9))), None];
        resolve(&lone, &mut decisions);
        assert_eq!(
            decisions[0],
            Some(waiting_on(n(9))),
            "no boarder: untouched"
        );
    }

    #[test]
    fn a_boarder_plunders_or_snipes_the_victim_any_way_and_nothing_else_counts() {
        let counts = |boarder: Npc| {
            let npcs = [ship(1, 1, Goal::Idle), boarder, victim()];
            resolved(&npcs, vec![Some(waiting_on(n(9))), None, None])[0] == Some(WAIT)
        };
        assert!(counts(ship(2, 2, plunder(n(9)))));
        assert!(counts(ship(2, 2, Goal::Snipe(n(9)))), "sub-goal 0xe");
        assert!(counts(ship(
            2,
            2,
            Goal::Plunder {
                ship: n(9),
                hopeless: true
            }
        )));
        assert!(!counts(ship(2, 2, Goal::Attack(n(9)))), "goal 4 in no 0xe");
        assert!(!counts(ship(2, 2, Goal::Flee(n(9)))));
        let mut band = ship(2, 2, Goal::Attack(n(9)));
        band.pursuit = Pursuit {
            kept: None,
            frame: Some(Frame {
                goal: Goal::Attack(n(9)),
                bodies: [
                    Some(Body {
                        manoeuvre: Manoeuvre::Snipe,
                        heading: 0,
                    }),
                    None,
                ],
                speed_cap: None,
            }),
        };
        assert!(counts(band.clone()), "a stand-off ship in its band");
        band.pursuit.frame = band.pursuit.frame.map(|frame| Frame {
            goal: Goal::Attack(n(8)),
            ..frame
        });
        assert!(!counts(band), "a frame of another goal is stale");
    }

    #[test]
    fn a_docked_pirate_whose_victim_another_boards_waits_and_drops_its_plunder() {
        let npcs = [ship(1, 1, Goal::Idle), ship(2, 2, plunder(n(9))), victim()];
        let docked = Decision {
            goal: Goal::Drift,
            plunders: Some(n(9)),
            ..waiting_on(n(9))
        };
        let mut decisions = vec![Some(docked), None, None];
        resolve(&npcs, &mut decisions);
        let waited = decisions[0].expect("decided");
        assert_eq!((waited.goal, waited.plunders), (WAIT, None));
        let lone = [ship(1, 1, Goal::Idle), victim()];
        let mut decisions = vec![Some(docked), None];
        resolve(&lone, &mut decisions);
        assert_eq!(decisions[0], Some(docked), "no boarder: it plunders");
    }

    #[test]
    fn a_pirate_docked_with_the_victim_is_no_boarder() {
        // Goal 4 with sub 0xf: neither goal 0xd nor sub-goal 0xe.
        let mut docked = ship(2, 2, Goal::Attack(n(9)));
        docked.docking = Some(crate::traffic::plunder_dock::Docking {
            ship: n(9),
            sub: crate::traffic::plunder_dock::DockSub::Dock,
        });
        docked.ai_wait = 50;
        let npcs = [ship(1, 1, Goal::Idle), docked, victim()];
        assert_eq!(
            resolved(&npcs, vec![Some(waiting_on(n(9))), None, None])[0],
            Some(Goal::Attack(n(9)))
        );
    }

    #[test]
    fn each_gate_of_the_loop_turns_a_boarder_away() {
        let counts = |boarder: Npc| {
            let npcs = [ship(1, 1, Goal::Idle), boarder, victim()];
            resolved(&npcs, vec![Some(waiting_on(n(9))), None, None])[0] == Some(WAIT)
        };
        let boarder = || ship(2, 2, plunder(n(9)));
        let mut led = boarder();
        led.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        assert!(!counts(led), "led by the player (+0x9a 0)");
        let mut fleet = boarder();
        fleet.leader = Some(NpcId(7));
        assert!(counts(fleet), "led by an NPC");
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 2,
                starting: false,
            },
        ] {
            let mut down = boarder();
            down.condition = condition;
            assert!(!counts(down), "{condition:?}");
        }
        let mut elsewhere = boarder();
        elsewhere.target = Some(n(8));
        assert!(!counts(elsewhere), "+0x70 another ship");
        // The victim itself, were it to plunder itself.
        let mut selfish = victim();
        selfish.condition = Condition::Intact;
        selfish.goal = plunder(n(9));
        selfish.target = Some(n(9));
        let npcs = [ship(1, 1, Goal::Idle), selfish];
        assert_eq!(
            resolved(&npcs, vec![Some(waiting_on(n(9))), None])[0],
            Some(Goal::Attack(n(9)))
        );
        // The pirate itself, plundering last tick.
        let npcs = [ship(1, 1, plunder(n(9))), victim()];
        assert_eq!(
            resolved(&npcs, vec![Some(waiting_on(n(9))), None])[0],
            Some(Goal::Attack(n(9)))
        );
    }

    #[test]
    fn a_lower_berth_that_did_not_think_is_read_as_it_was_and_one_that_did_by_its_decision() {
        let npcs = [ship(1, 2, Goal::Idle), ship(2, 1, plunder(n(9))), victim()];
        assert_eq!(
            resolved(&npcs, vec![Some(waiting_on(n(9))), None, None])[0],
            Some(WAIT),
            "no think: as the tick found it"
        );
        let dropped = Some(Decision {
            goal: Goal::Idle,
            clears_target: true,
            ..Decision::default()
        });
        assert_eq!(
            resolved(&npcs, vec![Some(waiting_on(n(9))), dropped, None])[0],
            Some(Goal::Attack(n(9))),
            "its think dropped the plunder"
        );
        let mut higher = npcs.clone();
        higher[1].berth = 3;
        assert_eq!(
            resolved(&higher, vec![Some(waiting_on(n(9))), dropped, None])[0],
            Some(WAIT),
            "a higher berth is read as the tick found it"
        );
        let took_up = Some(Decision {
            goal: plunder(n(9)),
            ..Decision::default()
        });
        let idle = [ship(1, 2, Goal::Idle), ship(2, 1, Goal::Idle), victim()];
        assert_eq!(
            resolved(&idle, vec![Some(waiting_on(n(9))), took_up, None])[0],
            Some(WAIT),
            "its think took up the plunder"
        );
    }

    /// NPC `id` in berth `id`, in its count wait with `ticks` left.
    fn waiting(id: u32, ticks: u32) -> Npc {
        let mut ship = ship(id, id, Goal::Idle);
        count_waiting(&mut ship, ticks);
        ship
    }

    #[test]
    fn a_count_wait_lapses_at_none_left_on_a_think() {
        let mut npcs = vec![waiting(1, 0), waiting(2, 0), waiting(3, 1)];
        npcs.push(ship(4, 4, Goal::Hold));
        lapse(&mut npcs, &[true, false, true, true]);
        let goals: Vec<Goal> = npcs.iter().map(|npc| npc.goal).collect();
        assert_eq!(
            goals,
            [Goal::Idle, Goal::CountWait, Goal::CountWait, Goal::Hold],
            "another goal at none left is no count wait to end"
        );
    }

    #[test]
    fn a_ship_taking_goal_0x16_waits_120_and_one_already_waiting_keeps_its_count() {
        let mut idle = ship(1, 1, Goal::Idle);
        enter(&mut idle, Goal::CountWait);
        assert_eq!(idle.ai_wait, BOARD_WAIT, "+0x4c = 120 (@0x8c71c)");
        let mut kept = waiting(2, 7);
        kept.provoked = 5.0;
        enter(&mut kept, Goal::CountWait);
        assert_eq!(kept.ai_wait, 7, "its think keeps goal 0x16");
        assert_eq!(kept.provoked, 5.0, "@0x8c713 runs only on the way in");
        let mut turned = waiting(3, 7);
        enter(&mut turned, Goal::Attack(ShipRef::Player));
        assert_eq!(turned.ai_wait, 7, "a new goal writes no +0x4c");
        let mut held = ship(4, 4, Goal::Hold);
        held.ai_wait = 9;
        enter(&mut held, Goal::Idle);
        assert_eq!(held.ai_wait, 9);
    }

    /// A pirate stand-in: it keeps a count wait, waits on NPC 9 while it
    /// plunders it, and otherwise leaves (its "fresh" decision), holding
    /// its target; its target is its quarry or the one it holds.
    #[derive(Debug)]
    struct Waiter;

    impl Behaviour for Waiter {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.decision(npc, around, chance).goal
        }

        fn decision(
            &self,
            npc: &Npc,
            _around: &Surroundings,
            _chance: &mut dyn Chance,
        ) -> Decision {
            match npc.goal {
                Goal::CountWait => Decision {
                    goal: npc.goal,
                    ..Decision::default()
                },
                Goal::Plunder { ship, .. } => Decision {
                    goal: Goal::Attack(ship),
                    board_wait: Some(ship),
                    calls_reinforcements: true,
                    ..Decision::default()
                },
                _ => Decision {
                    goal: Goal::JumpOut,
                    ..Decision::default()
                },
            }
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            match npc.goal {
                Goal::CountWait => None,
                goal => goal.quarry().or(npc.target),
            }
        }
    }

    /// The traffic of `npcs`, deciding every tick.
    fn traffic_of(npcs: Vec<Npc>) -> Traffic {
        let mut traffic = Traffic::with_interval(1);
        traffic.npcs = npcs;
        traffic
    }

    /// `traffic` a tick on by `behaviour`, among no stellars.
    fn tick(traffic: &mut Traffic, behaviour: &dyn Behaviour) {
        traffic.tick_in(behaviour, World::new(&[]), &[], &mut Draws::of(&[0; 8]));
    }

    #[test]
    fn the_traffic_runs_the_wait_loop_in_berth_order_and_its_calls_follow_it() {
        let provoked = |mut npc: Npc| {
            npc.provoked = 5.0;
            npc
        };
        let mut traffic = traffic_of(vec![
            provoked(ship(1, 2, plunder(n(9)))),
            provoked(ship(2, 1, plunder(n(9)))),
            victim(),
        ]);
        tick(&mut traffic, &Waiter);
        let provocations: Vec<f32> = traffic.npcs()[..2].iter().map(|npc| npc.provoked).collect();
        assert_eq!(provocations, [5.0, 0.0], "+0x96 0 as it waits (@0x8c713)");
        let goals: Vec<Goal> = traffic.npcs().iter().map(|npc| npc.goal).collect();
        assert_eq!(
            goals[..2],
            [Goal::Attack(n(9)), Goal::CountWait],
            "berth 1 waits"
        );
        assert_eq!(
            traffic.npcs()[1].ai_wait,
            BOARD_WAIT - 1,
            "its tick counted down as it flew"
        );
        assert_eq!(traffic.npcs()[0].ai_wait, 0, "the attacker waits none");
        assert_eq!(traffic.npcs()[1].target, None);
        assert_eq!(
            crate::traffic::fixture::calls_on_line(&traffic.take_on_line()),
            [(NpcId(1), CommCall::Reinforcements, 0.0)],
            "the waiting pirate calls for none"
        );
    }

    #[test]
    fn a_pirate_already_in_its_count_wait_thinks_again_and_keeps_its_wait_and_provocation() {
        let mut kept = waiting(1, 50);
        kept.provoked = 5.0;
        let mut traffic = traffic_of(vec![kept]);
        tick(&mut traffic, &Waiter);
        let npc = &traffic.npcs()[0];
        assert_eq!((npc.goal, npc.ai_wait), (Goal::CountWait, 49));
        assert_eq!(npc.provoked, 5.0, "no second zero (@0x8c713)");
    }

    #[test]
    fn a_pirate_entering_its_wait_drops_its_pursuit_though_its_wait_runs() {
        // The loop sets sub 0 (@0x8c6f5-0x8c71c), and goal 0x16's block
        // runs ahead of the wait's return (@0x8d462-0x8d4b9): no kept
        // rush or frame survives into the wait.
        let mut fighting = ship(1, 1, plunder(n(9)));
        fighting.pursuit = Pursuit {
            kept: Some(Kept::Rush),
            frame: Some(Frame {
                goal: plunder(n(9)),
                bodies: [
                    Some(Body {
                        manoeuvre: Manoeuvre::Snipe,
                        heading: 0,
                    }),
                    None,
                ],
                speed_cap: None,
            }),
        };
        let mut traffic = traffic_of(vec![fighting, ship(2, 2, plunder(n(9))), victim()]);
        tick(&mut traffic, &Waiter);
        let waiting = &traffic.npcs()[0];
        assert_eq!(waiting.goal, Goal::CountWait);
        assert_eq!(waiting.ai_wait, BOARD_WAIT - 1);
        assert_eq!(waiting.pursuit, Pursuit::default());
    }

    #[test]
    fn a_count_wait_holds_120_ticks_and_the_pirate_decides_afresh_on_the_next() {
        // The tick that set it has counted one off.
        let mut traffic = traffic_of(vec![waiting(1, BOARD_WAIT - 1)]);
        for tick_after in 1..BOARD_WAIT {
            tick(&mut traffic, &Waiter);
            let npc = &traffic.npcs()[0];
            assert_eq!(
                (npc.goal, npc.ai_wait),
                (Goal::CountWait, BOARD_WAIT - 1 - tick_after),
                "tick {tick_after} after"
            );
        }
        tick(&mut traffic, &Waiter);
        assert_eq!(traffic.npcs()[0].goal, Goal::Idle, "the 120th: goal 0");
        tick(&mut traffic, &Waiter);
        assert_eq!(
            traffic.npcs()[0].goal,
            Goal::JumpOut,
            "the 121st: it thinks again"
        );
    }

    #[test]
    fn a_count_wait_counts_down_on_double_times_second_pass_too_and_lapses_only_on_a_think() {
        let mut traffic = traffic_of(vec![waiting(1, 1)]);
        traffic.move_in(World::new(&[]), &mut Draws::of(&[]));
        let npc = &traffic.npcs()[0];
        assert_eq!((npc.goal, npc.ai_wait), (Goal::CountWait, 0));
        traffic.move_in(World::new(&[]), &mut Draws::of(&[]));
        assert_eq!(
            traffic.npcs()[0].goal,
            Goal::CountWait,
            "no think on the second pass"
        );
    }

    /// Leaves its target set, but its think cleared it or not as
    /// `clears` says; drops its fight keeping the pursuit as `keeps`
    /// says.
    #[derive(Debug)]
    struct Dropping {
        clears: bool,
        keeps: bool,
    }

    impl Behaviour for Dropping {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }

        fn decision(
            &self,
            _npc: &Npc,
            _around: &Surroundings,
            _chance: &mut dyn Chance,
        ) -> Decision {
            Decision {
                goal: Goal::Idle,
                clears_target: self.clears,
                keeps_pursuit: self.keeps,
                ..Decision::default()
            }
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    #[test]
    fn the_traffic_clears_a_target_and_keeps_a_pursuit_as_the_decision_says() {
        for (clears, keeps) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut pirate = ship(1, 1, Goal::Attack(n(9)));
            pirate.pursuit.kept = Some(Kept::Rush);
            let mut traffic = traffic_of(vec![pirate, victim()]);
            tick(&mut traffic, &Dropping { clears, keeps });
            let npc = &traffic.npcs()[0];
            assert_eq!(npc.target, (!clears).then_some(n(9)), "{clears} {keeps}");
            assert_eq!(
                npc.pursuit.kept,
                keeps.then_some(Kept::Rush),
                "{clears} {keeps}"
            );
            assert_eq!(npc.pursuit.frame, None, "no fight to fly");
        }
    }
}
