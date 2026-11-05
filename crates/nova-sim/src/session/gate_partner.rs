//! Which gate a hypergate's entry comes out of: the linked `spöb` itself,
//! not merely a gate of the linked system.
//!
//! A `spöb`'s `HyperLink1-8` hold `spöb` IDs. `_PlayerEnterHypergate`
//! takes the first of the entered gate's own links whose system is the
//! one picked (@0x6389d-0x63999, `nova_sim_ai`'s `gate::hypergate_exit`),
//! and the ship comes out at that linked `spöb`'s position
//! (@0x63a7f-0x63aa6). Nothing reads whether the linked gate links back.
//! So two gates of one system leading to one neighbour each come out at
//! their own partner, and a one-way gate, whose partner links elsewhere,
//! still comes out at that partner. These tests pin it over today's code
//! (step 1, check (a) of the new game's fly-the-sample plan).

use crate::catalog::{LandingSite, StellarId, SystemId};
use crate::chance::NeverFires;
use crate::gate::{GateKind, HYPERGATE};
use crate::geometry::Vec2;
use crate::navigation::NavMode;
use crate::session::tests::gate_site;
use crate::session::{LandPress, Session};
use crate::testkit::{FakePilotCatalog, catalog};
use nova_sim_catalog::fixture::planet;

/// A landable hypergate `id` at (`x`, `y`).
fn gate(id: i16, x: f32, y: f32) -> LandingSite {
    LandingSite {
        flags2: HYPERGATE,
        ..planet(id, x, y)
    }
}

/// System 130 holds three hypergates: 300 at the centre, linked to 310
/// in 131 at (100, 200); 301 at (1000, 0), linked to 311 in 131 at
/// (-400, -300); and 302 at (-1000, 0), one-way, linked to 311 too,
/// which links back to 301 alone.
fn two_roads() -> FakePilotCatalog {
    FakePilotCatalog {
        sites: vec![
            (
                SystemId(130),
                vec![
                    gate(300, 0.0, 0.0),
                    gate(301, 1000.0, 0.0),
                    gate(302, -1000.0, 0.0),
                ],
            ),
            (
                SystemId(131),
                vec![gate(310, 100.0, 200.0), gate(311, -400.0, -300.0)],
            ),
        ],
        gates: vec![
            gate_site(300, (0.0, 0.0), HYPERGATE, &[310], 90),
            gate_site(301, (1000.0, 0.0), HYPERGATE, &[311], 90),
            gate_site(302, (-1000.0, 0.0), HYPERGATE, &[311], 90),
            gate_site(310, (100.0, 200.0), HYPERGATE, &[300], 90),
            gate_site(311, (-400.0, -300.0), HYPERGATE, &[301], 90),
        ],
        gate_listers: vec![
            (SystemId(130), StellarId(300)),
            (SystemId(130), StellarId(301)),
            (SystemId(130), StellarId(302)),
            (SystemId(131), StellarId(310)),
            (SystemId(131), StellarId(311)),
        ],
        ..catalog()
    }
}

/// Takes the hypergate at `at`, `stellar`, to 131, and gives where the
/// ship comes out.
fn through(catalog: &FakePilotCatalog, stellar: i16, at: Vec2) -> Vec2 {
    let mut session = Session::start(catalog).expect("starts");
    session.ship.player.position = at;
    session.ship.player.velocity = Vec2::ZERO;
    session.chart.nav = NavMode::Off;
    session.land().expect("requests");
    session.dock_approach();
    assert_eq!(
        session.land(),
        Ok(LandPress::AtGate {
            stellar: StellarId(stellar),
            kind: GateKind::Hypergate,
        })
    );
    assert_eq!(session.open_hypergate(catalog), Ok(vec![SystemId(131)]));
    assert_eq!(
        session.enter_hypergate(Some(SystemId(131)), catalog, &mut NeverFires),
        Ok(SystemId(131))
    );
    session.player().position
}

#[test]
fn two_gates_to_one_neighbour_each_come_out_at_their_own_partner() {
    let catalog = two_roads();
    assert_eq!(
        through(&catalog, 300, Vec2::ZERO),
        Vec2::new(100.0, 200.0),
        "300 at 310"
    );
    assert_eq!(
        through(&catalog, 301, Vec2::new(1000.0, 0.0)),
        Vec2::new(-400.0, -300.0),
        "301 at 311"
    );
}

#[test]
fn a_one_way_gate_comes_out_at_its_partner_though_the_partner_links_elsewhere() {
    let catalog = two_roads();
    assert_eq!(
        through(&catalog, 302, Vec2::new(-1000.0, 0.0)),
        Vec2::new(-400.0, -300.0),
        "302 at 311, which leads back to 301"
    );
}
