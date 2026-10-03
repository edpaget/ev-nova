//! The developer tools overlay, drawn with egui (the `dev-tools` feature
//! only).
//!
//! Everything the overlay decides lives in `nova_view::devtools` and is
//! built and tested without egui; this module only draws that model and
//! wires egui to the window and the GPU:
//!
//! - [`textures`]: the preview frame as an egui texture.
//! - [`panel`]: the [`DevPanel`] window, the frame-time readout and the
//!   resource browser.
//! - [`layer`]: the [`EguiLayer`], which paints egui's output over each
//!   frame with egui-wgpu.
//! - [`DevTools`]: the window glue, egui-winit's input state between the
//!   window and the panel.
//!
//! The runner shows the overlay only while it is visible: on each redraw
//! it runs [`DevTools::run_frame`] first, then hands the app the window's
//! GPU wrapped in `WithOverlay` with the layer as its painter. It forwards a
//! raw window event to [`DevTools::on_window_event`] only when
//! `App::overlay_wants` it.

pub mod layer;
pub mod panel;
pub mod textures;

use std::rc::Rc;

use egui::ViewportId;
use nova_data::GameData;
use nova_render::wgpu::MAX_TEXTURE_SIDE;
use nova_view::devtools::DevOverlay;
use winit::event::WindowEvent as WinitEvent;
use winit::window::Window;

pub use layer::EguiLayer;
pub use panel::DevPanel;

/// The developer tools in a window: the panel over the game data, egui's
/// input state for the window, and the layer that paints the panel.
pub struct DevTools {
    panel: DevPanel<Rc<GameData>>,
    state: egui_winit::State,
    layer: EguiLayer,
}

impl DevTools {
    /// The developer tools for `window`, browsing `catalog`.
    #[must_use]
    pub fn new(window: &Window, catalog: Rc<GameData>) -> Self {
        let panel = DevPanel::new(catalog);
        let state = egui_winit::State::new(
            panel.context().clone(),
            ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            Some(MAX_TEXTURE_SIDE as usize),
        );
        Self {
            panel,
            state,
            layer: EguiLayer::default(),
        }
    }

    /// Hands egui one raw window event.
    pub fn on_window_event(&mut self, window: &Window, event: &WinitEvent) {
        // The app has already decided where input goes, so egui's own
        // "consumed" verdict is not needed.
        let _ = self.state.on_window_event(window, event);
    }

    /// Runs one egui frame of the panel, showing `overlay`'s frame times,
    /// and prepares the layer to paint it over the next frame.
    pub fn run_frame(&mut self, window: &Window, overlay: &DevOverlay) {
        let input = self.state.take_egui_input(window);
        let output = self.panel.run(input, overlay);
        self.state
            .handle_platform_output(window, output.platform_output);
        self.layer.prepare(
            self.panel.context(),
            output.shapes,
            output.textures_delta,
            output.pixels_per_point,
        );
    }

    /// The layer, to paint over the next frame.
    pub fn layer_mut(&mut self) -> &mut EguiLayer {
        &mut self.layer
    }
}

#[cfg(test)]
mod tests {
    use nova_render::wgpu::MAX_TEXTURE_SIDE;
    use nova_view::devtools::MAX_PREVIEW_SIDE;

    #[test]
    fn every_previewed_frame_fits_in_a_texture() {
        const { assert!(MAX_PREVIEW_SIDE <= MAX_TEXTURE_SIDE) };
        assert_eq!((MAX_PREVIEW_SIDE, MAX_TEXTURE_SIDE), (4096, 8192));
    }
}
