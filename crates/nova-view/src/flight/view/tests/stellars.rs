use super::*;

// Session events.

#[test]
fn the_hull_redraws_from_the_event_on_the_next_frame() {
    let mut view = FlightView::new(shipbuying());
    run_set(&mut view, "H129");
    settle_session(&mut view);
    assert_eq!(sheets(&view), [ShipId(128)], "not before the frame");
    view.tick(TICK);
    assert_eq!(
        sheets(&view),
        [ShipId(128), ShipId(129)],
        "a set expression"
    );
    view.tick(TICK);
    assert_eq!(sheets(&view), [ShipId(128), ShipId(129)], "once");

    let mut view = captured();
    let asked = sheets(&view).len();
    let session = view.session.as_mut().expect("flying");
    assert_eq!(
        session.assign(Assignment::MyShip, &mut NeverFires),
        Some(Assigned::MyShip)
    );
    assert_eq!(view.sheet_ship, Some(ShipId(128)), "not before the frame");
    view.tick(TICK);
    assert_eq!(view.sheet_ship, Some(ShipId(129)), "a capture");
    let player = sheets(&view)[asked..]
        .iter()
        .filter(|&&ship| ship == ShipId(129))
        .count();
    assert_eq!(player, 1, "read once, beside the old ship's as an escort");

    let mut view = FlightView::new(shipbuying());
    land_now(&mut view);
    let session = view.session.as_mut().expect("flying");
    session
        .buy_ship(ShipId(129), "Kestrel", &mut NeverFires)
        .expect("bought");
    view.tick(TICK);
    assert_eq!(sheets(&view), [ShipId(128), ShipId(129)], "a purchase");
}

fn bars_read(view: &View) -> Vec<i16> {
    view.catalog().bars_asked.borrow().clone()
}

#[test]
fn the_status_bar_follows_the_ship_type_on_the_next_frame() {
    let mut view = FlightView::new(governed(shipbuying(), 129, 129));
    run_set(&mut view, "H129");
    settle_session(&mut view);
    assert_eq!(bar_shown(&view), Some(700), "not before the frame");
    view.tick(TICK);
    assert_eq!(bar_shown(&view), Some(705), "a set expression");
    view.tick(TICK);
    assert_eq!(bars_read(&view), [128, 129], "once");

    let mut view = captured_in(governed(boardable(), 129, 129));
    let session = view.session.as_mut().expect("flying");
    assert_eq!(
        session.assign(Assignment::MyShip, &mut NeverFires),
        Some(Assigned::MyShip)
    );
    assert_eq!(bar_shown(&view), Some(700), "not before the frame");
    view.tick(TICK);
    assert_eq!(bar_shown(&view), Some(705), "a capture");

    let mut view = FlightView::new(governed(shipbuying(), 129, 129));
    land_now(&mut view);
    let session = view.session.as_mut().expect("flying");
    session
        .buy_ship(ShipId(129), "Kestrel", &mut NeverFires)
        .expect("bought");
    view.tick(TICK);
    assert_eq!(bar_shown(&view), Some(705), "a purchase");
}

#[test]
fn a_new_ship_types_missing_bar_keeps_the_layout_over_picture_128() {
    let mut view = FlightView::new(governed(shipbuying(), 129, 131));
    run_set(&mut view, "H129");
    view.tick(TICK);
    assert_eq!(bars_read(&view), [128, 131]);
    assert_eq!(
        view.status_bar(),
        Ok(&StatusBar {
            layout: layout(),
            background: Some(hud::Background {
                id: 128,
                width: 194.0,
                height: 767.0,
            }),
        })
    );
}

#[test]
fn the_system_is_laid_out_from_the_event() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.catalog().systems_read.borrow_mut().clear();
    jump_now(&mut view);
    arrive_now(&mut view);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(systems_read(&view), [SystemId(131)], "a jump");

    let mut view = FlightView::new(holed());
    view.catalog().systems_read.borrow_mut().clear();
    land_now(&mut view);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(systems_read(&view), [SystemId(131)], "a wormhole");

    let mut view = flight();
    view.catalog().systems_read.borrow_mut().clear();
    run_set(&mut view, "M131");
    settle_session(&mut view);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    view.tick(TICK);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(systems_read(&view), [SystemId(131)], "a set expression");

    let mut view = landed_view();
    move_to(&mut view, 131, 140);
    view.tick(TICK);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(systems_read(&view), [SystemId(131)], "a relocation");
}

#[test]
fn several_moves_before_a_frame_lay_out_once_the_last_system() {
    let mut view = FlightView::new(FakeCatalog {
        onward: true,
        ..catalog()
    });
    view.catalog().systems_read.borrow_mut().clear();
    run_set(&mut view, "M131 M132");
    settle_session(&mut view);
    view.tick(TICK);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(132)));
    assert_eq!(systems_read(&view), [SystemId(132)], "one layout");
}

/// A stellar moved to, then a system entered, before one frame: one
/// layout, of the system entered. (Placing the ship as well after the
/// layout would change nothing to see: its writes are the layout's.)
#[test]
fn a_system_entered_supersedes_a_stellar_moved_to_before_the_frame() {
    let mut view = landed_view();
    move_to(&mut view, 130, 129);
    let session = view.session.as_mut().expect("flying");
    assert_eq!(
        session.take_off(&view.catalog, &mut NeverFires),
        Some(StellarId(129))
    );
    run_set(&mut view, "M131");
    settle_session(&mut view);
    view.tick(TICK);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(systems_read(&view), [SystemId(131)], "one layout");
    assert_eq!(view.course_map().current(), Some(SystemId(131)));
    assert_eq!(view.shown_position(), Point::new(0.0, 0.0), "at Proxima");
}

// The stellars shown.

/// The IDs of the stellars `view`'s scene draws.
fn scene_ids(view: &View) -> Vec<StellarId> {
    view.scene()
        .expect("laid out")
        .stellars()
        .iter()
        .map(|stellar| stellar.id)
        .collect()
}

/// The stellars `view`'s session shows.
fn shown_ids(view: &View) -> Vec<StellarId> {
    view.session().expect("flying").stellars_shown().collect()
}

/// [`catalog`] with Sol listing Proxima (140) too: Sol, the lower and
/// visible, owns it, though Alpha Centauri lists it.
fn proxima_in_sol() -> FakeCatalog {
    let mut catalog = catalog();
    catalog
        .sites
        .push(site(140, (0.0, 0.0), StellarFlags::CAN_LAND));
    catalog
}

/// Sol owning a wormhole, Hole (150), 300 below the centre the ship
/// starts at, and a hypergate, Gate (151), 300 above it, each a square
/// on the radar as the data adapter reads them, after Earth and the
/// Moon.
fn sol_with_gates() -> View {
    let mut catalog = catalog();
    catalog.sites.extend([
        LandingSite {
            flags2: 0x2000,
            ..site(
                150,
                (0.0, 300.0),
                StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            )
        },
        LandingSite {
            flags2: 0x1000,
            ..site(151, (0.0, -300.0), StellarFlags::CAN_LAND)
        },
    ]);
    catalog.sol_more = vec![
        StellarContents {
            radar_square: true,
            ..stellar(150, "Hole", (0, 300), 1)
        },
        StellarContents {
            radar_square: true,
            ..stellar(151, "Gate", (0, -300), 1)
        },
    ];
    FlightView::new(catalog)
}

/// `_HandleStellarSprites` draws every active stellar the system owns,
/// gates too (@0x2e713-0x2e74f): no gate test.
#[test]
fn the_flight_scene_draws_the_systems_wormhole_and_hypergate() {
    let view = sol_with_gates();
    let ids = scene_ids(&view);
    assert!(ids.contains(&StellarId(150)), "{ids:?}");
    assert!(ids.contains(&StellarId(151)), "{ids:?}");
}

/// `_DrawStatusRadar` draws every nav slot, gates as squares
/// (@0x498a0-0x49a8e): no gate test.
#[test]
fn the_radar_shows_the_systems_wormhole_and_hypergate_as_squares() {
    let mut view = sol_with_gates();
    ticks(&mut view, 1);
    let radar = view.radar_shown();
    let gates: Vec<(Point, bool)> = radar
        .stellars
        .iter()
        .filter(|stellar| stellar.position.x == 0.0 && stellar.position.y.abs() == 300.0)
        .map(|stellar| (stellar.position, stellar.square))
        .collect();
    assert_eq!(
        gates,
        [
            (Point::new(0.0, 300.0), true),
            (Point::new(0.0, -300.0), true)
        ]
    );
}

/// `_DrawStatusNav` names any selected stellar (@0x4a09c-0x4a18c): F5
/// within 500 pixels selects the wormhole, and the nav area names it.
#[test]
fn f5_near_the_wormhole_selects_it_and_the_nav_area_names_it() {
    let mut view = sol_with_gates();
    tap_slot(&mut view, NEAREST_STELLAR_KEY);
    assert_eq!(nav_target(&view), Some(StellarId(150)));
    assert_eq!(nav(&view), [hud::NAV_STELLAR, "Hole"]);
}

#[test]
fn the_flight_scene_shows_only_the_sessions_stellars() {
    let view = FlightView::new(FakeCatalog {
        character: Ok(CharacterStart {
            ship: Some(ShipId(128)),
            systems: [Some(SystemId(131)), None, None, None],
            ..CharacterStart::default()
        }),
        ..proxima_in_sol()
    });
    assert_eq!(system_of(&view), SystemId(131));
    assert_eq!(shown_ids(&view), [], "Proxima is Sol's");
    assert_eq!(scene_ids(&view), [], "Alpha Centauri lists it, unshown");
}

#[test]
fn a_rebuild_that_changes_the_stellars_shown_lays_out_the_scene_again() {
    let mut view = FlightView::with_pilot(proxima_in_sol(), docked_pilot("Ada"));
    land_now(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    move_to(&mut view, 131, 140);
    view.tick(TICK);
    assert_eq!(shown_ids(&view), [StellarId(140)], "docked at it");
    assert_eq!(scene_ids(&view), [StellarId(140)]);
    assert_eq!(view.take_off(), Some(StellarId(140)));
    assert_eq!(shown_ids(&view), [], "in flight, Sol's again");
    assert_eq!(scene_ids(&view), shown_ids(&view));
}

// The galaxy overlay.

/// Whether `view`'s course map draws system `id`.
fn on_the_course_map(view: &View, id: i16) -> bool {
    view.course_map().model().system(SystemId(id)).is_some()
}

#[test]
fn a_pilot_flown_with_a_galaxy_overlay_shows_it_on_the_course_map() {
    // 132 hidden by its `Visibility`, as the load evaluates it.
    let catalog = FakeCatalog {
        visibility: vec![nova_sim::catalog::SystemVisibility {
            id: SystemId(132),
            visibility: nova_sim::Test::parse("b50"),
        }],
        ..catalog()
    };
    let session = Session::start(&catalog).expect("starts");
    assert!(!session.galaxy().visible(SystemId(132)));
    let view = FlightView::with_pilot(catalog, session.pilot().clone());
    assert!(!on_the_course_map(&view, 132));
    assert!(on_the_course_map(&view, 131));
}

#[test]
fn a_change_to_the_galaxy_is_drawn_on_the_next_frame_and_not_before() {
    let mut view = flight();
    change_galaxy(&mut view, hiding(132));
    assert!(on_the_course_map(&view, 132), "not before the frame");
    view.tick(TICK);
    assert!(!on_the_course_map(&view, 132));
}

/// The rules that keep a loaded pilot's course, so a flight can start
/// on one.
fn course_kept() -> Rulebook {
    Rulebook::default().with_override(RuleKey::LoadedCourse, RuleSource::Bible)
}

#[test]
fn the_course_map_shows_the_course_as_it_is_flown() {
    let catalog = catalog();
    let mut session = Session::start(&catalog).expect("starts");
    session.plot_course(SystemId(131)).expect("a route");
    let mut view = FlightView::loaded(catalog, session.pilot().clone(), course_kept());
    assert_eq!(view.course_map().route(), [SystemId(131)]);
    change_galaxy(&mut view, hiding(131));
    view.tick(TICK);
    assert_eq!(view.course_map().route(), [], "no system at its place");
    let session = view.session.as_ref().expect("flying");
    assert_eq!(session.course(), [SystemId(131)], "kept");
}

/// A flight of `reborn` Sol, bound for Alpha Centauri, Alpha Reborn
/// explored.
fn bound_for_alpha_with_reborn_explored() -> View {
    let catalog = FakeCatalog {
        reborn: true,
        ..catalog()
    };
    let mut session = Session::start(&catalog).expect("starts");
    session.plot_course(SystemId(131)).expect("a route");
    let explore = nova_sim::SetExpr::parse("X133").expect("parses");
    session.run_set(&explore, &mut NeverFires);
    let mut view = FlightView::loaded(catalog, session.pilot().clone(), course_kept());
    // A pilot flies with the nav system off: H cues the course.
    tap(&mut view, HYPERSPACE_MODE_KEY);
    view
}

#[test]
fn the_nav_area_shows_the_system_standing_for_a_hidden_next_hop() {
    let mut view = bound_for_alpha_with_reborn_explored();
    change_galaxy(&mut view, hiding(131));
    view.tick(TICK);
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Alpha Reborn"]);
}

#[test]
fn the_nav_area_shows_no_destination_for_a_hop_with_no_system_visible_at_its_place() {
    let mut view = bound_for_alpha_with_reborn_explored();
    change_galaxy(&mut view, |overlay| {
        overlay.set_visible(SystemId(131), Some(false));
        overlay.set_visible(SystemId(133), Some(false));
    });
    view.tick(TICK);
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_NO_DESTINATION]);
    let session = view.session().expect("flying");
    assert_eq!(session.course(), [SystemId(131)], "kept");
}

#[test]
fn several_galaxy_changes_before_a_frame_rebuild_the_map_once() {
    let mut view = flight();
    change_galaxy(&mut view, hiding(132));
    change_galaxy(&mut view, |overlay| {
        overlay.set_visible(SystemId(132), None);
        overlay.set_visible(SystemId(131), Some(false));
    });
    let redrawn = view.catch_up();
    assert!(redrawn.galaxy);
    assert!(on_the_course_map(&view, 132), "the last galaxy");
    assert!(!on_the_course_map(&view, 131));
    assert!(!view.catch_up().galaxy, "once");
}

// Destroyed stellars (story-runtime phase 7).

const DEAD_TYPE: (bool, bool) = (false, true);

#[test]
fn a_stellar_destroyed_with_y_is_drawn_with_its_dead_type_from_the_next_frame_and_regenerated_with_u()
 {
    let mut view = destructible();
    view.tick(TICK);
    assert_eq!(sheets_drawn(&view, 128), TYPE);
    view.catalog().systems_read.borrow_mut().clear();

    run_set(&mut view, "Y128");
    settle_session(&mut view);
    assert_eq!(sheets_drawn(&view, 128), TYPE, "not before the frame");
    view.tick(TICK);
    assert_eq!(sheets_drawn(&view, 128), DEAD_TYPE, "from the next frame");
    assert_eq!(systems_read(&view), [], "not laid out again");

    run_set(&mut view, "U128");
    settle_session(&mut view);
    assert_eq!(sheets_drawn(&view, 128), DEAD_TYPE, "not before the frame");
    view.tick(TICK);
    assert_eq!(sheets_drawn(&view, 128), TYPE, "regenerated");
    assert_eq!(systems_read(&view), []);
}

#[test]
fn y_and_u_before_one_frame_draw_type() {
    let mut view = destructible();
    view.tick(TICK);
    run_set(&mut view, "Y128");
    run_set(&mut view, "U128");
    view.tick(TICK);
    assert_eq!(sheets_drawn(&view, 128), TYPE);
    let scene = view.scene().expect("a scene");
    assert!(scene.stellars().iter().all(|stellar| !stellar.destroyed));
}

#[test]
fn y_on_a_stellar_of_another_system_changes_nothing_drawn() {
    let mut view = destructible();
    view.tick(TICK);
    view.catalog().systems_read.borrow_mut().clear();
    run_set(&mut view, "Y140");
    view.tick(TICK);
    assert_eq!(sheets_drawn(&view, 128), TYPE);
    let scene = view.scene().expect("a scene");
    assert!(scene.stellars().iter().all(|stellar| !stellar.destroyed));
    assert_eq!(systems_read(&view), []);
}

#[test]
fn a_destroyed_stellar_with_no_dead_type_keeps_its_type() {
    let mut view = destructible();
    run_set(&mut view, "Y129");
    view.tick(TICK);
    assert_eq!(sheets_drawn(&view, 129), TYPE);
    let scene = view.scene().expect("a scene");
    assert!(scene.stellars()[1].destroyed, "destroyed all the same");
}

#[test]
fn a_system_entered_draws_its_destroyed_stellars_dead() {
    let mut view = destructible();
    run_set(&mut view, "Y140");
    run_set(&mut view, "M131");
    settle_session(&mut view);
    view.tick(TICK);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(sheets_drawn(&view, 140), DEAD_TYPE);
}

#[test]
fn a_system_entered_after_its_stellar_was_destroyed_draws_it_dead() {
    let mut view = destructible();
    run_set(&mut view, "Y140");
    view.tick(TICK);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    run_set(&mut view, "M131");
    settle_session(&mut view);
    let redrawn = view.catch_up();
    assert!(redrawn.system, "laid out");
    assert!(!redrawn.galaxy, "by the system's reaction alone");
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(sheets_drawn(&view, 140), DEAD_TYPE);
}

#[test]
fn a_new_pilot_with_a_stellar_that_starts_destroyed_draws_it_dead_from_the_first_frame() {
    let mut view = FlightView::new(FakeCatalog {
        lives: vec![life(128, 0x0040)],
        ..catalog()
    });
    assert_eq!(sheets_drawn(&view, 128), TYPE, "not begun yet");
    view.begin();
    view.tick(TICK);
    assert_eq!(sheets_drawn(&view, 128), DEAD_TYPE);
}

#[test]
fn a_pilot_flown_with_a_stellar_destroyed_draws_it_dead_at_once() {
    let catalog = FakeCatalog {
        lives: vec![life(128, 0)],
        ..catalog()
    };
    let mut session = Session::start(&catalog).expect("starts");
    let destroy = nova_sim::SetExpr::parse("Y128").expect("parses");
    session.run_set(&destroy, &mut NeverFires);
    let view = FlightView::with_pilot(catalog, session.pilot().clone());
    assert_eq!(sheets_drawn(&view, 128), DEAD_TYPE, "before any frame");
}

// Stellar animation.

/// At 60 fps each frame is half a tick: the halves add up, so the Moon
/// (1129) steps once the second one completes the tick.
#[test]
fn two_half_tick_frames_step_the_stellars_once() {
    let mut view = flight();
    view.tick(TICK / 2);
    assert_eq!(
        sprites(&drawn(&view))[1].0,
        ImageKey::sprite(1129, 0),
        "not on half a tick"
    );
    view.tick(REST_OF_TICK);
    assert_eq!(
        sprites(&drawn(&view))[1].0,
        ImageKey::sprite(1129, 1),
        "on the tick's second half"
    );
}

// A `DeadType` with no sheet after a rebuild point (story-runtime
// phase 9).

/// The Moon after `Y129` and a frame; then after an in-flight `M130`
/// and a frame; then `U129` and a frame; then `Y129` and a frame.
fn moon_through_a_rebuild(mut view: View) -> [Moon; 4] {
    run_set(&mut view, "Y129");
    view.tick(TICK);
    let before = moon(&view);
    run_set(&mut view, "M130");
    settle_session(&mut view);
    view.tick(TICK);
    let rebuilt = moon(&view);
    run_set(&mut view, "U129");
    view.tick(TICK);
    let regenerated = moon(&view);
    run_set(&mut view, "Y129");
    view.tick(TICK);
    [before, rebuilt, regenerated, moon(&view)]
}

#[test]
fn a_stellar_with_no_dead_type_draws_type_zero_from_the_next_rebuild_point_until_seen_whole() {
    assert_eq!(
        moon_through_a_rebuild(destructible()),
        [Moon::Type, Moon::TypeZero, Moon::Type, Moon::Type],
        "by default, the engine's"
    );
    assert_eq!(
        moon_through_a_rebuild(reading(RuleSource::Engine)),
        [Moon::Type, Moon::TypeZero, Moon::Type, Moon::Type]
    );
}

#[test]
fn by_the_bible_a_stellar_with_no_dead_type_keeps_its_type_throughout() {
    assert_eq!(
        moon_through_a_rebuild(reading(RuleSource::Bible)),
        [Moon::Type; 4]
    );
}

#[test]
fn a_take_off_that_only_rebuilds_draws_type_zero() {
    let mut view = FlightView::new(FakeCatalog {
        sites: vec![
            site(128, (6.0, -8.0), StellarFlags::CAN_LAND),
            site(129, (300.0, -200.0), StellarFlags::CAN_LAND),
        ],
        lives: vec![life(129, 0)],
        ..catalog()
    });
    land_now(&mut view);
    assert_eq!(
        view.session().expect("flying").landed(),
        Some(StellarId(128))
    );
    run_set(&mut view, "Y129");
    view.tick(TICK);
    assert_eq!(
        moon(&view),
        Moon::Type,
        "destroyed after the landing's rebuild"
    );
    view.catalog().systems_read.borrow_mut().clear();
    view.take_off();
    assert_eq!(moon(&view), Moon::TypeZero, "the take-off's rebuild");
    assert_eq!(systems_read(&view), [], "not laid out again");
}

#[test]
fn a_pilot_loaded_with_a_stellar_destroyed_draws_it_as_its_rules_say_from_the_first_frame() {
    let view = loaded(Rulebook::default());
    assert!(
        view.session()
            .expect("flying")
            .dead_since_rebuild(StellarId(129))
    );
    assert_eq!(moon(&view), Moon::TypeZero, "a load is a rebuild point");
    let bible = Rulebook::default().with_override(RuleKey::DeadTypeFallback, RuleSource::Bible);
    let view = loaded(bible);
    assert_eq!(moon(&view), Moon::Type, "before any event");
    let view = loaded(bible).with_rules(Rulebook::default());
    assert_eq!(moon(&view), Moon::TypeZero, "and back");
}

#[test]
fn a_system_entered_by_a_jump_with_a_stellar_destroyed_draws_type_zero_from_its_first_frame() {
    let mut view = destructible();
    run_set(&mut view, "Y129");
    jump_to_alpha(&mut view);
    plot(&mut view, 130);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    arrive_now(&mut view);
    assert_eq!(view.session().expect("flying").system(), SystemId(130));
    assert_eq!(moon(&view), Moon::TypeZero);
}

// A whole stellar whose `Type` sheet cannot be read (stellar-fidelity
// phase 3).

/// Why the Moon's `Type` sheet cannot be read.
const UNREADABLE: &str = "spöb 129: no spïn 1004 for its graphic type 4";

/// A flight in Sol whose Moon (129) is whole and its `Type` sheet cannot
/// be read, by `rules`, after its first frame.
fn unreadable_moon(rules: Rulebook) -> View {
    let mut view = FlightView::new(FakeCatalog {
        moon_unreadable: Some(UNREADABLE),
        ..catalog()
    })
    .with_rules(rules);
    view.tick(TICK);
    view
}

/// Whether the placeholder's reason for the Moon's `Type` sheet is drawn.
fn moon_placeholder_drawn(view: &View) -> bool {
    texts(&drawn(view)).contains(&format!("Sprite unavailable: {UNREADABLE}"))
}

/// The scene's problems.
fn problems(view: &View) -> Vec<String> {
    view.scene().expect("a scene").problems().to_vec()
}

#[test]
fn a_whole_stellar_with_no_type_sheet_draws_type_zero_in_flight() {
    for rules in [
        Rulebook::default(),
        Rulebook::default().with_override(RuleKey::TypeSheetFallback, RuleSource::Engine),
    ] {
        let view = unreadable_moon(rules);
        assert!(type_zero_drawn(&view), "the entry's rebuild point");
        assert!(!moon_placeholder_drawn(&view));
        assert!(
            problems(&view).contains(&format!("spöb 129 Type: {UNREADABLE}")),
            "{:?}",
            problems(&view)
        );
    }
}

#[test]
fn by_the_other_reading_it_draws_the_placeholder_and_why() {
    let view = unreadable_moon(
        Rulebook::default().with_override(RuleKey::TypeSheetFallback, RuleSource::Bible),
    );
    assert!(!type_zero_drawn(&view));
    assert!(moon_placeholder_drawn(&view));
    assert!(
        problems(&view).contains(&format!("spöb 129 Type: {UNREADABLE}")),
        "{:?}",
        problems(&view)
    );
}

// The sheet drawn and the click box agree (stellar-fidelity phase 1).

/// A flight in Sol whose Moon (129) can be destroyed, 50 square, with
/// a `DeadType` sheet 70 square when `dead`, and graphic type 0's
/// sheet 90 square: every one taller than a small sprite, so a click
/// box is its frame alone. Nothing is to the Moon's left.
fn agreeing(dead: bool, reading: RuleSource) -> View {
    FlightView::new(FakeCatalog {
        lives: vec![life(129, 0)],
        moon: Some(MoonSheets {
            width: 50,
            dead_width: dead.then_some(70),
            zero_width: 90,
            frames: 4,
            dead_frames: 1,
            flags2: 0,
        }),
        ..catalog()
    })
    .with_rules(Rulebook::default().with_override(RuleKey::DeadTypeFallback, reading))
}

/// The `rlëD` of the Moon's sheet drawn, having checked that a click
/// just inside that sheet's left edge picks the Moon and one just
/// outside picks nothing (its right edge can be under the status bar).
fn moon_drawn_and_clicked(view: &mut View) -> i16 {
    let moon = &view.scene().expect("a scene").stellars()[1];
    assert_eq!(moon.id, StellarId(129));
    let sheet = moon.drawn().expect("drawn").clone().expect("a sheet");
    let (at, half) = (moon.position, sheet.frame_width as f32 / 2.0);
    for (dx, picked) in [(1.0 - half, Some(StellarId(129))), (-1.0 - half, None)] {
        view.session.as_mut().expect("flying").nav_off();
        let screen = view.camera().world_to_screen(Point::new(at.x + dx, at.y));
        view.click_in_space(screen);
        let target = view.session().expect("flying").nav_target();
        assert_eq!(target, picked, "{:?} at {dx:+}", sheet.image);
    }
    let crate::SheetImage::Rled(id) = sheet.image else {
        panic!("an rlëD sheet: {:?}", sheet.image)
    };
    id
}

/// The Moon's sheet drawn whole, after `Y129` and a frame, after an
/// in-flight `M130` and a frame, after `U129` and a frame, and after
/// `Y129` and a frame, each agreeing with the click box.
fn agreement_walk(mut view: View) -> [i16; 5] {
    let whole = moon_drawn_and_clicked(&mut view);
    run_set(&mut view, "Y129");
    view.tick(TICK);
    let destroyed = moon_drawn_and_clicked(&mut view);
    run_set(&mut view, "M130");
    settle_session(&mut view);
    view.tick(TICK);
    let rebuilt = moon_drawn_and_clicked(&mut view);
    run_set(&mut view, "U129");
    view.tick(TICK);
    let regenerated = moon_drawn_and_clicked(&mut view);
    run_set(&mut view, "Y129");
    view.tick(TICK);
    [
        whole,
        destroyed,
        rebuilt,
        regenerated,
        moon_drawn_and_clicked(&mut view),
    ]
}

#[test]
fn the_sheet_drawn_and_the_click_box_agree_through_y_a_rebuild_point_and_u() {
    for reading in [RuleSource::Engine, RuleSource::Bible] {
        assert_eq!(
            agreement_walk(agreeing(true, reading)),
            [1129, 2129, 2129, 1129, 2129],
            "{reading:?}"
        );
    }
    assert_eq!(
        agreement_walk(agreeing(false, RuleSource::Engine)),
        [1129, 1129, TYPE_ZERO, 1129, 1129]
    );
    assert_eq!(
        agreement_walk(agreeing(false, RuleSource::Bible)),
        [1129; 5]
    );
}

// The frame count kept from the last rebuild point (stellar-fidelity
// phase 1).

/// A flight in Sol, by `rules`, whose Moon (129) can be destroyed and
/// animates a frame a tick only while destroyed (`Flags2` 0x0080), over
/// a `Type` sheet of `frames` frames and a 3-frame `DeadType` sheet.
fn flickering(frames: u16, rules: Rulebook) -> View {
    FlightView::new(FakeCatalog {
        lives: vec![life(129, 0x0080)],
        moon: Some(MoonSheets {
            width: 50,
            dead_width: Some(70),
            zero_width: 90,
            frames,
            dead_frames: 3,
            flags2: 0x0080,
        }),
        ..catalog()
    })
    .with_rules(rules)
}

/// The Moon's frame shown after each of `n` ticks, all of its
/// `DeadType` sheet.
fn dead_moon_frames(view: &mut View, n: usize) -> Vec<u16> {
    (0..n)
        .map(|_| {
            view.tick(TICK);
            let image = sprites(&drawn(view))[1].0;
            assert_eq!(image.id, 2129, "the DeadType sheet");
            image.frame
        })
        .collect()
}

/// An in-flight `M130`, a rebuild point.
fn move_in_flight(view: &mut View) {
    run_set(view, "M130");
    settle_session(view);
}

#[test]
fn after_y_a_stellar_animates_over_its_old_frame_count_until_the_next_rebuild_point() {
    let mut view = flickering(4, Rulebook::default());
    run_set(&mut view, "Y129");
    assert_eq!(
        dead_moon_frames(&mut view, 9),
        [1, 2, 0, 0, 1, 2, 0, 0, 1],
        "Type's 4 frames over the DeadType's 3"
    );
    // The rebuild keeps current frame 1, next 2, from the Type's count.
    move_in_flight(&mut view);
    assert_eq!(
        dead_moon_frames(&mut view, 7),
        [2, 0, 1, 2, 0, 1, 2],
        "the DeadType's 3"
    );
    let mut view = flickering(4, frame_count_by(RuleSource::Engine));
    run_set(&mut view, "Y129");
    assert_eq!(dead_moon_frames(&mut view, 4), [1, 2, 0, 0]);
}

fn frame_count_by(reading: RuleSource) -> Rulebook {
    Rulebook::default().with_override(RuleKey::StellarFrameCount, reading)
}

#[test]
fn by_the_bible_a_stellar_animates_over_the_sheet_it_draws_at_once() {
    let mut view = flickering(4, frame_count_by(RuleSource::Bible));
    run_set(&mut view, "Y129");
    assert_eq!(dead_moon_frames(&mut view, 7), [1, 2, 0, 1, 2, 0, 1]);
}

#[test]
fn a_stellar_of_one_type_frame_stays_still_after_y_until_the_next_rebuild_point() {
    let mut view = flickering(1, Rulebook::default());
    run_set(&mut view, "Y129");
    assert_eq!(dead_moon_frames(&mut view, 6), [0; 6]);
    move_in_flight(&mut view);
    assert_eq!(dead_moon_frames(&mut view, 4), [1, 2, 0, 1]);
    let mut view = flickering(1, frame_count_by(RuleSource::Bible));
    run_set(&mut view, "Y129");
    assert_eq!(dead_moon_frames(&mut view, 4), [1, 2, 0, 1], "the Bible's");
}

// A `spïn` that counts no frames (stellar-fidelity phase 8).

/// A flight in Sol, by `rules`, whose Moon (129) can be destroyed and
/// animates a frame a tick while whole, over a 4-frame `Type` sheet 50
/// square whose `spïn` counts no frames, with a 2-frame `DeadType` sheet
/// 70 square that counts.
fn uncounted_moon(rules: Rulebook) -> View {
    let mut catalog = FakeCatalog {
        lives: vec![life(129, 0)],
        moon: Some(MoonSheets {
            width: 50,
            dead_width: Some(70),
            zero_width: 90,
            frames: 4,
            dead_frames: 2,
            flags2: 0,
        }),
        ..catalog()
    };
    for site in catalog.sites.iter_mut().filter(|s| s.id == StellarId(129)) {
        site.type_counts_frames = false;
    }
    FlightView::new(catalog).with_rules(rules)
}

/// The Moon's sprites drawn, with where each is centred less where the
/// Moon is on screen.
fn moon_sprites(view: &View) -> Vec<(ImageKey, Point)> {
    let moon = view.scene().expect("a scene").stellars()[1].position;
    let at = view.camera().world_to_screen(moon);
    sprites(&drawn(view))
        .into_iter()
        .filter(|(image, _)| [1129, 2129, TYPE_ZERO].contains(&image.id))
        .map(|(image, centre)| (image, Point::new(centre.x - at.x, centre.y - at.y)))
        .collect()
}

#[test]
fn a_stellar_whose_type_counts_no_frames_draws_nothing_then_its_dead_frame_frozen_and_shifted() {
    let mut view = uncounted_moon(Rulebook::default());
    view.tick(TICK);
    assert_eq!(moon_sprites(&view), [], "flown: its slot shows no frame");
    run_set(&mut view, "Y129");
    view.tick(TICK);
    assert_eq!(
        moon_sprites(&view),
        [(ImageKey::sprite(2129, 0), Point::new(0.0, 0.0))],
        "its `DeadType`, still: it counted none at the rebuild point"
    );
    run_set(&mut view, "U129");
    for _ in 0..4 {
        view.tick(TICK);
        assert_eq!(
            moon_sprites(&view),
            [(
                ImageKey::sprite(2129, 0),
                Point::new(35.0 - 16.0, 35.0 - 16.0)
            )],
            "the `DeadType` frame kept, where a 32-pixel slot puts it"
        );
    }
    move_in_flight(&mut view);
    view.tick(TICK);
    assert_eq!(moon_sprites(&view), [], "a rebuild point empties it");
}

#[test]
fn by_the_other_reading_a_type_that_counts_no_frames_draws_and_animates_its_sheet() {
    let mut view = uncounted_moon(
        Rulebook::default().with_override(RuleKey::UncountedSheet, RuleSource::Bible),
    );
    let frames: Vec<Vec<(ImageKey, Point)>> = (0..3)
        .map(|_| {
            view.tick(TICK);
            moon_sprites(&view)
        })
        .collect();
    assert_eq!(
        frames,
        [1, 2, 3].map(|frame| vec![(ImageKey::sprite(1129, frame), Point::new(0.0, 0.0))])
    );
}

/// The Moon's frame shown.
fn moon_frame(view: &View) -> u16 {
    sprites(&drawn(view))[1].0.frame
}

#[test]
fn a_take_off_shows_a_stellars_current_frame_on_the_sheet_it_now_draws() {
    let mut view = FlightView::new(FakeCatalog {
        sites: vec![
            site(128, (6.0, -8.0), StellarFlags::CAN_LAND),
            site(129, (300.0, -200.0), StellarFlags::CAN_LAND),
        ],
        lives: vec![life(129, 0)],
        moon: Some(MoonSheets {
            width: 50,
            dead_width: Some(70),
            zero_width: 90,
            frames: 4,
            dead_frames: 2,
            flags2: 0,
        }),
        ..catalog()
    });
    ticks(&mut view, 2);
    assert_eq!(moon_frame(&view), 2);
    // The landing's two presses take a tick between them: frame 3.
    land_now(&mut view);
    run_set(&mut view, "Y129");
    view.take_off();
    assert_eq!(
        sprites(&drawn(&view))[1].0,
        ImageKey::sprite(2129, 1),
        "frame 3 of the DeadType's 2 at the take-off's rebuild, not 0"
    );
}

#[test]
fn a_move_within_the_system_keeps_its_stellars_animations() {
    let mut view = flight();
    ticks(&mut view, 2);
    assert_eq!(moon_frame(&view), 2);
    move_in_flight(&mut view);
    assert!(view.catch_up().system, "laid out");
    assert_eq!(moon_frame(&view), 2, "where it was");
    view.tick(TICK);
    assert_eq!(moon_frame(&view), 3, "and on from there");
}

#[test]
fn a_stellar_left_on_frame_2_shows_frame_2_after_jumping_away_and_back() {
    let mut view = flight();
    ticks(&mut view, 2);
    assert_eq!(moon_frame(&view), 2);
    run_set(&mut view, "M131");
    settle_session(&mut view);
    assert!(view.catch_up().system, "laid out in Alpha Centauri");
    ticks(&mut view, 5);
    run_set(&mut view, "M130");
    settle_session(&mut view);
    assert!(view.catch_up().system, "laid out in Sol again");
    assert_eq!(moon_frame(&view), 2, "nothing stepped while away");
    view.tick(TICK);
    assert_eq!(moon_frame(&view), 3, "and on from there");
}

/// The Moon's animation state as `view` keeps it while Sol is not shown.
fn moon_kept(view: &View) -> Option<u16> {
    view.kept_frames
        .get(StellarId(129))
        .map(|frames| frames.shown())
}

#[test]
fn a_real_jump_away_and_back_resumes_the_frame_shown_on_leaving() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    let mut last = moon_frame(&view);
    while view.session().map(Session::system) == Ok(SystemId(130)) {
        last = moon_frame(&view);
        view.tick(TICK);
    }
    // The tick the jump ends in steps the stellars before the session
    // arrives (`step_stellars` runs before the steps): one frame on from
    // the last drawn in Sol.
    let left = moon_kept(&view).expect("kept as Sol was let go");
    assert_eq!(left, (last + 1) % 4);
    assert_ne!(left, 0, "a frame a fresh layout would not show");
    while view.jump_effect().is_some() {
        view.tick(TICK);
    }
    plot(&mut view, 130);
    fly_out(&mut view);
    jump_now(&mut view);
    assert_eq!(moon_kept(&view), Some(left), "still while away");
    while view.session().map(Session::system) == Ok(SystemId(131)) {
        view.tick(TICK);
    }
    // Sol is laid out on the tick the jump ends, after that tick's step:
    // the frame drawn on arrival is the one it was left on.
    assert_eq!(moon_frame(&view), left, "resumed on arrival");
    view.tick(TICK);
    assert_eq!(moon_frame(&view), (left + 1) % 4, "and on from there");
}

/// The frame Proxima (140) shows, while the scene holds it.
fn proxima_frame(view: &View) -> Option<u16> {
    view.scene()
        .expect("laid out")
        .stellars()
        .iter()
        .find(|stellar| stellar.id == StellarId(140))
        .map(|stellar| stellar.frames.shown())
}

#[test]
fn a_stellar_shown_again_in_the_same_system_resumes() {
    let catalog = FakeCatalog {
        proxima_frames: 4,
        ..proxima_in_sol()
    };
    let mut view = FlightView::with_pilot(catalog, docked_pilot("Ada"));
    land_now(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    move_to(&mut view, 131, 140);
    view.tick(TICK);
    assert_eq!(scene_ids(&view), [StellarId(140)], "docked at it");
    ticks(&mut view, 2);
    let docked = proxima_frame(&view).expect("shown");
    assert_ne!(docked, 0, "under way");
    assert_eq!(view.take_off(), Some(StellarId(140)));
    assert_eq!(scene_ids(&view), [], "let go by the take-off's rebuild");
    // Docked at Earth, then moved back to Proxima: shown again.
    let session = view.session.as_mut().expect("flying");
    session
        .land_at(&view.catalog, SystemId(130), StellarId(128))
        .expect("lands");
    move_to(&mut view, 131, 140);
    view.catch_up();
    assert_eq!(scene_ids(&view), [StellarId(140)], "shown again");
    assert_eq!(proxima_frame(&view), Some(docked), "where it was left");
}

#[test]
fn a_load_shows_every_stellar_from_frame_0() {
    let mut view = flight();
    ticks(&mut view, 2);
    assert_eq!(moon_frame(&view), 2);
    let pilot = view.pilot().expect("flying").clone();
    let mut loaded = FlightView::loaded(catalog(), pilot.clone(), Rulebook::default());
    assert_eq!(moon_frame(&loaded), 0, "loaded");
    loaded.tick(TICK);
    assert_eq!(moon_frame(&loaded), 1, "loaded, then on");
    let mut resumed = FlightView::with_pilot(catalog(), pilot);
    assert_eq!(moon_frame(&resumed), 0, "resumed");
    resumed.tick(TICK);
    assert_eq!(moon_frame(&resumed), 1, "resumed, then on");
    // A new pilot's flight is ticked once past the land hint.
    let mut new = new_pilot(FlightView::new(catalog()));
    assert_eq!(moon_frame(&new), 1, "a new pilot's, one tick in");
    new.tick(TICK);
    assert_eq!(moon_frame(&new), 2, "a new pilot's, then on");
}

#[test]
fn a_galaxy_change_keeps_the_stellars_animations() {
    let mut view = destructible();
    ticks(&mut view, 2);
    run_set(&mut view, "Y140");
    assert!(view.catch_up().galaxy);
    assert_eq!(moon_frame(&view), 2, "not laid out afresh");
}

#[test]
fn a_frame_with_the_map_open_still_redraws_from_the_events() {
    let mut view = flight();
    tap(&mut view, MAP);
    assert!(view.map_open());
    view.catalog().systems_read.borrow_mut().clear();
    run_set(&mut view, "M131");
    settle_session(&mut view);
    view.tick(TICK);
    assert!(view.map_open(), "still open");
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(systems_read(&view), [SystemId(131)]);

    let mut view = FlightView::new(shipbuying());
    tap(&mut view, MAP);
    run_set(&mut view, "H129");
    view.tick(TICK);
    assert!(view.map_open(), "still open");
    assert_eq!(sheets(&view), [ShipId(128), ShipId(129)]);
}

#[test]
fn a_frame_during_the_streak_still_redraws_the_hull() {
    let mut view = FlightView::new(shipbuying());
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    run_set(&mut view, "H129");
    view.tick(TICK);
    assert!(view.jump_effect().is_some(), "still streaking");
    assert_eq!(view.session().map(Session::system), Ok(SystemId(130)));
    assert_eq!(sheets(&view), [ShipId(128), ShipId(129)]);
}

#[test]
fn fighters_abandoned_are_told_only_with_the_layout_of_the_move_settled() {
    let rules = Rulebook::default().with_override(RuleKey::MoveArrival, RuleSource::Bible);
    let mut view = fighters_out().with_rules(rules);
    run_set(&mut view, "M131");
    settle_session(&mut view);
    view.tick(TICK);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    view.settle_script();
    assert_eq!(view.message(), None, "no layout from this settling");
    let session = view.session.as_mut().expect("flying");
    assert_eq!(
        session.take_fighter_notes(),
        [FighterNote::Abandoned(2)],
        "left for the next arrival"
    );
}

#[test]
fn a_desk_move_redraws_with_no_extra_call() {
    let mut view = landed_view();
    move_to(&mut view, 131, 140);
    view.tick(TICK);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(systems_read(&view), [SystemId(131)]);
    assert_eq!(view.course_map().current(), Some(SystemId(131)));
    assert_eq!(view.shown_position(), Point::new(0.0, 0.0), "at Proxima");

    let mut view = landed_view();
    plot(&mut view, 131);
    move_to(&mut view, 130, 129);
    view.tick(TICK);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    assert_eq!(systems_read(&view), [], "the scene kept");
    assert_eq!(view.course_map().route(), [], "the course cleared");
    assert_eq!(
        view.shown_position(),
        Point::new(300.0, -200.0),
        "at the Moon"
    );
}

#[test]
fn the_disable_rule_reaches_the_session_too() {
    let rule = Rc::new(Counting::default());
    let view = flight().with_disable_rule(rule.clone());
    let session = view.session().expect("flying");
    assert!(
        !session
            .disable_rule()
            .disabled(nova_sim::Gauge::full(10.0), &session.hull())
    );
    assert_eq!(rule.asked.get(), 1, "the session asks the rule given");
}

#[test]
fn the_string_lists_reach_the_session() {
    let strings = FakeCatalog {
        strings: vec![(25040, vec!["Kestrel".to_owned()])],
        ..catalog()
    };
    let view = flight().with_strings(Rc::new(strings));
    assert_eq!(
        view.session().expect("flying").strings().string_list(25040),
        ["Kestrel"]
    );
}

/// A flight in Sol with an arrival coming out of hypergate 141 at
/// (200, 100), which can be destroyed, opens only while destroyed
/// (`Flags2` 0x0080) and steps every tick; its `Type` sheet (1141) is
/// 20 square and its `spïn` counts no frames, while its `DeadType` sheet
/// (2141), 100 square, counts 6.
fn uncounted_gate() -> View {
    let mut catalog = trafficked(&[130], 1, 129, 3);
    catalog.sites.push(LandingSite {
        flags2: nova_sim::gate::HYPERGATE | 0x0080,
        exit_angle: 90,
        type_counts_frames: false,
        dead_graphic: Some(9),
        dead_frame_size: Some((100, 100)),
        animation: nova_sim::catalog::SiteAnimation {
            type_frames: 6,
            dead_frames: 6,
            ..nova_sim::catalog::SiteAnimation::default()
        },
        ..site(141, (200.0, 100.0), StellarFlags::CAN_LAND)
    });
    catalog.sol_more.push(StellarContents {
        dead_sprite: Some(Ok(StellarSheet {
            image: crate::SheetImage::Rled(2141),
            frames: NonZeroU16::new(6).expect("non-zero"),
            frame_width: 100,
            frame_height: 100,
        })),
        ..stellar(141, "Gate", (200, 100), 6)
    });
    catalog.lives = vec![life(141, 0x0080)];
    // Setup brings nobody; then an arrival comes out of the gate, the
    // third of the stellars.
    let (_, chance) = scripted(&[0, 0, 6, 6, 0, 0, 0, 2]);
    FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still))
}

/// The frames of hypergate 141's sheets drawn.
fn gate_drawn(view: &View) -> Vec<ImageKey> {
    sprites(&drawn(view))
        .into_iter()
        .map(|(image, _)| image)
        .filter(|image| [1141, 2141].contains(&image.id))
        .collect()
}

#[test]
fn a_gate_frozen_by_u_keeps_the_frame_it_showed_before_its_session_steps_it() {
    // The view reads the freeze before the gate frames: on the tick the
    // `U` clone freezes the slot, it holds the frame drawn on the tick
    // before, not the one the session steps the gate to on this one.
    let mut view = uncounted_gate();
    ticks(&mut view, 2);
    assert_eq!(gate_drawn(&view), [], "flown: its slot shows no frame");
    run_set(&mut view, "Y141");
    ticks(&mut view, 3);
    let shown = gate_drawn(&view);
    assert_eq!(
        shown,
        [ImageKey::sprite(2141, 3)],
        "opening while destroyed"
    );
    run_set(&mut view, "U141");
    for _ in 0..4 {
        view.tick(TICK);
        let session = view.session().expect("flying");
        assert!(session.frame_frozen(StellarId(141)));
        assert_eq!(
            session.gate_frame(StellarId(141)),
            Some(0),
            "whole, the session steps it back to 0"
        );
        assert_eq!(gate_drawn(&view), shown, "the frame shown before, kept");
    }
}

// Random next frames (stellar-fidelity phase 10).

/// The Moon's frames over 3 ticks of a flight in Sol whose Moon (129), of
/// 4 frames a tick, has `flags2`, its effects drawn on `draws` and its
/// session on a script of its own: those frames, and what each was asked.
fn moon_frames_drawn(flags2: u16, draws: &[u32]) -> (Vec<u16>, Vec<u32>, Vec<u32>) {
    let (effects, effects_chance) = scripted(draws);
    let (session, chance) = scripted(&[]);
    let mut view = FlightView::new(FakeCatalog {
        moon: Some(MoonSheets {
            width: 50,
            dead_width: None,
            zero_width: 90,
            frames: 4,
            dead_frames: 1,
            flags2,
        }),
        ..catalog()
    })
    .with_effects_chance(effects_chance)
    .with_chance(chance);
    let frames = (0..3)
        .map(|_| {
            view.tick(TICK);
            moon_frame(&view)
        })
        .collect();
    let effects = effects.borrow().asked.clone();
    let session = session.borrow().asked.clone();
    (frames, effects, session)
}

#[test]
fn a_random_stellars_frames_roll_on_the_effects_chance() {
    let random = nova_sim::gate_sprite::RANDOM;
    let (frames, effects, session) = moon_frames_drawn(random, &[0, 0, 0]);
    // From 1, a draw of 0 is frame 0; from 0, frame 1.
    assert_eq!(frames, [1, 0, 1]);
    assert_eq!(effects, [3, 3, 3]);
    let (plain, unasked, plain_session) = moon_frames_drawn(0, &[]);
    assert_eq!(plain, [1, 2, 3]);
    assert!(unasked.is_empty(), "{unasked:?}");
    assert_eq!(session, plain_session, "the session's script untouched");
}

// Draw order (crons-news-ranks-fidelity phase 14).

#[test]
fn overlapping_stellars_are_drawn_lowest_id_first_whatever_their_navdefs_order() {
    let mut catalog = FakeCatalog {
        sol_more: vec![stellar(127, "Under the Moon", (300, -200), 1)],
        ..catalog()
    };
    catalog
        .sites
        .push(site(127, (300.0, -200.0), StellarFlags::CAN_LAND));
    let mut view = FlightView::new(catalog);
    view.tick(TICK);
    assert_eq!(
        scene_ids(&view),
        [StellarId(128), StellarId(129), StellarId(127)],
        "Sol lists the Moon before 127"
    );
    let images: Vec<i16> = sprites(&drawn(&view))
        .into_iter()
        .map(|(image, _)| image.id)
        .filter(|id| (1127..=1129).contains(id))
        .collect();
    assert_eq!(images, [1127, 1128, 1129], "127 under the Moon");
}
