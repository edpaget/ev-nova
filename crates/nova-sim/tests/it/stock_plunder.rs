//! Stock pirates' plunder (`_AIPlunderShipContents` @0x83fab-0x84a2d and
//! the docking of `_LowLevelAIHandler`'s substate 0xf; see
//! `nova_sim::ai::plunder_contents` and `nova_sim::traffic::plunder`).
//! Skips, passing, when `NOVA_DATA` is unset.
//!
//! - Eight stock missions fail when pirates plunder the player (`mïsn`
//!   `Flags` 0x8000): 719-726, the Pirate Offshoot strand.
//! - Only the Unrelenting (`shïp` 374) carries marines among its default
//!   items: two of outfit 227 (+25) in item slot 4, which the original
//!   counts at outfit 131's `ModVal` (the Medium Blaster Turret's 131):
//!   262, against 50 by the item's own outfit.
//! - No stock class has a negative `Strength`.
//! - A stock Pirate Starbridge docks with a disabled Federation Shuttle
//!   carrying cargo, takes it into its free holds, and drifts off.

use crate::common;

use nova_data::GameData;
use nova_sim::combat::armament::Arsenal;
use nova_sim::geometry::Vec2;
use nova_sim::govt::Governments;
use nova_sim::traffic::npc::{AiType, Npc, NpcId};
use nova_sim::traffic::plunder::PirateEvent;
use nova_sim::traffic::table::kind;
use nova_sim::traffic::{Traffic, World};
use nova_sim::{Condition, Goal, GovtId, NovaAi, PilotCatalog, RuleSource, Rulebook, ShipId};
use nova_sim_base::fixture::Draws;

const FEDERATION: GovtId = GovtId(128);
const PIRATES: GovtId = GovtId(137);
const SHUTTLE: ShipId = ShipId(128);
const PIRATE_STARBRIDGE: ShipId = ShipId(148);
const UNRELENTING: ShipId = ShipId(374);

fn opened() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// NPC `id` in berth `id`, of stock ship type `ship` and `govt`, at `at`,
/// as the traffic builds its kind, of its class's AI type.
fn stock_npc(
    data: &GameData,
    arsenal: &Arsenal,
    id: u32,
    (ship, govt): (ShipId, GovtId),
    at: Vec2,
) -> Npc {
    let record = data
        .ships()
        .into_iter()
        .find(|record| record.id == ship)
        .expect("the ship type reads");
    let kind = kind(&record, &data.outfits(), arsenal);
    let mut npc = nova_sim::traffic::fixture::npc(id, kind.stats);
    npc.berth = id;
    npc.ship = ship;
    npc.hull = kind.hull;
    npc.armament = kind.armament;
    npc.rounds = kind.rounds;
    npc.holds = kind.holds;
    npc.marines = kind.marines;
    npc.govt = Some(govt);
    npc.ai_type = AiType::from_raw(record.inherent_ai);
    npc.aggression = 2;
    npc.state.position = at;
    npc
}

#[test]
fn the_pirate_offshoot_missions_alone_fail_when_pirates_plunder_the_player() {
    let Some(data) = opened() else {
        return;
    };
    let plundered: Vec<i16> = data
        .missions()
        .iter()
        .filter(|record| record.flags & 0x8000 != 0)
        .map(|record| record.id.0)
        .collect();
    assert_eq!(plundered, [719, 720, 721, 722, 723, 724, 725, 726]);
}

#[test]
fn the_unrelenting_counts_its_marines_at_its_item_slots_outfit_by_the_engine() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let outfits = data.outfits();
    let ships = data.ships();
    let carrying: Vec<ShipId> = ships
        .iter()
        .filter(|record| {
            let marines = kind(record, &outfits, &arsenal).marines;
            marines.by_outfit.crew != 0 || marines.by_outfit.defence != 0
        })
        .map(|record| record.id)
        .collect();
    assert_eq!(carrying, [UNRELENTING], "the only class with marines");
    let record = ships
        .iter()
        .find(|record| record.id == UNRELENTING)
        .expect("the Unrelenting reads");
    let marines = kind(record, &outfits, &arsenal).marines;
    assert_eq!(marines.read(RuleSource::Engine).crew, 262, "2 × 131");
    assert_eq!(marines.read(RuleSource::Bible).crew, 50, "2 × 25");
}

#[test]
fn no_stock_class_has_a_negative_strength() {
    let Some(data) = opened() else {
        return;
    };
    let negative: Vec<ShipId> = data
        .ships()
        .iter()
        .filter(|record| record.strength < 0)
        .map(|record| record.id)
        .collect();
    assert_eq!(negative, []);
}

#[test]
fn a_stock_pirate_docks_with_a_disabled_shuttle_takes_its_cargo_and_drifts() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let govts = Governments::read(&data);
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    let pirate = stock_npc(&data, &arsenal, 1, (PIRATE_STARBRIDGE, PIRATES), Vec2::ZERO);
    let mut shuttle = stock_npc(
        &data,
        &arsenal,
        2,
        (SHUTTLE, FEDERATION),
        Vec2::new(0.0, -150.0),
    );
    shuttle.condition = Condition::Disabled;
    shuttle.cargo.add(2, shuttle.holds.into());
    let (free, carried) = (u32::from(pirate.holds), shuttle.cargo.tons(2));
    let mut traffic = Traffic::with_interval(1);
    let pirate = traffic.add_npc(pirate);
    let shuttle = traffic.add_npc(shuttle);
    let world = World {
        govts: &govts,
        ..World::new(&[])
    };
    let mut events = Vec::new();
    for _ in 0..600 {
        traffic.tick_in(&ai, world, &[], &mut Draws::of(&[0; 256]));
        events.extend(traffic.take_pirate_events());
        if !events.is_empty() {
            break;
        }
    }
    let [PirateEvent::Plundered { tons, .. }] = events[..] else {
        panic!("one plunder: {events:?}");
    };
    assert_eq!(tons, free.min(carried), "into its free holds");
    let ship = |id: NpcId| {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("here")
    };
    assert_eq!(ship(pirate).cargo.tons(2), tons);
    assert_eq!(ship(shuttle).cargo.tons(2), carried - tons);
    assert_eq!(
        ship(pirate).goal,
        Goal::Drift,
        "a stranger taken silently, if at all"
    );
}
