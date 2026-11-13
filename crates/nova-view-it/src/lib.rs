//! nova-view's integration tests, under `tests/it`. The crate has no code of
//! its own; Cargo needs a library or binary target, so this file is it.
//!
//! The tests live here rather than in nova-view's own `tests/` so that
//! nova-view needs no dev-dependency on itself. With one, every build of
//! nova-view's unit tests waited on a second build of its library, the one
//! with `fixture` on, which the integration tests link (faster-builds).
//!
//! No test here starts a real `nova_sim::Session`: the seam tests and the
//! stock tests over a real session moved to nova-flight's `tests/it` with
//! the adapter (rdm:phase/faster-builds/phase-6-nova-view-off-nova-sim).
