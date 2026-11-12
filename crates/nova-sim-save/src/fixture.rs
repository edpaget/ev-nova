//! Test support for tests outside the crate's own unit tests (the
//! `fixture` feature): [`MemoryPilots`], a
//! [`PilotStore`](crate::saves::PilotStore) in memory, the pilot store
//! port's fake; and [`pilot_store`], the port's contract, which every
//! store meets. nova-sim's `fixture` re-exports both at their old paths.

mod memory_pilots;
pub mod pilot_store;

pub use memory_pilots::MemoryPilots;
