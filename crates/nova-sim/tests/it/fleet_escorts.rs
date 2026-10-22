//! A fleet's escort through the traffic and Nova's AI
//! (`nova_sim::ai::fleet_escort`, `_EscortAI` @0x838d2-0x83f5d under an
//! NPC parent), flying the order its lead issues on its order frame
//! (`nova_sim::ai::escort_orders`, `_AIIssueEscortOrders`
//! @0x8986d-0x89b8d): until the first, it keeps formation; a lead
//! attacking with its shield full orders a medium escort to Attack, and
//! it attacks a ship attacking the lead; an idle lead orders Return, and
//! it follows the lead, guarding against that ship with its turret; a
//! lead whose odds are not yet cached orders its warship escort to keep
//! formation.

use std::collections::BTreeSet;

use nova_sim::catalog::{EscortClass, GovtId, GovtRecord, WeaponId, WeaponRecord};
use nova_sim::combat::ShipRef;
use nova_sim::combat::armament::Armament;
use nova_sim::combat::weapon::WeaponSpec;
use nova_sim::fleet::OrderWord;
use nova_sim::geometry::Vec2;
use nova_sim::stats::ShipStats;
use nova_sim::traffic::npc::AiType;
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
