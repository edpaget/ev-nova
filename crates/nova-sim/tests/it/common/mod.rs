//! The program edge shared by the stock-data test modules.
//!
//! This module is the only place `NOVA_DATA` is read. The game data is
//! copyrighted and never committed, so each stock test asks [`nova_data`] for
//! the `Nova Files` directory and skips, passing, when it is unset. The
//! data opened ([`opened`]), a stock ship as the traffic builds it
//! ([`stock_npc`]) and the player beside it ([`player`]) are shared too,
//! and what a session raised, drained ([`drained`], [`sounds`],
//! [`messages`], [`drained_sounds`]).

use nova_sim::flight_frame::FlightEvent;
use nova_sim::{Session, SimMessage, SimSound};

use std::path::PathBuf;

use nova_data::GameData;
use nova_sim::ai::PlayerSide;
use nova_sim::combat::armament::Arsenal;
use nova_sim::flight::ShipState;
use nova_sim::geometry::Vec2;
use nova_sim::traffic::npc::{AiType, Npc};
use nova_sim::traffic::table::kind;
use nova_sim::{Condition, GovtId, PilotCatalog, ShipId};

/// The `Nova Files` directory named by `NOVA_DATA`, or `None` (after printing
/// a skip message) when it is unset.
pub fn nova_data() -> Option<PathBuf> {
    let dir = std::env::var_os("NOVA_DATA").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("skipping: NOVA_DATA not set");
    }
    dir
}

/// The stock data named by `NOVA_DATA`, opened, or `None` when it is unset.
pub fn opened() -> Option<GameData> {
    let dir = nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// NPC `id` in berth `id`, of stock ship type `ship` and `govt`, at `at`,
/// as the traffic builds its kind, of its class's AI type.
pub fn stock_npc(
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
    let kind = kind(
        &record,
        &data.outfits(),
        arsenal,
        nova_sim::RuleSource::Engine,
    );
    let mut npc = nova_sim::traffic::fixture::npc(id, kind.stats);
    npc.berth = id;
    npc.ship = ship;
    npc.hull = kind.hull;
    npc.armament = kind.armament;
    npc.rounds = kind.rounds;
    npc.holds = kind.holds;
    npc.govt = Some(govt);
    npc.ai_type = AiType::from_raw(record.inherent_ai);
    npc.state.position = at;
    npc
}

/// The player at the centre, at rest facing up, in `condition`, its
/// reserves 100 of 100 each.
pub fn player(condition: Condition) -> PlayerSide {
    PlayerSide {
        state: ShipState::default(),
        condition,
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
        speed: 0.0,
        cloak: nova_sim::cloak::Cloak::default(),
        area_cloak: false,
    }
}

/// What `session` raised since it was last drained, in its order
/// (`Session::drain_events`).
pub fn drained(session: &mut Session) -> Vec<FlightEvent> {
    let mut events = Vec::new();
    session.drain_events(&mut events);
    events
}

/// The sounds among `events`, in order.
pub fn sounds(events: &[FlightEvent]) -> Vec<SimSound> {
    events
        .iter()
        .filter_map(|event| match event {
            FlightEvent::Sound(sound) => Some(*sound),
            _ => None,
        })
        .collect()
}

/// The messages among `events`, in order.
pub fn messages(events: &[FlightEvent]) -> Vec<SimMessage> {
    events
        .iter()
        .filter_map(|event| match event {
            FlightEvent::Message(message) => Some(message.clone()),
            _ => None,
        })
        .collect()
}

/// The sounds `session` raised since it was last drained, what else it
/// raised let go.
pub fn drained_sounds(session: &mut Session) -> Vec<SimSound> {
    sounds(&drained(session))
}
