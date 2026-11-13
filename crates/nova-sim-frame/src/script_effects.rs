//! What settling a script's moves and messages did ([`Settled`]) and what
//! a take-off did ([`Departure`]), from `nova_sim::session::script_effects`,
//! which re-exports them and keeps the set operators.

use nova_sim_catalog::catalog::{StellarId, SystemId};

/// What `nova_sim::Session::settle_script` did.
#[must_use]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settled {
    /// The system the last move applied went to, if any did.
    pub moved: Option<SystemId>,
    /// The message a `Q` in flight shows, if it has one. A landed `Q` is
    /// held for the take-off instead (`nova_sim::Session::leave_due`).
    pub message: Option<String>,
}

/// What `nova_sim::Session::leave_port` did: the stellar taken off from, and what
/// settling the take-off did.
#[must_use]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Departure {
    /// The stellar the ship took off from.
    pub stellar: StellarId,
    /// What settling the take-off's events did.
    pub settled: Settled,
}
