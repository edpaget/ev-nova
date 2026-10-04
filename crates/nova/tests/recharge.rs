//! Recharging through the whole app: synthetic game data (a `chär` flying
//! a ship with 300 fuel and no regeneration from Alpha, linked to Beta,
//! each with one inhabited planet) and a synthetic interface file holding
//! the stock "Create a new pilot:" and "Spaceport" dialogs, laid out by the
//! real glyphon metrics, drawn through the renderer into the recording Gpu
//! and driven only by window events, keys and typed text made by the
//! platform's own translation. Pilots are kept in the in-memory store, so
//! the test sees what is saved.
//!
//! A new pilot jumps to Beta, a jump's fuel (100) gone, lands on Beta
//! Prime and clicks Recharge: the tank fills for 100 credits, and the
//! pilot is saved. Recharge again is refused, the tank being full, and
//! says so in the description box; nothing changes and nothing is saved.
//! It takes off with the tank full. A pilot with too few credits is
//! refused, and told why.

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
use nova_sim::{Gauge, Pilot, PilotKeeper, PilotStore, Session, ShipState, StellarId, SystemId};
use nova_view::MouseButton;
use nova_view::geometry::Point;
use nova_view::menu::MenuChoice;
use nova_view::spaceport::layout::RECHARGE_ITEM;
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

/// The `chär`: `cash` credits, ship 128 in Alpha (`sÿst` 128) on 23 June
/// 1177, and no legal records.
fn character(cash: i32) -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    bytes[0x00..0x04].copy_from_slice(&cash.to_be_bytes());
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    put_i16s(&mut bytes, 0x134, &[23, 6, 1177]);
    bytes
}

/// The `shïp`: 20 tons of cargo space, shield 30, accel 3000 (a pixel a
/// tick, a tick), speed 300, maneuver 300 (30° a tick), 300 fuel and no
/// fuel regeneration, armour 45.
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

/// An inhabited planet that can be landed on (flags `0x01`, no services),
/// at (`x`, 0), of graphic type `graphic_type`.
fn stellar(x: i16, graphic_type: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0, graphic_type]);
    bytes[0x06..0x0A].copy_from_slice(&0x01_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x0C, &[1]);
    put_i16s(&mut bytes, 0x14, &[-1]);
    put_i16s(&mut bytes, 0x18, &[-1, -1]);
    bytes
}

/// Beta Prime's place: where a ship arriving from Alpha comes to rest.
const BETA_PRIME_X: i16 = -960;

/// A `spïn` naming `rlëD` `image`, one frame across.
fn spin(image: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, -1, 0, 0, 1, 1]);
    bytes
}

/// Beta Prime's description.
const DESCRIPTION: &str = "A quiet world of farms.";

fn description() -> Vec<u8> {
    let mut bytes = DESCRIPTION.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend([0xFF; 2]);
    bytes.extend([0; 34]);
    bytes
}

/// Alpha (128) and Beta (129), linked, 100 apart on the map: Alpha Prime,
/// small, at Alpha's centre, and Beta Prime, 200 pixels across, near where
/// a ship from Alpha arrives, with its description. The `chär` holds
/// `cash`.
fn game_data(cash: i32) -> Rc<GameData> {
    let mut resources = vec![
        (Character::TYPE, 128, character(cash)),
        (Ship::TYPE, 128, ship()),
        (ShipAnim::TYPE, 128, ship_anim()),
        (RLED, 2000, sheet(36, 1)),
        (System::TYPE, 128, system(0, 129, 128)),
        (System::TYPE, 129, system(100, 128, 129)),
        (Stellar::TYPE, 128, stellar(0, 4)),
        (Stellar::TYPE, 129, stellar(BETA_PRIME_X, 5)),
        (Desc::TYPE, 129, description()),
        (Spin::TYPE, 1004, spin(1000)),
        (Spin::TYPE, 1005, spin(1001)),
        (RLED, 1000, sheet(1, 8)),
        (RLED, 1001, sheet(1, 200)),
        (PICT, 8500, pict(6, 5, [40, 40, 40])),
        (PICT, 10_004, pict(6, 3, [0, 90, 0])),
        (PICT, 10_005, pict(6, 3, [0, 0, 90])),
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

/// Stock "Create a new pilot:" (`DLOG` 3102) and "Spaceport" (`DLOG`
/// 1000), whose item 4 is Recharge's.
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
    let file = fork(&[
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
    /// The app on the main menu, its `chär` holding `cash`, keeping pilots
    /// in `store`, before any frame.
    fn opening(store: &MemoryPilots, cash: i32) -> Self {
        let data = game_data(cash);
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

    fn fuel(&self) -> Gauge {
        self.pilot().reserves().fuel
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

    /// Clicks Recharge, item 4 of the spaceport as laid out.
    fn recharge(&mut self) {
        let at = self
            .app
            .screen()
            .spaceport_view()
            .expect("landed")
            .dialog()
            .expect("laid out")
            .item_bounds(RECHARGE_ITEM)
            .expect("an item")
            .center();
        self.click(at);
        assert_eq!(self.showing(), Showing::Spaceport);
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

    /// Brings the ship to rest: Down turns it to face against its motion,
    /// then Up thrusts until it has all but stopped, and both let go.
    fn brake(&mut self) {
        let mut held = Vec::new();
        for _ in 0..600 {
            let ship = self.ship();
            if ship.velocity.length() < 0.5 {
                self.hold(&mut held, &[]);
                return;
            }
            let against = heading_of(ship.velocity * -1.0);
            let codes = if shortest_turn(ship.heading, against).abs() < 1e-3 {
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
        map.view().world_to_screen(system.position())
    }

    /// Plots a course to Beta on flight's map (M, a click, M), flies out
    /// and jumps (J), runs frames until the jump is over, brakes and lands
    /// on Beta Prime (L).
    fn land_on_beta_prime(&mut self) {
        self.press(KeyCode::KeyM);
        let beta = self.on_map(129);
        self.click(beta);
        self.press(KeyCode::KeyM);
        assert_eq!(self.session().course(), [SystemId(129)]);
        self.fly_out();
        self.press(KeyCode::KeyJ);
        for _ in 0..600 {
            self.frame();
            let flight = self.app.screen().flight_view().expect("flying");
            if flight.jump_effect().is_none() {
                break;
            }
        }
        assert_eq!(self.session().system(), SystemId(129), "the jump ended");
        self.brake();
        self.land();
        assert_eq!(
            self.showing(),
            Showing::Spaceport,
            "landed: {:?}",
            self.ship()
        );
        assert_eq!(self.pilot().stellar(), Some(StellarId(129)));
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

/// A new pilot named Ada, its `chär` holding `cash`, landed on Beta Prime
/// with a jump's fuel gone.
fn at_beta_prime(store: &MemoryPilots, cash: i32) -> Harness {
    let mut game = Harness::opening(store, cash);
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
    assert_eq!(game.fuel(), Gauge::full(300.0));
    game.land_on_beta_prime();
    assert_eq!(
        game.fuel(),
        Gauge {
            now: 200.0,
            max: 300.0
        },
        "a jump"
    );
    assert_eq!(game.pilot().cash(), i64::from(cash));
    let shown = texts(&game.frame());
    for text in ["Recharge", DESCRIPTION] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }
    game
}

#[test]
fn recharge_fills_the_tank_for_a_credit_a_unit_and_a_full_one_is_refused() {
    let store = MemoryPilots::new();
    let mut game = at_beta_prime(&store, 10_000);

    // Recharge: 100 units missing, 100 credits, and the pilot saved.
    let writes = store.writes();
    game.recharge();
    assert_eq!(game.fuel(), Gauge::full(300.0));
    assert_eq!(game.pilot().cash(), 10_000 - 100);
    assert_eq!(store.writes(), writes + 1, "saved");
    let shown = texts(&game.frame());
    assert!(shown.contains(&DESCRIPTION.to_owned()), "{shown:?}");

    // Again, with the tank full: refused, and the box says why.
    let before = game.pilot();
    game.recharge();
    assert_eq!(game.pilot(), before, "nothing changes");
    assert_eq!(store.writes(), writes + 1, "nothing saved");
    let shown = texts(&game.frame());
    assert!(
        shown.contains(&"Your ship is already fully recharged.".to_owned()),
        "{shown:?}"
    );
    assert!(!shown.contains(&DESCRIPTION.to_owned()), "{shown:?}");

    // Leave: it takes off with the tank full and the credits paid.
    game.press(KeyCode::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    game.frame();
    assert_eq!(game.fuel(), Gauge::full(300.0));
    assert_eq!(game.pilot().cash(), 10_000 - 100);
}

#[test]
fn a_pilot_short_of_credits_is_refused_and_told_why() {
    let store = MemoryPilots::new();
    let mut game = at_beta_prime(&store, 99);
    let before = game.pilot();
    let writes = store.writes();
    game.recharge();
    assert_eq!(game.pilot(), before, "nothing changes");
    assert_eq!(store.writes(), writes, "nothing saved");
    let shown = texts(&game.frame());
    assert!(
        shown.contains(&"You don't have enough credits to recharge.".to_owned()),
        "{shown:?}"
    );
}
