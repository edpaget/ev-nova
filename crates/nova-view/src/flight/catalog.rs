//! The ship sprite port: the player ship's sprite sheet, in the view's own
//! terms.

use std::num::NonZeroU16;
use std::rc::Rc;

pub use nova_sim::ShipId;

/// A ship's resolved sprite sheet: its `rlëD`, how many frames make one
/// turn, and each frame's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipSheet {
    /// The `rlëD`'s ID.
    pub image_id: i16,
    /// The `shän`'s frames per rotation: the frames of one set, which turn
    /// the ship once round.
    pub rotations: NonZeroU16,
    /// Each frame's width in pixels.
    pub frame_width: u32,
    /// Each frame's height in pixels.
    pub frame_height: u32,
}

/// The ships' sprite sheets.
pub trait ShipSprites {
    /// Ship `id`'s sheet, or why it cannot be shown.
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String>;
}

/// A borrowed catalog is a catalog.
impl<T: ShipSprites + ?Sized> ShipSprites for &T {
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
        (**self).ship_sheet(id)
    }
}

/// A shared catalog is a catalog, so a view and the renderer can read the
/// same game data.
impl<T: ShipSprites + ?Sized> ShipSprites for Rc<T> {
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
        (**self).ship_sheet(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every ship has a 36-rotation sheet with its own ID.
    struct Sheets;

    impl ShipSprites for Sheets {
        fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
            Ok(ShipSheet {
                image_id: id.0,
                rotations: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 48,
                frame_height: 48,
            })
        }
    }

    fn image(catalog: impl ShipSprites, id: i16) -> Result<i16, String> {
        catalog.ship_sheet(ShipId(id)).map(|sheet| sheet.image_id)
    }

    #[test]
    fn borrowed_and_shared_catalogs_are_catalogs() {
        assert_eq!(image(&Sheets, 130), Ok(130));
        assert_eq!(image(Rc::new(Sheets), 131), Ok(131));
    }
}
