//! Gravity and deadly stellars over the stock data: no stock `spöb` has
//! `Gravity` other than none or `Flags2` 0x0100, so a stock system has no
//! gravity source and no deadly stellar, and the stock first chär's
//! session has no gravity source. Nor does stock have a stellar-immune
//! hull (`shïp` `Flags3` 0x0020) or a resistance outfit (`oütf` `ModType`
//! 41 or 42): both are plug-ins' alone. Skips, passing, when `NOVA_DATA`
//! is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::deadly_stellar::{DEADLY_RESISTANCE, is_deadly};
use nova_sim::gravity::{GRAVITY_RESISTANCE, STELLAR_IMMUNE_HULL};
use nova_sim::{CombatCatalog, OutfitId, PilotCatalog, Session, ShipId, StellarId};

#[test]
fn no_stock_stellar_has_gravity_or_is_deadly() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let systems = data.star_map();
    assert!(systems.len() > 100, "{}", systems.len());
    let mut stellars = 0;
    let mut odd: Vec<StellarId> = Vec::new();
    for system in &systems {
        for site in data.landing_sites(system.id) {
            stellars += 1;
            if site.gravity != 0 || is_deadly(site.flags2) {
                odd.push(site.id);
            }
        }
    }
    assert!(stellars > 100, "{stellars}");
    assert_eq!(odd, [], "gravity or deadly");
}

#[test]
fn the_stock_first_chars_session_has_no_gravity_source() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let session = Session::start(&data).expect("the stock first chär starts");
    assert!(!session.gravity_sources_present());
}

#[test]
fn the_stock_stellar_immune_hulls_and_resistance_outfits_are_as_pinned() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let immune: Vec<ShipId> = data
        .hulls()
        .into_iter()
        .filter(|hull| hull.flags3 & STELLAR_IMMUNE_HULL != 0)
        .map(|hull| hull.id)
        .collect();
    assert_eq!(immune, [], "Flags3 0x0020");
    let carrying = |mod_type: i16| -> Vec<OutfitId> {
        data.outfits()
            .into_iter()
            .filter(|outfit| outfit.mods.iter().any(|&(kind, _)| kind == mod_type))
            .map(|outfit| outfit.id)
            .collect()
    };
    assert_eq!(carrying(GRAVITY_RESISTANCE), [], "ModType 41");
    assert_eq!(carrying(DEADLY_RESISTANCE), [], "ModType 42");
}
