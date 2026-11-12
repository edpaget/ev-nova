//! The original's fixed tables whose entries take the first free slot:
//! the cargo boxes ([`cargo_box`](crate::cargo_box)) and the escape pods
//! ([`escape_pod`](crate::escape_pod)) keep theirs in a [`SlotPool`],
//! which lives in `nova_sim_base::shown::slot_pool` with its docs, so the
//! view's vocabulary can name it below the ai; this module re-exports it
//! at its old path.

pub use nova_sim_base::shown::slot_pool::SlotPool;
