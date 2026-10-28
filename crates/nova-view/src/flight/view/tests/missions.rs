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

/// Inspects the player on its first decision; then it has looked the
/// player over, and idles.
#[derive(Debug, Default)]
struct Inspector(std::cell::Cell<bool>);

impl Behaviour for Inspector {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        if self.0.replace(true) {
            Goal::Idle
        } else {
            Goal::Inspect(ShipRef::Player)
        }
    }
}

/// A flight in Sol with 1200 credits, its ship named Kestrel, with
/// mission 130's 4 tons
/// of food (`ScanMask` 0x0004, `Flags` `flags`) aboard in slot 0, and
/// one interceptor (ship 129) of a government scanning for 0x0004 and
/// fining `fine`, placed on the ship, that inspects it, its scan
/// drawing 0.
fn smuggling(flags: u16, fine: i16) -> View {
    let mut catalog = trafficked(&[130], 1, 129, 4);
    catalog.character = Ok(CharacterStart {
        cash: 1200,
        ..catalog.character.expect("a chär")
    });
    catalog.dudes[0].1.govt = Some(GovtId(140));
    catalog.govts = vec![nova_sim::GovtRecord {
        id: GovtId(140),
        name: String::new(),
        flags: 0,
        flags2: 0,
        crime_tol: 0,
        initial_rec: 0,
        penalties: nova_sim::Penalties {
            smuggle: 5,
            ..nova_sim::Penalties::default()
        },
        max_odds: 100,
        classes: [-1; 4],
        allies: [-1; 4],
        enemies: [-1; 4],
        comm_name: String::new(),
        medium_name: String::new(),
        scan_mask: 0x0004,
        scan_fine: fine,
        news_pic: -1,
        require: 0,
        inh_jam: [0; 4],
        skill_mult: 0,
    }];
    catalog.missions = vec![MissionRecord {
        cargo_type: 0,
        cargo_qty: 4,
        pickup_mode: 0,
        scan_mask: 0x0004,
        flags,
        ..errand(130)
    }];
    // Mission 130 active in slot 0, its 4 tons of food aboard.
    let smuggled = nova_sim::mission::ActiveMission {
        mission: MissionId(130),
        destination: None,
        return_to: None,
        cargo_type: Some(0),
        cargo_tons: 4,
        pay: 0,
        days_left: None,
        deadline: None,
        reached: false,
        goals_met: true,
        aboard: true,
        failed: None,
        ships: nova_sim::mission::ships::SpecialShips::default(),
        aux: nova_sim::mission::aux_ships::AuxShips::default(),
    };
    let session = SessionBuilder::flying(&catalog)
        .named("Ada")
        .with_ship_name("Kestrel")
        .with_mission(0, smuggled)
        .build();
    let mut draws = placed(750, 750, 0).to_vec();
    draws.push(0);
    let (_, chance) = scripted(&draws);
    let sim =
        SessionFlight::from(session).with_collaborators(on(chance, Rc::new(Inspector::default())));
    FlightView::from_session(catalog, sim, &Rulebook::default())
}

#[test]
fn a_scan_in_the_warps_lead_in_is_said_under_the_streak() {
    let done = Rc::new(std::cell::Cell::new(false));
    let mut view = now_deciding(
        smuggling(0, 500),
        Rc::new(InspectingUntil(Rc::clone(&done), Goal::Idle)),
    );
    plot(&mut view, 131);
    ready_to_jump(&mut view);
    view.input(&key(JUMP, true));
    assert!(view.jump_effect().is_some(), "streaking on J");
    assert_eq!(view.sim().pilot().expect("flying").cash(), 1200, "not yet");
    done.set(true);
    view.tick(TICK);
    assert_eq!(view.sim().pilot().expect("flying").cash(), 700, "convicted");
    assert_eq!(
        view.message(),
        Some(
            "Ship 129:  Kestrel, your attempt to smuggle illegal cargo through this \
             system has been detected. You have been fined 500 credits."
        ),
        "said under the streak"
    );
    assert!(view.jump_effect().is_some());
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

/// Kept real for phase 13 (rdm:phase/view-sim-seam/phase-13-lifecycle-at-the-edge):
/// `Screen::input`'s press filter routes P to the player info's press,
/// and lets no repeat through. What the press does runs on `FakeFlight`
/// (`tests/faked_player.rs`).
#[test]
fn p_through_the_screen_asks_once_and_a_repeat_does_not() {
    let mut view = flight();
    assert!(!view.take_player_info_request());
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(view.take_player_info_request());
    view.input(&Input::Key {
        key: PLAYER_INFO_KEY,
        pressed: true,
        repeat: true,
    });
    assert!(!view.take_player_info_request(), "not a repeat");
}

#[test]
fn p_during_a_jump_beeps_and_asks_nothing() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    assert!(view.session().expect("flying").preparing_jump().is_some());
    let _ = view.take_sounds();
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(!view.take_player_info_request());
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(SimSound::Refused)],
        "turning to jump"
    );
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    let _ = view.take_sounds();
    view.input(&Input::Key {
        key: PLAYER_INFO_KEY,
        pressed: true,
        repeat: true,
    });
    assert!(view.take_sounds().is_empty(), "not a repeat");
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(!view.take_player_info_request());
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(SimSound::Refused)],
        "streaking"
    );
}

#[test]
fn p_once_the_player_has_exploded_or_ejected_does_nothing_and_says_nothing() {
    let mut holding = exploded();
    assert!(matches!(fate_of(&holding), PlayerFate::Holding { .. }));
    let mut pod = ejecting_view(ejecting_in(100, &[188, 187]), &[]);
    assert!(until_ejected(&mut pod).0);
    for (why, view) in [("holding", &mut holding), ("in the pod", &mut pod)] {
        let _ = view.take_sounds();
        tap(view, PLAYER_INFO_KEY);
        assert!(!view.take_player_info_request(), "{why}");
        assert_eq!(view.take_sounds(), [], "{why}");
    }
}

#[test]
fn p_with_the_map_open_does_nothing() {
    let mut view = flight();
    tap(&mut view, MAP_KEY);
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(!view.take_player_info_request());
    assert!(view.map_open());
    assert!(HELP.contains("P: player info"), "{HELP}");
}

#[test]
fn the_help_line_names_mission_info() {
    assert!(HELP.contains("P: player info   I: mission info"), "{HELP}");
}

#[test]
fn i_with_the_map_open_does_nothing() {
    let mut view = flight();
    tap(&mut view, MAP_KEY);
    assert!(view.map_open());
    tap(&mut view, MISSION_INFO_KEY);
    assert_eq!(view.message(), None);
    assert_eq!(view.take_mission_info_request(), None);
}
