//! The winit event loop's handler: opens the window and the GPU surface,
//! then forwards every event to the [`App`].
//!
//! On macOS, with `Runner::with_menu_proxy`, it installs the game's menu
//! bar (`nova_appkit::MenuBar`) and its Apple event handlers
//! (`nova_appkit::AppleEvents`) once the window opens: each pick, and the
//! quit and Preferences events the system sends (the Dock's Quit, logout,
//! restart, shutdown, `osascript`'s `quit app`), comes back through the
//! event loop as a [`MenuCommand`] user event, which the app takes as
//! [`WindowEvent::Menu`], and after every event the menu bar is told what
//! the screen now offers (`MenuSync`). The quit event is the menu's Quit,
//! so it quits as Command-Q does, saving the pilot flying.
//!
//! The event loop's end (`ApplicationHandler::exiting`) is the backstop
//! for a quit that never reached the game: on macOS a `terminate:` sent
//! before the handlers are installed, or by something else, which ends the
//! process from `applicationWillTerminate:`. It tells the app to quit
//! ([`App::exiting`]), so the pilot is saved, and prints what the app
//! reports. winit ends every run through it, so it follows the game's own
//! quits too, and then does nothing more.
//!
//! With [`Runner::with_audio`], the app it opens plays sound, with
//! [`Runner::with_settings`] it saves the player's settings, and with
//! [`Runner::with_diagnostics`] it writes out the screen's diagnostics
//! about the game data; the runner prints the warnings the app keeps, such as a
//! failed save.
//!
//! With [`Runner::with_report`], the warnings of the window's opening (a
//! saved mode or Scaling it cannot restore, a menu bar it cannot install)
//! go to the startup [`Report`] instead, and once the window is up and the
//! loop is waiting, the report's warnings alert, if any, is shown as a sheet
//! over the window (`dialogs::show_warnings`). Later warnings are the
//! game's, not startup's, and stay on stderr.
//!
//! With the `dev-tools` feature and `Runner::with_dev_tools`, it also
//! drives the developer tools: while their overlay shows, each redraw runs
//! the egui panel first, over the pilot flying (`App::pilot_desk_frame`),
//! and submits the frame with the panel painted over it, and the raw
//! window events the app says the overlay wants go to egui.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "dev-tools")]
use nova_data::GameData;
#[cfg(feature = "dev-tools")]
use nova_render::wgpu::WithOverlay;
#[cfg(feature = "dev-tools")]
use nova_view::devtools::DevOverlay;

use nova_audio::{Audio, AudioCore};
use nova_render::wgpu::{InitError, SurfaceGpu};
use nova_render::{FontFaces, ImageSource, Upscaler};
use nova_view::MenuCommand;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::WindowEvent as WinitEvent;
#[cfg(target_os = "macos")]
use winit::event_loop::EventLoopProxy;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use super::window::WinitWindow;
use super::{dialogs, text_event, window_events};
#[cfg(target_os = "macos")]
use crate::app::MenuSync;
use crate::app::{App, AppScreen, Control, Handled, WindowEvent};
#[cfg(feature = "dev-tools")]
use crate::devtools::DevTools;
use crate::exit::OpenFailure;
use crate::settings::GameKeeper;
use crate::startup::Report;

/// The window, its GPU surface and the app, once the window is open.
struct Running<S> {
    app: App<S>,
    // Dropped before the window it draws into.
    gpu: SurfaceGpu,
    #[cfg(feature = "dev-tools")]
    dev_tools: Option<DevTools>,
    /// The menu bar, kept for the whole run (its items' target lives in
    /// it), and what it was last told; none until it is installed.
    #[cfg(target_os = "macos")]
    menu: Option<(nova_appkit::MenuBar, MenuSync)>,
    /// The quit and Preferences Apple event handlers, kept for the whole
    /// run (their target lives in them); none until they are installed.
    #[cfg(target_os = "macos")]
    apple_events: Option<nova_appkit::AppleEvents>,
    window: WinitWindow,
}

/// Runs the app in a winit event loop.
pub struct Runner<S> {
    pending: Option<(S, AppScreen, FontFaces)>,
    running: Option<Running<S>>,
    failure: Option<OpenFailure>,
    start: Instant,
    #[cfg(feature = "dev-tools")]
    dev_catalog: Option<Rc<GameData>>,
    /// The audio core, until the app takes it.
    audio: Option<AudioCore<Box<dyn Audio>>>,
    /// The settings keeper, until the app takes it.
    settings: Option<GameKeeper>,
    /// Where the diagnostics are written, until the app takes it.
    diagnostics: Option<Box<dyn Write>>,
    /// Where the menu bar sends its picks, until it is installed.
    #[cfg(target_os = "macos")]
    menu_proxy: Option<EventLoopProxy<MenuCommand>>,
    /// The startup report, shared with `main`, which reports the run's
    /// end through it.
    report: Option<Rc<RefCell<Report>>>,
}

impl<S: ImageSource> Runner<S> {
    /// A runner that will open a window showing `screen` with images from
    /// `images` and text in `fonts` when the event loop starts.
    pub fn new(images: S, screen: AppScreen, fonts: FontFaces) -> Self {
        Self {
            pending: Some((images, screen, fonts)),
            running: None,
            failure: None,
            start: Instant::now(),
            #[cfg(feature = "dev-tools")]
            dev_catalog: None,
            audio: None,
            settings: None,
            diagnostics: None,
            #[cfg(target_os = "macos")]
            menu_proxy: None,
            report: None,
        }
    }

    /// The runner with the startup `report`: the window's opening warnings
    /// go to it, and its warnings alert is shown over the window once it
    /// is up.
    #[must_use]
    pub fn with_report(mut self, report: Rc<RefCell<Report>>) -> Self {
        self.report = Some(report);
        self
    }

    /// The runner with the macOS menu bar: once the window opens, it
    /// installs it, sending each pick through `proxy` as a user event.
    #[cfg(target_os = "macos")]
    #[must_use]
    pub fn with_menu_proxy(mut self, proxy: EventLoopProxy<MenuCommand>) -> Self {
        self.menu_proxy = Some(proxy);
        self
    }

    /// The runner with the developer tools, browsing `catalog`; backquote
    /// shows and hides them.
    #[cfg(feature = "dev-tools")]
    #[must_use]
    pub fn with_dev_tools(mut self, catalog: Rc<GameData>) -> Self {
        self.dev_catalog = Some(catalog);
        self
    }

    /// The runner with sound: the app it opens plays through `core`.
    #[must_use]
    pub fn with_audio(mut self, core: AudioCore<Box<dyn Audio>>) -> Self {
        self.audio = Some(core);
        self
    }

    /// The runner with the player's settings kept by `keeper`: the app it
    /// opens saves each change through it.
    #[must_use]
    pub fn with_settings(mut self, keeper: GameKeeper) -> Self {
        self.settings = Some(keeper);
        self
    }

    /// The runner with the screen's diagnostics written as lines
    /// through `out` by the app it opens.
    #[must_use]
    pub fn with_diagnostics(mut self, out: Box<dyn Write>) -> Self {
        self.diagnostics = Some(out);
        self
    }

    /// Why the window could not be opened, if it could not; the event loop
    /// exits as soon as that happens.
    pub fn open_failure(&self) -> Option<&OpenFailure> {
        self.failure.as_ref()
    }
}

/// Reports the startup warning `line` to `report`, or prints it without
/// one.
fn startup_warning(report: Option<&Rc<RefCell<Report>>>, line: &str) {
    match report {
        Some(report) => report.borrow_mut().warn(line),
        None => eprintln!("{line}"),
    }
}

/// The open failure for a GPU that could not be set up for the window.
fn gpu_failure(error: &InitError) -> OpenFailure {
    let detail = error.to_string();
    match error {
        InitError::NoAdapter(_) | InitError::NoDevice(_) => OpenFailure::NoGpu(detail),
        InitError::Surface(_) | InitError::NoSurfaceFormat => OpenFailure::Surface(detail),
    }
}

/// Opens the 1024x768-point, resizable "EV Nova" window and its surface,
/// drawing text in `fonts`.
fn open(
    event_loop: &ActiveEventLoop,
    fonts: &FontFaces,
) -> Result<(WinitWindow, SurfaceGpu), OpenFailure> {
    let attributes = Window::default_attributes()
        .with_title("EV Nova")
        .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0))
        .with_resizable(true);
    #[cfg(windows)]
    let attributes = super::window::with_exe_icon(attributes);
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    let attributes = super::window::with_app_id(attributes);
    let window = event_loop
        .create_window(attributes)
        .map_err(|error| OpenFailure::Window(error.to_string()))?;
    let window = Arc::new(window);
    // Flight's world drawn natively until the app restores the Scaling the
    // settings choose (`App::restore_scaling`), which needs this device to
    // say which upscalers it can use.
    let gpu = SurfaceGpu::new(
        Box::new(event_loop.owned_display_handle()),
        window.clone(),
        fonts,
        Upscaler::Native,
    )
    .map_err(|error| gpu_failure(&error))?;
    Ok((WinitWindow(window), gpu))
}

/// The event loop the game runs on, carrying the menu bar's picks as user
/// events.
///
/// On macOS it has no winit menu. winit 0.30's
/// (`platform_impl/macos/menu.rs`) gives its Quit item, Command-Q, the
/// action `terminate:`, which `AppKit` handles before the window sees the
/// key: the process ends through `applicationWillTerminate:` with no close
/// request and no key event, so the screen never hears of it and the
/// pilot flying is not saved. The game's own menu bar comes from
/// `nova_appkit` instead, installed in `resumed` once the window opens:
/// its Quit sends the game's quit (Command-Q's, which saves), never
/// `terminate:`. So do the Dock's Quit, logout and `osascript`'s `quit
/// app`: their quit Apple event, which `AppKit` would answer with
/// `terminate:`, is handled by `nova_appkit::AppleEvents`, installed with
/// the menu bar, and sends the menu's Quit. A `terminate:` that still
/// comes (a quit event before the handlers are installed) ends the loop
/// through `ApplicationHandler::exiting`, which saves the pilot. A quit
/// event that arrives while an `AppKit` modal runs (the startup warnings
/// sheet, a file dialog) is taken at once, but its command waits in the
/// loop until the modal ends, as Command-Q's does. It needs the OS, so no
/// test runs it.
pub fn event_loop() -> Result<EventLoop<MenuCommand>, EventLoopError> {
    let mut builder = EventLoop::<MenuCommand>::with_user_event();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        builder.with_default_menu(false);
    }
    builder.build()
}

impl<S: ImageSource> ApplicationHandler<MenuCommand> for Runner<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some((images, screen, fonts)) = self.pending.take() else {
            return;
        };
        match open(event_loop, &fonts) {
            Ok((mut window, mut gpu)) => {
                let app = App::new(&window, images, screen);
                let app = match self.diagnostics.take() {
                    Some(out) => app.with_diagnostics(out),
                    None => app,
                };
                let app = match self.audio.take() {
                    Some(core) => app.with_audio(core),
                    None => app,
                };
                let mut app = match self.settings.take() {
                    Some(keeper) => app.with_settings(keeper),
                    None => app,
                };
                // Fullscreen if the player quit fullscreen; a saved mode
                // this monitor lacks says so at startup. So does a saved
                // Scaling this device cannot use.
                app.restore_window(&mut window);
                app.restore_scaling(&mut gpu);
                for warning in app.take_warnings() {
                    startup_warning(self.report.as_ref(), &warning);
                }
                #[cfg(feature = "dev-tools")]
                let (app, dev_tools) = match self.dev_catalog.take() {
                    Some(catalog) => (
                        app.with_dev_overlay(),
                        Some(DevTools::new(&window.0, catalog)),
                    ),
                    None => (app, None),
                };
                crate::app::WindowPort::request_redraw(&mut window);
                let mut running = Running {
                    app,
                    gpu,
                    #[cfg(feature = "dev-tools")]
                    dev_tools,
                    #[cfg(target_os = "macos")]
                    menu: None,
                    #[cfg(target_os = "macos")]
                    apple_events: None,
                    window,
                };
                #[cfg(target_os = "macos")]
                if let Some(proxy) = self.menu_proxy.take() {
                    for warning in running.install_menu(&proxy) {
                        startup_warning(self.report.as_ref(), &warning);
                    }
                }
                running.sync_menu();
                self.running = Some(running);
            }
            Err(failure) => {
                self.failure = Some(failure);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WinitEvent) {
        let Some(running) = &mut self.running else {
            return;
        };
        let raw = event;
        // The fullscreen report goes first (see `window_events`); what the
        // event itself did decides whether the overlay sees it raw.
        let events: Vec<WindowEvent> =
            window_events(&raw, &running.window, self.start.elapsed()).collect();
        let mut handled = None;
        let mut exit = false;
        for event in events {
            let this = running.handle(event);
            exit |= this.control == Control::Exit;
            if !matches!(event, WindowEvent::FullscreenChanged { .. }) {
                handled = Some(this);
            }
        }
        // The text a key press types follows the key.
        let typed = match &raw {
            WinitEvent::KeyboardInput { event, .. } => {
                text_event(event.text.as_deref(), event.state)
            }
            _ => None,
        };
        if let Some(event) = typed {
            exit |= running.handle(event).control == Control::Exit;
        }
        #[cfg(feature = "dev-tools")]
        if let Some(dev_tools) = &mut running.dev_tools
            && running.app.overlay_wants(handled.as_ref())
        {
            dev_tools.on_window_event(&running.window.0, &raw);
        }
        #[cfg(not(feature = "dev-tools"))]
        let _ = handled;
        running.after_events(event_loop, exit);
    }

    /// Once the window is up and the application running: the startup
    /// warnings, if any, as a sheet over the window. The report gives them
    /// once, and only without a terminal.
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let (Some(running), Some(report)) = (&self.running, &self.report)
            && let Some(body) = report.borrow_mut().take_alert()
        {
            dialogs::show_warnings(&running.window.0, &body);
        }
    }

    /// A menu bar pick: the app takes it as [`WindowEvent::Menu`].
    fn user_event(&mut self, event_loop: &ActiveEventLoop, command: MenuCommand) {
        let Some(running) = &mut self.running else {
            return;
        };
        let exit = running.handle(WindowEvent::Menu(command)).control == Control::Exit;
        running.after_events(event_loop, exit);
    }

    /// The event loop is ending: tells the app, which quits unless it
    /// already has (saving the pilot flying), and prints what it reports.
    ///
    /// winit ends every run here, after the game's own quit too, when the
    /// app has nothing more to do. On macOS a `terminate:` that bypassed
    /// the game also ends here, inside `applicationWillTerminate:`, and
    /// `AppKit` ends the process right after: `main`'s report never runs,
    /// so a failed save goes to stderr only.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(running) = &mut self.running else {
            return;
        };
        running.app.exiting(&mut running.window);
        running.print_reports();
    }
}

impl<S: ImageSource> Running<S> {
    /// After the events of one winit event: prints the image failures and
    /// the warnings the app kept, tells the menu bar what the screen now
    /// offers, and ends the event loop when the app exits.
    fn after_events(&mut self, event_loop: &ActiveEventLoop, exit: bool) {
        self.print_reports();
        self.sync_menu();
        if exit {
            event_loop.exit();
        }
    }

    /// Prints the image failures and the warnings the app kept.
    fn print_reports(&mut self) {
        for (key, error) in self.app.take_failures() {
            eprintln!("nova: image {key:?}: {error}");
        }
        for warning in self.app.take_warnings() {
            eprintln!("{warning}");
        }
    }

    /// Installs the menu bar and the Apple event handlers, their picks and
    /// events sent through `proxy`. One that can't be installed is a
    /// warning, given back, and the game runs without it.
    #[cfg(target_os = "macos")]
    fn install_menu(&mut self, proxy: &EventLoopProxy<MenuCommand>) -> Vec<String> {
        // A closed loop drops the command: the app is exiting.
        let send = |proxy: EventLoopProxy<MenuCommand>| {
            move |command| {
                let _ = proxy.send_event(super::menu_bar::menu_command(command));
            }
        };
        let mut warnings = Vec::new();
        match nova_appkit::MenuBar::install("EV Nova", send(proxy.clone())) {
            Ok(bar) => self.menu = Some((bar, MenuSync::new())),
            Err(error) => warnings.push(format!("nova: no menu bar: {error}")),
        }
        match nova_appkit::AppleEvents::install(send(proxy.clone())) {
            Ok(events) => self.apple_events = Some(events),
            Err(error) => warnings.push(format!("nova: no quit event handler: {error}")),
        }
        warnings
    }

    /// Tells the menu bar, if any, what the screen now offers.
    #[cfg_attr(not(target_os = "macos"), allow(clippy::unused_self))]
    fn sync_menu(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some((bar, sync)) = &mut self.menu {
            sync.sync(self.app.screen(), bar);
        }
    }

    /// Hands the app one event. A redraw while the developer overlay shows
    /// first runs the egui panel over the pilot flying, catching the game
    /// up after an edit, then submits through a GPU that paints it over
    /// the frame.
    fn handle(&mut self, event: WindowEvent) -> Handled {
        #[cfg(feature = "dev-tools")]
        if let (WindowEvent::Redraw { .. }, Some(dev_tools)) = (event, &mut self.dev_tools)
            && self.app.dev_overlay().is_some_and(DevOverlay::visible)
        {
            let window = &self.window.0;
            self.app
                .pilot_desk_frame(|overlay, desk| dev_tools.run_frame(window, overlay, desk));
            let mut gpu = WithOverlay {
                gpu: &mut self.gpu,
                painter: dev_tools.layer_mut(),
            };
            return self.app.handle_routed(event, &mut self.window, &mut gpu);
        }
        self.app
            .handle_routed(event, &mut self.window, &mut self.gpu)
    }
}

#[cfg(test)]
mod tests {
    use nova_data::graphics::Image;
    use nova_render::ImageError;
    use nova_render::wgpu::InitError;
    use nova_view::ImageKind;

    use std::io;
    use std::path::Path;
    use std::rc::Rc;

    use nova_data::GameData;
    use nova_data::store::fs::{DirLister, Listing};
    use nova_rsrc::{Fork, ForkReader};

    use nova_audio::recording::{MemorySettings, RecordingAudio};
    use nova_audio::{AudioCommand, AudioSettings, SettingsStore};

    use crate::settings::{GameSettings, SettingsKeeper};
    use nova_view::Showing;

    use super::*;
    use crate::app::start_screen;
    use crate::exit::OpenFailure;

    struct NoImages;

    impl ImageSource for NoImages {
        fn frames(&self, _kind: ImageKind, _id: i16) -> Result<Vec<Image>, ImageError> {
            Err(ImageError::Missing)
        }
    }

    /// A data directory with no files in it.
    struct NoFiles;

    impl DirLister for NoFiles {
        fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
            Ok(Vec::new())
        }
    }

    impl ForkReader for NoFiles {
        fn read_fork(&self, _path: &Path, _fork: Fork) -> io::Result<Option<Vec<u8>>> {
            Ok(None)
        }
    }

    fn no_data() -> Rc<GameData> {
        Rc::new(GameData::load(&NoFiles, &NoFiles, Path::new("/data"), None).expect("opens"))
    }

    #[test]
    fn a_gpu_with_no_adapter_or_device_is_no_gpu() {
        let error = InitError::NoAdapter(wgpu::RequestAdapterError::EnvNotSet);
        assert_eq!(gpu_failure(&error), OpenFailure::NoGpu(error.to_string()));
    }

    #[test]
    fn a_gpu_that_cannot_use_the_surface_is_a_surface_failure() {
        let error = InitError::NoSurfaceFormat;
        assert_eq!(gpu_failure(&error), OpenFailure::Surface(error.to_string()));
    }

    #[test]
    fn the_runner_keeps_the_fonts_for_the_window_it_opens() {
        let fonts = FontFaces::new(&b"fallback"[..]).with_charcoal(&b"charcoal"[..]);
        let runner = Runner::new(NoImages, start_screen(no_data()), fonts.clone());
        let (_, _, pending) = runner.pending.as_ref().expect("not opened yet");
        assert_eq!(pending, &fonts);
    }

    #[test]
    fn the_runner_reports_the_failure_that_stopped_the_window_opening() {
        let mut runner = Runner::new(NoImages, start_screen(no_data()), FontFaces::bundled());
        assert_eq!(runner.open_failure(), None);
        runner.failure = Some(OpenFailure::Window("no display".into()));
        assert_eq!(
            runner.open_failure(),
            Some(&OpenFailure::Window("no display".into()))
        );
    }

    #[test]
    fn the_runner_keeps_the_audio_for_the_app_it_opens() {
        let runner = Runner::new(NoImages, start_screen(no_data()), FontFaces::bundled());
        assert!(runner.audio.is_none(), "silent unless given audio");
        let audio = RecordingAudio::new();
        let log = audio.log();
        let core = AudioCore::new(Box::new(audio) as Box<dyn Audio>);
        let mut runner = runner.with_audio(core);
        let core = runner.audio.as_mut().expect("kept");
        core.update(Some(Showing::MainMenu), &[]);
        assert_eq!(
            *log.borrow(),
            [AudioCommand::StartMusic {
                volume: AudioSettings::default().music_volume
            }]
        );
    }

    #[test]
    fn the_runner_keeps_the_settings_for_the_app_it_opens() {
        let runner = Runner::new(NoImages, start_screen(no_data()), FontFaces::bundled());
        assert!(runner.settings.is_none(), "unsaved unless given a keeper");
        let store = MemorySettings::new();
        let (keeper, _) = SettingsKeeper::open(Box::new(store.clone()) as Box<dyn SettingsStore>);
        let mut runner = runner.with_settings(keeper);
        let keeper = runner.settings.as_mut().expect("kept");
        let quiet = GameSettings {
            audio: AudioSettings {
                sound: false,
                ..AudioSettings::default()
            },
            hyperspace_effects: false,
            fullscreen: false,
            fullscreen_kind: crate::settings::FullscreenSetting::Borderless,
            quicktime_movies: true,
            map_borders: true,
            smoke_trails: true,
            weapon_effects: true,
            parallax_starfield: true,
            frame_upscaler: crate::settings::ScalingSetting::default(),
            nova_files: None,
        };
        keeper.change(quiet).expect("saves");
        assert_eq!(store.writes(), 1);
    }

    struct NoTerminal;

    impl crate::startup::Terminal for NoTerminal {
        fn stderr_is_terminal(&self) -> bool {
            false
        }
    }

    #[test]
    fn the_runner_reports_its_startup_warnings_through_the_report_it_keeps() {
        let runner = Runner::new(NoImages, start_screen(no_data()), FontFaces::bundled());
        assert!(runner.report.is_none(), "printed unless given a report");
        let report = Rc::new(RefCell::new(Report::new(
            &NoTerminal,
            None,
            Box::new(io::sink()),
        )));
        let runner = runner.with_report(Rc::clone(&report));
        startup_warning(runner.report.as_ref(), "nova: no menu bar: none");
        assert_eq!(
            report.borrow_mut().take_alert(),
            Some("nova: no menu bar: none".to_owned())
        );
    }

    #[test]
    fn the_runner_keeps_the_diagnostics_writer_for_the_app_it_opens() {
        let runner = Runner::new(NoImages, start_screen(no_data()), FontFaces::bundled());
        assert!(runner.diagnostics.is_none(), "let go unless given a writer");
        let mut runner = runner.with_diagnostics(Box::new(Vec::new()));
        let out = runner.diagnostics.as_mut().expect("kept");
        assert_eq!(out.write(b"x").expect("writes"), 1);
    }

    #[cfg(feature = "dev-tools")]
    #[test]
    fn the_developer_tools_browse_the_data_they_are_given() {
        let data = no_data();
        let runner = Runner::new(NoImages, start_screen(no_data()), FontFaces::bundled());
        assert!(runner.dev_catalog.is_none(), "off unless asked for");
        let runner = runner.with_dev_tools(Rc::clone(&data));
        assert!(
            runner
                .dev_catalog
                .as_ref()
                .is_some_and(|catalog| Rc::ptr_eq(catalog, &data))
        );
    }
}
