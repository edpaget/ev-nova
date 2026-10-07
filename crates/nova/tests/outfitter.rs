//! Outfitting through the whole app: synthetic game data (a `chär` with
//! 25,000 credits flying an average ship with 8 tons free from a system
//! whose one planet, an outfitter of tech level 2, it starts over, and four
//! outfits: a speed booster, a fuel tank and a fuel scoop it sells, and a
//! warp core of tech level 5 it does not) and a synthetic interface file
//! holding the stock "Create a new pilot:", "Spaceport" and "Outfit"
//! dialogs, laid out by the real glyphon metrics, drawn through the
//! renderer into the recording Gpu and driven only by window events, keys
//! and typed text made by the platform's own translation. Pilots are kept
//! in the in-memory store, so the test sees what is saved, and "restarts"
//! with a new app over the same store.
//!
//! A new pilot lands, opens the Outfitter, which lists only what the
//! planet's tech level allows, and buys speed boosters until it can have
//! no more, and a fuel tank; each order saves the pilot. It takes off
//! faster, with more fuel. In a new app the pilot's outfits are restored,
//! and so are the stats they give.

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova::platform;
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::desc::Desc;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::outfit::Outfit;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, InterfaceData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_render::{Batch, FontFaces, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};
use nova_sim::fixture::MemoryPilots;
use nova_sim::{OutfitId, Pilot, PilotKeeper, PilotStore};
use nova_view::MouseButton;
use nova_view::geometry::Point;
use nova_view::menu::MenuChoice;
use nova_view::spaceport::OutfitterScreen;
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

/// The ship's top speed, `Speed`, and its free mass, `FreeMass`.
const SPEED: i16 = 300;
const FREE_MASS: i16 = 8;

/// An average `shïp`: 10 tons of cargo space, shield 30, accel 300,
/// speed 300, maneuver 30, fuel 300, 8 tons free and armour 45.
fn ship() -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(
        &mut bytes,
        0x00,
        &[10, 30, 300, SPEED, 30, 300, FREE_MASS, 45],
    );
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
/// landscape is `PICT` 10004, with a bar, a trade center and an outfitter
/// of tech level 2.
fn stellar() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 4]);
    bytes[0x06..0x0A].copy_from_slice(&0x2000_1047_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x0C, &[2]);
    put_i16s(&mut bytes, 0x14, &[-1]);
    put_i16s(&mut bytes, 0x18, &[-1, -1]);
    bytes
}

/// An `oütf` of `mass` tons, tech level `tech`, one mod, up to `max`, at
/// `cost`, shown `weight`-high and named `short_name`.
fn outfit(
    weight: i16,
    mass: i16,
    tech: i16,
    (mod_type, mod_val): (i16, i16),
    max: i16,
    cost: i32,
    short_name: &str,
) -> Vec<u8> {
    let mut bytes = vec![0; Outfit::SIZE.expect("fixed")];
    put_i16s(
        &mut bytes,
        0x00,
        &[weight, mass, tech, mod_type, mod_val, max],
    );
    bytes[0x0E..0x12].copy_from_slice(&cost.to_be_bytes());
    put_i16s(&mut bytes, 0x12, &[0; 6]);
    put_i16s(&mut bytes, 0x3F2, &[-1]);
    bytes[0x32B..0x32B + short_name.len()].copy_from_slice(short_name.as_bytes());
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

/// The speed booster's ID: `oütf` 128, +300 speed, 3 tons, up to 2, for
/// 5000 credits.
const BOOSTER: OutfitId = OutfitId(128);
/// The fuel tank's ID: `oütf` 129, +100 fuel, a ton, for 2000 credits.
const TANK: OutfitId = OutfitId(129);

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
        (PICT, 8502, pict(6, 5, [10, 10, 10])),
        (PICT, 6000, pict(4, 4, [200, 200, 0])),
        (PICT, 10_004, pict(6, 3, [0, 90, 0])),
        (
            Outfit::TYPE,
            128,
            outfit(30, 3, 1, (8, 300), 2, 5000, "Speed\\nBooster"),
        ),
        (
            Outfit::TYPE,
            129,
            outfit(20, 1, 2, (12, 100), 5, 2000, "Fuel Tank"),
        ),
        (
            Outfit::TYPE,
            130,
            outfit(10, 1, 1, (18, 10), 1, 3000, "Fuel Scoop"),
        ),
        (
            Outfit::TYPE,
            131,
            outfit(40, 1, 5, (8, 900), 1, 100, "Warp Core"),
        ),
        (Desc::TYPE, 3000, desc("Bolted-on thrusters.")),
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
/// items, and stock "Outfit" (`DLOG` 1002, 765 x 321, centred) and its
/// thirteen user items.
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
    let outfit: Vec<Vec<u8>> = [
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
    ]
    .into_iter()
    .map(|((x, y, w, h), enabled)| {
        let byte = if enabled { USER } else { USER | DISABLED };
        ditl_item((x, y, x + w, y + h), byte, &[])
    })
    .collect();
    let file = fork(&[
        (Dlog::TYPE, 1002, dlog((100, 100, 421, 865), 1002)),
        (Ditl::TYPE, 1002, ditl(&outfit)),
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

    /// The Outfitter, open.
    fn outfitter(&self) -> &OutfitterScreen {
        self.app
            .screen()
            .spaceport_view()
            .expect("landed")
            .open_outfitter()
            .expect("outfitting")
    }

    /// Clicks the spaceport's Outfitter.
    fn open_outfitter(&mut self) {
        let outfitter = self.spaceport_item(8);
        self.click(outfitter);
        assert!(self.outfitter().problem().is_none());
        assert_eq!(self.showing(), Showing::Spaceport);
    }

    /// The flight session's top speed, in pixels a tick.
    fn top_speed(&self) -> f32 {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .session()
            .expect("a session")
            .handling()
            .max_speed
    }
}

#[test]
fn a_pilot_outfits_its_ship_flies_faster_and_keeps_its_outfits_after_a_restart() {
    let store = MemoryPilots::new();
    let mut game = Harness::opening(&store);
    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    game.type_name("Ada");
    game.press(KeyCode::Enter);
    assert_eq!(game.showing(), Showing::Flight);
    game.frame();
    let base_speed = game.top_speed();
    assert!((base_speed - 3.0).abs() < 1e-6, "{base_speed}");
    game.land();
    assert_eq!(game.showing(), Showing::Spaceport);

    // The Outfitter lists what tech level 2 allows, heaviest weight first.
    game.open_outfitter();
    let ids: Vec<_> = game
        .outfitter()
        .outfitter()
        .rows
        .iter()
        .map(|row| row.id)
        .collect();
    assert_eq!(ids, [BOOSTER, TANK, OutfitId(130)], "no warp core");
    let shown = texts(&game.frame());
    for text in [
        "Speed",
        "Booster",
        "Fuel Tank",
        "Fuel Scoop",
        "Bolted-on thrusters.",
        "Item Price: 5000",
        "Item Mass: 3 tons",
        "You Have: 0",
        "Available: 8 tons",
        "Buy",
        "Sell",
        "Done",
    ] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }
    assert!(!shown.contains(&"Warp Core".to_owned()));

    // Select the booster by its cell and buy one with B: the pilot pays
    // and is saved.
    let cell = game.outfitter().cell_bounds(0).expect("a cell").center();
    game.click(cell);
    let writes = store.writes();
    game.press(KeyCode::KeyB);
    assert_eq!(game.pilot().owned(BOOSTER), 1);
    assert_eq!(game.pilot().cash(), 20_000);
    assert_eq!(store.writes(), writes + 1);
    assert_eq!(saved(&store, "Ada").owned(BOOSTER), 1);
    let shown = texts(&game.frame());
    assert!(shown.contains(&"You Have: 1".to_owned()), "{shown:?}");
    assert!(shown.contains(&"Available: 5 tons".to_owned()));

    // Holding B buys until it can have no more, then nothing changes.
    game.key(KeyCode::KeyB, true, false);
    for _ in 0..5 {
        game.key(KeyCode::KeyB, true, true);
    }
    game.key(KeyCode::KeyB, false, false);
    assert_eq!(game.pilot().owned(BOOSTER), 2);
    assert_eq!(game.pilot().cash(), 15_000);
    assert!(texts(&game.frame()).contains(&"Can't have any more!".to_owned()));
    let writes = store.writes();
    let before = game.pilot();
    game.press(KeyCode::KeyB);
    assert_eq!(game.pilot(), before, "refused");
    assert_eq!(store.writes(), writes, "nothing to save");

    // The fuel tank, the next cell along, adds a jump of fuel.
    game.press(KeyCode::ArrowRight);
    game.press(KeyCode::KeyB);
    assert_eq!(game.pilot().owned(TANK), 1);
    assert_eq!(game.pilot().cash(), 13_000);
    let fuel = game.pilot().reserves().fuel;
    assert!((fuel.max - 400.0).abs() < 1e-3 && (fuel.now - 400.0).abs() < 1e-3);

    // Done, then Leave: the ship takes off faster.
    game.press(KeyCode::Escape);
    assert!(
        game.app
            .screen()
            .spaceport_view()
            .expect("landed")
            .open_outfitter()
            .is_none()
    );
    game.press(KeyCode::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    let faster = game.top_speed();
    assert!((faster - (base_speed + 6.0)).abs() < 1e-6, "{faster}");
    game.frame();

    // Closing the window saves; a new app opens the pilot, docked where it
    // last landed, with its outfits and the stats they give.
    let flown: Pilot = game.pilot();
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Exit);
    assert_eq!(saved(&store, "Ada"), flown);
    let mut game = Harness::opening(&store);
    let open_pilot = game.menu_button(MenuChoice::OpenPilot);
    game.click(open_pilot);
    game.press(KeyCode::Enter);
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(game.pilot().owned(BOOSTER), 2);
    assert_eq!(game.pilot().owned(TANK), 1);
    assert!((game.top_speed() - faster).abs() < 1e-6);
    let fuel = game.pilot().reserves().fuel;
    assert!((fuel.max - 400.0).abs() < 1e-3, "{fuel:?}");
    game.open_outfitter();
    let shown = texts(&game.frame());
    assert!(shown.contains(&"You Have: 2".to_owned()), "{shown:?}");
    assert!(shown.contains(&"Available: 1 tons".to_owned()), "{shown:?}");
}
