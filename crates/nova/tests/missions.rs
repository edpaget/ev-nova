//! Missions through the whole app: synthetic game data (a `chär` with
//! 25,000 credits flying the "Boarder" from Alpha, whose one stellar, the
//! "Pad" at the centre, has a bar; mission 128, "Run to <DST>", offered
//! at the Pad's mission computer, going to the Pad and back, with an
//! offer text, a `BriefText` and a `CompText`; mission 129, "Errand",
//! going nowhere, with no text) and a synthetic interface file holding
//! the "Spaceport", the mission computer (`DLOG` 1006) and "Desc Dialog"
//! (3003), driven only by window events, with pilots kept in the
//! in-memory store.
//!
//! The mission computer: the pilot lands, and Mission BBS lists "Run to
//! Pad" and "Errand", the first one's text filled. Accept shows its
//! `BriefText`, filled, over the computer, which lists the Errand alone;
//! accepting that too closes the computer silently, nothing being left.
//! Both are active and saved. Taking off and landing again completes the
//! run, at its destination, and its `CompText` shows over the spaceport.

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
use nova_sim::{Chance, MissionId, Pilot, PilotKeeper, PilotStore, Session};
use nova_view::Key;
use nova_view::flight::SharedChance;
use nova_view::geometry::Point;
use nova_view::menu::MenuChoice;
use nova_view::spaceport::SpaceportView;
use nova_view::spaceport::missions::{ACCEPT_ITEM, MissionComputerScreen};
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

/// The game data (see the module docs) and the pictures the screens
/// draw.
fn game_data() -> Rc<GameData> {
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Boarder"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system())
        .resource(Stellar::TYPE, 128, Some(b"Pad"), &pad())
        .resource(Spin::TYPE, 1004, None, &spin(1000))
        .resource(RLED, 1000, None, &sheet(1, 40))
        .resource(
            Mission::TYPE,
            128,
            Some(b"Run to <DST>;courier"),
            &mission(128, 200, 201),
        )
        .resource(Mission::TYPE, 129, Some(b"Errand"), &mission(-1, -1, -1))
        .resource(Desc::TYPE, 4000, None, &desc("Take this to <DST>."))
        .resource(Desc::TYPE, 200, None, &desc("Off you go to <DST>."))
        .resource(Desc::TYPE, 201, None, &desc("Thanks from <DST>."))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &status_bar(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &pict(194, 16, [66, 66, 66]));
    for id in [8500, 8501, 8505, 8524, 8525, 8526, 10_004] {
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

/// The interface file: stock "Spaceport" (`DLOG` 1000), the mission
/// computer (1006) and "Desc Dialog" (3003), each with its items as stock
/// has them.
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
    let fork = [spaceport, computer, text]
        .into_iter()
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

struct Game {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
}

impl Game {
    /// The app with `pilot` saved in `store` and opened from the main
    /// menu, in flight.
    fn with_pilot(store: &MemoryPilots, pilot: &Pilot) -> Self {
        PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>)
            .save(pilot)
            .expect("saved");
        let data = game_data();
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Nothing));
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

/// Mission BBS's item.
const MISSION_BBS: usize = 11;

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
