//! A ship whose sheets came from `PICT` pairs (rdm rendering phase 20):
//! a `shän` image with no `rlëD` is cut from its `PICT` pair
//! (`nova_data::ShipImage`), and the flight draws it by its ship and image
//! ([`ImageKey::ship_sheet`]), as it draws an `rlëD` by its ID. On
//! [`FakeFlight`], with the NPC's sheet from the fake catalog.

use nova_data::ShipImage;

use super::faked_world::world_list;
use super::ship_drawing::{display, flying_with, npc_sheet, shown};
use super::*;
use crate::{ImageKind, SheetImage};

/// NPC ship 129's `image`, cut from its `PICT` pair.
fn pair(image: ShipImage) -> SheetImage {
    SheetImage::Ship { ship: 129, image }
}

/// Every sprite drawn: its key.
fn drawn_keys(list: &DrawList) -> Vec<ImageKey> {
    list.iter()
        .filter_map(|command| match *command {
            DrawCommand::Sprite { image, .. } | DrawCommand::Colorized { image, .. } => Some(image),
            _ => None,
        })
        .collect()
}

#[test]
fn a_ship_whose_hull_and_glow_came_from_pict_pairs_draws_their_cut_frames() {
    let sheet = ShipSheet {
        image: pair(ShipImage::Base),
        glow: Some(LayerSheet {
            image: pair(ShipImage::Glow),
            ..layer(2101, 72)
        }),
        ..npc_sheet()
    };
    let catalog = shown(display(0, 1, 0), display(0, 1, 0), Some(sheet));
    let view = flying_with(catalog, 90.0);
    let keys = drawn_keys(&world_list(&view));
    let cut: Vec<ImageKey> = keys
        .iter()
        .copied()
        .filter(|key| matches!(key.kind, ImageKind::ShipSheet { .. }))
        .collect();
    let frame = cut.first().expect("the hull is drawn").frame;
    assert_eq!(
        cut,
        [
            ImageKey::ship_sheet(129, ShipImage::Base, frame),
            ImageKey::ship_sheet(129, ShipImage::Glow, frame % 72),
        ]
    );
    assert_eq!(frame, 18, "a quarter turn of 72");
    // Neither is drawn from an `rlëD` of its old IDs; the lights, still an
    // `rlëD`, are.
    assert!(!keys.iter().any(|key| key.id == 2001 || key.id == 2101));
    assert!(keys.contains(&ImageKey::sprite(2201, frame)));
}
