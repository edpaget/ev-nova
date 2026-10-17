//! The combat layer of EV Nova's simulation: ships fighting, and the
//! pieces of a flight that fighting reads and changes, such as the
//! asteroid field, fighter bays, cloaking, sabotage, self-repair and the
//! player's fleet and fate. It builds on the foundation, `nova-sim-base`,
//! the catalog, `nova-sim-catalog`, and the state, `nova-sim-state`.
//! `nova-sim` re-exports each module at its old path (`nova_sim::combat`,
//! `nova_sim::Armament`, …), so the rest of the workspace reaches them
//! through it.
//!
//! - [`asteroid`]: a system's asteroid field: the 16 slots
//!   ([`Asteroids`](asteroid::Asteroids)), how an asteroid spawns at the
//!   field's edge or in place, how it drifts and spins, and when it is
//!   culled.
//! - [`bay`]: fighter bays: the fighters a bay holds
//!   ([`capacity`](bay::capacity)), how one is launched, the window it
//!   docks in ([`dock_window`](bay::dock_window)) with its
//!   [`Carrier`](bay::Carrier), and what the flight is told of fighters
//!   abandoned ([`FighterNote`](bay::FighterNote)).
//! - [`cloak`]: cloaking: how a ship fades into its
//!   [`Cloak`](cloak::Cloak) and out again, what makes an NPC cloak or
//!   uncloak ([`cloak::think`]), and whether the player sees a cloaked
//!   ship ([`cloak::visible`]).
//! - [`combat`]: ships fighting: firing their
//!   [`Armament`](combat::armament::Armament) on a
//!   [`Trigger`](combat::armament::Trigger) at their target, shots and
//!   beams flying and hitting, homing missiles steering, turrets aiming in
//!   their arcs, point defence shooting missiles down, sub-munitions,
//!   damage to shields then armour, the fight's
//!   [`Rules`](combat::Rules): the
//!   [`DisableRule`](combat::hull::DisableRule) port with Nova's
//!   [`NovaDisable`](combat::hull::NovaDisable) and the
//!   [`PointDefenceRule`](combat::defence::PointDefenceRule) port with
//!   Nova's [`Allegiance`](combat::defence::Allegiance), which point
//!   defence asks only by the other reading of
//!   [`RuleKey::PdHostility`], and destruction, reported as [`CombatEvent`](combat::CombatEvent)s, with
//!   each weapon feature not done yet reported once as a
//!   [`SimDiagnostic`](combat::report::SimDiagnostic).
//! - [`fleet`]: the player's fleet as data: each ship in it, an
//!   [`Escort`](fleet::Escort), and the
//!   [`EscortOrder`](fleet::EscortOrder) it follows.
//! - [`message`]: the [`SimMessage`](message::SimMessage) events a
//!   session raises for the message line, such as arriving from a jump or
//!   through a gate; the view words them.
//! - [`player_fate`]: the player's lifecycle, a
//!   [`PlayerFate`](player_fate::PlayerFate) read off its
//!   [`Condition`](combat::hull::Condition), and each fate's
//!   [`FateRow`](player_fate::FateRow): how much of the ship is
//!   [`Drawn`](player_fate::Drawn).
//! - [`sabotage`]: sabotage outfits: the flag a sabotaged loadout sets
//!   ([`sabotage_flag`](sabotage::sabotage_flag)), the
//!   [`SabotageClock`](sabotage::SabotageClock) that sets them off in
//!   flight, and the `dësc`s a sabotaged player sees as it dies.
//! - [`self_repair`]: a disabled ship repairing itself
//!   ([`can_self_repair`](self_repair::can_self_repair)), and a former
//!   escort rejoining the player as it does.
//! - [`ship_layer`]: the layers ships are drawn in
//!   ([`ShipLayer`](ship_layer::ShipLayer)): the disabled at the bottom,
//!   then the ships the player leads, then the rest with the player.
//! - [`tutorial`]: the new pilot's first-flight
//!   [`Tutorial`](tutorial::Tutorial): its hints
//!   ([`TutorialHint`](tutorial::TutorialHint)), the take-off greeting and
//!   the newbie-only "No destination".
//!
//! A doc here links only to items of this crate, `nova-sim-base`,
//! `nova-sim-catalog` and `nova-sim-state`. An item of a crate above it is
//! named by its facade path as inline code, such as `nova_sim::Session`.

// The foundation, catalog and state modules and items the moved code and
// its docs name by their old `crate::` paths, which now resolve to
// `nova-sim-base`, `nova-sim-catalog` and `nova-sim-state`.
#[allow(unused_imports)]
use nova_sim_base::{
    RuleKey, chance, flight, fuel, geometry, handling, reserves, rulebook, rules, speed_cap, trig,
};
#[allow(unused_imports)]
use nova_sim_catalog::{ControlBits, buoy, catalog, control};
#[allow(unused_imports)]
use nova_sim_state::{clearance, cron, govt, legal};

pub mod asteroid;
pub mod bay;
pub mod cloak;
pub mod combat;
pub mod fleet;
pub mod message;
pub mod player_fate;
pub mod sabotage;
pub mod self_repair;
pub mod ship_layer;
pub mod tutorial;
