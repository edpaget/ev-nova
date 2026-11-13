//! nova's integration tests, under `tests/it`. The crate has no code of its
//! own; Cargo needs a library or binary target, so this file is it.
//!
//! The tests live here rather than in nova's own `tests/` so that nova
//! needs no dev-dependency on itself. With one, every build of nova's unit
//! tests waited on a second build of its library, the one with `fixture`
//! on, plus its bin and the `it` binary
//! (rdm:phase/faster-builds/phase-10-nova-integration-tests-crate).
//!
//! They are wiring tests of the app over the real crates. The tests of the
//! flight's seam over a real `nova_sim::Session` are `nova_flight`'s.
