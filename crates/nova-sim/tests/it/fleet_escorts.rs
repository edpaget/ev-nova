//! A fleet's escort through the traffic and Nova's AI
//! (`nova_sim::ai::fleet_escort`, `_EscortAI` @0x838d2-0x83f5d under an
//! NPC parent), flying the order its lead issues on its order frame
//! (`nova_sim::ai::escort_orders`, `_AIIssueEscortOrders`
//! @0x8986d-0x89b8d): until the first, it keeps formation; a lead
//! attacking with its shield full orders a medium escort to Attack, and
//! it attacks a ship attacking the lead; an idle lead orders Return, and
//! it follows the lead, guarding against that ship with its turret; a
//! lead whose odds are not yet cached orders its warship escort to keep
//! formation. Keeping formation, it holds its slot in its lead's
//! formation as the lead cruises (`_AICalcFormationPositions`, goal 0xa),
//! or, by the other reading of `npc_formation`, follows within 100 of it.
//! Its lead disabled, the heaviest escort leads in its place and the other
//! pauses, keeping formation on it, until a draw releases it to its kept
//! order (`_AICheckParentExists` @0x8926a-0x89501, goal 0x13).

use std::collections::BTreeSet;

use nova_sim::ai::regroup::Kept;
use nova_sim::ai::{Behaviour, Decision, FleetEscortAi, Surroundings};
use nova_sim::catalog::{EscortClass, GovtId, GovtRecord, WeaponId, WeaponRecord};
use nova_sim::chance::Chance;
use nova_sim::combat::ShipRef;
use nova_sim::combat::armament::Armament;
use nova_sim::combat::weapon::WeaponSpec;
use nova_sim::escort::{FORMATION_NUDGE, FORMATION_SLACK};
use nova_sim::fleet::OrderWord;
use nova_sim::flight::{FlightRules, NpcFormation};
use nova_sim::geometry::Vec2;
use nova_sim::stats::ShipStats;
use nova_sim::traffic::autopilot::FOLLOW_DISTANCE;
use nova_sim::traffic::npc::AiType;
use nova_sim::traffic::regroup::RELEASE_DRAW;
use nova_sim::{Goal, Governments, NovaAi, Npc, NpcId, Traffic, World};
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_catalog::fixture::{govt, weapon};

/// The lead, the traffic's first ship.
const LEAD: NpcId = NpcId(0);
/// Its escort, the second.
const ESCORT: NpcId = NpcId(1);
/// A raider attacking the lead, the third.
const RAIDER: NpcId = NpcId(2);
/// The lead's quarry, the fourth.
const QUARRY: NpcId = NpcId(3);
/// The escort's turret.
const TURRET: WeaponId = WeaponId(140);

/// The fleet's government and the raiders', at war.
fn govts() -> Governments {
    Governments::new([
        GovtRecord {
            enemies: [1, -1, -1, -1],
            classes: [0, -1, -1, -1],
            ..govt(128)
        },
        GovtRecord {
            enemies: [0, -1, -1, -1],
            classes: [1, -1, -1, -1],
            ..govt(129)
        },
    ])
}

/// A ship of `govt` and `ai_type` at (`x`, `y`) with `goal`.
fn ship(govt: i16, ai_type: AiType, (x, y): (f32, f32), goal: Goal) -> Npc {
    let mut npc = nova_sim::traffic::fixture::npc(0, ShipStats::new(FAST, &[]));
    npc.govt = Some(GovtId(govt));
    npc.ai_type = ai_type;
    npc.state.position = Vec2::new(x, y);
    npc.goal = goal;
    npc.target = goal.quarry();
    npc.hull.strength = 100.0;
    npc
}

/// The traffic: the lead at the centre with `goal`, its shield full, its
/// escort of `class` beside it with a turret reaching 300 pixels, a
/// raider 150 above attacking the lead, and the lead's quarry far below.
fn traffic(goal: Goal, class: EscortClass) -> Traffic {
    let mut escort = ship(128, AiType::Warship, (40.0, 0.0), Goal::Idle);
    escort.leader = Some(LEAD);
    escort.class = class;
    escort.armament = Armament::new([(
        WeaponSpec::new(&WeaponRecord {
            guidance: 4,
            speed: 1000,
            count: 30,
            energy_dmg: 5,
            ..weapon(TURRET.0)
        }),
        1,
    )]);
    escort.hull.stock_reach = Some(332.0);
    let mut traffic = Traffic::with_interval(1);
    for npc in [
        ship(128, AiType::Warship, (0.0, 0.0), goal),
        escort,
        ship(
            129,
            AiType::Warship,
            (0.0, -150.0),
            Goal::Attack(ShipRef::Npc(LEAD)),
        ),
        ship(129, AiType::WimpyTrader, (0.0, 3000.0), Goal::Idle),
    ] {
        traffic.add_npc(npc);
    }
    traffic
}

/// `traffic` a tick on by Nova's AI on `frame`, every ship kept in
/// place. The lead, in berth 1, issues on frames that are multiples of 8.
fn tick(traffic: &mut Traffic, frame: i16) {
    let govts = govts();
    let world = World {
        govts: &govts,
        frame,
        ..World::new(&[])
    };
    let still: BTreeSet<NpcId> = traffic.npcs().iter().map(|npc| npc.id).collect();
    traffic.tick_holding(
        &NovaAi::default(),
        world,
        &[],
        &still,
        &mut Draws::of(&[0; 64]),
    );
}

fn escort(traffic: &Traffic) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == ESCORT)
        .expect("in the system")
}

/// Keeping formation on the lead, guarding against the raider.
fn guarding() -> Goal {
    Goal::Follow {
        lead: LEAD,
        guard: Some(ShipRef::Npc(RAIDER)),
    }
}

#[test]
fn until_its_leads_first_order_a_fleet_escort_keeps_formation() {
    let mut traffic = traffic(Goal::Attack(ShipRef::Npc(QUARRY)), EscortClass::Medium);
    tick(&mut traffic, 1);
    let escort = escort(&traffic);
    assert_eq!(escort.order_word, OrderWord::NONE, "-1 from its spawn");
    assert_eq!(escort.goal, guarding());
}

#[test]
fn a_lead_attacking_with_its_shield_full_orders_its_medium_escort_to_attack_a_ship_attacking_it() {
    let mut traffic = traffic(Goal::Attack(ShipRef::Npc(QUARRY)), EscortClass::Medium);
    tick(&mut traffic, 0);
    let escort = escort(&traffic);
    assert_eq!(escort.order_word, OrderWord(2), "Attack");
    assert_eq!(escort.goal, Goal::Attack(ShipRef::Npc(RAIDER)));
    assert_eq!(escort.target, Some(ShipRef::Npc(RAIDER)));
}

#[test]
fn a_lead_whose_odds_are_not_yet_cached_orders_its_warship_escort_to_keep_formation() {
    let mut traffic = traffic(Goal::Attack(ShipRef::Npc(QUARRY)), EscortClass::Warship);
    tick(&mut traffic, 0);
    let escort = escort(&traffic);
    assert_eq!(escort.order_word, OrderWord(0), "formation");
    assert_eq!(escort.goal, guarding());
}

#[test]
fn an_idle_lead_orders_return_and_its_escort_follows_it_guarding_with_its_turret() {
    let mut traffic = traffic(Goal::Idle, EscortClass::Medium);
    tick(&mut traffic, 0);
    let escort = escort(&traffic);
    assert_eq!(escort.order_word, OrderWord(3), "Return");
    assert_eq!(escort.goal, guarding());
    assert_eq!(escort.target, Some(ShipRef::Npc(RAIDER)));
    assert_eq!(escort.trigger.only, Some(TURRET), "its turret pick fired");
    assert!(escort.trigger.point_defence);
}

/// The lead keeps its goal; its escort flies `_EscortAI` under it.
#[derive(Debug)]
struct Cruise;

impl Behaviour for Cruise {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        if npc.leader.is_some() {
            FleetEscortAi::default().decision(npc, around, chance)
        } else {
            Decision {
                goal: npc.goal,
                ..Decision::default()
            }
        }
    }
}

/// A lead cruising right at 2 a tick, holding its course, and its escort
/// a little behind it, flown 300 ticks under `flight` off the lead's order
/// frames (so the escort keeps formation): after each of the last 100,
/// the escort's place off its slot and off its lead.
fn cruising(flight: FlightRules) -> Vec<(Vec2, Vec2)> {
    let mut lead = ship(128, AiType::Warship, (0.0, 0.0), Goal::Hold);
    lead.state.velocity = Vec2::new(2.0, 0.0);
    lead.state.heading = 90.0;
    let mut escort = ship(128, AiType::Warship, (-60.0, 50.0), Goal::Idle);
    escort.state.velocity = Vec2::new(2.0, 0.0);
    escort.state.heading = 90.0;
    escort.leader = Some(LEAD);
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(lead);
    traffic.add_npc(escort);
    let world = World {
        flight,
        frame: 1,
        ..World::new(&[])
    };
    let mut offs = Vec::new();
    for tick in 0..300 {
        traffic.tick_in(&Cruise, world, &[], &mut Draws::of(&[0; 64]));
        let (lead, escort) = (&traffic.npcs()[0], &traffic.npcs()[1]);
        assert_eq!(escort.order_word, OrderWord::NONE);
        if tick >= 200 {
            let slot = escort
                .fleet_slot
                .expect("a slot in its lead's formation")
                .position(&lead.state);
            offs.push((
                escort.state.position - slot,
                escort.state.position - lead.state.position,
            ));
        }
    }
    offs
}

#[test]
fn a_fleet_escort_keeps_its_slot_on_a_cruising_lead() {
    let flight = FlightRules::default();
    let thrust = flight.handling(ShipStats::new(FAST, &[]).handling).accel;
    let slack = FORMATION_SLACK + FORMATION_NUDGE * thrust;
    for (tick, (off_slot, _)) in cruising(flight).into_iter().enumerate() {
        assert!(
            off_slot.x.abs() <= slack && off_slot.y.abs() <= slack,
            "tick {tick}: {off_slot:?} off its slot"
        );
    }
}

#[test]
fn by_the_earlier_follow_a_fleet_escort_keeps_within_100_of_its_cruising_lead() {
    let flight = FlightRules {
        npc_formation: NpcFormation::Earlier,
        ..FlightRules::default()
    };
    let offs = cruising(flight);
    for (tick, (_, off_lead)) in offs.iter().enumerate() {
        assert!(
            off_lead.length() <= FOLLOW_DISTANCE,
            "tick {tick}: {off_lead:?} off its lead"
        );
    }
    let slack = FORMATION_SLACK + FORMATION_NUDGE;
    assert!(
        offs.iter()
            .any(|(off_slot, _)| off_slot.x.abs() > slack || off_slot.y.abs() > slack),
        "not on its slot"
    );
}

/// The heavy escort, the second ship.
const HEAVY: NpcId = NpcId(1);
/// The light escort, the third.
const LIGHT: NpcId = NpcId(2);

/// A lead cruising right at 2 a tick holding its course, and a heavy and
/// a light medium escort keeping formation on it, cruising with it, the
/// light one holding the word its lead last gave it, Return.
fn cruising_fleet() -> Traffic {
    let cruise = |x: f32, y: f32, goal: Goal| {
        let mut npc = ship(128, AiType::Warship, (x, y), goal);
        npc.state.velocity = Vec2::new(2.0, 0.0);
        npc.state.heading = 90.0;
        npc.class = EscortClass::Medium;
        npc
    };
    let follow = Goal::Follow {
        lead: LEAD,
        guard: None,
    };
    let mut heavy = cruise(-60.0, 50.0, follow);
    heavy.leader = Some(LEAD);
    heavy.hull.signed_mass = 300;
    let mut light = cruise(-60.0, -50.0, follow);
    light.leader = Some(LEAD);
    light.hull.signed_mass = 100;
    light.order_word = OrderWord(3);
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(cruise(0.0, 0.0, Goal::Hold));
    traffic.add_npc(heavy);
    traffic.add_npc(light);
    traffic
}

fn npc(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("in the system")
}

#[test]
fn a_disabled_leads_heaviest_escort_leads_and_the_other_pauses_in_formation_then_takes_up_its_order()
 {
    let mut traffic = cruising_fleet();
    let govts = govts();
    // Off the order frames, so no lead issues a word.
    let world = World {
        govts: &govts,
        frame: 1,
        ..World::new(&[])
    };
    for _ in 0..20 {
        traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::of(&[0; 64]));
    }
    traffic
        .npcs_mut()
        .iter_mut()
        .find(|npc| npc.id == LEAD)
        .expect("there")
        .condition = nova_sim::combat::hull::Condition::Disabled;
    let held = || Draws::of(&[0; 64]).keyed(RELEASE_DRAW, 99);
    traffic.tick_in(&NovaAi::default(), world, &[], &mut held());
    let heavy = npc(&traffic, HEAVY);
    assert_eq!(heavy.leader, None, "it leads");
    assert_eq!(heavy.goal, Goal::Hold, "its lead's hold, taken over");
    let light = npc(&traffic, LIGHT);
    assert_eq!(light.leader, Some(HEAVY));
    assert_eq!(
        light.goal,
        Goal::Regroup {
            kept: Kept::Formation { guard: None }
        }
    );
    let flight = FlightRules::default();
    let thrust = flight.handling(ShipStats::new(FAST, &[]).handling).accel;
    let slack = FORMATION_SLACK + FORMATION_NUDGE * thrust;
    for tick in 0..150 {
        traffic.tick_in(&NovaAi::default(), world, &[], &mut held());
        let light = npc(&traffic, LIGHT);
        assert!(
            matches!(light.goal, Goal::Regroup { .. }),
            "tick {tick}: still paused"
        );
        if tick >= 100 {
            let slot = light
                .fleet_slot
                .expect("a slot in its new lead's formation")
                .position(&npc(&traffic, HEAVY).state);
            let off = light.state.position - slot;
            assert!(
                off.x.abs() <= slack && off.y.abs() <= slack,
                "tick {tick}: {off:?} off its slot on the heavy escort"
            );
        }
    }
    let mut releasing = Draws::of(&[0; 64]).keyed(RELEASE_DRAW, 0);
    traffic.tick_in(&NovaAi::default(), world, &[], &mut releasing);
    let light = npc(&traffic, LIGHT);
    assert_eq!(light.order_word, OrderWord(3), "the word it kept");
    assert_eq!(
        light.goal,
        Goal::Follow {
            lead: HEAVY,
            guard: None
        },
        "released, it forms on its new lead under its word"
    );
}
