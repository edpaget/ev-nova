//! The app wired to its screen router over minimal synthetic game data
//! (one ship and one system), the renderer and the recording Gpu, driven
//! by synthetic window events.

// Sizes here are small powers of two, exact in floating point.
#![allow(clippy::float_cmp)]

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, WindowEvent, WindowPort, start_screen};
use nova::config::Os;
use nova::settings::{FullscreenSetting, GameKeeper, GameSettings, SettingsKeeper};
use nova_audio::SettingsStore;
use nova_audio::recording::MemorySettings;
use nova_data::graphics::RLED;
use nova_data::graphics::fixture::RledBuilder;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame, PixelRect};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_view::{FullscreenKind, Key, MonitorMode, MouseButton, Point, VideoMode};

/// A window of settable size, on a monitor of settable modes and name,
/// that counts redraw requests and records each fullscreen request.
struct FakeWindow {
    size_px: (u32, u32),
    scale_factor: f64,
    redraws: usize,
    fullscreen: Vec<Option<FullscreenKind>>,
    modes: Vec<MonitorMode>,
    monitor: Option<String>,
}

impl FakeWindow {
    fn new() -> Self {
        Self {
            size_px: (1024, 768),
            scale_factor: 1.0,
            redraws: 0,
            fullscreen: Vec::new(),
            modes: Vec::new(),
            monitor: None,
        }
    }
}

impl WindowPort for FakeWindow {
    fn size_px(&self) -> (u32, u32) {
        self.size_px
    }

    fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    fn request_redraw(&mut self) {
        self.redraws += 1;
    }

    fn set_fullscreen(&mut self, fullscreen: Option<FullscreenKind>) {
        self.fullscreen.push(fullscreen);
    }

    fn video_modes(&self) -> Vec<MonitorMode> {
        self.modes.clone()
    }

    fn monitor_name(&self) -> Option<String> {
        self.monitor.clone()
    }
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

fn put_i16(bytes: &mut [u8], at: usize, value: i16) {
    bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
}

/// Ship 128, the Shuttle, whose `shän` names a 4-frame 8x8 `rlëD`, and
/// system 128, Alpha, alone at (0, 0) with no hyperlinks or stellars.
fn data() -> Rc<GameData> {
    let mut anim = vec![0; ShipAnim::SIZE.expect("fixed")];
    put_i16(&mut anim, 0x00, 1000);
    put_i16(&mut anim, 0x04, 1);
    put_i16(&mut anim, 0x34, 4);
    let sheet = (0..4)
        .fold(RledBuilder::new(8, 8), |sheet, _| {
            sheet.frame(|f| (0..8).fold(f, |f, _| f.line().pixels(&[0x7C00; 8])))
        })
        .build();
    let mut system = vec![0; System::SIZE.expect("fixed")];
    for slot in 0..32 {
        // Every hyperlink and stellar slot unused.
        put_i16(&mut system, 0x04 + 2 * slot, -1);
    }
    put_i16(&mut system, 0x66, -1);
    let fork = ForkBuilder::new()
        .resource(
            Ship::TYPE,
            128,
            Some(b"Shuttle"),
            &vec![0; Ship::SIZE.expect("fixed")],
        )
        .resource(ShipAnim::TYPE, 128, None, &anim)
        .resource(RLED, 1000, None, &sheet)
        .resource(System::TYPE, 128, Some(b"Alpha"), &system)
        .build()
        .bytes;
    let file = OneFile(fork);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

struct Harness {
    app: App<Rc<GameData>>,
    window: FakeWindow,
    gpu: RecordingGpu,
    clock: Duration,
}

impl Harness {
    fn new() -> Self {
        Self::on(Os::current())
    }

    /// The app reading `os`'s fullscreen shortcut.
    fn on(os: Os) -> Self {
        let window = FakeWindow::new();
        let data = data();
        let app = App::new(&window, Rc::clone(&data), start_screen(data)).with_os(os);
        Self {
            app,
            window,
            gpu: RecordingGpu::new(),
            clock: Duration::ZERO,
        }
    }

    fn send(&mut self, event: WindowEvent) -> Control {
        self.app.handle(event, &mut self.window, &mut self.gpu)
    }

    /// Sends a redraw 1/60 s after the last and returns its frame.
    fn frame(&mut self) -> Frame {
        self.clock += Duration::from_micros(16_667);
        let before = self.gpu.submits().len();
        self.send(WindowEvent::Redraw {
            elapsed: self.clock,
        });
        let submits = self.gpu.submits();
        assert_eq!(submits.len(), before + 1, "one submit per redraw");
        let frame = submits[before].clone();
        assert_eq!(self.app.take_failures(), []);
        // Only flight draws a world layer.
        assert_eq!(frame.world, [0_usize; 0], "no world");
        frame
    }
}

/// The frame's batches as (kind, length).
fn shape(frame: &Frame) -> Vec<(&'static str, usize)> {
    frame
        .batches
        .iter()
        .map(|batch| match batch {
            Batch::Sprites { quads, .. } => ("sprites", quads.len()),
            Batch::Solid(quads) => ("solid", quads.len()),
            Batch::Text(runs) => ("text", runs.len()),
        })
        .collect()
}

fn text_size(frame: &Frame) -> f32 {
    frame
        .batches
        .iter()
        .find_map(|batch| match batch {
            // The size it draws at, in pixels.
            Batch::Text(runs) => Some(runs[0].size * runs[0].scale),
            _ => None,
        })
        .expect("a text batch")
}

#[test]
fn every_redraw_submits_one_frame_and_asks_for_the_next() {
    let mut harness = Harness::new();

    let first = harness.frame();
    let second = harness.frame();

    // The ship's sprite, then its name, four stats and the footer, and the
    // router's hint.
    let expected = [("sprites", 1), ("text", 7)];
    assert_eq!(shape(&first), expected);
    assert_eq!(shape(&second), expected);
    assert_eq!(harness.window.redraws, 2);
    assert_eq!(first.target, (1024, 768));
    assert_eq!(
        first.viewport,
        PixelRect {
            x: 0,
            y: 0,
            w: 1024,
            h: 768
        }
    );
}

#[test]
fn a_resize_updates_the_next_frames_viewport() {
    let mut harness = Harness::new();
    let normal = harness.frame();

    harness.send(WindowEvent::Resized {
        size_px: (2048, 1536),
        scale_factor: 2.0,
    });
    let retina = harness.frame();
    harness.send(WindowEvent::Resized {
        size_px: (1280, 768),
        scale_factor: 1.0,
    });
    let wide = harness.frame();

    assert_eq!(
        retina.viewport,
        PixelRect {
            x: 0,
            y: 0,
            w: 2048,
            h: 1536
        }
    );
    assert_eq!(retina.target, (2048, 1536));
    assert_eq!(text_size(&retina), 2.0 * text_size(&normal));
    assert_eq!(wide.viewport.x, 128);
    assert_eq!(wide.viewport.w, 1024);
}

#[test]
fn escape_and_closing_exit() {
    let mut harness = Harness::new();
    assert_eq!(
        harness.send(WindowEvent::Key {
            key: Key::Escape,
            pressed: true,
            repeat: false
        }),
        Control::Exit
    );
    assert_eq!(harness.send(WindowEvent::CloseRequested), Control::Exit);
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

#[test]
fn tab_switches_between_the_ship_browser_and_the_galaxy_map() {
    let mut harness = Harness::new();
    let tab = |pressed| WindowEvent::Key {
        key: Key::Tab,
        pressed,
        repeat: false,
    };
    let held_tab = WindowEvent::Key {
        key: Key::Tab,
        pressed: true,
        repeat: true,
    };
    let ships = texts(&harness.frame());
    assert!(ships.contains(&"Shuttle".to_owned()), "{ships:?}");
    assert!(ships.contains(&"Tab: ships / galaxy map   F: fly".to_owned()));

    assert_eq!(harness.send(tab(true)), Control::Continue);
    // Holding Tab past the key-repeat delay stays on the map.
    for _ in 0..3 {
        assert_eq!(harness.send(held_tab), Control::Continue);
    }
    assert_eq!(harness.send(tab(false)), Control::Continue);
    let map = texts(&harness.frame());
    assert!(
        map.contains(&"Click a system to see its stellars".to_owned()),
        "{map:?}"
    );
    assert!(!map.contains(&"Shuttle".to_owned()));
    assert!(map.contains(&"Tab: ships / galaxy map   F: fly".to_owned()));

    harness.send(tab(true));
    let back = texts(&harness.frame());
    assert!(back.contains(&"Shuttle".to_owned()), "{back:?}");
}

// Fullscreen.

const BORDERLESS: Option<FullscreenKind> = Some(FullscreenKind::Borderless);

const NOT_FULLSCREEN: Vec<Option<FullscreenKind>> = Vec::new();

fn press(key: Key) -> [WindowEvent; 2] {
    [true, false].map(|pressed| WindowEvent::Key {
        key,
        pressed,
        repeat: false,
    })
}

impl Harness {
    /// Sends each of `events`.
    fn send_all(&mut self, events: impl IntoIterator<Item = WindowEvent>) {
        for event in events {
            assert_eq!(self.send(event), Control::Continue, "{event:?}");
        }
    }

    /// Clicks at `px`, a window position in physical pixels.
    fn click_px(&mut self, px: (f64, f64)) {
        self.send_all([
            WindowEvent::PointerMoved { px },
            WindowEvent::PointerButton {
                button: MouseButton::Left,
                pressed: true,
            },
            WindowEvent::PointerButton {
                button: MouseButton::Left,
                pressed: false,
            },
        ]);
    }

    /// Where system 128, Alpha, is on the galaxy map, logically.
    fn alpha(&self) -> Point {
        let map = self.app.screen().galaxy_map();
        let alpha = map.model().system(nova_sim::SystemId(128)).expect("Alpha");
        map.view()
            .expect("the viewer")
            .world_to_screen(alpha.position())
    }

    /// Whether the map shows Alpha selected.
    fn alpha_selected(&mut self) -> bool {
        texts(&self.frame()).contains(&"Alpha (sÿst 128)".to_owned())
    }
}

#[test]
fn clicks_in_fullscreen_land_where_they_land_in_a_window_and_the_bars_take_none() {
    let mut harness = Harness::on(Os::Windows);
    harness.send_all(press(Key::Tab));
    let alpha = harness.alpha();
    // In the 1024x768 window, a pixel is a logical point.
    harness.click_px((f64::from(alpha.x), f64::from(alpha.y)));
    assert!(harness.alpha_selected(), "windowed");
    harness.click_px((f64::from(alpha.x) + 200.0, f64::from(alpha.y) + 100.0));
    assert!(!harness.alpha_selected(), "a click elsewhere deselects");

    harness.send_all([
        WindowEvent::Key {
            key: Key::Alt,
            pressed: true,
            repeat: false,
        },
        WindowEvent::Key {
            key: Key::Enter,
            pressed: true,
            repeat: false,
        },
    ]);
    assert_eq!(harness.window.fullscreen, [BORDERLESS]);
    harness.send_all([
        WindowEvent::FullscreenChanged { fullscreen: true },
        WindowEvent::Resized {
            size_px: (2560, 1440),
            scale_factor: 1.0,
        },
    ]);
    // 16:9: the 4:3 game is 1920 pixels wide at 1.875 pixels a point,
    // between two 320-pixel bars.
    let (content, scale) = harness.app.viewport().content().expect("a content rect");
    assert_eq!(
        (content, scale),
        (
            PixelRect {
                x: 320,
                y: 0,
                w: 1920,
                h: 1440
            },
            1.875
        )
    );
    let px = (
        320.0 + 1.875 * f64::from(alpha.x),
        1.875 * f64::from(alpha.y),
    );
    assert_eq!(harness.app.viewport().window_to_logical(px), Some(alpha));
    harness.click_px(px);
    assert!(harness.alpha_selected(), "the same point, fullscreen");
    harness.click_px((100.0, px.1 + 100.0));
    assert!(harness.alpha_selected(), "a click in a bar reaches nothing");
}

fn keeper(store: &MemorySettings) -> GameKeeper {
    let (keeper, warning) = SettingsKeeper::open(Box::new(store.clone()) as Box<dyn SettingsStore>);
    assert_eq!(warning, None);
    keeper
}

/// Starts the game over `store` as the runner does, restoring the window
/// it opens on a monitor with no modes or name; plays `events`, then
/// quits. Returns the fullscreen requests its window got, and checks
/// there were no warnings.
fn session(store: &MemorySettings, events: &[WindowEvent]) -> Vec<Option<FullscreenKind>> {
    let (requests, warnings) = session_on(store, &Monitor::default(), events);
    assert_eq!(warnings, Vec::<String>::new());
    requests
}

/// A monitor's modes and name.
#[derive(Default)]
struct Monitor {
    modes: Vec<VideoMode>,
    name: Option<&'static str>,
}

/// As [`session`], on `monitor`, returning the warnings too, running on
/// Windows.
fn session_on(
    store: &MemorySettings,
    monitor: &Monitor,
    events: &[WindowEvent],
) -> (Vec<Option<FullscreenKind>>, Vec<String>) {
    session_on_os(store, monitor, events, Os::Windows)
}

/// As [`session_on`], running on `os`.
fn session_on_os(
    store: &MemorySettings,
    monitor: &Monitor,
    events: &[WindowEvent],
    os: Os,
) -> (Vec<Option<FullscreenKind>>, Vec<String>) {
    let mut window = FakeWindow::new();
    window.modes = monitor
        .modes
        .iter()
        .map(|&mode| MonitorMode {
            mode,
            bit_depth: 32,
        })
        .collect();
    window.monitor = monitor.name.map(str::to_owned);
    let data = data();
    let mut app = App::new(&window, Rc::clone(&data), start_screen(data))
        .with_os(os)
        .with_settings(keeper(store));
    app.restore_window(&mut window);
    let mut gpu = RecordingGpu::new();
    for event in events {
        assert_eq!(app.handle(*event, &mut window, &mut gpu), Control::Continue);
    }
    assert_eq!(
        app.handle(WindowEvent::CloseRequested, &mut window, &mut gpu),
        Control::Exit
    );
    (window.fullscreen, app.take_warnings())
}

#[test]
fn a_game_quit_fullscreen_opens_fullscreen_and_one_quit_windowed_opens_windowed() {
    let store = MemorySettings::new();
    let f11 = press(Key::Function(11));
    assert_eq!(
        session(&store, &f11),
        [BORDERLESS],
        "the first opens windowed"
    );
    assert_eq!(
        session(&store, &f11),
        [BORDERLESS, None],
        "opened fullscreen"
    );
    assert_eq!(session(&store, &[]), NOT_FULLSCREEN, "opened windowed");
    let system = [WindowEvent::FullscreenChanged { fullscreen: true }];
    assert_eq!(session(&store, &system), NOT_FULLSCREEN);
    assert_eq!(
        session(&store, &[]),
        [BORDERLESS],
        "entered through the system"
    );
}

fn fhd() -> VideoMode {
    VideoMode {
        width: 1920,
        height: 1080,
        refresh_millihertz: 60_000,
    }
}

#[test]
fn on_macos_a_saved_exclusive_mode_reopens_in_the_native_space_and_quits_leaving_it_alone() {
    let saved_kind = FullscreenSetting::Exclusive {
        mode: fhd(),
        monitor: Some("DELL U2720Q".to_owned()),
    };
    let store = MemorySettings::new();
    let (mut seed, _) = SettingsKeeper::open(store.clone());
    seed.change(GameSettings {
        fullscreen: true,
        fullscreen_kind: saved_kind.clone(),
        ..GameSettings::default()
    })
    .expect("seeds");
    let dell = Monitor {
        modes: vec![fhd()],
        name: Some("DELL U2720Q"),
    };
    let focus = [WindowEvent::FocusLost, WindowEvent::FocusGained];
    assert_eq!(
        session_on_os(&store, &dell, &focus, Os::MacOs),
        (vec![BORDERLESS], vec![]),
        "no display mode change on focus or exit, and no warning"
    );
    let (kept, _) = SettingsKeeper::open(store.clone());
    assert!(kept.settings().fullscreen);
    assert_eq!(kept.settings().fullscreen_kind, saved_kind, "not migrated");
}

#[test]
fn an_exclusive_mode_reopens_falls_back_when_gone_and_survives_a_windowed_launch() {
    let saved_kind = FullscreenSetting::Exclusive {
        mode: fhd(),
        monitor: Some("DELL U2720Q".to_owned()),
    };
    let store = MemorySettings::new();
    let (mut seed, _) = SettingsKeeper::open(store.clone());
    seed.change(GameSettings {
        fullscreen: true,
        fullscreen_kind: saved_kind.clone(),
        ..GameSettings::default()
    })
    .expect("seeds");
    let qhd = VideoMode {
        width: 2560,
        height: 1440,
        refresh_millihertz: 144_000,
    };
    let dell = Monitor {
        modes: vec![qhd, fhd()],
        name: Some("DELL U2720Q"),
    };
    let exclusive = Some(FullscreenKind::Exclusive(fhd()));
    assert_eq!(
        session_on(&store, &dell, &[]),
        (vec![exclusive, None], vec![]),
        "reopens in the mode, and leaves it before the exit"
    );

    let label = "1920 x 1080, 60 Hz";
    for gone in [
        Monitor {
            modes: vec![qhd],
            ..dell
        },
        Monitor {
            modes: vec![qhd, fhd()],
            name: Some("LG HDR 4K"),
        },
    ] {
        let (requests, warnings) = session_on(&store, &gone, &[]);
        assert_eq!(requests, [BORDERLESS], "borderless, left as it is");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains(label), "{warnings:?}");
        let (kept, _) = SettingsKeeper::open(store.clone());
        assert_eq!(kept.settings().fullscreen_kind, saved_kind, "kept");
    }

    let dell = Monitor {
        modes: vec![qhd, fhd()],
        name: Some("DELL U2720Q"),
    };
    let f11 = press(Key::Function(11));
    assert_eq!(
        session_on(&store, &dell, &f11),
        (vec![exclusive, None], vec![]),
        "F11 leaves to the window"
    );
    let (windowed, _) = SettingsKeeper::open(store.clone());
    assert!(!windowed.settings().fullscreen);
    assert_eq!(
        session_on(&store, &dell, &[]),
        (vec![], vec![]),
        "opens windowed"
    );
    assert_eq!(
        session_on(&store, &dell, &f11),
        (vec![exclusive, None], vec![]),
        "F11 enters the saved mode"
    );
}
