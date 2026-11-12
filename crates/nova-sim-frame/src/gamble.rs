//! The names of `nova_sim::gamble` the view reaches, re-exported from the
//! crates below the layers that nova-sim's `gamble` comes from, where they now
//! live: the same items at the same path, so the view can name them through
//! this crate instead of nova-sim.

pub use nova_sim_state::shown::gamble::{
    AMOUNT_TO_BET, BET, Bet, GambleOpening, NO_CASH_TO_BET, RACE_HELP_DESC, RACERS, RaceRun,
    result_text,
};
