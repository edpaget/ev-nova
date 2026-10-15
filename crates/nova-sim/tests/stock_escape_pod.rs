//! The escape pod over the stock data: `shïp` 895 is the Escape Pod the
//! eject makes the player, `oütf` 188 (Escape Pod) is the eject's
//! `ModType` 11 and 187 (Auto-eject) its `ModType` 20, the Fed Viper's bay
//! offers an escape ship, and `dësc` 13999 is the rescue's text. Skips,
//! passing, when `NOVA_DATA` is unset.

mod common;

use std::collections::BTreeMap;

use nova_data::GameData;
use nova_data::records::desc::Desc;
use nova_sim::combat::armament::Arsenal;
use nova_sim::escape::{escape_ship_type, has_auto_eject, has_escape_pod};
use nova_sim::{ESCAPE_POD_SHIP, OutfitId, PilotCatalog, ShipId};

fn data() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

#[test]
fn ship_895_is_the_escape_pod() {
    let Some(data) = data() else {
        return;
    };
    let ships = data.ships();
    let pod = ships
        .iter()
        .find(|record| record.id == ESCAPE_POD_SHIP)
        .expect("shïp 895");
    assert_eq!(pod.name, "Escape Pod");
}

#[test]
fn outfits_188_and_187_are_the_escape_pod_and_auto_eject() {
    let Some(data) = data() else {
        return;
    };
    let outfits = data.outfits();
    let owning = |id: i16| BTreeMap::from([(OutfitId(id), 1)]);
    assert!(has_escape_pod(&owning(188), &outfits));
    assert!(!has_auto_eject(&owning(188), &outfits));
    assert!(has_auto_eject(&owning(187), &outfits));
    assert!(!has_escape_pod(&owning(187), &outfits));
}

#[test]
fn the_fed_vipers_bay_offers_an_escape_ship() {
    let Some(data) = data() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let viper = ShipId(144);
    let bay = arsenal
        .weapons()
        .find(|weapon| weapon.carried == Some(viper))
        .expect("a Viper bay")
        .id;
    let ships = data.ships();
    let aboard = |weapon| u32::from(weapon == bay);
    assert_eq!(
        escape_ship_type(arsenal.weapons(), aboard, &ships),
        Some(viper)
    );
    assert_eq!(escape_ship_type(arsenal.weapons(), |_| 0, &ships), None);
}

#[test]
fn desc_13999_is_the_rescues_text() {
    let Some(data) = data() else {
        return;
    };
    let desc = data
        .get::<Desc>(nova_sim::escape::RESCUE_DESC)
        .expect("dësc 13999")
        .expect("reads");
    assert!(
        desc.record
            .text
            .as_str()
            .starts_with("After several weeks of sensory deprivation"),
        "{:?}",
        desc.record.text.as_str()
    );
}
