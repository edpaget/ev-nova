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
//! - [`bay`]: what became of the player's fighters, from
//!   `nova_sim_combat::bay`.
//! - [`cloak`]: how a cloak lets a ship be shown, and the area a view
//!   shows, from `nova_sim_combat::cloak`.
//! - [`combat`]: the fight's ids, refs and events, a hull's
//!   [`Condition`](combat::hull::Condition), a class's display
//!   ([`FrameSpec`](combat::ship_frames::FrameSpec)), a shot's layer and
//!   fade, and the sprite and report types the flight frame draws by,
//!   from `nova_sim_combat::combat`.
//! - [`fleet`]: an escort's shipyard mark and standing order, from
//!   `nova_sim_combat::fleet`.
//! - [`message`]: the messages a flight session raises, from
//!   `nova_sim_combat::message`.
//! - [`player_fate`]: the player's lifecycle and what each stage allows,
//!   from `nova_sim_combat::player_fate`.
//! - [`self_repair`]: what a former escort was, from
//!   `nova_sim_combat::self_repair`.
//! - [`ship_layer`]: the layer a ship is drawn in, from
//!   `nova_sim_combat::ship_layer`.
//! - [`sprite_layer`]: the order the flight world's layers are drawn in,
//!   from `nova_sim_combat::sprite_layer`.
//! - [`tutorial`]: a first-flight tutorial hint, from
//!   `nova_sim_combat::tutorial`.

pub mod bay;
pub mod cloak;
pub mod combat;
pub mod fleet;
pub mod message;
pub mod player_fate;
pub mod self_repair;
pub mod ship_layer;
pub mod sprite_layer;
pub mod tutorial;
