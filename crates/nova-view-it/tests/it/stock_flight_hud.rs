//! The flight HUD over the stock data: every stock `ïntf` reads with a
//! background, and the default one has its areas; and
//! the combat looks: every weapon the simulation flies, every explosion
//! type, every cargo box graphic, the cursor and a ship's target card read, and
//! every looped weapon sound's length; every ship's sheet scans as it
//! decodes, and its target card reads the same in a batch; a boarding grant's message reads
//! the strings it is built from; and the radar draws the
//! stock stations, hypergates and wormholes as squares. The tests that fly
//! a real session under a status bar (a new pilot's, a Fed Destroyer's, the
//! cargo area's) are nova-flight's `stock_flight_hud`. Skips, passing,
//! when `NOVA_DATA` is unset.

// The stock areas are small whole numbers, exact in floating point.
#![allow(clippy::float_cmp)]

use crate::common;

use nova_data::records::interface::Interface;
use nova_data::records::weapon::Weapon;
use nova_data::{GameData, Record};
use nova_sim_frame::catalog::CommCatalog;
use nova_sim_frame::{BoomId, ShipId, SoundId, WeaponId};
use nova_view::flight::hud::interface_id;
use nova_view::flight::{CombatLooks, LayerSheet, Looks, ShipSheet, ShipSprites, StatusBars};
use nova_view::geometry::Bounds;
use nova_view::{Point, SheetImage};

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
        let image = look.sheet.expect("the sheet reads").image;
        assert_eq!(image, SheetImage::Rled(sheet.image_id), "bööm {boom}");
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
        assert_eq!(
            sheet.image,
            SheetImage::Rled(spin.image_id),
            "box {graphic}"
        );
    }
}

#[test]
fn the_cursors_sheet_is_spin_650s() {
    let Some(data) = open() else {
        return;
    };
    let sheet = data.cursor_sheet().expect("spïn 650 reads");
    assert_eq!(
        (sheet.image, sheet.frames.get()),
        (SheetImage::Rled(8080), 9),
        "rlëD 8080, 9 frames"
    );
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

/// The status bar and the pilot panel show the same target picture for
/// every stock `shïp`, and some take their base ship's.
#[test]
fn every_stock_ships_target_card_matches_the_pilot_panel() {
    use nova_data::graphics::PICT;
    use nova_data::records::ship::Ship;
    use nova_view::menu::PilotPanelSource;
    let Some(data) = open() else {
        return;
    };
    let mut borrowed = Vec::new();
    for &id in data.ids(Ship::TYPE) {
        let card = data.target_card(ShipId(id)).picture;
        assert_eq!(
            card,
            data.panel_ship(ShipId(id)).picture.map(|p| p.picture),
            "shïp {id}"
        );
        assert!(card.is_some(), "shïp {id} has a target picture");
        if data.resource(PICT, 3000 + (id - 128)).is_none() {
            borrowed.push(id);
        }
    }
    assert_ne!(borrowed, Vec::<i16>::new(), "some ship borrows its base's");
    let ships: Vec<ShipId> = data.ids(Ship::TYPE).iter().map(|&id| ShipId(id)).collect();
    let one_by_one: Vec<_> = ships.iter().map(|&ship| data.target_card(ship)).collect();
    assert_eq!(data.target_cards(&ships), one_by_one, "in one batch");
}

/// Every stock `shïp`'s target card is the one its pictures give when
/// each is sized by decoding it, as cards were read before
/// `picture_size` scanned: the same subtitle and picture.
#[test]
fn every_stock_ships_target_card_is_the_one_its_decoded_pictures_give() {
    use nova_data::graphics::{PICT, decode_pict};
    use nova_data::records::ship::Ship;
    use nova_view::flight::TargetCard;
    use nova_view::target_picture::{game_ship_bases, target_picture};
    let Some(data) = open() else {
        return;
    };
    let bases = game_ship_bases(&data);
    let decoded_size = |id: i16| {
        let image = decode_pict(data.resource(PICT, id)?.resource.data()).ok()?;
        Some((image.width(), image.height()))
    };
    let ships: Vec<ShipId> = data.ids(Ship::TYPE).iter().map(|&id| ShipId(id)).collect();
    let decoded: Vec<TargetCard> = ships
        .iter()
        .map(|&ship| TargetCard {
            subtitle: data
                .get::<Ship>(ship.0)
                .expect("a shïp")
                .expect("it decodes")
                .record
                .subtitle
                .as_str()
                .to_owned(),
            picture: target_picture(ship, &bases, decoded_size).map(|p| p.picture),
        })
        .collect();
    assert_eq!(data.target_cards(&ships), decoded);
}

/// A ship's sheet as the flight read it before scanning: from the
/// decoded base and layer sheets.
fn decoded_ship_sheet(data: &GameData, id: ShipId) -> Result<ShipSheet, String> {
    use nova_data::ShipImage;
    use nova_data::records::ship_anim::ShipAnim;
    use std::num::NonZeroU16;
    let sprite = data.ship_sprite(id).map_err(|err| err.to_string())?;
    let layer = |(image, layer): (
        ShipImage,
        Option<Result<nova_data::LayerSprite<'_>, nova_data::LayerError>>,
    )| {
        let layer = layer?.ok()?;
        Some(LayerSheet {
            image: SheetImage::of_ship(layer.kind, id.0, image, layer.image_id),
            frames: NonZeroU16::new(layer.sheet.frames().len() as u16)?,
            frame_width: layer.sheet.frame_width(),
            frame_height: layer.sheet.frame_height(),
            image_size: (layer.sheet.image_width(), layer.sheet.image_height()),
            clip_quirks: layer.sheet.clip_quirks(),
            solid: layer.sheet.solid_frames().is_some(),
            eight_bit: false,
        })
    };
    let [glow, lights, weapon, alt, shield] = data.ship_layers(id).map_or([None; 5], |l| {
        [
            (ShipImage::Glow, l.glow),
            (ShipImage::Lights, l.lights),
            (ShipImage::Weapon, l.weapon),
            (ShipImage::Alt, l.alt),
            (ShipImage::Shield, l.shield),
        ]
        .map(layer)
    });
    let anim = data.get::<ShipAnim>(id.0).and_then(Result::ok);
    let field = |read: fn(&ShipAnim) -> i16| anim.map_or(0, |anim| read(anim.record));
    Ok(ShipSheet {
        image: SheetImage::of_ship(sprite.kind, id.0, ShipImage::Base, sprite.image_id),
        rotations: NonZeroU16::new(sprite.sheet.layout().columns()).expect("a column"),
        frame_width: sprite.sheet.frame_width(),
        frame_height: sprite.sheet.frame_height(),
        image_size: (sprite.sheet.image_width(), sprite.sheet.image_height()),
        clip_quirks: sprite.sheet.clip_quirks(),
        glow,
        lights,
        weapon,
        weap_decay: field(|anim| anim.weap_decay),
        alt,
        shield,
        base_transp: field(|anim| anim.base_transp),
        solid: sprite.sheet.solid_frames().is_some(),
        eight_bit: false,
    })
}

/// Every stock ship's sheet, read by scanning, is what decoding gave.
#[test]
fn every_stock_ships_scanned_sheet_matches_its_decoded_one() {
    use nova_data::records::ship::Ship;
    let Some(data) = open() else {
        return;
    };
    let mut solid = 0;
    for &id in data.ids(Ship::TYPE) {
        let scanned = data.ship_sheet(ShipId(id));
        assert_eq!(scanned, decoded_ship_sheet(&data, ShipId(id)), "shïp {id}");
        solid += usize::from(scanned.is_ok_and(|sheet| sheet.weapon.is_some_and(|w| w.solid)));
    }
    assert_eq!(solid, 1, "only the Thunderforge's weapon glow has runs");
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
    let strings = CommCatalog::string_list(&data, 2002);
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
/// are built from `STR#` 2002 #53 and #65-#101, as the stock strings read.
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
    use nova_view::flight::view::{
        DOCK_AT, HOSTILE_PLANET, HOSTILE_STATION, HYPERGATE_OFFLINE, LAND_ON, TOO_FAR_HYPERGATE,
        TOO_FAR_PLANET, TOO_FAR_STATION, TOO_FAR_WORMHOLE, TOO_FAST_HYPERGATE, TOO_FAST_PLANET,
        TOO_FAST_STATION, TOO_FAST_WORMHOLE, UNABLE_TO, WORMHOLE_TOO_HOT,
    };
    let Some(data) = open() else {
        return;
    };
    let strings = CommCatalog::string_list(&data, 2002);
    assert_eq!(strings[52], NO_RESPONSE);
    assert_eq!(
        &strings[64..72],
        [
            TOO_FAR_HYPERGATE,
            TOO_FAR_WORMHOLE,
            TOO_FAR_STATION,
            TOO_FAR_PLANET,
            TOO_FAST_HYPERGATE,
            TOO_FAST_WORMHOLE,
            TOO_FAST_STATION,
            TOO_FAST_PLANET,
        ]
    );
    assert_eq!(
        &strings[83..90],
        [
            UNABLE_TO,
            HYPERGATE_OFFLINE,
            WORMHOLE_TOO_HOT,
            DOCK_AT,
            LAND_ON,
            HOSTILE_STATION,
            HOSTILE_PLANET,
        ]
    );
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

/// The sabotage notices are built from `STR#` 2002 #38-#41, as the stock
/// strings read.
#[test]
fn the_sabotage_notices_read_their_stock_strings() {
    use nova_view::flight::view::{BOMB_EXPLODED_ONBOARD, HAS_EXPLODED, HAVE_EXPLODED, YOUR};
    let Some(data) = open() else {
        return;
    };
    let strings = CommCatalog::string_list(&data, 2002);
    assert_eq!(
        [&strings[37], &strings[38], &strings[39], &strings[40]],
        [BOMB_EXPLODED_ONBOARD, YOUR, HAS_EXPLODED, HAVE_EXPLODED]
    );
}

/// The self-destruct's lines are built from `STR#` 2002 #385-#390 and
/// #395, as the stock strings read; #388 is the other reading's alone.
#[test]
fn the_self_destructs_lines_read_their_stock_strings() {
    use nova_view::flight::view::{
        HAVE_A_NICE_DAY, SECOND, SECONDS, SELF_DESTRUCT_IN, SELF_DESTRUCT_LEAD, SEQUENCE_CANCELLED,
        SEQUENCE_INITIATED,
    };
    let Some(data) = open() else {
        return;
    };
    let strings = CommCatalog::string_list(&data, 2002);
    assert_eq!(
        [
            &strings[384],
            &strings[385],
            &strings[386],
            &strings[387],
            &strings[388],
            &strings[389],
            &strings[394],
        ],
        [
            SELF_DESTRUCT_LEAD,
            SEQUENCE_INITIATED,
            SEQUENCE_CANCELLED,
            SECOND,
            SECONDS,
            HAVE_A_NICE_DAY,
            SELF_DESTRUCT_IN,
        ]
    );
}

/// The frame-rate readout's lines are built from `STR#` 2002 #282 and
/// #283, as the stock strings read.
#[test]
fn the_frame_rate_readouts_lines_read_their_stock_strings() {
    use nova_view::flight::frame_rate::{CALCULATING_FPS, FPS};
    let Some(data) = open() else {
        return;
    };
    let strings = CommCatalog::string_list(&data, 2002);
    assert_eq!([&strings[281], &strings[282]], [CALCULATING_FPS, FPS]);
}

/// A disabled player's jump collapse says `STR#` 2002 #35, as the stock
/// string reads.
#[test]
fn the_jump_collapse_notice_reads_its_stock_string() {
    use nova_view::flight::view::JUMP_COLLAPSED;
    let Some(data) = open() else {
        return;
    };
    let strings = CommCatalog::string_list(&data, 2002);
    assert_eq!(strings[34], JUMP_COLLAPSED);
}

/// A jump refused while a tractor beam holds the player says `STR#` 2002
/// #36, as the stock strings read.
#[test]
fn a_tractored_jump_says_its_stock_string() {
    use nova_view::flight::view::TRACTORED;
    let Some(data) = open() else {
        return;
    };
    let strings = CommCatalog::string_list(&data, 2002);
    assert_eq!(strings[35], TRACTORED);
}

/// The player's Repair Droids repairing it say `STR#` 2002 #37, as the
/// stock string reads.
#[test]
fn the_propulsion_repaired_notice_reads_its_stock_string() {
    use nova_view::flight::view::PROPULSION_REPAIRED;
    let Some(data) = open() else {
        return;
    };
    let strings = CommCatalog::string_list(&data, 2002);
    assert_eq!(strings[36], PROPULSION_REPAIRED);
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
    let strings = CommCatalog::string_list(&data, 2002);
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
    let strings = CommCatalog::string_list(&data, 2002);
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

/// The tutorial hints' key names (`_HandlePlayer` @0x70b01-0x70b1b,
/// @0x70c3d-0x70cb2) are `Nova-DF.rsrc`'s `STR#` 129, entry by entry. The
/// Windows `Nova.rez` is another format, so only the Mac file is read.
#[test]
fn the_hints_key_names_are_the_interface_file_s_str_129() {
    use nova_view::bound_names::STR_129;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let Some(path) = common::interface_file(&dir) else {
        return;
    };
    let ui = nova_data::InterfaceData::open(&path).expect("the interface file opens");
    let (list, _) = ui
        .string_list(129)
        .expect("STR# 129 is there")
        .expect("STR# 129 decodes");
    let strings: Vec<&str> = list.record.strings.iter().map(|s| &**s).collect();
    assert_eq!(strings, STR_129);
}
