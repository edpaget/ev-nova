//! The catalog sees a stellar's sheet made of a `PICT` pair
//! (stellar-fidelity phase 4): its frame size comes through
//! `GameData::spin_sheet` like an `rlëD`'s, so a stellar whose `Type` or
//! `DeadType` names one draws it.

use std::io;
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

use crate::catalog::{PilotCatalog, SystemId};

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
        bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
    }
}

/// A `sÿst` whose navigation defaults are `stellars`.
fn system(stellars: &[i16]) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x24, &[-1; 16]);
    put_i16s(&mut bytes, 0x24, stellars);
    bytes
}

/// A `spöb` of graphic type `graphic_type` and `DeadType` `dead_type`.
fn stellar(graphic_type: i16, dead_type: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[graphic_type]);
    put_i16s(&mut bytes, 0x14, &[-1]);
    put_i16s(&mut bytes, 0x240, &[dead_type]);
    bytes
}

/// A `spïn` naming sprites `sprites` and masks `masks`, `size` x `size`
/// each, `x_tiles` x 1.
fn spin(sprites: i16, masks: i16, size: i16, x_tiles: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[sprites, masks, size, size, x_tiles, 1]);
    bytes
}

#[test]
fn a_stellar_whose_dead_type_is_a_pict_pair_has_its_frame_size() {
    let white = rgb888_pict(8, 4, &[[0xFF; 3]; 32]);
    let data = store(&[
        (System::TYPE, 130, system(&[129, 131])),
        // Type 0, an rlëD, destroyed as type 3.
        (Stellar::TYPE, 129, stellar(0, 3)),
        // Type 3 whole.
        (Stellar::TYPE, 131, stellar(3, -1)),
        (Spin::TYPE, 1000, spin(1000, -1, 6, 1)),
        (
            RLED,
            1000,
            RledBuilder::new(6, 6).frame(|f| f.line().skip(6)).build(),
        ),
        // Two 4x4 frames from an 8x4 PICT pair.
        (Spin::TYPE, 1003, spin(2003, 2103, 4, 2)),
        (PICT, 2003, rgb888_pict(8, 4, &[[0x80; 3]; 32])),
        (PICT, 2103, white),
    ]);
    let sites = data.landing_sites(SystemId(130));
    let looks: Vec<_> = sites
        .iter()
        .map(|site| {
            (
                site.id.0,
                site.frame_size,
                site.dead_graphic,
                site.dead_frame_size,
                site.animation.type_frames,
                site.animation.dead_frames,
            )
        })
        .collect();
    assert_eq!(
        looks,
        [
            (129, Some((6, 6)), Some(3), Some((4, 4)), 1, 2),
            (131, Some((4, 4)), None, None, 2, 0),
        ]
    );
}
