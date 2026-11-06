//! A stellar whose sheet is a 32-bit `rlëD`, through the game data, the
//! catalog and the scene (rendering phase 1). The original draws such a
//! sheet into its 16-bit world twice its width from the box's left edge,
//! while its click box stays the header's size; when the box touches the
//! world's edge, the draw proc's clipped blit cuts it to the box
//! (`_BlitPixieRLETranslucentDrawProc` @0xbb56d-0xbb579,
//! `_BlitPixieRLEClipped` @0xbde9c-0xbe0b3).

use std::num::NonZeroU16;

use nova_data::graphics::RLED;
use nova_data::graphics::fixture::RledBuilder;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fixture::one_file;
use nova_data::{GameData, Record};
use nova_rsrc::ResType;
use nova_rsrc::fixture::ForkBuilder;

use super::camera::Camera;
use super::catalog::{StellarSheet, SystemCatalog, SystemId};
use super::scene::{SystemScene, draw_stellars, draw_stellars_clipped_to};
use crate::geometry::Bounds;
use crate::{DrawCommand, DrawList, ImageKey, Point, SheetImage};

fn store(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
    let fork = resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
            fork.resource(*ty, *id, None, data)
        })
        .build()
        .bytes;
    one_file(fork).load()
}

fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (i, value) in values.iter().enumerate() {
        let start = at + 2 * i;
        bytes[start..start + 2].copy_from_slice(&value.to_be_bytes());
    }
}

/// A `sÿst` holding these stellars.
fn system(stellars: &[i16]) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, stellars);
    put_i16s(&mut bytes, 0x66, &[-1]);
    bytes
}

/// A `spöb` at (0, 0) of graphic type `graphic_type`.
fn stellar(graphic_type: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[graphic_type]);
    put_i16s(&mut bytes, 0x240, &[-1]);
    bytes
}

/// A `spïn` naming sprites `sprites`, `size` x `size` each, one tile.
fn spin(sprites: i16, size: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[sprites, -1, size, size, 1, 1]);
    bytes
}

/// Sol (131) holding stellar 140 of `Type` 1, whose `spïn` 1001 names
/// `rlëD` 1001 of `depth`: one 2x2 frame, each row two red pixels at that
/// depth's width (32-bit `00 00 7C 00`, or 16-bit `0x7C00`). Graphic type
/// 0 is a 16-bit 6x6 sheet.
fn sol(depth: u16) -> GameData {
    let type_one = match depth {
        32 => RledBuilder::new(2, 2)
            .depth(32)
            .count_lines()
            .frame(|f| {
                f.line()
                    .pixels32(&[0x7C00, 0x7C00])
                    .line()
                    .pixels32(&[0x7C00, 0x7C00])
            })
            .build(),
        _ => RledBuilder::new(2, 2)
            .frame(|f| f.line().pixels(&[0x7C00; 2]).line().pixels(&[0x7C00; 2]))
            .build(),
    };
    store(&[
        (System::TYPE, 131, system(&[140])),
        (Stellar::TYPE, 140, stellar(1)),
        (Spin::TYPE, 1000, spin(1000, 6)),
        (
            RLED,
            1000,
            RledBuilder::new(6, 6).frame(|f| f.line().skip(6)).build(),
        ),
        (Spin::TYPE, 1001, spin(1001, 2)),
        (RLED, 1001, type_one),
    ])
}

fn camera() -> Camera {
    Camera::centred_on(Point::default())
}

/// The stellar's sprite, drawn with no world clip, and its box: the 2x2
/// square centred where the sprite is.
fn unclipped(scene: &SystemScene) -> (DrawCommand, Bounds) {
    let mut list = DrawList::new();
    draw_stellars(&mut list, scene, &camera(), None);
    let sprite = list
        .iter()
        .find(|command| matches!(command, DrawCommand::Sprite { .. }))
        .cloned()
        .expect("a sprite");
    let DrawCommand::Sprite { center, .. } = sprite else {
        unreachable!()
    };
    let corner = Point::new(center.x - 1.0, center.y - 1.0);
    (sprite, Bounds::at(corner, 2.0, 2.0))
}

/// What `draw_stellars` draws of `scene` in a world clipped to `clip`,
/// but for the names.
fn in_world(scene: &SystemScene, clip: Bounds) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    list.world_in(clip, |list| draw_stellars(list, scene, &camera(), None));
    list.iter()
        .filter(|command| !matches!(command, DrawCommand::Text { .. }))
        .cloned()
        .collect()
}

/// A clip whose left edge is `left`, reaching far right and down.
fn clip_from(left: f32) -> Bounds {
    Bounds {
        min: Point::new(left, -10_000.0),
        max: Point::new(10_000.0, 10_000.0),
    }
}

/// A clip whose bottom-right corner is `max`, reaching far left and up.
fn clip_to(max: Point) -> Bounds {
    Bounds {
        min: Point::new(-10_000.0, -10_000.0),
        max,
    }
}

#[test]
fn a_stellar_whose_sheet_is_32_bit_draws_it_at_the_drawn_size_while_its_box_keeps_the_header_size()
{
    let data = sol(32);
    let contents = data.system(SystemId(131));
    assert_eq!(contents.problems, Vec::<String>::new());
    assert_eq!(
        contents.stellars[0].sprite,
        Ok(StellarSheet {
            image: SheetImage::Rled(1001),
            frames: NonZeroU16::MIN,
            frame_width: 2,
            frame_height: 2,
            image_size: (6, 2),
        })
    );
    let scene = SystemScene::load(&data, SystemId(131));
    let stellar = &scene.stellars()[0];
    assert_eq!(
        stellar.shown_image(),
        Some((ImageKey::sprite(1001, 0), (2.0, 2.0))),
        "the box clicks and places it"
    );
    // With no world clip, it is drawn plainly, centred on its box: the
    // image is padded so that lands it where the original's blit does.
    let (sprite, frame_box) = unclipped(&scene);
    assert_eq!(
        sprite,
        DrawCommand::Sprite {
            image: ImageKey::sprite(1001, 0),
            center: frame_box.center(),
            tint: crate::Color::WHITE,
            blend: crate::Blend::Normal,
        }
    );
    // The same sprite stands where a 16-bit sheet of the size would.
    let (sixteen, _) = unclipped(&SystemScene::load(&sol(16), SystemId(131)));
    assert_eq!(sprite, sixteen);
}

#[test]
fn a_32_bit_stellar_touching_the_worlds_edge_is_cut_to_its_box() {
    let scene = SystemScene::load(&sol(32), SystemId(131));
    let (sprite, frame_box) = unclipped(&scene);
    let cut = DrawCommand::Clipped {
        clip: frame_box,
        command: Box::new(sprite.clone()),
    };
    // Straddling the world's left edge.
    assert_eq!(
        in_world(&scene, clip_from(frame_box.min.x + 1.0)),
        std::slice::from_ref(&cut)
    );
    // Wholly inside, edges included: the plain blit.
    assert_eq!(
        in_world(&scene, clip_from(frame_box.min.x)),
        std::slice::from_ref(&sprite)
    );
    // Wholly outside: cut to the box, which the world's clip leaves
    // nothing of.
    assert_eq!(
        in_world(&scene, clip_from(frame_box.max.x + 5.0)),
        std::slice::from_ref(&cut)
    );
    // Straddling the world's top edge.
    let top = Bounds {
        min: Point::new(-10_000.0, frame_box.min.y + 1.0),
        ..clip_from(-10_000.0)
    };
    assert_eq!(in_world(&scene, top), std::slice::from_ref(&cut));
    // Straddling the world's right edge.
    let right = clip_to(Point::new(frame_box.max.x - 1.0, 10_000.0));
    assert_eq!(in_world(&scene, right), std::slice::from_ref(&cut));
    // Straddling the world's bottom edge.
    let bottom = clip_to(Point::new(10_000.0, frame_box.max.y - 1.0));
    assert_eq!(in_world(&scene, bottom), std::slice::from_ref(&cut));
    // Wholly inside, its right and bottom edges on the world's: the plain
    // blit.
    assert_eq!(in_world(&scene, clip_to(frame_box.max)), [sprite]);
}

#[test]
fn a_16_bit_stellar_at_the_edge_is_drawn_as_before() {
    let scene = SystemScene::load(&sol(16), SystemId(131));
    let (sprite, frame_box) = unclipped(&scene);
    assert_eq!(in_world(&scene, clip_from(frame_box.min.x + 1.0)), [sprite]);
}

/// The rescue's closing repaints the stellars into no world, each command
/// cut to the play area, as `_SWUpdateSpriteWorld` (@0x6c0d) draws them
/// through the same clipped draw proc: an overhanging stellar at the play
/// area's edge is cut to its box there too.
#[test]
fn the_rescue_repaint_cuts_an_overhanging_stellar_to_its_box_at_the_play_areas_edge() {
    let scene = SystemScene::load(&sol(32), SystemId(131));
    let (sprite, frame_box) = unclipped(&scene);
    let play = clip_from(frame_box.min.x + 1.0);
    let mut list = DrawList::new();
    draw_stellars_clipped_to(&mut list, &scene, &camera(), None, play);
    let drawn: Vec<_> = list
        .iter()
        .filter(|command| {
            !matches!(command, DrawCommand::Clipped { command, .. } if matches!(**command, DrawCommand::Text { .. }))
        })
        .cloned()
        .collect();
    assert_eq!(
        drawn,
        [DrawCommand::Clipped {
            clip: play,
            command: Box::new(DrawCommand::Clipped {
                clip: frame_box,
                command: Box::new(sprite.clone()),
            }),
        }]
    );
    // Its commands are the overlay's, as the repaint's space area is.
    assert!(
        list.layered()
            .all(|(layer, _)| layer == crate::DrawLayer::Overlay)
    );
    assert_eq!(list.world_clip(), None);
    // Wholly inside the play area, it is drawn whole.
    let mut list = DrawList::new();
    let play = clip_from(frame_box.min.x);
    draw_stellars_clipped_to(&mut list, &scene, &camera(), None, play);
    assert!(list.iter().any(|command| *command
        == DrawCommand::Clipped {
            clip: play,
            command: Box::new(sprite.clone()),
        }));
}
