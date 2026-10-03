//! Resolving a ship to its sprite sheet.

use nova_rsrc::ResType;

use super::{GameData, SourceFile};
use crate::error::DecodeError;
use crate::graphics::{GraphicsError, SpriteSheet, decode_rled};
use crate::records::ship::Ship;
use crate::records::ship_anim::ShipAnim;
use crate::wire::id::ShipId;

/// The `rlëD` resource type.
pub const RLED: ResType = ResType::new([b'r', b'l', 0x91, b'D']);

/// A ship's base sprite sheet and the files each step came from.
#[derive(Debug)]
pub struct ShipSprite<'a> {
    /// The decoded sheet, laid out with the `shän`'s frames per rotation.
    pub sheet: SpriteSheet,
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

impl GameData {
    /// Resolves `ship` to its base sprite sheet: the `shïp` must exist and
    /// decode; its `shän` shares its ID (as the Bible says); the `rlëD` is
    /// the `shän`'s base image, decoded with the `shän`'s layout (its frames
    /// per rotation as columns).
    ///
    /// The sheet is decoded on every call and not cached.
    pub fn ship_sprite(&self, ship: ShipId) -> Result<ShipSprite<'_>, SpriteError> {
        let id = ship.0;
        let ship = self
            .get::<Ship>(id)
            .ok_or(SpriteError::NoShip(id))?
            .map_err(|e| SpriteError::Decode(e.clone()))?;
        let anim = self
            .get::<ShipAnim>(id)
            .ok_or(SpriteError::NoShipAnim(id))?
            .map_err(|e| SpriteError::Decode(e.clone()))?;
        let image_id = anim.record.base_image_id;
        let found = self
            .resource(RLED, image_id)
            .ok_or(SpriteError::NoSheet { ship: id, image_id })?;
        let sheet = decode_rled(found.resource.data(), anim.record.sheet_layout())
            .map_err(|source| SpriteError::Graphics { image_id, source })?;
        Ok(ShipSprite {
            sheet,
            ship: ship.source,
            anim: anim.source,
            sheet_source: found.source,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

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
}
