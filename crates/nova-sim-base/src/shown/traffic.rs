//! How opaque the NPC traffic is drawn at a gate, from
//! `nova_sim_ai::traffic`, which re-exports each item at its old path and
//! keeps the traffic itself.
//!
//! - [`emerge`]: a ship coming out of a gate, from
//!   `nova_sim_ai::traffic::emerge`.
//! - [`gate_entry`]: a ship going into a gate, from
//!   `nova_sim_ai::traffic::gate_entry`.

pub mod emerge;
pub mod gate_entry;
