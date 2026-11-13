//! What a mission's offer and abort refused or left shown
//! ([`NotOffered`], [`RefuseShown`], [`AbortError`]), from
//! `nova_sim::session::missions`, which re-exports them and keeps the
//! missions' lifecycle.

use nova_sim_state::shown::mission::active::AbortRefusal;

/// The mission is not offered on the visit under way: there is no such
/// offer, or the ship is not docked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the mission is not offered here")]
pub struct NotOffered;

/// What a refusal from the offer dialog left shown
/// (`nova_sim::Session::refuse_mission`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefuseShown {
    /// Whether the offer dialog was hidden: only when the mission's
    /// `RefuseText` is not -1 (`_OfferOneMission` @0xa2e70-0xa2e8e). One
    /// not hidden stays drawn, answered, under the texts the refusal
    /// queued and the after-movie, until they have all closed
    /// (`DisposeDialogNoFlash` @0xa361e).
    pub offer_hidden: bool,
}

/// Why `nova_sim::Session::abort_mission` aborted nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AbortError {
    /// The slot holds no mission.
    #[error("no mission is in that slot")]
    NotActive,
    /// An abort check refused it: its `CanAbort` is 0.
    #[error("the mission cannot be aborted")]
    Refused(AbortRefusal),
}

#[cfg(test)]
mod tests {
    use nova_sim_state::shown::mission::active::{AcceptError, AcceptRefusal};

    use super::*;

    #[test]
    fn a_refused_accept_or_abort_reads_plainly() {
        assert_eq!(
            AcceptError::Refused(AcceptRefusal::SlotsFull).to_string(),
            "the mission cannot be accepted: every mission slot is taken"
        );
        assert_eq!(
            AcceptError::NotOffered.to_string(),
            "the mission is not offered here"
        );
        assert_eq!(
            AbortError::Refused(AbortRefusal::CannotAbort).to_string(),
            "the mission cannot be aborted"
        );
        assert_eq!(
            AbortError::NotActive.to_string(),
            "no mission is in that slot"
        );
    }
}
