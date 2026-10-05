//! The app layer: window events in, frames out, through ports.
//!
//! [`App`] owns the viewport and the current screen, an [`AppScreen`] that
//! routes to whichever of the game's screens is showing; new screens are
//! added there, so the platform adapter never names one. The adapter
//! hands it [`WindowEvent`]s, already in the core's terms, together with
//! the [`WindowPort`] and a [`Gpu`]; the app turns them into screen input
//! (every key, Escape included, is the screen's to act on, and so is the
//! text each key press types),
//! ticks and draws the screen, and renders each frame through
//! `nova-render`. It never blocks and never names a winit or wgpu type.
//!
//! # The developer overlay
//!
//! An app built [`App::with_dev_overlay`] holds a [`DevOverlay`]. Its
//! toggle key (backquote) shows and hides it; while it shows, it takes
//! every key and pointer event, and the screen sees none of them.
//! [`App::handle_routed`] says where each event went, and
//! [`App::overlay_wants`] whether the overlay's drawing (egui, in the
//! `dev-tools` build) should see the raw event too. Without an overlay,
//! every event goes to the game as before.
//!
//! # Sound
//!
//! An app built [`App::with_audio`] holds the audio core. After every
//! input it routes and every frame it ticks, it hands the core the screen
//! shown ([`Screen::now_showing`]) and the sounds the screen made
//! ([`Screen::take_sounds`]); the core decides what plays. Without one,
//! the sounds are let go.
//!
//! # Settings
//!
//! When the screen reports new sound preferences
//! ([`Screen::take_sound_prefs`], from the Preferences dialog), the app
//! applies them to the audio core, if it has one, and saves them through
//! the settings keeper it was built [`App::with_settings`], if any. The
//! app prints nothing: a save that fails is kept as a warning until the
//! caller takes it ([`App::take_warnings`]) and shows it. So are the
//! warnings the screen reports ([`Screen::take_warnings`]), which the app
//! takes after every input and frame.
//!
//! # Diagnostics
//!
//! An app built [`App::with_diagnostics`] writes each diagnostic the
//! screen reports ([`Screen::take_diagnostics`]: game data the screen could
//! not read or the simulation does not handle yet) as a line, `nova: ` and the diagnostic, through
//! the writer it was given; it takes them where it takes the sounds, after
//! every input and frame. Without a writer, they are let go.
//!
//! # Quitting
//!
//! Whichever way the app quits (the window closed, or the screen asking
//! to quit), it first tells the screen ([`Screen::quit`]), so the screen
//! can keep what should outlive it, such as the pilot.

use std::io::Write;
use std::time::Duration;

use nova_audio::{Audio, AudioCore, AudioSettings, SettingsKeeper, SettingsStore};
use nova_render::{Gpu, ImageError, ImageSource, LOGICAL, Renderer, Viewport};
use nova_view::devtools::{DevOverlay, Routing};
use nova_view::{DrawList, ImageKey, Input, Key, MouseButton, Screen, ScreenAction};

pub mod screen;

pub use screen::{AppScreen, Showing, start_screen};

/// What the app needs from the window.
pub trait WindowPort {
    /// The window's inner size in physical pixels.
    fn size_px(&self) -> (u32, u32);
    /// Physical pixels per point on the window's display.
    fn scale_factor(&self) -> f64;
    /// Asks for another [`WindowEvent::Redraw`].
    fn request_redraw(&mut self);
}

/// A window event, in the core's terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowEvent {
    /// The window's size or display scale changed.
    Resized {
        /// The new inner size in physical pixels.
        size_px: (u32, u32),
        /// Physical pixels per point.
        scale_factor: f64,
    },
    /// A key went down or up.
    Key {
        /// The key.
        key: Key,
        /// Down (`true`) or up.
        pressed: bool,
        /// Whether this press is the OS repeating a held key; see
        /// [`Input::Key`].
        repeat: bool,
    },
    /// A printable character was typed: it follows the [`WindowEvent::Key`]
    /// press that typed it.
    Text(char),
    /// The pointer moved to a window position in physical pixels.
    PointerMoved {
        /// Where, in physical pixels from the window's top-left.
        px: (f64, f64),
    },
    /// A mouse button went down or up at the last pointer position.
    PointerButton {
        /// The button.
        button: MouseButton,
        /// Down (`true`) or up.
        pressed: bool,
    },
    /// Time to draw. The clock is read at the edge and passed in.
    Redraw {
        /// Time since the app started.
        elapsed: Duration,
    },
    /// The window stopped receiving keyboard input; the releases of any
    /// keys held down will not arrive.
    FocusLost,
    /// The user asked to close the window.
    CloseRequested,
}

/// Whether the app goes on after an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// Keep running.
    Continue,
    /// Quit.
    Exit,
}

/// What handling one window event did: whether the app goes on, and where
/// the event went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handled {
    /// Whether the app goes on.
    pub control: Control,
    /// Where the event went: [`Routing::Game`] for every event without an
    /// overlay, and for events that are not input.
    pub routing: Routing,
}

/// The app: routes window events to the current screen and draws it.
///
/// The program runs `App<S>`, whose screen is the [`AppScreen`] router.
/// The screen stays a type parameter only so the app's own tests can drive
/// it with a screen that records what reaches it.
pub struct App<S, C = AppScreen> {
    renderer: Renderer<S>,
    viewport: Viewport,
    screen: C,
    last_redraw: Duration,
    pointer: Option<(f64, f64)>,
    failures: Vec<(ImageKey, ImageError)>,
    overlay: Option<DevOverlay>,
    audio: Option<AudioCore<Box<dyn Audio>>>,
    settings: Option<SettingsKeeper<Box<dyn SettingsStore>>>,
    warnings: Vec<String>,
    diagnostics: Option<Box<dyn Write>>,
}

impl<S: ImageSource, C: Screen> App<S, C> {
    /// An app drawing `screen` with images from `images` into `window`.
    pub fn new(window: &impl WindowPort, images: S, screen: C) -> Self {
        Self {
            renderer: Renderer::new(images),
            viewport: Viewport::new(LOGICAL, window.size_px(), window.scale_factor()),
            screen,
            last_redraw: Duration::ZERO,
            pointer: None,
            failures: Vec::new(),
            overlay: None,
            audio: None,
            settings: None,
            warnings: Vec::new(),
            diagnostics: None,
        }
    }

    /// The app with each diagnostic the screen reports written as a line
    /// through `out`.
    #[must_use]
    pub fn with_diagnostics(mut self, out: Box<dyn Write>) -> Self {
        self.diagnostics = Some(out);
        self
    }

    /// The app with a developer overlay, hidden until its toggle key.
    #[must_use]
    pub fn with_dev_overlay(mut self) -> Self {
        self.overlay = Some(DevOverlay::new());
        self
    }

    /// The app with sound: after each input it routes and each frame it
    /// ticks, it gives `core` the screen shown and the sounds the screen
    /// made. The core takes in the screen it starts on at once.
    #[must_use]
    pub fn with_audio(mut self, core: AudioCore<Box<dyn Audio>>) -> Self {
        self.audio = Some(core);
        self.feed_audio();
        self
    }

    /// The app with the player's settings kept by `keeper`: each change
    /// of the sound preferences is saved through it.
    #[must_use]
    pub fn with_settings(mut self, keeper: SettingsKeeper<Box<dyn SettingsStore>>) -> Self {
        self.settings = Some(keeper);
        self
    }

    /// The warnings since they were last taken, such as a failed save,
    /// for the caller to show; taking them empties the list.
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// Gives the audio core, if there is one, the screen shown and the
    /// sounds the screen has made; without one, the sounds are let go.
    /// Then writes out the screen's diagnostics.
    ///
    /// First, a change of the sound preferences the screen reports is
    /// applied to the core and saved through the keeper, each when there
    /// is one; a failed save is kept as a warning.
    fn feed_audio(&mut self) {
        if let Some(prefs) = self.screen.take_sound_prefs() {
            let current = match (&self.settings, &self.audio) {
                (Some(keeper), _) => keeper.settings(),
                (None, Some(core)) => core.settings(),
                (None, None) => AudioSettings::default(),
            };
            let settings = current.with_prefs(prefs);
            if let Some(core) = &mut self.audio {
                core.apply(settings);
            }
            if let Some(keeper) = &mut self.settings
                && let Err(warning) = keeper.change(settings)
            {
                self.warnings.push(warning);
            }
        }
        let sounds = self.screen.take_sounds();
        if let Some(core) = &mut self.audio {
            core.update(self.screen.now_showing(), &sounds);
        }
        self.write_diagnostics();
    }

    /// Writes each diagnostic the screen reports as a line through the
    /// writer, if there is one; without one, they are let go. A line that
    /// cannot be written is let go too: there is nowhere else to say so.
    fn write_diagnostics(&mut self) {
        let diagnostics = self.screen.take_diagnostics();
        if let Some(out) = &mut self.diagnostics {
            for diagnostic in diagnostics {
                let _ = writeln!(out, "nova: {diagnostic}");
            }
        }
    }

    /// The developer overlay, if the app has one.
    pub fn dev_overlay(&self) -> Option<&DevOverlay> {
        self.overlay.as_ref()
    }

    /// Whether the overlay's drawing should see the raw window event that
    /// was `handled` as given, or `None` for one the app does not take: true
    /// while the overlay shows, unless the event was its toggle key's.
    pub fn overlay_wants(&self, handled: Option<&Handled>) -> bool {
        self.overlay.as_ref().is_some_and(DevOverlay::visible)
            && handled.is_none_or(|handled| handled.routing != Routing::Toggle)
    }

    /// The current screen.
    pub fn screen(&self) -> &C {
        &self.screen
    }

    /// The image source.
    pub fn images(&self) -> &S {
        self.renderer.images()
    }

    /// The logical space's fit into the window.
    pub fn viewport(&self) -> &Viewport {
        &self.viewport
    }

    /// Handles one window event.
    ///
    /// A resize refits the viewport. Keys go to the screen, Escape
    /// included: what it does is the screen's to decide. So does the text
    /// a key press types. Losing focus tells the screen to let go of
    /// the keys it holds down, since their releases will not arrive. Pointer events go to the screen in logical
    /// units, and are dropped in the bars. A redraw ticks the screen by the
    /// time since the last redraw, renders its draw list through `gpu` and
    /// asks the window for the next redraw. Closing the window, or a screen
    /// asking to quit, exits.
    ///
    /// With a developer overlay, see [`App::handle_routed`].
    pub fn handle(
        &mut self,
        event: WindowEvent,
        window: &mut impl WindowPort,
        gpu: &mut impl Gpu,
    ) -> Control {
        self.handle_routed(event, window, gpu).control
    }

    /// Handles one window event as [`App::handle`] does, and says where it
    /// went.
    ///
    /// With a developer overlay, keys go through it first: its toggle key
    /// shows or hides it and goes nowhere else, and opening it makes the
    /// screen abandon its pointer gesture and let go of its keys. While it
    /// shows, every other key and pointer event goes to it alone, wherever
    /// the pointer is, though the pointer's position is still recorded. Every
    /// redraw also gives the overlay its time.
    pub fn handle_routed(
        &mut self,
        event: WindowEvent,
        window: &mut impl WindowPort,
        gpu: &mut impl Gpu,
    ) -> Handled {
        let game = |control| Handled {
            control,
            routing: Routing::Game,
        };
        let overlay = Handled {
            control: Control::Continue,
            routing: Routing::Overlay,
        };
        match event {
            WindowEvent::Resized {
                size_px,
                scale_factor,
            } => {
                self.viewport = Viewport::new(LOGICAL, size_px, scale_factor);
                game(Control::Continue)
            }
            WindowEvent::CloseRequested => {
                self.quit();
                game(Control::Exit)
            }
            WindowEvent::FocusLost => {
                self.screen.release_keys();
                game(Control::Continue)
            }
            WindowEvent::Key {
                key,
                pressed,
                repeat,
            } => match self.route_key(key, pressed, repeat) {
                Routing::Game => game(self.route(Input::Key {
                    key,
                    pressed,
                    repeat,
                })),
                routing => Handled {
                    control: Control::Continue,
                    routing,
                },
            },
            WindowEvent::Text(c) => {
                if self.overlay.as_ref().is_some_and(DevOverlay::visible) {
                    return overlay;
                }
                game(self.route(Input::Text(c)))
            }
            WindowEvent::PointerMoved { px } => {
                self.pointer = Some(px);
                if self.pointer_routing() == Routing::Overlay {
                    return overlay;
                }
                game(match self.viewport.window_to_logical(px) {
                    Some(at) => self.route(Input::PointerMoved(at)),
                    None => Control::Continue,
                })
            }
            WindowEvent::PointerButton { button, pressed } => {
                if self.pointer_routing() == Routing::Overlay {
                    return overlay;
                }
                let at = self
                    .pointer
                    .and_then(|px| self.viewport.window_to_logical(px));
                game(match at {
                    Some(at) => self.route(Input::PointerButton {
                        button,
                        pressed,
                        at,
                    }),
                    None => Control::Continue,
                })
            }
            WindowEvent::Redraw { elapsed } => {
                if let Some(overlay) = &mut self.overlay {
                    overlay.frame(elapsed);
                }
                self.screen.tick(elapsed.saturating_sub(self.last_redraw));
                self.feed_audio();
                self.take_screen_warnings();
                self.last_redraw = self.last_redraw.max(elapsed);
                let mut list = DrawList::new();
                self.screen.draw(&mut list);
                let report = self.renderer.render(&list, &self.viewport, gpu);
                self.failures.extend(report.new_failures);
                window.request_redraw();
                game(Control::Continue)
            }
        }
    }

    /// Where a key goes; opening the overlay lets go of the screen.
    fn route_key(&mut self, key: Key, pressed: bool, repeat: bool) -> Routing {
        let Some(overlay) = &mut self.overlay else {
            return Routing::Game;
        };
        let was_visible = overlay.visible();
        let routing = overlay.key(key, pressed, repeat);
        if overlay.visible() && !was_visible {
            self.screen.cancel_pointer();
            self.screen.release_keys();
        }
        routing
    }

    fn pointer_routing(&self) -> Routing {
        self.overlay
            .as_ref()
            .map_or(Routing::Game, DevOverlay::pointer)
    }

    /// The image failures reported since the last call, each once.
    pub fn take_failures(&mut self) -> Vec<(ImageKey, ImageError)> {
        std::mem::take(&mut self.failures)
    }

    fn route(&mut self, input: Input) -> Control {
        let action = self.screen.input(&input);
        self.feed_audio();
        self.take_screen_warnings();
        match action {
            ScreenAction::None => Control::Continue,
            ScreenAction::Quit => {
                self.quit();
                Control::Exit
            }
        }
    }

    /// Tells the screen the app is quitting, and keeps what it reports.
    fn quit(&mut self) {
        self.screen.quit();
        self.take_screen_warnings();
    }

    /// Keeps the warnings the screen reports, for the caller to take.
    fn take_screen_warnings(&mut self) {
        self.warnings.extend(self.screen.take_warnings());
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::time::Duration;

    use nova_audio::recording::{AudioLog, MemorySettings, RecordingAudio};
    use nova_audio::{Audio, AudioCommand, AudioCore, SettingsKeeper, SettingsStore, Volume};
    use nova_data::graphics::Image;
    use nova_render::recording::RecordingGpu;
    use nova_render::{ImageError, ImageSource, PixelRect};
    use nova_view::sound::SimSound;
    use nova_view::{
        Color, Diagnostic, DrawList, ImageKey, ImageKind, Input, Key, MouseButton, Point, Screen,
        ScreenAction, Showing, Sound, SoundPrefs, UiSound,
    };

    use super::*;

    /// A window of settable size that counts redraw requests.
    struct FakeWindow {
        size_px: (u32, u32),
        scale_factor: f64,
        redraws: usize,
    }

    impl FakeWindow {
        fn new(size_px: (u32, u32), scale_factor: f64) -> Self {
            Self {
                size_px,
                scale_factor,
                redraws: 0,
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
    }

    /// No images at all.
    struct NoImages;

    impl ImageSource for NoImages {
        fn frames(&self, _kind: ImageKind, _id: i16) -> Result<Vec<Image>, ImageError> {
            Err(ImageError::Missing)
        }
    }

    /// Records its inputs and ticks; quits on `quit_on`; draws one sprite;
    /// counts the times it is told the app quits; hands out `warnings` and
    /// `diagnostics`.
    #[derive(Default)]
    struct RecordingScreen {
        inputs: Vec<Input>,
        ticks: Vec<Duration>,
        quit_on: Option<Key>,
        releases: usize,
        cancels: usize,
        quits: usize,
        warnings: Vec<String>,
        diagnostics: Vec<Diagnostic>,
    }

    impl Screen for RecordingScreen {
        fn input(&mut self, input: &Input) -> ScreenAction {
            self.inputs.push(*input);
            match input {
                Input::Key { key, .. } if Some(*key) == self.quit_on => ScreenAction::Quit,
                _ => ScreenAction::None,
            }
        }

        fn tick(&mut self, dt: Duration) {
            self.ticks.push(dt);
        }

        fn draw(&self, list: &mut DrawList) {
            list.sprite(ImageKey::sprite(1, 0), Point::new(0.0, 0.0), Color::WHITE);
        }

        fn release_keys(&mut self) {
            self.releases += 1;
        }

        fn cancel_pointer(&mut self) {
            self.cancels += 1;
        }

        fn quit(&mut self) {
            self.quits += 1;
            self.warnings.push(format!("quit {}", self.quits));
        }

        fn take_warnings(&mut self) -> Vec<String> {
            std::mem::take(&mut self.warnings)
        }

        fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
            std::mem::take(&mut self.diagnostics)
        }
    }

    type TestApp = App<NoImages, RecordingScreen>;

    /// Bytes written, kept where the test can read them.
    #[derive(Clone, Default)]
    struct Written(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

    impl std::io::Write for Written {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Written {
        fn text(&self) -> String {
            String::from_utf8(self.0.borrow().clone()).expect("UTF-8")
        }
    }

    /// Fails every write.
    struct Broken;

    impl std::io::Write for Broken {
        fn write(&mut self, _bytes: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("closed"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn flag(weapon: i16) -> Diagnostic {
        Diagnostic::Sim(nova_sim::SimDiagnostic::UnimplementedWeaponFlag {
            weapon: nova_sim::WeaponId(weapon),
            field: nova_sim::combat::flags::FlagField::Flags2,
            bit: 0x8000,
        })
    }

    #[test]
    fn each_diagnostic_is_written_as_a_line_after_a_frame_or_an_input() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let written = Written::default();
        let mut app = app(&window).with_diagnostics(Box::new(written.clone()));
        let unreadable = Diagnostic::Unreadable("wëap 140: no spïn 3005".to_owned());
        app.screen.diagnostics = vec![flag(181), unreadable, flag(165)];
        let redraw = WindowEvent::Redraw {
            elapsed: Duration::from_millis(16),
        };
        handle(&mut app, &mut window, redraw);
        assert_eq!(
            written.text(),
            "nova: wëap 181 Flags2 0x8000 not implemented\n\
             nova: wëap 140: no spïn 3005\n\
             nova: wëap 165 Flags2 0x8000 not implemented\n"
        );
        app.screen.diagnostics = vec![flag(167)];
        handle(&mut app, &mut window, key(Key::Up, true));
        assert!(
            written
                .text()
                .ends_with("\nnova: wëap 167 Flags2 0x8000 not implemented\n"),
            "{}",
            written.text()
        );
        assert_eq!(app.screen.diagnostics, [], "taken");
    }

    #[test]
    fn without_a_writer_or_with_a_broken_one_diagnostics_are_let_go() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        app.screen.diagnostics = vec![flag(181)];
        handle(&mut app, &mut window, key(Key::Up, true));
        assert_eq!(app.screen.diagnostics, [], "taken all the same");
        let mut broken = self::app(&window).with_diagnostics(Box::new(Broken));
        broken.screen.diagnostics = vec![flag(181)];
        handle(&mut broken, &mut window, key(Key::Up, true));
        assert_eq!(broken.screen.diagnostics, []);
        assert_eq!(broken.take_warnings(), Vec::<String>::new());
    }

    fn app(window: &FakeWindow) -> TestApp {
        App::new(window, NoImages, RecordingScreen::default())
    }

    fn handle(app: &mut TestApp, window: &mut FakeWindow, event: WindowEvent) -> Control {
        app.handle(event, window, &mut RecordingGpu::new())
    }

    fn key(key: Key, pressed: bool) -> WindowEvent {
        WindowEvent::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    #[test]
    fn the_viewport_starts_at_the_windows_size() {
        let window = FakeWindow::new((2560, 1536), 2.0);
        let app = app(&window);
        assert_eq!(app.viewport().window_px(), (2560, 1536));
        assert_eq!(app.viewport().scale_factor(), 2.0);
        assert_eq!(app.viewport().logical(), nova_render::LOGICAL);
    }

    #[test]
    fn a_resize_rebuilds_the_viewport() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        let resized = WindowEvent::Resized {
            size_px: (1280, 768),
            scale_factor: 1.5,
        };
        assert_eq!(handle(&mut app, &mut window, resized), Control::Continue);
        assert_eq!(app.viewport().window_px(), (1280, 768));
        assert_eq!(app.viewport().scale_factor(), 1.5);
        assert_eq!(
            app.viewport().content().map(|(rect, _)| rect),
            Some(PixelRect {
                x: 128,
                y: 0,
                w: 1024,
                h: 768
            })
        );
    }

    #[test]
    fn keys_go_to_the_screen() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Up, true)),
            Control::Continue
        );
        let held = WindowEvent::Key {
            key: Key::Up,
            pressed: true,
            repeat: true,
        };
        handle(&mut app, &mut window, held);
        handle(&mut app, &mut window, key(Key::Up, false));
        handle(&mut app, &mut window, key(Key::Escape, false));
        assert_eq!(
            app.screen().inputs,
            [
                Input::Key {
                    key: Key::Up,
                    pressed: true,
                    repeat: false
                },
                Input::Key {
                    key: Key::Up,
                    pressed: true,
                    repeat: true
                },
                Input::Key {
                    key: Key::Up,
                    pressed: false,
                    repeat: false
                },
                Input::Key {
                    key: Key::Escape,
                    pressed: false,
                    repeat: false
                },
            ]
        );
    }

    #[test]
    fn escape_reaches_the_screen() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Escape, true)),
            Control::Continue
        );
        assert_eq!(
            app.screen().inputs,
            [Input::Key {
                key: Key::Escape,
                pressed: true,
                repeat: false
            }]
        );
    }

    #[test]
    fn a_screen_quit_tells_the_screen_and_exits() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let screen = RecordingScreen {
            quit_on: Some(Key::Enter),
            ..RecordingScreen::default()
        };
        let mut app = App::new(&window, NoImages, screen);
        handle(&mut app, &mut window, key(Key::Up, true));
        assert_eq!(app.screen().quits, 0);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Enter, true)),
            Control::Exit
        );
        assert_eq!(app.screen().quits, 1);
        assert_eq!(app.take_warnings(), ["quit 1"], "what quitting reported");
    }

    #[test]
    fn typed_text_reaches_the_screen_after_its_key() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Char('p'), true)),
            Control::Continue
        );
        assert_eq!(
            handle(&mut app, &mut window, WindowEvent::Text('P')),
            Control::Continue
        );
        assert_eq!(
            app.screen().inputs,
            [
                Input::Key {
                    key: Key::Char('p'),
                    pressed: true,
                    repeat: false
                },
                Input::Text('P'),
            ]
        );
    }

    #[test]
    fn a_screen_quit_on_typed_text_exits() {
        struct QuitOnText;
        impl Screen for QuitOnText {
            fn input(&mut self, input: &Input) -> ScreenAction {
                if matches!(input, Input::Text(_)) {
                    ScreenAction::Quit
                } else {
                    ScreenAction::None
                }
            }
            fn tick(&mut self, _dt: Duration) {}
            fn draw(&self, _list: &mut DrawList) {}
        }
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = App::new(&window, NoImages, QuitOnText);
        assert_eq!(
            app.handle(
                WindowEvent::Text('q'),
                &mut window,
                &mut RecordingGpu::new()
            ),
            Control::Exit
        );
    }

    #[test]
    fn the_screens_warnings_are_kept_after_each_input_and_redraw() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let screen = RecordingScreen {
            warnings: vec!["first".to_owned()],
            ..RecordingScreen::default()
        };
        let mut app = App::new(&window, NoImages, screen);
        assert_eq!(app.take_warnings(), Vec::<String>::new(), "none taken yet");
        handle(&mut app, &mut window, key(Key::Up, true));
        assert_eq!(app.take_warnings(), ["first"]);
        app.screen.warnings.push("second".to_owned());
        handle(
            &mut app,
            &mut window,
            WindowEvent::Redraw {
                elapsed: Duration::from_millis(16),
            },
        );
        assert_eq!(app.take_warnings(), ["second"]);
        app.screen.warnings.push("third".to_owned());
        handle(&mut app, &mut window, WindowEvent::Text('a'));
        assert_eq!(app.take_warnings(), ["third"]);
        app.screen.warnings.push("fourth".to_owned());
        handle(
            &mut app,
            &mut window,
            WindowEvent::PointerMoved { px: (5.0, 5.0) },
        );
        assert_eq!(app.take_warnings(), ["fourth"]);
    }

    #[test]
    fn losing_focus_releases_the_screens_keys_and_carries_on() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        handle(&mut app, &mut window, key(Key::Right, true));
        assert_eq!(app.screen().releases, 0);
        assert_eq!(
            handle(&mut app, &mut window, WindowEvent::FocusLost),
            Control::Continue
        );
        assert_eq!(app.screen().releases, 1);
        assert_eq!(app.screen().inputs.len(), 1, "no input for it");
    }

    #[test]
    fn closing_the_window_tells_the_screen_and_exits() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert_eq!(
            handle(&mut app, &mut window, WindowEvent::CloseRequested),
            Control::Exit
        );
        assert_eq!(app.screen().quits, 1);
        assert_eq!(app.take_warnings(), ["quit 1"], "what quitting reported");
    }

    #[test]
    fn pointer_events_reach_the_screen_in_logical_units() {
        let mut window = FakeWindow::new((2560, 1536), 2.0);
        let mut app = app(&window);
        // Pillarboxed: content x 256..2304 at scale 2.
        let moved = WindowEvent::PointerMoved { px: (456.0, 100.0) };
        let click = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed: true,
        };
        assert_eq!(handle(&mut app, &mut window, moved), Control::Continue);
        assert_eq!(handle(&mut app, &mut window, click), Control::Continue);
        assert_eq!(
            app.screen().inputs,
            [
                Input::PointerMoved(Point::new(100.0, 50.0)),
                Input::PointerButton {
                    button: MouseButton::Left,
                    pressed: true,
                    at: Point::new(100.0, 50.0)
                },
            ]
        );
    }

    #[test]
    fn pointer_events_in_the_bars_are_dropped() {
        let mut window = FakeWindow::new((2560, 1536), 2.0);
        let mut app = app(&window);
        let click = WindowEvent::PointerButton {
            button: MouseButton::Right,
            pressed: false,
        };
        // No pointer position yet.
        handle(&mut app, &mut window, click);
        handle(
            &mut app,
            &mut window,
            WindowEvent::PointerMoved { px: (10.0, 10.0) },
        );
        handle(&mut app, &mut window, click);
        assert_eq!(app.screen().inputs, []);
    }

    #[test]
    fn a_screen_quit_on_a_pointer_event_exits() {
        struct QuitOnPointer;
        impl Screen for QuitOnPointer {
            fn input(&mut self, _input: &Input) -> ScreenAction {
                ScreenAction::Quit
            }
            fn tick(&mut self, _dt: Duration) {}
            fn draw(&self, _list: &mut DrawList) {}
        }
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = App::new(&window, NoImages, QuitOnPointer);
        let moved = WindowEvent::PointerMoved { px: (10.0, 10.0) };
        let click = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed: true,
        };
        assert_eq!(
            app.handle(moved, &mut window, &mut RecordingGpu::new()),
            Control::Exit
        );
        assert_eq!(
            app.handle(click, &mut window, &mut RecordingGpu::new()),
            Control::Exit
        );
    }

    #[test]
    fn redraws_tick_by_the_time_since_the_last_one() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        for ms in [100, 250, 200, 300] {
            let redraw = WindowEvent::Redraw {
                elapsed: Duration::from_millis(ms),
            };
            assert_eq!(handle(&mut app, &mut window, redraw), Control::Continue);
        }
        // The clock never runs backwards; if it seems to, no time passes.
        assert_eq!(
            app.screen().ticks,
            [100, 150, 0, 50].map(Duration::from_millis)
        );
        assert_eq!(window.redraws, 4);
    }

    #[test]
    fn a_redraw_submits_the_screens_frame_and_reports_failures_once() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        let mut gpu = RecordingGpu::new();
        let redraw = WindowEvent::Redraw {
            elapsed: Duration::ZERO,
        };

        app.handle(redraw, &mut window, &mut gpu);

        assert_eq!(gpu.submits().len(), 1);
        assert_eq!(
            app.take_failures(),
            [(ImageKey::sprite(1, 0), ImageError::Missing)]
        );
        assert_eq!(app.take_failures(), []);
        app.handle(redraw, &mut window, &mut gpu);
        assert_eq!(gpu.submits().len(), 2);
        assert_eq!(app.take_failures(), []);
    }

    fn overlay_app(window: &FakeWindow) -> TestApp {
        let screen = RecordingScreen {
            quit_on: Some(Key::Escape),
            ..RecordingScreen::default()
        };
        App::new(window, NoImages, screen).with_dev_overlay()
    }

    fn routed(app: &mut TestApp, window: &mut FakeWindow, event: WindowEvent) -> Handled {
        app.handle_routed(event, window, &mut RecordingGpu::new())
    }

    fn handled(control: Control, routing: Routing) -> Handled {
        Handled { control, routing }
    }

    const BACKQUOTE: Key = Key::Char('`');

    fn visible(app: &TestApp) -> bool {
        app.dev_overlay().expect("an overlay").visible()
    }

    #[test]
    fn without_an_overlay_backquote_reaches_the_screen() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = app(&window);
        assert!(app.dev_overlay().is_none());
        assert_eq!(
            routed(&mut app, &mut window, key(BACKQUOTE, true)),
            handled(Control::Continue, Routing::Game)
        );
        assert_eq!(app.screen().inputs.len(), 1);
        assert!(!app.overlay_wants(None));
        let game = handled(Control::Continue, Routing::Game);
        assert!(!app.overlay_wants(Some(&game)));
    }

    #[test]
    fn handle_gives_the_routed_control() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        assert_eq!(
            handle(&mut app, &mut window, key(Key::Escape, true)),
            Control::Exit
        );
    }

    #[test]
    fn the_toggle_key_opens_the_overlay_and_lets_go_of_the_screen() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        assert!(!visible(&app), "starts hidden");
        assert_eq!(
            routed(&mut app, &mut window, key(BACKQUOTE, true)),
            handled(Control::Continue, Routing::Toggle)
        );
        assert!(visible(&app));
        assert_eq!(app.screen().inputs, []);
        assert_eq!((app.screen().cancels, app.screen().releases), (1, 1));
    }

    #[test]
    fn the_toggle_keys_repeat_and_release_touch_nothing() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        let repeat = WindowEvent::Key {
            key: BACKQUOTE,
            pressed: true,
            repeat: true,
        };
        for event in [repeat, key(BACKQUOTE, false)] {
            assert_eq!(
                routed(&mut app, &mut window, event),
                handled(Control::Continue, Routing::Toggle)
            );
        }
        assert!(visible(&app));
        assert_eq!(app.screen().inputs, []);
        assert_eq!((app.screen().cancels, app.screen().releases), (1, 1));
    }

    #[test]
    fn keys_go_to_the_overlay_alone_while_it_shows() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        for k in [Key::Escape, Key::Tab, Key::Right, Key::Char('d')] {
            for pressed in [true, false] {
                assert_eq!(
                    routed(&mut app, &mut window, key(k, pressed)),
                    handled(Control::Continue, Routing::Overlay),
                    "{k:?}"
                );
            }
        }
        assert_eq!(app.screen().inputs, []);
        assert!(visible(&app), "Escape does not close it");
    }

    #[test]
    fn typed_text_goes_to_the_overlay_alone_while_it_shows() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        assert_eq!(
            routed(&mut app, &mut window, WindowEvent::Text('a')),
            handled(Control::Continue, Routing::Game)
        );
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        assert_eq!(
            routed(&mut app, &mut window, WindowEvent::Text('b')),
            handled(Control::Continue, Routing::Overlay)
        );
        assert_eq!(app.screen().inputs, [Input::Text('a')]);
    }

    #[test]
    fn closing_the_overlay_releases_nothing_and_gives_keys_back() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        routed(&mut app, &mut window, key(BACKQUOTE, false));
        assert_eq!(
            routed(&mut app, &mut window, key(BACKQUOTE, true)),
            handled(Control::Continue, Routing::Toggle)
        );
        assert!(!visible(&app));
        assert_eq!((app.screen().cancels, app.screen().releases), (1, 1));
        assert_eq!(
            routed(&mut app, &mut window, key(Key::Tab, true)),
            handled(Control::Continue, Routing::Game)
        );
        assert_eq!(
            routed(&mut app, &mut window, key(Key::Escape, true)),
            handled(Control::Exit, Routing::Game)
        );
        assert_eq!(app.screen().inputs.len(), 2);
    }

    #[test]
    fn the_pointer_goes_to_the_overlay_alone_while_it_shows_even_in_the_bars() {
        let mut window = FakeWindow::new((2560, 1536), 2.0);
        let mut app = overlay_app(&window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        let click = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed: true,
        };
        for px in [(456.0, 100.0), (10.0, 10.0)] {
            assert_eq!(
                routed(&mut app, &mut window, WindowEvent::PointerMoved { px }),
                handled(Control::Continue, Routing::Overlay)
            );
            assert_eq!(
                routed(&mut app, &mut window, click),
                handled(Control::Continue, Routing::Overlay)
            );
        }
        assert_eq!(app.screen().inputs, []);
    }

    #[test]
    fn after_closing_a_click_lands_where_the_pointer_is() {
        let mut window = FakeWindow::new((2560, 1536), 2.0);
        let mut app = overlay_app(&window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        routed(
            &mut app,
            &mut window,
            WindowEvent::PointerMoved { px: (456.0, 100.0) },
        );
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        let click = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed: true,
        };
        assert_eq!(
            routed(&mut app, &mut window, click),
            handled(Control::Continue, Routing::Game)
        );
        assert_eq!(
            app.screen().inputs,
            [Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at: Point::new(100.0, 50.0)
            }]
        );
    }

    #[test]
    fn redraws_feed_the_overlays_frame_times_hidden_or_visible() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        let redraw = |ms| WindowEvent::Redraw {
            elapsed: Duration::from_millis(ms),
        };
        assert_eq!(
            routed(&mut app, &mut window, redraw(0)),
            handled(Control::Continue, Routing::Game)
        );
        routed(&mut app, &mut window, redraw(16));
        let times = app.dev_overlay().expect("an overlay").frame_times();
        assert_eq!(times.count(), 1);
        assert_eq!(times.last(), Some(Duration::from_millis(16)));
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        assert_eq!(
            routed(&mut app, &mut window, redraw(33)),
            handled(Control::Continue, Routing::Game)
        );
        let times = app.dev_overlay().expect("an overlay").frame_times();
        assert_eq!(times.count(), 2);
        assert_eq!(times.last(), Some(Duration::from_millis(17)));
        assert_eq!(app.screen().ticks.len(), 3, "the screen still ticks");
        assert_eq!(window.redraws, 3);
    }

    #[test]
    fn other_events_go_to_the_game() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        let resized = WindowEvent::Resized {
            size_px: (1280, 768),
            scale_factor: 1.0,
        };
        assert_eq!(
            routed(&mut app, &mut window, resized),
            handled(Control::Continue, Routing::Game)
        );
        assert_eq!(
            routed(&mut app, &mut window, WindowEvent::FocusLost),
            handled(Control::Continue, Routing::Game)
        );
        assert_eq!(
            routed(&mut app, &mut window, WindowEvent::CloseRequested),
            handled(Control::Exit, Routing::Game)
        );
    }

    #[test]
    fn the_overlay_wants_every_event_but_its_toggle_while_it_shows() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let mut app = overlay_app(&window);
        let overlay = handled(Control::Continue, Routing::Overlay);
        let game = handled(Control::Continue, Routing::Game);
        let toggle = handled(Control::Continue, Routing::Toggle);
        for event in [None, Some(&overlay), Some(&game), Some(&toggle)] {
            assert!(!app.overlay_wants(event), "hidden: {event:?}");
        }
        routed(&mut app, &mut window, key(BACKQUOTE, true));
        assert!(app.overlay_wants(None));
        assert!(app.overlay_wants(Some(&overlay)));
        assert!(app.overlay_wants(Some(&game)));
        assert!(!app.overlay_wants(Some(&toggle)));
    }

    // Audio.

    /// Shows `showing`, which Tab changes to the ship browser; sounds a
    /// button going down on each input and coming up on each tick.
    struct SoundingScreen {
        showing: Showing,
        sounds: Vec<Sound>,
    }

    impl Screen for SoundingScreen {
        fn input(&mut self, input: &Input) -> ScreenAction {
            if let Input::Key { key: Key::Tab, .. } = input {
                self.showing = Showing::ShipBrowser;
            }
            self.sounds.push(Sound::Ui(UiSound::ButtonDown));
            ScreenAction::None
        }

        fn tick(&mut self, _dt: Duration) {
            self.sounds.push(Sound::Ui(UiSound::ButtonUp));
        }

        fn draw(&self, _list: &mut DrawList) {}

        fn take_sounds(&mut self) -> Vec<Sound> {
            std::mem::take(&mut self.sounds)
        }

        fn now_showing(&self) -> Option<Showing> {
            Some(self.showing)
        }
    }

    fn play(id: i16) -> AudioCommand {
        AudioCommand::Play {
            sound: nova_data::SoundId(id),
            volume: Volume::FULL,
        }
    }

    const MUSIC: AudioCommand = AudioCommand::StartMusic {
        volume: Volume::FULL,
    };

    /// An app over a screen on the galaxy map, with audio recorded.
    fn sounding(window: &FakeWindow) -> (App<NoImages, SoundingScreen>, AudioLog) {
        let screen = SoundingScreen {
            showing: Showing::GalaxyMap,
            sounds: vec![Sound::Sim(SimSound::JumpBegan)],
        };
        let audio = RecordingAudio::new();
        let log = audio.log();
        let core = AudioCore::new(Box::new(audio) as Box<dyn Audio>);
        (App::new(window, NoImages, screen).with_audio(core), log)
    }

    #[test]
    fn attaching_audio_takes_in_the_screen_and_its_sounds_at_once() {
        let window = FakeWindow::new((1024, 768), 1.0);
        let (app, log) = sounding(&window);
        assert_eq!(*log.borrow(), [MUSIC, play(128)]);
        assert_eq!(app.screen().sounds, [], "taken");
    }

    #[test]
    fn each_input_and_redraw_feeds_the_screens_sounds_to_the_audio() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let (mut app, log) = sounding(&window);
        log.borrow_mut().clear();
        let mut gpu = RecordingGpu::new();
        app.handle(key(Key::Up, true), &mut window, &mut gpu);
        assert_eq!(*log.borrow(), [play(600)]);
        app.handle(
            WindowEvent::Redraw {
                elapsed: Duration::from_millis(16),
            },
            &mut window,
            &mut gpu,
        );
        assert_eq!(*log.borrow(), [play(600), play(601)]);
        app.handle(
            WindowEvent::PointerMoved { px: (5.0, 5.0) },
            &mut window,
            &mut gpu,
        );
        assert_eq!(log.borrow().len(), 3, "pointer input too");
        app.handle(key(Key::Tab, true), &mut window, &mut gpu);
        assert_eq!(
            log.borrow()[3..],
            [AudioCommand::StopMusic, play(600)],
            "the screen shown, then its sounds"
        );
    }

    #[test]
    fn without_audio_the_screens_sounds_are_let_go() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let screen = SoundingScreen {
            showing: Showing::GalaxyMap,
            sounds: Vec::new(),
        };
        let mut app = App::new(&window, NoImages, screen);
        let mut gpu = RecordingGpu::new();
        app.handle(key(Key::Up, true), &mut window, &mut gpu);
        assert_eq!(app.screen().sounds, [], "not left to pile up");
        let redraw = WindowEvent::Redraw {
            elapsed: Duration::from_millis(16),
        };
        app.handle(redraw, &mut window, &mut gpu);
        assert_eq!(app.screen().sounds, []);
    }

    // Settings.

    /// Reports `prefs` once, after the next input.
    struct PrefsScreen {
        prefs: Vec<SoundPrefs>,
        pending: Option<SoundPrefs>,
    }

    impl Screen for PrefsScreen {
        fn input(&mut self, _input: &Input) -> ScreenAction {
            if !self.prefs.is_empty() {
                self.pending = Some(self.prefs.remove(0));
            }
            ScreenAction::None
        }

        fn tick(&mut self, _dt: Duration) {}

        fn draw(&self, _list: &mut DrawList) {}

        fn take_sound_prefs(&mut self) -> Option<SoundPrefs> {
            self.pending.take()
        }

        fn now_showing(&self) -> Option<Showing> {
            Some(Showing::Preferences)
        }
    }

    fn prefs_app(window: &FakeWindow, prefs: &[SoundPrefs]) -> App<NoImages, PrefsScreen> {
        let screen = PrefsScreen {
            prefs: prefs.to_vec(),
            pending: None,
        };
        App::new(window, NoImages, screen)
    }

    fn keeper(store: &MemorySettings) -> SettingsKeeper<Box<dyn SettingsStore>> {
        let (keeper, warning) =
            SettingsKeeper::open(Box::new(store.clone()) as Box<dyn SettingsStore>);
        assert_eq!(warning, None);
        keeper
    }

    const MUSIC_OFF: SoundPrefs = SoundPrefs {
        sound: true,
        music: false,
        effects_level: 7,
        music_level: 7,
    };

    #[test]
    fn a_change_of_prefs_plays_and_is_saved() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let audio = RecordingAudio::new();
        let log = audio.log();
        let core = AudioCore::new(Box::new(audio) as Box<dyn Audio>);
        let store = MemorySettings::new();
        let quieter = SoundPrefs {
            effects_level: 4,
            ..MUSIC_OFF
        };
        let mut app = prefs_app(&window, &[MUSIC_OFF, quieter])
            .with_audio(core)
            .with_settings(keeper(&store));
        let mut gpu = RecordingGpu::new();
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        assert_eq!(*log.borrow(), [], "music was not playing");
        let (saved, _) = SettingsKeeper::open(store.clone());
        assert!(!saved.settings().music);
        assert_eq!(store.writes(), 1);
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        assert_eq!(store.writes(), 2);
        let (saved, _) = SettingsKeeper::open(store.clone());
        assert_eq!(saved.settings().effects_volume, Volume::new(4.0 / 7.0));
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        assert_eq!(store.writes(), 2, "no change, no save");
        assert_eq!(app.take_warnings(), Vec::<String>::new());
    }

    /// With no keeper: the change still plays.
    #[test]
    fn a_change_reaches_the_audio_core_and_what_it_plays() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let audio = RecordingAudio::new();
        let log = audio.log();
        let core = AudioCore::new(Box::new(audio) as Box<dyn Audio>);
        let screen = SoundingScreen {
            showing: Showing::GalaxyMap,
            sounds: Vec::new(),
        };
        let mut app = App::new(
            &window,
            NoImages,
            PrefsAndSounds {
                screen,
                prefs: Some(MUSIC_OFF),
            },
        )
        .with_audio(core);
        log.borrow_mut().clear();
        let mut gpu = RecordingGpu::new();
        app.handle(key(Key::Up, true), &mut window, &mut gpu);
        assert_eq!(
            *log.borrow(),
            [AudioCommand::StopMusic, play(600)],
            "the change, then the sounds"
        );
    }

    /// A sounding screen that also reports `prefs` once, after an input.
    struct PrefsAndSounds {
        screen: SoundingScreen,
        prefs: Option<SoundPrefs>,
    }

    impl Screen for PrefsAndSounds {
        fn input(&mut self, input: &Input) -> ScreenAction {
            self.screen.input(input)
        }

        fn tick(&mut self, dt: Duration) {
            self.screen.tick(dt);
        }

        fn draw(&self, list: &mut DrawList) {
            self.screen.draw(list);
        }

        fn take_sounds(&mut self) -> Vec<Sound> {
            self.screen.take_sounds()
        }

        fn take_sound_prefs(&mut self) -> Option<SoundPrefs> {
            if self.screen.sounds.is_empty() {
                None
            } else {
                self.prefs.take()
            }
        }

        fn now_showing(&self) -> Option<Showing> {
            self.screen.now_showing()
        }
    }

    #[test]
    fn without_audio_the_settings_still_save() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::new();
        let mut app = prefs_app(&window, &[MUSIC_OFF]).with_settings(keeper(&store));
        app.handle(key(Key::Space, true), &mut window, &mut RecordingGpu::new());
        let (saved, _) = SettingsKeeper::open(store);
        assert!(!saved.settings().music);
    }

    #[test]
    fn each_failed_save_is_one_warning_until_taken() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::new();
        let quieter = SoundPrefs {
            effects_level: 1,
            ..MUSIC_OFF
        };
        let mut app = prefs_app(&window, &[MUSIC_OFF, quieter, SoundPrefs::default()])
            .with_settings(keeper(&store));
        store.fail_writes(true);
        let mut gpu = RecordingGpu::new();
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        let message = "nova: cannot save the settings: the disk is full".to_owned();
        assert_eq!(app.take_warnings(), [message.clone(), message]);
        assert_eq!(app.take_warnings(), Vec::<String>::new(), "taken");
        store.fail_writes(false);
        app.handle(key(Key::Space, true), &mut window, &mut gpu);
        assert_eq!(app.take_warnings(), Vec::<String>::new(), "saved");
        assert_eq!(store.writes(), 1);
    }

    #[test]
    fn a_change_reported_on_a_redraw_is_taken_too() {
        let mut window = FakeWindow::new((1024, 768), 1.0);
        let store = MemorySettings::new();
        let mut app = prefs_app(&window, &[]).with_settings(keeper(&store));
        app.screen.pending = Some(MUSIC_OFF);
        app.handle(
            WindowEvent::Redraw {
                elapsed: Duration::from_millis(16),
            },
            &mut window,
            &mut RecordingGpu::new(),
        );
        assert_eq!(store.writes(), 1);
    }
}
