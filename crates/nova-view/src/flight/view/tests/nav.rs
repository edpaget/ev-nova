use super::*;

// The navigation target.

/// Two stellars the ship starts over, at rest: A (128), small and
/// nearest, then B (129), wide and a little further off, in that nav
/// order; and C (130), drawn where the status panel covers it.
fn a_and_b() -> View {
    let wide = LandingSite {
        frame_size: Some((200, 200)),
        ..site(129, (40.0, 0.0), StellarFlags::CAN_LAND)
    };
    flight_among(vec![
        site(128, (10.0, 0.0), StellarFlags::CAN_LAND),
        wide,
        site(130, (427.0, 0.0), StellarFlags::CAN_LAND),
    ])
}

/// A scripted flight in Sol showing Earth and the Moon, the player at the
/// centre.
fn faked_sol_scene() -> Faked {
    let mut frame =
        super::faked_world::in_sol_with(vec![super::faked_world::the_player_at(Vec2::ZERO, 0.0)]);
    frame.nav_mode = NavMode::Stellar(None);
    super::faked_world::world(catalog(), frame)
}

/// Where `view` puts world point (`x`, `y`) on screen.
fn faked_on_screen(view: &Faked, (x, y): (f32, f32)) -> Point {
    view.camera().world_to_screen(at(x, y))
}

/// The world points the clicks `view` sent in space hit.
fn clicked(view: &Faked) -> Vec<Vec2> {
    sent(view)
        .iter()
        .filter_map(|command| match command {
            FlightCommand::ClickInSpace { at, .. } => Some(*at),
            _ => None,
        })
        .collect()
}

#[test]
fn a_click_in_space_sends_the_world_point_drawn_under_it() {
    // What it then picks (the stellar under it, or nothing, keeping the
    // target) is the session's (nova-sim's `session::hover` click tests,
    // `session::apply::tests::a_click_in_space_on_nothing_is_refused_and_changes_nothing`).
    let mut view = faked_sol_scene();
    for (x, y) in [(0.0, -150.0), (0.0, 150.0), (40.0, 0.0), (-300.0, -300.0)] {
        let drawn = faked_on_screen(&view, (x, y));
        view.click_in_space(drawn);
    }
    assert_eq!(
        clicked(&view),
        [
            Vec2::new(0.0, -150.0),
            Vec2::new(0.0, 150.0),
            Vec2::new(40.0, 0.0),
            Vec2::new(-300.0, -300.0)
        ]
    );
}

#[test]
fn a_right_click_or_a_release_selects_nothing() {
    let mut view = a_and_b();
    let b = on_screen(&view, (40.0, 0.0));
    for (button, pressed) in [
        (crate::MouseButton::Right, true),
        (crate::MouseButton::Left, false),
    ] {
        view.input(&Input::PointerButton {
            button,
            pressed,
            at: b,
        });
        assert_eq!(nav_target(&view), None, "{button:?} {pressed}");
    }
}

#[test]
fn a_click_on_the_status_panel_is_no_click_in_space() {
    let mut view = faked_sol_scene();
    let edge = VIEW_SIZE.0 - STATUS_BAR_WIDTH;
    let c = faked_on_screen(&view, (427.0, 0.0));
    assert!(c.x > edge, "{c:?} is under the panel");
    view.click_in_space(c);
    view.click_in_space(at(edge, c.y));
    assert_eq!(clicked(&view), [], "the edge is panel, and silent");
    view.click_in_space(at(edge - 0.5, c.y));
    assert_eq!(clicked(&view).len(), 1, "left of it is space");
}

#[test]
fn far_from_the_centre_nav_off_with_a_course_lets_the_arrow_show() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_far(&mut view);
    let most_lines = |view: &mut View| {
        (0..32)
            .map(|_| {
                ticks(view, 1);
                arrow_lines(view)
            })
            .max()
    };
    assert_eq!(most_lines(&mut view), Some(0), "mode 3 hides it");
    tap(&mut view, NAV_OFF_KEY);
    assert_eq!(most_lines(&mut view), Some(3), "mode -1 shows it");
}

#[test]
fn closing_the_course_map_cues_the_courses_first_hop() {
    let mut view = flight();
    plot(&mut view, 131);
    tap(&mut view, NAV_OFF_KEY);
    assert_eq!(nav_mode(&view), NavMode::Off);
    tap(&mut view, MAP);
    tap(&mut view, MAP);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
}

#[test]
fn the_hypergate_map_opens_with_the_current_system_selected() {
    let mut view = FlightView::new(gated());
    plot(&mut view, 131);
    land_now(&mut view);
    let map = view.course_map();
    assert_eq!(map.mode(), MapMode::Hypergate);
    assert_eq!(map.selected(), Some(SystemId(130)), "Sol");
    assert_eq!(map.hyper_selection(), None, "nothing in mode 3");
    tap(&mut view, MAP);
    assert_eq!(view.message(), Some(HYPERGATE_CANCELLED), "no pick");
}

#[test]
fn closing_the_hypergate_map_turns_the_nav_off_and_cues_nothing() {
    let mut view = FlightView::new(gated());
    plot(&mut view, 131);
    land_now(&mut view);
    tap(&mut view, MAP);
    assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
    assert_eq!(course(&view), [SystemId(131)], "the course kept");
    assert_eq!(nav_mode(&view), NavMode::Off);
}

// The nav target's brackets.

/// The nav bracket `cïcn`s drawn (10000-10007), and where.
fn nav_icons(view: &View) -> Vec<(i16, Point)> {
    drawn(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Picture { image, top_left } if icon_in(10000..=10007)(command) => {
                Some((image.id, *top_left))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn selecting_a_stellar_draws_cicn_10000_to_10003_flying_in_then_settled() {
    let mut view = flight();
    assert_eq!(nav_icons(&view), []);
    tap_slot(&mut view, STELLAR_SLOT_KEYS[0][0]);
    // Earth (128), 20 x 20: half 10.
    let earth = on_screen(&view, (0.0, -600.0));
    assert_eq!(
        nav_icons(&view),
        corners_at(10000, earth, 10.0 + 196.0),
        "the frame of the pick"
    );
    for expand in [136.0, 76.0, 16.0, 0.0, 0.0] {
        view.tick(TICK);
        let earth = on_screen(&view, (0.0, -600.0));
        assert_eq!(
            nav_icons(&view),
            corners_at(10000, earth, 10.0 + expand),
            "{expand}"
        );
    }
}

/// The nav bracket `cïcn`s `view` draws (10000-10007) with `nav`, and
/// where.
fn faked_nav_icons(
    view: &Faked,
    nav: Option<nova_sim::brackets::NavBrackets>,
) -> Vec<(i16, Point)> {
    let hud = super::super::world::WorldHud {
        nav_brackets: nav,
        ..super::super::world::WorldHud::none()
    };
    super::faked_world::world_list_with(view, &hud)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Picture { image, top_left } if icon_in(10000..=10007)(command) => {
                Some((image.id, *top_left))
            }
            _ => None,
        })
        .collect()
}

/// The brackets round stellar `id` at `at`, settled or `expand` out.
fn nav_at(id: i16, (x, y): (f32, f32), expand: f32) -> nova_sim::brackets::NavBrackets {
    nova_sim::brackets::NavBrackets {
        stellar: StellarId(id),
        position: Vec2::new(x, y),
        expand,
        dominated: false,
    }
}

#[test]
fn no_nav_brackets_are_drawn_without_the_flights_and_its_are_drawn_round_the_stellar() {
    // When the session has none (Nav Off, Hyper Select) and how they fly
    // in: nova-sim's brackets tests.
    let view = faked_sol_scene();
    assert_eq!(faked_nav_icons(&view, None), []);
    let moon = faked_on_screen(&view, (300.0, -200.0));
    assert_eq!(
        faked_nav_icons(&view, Some(nav_at(129, (300.0, -200.0), 196.0))),
        corners_at(10000, moon, 10.0 + 196.0),
        "flying in"
    );
}

#[test]
fn a_stellar_whose_sprite_cannot_be_read_is_bracketed_at_half_64() {
    let unreadable = StellarContents {
        sprite: Err("no rlëD".to_owned()),
        ..stellar(131, "Broken", (100, 0), 1)
    };
    let mut frame =
        super::faked_world::in_sol_with(vec![super::faked_world::the_player_at(Vec2::ZERO, 0.0)]);
    frame.stellars.push(record::stellar(StellarId(131)));
    let view = super::faked_world::world(
        FakeCatalog {
            sol_more: vec![unreadable],
            ..catalog()
        },
        frame,
    );
    let broken = faked_on_screen(&view, (100.0, 0.0));
    assert_eq!(
        faked_nav_icons(&view, Some(nav_at(131, (100.0, 0.0), 0.0))),
        corners_at(10000, broken, 64.0)
    );
}

/// A selection the scene doesn't hold is bracketed at its spöb position
/// at half 64 (`_HandleNavTargetSprites` @0x2fd9f-0x2fdda, @0x2fe41).
/// A selection the scene doesn't hold is bracketed at its spöb position
/// at half 64 (`_HandleNavTargetSprites` @0x2fd9f-0x2fdda, @0x2fe41).
#[test]
fn an_unseen_selection_is_bracketed_at_its_spot_at_half_64() {
    let view = faked_sol_scene();
    let hole = faked_on_screen(&view, (0.0, 300.0));
    assert_eq!(
        faked_nav_icons(&view, Some(nav_at(150, (0.0, 300.0), 196.0))),
        corners_at(10000, hole, 64.0 + 196.0),
        "flying in"
    );
    assert_eq!(
        faked_nav_icons(&view, Some(nav_at(150, (0.0, 300.0), 0.0))),
        corners_at(10000, hole, 64.0)
    );
}

/// `_DrawStatusNav` names any selected stellar by its spöb name
/// (@0x4a0fd-0x4a149).
#[test]
fn the_hud_names_an_unseen_selection() {
    let mut view = flight_with_unseen(unseen_wormhole());
    tap(&mut view, LAND);
    assert_eq!(nav(&view), [hud::NAV_STELLAR, "Hole"]);
}

#[test]
fn a_dominated_stellar_is_bracketed_green() {
    // Whether it is: the session's (its `Flags2` DOMINATED).
    let view = faked_sol_scene();
    let earth = faked_on_screen(&view, (0.0, -600.0));
    let dominated = Some(nova_sim::brackets::NavBrackets {
        dominated: true,
        ..nav_at(128, (0.0, -600.0), 0.0)
    });
    assert_eq!(
        faked_nav_icons(&view, dominated),
        corners_at(10004, earth, 10.0)
    );
}

#[test]
fn with_the_map_open_a_click_goes_to_the_map_not_the_stellars() {
    let mut view = a_and_b();
    let b = on_screen(&view, (40.0, 0.0));
    tap(&mut view, MAP);
    click(&mut view, b);
    tap(&mut view, MAP);
    assert!(!view.map_open());
    assert_eq!(nav_target(&view), None);
}

/// The texts drawn in the HUD's nav area, as [`nav`] gives them, each
/// with its colour.
fn nav_colored(view: &View) -> Vec<(String, Color)> {
    let area = layout().nav.offset(at(830.0, 0.0));
    drawn(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text {
                text,
                origin,
                color,
                ..
            } if area.contains(*origin) => Some((text.clone(), *color)),
            _ => None,
        })
        .collect()
}

/// The colour the nav area's value, its second line, is drawn in.
fn nav_value_color(view: &View) -> Color {
    nav_colored(view)[1].1
}

/// `text` from `x` right of the stock cargo area's left, (8, 458) at
/// (830, 0), on `baseline` below its top, in Geneva 12.
fn cargo_line(text: &str, x: f32, baseline: f32) -> (String, Point) {
    (text.to_owned(), at(838.0 + x, 458.0 + baseline - 12.0))
}

#[test]
fn the_hud_lists_the_goods_bought_the_free_space_and_the_cash() {
    let labels = FakeCatalog {
        strings: vec![(4003, vec!["Food:".to_owned(), "Ind:".to_owned()])],
        ..catalog()
    };
    let mut view = FlightView::new(trading()).with_strings(Rc::new(labels));
    assert_eq!(
        cargo(&view),
        [
            cargo_line(hud::CARGO_FREE, 77.0, 12.0),
            cargo_line("10", 110.0, 12.0),
            cargo_line("Credits:", 77.0, 66.0),
            cargo_line("1,000", 87.0, 82.0),
        ],
        "an empty 10-ton hold"
    );
    land_now(&mut view);
    assert_eq!(view.trade(BUY_FOOD), Ok(10));
    assert_eq!(
        cargo(&view),
        [
            cargo_line("Food:", 3.0, 12.0),
            cargo_line("10", 41.0, 12.0),
            cargo_line(hud::CARGO_FREE, 77.0, 12.0),
            cargo_line("0", 110.0, 12.0),
            cargo_line("Credits:", 77.0, 66.0),
            cargo_line("250", 87.0, 82.0),
        ]
    );
}

#[test]
fn a_cargo_area_with_room_below_the_credits_draws_no_date() {
    assert_eq!(
        layout().cargo,
        crate::geometry::Bounds {
            min: at(8.0, 458.0),
            max: at(184.0, 552.0),
        }
    );
    let taller = FakeCatalog {
        bar: Ok(StatusBarLayout {
            cargo: crate::geometry::Bounds {
                min: at(8.0, 440.0),
                max: at(184.0, 552.0),
            },
            ..layout()
        }),
        ..catalog()
    };
    let mut view = FlightView::new(taller);
    let no_date = |view: &View| {
        let session = view.session().expect("flying");
        let date = [session.date_text(), session.long_date_text()];
        let dates: Vec<_> = drawn(view)
            .iter()
            .filter(
                |command| matches!(command, DrawCommand::Text { text, .. } if date.contains(text)),
            )
            .cloned()
            .collect();
        assert_eq!(dates, [], "no date drawn for {date:?}");
        assert!(
            cargo(view).iter().any(|(text, _)| text == "Credits:"),
            "the cargo lines are still drawn"
        );
    };
    no_date(&view);
    jump_to_alpha(&mut view);
    no_date(&view);
}

#[test]
fn the_nav_area_is_centred_by_the_metrics_given() {
    use crate::text::TextMetrics as _;
    use crate::text::fixture::MonoMetrics;
    let view = flight().with_metrics(Rc::new(MonoMetrics));
    let origin = drawn(&view)
        .iter()
        .find_map(|c| match c {
            DrawCommand::Text { text, origin, .. } if text == hud::NAV_OFF => Some(*origin),
            _ => None,
        })
        .expect("Nav System Off");
    let layout = layout();
    let area = layout.nav.offset(at(830.0, 0.0));
    let width = MonoMetrics.width(layout.font, layout.font_size, hud::NAV_OFF);
    // The stock area's middle is 830 + (8 + 184) / 2 = 926.
    assert_eq!(f32::midpoint(area.min.x, area.max.x), 926.0);
    assert_eq!(
        origin,
        at(
            926.0 - (width / 2.0).trunc(),
            area.min.y + hud::NAV_OFF_BASELINE - layout.font_size
        )
    );
}

#[test]
fn the_destination_is_dim_until_the_ship_is_out_far_enough_to_jump() {
    // Read the Bible's way, the nav area follows every frame.
    let mut view = every_frame(flight());
    plot(&mut view, 131);
    let hyperspace = |value: Color| {
        vec![
            (hud::NAV_HYPERSPACE.to_owned(), Color::DIM),
            (hud::NAV_UNEXPLORED.to_owned(), value),
        ]
    };
    assert_eq!(nav_colored(&view), hyperspace(Color::DIM));
    let clear_iff_out = |view: &View| {
        let out = player(view).position.length() >= MIN_JUMP_DISTANCE;
        let expected = if out { Color::WHITE } else { Color::DIM };
        assert_eq!(nav_value_color(view), expected, "{:?}", player(view));
        out
    };
    view.input(&key(Key::Up, true));
    let mut dim_ticks = 0;
    while !clear_iff_out(&view) {
        dim_ticks += 1;
        assert!(dim_ticks < 2000, "never got out");
        view.tick(TICK);
    }
    assert!(dim_ticks > 0, "it started inside");
    assert_eq!(nav_colored(&view), hyperspace(Color::WHITE));
    // Back inside: Down turns against the motion, then Up thrusts.
    view.input(&key(Key::Up, false));
    view.input(&key(Key::Down, true));
    for _ in 0..90 {
        view.tick(TICK);
        clear_iff_out(&view);
    }
    view.input(&key(Key::Down, false));
    view.input(&key(Key::Up, true));
    let mut out_ticks = 0;
    while clear_iff_out(&view) {
        out_ticks += 1;
        assert!(out_ticks < 2000, "never got back in");
        view.tick(TICK);
    }
    view.release_keys();
    assert_eq!(nav_colored(&view), hyperspace(Color::DIM));
}

#[test]
fn in_a_system_with_no_stellars_the_destination_is_bright_from_the_centre() {
    let mut view = FlightView::new(FakeCatalog {
        sites: Vec::new(),
        ..catalog()
    });
    plot(&mut view, 131);
    assert_eq!(player(&view).position, Vec2::ZERO);
    assert_eq!(nav_value_color(&view), Color::WHITE);
}

#[test]
fn once_j_is_accepted_the_hyperspace_label_turns_bright_too() {
    let mut view = every_frame(flight());
    plot(&mut view, 131);
    fly_out(&mut view);
    tap(&mut view, JUMP);
    assert_eq!(
        nav_colored(&view),
        [
            (hud::NAV_HYPERSPACE.to_owned(), Color::WHITE),
            (hud::NAV_UNEXPLORED.to_owned(), Color::WHITE),
        ]
    );
}

#[test]
fn the_nav_area_holds_its_colours_until_its_next_refresh() {
    let mut view = flight();
    plot(&mut view, 131);
    assert_eq!(nav_value_color(&view), Color::DIM, "inside, at once");
    view.input(&key(Key::Up, true));
    let mut expected = Color::DIM;
    let mut held = 0;
    for _ in 0..2000 {
        let before = now_tick(&view);
        view.tick(TICK);
        let now = now_tick(&view);
        let out = player(&view).position.length() >= MIN_JUMP_DISTANCE;
        if now / NAV_PERIOD_TICKS > before / NAV_PERIOD_TICKS {
            expected = if out { Color::WHITE } else { Color::DIM };
        }
        assert_eq!(nav_value_color(&view), expected, "tick {now}");
        if out && expected == Color::DIM {
            held += 1;
        }
        if expected == Color::WHITE {
            break;
        }
    }
    view.release_keys();
    assert_eq!(expected, Color::WHITE, "it got out");
    assert!(
        (1..u32::try_from(NAV_PERIOD_TICKS).expect("small")).contains(&held),
        "held dim {held} ticks after the ship got clear"
    );
}

#[test]
fn the_nav_area_holds_its_text_until_its_next_refresh() {
    let mut view = flight();
    plot(&mut view, 131);
    // Move to just past a refresh.
    while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
        view.tick(TICK);
    }
    run_set(&mut view, "X131");
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
    for _ in 1..NAV_PERIOD_TICKS - 1 {
        view.tick(TICK);
        assert_eq!(
            nav(&view),
            [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED],
            "tick {}",
            now_tick(&view)
        );
    }
    view.tick(TICK);
    assert_eq!(now_tick(&view) % NAV_PERIOD_TICKS, 0);
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Alpha Centauri"]);
}

#[test]
fn read_the_bibles_way_the_nav_area_follows_every_frame() {
    let mut view = every_frame(flight());
    plot(&mut view, 131);
    while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
        view.tick(TICK);
    }
    run_set(&mut view, "X131");
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Alpha Centauri"]);
}

#[test]
fn a_selection_change_shows_at_once() {
    let mut view = flight();
    while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
        view.tick(TICK);
    }
    tap(&mut view, LAND);
    assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"], "drawn at once");
    view.tick(TICK);
    assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"], "and kept");
    tap(&mut view, HYPER_SELECT);
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
    view.tick(TICK);
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
    assert_ne!(now_tick(&view) % NAV_PERIOD_TICKS, 0, "between refreshes");
}

#[test]
fn redrawing_the_status_retakes_the_nav_area() {
    let mut view = flight();
    plot(&mut view, 131);
    while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
        view.tick(TICK);
    }
    run_set(&mut view, "X131");
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
    view.redraw_status();
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Alpha Centauri"]);
}

#[test]
fn once_j_is_accepted_the_label_turns_bright_at_the_next_refresh() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
        view.tick(TICK);
    }
    let label = |view: &View| nav_colored(view)[0].1;
    assert_eq!(label(&view), Color::DIM);
    tap(&mut view, JUMP);
    assert_eq!(label(&view), Color::DIM, "held");
    for _ in 1..NAV_PERIOD_TICKS - 1 {
        view.tick(TICK);
        assert_eq!(label(&view), Color::DIM, "tick {}", now_tick(&view));
    }
    view.tick(TICK);
    assert_eq!(label(&view), Color::WHITE, "tick {}", now_tick(&view));
}

#[test]
fn arriving_clears_the_target_and_l_selects_in_the_new_system() {
    let mut view = flight();
    tap(&mut view, LAND);
    jump_to_alpha(&mut view);
    assert_eq!(nav_target(&view), None);
    assert_eq!(nav(&view), [hud::NAV_OFF]);
    tap(&mut view, LAND);
    assert_eq!(nav(&view), [hud::NAV_STELLAR, "Proxima"]);
}

#[test]
fn tab_in_a_flight_that_never_started_does_nothing() {
    let mut view = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert_eq!(view.input(&key(Key::Tab, true)), ScreenAction::None);
    assert!(view.session().is_err());
}

// The radar's IFF, density scanner, centre arrow and static.

/// `catalog`, its ship carrying `defaults` of [`radar_outfits`].
fn outfitted(catalog: FakeCatalog, defaults: &[(i16, u16)]) -> FakeCatalog {
    FakeCatalog {
        outfits: radar_outfits(),
        defaults: defaults.iter().map(|&(id, n)| (OutfitId(id), n)).collect(),
        ..catalog
    }
}

#[test]
fn an_iff_outfit_turns_on_the_black_backdrop_and_the_cyan_player() {
    let mut view = FlightView::new(outfitted(catalog(), &[(300, 1)]));
    ticks(&mut view, 1);
    assert!(view.radar_shown().iff);
    let marks = radar_marks(&view);
    assert_eq!(
        marks[0],
        RadarMark::Fill {
            rect: STOCK_RADAR,
            color: RadarColor::Black
        }
    );
    assert!(marks.contains(&RadarMark::Pixel {
        x: 926,
        y: 96,
        color: RadarColor::Cyan
    }));
    // Earth and the Moon, landable to a clean record: yellow.
    let stellars: Vec<StellarIff> = view.radar_shown().stellars.iter().map(|s| s.iff).collect();
    assert_eq!(stellars, [StellarIff::Cleared, StellarIff::Cleared]);
    let list = drawn(&view);
    let black = list.iter().any(|command| {
        matches!(command, DrawCommand::Line { color, .. } if *color == Color::rgba(0, 0, 0, 255))
    });
    assert!(black, "the black fill is drawn");
    for without in [outfitted(catalog(), &[]), outfitted(catalog(), &[(301, 1)])] {
        let mut view = FlightView::new(without);
        ticks(&mut view, 1);
        assert!(!view.radar_shown().iff);
        assert_eq!(radar_marks(&view).last(), Some(&PLAYER_PIXEL));
        assert!(
            !radar_marks(&view)
                .iter()
                .any(|m| matches!(m, RadarMark::Fill { .. }))
        );
    }
}

#[test]
fn a_stellar_of_a_landing_blessed_government_is_yellow_on_the_radar() {
    // `_ColorCodeStellar` @0xaab9: the government's landing blessing
    // gives yellow whatever `MinStatus` says.
    let earth = |on_start: &str| {
        let base = outfitted(starting_with(on_start), &[(300, 1)]);
        let mut sites = base.sites.clone();
        sites[0].govt = Some(GovtId(140));
        sites[0].min_status = 10;
        let catalog = FakeCatalog {
            ranks: vec![nova_sim::catalog::RankRecord {
                id: nova_sim::RankId(200),
                name: String::new(),
                weight: 0,
                affil_govt: Some(GovtId(140)),
                salary: 0,
                salary_cap: 0,
                contribute: 0,
                flags: nova_sim::rank::BLESSES_LANDING,
                conv_name: String::new(),
                short_name: String::new(),
                price_mod: 100,
            }],
            sites,
            ..base
        };
        let mut view = flying_as(catalog, "Ada");
        // Begun, the ship sits at (50, 50), so Earth is drawn up and
        // to the left of the radar's centre.
        view.begin();
        ticks(&mut view, 1);
        radar_marks(&view)
            .into_iter()
            .find(|mark| matches!(mark, RadarMark::Oval { rect, .. } if *rect == IRect::around(924, 75, 1)))
    };
    let oval = |color| RadarMark::Oval {
        rect: IRect::around(924, 75, 1),
        color,
    };
    assert_eq!(earth("K200"), Some(oval(RadarColor::Yellow)));
    assert_eq!(earth(""), Some(oval(RadarColor::Orange)), "unblessed");
}

/// [`outfitting`] selling [`radar_outfits`] too, landed.
fn landed_radar_outfitter() -> View {
    let mut catalog = outfitting();
    catalog.outfits.extend(radar_outfits());
    let mut view = FlightView::new(catalog);
    land_now(&mut view);
    view
}

#[test]
fn an_outfitter_order_redraws_the_radar_at_once_while_docked() {
    type Place = fn(&mut View, OutfitOrder);
    let order = |direction| OutfitOrder {
        outfit: OutfitId(300),
        direction,
    };
    let entries: [(&str, Place); 3] = [
        ("outfit", |view, order| {
            assert_eq!(view.outfit(order), Ok(()));
        }),
        ("counted", |view, order| {
            assert_eq!(view.outfit_counted(order, 1), Ok(1));
        }),
        ("order", |view, order| {
            assert_eq!(view.outfit_order(order, 1).run.done, 1);
        }),
    ];
    for (name, place) in entries {
        let mut view = landed_radar_outfitter();
        assert!(view.hud_waits(), "the engine's reading");
        assert!(!view.radar_shown().iff, "{name}: landing's");
        place(&mut view, order(Direction::Buy));
        assert!(view.radar_shown().iff, "{name}: bought");
        assert!(radar_filled(&view), "{name}: bought");
        assert!(radar_marks(&view).contains(&RadarMark::Pixel {
            x: 926,
            y: 96,
            color: RadarColor::Cyan
        }));
        place(&mut view, order(Direction::Sell));
        assert!(!view.radar_shown().iff, "{name}: sold");
        assert!(!radar_filled(&view), "{name}: sold");
        assert_eq!(radar_marks(&view).last(), Some(&PLAYER_PIXEL), "{name}");
    }
}

/// A trafficked flight with one idle NPC of ship 129, whose class
/// weighs `mass`, at (100, -100), radar pixel (929, 92); the player
/// carries `defaults`.
fn with_heavy_npc(mass: i16, defaults: &[(i16, u16)]) -> View {
    let (_, chance) = scripted(&placed(850, 650, 90));
    let catalog = FakeCatalog {
        hulls: vec![nova_sim::HullRecord {
            mass,
            ..hull_of(129, &[])
        }],
        ..outfitted(trafficked(&[130], 1, 129, 3), defaults)
    };
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    view
}

#[test]
fn a_heavy_npc_shows_as_a_square_only_with_a_density_scanner() {
    let square = RadarMark::Square {
        rect: IRect::around(929, 92, 1),
        color: RadarColor::Dim,
    };
    let dot = RadarMark::Pixel {
        x: 929,
        y: 92,
        color: RadarColor::Dim,
    };
    // The ships' marks: squares and pixels but the player's, last.
    let ships = |view: &View| {
        let mut marks: Vec<RadarMark> = radar_marks(view)
            .into_iter()
            .filter(|mark| matches!(mark, RadarMark::Pixel { .. } | RadarMark::Square { .. }))
            .collect();
        assert_eq!(marks.pop(), Some(PLAYER_PIXEL));
        marks
    };
    assert_eq!(ships(&with_heavy_npc(100, &[(301, 1)])), [square]);
    assert_eq!(ships(&with_heavy_npc(100, &[])), [dot]);
    assert_eq!(ships(&with_heavy_npc(99, &[(301, 1)])), [dot]);
}

#[test]
fn a_ships_iff_code_reaches_the_radar() {
    let view = with_heavy_npc(0, &[(300, 1)]);
    let ships = &view.radar_shown().ships;
    assert_eq!(ships.len(), 1);
    assert_eq!(ships[0].iff, ShipIff::Other);
    assert!(!ships[0].heavy);
    assert!(radar_marks(&view).contains(&RadarMark::Pixel {
        x: 929,
        y: 92,
        color: RadarColor::Blue
    }));
}

#[test]
fn a_plotted_course_hides_the_arrow_until_a_stellar_is_selected() {
    let mut view = flight();
    assert!(!view.radar_now().hyperspace_mode, "no course");
    plot(&mut view, 131);
    assert!(view.radar_now().hyperspace_mode);
    assert!(
        view.radar_shown().hyperspace_mode,
        "redrawn as the map closes"
    );
    tap_slot(&mut view, NEAREST_STELLAR_KEY);
    assert!(view.session().expect("flying").nav_target().is_some());
    assert!(!view.radar_now().hyperspace_mode, "a stellar selected");
}

fn arrow_lines(view: &View) -> usize {
    radar_marks(view)
        .iter()
        .filter(|mark| matches!(mark, RadarMark::Line { .. }))
        .count()
}

#[test]
fn far_from_the_centre_the_arrow_blinks_with_the_target() {
    let mut view = flight();
    assert_eq!(arrow_lines(&view), 0, "near the centre");
    fly_far(&mut view);
    let mut shown = Vec::new();
    for _ in 0..32 {
        ticks(&mut view, 1);
        shown.push((view.hud_cadence.blink_on(), arrow_lines(&view)));
    }
    for &(blink_on, lines) in &shown {
        assert_eq!(lines, if blink_on { 3 } else { 0 }, "{shown:?}");
    }
    let flips = shown
        .windows(2)
        .filter(|pair| pair[0].0 != pair[1].0)
        .count();
    assert_eq!(flips, 4, "every 8 ticks: {shown:?}");
}

/// `catalog` with Sol's `Interference` at `level`, its effects rolled
/// on `draws`.
fn interfered(level: i16, draws: &[u32]) -> (Rc<RefCell<Script>>, View) {
    let (script, chance) = scripted(draws);
    let catalog = FakeCatalog {
        sol_interference: level,
        ..catalog()
    };
    (script, FlightView::new(catalog).with_effects_chance(chance))
}

#[test]
fn interference_shows_static_at_the_next_refresh() {
    // The threshold roll 0 (1 ≤ 100), then pattern 3.
    let (script, mut view) = interfered(100, &[0, 3]);
    assert_eq!(view.radar_shown().static_pattern, None, "before a tick");
    ticks(&mut view, 1);
    assert_eq!(script.borrow().asked, [100, 10]);
    assert_eq!(view.radar_shown().static_pattern, Some(131));
    assert_eq!(
        radar_marks(&view)[0],
        RadarMark::Pattern {
            rect: STOCK_RADAR,
            id: 131
        }
    );
    let list = drawn(&view);
    assert!(list.iter().any(|command| *command
        == DrawCommand::TiledPicture {
            image: ImageKey::pattern(131),
            area: STOCK_RADAR.to_bounds(),
            origin: Point::new(0.0, 0.0),
        }));
}

#[test]
fn without_interference_no_static_shows_and_nothing_is_rolled() {
    let (script, mut view) = interfered(0, &[]);
    for _ in 0..20 {
        ticks(&mut view, 1);
        assert_eq!(view.radar_shown().static_pattern, None);
    }
    assert!(script.borrow().asked.is_empty());
}

#[test]
fn anti_interference_lowers_the_level_static_is_rolled_against() {
    // 100 less 30: a roll of 70 (71 > 70) shows none.
    let (script, chance) = scripted(&[70]);
    let catalog = FakeCatalog {
        sol_interference: 100,
        ..outfitted(catalog(), &[(302, 1)])
    };
    let mut view = FlightView::new(catalog).with_effects_chance(chance);
    ticks(&mut view, 1);
    assert_eq!(script.borrow().asked, [100]);
    assert_eq!(view.radar_shown().static_pattern, None);
}

#[test]
fn the_hud_cadence_and_blink_advance_through_a_jumps_warp() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    let streaking = |view: &View| {
        matches!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::Warp)
        )
    };
    assert!(streaking(&view));
    // Mark the snapshot, then tick through the streak: the radar is
    // taken afresh, and the blink flips, every 8 ticks.
    let marker = RadarScene {
        player: at(-1.0, -1.0),
        ..RadarScene::default()
    };
    let mut refreshes = Vec::new();
    let mut tick = 0;
    while streaking(&view) {
        view.radar_scene = Some(marker.clone());
        let blink = view.hud_cadence.blink_on();
        view.tick(TICK);
        tick += 1;
        let retaken = view.radar_scene.as_ref() != Some(&marker);
        assert_eq!(retaken, view.hud_cadence.blink_on() != blink, "tick {tick}");
        if retaken {
            refreshes.push(tick);
        }
    }
    assert!(tick >= 24, "the streak lasts a second: {tick}");
    assert!(refreshes.len() >= 3, "{refreshes:?}");
    let gaps: Vec<u32> = refreshes.windows(2).map(|pair| pair[1] - pair[0]).collect();
    assert!(gaps.iter().all(|&gap| gap == 8), "{refreshes:?}");
}

// The Red Alert.

/// The Red Alert `cïcn`s drawn, and where.
fn icons_drawn(view: &View) -> Vec<(i16, Point)> {
    drawn(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Picture { image, top_left } if icon_in(18000..=18001)(command) => {
                Some((image.id, *top_left))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_red_alert_draws_cicn_18000_then_18001_at_the_playfields_top_right() {
    let mut view = threatened(QUIET);
    for tick in 1..60 {
        view.tick(TICK);
        assert_eq!(icons_drawn(&view), [], "tick {tick}: before the check");
    }
    let corner = Point::new(1024.0 - 194.0 - 24.0, 0.0);
    for tick in 0..160_usize {
        view.tick(TICK);
        let expected = match tick {
            146.. => vec![],
            1.. if ((tick - 1) / 5).is_multiple_of(2) => vec![(18001, corner)],
            _ => vec![(18000, corner)],
        };
        assert_eq!(icons_drawn(&view), expected, "tick {tick} of the blink");
    }
}

#[test]
fn no_alert_icon_is_drawn_for_a_loud_player() {
    let mut view = threatened(SoundPrefs::default());
    for tick in 0..300 {
        view.tick(TICK);
        assert_eq!(icons_drawn(&view), [], "tick {tick}");
    }
    assert!(!view.alert_quiet());
}

#[test]
fn the_sound_prefs_reach_the_session_when_set() {
    let mut view = threatened(SoundPrefs::default());
    view.set_sound_prefs(QUIET);
    assert!(view.alert_quiet());
    ticks(&mut view, 60);
    assert_eq!(icons_drawn(&view).len(), 1);
    view.set_sound_prefs(SoundPrefs {
        sound: false,
        ..SoundPrefs::default()
    });
    assert!(view.alert_quiet(), "sound off is quiet");
    view.set_sound_prefs(SoundPrefs::default());
    assert!(!view.alert_quiet());
}

#[test]
fn the_red_alert_sounds_once_through_the_view() {
    let mut view = threatened(QUIET);
    view.take_sounds();
    let mut alerts = 0;
    for _ in 0..300 {
        view.tick(TICK);
        alerts += view
            .take_sounds()
            .iter()
            .filter(|&&sound| sound == Sound::Sim(SimSound::RedAlert))
            .count();
    }
    assert_eq!(alerts, 1);
}

#[test]
fn the_alert_icon_is_drawn_over_the_brackets() {
    let mut view = threatened(QUIET);
    ticks(&mut view, 1);
    tap(&mut view, NEAREST_KEY);
    assert_eq!(target_of(&view), Some(NpcId(0)));
    ticks(&mut view, 59);
    let list = drawn(&view);
    let commands: Vec<_> = list.iter().collect();
    let icon = commands
        .iter()
        .position(|command| icon_in(18000..=18001)(command))
        .expect("the icon is drawn");
    assert!(
        icon_in(10011..=10011)(commands[icon - 1]),
        "the brackets' bottom-left corner just before it: {:?}",
        commands[icon - 1]
    );
}

#[test]
fn a_missing_alert_icon_draws_nothing_and_says_so_once() {
    let catalog = FakeCatalog {
        alert_icon: None,
        ..armed(2, &[])
    };
    let (_, chance) = scripted(&two_npcs());
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Threatening))
        .with_sound_prefs(QUIET);
    let missing =
        Diagnostic::Unreadable("no cïcn 18000: the Red Alert icon is not drawn".to_owned());
    let told = |view: &mut View| {
        let diagnostics = view.take_diagnostics();
        diagnostics.iter().filter(|&d| *d == missing).count()
    };
    assert_eq!(told(&mut view), 1);
    let mut blinked = false;
    for _ in 0..200 {
        view.tick(TICK);
        blinked |= view.session().expect("flying").red_alert_icon().is_some();
        assert_eq!(icons_drawn(&view), []);
    }
    assert!(blinked, "the session blinked it");
    assert_eq!(told(&mut view), 0, "once");
    assert_eq!(told(&mut flight()), 0, "the default catalog has it");
}

// On a scripted flight: what the nav keys send, and what the nav area
// shows of the frame.

/// The nav mode in Sol `mode`.
fn sol_in(mode: NavMode) -> FlightFrame {
    FlightFrame {
        nav_mode: mode,
        ..in_sol()
    }
}

/// Presses `k` as flight's input does for a key with no arm of its own
/// ([`FlightView::nav_key`]), then lets it go.
fn faked_tap(view: &mut Faked, k: Key) {
    view.nav_key(k);
    view.track_held(&key(k, true));
    view.track_held(&key(k, false));
}

/// [`faked_tap`], then the frame whose slot block takes it.
fn faked_tap_slot(view: &mut Faked, k: Key) {
    faked_tap(view, k);
    view.nav_held_frame();
}

/// Presses `k` and holds it, as flight's input does for a key with no arm
/// of its own.
fn faked_hold(view: &mut Faked, k: Key) {
    view.nav_key(k);
    view.track_held(&key(k, true));
}

/// The stellar picks `view` sent.
fn picks(view: &Faked) -> Vec<StellarPick> {
    sent(view)
        .iter()
        .filter_map(|command| match command {
            FlightCommand::SelectStellar(pick) => Some(*pick),
            _ => None,
        })
        .collect()
}

#[test]
fn the_slot_keys_are_1_to_4_and_f1_to_f4_in_nav_order() {
    assert_eq!(
        STELLAR_SLOT_KEYS,
        [
            [Key::Char('1'), Key::Function(1)],
            [Key::Char('2'), Key::Function(2)],
            [Key::Char('3'), Key::Function(3)],
            [Key::Char('4'), Key::Function(4)],
        ]
    );
    for (slot, keys) in STELLAR_SLOT_KEYS.iter().enumerate() {
        for &slot_key in keys {
            let mut view = faked(in_sol());
            faked_tap_slot(&mut view, slot_key);
            assert_eq!(picks(&view), [StellarPick::Slot(slot)], "{slot_key:?}");
        }
    }
}

#[test]
fn a_slot_key_picks_on_the_slot_blocks_next_frame() {
    let mut view = faked(in_sol());
    faked_tap(&mut view, STELLAR_SLOT_KEYS[1][0]);
    assert_eq!(sent(&view), [], "the press waits for the frame");
    view.nav_held_frame();
    assert_eq!(
        sent(&view),
        [FlightCommand::SelectStellar(StellarPick::Slot(1))]
    );
}

#[test]
fn a_slot_key_repeat_or_release_sends_nothing() {
    let mut view = faked(in_sol());
    view.track_held(&held(Key::Char('2')));
    view.track_held(&key(Key::Char('2'), false));
    view.nav_held_frame();
    assert_eq!(sent(&view), []);
}

#[test]
fn f5_picks_the_nearest_stellar() {
    let mut view = faked(in_sol());
    faked_tap_slot(&mut view, NEAREST_STELLAR_KEY);
    assert_eq!(picks(&view), [StellarPick::Nearest]);
    assert_eq!(NEAREST_STELLAR_KEY, Key::Function(5));
}

#[test]
fn no_slot_key_picks_while_the_flight_refuses_the_nav_keys_or_during_a_jump() {
    let mut view = faked(in_sol());
    view.sim.refuse(PlayerCommand::Nav);
    faked_tap_slot(&mut view, STELLAR_SLOT_KEYS[0][0]);
    assert_eq!(sent(&view), [], "refused");
    let mut view = faked(FlightFrame {
        jump_under_way: true,
        ..in_sol()
    });
    faked_tap_slot(&mut view, STELLAR_SLOT_KEYS[0][0]);
    assert_eq!(sent(&view), [], "during a jump");
}

/// Presses `k` with Command held, as [`command_tap`] does, then runs the
/// slot block's frame.
fn faked_command_tap_slot(view: &mut Faked, k: Key) {
    view.track_held(&key(Key::Command, true));
    faked_tap(view, k);
    view.track_held(&key(Key::Command, false));
    view.nav_held_frame();
}

#[test]
fn command_5_picks_the_nearest_as_f5_does() {
    let mut view = faked(in_sol());
    faked_command_tap_slot(&mut view, NEAREST_STELLAR_COMMAND_KEY);
    assert_eq!(picks(&view), [StellarPick::Nearest]);
    assert_eq!(NEAREST_STELLAR_COMMAND_KEY, Key::Char('5'));
}

#[test]
fn five_without_command_picks_nothing() {
    let mut view = faked(in_sol());
    faked_tap_slot(&mut view, NEAREST_STELLAR_COMMAND_KEY);
    assert_eq!(sent(&view), []);
}

#[test]
fn command_5_held_then_let_go_of_with_5_still_down_picks_once() {
    let mut view = faked(in_sol());
    view.track_held(&key(Key::Command, true));
    faked_hold(&mut view, NEAREST_STELLAR_COMMAND_KEY);
    for _ in 0..3 {
        view.nav_held_frame();
    }
    assert_eq!(picks(&view), [StellarPick::Nearest]);
    view.track_held(&key(Key::Command, false));
    view.nav_held_frame();
    faked_tap_slot(&mut view, STELLAR_SLOT_KEYS[1][1]);
    assert_eq!(
        picks(&view),
        [StellarPick::Nearest, StellarPick::Slot(1)],
        "5 alone holds nothing"
    );
}

#[test]
fn a_slot_key_pressed_while_another_is_held_picks_nothing() {
    let mut view = faked(in_sol());
    faked_hold(&mut view, Key::Function(1));
    view.nav_held_frame();
    faked_tap_slot(&mut view, STELLAR_SLOT_KEYS[1][0]);
    faked_tap_slot(&mut view, NEAREST_STELLAR_KEY);
    assert_eq!(picks(&view), [StellarPick::Slot(0)], "F1 holds the latch");
    view.track_held(&key(Key::Function(1), false));
    faked_tap_slot(&mut view, STELLAR_SLOT_KEYS[1][0]);
    assert_eq!(
        picks(&view),
        [StellarPick::Slot(0), StellarPick::Slot(1)],
        "let go, 2 acts"
    );
}

#[test]
fn a_slot_key_held_picks_once() {
    let mut view = faked(in_sol());
    faked_hold(&mut view, STELLAR_SLOT_KEYS[1][0]);
    for _ in 0..6 {
        view.nav_held_frame();
    }
    assert_eq!(picks(&view), [StellarPick::Slot(1)], "one pick a hold");
}

#[test]
fn slot_keys_pressed_in_one_frame_make_one_pick() {
    let mut view = faked(in_sol());
    faked_tap(&mut view, STELLAR_SLOT_KEYS[1][1]);
    faked_tap(&mut view, STELLAR_SLOT_KEYS[2][0]);
    view.nav_held_frame();
    assert_eq!(picks(&view), [StellarPick::Slot(1)], "F2 over 3");
    faked_tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
    faked_tap(&mut view, STELLAR_SLOT_KEYS[2][0]);
    view.nav_held_frame();
    faked_tap(&mut view, STELLAR_SLOT_KEYS[2][1]);
    faked_tap(&mut view, NEAREST_STELLAR_KEY);
    view.nav_held_frame();
    assert_eq!(
        picks(&view),
        [
            StellarPick::Slot(1),
            StellarPick::Slot(2),
            StellarPick::Nearest
        ],
        "3 over 1, then F5 over F3"
    );
}

#[test]
fn nav_off_sends_the_blocks_nav_off_press() {
    let mut view = faked(sol_in(NavMode::Stellar(Some(StellarId(129)))));
    faked_tap(&mut view, NAV_OFF_KEY);
    assert_eq!(
        sent(&view),
        [FlightCommand::NavModeKeys {
            nav_off: true,
            h: false
        }]
    );
    assert_eq!(NAV_OFF_KEY, Key::Char('`'));
}

#[test]
fn hyperspace_mode_sends_the_blocks_h_press_and_shows_the_floating_map() {
    let mut view = faked(in_sol());
    view.hyperspace_key(HYPERSPACE_MODE_KEY);
    assert_eq!(
        sent(&view),
        [FlightCommand::NavModeKeys {
            nav_off: false,
            h: true
        }]
    );
    assert!(view.floating.is_up());
    assert_eq!(HYPERSPACE_MODE_KEY, Key::Char('h'));
}

#[test]
fn hyper_select_sends_the_next_system_and_shows_the_floating_map_only_in_mode_3() {
    let mut view = faked(in_sol());
    view.hyperspace_key(HYPER_SELECT_KEY);
    assert_eq!(sent(&view), [FlightCommand::SelectNextSystem]);
    assert!(!view.floating.is_up(), "the frame's nav is off");
    let mut view = faked(sol_in(NavMode::Hyperspace(None)));
    view.hyperspace_key(HYPER_SELECT_KEY);
    assert!(view.floating.is_up(), "mode 3");
}

#[test]
fn option_backquote_lets_go_of_the_target_and_sends_no_block_press() {
    let mut view = faked(sol_in(NavMode::Stellar(Some(StellarId(129)))));
    view.track_held(&key(Key::Alt, true));
    faked_tap(&mut view, NAV_OFF_KEY);
    assert_eq!(sent(&view), [FlightCommand::ClearTarget]);
}

#[test]
fn option_backquote_lets_go_of_the_target_during_a_jump_too() {
    let mut view = faked(FlightFrame {
        jump_under_way: true,
        ..in_sol()
    });
    view.track_held(&key(Key::Alt, true));
    faked_tap(&mut view, NAV_OFF_KEY);
    view.track_held(&key(Key::Alt, false));
    faked_tap(&mut view, NAV_OFF_KEY);
    assert_eq!(sent(&view), [FlightCommand::ClearTarget], "no block press");
}

#[test]
fn with_the_nav_off_the_nav_area_says_nav_system_off() {
    assert_eq!(faked(in_sol()).nav_now(), NavDisplay::Off);
}

#[test]
fn a_stellar_the_scene_doesnt_hold_is_named_as_the_flight_names_it_or_not_at_all() {
    let mut view = faked(sol_in(NavMode::Stellar(Some(StellarId(150)))));
    assert_eq!(view.nav_now(), NavDisplay::Stellar(None), "no name");
    view.sim.map_mut().stellar_names = vec![(StellarId(150), "Vega Prime".to_owned())];
    assert_eq!(
        view.nav_now(),
        NavDisplay::Stellar(Some("Vega Prime".to_owned()))
    );
}

#[test]
fn the_nav_area_names_a_stellar_by_its_name_in_the_scene() {
    let mut view = faked(sol_in(NavMode::Stellar(Some(StellarId(129)))));
    let shown = [StellarId(128), StellarId(129)].into_iter().collect();
    view.scene = Some(SystemScene::load_shown(
        &*view.catalog,
        SystemId(130),
        &shown,
    ));
    view.sim.map_mut().stellar_names = vec![(StellarId(129), "Luna".to_owned())];
    assert_eq!(
        view.nav_now(),
        NavDisplay::Stellar(Some("Moon".to_owned())),
        "the scene's name first"
    );
}

#[test]
fn a_hyperspace_selection_is_named_once_explored_with_the_frames_readiness() {
    let mut view = faked(FlightFrame {
        jump_readiness: nova_sim::JumpReadiness::Blocked,
        ..sol_in(NavMode::Hyperspace(Some(SystemId(131))))
    });
    assert_eq!(
        view.nav_now(),
        NavDisplay::Hyperspace {
            destination: hud::NavDestination::Unexplored,
            readiness: nova_sim::JumpReadiness::Blocked,
        }
    );
    view.sim.map_mut().explored = vec![SystemId(131)];
    view.sim.set_frame(FlightFrame {
        jump_readiness: nova_sim::JumpReadiness::Underway,
        ..sol_in(NavMode::Hyperspace(Some(SystemId(131))))
    });
    assert_eq!(
        view.nav_now(),
        NavDisplay::Hyperspace {
            destination: hud::NavDestination::Named("Alpha Centauri".to_owned()),
            readiness: nova_sim::JumpReadiness::Underway,
        }
    );
}

#[test]
fn hyperspace_with_nothing_selected_is_no_destination() {
    let view = faked(FlightFrame {
        jump_readiness: nova_sim::JumpReadiness::Blocked,
        ..sol_in(NavMode::Hyperspace(None))
    });
    assert_eq!(
        view.nav_now(),
        NavDisplay::Hyperspace {
            destination: hud::NavDestination::None,
            readiness: nova_sim::JumpReadiness::Blocked,
        }
    );
}

#[test]
fn retaking_the_nav_area_notes_the_frames_text_count() {
    let mut view = faked(FlightFrame {
        texts_queued: 3,
        ..sol_in(NavMode::Stellar(None))
    });
    view.nav_marked = true;
    view.retake_nav();
    assert_eq!(view.nav_texts, 3);
    assert!(!view.nav_marked);
    assert_eq!(
        view.nav_scene,
        Some((NavMode::Stellar(None), NavDisplay::Stellar(None)))
    );
}

/// The commands a frame of held keys sends: `keys` held, `refused` the
/// one gate the flight refuses.
fn held_frame_sends(keys: &[Key], refused: PlayerCommand) -> Vec<FlightCommand> {
    let mut view = faked(in_sol());
    view.sim.refuse(refused);
    for &k in keys {
        view.track_held(&key(k, true));
    }
    view.nav_held_frame();
    sent(&view).to_vec()
}

#[test]
fn a_held_nav_block_acts_only_while_the_flight_takes_the_nav_keys() {
    let block = [NAV_OFF_KEY, HYPERSPACE_MODE_KEY];
    assert_eq!(held_frame_sends(&block, PlayerCommand::Nav), []);
    assert_eq!(
        held_frame_sends(&block, PlayerCommand::SelectSystem),
        [FlightCommand::NavModeKeys {
            nav_off: true,
            h: true
        }]
    );
    let option_backquote = [Key::Alt, NAV_OFF_KEY];
    assert_eq!(held_frame_sends(&option_backquote, PlayerCommand::Nav), []);
    assert_eq!(
        held_frame_sends(&option_backquote, PlayerCommand::SelectSystem),
        [FlightCommand::ClearTarget]
    );
}

#[test]
fn a_held_hyper_select_acts_only_while_the_flight_takes_it() {
    let select = [HYPER_SELECT_KEY];
    assert_eq!(held_frame_sends(&select, PlayerCommand::SelectSystem), []);
    assert_eq!(
        held_frame_sends(&select, PlayerCommand::Nav),
        [FlightCommand::SelectNextSystem]
    );
}
