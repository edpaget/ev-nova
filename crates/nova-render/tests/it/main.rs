//! nova-render's integration tests, built as one test binary.
//!
//! Each file under `tests/it/` is a module of this binary. Add new
//! integration tests as a module here, not as a new `tests/*.rs` file:
//! every `tests/*.rs` file is a separate binary to link.

mod common;

mod enlarged_atlas;
mod flight_frame;
mod frame_cost;
mod metal_fx;
mod prefs_labels;
mod sharp_bilinear;
mod stock_fonts;
mod stock_render;
mod stock_super_xbr;
mod stock_super_xbr_cost;
mod text_hinting;
mod text_layout;
mod wgpu_offscreen;
mod world_layer;
