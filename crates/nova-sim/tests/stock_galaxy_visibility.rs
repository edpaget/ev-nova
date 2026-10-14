//! The galaxy refresh over the stock data: every `sÿst` `Visibility`
//! parses, a pair of alternate systems swaps as the bit that selects them
//! is set, and Aldebaran's two versions, both visible between Polaris
//! missions 183 and 887, resolve to 141 by default (`colocated_resolution`
//! engine). Skips, passing, when `NOVA_DATA` is unset.

mod common;

use std::collections::BTreeSet;

use nova_data::GameData;
use nova_sim::control::{Bit, Gate, NovaBits, PilotFacts};
use nova_sim::pilot::Gender;
use nova_sim::story::{GalaxyOverlay, HiddenMove, OverlaidGalaxy};
use nova_sim::{CoLocated, HyperlinkRule, OutfitId, PilotCatalog, StellarId, SystemId};

/// A pilot with only these bits set, as a control-bit test reads it.
#[derive(Debug)]
struct Bits(BTreeSet<u16>);

impl PilotFacts for Bits {
    fn bit(&self, bit: Bit) -> bool {
        self.0.contains(&bit.get())
    }

    fn gender(&self) -> Gender {
        Gender::Male
    }

    fn paid(&self, _days: u16) -> bool {
        true
    }

    fn has_outfit(&self, _outfit: OutfitId) -> bool {
        false
    }

    fn explored(&self, _system: SystemId) -> bool {
        false
    }
}

/// The stock galaxy refreshed for a pilot with `bits` set, a place two
/// visible systems share resolved as `colocated` says.
fn refreshed(data: &GameData, bits: &[u16], colocated: CoLocated) -> OverlaidGalaxy {
    let mut galaxy = OverlaidGalaxy::new(data.star_map(), GalaxyOverlay::default());
    galaxy.set_colocated(colocated);
    let pilot = Bits(bits.iter().copied().collect());
    let gate = Gate {
        control_bits: &NovaBits,
        pilot: &pilot,
    };
    let mut overlay = GalaxyOverlay::default();
    overlay.refresh_visibility(
        galaxy.catalog_systems().collect::<Vec<_>>(),
        &data.system_visibility(),
        &gate,
    );
    galaxy.set_overlay(overlay);
    galaxy
}

/// The owner `galaxy` gives stellar `id`, from the systems listing it.
fn owner(data: &GameData, galaxy: &OverlaidGalaxy, id: i16) -> Option<i16> {
    let listed = data.listed_stellars();
    let stellar = listed
        .iter()
        .find(|stellar| stellar.id == StellarId(id))
        .unwrap_or_else(|| panic!("spöb {id} is listed"));
    galaxy.owner(&stellar.systems).map(|system| system.0)
}

fn open() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// Polaris mission 183 sets b305; mission 887 sets b148.
const B305: u16 = 305;
const B148: u16 = 148;

/// Stellars 130, 169 and 170 are listed by both versions of Aldebaran;
/// 430 and 433 by 532 alone.
const SHARED: [i16; 3] = [130, 169, 170];

#[test]
fn every_stock_visibility_parses() {
    let Some(data) = open() else {
        return;
    };
    let tests = data.system_visibility();
    assert_eq!(tests.len(), data.star_map().len(), "one for each system");
    let broken: Vec<i16> = tests
        .iter()
        .filter(|test| test.visibility.tree().is_none())
        .map(|test| test.id.0)
        .collect();
    assert_eq!(broken, [] as [i16; 0]);
}

#[test]
fn aldebaran_swaps_from_141_to_532_once_mission_887_sets_b148() {
    let Some(data) = open() else {
        return;
    };
    let fresh = refreshed(&data, &[], CoLocated::Lowest);
    assert!(fresh.visible(SystemId(141)));
    assert!(!fresh.visible(SystemId(532)));
    for stellar in SHARED.into_iter().chain([148, 171]) {
        assert_eq!(owner(&data, &fresh, stellar), Some(141), "spöb {stellar}");
    }
    for stellar in [430, 433] {
        assert_eq!(owner(&data, &fresh, stellar), None, "spöb {stellar}");
    }
    let swapped = refreshed(&data, &[B305, B148], CoLocated::Lowest);
    assert!(!swapped.visible(SystemId(141)));
    assert!(swapped.visible(SystemId(532)));
    for stellar in SHARED.into_iter().chain([430, 433]) {
        assert_eq!(owner(&data, &swapped, stellar), Some(532), "spöb {stellar}");
    }
    for stellar in [148, 171] {
        assert_eq!(owner(&data, &swapped, stellar), None, "141's own");
    }
    assert_eq!(swapped.same_place(SystemId(141)), Some(SystemId(532)));
}

#[test]
fn with_both_versions_of_aldebaran_visible_141_stands_for_the_place_by_default() {
    let Some(data) = open() else {
        return;
    };
    let both = refreshed(&data, &[B305], CoLocated::Lowest);
    assert!(both.visible(SystemId(141)) && both.visible(SystemId(532)));
    assert_eq!(both.same_place(SystemId(532)), Some(SystemId(141)));
    assert_eq!(
        both.arrival(SystemId(532)),
        HiddenMove {
            to: SystemId(141),
            make_visible: false,
        }
    );
    assert_eq!(both.hidden_move(SystemId(532)), None, "visible: no move");
    assert_eq!(
        both.map().jumps(SystemId(173), HyperlinkRule::Engine),
        [SystemId(141)],
        "173 lists 532 and 141"
    );
    for stellar in SHARED {
        assert_eq!(owner(&data, &both, stellar), Some(141), "spöb {stellar}");
    }
    for stellar in [430, 433] {
        assert_eq!(owner(&data, &both, stellar), None, "532 is shadowed");
    }
}

#[test]
fn by_the_other_reading_both_versions_of_aldebaran_stand_for_themselves() {
    let Some(data) = open() else {
        return;
    };
    let both = refreshed(&data, &[B305], CoLocated::Itself);
    assert_eq!(both.same_place(SystemId(532)), Some(SystemId(532)));
    assert_eq!(both.arrival(SystemId(532)).to, SystemId(532));
    assert_eq!(
        both.map().jumps(SystemId(173), HyperlinkRule::Engine),
        [SystemId(141), SystemId(532)]
    );
    for stellar in SHARED {
        assert_eq!(owner(&data, &both, stellar), Some(141), "the lowest");
    }
    for stellar in [430, 433] {
        assert_eq!(owner(&data, &both, stellar), Some(532), "spöb {stellar}");
    }
}
