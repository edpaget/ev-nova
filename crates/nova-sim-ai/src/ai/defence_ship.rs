//! How a stellar's defence ship decides: `_DefenseShipAI`
//! (@0x7f7ed-0x7f9d4 in the `EV Nova` executable), which `_AIDispatch`
//! calls in place of its type's think for a ship that defends a stellar
//! (+0x8c ≠ −1, @0x8ff4d-0x8ff5a; see [`NovaAi`](crate::ai::NovaAi)). Its
//! fire, gate wait and answer to hits stay its AI type's, a warship's,
//! as the dispatch replaces only the think. The ships come up as
//! [`stellar_defence`](crate::traffic::stellar_defence) launches them.
//!
//! Each think, in this order:
//!
//! 1. A dying defender, or one in its count wait ([`Goal::CountWait`],
//!    0x16), decides nothing (@0x7f7fc, @0x7f80c).
//! 2. **The candidates** (@0x7f834-0x7f8d7): the player (slot 0) and the
//!    ships the player leads ([`Npc::escorts_player`], leader +0x9a 0),
//!    in slot order, each that the defender sees ([`Surroundings::sees`],
//!    `_ShipVisibleToShip(candidate, ship)` @0x7f862). A defender never
//!    picks another NPC on its own: it fights one only as its answer to
//!    a hit.
//! 3. **Two passes** (@0x7f8dd-0x7f8f8) on the squared distance between
//!    centres: the first takes only candidates within
//!    [`DEFENCE_RADIUS_SQ`], 6000 px; the second, run only when the first
//!    found none, takes any. The nearest wins, its distance held as a
//!    truncated whole number and beaten only by a strictly nearer one
//!    (@0x7f8b3, @0x7f8c1), so a tie keeps the lower slot.
//! 4. It ends its AI wait (+0x4c = 0) and clears its travel stellar
//!    (+0x6c = −1) (@0x7f906-0x7f90d): a defender that landed is up
//!    again the next think, as it keeps its slot.
//! 5. **The keep test** (@0x7f901-0x7f942): a target it no longer sees is
//!    dropped. The original asks `_ShipVisibleToShip(&ships[64], ship)`
//!    (@0x7f915-0x7f928): the compiler folded the loop index left at 64
//!    into 0x325000, one slot past the 64-slot table, a read that can't
//!    be replicated. This takes the intended reading, the defender's own
//!    target, with no rulebook key, as the original's reading is not
//!    knowable.
//! 6. **No target** (@0x7f944-0x7f971): it attacks the candidate (goal 4),
//!    or with none lands on its own stellar (goal 1, +0x6c = +0x8c).
//! 7. **The target check** (@0x7f975-0x7f9c4): a target that is neither
//!    the player nor a ship the player leads (a hit's answer set it) is
//!    swapped for the candidate, its goal kept, or with none dropped to
//!    idle (goal 0, sub 0).
//!
//! It plays no sound and draws nothing.

use std::cmp::Ordering;

use crate::ai::travel::TravelWrite;
use crate::ai::{Behaviour, Decision, Goal, Surroundings};
use crate::catalog::StellarId;
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::geometry::Vec2;
use crate::traffic::npc::Npc;

/// How near a candidate must be for the first pass, squared: 36,000,000.0
/// (0xdda20), 6000 px.
pub const DEFENCE_RADIUS_SQ: f32 = 36_000_000.0;

/// What a defender's think comes to (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Think {
    /// Its goal.
    pub goal: Goal,
    /// Its target (+0x70).
    pub target: Option<ShipRef>,
}

/// The candidate `npc` turns on among `around` (steps 2 and 3 of the
/// module docs), if any.
#[must_use]
pub fn candidate(npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
    let mut led: Vec<&Npc> = around
        .npcs
        .iter()
        .filter(|other| other.escorts_player())
        .collect();
    led.sort_by_key(|other| other.berth);
    let candidates: Vec<(ShipRef, Vec2)> = around
        .player
        .map(|player| (ShipRef::Player, player.state.position))
        .into_iter()
        .chain(
            led.into_iter()
                .map(|other| (ShipRef::Npc(other.id), other.state.position)),
        )
        .filter(|&(ship, _)| around.sees(npc, ship))
        .collect();
    [Some(DEFENCE_RADIUS_SQ), None]
        .into_iter()
        .find_map(|limit| nearest(npc.state.position, &candidates, limit))
}

/// The nearest of `candidates` to `at` within `limit`, squared, if any,
/// as one pass of step 3 picks it.
fn nearest(at: Vec2, candidates: &[(ShipRef, Vec2)], limit: Option<f32>) -> Option<ShipRef> {
    let mut best: Option<(ShipRef, i32)> = None;
    for &(ship, position) in candidates {
        let off = position - at;
        let dist = (off.x * off.x + off.y * off.y).abs();
        // `setae` (@0x7f89b): not past the limit, and a distance that is
        // no number is past it.
        if limit
            .is_some_and(|limit| matches!(dist.partial_cmp(&limit), Some(Ordering::Greater) | None))
        {
            continue;
        }
        // The best is kept truncated, and only a strictly nearer one
        // beats it (@0x7f8aa-0x7f8ca).
        if best.is_none_or(|(_, held)| held as f32 > dist) {
            best = Some((ship, dist as i32));
        }
    }
    best.map(|(ship, _)| ship)
}

/// `npc`'s think among `around` (see the module docs), defending
/// `stellar`; none when it decides nothing (step 1).
#[must_use]
pub fn think(npc: &Npc, stellar: StellarId, around: &Surroundings) -> Option<Think> {
    if npc.condition.dying_or_destroyed(npc.reserves.armor) || npc.goal == Goal::CountWait {
        return None;
    }
    let best = candidate(npc, around);
    let mut goal = npc.goal;
    let mut target = npc.target.filter(|&kept| around.sees(npc, kept));
    if target.is_none() {
        if let Some(ship) = best {
            target = Some(ship);
            goal = Goal::Attack(ship);
        } else {
            goal = Goal::Land(stellar);
        }
    }
    if let Some(kept) = target
        && !players_side(kept, around)
    {
        if let Some(ship) = best {
            target = Some(ship);
            goal = with_quarry(goal, ship);
        } else {
            target = None;
            goal = Goal::Idle;
        }
    }
    Some(Think { goal, target })
}

/// Whether `ship` is the player or a ship the player leads, as step 7
/// reads its leader (+0x9a 0); a ship gone from `around` is neither.
fn players_side(ship: ShipRef, around: &Surroundings) -> bool {
    match ship {
        ShipRef::Player => true,
        ShipRef::Npc(id) => around.npc(id).is_some_and(Npc::escorts_player),
    }
}

/// `goal` with its ship `ship`, as a new target leaves the goal number
/// as it was: a goal of no ship is kept.
fn with_quarry(goal: Goal, ship: ShipRef) -> Goal {
    match goal {
        Goal::Attack(_) => Goal::Attack(ship),
        Goal::Snipe(_) => Goal::Snipe(ship),
        Goal::Flee(_) => Goal::Flee(ship),
        Goal::Inspect(_) => Goal::Inspect(ship),
        other => other,
    }
}

/// How a defence ship decides and targets (see the module docs): its
/// fire, gate wait and answer to hits stay its AI type's, which
/// [`NovaAi`](crate::ai::NovaAi) keeps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DefenceShipAi;

impl Behaviour for DefenceShipAi {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    /// Its [`think`]'s goal, its AI wait ended and its travel stellar
    /// cleared, or set to its own stellar as it lands (step 4, step 6);
    /// its goal kept, writing nothing, when it decides nothing or defends
    /// none.
    fn decision(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Decision {
        let Some(think) = npc.defends.and_then(|stellar| think(npc, stellar, around)) else {
            return Decision {
                goal: npc.goal,
                ..Decision::default()
            };
        };
        let travel = match think.goal {
            Goal::Land(_) => TravelWrite::Set,
            _ => TravelWrite::Clear,
        };
        Decision {
            goal: think.goal,
            travel,
            ends_wait: true,
            ..Decision::default()
        }
    }

    /// Its [`think`]'s target; the one it holds when it decides nothing.
    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        npc.defends
            .and_then(|stellar| think(npc, stellar, around))
            .map_or(npc.target, |think| think.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{TRADERS, cloaked_player, n, player, ship};
    use crate::combat::hull::Condition;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::traffic::npc::AiType;
    use nova_sim_base::fixture::Draws;

    const EARTH: StellarId = StellarId(128);

    /// A defender of Earth, berth `id`, at the centre.
    fn defender(id: u32) -> Npc {
        let mut npc = ship(id, TRADERS, AiType::Warship, 0.0, 0.0);
        npc.berth = id;
        npc.defends = Some(EARTH);
        npc
    }

    /// An escort of the player at berth `id`, at (`x`, 0).
    fn escort(id: u32, x: f32) -> Npc {
        let mut npc = ship(id, TRADERS, AiType::Warship, x, 0.0);
        npc.berth = id;
        npc.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        npc
    }

    /// What `npcs[0]` decides among `npcs` with the player at (`x`, 0).
    fn thinks(npcs: &[Npc], at: Option<f32>) -> (Goal, Option<ShipRef>, Decision) {
        let around = Surroundings {
            player: at.map(|x| player(x, 0.0)),
            ..Surroundings::new(&[], npcs)
        };
        let ai = DefenceShipAi;
        let decision = ai.decision(&npcs[0], &around, &mut Draws::of(&[]));
        (decision.goal, ai.target(&npcs[0], &around), decision)
    }

    #[test]
    fn it_attacks_the_nearest_candidate_within_6000() {
        let npcs = [defender(1), escort(3, 2000.0), escort(2, -1500.0)];
        let (goal, target, decision) = thinks(&npcs, Some(5000.0));
        assert_eq!(goal, Goal::Attack(n(2)));
        assert_eq!(target, Some(n(2)));
        assert!(decision.ends_wait, "+0x4c = 0");
        assert_eq!(decision.travel, TravelWrite::Clear, "+0x6c = -1");
    }

    #[test]
    fn with_none_within_6000_it_attacks_the_nearest_visible_at_any_distance() {
        let npcs = [defender(1), escort(2, 9000.0)];
        let (goal, target, _) = thinks(&npcs, Some(8000.0));
        assert_eq!(
            (goal, target),
            (Goal::Attack(ShipRef::Player), Some(ShipRef::Player))
        );
        // Exactly 6000 off is in range.
        let npcs = [defender(1), escort(2, 6000.0)];
        let (goal, _, _) = thinks(&npcs, Some(5999.0 + 2.0));
        assert_eq!(goal, Goal::Attack(n(2)));
    }

    #[test]
    fn a_tie_keeps_the_lower_slot() {
        let npcs = [defender(1), escort(4, 100.0), escort(3, -100.0)];
        let (goal, _, _) = thinks(&npcs, Some(100.0));
        assert_eq!(goal, Goal::Attack(ShipRef::Player), "slot 0 first");
        let (goal, _, _) = thinks(&npcs, None);
        assert_eq!(goal, Goal::Attack(n(3)), "berth 3 before berth 4");
    }

    #[test]
    fn with_no_candidate_visible_it_lands_on_its_own_stellar() {
        let npcs = [defender(1)];
        let (goal, target, decision) = thinks(&npcs, None);
        assert_eq!((goal, target), (Goal::Land(EARTH), None));
        assert_eq!(decision.travel, TravelWrite::Set, "+0x6c = +0x8c");
        assert!(decision.ends_wait, "its stay ended");
        // A cloaked player it can't see is no candidate.
        let around = Surroundings {
            player: Some(cloaked_player(player(100.0, 0.0))),
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            DefenceShipAi
                .decision(&npcs[0], &around, &mut Draws::of(&[]))
                .goal,
            Goal::Land(EARTH)
        );
    }

    #[test]
    fn a_trader_nearer_than_the_player_is_never_picked() {
        let npcs = [
            defender(1),
            ship(2, TRADERS, AiType::WimpyTrader, 10.0, 0.0),
        ];
        let (goal, _, _) = thinks(&npcs, Some(3000.0));
        assert_eq!(goal, Goal::Attack(ShipRef::Player));
    }

    #[test]
    fn a_kept_target_it_still_sees_is_kept_and_one_it_lost_is_replaced() {
        let mut npcs = [defender(1), escort(2, 9000.0), escort(3, 100.0)];
        npcs[0].target = Some(n(2));
        npcs[0].goal = Goal::Attack(n(2));
        let (goal, target, _) = thinks(&npcs, None);
        assert_eq!((goal, target), (Goal::Attack(n(2)), Some(n(2))), "kept");
        npcs[1] = crate::ai::fixture::cloaked(npcs[1].clone());
        let (goal, target, _) = thinks(&npcs, None);
        assert_eq!((goal, target), (Goal::Attack(n(3)), Some(n(3))), "replaced");
    }

    #[test]
    fn a_target_off_the_players_side_is_swapped_for_the_candidate_or_dropped() {
        let mut npcs = [
            defender(1),
            ship(2, TRADERS, AiType::WimpyTrader, 10.0, 0.0),
        ];
        npcs[0].target = Some(n(2));
        npcs[0].goal = Goal::Attack(n(2));
        let (goal, target, _) = thinks(&npcs, Some(500.0));
        assert_eq!(
            (goal, target),
            (Goal::Attack(ShipRef::Player), Some(ShipRef::Player)),
            "the goal kept, its ship the candidate's"
        );
        npcs[0].goal = Goal::Flee(n(2));
        let (goal, _, _) = thinks(&npcs, Some(500.0));
        assert_eq!(goal, Goal::Flee(ShipRef::Player));
        let (goal, target, _) = thinks(&npcs, None);
        assert_eq!((goal, target), (Goal::Idle, None), "goal 0, sub 0");
    }

    #[test]
    fn a_dying_defender_or_one_in_its_count_wait_decides_nothing() {
        let mut npcs = [defender(1)];
        npcs[0].goal = Goal::CountWait;
        npcs[0].target = Some(n(9));
        let (goal, target, decision) = thinks(&npcs, Some(10.0));
        assert_eq!((goal, target), (Goal::CountWait, Some(n(9))));
        assert!(!decision.ends_wait);
        npcs[0].goal = Goal::Idle;
        npcs[0].condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let (goal, target, _) = thinks(&npcs, Some(10.0));
        assert_eq!((goal, target), (Goal::Idle, Some(n(9))));
    }

    #[test]
    fn a_ship_that_defends_nothing_decides_nothing() {
        let mut npcs = [defender(1)];
        npcs[0].defends = None;
        let (goal, target, decision) = thinks(&npcs, Some(10.0));
        assert_eq!((goal, target), (Goal::Idle, None));
        assert_eq!(decision, Decision::default());
    }
}
