//! nova-render's integration tests, built as one test binary.
//!
//! Each file under `tests/it/` is a module of this binary. Add new
//! integration tests as a module here, not as a new `tests/*.rs` file:
//! every `tests/*.rs` file is a separate binary to link.

mod common;

mod frame_cost;
mod metal_fx;
mod sharp_bilinear;
mod stock_fonts;
mod stock_render;
mod wgpu_offscreen;
mod world_layer;
