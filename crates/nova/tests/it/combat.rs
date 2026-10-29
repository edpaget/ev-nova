//! The app fighting over synthetic game data, wired to the renderer, the
//! recording Gpu and a recording audio port, and driven only by window
//! events: the player's ship carries a blaster; one trader is placed 100
//! pixels above it by the app's source of chance.
//!
//! - Tab reaches flight, and the target's name, from the game data,
//!   reaches the Gpu.
//! - A kill draws its debris on the router's effects chance, and its
//!   explosion reaches the Gpu and the audio port.
//! - The router's point-defence rule reaches the flight.
//! - The router's law judges the flight's crimes, and the law the
//!   settings file at the platform's settings path chooses decides them.
//! - Boarded, with a pilot opened from the main menu and saved in a memory
//!   store, and the interface file's plunder and assignment dialogs: B,
//!   Capture Ship and "Use As Escort" reach the dialogs and the fleet,
//!   which the saved pilot keeps and a new app opens again.
//! - A hail option registered at the edge reaches the comm dialog, drawn,
//!   and its press reaches the reply.
//! - Option-Tab and Y open the escort dialog, whose Release reaches the
//!   saved fleet; standing orders are saved on landing and reset or kept
//!   on reopening as the settings file's `escort_orders` says.
//! - `fighter_launch` and `fighter_recall`, read from the settings file,
//!   reach the fighters.
//!
//! What a key selects, how ships fight, fly, flee and follow, prices,
//! wording and sounds' priorities are nova-sim's, nova-view's and
//! nova-audio's, pinned there.

// Positions here are compared after the same arithmetic on both sides.
#![allow(clippy::float_cmp)]

use crate::bracket_art;
use crate::common::Frames;

use std::cell::RefCell;
use std::collections::VecDeque;
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
use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
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
use nova_data::store::fixture::{one_file, one_file_at};
use nova_data::{GameData, Record, SoundId};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, Rect, TextRun};
use nova_rsrc::ResType;
use nova_rsrc::fixture::ForkBuilder;
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
    let (ids, counts): (Vec<i16>, Vec<i16>) = items.iter().copied().unzip();
    ShipBuilder::new()
        .shield(shield)
        .accel(300)
        .speed(300)
        .maneuver(30)
        .fuel(300)
        .armor(armor)
        .weap_types(&[-1; 8])
        .weap_types(weapons)
        .weap_counts(&[1; 4])
        .explode1(-1)
        .inherent_ai(1)
        .default_items(&[-1; 8])
        .default_items(&ids)
        .item_counts(&counts)
        .build()
}

/// `bytes`, a `shïp`, of no `Speed`: setup starts it at rest, and it
/// never moves.
fn still(bytes: Vec<u8>) -> Vec<u8> {
    ShipBuilder::from_bytes(bytes).speed(0).build()
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

/// `oütf` 100 + `weapon`, which is one `wëap` `weapon` (`ModType` 1), up
/// to 10, never for sale: how the player's ship holds a stock weapon.
fn holder(weapon: i16) -> (i16, Vec<u8>) {
    let mut bytes = vec![0; Outfit::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x06, &[1, weapon, 10]);
    (100 + weapon, bytes)
}

/// `fork` with an [`holder`] for each of `weapons`.
fn holding(fork: ForkBuilder, weapons: &[i16]) -> ForkBuilder {
    weapons.iter().fold(fork, |fork, &weapon| {
        let (id, bytes) = holder(weapon);
        fork.resource(Outfit::TYPE, id, None, &bytes)
    })
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
    ShipAnimBuilder::new()
        .base_image_id(image)
        .base_set_count(1)
        .frames_per(36)
        .build()
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

/// The size of the explosion's frames.
const EXPLOSION: f32 = 5.0;

/// The first `chär` flies the "Gunship" (ship 128: a 1 x 1 sheet, a
/// blaster and a rocket launcher, and three rockets) from Alpha (128),
/// whose one `düde` flies the "Trader" (ship 129: a 2 x 2 sheet, no
/// shield, 10 armour and no speed, so it stays where setup places it).
/// The blaster (`wëap` 128) fires every tick, 5
/// mass damage a shot, its 3 x 3 shots sounding `snd ` 208; the rocket
/// launcher (`wëap` 138, a secondary) fires its 4 x 4 rockets every 30
/// ticks, each a round of its own. Explosion type 0 is 5 x 5 frames.
/// Each weapon the player's ship may carry is held by its [`holder`].
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
    let fork = holding(ForkBuilder::new(), &[128, 133, 134, 138, 155])
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
            &still(ship(0, 10, trader, &[])),
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
    Rc::new(one_file(bracket_art::with_bracket_art(fork).build().bytes).load())
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

/// The draws of a pilot opened from a save: its heading, `draws[0]`;
/// the offers' roll table, each the last outcome; then the rest of
/// `draws`.
fn resumed(draws: &[u32]) -> Vec<u32> {
    let table = std::iter::repeat_n(99, nova_sim::mission::ROLLED);
    draws[..1]
        .iter()
        .copied()
        .chain(table)
        .chain(draws[1..].iter().copied())
        .collect()
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
    /// the router as `router` makes it, drawn until the system's traffic
    /// is placed.
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
        harness.settle_until("the system's traffic", |harness| {
            harness
                .flight()
                .session()
                .is_ok_and(|session| !session.npcs().is_empty())
        });
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

    fn tap(&mut self, key: Key) {
        self.key(key, true);
        self.key(key, false);
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
        let frame = self.last_frame();
        assert_eq!(self.app.take_failures(), []);
        frame
    }

    /// The last frame the Gpu was given.
    fn last_frame(&self) -> Frame {
        (*self.gpu.submits().last().expect("a frame")).clone()
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

/// The text runs of `frame`.
fn run_texts(frame: &Frame) -> Vec<String> {
    runs(frame).into_iter().map(|run| run.text).collect()
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

#[test]
fn tab_reaches_flight_and_the_targets_name_reaches_the_gpu() {
    let mut harness = Harness::flying();
    assert_eq!(text_at(&harness.frame(), "Trader"), None, "no target yet");
    harness.tap(Key::Tab);
    harness.settle();
    assert!(harness.session().target().is_some(), "targeted");
    assert!(
        text_at(&harness.last_frame(), "Trader").is_some(),
        "the game data's name for it, drawn"
    );
}

#[test]
fn a_kill_rolls_debris_on_the_routers_effects_chance_and_its_explosion_is_drawn_and_heard() {
    let mut harness = Harness::flying();
    harness.key(Key::Space, true);
    harness.settle_until("the trader's destruction", |harness| {
        harness.session().npcs().is_empty()
    });
    harness.key(Key::Space, false);
    assert!(
        !sprites_of(&harness.last_frame(), EXPLOSION).is_empty(),
        "its explosion"
    );
    assert!(
        !harness.effects.borrow().asked.is_empty(),
        "its debris, on the effects' own chance"
    );
    assert!(
        harness.log.borrow().iter().any(|command| matches!(
            command,
            AudioCommand::Play {
                sound: SoundId(300),
                ..
            }
        )),
        "the explosion, heard: {:?}",
        harness.log.borrow()
    );
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
            bays: false,
            bays_after: false,
            point_defence: false,
            keep: None,
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

#[test]
fn the_routers_point_defence_rule_reaches_the_flight() {
    let rule = Rc::new(Unalarmed::default());
    let given: Rc<dyn PointDefenceRule> = rule.clone();
    let asking = nova_sim::Rulebook::default()
        .with_override(nova_sim::RuleKey::PdHostility, nova_sim::RuleSource::Bible);
    // The player carries the Quad, and the trader, 200 pixels above it,
    // fires the IR Missile at it.
    let mut harness = Harness::flying_over(
        data_arming(&[133], &[134]),
        &[6, 6, 0, 0, 750, 550, 180],
        |screen| {
            screen
                .with_behaviour(Rc::new(Attacking))
                .with_point_defence_rule(given)
                .with_rules(asking)
        },
    );
    harness.settle_until("the router's rule asked", |_| rule.asked.get() > 0);
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
    let fork = holding(ForkBuilder::new(), &[128])
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
    Rc::new(one_file(bracket_art::with_bracket_art(fork).build().bytes).load())
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

    fn system_penalties(
        &self,
        _crime: nova_sim::Crime,
        _victim: Option<nova_sim::GovtId>,
        _from: nova_sim::SystemId,
        _map: &dyn nova_sim::CrimeMap,
        _govts: &nova_sim::Governments,
    ) -> Vec<(nova_sim::SystemId, f32)> {
        Vec::new()
    }
}

#[test]
fn the_routers_law_judges_the_flights_crimes() {
    let witness = Rc::new(Witness::default());
    let law: Rc<dyn nova_sim::LegalCode> = witness.clone();
    // The trader 100 above, facing away: it flies on up the line of fire.
    let mut harness = Harness::flying_over(patrolled(), &[6, 6, 0, 0, 750, 650, 0, 0], |screen| {
        screen
            .with_behaviour(Rc::new(Still))
            .with_disable_rule(Rc::new(nova_sim::NovaDisable))
            .with_law(law)
    });
    harness.key(Key::Space, true);
    harness.settle_until("a crime the witness sees", |_| {
        !witness.seen.borrow().is_empty()
    });
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
        // The trader 100 above, facing away: it flies on up the line of
        // fire.
        let mut harness =
            Harness::flying_over(patrolled(), &[6, 6, 0, 0, 750, 650, 0, 0], |screen| {
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
        harness.settle_until("the traders' record lowered", |harness| {
            record(harness, 128) != 0
        });
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
/// a crew of 1, no shield, 40 armour, 20 holds, 300 fuel and a third of
/// the Boarder's speed, which it starts at and, disabled, drifts on at)
/// for food and money (`Booty` 0x0041); food is commodity 0. The status
/// bar, Nova's button pictures and the plunder and assignment dialogs'
/// pictures (`PICT` 8515 and 8516) are there.
fn boarding_data() -> Rc<GameData> {
    let boarder = ShipBuilder::from_bytes(ship(30, 45, &[128], &[]))
        .crew(10)
        .build();
    let trader = ShipBuilder::from_bytes(ship(0, 40, &[], &[]))
        .holds(20)
        .speed(100)
        .crew(1)
        .build();
    let mut dude = dude();
    put_i16s(&mut dude, 0x04, &[0x0041]);
    let mut fork = holding(ForkBuilder::new(), &[128])
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
    Rc::new(one_file(bracket_art::with_bracket_art(fork).build().bytes).load())
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
    let path = Path::new("/Nova-DF.rsrc");
    InterfaceData::load(&one_file_at(path, fork), path).expect("loads")
}

/// The load's heading (0, facing up), then the traffic's setup draws (the trader 100 above the player,
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
            setup: resumed(&[0, 6, 6, 0, 0, 750, 650, 180, 0]).into(),
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
            app: App::new(&FakeWindow, data, crate::common::appeared(screen)),
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
        self.last_frame()
    }

    /// The last frame the Gpu was given.
    fn last_frame(&self) -> Frame {
        (*self.gpu.submits().last().expect("a frame")).clone()
    }

    /// Draws frames until the dock approach has cleared the request.
    fn approach(&mut self) {
        self.settle_until("the dock approach's clearance", |game| {
            game.session().dock_cleared()
        });
    }

    /// Draws frames until the system's ships are placed.
    fn populated(&mut self) {
        self.settle_until("the system's ships", |game| {
            !game.session().npcs().is_empty()
        });
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
            .press_point(MenuChoice::OpenPilot);
        self.click(at);
        assert_eq!(self.showing(), Showing::OpenPilot);
        self.tap(Key::Enter);
        self.enter_ship();
        assert_eq!(self.showing(), showing);
    }

    /// The centre of the open plunder or assignment dialog's item.
    fn item(&self, item: usize) -> Point {
        let screen = self.app.screen();
        // The one in front: the rename, then the assignment, over the
        // plunder dialog kept under them.
        let dialog = match (
            screen.capture_naming(),
            screen.assignment(),
            screen.plunder(),
        ) {
            (Some(rename), _, _) => rename.dialog(),
            (None, Some(assignment), _) => assignment.dialog(),
            (None, None, Some(plunder)) => plunder.dialog(),
            (None, None, None) => panic!("no boarding dialog is open"),
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
    if error.length() > accel
        || distance > quarry.hull.board_reach.x.min(quarry.hull.board_reach.y) / 3.0
    {
        let (turn, side) = toward(heading_of(error));
        return nova_sim::Controls {
            thrust: turn.abs() < 10.0 && error.length() > accel / 2.0,
            turn: side,
            reverse: false,
            ..nova_sim::Controls::default()
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
/// (Tab), flies over it with the arrow keys and boards it (B), the trader
/// placed and the capture rolled as [`Boarder::opening`] rolls them.
fn boarded(store: &MemoryPilots) -> Boarder {
    let data = boarding_data();
    PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>)
        .save(&Pilot::new(data.as_ref(), "Ada").expect("a pilot"))
        .expect("saved");
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(BoardChance {
        setup: resumed(&[0, 6, 6, 0, 0, 750, 650, 180, 0]).into(),
        of_100: [99, 0].into(),
    }));
    let mut game = Boarder::opening_over(
        data,
        boarding_interface(),
        store,
        SharedChance::new(chance),
        |screen| screen.with_behaviour(Rc::new(Still)),
    );
    game.open_pilot();
    game.populated();
    let trader = game.session().npcs()[0].id;
    game.tap(Key::Tab);
    game.key(Key::Space, true);
    game.settle_until("the trader disabled", |game| {
        game.session().npcs()[0].condition == nova_sim::Condition::Disabled
    });
    game.key(Key::Space, false);
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

#[test]
fn b_and_capture_reach_the_boarding_dialogs_and_the_escort_is_saved_and_reopened() {
    let store = MemoryPilots::new();
    let mut game = boarded(&store);
    // The credits, then the capture: its press rolls the self-destruct.
    game.click(game.item(3));
    game.click(game.item(7));
    assert_eq!(game.showing(), Showing::Assignment, "captured");
    game.click(game.item(2));
    assert_eq!(game.showing(), Showing::Flight);
    let saved = nova_sim::save::decode(&store.text("Ada").expect("saved")).expect("a pilot");
    assert_eq!(saved.escorts().len(), 1, "the store's pilot keeps it");

    // A new app: Open Pilot resumes the pilot with its escort.
    let mut game = Boarder::opening(&store);
    game.open_pilot();
    assert_eq!(game.pilot().escorts().len(), 1);
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
    ShipBuilder::from_bytes(ship(shield, armor, weapons, &[]))
        .comm_name(name)
        .build()
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
    let file = one_file(
        bracket_art::with_bracket_art(hailing_strings(fork))
            .build()
            .bytes,
    );
    Rc::new(file.load())
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
    harness.tap(Key::Tab);
    harness.tap(Key::Char('y'));
    harness.settle();
    assert_eq!(harness.app.screen().showing(), Showing::Comm);
    assert!(
        text_at(&harness.last_frame(), "Weather").is_some(),
        "its button"
    );
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
    harness.settle();
    assert!(
        text_at(&harness.last_frame(), "Fine weather.").is_some(),
        "its reply"
    );
}

// Escorts.

use crate::common::EntryHarness;
use nova_sim::{AiType, EscortOrder, NovaAi, RuleKey, RuleSource};

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
/// (ship 129, a freighter escort, `EscortType` 3, at a third of the
/// Boarder's speed, carrying food and money), and 129 the pirates' (`gövt` 137) "Raider" (ship 131: 30
/// shield, 45 armour, unarmed). Ship 130 is the "Warship", a warship
/// escort (`EscortType` 2, `InherentAI` 3: 30 shield, 45 armour and a gun,
/// `wëap` 129, firing every 10 ticks for 5 mass and 10 energy damage). `STR#` 3000 and 3001 hold the replies, with "What can I do
/// for you?" (group 4) for an escort.
fn escort_data(alpha_dudes: &[i16]) -> Rc<GameData> {
    use nova_data::records::string_list::StrList;
    let boarder = ShipBuilder::from_bytes(ship(30, 45, &[128], &[]))
        .crew(10)
        .build();
    let trader = ShipBuilder::from_bytes(ship(0, 40, &[], &[]))
        .holds(20)
        .speed(100)
        .crew(1)
        .escort_type(3)
        .build();
    let mut gun = weapon(10, 5, -1, 0, 8, 0);
    put_i16s(&mut gun, 0x06, &[10]);
    let warship = ShipBuilder::from_bytes(ship(30, 45, &[129], &[]))
        .inherent_ai(3)
        .escort_type(2)
        .build();
    let mut traders = dude();
    put_i16s(&mut traders, 0x04, &[0x0041]);
    let mut replies = comm_strings();
    for variant in 0..5 {
        "What can I do for you?".clone_into(&mut replies[20 + variant]);
    }
    let mut fork = holding(ForkBuilder::new(), &[128])
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
    Rc::new(one_file(bracket_art::with_bracket_art(fork).build().bytes).load())
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
                "person": null,
                "mark": null
            })
        })
        .collect();
    nova_sim::save::decode(&save.to_string()).expect("a pilot")
}

/// The app over [`escort_data`] of `alpha_dudes` and [`escort_interface`],
/// with a saved pilot, "Ada", whose fleet is `fleet`, resumed in flight
/// from the main menu, its traffic placed 300 above the player facing
/// down; the escorts flying alone ([`escorts_only`]), the router as
/// `router` makes it on top, drawn until the system's ships are placed.
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
    game.populated();
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
        &[0]
    } else {
        &[0, 6, 6, 0, 0, 750, 450, 180, 0]
    };
    let (_, chance) = scripted(&resumed(setup));
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
}

/// The escort dialog takes the rulebook's `dialog_focus_ring` as it
/// opens: on by default, off under the engine's reading.
#[test]
fn the_escort_dialog_opens_with_the_rulebooks_focus_ring() {
    use nova_view::ui::FocusRing;
    for (rules, ring) in [
        (nova_sim::Rulebook::default(), FocusRing::Tab),
        (
            nova_sim::Rulebook::default()
                .with_override(nova_sim::RuleKey::DialogFocusRing, RuleSource::Engine),
            FocusRing::Off,
        ),
    ] {
        let store = MemoryPilots::new();
        let mut game = escorted(&store, &[], &[129], |screen| screen.with_rules(rules));
        game.key(Key::Alt, true);
        game.tap(Key::Tab);
        game.key(Key::Alt, false);
        game.tap(Key::Char('y'));
        assert_eq!(game.showing(), Showing::EscortDialog);
        assert_eq!(
            game.app
                .screen()
                .escort_dialog()
                .map(|dialog| dialog.dialog().focus_ring()),
            Some(ring)
        );
    }
}

#[test]
fn option_tab_and_y_reach_the_escort_dialog_and_its_release_reaches_the_saved_fleet() {
    let store = MemoryPilots::new();
    let mut game = escorted(&store, &[], &[129], |screen| screen);
    game.key(Key::Alt, true);
    game.tap(Key::Tab);
    game.key(Key::Alt, false);
    game.tap(Key::Char('y'));
    game.settle();
    assert_eq!(game.showing(), Showing::EscortDialog);
    game.tap(Key::Char('r'));
    game.settle();
    assert_eq!(game.showing(), Showing::Flight, "released");
    let saved = nova_sim::save::decode(&store.text("Ada").expect("saved")).expect("a pilot");
    assert_eq!(saved.escorts(), [], "the store's pilot lets it go");
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
            screen.with_rules(Rulebook::default().with_override(RuleKey::EscortOrders, source))
        });
        game.tap(Key::Char('d'));
        game.settle();
        let shown = run_texts(&game.last_frame());
        assert!(
            shown
                .iter()
                .any(|text| text == "New escort orders assigned:  All ships defending."),
            "{shown:?}"
        );
        // L requests clearance, a second L lands.
        game.tap(Key::Char('l'));
        game.approach();
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
                .with_rules(Rulebook::default().with_override(RuleKey::EscortOrders, source))
        });
        game.open_pilot_to(Showing::Flight);
        game.populated();
        assert_eq!(game.escorts().len(), 1, "{text}");
        assert_eq!(game.pilot().escorts()[0].order, order, "{text}");
        game.tap(Key::Char('e'));
        game.settle();
        let shown = run_texts(&game.last_frame());
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
        ShipBuilder::from_bytes(ship(30, 45, weapons, items))
            .crew(10)
            .build()
    };
    let boarder = carrier_of(&[128, 130], &[(128, 2)]);
    let mut bay = weapon(30, 0, 132, 0, 8, 0x0002);
    put_i16s(&mut bay, 0x08, &[99, 400]);
    put_i16s(&mut bay, 0x6C, &[4]);
    let mut gun = weapon(10, 5, -1, 0, 8, 0);
    put_i16s(&mut gun, 0x06, &[10]);
    let fighter = ShipBuilder::from_bytes(ship(20, 30, &[129], &[]))
        .inherent_ai(4)
        .escort_type(0)
        .build();
    let carrier = ShipBuilder::from_bytes(carrier_of(&[130], &[]))
        .ammo_loads(&[2])
        .inherent_ai(3)
        .escort_type(2)
        .build();
    let mut fighters = vec![0; Outfit::SIZE.expect("fixed")];
    put_i16s(&mut fighters, 0x06, &[3, 130, 9999]);
    put_i16s(&mut fighters, 0x3F0, &[100]);
    let mut fork = holding(ForkBuilder::new(), &[128, 130])
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
    Rc::new(one_file(bracket_art::with_bracket_art(fork).build().bytes).load())
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
        &[0]
    } else {
        &[0, 6, 6, 0, 0, 750, 450, 180, 0]
    };
    let (_, chance) = scripted(&resumed(setup));
    Boarder::opening_over(
        fighter_data(alpha_dudes),
        escort_interface(),
        store,
        chance,
        |screen| router(screen.with_behaviour(escorts_only())),
    )
}

/// [`refly`]'s app with a saved pilot, "Ada", resumed in flight from the
/// main menu.
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
fn by_the_settings_fighter_launch_a_fighter_attacks_the_target_at_once() {
    let source = saved_rule(
        r#"{"rule_overrides": {"fighter_launch": "bible"}}"#,
        nova_sim::RuleKey::FighterLaunch,
    );
    assert_eq!(source, RuleSource::Bible);
    let store = MemoryPilots::new();
    let mut game = carrying(&store, &[129], |screen| {
        screen.with_rules(Rulebook::default().with_override(RuleKey::FighterLaunch, source))
    });
    game.populated();
    game.tap(Key::Tab);
    game.settle();
    let pirate = game.session().target().map(|npc| npc.id).expect("targeted");
    game.launch();
    assert!(game.shield_drops(pirate, 300), "without F");
}

#[test]
fn by_the_settings_fighter_recall_the_fighters_are_aboard_as_soon_as_the_pilot_lands() {
    let source = saved_rule(
        r#"{"rule_overrides": {"fighter_recall": "bible"}}"#,
        nova_sim::RuleKey::FighterRecall,
    );
    assert_eq!(source, RuleSource::Bible);
    let store = MemoryPilots::new();
    let mut game = carrying(&store, &[], |screen| {
        screen.with_rules(Rulebook::default().with_override(RuleKey::FighterRecall, source))
    });
    game.launch();
    // L requests clearance, a second L lands.
    game.tap(Key::Char('l'));
    game.approach();
    game.tap(Key::Char('l'));
    assert_eq!(game.showing(), Showing::Spaceport, "landed");
    let saved = nova_sim::save::decode(&store.text("Ada").expect("saved")).expect("a pilot");
    assert_eq!(saved.escorts(), []);
    assert_eq!(saved.owned(nova_sim::OutfitId(128)), 2, "both aboard");
    let mut game = refly(&store, &[], |screen| {
        screen.with_rules(Rulebook::default().with_override(RuleKey::FighterRecall, source))
    });
    game.open_pilot_to(Showing::Flight);
    assert_eq!(game.fighters_out(), 0, "no fighter out");
    game.tap(Key::Char('w'));
    game.settle_until("W's secondary selected", |game| {
        game.session().secondary().is_some()
    });
    assert!(text_at(&game.last_frame(), "Bay - 2").is_some());
}

crate::common::entry_harness!(Boarder, click, frame);
crate::common::frames!(Harness, frame);
