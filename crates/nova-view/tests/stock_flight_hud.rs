//! The flight HUD over the stock data: a new pilot, whose shuttle has no
//! government, shows `ïntf` 128, the "Default status bar", over its 194 x
//! 767 background against the right edge, a Fed Destroyer shows the
//! Federation's `ïntf` 130, and every stock `ïntf` reads; and
//! the combat looks: every weapon the simulation flies, every explosion
//! type, every cargo box graphic and a ship's target card read, and
//! every looped weapon sound's length; a boarding grant's message reads
//! the strings it is built from; the cargo area's strings and area read; and the radar draws the
//! stock stations, hypergates and wormholes as squares. Skips, passing,
//! when `NOVA_DATA` is unset.

// The stock areas are small whole numbers, exact in floating point.
#![allow(clippy::float_cmp)]

mod common;

use nova_data::records::interface::Interface;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::records::weapon::Weapon;
use nova_data::{GameData, Record};
use nova_sim::{BoomId, Pilot, ShipId, SoundId, WeaponId};
use nova_view::flight::hud::{Background, bar_origin, interface_id};
use nova_view::flight::{CombatLooks, FlightView, Looks, StatusBars};
use nova_view::geometry::Bounds;
use nova_view::system::catalog::{SystemCatalog, SystemId};
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
            let sheet = look.sheet.expect("a projectile's sheet");
            let frames = sheet.expect("the sheet reads").frames.get();
            let expected = if id == 140 { 2 } else { 36 };
            assert_eq!(frames, expected, "wëap {id}");
        }
    }
    assert_eq!(projectiles, [128, 129, 138, 140, 181, 231]);
}

/// Every stock weapon whose sound loops (`Flags` 0x0010) knows how long
/// its sound lasts; the Hail Chaingun's (`snd ` 205) about 0.47 s, 15
/// ticks rounded up.
#[test]
fn every_looped_stock_weapon_knows_how_long_its_sound_lasts() {
    let Some(data) = open() else {
        return;
    };
    let mut looped = Vec::new();
    for (id, weapon) in data.records::<Weapon>() {
        let weapon = weapon.expect("decodes").record;
        if weapon.flags.bits() & 0x0010 == 0 || weapon.sound < 0 {
            continue;
        }
        let look = data.weapon_look(WeaponId(id)).expect("reads");
        assert!(look.sound_ticks.is_some_and(|ticks| ticks > 0), "wëap {id}");
        looped.push(id);
    }
    assert!(looped.contains(&155), "{looped:?}");
    let chaingun = data.weapon_look(WeaponId(155)).expect("the Hail Chaingun");
    assert_eq!(chaingun.sound, Some(SoundId(205)));
    // 10,304 frames at 22,050 Hz: 14.02 ticks, rounded up.
    assert_eq!(chaingun.sound_ticks, Some(15));
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
            .expect("there")
            .unwrap_or_else(|err| panic!("bööm {boom}: {err}"));
        let sheet = data.spin_sheet(spin).expect("the spïn reads");
        let image = look.sheet.expect("the sheet reads").image_id;
        assert_eq!(image, sheet.image_id, "bööm {boom}");
        assert_eq!(look.sound, Some(SoundId(sound)), "bööm {boom}");
    }
    assert_eq!(data.boom_look(BoomId(143)), None, "past the last");
}

#[test]
fn every_cargo_box_graphic_reads_its_spin() {
    let Some(data) = open() else {
        return;
    };
    for graphic in 0..5 {
        let sheet = data
            .box_sheet(graphic)
            .unwrap_or_else(|err| panic!("box {graphic}: {err}"));
        let spin = data.spin_sheet(500 + graphic.cast_signed()).expect("reads");
        assert_eq!(sheet.image_id, spin.image_id, "box {graphic}");
    }
}

#[test]
fn the_looks_the_flight_reads_have_no_problems() {
    let Some(data) = open() else {
        return;
    };
    let weapons = data.records::<Weapon>().map(|(id, _)| WeaponId(id));
    let looks = Looks::read(&data, weapons);
    assert_eq!(looks.problems(), Vec::<String>::new());
    assert_eq!(looks.booms.len(), 15, "bööm 128-142");
}

#[test]
fn the_first_ships_target_card_has_its_picture() {
    let Some(data) = open() else {
        return;
    };
    let card = data.target_card(ShipId(128));
    assert_eq!(card.picture, Some(3000));
}

/// A grant's message is built from `STR#` 2002 #106, #108, #393 and
/// #394, and a boarding pickup's from #106-#108, as the stock strings
/// read.
#[test]
fn a_grants_message_reads_its_stock_strings() {
    use nova_view::flight::view::{ARTICLE_A, ARTICLE_AN, FROM_THIS_SHIP, RETRIEVED, THE};
    let Some(data) = open() else {
        return;
    };
    let strings = nova_sim::CommCatalog::string_list(&data, 2002);
    assert_eq!(
        [
            &strings[105],
            &strings[106],
            &strings[107],
            &strings[392],
            &strings[393]
        ],
        [RETRIEVED, THE, FROM_THIS_SHIP, ARTICLE_A, ARTICLE_AN]
    );
}

/// The dock request's line, its denials and the dock approach's greeting
/// are built from `STR#` 2002 #53 and #74-#101, as the stock strings read.
#[test]
fn the_dock_requests_and_the_approachs_words_read_their_stock_strings() {
    use nova_view::flight::view::{
        BEGIN_APPROACH, CLEARED_FOR_HYPERGATE, CLEARED_TO_DOCK, CLEARED_TO_DOCK_LEAD,
        CLEARED_TO_LAND, CLEARED_TO_LAND_LEAD, COMMENCE_FINAL_APPROACH, DOCKING_DENIED,
        DOCKING_REQUEST_RECEIVED, DOCKMASTER_READS_YOU, HYPERGATE_DENIED, HYPERGATE_ENERGIZED,
        HYPERGATE_ONLINE, LANDING_DENIED, LANDING_REQUEST_RECEIVED, NO_RESPONSE,
        TRAFFIC_CONTROL_READS_YOU, WELCOME_TO, YOU_ARE_CLEARED_FOR_HYPERGATE,
        YOU_ARE_CLEARED_TO_DOCK, YOU_ARE_CLEARED_TO_LAND, YOURE_CLEARED_FOR_HYPERGATE,
    };
    let Some(data) = open() else {
        return;
    };
    let strings = nova_sim::CommCatalog::string_list(&data, 2002);
    assert_eq!(strings[52], NO_RESPONSE);
    assert_eq!(
        &strings[73..83],
        [
            HYPERGATE_ENERGIZED,
            HYPERGATE_ONLINE,
            DOCKMASTER_READS_YOU,
            DOCKING_REQUEST_RECEIVED,
            TRAFFIC_CONTROL_READS_YOU,
            LANDING_REQUEST_RECEIVED,
            BEGIN_APPROACH,
            HYPERGATE_DENIED,
            DOCKING_DENIED,
            LANDING_DENIED,
        ]
    );
    assert_eq!(
        &strings[90..101],
        [
            CLEARED_FOR_HYPERGATE,
            YOURE_CLEARED_FOR_HYPERGATE,
            YOU_ARE_CLEARED_FOR_HYPERGATE,
            CLEARED_TO_DOCK_LEAD,
            CLEARED_TO_DOCK,
            YOU_ARE_CLEARED_TO_DOCK,
            CLEARED_TO_LAND_LEAD,
            CLEARED_TO_LAND,
            YOU_ARE_CLEARED_TO_LAND,
            COMMENCE_FINAL_APPROACH,
            WELCOME_TO,
        ]
    );
}

/// A scan's notices are built from `STR#` 2002 #375-#380, as the stock
/// strings read ("posession" as they spell it).
#[test]
fn a_scans_notices_read_their_stock_strings() {
    use nova_view::flight::view::{
        ATTEMPT_TO_SMUGGLE, FINED, HAS_BEEN_DETECTED, ILLEGAL_CARGO, POSSESSION_OF,
        THROUGH_THIS_SYSTEM,
    };
    let Some(data) = open() else {
        return;
    };
    let strings = nova_sim::CommCatalog::string_list(&data, 2002);
    assert_eq!(
        &strings[374..380],
        [
            POSSESSION_OF,
            ATTEMPT_TO_SMUGGLE,
            ILLEGAL_CARGO,
            THROUGH_THIS_SYSTEM,
            HAS_BEEN_DETECTED,
            FINED
        ]
    );
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
    let view = FlightView::new(&data);
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
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["ship"] = serde_json::json!(141);
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    assert_eq!(pilot.ship(), ShipId(141));
    let view = FlightView::with_pilot(&data, pilot);
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

#[test]
fn stock_stations_and_gates_are_squares_on_the_radar_and_the_rest_ovals() {
    let Some(data) = open() else {
        return;
    };
    let (mut stations, mut gates, mut ovals) = (0, 0, 0);
    for &id in data.ids(System::TYPE) {
        for stellar in SystemCatalog::system(&data, SystemId(id)).stellars {
            let Some(Ok(spob)) = data.get::<Stellar>(stellar.id.0) else {
                continue;
            };
            let station = spob.record.flags.bits() & 0x10 != 0;
            let gate = spob.record.flags2.bits() & 0x3000 != 0;
            assert_eq!(
                stellar.radar_square,
                station || gate,
                "spöb {} in sÿst {id}",
                stellar.id.0
            );
            stations += usize::from(station);
            gates += usize::from(gate);
            ovals += usize::from(!station && !gate);
        }
    }
    assert!(
        stations > 0 && gates > 0 && ovals > 0,
        "{stations} {gates} {ovals}"
    );
}

/// The tutorial's hints are `STR#` 2002 #22-#28, #30 and #31, and the
/// take-off greeting's words #55-#60, as the stock strings read.
#[test]
fn the_tutorial_and_the_greeting_read_their_stock_strings() {
    use nova_view::flight::view::{
        FAR_ENOUGH, NOTHING_OUT_HERE, TAKE_OFF_LEADS, TAKE_OFF_ON, TUTORIAL_DOCK,
        TUTORIAL_HYPERSPACE, TUTORIAL_JUMP, TUTORIAL_LAND, TUTORIAL_LOOK_AROUND, TUTORIAL_MAP,
        TUTORIAL_REQUEST,
    };
    let Some(data) = open() else {
        return;
    };
    let strings = nova_sim::CommCatalog::string_list(&data, 2002);
    assert_eq!(
        &strings[21..28],
        [
            TUTORIAL_DOCK,
            TUTORIAL_LAND,
            TUTORIAL_LOOK_AROUND,
            TUTORIAL_REQUEST,
            TUTORIAL_HYPERSPACE,
            TUTORIAL_MAP,
            TUTORIAL_JUMP
        ]
    );
    assert_eq!(&strings[29..31], [FAR_ENOUGH, NOTHING_OUT_HERE]);
    assert_eq!(&strings[54..59], TAKE_OFF_LEADS);
    assert_eq!(strings[59], TAKE_OFF_ON);
}
