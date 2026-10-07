//! Persons joining the player and boarding grants through the whole app:
//! synthetic game data (a `chär` flying the "Gunship", ship 128, from
//! Alpha, whose one stellar, a pad at the centre, can be landed on; Alpha
//! has no traffic of its own but two Person slots at 100 %: "Merchant",
//! `përs` 128, independent, of `Flags` 0x0040, in the "Hauler", whose
//! `LinkMission` 128 has one special ship escorting the player (`mïsn`
//! `ShipCount` 1, `ShipGoal` 3); and "Wreck", `përs` 129, of a derelict
//! government (`gövt` `Flags` 0x0800), so disabled from the start, in the
//! "Hulk" (a crew of 3), granting up to 2 outfits of class 7 every time;
//! and `oütf` 128, "Spare Part", of `ItemClass` 7, massless) and a
//! synthetic interface file holding the stock comm dialog, driven only by
//! window events, with pilots kept in the in-memory store.
//!
//! By the engine's default, hailing Merchant lists Greetings and Request
//! Assistance only. With `person_join` set to its other reading in the
//! settings file, it lists Use As Escort too, and U makes it say "Okay,
//! I'm on my way." and join the fleet; landing saves the pilot with the
//! escort's `"person": 128`, and the pilot reopened in a new app takes
//! off with Merchant beside it, shown by its name. Boarding Wreck says
//! "You retrieved two spare parts from this ship." in the message line,
//! opens the plunder dialog and adds the two to the pilot's outfits; with
//! a draw that yields one, it says "a spare part".

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::govt::Govt;
use nova_data::records::interface::Interface;
use nova_data::records::mission::Mission;
use nova_data::records::outfit::Outfit;
use nova_data::records::person::Person;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, InterfaceData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_sim::fixture::MemoryPilots;
use nova_sim::{
    Behaviour, Chance, Goal, Npc, OutfitId, Pilot, PilotKeeper, PilotStore, Session, Surroundings,
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

/// An unarmed `shïp` of 30 shield and 45 armour, a trader escort
/// (`InherentAI` 1) with a crew of `crew`, named `name` when hailed.
fn ship(crew: i16, name: &str) -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[30, 300, 300, 30, 300]);
    put_i16s(&mut bytes, 0x0E, &[45]);
    put_i16s(&mut bytes, 0x12, &[-1; 4]);
    put_i16s(&mut bytes, 0x1A, &[1; 4]);
    put_i16s(&mut bytes, 0x38, &[-1, 0]);
    put_i16s(&mut bytes, 0x42, &[1, crew]);
    put_i16s(&mut bytes, 0x4E, &[-1; 4]);
    put_i16s(&mut bytes, 0x370, &[-1; 4]);
    put_i16s(&mut bytes, 0x6CE, &[-1; 4]);
    bytes[0x60E..0x60E + name.len()].copy_from_slice(name.as_bytes());
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

/// A `përs` of `govt`, a warship flying `ship`, linked nowhere, of no
/// quotes, with these `Flags`, `LinkMission` and grant (`GrantClass`,
/// `GrantCount`, `GrantProb`).
fn person(govt: i16, ship: i16, flags: u16, mission: i16, grant: [i16; 3]) -> Vec<u8> {
    let mut bytes = vec![0; Person::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[-2, govt, 3, 2, 0, ship]);
    put_i16s(&mut bytes, 0x0C, &[-1; 4]);
    put_i16s(&mut bytes, 0x2A, &[-1, -1, -1, mission]);
    bytes[0x32..0x34].copy_from_slice(&flags.to_be_bytes());
    put_i16s(&mut bytes, 0x134, &grant);
    bytes
}

/// A `mïsn` with one special ship escorting the player.
fn escort_mission() -> Vec<u8> {
    let mut bytes = vec![0; Mission::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x20, &[1]);
    put_i16s(&mut bytes, 0x26, &[3]);
    bytes
}

/// An `oütf` of `ItemClass` 7, massless, up to 10 owned, "spare part" and
/// "spare parts".
fn spare_part() -> Vec<u8> {
    let mut bytes = vec![0; Outfit::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 1]);
    put_i16s(&mut bytes, 0x0A, &[10]);
    bytes[0x36B..0x36B + 10].copy_from_slice(b"spare part");
    bytes[0x3AB..0x3AB + 11].copy_from_slice(b"spare parts");
    put_i16s(&mut bytes, 0x3EC, &[7]);
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

/// Alpha: independent, with the pad, no traffic of its own, and Merchant
/// and Wreck in its Person slots at 100 %.
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
/// (0) and "Okay, I'm on my way." (29).
fn comm_strings() -> Vec<String> {
    (0..40)
        .flat_map(|group| {
            let said = match group {
                0 => "Channel open.".to_owned(),
                29 => "Okay, I'm on my way.".to_owned(),
                _ => format!("c{group}"),
            };
            std::iter::repeat_n(said, 5)
        })
        .collect()
}

/// The game data (see the module docs).
fn game_data() -> Rc<GameData> {
    let messages: Vec<String> = (1..=200).map(|n| format!("m{n}")).collect();
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Gunship"), &ship(10, "Gunship"))
        .resource(Ship::TYPE, 129, Some(b"Hauler"), &ship(3, "Hauler"))
        .resource(Ship::TYPE, 130, Some(b"Hulk"), &ship(3, "Hulk"))
        .resource(Govt::TYPE, 150, Some(b"Derelicts"), &govt(0x0800))
        .resource(
            Person::TYPE,
            128,
            Some(b"Merchant"),
            &person(-1, 129, 0x0040, 128, [0, 0, 0]),
        )
        .resource(
            Person::TYPE,
            129,
            Some(b"Wreck"),
            &person(150, 130, 0, -1, [7, 2, 100]),
        )
        .resource(Mission::TYPE, 128, Some(b"Escort"), &escort_mission())
        .resource(Outfit::TYPE, 128, Some(b"Spare Part"), &spare_part())
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

/// The draws listing Merchant and placing it 300 above the player,
/// facing up, then listing Wreck and placing it on the player, facing
/// up.
const BOTH: [u32; 8] = [99, 750, 450, 0, 99, 750, 750, 0];

/// Every NPC idles.
#[derive(Debug)]
struct Still;

impl Behaviour for Still {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
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
    /// every NPC idling, the rules as the settings file holding
    /// `settings` chooses; then two frames, the second setting the system
    /// up in flight.
    fn opened(store: &MemoryPilots, draws: &[u32], showing: Showing, settings: &str) -> Self {
        let data = game_data();
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        if store.text("Pilot").is_none() {
            let pilot = Pilot::new(data.as_ref(), "Pilot").expect("a pilot");
            keeper.save(&pilot).expect("saved");
        }
        let chance: Rc<RefCell<dyn Chance>> =
            Rc::new(RefCell::new(Script(draws.iter().copied().collect())));
        let rulebook = saved_rulebook(settings);
        let screen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), Rc::new(MonoMetrics))
            .with_dialogs(Rc::new(interface()), Rc::new(MonoMetrics))
            .with_chance(SharedChance::new(chance))
            .with_rulebook(&rulebook)
            .with_behaviour(Rc::new(Still));
        let mut game = Self {
            app: App::new(&FakeWindow, data, screen),
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

    /// The person NPC `id` targeted: Tab until it is.
    fn target_person(&mut self, id: i16) {
        for _ in 0..4 {
            self.tap(Key::Tab);
            let targeted = self
                .session()
                .target()
                .and_then(|npc| npc.person)
                .map(|person| person.id.0);
            if targeted == Some(id) {
                return;
            }
        }
        panic!("never targeted {id}");
    }

    /// The comm dialog's options and reply, as the flight gives them.
    fn hailing(&self) -> (Vec<String>, String) {
        let view = self
            .app
            .screen()
            .flight_view()
            .expect("flying")
            .hailing()
            .expect("hailing");
        let labels = view
            .options
            .into_iter()
            .map(|button| button.label)
            .collect();
        (labels, view.reply)
    }

    /// The message line, as the flight gives it.
    fn message(&self) -> Option<String> {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .message()
            .map(str::to_owned)
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

/// The settings choosing `person_join`'s other reading.
const JOINING: &str = r#"{"rule_overrides": {"person_join": "bible"}}"#;

#[test]
fn by_the_engines_default_merchant_offers_only_greetings_and_assistance() {
    let store = MemoryPilots::new();
    let mut game = Game::opened(&store, &BOTH, Showing::Flight, "{}");
    game.target_person(128);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Comm);
    let (options, reply) = game.hailing();
    assert_eq!(reply, "Channel open.");
    assert_eq!(options, ["Greetings", "Request Assistance"]);
}

#[test]
fn by_the_settings_other_reading_merchant_joins_and_flies_again_after_a_save() {
    let store = MemoryPilots::new();
    let mut game = Game::opened(&store, &BOTH, Showing::Flight, JOINING);
    game.target_person(128);
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Comm);
    let (options, _) = game.hailing();
    assert_eq!(
        options,
        ["Greetings", "Request Assistance", "Use As Escort"]
    );
    let shown = texts(&game.frame());
    assert!(
        shown.iter().any(|text| text == "Use As Escort"),
        "{shown:?}"
    );
    game.tap(Key::Char('u'));
    let (options, reply) = game.hailing();
    assert_eq!(reply, "Okay, I'm on my way.");
    assert_eq!(options, ["Release"]);
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.session().pilot().escorts().len(), 1);
    game.frame();
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Spaceport);
    let saved: serde_json::Value =
        serde_json::from_str(&store.text("Pilot").expect("saved")).expect("JSON");
    assert_eq!(saved["escorts"][0]["person"], 128);
    // Reopened in a new app, the pilot takes off: Merchant's slot is
    // passed over, Wreck's draws, and Merchant flies beside the player.
    let mut game = Game::opened(&store, &[99, 750, 450, 0], Showing::Spaceport, JOINING);
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    game.frame();
    game.frame();
    let escort = game
        .session()
        .npcs()
        .iter()
        .find(|npc| npc.escort.is_some())
        .expect("placed");
    assert_eq!(escort.person.map(|person| person.id.0), Some(128));
    assert_eq!(game.session().npc_name(escort), Some("Merchant"));
    game.key(Key::Alt, true);
    game.tap(Key::Tab);
    game.key(Key::Alt, false);
    let shown = texts(&game.frame());
    assert!(shown.iter().any(|text| text == "Merchant"), "{shown:?}");
}

/// Boards Wreck, the draws after the placement `draws`: the message line
/// and how many spare parts the pilot owns.
fn boarding_wreck(draws: &[u32]) -> (Option<String>, u16) {
    let store = MemoryPilots::new();
    let script: Vec<u32> = BOTH.iter().chain(draws).copied().collect();
    let mut game = Game::opened(&store, &script, Showing::Flight, "{}");
    game.target_person(129);
    game.tap(Key::Char('b'));
    assert_eq!(game.showing(), Showing::Plunder);
    let owned = game.session().pilot().owned(OutfitId(128));
    (game.message(), owned)
}

#[test]
fn boarding_wreck_says_what_it_retrieved_and_adds_it() {
    assert_eq!(
        boarding_wreck(&[]),
        (
            Some("You retrieved two spare parts from this ship.".to_owned()),
            2
        )
    );
    // The threshold, the energy, the jitter, the odds (1 of 100), then a
    // count of trunc(50 x 2 / 100) = 1.
    assert_eq!(
        boarding_wreck(&[0, 0, 0, 0, 0]),
        (
            Some("You retrieved a spare part from this ship.".to_owned()),
            1
        )
    );
}
