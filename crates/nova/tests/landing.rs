//! Landing through the whole app: synthetic game data (a pilot flying over
//! a planet with a trade center and a bar, its landscape and description,
//! the spaceport and bar backgrounds and Nova's button pictures) and a
//! synthetic interface file holding the stock "Spaceport" and "Bar"
//! dialogs, laid out by the real glyphon metrics, drawn through the
//! renderer into the recording Gpu and driven only by window events: L
//! lands and shows the spaceport, and Leave takes off back into flight.
//! The Bar opens the bar's own dialog, with its Hire Escort button, and
//! the Mission BBS its placeholder. With a recording audio port, landing
//! and Leave play their sounds and the music follows the screen.

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova::platform;
use nova_audio::recording::{AudioLog, RecordingAudio};
use nova_audio::{Audio, AudioCommand, AudioCore, Volume};
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
use nova_render::{Batch, FontFaces, Frame, Rect};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};
use nova_sim::{ShipState, StellarId, Vec2};
use nova_view::geometry::{Bounds, Point};
use nova_view::spaceport::SpaceportView;
use nova_view::spaceport::layout::{LANDSCAPE_ITEM, LEAVE_ITEM};
use nova_view::spaceport::service::DONE_BUTTON;
use nova_view::{Key, MouseButton};
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

/// A `chär` starting in ship 128 in system 128.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    bytes
}

/// An average `shïp`: 3 pixels a tick at most, 0.1 more a tick, 3° a tick.
fn ship() -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[30, 300, 300, 30, 300]);
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

/// The landing site's flags: it can be landed on, and has a trade center
/// and a bar.
const FLAGS: u32 = 0x01 | 0x02 | 0x40;

/// Alpha Prime's landing sound, its `CustSndID`: Port Kane's
/// "Federation Station.SFIL" in the stock data.
const LANDING_SOUND: i16 = 10_032;

/// Alpha Prime: a planet at (0, `y`) of graphic type 4 and no custom
/// picture, so its landscape is `PICT` 10004, with its own landing sound.
fn stellar(y: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, y, 4]);
    bytes[0x06..0x0A].copy_from_slice(&FLAGS.to_be_bytes());
    put_i16s(&mut bytes, 0x18, &[-1, LANDING_SOUND]);
    bytes
}

/// A `spïn` naming `rlëD` 1000, one frame across.
fn spin() -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[1000, -1, 0, 0, 1, 1]);
    bytes
}

/// The planet's description.
const DESCRIPTION: &str = "A quiet world of farms.";

fn description() -> Vec<u8> {
    let mut bytes = DESCRIPTION.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend([0xFF; 2]);
    bytes.extend([0; 34]);
    bytes
}

/// A resource: its type, ID, name and bytes.
type Resource = (ResType, i16, Option<&'static [u8]>, Vec<u8>);

/// The pilot flies ship 128 from Alpha (128), whose one stellar, Alpha
/// Prime (128), is at (0, `y`), an 8 x 8 sprite; with its landscape, its
/// description, the spaceport and bar backgrounds and the button
/// pictures.
fn game_data(y: i16) -> Rc<GameData> {
    let mut resources: Vec<Resource> = vec![
        (Character::TYPE, 128, None, character()),
        (Ship::TYPE, 128, None, ship()),
        (ShipAnim::TYPE, 128, None, ship_anim()),
        (RLED, 2000, None, sheet(36, 1)),
        (System::TYPE, 128, Some(b"Alpha"), system()),
        (Stellar::TYPE, 128, Some(b"Alpha Prime"), stellar(y)),
        (Spin::TYPE, 1004, None, spin()),
        (RLED, 1000, None, sheet(1, 8)),
        (Desc::TYPE, 128, None, description()),
        (PICT, 8500, Some(b"Spaceport"), pict(6, 5, [40, 40, 40])),
        (PICT, 8503, Some(b"Bar"), pict(6, 5, [60, 30, 10])),
        (PICT, 10_004, None, pict(6, 3, [0, 90, 0])),
    ];
    for state in [7500, 7503, 7506] {
        resources.push((PICT, state, None, pict(13, 25, [200, 0, 0])));
        resources.push((PICT, state + 1, None, pict(2, 25, [0, 200, 0])));
        resources.push((PICT, state + 2, None, pict(13, 25, [0, 0, 200])));
        resources.push((PICT, state + 100, None, pict(13, 25, [0, 0, 0])));
        resources.push((PICT, state + 102, None, pict(13, 25, [0, 0, 0])));
    }
    let fork = resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, name, data)| {
            fork.resource(*ty, *id, *name, data)
        })
        .build()
        .bytes;
    let file = OneFile(fork);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

/// One `DITL` user item: (left, top, width, height), enabled or not.
fn user_item((x, y, w, h): (i16, i16, i16, i16), enabled: bool) -> Vec<u8> {
    let mut bytes = vec![0; 4];
    bytes.extend(be(&[y, x, y + h, x + w]));
    bytes.extend([if enabled { 0 } else { 0x80 }, 0]);
    bytes
}

/// Stock "Spaceport": `DLOG` 1000, 618 x 517 and centred, and its fifteen
/// user items; and stock "Bar": `DLOG` 1013, 263 x 185 and centred, and
/// its ten user items, all disabled, 4, 6 and 8-10 parked outside it.
fn interface() -> InterfaceData {
    let mut dlog = be(&[-201, 60, 316, 678, 2]);
    dlog.extend([1, 0, 0, 0, 0, 0, 0, 0]);
    dlog.extend(be(&[1000]));
    dlog.extend([0, 0, 0xA8, 0x0A]);
    let items = [
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
    ];
    let mut ditl = be(&[items.len() as i16 - 1]);
    for (bounds, enabled) in items {
        ditl.extend(user_item(bounds, enabled));
    }
    let mut bar_dlog = be(&[40, 40, 225, 303, 2]);
    bar_dlog.extend([1, 0, 0, 0, 0, 0, 0, 0]);
    bar_dlog.extend(be(&[1013]));
    bar_dlog.extend([0, 0, 0xA8, 0x0A]);
    let bar_items = [
        (156, 154, 99, 26),
        (156, 125, 99, 26),
        (6, 154, 146, 26),
        (54, 214, 121, 25),
        (6, 125, 146, 26),
        (105, 227, 121, 25),
        (16, 10, 230, 106),
        (61, 303, 32, 32),
        (103, 303, 32, 32),
        (146, 303, 32, 32),
    ];
    let mut bar_ditl = be(&[bar_items.len() as i16 - 1]);
    for bounds in bar_items {
        bar_ditl.extend(user_item(bounds, false));
    }
    let bytes = [
        (Dlog::TYPE, 1000, dlog),
        (Ditl::TYPE, 1000, ditl),
        (Dlog::TYPE, 1013, bar_dlog),
        (Ditl::TYPE, 1013, bar_ditl),
    ]
    .iter()
    .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
        fork.resource(*ty, *id, None, data)
    })
    .build()
    .bytes;
    InterfaceData::load(&OneFile(bytes), Path::new("/Nova-DF.rsrc")).expect("loads")
}

/// Where the dialog is: (1024 - 618) / 2, (768 - 517) / 2, floored.
const ORIGIN: Point = Point::new(203.0, 125.0);

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
}

impl Harness {
    /// The app on the ship browser over a planet at (0, `y`), before any
    /// frame.
    fn browsing(y: i16) -> Self {
        let data = game_data(y);
        let screen = start_screen(Rc::clone(&data)).with_dialogs(
            Rc::new(interface()),
            Rc::new(GlyphonMetrics::new(&FontFaces::bundled())),
        );
        Self {
            app: App::new(&FakeWindow, data, screen),
            gpu: RecordingGpu::new(),
            frames: 0,
        }
    }

    /// The app in flight over a planet at (0, `y`), before any frame.
    fn flying(y: i16) -> Self {
        let mut harness = Self::browsing(y);
        harness.press(Key::Char('f'));
        assert_eq!(harness.showing(), Showing::Flight);
        harness
    }

    /// The app on the ship browser over a planet at (0, `y`), playing the
    /// original's sounds through a recording port, and the port's log.
    fn sounding(y: i16) -> (Self, AudioLog) {
        let audio = RecordingAudio::new();
        let log = audio.log();
        let mut harness = Self::browsing(y);
        harness.app = harness
            .app
            .with_audio(AudioCore::new(Box::new(audio) as Box<dyn Audio>));
        (harness, log)
    }

    fn send(&mut self, event: WindowEvent) {
        let control = self.app.handle(event, &mut FakeWindow, &mut self.gpu);
        assert_eq!(control, Control::Continue, "{event:?}");
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

    /// Presses and releases the physical key `code` through winit's
    /// translation, as the real window does.
    fn press_physical(&mut self, code: KeyCode) {
        for state in [ElementState::Pressed, ElementState::Released] {
            self.send(platform::key_event(PhysicalKey::Code(code), state, false));
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

    fn ship(&self) -> ShipState {
        *self
            .app
            .screen()
            .flight_view()
            .expect("flight entered")
            .session()
            .expect("flying")
            .player()
    }

    /// Where item `item` of the spaceport dialog is.
    fn item(&self, item: usize) -> Bounds {
        self.app
            .screen()
            .spaceport_view()
            .expect("landed")
            .dialog()
            .expect("laid out")
            .item_bounds(item)
            .expect("an item")
    }

    fn frame(&mut self) -> Frame {
        self.frames += 1;
        self.send(WindowEvent::Redraw {
            elapsed: Duration::from_millis(16 * self.frames),
        });
        assert_eq!(self.app.take_failures(), []);
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

fn quads(frame: &Frame) -> Vec<Rect> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Sprites { quads, .. } => quads.iter().map(|quad| quad.dest).collect(),
            _ => Vec::new(),
        })
        .collect()
}

fn rect(bounds: Bounds) -> Rect {
    Rect {
        x: bounds.min.x,
        y: bounds.min.y,
        w: bounds.width(),
        h: bounds.height(),
    }
}

#[test]
fn l_over_the_planet_shows_its_spaceport_with_only_its_services() {
    let mut harness = Harness::flying(0);
    harness.frame();
    harness.press_physical(KeyCode::KeyL);
    assert_eq!(harness.showing(), Showing::Spaceport);
    let frame = harness.frame();

    let quads = quads(&frame);
    let landscape = harness.item(LANDSCAPE_ITEM);
    assert_eq!(
        landscape,
        Bounds::at(Point::new(206.0, 128.0), 612.0, 285.0)
    );
    assert!(quads.contains(&rect(landscape)), "{quads:?}");
    assert!(
        quads.contains(&Rect {
            x: ORIGIN.x,
            y: ORIGIN.y,
            w: 6.0,
            h: 5.0,
        }),
        "the background, unscaled: {quads:?}"
    );

    let texts = texts(&frame);
    for shown in [
        "Alpha Prime",
        DESCRIPTION,
        "Trade Center",
        "Bar",
        "Mission BBS",
        "Leave",
    ] {
        assert!(texts.iter().any(|t| t == shown), "{shown}: {texts:?}");
    }
    for hidden in ["Shipyard", "Outfitter"] {
        assert!(!texts.iter().any(|t| t == hidden), "{hidden}: {texts:?}");
    }
}

#[test]
fn the_bar_opens_over_the_spaceport_with_its_hire_escort_button() {
    let mut harness = Harness::flying(0);
    harness.press(Key::Char('l'));
    let bar = harness.item(10).center();
    harness.click(bar);
    let frame = harness.frame();
    let texts = texts(&frame);
    for shown in ["Hire Escort", "Gamble", "Holovid", "Leave", "Alpha Prime"] {
        assert!(texts.iter().any(|t| t == shown), "{shown}: {texts:?}");
    }
    assert!(!texts.iter().any(|t| t == "Not available yet"), "{texts:?}");
    harness.press(Key::Escape);
    assert_eq!(harness.showing(), Showing::Spaceport);
    assert!(
        harness
            .app
            .screen()
            .spaceport_view()
            .expect("landed")
            .open_bar()
            .is_none(),
        "Escape leaves the bar alone"
    );
}

#[test]
fn a_service_opens_its_placeholder_and_done_returns() {
    let mut harness = Harness::flying(0);
    harness.press(Key::Char('l'));
    // The Mission BBS: the Trade Center opens the exchange instead
    // (`trade.rs`), and the Bar the bar.
    let bbs = harness.item(11).center();
    harness.click(bbs);
    let frame = harness.frame();
    let texts = texts(&frame);
    assert!(texts.iter().any(|t| t == "Mission BBS"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Not available yet"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Leave"), "{texts:?}");

    harness.click(DONE_BUTTON.center());
    assert_eq!(harness.showing(), Showing::Spaceport);
    let frame = harness.frame();
    assert!(self::texts(&frame).iter().any(|t| t == "Leave"));
}

/// Where the planet is in the take-off test: off the ship's start, but
/// within the planet's landing radius of 4.
const OFF_START: i16 = -3;

#[test]
fn leave_takes_off_back_into_flight_at_the_planet() {
    let mut harness = Harness::flying(OFF_START);
    assert_eq!(harness.ship().position, Vec2::ZERO, "starts off the planet");
    harness.press(Key::Char('l'));
    assert_eq!(harness.showing(), Showing::Spaceport);
    let leave = harness.item(LEAVE_ITEM).center();
    harness.click(leave);
    assert_eq!(harness.showing(), Showing::Flight);
    assert!(harness.app.screen().spaceport_view().is_none());
    let ship = harness.ship();
    assert_eq!(
        (ship.position, ship.velocity),
        (Vec2::new(0.0, f32::from(OFF_START)), Vec2::ZERO),
        "at the planet's centre, at rest"
    );
    let session = harness
        .app
        .screen()
        .flight_view()
        .expect("flight")
        .session()
        .expect("flying");
    assert_eq!(session.landed(), None);
    let frame = harness.frame();
    assert!(
        texts(&frame)
            .iter()
            .any(|t| t == nova_view::flight::view::HELP),
        "flight is drawn"
    );
    // And it can land again.
    harness.press(Key::Char('l'));
    assert_eq!(harness.showing(), Showing::Spaceport);
    assert_eq!(
        harness
            .app
            .screen()
            .spaceport_view()
            .map(SpaceportView::stellar),
        Some(StellarId(128))
    );
}

#[test]
fn far_from_the_planet_l_says_so_and_stays_in_flight() {
    let mut harness = Harness::flying(-600);
    harness.frame();
    harness.press(Key::Char('l'));
    assert_eq!(harness.showing(), Showing::Flight);
    let frame = harness.frame();
    assert!(
        texts(&frame)
            .iter()
            .any(|t| t == "You're too far away to land on this planet."),
        "{:?}",
        texts(&frame)
    );
}

/// The commands logged since the last call.
fn drain(log: &AudioLog) -> Vec<AudioCommand> {
    std::mem::take(&mut *log.borrow_mut())
}

fn play(id: i16) -> AudioCommand {
    AudioCommand::Play {
        sound: nova_data::SoundId(id),
        volume: Volume::FULL,
    }
}

#[test]
fn landing_and_leaving_play_their_sounds_and_the_music_follows_the_screen() {
    let (mut harness, log) = Harness::sounding(0);
    harness.frame();
    assert_eq!(drain(&log), [], "the ship browser is silent");

    harness.press(Key::Char('f'));
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(
        drain(&log),
        [AudioCommand::StartMusic {
            volume: Volume::FULL
        }],
        "music in space"
    );
    harness.frame();

    // The clearance beep, then the planet's own sound; the music plays on.
    harness.press(Key::Char('l'));
    assert_eq!(harness.showing(), Showing::Spaceport);
    assert_eq!(drain(&log), [play(151), play(LANDING_SOUND)]);
    harness.frame();

    // Leave's button going down and coming up; taking off is silent.
    let leave = harness.item(LEAVE_ITEM).center();
    harness.click(leave);
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(drain(&log), [play(600), play(601)]);
    harness.frame();
    assert_eq!(drain(&log), []);

    // Back to the ship browser flight was entered from: the music stops.
    harness.press(Key::Escape);
    assert_eq!(harness.showing(), Showing::ShipBrowser);
    assert_eq!(drain(&log), [AudioCommand::StopMusic]);
}
