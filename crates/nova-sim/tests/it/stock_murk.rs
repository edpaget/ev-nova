//! The player's effective murk over the stock data (rdm weapons-fidelity
//! phase 24): a system's `Murk` above 7 murks as it says, and one of 1 to
//! 7 not at all by the engine's reading of `low_murk`, as it says by the
//! Bible's. Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::pilot::Pilot;
use nova_sim::{PilotCatalog, RuleKey, RuleSource, Rulebook, Session, SystemId};

/// A new stock pilot flying in `system`, at its first stellar.
fn flying_in(data: &GameData, system: i16) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let stellar = data
        .landing_sites(SystemId(system))
        .first()
        .expect("a stellar")
        .id;
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(stellar.0);
    save["system"] = serde_json::json!(system);
    let there = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    Session::fly(data, there).expect("flies")
}

/// Stock system `id`'s raw `Murk`.
fn murk_of(data: &GameData, id: i16) -> i16 {
    data.star_map()
        .iter()
        .find(|system| system.id == SystemId(id))
        .expect("on the map")
        .murk
}

#[test]
fn a_stock_system_of_murk_20_murks_at_20() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    assert_eq!(murk_of(&data, 131), 20);
    let session = flying_in(&data, 131);
    assert_eq!(session.pilot().system(), SystemId(131));
    assert_eq!(session.effective_murk(), 20);
}

#[test]
fn the_stock_murky_systems_murk_toward_their_background_colours() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    for (system, murk, colour) in [(131, 20, 0x1081), (155, 50, 0x1081), (271, 50, 0x1042)] {
        assert_eq!(murk_of(&data, system), murk, "system {system}");
        assert_eq!(
            flying_in(&data, system).murk_colour(),
            colour,
            "system {system}"
        );
    }
}

#[test]
fn a_stock_system_of_murk_5_murks_not_at_all_by_the_engine_and_at_5_by_the_bible() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    assert_eq!(murk_of(&data, 133), 5);
    assert_eq!(flying_in(&data, 133).effective_murk(), 0);
    let bible = Rulebook::default().with_override(RuleKey::LowMurk, RuleSource::Bible);
    assert_eq!(flying_in(&data, 133).with_rules(bible).effective_murk(), 5);
}
