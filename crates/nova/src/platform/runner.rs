//! The winit event loop's handler: opens the window and the GPU surface,
//! then forwards every event to the [`App`].

use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

use nova_render::ImageSource;
use nova_render::wgpu::SurfaceGpu;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent as WinitEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use super::translate;
use super::window::WinitWindow;
use crate::app::{App, Control, Placeholder, WindowEvent};

/// The window, its GPU surface and the app, once the window is open.
struct Running<S> {
    app: App<S, Placeholder>,
    // Dropped before the window it draws into.
    gpu: SurfaceGpu,
    window: WinitWindow,
}

/// Runs the app in a winit event loop.
pub struct Runner<S> {
    pending: Option<(S, Placeholder)>,
    running: Option<Running<S>>,
    start: Instant,
}

impl<S: ImageSource> Runner<S> {
    /// A runner that will open a window showing `screen` with images from
    /// `images` when the event loop starts.
    pub fn new(images: S, screen: Placeholder) -> Self {
        Self {
            pending: Some((images, screen)),
            running: None,
            start: Instant::now(),
        }
    }
}

/// Opens the 1024x768-point, resizable "EV Nova" window and its surface.
fn open(event_loop: &ActiveEventLoop) -> Result<(WinitWindow, SurfaceGpu), Box<dyn Error>> {
    let attributes = Window::default_attributes()
        .with_title("EV Nova")
        .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0))
        .with_resizable(true);
    let window = Arc::new(event_loop.create_window(attributes)?);
    let gpu = SurfaceGpu::new(Box::new(event_loop.owned_display_handle()), window.clone())?;
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
            Err(error) => {
                eprintln!("nova: opening the window: {error}");
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
