//! The view vocabulary the layers above build: the plain data the flight
//! frame, its events and its screens carry, moved down here from the
//! crate that builds it so that the view's vocabulary
//! (`nova_sim_frame`) can sit below combat, the ai, the story and the
//! port. Each module is named for the module it came from, which
//! re-exports its items at their old paths (`nova_sim::combat::NpcId`
//! is [`combat::NpcId`]), and keeps the behaviour that reads the state
//! above: here are only the types, their own methods and the pure rules
//! they are read by.
//!
//! - [`combat`]: the fight's ids, refs and events, a hull's
//!   [`Condition`](combat::hull::Condition), and the sprite and report
//!   types the flight frame draws by, from `nova_sim_combat::combat`.

pub mod combat;
