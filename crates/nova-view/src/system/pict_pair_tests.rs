//! A stellar sheet made of a `PICT` pair, through the game data, the
//! catalog and the scene (stellar-fidelity phase 4): the original cuts a
//! `spïn` with no `rlëD` from its sprites and mask `PICT`s, so a stellar
//! whose `DeadType` names one draws it.

use std::io;
use std::num::NonZeroU16;
use std::path::Path;

use nova_data::graphics::fixture::{RledBuilder, rgb888_pict};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};
use nova_sim::DrawnSheet;

use super::camera::Camera;
use super::catalog::{StellarSheet, SystemCatalog, SystemId};
use super::scene::{StellarLook, SystemScene, draw_stellars};
use crate::{DrawCommand, DrawList, ImageKey, ImageKind, Point, SheetImage};

/// One data file, `/data/Nova Data`, holding a fork.
struct OneFile(Vec<u8>);

impl DirLister for OneFile {
    fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
        Ok(vec![Listing {
            name: "Nova Data".into(),
            kind: EntryKind::File,
        }])
    }
}

impl ForkReader for OneFile {
    fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        Ok((fork == Fork::Data).then(|| self.0.clone()))
    }
}

fn store(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
    let fork = resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
            fork.resource(*ty, *id, None, data)
        })
        .build()
        .bytes;
    let file = OneFile(fork);
    GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
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

/// A `spöb` at (0, 0) of graphic type `graphic_type` and `DeadType`
/// `dead_type`.
fn stellar(graphic_type: i16, dead_type: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[graphic_type]);
    put_i16s(&mut bytes, 0x240, &[dead_type]);
    bytes
}

/// A `spïn` naming sprites `sprites` and masks `masks`, `size` x `size`
/// each, `x_tiles` across one row.
fn spin(sprites: i16, masks: i16, size: i16, x_tiles: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[sprites, masks, size, size, x_tiles, 1]);
    bytes
}

/// An `rlëD` of one `size` x `size` frame.
fn sheet(size: u16) -> Vec<u8> {
    RledBuilder::new(size, size)
        .frame(|f| (0..size).fold(f, |f, _| f.line().pixels(&vec![0x7C00; size.into()])))
        .build()
}

/// Sol (131) holding stellar 140 of `Type` 1 (`spïn` and `rlëD` 1001)
/// whose `DeadType` 3 is `spïn` 1003: two 4x4 frames cut from the 8x4
/// `PICT` 2003, masked by the white `PICT` 2103. Graphic type 0 is `spïn`
/// and `rlëD` 1000.
fn sol() -> GameData {
    store(&[
        (System::TYPE, 131, system(&[140])),
        (Stellar::TYPE, 140, stellar(1, 3)),
        (Spin::TYPE, 1000, spin(1000, -1, 6, 1)),
        (RLED, 1000, sheet(6)),
        (Spin::TYPE, 1001, spin(1001, -1, 5, 1)),
        (RLED, 1001, sheet(5)),
        (Spin::TYPE, 1003, spin(2003, 2103, 4, 2)),
        (PICT, 2003, rgb888_pict(8, 4, &[[0x80, 0x40, 0x20]; 32])),
        (PICT, 2103, rgb888_pict(8, 4, &[[0xFF; 3]; 32])),
    ])
}

/// The sprites drawn, and whether any placeholder text is.
fn drawn(scene: &SystemScene) -> (Vec<ImageKey>, bool) {
    let mut list = DrawList::new();
    draw_stellars(&mut list, scene, &Camera::centred_on(Point::default()));
    let sprites = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Sprite { image, .. } => Some(*image),
            _ => None,
        })
        .collect();
    let placeholder = list.iter().any(|command| {
        matches!(command, DrawCommand::Text { text, .. } if text.starts_with("Sprite unavailable"))
    });
    (sprites, placeholder)
}

#[test]
fn a_destroyed_stellar_whose_dead_type_is_a_pict_pair_draws_it() {
    let data = sol();
    let contents = data.system(SystemId(131));
    assert_eq!(contents.problems, Vec::<String>::new());
    let stellar = &contents.stellars[0];
    assert_eq!(
        stellar.dead_sprite,
        Some(Ok(StellarSheet {
            image: SheetImage::Spin(1003),
            frames: NonZeroU16::new(2).expect("non-zero"),
            frame_width: 4,
            frame_height: 4,
        }))
    );
    let mut scene = SystemScene::load(&data, SystemId(131));
    assert_eq!(scene.problems(), [] as [String; 0]);
    assert_eq!(drawn(&scene), (vec![ImageKey::sprite(1001, 0)], false));
    scene.set_destroyed(|_| StellarLook {
        destroyed: true,
        drawn: DrawnSheet::Dead,
        counted: DrawnSheet::Dead,
    });
    let (sprites, placeholder) = drawn(&scene);
    assert_eq!(
        sprites,
        [ImageKey {
            kind: ImageKind::SpinSheet,
            id: 1003,
            frame: 0
        }]
    );
    assert_ne!(sprites[0], ImageKey::sprite(1000, 0), "not graphic type 0");
    assert_ne!(sprites[0], ImageKey::sprite(1001, 0), "not its Type");
    assert!(!placeholder);
}

#[test]
fn a_pict_pair_of_more_frames_than_a_u16_holds_counts_the_most_it_can() {
    let data = store(&[
        (System::TYPE, 131, system(&[140])),
        (Stellar::TYPE, 140, stellar(1, -1)),
        (Spin::TYPE, 1001, spin(2001, 2001, 1, 256)),
        (PICT, 2001, rgb888_pict(256, 256, &vec![[0; 3]; 256 * 256])),
    ]);
    let contents = data.system(SystemId(131));
    assert_eq!(
        contents.stellars[0].sprite,
        Ok(StellarSheet {
            image: SheetImage::Spin(1001),
            frames: NonZeroU16::MAX,
            frame_width: 1,
            frame_height: 1,
        })
    );
}
