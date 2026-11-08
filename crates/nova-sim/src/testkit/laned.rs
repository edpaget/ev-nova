//! A galaxy of jump points alone, shaped like nova-data's new-game fixture
//! (`crates/nova-data/tests/fixtures/new-game/systems`): three systems
//! joined only by hypergate pairs, with no `sÿst` Con link anywhere.

use super::{FakePilotCatalog, catalog, planet, star};
use crate::catalog::{GateSite, GateStability, LandingSite, StellarId, SystemId};
use crate::gate::HYPERGATE;
use crate::geometry::Vec2;

/// Drift, where the pilot starts: a system of gates alone.
pub(crate) const DRIFT: SystemId = SystemId(130);
/// Hearth, the hub.
pub(crate) const HEARTH: SystemId = SystemId(131);
/// Verge, two roads from hearth.
pub(crate) const VERGE: SystemId = SystemId(132);

/// Each gate: its stellar, its system, where it sits and its one link.
type Gate = (i16, SystemId, (f32, f32), i16);
const GATES: [Gate; 9] = [
    // drift/hearth-road ↔ hearth/drift-road, at drift's centre, where the
    // pilot starts.
    (300, DRIFT, (0.0, 0.0), 310),
    // drift/verge-road ↔ verge/drift-road.
    (301, DRIFT, (0.0, -1100.0), 322),
    // drift/back-door → verge/side-road, one way.
    (302, DRIFT, (-1000.0, -1000.0), 321),
    (310, HEARTH, (-1200.0, 0.0), 300),
    // hearth/verge-road ↔ verge/hearth-road.
    (311, HEARTH, (1200.0, 0.0), 320),
    // hearth/side-road ↔ verge/side-road.
    (312, HEARTH, (0.0, 1300.0), 321),
    (320, VERGE, (-1200.0, 0.0), 311),
    (321, VERGE, (0.0, -1300.0), 312),
    (322, VERGE, (0.0, 1250.0), 301),
];

/// Hearth, drift and verge, with hypergate pairs only and no Con: a
/// drift↔hearth pair (300, 310), two gates hearth→verge each paired with
/// a gate in verge (311↔320, 312↔321), a drift↔verge pair (301↔322) and a
/// one-way drift→verge back door (302 → 321). Each gate is a landable
/// stellar of its system. The pilot starts in drift, over its gate to
/// hearth (300).
///
/// The stabilities are the fixture's: the drift↔hearth pair stable, one
/// hearth→verge gate's far end (verge's 320) unstable, the drift↔verge
/// pair dead and the back door unstable. So drift→verge is unstable (the
/// back door beats the dead pair) and verge→drift dead, both closed, and
/// hearth↔verge stable both ways (by 312↔321), open. [`laned_stable`]
/// has every gate stable, for the tests whose subject isn't stability.
pub(crate) fn laned() -> FakePilotCatalog {
    laned_stable()
        .with_gate_stability(320, GateStability::Unstable)
        .with_gate_stability(301, GateStability::Dead)
        .with_gate_stability(322, GateStability::Dead)
        .with_gate_stability(302, GateStability::Unstable)
}

/// [`laned`] with every gate stable: every lane open, drift↔verge both
/// ways by the pair 301↔322 and drift→verge by the back door too.
pub(crate) fn laned_stable() -> FakePilotCatalog {
    let site = |id: i16, (x, y): (f32, f32)| LandingSite {
        flags2: HYPERGATE,
        ..planet(id, x, y)
    };
    let sites = [DRIFT, HEARTH, VERGE]
        .map(|system| {
            let held = GATES
                .iter()
                .filter(|gate| gate.1 == system)
                .map(|&(id, _, at, _)| site(id, at))
                .collect();
            (system, held)
        })
        .to_vec();
    let gates = GATES
        .iter()
        .map(|&(id, _, (x, y), to)| {
            let mut links = [None; 8];
            links[0] = Some(StellarId(to));
            GateSite {
                id: StellarId(id),
                position: Vec2::new(x, y),
                flags2: HYPERGATE,
                links,
                exit_angle: -1,
                stability: GateStability::Stable,
            }
        })
        .collect();
    FakePilotCatalog {
        systems: vec![DRIFT, HEARTH, VERGE],
        sites,
        star_map: vec![
            star(DRIFT.0, (150.0, 200.0), &[]),
            star(HEARTH.0, (0.0, 0.0), &[]),
            star(VERGE.0, (300.0, -120.0), &[]),
        ],
        gates,
        ..catalog()
    }
}
