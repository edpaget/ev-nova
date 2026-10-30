//! The cargo boxes, asteroids, stellar sprites and escape pods murked as
//! `_HandleSpriteMurkiness` (@0x2b42f-0x2b513) murks them, at their
//! positions seen from the player's ship in the frame's effective murk
//! (rdm:phase/play-feel-4/phase-17-murk-other-sprites). On [`FakeFlight`]:
//! each frame scripted, the pod raised on the view's own effects.

use super::faked_world::{asteroid_at, box_at, raise, world, world_list};
use super::missions::rocky;
use super::*;
use crate::flight::murk::{murk_tint, sprite_murk};

/// The murk the frames are in: the most there is.
const MURK: u8 = 100;

/// Where the sprites are put, far enough from the player to murk.
const FAR: Vec2 = Vec2::new(150.0, -100.0);

/// The tint of the first sprite of `rlëD`/`spïn` `id` drawn in `list`.
fn tint_of(list: &DrawList, id: i16) -> Color {
    list.iter()
        .find_map(|command| match unclipped(command) {
            DrawCommand::Sprite { image, tint, .. } if image.id == id => Some(*tint),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{id} not drawn: {list:?}"))
}

/// A frame in Sol in [`MURK`], the player at `player`, a cargo box and an
/// asteroid at `at`.
fn murky(player: Vec2, at: Vec2) -> FlightFrame {
    FlightFrame {
        asteroids: vec![asteroid_at(at)],
        cargo_boxes: vec![box_at(at)],
        effective_murk: MURK,
        ..in_sol_with(vec![the_player_at(player, 0.0)])
    }
}

/// The view over [`murky`], a pod adrift from `at`.
fn view_of(player: Vec2, at: Vec2) -> Faked {
    let mut view = world(rocky(trading()), murky(player, at));
    raise(
        &mut view,
        &[CombatEvent::PodEjected {
            ship: ShipRef::Npc(NpcId(9)),
            at,
            velocity: Vec2::ZERO,
        }],
    );
    view
}

/// The darkening a sprite at `at` has seen from `player` in [`MURK`].
fn murked(at: Vec2, player: Vec2) -> Color {
    let level = sprite_murk(at, player, MURK);
    assert!(level > 0, "{at:?} is murky from {player:?}");
    murk_tint(Color::WHITE, level)
}

#[test]
fn a_murky_cargo_box_far_from_the_player_is_darkened_and_one_beside_it_is_not() {
    let list = world_list(&view_of(Vec2::ZERO, FAR));
    assert_eq!(tint_of(&list, 500), murked(FAR, Vec2::ZERO));
    let beside = world_list(&view_of(FAR, FAR));
    assert_eq!(tint_of(&beside, 500), Color::WHITE);
}

#[test]
fn a_murky_asteroid_far_from_the_player_is_darkened_and_one_beside_it_is_not() {
    let list = world_list(&view_of(Vec2::ZERO, FAR));
    assert_eq!(tint_of(&list, 1800), murked(FAR, Vec2::ZERO));
    let beside = world_list(&view_of(FAR, FAR));
    assert_eq!(tint_of(&beside, 1800), Color::WHITE);
}

#[test]
fn a_murky_escape_pod_far_from_the_player_is_darkened_and_one_beside_it_is_not() {
    let view = view_of(Vec2::ZERO, FAR);
    let pod = view.effects.pods()[0];
    let at = Vec2::new(pod.at.x, pod.at.y);
    assert_eq!(tint_of(&world_list(&view), 1095), murked(at, Vec2::ZERO));
    let beside = view_of(at, FAR);
    let pod = beside.effects.pods()[0];
    let level = sprite_murk(Vec2::new(pod.at.x, pod.at.y), at, MURK);
    assert_eq!(level, 0, "beside the player, as the push is the same");
    assert_eq!(tint_of(&world_list(&beside), 1095), Color::WHITE);
}

#[test]
fn a_murky_stellar_sprite_far_from_the_player_is_darkened_and_one_beside_it_is_not() {
    let view = view_of(Vec2::ZERO, FAR);
    let scene = view.scene().expect("a scene");
    let (stellar, image) = scene
        .stellars()
        .iter()
        .find_map(|stellar| {
            let (image, _) = scene.shown_frame(stellar.id)?;
            Some((stellar.position, image))
        })
        .expect("a stellar sprite");
    let at = Vec2::new(stellar.x, stellar.y);
    let shown = |player: Vec2| {
        let view = view_of(player, FAR);
        let list = world_list(&view);
        list.iter()
            .find_map(|command| match unclipped(command) {
                DrawCommand::Sprite {
                    image: drawn, tint, ..
                } if *drawn == image => Some(*tint),
                _ => None,
            })
            .expect("the stellar drawn")
    };
    let player = Vec2::new(at.x + 300.0, at.y);
    assert_eq!(shown(player), murked(at, player));
    assert_eq!(shown(at), Color::WHITE, "beside the player");
}
