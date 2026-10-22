//! nova-data's integration tests, built as one test binary.
//!
//! Each file under `tests/it/` is a module of this binary. Add new
//! integration tests as a module here, not as a new `tests/*.rs` file:
//! every `tests/*.rs` file is a separate binary to link.

mod common;

mod file;
mod fonts;
mod graphics;
mod interface;
mod sound;
mod stock;
mod stock_expr;
mod stock_fonts;
mod stock_graphics;
mod stock_interface;
mod stock_sound;
mod stock_store;
mod store;
