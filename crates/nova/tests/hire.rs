//! Hiring escorts in the bar through the whole app: synthetic game data (a
//! `chär` with 25,000 credits flying the "Boarder" from Alpha, whose one
//! stellar, a pad at the centre with a bar, is of tech level 7, linked to
//! Beta; the "Hireling", ship 129, of `Cost` 10,000, tech level 1 and
//! `HireRandom` 100, with a 6 x 6 sprite; the bar's text and the
//! Hireling's pilot description) and a synthetic interface file holding
//! the stock "Spaceport", "Bar", "Shipyard", "Shipyard Info" and comm
//! dialogs, driven only by window events, with pilots kept in the
//! in-memory store.
//!
//! Hire: the pilot lands, the Bar opens the bar, and H its hire screen,
//! "Hiring Price: 1,000" and "Pay: 100 credits per day"; H hires the
//! Hireling and the bar is back. Leaving and taking off costs the fee and,
//! by `take_off_pay`'s engine reading, a day's wage, and the hired escort
//! is drawn beside the player. Landed again the pilot is saved; reopened
//! in a new app, the escort is still hired, and Option-Tab and Y show
//! "Status: Hired Escort" and "Pay: 100 credits per day".
//!
//! Defection: a pilot with 50 credits and a hired escort of wage 100
//! jumps to Beta; "Due to lack of pay, one of your escorts has defected."
//! shows, no escort is drawn there, and the saved pilot has none. With
//! `take_off_pay` set to the other reading in the settings file, a
//! take-off costs nothing. The router's hire terms set the fee and pay
//! the hire screen shows and a hire takes, and its control bits can make
//! a ship's `Availability` refuse the hire. With six escorts, the bar's
//! Hire Escort asks for nothing.

// Positions here are compared after the same arithmetic on both sides.
#![allow(clippy::float_cmp)]

use std::cell::RefCell;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, AppScreen, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::desc::Desc;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::interface::Interface;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, InterfaceData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, Rect};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};
use nova_sim::fixture::MemoryPilots;
use nova_sim::hire::DEFECTED_ONE;
use nova_sim::{Chance, Pilot, PilotKeeper, PilotStore, RuleKey, RuleSource, Session, ShipId};
use nova_view::Key;
use nova_view::flight::SharedChance;
use nova_view::geometry::Point;
use nova_view::menu::MenuChoice;
use nova_view::text::fixture::MonoMetrics;

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

/// The starting credits.
const CASH: i32 = 25_000;
/// The Hireling.
const HIRELING: ShipId = ShipId(129);
/// The size of the Hireling's sprite's frames.
const HIRELING_SIZE: f32 = 6.0;

/// The `chär`: 25,000 credits, ship 128 in system 128, no legal records.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    bytes[0x00..0x04].copy_from_slice(&CASH.to_be_bytes());
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    bytes
}

/// A `shïp` of 30 shield and 45 armour, slow and steady, unarmed, of tech
/// level 1, costing `cost`, for hire `hire_random` % of days.
fn ship(cost: i32, hire_random: i16) -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[30, 300, 300, 30, 300]);
    put_i16s(&mut bytes, 0x0E, &[45]);
    put_i16s(&mut bytes, 0x12, &[-1; 4]);
    put_i16s(&mut bytes, 0x1A, &[1; 4]);
    put_i16s(&mut bytes, 0x2E, &[1]);
    bytes[0x30..0x34].copy_from_slice(&cost.to_be_bytes());
    put_i16s(&mut bytes, 0x38, &[-1, 0]);
    put_i16s(&mut bytes, 0x42, &[1]);
    put_i16s(&mut bytes, 0x4E, &[-1; 4]);
    put_i16s(&mut bytes, 0x370, &[-1; 4]);
    put_i16s(&mut bytes, 0x38A, &[hire_random]);
    put_i16s(&mut bytes, 0x6CE, &[-1; 4]);
    bytes
}

/// A `shän` whose base image is `rlëD` `image`, one set of 36 rotations.
fn ship_anim(image: i16) -> Vec<u8> {
    let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, 0, 1]);
    put_i16s(&mut bytes, 0x34, &[36]);
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

/// A `width` x `height` picture of one colour.
fn pict(width: i16, height: i16, rgb: [u8; 3]) -> Vec<u8> {
    let frame = [0, 0, height, width];
    let pixels = vec![rgb; (width * height) as usize];
    PictBuilder::new(frame)
        .direct_bits(&DirectBits::rgb888(frame, &pixels))
        .end()
        .build()
}

/// A `sÿst` at map (`x`, 0) with these hyperlinks and `stellar` if any;
/// independent, with no traffic.
fn system(x: i16, links: &[i16], stellar: Option<i16>) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0]);
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x04, links);
    if let Some(stellar) = stellar {
        put_i16s(&mut bytes, 0x24, &[stellar]);
    }
    put_i16s(&mut bytes, 0x44, &[-1; 8]);
    put_i16s(&mut bytes, 0x64, &[0, -1]);
    bytes
}

/// The pad: a `spöb` at the centre that can be landed on, with a bar
/// (`Flags` 0x41), of tech level 7, drawn from `spïn` 1004.
fn pad() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 4]);
    bytes[0x06..0x0A].copy_from_slice(&0x41_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x0C, &[7]);
    put_i16s(&mut bytes, 0x14, &[-1]);
    put_i16s(&mut bytes, 0x18, &[-1, -1]);
    bytes
}

/// A `dësc` of `text`.
fn desc(text: &str) -> Vec<u8> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend([0xFF; 2]);
    bytes.extend([0; 34]);
    bytes
}

/// Stock `ïntf` 128's areas and font, over background `PICT` 700.
fn status_bar() -> Vec<u8> {
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
    put_i16s(&mut bytes, 0x48, &[300, 8, 315, 184]);
    put_i16s(&mut bytes, 0x50, &[330, 8, 442, 184]);
    bytes[0x60..0x66].copy_from_slice(b"Geneva");
    put_i16s(&mut bytes, 0xA0, &[12, 10, 700]);
    bytes
}

/// The game data (see the module docs): the Boarder (ship 128, a 1 x 1
/// sprite, never for hire), the Hireling (129, a 6 x 6 sprite), Alpha
/// (128) with the pad, Beta (129), the bar's text (`dësc` 10000), the
/// Hireling's pilot (14001), and the pictures the screens draw.
fn game_data() -> Rc<GameData> {
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Boarder"), &ship(5000, 0))
        .resource(Ship::TYPE, 129, Some(b"Hireling"), &ship(10_000, 100))
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(ShipAnim::TYPE, 129, None, &ship_anim(2001))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(RLED, 2001, None, &sheet(36, 6))
        .resource(
            System::TYPE,
            128,
            Some(b"Alpha"),
            &system(0, &[129], Some(128)),
        )
        .resource(System::TYPE, 129, Some(b"Beta"), &system(100, &[128], None))
        .resource(Stellar::TYPE, 128, Some(b"Pad"), &pad())
        .resource(Spin::TYPE, 1004, None, &spin(1000))
        .resource(RLED, 1000, None, &sheet(1, 40))
        .resource(Desc::TYPE, 10_000, None, &desc("Smoke and quiet talk."))
        .resource(Desc::TYPE, 14_001, None, &desc("A steady hand."))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &status_bar(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &pict(194, 16, [66, 66, 66]));
    for id in [8500, 8501, 8503, 8506, 8511, 5001, 10_004] {
        fork = fork.resource(PICT, id, None, &pict(6, 5, [40, 40, 40]));
    }
    for state in [7500, 7503, 7506] {
        fork = fork
            .resource(PICT, state, None, &pict(13, 25, [200, 0, 0]))
            .resource(PICT, state + 1, None, &pict(2, 25, [0, 200, 0]))
            .resource(PICT, state + 2, None, &pict(13, 25, [0, 0, 200]))
            .resource(PICT, state + 100, None, &pict(13, 25, [0, 0, 0]))
            .resource(PICT, state + 102, None, &pict(13, 25, [0, 0, 0]));
    }
    let file = OneFile(fork.build().bytes);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

/// One `DITL` user item at (left, top, width, height), enabled or not.
fn user_item((l, t, w, h): (i16, i16, i16, i16), enabled: bool) -> Vec<u8> {
    let mut bytes = vec![0; 4];
    for value in [t, l, t + h, l + w] {
        bytes.extend(value.to_be_bytes());
    }
    bytes.push(if enabled { 0 } else { 0x80 });
    bytes.push(0);
    bytes
}

/// A `DITL` user item: its place, (left, top, width, height), and whether
/// it is enabled.
type Item = ((i16, i16, i16, i16), bool);

/// A centred `DLOG` `id` of `width` x `height` naming `DITL` `id`, and
/// the `DITL` of `items`, each (left, top, width, height) and enabled or
/// not.
fn dialog(id: i16, (width, height): (i16, i16), items: &[Item]) -> [(ResType, i16, Vec<u8>); 2] {
    let mut dlog: Vec<u8> = [40, 40, 40 + height, 40 + width, 1]
        .iter()
        .flat_map(|v: &i16| v.to_be_bytes())
        .collect();
    dlog.extend([1, 0, 0, 0, 0, 0, 0, 0]);
    dlog.extend(id.to_be_bytes());
    dlog.extend([0, 0, 0xA8, 0x0A]);
    let mut list = (i16::try_from(items.len()).expect("few") - 1)
        .to_be_bytes()
        .to_vec();
    for &(bounds, enabled) in items {
        list.extend(user_item(bounds, enabled));
    }
    [(Dlog::TYPE, id, dlog), (Ditl::TYPE, id, list)]
}

/// The interface file: stock "Spaceport" (`DLOG` 1000), "Bar" (1013),
/// "Shipyard" (1004), "Shipyard Info" (1005) and the comm dialog (1007),
/// each with its items as stock has them.
fn interface() -> InterfaceData {
    let spaceport = dialog(
        1000,
        (618, 517),
        &[
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
        ],
    );
    let bar = dialog(
        1013,
        (263, 185),
        &[
            ((156, 154, 99, 26), false),
            ((156, 125, 99, 26), false),
            ((6, 154, 146, 26), false),
            ((54, 214, 121, 25), false),
            ((6, 125, 146, 26), false),
            ((105, 227, 121, 25), false),
            ((16, 10, 230, 106), false),
            ((61, 303, 32, 32), false),
            ((103, 303, 32, 32), false),
            ((146, 303, 32, 32), false),
        ],
    );
    let shipyard = dialog(
        1004,
        (765, 323),
        &[
            ((365, 289, 109, 25), false),
            ((248, 440, 68, 30), false),
            ((201, 380, 102, 18), false),
            ((144, 438, 69, 22), false),
            ((9, 8, 333, 271), false),
            ((354, 10, 192, 267), false),
            ((480, 289, 109, 25), false),
            ((557, 8, 200, 200), false),
            ((614, 214, 143, 100), false),
            ((253, 289, 89, 25), false),
            ((365, 431, 69, 22), false),
            ((141, 288, 25, 25), false),
            ((171, 288, 25, 25), false),
        ],
    );
    let info = dialog(
        1005,
        (250, 285),
        &[
            ((86, 253, 74, 25), false),
            ((202, 368, 68, 30), false),
            ((3, 3, 240, 24), false),
            ((27, 334, 55, 31), false),
            ((9, 32, 234, 214), false),
        ],
    );
    let comm = dialog(
        1007,
        (423, 215),
        &[
            ((21, 181, 166, 26), true),
            ((21, 153, 166, 26), true),
            ((21, 125, 166, 26), true),
            ((46, 241, 200, 25), true),
            ((7, 320, 200, 25), true),
            ((199, 335, 200, 25), true),
            ((178, 261, 200, 25), true),
            ((178, 289, 200, 25), true),
            ((34, 299, 112, 16), false),
            ((11, 8, 192, 58), false),
            ((216, 7, 200, 200), false),
            ((40, 73, 134, 46), false),
        ],
    );
    let fork = [spaceport, bar, shipyard, info, comm]
        .into_iter()
        .flatten()
        .fold(ForkBuilder::new(), |fork, (ty, id, bytes)| {
            fork.resource(ty, id, None, &bytes)
        })
        .build()
        .bytes;
    InterfaceData::load(&OneFile(fork), Path::new("/Nova-DF.rsrc")).expect("loads")
}

/// Never fires; every draw is the last outcome.
struct Nothing;

impl Chance for Nothing {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n - 1
    }
}

struct Game {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
}

impl Game {
    /// The app on the main menu, keeping pilots in `store`, the router as
    /// `router` makes it.
    fn opening(store: &MemoryPilots, router: impl FnOnce(AppScreen) -> AppScreen) -> Self {
        let data = game_data();
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Nothing));
        let screen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), Rc::new(MonoMetrics))
            .with_dialogs(Rc::new(interface()), Rc::new(MonoMetrics))
            .with_chance(SharedChance::new(chance));
        Self {
            app: App::new(&FakeWindow, data, router(screen)),
            gpu: RecordingGpu::new(),
            frames: 0,
        }
    }

    /// The app with `pilot` saved in `store` and opened from the main
    /// menu, which shows `showing`.
    fn with_pilot(
        store: &MemoryPilots,
        pilot: &Pilot,
        showing: Showing,
        router: impl FnOnce(AppScreen) -> AppScreen,
    ) -> Self {
        PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>)
            .save(pilot)
            .expect("saved");
        let mut game = Self::opening(store, router);
        game.open_pilot(showing);
        game
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue,
            "{event:?}"
        );
    }

    fn key(&mut self, key: Key, pressed: bool) {
        self.send(WindowEvent::Key {
            key,
            pressed,
            repeat: false,
        });
    }

    fn tap(&mut self, key: Key) {
        self.key(key, true);
        self.key(key, false);
    }

    fn click(&mut self, at: Point) {
        self.send(WindowEvent::PointerMoved {
            px: (f64::from(at.x), f64::from(at.y)),
        });
        for pressed in [true, false] {
            self.send(WindowEvent::PointerButton {
                button: nova_view::MouseButton::Left,
                pressed,
            });
        }
    }

    fn frame(&mut self) -> Frame {
        self.frames += 1;
        let elapsed = Duration::from_nanos(self.frames * 1_000_000_000 / 60);
        self.send(WindowEvent::Redraw { elapsed });
        assert_eq!(self.app.take_failures(), []);
        (*self.gpu.submits().last().expect("a frame")).clone()
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn session(&self) -> &Session {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .session()
            .expect("a session")
    }

    fn pilot(&self) -> Pilot {
        self.session().pilot().clone()
    }

    /// Opens the saved pilot from the main menu, which then shows
    /// `showing`.
    fn open_pilot(&mut self, showing: Showing) {
        let at = self
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .button(MenuChoice::OpenPilot)
            .rect
            .center();
        self.click(at);
        assert_eq!(self.showing(), Showing::OpenPilot);
        self.tap(Key::Enter);
        assert_eq!(self.showing(), showing);
    }

    /// Clicks the spaceport's Bar (item 10).
    fn open_bar(&mut self) {
        let bar = self
            .app
            .screen()
            .spaceport_view()
            .expect("landed")
            .dialog()
            .expect("laid out")
            .item_bounds(10)
            .expect("the Bar")
            .center();
        self.click(bar);
        let spaceport = self.app.screen().spaceport_view().expect("landed");
        let bar = spaceport.open_bar().expect("in the bar");
        assert_eq!(bar.problem(), None);
    }

    /// Whether the hire screen is open over the bar.
    fn hiring(&self) -> bool {
        self.app
            .screen()
            .spaceport_view()
            .and_then(|spaceport| spaceport.open_bar())
            .is_some_and(|bar| bar.hire_screen().is_some())
    }

    /// Sends redraws until `done` says so, at most a second's, and returns
    /// the last frame.
    fn until(&mut self, done: impl Fn(&Frame) -> bool) -> Frame {
        for _ in 0..60 {
            let frame = self.frame();
            if done(&frame) {
                return frame;
            }
        }
        panic!("not within a second")
    }

    /// Plots a course to Beta on flight's map, flies out and jumps there.
    fn jump_to_beta(&mut self) {
        self.tap(Key::Char('m'));
        let map = self
            .app
            .screen()
            .flight_view()
            .expect("flying")
            .course_map();
        let beta = map
            .model()
            .system(nova_sim::SystemId(129))
            .expect("on the map");
        let at = map.view().world_to_screen(beta.position());
        self.click(at);
        self.tap(Key::Char('m'));
        self.fly_out();
        self.tap(Key::Char('j'));
        for _ in 0..600 {
            self.frame();
            if self.session().system() == nova_sim::SystemId(129)
                && self
                    .app
                    .screen()
                    .flight_view()
                    .expect("flying")
                    .jump_effect()
                    .is_none()
            {
                return;
            }
        }
        panic!("never arrived");
    }

    /// Flies out from the centre to the minimum jump distance: turns to
    /// face away from the centre, then thrusts.
    fn fly_out(&mut self) {
        use nova_sim::flight::{heading_of, shortest_turn};
        let mut held: Option<Key> = None;
        for _ in 0..2400 {
            let ship = *self.session().player();
            if ship.position.length() >= nova_sim::hyperspace::MIN_JUMP_DISTANCE {
                if let Some(key) = held {
                    self.key(key, false);
                }
                return;
            }
            let out = if ship.position.length() > 0.0 {
                heading_of(ship.position)
            } else {
                ship.heading
            };
            let off = shortest_turn(ship.heading, out);
            let want = if off > 3.0 {
                Key::Right
            } else if off < -3.0 {
                Key::Left
            } else {
                Key::Up
            };
            if held != Some(want) {
                if let Some(key) = held {
                    self.key(key, false);
                }
                self.key(want, true);
                held = Some(want);
            }
            self.frame();
        }
        panic!("never got out: {:?}", self.session().player());
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

/// The sprites drawn `size` x `size`.
fn sprites_of(frame: &Frame, size: f32) -> Vec<Rect> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Sprites { quads, .. } => quads.clone(),
            _ => Vec::new(),
        })
        .map(|quad| quad.dest)
        .filter(|dest| dest.w == size && dest.h == size)
        .collect()
}

fn saved(store: &MemoryPilots) -> Pilot {
    nova_sim::save::decode(&store.text("Ada").expect("saved")).expect("a pilot")
}

/// A new pilot, "Ada", in flight at Alpha's centre.
fn ada() -> Pilot {
    Pilot::new(game_data().as_ref(), "Ada").expect("a pilot")
}

/// `pilot`, holding `cash`, docked at the pad when `docked`, with a hired
/// Hireling at a wage of 100, full.
fn hiring_already(pilot: &Pilot, cash: i64, docked: bool) -> Pilot {
    with_hirelings(pilot, cash, docked, 1)
}

/// `pilot`, holding `cash`, docked at the pad when `docked`, with `count`
/// hired Hirelings at a wage of 100, full.
fn with_hirelings(pilot: &Pilot, cash: i64, docked: bool, count: usize) -> Pilot {
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(pilot)).expect("JSON");
    let gauge = |max: f32| serde_json::json!({"now": max, "max": max});
    save["cash"] = serde_json::json!(cash);
    if docked {
        save["stellar"] = serde_json::json!(128);
    }
    let hireling = serde_json::json!({
        "ship": 129,
        "reserves": {"shield": gauge(30.0), "armor": gauge(45.0), "fuel": gauge(300.0)},
        "order": null,
        "carried": false,
        "wage": 100,
        "person": null
    });
    save["escorts"] = serde_json::Value::Array(vec![hireling; count]);
    nova_sim::save::decode(&save.to_string()).expect("a pilot")
}

#[test]
fn a_pilot_hires_an_escort_in_the_bar_and_it_follows_and_shows_its_pay_after_a_restart() {
    let store = MemoryPilots::new();
    let mut game = Game::with_pilot(&store, &ada(), Showing::Flight, |screen| screen);
    game.frame();
    // L requests clearance, a second L lands.
    game.tap(Key::Char('l'));
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Spaceport);

    // The bar, and H its hire screen: the Hireling's fee and pay.
    game.open_bar();
    let shown = texts(&game.frame());
    for text in ["Hire Escort", "Smoke and quiet talk.", "Leave"] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }
    game.tap(Key::Char('h'));
    assert!(game.hiring());
    let shown = texts(&game.frame());
    for text in [
        "Hiring Price: 1,000",
        "Pay: 100 credits per day",
        "You Have: 25,000",
        "A steady hand.",
    ] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }

    // H hires it: the bar is back, and the pilot saved with its escort.
    game.tap(Key::Char('h'));
    assert!(!game.hiring(), "back in the bar");
    assert_eq!(game.pilot().cash(), 24_000);
    let escorts = game.pilot().escorts().to_vec();
    assert_eq!(escorts.len(), 1);
    assert_eq!((escorts[0].ship, escorts[0].wage), (HIRELING, Some(100)));
    assert_eq!(saved(&store).escorts(), escorts);

    // Leaving, and taking off, costs a day's wage; the escort flies beside
    // the player.
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Spaceport);
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.pilot().cash(), 25_000 - 1000 - 100);
    let frame = game.until(|frame| !sprites_of(frame, HIRELING_SIZE).is_empty());
    let escort = sprites_of(&frame, HIRELING_SIZE)[0];
    let (cx, cy) = (escort.x + escort.w / 2.0, escort.y + escort.h / 2.0);
    assert!(
        (cx - 512.0).abs() < 200.0 && (cy - 384.0).abs() < 200.0,
        "beside the player at the screen's centre: {escort:?}"
    );

    // Landed again it is saved; a new app opens it still hired, and its
    // hail shows its pay. L requests clearance, a second L lands.
    game.tap(Key::Char('l'));
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(saved(&store).escorts()[0].wage, Some(100));
    let mut game = Game::opening(&store, |screen| screen);
    game.open_pilot(Showing::Spaceport);
    assert_eq!(game.pilot().escorts()[0].wage, Some(100));
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    game.frame();
    game.frame();
    game.key(Key::Alt, true);
    game.tap(Key::Tab);
    game.key(Key::Alt, false);
    assert!(
        game.session()
            .target()
            .is_some_and(|npc| npc.escort.is_some())
    );
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Comm);
    let shown = texts(&game.frame());
    for text in ["Status: Hired Escort", "Pay: 100 credits per day"] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }
}

#[test]
fn an_escort_left_unpaid_by_a_jump_defects_and_the_flight_says_so() {
    let store = MemoryPilots::new();
    let pilot = hiring_already(&ada(), 50, false);
    let mut game = Game::with_pilot(&store, &pilot, Showing::Flight, |screen| screen);
    game.frame();
    game.frame();
    assert!(!sprites_of(&game.frame(), HIRELING_SIZE).is_empty());
    game.jump_to_beta();
    assert_eq!(
        game.app.screen().flight_view().expect("flying").message(),
        Some(
            format!(
                "Jumping into the Beta system on January 2, 0. No stellar objects present.  \
                 {DEFECTED_ONE}"
            )
            .as_str()
        ),
        "after the arrival"
    );
    let frame = game.frame();
    assert!(
        texts(&frame).iter().any(|text| text.contains(DEFECTED_ONE)),
        "{:?}",
        texts(&frame)
    );
    assert_eq!(sprites_of(&frame, HIRELING_SIZE), []);
    assert_eq!(game.pilot().escorts(), []);
    assert_eq!(game.pilot().cash(), 50);
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Flight, "nowhere to land in Beta");
    assert_eq!(
        game.app
            .handle(WindowEvent::CloseRequested, &mut FakeWindow, &mut game.gpu),
        Control::Exit
    );
    assert_eq!(saved(&store).escorts(), []);
}

/// The take-off pay rule the settings file holding `text` chooses, read as
/// `main` reads it.
fn saved_take_off_pay(text: &str) -> RuleSource {
    let home = tempfile::tempdir().expect("a temporary directory");
    let path = home.path().join("settings.json");
    std::fs::write(&path, text).expect("writes");
    let mut store: Option<Box<dyn nova_audio::SettingsStore>> =
        Some(Box::new(nova_audio::FileSettings::new(&path)));
    let (rulebook, warnings) = nova::rulebook::game_rulebook(store.as_deref_mut());
    assert_eq!(warnings, Vec::<String>::new(), "{text}");
    rulebook.source_for(RuleKey::TakeOffPay)
}

#[test]
fn a_take_off_pays_a_days_wages_unless_the_settings_choose_the_other_reading() {
    for (text, cash) in [
        ("{}", 900),
        (r#"{"rule_overrides": {"take_off_pay": "bible"}}"#, 1000),
    ] {
        let source = saved_take_off_pay(text);
        let store = MemoryPilots::new();
        let pilot = hiring_already(&ada(), 1000, true);
        let mut game = Game::with_pilot(&store, &pilot, Showing::Spaceport, |screen| {
            screen.with_take_off_pay(source)
        });
        game.tap(Key::Escape);
        assert_eq!(game.showing(), Showing::Flight, "{text}");
        assert_eq!(game.pilot().cash(), cash, "{text}");
        assert_eq!(game.pilot().escorts().len(), 1, "{text}");
    }
}

/// Terms of a fee of 7 and a wage of 3 for every ship.
#[derive(Debug)]
struct Sevens;

impl nova_sim::HireTerms for Sevens {
    fn fee(&self, _ship: &nova_sim::ShipRecord, _site: &nova_sim::LandingSite) -> i64 {
        7
    }

    fn charge(
        &self,
        _ship: &nova_sim::ShipRecord,
        _site: &nova_sim::LandingSite,
        _cash: i64,
    ) -> i64 {
        7
    }

    fn wage(&self, _ship: &nova_sim::ShipRecord) -> i64 {
        3
    }
}

/// Control bits where nothing holds.
#[derive(Debug)]
struct NothingHolds;

impl nova_sim::ControlBits for NothingHolds {
    fn allows(&self, _test: &nova_sim::TestExpr) -> bool {
        false
    }
}

#[test]
fn the_routers_hire_terms_and_control_bits_decide_the_bar() {
    let store = MemoryPilots::new();
    let pilot = hiring_already(&ada(), 25_000, true);
    let mut game = Game::with_pilot(&store, &pilot, Showing::Spaceport, |screen| {
        screen.with_hire_terms(Rc::new(Sevens))
    });
    game.open_bar();
    game.tap(Key::Char('h'));
    let shown = texts(&game.frame());
    for text in ["Hiring Price: 7", "Pay: 3 credits per day"] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }
    game.tap(Key::Char('h'));
    assert_eq!(game.pilot().cash(), 25_000 - 7);

    let store = MemoryPilots::new();
    let mut game = Game::with_pilot(&store, &pilot, Showing::Spaceport, |screen| {
        screen.with_control_bits(Rc::new(NothingHolds))
    });
    game.open_bar();
    game.tap(Key::Char('h'));
    assert!(game.hiring());
    game.tap(Key::Char('h'));
    assert_eq!(
        game.pilot().cash(),
        25_000,
        "its Availability does not hold"
    );
    assert_eq!(game.pilot().escorts().len(), 1, "the one hired already");
}

#[test]
fn with_a_full_fleet_the_bars_hire_escort_asks_nothing() {
    for (count, opens) in [(5, true), (6, false)] {
        let store = MemoryPilots::new();
        let pilot = with_hirelings(&ada(), 25_000, true, count);
        let mut game = Game::with_pilot(&store, &pilot, Showing::Spaceport, |screen| screen);
        game.open_bar();
        game.tap(Key::Char('h'));
        assert_eq!(game.hiring(), opens, "{count} escorts");
    }
}
