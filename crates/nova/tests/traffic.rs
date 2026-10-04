//! The app flying among NPC traffic over synthetic game data, wired to the
//! renderer and the recording Gpu and driven only by key and mouse events
//! and redraws, as the window sends them: a system whose one `düde` flies
//! a trader, one ship on average. Entering flight populates it on the
//! app's source of chance: the trader is drawn with its own sprite and as
//! a dim radar blip, flies to the system's planet and lands there, and is
//! gone. Every flight's NPCs decide as the router's behaviour says,
//! whether flight was entered with the developer's F or a new pilot flies
//! it.

// Sizes and positions here are compared after the same arithmetic on
// both sides.
#![allow(clippy::float_cmp)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::dude::Dude;
use nova_data::records::interface::Interface;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, QuadInstance, Rect, SolidQuad};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_sim::fixture::MemoryPilots;
use nova_sim::{
    Behaviour, Chance, Goal, Npc, PilotKeeper, PilotStore, Session, ShipId, StellarId,
    Surroundings, Vec2,
};
use nova_view::flight::{FlightView, SharedChance};
use nova_view::menu::MenuChoice;
use nova_view::text::fixture::MonoMetrics;
use nova_view::{Key, MouseButton, Point};

/// A 1024x768 window at scale 1: window pixels are logical units.
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

/// A `chär` starting in ship 128 in system 128, with no legal records.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    bytes
}

/// A `shïp` with this `Shield`, `Accel`, `Speed`, `Maneuver` and `Fuel`,
/// 45 `Armor`, and `InherentAI` 1, a wimpy trader.
fn ship(accel: i16, speed: i16) -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[30, accel, speed, 30, 300]);
    put_i16s(&mut bytes, 0x0E, &[45]);
    put_i16s(&mut bytes, 0x42, &[1]);
    bytes
}

/// A `shän` whose base image is `rlëD` `image`, one set of 36 rotations.
fn ship_anim(image: i16) -> Vec<u8> {
    let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, 0, 1]);
    put_i16s(&mut bytes, 0x34, &[36]);
    bytes
}

/// An `rlëD` of `frames` frames of `size` x `size`.
fn sheet(frames: u16, size: u16) -> Vec<u8> {
    (0..frames)
        .fold(RledBuilder::new(size, size), |sheet, frame| {
            let color = 0x0400 + frame;
            sheet.frame(|f| (0..size).fold(f, |f, _| f.line().pixels(&vec![color; size.into()])))
        })
        .build()
}

/// The dim radar colour of the status bar fixture.
const DIM_RADAR: u32 = 0x0000_8000;

/// Stock `ïntf` 128's areas and font, over background `PICT` 700.
fn interface() -> Vec<u8> {
    let mut bytes = vec![0; Interface::SIZE.expect("fixed")];
    put_u32s(&mut bytes, 0x00, &[0x00FF_FFFF, 0x0080_8080]);
    put_i16s(&mut bytes, 0x08, &[8, 8, 184, 184]);
    put_u32s(&mut bytes, 0x10, &[0x0000_FF00, DIM_RADAR]);
    put_i16s(&mut bytes, 0x18, &[199, 35, 206, 184]);
    put_u32s(&mut bytes, 0x20, &[0x0000_00FF]);
    put_i16s(&mut bytes, 0x24, &[216, 35, 223, 184]);
    put_u32s(&mut bytes, 0x2C, &[0x00FF_0000]);
    put_i16s(&mut bytes, 0x30, &[234, 35, 241, 184]);
    put_u32s(&mut bytes, 0x38, &[0x00FF_FF00, 0x0080_8000]);
    put_i16s(&mut bytes, 0x40, &[254, 8, 286, 184]);
    bytes[0x60..0x66].copy_from_slice(b"Geneva");
    put_i16s(&mut bytes, 0xA0, &[12]);
    put_i16s(&mut bytes, 0xA4, &[700]);
    bytes
}

/// A grey 194 x 16 `PICT`: the status bar's background, cut short.
fn status_picture() -> Vec<u8> {
    let bounds = [0, 0, 16, 194];
    PictBuilder::new(bounds)
        .direct_bits(&DirectBits::rgb555(bounds, &[0x4210; 194 * 16]))
        .end()
        .build()
}

/// An independent `sÿst` holding stellar 128, with `düde` 128 at 100 % and
/// one ship on average.
fn system() -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[128]);
    put_i16s(&mut bytes, 0x44, &[128, -1, -1, -1, -1, -1, -1, -1]);
    put_i16s(&mut bytes, 0x54, &[100]);
    put_i16s(&mut bytes, 0x64, &[1, -1]);
    bytes
}

/// A landable `spöb` at (0, -300), graphic type 0.
fn planet() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, -300, 0]);
    put_u32s(&mut bytes, 0x06, &[0x01]);
    bytes
}

/// A `spïn` naming `rlëD` 1000, one frame across.
fn spin() -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[1000, -1, 0, 0, 1, 1]);
    bytes
}

/// A `düde` of wimpy traders flying ship 129, independent.
fn dude() -> Vec<u8> {
    let mut bytes = vec![0; Dude::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[1, -1]);
    put_i16s(&mut bytes, 0x08, &[-1; 16]);
    put_i16s(&mut bytes, 0x08, &[129]);
    put_i16s(&mut bytes, 0x28, &[100]);
    bytes
}

/// The first `chär` flies ship 128 (a 1 x 1 sheet) from Alpha (128),
/// which holds the planet Alpha Prime (128) at (0, -300), 8 x 8, and
/// whose one `düde` flies ship 129, a trader with a 2 x 2 sheet.
fn data() -> Rc<GameData> {
    let fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Shuttle"), &ship(300, 300))
        .resource(Ship::TYPE, 129, Some(b"Trader"), &ship(600, 600))
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(ShipAnim::TYPE, 129, None, &ship_anim(2001))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(RLED, 2001, None, &sheet(36, 2))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system())
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &planet())
        .resource(Spin::TYPE, 1000, None, &spin())
        .resource(RLED, 1000, None, &sheet(1, 8))
        .resource(Dude::TYPE, 128, Some(b"Traders"), &dude())
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &interface(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture());
    let file = OneFile(fork.build().bytes);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
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

/// The app's chance: setup places the trader at (100, -100), facing up.
fn placing_the_trader() -> SharedChance {
    let script: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Script(VecDeque::from([
        6, 6, 0, 0, 850, 650, 0,
    ]))));
    SharedChance::new(script)
}

/// Every NPC idles.
#[derive(Debug)]
struct Still;

impl Behaviour for Still {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }
}

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    /// Frames sent, at 60 a second.
    frames: u64,
}

impl Harness {
    /// The app on the ship browser, its NPCs deciding as `behaviour` says
    /// when one is given.
    fn new(behaviour: Option<Rc<dyn Behaviour>>) -> Self {
        let data = data();
        let screen = start_screen(Rc::clone(&data)).with_chance(placing_the_trader());
        let screen = match behaviour {
            Some(behaviour) => screen.with_behaviour(behaviour),
            None => screen,
        };
        Self {
            app: App::new(&FakeWindow, data, screen),
            gpu: RecordingGpu::new(),
            frames: 0,
        }
    }

    /// The app in flight, entered from the ship browser with F.
    fn flying(behaviour: Option<Rc<dyn Behaviour>>) -> Self {
        let mut harness = Self::new(behaviour);
        harness.press(Key::Char('f'));
        assert_eq!(harness.showing(), Showing::Flight);
        harness
    }

    /// A new pilot flying, from the main menu, its NPCs deciding as
    /// `behaviour` says.
    fn new_pilot(behaviour: Rc<dyn Behaviour>) -> Self {
        let data = data();
        let keeper = PilotKeeper::new(Box::new(MemoryPilots::new()) as Box<dyn PilotStore>);
        let screen = start_screen(Rc::clone(&data))
            .with_chance(placing_the_trader())
            .with_behaviour(behaviour)
            .with_pilots(Some(keeper), Rc::new(MonoMetrics));
        let mut harness = Self {
            app: App::new(&FakeWindow, data, screen),
            gpu: RecordingGpu::new(),
            frames: 0,
        };
        let at = harness
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .button(MenuChoice::NewPilot)
            .rect
            .center();
        harness.click(at);
        for c in "Trader".chars() {
            harness.send(WindowEvent::Text(c));
        }
        harness.press(Key::Enter);
        assert_eq!(harness.showing(), Showing::Flight);
        harness
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue,
            "{event:?}"
        );
    }

    fn press(&mut self, key: Key) {
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
                button: MouseButton::Left,
                pressed,
            });
        }
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn flight(&self) -> &FlightView<Rc<GameData>> {
        self.app.screen().flight_view().expect("flight entered")
    }

    fn session(&self) -> &Session {
        self.flight().session().expect("flying")
    }

    /// Sends the next redraw and returns its frame.
    fn frame(&mut self) -> Frame {
        self.frames += 1;
        let elapsed = Duration::from_nanos(self.frames * 1_000_000_000 / 60);
        self.send(WindowEvent::Redraw { elapsed });
        let frame = (*self.gpu.submits().last().expect("a frame")).clone();
        assert_eq!(self.app.take_failures(), []);
        frame
    }

    /// Sends `seconds` of redraws and returns the last frame.
    fn run(&mut self, seconds: u64) -> Frame {
        let mut last = None;
        for _ in 0..seconds * 60 {
            last = Some(self.frame());
        }
        last.expect("at least one frame")
    }
}

fn quads(frame: &Frame) -> Vec<QuadInstance> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Sprites { quads, .. } => quads.clone(),
            _ => Vec::new(),
        })
        .collect()
}

fn solids(frame: &Frame) -> Vec<SolidQuad> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Solid(quads) => quads.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// A `w` x `h` rectangle centred on (`x`, `y`).
fn centred(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect {
        x: x - w / 2.0,
        y: y - h / 2.0,
        w,
        h,
    }
}

/// The centre of the rectangle a solid quad's corners span.
fn centre(quad: &SolidQuad) -> Point {
    let xs = quad.corners.map(|c| c.x);
    let ys = quad.corners.map(|c| c.y);
    let mid = |v: [f32; 4]| {
        let min = v.into_iter().fold(f32::INFINITY, f32::min);
        let max = v.into_iter().fold(f32::NEG_INFINITY, f32::max);
        f32::midpoint(min, max)
    };
    Point::new(mid(xs), mid(ys))
}

/// A 24-bit colour as the renderer submits it.
fn submitted(raw: u32) -> [f32; 4] {
    let [_, r, g, b] = raw.to_be_bytes();
    [r, g, b, 255].map(|c| f32::from(c) / 255.0)
}

/// The 2 x 2 quads drawn: the trader's.
fn traders(frame: &Frame) -> Vec<Rect> {
    quads(frame)
        .into_iter()
        .map(|quad| quad.dest)
        .filter(|dest| dest.w == 2.0 && dest.h == 2.0)
        .collect()
}

#[test]
fn entering_flight_shows_the_trader_with_its_sprite_and_a_radar_blip() {
    let mut harness = Harness::flying(None);
    let first = harness.frame();
    let session = harness.session();
    assert_eq!(session.npcs().len(), 1);
    let trader = session.npcs()[0];
    assert_eq!(trader.ship, ShipId(129));
    assert_eq!(trader.state.position, Vec2::new(100.0, -100.0));
    // The player is at the centre of the screen, (512, 384).
    assert_eq!(traders(&first), [centred(612.0, 284.0, 2.0, 2.0)]);
    let blips: Vec<Point> = solids(&first)
        .iter()
        .filter(|quad| quad.color == submitted(DIM_RADAR))
        .map(centre)
        .collect();
    assert_eq!(
        blips,
        [Point::new(926.0 + 100.0 / 16.0, 96.0 - 100.0 / 16.0)]
    );
}

#[test]
fn the_trader_flies_to_the_planet_lands_and_is_gone() {
    let mut harness = Harness::flying(None);
    harness.frame();
    harness.run(1);
    let trader = harness.session().npcs()[0];
    assert_eq!(trader.goal, Goal::Land(StellarId(128)));
    let last = harness.run(20);
    assert_eq!(harness.session().npcs(), [], "landed and gone");
    assert_eq!(traders(&last), []);
}

#[test]
fn the_routers_behaviour_decides_for_a_flight_entered_with_f() {
    let mut harness = Harness::flying(Some(Rc::new(Still)));
    harness.run(3);
    let trader = harness.session().npcs()[0];
    assert_eq!(trader.goal, Goal::Idle);
    assert_eq!(trader.state.position, Vec2::new(100.0, -100.0));
}

#[test]
fn the_routers_behaviour_decides_for_a_new_pilots_flight() {
    let mut harness = Harness::new_pilot(Rc::new(Still));
    harness.run(3);
    let trader = harness.session().npcs()[0];
    assert_eq!(trader.goal, Goal::Idle);
    assert_eq!(trader.state.position, Vec2::new(100.0, -100.0));
}
