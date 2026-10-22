//! The traffic's cloak think through goal 8: `_AIDispatch` calls
//! `_AIHandleCloaking` whenever the AI wait +0x4c is 0 or below
//! (@0x8fc9e-0x8fcaa), which it is through a hyperspace arrival's glide
//! and a gate burst (see [`cloak_situation`](super::cloak_situation)).

use super::{Traffic, World};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::chance::Chance;
use crate::cloak::{CLOAK_JUMPING, CLOAK_PRESSING, Cloak, CloakGear, FADE_FULL};
use crate::geometry::Vec2;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{Mode, Npc};
use nova_sim_base::fixture::Draws;
use nova_sim_combat::asteroid::Asteroid;

/// Keeps each NPC's goal.
#[derive(Debug)]
struct Keep;

impl Behaviour for Keep {
    fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
        npc.goal
    }
}

const CLOAKED: Cloak = Cloak {
    fade: FADE_FULL,
    direction: 0,
};

/// The cloak, after one think half, of an NPC in `mode` whose ship type
/// carries a cloak and `flags2`, starting at `start`.
fn thought(mode: Mode, flags2: u16, start: Cloak) -> Cloak {
    let mut ship = npc(1, ShipStats::default());
    ship.mode = mode;
    ship.cloak = start;
    ship.cloak_gear = CloakGear {
        device: true,
        flags2,
        ..CloakGear::default()
    };
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![ship];
    traffic.think_holding(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
    traffic.npcs()[0].cloak
}

#[test]
fn a_ship_cloaking_when_it_jumps_cloaks_through_its_glide_in() {
    for tick in [0, 12] {
        let mode = Mode::JumpingIn { tick };
        assert_eq!(
            thought(mode, CLOAK_JUMPING, Cloak::default()),
            Cloak {
                fade: 0.0,
                direction: 1
            },
            "{mode:?}"
        );
        assert_eq!(thought(mode, CLOAK_JUMPING, CLOAKED), CLOAKED, "{mode:?}");
    }
}

#[test]
fn a_ship_cloaking_when_it_jumps_cloaks_through_its_gate_burst_from_its_first_moving_tick() {
    for tick in [1, 5] {
        let mode = Mode::LeavingGate { tick };
        assert_eq!(thought(mode, CLOAK_JUMPING, CLOAKED), CLOAKED, "{mode:?}");
    }
    assert_eq!(
        thought(Mode::LeavingGate { tick: 0 }, CLOAK_JUMPING, CLOAKED).direction,
        -1,
        "tick 0 is still goal 0x15"
    );
}

#[test]
fn a_cloaked_ship_with_nothing_to_keep_it_uncloaks_in_its_glide() {
    let gliding = thought(Mode::JumpingIn { tick: 3 }, 0, CLOAKED);
    assert_eq!(gliding.direction, -1);
}

/// The cloak, after one think half, of a cloaked miner pressing on
/// (`Flags2` [`CLOAK_PRESSING`]) in a world whose slot-0 rock is `rock`.
fn mining_thought(rock: Option<Asteroid>) -> Cloak {
    let mut ship = npc(1, ShipStats::default());
    ship.goal = Goal::Mine;
    ship.cloak = CLOAKED;
    ship.cloak_gear = CloakGear {
        device: true,
        flags2: CLOAK_PRESSING,
        ..CloakGear::default()
    };
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![ship];
    let world = World {
        mining_rock: rock,
        ..World::new(&[])
    };
    traffic.think_holding(&Keep, world, &[], &mut Draws::of(&[]));
    traffic.npcs()[0].cloak
}

#[test]
fn a_cloaked_pressing_miner_keeps_its_cloak_until_the_world_shows_it_a_rock() {
    assert_eq!(mining_thought(None), CLOAKED, "no rock: MineOrder::Stop");
    let rock = Asteroid {
        position: Vec2::new(40.0, -20.0),
        velocity: Vec2::ZERO,
        frame: 0.0,
        spin: 0.0,
        strength: 30,
        kind: 2,
    };
    assert_eq!(mining_thought(Some(rock)).direction, -1, "a live rock");
}

/// The cloak, after one think half, of the player's hired escort at `at`,
/// keeping slot 0 of its formation with the player at the centre, its
/// ship type carrying a cloak or not as `device` says and no AI-cloak
/// bit, the player's cloak `player_cloak`.
fn escort_thought(device: bool, at: Vec2, player_cloak: Cloak) -> Cloak {
    use crate::escort::{EscortDuty, FormationSlot};
    let mut ship = npc(1, ShipStats::default());
    ship.goal = Goal::Formation { guard: None };
    ship.escort = Some(EscortDuty {
        formation: FormationSlot {
            slot: 0,
            ships: 1,
            spacing: 30.0,
        },
        order: None,
    });
    ship.state.position = at;
    ship.cloak_gear = CloakGear {
        device,
        ..CloakGear::default()
    };
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![ship];
    let mut player = crate::ai::fixture::player(0.0, 0.0);
    player.cloak = player_cloak;
    let world = World {
        player: Some(player),
        ..World::new(&[])
    };
    traffic.think_holding(&Keep, world, &[], &mut Draws::of(&[]));
    traffic.npcs()[0].cloak
}

/// Fading in, as a think's cloak starts it.
const CLOAKING: Cloak = Cloak {
    fade: 0.0,
    direction: 1,
};

#[test]
fn the_players_escort_with_a_cloak_cloaks_with_the_cloaked_player() {
    // `_AIHandleCloaking` @0x83633-0x8366d: AI type above 4, the parent
    // (+0x9a 0, the player) cloaked, sub-goal 0xc.
    assert_eq!(escort_thought(true, Vec2::ZERO, CLOAKED), CLOAKING);
    assert_eq!(
        escort_thought(true, Vec2::ZERO, Cloak::default()),
        Cloak::default(),
        "the player uncloaked"
    );
}

#[test]
fn the_players_escort_without_a_cloak_stays_uncloaked_by_its_cloaked_player() {
    assert_eq!(escort_thought(false, Vec2::ZERO, CLOAKED), Cloak::default());
}

#[test]
fn the_players_escort_out_of_close_formation_takes_no_parent_rule() {
    assert_eq!(
        escort_thought(true, Vec2::new(5000.0, 5000.0), CLOAKED),
        Cloak::default()
    );
}
