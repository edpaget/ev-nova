//! Tests that a ship the attack block drops to goal 0 fires sub 0's
//! `_AIEscortFireUnprovoked` (@0x8521e-0x85236) at the target the frame
//! ends with, as the traffic's tick runs it: the fleet drop clears the
//! target first (@0x8e381-0x8e399, @0x8e820-0x8e838), so it fires nothing;
//! the ally drop keeps it (@0x8e2d2), so it fires its turret at the ally.
#![allow(clippy::float_cmp)]

use std::collections::BTreeSet;

use super::{Traffic, World};
use crate::ai::fixture::{PIRATES, TRADERS, n, player, ship};
use crate::ai::{Goal, NovaAi};
use crate::catalog::{WeaponId, WeaponRecord};
use crate::combat::ShipRef;
use crate::combat::armament::{Armament, Trigger};
use crate::combat::weapon::WeaponSpec;
use crate::escort::{EscortDuty, EscortOrder, FormationSlot};
use crate::fleet::OrderWord;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::Ranged;
use nova_sim_catalog::fixture::weapon;

/// A turret (140) reaching 300 pixels.
fn turret() -> Armament {
    Armament::new([(
        WeaponSpec::new(&WeaponRecord {
            guidance: 4,
            speed: 1000,
            count: 30,
            energy_dmg: 5,
            ..weapon(140)
        }),
        1,
    )])
}

/// NPC 1 at the centre, armed with [`turret`], attacking `target` on its
/// stored word Attack.
fn attacking(mut npc: Npc, target: ShipRef) -> Npc {
    npc.berth = 1;
    npc.armament = turret();
    npc.order_word = OrderWord::of(Some(EscortOrder::Attack));
    npc.goal = Goal::Attack(target);
    npc.target = Some(target);
    npc
}

/// NPC 1, the player's escort on Attack.
fn players_escort() -> Npc {
    let mut escort = ship(1, TRADERS, AiType::Warship, 0.0, 0.0);
    escort.escort = Some(EscortDuty {
        formation: FormationSlot::default(),
        order: Some(EscortOrder::Attack),
    });
    escort
}

/// A ship of `govt`, `id`, 100 above the centre, thinking nothing.
fn bystander(id: u32, govt: crate::catalog::GovtId) -> Npc {
    let mut npc = ship(id, govt, AiType::Warship, 0.0, -100.0);
    npc.berth = id;
    npc.no_ai = true;
    npc
}

/// Traffic of `npcs` one tick on by Nova's AI, each thinking every tick,
/// the player at `(x, y)`.
fn ticked(npcs: Vec<Npc>, (x, y): (f32, f32)) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = npcs;
    let world = World {
        player: Some(player(x, y)),
        ..World::new(&[])
    };
    traffic.tick_holding(
        &NovaAi::default(),
        world,
        &[],
        &BTreeSet::new(),
        &mut Ranged::of(&[]),
    );
    traffic
}

#[test]
fn a_players_escort_whose_attack_on_a_fellow_escort_is_dropped_fires_nothing_that_tick() {
    // The fellow's +0x9a is its own (@0x8e322 → @0x8e81d).
    let mut fellow = bystander(2, PIRATES);
    fellow.escort = Some(EscortDuty {
        formation: FormationSlot::default(),
        order: None,
    });
    let traffic = ticked(
        vec![attacking(players_escort(), n(2)), fellow],
        (0.0, 90_000.0),
    );
    let escort = &traffic.npcs()[0];
    assert_eq!((escort.goal, escort.target), (Goal::Idle, None));
    assert_eq!(escort.trigger, Trigger::default(), "no target to fire at");
}

#[test]
fn a_players_escort_whose_attack_on_the_player_is_dropped_fires_nothing_that_tick() {
    // +0x9a 0 is +0x70 0 (@0x8e306 → @0x8e381).
    let traffic = ticked(
        vec![attacking(players_escort(), ShipRef::Player)],
        (0.0, -100.0),
    );
    let escort = &traffic.npcs()[0];
    assert_eq!((escort.goal, escort.target), (Goal::Idle, None));
    assert_eq!(escort.trigger, Trigger::default(), "no target to fire at");
}

/// Decides `0`, and fires its primary only while its target is NPC 2.
#[derive(Debug)]
struct FiresAtTwo(crate::ai::Decision);

impl crate::ai::Behaviour for FiresAtTwo {
    fn decide(
        &self,
        _npc: &Npc,
        _around: &crate::ai::Surroundings,
        _chance: &mut dyn crate::chance::Chance,
    ) -> Goal {
        self.0.goal
    }

    fn decision(
        &self,
        _npc: &Npc,
        _around: &crate::ai::Surroundings,
        _chance: &mut dyn crate::chance::Chance,
    ) -> crate::ai::Decision {
        self.0
    }

    fn trigger(&self, npc: &Npc, _around: &crate::ai::Surroundings) -> Trigger {
        Trigger {
            primary: npc.target == Some(n(2)),
            ..Trigger::default()
        }
    }
}

#[test]
fn the_trigger_reads_the_target_the_frame_ends_with() {
    // A held target (the ally drop's +0x70, @0x8e2d2) is the low level's
    // to fire at, whatever the ship held before the think.
    for before in [None, Some(n(5))] {
        let mut npc = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        npc.target = before;
        let mut traffic = Traffic::with_interval(1);
        traffic.npcs = vec![npc, bystander(2, PIRATES)];
        let held = crate::ai::Decision {
            holds_target: Some(n(2)),
            ..crate::ai::Decision::default()
        };
        let world = World {
            player: Some(player(0.0, 90_000.0)),
            ..World::new(&[])
        };
        traffic.tick_holding(
            &FiresAtTwo(held),
            world,
            &[],
            &BTreeSet::new(),
            &mut Ranged::of(&[]),
        );
        let npc = &traffic.npcs()[0];
        assert_eq!(npc.target, Some(n(2)), "{before:?}");
        assert!(npc.trigger.primary, "{before:?}: fired at the held target");
    }
}

#[test]
fn a_fleet_escort_whose_attack_on_an_ally_is_dropped_fires_its_turret_at_it_that_tick() {
    // The world holds no government records, so a ship of its own
    // government is allied (`_GovtAllies` @0x4e4e): the ally drop writes
    // goal and sub 0 alone (@0x8e2d2), keeping +0x70.
    let mut lead = ship(9, PIRATES, AiType::Warship, 0.0, 200.0);
    lead.berth = 9;
    lead.no_ai = true;
    let mut escort = attacking(ship(1, PIRATES, AiType::Warship, 0.0, 0.0), n(2));
    escort.leader = Some(NpcId(9));
    let traffic = ticked(vec![escort, bystander(2, PIRATES), lead], (0.0, 90_000.0));
    let escort = &traffic.npcs()[0];
    assert_eq!((escort.goal, escort.target), (Goal::Idle, Some(n(2))));
    assert_eq!(escort.trigger.only, Some(WeaponId(140)), "its turret pick");
    assert!(escort.trigger.point_defence);
}
