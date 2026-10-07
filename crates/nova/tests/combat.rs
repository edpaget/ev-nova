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
//!
//! Hailed, with the interface file's comm and haggle dialogs: Y opens the
//! comm dialog on a xenophobic pirate attacking the player, "What is it
//! you want?", with Greetings, Beg For Mercy and Close Channel; R asks
//! "Pay me 3,000 credits.", and Accept Price pays it off ("A pleasure
//! doing business with you."), after which the pirate fires no more. A
//! friendly trader asked for assistance by a player out of fuel names its
//! price, and once paid flies alongside and refuels the player, saying
//! "Hauler:  Energy transfer complete.". A hail option registered at the
//! edge shows its button, and pressing it shows its reply.
//!
//! Escorted, by Nova's escort AI over idle traffic: a trader captured
//! "Use As Escort" stays beside the player, and follows it through a jump
//! plotted on the map. F sends a warship escort at a targeted pirate,
//! saying "New escort orders assigned:  All ships attacking target.", and
//! the pirate's shield drops. E, 4 and V hold the warships alone, which
//! stay put while the freighter follows the player, and C recalls them.
//! Option-Tab and Y hail an escort, "What can I do for you?", with Release
//! alone; R says "Goodbye, captain." and closing the channel releases it,
//! which the saved pilot keeps. Standing orders are saved on landing, and
//! reopened they are back to formation by the engine's `escort_orders`,
//! or kept by its other reading in the settings file.
//!
//! Carrying fighters: W selects the player's fighter bay and Control
//! launches a fighter, which is drawn beside the player while the
//! secondary line counts one fewer; F sends it at a pirate targeted,
//! whose shield drops, or by `fighter_launch`'s other reading in the
//! settings file it attacks the target at once. Option-C says "New escort
//! orders assigned:  All ships returning to hangar." and the fighters fly
//! home and dock, the line counting them again. A pirate carrier attacking
//! the player launches fighters that take the player's shield down. A
//! fighter out is saved on landing and flies again after take-off, or by
//! `fighter_recall`'s other reading is aboard as soon as the pilot lands.

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
use nova_data::records::stellar::Stellar;
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
    // For sale every day (`BuyRandom` 100).
    put_i16s(&mut bytes, 0x3F0, &[100]);
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
            turrets_only: false,
            bays: false,
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
    interface_of(&boarding_dialogs())
}

/// [`boarding_interface`]'s resources.
fn boarding_dialogs() -> Vec<(ResType, i16, Vec<u8>)> {
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
    vec![
        (Dlog::TYPE, 1011, plunder_dlog),
        (Ditl::TYPE, 1011, plunder_ditl),
        (Dlog::TYPE, 1018, assign_dlog),
        (Ditl::TYPE, 1018, assign_ditl),
    ]
}

/// An interface file of `resources`.
fn interface_of(resources: &[(ResType, i16, Vec<u8>)]) -> InterfaceData {
    let fork = resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, bytes)| {
            fork.resource(*ty, *id, None, bytes)
        })
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
        let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(BoardChance {
            setup: [6, 6, 0, 0, 750, 650, 180, 0].into(),
            of_100: [99, 0].into(),
        }));
        Self::opening_over(
            boarding_data(),
            boarding_interface(),
            store,
            SharedChance::new(chance),
            |screen| screen.with_behaviour(Rc::new(Still)),
        )
    }

    /// The app over `data` and `interface`, on the main menu, keeping
    /// pilots in `store`, rolling on `chance`, the router as `router`
    /// makes it.
    fn opening_over(
        data: Rc<GameData>,
        interface: InterfaceData,
        store: &MemoryPilots,
        chance: SharedChance,
        router: impl FnOnce(AppScreen) -> AppScreen,
    ) -> Self {
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        let screen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), Rc::new(MonoMetrics))
            .with_dialogs(Rc::new(interface), Rc::new(MonoMetrics))
            .with_chance(chance);
        let screen = router(screen);
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

    /// Opens the first saved pilot from the main menu, in flight.
    fn open_pilot(&mut self) {
        self.open_pilot_to(Showing::Flight);
    }

    /// Opens the first saved pilot from the main menu, which then shows
    /// `showing`: flight, or the spaceport of a pilot saved landed.
    fn open_pilot_to(&mut self, showing: Showing) {
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
    boarded_with(store, boarding_data(), boarding_interface(), |screen| {
        screen.with_behaviour(Rc::new(Still))
    })
}

/// [`boarded`] over `data` and `interface`, the router as `router` makes
/// it, the trader placed and the capture rolled as [`Boarder::opening`]
/// rolls them.
fn boarded_with(
    store: &MemoryPilots,
    data: Rc<GameData>,
    interface: InterfaceData,
    router: impl FnOnce(AppScreen) -> AppScreen,
) -> Boarder {
    PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>)
        .save(&Pilot::new(data.as_ref(), "Ada").expect("a pilot"))
        .expect("saved");
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(BoardChance {
        setup: [6, 6, 0, 0, 750, 650, 180, 0].into(),
        of_100: [99, 0].into(),
    }));
    let mut game = Boarder::opening_over(data, interface, store, SharedChance::new(chance), router);
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

// Hailing.

use nova_sim::hail::{Answer, Hail, HailOption};
use nova_sim::{HailOptions, Reply, Rulebook};

/// Each ship comm string group's first words, said in every variant;
/// every other group is "c<g>".
const SAID: [(usize, &str); 7] = [
    (0, "Channel open."),
    (2, "What is it you want?"),
    (20, "A pleasure doing business with you."),
    (23, "You're lucky - I'm in a good mood today."),
    (24, "I'm in a bad mood today, so it's going to cost you."),
    (28, "I'll help you out if you pay me."),
    (29, "Okay, I'm on my way."),
];

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

/// `STR#` 3000: [`SAID`]'s words for each group's five variants.
fn comm_strings() -> Vec<String> {
    (0..40)
        .flat_map(|group| {
            let said = SAID
                .iter()
                .find(|(at, _)| *at == group)
                .map_or_else(|| format!("c{group}"), |(_, said)| (*said).to_owned());
            std::iter::repeat_n(said, 5)
        })
        .collect()
}

/// A `gövt` of `flags`, named `name` when hailed.
fn hailed_govt(flags: u16, name: &str) -> Vec<u8> {
    let mut bytes = govt(4, -1);
    bytes[0x02..0x04].copy_from_slice(&flags.to_be_bytes());
    bytes[0x34..0x34 + name.len()].copy_from_slice(name.as_bytes());
    bytes
}

/// A `shïp` with this `Shield`, `Armor` and `weapons`, named `name` when
/// hailed.
fn hailed_ship(shield: i16, armor: i16, weapons: &[i16], name: &str) -> Vec<u8> {
    let mut bytes = ship(shield, armor, weapons, &[]);
    bytes[0x60E..0x60E + name.len()].copy_from_slice(name.as_bytes());
    bytes
}

/// The first `chär` flies the "Hailer" (ship 128: 30 shield, 45 armour,
/// a blaster) in Alpha (128), independent, whose one `düde`, of AI type
/// `ai_type` and `gövt` `govt`, flies ship `ship`: the pirates' (137,
/// xenophobes whose warships take bribes, `Flags` 0x0201) "Raider" (129:
/// 30 shield, 45 armour and a gun firing every 10 ticks), or the
/// Civvies' (157, whose traders take bribes, 0x2000) "Hauler" (130: no
/// shield, 40 armour). `STR#` 3000 holds the replies and 2002 #175
/// "Greetings."; the status bar, the buttons' pictures, the comm and
/// haggle dialogs' (`PICT` 8511 and 8514) and the ships' (5001 and 5002)
/// are there.
fn hailing_data(ai_type: i16, govt: i16, ship: i16) -> Rc<GameData> {
    let mut gun = weapon(10, 5, -1, 0, 8, 0);
    put_i16s(&mut gun, 0x06, &[10]);
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(
            Ship::TYPE,
            128,
            Some(b"Hailer"),
            &hailed_ship(30, 45, &[128], "Hailer"),
        )
        .resource(
            Ship::TYPE,
            129,
            Some(b"Raider"),
            &hailed_ship(30, 45, &[129], "Raider"),
        )
        .resource(
            Ship::TYPE,
            130,
            Some(b"Hauler"),
            &hailed_ship(0, 40, &[], "Hauler"),
        )
        .resource(
            Weapon::TYPE,
            128,
            Some(b"Blaster"),
            &weapon(10, 10, -1, 0, 8, 0),
        )
        .resource(Weapon::TYPE, 129, Some(b"Gun"), &gun)
        .resource(
            Govt::TYPE,
            137,
            Some(b"Pirates"),
            &hailed_govt(0x0201, "Pirate"),
        )
        .resource(
            Govt::TYPE,
            157,
            Some(b"Civvies"),
            &hailed_govt(0x2000, "Civilian"),
        )
        .resource(
            Dude::TYPE,
            128,
            Some(b"Hailed"),
            &dude_of(ai_type, govt, ship),
        )
        .resource(System::TYPE, 128, Some(b"Alpha"), &system())
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
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture())
        .resource(PICT, 8511, None, &pict(30, 20, [40, 40, 40]))
        .resource(PICT, 8514, None, &pict(30, 20, [40, 40, 40]))
        .resource(PICT, 5001, None, &pict(20, 20, [90, 90, 90]))
        .resource(PICT, 5002, None, &pict(20, 20, [90, 90, 90]));
    for state in [7500, 7503, 7506] {
        fork = fork
            .resource(PICT, state, None, &pict(13, 25, [200, 0, 0]))
            .resource(PICT, state + 1, None, &pict(2, 25, [0, 200, 0]))
            .resource(PICT, state + 2, None, &pict(13, 25, [0, 0, 200]))
            .resource(PICT, state + 100, None, &pict(13, 25, [0, 0, 0]))
            .resource(PICT, state + 102, None, &pict(13, 25, [0, 0, 0]));
    }
    let file = OneFile(hailing_strings(fork).build().bytes);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

/// `fork` with the strings hailing reads: `STR#` 3000's replies, 2002's
/// messages ("Greetings." at #175, "m<n>" elsewhere) and 9000's "Fine
/// weather.".
fn hailing_strings(fork: ForkBuilder) -> ForkBuilder {
    use nova_data::records::string_list::StrList;
    let messages: Vec<String> = (1..=200)
        .map(|n| {
            if n == 175 {
                "Greetings.".to_owned()
            } else {
                format!("m{n}")
            }
        })
        .collect();
    fork.resource(StrList::TYPE, 3000, None, &strings(&comm_strings()))
        .resource(StrList::TYPE, 2002, None, &strings(&messages))
        .resource(StrList::TYPE, 9000, None, &str_list("Fine weather."))
}

/// The interface file's comm dialog (`DLOG` 1007, 423 x 215) and haggle
/// dialog (`DLOG` 1008, 262 x 107), as stock.
fn hailing_interface() -> InterfaceData {
    interface_of(&hailing_dialogs())
}

/// [`hailing_interface`]'s resources.
fn hailing_dialogs() -> Vec<(ResType, i16, Vec<u8>)> {
    let at = |l: i16, t: i16, w: i16, h: i16| (l, t, l + w, t + h);
    let (comm_dlog, comm_ditl) = dialog(
        (78, 51, 293, 474),
        1007,
        &[
            user_item(at(21, 181, 166, 26), true),
            user_item(at(21, 153, 166, 26), true),
            user_item(at(21, 125, 166, 26), true),
            user_item(at(46, 241, 200, 25), true),
            user_item(at(7, 320, 200, 25), true),
            user_item(at(199, 335, 200, 25), true),
            user_item(at(178, 261, 200, 25), true),
            user_item(at(178, 289, 200, 25), true),
            user_item(at(34, 299, 112, 16), false),
            user_item(at(11, 8, 192, 58), false),
            user_item(at(216, 7, 200, 200), false),
            user_item(at(40, 73, 134, 46), false),
        ],
    );
    let (haggle_dlog, haggle_ditl) = dialog(
        (40, 40, 147, 302),
        1008,
        &[
            user_item(at(58, 74, 146, 26), true),
            user_item(at(58, 39, 146, 26), true),
            user_item(at(7, 6, 248, 25), false),
        ],
    );
    vec![
        (Dlog::TYPE, 1007, comm_dlog),
        (Ditl::TYPE, 1007, comm_ditl),
        (Dlog::TYPE, 1008, haggle_dlog),
        (Ditl::TYPE, 1008, haggle_ditl),
    ]
}

/// The app over [`hailing_data`] of `dude` and [`hailing_interface`],
/// with a saved pilot, "Ada", holding 10,000 credits and `fuel`, resumed
/// in flight from the main menu; the hailed ship placed 100 pixels above
/// the player facing down by the setup draws, then every draw the last
/// outcome; Nova's AI.
fn hailer(store: &MemoryPilots, dude: (i16, i16, i16), fuel: f32) -> Boarder {
    let data = hailing_data(dude.0, dude.1, dude.2);
    let pilot = Pilot::new(data.as_ref(), "Ada").expect("a pilot");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["cash"] = serde_json::json!(10_000);
    save["reserves"]["fuel"]["now"] = serde_json::json!(fuel);
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>)
        .save(&pilot)
        .expect("saved");
    let (_, chance) = scripted(&[6, 6, 0, 0, 750, 650, 180, 0]);
    let mut game = Boarder::opening_over(data, hailing_interface(), store, chance, |screen| screen);
    game.open_pilot();
    // The first frame runs no step; the second's sets the system up.
    game.frame();
    game.frame();
    assert_eq!(
        game.session().npcs()[0].state.position,
        nova_sim::Vec2::new(0.0, -100.0)
    );
    game
}

impl Boarder {
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

    /// Targets the ship with Tab and hails it with Y.
    fn hail(&mut self) {
        self.tap(Key::Tab);
        self.tap(Key::Char('y'));
        assert_eq!(self.showing(), Showing::Comm);
    }
}

#[test]
fn begging_a_hostile_pirate_for_mercy_pays_it_off() {
    let store = MemoryPilots::new();
    let mut game = hailer(&store, (3, 137, 129), 300.0);
    for _ in 0..600 {
        game.frame();
        if game.session().reserves().shield.now < 30.0 {
            break;
        }
    }
    assert!(
        game.session().reserves().shield.now < 30.0,
        "the pirate attacks"
    );
    game.hail();
    assert_eq!(game.reply(), "What is it you want?");
    let shown = run_texts(&game.frame());
    let buttons: Vec<&String> = shown
        .iter()
        .filter(|text| {
            [
                "Greetings",
                "Beg For Mercy",
                "Close Channel",
                "Request Assistance",
            ]
            .contains(&text.as_str())
        })
        .collect();
    assert_eq!(buttons, ["Close Channel", "Beg For Mercy", "Greetings"]);
    game.tap(Key::Char('r'));
    assert_eq!(game.showing(), Showing::Haggle);
    let shown = run_texts(&game.frame());
    assert!(
        shown.iter().any(|text| text == "Pay me 3,000 credits."),
        "{shown:?}"
    );
    game.tap(Key::Enter);
    assert_eq!(game.showing(), Showing::Comm);
    assert_eq!(game.reply(), "A pleasure doing business with you.");
    assert_eq!(game.pilot().cash(), 7000);
    game.tap(Key::Char('e'));
    assert_eq!(game.showing(), Showing::Flight);
    // The shots already in flight run their course.
    for _ in 0..60 {
        game.frame();
    }
    let shield = game.session().reserves().shield.now;
    for _ in 0..200 {
        game.frame();
    }
    assert_eq!(
        game.session().reserves().shield.now,
        shield,
        "spared: the pirate fires no more"
    );
}

#[test]
fn requesting_assistance_from_a_friendly_trader_refuels_the_player() {
    let store = MemoryPilots::new();
    let mut game = hailer(&store, (1, 157, 130), 0.0);
    game.hail();
    assert_eq!(game.reply(), "Channel open.");
    let shown = run_texts(&game.frame());
    let buttons: Vec<&String> = shown
        .iter()
        .filter(|text| {
            [
                "Greetings",
                "Beg For Mercy",
                "Close Channel",
                "Request Assistance",
            ]
            .contains(&text.as_str())
        })
        .collect();
    assert_eq!(
        buttons,
        ["Close Channel", "Request Assistance", "Greetings"]
    );
    game.tap(Key::Char('r'));
    assert_eq!(
        game.reply(),
        "I'm in a bad mood today, so it's going to cost you."
    );
    assert_eq!(game.showing(), Showing::Haggle);
    game.tap(Key::Enter);
    assert_eq!(game.reply(), "Okay, I'm on my way.");
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    let mut said = false;
    let mut alongside = false;
    for _ in 0..1200 {
        let frame = game.frame();
        let helper = &game.session().npcs()[0];
        let off = helper.state.position - game.session().player().position;
        alongside |= off.x.abs() <= 105.0 && off.y.abs() <= 105.0;
        said |= run_texts(&frame)
            .iter()
            .any(|text| text == "Hauler:  Energy transfer complete.");
        if said {
            break;
        }
    }
    assert!(alongside, "the trader flew alongside");
    assert!(said, "it said it was done");
    assert!(game.session().reserves().fuel.now > 100.0, "the fuel rose");
    assert_eq!(game.pilot().cash(), 7000, "paid");
}

/// A hail option for every ship, keyed W, that says `STR#` 9000 #1.
#[derive(Debug)]
struct Weather;

impl HailOption for Weather {
    fn label(&self) -> String {
        "Weather".to_owned()
    }

    fn key(&self) -> Option<char> {
        Some('W')
    }

    fn applies(&self, _hail: &Hail) -> bool {
        true
    }

    fn press(&self, _hail: &Hail, _chance: &mut dyn Chance) -> Answer {
        Answer::say(Reply::Line {
            list: 9000,
            index: 1,
        })
    }
}

#[test]
fn a_hail_option_registered_at_the_edge_shows_its_button_and_its_reply() {
    let options = HailOptions::nova(&Rulebook::default()).with(Rc::new(Weather));
    let mut harness = Harness::flying_over(
        hailing_data(1, 157, 130),
        &[6, 6, 0, 0, 750, 650, 180, 0],
        |screen| screen.with_hail_options(options),
    );
    harness.key(Key::Tab, true);
    harness.key(Key::Tab, false);
    harness.key(Key::Char('y'), true);
    harness.key(Key::Char('y'), false);
    assert_eq!(harness.app.screen().showing(), Showing::Comm);
    let frame = harness.frame();
    assert!(text_at(&frame, "Weather").is_some(), "its button");
    let at = harness
        .app
        .screen()
        .comm()
        .expect("open")
        .dialog()
        .item_bounds(13)
        .expect("the third option's place")
        .center();
    harness.app.handle(
        WindowEvent::PointerMoved {
            px: (f64::from(at.x), f64::from(at.y)),
        },
        &mut FakeWindow,
        &mut harness.gpu,
    );
    for pressed in [true, false] {
        harness.app.handle(
            WindowEvent::PointerButton {
                button: nova_view::MouseButton::Left,
                pressed,
            },
            &mut FakeWindow,
            &mut harness.gpu,
        );
    }
    let frame = harness.frame();
    assert!(text_at(&frame, "Fine weather.").is_some(), "its reply");
}

// Escorts.

use nova_sim::{AiType, EscortOrder, NovaAi, RuleSource};

/// A `sÿst` at map (`x`, 0) with these hyperlinks, `stellar` if any, and
/// `dudes` at an equal share, one ship of each on average; independent.
fn escort_system(x: i16, links: &[i16], stellar: Option<i16>, dudes: &[i16]) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0]);
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x04, links);
    if let Some(stellar) = stellar {
        put_i16s(&mut bytes, 0x24, &[stellar]);
    }
    put_i16s(&mut bytes, 0x44, &[-1; 8]);
    put_i16s(&mut bytes, 0x44, dudes);
    let share = i16::try_from(100 / dudes.len().max(1)).expect("small");
    for slot in 0..dudes.len() {
        put_i16s(&mut bytes, 0x54 + 2 * slot, &[share]);
    }
    let count = i16::try_from(dudes.len()).expect("few");
    put_i16s(&mut bytes, 0x64, &[count, -1]);
    bytes
}

/// A `spöb` at the centre that can be landed on, drawn from `spïn` 1004.
fn landing_pad() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, 0, 4]);
    bytes[0x06..0x0A].copy_from_slice(&0x01_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x18, &[-1, -1]);
    bytes
}

/// `STR#` 3001: "Goodbye, captain." and the farewells after it, each the
/// first of a group of five.
fn more_comm_strings() -> Vec<String> {
    ["Goodbye, captain.", "See you around the galaxy."]
        .iter()
        .flat_map(|said| std::iter::repeat_n((*said).to_owned(), 5))
        .collect()
}

/// [`boarding_data`]'s pilot (the "Boarder", ship 128, with a blaster) in
/// Alpha (128, at the centre of the map), over a landing pad, linked to
/// Beta (129), whose `düde`s are `alpha_dudes`: 128 the traders' "Trader"
/// (ship 129, a freighter escort, `EscortType` 3, carrying food and
/// money), and 129 the pirates' (`gövt` 137) "Raider" (ship 131: 30
/// shield, 45 armour, unarmed). Ship 130 is the "Warship", a warship
/// escort (`EscortType` 2, `InherentAI` 3: 30 shield, 45 armour and a gun,
/// `wëap` 129, firing every 10 ticks for 5 mass and 10 energy damage). `STR#` 3000 and 3001 hold the replies, with "What can I do
/// for you?" (group 4) for an escort.
fn escort_data(alpha_dudes: &[i16]) -> Rc<GameData> {
    use nova_data::records::string_list::StrList;
    let mut boarder = ship(30, 45, &[128], &[]);
    put_i16s(&mut boarder, 0x44, &[10]);
    let mut trader = ship(0, 40, &[], &[]);
    put_i16s(&mut trader, 0x00, &[20]);
    put_i16s(&mut trader, 0x44, &[1]);
    put_i16s(&mut trader, 0x732, &[3]);
    let mut gun = weapon(10, 5, -1, 0, 8, 0);
    put_i16s(&mut gun, 0x06, &[10]);
    let mut warship = ship(30, 45, &[129], &[]);
    put_i16s(&mut warship, 0x42, &[3]);
    put_i16s(&mut warship, 0x732, &[2]);
    let mut traders = dude();
    put_i16s(&mut traders, 0x04, &[0x0041]);
    let mut replies = comm_strings();
    for variant in 0..5 {
        "What can I do for you?".clone_into(&mut replies[20 + variant]);
    }
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Boarder"), &boarder)
        .resource(Ship::TYPE, 129, Some(b"Trader"), &trader)
        .resource(Ship::TYPE, 130, Some(b"Warship"), &warship)
        .resource(Ship::TYPE, 131, Some(b"Raider"), &ship(30, 45, &[], &[]))
        .resource(
            Weapon::TYPE,
            128,
            Some(b"Blaster"),
            &weapon(10, 10, -1, 0, 8, 0),
        )
        .resource(Weapon::TYPE, 129, Some(b"Gun"), &gun)
        .resource(Govt::TYPE, 137, Some(b"Pirates"), &hailed_govt(0, "Pirate"))
        .resource(Dude::TYPE, 128, Some(b"Traders"), &traders)
        .resource(Dude::TYPE, 129, Some(b"Pirates"), &dude_of(3, 137, 131))
        .resource(
            System::TYPE,
            128,
            Some(b"Alpha"),
            &escort_system(0, &[129], Some(128), alpha_dudes),
        )
        .resource(
            System::TYPE,
            129,
            Some(b"Beta"),
            &escort_system(100, &[128], None, &[]),
        )
        .resource(Stellar::TYPE, 128, Some(b"Pad"), &landing_pad())
        .resource(Spin::TYPE, 1004, None, &spin(1000, 1))
        .resource(RLED, 1000, None, &sheet(1, 40))
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(Spin::TYPE, 3000, None, &spin(3000, 6))
        .resource(RLED, 3000, None, &sheet(36, 3))
        .resource(RLED, 2001, None, &sheet(36, 2))
        .resource(StrList::TYPE, 4000, None, &str_list("Food"))
        .resource(StrList::TYPE, 4004, None, &str_list("75"))
        .resource(StrList::TYPE, 3000, None, &strings(&replies))
        .resource(StrList::TYPE, 3001, None, &strings(&more_comm_strings()))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &interface(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture());
    for id in 129..=131 {
        fork = fork.resource(ShipAnim::TYPE, id, None, &ship_anim(2001));
    }
    for id in [8511, 8514, 8515, 8516, 5001, 5002, 5003] {
        fork = fork.resource(PICT, id, None, &pict(30, 20, [40, 40, 40]));
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

/// The interface file's plunder, assignment, comm and haggle dialogs.
fn escort_interface() -> InterfaceData {
    let mut resources = boarding_dialogs();
    resources.extend(hailing_dialogs());
    interface_of(&resources)
}

/// Nova's AI with every AI type idling: the traffic sits still and only
/// the escorts fly, by Nova's escort AI.
fn escorts_only() -> Rc<dyn Behaviour> {
    let still: Rc<dyn Behaviour> = Rc::new(Still);
    let ai = [
        AiType::WimpyTrader,
        AiType::BraveTrader,
        AiType::Warship,
        AiType::Interceptor,
    ]
    .into_iter()
    .fold(NovaAi::default(), |ai, ai_type| {
        ai.with(ai_type, Rc::clone(&still))
    });
    Rc::new(ai)
}

/// `pilot` with a fleet of `ships`, full.
fn with_fleet(pilot: &Pilot, ships: &[i16]) -> Pilot {
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(pilot)).expect("JSON");
    let gauge = |max: f32| serde_json::json!({"now": max, "max": max});
    save["escorts"] = ships
        .iter()
        .map(|ship| {
            serde_json::json!({
                "ship": ship,
                "reserves": {"shield": gauge(30.0), "armor": gauge(45.0), "fuel": gauge(300.0)},
                "order": null,
                "carried": false,
                "wage": null,
                "person": null
            })
        })
        .collect();
    nova_sim::save::decode(&save.to_string()).expect("a pilot")
}

/// The app over [`escort_data`] of `alpha_dudes` and [`escort_interface`],
/// with a saved pilot, "Ada", whose fleet is `fleet`, resumed in flight
/// from the main menu, its traffic placed 300 above the player facing
/// down; the escorts flying alone ([`escorts_only`]), the router as
/// `router` makes it on top. Two frames place the system's ships.
fn escorted(
    store: &MemoryPilots,
    alpha_dudes: &[i16],
    fleet: &[i16],
    router: impl FnOnce(AppScreen) -> AppScreen,
) -> Boarder {
    let data = escort_data(alpha_dudes);
    let pilot = with_fleet(&Pilot::new(data.as_ref(), "Ada").expect("a pilot"), fleet);
    PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>)
        .save(&pilot)
        .expect("saved");
    let mut game = reopened(store, alpha_dudes, |screen| {
        router(screen.with_behaviour(escorts_only()))
    });
    game.open_pilot();
    game.frame();
    game.frame();
    game
}

/// The app over [`escort_data`] of `alpha_dudes` on the main menu, with
/// `store`'s pilots, its traffic placed 300 above the player; with no
/// traffic, nothing is drawn but the last outcome.
fn reopened(
    store: &MemoryPilots,
    alpha_dudes: &[i16],
    router: impl FnOnce(AppScreen) -> AppScreen,
) -> Boarder {
    let setup: &[u32] = if alpha_dudes.is_empty() {
        &[]
    } else {
        &[6, 6, 0, 0, 750, 450, 180, 0]
    };
    let (_, chance) = scripted(setup);
    Boarder::opening_over(
        escort_data(alpha_dudes),
        escort_interface(),
        store,
        chance,
        router,
    )
}

impl Boarder {
    /// The player's escorts' NPCs.
    fn escorts(&self) -> Vec<Npc> {
        self.session()
            .npcs()
            .iter()
            .filter(|npc| npc.escort.is_some())
            .cloned()
            .collect()
    }

    /// The NPC numbered `id`.
    fn npc(&self, id: NpcId) -> Npc {
        self.session()
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("in the system")
            .clone()
    }

    /// Where system `id` is on flight's map.
    fn on_map(&self, id: i16) -> Point {
        let map = self
            .app
            .screen()
            .flight_view()
            .expect("flying")
            .course_map();
        let system = map
            .model()
            .system(nova_sim::SystemId(id))
            .expect("on the map");
        map.view().world_to_screen(system.position())
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

    /// Plots a course to Beta on flight's map, flies out and jumps there.
    fn jump_to_beta(&mut self) {
        self.tap(Key::Char('m'));
        let beta = self.on_map(129);
        self.click(beta);
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
}

#[test]
fn a_captured_escort_stays_beside_the_player_and_follows_it_through_a_jump() {
    let store = MemoryPilots::new();
    let mut game = boarded_with(&store, escort_data(&[128]), escort_interface(), |screen| {
        screen.with_behaviour(escorts_only())
    });
    let trader = game.session().npcs()[0].id;
    game.click(game.item(3));
    game.click(game.item(7));
    assert_eq!(game.showing(), Showing::Assignment);
    game.click(game.item(2));
    assert_eq!(game.showing(), Showing::Flight);
    assert!(game.session().is_escort(trader), "it joined where it was");
    for _ in 0..120 {
        game.frame();
    }
    let off = game.npc(trader).state.position - game.session().player().position;
    assert!(off.length() < 100.0, "beside the player: {off:?}");
    game.jump_to_beta();
    let escorts = game.escorts();
    assert_eq!(escorts.len(), 1, "it came along");
    assert_eq!(escorts[0].ship, nova_sim::ShipId(129));
    let off = escorts[0].state.position - game.session().player().position;
    assert!(
        off.length() < 100.0,
        "beside the player on arrival: {off:?}"
    );
}

#[test]
fn f_sends_the_escorts_at_the_pirate_targeted() {
    let store = MemoryPilots::new();
    let mut game = escorted(&store, &[129], &[130], |screen| screen);
    let pirate = game
        .session()
        .npcs()
        .iter()
        .find(|npc| npc.ship == nova_sim::ShipId(131))
        .expect("the pirate")
        .id;
    game.tap(Key::Tab);
    assert_eq!(game.session().target().map(|npc| npc.id), Some(pirate));
    game.tap(Key::Char('f'));
    let shown = run_texts(&game.frame());
    assert!(
        shown
            .iter()
            .any(|text| text == "New escort orders assigned:  All ships attacking target."),
        "{shown:?}"
    );
    let mut hit = false;
    for _ in 0..300 {
        game.frame();
        hit |= game.npc(pirate).reserves.shield.now < 30.0;
        if hit {
            break;
        }
    }
    assert!(hit, "the pirate's shield dropped");
}

#[test]
fn the_escort_menu_holds_the_warships_alone_and_c_recalls_them() {
    let store = MemoryPilots::new();
    let mut game = escorted(&store, &[], &[130, 129], |screen| screen);
    game.tap(Key::Char('e'));
    game.tap(Key::Char('4'));
    game.tap(Key::Char('v'));
    let shown = run_texts(&game.frame());
    assert!(
        shown
            .iter()
            .any(|text| text == "New escort orders assigned:  Warships holding position."),
        "{shown:?}"
    );
    let ship = |game: &Boarder, id| game.npc(id);
    let [warship, freighter] = [0, 1].map(|at| game.escorts()[at].id);
    assert_eq!(ship(&game, warship).ship, nova_sim::ShipId(130));
    let held = ship(&game, warship).state.position;
    game.key(Key::Up, true);
    for _ in 0..400 {
        game.frame();
    }
    game.key(Key::Up, false);
    let player = game.session().player().position;
    assert!(
        (ship(&game, warship).state.position - held).length() < 5.0,
        "the warship stayed put"
    );
    assert!((player - held).length() > 300.0, "the player flew on");
    assert!(
        (ship(&game, freighter).state.position - player).length() < 150.0,
        "the freighter followed"
    );
    game.tap(Key::Char('c'));
    let shown = run_texts(&game.frame());
    assert!(
        shown
            .iter()
            .any(|text| text == "New escort orders assigned:  Warships returning to formation."),
        "{shown:?}"
    );
    assert_eq!(
        game.pilot()
            .escorts()
            .iter()
            .map(|escort| escort.order)
            .collect::<Vec<_>>(),
        [None, None]
    );
}

#[test]
fn hailing_an_escort_offers_release_which_removes_it_from_the_saved_fleet() {
    let store = MemoryPilots::new();
    let mut game = escorted(&store, &[], &[129], |screen| screen);
    let escort = game.escorts()[0].id;
    game.key(Key::Alt, true);
    game.tap(Key::Tab);
    game.key(Key::Alt, false);
    assert_eq!(game.session().target().map(|npc| npc.id), Some(escort));
    game.tap(Key::Char('y'));
    assert_eq!(game.showing(), Showing::Comm);
    assert_eq!(game.reply(), "What can I do for you?");
    let shown = run_texts(&game.frame());
    let buttons: Vec<&String> = shown
        .iter()
        .filter(|text| {
            [
                "Greetings",
                "Request Assistance",
                "Beg For Mercy",
                "Release",
                "Close Channel",
            ]
            .contains(&text.as_str())
        })
        .collect();
    assert_eq!(buttons, ["Close Channel", "Release"]);
    game.tap(Key::Char('r'));
    assert_eq!(game.reply(), "Goodbye, captain.");
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    assert!(!game.session().is_escort(escort));
    let saved = nova_sim::save::decode(&store.text("Ada").expect("saved")).expect("a pilot");
    assert_eq!(saved.escorts(), []);
}

/// The escorts' standing-order rule the settings file holding `text`
/// chooses, read as `main` reads it.
fn saved_escort_orders(text: &str) -> RuleSource {
    let home = tempfile::tempdir().expect("a temporary directory");
    let path = home.path().join("settings.json");
    std::fs::write(&path, text).expect("writes");
    let mut store: Option<Box<dyn nova_audio::SettingsStore>> =
        Some(Box::new(nova_audio::FileSettings::new(&path)));
    let (rulebook, warnings) = nova::rulebook::game_rulebook(store.as_deref_mut());
    assert_eq!(warnings, Vec::<String>::new(), "{text}");
    rulebook.source_for(nova_sim::RuleKey::EscortOrders)
}

#[test]
fn standing_orders_are_saved_and_reset_on_reopening_unless_the_settings_keep_them() {
    for (text, order, label) in [
        ("{}", None, "Formation"),
        (
            r#"{"rule_overrides": {"escort_orders": "bible"}}"#,
            Some(EscortOrder::Defend),
            "Defend",
        ),
    ] {
        let source = saved_escort_orders(text);
        let store = MemoryPilots::new();
        let mut game = escorted(&store, &[], &[130], |screen| {
            screen.with_escort_orders(source)
        });
        game.tap(Key::Char('d'));
        let shown = run_texts(&game.frame());
        assert!(
            shown
                .iter()
                .any(|text| text == "New escort orders assigned:  All ships defending."),
            "{shown:?}"
        );
        // L requests clearance, a second L lands.
        game.tap(Key::Char('l'));
        game.tap(Key::Char('l'));
        assert_eq!(game.showing(), Showing::Spaceport, "landed");
        let saved = nova_sim::save::decode(&store.text("Ada").expect("saved")).expect("a pilot");
        assert_eq!(
            saved.escorts()[0].order,
            Some(EscortOrder::Defend),
            "{text}: saved either way"
        );

        let mut game = reopened(&store, &[], |screen| {
            screen
                .with_behaviour(escorts_only())
                .with_escort_orders(source)
        });
        game.open_pilot_to(Showing::Spaceport);
        game.tap(Key::Escape);
        assert_eq!(game.showing(), Showing::Flight, "took off");
        game.frame();
        game.frame();
        assert_eq!(game.escorts().len(), 1, "{text}");
        assert_eq!(game.pilot().escorts()[0].order, order, "{text}");
        game.tap(Key::Char('e'));
        let shown = run_texts(&game.frame());
        assert!(
            shown.iter().any(|shown| shown == label),
            "{text}: {shown:?}"
        );
        assert!(
            !shown.iter().any(|shown| shown
                == if order.is_some() {
                    "Formation"
                } else {
                    "Defend"
                }),
            "{text}: {shown:?}"
        );
    }
}

// Fighters.

/// How wide a fighter's sprite is drawn.
const FIGHTER: f32 = 5.0;

/// [`escort_data`]'s world, where the player's ship (128) carries a
/// fighter bay (`wëap` 130, "Bay": guidance 99, carrying ship 132,
/// reloading every 30 ticks, pushing its fighter out at 4 pixels a tick,
/// four a bay) with two fighters aboard (`oütf` 128, its rounds); the
/// "Fighter" (ship 132, `EscortType` 0, `InherentAI` 4: 20 shield, 30
/// armour and the gun, drawn from a sheet 5 across); and the pirates'
/// "Carrier" (`düde` 130, ship 133: a warship carrying a bay with two
/// fighters aboard).
fn fighter_data(alpha_dudes: &[i16]) -> Rc<GameData> {
    use nova_data::records::string_list::StrList;
    let carrier_of = |weapons: &[i16], items: &[(i16, i16)]| {
        let mut bytes = ship(30, 45, weapons, items);
        put_i16s(&mut bytes, 0x44, &[10]);
        bytes
    };
    let boarder = carrier_of(&[128, 130], &[(128, 2)]);
    let mut bay = weapon(30, 0, 132, 0, 8, 0x0002);
    put_i16s(&mut bay, 0x08, &[99, 400]);
    put_i16s(&mut bay, 0x6C, &[4]);
    let mut gun = weapon(10, 5, -1, 0, 8, 0);
    put_i16s(&mut gun, 0x06, &[10]);
    let mut fighter = ship(20, 30, &[129], &[]);
    put_i16s(&mut fighter, 0x42, &[4]);
    put_i16s(&mut fighter, 0x732, &[0]);
    let mut carrier = carrier_of(&[130], &[]);
    put_i16s(&mut carrier, 0x22, &[2]);
    put_i16s(&mut carrier, 0x42, &[3]);
    put_i16s(&mut carrier, 0x732, &[2]);
    let mut fighters = vec![0; Outfit::SIZE.expect("fixed")];
    put_i16s(&mut fighters, 0x06, &[3, 130, 9999]);
    put_i16s(&mut fighters, 0x3F0, &[100]);
    let mut fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Boarder"), &boarder)
        .resource(Ship::TYPE, 131, Some(b"Raider"), &ship(30, 45, &[], &[]))
        .resource(Ship::TYPE, 132, Some(b"Fighter"), &fighter)
        .resource(Ship::TYPE, 133, Some(b"Carrier"), &carrier)
        .resource(
            Weapon::TYPE,
            128,
            Some(b"Blaster"),
            &weapon(10, 10, -1, 0, 8, 0),
        )
        .resource(Weapon::TYPE, 129, Some(b"Gun"), &gun)
        .resource(Weapon::TYPE, 130, Some(b"Bay"), &bay)
        .resource(Outfit::TYPE, 128, Some(b"Fighters"), &fighters)
        .resource(Govt::TYPE, 137, Some(b"Pirates"), &hailed_govt(0, "Pirate"))
        .resource(Dude::TYPE, 129, Some(b"Pirates"), &dude_of(3, 137, 131))
        .resource(Dude::TYPE, 130, Some(b"Carriers"), &dude_of(3, 137, 133))
        .resource(
            System::TYPE,
            128,
            Some(b"Alpha"),
            &escort_system(0, &[129], Some(128), alpha_dudes),
        )
        .resource(
            System::TYPE,
            129,
            Some(b"Beta"),
            &escort_system(100, &[128], None, &[]),
        )
        .resource(Stellar::TYPE, 128, Some(b"Pad"), &landing_pad())
        .resource(Spin::TYPE, 1004, None, &spin(1000, 1))
        .resource(RLED, 1000, None, &sheet(1, 40))
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(Spin::TYPE, 3000, None, &spin(3000, 6))
        .resource(RLED, 3000, None, &sheet(36, 3))
        .resource(RLED, 2001, None, &sheet(36, 2))
        .resource(ShipAnim::TYPE, 132, None, &ship_anim(2002))
        .resource(RLED, 2002, None, &sheet(36, 5))
        .resource(StrList::TYPE, 4000, None, &str_list("Food"))
        .resource(StrList::TYPE, 4004, None, &str_list("75"))
        .resource(StrList::TYPE, 3000, None, &strings(&comm_strings()))
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &interface(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture());
    for id in [131, 133] {
        fork = fork.resource(ShipAnim::TYPE, id, None, &ship_anim(2001));
    }
    for id in [8511, 8514, 8515, 8516, 5001, 5002, 5003] {
        fork = fork.resource(PICT, id, None, &pict(30, 20, [40, 40, 40]));
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

/// Attacks the player as soon as it can, firing as any ship does: the
/// pirate carrier.
#[derive(Debug)]
struct Raider;

impl Behaviour for Raider {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Attack(nova_sim::ShipRef::Player)
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> nova_sim::Trigger {
        nova_sim::ai::fire::trigger(npc, around)
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<nova_sim::ShipRef> {
        npc.goal.attacking()
    }
}

/// Nova's AI with every warship raiding the player and the other AI
/// types idling.
fn raiders() -> Rc<dyn Behaviour> {
    let still: Rc<dyn Behaviour> = Rc::new(Still);
    let ai = [
        AiType::WimpyTrader,
        AiType::BraveTrader,
        AiType::Interceptor,
    ]
    .into_iter()
    .fold(NovaAi::default(), |ai, ai_type| {
        ai.with(ai_type, Rc::clone(&still))
    })
    .with(AiType::Warship, Rc::new(Raider));
    Rc::new(ai)
}

/// The app over [`fighter_data`] of `alpha_dudes` on the main menu, with
/// `store`'s pilots, its traffic placed 300 above the player facing
/// down, the traffic idling unless `router` says otherwise.
fn refly(
    store: &MemoryPilots,
    alpha_dudes: &[i16],
    router: impl FnOnce(AppScreen) -> AppScreen,
) -> Boarder {
    let setup: &[u32] = if alpha_dudes.is_empty() {
        &[]
    } else {
        &[6, 6, 0, 0, 750, 450, 180, 0]
    };
    let (_, chance) = scripted(setup);
    Boarder::opening_over(
        fighter_data(alpha_dudes),
        escort_interface(),
        store,
        chance,
        |screen| router(screen.with_behaviour(escorts_only())),
    )
}

/// [`refly`]'s app with a saved pilot, "Ada", resumed in flight from the
/// main menu. Two frames place the system's ships.
fn carrying(
    store: &MemoryPilots,
    alpha_dudes: &[i16],
    router: impl FnOnce(AppScreen) -> AppScreen,
) -> Boarder {
    let data = fighter_data(alpha_dudes);
    let pilot = Pilot::new(data.as_ref(), "Ada").expect("a pilot");
    PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>)
        .save(&pilot)
        .expect("saved");
    let mut game = refly(store, alpha_dudes, router);
    game.open_pilot();
    game.frame();
    game.frame();
    game
}

impl Boarder {
    /// The fighters out of the player's bays.
    fn fighters_out(&self) -> usize {
        self.pilot()
            .escorts()
            .iter()
            .filter(|escort| escort.carried)
            .count()
    }

    /// W, then Control held until a fighter is out; the frame then.
    fn launch(&mut self) -> Frame {
        self.tap(Key::Char('w'));
        let out = self.fighters_out();
        self.key(Key::Control, true);
        for _ in 0..120 {
            let frame = self.frame();
            if self.fighters_out() > out {
                self.key(Key::Control, false);
                return frame;
            }
        }
        panic!("no fighter launched");
    }

    /// Whether `ship`'s shield drops within `frames` frames.
    fn shield_drops(&mut self, ship: NpcId, frames: u32) -> bool {
        (0..frames).any(|_| {
            self.frame();
            self.npc(ship).reserves.shield.now < 30.0
        })
    }
}

/// The settings file holding `text`, read as `main` reads it: its
/// rulebook's source for `rule`.
fn saved_rule(text: &str, rule: nova_sim::RuleKey) -> RuleSource {
    let home = tempfile::tempdir().expect("a temporary directory");
    let path = home.path().join("settings.json");
    std::fs::write(&path, text).expect("writes");
    let mut store: Option<Box<dyn nova_audio::SettingsStore>> =
        Some(Box::new(nova_audio::FileSettings::new(&path)));
    let (rulebook, warnings) = nova::rulebook::game_rulebook(store.as_deref_mut());
    assert_eq!(warnings, Vec::<String>::new(), "{text}");
    rulebook.source_for(rule)
}

#[test]
fn w_and_control_launch_a_fighter_that_attacks_the_pirate_targeted_on_f() {
    let store = MemoryPilots::new();
    let mut game = carrying(&store, &[129], |screen| screen);
    let frame = game.frame();
    assert!(
        text_at(&frame, "Bay - 2").is_some(),
        "{:?}",
        run_texts(&frame)
    );
    let frame = game.launch();
    assert!(
        text_at(&frame, "Bay - 1").is_some(),
        "{:?}",
        run_texts(&frame)
    );
    let frame = game.frame();
    assert_eq!(sprites_of(&frame, FIGHTER).len(), 1, "the fighter drawn");
    let fighter = game.escorts()[0].clone();
    let off = fighter.state.position - game.session().player().position;
    assert!(off.length() < 50.0, "beside the player: {off:?}");
    let pirate = game
        .session()
        .npcs()
        .iter()
        .find(|npc| npc.ship == nova_sim::ShipId(131))
        .expect("the pirate")
        .id;
    game.tap(Key::Tab);
    assert_eq!(game.session().target().map(|npc| npc.id), Some(pirate));
    assert!(!game.shield_drops(pirate, 60), "no command yet");
    game.tap(Key::Char('f'));
    let shown = run_texts(&game.frame());
    assert!(
        shown
            .iter()
            .any(|text| text == "New escort orders assigned:  All ships attacking target."),
        "{shown:?}"
    );
    assert!(game.shield_drops(pirate, 300));
}

#[test]
fn by_the_settings_fighter_launch_a_fighter_attacks_the_target_at_once() {
    let source = saved_rule(
        r#"{"rule_overrides": {"fighter_launch": "bible"}}"#,
        nova_sim::RuleKey::FighterLaunch,
    );
    assert_eq!(source, RuleSource::Bible);
    let store = MemoryPilots::new();
    let mut game = carrying(&store, &[129], |screen| screen.with_fighter_launch(source));
    game.tap(Key::Tab);
    let pirate = game.session().target().map(|npc| npc.id).expect("targeted");
    game.launch();
    assert!(game.shield_drops(pirate, 300), "without F");
}

#[test]
fn option_c_brings_the_fighters_home_and_the_bay_counts_them_again() {
    let store = MemoryPilots::new();
    let mut game = carrying(&store, &[], |screen| screen);
    game.launch();
    game.launch();
    assert_eq!(game.fighters_out(), 2);
    assert!(text_at(&game.frame(), "Bay - 0").is_some());
    game.key(Key::Alt, true);
    game.tap(Key::Char('c'));
    game.key(Key::Alt, false);
    let shown = run_texts(&game.frame());
    assert!(
        shown
            .iter()
            .any(|text| text == "New escort orders assigned:  All ships returning to hangar."),
        "{shown:?}"
    );
    let home = (0..600).find(|_| {
        game.frame();
        game.fighters_out() == 0
    });
    assert!(home.is_some(), "{:?}", game.escorts());
    let frame = game.frame();
    assert_eq!(sprites_of(&frame, FIGHTER), [], "none drawn");
    assert!(
        text_at(&frame, "Bay - 2").is_some(),
        "{:?}",
        run_texts(&frame)
    );
}

#[test]
fn a_pirate_carrier_launches_fighters_that_take_the_players_shield_down() {
    let store = MemoryPilots::new();
    let mut game = carrying(&store, &[130], |screen| screen.with_behaviour(raiders()));
    let shield = game.session().reserves().shield.now;
    let mut drawn = false;
    let mut hit = false;
    for _ in 0..300 {
        let frame = game.frame();
        drawn |= !sprites_of(&frame, FIGHTER).is_empty();
        hit |= game.session().reserves().shield.now < shield;
    }
    assert!(
        game.session()
            .npcs()
            .iter()
            .any(|npc| npc.ship == nova_sim::ShipId(132)),
        "a fighter launched"
    );
    assert!(drawn, "and drawn");
    assert!(hit, "the player's shield dropped");
}

#[test]
fn a_fighter_out_is_saved_on_landing_and_flies_again_after_take_off() {
    let store = MemoryPilots::new();
    let mut game = carrying(&store, &[], |screen| screen);
    game.launch();
    // L requests clearance, a second L lands.
    game.tap(Key::Char('l'));
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Spaceport, "landed");
    let saved = nova_sim::save::decode(&store.text("Ada").expect("saved")).expect("a pilot");
    assert_eq!(saved.escorts().len(), 1);
    assert!(saved.escorts()[0].carried);
    let mut game = refly(&store, &[], |screen| screen);
    game.open_pilot_to(Showing::Spaceport);
    game.tap(Key::Escape);
    assert_eq!(game.showing(), Showing::Flight, "took off");
    game.frame();
    game.frame();
    let fighters = game.escorts();
    assert_eq!(fighters.len(), 1, "beside the player again");
    let off = fighters[0].state.position - game.session().player().position;
    assert!(off.length() < 100.0, "{off:?}");
    assert!(text_at(&game.frame(), "Bay - 1").is_some());
    game.key(Key::Alt, true);
    game.tap(Key::Char('c'));
    game.key(Key::Alt, false);
    assert!(
        (0..600).any(|_| {
            game.frame();
            game.fighters_out() == 0
        }),
        "docked"
    );
    assert!(text_at(&game.frame(), "Bay - 2").is_some());
}

#[test]
fn by_the_settings_fighter_recall_the_fighters_are_aboard_as_soon_as_the_pilot_lands() {
    let source = saved_rule(
        r#"{"rule_overrides": {"fighter_recall": "bible"}}"#,
        nova_sim::RuleKey::FighterRecall,
    );
    assert_eq!(source, RuleSource::Bible);
    let store = MemoryPilots::new();
    let mut game = carrying(&store, &[], |screen| screen.with_fighter_recall(source));
    game.launch();
    // L requests clearance, a second L lands.
    game.tap(Key::Char('l'));
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Spaceport, "landed");
    let saved = nova_sim::save::decode(&store.text("Ada").expect("saved")).expect("a pilot");
    assert_eq!(saved.escorts(), []);
    assert_eq!(saved.owned(nova_sim::OutfitId(128)), 2, "both aboard");
    let mut game = refly(&store, &[], |screen| screen.with_fighter_recall(source));
    game.open_pilot_to(Showing::Spaceport);
    game.tap(Key::Escape);
    game.frame();
    game.frame();
    assert_eq!(game.fighters_out(), 0, "no fighter out");
    assert!(text_at(&game.frame(), "Bay - 2").is_some());
}
