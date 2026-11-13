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
//! - [`board`]: why the player cannot board, what a board did, the
//!   plunder dialog's view and presses and a capture's assignment, from
//!   `nova_sim_port::board`.
//! - [`board_repair`]: which own-ship path a board takes, from
//!   `nova_sim_port::board_repair`.
//! - [`capture_naming`]: the prompt for renaming a captured ship, from
//!   `nova_sim_port::capture_naming`.
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
//! - [`escort_dialog`]: the escort dialog's view, buttons and labels,
//!   from `nova_sim_port::escort_dialog`.
//! - [`exploration`]: the `spöb` bits of a gate, from
//!   `nova_sim_story::exploration`.
//! - [`fleet`]: an escort's shipyard mark and standing order, from
//!   `nova_sim_combat::fleet`.
//! - [`gamble`]: the bar's race as it is shown: the opening, a bet, a
//!   run and its text, from `nova_sim_port::gamble`.
//! - [`gate`]: why a gate took the ship nowhere, and the hypergate map's
//!   next pick, from `nova_sim_ai::gate`.
//! - [`grant`]: what a grant gave, from `nova_sim_port::grant`.
//! - [`hail`]: the comm dialog's view and the haggle dialog's choices,
//!   from `nova_sim_ai::hail`.
//! - [`hire`]: the bar's hire list, its refusals and what a hire did,
//!   from `nova_sim_port::hire`.
//! - [`hover`]: the boxes the view draws ships in and what the pointer
//!   is over, from `nova_sim_ai::hover`.
//! - [`hyperspace`]: a jump's fuel and whether the ship can jump, from
//!   `nova_sim_ai::hyperspace`.
//! - [`iff`]: what the outfits give the radar and the IFF colours of
//!   ships, stellars and systems, from `nova_sim_port::iff`.
//! - [`message`]: the messages a flight session raises, from
//!   `nova_sim_combat::message`.
//! - [`mission`]: the offer dialog, the mission screens' words and why
//!   a mission is not accepted or aborted, from `nova_sim_story::mission`.
//! - [`navigation`]: the player's nav mode and a sprite slot's rect,
//!   from `nova_sim_ai::navigation`.
//! - [`news`]: a bar visit's news, from `nova_sim_story::news`.
//! - [`order_loop`]: what an order loop did and where it stopped, from
//!   `nova_sim_port::order_loop`.
//! - [`outfitter`]: a stellar's outfitter as the player sees it, an
//!   order and what it did, and the ship's guns and turrets, from
//!   `nova_sim_port::outfitter`.
//! - [`pilot`]: how far a system is explored, from
//!   `nova_sim_story::pilot`.
//! - [`planet_comm`]: the planet comm dialog's view, from
//!   `nova_sim_ai::planet_comm`.
//! - [`player_fate`]: the player's lifecycle and what each stage allows,
//!   from `nova_sim_combat::player_fate`.
//! - [`player_info`]: what the player info screen shows, its words and
//!   cheat mode's numbers, from `nova_sim_port::player_info`.
//! - [`recharge`]: why the ship can't be recharged and whether a
//!   stellar sells fuel, from `nova_sim_port::recharge`.
//! - [`self_repair`]: what a former escort was, from
//!   `nova_sim_combat::self_repair`.
//! - [`ship_layer`]: the layer a ship is drawn in, from
//!   `nova_sim_combat::ship_layer`.
//! - [`shipyard`]: what the Shipyard Info panel shows of a ship class,
//!   from `nova_sim_port::shipyard`.
//! - [`sprite_layer`]: the order the flight world's layers are drawn in,
//!   from `nova_sim_combat::sprite_layer`.
//! - [`stellar_frames`]: a stellar's animation fields and frames, from
//!   `nova_sim_port::stellar_frames`.
//! - [`story`]: which sheet a stellar shown draws, from
//!   `nova_sim_story::story`.
//! - [`storylines`]: a news item a system has heard, from
//!   `nova_sim_story::storylines`.
//! - [`targeting`]: how a target command picks, from
//!   `nova_sim_ai::targeting`.
//! - [`tutorial`]: a first-flight tutorial hint, from
//!   `nova_sim_combat::tutorial`.
//! - [`wares`]: a number of tons as a shop shows it, from
//!   `nova_sim_port::wares`.

pub mod ai;
pub mod bay;
pub mod board;
pub mod board_repair;
pub mod brackets;
pub mod capture_naming;
pub mod cloak;
pub mod combat;
pub mod dialog;
pub mod escape_pod;
pub mod escort;
pub mod escort_dialog;
pub mod exploration;
pub mod fleet;
pub mod gamble;
pub mod gate;
pub mod grant;
pub mod hail;
pub mod has_max;
pub mod hire;
pub mod hover;
pub mod hyperspace;
pub mod iff;
pub mod message;
pub mod mission;
pub mod navigation;
pub mod news;
pub mod order_loop;
pub mod outfitter;
pub mod pilot;
pub mod planet_comm;
pub mod player_fate;
pub mod player_info;
pub mod recharge;
pub mod self_repair;
pub mod ship_layer;
pub mod shipyard;
pub mod sprite_layer;
pub mod stellar_frames;
pub mod story;
pub mod storylines;
pub mod targeting;
pub mod tutorial;
pub mod wares;
