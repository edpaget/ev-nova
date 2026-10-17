//! Test support for the story layer's tests and the nova-sim crates'
//! above, behind the `fixture` feature: an [`ActiveMission`] built
//! plainly, for tests to change with struct-update syntax.

use crate::catalog::MissionId;
use crate::mission::aux_ships::AuxShips;
use crate::mission::{ActiveMission, SpecialShips};

/// Mission `id` active, going nowhere, with `tons` of cargo 0 (food)
/// aboard.
#[must_use]
pub fn carrying(id: i16, tons: i64) -> ActiveMission {
    ActiveMission {
        mission: MissionId(id),
        destination: None,
        return_to: None,
        cargo_type: Some(0),
        cargo_tons: tons,
        pay: 0,
        days_left: None,
        deadline: None,
        reached: false,
        goals_met: true,
        aboard: true,
        failed: None,
        ships: SpecialShips::default(),
        aux: AuxShips::default(),
    }
}
