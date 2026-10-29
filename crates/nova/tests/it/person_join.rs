//! Persons joining the player and boarding grants through the whole app:
//! synthetic game data (a `chär` flying the "Gunship", ship 128, from
//! Alpha, whose one stellar, a pad at the centre, can be landed on; Alpha
//! has no traffic of its own but two Person slots at 100 %: "Merchant",
//! `përs` 128, independent, of `Flags` 0x0040, in the "Hauler", whose
//! `LinkMission` 128, offered from a ship (`AvailLoc` 2) on every visit
//! and with no offer text, has one special ship escorting the player
//! (`mïsn` `ShipCount` 1, `ShipGoal` 3); and "Wreck", `përs` 129, of a
//! derelict government (`gövt` `Flags` 0x0800), so disabled from the
//! start, in the "Hulk" (a crew of 3), granting up to 2 outfits of class
//! 7 every time; and `oütf` 128, "Spare Part", of `ItemClass` 7,
//! massless) and a synthetic interface file holding the stock comm dialog
//! and an offer dialog, driven only by window events, with pilots kept in
//! the in-memory store.
//!
//! By the engine's default, hailing Merchant opens the offer of mission
//! 128 over the paused flight in place of the comm dialog: Return accepts
//! it and N refuses it, each closing the dialog and resuming flight. A
//! mission that cannot be refused, with no offer text, is accepted at
//! once and shows nothing. With mission 128's `BriefText` "Stay close."
//! and `RefuseText` "Your loss.", accepting it shows the first and
//! refusing it the second over the paused flight, until Done goes back
//! to flight; accepted at once by a hail made with Up held, its
//! `BriefText` shows and lets go of Up, so the ship coasts after Done. With `person_join` set to its other reading in
//! the settings file, hailing Merchant opens the comm dialog listing
//! Greetings, Request Assistance and Use As Escort, and U makes it say
//! "Okay, I'm on my way." and join the fleet; landing saves the pilot
//! with the escort's `"person": 128`, and the pilot reopened in a new app
//! takes off with Merchant beside it, shown by its name. Boarding Wreck
//! says "You retrieved two spare parts from this ship." in the message
//! line, opens the plunder dialog and adds the two to the pilot's
//! outfits; with a draw that yields one, it says "a spare part". Wreck of
//! `Flags` 0x0200, linked to mission 128, offers it on boarding in place
//! of the plunder dialog.

use crate::bracket_art;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use crate::common::EntryHarness;
use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::desc::Desc;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::govt::Govt;
use nova_data::records::interface::Interface;
use nova_data::records::mission::Mission;
use nova_data::records::outfit::Outfit;
use nova_data::records::person::Person;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, InterfaceData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_sim::fixture::MemoryPilots;
use nova_sim::{
    Behaviour, Chance, Goal, Npc, OutfitId, Pilot, PilotKeeper, PilotStore, Session, Surroundings,
};
use nova_view::Key;
use nova_view::flight::SharedChance;
use nova_view::geometry::Point;
use nova_view::menu::MenuChoice;
use nova_view::movie::MoviePlayer;
use nova_view::text::fixture::MonoMetrics;

/// A 1024 x 768 window at scale 1: window pixels are logical units.
struct FakeWindow;

impl WindowPort for FakeWindow {
    fn size_px(&self) -> (u32, u32) {
        (1024, 768)
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }

    fn request_redraw(&mut self) {}
}

/// One data file, `/data/Nova Data`, holding a fork.
struct OneFile(Vec<u8>);

impl DirLister for OneFile {
    fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
        Ok(vec![Listing {
            name: "Nova Data".into(),
            kind: EntryKind::File,
        }])
    }
}

impl ForkReader for OneFile {
    fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        Ok((fork == Fork::Data).then(|| self.0.clone()))
    }
}

fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
    }
}

fn put_u32s(bytes: &mut [u8], at: usize, values: &[u32]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 4 * i..at + 4 * i + 4].copy_from_slice(&value.to_be_bytes());
    }
}

/// The `chär`: ship 128 in system 128, no legal records.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    bytes
}

/// An unarmed `shïp` of 30 shield and 45 armour, a trader escort
/// (`InherentAI` 1) with a crew of `crew`, named `name` when hailed.
fn ship(crew: i16, name: &str) -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[30, 300, 300, 30, 300]);
    put_i16s(&mut bytes, 0x0E, &[45]);
    put_i16s(&mut bytes, 0x12, &[-1; 4]);
    put_i16s(&mut bytes, 0x1A, &[1; 4]);
    put_i16s(&mut bytes, 0x38, &[-1, 0]);
    put_i16s(&mut bytes, 0x42, &[1, crew]);
    put_i16s(&mut bytes, 0x4E, &[-1; 4]);
    put_i16s(&mut bytes, 0x370, &[-1; 4]);
    put_i16s(&mut bytes, 0x6CE, &[-1; 4]);
    bytes[0x60E..0x60E + name.len()].copy_from_slice(name.as_bytes());
    bytes
}

/// A `gövt` of `flags`, of no class, ally or enemy.
fn govt(flags: u16) -> Vec<u8> {
    let mut bytes = vec![0; Govt::SIZE.expect("fixed")];
    bytes[0x02..0x04].copy_from_slice(&flags.to_be_bytes());
    put_i16s(&mut bytes, 0x08, &[6, 0, 3, 0, 7, 0, 0, 100]);
    put_i16s(&mut bytes, 0x18, &[-1; 12]);
    bytes
}

/// A `përs` of `govt`, a warship flying `ship`, linked nowhere, of no
/// quotes, with these `Flags`, `LinkMission` and grant (`GrantClass`,
/// `GrantCount`, `GrantProb`).
fn person(govt: i16, ship: i16, flags: u16, mission: i16, grant: [i16; 3]) -> Vec<u8> {
    let mut bytes = vec![0; Person::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[-2, govt, 3, 2, 0, ship]);
    put_i16s(&mut bytes, 0x0C, &[-1; 4]);
    put_i16s(&mut bytes, 0x2A, &[-1, -1, -1, mission]);
    bytes[0x32..0x34].copy_from_slice(&flags.to_be_bytes());
    put_i16s(&mut bytes, 0x134, &grant);
    bytes
}

/// A `mïsn` offered from a ship (`AvailLoc` 2) on every visit
/// (`AvailRandom` 100), of these `Flags`, with one special ship escorting
/// the player, its `BriefText` and `RefuseText` the `dësc`s `brief` and
/// `refuse`.
fn escort_mission(flags: u16, brief: i16, refuse: i16) -> Vec<u8> {
    let mut bytes = vec![0; Mission::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[2]);
    put_i16s(&mut bytes, 0x0A, &[100]);
    put_i16s(&mut bytes, 0x20, &[1]);
    put_i16s(&mut bytes, 0x26, &[3]);
    put_i16s(&mut bytes, 0x34, &[brief]);
    bytes[0x50..0x52].copy_from_slice(&flags.to_be_bytes());
    put_i16s(&mut bytes, 0x58, &[refuse]);
    bytes
}

/// A `dësc` of `text`.
fn desc(text: &str) -> Vec<u8> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend([0xFF; 2]);
    bytes.extend([0; 34]);
    bytes
}

/// A `dësc` of `text` naming the movie `file`, played as `flags` say.
fn filmed(text: &str, file: &str, flags: u16) -> Vec<u8> {
    let mut bytes = desc(text);
    let at = text.len() + 3;
    bytes[at..at + file.len()].copy_from_slice(file.as_bytes());
    bytes[at + 32..at + 34].copy_from_slice(&flags.to_be_bytes());
    bytes
}

/// What the game data's knobs are set to: Wreck's `Flags` and
/// `LinkMission`, and mission 128's `Flags`, `BriefText` and
/// `RefuseText`.
#[derive(Clone, Copy)]
struct Knobs {
    wreck_flags: u16,
    wreck_mission: i16,
    mission_flags: u16,
    brief: i16,
    refuse: i16,
    /// The `Flags` of mission 128's offer `dësc` 4000, of
    /// [`Knobs::offer_text`], naming the movie "Offer.mov": none without
    /// one.
    offer_movie: Option<u16>,
    /// The text of mission 128's offer `dësc` 4000: with no
    /// [`Knobs::offer_movie`] and no text, there is no such `dësc`.
    offer_text: &'static str,
    /// The `Flags` of the `BriefText`'s `dësc` 200, naming "Brief.mov".
    brief_movie: Option<u16>,
}

/// The game data as the module docs give it.
const PLAIN: Knobs = Knobs {
    wreck_flags: 0,
    wreck_mission: -1,
    mission_flags: 0,
    brief: -1,
    refuse: -1,
    offer_movie: None,
    offer_text: "",
    brief_movie: None,
};

/// The game data with mission 128's `BriefText` `dësc` 200, "Stay close.",
/// and its `RefuseText` `dësc` 201, "Your loss.".
const TEXTED: Knobs = Knobs {
    brief: 200,
    refuse: 201,
    ..PLAIN
};

/// An `oütf` of `ItemClass` 7, massless, up to 10 owned, "spare part" and
/// "spare parts".
fn spare_part() -> Vec<u8> {
    let mut bytes = vec![0; Outfit::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 1]);
    put_i16s(&mut bytes, 0x0A, &[10]);
    bytes[0x36B..0x36B + 10].copy_from_slice(b"spare part");
    bytes[0x3AB..0x3AB + 11].copy_from_slice(b"spare parts");
    // `ItemClass` 7, for sale every day (`BuyRandom` 100).
    put_i16s(&mut bytes, 0x3EC, &[7, 0, 100]);
    bytes
}

/// A `shän` whose base image is `rlëD` `image`, one set of 36 rotations.
fn ship_anim(image: i16) -> Vec<u8> {
    let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, 0, 1]);
    put_i16s(&mut bytes, 0x34, &[36]);
    bytes
}

/// A `spïn` naming `rlëD` `image`, one frame across.
fn spin(image: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, -1, 0, 0, 1, 1]);
    bytes
}

/// An `rlëD` of `frames` frames of `size` x `size`.
fn sheet(frames: u16, size: u16) -> Vec<u8> {
    (0..frames)
        .fold(RledBuilder::new(size, size), |sheet, _| {
            sheet.frame(|f| (0..size).fold(f, |f, _| f.line().pixels(&vec![0x0400; size.into()])))
        })
        .build()
}

/// A `width` x `height` picture of one colour.
fn pict(width: i16, height: i16, rgb: [u8; 3]) -> Vec<u8> {
    let frame = [0, 0, height, width];
    let pixels = vec![rgb; (width * height) as usize];
    PictBuilder::new(frame)
        .direct_bits(&DirectBits::rgb888(frame, &pixels))
        .end()
        .build()
}

/// Alpha: independent, with the pad, no traffic of its own, and Merchant
/// and Wreck in its Person slots at 100 %.
fn alpha() -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[128]);
    put_i16s(&mut bytes, 0x44, &[-1; 8]);
    put_i16s(&mut bytes, 0x64, &[0, -1]);
    put_i16s(&mut bytes, 0x6E, &[128, 129, -1, -1, -1, -1, -1, -1]);
    put_i16s(&mut bytes, 0x7E, &[100, 100]);
    bytes
}

/// The pad: a `spöb` at the centre that can be landed on, drawn from
/// `spïn` 1004.
fn pad() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 4]);
    bytes[0x06..0x0A].copy_from_slice(&0x01_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x14, &[-1]);
    put_i16s(&mut bytes, 0x18, &[-1, -1]);
    bytes
}

/// Stock `ïntf` 128's areas and font, over background `PICT` 700.
fn status_bar() -> Vec<u8> {
    let mut bytes = vec![0; Interface::SIZE.expect("fixed")];
    put_u32s(&mut bytes, 0x00, &[0x00FF_FFFF, 0x0080_8080]);
    put_i16s(&mut bytes, 0x08, &[8, 8, 184, 184]);
    put_u32s(&mut bytes, 0x10, &[0x0000_FF00, 0x0000_8000]);
    put_i16s(&mut bytes, 0x18, &[199, 35, 206, 184]);
    put_u32s(&mut bytes, 0x20, &[0x0000_00FF]);
    put_i16s(&mut bytes, 0x24, &[216, 35, 223, 184]);
    put_u32s(&mut bytes, 0x2C, &[0x00FF_0000]);
    put_i16s(&mut bytes, 0x30, &[234, 35, 241, 184]);
    put_u32s(&mut bytes, 0x38, &[0x00FF_FF00, 0x0080_8000]);
    put_i16s(&mut bytes, 0x40, &[254, 8, 286, 184]);
    put_i16s(&mut bytes, 0x48, &[300, 8, 315, 184]);
    put_i16s(&mut bytes, 0x50, &[330, 8, 442, 184]);
    bytes[0x60..0x66].copy_from_slice(b"Geneva");
    put_i16s(&mut bytes, 0xA0, &[12, 10, 700]);
    bytes
}

/// A `STR#` of `strings`.
fn strings(strings: &[String]) -> Vec<u8> {
    let mut bytes = u16::try_from(strings.len())
        .expect("few")
        .to_be_bytes()
        .to_vec();
    for string in strings {
        bytes.push(u8::try_from(string.len()).expect("short"));
        bytes.extend(string.as_bytes());
    }
    bytes
}

/// `STR#` 3000: each group's five variants "c<g>", but "Channel open."
/// (0) and "Okay, I'm on my way." (29).
fn comm_strings() -> Vec<String> {
    (0..40)
        .flat_map(|group| {
            let said = match group {
                0 => "Channel open.".to_owned(),
                29 => "Okay, I'm on my way.".to_owned(),
                _ => format!("c{group}"),
            };
            std::iter::repeat_n(said, 5)
        })
        .collect()
}

/// The game data (see the module docs), its knobs set as `knobs` say.
fn game_data(knobs: Knobs) -> Rc<GameData> {
    let messages: Vec<String> = (1..=200).map(|n| format!("m{n}")).collect();
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Gunship"), &ship(10, "Gunship"))
        .resource(Ship::TYPE, 129, Some(b"Hauler"), &ship(3, "Hauler"))
        .resource(Ship::TYPE, 130, Some(b"Hulk"), &ship(3, "Hulk"))
        .resource(Govt::TYPE, 150, Some(b"Derelicts"), &govt(0x0800))
        .resource(
            Person::TYPE,
            128,
            Some(b"Merchant"),
            &person(-1, 129, 0x0040, 128, [0, 0, 0]),
        )
        .resource(
            Person::TYPE,
            129,
            Some(b"Wreck"),
            &person(
                150,
                130,
                knobs.wreck_flags,
                knobs.wreck_mission,
                [7, 2, 100],
            ),
        )
        .resource(
            Mission::TYPE,
            128,
            Some(b"Escort"),
            &escort_mission(knobs.mission_flags, knobs.brief, knobs.refuse),
        )
        .resource(
            Desc::TYPE,
            200,
            None,
            &match knobs.brief_movie {
                Some(flags) => filmed("Stay close.", "Brief.mov", flags),
                None => desc("Stay close."),
            },
        )
        .resource(Desc::TYPE, 201, None, &desc("Your loss."))
        .resource(Outfit::TYPE, 128, Some(b"Spare Part"), &spare_part())
        .resource(System::TYPE, 128, Some(b"Alpha"), &alpha())
        .resource(Stellar::TYPE, 128, Some(b"Pad"), &pad())
        .resource(Spin::TYPE, 1004, None, &spin(1000))
        .resource(RLED, 1000, None, &sheet(1, 40))
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(ShipAnim::TYPE, 129, None, &ship_anim(2001))
        .resource(ShipAnim::TYPE, 130, None, &ship_anim(2001))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(RLED, 2001, None, &sheet(36, 6))
        .resource(Spin::TYPE, 3000, None, &spin(3000))
        .resource(RLED, 3000, None, &sheet(1, 3))
        .resource(StrList::TYPE, 3000, None, &strings(&comm_strings()))
        .resource(StrList::TYPE, 2002, None, &strings(&messages))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &status_bar(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &pict(194, 16, [66, 66, 66]));
    if let Some(flags) = knobs.offer_movie {
        fork = fork.resource(
            Desc::TYPE,
            4000,
            None,
            &filmed(knobs.offer_text, "Offer.mov", flags),
        );
    } else if !knobs.offer_text.is_empty() {
        fork = fork.resource(Desc::TYPE, 4000, None, &desc(knobs.offer_text));
    }
    for id in [8511, 5001, 5002] {
        fork = fork.resource(PICT, id, None, &pict(6, 5, [40, 40, 40]));
    }
    // The offer dialog's frame.
    for (id, height) in [(8521, 9), (8522, 4), (8523, 40)] {
        fork = fork.resource(PICT, id, None, &pict(441, height, [30, 30, 60]));
    }
    // The Desc Dialog's frame.
    for (id, height) in [(8524, 9), (8525, 4), (8526, 40)] {
        fork = fork.resource(PICT, id, None, &pict(441, height, [30, 30, 60]));
    }
    for state in [7500, 7503, 7506] {
        fork = fork
            .resource(PICT, state, None, &pict(13, 25, [200, 0, 0]))
            .resource(PICT, state + 1, None, &pict(2, 25, [0, 200, 0]))
            .resource(PICT, state + 2, None, &pict(13, 25, [0, 0, 200]))
            .resource(PICT, state + 100, None, &pict(13, 25, [0, 0, 0]))
            .resource(PICT, state + 102, None, &pict(13, 25, [0, 0, 0]));
    }
    let file = OneFile(bracket_art::with_bracket_art(fork).build().bytes);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

/// One `DITL` user item at (left, top, width, height), enabled or not.
fn user_item((l, t, w, h): (i16, i16, i16, i16), enabled: bool) -> Vec<u8> {
    let mut bytes = vec![0; 4];
    for value in [t, l, t + h, l + w] {
        bytes.extend(value.to_be_bytes());
    }
    bytes.push(if enabled { 0 } else { 0x80 });
    bytes.push(0);
    bytes
}

/// A `DITL` user item: its (left, top, width, height), and whether it is
/// enabled.
type Item = ((i16, i16, i16, i16), bool);

/// A `DLOG` of `id`, `width` x `height`, and its `DITL` of `items`, each
/// a user item.
fn dialog(id: i16, (width, height): (i16, i16), items: &[Item]) -> (Vec<u8>, Vec<u8>) {
    let mut dlog: Vec<u8> = [40_i16, 40, 40 + height, 40 + width, 1]
        .iter()
        .flat_map(|v| v.to_be_bytes())
        .collect();
    dlog.extend([1, 0, 0, 0, 0, 0, 0, 0]);
    dlog.extend(id.to_be_bytes());
    dlog.extend([0, 0, 0xA8, 0x0A]);
    let mut list = (i16::try_from(items.len()).expect("few") - 1)
        .to_be_bytes()
        .to_vec();
    for &(bounds, enabled) in items {
        list.extend(user_item(bounds, enabled));
    }
    (dlog, list)
}

/// The interface file: the stock comm dialog (`DLOG` 1007, 423 x 215),
/// an offer dialog (`DLOG` 1016, 441 x 317) of the stock's ten items,
/// and "Desc Dialog" (`DLOG` 3003, 441 x 313) of the stock's six.
fn interface() -> InterfaceData {
    let items = [
        ((21, 181, 166, 26), true),
        ((21, 153, 166, 26), true),
        ((21, 125, 166, 26), true),
        ((46, 241, 200, 25), true),
        ((7, 320, 200, 25), true),
        ((199, 335, 200, 25), true),
        ((178, 261, 200, 25), true),
        ((178, 289, 200, 25), true),
        ((34, 299, 112, 16), false),
        ((11, 8, 192, 58), false),
        ((216, 7, 200, 200), false),
        ((40, 73, 134, 46), false),
    ];
    let (dlog, list) = dialog(1007, (423, 215), &items);
    let offer_items = [
        ((330, 275, 100, 20), false),
        ((220, 275, 100, 20), false),
        ((15, 15, 410, 245), false),
        ((450, 15, 100, 20), false),
        ((450, 45, 100, 20), false),
        ((330, 275, 100, 20), false),
        ((450, 75, 100, 20), false),
        ((10, 275, 100, 20), false),
        ((425, 15, 10, 10), false),
        ((425, 245, 10, 10), false),
    ];
    let (offer_dlog, offer_list) = dialog(1016, (441, 317), &offer_items);
    let text_items = [
        ((173, 281, 99, 25), true),
        ((221, 329, 68, 30), false),
        ((11, 10, 417, 262), false),
        ((70, 345, 78, 25), true),
        ((330, 282, 23, 23), false),
        ((363, 282, 23, 23), false),
    ];
    let (text_dlog, text_list) = dialog(3003, (441, 313), &text_items);
    let fork = ForkBuilder::new()
        .resource(Dlog::TYPE, 1007, None, &dlog)
        .resource(Ditl::TYPE, 1007, None, &list)
        .resource(Dlog::TYPE, 1016, None, &offer_dlog)
        .resource(Ditl::TYPE, 1016, None, &offer_list)
        .resource(Dlog::TYPE, 3003, None, &text_dlog)
        .resource(Ditl::TYPE, 3003, None, &text_list)
        .build()
        .bytes;
    InterfaceData::load(&OneFile(fork), Path::new("/Nova-DF.rsrc")).expect("loads")
}

/// Draws its script, then the last outcome, so no roll fires.
struct Script(VecDeque<u32>);

impl Chance for Script {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0.pop_front().unwrap_or(n - 1)
    }
}

/// The draws listing Merchant and placing it 300 above the player,
/// facing up, then listing Wreck and placing it on the player, facing
/// up.
const BOTH: [u32; 8] = [99, 750, 450, 0, 99, 750, 750, 0];

/// Every NPC idles.
#[derive(Debug)]
struct Still;

impl Behaviour for Still {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }
}

struct Game {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
}

impl Game {
    /// The app over the game data with "Pilot" saved in `store`, opened
    /// from the main menu to `showing`, its traffic drawn from `draws`,
    /// every NPC idling, the rules as the settings file holding
    /// `settings` chooses; then two frames, the second setting the system
    /// up in flight.
    fn opened(store: &MemoryPilots, draws: &[u32], showing: Showing, settings: &str) -> Self {
        Self::opened_with(PLAIN, store, draws, showing, settings)
    }

    /// [`Game::opened`] over the game data with `knobs` set.
    fn opened_with(
        knobs: Knobs,
        store: &MemoryPilots,
        draws: &[u32],
        showing: Showing,
        settings: &str,
    ) -> Self {
        Self::opened_playing(knobs, store, draws, showing, settings, None)
    }

    /// [`Game::opened_with`], playing movies through `player` when given.
    fn opened_playing(
        knobs: Knobs,
        store: &MemoryPilots,
        draws: &[u32],
        showing: Showing,
        settings: &str,
        player: Option<Rc<dyn MoviePlayer>>,
    ) -> Self {
        let data = game_data(knobs);
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        if store.text("Pilot").is_none() {
            let pilot = Pilot::new(data.as_ref(), "Pilot").expect("a pilot");
            keeper.save(&pilot).expect("saved");
        }
        // The load's heading first: 0, facing up; then the offers' roll
        // table, each the last outcome.
        let heading = std::iter::once(0);
        let table = std::iter::repeat_n(99, nova_sim::mission::ROLLED);
        let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Script(
            heading.chain(table).chain(draws.iter().copied()).collect(),
        )));
        let rulebook = saved_rulebook(settings);
        let screen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), Rc::new(MonoMetrics))
            .with_dialogs(Rc::new(interface()), Rc::new(MonoMetrics))
            .with_chance(SharedChance::new(chance))
            .with_rulebook(&rulebook)
            .with_behaviour(Rc::new(Still));
        let screen = match player {
            Some(player) => screen.with_movie_player(player),
            None => screen,
        };
        let mut game = Self {
            app: App::new(&FakeWindow, data, crate::common::appeared(screen)),
            gpu: RecordingGpu::new(),
            frames: 0,
        };
        let at = game
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .press_point(MenuChoice::OpenPilot);
        game.click(at);
        assert_eq!(game.showing(), Showing::OpenPilot);
        game.tap(Key::Enter);
        game.enter_ship();
        assert_eq!(game.showing(), showing);
        game.frame();
        game.frame();
        game
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue,
            "{event:?}"
        );
    }

    fn key(&mut self, key: Key, pressed: bool) {
        self.send(WindowEvent::Key {
            key,
            pressed,
            repeat: false,
        });
    }

    fn tap(&mut self, key: Key) {
        self.key(key, true);
        self.key(key, false);
    }

    fn click(&mut self, at: Point) {
        self.send(WindowEvent::PointerMoved {
            px: (f64::from(at.x), f64::from(at.y)),
        });
        for pressed in [true, false] {
            self.send(WindowEvent::PointerButton {
                button: nova_view::MouseButton::Left,
                pressed,
            });
        }
    }

    fn frame(&mut self) -> Frame {
        self.frames += 1;
        let elapsed = Duration::from_nanos(self.frames * 1_000_000_000 / 60);
        self.send(WindowEvent::Redraw { elapsed });
        assert_eq!(self.app.take_failures(), []);
        (*self.gpu.submits().last().expect("a frame")).clone()
    }

    /// Sends four redraws, at least a tick of flight, for the dock
    /// approach to clear a request. The nav brackets' corners (`cicn`
    /// 10000-10003), which the press's selection draws, may be missing
    /// from the fixture, so the frames' image failures are not checked.
    fn approach(&mut self) {
        for _ in 0..4 {
            self.frames += 1;
            let elapsed = Duration::from_nanos(self.frames * 1_000_000_000 / 60);
            self.send(WindowEvent::Redraw { elapsed });
        }
        let _ = self.app.take_failures();
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn session(&self) -> &Session {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .session()
            .expect("a session")
    }

    /// The person NPC `id` targeted: Tab until it is.
    fn target_person(&mut self, id: i16) {
        for _ in 0..4 {
            self.tap(Key::Tab);
            let targeted = self
                .session()
                .target()
                .and_then(|npc| npc.person)
                .map(|person| person.id.0);
            if targeted == Some(id) {
                return;
            }
        }
        panic!("never targeted {id}");
    }

    /// The comm dialog's options and reply, as the flight gives them,
    /// asked through the view.
    fn hailing(&mut self) -> (Vec<String>, String) {
        let view = self
            .app
            .screen_mut()
            .flight_view_mut()
            .expect("flying")
            .hailing()
            .expect("hailing");
        let labels = view
            .options
            .into_iter()
            .map(|button| button.label)
            .collect();
        (labels, view.reply)
    }

    /// The message line, as the flight gives it.
    fn message(&self) -> Option<String> {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .message()
            .map(str::to_owned)
    }
}

fn texts(frame: &Frame) -> Vec<String> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Text(runs) => runs.iter().map(|run| run.text.clone()).collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// The rulebook the settings file holding `text` chooses, read as `main`
/// reads it.
fn saved_rulebook(text: &str) -> nova_sim::Rulebook {
    let home = tempfile::tempdir().expect("a temporary directory");
    let path = home.path().join("settings.json");
    std::fs::write(&path, text).expect("writes");
    let mut store: Option<Box<dyn nova_audio::SettingsStore>> =
        Some(Box::new(nova_audio::FileSettings::new(&path)));
    let (rulebook, warnings) = nova::rulebook::game_rulebook(store.as_deref_mut());
    assert_eq!(warnings, Vec::<String>::new(), "{text}");
    rulebook
}

/// The settings choosing `person_join`'s other reading.
const JOINING: &str = r#"{"rule_overrides": {"person_join": "bible"}}"#;

impl Game {
    /// The mission the offer dialog offers, while it is open.
    fn offered(&self) -> Option<i16> {
        let dialog = self.app.screen().offer()?;
        Some(dialog.prompt().mission.0)
    }

    /// The mission text shown over flight, its lines joined by spaces.
    fn flight_text(&self) -> Option<String> {
        self.app
            .screen()
            .flight_mission_text()
            .map(|dialog| dialog.lines().join(" "))
    }

    /// How far flight's display is between its last two steps: it moves
    /// on each frame flight ticks, and holds while flight is paused.
    fn alpha(&self) -> f32 {
        self.app.screen().flight_view().expect("flying").alpha()
    }

    /// The missions the pilot has accepted, in slot order.
    fn active(&self) -> Vec<i16> {
        self.session()
            .pilot()
            .missions()
            .missions()
            .iter()
            .map(|mission| mission.0)
            .collect()
    }
}

#[test]
fn by_the_engines_default_merchant_offers_its_mission_in_place_of_the_comm_dialog() {
    let store = MemoryPilots::new();
    let mut game = Game::opened(&store, &BOTH, Showing::Flight, "{}");
    game.target_person(128);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Offer, "over the paused flight");
    assert_eq!(game.offered(), Some(128));
    let dialog = game.app.screen().offer().expect("open");
    assert_eq!(dialog.problem(), None, "laid out by the interface file");
    let shown = texts(&game.frame());
    assert!(shown.iter().any(|text| text == "Yes"), "{shown:?}");
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight, "flight resumes");
    assert_eq!(game.active(), [128]);
}

#[test]
fn p_on_merchants_offer_opens_the_player_info_over_it() {
    let store = MemoryPilots::new();
    let mut game = Game::opened(&store, &BOTH, Showing::Flight, "{}");
    game.target_person(128);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Offer);
    game.tap(Key::Char('p'));
    assert_eq!(game.showing(), Showing::PlayerInfo, "over the offer");
    let open = game.app.screen().flight_player_info().expect("open");
    assert_eq!(
        open.problem(),
        Some("no DLOG 1017"),
        "the interface file has no player info dialog: it says why"
    );
    assert_eq!(game.offered(), Some(128), "the offer stays");
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Offer, "back to the offer");
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.active(), [128]);
}

#[test]
fn n_refuses_merchants_offer_and_flight_resumes() {
    let store = MemoryPilots::new();
    let mut game = Game::opened(&store, &BOTH, Showing::Flight, "{}");
    game.target_person(128);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Offer);
    game.tap(Key::Char('n'));
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.active(), Vec::<i16>::new());
    game.tap(Key::Char('y'));
    assert_eq!(game.offered(), Some(128), "offered again");
}

#[test]
fn an_unrefusable_mission_with_no_offer_text_is_accepted_at_once_showing_nothing() {
    let store = MemoryPilots::new();
    let knobs = Knobs {
        mission_flags: 0x0004,
        ..PLAIN
    };
    let mut game = Game::opened_with(knobs, &store, &BOTH, Showing::Flight, "{}");
    game.target_person(128);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.active(), [128]);
}

/// Merchant's offer answered with `answer`, then its text shown over the
/// paused flight until Done: `shown`.
fn answered_over_flight(answer: Key, shown: &str) -> Game {
    let store = MemoryPilots::new();
    let mut game = Game::opened_with(TEXTED, &store, &BOTH, Showing::Flight, "{}");
    game.target_person(128);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Offer);
    game.tap(answer);
    assert_eq!(game.showing(), Showing::MissionText, "over flight");
    assert_eq!(game.flight_text().as_deref(), Some(shown));
    let alpha = game.alpha();
    for _ in 0..2 {
        let frame = game.frame();
        assert!(texts(&frame).contains(&shown.to_owned()), "drawn");
        assert_eq!(game.showing(), Showing::MissionText);
        assert_eq!(game.alpha().to_bits(), alpha.to_bits(), "flight is paused");
    }
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight, "Done goes back to flight");
    assert_eq!(game.flight_text(), None);
    game
}

#[test]
fn merchants_offer_accepted_shows_its_brief_text_over_the_paused_flight() {
    let game = answered_over_flight(Key::Enter, "Stay close.");
    assert_eq!(game.active(), [128]);
}

#[test]
fn merchants_offer_refused_shows_its_refuse_text_over_the_paused_flight() {
    let game = answered_over_flight(Key::Char('n'), "Your loss.");
    assert_eq!(game.active(), Vec::<i16>::new());
}

#[test]
fn a_hail_accepting_at_once_lets_go_of_flights_keys_under_the_brief_text() {
    let store = MemoryPilots::new();
    let knobs = Knobs {
        mission_flags: 0x0004,
        ..TEXTED
    };
    let mut game = Game::opened_with(knobs, &store, &BOTH, Showing::Flight, "{}");
    game.target_person(128);

    // Up held, Y hails Merchant, whose mission, unrefusable and with no
    // offer text, is accepted at once: its BriefText shows over flight,
    // and Up is let go under it.
    game.key(Key::Up, true);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::MissionText);
    assert_eq!(game.flight_text().as_deref(), Some("Stay close."));
    game.key(Key::Up, false);
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);

    // The ship coasts: it no longer speeds up.
    game.frame();
    let speed = game.session().player().velocity.length();
    for _ in 0..30 {
        game.frame();
    }
    assert_eq!(
        game.session().player().velocity.length().to_bits(),
        speed.to_bits(),
        "Up is no longer held"
    );
}

#[test]
fn boarding_wreck_of_0x0200_offers_its_mission_in_place_of_the_plunder() {
    let store = MemoryPilots::new();
    let knobs = Knobs {
        wreck_flags: 0x0200,
        wreck_mission: 128,
        ..PLAIN
    };
    let mut game = Game::opened_with(knobs, &store, &BOTH, Showing::Flight, "{}");
    game.target_person(129);
    game.tap(Key::Char('b'));
    assert_eq!(game.showing(), Showing::Offer);
    assert_eq!(game.offered(), Some(128));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.active(), [128]);
    assert_eq!(game.session().pilot().owned(OutfitId(128)), 0, "no grant");
}

#[test]
fn b_held_as_the_wreck_of_0x0200_is_targeted_offers_its_mission_on_the_next_frame() {
    // `_HandlePlayer` boards every frame B is down until an attempt gets
    // past the no-target return (@0x6a98b, @0x65031).
    let store = MemoryPilots::new();
    let knobs = Knobs {
        wreck_flags: 0x0200,
        wreck_mission: 128,
        ..PLAIN
    };
    let mut game = Game::opened_with(knobs, &store, &BOTH, Showing::Flight, "{}");
    game.key(Key::Char('b'), true);
    game.target_person(129);
    assert_eq!(game.showing(), Showing::Flight);
    game.frame();
    assert_eq!(game.showing(), Showing::Offer);
    assert_eq!(game.offered(), Some(128));
}

#[test]
fn by_the_settings_other_reading_merchant_joins_and_flies_again_after_a_save() {
    let store = MemoryPilots::new();
    let mut game = Game::opened(&store, &BOTH, Showing::Flight, JOINING);
    game.target_person(128);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Comm);
    let (options, _) = game.hailing();
    assert_eq!(
        options,
        ["Greetings", "Request Assistance", "Use As Escort"]
    );
    let shown = texts(&game.frame());
    assert!(
        shown.iter().any(|text| text == "Use As Escort"),
        "{shown:?}"
    );
    game.tap(Key::Char('u'));
    let (options, reply) = game.hailing();
    assert_eq!(reply, "Okay, I'm on my way.");
    assert_eq!(options, ["Greetings", "Release"]);
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.session().pilot().escorts().len(), 1);
    game.frame();
    // L requests clearance, a second L lands.
    game.tap(Key::Char('l'));
    game.approach();
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Spaceport);
    let saved: serde_json::Value =
        serde_json::from_str(&store.text("Pilot").expect("saved")).expect("JSON");
    assert_eq!(saved["escorts"][0]["person"], 128);
    // Reopened in a new app, the pilot resumes in flight: Merchant's slot
    // is passed over, Wreck's draws, and Merchant flies beside the player.
    let mut game = Game::opened(&store, &[99, 750, 450, 0], Showing::Flight, JOINING);
    game.frame();
    game.frame();
    let escort = game
        .session()
        .npcs()
        .iter()
        .find(|npc| npc.escort.is_some())
        .expect("placed");
    assert_eq!(escort.person.map(|person| person.id.0), Some(128));
    assert_eq!(game.session().npc_name(escort), Some("Merchant"));
    game.key(Key::Alt, true);
    game.tap(Key::Tab);
    game.key(Key::Alt, false);
    let shown = texts(&game.frame());
    assert!(shown.iter().any(|text| text == "Merchant"), "{shown:?}");
}

/// Boards Wreck, the draws after the placement `draws`: the message line
/// and how many spare parts the pilot owns.
fn boarding_wreck(draws: &[u32]) -> (Option<String>, u16) {
    let store = MemoryPilots::new();
    let script: Vec<u32> = BOTH.iter().chain(draws).copied().collect();
    let mut game = Game::opened(&store, &script, Showing::Flight, "{}");
    game.target_person(129);
    game.tap(Key::Char('b'));
    assert_eq!(game.showing(), Showing::Plunder);
    let owned = game.session().pilot().owned(OutfitId(128));
    (game.message(), owned)
}

#[test]
fn boarding_wreck_says_what_it_retrieved_and_adds_it() {
    assert_eq!(
        boarding_wreck(&[]),
        (
            Some("You retrieved two spare parts from this ship.".to_owned()),
            2
        )
    );
    // The threshold, the energy, the jitter, the odds (1 of 100), then a
    // count of trunc(50 x 2 / 100) = 1.
    assert_eq!(
        boarding_wreck(&[0, 0, 0, 0, 0]),
        (
            Some("You retrieved a spare part from this ship.".to_owned()),
            1
        )
    );
}

// A dësc's movie around a person's offer and its texts.

/// A movie player whose movies, 160 x 120, end on their third advance;
/// it logs each start and stop.
#[derive(Default)]
struct Projector {
    log: RefCell<Vec<String>>,
    advanced: std::cell::Cell<u32>,
}

impl MoviePlayer for Projector {
    fn start(&self, file: &str) -> Option<(u32, u32)> {
        self.log.borrow_mut().push(format!("start {file}"));
        self.advanced.set(0);
        Some((160, 120))
    }

    fn advance(&self, _dt: Duration) -> bool {
        self.advanced.set(self.advanced.get() + 1);
        self.advanced.get() < 3
    }

    fn stop(&self) {
        self.log.borrow_mut().push("stop".to_owned());
    }

    fn draw_frame(&self, _rect: nova_view::geometry::Bounds, _list: &mut nova_view::DrawList) {}
}

/// Merchant hailed, with `knobs` and movies played through a new
/// [`Projector`].
fn hailed_with_movies(knobs: Knobs) -> (Game, Rc<Projector>) {
    let store = MemoryPilots::new();
    let projector = Rc::new(Projector::default());
    let player: Rc<dyn MoviePlayer> = Rc::clone(&projector) as Rc<dyn MoviePlayer>;
    let mut game = Game::opened_playing(knobs, &store, &BOTH, Showing::Flight, "{}", Some(player));
    game.target_person(128);
    game.tap(Key::Char('y'));
    (game, projector)
}

fn log(projector: &Projector) -> Vec<String> {
    projector.log.borrow().clone()
}

impl Game {
    /// Clicks Done on the mission text shown over flight.
    fn click_done(&mut self) {
        let done = self
            .app
            .screen()
            .flight_mission_text()
            .and_then(|text| text.dialog().item_bounds(nova_view::ui::desc::DONE_ITEM))
            .expect("Done")
            .center();
        self.click(done);
    }
}

#[test]
fn a_persons_offer_over_flight_plays_its_movie_before_the_dialog() {
    let (mut game, projector) = hailed_with_movies(Knobs {
        offer_movie: Some(0),
        ..PLAIN
    });
    assert_eq!(log(&projector), ["start Offer.mov"]);
    assert_eq!(game.showing(), Showing::Movie, "over the paused flight");
    let alpha = game.alpha();
    let shown = texts(&game.frame());
    assert!(
        !shown.iter().any(|text| text == "Yes"),
        "no dialog: {shown:?}"
    );
    game.frame();
    assert_eq!(game.alpha().to_bits(), alpha.to_bits(), "flight is paused");
    game.frame();
    assert_eq!(log(&projector), ["start Offer.mov", "stop"]);
    assert_eq!(game.showing(), Showing::Offer, "then the dialog");
    assert!(texts(&game.frame()).iter().any(|text| text == "Yes"));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.active(), [128]);
}

#[test]
fn n_pressed_under_a_persons_offer_movie_refuses_it_on_the_frame_the_movie_ends() {
    let (mut game, projector) = hailed_with_movies(Knobs {
        offer_movie: Some(0),
        ..TEXTED
    });
    game.tap(Key::Char('n'));
    assert_eq!(game.showing(), Showing::Movie);
    game.frame();
    game.frame();
    assert_eq!(game.showing(), Showing::Movie);
    game.frame();
    assert_eq!(log(&projector), ["start Offer.mov", "stop"]);
    assert_eq!(
        game.showing(),
        Showing::MissionText,
        "refused with no input after the movie: its refuse text shows"
    );
    assert_eq!(game.flight_text().as_deref(), Some("Your loss."));
    assert_eq!(game.active(), Vec::<i16>::new());
}

#[test]
fn a_persons_offer_over_flight_plays_its_after_movie_after_the_brief() {
    let (mut game, projector) = hailed_with_movies(Knobs {
        offer_movie: Some(0x0001),
        ..TEXTED
    });
    assert_eq!(game.showing(), Showing::Offer, "the dialog first");
    assert!(log(&projector).is_empty());
    game.tap(Key::Enter);
    assert_eq!(game.active(), [128], "accepted at once");
    assert_eq!(game.showing(), Showing::MissionText);
    assert_eq!(game.flight_text().as_deref(), Some("Stay close."));
    assert!(log(&projector).is_empty(), "not before the brief");
    game.click_done();
    assert_eq!(log(&projector), ["start Offer.mov"]);
    assert_eq!(game.showing(), Showing::Movie);
    let alpha = game.alpha();
    game.frame();
    game.frame();
    assert_eq!(game.alpha().to_bits(), alpha.to_bits(), "flight is paused");
    game.frame();
    assert_eq!(
        game.showing(),
        Showing::Flight,
        "flight resumes on the frame the movie ends"
    );
}

#[test]
fn a_persons_offer_refused_without_a_text_stays_under_its_after_movie_over_flight() {
    let (mut game, projector) = hailed_with_movies(Knobs {
        offer_movie: Some(0x0001),
        ..PLAIN
    });
    assert_eq!(game.showing(), Showing::Offer);
    game.tap(Key::Char('n'));
    assert_eq!(log(&projector), ["start Offer.mov"]);
    assert_eq!(game.showing(), Showing::Movie);
    assert!(
        texts(&game.frame()).iter().any(|text| text == "Yes"),
        "not hidden with no RefuseText (@0xa2e70-0xa2e78)"
    );
    game.frame();
    game.frame();
    assert_eq!(
        log(&projector),
        ["start Offer.mov", "stop"],
        "the movie runs on"
    );
    assert_eq!(game.showing(), Showing::Flight);
    assert!(
        !texts(&game.frame()).iter().any(|text| text == "Yes"),
        "disposed once it closed (@0xa361e)"
    );
}

#[test]
fn a_persons_unrefusable_offer_accepted_stays_drawn_under_its_brief_over_flight() {
    let store = MemoryPilots::new();
    let knobs = Knobs {
        mission_flags: 0x0004,
        offer_text: "Come along.",
        ..TEXTED
    };
    let mut game = Game::opened_with(knobs, &store, &BOTH, Showing::Flight, "{}");
    game.target_person(128);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Offer, "one button, with a text");
    let shown = texts(&game.frame());
    assert!(shown.iter().any(|text| text == "Okay"), "{shown:?}");
    assert!(!shown.iter().any(|text| text == "No"), "{shown:?}");
    game.tap(Key::Enter);
    assert_eq!(game.active(), [128]);
    assert_eq!(game.showing(), Showing::MissionText);
    assert_eq!(game.flight_text().as_deref(), Some("Stay close."));
    let shown = texts(&game.frame());
    assert!(
        shown.iter().any(|text| text == "Okay"),
        "the one-button dialog's accept leaves it drawn (@0xa2f52-0xa2ff7): {shown:?}"
    );
    assert!(shown.iter().any(|text| text == "Stay close."), "{shown:?}");
    game.tap(Key::Enter);
    assert_eq!(
        game.flight_text(),
        None,
        "Return reaches the brief over the answered offer"
    );
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.offered(), None, "disposed with it (@0xa361e)");
    let shown = texts(&game.frame());
    assert!(!shown.iter().any(|text| text == "Okay"), "{shown:?}");
}

#[test]
fn a_persons_offer_refused_with_a_text_is_hidden_under_it() {
    let (mut game, _) = hailed_with_movies(TEXTED);
    game.tap(Key::Char('n'));
    assert_eq!(game.flight_text().as_deref(), Some("Your loss."));
    assert!(
        !texts(&game.frame()).iter().any(|text| text == "Yes"),
        "hidden for its RefuseText (@0xa2e8e)"
    );
}

#[test]
fn an_offer_accepted_at_once_over_flight_plays_its_movie_before_its_brief() {
    let (mut game, projector) = hailed_with_movies(Knobs {
        mission_flags: 0x0004,
        offer_movie: Some(0x0004),
        brief_movie: Some(0x0001),
        ..TEXTED
    });
    assert_eq!(game.active(), [128]);
    assert_eq!(game.showing(), Showing::Movie, "the offer's movie first");
    assert_eq!(log(&projector), ["start Offer.mov"]);
    for _ in 0..(2 * 60 + 3 + 2 * 60 + 2) {
        game.frame();
    }
    assert_eq!(game.showing(), Showing::MissionText, "then the brief");
    assert_eq!(game.flight_text().as_deref(), Some("Stay close."));
    game.click_done();
    assert_eq!(
        log(&projector),
        ["start Offer.mov", "stop", "start Brief.mov"],
        "the brief's own after-movie"
    );
}

#[test]
fn the_return_that_closes_the_brief_skips_the_after_movie_as_it_is_let_go() {
    let (mut game, projector) = hailed_with_movies(Knobs {
        offer_movie: Some(0x0001),
        ..TEXTED
    });
    game.tap(Key::Enter);
    game.key(Key::Enter, true);
    assert_eq!(log(&projector), ["start Offer.mov"]);
    assert_eq!(game.showing(), Showing::Movie);
    game.key(Key::Enter, false);
    assert_eq!(
        log(&projector),
        ["start Offer.mov", "stop"],
        "Return still down as the movie starts skips it (KeyCheck 0x24 @0x55e6c)"
    );
    assert_eq!(game.showing(), Showing::Flight);
}

#[test]
fn with_the_engines_player_a_texts_movie_is_skipped_and_its_text_shows() {
    let store = MemoryPilots::new();
    let knobs = Knobs {
        offer_movie: Some(0),
        brief_movie: Some(0),
        ..TEXTED
    };
    let mut game = Game::opened_with(knobs, &store, &BOTH, Showing::Flight, "{}");
    game.target_person(128);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Offer, "no movie plays");
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::MissionText);
    assert_eq!(game.flight_text().as_deref(), Some("Stay close."));
}

#[test]
fn a_cinematic_offer_movie_over_flight_hides_the_pointer() {
    use nova_view::Screen;
    let (mut game, _) = hailed_with_movies(Knobs {
        offer_movie: Some(0x0004),
        ..PLAIN
    });
    assert!(!game.app.screen().hides_pointer(), "during the fade out");
    for _ in 0..61 {
        game.frame();
    }
    assert_eq!(game.showing(), Showing::Movie);
    assert!(game.app.screen().hides_pointer(), "_HideTheCursor @0x55a73");
}

crate::common::entry_harness!(Game, click, frame);
