//! The catalog sees a stellar whose sheet is a 32-bit `rlëD` (rendering
//! phase 1): its frame size is the header's, the box the original clicks,
//! though the original draws the sheet twice that wide.

use nova_data::Record;
use nova_data::graphics::RLED;
use nova_data::graphics::fixture::RledBuilder;
use nova_data::records::fixture::SpinBuilder;
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

/// A `spöb` of graphic type `graphic_type`.
fn stellar(graphic_type: i16) -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[graphic_type]);
    put_i16s(&mut bytes, 0x14, &[-1]);
    put_i16s(&mut bytes, 0x240, &[-1]);
    bytes
}

#[test]
fn a_stellar_whose_type_is_a_32_bit_sheet_has_the_headers_frame_size() {
    let spin = SpinBuilder::new()
        .sprites_id(1001)
        .masks_id(-1)
        .x_size(3)
        .y_size(2)
        .x_tiles(1)
        .y_tiles(1)
        .build();
    let data = store(&[
        (System::TYPE, 130, system(&[129])),
        (Stellar::TYPE, 129, stellar(1)),
        (Spin::TYPE, 1001, spin),
        (
            RLED,
            1001,
            RledBuilder::new(3, 2)
                .depth(32)
                .count_lines()
                .frame(|f| f.line().pixels32(&[1, 2, 3]).line().skip32(3))
                .build(),
        ),
    ]);
    let sites = data.landing_sites(SystemId(130));
    let looks: Vec<_> = sites
        .iter()
        .map(|site| (site.id.0, site.frame_size, site.animation.type_frames))
        .collect();
    assert_eq!(looks, [(129, Some((3, 2)), 1)]);
}
