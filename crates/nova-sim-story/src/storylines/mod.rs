//! The new game's missions: when one is offered or happens, what its
//! outcomes and effects do to world state, and how its news spreads.
//!
//! The data (`missions/` and `news/` in the format, served as the
//! catalog's [`Storylines`](nova_sim_catalog::storylines::Storylines))
//! says what each mission requires and sets. This module holds the rules,
//! pure over that data and the pilot's state: the world flags
//! ([`Pilot::flags`](crate::pilot::Pilot::flags)) and the story state
//! ([`Pilot::story`](crate::pilot::Pilot::story)). The session plays the
//! conversations and calls in here (`nova_sim::Session`'s `storylines`
//! module).
//!
//! # State
//!
//! World state is the flags and each jump point's owner and stability. A
//! point stands as the data writes it until an effect changes it: play's
//! changes are an overlay ([`StoryState::points`](state::StoryState::points)),
//! read field by field over the data's ([`StoryState::point`](state::StoryState::point)).
//! Travel doesn't read it yet; `play-the-sample` phase 1 repoints gates,
//! lanes, the map and traffic to it.
//!
//! The story state is keyed by the data's ids as text, never by Nova IDs,
//! which renumber as data files are added. It holds the day of the game,
//! each mission's [`MissionState`](state::MissionState), the points'
//! overlay and the day each news item broke. The stock game has no
//! storylines, so its pilots' story state stays empty.
//!
//! # Rules
//!
//! - [`requires::holds`]: whether a mission's conditions hold.
//! - [`effects::apply`]: one effect block.
//!
//! The day of the game counts from 0, for a new pilot, by one for each
//! day passed; `not_before_day: n` holds from day `n`.

pub mod effects;
pub mod requires;
pub mod state;
