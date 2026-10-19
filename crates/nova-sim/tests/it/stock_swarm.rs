//! The stock swarming ships, as the traffic reads their ship types (see
//! `nova_sim::ai::swarm`). Skips, passing, when `NOVA_DATA` is unset.
//!
//! - 155 of the 288 stock ship types have `shïp` `Flags2` 0x0001 and
//!   swarm, among them the Fed Viper (144) and Fed Anaconda (145), the
//!   Cargo Drone (130), Firebird (155), Thunderhead (157), Vell-os Dart
//!   and Arrow (173, 174), Krypt Pod (176) and the Rebel fighters.
//! - A Fed Viper and a Fed Anaconda of one government attacking the
//!   player: the lower berth leads, and outside 165 the other flies at a
//!   point ahead of it.

use crate::common;

use nova_data::GameData;
use nova_sim::ai::fire::Manoeuvre;
use nova_sim::ai::{Behaviour, Goal, PlayerSide, Surroundings};
use nova_sim::catalog::GovtId;
use nova_sim::chance::Chance;
use nova_sim::combat::ShipRef;
use nova_sim::combat::armament::Arsenal;
use nova_sim::flight::ShipState;
use nova_sim::geometry::Vec2;
use nova_sim::traffic::npc::Npc;
use nova_sim::traffic::table::kind;
use nova_sim::traffic::{Traffic, World};
use nova_sim::{PilotCatalog, ShipId};
use nova_sim_base::fixture::Ranged;

/// The Fed Viper.
const VIPER: ShipId = ShipId(144);
/// The Fed Anaconda.
const ANACONDA: ShipId = ShipId(145);

fn opened() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

#[test]
fn stock_swarming_ships_are_155_of_288() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let ships = data.ships();
    let swarming = ships
        .iter()
        .filter(|ship| arsenal.hull(ship.id).swarms)
        .count();
    assert_eq!((swarming, ships.len()), (155, 288));
    for id in [130, 144, 145, 155, 157, 173, 174, 176, 177] {
        assert!(arsenal.hull(ShipId(id)).swarms, "{id}");
    }
    assert!(!arsenal.hull(ShipId(128)).swarms, "the Shuttle doesn't");
}

/// Every NPC attacks the player.
#[derive(Debug)]
struct AttacksPlayer;

impl Behaviour for AttacksPlayer {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Attack(ShipRef::Player)
    }

    fn target(&self, _npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        Some(ShipRef::Player)
    }
}

/// A stock `ship` of the Federation (128) at `at`, as the traffic spawns
/// it, attacking the player.
fn federal(data: &GameData, ship: ShipId, at: Vec2) -> Npc {
    let record = data
        .ships()
        .into_iter()
        .find(|record| record.id == ship)
        .expect("the ship type");
    let kind = kind(&record, &data.outfits(), &Arsenal::read(data));
    let mut npc = nova_sim::traffic::fixture::npc(0, kind.stats);
    npc.ship = ship;
    npc.hull = kind.hull;
    npc.armament = kind.armament;
    npc.rounds = kind.rounds;
    npc.govt = Some(GovtId(128));
    npc.goal = Goal::Attack(ShipRef::Player);
    npc.target = Some(ShipRef::Player);
    npc.state = ShipState {
        position: at,
        ..ShipState::default()
    };
    npc
}

/// The player at rest at the centre.
fn player() -> PlayerSide {
    PlayerSide {
        state: ShipState::default(),
        condition: nova_sim::combat::hull::Condition::Intact,
        reserves: nova_sim::Reserves::full(100.0, 100.0, 100.0),
        hull: nova_sim::combat::hull::HullSpec::default(),
        handling: nova_sim::stats::ShipStats::default().handling,
        rating: 0,
        yardstick: 2,
        untargetable: false,
        blessing: nova_sim::rank::Blessing::NONE,
        ship: ShipId(128),
        combat_govt: None,
        target: None,
        class: nova_sim::escort::EscortClass::Freighter,
        boarded: false,
    }
}

#[test]
fn a_stock_fed_anaconda_follows_the_fed_viper_below_it_in_the_ship_table() {
    let Some(data) = opened() else {
        return;
    };
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(federal(&data, VIPER, Vec2::new(0.0, 800.0)));
    traffic.add_npc(federal(&data, ANACONDA, Vec2::new(300.0, 800.0)));
    let arsenal = Arsenal::read(&data);
    let world = World {
        player: Some(player()),
        weapons: &arsenal,
        ..World::new(&[])
    };
    traffic.tick_in(&AttacksPlayer, world, &[], &mut Ranged::of(&[]));
    let npcs = traffic.npcs();
    assert_eq!(
        npcs.iter()
            .map(|npc| (npc.ship, npc.berth, npc.swarm_leader))
            .collect::<Vec<_>>(),
        [(VIPER, 1, None), (ANACONDA, 2, Some(1))]
    );
    let flying = |npc: &Npc| {
        npc.pursuit
            .frame
            .and_then(|frame| frame.last())
            .map(|body| body.manoeuvre)
    };
    assert_eq!(flying(&npcs[1]), Some(Manoeuvre::Swarm));
    assert_ne!(flying(&npcs[0]), Some(Manoeuvre::Swarm));
}
