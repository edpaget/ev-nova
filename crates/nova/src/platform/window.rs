//! The window port over a real winit window.

use std::sync::Arc;

use winit::dpi::PhysicalSize;
use winit::window::{Fullscreen, Window};

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

    fn set_pointer_visible(&mut self, visible: bool) {
        self.0.set_cursor_visible(visible);
    }

    fn set_fullscreen(&mut self, on: bool) {
        // Borderless on the window's current monitor.
        self.0
            .set_fullscreen(on.then_some(Fullscreen::Borderless(None)));
    }

    fn is_fullscreen(&self) -> bool {
        self.0.fullscreen().is_some()
    }

    fn request_inner_size(&mut self, (width, height): (u32, u32)) {
        // The size it takes comes in a resize; the one returned at once
        // on some platforms adds nothing to that.
        let _ = self.0.request_inner_size(PhysicalSize::new(width, height));
    }
}
