//! The ships drawn, and boxed, at whole pixels from the player, as
//! `_HandleShipDisplay` places each sprite (@0x2cd17-0x2d083; rdm
//! play-feel-4 phase 12). The rule is the camera's
//! ([`Camera::sprite_top_left`](crate::system::camera::Camera::sprite_top_left),
//! pinned in its own tests); these pin the wiring: every ship sprite goes
//! through it, and the click and hover box is the rect the hull is drawn
//! in, in the same frame.

use super::faked_world::{shows, world_list};
use super::ship_drawing::{NPC_0, display, flying_with, npc_sheet, redrawn, shown};
use super::*;
use nova_sim::DrawnShips;
use nova_sim::hover::in_box;
use nova_sim::navigation::SpriteRect;

/// Where the player is: off a whole pixel on both axes.
const PLAYER: Vec2 = Vec2::new(0.25, -0.5);

/// The NPC's sheet, `rlëD` 2001, of `size` frames.
fn npc_of((frame_width, frame_height): (u32, u32)) -> ShipSheet {
    ShipSheet {
        frame_width,
        frame_height,
        ..npc_sheet()
    }
}

/// A flight whose NPC 0, drawn with `sheet`, is `offset` from the player,
/// who is at [`PLAYER`]; neither moving, so each is drawn where the frame
/// has it.
fn placed(sheet: ShipSheet, offset: Vec2) -> Faked {
    let catalog = shown(display(0, 1, 0), display(0, 1, 0), Some(sheet));
    let mut view = flying_with(catalog, 0.0);
    let mut frame = view.frame.clone();
    frame.ships[0].position = PLAYER;
    frame.ships[1].position = PLAYER + offset;
    shows(&mut view, frame);
    view.previous = view.frame.player().place();
    view.npc_previous.clear();
    view
}

/// The top left, on screen, of the sprite of `rlëD` `image` drawn in
/// `list`, `size` in size.
#[allow(clippy::cast_precision_loss)]
fn top_left_of(list: &DrawList, image: i16, (width, height): (u32, u32)) -> Point {
    let found: Vec<Point> = sprites(list)
        .into_iter()
        .filter(|(key, _)| key.id == image)
        .map(|(_, at)| at)
        .collect();
    let [centre] = found[..] else {
        panic!("one sprite of {image}: {found:?}");
    };
    at(
        centre.x - width as f32 / 2.0,
        centre.y - height as f32 / 2.0,
    )
}

/// The rect, in world coordinates, the sprite of `rlëD` `image` is drawn
/// in by `view`, `size` in size.
fn drawn_rect(view: &Faked, image: i16, size: (u32, u32)) -> SpriteRect {
    let top_left = top_left_of(&world_list(view), image, size);
    let world = view.camera().screen_to_world(top_left);
    SpriteRect {
        left: world.x,
        top: world.y,
        width: size.0,
        height: size.1,
    }
}

/// Whether `p`'s coordinates are whole.
fn whole(p: Point) -> bool {
    p.x.fract() == 0.0 && p.y.fract() == 0.0
}

#[test]
fn an_npc_is_drawn_at_whole_pixels_from_the_player_its_odd_pixel_right_and_below() {
    // The engine's anchor, (415, 384).
    for (size, offset, top_left) in [
        // (10.7, -20.3) truncates to (10, -20); 41 x 40 halves to (20, 20).
        ((41, 40), Vec2::new(10.7, -20.3), at(405.0, 344.0)),
        // (-10.7, 20.3) truncates toward zero to (-10, 20).
        ((41, 40), Vec2::new(-10.7, 20.3), at(385.0, 384.0)),
        // 40 x 41: the axes are not swapped.
        ((40, 41), Vec2::new(10.7, -20.3), at(405.0, 344.0)),
        ((40, 41), Vec2::new(-10.7, 20.3), at(385.0, 384.0)),
        ((39, 38), Vec2::new(10.7, -20.3), at(406.0, 345.0)),
    ] {
        let view = placed(npc_of(size), offset);
        let drawn = top_left_of(&world_list(&view), 2001, size);
        assert_eq!(drawn, top_left, "{size:?} {offset:?}");
        assert!(whole(drawn), "{size:?} {offset:?}");
    }
}

#[test]
fn the_players_odd_sized_ship_is_drawn_half_a_pixel_right_of_and_below_the_anchor() {
    let mut catalog = shown(display(0, 1, 0), display(0, 1, 0), None);
    catalog.sheet = Ok(ShipSheet {
        frame_width: 41,
        frame_height: 39,
        ..sheet()
    });
    let mut view = flying_with(catalog, 0.0);
    redrawn(&mut view, 0, |player| player.position = PLAYER);
    view.previous = view.frame.player().place();
    let centre = sprites(&world_list(&view))
        .into_iter()
        .find(|(key, _)| key.id == 2000)
        .expect("the hull")
        .1;
    // (415, 384) less (20, 19), plus (20.5, 19.5).
    assert_eq!(centre, at(415.5, 384.5));
    assert_eq!(view.camera().anchor(), at(415.0, 384.0));
}

#[test]
fn an_npcs_box_is_the_rect_its_hull_is_drawn_in() {
    for (size, offset) in [
        ((41, 40), Vec2::new(10.7, -20.3)),
        ((40, 41), Vec2::new(-10.7, 20.3)),
        ((61, 63), Vec2::new(-30.6, -12.9)),
    ] {
        let view = placed(npc_of(size), offset);
        let boxed = view.drawn_boxes().npc(NPC_0).expect("boxed");
        assert_eq!(
            boxed.rect,
            drawn_rect(&view, 2001, size),
            "{size:?} {offset:?}"
        );
    }
}

#[test]
fn the_players_box_is_the_rect_its_hull_is_drawn_in() {
    let mut catalog = shown(display(0, 1, 0), display(0, 1, 0), None);
    catalog.sheet = Ok(ShipSheet {
        frame_width: 41,
        frame_height: 39,
        ..sheet()
    });
    let mut view = flying_with(catalog, 0.0);
    redrawn(&mut view, 0, |player| player.position = PLAYER);
    view.previous = view.frame.player().place();
    let boxed = view.drawn_boxes().player.expect("boxed");
    assert_eq!(boxed.rect, drawn_rect(&view, 2000, (41, 39)));
}

#[test]
fn a_player_held_in_another_types_frames_is_boxed_at_whole_pixels_in_them() {
    // Which frames is nova-sim's (`a_hidden_player_is_framed_by_its_last_shown_ship`).
    let mut catalog = shown(display(0, 1, 0), display(0, 1, 0), Some(npc_of((41, 39))));
    catalog.sheet = Ok(sheet());
    let mut view = flying_with(catalog, 0.0);
    redrawn(&mut view, 0, |player| {
        player.position = PLAYER;
        player.framed = Some(ShipId(129));
    });
    view.previous = view.frame.player().place();
    let boxed = view.drawn_boxes().player.expect("boxed");
    let camera = view.camera();
    let top_left = camera.world_to_screen(at(boxed.rect.left, boxed.rect.top));
    // The anchor less (20, 19).
    assert_eq!(top_left, at(395.0, 365.0));
    assert_eq!((boxed.rect.width, boxed.rect.height), (41, 39));
}

#[test]
fn a_click_and_a_hover_send_the_box_the_hull_is_drawn_in() {
    let size = (61, 63);
    let mut view = placed(npc_of(size), Vec2::new(-30.6, -12.9));
    let rect = drawn_rect(&view, 2001, size);
    let over = view
        .camera()
        .world_to_screen(at(rect.left + 30.0, rect.top + 30.0));
    view.click_in_space(over);
    let [FlightCommand::ClickInSpace { drawn, .. }] = sent(&view) else {
        panic!("{:?}", sent(&view));
    };
    let boxed = drawn.npc(NPC_0).expect("boxed");
    assert_eq!(boxed.rect, rect, "clicked");
    view.cursor_input(&Input::PointerMoved(over));
    super::faked_flight::ticked(&mut view, 1);
    let (_, drawn) = view
        .sim
        .asks()
        .iter()
        .rev()
        .find_map(|ask| match ask {
            FlightCommand::Hover { at, drawn } => Some((*at, drawn.clone())),
            _ => None,
        })
        .expect("hovered");
    let rect = drawn_rect(&view, 2001, size);
    assert_eq!(drawn.npc(NPC_0).expect("boxed").rect, rect, "hovered");
}

#[test]
fn a_pointer_just_inside_the_drawn_right_edge_is_over_the_ship_and_one_on_it_is_not() {
    // 61 wide: past the margin's 47, so the box is the rect alone.
    let size = (61, 63);
    let view = placed(npc_of(size), Vec2::new(10.7, -20.3));
    let rect = drawn_rect(&view, 2001, size);
    let boxed = view.drawn_boxes().npc(NPC_0).expect("boxed");
    let camera = view.camera();
    let right = camera.world_to_screen(at(rect.left, rect.top)).x + 61.0;
    let middle = camera.world_to_screen(at(rect.left, rect.top)).y + 30.0;
    let world = |x: f32| {
        let p = camera.screen_to_world(at(x, middle));
        Vec2::new(p.x, p.y)
    };
    assert!(in_box(world(right - 1.0), boxed), "a pixel inside");
    assert!(!in_box(world(right), boxed), "on the edge");
}

#[test]
fn a_stale_slots_box_sits_at_its_truncated_offset_from_the_player() {
    // Where the slot was last laid out is nova-sim's
    // (`a_ship_its_slot_has_not_displayed_is_hovered_where_the_slot_last_was`).
    let size = (41, 40);
    let mut view = placed(npc_of(size), Vec2::new(200.0, 0.0));
    redrawn(&mut view, 1, |npc| {
        npc.hover = nova_sim::HoverPlace::Stale(Vec2::new(30.7, -12.9));
    });
    view.npc_previous.clear();
    let boxed = view.drawn_boxes().npc(NPC_0).expect("boxed");
    let top_left = view
        .camera()
        .world_to_screen(at(boxed.rect.left, boxed.rect.top));
    // (415, 384) + (30, -12) − (20, 20).
    assert_eq!(top_left, at(425.0, 352.0));
    assert_eq!((boxed.rect.width, boxed.rect.height), size);
}

#[test]
fn a_held_alt_layer_stays_at_the_whole_pixel_place_it_was_drawn_at() {
    use nova_sim::catalog::ShipDisplay;
    use nova_sim::combat::ship_frames::Alt;
    let npc = ShipDisplay {
        alt_sets: 2,
        alt_image: true,
        ..display(0x0020, 1, 0)
    };
    let sheet = ShipSheet {
        alt: Some(LayerSheet {
            frame_width: 31,
            frame_height: 29,
            ..layer(2401, 144)
        }),
        ..npc_sheet()
    };
    let catalog = shown(display(0, 1, 0), npc, Some(sheet));
    let mut view = flying_with(catalog, 0.0);
    let mut frame = view.frame.clone();
    frame.ships[0].position = PLAYER;
    frame.ships[1].position = PLAYER + Vec2::new(10.7, -20.3);
    frame.ships[1].drawn.alt = Alt::Drawn(0);
    shows(&mut view, frame);
    view.previous = view.frame.player().place();
    view.npc_previous.clear();
    let live = top_left_of(&world_list(&view), 2401, (31, 29));
    // (415, 384) + (10, -20) − (15, 14).
    assert_eq!(live, at(410.0, 350.0));
    redrawn(&mut view, 1, |npc| npc.drawn.alt = Alt::Held);
    redrawn(&mut view, 0, |player| {
        player.position = PLAYER + Vec2::new(0.0, -60.0);
    });
    view.previous = view.frame.player().place();
    assert_eq!(
        top_left_of(&world_list(&view), 2401, (31, 29)),
        live,
        "held"
    );
}
