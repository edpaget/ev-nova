//! The story layer of EV Nova's simulation: the missions offered and
//! under way, the story runtime that settles their events, the persons,
//! ranks and news, nebulae and exploration, the text tags and conditional
//! text every loaded text goes through, and the [`Pilot`](pilot::Pilot)
//! a save keeps. It builds on the foundation, `nova-sim-base`, the
//! catalog, `nova-sim-catalog`, the state, `nova-sim-state`, combat,
//! `nova-sim-combat`, and the ai and traffic layer, `nova-sim-ai`, since
//! the story acts on the ships that layer flies. `nova-sim` re-exports
//! each module at its old path (`nova_sim::mission`, `nova_sim::Pilot`,
//! …), so the rest of the workspace reaches them through it.
//!
//! - [`exchange_events`]: what the commodity exchange says of the `öops`
//!   price events at the stellar docked at, the disputed
//!   [`exchange_events::exchange_events`]: the first local event's line
//!   ([`exchange_events::event_line`]), or every local event's name.
//! - [`exploration`]: which systems a map outfit explores
//!   ([`exploration::map_reveals`]) and which are inhabited.
//! - [`mission`]: mission offers: which missions a docked stellar offers
//!   at its mission computer and outside it ([`mission::availability`],
//!   each field a plain [`Check`](nova_sim_base::rules::Check) refusing
//!   with a [`mission::Refusal`]), the stellars a mission's codes choose
//!   ([`mission::select_stellar`]), each [`mission::Offer`]'s resolved
//!   values ([`mission::resolve`]), and the per-visit [`mission::Offers`]
//!   with the roll table `AvailRandom` reads.
//! - [`nebula`]: nebulae: when one is active, which systems it holds, and
//!   when an exploration explores it, as the original's `nëbu` table
//!   keeps them.
//! - [`news`]: the bar's news, a commercial line and a story
//!   ([`News`](news::News)), and which running `crön`s have a story for
//!   the stellar docked at ([`news::candidates`], with the disputed
//!   [`news::news_govt_match`]), and the `öops` price story
//!   ([`news::oops_story`]), drawn by the session as the bar opens.
//! - [`person`]: persons (`përs`): where they may appear
//!   ([`PersonLink`](person::PersonLink)), how often
//!   ([`PersonRules`](person::PersonRules), Nova's
//!   [`NovaPersons`](person::NovaPersons)), the ship each flies
//!   ([`person::fit`]), and when it says its hail quote
//!   ([`person::quote_eligible`], [`person::expand_tags`]).
//! - [`pilot`]: the [`Pilot`](pilot::Pilot), everything about the player
//!   a save keeps: ship, location, date, cash, reserves, course, explored
//!   systems, legal records, cargo, the events under way, the outfits
//!   owned and the fleet of [`Escort`](pilot::Escort)s, the control bits
//!   and the player's [`Gender`](pilot::Gender) and its ship's paint and
//!   name, starting from the first `chär`.
//! - [`rank`]: the ranks, as `_ActivateRank` and `_DeactivateRank` change
//!   them, their salaries, `Contribute` bits and names in the text tags.
//! - [`story`]: the story runtime's design, with the original's event
//!   sources and orderings, and the pure types it names: the
//!   [`StoryEvent`](story::StoryEvent)s, the
//!   [`Subscription`](story::Subscription)s consumers register and the
//!   order they run in ([`dispatch_order`](story::dispatch_order)), and
//!   the [`GalaxyOverlay`](story::GalaxyOverlay) play changes over the
//!   static galaxy, which every reader sees through the
//!   [`OverlaidGalaxy`](story::OverlaidGalaxy).
//! - [`text_parse`]: the conditional text (`{G "he" "she"}`, `{bnnn …}`,
//!   `{Pnnn …}`) every loaded text is resolved by, as the original's
//!   `_ParseTextDescription` ([`text_parse::conditional_text`]).
//! - [`text_tags`]: the one substitution path for the text tags, as the
//!   original's `_MungeBriefing`: a
//!   [`TagContext`](text_tags::TagContext) of the pilot, the person, the
//!   ranks and the mission's resolved
//!   [`MissionValues`](text_tags::MissionValues), its names read through
//!   the [`TagNames`](text_tags::TagNames) port
//!   ([`text_tags::expand`]).
//!
//! A doc here links only to items of this crate, `nova-sim-base`,
//! `nova-sim-catalog`, `nova-sim-state`, `nova-sim-combat` and
//! `nova-sim-ai`. An item of a crate above it is named by its facade path
//! as inline code, such as `nova_sim::Session`.

// The foundation, catalog, state, combat and ai modules the moved code and
// its docs name by their old `crate::` paths (and `RuleKey`, which the
// docs link by `crate::RuleKey`), which now resolve to `nova-sim-base`,
// `nova-sim-catalog`, `nova-sim-state`, `nova-sim-combat` and
// `nova-sim-ai`.
#[allow(unused_imports)]
use nova_sim_ai::{ai, cargo_box, escort, gate, hail, hyperspace, scan, traffic};
#[allow(unused_imports)]
use nova_sim_base::{RuleKey, chance, geometry, reserves, rulebook, rules};
#[allow(unused_imports)]
use nova_sim_catalog::{catalog, control, strings, warp};
#[allow(unused_imports)]
use nova_sim_combat::{cloak, combat, fleet};
#[allow(unused_imports)]
use nova_sim_state::{
    cron, date, govt, landing, legal, market, outfit_effects, stats, trade_dialog,
};

pub mod exchange_events;
pub mod exploration;
pub mod mission;
pub mod nebula;
pub mod news;
pub mod person;
pub mod pilot;
pub mod rank;
pub mod story;
pub mod text_parse;
pub mod text_tags;
