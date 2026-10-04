//! The flight screen's ports, in the view's own terms: the ship sprite
//! port, the player ship's sprite sheet, and the status bar port, the
//! `ïntf` layouts the HUD is drawn from.

use std::num::NonZeroU16;
use std::rc::Rc;

pub use nova_sim::{GovtId, ShipId};

use crate::color::Color;
use crate::font::Font;
use crate::geometry::Bounds;

/// One of a ship's `shän` layers drawn over its base sprite: the layer's
/// `rlëD` and how many frames it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerSheet {
    /// The `rlëD`'s ID.
    pub image_id: i16,
    /// The layer's frames. The base's rotation frame `f` is drawn with the
    /// layer's frame `f % frames`.
    pub frames: NonZeroU16,
}

/// A ship's resolved sprite sheet: its `rlëD`, how many frames make one
/// turn, each frame's size, and the engine glow and running lights drawn
/// over it.
///
/// A layer the `shän` names but whose image cannot be resolved is `None`,
/// like a layer it does not name: flight draws the ship without it and
/// says nothing. The ship browser is where layer errors are reported.
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
    /// The engine glow, drawn while the ship thrusts.
    pub glow: Option<LayerSheet>,
    /// The running lights, drawn always.
    pub lights: Option<LayerSheet>,
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

/// An `ïntf` status bar layout. Every area is relative to the status
/// bar's top-left corner.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusBarLayout {
    /// `RadarArea`.
    pub radar: Bounds,
    /// `ShieldArea`.
    pub shield: Bounds,
    /// `ArmorArea`.
    pub armor: Bounds,
    /// `FuelArea`.
    pub fuel: Bounds,
    /// `NavArea`.
    pub nav: Bounds,
    /// `CargoArea`: its last line shows the date.
    pub cargo: Bounds,
    /// `BrightText`.
    pub bright_text: Color,
    /// `DimText`.
    pub dim_text: Color,
    /// `BrightRadar`.
    pub bright_radar: Color,
    /// `DimRadar`.
    pub dim_radar: Color,
    /// `ShieldColor`.
    pub shield_color: Color,
    /// `ArmorColor`.
    pub armor_color: Color,
    /// `FuelFull`: whole jumps' worth of fuel.
    pub fuel_full: Color,
    /// `FuelPartial`: the fuel left over a whole jump.
    pub fuel_partial: Color,
    /// `StatusFont`.
    pub font: Font,
    /// `StatFontSize`.
    pub font_size: f32,
    /// `StatusBkgnd`, raw: the background `PICT`'s ID, where values below
    /// 128 mean 128.
    pub status_bkgnd: i16,
}

/// The status bars: which one each government shows and how each is laid
/// out.
pub trait StatusBars {
    /// Govt `id`'s raw `Interface` field, or why it cannot be read.
    fn government_interface(&self, id: GovtId) -> Result<i16, String>;
    /// `ïntf` `id`, or why it cannot be read.
    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String>;
    /// `PICT` `id`'s width and height in pixels, or `None` when it is
    /// missing or does not decode.
    fn picture_size(&self, id: i16) -> Option<(u32, u32)>;
}

/// A borrowed catalog is a catalog.
impl<T: StatusBars + ?Sized> StatusBars for &T {
    fn government_interface(&self, id: GovtId) -> Result<i16, String> {
        (**self).government_interface(id)
    }

    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
        (**self).status_bar(id)
    }

    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        (**self).picture_size(id)
    }
}

/// A shared catalog is a catalog.
impl<T: StatusBars + ?Sized> StatusBars for Rc<T> {
    fn government_interface(&self, id: GovtId) -> Result<i16, String> {
        (**self).government_interface(id)
    }

    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
        (**self).status_bar(id)
    }

    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        (**self).picture_size(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    /// Every ship has a 36-rotation sheet with its own ID.
    struct Sheets;

    impl ShipSprites for Sheets {
        fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
            Ok(ShipSheet {
                image_id: id.0,
                rotations: NonZeroU16::new(36).expect("non-zero"),
                frame_width: 48,
                frame_height: 48,
                glow: None,
                lights: None,
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

    /// Every govt shows its own ID's bar; every bar is empty but for its
    /// background, its own ID; every picture is as wide as its ID.
    struct Bars;

    impl StatusBars for Bars {
        fn government_interface(&self, id: GovtId) -> Result<i16, String> {
            Ok(id.0)
        }

        fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
            let none = Bounds::at(Point::default(), 0.0, 0.0);
            Ok(StatusBarLayout {
                radar: none,
                shield: none,
                armor: none,
                fuel: none,
                nav: none,
                cargo: none,
                bright_text: Color::WHITE,
                dim_text: Color::WHITE,
                bright_radar: Color::WHITE,
                dim_radar: Color::WHITE,
                shield_color: Color::WHITE,
                armor_color: Color::WHITE,
                fuel_full: Color::WHITE,
                fuel_partial: Color::WHITE,
                font: Font::Geneva,
                font_size: 12.0,
                status_bkgnd: id,
            })
        }

        fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
            Some((u32::try_from(id).ok()?, 1))
        }
    }

    /// Everything `catalog` says about govt 130, `ïntf` 131 and `PICT` 194.
    fn bar(catalog: impl StatusBars) -> String {
        format!(
            "{:?} {:?} {:?}",
            catalog.government_interface(GovtId(130)),
            catalog.status_bar(131).map(|bar| bar.status_bkgnd),
            catalog.picture_size(194),
        )
    }

    #[test]
    fn borrowed_and_shared_status_bars_are_status_bars() {
        let direct = "Ok(130) Ok(131) Some((194, 1))";
        assert_eq!(bar(Bars), direct);
        assert_eq!(bar(&Bars), direct);
        assert_eq!(bar(Rc::new(Bars)), direct);
    }
}
