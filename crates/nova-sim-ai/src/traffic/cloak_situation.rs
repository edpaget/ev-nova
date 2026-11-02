//! What an NPC's cloak think knows of it ([`situation`]): the engine's
//! goals, modes and warps read as the original's goals and sub-goals that
//! `_AIHandleCloaking` (@0x833e1-0x836a3) tests, for
//! [`cloak::think`].
//!
//! The original's goal 8 is a hyperspace arrival's glide
//! (`_AIMakeShipJumpIn` @0x82ebb, written again each frame while +0x50 is
//! below -900, `_AIDispatch` @0x8fd08) and a gate burst from its first
//! moving tick (@0x8dd8a): here [`Mode::JumpingIn`] and
//! [`Mode::LeavingGate`] past tick 0. Tick 0 of a burst is still goal
//! 0x15, which no term tests. While either holds, the goal the NPC keeps
//! here (its [`Npc::goal`]) is not the original's, so no goal term but
//! the jump's reads it.
//!
//! Goal 0xb, a jump with its parent (`_AIMakeShipJumpWithParent`, which
//! replaces goal 0xa), is handled like goal 8: here a
//! [`WarpGoal::Follow`] warp, whose kept [`Npc::goal`] (an escort's
//! formation, or a stale attack) no goal term but the jump's reads. Its
//! own jump ([`WarpGoal::Own`], sub-goal 4) keeps its goal, as the
//! original's sub-goal does.

use crate::ai::Goal;
use crate::ai::regroup::Kept;
use crate::area_cloak;
use crate::cloak::{self, Cloak};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::traffic::gone::GoneShip;
use crate::traffic::npc::{Mode, Npc};
use crate::traffic::warp_out::WarpGoal;

/// What the think reads around an NPC, besides the NPC itself.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Around {
    /// Where the ship its goal attacks is from it, if it attacks one.
    pub attacking: Option<Vec2>,
    /// Its leader's cloak, if it has a leader here.
    pub leader: Option<Cloak>,
    /// Whether its leader carries an area cloak
    /// ([`CloakGear::area`](crate::cloak::CloakGear::area), or the
    /// player's [`PlayerSide::area_cloak`](crate::ai::PlayerSide::area_cloak)).
    pub leader_area: bool,
    /// How the `cloak_on_reload` rule reads a reloading weapon
    /// ([`RuleKey::CloakOnReload`](crate::RuleKey::CloakOnReload)).
    pub cloak_on_reload: RuleSource,
}

/// What the cloak think of `npc`, with `around`, knows
/// ([`cloak::Situation`]).
///
/// - Goal 8 ([`Mode::JumpingIn`], [`Mode::LeavingGate`] past tick 0),
///   its own jump (sub-goal 4,
///   [`WarpGoal::Own`](crate::traffic::warp_out::WarpGoal::Own)) or a
///   jump with its parent (goal 0xb,
///   [`WarpGoal::Follow`](crate::traffic::warp_out::WarpGoal::Follow), or
///   goal 0xb written under a wait, [`Npc::joins_leaders_jump`]) is
///   [`jumping`](cloak::Situation::jumping) (@0x834ee-0x83509). The regroup
///   pause keeping sub 4 is too, as the term reads sub 4 whatever the goal;
///   keeping sub 0xd it isn't, as goal 0x13 is neither 8 nor 0xb.
/// - Landing, waiting, entering a gate, inspecting, or flying in form
///   (goal 0xa: [`Goal::Follow`], [`Goal::Formation`] or [`Goal::Hold`])
///   is [`travelling`](cloak::Situation::travelling).
/// - [`Goal::Idle`] is [`idle`](cloak::Situation::idle).
/// - [`Goal::Mine`] (goal 0x10) mines on the order its last flight stored
///   ([`Npc::mine_order`], the sub-goal +0xc8ca the terms read at
///   @0x834d6 and @0x835e9-0x835fb): order 1
///   ([`MineOrder::Stop`](cloak::MineOrder::Stop)) with slot 0 empty,
///   0x13 ([`MineOrder::Chase`](cloak::MineOrder::Chase)) outside the
///   box, 0x14 ([`MineOrder::Close`](cloak::MineOrder::Close)) inside it.
///   [`Goal::Scoop`] (goal 0x11) mines on nothing, as no term tests goal
///   0x11.
/// - Its last flight kept close formation ([`Npc::close_form`], the
///   stored sub-goal 0xc, which the parent rule reads directly at
///   @0x83633-0x8366d: the think runs before the high level writes it,
///   @0x8fcaa before @0x901e5) with its leader cloaked:
///   [`close_to_cloaked_parent`](cloak::Situation::close_to_cloaked_parent),
///   for a ship of AI type above 4 alone ([`Npc::original_ai_type`],
///   `cmpw $4, 0x88` @0x83633, ahead of the parent and sub-goal tests).
///   A bribed led ship (+0x88 1, `_DoCommDialog` @0x961f8) fails it,
///   though its last flight's close formation still stands on its first
///   think after the bribe. Outside goal 8 and 0xb.
/// - That, with its leader carrying an area cloak, is a ride on it
///   ([`area_cloak::rides`],
///   [`area_cloak`](cloak::Situation::area_cloak)): `_HasCloak`'s area
///   branch (@0x964c-0x96bf), which tests no goal, so a glide in keeps it.
#[must_use]
pub fn situation(npc: &Npc, around: Around) -> cloak::Situation {
    let arriving = match npc.mode {
        Mode::JumpingIn { .. } => true,
        Mode::LeavingGate { tick } => tick > 0,
        _ => false,
    };
    let following_jump =
        matches!(npc.warp.goal, Some(WarpGoal::Follow(_))) || npc.joins_leaders_jump();
    let goal = match npc.mode {
        Mode::JumpingIn { .. } | Mode::LeavingGate { .. } | Mode::Emerging { .. } => None,
        _ if following_jump => None,
        _ => Some(npc.goal),
    };
    let leader_cloaked = around.leader.is_some_and(Cloak::is_cloaked);
    let close_to_cloaked_parent =
        goal.is_some() && npc.original_ai_type() > 4 && npc.close_form && leader_cloaked;
    cloak::Situation {
        disabled: npc.condition == Condition::Disabled,
        retreating: matches!(goal, Some(Goal::Flee(_))),
        attacking: around.attacking.filter(|_| goal.is_some()),
        reloading: npc.armament.reloading(around.cloak_on_reload),
        jumping: arriving
            || npc.joins_leaders_jump()
            || (npc.warp.goal.is_some()
                && npc.goal
                    != Goal::Regroup {
                        kept: Kept::FollowsJump,
                    }),
        travelling: matches!(
            goal,
            Some(
                Goal::Land(_)
                    | Goal::Wait
                    | Goal::EnterGate(_)
                    | Goal::Inspect(_)
                    | Goal::Follow { .. }
                    | Goal::Formation { .. }
                    | Goal::Hold
            )
        ),
        idle: goal == Some(Goal::Idle),
        mining: (goal == Some(Goal::Mine)).then(|| npc.mine_order.into()),
        close_to_cloaked_parent,
        area_cloak: area_cloak::rides(npc.close_form, leader_cloaked, around.leader_area),
    }
}

/// The ship whose place the 0x1000 term of `npc`'s cloak think reads
/// (`_AIHandleCloaking` @0x8355e-0x835d7): on goal 4 ([`Goal::attacking`]),
/// the ship its target (+0x70) names, not its goal's, as it stands among
/// `npcs` or as `player`, else the record of a gone ship it still holds
/// among `gone` ([`GoneShip::held_by`]), as the term reads the slot's record
/// with no +0xb8 test; none on any other goal or with no target.
#[must_use]
pub fn pressed(
    npc: &Npc,
    npcs: &[Npc],
    gone: &[GoneShip],
    player: Option<ShipState>,
) -> Option<ShipState> {
    npc.goal.attacking()?;
    match npc.target? {
        ShipRef::Player => player,
        ShipRef::Npc(id) => npcs
            .iter()
            .find(|other| other.id == id)
            .or_else(|| {
                let held = gone.iter().find(|gone| gone.held_by(npc))?;
                Some(&held.ship)
            })
            .map(|other| other.state),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::catalog::StellarId;
    use crate::cloak::FADE_FULL;
    use crate::cloak::MineOrder;
    use crate::combat::ShipRef;
    use crate::combat::armament::Armament;
    use crate::combat::weapon::WeaponSpec;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::stats::ShipStats;
    use crate::traffic::fixture;
    use crate::traffic::npc::NpcId;
    use crate::traffic::warp_out::Warp;
    use nova_sim_catalog::catalog::WeaponRecord;
    use nova_sim_catalog::fixture::weapon;

    fn npc(goal: Goal, mode: Mode) -> Npc {
        Npc {
            mode,
            goal,
            ..fixture::npc(1, ShipStats::default())
        }
    }

    fn of(goal: Goal, mode: Mode) -> cloak::Situation {
        situation(&npc(goal, mode), Around::default())
    }

    const FLYING: Mode = Mode::Flying;

    #[test]
    fn the_goals_read_as_the_originals() {
        let none = cloak::Situation::default();
        let travelling = cloak::Situation {
            travelling: true,
            ..none
        };
        let rows = [
            (Goal::Idle, cloak::Situation { idle: true, ..none }),
            (Goal::Land(StellarId(128)), travelling),
            (Goal::Wait, travelling),
            (Goal::EnterGate(StellarId(128)), travelling),
            (Goal::Inspect(ShipRef::Player), travelling),
            (
                Goal::Follow {
                    lead: NpcId(2),
                    guard: None,
                },
                travelling,
            ),
            (Goal::Formation { guard: None }, travelling),
            (Goal::Hold, travelling),
            (
                Goal::Flee(ShipRef::Player),
                cloak::Situation {
                    retreating: true,
                    ..none
                },
            ),
            (Goal::Attack(ShipRef::Player), none),
            (Goal::JumpOut, none),
            (
                Goal::Mine,
                cloak::Situation {
                    mining: Some(MineOrder::Stop),
                    ..none
                },
            ),
        ];
        for (goal, expected) in rows {
            assert_eq!(of(goal, FLYING), expected, "{goal:?}");
        }
    }

    #[test]
    fn a_glide_in_and_a_gate_burst_past_its_first_tick_are_goal_8() {
        let jumping = cloak::Situation {
            jumping: true,
            ..cloak::Situation::default()
        };
        for goal in [Goal::Idle, Goal::Hold, Goal::Flee(ShipRef::Player)] {
            assert_eq!(of(goal, Mode::JumpingIn { tick: 0 }), jumping, "{goal:?}");
            assert_eq!(of(goal, Mode::JumpingIn { tick: 9 }), jumping);
            assert_eq!(of(goal, Mode::LeavingGate { tick: 1 }), jumping);
            assert_eq!(
                of(goal, Mode::LeavingGate { tick: 0 }),
                cloak::Situation::default(),
                "tick 0 is still goal 0x15"
            );
            assert_eq!(
                of(goal, Mode::Emerging { ticks_left: 3 }),
                cloak::Situation::default(),
            );
        }
        let attacker = situation(
            &npc(Goal::Attack(ShipRef::Player), Mode::JumpingIn { tick: 0 }),
            Around {
                attacking: Some(Vec2::new(500.0, 0.0)),
                ..Around::default()
            },
        );
        assert_eq!(attacker.attacking, None, "goal 8 attacks nothing");
    }

    #[test]
    fn goal_0xb_written_under_a_wait_is_jumping_whatever_substate_stands() {
        // The term reads goal 0xb (@0x83505), and goal 0xb is no goal of
        // the travelling or idle terms.
        for goal in [
            Goal::Formation { guard: None },
            Goal::Follow {
                lead: NpcId(2),
                guard: None,
            },
        ] {
            let mut joining = npc(goal, FLYING);
            joining.warp.joining = true;
            let read = situation(&joining, Around::default());
            assert!(read.jumping, "{goal:?}");
            assert!(!read.travelling, "{goal:?}");
        }
    }

    #[test]
    fn its_own_jump_and_a_jump_with_its_parent_are_jumping() {
        for warp in [WarpGoal::Own, WarpGoal::Follow(ShipRef::Npc(NpcId(2)))] {
            let jumper = Npc {
                warp: Warp {
                    goal: Some(warp),
                    ..Warp::default()
                },
                ..npc(Goal::JumpOut, FLYING)
            };
            assert!(situation(&jumper, Around::default()).jumping, "{warp:?}");
        }
        assert!(!of(Goal::JumpOut, FLYING).jumping);
    }

    #[test]
    fn of_a_regroup_pause_only_one_keeping_its_own_jump_is_jumping() {
        // The term is sub 4 whatever the goal, or goal 8 or 0xb
        // (@0x834ee-0x83509): goal 0x13 keeping sub 0xd is none of them.
        let paused = |kept, warp| Npc {
            warp: Warp {
                goal: Some(warp),
                ..Warp::default()
            },
            ..npc(Goal::Regroup { kept }, FLYING)
        };
        let own = paused(Kept::OwnJump, WarpGoal::Own);
        assert!(situation(&own, Around::default()).jumping);
        let follows = paused(Kept::FollowsJump, WarpGoal::Follow(ShipRef::Player));
        assert!(!situation(&follows, Around::default()).jumping);
    }

    fn following_jump(goal: Goal) -> Npc {
        Npc {
            warp: Warp {
                goal: Some(WarpGoal::Follow(ShipRef::Player)),
                ..Warp::default()
            },
            ..npc(goal, FLYING)
        }
    }

    #[test]
    fn a_jump_with_its_parent_is_goal_0xb_and_reads_no_goal_term() {
        let jumping = cloak::Situation {
            jumping: true,
            ..cloak::Situation::default()
        };
        for goal in [
            Goal::Formation { guard: None },
            Goal::Follow {
                lead: NpcId(2),
                guard: None,
            },
            Goal::Hold,
            Goal::Idle,
            Goal::Flee(ShipRef::Player),
        ] {
            assert_eq!(
                situation(&following_jump(goal), Around::default()),
                jumping,
                "{goal:?}"
            );
        }
        let attacker = situation(
            &following_jump(Goal::Attack(ShipRef::Player)),
            Around {
                attacking: Some(Vec2::new(200.0, 0.0)),
                ..Around::default()
            },
        );
        assert_eq!(attacker, jumping, "goal 0xb attacks nothing");
    }

    #[test]
    fn the_pressing_term_reads_the_target_on_goal_4_and_a_gone_ones_record() {
        use crate::traffic::gone::Freed;
        let at = |id, berth, x| Npc {
            berth,
            state: ShipState {
                position: Vec2::new(x, 0.0),
                ..ShipState::default()
            },
            ..fixture::npc(id, ShipStats::default())
        };
        let (live, other, left) = (at(2, 3, 300.0), at(3, 4, 400.0), at(4, 5, 500.0));
        let npcs = [live.clone(), other.clone()];
        let gone = [GoneShip {
            ship: left.clone(),
            freed: Freed::InShipPass,
        }];
        let player = Some(ShipState::default());
        let reader = |goal, target, berth| Npc {
            goal,
            target: Some(target),
            berth,
            ..fixture::npc(1, ShipStats::default())
        };
        let read = |npc: &Npc| pressed(npc, &npcs, &gone, player).map(|s| s.position.x);
        let (n, p) = (|id| ShipRef::Npc(NpcId(id)), ShipRef::Player);
        // +0x70, not the goal's ship (@0x8355e).
        assert_eq!(read(&reader(Goal::Attack(n(2)), n(3), 1)), Some(400.0));
        assert_eq!(read(&reader(Goal::Snipe(n(2)), n(2), 1)), Some(300.0));
        assert_eq!(read(&reader(Goal::Attack(p), p, 1)), Some(0.0));
        // The record of a slot it still holds, with no +0xb8 test.
        assert_eq!(read(&reader(Goal::Attack(n(4)), n(4), 1)), Some(500.0));
        assert_eq!(read(&reader(Goal::Attack(n(4)), n(4), 6)), None, "let go");
        assert_eq!(read(&reader(Goal::Flee(n(2)), n(2), 1)), None, "not goal 4");
        let none = Npc {
            target: None,
            ..reader(Goal::Attack(n(2)), n(2), 1)
        };
        assert_eq!(read(&none), None, "+0x70 -1");
    }

    #[test]
    fn a_miner_reads_the_order_its_last_flight_stored() {
        use crate::ai::mine_orders::MineOrder as Flown;
        let mining = |goal: Goal, order: Flown| {
            let miner = Npc {
                mine_order: order,
                ..npc(goal, FLYING)
            };
            situation(&miner, Around::default()).mining
        };
        assert_eq!(mining(Goal::Mine, Flown::Stop), Some(MineOrder::Stop));
        assert_eq!(mining(Goal::Mine, Flown::Chase), Some(MineOrder::Chase));
        assert_eq!(mining(Goal::Mine, Flown::Kill), Some(MineOrder::Close));
        // Goal 0x11 (order 0x15): no term tests it.
        assert_eq!(mining(Goal::Scoop, Flown::Chase), None, "scoop");
        assert_eq!(mining(Goal::Idle, Flown::Chase), None, "not goal 0x10");
    }

    #[test]
    fn disabled_and_attacking_are_carried_over() {
        let disabled = Npc {
            condition: Condition::Disabled,
            ..npc(Goal::Idle, FLYING)
        };
        assert!(situation(&disabled, Around::default()).disabled);
        assert!(!of(Goal::Idle, FLYING).disabled);
        let off = Some(Vec2::new(200.0, 0.0));
        let attacker = situation(
            &npc(Goal::Attack(ShipRef::Player), FLYING),
            Around {
                attacking: off,
                ..Around::default()
            },
        );
        assert_eq!(attacker.attacking, off);
    }

    #[test]
    fn reloading_is_read_by_the_rule() {
        // `Reload` 15 with a burst reload of 15: past `Reload` only by
        // the Bible's reading.
        let spec = WeaponSpec::new(&WeaponRecord {
            reload: 15,
            burst_count: 15,
            burst_reload: 15,
            ..weapon(146)
        });
        let mut armament = Armament::new([(spec, 1)]);
        let class = armament.clone();
        armament.force_burst_reload(&class);
        let reloading = Npc {
            armament,
            ..npc(Goal::Idle, FLYING)
        };
        let by = |rule| {
            situation(
                &reloading,
                Around {
                    cloak_on_reload: rule,
                    ..Around::default()
                },
            )
            .reloading
        };
        assert!(!by(RuleSource::Engine));
        assert!(by(RuleSource::Bible));
    }

    /// The player's escort, its last flight in close formation as
    /// `close_form` says.
    fn escort(close_form: bool) -> Npc {
        Npc {
            escort: Some(EscortDuty {
                formation: FormationSlot {
                    slot: 0,
                    ships: 1,
                    spacing: 30.0,
                },
                order: None,
            }),
            close_form,
            ..npc(Goal::Formation { guard: None }, FLYING)
        }
    }

    const CLOAKED: Cloak = Cloak {
        fade: FADE_FULL,
        direction: 0,
    };

    #[test]
    fn the_parent_rule_reads_the_stored_close_formation() {
        // `_AIHandleCloaking` runs before the high level writes +0xc8ca
        // (@0x8fcaa before @0x901e5), so it reads the last flight's.
        let with = |npc: &Npc, cloak| {
            situation(
                npc,
                Around {
                    leader: Some(cloak),
                    ..Around::default()
                },
            )
            .close_to_cloaked_parent
        };
        assert!(with(&escort(true), CLOAKED));
        assert!(!with(&escort(true), Cloak::default()), "leader uncloaked");
        assert!(!with(&escort(false), CLOAKED), "its last flight out of it");
        assert!(
            !situation(&escort(true), Around::default()).close_to_cloaked_parent,
            "no leader"
        );
        let gliding = Npc {
            mode: Mode::JumpingIn { tick: 0 },
            ..escort(true)
        };
        assert!(!with(&gliding, CLOAKED), "goal 8 reads no goal term");
    }

    /// A ship NPC 7 leads, its last flight in close formation.
    fn led_close() -> Npc {
        Npc {
            leader: Some(crate::traffic::npc::NpcId(7)),
            close_form: true,
            ..npc(Goal::Formation { guard: None }, FLYING)
        }
    }

    #[test]
    fn a_bribed_led_ship_in_close_formation_does_not_cloak_with_its_cloaked_leader() {
        // `cmpw $4, 0x88; jle` ahead of the parent rule (@0x83633), and
        // the bribe's word is 1 (`_DoCommDialog` @0x961f8); its last
        // flight's close formation still stands on its first think after.
        let around = Around {
            leader: Some(CLOAKED),
            ..Around::default()
        };
        assert!(situation(&led_close(), around).close_to_cloaked_parent);
        let mut bribed = led_close();
        bribed.take_bribe();
        assert!(!situation(&bribed, around).close_to_cloaked_parent);
        // A word of 4 fails the `jle` too.
        bribed.ai_type = crate::traffic::npc::AiType::Interceptor;
        assert_eq!(bribed.original_ai_type(), 4);
        assert!(!situation(&bribed, around).close_to_cloaked_parent);
    }

    #[test]
    fn a_bribed_led_ship_in_close_formation_still_rides_its_leaders_area_cloak() {
        // `_HasCloak`'s area branch reads +0x9a, +0xc8ca and the parent's
        // area cloak, no +0x88 (@0x964c-0x96bf).
        let mut bribed = led_close();
        bribed.take_bribe();
        let around = Around {
            leader: Some(CLOAKED),
            leader_area: true,
            ..Around::default()
        };
        assert!(situation(&bribed, around).area_cloak);
    }

    #[test]
    fn an_escort_riding_its_leaders_area_cloak_has_a_cloak() {
        let rides = |npc: &Npc, cloak, leader_area| {
            situation(
                npc,
                Around {
                    leader: Some(cloak),
                    leader_area,
                    ..Around::default()
                },
            )
            .area_cloak
        };
        assert!(rides(&escort(true), CLOAKED, true));
        assert!(!rides(&escort(true), CLOAKED, false), "no area cloak");
        assert!(!rides(&escort(true), Cloak::default(), true), "uncloaked");
        assert!(
            !rides(&escort(false), CLOAKED, true),
            "out of close formation"
        );
        let gliding = Npc {
            mode: Mode::JumpingIn { tick: 0 },
            ..escort(true)
        };
        assert!(
            rides(&gliding, CLOAKED, true),
            "the stored sub-goal survives a glide in"
        );
        assert!(
            !situation(&escort(true), Around::default()).area_cloak,
            "no leader"
        );
    }
}
