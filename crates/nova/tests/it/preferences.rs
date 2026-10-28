//! The Preferences dialog through the whole app: synthetic game data (a
//! pilot flying over a planet that can be landed on, and Nova's button
//! pictures) and a synthetic interface file holding the stock "new prefs
//! dialog", laid out by the real glyphon metrics, drawn through the
//! renderer into the recording Gpu and driven only by window events. The
//! audio core plays into the recording audio port, and the settings are
//! kept in the in-memory store, so the test sees what plays and what is
//! saved, and "restarts" by reading the store back.

use std::rc::Rc;
use std::time::Duration;

use crate::prefs_fixture;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova::config::Os;
use nova::platform;
use nova::settings::{
    FullscreenSetting, GameSettings, ScalingSetting, SettingsKeeper, game_settings,
};
use nova_audio::recording::{AudioLog, MemorySettings, RecordingAudio};
use nova_audio::settings::{effects_level_volume, music_level_volume};
use nova_audio::{
    Audio, AudioCommand, AudioCore, AudioSettings, Priority, SettingsStore, SoundTable,
};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fixture::one_file;
use nova_data::{GameData, Record, SoundId};
use nova_render::WorldSpace;
use nova_render::recording::{GpuCall, RecordingGpu};
use nova_render::wgpu::GlyphonMetrics;
use nova_render::{Batch, FontFaces, Frame};
use nova_rsrc::ResType;
use nova_rsrc::fixture::ForkBuilder;
use nova_view::geometry::Point;
use nova_view::sound::{SimSound, UiSound};
use nova_view::text::TextMetrics;
use nova_view::ui::PrefsDialog;
use nova_view::ui::prefs::OK_ITEM;
use nova_view::ui::volume::TEXT_SIZE;
use nova_view::{
    DrawLayer, Font, FullscreenKind, Key, MonitorMode, MouseButton, Scaling, Upscaler, VideoMode,
};
use winit::event::ElementState;
use winit::keyboard::{KeyCode, PhysicalKey};

/// A 1024 x 768 window at scale 1, where window pixels are logical
/// units, on a monitor named [`MONITOR`] offering [`raw_modes`]; it
/// records each fullscreen request.
struct FakeWindow {
    fullscreen: Vec<Option<FullscreenKind>>,
}

impl WindowPort for FakeWindow {
    fn size_px(&self) -> (u32, u32) {
        (1024, 768)
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }

    fn request_redraw(&mut self) {}

    fn set_fullscreen(&mut self, fullscreen: Option<FullscreenKind>) {
        self.fullscreen.push(fullscreen);
    }

    fn video_modes(&self) -> Vec<MonitorMode> {
        raw_modes()
    }

    fn monitor_name(&self) -> Option<String> {
        Some(MONITOR.to_owned())
    }
}

const MONITOR: &str = "DELL U2720Q";

fn mode(width: u32, height: u32, refresh_millihertz: u32) -> VideoMode {
    VideoMode {
        width,
        height,
        refresh_millihertz,
    }
}

/// The monitor's modes as the platform lists them: unsorted, some at two
/// depths, some smaller than the game.
fn raw_modes() -> Vec<MonitorMode> {
    [
        (mode(1920, 1080, 60_000), 32),
        (mode(800, 600, 60_000), 32),
        (mode(2560, 1440, 144_000), 32),
        (mode(1920, 1080, 59_940), 32),
        (mode(1920, 1080, 60_000), 16),
        (mode(1280, 720, 60_000), 32),
        (mode(2560, 1440, 144_000), 24),
    ]
    .into_iter()
    .map(|(mode, bit_depth)| MonitorMode { mode, bit_depth })
    .collect()
}

fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> Vec<u8> {
    resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
            fork.resource(*ty, *id, None, data)
        })
        .build()
        .bytes
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

/// Alpha Prime's ambient sound, its `CustSndID`.
const LANDING_SOUND: i16 = 10_032;

/// The pilot flies ship 128 in Alpha (128), over Alpha Prime (128) at
/// (0, 0), which can be landed on; with Nova's button pictures and the
/// hover cursor (`spïn` 650).
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
    let mut cursor = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut cursor, 0x00, &[651, -1, 1, 1, 8, 1]);
    let mut resources = vec![
        (Character::TYPE, 128, character),
        (Ship::TYPE, 128, ship),
        (ShipAnim::TYPE, 128, anim),
        (RLED, 2000, sheet(36)),
        (System::TYPE, 128, system),
        (Stellar::TYPE, 128, stellar),
        (Spin::TYPE, 1004, spin),
        (RLED, 1000, sheet(1)),
        (Spin::TYPE, 650, cursor),
        (RLED, 651, sheet(8)),
    ];
    for state in [7500, 7503, 7506] {
        resources.push((PICT, state, pict(13, 25, [200, 0, 0])));
        resources.push((PICT, state + 1, pict(2, 25, [0, 200, 0])));
        resources.push((PICT, state + 2, pict(13, 25, [0, 0, 200])));
        resources.push((PICT, state + 100, pict(13, 25, [0, 0, 0])));
        resources.push((PICT, state + 102, pict(13, 25, [0, 0, 0])));
    }
    let file = fork(&resources);
    Rc::new(one_file(file).load())
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
                effects_volume: effects_level_volume(4),
                music_volume: music_level_volume(6),
            },
            hyperspace_effects: true,
            fullscreen: false,
            fullscreen_kind: FullscreenSetting::Borderless,
            quicktime_movies: true,
            map_borders: true,
            smoke_trails: true,
            weapon_effects: true,
            parallax_starfield: true,
            frame_upscaler: ScalingSetting::default(),
            nova_files: None,
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
    /// read through it start the audio core and the router. It runs as on
    /// Windows, whatever the host, so its monitor offers exclusive modes.
    fn new(store: &MemorySettings) -> Self {
        Self::with_table(store, SoundTable::ORIGINAL)
    }

    /// The game started over `store` as [`Harness::new`] starts it, but
    /// running on `os`.
    fn on_os(store: &MemorySettings, os: Os) -> Self {
        Self::build(store, SoundTable::ORIGINAL, false, os)
    }

    /// The game started over `store`, its audio core playing `table`.
    fn with_table(store: &MemorySettings, table: SoundTable) -> Self {
        Self::build(store, table, false, Os::Windows)
    }

    /// The game started over `store` on the main menu, where the music
    /// plays.
    fn on_main_menu(store: &MemorySettings) -> Self {
        Self::build(store, SoundTable::ORIGINAL, true, Os::Windows)
    }

    fn build(store: &MemorySettings, table: SoundTable, main_menu: bool, os: Os) -> Self {
        let data = game_data();
        let (keeper, settings, warning) =
            game_settings(Some(Box::new(store.clone()) as Box<dyn SettingsStore>));
        assert_eq!(warning, None);
        let keeper = keeper.expect("a keeper");
        let mut screen = start_screen(Rc::clone(&data));
        if main_menu {
            let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
            screen = screen.with_pilots(None, metrics);
        }
        let screen = screen
            .with_dialogs(
                Rc::new(prefs_fixture::interface()),
                Rc::new(GlyphonMetrics::new(&FontFaces::bundled())),
            )
            .with_prefs(settings.prefs());
        let audio = RecordingAudio::new();
        let log = audio.log();
        let core = AudioCore::with_table(Box::new(audio) as Box<dyn Audio>, table)
            .with_settings(settings.audio);
        let window = FakeWindow {
            fullscreen: Vec::new(),
        };
        Self {
            app: App::new(&window, data, crate::common::appeared(screen))
                .with_os(os)
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

    /// Presses Command+comma through winit's translation, as the player
    /// opens the Preferences in flight, where P is the player info.
    fn command_comma(&mut self) {
        let super_key = PhysicalKey::Code(KeyCode::SuperLeft);
        self.send(platform::key_event(super_key, ElementState::Pressed, false));
        self.press_physical(KeyCode::Comma);
        self.send(platform::key_event(
            super_key,
            ElementState::Released,
            false,
        ));
    }

    /// Presses L twice, as the player lands: the first requests clearance,
    /// a few frames let the dock approach clear it, and the second lands.
    fn land(&mut self) {
        self.press_physical(KeyCode::KeyL);
        self.approach();
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

    /// Sends four redraws, at least a tick of flight, for the dock
    /// approach to clear a request. The nav brackets' corners (`cicn`
    /// 10000-10003), which the press's selection draws, may be missing
    /// from the fixture, so the frames' image failures are not checked.
    fn approach(&mut self) {
        for _ in 0..4 {
            self.frames += 1;
            let elapsed = Duration::from_millis(16 * self.frames);
            self.send(WindowEvent::Redraw { elapsed });
        }
        let _ = self.app.take_failures();
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
fn command_comma_opens_the_preferences_and_their_changes_play_and_survive_a_restart() {
    let store = seeded();
    let mut harness = Harness::new(&store);
    harness.press_physical(KeyCode::KeyF);
    assert_eq!(harness.showing(), Showing::Flight);
    let flying = harness.frame();
    assert_eq!(harness.played(), [], "no music in flight");

    harness.command_comma();
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
    assert_eq!(harness.played(), []);

    let music = harness.dialog().music().rect().center();
    harness.click(music);
    assert!(!harness.dialog().prefs().sound.music);
    let up = harness.dialog().effects_volume().rects().up.center();
    harness.click(up);
    assert_eq!(harness.dialog().prefs().sound.effects_level, 5);
    // Past Weapon Effects, Smoke Trails, Parallax Starfield, the display
    // picker and the Scaling list to the music volume.
    for _ in 0..11 {
        harness.press(Key::Tab);
    }
    harness.press(Key::Down);
    assert_eq!(harness.dialog().prefs().sound.music_level, 5);
    let changed = harness.frame();
    assert!(texts(&changed).iter().any(|text| text == "5"));
    harness.press(Key::Enter);
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(harness.played(), [], "saved, with no music to stop");

    // Later sounds play at the new effects volume.
    harness.land();
    assert_eq!(harness.showing(), Showing::Spaceport);
    let five = effects_level_volume(5);
    let beep = SimSound::TrafficControl
        .cue()
        .expect("traffic control beeps");
    assert_eq!(
        harness.played(),
        [
            // Traffic control's greeting; an inhabited stellar's
            // touchdown is silent (`_PlayerLandOnStellar` @0x632be).
            AudioCommand::Play {
                sound: beep.sound,
                volume: five,
                priority: Priority::new(beep.priority),
            },
        ]
    );
    // No ambient sound follows: without the interface's "Spaceport"
    // dialog the spaceport can't be shown, and plays nothing.
    harness.frame();
    assert_eq!(harness.played(), []);
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
            music_volume: music_level_volume(5),
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

/// The real Preferences dialog over the main menu, where the music plays,
/// drives the audio core: the music starts at the seeded level, a new
/// music level changes its volume, and the Music checkbox off stops it.
#[test]
fn the_preferences_over_the_main_menu_set_the_music_playing_there() {
    let store = seeded();
    let mut harness = Harness::on_main_menu(&store);
    assert_eq!(harness.showing(), Showing::MainMenu);
    harness.frame();
    assert_eq!(
        harness.played(),
        [AudioCommand::StartMusic {
            volume: music_level_volume(6)
        }],
        "the seeded music level"
    );

    // A lower music level turns the music down.
    harness.press(Key::Char('p'));
    assert_eq!(harness.showing(), Showing::Preferences);
    // Past Weapon Effects, Smoke Trails, Parallax Starfield, the display
    // picker and the Scaling list to the music volume.
    for _ in 0..11 {
        harness.press(Key::Tab);
    }
    harness.press(Key::Down);
    assert_eq!(harness.dialog().prefs().sound.music_level, 5);
    harness.press(Key::Enter);
    assert_eq!(harness.showing(), Showing::MainMenu);
    harness.frame();
    let click = |sound: UiSound| {
        let cue = sound.cue().expect("a click plays");
        AudioCommand::Play {
            sound: cue.sound,
            volume: effects_level_volume(4),
            priority: Priority::new(cue.priority),
        }
    };
    assert_eq!(
        harness.played(),
        [
            click(UiSound::ButtonDown),
            click(UiSound::ButtonUp),
            AudioCommand::SetMusicVolume(music_level_volume(5))
        ],
        "P's keyed click, at the seeded effects level, then the new music level"
    );

    // The Music checkbox off stops it.
    harness.press(Key::Char('p'));
    let music = harness.dialog().music().rect().center();
    harness.click(music);
    assert!(!harness.dialog().prefs().sound.music);
    harness.press(Key::Enter);
    assert_eq!(harness.showing(), Showing::MainMenu);
    harness.frame();
    assert_eq!(
        harness.played(),
        [
            click(UiSound::ButtonDown),
            click(UiSound::ButtonUp),
            AudioCommand::StopMusic
        ]
    );
}

#[test]
fn a_failed_save_is_a_warning_and_the_change_still_applies() {
    let store = seeded();
    let mut harness = Harness::new(&store);
    harness.press(Key::Char('f'));
    harness.played();
    harness.command_comma();
    store.fail_writes(true);
    let music = harness.dialog().music().rect().center();
    harness.click(music);
    assert!(!harness.dialog().prefs().sound.music);
    assert_eq!(harness.played(), [], "no music in flight to stop");
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
    };
    let mut harness = Harness::with_table(&store, table);
    let effects = effects_level_volume(4);
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

    harness.command_comma();
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

#[test]
fn the_preferences_over_flight_are_drawn_in_the_overlay_over_its_world() {
    let store = seeded();
    let mut harness = Harness::new(&store);
    harness.press_physical(KeyCode::KeyF);
    assert_eq!(harness.showing(), Showing::Flight);
    harness.frame();
    harness.command_comma();
    assert_eq!(harness.showing(), Showing::Preferences);
    let frame = harness.frame();
    let texts_in = |layer: DrawLayer| -> Vec<String> {
        frame
            .in_layer(layer)
            .flat_map(|batch| match batch {
                Batch::Text(runs) => runs.iter().map(|run| run.text.clone()).collect(),
                _ => Vec::new(),
            })
            .collect()
    };
    assert!(
        frame.in_layer(DrawLayer::World).next().is_some(),
        "flight's world beneath"
    );
    assert!(texts_in(DrawLayer::World).is_empty());
    let overlay = texts_in(DrawLayer::Overlay);
    for label in ["Music", "OK", "Key Settings"] {
        assert!(
            overlay.iter().any(|text| text == label),
            "{label}: {overlay:?}"
        );
    }
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
            effects_volume: effects_level_volume(4),
            music_volume: music_level_volume(6),
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

#[test]
fn parallax_starfield_toggles_by_click_and_key_and_survives_a_restart() {
    let store = seeded();
    let mut harness = Harness::new(&store);
    harness.press(Key::Char('p'));
    assert!(harness.dialog().parallax_starfield().on(), "on at first");
    assert!(
        texts(&harness.frame())
            .iter()
            .any(|text| text == "Parallax Starfield")
    );
    let box_at = harness.dialog().parallax_starfield().rect().center();
    harness.click(box_at);
    assert!(!harness.dialog().parallax_starfield().on());
    let json = saved(&store);
    assert_eq!(json["parallax_starfield"], false);
    assert_eq!(json["music"], true, "the seeded audio settings intact");
    harness.press(Key::Enter);
    assert_eq!(harness.app.take_warnings(), Vec::<String>::new());

    let mut again = Harness::new(&store);
    again.press(Key::Char('p'));
    assert!(
        !again.dialog().parallax_starfield().on(),
        "off after a restart"
    );
    for _ in 0..7 {
        again.press(Key::Tab);
    }
    again.press(Key::Space);
    assert!(again.dialog().parallax_starfield().on());
    assert_eq!(saved(&store)["parallax_starfield"], true);
}

#[test]
fn quicktime_movies_toggle_by_click_and_key_and_survive_a_restart() {
    let store = seeded();
    let mut harness = Harness::new(&store);
    harness.press(Key::Char('p'));
    assert!(harness.dialog().quicktime_movies().on(), "on at first");
    assert!(
        texts(&harness.frame())
            .iter()
            .any(|text| text == "QuickTime Movies")
    );
    let box_at = harness.dialog().quicktime_movies().rect().center();
    harness.click(box_at);
    assert!(!harness.dialog().quicktime_movies().on());
    assert_eq!(saved(&store)["quicktime_movies"], false);
    assert_eq!(saved(&store)["hyperspace_effects"], true, "the rest kept");
    harness.press(Key::Enter);
    assert_eq!(harness.app.take_warnings(), Vec::<String>::new());

    let mut again = Harness::new(&store);
    again.press(Key::Char('p'));
    assert!(
        !again.dialog().quicktime_movies().on(),
        "off after a restart"
    );
    for _ in 0..4 {
        again.press(Key::Tab);
    }
    again.press(Key::Space);
    assert!(again.dialog().quicktime_movies().on());
    assert_eq!(saved(&store)["quicktime_movies"], true);
}

/// The flight view under the harness.
fn flight(harness: &Harness) -> &nova_view::flight::FlightView<Rc<GameData>> {
    harness.app.screen().flight_view().expect("flying")
}

#[test]
fn smoke_trails_and_weapon_effects_clicked_off_in_flight_are_saved_and_the_glow_waits_for_a_restart()
 {
    let store = seeded();
    let mut harness = Harness::new(&store);
    harness.press(Key::Char('f'));
    harness.fly();
    assert!(flight(&harness).smoke_trails() && flight(&harness).weapon_effects());
    harness.command_comma();
    assert_eq!(harness.showing(), Showing::Preferences);
    let smoke = harness.dialog().smoke_trails().rect().center();
    harness.click(smoke);
    assert!(!flight(&harness).smoke_trails(), "at once");
    assert_eq!(saved(&store)["smoke_trails"], false);
    let weapons = harness.dialog().weapon_effects().rect().center();
    harness.click(weapons);
    assert_eq!(saved(&store)["weapon_effects"], false);
    assert_eq!(saved(&store)["smoke_trails"], false, "the rest kept");
    assert!(harness.dialog().relaunch_notice_shown());
    assert!(
        texts(&harness.frame())
            .iter()
            .any(|text| text.starts_with("(Changes will")),
        "the notice is drawn"
    );
    harness.press(Key::Enter);
    assert_eq!(harness.showing(), Showing::Flight);
    assert!(
        flight(&harness).weapon_effects(),
        "the running game keeps the glow until the next launch"
    );
    assert_eq!(harness.app.take_warnings(), Vec::<String>::new());

    let mut again = Harness::new(&store);
    again.press(Key::Char('f'));
    again.fly();
    assert!(!flight(&again).weapon_effects(), "off after a restart");
    assert!(!flight(&again).smoke_trails());
}

/// Where flight draws its hover cursor.
fn cursor(harness: &Harness) -> Option<Point> {
    harness
        .app
        .screen()
        .flight_view()
        .expect("flying")
        .cursor_at()
}

#[test]
fn the_cursor_comes_back_under_the_pointer_when_the_preferences_close() {
    let store = seeded();
    let mut harness = Harness::new(&store);
    harness.press(Key::Char('f'));
    let a = Point::new(100.0, 600.0);
    harness.send(WindowEvent::PointerMoved {
        px: (f64::from(a.x), f64::from(a.y)),
    });
    harness.frame();
    assert_eq!(cursor(&harness), Some(a));

    harness.command_comma();
    assert_eq!(harness.showing(), Showing::Preferences);
    harness.frame();
    let ok = harness
        .dialog()
        .dialog()
        .item_bounds(nova_view::ui::prefs::OK_ITEM)
        .expect("an OK button")
        .center();
    harness.click(ok);
    assert_eq!(harness.showing(), Showing::Flight);
    harness.frame();
    assert_eq!(cursor(&harness), Some(ok), "under the pointer, not at A");
}

#[test]
fn the_picker_lists_the_monitors_modes_and_ok_enters_the_one_picked_and_saves_it() {
    let store = seeded();
    let mut harness = Harness::new(&store);
    harness.command_comma();
    assert_eq!(harness.showing(), Showing::Preferences);
    assert!(
        texts(&harness.frame())
            .iter()
            .any(|text| text == "Borderless")
    );
    assert_eq!(
        harness.dialog().display().entries(),
        [
            FullscreenKind::Borderless,
            FullscreenKind::Exclusive(mode(2560, 1440, 144_000)),
            FullscreenKind::Exclusive(mode(1920, 1080, 60_000)),
            FullscreenKind::Exclusive(mode(1920, 1080, 59_940)),
        ],
        "at least 1024x768, largest first, each once"
    );

    // Down on the focused picker, then a click on its down arrow.
    for _ in 0..8 {
        harness.press(Key::Tab);
    }
    harness.press(Key::Down);
    assert!(
        texts(&harness.frame())
            .iter()
            .any(|text| text == "2560 x 1440, 144 Hz")
    );
    let down = harness.dialog().display().rects().down.center();
    harness.click(down);
    assert!(
        texts(&harness.frame())
            .iter()
            .any(|text| text == "1920 x 1080, 60 Hz")
    );
    assert_eq!(harness.window.fullscreen, [], "nothing while it is open");
    assert_eq!(saved(&store)["fullscreen_kind"], "borderless");

    let ok = harness
        .dialog()
        .dialog()
        .item_bounds(OK_ITEM)
        .expect("OK")
        .center();
    harness.click(ok);
    assert_ne!(harness.showing(), Showing::Preferences, "closed");
    assert_eq!(
        harness.window.fullscreen,
        [Some(FullscreenKind::Exclusive(mode(1920, 1080, 60_000)))]
    );
    assert_eq!(harness.app.take_warnings(), Vec::<String>::new());
    let (restarted, warning) = SettingsKeeper::open(store.clone());
    assert_eq!(warning, None);
    assert!(restarted.settings().fullscreen);
    assert_eq!(
        restarted.settings().fullscreen_kind,
        FullscreenSetting::Exclusive {
            mode: mode(1920, 1080, 60_000),
            monitor: Some(MONITOR.to_owned()),
        }
    );
    assert!(restarted.settings().audio.music, "the rest kept");

    // Reopened, the picker starts on the mode in use.
    harness.command_comma();
    assert_eq!(
        harness.dialog().display().selected(),
        FullscreenKind::Exclusive(mode(1920, 1080, 60_000))
    );
}

/// The settings `seeded` saves, with the game fullscreen in 1920x1080 at
/// 60 Hz chosen on another monitor, which [`MONITOR`] also lists.
fn seeded_on_another_monitor() -> MemorySettings {
    let store = seeded();
    let (mut keeper, _) = SettingsKeeper::open(store.clone());
    let settings = GameSettings {
        fullscreen: true,
        fullscreen_kind: FullscreenSetting::Exclusive {
            mode: mode(1920, 1080, 60_000),
            monitor: Some("LG HDR 4K".to_owned()),
        },
        ..keeper.settings()
    };
    keeper.change(settings).expect("seeds");
    store
}

impl Harness {
    /// The game started over `store` as the runner starts it, its window
    /// and Scaling restored on a device offering native alone, its startup
    /// warnings taken.
    fn launched(store: &MemorySettings) -> (Self, Vec<String>) {
        Self::launched_probing(store, &[Upscaler::Native])
    }

    /// [`Harness::launched`] on a device whose probe answers `probe`.
    fn launched_probing(store: &MemorySettings, probe: &[Upscaler]) -> (Self, Vec<String>) {
        let mut harness = Self::new(store);
        harness.gpu = RecordingGpu::probing(probe);
        harness.app.restore_window(&mut harness.window);
        harness.app.restore_scaling(&mut harness.gpu);
        let warnings = harness.app.take_warnings();
        (harness, warnings)
    }

    /// Opens the Preferences, steps the display picker by `steps` (Down
    /// for each `true`, Up for each `false`) and presses Return.
    fn pick(&mut self, steps: &[bool]) {
        self.command_comma();
        for _ in 0..8 {
            self.press(Key::Tab);
        }
        for &down in steps {
            self.press(if down { Key::Down } else { Key::Up });
        }
        self.press(Key::Enter);
        assert_ne!(self.showing(), Showing::Preferences, "closed");
    }
}

#[test]
fn after_a_fallback_launch_the_stock_dialog_saves_borderless_or_the_mode_picked() {
    let fhd = Some(FullscreenKind::Exclusive(mode(1920, 1080, 60_000)));
    let borderless = Some(FullscreenKind::Borderless);

    // Closed untouched: the saved kind stays.
    let store = seeded_on_another_monitor();
    let (mut harness, warnings) = Harness::launched(&store);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("1920 x 1080, 60 Hz"), "{warnings:?}");
    assert_eq!(harness.window.fullscreen, [borderless]);
    harness.command_comma();
    assert_eq!(
        harness.dialog().display().selected(),
        FullscreenKind::Borderless,
        "what runs"
    );
    harness.press(Key::Enter);
    assert_eq!(harness.window.fullscreen, [borderless]);
    assert_eq!(
        saved(&store)["fullscreen_kind"]["exclusive"]["monitor"],
        "LG HDR 4K"
    );

    // Borderless, picked: saved, and the next launch is quiet.
    let store = seeded_on_another_monitor();
    let (mut harness, _) = Harness::launched(&store);
    harness.pick(&[true, false]);
    assert_eq!(saved(&store)["fullscreen_kind"], "borderless");
    assert_eq!(saved(&store)["fullscreen"], true);
    assert_eq!(harness.app.take_warnings(), Vec::<String>::new());
    let (_, warnings) = Harness::launched(&store);
    assert_eq!(warnings, Vec::<String>::new(), "quiet");

    // The saved mode, picked: entered on this monitor and saved with its
    // name.
    let store = seeded_on_another_monitor();
    let (mut harness, _) = Harness::launched(&store);
    harness.pick(&[true, true]);
    assert_eq!(harness.window.fullscreen, [borderless, fhd]);
    assert_eq!(
        saved(&store)["fullscreen_kind"]["exclusive"]["monitor"],
        MONITOR
    );
    assert_eq!(harness.app.take_warnings(), Vec::<String>::new());
    let (relaunched, warnings) = Harness::launched(&store);
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(relaunched.window.fullscreen, [fhd], "reopens in the mode");
}

#[test]
fn on_macos_a_saved_exclusive_mode_opens_native_fullscreen_and_the_picker_offers_only_it() {
    let store = seeded();
    let (mut keeper, _) = SettingsKeeper::open(store.clone());
    let saved_kind = FullscreenSetting::Exclusive {
        mode: mode(1920, 1080, 60_000),
        monitor: Some(MONITOR.to_owned()),
    };
    let settings = GameSettings {
        fullscreen: true,
        fullscreen_kind: saved_kind.clone(),
        ..keeper.settings()
    };
    keeper.change(settings).expect("seeds");
    let mut harness = Harness::on_os(&store, Os::MacOs);
    harness.app.restore_window(&mut harness.window);
    assert_eq!(harness.app.take_warnings(), Vec::<String>::new(), "quiet");
    assert_eq!(
        harness.window.fullscreen,
        [Some(FullscreenKind::Borderless)],
        "the native Space"
    );
    harness.command_comma();
    assert_eq!(harness.showing(), Showing::Preferences);
    assert_eq!(
        harness.dialog().display().entries(),
        [FullscreenKind::Borderless]
    );
    assert_eq!(
        harness.dialog().display().selected(),
        FullscreenKind::Borderless
    );
    harness.press(Key::Enter);
    assert_eq!(
        harness.window.fullscreen,
        [Some(FullscreenKind::Borderless)]
    );
    let (kept, _) = SettingsKeeper::open(store.clone());
    assert_eq!(kept.settings().fullscreen_kind, saved_kind, "not migrated");
}

const NATIVE: Upscaler = Upscaler::Native;
const SHARP: Upscaler = Upscaler::SharpBilinear;
const METAL_FX: Upscaler = Upscaler::MetalFxSpatial;

/// Flight's world in a texture of the 1024 x 768 window's logical size.
const TEXTURE: WorldSpace = WorldSpace::Texture { size: (1024, 768) };

/// The settings `seeded` saves, with the Scaling saved as `name` under
/// phase 6's `frame_upscaler` key.
fn seeded_with_upscaler(name: &str) -> MemorySettings {
    let store = seeded();
    let (mut keeper, _) = SettingsKeeper::open(store.clone());
    let settings = GameSettings {
        frame_upscaler: ScalingSetting::from(name.to_owned()),
        ..keeper.settings()
    };
    keeper.change(settings).expect("seeds");
    store
}

impl Harness {
    /// Opens the Preferences, tabs to the Scaling list and steps it down
    /// `downs` times.
    fn step_scaling(&mut self, downs: usize) {
        self.step_scaling_by(Key::Down, downs);
    }

    /// Opens the Preferences, tabs to the Scaling list and presses `key`
    /// `times` times.
    fn step_scaling_by(&mut self, key: Key, times: usize) {
        self.command_comma();
        assert_eq!(self.showing(), Showing::Preferences);
        for _ in 0..9 {
            self.press(Key::Tab);
        }
        for _ in 0..times {
            self.press(key);
        }
    }

    /// The GPU calls drawing the next frame, the frame's own and no older.
    fn frame_calls(&mut self) -> Vec<GpuCall> {
        self.gpu.take();
        self.frame();
        self.gpu.take()
    }

    /// Clicks the open Preferences dialog's OK.
    fn ok(&mut self) {
        let ok = self
            .dialog()
            .dialog()
            .item_bounds(OK_ITEM)
            .expect("OK")
            .center();
        self.click(ok);
        assert_ne!(self.showing(), Showing::Preferences, "closed");
    }
}

#[test]
fn the_scaling_list_is_native_the_probes_scalers_then_super_xbr() {
    use Scaling::{MetalFxSpatial, Native, SharpBilinear, SuperXbr};
    let store = seeded();
    let (mut native_only, _) = Harness::launched_probing(&store, &[NATIVE]);
    native_only.command_comma();
    assert_eq!(native_only.dialog().scaling().entries(), [Native, SuperXbr]);
    assert!(
        texts(&native_only.frame())
            .iter()
            .any(|text| text == "Scaling: Native")
    );
    native_only.press(Key::Escape);
    native_only.step_scaling(1);
    assert!(
        texts(&native_only.frame())
            .iter()
            .any(|text| text == "Scaling: Super-xBR"),
        "drawn when selected"
    );
    let (mut both, _) = Harness::launched_probing(&store, &[SHARP, NATIVE]);
    both.command_comma();
    assert_eq!(
        both.dialog().scaling().entries(),
        [Native, SharpBilinear, SuperXbr]
    );
    assert_eq!(both.dialog().scaling().selected(), Native);
    let (mut all, _) = Harness::launched_probing(&store, &[SHARP, METAL_FX]);
    all.command_comma();
    assert_eq!(
        all.dialog().scaling().entries(),
        [Native, SharpBilinear, MetalFxSpatial, SuperXbr]
    );
}

#[test]
fn the_widest_scaling_label_fits_its_row() {
    let store = seeded();
    let (mut harness, _) = Harness::launched_probing(&store, &[SHARP]);
    harness.command_comma();
    let row = harness.dialog().scaling().rects().value;
    let metrics = GlyphonMetrics::new(&FontFaces::bundled());
    for scaling in Scaling::ALL {
        let width = metrics.width(Font::Geneva, TEXT_SIZE, scaling.label());
        assert!(
            width <= row.width(),
            "{scaling:?}: {width} in {}",
            row.width()
        );
    }
}

/// The sizes of the images `calls` upload, in order.
fn uploads(calls: &[GpuCall]) -> Vec<(u32, u32)> {
    calls
        .iter()
        .filter_map(|call| match call {
            GpuCall::Upload { width, height, .. } => Some((*width, *height)),
            _ => None,
        })
        .collect()
}

/// The places of the world's textured quads in `calls`' frame.
fn world_quads(calls: &[GpuCall]) -> Vec<nova_render::Rect> {
    let Some(GpuCall::Submit(frame)) = calls.last() else {
        panic!("not a frame: {calls:?}");
    };
    frame
        .world
        .iter()
        .flat_map(|&index| match &frame.batches[index] {
            Batch::Sprites { quads, .. } => quads.iter().map(|quad| quad.dest).collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// The world space of `calls`' frame.
fn world_space(calls: &[GpuCall]) -> WorldSpace {
    let Some(GpuCall::Submit(frame)) = calls.last() else {
        panic!("not a frame: {calls:?}");
    };
    frame.world_space
}

/// Whether `calls` upload the fixture's 1 x 1 sprite frames enlarged (2 x
/// 2) and none at their own size.
fn uploads_enlarged(calls: &[GpuCall]) -> bool {
    let sizes = uploads(calls);
    sizes.contains(&(2, 2)) && !sizes.contains(&(1, 1))
}

/// Whether `calls` upload the fixture's 1 x 1 sprite frames at their own
/// size and none enlarged.
fn uploads_native(calls: &[GpuCall]) -> bool {
    let sizes = uploads(calls);
    sizes.contains(&(1, 1)) && !sizes.contains(&(2, 2))
}

#[test]
fn choosing_super_xbr_in_flight_or_leaving_it_rebuilds_the_atlas_when_the_dialog_closes() {
    let store = seeded();
    let (mut harness, warnings) = Harness::launched_probing(&store, &[NATIVE]);
    assert_eq!(warnings, Vec::<String>::new());
    harness.press_physical(KeyCode::KeyF);
    assert_eq!(harness.showing(), Showing::Flight);
    let before = harness.frame_calls();
    assert!(uploads_native(&before), "{:?}", uploads(&before));

    harness.step_scaling(1);
    assert_eq!(harness.dialog().scaling().selected(), Scaling::SuperXbr);
    for _ in 0..2 {
        let open = harness.frame_calls();
        assert!(!open.contains(&GpuCall::ClearPages), "held while open");
        assert!(!uploads(&open).contains(&(2, 2)), "native while open");
        assert_eq!(world_space(&open), WorldSpace::Target);
    }
    assert_eq!(saved(&store)["frame_upscaler"], "native");

    harness.ok();
    // The cursor sprite is in the world now: take it away from the OK
    // button, where the click left it, so the world is as before.
    harness.send(WindowEvent::PointerLeft);
    let after = harness.frame_calls();
    assert_eq!(after[0], GpuCall::ClearPages, "the next frame rebuilds");
    assert!(uploads_enlarged(&after), "{:?}", uploads(&after));
    assert_eq!(world_quads(&after), world_quads(&before), "the same places");
    assert_eq!(world_space(&after), WorldSpace::Target);
    assert_eq!(saved(&store)["frame_upscaler"], "super_xbr");
    assert_eq!(harness.app.take_warnings(), Vec::<String>::new());

    harness.step_scaling_by(Key::Up, 1);
    harness.ok();
    let back = harness.frame_calls();
    assert_eq!(back[0], GpuCall::ClearPages, "leaving it rebuilds too");
    assert!(uploads_native(&back), "{:?}", uploads(&back));
    assert_eq!(saved(&store)["frame_upscaler"], "native");
}

#[test]
fn super_xbr_draws_with_no_upscaler_and_a_per_frame_scaler_with_native_sprites() {
    let store = seeded_with_upscaler("super_xbr");
    let (mut harness, warnings) = Harness::launched_probing(&store, &[SHARP]);
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(harness.gpu.upscaler(), NATIVE);
    harness.press_physical(KeyCode::KeyF);
    let enlarged = harness.frame_calls();
    assert_eq!(world_space(&enlarged), WorldSpace::Target);
    assert!(uploads_enlarged(&enlarged), "{:?}", uploads(&enlarged));

    harness.step_scaling_by(Key::Up, 1);
    assert_eq!(
        harness.dialog().scaling().selected(),
        Scaling::SharpBilinear
    );
    harness.ok();
    let sharp = harness.frame_calls();
    assert_eq!(world_space(&sharp), TEXTURE);
    assert_eq!(sharp[0], GpuCall::ClearPages);
    assert!(uploads_native(&sharp), "{:?}", uploads(&sharp));
    assert_eq!(harness.gpu.upscaler(), SHARP);
    for _ in 0..3 {
        let later = harness.frame_calls();
        assert!(!later.contains(&GpuCall::ClearPages), "no more rebuilds");
    }
}

#[test]
fn super_xbr_picked_is_saved_and_still_enlarges_after_a_restart() {
    let store = seeded();
    let (mut harness, _) = Harness::launched(&store);
    harness.step_scaling(1);
    harness.press(Key::Enter);
    assert_eq!(saved(&store)["frame_upscaler"], "super_xbr");
    assert_eq!(saved(&store)["music"], true, "the rest kept");

    let (mut restarted, warnings) = Harness::launched(&store);
    assert_eq!(warnings, Vec::<String>::new());
    restarted.press_physical(KeyCode::KeyF);
    let first = restarted.frame_calls();
    assert!(uploads_enlarged(&first), "{:?}", uploads(&first));
    assert_eq!(world_space(&first), WorldSpace::Target);
    restarted.command_comma();
    assert_eq!(restarted.dialog().scaling().selected(), Scaling::SuperXbr);
    assert_eq!(restarted.dialog().prefs().scaling, Scaling::SuperXbr);
}

#[test]
fn an_upscaler_picked_in_flight_draws_the_first_frame_after_the_dialog_closes() {
    let store = seeded();
    let (mut harness, warnings) = Harness::launched_probing(&store, &[SHARP]);
    assert_eq!(warnings, Vec::<String>::new());
    harness.press_physical(KeyCode::KeyF);
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(harness.frame().world_space, WorldSpace::Target);
    harness.step_scaling(1);
    let open = harness.frame();
    assert_eq!(open.world_space, WorldSpace::Target, "held while open");
    assert!(
        texts(&open)
            .iter()
            .any(|text| text == "Scaling: Sharp-bilinear")
    );
    assert_eq!(saved(&store)["frame_upscaler"], "native");
    harness.ok();
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(harness.frame().world_space, TEXTURE, "the next frame");
    assert_eq!(harness.app.take_warnings(), Vec::<String>::new());
}

#[test]
fn an_upscaler_picked_is_saved_and_still_draws_after_a_restart() {
    let store = seeded();
    let (mut harness, _) = Harness::launched_probing(&store, &[SHARP]);
    harness.step_scaling(1);
    harness.press(Key::Enter);
    assert_eq!(saved(&store)["frame_upscaler"], "sharp_bilinear");
    assert_eq!(saved(&store)["music"], true, "the rest kept");

    let (mut restarted, warnings) = Harness::launched_probing(&store, &[SHARP]);
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(
        restarted.frame().world_space,
        TEXTURE,
        "from the first frame"
    );
    restarted.command_comma();
    assert_eq!(
        restarted.dialog().scaling().selected(),
        Scaling::SharpBilinear
    );
    assert_eq!(restarted.dialog().prefs().scaling, Scaling::SharpBilinear);
}

#[test]
fn a_stored_upscaler_unknown_or_unavailable_starts_native_with_one_warning_and_is_kept() {
    for name in [
        "sharp_bilinear",
        "metalfx_spatial",
        "warp_drive",
        "hd_pack_v9",
    ] {
        let store = seeded_with_upscaler(name);
        let before = store.text();
        let writes = store.writes();
        let (mut harness, warnings) = Harness::launched_probing(&store, &[NATIVE]);
        assert_eq!(warnings.len(), 1, "{name}: {warnings:?}");
        assert!(warnings[0].contains(&format!("\"{name}\"")), "{warnings:?}");
        assert_eq!(harness.frame().world_space, WorldSpace::Target, "{name}");
        assert_eq!((store.text(), store.writes()), (before, writes), "{name}");

        // A later change of another preference keeps it.
        harness.press(Key::Char('p'));
        assert_eq!(
            harness.dialog().scaling().selected(),
            Scaling::Native,
            "{name}"
        );
        let music = harness.dialog().music().rect().center();
        harness.click(music);
        harness.press(Key::Enter);
        assert_eq!(saved(&store)["music"], false, "{name}");
        assert_eq!(saved(&store)["frame_upscaler"], name);
        assert_eq!(harness.app.take_warnings(), Vec::<String>::new(), "{name}");
    }
}
