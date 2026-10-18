//! The bar's news through the whole app: synthetic game data (a `chär`
//! flying the "Boarder" from Alpha, whose one stellar, a pad at the
//! centre with a bar, `spöb` flags 0x41; the bar's text; `STR#` 8100 with
//! two commercials and `STR#` 8101 with two stories, each holding a
//! tag-like `<PN>`) and a synthetic interface file holding the stock
//! "Spaceport", "Bar" and "News" (`DLOG` 1014, 300 x 230, no items)
//! dialogs, driven only by window events, with pilots kept in the
//! in-memory store. The app draws on a [`Dial`] the test turns, so each
//! draw gives the outcome the dial is set to.
//!
//! The pilot lands and opens the bar, whose News button (Holovid), or N,
//! shows the commercial and the story over the bar, `<PN>` left as
//! written; Escape closes it back to the bar, and News again shows the
//! same two strings though the dial has turned. Leaving the bar and
//! opening it again draws the other two.

use std::cell::{Cell, RefCell};
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::desc::Desc;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::govt::Govt;
use nova_data::records::interface::Interface;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, InterfaceData, Record};
use nova_render::recording::{GpuCall, RecordingGpu};
use nova_render::{Batch, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};
use nova_sim::fixture::MemoryPilots;
use nova_sim::{Chance, Pilot, PilotKeeper, PilotStore};
use nova_view::Key;
use nova_view::flight::SharedChance;
use nova_view::geometry::Point;
use nova_view::menu::MenuChoice;
use nova_view::text::fixture::MonoMetrics;

/// The commercials, `STR#` 8100.
const COMMERCIALS: [&str; 2] = [
    "Bibble: the drink of champions.",
    "Fizz: the drink of runners-up.",
];
/// The stories, `STR#` 8101, each with a tag the news leaves as written.
const STORIES: [&str; 2] = ["<PN> wins the Nova Cup.", "<PN> loses the Nova Cup."];

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
    bytes[0x30..0x34].copy_from_slice(&5000_i32.to_be_bytes());
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

/// Alpha, a `sÿst` at the map's centre with the pad, independent, with no
/// traffic.
fn system() -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[128]);
    put_i16s(&mut bytes, 0x44, &[-1; 8]);
    put_i16s(&mut bytes, 0x64, &[0, -1]);
    bytes
}

/// The pad: a `spöb` at the centre that can be landed on, with a bar
/// (`Flags` 0x41), drawn from `spïn` 1004, held by `gövt` `govt` (-1 for
/// independent).
fn pad(govt: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 4]);
    bytes[0x06..0x0A].copy_from_slice(&0x41_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x0C, &[7]);
    put_i16s(&mut bytes, 0x14, &[govt]);
    put_i16s(&mut bytes, 0x18, &[-1, -1]);
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

/// A `STR#` of `strings`.
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

/// A `gövt` with no classes, allies or enemies, whose news shows over
/// `PICT` `news_pic`.
fn govt(news_pic: i16) -> Vec<u8> {
    let mut bytes = vec![0; Govt::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x18, &[-1; 12]);
    put_i16s(&mut bytes, 0xAE, &[news_pic]);
    bytes
}

/// The news pictures' sizes, which tell them apart once uploaded: `PICT`
/// 9000, the default, and 9001, the Federation's.
const DEFAULT_NEWS: (u32, u32) = (9, 3);
const FEDERATION_NEWS: (u32, u32) = (11, 7);

/// The game data (see the module docs), the pad held by `gövt` `govt`
/// (-1 for independent).
fn game_data(govt_id: i16) -> Rc<GameData> {
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Boarder"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system())
        .resource(Stellar::TYPE, 128, Some(b"Pad"), &pad(govt_id))
        .resource(Govt::TYPE, 128, Some(b"Federation"), &govt(9001))
        .resource(PICT, 9000, None, &pict(9, 3, [10, 20, 30]))
        .resource(PICT, 9001, None, &pict(11, 7, [30, 20, 10]))
        .resource(Spin::TYPE, 1004, None, &spin(1000))
        .resource(RLED, 1000, None, &sheet(1, 40))
        .resource(Desc::TYPE, 10_000, None, &desc("Smoke and quiet talk."))
        .resource(StrList::TYPE, 8100, None, &str_list(&COMMERCIALS))
        .resource(StrList::TYPE, 8101, None, &str_list(&STORIES))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &status_bar(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &pict(194, 16, [66, 66, 66]));
    for id in [8500, 8501, 8503, 10_004] {
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
/// the `DITL` of `items`, each (left, top, width, height) and enabled or
/// not.
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

/// The interface file: stock "Spaceport" (`DLOG` 1000), "Bar" (1013)
/// and "News" (1014, with no items), each with its items as stock has
/// them.
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
    let news = dialog(1014, (300, 230), &[]);
    let fork = [spaceport, bar, news]
        .into_iter()
        .flatten()
        .fold(ForkBuilder::new(), |fork, (ty, id, bytes)| {
            fork.resource(ty, id, None, &bytes)
        })
        .build()
        .bytes;
    InterfaceData::load(&OneFile(fork), Path::new("/Nova-DF.rsrc")).expect("loads")
}

/// Never fires; every draw is the outcome the dial is set to, or the
/// last when there are fewer.
struct Dial(Rc<Cell<u32>>);

impl Chance for Dial {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0.get().min(n - 1)
    }
}

struct Game {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
    /// The outcome the app's draws give.
    dial: Rc<Cell<u32>>,
}

impl Game {
    /// The app with a new pilot saved in a fresh store and opened from
    /// the main menu, in flight at Alpha's centre.
    fn flying() -> Self {
        Self::flying_at(-1)
    }

    /// As [`Game::flying`], the pad held by `gövt` `govt`.
    fn flying_at(govt: i16) -> Self {
        let data = game_data(govt);
        let store = MemoryPilots::new();
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        keeper
            .save(&Pilot::new(data.as_ref(), "Ada").expect("a pilot"))
            .expect("saved");
        let dial = Rc::new(Cell::new(0));
        let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Dial(Rc::clone(&dial))));
        let screen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), Rc::new(MonoMetrics))
            .with_dialogs(Rc::new(interface()), Rc::new(MonoMetrics))
            .with_chance(SharedChance::new(chance));
        let mut game = Self {
            app: App::new(&FakeWindow, data, crate::common::appeared(screen)),
            gpu: RecordingGpu::new(),
            frames: 0,
            dial,
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
        game.click(crate::common::enter_ship_at(game.app.screen()));
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

    fn tap(&mut self, key: Key) {
        for pressed in [true, false] {
            self.send(WindowEvent::Key {
                key,
                pressed,
                repeat: false,
            });
        }
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

    /// Lands at the pad: L requests clearance, a few frames let the dock
    /// approach clear it, and a second L lands.
    fn land(&mut self) {
        self.frame();
        self.tap(Key::Char('l'));
        self.approach();
        self.tap(Key::Char('l'));
        assert_eq!(self.showing(), Showing::Spaceport);
    }

    /// Clicks the spaceport's Bar (item 10).
    fn open_bar(&mut self) {
        let bar = self
            .app
            .screen()
            .spaceport_view()
            .expect("landed")
            .dialog()
            .expect("laid out")
            .item_bounds(10)
            .expect("the Bar")
            .center();
        self.click(bar);
        let bar = self
            .app
            .screen()
            .spaceport_view()
            .and_then(|spaceport| spaceport.open_bar())
            .expect("in the bar");
        assert_eq!(bar.problem(), None);
    }

    /// Clicks the bar's News button, Holovid (item 3).
    fn click_news(&mut self) {
        let at = self
            .app
            .screen()
            .spaceport_view()
            .and_then(|spaceport| spaceport.open_bar())
            .and_then(|bar| bar.dialog())
            .and_then(|dialog| dialog.item_bounds(3))
            .expect("News")
            .center();
        self.click(at);
    }

    /// Whether the news is open over the bar.
    fn reading(&self) -> bool {
        self.app
            .screen()
            .spaceport_view()
            .and_then(|spaceport| spaceport.open_bar())
            .is_some_and(|bar| bar.news_screen().is_some())
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

/// Whether `frame` shows commercial `n` and story `n` and not the
/// others, `<PN>` as written, wrapped to the news dialog's lines.
fn shows_the_news(frame: &Frame, n: usize) -> bool {
    let shown = texts(frame).join(" ");
    let other = 1 - n;
    shown.contains(COMMERCIALS[n])
        && shown.contains(STORIES[n])
        && !shown.contains(COMMERCIALS[other])
        && !shown.contains(STORIES[other])
}

/// Whether `frame` shows any of the news.
fn shows_any_news(frame: &Frame) -> bool {
    let shown = texts(frame).join(" ");
    COMMERCIALS
        .iter()
        .chain(&STORIES)
        .any(|text| shown.contains(text))
}

#[test]
fn the_bars_news_shows_the_commercial_and_the_story_as_written() {
    let mut game = Game::flying();
    game.land();
    game.open_bar();
    let shown = texts(&game.frame());
    assert!(!shows_any_news(&game.frame()), "not before News: {shown:?}");
    game.dial.set(1);
    game.tap(Key::Char('n'));
    assert!(game.reading());
    let frame = game.frame();
    assert!(
        shows_the_news(&frame, 0),
        "drawn as the bar opened: {:?}",
        texts(&frame)
    );
    assert!(
        !texts(&frame).contains(&"Smoke and quiet talk.".to_owned()),
        "the news covers the bar"
    );

    // Escape closes it back to the bar, and News again shows the same
    // two strings, though the dial has turned.
    game.tap(Key::Escape);
    assert!(!game.reading());
    assert_eq!(game.showing(), Showing::Spaceport);
    let shown = texts(&game.frame());
    assert!(
        shown.contains(&"Smoke and quiet talk.".to_owned()),
        "{shown:?}"
    );
    game.click_news();
    assert!(game.reading());
    let frame = game.frame();
    assert!(
        shows_the_news(&frame, 0),
        "the same visit: {:?}",
        texts(&frame)
    );
}

#[test]
fn opening_the_bar_again_draws_the_news_afresh() {
    let mut game = Game::flying();
    game.land();
    game.open_bar();
    game.tap(Key::Char('n'));
    let frame = game.frame();
    assert!(shows_the_news(&frame, 0), "{:?}", texts(&frame));

    // Closing the news and then the bar, and opening the bar again with
    // the dial turned, draws the other two.
    game.tap(Key::Escape);
    game.tap(Key::Escape);
    assert!(
        game.app
            .screen()
            .spaceport_view()
            .and_then(|spaceport| spaceport.open_bar())
            .is_none(),
        "left the bar"
    );
    game.dial.set(1);
    game.open_bar();
    game.click_news();
    assert!(game.reading());
    let frame = game.frame();
    assert!(
        shows_the_news(&frame, 1),
        "a new visit: {:?}",
        texts(&frame)
    );
}

/// The size of the picture stretched over the 300 x 230 news dialog in
/// `frame`, told apart by the uploads `game` has made.
fn news_backdrop(game: &Game, frame: &Frame) -> Option<(u32, u32)> {
    let stretched = frame.batches.iter().any(|batch| match batch {
        Batch::Sprites { quads, .. } => quads
            .iter()
            .any(|quad| (quad.dest.w, quad.dest.h) == (300.0, 230.0)),
        _ => false,
    });
    let uploaded: Vec<(u32, u32)> = game
        .gpu
        .calls
        .iter()
        .filter_map(|call| match call {
            GpuCall::Upload { width, height, .. } => Some((*width, *height)),
            _ => None,
        })
        .filter(|size| [DEFAULT_NEWS, FEDERATION_NEWS].contains(size))
        .collect();
    match uploaded.as_slice() {
        [size] if stretched => Some(*size),
        _ => None,
    }
}

#[test]
fn the_news_shows_over_the_stellar_governments_picture() {
    let mut game = Game::flying_at(128);
    game.land();
    game.open_bar();
    game.tap(Key::Char('n'));
    let frame = game.frame();
    assert_eq!(news_backdrop(&game, &frame), Some(FEDERATION_NEWS));
}

#[test]
fn an_independent_stellars_news_shows_over_the_default_picture() {
    let mut game = Game::flying();
    game.land();
    game.open_bar();
    game.tap(Key::Char('n'));
    let frame = game.frame();
    assert_eq!(news_backdrop(&game, &frame), Some(DEFAULT_NEWS));
}
