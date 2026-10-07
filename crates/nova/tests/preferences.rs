//! The Preferences dialog through the whole app: synthetic game data (a
//! pilot flying over a planet that can be landed on, and Nova's button
//! pictures) and a synthetic interface file holding the stock "new prefs
//! dialog", laid out by the real glyphon metrics, drawn through the
//! renderer into the recording Gpu and driven only by window events. The
//! audio core plays into the recording audio port, and the settings are
//! kept in the in-memory store, so the test sees what plays and what is
//! saved, and "restarts" by reading the store back.

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

mod prefs_fixture;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova::platform;
use nova::settings::{GameSettings, SettingsKeeper, game_settings};
use nova_audio::recording::{AudioLog, MemorySettings, RecordingAudio};
use nova_audio::settings::level_volume;
use nova_audio::{
    Audio, AudioCommand, AudioCore, AudioSettings, SettingsStore, SoundTable, Volume,
};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record, SoundId};
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_render::{Batch, FontFaces, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};
use nova_view::geometry::Point;
use nova_view::ui::PrefsDialog;
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

fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> OneFile {
    let bytes = resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
            fork.resource(*ty, *id, None, data)
        })
        .build()
        .bytes;
    OneFile(bytes)
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

/// An `rlëD` of `frames` 1 x 1 frames.
fn sheet(frames: u16) -> Vec<u8> {
    (0..frames)
        .fold(RledBuilder::new(1, 1), |sheet, _| {
            sheet.frame(|f| f.line().pixels(&[0x0400]))
        })
        .build()
}

/// Alpha Prime's own landing sound.
const LANDING_SOUND: i16 = 10_032;

/// The original table's landing sound.
const LANDING_BEEP: i16 = 151;

/// The pilot flies ship 128 in Alpha (128), over Alpha Prime (128) at
/// (0, 0), which can be landed on; with Nova's button pictures.
fn game_data() -> Rc<GameData> {
    let mut character = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut character, 0x04, &[128, 128, -1, -1, -1]);
    let mut ship = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut ship, 0x02, &[30, 300, 300, 30, 300]);
    let mut anim = vec![0; ShipAnim::SIZE.expect("fixed")];
    put_i16s(&mut anim, 0x00, &[2000, 0, 1]);
    put_i16s(&mut anim, 0x34, &[36]);
    let mut system = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut system, 0x04, &[-1; 32]);
    put_i16s(&mut system, 0x24, &[128]);
    put_i16s(&mut system, 0x66, &[-1]);
    let mut stellar = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut stellar, 0x00, &[0, 0, 4]);
    stellar[0x06..0x0A].copy_from_slice(&0x01_u32.to_be_bytes());
    put_i16s(&mut stellar, 0x18, &[-1, LANDING_SOUND]);
    let mut spin = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut spin, 0x00, &[1000, -1, 0, 0, 1, 1]);
    let mut resources = vec![
        (Character::TYPE, 128, character),
        (Ship::TYPE, 128, ship),
        (ShipAnim::TYPE, 128, anim),
        (RLED, 2000, sheet(36)),
        (System::TYPE, 128, system),
        (Stellar::TYPE, 128, stellar),
        (Spin::TYPE, 1004, spin),
        (RLED, 1000, sheet(1)),
    ];
    for state in [7500, 7503, 7506] {
        resources.push((PICT, state, pict(13, 25, [200, 0, 0])));
        resources.push((PICT, state + 1, pict(2, 25, [0, 200, 0])));
        resources.push((PICT, state + 2, pict(13, 25, [0, 0, 200])));
        resources.push((PICT, state + 100, pict(13, 25, [0, 0, 0])));
        resources.push((PICT, state + 102, pict(13, 25, [0, 0, 0])));
    }
    let file = fork(&resources);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

/// The settings saved before the game starts: music on, the effects at
/// level 4 and the music at level 6.
fn seeded() -> MemorySettings {
    let store = MemorySettings::new();
    let (mut keeper, _) = SettingsKeeper::open(store.clone());
    keeper
        .change(GameSettings {
            audio: AudioSettings {
                sound: true,
                music: true,
                effects_volume: level_volume(4),
                music_volume: level_volume(6),
            },
            hyperspace_effects: true,
        })
        .expect("seeds");
    store
}

struct Harness {
    app: App<Rc<GameData>>,
    window: FakeWindow,
    gpu: RecordingGpu,
    log: AudioLog,
    frames: u64,
}

impl Harness {
    /// The game started over `store`, as `main` starts it: the settings
    /// read through it start the audio core and the router.
    fn new(store: &MemorySettings) -> Self {
        Self::with_table(store, SoundTable::ORIGINAL)
    }

    /// The game started over `store`, its audio core playing `table`.
    fn with_table(store: &MemorySettings, table: SoundTable) -> Self {
        let data = game_data();
        let (keeper, settings, warning) =
            game_settings(Some(Box::new(store.clone()) as Box<dyn SettingsStore>));
        assert_eq!(warning, None);
        let keeper = keeper.expect("a keeper");
        let screen = start_screen(Rc::clone(&data))
            .with_dialogs(
                Rc::new(prefs_fixture::interface()),
                Rc::new(GlyphonMetrics::new(&FontFaces::bundled())),
            )
            .with_prefs(settings.prefs());
        let audio = RecordingAudio::new();
        let log = audio.log();
        let core = AudioCore::with_table(Box::new(audio) as Box<dyn Audio>, table)
            .with_settings(settings.audio);
        let window = FakeWindow;
        Self {
            app: App::new(&window, data, screen)
                .with_audio(core)
                .with_settings(keeper),
            window,
            gpu: RecordingGpu::new(),
            log,
            frames: 0,
        }
    }

    fn send(&mut self, event: WindowEvent) {
        let control = self.app.handle(event, &mut self.window, &mut self.gpu);
        assert_eq!(control, Control::Continue, "{event:?}");
    }

    fn press(&mut self, key: Key) {
        for pressed in [true, false] {
            self.key(key, pressed);
        }
    }

    /// Presses or releases `key`.
    fn key(&mut self, key: Key, pressed: bool) {
        self.send(WindowEvent::Key {
            key,
            pressed,
            repeat: false,
        });
    }

    /// Presses and releases the physical key `code` through winit's
    /// translation, as the real window does.
    fn press_physical(&mut self, code: KeyCode) {
        for state in [ElementState::Pressed, ElementState::Released] {
            self.send(platform::key_event(PhysicalKey::Code(code), state, false));
        }
    }

    /// Presses L twice, as the player lands: the first requests clearance,
    /// the second lands.
    fn land(&mut self) {
        self.press_physical(KeyCode::KeyL);
        self.press_physical(KeyCode::KeyL);
    }

    /// Moves to the logical point `at`, then presses and releases there.
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

    fn dialog(&self) -> &PrefsDialog {
        self.app.screen().preferences().expect("open")
    }

    fn frame(&mut self) -> Frame {
        self.frames += 1;
        self.send(WindowEvent::Redraw {
            elapsed: Duration::from_millis(16 * self.frames),
        });
        assert_eq!(self.app.take_failures(), []);
        (*self.gpu.submits().last().expect("a frame")).clone()
    }

    /// Draws frames enough for flight to tick.
    fn fly(&mut self) {
        for _ in 0..4 {
            self.frame();
        }
    }

    /// The audio commands since the last call.
    fn played(&self) -> Vec<AudioCommand> {
        std::mem::take(&mut *self.log.borrow_mut())
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

fn solids(frame: &Frame) -> usize {
    frame
        .batches
        .iter()
        .map(|batch| match batch {
            Batch::Solid(quads) => quads.len(),
            _ => 0,
        })
        .sum()
}

#[test]
fn p_opens_the_preferences_and_their_changes_play_and_survive_a_restart() {
    let store = seeded();
    let mut harness = Harness::new(&store);
    harness.press_physical(KeyCode::KeyF);
    assert_eq!(harness.showing(), Showing::Flight);
    let flying = harness.frame();
    assert_eq!(
        harness.played(),
        [AudioCommand::StartMusic {
            volume: level_volume(6)
        }],
        "the seeded music volume"
    );

    harness.press_physical(KeyCode::KeyP);
    assert_eq!(harness.showing(), Showing::Preferences);
    let open = harness.frame();
    let shown = texts(&open);
    for label in [
        "Music",
        "Sound",
        "Sound Volume:",
        "Music Volume:",
        "4",
        "6",
        "OK",
        "Key Settings",
        "Smoke Trails",
    ] {
        assert!(shown.iter().any(|text| text == label), "{label}: {shown:?}");
    }
    assert!(
        solids(&open) > solids(&flying) + 40,
        "the backdrop, the boxes and the arrows: {} then {}",
        solids(&flying),
        solids(&open)
    );
    assert_eq!(harness.played(), [], "the music plays on");

    let music = harness.dialog().music().rect().center();
    harness.click(music);
    assert!(!harness.dialog().prefs().sound.music);
    let up = harness.dialog().effects_volume().rects().up.center();
    harness.click(up);
    assert_eq!(harness.dialog().prefs().sound.effects_level, 5);
    for _ in 0..5 {
        harness.press(Key::Tab);
    }
    harness.press(Key::Down);
    assert_eq!(harness.dialog().prefs().sound.music_level, 5);
    let changed = harness.frame();
    assert!(texts(&changed).iter().any(|text| text == "5"));
    harness.press(Key::Enter);
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(harness.played(), [AudioCommand::StopMusic]);

    // Later sounds play at the new effects volume.
    harness.land();
    assert_eq!(harness.showing(), Showing::Spaceport);
    let five = Volume::new(5.0 / 7.0);
    assert_eq!(
        harness.played(),
        [
            AudioCommand::Play {
                sound: SoundId(LANDING_BEEP),
                volume: five
            },
            AudioCommand::Play {
                sound: SoundId(LANDING_SOUND),
                volume: five
            },
        ]
    );
    assert_eq!(harness.app.take_warnings(), Vec::<String>::new());

    // A restart reads back exactly what was chosen.
    let (restarted, warning) = SettingsKeeper::open(store.clone());
    assert_eq!(warning, None);
    assert_eq!(
        restarted.settings().audio,
        AudioSettings {
            sound: true,
            music: false,
            effects_volume: five,
            music_volume: level_volume(5),
        }
    );
    let mut again = Harness::new(&store);
    again.press(Key::Char('p'));
    assert_eq!(
        again.dialog().prefs().sound,
        nova_view::SoundPrefs {
            sound: true,
            music: false,
            effects_level: 5,
            music_level: 5,
        }
    );
}

#[test]
fn a_failed_save_is_a_warning_and_the_change_still_plays() {
    let store = seeded();
    let mut harness = Harness::new(&store);
    harness.press(Key::Char('f'));
    harness.played();
    harness.press(Key::Char('p'));
    store.fail_writes(true);
    let music = harness.dialog().music().rect().center();
    harness.click(music);
    assert_eq!(harness.played(), [AudioCommand::StopMusic]);
    assert_eq!(
        harness.app.take_warnings(),
        ["nova: cannot save the settings: the disk is full".to_owned()]
    );
    let (unchanged, _) = SettingsKeeper::open(store.clone());
    assert!(
        unchanged.settings().audio.music,
        "the old settings stay saved"
    );
}

/// An engine sound, for a table that has one.
const ENGINE: i16 = 200;

#[test]
fn the_preferences_pause_flight_and_stop_the_engine_until_a_fresh_thrust() {
    let store = seeded();
    let table = SoundTable {
        engine: Some(SoundId(ENGINE)),
        ..SoundTable::ORIGINAL
    };
    let mut harness = Harness::with_table(&store, table);
    let effects = level_volume(4);
    let engine = AudioCommand::StartLoop {
        sound: SoundId(ENGINE),
        volume: effects,
    };
    harness.press(Key::Char('f'));
    harness.fly();
    harness.played();
    harness.key(Key::Up, true);
    harness.fly();
    assert_eq!(harness.played(), [engine], "thrusting");

    harness.press(Key::Char('p'));
    assert_eq!(harness.showing(), Showing::Preferences);
    harness.fly();
    assert_eq!(
        harness.played(),
        [AudioCommand::StopLoop],
        "flight is paused, the music plays on"
    );
    harness.key(Key::Up, false);
    harness.press(Key::Enter);
    assert_eq!(harness.showing(), Showing::Flight);
    harness.fly();
    assert_eq!(harness.played(), [], "no thrust on return");

    harness.key(Key::Up, true);
    harness.fly();
    assert_eq!(harness.played(), [engine], "a fresh thrust");
}

/// The settings saved in `store`, as JSON.
fn saved(store: &MemorySettings) -> serde_json::Value {
    serde_json::from_str(&store.text().expect("saved")).expect("JSON")
}

#[test]
fn hyperspace_effects_toggle_by_click_and_key_and_survive_a_restart() {
    let store = seeded();
    let mut harness = Harness::new(&store);
    harness.press(Key::Char('p'));
    assert_eq!(harness.showing(), Showing::Preferences);
    assert!(harness.dialog().hyperspace_effects().on(), "on at first");
    let open = harness.frame();
    assert!(texts(&open).iter().any(|text| text == "Hyperspace Effects"));

    // A click turns it off, and it is saved with the audio settings.
    let box_at = harness.dialog().hyperspace_effects().rect().center();
    harness.click(box_at);
    assert!(!harness.dialog().hyperspace_effects().on());
    assert_eq!(saved(&store)["hyperspace_effects"], false);
    let (kept, _) = SettingsKeeper::open(store.clone());
    assert_eq!(
        kept.settings().audio,
        AudioSettings {
            sound: true,
            music: true,
            effects_volume: level_volume(4),
            music_volume: level_volume(6),
        },
        "the seeded audio settings intact"
    );

    // An audio change keeps it.
    let music = harness.dialog().music().rect().center();
    harness.click(music);
    let json = saved(&store);
    assert_eq!(json["music"], false);
    assert_eq!(json["hyperspace_effects"], false);
    harness.press(Key::Enter);
    assert_ne!(harness.showing(), Showing::Preferences);
    assert_eq!(harness.app.take_warnings(), Vec::<String>::new());

    // After a restart it is still off, and the keyboard turns it back on.
    let mut again = Harness::new(&store);
    again.press(Key::Char('p'));
    assert!(
        !again.dialog().hyperspace_effects().on(),
        "off after a restart"
    );
    for _ in 0..3 {
        again.press(Key::Tab);
    }
    again.press(Key::Space);
    assert!(again.dialog().hyperspace_effects().on());
    let json = saved(&store);
    assert_eq!(json["hyperspace_effects"], true);
    assert_eq!(json["music"], false, "the audio change kept");
}
