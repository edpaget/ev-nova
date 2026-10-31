//! What a ship's think makes of the goal `_HandleShip`'s slot test leaves
//! with no target (`Traffic::let_go_at_step`, @0x33865-0x33888): a ship in
//! the fight goal with +0x70 -1 decides as one whose target its own think
//! would drop, as every AI here tests the slot before it reads the target:
//! `_EscortAI` @0x83a9b-0x83ad2, `_PirateWarshipAI` @0x8c2fc-0x8c35d (goal 4
//! with -1 falls to the same search @0x8c35d).

use crate::ai::fire::FireRules;
use crate::ai::fixture::{PIRATES, TRADERS, around, govts, n, ship, sites};
use crate::ai::pirate::PirateWarship;
use crate::ai::{Behaviour, CarriedAi, EscortAi, FleetEscortAi, Goal, Help};
use crate::bay::Carrier;
use crate::chance::NeverFires;
use crate::combat::ShipRef;
use crate::escort::{EscortDuty, EscortOrder, FormationSlot};
use crate::fleet::OrderWord;
use crate::rulebook::RuleSource;
use crate::traffic::npc::{AiType, Npc, NpcId};

/// The ship gone, in no list.
const GONE: ShipRef = ShipRef::Npc(NpcId(99));

/// The goals a ship may hold on a ship gone.
const GOALS: [Goal; 6] = [
    Goal::Idle,
    Goal::Attack(GONE),
    Goal::Snipe(GONE),
    Goal::Flee(GONE),
    Goal::Plunder {
        ship: GONE,
        hopeless: false,
    },
    Goal::Formation { guard: Some(GONE) },
];

/// The ship `base` in `goal`, holding the gone ship as its target and
/// let go of it: what `behaviour` decides of each is the same, among
/// `others`, provoked or not, and the target it then writes.
fn decides_alike(behaviour: &impl Behaviour, base: &Npc, others: &[Npc]) {
    let sites = sites();
    let govts = govts(0, 0);
    for goal in GOALS {
        for provoked in [0.0, 10.0] {
            let holding = Npc {
                goal,
                provoked,
                target: Some(GONE),
                ..base.clone()
            };
            let let_go = Npc {
                target: None,
                ..holding.clone()
            };
            let think = |npc: &Npc| {
                let npcs: Vec<Npc> = std::iter::once(npc.clone())
                    .chain(others.iter().cloned())
                    .collect();
                let around = around(&sites, &npcs, &govts, (0.0, 5000.0), 0);
                let decision = behaviour.decision(&npcs[0], &around, &mut NeverFires);
                let flown = Npc {
                    goal: decision.goal,
                    target: npcs[0].target.filter(|_| !decision.clears_target),
                    ..npcs[0].clone()
                };
                let target = behaviour.target(&flown, &around);
                let trigger = behaviour.trigger(&flown, &around);
                (decision, target, trigger)
            };
            assert_eq!(think(&holding), think(&let_go), "{goal:?} {provoked}");
        }
    }
}

#[test]
fn the_players_escort_thinks_alike() {
    for order in [
        None,
        Some(EscortOrder::Defend),
        Some(EscortOrder::Attack),
        Some(EscortOrder::Hold),
    ] {
        for escort_ai in [RuleSource::Engine, RuleSource::Bible] {
            let mut escort = ship(1, TRADERS, AiType::Warship, 0.0, 4900.0);
            escort.escort = Some(EscortDuty {
                formation: FormationSlot::default(),
                order,
            });
            let ai = EscortAi {
                escort_ai,
                fire: FireRules::default(),
            };
            decides_alike(&ai, &escort, &[]);
        }
    }
}

#[test]
fn a_fleet_escort_thinks_alike() {
    let lead = ship(2, TRADERS, AiType::Warship, 100.0, 0.0);
    for order in [
        None,
        Some(EscortOrder::Defend),
        Some(EscortOrder::Attack),
        Some(EscortOrder::Hold),
        Some(EscortOrder::Dock),
    ] {
        let mut escort = ship(1, TRADERS, AiType::Warship, 0.0, 0.0);
        escort.leader = Some(lead.id);
        escort.order_word = OrderWord::of(order);
        decides_alike(
            &FleetEscortAi::default(),
            &escort,
            std::slice::from_ref(&lead),
        );
    }
}

#[test]
fn a_carried_fighter_thinks_alike() {
    for carrier_goal in [Goal::Idle, Goal::Attack(n(7))] {
        let mut carrier = ship(2, TRADERS, AiType::Warship, 100.0, 0.0);
        carrier.goal = carrier_goal;
        let enemy = ship(7, PIRATES, AiType::Warship, 200.0, 0.0);
        let mut fighter = ship(1, TRADERS, AiType::Warship, 0.0, 0.0);
        fighter.carrier = Some(Carrier {
            ship: ShipRef::Npc(carrier.id),
            window: 100.0,
            reach: 30.0,
        });
        decides_alike(&CarriedAi::default(), &fighter, &[carrier, enemy]);
    }
}

#[test]
fn a_pirate_thinks_alike_with_something_to_pick_or_nothing() {
    let pirate = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
    let prey = ship(2, TRADERS, AiType::WimpyTrader, 100.0, 0.0);
    decides_alike(&PirateWarship::default(), &pirate, &[]);
    decides_alike(&PirateWarship::default(), &pirate, &[prey]);
}

#[test]
fn an_assister_that_let_go_keeps_its_goal_with_no_block_to_run() {
    // No target, no block (@0x8f3f3, @0x8f6e9), the goal kept: a ship that
    // let go at its step never reaches `repair_dock`'s give-up.
    use crate::traffic::World;
    use crate::traffic::plunder::{HighLevel, high_level};
    let sites = sites();
    let world = World::new(&sites);
    for help in [Help::Refuel, Help::Repair] {
        let mut assister = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        assister.goal = Goal::Assist(help);
        let npcs = [assister];
        let around = world.around(&npcs);
        assert_eq!(
            high_level(&npcs[0], &around, world.flight.inertial_dampener),
            HighLevel::Skipped,
            "{help:?}"
        );
    }
}
