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

pub mod layer;
pub mod panel;
pub mod textures;

pub use layer::EguiLayer;
pub use panel::DevPanel;
