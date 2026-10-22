//! Flight in a view widened to 1365 x 768, a 16:9 window's: the HUD at
//! the original's places for that width, the ship at the view's centre,
//! every visible-area rule measured against the widened view, and the
//! course map in its 1024-wide frame, centred.

use super::*;
use crate::flight::escorts::MENU_KEY;
use crate::flight::floating;
use crate::flight::{alert, brackets};
use crate::view_shape::ViewShape;

/// A 16:9 window's widened view.
const WIDE: (f32, f32) = (1365.0, 768.0);

/// `view` widened to [`WIDE`].
fn widened(mut view: View) -> View {
    view.set_view_size(WIDE);
    view
}

#[test]
fn flight_asks_to_widen_and_takes_the_widened_size() {
    let mut view = flight();
    assert_eq!(view.view_shape(), ViewShape::Widened);
    assert_eq!(view.view_size(), VIEW_SIZE, "classic until told");
    view.set_widened_size(WIDE);
    assert_eq!(view.view_size(), WIDE);
    assert_eq!(view.camera().view_size(), WIDE);
}

#[test]
fn the_status_bar_sits_against_the_widened_views_right_edge() {
    let view = widened(flight());
    let list = drawn(&view);
    assert!(
        list.iter().any(|command| *command
            == DrawCommand::Picture {
                image: ImageKey::picture(700),
                top_left: at(1171.0, 0.0),
            }),
        "the background PICT at 1365 − 194"
    );
    // Every area of the bar (the radar, the gauges, the nav and cargo
    // areas) moves with it, 1171 − 830 right of where the classic bar has
    // them.
    let mut classic = DrawList::new();
    view.draw_status_bar(&mut classic);
    assert!(classic.len() > 2, "{classic:?}");
    let mut moved = DrawList::new();
    moved.extend_translated(&classic, at(341.0, 0.0));
    for command in &moved {
        assert!(list.iter().any(|drawn| drawn == command), "{command:?}");
    }
}

#[test]
fn the_message_box_spans_the_widened_play_area() {
    assert_eq!(
        hud::message_box(WIDE),
        crate::geometry::Bounds {
            min: at(25.0, 737.0),
            max: at(1121.0, 763.0),
        }
    );
    let mut view = widened(flight());
    view.say_for("Widened", 100);
    let list = drawn(&view);
    assert!(
        list.iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, origin, wrap_width, .. }
                if text == "Widened" && *origin == at(25.0, 737.0) && *wrap_width == Some(1096.0)
        )),
        "{list:?}"
    );
}

#[test]
fn the_red_alert_icon_sits_left_of_the_widened_bar() {
    assert_eq!(alert::alert_origin(24, 1365.0), at(1146.0, 0.0));
    assert_eq!(alert::alert_origin(24, 1024.0), at(806.0, 0.0));
}

#[test]
fn the_floating_map_and_the_escort_menu_and_double_time_icon_keep_their_rules() {
    assert_eq!(
        floating::frame(WIDE),
        crate::geometry::Bounds::at(at(814.0, 150.0), 341.0, 341.0)
    );
    let mut view = widened(fighters_out());
    view.set_double_time(true);
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    let list = drawn(&view);
    assert!(list.iter().any(|command| *command
        == DrawCommand::Picture {
            image: ImageKey::icon(alert::DOUBLE_TIME_CICN),
            top_left: at(0.0, 0.0),
        }));
    assert!(
        list.iter().any(|command| matches!(
            command,
            DrawCommand::Line { from, color, .. }
                if *from == at(15.0, 150.0) && *color == MENU_COLORS.border
        )),
        "the escort menu at the left edge"
    );
}

#[test]
fn the_ship_is_drawn_at_the_widened_views_centre() {
    let view = widened(flight());
    assert_eq!(
        ship_sprites(&view),
        [(ImageKey::sprite(2000, 0), at(682.0, 384.0))]
    );
}

/// An idle NPC at (`x`, 0), the player at the centre, and stellar 150 at
/// (−600, 0).
fn npc_at(x: i32) -> View {
    let placed = placed(u32::try_from(x + 750).expect("in range"), 750, 0);
    let (_, chance) = scripted(&placed);
    let mut catalog = FakeCatalog {
        sol_more: vec![stellar(150, "West", (-600, 0), 1)],
        ..trafficked(&[130], 1, 129, 3)
    };
    catalog
        .sites
        .push(site(150, (-600.0, 0.0), StellarFlags::CAN_LAND));
    let mut view = widened(
        FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still)),
    );
    view.tick(TICK);
    view
}

#[test]
fn a_ship_and_a_stellar_600_left_of_the_player_are_drawn_on_screen() {
    let mut view = npc_at(-600);
    let list = drawn(&view);
    let at_82: Vec<ImageKey> = sprites(&list)
        .into_iter()
        .filter(|&(_, center)| center == at(82.0, 384.0))
        .map(|(image, _)| image)
        .collect();
    assert_eq!(
        at_82.len(),
        4,
        "the stellar, and the ship with its glow and lights: {list:?}"
    );
    let session = view.session().expect("flying");
    assert!(
        session.on_screen(&session.npcs()[0]),
        "inside the widened view"
    );
    tap(&mut view, Key::Tab);
    assert_eq!(target_of(&view), Some(NpcId(0)));
    // Its brackets, flying in, are centred on it: the top-left corner's
    // art is a corner's width further out than the top-right's.
    let corners: Vec<Point> = drawn(&view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Picture { image, top_left }
                if (brackets::SHIP_CICN..brackets::SHIP_CICN + 16).contains(&image.id) =>
            {
                Some(*top_left)
            }
            _ => None,
        })
        .collect();
    let [top_left, top_right, ..] = corners[..] else {
        panic!("no brackets: {corners:?}");
    };
    assert_eq!(top_left.y, top_right.y);
    let half = top_right.x - 82.0;
    assert_eq!(brackets::corners(at(82.0, 384.0), half)[0], top_left);
}

#[test]
fn the_session_measures_the_widened_play_area() {
    let mut view = widened(flight());
    view.tick(TICK);
    let area = view.session().expect("flying").play_area();
    assert_eq!(
        (area.left, area.top, area.right, area.bottom),
        (-682.0, -384.0, 489.0, 384.0)
    );
    let mut classic = flight();
    classic.tick(TICK);
    let area = classic.session().expect("flying").play_area();
    assert_eq!(
        (area.left, area.top, area.right, area.bottom),
        (-512.0, -384.0, 318.0, 384.0)
    );
}

#[test]
fn a_click_left_of_the_widened_bar_targets_and_one_on_it_does_nothing() {
    let mut view = npc_at(1100 - 682);
    click(&mut view, at(1100.0, 384.0));
    assert_eq!(target_of(&view), Some(NpcId(0)), "drawn at x 1100");
    let mut view = npc_at(1180 - 682);
    click(&mut view, at(1180.0, 384.0));
    assert_eq!(target_of(&view), None, "a status bar click");
}

#[test]
fn the_jumps_white_fade_covers_the_widened_view() {
    let mut view = flight();
    plot(&mut view, 131);
    let mut view = widened(view);
    fly_out(&mut view);
    jump_now(&mut view);
    tick_until_lit(&mut view);
    view.tick(ms(500));
    let list = drawn(&view);
    assert!(
        list.iter().any(|command| matches!(
            command,
            DrawCommand::Line { from, to, width, color }
                if *from == at(0.0, 384.0)
                    && *to == at(1365.0, 384.0)
                    && *width == 768.0
                    && color.r == 255
        )),
        "{list:?}"
    );
}

#[test]
fn the_course_map_in_flight_is_its_1024_frame_centred_and_takes_clicks_there() {
    let mut view = widened(flight());
    tap(&mut view, MAP);
    let target = on_map(&mut view, 131);
    let mut classic = DrawList::new();
    view.course_map().draw(&mut classic);
    let mut moved = DrawList::new();
    moved.extend_translated(&classic, at(170.0, 0.0));
    let list: Vec<DrawCommand> = drawn(&view).iter().cloned().collect();
    let tail: Vec<DrawCommand> = moved.iter().cloned().collect();
    assert!(list.ends_with(&tail), "the map, 170 right, last");
    click(&mut view, at(target.x + 170.0, target.y));
    assert_eq!(view.course_map().selected(), Some(SystemId(131)));
}

#[test]
fn a_classic_size_draws_flight_exactly_as_before() {
    let build = || {
        let (_, chance) = scripted(&two_npcs());
        let mut view = FlightView::new(armed(2, &[]))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        tap(&mut view, Key::Tab);
        tap(&mut view, MENU_KEY);
        view.say_for("Hello", 100);
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        view.tick(TICK);
        view
    };
    let before = drawn(&build());
    let mut sized = build();
    sized.set_view_size(VIEW_SIZE);
    assert_eq!(drawn(&sized), before);
}

/// `base` with asteroid type 0 and system 130 holding `count` of it.
fn rocky(base: FakeCatalog, count: i16) -> FakeCatalog {
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
            count,
            types: 0x0001,
        }],
        ..base
    }
}

/// The asteroids of `view`'s field placed before any step, by the
/// reading of [`RuleKey::AsteroidEntryField`] that places one.
fn placed_before_any_step(mut view: View) -> usize {
    view.tick(Duration::ZERO);
    view.session().expect("flying").asteroids().count()
}

#[test]
fn a_widened_views_session_stores_its_asteroids_by_the_widened_width() {
    let placed = || {
        FlightView::new(rocky(catalog(), 3)).with_rules(
            Rulebook::default().with_override(RuleKey::AsteroidEntryField, RuleSource::Bible),
        )
    };
    assert_eq!(placed_before_any_step(placed()), 4, "3 × 1024 / 640");
    assert_eq!(
        placed_before_any_step(widened(placed())),
        6,
        "3 × 1365 / 640"
    );
}

/// The hazards of the course map's panel, opened on Sol before any step.
fn hazards_on_opening(mut view: View) -> String {
    tap(&mut view, MAP);
    match view.course_map().panel().map(|panel| &panel.strip) {
        Some(nova_sim::map_panel::Strip::Info { hazards, .. }) => hazards.clone(),
        strip => panic!("{strip:?}"),
    }
}

#[test]
fn a_map_opened_before_any_step_reads_the_asteroids_stored_by_the_views_width() {
    // 2 × 1024 / 640 is 3, "sparse" (#313); 2 × 1365 / 640 is 4,
    // "moderate" (#314).
    let words = FakeCatalog {
        strings: vec![(
            nova_sim::news::MISC_STRINGS,
            (1..=400).map(|n| format!("#{n}")).collect(),
        )],
        ..catalog()
    };
    let words: Rc<dyn nova_sim::CommCatalog> = Rc::new(words);
    let view = || FlightView::new(rocky(catalog(), 2)).with_strings(Rc::clone(&words));
    assert_eq!(hazards_on_opening(view()), "#313 #316");
    assert_eq!(hazards_on_opening(widened(view())), "#314 #316");
}
