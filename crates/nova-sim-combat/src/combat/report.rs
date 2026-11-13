//! What the simulation reports about game data it does not handle yet:
//! each [`SimDiagnostic`] once a session, as data for the program's edge
//! to write out, never logged from the core.

use std::collections::BTreeSet;

pub use nova_sim_state::shown::combat::report::SimDiagnostic;

/// The diagnostics a session has made, each once, and those not yet
/// taken.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reports {
    /// Every diagnostic made so far.
    made: BTreeSet<SimDiagnostic>,
    /// Those made since they were last taken, in order.
    pending: Vec<SimDiagnostic>,
}

impl Reports {
    /// Makes `diagnostic`, unless it has been made before.
    pub fn report(&mut self, diagnostic: SimDiagnostic) {
        if self.made.insert(diagnostic) {
            self.pending.push(diagnostic);
        }
    }

    /// The diagnostics made since they were last taken, in order; taking
    /// them empties the list.
    pub fn take(&mut self) -> Vec<SimDiagnostic> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::WeaponId;

    fn guidance(weapon: i16, guidance: i16) -> SimDiagnostic {
        SimDiagnostic::UnimplementedGuidance {
            weapon: WeaponId(weapon),
            guidance,
        }
    }

    #[test]
    fn a_diagnostic_is_reported_once_and_taken_once() {
        let mut reports = Reports::default();
        reports.report(guidance(181, 99));
        reports.report(guidance(165, 99));
        reports.report(guidance(181, 99));
        assert_eq!(reports.take(), [guidance(181, 99), guidance(165, 99)]);
        assert_eq!(reports.take(), [], "taken");
        reports.report(guidance(181, 99));
        assert_eq!(reports.take(), [], "made before");
    }
}
