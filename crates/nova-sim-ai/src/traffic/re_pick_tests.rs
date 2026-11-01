//! Tests of an inspecting interceptor's re-pick as the traffic takes it
//! (`_InterceptorAI` @0x8d404-0x8d43a, see
//! [`interceptor`](crate::ai::interceptor)): the state-7 block's reach
//! reads the re-picked target (@0x8f11e), so a re-pick away from the
//! player within its reach scans nothing; and of a pick looked over in
//! its own think ([`Decision::looked_over`], @0x8f11e-0x8f394).

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

/// A traffic of one interceptor with goal `goal`, the player 50 off, after
/// a tick whose decision is `decision`: what it put on the line, and the
/// ship it remembers inspecting.
fn after_tick(goal: Goal, decision: Decision) -> (Vec<OnLine>, Option<ShipRef>) {
    let mut traffic = Traffic::with_interval(1);
    let mut cop = npc(1, ShipStats::new(FAST, &[]));
    cop.goal = goal;
    traffic.npcs = vec![cop];
    let world = World {
        player: Some(crate::ai::fixture::player(50.0, 0.0)),
        ..World::new(&[])
    };
    traffic.tick_in(&Fixed(decision), world, &[], &mut Draws::of(&[]));
    (traffic.take_on_line(), traffic.npcs()[0].inspected)
}

/// What a traffic of one interceptor inspecting the player 50 off puts
/// on the line when its decision is `decision`.
fn on_line_after(decision: Decision) -> Vec<OnLine> {
    after_tick(Goal::Inspect(ShipRef::Player), decision).0
}

/// A pick of `ship` the think looked over at once.
fn looked_over(ship: ShipRef) -> Decision {
    Decision {
        goal: Goal::Idle,
        clears_target: true,
        looked_over: Some(ship),
        ..Decision::default()
    }
}

#[test]
fn a_pick_of_the_player_within_reach_scans_that_tick() {
    let (line, inspected) = after_tick(Goal::Idle, looked_over(ShipRef::Player));
    assert_eq!(line, [OnLine::Scan(NpcId(1))], "@0x8f38a");
    assert_eq!(inspected, Some(ShipRef::Player));
}

#[test]
fn a_pick_of_an_npc_within_reach_records_it_and_scans_nothing() {
    let other = ShipRef::Npc(NpcId(5));
    let (line, inspected) = after_tick(Goal::Idle, looked_over(other));
    assert_eq!(line, []);
    assert_eq!(inspected, Some(other));
}

#[test]
fn a_kept_inspection_of_the_player_within_reach_still_scans_beside_a_fresh_pick_within_reach() {
    // The original does these on two frames: the kept one, then the pick.
    let other = ShipRef::Npc(NpcId(5));
    let (line, inspected) = after_tick(Goal::Inspect(ShipRef::Player), looked_over(other));
    assert_eq!(line, [OnLine::Scan(NpcId(1))]);
    assert_eq!(inspected, Some(other));
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
