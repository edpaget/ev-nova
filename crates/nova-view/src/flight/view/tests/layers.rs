use super::*;
use crate::DrawLayer;

// Which of flight's drawing is the world layer and which the overlay.

/// Whether `command` draws text, clipped or not.
fn is_text(command: &DrawCommand) -> bool {
    match command {
        DrawCommand::Text { .. } => true,
        DrawCommand::Clipped { command, .. } => is_text(command),
        _ => false,
    }
}

/// One past `list`'s last world command: 0 with none.
fn world_end(list: &DrawList) -> usize {
    list.layered()
        .enumerate()
        .filter(|(_, (layer, _))| *layer == DrawLayer::World)
        .map(|(at, _)| at + 1)
        .last()
        .unwrap_or(0)
}

/// The layers of the commands in `list` that are `wanted`, in order.
fn layers_of(list: &DrawList, wanted: impl Fn(&DrawCommand) -> bool) -> Vec<DrawLayer> {
    list.layered()
        .filter(|(_, command)| wanted(command))
        .map(|(layer, _)| layer)
        .collect()
}

/// Asserts that `list` draws its world first, as one run broken only by
/// text, which is overlay wherever it is drawn, and then only overlay.
#[track_caller]
fn assert_world_then_overlay(list: &DrawList) {
    let end = world_end(list);
    for (at, (layer, command)) in list.layered().enumerate() {
        let expected = if is_text(command) || at >= end {
            DrawLayer::Overlay
        } else {
            DrawLayer::World
        };
        assert_eq!(layer, expected, "command {at}: {command:?}");
    }
}

/// Asserts that every command in `list` that is `wanted` (and there is
/// one) is in `layer`.
#[track_caller]
fn assert_all_in(list: &DrawList, layer: DrawLayer, wanted: impl Fn(&DrawCommand) -> bool) {
    let found = layers_of(list, wanted);
    assert!(!found.is_empty(), "none drawn");
    assert!(found.iter().all(|&found| found == layer), "{found:?}");
}

/// A fight, titled and with its help line, where the player has blasted
/// NPC 0 (straight above it) into an explosion and targets NPC 1, its
/// lights on, its blaster still firing and the pointer in space.
fn mid_fight() -> View {
    let mut catalog = armed(2, &[BLASTER]);
    catalog.weapons[0].mass_dmg = 1000;
    catalog.weapons[0].flags = 0x0020;
    catalog.weapons[0].explod_type = -1;
    catalog.sheet = Ok(layered_sheet());
    let draws = [placed(750, 650, 180), placed(700, 750, 0)].concat();
    let mut view = fighting(catalog, &draws).with_overlays(TITLED);
    view.input(&key(Key::Up, true));
    view.tick(TICK);
    view.input(&key(Key::Up, false));
    view.input(&key(FIRE_KEY, true));
    for _ in 0..10 {
        view.tick(TICK);
        if view.session().expect("flying").npcs().len() == 1 {
            break;
        }
    }
    assert_eq!(view.effects().explosions().len(), 1, "NPC 0 exploded");
    tap(&mut view, Key::Tab);
    assert!(target_of(&view).is_some(), "NPC 1 targeted");
    view.input(&Input::PointerMoved(Point::new(40.0, 400.0)));
    view
}

/// [`sheet`] with a glow and lights.
fn layered_sheet() -> ShipSheet {
    ShipSheet {
        glow: Some(layer(2100, 36)),
        lights: Some(layer(2200, 12)),
        ..sheet()
    }
}

#[test]
fn a_fights_scene_is_the_world_and_its_hud_every_text_and_the_cursor_overlay() {
    let view = mid_fight();
    let list = drawn(&view);
    assert_world_then_overlay(&list);
    let world = DrawLayer::World;
    assert_eq!(list.layered().next().map(|(layer, _)| layer), Some(world));
    // The stars, the stellars, the explosion and its debris, the NPC, the
    // player's ship and its ORed lights, and the shots.
    assert!(matches!(list.iter().next(), Some(DrawCommand::Dot { .. })));
    for id in [1128, 1129, 400, 2001, 2000, 2200, 3500] {
        let found = layers_of(&list, sprite_of(id));
        assert!(!found.is_empty(), "sprite {id} drawn");
        assert!(found.iter().all(|&found| found == world), "{id}: {found:?}");
    }
    assert_all_in(&list, world, |command| {
        matches!(
            command,
            DrawCommand::Sprite {
                blend: Blend::Or,
                ..
            }
        )
    });
    assert_all_in(
        &list,
        world,
        |command| matches!(command, DrawCommand::Dot { color, .. } if *color == DEBRIS_COLOR),
    );
    // The brackets, the status bar, the title and help, every text, and
    // the cursor.
    let overlay = DrawLayer::Overlay;
    assert_all_in(&list, overlay, icon_in(10000..=10023));
    assert_all_in(
        &list,
        overlay,
        |command| matches!(command, DrawCommand::Picture { image, .. } if *image == ImageKey::picture(700)),
    );
    assert_all_in(&list, overlay, is_text);
    assert!(texts(&list).contains(&HELP.to_owned()));
    assert_all_in(&list, overlay, sprite_of(651));
    assert!(world_end(&list) < hud_at(&list), "the world under the HUD");
}

#[test]
fn the_message_line_is_overlay_text_drawn_among_the_world() {
    let mut view = flight();
    plot(&mut view, 131);
    view.input(&key(JUMP, true));
    let list = drawn(&view);
    let message = list
        .iter()
        .position(|command| *command == message_drawn(TOO_CLOSE))
        .expect("the message");
    let layers: Vec<DrawLayer> = list.layered().map(|(layer, _)| layer).collect();
    assert_eq!(layers[message], DrawLayer::Overlay);
    assert_eq!(
        list.iter().nth(message - 1),
        Some(&fill_command(hud::message_box(VIEW_SIZE), Color::BLACK)),
        "its box under it"
    );
    assert_eq!(
        layers[message - 1],
        DrawLayer::World,
        "the box in the world"
    );
    assert_eq!(layers[message - 2], DrawLayer::World, "over the stars");
    assert_eq!(layers[message + 1], DrawLayer::World, "under the stellars");
    assert!(sprite_of(1128)(
        list.iter().nth(message + 1).expect("drawn")
    ));
    assert_world_then_overlay(&list);
}

#[test]
fn a_placeholders_box_is_world_and_its_reason_overlay() {
    let sheetless = FakeCatalog {
        sheet: Err("no shän 128 for shïp 128".to_owned()),
        ..catalog()
    };
    let view = FlightView::new(sheetless);
    let list = drawn(&view);
    let mut placeholder = DrawList::new();
    crossed_box(&mut placeholder, VIEW_CENTER, PLACEHOLDER_SIZE, PLACEHOLDER);
    for line in &placeholder {
        assert_all_in(&list, DrawLayer::World, |command| command == line);
    }
    assert_all_in(
        &list,
        DrawLayer::Overlay,
        |command| matches!(command, DrawCommand::Text { text, .. } if text.starts_with("Sprite unavailable")),
    );
    assert_world_then_overlay(&list);
}

#[test]
fn the_course_map_is_overlay_over_the_paused_worlds_scene() {
    let mut view = flight();
    tap(&mut view, MAP_KEY);
    assert!(view.map_open());
    let list = drawn(&view);
    let mut map = DrawList::new();
    view.map.draw(&mut map);
    let map_at = list.len() - map.len();
    assert!(world_end(&list) > 0, "the scene's world is still marked");
    assert!(world_end(&list) <= map_at, "the map over it");
    assert!(
        list.layered()
            .skip(map_at)
            .all(|(layer, _)| layer == DrawLayer::Overlay)
    );
    assert_world_then_overlay(&list);
}

#[test]
fn the_map_in_port_and_a_flight_that_could_not_start_draw_no_world() {
    let mut view = in_the_bar(None);
    view.open_bar_map();
    assert!(view.map_open());
    let list = drawn(&view);
    assert!(!list.is_empty());
    assert_eq!(world_end(&list), 0, "the map alone");

    let failed = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert_eq!(world_end(&drawn(&failed)), 0);
}

#[test]
fn the_stars_are_world_through_a_jump_and_the_jumps_fade_overlay() {
    let [_, jumping] = through_a_jump(flight);
    let list = drawn(&jumping);
    assert_eq!(
        list.layered().next().map(|(layer, _)| layer),
        Some(DrawLayer::World),
        "the stars"
    );
    assert_world_then_overlay(&list);

    let view = fading_in_on_course_to_barnard();
    let list = drawn(&view);
    let (at, _) = fade(&list).expect("fading in");
    let layers: Vec<DrawLayer> = list.layered().map(|(layer, _)| layer).collect();
    assert_eq!(layers[at], DrawLayer::Overlay);
    assert!(world_end(&list) > 0);
    assert_world_then_overlay(&list);
}
