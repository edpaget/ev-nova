//! Tests of the attack block's drop as the traffic's tick runs it (see
//! [`attack_drop`](crate::ai::attack_drop)): the block's target -1
//! (@0x8e717) stands to the end of the frame for every thinker, so the
//! next think finds no target.

use std::collections::BTreeSet;

use super::{Traffic, World};
use crate::ai::fixture::{PIRATES, TRADERS, n, player, ship};
use crate::ai::{Goal, NovaAi};
use crate::catalog::{StellarId, WeaponRecord};
use crate::combat::ShipRef;
use crate::combat::armament::Armament;
use crate::combat::hull::Condition;
use crate::combat::weapon::WeaponSpec;
use crate::escort::{EscortDuty, FormationSlot};
use crate::traffic::ShipBehav;
use crate::traffic::npc::{AiType, MissionMark, Npc};
use nova_sim_base::fixture::Ranged;
use nova_sim_catalog::fixture::weapon;

const BREAKING_UP: Condition = Condition::Dying {
    ticks_left: 100,
    starting: false,
};

/// NPC 2, an escort of the player breaking up 500 to the right.
fn led_wreck() -> Npc {
    let mut wreck = ship(2, TRADERS, AiType::Warship, 500.0, 0.0);
    wreck.berth = 2;
    wreck.escort = Some(EscortDuty {
        formation: FormationSlot::default(),
        order: None,
    });
    wreck.condition = BREAKING_UP;
    wreck
}

/// NPC 1, defending Earth at the centre, attacking `led_wreck`.
fn defender() -> Npc {
    let mut npc = ship(1, TRADERS, AiType::Warship, 0.0, 0.0);
    npc.berth = 1;
    npc.defends = Some(StellarId(128));
    npc.goal = Goal::Attack(n(2));
    npc.target = Some(n(2));
    npc
}

/// Traffic of `npcs`, each thinking every tick.
fn traffic_of(npcs: Vec<Npc>) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = npcs;
    traffic
}

/// `traffic` one tick on by Nova's AI, the player as `side` says.
fn tick(traffic: &mut Traffic, side: &crate::ai::PlayerSide) {
    let world = World {
        player: Some(*side),
        ..World::new(&[])
    };
    traffic.tick_holding(
        &NovaAi::default(),
        world,
        &[],
        &BTreeSet::new(),
        &mut Ranged::of(&[]),
    );
}

/// `traffic`'s NPC 1, its goal and target.
fn first(traffic: &Traffic) -> (Goal, Option<ShipRef>) {
    let npc = &traffic.npcs()[0];
    (npc.goal, npc.target)
}

#[test]
fn a_defender_whose_led_target_breaks_up_attacks_the_nearer_player_on_its_next_think() {
    // The block writes target -1 and goal 0 (@0x8e717, @0x8e726), so the
    // next `_DefenseShipAI` finds no target and attacks the candidate
    // (@0x7f944-0x7f971).
    let mut traffic = traffic_of(vec![defender(), led_wreck()]);
    let near = player(100.0, 0.0);
    tick(&mut traffic, &near);
    assert_eq!(first(&traffic), (Goal::Idle, None), "@0x8e717, @0x8e726");
    tick(&mut traffic, &near);
    assert_eq!(
        first(&traffic),
        (Goal::Attack(ShipRef::Player), Some(ShipRef::Player)),
        "@0x7f944-0x7f971"
    );
}

#[test]
fn a_defender_whose_wreck_is_the_nearest_candidate_takes_it_and_drops_it_each_think() {
    // Far from the player the wreck is the candidate again, and the block
    // drops it again: the target -1 holds at each frame's end.
    let mut traffic = traffic_of(vec![defender(), led_wreck()]);
    for _ in 0..3 {
        tick(&mut traffic, &player(0.0, 90_000.0));
        assert_eq!(first(&traffic), (Goal::Idle, None));
    }
}

#[test]
fn a_mission_hunter_ends_the_frame_with_no_target_on_a_player_breaking_up() {
    // `NovaAi`'s target step answers the hunt, not the goal, as the
    // defender's answers its think: the block's -1 stands over it.
    let mut hunter = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
    hunter.mission = Some(MissionMark {
        order: None,
        slot: 1,
        behav: ShipBehav::from_code(0),
        formation: None,
    });
    // A destroying weapon, or it gives up on a player breaking up
    // (@0x8d684-0x8d6e9).
    hunter.armament = Armament::new([(
        WeaponSpec::new(&WeaponRecord {
            speed: 1000,
            count: 30,
            mass_dmg: 5,
            ..weapon(141)
        }),
        1,
    )]);
    let mut traffic = traffic_of(vec![hunter]);
    let mut side = player(100.0, 0.0);
    side.condition = BREAKING_UP;
    tick(&mut traffic, &side);
    assert_eq!(first(&traffic), (Goal::Idle, None), "@0x8e717, @0x8e726");
}
