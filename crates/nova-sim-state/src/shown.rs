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
//! - [`ai`]: the help an NPC gives the player, from `nova_sim_ai::ai`.
//! - [`bay`]: what became of the player's fighters, from
//!   `nova_sim_combat::bay`.
//! - [`cloak`]: how a cloak lets a ship be shown, and the area a view
//!   shows, from `nova_sim_combat::cloak`.
//! - [`brackets`]: the brackets round the nav and ship targets, from
//!   `nova_sim_ai::brackets`.
//! - [`combat`]: the fight's ids, refs and events, a hull's
//!   [`Condition`](combat::hull::Condition), a class's display
//!   ([`FrameSpec`](combat::ship_frames::FrameSpec)), a shot's layer and
//!   fade, and the sprite and report types the flight frame draws by,
//!   from `nova_sim_combat::combat`.
//! - [`dialog`]: what a conversation screen shows, from
//!   `nova_sim_story::dialog`.
//! - [`escape_pod`]: an escape pod adrift, from
//!   `nova_sim_ai::escape_pod`.
//! - [`escort`]: the escort menu's commands, groups and strings, from
//!   `nova_sim_ai::escort`.
//! - [`exploration`]: the `spöb` bits of a gate, from
//!   `nova_sim_story::exploration`.
//! - [`fleet`]: an escort's shipyard mark and standing order, from
//!   `nova_sim_combat::fleet`.
//! - [`gate`]: why a gate took the ship nowhere, and the hypergate map's
//!   next pick, from `nova_sim_ai::gate`.
//! - [`hail`]: the comm dialog's view and the haggle dialog's choices,
//!   from `nova_sim_ai::hail`.
//! - [`hover`]: the boxes the view draws ships in and what the pointer
//!   is over, from `nova_sim_ai::hover`.
//! - [`hyperspace`]: a jump's fuel and whether the ship can jump, from
//!   `nova_sim_ai::hyperspace`.
//! - [`message`]: the messages a flight session raises, from
//!   `nova_sim_combat::message`.
//! - [`mission`]: the offer dialog, the mission screens' words and why
//!   a mission is not accepted or aborted, from `nova_sim_story::mission`.
//! - [`navigation`]: the player's nav mode and a sprite slot's rect,
//!   from `nova_sim_ai::navigation`.
//! - [`news`]: a bar visit's news, from `nova_sim_story::news`.
//! - [`pilot`]: how far a system is explored, from
//!   `nova_sim_story::pilot`.
//! - [`planet_comm`]: the planet comm dialog's view, from
//!   `nova_sim_ai::planet_comm`.
//! - [`player_fate`]: the player's lifecycle and what each stage allows,
//!   from `nova_sim_combat::player_fate`.
//! - [`self_repair`]: what a former escort was, from
//!   `nova_sim_combat::self_repair`.
//! - [`ship_layer`]: the layer a ship is drawn in, from
//!   `nova_sim_combat::ship_layer`.
//! - [`sprite_layer`]: the order the flight world's layers are drawn in,
//!   from `nova_sim_combat::sprite_layer`.
//! - [`story`]: which sheet a stellar shown draws, from
//!   `nova_sim_story::story`.
//! - [`storylines`]: a news item a system has heard, from
//!   `nova_sim_story::storylines`.
//! - [`targeting`]: how a target command picks, from
//!   `nova_sim_ai::targeting`.
//! - [`tutorial`]: a first-flight tutorial hint, from
//!   `nova_sim_combat::tutorial`.

pub mod ai;
pub mod bay;
pub mod brackets;
pub mod cloak;
pub mod combat;
pub mod dialog;
pub mod escape_pod;
pub mod escort;
pub mod exploration;
pub mod fleet;
pub mod gate;
pub mod hail;
pub mod hover;
pub mod hyperspace;
pub mod message;
pub mod mission;
pub mod navigation;
pub mod news;
pub mod pilot;
pub mod planet_comm;
pub mod player_fate;
pub mod self_repair;
pub mod ship_layer;
pub mod sprite_layer;
pub mod story;
pub mod storylines;
pub mod targeting;
pub mod tutorial;
