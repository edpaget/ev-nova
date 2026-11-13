//! The view vocabulary the layers above build that needs nothing above
//! this crate: plain data and pure rules moved down here from the crate
//! that builds them, so that the view's vocabulary (`nova_sim_frame`) can
//! sit below combat, the ai, the story and the port. Each module is named
//! for the module it came from, which re-exports its items at their old
//! paths (`nova_sim::slot_pool::SlotPool` is
//! [`slot_pool::SlotPool`]), and keeps the behaviour that reads the state
//! above.
//!
//! - [`person`]: how long a person's hail quote shows, from
//!   `nova_sim_story::person`.
//! - [`slot_pool`]: the pool whose items take the first free slot, from
//!   `nova_sim_ai::slot_pool`.
//! - [`text_tags`]: a number as the original writes it, from
//!   `nova_sim_story::text_tags`.
//! - [`traffic`]: how opaque a ship coming out of or going into a gate
//!   is drawn, from `nova_sim_ai::traffic`.

pub mod person;
pub mod slot_pool;
pub mod text_tags;
pub mod traffic;
