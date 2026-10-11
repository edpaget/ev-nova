//! Mission ships over the stock data: a stock mission with special ships,
//! started by `S` on a new pilot, puts them in the system its `ShipSyst`
//! names, and the system is set up with them there and nowhere else; a
//! stock mission of each ship goal has its goal met by the storyline
//! walker's `meet goal`; and a stock mission's escorts come in on their
//! timer and join the player's formation. Skips, passing, when
//! `NOVA_DATA` is unset.

mod common;

use nova_data::GameData;
use nova_sim::catalog::{DudeId, ShipId, TrafficCatalog};
use nova_sim::mission::ShipSystem;
use nova_sim::{
    MissionId, NeverFires, NovaAi, Pilot, PilotCatalog, Session, SetExpr, SystemId, is_landable,
};

/// `mïsn` 183, "Capture Vell-os": six ships of `düde` 130 (the Lone Big
/// Federation Ship), `ShipSyst` -3 (its `TravelStel` 128 is Earth, in Sol),
/// `ShipStart` -1 (Sol's first nav slot, Earth).
const MISSION: i16 = 183;
/// Sol.
const SOL: SystemId = SystemId(130);

/// A new stock pilot docked at the first landable stellar of its system,
/// flown and settled.
fn docked_new_pilot(data: &GameData) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let site = data
        .landing_sites(pilot.system())
        .into_iter()
        .find(is_landable)
        .expect("a landable stellar where the pilot starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(site.id.0);
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(data, docked).expect("flies");
    let _ = session.settle_script(data, &mut NeverFires);
    session
}

/// `pilot` in flight in `system`, its system set up by the first traffic
/// tick.
fn in_flight(data: &GameData, pilot: &Pilot, system: SystemId) -> Session {
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(pilot)).expect("JSON");
    save["system"] = serde_json::json!(system.0);
    save["stellar"] = serde_json::Value::Null;
    let there = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(data, there).expect("flies");
    let _ = session.settle_script(data, &mut NeverFires);
    session.tick_traffic(data, &NovaAi::default(), &mut NeverFires);
    session
}

#[test]
fn a_stock_missions_special_ships_spawn_in_their_system_and_no_other() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = docked_new_pilot(&data);
    let start = session.pilot().system();
    session.run_set(
        &SetExpr::parse(&format!("S{MISSION}")).expect("parses"),
        &mut NeverFires,
    );
    let _ = session.settle_script(&data, &mut NeverFires);
    let slot = *session
        .pilot()
        .missions()
        .holding(MissionId(MISSION))
        .first()
        .expect("started");
    let ships = &session.pilot().missions().get(slot).expect("held").ships;
    assert_eq!(ships.system, ShipSystem::At(SOL), "its TravelStel's system");
    assert_eq!(ships.left, 6);
    assert_eq!(ships.appear, -1, "ShipStart -1 runs no timer");

    let sol = in_flight(&data, session.pilot(), SOL);
    let dude_ships: Vec<ShipId> = data
        .dude(DudeId(130))
        .expect("düde 130")
        .ships
        .iter()
        .map(|&(ship, _)| ship)
        .collect();
    let earth = data
        .landing_sites(SOL)
        .first()
        .expect("Sol's first nav slot")
        .position;
    let marked: Vec<_> = sol
        .npcs()
        .iter()
        .filter(|npc| npc.mission.is_some_and(|mark| mark.slot == slot))
        .collect();
    assert_eq!(marked.len(), 6, "ShipCount of them in Sol");
    for npc in marked {
        assert!(dude_ships.contains(&npc.ship), "{:?}", npc.ship);
        assert_eq!(npc.govt.map(|govt| govt.0), Some(128), "the düde's");
        assert_eq!(npc.state.position, earth, "at Earth");
    }

    let elsewhere = if start == SOL { SystemId(131) } else { start };
    let other = in_flight(&data, session.pilot(), elsewhere);
    assert!(
        other.npcs().iter().all(|npc| npc.mission.is_none()),
        "none in {elsewhere:?}"
    );
}

/// Starts `mission` by `S` on a new pilot docked where it starts, then
/// meets its ship goal by the walker's `meet goal`: whether the walk
/// passed, and the mission's goals met after it, or none once it ended
/// (an auto-abort).
fn met_by_the_walker(data: &GameData, mission: MissionId) -> (bool, Option<bool>) {
    let mut session = docked_new_pilot(data);
    session.run_set(
        &SetExpr::parse(&format!("S{}", mission.0)).expect("parses"),
        &mut NeverFires,
    );
    let _ = session.settle_script(data, &mut NeverFires);
    let walked = nova_sim::fixture::StoryWalker::new(data)
        .walk(&mut session, &[nova_sim::fixture::Step::MeetGoal(mission)])
        .is_ok();
    let met = session
        .pilot()
        .missions()
        .iter()
        .find(|(_, active)| active.mission == mission)
        .map(|(_, active)| active.goals_met);
    (walked, met)
}

#[test]
fn a_stock_mission_of_each_ship_goal_has_it_met_by_the_walker() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let missions = data.missions();
    for goal in 0..=6 {
        let record = missions
            .iter()
            .find(|record| {
                record.ships.goal == goal && record.ships.count > 0 && record.ships.dude.is_some()
            })
            .unwrap_or_else(|| panic!("a stock mission of ship goal {goal}"));
        let (walked, met) = met_by_the_walker(&data, record.id);
        assert!(walked, "mission {} of goal {goal}", record.id.0);
        assert_ne!(met, Some(false), "mission {} of goal {goal}", record.id.0);
    }
}

/// `mïsn` 171: ships escorting the player (`ShipBehav` 1) that it must
/// escort (`ShipGoal` 3), following it (`ShipSyst` -6) and coming later
/// (`ShipStart` 1).
const ESCORTED: i16 = 171;

#[test]
fn a_stock_missions_escorts_come_in_on_their_timer_and_join_the_players_formation() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let record = data
        .missions()
        .into_iter()
        .find(|record| record.id == MissionId(ESCORTED))
        .expect("mïsn 171");
    assert_eq!(
        (
            record.ships.behav,
            record.ships.goal,
            record.ships.syst,
            record.ships.start
        ),
        (1, 3, -6, 1)
    );
    let mut session = docked_new_pilot(&data);
    session.run_set(
        &SetExpr::parse(&format!("S{ESCORTED}")).expect("parses"),
        &mut NeverFires,
    );
    let _ = session.settle_script(&data, &mut NeverFires);
    let mut flying = in_flight(&data, session.pilot(), session.pilot().system());
    let escorts = |session: &Session| {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.mission_escort())
            .count()
    };
    assert_eq!(escorts(&flying), 0, "none at setup");
    for _ in 0..30 {
        flying.tick_traffic(&data, &NovaAi::default(), &mut NeverFires);
    }
    let count = usize::try_from(record.ships.count).expect("a count");
    assert_eq!(escorts(&flying), count, "come in after the timer, joined");
    for npc in flying.npcs().iter().filter(|npc| npc.mission_escort()) {
        assert_eq!(npc.escort, None, "not hired");
        assert!(npc.formation().is_some());
    }
    assert!(flying.pilot().escorts().is_empty());
}
