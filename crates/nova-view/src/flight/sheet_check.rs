//! The [`ShipSprites`] port's sheet contract, which every adapter's sheets
//! keep and the flight's drawing assumes: a sheet it answers has frames of
//! some size, each frame's image is no smaller than its frame (the image
//! may pad the frame, never crop it), and each layer alike. An error is
//! always allowed: the flight then draws the outline or the crossed box.
//!
//! The game data's adapter (`flight::data`) and the borrowed stock look
//! ([`StockShipLooks`](super::stock_looks::StockShipLooks)) both run
//! [`check_sheets`]; other crates' adapters may too, behind the `fixture`
//! feature.

use super::catalog::{LayerSheet, ShipId, ShipSheet, ShipSprites};

/// Checks the sheet `sprites` answers for each of `ships` against the
/// contract (see the module docs): `Err` names the first ship and what is
/// wrong with its sheet.
///
/// # Errors
///
/// The first sheet that breaks the contract, and how.
pub fn check_sheets(sprites: &impl ShipSprites, ships: &[ShipId]) -> Result<(), String> {
    for &ship in ships {
        if let Ok(sheet) = sprites.ship_sheet(ship) {
            check_sheet(&sheet).map_err(|why| format!("shïp {}: {why}", ship.0))?;
        }
    }
    Ok(())
}

/// Whether `sheet` and its layers keep the contract.
fn check_sheet(sheet: &ShipSheet) -> Result<(), String> {
    check_frame("the hull", sheet.size(), sheet.image_size)?;
    let layers: [(&str, Option<LayerSheet>); 5] = [
        ("the glow", sheet.glow),
        ("the lights", sheet.lights),
        ("the weapon glow", sheet.weapon),
        ("the alt layer", sheet.alt),
        ("the shield", sheet.shield),
    ];
    for (what, layer) in layers {
        if let Some(layer) = layer {
            check_frame(what, layer.size(), layer.image_size)?;
        }
    }
    Ok(())
}

/// Whether `what`'s frame of `size` has some area and an image of
/// `image` no smaller than it.
fn check_frame(what: &str, size: (u32, u32), image: (u32, u32)) -> Result<(), String> {
    if size.0 == 0 || size.1 == 0 {
        return Err(format!("{what}'s frame is {} x {}", size.0, size.1));
    }
    if image.0 < size.0 || image.1 < size.1 {
        return Err(format!(
            "{what}'s image, {} x {}, is smaller than its {} x {} frame",
            image.0, image.1, size.0, size.1
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use super::*;
    use crate::outline::Outline;
    use crate::sheet_image::SheetImage;

    /// Answers `sheet` for ship 128 and an error for every other.
    struct One(ShipSheet);

    impl ShipSprites for One {
        fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
            (id == ShipId(128))
                .then_some(self.0)
                .ok_or_else(|| "none".to_owned())
        }

        fn ship_outline(&self, _id: ShipId) -> Option<Outline> {
            None
        }
    }

    fn layer(size: (u32, u32), image_size: (u32, u32)) -> LayerSheet {
        LayerSheet {
            image: SheetImage::Rled(1001),
            frames: NonZeroU16::MIN,
            frame_width: size.0,
            frame_height: size.1,
            image_size,
            clip_quirks: false,
            solid: false,
            eight_bit: false,
        }
    }

    fn sheet(size: (u32, u32), image_size: (u32, u32)) -> ShipSheet {
        ShipSheet {
            image: SheetImage::Rled(1000),
            rotations: NonZeroU16::MIN,
            frame_width: size.0,
            frame_height: size.1,
            image_size,
            clip_quirks: false,
            glow: None,
            lights: None,
            weapon: None,
            weap_decay: 0,
            alt: None,
            shield: None,
            base_transp: 0,
            solid: false,
            eight_bit: false,
        }
    }

    fn checked(sheet: ShipSheet) -> Result<(), String> {
        check_sheets(&One(sheet), &[ShipId(127), ShipId(128)])
    }

    #[test]
    fn a_sheet_with_a_padded_image_and_errors_keep_the_contract() {
        assert_eq!(checked(sheet((4, 3), (4, 3))), Ok(()));
        assert_eq!(checked(sheet((4, 3), (6, 5))), Ok(()));
        let glowing = ShipSheet {
            glow: Some(layer((6, 5), (6, 5))),
            ..sheet((4, 3), (4, 3))
        };
        assert_eq!(checked(glowing), Ok(()));
    }

    #[test]
    fn a_frame_with_no_area_breaks_it() {
        assert_eq!(
            checked(sheet((0, 3), (0, 3))),
            Err("shïp 128: the hull's frame is 0 x 3".to_owned())
        );
    }

    #[test]
    fn an_image_smaller_than_its_frame_breaks_it() {
        assert_eq!(
            checked(sheet((4, 3), (4, 2))),
            Err("shïp 128: the hull's image, 4 x 2, is smaller than its 4 x 3 frame".to_owned())
        );
    }

    #[test]
    fn a_layer_breaks_it_as_the_hull_does() {
        let shield = ShipSheet {
            shield: Some(layer((5, 5), (4, 5))),
            ..sheet((4, 3), (4, 3))
        };
        assert_eq!(
            checked(shield),
            Err("shïp 128: the shield's image, 4 x 5, is smaller than its 5 x 5 frame".to_owned())
        );
        let lights = ShipSheet {
            lights: Some(layer((0, 0), (0, 0))),
            ..sheet((4, 3), (4, 3))
        };
        assert!(checked(lights).is_err());
    }
}
