//! The afterburner over the stock data: `oütf` 197 "Afterburner" and 338
//! "Telekinetic Boost" each carry `ModType` 15, and the stock first
//! `chär`'s ship carrying the Afterburner, holding thrust and the
//! afterburner, reaches 1.8 times its top speed on an axis and burns its
//! `ModVal` x 0.0333 fuel a tick, as `_HandlePlayer` flies it. Skips,
//! passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::afterburner::afterburner_top;
use nova_sim::stats::AFTERBURNER;
use nova_sim::{Controls, OutfitId, Pilot, PilotCatalog, Session};

const AFTERBURNER_OUTFIT: OutfitId = OutfitId(197);
const TELEKINETIC_BOOST: OutfitId = OutfitId(338);

#[test]
fn the_stock_afterburner_and_telekinetic_boost_carry_mod_type_15() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let outfits = data.outfits();
    let mods = |id: OutfitId| {
        let record = outfits
            .iter()
            .find(|record| record.id == id)
            .expect("present");
        let burners: Vec<i16> = record
            .mods
            .iter()
            .filter(|&&(mod_type, _)| mod_type == AFTERBURNER)
            .map(|&(_, mod_val)| mod_val)
            .collect();
        (record.name.clone(), burners)
    };
    assert_eq!(
        mods(AFTERBURNER_OUTFIT),
        ("Afterburner".to_owned(), vec![37])
    );
    assert_eq!(
        mods(TELEKINETIC_BOOST),
        ("Telekinetic Boost".to_owned(), vec![20])
    );
    let carrying: Vec<OutfitId> = outfits
        .iter()
        .filter(|record| {
            record
                .mods
                .iter()
                .any(|&(mod_type, _)| mod_type == AFTERBURNER)
        })
        .map(|record| record.id)
        .collect();
    assert_eq!(carrying, [AFTERBURNER_OUTFIT, TELEKINETIC_BOOST]);
}

#[test]
fn the_stock_first_ship_with_an_afterburner_reaches_1_8_times_its_top_speed() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::Value::Null;
    save["outfits"] = serde_json::json!([{"outfit": AFTERBURNER_OUTFIT.0, "count": 1}]);
    let flown = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(&data, flown).expect("flies");
    assert_eq!(session.landed(), None);
    let top = session.top_speed();
    let usage = (f64::from(37_i16) * 0.0333) as f32;

    // Up the screen, as it starts. The first tick's box of none holds the
    // ship at rest.
    assert!(session.player().heading.abs() < f32::EPSILON);
    session.tick(Controls::default());
    let burn = Controls {
        thrust: true,
        afterburner: true,
        ..Controls::default()
    };
    let fuel = session.reserves().fuel.now;
    assert!(fuel > 160.0 * usage, "{fuel}");
    session.tick(burn);
    assert!(session.afterburner_on());
    let spent = fuel - session.reserves().fuel.now;
    assert!((spent - usage).abs() < 1e-4, "{spent} of {usage}");
    for _ in 0..150 {
        session.tick(burn);
    }
    assert!(session.afterburner_on(), "fuel left");
    let at = session.player().position;
    session.tick(burn);
    let step = session.player().position - at;
    let burnt = afterburner_top(top);
    assert!((step.y + burnt).abs() < 1e-2, "{step:?} at {burnt}");
    assert!(step.x.abs() < 1e-3, "{step:?}");
    assert!(burnt > top);
}
