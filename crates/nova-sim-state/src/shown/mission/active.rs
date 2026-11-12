//! Why a mission is not accepted ([`AcceptError`], [`AcceptRefusal`]) or
//! aborted ([`AbortRefusal`]), and how many missions can be active
//! ([`SLOTS`]), from `nova_sim_story::mission::active`, which re-exports
//! them and keeps the active missions and the accept and abort checks.

/// How many missions can be active at once: the original's slots
/// (`_currentMissions`, 0x184688).
pub const SLOTS: usize = 16;

/// Why a mission is not accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AcceptRefusal {
    /// All 16 slots are taken.
    #[error("every mission slot is taken")]
    SlotsFull,
    /// The ship's holds are fewer than the cargo's tons.
    #[error("the ship's cargo holds are too few for the mission's cargo")]
    NoHolds,
    /// The ship's free cargo space is less than the cargo's tons.
    #[error("the ship's free cargo space is too little for the mission's cargo")]
    NoSpace,
}

/// Why `nova_sim::Session::accept_mission` accepted nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AcceptError {
    /// It is not offered here.
    #[error("the mission is not offered here")]
    NotOffered,
    /// An accept check refused it.
    #[error("the mission cannot be accepted: {0}")]
    Refused(AcceptRefusal),
}

/// Why the player cannot abort a mission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbortRefusal {
    /// Its `CanAbort` is 0.
    CannotAbort,
}
