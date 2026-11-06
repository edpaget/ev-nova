//! The session's wiring of where a stationary warship is placed:
//! `_RandomShipSpawn` reads slot 0 of the system's `NavDefs`
//! (@0x3c274-0x3c27d), shown or not, and falls to the box when the slot is
//! empty (@0x3c32b). The session hands the traffic that slot from
//! `Statics::nav_def_slot`; the rule itself is pinned by the traffic's
//! own tests (`nova_sim_ai::traffic`'s `nav_def_zero_tests`).

use super::*;
use crate::catalog::{LandingSite, NAV_DEF_SLOTS, SystemId};
use crate::handling::ShipFields;
use crate::testkit::{FAST, FakePilotCatalog, Ranged};
use crate::traffic::spawn::STATIONARY_DISTANCE;

/// One ship on average in 130: ship 129 of no speed, flown as a warship,
/// with system 130's `NavDefs` slots `slots`.
fn still_warship_in_130(slots: [Option<StellarId>; NAV_DEF_SLOTS]) -> FakePilotCatalog {
    let mut catalog = super::tests::trafficked(130, 1, 3);
    catalog.ship_records = vec![crate::testkit::ship(129, ShipFields { speed: 0, ..FAST })];
    catalog.nav_defs = vec![(SystemId(130), slots)];
    catalog
}

/// The one ship a populated `session` holds, each angle drawn 90 and each
/// box draw 1499.
fn populated(catalog: &FakePilotCatalog) -> Session {
    let mut session = Session::start(catalog).expect("starts");
    session.populate(catalog, &mut Ranged::of(&[(360, 90)]));
    assert_eq!(session.npcs().len(), 1, "{:?}", session.npcs());
    session
}

#[test]
fn a_stationary_warship_is_placed_by_slot_0_though_another_visible_system_owns_it() {
    // 130 lists 127 first, but 129, visible and lower, owns it, so 130
    // doesn't show it.
    let unshown = crate::testkit::planet(127, -500.0, 300.0);
    let mut slots = [None; NAV_DEF_SLOTS];
    slots[..3].copy_from_slice(&[127, 128, 129].map(|id| Some(StellarId(id))));
    let mut catalog = still_warship_in_130(slots);
    let shown: Vec<LandingSite> = catalog.sites[0].1.clone();
    catalog.sites = vec![
        (SystemId(129), vec![unshown]),
        (SystemId(130), [vec![unshown], shown].concat()),
        catalog.sites[1].clone(),
    ];
    catalog.systems.push(SystemId(129));
    catalog
        .star_map
        .push(crate::testkit::star(129, (-600.0, 600.0), &[]));
    let session = populated(&catalog);
    let ids = |sites: &[LandingSite]| sites.iter().map(|site| site.id.0).collect::<Vec<_>>();
    assert_eq!(ids(&session.chart.sites), [128, 129], "127 not shown");
    let state = session.npcs()[0].state;
    let off = (state.position - unshown.position).length();
    assert!((off - STATIONARY_DISTANCE).abs() < 1e-3, "{state:?}");
}

#[test]
fn with_slot_0_empty_a_stationary_warship_is_placed_in_the_box_not_by_slot_1() {
    let mut slots = [None; NAV_DEF_SLOTS];
    slots[1] = Some(StellarId(128));
    slots[2] = Some(StellarId(129));
    let session = populated(&still_warship_in_130(slots));
    let state = session.npcs()[0].state;
    let by_128 = state.position - Vec2::new(30.0, -40.0);
    assert!(
        (by_128.length() - STATIONARY_DISTANCE).abs() > 1.0,
        "not by slot 1's 128: {state:?}"
    );
}
