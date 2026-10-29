//! The roadmap's "Done looks like", end to end through the whole app:
//! synthetic game data and a synthetic interface file holding the stock
//! "Create a new pilot:", "Spaceport", "Trade", "Outfit", "Shipyard" and
//! "Shipyard Info" dialogs, laid out by the real glyphon metrics, drawn
//! through the renderer into the recording Gpu and driven only by window
//! events, keys and typed text made by the platform's own translation.
//! Pilots are kept in the in-memory store, and the test "restarts" with a
//! new app over the same store.
//!
//! A new pilot buys food in Alpha's trade center, jumps to Beta, sells it
//! there, and buys a fuel tank in Beta's outfitter and a ship in its
//! shipyard; each order writes the pilot to the store. It takes off in the
//! new ship and quits. In a new app, Open Pilot restores the pilot flown.
//!
//! The prices, what a trade moves, what a shipyard lists and refuses, the
//! trade-ins and the days that pass are nova-sim's, pinned there.

use crate::bracket_art;
use crate::common::text_input_items;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use crate::common::{EntryHarness, Frames};
use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova::platform;
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
use nova_data::records::outfit::Outfit;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;
use nova_data::store::fixture::{one_file, one_file_at};
use nova_data::{GameData, InterfaceData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_render::{FontFaces, Frame};
use nova_rsrc::ResType;
use nova_rsrc::fixture::ForkBuilder;
use nova_sim::fixture::MemoryPilots;
use nova_sim::flight::{heading_of, shortest_turn};
use nova_sim::hyperspace::MIN_JUMP_DISTANCE;
use nova_sim::{
    Good, OutfitId, Pilot, PilotKeeper, PilotStore, Session, ShipId, ShipState, SystemId,
};
use nova_view::MouseButton;
use nova_view::geometry::Point;
use nova_view::menu::MenuChoice;
use nova_view::menu::fixture::main_screen_resources;
use nova_view::spaceport::{OutfitterScreen, ShipyardScreen};
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

fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> Vec<u8> {
    resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
            fork.resource(*ty, *id, None, data)
        })
        .build()
        .bytes
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
const CASH: i32 = 10_000;

/// The `chär`: 10,000 credits, ship 128 in Alpha (`sÿst` 128) on 23 June
/// 1177, and no legal records.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    bytes[0x00..0x04].copy_from_slice(&CASH.to_be_bytes());
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    put_i16s(&mut bytes, 0x134, &[23, 6, 1177]);
    bytes
}

/// A `shïp` of tech level 1 with `holds` tons of cargo space, shield 30,
/// accel 3000 (a pixel a tick, a tick), `speed`, maneuver 300 (30° a
/// tick), fuel 300, `free_mass` tons free, armour 45, of `mass`, at
/// `cost`, for sale `buy_random` % of days, with no default items.
fn ship(holds: i16, speed: i16, free_mass: i16, mass: i16, cost: i32, buy_random: i16) -> Vec<u8> {
    ShipBuilder::new()
        .holds(holds)
        .shield(30)
        .accel(3000)
        .speed(speed)
        .maneuver(300)
        .fuel(300)
        .free_mass(free_mass)
        .armor(45)
        .tech_level(1)
        .cost(cost)
        .mass(mass)
        .default_items(&[-1; 8])
        .buy_random(buy_random)
        .build()
}

/// The ship bought: 30 tons of cargo space, speed 450, 15 tons free, mass
/// 30, 8,000 credits, for sale every day.
const NEW: ShipId = ShipId(129);

/// The fuel tank, `oütf` 128.
const TANK: OutfitId = OutfitId(128);

/// A fuel tank: tech level 1, +100 fuel, 2 tons, up to 5, 2,000 credits,
/// for sale every day (`BuyRandom` 100) and requiring nothing anywhere.
fn tank() -> Vec<u8> {
    let mut bytes = vec![0; Outfit::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[30, 2, 1, 12, 100, 5]);
    bytes[0x0E..0x12].copy_from_slice(&2000_i32.to_be_bytes());
    put_i16s(&mut bytes, 0x12, &[0; 6]);
    put_i16s(&mut bytes, 0x3F0, &[100, -1]);
    bytes
}

/// A `shän` whose base image is `rlëD` 2000, one set of 36 rotations.
fn ship_anim() -> Vec<u8> {
    ShipAnimBuilder::new()
        .base_image_id(2000)
        .base_set_count(1)
        .frames_per(36)
        .build()
}

/// An `rlëD` of `frames` frames of `size` x `size`.
fn sheet(frames: u16, size: u16) -> Vec<u8> {
    (0..frames)
        .fold(RledBuilder::new(size, size), |sheet, _| {
            sheet.frame(|f| (0..size).fold(f, |f, _| f.line().pixels(&vec![0x0400; size.into()])))
        })
        .build()
}

/// An independent `sÿst` at map (`x`, 0), linked to `link`, holding
/// `stellar` alone.
fn system(x: i16, link: i16, stellar: i16) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0]);
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x04, &[link]);
    put_i16s(&mut bytes, 0x24, &[stellar]);
    put_i16s(&mut bytes, 0x66, &[-1]);
    bytes
}

/// A landable planet at `(x, y)` of graphic type `graphic_type`, with
/// these `Flags`, of tech level 1.
fn stellar((x, y): (i16, i16), graphic_type: i16, flags: u32) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, y, graphic_type]);
    bytes[0x06..0x0A].copy_from_slice(&flags.to_be_bytes());
    put_i16s(&mut bytes, 0x0C, &[1]);
    put_i16s(&mut bytes, 0x14, &[-1]);
    put_i16s(&mut bytes, 0x18, &[-1, -1]);
    bytes
}

/// Alpha Prime, at (50, 50), where a new pilot's ship starts: a trade
/// center with food low.
const ALPHA_PRIME: u32 = 0x1000_0003;
/// Beta Prime, where the ship drops out of hyperspace from Alpha: a trade
/// center with food high, an outfitter and a shipyard.
const BETA_PRIME: u32 = 0x4000_000F;
/// Beta Prime's place: where a ship arriving from Alpha comes to rest.
const BETA_PRIME_X: i16 = -960;

/// A `spïn` naming `rlëD` `image`, one frame across.
fn spin(image: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, -1, 0, 0, 1, 1]);
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

/// Alpha (128) and Beta (129), linked, 100 apart on the map: Alpha
/// Prime, small, at (50, 50), and Beta Prime, 200 pixels across,
/// near where a ship from Alpha arrives. Food's base price is 100. The
/// ship flown first, never for sale, the ship bought, the fuel tank, and
/// the pictures the screens draw.
fn game_data() -> Rc<GameData> {
    let mut resources = vec![
        (Character::TYPE, 128, character()),
        (Ship::TYPE, 128, ship(20, 300, 10, 20, 10_000, 0)),
        (Ship::TYPE, 129, ship(30, 450, 15, 30, 8000, 100)),
        (ShipAnim::TYPE, 128, ship_anim()),
        (ShipAnim::TYPE, 129, ship_anim()),
        (RLED, 2000, sheet(36, 1)),
        (System::TYPE, 128, system(0, 129, 128)),
        (System::TYPE, 129, system(100, 128, 129)),
        (Stellar::TYPE, 128, stellar((50, 50), 4, ALPHA_PRIME)),
        (
            Stellar::TYPE,
            129,
            stellar((BETA_PRIME_X, 0), 5, BETA_PRIME),
        ),
        (Spin::TYPE, 1004, spin(1000)),
        (Spin::TYPE, 1005, spin(1001)),
        (RLED, 1000, sheet(1, 8)),
        (RLED, 1001, sheet(1, 200)),
        (Outfit::TYPE, 128, tank()),
        (StrList::TYPE, 4000, str_list(&["Food"])),
        (StrList::TYPE, 4004, str_list(&["100"])),
        (PICT, 10_004, pict(6, 3, [0, 90, 0])),
        (PICT, 10_005, pict(6, 3, [0, 0, 90])),
    ];
    for (id, shade) in [(8500, 40), (8501, 10), (8502, 20), (8506, 30), (8510, 50)] {
        resources.push((PICT, id, pict(6, 5, [shade, shade, shade])));
    }
    for state in [7500, 7503, 7506] {
        resources.push((PICT, state, pict(13, 25, [200, 0, 0])));
        resources.push((PICT, state + 1, pict(2, 25, [0, 200, 0])));
        resources.push((PICT, state + 2, pict(13, 25, [0, 0, 200])));
        resources.push((PICT, state + 100, pict(13, 25, [0, 0, 0])));
        resources.push((PICT, state + 102, pict(13, 25, [0, 0, 0])));
    }
    // The main menu's art, so the menu draws the original's screen.
    resources.extend(main_screen_resources());
    let fork = resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
            fork.resource(*ty, *id, None, data)
        });
    Rc::new(one_file(bracket_art::with_bracket_art(fork).build().bytes).load())
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

/// An item's place: (x, y, w, h).
type Place = (i16, i16, i16, i16);

/// User items at these places, each enabled or not.
fn user_items(items: &[(Place, bool)]) -> Vec<Vec<u8>> {
    items
        .iter()
        .map(|&((x, y, w, h), enabled)| {
            let byte = if enabled { USER } else { USER | DISABLED };
            ditl_item((x, y, x + w, y + h), byte, &[])
        })
        .collect()
}

/// Stock "Shipyard" (`DLOG` 1004, 765 x 323, centred) and its thirteen
/// user items, where 2, 3, 4 and 11 are parked outside it, and stock
/// "Shipyard Info" (`DLOG` 1005, 250 x 285, centred) with its Done (1),
/// title (3) and text (5), and stock "Text Input" (`DLOG` 3001).
fn shipyard_dialogs() -> Vec<(ResType, i16, Vec<u8>)> {
    let shipyard = user_items(&[
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
    ]);
    let info = user_items(&[
        ((86, 253, 74, 25), true),
        ((300, 0, 1, 1), false),
        ((3, 3, 240, 24), false),
        ((300, 0, 1, 1), false),
        ((9, 32, 234, 214), false),
    ]);
    vec![
        (Dlog::TYPE, 1004, dlog((100, 100, 423, 865), 1004)),
        (Ditl::TYPE, 1004, ditl(&shipyard)),
        (Dlog::TYPE, 1005, dlog((100, 100, 385, 350), 1005)),
        (Ditl::TYPE, 1005, ditl(&info)),
        (Dlog::TYPE, 3001, dlog((100, 100, 238, 460), 3001)),
        (Ditl::TYPE, 3001, ditl(&text_input_items(ditl_item))),
    ]
}

/// Stock "Create a new pilot:" (`DLOG` 3102), "Spaceport" (`DLOG` 1000),
/// "Trade" (`DLOG` 1001), "Outfit" (`DLOG` 1002) and the shipyard's
/// dialogs, with stock "Text Input" (`DLOG` 3001), the ship's christening
/// and the shipyard's name prompt.
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
    let mut resources = vec![
        (Dlog::TYPE, 3102, dlog((78, 175, 291, 501), 3102)),
        (Ditl::TYPE, 3102, ditl(&new_pilot)),
        (Dlog::TYPE, 1000, dlog((-201, 60, 316, 678), 1000)),
        (Ditl::TYPE, 1000, ditl(&spaceport)),
        (Dlog::TYPE, 1001, dlog((35, 32, 287, 458), 1001)),
        (Ditl::TYPE, 1001, ditl(&trade)),
        (Dlog::TYPE, 1002, dlog((100, 100, 421, 865), 1002)),
        (Ditl::TYPE, 1002, ditl(&outfit)),
    ];
    resources.extend(shipyard_dialogs());
    let path = Path::new("/Nova-DF.rsrc");
    InterfaceData::load(&one_file_at(path, fork(&resources)), path).expect("loads")
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
            app: App::new(&FakeWindow, data, crate::common::appeared(screen)),
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
    /// the dock approach clears it, and the second lands.
    fn land(&mut self) {
        self.press(KeyCode::KeyL);
        self.settle();
        self.settle_until("the dock approach's clearance", |game| {
            game.session().dock_cleared()
        });
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
            .press_point(choice)
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn pilot(&self) -> Pilot {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .sim()
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

    /// The spaceport's screen open, if any.
    fn spaceport(&self) -> &nova_view::spaceport::SpaceportView {
        self.app.screen().spaceport_view().expect("landed")
    }

    /// The Outfitter, open.
    fn outfitter(&self) -> &OutfitterScreen {
        self.spaceport().open_outfitter().expect("outfitting")
    }

    /// The Shipyard, open.
    fn shipyard(&self) -> &ShipyardScreen {
        self.spaceport().open_shipyard().expect("shipbuying")
    }

    /// Clicks spaceport item `number`, opening a service over it.
    fn open(&mut self, number: usize) {
        let at = self.spaceport_item(number);
        self.click(at);
        assert_eq!(self.showing(), Showing::Spaceport);
    }

    /// Closes the service open, then leaves the spaceport: Escape twice.
    fn leave(&mut self) {
        self.press(KeyCode::Escape);
        self.press(KeyCode::Escape);
        assert_eq!(self.showing(), Showing::Flight);
    }

    fn session(&self) -> &Session {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .session()
            .expect("a session")
    }

    fn ship(&self) -> ShipState {
        *self.session().player()
    }

    /// Holds exactly the arrow keys `codes` of the flight keys held, each
    /// pressed or released through the platform as it changes.
    fn hold(&mut self, held: &mut Vec<KeyCode>, codes: &[KeyCode]) {
        for code in held.clone() {
            if !codes.contains(&code) {
                self.key(code, false, false);
            }
        }
        for &code in codes {
            if !held.contains(&code) {
                self.key(code, true, false);
            }
        }
        *held = codes.to_vec();
    }

    /// Flies out from the centre until the ship is at least the minimum
    /// jump distance from it, then lets go: Down, to face away from the
    /// centre while drifting in, then Up.
    fn fly_out(&mut self) {
        let mut held = Vec::new();
        for _ in 0..3000 {
            let ship = self.ship();
            if ship.position.length() >= MIN_JUMP_DISTANCE {
                self.hold(&mut held, &[]);
                return;
            }
            // Drifting in at a crawl, as a ship at rest off the centre
            // can, isn't moving in: Down would hold it there.
            let moving_in =
                ship.position.x * ship.velocity.x + ship.position.y * ship.velocity.y < -1e-3;
            let facing_out = ship.position.length() == 0.0
                || shortest_turn(ship.heading, heading_of(ship.position)).abs() < 1e-3;
            let codes = if moving_in && !facing_out {
                [KeyCode::ArrowDown]
            } else {
                [KeyCode::ArrowUp]
            };
            self.hold(&mut held, &codes);
            self.frame();
        }
        panic!("never got out: {:?}", self.ship());
    }

    /// Brings the ship to rest: Down turns it to face against its motion,
    /// within a turn of where reverse aims, where it stops turning, then Up
    /// thrusts until it has all but stopped, and both let go.
    fn brake(&mut self) {
        let mut held = Vec::new();
        let turn = self.session().handling().turn_rate;
        for _ in 0..600 {
            let ship = self.ship();
            let aim = nova_sim::player_turn::reverse_aim(ship.velocity);
            let Some(against) = aim.filter(|_| ship.velocity.length() >= 0.5) else {
                self.hold(&mut held, &[]);
                return;
            };
            let codes = if shortest_turn(ship.heading, f32::from(against)).abs() <= turn {
                [KeyCode::ArrowUp]
            } else {
                [KeyCode::ArrowDown]
            };
            self.hold(&mut held, &codes);
            self.frame();
        }
        panic!("never stopped: {:?}", self.ship());
    }

    /// Where system `id` is on flight's course map.
    fn on_map(&self, id: i16) -> Point {
        let map = self
            .app
            .screen()
            .flight_view()
            .expect("flying")
            .course_map();
        let system = map.model().system(SystemId(id)).expect("on the map");
        map.transform().world_to_screen(system.position())
    }

    /// Draws frames until the map key is taken: a take-off refuses it for
    /// [`nova_sim::TAKEOFF_GRACE_TICKS`].
    fn past_the_takeoff_grace(&mut self) {
        self.settle_until("the map key taken", |game| {
            game.session().takes(nova_sim::PlayerCommand::Map)
        });
    }

    /// Plots a course to Beta on flight's map (M, a Shift-click, M), once
    /// the take-off grace is over, flies out and jumps (J), and draws frames
    /// until the jump is over.
    fn jump_to_beta(&mut self) {
        self.past_the_takeoff_grace();
        self.press(KeyCode::KeyM);
        let beta = self.on_map(129);
        self.key(KeyCode::ShiftLeft, true, false);
        self.click(beta);
        self.key(KeyCode::ShiftLeft, false, false);
        self.press(KeyCode::KeyM);
        assert_eq!(self.session().course(), [SystemId(129)]);
        self.fly_out();
        self.press(KeyCode::KeyJ);
        self.settle_until("the jump's arrival in Beta", |game| {
            game.session().system() == SystemId(129)
                && game
                    .app
                    .screen()
                    .flight_view()
                    .is_some_and(|flight| flight.jump_effect().is_none())
        });
    }
}

const FOOD: Good = Good::Commodity(0);

/// A new pilot named Ada, flying from Alpha.
fn new_pilot(store: &MemoryPilots) -> Harness {
    let mut game = Harness::opening(store);
    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    game.type_name("Ada");
    game.press(KeyCode::Enter);
    // The ship's christening, confirmed with its default.
    game.press(KeyCode::Enter);
    game.enter_ship();
    assert_eq!(game.showing(), Showing::Flight);
    game
}

impl Harness {
    /// Sends `order` to the screen open over the spaceport, and asserts
    /// it wrote the pilot to the store.
    fn order(&mut self, store: &MemoryPilots, order: impl FnOnce(&mut Self)) {
        let writes = store.writes();
        order(self);
        assert_eq!(store.writes(), writes + 1, "saved");
        assert_eq!(saved(store, "Ada"), self.pilot());
    }
}

#[test]
fn orders_in_the_trade_center_outfitter_and_shipyard_across_a_jump_are_restored_by_open_pilot() {
    let store = MemoryPilots::new();
    let mut game = new_pilot(&store);

    // Alpha Prime's trade center: B buys food.
    game.land();
    assert_eq!(game.showing(), Showing::Spaceport);
    game.open(7);
    game.order(&store, |game| game.press(KeyCode::KeyB));
    let held = game.pilot().held(FOOD);
    assert!(held > 0, "bought");

    // To Beta, and down on Beta Prime.
    game.leave();
    game.jump_to_beta();
    game.brake();
    game.land();
    assert_eq!(
        game.showing(),
        Showing::Spaceport,
        "landed: {:?}",
        game.ship()
    );

    // Beta Prime's trade center: S sells food.
    game.open(7);
    game.order(&store, |game| game.press(KeyCode::KeyS));
    assert!(game.pilot().held(FOOD) < held, "sold");
    game.press(KeyCode::Escape);

    // The outfitter: the fuel tank, by its cell and B.
    game.open(8);
    let cell = game.outfitter().cell_bounds(0).expect("a cell").center();
    game.click(cell);
    game.order(&store, |game| game.press(KeyCode::KeyB));
    assert!(game.pilot().owned(TANK) > 0, "bought");
    game.press(KeyCode::Escape);

    // The shipyard: Right picks the ship, B asks its name, and Return
    // takes the default and buys it.
    game.open(9);
    game.press(KeyCode::ArrowRight);
    game.type_key(KeyCode::KeyB, Some("b"));
    assert!(game.shipyard().naming().is_some(), "the name prompt");
    game.order(&store, |game| game.press(KeyCode::Enter));
    assert_eq!(game.pilot().ship(), NEW);

    // Leave, and close the window, which saves.
    game.leave();
    let flown = game.pilot();
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Exit);
    assert_eq!(saved(&store, "Ada"), flown);

    // A new app opens the pilot: everything restored.
    let mut game = Harness::opening(&store);
    let open_pilot = game.menu_button(MenuChoice::OpenPilot);
    game.click(open_pilot);
    game.press(KeyCode::Enter);
    game.enter_ship();
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.pilot(), flown);
}

crate::common::entry_harness!(Harness, click, frame);
