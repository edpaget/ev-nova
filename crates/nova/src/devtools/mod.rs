//! The developer tools overlay, drawn with egui (the `dev-tools` feature
//! only).
//!
//! Everything the overlay decides lives in `nova_view::devtools` and is
//! built and tested without egui; this module only draws that model and
//! wires egui to the window and the GPU:
//!
//! - [`textures`]: the preview frame as an egui texture.
//! - [`panel`]: the [`DevPanel`] windows: the frame-time readout and the
//!   resource browser, and the pilot editor.
//! - [`layer`]: the [`EguiLayer`], which paints egui's output over each
//!   frame with egui-wgpu.
//! - [`input`]: the [`EguiInput`], the game's own [`WindowEvent`]s as
//!   egui's input. The overlay names no winit type: it takes the events
//!   the game does, already translated, and its module docs list what
//!   that leaves out (cursor icons, IME, horizontal scrolling, logical
//!   keys).
//! - [`DevTools`]: the three together for one window.
//!
//! The runner shows the overlay only while it is visible: on each redraw
//! it runs [`DevTools::run_frame`] first, through `App::pilot_desk_frame`,
//! which hands it the pilot flying to edit and catches the game up after
//! an edit, then hands the app the window's
//! GPU wrapped in `WithOverlay` with the layer as its painter. Of the
//! window events it gave the app, it forwards to [`DevTools::on_event`]
//! each that `App::overlay_wants`, by how the app handled it; the app
//! decides where the text a key types goes, with its key.

pub mod input;
pub mod layer;
pub mod panel;
pub mod textures;

use std::rc::Rc;
use std::time::Duration;

use nova_data::GameData;
use nova_view::devtools::{DevOverlay, PilotDesk};

use crate::app::{WindowEvent, WindowPort};

pub use input::EguiInput;
pub use layer::EguiLayer;
pub use panel::DevPanel;

/// The developer tools in a window: the panel over the game data, egui's
/// input from the window's events, and the layer that paints the panel.
pub struct DevTools {
    panel: DevPanel<Rc<GameData>>,
    input: EguiInput,
    layer: EguiLayer,
}

impl DevTools {
    /// The developer tools for a window at `scale_factor`, browsing
    /// `catalog`.
    #[must_use]
    pub fn new(catalog: Rc<GameData>, scale_factor: f64) -> Self {
        Self {
            panel: DevPanel::new(catalog),
            input: EguiInput::new(scale_factor, cfg!(target_os = "macos")),
            layer: EguiLayer::default(),
        }
    }

    /// Hands egui one of the game's window events.
    pub fn on_event(&mut self, event: &WindowEvent) {
        self.input.on_event(event);
    }

    /// Runs one egui frame of the panel in `window`, `elapsed` since the
    /// app started, showing `overlay`'s frame times and editing the pilot
    /// flying through `desk`, if any, and prepares the layer to paint it
    /// over the next frame. Returns whether the pilot changed. egui's
    /// platform output (cursor icons, the IME's place, links to open) goes
    /// unused.
    pub fn run_frame(
        &mut self,
        window: &impl WindowPort,
        elapsed: Duration,
        overlay: &DevOverlay,
        desk: Option<&mut dyn PilotDesk>,
    ) -> bool {
        let input = self
            .input
            .take(window.size_px(), window.scale_factor(), elapsed);
        let (output, edited) = self.panel.run(input, overlay, desk);
        self.layer.prepare(
            self.panel.context(),
            output.shapes,
            output.textures_delta,
            output.pixels_per_point,
        );
        edited
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
