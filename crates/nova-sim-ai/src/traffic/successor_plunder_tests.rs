//! A pirate fleet's successor keeps its lost lead's plunder, as the
//! traffic's tick flies it with Nova's AI deciding: the succession copies
//! the lead's goal, target, nav and docking (`_AICheckParentExists`
//! @0x893b0-0x893e1, [`succession`](crate::traffic::succession)), the
//! successor's type think keeps goal 0xd ([`trader_keep`], the
//! interceptor's gate), and goal 0xd's high level flies it
//! ([`plunder_high`], [`plunder::high_level`](crate::traffic::plunder::high_level)):
//! the dock, the goal-0 branch for a victim gone
//! ([`HighLevel::LetsGo`](crate::traffic::plunder::HighLevel::LetsGo),
//! @0x8e761 → @0x8eb5f) and the plunder at sub 0xf's completion.
//!
//! [`trader_keep`]: crate::ai::trader_keep
//! [`plunder_high`]: crate::ai::plunder_high
#![allow(clippy::float_cmp)]

use super::{Traffic, World};
use crate::ai::fixture::{PIRATES, TRADERS, n, ship};
use crate::ai::{Goal, NovaAi, travel};
use crate::catalog::GovtRecord;
use crate::combat::hull::Condition;
use crate::flight::FlightRules;
use crate::govt::{Governments, PLUNDERS};
use crate::traffic::npc::{AiType, Npc, NpcId};
use crate::traffic::plunder::{PirateEvent, fly_docking};
use crate::traffic::plunder_dock::{DockSub, Docking};
use nova_sim_base::fixture::Draws;
use nova_sim_catalog::fixture::govt;

/// The traders' government and the pirates', which plunders.
fn governments() -> Governments {
    Governments::new([
        govt(TRADERS.0),
        GovtRecord {
            flags: PLUNDERS,
            ..govt(PIRATES.0)
        },
    ])
}

/// The victim's slot.
const VICTIM_BERTH: u32 = 5;

/// A traffic of the lead (berth 1), a pirate warship docking in sub 9
/// with the victim, a disabled trader with a crew (berth 5) 300 below
/// it, and its follower (berth 2) of class `InherentAI` `inherent`, of
/// the pirates too, 40 to its side: the lead, the follower and the
/// victim.
fn fleet(inherent: i16) -> (Traffic, [NpcId; 3]) {
    let mut traffic = Traffic::with_interval(1);
    let mut victim = ship(0, TRADERS, AiType::WimpyTrader, 0.0, 300.0);
    victim.condition = Condition::Disabled;
    victim.hull.crew = 1;
    victim.hull.inherent_ai = 2;
    victim.no_ai = true;
    let victim = traffic.add_npc_at(victim, VICTIM_BERTH);
    let mut lead = ship(0, PIRATES, AiType::Warship, 0.0, 0.0);
    lead.hull.inherent_ai = 3;
    lead.hull.signed_mass = 150;
    lead.goal = Goal::Plunder {
        ship: n(victim.0),
        hopeless: false,
    };
    lead.target = Some(n(victim.0));
    lead.travel = Some(travel::of_slot(VICTIM_BERTH));
    lead.docking = Some(Docking {
        ship: n(victim.0),
        sub: DockSub::Approach,
    });
    let lead = traffic.add_npc_at(lead, 1);
    let mut follower = ship(0, PIRATES, AiType::from_raw(inherent), 40.0, 0.0);
    follower.hull.inherent_ai = inherent;
    follower.hull.signed_mass = 10;
    follower.leader = Some(lead);
    follower.goal = Goal::Follow { lead, guard: None };
    let follower = traffic.add_npc_at(follower, 2);
    (traffic, [lead, follower, victim])
}

/// `traffic` a tick on by Nova's AI among [`governments`], every draw 0.
fn tick(traffic: &mut Traffic, govts: &Governments) {
    let world = World {
        govts,
        ..World::new(&[])
    };
    traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::of(&[0; 64]));
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

/// The distance between `a` and `b` in `traffic`.
fn apart(traffic: &Traffic, a: NpcId, b: NpcId) -> f32 {
    let off = of(traffic, a).state.position - of(traffic, b).state.position;
    (off.x * off.x + off.y * off.y).sqrt()
}

#[test]
fn a_wimpy_successor_keeps_its_leads_plunder_and_docks() {
    // `_WimpyTraderAI` keeps goal 0xd unprovoked (@0x8b06c, @0x8b1ac), and
    // goal 0xd's high level docks it with the disabled victim (@0x8e873),
    // its nav the victim's slot (@0x8e8c9).
    let govts = governments();
    let (mut traffic, [lead, follower, victim]) = fleet(1);
    destroy(&mut traffic, lead);
    let before = apart(&traffic, follower, victim);
    tick(&mut traffic, &govts);
    let successor = of(&traffic, follower);
    assert_eq!(successor.leader, None);
    assert_eq!(successor.ai_type, AiType::WimpyTrader);
    assert_eq!(
        successor.goal,
        Goal::Plunder {
            ship: n(victim.0),
            hopeless: false
        }
    );
    assert_eq!(
        successor.docking.map(|docking| docking.ship),
        Some(n(victim.0))
    );
    assert_eq!(successor.travel, Some(travel::of_slot(VICTIM_BERTH)));
    for _ in 0..30 {
        tick(&mut traffic, &govts);
    }
    assert!(
        apart(&traffic, follower, victim) < before - 10.0,
        "it flies the dock: {} from {before}",
        apart(&traffic, follower, victim)
    );
    assert!(of(&traffic, follower).docking.is_some());
}

#[test]
fn a_wimpy_successor_whose_plundered_ship_went_steers_one_tick_at_the_record_then_drops_its_dock() {
    // The think keeps goal 0xd on the gone ship; the slot test clears
    // +0x70 and goal 0xd writes goal 0 alone (@0x8e761 → @0x8eb5f),
    // keeping the sub and nav for that tick's low level. The next tick's
    // goal 0 runs the idle block, whose high level writes sub 0.
    let govts = governments();
    let (mut traffic, [lead, follower, victim]) = fleet(1);
    destroy(&mut traffic, lead);
    tick(&mut traffic, &govts);
    let docking = of(&traffic, follower).docking;
    assert!(docking.is_some());
    let record = of(&traffic, victim).state;
    destroy(&mut traffic, victim);
    let mut flown = of(&traffic, follower).clone();
    fly_docking(&mut flown, Some(&record), 1.0, FlightRules::default(), None);
    tick(&mut traffic, &govts);
    let let_go = of(&traffic, follower);
    assert_eq!(let_go.goal, Goal::Idle, "goal 0 alone");
    assert_eq!(let_go.docking, docking, "its sub kept");
    assert_eq!(
        let_go.travel,
        Some(travel::of_slot(VICTIM_BERTH)),
        "its nav"
    );
    assert_eq!(let_go.state, flown.state, "it steered at the record");
    tick(&mut traffic, &govts);
    let idle = of(&traffic, follower);
    assert_eq!(idle.docking, None, "the next tick drops it");
    assert!(
        !matches!(idle.goal, Goal::Plunder { .. }),
        "{:?}",
        idle.goal
    );
}

#[test]
fn a_wimpy_successor_docked_completes_the_plunder() {
    // Sub 0xf's completion (@0x86f3c-0x86f93) plunders the nav and the
    // successor drifts (goal 0xe).
    let govts = governments();
    let (mut traffic, [lead, follower, victim]) = fleet(1);
    destroy(&mut traffic, lead);
    for ticks in 1..=3000 {
        tick(&mut traffic, &govts);
        let events = traffic.take_pirate_events();
        if let Some(event) = events
            .iter()
            .find(|event| matches!(event, PirateEvent::Plundered { .. }))
        {
            assert!(
                matches!(
                    event,
                    PirateEvent::Plundered { pirate, victim: plundered, .. }
                        if *pirate == follower && *plundered == victim
                ),
                "{event:?}"
            );
            assert_eq!(of(&traffic, follower).goal, Goal::Drift, "tick {ticks}");
            return;
        }
    }
    panic!("no plunder in 3000 ticks: {:?}", of(&traffic, follower));
}

#[test]
fn an_interceptor_successor_keeps_its_leads_plunder() {
    // A Pirate Viper's `InherentAI` 4: `_InterceptorAI` runs its search
    // and patrol for goal 1 or below, or 0x14, alone (@0x8cc6d-0x8cc7d),
    // so it keeps goal 0xd and docks.
    let govts = governments();
    let (mut traffic, [lead, follower, victim]) = fleet(4);
    destroy(&mut traffic, lead);
    tick(&mut traffic, &govts);
    let successor = of(&traffic, follower);
    assert_eq!(successor.ai_type, AiType::Interceptor);
    assert_eq!(
        successor.goal,
        Goal::Plunder {
            ship: n(victim.0),
            hopeless: false
        }
    );
    assert_eq!(
        successor.docking.map(|docking| docking.ship),
        Some(n(victim.0))
    );
}
