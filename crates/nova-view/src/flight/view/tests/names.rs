//! The flight's names and date over a real session
//! (rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks): this phase's
//! real check that each of the adapter's [`NameSource`] answers is the
//! session's own. What the view words with them runs on `FakeFlight`.

use super::*;
use crate::flight::sim_port::NameSource;
use nova_sim::TagNames;

/// [`trading`]'s flight, its ship named Kestrel, with [`shipbuying`]'s
/// ship class 129 (long name "Federation Fast"), outfit 200 and `jünk`
/// 150, and reading `STR#` 4001's cargo names.
fn named() -> View {
    let mut data = trading();
    data.ships = shipbuying().ships;
    data.ships[0].long_name = "Federation Fast".to_owned();
    data.outfits = vec![holding(200, ROCKET)];
    data.junk = vec![JunkRecord {
        id: nova_sim::JunkId(150),
        name: "Bio-weapons".to_owned(),
        abbrev: "Bio".to_owned(),
        base_price: 1000,
        sold_at: Vec::new(),
        bought_at: Vec::new(),
        buy_on: nova_sim::Test::default(),
        sell_on: nova_sim::Test::default(),
        flags: 0,
        scan_mask: 0,
        lc_name: "bio-weapons".to_owned(),
    }];
    let strings = FakeCatalog {
        strings: vec![(4001, vec!["Food".to_owned(), "*Passengers".to_owned()])],
        ..catalog()
    };
    let session = SessionBuilder::flying(&data)
        .with_ship_name("Kestrel")
        .build()
        .with_strings(Rc::new(strings));
    FlightView::from_session(data, session.into(), &Rulebook::default())
}

#[test]
fn the_flights_names_and_date_are_its_sessions() {
    let view = named();
    let flight = &view.sim;
    let session = view.session().expect("flying");
    let ship = ShipId(129);
    assert_eq!(flight.ship_name(ship), Some("Fast"));
    assert_eq!(flight.ship_name(ship), session.ship_name(ship));
    assert_eq!(flight.ship_long_name(ship), Some("Federation Fast"));
    assert_eq!(flight.ship_long_name(ship), session.ship_long_name(ship));
    let outfit = OutfitId(200);
    assert_eq!(flight.outfit_name(outfit), Some("Gun 200"));
    assert_eq!(flight.outfit_name(outfit), session.outfit_name(outfit));
    assert_eq!(flight.outfit_names(outfit), Some(("gun 200", "guns 200")));
    assert_eq!(flight.outfit_names(outfit), session.outfit_names(outfit));
    let food = Good::Commodity(0);
    assert_eq!(flight.good_name(food), Some("Food"));
    assert_eq!(flight.good_name(food), session.good_name(food));
    let junk = nova_sim::JunkId(150);
    assert_eq!(flight.junk_lc_name(junk), Some("bio-weapons"));
    assert_eq!(flight.junk_lc_name(junk), session.junk_lc_name(junk));
    assert_eq!(flight.cargo_name(1).as_deref(), Some("*Passengers"));
    assert_eq!(flight.cargo_name(1), TagNames::cargo_name(session, 1));
    assert_eq!(flight.player_ship_name(), Some("Kestrel"));
    assert_eq!(flight.long_date(), session.long_date_text());
    assert!(!flight.long_date().is_empty(), "the date is said");
}
