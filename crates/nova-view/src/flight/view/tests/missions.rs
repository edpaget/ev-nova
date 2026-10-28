use super::*;
use crate::flight::sim_port::PortSource;

// The mission screens.

/// The adapter's own check
/// (rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks): the flight's
/// offer texts are the session's. The `OfferNames` impl over them runs on
/// `FakeFlight` (`tests/faked_missions.rs`).
#[test]
fn the_offer_names_are_the_sessions() {
    let mut errands = catalog();
    errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    errands.missions = vec![errand(130), errand(131)];
    let mut view = test_flight::new(errands);
    land_now(&mut view);
    view.settle_script();
    let munged = view.sim.offer_text(MissionId(130), "Run <PAY>");
    assert_eq!(
        munged,
        view.session()
            .ok()
            .and_then(|session| session.offer_text(MissionId(130), "Run <PAY>")),
        "the session's munge"
    );
    assert!(munged.is_some_and(|name| !name.contains("<PAY>")), "filled");
}

/// `base` with asteroid type 0, its sheet `rlëD` 1800, and system 130
/// holding 3 of it.
pub(super) fn rocky(base: FakeCatalog) -> FakeCatalog {
    FakeCatalog {
        asteroid_types: vec![nova_sim::AsteroidTypeRecord {
            kind: 0,
            strength: 50,
            spin_rate: 100,
            frames: 36,
            frame_size: Some((40, 40)),
            ..nova_sim::AsteroidTypeRecord::default()
        }],
        asteroid_fields: vec![nova_sim::AsteroidFieldRecord {
            system: SystemId(130),
            count: 3,
            types: 0x0001,
        }],
        ..base
    }
}

/// The adapter's own check (rdm:phase/view-sim-seam/phase-11-spaceport-on-port):
/// a spaceport offer's dialog has a refuse button as the session says.
/// What the view redraws for it runs on `FakeFlight`
/// (`tests/faked_missions.rs`).
#[test]
fn an_offers_refuse_button_is_the_sessions() {
    let mut errands = catalog();
    errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    errands.missions = vec![MissionRecord {
        avail_loc: SPACEPORT,
        ..errand(130)
    }];
    let mut view = test_flight::new(errands);
    land_now(&mut view);
    view.settle_script();
    view.enter_offer_venue(SPACEPORT);
    assert!(matches!(
        view.next_offer(SPACEPORT),
        Some(VenueOffer::Prompt(_))
    ));
    assert!(view.sim.refusable_offer_dialog(MissionId(130)));
    assert!(
        !view.sim.refusable_offer_dialog(MissionId(131)),
        "not offered"
    );
}

/// The adapter's own check (rdm:phase/view-sim-seam/phase-11-spaceport-on-port):
/// it hands the flight's chance to a command, the next offer, and to the
/// offer gap. What the view does with the answers runs on `FakeFlight`
/// (`tests/faked_missions.rs`).
#[test]
fn the_bars_offers_go_through_the_session_on_the_flights_chance() {
    let mut errands = catalog();
    errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    errands.missions = vec![MissionRecord {
        avail_loc: 1,
        ..errand(141)
    }];
    let (script, chance) = scripted(&[]);
    let mut view = new_on(errands, chance);
    assert_eq!(view.next_offer(1), None, "in flight");
    land_now(&mut view);
    view.settle_script();
    script.borrow_mut().asked.clear();
    script.borrow_mut().draws.push_back(7);
    view.enter_offer_venue(1);
    let Some(VenueOffer::Prompt(prompt)) = view.next_offer(1) else {
        panic!("an offer");
    };
    assert_eq!(prompt.mission, MissionId(141));
    assert_eq!(view.offer_gap(), nova_sim::mission::screens::mac_ticks(37));
    assert_eq!(script.borrow().asked, [30]);
}

// Persons.

// Persons' mission offers.

#[test]
fn the_help_line_names_mission_info() {
    assert!(HELP.contains("P: player info   I: mission info"), "{HELP}");
}
