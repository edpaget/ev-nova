//! A stellar whose sheet draws past its frame box, as a 32-bit `rlëD`'s
//! does, through the catalog port and the scene (rendering phase 1). The
//! original draws such a sheet into its 16-bit world wider than its box,
//! while its click box stays the header's size; when the box touches the
//! world's edge, the draw proc's clipped blit cuts it to the box
//! (`_BlitPixieRLETranslucentDrawProc` @0xbb56d-0xbb579,
//! `_BlitPixieRLEClipped` @0xbde9c-0xbe0b3).
//!
//! The scene's tests run over a hand-written [`SystemCatalog`] that hands
//! out a [`StellarSheet`] whose `image_size` is larger than its frame: how
//! much larger is `nova_data`'s rule, and its tests pin it. That the
//! adapter over `GameData` reports the decoded sheet's image size is
//! checked in `system::data`'s tests.

use std::num::NonZeroU16;

use super::camera::Camera;
use super::catalog::{
    StellarAnimation, StellarContents, StellarId, StellarSheet, SystemCatalog, SystemContents,
    SystemId,
};
use super::scene::{SystemScene, draw_stellars, draw_stellars_clipped_to};
use crate::geometry::Bounds;
use crate::{DrawCommand, DrawList, ImageKey, Point, SheetImage};

/// Hands out one canned system.
struct OneSystem(SystemContents);

impl SystemCatalog for OneSystem {
    fn system(&self, id: SystemId) -> SystemContents {
        assert_eq!(id, self.0.id, "the canned system");
        self.0.clone()
    }
}

/// The image of a 2x2 sheet that draws past its box, as a 32-bit `rlëD`
/// does: any image wider than the box will do here.
const WIDE: (u32, u32) = (6, 2);

/// The image of a 2x2 sheet that draws only its box, as a 16-bit `rlëD`
/// does.
const BOXED: (u32, u32) = (2, 2);

/// `rlëD` 1001: one 2x2 frame, its image `image_size`.
fn sheet(image_size: (u32, u32)) -> StellarSheet {
    StellarSheet {
        image: SheetImage::Rled(1001),
        frames: NonZeroU16::MIN,
        frame_width: 2,
        frame_height: 2,
        image_size,
        eight_bit: false,
    }
}

/// Sol (131) holding stellar 140 at (0, 0), whose sheet's image is
/// `image_size` ([`WIDE`] or [`BOXED`]).
fn sol(image_size: (u32, u32)) -> SystemScene {
    let catalog = OneSystem(SystemContents {
        id: SystemId(131),
        name: "Sol".to_owned(),
        stellars: vec![StellarContents {
            id: StellarId(140),
            name: "Stellar 140".to_owned(),
            x: 0,
            y: 0,
            sprite: Ok(sheet(image_size)),
            dead_sprite: None,
            animation: StellarAnimation::default(),
            stand_in: None,
        }],
        type_zero_sprite: Ok(StellarSheet {
            image: SheetImage::Rled(1000),
            frames: NonZeroU16::MIN,
            frame_width: 6,
            frame_height: 6,
            image_size: (6, 6),
            eight_bit: false,
        }),
        problems: Vec::new(),
    });
    SystemScene::load(&catalog, SystemId(131))
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
    let scene = sol(WIDE);
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
    let (sixteen, _) = unclipped(&sol(BOXED));
    assert_eq!(sprite, sixteen);
}

#[test]
fn a_32_bit_stellar_touching_the_worlds_edge_is_cut_to_its_box() {
    let scene = sol(WIDE);
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
    let scene = sol(BOXED);
    let (sprite, frame_box) = unclipped(&scene);
    assert_eq!(in_world(&scene, clip_from(frame_box.min.x + 1.0)), [sprite]);
}

/// The rescue's closing repaints the stellars into no world, each command
/// cut to the play area, as `_SWUpdateSpriteWorld` (@0x6c0d) draws them
/// through the same clipped draw proc: an overhanging stellar at the play
/// area's edge is cut to its box there too.
#[test]
fn the_rescue_repaint_cuts_an_overhanging_stellar_to_its_box_at_the_play_areas_edge() {
    let scene = sol(WIDE);
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
