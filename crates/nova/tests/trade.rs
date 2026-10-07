//! Trading through the whole app: synthetic game data (a `chär` with
//! 25,000 credits flying a ship with 10 tons of cargo space from a system
//! whose one planet, a trade center trading food and metal, it starts
//! over) and a synthetic interface file holding the stock "Create a new
//! pilot:", "Spaceport" and "Trade" dialogs, laid out by the real glyphon
//! metrics, drawn through the renderer into the recording Gpu and driven
//! only by window events, keys and typed text made by the platform's own
//! translation. Pilots are kept in the in-memory store, so the test sees
//! what is saved, and "restarts" with a new app over the same store.
//!
//! A new pilot lands, opens the Trade Center, which lists only the goods
//! the planet trades at its prices, and buys food until the hold is full;
//! each trade saves the pilot. In a new app the pilot resumes docked with
//! its cargo, and selling pays the local price.

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova::platform;
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, InterfaceData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_render::{Batch, FontFaces, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};
use nova_sim::fixture::MemoryPilots;
use nova_sim::{Good, Pilot, PilotKeeper, PilotStore};
use nova_view::MouseButton;
use nova_view::geometry::Point;
use nova_view::menu::MenuChoice;
use nova_view::spaceport::TradeScreen;
use winit::event::ElementState;
use winit::keyboard::{KeyCode, PhysicalKey};

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

/// One file, holding a fork, wherever it is looked for.
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

fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> OneFile {
    let bytes = resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
            fork.resource(*ty, *id, None, data)
        })
        .build()
        .bytes;
    OneFile(bytes)
}

fn be(values: &[i16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}

fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
    }
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

/// The starting credits.
const CASH: i32 = 25_000;

/// The `chär`: 25,000 credits, ship 128 in system 128 on 23 June 1177,
/// and no legal records.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    bytes[0x00..0x04].copy_from_slice(&CASH.to_be_bytes());
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    put_i16s(&mut bytes, 0x134, &[23, 6, 1177]);
    bytes
}

/// The ship's cargo space.
const HOLDS: i16 = 10;

/// An average `shïp` with shield, armour, fuel and 10 tons of cargo
/// space.
fn ship() -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[HOLDS, 30, 300, 300, 30, 300, 45]);
    bytes
}

/// A `shän` whose base image is `rlëD` 2000, one set of 36 rotations.
fn ship_anim() -> Vec<u8> {
    let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[2000, 0, 1]);
    put_i16s(&mut bytes, 0x34, &[36]);
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

/// An independent `sÿst` holding stellar 128 alone.
fn system() -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[128]);
    put_i16s(&mut bytes, 0x66, &[-1]);
    bytes
}

/// Alpha Prime: a landable planet at the system's centre, whose
/// landscape is `PICT` 10004, with a bar and a trade center trading food
/// at medium and metal at low.
fn stellar() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 4]);
    bytes[0x06..0x0A].copy_from_slice(&0x2000_1043_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x18, &[-1, -1]);
    bytes
}

/// A `STR#` of `strings`.
fn str_list(strings: &[&str]) -> Vec<u8> {
    let mut bytes = be(&[strings.len() as i16]);
    for string in strings {
        bytes.push(string.len() as u8);
        bytes.extend(string.as_bytes());
    }
    bytes
}

/// A `spïn` naming `rlëD` 1000, one frame across.
fn spin() -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[1000, -1, 0, 0, 1, 1]);
    bytes
}

fn game_data() -> Rc<GameData> {
    let mut resources = vec![
        (Character::TYPE, 128, character()),
        (Ship::TYPE, 128, ship()),
        (ShipAnim::TYPE, 128, ship_anim()),
        (RLED, 2000, sheet(36, 1)),
        (System::TYPE, 128, system()),
        (Stellar::TYPE, 128, stellar()),
        (Spin::TYPE, 1004, spin()),
        (RLED, 1000, sheet(1, 8)),
        (PICT, 8500, pict(6, 5, [40, 40, 40])),
        (PICT, 8510, pict(6, 5, [10, 10, 10])),
        (
            StrList::TYPE,
            4000,
            str_list(&[
                "Food",
                "Industrial",
                "Medical Supplies",
                "Luxury Goods",
                "Metal",
                "Equipment",
            ]),
        ),
        (
            StrList::TYPE,
            4004,
            str_list(&["75", "350", "750", "900", "200", "550"]),
        ),
        (PICT, 10_004, pict(6, 3, [0, 90, 0])),
    ];
    for state in [7500, 7503, 7506] {
        resources.push((PICT, state, pict(13, 25, [200, 0, 0])));
        resources.push((PICT, state + 1, pict(2, 25, [0, 200, 0])));
        resources.push((PICT, state + 2, pict(13, 25, [0, 0, 200])));
        resources.push((PICT, state + 100, pict(13, 25, [0, 0, 0])));
        resources.push((PICT, state + 102, pict(13, 25, [0, 0, 0])));
    }
    let file = fork(&resources);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

/// One `DITL` item: (left, top, right, bottom), its type byte and data.
fn ditl_item((l, t, r, b): (i16, i16, i16, i16), type_byte: u8, data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 4];
    bytes.extend(be(&[t, l, b, r]));
    bytes.push(type_byte);
    bytes.push(data.len() as u8);
    bytes.extend(data);
    if data.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes
}

const USER: u8 = 0;
const BUTTON: u8 = 4;
const CHECK_BOX: u8 = 5;
const CONTROL: u8 = 7;
const STATIC_TEXT: u8 = 8;
const EDIT_TEXT: u8 = 16;
const PICTURE: u8 = 64;
const DISABLED: u8 = 0x80;

/// A `DLOG` of `bounds` (top, left, bottom, right), centred, naming its
/// `DITL`.
fn dlog((t, l, b, r): (i16, i16, i16, i16), ditl: i16) -> Vec<u8> {
    let mut bytes = be(&[t, l, b, r, 1]);
    bytes.extend([1, 0, 0, 0, 0, 0, 0, 0]);
    bytes.extend(be(&[ditl]));
    bytes.extend([0, 0, 0xA8, 0x0A]);
    bytes
}

fn ditl(items: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = be(&[items.len() as i16 - 1]);
    bytes.extend(items.concat());
    bytes
}

/// Stock "Create a new pilot:" (`DLOG` 3102, 326 x 213, centred) and its
/// fourteen items, stock "Spaceport" (`DLOG` 1000) and its fifteen user
/// items, and stock "Trade" (`DLOG` 1001) and its seventeen user items.
fn interface() -> InterfaceData {
    let text = |bounds, s: &str| ditl_item(bounds, STATIC_TEXT | DISABLED, s.as_bytes());
    let field = |bounds| ditl_item(bounds, EDIT_TEXT, b"Edit Text");
    let new_pilot = [
        ditl_item((238, 183, 308, 203), BUTTON, b"OK"),
        ditl_item((156, 183, 226, 203), BUTTON, b"Cancel"),
        ditl_item((79, 141, 261, 163), PICTURE | DISABLED, &be(&[129])),
        ditl_item((59, 122, 154, 139), CHECK_BOX, b"Strict Play"),
        text((54, 36, 138, 52), "Full Name:"),
        text((54, 59, 133, 75), "Nickname:"),
        text((49, 357, 127, 373), "name3:"),
        field((149, 36, 319, 52)),
        field((149, 59, 319, 75)),
        field((144, 357, 314, 373)),
        ditl_item((51, 92, 251, 112), CONTROL, &be(&[500])),
        text((53, 6, 181, 22), "Create a new pilot:"),
        ditl_item((85, 277, 356, 297), CONTROL, &be(&[501])),
        ditl_item((7, 5, 39, 37), PICTURE | DISABLED, &be(&[130])),
    ];
    let spaceport: Vec<Vec<u8>> = [
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
    ]
    .into_iter()
    .map(|((x, y, w, h), enabled)| {
        let byte = if enabled { USER } else { USER | DISABLED };
        ditl_item((x, y, x + w, y + h), byte, &[])
    })
    .collect();
    let trade: Vec<Vec<u8>> = [
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
    ]
    .into_iter()
    .map(|((x, y, w, h), enabled)| {
        let byte = if enabled { USER } else { USER | DISABLED };
        ditl_item((x, y, x + w, y + h), byte, &[])
    })
    .collect();
    let file = fork(&[
        (Dlog::TYPE, 1001, dlog((35, 32, 287, 458), 1001)),
        (Ditl::TYPE, 1001, ditl(&trade)),
        (Dlog::TYPE, 3102, dlog((78, 175, 291, 501), 3102)),
        (Ditl::TYPE, 3102, ditl(&new_pilot)),
        (Dlog::TYPE, 1000, dlog((-201, 60, 316, 678), 1000)),
        (Ditl::TYPE, 1000, ditl(&spaceport)),
    ]);
    InterfaceData::load(&file, Path::new("/Nova-DF.rsrc")).expect("loads")
}

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
}

impl Harness {
    /// The app on the main menu, keeping pilots in `store`, before any
    /// frame.
    fn opening(store: &MemoryPilots) -> Self {
        let data = game_data();
        let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        let screen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), metrics.clone())
            .with_dialogs(Rc::new(interface()), metrics);
        Self {
            app: App::new(&FakeWindow, data, screen),
            gpu: RecordingGpu::new(),
            frames: 0,
        }
    }

    fn handle(&mut self, event: WindowEvent) -> Control {
        self.app.handle(event, &mut FakeWindow, &mut self.gpu)
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(self.handle(event), Control::Continue, "{event:?}");
    }

    /// Presses and releases the physical key `code`, typing `text` (as
    /// winit reports it) on the press, through the platform's translation:
    /// the key, then its text.
    fn type_key(&mut self, code: KeyCode, text: Option<&str>) {
        for state in [ElementState::Pressed, ElementState::Released] {
            self.send(platform::key_event(PhysicalKey::Code(code), state, false));
            if let Some(typed) = platform::text_event(text, state) {
                self.send(typed);
            }
        }
    }

    /// Types `name`, each letter on its own key.
    fn type_name(&mut self, name: &str) {
        for c in name.chars() {
            let code = match c.to_ascii_lowercase() {
                'a' => KeyCode::KeyA,
                'd' => KeyCode::KeyD,
                'l' => KeyCode::KeyL,
                'p' => KeyCode::KeyP,
                'x' => KeyCode::KeyX,
                ' ' => KeyCode::Space,
                other => panic!("no key listed for {other:?}"),
            };
            self.type_key(code, Some(&c.to_string()));
        }
    }

    fn press(&mut self, code: KeyCode) {
        self.type_key(code, None);
    }

    /// Presses L twice, as the player lands: the first requests clearance,
    /// the second lands.
    fn land(&mut self) {
        self.press(KeyCode::KeyL);
        self.press(KeyCode::KeyL);
    }

    fn click(&mut self, at: Point) {
        self.send(WindowEvent::PointerMoved {
            px: (f64::from(at.x), f64::from(at.y)),
        });
        for pressed in [true, false] {
            self.send(WindowEvent::PointerButton {
                button: MouseButton::Left,
                pressed,
            });
        }
    }

    /// Where the main menu's `choice` button is.
    fn menu_button(&self, choice: MenuChoice) -> Point {
        self.app
            .screen()
            .main_menu()
            .expect("a main menu")
            .button(choice)
            .rect
            .center()
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn pilot(&self) -> Pilot {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .pilot()
            .expect("a pilot")
            .clone()
    }

    fn frame(&mut self) -> Frame {
        self.frames += 1;
        self.send(WindowEvent::Redraw {
            elapsed: Duration::from_millis(16 * self.frames),
        });
        assert_eq!(self.app.take_failures(), []);
        assert_eq!(self.app.take_warnings(), Vec::<String>::new());
        (*self.gpu.submits().last().expect("a frame")).clone()
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

fn saved(store: &MemoryPilots, key: &str) -> Pilot {
    nova_sim::save::decode(&store.text(key).expect("saved")).expect("a pilot")
}

const FOOD: Good = Good::Commodity(0);

impl Harness {
    /// Sends the physical key `code` going down (a repeat when `repeat`)
    /// or, when not `pressed`, up.
    fn key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        let state = if pressed {
            ElementState::Pressed
        } else {
            ElementState::Released
        };
        self.send(platform::key_event(PhysicalKey::Code(code), state, repeat));
    }

    /// Where spaceport item `number` is.
    fn spaceport_item(&self, number: usize) -> Point {
        self.app
            .screen()
            .spaceport_view()
            .expect("landed")
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
            .center()
    }

    /// The Trade Center, open.
    fn trade(&self) -> &TradeScreen {
        self.app
            .screen()
            .spaceport_view()
            .expect("landed")
            .open_trade()
            .expect("trading")
    }

    /// Clicks the spaceport's Trade Center.
    fn open_trade_center(&mut self) {
        let trade_center = self.spaceport_item(7);
        self.click(trade_center);
        assert!(self.trade().problem().is_none());
        assert_eq!(self.showing(), Showing::Spaceport);
    }
}

/// The status bar's text for `free` tons and `cash` credits.
fn status(free: u32, cash: i64) -> String {
    format!("Free cargo space: {free} tons    Credits: {cash}")
}

#[test]
fn a_pilot_buys_until_the_hold_is_full_and_sells_its_cargo_after_a_restart() {
    let store = MemoryPilots::new();
    let mut game = Harness::opening(&store);
    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    game.type_name("Ada");
    game.press(KeyCode::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    game.frame();
    game.land();
    assert_eq!(game.showing(), Showing::Spaceport);

    // The Trade Center lists only what the planet trades, at its levels.
    game.open_trade_center();
    let cash = i64::from(CASH);
    let shown = texts(&game.frame());
    for text in ["Food", "75", "Metal", "160", "Buy", "Sell", "Done"] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }
    assert!(shown.contains(&status(10, cash)), "{shown:?}");
    for untraded in [
        "Industrial",
        "Medical Supplies",
        "Luxury Goods",
        "Equipment",
    ] {
        assert!(!shown.contains(&untraded.to_owned()), "{untraded}");
    }

    // B buys a ton, and the pilot is saved.
    let writes = store.writes();
    game.press(KeyCode::KeyB);
    assert_eq!(game.pilot().held(FOOD), 1);
    assert_eq!(game.pilot().cash(), cash - 75);
    assert_eq!(store.writes(), writes + 1);
    assert_eq!(saved(&store, "Ada").held(FOOD), 1);

    // Holding B keeps buying until the hold is full, then buys nothing.
    game.key(KeyCode::KeyB, true, false);
    for _ in 0..15 {
        game.key(KeyCode::KeyB, true, true);
    }
    game.key(KeyCode::KeyB, false, false);
    assert_eq!(game.pilot().held(FOOD), 10);
    let full = cash - 10 * 75;
    assert_eq!(game.pilot().cash(), full);
    assert!(texts(&game.frame()).contains(&status(0, full)));
    assert_eq!(game.trade().market().free, 0);
    let writes = store.writes();
    game.key(KeyCode::AltLeft, true, false);
    game.press(KeyCode::KeyB);
    game.key(KeyCode::AltLeft, false, false);
    assert_eq!(game.pilot().held(FOOD), 10, "no space left");
    assert_eq!(store.writes(), writes, "nothing to save");

    // Done closes the exchange; closing the window saves the cargo.
    game.press(KeyCode::Escape);
    assert!(
        game.app
            .screen()
            .spaceport_view()
            .expect("landed")
            .open_trade()
            .is_none()
    );
    let landed: Pilot = game.pilot();
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Exit);
    assert_eq!(saved(&store, "Ada"), landed);

    // A new app resumes the pilot docked, with its cargo.
    let mut game = Harness::opening(&store);
    let open_pilot = game.menu_button(MenuChoice::OpenPilot);
    game.click(open_pilot);
    game.press(KeyCode::Enter);
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(game.pilot().held(FOOD), 10);
    game.open_trade_center();
    assert_eq!(
        game.trade().market().row(FOOD).map(|row| row.held),
        Some(10)
    );
    assert!(texts(&game.frame()).contains(&status(0, full)));

    // Selling pays the local price: a ton with S, the rest with Alt-S.
    game.press(KeyCode::KeyS);
    assert_eq!(game.pilot().cash(), full + 75);
    assert_eq!(game.pilot().held(FOOD), 9);
    game.key(KeyCode::AltRight, true, false);
    game.press(KeyCode::KeyS);
    game.key(KeyCode::AltRight, false, false);
    assert_eq!(game.pilot().held(FOOD), 0);
    assert_eq!(game.pilot().cash(), cash);
    assert!(texts(&game.frame()).contains(&status(10, cash)));
    assert_eq!(saved(&store, "Ada").cash(), cash);
}
