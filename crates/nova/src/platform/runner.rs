//! The winit event loop's handler: opens the window and the GPU surface,
//! then forwards every event to the [`App`].
//!
//! With [`Runner::with_audio`], the app it opens plays sound, and with
//! [`Runner::with_settings`] it saves the player's settings; the runner
//! prints the warnings the app keeps, such as a failed save.
//!
//! With the `dev-tools` feature and [`Runner::with_dev_tools`], it also
//! drives the developer tools: while their overlay shows, each redraw runs
//! the egui panel first and submits the frame with the panel painted over
//! it, and the raw window events the app says the overlay wants go to egui.

#[cfg(feature = "dev-tools")]
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

#[cfg(feature = "dev-tools")]
use nova_data::GameData;
#[cfg(feature = "dev-tools")]
use nova_render::wgpu::WithOverlay;

use nova_audio::{Audio, AudioCore, SettingsKeeper, SettingsStore};
use nova_render::wgpu::{InitError, SurfaceGpu};
use nova_render::{FontFaces, ImageSource};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent as WinitEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use super::translate;
use super::window::WinitWindow;
use crate::app::{App, AppScreen, Control, Handled, WindowEvent};
#[cfg(feature = "dev-tools")]
use crate::devtools::DevTools;
use crate::exit::OpenFailure;

/// The window, its GPU surface and the app, once the window is open.
struct Running<S> {
    app: App<S>,
    // Dropped before the window it draws into.
    gpu: SurfaceGpu,
    #[cfg(feature = "dev-tools")]
    dev_tools: Option<DevTools>,
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
    settings: Option<SettingsKeeper<Box<dyn SettingsStore>>>,
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
        }
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
    pub fn with_settings(mut self, keeper: SettingsKeeper<Box<dyn SettingsStore>>) -> Self {
        self.settings = Some(keeper);
        self
    }

    /// Why the window could not be opened, if it could not; the event loop
    /// exits as soon as that happens.
    pub fn open_failure(&self) -> Option<&OpenFailure> {
        self.failure.as_ref()
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
    let window = event_loop
        .create_window(attributes)
        .map_err(|error| OpenFailure::Window(error.to_string()))?;
    let window = Arc::new(window);
    let gpu = SurfaceGpu::new(
        Box::new(event_loop.owned_display_handle()),
        window.clone(),
        fonts,
    )
    .map_err(|error| gpu_failure(&error))?;
    Ok((WinitWindow(window), gpu))
}

impl<S: ImageSource> ApplicationHandler for Runner<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some((images, screen, fonts)) = self.pending.take() else {
            return;
        };
        match open(event_loop, &fonts) {
            Ok((mut window, gpu)) => {
                let app = App::new(&window, images, screen);
                let app = match self.audio.take() {
                    Some(core) => app.with_audio(core),
                    None => app,
                };
                let app = match self.settings.take() {
                    Some(keeper) => app.with_settings(keeper),
                    None => app,
                };
                #[cfg(feature = "dev-tools")]
                let (app, dev_tools) = match self.dev_catalog.take() {
                    Some(catalog) => (
                        app.with_dev_overlay(),
                        Some(DevTools::new(&window.0, catalog)),
                    ),
                    None => (app, None),
                };
                crate::app::WindowPort::request_redraw(&mut window);
                self.running = Some(Running {
                    app,
                    gpu,
                    #[cfg(feature = "dev-tools")]
                    dev_tools,
                    window,
                });
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
        let event = match &raw {
            WinitEvent::RedrawRequested => Some(WindowEvent::Redraw {
                elapsed: self.start.elapsed(),
            }),
            other => translate(other, &running.window),
        };
        // A redraw while the developer overlay shows first runs the egui
        // panel, then submits through a GPU that paints it over the frame.
        let handled = event.map(|event| {
            #[cfg(feature = "dev-tools")]
            if let (WindowEvent::Redraw { .. }, Some(dev_tools), Some(overlay)) =
                (event, &mut running.dev_tools, running.app.dev_overlay())
                && overlay.visible()
            {
                dev_tools.run_frame(&running.window.0, overlay);
                let mut gpu = WithOverlay {
                    gpu: &mut running.gpu,
                    painter: dev_tools.layer_mut(),
                };
                return running
                    .app
                    .handle_routed(event, &mut running.window, &mut gpu);
            }
            running
                .app
                .handle_routed(event, &mut running.window, &mut running.gpu)
        });
        // The text a key press types follows the key.
        let typed = match &raw {
            WinitEvent::KeyboardInput { event, .. } => {
                super::text_event(event.text.as_deref(), event.state)
            }
            _ => None,
        };
        let typed = typed.map(|event| {
            running
                .app
                .handle_routed(event, &mut running.window, &mut running.gpu)
        });
        #[cfg(feature = "dev-tools")]
        if let Some(dev_tools) = &mut running.dev_tools
            && running.app.overlay_wants(handled.as_ref())
        {
            dev_tools.on_window_event(&running.window.0, &raw);
        }
        for (key, error) in running.app.take_failures() {
            eprintln!("nova: image {key:?}: {error}");
        }
        for warning in running.app.take_warnings() {
            eprintln!("{warning}");
        }
        let exits = |handled: Option<Handled>| handled.is_some_and(|h| h.control == Control::Exit);
        if exits(handled) || exits(typed) {
            event_loop.exit();
        }
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
    use nova_audio::{AudioCommand, AudioSettings, Volume};
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
        core.update(Some(Showing::GalaxyMap), &[]);
        assert_eq!(
            *log.borrow(),
            [AudioCommand::StartMusic {
                volume: Volume::FULL
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
        let quiet = AudioSettings {
            sound: false,
            ..AudioSettings::default()
        };
        keeper.change(quiet).expect("saves");
        assert_eq!(store.writes(), 1);
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
