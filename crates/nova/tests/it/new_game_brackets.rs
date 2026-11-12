//! The new game's brackets: over the fixture as the app opens it
//! (`DiskData::open_new_game`, its world models wired as `main` wires
//! them), a new pilot selects Hearth's first stellar and targets a ship
//! by the keys, and the flight draws four corner pictures round each.
//! Each resolves to an image through the game's images as `main` builds
//! them, no icon fails to draw, and each corner lies outside what is
//! drawn of the stellar and the ship.
//!
//! Wiring only: the corners are nova-data's (`new_game::brackets`), the
//! store answering them nova-data's (`GameData::icon`) and the renderer's
//! route to it nova-render's (`source`); where the brackets go and how
//! big they are is nova-view's (`flight::brackets`,
//! `SceneStellar::bracket_size`), each pinned there. This pins that they
//! meet: no test here asserts a bracket's size.

use std::rc::Rc;

use nova::app::Control;
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova::startup::{DataSource, DiskData, NewGameOpened};
use nova_data::{GameData, StandIn, StandInOf, new_game};
use nova_render::{ImageError, ImageSource, WorldAssets};
use nova_sim::fixture::MemoryPilots;
use nova_sim::flight_frame::FlightFrame;
use nova_sim::{ShipRef, Vec2};
use nova_view::flight::brackets::{CORNER, DOMINATED_CICN, NAV_CICN, ship_cicn};
use nova_view::{DrawCommand, DrawLayer, DrawList, ImageKey, ImageKind, Point, Screen};
use winit::keyboard::KeyCode;

use crate::new_game_models::{flying_over, frame};
use crate::pilots::Harness;

/// How far the drawing of a ship may reach past its brackets' inner
/// edges: under a pixel. The original halves a sprite's frame in whole
/// pixels and puts a ship's brackets at its whole point, so an
/// odd-sized frame's drawing may reach a fraction of a pixel under the
/// far corners (nova-view's `flight::brackets` docs).
const ROUNDING: f32 = 1.0;

/// The fixture as the app opens it: its data and its world models.
fn opened() -> (Rc<GameData>, WorldAssets) {
    let NewGameOpened::Loaded(loaded) = DiskData.open_new_game(&new_game::fixture::folder()) else {
        panic!("the fixture opens");
    };
    assert_eq!(loaded.problems, Vec::<String>::new());
    (Rc::new(loaded.data), loaded.world)
}

/// Everything the view draws now, with its layer.
fn drawn(game: &Harness) -> Vec<(DrawLayer, DrawCommand)> {
    let view = game.app.screen().flight_view().expect("flying");
    let mut list = DrawList::new();
    view.draw(&mut list);
    list.layered()
        .map(|(layer, command)| (layer, command.clone()))
        .collect()
}

/// Sends a frame, keeping any icon that failed to draw in `failed`.
fn redraw(game: &mut Harness, failed: &mut Vec<(ImageKey, ImageError)>) {
    assert_eq!(game.redraw(), Control::Continue);
    failed.extend(
        game.app
            .take_failures()
            .into_iter()
            .filter(|(key, _)| key.kind == ImageKind::Icon),
    );
}

/// Presses `key` each frame until `taken` holds of the frame, as the
/// flight may not step its first frames; then draws frames until `settled`
/// holds.
fn press_until(
    game: &mut Harness,
    key: KeyCode,
    failed: &mut Vec<(ImageKey, ImageError)>,
    taken: impl Fn(&FlightFrame) -> bool,
    settled: impl Fn(&FlightFrame) -> bool,
) {
    for _ in 0..crate::common::UNTIL_FRAMES {
        if taken(&frame(game)) {
            break;
        }
        game.press(key);
        redraw(game, failed);
    }
    assert!(taken(&frame(game)), "{key:?} never took");
    for _ in 0..crate::common::UNTIL_FRAMES {
        if settled(&frame(game)) {
            return;
        }
        redraw(game, failed);
    }
    panic!("never settled after {key:?}");
}

/// The four corners of `cïcn` `first` on that `list` draws, in order:
/// top-left, top-right, bottom-right, bottom-left.
fn corners(list: &[(DrawLayer, DrawCommand)], first: i16) -> [(ImageKey, Point); 4] {
    let found: Vec<(ImageKey, Point)> = list
        .iter()
        .filter_map(|(_, command)| match command {
            DrawCommand::Picture { image, top_left }
                if image.kind == ImageKind::Icon && (first..first + 4).contains(&image.id) =>
            {
                Some((*image, *top_left))
            }
            _ => None,
        })
        .collect();
    let ids: Vec<i16> = found.iter().map(|(key, _)| key.id).collect();
    assert_eq!(ids, (first..first + 4).collect::<Vec<_>>(), "four corners");
    [found[0], found[1], found[2], found[3]]
}

/// A box: left, top, right, bottom.
#[derive(Clone, Copy, Debug)]
struct Bounds {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl Bounds {
    fn round(points: impl IntoIterator<Item = Point>) -> Self {
        points.into_iter().fold(
            Self {
                left: f32::MAX,
                top: f32::MAX,
                right: f32::MIN,
                bottom: f32::MIN,
            },
            |b, p| Self {
                left: b.left.min(p.x),
                top: b.top.min(p.y),
                right: b.right.max(p.x),
                bottom: b.bottom.max(p.y),
            },
        )
    }

    fn holds(self, at: Point, slack: f32) -> bool {
        (self.left - slack..=self.right + slack).contains(&at.x)
            && (self.top - slack..=self.bottom + slack).contains(&at.y)
    }
}

/// The bounds of the shape the world layer of `list` draws round `at` in
/// `colours`: its triangles whose box holds `at` (within two pixels, as
/// the frame's position and the drawn one may differ by the frame's
/// interpolation), and every triangle that shares a corner with one of
/// them, as a disc's fan, a ring's band and an outline's triangles do.
fn shape_round(list: &[(DrawLayer, DrawCommand)], at: Point, colours: &[[u8; 3]]) -> Bounds {
    let triangles: Vec<[Point; 3]> = list
        .iter()
        .filter_map(|(layer, command)| match command {
            DrawCommand::Triangle { corners, color }
                if *layer == DrawLayer::World && colours.contains(&[color.r, color.g, color.b]) =>
            {
                Some(*corners)
            }
            _ => None,
        })
        .collect();
    let mut shape: Vec<bool> = triangles
        .iter()
        .map(|t| Bounds::round(*t).holds(at, 2.0))
        .collect();
    assert!(shape.contains(&true), "a shape round {at:?} in {colours:?}");
    loop {
        let corners: Vec<Point> = triangles
            .iter()
            .zip(&shape)
            .filter(|(_, inside)| **inside)
            .flat_map(|(t, _)| *t)
            .collect();
        let grown: Vec<bool> = triangles
            .iter()
            .zip(&shape)
            .map(|(t, &inside)| inside || t.iter().any(|p| corners.contains(p)))
            .collect();
        if grown == shape {
            break;
        }
        shape = grown;
    }
    Bounds::round(
        triangles
            .iter()
            .zip(&shape)
            .filter(|(_, inside)| **inside)
            .flat_map(|(t, _)| *t),
    )
}

/// The colours `data` draws `of`'s stand-in in.
fn stand_in_colours(data: &GameData, of: StandInOf) -> Vec<[u8; 3]> {
    match data.stand_in(of).expect("a stand-in") {
        StandIn::Outline(outline) => outline.triangles.iter().map(|t| t.color).collect(),
        StandIn::Disc { color } | StandIn::Ring { color } => vec![*color],
    }
}

/// Asserts each corner lies outside `drawn` on its own diagonal, the
/// drawing reaching less than `rounding` past a corner's inner edges.
fn assert_outside(corners: &[(ImageKey, Point); 4], drawn: Bounds, rounding: f32, what: &str) {
    let [
        (_, top_left),
        (_, top_right),
        (_, bottom_right),
        (_, bottom_left),
    ] = *corners;
    let past = [
        top_left.x + CORNER - drawn.left,
        top_left.y + CORNER - drawn.top,
        drawn.right - top_right.x,
        top_right.y + CORNER - drawn.top,
        drawn.right - bottom_right.x,
        drawn.bottom - bottom_right.y,
        bottom_left.x + CORNER - drawn.left,
        drawn.bottom - bottom_left.y,
    ];
    assert!(
        past.iter().all(|&over| over < rounding),
        "{what}: corners {corners:?} round {drawn:?}, reaching past by {past:?}"
    );
}

#[test]
fn a_new_pilot_s_selected_stellar_and_targeted_ship_are_bracketed_by_drawn_corners() {
    let (data, world) = opened();
    let pilots = MemoryPilots::new();
    let mut game = flying_over(Ruleset::NewGame, &data, &world, &pilots);
    let rules = ruleset_rulebook(None, Ruleset::NewGame.profile().readings).0;
    let images = nova::stock_look::images(&data, &rules, None, None);
    let mut failed = Vec::new();

    // 1 selects Hearth's first stellar, and its brackets fly in.
    press_until(
        &mut game,
        KeyCode::Digit1,
        &mut failed,
        |frame| frame.hud.nav_brackets.is_some(),
        |frame| frame.hud.nav_brackets.is_some_and(|nav| nav.expand == 0.0),
    );
    // Tab targets the nearest ship, and its brackets fly in.
    press_until(
        &mut game,
        KeyCode::Tab,
        &mut failed,
        |frame| frame.hud.target_brackets.is_some(),
        |frame| {
            frame
                .hud
                .target_brackets
                .is_some_and(|target| target.expand == 0.0)
        },
    );
    assert_eq!(failed, [], "no icon failed to draw");

    let now = frame(&game);
    let list = drawn(&game);
    let camera = game.app.screen().flight_view().expect("flying").camera();
    let nav = now.hud.nav_brackets.expect("a stellar selected");
    let target = now.hud.target_brackets.expect("a ship targeted");
    let ship = now
        .ships
        .iter()
        .find(|ship| ship.ship == ShipRef::Npc(target.npc))
        .expect("the target in the frame");
    let on_screen = |at: Vec2| camera.world_to_screen(Point::new(at.x, at.y));

    let nav_first = if nav.dominated {
        DOMINATED_CICN
    } else {
        NAV_CICN
    };
    let round_stellar = corners(&list, nav_first);
    let round_ship = corners(&list, ship_cicn(target.standing));
    for (key, _) in round_stellar.iter().chain(&round_ship) {
        let frames = images.frames(key.kind, key.id);
        assert!(
            frames.as_ref().is_ok_and(|frames| !frames.is_empty()),
            "{key:?} resolves: {frames:?}"
        );
    }

    let stellar = shape_round(
        &list,
        on_screen(nav.position),
        &stand_in_colours(&data, StandInOf::Stellar(nav.stellar)),
    );
    assert_outside(&round_stellar, stellar, f32::EPSILON, "the stellar");
    let hull = shape_round(
        &list,
        on_screen(ship.position),
        &stand_in_colours(&data, StandInOf::Ship(ship.class)),
    );
    assert_outside(&round_ship, hull, ROUNDING, "the ship");
}
