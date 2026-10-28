use super::faked_hud::{
    among, barred, barred_by, cargo_texts, changes, flown, hud_tick, nav_of, nav_texts, targeted,
};
use super::faked_world::{in_sol_with, npc_129, the_player_at, world};
use super::*;
use nova_sim::flight_frame::FrameShip;
use nova_sim::status_cargo::StatusCargo;
use nova_sim::{AlertIcon, JumpReadiness};

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
    // A plot's mode 3 and Nav Off's mode -1 are the session's
    // (`plotting_a_course_selects_its_first_hop_and_picking_a_stellar_keeps_the_course`,
    // `nav_off_with_a_course_turns_the_nav_off_keeps_the_course_and_refuses_the_jump`).
    let mut frame = in_sol_with(vec![the_player_at(Vec2::new(0.0, -3000.0), 0.0)]);
    frame.nav_mode = NavMode::Hyperspace(Some(SystemId(131)));
    let mut view = barred(catalog(), frame);
    let most_lines = |view: &mut Faked| {
        (0..32)
            .map(|_| {
                hud_tick(view);
                faked_arrow_lines(view)
            })
            .max()
    };
    assert_eq!(most_lines(&mut view), Some(0), "mode 3 hides it");
    changes(&mut view, |frame| frame.nav_mode = NavMode::Off);
    assert_eq!(most_lines(&mut view), Some(3), "mode -1 shows it");
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
    view: &mut Faked,
    nav: Option<nova_sim::brackets::NavBrackets>,
) -> Vec<(i16, Point)> {
    let hud = nova_sim::flight_frame::FrameHud {
        nav_brackets: nav,
        ..nova_sim::flight_frame::FrameHud::default()
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
    let mut view = faked_sol_scene();
    assert_eq!(faked_nav_icons(&mut view, None), []);
    let moon = faked_on_screen(&view, (300.0, -200.0));
    assert_eq!(
        faked_nav_icons(&mut view, Some(nav_at(129, (300.0, -200.0), 196.0))),
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
    let mut view = super::faked_world::world(
        FakeCatalog {
            sol_more: vec![unreadable],
            ..catalog()
        },
        frame,
    );
    let broken = faked_on_screen(&view, (100.0, 0.0));
    assert_eq!(
        faked_nav_icons(&mut view, Some(nav_at(131, (100.0, 0.0), 0.0))),
        corners_at(10000, broken, 64.0)
    );
}

/// A selection the scene doesn't hold is bracketed at its spöb position
/// at half 64 (`_HandleNavTargetSprites` @0x2fd9f-0x2fdda, @0x2fe41).
/// A selection the scene doesn't hold is bracketed at its spöb position
/// at half 64 (`_HandleNavTargetSprites` @0x2fd9f-0x2fdda, @0x2fe41).
#[test]
fn an_unseen_selection_is_bracketed_at_its_spot_at_half_64() {
    let mut view = faked_sol_scene();
    let hole = faked_on_screen(&view, (0.0, 300.0));
    assert_eq!(
        faked_nav_icons(&mut view, Some(nav_at(150, (0.0, 300.0), 196.0))),
        corners_at(10000, hole, 64.0 + 196.0),
        "flying in"
    );
    assert_eq!(
        faked_nav_icons(&mut view, Some(nav_at(150, (0.0, 300.0), 0.0))),
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
    let mut view = faked_sol_scene();
    let earth = faked_on_screen(&view, (0.0, -600.0));
    let dominated = Some(nova_sim::brackets::NavBrackets {
        dominated: true,
        ..nav_at(128, (0.0, -600.0), 0.0)
    });
    assert_eq!(
        faked_nav_icons(&mut view, dominated),
        corners_at(10004, earth, 10.0)
    );
}

/// `text` from `x` right of the stock cargo area's left, (8, 458) at
/// (830, 0), on `baseline` below its top, in Geneva 12.
fn cargo_line(text: &str, x: f32, baseline: f32) -> (String, Point) {
    (text.to_owned(), at(838.0 + x, 458.0 + baseline - 12.0))
}

#[test]
fn the_hud_lists_the_goods_bought_the_free_space_and_the_cash() {
    // What the hold holds and what is free after a trade is nova-sim's
    // (`buying_pays_loads_and_makes_a_save_due`, `buying_stops_when_the_hold_is_full`,
    // `the_cargo_area_reads_the_hold_the_missions_the_junk_and_the_cash`).
    let mut frame = among(Vec::new());
    frame.hud.cargo = StatusCargo {
        free: 10,
        cash: 1000,
        ..StatusCargo::default()
    };
    let mut view = barred(catalog(), frame);
    assert_eq!(
        cargo_texts(&flown(&view)),
        [
            cargo_line(hud::CARGO_FREE, 77.0, 12.0),
            cargo_line("10", 110.0, 12.0),
            cargo_line("Credits:", 77.0, 66.0),
            cargo_line("1,000", 87.0, 82.0),
        ],
        "an empty 10-ton hold"
    );
    changes(&mut view, |frame| {
        frame.hud.cargo.rows[0] = Some(nova_sim::status_cargo::CommodityRow {
            label: "Food:".to_owned(),
            tons: 10,
        });
        frame.hud.cargo.free = 0;
        frame.hud.cargo.cash = 250;
    });
    assert_eq!(
        cargo_texts(&flown(&view)),
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
    let mut frame = among(Vec::new());
    frame.hud.cargo = StatusCargo {
        free: 10,
        cash: 1000,
        ..StatusCargo::default()
    };
    let view = barred(taller, frame);
    let area = view.status_bar.as_ref().expect("a bar").layout.cargo;
    let area = area.offset(at(830.0, 0.0));
    let texts: Vec<String> = texts(&flown(&view))
        .into_iter()
        .zip(flown(&view).iter().filter_map(|command| match command {
            DrawCommand::Text { origin, .. } => Some(*origin),
            _ => None,
        }))
        .filter(|(_, origin)| area.contains(*origin))
        .map(|(text, _)| text)
        .collect();
    assert_eq!(
        texts,
        [hud::CARGO_FREE, "10", "Credits:", "1,000"],
        "the cargo lines alone, no date"
    );
}

#[test]
fn the_nav_area_is_centred_by_the_metrics_given() {
    use crate::text::TextMetrics as _;
    use crate::text::fixture::MonoMetrics;
    let mut view = barred(catalog(), among(Vec::new()));
    view.metrics = Some(Metrics(Rc::new(MonoMetrics)));
    let origin = flown(&view)
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
    // Where the ship is clear to jump is nova-sim's
    // (`a_plotted_course_is_blocked_until_the_ship_is_out_and_clear_from_then`).
    // Read the Bible's way, the nav area follows every frame.
    let mut view = faked_hyperspace(JumpReadiness::Blocked, &bible_cadence());
    let hyperspace = |value: Color| {
        vec![
            (hud::NAV_HYPERSPACE.to_owned(), Color::DIM),
            (hud::NAV_UNEXPLORED.to_owned(), value),
        ]
    };
    assert_eq!(nav_texts(&flown(&view)), hyperspace(Color::DIM));
    changes(&mut view, |frame| {
        frame.jump_readiness = JumpReadiness::Clear;
    });
    assert_eq!(nav_texts(&flown(&view)), hyperspace(Color::WHITE));
    changes(&mut view, |frame| {
        frame.jump_readiness = JumpReadiness::Blocked;
    });
    assert_eq!(nav_texts(&flown(&view)), hyperspace(Color::DIM));
}

#[test]
fn once_j_is_accepted_the_hyperspace_label_turns_bright_too() {
    // That an accepted J leaves the jump under way is nova-sim's
    // (`once_j_is_accepted_the_jump_is_underway_until_arrival`).
    let view = faked_hyperspace(JumpReadiness::Underway, &bible_cadence());
    assert_eq!(
        nav_texts(&flown(&view)),
        [
            (hud::NAV_HYPERSPACE.to_owned(), Color::WHITE),
            (hud::NAV_UNEXPLORED.to_owned(), Color::WHITE),
        ]
    );
}

#[test]
fn the_nav_area_holds_its_colours_until_its_next_refresh() {
    let mut view = faked_hyperspace(JumpReadiness::Blocked, &Rulebook::default());
    let value = |view: &Faked| nav_texts(&flown(view))[1].1;
    assert_eq!(value(&view), Color::DIM, "inside, at once");
    while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
        hud_tick(&mut view);
    }
    changes(&mut view, |frame| {
        frame.jump_readiness = JumpReadiness::Clear;
    });
    for _ in 1..NAV_PERIOD_TICKS {
        assert_eq!(value(&view), Color::DIM, "held, tick {}", now_tick(&view));
        hud_tick(&mut view);
    }
    assert_eq!(now_tick(&view) % NAV_PERIOD_TICKS, 0);
    assert_eq!(value(&view), Color::WHITE, "at the refresh");
}

#[test]
fn the_nav_area_holds_its_text_until_its_next_refresh() {
    // That X131 explores Alpha Centauri is nova-sim's
    // (`exploring_a_head_explores_only_itself`).
    let mut view = faked_hyperspace(JumpReadiness::Blocked, &Rulebook::default());
    while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
        hud_tick(&mut view);
    }
    view.sim.map_mut().explored = vec![SystemId(131)];
    assert_eq!(nav_of(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
    for _ in 1..NAV_PERIOD_TICKS - 1 {
        hud_tick(&mut view);
        assert_eq!(
            nav_of(&view),
            [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED],
            "tick {}",
            now_tick(&view)
        );
    }
    hud_tick(&mut view);
    assert_eq!(now_tick(&view) % NAV_PERIOD_TICKS, 0);
    assert_eq!(nav_of(&view), [hud::NAV_HYPERSPACE, "Alpha Centauri"]);
}

#[test]
fn read_the_bibles_way_the_nav_area_follows_every_frame() {
    let mut view = faked_hyperspace(JumpReadiness::Blocked, &bible_cadence());
    while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
        hud_tick(&mut view);
    }
    view.sim.map_mut().explored = vec![SystemId(131)];
    assert_eq!(nav_of(&view), [hud::NAV_HYPERSPACE, "Alpha Centauri"]);
}

#[test]
fn a_selection_change_shows_at_once() {
    // What L and `\` select is nova-sim's
    // (`the_land_key_selects_the_nearest_in_silence_from_any_other_mode`,
    // `hyper_select_answers_the_system_it_selects`).
    let mut view = barred(catalog(), among(Vec::new()));
    while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
        hud_tick(&mut view);
    }
    changes(&mut view, |frame| {
        frame.nav_mode = NavMode::Stellar(Some(StellarId(129)));
    });
    assert_eq!(nav_of(&view), [hud::NAV_STELLAR, "Moon"], "drawn at once");
    hud_tick(&mut view);
    assert_eq!(nav_of(&view), [hud::NAV_STELLAR, "Moon"], "and kept");
    changes(&mut view, |frame| {
        frame.nav_mode = NavMode::Hyperspace(Some(SystemId(131)));
    });
    assert_eq!(nav_of(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
    hud_tick(&mut view);
    assert_eq!(nav_of(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
    assert_ne!(now_tick(&view) % NAV_PERIOD_TICKS, 0, "between refreshes");
}

#[test]
fn redrawing_the_status_retakes_the_nav_area() {
    let mut view = faked_hyperspace(JumpReadiness::Blocked, &Rulebook::default());
    while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
        hud_tick(&mut view);
    }
    view.sim.map_mut().explored = vec![SystemId(131)];
    assert_eq!(nav_of(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
    view.redraw_status();
    assert_eq!(nav_of(&view), [hud::NAV_HYPERSPACE, "Alpha Centauri"]);
}

#[test]
fn once_j_is_accepted_the_label_turns_bright_at_the_next_refresh() {
    let mut view = faked_hyperspace(JumpReadiness::Clear, &Rulebook::default());
    while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
        hud_tick(&mut view);
    }
    let label = |view: &Faked| nav_texts(&flown(view))[0].1;
    assert_eq!(label(&view), Color::DIM);
    changes(&mut view, |frame| {
        frame.jump_readiness = JumpReadiness::Underway;
    });
    assert_eq!(label(&view), Color::DIM, "held");
    for _ in 1..NAV_PERIOD_TICKS - 1 {
        hud_tick(&mut view);
        assert_eq!(label(&view), Color::DIM, "tick {}", now_tick(&view));
    }
    hud_tick(&mut view);
    assert_eq!(label(&view), Color::WHITE, "tick {}", now_tick(&view));
}

// The radar's IFF, density scanner, centre arrow and static.

#[test]
fn an_iff_outfit_turns_on_the_black_backdrop_and_the_cyan_player() {
    // The gear an outfit gives and the stellars' colours are nova-sim's
    // (`the_radar_gear_is_the_players_outfits`,
    // `a_stellar_of_a_landing_blessed_government_is_cleared_whatever_its_record`).
    let mut frame = among(Vec::new());
    frame.hud.radar_gear.iff = true;
    for stellar in &mut frame.stellars {
        stellar.iff = Some(StellarIff::Cleared);
    }
    let view = barred(catalog(), frame);
    assert!(view.radar_shown(&view.frame).iff);
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
    let stellars: Vec<StellarIff> = view
        .radar_shown(&view.frame)
        .stellars
        .iter()
        .map(|s| s.iff)
        .collect();
    assert_eq!(stellars, [StellarIff::Cleared, StellarIff::Cleared]);
    let list = flown(&view);
    let black = list.iter().any(|command| {
        matches!(command, DrawCommand::Line { color, .. } if *color == Color::rgba(0, 0, 0, 255))
    });
    assert!(black, "the black fill is drawn");
    let view = barred(catalog(), among(Vec::new()));
    assert!(!view.radar_shown(&view.frame).iff);
    assert_eq!(radar_marks(&view).last(), Some(&PLAYER_PIXEL));
    assert!(
        !radar_marks(&view)
            .iter()
            .any(|m| matches!(m, RadarMark::Fill { .. }))
    );
}

#[test]
fn a_stellar_of_a_landing_blessed_government_is_yellow_on_the_radar() {
    // A blessing's colour is nova-sim's
    // (`a_stellar_of_a_landing_blessed_government_is_cleared_whatever_its_record`);
    // the radar draws a cleared stellar yellow, a denied one orange.
    let earth = |iff| {
        let mut frame = among(Vec::new());
        frame.hud.radar_gear.iff = true;
        frame.stellars[0].iff = Some(iff);
        let view = barred(catalog(), frame);
        radar_marks(&view)
            .into_iter()
            .find(|mark| matches!(mark, RadarMark::Oval { rect, .. } if *rect == IRect::around(926, 77, 1)))
    };
    let oval = |color| RadarMark::Oval {
        rect: IRect::around(926, 77, 1),
        color,
    };
    assert_eq!(earth(StellarIff::Cleared), Some(oval(RadarColor::Yellow)));
    assert_eq!(
        earth(StellarIff::Denied),
        Some(oval(RadarColor::Orange)),
        "unblessed"
    );
}

#[test]
fn a_heavy_npc_shows_as_a_square_only_with_a_density_scanner() {
    // A ship's mass is its class's (`a_fill_writes_every_ship_the_player_first_then_each_npc_in_the_systems_order`).
    let square = RadarMark::Square {
        rect: IRect::around(929, 92, 1),
        color: RadarColor::Dim,
    };
    let dot = RadarMark::Pixel {
        x: 929,
        y: 92,
        color: RadarColor::Dim,
    };
    let ships = |mass: f32, scanner: bool| {
        let npc = FrameShip {
            mass,
            ..npc_129(0, Vec2::new(100.0, -100.0), 90.0)
        };
        let mut frame = among(vec![npc]);
        frame.hud.radar_gear.density_scanner = scanner;
        let view = barred(trafficked(&[130], 1, 129, 3), frame);
        let mut marks: Vec<RadarMark> = radar_marks(&view)
            .into_iter()
            .filter(|mark| matches!(mark, RadarMark::Pixel { .. } | RadarMark::Square { .. }))
            .collect();
        assert_eq!(marks.pop(), Some(PLAYER_PIXEL));
        marks
    };
    assert_eq!(ships(100.0, true), [square]);
    assert_eq!(ships(100.0, false), [dot]);
    assert_eq!(ships(99.0, true), [dot]);
}

#[test]
fn a_ships_iff_code_reaches_the_radar() {
    // Its code is nova-sim's (`a_ships_iff_is_its_colour_code`).
    let mut frame = among(vec![npc_129(0, Vec2::new(100.0, -100.0), 90.0)]);
    frame.hud.radar_gear.iff = true;
    let view = barred(trafficked(&[130], 1, 129, 3), frame);
    let ships = &view.radar_shown(&view.frame).ships;
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
    // A plot's mode 3 and F5's mode 2 are nova-sim's
    // (`plotting_a_course_selects_its_first_hop_and_picking_a_stellar_keeps_the_course`).
    let mut view = barred(catalog(), among(Vec::new()));
    assert!(!view.radar_now(&view.frame).hyperspace_mode, "no course");
    changes(&mut view, |frame| {
        frame.nav_mode = NavMode::Hyperspace(Some(SystemId(131)));
    });
    assert!(view.radar_now(&view.frame).hyperspace_mode);
    view.redraw_status();
    assert!(
        view.radar_shown(&view.frame).hyperspace_mode,
        "redrawn as the map closes"
    );
    changes(&mut view, |frame| {
        frame.nav_mode = NavMode::Stellar(Some(StellarId(128)));
    });
    assert!(
        !view.radar_now(&view.frame).hyperspace_mode,
        "a stellar selected"
    );
}

#[test]
fn far_from_the_centre_the_arrow_blinks_with_the_target() {
    assert_eq!(
        faked_arrow_lines(&barred(catalog(), among(Vec::new()))),
        0,
        "near the centre"
    );
    let far = in_sol_with(vec![the_player_at(Vec2::new(0.0, -3000.0), 0.0)]);
    let mut view = barred(catalog(), far);
    let mut shown = Vec::new();
    for _ in 0..32 {
        hud_tick(&mut view);
        shown.push((view.hud_cadence.blink_on(), faked_arrow_lines(&view)));
    }
    for &(blink_on, lines) in &shown {
        assert_eq!(lines, if blink_on { 3 } else { 0 }, "{shown:?}");
    }
    let flips: Vec<usize> = shown
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0].0 != pair[1].0)
        .map(|(tick, _)| tick)
        .collect();
    assert!(flips.len() >= 3, "{shown:?}");
    assert!(
        flips.windows(2).all(|pair| pair[1] - pair[0] == 8),
        "every 8 ticks: {shown:?}"
    );
}

#[test]
fn interference_shows_static_at_the_next_refresh() {
    // The threshold roll 0 (1 ≤ 100), then pattern 3.
    let (script, mut view) = faked_interfered(100, &[0, 3]);
    assert_eq!(
        view.radar_shown(&view.frame).static_pattern,
        None,
        "before a tick"
    );
    hud_tick(&mut view);
    assert_eq!(script.borrow().asked, [100, 10]);
    assert_eq!(view.radar_shown(&view.frame).static_pattern, Some(131));
    assert_eq!(
        radar_marks(&view)[0],
        RadarMark::Pattern {
            rect: STOCK_RADAR,
            id: 131
        }
    );
    let list = flown(&view);
    assert!(list.iter().any(|command| *command
        == DrawCommand::TiledPicture {
            image: ImageKey::pattern(131),
            area: STOCK_RADAR.to_bounds(),
            origin: Point::new(0.0, 0.0),
        }));
}

#[test]
fn without_interference_no_static_shows_and_nothing_is_rolled() {
    let (script, mut view) = faked_interfered(0, &[]);
    for _ in 0..20 {
        hud_tick(&mut view);
        assert_eq!(view.radar_shown(&view.frame).static_pattern, None);
    }
    assert!(script.borrow().asked.is_empty());
}

#[test]
fn anti_interference_lowers_the_level_static_is_rolled_against() {
    // 100 less 30 is nova-sim's
    // (`interference_is_the_systems_less_the_anti_interference`): at 70, a
    // roll of 70 (71 > 70) shows none.
    let (script, mut view) = faked_interfered(70, &[70]);
    hud_tick(&mut view);
    assert_eq!(script.borrow().asked, [100]);
    assert_eq!(view.radar_shown(&view.frame).static_pattern, None);
}

// The Red Alert.

/// The Red Alert `cïcn`s drawn, and where.
pub(super) fn icons_drawn(view: &FlightView<FakeCatalog, impl FlightSim>) -> Vec<(i16, Point)> {
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
    // When it blinks, and which frame on which tick, is nova-sim's
    // (`the_red_alert_icon_follows_the_pattern_when_sound_is_quiet`).
    let mut view = world(catalog(), among(Vec::new()));
    view.alert_width = Some(24);
    let corner = Point::new(1024.0 - 194.0 - 24.0, 0.0);
    for (icon, expected) in [
        (None, vec![]),
        (Some(AlertIcon::Red), vec![(18000, corner)]),
        (Some(AlertIcon::Dark), vec![(18001, corner)]),
    ] {
        changes(&mut view, |frame| frame.hud.red_alert = icon);
        assert_eq!(faked_icons_drawn(&view), expected, "{icon:?}");
    }
}

#[test]
fn no_alert_icon_is_drawn_for_a_loud_player() {
    // That a loud player's alert shows no icon is nova-sim's
    // (`a_loud_player_hears_the_alert_without_the_icon`).
    let mut view = world(catalog(), among(Vec::new()));
    view.alert_width = Some(24);
    assert_eq!(faked_icons_drawn(&view), [], "the frame holds none");
    assert!(!view.sound_prefs.alert_quiet(), "the default preferences");
}

#[test]
fn the_red_alert_sounds_once_through_the_view() {
    // That a threat sounds it once is nova-sim's (`staying_threatened_sounds_once`).
    let mut view = faked(in_sol());
    view.sim
        .queue_event(nova_sim::flight_frame::FlightEvent::Sound(
            SimSound::RedAlert,
        ));
    assert_eq!(view.drained_sounds(), [Sound::Sim(SimSound::RedAlert)]);
    assert_eq!(view.drained_sounds(), [], "once");
}

#[test]
fn the_alert_icon_is_drawn_over_the_brackets() {
    let mut frame = targeted(
        vec![npc_129(0, Vec2::new(100.0, -100.0), 90.0)],
        record::target(0),
    );
    frame.hud = super::faked_world::targeting(0);
    frame.hud.red_alert = Some(AlertIcon::Red);
    let mut view = world(trafficked(&[130], 1, 129, 3), frame);
    view.alert_width = Some(24);
    let list = super::faked_world::world_list(&view);
    let commands: Vec<_> = list.iter().collect();
    let icon = commands
        .iter()
        .position(|command| icon_in(18000..=18001)(command))
        .expect("the icon is drawn");
    assert!(
        icon_in(10015..=10015)(commands[icon - 1]),
        "the brackets' bottom-left corner just before it: {:?}",
        commands[icon - 1]
    );
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

/// A view in Sol by `rules`, its nav in mode 3 for Alpha Centauri, not
/// explored, the ship as `readiness` says.
fn faked_hyperspace(readiness: JumpReadiness, rules: &Rulebook) -> Faked {
    let mut frame = among(Vec::new());
    frame.nav_mode = NavMode::Hyperspace(Some(SystemId(131)));
    frame.jump_readiness = readiness;
    barred_by(catalog(), frame, rules)
}

/// The rules with the HUD following every frame, the Bible's reading.
fn bible_cadence() -> Rulebook {
    Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Bible)
}

/// The centre arrow's lines on `view`'s radar.
fn faked_arrow_lines(view: &Faked) -> usize {
    radar_marks(view)
        .iter()
        .filter(|mark| matches!(mark, RadarMark::Line { .. }))
        .count()
}

/// A view in Sol of interference `level`, its effects rolled on `draws`.
fn faked_interfered(level: u8, draws: &[u32]) -> (Rc<RefCell<Script>>, Faked) {
    let (script, chance) = scripted(draws);
    let mut frame = among(Vec::new());
    frame.hud.interference = level;
    let mut view = barred(catalog(), frame);
    view.effects_chance = chance;
    (script, view)
}

/// The Red Alert `cïcn`s `view`'s world draws, and where.
fn faked_icons_drawn(view: &Faked) -> Vec<(i16, Point)> {
    super::faked_world::world_list(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Picture { image, top_left } if icon_in(18000..=18001)(command) => {
                Some((image.id, *top_left))
            }
            _ => None,
        })
        .collect()
}
