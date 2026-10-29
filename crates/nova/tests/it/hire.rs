//! Hiring escorts in the bar through the whole app: synthetic game data (a
//! `chär` with 25,000 credits flying the "Boarder" from Alpha, whose one
//! stellar, a pad at the centre with a bar, is of tech level 7; the
//! "Hireling", ship 129, of `Cost` 10,000, tech level 1 and `HireRandom`
//! 100, with a 6 x 6 sprite; the bar's text and the Hireling's pilot
//! description) and a synthetic interface file holding the stock
//! "Spaceport", "Bar", "Shipyard", "Shipyard Info" and comm dialogs, driven
//! only by window events, with pilots kept in the in-memory store.
//!
//! - Hire: the pilot lands, the Bar opens the bar, H its hire screen, and
//!   H hires the Hireling, writing the pilot to the store with its escort.
//!   Taking off, the escort's sprite reaches the Gpu; reopened in a new
//!   app, the escort is still hired, and Option-Tab and Y open the escort
//!   dialog.
//! - Sale: with the pad a shipyard too, the escort dialog's S marks a
//!   captured Hireling for sale. Landed there, Leave is held while the
//!   report shows over the spaceport, and closing it takes off, the saved
//!   pilot holding no escort. A run's `Q` on landing there is held the same
//!   way until the report closes.
//! - The settings file's `take_off_pay` reaches the take-off's pay, and the
//!   router's hire terms and control bits reach the bar.
//!
//! The fee, the wages, the fleet's room, the screens' wording and what the
//! dialogs show are nova-sim's and nova-view's, pinned there.

// Positions here are compared after the same arithmetic on both sides.
#![allow(clippy::float_cmp)]

use crate::bracket_art;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use crate::common::{EntryHarness, Frames};
use nova::app::{App, AppScreen, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::desc::Desc;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
use nova_data::records::interface::Interface;
use nova_data::records::mission::Mission;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;
use nova_data::store::fixture::{one_file, one_file_at};
use nova_data::{GameData, InterfaceData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, Rect};
use nova_rsrc::ResType;
use nova_rsrc::fixture::ForkBuilder;
use nova_sim::fixture::MemoryPilots;
use nova_sim::{Chance, Pilot, PilotKeeper, PilotStore, RuleKey, RuleSource, Rulebook, Session};
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
    ShipBuilder::new()
        .shield(30)
        .accel(300)
        .speed(300)
        .maneuver(30)
        .fuel(300)
        .armor(45)
        .weap_types(&[-1; 8])
        .weap_counts(&[1; 4])
        .tech_level(1)
        .cost(cost)
        .explode1(-1)
        .inherent_ai(1)
        .default_items(&[-1; 8])
        .hire_random(hire_random)
        .build()
}

/// A `shän` whose base image is `rlëD` `image`, one set of 36 rotations.
fn ship_anim(image: i16) -> Vec<u8> {
    ShipAnimBuilder::new()
        .base_image_id(image)
        .base_set_count(1)
        .frames_per(36)
        .build()
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

/// A `sÿst` at the map's centre holding `stellar`; unlinked,
/// independent, with no traffic and no message buoy.
fn system(stellar: i16) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[stellar]);
    put_i16s(&mut bytes, 0x44, &[-1; 8]);
    put_i16s(&mut bytes, 0x64, &[0, -1, -1]);
    bytes
}

/// The pad's `Flags`: it can be landed on, with a bar.
const PAD_FLAGS: u32 = 0x41;
/// A shipyard's `Flags` bit.
const SHIPYARD: u32 = 0x08;

/// The pad: a `spöb` at the centre of `flags`, of tech level 7, drawn
/// from `spïn` 1004.
fn pad(flags: u32) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 4]);
    bytes[0x06..0x0A].copy_from_slice(&flags.to_be_bytes());
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
/// (128) with the pad, the bar's text (`dësc` 10000), the Hireling's pilot
/// (14001), and the pictures the screens draw.
fn game_data() -> Rc<GameData> {
    game_data_of(PAD_FLAGS)
}

/// [`game_data`] with the pad of `pad_flags`.
fn game_data_of(pad_flags: u32) -> Rc<GameData> {
    game_data_around(pad_flags, |fork| fork)
}

/// [`game_data_of`] with `more` resources added.
fn game_data_around(pad_flags: u32, more: impl FnOnce(ForkBuilder) -> ForkBuilder) -> Rc<GameData> {
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Boarder"), &ship(5000, 0))
        .resource(Ship::TYPE, 129, Some(b"Hireling"), &ship(10_000, 100))
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(ShipAnim::TYPE, 129, None, &ship_anim(2001))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(RLED, 2001, None, &sheet(36, 6))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system(128))
        .resource(Stellar::TYPE, 128, Some(b"Pad"), &pad(pad_flags))
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
    for id in [
        8500, 8501, 8503, 8506, 8511, 8513, 8524, 8525, 8526, 5001, 10_004,
    ] {
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
    let file = one_file(bracket_art::with_bracket_art(more(fork)).build().bytes);
    Rc::new(file.load())
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

/// Stock comm dialog (`DLOG` 1007, 423 x 215) and its twelve items.
fn comm_dialog() -> [(ResType, i16, Vec<u8>); 2] {
    dialog(
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
    )
}

/// Stock escort dialog (`DLOG` 1022, 424 x 259) and its twelve items:
/// the buttons (1-4), 5-9 parked below it, the info (10), the picture
/// (11) and the title (12).
fn escort_dialog() -> [(ResType, i16, Vec<u8>); 2] {
    dialog(
        1022,
        (424, 259),
        &[
            ((29, 225, 146, 26), true),
            ((29, 197, 146, 26), true),
            ((29, 141, 146, 26), true),
            ((29, 169, 146, 26), true),
            ((5, 304, 200, 25), true),
            ((213, 316, 200, 25), true),
            ((226, 309, 200, 25), true),
            ((226, 337, 200, 25), true),
            ((38, 315, 112, 16), false),
            ((14, 9, 192, 58), false),
            ((217, 30, 200, 200), false),
            ((14, 79, 192, 52), false),
        ],
    )
}

/// Stock "Desc Dialog" (`DLOG` 3003, 441 x 313), which the escorts'
/// report shows in: Done (1) and the text (3).
fn desc_dialog() -> [(ResType, i16, Vec<u8>); 2] {
    dialog(
        3003,
        (441, 313),
        &[
            ((173, 281, 99, 25), true),
            ((221, 329, 68, 30), false),
            ((11, 10, 417, 262), false),
        ],
    )
}

/// The interface file: stock "Spaceport" (`DLOG` 1000), "Bar" (1013),
/// "Shipyard" (1004), "Shipyard Info" (1005), the comm dialog (1007), the
/// escort dialog (1022) and "Desc Dialog" (3003), each with its items as
/// stock has them.
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
    let fork = [
        spaceport,
        bar,
        shipyard,
        info,
        comm_dialog(),
        escort_dialog(),
        desc_dialog(),
    ]
    .into_iter()
    .flatten()
    .fold(ForkBuilder::new(), |fork, (ty, id, bytes)| {
        fork.resource(ty, id, None, &bytes)
    })
    .build()
    .bytes;
    let path = Path::new("/Nova-DF.rsrc");
    InterfaceData::load(&one_file_at(path, fork), path).expect("loads")
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
        Self::opening_over(store, game_data(), router)
    }

    /// The app on the main menu over `data`, keeping pilots in `store`,
    /// the router as `router` makes it.
    fn opening_over(
        store: &MemoryPilots,
        data: Rc<GameData>,
        router: impl FnOnce(AppScreen) -> AppScreen,
    ) -> Self {
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Nothing));
        let screen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), Rc::new(MonoMetrics))
            .with_dialogs(Rc::new(interface()), Rc::new(MonoMetrics))
            .with_chance(SharedChance::new(chance));
        Self {
            app: App::new(&FakeWindow, data, router(crate::common::appeared(screen))),
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

    /// L requests clearance, the dock approach clears it, and a second L
    /// lands: the spaceport shows.
    fn land(&mut self) {
        self.tap(Key::Char('l'));
        self.settle();
        self.settle_until("the dock approach's clearance", |game| {
            game.session().dock_cleared()
        });
        self.tap(Key::Char('l'));
        assert_eq!(self.showing(), Showing::Spaceport);
    }

    /// Draws frames until an escort flies beside the player, then Option-
    /// Tab targets it and Y hails it: the escort dialog shows.
    fn hail_escort(&mut self) {
        self.settle_until("an escort in flight", |game| {
            game.session().npcs().iter().any(|npc| npc.escort.is_some())
        });
        self.key(Key::Alt, true);
        self.tap(Key::Tab);
        self.key(Key::Alt, false);
        self.settle();
        self.tap(Key::Char('y'));
        assert_eq!(self.showing(), Showing::EscortDialog);
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

    /// Opens the saved pilot from the main menu, which resumes it in
    /// flight over the stellar it last landed on, and, for `showing` the
    /// spaceport, lands there; then `showing` shows.
    fn open_pilot(&mut self, showing: Showing) {
        let at = self
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .press_point(MenuChoice::OpenPilot);
        self.click(at);
        assert_eq!(self.showing(), Showing::OpenPilot);
        self.tap(Key::Enter);
        self.enter_ship();
        assert_eq!(self.showing(), Showing::Flight, "resumed in flight");
        if showing == Showing::Spaceport {
            self.land();
        }
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
            .item_bounds(11)
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
        "person": null,
        "mark": null
    });
    save["escorts"] = serde_json::json!([hireling]);
    nova_sim::save::decode(&save.to_string()).expect("a pilot")
}

#[test]
fn a_hire_in_the_bar_is_saved_flies_beside_the_player_and_is_hailed_after_a_restart() {
    let store = MemoryPilots::new();
    let mut game = Game::with_pilot(&store, &ada(), Showing::Flight, |screen| screen);
    game.land();

    // The bar, and H its hire screen; Right picks a ship, and H hires it:
    // the bar is back, and the pilot saved with its escort.
    game.open_bar();
    game.tap(Key::Char('h'));
    assert!(game.hiring());
    game.tap(Key::Right);
    game.tap(Key::Char('h'));
    assert!(!game.hiring(), "back in the bar");
    let escorts = game.pilot().escorts().to_vec();
    assert!(!escorts.is_empty(), "hired");
    assert_eq!(saved(&store).escorts(), escorts);

    // Leaving and taking off, the escort's sprite reaches the Gpu.
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Spaceport);
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    game.settle();
    assert!(!sprites_of(&game.frame(), HIRELING_SIZE).is_empty());

    // A new app opens it still hired, and its hail opens the escort
    // dialog.
    let mut game = Game::opening(&store, |screen| screen);
    game.open_pilot(Showing::Flight);
    assert_eq!(game.pilot().escorts(), escorts);
    game.hail_escort();
}

/// `pilot`, holding `cash`, with a captured Hireling (no wage), full.
fn with_captive(pilot: &Pilot, cash: i64) -> Pilot {
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&hiring_already(pilot, cash, false)))
            .expect("JSON");
    save["escorts"][0]["wage"] = serde_json::Value::Null;
    nova_sim::save::decode(&save.to_string()).expect("a pilot")
}

#[test]
fn the_escort_dialogs_sale_mark_reaches_the_shipyard_whose_leave_holds_for_the_report() {
    let store = MemoryPilots::new();
    PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>)
        .save(&with_captive(&ada(), 2_000))
        .expect("saved");
    let mut game = Game::opening_over(&store, game_data_of(PAD_FLAGS | SHIPYARD), |screen| screen);
    game.open_pilot(Showing::Flight);
    game.hail_escort();
    assert!(game.app.screen().escort_dialog().is_some());
    // S marks it for sale.
    game.tap(Key::Char('s'));
    assert_eq!(
        game.pilot().escorts()[0].mark,
        Some(nova_sim::fleet::EscortMark::Sell)
    );
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);

    // Landed at the shipyard, Leave shows the report over the spaceport,
    // holding the leave; closing it takes off.
    game.land();
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Spaceport, "held");
    assert!(
        game.app
            .screen()
            .spaceport_view()
            .and_then(|spaceport| spaceport.mission_text())
            .is_some(),
        "the report over the spaceport"
    );
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight, "the leave goes on");
    assert!(game.app.screen().spaceport_view().is_none());
    assert!(saved(&store).escorts().is_empty());
}

/// A run, `mïsn` 128, to the pad and no further, its `OnSuccess` a `Q`
/// of `STR#` 25048, "Off you go."; no texts of its own.
fn leaving_run(fork: ForkBuilder) -> ForkBuilder {
    let mut run = vec![0; Mission::SIZE.expect("fixed")];
    put_i16s(&mut run, 0x00, &[128, 0, 0, 0, 0, 100, 128, -1, -1, -1]);
    run[0x1C..0x20].copy_from_slice(&1000_i32.to_be_bytes());
    put_i16s(&mut run, 0x20, &[0, 0, -1, -1, 0, -1, 0, -1, 0]);
    put_i16s(&mut run, 0x32, &[-1, -1, -1, -1, -1, -1, -1, 0, 1, -1]);
    put_i16s(&mut run, 0x58, &[-1]);
    let leave = b"Q25048\0";
    run[0x359..0x359 + leave.len()].copy_from_slice(leave);
    let mut strings = 1_i16.to_be_bytes().to_vec();
    let off = "Off you go.";
    strings.push(u8::try_from(off.len()).expect("short"));
    strings.extend(off.as_bytes());
    fork.resource(Mission::TYPE, 128, Some(b"Run"), &run)
        .resource(StrList::TYPE, 25048, None, &strings)
}

/// `pilot` holding the [`leaving_run`] in slot 0, to the pad and back, with its
/// one escort marked for sale.
fn running_with_a_sale(pilot: &Pilot) -> Pilot {
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(pilot)).expect("JSON");
    save["escorts"][0]["mark"] = serde_json::json!("sell");
    let pad = serde_json::json!({"stellar": 128, "system": 128});
    save["missions"] = serde_json::json!([{
        "slot": 0,
        "mission": 128,
        "destination": pad,
        "return": pad,
        "cargo_type": null,
        "cargo_tons": 0,
        "pay": 1000,
        "days_left": null,
        "deadline": null,
        "reached": false,
        "goals_met": false,
        "aboard": false,
        "failed": null,
        "ships": {"system": null, "left": 0, "arrived": 0, "appear": -1, "name": "", "subtitle": "", "destroyed": 0, "boarded": 0, "disabled": 0, "chased_off": 0, "uniform_type": null, "ship_done": false},
        "aux": {"countdown": 0, "arrived": 0, "left": 0},
    }]);
    nova_sim::save::decode(&save.to_string()).expect("a pilot")
}

#[test]
fn a_q_leaving_a_shipyard_shows_the_escort_report_over_the_spaceport_before_the_take_off() {
    let store = MemoryPilots::new();
    PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>)
        .save(&running_with_a_sale(&with_captive(&ada(), 2_000)))
        .expect("saved");
    let data = game_data_around(PAD_FLAGS | SHIPYARD, leaving_run);
    let mut game = Game::opening_over(&store, data, |screen| screen);
    game.open_pilot(Showing::Flight);

    // Landing at the shipyard completes the run, whose Q leaves: the
    // report shows over the spaceport, the escort already sold, and holds
    // the leave.
    game.land();
    assert!(game.session().landed().is_some(), "still landed");
    assert!(
        game.app.screen().flight_view().expect("flying").leave_due(),
        "the Q's leave waits"
    );
    assert!(
        game.app
            .screen()
            .spaceport_view()
            .and_then(|spaceport| spaceport.mission_text())
            .is_some(),
        "the report over the spaceport"
    );
    let shown = texts(&game.frame());
    assert!(
        shown.contains(&"One escort was sold for a profit of 1,000 credits.".to_owned()),
        "{shown:?}"
    );
    assert!(game.pilot().escorts().is_empty());

    // Closing it takes off.
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Flight, "the leave goes on");
    assert!(game.app.screen().spaceport_view().is_none());
    assert_eq!(game.session().landed(), None);
    assert!(saved(&store).escorts().is_empty());
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
            screen.with_rules(Rulebook::default().with_override(RuleKey::TakeOffPay, source))
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
    fn fee(&self, _ship: &nova_sim::ShipRecord, _site: &nova_sim::LandingSite, _mult: f32) -> i64 {
        7
    }

    fn charge(
        &self,
        _ship: &nova_sim::ShipRecord,
        _site: &nova_sim::LandingSite,
        _mult: f32,
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
    fn allows(&self, _test: &nova_sim::TestExpr, _pilot: &dyn nova_sim::PilotFacts) -> bool {
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
    game.tap(Key::Right);
    let shown = texts(&game.frame());
    for text in ["Hiring Price:", "7 cr"] {
        assert!(shown.contains(&text.to_owned()), "{text}: {shown:?}");
    }
    game.tap(Key::Char('h'));
    assert_eq!(game.pilot().cash(), 25_000 - 7);
    assert_eq!(game.pilot().escorts().len(), 2);
    assert_eq!(
        game.pilot().escorts().last().map(|escort| escort.wage),
        Some(Some(3)),
        "hired at the router's wage"
    );

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

crate::common::entry_harness!(Game, click, frame);
