//! The traffic's cloak think through goal 8: `_AIDispatch` calls
//! `_AIHandleCloaking` whenever the AI wait +0x4c is 0 or below
//! (@0x8fc9e-0x8fcaa), which it is through a hyperspace arrival's glide
//! and a gate burst (see [`cloak_situation`](super::cloak_situation)).

use super::{Traffic, World};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::chance::{Chance, NeverFires};
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

/// The cloak, after a fly half and a think half ([`fly_then_think`]), of
/// the player's hired escort at `at`,
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
    fly_then_think(&mut traffic, &world);
    traffic.npcs()[0].cloak
}

/// Each ship's low level at its slot in `world`, which marks each escort's
/// close formation ([`Npc::close_form`]), and a fly half, then a think
/// half, whose parent rule reads the mark.
fn fly_then_think(traffic: &mut Traffic, world: &World) {
    traffic.fly_tick(world, &mut NeverFires);
    traffic.think_holding(&Keep, *world, &[], &mut Draws::of(&[]));
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
fn the_players_escort_without_a_cloak_rides_the_area_cloaked_players_cloak() {
    // `_HasCloak`'s area branch (@0x964c-0x96bf): in close formation with
    // its cloaked, area-cloaked parent it has a cloak, and the parent rule
    // cloaks it. It was in close formation before the player cloaked.
    let ride = |area: bool, close_form: bool| {
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
        ship.close_form = close_form;
        let mut traffic = Traffic::with_interval(1);
        traffic.npcs = vec![ship];
        let mut player = crate::ai::fixture::player(0.0, 0.0);
        player.cloak = CLOAKED;
        player.area_cloak = area;
        let world = World {
            player: Some(player),
            ..World::new(&[])
        };
        fly_then_think(&mut traffic, &world);
        traffic.npcs()[0].cloak
    };
    assert_eq!(ride(true, true), CLOAKING);
    assert_eq!(ride(false, true), Cloak::default(), "no area cloak");
    assert_eq!(
        ride(true, false),
        Cloak::default(),
        "out of close formation, it can't see its cloaked player to close in"
    );
}

#[test]
fn the_players_escort_out_of_close_formation_takes_no_parent_rule() {
    assert_eq!(
        escort_thought(true, Vec2::new(5000.0, 5000.0), CLOAKED),
        Cloak::default()
    );
}

/// A ship with a cloak, keeping slot 0 of a formation beside NPC 2 at the
/// centre with `leader`'s cloak and mode, `own_leader` its leader or not,
/// escorting the player or not as `escorts_player` says; the player
/// cloaked at the centre. Its cloak after a fly half and a think half
/// ([`fly_then_think`]).
fn led_thought(leader: Npc, own_leader: bool, escorts_player: bool) -> Cloak {
    use crate::escort::{EscortDuty, FormationSlot};
    let slot = FormationSlot {
        slot: 0,
        ships: 1,
        spacing: 30.0,
    };
    let mut ship = npc(1, ShipStats::default());
    ship.cloak_gear = CloakGear {
        device: true,
        ..CloakGear::default()
    };
    if own_leader {
        ship.leader = Some(leader.id);
        ship.fleet_slot = Some(slot);
    }
    if escorts_player {
        ship.goal = Goal::Formation { guard: None };
        ship.escort = Some(EscortDuty {
            formation: slot,
            order: None,
        });
    } else {
        ship.goal = Goal::Follow {
            lead: leader.id,
            guard: None,
        };
    }
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![ship, leader];
    let mut player = crate::ai::fixture::player(0.0, 0.0);
    player.cloak = CLOAKED;
    let world = World {
        player: Some(player),
        ..World::new(&[])
    };
    fly_then_think(&mut traffic, &world);
    traffic.npcs()[0].cloak
}

/// NPC 2 at the centre, its cloak `cloak`.
fn leader_npc(cloak: Cloak) -> Npc {
    let mut leader = npc(2, ShipStats::default());
    leader.cloak = cloak;
    leader
}

#[test]
fn the_players_escort_reads_the_player_even_if_it_names_an_npc_leader() {
    // The parent rule reads the ship's parent, +0x9a (@0x83633-0x8366d),
    // its single leader slot: the player (0) while it escorts the player
    // ([`Npc::leader_ref`]), as its formation flight flies at the player.
    assert_eq!(
        led_thought(leader_npc(Cloak::default()), true, true),
        CLOAKING,
        "the cloaked player, not its uncloaked NPC leader"
    );
    assert_eq!(
        led_thought(leader_npc(Cloak::default()), false, true),
        CLOAKING,
        "no NPC leader: the cloaked player"
    );
}

#[test]
fn a_fleet_escort_takes_no_parent_rule_from_a_cloaked_leader_it_cannot_see() {
    // A leader waiting in its gate is unseen (`_ShipVisibleToShip`
    // @0x972d), so its escort brakes rather than keeping close formation.
    assert_eq!(
        led_thought(leader_npc(CLOAKED), true, false),
        CLOAKING,
        "seen: its cloak shows it its cloaked leader"
    );
    let mut emerging = leader_npc(CLOAKED);
    emerging.mode = Mode::Emerging { ticks_left: 5 };
    assert_eq!(
        led_thought(emerging, true, false),
        Cloak::default(),
        "unseen"
    );
}

/// A cloaked ship of `Flags2` [`CLOAK_PRESSING`] in slot `berth`, on
/// goal 4 at `target` (+0x70 too), at the centre.
fn pressing(berth: u32, target: crate::combat::ShipRef) -> (Npc, u32) {
    let mut ship = npc(1, ShipStats::default());
    ship.goal = Goal::Attack(target);
    ship.target = Some(target);
    ship.cloak = CLOAKED;
    ship.cloak_gear = CloakGear {
        device: true,
        flags2: CLOAK_PRESSING,
        ..CloakGear::default()
    };
    (ship, berth)
}

/// A ship in slot `berth` of `traffic`, 500 off the centre: more than
/// 165 off on an axis.
fn far_ship(traffic: &mut Traffic, berth: u32) -> crate::traffic::npc::NpcId {
    let mut ship = npc(0, ShipStats::default());
    ship.state.position = Vec2::new(500.0, 0.0);
    traffic.add_npc_at(ship, berth)
}

#[test]
fn a_cloak_think_on_goal_4_reads_a_gone_targets_record_where_it_was() {
    // `_AIHandleCloaking` reads the +0x70 record's place with no +0xb8
    // test (@0x8355e-0x835d7): a slot freed in the ship pass is still
    // named by a lower slot's +0x70 at its next think, and the record
    // stands where the ship was.
    let mut traffic = Traffic::with_interval(1);
    let gone = far_ship(&mut traffic, 4);
    let aim = crate::combat::ShipRef::Npc(gone);
    let (ship, berth) = pressing(2, aim);
    let below = traffic.add_npc_at(ship, berth);
    let (ship, berth) = pressing(6, aim);
    let above = traffic.add_npc_at(ship, berth);
    traffic.remove_destroyed(gone);
    // The slot above's `_HandleShip` came after the wreck's display.
    traffic.let_go_at_step(above, true);
    traffic.think_holding(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
    let cloak = |id| {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("there")
            .cloak
    };
    assert_eq!(cloak(below), CLOAKED, "pressing on the record");
    assert_eq!(cloak(above).direction, -1, "let go: nothing to press");
}

#[test]
fn a_cloak_think_reads_the_goal_and_target_the_last_tick_left() {
    // The think runs at the dispatch's head (@0x8fcaa), before the high
    // level writes the frame's goal (@0x901e5).
    #[derive(Debug)]
    struct Idles;
    impl Behaviour for Idles {
        fn decide(&self, _: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            Goal::Idle
        }
    }
    let (ship, _) = pressing(1, crate::combat::ShipRef::Player);
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![ship];
    let world = World {
        player: Some(crate::ai::fixture::player(500.0, 0.0)),
        ..World::new(&[])
    };
    traffic.think_holding(&Idles, world, &[], &mut Draws::of(&[]));
    assert_eq!(traffic.npcs()[0].goal, Goal::Idle, "it turned idle");
    assert_eq!(traffic.npcs()[0].cloak, CLOAKED, "on last tick's attack");
}

#[test]
fn a_ship_whose_ai_wait_runs_does_not_think_its_cloak() {
    // `_AIDispatch` calls `_AIHandleCloaking` only with +0x4c at 0 or
    // below (@0x8fc9e-0x8fca5).
    let waiting = |ai_wait| {
        let mut ship = npc(1, ShipStats::default());
        ship.cloak = CLOAKED;
        ship.ai_wait = ai_wait;
        ship.cloak_gear = CloakGear {
            device: true,
            ..CloakGear::default()
        };
        let mut traffic = Traffic::with_interval(1);
        traffic.npcs = vec![ship];
        traffic.think_holding(&Keep, World::new(&[]), &[], &mut Draws::of(&[]));
        traffic.npcs()[0].cloak
    };
    assert_eq!(waiting(10), CLOAKED, "no think");
    assert_eq!(waiting(0).direction, -1, "nothing keeps it");
}
