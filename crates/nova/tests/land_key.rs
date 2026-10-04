//! The land key through the whole app: synthetic game data, wired to the
//! renderer and the recording Gpu and driven only by window events, with
//! every key made by the platform's own translation of a physical key.
//!
//! Alpha holds two planets the ship starts over, both 64 x 64: Alpha
//! Prime (128) at the centre and Alpha Minor (131) just below it. Its
//! navigation defaults are Alpha Minor, then Alpha Prime, so Tab selects
//! Alpha Minor first, though Alpha Prime is nearer.
//!
//! - With no target, L requests clearance at the nearest, Alpha Prime: its
//!   traffic control's reply shows, the HUD's nav area shows it, and the
//!   ship stays in flight. A second L lands there.
//! - With Alpha Minor chosen by Tab, L lands there, not on Alpha Prime.
//! - Moving too fast, or far from every stellar, the second L says so in
//!   the original's words (`STR#` 2002 #72, #68) and the ship flies on.

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
use nova_sim::{Session, ShipState, StellarId};
use nova_view::flight::FlightView;
use nova_view::flight::hud::{NAV_NO_DESTINATION, NAV_STELLAR};
use nova_view::spaceport::SpaceportView;
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

/// A `chär` starting in ship 128 in system 128.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x134, &[23, 6, 1177]);
    bytes
}

/// An average `shïp`: 3 pixels a tick at most, 0.1 more a tick, 3° a tick.
fn ship() -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[30, 300, 300, 30, 300]);
    bytes
}

/// Stock `ïntf` 128's areas and font, over background `PICT` 700: its
/// `NavArea` is (8, 254)-(184, 286).
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

/// An independent `sÿst` with these navigation defaults, in order.
fn system(nav_defs: &[i16]) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, nav_defs);
    put_i16s(&mut bytes, 0x66, &[-1]);
    bytes
}

/// A still planet anyone may land on at (0, `y`), of graphic type 0:
/// `spïn` 1000, 64 x 64 (a landing radius of 32).
fn planet(y: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, y, 0]);
    bytes[0x06..0x0A].copy_from_slice(&1_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x14, &[-1, 0, -1, -1]);
    bytes
}

/// A `spïn` naming `rlëD` 1000, one frame across.
fn spin() -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[1000, -1, 0, 0, 1, 1]);
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

/// Alpha (128), whose navigation defaults are Alpha Minor (131), at
/// (0, 24), then Alpha Prime (128), at the centre. The first `chär` flies
/// ship 128 from Alpha. The status bar is `ïntf` 128, over a 194 x 16
/// `PICT` 700.
fn data() -> Rc<GameData> {
    let fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Courier"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim())
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system(&[131, 128]))
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &planet(0))
        .resource(Stellar::TYPE, 131, Some(b"Alpha Minor"), &planet(24))
        .resource(Spin::TYPE, 1000, None, &spin())
        .resource(RLED, 1000, None, &sheet(1, 64))
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

/// "Alpha Prime traffic control reads you, you're cleared to land.": the
/// reply to L at Alpha Prime (`STR#` 2002 #78 and #98).
const CLEARED: &str = "Alpha Prime traffic control reads you, you're cleared to land.";

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    /// Frames sent, at 60 a second.
    frames: u64,
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
        };
        harness.press(KeyCode::KeyF);
        assert_eq!(harness.showing(), Showing::Flight);
        harness.frame();
        harness
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue,
            "{event:?}"
        );
    }

    /// The physical key `code` going down or up, through the platform's
    /// translation.
    fn key(&mut self, code: KeyCode, pressed: bool) {
        let state = if pressed {
            ElementState::Pressed
        } else {
            ElementState::Released
        };
        self.send(platform::key_event(PhysicalKey::Code(code), state, false));
    }

    fn press(&mut self, code: KeyCode) {
        self.key(code, true);
        self.key(code, false);
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

    fn ship(&self) -> ShipState {
        *self.session().player()
    }

    /// The stellar whose spaceport is showing, if any.
    fn spaceport(&self) -> Option<StellarId> {
        self.app
            .screen()
            .spaceport_view()
            .map(SpaceportView::stellar)
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

    /// Holds Up until the ship is moving faster than `speed`, then lets go.
    fn thrust_past(&mut self, speed: f32) {
        self.key(KeyCode::ArrowUp, true);
        for _ in 0..600 {
            if self.ship().velocity.length() > speed {
                self.key(KeyCode::ArrowUp, false);
                return;
            }
            self.frame();
        }
        panic!("never got faster than {speed}: {:?}", self.ship());
    }

    /// Every text the next frame draws.
    fn texts(&mut self) -> Vec<String> {
        self.frame()
            .batches
            .iter()
            .flat_map(|batch| match batch {
                Batch::Text(runs) => runs.iter().map(|run| run.text.clone()).collect(),
                _ => Vec::new(),
            })
            .collect()
    }

    /// The texts the next frame draws in the nav area: stock `ïntf` 128's
    /// (8, 254)-(184, 286), on the bar against the right edge of the
    /// 1024-wide window, 194 wide.
    fn nav(&mut self) -> Vec<String> {
        self.frame()
            .batches
            .iter()
            .flat_map(|batch| match batch {
                Batch::Text(runs) => runs.clone(),
                _ => Vec::new(),
            })
            .filter(|run| {
                let (x, y) = run.origin_px;
                (838.0..=1014.0).contains(&x) && (254.0..=286.0).contains(&y)
            })
            .map(|run| run.text)
            .collect()
    }
}

#[test]
fn with_no_target_l_requests_clearance_at_the_nearest_and_a_second_l_lands_there() {
    let mut game = Harness::flying();
    assert_eq!(game.nav(), [NAV_NO_DESTINATION]);

    game.press(KeyCode::KeyL);
    assert_eq!(game.showing(), Showing::Flight, "cleared, not landed");
    assert_eq!(game.session().nav_target(), Some(StellarId(128)));
    assert!(game.texts().iter().any(|text| text == CLEARED));
    assert_eq!(game.nav(), [NAV_STELLAR, "Alpha Prime"]);

    game.press(KeyCode::KeyL);
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(game.spaceport(), Some(StellarId(128)));
}

#[test]
fn with_a_stellar_chosen_by_tab_l_lands_on_it_not_the_nearest() {
    let mut game = Harness::flying();
    game.press(KeyCode::Tab);
    assert_eq!(game.nav(), [NAV_STELLAR, "Alpha Minor"]);
    game.press(KeyCode::KeyL);
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(game.spaceport(), Some(StellarId(131)));
}

#[test]
fn moving_too_fast_the_second_l_says_so_and_the_ship_flies_on() {
    let mut game = Harness::flying();
    game.thrust_past(1.5);
    game.press(KeyCode::KeyL);
    assert_eq!(game.session().nav_target(), Some(StellarId(128)));
    game.press(KeyCode::KeyL);
    assert_eq!(game.showing(), Showing::Flight);
    let texts = game.texts();
    assert!(
        texts
            .iter()
            .any(|text| text == "You're moving too fast to land on this planet."),
        "{texts:?}"
    );
    assert_eq!(
        game.session().nav_target(),
        Some(StellarId(128)),
        "kept: try again"
    );
}

#[test]
fn far_from_every_stellar_the_second_l_says_so_and_the_ship_flies_on() {
    let mut game = Harness::flying();
    game.thrust_past(2.9);
    while game.ship().position.length() < 200.0 {
        game.frame();
    }
    game.press(KeyCode::KeyL);
    game.press(KeyCode::KeyL);
    assert_eq!(game.showing(), Showing::Flight);
    let texts = game.texts();
    assert!(
        texts
            .iter()
            .any(|text| text == "You're too far away to land on this planet."),
        "{texts:?}"
    );
}
