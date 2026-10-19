//! Ionization over the stock data: `oütf` 262 "Ion Dissipator" raises the
//! player's capacity by 200 and its rate by 0.5 a tick, every stock
//! `shïp` sheds its `Deionize` / 100 a tick and fills at its `IonizeMax`,
//! an Ion Cannon's charge slows the stock first ship's thrust, and the
//! Solar Lance (164) and Polaron Massive Torp. (199), of `IonizeColor` 0,
//! dose the default blue. Skips, passing, when `NOVA_DATA` is unset.

#![allow(clippy::float_cmp)]

use crate::common;

use nova_data::GameData;
use nova_sim::combat::ion::{Dose, Ion};
use nova_sim::combat::weapon::{DEFAULT_IONIZE_COLOUR, WeaponSpec};
use nova_sim::flight::Flyer;
use nova_sim::stats::{ION_ABSORBER, ION_DISSIPATER, ShipStats};
use nova_sim::{CombatCatalog, Controls, OutfitId, Pilot, PilotCatalog, Session, Vec2};

const ION_DISSIPATOR: OutfitId = OutfitId(262);

fn data() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// The stock first `chär`'s session in space, carrying `outfits`.
fn flying(data: &GameData, outfits: serde_json::Value) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::Value::Null;
    save["outfits"] = outfits;
    let flown = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let session = Session::fly(data, flown).expect("flies");
    assert_eq!(session.landed(), None);
    session
}

fn weapon(data: &GameData, id: i16) -> WeaponSpec {
    data.weapons()
        .iter()
        .map(WeaponSpec::new)
        .find(|spec| spec.id.0 == id)
        .expect("a stock weapon")
}

#[test]
fn the_stock_ion_dissipator_adds_200_capacity_and_half_a_tick_for_the_player() {
    let Some(data) = data() else {
        return;
    };
    let outfits = data.outfits();
    let record = outfits
        .iter()
        .find(|record| record.id == ION_DISSIPATOR)
        .expect("oütf 262");
    assert_eq!(record.name, "Ion Dissipator");
    assert_eq!(
        record.mods[..2],
        [(ION_ABSORBER, 200), (ION_DISSIPATER, 50)]
    );
    let plain = flying(&data, serde_json::json!([]));
    let carrying = flying(
        &data,
        serde_json::json!([{"outfit": ION_DISSIPATOR.0, "count": 1}]),
    );
    assert_eq!(carrying.ion_capacity(), plain.ion_capacity() + 200.0);
    let rate = carrying.ion_rate() - plain.ion_rate();
    assert!((rate - 0.5).abs() < 1e-6, "{rate}");
    // No stock ship carries one, so no NPC's capacity or rate counts it.
    assert!(
        data.ships()
            .iter()
            .all(|ship| ship.defaults.iter().all(|&(id, _)| id != ION_DISSIPATOR))
    );
}

#[test]
fn every_stock_ship_sheds_its_deionize_a_hundredth_and_fills_at_its_ionize_max() {
    let Some(data) = data() else {
        return;
    };
    let ships = data.ships();
    assert_eq!(ships.len(), 288);
    for ship in &ships {
        let fields = ship.fields;
        assert!(
            fields.ionize_max > 0 && fields.deionize > 0,
            "{}",
            ship.id.0
        );
        let stats = ShipStats::new(fields, &[]);
        let rate = (f64::from(fields.deionize) * 0.01) as f32;
        assert_eq!(stats.ion.rate(Flyer::Npc), rate, "{}", ship.id.0);
        assert_eq!(
            stats.ion.capacity(Flyer::Npc),
            f32::from(fields.ionize_max),
            "{}",
            ship.id.0
        );
    }
}

#[test]
fn an_ion_cannons_charge_slows_the_stock_first_ships_thrust_and_sheds() {
    let Some(data) = data() else {
        return;
    };
    let cannon = weapon(&data, 142);
    let thrust = Controls {
        thrust: true,
        ..Controls::default()
    };
    let speed_after = |charge: f32| {
        let mut session = flying(&data, serde_json::json!([]));
        session.tick(Controls::default());
        nova_sim::fixture::set_player_ion(
            &mut session,
            Ion {
                charge,
                colour: cannon.ionize_colour,
                ..Ion::default()
            },
        );
        session.tick(thrust);
        (
            session.player().velocity,
            session.ion().charge,
            session.ion_rate(),
        )
    };
    let (plain, ..) = speed_after(0.0);
    assert!(plain.length() > 0.0);
    // Ten hits.
    let charge = 10.0 * cannon.ionization as f32;
    let (slowed, left, rate) = speed_after(charge);
    assert!(
        slowed.length() < plain.length() * 0.99,
        "{slowed:?} {plain:?}"
    );
    assert_eq!(left, charge - rate);
    assert_ne!(slowed, Vec2::ZERO);
}

#[test]
fn the_stock_weapons_of_ionize_color_0_dose_the_default_blue() {
    let Some(data) = data() else {
        return;
    };
    for id in [164, 199] {
        let spec = weapon(&data, id);
        assert!(spec.ionization > 0, "{id}");
        assert_eq!(spec.ionize_colour, DEFAULT_IONIZE_COLOUR, "{id}");
        let mut ion = Ion::default();
        ion.take(Dose::of(&spec, None), Vec2::ZERO);
        assert_eq!(ion.colour, 0x0002_02ff, "{id}");
    }
}
