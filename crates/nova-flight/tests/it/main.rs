//! nova-flight's integration tests, built as one test binary: the flight
//! view's seam wired to a real session (`flight_seam`) and the stock
//! tests over a real session (`stock_*`), moved from nova-view-it with
//! the adapter (rdm:phase/faster-builds/phase-6-nova-view-off-nova-sim).
//!
//! Each file under `tests/it/` is a module of this binary. Add new
//! integration tests as a module here, not as a new `tests/*.rs` file:
//! every `tests/*.rs` file is a separate binary to link.

mod common;

mod flight_seam;
mod stock_flight_hud;
mod stock_flight_seam;
mod stock_map_borders;
mod stock_spaceport;
