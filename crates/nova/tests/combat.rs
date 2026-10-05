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
//!
//! Armed otherwise: a front-quadrant chaingun whose looped sound lasts 14
//! ticks is heard once a burst, not once a shot, and fires at the
//! lead angle at a trader targeted 30 degrees off the nose; and the
//! player's point defence shoots down a trader's missile, as the router's
//! point-defence rule says.
//!
//! Patrolled, by Nova's AI and law: the player firing on a wimpy trader
//! puts it to flight and brings the police, allied with the traders, down
//! on the player, whose shield drops; once the trader is disabled the
//! player's record with its government is lower, and R targets the
//! police, a threat, in red brackets, not the nearer disabled trader.
//! The law follows the rulebook the settings file at the platform's
//! settings path chooses: by the engine's crime gains, the default,
//! disabling the trader pleases a neutral government; by the Bible's, it
//! does not.
//!
//! Boarded, with a pilot opened from the main menu and saved in a memory
//! store, and the interface file's plunder and assignment dialogs: the
//! blaster disables a trader carrying food and money, Tab targets it, the
//! arrow keys bring the player over it at its speed and heading, and B
//! boards it. The plunder dialog offers its credits and cargo, not ammo,
//! at capture odds of 75 %; Credits takes the credits, and Capture Ship
//! opens the assignment dialog: "Use As Escort" adds it to the fleet,
//! which the saved pilot keeps and a new app opens again, and "Use As My
//! Ship" flies it, keeping the old ship as an escort.

// Positions here are compared after the same arithmetic on both sides.
#![allow(clippy::float_cmp)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, AppScreen, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_audio::recording::{AudioLog, RecordingAudio};
use nova_audio::{Audio, AudioCommand, AudioCore};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::boom::Boom;
use nova_data::records::character::Character;
use nova_data::records::dude::Dude;
use nova_data::records::govt::Govt;
use nova_data::records::interface::Interface;
use nova_data::records::outfit::Outfit;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::system::System;
use nova_data::records::weapon::Weapon;
use nova_data::sound::fixture::{Header, SndBuilder, SndFormat};
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record, SoundId};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, Rect, TextRun};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};
use nova_sim::combat::defence::Side;
use nova_sim::{
    Behaviour, Chance, DisableRule, Gauge, Goal, HullSpec, Npc, NpcId, PointDefenceRule, Session,
    Surroundings,
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

/// `bytes`, a `wëap`, of `guidance`, living `count` ticks at `speed`
/// pixels a tick x100, in bursts of `burst` then `burst_reload` ticks.
fn guided(mut bytes: Vec<u8>, guidance: i16, count: i16, speed: i16, burst: (i16, i16)) -> Vec<u8> {
    put_i16s(&mut bytes, 0x02, &[count]);
    put_i16s(&mut bytes, 0x08, &[guidance]);
    put_i16s(&mut bytes, 0x0A, &[speed]);
    put_i16s(&mut bytes, 0x5A, &[burst.0, burst.1]);
    bytes
}

/// A mono `snd ` of `frames` frames at 22,050 Hz.
fn snd(frames: usize) -> Vec<u8> {
    SndBuilder::new(
        SndFormat::Two,
        Header::Standard {
            rate: 22_050 << 16,
            loop_points: (0, 0),
            base_note: 60,
            samples: vec![0x80; frames],
        },
    )
    .bytes()
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

/// The `snd ` resource type.
const SND: ResType = ResType::new(*b"snd ");

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
    data_arming(&[128, 138], &[])
}

/// [`data`] with the Gunship carrying `player` and the Trader `trader`,
/// and these weapons besides: the Quad (`wëap` 133), point defence firing
/// every 5 ticks, 20 pixels a tick for 12; the IR Missile (134), homing
/// at 5 pixels a tick for 200 ticks, fired once; and the Chaingun (155), a
/// front-quadrant turret firing every tick in bursts of 10 then 15 ticks
/// (sound looped, `snd ` 205 lasting 14 ticks), its 3 x 3 shots 18 pixels
/// a tick for 20.
fn data_arming(player: &[i16], trader: &[i16]) -> Rc<GameData> {
    let fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(
            Ship::TYPE,
            128,
            Some(b"Gunship"),
            &ship(30, 45, player, &[(128, 3)]),
        )
        .resource(
            Ship::TYPE,
            129,
            Some(b"Trader;merchant"),
            &ship(0, 10, trader, &[]),
        )
        .resource(
            Weapon::TYPE,
            133,
            Some(b"Quad"),
            &guided(weapon(5, 1, -1, 0, -1, 0), 9, 12, 2000, (0, 0)),
        )
        .resource(
            Weapon::TYPE,
            134,
            Some(b"IR Missile"),
            &guided(weapon(1000, 20, -1, 0, -1, 0), 1, 200, 500, (0, 0)),
        )
        .resource(
            Weapon::TYPE,
            155,
            Some(b"Chaingun"),
            &guided(weapon(0, 4, -1, 0, 5, 0x0010), 7, 20, 1800, (10, 15)),
        )
        .resource(SND, 205, Some(b"Chaingun"), &snd(10_290))
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
        Self::flying_over(data(), &[6, 6, 0, 0, 750, 650, 180], |screen| {
            screen.with_behaviour(Rc::new(Still))
        })
    }

    /// [`Harness::flying`] over `data`, its trader placed by `placing`,
    /// the router as `router` makes it.
    fn flying_over(
        data: Rc<GameData>,
        placing: &[u32],
        router: impl FnOnce(AppScreen) -> AppScreen,
    ) -> Self {
        let (_, chance) = scripted(placing);
        let (effects, effects_chance) = scripted(&[]);
        let screen = start_screen(Rc::clone(&data))
            .with_dialogs(Rc::new(NoDialogs), Rc::new(MonoMetrics))
            .with_chance(chance)
            .with_effects_chance(effects_chance)
            .with_disable_rule(Rc::new(Never));
        let screen = router(screen);
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
        assert!(!harness.session().npcs().is_empty());
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

/// How many times `sound` was played, at any volume.
fn plays(log: &AudioLog, sound: i16) -> usize {
    log.borrow()
        .iter()
        .filter(|command| {
            matches!(command, AudioCommand::Play { sound: played, .. } if *played == SoundId(sound))
        })
        .count()
}

/// The Chaingun's shots seen in flight, by number.
fn chaingun_shots(harness: &Harness) -> Vec<u32> {
    harness
        .session()
        .shots()
        .iter()
        .filter(|shot| shot.weapon.id == nova_sim::WeaponId(155))
        .map(|shot| shot.id.0)
        .collect()
}

#[test]
fn a_chaingun_held_on_is_heard_once_a_burst_not_once_a_shot() {
    let mut harness = Harness::flying_over(
        data_arming(&[155], &[]),
        &[6, 6, 0, 0, 750, 650, 180],
        |screen| screen.with_behaviour(Rc::new(Still)),
    );
    let mut fired = std::collections::BTreeMap::new();
    harness.key(Key::Space, true);
    // 60 ticks, at two frames a tick.
    for frame in 0..120 {
        harness.frame();
        for id in chaingun_shots(&harness) {
            fired.entry(id).or_insert(frame / 2);
        }
    }
    harness.key(Key::Space, false);
    let ticks: Vec<u64> = fired.values().copied().collect();
    let first = ticks[0];
    let bursts: Vec<u64> = (0..10)
        .chain(24..34)
        .chain(48..58)
        .map(|tick| first + tick)
        .collect();
    assert_eq!(ticks, bursts, "three bursts of ten");
    assert_eq!(plays(&harness.log, 205), 3, "once a burst");
}

#[test]
fn a_front_quadrant_turret_fires_at_a_trader_targeted_30_degrees_off_the_nose() {
    // The trader at (50, -87) from the player, who faces up.
    let mut harness = Harness::flying_over(
        data_arming(&[155], &[]),
        &[6, 6, 0, 0, 800, 663, 180],
        |screen| screen.with_behaviour(Rc::new(Still)),
    );
    harness.key(Key::Tab, true);
    harness.key(Key::Tab, false);
    assert_eq!(harness.session().target().map(|npc| npc.id), Some(NpcId(0)));
    harness.key(Key::Space, true);
    harness.until(|harness| !chaingun_shots(harness).is_empty());
    harness.key(Key::Space, false);
    let bearing = 50.0_f32.atan2(87.0).to_degrees();
    let shot = harness.session().shots()[0];
    assert!((shot.heading - bearing).abs() < 1e-3, "{shot:?}");
    let first = sprites_of(&harness.frame(), SHOT);
    harness.frame();
    let second = sprites_of(&harness.frame(), SHOT);
    let (dx, dy) = (second[0].x - first[0].x, second[0].y - first[0].y);
    let drawn = dx.atan2(-dy).to_degrees();
    assert!((drawn - bearing).abs() < 1.0, "drawn moving at {drawn}");
}

/// Every NPC idles, targets the player and holds its trigger.
#[derive(Debug)]
struct Attacking;

impl Behaviour for Attacking {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }

    fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> nova_sim::Trigger {
        nova_sim::Trigger {
            primary: true,
            secondary: None,
            only: None,
        }
    }

    fn target(&self, _npc: &Npc, _around: &Surroundings) -> Option<nova_sim::ShipRef> {
        Some(nova_sim::ShipRef::Player)
    }
}

/// Says no missile is hostile, counting the times it is asked.
#[derive(Debug, Default)]
struct Unalarmed {
    asked: std::cell::Cell<usize>,
}

impl PointDefenceRule for Unalarmed {
    fn hostile(&self, _defender: Side, _firer: Side, _govts: &nova_sim::Governments) -> bool {
        self.asked.set(self.asked.get() + 1);
        false
    }
}

/// The app in flight with the player carrying the Quad and the trader,
/// 200 pixels above it, firing the IR Missile at it, the router as
/// `router` makes it.
fn defending(router: impl FnOnce(AppScreen) -> AppScreen) -> Harness {
    Harness::flying_over(
        data_arming(&[133], &[134]),
        &[6, 6, 0, 0, 750, 550, 180],
        |screen| router(screen.with_behaviour(Rc::new(Attacking))),
    )
}

fn missiles(harness: &Harness) -> usize {
    harness
        .session()
        .shots()
        .iter()
        .filter(|shot| shot.weapon.id == nova_sim::WeaponId(134))
        .count()
}

#[test]
fn the_players_point_defence_shoots_down_the_traders_missile() {
    let mut harness = defending(|screen| screen);
    harness.until(|harness| missiles(harness) == 1);
    harness.until(|harness| missiles(harness) == 0);
    assert_eq!(harness.session().reserves().shield.now, 30.0, "untouched");
}

#[test]
fn the_routers_point_defence_rule_decides_for_the_flight() {
    let rule = Rc::new(Unalarmed::default());
    let given: Rc<dyn PointDefenceRule> = rule.clone();
    let mut harness = defending(|screen| screen.with_point_defence_rule(given));
    harness.until(|harness| missiles(harness) == 1);
    for _ in 0..20 {
        harness.frame();
    }
    assert!(rule.asked.get() > 0);
    assert_eq!(missiles(&harness), 1, "never hostile, never shot down");
}

// Patrolled, by Nova's AI and law.

/// A `gövt` of `class`, allied with the class `allies` (-1 for none);
/// disabling one of its ships costs 3, destroying one 7.
fn govt(class: i16, allies: i16) -> Vec<u8> {
    let mut bytes = vec![0; Govt::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x08, &[6, 0, 3, 0, 7, 0, 0, 100]);
    put_i16s(&mut bytes, 0x18, &[class, -1, -1, -1]);
    put_i16s(&mut bytes, 0x20, &[allies, -1, -1, -1]);
    put_i16s(&mut bytes, 0x28, &[-1; 4]);
    bytes
}

/// A `düde` of AI type `ai_type` and government `govt` flying `ship`.
fn dude_of(ai_type: i16, govt: i16, ship: i16) -> Vec<u8> {
    let mut bytes = vec![0; Dude::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[ai_type, govt]);
    put_i16s(&mut bytes, 0x08, &[-1; 16]);
    put_i16s(&mut bytes, 0x08, &[ship]);
    put_i16s(&mut bytes, 0x28, &[100]);
    bytes
}

/// An independent `sÿst` with no stellars, with `düde`s 128 and 129 at
/// half each and two ships on average.
fn patrolled_system() -> Vec<u8> {
    let mut bytes = system();
    put_i16s(&mut bytes, 0x44, &[128, 129, -1, -1, -1, -1, -1, -1]);
    put_i16s(&mut bytes, 0x54, &[50, 50]);
    put_i16s(&mut bytes, 0x64, &[2, -1]);
    bytes
}

/// The Gunship (ship 128) carries the blaster; Alpha flies wimpy traders
/// of `gövt` 128 in the "Trader" (ship 129: no shield, 300 armour) and
/// police interceptors of `gövt` 129, allied with the traders, in the
/// "Patrol" (ship 130: 30 shield, 45 armour, a gun, `wëap` 129, firing
/// every 10 ticks for 5 mass and 10 energy damage).
fn patrolled() -> Rc<GameData> {
    let mut gun = weapon(10, 5, -1, 0, 8, 0);
    put_i16s(&mut gun, 0x06, &[10]);
    let fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(
            Ship::TYPE,
            128,
            Some(b"Gunship"),
            &ship(30, 45, &[128], &[]),
        )
        .resource(Ship::TYPE, 129, Some(b"Trader"), &ship(0, 300, &[], &[]))
        .resource(Ship::TYPE, 130, Some(b"Patrol"), &ship(30, 45, &[129], &[]))
        .resource(
            Weapon::TYPE,
            128,
            Some(b"Blaster"),
            &weapon(0, 5, -1, 0, 8, 0),
        )
        .resource(Weapon::TYPE, 129, Some(b"Gun"), &gun)
        .resource(Govt::TYPE, 128, Some(b"Traders"), &govt(1, -1))
        .resource(Govt::TYPE, 129, Some(b"Police"), &govt(2, 1))
        .resource(Govt::TYPE, 130, Some(b"Neutrals"), &govt(3, -1))
        .resource(Dude::TYPE, 128, Some(b"Traders"), &dude_of(1, 128, 129))
        .resource(Dude::TYPE, 129, Some(b"Police"), &dude_of(4, 129, 130))
        .resource(System::TYPE, 128, Some(b"Alpha"), &patrolled_system())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(ShipAnim::TYPE, 129, None, &ship_anim(2001))
        .resource(ShipAnim::TYPE, 130, None, &ship_anim(2001))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(RLED, 2001, None, &sheet(36, 2))
        .resource(Spin::TYPE, 3000, None, &spin(3000, 6))
        .resource(RLED, 3000, None, &sheet(36, 3))
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

/// The red solid quads at most 17 units across each way: the hostile
/// brackets' lines.
fn red_lines(frame: &Frame) -> usize {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Solid(quads) => quads.clone(),
            _ => Vec::new(),
        })
        .filter(|quad| {
            let xs = quad.corners.map(|corner| corner.x);
            let ys = quad.corners.map(|corner| corner.y);
            let span = |values: [f32; 4]| {
                values.iter().copied().fold(f32::MIN, f32::max)
                    - values.iter().copied().fold(f32::MAX, f32::min)
            };
            quad.color == [1.0, 0.0, 0.0, 1.0] && span(xs) <= 17.0 && span(ys) <= 17.0
        })
        .count()
}

#[test]
fn attacking_a_trader_brings_the_police_costs_the_record_and_r_finds_the_threat() {
    // The trader 100 above the player, facing it; the police 699 below.
    let placing = [6, 6, 0, 0, 750, 650, 180, 0, 6, 6, 50, 0, 750, 1449, 0, 0];
    let mut harness = Harness::flying_over(patrolled(), &placing, |screen| {
        screen.with_disable_rule(Rc::new(nova_sim::NovaDisable))
    });
    let (trader, police) = (NpcId(0), NpcId(1));
    let goal_of = |harness: &Harness, id| {
        harness
            .session()
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .map(|npc| (npc.goal, npc.condition))
    };
    harness.key(Key::Space, true);
    let (mut fled, mut answered, mut disabled) = (false, false, false);
    for _ in 0..1200 {
        harness.frame();
        fled |= goal_of(&harness, trader).map(|(goal, _)| goal)
            == Some(Goal::Flee(nova_sim::ShipRef::Player));
        answered |= goal_of(&harness, police).map(|(goal, _)| goal)
            == Some(Goal::Attack(nova_sim::ShipRef::Player));
        if goal_of(&harness, trader).map(|(_, condition)| condition)
            == Some(nova_sim::Condition::Disabled)
        {
            disabled = true;
            break;
        }
    }
    harness.key(Key::Space, false);
    assert!(fled, "the trader fled from the player");
    assert!(answered, "the police turned on the player");
    assert!(disabled, "the trader was disabled");
    assert_eq!(
        harness
            .session()
            .pilot()
            .legal_record(nova_sim::GovtId(128)),
        -3
    );
    assert_eq!(
        harness
            .session()
            .pilot()
            .legal_record(nova_sim::GovtId(129)),
        -3,
        "the traders' allies hear of it"
    );
    for _ in 0..1200 {
        harness.frame();
        if harness.session().reserves().shield.now < 30.0 {
            break;
        }
    }
    assert!(
        harness.session().reserves().shield.now < 30.0,
        "the police's shots reached the player"
    );
    harness.key(Key::Char('r'), true);
    harness.key(Key::Char('r'), false);
    assert_eq!(
        harness.session().target().map(|npc| npc.id),
        Some(police),
        "the threat, not the disabled trader"
    );
    let frame = harness.frame();
    assert_eq!(red_lines(&frame), 8, "the hostile brackets");
}

/// Records each crime, and changes nothing.
#[derive(Debug, Default)]
struct Witness {
    seen: RefCell<Vec<nova_sim::Crime>>,
}

impl nova_sim::LegalCode for Witness {
    fn penalties(
        &self,
        crime: nova_sim::Crime,
        _victim: Option<nova_sim::GovtId>,
        _govts: &nova_sim::Governments,
    ) -> Vec<(nova_sim::GovtId, i32)> {
        self.seen.borrow_mut().push(crime);
        Vec::new()
    }
}

#[test]
fn the_routers_law_judges_the_flights_crimes() {
    let witness = Rc::new(Witness::default());
    let law: Rc<dyn nova_sim::LegalCode> = witness.clone();
    let mut harness =
        Harness::flying_over(patrolled(), &[6, 6, 0, 0, 750, 650, 180, 0], |screen| {
            screen
                .with_behaviour(Rc::new(Still))
                .with_disable_rule(Rc::new(nova_sim::NovaDisable))
                .with_law(law)
        });
    harness.key(Key::Space, true);
    for _ in 0..1200 {
        harness.frame();
        if !witness.seen.borrow().is_empty() {
            break;
        }
    }
    harness.key(Key::Space, false);
    assert_eq!(*witness.seen.borrow(), [nova_sim::Crime::Disable]);
    assert_eq!(
        harness
            .session()
            .pilot()
            .legal_record(nova_sim::GovtId(128)),
        0,
        "as the witness says"
    );
}

/// The law that the settings file at `path`, holding `text`, chooses,
/// read as `main` reads it: through the file adapter, into the rulebook
/// the law is built from.
fn saved_law(path: &Path, text: &str) -> nova_sim::NovaLaw {
    std::fs::create_dir_all(path.parent().expect("in a directory")).expect("creates");
    std::fs::write(path, text).expect("writes");
    let mut store: Option<Box<dyn nova_audio::SettingsStore>> =
        Some(Box::new(nova_audio::FileSettings::new(path)));
    let (rulebook, warnings) = nova::rulebook::game_rulebook(store.as_deref_mut());
    assert_eq!(warnings, Vec::<String>::new(), "{text}");
    nova_sim::NovaLaw::from_rulebook(&rulebook)
}

#[test]
fn the_saved_crime_gains_choose_whether_the_flights_crimes_please_a_neutral() {
    for (text, neutral) in [
        ("{}", 1),
        (r#"{"sound": false, "rules": "engine"}"#, 1),
        (r#"{"rules": "bible"}"#, 0),
        (
            r#"{"rules": "bible", "rule_overrides": {"crime_gains": "engine"}}"#,
            1,
        ),
        (r#"{"rule_overrides": {"crime_gains": "bible"}}"#, 0),
    ] {
        let home = tempfile::tempdir().expect("a temporary directory");
        let env = |name: &str| {
            matches!(name, "HOME" | "APPDATA" | "XDG_CONFIG_HOME")
                .then(|| home.path().as_os_str().to_owned())
        };
        let path =
            nova::config::settings_path(nova::config::Os::current(), env).expect("a settings path");
        let law = saved_law(&path, text);
        let mut harness =
            Harness::flying_over(patrolled(), &[6, 6, 0, 0, 750, 650, 180, 0], |screen| {
                screen
                    .with_behaviour(Rc::new(Still))
                    .with_disable_rule(Rc::new(nova_sim::NovaDisable))
                    .with_law(Rc::new(law))
            });
        let record = |harness: &Harness, govt| {
            harness
                .session()
                .pilot()
                .legal_record(nova_sim::GovtId(govt))
        };
        harness.key(Key::Space, true);
        for _ in 0..1200 {
            harness.frame();
            if record(&harness, 128) != 0 {
                break;
            }
        }
        harness.key(Key::Space, false);
        assert_eq!(
            [128, 129, 130].map(|govt| record(&harness, govt)),
            [-3, -3, neutral],
            "{text}: the traders and their allied police lower; the neutrals, \
             their own DisabPenalty 3, by the engine's law up 1"
        );
    }
}

// Boarding.

use nova_data::InterfaceData;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_sim::fixture::MemoryPilots;
use nova_sim::{Pilot, PilotKeeper, PilotStore};
use nova_view::geometry::Point;
use nova_view::menu::MenuChoice;

/// A `width` x `height` picture of one colour.
fn pict(width: i16, height: i16, rgb: [u8; 3]) -> Vec<u8> {
    let frame = [0, 0, height, width];
    let pixels = vec![rgb; (width * height) as usize];
    PictBuilder::new(frame)
        .direct_bits(&DirectBits::rgb888(frame, &pixels))
        .end()
        .build()
}

/// The first `chär` flies the "Boarder" (ship 128: a crew of 10, 30
/// shield, 45 armour and a blaster firing every 10 ticks, 10 mass damage
/// a shot) in Alpha (128), whose one `düde` flies the "Trader" (ship 129:
/// a crew of 1, no shield, 40 armour, 20 holds and 300 fuel) for food and
/// money (`Booty` 0x0041); food is commodity 0. The status bar, Nova's
/// button pictures and the plunder and assignment dialogs' pictures
/// (`PICT` 8515 and 8516) are there.
fn boarding_data() -> Rc<GameData> {
    let mut boarder = ship(30, 45, &[128], &[]);
    put_i16s(&mut boarder, 0x44, &[10]);
    let mut trader = ship(0, 40, &[], &[]);
    put_i16s(&mut trader, 0x00, &[20]);
    put_i16s(&mut trader, 0x44, &[1]);
    let mut dude = dude();
    put_i16s(&mut dude, 0x04, &[0x0041]);
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Boarder"), &boarder)
        .resource(Ship::TYPE, 129, Some(b"Trader"), &trader)
        .resource(
            Weapon::TYPE,
            128,
            Some(b"Blaster"),
            &weapon(10, 10, -1, 0, 8, 0),
        )
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(ShipAnim::TYPE, 129, None, &ship_anim(2001))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(RLED, 2001, None, &sheet(36, 2))
        .resource(Spin::TYPE, 3000, None, &spin(3000, 6))
        .resource(RLED, 3000, None, &sheet(36, 3))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system())
        .resource(Dude::TYPE, 128, Some(b"Traders"), &dude)
        .resource(
            nova_data::records::string_list::StrList::TYPE,
            4000,
            None,
            &str_list("Food"),
        )
        .resource(
            nova_data::records::string_list::StrList::TYPE,
            4004,
            None,
            &str_list("75"),
        )
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &interface(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture())
        .resource(PICT, 8515, None, &pict(30, 20, [40, 40, 40]))
        .resource(PICT, 8516, None, &pict(30, 20, [40, 40, 40]));
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

/// A `STR#` of one string.
fn str_list(string: &str) -> Vec<u8> {
    let mut bytes = 1_u16.to_be_bytes().to_vec();
    bytes.push(u8::try_from(string.len()).expect("short"));
    bytes.extend(string.as_bytes());
    bytes
}

/// One `DITL` user item at (left, top, right, bottom), enabled or not.
fn user_item((l, t, r, b): (i16, i16, i16, i16), enabled: bool) -> Vec<u8> {
    let mut bytes = vec![0; 4];
    for value in [t, l, b, r] {
        bytes.extend(value.to_be_bytes());
    }
    bytes.push(if enabled { 0 } else { 0x80 });
    bytes.push(0);
    bytes
}

/// A centred `DLOG` of `bounds` (top, left, bottom, right) naming `DITL`
/// `ditl`, and the `DITL` of `items`.
fn dialog(bounds: (i16, i16, i16, i16), ditl: i16, items: &[Vec<u8>]) -> (Vec<u8>, Vec<u8>) {
    let (t, l, b, r) = bounds;
    let mut dlog: Vec<u8> = [t, l, b, r, 1]
        .iter()
        .flat_map(|v| v.to_be_bytes())
        .collect();
    dlog.extend([1, 0, 0, 0, 0, 0, 0, 0]);
    dlog.extend(ditl.to_be_bytes());
    dlog.extend([0, 0, 0xA8, 0x0A]);
    let mut list = (i16::try_from(items.len()).expect("few") - 1)
        .to_be_bytes()
        .to_vec();
    list.extend(items.concat());
    (dlog, list)
}

/// The interface file's plunder dialog (`DLOG` 1011, 309 x 198) and
/// captured-ship assignment dialog (`DLOG` 1018, 257 x 114), as stock.
fn boarding_interface() -> InterfaceData {
    let at = |l: i16, t: i16, w: i16, h: i16| (l, t, l + w, t + h);
    let (plunder_dlog, plunder_ditl) = dialog(
        (40, 40, 238, 349),
        1011,
        &[
            user_item(at(91, 166, 126, 25), true),
            user_item(at(110, 110, 89, 25), true),
            user_item(at(35, 138, 89, 25), true),
            user_item(at(204, 110, 89, 25), true),
            user_item(at(11, 7, 287, 96), false),
            user_item(at(16, 110, 89, 25), true),
            user_item(at(129, 138, 146, 25), true),
        ],
    );
    let (assign_dlog, assign_ditl) = dialog(
        (40, 40, 154, 297),
        1018,
        &[
            user_item(at(55, 51, 146, 26), true),
            user_item(at(55, 83, 146, 26), true),
            user_item(at(9, 6, 238, 40), true),
        ],
    );
    let fork = ForkBuilder::new()
        .resource(Dlog::TYPE, 1011, None, &plunder_dlog)
        .resource(Ditl::TYPE, 1011, None, &plunder_ditl)
        .resource(Dlog::TYPE, 1018, None, &assign_dlog)
        .resource(Ditl::TYPE, 1018, None, &assign_ditl)
        .build()
        .bytes;
    InterfaceData::load(&OneFile(fork), Path::new("/Nova-DF.rsrc")).expect("loads")
}

/// The traffic's setup draws first (the trader 100 above the player,
/// facing down), then, for the capture, a self-destruct roll that misses
/// (99) and a capture roll that succeeds (0) on the draws of 100, and 1 on
/// every draw of 10 (so the capture's 1 in 10 misses); otherwise the last
/// outcome.
struct BoardChance {
    setup: VecDeque<u32>,
    of_100: VecDeque<u32>,
}

impl Chance for BoardChance {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        if let Some(draw) = self.setup.pop_front() {
            return draw;
        }
        match n {
            100 => self.of_100.pop_front().unwrap_or(n - 1),
            10 => 1,
            _ => n - 1,
        }
    }
}

struct Boarder {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
}

impl Boarder {
    /// The app over [`boarding_data`] and [`boarding_interface`], on the
    /// main menu, keeping pilots in `store`, its traffic and capture
    /// rolled on a [`BoardChance`], its NPCs idling.
    fn opening(store: &MemoryPilots) -> Self {
        let data = boarding_data();
        let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(BoardChance {
            setup: [6, 6, 0, 0, 750, 650, 180, 0].into(),
            of_100: [99, 0].into(),
        }));
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        let screen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), Rc::new(MonoMetrics))
            .with_dialogs(Rc::new(boarding_interface()), Rc::new(MonoMetrics))
            .with_chance(SharedChance::new(chance))
            .with_behaviour(Rc::new(Still));
        Self {
            app: App::new(&FakeWindow, data, screen),
            gpu: RecordingGpu::new(),
            frames: 0,
        }
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

    /// Opens the first saved pilot from the main menu.
    fn open_pilot(&mut self) {
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
        assert_eq!(self.showing(), Showing::Flight);
    }

    /// The centre of the open plunder or assignment dialog's item.
    fn item(&self, item: usize) -> Point {
        let screen = self.app.screen();
        let dialog = match (screen.plunder(), screen.assignment()) {
            (Some(plunder), _) => plunder.dialog(),
            (None, Some(assignment)) => assignment.dialog(),
            (None, None) => panic!("no boarding dialog is open"),
        };
        dialog.item_bounds(item).expect("an item").center()
    }
}

/// The flight keys a press of which `controls` hold.
fn flight_keys(controls: nova_sim::Controls) -> [(Key, bool); 3] {
    [
        (Key::Up, controls.thrust),
        (Key::Left, controls.turn == nova_sim::Turn::Left),
        (Key::Right, controls.turn == nova_sim::Turn::Right),
    ]
}

/// The controls to come over `quarry` at its velocity, closing in more
/// slowly the nearer it is, then to face its heading or the reverse.
fn steering(session: &Session, quarry: &Npc) -> nova_sim::Controls {
    use nova_sim::flight::{heading_of, shortest_turn};
    let player = *session.player();
    let accel = session.handling().accel;
    let off = quarry.state.position - player.position;
    let distance = off.length();
    let wanted = if distance > 0.0 {
        off * ((distance * 0.02).min(1.5) / distance)
    } else {
        nova_sim::Vec2::ZERO
    };
    let error = wanted - (player.velocity - quarry.state.velocity);
    let toward = |heading: f32| {
        let turn = shortest_turn(player.heading, heading);
        let side = if turn > 1.5 {
            nova_sim::Turn::Right
        } else if turn < -1.5 {
            nova_sim::Turn::Left
        } else {
            nova_sim::Turn::None
        };
        (turn, side)
    };
    if error.length() > accel || distance > quarry.hull.board_reach / 3.0 {
        let (turn, side) = toward(heading_of(error));
        return nova_sim::Controls {
            thrust: turn.abs() < 10.0 && error.length() > accel / 2.0,
            turn: side,
            reverse: false,
        };
    }
    let ahead = toward(quarry.state.heading);
    let back = toward((quarry.state.heading + 180.0) % 360.0);
    let (turn, side) = if ahead.0.abs() <= back.0.abs() {
        ahead
    } else {
        back
    };
    nova_sim::Controls {
        turn: if turn.abs() <= 20.0 {
            nova_sim::Turn::None
        } else {
            side
        },
        ..nova_sim::Controls::default()
    }
}

/// The app with a saved pilot, "Ada", resumed in flight from the main
/// menu, who disables the trader with the blaster (Space), targets it
/// (Tab), flies over it with the arrow keys and boards it (B).
fn boarded(store: &MemoryPilots) -> Boarder {
    let data = boarding_data();
    PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>)
        .save(&Pilot::new(data.as_ref(), "Ada").expect("a pilot"))
        .expect("saved");
    let mut game = Boarder::opening(store);
    game.open_pilot();
    // The first frame runs no step; the second's sets the system up.
    game.frame();
    game.frame();
    let trader = game.session().npcs()[0].id;
    assert_eq!(
        game.session().npcs()[0].state.position,
        nova_sim::Vec2::new(0.0, -100.0)
    );
    game.tap(Key::Tab);
    game.key(Key::Space, true);
    for _ in 0..600 {
        game.frame();
        if game.session().npcs()[0].condition == nova_sim::Condition::Disabled {
            break;
        }
    }
    game.key(Key::Space, false);
    assert_eq!(
        game.session().npcs()[0].condition,
        nova_sim::Condition::Disabled,
        "the blaster disabled the trader"
    );
    let mut held = [(Key::Up, false), (Key::Left, false), (Key::Right, false)];
    for _ in 0..6000 {
        let quarry = game
            .session()
            .npcs()
            .iter()
            .find(|npc| npc.id == trader)
            .expect("the trader")
            .clone();
        let keys = flight_keys(steering(game.session(), &quarry));
        for (now, was) in keys.iter().zip(held.iter_mut()) {
            if now.1 != was.1 {
                game.key(now.0, now.1);
                was.1 = now.1;
            }
        }
        game.tap(Key::Char('b'));
        if game.showing() == Showing::Plunder {
            break;
        }
        game.frame();
    }
    assert_eq!(game.showing(), Showing::Plunder, "boarded");
    for (key, down) in held {
        if down {
            game.key(key, false);
        }
    }
    game
}

/// The text runs of `frame`.
fn run_texts(frame: &Frame) -> Vec<String> {
    runs(frame).into_iter().map(|run| run.text).collect()
}

#[test]
fn boarding_a_disabled_trader_plunders_it_and_captures_it_into_the_saved_fleet() {
    let store = MemoryPilots::new();
    let mut game = boarded(&store);
    let frame = game.frame();
    let shown = run_texts(&frame);
    assert!(
        shown.iter().any(|text| text == "Capture Odds: 75%"),
        "{shown:?}"
    );
    let color = |label: &str| {
        runs(&frame)
            .into_iter()
            .find(|run| run.text == label)
            .map(|run| run.color)
    };
    assert_eq!(color("Credits"), color("Cargo"), "both enabled");
    assert_ne!(color("Credits"), color("Ammo"), "no ammo: greyed");
    let cash = game.pilot().cash();
    game.click(game.item(3));
    assert_eq!(game.pilot().cash(), cash + 1000, "the credits taken");
    game.click(game.item(7));
    assert_eq!(game.showing(), Showing::Assignment, "captured");
    game.click(game.item(2));
    assert_eq!(game.showing(), Showing::Flight);
    let shown = run_texts(&game.frame());
    assert!(
        shown
            .iter()
            .any(|text| text == "You assigned this ship to your fleet of escorts."),
        "{shown:?}"
    );
    let saved = nova_sim::save::decode(&store.text("Ada").expect("saved")).expect("a pilot");
    assert_eq!(
        saved
            .escorts()
            .iter()
            .map(|escort| escort.ship)
            .collect::<Vec<_>>(),
        [nova_sim::ShipId(129)]
    );

    // A new app: Open Pilot resumes the pilot with its escort.
    let mut game = Boarder::opening(&store);
    game.open_pilot();
    assert_eq!(game.pilot().escorts().len(), 1);
    assert_eq!(game.pilot().escorts()[0].ship, nova_sim::ShipId(129));
}

#[test]
fn use_as_my_ship_flies_the_captured_trader_and_keeps_the_old_ship() {
    let store = MemoryPilots::new();
    let mut game = boarded(&store);
    // The credits, then the capture: its press rolls the self-destruct.
    game.click(game.item(3));
    game.click(game.item(7));
    assert_eq!(game.showing(), Showing::Assignment);
    game.click(game.item(1));
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.session().ship(), nova_sim::ShipId(129));
    let shown = run_texts(&game.frame());
    assert!(
        shown
            .iter()
            .any(|text| text == "You retained your old ship as an escort."),
        "{shown:?}"
    );
    assert_eq!(
        game.pilot()
            .escorts()
            .iter()
            .map(|escort| escort.ship)
            .collect::<Vec<_>>(),
        [nova_sim::ShipId(128)]
    );
}
