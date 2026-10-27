//! The frame hold of an escort riding its leader's area cloak
//! (`_HandleShip` @0x3413d-0x3415f, `_HasCloak` @0x964c-0x96bf), under
//! both readings of
//! [`RuleKey::AreaCloakHold`](crate::RuleKey::AreaCloakHold): see
//! [`area_cloak`](crate::area_cloak).

use std::collections::BTreeSet;

use super::fly_pass::FlyHalf;
use super::{Traffic, World};
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::chance::{Chance, NeverFires};
use crate::cloak::{CLOAK_IDLE, Cloak, CloakGear, FADE_FULL};
use crate::escort::{EscortDuty, FormationSlot};
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{Npc, NpcId};
use nova_sim_base::fixture::{Draws, FAST};

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

/// Slot 0 of a formation, 30 apart.
const SLOT: FormationSlot = FormationSlot {
    slot: 0,
    ships: 1,
    spacing: 30.0,
};

/// The player's escort (NPC 1) at `at`, keeping slot 0 of its formation,
/// with no cloak of its own, its last flight in close formation as
/// `close_form` says.
fn players_escort(at: Vec2, close_form: bool) -> Npc {
    let mut escort = npc(1, ShipStats::new(FAST, &[]));
    escort.goal = Goal::Formation { guard: None };
    escort.escort = Some(EscortDuty {
        formation: SLOT,
        order: None,
    });
    escort.state.position = at;
    escort.close_form = close_form;
    escort
}

/// A world with the player at the centre, cloaked as `cloak` says and
/// carrying an area cloak, the hold under `hold`.
fn world(cloak: Cloak, hold: RuleSource) -> World<'static> {
    let mut player = crate::ai::fixture::player(0.0, 0.0);
    player.cloak = cloak;
    player.area_cloak = true;
    World {
        player: Some(player),
        area_cloak_hold: hold,
        ..World::new(&[])
    }
}

/// One traffic tick in `world`: its think half, then its fly half (whose
/// start holds every cloak), then the fades' step. Whether NPC `id` read
/// cloaked as the hold began, and its cloak after the hold.
fn tick(traffic: &mut Traffic, world: &World, id: NpcId) -> (bool, Cloak) {
    traffic.think_holding(&Keep, *world, &[], &mut Draws::of(&[]));
    let of = |traffic: &Traffic| {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("there")
            .cloak
    };
    let cloaked = of(traffic).is_cloaked();
    let mut pass = traffic.begin_fly(world, &BTreeSet::new(), FlyHalf::Advance);
    let held = of(traffic);
    for each in pass.order().to_vec() {
        traffic.fly_npc(&mut pass, each, world, &mut NeverFires);
    }
    traffic.end_fly(pass);
    traffic.step_fades();
    (cloaked, held)
}

#[test]
fn by_the_engine_an_escort_riding_an_area_cloak_with_no_cloak_of_its_own_is_uncloaked_the_frame_it_reads_cloaked()
 {
    let world = world(CLOAKED, RuleSource::Engine);
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![players_escort(Vec2::ZERO, true)];
    let mut read_cloaked = 0;
    for frame in 0..120 {
        let (cloaked, held) = tick(&mut traffic, &world, NpcId(1));
        if cloaked {
            read_cloaked += 1;
            assert!(held.direction < 0, "frame {frame}: the hold uncloaks it");
        }
        assert!(!traffic.npcs()[0].cloak.full(), "frame {frame}: never full");
    }
    assert!(read_cloaked > 0, "its think cloaks it, so it reads cloaked");
}

#[test]
fn by_the_bible_an_escort_riding_an_area_cloak_stays_cloaked_while_it_rides() {
    let bible = world(CLOAKED, RuleSource::Bible);
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![players_escort(Vec2::ZERO, true)];
    for _ in 0..60 {
        let (cloaked, held) = tick(&mut traffic, &bible, NpcId(1));
        if cloaked {
            assert!(held.direction >= 0, "the hold keeps it");
        }
    }
    assert_eq!(traffic.npcs()[0].cloak, CLOAKED, "fully faded and held");
    assert!(traffic.npcs()[0].close_form);
    // The player uncloaks: the escort rides no longer, has no cloak, and
    // its next think uncloaks it.
    let uncloaked = world(Cloak::default(), RuleSource::Bible);
    traffic.think_holding(&Keep, uncloaked, &[], &mut Draws::of(&[]));
    assert_eq!(traffic.npcs()[0].cloak.direction, -1);
}

#[test]
fn an_npc_leaders_area_cloak_rides_its_fleet_escort_by_the_bible() {
    // A lead cloaked while it idles, with an area cloak, and its fleet
    // escort in close formation with it, by the engine's NPC formation.
    let fleet = || {
        let mut lead = npc(2, ShipStats::new(FAST, &[]));
        lead.cloak = CLOAKED;
        lead.reserves.fuel.now = 100.0;
        lead.cloak_gear = CloakGear {
            device: true,
            flags2: CLOAK_IDLE,
            area: true,
            ..CloakGear::default()
        };
        let mut escort = npc(1, ShipStats::new(FAST, &[]));
        escort.leader = Some(lead.id);
        escort.goal = Goal::Follow {
            lead: lead.id,
            guard: None,
        };
        escort.fleet_slot = Some(SLOT);
        escort.close_form = true;
        let mut traffic = Traffic::with_interval(1);
        traffic.npcs = vec![escort, lead];
        traffic
    };
    let ridden = |hold| {
        let world = World {
            area_cloak_hold: hold,
            ..World::new(&[])
        };
        let mut traffic = fleet();
        for _ in 0..60 {
            tick(&mut traffic, &world, NpcId(1));
        }
        assert!(traffic.npcs()[1].cloak.full(), "the lead holds its cloak");
        traffic.npcs()[0].cloak
    };
    assert_eq!(ridden(RuleSource::Bible), CLOAKED);
    assert!(!ridden(RuleSource::Engine).full(), "the engine's hold");
}

#[test]
fn the_hold_reads_the_ride_this_ticks_flight_picks() {
    // The hold (@0x3413d) runs after this frame's high level (@0x4429d
    // before @0x444ad), so it reads the sub-goal this tick's flight picks,
    // not the one stored by the last. Its last flight was out of close
    // formation; now, within 300 of its cloaked player, which its scanner
    // sees, it keeps close formation.
    let held = |hold| {
        let mut escort = players_escort(Vec2::new(-150.0, 0.0), false);
        escort.cloak = CLOAKED;
        escort.cloak_gear.scanner.on_screen = true;
        let mut traffic = Traffic::with_interval(1);
        traffic.npcs = vec![escort];
        let world = world(CLOAKED, hold);
        let _pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
        traffic.npcs()[0].cloak
    };
    assert_eq!(held(RuleSource::Bible), CLOAKED, "kept on this tick");
    assert_eq!(held(RuleSource::Engine).direction, -1);
    // Out of 300 it keeps no close formation this tick: no ride.
    let mut far = players_escort(Vec2::new(-450.0, 0.0), true);
    far.cloak = CLOAKED;
    far.cloak_gear.scanner.on_screen = true;
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![far];
    let _pass = traffic.begin_fly(
        &world(CLOAKED, RuleSource::Bible),
        &BTreeSet::new(),
        FlyHalf::Advance,
    );
    assert_eq!(
        traffic.npcs()[0].cloak.direction,
        -1,
        "its stored mark is no ride"
    );
}
