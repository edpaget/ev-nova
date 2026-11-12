//! A news item a system has heard ([`HeardNews`]), from
//! `nova_sim_story::storylines::news`, which re-exports it and keeps how
//! news spreads.

use nova_sim_catalog::catalog::SystemId;

/// A news item a system has heard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeardNews {
    /// Its id.
    pub id: String,
    /// Its headline.
    pub headline: String,
    /// The news as told.
    pub text: String,
    /// The system where it broke.
    pub from: SystemId,
    /// How many days ago the system heard it.
    pub age: u32,
    /// Whether the pilot has read it (`nova_sim_story::storylines::state::StoryState::read`), here or in
    /// any other system.
    pub read: bool,
}
