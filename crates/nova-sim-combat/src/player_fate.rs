//! The player's lifecycle: one [`PlayerFate`] for each stage of flying,
//! breaking up and dying, read off the player's
//! [`Condition`](crate::combat::hull::Condition), and one table row
//! ([`FateRow`]) for what each stage allows. They are
//! [`nova_sim_state::shown::player_fate`]'s, re-exported here.

pub use nova_sim_state::shown::player_fate::{
    Drawn, Ending, FateRow, HOLD_TICKS, POD_TICKS, PlayerCommand, PlayerFate, Takes,
};
