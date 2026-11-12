//! Which stellar a navigation key selects ([`StellarPick`]) and what the
//! land key did ([`LandPress`]), from `nova_sim::session`, which
//! re-exports them and keeps the selection and the landing.

use nova_sim_catalog::catalog::StellarId;
use nova_sim_state::landing::{GateKind, LandOutcome};

/// Which stellar `nova_sim::Session::select_stellar` makes the navigation
/// target, by the original's keys (`_HandlePlayer` @0x68390); a click in
/// space is `nova_sim::Session::click_in_space`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StellarPick {
    /// The stellar in this slot of the system's navigation defaults
    /// (`NavDefs`), from 0, gaps kept: the original's keys 1-4 and F1-F4
    /// (@0x69cf6-0x69f17). An empty slot, or one naming a stellar another
    /// system owns, selects nothing.
    Slot(usize),
    /// The nearest stellar with the can-land flag, destroyed or not, the one the land key selects with no
    /// target ([`nearest_stellar`](nova_sim_state::landing::nearest_stellar)): the original's F5, which picks
    /// through the same `_FindNearestStellar` (@0x276c) its land key does.
    Nearest,
}

/// What `nova_sim::Session::land` did, when it was not refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandPress {
    /// What the [`landing`](nova_sim_state::landing) rules gave: a stellar
    /// requested, or one landed on and docked at.
    Outcome(LandOutcome),
    /// The ship is cleared and over a hypergate or wormhole, and the
    /// entry awaits `nova_sim::Session::enter_hypergate` or
    /// `nova_sim::Session::enter_wormhole`: what the landing rules would let it
    /// land on, it enters instead (see `nova_sim::gate`).
    AtGate {
        /// The stellar.
        stellar: StellarId,
        /// What it is.
        kind: GateKind,
    },
}

impl From<LandOutcome> for LandPress {
    fn from(outcome: LandOutcome) -> Self {
        Self::Outcome(outcome)
    }
}
