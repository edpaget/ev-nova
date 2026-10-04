//! What the simulation reports about game data it does not handle yet:
//! each [`SimDiagnostic`] once a session, as data for the program's edge
//! to write out, never logged from the core.

use std::collections::BTreeSet;
use std::fmt;

use super::flags::{self, FlagField};
use super::weapon::WeaponSpec;
use crate::catalog::WeaponId;

/// Something in the game data the simulation does not do yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimDiagnostic {
    /// A weapon fired with a flag bit the simulation ignores.
    UnimplementedWeaponFlag {
        /// The weapon.
        weapon: WeaponId,
        /// Which of its flag words.
        field: FlagField,
        /// The bit.
        bit: u16,
    },
    /// A weapon of a guidance a later phase flies was fired, and did not
    /// fire.
    UnimplementedGuidance {
        /// The weapon.
        weapon: WeaponId,
        /// Its `Guidance`.
        guidance: i16,
    },
}

impl fmt::Display for SimDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::UnimplementedWeaponFlag { weapon, field, bit } => {
                write!(f, "wëap {} {field} {bit:#06x} not implemented", weapon.0)
            }
            Self::UnimplementedGuidance { weapon, guidance } => {
                write!(f, "wëap {} Guidance {guidance} not implemented", weapon.0)
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

    /// Reports each flag `weapon` sets that is not done yet, as it fires.
    pub fn fired(&mut self, weapon: &WeaponSpec) {
        for (field, bit) in flags::unimplemented(weapon) {
            self.report(SimDiagnostic::UnimplementedWeaponFlag {
                weapon: weapon.id,
                field,
                bit,
            });
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
    use crate::catalog::WeaponRecord;
    use crate::testkit::weapon;

    fn flag(weapon: i16, field: FlagField, bit: u16) -> SimDiagnostic {
        SimDiagnostic::UnimplementedWeaponFlag {
            weapon: WeaponId(weapon),
            field,
            bit,
        }
    }

    #[test]
    fn each_diagnostic_reads_as_a_line() {
        assert_eq!(
            flag(181, FlagField::Flags2, 0x8000).to_string(),
            "wëap 181 Flags2 0x8000 not implemented"
        );
        assert_eq!(
            flag(128, FlagField::Flags, 0x40).to_string(),
            "wëap 128 Flags 0x0040 not implemented"
        );
        assert_eq!(
            SimDiagnostic::UnimplementedGuidance {
                weapon: WeaponId(131),
                guidance: 1
            }
            .to_string(),
            "wëap 131 Guidance 1 not implemented"
        );
    }

    #[test]
    fn a_diagnostic_is_reported_once_and_taken_once() {
        let mut reports = Reports::default();
        reports.report(flag(181, FlagField::Flags2, 0x8000));
        reports.report(flag(165, FlagField::Flags2, 0x8000));
        reports.report(flag(181, FlagField::Flags2, 0x8000));
        assert_eq!(
            reports.take(),
            [
                flag(181, FlagField::Flags2, 0x8000),
                flag(165, FlagField::Flags2, 0x8000)
            ]
        );
        assert_eq!(reports.take(), [], "taken");
        reports.report(flag(181, FlagField::Flags2, 0x8000));
        assert_eq!(reports.take(), [], "made before");
    }

    #[test]
    fn a_weapon_firing_reports_each_unimplemented_flag_it_sets() {
        let mining = WeaponSpec::new(&WeaponRecord {
            flags: 0x0140,
            flags2: 0x8000,
            ..weapon(181)
        });
        let mut reports = Reports::default();
        reports.fired(&mining);
        reports.fired(&mining);
        assert_eq!(
            reports.take(),
            [
                flag(181, FlagField::Flags, 0x0040),
                flag(181, FlagField::Flags2, 0x8000)
            ]
        );
        reports.fired(&WeaponSpec::new(&weapon(128)));
        assert_eq!(reports.take(), []);
    }
}
