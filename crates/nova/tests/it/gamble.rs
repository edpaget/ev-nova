//! The bar's Gamble through the whole app: synthetic game data (a `chär`
//! flying the "Boarder" from Alpha, whose one stellar, a pad at the
//! centre with a bar, `spöb` flags 0x41; the bar's text; the race's
//! result `dësc`s 32760-32763 and its Help, 32764) and a synthetic
//! interface file holding the stock "Spaceport", "Bar", "Race" (`DLOG`
//! 1023) and "Desc Dialog" dialogs, driven only by window events, with
//! pilots kept in the in-memory store. The app draws on a [`Dial`] the
//! test turns, so each draw gives the outcome the dial is set to.
//!
//! Wiring only: with no cash, G shows `STR#` 2002 #361 over the bar and
//! its close returns to it; with cash, G opens the race, a racer's click
//! bets, the result text shows with the cash less the bet, and the
//! winnings are credited once it and the movie after it are done, on the
//! tick that ends the movie. Which keys press Gamble, how the race draws and
//! tracks, the draw and the payout are nova-view's and nova-sim's,
//! pinned there.

use crate::bracket_art;

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use crate::common::{EntryHarness, Frames};
use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::desc::Desc;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
use nova_data::records::interface::Interface;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fixture::{one_file, one_file_at};
use nova_data::{GameData, InterfaceData, Record};
use nova_render::Frame;
use nova_render::recording::RecordingGpu;
use nova_rsrc::ResType;
use nova_rsrc::fixture::ForkBuilder;
use nova_sim::fixture::MemoryPilots;
use nova_sim::{Chance, Pilot, PilotKeeper, PilotStore};
use nova_view::Key;
use nova_view::flight::SharedChance;
use nova_view::geometry::Point;
use nova_view::menu::MenuChoice;
use nova_view::movie::MoviePlayer;
use nova_view::movie::fixture::{Call, MockMoviePlayer};
use nova_view::text::fixture::MonoMetrics;

/// The race's result texts, `dësc` 32760 to 32763, by winner.
const RESULTS: [&str; 4] = [
    "Racer 1 wins.",
    "Racer 2 wins.",
    "Racer 3 wins.",
    "Racer 4 wins.",
];

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

/// The `chär`: `cash` credits, ship 128 in system 128, no legal records.
fn character(cash: i32) -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    bytes[0x00..0x04].copy_from_slice(&cash.to_be_bytes());
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    bytes
}

/// A `shïp` of 30 shield and 45 armour, slow and steady, unarmed.
fn ship() -> Vec<u8> {
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
        .cost(5000)
        .explode1(-1)
        .inherent_ai(1)
        .default_items(&[-1; 8])
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

/// Alpha, a `sÿst` at the map's centre with the pad, independent, with no
/// traffic.
fn system() -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[128]);
    put_i16s(&mut bytes, 0x44, &[-1; 8]);
    put_i16s(&mut bytes, 0x64, &[0, -1]);
    bytes
}

/// The pad: a `spöb` at the centre that can be landed on, with a bar
/// (`Flags` 0x41), drawn from `spïn` 1004, held by `gövt` `govt` (-1 for
/// independent).
fn pad(govt: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 4]);
    bytes[0x06..0x0A].copy_from_slice(&0x41_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x0C, &[7]);
    put_i16s(&mut bytes, 0x14, &[govt]);
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

/// A `dësc` of `text` naming movie `file`, played after it (`Flags`
/// 0x0001).
fn desc_with_after_movie(text: &str, file: &str) -> Vec<u8> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend([0xFF; 2]);
    let mut name = file.as_bytes().to_vec();
    name.resize(32, 0);
    bytes.extend(name);
    bytes.extend(1_u16.to_be_bytes());
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

/// The game data (see the module docs), the pilot starting with `cash`.
fn game_data(cash: i32) -> Rc<GameData> {
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character(cash))
        .resource(Ship::TYPE, 128, Some(b"Boarder"), &ship())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system())
        .resource(Stellar::TYPE, 128, Some(b"Pad"), &pad(-1))
        .resource(Spin::TYPE, 1004, None, &spin(1000))
        .resource(RLED, 1000, None, &sheet(1, 40))
        .resource(Desc::TYPE, 10_000, None, &desc("Smoke and quiet talk."))
        .resource(Desc::TYPE, 32_764, None, &desc("Pick a racer."))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &status_bar(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &pict(194, 16, [66, 66, 66]));
    // Racer 1's result plays its movie after its text, so the winnings
    // are credited only once that movie is over.
    fork = fork.resource(
        Desc::TYPE,
        32_760,
        None,
        &desc_with_after_movie(RESULTS[0], "Race 1.mov"),
    );
    for (id, text) in (32_761..).zip(&RESULTS[1..]) {
        fork = fork.resource(Desc::TYPE, id, None, &desc(text));
    }
    // The race's background and its racers' pictures.
    let racers = [8530, 8540, 8550, 8560]
        .into_iter()
        .flat_map(|first| first..first + 4);
    for id in std::iter::once(8529).chain(racers) {
        fork = fork.resource(PICT, id, None, &pict(4, 4, [90, 90, 90]));
    }
    for id in [8500, 8501, 8503, 10_004] {
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
    Rc::new(one_file(bracket_art::with_bracket_art(fork).build().bytes).load())
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
/// "Race" (1023) and "Desc Dialog" (3003), each with its items as stock
/// has them.
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
    let race = dialog(
        1023,
        (470, 230),
        &[
            ((243, 196, 99, 25), false),
            ((129, 196, 99, 25), false),
            ((13, 88, 100, 100), false),
            ((128, 88, 100, 100), false),
            ((243, 88, 100, 100), false),
            ((357, 88, 100, 100), false),
        ],
    );
    let text = dialog(
        3003,
        (441, 313),
        &[
            ((173, 281, 99, 25), true),
            ((221, 329, 68, 30), false),
            ((11, 10, 417, 262), false),
        ],
    );
    let fork = [spaceport, bar, race, text]
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

/// Never fires; every draw is the outcome the dial is set to, or the
/// last when there are fewer.
struct Dial(Rc<Cell<u32>>);

impl Chance for Dial {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0.get().min(n - 1)
    }
}

struct Game {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
    /// The outcome the app's draws give.
    dial: Rc<Cell<u32>>,
    /// The movie player, whose movies end on their third advance.
    player: Rc<MockMoviePlayer>,
}

impl Game {
    /// The app with a new pilot holding `cash` saved in a fresh store and
    /// opened from the main menu, in flight at Alpha's centre.
    fn flying_with(cash: i32) -> Self {
        let data = game_data(cash);
        let store = MemoryPilots::new();
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        keeper
            .save(&Pilot::new(data.as_ref(), "Ada").expect("a pilot"))
            .expect("saved");
        let dial = Rc::new(Cell::new(0));
        let player = MockMoviePlayer::playing((160, 120), 3);
        let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Dial(Rc::clone(&dial))));
        let screen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), Rc::new(MonoMetrics))
            .with_dialogs(Rc::new(interface()), Rc::new(MonoMetrics))
            .with_chance(SharedChance::new(chance))
            .with_movie_player(Rc::clone(&player) as Rc<dyn MoviePlayer>);
        let mut game = Self {
            app: App::new(&FakeWindow, data, crate::common::appeared(screen)),
            gpu: RecordingGpu::new(),
            frames: 0,
            dial,
            player,
        };
        let at = game
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .press_point(MenuChoice::OpenPilot);
        game.click(at);
        assert_eq!(game.showing(), Showing::OpenPilot);
        game.tap(Key::Enter);
        game.enter_ship();
        assert_eq!(game.showing(), Showing::Flight);
        game
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue,
            "{event:?}"
        );
    }

    fn tap(&mut self, key: Key) {
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

    /// Lands at the pad: L requests clearance, the dock approach clears
    /// it, and a second L lands.
    fn land(&mut self) {
        self.tap(Key::Char('l'));
        self.settle();
        self.settle_until("the dock approach's clearance", |game| {
            game.app
                .screen()
                .flight_view()
                .and_then(|flight| flight.session().ok())
                .is_some_and(nova_sim::Session::dock_cleared)
        });
        self.tap(Key::Char('l'));
        assert_eq!(self.showing(), Showing::Spaceport);
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
        let bar = self
            .app
            .screen()
            .spaceport_view()
            .and_then(|spaceport| spaceport.open_bar())
            .expect("in the bar");
        assert_eq!(bar.problem(), None);
    }

    /// The bar open.
    fn bar(&self) -> Option<&nova_view::spaceport::bar::BarScreen> {
        self.app
            .screen()
            .spaceport_view()
            .and_then(|spaceport| spaceport.open_bar())
    }

    /// The text shown over the spaceport, its lines joined.
    fn text_shown(&self) -> Option<String> {
        self.app
            .screen()
            .spaceport_view()
            .and_then(|spaceport| spaceport.mission_text())
            .map(|text| text.lines().join(" "))
    }

    /// The pilot's cash, as the flight has it.
    fn cash(&self) -> i64 {
        self.app
            .screen()
            .flight_view()
            .and_then(|flight| flight.session().ok())
            .map(|session| session.pilot().cash)
            .expect("a session")
    }
}

#[test]
fn with_no_cash_gamble_shows_361_over_the_bar_and_its_close_returns_to_the_bar() {
    let mut game = Game::flying_with(0);
    game.land();
    game.open_bar();
    game.tap(Key::Char('g'));
    assert_eq!(
        game.text_shown().as_deref(),
        Some(nova_sim::gamble::NO_CASH_TO_BET)
    );
    assert!(game.bar().is_some_and(|bar| bar.race_screen().is_none()));
    game.tap(Key::Enter);
    assert_eq!(game.text_shown(), None);
    assert!(game.bar().is_some(), "back in the bar");
    assert_eq!(game.showing(), Showing::Spaceport);
}

#[test]
fn with_cash_gamble_opens_the_race_and_a_bet_pays_once_its_text_closes() {
    let mut game = Game::flying_with(25_000);
    game.land();
    game.open_bar();
    game.dial.set(0);
    game.tap(Key::Char('g'));
    let racer = game
        .bar()
        .and_then(|bar| bar.race_screen())
        .and_then(|race| race.dialog())
        .and_then(|dialog| dialog.item_bounds(nova_view::spaceport::race::FIRST_RACER_ITEM))
        .expect("the race over the bar")
        .center();
    game.click(racer);
    let shown = game.text_shown().expect("the result");
    assert!(shown.starts_with(RESULTS[0]), "{shown}");
    assert!(shown.contains("Your winnings:  4,000 credits."), "{shown}");
    assert_eq!(game.cash(), 24_000, "the bet taken while the text is up");
    // Return pressed and held: its release would skip the movie.
    game.send(WindowEvent::Key {
        key: Key::Enter,
        pressed: true,
        repeat: false,
    });
    assert_eq!(game.text_shown(), None, "closed");
    assert!(
        game.player
            .calls()
            .contains(&Call::Start("Race 1.mov".to_owned())),
        "its after-movie playing"
    );
    assert_eq!(game.cash(), 24_000, "not while its after-movie plays");
    for _ in 0..10 {
        game.frame();
    }
    assert!(game.player.calls().ends_with(&[Call::Stop]), "it ended");
    assert_eq!(
        game.cash(),
        28_000,
        "4 x 1,000 credited once the movie is over"
    );
    game.tap(Key::Escape);
    assert!(
        game.bar().is_some_and(|bar| bar.race_screen().is_none()),
        "back in the bar"
    );
}

crate::common::entry_harness!(Game, click, frame);
