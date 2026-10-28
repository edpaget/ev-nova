//! The builders' hand-offs over a real session, kept real for phase 13
//! (rdm:phase/view-sim-seam/phase-13-lifecycle-at-the-edge): beginning,
//! resuming and opening a flight, each on the flight's chance. The
//! docked bar's tests run on `FakeFlight` (`faked_docked.rs`,
//! `faked_shops.rs`, `faked_missions.rs`).

use super::*;

#[test]
fn beginning_runs_on_start_once_and_explores_what_it_sets() {
    let catalog = starting_with("R(b5 b6) ^b7 X131");
    let mut view = flying_as_on(catalog, "Ada", rolling_one());
    assert!(!bit_set(&view, 7), "flying runs nothing");
    view.begin(SessionFlight::begin);
    assert!(bit_set(&view, 5), "on the flight's chance");
    assert!(bit_set(&view, 7), "once");
    let explored: Vec<SystemId> = view.session().expect("flying").pilot().explored().collect();
    assert_eq!(explored, [SystemId(130), SystemId(131)]);
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..starting_with("b5")
    };
    let mut broken = test_flight::new(broken);
    broken.begin(SessionFlight::begin);
    assert_eq!(broken.sim().pilot(), None);
}

#[test]
fn resuming_faces_the_heading_drawn_on_the_flights_chance() {
    let (script, chance) = scripted(&[123]);
    let mut view = over_earth_on(catalog(), chance);
    assert_eq!(player(&view).heading, 0.0, "flying alone faces up");
    view.sim_mut().resume();
    assert_eq!(player(&view).heading, 123.0);
    let asked = &script.borrow().asked;
    assert_eq!(asked[0], 360, "the heading first");
    assert_eq!(
        asked[1..],
        [100; nova_sim::mission::ROLLED],
        "then the offers' roll table, on the same chance"
    );
}

#[test]
fn beginning_draws_the_offer_roll_table_on_the_flights_chance() {
    let (script, chance) = scripted(&[]);
    let catalog = catalog();
    let mut view = flying_as_on(catalog, "Ada", chance);
    view.begin(SessionFlight::begin);
    let asked = &script.borrow().asked;
    assert!(asked.len() >= nova_sim::mission::ROLLED);
    assert_eq!(
        asked[asked.len() - nova_sim::mission::ROLLED..],
        [100; nova_sim::mission::ROLLED]
    );
}

#[test]
fn beginning_a_new_pilot_leaves_it_facing_up() {
    let (_, chance) = scripted(&[123]);
    let catalog = catalog();
    let mut view = flying_as_on(catalog, "Ada", chance);
    view.begin(SessionFlight::begin);
    assert_eq!(player(&view).heading, 0.0);
}

#[test]
fn a_failed_flight_resumes_nothing() {
    let (script, chance) = scripted(&[123]);
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    };
    let mut broken = new_on(broken, chance);
    broken.sim_mut().resume();
    assert_eq!(broken.sim().pilot(), None);
    assert!(script.borrow().asked.is_empty(), "nothing drawn");
}

/// A catalog of mission 130 with two auxiliary ships, and the mission
/// as its accept leaves it.
fn aux_errand() -> (impl Fn() -> FakeCatalog, nova_sim::mission::ActiveMission) {
    let errands = || {
        let mut errands = catalog();
        errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        errands.missions = vec![MissionRecord {
            aux: nova_sim::catalog::AuxShip {
                count: 2,
                dude: Some(nova_sim::catalog::DudeId(200)),
                syst: -1,
            },
            ..errand(130)
        }];
        errands
    };
    let accepted = {
        let mut view = test_flight::new(errands());
        land_now(&mut view);
        view.settle_script();
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        let pilot = view.sim().pilot().expect("flying");
        pilot.missions().get(0).expect("held").clone()
    };
    (errands, accepted)
}

/// A flight over Earth holding [`aux_errand`]'s mission in slot 0, on
/// `chance`.
fn aux_flight(chance: SharedChance) -> View {
    let (errands, accepted) = aux_errand();
    let catalog = errands();
    let session = SessionBuilder::flying(&catalog)
        .over(StellarId(128))
        .with_mission(0, accepted)
        .build();
    FlightView::from_session(catalog, on_chance(session, chance), &Rulebook::default())
}

/// Mission 130's auxiliary ships in `view`.
fn aux_of(view: &View) -> nova_sim::mission::aux_ships::AuxShips {
    view.sim()
        .pilot()
        .expect("flying")
        .missions()
        .get(0)
        .expect("held")
        .aux
}

/// A flight that failed to start, on `chance`.
fn broken_flight(chance: SharedChance) -> View {
    let broken = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    };
    new_on(broken, chance)
}

#[test]
fn opening_on_the_main_screen_restarts_the_aux_countdown_on_the_flights_chance() {
    let (script, chance) = scripted(&[9]);
    let mut view = aux_flight(chance);
    assert_eq!(aux_of(&view).countdown, 139, "the accept's draw");
    view.sim_mut().open_on_main_screen();
    assert_eq!(
        aux_of(&view),
        nova_sim::mission::aux_ships::AuxShips {
            countdown: 79,
            arrived: 0,
            left: 2
        }
    );
    assert_eq!(script.borrow().asked, [70]);
    let (script, chance) = scripted(&[]);
    broken_flight(chance).sim_mut().open_on_main_screen();
    assert!(
        script.borrow().asked.is_empty(),
        "a failed flight draws nothing"
    );
}

#[test]
fn opening_at_launch_restarts_the_aux_countdown_on_the_flights_chance() {
    let (script, chance) = scripted(&[4]);
    let mut view = aux_flight(chance);
    view.sim_mut().open_at_launch();
    assert_eq!(
        aux_of(&view),
        nova_sim::mission::aux_ships::AuxShips {
            countdown: 74,
            arrived: 0,
            left: 2
        }
    );
    assert_eq!(script.borrow().asked, [70]);
    let (script, chance) = scripted(&[]);
    broken_flight(chance).sim_mut().open_at_launch();
    assert!(
        script.borrow().asked.is_empty(),
        "a failed flight draws nothing"
    );
}
