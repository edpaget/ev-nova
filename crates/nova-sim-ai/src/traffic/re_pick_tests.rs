//! Tests of an inspecting interceptor's re-pick as the traffic takes it
//! (`_InterceptorAI` @0x8d404-0x8d43a, see
//! [`interceptor`](crate::ai::interceptor)): the state-7 block's reach
//! reads the re-picked target (@0x8f11e), so a re-pick away from the
//! player within its reach scans nothing; of a pick looked over in its
//! own think ([`Decision::looked_over`], @0x8f11e-0x8f394); and of the
//! pick's record of the last-inspected ship, which the re-pick leaves
//! standing ([`Decision::inspection_pick`], @0x8ce3b).

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

/// A pick of `ship` the think looked over at once: the look-over drives
/// the scan, the pick the record.
fn looked_over(ship: ShipRef) -> Decision {
    Decision {
        goal: Goal::Idle,
        clears_target: true,
        looked_over: Some(ship),
        inspection_pick: Some(ship),
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
fn a_look_over_alone_records_nothing() {
    // A re-pick looked over at once (@0x8f11e) writes no +0x90.
    let re_picked = Decision {
        inspection_pick: None,
        ..looked_over(ShipRef::Player)
    };
    let (line, inspected) = after_tick(Goal::Idle, re_picked);
    assert_eq!(line, [OnLine::Scan(NpcId(1))], "still a scan");
    assert_eq!(inspected, None);
}

#[test]
fn a_fresh_pick_re_picked_in_the_same_think_records_the_pick() {
    // A traders' interceptor whose first search misses the player flying
    // as a pirate (the enemy class roll, `Rand(50)` @0x8a993) picks NPC 2
    // to inspect, and the re-pick's roll then turns it on the player:
    // +0x90 names NPC 2 all the same (@0x8ce3b).
    use crate::ai::fixture::{PIRATES, TRADERS, govts, n, player, ship};
    use crate::ai::interceptor::Interceptor;
    use crate::traffic::npc::AiType;
    let govts = govts(0, 0);
    let mut traffic = Traffic::with_interval(1);
    let mut cop = ship(0, TRADERS, AiType::Interceptor, 0.0, 0.0);
    cop.goal = Goal::Idle;
    let cop = traffic.add_npc_at(cop, 1);
    let mut trader = ship(0, TRADERS, AiType::WimpyTrader, 500.0, 0.0);
    trader.no_ai = true;
    let trader = traffic.add_npc_at(trader, 2);
    let mut pirate_player = player(0.0, -1000.0);
    pirate_player.combat_govt = Some(PIRATES);
    let world = World {
        govts: &govts,
        player: Some(pirate_player),
        ..World::new(&[])
    };
    traffic.tick_in(
        &Interceptor::default(),
        world,
        &[],
        &mut Draws::of(&[1, 1, 0]),
    );
    let after = traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == cop)
        .expect("there");
    assert_eq!(after.goal, Goal::Attack(ShipRef::Player), "re-picked");
    assert_eq!(after.inspected, Some(n(trader.0)), "the pick's");
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

/// The travel stellar of one interceptor holding stellar 140, inspecting
/// `inspected` 50 off with the player there too, once it takes
/// `decision`.
fn travel_after(inspected: ShipRef, decision: Decision) -> Option<crate::catalog::StellarId> {
    let mut traffic = Traffic::with_interval(1);
    let mut cop = npc(1, ShipStats::new(FAST, &[]));
    cop.goal = Goal::Inspect(inspected);
    cop.travel = Some(crate::catalog::StellarId(140));
    traffic.npcs = vec![cop];
    let world = World {
        player: Some(crate::ai::fixture::player(50.0, 0.0)),
        ..World::new(&[])
    };
    traffic.tick_in(&Fixed(decision), world, &[], &mut Draws::of(&[]));
    traffic.npcs[0].travel
}

#[test]
fn scanning_the_player_clears_the_travel_stellar() {
    // Goal 7 within reach of the player: goal 0, +0x6c -1, then
    // `_ScanPlayer` (@0x8f369-0x8f38a).
    let done = Decision {
        goal: Goal::Idle,
        ..Decision::default()
    };
    assert_eq!(travel_after(ShipRef::Player, done), None);
}

#[test]
fn looking_over_another_ship_keeps_it() {
    // Within reach of any other ship: goal 0, +0x6c kept (@0x8f391).
    let done = Decision {
        goal: Goal::Idle,
        ..Decision::default()
    };
    assert_eq!(
        travel_after(ShipRef::Npc(NpcId(5)), done),
        Some(crate::catalog::StellarId(140))
    );
    let re_picked = Decision {
        goal: Goal::Inspect(ShipRef::Npc(NpcId(5))),
        re_picked: true,
        ..Decision::default()
    };
    assert_eq!(
        travel_after(ShipRef::Player, re_picked),
        Some(crate::catalog::StellarId(140)),
        "a re-pick scans nothing"
    );
}
