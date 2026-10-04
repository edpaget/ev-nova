//! The flight HUD over the stock data: a new pilot, with no government,
//! shows `ïntf` 128, the "Default status bar", over its 194 x 767
//! background against the right edge, and every stock `ïntf` reads. Skips,
//! passing, when `NOVA_DATA` is unset.

// The stock areas are small whole numbers, exact in floating point.
#![allow(clippy::float_cmp)]

mod common;

use nova_data::records::interface::Interface;
use nova_data::{GameData, Record};
use nova_view::flight::hud::{Background, bar_origin, interface_id};
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
    let view = FlightView::new(&data);
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
    assert_eq!(bar_origin(bar), Point::new(830.0, 0.0));
    let mut list = DrawList::new();
    view.draw(&mut list);
    assert!(list.iter().any(|c| *c
        == DrawCommand::Picture {
            image: ImageKey::picture(700),
            top_left: Point::new(830.0, 0.0),
        }));
}

#[test]
fn every_stock_status_bar_reads_with_a_background() {
    let Some(data) = open() else {
        return;
    };
    let ids: Vec<i16> = data.ids(Interface::TYPE).to_vec();
    assert_eq!(ids, (128..=134).collect::<Vec<_>>());
    for id in ids {
        let layout = data.status_bar(id).expect("reads");
        let picture = interface_id(layout.status_bkgnd);
        assert_eq!(picture, 572 + id, "ïntf {id}");
        assert_eq!(data.picture_size(picture), Some((194, 767)), "ïntf {id}");
    }
}
