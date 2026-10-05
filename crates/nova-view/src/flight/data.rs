//! The flight screen's ports over the game data: thin mappings from
//! `GameData`'s ship sprite lookup to [`ShipSheet`], and from its `gövt`,
//! `ïntf` and `PICT` resources to the status bars.

use std::num::NonZeroU16;

use nova_data::graphics::{PICT, decode_pict};
use nova_data::records::govt::Govt;
use nova_data::records::interface::Interface;
use nova_data::{GameData, LayerError, LayerSprite};
use nova_sim::data::ship_blink;

use super::catalog::{
    GovtId, LayerSheet, ShipId, ShipSheet, ShipSprites, StatusBarLayout, StatusBars,
};
use crate::color::Color;
use crate::font::Font;
use crate::geometry::{Bounds, Point};

/// Decodes the sheet and its layers on every call; the flight screen asks
/// once, when it opens and when a ship is bought.
impl ShipSprites for GameData {
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
        let sprite = self.ship_sprite(id).map_err(|err| err.to_string())?;
        let sheet = &sprite.sheet;
        // The sheet's layout is the `shän`'s frames per rotation as columns.
        let rotations = NonZeroU16::new(sheet.layout().columns())
            .expect("a sheet layout has at least one column");
        // A layer that cannot be resolved is drawn as no layer; the ship
        // browser is where its error is shown.
        let (glow, lights) = self.ship_layers(id).map_or((None, None), |layers| {
            (
                layers.glow.and_then(layer_sheet),
                layers.lights.and_then(layer_sheet),
            )
        });
        Ok(ShipSheet {
            image_id: sprite.image_id,
            rotations,
            frame_width: sheet.frame_width(),
            frame_height: sheet.frame_height(),
            glow,
            lights,
            blink: ship_blink(self, id.0),
        })
    }
}

/// A resolved layer's ID and frame count; `None` for one that failed.
fn layer_sheet(layer: Result<LayerSprite<'_>, LayerError>) -> Option<LayerSheet> {
    let layer = layer.ok()?;
    // An `rlëD` header counts its frames in a u16, and the decoder rejects
    // a sheet with none (`GraphicsError::NoFrames`).
    let frames = NonZeroU16::new(layer.sheet.frames().len() as u16)
        .expect("a decoded rlëD has at least one frame");
    Some(LayerSheet {
        image_id: layer.image_id,
        frames,
    })
}

/// Decodes on every call; the flight screen asks once, when it opens.
impl StatusBars for GameData {
    fn government_interface(&self, id: GovtId) -> Result<i16, String> {
        match self.get::<Govt>(id.0) {
            Some(Ok(govt)) => Ok(govt.record.interface),
            Some(Err(err)) => Err(err.to_string()),
            None => Err(format!("no gövt {}", id.0)),
        }
    }

    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
        let intf = match self.get::<Interface>(id) {
            Some(Ok(intf)) => intf.record,
            Some(Err(err)) => return Err(err.to_string()),
            None => return Err(format!("no ïntf {id}")),
        };
        Ok(StatusBarLayout {
            radar: bounds(intf.radar_area),
            shield: bounds(intf.shield_area),
            armor: bounds(intf.armor_area),
            fuel: bounds(intf.fuel_area),
            nav: bounds(intf.nav_area),
            cargo: bounds(intf.cargo_area),
            bright_text: Color::from_rgb24(intf.bright_text),
            dim_text: Color::from_rgb24(intf.dim_text),
            bright_radar: Color::from_rgb24(intf.bright_radar),
            dim_radar: Color::from_rgb24(intf.dim_radar),
            shield_color: Color::from_rgb24(intf.shield_color),
            armor_color: Color::from_rgb24(intf.armor_color),
            fuel_full: Color::from_rgb24(intf.fuel_full),
            fuel_partial: Color::from_rgb24(intf.fuel_partial),
            font: Font::named(intf.status_font.as_str()),
            font_size: f32::from(intf.stat_font_size),
            status_bkgnd: intf.status_bkgnd,
        })
    }

    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        let found = self.resource(PICT, id)?;
        let picture = decode_pict(found.resource.data()).ok()?;
        Some((picture.width(), picture.height()))
    }
}

fn bounds(rect: nova_data::Rect) -> Bounds {
    Bounds {
        min: Point::new(f32::from(rect.left), f32::from(rect.top)),
        max: Point::new(f32::from(rect.right), f32::from(rect.bottom)),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::io;
    use std::path::Path;

    use nova_data::Record;
    use nova_data::graphics::RLED;
    use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
    use nova_data::records::ship::Ship;
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};
    use nova_sim::Blink;

    use super::*;

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

    fn put_i16(bytes: &mut [u8], at: usize, value: i16) {
        bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
    }

    /// A `shän` with base image `base`, `sets` sets of `frames_per`
    /// frames, and no glow or lights.
    fn anim(base: i16, sets: i16, frames_per: i16) -> Vec<u8> {
        layered(base, sets, frames_per, 0, 0)
    }

    /// A `shän` like [`anim`]'s, with glow image `glow` and lights image
    /// `lights`.
    fn layered(base: i16, sets: i16, frames_per: i16, glow: i16, lights: i16) -> Vec<u8> {
        let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
        put_i16(&mut bytes, 0x00, base);
        put_i16(&mut bytes, 0x04, sets);
        put_i16(&mut bytes, 0x16, glow);
        put_i16(&mut bytes, 0x1E, lights);
        put_i16(&mut bytes, 0x34, frames_per);
        bytes
    }

    fn layer(image_id: i16, frames: u16) -> LayerSheet {
        LayerSheet {
            image_id,
            frames: NonZeroU16::new(frames).expect("non-zero"),
        }
    }

    /// An `rlëD` of `frames` 3 x 2 frames.
    fn sheet(frames: u16) -> Vec<u8> {
        (0..frames)
            .fold(RledBuilder::new(3, 2), |b, _| {
                b.frame(|f| f.line().pixels(&[0x7C00; 3]).line().pixels(&[0x7C00; 3]))
            })
            .build()
    }

    fn ship() -> Vec<u8> {
        vec![0; Ship::SIZE.expect("fixed")]
    }

    #[test]
    fn a_ships_sheet_has_its_image_rotations_and_frame_size() {
        // Two sets of 8 rotations: 16 frames, a turn every 8.
        let data = store(&[
            (Ship::TYPE, 128, ship()),
            (ShipAnim::TYPE, 128, anim(1000, 2, 8)),
            (RLED, 1000, sheet(16)),
        ]);
        assert_eq!(
            data.ship_sheet(ShipId(128)),
            Ok(ShipSheet {
                image_id: 1000,
                rotations: NonZeroU16::new(8).expect("non-zero"),
                frame_width: 3,
                frame_height: 2,
                glow: None,
                lights: None,
                blink: Blink::STEADY,
            })
        );
    }

    #[test]
    fn a_ships_sheet_carries_its_shans_blink() {
        // The Shuttle's: mode 1, A=4 B=1 C=2 D=20.
        let mut shan = layered(1000, 1, 8, 0, 1200);
        for (at, value) in [(0x36, 1), (0x38, 4), (0x3A, 1), (0x3C, 2), (0x3E, 20)] {
            put_i16(&mut shan, at, value);
        }
        let data = store(&[
            (Ship::TYPE, 128, ship()),
            (ShipAnim::TYPE, 128, shan),
            (RLED, 1000, sheet(8)),
            (RLED, 1200, sheet(8)),
        ]);
        assert_eq!(
            data.ship_sheet(ShipId(128)).map(|sheet| sheet.blink),
            Ok(Blink {
                mode: 1,
                a: 4,
                b: 1,
                c: 2,
                d: 20,
            })
        );
    }

    #[test]
    fn a_ships_sheet_carries_its_glow_and_lights_layers() {
        // The glow has as many frames as the base; the lights one set of
        // rotations under the two-set base.
        let data = store(&[
            (Ship::TYPE, 128, ship()),
            (ShipAnim::TYPE, 128, layered(1000, 2, 8, 1100, 1200)),
            (RLED, 1000, sheet(16)),
            (RLED, 1100, sheet(16)),
            (RLED, 1200, sheet(8)),
        ]);
        let found = data.ship_sheet(ShipId(128)).expect("a sheet");
        assert_eq!(found.image_id, 1000);
        assert_eq!(found.glow, Some(layer(1100, 16)));
        assert_eq!(found.lights, Some(layer(1200, 8)));
    }

    #[test]
    fn a_layer_the_shan_does_not_name_is_none() {
        let data = store(&[
            (Ship::TYPE, 128, ship()),
            (ShipAnim::TYPE, 128, layered(1000, 1, 8, 0, -1)),
            (RLED, 1000, sheet(8)),
        ]);
        let found = data.ship_sheet(ShipId(128)).expect("a sheet");
        assert_eq!((found.glow, found.lights), (None, None));
    }

    #[test]
    fn a_broken_layer_is_none_and_never_hides_the_other() {
        // Ship 128's glow `rlëD` is missing; ship 129's lights `rlëD` does
        // not decode.
        let data = store(&[
            (Ship::TYPE, 128, ship()),
            (ShipAnim::TYPE, 128, layered(1000, 1, 8, 1100, 1200)),
            (Ship::TYPE, 129, ship()),
            (ShipAnim::TYPE, 129, layered(1000, 1, 8, 1200, 1300)),
            (RLED, 1000, sheet(8)),
            (RLED, 1200, sheet(8)),
            (RLED, 1300, vec![0; 4]),
        ]);
        let missing = data.ship_sheet(ShipId(128)).expect("a sheet");
        assert_eq!((missing.glow, missing.lights), (None, Some(layer(1200, 8))));
        let undecodable = data.ship_sheet(ShipId(129)).expect("a sheet");
        assert_eq!(
            (undecodable.glow, undecodable.lights),
            (Some(layer(1200, 8)), None)
        );
    }

    #[test]
    fn a_ship_whose_sprite_cannot_be_resolved_says_why() {
        let data = store(&[
            (Ship::TYPE, 128, ship()),
            (Ship::TYPE, 129, ship()),
            (ShipAnim::TYPE, 129, anim(1001, 1, 36)),
        ]);
        assert_eq!(
            data.ship_sheet(ShipId(128)),
            Err("no shän 128 for shïp 128".to_owned())
        );
        assert_eq!(
            data.ship_sheet(ShipId(129)),
            Err("shïp 129: no rlëD 1001 for its base image".to_owned())
        );
        assert_eq!(data.ship_sheet(ShipId(140)), Err("no shïp 140".to_owned()));
    }

    // Status bars.

    fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_be_bytes());
    }

    /// Writes `(top, left, bottom, right)` at `at`.
    fn put_rect(bytes: &mut [u8], at: usize, (top, left, bottom, right): (i16, i16, i16, i16)) {
        for (i, edge) in [top, left, bottom, right].into_iter().enumerate() {
            put_i16(bytes, at + 2 * i, edge);
        }
    }

    /// Stock `ïntf` 128's areas, distinct colours 1 to 8, `font` at
    /// `size`, and background `bkgnd`.
    fn interface(font: &str, size: i16, bkgnd: i16) -> Vec<u8> {
        let mut bytes = vec![0; Interface::SIZE.expect("fixed")];
        put_u32(&mut bytes, 0x00, 0x00FF_FFFF);
        put_u32(&mut bytes, 0x04, 0x0080_8080);
        put_rect(&mut bytes, 0x08, (8, 8, 184, 184));
        put_u32(&mut bytes, 0x10, 0x0000_FF00);
        put_u32(&mut bytes, 0x14, 0x0000_8000);
        put_rect(&mut bytes, 0x18, (199, 35, 206, 184));
        put_u32(&mut bytes, 0x20, 0x0000_00FF);
        put_rect(&mut bytes, 0x24, (216, 35, 223, 184));
        put_u32(&mut bytes, 0x2C, 0x00FF_0000);
        put_rect(&mut bytes, 0x30, (234, 35, 241, 184));
        put_u32(&mut bytes, 0x38, 0x00FF_FF00);
        put_u32(&mut bytes, 0x3C, 0x0080_8000);
        put_rect(&mut bytes, 0x40, (254, 8, 286, 184));
        put_rect(&mut bytes, 0x58, (458, 8, 552, 184));
        bytes[0x60..0x60 + font.len()].copy_from_slice(font.as_bytes());
        put_i16(&mut bytes, 0xA0, size);
        put_i16(&mut bytes, 0xA4, bkgnd);
        bytes
    }

    /// A `gövt` whose `Interface` is `interface`.
    fn govt(interface: i16) -> Vec<u8> {
        let mut bytes = vec![0; Govt::SIZE.expect("fixed")];
        put_i16(&mut bytes, 0xAC, interface);
        bytes
    }

    /// A grey `width` x `height` `PICT`.
    fn picture(width: i16, height: i16) -> Vec<u8> {
        let bounds = [0, 0, height, width];
        let pixels = vec![0x4210; (width * height) as usize];
        PictBuilder::new(bounds)
            .direct_bits(&DirectBits::rgb555(bounds, &pixels))
            .end()
            .build()
    }

    fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
        Bounds {
            min: Point::new(left, top),
            max: Point::new(right, bottom),
        }
    }

    #[test]
    fn a_status_bar_is_its_areas_colours_font_and_raw_background() {
        let data = store(&[(Interface::TYPE, 128, interface("Charcoal", 12, 700))]);
        assert_eq!(
            data.status_bar(128),
            Ok(StatusBarLayout {
                radar: rect(8.0, 8.0, 184.0, 184.0),
                shield: rect(35.0, 199.0, 184.0, 206.0),
                armor: rect(35.0, 216.0, 184.0, 223.0),
                fuel: rect(35.0, 234.0, 184.0, 241.0),
                nav: rect(8.0, 254.0, 184.0, 286.0),
                cargo: rect(8.0, 458.0, 184.0, 552.0),
                bright_text: Color::rgba(255, 255, 255, 255),
                dim_text: Color::rgba(128, 128, 128, 255),
                bright_radar: Color::rgba(0, 255, 0, 255),
                dim_radar: Color::rgba(0, 128, 0, 255),
                shield_color: Color::rgba(0, 0, 255, 255),
                armor_color: Color::rgba(255, 0, 0, 255),
                fuel_full: Color::rgba(255, 255, 0, 255),
                fuel_partial: Color::rgba(128, 128, 0, 255),
                font: Font::Charcoal,
                font_size: 12.0,
                status_bkgnd: 700,
            })
        );
        let geneva = store(&[(Interface::TYPE, 128, interface("Geneva", 9, 0))]);
        let bar = geneva.status_bar(128).expect("decodes");
        assert_eq!((bar.font, bar.font_size), (Font::Geneva, 9.0));
        assert_eq!(bar.status_bkgnd, 0, "raw, below 128");
        let below = store(&[(Interface::TYPE, 128, interface("Geneva", 9, -1))]);
        assert_eq!(below.status_bar(128).map(|bar| bar.status_bkgnd), Ok(-1));
    }

    #[test]
    fn a_missing_or_short_status_bar_says_why() {
        let mut short = interface("Geneva", 12, 700);
        short.pop();
        let data = store(&[(Interface::TYPE, 129, short)]);
        assert_eq!(data.status_bar(128), Err("no ïntf 128".to_owned()));
        let Err(message) = data.status_bar(129) else {
            panic!("an error")
        };
        assert!(message.contains("129"), "{message}");
    }

    #[test]
    fn a_governments_interface_is_its_raw_field() {
        let mut short = govt(130);
        short.pop();
        let data = store(&[
            (Govt::TYPE, 128, govt(130)),
            (Govt::TYPE, 129, govt(-1)),
            (Govt::TYPE, 131, short),
        ]);
        assert_eq!(data.government_interface(GovtId(128)), Ok(130));
        assert_eq!(data.government_interface(GovtId(129)), Ok(-1));
        assert_eq!(
            data.government_interface(GovtId(130)),
            Err("no gövt 130".to_owned())
        );
        let Err(message) = data.government_interface(GovtId(131)) else {
            panic!("an error")
        };
        assert!(message.contains("131"), "{message}");
    }

    #[test]
    fn a_pictures_size_is_its_decoded_width_and_height() {
        let data = store(&[(PICT, 700, picture(194, 16)), (PICT, 701, vec![0; 4])]);
        assert_eq!(data.picture_size(700), Some((194, 16)));
        assert_eq!(data.picture_size(701), None, "undecodable");
        assert_eq!(data.picture_size(702), None, "missing");
    }
}
