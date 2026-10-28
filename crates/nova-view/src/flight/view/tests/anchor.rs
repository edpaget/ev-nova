//! Where flight draws the player's ship, by both readings of
//! [`RuleKey::FlightAnchor`]: the original's `_screenCenter`, the play
//! area's centre left of the status bar (`_SetupWind` @0x1b720-0x1b78a),
//! or the view's centre. The play area the flight is told, the
//! brackets, clicks in space and the starfield (the original's stars and
//! the stand-in's dots) are placed from the same anchor. On
//! [`FakeFlight`]: the frames are scripted.

use super::faked_world::{faked_ship_sprites, shows, world_by, world_list, world_list_with};
use super::*;
use crate::flight::brackets;

/// A 16:9 window's widened view.
const WIDE: (f32, f32) = (1365.0, 768.0);

/// The rule set reading `source` for where flight is drawn from.
fn reading(source: RuleSource) -> Rulebook {
    Rulebook::default().with_override(RuleKey::FlightAnchor, source)
}

/// The anchor each reading draws the ship at in a view `view` in size.
fn anchor_for(source: RuleSource, view: (f32, f32)) -> Point {
    match (source, view) {
        (RuleSource::Engine, (1024.0, _)) => at(415.0, 384.0),
        (RuleSource::Engine, _) => at(585.0, 384.0),
        (RuleSource::Bible, (1024.0, _)) => VIEW_CENTER,
        (RuleSource::Bible, _) => at(682.0, 384.0),
    }
}

/// A flight of `ships` in Sol from `catalog`, in a view `view` in size,
/// by `rules`.
fn sized_in(
    catalog: FakeCatalog,
    ships: Vec<nova_sim::flight_frame::FrameShip>,
    rules: &Rulebook,
    view: (f32, f32),
) -> Faked {
    let mut flight = world_by(catalog, in_sol_with(ships), rules);
    flight.view = view;
    flight
}

/// The player alone at the centre, in a view `view` in size, by
/// `source`'s reading.
fn sized(source: RuleSource, view: (f32, f32)) -> Faked {
    sized_in(
        catalog(),
        vec![the_player_at(Vec2::ZERO, 0.0)],
        &reading(source),
        view,
    )
}

#[test]
fn by_the_original_the_ship_is_drawn_at_the_play_areas_centre() {
    for size in [VIEW_SIZE, WIDE] {
        let view = sized(RuleSource::Engine, size);
        assert_eq!(view.camera().anchor(), anchor_for(RuleSource::Engine, size));
        assert_eq!(
            faked_ship_sprites(&view),
            [(
                ImageKey::sprite(2000, 0),
                anchor_for(RuleSource::Engine, size)
            )],
            "{size:?}"
        );
    }
}

#[test]
fn by_the_other_reading_the_ship_is_drawn_at_the_views_centre() {
    for size in [VIEW_SIZE, WIDE] {
        let view = sized(RuleSource::Bible, size);
        assert_eq!(
            faked_ship_sprites(&view),
            [(
                ImageKey::sprite(2000, 0),
                anchor_for(RuleSource::Bible, size)
            )],
            "{size:?}"
        );
        assert_eq!(view.camera().anchor(), anchor_for(RuleSource::Bible, size));
    }
}

#[test]
fn by_the_other_reading_flight_s_camera_is_the_one_it_had_before() {
    for size in [VIEW_SIZE, WIDE] {
        let mut view = sized(RuleSource::Bible, size);
        shows(
            &mut view,
            in_sol_with(vec![the_player_at(Vec2::new(30.0, -60.0), 45.0)]),
        );
        view.alpha = 0.5;
        assert_eq!(view.shown_position(), at(15.0, -30.0), "half way");
        assert_eq!(
            view.camera(),
            crate::system::Camera::centred_on_in(view.shown_position(), size),
            "{size:?}"
        );
    }
}

#[test]
fn the_flight_is_told_the_play_area_from_the_anchor() {
    // What the session holds of it is nova-sim's
    // (`session::step::tests::with_no_step_the_input_is_held_and_the_field_settled`).
    for (source, size, area) in [
        (
            RuleSource::Engine,
            VIEW_SIZE,
            (-415.0, -384.0, 415.0, 384.0),
        ),
        (RuleSource::Engine, WIDE, (-585.0, -384.0, 586.0, 384.0)),
        (RuleSource::Bible, VIEW_SIZE, (-512.0, -384.0, 318.0, 384.0)),
        (RuleSource::Bible, WIDE, (-682.0, -384.0, 489.0, 384.0)),
    ] {
        let mut view = sized(source, size);
        view.step_frame(1, view.anchor());
        let told = view.sim.inputs()[0].play_area;
        assert_eq!(
            (told.left, told.top, told.right, told.bottom),
            area,
            "{source:?} {size:?}"
        );
    }
}

#[test]
fn a_click_where_a_ship_is_drawn_sends_its_box_there_and_its_brackets_frame_it() {
    // What the click then targets is nova-sim's
    // (`session::apply::tests::a_click_in_space_on_a_ship_drawn_answers_the_ship_it_targets`).
    for source in [RuleSource::Engine, RuleSource::Bible] {
        for size in [VIEW_SIZE, WIDE] {
            let anchor = anchor_for(source, size);
            let ships = vec![
                the_player_at(Vec2::ZERO, 0.0),
                npc_129(0, Vec2::new(-300.0, 0.0), 0.0),
            ];
            let mut view = sized_in(trafficked(&[130], 1, 129, 3), ships, &reading(source), size);
            let drawn_at = at(anchor.x - 300.0, anchor.y);
            assert!(
                faked_ship_sprites(&view)
                    .iter()
                    .any(|&(_, p)| p == drawn_at),
                "{source:?} {size:?}: {:?}",
                faked_ship_sprites(&view)
            );
            view.click_in_space(drawn_at);
            let [FlightCommand::ClickInSpace { at: clicked, drawn }] = sent(&view) else {
                panic!("{:?}", sent(&view));
            };
            assert_eq!(*clicked, Vec2::new(-300.0, 0.0), "{source:?} {size:?}");
            let boxed = drawn
                .npcs
                .iter()
                .find(|&&(id, _)| id == NpcId(0))
                .expect("boxed")
                .1;
            assert_eq!(boxed.centre, Vec2::new(-300.0, 0.0), "where it is drawn");
            let hud = nova_sim::flight_frame::FrameHud {
                target_brackets: Some(nova_sim::brackets::TargetBrackets {
                    npc: NpcId(0),
                    expand: 0.0,
                    standing: brackets::Standing::Neutral,
                }),
                ..nova_sim::flight_frame::FrameHud::default()
            };
            let corners: Vec<Point> = world_list_with(&mut view, &hud)
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
            let half = top_right.x - drawn_at.x;
            assert_eq!(
                brackets::corners(drawn_at, half)[0],
                top_left,
                "{source:?} {size:?}"
            );
        }
    }
}

#[test]
fn the_stand_in_starfield_is_drawn_about_the_anchor_and_covers_the_view() {
    for source in [RuleSource::Engine, RuleSource::Bible] {
        for size in [VIEW_SIZE, WIDE] {
            let rules = reading(source).with_override(RuleKey::FlightStarfield, RuleSource::Bible);
            let view = sized_in(starry(), vec![the_player_at(Vec2::ZERO, 0.0)], &rules, size);
            let camera = view.camera();
            assert_eq!(camera.anchor(), anchor_for(source, size));
            let mut field = DrawList::new();
            starfield::draw(&mut field, &camera);
            let field: Vec<DrawCommand> = field.iter().cloned().collect();
            let list: Vec<DrawCommand> = world_list(&view).iter().cloned().collect();
            assert_eq!(
                list[..field.len()],
                field[..],
                "{source:?} {size:?}: the starfield through the flight's camera, first"
            );
            assert!(!list.iter().any(is_star), "{source:?} {size:?}");
            let xs: Vec<f32> = field
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Dot { center, .. } => Some(center.x),
                    _ => None,
                })
                .collect();
            let left = xs.iter().copied().fold(f32::INFINITY, f32::min);
            let right = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            assert!(left < 50.0, "{source:?} {size:?}: leftmost star at {left}");
            assert!(
                right > size.0 - 50.0,
                "{source:?} {size:?}: rightmost star at {right}"
            );
        }
    }
}

#[test]
fn the_originals_stars_are_scattered_over_the_play_area_and_placed_about_the_anchor() {
    for source in [RuleSource::Engine, RuleSource::Bible] {
        for size in [VIEW_SIZE, WIDE] {
            let mut view = sized_in(
                starry(),
                vec![the_player_at(Vec2::ZERO, 0.0)],
                &reading(source),
                size,
            );
            view.reset_stars();
            let anchor = anchor_for(source, size);
            // The default chance draws the last of each range: each star
            // W − 1 − trunc(W / 2) right of the ship and H − 1 − trunc(H /
            // 2) below it, placed at the anchor less its whole size and
            // drawn about its rect's centre.
            let width = size.0 - 194.0;
            let right = width - 1.0 - (width / 2.0).trunc();
            let below = size.1 - 1.0 - (size.1 / 2.0).trunc();
            let centre = at(anchor.x + right - 2.5, anchor.y + below - 2.5);
            let stars: Vec<Point> = world_list(&view)
                .iter()
                .filter(|command| is_star(command))
                .filter_map(|command| match command {
                    DrawCommand::Sprite { center, .. } => Some(*center),
                    _ => None,
                })
                .collect();
            assert_eq!(stars, [centre; 20], "{source:?} {size:?}");
        }
    }
}
