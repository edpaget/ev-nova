//! The app fighting over synthetic game data, wired to the renderer, the
//! recording Gpu and a recording audio port, and driven only by key events
//! and redraws, as the window sends them: the player's ship carries a
//! blaster and a rocket launcher, with three rockets; one trader is placed
//! 100 pixels above it by the app's source of chance. Tab targets the
//! trader, whose name the target panel shows; Space fires the blaster,
//! whose shots are drawn and heard; Control fires a rocket, and the
//! secondary weapon's line counts it; and firing until the trader is
//! destroyed draws its explosion, scatters debris on the effects' own
//! source of chance, and lets go of the target.

// Positions here are compared after the same arithmetic on both sides.
#![allow(clippy::float_cmp)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_audio::recording::{AudioLog, RecordingAudio};
use nova_audio::{Audio, AudioCommand, AudioCore};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::boom::Boom;
use nova_data::records::character::Character;
use nova_data::records::dude::Dude;
use nova_data::records::interface::Interface;
use nova_data::records::outfit::Outfit;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::system::System;
use nova_data::records::weapon::Weapon;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record, SoundId};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, Rect, TextRun};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_sim::{
    Behaviour, Chance, DisableRule, Gauge, Goal, HullSpec, Npc, NpcId, Session, Surroundings,
};
use nova_view::Key;
use nova_view::flight::{FlightView, SharedChance};
use nova_view::text::fixture::MonoMetrics;
use nova_view::ui::{DialogResources, DialogTemplate};

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

/// No dialogs: the router is given them only for their text metrics.
struct NoDialogs;

impl DialogResources for NoDialogs {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        Err(format!("no DLOG {id}"))
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

/// A `chär` starting in ship 128 in system 128, with no legal records.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    bytes
}

/// A `shïp` with this `Shield` and `Armor`, slow and steady, carrying
/// `weapons` (one of each) and `items` (each with its count), destroyed at
/// once in explosion type 0, `bööm` 128.
fn ship(shield: i16, armor: i16, weapons: &[i16], items: &[(i16, i16)]) -> Vec<u8> {
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
    for (slot, &(item, count)) in items.iter().enumerate() {
        put_i16s(&mut bytes, 0x4E + 2 * slot, &[item]);
        put_i16s(&mut bytes, 0x56 + 2 * slot, &[count]);
    }
    bytes
}

/// An unguided `wëap` firing every `reload` ticks, 20 pixels a tick for
/// 30 ticks, doing `damage` mass damage, its shots `spïn` 3000 +
/// `graphic`, sounding `snd ` 200 + `sound`, with `flags` and spending
/// `ammo`.
fn weapon(reload: i16, damage: i16, ammo: i16, graphic: i16, sound: i16, flags: u16) -> Vec<u8> {
    let mut bytes = vec![0; Weapon::SIZE.expect("fixed")];
    put_i16s(
        &mut bytes,
        0x00,
        &[reload, 30, damage, 0, -1, 2000, ammo, graphic, 0, sound],
    );
    put_i16s(&mut bytes, 0x16, &[-1]);
    bytes[0x1C..0x1E].copy_from_slice(&flags.to_be_bytes());
    bytes
}

/// An `oütf` that is three rounds of `wëap` 138, up to 10.
fn rockets() -> Vec<u8> {
    let mut bytes = vec![0; Outfit::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x06, &[3, 138, 10]);
    bytes
}

/// A `shän` whose base image is `rlëD` `image`, one set of 36 rotations.
fn ship_anim(image: i16) -> Vec<u8> {
    let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, 0, 1]);
    put_i16s(&mut bytes, 0x34, &[36]);
    bytes
}

/// A `spïn` naming `rlëD` `image`, in a grid `tiles` across.
fn spin(image: i16, tiles: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, -1, 0, 0, tiles, tiles]);
    bytes
}

/// An `rlëD` of `frames` frames of `size` x `size`.
fn sheet(frames: u16, size: u16) -> Vec<u8> {
    (0..frames)
        .fold(RledBuilder::new(size, size), |sheet, frame| {
            let color = 0x0400 + frame;
            sheet.frame(|f| (0..size).fold(f, |f, _| f.line().pixels(&vec![color; size.into()])))
        })
        .build()
}

/// `bööm` 128: a frame a tick of `spïn` 400, sounding `snd ` 300.
fn boom() -> Vec<u8> {
    let mut bytes = vec![0; Boom::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[100, 0, 0]);
    bytes
}

/// Stock `ïntf` 128's areas and font, over background `PICT` 700.
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
    put_i16s(&mut bytes, 0x48, &[300, 8, 315, 184]);
    put_i16s(&mut bytes, 0x50, &[330, 8, 442, 184]);
    bytes[0x60..0x66].copy_from_slice(b"Geneva");
    put_i16s(&mut bytes, 0xA0, &[12, 10, 700]);
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

/// An independent `sÿst` with no stellars, with `düde` 128 at 100 % and
/// one ship on average.
fn system() -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[-1; 16]);
    put_i16s(&mut bytes, 0x44, &[128, -1, -1, -1, -1, -1, -1, -1]);
    put_i16s(&mut bytes, 0x54, &[100]);
    put_i16s(&mut bytes, 0x64, &[1, -1]);
    bytes
}

/// A `düde` of wimpy traders flying ship 129, independent.
fn dude() -> Vec<u8> {
    let mut bytes = vec![0; Dude::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[1, -1]);
    put_i16s(&mut bytes, 0x08, &[-1; 16]);
    put_i16s(&mut bytes, 0x08, &[129]);
    put_i16s(&mut bytes, 0x28, &[100]);
    bytes
}

/// The size of the blaster's shots' frames.
const SHOT: f32 = 3.0;
/// The size of the rockets' frames.
const ROCKET: f32 = 4.0;
/// The size of the explosion's frames.
const EXPLOSION: f32 = 5.0;

/// The first `chär` flies the "Gunship" (ship 128: a 1 x 1 sheet, a
/// blaster and a rocket launcher, and three rockets) from Alpha (128),
/// whose one `düde` flies the "Trader" (ship 129: a 2 x 2 sheet, no
/// shield and 10 armour). The blaster (`wëap` 128) fires every tick, 5
/// mass damage a shot, its 3 x 3 shots sounding `snd ` 208; the rocket
/// launcher (`wëap` 138, a secondary) fires its 4 x 4 rockets every 30
/// ticks, each a round of its own. Explosion type 0 is 5 x 5 frames.
fn data() -> Rc<GameData> {
    let fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(
            Ship::TYPE,
            128,
            Some(b"Gunship"),
            &ship(30, 45, &[128, 138], &[(128, 3)]),
        )
        .resource(
            Ship::TYPE,
            129,
            Some(b"Trader;merchant"),
            &ship(0, 10, &[], &[]),
        )
        .resource(
            Weapon::TYPE,
            128,
            Some(b"Blaster"),
            &weapon(0, 5, -1, 0, 8, 0),
        )
        .resource(
            Weapon::TYPE,
            138,
            Some(b"Rocket"),
            &weapon(30, 0, 10, 1, 9, 0x0002),
        )
        .resource(Outfit::TYPE, 128, Some(b"Rockets"), &rockets())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(ShipAnim::TYPE, 129, None, &ship_anim(2001))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(RLED, 2001, None, &sheet(36, 2))
        .resource(Spin::TYPE, 3000, None, &spin(3000, 6))
        .resource(RLED, 3000, None, &sheet(36, 3))
        .resource(Spin::TYPE, 3001, None, &spin(3001, 6))
        .resource(RLED, 3001, None, &sheet(36, 4))
        .resource(Boom::TYPE, 128, None, &boom())
        .resource(Spin::TYPE, 400, None, &spin(400, 3))
        .resource(RLED, 400, None, &sheet(9, 5))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system())
        .resource(Dude::TYPE, 128, Some(b"Traders"), &dude())
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &interface(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture());
    let file = OneFile(fork.build().bytes);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

/// Draws its script, then the last outcome, so no roll fires; records
/// each `n` asked.
struct Script {
    draws: VecDeque<u32>,
    asked: Vec<u32>,
}

impl Chance for Script {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.asked.push(n);
        self.draws.pop_front().unwrap_or(n - 1)
    }
}

/// A shared source of chance drawing `draws`, and the script behind it.
fn scripted(draws: &[u32]) -> (Rc<RefCell<Script>>, SharedChance) {
    let script = Rc::new(RefCell::new(Script {
        draws: draws.iter().copied().collect(),
        asked: Vec::new(),
    }));
    let shared: Rc<RefCell<dyn Chance>> = script.clone();
    (script, SharedChance::new(shared))
}

/// Every NPC idles.
#[derive(Debug)]
struct Still;

impl Behaviour for Still {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }
}

/// Disables no ship: the trader breaks up once its armour is gone.
#[derive(Debug)]
struct Never;

impl DisableRule for Never {
    fn disabled(&self, _armor: Gauge, _hull: &HullSpec) -> bool {
        false
    }
}

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    log: AudioLog,
    effects: Rc<RefCell<Script>>,
    /// Frames sent, at 60 a second.
    frames: u64,
}

impl Harness {
    /// The app in flight, entered from the ship browser with F, its trader
    /// placed 100 pixels above the player facing down, idling, never
    /// disabled; its text measured by the monospaced metrics; its effects
    /// rolled on a script of their own; playing through a recording port.
    fn flying() -> Self {
        let data = data();
        let (_, chance) = scripted(&[6, 6, 0, 0, 750, 650, 180]);
        let (effects, effects_chance) = scripted(&[]);
        let screen = start_screen(Rc::clone(&data))
            .with_dialogs(Rc::new(NoDialogs), Rc::new(MonoMetrics))
            .with_chance(chance)
            .with_effects_chance(effects_chance)
            .with_behaviour(Rc::new(Still))
            .with_disable_rule(Rc::new(Never));
        let audio = RecordingAudio::new();
        let log = audio.log();
        let app = App::new(&FakeWindow, data, screen)
            .with_audio(AudioCore::new(Box::new(audio) as Box<dyn Audio>));
        let mut harness = Self {
            app,
            gpu: RecordingGpu::new(),
            log,
            effects,
            frames: 0,
        };
        harness.key(Key::Char('f'), true);
        harness.key(Key::Char('f'), false);
        assert_eq!(harness.app.screen().showing(), Showing::Flight);
        // The first frame runs no step; the second's sets the system up.
        harness.frame();
        harness.frame();
        assert_eq!(harness.session().npcs().len(), 1);
        harness
    }

    fn key(&mut self, key: Key, pressed: bool) {
        let event = WindowEvent::Key {
            key,
            pressed,
            repeat: false,
        };
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue
        );
    }

    fn flight(&self) -> &FlightView<Rc<GameData>> {
        self.app.screen().flight_view().expect("flight entered")
    }

    fn session(&self) -> &Session {
        self.flight().session().expect("flying")
    }

    /// Sends the next redraw and returns its frame.
    fn frame(&mut self) -> Frame {
        self.frames += 1;
        let elapsed = Duration::from_nanos(self.frames * 1_000_000_000 / 60);
        let event = WindowEvent::Redraw { elapsed };
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue
        );
        let frame = (*self.gpu.submits().last().expect("a frame")).clone();
        assert_eq!(self.app.take_failures(), []);
        frame
    }

    /// Sends redraws until `done` says so, at most a second's, and returns
    /// the last frame.
    fn until(&mut self, done: impl Fn(&Self) -> bool) -> Frame {
        for _ in 0..60 {
            let frame = self.frame();
            if done(self) {
                return frame;
            }
        }
        panic!("not within a second")
    }
}

fn runs(frame: &Frame) -> Vec<TextRun> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Text(runs) => runs.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// Where the text `text` is drawn, if it is.
fn text_at(frame: &Frame, text: &str) -> Option<(f32, f32)> {
    runs(frame)
        .into_iter()
        .find(|run| run.text == text)
        .map(|run| run.origin_px)
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

/// The status bar's target area, against the right edge at the top.
fn in_target_area((x, y): (f32, f32)) -> bool {
    (838.0..=1014.0).contains(&x) && (330.0..=442.0).contains(&y)
}

#[test]
fn tab_targets_the_trader_and_the_panel_shows_its_name() {
    let mut harness = Harness::flying();
    let before = harness.frame();
    // "No Target", 9 characters of 6, centred in the 176 from 838.
    assert_eq!(text_at(&before, "No Target"), Some((899.0, 365.0)));
    harness.key(Key::Tab, true);
    harness.key(Key::Tab, false);
    let target = harness.session().target().map(|npc| npc.id);
    assert_eq!(target, Some(NpcId(0)));
    let frame = harness.frame();
    let name = text_at(&frame, "Trader").expect("the trader's name");
    assert!(in_target_area(name), "{name:?}");
    assert_eq!(text_at(&frame, "No Target"), None);
}

#[test]
fn space_fires_the_blaster_whose_shots_are_drawn_and_heard() {
    let mut harness = Harness::flying();
    harness.key(Key::Space, true);
    let frame = harness.until(|harness| !harness.session().shots().is_empty());
    harness.key(Key::Space, false);
    assert_eq!(harness.session().shots().len(), 1);
    assert_eq!(sprites_of(&frame, SHOT).len(), 1, "the shot's frame");
    assert!(
        harness.log.borrow().contains(&AudioCommand::Play {
            sound: SoundId(208),
            volume: nova_audio::Volume::FULL,
        }),
        "{:?}",
        harness.log.borrow()
    );
}

#[test]
fn control_fires_a_rocket_and_the_secondary_line_counts_it() {
    let mut harness = Harness::flying();
    let frame = harness.frame();
    let line = text_at(&frame, "Rocket - 3").expect("three rockets");
    assert_eq!(line.1, 300.0, "in the weapon area");
    harness.key(Key::Control, true);
    let frame = harness.until(|harness| !harness.session().shots().is_empty());
    harness.key(Key::Control, false);
    assert_eq!(sprites_of(&frame, ROCKET).len(), 1, "the rocket's frame");
    assert_eq!(text_at(&frame, "Rocket - 2").map(|at| at.1), Some(300.0));
}

#[test]
fn a_trader_shot_down_explodes_scatters_debris_and_is_no_longer_targeted() {
    let mut harness = Harness::flying();
    harness.key(Key::Tab, true);
    harness.key(Key::Tab, false);
    harness.key(Key::Space, true);
    let frame = harness.until(|harness| harness.session().npcs().is_empty());
    harness.key(Key::Space, false);
    assert_eq!(harness.session().target().map(|npc| npc.id), None);
    let explosion = sprites_of(&frame, EXPLOSION);
    assert_eq!(explosion.len(), 1, "its explosion");
    assert_eq!(
        harness.effects.borrow().asked,
        [360, 101, 31].repeat(16),
        "its debris, on the effects' own chance"
    );
    assert!(text_at(&frame, "No Target").is_some());
    assert!(
        harness.log.borrow().contains(&AudioCommand::Play {
            sound: SoundId(300),
            volume: nova_audio::Volume::FULL,
        }),
        "the explosion, within 200 pixels: {:?}",
        harness.log.borrow()
    );
}
