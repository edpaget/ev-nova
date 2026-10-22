//! A ship type's target picture, which the status bar (`_DrawStatusTarg`
//! reads `shipClass[type]+0xa0c` @0x4b7ed) and the main screen's pilot
//! panel both draw.
//!
//! `_LoadShipSprites` sets +0xa0c to the ship's own index when its
//! `targetInfoPic`, `PICT` 3000 + (ID − 128), loaded (`_LoadSprites`
//! @0x217f2-0x2180c; @0x23f10-0x23f1e), and otherwise to the base ship's
//! (@0x23f28-0x23f3b): the first lower-ID `shïp` whose `shän` has the same
//! `BaseImage` (`_LoadExtendedShipSprites` @0x22704, @0x227d6-0x22849). A
//! `shïp` with no `shän` skips the fill (@0x23ef3). [`target_picture`] is
//! the pure decision; [`game_target_picture`] feeds it from [`GameData`].

use nova_data::GameData;
use nova_data::records::ship::Ship;
use nova_sim::catalog::ShipId;

use crate::spaceport::ShipBaseImages;

/// The first target picture, `PICT` 3000 for `shïp` 128
/// (`_LoadSprites`).
pub const FIRST_TARGET_PICTURE: i16 = 3000;

/// A ship's target picture: `PICT` `picture`, `width` x `height`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipPicture {
    /// The `PICT`'s ID.
    pub picture: i16,
    /// Its width.
    pub width: u32,
    /// Its height.
    pub height: u32,
}

/// The first `shïp`'s ID, whose target picture is [`FIRST_TARGET_PICTURE`].
pub const FIRST_SHIP: i16 = 128;

/// Ship type `ship`'s target picture, as `_LoadShipSprites` picks
/// `shipClass[type]+0xa0c`.
///
/// It is `PICT` 3000 + (ID − 128), the status bar's, when it exists
/// (`_LoadSprites` @0x217f2-0x2180c, `_LoadShipSprites`
/// @0x23f10-0x23f1e). Otherwise it is the base ship's: the first lower-ID
/// `shïp` whose `shän` has the same `BaseImage`
/// (`_LoadExtendedShipSprites` @0x22704, @0x227d6-0x22849), and its picture
/// if it has one (@0x23f28-0x23f3b), with no search past it. A `shïp` with
/// no `shän` has none (@0x23ef3), and no ID below 128 is a `shïp`.
///
/// `bases` gives each `shïp`'s `shän` `BaseImage`, in any order; `size`
/// gives a `PICT`'s width and height when it exists and decodes. Unlike the
/// shipyard's `ship_picture`, which takes the lowest sharer of any ID that
/// has a picture, this stops at the first lower sharer.
#[must_use]
pub fn target_picture(
    ship: ShipId,
    bases: &[(ShipId, i16)],
    size: impl Fn(i16) -> Option<(u32, u32)>,
) -> Option<ShipPicture> {
    let base_image = |id: ShipId| {
        bases
            .iter()
            .find(|(other, _)| *other == id)
            .map(|&(_, b)| b)
    };
    let picture = |id: ShipId| {
        let picture = FIRST_TARGET_PICTURE.checked_add(id.0.checked_sub(FIRST_SHIP)?)?;
        let (width, height) = size(picture)?;
        Some(ShipPicture {
            picture,
            width,
            height,
        })
    };
    if ship.0 < FIRST_SHIP {
        return None;
    }
    let image = base_image(ship)?;
    picture(ship).or_else(|| {
        let base = bases
            .iter()
            .filter(|&&(id, other)| other == image && (FIRST_SHIP..ship.0).contains(&id.0))
            .map(|&(id, _)| id)
            .min()?;
        picture(base)
    })
}

/// Ship type `ship`'s target picture in `data` ([`target_picture`]), from
/// each decodable `shïp`'s `shän` `BaseImage` and the `PICT` sizes.
#[must_use]
pub fn game_target_picture(data: &GameData, ship: ShipId) -> Option<ShipPicture> {
    let mut bases = data.ship_base_images();
    bases.retain(|(id, _)| matches!(data.get::<Ship>(id.0), Some(Ok(_))));
    target_picture(ship, &bases, |id| {
        crate::ui::PictureSizes::picture_size(data, id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // The target picture's base-ship fallback.

    /// Pictures that exist, each `(id, width, height)`.
    fn existing(found: &[(i16, u32, u32)]) -> impl Fn(i16) -> Option<(u32, u32)> + '_ {
        |id| {
            found
                .iter()
                .find(|(picture, ..)| *picture == id)
                .map(|&(_, width, height)| (width, height))
        }
    }

    fn shown(picture: i16, width: u32, height: u32) -> ShipPicture {
        ShipPicture {
            picture,
            width,
            height,
        }
    }

    #[test]
    fn a_ship_shows_its_own_target_picture_when_it_exists() {
        let bases = [
            (ShipId(128), 1000),
            (ShipId(129), 1000),
            (ShipId(130), 1000),
        ];
        let found = existing(&[(3000, 2, 2), (3001, 6, 3), (3002, 41, 20)]);
        assert_eq!(
            target_picture(ShipId(130), &bases, &found),
            Some(shown(3002, 41, 20))
        );
        assert_eq!(
            target_picture(ShipId(129), &bases, &found),
            Some(shown(3001, 6, 3))
        );
        assert_eq!(
            target_picture(ShipId(128), &bases, &found),
            Some(shown(3000, 2, 2)),
            "the first shïp"
        );
    }

    #[test]
    fn a_ship_without_a_picture_takes_the_first_lower_one_sharing_its_base_image() {
        let bases = [
            (ShipId(128), 1100),
            (ShipId(131), 1000),
            (ShipId(130), 1000),
            (ShipId(129), 1000),
            (ShipId(133), 1200),
            (ShipId(134), 1200),
        ];
        let found = existing(&[(3000, 5, 5), (3001, 6, 3), (3002, 7, 4), (3006, 9, 9)]);
        assert_eq!(
            target_picture(ShipId(131), &bases, &found),
            Some(shown(3001, 6, 3)),
            "129's, the lowest sharing 1000, in any order"
        );
        assert_eq!(
            target_picture(ShipId(133), &bases, &found),
            None,
            "only lower ships are searched"
        );
    }

    #[test]
    fn the_base_ship_is_the_first_lower_sharer_even_without_a_picture() {
        let bases = [
            (ShipId(129), 1000),
            (ShipId(130), 1000),
            (ShipId(131), 1000),
        ];
        let found = existing(&[(3002, 7, 4)]);
        assert_eq!(
            target_picture(ShipId(131), &bases, &found),
            None,
            "129 is the base, and it has none; 130's is not looked at"
        );
    }

    #[test]
    fn a_ship_with_no_base_image_has_no_picture() {
        let found = existing(&[(3002, 4, 4)]);
        assert_eq!(
            target_picture(ShipId(130), &[(ShipId(129), 1000)], &found),
            None,
            "no shän, though PICT 3002 exists"
        );
        let below = [(ShipId(126), 1000), (ShipId(127), 1000)];
        assert_eq!(
            target_picture(ShipId(127), &below, existing(&[(2999, 1, 1)])),
            None,
            "below the first shïp, with no picture and no base"
        );
    }
}
