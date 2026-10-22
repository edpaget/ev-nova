//! Tests of the voice cues the traffic's think half queues for the
//! session ([`Traffic::take_voice_cues`]), as [`NovaAi`] flies the
//! player's escorts: each think's [`Decision::voice`], in the order the
//! ships think (see [`escort_voice`](crate::escort_voice)).
//!
//! [`Decision::voice`]: crate::ai::Decision::voice

use super::{Traffic, World};
use crate::ai::Goal;
use crate::ai::nova::NovaAi;
use crate::combat::ShipRef;
use crate::escort::{EscortDuty, EscortOrder, FormationSlot};
use crate::escort_voice::VoiceCue;
use crate::geometry::Vec2;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{AiType, Npc, NpcId};
use nova_sim_base::fixture::{Draws, FAST};

/// The player's escort `id` at the centre, following `order`.
fn players_escort(id: u32, order: Option<EscortOrder>) -> Npc {
    let mut ship = npc(id, ShipStats::new(FAST, &[]));
    ship.ai_type = AiType::Warship;
    ship.escort = Some(EscortDuty {
        formation: FormationSlot::default(),
        order,
    });
    ship
}

/// NPC `id`, 100 above the centre, attacking the player.
fn raider(id: u32) -> Npc {
    let mut ship = npc(id, ShipStats::new(FAST, &[]));
    ship.ai_type = AiType::Warship;
    ship.state.position = Vec2::new(0.0, 100.0);
    ship.goal = Goal::Attack(ShipRef::Player);
    ship.target = Some(ShipRef::Player);
    ship
}

/// The cues a think half of `ships` queues, the player at the centre.
fn cues(ships: Vec<Npc>) -> (Traffic, Vec<(NpcId, VoiceCue)>) {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = ships;
    let world = World {
        player: Some(crate::ai::fixture::player(0.0, 0.0)),
        ..World::new(&[])
    };
    traffic.tick_in(&NovaAi::default(), world, &[], &mut Draws::of(&[]));
    let cues = traffic.take_voice_cues();
    (traffic, cues)
}

#[test]
fn each_think_of_the_players_escort_queues_its_cue_in_turn_and_a_take_empties_them() {
    let (mut traffic, queued) = cues(vec![
        players_escort(1, Some(EscortOrder::Defend)),
        raider(2),
        players_escort(3, Some(EscortOrder::Hold)),
    ]);
    assert_eq!(
        queued,
        [
            (NpcId(1), VoiceCue::NewThreat),
            (NpcId(3), VoiceCue::Thought)
        ],
        "the raider, no escort of the player's, cues nothing"
    );
    assert_eq!(traffic.take_voice_cues(), [], "taken");
}

#[test]
fn a_ship_that_thinks_nothing_queues_no_cue() {
    let mut idle = players_escort(1, Some(EscortOrder::Defend));
    idle.no_ai = true;
    assert_eq!(cues(vec![idle, raider(2)]).1, []);
}
