//! A stellar whose sheet can draw otherwise at the world's edge than cut
//! to its box (rendering phase 17), through the catalog port and the
//! scene. When the world's edge cuts a frame's box, the draw proc hands
//! the frame to `_BlitPixieRLEClipped` (@0xbde9c-0xbe0b8), whose row skip
//! and run restart depend on how much of the box the world cuts at the
//! top and the left. The scene asks for that draw by key
//! ([`ImageKind::RledClipped`](crate::ImageKind::RledClipped)) for a
//! sheet marked `clip_quirks`, and cuts it to the box as it cuts an
//! overhanging sheet.
//!
//! The scene's tests run over a hand-written [`SystemCatalog`] whose sheet
//! is marked or not; what the clipped draw holds is `nova_data`'s rule,
//! and its tests pin it. That the adapter over `GameData` marks a sheet so
//! is checked in `system::data`'s tests.

use std::num::NonZeroU16;

use nova_sim::Vec2;

use super::camera::Camera;
use super::catalog::{
    StellarAnimation, StellarContents, StellarId, StellarSheet, SystemCatalog, SystemContents,
    SystemId,
};
use super::scene::{SystemScene, draw_stellars, draw_stellars_clipped_to};
use crate::flight::murk::SeenMurk;
use crate::geometry::Bounds;
use crate::{Blend, Color, Colorize, DrawCommand, DrawList, ImageKey, Point, SheetImage};

/// Hands out one canned system.
struct OneSystem(SystemContents);

impl SystemCatalog for OneSystem {
    fn system(&self, id: SystemId) -> SystemContents {
        assert_eq!(id, self.0.id, "the canned system");
        self.0.clone()
    }
}

/// `rlëD` 1001: one 8x6 frame, marked `clip_quirks` or not, its image
/// `image_size`.
fn sheet(clip_quirks: bool, image_size: (u32, u32)) -> StellarSheet {
    StellarSheet {
        image: SheetImage::Rled(1001),
        frames: NonZeroU16::MIN,
        frame_width: 8,
        frame_height: 6,
        image_size,
        eight_bit: false,
        clip_quirks,
    }
}

/// The box's own size: a sheet drawing only its box.
const BOXED: (u32, u32) = (8, 6);

/// Sol (131) holding stellar 140 at (0, 0) with `sprite`.
fn sol(sprite: StellarSheet) -> SystemScene {
    let catalog = OneSystem(SystemContents {
        id: SystemId(131),
        name: "Sol".to_owned(),
        stellars: vec![StellarContents {
            id: StellarId(140),
            name: "Stellar 140".to_owned(),
            x: 0,
            y: 0,
            sprite: Ok(sprite),
            dead_sprite: None,
            animation: StellarAnimation::default(),
            stand_in: None,
        }],
        type_zero_sprite: Err("none".to_owned()),
        problems: Vec::new(),
    });
    SystemScene::load(&catalog, SystemId(131))
}

fn quirky() -> SystemScene {
    sol(sheet(true, BOXED))
}

fn camera() -> Camera {
    Camera::centred_on(Point::default())
}

/// The stellar's sprite, drawn with no world clip, and its box.
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
    let corner = Point::new(center.x - 4.0, center.y - 3.0);
    (sprite, Bounds::at(corner, 8.0, 6.0))
}

/// What `draw_stellars` draws of `scene` through `seen` in a world clipped
/// to `clip`, but for the names.
fn in_world(scene: &SystemScene, clip: Bounds, seen: Option<SeenMurk>) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    list.world_in(clip, |list| draw_stellars(list, scene, &camera(), seen));
    list.iter()
        .filter(|command| !matches!(command, DrawCommand::Text { .. }))
        .cloned()
        .collect()
}

/// A clip with `min` its top-left corner, reaching far right and down.
fn clip_from(min: Point) -> Bounds {
    Bounds {
        min,
        max: Point::new(10_000.0, 10_000.0),
    }
}

/// A clip with `max` its bottom-right corner, reaching far left and up.
fn clip_to(max: Point) -> Bounds {
    Bounds {
        min: Point::new(-10_000.0, -10_000.0),
        max,
    }
}

/// `sprite`, a plain sprite command, drawing `image` instead.
fn drawing(sprite: &DrawCommand, image: ImageKey) -> DrawCommand {
    let DrawCommand::Sprite {
        center,
        tint,
        blend,
        ..
    } = *sprite
    else {
        panic!("not a plain sprite: {sprite:?}")
    };
    DrawCommand::Sprite {
        image,
        center,
        tint,
        blend,
    }
}

/// `command` cut to `clip`.
fn cut_to(clip: Bounds, command: DrawCommand) -> DrawCommand {
    DrawCommand::Clipped {
        clip,
        command: Box::new(command),
    }
}

#[test]
fn a_quirky_sheet_straddling_the_worlds_left_and_top_draws_clipped_by_its_cut() {
    let scene = quirky();
    let (sprite, frame_box) = unclipped(&scene);
    let world = clip_from(Point::new(frame_box.min.x + 3.0, frame_box.min.y + 2.0));
    let clipped = drawing(&sprite, ImageKey::clipped_sprite(1001, 0, (2, 3)));
    assert_eq!(in_world(&scene, world, None), [cut_to(frame_box, clipped)]);
    // Only the left.
    let world = clip_from(Point::new(frame_box.min.x + 1.0, -10_000.0));
    let clipped = drawing(&sprite, ImageKey::clipped_sprite(1001, 0, (0, 1)));
    assert_eq!(in_world(&scene, world, None), [cut_to(frame_box, clipped)]);
}

#[test]
fn the_cut_is_rounded_to_whole_pixels_and_kept_to_the_box() {
    let scene = quirky();
    let (sprite, frame_box) = unclipped(&scene);
    let world = clip_from(Point::new(frame_box.min.x + 2.6, frame_box.min.y + 1.4));
    let clipped = drawing(&sprite, ImageKey::clipped_sprite(1001, 0, (1, 3)));
    assert_eq!(in_world(&scene, world, None), [cut_to(frame_box, clipped)]);
    // Wholly outside: the whole box cut.
    let world = clip_from(Point::new(frame_box.max.x + 5.0, frame_box.max.y + 5.0));
    let clipped = drawing(&sprite, ImageKey::clipped_sprite(1001, 0, (6, 8)));
    assert_eq!(in_world(&scene, world, None), [cut_to(frame_box, clipped)]);
}

#[test]
fn a_quirky_sheet_cut_only_at_the_right_or_bottom_draws_clipped_uncut() {
    let scene = quirky();
    let (sprite, frame_box) = unclipped(&scene);
    let clipped = cut_to(
        frame_box,
        drawing(&sprite, ImageKey::clipped_sprite(1001, 0, (0, 0))),
    );
    let right = clip_to(Point::new(frame_box.max.x - 1.0, 10_000.0));
    assert_eq!(
        in_world(&scene, right, None),
        std::slice::from_ref(&clipped)
    );
    let bottom = clip_to(Point::new(10_000.0, frame_box.max.y - 1.0));
    assert_eq!(in_world(&scene, bottom, None), [clipped]);
}

#[test]
fn a_quirky_sheet_wholly_inside_the_world_draws_plainly() {
    let scene = quirky();
    let (sprite, frame_box) = unclipped(&scene);
    assert_eq!(in_world(&scene, clip_from(frame_box.min), None), [sprite]);
    let (sprite, _) = unclipped(&scene);
    assert_eq!(in_world(&scene, clip_to(frame_box.max), None), [sprite]);
}

#[test]
fn a_sheet_without_quirks_draws_as_before_at_the_edge() {
    // Drawing only its box: plainly, the world's clip cutting it.
    let scene = sol(sheet(false, BOXED));
    let (sprite, frame_box) = unclipped(&scene);
    let world = clip_from(Point::new(frame_box.min.x + 1.0, frame_box.min.y + 1.0));
    assert_eq!(in_world(&scene, world, None), [sprite]);
    // Drawing past its box: cut to the box.
    let scene = sol(sheet(false, (24, 6)));
    let (sprite, frame_box) = unclipped(&scene);
    let world = clip_from(Point::new(frame_box.min.x + 1.0, frame_box.min.y + 1.0));
    assert_eq!(in_world(&scene, world, None), [cut_to(frame_box, sprite)]);
}

#[test]
fn a_quirky_sheet_that_draws_past_its_box_draws_clipped_too() {
    let scene = sol(sheet(true, (24, 6)));
    let (sprite, frame_box) = unclipped(&scene);
    let world = clip_from(Point::new(frame_box.min.x + 1.0, frame_box.min.y));
    let clipped = drawing(&sprite, ImageKey::clipped_sprite(1001, 0, (0, 1)));
    assert_eq!(in_world(&scene, world, None), [cut_to(frame_box, clipped)]);
}

#[test]
fn a_murked_quirky_sheet_keeps_its_colorize() {
    let scene = quirky();
    let (_, frame_box) = unclipped(&scene);
    let murky = SeenMurk {
        viewer: Vec2::new(-200.0, 0.0),
        murk: 50,
        colour: 0x1081,
    };
    let world = clip_from(Point::new(frame_box.min.x + 1.0, frame_box.min.y));
    let [DrawCommand::Clipped { clip, command }] = &in_world(&scene, world, Some(murky))[..] else {
        panic!("one clipped command")
    };
    assert_eq!(*clip, frame_box);
    assert!(
        matches!(
            **command,
            DrawCommand::Colorized { image, colorize, tint: Color::WHITE, blend: Blend::Normal, .. }
                if image == ImageKey::clipped_sprite(1001, 0, (0, 1))
                    && colorize == Colorize::toward_pixel(24, 0x1081)
        ),
        "{command:?}"
    );
}

/// The rescue's closing repaint draws through the same clipped draw proc.
#[test]
fn the_rescue_repaint_draws_a_quirky_sheet_clipped_by_its_cut() {
    let scene = quirky();
    let (sprite, frame_box) = unclipped(&scene);
    let play = clip_from(Point::new(frame_box.min.x + 2.0, -10_000.0));
    let mut list = DrawList::new();
    draw_stellars_clipped_to(&mut list, &scene, &camera(), None, play);
    let clipped = drawing(&sprite, ImageKey::clipped_sprite(1001, 0, (0, 2)));
    assert!(
        list.iter()
            .any(|command| *command == cut_to(play, cut_to(frame_box, clipped.clone()))),
        "{:?}",
        list.iter().collect::<Vec<_>>()
    );
}
