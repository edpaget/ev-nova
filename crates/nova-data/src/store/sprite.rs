//! Resolving a ship to its sprite sheet and its glow and lights layers.

use std::fmt;

use super::{GameData, SourceFile, StoreEntry};
use crate::error::DecodeError;
use crate::graphics::{
    GraphicsError, RLED, RledInfo, SheetLayout, SpriteSheet, decode_rled, scan_rled,
};
use crate::records::ship::Ship;
use crate::records::ship_anim::ShipAnim;
use crate::wire::id::ShipId;

/// A ship's base sprite sheet and the files each step came from.
///
/// `S` is the sheet: decoded ([`SpriteSheet`]) by
/// [`GameData::ship_sprite`], or only scanned ([`SheetScan`]) by
/// [`GameData::ship_sprite_info`].
#[derive(Debug)]
pub struct ShipSprite<'a, S = SpriteSheet> {
    /// The base image's `rlëD` ID (the `shän`'s `BaseImageID`).
    pub image_id: i16,
    /// The sheet, laid out with the `shän`'s frames per rotation.
    pub sheet: S,
    /// The file the `shïp` came from.
    pub ship: &'a SourceFile,
    /// The file the `shän` came from.
    pub anim: &'a SourceFile,
    /// The file the `rlëD` came from.
    pub sheet_source: &'a SourceFile,
}

/// Why a ship's sprite could not be resolved.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SpriteError {
    /// There is no `shïp` with this ID.
    #[error("no shïp {0}")]
    NoShip(i16),
    /// There is no `shän` with the ship's ID.
    #[error("no shän {0} for shïp {0}")]
    NoShipAnim(i16),
    /// The `shïp` or `shän` failed to decode.
    #[error(transparent)]
    Decode(DecodeError),
    /// The `shän`'s base image has no `rlëD` (a `PICT` base image is not
    /// supported by this lookup).
    #[error("shïp {ship}: no rlëD {image_id} for its base image")]
    NoSheet {
        /// The ship's ID.
        ship: i16,
        /// The `shän`'s base image ID.
        image_id: i16,
    },
    /// The `rlëD` failed to decode.
    #[error("rlëD {image_id}: {source}")]
    Graphics {
        /// The `rlëD`'s ID.
        image_id: i16,
        /// What went wrong.
        source: GraphicsError,
    },
}

/// A sprite layer drawn over a ship's base image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShipLayer {
    /// The engine glow (`shän` `GlowImageID`).
    Glow,
    /// The running lights (`shän` `LightImageID`).
    Lights,
    /// The weapon glow (`shän` `WeapImageID`), which a `Flags2` 0x0200
    /// weapon lights.
    Weapon,
    /// The alt layer (`shän` `AltImageID`), drawn over the hull at its own
    /// set.
    Alt,
    /// The shield (`shän` `ShieldImageID`), flashed on a hit.
    Shield,
}

impl fmt::Display for ShipLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Glow => "glow",
            Self::Lights => "lights",
            Self::Weapon => "weapon",
            Self::Alt => "alt",
            Self::Shield => "shield",
        })
    }
}

/// A ship's base sheet as [`GameData::ship_sprite_info`] reads it: its
/// metadata without its pixels.
pub type ShipSpriteInfo<'a> = ShipSprite<'a, SheetScan>;

/// What a sheet would hold once decoded, read without decoding it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SheetScan {
    /// The frame size, frame count and whether it has runs.
    pub info: RledInfo,
    /// The layout the decoded sheet would have: the `shän`'s, or
    /// [`SheetLayout::DEFAULT`].
    pub layout: SheetLayout,
}

impl SheetScan {
    /// Scans `data` as [`decode_rled`] would decode it with `layout`.
    fn read(data: &[u8], layout: Option<SheetLayout>) -> Result<Self, GraphicsError> {
        Ok(Self {
            info: scan_rled(data)?,
            layout: layout.unwrap_or(SheetLayout::DEFAULT),
        })
    }
}

/// One layer's sprite sheet and the file it came from; `S` as for
/// [`ShipSprite`].
#[derive(Debug)]
pub struct LayerSprite<'a, S = SpriteSheet> {
    /// The layer's `rlëD` ID.
    pub image_id: i16,
    /// The sheet, laid out like the base (the `shän`'s frames per
    /// rotation as columns).
    pub sheet: S,
    /// The file the `rlëD` came from.
    pub source: &'a SourceFile,
}

/// Why one of a ship's layers could not be resolved.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LayerError {
    /// The `shän` names a layer image with no `rlëD`.
    #[error("shïp {ship}: no rlëD {image_id} for its {layer} image")]
    NoSheet {
        /// The ship's ID.
        ship: i16,
        /// Which layer.
        layer: ShipLayer,
        /// The layer's image ID.
        image_id: i16,
    },
    /// The layer's `rlëD` failed to decode.
    #[error("{layer} rlëD {image_id}: {source}")]
    Graphics {
        /// Which layer.
        layer: ShipLayer,
        /// The `rlëD`'s ID.
        image_id: i16,
        /// What went wrong.
        source: GraphicsError,
    },
}

/// A ship's glow, lights, weapon, alt and shield layers. Each is `None` when the `shän` does not
/// define it (an image ID of 0 or less), and otherwise resolves or fails on
/// its own, so one broken layer never hides the other.
///
/// `S` is the sheet, as for [`ShipSprite`].
#[derive(Debug)]
pub struct ShipLayers<'a, S = SpriteSheet> {
    /// The engine glow.
    pub glow: Option<Result<LayerSprite<'a, S>, LayerError>>,
    /// The running lights.
    pub lights: Option<Result<LayerSprite<'a, S>, LayerError>>,
    /// The weapon glow.
    pub weapon: Option<Result<LayerSprite<'a, S>, LayerError>>,
    /// The alt layer: defined only with an `AltSetCount` above 0 too, as
    /// `_LoadExtendedShipSprites` loads it (@0x22aec-0x22b1e).
    pub alt: Option<Result<LayerSprite<'a, S>, LayerError>>,
    /// The shield.
    pub shield: Option<Result<LayerSprite<'a, S>, LayerError>>,
}

/// A ship's layers as [`GameData::ship_layer_infos`] reads them: their
/// metadata without their pixels.
pub type ShipLayerInfos<'a> = ShipLayers<'a, SheetScan>;

/// Reads one `rlëD` as a sheet `S`, given the `shän`'s layout.
type ReadSheet<S> = fn(&[u8], Option<SheetLayout>) -> Result<S, GraphicsError>;

impl GameData {
    /// Resolves `ship`'s glow, lights, weapon, alt and shield layers through its `shän` (as
    /// [`GameData::ship_sprite`] does), each decoded with the base's
    /// layout: the Bible gives one `FramesPer` for every layer.
    ///
    /// A missing or undecodable `shïp` or `shän` is a [`SpriteError`]; a
    /// layer's own failure is that layer's [`LayerError`]. The sheets are
    /// decoded on every call and not cached.
    pub fn ship_layers(&self, ship: ShipId) -> Result<ShipLayers<'_>, SpriteError> {
        self.resolve_layers(ship, decode_rled)
    }

    /// What [`GameData::ship_layers`] resolves, with each sheet only
    /// scanned: its frame size, frame count, runs and layout, but no
    /// pixels. It fails exactly where and as that does.
    pub fn ship_layer_infos(&self, ship: ShipId) -> Result<ShipLayerInfos<'_>, SpriteError> {
        self.resolve_layers(ship, SheetScan::read)
    }

    fn resolve_layers<S>(
        &self,
        ship: ShipId,
        read: ReadSheet<S>,
    ) -> Result<ShipLayers<'_, S>, SpriteError> {
        let (_, anim) = self.ship_and_anim(ship)?;
        let layer = |layer, image_id: i16| {
            (image_id > 0).then(|| self.layer_sprite(ship.0, layer, image_id, anim.record, read))
        };
        Ok(ShipLayers {
            glow: layer(ShipLayer::Glow, anim.record.glow_image_id),
            lights: layer(ShipLayer::Lights, anim.record.light_image_id),
            weapon: layer(ShipLayer::Weapon, anim.record.weap_image_id),
            alt: (anim.record.alt_set_count > 0)
                .then(|| layer(ShipLayer::Alt, anim.record.alt_image_id))
                .flatten(),
            shield: layer(ShipLayer::Shield, anim.record.shield_image_id),
        })
    }

    fn layer_sprite<S>(
        &self,
        ship: i16,
        layer: ShipLayer,
        image_id: i16,
        anim: &ShipAnim,
        read: ReadSheet<S>,
    ) -> Result<LayerSprite<'_, S>, LayerError> {
        let found = self.resource(RLED, image_id).ok_or(LayerError::NoSheet {
            ship,
            layer,
            image_id,
        })?;
        let sheet = read(found.resource.data(), anim.sheet_layout()).map_err(|source| {
            LayerError::Graphics {
                layer,
                image_id,
                source,
            }
        })?;
        Ok(LayerSprite {
            image_id,
            sheet,
            source: found.source,
        })
    }

    /// Resolves `ship` to its base sprite sheet: the `shïp` must exist and
    /// decode; its `shän` shares its ID (as the Bible says); the `rlëD` is
    /// the `shän`'s base image, decoded with the `shän`'s layout (its frames
    /// per rotation as columns).
    ///
    /// The sheet is decoded on every call and not cached.
    pub fn ship_sprite(&self, ship: ShipId) -> Result<ShipSprite<'_>, SpriteError> {
        self.resolve_sprite(ship, decode_rled)
    }

    /// What [`GameData::ship_sprite`] resolves, with the sheet only
    /// scanned: its frame size, frame count, runs and layout, but no
    /// pixels. It fails exactly where and as that does.
    pub fn ship_sprite_info(&self, ship: ShipId) -> Result<ShipSpriteInfo<'_>, SpriteError> {
        self.resolve_sprite(ship, SheetScan::read)
    }

    fn resolve_sprite<S>(
        &self,
        ship: ShipId,
        read: ReadSheet<S>,
    ) -> Result<ShipSprite<'_, S>, SpriteError> {
        let id = ship.0;
        let (ship, anim) = self.ship_and_anim(ship)?;
        let image_id = anim.record.base_image_id;
        let found = self
            .resource(RLED, image_id)
            .ok_or(SpriteError::NoSheet { ship: id, image_id })?;
        let sheet = read(found.resource.data(), anim.record.sheet_layout())
            .map_err(|source| SpriteError::Graphics { image_id, source })?;
        Ok(ShipSprite {
            image_id,
            sheet,
            ship: ship.source,
            anim: anim.source,
            sheet_source: found.source,
        })
    }

    /// The `shïp` and its `shän` (same ID), both decoded.
    fn ship_and_anim(
        &self,
        ship: ShipId,
    ) -> Result<(StoreEntry<'_, Ship>, StoreEntry<'_, ShipAnim>), SpriteError> {
        let id = ship.0;
        let ship = self
            .get::<Ship>(id)
            .ok_or(SpriteError::NoShip(id))?
            .map_err(|e| SpriteError::Decode(e.clone()))?;
        let anim = self
            .get::<ShipAnim>(id)
            .ok_or(SpriteError::NoShipAnim(id))?
            .map_err(|e| SpriteError::Decode(e.clone()))?;
        Ok((ship, anim))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_rsrc::ResType;
    use nova_rsrc::fixture::ForkBuilder;

    use super::*;
    use crate::decode::Record;
    use crate::graphics::fixture::RledBuilder;
    use crate::store::fake::{FakeForks, FakeTree};
    use crate::store::fs::EntryKind::File;
    use crate::testutil::buf;

    /// A `shän` with base image `image`, `sets` sprite sets and
    /// `frames_per` frames per rotation.
    fn anim(image: i16, sets: i16, frames_per: i16) -> Vec<u8> {
        buf::<ShipAnim>()
            .i16(0x00, image)
            .i16(0x04, sets)
            .i16(0x34, frames_per)
            .0
    }

    /// An `rlëD` of `frames` 2x1 frames whose pixels are `color`.
    fn sheet(frames: u16, color: u16) -> Vec<u8> {
        let mut builder = RledBuilder::new(2, 1);
        for _ in 0..frames {
            builder = builder.frame(|f| f.line().pixels(&[color, color]));
        }
        builder.build()
    }

    fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> Vec<u8> {
        resources
            .iter()
            .fold(ForkBuilder::new(), |b, (ty, id, data)| {
                b.resource(*ty, *id, None, data)
            })
            .build()
            .bytes
    }

    fn ship() -> Vec<u8> {
        buf::<Ship>().0
    }

    /// Ship 128 with its `shïp`, `shän` (base image 1000, 3 sets of 4
    /// frames) and `rlëD` in three files, plus `extra` plug-ins.
    fn store(extra: &[(&str, Vec<u8>)]) -> GameData {
        let plugins: Vec<(&str, _)> = extra.iter().map(|(name, _)| (*name, File)).collect();
        let tree = FakeTree::new()
            .dir("/d", &[("ships", File), ("anims", File), ("sprites", File)])
            .dir("/p", &plugins);
        let mut forks = FakeForks::new()
            .file("/d/ships", fork(&[(Ship::TYPE, 128, ship())]))
            .file("/d/anims", fork(&[(ShipAnim::TYPE, 128, anim(1000, 3, 4))]))
            .file("/d/sprites", fork(&[(RLED, 1000, sheet(12, 0x7C00))]));
        for (name, bytes) in extra {
            forks = forks.file(&format!("/p/{name}"), bytes.clone());
        }
        GameData::load(&tree, &forks, Path::new("/d"), Some(Path::new("/p"))).expect("opens")
    }

    #[test]
    fn a_ship_resolves_through_its_shan_to_its_sheet() {
        let data = store(&[]);
        let sprite = data.ship_sprite(ShipId(128)).expect("resolves");
        assert_eq!(sprite.image_id, 1000);
        assert_eq!(sprite.sheet.frames().len(), 3 * 4);
        assert_eq!(sprite.sheet.layout().columns(), 4);
        assert_eq!(sprite.sheet.frame_width(), 2);
        assert_eq!(sprite.sheet.frames()[0].pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(sprite.ship.path, Path::new("/d/ships"));
        assert_eq!(sprite.anim.path, Path::new("/d/anims"));
        assert_eq!(sprite.sheet_source.path, Path::new("/d/sprites"));
    }

    #[test]
    fn a_plugin_overriding_only_the_sheet_changes_its_source() {
        let data = store(&[("New Paint", fork(&[(RLED, 1000, sheet(12, 0x001F))]))]);
        let sprite = data.ship_sprite(ShipId(128)).expect("resolves");
        assert_eq!(sprite.sheet_source.path, Path::new("/p/New Paint"));
        assert_eq!(sprite.ship.path, Path::new("/d/ships"));
        assert_eq!(sprite.anim.path, Path::new("/d/anims"));
        assert_eq!(sprite.sheet.frames()[0].pixel(0, 0), Some([0, 0, 255, 255]));
    }

    #[test]
    fn a_missing_ship_is_no_ship() {
        let err = store(&[]).ship_sprite(ShipId(129)).expect_err("absent");
        assert_eq!(err, SpriteError::NoShip(129));
        assert_eq!(err.to_string(), "no shïp 129");
    }

    #[test]
    fn a_ship_without_a_shan_is_no_ship_anim() {
        let data = store(&[("Ship 129", fork(&[(Ship::TYPE, 129, ship())]))]);
        let err = data.ship_sprite(ShipId(129)).expect_err("no shän");
        assert_eq!(err, SpriteError::NoShipAnim(129));
        assert_eq!(err.to_string(), "no shän 129 for shïp 129");
    }

    #[test]
    fn a_ship_or_shan_that_fails_to_decode_is_a_decode_error() {
        let short_ship = fork(&[(Ship::TYPE, 128, ship()[1..].to_vec())]);
        let data = store(&[("Broken", short_ship)]);
        let err = data.ship_sprite(ShipId(128)).expect_err("bad shïp");
        let SpriteError::Decode(inner) = &err else {
            panic!("a decode error: {err:?}")
        };
        assert_eq!((inner.res_type, inner.id), (Ship::TYPE, 128));
        assert_eq!(err.to_string(), inner.to_string());

        let short_anim = fork(&[(ShipAnim::TYPE, 128, anim(1000, 3, 4)[1..].to_vec())]);
        let data = store(&[("Broken", short_anim)]);
        let err = data.ship_sprite(ShipId(128)).expect_err("bad shän");
        assert!(
            matches!(&err, SpriteError::Decode(e) if e.res_type == ShipAnim::TYPE && e.id == 128),
            "{err:?}"
        );
    }

    #[test]
    fn a_base_image_with_no_sheet_is_no_sheet() {
        let other_image = fork(&[(ShipAnim::TYPE, 128, anim(1001, 3, 4))]);
        let data = store(&[("Other", other_image)]);
        let err = data.ship_sprite(ShipId(128)).expect_err("no rlëD 1001");
        assert_eq!(
            err,
            SpriteError::NoSheet {
                ship: 128,
                image_id: 1001
            }
        );
        assert_eq!(err.to_string(), "shïp 128: no rlëD 1001 for its base image");
    }

    #[test]
    fn a_corrupt_sheet_is_a_graphics_error() {
        let eight_bit = RledBuilder::new(2, 1).depth(8).build();
        let data = store(&[("Bad Paint", fork(&[(RLED, 1000, eight_bit)]))]);
        let err = data.ship_sprite(ShipId(128)).expect_err("8-bit");
        assert_eq!(
            err,
            SpriteError::Graphics {
                image_id: 1000,
                source: GraphicsError::UnsupportedDepth { depth: 8 },
            }
        );
        assert_eq!(err.to_string(), "rlëD 1000: unsupported rlëD depth 8");
    }

    /// A plug-in, `Layers`, giving ship 128 a `shän` with glow image `glow`
    /// and lights image `lights` (base 1000, 3 sets of 4), plus `sheets`.
    fn layered(glow: i16, lights: i16, sheets: &[(i16, Vec<u8>)]) -> (&'static str, Vec<u8>) {
        let mut resources = vec![(ShipAnim::TYPE, 128, layered_anim(glow, lights))];
        resources.extend(sheets.iter().map(|(id, bytes)| (RLED, *id, bytes.clone())));
        ("Layers", fork(&resources))
    }

    fn layered_anim(glow: i16, lights: i16) -> Vec<u8> {
        let mut bytes = anim(1000, 3, 4);
        bytes[0x16..0x18].copy_from_slice(&glow.to_be_bytes());
        bytes[0x1E..0x20].copy_from_slice(&lights.to_be_bytes());
        bytes
    }

    /// Glow 1100 (one set of 4 frames) and lights 1200 (3 sets of 4).
    fn both_layers() -> (&'static str, Vec<u8>) {
        layered(
            1100,
            1200,
            &[(1100, sheet(4, 0x03E0)), (1200, sheet(12, 0x001F))],
        )
    }

    #[test]
    fn glow_and_lights_resolve_with_the_shans_layout() {
        let data = store(&[both_layers()]);
        let layers = data.ship_layers(ShipId(128)).expect("resolves");
        let glow = layers.glow.expect("defined").expect("resolves");
        assert_eq!(glow.image_id, 1100);
        assert_eq!(glow.sheet.frames().len(), 4);
        assert_eq!(glow.sheet.layout().columns(), 4);
        assert_eq!(glow.sheet.frames()[0].pixel(0, 0), Some([0, 255, 0, 255]));
        assert_eq!(glow.source.path, Path::new("/p/Layers"));
        let lights = layers.lights.expect("defined").expect("resolves");
        assert_eq!(lights.image_id, 1200);
        assert_eq!(lights.sheet.frames().len(), 12);
        assert_eq!(lights.sheet.layout().columns(), 4);
        assert_eq!(lights.sheet.frames()[0].pixel(0, 0), Some([0, 0, 255, 255]));
    }

    #[test]
    fn the_weapon_layer_resolves_with_the_shans_layout() {
        let mut bytes = layered_anim(0, 0);
        bytes[0x26..0x28].copy_from_slice(&1300_i16.to_be_bytes());
        let resources = vec![(ShipAnim::TYPE, 128, bytes), (RLED, 1300, sheet(8, 0x7C00))];
        let data = store(&[("Layers", fork(&resources))]);
        let layers = data.ship_layers(ShipId(128)).expect("resolves");
        let weapon = layers.weapon.expect("defined").expect("resolves");
        assert_eq!(weapon.image_id, 1300);
        assert_eq!(weapon.sheet.frames().len(), 8);
        assert_eq!(weapon.sheet.layout().columns(), 4);
        assert!(layers.glow.is_none() && layers.lights.is_none());
        assert_eq!(ShipLayer::Weapon.to_string(), "weapon");
        let plain = store(&[]);
        let layers = plain.ship_layers(ShipId(128)).expect("resolves");
        assert!(layers.weapon.is_none(), "0 is undefined");
    }

    #[test]
    fn the_alt_and_shield_layers_resolve_with_the_shans_layout() {
        let mut bytes = layered_anim(0, 0);
        // `AltImageID` 1400, `AltSetCount` 2; `ShieldImageID` 1500.
        bytes[0x0C..0x0E].copy_from_slice(&1400_i16.to_be_bytes());
        bytes[0x10..0x12].copy_from_slice(&2_i16.to_be_bytes());
        bytes[0x40..0x42].copy_from_slice(&1500_i16.to_be_bytes());
        let resources = vec![
            (ShipAnim::TYPE, 128, bytes.clone()),
            (RLED, 1400, sheet(8, 0x7C00)),
            (RLED, 1500, sheet(4, 0x001F)),
        ];
        let data = store(&[("Layers", fork(&resources))]);
        let layers = data.ship_layers(ShipId(128)).expect("resolves");
        let alt = layers.alt.expect("defined").expect("resolves");
        assert_eq!((alt.image_id, alt.sheet.frames().len()), (1400, 8));
        assert_eq!(alt.sheet.layout().columns(), 4);
        let shield = layers.shield.expect("defined").expect("resolves");
        assert_eq!((shield.image_id, shield.sheet.frames().len()), (1500, 4));
        assert_eq!(ShipLayer::Alt.to_string(), "alt");
        assert_eq!(ShipLayer::Shield.to_string(), "shield");
        // No alt sets: no alt layer, whatever its image.
        bytes[0x10..0x12].copy_from_slice(&0_i16.to_be_bytes());
        let resources = vec![(ShipAnim::TYPE, 128, bytes), (RLED, 1400, sheet(8, 0x7C00))];
        let data = store(&[("Layers", fork(&resources))]);
        let layers = data.ship_layers(ShipId(128)).expect("resolves");
        assert!(layers.alt.is_none());
        assert!(layers.shield.is_some(), "its shield still named");
        let plain = store(&[]);
        let layers = plain.ship_layers(ShipId(128)).expect("resolves");
        assert!(layers.alt.is_none() && layers.shield.is_none());
    }

    #[test]
    fn a_layer_id_of_minus_one_or_zero_is_undefined() {
        for (glow, lights) in [(-1, 0), (0, -1)] {
            let data = store(&[layered(glow, lights, &[])]);
            let layers = data.ship_layers(ShipId(128)).expect("resolves");
            assert!(layers.glow.is_none(), "glow {glow}");
            assert!(layers.lights.is_none(), "lights {lights}");
        }
        // The plain fixture's `shän` leaves both at 0.
        let data = store(&[]);
        let layers = data.ship_layers(ShipId(128)).expect("resolves");
        assert!(layers.glow.is_none() && layers.lights.is_none());
    }

    #[test]
    fn a_layer_with_no_sheet_is_no_sheet_for_that_layer() {
        let data = store(&[layered(1100, 1200, &[(1200, sheet(12, 0x001F))])]);
        let layers = data.ship_layers(ShipId(128)).expect("resolves");
        let err = layers.glow.expect("defined").expect_err("no rlëD 1100");
        assert_eq!(
            err,
            LayerError::NoSheet {
                ship: 128,
                layer: ShipLayer::Glow,
                image_id: 1100
            }
        );
        assert_eq!(err.to_string(), "shïp 128: no rlëD 1100 for its glow image");
        assert!(layers.lights.expect("defined").is_ok());

        let data = store(&[layered(1100, 1200, &[(1100, sheet(4, 0x03E0))])]);
        let layers = data.ship_layers(ShipId(128)).expect("resolves");
        let err = layers.lights.expect("defined").expect_err("no rlëD 1200");
        assert_eq!(
            err.to_string(),
            "shïp 128: no rlëD 1200 for its lights image"
        );
        assert!(layers.glow.expect("defined").is_ok());
    }

    #[test]
    fn a_corrupt_layer_sheet_is_a_graphics_error_for_that_layer_only() {
        let eight_bit = RledBuilder::new(2, 1).depth(8).build();
        let data = store(&[layered(
            1100,
            1200,
            &[(1100, eight_bit), (1200, sheet(12, 0x001F))],
        )]);
        let layers = data.ship_layers(ShipId(128)).expect("resolves");
        let err = layers.glow.expect("defined").expect_err("8-bit");
        assert_eq!(
            err,
            LayerError::Graphics {
                layer: ShipLayer::Glow,
                image_id: 1100,
                source: GraphicsError::UnsupportedDepth { depth: 8 },
            }
        );
        assert_eq!(err.to_string(), "glow rlëD 1100: unsupported rlëD depth 8");
        assert_eq!(
            layers.lights.expect("defined").expect("resolves").image_id,
            1200
        );
        // The base still resolves.
        assert!(data.ship_sprite(ShipId(128)).is_ok());
    }

    #[test]
    fn a_plugin_overriding_only_the_glow_sheet_changes_its_source() {
        let paint = ("Zed Paint", fork(&[(RLED, 1100, sheet(4, 0x7C00))]));
        let data = store(&[both_layers(), paint]);
        let layers = data.ship_layers(ShipId(128)).expect("resolves");
        let glow = layers.glow.expect("defined").expect("resolves");
        assert_eq!(glow.source.path, Path::new("/p/Zed Paint"));
        assert_eq!(glow.sheet.frames()[0].pixel(0, 0), Some([255, 0, 0, 255]));
        let lights = layers.lights.expect("defined").expect("resolves");
        assert_eq!(lights.source.path, Path::new("/p/Layers"));
    }

    #[test]
    fn layers_of_a_missing_ship_or_shan_are_sprite_errors() {
        let data = store(&[("Ship 129", fork(&[(Ship::TYPE, 129, ship())]))]);
        assert_eq!(
            data.ship_layers(ShipId(130)).expect_err("absent"),
            SpriteError::NoShip(130)
        );
        assert_eq!(
            data.ship_layers(ShipId(129)).expect_err("no shän"),
            SpriteError::NoShipAnim(129)
        );
    }

    /// What a scan should read from a decoded sheet.
    fn info_of(sheet: &SpriteSheet) -> SheetScan {
        SheetScan {
            info: RledInfo {
                frame_width: sheet.frame_width(),
                frame_height: sheet.frame_height(),
                frames: sheet.frames().len() as u16,
                has_runs: sheet.solid_frames().is_some(),
            },
            layout: sheet.layout(),
        }
    }

    /// `ship_sprite_info` gives what `ship_sprite` decodes, minus pixels.
    fn assert_sprite_infos_agree(data: &GameData, ship: ShipId) {
        let decoded = data.ship_sprite(ship).map(|s| ShipSprite {
            image_id: s.image_id,
            sheet: info_of(&s.sheet),
            ship: s.ship,
            anim: s.anim,
            sheet_source: s.sheet_source,
        });
        let scanned = data.ship_sprite_info(ship);
        assert_eq!(
            format!("{scanned:?}"),
            format!("{decoded:?}"),
            "ship {ship:?}"
        );
    }

    /// `ship_layer_infos` gives what `ship_layers` decodes, minus pixels.
    fn assert_layer_infos_agree(data: &GameData, ship: ShipId) {
        fn info(
            layer: Option<Result<LayerSprite<'_>, LayerError>>,
        ) -> Option<Result<LayerSprite<'_, SheetScan>, LayerError>> {
            layer.map(|l| {
                l.map(|l| LayerSprite {
                    image_id: l.image_id,
                    sheet: info_of(&l.sheet),
                    source: l.source,
                })
            })
        }
        let decoded = data.ship_layers(ship).map(|l| ShipLayers {
            glow: info(l.glow),
            lights: info(l.lights),
            weapon: info(l.weapon),
            alt: info(l.alt),
            shield: info(l.shield),
        });
        let scanned = data.ship_layer_infos(ship);
        assert_eq!(
            format!("{scanned:?}"),
            format!("{decoded:?}"),
            "ship {ship:?}"
        );
    }

    /// Every layer defined, the glow with runs, as one plug-in.
    fn every_layer(alt_sets: i16) -> (&'static str, Vec<u8>) {
        let mut bytes = layered_anim(1100, 1200);
        bytes[0x26..0x28].copy_from_slice(&1300_i16.to_be_bytes());
        bytes[0x0C..0x0E].copy_from_slice(&1400_i16.to_be_bytes());
        bytes[0x10..0x12].copy_from_slice(&alt_sets.to_be_bytes());
        bytes[0x40..0x42].copy_from_slice(&1500_i16.to_be_bytes());
        let glow = RledBuilder::new(3, 1)
            .frame(|f| f.line().run(2, 0x7FFF, 0x7FFF))
            .build();
        let resources = vec![
            (ShipAnim::TYPE, 128, bytes),
            (RLED, 1100, glow),
            (RLED, 1200, sheet(12, 0x001F)),
            (RLED, 1300, sheet(8, 0x7C00)),
            (RLED, 1400, sheet(6, 0x7C00)),
            (RLED, 1500, sheet(4, 0x001F)),
        ];
        ("Layers", fork(&resources))
    }

    #[test]
    fn a_ships_infos_agree_with_its_decoded_sheets() {
        let cases = [
            ("every layer", store(&[every_layer(2)])),
            ("no alt sets", store(&[every_layer(0)])),
            ("no layers", store(&[])),
            ("no layer sheet", store(&[layered(1100, 1200, &[])])),
            (
                "broken layer sheet",
                store(&[layered(
                    1100,
                    1200,
                    &[(1100, RledBuilder::new(2, 1).depth(8).build())],
                )]),
            ),
            (
                "broken base sheet",
                store(&[(
                    "Bad Paint",
                    fork(&[(RLED, 1000, RledBuilder::new(2, 1).depth(8).build())]),
                )]),
            ),
            (
                "no base sheet",
                store(&[("Other", fork(&[(ShipAnim::TYPE, 128, anim(1001, 3, 4))]))]),
            ),
            (
                "no shän",
                store(&[("Ship 129", fork(&[(Ship::TYPE, 129, ship())]))]),
            ),
        ];
        for (_, data) in &cases {
            for ship in [ShipId(128), ShipId(129), ShipId(130)] {
                assert_sprite_infos_agree(data, ship);
                assert_layer_infos_agree(data, ship);
            }
        }
        let layers = cases[0].1.ship_layer_infos(ShipId(128)).expect("resolves");
        let glow = layers.glow.expect("defined").expect("resolves");
        assert!(glow.sheet.info.has_runs);
        assert_eq!(glow.sheet.layout.columns(), 4);
        assert!(layers.alt.is_some());
        assert!(
            cases[1]
                .1
                .ship_layer_infos(ShipId(128))
                .unwrap()
                .alt
                .is_none()
        );
    }

    #[test]
    fn layers_display_their_names() {
        assert_eq!(ShipLayer::Glow.to_string(), "glow");
        assert_eq!(ShipLayer::Lights.to_string(), "lights");
    }
}
