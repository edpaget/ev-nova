//! The stellars' defence fleets over the stock data (play-feel-4 phase 1,
//! `_SpawnStellarDefenseShip` @0x3d2c9, `_EnterMoreShips` @0x43c45): the
//! four `Flags2` 0x0200 stellars' `DefenseDude`s and counts as read, Earth
//! under attack sending up a defender a frame to its wave of 6, and its
//! gun firing once one is up. Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::pilot::Pilot;
use nova_sim::traffic::stellar_defence::{initial_left, wave};
use nova_sim::{CombatEvent, DudeId, PilotCatalog, Session, StellarId, SystemId};

/// The system that lists `stellar`.
fn system_of(data: &GameData, stellar: StellarId) -> SystemId {
    data.star_map()
        .into_iter()
        .find(|system| {
            data.landing_sites(system.id)
                .iter()
                .any(|site| site.id == stellar)
        })
        .expect("a system lists it")
        .id
}

/// A new stock pilot flying in `system` over `stellar`, its record there
/// `record`, at rest on the stellar's centre as a load puts it.
fn over(data: &GameData, system: SystemId, stellar: StellarId, record: i16) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(stellar.0);
    save["system"] = serde_json::json!(system.0);
    save["system_legal"] = serde_json::json!([{"system": system.0, "record": record}]);
    let there = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    Session::fly(data, there).expect("flies")
}

/// The ships of `session` defending `stellar`.
fn up(session: &Session, stellar: StellarId) -> usize {
    session
        .npcs()
        .iter()
        .filter(|npc| npc.defends == Some(stellar))
        .count()
}

/// One pass of `session`, and whether `stellar` fired in it.
fn pass(session: &mut Session, data: &GameData, stellar: StellarId) -> bool {
    crate::passes::work(session, data, nova_sim::Controls::default());
    session.take_combat_events().into_iter().any(
        |event| matches!(event, CombatEvent::StellarFired { stellar: fired, .. } if fired == stellar),
    )
}

#[test]
fn the_four_guarded_stellars_read_their_defence_dudes_and_counts() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let read: Vec<_> = [128, 137, 161, 508]
        .into_iter()
        .map(|id| {
            let stellar = StellarId(id);
            let site = data
                .landing_sites(system_of(&data, stellar))
                .into_iter()
                .find(|site| site.id == stellar)
                .expect("listed");
            (
                id,
                site.defense_dude,
                site.def_count,
                initial_left(site.def_count),
                wave(site.def_count),
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            (128, Some(DudeId(130)), 7006, 600, 6),
            (137, Some(DudeId(130)), 2206, 120, 6),
            (161, Some(DudeId(150)), 2206, 120, 6),
            (508, Some(DudeId(151)), 2206, 120, 6),
        ],
        "Earth, Port Kane, Pyrogenesis I and New Ireland"
    );
}

#[test]
fn earth_under_attack_sends_up_a_defender_a_frame_to_its_wave_and_then_fires() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let earth = StellarId(128);
    let sol = system_of(&data, earth);
    let mut session = over(&data, sol, earth, -1000);
    // The first pass fills the system.
    assert!(!pass(&mut session, &data, earth), "no defender, no fire");
    assert_eq!(up(&session, earth), 0);
    assert_eq!(
        nova_sim::fixture::defence_ships_left(&session, earth),
        Some(600)
    );
    nova_sim::fixture::alarm_stellar(&mut session, earth);
    let mut fired = false;
    for frame in 1..=6 {
        fired |= pass(&mut session, &data, earth);
        assert_eq!(up(&session, earth), frame, "one a frame");
        assert!(
            session
                .npcs()
                .iter()
                .filter(|npc| npc.defends == Some(earth))
                .all(|npc| npc.ship.0 >= 128),
            "düde 130's ships"
        );
    }
    pass(&mut session, &data, earth);
    assert_eq!(up(&session, earth), 6, "its wave of 7006 % 10");
    assert_eq!(
        nova_sim::fixture::defence_ships_left(&session, earth),
        Some(594)
    );
    assert!(
        fired,
        "Earth's gun fires at a criminal once a defender is up"
    );
}
