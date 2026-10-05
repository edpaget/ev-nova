//! The flight HUD over the stock data: a new pilot, with no government,
//! shows `ïntf` 128, the "Default status bar", over its 194 x 767
//! background against the right edge, and every stock `ïntf` reads; and
//! the combat looks: every weapon the simulation flies, every explosion
//! type and a ship's target card read. Skips, passing, when `NOVA_DATA`
//! is unset.

// The stock areas are small whole numbers, exact in floating point.
#![allow(clippy::float_cmp)]

mod common;

use nova_data::records::interface::Interface;
use nova_data::records::weapon::Weapon;
use nova_data::{GameData, Record};
use nova_sim::{BoomId, ShipId, SoundId, WeaponId};
use nova_view::flight::hud::{Background, bar_origin, interface_id};
use nova_view::flight::{CombatLooks, FlightView, StatusBars};
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

#[test]
fn the_default_status_bar_has_its_target_and_weapon_areas() {
    let Some(data) = open() else {
        return;
    };
    let layout = data.status_bar(128).expect("reads");
    assert_eq!(layout.targ, rect(8.0, 330.0, 184.0, 442.0));
    assert_eq!(layout.weap, rect(8.0, 300.0, 184.0, 315.0));
    assert_eq!((layout.font_size, layout.subtitle_size), (12.0, 10.0));
}

#[test]
fn every_weapon_the_simulation_flies_has_a_look() {
    let Some(data) = open() else {
        return;
    };
    let mut projectiles = Vec::new();
    for (id, weapon) in data.records::<Weapon>() {
        let weapon = weapon.expect("decodes").record;
        if ![-1, 0, 5, 6].contains(&weapon.guidance) {
            continue;
        }
        let look = data
            .weapon_look(WeaponId(id))
            .unwrap_or_else(|err| panic!("wëap {id}: {err}"));
        if weapon.guidance != 0 && weapon.graphic >= 0 {
            projectiles.push(id);
            let frames = look.sheet.expect("a projectile's sheet").frames.get();
            let expected = if id == 140 { 2 } else { 36 };
            assert_eq!(frames, expected, "wëap {id}");
        }
    }
    assert_eq!(projectiles, [128, 129, 138, 140, 181, 231]);
}

#[test]
fn every_explosion_type_reads_with_its_sound_and_sprite() {
    let Some(data) = open() else {
        return;
    };
    let table: [(i16, i16, i16); 15] = [
        (128, 302, 400),
        (129, 301, 401),
        (130, 300, 402),
        (131, 301, 403),
        (132, 302, 400),
        (133, 303, 402),
        (134, 301, 404),
        (135, 300, 405),
        (136, 302, 406),
        (137, 302, 407),
        (138, 301, 408),
        (139, 301, 409),
        (140, 302, 410),
        (141, 301, 411),
        (142, 314, 412),
    ];
    for (boom, sound, spin) in table {
        let look = data
            .boom_look(BoomId(boom))
            .unwrap_or_else(|err| panic!("bööm {boom}: {err}"));
        let sheet = data.spin_sheet(spin).expect("the spïn reads");
        assert_eq!(look.sheet.image_id, sheet.image_id, "bööm {boom}");
        assert_eq!(look.sound, Some(SoundId(sound)), "bööm {boom}");
    }
}

#[test]
fn the_first_ships_target_card_has_its_name_and_picture() {
    let Some(data) = open() else {
        return;
    };
    let card = data.target_card(ShipId(128));
    assert!(!card.name.is_empty());
    assert!(!card.name.contains(';'), "{}", card.name);
    assert_eq!(card.picture, Some(3000));
}
