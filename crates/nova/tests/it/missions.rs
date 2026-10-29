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
//! "Shipyard Info" (1005), the mission computer (`DLOG` 1006), the
//! mission info screen (1012), "Bar" (1013), "News" (1014), the offer
//! dialog (1016, and 1020 with a picture), "Player Info" (1017) and
//! "Desc Dialog" (3003, and 3004 with a picture), driven
//! only by window events, with pilots kept in the in-memory store.
//!
//! The mission computer: the pilot lands, and Mission BBS lists "Run to
//! Pad" and "Errand", the first one's text filled. Accept shows its
//! `BriefText`, filled, over the computer, which lists the Errand alone;
//! accepting that too closes the computer silently, nothing being left.
//! Both are active and saved. Taking off and landing again completes the
//! run, at its destination, and its `CompText` shows over the spaceport.
//!
//! A run whose `OnSuccess` moves the player (`M129`, to Beta and its
//! Dock) with no `Q`: under its `CompText` the port's status bar names
//! the Pad, landed on, and once the Trade Center has opened and closed
//! it names the Dock.
//! Closing the mission computer afterwards redraws the nav too: the
//! Dock. A run whose `OnAccept` moves the player instead: under its
//! `BriefText` the bar names the Dock, and the computer's close restores
//! the nav it opened with, naming the Pad.
//! At a Pad with an outfitter and a shipyard, buying the "Racer", whose
//! `OnPurchase` moves the player, names the Dock; the shipyard's close
//! restores the Pad but leaves the bar naming the Dock, until a Cancel
//! of an Option-Buy in the outfitter draws it: the Pad. So does the
//! shipyard's Info panel closing, the name prompt's Cancel for the
//! "Scout", and the bar's hire screen closing, each from that Dock.
//! With the Dock held after the run's `CompText`, the bar's Leave,
//! News's close, player info's and mission info's each redraw it.
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
//! Pictures: with the bar job's offer `dësc` and `BriefText` showing
//! `PICT` 5008, the bar's offer opens in the offer dialog's picture
//! variant (`DLOG` 1020), and the `BriefText` in "Desc Dialog"'s (3004).
//!
//! I in flight: with no mission, the message line says "You have no
//! active missions."; with the run and the errand held, it opens the
//! mission info screen over the paused flight, Abort there frees the
//! run's slot, and Done goes back into flight.
//!
//! A `Q` that ends a landing: with the run's `OnSuccess` a `Q` of `STR#`
//! 25048 ("Off you go."), landing at the Pad completes it, and its
//! `CompText` shows over the spaceport while the ship stays landed; Done
//! leaves, as the port's Leave does, the `Q`'s message the take-off line.
//! The run's cargo dropped off there shows its `DumpCargoText` first; a
//! failed bar job in a later slot shows its `FailText` after the
//! `CompText`; a job offered at the spaceport shows after the texts, and
//! refusing it leaves, or accepting it shows its `BriefText`, then
//! leaves. An accept in the mission computer whose `OnAccept` is the `Q`
//! shows its `BriefText`, then leaves. Without dialogs the landing
//! leaves at once. A bar job whose deadline the take-off misses has the
//! `BriefText` its `OnFailure` queues show over flight in the Done that
//! leaves, before a frame.
//!
//! Screen-loaded `dësc`s: the Pad's and the bar's conditional text shows
//! resolved on their first frame; the Pad's `<PRKnnn>`/`<SRKnnn>` cross
//! the governments the computer's names read, until the bar's load
//! resets them.

use std::cell::RefCell;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use crate::common::EntryHarness;
use nova::app::{App, AppScreen, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::desc::Desc;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::interface::Interface;
use nova_data::records::mission::Mission;
use nova_data::records::outfit::Outfit;
use nova_data::records::rank::Rank;
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
    NAME_ITEM as COMPUTER_NAME_ITEM, TEXT_ITEM as COMPUTER_TEXT_ITEM,
};
use nova_view::spaceport::{OfferDialog, SpaceportView};
use nova_view::text::fixture::MonoMetrics;
use nova_view::ui::quantity::CANCEL_ITEM;

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
    listing(128)
}

/// A `sÿst` at the map's centre listing `stellar` alone; independent,
/// with no traffic.
fn listing(stellar: i16) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[stellar]);
    put_i16s(&mut bytes, 0x44, &[-1; 8]);
    put_i16s(&mut bytes, 0x64, &[0, -1]);
    bytes
}

/// [`pad`] with a commodity exchange too (`Flags` 0x43).
fn trading_pad() -> Vec<u8> {
    let mut bytes = pad();
    bytes[0x06..0x0A].copy_from_slice(&0x43_u32.to_be_bytes());
    bytes
}

/// The Pad: a `spöb` at the centre that can be landed on, with a bar
/// (`Flags` 0x41), drawn from `spïn` 1004.
fn pad() -> Vec<u8> {
    flagged_pad(0x41)
}

/// [`pad`] with these `Flags`.
fn flagged_pad(flags: u32) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 4]);
    bytes[0x06..0x0A].copy_from_slice(&flags.to_be_bytes());
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
    pictured_desc(text, -1)
}

/// A `dësc` of `text` whose `Graphic` is `graphic`.
fn pictured_desc(text: &str, graphic: i16) -> Vec<u8> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend(graphic.to_be_bytes());
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

/// [`game_data_from`] with this Pad, over the resources `fork` holds.
fn game_data_around(
    run: &[u8],
    bar_job: (Vec<u8>, &str),
    pad: &[u8],
    fork: ForkBuilder,
) -> Rc<GameData> {
    game_data_over(run, bar_job, "Take this to <DST>.", -1, pad, fork, |fork| {
        fork
    })
}

/// [`game_data_from`] with the run's offer text `run_offer`, and `more`
/// resources added.
fn game_data_built(
    run: &[u8],
    bar_job: (Vec<u8>, &str),
    run_offer: &str,
    more: impl FnOnce(ForkBuilder) -> ForkBuilder,
) -> Rc<GameData> {
    game_data_pictured(run, bar_job, run_offer, -1, more)
}

/// [`game_data_built`] with the bar job's offer `dësc` and `BriefText`
/// showing `graphic`.
fn game_data_pictured(
    run: &[u8],
    bar_job: (Vec<u8>, &str),
    run_offer: &str,
    graphic: i16,
    more: impl FnOnce(ForkBuilder) -> ForkBuilder,
) -> Rc<GameData> {
    game_data_landing(run, bar_job, run_offer, graphic, &pad(), more)
}

/// [`game_data_pictured`] with the Pad's `spöb` `pad`.
fn game_data_landing(
    run: &[u8],
    bar_job: (Vec<u8>, &str),
    run_offer: &str,
    graphic: i16,
    pad: &[u8],
    more: impl FnOnce(ForkBuilder) -> ForkBuilder,
) -> Rc<GameData> {
    game_data_over(
        run,
        bar_job,
        run_offer,
        graphic,
        pad,
        ForkBuilder::new(),
        more,
    )
}

/// [`game_data_landing`] over the resources `base` holds.
fn game_data_over(
    run: &[u8],
    bar_job: (Vec<u8>, &str),
    run_offer: &str,
    graphic: i16,
    pad: &[u8],
    base: ForkBuilder,
    more: impl FnOnce(ForkBuilder) -> ForkBuilder,
) -> Rc<GameData> {
    game_data_named(
        Named {
            run: b"Run to <DST>;courier",
            bar: "Smoke and quiet talk.",
        },
        run,
        bar_job,
        run_offer,
        graphic,
        pad,
        base,
        more,
    )
}

/// The texts [`game_data_named`] gives the fixed resources: the run's
/// name, mission 128's, and the bar's `dësc`, 10000.
#[derive(Clone, Copy)]
struct Named<'a> {
    run: &'a [u8],
    bar: &'a str,
}

/// [`game_data_over`] with the run and the bar `named`.
#[allow(clippy::too_many_arguments)]
fn game_data_named(
    named: Named,
    run: &[u8],
    (bar_job, bar_job_offer): (Vec<u8>, &str),
    run_offer: &str,
    graphic: i16,
    pad: &[u8],
    base: ForkBuilder,
    more: impl FnOnce(ForkBuilder) -> ForkBuilder,
) -> Rc<GameData> {
    let mut fork = base
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Boarder"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system())
        .resource(Stellar::TYPE, 128, Some(b"Pad"), pad)
        .resource(Spin::TYPE, 1004, None, &spin(1000))
        .resource(RLED, 1000, None, &sheet(1, 40))
        .resource(Mission::TYPE, 128, Some(named.run), run)
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
        .resource(
            Desc::TYPE,
            4002,
            None,
            &pictured_desc(bar_job_offer, graphic),
        )
        .resource(Desc::TYPE, 4003, None, &desc("One more job."))
        .resource(
            Desc::TYPE,
            202,
            None,
            &pictured_desc("Welcome aboard.", graphic),
        )
        .resource(Desc::TYPE, 203, None, &desc("Suit yourself."))
        .resource(Desc::TYPE, 10_000, None, &desc(named.bar))
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
        5008, 8500, 8501, 8503, 8505, 8517, 8521, 8522, 8523, 8524, 8525, 8526, 8527, 8528, 10_004,
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

/// The picture variants of the offer dialog (`DLOG` 1020) and "Desc
/// Dialog" (3004), each with its items as stock has them.
fn picture_dialogs() -> [[(ResType, i16, Vec<u8>); 2]; 2] {
    let picture_offer = dialog(
        1020,
        (651, 244),
        &[
            ((221, 213, 99, 25), false),
            ((115, 213, 99, 25), false),
            ((13, 7, 413, 200), false),
            ((147, 319, 32, 32), false),
            ((194, 322, 32, 32), false),
            ((170, 213, 99, 25), false),
            ((169, 333, 32, 32), false),
            ((441, 7, 200, 200), false),
            ((340, 214, 23, 23), false),
            ((373, 214, 23, 23), false),
        ],
    );
    let picture_text = dialog(
        3004,
        (648, 245),
        &[
            ((173, 214, 99, 25), true),
            ((439, 7, 200, 200), false),
            ((12, 7, 415, 201), false),
            ((70, 345, 78, 25), true),
            ((330, 215, 23, 23), false),
            ((363, 215, 23, 23), false),
        ],
    );
    [picture_offer, picture_text]
}

/// "Trade" (`DLOG` 1001), with its items as stock has them.
fn trade_dialog() -> [(ResType, i16, Vec<u8>); 2] {
    dialog(
        1001,
        (426, 252),
        &[
            ((272, 221, 99, 25), true),
            ((129, 299, 68, 30), false),
            ((38, 9, 352, 17), false),
            ((38, 25, 352, 13), true),
            ((38, 37, 352, 13), true),
            ((38, 49, 352, 13), true),
            ((38, 61, 352, 13), true),
            ((38, 73, 352, 13), true),
            ((38, 85, 352, 14), true),
            ((38, 98, 352, 14), true),
            ((38, 111, 352, 14), true),
            ((38, 124, 352, 60), false),
            ((60, 221, 99, 25), true),
            ((166, 221, 99, 25), true),
            ((41, 190, 346, 24), false),
            ((212, 304, 68, 30), false),
            ((178, 339, 68, 30), false),
        ],
    )
}

/// "Outfit" (`DLOG` 1002), with its items as stock has them.
fn outfit_dialog() -> [(ResType, i16, Vec<u8>); 2] {
    dialog(
        1002,
        (765, 321),
        &[
            ((500, 289, 99, 25), true),
            ((248, 440, 68, 30), false),
            ((251, 291, 29, 21), false),
            ((394, 289, 99, 25), false),
            ((9, 8, 333, 271), true),
            ((354, 10, 192, 267), false),
            ((288, 289, 99, 25), true),
            ((557, 8, 200, 200), false),
            ((618, 214, 135, 100), false),
            ((148, 288, 25, 25), true),
            ((178, 288, 25, 25), true),
            ((170, 449, 32, 32), false),
            ((356, 422, 68, 30), false),
        ],
    )
}

/// "Shipyard" (`DLOG` 1004), with its items as stock has them.
fn shipyard_dialog() -> [(ResType, i16, Vec<u8>); 2] {
    dialog(
        1004,
        (765, 323),
        &[
            ((365, 289, 109, 25), true),
            ((248, 440, 68, 30), false),
            ((251, 491, 29, 21), false),
            ((394, 489, 99, 25), false),
            ((9, 8, 333, 271), true),
            ((354, 10, 192, 267), false),
            ((480, 289, 109, 25), true),
            ((557, 8, 200, 200), false),
            ((614, 214, 143, 100), false),
            ((253, 289, 89, 25), true),
            ((170, 449, 32, 32), false),
            ((141, 288, 25, 25), true),
            ((171, 288, 25, 25), true),
        ],
    )
}

/// "Shipyard Info" (`DLOG` 1005), "News" (1014, with no items) and
/// "Player Info" (1017), each with its items as stock has them.
fn info_dialogs() -> [[(ResType, i16, Vec<u8>); 2]; 3] {
    let ship_info = dialog(
        1005,
        (250, 285),
        &[
            ((86, 253, 74, 25), true),
            ((300, 0, 1, 1), false),
            ((3, 3, 240, 24), false),
            ((300, 0, 1, 1), false),
            ((9, 32, 234, 214), false),
        ],
    );
    let news = dialog(1014, (300, 230), &[]);
    let player_info = dialog(
        1017,
        (413, 227),
        &[
            ((293, 195, 99, 25), true),
            ((7, 8, 99, 25), true),
            ((107, 8, 99, 25), true),
            ((207, 8, 99, 25), true),
            ((307, 8, 99, 25), true),
            ((4, 40, 405, 141), false),
            ((60, 195, 150, 25), true),
        ],
    );
    [ship_info, news, player_info]
}

/// The interface file: stock "Spaceport" (`DLOG` 1000), "Trade" (1001),
/// "Outfit" (1002), "Shipyard" (1004), "Shipyard Info" (1005), the
/// mission computer (1006), the mission info screen (1012), "Bar" (1013),
/// "News" (1014), the offer dialog (1016, and 1020 with a picture),
/// "Player Info" (1017) and "Desc Dialog" (3003, and 3004 with a
/// picture), each with its items as stock has them.
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
    let fork = [
        spaceport,
        trade_dialog(),
        outfit_dialog(),
        shipyard_dialog(),
    ]
    .into_iter()
    .chain(mission_dialogs())
    .chain(info_dialogs())
    .chain(picture_dialogs())
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
        game.tap(Key::Enter);
        game.enter_ship();
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

    /// L requests clearance, a few frames let the dock approach clear it,
    /// and a second L lands.
    fn land(&mut self) {
        self.tap(Key::Char('l'));
        self.approach();
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
const MISSION_BBS: usize = 10;

/// The Bar's item.
const BAR: usize = 11;

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
fn the_mission_computer_shows_the_offers_name_cut_then_filled_on_its_first_frame() {
    // Item 5 is 269 wide: the stored name is cut to 268 in Times 18
    // (9.9 a character in `MonoMetrics`), 26 characters and the
    // ellipsis, before its tags are filled.
    let data = game_data_named(
        Named {
            run: b"Run to <DST> by the long road home;courier",
            bar: "Smoke and quiet talk.",
        },
        &timed(mission(128, 200, 201)),
        bar_job(),
        "Take this to <DST>.",
        -1,
        &pad(),
        ForkBuilder::new(),
        |fork| fork,
    );
    let store = MemoryPilots::new();
    let mut game = Game::with_data(&store, &ada(), data);
    game.frame();
    game.land();
    game.click_port(MISSION_BBS);
    let computer = game.spaceport().mission_computer().expect("open");
    let name = computer
        .dialog()
        .and_then(|dialog| dialog.text(COMPUTER_NAME_ITEM))
        .expect("the name item")
        .lines()
        .join(" ");
    assert_eq!(name, "Run to Pad by the long r…", "cut, then filled");
    let frame = game.frame();
    assert!(
        texts(&frame).contains(&"Run to Pad by the long r…".to_owned()),
        "drawn on the first frame: {:?}",
        texts(&frame)
    );
}

/// Whether the frame drew `text`, and drew no conditional text left
/// unresolved.
fn shows_resolved(frame: &Frame, text: &str) -> bool {
    let shown = texts(frame);
    shown.contains(&text.to_owned()) && !shown.iter().any(|shown| shown.contains('{'))
}

#[test]
fn a_landing_shows_the_spaceports_conditional_text_resolved_on_its_first_frame() {
    let data = game_data_built(
        &timed(mission(128, 200, 201)),
        bar_job(),
        "Take this to <DST>.",
        |fork| {
            fork.resource(
                Desc::TYPE,
                128,
                None,
                &desc("Welcome, {G \"sir\" \"ma'am\"}."),
            )
        },
    );
    let store = MemoryPilots::new();
    let mut game = Game::with_data(&store, &ada(), data);
    game.frame();
    game.land();
    let frame = game.frame();
    assert!(
        shows_resolved(&frame, "Welcome, sir."),
        "the male reading: {:?}",
        texts(&frame)
    );
}

#[test]
fn the_bars_desc_shows_its_conditional_text_resolved_on_its_first_frame() {
    let data = game_data_named(
        Named {
            run: b"Run to <DST>;courier",
            bar: "Smoke, {G \"sir\" \"ma'am\"}.",
        },
        &timed(mission(128, 200, 201)),
        bar_job(),
        "Take this to <DST>.",
        -1,
        &pad(),
        ForkBuilder::new(),
        |fork| fork,
    );
    let store = MemoryPilots::new();
    let mut game = Game::with_data(&store, &ada(), data);
    game.frame();
    game.land();
    game.click_port(BAR);
    assert!(game.spaceport().open_bar().is_some(), "in the bar");
    let frame = game.frame();
    assert!(
        shows_resolved(&frame, "Smoke, sir."),
        "the male reading: {:?}",
        texts(&frame)
    );
}

/// A `ränk` affiliated with `govt`, its names `name`.
fn rank(govt: i16, name: &str) -> Vec<u8> {
    let mut bytes = vec![0; Rank::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[govt]);
    for at in [0x18, 0x58] {
        bytes[at..at + name.len()].copy_from_slice(name.as_bytes());
    }
    bytes
}

/// The run's row name in the mission computer, opened and then closed.
fn computer_run_name(game: &mut Game) -> String {
    game.click_port(MISSION_BBS);
    let name = game
        .spaceport()
        .mission_computer()
        .expect("open")
        .computer()
        .rows[0]
        .name
        .clone();
    game.tap(Key::Escape);
    assert!(game.spaceport().mission_computer().is_none(), "closed");
    name
}

#[test]
fn a_spaceport_desc_naming_srk_resets_the_governments_a_later_computer_row_reads() {
    // Ranks 200 ("Cdr", of gövt 128) and 201 ("Ad", of 129), both held.
    // The Pad's dësc and the run's offer text cross the governments the
    // way the session's tests do; the bar's names no rank.
    let crossed = "Earlier: <PRK128> <SRK129>.";
    let data = game_data_named(
        Named {
            run: b"Run <PRK128>",
            bar: "Smoke.",
        },
        &timed(mission(128, 200, 201)),
        bar_job(),
        crossed,
        -1,
        &pad(),
        ForkBuilder::new(),
        |fork| {
            fork.resource(Desc::TYPE, 128, None, &desc(crossed))
                .resource(Rank::TYPE, 200, None, &rank(128, "Cdr"))
                .resource(Rank::TYPE, 201, None, &rank(129, "Ad"))
        },
    );
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&ada())).expect("a save");
    save["ranks"] = serde_json::json!([200, 201]);
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let store = MemoryPilots::new();
    let mut game = Game::with_data(&store, &pilot, data);
    game.frame();
    game.land();

    // The landing loaded the Pad's dësc, so the computer's names, munged
    // before its text loads (@0xa0947), read the governments it left.
    assert_eq!(computer_run_name(&mut game), "Run Ad");
    assert_eq!(
        computer_run_name(&mut game),
        "Run Ad",
        "the run's own text left them the same"
    );

    // The bar's screen load resets them (@0x71926-0x71930), and with no
    // government remembered the tag is left as written (@0x9d53d-0x9d5a5).
    game.click_port(BAR);
    assert!(game.spaceport().open_bar().is_some(), "in the bar");
    game.tap(Key::Escape);
    assert!(game.spaceport().open_bar().is_none(), "left");
    assert_eq!(computer_run_name(&mut game), "Run <PRK128>");
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

/// [`game_data`] where the run's `OnSuccess` moves the player to Beta
/// (`M129`), with no `Q` after it, Beta listing the Dock (129) alone, the
/// Pad has a trade center too (`Flags` 0x43), and the bar's news has a
/// commercial (`STR#` 8100) and a story (8101).
fn moving_run_data() -> Rc<GameData> {
    let mut run = timed(mission(128, 200, 201));
    let on_success = b"M129\0";
    run[0x359..0x359 + on_success.len()].copy_from_slice(on_success);
    let news = beta()
        .resource(StrList::TYPE, 8100, None, &str_list(&["Buy Bibble"]))
        .resource(StrList::TYPE, 8101, None, &str_list(&["Nothing new"]));
    game_data_around(&run, bar_job(), &flagged_pad(0x43), news)
}

/// `STR#` of `strings`.
fn str_list(strings: &[&str]) -> Vec<u8> {
    let mut bytes = i16::try_from(strings.len())
        .expect("few")
        .to_be_bytes()
        .to_vec();
    for string in strings {
        bytes.push(u8::try_from(string.len()).expect("short"));
        bytes.extend(string.as_bytes());
    }
    bytes
}

/// Beta (129), listing the Dock (129) alone.
fn beta() -> ForkBuilder {
    ForkBuilder::new()
        .resource(System::TYPE, 129, Some(b"Beta"), &listing(129))
        .resource(Stellar::TYPE, 129, Some(b"Dock"), &pad())
}

/// [`game_data`] where the run's `OnAccept` moves the player to Beta
/// (`M129`), Beta listing the Dock (129) alone.
fn moving_accept_data() -> Rc<GameData> {
    let mut run = timed(mission(128, 200, 201));
    let on_accept = b"M129\0";
    run[0x15B..0x15B + on_accept.len()].copy_from_slice(on_accept);
    game_data_around(&run, bar_job(), &pad(), beta())
}

/// The texts of flight's status bar as the spaceport draws it beside
/// itself.
fn port_bar_texts(game: &Game) -> Vec<String> {
    let mut bar = nova_view::DrawList::new();
    game.app
        .screen()
        .flight_view()
        .expect("flying")
        .draw_status_bar(&mut bar);
    bar.iter()
        .filter_map(|command| match command {
            nova_view::DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_comp_text_shows_the_stellar_landed_on_and_a_shop_closing_on_successs_move() {
    let store = MemoryPilots::new();
    let mut game = Game::with_data(&store, &ada(), moving_run_data());
    game.frame();
    game.land();
    game.click_port(MISSION_BBS);
    game.accept();
    game.tap(Key::Enter);
    for _ in 0..3 {
        if game.showing() == Showing::Spaceport {
            game.tap(Key::Escape);
        }
    }
    assert_eq!(game.showing(), Showing::Flight);

    // Landing at the run's return completes it: OnSuccess's M129 selects
    // the Dock, but under the CompText the port's bar names the Pad.
    game.land();
    assert_eq!(game.shown_text().as_deref(), Some("Thanks from Pad."));
    assert_eq!(
        game.session().nav_mode(),
        nova_sim::NavMode::Stellar(Some(nova_sim::StellarId(129)))
    );
    let bar = port_bar_texts(&game);
    assert!(bar.contains(&"Pad".to_owned()), "{bar:?}");
    assert!(!texts(&game.frame()).contains(&"Dock".to_owned()));
    game.tap(Key::Escape);
    assert_eq!(game.shown_text(), None);
    assert!(!texts(&game.frame()).contains(&"Dock".to_owned()), "held");

    // The Trade Center's close redraws it: the Dock.
    game.click_port(7);
    assert!(game.spaceport().open_trade().is_some(), "trading");
    game.tap(Key::Escape);
    assert!(game.spaceport().open_trade().is_none(), "closed");
    let bar = port_bar_texts(&game);
    assert!(bar.contains(&"Dock".to_owned()), "{bar:?}");
    assert!(texts(&game.frame()).contains(&"Dock".to_owned()));
}

#[test]
fn a_landed_m_in_on_accept_is_undone_as_the_mission_computer_closes() {
    let store = MemoryPilots::new();
    let mut game = Game::with_data(&store, &ada(), moving_accept_data());
    game.frame();
    game.land();
    game.click_port(MISSION_BBS);
    game.accept();

    // Under the BriefText the bar is still the one held before the accept,
    // naming the Pad: the computer redraws nothing before it
    // (`_DoMissionDialog` @0xa3b37-0xa3bb4), and `OnAccept`'s `M` marks
    // nothing. The brief's close redraws it.
    assert_eq!(game.shown_text().as_deref(), Some("Off you go to Pad."));
    assert_eq!(
        game.session().nav_mode(),
        nova_sim::NavMode::Stellar(Some(nova_sim::StellarId(129)))
    );
    let bar = port_bar_texts(&game);
    assert!(bar.contains(&"Pad".to_owned()), "{bar:?}");
    assert!(!bar.contains(&"Dock".to_owned()), "{bar:?}");

    // The computer's close restores the Pad and draws it.
    game.tap(Key::Enter);
    if game.spaceport().mission_computer().is_some() {
        game.tap(Key::Escape);
    }
    assert!(game.spaceport().mission_computer().is_none(), "closed");
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(
        game.session().nav_mode(),
        nova_sim::NavMode::Stellar(Some(nova_sim::StellarId(128)))
    );
    let bar = port_bar_texts(&game);
    assert!(bar.contains(&"Pad".to_owned()), "{bar:?}");
    let shown = texts(&game.frame());
    assert!(shown.contains(&"Pad".to_owned()), "{shown:?}");
    assert!(!shown.contains(&"Dock".to_owned()), "{shown:?}");
}

/// Over [`moving_run_data`]: the run and the errand accepted, the run
/// completed on landing again, its `OnSuccess` selecting the Dock, and
/// its `CompText` closed, the port's bar still naming the Pad, held, and
/// the errand left active.
fn docked_with_the_dock_held(store: &MemoryPilots) -> Game {
    let mut game = Game::with_data(store, &ada(), moving_run_data());
    game.frame();
    game.land();
    game.click_port(MISSION_BBS);
    game.accept();
    game.tap(Key::Enter);
    if game.spaceport().mission_computer().is_some() {
        game.accept();
    }
    for _ in 0..3 {
        if game.showing() == Showing::Spaceport {
            game.tap(Key::Escape);
        }
    }
    game.land();
    game.tap(Key::Escape);
    assert_eq!(game.shown_text(), None, "the CompText closed");
    assert_eq!(game.active(), [(1, MissionId(129))], "the errand");
    assert_eq!(
        game.session().nav_mode(),
        nova_sim::NavMode::Stellar(Some(nova_sim::StellarId(129)))
    );
    let bar = port_bar_texts(&game);
    assert!(bar.contains(&"Pad".to_owned()), "held: {bar:?}");
    game
}

#[test]
fn the_mission_computers_close_redraws_the_port_bars_nav() {
    let store = MemoryPilots::new();
    let mut game = docked_with_the_dock_held(&store);

    // Mission BBS, then its close, which redraws the nav: the Dock.
    game.click_port(MISSION_BBS);
    if game.spaceport().mission_computer().is_some() {
        game.tap(Key::Escape);
    } else {
        game.tap(Key::Enter);
    }
    assert!(game.spaceport().mission_computer().is_none());
    assert_eq!(game.shown_text(), None);
    let bar = port_bar_texts(&game);
    assert!(bar.contains(&"Dock".to_owned()), "{bar:?}");
    assert!(texts(&game.frame()).contains(&"Dock".to_owned()));
}

#[test]
fn each_bar_side_closing_redraws_the_port_bars_nav() {
    type Step = fn(&mut Game);
    let rows: [(&str, Step, Step); 4] = [
        (
            "the bar's Leave (@0x4945e)",
            |game| {
                game.click_port(BAR);
                assert!(game.spaceport().open_bar().is_some(), "in the bar");
            },
            |game| {
                game.tap(Key::Escape);
                assert!(game.spaceport().open_bar().is_none(), "left");
            },
        ),
        (
            "News's close (@0x48153)",
            |game| {
                game.click_port(BAR);
                game.tap(Key::Char('n'));
                let bar = game.spaceport().open_bar().expect("in the bar");
                let news = bar.news_screen().expect("the news");
                assert_eq!(news.problem(), None, "laid out");
            },
            |game| {
                game.tap(Key::Escape);
                let bar = game.spaceport().open_bar().expect("back in the bar");
                assert!(bar.news_screen().is_none());
            },
        ),
        (
            "player info's close (@0x5a725)",
            |game| {
                game.tap(Key::Char('p'));
                let open = game.spaceport().player_info().expect("player info");
                assert_eq!(open.problem(), None, "laid out");
            },
            |game| {
                game.tap(Key::Escape);
                assert!(game.spaceport().player_info().is_none());
            },
        ),
        (
            "mission info's close (@0x9e769)",
            |game| {
                game.tap(Key::Char('i'));
                let open = game.spaceport().mission_info().expect("mission info");
                assert_eq!(open.problem(), None, "laid out");
            },
            |game| {
                game.tap(Key::Escape);
                assert!(game.spaceport().mission_info().is_none());
            },
        ),
    ];
    for (what, open, close) in rows {
        let store = MemoryPilots::new();
        let mut game = docked_with_the_dock_held(&store);
        open(&mut game);
        let bar = port_bar_texts(&game);
        assert!(bar.contains(&"Pad".to_owned()), "{what}: held: {bar:?}");
        close(&mut game);
        let bar = port_bar_texts(&game);
        assert!(bar.contains(&"Dock".to_owned()), "{what}: {bar:?}");
        assert!(texts(&game.frame()).contains(&"Dock".to_owned()), "{what}");
    }
}

/// The Pad with an outfitter and a shipyard too (`Flags` 0x4D), of tech
/// level 1.
fn shopping_pad() -> Vec<u8> {
    let mut bytes = flagged_pad(0x4D);
    put_i16s(&mut bytes, 0x0C, &[1]);
    bytes
}

/// The "Racer" (129), for sale and for hire every day at 1,000 credits,
/// whose `OnPurchase` moves the player to Beta (`M129`); the "Scout"
/// (130), for sale every day at 1,000 credits; and a fuel pod (128),
/// massless, at 100 credits, up to 10.
fn shop_goods() -> ForkBuilder {
    let mut racer = ship();
    racer[0x30..0x34].copy_from_slice(&1000_i32.to_be_bytes());
    put_i16s(&mut racer, 0x388, &[100, 100]);
    let on_purchase = b"M129\0";
    let mut scout = racer.clone();
    put_i16s(&mut scout, 0x388, &[100, 0]);
    racer[0x26A..0x26A + on_purchase.len()].copy_from_slice(on_purchase);
    let mut pod = vec![0; Outfit::SIZE.expect("fixed")];
    put_i16s(&mut pod, 0x00, &[10, 0, 1, 0, 0, 10]);
    pod[0x0E..0x12].copy_from_slice(&100_i32.to_be_bytes());
    put_i16s(&mut pod, 0x3F0, &[100, -1]);
    beta()
        .resource(Ship::TYPE, 129, Some(b"Racer"), &racer)
        .resource(ShipAnim::TYPE, 129, None, &ship_anim(2000))
        .resource(Ship::TYPE, 130, Some(b"Scout"), &scout)
        .resource(ShipAnim::TYPE, 130, None, &ship_anim(2000))
        .resource(Outfit::TYPE, 128, Some(b"Fuel Pod"), &pod)
}

/// [`game_data`] at the [`shopping_pad`], with the [`shop_goods`].
fn shopping_data() -> Rc<GameData> {
    game_data_around(
        &timed(mission(128, 200, 201)),
        bar_job(),
        &shopping_pad(),
        shop_goods(),
    )
}

/// The Outfitter's item.
const OUTFITTER: usize = 8;
/// The Shipyard's item.
const SHIPYARD: usize = 9;

/// Over [`shopping_data`], the Racer bought, its `OnPurchase` moving the
/// player to Beta's Dock, which the purchase draws, and the shipyard
/// closed, restoring the Pad but drawing the bar as the purchase left
/// it: naming the Dock.
fn bought_the_racer(store: &MemoryPilots) -> Game {
    let mut game = Game::with_data(store, &ada(), shopping_data());
    game.frame();
    game.land();
    game.click_port(SHIPYARD);
    select_the_racer(&mut game);
    game.tap(Key::Char('b'));
    assert!(
        game.spaceport()
            .open_shipyard()
            .and_then(|shipyard| shipyard.naming())
            .is_some(),
        "the name prompt"
    );
    game.tap(Key::Enter);
    assert_eq!(game.session().ship(), nova_sim::ShipId(129), "bought");
    assert!(
        game.spaceport().open_shipyard().is_none(),
        "the purchase closes it"
    );
    assert_eq!(
        game.session().nav_mode(),
        nova_sim::NavMode::Stellar(Some(nova_sim::StellarId(128)))
    );
    let bar = port_bar_texts(&game);
    assert!(bar.contains(&"Dock".to_owned()), "the close's: {bar:?}");
    game
}

/// Clicks the Racer's cell in the shipyard open.
fn select_the_racer(game: &mut Game) {
    select_cell(game, 0);
}

/// Clicks cell `cell` in the shipyard open: 0 the Racer's, 1 the
/// Scout's.
fn select_cell(game: &mut Game, cell: usize) {
    let at = game
        .spaceport()
        .open_shipyard()
        .and_then(|shipyard| shipyard.cell_bounds(cell))
        .expect("a ship's cell")
        .center();
    game.click(at);
}

#[test]
fn the_shipyards_info_panels_close_redraws_the_port_bar() {
    let store = MemoryPilots::new();
    let mut game = bought_the_racer(&store);
    game.click_port(SHIPYARD);
    select_the_racer(&mut game);
    game.tap(Key::Space);
    let shipyard = game.spaceport().open_shipyard().expect("shipbuying");
    assert!(shipyard.info_panel().is_some(), "the Info panel");
    let bar = port_bar_texts(&game);
    assert!(bar.contains(&"Dock".to_owned()), "held: {bar:?}");
    game.tap(Key::Escape);
    let shipyard = game.spaceport().open_shipyard().expect("still open");
    assert!(shipyard.info_panel().is_none(), "closed");
    let bar = port_bar_texts(&game);
    assert!(bar.contains(&"Pad".to_owned()), "@0x5bac2: {bar:?}");
}

#[test]
fn the_shipyards_name_prompts_cancel_redraws_the_port_bar() {
    let store = MemoryPilots::new();
    let mut game = bought_the_racer(&store);
    game.click_port(SHIPYARD);
    select_cell(&mut game, 1);
    game.tap(Key::Char('b'));
    let shipyard = game.spaceport().open_shipyard().expect("shipbuying");
    assert!(shipyard.naming().is_some(), "the name prompt");
    let bar = port_bar_texts(&game);
    assert!(bar.contains(&"Dock".to_owned()), "held: {bar:?}");
    let cancel = game
        .spaceport()
        .open_shipyard()
        .and_then(|shipyard| shipyard.naming())
        .and_then(|prompt| prompt.dialog().item_bounds(6))
        .expect("the prompt's Cancel")
        .center();
    game.click(cancel);
    let shipyard = game.spaceport().open_shipyard().expect("still open");
    assert!(shipyard.naming().is_none(), "cancelled");
    assert_eq!(game.session().ship(), nova_sim::ShipId(129), "not bought");
    let bar = port_bar_texts(&game);
    assert!(bar.contains(&"Pad".to_owned()), "@0x5eb73: {bar:?}");
}

#[test]
fn the_hire_screens_close_redraws_the_port_bar() {
    let store = MemoryPilots::new();
    let mut game = bought_the_racer(&store);
    game.click_port(BAR);
    game.tap(Key::Char('h'));
    let bar = game.spaceport().open_bar().expect("in the bar");
    assert!(bar.hire_screen().is_some(), "the hire screen");
    let shown = port_bar_texts(&game);
    assert!(shown.contains(&"Dock".to_owned()), "held: {shown:?}");
    game.tap(Key::Escape);
    let bar = game.spaceport().open_bar().expect("back in the bar");
    assert!(bar.hire_screen().is_none(), "cancelled");
    let shown = port_bar_texts(&game);
    assert!(shown.contains(&"Pad".to_owned()), "@0x5f8d4: {shown:?}");
    assert_eq!(
        game.session().nav_mode(),
        nova_sim::NavMode::Stellar(Some(nova_sim::StellarId(128))),
        "restored"
    );
}

#[test]
fn the_hire_screens_info_panels_close_redraws_the_port_bar() {
    let store = MemoryPilots::new();
    let mut game = bought_the_racer(&store);
    game.click_port(BAR);
    game.tap(Key::Char('h'));
    // It opens with nothing selected: Right picks the first ship.
    game.tap(Key::Right);
    game.tap(Key::Space);
    let hire = game
        .spaceport()
        .open_bar()
        .and_then(|bar| bar.hire_screen())
        .expect("the hire screen");
    assert!(hire.info_panel().is_some(), "the Info panel");
    let shown = port_bar_texts(&game);
    assert!(shown.contains(&"Dock".to_owned()), "held: {shown:?}");
    game.tap(Key::Escape);
    let hire = game
        .spaceport()
        .open_bar()
        .and_then(|bar| bar.hire_screen())
        .expect("the hire screen still open");
    assert!(hire.info_panel().is_none(), "closed");
    let shown = port_bar_texts(&game);
    assert!(shown.contains(&"Pad".to_owned()), "@0x5bac2: {shown:?}");
}

#[test]
fn an_outfitter_cancel_draws_the_nav_the_shipyards_close_restored() {
    let store = MemoryPilots::new();
    let mut game = bought_the_racer(&store);

    // Option-B asks how many fuel pods; Cancel buys none, and the buy's
    // draw shows the Pad.
    game.click_port(OUTFITTER);
    let cell = game
        .spaceport()
        .open_outfitter()
        .and_then(|outfitter| outfitter.cell_bounds(0))
        .expect("the pod's cell")
        .center();
    game.click(cell);
    let cash = game.session().pilot().cash();
    game.key(Key::Alt, true);
    game.tap(Key::Char('b'));
    game.key(Key::Alt, false);
    let cancel = game
        .spaceport()
        .open_outfitter()
        .and_then(|outfitter| outfitter.quantity())
        .and_then(|asking| asking.dialog().item_bounds(CANCEL_ITEM))
        .expect("the quantity dialog's Cancel")
        .center();
    game.click(cancel);
    let outfitter = game.spaceport().open_outfitter().expect("still open");
    assert!(outfitter.quantity().is_none(), "cancelled");
    assert_eq!(game.session().pilot().owned(nova_sim::OutfitId(128)), 0);
    assert_eq!(game.session().pilot().cash(), cash, "nothing bought");
    let bar = port_bar_texts(&game);
    assert!(bar.contains(&"Pad".to_owned()), "{bar:?}");
    assert!(!bar.contains(&"Dock".to_owned()), "{bar:?}");
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
fn a_bar_job_whose_dëscs_have_a_picture_shows_the_picture_variants() {
    let store = MemoryPilots::new();
    let data = game_data_pictured(
        &timed(mission(128, 200, 201)),
        bar_job(),
        "Take this to <DST>.",
        5008,
        |fork| fork,
    );
    let mut game = Game::with_data(&store, &ada(), data);
    game.frame();
    game.land();
    game.click_port(BAR);

    // The offer opens in DLOG 1020, 651 wide, its picture PICT 5008.
    assert_eq!(game.await_offer(), Some(MissionId(130)));
    let offer = game.offer().expect("open");
    assert_eq!(offer.prompt().graphic, Some(5008));
    assert_eq!(
        offer.dialog().map(|dialog| dialog.bounds().width()),
        Some(651.0)
    );
    let shown = texts(&game.frame());
    for text in ["Ada, join us.", "I'm in", "No"] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }

    // Return accepts it, and its BriefText shows in DLOG 3004.
    game.tap(Key::Enter);
    assert_eq!(game.shown_text().as_deref(), Some("Welcome aboard."));
    let text = game.spaceport().mission_text().expect("shown");
    assert_eq!(text.picture(), Some(5008));
    assert_eq!(Some(text.dialog().bounds().width()), Some(648.0));
    game.frame();
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
fn the_spaceports_mission_info_loads_the_quick_brief_of_the_slot_selected() {
    // The only mission is in slot 1: the text loaded is that slot's.
    let mut run = mission(128, 200, 201);
    put_i16s(&mut run, 0x36, &[205]);
    let data = game_data_built(&run, bar_job(), "Take this to <DST>.", |fork| {
        fork.resource(ResType::new(*b"l33t"), 128, None, &[])
            .resource(Desc::TYPE, 205, None, &desc(&"a".repeat(24)))
    });
    let store = MemoryPilots::new();
    let pilot = holding(&ada(), &[(1, 128, None, None)]);
    let mut game = Game::with_chance(&store, &pilot, data, Rc::new(RefCell::new(Leetest)));
    game.frame();
    game.land();
    game.tap(Key::Char('i'));
    let info = game.spaceport().mission_info().expect("open");
    let shown = info
        .dialog()
        .and_then(|dialog| dialog.text(INFO_TEXT_ITEM))
        .expect("the text item")
        .lines()
        .concat();
    assert_eq!(shown, "4".repeat(24), "slot 1's, loaded");
}

#[test]
fn the_trade_centers_i_opens_mission_info_over_it_and_loads_the_quick_brief_through_flight() {
    // A l33t resource turns the leet mode on, and the chance replaces
    // every letter drawn: only a loaded text draws.
    let mut run = mission(128, 200, 201);
    put_i16s(&mut run, 0x36, &[205]);
    let data = game_data_landing(
        &run,
        bar_job(),
        "Take this to <DST>.",
        -1,
        &trading_pad(),
        |fork| {
            fork.resource(ResType::new(*b"l33t"), 128, None, &[])
                .resource(Desc::TYPE, 205, None, &desc(&"a".repeat(24)))
        },
    );
    let store = MemoryPilots::new();
    let pilot = holding(&ada(), &[(0, 128, None, None)]);
    let mut game = Game::with_chance(&store, &pilot, data, Rc::new(RefCell::new(Leetest)));
    game.frame();
    game.land();
    game.click_port(7);
    assert!(game.spaceport().open_trade().is_some(), "trading");
    game.tap(Key::Char('i'));
    let info = game.spaceport().mission_info().expect("open over it");
    let shown = info
        .dialog()
        .and_then(|dialog| dialog.text(INFO_TEXT_ITEM))
        .expect("the text item")
        .lines()
        .concat();
    assert_eq!(shown, "4".repeat(24), "loaded, its letters drawn");
    game.tap(Key::Enter);
    assert!(
        game.spaceport().mission_info().is_none(),
        "Return closed it"
    );
    assert!(game.spaceport().open_trade().is_some(), "still trading");
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
fn in_flight_mission_info_loads_the_quick_brief_of_the_slot_selected() {
    // The only mission is in slot 1: the text loaded is that slot's.
    let mut run = mission(128, 200, 201);
    put_i16s(&mut run, 0x36, &[205]);
    let data = game_data_built(&run, bar_job(), "Take this to <DST>.", |fork| {
        fork.resource(ResType::new(*b"l33t"), 128, None, &[])
            .resource(Desc::TYPE, 205, None, &desc(&"a".repeat(24)))
    });
    let store = MemoryPilots::new();
    let pilot = holding(&ada(), &[(1, 128, None, None)]);
    let mut game = Game::with_chance(&store, &pilot, data, Rc::new(RefCell::new(Leetest)));
    game.frame();
    game.tap(Key::Char('i'));
    let info = game.app.screen().flight_mission_info().expect("open");
    let shown = info
        .dialog()
        .and_then(|dialog| dialog.text(INFO_TEXT_ITEM))
        .expect("the text item")
        .lines()
        .concat();
    assert_eq!(shown, "4".repeat(24), "slot 1's, loaded");
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

// A `Q` that ends a landing: held until the texts over the port close.

/// `mission` with this raw `OnSuccess`.
fn succeeds_into(mut mission: Vec<u8>, on_success: &[u8]) -> Vec<u8> {
    mission[0x359..0x359 + on_success.len()].copy_from_slice(on_success);
    mission
}

/// `mission` with this raw `OnAccept`.
fn accepts_into(mut mission: Vec<u8>, on_accept: &[u8]) -> Vec<u8> {
    mission[0x15B..0x15B + on_accept.len()].copy_from_slice(on_accept);
    mission
}

/// `mission` with the `DumpCargoText` `dësc` 205.
fn dumping(mut mission: Vec<u8>) -> Vec<u8> {
    put_i16s(&mut mission, 0x3A, &[205]);
    mission
}

/// `mission` with the `FailText` `dësc` 206.
fn failing(mut mission: Vec<u8>) -> Vec<u8> {
    put_i16s(&mut mission, 0x3E, &[206]);
    mission
}

/// `mission` with the `ShipDoneText` `dësc` 207.
fn ship_done(mut mission: Vec<u8>) -> Vec<u8> {
    put_i16s(&mut mission, 0x44, &[207]);
    mission
}

/// `mission` picking its cargo up at its destination (`PickupMode` 1)
/// and dropping it off at its return (`DropOffMode` 1), with the
/// `LoadCargText` `dësc` 208.
fn picking_up(mut mission: Vec<u8>) -> Vec<u8> {
    put_i16s(&mut mission, 0x14, &[1, 1]);
    put_i16s(&mut mission, 0x38, &[208]);
    mission
}

/// The run's `Q`: `STR#` 25048.
const LEAVE: &[u8] = b"Q25048\0";

/// The game data with this run and bar job, `STR#` 25048 holding "Off
/// you go.", and the `dësc`s 205 ("Unloaded at <DST>."), 206
/// ("Failed."), 207 ("Ships seen off.") and 208 ("Loaded at <DST>.").
fn leaving_data(run: &[u8], bar_job: (Vec<u8>, &str)) -> Rc<GameData> {
    game_data_built(run, bar_job, "Take this to <DST>.", |fork| {
        fork.resource(StrList::TYPE, 25048, None, &str_list(&["Off you go."]))
            .resource(Desc::TYPE, 205, None, &desc("Unloaded at <DST>."))
            .resource(Desc::TYPE, 206, None, &desc("Failed."))
            .resource(Desc::TYPE, 207, None, &desc("Ships seen off."))
            .resource(Desc::TYPE, 208, None, &desc("Loaded at <DST>."))
    })
}

/// The run, going to the Pad and back, its `OnSuccess` the `Q`.
fn leaving_run() -> Vec<u8> {
    succeeds_into(mission(128, 200, 201), LEAVE)
}

/// `pilot` with `field` of the mission it lists `at` set to `value`, as
/// its save lists them.
fn with_mission_field(pilot: &Pilot, at: usize, field: &str, value: serde_json::Value) -> Pilot {
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(pilot)).expect("a save");
    save["missions"][at][field] = value;
    nova_sim::save::decode(&save.to_string()).expect("a pilot")
}

/// Ada holding the run in slot 0, at the Pad and back.
fn running() -> Pilot {
    holding(&ada(), &[(0, 128, None, Some(128))])
}

/// Whether the session holds a `Q`'s leave.
fn leave_due(game: &Game) -> bool {
    game.app.screen().flight_view().expect("flying").leave_due()
}

#[test]
fn a_landing_qs_comp_text_shows_over_the_port_and_the_ship_leaves_once_it_closes() {
    let store = MemoryPilots::new();
    let data = leaving_data(&leaving_run(), bar_job());
    let mut game = Game::with_data(&store, &running(), data);
    game.frame();
    let writes = store.writes();
    game.land();

    // The run completes, and its CompText shows over the spaceport.
    assert_eq!(game.shown_text().as_deref(), Some("Thanks from Pad."));
    assert_eq!(game.active(), []);
    assert!(game.session().landed().is_some(), "still landed");
    assert!(leave_due(&game));
    assert_eq!(store.writes(), writes + 1, "the landing's save");
    let shown = texts(&game.frame());
    assert!(shown.contains(&"Thanks from Pad.".to_owned()), "{shown:?}");

    // Done leaves, as the port's Leave does, the Q's message the
    // take-off line.
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.session().landed(), None);
    assert!(!leave_due(&game));
    assert_eq!(game.flight_message().as_deref(), Some("Off you go."));
    assert_eq!(store.writes(), writes + 2, "and the take-off's");
}

#[test]
fn a_q_leaves_take_off_settles_its_deadline_before_the_save_in_the_same_input() {
    let store = MemoryPilots::new();
    let (job, offer) = bar_job();
    let data = leaving_data(&leaving_run(), (timed(job), offer));
    let pilot = holding(
        &ada(),
        &[(0, 128, None, Some(128)), (1, 130, Some(0), None)],
    );
    let mut game = Game::with_data(&store, &pilot, data);
    game.frame();
    let writes = store.writes();
    game.land();
    assert_eq!(game.shown_text().as_deref(), Some("Thanks from Pad."));
    assert_eq!(game.active(), [(1, MissionId(130))], "not failed yet");

    // Done leaves; the take-off's TookOff fails the bar job's deadline,
    // in that input, before the take-off's save and the first frame,
    // as the port's Leave does.
    game.key(Key::Enter, true);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.active(), [], "the deadline failed it");
    assert_eq!(store.writes(), writes + 2, "the landing's and take-off's");
    let missions: Vec<_> = saved(&store)
        .missions()
        .iter()
        .map(|(slot, active)| (slot, active.mission))
        .collect();
    assert_eq!(missions, [], "saved failed");
    game.key(Key::Enter, false);
    assert_eq!(store.writes(), writes + 2, "nothing left to save");
}

#[test]
fn a_q_leaves_take_off_shows_the_text_its_deadline_queues_over_flight_before_a_frame() {
    let store = MemoryPilots::new();
    let (job, offer) = bar_job();
    let job = fails_into(timed(job), b"S130\0");
    let data = leaving_data(&leaving_run(), (job, offer));
    let pilot = holding(
        &ada(),
        &[(0, 128, None, Some(128)), (1, 130, Some(0), None)],
    );
    let mut game = Game::with_data(&store, &pilot, data);
    game.frame();
    game.land();
    assert_eq!(game.shown_text().as_deref(), Some("Thanks from Pad."));

    // Done leaves; the take-off's TookOff fails the bar job's deadline,
    // and the BriefText its OnFailure's S130 queues shows over flight in
    // that input, before any frame.
    game.key(Key::Enter, true);
    assert_eq!(game.session().landed(), None, "left");
    assert_eq!(game.showing(), Showing::MissionText);
    assert_eq!(game.flight_text().as_deref(), Some("Welcome aboard."));
}

#[test]
fn a_landing_shows_the_cargo_dropped_off_then_the_comp_text_before_a_q_leaves() {
    let store = MemoryPilots::new();
    let data = leaving_data(&dumping(leaving_run()), bar_job());
    let pilot = with_mission_field(&running(), 0, "aboard", true.into());
    let pilot = with_mission_field(&pilot, 0, "cargo_tons", 1.into());
    let mut game = Game::with_data(&store, &pilot, data);
    game.frame();
    game.land();
    assert_eq!(game.shown_text().as_deref(), Some("Unloaded at Pad."));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(game.shown_text().as_deref(), Some("Thanks from Pad."));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.flight_message().as_deref(), Some("Off you go."));
}

#[test]
fn a_later_slots_fail_text_shows_after_a_qs_comp_text_before_the_leave() {
    let store = MemoryPilots::new();
    let (job, offer) = bar_job();
    let data = leaving_data(&leaving_run(), (failing(job), offer));
    let pilot = holding(
        &ada(),
        &[(0, 128, None, Some(128)), (1, 130, None, Some(128))],
    );
    let pilot = with_mission_field(&pilot, 1, "failed", "operator".into());
    let mut game = Game::with_data(&store, &pilot, data);
    game.frame();
    game.land();
    assert_eq!(game.shown_text().as_deref(), Some("Thanks from Pad."));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(game.shown_text().as_deref(), Some("Failed."));
    assert_eq!(game.active(), []);
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.flight_message().as_deref(), Some("Off you go."));
}

#[test]
fn a_landing_shows_the_ship_done_text_then_the_comp_text_before_a_q_leaves() {
    let store = MemoryPilots::new();
    let data = leaving_data(&ship_done(leaving_run()), bar_job());
    let mut game = Game::with_data(&store, &running(), data);
    game.frame();
    game.land();
    assert_eq!(game.shown_text().as_deref(), Some("Ships seen off."));
    assert!(leave_due(&game));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(game.shown_text().as_deref(), Some("Thanks from Pad."));
    assert!(leave_due(&game));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.flight_message().as_deref(), Some("Off you go."));
}

/// Ada holding the run in slot 0 and, in slot 1, the bar job picking up
/// `tons` of cargo at the Pad, with no return. Her ship has no holds, so
/// none fits and more is refused.
fn picking_up_at_the_pad(tons: i32) -> Pilot {
    let pilot = holding(
        &ada(),
        &[(0, 128, None, Some(128)), (1, 130, None, Some(128))],
    );
    let pilot = with_mission_field(&pilot, 1, "return", serde_json::Value::Null);
    with_mission_field(&pilot, 1, "cargo_tons", tons.into())
}

#[test]
fn a_later_slots_pickup_shows_its_load_carg_text_after_a_qs_comp_text_before_the_leave() {
    let store = MemoryPilots::new();
    let (job, offer) = bar_job();
    let data = leaving_data(&leaving_run(), (picking_up(job), offer));
    let mut game = Game::with_data(&store, &picking_up_at_the_pad(0), data);
    game.frame();
    game.land();
    assert_eq!(game.shown_text().as_deref(), Some("Thanks from Pad."));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(game.shown_text().as_deref(), Some("Loaded at Pad."));
    assert!(leave_due(&game));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.active(), [(1, MissionId(130))], "carrying it on");
    assert_eq!(game.flight_message().as_deref(), Some("Off you go."));
}

#[test]
fn a_later_slots_pickup_refused_for_room_says_so_after_a_qs_comp_text_before_the_leave() {
    let store = MemoryPilots::new();
    let (job, offer) = bar_job();
    let data = leaving_data(&leaving_run(), (picking_up(job), offer));
    let mut game = Game::with_data(&store, &picking_up_at_the_pad(1), data);
    game.frame();
    game.land();
    assert_eq!(game.shown_text().as_deref(), Some("Thanks from Pad."));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(
        game.shown_text().as_deref(),
        Some("Your ship doesn't have enough cargo space to load this cargo.")
    );
    assert!(leave_due(&game));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.flight_message().as_deref(), Some("Off you go."));
}

/// Lands Ada, holding the run, where `job`, as mission 130, is offered
/// at the spaceport, and closes the run's `CompText`: the job's offer
/// shows.
fn offered_after_the_comp_text(store: &MemoryPilots, (job, offer): (Vec<u8>, &str)) -> Game {
    let data = leaving_data(&leaving_run(), (at_venue(job, 3), offer));
    let mut game = Game::with_data(store, &running(), data);
    game.frame();
    game.land();
    assert_eq!(game.shown_text().as_deref(), Some("Thanks from Pad."));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Spaceport, "the offer holds it");
    let shown = game
        .spaceport()
        .offer_dialog()
        .map(|offer| offer.prompt().mission);
    assert_eq!(shown, Some(MissionId(130)));
    game
}

#[test]
fn the_ports_offer_shows_after_the_texts_and_a_refusal_leaves() {
    let store = MemoryPilots::new();
    let mut game = offered_after_the_comp_text(&store, bar_job());
    game.tap(Key::Char('n'));
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.active(), []);
    assert_eq!(game.flight_message().as_deref(), Some("Off you go."));
}

#[test]
fn the_ports_offer_refused_shows_its_refuse_text_then_leaves() {
    let store = MemoryPilots::new();
    let job = bar_mission(202, 203, b"I'm in\0", b"\0");
    let mut game = offered_after_the_comp_text(&store, (job, "<PN>, join us."));
    game.tap(Key::Char('n'));
    assert_eq!(
        game.showing(),
        Showing::Spaceport,
        "its RefuseText holds it"
    );
    assert_eq!(game.shown_text().as_deref(), Some("Suit yourself."));
    assert!(leave_due(&game));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.active(), []);
    assert_eq!(game.flight_message().as_deref(), Some("Off you go."));
}

#[test]
fn the_ports_offer_accepted_shows_its_brief_text_then_leaves() {
    let store = MemoryPilots::new();
    let mut game = offered_after_the_comp_text(&store, bar_job());
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(game.active(), [(0, MissionId(130))]);
    assert_eq!(game.shown_text().as_deref(), Some("Welcome aboard."));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.flight_message().as_deref(), Some("Off you go."));
}

#[test]
fn an_accept_whose_on_accept_leaves_shows_its_brief_text_then_leaves() {
    let store = MemoryPilots::new();
    let run = accepts_into(mission(128, 200, 201), LEAVE);
    let data = leaving_data(&run, bar_job());
    let mut game = Game::with_data(&store, &ada(), data);
    game.frame();
    game.land();
    game.click_port(MISSION_BBS);
    game.accept();
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(game.shown_text().as_deref(), Some("Off you go to Pad."));
    assert!(leave_due(&game));
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.active(), [(0, MissionId(128))]);
    assert_eq!(game.flight_message().as_deref(), Some("Off you go."));
}

#[test]
fn without_dialogs_a_landing_q_leaves_in_the_landings_input() {
    let store = MemoryPilots::new();
    let data = leaving_data(&leaving_run(), bar_job());
    PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>)
        .save(&running())
        .expect("saved");
    let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
    let screen: AppScreen = start_screen(Rc::clone(&data))
        .with_pilots(Some(keeper), Rc::new(MonoMetrics))
        .with_chance(SharedChance::new(Rc::new(RefCell::new(Nothing))));
    let mut game = Game {
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
    game.tap(Key::Enter);
    game.enter_ship();
    assert_eq!(game.showing(), Showing::Flight);
    game.frame();
    game.tap(Key::Char('l'));
    game.approach();
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Flight, "left at once");
    assert_eq!(game.active(), [], "the run completed");
    assert_eq!(game.session().landed(), None);
    assert_eq!(game.flight_message().as_deref(), Some("Off you go."));
}

// The map key on the mission dialogs: each opens flight's course map over
// itself, marked for its mission, and closes back to it.

/// `mission` marking its destination green on its dialogs' maps: `Flags`
/// 0x0100.
fn marking(mut mission: Vec<u8>) -> Vec<u8> {
    put_i16s(&mut mission, 0x50, &[0x0100]);
    mission
}

/// [`game_data`] with the run, mission 128, [`marking`] its destination.
fn marked_data() -> Rc<GameData> {
    game_data_from(&marking(timed(mission(128, 200, 201))), bar_job())
}

impl Game {
    /// Whether a mission dialog's map is open, and whether it draws a
    /// green arrow.
    fn dialog_map(&self) -> (bool, bool) {
        let flight = self.app.screen().flight_view().expect("flying");
        let green = flight
            .course_map()
            .picture()
            .is_some_and(|picture| picture.systems.iter().any(|system| system.green_arrow));
        (flight.dialog_map_open(), green)
    }
}

#[test]
fn m_on_the_mission_computer_opens_its_offers_map_and_closes_back_to_it() {
    let store = MemoryPilots::new();
    let mut game = Game::with_data(&store, &ada(), marked_data());
    game.frame();
    game.land();
    game.click_port(MISSION_BBS);
    game.tap(Key::Char('m'));
    assert_eq!(game.showing(), Showing::FlightMap);
    assert_eq!(game.dialog_map(), (true, true), "the Pad's system green");
    game.tap(Key::Char('m'));
    assert_eq!(game.showing(), Showing::Spaceport);
    assert!(game.spaceport().mission_computer().is_some(), "back to it");
}

#[test]
fn m_on_the_briefing_after_an_accept_opens_its_map_and_closes_back_to_the_text() {
    let store = MemoryPilots::new();
    let mut game = Game::with_data(&store, &ada(), marked_data());
    game.frame();
    game.land();
    game.click_port(MISSION_BBS);
    game.accept();
    assert_eq!(game.shown_text().as_deref(), Some("Off you go to Pad."));
    game.tap(Key::Char('m'));
    assert_eq!(game.showing(), Showing::FlightMap);
    assert_eq!(game.dialog_map(), (true, true));
    game.tap(Key::Char('m'));
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(
        game.shown_text().as_deref(),
        Some("Off you go to Pad."),
        "the briefing still open"
    );
}

#[test]
fn m_on_the_bars_offer_opens_its_map_and_closes_back_to_the_offer() {
    let store = MemoryPilots::new();
    let mut game = Game::with_pilot(&store, &ada());
    game.frame();
    game.land();
    game.click_port(BAR);
    assert_eq!(game.await_offer(), Some(MissionId(130)));
    game.tap(Key::Char('m'));
    assert_eq!(game.showing(), Showing::FlightMap);
    assert_eq!(game.dialog_map(), (true, false), "the offer's, unmarked");
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Spaceport);
    assert!(game.offer().is_some(), "the offer still open");
}

#[test]
fn over_flight_m_on_mission_info_opens_its_map_and_escape_returns_to_it() {
    let store = MemoryPilots::new();
    let pilot = holding(&ada(), &[(0, 128, None, Some(128))]);
    let mut game = Game::with_data(&store, &pilot, marked_data());
    game.frame();
    game.tap(Key::Char('i'));
    assert_eq!(game.showing(), Showing::MissionInfo);
    game.tap(Key::Char('m'));
    assert_eq!(game.showing(), Showing::FlightMap);
    assert_eq!(game.dialog_map(), (true, true), "the run's system green");
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::MissionInfo, "back to it");
    assert!(game.app.screen().flight_mission_info().is_some());
}

crate::common::entry_harness!(Game, click, frame);
