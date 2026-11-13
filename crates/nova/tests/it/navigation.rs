//! The flight HUD's navigation area through the whole app: synthetic game
//! data, wired to the renderer and the recording Gpu and driven only by
//! window events, with every key made by the platform's own translation
//! of a physical key.
//!
//! A course plotted with Shift-clicks on flight's map reaches the session,
//! and the HUD's navigation area, drawn from the `ïntf`'s area, reaches
//! the Gpu and changes with it. What the nav area says, the date, the
//! cargo area, and Tab, Nav Off and Hyperspace Mode's effects on the
//! course are nova-view's and nova-sim's, pinned there.

use crate::common::Frames;

use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova::platform;
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::fixture::{
    CharacterBuilder, InterfaceBuilder, ShipAnimBuilder, ShipBuilder, SpinBuilder,
};
use nova_data::records::interface::Interface;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fixture::one_file;
use nova_data::{GameData, Record};
use nova_flight::SessionView;
use nova_flight::SessionViewExt as _;
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_sim::hyperspace::HyperlinkRule;
use nova_sim::{Session, SystemId};
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

fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
    }
}

/// A `chär` starting in ship 128 in system 128 on 23 June 1177, its
/// dates shown as stock's are: no `DatePrefix` (0x13A) and `DateSuffix`
/// (0x14A) " NC".
fn character() -> Vec<u8> {
    CharacterBuilder::new()
        .ship_type(128)
        .system(&[128, -1, -1, -1])
        .unknown_0x134(23)
        .unknown_0x136(6)
        .start_year(1177)
        .date_suffix(" NC")
        .build()
}

/// A fast `shïp`: `Accel` 1500, `Speed` 1000 and `Maneuver` 30, with
/// `Shield` 30, `Fuel` 300 (three jumps) and `Armor` 45.
fn ship() -> Vec<u8> {
    ShipBuilder::new()
        .shield(30)
        .accel(1500)
        .speed(1000)
        .maneuver(30)
        .fuel(300)
        .armor(45)
        .build()
}

/// Stock `ïntf` 128's areas and font, over background `PICT` 700: its
/// `NavArea` is (8, 254)-(184, 286) and its `CargoArea` (8, 458)-(184,
/// 552).
fn interface() -> Vec<u8> {
    InterfaceBuilder::new()
        .bright_text(0x00FF_FFFF)
        .dim_text(0x0080_8080)
        .radar_area([8, 8, 184, 184])
        .bright_radar(0x0000_FF00)
        .dim_radar(0x0000_8000)
        .shield_area([199, 35, 206, 184])
        .shield_color(0x0000_00FF)
        .armor_area([216, 35, 223, 184])
        .armor_color(0x00FF_0000)
        .fuel_area([234, 35, 241, 184])
        .fuel_full(0x00FF_FF00)
        .fuel_partial(0x0080_8000)
        .nav_area([254, 8, 286, 184])
        .cargo_area([458, 8, 552, 184])
        .status_font("Geneva")
        .stat_font_size(12)
        .status_bkgnd(700)
        .build()
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
    ShipAnimBuilder::new()
        .base_image_id(2000)
        .base_set_count(1)
        .frames_per(36)
        .build()
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
    SpinBuilder::new()
        .sprites_id(image)
        .masks_id(-1)
        .x_tiles(1)
        .y_tiles(1)
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
        .resource(
            System::TYPE,
            129,
            Some(b"Beta"),
            &system(100, &[128], &[129]),
        )
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
    Rc::new(one_file(fork).load())
}

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

    fn flight(&self) -> &SessionView<Rc<GameData>> {
        self.app.screen().flight_view().expect("flight entered")
    }

    fn session(&self) -> &Session {
        self.flight().session().expect("flying")
    }

    /// Plots a course to system `id` on flight's map: M, a Shift-click on
    /// each hop of the shortest route there, M, as `_DoSystemMap` builds a
    /// course a hop at a time; the close cues its first hop.
    fn plot(&mut self, id: i16) {
        self.frame();
        let session = self.session();
        let route = session
            .star_map()
            .route(session.system(), SystemId(id), HyperlinkRule::Engine)
            .expect("a route");
        self.press(KeyCode::KeyM);
        self.key(KeyCode::ShiftLeft, true, false);
        for hop in &route {
            let map = self.flight().course_map();
            let system = map.model().system(*hop).expect("on the map");
            let at = map.transform().world_to_screen(system.position());
            self.click(at);
        }
        self.key(KeyCode::ShiftLeft, false, false);
        assert_eq!(self.session().course(), route, "plotted");
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

    /// The texts the next frame draws in the nav area: stock `ïntf` 128's
    /// (8, 254)-(184, 286), on the bar against the right edge of the
    /// 1024-wide window, 194 wide.
    fn nav(&mut self) -> Vec<String> {
        self.texts_on_bar(254.0..=286.0)
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
fn a_course_plotted_on_flights_map_reaches_the_hud_navigation_area() {
    let mut game = Harness::flying();
    let before = game.nav();
    assert!(!before.is_empty(), "the nav area drawn");

    game.plot(129);
    game.settle();
    assert_eq!(game.session().course(), [SystemId(129)]);
    let after = game.nav();
    assert!(!after.is_empty(), "the nav area drawn");
    assert_ne!(after, before, "the course shown");
}

crate::common::frames!(Harness, frame);
