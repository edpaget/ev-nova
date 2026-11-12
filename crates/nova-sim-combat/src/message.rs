//! The messages a flight session raises, as events: what happened, not
//! the words. The view decides what each one says on the message line.
//! They are [`nova_sim_state::shown::message`]'s, re-exported here.

pub use nova_sim_state::shown::message::{DockFind, SelfDestructLine, SimMessage};
