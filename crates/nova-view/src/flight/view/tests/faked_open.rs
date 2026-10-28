//! The view's opening read over a flight (`FlightView::over`), a flight
//! that never started, `Screen`'s keys held and let go and its clock, and
//! entering play's view half, on `FakeFlight`
//! (rdm:phase/view-sim-seam/phase-13-lifecycle-at-the-edge): the tests
//! kept real for this phase, converted under their own names. What the
//! session does as it starts, steps and begins is nova-sim's, named in
//! the commits that moved them.

use super::faked_screen::{in_flight, sent_since, warp_ends};
use super::faked_travel::jump_now;
use super::*;
use nova_sim::SessionEvent;
use nova_sim::flight_frame::FlightEvent;

/// The view `FlightView::over` reads of `fake` over `catalog`, by the
/// default rules.
fn opened(catalog: FakeCatalog, fake: FakeFlight) -> Faked {
    let opening = fake.opening();
    FlightView::over(
        Rc::new(catalog),
        fake,
        opening,
        &Rulebook::default(),
        SharedChance::default(),
    )
}

/// A fake flight showing `frame`.
fn showing(frame: FlightFrame) -> FakeFlight {
    let mut fake = FakeFlight::new();
    fake.set_frame(frame);
    fake
}

/// A fake flight that could not start for `reason`.
fn failed(reason: &str) -> FakeFlight {
    let mut fake = FakeFlight::new();
    fake.fail(reason);
    fake
}

// The opening read.

#[test]
fn the_looks_read_are_the_flights_weapons_not_the_catalogs() {
    // The catalog lists the unseen weapon, which has no look; the flight
    // lists one the catalog doesn't hold.
    let mut fake = showing(sol());
    fake.set_weapons(vec![WeaponId(199)]);
    let mut view = opened(armed(0, &[]), fake);
    let diagnostics = view.take_diagnostics();
    assert!(
        diagnostics.contains(&Diagnostic::Unreadable("no wëap 199".to_owned())),
        "{diagnostics:?}"
    );
    assert!(
        !diagnostics.contains(&Diagnostic::Unreadable("no wëap 150".to_owned())),
        "{diagnostics:?}"
    );
}

#[test]
fn a_flight_built_from_a_session_shows_its_course_from_the_first_frame() {
    let mut fake = showing(FlightFrame {
        course_shown: vec![SystemId(131)],
        ..sol()
    });
    fake.map_mut().galaxy = Some(galaxy_with(|_| {}));
    let view = opened(catalog(), fake);
    assert_eq!(view.course_map().route(), [SystemId(131)]);
    assert_eq!(view.course_map().current(), Some(SystemId(130)));
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    assert_eq!(
        view.sim.applied(),
        [FlightCommand::SetWindowWidth(1024)],
        "the window's width told before the first frame"
    );
}

#[test]
fn a_pilot_saved_at_a_stellar_resumes_in_flight_over_it_silently() {
    // Where a resumed pilot flies, and that nothing lands, is the
    // session's: the view shows the opening frame's place at once.
    let mut view = opened(
        catalog(),
        showing(in_sol_with(vec![the_player_at(
            Vec2::new(0.0, -600.0),
            0.0,
        )])),
    );
    assert_eq!(view.take_landing(), None, "nothing lands");
    assert_eq!(view.shown_position(), Point::new(0.0, -600.0));
    assert_eq!(view.take_sounds(), [], "no landing sound");
    assert!(!view.take_save_due());
}

#[test]
fn a_session_that_cannot_start_says_why_and_draws_nothing_else() {
    let mut view = opened(catalog(), failed("no chär to start from"));
    assert_eq!(view.failure(), Some("no chär to start from"));
    assert!(view.scene().is_none());
    assert_eq!(view.frame(), None);
    view.input(&key(Key::Up, true));
    for _ in 0..3 {
        view.tick(TICK);
    }
    assert_eq!(
        drawn(&view).iter().cloned().collect::<Vec<_>>(),
        [DrawCommand::Text {
            text: "Cannot start flight: no chär to start from".to_owned(),
            font: Font::Geneva,
            origin: TITLE,
            size: TITLE_SIZE,
            wrap_width: None,
            color: Color::ERROR,
        },]
    );
}

#[test]
fn the_unstarted_flight_shows_help_only_when_turned_on() {
    let view = opened(catalog(), failed("no chär to start from")).with_overlays(FlightOverlays {
        title: false,
        help: true,
        cheats: false,
    });
    let list = drawn(&view);
    let help = overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM);
    assert_eq!(list.iter().last(), Some(&help));
    assert_eq!(list.iter().count(), 2, "the error line and the help");
}

#[test]
fn a_session_that_cannot_start_has_no_status_bar() {
    let view = opened(catalog(), failed("no chär to start from"));
    assert_eq!(view.status_bar(), Err("no chär to start from"));
}

#[test]
fn a_new_pilot_shows_the_default_status_bar() {
    let view = opened(catalog(), showing(sol()));
    assert_eq!(
        view.status_bar(),
        Ok(&StatusBar {
            layout: layout(),
            background: Some(hud::Background {
                id: 700,
                width: 194.0,
                height: 767.0,
            }),
        })
    );
}

#[test]
fn a_ship_type_whose_government_has_its_own_interface_shows_that_bar() {
    let mut frame = sol();
    frame.hud.interface_government = Some(nova_sim::GovtId(150));
    let view = opened(governed(catalog(), 128, 129), showing(frame));
    assert_eq!(
        view.status_bar(),
        Ok(&StatusBar {
            layout: bar_129(),
            background: Some(hud::Background {
                id: 705,
                width: 194.0,
                height: 767.0,
            }),
        })
    );
    let list = drawn(&view);
    assert!(
        list.iter().any(|c| *c
            == DrawCommand::Picture {
                image: ImageKey::picture(705),
                top_left: at(830.0, 0.0),
            }),
        "{list:?}"
    );
    for interface in [127, 0, -1] {
        let mut frame = sol();
        frame.hud.interface_government = Some(nova_sim::GovtId(150));
        let view = opened(governed(catalog(), 128, interface), showing(frame));
        assert_eq!(
            view.status_bar().map(|bar| &bar.layout),
            Ok(&layout()),
            "{interface}"
        );
    }
}

#[test]
fn a_new_system_shows_on_the_radar_at_once_leaving_the_timer_and_blink() {
    let mut view = super::faked_world::world(catalog(), in_system(130, 128, true));
    for _ in 0..3 {
        view.tick(TICK);
    }
    let cadence = view.hud_cadence;
    view.sim.set_frame(FlightFrame {
        stellars: vec![record::stellar(StellarId(140))],
        ..in_system(131, 128, true)
    });
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
    view.resync();
    assert_eq!(view.hud_cadence, cadence, "neither timed nor flipped");
    let shown = view.radar_shown(&view.frame_now());
    assert_eq!(shown.player, view.shown_position());
    let stellars: Vec<Point> = shown.stellars.iter().map(|s| s.position).collect();
    let scene: Vec<Point> = view
        .scene()
        .expect("a scene")
        .stellars()
        .iter()
        .map(|s| s.position)
        .collect();
    assert_eq!(stellars, scene, "Alpha Centauri's");
}

// The keys held, and the clock.

#[test]
fn repeats_hold_a_key_and_releasing_the_keys_stops_the_thrust() {
    let mut view = in_flight();
    view.input(&held(Key::Up));
    view.tick(TICK);
    assert_eq!(
        super::faked_flight::last_controls(&view).thrust,
        THRUST.thrust,
        "a repeat holds the key"
    );
    view.release_keys();
    view.tick(TICK);
    assert_eq!(
        super::faked_flight::last_controls(&view),
        Controls::default(),
        "coasting"
    );
}

#[test]
fn other_input_changes_nothing_and_never_quits() {
    let mut view = in_flight();
    let others = [
        key(Key::Escape, true),
        key(Key::Space, true),
        key(Key::Tab, true),
        key(Key::Space, true),
        key(Key::Enter, true),
        key(Key::Char('w'), true),
        key(Key::Char('w'), true),
        key(Key::Other, true),
        Input::PointerMoved(at(1.0, 2.0)),
        Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed: true,
            at: at(1.0, 2.0),
        },
    ];
    for input in others {
        assert_eq!(view.input(&input), ScreenAction::None, "{input:?}");
    }
    for input in [key(Key::Space, false), key(Key::Escape, false)] {
        view.input(&input);
    }
    view.tick(TICK);
    assert_eq!(
        super::faked_flight::last_controls(&view),
        Controls::default(),
        "no steering or thrust"
    );
}

/// The controls each fixed step of a three-second script was given at
/// `fps`, each redraw at `n / fps` seconds to the nanosecond: thrust for
/// a second, turn right for half a second, reverse for a second, then
/// coast.
fn controls_flown_at(fps: u64) -> Vec<Controls> {
    let mut view = in_flight();
    let script = [
        (0, Key::Up, true),
        (2, Key::Up, false),
        (2, Key::Right, true),
        (3, Key::Right, false),
        (3, Key::Down, true),
        (5, Key::Down, false),
    ];
    let mut last = 0;
    for frame in 1..=3 * fps {
        let half_second = (frame - 1) * 2 / fps;
        if (frame - 1) * 2 % fps == 0 {
            for &(_, k, pressed) in script.iter().filter(|(at, ..)| *at == half_second) {
                view.input(&key(k, pressed));
            }
        }
        let now = frame * 1_000_000_000 / fps;
        view.tick(Duration::from_nanos(now - last));
        last = now;
    }
    let mut per_step = Vec::new();
    for &(input, steps) in view.sim.stepped_calls() {
        per_step.extend(std::iter::repeat_n(
            input.controls,
            usize::try_from(steps).expect("a frame's steps"),
        ));
    }
    per_step
}

#[test]
fn the_same_keys_fly_the_same_at_30_60_and_120_fps() {
    let at_30 = controls_flown_at(30);
    assert_eq!(at_30.len(), 90, "three seconds of fixed steps");
    assert!(at_30.iter().any(|controls| controls.thrust), "thrust");
    assert_eq!(controls_flown_at(60), at_30);
    assert_eq!(controls_flown_at(120), at_30);
}

// Entering play's view half.

#[test]
fn beginning_places_the_ship_where_the_flight_began_it_and_scatters_the_stars() {
    let mut view = in_flight();
    view.stars_due = false;
    view.alpha = 0.5;
    view.sim.frame_mut().ships[0] = record::player(128, Vec2::new(50.0, 50.0), 0.0);
    let mut asked = 0;
    view.begin(|_| {
        asked += 1;
        true
    });
    assert_eq!(asked, 1, "the flight's half, once");
    assert_eq!(view.shown_position(), Point::new(50.0, 50.0), "drawn there");
    assert_eq!(view.alpha, 0.0);
    assert!(view.stars_due, "the stars scattered afresh at the catch-up");
    let mut unbegun = in_flight();
    unbegun.stars_due = false;
    unbegun.begin(|_| false);
    assert!(!unbegun.stars_due, "a flight that begins nothing");
}

#[test]
fn entering_play_scatters_the_stars_once_the_flight_has_entered() {
    let mut view = in_flight();
    view.stars_due = false;
    view.enter_game(|_| true);
    assert!(view.stars_due);
    let mut failed = in_flight();
    failed.stars_due = false;
    failed.enter_game(|_| false);
    assert!(!failed.stars_due, "a flight that enters nothing");
}

#[test]
fn an_arrival_after_beginning_is_laid_out_as_any_other() {
    let mut view = in_flight();
    view.begin(|_| true);
    jump_now(&mut view);
    warp_ends(&mut view);
    view.tick(TICK);
    assert!(sent_since(&view, 0).contains(&FlightCommand::Arrive));
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
}

// The edge's follow-ups once it has set the session up.

#[test]
fn showing_the_set_up_flight_redraws_its_stellars_and_reads_the_sheets_its_slots_ask_for() {
    for slots_carried in [false, true] {
        let mut view = super::faked_world::world(catalog(), sol());
        let mut after = sol();
        after.stellars[1].destroyed = true;
        after
            .ships
            .push(record::npc(7, 131, Vec2::new(100.0, 0.0), 0.0));
        // The edge sets the flight up through `change_sim`, which fills
        // the kept frame afresh before the view shows the set-up.
        view.change_sim(|sim| sim.set_frame(after));
        view.show_set_up(slots_carried);
        assert_eq!(view.frame, view.frame_now(), "the kept frame the flight's");
        assert!(
            view.scene().expect("a scene").stellars()[1].destroyed,
            "the stellars drawn again"
        );
        assert_eq!(
            view.npc_sheets.contains_key(&ShipId(131)),
            slots_carried,
            "the sheets read only for slots carried"
        );
    }
}
