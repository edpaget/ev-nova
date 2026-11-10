//! nova-render's integration tests, built as one test binary.
//!
//! Each file under `tests/it/` is a module of this binary. Add new
//! integration tests as a module here, not as a new `tests/*.rs` file:
//! every `tests/*.rs` file is a separate binary to link.

mod common;

mod enlarged_atlas;
mod flight_frame;
mod floater_text;
mod frame_cost;
mod fsr1;
mod fsr1_reference;
mod livery_paint;
mod message_line;
mod metal_fx;
mod mounted_outfits;
mod prefs_labels;
mod sample_models;
mod sharp_bilinear;
mod stock_fonts;
mod stock_interface_images;
mod stock_quickdraw_stretch;
mod stock_render;
mod stock_stock_art;
mod stock_super_xbr;
mod stock_super_xbr_cost;
mod text_hinting;
mod text_layout;
mod wgpu_offscreen;
mod world_layer;
mod world_models;
