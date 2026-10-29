//! Flight in a view widened to 1365 x 768, a 16:9 window's: the HUD at
//! the original's places for that width, the ship at the play area's
//! centre (the `anchor` tests cover the other reading),
//! every visible-area rule measured against the widened view, and the
//! course map in its 1024-wide frame, centred.

use super::faked_screen::in_flight;
use super::faked_travel::{jump_now, lit};
use super::faked_world::{
    faked_ship_sprites, saying, targeting, world, world_list, world_list_with,
};
use super::*;
use crate::flight::floating;
use crate::flight::{alert, brackets};
use crate::view_shape::ViewShape;
use nova_sim::flight_frame::FrameHud;

/// A 16:9 window's widened view.
const WIDE: (f32, f32) = (1365.0, 768.0);

#[test]
fn the_message_box_spans_the_widened_play_area() {
    assert_eq!(
        hud::message_box(WIDE),
        crate::geometry::Bounds {
            min: at(25.0, 737.0),
            max: at(1121.0, 763.0),
        }
    );
    let mut view = widened_world(vec![the_player_at(Vec2::ZERO, 0.0)]);
    saying(&mut view, "Widened", 100);
    let list = world_list(&view);
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
    let mut view = widened_world(vec![the_player_at(Vec2::ZERO, 0.0)]);
    view.sim.frame_mut().double_time = true;
    view.refresh_frame();
    let mut hud = FrameHud::default();
    hud.escort_rows[0].present = true;
    view.escort_menu.toggle(view.elapsed, &hud.escort_rows);
    assert!(view.escort_menu.is_open());
    let list = world_list_with(&mut view, &hud);
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
fn the_ship_is_drawn_at_the_widened_play_areas_centre() {
    let view = widened_world(vec![the_player_at(Vec2::ZERO, 0.0)]);
    assert_eq!(
        faked_ship_sprites(&view),
        [(ImageKey::sprite(2000, 0), at(585.0, 384.0))]
    );
}

/// A widened flight of `ships` in Sol, with stellar 150 at (−500, 0)
/// shown too.
fn widened_world(ships: Vec<nova_sim::flight_frame::FrameShip>) -> Faked {
    let catalog = FakeCatalog {
        sol_more: vec![stellar(150, "West", (-500, 0), 1)],
        ..trafficked(&[130], 1, 129, 3)
    };
    let mut frame = in_sol_with(ships);
    frame.stellars.push(record::stellar(StellarId(150)));
    let mut view = world(catalog, frame);
    view.view = WIDE;
    view
}

/// The widened flight with an idle NPC at (`x`, 0), the player at the
/// play area's centre.
fn npc_at(x: f32) -> Faked {
    let mut npc = npc_129(0, Vec2::new(x, 0.0), 0.0);
    npc.drawn.lights = Some(32);
    npc.glow = 24;
    widened_world(vec![the_player_at(Vec2::ZERO, 0.0), npc])
}

#[test]
fn a_ship_and_a_stellar_500_left_of_the_player_are_drawn_on_screen() {
    // 500 left of the player is off the classic view's left edge, 415
    // from it. Whether the session counts the ship on screen is nova-sim's
    // (the play area it is told, `the_flight_is_told_the_widened_play_area`).
    let mut view = npc_at(-500.0);
    let list = world_list_with(&mut view, &targeting(0));
    let at_85: Vec<ImageKey> = sprites(&list)
        .into_iter()
        .filter(|&(_, center)| center == at(85.0, 384.0))
        .map(|(image, _)| image)
        .collect();
    assert_eq!(
        at_85.len(),
        4,
        "the stellar, and the ship with its glow and lights: {list:?}"
    );
    // Its brackets are centred on it: the top-left corner's art is a
    // corner's width further out than the top-right's.
    let corners: Vec<Point> = list
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
    let half = top_right.x - 85.0;
    assert_eq!(brackets::corners(at(85.0, 384.0), half)[0], top_left);
}

#[test]
fn the_flight_is_told_the_widened_play_area() {
    let mut view = widened_world(vec![the_player_at(Vec2::ZERO, 0.0)]);
    view.step_frame(0, view.anchor());
    let area = view.sim.inputs()[0].play_area;
    assert_eq!(
        (area.left, area.top, area.right, area.bottom),
        (-585.0, -384.0, 586.0, 384.0)
    );
    view.view = VIEW_SIZE;
    view.step_frame(0, view.anchor());
    let area = view.sim.inputs()[1].play_area;
    assert_eq!(
        (area.left, area.top, area.right, area.bottom),
        (-415.0, -384.0, 415.0, 384.0)
    );
}

#[test]
fn a_click_left_of_the_widened_bar_is_in_space_and_one_on_it_is_not() {
    // What the click in space targets is nova-sim's
    // (`session::apply::tests::a_click_in_space_on_a_ship_drawn_answers_the_ship_it_targets`).
    let mut view = npc_at(1100.0 - 585.0);
    view.click_in_space(at(1100.0, 384.0));
    let [FlightCommand::ClickInSpace { at: clicked, .. }] = sent(&view) else {
        panic!("{:?}", sent(&view));
    };
    assert_eq!(*clicked, Vec2::new(1100.0 - 585.0, 0.0), "drawn at x 1100");
    let mut view = npc_at(1180.0 - 585.0);
    view.click_in_space(at(1180.0, 384.0));
    assert_eq!(sent(&view), [], "a status bar click");
}

/// `view` widened to [`WIDE`].
fn widened(mut view: Faked) -> Faked {
    view.set_widened_size(WIDE);
    view
}

#[test]
fn flight_asks_to_widen_and_takes_the_widened_size() {
    let mut view = in_flight();
    assert_eq!(view.view_shape(), ViewShape::Widened);
    assert_eq!(view.view_size(), VIEW_SIZE, "classic until told");
    view.set_widened_size(WIDE);
    assert_eq!(view.view_size(), WIDE);
    assert_eq!(view.camera().view_size(), WIDE);
}

#[test]
fn widening_tells_the_flight_the_widened_windows_width_at_once() {
    // What the session stores by the width (the asteroids it places, the
    // map panel's hazards) is nova-sim's.
    let view = widened(in_flight());
    assert_eq!(
        sent(&view).last(),
        Some(&FlightCommand::SetWindowWidth(1365)),
        "before any step"
    );
    assert_eq!(view.sim.steps(), 0);
}

#[test]
fn the_status_bar_sits_against_the_widened_views_right_edge() {
    let view = widened(super::faked_hud::barred(catalog(), sol()));
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
fn the_jumps_white_fade_covers_the_widened_view() {
    let mut view = widened(in_flight());
    jump_now(&mut view);
    lit(&mut view, 1.0);
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
    let mut view = widened(in_flight());
    tap(&mut view, MAP);
    let target = faked_on_map(&mut view, 131);
    let mut classic = DrawList::new();
    view.course_map().draw(&mut classic);
    let mut moved = DrawList::new();
    moved.extend_translated(&classic, at(170.0, 0.0));
    let list: Vec<DrawCommand> = drawn(&view).iter().cloned().collect();
    let tail: Vec<DrawCommand> = moved.iter().cloned().collect();
    assert!(list.ends_with(&tail), "the map, 170 right, last");
    for pressed in [true, false] {
        view.input(&Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed,
            at: at(target.x + 170.0, target.y),
        });
    }
    let clicked: Vec<_> = sent(&view)
        .iter()
        .filter_map(|command| match command {
            FlightCommand::MapClick(click) => Some(click.hits.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(clicked, [vec![SystemId(131)]], "Alpha Centauri's hit box");
}

#[test]
fn a_classic_size_draws_flight_exactly_as_before() {
    let build = || {
        let mut npc = npc_129(0, Vec2::new(100.0, -100.0), 90.0);
        npc.drawn.lights = Some(32);
        let mut frame = in_sol_with(vec![
            the_player_at(Vec2::ZERO, 0.0),
            npc,
            npc_129(1, Vec2::new(-50.0, 100.0), 0.0),
        ]);
        frame.target = Some(record::target(0));
        frame.hud = targeting(0);
        frame.hud.escort_rows[0].present = true;
        frame.hud.escorts_flying = true;
        let mut view = world(armed(2, &[]), frame);
        view.sim.queue_screen(floating_shown(sol_drawing(), None));
        tap(&mut view, crate::flight::escorts::MENU_KEY);
        view.say_for("Hello", 100);
        view.hyperspace_key(HYPERSPACE_MODE_KEY);
        view.tick(TICK);
        view
    };
    let before = drawn(&build());
    assert!(texts(&before).contains(&"Hello".to_owned()), "the message");
    assert!(
        texts(&before).contains(&"Escort Commands".to_owned()),
        "the menu"
    );
    assert!(!floating_border(&before).is_empty(), "the floating map");
    let mut sized = build();
    sized.set_view_size(VIEW_SIZE);
    assert_eq!(drawn(&sized), before);
}
