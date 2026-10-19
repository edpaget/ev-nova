//! A ship docking with the disabled player to repair it: goal 0xf's block
//! of `_HighLevelAIHandler` (@0x8f6d4-0x8f82f in the `EV Nova` executable,
//! [`high_level`]) and the repair half of `_LowLevelAIHandler`'s sub 0xf
//! (@0x86d4f-0x86f37, [`complete`]). The docking itself, its substates
//! 0xb and 0xf, is the plunder's ([`plunder_dock`]), which this shares.
//!
//! - **The high level** ([`high_level`]), on each think while its AI wait
//!   is out (@0x8d4b9-0x8d4d2 returns for every goal but 10 and 0xe while
//!   it runs). With no target nothing changes (@0x8f6e9). With the player
//!   gone or no longer disabled (@0x8f702-0x8f715) it gives up: goal 0,
//!   sub 0, target and nav -1 (@0x8f717-0x8f732). Otherwise nav = the
//!   player (@0x8f74c) and, within [`repair_box`] of it on both axes,
//!   inclusive, it docks (sub 0xf, @0x8f815); otherwise it closes (sub 0xb,
//!   @0x8f823). There is no approach (sub 9): the box is 2-way, not the
//!   plunder's 3-way one.
//! - **The dock** is the plunder's sub 0xf: inside 3 pixels with no wait
//!   left, the player is boarded and the wait is `Rand(80)` + 100
//!   (@0x86fa4-0x8700a).
//! - **The completion** ([`complete`]): docked, with its wait above none and
//!   at most [`DOCK_WAIT`](plunder_dock::DOCK_WAIT) (@0x86d54-0x86d6d), the
//!   player is unboarded (@0x86d92), its armour raised [`REPAIR_STEP`] at a
//!   time while it is disabled (@0x86d9c-0x86dcf), and "`CommName`:  Repairs
//!   complete, `ship name`." said for 240 ticks with `snd ` 151
//!   (@0x86ddf-0x86ef4): the session's, from
//!   [`PirateEvent::PlayerRepaired`](crate::traffic::plunder::PirateEvent::PlayerRepaired).
//!   The repairer then drifts (goal 0xe) with a wait of [`REPAIR_HOLD`],
//!   its target, nav and substate cleared (@0x86f09-0x86f31).
//!
//! With the wait counted down after each tick's flight (`_HandleShip`
//! @0x35037-0x35048, after every ship's `_AIDispatch`), a dock started on
//! tick k with wait W repairs on tick k + max(1, W - 100), and the drift
//! that follows steps 100 times and sets goal 0 on its 100th.
//!
//! The repair's high level runs on each of its thinks though an
//! unprovoked assister decides nothing: the type AIs return at once for
//! goals 9 and 0xf (`_WimpyTraderAI` @0x8b045-0x8b05c), but `_AIDispatch`
//! runs the high and low levels whatever they did (@0x901e5-0x901f4).
//!
//! Left out: the ×4 box for an inertialess repairer (@0x8f781-0x8f797),
//! which needs sub 0xf's inertialess branch,
//! rdm:phase/combat-ai-fidelity-cross-roadmap/phase-3-inertialess-repair-box.
//! Not built, as unreachable: the completion's path for an NPC nav and
//! goal 0xf against an NPC target, as only `_AIMakeShipRepairPlayer`
//! (@0x82df2) sets goal 0xf, always at the player. The `_statusRedraw`
//! writes and +0x8c -1 are recorded only. The original's armour loop has
//! no cap; the session's stops at the armour's most, so a disable rule
//! that never lets the ship fly still ends.

use crate::ai::{Goal, Surroundings};
use crate::hail::Help;
use crate::traffic::npc::Npc;
#[cfg(doc)]
use crate::traffic::plunder_dock;
use crate::traffic::plunder_dock::{BOX_PER_TURN, BOX_TURNS, DockStep, DockSub};

/// What a repair box adds (the float 50.0 @0xdd064).
pub const REPAIR_BOX_BASE: f32 = 50.0;
/// The AI wait a repairer drifts for after its repair (the float 100.0
/// @0x86f09).
pub const REPAIR_HOLD: u32 = 100;
/// The armour a repair raises at a time (the float 1.0 @0xdd068).
pub const REPAIR_STEP: f32 = 1.0;

/// The repair box of a class whose turn (+0x38) is `class_turn`:
/// trunc((10 − turn) × 30 + 50), in floats (@0x8f740-0x8f77d), with no
/// guard.
#[must_use]
pub fn repair_box(class_turn: f32) -> i32 {
    // A multiply then an add, rounded at each, as the original's two
    // instructions are (@0x8f76d-0x8f775); within an i32 for any turn.
    #[allow(clippy::suboptimal_flops, clippy::cast_possible_truncation)]
    let side = ((BOX_TURNS - class_turn) * BOX_PER_TURN + REPAIR_BOX_BASE) as i32;
    side
}

/// The high level of `npc`'s repair of the player among `around` (see the
/// module docs): none when it is no repair or has no target.
#[must_use]
pub fn high_level(npc: &Npc, around: &Surroundings) -> Option<DockStep> {
    if npc.goal != Goal::Assist(Help::Repair) {
        return None;
    }
    npc.target?;
    let Some(at) = around
        .player
        .filter(|player| player.condition.disabled())
        .map(|player| player.state.position)
    else {
        return Some(DockStep::GiveUp);
    };
    // f32: the box is converted back to a float for the test.
    #[allow(clippy::cast_precision_loss)]
    let side = repair_box(npc.hull.class_turn) as f32;
    let off = npc.state.position - at;
    Some(DockStep::Sub(
        if off.x.abs() <= side && off.y.abs() <= side {
            DockSub::Dock
        } else {
            DockSub::Close
        },
    ))
}

/// `npc`'s repair done (@0x86f09-0x86f31): it drifts with a wait of
/// [`REPAIR_HOLD`], its target and docking dropped, its travel stellar
/// with the nav (+0x6c -1, @0x86f31).
pub fn complete(npc: &mut Npc) {
    npc.goal = Goal::Drift;
    npc.ai_wait = REPAIR_HOLD;
    npc.target = None;
    npc.docking = None;
    npc.travel_set = false;
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::PlayerSide;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::combat::hull::class_turn;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::World;
    use crate::traffic::fixture::npc;
    use crate::traffic::plunder_dock::Docking;
    use nova_sim_base::fixture::FAST;

    /// A ship repairing the player from (`x`, `y`), its class turn 3 (a
    /// box of 260).
    fn repairer(x: f32, y: f32) -> Npc {
        let mut ship = npc(1, ShipStats::new(FAST, &[]));
        ship.goal = Goal::Assist(Help::Repair);
        ship.target = Some(ShipRef::Player);
        ship.hull.class_turn = 3.0;
        ship.state.position = Vec2::new(x, y);
        ship
    }

    /// The player at (`x`, `y`) in `condition`.
    fn player(x: f32, y: f32, condition: Condition) -> PlayerSide {
        PlayerSide {
            condition,
            ..crate::ai::fixture::player(x, y)
        }
    }

    /// The high level of `ship` with `player` in the system, if any.
    fn step(ship: &Npc, player: Option<&PlayerSide>) -> Option<DockStep> {
        let npcs = [ship.clone()];
        let world = World {
            player: player.copied(),
            ..World::new(&[])
        };
        high_level(&npcs[0], &world.around(&npcs))
    }

    #[test]
    fn the_repair_box_is_ten_less_the_class_turn_times_thirty_plus_fifty_in_floats() {
        assert_eq!(repair_box(3.0), 260);
        assert_eq!(repair_box(8.0), 110);
        // In floats (10 - 3.3) × 30 is 201.0, and 251 with the 50.
        assert_eq!(repair_box(class_turn(33)), 251);
        assert_eq!(repair_box(10.5), 35);
        assert_eq!(repair_box(12.0), -10, "no guard");
    }

    #[test]
    fn the_repair_high_level_docks_inside_the_box_on_both_axes_inclusive_else_closes() {
        let downed = player(0.0, 0.0, Condition::Disabled);
        let at = |x: f32, y: f32| step(&repairer(x, y), Some(&downed));
        assert_eq!(at(260.0, -260.0), Some(DockStep::Sub(DockSub::Dock)));
        assert_eq!(at(-260.0, 260.0), Some(DockStep::Sub(DockSub::Dock)));
        assert_eq!(at(260.5, 0.0), Some(DockStep::Sub(DockSub::Close)));
        assert_eq!(at(0.0, -260.5), Some(DockStep::Sub(DockSub::Close)));
        assert_eq!(
            at(5000.0, 5000.0),
            Some(DockStep::Sub(DockSub::Close)),
            "never an approach"
        );
    }

    #[test]
    fn the_repair_high_level_measures_the_box_from_the_player_where_it_is() {
        let downed = player(1000.0, -1000.0, Condition::Disabled);
        assert_eq!(
            step(&repairer(1260.0, -740.0), Some(&downed)),
            Some(DockStep::Sub(DockSub::Dock))
        );
        assert_eq!(
            step(&repairer(0.0, 0.0), Some(&downed)),
            Some(DockStep::Sub(DockSub::Close))
        );
    }

    #[test]
    fn the_repair_high_level_gives_up_on_a_player_gone_or_no_longer_disabled() {
        let ship = repairer(0.0, 0.0);
        assert_eq!(step(&ship, None), Some(DockStep::GiveUp));
        assert_eq!(
            step(&ship, Some(&player(0.0, 0.0, Condition::Intact))),
            Some(DockStep::GiveUp)
        );
        assert_eq!(
            step(&ship, Some(&player(0.0, 0.0, Condition::Disabled))),
            Some(DockStep::Sub(DockSub::Dock))
        );
    }

    #[test]
    fn the_repair_high_level_changes_nothing_with_no_target() {
        let mut ship = repairer(0.0, 0.0);
        ship.target = None;
        assert_eq!(
            step(&ship, Some(&player(0.0, 0.0, Condition::Intact))),
            None
        );
    }

    #[test]
    fn the_repair_high_level_is_for_a_repair_only() {
        let mut ship = repairer(0.0, 0.0);
        ship.goal = Goal::Assist(Help::Refuel);
        assert_eq!(
            step(&ship, Some(&player(0.0, 0.0, Condition::Disabled))),
            None
        );
    }

    #[test]
    fn a_completed_repair_drifts_with_a_hold_of_100_and_drops_its_target_and_docking() {
        let mut ship = repairer(0.0, 0.0);
        ship.ai_wait = 40;
        ship.docking = Some(Docking {
            ship: ShipRef::Player,
            sub: DockSub::Dock,
        });
        ship.travel_set = true;
        complete(&mut ship);
        assert_eq!(ship.goal, Goal::Drift);
        assert_eq!(ship.ai_wait, REPAIR_HOLD);
        assert_eq!((ship.target, ship.docking), (None, None));
        assert!(!ship.travel_set, "nav -1, @0x86f31");
    }

    #[test]
    fn the_values_are_the_originals() {
        assert_eq!([REPAIR_BOX_BASE, REPAIR_STEP], [50.0, 1.0]);
        assert_eq!(REPAIR_HOLD, 100);
    }
}
