//! The app flying the first `chär`'s ship in its starting system, over
//! synthetic game data, wired to the renderer and the recording Gpu and
//! driven only by key events and redraws, as the window sends them.

use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
use nova_data::records::interface::Interface;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fixture::one_file;
use nova_data::{GameData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, QuadInstance, Rect, SolidQuad};
use nova_rsrc::fixture::ForkBuilder;
use nova_sim::flight::{heading_of, shortest_turn};
use nova_sim::player_turn::{PlayerTurn, reverse_aim, settled};
use nova_sim::{ShipId, ShipState, SystemId, Vec2};
use nova_view::flight::hud::NAV_OFF;
use nova_view::flight::radar::radar_offset;
use nova_view::flight::{FlightOverlays, FlightView};
use nova_view::{Blend, Key, Point};

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

fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
    }
}

/// A `chär` starting in ship 128, its systems -1, 999 (missing), 128 and
/// -1: the first that exists is 128.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, -1, 999, 128, -1]);
    bytes
}

/// A `shïp` with `Accel` 300, `Speed` 300 and `Maneuver` 30: 3 pixels a
/// tick at most, 0.1 more a tick, 3° a tick. Its `Shield` is 30, `Fuel`
/// 300 and `Armor` 45.
fn ship() -> Vec<u8> {
    ShipBuilder::new()
        .shield(30)
        .accel(300)
        .speed(300)
        .maneuver(30)
        .fuel(300)
        .armor(45)
        .build()
}

/// Writes 24-bit colours from `at`.
fn put_u32s(bytes: &mut [u8], at: usize, values: &[u32]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 4 * i..at + 4 * i + 4].copy_from_slice(&value.to_be_bytes());
    }
}

/// The colours of the status bar fixture's shield, armour and fuel bars and
/// radar dots.
const SHIELD: u32 = 0x0000_00FF;
const ARMOR: u32 = 0x00FF_0000;
const FUEL: u32 = 0x00FF_FF00;
const RADAR: u32 = 0x0000_FF00;
const DIM_RADAR: u32 = 0x0000_8000;

/// Stock `ïntf` 128's areas and font, over background `PICT` 700.
fn interface() -> Vec<u8> {
    let mut bytes = vec![0; Interface::SIZE.expect("fixed")];
    put_u32s(&mut bytes, 0x00, &[0x00FF_FFFF, 0x0080_8080]);
    put_i16s(&mut bytes, 0x08, &[8, 8, 184, 184]);
    put_u32s(&mut bytes, 0x10, &[RADAR, DIM_RADAR]);
    put_i16s(&mut bytes, 0x18, &[199, 35, 206, 184]);
    put_u32s(&mut bytes, 0x20, &[SHIELD]);
    put_i16s(&mut bytes, 0x24, &[216, 35, 223, 184]);
    put_u32s(&mut bytes, 0x2C, &[ARMOR]);
    put_i16s(&mut bytes, 0x30, &[234, 35, 241, 184]);
    put_u32s(&mut bytes, 0x38, &[FUEL, 0x0080_8000]);
    put_i16s(&mut bytes, 0x40, &[254, 8, 286, 184]);
    put_i16s(&mut bytes, 0x48, &[300, 8, 315, 184]);
    put_i16s(&mut bytes, 0x50, &[330, 8, 442, 184]);
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

/// A `shän` whose base image is `rlëD` 2000, one set of 36 rotations,
/// with an engine glow, `rlëD` 2100, and (if `lights`) lights, `rlëD`
/// 2200.
fn ship_anim(lights: bool) -> Vec<u8> {
    let anim = ShipAnimBuilder::new()
        .base_image_id(2000)
        .base_set_count(1)
        .glow_image_id(2100)
        .frames_per(36);
    if lights {
        anim.light_image_id(2200)
    } else {
        anim
    }
    .build()
}

/// An independent `sÿst` at map (`x`, 0) with these stellars; every other
/// slot is -1.
fn system(x: i16, stellars: &[i16]) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0]);
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, stellars);
    put_i16s(&mut bytes, 0x66, &[-1]);
    bytes
}

/// A still `spöb` at (`x`, `y`) of graphic type `graphic_type`.
fn stellar(x: i16, y: i16, graphic_type: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, y, graphic_type]);
    bytes
}

/// A `spïn` naming `rlëD` `image`, one frame across.
fn spin(image: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, -1, 0, 0, 1, 1]);
    bytes
}

/// An `rlëD` of `frames` frames of `size` x `size`, each its own colour.
fn sheet(frames: u16, size: u16) -> Vec<u8> {
    (0..frames)
        .fold(RledBuilder::new(size, size), |sheet, frame| {
            let color = 0x0400 + frame;
            sheet.frame(|f| (0..size).fold(f, |f, _| f.line().pixels(&vec![color; size.into()])))
        })
        .build()
}

/// Where the stellars of Alpha are.
const STELLARS: [Point; 2] = [Point::new(0.0, -600.0), Point::new(300.0, -200.0)];

/// The first `chär` (if `with_character`) flies ship 128 from Alpha (128),
/// which holds Alpha Prime (128) at (0, -600), an 8 x 8 sprite, and Alpha
/// Station (129) at (300, -200), a 6 x 6 one. The ship's sheet is 36
/// rotations of 1 x 1, its glow's 36 of 3 x 3. Beta (129) holds nothing. The status bar is `ïntf`
/// 128, over a 194 x 16 `PICT` 700. The ship has no lights.
fn data(with_character: bool) -> Rc<GameData> {
    data_with(with_character, false)
}

/// [`data`], with the ship's lights, 36 rotations of 3 x 3 in `rlëD`
/// 2200, if `lights`.
fn data_with(with_character: bool, lights: bool) -> Rc<GameData> {
    data_of(with_character, lights, false)
}

/// [`data_with`], with the stars' `spïn` 700, one frame of 5 x 5 in
/// `rlëD` 700, if `stars`.
fn data_of(with_character: bool, lights: bool, stars: bool) -> Rc<GameData> {
    let mut fork = ForkBuilder::new()
        .resource(Ship::TYPE, 128, Some(b"Shuttle"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(lights))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(RLED, 2100, None, &sheet(36, 3))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system(0, &[128, 129]))
        .resource(System::TYPE, 129, Some(b"Beta"), &system(600, &[]))
        .resource(
            Stellar::TYPE,
            128,
            Some(b"Alpha Prime"),
            &stellar(0, -600, 0),
        )
        .resource(
            Stellar::TYPE,
            129,
            Some(b"Alpha Station"),
            &stellar(300, -200, 1),
        )
        .resource(Spin::TYPE, 1000, None, &spin(1000))
        .resource(Spin::TYPE, 1001, None, &spin(1001))
        .resource(RLED, 1000, None, &sheet(1, 8))
        .resource(RLED, 1001, None, &sheet(1, 6))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &interface(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture());
    if with_character {
        fork = fork.resource(Character::TYPE, 128, Some(b"Pilot"), &character());
    }
    if lights {
        fork = fork.resource(RLED, 2200, None, &sheet(36, 3));
    }
    if stars {
        fork = fork.resource(Spin::TYPE, 700, None, &spin(700)).resource(
            RLED,
            700,
            None,
            &sheet(1, 5),
        );
    }
    Rc::new(one_file(fork.build().bytes).load())
}

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    /// Display frames a second.
    fps: u64,
    /// Frames sent since flight was entered.
    frames: u64,
    /// The flight keys held.
    held: Vec<Key>,
}

impl Harness {
    /// The app on the ship browser, before any frame.
    fn new(fps: u64, with_character: bool) -> Self {
        Self::over(fps, data(with_character))
    }

    /// The app on the ship browser over `data`, before any frame, its
    /// flights drawing the developer's title and help line, which these
    /// tests read the frame's shape and the system shown by.
    fn over(fps: u64, data: Rc<GameData>) -> Self {
        let screen = start_screen(Rc::clone(&data)).with_flight_overlays(FlightOverlays {
            title: true,
            help: true,
            cheats: false,
        });
        Self {
            app: App::new(&FakeWindow, data, screen),
            gpu: RecordingGpu::new(),
            fps,
            frames: 0,
            held: Vec::new(),
        }
    }

    /// The app in flight, entered from the ship browser, before any frame.
    fn flying(fps: u64) -> Self {
        Self::new(fps, true).into_flight()
    }

    /// [`Harness::flying`], with the ship's lights.
    fn flying_with_lights(fps: u64) -> Self {
        Self::over(fps, data_with(true, true)).into_flight()
    }

    /// [`Harness::flying`], with the stars' sheet.
    fn flying_with_stars(fps: u64) -> Self {
        Self::over(fps, data_of(true, false, true)).into_flight()
    }

    /// Enters flight from the ship browser.
    fn into_flight(mut self) -> Self {
        self.press(Key::Char('f'));
        assert_eq!(self.showing(), Showing::Flight);
        self
    }

    fn handle(&mut self, event: WindowEvent) -> Control {
        self.app.handle(event, &mut FakeWindow, &mut self.gpu)
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(self.handle(event), Control::Continue, "{event:?}");
    }

    fn key(&mut self, key: Key, pressed: bool) {
        self.send(WindowEvent::Key {
            key,
            pressed,
            repeat: false,
        });
    }

    fn press(&mut self, key: Key) {
        self.key(key, true);
        self.key(key, false);
    }

    /// Holds exactly `keys` of the flight keys, pressing and releasing
    /// only those that change.
    fn hold(&mut self, keys: &[Key]) {
        for key in self.held.clone() {
            if !keys.contains(&key) {
                self.key(key, false);
            }
        }
        for &key in keys {
            if !self.held.contains(&key) {
                self.key(key, true);
            }
        }
        self.held = keys.to_vec();
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn flight(&self) -> &FlightView<Rc<GameData>> {
        self.app.screen().flight_view().expect("flight entered")
    }

    fn ship(&self) -> ShipState {
        *self.flight().session().expect("flying").player()
    }

    /// Sends the next redraw, at `frames / fps` seconds to the nanosecond
    /// (a display clock's reading, not a sum of rounded frame times), and
    /// returns its frame.
    fn frame(&mut self) -> Frame {
        self.frames += 1;
        let elapsed = Duration::from_nanos(self.frames * 1_000_000_000 / self.fps);
        self.send(WindowEvent::Redraw { elapsed });
        let frame = (*self.gpu.submits().last().expect("a frame")).clone();
        assert_eq!(self.app.take_failures(), []);
        frame
    }

    /// Sends `seconds` of redraws, checking the ship never passes its top
    /// speed, and returns the last frame.
    fn run(&mut self, seconds: f64) -> Frame {
        let count = (seconds * self.fps as f64).round() as u64;
        let max_speed = self.flight().session().expect("flying").top_speed();
        let mut last = None;
        for _ in 0..count {
            last = Some(self.frame());
            let speed = self.ship().velocity.length();
            assert!(speed <= max_speed + 1e-4, "{speed} past {max_speed}");
        }
        last.expect("at least one frame")
    }
}

fn shape(frame: &Frame) -> Vec<(&'static str, usize)> {
    frame
        .batches
        .iter()
        .map(|batch| match batch {
            Batch::Sprites { quads, .. } => ("sprites", quads.len()),
            Batch::Solid(quads) => ("solid", quads.len()),
            Batch::Text(runs) => ("text", runs.len()),
        })
        .collect()
}

/// Batch `at`'s blend and quad count, if it is a sprites batch.
fn sprite_batch(frame: &Frame, at: usize) -> Option<(Blend, usize)> {
    match frame.batches.get(at) {
        Some(Batch::Sprites { blend, quads, .. }) => Some((*blend, quads.len())),
        _ => None,
    }
}

/// Every sprite drawn, in order: the stellars, the ship, then the status
/// bar's picture.
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

fn centre(rect: Rect) -> Point {
    Point::new(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0)
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

fn point(v: Vec2) -> Point {
    Point::new(v.x, v.y)
}

fn distance(a: Point, b: Point) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

#[test]
fn f_enters_flight_with_the_first_chärs_ship_in_its_first_system_that_exists() {
    let mut harness = Harness::flying(60);
    let session = harness.flight().session().expect("flying");
    assert_eq!(session.system(), SystemId(128));
    assert_eq!(session.ship(), ShipId(128));

    let first = harness.frame();
    // No stars, as the fixture has no `spïn` 700; the two stellars, then
    // their names; the ship; the title and help lines; then the HUD: the
    // status bar's picture, the radar's two small ovals (4 pixels each)
    // and the player's pixel, and three bars, then the nav area's "Nav
    // System Off" (never the system's name), and the target and secondary
    // weapon lines. This `ïntf` has no `CargoArea`, so no cargo lines.
    let shape = shape(&first);
    assert_eq!(
        shape,
        [
            ("sprites", 2),
            ("text", 2),
            ("sprites", 1),
            ("text", 2),
            ("sprites", 1),
            ("solid", 12),
            ("text", 3)
        ]
    );
    let start = quads(&first);
    // The ship at the play area's centre, (415, 384).
    assert_eq!(start[0].dest, centred(415.0, -216.0, 8.0, 8.0));
    assert_eq!(start[1].dest, centred(715.0, 184.0, 6.0, 6.0));
    assert_eq!(start[2].dest, centred(415.0, 384.0, 1.0, 1.0), "the ship");
    for expected in ["Alpha Prime", "Alpha Station", "Alpha (sÿst 128)"] {
        assert!(
            texts(&first).contains(&expected.to_owned()),
            "{expected:?} in {:?}",
            texts(&first)
        );
    }
}

/// The quads of the last solid batch: the HUD's radar dots and bars.
fn hud_solids(frame: &Frame) -> Vec<SolidQuad> {
    frame
        .batches
        .iter()
        .rev()
        .find_map(|batch| match batch {
            Batch::Solid(quads) => Some(quads.clone()),
            _ => None,
        })
        .expect("a solid batch")
}

/// The rectangle a solid quad's corners span.
fn span(quad: &SolidQuad) -> Rect {
    let xs = quad.corners.map(|c| c.x);
    let ys = quad.corners.map(|c| c.y);
    let min = |v: [f32; 4]| v.into_iter().fold(f32::INFINITY, f32::min);
    let max = |v: [f32; 4]| v.into_iter().fold(f32::NEG_INFINITY, f32::max);
    Rect {
        x: min(xs),
        y: min(ys),
        w: max(xs) - min(xs),
        h: max(ys) - min(ys),
    }
}

/// A 24-bit colour as the renderer submits it.
fn submitted(raw: u32) -> [f32; 4] {
    let [_, r, g, b] = raw.to_be_bytes();
    [r, g, b, 255].map(|c| f32::from(c) / 255.0)
}

/// The 1-pixel quads the radar frames a small stellar at `stellar` with,
/// to a ship drawn at `shown`: a 2×2 oval about the stock radar's centre,
/// (926, 96) on screen, plus a thirty-second of the offset.
fn on_radar(shown: Point, stellar: Point) -> Vec<Rect> {
    let x = radar_offset(stellar.x, shown.x, 926) as f32;
    let y = radar_offset(stellar.y, shown.y, 96) as f32;
    [(x - 1.0, y - 1.0), (x, y - 1.0), (x - 1.0, y), (x, y)]
        .map(|(x, y)| pixel(x, y))
        .to_vec()
}

/// The pixel at (`x`, `y`).
fn pixel(x: f32, y: f32) -> Rect {
    Rect {
        x,
        y,
        w: 1.0,
        h: 1.0,
    }
}

/// The radar's ovals drawn: the first 8 HUD quads' rects, in the dim
/// radar colour.
#[allow(clippy::float_cmp)]
fn ovals(solids: &[SolidQuad]) -> Vec<Rect> {
    solids[..8]
        .iter()
        .map(|quad| {
            assert_eq!(quad.color, submitted(DIM_RADAR));
            span(quad)
        })
        .collect()
}

#[test]
#[allow(clippy::float_cmp)]
fn the_hud_is_drawn_while_flying() {
    let mut harness = Harness::flying(60);
    let first = harness.frame();

    // The status bar's picture, against the right edge.
    let status = *quads(&first).last().expect("the status bar");
    assert_eq!(
        status.dest,
        Rect {
            x: 830.0,
            y: 0.0,
            w: 194.0,
            h: 16.0,
        }
    );

    // A small oval for each stellar, dim, the player's pixel at the
    // centre, bright, then the full shield, armour and fuel bars at their
    // ïntf areas, moved to the bar.
    let solids = hud_solids(&first);
    assert_eq!(solids.len(), 12, "{solids:?}");
    let still = |stellar| on_radar(Point::new(0.0, 0.0), stellar);
    assert_eq!(
        ovals(&solids),
        [still(STELLARS[0]), still(STELLARS[1])].concat()
    );
    assert_eq!(
        (span(&solids[8]), solids[8].color),
        (pixel(926.0, 96.0), submitted(RADAR))
    );
    let bars: Vec<(Rect, [f32; 4])> = solids[9..].iter().map(|q| (span(q), q.color)).collect();
    let bar = |top: f32| Rect {
        x: 865.0,
        y: top,
        w: 149.0,
        h: 7.0,
    };
    assert_eq!(
        bars,
        [
            (bar(199.0), submitted(SHIELD)),
            (bar(216.0), submitted(ARMOR)),
            (bar(234.0), submitted(FUEL)),
        ]
    );

    // The nav area comes last but for the target and secondary weapon
    // lines, with nothing selected and no course: "Nav System Off", not
    // the system's name. With no `CargoArea` there is no room for the
    // date.
    let Some(Batch::Text(runs)) = first.batches.last() else {
        panic!("text last: {:?}", shape(&first))
    };
    let lines: Vec<&str> = runs.iter().map(|run| run.text.as_str()).collect();
    assert_eq!(lines, [NAV_OFF, "No Target", "No Secondary Weapon"]);
    // "Nav System Off" on its baseline 22 below the nav area's top
    // (254), its top 12 (the bar's font size) above that; at the area's
    // left, as this harness measures no text.
    assert_eq!(runs[0].origin_px, (838.0, 264.0));
    assert!(!lines.contains(&"Alpha"), "{lines:?}");

    // Flying up moves the ovals down the radar, with the ship as drawn
    // when the radar last refreshed: within the last 8 ticks, 16 frames.
    // 40 ticks of 0.06 more a tick take it 49 up, past a radar pixel's 32.
    harness.hold(&[Key::Up]);
    let mut shown = Vec::new();
    let mut flown = harness.frame();
    for _ in 1..80 {
        shown.push(harness.flight().shown_position());
        flown = harness.frame();
    }
    shown.push(harness.flight().shown_position());
    assert!(shown[79].y < -40.0, "{shown:?}");
    let moved = hud_solids(&flown);
    let now = ovals(&moved);
    assert!(now[0].y > ovals(&solids)[0].y, "moved down");
    let refreshed = shown[64..]
        .iter()
        .any(|&at| now == [on_radar(at, STELLARS[0]), on_radar(at, STELLARS[1])].concat());
    assert!(refreshed, "{now:?} from none of {:?}", &shown[64..]);
    assert_eq!(moved[8..], solids[8..], "the player's pixel and the bars");
}

#[test]
fn the_default_keys_fly_the_ship_and_the_camera_follows_it() {
    let mut harness = Harness::flying(60);
    let start = quads(&harness.frame());

    // Up thrusts the ship up the screen; the camera follows, so the
    // stellars move down by as much, and the ship stays in the middle.
    harness.hold(&[Key::Up]);
    let thrust = harness.run(1.0);
    let ship = harness.ship();
    // The first tick held at rest by the new session's box of none, then
    // 29 ticks of 0.06 more a tick: 0.06 x 435 = 26.1 up.
    assert!(ship.position.y < -26.0, "{ship:?}");
    let camera = harness.flight().camera().center();
    assert_eq!(camera, harness.flight().shown_position());
    assert!(camera.y < -25.0, "{camera:?}");
    let moved = quads(&thrust);
    for (now, was) in moved.iter().zip(&start).take(2) {
        let (now, was) = (centre(now.dest), centre(was.dest));
        assert!(
            (now.y - (was.y - camera.y)).abs() < 1e-3,
            "{was:?} to {now:?}"
        );
        assert!((now.x - was.x).abs() < 1e-3);
    }
    assert_eq!(moved[2].dest, start[2].dest, "the ship stays in the middle");

    // Releasing Up coasts on.
    harness.hold(&[]);
    harness.run(0.5);
    let coasting = harness.ship();
    harness.run(0.5);
    assert_eq!(harness.ship().velocity, coasting.velocity);
    assert_ne!(harness.ship().position, coasting.position);

    // Right turns the ship and its sprite frame.
    harness.hold(&[Key::Right]);
    let turned = harness.run(0.5);
    assert!(harness.ship().heading > 40.0, "{:?}", harness.ship());
    assert_ne!(quads(&turned)[2].uv, start[2].uv, "another rotation frame");

    // Down turns the ship to face against its motion, stopping within a
    // whole-degree turn of the whole-degree aim.
    harness.hold(&[Key::Down]);
    harness.run(2.5);
    let ship = harness.ship();
    let rate = harness
        .flight()
        .session()
        .expect("flying")
        .handling()
        .turn_rate;
    let behind = reverse_aim(ship.velocity).expect("moving");
    assert!(
        settled(ship.heading, rate, behind, PlayerTurn::WholeDegrees),
        "{ship:?} against {behind}"
    );
}

#[test]
fn the_stars_are_drawn_first_each_its_own_ored_sprite_over_the_play_area() {
    let mut harness = Harness::flying_with_stars(60);
    let first = harness.frame();
    let stars: Vec<QuadInstance> = first
        .batches
        .iter()
        .take(20)
        .map(|batch| match batch {
            Batch::Sprites {
                blend: Blend::Or,
                quads,
                ..
            } => {
                assert_eq!(quads.len(), 1);
                quads[0]
            }
            other => panic!("a star: {other:?}"),
        })
        .collect();
    assert_eq!(stars.len(), 20);
    for star in &stars {
        assert_eq!((star.dest.w, star.dest.h), (5.0, 5.0));
        assert!((0.0..=830.0).contains(&centre(star.dest).x), "{star:?}");
        assert!((0.0..=768.0).contains(&centre(star.dest).y), "{star:?}");
    }
    assert_eq!(
        sprite_batch(&first, 20),
        Some((Blend::Normal, 2)),
        "then the stellars"
    );
}

#[test]
fn holding_up_ors_the_engine_glow_over_the_ship_and_it_fades_out_after_release() {
    let mut harness = Harness::flying(60);
    let ship_and_glow = |frame: &Frame| (sprite_batch(frame, 2), sprite_batch(frame, 3));
    let unlit = |frame: &Frame| {
        let (ship, next) = ship_and_glow(frame);
        ship == Some((Blend::Normal, 1)) && !matches!(next, Some((Blend::Or, _)))
    };
    let first = harness.frame();
    assert!(unlit(&first), "no glow at rest: {:?}", shape(&first));

    // A second of thrust: the glow's base is at cruise, 24.
    harness.hold(&[Key::Up]);
    let thrusting = harness.run(1.0);
    assert_eq!(
        ship_and_glow(&thrusting),
        (Some((Blend::Normal, 1)), Some((Blend::Or, 1))),
        "the ship, then its glow ORed over it"
    );
    let drawn = quads(&thrusting);
    let (ship, glow) = (drawn[2].dest, drawn[3].dest);
    assert_eq!((ship.w, glow.w), (1.0, 3.0));
    assert_eq!(centre(glow), centre(ship), "centred on the ship");
    // An OR quad's tint is its level in 32nds: lights_tint(level), as the
    // batcher turns it into levels.
    let tint = drawn[3].tint;
    assert!(
        (20..=25_u8).any(|level| {
            let n = f32::from(level) / 32.0;
            tint.iter()
                .zip([n, n, n, 1.0])
                .all(|(got, want)| (got - want).abs() < 1e-6)
        }),
        "a flickering partial level: {tint:?}"
    );

    // Three ticks after release the glow is fading, not out.
    harness.hold(&[]);
    let released = harness.run(0.1);
    assert_eq!(
        sprite_batch(&released, 3),
        Some((Blend::Or, 1)),
        "still glowing: {:?}",
        shape(&released)
    );

    let coasting = harness.run(1.0);
    assert!(unlit(&coasting), "faded out: {:?}", shape(&coasting));
}

#[test]
fn the_ships_lights_are_ored_over_it() {
    let mut harness = Harness::flying_with_lights(60);
    let frame = harness.frame();

    assert_eq!(
        sprite_batch(&frame, 2),
        Some((Blend::Normal, 1)),
        "the ship"
    );
    assert_eq!(sprite_batch(&frame, 3), Some((Blend::Or, 1)), "its lights");
    let drawn = quads(&frame);
    let (ship, lights) = (drawn[2].dest, drawn[3].dest);
    assert_eq!((ship.w, lights.w, lights.h), (1.0, 3.0, 3.0));
    assert_eq!(centre(lights), centre(ship), "centred on the ship");
}

/// Flies to `target` with the default keys alone: turns towards it,
/// thrusts, and near it (or when drifting off) turns against the motion
/// with Down and thrusts to brake. Returns once the ship is within 50
/// pixels, or panics after a minute.
fn fly_to(harness: &mut Harness, target: Point) -> Frame {
    let handling = harness.flight().session().expect("flying").handling();
    for _ in 0..60 * harness.fps {
        let ship = harness.ship();
        let to_target = Vec2::new(target.x - ship.position.x, target.y - ship.position.y);
        let range = to_target.length();
        if range < 50.0 {
            return harness.frame();
        }
        let speed = ship.velocity.length();
        let bearing = heading_of(to_target);
        let half_turn_ticks = 180.0 / handling.turn_rate;
        let stopping = speed * speed / (2.0 * handling.accel) + half_turn_ticks * speed + 20.0;
        let drifting =
            speed > 0.0 && shortest_turn(heading_of(ship.velocity), bearing).abs() > 30.0;
        let braking = speed > handling.accel && (range < stopping || drifting);
        let keys: &[Key] = if braking {
            // Down stops within a turn of its aim.
            let facing_back = reverse_aim(ship.velocity).is_none_or(|behind| {
                settled(
                    ship.heading,
                    handling.turn_rate,
                    behind,
                    PlayerTurn::WholeDegrees,
                )
            });
            if facing_back {
                &[Key::Down, Key::Up]
            } else {
                &[Key::Down]
            }
        } else {
            let off = shortest_turn(ship.heading, bearing);
            if off > handling.turn_rate {
                &[Key::Right]
            } else if off < -handling.turn_rate {
                &[Key::Left]
            } else {
                &[Key::Up]
            }
        };
        harness.hold(keys);
        harness.run(1.0 / harness.fps as f64);
    }
    panic!("never reached {target:?}: {:?}", harness.ship());
}

#[test]
fn the_player_can_fly_to_every_stellar_in_the_system() {
    let mut harness = Harness::flying(30);
    harness.frame();
    for (index, &target) in STELLARS.iter().enumerate() {
        let arrived = fly_to(&mut harness, target);
        let ship = point(harness.ship().position);
        assert!(distance(ship, target) < 50.0, "{ship:?} from {target:?}");
        // The camera is on the ship, so the stellar is drawn near the
        // play area's centre, (415, 384) (the ship is drawn up to a step
        // behind).
        let drawn = centre(quads(&arrived)[index].dest);
        assert!(
            distance(drawn, Point::new(415.0, 384.0)) < 55.0,
            "stellar {index} drawn at {drawn:?}"
        );
    }
}

/// Flies one open-loop key script, its keys changing only on half-second
/// boundaries, for three seconds at `fps`, and returns the ship.
fn scripted(fps: u64) -> ShipState {
    let mut harness = Harness::flying(fps);
    let script: [&[Key]; 6] = [
        &[Key::Up],
        &[Key::Up, Key::Left],
        &[Key::Left],
        &[Key::Down],
        &[Key::Down],
        &[],
    ];
    for keys in script {
        harness.hold(keys);
        harness.run(0.5);
    }
    harness.ship()
}

#[test]
fn the_same_keys_fly_the_same_through_the_app_at_30_60_and_120_fps() {
    let at_30 = scripted(30);
    assert!(at_30.position.y < 0.0, "{at_30:?}");
    assert!(at_30.heading > 0.0, "{at_30:?}");
    assert_eq!(scripted(60), at_30);
    assert_eq!(scripted(120), at_30);
}

#[test]
fn escape_goes_back_to_where_flight_was_entered_and_then_quits_as_before() {
    let mut harness = Harness::new(60, true);
    harness.press(Key::Tab);
    assert_eq!(harness.showing(), Showing::GalaxyMap);
    harness.press(Key::Char('f'));
    assert_eq!(harness.showing(), Showing::Flight);
    harness.frame();
    harness.press(Key::Escape);
    assert_eq!(harness.showing(), Showing::GalaxyMap);
    let map = harness.frame();
    assert!(
        texts(&map).contains(&"Tab: ships / galaxy map   F: fly".to_owned()),
        "{:?}",
        texts(&map)
    );
    assert_eq!(
        harness.handle(WindowEvent::Key {
            key: Key::Escape,
            pressed: true,
            repeat: false,
        }),
        Control::Exit
    );
}

#[test]
fn losing_focus_while_holding_up_stops_the_thrust() {
    let mut harness = Harness::flying(60);
    harness.key(Key::Up, true);
    harness.run(0.5);
    harness.send(WindowEvent::FocusLost);
    // Up's release went to another window.
    let coasting = harness.ship();
    harness.run(0.5);
    assert_eq!(harness.ship().velocity, coasting.velocity);
}

#[test]
fn without_a_chär_flight_says_it_cannot_start() {
    let mut harness = Harness::new(60, false);
    harness.press(Key::Char('f'));
    assert_eq!(harness.showing(), Showing::Flight);
    let frame = harness.frame();
    assert!(
        texts(&frame).contains(&"Cannot start flight: no chär to start from".to_owned()),
        "{:?}",
        texts(&frame)
    );
    assert!(quads(&frame).is_empty());
    harness.press(Key::Escape);
    assert_eq!(harness.showing(), Showing::ShipBrowser);
}
