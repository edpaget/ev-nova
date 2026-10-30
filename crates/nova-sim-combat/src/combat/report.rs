//! What the simulation reports about game data it does not handle yet:
//! each [`SimDiagnostic`] once a session, as data for the program's edge
//! to write out, never logged from the core.

use std::collections::BTreeSet;
use std::fmt;

use crate::catalog::{CronId, WeaponId};
use crate::cron::{CronHook, RUN_CAP};

/// Something in the game data the simulation does not do yet, or that
/// runs away.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimDiagnostic {
    /// A weapon of a guidance the simulation does not fly (99 with no ship
    /// to carry, or a fighter bay as a sub-munition, reported as 99) or
    /// that is undocumented was fired, and did not fire.
    UnimplementedGuidance {
        /// The weapon.
        weapon: WeaponId,
        /// Its `Guidance`.
        guidance: i16,
    },
    /// A crön's iterative script stopped at [`RUN_CAP`] runs, its
    /// conditions still holding (`_ActivateCron` @0x38655,
    /// `_TerminateCron` @0x384e1).
    CronRunsCapped {
        /// The crön.
        cron: CronId,
        /// The script.
        hook: CronHook,
    },
}

impl fmt::Display for SimDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::UnimplementedGuidance { weapon, guidance } => {
                write!(f, "wëap {} Guidance {guidance} not implemented", weapon.0)
            }
            Self::CronRunsCapped { cron, hook } => {
                write!(f, "crön {} {hook} stopped after {RUN_CAP} runs", cron.0)
            }
        }
    }
}

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

    fn guidance(weapon: i16, guidance: i16) -> SimDiagnostic {
        SimDiagnostic::UnimplementedGuidance {
            weapon: WeaponId(weapon),
            guidance,
        }
    }

    #[test]
    fn each_diagnostic_reads_as_a_line() {
        assert_eq!(
            guidance(131, 1).to_string(),
            "wëap 131 Guidance 1 not implemented"
        );
        assert_eq!(
            guidance(148, 99).to_string(),
            "wëap 148 Guidance 99 not implemented"
        );
        for (hook, line) in [
            (CronHook::OnEnd, "crön 128 OnEnd stopped after 10001 runs"),
            (
                CronHook::OnStart,
                "crön 128 OnStart stopped after 10001 runs",
            ),
        ] {
            assert_eq!(
                SimDiagnostic::CronRunsCapped {
                    cron: CronId(128),
                    hook
                }
                .to_string(),
                line
            );
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
