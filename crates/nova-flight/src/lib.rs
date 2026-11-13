//! The flight view's adapter over a real nova-sim `Session`, and the edge
//! that builds a flight.
//!
//! nova-view's flight screen ([`FlightView`]) runs any flight behind its
//! port, [`FlightSim`](nova_view::flight::FlightSim). This crate holds the
//! one real flight and the wiring round it, above nova-view and nova-sim,
//! so that nova-view builds without nova-sim
//! (rdm:phase/faster-builds/phase-6-nova-view-off-nova-sim):
//!
//! - [`sim_adapter`]: [`SessionFlight`], the [`FlightSim`](nova_view::flight::FlightSim)
//!   adapter over a real session, with the [`Collaborators`] its calls
//!   are handed;
//! - [`edge`]: the edge that builds a flight ([`open_flight`]) and enters
//!   play ([`begin_game`], [`enter_game`]), and `test_flight`, the bare
//!   flights the wiring tests build;
//! - [`session_view`]: [`SessionView`], the view of a real session, and
//!   [`SessionViewExt`], the two reads of its session the wiring tests
//!   make.
//!
//! # Where the tests that start a real `Session` live
//!
//! nova-view's tests never start one: its component tests run on
//! `FakeFlight` (`nova_view::flight::sim_port::fixture`). The tests that
//! wire a real session behind the port are this crate's:
//!
//! - the adapter's checks, `sim_adapter`'s tests and
//!   `sim_adapter/fake_catalog_checks.rs`;
//! - the edge's checks, `edge/edge_checks.rs`;
//! - `tests/it/flight_seam.rs`, one test per port surface or router
//!   handoff, as its module doc says;
//! - the `stock_*` modules under `tests/it`, over the stock data.
//!
//! What the session decides is nova-sim's, pinned by its tests; these
//! check only that the wiring hands it through.

pub mod edge;
pub mod session_view;
pub mod sim_adapter;

#[cfg(test)]
mod fakes;

pub use edge::{EdgeCatalog, FlightStart, begin_game, enter_game, open_flight, test_flight};
pub use session_view::{SessionView, SessionViewExt};
pub use sim_adapter::{Collaborators, SessionFlight};

#[cfg(doc)]
use nova_view::flight::FlightView;
