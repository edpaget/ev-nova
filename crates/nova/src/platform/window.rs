//! The window port over a real winit window.

use std::sync::Arc;

use winit::window::Window;

use crate::app::WindowPort;

/// A [`WindowPort`] over a winit window.
#[derive(Clone, Debug)]
pub struct WinitWindow(pub Arc<Window>);

impl WindowPort for WinitWindow {
    fn size_px(&self) -> (u32, u32) {
        let size = self.0.inner_size();
        (size.width, size.height)
    }

    fn scale_factor(&self) -> f64 {
        self.0.scale_factor()
    }

    fn request_redraw(&mut self) {
        self.0.request_redraw();
    }
}
