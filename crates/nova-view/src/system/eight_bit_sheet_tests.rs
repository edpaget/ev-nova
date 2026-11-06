//! A stellar whose sheet is 8-bit, through the catalog port and the scene
//! (rendering phase 2). `_HandleStellarSprites` zeroes a stellar's
//! translucency and channel factors every tick (@0x2ee57-0x2ee7e), then
//! murks it (+0xb0, @0x2eea4), so the draw proc's depth-8 dispatch
//! ([`EightBitBlit`](crate::eight_bit_blit::EightBitBlit)) draws an
//! unmurked 8-bit stellar plainly (`_BlitPixieRLE`), and a murked one
//! through `_BlitPixieRLETranslucent8`, which crashes the original's
//! 16-bit world: the engine draws it as a 16-bit sheet is drawn murked.
//! No stellar reaches a translucency of 0x20 or more, so none draws
//! nothing.
//!
//! The scene's tests run over a hand-written [`SystemCatalog`] that hands
//! out a [`StellarSheet`] marked `eight_bit`; that the adapter over
//! `GameData` marks an 8-bit `rlëD` so is checked in `system::data`'s
//! tests.

use std::num::NonZeroU16;

use nova_sim::Vec2;

use super::camera::Camera;
use super::catalog::{
    StellarAnimation, StellarContents, StellarId, StellarSheet, SystemCatalog, SystemContents,
    SystemId,
};
use super::scene::{SystemScene, draw_stellars};
use crate::flight::murk::SeenMurk;
use crate::{Blend, Color, Colorize, DrawCommand, DrawList, ImageKey, Point, SheetImage};

/// Hands out one canned system.
struct OneSystem(SystemContents);

impl SystemCatalog for OneSystem {
    fn system(&self, id: SystemId) -> SystemContents {
        assert_eq!(id, self.0.id, "the canned system");
        self.0.clone()
    }
}

/// `rlëD` 1001: one 2x2 frame, 8-bit or not.
fn sheet(eight_bit: bool) -> StellarSheet {
    StellarSheet {
        image: SheetImage::Rled(1001),
        frames: NonZeroU16::MIN,
        frame_width: 2,
        frame_height: 2,
        image_size: (2, 2),
        eight_bit,
    }
}

/// Sol (131) holding stellar 140 at (200, 0), its sheet 8-bit or not.
fn sol(eight_bit: bool) -> SystemScene {
    let catalog = OneSystem(SystemContents {
        id: SystemId(131),
        name: "Sol".to_owned(),
        stellars: vec![StellarContents {
            id: StellarId(140),
            name: "Stellar 140".to_owned(),
            x: 200,
            y: 0,
            sprite: Ok(sheet(eight_bit)),
            dead_sprite: None,
            animation: StellarAnimation::default(),
            stand_in: None,
        }],
        type_zero_sprite: Err("none".to_owned()),
        problems: Vec::new(),
    });
    SystemScene::load(&catalog, SystemId(131))
}

/// The stellar's sprite command, seen through `seen`.
fn sprite(scene: &SystemScene, seen: Option<SeenMurk>) -> DrawCommand {
    let mut list = DrawList::new();
    draw_stellars(
        &mut list,
        scene,
        &Camera::centred_on(Point::default()),
        seen,
    );
    list.iter()
        .find(|command| !matches!(command, DrawCommand::Text { .. }))
        .cloned()
        .expect("a sprite")
}

/// Murk 50 seen from the origin: level 24 at the stellar.
fn murky() -> SeenMurk {
    SeenMurk {
        viewer: Vec2::ZERO,
        murk: 50,
        colour: 0x1081,
    }
}

#[test]
fn a_murked_8_bit_stellar_is_drawn_as_a_16_bit_one_is() {
    let eight = sprite(&sol(true), Some(murky()));
    assert_eq!(eight, sprite(&sol(false), Some(murky())));
    assert!(
        matches!(
            eight,
            DrawCommand::Colorized { colorize, .. }
                if colorize == Colorize::toward_pixel(24, 0x1081)
        ),
        "{eight:?}"
    );
}

#[test]
fn an_unmurked_8_bit_stellar_draws_plainly() {
    let plain = DrawCommand::Sprite {
        image: ImageKey::sprite(1001, 0),
        center: match sprite(&sol(true), None) {
            DrawCommand::Sprite { center, .. } => center,
            other => panic!("not a plain sprite: {other:?}"),
        },
        tint: Color::WHITE,
        blend: Blend::Normal,
    };
    assert_eq!(sprite(&sol(true), None), plain);
    // Murk that leaves it at level 0, seen from beside it, is no bias.
    let beside = SeenMurk {
        viewer: Vec2::new(200.0, 0.0),
        ..murky()
    };
    assert_eq!(sprite(&sol(true), Some(beside)), plain);
}
