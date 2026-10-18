//! The foundation of EV Nova's simulation: the leaves every layer of the
//! nova-sim crates builds on. `nova-sim` re-exports each module at its old
//! path (`nova_sim::rulebook`, `nova_sim::Vec2`, …), so the rest of the
//! workspace reaches them through it.
//!
//! - [`blink`]: how bright a ship's running lights are at a tick,
//!   [`lights_level`], from its `shän`'s [`Blink`] fields, with random
//!   mode rolling on the [`BlinkChance`] port.
//! - [`chance`]: the [`Chance`] port, whether a percentage chance fires,
//!   which the day's planetary events roll on, and uniform draws, which
//!   the NPC traffic rolls on.
//! - [`clock`]: the fixed-step clock, Nova's 1/30 s tick, which turns
//!   display frames of any length into whole simulation steps.
//! - [`flight`]: one tick of a ship's Newtonian flight, [`step`], under
//!   the player's [`Controls`].
//! - [`fuel`]: how much fuel a ship regenerates each tick, from its
//!   `shïp` and its outfits' fuel scoops.
//! - [`geometry`]: the simulation's own [`Vec2`], in pixels with y
//!   growing down.
//! - [`handling`]: a ship's [`Handling`], its speed, acceleration and turn
//!   rate in pixels and ticks, and the [`ShipFields`] its stats start from.
//! - [`player_turn`]: the player's turn block, `_HandlePlayer`'s: the
//!   whole-degree turn, the auto-turn toward an aim that holds the turn
//!   keys off, and the aims reverse and the jump's brake take.
//! - [`reserves`]: a ship's [`Reserves`], its shield, armour and fuel
//!   [`Gauge`]s, full when it starts.
//! - [`rule_values`]: the rules whose setting is a number, each a
//!   [`RuleValue`] the [`Rulebook`] holds, the original's by default.
//! - [`rulebook`]: the rules where the Nova Bible and the original engine
//!   disagree: each follows a [`RuleSource`], the engine's by default, as
//!   the [`Rulebook`] says, by its [`RuleKey`].
//! - [`rules`]: the rules core over any context: ordered
//!   [`Checks`](rules::Checks) refusing with a reason, [`Value`](rules::Value)s
//!   giving data, each plain, disputed (its reading chosen by the
//!   [`Rulebook`]) or drawing on the chance in rule order, and each
//!   described by a [`Descriptor`](rules::Descriptor) tools list without
//!   running it.
//! - [`speed_cap`]: the original's per-axis speed caps, `_AdjustedAccel`
//!   and `_SpeedLimit`, which flight, launches and knockback share.
//! - [`trig`]: the original's table trigonometry in whole degrees.
//!
//! A doc here links only to items of this crate. An item of a crate above
//! it is named by its facade path as inline code, such as
//! `nova_sim::Session`.

pub mod blink;
pub mod chance;
pub mod clock;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod flight;
pub mod fuel;
pub mod geometry;
pub mod handling;
pub mod inertialess;
pub mod player_turn;
pub mod reserves;
pub mod rule_values;
pub mod rulebook;
pub mod rules;
pub mod speed_cap;
pub mod trig;

pub use blink::{Blink, BlinkChance, HashedRolls, lights_level};
pub use chance::{Chance, NeverFires};
pub use clock::{FixedStep, MAX_STEPS, Steps, TICK, TICKS_PER_SECOND};
pub use flight::{Controls, Face, FlightRules, Flyer, ShipState, Turn, step};
pub use fuel::{OutfitMod, fuel_regen_per_tick};
pub use geometry::Vec2;
pub use handling::{Handling, ShipFields};
pub use reserves::{Gauge, Reserve, Reserves};
pub use rule_values::RuleValue;
pub use rulebook::{RuleKey, RuleSource, Rulebook};
