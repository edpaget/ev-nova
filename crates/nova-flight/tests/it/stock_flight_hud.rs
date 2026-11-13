//! The flight HUD over the stock data, on a real session: a new pilot,
//! whose shuttle has no government, shows `ïntf` 128, the "Default status
//! bar", over its 194 x 767 background against the right edge, a Fed
//! Destroyer shows the Federation's `ïntf` 130, and the cargo area's
//! strings and area read. The rest of the HUD's stock tests start no
//! session and stay in nova-view-it's `stock_flight_hud`. Skips, passing,
//! when `NOVA_DATA` is unset.

// The stock areas are small whole numbers, exact in floating point.
#![allow(clippy::float_cmp)]

use crate::common;

use std::rc::Rc;

use nova_data::GameData;
use nova_flight::SessionViewExt as _;
use nova_flight::test_flight;
use nova_sim::Rulebook;
use nova_sim::ShipId;
use nova_sim::fixture::SessionBuilder;
use nova_view::flight::hud::{Background, bar_origin};
use nova_view::flight::{FlightView, StatusBars};
use nova_view::geometry::Bounds;
use nova_view::{DrawCommand, DrawList, ImageKey, Point, Screen};

fn open() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
    Bounds {
        min: Point::new(left, top),
        max: Point::new(right, bottom),
    }
}

#[test]
fn a_new_pilot_flies_under_the_default_status_bar() {
    let Some(data) = open() else {
        return;
    };
    let view = test_flight::new(Rc::new(data));
    let bar = view.status_bar().expect("the stock status bar reads");
    assert_eq!(
        bar.background,
        Some(Background {
            id: 700,
            width: 194.0,
            height: 767.0,
        })
    );
    assert_eq!(bar.layout.radar, rect(8.0, 8.0, 184.0, 184.0));
    assert_eq!(bar.layout.shield, rect(35.0, 199.0, 184.0, 206.0));
    assert_eq!(bar.layout.armor, rect(35.0, 216.0, 184.0, 223.0));
    assert_eq!(bar.layout.fuel, rect(35.0, 234.0, 184.0, 241.0));
    assert_eq!(bar.layout.nav, rect(8.0, 254.0, 184.0, 286.0));
    assert_eq!(bar_origin(bar, 1024.0), Point::new(830.0, 0.0));
    let mut list = DrawList::new();
    view.draw(&mut list);
    assert!(list.iter().any(|c| *c
        == DrawCommand::Picture {
            image: ImageKey::picture(700),
            top_left: Point::new(830.0, 0.0),
        }));
}

/// The cargo area's labels are `STR#` 2002 #19-#21 and #33, its rows'
/// `STR#` 4003, and a mission's cargo names `STR#` 4002, as the stock
/// strings read, in the default bar's `CargoArea`.
#[test]
fn the_cargo_area_reads_its_stock_strings_and_area() {
    use nova_view::flight::hud::{
        CARGO_CREDITS, CARGO_FREE, CARGO_MULTIPLE, CARGO_SPECIAL, credits_label,
    };
    let Some(data) = open() else {
        return;
    };
    let strings = nova_sim::CommCatalog::string_list(&data, 2002);
    assert_eq!(
        [&strings[18], &strings[19], &strings[20], &strings[32]],
        [CARGO_FREE, CARGO_SPECIAL, CARGO_MULTIPLE, CARGO_CREDITS]
    );
    assert_eq!(credits_label(&strings[32]), "Credits:");
    assert_eq!(
        nova_sim::CommCatalog::string_list(&data, 4003),
        ["Food:", "Ind:", "Med:", "LuxG:", "Met:", "Equ:"]
    );
    assert_eq!(
        nova_sim::CommCatalog::string_list(&data, 4002)[..7],
        [
            "Food",
            "IndGds",
            "MedGds",
            "LuxGds",
            "Metal",
            "Equpt",
            "Passengers"
        ]
    );
    let view = test_flight::new(Rc::new(data));
    let bar = view.status_bar().expect("the stock status bar reads");
    assert_eq!(bar.layout.cargo, rect(8.0, 458.0, 184.0, 552.0));
}

#[test]
fn a_fed_destroyer_flies_under_the_federation_status_bar() {
    let Some(data) = open() else {
        return;
    };
    // shïp 141, the Fed Destroyer, is of gövt 128, the Federation, whose
    // Interface is ïntf 130, over PICT 702.
    let session = SessionBuilder::flying(&data)
        .named("Stock")
        .in_ship(ShipId(141))
        .build();
    assert_eq!(session.ship(), ShipId(141));
    let data = Rc::new(data);
    let view = FlightView::from_session(Rc::clone(&data), session.into(), &Rulebook::default());
    let bar = view.status_bar().expect("the Federation status bar reads");
    assert_eq!(bar.layout, data.status_bar(130).expect("reads"));
    assert_eq!(
        bar.background,
        Some(Background {
            id: 702,
            width: 194.0,
            height: 767.0,
        })
    );
}
