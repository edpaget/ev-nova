//! A hyperspace outfit through the whole app: synthetic game data (a
//! `chär` with 25,000 credits flying a ship from Alpha, linked to Beta;
//! Alpha's one planet, at its centre, an outfitter selling a Horizontal
//! Booster, `oütf` `ModType` 23 at -500 as in the stock data) and a
//! synthetic interface file holding the stock "Create a new pilot:",
//! "Spaceport" and "Outfit" dialogs, laid out by the real glyphon metrics,
//! drawn through the renderer into the recording Gpu and driven only by
//! window events, keys and typed text made by the platform's own
//! translation.
//!
//! A new pilot lands, buys the booster, takes off, plots a course to Beta
//! and flies out past half the standard jump distance but not the whole of
//! it: J jumps, and it arrives in Beta. A pilot who did not buy the booster
//! flies the same way and is refused, as too close.

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
use nova_sim::flight::{heading_of, shortest_turn};
use nova_sim::hyperspace::MIN_JUMP_DISTANCE;
use nova_sim::{OutfitId, Pilot, PilotKeeper, PilotStore, Session, ShipState, SystemId};
use nova_view::MouseButton;
use nova_view::flight::view::TOO_CLOSE;
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

/// The `chär`: 25,000 credits, ship 128 in Alpha (`sÿst` 128) on 23 June
/// 1177, and no legal records.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    bytes[0x00..0x04].copy_from_slice(&25_000_i32.to_be_bytes());
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    put_i16s(&mut bytes, 0x134, &[23, 6, 1177]);
    bytes
}

/// The `shïp`: 20 tons of cargo space, shield 30, accel 3000 (a pixel a
/// tick, a tick), speed 300, maneuver 300 (30° a tick), 300 fuel and no
/// fuel regeneration, 10 tons free, armour 45.
fn ship() -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[20, 30, 3000, 300, 300, 300, 10, 45]);
    put_i16s(&mut bytes, 0x2E, &[1]);
    put_i16s(&mut bytes, 0x3E, &[20]);
    put_i16s(&mut bytes, 0x4E, &[-1; 4]);
    put_i16s(&mut bytes, 0x370, &[-1; 4]);
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

/// A landable planet at (`x`, 0) of graphic type `graphic_type`, with
/// `flags`, of tech level 2.
fn stellar(x: i16, graphic_type: i16, flags: u32) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0, graphic_type]);
    bytes[0x06..0x0A].copy_from_slice(&flags.to_be_bytes());
    put_i16s(&mut bytes, 0x0C, &[2]);
    put_i16s(&mut bytes, 0x14, &[-1]);
    put_i16s(&mut bytes, 0x18, &[-1, -1]);
    bytes
}

/// The stock Horizontal Booster (`oütf` 202): `ModType` 23, the no-jump
/// zone's radius, at -500, a ton, up to one; here `oütf` 128, for 5000
/// credits.
const BOOSTER: OutfitId = OutfitId(128);

fn booster() -> Vec<u8> {
    let mut bytes = vec![0; Outfit::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[30, 1, 1, 23, -500, 1]);
    bytes[0x0E..0x12].copy_from_slice(&5000_i32.to_be_bytes());
    put_i16s(&mut bytes, 0x12, &[0; 6]);
    put_i16s(&mut bytes, 0x3F2, &[-1]);
    let name = b"Horizontal\\nBooster";
    bytes[0x32B..0x32B + name.len()].copy_from_slice(name);
    bytes
}

/// A `spïn` naming `rlëD` `image`, one frame across.
fn spin(image: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, -1, 0, 0, 1, 1]);
    bytes
}

/// Alpha (128) and Beta (129), linked, 100 apart on the map: Alpha Prime,
/// at Alpha's centre, a bar, trade center and outfitter selling the
/// booster; Beta Prime, at Beta's centre, with no services.
fn game_data() -> Rc<GameData> {
    let mut resources = vec![
        (Character::TYPE, 128, character()),
        (Ship::TYPE, 128, ship()),
        (ShipAnim::TYPE, 128, ship_anim()),
        (RLED, 2000, sheet(36, 1)),
        (System::TYPE, 128, system(0, 129, 128)),
        (System::TYPE, 129, system(100, 128, 129)),
        (Stellar::TYPE, 128, stellar(0, 4, 0x2000_1047)),
        (Stellar::TYPE, 129, stellar(0, 5, 0x01)),
        (Spin::TYPE, 1004, spin(1000)),
        (Spin::TYPE, 1005, spin(1001)),
        (RLED, 1000, sheet(1, 8)),
        (RLED, 1001, sheet(1, 8)),
        (PICT, 8500, pict(6, 5, [40, 40, 40])),
        (PICT, 8502, pict(6, 5, [10, 10, 10])),
        (PICT, 6000, pict(4, 4, [200, 200, 0])),
        (PICT, 10_004, pict(6, 3, [0, 90, 0])),
        (PICT, 10_005, pict(6, 3, [0, 0, 90])),
        (Outfit::TYPE, 128, booster()),
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

/// A user item's (x, y, width, height).
type Place = (i16, i16, i16, i16);

/// User items at their places, each enabled or not.
fn user_items(items: &[(Place, bool)]) -> Vec<Vec<u8>> {
    items
        .iter()
        .map(|&((x, y, w, h), enabled)| {
            let byte = if enabled { USER } else { USER | DISABLED };
            ditl_item((x, y, x + w, y + h), byte, &[])
        })
        .collect()
}

/// Stock "Create a new pilot:" (`DLOG` 3102), "Spaceport" (`DLOG` 1000),
/// whose item 8 is the Outfitter, and "Outfit" (`DLOG` 1002).
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
    let spaceport = user_items(&[
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
    ]);
    let outfit = user_items(&[
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
    ]);
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

    fn press(&mut self, code: KeyCode) {
        self.type_key(code, None);
    }

    /// Presses L twice, as the player lands: the first requests clearance,
    /// the second lands.
    fn land(&mut self) {
        self.press(KeyCode::KeyL);
        self.press(KeyCode::KeyL);
    }

    /// Sends the physical key `code` going down or, when not `pressed`, up.
    fn key(&mut self, code: KeyCode, pressed: bool) {
        let state = if pressed {
            ElementState::Pressed
        } else {
            ElementState::Released
        };
        self.send(platform::key_event(PhysicalKey::Code(code), state, false));
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

    fn jumping(&self) -> bool {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .jump_effect()
            .is_some()
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

    /// A new pilot named Ada, in flight.
    fn new_pilot(store: &MemoryPilots) -> Self {
        let mut game = Self::opening(store);
        let new_pilot = game
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .button(MenuChoice::NewPilot)
            .rect
            .center();
        game.click(new_pilot);
        for (code, letter) in [
            (KeyCode::KeyA, "A"),
            (KeyCode::KeyD, "d"),
            (KeyCode::KeyA, "a"),
        ] {
            game.type_key(code, Some(letter));
        }
        game.press(KeyCode::Enter);
        assert_eq!(game.showing(), Showing::Flight);
        game.frame();
        game
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

    /// Lands on Alpha Prime (L, L), clicks the Outfitter (spaceport item 8),
    /// clicks the booster's cell and buys it (B), then chooses Done and
    /// Leave (Escape twice).
    fn buy_the_booster(&mut self) {
        self.land();
        assert_eq!(self.showing(), Showing::Spaceport);
        let outfitter = self
            .app
            .screen()
            .spaceport_view()
            .expect("landed")
            .dialog()
            .expect("laid out")
            .item_bounds(8)
            .expect("an item")
            .center();
        self.click(outfitter);
        assert!(self.outfitter().problem().is_none());
        let rows: Vec<_> = self
            .outfitter()
            .outfitter()
            .rows
            .iter()
            .map(|row| row.id)
            .collect();
        assert_eq!(rows, [BOOSTER]);
        let cell = self.outfitter().cell_bounds(0).expect("a cell").center();
        self.click(cell);
        self.press(KeyCode::KeyB);
        assert_eq!(self.pilot().owned(BOOSTER), 1);
        assert_eq!(self.pilot().cash(), 20_000);
        self.press(KeyCode::Escape);
        self.press(KeyCode::Escape);
        assert_eq!(self.showing(), Showing::Flight);
        self.frame();
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
        map.view().world_to_screen(system.position())
    }

    /// Plots a course to Beta on flight's map: M, a click, M.
    fn plot_beta(&mut self) {
        self.press(KeyCode::KeyM);
        let beta = self.on_map(129);
        self.click(beta);
        self.press(KeyCode::KeyM);
        assert_eq!(self.session().course(), [SystemId(129)]);
    }

    /// Holds exactly the arrow keys `codes` of the flight keys held, each
    /// pressed or released through the platform as it changes.
    fn hold(&mut self, held: &mut Vec<KeyCode>, codes: &[KeyCode]) {
        for code in held.clone() {
            if !codes.contains(&code) {
                self.key(code, false);
            }
        }
        for &code in codes {
            if !held.contains(&code) {
                self.key(code, true);
            }
        }
        *held = codes.to_vec();
    }

    /// Flies out from the centre until the ship is at least `distance`
    /// from it, then lets go: Down, to face away from the centre while
    /// drifting in, then Up.
    fn fly_out_to(&mut self, distance: f32) {
        let mut held = Vec::new();
        for _ in 0..3000 {
            let ship = self.ship();
            if ship.position.length() >= distance {
                self.hold(&mut held, &[]);
                return;
            }
            let moving_in =
                ship.position.x * ship.velocity.x + ship.position.y * ship.velocity.y < 0.0;
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

    /// Plots a course to Beta and flies out past half the standard jump
    /// distance, but short of the whole of it.
    fn halfway_out(&mut self) {
        self.plot_beta();
        self.fly_out_to(MIN_JUMP_DISTANCE / 2.0);
        let distance = self.ship().position.length();
        assert!(
            (MIN_JUMP_DISTANCE / 2.0..MIN_JUMP_DISTANCE).contains(&distance),
            "{distance}"
        );
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

#[test]
fn a_horizontal_booster_bought_in_the_outfitter_lets_the_ship_jump_from_half_as_far_out() {
    let store = MemoryPilots::new();
    let mut game = Harness::new_pilot(&store);
    game.buy_the_booster();
    game.halfway_out();

    game.press(KeyCode::KeyJ);
    assert!(game.jumping(), "the jump effect begins");
    for _ in 0..600 {
        game.frame();
        if !game.jumping() {
            break;
        }
    }
    assert!(!game.jumping(), "the effect is over");
    let session = game.session();
    assert_eq!(session.system(), SystemId(129), "arrived in Beta");
    assert_eq!(session.course(), []);
    assert!((session.reserves().fuel.now - 200.0).abs() < 1e-3);
}

#[test]
fn without_the_booster_the_same_flight_is_too_close_to_jump() {
    let store = MemoryPilots::new();
    let mut game = Harness::new_pilot(&store);
    game.halfway_out();

    game.press(KeyCode::KeyJ);
    assert!(!game.jumping(), "no jump effect");
    let refused = game.frame();
    let shown = texts(&refused);
    assert!(shown.contains(&TOO_CLOSE.to_owned()), "{shown:?}");
    assert!(!game.jumping());
    assert_eq!(game.session().system(), SystemId(128));
    assert_eq!(game.pilot().owned(BOOSTER), 0);
}
