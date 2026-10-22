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

#[test]
fn the_course_map_shows_the_course_as_it_is_flown() {
    let catalog = catalog();
    let mut session = Session::start(&catalog).expect("starts");
    session.plot_course(SystemId(131)).expect("a route");
    let mut view = FlightView::with_pilot(catalog, session.pilot().clone());
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
    let mut view = FlightView::with_pilot(catalog, session.pilot().clone());
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
            only_when_destroyed: false,
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
    let sheet = moon.drawn().clone().expect("a sheet");
    let (at, half) = (moon.position, sheet.frame_width as f32 / 2.0);
    for (dx, picked) in [(1.0 - half, Some(StellarId(129))), (-1.0 - half, None)] {
        view.session.as_mut().expect("flying").nav_off();
        let screen = view.camera().world_to_screen(Point::new(at.x + dx, at.y));
        view.click_in_space(screen);
        let target = view.session().expect("flying").nav_target();
        assert_eq!(target, picked, "{} at {dx:+}", sheet.image_id);
    }
    sheet.image_id
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
            only_when_destroyed: true,
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
    move_in_flight(&mut view);
    assert_eq!(
        dead_moon_frames(&mut view, 7),
        [1, 2, 0, 1, 2, 0, 1],
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
            only_when_destroyed: false,
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
fn laying_a_system_out_starts_its_stellars_animations_afresh() {
    let mut view = flight();
    ticks(&mut view, 2);
    assert_eq!(moon_frame(&view), 2);
    move_in_flight(&mut view);
    assert!(view.catch_up().system, "laid out");
    assert_eq!(moon_frame(&view), 0);
    view.tick(TICK);
    assert_eq!(moon_frame(&view), 1);
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
