//! The ships' display over the stock data (rdm ship-handling-fidelity
//! phase 10): shän 379, the Asteroid Miner, folds to fire, of `Flags`
//! 0xc2 and six sets; shän 380, the Thunderforge, animates its alt layer
//! of six sets over a single base set; the Shuttle banks. Skips, passing,
//! when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::combat::hull::HullSpec;
use nova_sim::combat::ship_frames::{BANKS, FOLDS, UNFOLDS_TO_FIRE};
use nova_sim::{CombatCatalog, ShipId};

fn hull(data: &GameData, id: i16) -> HullSpec {
    let record = data
        .hulls()
        .into_iter()
        .find(|hull| hull.id == ShipId(id))
        .expect("a hull");
    HullSpec::new(&record)
}

#[test]
fn the_asteroid_miner_folds_to_fire_through_six_sets() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let miner = hull(&data, 379).frames;
    assert_eq!(miner.flags, 0x00C2);
    assert!(miner.has(FOLDS) && miner.has(UNFOLDS_TO_FIRE));
    assert_eq!((miner.base_sets, miner.anim_delay), (6, 5));
    assert_eq!(miner.unfolds_to_fire(), Some(6));
    let argosy = hull(&data, 138).frames;
    assert_eq!(argosy.unfolds_to_fire(), None, "folds, not to fire");
    assert!(argosy.has(FOLDS));
}

#[test]
fn the_thunderforge_animates_its_alt_layer_over_one_base_set() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let forge = hull(&data, 380).frames;
    assert_eq!(forge.flags, 0x0058);
    assert_eq!((forge.base_sets, forge.alt_sets), (1, 6));
    assert!(!forge.multi_set(), "its set block skipped: no double step");
    let shuttle = hull(&data, 128).frames;
    assert!(shuttle.has(BANKS));
    assert_eq!(shuttle.base_sets, 3);
    assert_eq!(shuttle.alt_sets, 0);
}
