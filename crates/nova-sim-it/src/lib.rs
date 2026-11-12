//! nova-sim's integration tests, under `tests/it`. The crate has no code of
//! its own; Cargo needs a library or binary target, so this file is it.
//!
//! The tests live here rather than in nova-sim's own `tests/` so that
//! nova-sim needs no dev-dependency on itself. With one, every build of
//! nova-sim's unit tests waited on a second build of its library, the one
//! with `fixture` on, which the integration tests link (faster-builds).
