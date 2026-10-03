//! The winit event loop's handler: opens the window and the GPU surface,
//! then forwards every event to the [`App`].

use std::sync::Arc;
use std::time::Instant;

use nova_render::ImageSource;
use nova_render::wgpu::{InitError, SurfaceGpu};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent as WinitEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use super::translate;
use super::window::WinitWindow;
use crate::app::{App, AppScreen, Control, WindowEvent};
use crate::exit::OpenFailure;

/// The window, its GPU surface and the app, once the window is open.
struct Running<S> {
    app: App<S>,
    // Dropped before the window it draws into.
    gpu: SurfaceGpu,
    window: WinitWindow,
}

/// Runs the app in a winit event loop.
pub struct Runner<S> {
    pending: Option<(S, AppScreen)>,
    running: Option<Running<S>>,
    failure: Option<OpenFailure>,
    start: Instant,
}

impl<S: ImageSource> Runner<S> {
    /// A runner that will open a window showing `screen` with images from
    /// `images` when the event loop starts.
    pub fn new(images: S, screen: AppScreen) -> Self {
        Self {
            pending: Some((images, screen)),
            running: None,
            failure: None,
            start: Instant::now(),
        }
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

/// Opens the 1024x768-point, resizable "EV Nova" window and its surface.
fn open(event_loop: &ActiveEventLoop) -> Result<(WinitWindow, SurfaceGpu), OpenFailure> {
    let attributes = Window::default_attributes()
        .with_title("EV Nova")
        .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0))
        .with_resizable(true);
    let window = event_loop
        .create_window(attributes)
        .map_err(|error| OpenFailure::Window(error.to_string()))?;
    let window = Arc::new(window);
    let gpu = SurfaceGpu::new(Box::new(event_loop.owned_display_handle()), window.clone())
        .map_err(|error| gpu_failure(&error))?;
    Ok((WinitWindow(window), gpu))
}

impl<S: ImageSource> ApplicationHandler for Runner<S> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some((images, screen)) = self.pending.take() else {
            return;
        };
        match open(event_loop) {
            Ok((mut window, gpu)) => {
                let app = App::new(&window, images, screen);
                crate::app::WindowPort::request_redraw(&mut window);
                self.running = Some(Running { app, gpu, window });
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
        let event = match event {
            WinitEvent::RedrawRequested => Some(WindowEvent::Redraw {
                elapsed: self.start.elapsed(),
            }),
            other => translate(&other, &running.window),
        };
        let Some(event) = event else {
            return;
        };
        let control = running
            .app
            .handle(event, &mut running.window, &mut running.gpu);
        for (key, error) in running.app.take_failures() {
            eprintln!("nova: image {key:?}: {error}");
        }
        if control == Control::Exit {
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
    fn the_runner_reports_the_failure_that_stopped_the_window_opening() {
        let mut runner = Runner::new(NoImages, start_screen(no_data()));
        assert_eq!(runner.open_failure(), None);
        runner.failure = Some(OpenFailure::Window("no display".into()));
        assert_eq!(
            runner.open_failure(),
            Some(&OpenFailure::Window("no display".into()))
        );
    }
}
