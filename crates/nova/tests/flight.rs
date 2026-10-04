//! The app flying the first `chär`'s ship in its starting system, over
//! synthetic game data, wired to the renderer and the recording Gpu and
//! driven only by key events and redraws, as the window sends them.

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::RLED;
use nova_data::graphics::fixture::RledBuilder;
use nova_data::records::character::Character;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, QuadInstance, Rect};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_sim::flight::{heading_of, shortest_turn};
use nova_sim::{ShipId, ShipState, SystemId, Vec2};
use nova_view::flight::FlightView;
use nova_view::{Key, Point};

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

/// A `chär` starting in ship 128, its systems -1, 999 (missing), 128 and
/// -1: the first that exists is 128.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, -1, 999, 128, -1]);
    bytes
}

/// A `shïp` with `Accel` 300, `Speed` 300 and `Maneuver` 30: 3 pixels a
/// tick at most, 0.1 more a tick, 3° a tick.
fn ship() -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[300, 300, 30]);
    bytes
}

/// A `shän` whose base image is `rlëD` 2000, one set of 36 rotations.
fn ship_anim() -> Vec<u8> {
    let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[2000, 0, 1]);
    put_i16s(&mut bytes, 0x34, &[36]);
    bytes
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
/// rotations of 1 x 1. Beta (129) holds nothing.
fn data(with_character: bool) -> Rc<GameData> {
    let mut fork = ForkBuilder::new()
        .resource(Ship::TYPE, 128, Some(b"Shuttle"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim())
        .resource(RLED, 2000, None, &sheet(36, 1))
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
        .resource(RLED, 1001, None, &sheet(1, 6));
    if with_character {
        fork = fork.resource(Character::TYPE, 128, Some(b"Pilot"), &character());
    }
    let file = OneFile(fork.build().bytes);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
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
        let data = data(with_character);
        Self {
            app: App::new(&FakeWindow, Rc::clone(&data), start_screen(data)),
            gpu: RecordingGpu::new(),
            fps,
            frames: 0,
            held: Vec::new(),
        }
    }

    /// The app in flight, entered from the ship browser, before any frame.
    fn flying(fps: u64) -> Self {
        let mut harness = Self::new(fps, true);
        harness.press(Key::Char('f'));
        assert_eq!(harness.showing(), Showing::Flight);
        harness
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

    fn flight(&self) -> &FlightView {
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
        let max_speed = self
            .flight()
            .session()
            .expect("flying")
            .handling()
            .max_speed;
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

/// Every sprite drawn, in order: the stellars, then the ship.
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
    // The stars; the two stellars, then their names; the ship; then the
    // title and help lines.
    let shape = shape(&first);
    assert_eq!(shape[0].0, "solid");
    assert!(shape[0].1 > 0, "stars");
    assert_eq!(
        &shape[1..],
        [("sprites", 2), ("text", 2), ("sprites", 1), ("text", 2)]
    );
    let start = quads(&first);
    assert_eq!(start[0].dest, centred(512.0, -216.0, 8.0, 8.0));
    assert_eq!(start[1].dest, centred(812.0, 184.0, 6.0, 6.0));
    assert_eq!(start[2].dest, centred(512.0, 384.0, 1.0, 1.0), "the ship");
    for expected in ["Alpha Prime", "Alpha Station", "Alpha (sÿst 128)"] {
        assert!(
            texts(&first).contains(&expected.to_owned()),
            "{expected:?} in {:?}",
            texts(&first)
        );
    }
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
    assert!(ship.position.y < -40.0, "{ship:?}");
    let camera = harness.flight().camera().center();
    assert_eq!(camera, harness.flight().shown_position());
    assert!(camera.y < -40.0, "{camera:?}");
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

    // Down turns the ship to face against its motion.
    harness.hold(&[Key::Down]);
    harness.run(2.5);
    let ship = harness.ship();
    let behind = heading_of(ship.velocity * -1.0);
    assert!(
        shortest_turn(ship.heading, behind).abs() < 1e-2,
        "{ship:?} against {behind}"
    );
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
            let behind = heading_of(ship.velocity * -1.0);
            if shortest_turn(ship.heading, behind).abs() <= handling.turn_rate {
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
        // middle of the screen (the ship is drawn up to a step behind).
        let drawn = centre(quads(&arrived)[index].dest);
        assert!(
            distance(drawn, Point::new(512.0, 384.0)) < 55.0,
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
        texts(&map).contains(&"Tab: ships / galaxy map   F: fly   I: about".to_owned()),
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
