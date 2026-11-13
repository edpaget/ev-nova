//! The save layer of nova-sim's layered crates, on the story layer: the
//! save schema ([`save`]), which encodes a pilot as JSON and decodes and
//! upgrades every older version, and the pilot store port ([`saves`]),
//! with the rules for saving, listing, opening and deleting pilots
//! through it. The test support, the port's fake and contract, is in
//! `fixture`.
//!
//! The `nova-sim` facade re-exports both modules at their old paths
//! (`nova_sim::save`, `nova_sim::saves`), and its `fixture` re-exports
//! this crate's.

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod save;
pub mod saves;
