//! A fleet's escort through the traffic and Nova's AI
//! (`nova_sim::ai::fleet_escort`, `_EscortAI` @0x838d2-0x83f5d under an
//! NPC parent): while its lead attacks, the escort attacks a ship
//! attacking the lead (the stand-in Attack order); while its lead idles,
//! it follows the lead, guarding against that ship with its turret.

use std::collections::BTreeSet;

use nova_sim::catalog::{GovtId, GovtRecord, WeaponId, WeaponRecord};
use nova_sim::combat::ShipRef;
use nova_sim::combat::armament::Armament;
use nova_sim::combat::weapon::WeaponSpec;
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

/// The traffic: the lead at the centre with `goal`, its escort beside it
/// with a turret reaching 300 pixels, a raider 150 above attacking the
/// lead, and the lead's quarry far below.
fn traffic(goal: Goal) -> Traffic {
    let mut escort = ship(128, AiType::Warship, (40.0, 0.0), Goal::Idle);
    escort.leader = Some(LEAD);
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

/// `traffic` a tick on by Nova's AI, every ship kept in place.
fn tick(traffic: &mut Traffic) {
    let govts = govts();
    let world = World {
        govts: &govts,
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

#[test]
fn while_its_lead_attacks_a_fleet_escort_attacks_a_ship_attacking_the_lead() {
    let mut traffic = traffic(Goal::Attack(ShipRef::Npc(QUARRY)));
    tick(&mut traffic);
    let escort = escort(&traffic);
    assert_eq!(escort.goal, Goal::Attack(ShipRef::Npc(RAIDER)));
    assert_eq!(escort.target, Some(ShipRef::Npc(RAIDER)));
}

#[test]
fn while_its_lead_idles_a_fleet_escort_follows_it_guarding_with_its_turret() {
    let mut traffic = traffic(Goal::Idle);
    tick(&mut traffic);
    let escort = escort(&traffic);
    assert_eq!(
        escort.goal,
        Goal::Follow {
            lead: LEAD,
            guard: Some(ShipRef::Npc(RAIDER))
        }
    );
    assert_eq!(escort.target, Some(ShipRef::Npc(RAIDER)));
    assert_eq!(escort.trigger.only, Some(TURRET), "its turret pick fired");
    assert!(escort.trigger.point_defence);
}
