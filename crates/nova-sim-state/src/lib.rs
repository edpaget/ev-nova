//! The state layer of EV Nova's simulation: the pilot's and the galaxy's
//! standing state that the subsystems above read and change, such as the
//! date, the governments and the legal record, the market, the crons and
//! a ship's computed stats. It builds on the foundation, `nova-sim-base`,
//! and the catalog, `nova-sim-catalog`. `nova-sim` re-exports each module
//! at its old path (`nova_sim::market`, `nova_sim::GameDate`, …), so the
//! rest of the workspace reaches them through it.
//!
//! - [`clearance`]: the dock request the land key makes
//!   ([`DockRequest`](clearance::DockRequest)), its counter, and the dock
//!   approach that counts it on every frame and greets the pilot
//!   ([`approach`](clearance::approach)).
//! - [`cron`]: the `crön`s the day's second participant runs: whether an
//!   inactive one starts ([`cron::eligibility`], with its disputed
//!   `Random` and date range), the daily pass over their holdoffs and
//!   durations, each one's [`CronState`](cron::CronState), and whose
//!   `Contribute` bits count ([`cron::contributes`]).
//! - [`date`]: the in-game [`GameDate`](date::GameDate), a Gregorian day
//!   that advances a day for each day a jump takes, and
//!   [`date::date_text`], how it is displayed.
//! - [`govt`]: the [`Governments`](govt::Governments) and their
//!   relations: allies, enemies, xenophobes and the flags that say how
//!   their ships behave.
//! - [`landing`]: what the land key does,
//!   [`land_or_select`](landing::land_or_select): request clearance,
//!   then, once the dock approach has cleared the request, land (or, over
//!   a hypergate or wormhole, enter it); whether traffic control clears
//!   the pilot, [`cleared`](landing::cleared); whether the ship can land,
//!   [`check_landing`](landing::check_landing), each
//!   [`LandingRefusal`](landing::LandingRefusal) in the order it applies;
//!   and the [`Service`](landing::Service)s a stellar's flags offer.
//! - [`legal`]: the player's legal record, kept with each government and
//!   in each system: the [`LegalCode`](legal::LegalCode) port that says
//!   what a [`Crime`](legal::Crime) against a ship does to it, with each
//!   government and along its walk through the
//!   [`CrimeMap`](legal::CrimeMap) from system to system, Nova's
//!   [`NovaLaw`](legal::NovaLaw), [`legal::convict`], which applies it,
//!   and [`legal::systems_from_govts`], which derives the records in each
//!   system, and [`legal::spread_statuses`], which spreads a new pilot's
//!   `chär` statuses.
//! - [`market`]: the commodity exchange: what a stellar trades and at
//!   what price ([`Market`](market::Market)), cargo space, buying and
//!   selling ([`Order`](market::Order)), and the planetary events that
//!   move prices.
//! - [`outfit_effects`]: what granting an outfit does: a map explores, a
//!   clean-record outfit cleans the legal record and a paint paints the
//!   ship ([`Rgb15`](outfit_effects::Rgb15)) instead of being added
//!   ([`outfit_effects::GrantEffect`]), and the disputed
//!   [`OutfitRules`](outfit_effects::OutfitRules).
//! - [`stats`]: a ship's computed [`ShipStats`](stats::ShipStats), from
//!   its `shïp`'s fields and the outfits it carries: the one place its
//!   handling, reserve capacities, fuel, shield and armour regeneration
//!   and cargo space come from.
//!
//! A doc here links only to items of this crate, `nova-sim-base` and
//! `nova-sim-catalog`. An item of a crate above it is named by its facade
//! path as inline code, such as `nova_sim::Session`.

// The foundation and catalog modules and items the moved code and its
// docs name by their old `crate::` paths, which now resolve to
// `nova-sim-base` and `nova-sim-catalog`.
#[allow(unused_imports)]
use nova_sim_base::{
    RuleKey, chance, flight, fuel, geometry, handling, inertialess, reserves, rulebook, rules,
};
#[allow(unused_imports)]
use nova_sim_catalog::{ControlBits, catalog, control};

pub mod clearance;
pub mod cron;
pub mod date;
pub mod govt;
pub mod landing;
pub mod legal;
pub mod market;
pub mod outfit_effects;
pub mod stats;
