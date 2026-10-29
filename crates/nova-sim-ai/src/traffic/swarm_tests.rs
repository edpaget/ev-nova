//! Swarming through the traffic (`_AIFindSwarmLeader` and
//! `_LowLevelAIHandler`'s substate 0x12; see [`swarm`](crate::ai::swarm)):
//! three warships of one swarming class and one government attack the
//! player, the lowest berth leads, and outside 165 the other two fly at a
//! point ahead of it, firing their missiles whatever their facing.

use crate::Rulebook;
use crate::ai::PlayerSide;
use crate::ai::fire::Manoeuvre;
use crate::ai::swarm::SWARM_AHEAD;
use crate::ai::{Goal, NovaAi};
use crate::catalog::ShipId;
use crate::catalog::{GovtId, WeaponId, WeaponRecord};
use crate::combat::ShipRef;
use crate::combat::armament::Armament;
use crate::combat::weapon::WeaponSpec;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::stats::ShipStats;
use crate::traffic::npc::AiType;
use crate::traffic::npc::Npc;
use crate::traffic::{Traffic, World};
use crate::trig;
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_catalog::fixture::weapon;

/// A homing missile (guidance 1) reaching 1000 pixels.
const MISSILE: i16 = 131;

/// A warship of a swarming class of government 128 at `at`, facing
/// `heading`, attacking the player with a homing missile.
fn swarmer(at: Vec2, heading: f32) -> Npc {
    let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
    npc.ai_type = AiType::Warship;
    npc.govt = Some(GovtId(128));
    npc.hull.swarms = true;
    npc.hull.strength = 100.0;
    npc.goal = Goal::Attack(ShipRef::Player);
    npc.target = Some(ShipRef::Player);
    npc.provoked = 1.0;
    npc.aggression = 2;
    npc.armament = Armament::new([(
        WeaponSpec::new(&WeaponRecord {
            guidance: 1,
            speed: 1000,
            count: 100,
            ..weapon(MISSILE)
        }),
        1,
    )]);
    npc.state.position = at;
    npc.state.heading = heading;
    npc
}

/// Three swarmers at berths 1-3, 600 below the player at the centre, side
/// by side, the first facing up and the others down and away.
fn traffic() -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(swarmer(Vec2::new(0.0, 600.0), 0.0));
    traffic.add_npc(swarmer(Vec2::new(-200.0, 600.0), 180.0));
    traffic.add_npc(swarmer(Vec2::new(200.0, 600.0), 180.0));
    traffic
}

/// The player at rest at the centre.
fn player() -> PlayerSide {
    PlayerSide {
        state: ShipState::default(),
        condition: crate::combat::hull::Condition::Intact,
        reserves: crate::reserves::Reserves::full(100.0, 100.0, 100.0),
        hull: crate::combat::hull::HullSpec::default(),
        handling: ShipStats::default().handling,
        rating: 0,
        yardstick: 2,
        untargetable: false,
        blessing: crate::ai::Blessing::NONE,
        ship: ShipId(128),
        combat_govt: None,
        target: None,
        class: crate::escort::EscortClass::Freighter,
        boarded: false,
        speed: 0.0,
        cloak: crate::cloak::Cloak::default(),
        area_cloak: false,
    }
}

/// `traffic` one tick on, the player at rest at the centre.
fn tick(traffic: &mut Traffic, draws: &mut Draws) {
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    let world = World {
        player: Some(player()),
        ..World::new(&[])
    };
    traffic.tick_in(&ai, world, &[], draws);
}

#[test]
fn the_lowest_berth_of_three_swarmers_leads_the_other_two() {
    let mut traffic = traffic();
    let mut draws = Draws::of(&[0; 64]);
    tick(&mut traffic, &mut draws);
    let named: Vec<(u32, Option<u32>)> = traffic
        .npcs()
        .iter()
        .map(|npc| (npc.berth, npc.swarm_leader))
        .collect();
    assert_eq!(named, [(1, None), (2, Some(1)), (3, Some(1))]);
}

/// The manoeuvre and heading of the last body of `npc`'s frame.
fn flying(npc: &Npc) -> Option<(Manoeuvre, i16)> {
    npc.pursuit
        .frame
        .and_then(|frame| frame.last())
        .map(|body| (body.manoeuvre, body.heading))
}

#[test]
fn outside_165_the_two_led_fly_at_a_point_ahead_of_their_swarm_leader_firing_missiles() {
    let mut traffic = traffic();
    let mut draws = Draws::of(&[0; 64]);
    let before: Vec<Npc> = traffic.npcs().to_vec();
    tick(&mut traffic, &mut draws);
    let npcs = traffic.npcs();
    assert_eq!(
        flying(&npcs[0]).map(|(manoeuvre, _)| manoeuvre),
        Some(Manoeuvre::Approach),
        "berth 1 leads no one and approaches"
    );
    // Berth 1 faces up: the point is 15 top speeds above it.
    let leader = &before[0];
    let aim = leader.state.position
        + trig::accel(
            trig::whole(leader.state.heading),
            leader.stats.handling.max_speed * SWARM_AHEAD,
        );
    for (was, now) in before.iter().zip(npcs).skip(1) {
        assert_eq!(
            flying(now),
            Some((
                Manoeuvre::Swarm,
                trig::intercept_angle(was.state.position, aim)
            )),
            "berth {}",
            now.berth
        );
        assert_eq!(now.trigger.only, Some(WeaponId(MISSILE)), "facing away");
    }
    // Over the ticks each turns to the heading it wants.
    let off = |npc: &Npc| {
        flying(npc)
            .map(|(_, heading)| trig::angular_difference(trig::whole(npc.state.heading), heading))
    };
    let first: Vec<Option<i16>> = npcs.iter().skip(1).map(off).collect();
    for _ in 0..10 {
        tick(&mut traffic, &mut draws);
    }
    for (npc, first) in traffic.npcs().iter().skip(1).zip(first) {
        assert_eq!(
            flying(npc).map(|(manoeuvre, _)| manoeuvre),
            Some(Manoeuvre::Swarm)
        );
        assert!(off(npc) < first, "{:?} closes from {first:?}", off(npc));
    }
}
