//! The flight HUD's navigation area through the whole app: synthetic game
//! data, wired to the renderer and the recording Gpu and driven only by
//! window events, with every key made by the platform's own translation
//! of a physical key.
//!
//! Alpha's navigation defaults are Alpha Minor, then Alpha Prime. With
//! nothing selected and no course the nav area says "No Destination". A
//! course plotted on flight's map to Beta, not yet explored, shows
//! "Hyperspace" over "Unexplored System". Tab selects Alpha Minor, then
//! Alpha Prime, then Alpha Minor again, and the selected stellar takes
//! the place of the course. Arriving in Beta clears both; a course back to
//! Alpha, explored, shows its name.
//!
//! The date, at the foot of the cargo area, reads "June 23, 1177 NC" and
//! moves on a day with each jump.

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova::platform;
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::interface::Interface;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_sim::flight::{heading_of, shortest_turn};
use nova_sim::hyperspace::MIN_JUMP_DISTANCE;
use nova_sim::{Session, ShipState, StellarId, SystemId};
use nova_view::flight::FlightView;
use nova_view::flight::hud::{NAV_HYPERSPACE, NAV_NO_DESTINATION, NAV_STELLAR, NAV_UNEXPLORED};
use nova_view::{MouseButton, Point};
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

/// A `chär` starting in ship 128 in system 128 on 23 June 1177, its
/// dates shown as stock's are: no `DatePrefix` (0x13A) and `DateSuffix`
/// (0x14A) " NC".
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x134, &[23, 6, 1177]);
    bytes[0x14A..0x14E].copy_from_slice(b" NC\0");
    bytes
}

/// A fast `shïp`: `Accel` 1500, `Speed` 1000 and `Maneuver` 30, with
/// `Shield` 30, `Fuel` 300 (three jumps) and `Armor` 45.
fn ship() -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[30, 1500, 1000, 30, 300]);
    put_i16s(&mut bytes, 0x0E, &[45]);
    bytes
}

/// Stock `ïntf` 128's areas and font, over background `PICT` 700: its
/// `NavArea` is (8, 254)-(184, 286) and its `CargoArea` (8, 458)-(184,
/// 552).
fn interface() -> Vec<u8> {
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
    put_i16s(&mut bytes, 0x58, &[458, 8, 552, 184]);
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

/// A `shän` whose base image is `rlëD` 2000, one set of 36 rotations.
fn ship_anim() -> Vec<u8> {
    let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[2000, 0, 1]);
    put_i16s(&mut bytes, 0x34, &[36]);
    bytes
}

/// An independent `sÿst` at map (`x`, 0) with these hyperlinks and these
/// navigation defaults, in order; every other slot is -1.
fn system(x: i16, links: &[i16], nav_defs: &[i16]) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0]);
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x04, links);
    put_i16s(&mut bytes, 0x24, nav_defs);
    put_i16s(&mut bytes, 0x66, &[-1]);
    bytes
}

/// A still `spöb` at (`x`, `y`) of graphic type 0: `spïn` 1000.
fn stellar(x: i16, y: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, y, 0]);
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

/// Alpha (128) and Beta (129), 100 apart on the map and linked. Alpha's
/// navigation defaults are Alpha Minor (131), at (0, 300), then Alpha
/// Prime (128), at (0, -300): not in resource order. Beta holds Beta
/// Prime (129). The first `chär` flies the fast ship 128 from Alpha. The
/// status bar is `ïntf` 128, over a 194 x 16 `PICT` 700.
fn data() -> Rc<GameData> {
    let fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Courier"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim())
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(
            System::TYPE,
            128,
            Some(b"Alpha"),
            &system(0, &[129], &[131, 128]),
        )
        .resource(System::TYPE, 129, Some(b"Beta"), &system(100, &[], &[129]))
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &stellar(0, -300))
        .resource(Stellar::TYPE, 129, Some(b"Beta Prime"), &stellar(0, -300))
        .resource(Stellar::TYPE, 131, Some(b"Alpha Minor"), &stellar(0, 300))
        .resource(Spin::TYPE, 1000, None, &spin(1000))
        .resource(RLED, 1000, None, &sheet(1, 8))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &interface(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture())
        .build()
        .bytes;
    let file = OneFile(fork);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    /// Frames sent, at 60 a second.
    frames: u64,
    /// The flight keys held.
    held: Vec<KeyCode>,
}

impl Harness {
    /// The app in flight, entered from the ship browser with F.
    fn flying() -> Self {
        let data = data();
        let screen = start_screen(Rc::clone(&data));
        let mut harness = Self {
            app: App::new(&FakeWindow, data, screen),
            gpu: RecordingGpu::new(),
            frames: 0,
            held: Vec::new(),
        };
        harness.press(KeyCode::KeyF);
        assert_eq!(harness.app.screen().showing(), Showing::Flight);
        harness
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue,
            "{event:?}"
        );
    }

    /// The physical key `code` going down (or, as `repeat`, held) or up,
    /// through the platform's translation.
    fn key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        let state = if pressed {
            ElementState::Pressed
        } else {
            ElementState::Released
        };
        self.send(platform::key_event(PhysicalKey::Code(code), state, repeat));
    }

    fn press(&mut self, code: KeyCode) {
        self.key(code, true, false);
        self.key(code, false, false);
    }

    /// Holds exactly `keys` of the flight keys, pressing and releasing
    /// only those that change.
    fn hold(&mut self, keys: &[KeyCode]) {
        for code in self.held.clone() {
            if !keys.contains(&code) {
                self.key(code, false, false);
            }
        }
        for &code in keys {
            if !self.held.contains(&code) {
                self.key(code, true, false);
            }
        }
        self.held = keys.to_vec();
    }

    /// Clicks the left button at `at` (window pixels are logical units).
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

    fn flight(&self) -> &FlightView<Rc<GameData>> {
        self.app.screen().flight_view().expect("flight entered")
    }

    fn session(&self) -> &Session {
        self.flight().session().expect("flying")
    }

    fn ship(&self) -> ShipState {
        *self.session().player()
    }

    /// Plots a course to system `id` on flight's map: M, a click, M.
    fn plot(&mut self, id: i16) {
        self.frame();
        self.press(KeyCode::KeyM);
        let map = self.flight().course_map();
        let system = map.model().system(SystemId(id)).expect("on the map");
        let at = map.view().world_to_screen(system.position());
        self.click(at);
        self.press(KeyCode::KeyM);
        assert!(!self.flight().map_open());
    }

    /// Sends the next redraw, 1/60 s after the last, and returns its frame.
    fn frame(&mut self) -> Frame {
        self.frames += 1;
        let elapsed = Duration::from_nanos(self.frames * 1_000_000_000 / 60);
        self.send(WindowEvent::Redraw { elapsed });
        let frame = (*self.gpu.submits().last().expect("a frame")).clone();
        assert_eq!(self.app.take_failures(), []);
        frame
    }

    /// Sends `seconds` of redraws.
    fn run(&mut self, seconds: u64) {
        for _ in 0..seconds * 60 {
            self.frame();
        }
    }

    /// Flies out from the centre until the ship is at least the minimum
    /// jump distance from it, then lets go: first Down, to face away from
    /// the centre while drifting in, then Up.
    fn fly_out(&mut self) {
        for _ in 0..600 {
            let ship = self.ship();
            if ship.position.length() >= MIN_JUMP_DISTANCE {
                self.hold(&[]);
                return;
            }
            let moving_in =
                ship.position.x * ship.velocity.x + ship.position.y * ship.velocity.y < 0.0;
            let facing_out = ship.position.length() == 0.0
                || shortest_turn(ship.heading, heading_of(ship.position)).abs() < 1e-3;
            if moving_in && !facing_out {
                self.hold(&[KeyCode::ArrowDown]);
            } else {
                self.hold(&[KeyCode::ArrowUp]);
            }
            self.frame();
        }
        panic!("never got out: {:?}", self.ship());
    }

    /// The texts the next frame draws in the nav area: stock `ïntf` 128's
    /// (8, 254)-(184, 286), on the bar against the right edge of the
    /// 1024-wide window, 194 wide.
    fn nav(&mut self) -> Vec<String> {
        self.texts_on_bar(254.0..=286.0)
    }

    /// The texts the next frame draws in the cargo area, where the date
    /// goes: stock `ïntf` 128's (8, 458)-(184, 552), on the bar.
    fn date(&mut self) -> Vec<String> {
        self.texts_on_bar(458.0..=552.0)
    }

    /// The texts the next frame draws starting across the bar's areas
    /// (from 8 to 184 of its 194, against the right edge of the 1024-wide
    /// window) at a height in `ys`.
    fn texts_on_bar(&mut self, ys: std::ops::RangeInclusive<f32>) -> Vec<String> {
        let frame = self.frame();
        frame
            .batches
            .iter()
            .flat_map(|batch| match batch {
                Batch::Text(runs) => runs.clone(),
                _ => Vec::new(),
            })
            .filter(|run| {
                let (x, y) = run.origin_px;
                (838.0..=1014.0).contains(&x) && ys.contains(&y)
            })
            .map(|run| run.text)
            .collect()
    }
}

#[test]
fn tab_and_the_course_map_set_what_the_hud_navigation_area_shows() {
    let mut game = Harness::flying();
    assert_eq!(game.nav(), [NAV_NO_DESTINATION]);

    game.plot(129);
    assert_eq!(game.nav(), [NAV_HYPERSPACE, NAV_UNEXPLORED]);

    // Tab walks Alpha's navigation defaults in their order, and wraps; the
    // selected stellar takes the place of the course.
    game.press(KeyCode::Tab);
    assert_eq!(game.nav(), [NAV_STELLAR, "Alpha Minor"]);
    game.press(KeyCode::Tab);
    assert_eq!(game.nav(), [NAV_STELLAR, "Alpha Prime"]);
    game.key(KeyCode::Tab, true, false);
    game.key(KeyCode::Tab, true, true);
    game.key(KeyCode::Tab, false, false);
    assert_eq!(
        game.nav(),
        [NAV_STELLAR, "Alpha Minor"],
        "wrapped, once a press"
    );
    assert_eq!(game.session().nav_target(), Some(StellarId(131)));
    assert_eq!(
        game.session().course(),
        [SystemId(129)],
        "the course is kept"
    );
    assert_eq!(game.app.screen().showing(), Showing::Flight);

    // Arriving clears the target, and the course is done.
    game.fly_out();
    game.press(KeyCode::KeyJ);
    game.run(2);
    assert_eq!(game.session().system(), SystemId(129));
    assert_eq!(game.nav(), [NAV_NO_DESTINATION]);

    // A course back to Alpha, explored, names it.
    game.plot(128);
    assert_eq!(game.nav(), [NAV_HYPERSPACE, "Alpha"]);
    game.press(KeyCode::Tab);
    assert_eq!(game.nav(), [NAV_STELLAR, "Beta Prime"]);
}

#[test]
fn the_hud_date_moves_on_a_day_with_each_jump() {
    let mut game = Harness::flying();
    assert_eq!(game.date(), ["June 23, 1177 NC"]);

    game.plot(129);
    game.fly_out();
    game.press(KeyCode::KeyJ);
    game.run(2);
    assert_eq!(game.session().system(), SystemId(129));
    assert_eq!(game.date(), ["June 24, 1177 NC"]);

    game.plot(128);
    game.fly_out();
    game.press(KeyCode::KeyJ);
    game.run(2);
    assert_eq!(game.session().system(), SystemId(128));
    assert_eq!(game.date(), ["June 25, 1177 NC"]);
}
