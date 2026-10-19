//! Tests of an inspecting interceptor's re-pick as the traffic takes it
//! (`_InterceptorAI` @0x8d404-0x8d43a, see
//! [`interceptor`](crate::ai::interceptor)): the state-7 block's reach
//! reads the re-picked target (@0x8f11e), so a re-pick away from the
//! player within its reach scans nothing.

use super::on_line::OnLine;
use super::{Traffic, World};
use crate::ai::{Behaviour, Decision, Goal, Surroundings};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{Npc, NpcId};
use nova_sim_base::fixture::{Draws, FAST};

/// Every NPC takes this decision.
#[derive(Debug)]
struct Fixed(Decision);

impl Behaviour for Fixed {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        self.0.goal
    }

    fn decision(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Decision {
        self.0
    }
}

/// What a traffic of one interceptor inspecting the player 50 off puts
/// on the line when its decision is `decision`.
fn on_line_after(decision: Decision) -> Vec<OnLine> {
    let mut traffic = Traffic::with_interval(1);
    let mut cop = npc(1, ShipStats::new(FAST, &[]));
    cop.goal = Goal::Inspect(ShipRef::Player);
    traffic.npcs = vec![cop];
    let world = World {
        player: Some(crate::ai::fixture::player(50.0, 0.0)),
        ..World::new(&[])
    };
    traffic.tick_in(&Fixed(decision), world, &[], &mut Draws::of(&[]));
    traffic.take_on_line()
}

#[test]
fn a_re_pick_away_from_the_player_within_reach_scans_nothing() {
    let looked_over = Decision {
        goal: Goal::Inspect(ShipRef::Npc(NpcId(5))),
        ..Decision::default()
    };
    assert_eq!(
        on_line_after(looked_over),
        [OnLine::Scan(NpcId(1))],
        "looked over, on to another: the scan (@0x8f38a)"
    );
    let re_picked = Decision {
        re_picked: true,
        ..looked_over
    };
    assert_eq!(
        on_line_after(re_picked),
        [],
        "the reach reads the new target"
    );
    let attacking = Decision {
        goal: Goal::Attack(ShipRef::Player),
        re_picked: true,
        ..Decision::default()
    };
    assert_eq!(on_line_after(attacking), [], "state 4 runs no reach");
}
