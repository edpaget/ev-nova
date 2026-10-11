//! Mission ships over the stock data: a stock mission with special ships,
//! started by `S` on a new pilot, puts them in the system its `ShipSyst`
//! names, and the system is set up with them there and nowhere else; a
//! stock mission of each ship goal has its goal met by the storyline
//! walker's `meet goal`; and a stock mission's escorts come in on their
//! timer and join the player's formation. Every stock mission's
//! auxiliary ships name a `düde` in range and an `AuxShipSyst` the engine
//! reads, and a stock mission's come in on their countdown as ordinary
//! traffic. Skips, passing, when `NOVA_DATA` is unset.

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

/// The "Avoid" missions' shape: no ships of their own to fight
/// (`ShipGoal` 0, `ShipBehav` 0), hunters following the player
/// (`ShipSyst` -6), `Flags` 0x1445 with 0x0001.
fn avoid(data: &GameData, id: i16, start: i16) -> nova_sim::catalog::MissionRecord {
    let record = data
        .missions()
        .into_iter()
        .find(|record| record.id == MissionId(id))
        .unwrap_or_else(|| panic!("mïsn {id}"));
    assert_eq!(
        (
            record.ships.goal,
            record.ships.behav,
            record.ships.syst,
            record.ships.start,
            record.flags
        ),
        (0, 0, -6, start, 0x1445),
        "mïsn {id}"
    );
    record
}

/// A new pilot docked with `mission` started by `S`, its slot.
fn started_docked(data: &GameData, mission: i16) -> (Session, u8) {
    let mut session = docked_new_pilot(data);
    session.run_set(
        &SetExpr::parse(&format!("S{mission}")).expect("parses"),
        &mut NeverFires,
    );
    let _ = session.settle_script(data, &mut NeverFires);
    let slot = *session
        .pilot()
        .missions()
        .holding(MissionId(mission))
        .first()
        .expect("started");
    (session, slot)
}

/// The NPCs of the system that fly one of `düde`'s ships.
fn of_dude(data: &GameData, session: &Session, dude: DudeId) -> usize {
    let ships: Vec<ShipId> = data
        .dude(dude)
        .expect("its düde")
        .ships
        .iter()
        .map(|&(ship, _)| ship)
        .collect();
    session
        .npcs()
        .iter()
        .filter(|npc| ships.contains(&npc.ship))
        .count()
}

/// `mïsn` 618, an "Avoid" mission whose hunters are set up with the
/// system (`ShipStart` 0).
const AVOID_SET_UP: i16 = 618;
/// `mïsn` 614, an "Avoid" mission whose hunters come in later
/// (`ShipStart` 1).
const AVOID_LATER: i16 = 614;

#[test]
fn a_stock_avoid_mission_set_up_with_the_system_spawns_its_hunters_then_auto_aborts() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let record = avoid(&data, AVOID_SET_UP, 0);
    let dude = record.ships.dude.expect("a düde");
    let count = usize::try_from(record.ships.count).expect("a count");
    let (mut session, slot) = started_docked(&data, AVOID_SET_UP);
    session.take_off(&data, &mut NeverFires).expect("takes off");
    assert!(
        session.pilot().missions().get(slot).is_none(),
        "auto-aborted at the take-off's setup"
    );
    let _ = session.settle_script(&data, &mut NeverFires);
    assert!(of_dude(&data, &session, dude) >= count, "its hunters came");
    session.tick_traffic(&data, &NovaAi::default(), &mut NeverFires);
    assert!(session.npcs().iter().all(|npc| npc.mission.is_none()));
    assert!(
        of_dude(&data, &session, dude) >= count,
        "and stay as traffic"
    );
}

#[test]
fn a_stock_avoid_mission_whose_hunters_come_later_auto_aborts_as_they_come_in() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let record = avoid(&data, AVOID_LATER, 1);
    let dude = record.ships.dude.expect("a düde");
    let count = usize::try_from(record.ships.count).expect("a count");
    let (mut session, slot) = started_docked(&data, AVOID_LATER);
    session.take_off(&data, &mut NeverFires).expect("takes off");
    let _ = session.settle_script(&data, &mut NeverFires);
    let ai = NovaAi::default();
    let mut before = 0;
    while let Some(active) = session.pilot().missions().get(slot) {
        assert!(active.ships.appear > 0, "the timer runs until they come");
        before = session.npcs().len();
        session.tick_traffic(&data, &ai, &mut NeverFires);
        let _ = session.settle_script(&data, &mut NeverFires);
    }
    let jumping_in = session
        .npcs()
        .iter()
        .filter(|npc| matches!(npc.mode, nova_sim::traffic::npc::Mode::JumpingIn { .. }))
        .count();
    assert!(session.npcs().len() >= before, "none taken out");
    assert!(jumping_in >= count, "its hunters came in, then it aborted");
    assert!(of_dude(&data, &session, dude) >= count);
}

#[test]
fn every_stock_missions_aux_ships_are_of_a_built_aux_ship_syst_and_a_dude_in_range() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let missions = data.missions();
    let aux: Vec<_> = missions
        .iter()
        .filter(|record| record.aux.count > 0)
        .collect();
    assert_eq!(aux.len(), 136, "stock missions with aux ships");
    for record in aux {
        assert!(
            matches!(record.aux.syst, -1 | -2 | -3 | 128..=2175),
            "{:?}: AuxShipSyst {}",
            record.id,
            record.aux.syst
        );
        let dude = nova_sim::mission::aux::aux_dude(&record.aux).expect("a düde in range");
        assert!(data.dude(dude).is_some(), "{:?}: {dude:?}", record.id);
    }
}

/// The auxiliary ships in `session`'s system.
fn aux(session: &Session) -> Vec<&nova_sim::traffic::npc::Npc> {
    session
        .npcs()
        .iter()
        .filter(|npc| npc.aux.is_some())
        .collect()
}

#[test]
fn a_stock_missions_aux_ships_come_in_on_their_countdown_as_ordinary_traffic() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let record = data
        .missions()
        .into_iter()
        .find(|record| {
            record.aux.count > 0
                && record.aux.syst == -1
                && record.flags & 0x0001 == 0
                && record.ships.count <= 0
        })
        .expect("a stock mission of aux ships anywhere and no special ship");
    let mut session = docked_new_pilot(&data);
    session.run_set(
        &SetExpr::parse(&format!("S{}", record.id.0)).expect("parses"),
        &mut NeverFires,
    );
    let _ = session.settle_script(&data, &mut NeverFires);
    let mut flying = in_flight(&data, session.pilot(), session.pilot().system());
    assert!(aux(&flying).is_empty(), "none at setup");
    for _ in 0..140 {
        flying.tick_traffic(&data, &NovaAi::default(), &mut NeverFires);
    }
    let came = aux(&flying);
    assert_eq!(
        came.len(),
        usize::try_from(record.aux.count).expect("a count"),
        "{:?}",
        record.id
    );
    assert!(came.iter().all(|npc| npc.mission.is_none()), "unmarked");
}

/// The cloak gear stock ship `id` carries: its `Flags2` and its default
/// items' cloak.
fn cloak_gear(data: &GameData, id: i16) -> nova_sim::cloak::CloakGear {
    let ship = data
        .ships()
        .into_iter()
        .find(|ship| ship.id == ShipId(id))
        .expect("the ship");
    let items = ship.defaults.iter().copied().collect();
    nova_sim::cloak::CloakGear::of(ship.fields.flags2, &items, &data.outfits())
}

#[test]
fn the_zephyrs_of_603_and_604_carry_a_cloak_and_no_stock_outfit_scans_for_one() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    // Dudes 200 and 201 fly ships 375 and 376, the Zephyr, with the
    // Cloaking Organ (outfit 268, `ModVal` 0x080b): it decloaks when hit
    // and shows on the radar; `Flags2` 0x5282 cloaks it retreating,
    // pressing an attack, and hit by a ship it is not attacking, fading
    // at the slow rate.
    for id in [375, 376] {
        let gear = cloak_gear(&data, id);
        assert_eq!(
            gear,
            nova_sim::cloak::CloakGear {
                device: true,
                decloaks_when_hit: true,
                on_radar: true,
                flags2: 0x5282,
            },
            "{id}"
        );
        assert!(!gear.fast(), "{id}");
    }
    assert!(!cloak_gear(&data, 128).device, "the shuttle has none");
    let scanners: Vec<i16> = data
        .outfits()
        .iter()
        .filter(|outfit| {
            outfit
                .mods
                .iter()
                .any(|&(kind, _)| kind == nova_sim::cloak::MOD_SCANNER)
        })
        .map(|outfit| outfit.id.0)
        .collect();
    assert!(
        scanners.is_empty(),
        "the stock player never sees a cloaked ship: {scanners:?}"
    );
}
