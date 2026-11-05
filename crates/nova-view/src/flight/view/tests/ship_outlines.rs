//! Ships drawn from outlines (rdm:phase/new-game-engine/phase-4-draw-flat-outlines):
//! a ship type whose sheet cannot be read but which has an outline
//! ([`ShipSprites::ship_outline`]) draws it in place of the crossed box,
//! turned by its shown heading, and is boxed, bracketed and broken up by
//! the outline's extent. Which ships have outlines is the data's
//! (nova-data's `new_game::looks`); how an outline turns is the outline
//! module's.

use nova_sim::DrawnShips as _;

use super::faked_world::{world, world_list, world_list_with};
use super::*;
use crate::flight::brackets;
use crate::outline::{self as stand_in, Outline, OutlineTriangle};

/// A dart 10 long from its centre to its nose (so 20 across as it turns),
/// red, with a blue fin.
fn dart() -> Outline {
    Outline {
        triangles: vec![
            OutlineTriangle {
                corners: [at(0.0, 10.0), at(3.0, -4.0), at(-3.0, -4.0)],
                color: Color::rgba(255, 0, 0, 255),
            },
            OutlineTriangle {
                corners: [at(0.0, -4.0), at(2.0, -6.0), at(-2.0, -6.0)],
                color: Color::rgba(0, 0, 255, 255),
            },
        ],
    }
}

/// The dart's frame: 20 by 20.
const DART: (u32, u32) = (20, 20);

/// `catalog` with the dart as the outline of ships 128 and 130.
fn darts(catalog: FakeCatalog) -> FakeCatalog {
    FakeCatalog {
        outlines: vec![(128, dart()), (130, dart())],
        ..catalog
    }
}

/// NPC `id` of ship 130, whose sheet cannot be read, at `at` facing
/// `heading`.
fn npc_130(id: u32, at: Vec2, heading: f32) -> FrameShip {
    record::npc(id, 130, at, heading)
}

/// The triangles `list` draws, in order.
fn triangles(list: &DrawList) -> Vec<DrawCommand> {
    list.iter()
        .filter(|command| matches!(command, DrawCommand::Triangle { .. }))
        .cloned()
        .collect()
}

/// The dart drawn about `centre` facing `heading`.
fn dart_at(centre: Point, heading: f32) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    stand_in::draw(&mut list, &dart(), centre, heading, 1.0);
    list.iter().cloned().collect()
}

/// Whether `list` holds a crossed box anywhere, in the placeholder's
/// colour.
fn has_crossed_box(list: &DrawList) -> bool {
    list.iter().any(|command| {
        matches!(command, DrawCommand::Line { color, width, .. } if *color == PLACEHOLDER && *width == 2.0)
    })
}

/// A view of the player at the centre and an NPC of ship 130 at
/// (100, -100) facing 90°, with the dart for both.
fn npc_drawn_as_a_dart() -> Faked {
    world(
        darts(trafficked(&[130], 1, 130, 3)),
        in_sol_with(vec![
            the_player_at(Vec2::ZERO, 0.0),
            npc_130(0, Vec2::new(100.0, -100.0), 90.0),
        ]),
    )
}

#[test]
fn an_npc_with_no_sheet_draws_its_outline_turned_by_its_heading_where_its_frame_goes() {
    let view = npc_drawn_as_a_dart();
    let npc = *view.drawn().npcs().next().expect("the npc");
    let (shown, heading) = view.shown_npc(&npc);
    assert_eq!(heading, 90.0);
    let centre = view.camera().sprite_centre(shown, DART);
    let list = world_list(&view);
    assert_eq!(triangles(&list), dart_at(centre, heading));
    assert!(!has_crossed_box(&list), "no crossed box");
}

#[test]
fn the_player_with_no_sheet_draws_its_outline_and_no_reason() {
    let sheetless = FakeCatalog {
        sheet: Err("no shän 128 for shïp 128".to_owned()),
        ..darts(catalog())
    };
    let view = world(
        sheetless,
        in_sol_with(vec![the_player_at(Vec2::ZERO, 45.0)]),
    );
    let centre = view.camera().sprite_centre(view.shown_position(), DART);
    let list = world_list(&view);
    assert_eq!(view.shown_heading(), 45.0);
    assert_eq!(triangles(&list), dart_at(centre, 45.0));
    assert!(!has_crossed_box(&list), "no crossed box");
    assert!(
        !list
            .iter()
            .any(|c| matches!(c, DrawCommand::Text { text, .. } if text.starts_with("Sprite"))),
        "no reason"
    );
}

#[test]
fn a_hidden_player_with_no_sheet_draws_no_outline() {
    let sheetless = FakeCatalog {
        sheet: Err("no shän 128 for shïp 128".to_owned()),
        ..darts(catalog())
    };
    let mut player = the_player_at(Vec2::ZERO, 45.0);
    player.shown = nova_sim::player_fate::Drawn::Hidden;
    let view = world(sheetless, in_sol_with(vec![player]));
    assert_eq!(triangles(&world_list(&view)), []);
}

#[test]
fn a_ship_drawn_from_its_outline_is_hovered_by_its_extent() {
    let sheetless = FakeCatalog {
        sheet: Err("no shän 128 for shïp 128".to_owned()),
        ..darts(trafficked(&[130], 1, 130, 3))
    };
    let view = world(
        sheetless,
        in_sol_with(vec![
            the_player_at(Vec2::ZERO, 0.0),
            npc_130(0, Vec2::new(100.0, -100.0), 90.0),
        ]),
    );
    let boxes = view.drawn_boxes();
    let npc = boxes.npc(NpcId(0)).expect("boxed");
    assert_eq!((npc.rect.width, npc.rect.height), DART);
    let player = boxes.player.expect("boxed");
    assert_eq!((player.rect.width, player.rect.height), DART);
}

#[test]
fn a_ship_drawn_from_its_outline_is_bracketed_by_its_extent() {
    let mut view = npc_drawn_as_a_dart();
    let hud = nova_sim::flight_frame::FrameHud {
        target_brackets: Some(nova_sim::brackets::TargetBrackets {
            npc: NpcId(0),
            expand: 0.0,
            standing: brackets::Standing::Neutral,
        }),
        ..nova_sim::flight_frame::FrameHud::default()
    };
    let list = world_list_with(&mut view, &hud);
    let npc = *view.drawn().npcs().next().expect("the npc");
    let at = view.camera().whole_point(view.shown_npc(&npc).0);
    let mut expected = DrawList::new();
    brackets::draw_ship(
        &mut expected,
        at,
        Some(DART),
        0.0,
        brackets::Standing::Neutral,
    );
    let expected: Vec<_> = expected.iter().cloned().collect();
    let drawn: Vec<_> = list.iter().cloned().collect();
    assert!(
        drawn.windows(expected.len()).any(|run| run == expected),
        "{drawn:?}"
    );
}

#[test]
fn a_ship_drawn_from_its_outline_breaks_up_as_wide_as_its_extent() {
    let sheetless = FakeCatalog {
        sheet: Err("no shän 128 for shïp 128".to_owned()),
        ..darts(trafficked(&[130], 1, 130, 3))
    };
    let view = world(
        sheetless,
        in_sol_with(vec![
            the_player_at(Vec2::ZERO, 0.0),
            npc_130(0, Vec2::new(100.0, -100.0), 90.0),
        ]),
    );
    let breaking: Vec<FrameShip> = view
        .drawn()
        .ships
        .iter()
        .map(|ship| FrameShip {
            condition: Condition::Dying {
                ticks_left: 10,
                starting: false,
            },
            ..*ship
        })
        .collect();
    let breakups = super::super::dying(
        &breaking,
        (&view.sheet, view.outline.as_ref()),
        &view.npc_sheets,
        &view.npc_outlines,
    );
    let widths: Vec<f32> = breakups.iter().map(|ship| ship.sprite_width).collect();
    assert_eq!(widths, [20.0, 20.0], "the player and the npc");
}

#[test]
fn a_ship_with_neither_a_sheet_nor_an_outline_breaks_up_as_wide_as_its_crossed_box() {
    let view = world(
        trafficked(&[130], 1, 130, 3),
        in_sol_with(vec![
            the_player_at(Vec2::ZERO, 0.0),
            npc_130(0, Vec2::new(100.0, -100.0), 90.0),
        ]),
    );
    let breaking: Vec<FrameShip> = view
        .drawn()
        .npcs()
        .map(|ship| FrameShip {
            condition: Condition::Dying {
                ticks_left: 10,
                starting: false,
            },
            ..*ship
        })
        .collect();
    let breakups = super::super::dying(
        &breaking,
        (&view.sheet, view.outline.as_ref()),
        &view.npc_sheets,
        &view.npc_outlines,
    );
    assert_eq!(breakups[0].sprite_width, PLACEHOLDER_SIZE);
    assert!(
        has_crossed_box(&world_list(&view)),
        "the crossed box, as before"
    );
}

#[test]
fn a_ship_with_a_sheet_reads_and_draws_no_outline() {
    let view = world(
        darts(trafficked(&[130], 1, 129, 3)),
        in_sol_with(vec![
            the_player_at(Vec2::ZERO, 0.0),
            npc_129(0, Vec2::new(100.0, -100.0), 90.0),
        ]),
    );
    assert_eq!(*view.catalog().outlines_asked.borrow(), []);
    assert_eq!(triangles(&world_list(&view)), []);
}

#[test]
fn each_ship_type_s_outline_is_read_once_beside_its_sheet() {
    let mut view = world(
        darts(trafficked(&[130], 1, 130, 3)),
        in_sol_with(vec![
            the_player_at(Vec2::ZERO, 0.0),
            npc_130(0, Vec2::new(100.0, -100.0), 90.0),
            npc_130(1, Vec2::new(-100.0, -100.0), 0.0),
        ]),
    );
    super::faked_hud::shows(
        &mut view,
        in_sol_with(vec![
            the_player_at(Vec2::ZERO, 0.0),
            npc_130(2, Vec2::new(0.0, -100.0), 0.0),
        ]),
    );
    assert_eq!(*view.catalog().outlines_asked.borrow(), [ShipId(130)]);
    assert_eq!(
        view.catalog()
            .sheets_asked
            .borrow()
            .iter()
            .filter(|&&ship| ship == ShipId(130))
            .count(),
        1
    );
    assert_eq!(triangles(&world_list(&view)).len(), dart().triangles.len());
}
