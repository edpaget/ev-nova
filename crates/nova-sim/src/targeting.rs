//! The player's target: which NPC a target command picks.
//!
//! The rules are the original's (`_HandlePlayer` and the searches it calls
//! in the `EV Nova` executable):
//!
//! - A ship is targetable ([`targetable`]) while it is intact or
//!   disabled: a ship breaking up or destroyed is not (`_IsDying`). Target
//!   Select and Closest Target skip the player's escorts ([`candidate`]);
//!   cloaked and untargetable ships and ships coming out of a hypergate
//!   are skipped too, once they exist.
//! - Escort Select (Option-Tab here) walks the player's escorts alone,
//!   in the same order, from the one after the target, and does not wrap
//!   ([`next_escort`]).
//! - Target Select (Tab, `_FindNextShipInSystem` @0xa5b1, its caller
//!   @0x69f1c-0x69ff5) walks the ship slots in order, from the one after
//!   the target, or from the first with none, and does not wrap: past the
//!   last it gives none. Here the slots are the NPCs in the order they
//!   appeared, whose [`NpcId`]s only grow and are never reused, so
//!   [`next`] is the first candidate numbered after the target, or the
//!   first with none, even when the target itself has gone.
//! - Closest Target (R, @0x6a9ab-0x6aadb) picks the candidate nearest the
//!   player by squared distance (@0x9368-0x93ba), with no limit; only a
//!   strictly nearer ship replaces the best, so a tie goes to the earlier.
//!   Option-R takes any ship (`_FindNearestShipToPlayer` @0x923f) and plain
//!   R only the threats (`_FindNearestThreatToPlayer` @0x93f9, with
//!   `_IsThreatToPlayer` @0x7f501: an intact ship attacking, sniping at or
//!   fleeing from the player, [`Npc::threatens_player`]), so [`nearest`]
//!   takes the filter.

use crate::combat::hull::Condition;
use crate::geometry::Vec2;
use crate::traffic::npc::{Npc, NpcId};

/// How a target command picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetPick {
    /// The candidate nearest the player.
    Nearest,
    /// The candidate nearest the player that threatens it
    /// ([`Npc::threatens_player`]).
    NearestThreat,
    /// The next candidate after the target ([`next`]).
    Next,
    /// The next of the player's escorts after the target
    /// ([`next_escort`]).
    NextEscort,
}

/// Whether `npc` can be targeted: it is intact or disabled.
#[must_use]
pub fn targetable(npc: &Npc) -> bool {
    matches!(npc.condition, Condition::Intact | Condition::Disabled)
}

/// Whether `npc` is a candidate for Target Select and Closest Target:
/// targetable, and not one of the player's escorts.
#[must_use]
pub fn candidate(npc: &Npc) -> bool {
    targetable(npc) && npc.escort.is_none()
}

/// The candidate among `npcs` that passes `filter` nearest `from`, by
/// squared distance, a tie going to the earlier; none when none qualifies.
#[must_use]
pub fn nearest(npcs: &[Npc], from: Vec2, filter: impl Fn(&Npc) -> bool) -> Option<NpcId> {
    let mut best: Option<(f32, NpcId)> = None;
    for npc in npcs.iter().filter(|npc| candidate(npc) && filter(npc)) {
        let off = npc.state.position - from;
        let distance = off.x * off.x + off.y * off.y;
        if best.is_none_or(|(nearest, _)| distance < nearest) {
            best = Some((distance, npc.id));
        }
    }
    best.map(|(_, id)| id)
}

/// The first candidate among `npcs` numbered after `current`, or the
/// first candidate when there is no target; none past the last.
#[must_use]
pub fn next(npcs: &[Npc], current: Option<NpcId>) -> Option<NpcId> {
    first_after(npcs, current, candidate)
}

/// The first of the player's escorts among `npcs` numbered after
/// `current`, or the first when there is no target; none past the last.
#[must_use]
pub fn next_escort(npcs: &[Npc], current: Option<NpcId>) -> Option<NpcId> {
    first_after(npcs, current, |npc| targetable(npc) && npc.escort.is_some())
}

/// The first NPC among `npcs` passing `filter` numbered after `current`,
/// or the first passing it when there is no target.
fn first_after(
    npcs: &[Npc],
    current: Option<NpcId>,
    filter: impl Fn(&Npc) -> bool,
) -> Option<NpcId> {
    npcs.iter()
        .filter(|npc| filter(npc))
        .map(|npc| npc.id)
        .find(|&id| current.is_none_or(|current| id > current))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::ShipStats;
    use crate::testkit::npc;

    /// NPC `id` at (`x`, `y`), in `condition`.
    fn at(id: u32, x: f32, y: f32, condition: Condition) -> Npc {
        let mut npc = npc(id, ShipStats::default());
        npc.state.position = Vec2::new(x, y);
        npc.condition = condition;
        npc
    }

    fn intact(id: u32, x: f32, y: f32) -> Npc {
        at(id, x, y, Condition::Intact)
    }

    const DYING: Condition = Condition::Dying { ticks_left: 5 };

    #[test]
    fn intact_and_disabled_ships_are_targetable_and_dying_or_destroyed_ones_are_not() {
        let of = |condition| targetable(&at(0, 0.0, 0.0, condition));
        assert!(of(Condition::Intact));
        assert!(of(Condition::Disabled));
        assert!(!of(DYING));
        assert!(!of(Condition::Destroyed));
    }

    #[test]
    fn the_nearest_is_by_squared_distance() {
        let npcs = [
            intact(1, 300.0, 0.0),
            intact(2, -100.0, 120.0),
            intact(3, 0.0, -200.0),
        ];
        let any = |_: &Npc| true;
        assert_eq!(nearest(&npcs, Vec2::ZERO, any), Some(NpcId(2)));
        assert_eq!(nearest(&npcs, Vec2::new(250.0, 0.0), any), Some(NpcId(1)));
        assert_eq!(nearest(&npcs, Vec2::new(0.0, -150.0), any), Some(NpcId(3)));
    }

    #[test]
    fn a_tie_goes_to_the_earlier_npc() {
        let npcs = [
            intact(4, 0.0, 100.0),
            intact(5, 100.0, 0.0),
            intact(6, -100.0, 0.0),
        ];
        assert_eq!(nearest(&npcs, Vec2::ZERO, |_| true), Some(NpcId(4)));
        let later = [intact(7, 0.0, 200.0), npcs[1].clone(), npcs[2].clone()];
        assert_eq!(nearest(&later, Vec2::ZERO, |_| true), Some(NpcId(5)));
    }

    #[test]
    fn the_nearest_may_be_disabled_but_never_dying_or_destroyed() {
        let npcs = [
            at(1, 10.0, 0.0, DYING),
            at(2, 20.0, 0.0, Condition::Destroyed),
            at(3, 30.0, 0.0, Condition::Disabled),
            intact(4, 40.0, 0.0),
        ];
        assert_eq!(nearest(&npcs, Vec2::ZERO, |_| true), Some(NpcId(3)));
        assert_eq!(nearest(&npcs[..2], Vec2::ZERO, |_| true), None);
    }

    #[test]
    fn the_nearest_passes_the_filter() {
        let npcs = [intact(1, 10.0, 0.0), intact(2, 20.0, 0.0)];
        assert_eq!(
            nearest(&npcs, Vec2::ZERO, |npc| npc.id != NpcId(1)),
            Some(NpcId(2))
        );
        assert_eq!(nearest(&npcs, Vec2::ZERO, |_| false), None);
        assert_eq!(nearest(&[], Vec2::ZERO, |_| true), None);
    }

    #[test]
    fn next_walks_the_npcs_in_order_then_gives_none() {
        let npcs = [
            intact(2, 0.0, 0.0),
            intact(5, 0.0, 0.0),
            intact(9, 0.0, 0.0),
        ];
        let mut walked = Vec::new();
        let mut current = None;
        for _ in 0..5 {
            current = next(&npcs, current);
            walked.push(current);
        }
        assert_eq!(
            walked,
            [
                Some(NpcId(2)),
                Some(NpcId(5)),
                Some(NpcId(9)),
                None,
                Some(NpcId(2))
            ]
        );
        assert_eq!(next(&[], None), None);
    }

    #[test]
    fn next_after_a_target_that_has_gone_is_the_next_number_after_it() {
        let npcs = [intact(2, 0.0, 0.0), intact(9, 0.0, 0.0)];
        assert_eq!(next(&npcs, Some(NpcId(5))), Some(NpcId(9)));
        assert_eq!(next(&npcs, Some(NpcId(1))), Some(NpcId(2)));
        assert_eq!(next(&npcs, Some(NpcId(10))), None);
    }

    #[test]
    fn next_skips_the_dying_and_takes_the_disabled() {
        let npcs = [
            at(1, 0.0, 0.0, DYING),
            at(2, 0.0, 0.0, Condition::Disabled),
            at(3, 0.0, 0.0, Condition::Destroyed),
            intact(4, 0.0, 0.0),
        ];
        assert_eq!(next(&npcs, None), Some(NpcId(2)));
        assert_eq!(next(&npcs, Some(NpcId(2))), Some(NpcId(4)));
        assert_eq!(next(&npcs[..1], None), None);
    }

    /// NPC `id` at (`x`, `y`), the player's escort.
    fn escort(id: u32, x: f32, y: f32) -> Npc {
        let mut npc = intact(id, x, y);
        npc.escort = Some(crate::escort::EscortDuty {
            slot: 2,
            ships: 2,
            spacing: 30.0,
            order: None,
        });
        npc
    }

    #[test]
    fn the_nearest_and_next_skip_the_players_escorts() {
        let npcs = [
            escort(1, 10.0, 0.0),
            intact(2, 20.0, 0.0),
            escort(3, 5.0, 0.0),
        ];
        assert_eq!(nearest(&npcs, Vec2::ZERO, |_| true), Some(NpcId(2)));
        assert_eq!(next(&npcs, None), Some(NpcId(2)));
        assert_eq!(next(&npcs, Some(NpcId(2))), None);
        assert_eq!(nearest(&[escort(1, 0.0, 0.0)], Vec2::ZERO, |_| true), None);
        assert!(targetable(&npcs[0]), "an escort targeted stays targeted");
    }

    #[test]
    fn next_escort_walks_the_escorts_alone_in_order_then_gives_none() {
        let npcs = [
            escort(1, 0.0, 0.0),
            intact(2, 0.0, 0.0),
            escort(4, 0.0, 0.0),
            intact(5, 0.0, 0.0),
            escort(7, 0.0, 0.0),
        ];
        let mut walked = Vec::new();
        let mut current = None;
        for _ in 0..5 {
            current = next_escort(&npcs, current);
            walked.push(current);
        }
        assert_eq!(
            walked,
            [
                Some(NpcId(1)),
                Some(NpcId(4)),
                Some(NpcId(7)),
                None,
                Some(NpcId(1))
            ]
        );
        assert_eq!(
            next_escort(&npcs, Some(NpcId(2))),
            Some(NpcId(4)),
            "after a non-escort"
        );
        assert_eq!(next_escort(&npcs[1..2], None), None);
    }
}
