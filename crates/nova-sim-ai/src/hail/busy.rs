//! Whether a hailed ship is busy, as `_AIIsShipBusy` finds it, and
//! whether another NPC attacks it, the engine's earlier reading of
//! `_IsShipThreatened`. Request Assistance reads both (`_DoCommDialog`
//! @0x9625d-0x9630b, their only caller; see
//! [`RequestAssistance`](super::nova::RequestAssistance)).
//!
//! **`_AIIsShipBusy`** (@0x82fac-0x82fe4) reads only the goal `+0xc8c8`
//! (@0x82fb2): it is not busy in goals 0 and 2 (@0x82fb9-0x82fc8), 1
//! (@0x82fca), 0x14 (@0x82fd0) and 7 (@0x82fd6), and busy in every other.
//! It reads no leader, no condition and no other field, so goal 0xa is
//! busy with a leader or without. [`ai_is_ship_busy`] maps the engine's
//! state onto the original's goal:
//!
//! | Engine state | Original goal | Busy |
//! |---|---|---|
//! | [`Goal::Idle`] | 0 | no |
//! | [`Goal::Land`] | 1 | no |
//! | [`Goal::JumpOut`] | 2 | no |
//! | [`Goal::Inspect`] | 7 | no |
//! | [`Goal::EnterGate`] (in [`Mode::EnteringGate`] too) | 0x14 | no |
//! | [`Goal::Flee`] | 3 | yes |
//! | [`Goal::Attack`], [`Goal::Snipe`] | 4 | yes |
//! | [`Goal::Dock`] | 5 | yes |
//! | [`Goal::Wait`] | 6 | yes |
//! | [`Goal::Assist`] | 9 (refuel), 0xf (repair) | yes |
//! | [`Goal::Follow`], [`Goal::Formation`], [`Goal::Hold`] | 0xa | yes |
//! | [`Goal::Plunder`] | 0xd | yes |
//! | [`Goal::Drift`] | 0xe | yes |
//! | [`Goal::Mine`] | 0x10 | yes |
//! | [`Goal::Scoop`] | 0x11 | yes |
//! | [`Goal::AttackStellar`] | 0x12 | yes |
//! | [`Goal::Regroup`] | 0x13 | yes |
//! | [`Goal::CountWait`] | 0x16 | yes |
//! | [`Npc::follows_leaders_jump`], whatever its goal | 0xb | yes |
//! | [`Npc::jumping_in`] ([`Mode::JumpingIn`], [`Mode::LeavingGate`] from tick 1) | 8 | yes |
//! | [`Mode::Emerging`], [`Mode::LeavingGate`] at tick 0 | 0x15 | yes |
//! | [`Mode::HeldAtGate`], [`Mode::Coasting`], [`Mode::Adrift`] | its goal's | its goal's |
//!
//! A follower of its leader's jump that the hold put on its own jump or
//! a flee (goals 2 and 3) or the regroup pause (0x13) is not in goal
//! 0xb, so it is busy by its goal. Goal 0xc, a `ShipBehav` 1 mission ship
//! with no target (@0x89fe0), has no engine state: such a ship joins the
//! player's formation before its type's AI decides (see the session's
//! `mission_escorts` module). An escape pod decides nothing and takes no
//! goal (see [`pod_flight`](crate::traffic::pod_flight)), so it is busy
//! by its spawner's goal.
//!
//! **`_IsShipThreatened`** (@0x82068-0x820fc), %ebx the hailed ship
//! throughout: none if it is inactive (@0x82072) or `_IsDisabled`
//! (@0x8207e); then, over slots 1-0x3f, an active one counts if the
//! hailed ship's own `+0x70` equals its own slot `+0x86`
//! ([`Npc::berth`]) (@0x8209e-0x820a9) and differs from the loop's slot
//! (@0x820ab), and the slot's goal is none of 7, 9, 0xf, 0xa, 0xb, 5 and
//! 0xc (@0x820b0-0x820df). No ship targets itself, so it never holds: by
//! the original no hailed ship is busy for being attacked. [`attacked`]
//! is the engine's earlier reading, an NPC attacking or sniping at it,
//! which Request Assistance reads as busy only under
//! [`RuleKey::HailBusyUnderAttack`](crate::RuleKey::HailBusyUnderAttack)'s
//! other reading.
//!
//! Neither plays a sound.

use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::traffic::npc::{Mode, Npc};

/// Whether `npc` is busy, as `_AIIsShipBusy` (@0x82fac-0x82fe4) finds
/// its goal (see the module docs' table): busy in its [`Goal`] by
/// [`goal_busy`], following its leader's jump (goal 0xb), jumping in
/// (goal 8) or emerging from a gate (goal 0x15).
#[must_use]
pub(crate) fn ai_is_ship_busy(npc: &Npc) -> bool {
    goal_busy(npc.goal)
        || npc.follows_leaders_jump()
        || npc.jumping_in()
        || matches!(npc.mode, Mode::Emerging { .. } | Mode::LeavingGate { .. })
}

/// Whether a ship in `goal` is busy: every goal but the original's 0
/// ([`Goal::Idle`]), 1 ([`Goal::Land`]), 2 ([`Goal::JumpOut`]), 7
/// ([`Goal::Inspect`]) and 0x14 ([`Goal::EnterGate`]). The match names
/// every goal, so a goal added later must be placed to compile.
fn goal_busy(goal: Goal) -> bool {
    match goal {
        Goal::Idle | Goal::Land(_) | Goal::JumpOut | Goal::Inspect(_) | Goal::EnterGate(_) => false,
        Goal::Wait
        | Goal::Follow { .. }
        | Goal::Attack(_)
        | Goal::Snipe(_)
        | Goal::Flee(_)
        | Goal::Assist(_)
        | Goal::Formation { .. }
        | Goal::Dock(_)
        | Goal::Hold
        | Goal::Mine
        | Goal::Scoop
        | Goal::Plunder { .. }
        | Goal::CountWait
        | Goal::Drift
        | Goal::Regroup { .. }
        | Goal::AttackStellar(_) => true,
    }
}

/// Whether an NPC of `npcs` other than `npc` attacks or snipes at it
/// ([`Goal::attacking`]): the engine's earlier reading of
/// `_IsShipThreatened` (see the module docs).
#[must_use]
pub(crate) fn attacked(npc: &Npc, npcs: &[Npc]) -> bool {
    let me = ShipRef::Npc(npc.id);
    npcs.iter()
        .any(|other| other.id != npc.id && other.goal.attacking() == Some(me))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Help;
    use crate::ai::regroup::Kept;
    use crate::catalog::StellarId;
    use crate::traffic::npc::NpcId;
    use crate::traffic::warp_out::WarpGoal;

    /// NPC `id` flying `goal`.
    fn flying(id: u32, goal: Goal) -> Npc {
        let mut npc = crate::traffic::fixture::npc(id, crate::stats::ShipStats::default());
        npc.goal = goal;
        npc
    }

    const OTHER: ShipRef = ShipRef::Npc(NpcId(5));
    const PLANET: StellarId = StellarId(128);

    #[test]
    fn the_five_goals_of_ai_is_ship_busy_are_not_busy() {
        for (goal, original) in [
            (Goal::Idle, 0),
            (Goal::Land(PLANET), 1),
            (Goal::JumpOut, 2),
            (Goal::Inspect(OTHER), 7),
            (Goal::EnterGate(PLANET), 0x14),
        ] {
            assert!(
                !ai_is_ship_busy(&flying(1, goal)),
                "{goal:?}: goal {original:#x}, @0x82fb9-0x82fd6"
            );
        }
    }

    #[test]
    fn every_other_goal_is_busy() {
        for (goal, original) in [
            (Goal::Flee(OTHER), 3),
            (Goal::Attack(OTHER), 4),
            (Goal::Snipe(OTHER), 4),
            (Goal::Dock(OTHER), 5),
            (Goal::Wait, 6),
            (Goal::Assist(Help::Refuel), 9),
            (Goal::Assist(Help::Repair), 0xf),
            (
                Goal::Follow {
                    lead: NpcId(9),
                    guard: None,
                },
                0xa,
            ),
            (Goal::Formation { guard: None }, 0xa),
            (Goal::Hold, 0xa),
            (
                Goal::Plunder {
                    ship: OTHER,
                    hopeless: false,
                },
                0xd,
            ),
            (Goal::Drift, 0xe),
            (Goal::Mine, 0x10),
            (Goal::Scoop, 0x11),
            (Goal::AttackStellar(PLANET), 0x12),
            (Goal::Regroup { kept: Kept::Coast }, 0x13),
            (Goal::CountWait, 0x16),
        ] {
            assert!(
                ai_is_ship_busy(&flying(1, goal)),
                "{goal:?}: goal {original:#x}, @0x82fda"
            );
        }
    }

    #[test]
    fn an_idle_ship_jumping_in_or_emerging_is_busy_and_one_coasting_is_not() {
        for (mode, original) in [
            (Mode::JumpingIn { tick: 0 }, 8),
            (Mode::LeavingGate { tick: 3 }, 8),
            (Mode::Emerging { ticks_left: 10 }, 0x15),
            (Mode::LeavingGate { tick: 0 }, 0x15),
        ] {
            let mut npc = flying(1, Goal::Idle);
            npc.mode = mode;
            assert!(ai_is_ship_busy(&npc), "{mode:?}: goal {original:#x}");
        }
        for mode in [
            Mode::Coasting { ticks_left: 10 },
            Mode::Adrift { ticks_left: 10 },
            Mode::Flying,
        ] {
            let mut npc = flying(1, Goal::Idle);
            npc.mode = mode;
            assert!(!ai_is_ship_busy(&npc), "{mode:?}: goal 0");
        }
    }

    #[test]
    fn a_ship_following_its_leaders_jump_is_busy_and_one_on_its_own_jump_by_its_goal() {
        let mut follower = flying(1, Goal::Idle);
        follower.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        assert!(ai_is_ship_busy(&follower), "goal 0xb");
        let mut held = flying(1, Goal::JumpOut);
        held.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        assert!(!ai_is_ship_busy(&held), "the hold's goal 2");
        let mut own = flying(1, Goal::JumpOut);
        own.warp.goal = Some(WarpGoal::Own);
        assert!(!ai_is_ship_busy(&own), "goal 2");
        own.goal = Goal::Flee(OTHER);
        assert!(ai_is_ship_busy(&own), "goal 3");
    }

    #[test]
    fn an_attack_or_snipe_at_it_counts_as_attacked() {
        for goal in [
            Goal::Attack(ShipRef::Npc(NpcId(1))),
            Goal::Snipe(ShipRef::Npc(NpcId(1))),
        ] {
            let npcs = [flying(1, Goal::Idle), flying(2, goal)];
            assert!(attacked(&npcs[0], &npcs), "{goal:?}");
        }
    }

    #[test]
    fn a_flee_from_it_an_attack_on_another_and_its_own_goal_are_not_attacks() {
        for goal in [
            Goal::Flee(ShipRef::Npc(NpcId(1))),
            Goal::Attack(ShipRef::Npc(NpcId(3))),
        ] {
            let npcs = [flying(1, Goal::Idle), flying(2, goal)];
            assert!(!attacked(&npcs[0], &npcs), "{goal:?}");
        }
        let npcs = [flying(1, Goal::Attack(ShipRef::Npc(NpcId(1))))];
        assert!(!attacked(&npcs[0], &npcs), "its own goal");
    }
}
