//! The catalog counts a stellar's sheets' frames as the sprite tick
//! animates over them (rendering phase 11): each sheet's master count,
//! the `spïn`'s tiles when fewer than the decoded frames, else the
//! decoded frames (`nova_data::store::master_frame_count`,
//! `_LoadMasterStellarSprite` @0x1ed8c-0x1ed97).

use nova_data::Record;
use nova_data::graphics::RLED;
use nova_data::graphics::fixture::RledBuilder;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;

use crate::catalog::{PilotCatalog, SystemId};
use crate::test_store::{put_i16s, store};

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

/// A `spïn` naming `rlëD` `image`, `x_tiles` x 1.
fn spin(image: i16, x_tiles: i16) -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[image, -1, 4, 4, x_tiles, 1]);
    bytes
}

/// An `rlëD` of `frames` 4 x 4 frames.
fn frames(frames: u16) -> Vec<u8> {
    (0..frames)
        .fold(RledBuilder::new(4, 4), |sheet, _| {
            sheet.frame(|f| (0..4).fold(f, |f, _| f.line().pixels(&[0x7C00; 4])))
        })
        .build()
}

/// The sheet counts of stellars 128 to 131 of a system, their `Type` and
/// `DeadType` sheets each of 4 frames under `tiles` (graphic types 1 to
/// 4, then a `DeadType` with no sheet).
fn counts(tiles: [i16; 3]) -> Vec<(u16, u16)> {
    let data = store(&[
        (System::TYPE, 130, system(&[128, 129, 130, 131])),
        (Stellar::TYPE, 128, stellar(1, 2)),
        (Stellar::TYPE, 129, stellar(2, 3)),
        (Stellar::TYPE, 130, stellar(3, 1)),
        // Its `DeadType` names a `spïn` that is not there.
        (Stellar::TYPE, 131, stellar(1, 9)),
        (Spin::TYPE, 1001, spin(1001, tiles[0])),
        (RLED, 1001, frames(4)),
        (Spin::TYPE, 1002, spin(1002, tiles[1])),
        (RLED, 1002, frames(4)),
        (Spin::TYPE, 1003, spin(1003, tiles[2])),
        (RLED, 1003, frames(4)),
    ]);
    data.landing_sites(SystemId(130))
        .iter()
        .map(|site| {
            (
                site.animation.type_sheet_frames,
                site.animation.dead_sheet_frames,
            )
        })
        .collect()
}

#[test]
fn a_stellars_sheet_frames_are_its_tiles_when_fewer_than_its_frames() {
    // Tiles 2, 0 and 6 over 4 frames: 2, then the decoded 4 twice.
    assert_eq!(counts([2, 0, 6]), [(2, 4), (4, 4), (4, 2), (2, 0)]);
}

#[test]
fn a_stellar_whose_sheet_does_not_read_counts_0_frames() {
    let data = store(&[
        (System::TYPE, 130, system(&[128])),
        (Stellar::TYPE, 128, stellar(1, 2)),
    ]);
    let sites = data.landing_sites(SystemId(130));
    assert_eq!(
        (
            sites[0].animation.type_sheet_frames,
            sites[0].animation.dead_sheet_frames
        ),
        (0, 0)
    );
}

#[test]
fn graphic_type_0s_frame_count_is_its_master_count() {
    for (x_tiles, count) in [(2, 2), (0, 4), (6, 4)] {
        let data = store(&[
            (Spin::TYPE, 1000, spin(1000, x_tiles)),
            (RLED, 1000, frames(4)),
        ]);
        assert_eq!(data.fallback_frame_count(), count, "{x_tiles}");
        let data = std::rc::Rc::new(data);
        assert_eq!(PilotCatalog::fallback_frame_count(&data), count, "Rc");
        assert_eq!(PilotCatalog::fallback_frame_count(&&*data), count, "&");
    }
    // A sheet that does not read counts 1, as a slot shows one frame.
    assert_eq!(store(&[]).fallback_frame_count(), 1);
}
