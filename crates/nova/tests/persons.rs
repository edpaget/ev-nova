//! Persons through the whole app: synthetic game data (a `chär` flying
//! the "Gunship", ship 128 with a blaster, from Alpha, whose one stellar,
//! a pad at the centre, can be landed on; Alpha has no traffic of its own
//! but two Person slots at 100 %: "Ace", `përs` 128, subtitle "Top Gun",
//! of the Pirates, who always attack the player (`gövt` `Flags` 0x0004),
//! saying "<OSN>: Prepare to die, <PN>!" (`STR#` 7101 #1) once, when it
//! begins to attack (`Flags` 0x0090), with no escape pod, in the "Raider";
//! and "Friend", `përs` 129, independent, with an escape pod (`Flags`
//! 0x0002), whose comm quote is "Well met." (`STR#` 7100 #1), in the
//! "Yacht") and a synthetic interface file holding the stock comm dialog,
//! driven only by window events, with pilots kept in the in-memory store.
//!
//! Tab targets Ace, and the target panel shows "Ace" and "Top Gun". Ace,
//! flying Nova's AI, attacks, and "Ace: Prepare to die, Pilot!" shows in
//! the message line. Tab to Friend: Y opens "Channel open." and G says
//! "Well met."; with `comm_quote` set to its other reading in the settings
//! file, Y alone says it. Shot down, both are gone; landing saves the
//! pilot with Ace gone for good, and the pilot reopened in a new app meets
//! Friend again on take-off, and never Ace.

// Positions here are compared after the same arithmetic on both sides.
#![allow(clippy::float_cmp)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, AppScreen, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::govt::Govt;
use nova_data::records::interface::Interface;
use nova_data::records::person::Person;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;
use nova_data::records::weapon::Weapon;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, InterfaceData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_sim::fixture::MemoryPilots;
use nova_sim::{
    Behaviour, Chance, DisableRule, Gauge, Goal, HullSpec, Npc, Pilot, PilotKeeper, PilotStore,
    Session, Surroundings,
};
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

/// The `chär`: ship 128 in system 128, no legal records.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    bytes
}

/// A `shïp` with this `Shield` and `Armor`, slow and steady, carrying
/// `weapons` (one of each), destroyed at once, named `name` when hailed.
fn ship(shield: i16, armor: i16, weapons: &[i16], name: &str) -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[shield, 300, 300, 30, 300]);
    put_i16s(&mut bytes, 0x0E, &[armor]);
    put_i16s(&mut bytes, 0x12, &[-1; 4]);
    put_i16s(&mut bytes, 0x12, weapons);
    put_i16s(&mut bytes, 0x1A, &[1; 4]);
    put_i16s(&mut bytes, 0x38, &[-1, 0]);
    put_i16s(&mut bytes, 0x42, &[1]);
    put_i16s(&mut bytes, 0x4E, &[-1; 4]);
    put_i16s(&mut bytes, 0x370, &[-1; 4]);
    put_i16s(&mut bytes, 0x6CE, &[-1; 4]);
    bytes[0x60E..0x60E + name.len()].copy_from_slice(name.as_bytes());
    bytes
}

/// An unguided `wëap` firing every 10 ticks, 20 pixels a tick for 30
/// ticks, doing 10 mass damage.
fn blaster() -> Vec<u8> {
    let mut bytes = vec![0; Weapon::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[10, 30, 10, 0, -1, 2000, -1, 0, 0, 8]);
    put_i16s(&mut bytes, 0x16, &[-1]);
    bytes
}

/// A `gövt` of `flags`, of no class, ally or enemy.
fn govt(flags: u16) -> Vec<u8> {
    let mut bytes = vec![0; Govt::SIZE.expect("fixed")];
    bytes[0x02..0x04].copy_from_slice(&flags.to_be_bytes());
    put_i16s(&mut bytes, 0x08, &[6, 0, 3, 0, 7, 0, 0, 100]);
    put_i16s(&mut bytes, 0x18, &[-1; 12]);
    bytes
}

/// A `përs` of `govt`, a warship flying `ship`, linked nowhere, with
/// these `Flags`, comm quote, hail quote and subtitle.
fn person(govt: i16, ship: i16, flags: u16, (comm, hail): (i16, i16), subtitle: &str) -> Vec<u8> {
    let mut bytes = vec![0; Person::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[-2, govt, 3, 2, 0, ship]);
    put_i16s(&mut bytes, 0x0C, &[-1; 4]);
    put_i16s(&mut bytes, 0x2A, &[-1, comm, hail, -1]);
    bytes[0x32..0x34].copy_from_slice(&flags.to_be_bytes());
    bytes[0x13A..0x13A + subtitle.len()].copy_from_slice(subtitle.as_bytes());
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

/// Alpha: independent, with the pad, no traffic of its own, and Ace and
/// Friend in its Person slots at 100 %.
fn alpha() -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[128]);
    put_i16s(&mut bytes, 0x44, &[-1; 8]);
    put_i16s(&mut bytes, 0x64, &[0, -1]);
    put_i16s(&mut bytes, 0x6E, &[128, 129, -1, -1, -1, -1, -1, -1]);
    put_i16s(&mut bytes, 0x7E, &[100, 100]);
    bytes
}

/// The pad: a `spöb` at the centre that can be landed on, drawn from
/// `spïn` 1004.
fn pad() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 4]);
    bytes[0x06..0x0A].copy_from_slice(&0x01_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x14, &[-1]);
    put_i16s(&mut bytes, 0x18, &[-1, -1]);
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

/// A `STR#` of `strings`.
fn strings(strings: &[String]) -> Vec<u8> {
    let mut bytes = u16::try_from(strings.len())
        .expect("few")
        .to_be_bytes()
        .to_vec();
    for string in strings {
        bytes.push(u8::try_from(string.len()).expect("short"));
        bytes.extend(string.as_bytes());
    }
    bytes
}

/// `STR#` 3000: each group's five variants "c<g>", but "Channel open."
/// (0) and "What is it you want?" (2).
fn comm_strings() -> Vec<String> {
    (0..40)
        .flat_map(|group| {
            let said = match group {
                0 => "Channel open.".to_owned(),
                2 => "What is it you want?".to_owned(),
                _ => format!("c{group}"),
            };
            std::iter::repeat_n(said, 5)
        })
        .collect()
}

/// The game data (see the module docs).
fn game_data() -> Rc<GameData> {
    let messages: Vec<String> = (1..=200)
        .map(|n| {
            if n == 175 {
                "Greetings.".to_owned()
            } else {
                format!("m{n}")
            }
        })
        .collect();
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(
            Ship::TYPE,
            128,
            Some(b"Gunship"),
            &ship(30, 45, &[128], "Gunship"),
        )
        .resource(
            Ship::TYPE,
            129,
            Some(b"Raider"),
            &ship(0, 10, &[], "Raider"),
        )
        .resource(Ship::TYPE, 130, Some(b"Yacht"), &ship(0, 10, &[], "Yacht"))
        .resource(Weapon::TYPE, 128, Some(b"Blaster"), &blaster())
        .resource(Govt::TYPE, 150, Some(b"Pirates"), &govt(0x0004))
        .resource(
            Person::TYPE,
            128,
            Some(b"Ace"),
            &person(150, 129, 0x0090, (-1, 1), "Top Gun"),
        )
        .resource(
            Person::TYPE,
            129,
            Some(b"Friend"),
            &person(-1, 130, 0x0002, (1, -1), ""),
        )
        .resource(System::TYPE, 128, Some(b"Alpha"), &alpha())
        .resource(Stellar::TYPE, 128, Some(b"Pad"), &pad())
        .resource(Spin::TYPE, 1004, None, &spin(1000))
        .resource(RLED, 1000, None, &sheet(1, 40))
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(ShipAnim::TYPE, 129, None, &ship_anim(2001))
        .resource(ShipAnim::TYPE, 130, None, &ship_anim(2001))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(RLED, 2001, None, &sheet(36, 6))
        .resource(Spin::TYPE, 3000, None, &spin(3000))
        .resource(RLED, 3000, None, &sheet(1, 3))
        .resource(StrList::TYPE, 3000, None, &strings(&comm_strings()))
        .resource(StrList::TYPE, 2002, None, &strings(&messages))
        .resource(
            StrList::TYPE,
            7100,
            None,
            &strings(&["Well met.".to_owned()]),
        )
        .resource(
            StrList::TYPE,
            7101,
            None,
            &strings(&["<OSN>: Prepare to die, <PN>!".to_owned()]),
        )
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &status_bar(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &pict(194, 16, [66, 66, 66]));
    for id in [8511, 5001, 5002] {
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

/// The interface file: the stock comm dialog (`DLOG` 1007, 423 x 215).
fn interface() -> InterfaceData {
    let items = [
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
    ];
    let mut dlog: Vec<u8> = [40_i16, 40, 40 + 215, 40 + 423, 1]
        .iter()
        .flat_map(|v| v.to_be_bytes())
        .collect();
    dlog.extend([1, 0, 0, 0, 0, 0, 0, 0]);
    dlog.extend(1007_i16.to_be_bytes());
    dlog.extend([0, 0, 0xA8, 0x0A]);
    let mut list = (i16::try_from(items.len()).expect("few") - 1)
        .to_be_bytes()
        .to_vec();
    for (bounds, enabled) in items {
        list.extend(user_item(bounds, enabled));
    }
    let fork = ForkBuilder::new()
        .resource(Dlog::TYPE, 1007, None, &dlog)
        .resource(Ditl::TYPE, 1007, None, &list)
        .build()
        .bytes;
    InterfaceData::load(&OneFile(fork), Path::new("/Nova-DF.rsrc")).expect("loads")
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

/// The draws listing Ace and placing it 100 above the player, then
/// listing Friend and placing it 150 above, both facing down.
const BOTH: [u32; 8] = [99, 750, 650, 180, 99, 750, 600, 180];

/// Every NPC idles.
#[derive(Debug)]
struct Still;

impl Behaviour for Still {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }
}

/// Disables no ship: a ship breaks up once its armour is gone.
#[derive(Debug)]
struct Never;

impl DisableRule for Never {
    fn disabled(&self, _armor: Gauge, _hull: &HullSpec) -> bool {
        false
    }
}

struct Game {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
}

impl Game {
    /// The app over the game data with "Pilot" saved in `store`, opened
    /// from the main menu to `showing`, its traffic drawn from `draws`,
    /// the router as `router` makes it; then two frames, the second
    /// setting the system up in flight.
    fn opened(
        store: &MemoryPilots,
        draws: &[u32],
        showing: Showing,
        router: impl FnOnce(AppScreen) -> AppScreen,
    ) -> Self {
        let data = game_data();
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        if store.text("Pilot").is_none() {
            let pilot = Pilot::new(data.as_ref(), "Pilot").expect("a pilot");
            keeper.save(&pilot).expect("saved");
        }
        let chance: Rc<RefCell<dyn Chance>> =
            Rc::new(RefCell::new(Script(draws.iter().copied().collect())));
        let screen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), Rc::new(MonoMetrics))
            .with_dialogs(Rc::new(interface()), Rc::new(MonoMetrics))
            .with_chance(SharedChance::new(chance));
        let mut game = Self {
            app: App::new(&FakeWindow, data, router(screen)),
            gpu: RecordingGpu::new(),
            frames: 0,
        };
        let at = game
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .button(MenuChoice::OpenPilot)
            .rect
            .center();
        game.click(at);
        assert_eq!(game.showing(), Showing::OpenPilot);
        game.tap(Key::Enter);
        assert_eq!(game.showing(), showing);
        game.frame();
        game.frame();
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

    /// The persons in the system, by `përs` ID.
    fn persons(&self) -> Vec<i16> {
        self.session()
            .npcs()
            .iter()
            .filter_map(|npc| Some(npc.person?.id.0))
            .collect()
    }

    /// The comm dialog's reply, as the flight gives it.
    fn reply(&self) -> String {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .hailing()
            .expect("hailing")
            .reply
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

/// Every NPC idling.
fn idle(screen: AppScreen) -> AppScreen {
    screen.with_behaviour(Rc::new(Still))
}

#[test]
fn the_target_panel_shows_a_persons_name_and_subtitle() {
    let store = MemoryPilots::new();
    let mut game = Game::opened(&store, &BOTH, Showing::Flight, idle);
    assert_eq!(game.persons(), [128, 129]);
    game.tap(Key::Tab);
    let shown = texts(&game.frame());
    assert!(shown.iter().any(|text| text == "Ace"), "{shown:?}");
    assert!(shown.iter().any(|text| text == "Top Gun"), "{shown:?}");
}

#[test]
fn a_person_beginning_to_attack_says_its_hail_quote_in_the_message_line() {
    let store = MemoryPilots::new();
    let mut game = Game::opened(&store, &BOTH, Showing::Flight, |screen| screen);
    for _ in 0..60 {
        let shown = texts(&game.frame());
        if shown
            .iter()
            .any(|text| text == "Ace: Prepare to die, Pilot!")
        {
            return;
        }
    }
    panic!("never said");
}

/// The rulebook the settings file holding `text` chooses, read as `main`
/// reads it.
fn saved_rulebook(text: &str) -> nova_sim::Rulebook {
    let home = tempfile::tempdir().expect("a temporary directory");
    let path = home.path().join("settings.json");
    std::fs::write(&path, text).expect("writes");
    let mut store: Option<Box<dyn nova_audio::SettingsStore>> =
        Some(Box::new(nova_audio::FileSettings::new(&path)));
    let (rulebook, warnings) = nova::rulebook::game_rulebook(store.as_deref_mut());
    assert_eq!(warnings, Vec::<String>::new(), "{text}");
    rulebook
}

/// The game with the rules the settings file holding `settings` chooses,
/// every NPC idling, Friend targeted and hailed: its opening line.
fn hailing_friend(settings: &str) -> (Game, String) {
    let rulebook = saved_rulebook(settings);
    let store = MemoryPilots::new();
    let mut game = Game::opened(&store, &BOTH, Showing::Flight, |screen| {
        idle(screen.with_rulebook(&rulebook))
    });
    game.tap(Key::Tab);
    game.tap(Key::Tab);
    assert_eq!(
        game.session()
            .target()
            .and_then(|npc| npc.person)
            .map(|p| p.id.0),
        Some(129)
    );
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Comm);
    let opened = game.reply();
    (game, opened)
}

#[test]
fn a_friendly_persons_greetings_says_its_comm_quote() {
    let (mut game, opened) = hailing_friend("{}");
    assert_eq!(opened, "Channel open.");
    game.tap(Key::Char('g'));
    assert_eq!(game.reply(), "Well met.");
    let shown = texts(&game.frame());
    assert!(shown.iter().any(|text| text == "Well met."), "{shown:?}");
}

#[test]
fn by_the_settings_other_reading_a_person_opens_the_hail_with_its_comm_quote() {
    let (mut game, opened) = hailing_friend(r#"{"rule_overrides": {"comm_quote": "bible"}}"#);
    assert_eq!(opened, "Well met.");
    let shown = texts(&game.frame());
    assert!(shown.iter().any(|text| text == "Well met."), "{shown:?}");
}

#[test]
fn a_unique_person_shot_down_is_gone_for_good_after_a_save_and_another_comes_back() {
    let store = MemoryPilots::new();
    let mut game = Game::opened(&store, &BOTH, Showing::Flight, |screen| {
        idle(screen).with_disable_rule(Rc::new(Never))
    });
    assert_eq!(game.persons(), [128, 129]);
    game.key(Key::Space, true);
    for _ in 0..600 {
        game.frame();
        if game.persons().is_empty() {
            break;
        }
    }
    game.key(Key::Space, false);
    assert_eq!(game.persons(), Vec::<i16>::new(), "both shot down");
    for _ in 0..60 {
        game.frame();
    }
    // L requests clearance, a second L lands.
    game.tap(Key::Char('l'));
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Spaceport);
    let saved: serde_json::Value =
        serde_json::from_str(&store.text("Pilot").expect("saved")).expect("JSON");
    assert_eq!(saved["gone_persons"], serde_json::json!([128]));
    // Reopened in a new app, the pilot takes off: Friend's slot draws, and
    // Ace's draws nothing.
    let mut game = Game::opened(&store, &[99, 750, 600, 180], Showing::Spaceport, idle);
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    game.frame();
    game.frame();
    assert_eq!(game.persons(), [129]);
    game.tap(Key::Tab);
    let shown = texts(&game.frame());
    assert!(shown.iter().any(|text| text == "Friend"), "{shown:?}");
    assert!(!shown.iter().any(|text| text == "Ace"), "{shown:?}");
}
