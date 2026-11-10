//! The frame a ship's loaded base sprite has, over the game data: the
//! adapter behind [`HullRecord::frame`](crate::catalog::HullRecord::frame).
//!
//! `_LoadExtendedShipSprites` makes a ship's base sprite from its
//! `BaseImage` `rlëD` when there is one (`_SWCreateSpriteFromRLEResource`
//! @0x228b5), which reads the header's width and height and decodes no
//! token, so a sheet with corrupt tokens still loads at its header's size.
//! With no `rlëD` (@0x2291d-0x2298b) it cuts the `PICT` by the `shän`'s
//! `BaseXSize`/`BaseYSize`. A ship sharing an earlier ship's `BaseImage`
//! clones that ship's sprite (@0x22817), so the frame is read once per
//! sprite base and shared (see `hulls` in the parent module).
//!
//! In a 16-bit world the type is `rlëD`; the `rlë8` branch
//! (@0x22866-0x22877) is unreachable, as the world is always 16-bit. A
//! failed create (`_ReportSpriteLoadError` @0x228fc, @0x229d2, @0x22a32)
//! leaves the original's master sprite null and crashes its next reader;
//! here an `rlëD` whose header can't be read falls back to the `shän`
//! sizes, and sizes that aren't both positive give no frame, so the hull
//! falls back to its own sizes.

use nova_data::GameData;

use crate::catalog::{FrameSize, ShipId};

/// The frame of `base`'s loaded base sprite: its `BaseImage` `rlëD`'s
/// header size, or the frames its `PICT` pair cuts into, when that reads
/// (`GameData::ship_sprite_frame`); otherwise its `shän`'s `(BaseXSize,
/// BaseYSize)`, `shan`, when both are positive; otherwise none.
pub(super) fn base_frame(data: &GameData, base: ShipId, shan: (i16, i16)) -> Option<FrameSize> {
    if let Ok(sprite) = data.ship_sprite_frame(base) {
        return Some(FrameSize {
            width: sprite.sheet.width,
            height: sprite.sheet.height,
        });
    }
    let (width, height) = shan;
    Some(FrameSize {
        width: u32::try_from(width).ok().filter(|&w| w > 0)?,
        height: u32::try_from(height).ok().filter(|&h| h > 0)?,
    })
}

#[cfg(test)]
mod tests {
    use nova_data::Record;
    use nova_data::graphics::fixture::{RledBuilder, rgb888_pict};
    use nova_data::graphics::{PICT, RLED};
    use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
    use nova_data::records::ship::Ship;
    use nova_data::records::ship_anim::ShipAnim;
    use nova_rsrc::ResType;

    use crate::catalog::{CombatCatalog, FrameSize, HullRecord, ShipId};
    use crate::test_store::store;

    const fn size(width: u32, height: u32) -> FrameSize {
        FrameSize { width, height }
    }

    /// A `shïp` `id`.
    fn ship(id: i16) -> (ResType, i16, Vec<u8>) {
        (Ship::TYPE, id, ShipBuilder::new().build())
    }

    /// A `shän` `id` with base image `image`, `width` x `height`.
    fn shan(id: i16, image: i16, width: i16, height: i16) -> (ResType, i16, Vec<u8>) {
        let anim = ShipAnimBuilder::new()
            .base_image_id(image)
            .base_x_size(width)
            .base_y_size(height)
            .build();
        (ShipAnim::TYPE, id, anim)
    }

    /// An `rlëD` `id` of one `width` x `height` frame.
    fn rled(id: i16, width: u16, height: u16) -> (ResType, i16, Vec<u8>) {
        (
            RLED,
            id,
            RledBuilder::new(width, height).frame(|f| f).build(),
        )
    }

    /// A one-pixel `PICT` `id`, standing for a sprites picture.
    fn pict(id: i16) -> (ResType, i16, Vec<u8>) {
        (PICT, id, rgb888_pict(1, 1, &[[0; 3]]))
    }

    fn hull(hulls: &[HullRecord], id: i16) -> &HullRecord {
        hulls.iter().find(|h| h.id == ShipId(id)).expect("a hull")
    }

    #[test]
    fn a_hulls_frame_is_its_rled_headers_size_not_its_shäns() {
        let hulls = store(&[ship(128), shan(128, 1000, 24, 40), rled(1000, 30, 50)]).hulls();
        let hull = hull(&hulls, 128);
        assert_eq!(hull.frame, Some(size(30, 50)));
        assert_eq!((hull.size, hull.height), (Some(24), Some(40)));
    }

    #[test]
    fn a_pict_sheet_hull_keeps_its_shäns_sizes() {
        let hulls = store(&[ship(128), shan(128, 1000, 24, 40), pict(1000)]).hulls();
        assert_eq!(hull(&hulls, 128).frame, Some(size(24, 40)));
    }

    #[test]
    fn a_hull_sharing_a_base_image_takes_its_bases_frame() {
        let shared = |sheet| {
            store(&[
                ship(128),
                ship(130),
                shan(128, 1000, 24, 24),
                shan(130, 1000, 40, 40),
                sheet,
            ])
            .hulls()
        };
        let hulls = shared(rled(1000, 30, 50));
        assert_eq!(hull(&hulls, 128).frame, Some(size(30, 50)));
        assert_eq!(hull(&hulls, 130).frame, Some(size(30, 50)));
        let hulls = shared(pict(1000));
        assert_eq!(hull(&hulls, 128).frame, Some(size(24, 24)));
        assert_eq!(hull(&hulls, 130).frame, Some(size(24, 24)), "the clone");
        assert_eq!(hull(&hulls, 130).size, Some(40), "its own shän's size");
    }

    #[test]
    fn an_unreadable_rled_header_falls_back_to_the_shän_sizes() {
        let deep = (RLED, 1000, RledBuilder::new(30, 50).depth(1).build());
        let hulls = store(&[ship(128), shan(128, 1000, 24, 40), deep]).hulls();
        assert_eq!(hull(&hulls, 128).frame, Some(size(24, 40)));
    }

    #[test]
    fn sizes_that_are_not_both_positive_give_no_frame() {
        for (width, height) in [(0, 40), (24, 0), (-1, 40), (24, -1)] {
            let hulls = store(&[ship(128), shan(128, 1000, width, height)]).hulls();
            assert_eq!(hull(&hulls, 128).frame, None, "{width} x {height}");
        }
        let hulls = store(&[ship(128), shan(128, 1000, 1, 1)]).hulls();
        assert_eq!(hull(&hulls, 128).frame, Some(size(1, 1)));
    }

    #[test]
    fn a_hull_without_a_shän_has_no_frame() {
        let hulls = store(&[ship(128), rled(128, 30, 50)]).hulls();
        assert_eq!(hull(&hulls, 128).frame, None);
    }
}
