//! Missions through the whole app: synthetic game data (a `chär` with
//! 25,000 credits flying the "Boarder" from Alpha, whose one stellar, the
//! "Pad" at the centre, has a bar; mission 128, "Run to <DST>", offered
//! at the Pad's mission computer, going to the Pad and back within 7
//! days, with an offer text, a `BriefText`, a `QuickBrief` and a
//! `CompText`; mission 129, "Errand", going nowhere, which cannot be
//! aborted, with no text; and at the bar, mission 130, "Bar job",
//! labelled "I'm in", with an offer text and a `BriefText`, and mission
//! 131, "Second", with an offer text, a `RefuseText` and an `OnRefuse`
//! setting bit 7) and a synthetic interface file holding the "Spaceport",
//! the mission computer (`DLOG` 1006), the mission info screen (1012),
//! "Bar" (1013), the offer dialog (1016) and "Desc Dialog" (3003), driven
//! only by window events, with pilots kept in the in-memory store.
//!
//! The mission computer: the pilot lands, and Mission BBS lists "Run to
//! Pad" and "Errand", the first one's text filled. Accept shows its
//! `BriefText`, filled, over the computer, which lists the Errand alone;
//! accepting that too closes the computer silently, nothing being left.
//! Both are active and saved. Taking off and landing again completes the
//! run, at its destination, and its `CompText` shows over the spaceport.
//!
//! The bar: a quarter second after the bar opens, the offer dialog shows
//! the bar job's text, filled, and "I'm in". Return accepts it, and its
//! `BriefText` shows. After the gap, the second is offered, and N refuses
//! it: its `RefuseText` shows, and its `OnRefuse` set bit 7, which the
//! saved pilot carries. Nothing more is offered.
//!
//! A full pilot at the bar: with all 16 slots taken by errands, accepting
//! the bar job is refused, and the second is offered in its place;
//! accepting that is refused too, and nothing more is offered. Leaving
//! the bar for the spaceport and coming back offers nothing, as nothing
//! was offered at in between; taking off, landing again and opening the
//! bar offers the bar job again, the spaceport offered at as it landed.
//!
//! The spaceport: with the bar job offered at the spaceport (`AvailLoc`
//! 3), landing shows it in the offer dialog over the spaceport; Return
//! accepts it, and its `BriefText` shows.
//!
//! An offer that cannot be refused: with the bar job unrefusable
//! (`Flags` 0x0004) and its offer text empty, opening the bar accepts it
//! with no offer dialog, and its `BriefText` shows. After the gap, the
//! second is offered.
//!
//! Mission info: with the run and the errand accepted, I in the spaceport
//! lists both, the run's `QuickBrief` showing its deadline. Abort does
//! nothing on the errand; on the run it frees its slot, which the saved
//! pilot shows, and the screen lists the errand alone.
//!
//! A deadline missed in flight: with the run's deadline due and its
//! `OnFailure` `S130`, leaving the spaceport takes off, the run fails,
//! the message line says "Time limit exceeded - mission failed.", and
//! the bar job its `S` accepted shows its `BriefText` over the paused
//! flight until Done.
//!
//! I in flight: with no mission, the message line says "You have no
//! active missions."; with the run and the errand held, it opens the
//! mission info screen over the paused flight, Abort there frees the
//! run's slot, and Done goes back into flight.

use std::cell::RefCell;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, AppScreen, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::desc::Desc;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::interface::Interface;
use nova_data::records::mission::Mission;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, InterfaceData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};
use nova_sim::fixture::MemoryPilots;
use nova_sim::mission::SLOTS;
use nova_sim::{Chance, MissionId, Pilot, PilotKeeper, PilotStore, Session};
use nova_view::Key;
use nova_view::flight::SharedChance;
use nova_view::geometry::Point;
use nova_view::menu::MenuChoice;
use nova_view::spaceport::missions::{
    ABORT_ITEM, ACCEPT_ITEM, INFO_TEXT_ITEM, MissionComputerScreen, MissionInfoScreen,
    TEXT_ITEM as COMPUTER_TEXT_ITEM,
};
use nova_view::spaceport::{OfferDialog, SpaceportView};
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

/// The `chär`: 25,000 credits, ship 128 in system 128, no legal records.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    bytes[0x00..0x04].copy_from_slice(&25_000_i32.to_be_bytes());
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    bytes
}

/// A `shïp` of 30 shield and 45 armour, slow and steady, unarmed.
fn ship() -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[30, 300, 300, 30, 300]);
    put_i16s(&mut bytes, 0x0E, &[45]);
    put_i16s(&mut bytes, 0x12, &[-1; 4]);
    put_i16s(&mut bytes, 0x1A, &[1; 4]);
    put_i16s(&mut bytes, 0x2E, &[1]);
    put_i16s(&mut bytes, 0x38, &[-1, 0]);
    put_i16s(&mut bytes, 0x42, &[1]);
    put_i16s(&mut bytes, 0x4E, &[-1; 4]);
    put_i16s(&mut bytes, 0x370, &[-1; 4]);
    put_i16s(&mut bytes, 0x6CE, &[-1; 4]);
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

/// Alpha: a `sÿst` at the map's centre listing the Pad; independent,
/// with no traffic.
fn system() -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[128]);
    put_i16s(&mut bytes, 0x44, &[-1; 8]);
    put_i16s(&mut bytes, 0x64, &[0, -1]);
    bytes
}

/// The Pad: a `spöb` at the centre that can be landed on, with a bar
/// (`Flags` 0x41), drawn from `spïn` 1004.
fn pad() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 4]);
    bytes[0x06..0x0A].copy_from_slice(&0x41_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x14, &[-1]);
    put_i16s(&mut bytes, 0x18, &[-1, -1]);
    bytes
}

/// A `mïsn` offered at the Pad's mission computer on every visit, going
/// to `travel` (and back there), paying 1,000, its `BriefText` and
/// `CompText` the `dësc`s `brief` and `comp`, and no other text.
fn mission(travel: i16, brief: i16, comp: i16) -> Vec<u8> {
    let mut bytes = vec![0; Mission::SIZE.expect("fixed")];
    put_i16s(
        &mut bytes,
        0x00,
        &[128, 0, 0, 0, 0, 100, travel, -1, -1, -1],
    );
    bytes[0x1C..0x20].copy_from_slice(&1000_i32.to_be_bytes());
    put_i16s(&mut bytes, 0x20, &[0, 0, -1, -1, 0, -1, 0, -1, 0]);
    put_i16s(
        &mut bytes,
        0x32,
        &[-1, brief, -1, -1, -1, comp, -1, 0, 1, -1],
    );
    put_i16s(&mut bytes, 0x58, &[-1]);
    bytes
}

/// `mission` with a `QuickBrief` of `dësc` 204 and a `TimeLimit` of 7
/// days.
fn timed(mut mission: Vec<u8>) -> Vec<u8> {
    put_i16s(&mut mission, 0x36, &[204]);
    put_i16s(&mut mission, 0x40, &[7]);
    mission
}

/// `mission` with a `CanAbort` of 0.
fn unabortable(mut mission: Vec<u8>) -> Vec<u8> {
    put_i16s(&mut mission, 0x42, &[0]);
    mission
}

/// [`mission`] going nowhere, offered at the Pad's bar, with these raw
/// labels, `RefuseText` and `OnRefuse`.
fn bar_mission(brief: i16, refuse: i16, accept_label: &[u8], on_refuse: &[u8]) -> Vec<u8> {
    let mut bytes = mission(-1, brief, -1);
    put_i16s(&mut bytes, 0x04, &[1]);
    put_i16s(&mut bytes, 0x58, &[refuse]);
    bytes[0x25A..0x25A + on_refuse.len()].copy_from_slice(on_refuse);
    bytes[0x75F..0x75F + accept_label.len()].copy_from_slice(accept_label);
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

/// The bar job, mission 130, as the module docs describe it, with its
/// offer text.
fn bar_job() -> (Vec<u8>, &'static str) {
    (bar_mission(202, -1, b"I'm in\0", b"\0"), "<PN>, join us.")
}

/// `mission` offered at venue `loc` (its `AvailLoc`).
fn at_venue(mut mission: Vec<u8>, loc: i16) -> Vec<u8> {
    put_i16s(&mut mission, 0x04, &[loc]);
    mission
}

/// `mission` with this raw `OnFailure`.
fn fails_into(mut mission: Vec<u8>, on_failure: &[u8]) -> Vec<u8> {
    mission[0x458..0x458 + on_failure.len()].copy_from_slice(on_failure);
    mission
}

/// `mission` that cannot be refused: `Flags` 0x0004.
fn unrefusable(mut mission: Vec<u8>) -> Vec<u8> {
    put_i16s(&mut mission, 0x50, &[0x0004]);
    mission
}

/// The game data (see the module docs) and the pictures the screens
/// draw.
fn game_data() -> Rc<GameData> {
    game_data_with(bar_job())
}

/// [`game_data`] with this bar job, mission 130, and its offer text.
fn game_data_with(bar_job: (Vec<u8>, &str)) -> Rc<GameData> {
    game_data_from(&timed(mission(128, 200, 201)), bar_job)
}

/// [`game_data`] with this run, mission 128, and this bar job, mission
/// 130, and its offer text.
fn game_data_from(run: &[u8], bar_job: (Vec<u8>, &str)) -> Rc<GameData> {
    game_data_built(run, bar_job, "Take this to <DST>.", |fork| fork)
}

/// [`game_data_from`] with the run's offer text `run_offer`, and `more`
/// resources added.
fn game_data_built(
    run: &[u8],
    (bar_job, bar_job_offer): (Vec<u8>, &str),
    run_offer: &str,
    more: impl FnOnce(ForkBuilder) -> ForkBuilder,
) -> Rc<GameData> {
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Boarder"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system())
        .resource(Stellar::TYPE, 128, Some(b"Pad"), &pad())
        .resource(Spin::TYPE, 1004, None, &spin(1000))
        .resource(RLED, 1000, None, &sheet(1, 40))
        .resource(Mission::TYPE, 128, Some(b"Run to <DST>;courier"), run)
        .resource(
            Mission::TYPE,
            129,
            Some(b"Errand"),
            &unabortable(mission(-1, -1, -1)),
        )
        .resource(Desc::TYPE, 204, None, &desc("Back to <DST> by <DL>."))
        .resource(Mission::TYPE, 130, Some(b"Bar job"), &bar_job)
        .resource(
            Mission::TYPE,
            131,
            Some(b"Second"),
            &bar_mission(-1, 203, b"\0", b"b7\0"),
        )
        .resource(Desc::TYPE, 4002, None, &desc(bar_job_offer))
        .resource(Desc::TYPE, 4003, None, &desc("One more job."))
        .resource(Desc::TYPE, 202, None, &desc("Welcome aboard."))
        .resource(Desc::TYPE, 203, None, &desc("Suit yourself."))
        .resource(Desc::TYPE, 10_000, None, &desc("Smoke and quiet talk."))
        .resource(Desc::TYPE, 4000, None, &desc(run_offer))
        .resource(Desc::TYPE, 200, None, &desc("Off you go to <DST>."))
        .resource(Desc::TYPE, 201, None, &desc("Thanks from <DST>."))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &status_bar(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &pict(194, 16, [66, 66, 66]));
    fork = more(fork);
    for id in [
        8500, 8501, 8503, 8505, 8517, 8521, 8522, 8523, 8524, 8525, 8526, 10_004,
    ] {
        fork = fork.resource(PICT, id, None, &pict(6, 5, [40, 40, 40]));
    }
    for state in [7500, 7503, 7506] {
        fork = fork
            .resource(PICT, state, None, &pict(13, 25, [200, 0, 0]))
            .resource(PICT, state + 1, None, &pict(2, 25, [0, 200, 0]))
            .resource(PICT, state + 2, None, &pict(13, 25, [0, 0, 200]))
            .resource(PICT, state + 100, None, &pict(13, 25, [0, 0, 0]))
            .resource(PICT, state + 102, None, &pict(13, 25, [0, 0, 0]));
    }
    let file = OneFile(fork.build().bytes);
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

/// A `DITL` user item: its place, (left, top, width, height), and whether
/// it is enabled.
type Item = ((i16, i16, i16, i16), bool);

/// A centred `DLOG` `id` of `width` x `height` naming `DITL` `id`, and
/// the `DITL` of `items`.
fn dialog(id: i16, (width, height): (i16, i16), items: &[Item]) -> [(ResType, i16, Vec<u8>); 2] {
    let mut dlog: Vec<u8> = [40, 40, 40 + height, 40 + width, 1]
        .iter()
        .flat_map(|v: &i16| v.to_be_bytes())
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
    [(Dlog::TYPE, id, dlog), (Ditl::TYPE, id, list)]
}

/// The mission computer (`DLOG` 1006), the mission info screen (1012),
/// "Bar" (1013), the offer dialog (1016) and "Desc Dialog" (3003), each
/// with its items as stock has them.
fn mission_dialogs() -> [[(ResType, i16, Vec<u8>); 2]; 5] {
    let computer = dialog(
        1006,
        (510, 201),
        &[
            ((266, 170, 99, 25), true),
            ((10, 30, 195, 144), true),
            ((205, 30, 15, 144), true),
            ((233, 60, 267, 93), false),
            ((233, 34, 269, 21), false),
            ((175, 246, 68, 30), false),
            ((368, 170, 99, 25), true),
            ((14, 3, 396, 15), false),
            ((285, 269, 32, 32), false),
            ((204, 297, 68, 30), false),
            ((311, 0, 87, 15), false),
        ],
    );
    let text = dialog(
        3003,
        (441, 313),
        &[
            ((173, 281, 99, 25), true),
            ((221, 329, 68, 30), false),
            ((11, 10, 417, 262), false),
            ((70, 345, 78, 25), true),
            ((330, 282, 23, 23), false),
            ((363, 282, 23, 23), false),
        ],
    );
    let bar = dialog(
        1013,
        (263, 185),
        &[
            ((156, 154, 99, 26), false),
            ((156, 125, 99, 26), false),
            ((6, 154, 146, 26), false),
            ((54, 214, 121, 25), false),
            ((6, 125, 146, 26), false),
            ((105, 227, 121, 25), false),
            ((16, 10, 230, 106), false),
            ((61, 303, 32, 32), false),
            ((103, 303, 32, 32), false),
            ((146, 303, 32, 32), false),
        ],
    );
    let offer = dialog(
        1016,
        (441, 317),
        &[
            ((225, 285, 99, 25), false),
            ((117, 285, 99, 25), false),
            ((12, 9, 415, 267), false),
            ((150, 323, 32, 32), false),
            ((197, 326, 32, 32), false),
            ((173, 285, 99, 25), false),
            ((172, 337, 32, 32), false),
            ((272, 345, 23, 23), false),
            ((340, 287, 23, 23), false),
            ((373, 287, 23, 23), false),
        ],
    );
    let info = dialog(
        1012,
        (471, 155),
        &[
            ((290, 125, 99, 25), true),
            ((9, 24, 195, 84), true),
            ((13, 1, 193, 12), true),
            ((218, 26, 242, 91), false),
            ((57, 125, 99, 25), true),
            ((177, 222, 55, 37), true),
            ((343, 4, 122, 12), false),
        ],
    );
    [computer, info, bar, offer, text]
}

/// The interface file: stock "Spaceport" (`DLOG` 1000), the mission
/// computer (1006), the mission info screen (1012), "Bar" (1013), the
/// offer dialog (1016) and "Desc Dialog" (3003), each with its items as
/// stock has them.
fn interface() -> InterfaceData {
    let spaceport = dialog(
        1000,
        (618, 517),
        &[
            ((242, 551, 200, 25), true),
            ((452, 549, 68, 30), false),
            ((159, 297, 303, 18), false),
            ((471, 416, 145, 25), true),
            ((3, 3, 612, 285), true),
            ((160, 327, 301, 185), false),
            ((3, 414, 145, 25), true),
            ((471, 375, 145, 25), true),
            ((471, 333, 145, 25), true),
            ((3, 374, 145, 25), true),
            ((3, 333, 145, 25), true),
            ((471, 456, 145, 25), true),
            ((3, 456, 145, 25), true),
            ((524, 550, 68, 30), false),
            ((605, 548, 32, 32), false),
        ],
    );
    let fork = std::iter::once(spaceport)
        .chain(mission_dialogs())
        .flatten()
        .fold(ForkBuilder::new(), |fork, (ty, id, bytes)| {
            fork.resource(ty, id, None, &bytes)
        })
        .build()
        .bytes;
    InterfaceData::load(&OneFile(fork), Path::new("/Nova-DF.rsrc")).expect("loads")
}

/// Never fires; every draw is the last outcome.
struct Nothing;

impl Chance for Nothing {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n - 1
    }
}

/// [`Nothing`], but every draw of 3 outcomes is the first: every `l33t`
/// letter is replaced.
struct Leetest;

impl Chance for Leetest {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        if n == 3 { 0 } else { n - 1 }
    }
}

struct Game {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
}

impl Game {
    /// The app with `pilot` saved in `store` and opened from the main
    /// menu, in flight.
    fn with_pilot(store: &MemoryPilots, pilot: &Pilot) -> Self {
        Self::with_data(store, pilot, game_data())
    }

    /// [`Game::with_pilot`] over `data`.
    fn with_data(store: &MemoryPilots, pilot: &Pilot, data: Rc<GameData>) -> Self {
        Self::with_chance(store, pilot, data, Rc::new(RefCell::new(Nothing)))
    }

    /// [`Game::with_data`] drawing on `chance`.
    fn with_chance(
        store: &MemoryPilots,
        pilot: &Pilot,
        data: Rc<GameData>,
        chance: Rc<RefCell<dyn Chance>>,
    ) -> Self {
        PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>)
            .save(pilot)
            .expect("saved");
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        let screen: AppScreen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), Rc::new(MonoMetrics))
            .with_dialogs(Rc::new(interface()), Rc::new(MonoMetrics))
            .with_chance(SharedChance::new(chance));
        let mut game = Self {
            app: App::new(&FakeWindow, data, screen),
            gpu: RecordingGpu::new(),
            frames: 0,
        };
        let at = game
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .button(MenuChoice::OpenPilot)
            .rect
            .center();
        game.click(at);
        game.tap(Key::Enter);
        assert_eq!(game.showing(), Showing::Flight);
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

    /// How far flight's display is between its last two steps: it moves
    /// on each frame flight ticks, and holds while flight is paused.
    fn alpha(&self) -> f32 {
        self.app.screen().flight_view().expect("flying").alpha()
    }

    fn spaceport(&self) -> &SpaceportView {
        self.app.screen().spaceport_view().expect("landed")
    }

    /// The missions active, by slot.
    fn active(&self) -> Vec<(u8, MissionId)> {
        self.session()
            .pilot()
            .missions()
            .iter()
            .map(|(slot, active)| (slot, active.mission))
            .collect()
    }

    /// L requests clearance, a second L lands.
    fn land(&mut self) {
        self.tap(Key::Char('l'));
        self.tap(Key::Char('l'));
        assert_eq!(self.showing(), Showing::Spaceport);
    }

    /// Clicks the spaceport's item `item`.
    fn click_port(&mut self, item: usize) {
        let at = self
            .spaceport()
            .dialog()
            .expect("laid out")
            .item_bounds(item)
            .expect("an item")
            .center();
        self.click(at);
    }

    /// Clicks the mission computer's Accept.
    fn accept(&mut self) {
        let at = self
            .spaceport()
            .mission_computer()
            .and_then(MissionComputerScreen::dialog)
            .and_then(|dialog| dialog.item_bounds(ACCEPT_ITEM))
            .expect("Accept")
            .center();
        self.click(at);
    }

    /// The offer open over the bar, if any.
    fn offer(&self) -> Option<&OfferDialog> {
        self.spaceport()
            .open_bar()
            .and_then(nova_view::spaceport::BarScreen::offer_dialog)
    }

    /// Sends redraws until an offer opens over the bar, at most two
    /// seconds', and gives its mission; `None` when none opens.
    fn await_offer(&mut self) -> Option<MissionId> {
        for _ in 0..120 {
            self.frame();
            if let Some(offer) = self.offer() {
                return Some(offer.prompt().mission);
            }
        }
        None
    }

    /// Sends redraws until a mission text shows, at most two seconds',
    /// and gives it; `None` when none shows.
    fn await_text(&mut self) -> Option<String> {
        for _ in 0..120 {
            self.frame();
            if let Some(text) = self.shown_text() {
                return Some(text);
            }
        }
        None
    }

    /// Accepts both of the mission computer's offers, closing the run's
    /// `BriefText`.
    fn accept_both(&mut self) {
        self.click_port(MISSION_BBS);
        self.accept();
        self.tap(Key::Enter);
        self.accept();
        assert_eq!(self.active(), [(0, MissionId(128)), (1, MissionId(129))]);
    }

    /// Clicks the mission info screen's item `item`.
    fn click_info(&mut self, item: usize) {
        let at = self
            .spaceport()
            .mission_info()
            .and_then(MissionInfoScreen::dialog)
            .and_then(|dialog| dialog.item_bounds(item))
            .expect("an item")
            .center();
        self.click(at);
    }

    /// The mission text shown over flight, its lines joined by spaces.
    fn flight_text(&self) -> Option<String> {
        self.app
            .screen()
            .flight_mission_text()
            .map(|dialog| dialog.lines().join(" "))
    }

    /// The flight's message line.
    fn flight_message(&self) -> Option<String> {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .message()
            .map(str::to_owned)
    }

    /// Clicks the item `item` of the mission info screen over flight.
    fn click_flight_info(&mut self, item: usize) {
        let at = self
            .app
            .screen()
            .flight_mission_info()
            .and_then(MissionInfoScreen::dialog)
            .and_then(|dialog| dialog.item_bounds(item))
            .expect("an item")
            .center();
        self.click(at);
    }

    /// The mission text shown, its lines joined by spaces.
    fn shown_text(&self) -> Option<String> {
        self.spaceport()
            .mission_text()
            .map(|dialog| dialog.lines().join(" "))
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

fn saved(store: &MemoryPilots) -> Pilot {
    nova_sim::save::decode(&store.text("Ada").expect("saved")).expect("a pilot")
}

/// A new pilot, "Ada", in flight at Alpha's centre.
fn ada() -> Pilot {
    Pilot::new(game_data().as_ref(), "Ada").expect("a pilot")
}

/// `pilot` holding each of `held`: a mission in a slot, with the days left
/// to it and the stellar of Alpha it goes to and back to, if any, as its
/// save lists them.
fn holding(pilot: &Pilot, held: &[(usize, i16, Option<i16>, Option<i16>)]) -> Pilot {
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(pilot)).expect("a save");
    save["missions"] = held
        .iter()
        .map(|&(slot, mission, days_left, stop)| {
            let stop = stop.map(|stellar| serde_json::json!({"stellar": stellar, "system": 128}));
            serde_json::json!({
                "slot": slot,
                "mission": mission,
                "destination": stop,
                "return": stop,
                "cargo_type": null,
                "cargo_tons": 0,
                "pay": 1000,
                "days_left": days_left,
                "deadline": null,
                "reached": false,
                "goals_met": false,
                "aboard": false,
                "failed": null,
                "ships": {"system": null, "left": 0, "arrived": 0, "appear": -1, "name": "", "subtitle": "", "destroyed": 0, "boarded": 0, "disabled": 0, "chased_off": 0, "uniform_type": null, "ship_done": false},
                "aux": {"countdown": 0, "arrived": 0, "left": 0},
            })
        })
        .collect();
    nova_sim::save::decode(&save.to_string()).expect("a pilot")
}

/// `pilot` with every mission slot taken by the errand, mission 129, as
/// its save lists them.
fn busy(pilot: &Pilot) -> Pilot {
    let held: Vec<_> = (0..SLOTS).map(|slot| (slot, 129, None, None)).collect();
    holding(pilot, &held)
}

/// `pilot` holding the run, mission 128, in slot 0, its deadline due:
/// no days left.
fn due(pilot: &Pilot) -> Pilot {
    holding(pilot, &[(0, 128, Some(0), None)])
}

/// Mission BBS's item.
const MISSION_BBS: usize = 11;

/// The Bar's item.
const BAR: usize = 10;

#[test]
fn the_mission_computers_selected_text_is_loaded_through_flight() {
    // A l33t resource turns the leet mode on: only a loaded text draws
    // its letters, on flight's chance, which here replaces every one.
    let offer = "a".repeat(24);
    let data = game_data_built(&timed(mission(128, 200, 201)), bar_job(), &offer, |fork| {
        fork.resource(ResType::new(*b"l33t"), 128, None, &[])
    });
    let store = MemoryPilots::new();
    let pilot = Pilot::new(data.as_ref(), "Ada").expect("a pilot");
    let mut game = Game::with_chance(&store, &pilot, data, Rc::new(RefCell::new(Leetest)));
    game.frame();
    game.land();
    game.click_port(MISSION_BBS);
    let computer = game.spaceport().mission_computer().expect("open");
    assert_eq!(
        computer.computer().rows[0].text,
        "a".repeat(24),
        "the computer's view draws nothing"
    );
    let shown = computer
        .dialog()
        .and_then(|dialog| dialog.text(COMPUTER_TEXT_ITEM))
        .expect("the text item")
        .lines()
        .concat();
    assert_eq!(shown, "4".repeat(24), "loaded, its letters drawn");
}

#[test]
fn the_mission_computer_lists_offers_accepts_them_and_their_texts_show() {
    let store = MemoryPilots::new();
    let mut game = Game::with_pilot(&store, &ada());
    game.frame();
    game.land();

    // Mission BBS lists both offers, the first one's text filled.
    game.click_port(MISSION_BBS);
    let computer = game.spaceport().mission_computer().expect("open");
    assert_eq!(computer.problem(), None);
    let shown = texts(&game.frame());
    for text in [
        "The following missions are available here:",
        "Run to Pad",
        "Errand",
        "Take this to Pad.",
        "Accept",
        "Leave",
    ] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }

    // Accept shows its BriefText, filled, over the computer, which lists
    // the errand alone.
    game.accept();
    assert_eq!(game.active(), [(0, MissionId(128))]);
    assert_eq!(game.shown_text().as_deref(), Some("Off you go to Pad."));
    let shown = texts(&game.frame());
    assert!(
        shown.contains(&"Off you go to Pad.".to_owned()),
        "{shown:?}"
    );
    game.tap(Key::Enter);
    assert_eq!(game.shown_text(), None);
    let computer = game.spaceport().mission_computer().expect("open again");
    let names: Vec<_> = computer
        .computer()
        .rows
        .iter()
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(names, ["Errand"]);

    // Accepting the errand, which has no text, leaves nothing: the
    // computer closes with nothing said.
    game.accept();
    assert!(game.spaceport().mission_computer().is_none());
    assert_eq!(game.shown_text(), None);
    assert_eq!(game.active(), [(0, MissionId(128)), (1, MissionId(129))]);
    let missions: Vec<_> = saved(&store)
        .missions()
        .iter()
        .map(|(slot, active)| (slot, active.mission))
        .collect();
    assert_eq!(missions, game.active(), "saved");

    // Taking off and landing again completes the run: its CompText shows
    // over the spaceport.
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    game.land();
    assert_eq!(game.active(), [(1, MissionId(129))]);
    assert_eq!(game.shown_text().as_deref(), Some("Thanks from Pad."));
    game.tap(Key::Escape);
    assert_eq!(game.shown_text(), None);
    assert_eq!(
        game.showing(),
        Showing::Spaceport,
        "Escape went to the text"
    );
}

#[test]
fn the_bar_offers_its_missions_one_after_another_and_a_refusal_runs_on_refuse() {
    let store = MemoryPilots::new();
    let mut game = Game::with_pilot(&store, &ada());
    game.frame();
    game.land();
    game.click_port(BAR);
    assert!(game.spaceport().open_bar().is_some(), "in the bar");

    // A quarter second on, the bar job is offered.
    assert_eq!(game.await_offer(), Some(MissionId(130)));
    let shown = texts(&game.frame());
    for text in ["Ada, join us.", "I'm in", "No"] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }
    // N refuses it; it has no RefuseText. An accept would drop the
    // second offer too (`offer_shift`), so this one is refused.
    game.tap(Key::Char('n'));
    assert!(game.offer().is_none());
    assert_eq!(game.shown_text(), None);

    // After the gap, the second; N refuses it, its RefuseText shows and
    // its OnRefuse sets bit 7.
    assert_eq!(game.await_offer(), Some(MissionId(131)));
    let shown = texts(&game.frame());
    for text in ["One more job.", "Yes", "No"] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }
    game.tap(Key::Char('n'));
    assert!(game.offer().is_none());
    assert_eq!(game.shown_text().as_deref(), Some("Suit yourself."));
    assert_eq!(game.active(), [], "refused");
    let bit = nova_sim::control::Bit::new(7).expect("in range");
    assert!(game.session().control_bit(bit));
    assert!(saved(&store).control_bit(bit), "saved");
    game.tap(Key::Enter);

    // Nothing more is offered.
    assert_eq!(game.await_offer(), None);
    assert!(game.spaceport().open_bar().is_some(), "still in the bar");
}

#[test]
fn accepting_the_bar_job_drops_the_second_offer_with_it() {
    let store = MemoryPilots::new();
    let mut game = Game::with_pilot(&store, &ada());
    game.frame();
    game.land();
    game.click_port(BAR);

    // Return accepts the bar job, and its BriefText shows.
    assert_eq!(game.await_offer(), Some(MissionId(130)));
    game.tap(Key::Enter);
    assert!(game.offer().is_none());
    assert_eq!(game.active(), [(0, MissionId(130))]);
    assert_eq!(game.shown_text().as_deref(), Some("Welcome aboard."));
    game.tap(Key::Enter);
    assert_eq!(game.shown_text(), None);

    // The list shifts from where the bar job was found, so the second
    // offer goes with it (`_OfferMissionFromPort` @0xa377d-0xa37a1).
    assert_eq!(game.await_offer(), None);
    assert!(game.spaceport().open_bar().is_some(), "still in the bar");
}

#[test]
fn an_offer_refused_for_want_of_a_slot_stays_suppressed_until_the_ship_lands_again() {
    let store = MemoryPilots::new();
    let mut game = Game::with_pilot(&store, &busy(&ada()));
    game.frame();
    game.land();
    game.click_port(BAR);
    let full: Vec<_> = (0..SLOTS)
        .map(|slot| (u8::try_from(slot).expect("a slot"), MissionId(129)))
        .collect();

    // Every slot is taken: accepting the bar job is refused, and the
    // second is offered in its place.
    assert_eq!(game.await_offer(), Some(MissionId(130)));
    game.tap(Key::Enter);
    assert!(game.offer().is_none());
    assert_eq!(game.active(), full, "refused");
    assert_eq!(game.await_offer(), Some(MissionId(131)), "not the bar job");
    game.tap(Key::Enter);
    assert_eq!(game.active(), full, "refused");

    // Nothing more is offered in this visit.
    assert_eq!(game.await_offer(), None);

    // Leaving the bar and coming back offers nothing: nothing was offered
    // at in between.
    game.tap(Key::Escape);
    assert!(game.spaceport().open_bar().is_none(), "left the bar");
    game.frame();
    game.click_port(BAR);
    assert!(game.spaceport().open_bar().is_some(), "back in the bar");
    assert_eq!(game.await_offer(), None, "still suppressed");

    // Taking off, landing again and opening the bar offers the bar job
    // again.
    game.tap(Key::Escape);
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    game.land();
    game.click_port(BAR);
    assert_eq!(game.await_offer(), Some(MissionId(130)));
}

#[test]
fn landing_offers_the_spaceport_job() {
    let store = MemoryPilots::new();
    let (job, offer) = bar_job();
    let data = game_data_with((at_venue(job, 3), offer));
    let mut game = Game::with_data(&store, &ada(), data);
    game.frame();
    game.land();

    // The job is offered over the spaceport as it lands.
    let shown = game
        .spaceport()
        .offer_dialog()
        .map(|offer| offer.prompt().mission);
    assert_eq!(shown, Some(MissionId(130)));
    let shown = texts(&game.frame());
    for text in ["Ada, join us.", "I'm in", "No"] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }

    // Return accepts it, and its BriefText shows.
    game.tap(Key::Enter);
    assert!(game.spaceport().offer_dialog().is_none());
    assert_eq!(game.active(), [(0, MissionId(130))]);
    assert_eq!(game.shown_text().as_deref(), Some("Welcome aboard."));
}

#[test]
fn an_offer_that_cannot_be_refused_with_no_text_is_accepted_without_a_dialog() {
    let store = MemoryPilots::new();
    let (job, _) = bar_job();
    let data = game_data_with((unrefusable(job), ""));
    let mut game = Game::with_data(&store, &ada(), data);
    game.frame();
    game.land();
    game.click_port(BAR);

    // A quarter second on, the bar job is accepted with no offer dialog,
    // and its BriefText shows.
    assert_eq!(game.await_text().as_deref(), Some("Welcome aboard."));
    assert!(game.offer().is_none(), "no offer dialog");
    assert_eq!(game.active(), [(0, MissionId(130))]);
    game.tap(Key::Enter);
    assert_eq!(game.shown_text(), None);

    // After the gap, the second is offered.
    assert_eq!(game.await_offer(), Some(MissionId(131)));
}

#[test]
fn the_spaceports_mission_infos_selected_quick_brief_is_loaded_through_flight() {
    // A l33t resource turns the leet mode on, and the chance replaces
    // every letter drawn: only a loaded text draws.
    let mut run = mission(128, 200, 201);
    put_i16s(&mut run, 0x36, &[205]);
    let data = game_data_built(&run, bar_job(), "Take this to <DST>.", |fork| {
        fork.resource(ResType::new(*b"l33t"), 128, None, &[])
            .resource(Desc::TYPE, 205, None, &desc(&"a".repeat(24)))
    });
    let store = MemoryPilots::new();
    let pilot = holding(&ada(), &[(0, 128, None, None)]);
    let mut game = Game::with_chance(&store, &pilot, data, Rc::new(RefCell::new(Leetest)));
    game.frame();
    game.land();
    game.tap(Key::Char('i'));
    let info = game.spaceport().mission_info().expect("open");
    assert_eq!(
        info.info().rows[0].text,
        "a".repeat(24),
        "the view draws nothing"
    );
    let shown = info
        .dialog()
        .and_then(|dialog| dialog.text(INFO_TEXT_ITEM))
        .expect("the text item")
        .lines()
        .concat();
    assert_eq!(shown, "4".repeat(24), "loaded, its letters drawn");
}

#[test]
fn mission_info_lists_the_active_missions_and_aborts_only_where_allowed() {
    let store = MemoryPilots::new();
    let mut game = Game::with_pilot(&store, &ada());
    game.frame();
    game.land();
    game.accept_both();

    // I lists both, the run's QuickBrief showing its deadline.
    game.tap(Key::Char('i'));
    let info = game.spaceport().mission_info().expect("open");
    let slots: Vec<_> = info.info().rows.iter().map(|row| row.slot).collect();
    assert_eq!(slots, [0, 1]);
    let shown = texts(&game.frame());
    for text in [
        "Currently active missions:",
        "Run to Pad",
        "Errand",
        "Back to Pad by January 8th, 0.",
        "Abort",
        "Done",
    ] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }

    // Abort does nothing on the errand, which cannot be aborted.
    game.tap(Key::Down);
    game.click_info(ABORT_ITEM);
    assert_eq!(game.active(), [(0, MissionId(128)), (1, MissionId(129))]);

    // On the run it frees its slot; the screen lists the errand alone.
    game.tap(Key::Up);
    game.click_info(ABORT_ITEM);
    assert_eq!(game.active(), [(1, MissionId(129))]);
    let missions: Vec<_> = saved(&store)
        .missions()
        .iter()
        .map(|(slot, active)| (slot, active.mission))
        .collect();
    assert_eq!(missions, [(1, MissionId(129))], "saved");
    let info = game.spaceport().mission_info().expect("still open");
    assert_eq!(info.info().rows.len(), 1);
    game.tap(Key::Escape);
    assert!(game.spaceport().mission_info().is_none());
    assert_eq!(game.showing(), Showing::Spaceport);
}

#[test]
fn a_missed_deadline_says_so_and_the_text_its_on_failure_queues_shows_over_flight() {
    let store = MemoryPilots::new();
    let run = fails_into(timed(mission(128, 200, 201)), b"S130\0");
    let data = game_data_from(&run, bar_job());
    let mut game = Game::with_data(&store, &due(&ada()), data);
    game.frame();
    game.land();

    // Leave takes off: the deadline fails the run, says so, and its
    // OnFailure's S130 accepts the bar job, whose BriefText shows over
    // the paused flight.
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::MissionText);
    assert_eq!(game.flight_text().as_deref(), Some("Welcome aboard."));
    assert_eq!(
        game.flight_message().as_deref(),
        Some("Time limit exceeded - mission failed.")
    );
    assert_eq!(
        game.active(),
        [(1, MissionId(130))],
        "the run's slot freed after its OnFailure took the next"
    );
    // The first frame after take-off only sets flight's clock, so the
    // second is the one a running flight would step on.
    let alpha = game.alpha();
    for _ in 0..2 {
        game.frame();
        assert_eq!(game.showing(), Showing::MissionText);
        assert_eq!(game.alpha().to_bits(), alpha.to_bits(), "flight is paused");
    }

    // Done closes it, back into flight, the message still shown.
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.flight_text(), None);
    assert_eq!(
        game.flight_message().as_deref(),
        Some("Time limit exceeded - mission failed.")
    );
}

#[test]
fn in_flight_i_says_there_are_no_active_missions() {
    let store = MemoryPilots::new();
    let mut game = Game::with_pilot(&store, &ada());
    game.frame();
    game.tap(Key::Char('i'));
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(
        game.flight_message().as_deref(),
        Some("You have no active missions.")
    );
    let shown = texts(&game.frame());
    assert!(
        shown.contains(&"You have no active missions.".to_owned()),
        "{shown:?}"
    );
}

#[test]
fn in_flight_mission_infos_selected_quick_brief_is_loaded_through_flight() {
    // A l33t resource turns the leet mode on, and the chance replaces
    // every letter drawn: only a loaded text draws.
    let mut run = mission(128, 200, 201);
    put_i16s(&mut run, 0x36, &[205]);
    let data = game_data_built(&run, bar_job(), "Take this to <DST>.", |fork| {
        fork.resource(ResType::new(*b"l33t"), 128, None, &[])
            .resource(Desc::TYPE, 205, None, &desc(&"a".repeat(24)))
    });
    let store = MemoryPilots::new();
    let pilot = holding(&ada(), &[(0, 128, None, None)]);
    let mut game = Game::with_chance(&store, &pilot, data, Rc::new(RefCell::new(Leetest)));
    game.frame();
    game.tap(Key::Char('i'));
    assert_eq!(game.showing(), Showing::MissionInfo);
    let info = game.app.screen().flight_mission_info().expect("open");
    assert_eq!(
        info.info().rows[0].text,
        "a".repeat(24),
        "the view draws nothing"
    );
    let shown = info
        .dialog()
        .and_then(|dialog| dialog.text(INFO_TEXT_ITEM))
        .expect("the text item")
        .lines()
        .concat();
    assert_eq!(shown, "4".repeat(24), "loaded, its letters drawn");
}

#[test]
fn in_flight_i_opens_mission_info_and_abort_works_there() {
    let store = MemoryPilots::new();
    let pilot = holding(
        &ada(),
        &[(0, 128, Some(7), Some(128)), (1, 129, None, None)],
    );
    let mut game = Game::with_pilot(&store, &pilot);
    game.frame();

    game.tap(Key::Char('i'));
    assert_eq!(game.showing(), Showing::MissionInfo);
    let alpha = game.alpha();
    let shown = texts(&game.frame());
    for text in ["Currently active missions:", "Run to Pad", "Errand"] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }
    assert_eq!(game.showing(), Showing::MissionInfo);
    assert_eq!(game.alpha().to_bits(), alpha.to_bits(), "flight is paused");

    // Abort on the run, the first row, frees its slot; the screen stays
    // open, listing the errand alone.
    game.click_flight_info(ABORT_ITEM);
    assert_eq!(game.active(), [(1, MissionId(129))]);
    assert_eq!(game.showing(), Showing::MissionInfo);
    let info = game.app.screen().flight_mission_info().expect("still open");
    assert_eq!(info.info().rows.len(), 1);

    // Done closes it, back into flight.
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert!(game.app.screen().flight_mission_info().is_none());
}
