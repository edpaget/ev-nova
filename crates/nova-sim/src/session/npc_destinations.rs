//! The session's tests of the NPCs' destinations over the system's whole
//! `NavDefs` listing (rdm play-feel-4 phase 9): in a system that lists a
//! stellar another visible system owns, which the player's system doesn't
//! show, the traffic is handed the listing, shown or not (`chart.listed`),
//! at the entry's setup (`Session::populate_as`) and in every pass
//! (`Session::step_traffic`), and the gates' widths cover every gate the
//! system lists (`Session::gate_type_widths`). These are wiring tests: the
//! pick, the landing box and the gate's reach are the traffic's, pinned in
//! `nova_sim_ai`'s `traffic::listed_stellar_tests`.
//!
//! The traffic never populates or steps while the ship is docked at a
//! stellar of another system (`chart.shown` not the player's system): a
//! move while landed brings in no traffic, every pass that steps it is a
//! flight pass, and the take-off swaps the player's system's listing in
//! (`chart.next_sites`) before it populates. So `chart.listed` is always
//! the NPCs' own system's listing when the traffic reads it.

use super::Session;
use crate::ai::{Goal, NovaAi};
use crate::catalog::{
    DudeId, DudeRecord, EscortRecord, FleetId, FleetRecord, GovtId, ShipId, StellarId, SystemId,
    SystemTraffic,
};
use crate::chance::NeverFires;
use crate::combat::Rules;
use crate::control::Test;
use crate::flight::Controls;
use crate::geometry::Vec2;
use crate::landing::TypeWidth;
use crate::testkit::{
    Draws, FAST, FakePilotCatalog, catalog, hideable, jump, npc, planet, ship, work_with,
};
use crate::traffic::autopilot::Outcome;
use crate::traffic::npc::{AiType, Mode, Npc};

/// The stellar every system lists, which 130, the lowest visible lister,
/// owns: 131 lists it but doesn't show it.
const SHARED: StellarId = StellarId(200);

/// The testkit catalog with [`SHARED`] listed by every system at (500,
/// 500), `gate` making it a hypergate, and every system hideable. 131's
/// own planet, 140, can't be landed on, so in 131 [`SHARED`] is the only
/// landable stellar.
fn twice_listed(gate: bool) -> FakePilotCatalog {
    let mut catalog = catalog();
    for (system, sites) in &mut catalog.sites {
        let mut shared = planet(200, 500.0, 500.0);
        if gate {
            shared.flags2 = crate::gate::HYPERGATE;
        }
        sites.push(shared);
        if *system == SystemId(131) {
            sites[0].flags = 0;
        }
    }
    hideable(&mut catalog);
    catalog
}

/// A session flown into 131 from 130, no chance firing.
fn in_131(catalog: &FakePilotCatalog) -> Session {
    let mut session = Session::start(catalog).expect("starts");
    jump(&mut session, catalog, 131).expect("arrives");
    assert_eq!(session.system(), SystemId(131));
    assert!(
        session.stellars_shown().all(|shown| shown != SHARED),
        "130 owns it: 131 shows it not"
    );
    session
}

/// The NPC `id` of `session`, if it is still in the system.
fn ship_of(session: &Session, id: crate::traffic::npc::NpcId) -> Option<Npc> {
    session.npcs().iter().find(|npc| npc.id == id).cloned()
}

/// Works a pass of `session` with the NPCs deciding by the original's AI.
fn pass(session: &mut Session, catalog: &FakePilotCatalog) {
    work_with(
        session,
        catalog,
        Controls::default(),
        &NovaAi::default(),
        Rules::default(),
        &mut NeverFires,
    );
}

/// A wimpy trader of `goal`, at rest at `(x, y)`.
fn trader(goal: Goal, (x, y): (f32, f32)) -> Npc {
    let mut trader = npc(1, crate::stats::ShipStats::new(FAST, &[]));
    trader.ai_type = AiType::WimpyTrader;
    trader.goal = goal;
    trader.state.position = Vec2::new(x, y);
    trader
}

#[test]
fn an_idle_npc_takes_a_stellar_its_system_lists_but_does_not_show_and_lands_on_it() {
    let catalog = twice_listed(false);
    let mut session = in_131(&catalog);
    let id = session
        .space
        .traffic
        .add_npc(trader(Goal::Idle, (0.0, 0.0)));
    for _ in 0..3000 {
        pass(&mut session, &catalog);
        let npc = ship_of(&session, id).expect("it stays");
        if npc.visited.is_some() && npc.ai_wait > 0 {
            assert_eq!(npc.visited, Some(SHARED), "landed on it");
            return;
        }
    }
    panic!("it never landed");
}

#[test]
fn every_gate_the_system_lists_has_a_width_one_it_does_not_hold_the_unloaded_one() {
    let catalog = twice_listed(true);
    let session = in_131(&catalog);
    assert_eq!(
        session.gate_type_widths().get(&SHARED),
        Some(&TypeWidth::Unloaded),
        "listed, not shown, held by 130 (@0xacb9-0xacc1)"
    );
}

#[test]
fn an_npc_bound_for_a_gate_its_system_lists_but_does_not_show_enters_it() {
    let catalog = twice_listed(true);
    let mut session = in_131(&catalog);
    let id = session
        .space
        .traffic
        .add_npc(trader(Goal::EnterGate(SHARED), (500.0, 500.0)));
    for _ in 0..100 {
        pass(&mut session, &catalog);
        if session
            .departed()
            .contains(&(id, Outcome::EnteredGate(SHARED)))
        {
            return;
        }
        let npc = ship_of(&session, id).expect("still entering");
        assert_eq!(npc.goal, Goal::EnterGate(SHARED));
    }
    panic!("it never entered the gate");
}

/// [`twice_listed`] with [`SHARED`] a hypergate, one ship on average in
/// 131, governed by 141, and `flët` 131, linked to govt 141: its fleet
/// comes in as the system is set up, out of a gate if the pick finds one.
fn fleeted() -> FakePilotCatalog {
    let mut catalog = twice_listed(true);
    let mut dude_types = [(-1, 0); 8];
    dude_types[0] = (128, 100);
    catalog.traffic = vec![(
        SystemId(131),
        SystemTraffic {
            dude_types,
            avg_ships: 1,
            persons: Default::default(),
        },
    )];
    catalog.dudes = vec![(
        DudeId(128),
        DudeRecord {
            ai_type: 1,
            govt: Some(GovtId(140)),
            ships: vec![(ShipId(129), 1)],
            booty: 0,
            info_types: 0,
        },
    )];
    catalog.ship_records = vec![ship(129, FAST), ship(130, FAST)];
    catalog.star_map[1].govt = Some(GovtId(141));
    catalog.fleets = vec![FleetRecord {
        id: FleetId(131),
        lead: Some(ShipId(129)),
        escorts: vec![EscortRecord {
            ship: ShipId(130),
            min: 1,
            max: 3,
        }],
        govt: Some(GovtId(150)),
        link_syst: 10_000 + (141 - 128),
        appear_on: Test::default(),
        quote: None,
        flags: 0,
    }];
    catalog
}

#[test]
fn the_setup_may_bring_a_fleet_out_of_a_gate_its_system_lists_but_does_not_show() {
    let catalog = fleeted();
    let mut session = in_131(&catalog);
    // The fleet in slot 3 with its escorts; then every draw its last
    // outcome, so the emerge pick takes the second of 131's two stellars.
    session.populate(&catalog, &mut Draws::of(&[1, 0, 3, 0, 2]));
    let lead = session.npcs().first().expect("the fleet came in");
    assert_eq!(lead.ship, ShipId(129));
    assert_eq!(lead.emerges_from, Some(SHARED));
    assert!(
        matches!(lead.mode, Mode::Emerging { .. }),
        "{:?}",
        lead.mode
    );
}
