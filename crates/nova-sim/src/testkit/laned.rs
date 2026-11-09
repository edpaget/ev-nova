//! A galaxy of jump points alone, shaped like nova-data's new-game fixture
//! (`crates/nova-data/tests/fixtures/new-game/systems`): three systems
//! joined only by hypergate pairs, with no `sÿst` Con link anywhere.

use super::{FakePilotCatalog, catalog, planet, star};
use crate::catalog::{GateSite, LandingSite, StellarId, SystemId};
use crate::gate::HYPERGATE;
use crate::geometry::Vec2;
use nova_sim_catalog::storylines::PointStability::{self, Dead, Stable, Unstable};

/// Drift, where the pilot starts: a system of gates alone.
pub(crate) const DRIFT: SystemId = SystemId(130);
/// Hearth, the hub.
pub(crate) const HEARTH: SystemId = SystemId(131);
/// Verge, two roads from hearth.
pub(crate) const VERGE: SystemId = SystemId(132);

/// Each gate: its stellar, its system, where it sits, its one link, its
/// jump point (`system/point`) and that point's stability in the fixture.
type Gate = (i16, SystemId, (f32, f32), i16, &'static str, PointStability);
const GATES: [Gate; 9] = [
    // drift/hearth-road ↔ hearth/drift-road, at drift's centre, where the
    // pilot starts.
    (300, DRIFT, (0.0, 0.0), 310, "drift/hearth-road", Stable),
    // drift/verge-road ↔ verge/drift-road.
    (301, DRIFT, (0.0, -1100.0), 322, "drift/verge-road", Dead),
    // drift/back-door → verge/side-road, one way.
    (
        302,
        DRIFT,
        (-1000.0, -1000.0),
        321,
        "drift/back-door",
        Unstable,
    ),
    (
        310,
        HEARTH,
        (-1200.0, 0.0),
        300,
        "hearth/drift-road",
        Stable,
    ),
    // hearth/verge-road ↔ verge/hearth-road.
    (311, HEARTH, (1200.0, 0.0), 320, "hearth/verge-road", Stable),
    // hearth/side-road ↔ verge/side-road.
    (312, HEARTH, (0.0, 1300.0), 321, "hearth/side-road", Stable),
    (
        320,
        VERGE,
        (-1200.0, 0.0),
        311,
        "verge/hearth-road",
        Unstable,
    ),
    (321, VERGE, (0.0, -1300.0), 312, "verge/side-road", Stable),
    (322, VERGE, (0.0, 1250.0), 301, "verge/drift-road", Dead),
];

/// Hearth, drift and verge, with hypergate pairs only and no Con: a
/// drift↔hearth pair (300, 310), two gates hearth→verge each paired with
/// a gate in verge (311↔320, 312↔321), a drift↔verge pair (301↔322) and a
/// one-way drift→verge back door (302 → 321). Each gate is a landable
/// stellar of its system. The pilot starts in drift, over its gate to
/// hearth (300).
///
/// Each gate is its jump point's, named as the fixture names it
/// (`verge/drift-road`), among the storylines' points, with no owner. The
/// stabilities are the fixture's: the drift↔hearth pair stable, one
/// hearth→verge gate's far end (verge's 320) unstable, the drift↔verge
/// pair dead and the back door unstable. So drift→verge is unstable (the
/// back door beats the dead pair) and verge→drift dead, both closed, and
/// hearth↔verge stable both ways (by 312↔321), open. [`laned_stable`]
/// has every point stable, for the tests whose subject isn't stability.
pub(crate) fn laned() -> FakePilotCatalog {
    laned_at(|stability| stability)
}

/// [`laned`] with every point stable: every lane open, drift↔verge both
/// ways by the pair 301↔322 and drift→verge by the back door too.
pub(crate) fn laned_stable() -> FakePilotCatalog {
    laned_at(|_| Stable)
}

/// [`laned`] with each point's stability as `stability` gives it from the
/// fixture's.
fn laned_at(stability: impl Fn(PointStability) -> PointStability) -> FakePilotCatalog {
    let site = |id: i16, (x, y): (f32, f32)| LandingSite {
        flags2: HYPERGATE,
        ..planet(id, x, y)
    };
    let sites = [DRIFT, HEARTH, VERGE]
        .map(|system| {
            let held = GATES
                .iter()
                .filter(|gate| gate.1 == system)
                .map(|&(id, _, at, ..)| site(id, at))
                .collect();
            (system, held)
        })
        .to_vec();
    let gates = GATES
        .iter()
        .map(|&(id, _, (x, y), to, ..)| {
            let mut links = [None; 8];
            links[0] = Some(StellarId(to));
            GateSite {
                id: StellarId(id),
                position: Vec2::new(x, y),
                flags2: HYPERGATE,
                links,
                exit_angle: -1,
            }
        })
        .collect();
    let catalog = FakePilotCatalog {
        systems: vec![DRIFT, HEARTH, VERGE],
        sites,
        star_map: vec![
            star(DRIFT.0, (150.0, 200.0), &[]),
            star(HEARTH.0, (0.0, 0.0), &[]),
            star(VERGE.0, (300.0, -120.0), &[]),
        ],
        gates,
        ..catalog()
    };
    GATES
        .iter()
        .fold(catalog, |catalog, &(id, _, _, _, point, at)| {
            catalog.with_point(point, id, stability(at))
        })
}
