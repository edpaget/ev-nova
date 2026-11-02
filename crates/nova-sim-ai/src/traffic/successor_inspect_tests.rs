//! A trader successor keeps its lost interceptor lead's inspection, as the
//! traffic's tick flies it with Nova's AI deciding: the succession copies
//! the lead's goal and travel stellar (`_AICheckParentExists`
//! @0x893b0-0x893e1, [`succession`](crate::traffic::succession)), the
//! wimpy trader's think keeps goal 7 unprovoked (@0x8b06c, @0x8b1ac,
//! [`trader_keep`]), and goal 7's high level, type-free, flies the
//! approach and ends it within reach, scanning the player there
//! (@0x8f11e-0x8f38a), or with its ship gone (@0x8f134 → @0x8f3bc)
//! ([`inspect_high`]).
//!
//! [`trader_keep`]: crate::ai::trader_keep
//! [`inspect_high`]: crate::ai::inspect_high
#![allow(clippy::float_cmp)]

use super::on_line::OnLine;
use super::{Traffic, World};
use crate::ai::fixture::{PIRATES, POLICE, TRADERS, govts, n, player, ship};
use crate::ai::interceptor::within_reach;
use crate::ai::{Goal, NovaAi};
use crate::catalog::StellarId;
use crate::combat::ShipRef;
use crate::combat::fixture::strike;
use crate::combat::hull::Condition;
use crate::govt::Governments;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::Draws;

/// A world of `govts` with the player at (400, 400), its record clean.
fn world(govts: &Governments) -> World<'_> {
    World {
        govts,
        player: Some(player(400.0, 400.0)),
        system_govt: Some(TRADERS),
        ..World::new(&[])
    }
}

/// `traffic` a tick on by Nova's AI among `govts`, every draw 0.
fn tick(traffic: &mut Traffic, govts: &Governments) {
    traffic.tick_in(
        &NovaAi::default(),
        world(govts),
        &[],
        &mut Draws::of(&[0; 64]),
    );
}

fn of(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("in the system")
}

/// `id` destroyed and gone from the system, its record kept.
fn destroy(traffic: &mut Traffic, id: NpcId) {
    if let Some(npc) = traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) {
        npc.condition = Condition::Destroyed;
    }
    traffic.remove_destroyed(id);
}

/// A traffic of a police interceptor lead (berth 1) at the centre, bound
/// for stellar 128, inspecting `inspected` (berth 5, with no AI of its
/// own) or the player with none, and its wimpy-trader follower of the
/// police too (berth 2), 40 to its side: the traffic, the lead, the
/// follower and the ship inspected.
fn fleet(inspected: Option<Npc>) -> (Traffic, NpcId, NpcId, ShipRef) {
    fleet_of(AiType::WimpyTrader, inspected)
}

/// [`fleet`], its follower of `ai_type`.
fn fleet_of(ai_type: AiType, inspected: Option<Npc>) -> (Traffic, NpcId, NpcId, ShipRef) {
    let mut traffic = Traffic::with_interval(1);
    let inspected = inspected.map_or(ShipRef::Player, |mut other| {
        other.no_ai = true;
        n(traffic.add_npc_at(other, 5).0)
    });
    let mut lead = ship(0, POLICE, AiType::Interceptor, 0.0, 0.0);
    lead.hull.inherent_ai = 4;
    lead.hull.signed_mass = 150;
    lead.goal = Goal::Inspect(inspected);
    lead.travel = Some(StellarId(128));
    let lead = traffic.add_npc_at(lead, 1);
    let mut follower = ship(0, POLICE, ai_type, 40.0, 0.0);
    follower.hull.inherent_ai = match ai_type {
        AiType::WimpyTrader => 1,
        AiType::BraveTrader => 2,
        AiType::Warship => 3,
        AiType::Interceptor => 4,
    };
    follower.hull.signed_mass = 10;
    follower.leader = Some(lead);
    follower.goal = Goal::Follow { lead, guard: None };
    let follower = traffic.add_npc_at(follower, 2);
    (traffic, lead, follower, inspected)
}

#[test]
fn a_trader_successor_keeps_its_leads_inspection_and_scans_the_player_within_reach() {
    let govts = govts(0, 0);
    let (mut traffic, lead, follower, _) = fleet(None);
    destroy(&mut traffic, lead);
    tick(&mut traffic, &govts);
    let successor = of(&traffic, follower);
    assert_eq!(successor.leader, None);
    assert_eq!(successor.goal, Goal::Inspect(ShipRef::Player));
    assert_eq!(successor.travel, Some(StellarId(128)));
    traffic.take_on_line();
    let mut ended = None;
    for at in 0..600 {
        let off = player(400.0, 400.0).state.position - of(&traffic, follower).state.position;
        tick(&mut traffic, &govts);
        let line = traffic.take_on_line();
        let now = of(&traffic, follower);
        if now.goal != Goal::Inspect(ShipRef::Player) {
            ended = Some((
                at,
                within_reach(off),
                now.goal,
                now.target,
                now.travel,
                line,
            ));
            break;
        }
    }
    let (at, near, goal, target, travel, line) = ended.expect("it closes in and ends it");
    assert!(at > 10, "it flies the approach first, ending at tick {at}");
    assert!(near, "it ended within reach");
    assert_eq!((goal, target, travel), (Goal::Idle, None, None));
    assert!(
        line.contains(&OnLine::Scan(follower)),
        "its scan of the player (@0x8f38a): {line:?}"
    );
}

#[test]
fn a_trader_successor_inspecting_a_ship_that_leaves_lets_it_go() {
    let govts = govts(0, 0);
    let far = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 2000.0);
    let (mut traffic, lead, follower, inspected) = fleet(Some(far));
    destroy(&mut traffic, lead);
    tick(&mut traffic, &govts);
    assert_eq!(of(&traffic, follower).goal, Goal::Inspect(inspected));
    if let ShipRef::Npc(id) = inspected {
        destroy(&mut traffic, id);
    }
    tick(&mut traffic, &govts);
    let successor = of(&traffic, follower);
    assert_eq!((successor.goal, successor.target), (Goal::Idle, None));
    assert_eq!(
        successor.travel,
        Some(StellarId(128)),
        "+0x6c kept (@0x8f3a5)"
    );
}

#[test]
fn a_warship_successor_hit_while_inspecting_flies_to_its_attacker_and_lets_it_go() {
    // `_WarshipAI` keeps goal 7 provoked (@0x8bb2d, @0x8be10); the hit
    // rewrites +0x70 (`_DamageShip` @0x3bcf6), and the state-7 block flies
    // to it and ends there: goal 0, +0x70 -1 (@0x8f11e, @0x8f391). The
    // pirate sits off-axis, as the approach settles about 104 off when it
    // comes straight on.
    let govts = govts(0, 0);
    let far = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 2000.0);
    let (mut traffic, lead, follower, inspected) = fleet_of(AiType::Warship, Some(far));
    let mut pirate = ship(0, PIRATES, AiType::Warship, 600.0, -500.0);
    pirate.no_ai = true;
    let pirate = traffic.add_npc_at(pirate, 6);
    destroy(&mut traffic, lead);
    tick(&mut traffic, &govts);
    assert_eq!(of(&traffic, follower).goal, Goal::Inspect(inspected));
    traffic.answer(
        &NovaAi::default(),
        &world(&govts),
        &[strike(n(follower.0), n(pirate.0))],
    );
    assert_eq!(of(&traffic, follower).target, Some(n(pirate.0)));
    tick(&mut traffic, &govts);
    assert_eq!(
        of(&traffic, follower).goal,
        Goal::Inspect(n(pirate.0)),
        "flown to the attacker"
    );
    let mut ended = None;
    for at in 0..600 {
        let at_pirate = of(&traffic, pirate).state.position;
        let off = at_pirate - of(&traffic, follower).state.position;
        tick(&mut traffic, &govts);
        let now = of(&traffic, follower);
        if now.goal != Goal::Inspect(n(pirate.0)) {
            ended = Some((at, within_reach(off), now.goal, now.target));
            break;
        }
    }
    let (at, near, goal, target) = ended.expect("it closes in and lets it go");
    assert!(at > 10, "it flies the approach first, ending at tick {at}");
    assert!(near, "it ended within reach");
    assert_eq!((goal, target), (Goal::Idle, None));
}

#[test]
fn an_inspection_of_the_player_moved_away_within_its_reach_scans_nothing() {
    // The state-7 block reads the hitter on +0x70 (@0x8f11e), not the
    // player within reach: no `_ScanPlayer` (@0x8f38a).
    let govts = govts(0, 0);
    let mut traffic = Traffic::with_interval(1);
    let mut pirate = ship(0, PIRATES, AiType::Warship, -600.0, -500.0);
    pirate.no_ai = true;
    let pirate = traffic.add_npc_at(pirate, 6);
    let mut inspector = ship(0, POLICE, AiType::Warship, 380.0, 380.0);
    inspector.goal = Goal::Inspect(ShipRef::Player);
    inspector.target = Some(n(pirate.0));
    inspector.provoked = 5.0;
    let inspector = traffic.add_npc_at(inspector, 2);
    traffic.take_on_line();
    tick(&mut traffic, &govts);
    assert_eq!(of(&traffic, inspector).goal, Goal::Inspect(n(pirate.0)));
    let line = traffic.take_on_line();
    assert!(
        !line.contains(&OnLine::Scan(inspector)),
        "no scan of the player: {line:?}"
    );
}
