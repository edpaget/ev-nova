//! The ship catalog over the game data: a thin mapping from `GameData`'s
//! ship, sprite, layer and description lookups to [`ShipEntry`].

use std::num::NonZeroU16;

use nova_data::graphics::SpriteSheet;
use nova_data::records::ship::Ship;
use nova_data::{GameData, LayerError, LayerSprite, Record, StoreEntry};
use nova_sim::data::ship_blink;

use super::catalog::{SheetInfo, ShipCatalog, ShipEntry, ShipId, ShipStats};

/// Each lookup is decoded afresh on every call; the browser asks once per
/// selection.
impl ShipCatalog for GameData {
    fn ship_ids(&self) -> Vec<ShipId> {
        self.ids(Ship::TYPE).iter().copied().map(ShipId).collect()
    }

    fn ship(&self, id: ShipId) -> ShipEntry {
        let (name, stats) = match self.get::<Ship>(id.0) {
            Some(Ok(ship)) => (name(&ship), Ok(stats(ship.record))),
            Some(Err(err)) => (fallback_name(id), Err(err.to_string())),
            None => (fallback_name(id), Err(format!("no shïp {}", id.0))),
        };
        let description = match self.ship_description(id) {
            Some(Ok(desc)) => Ok(Some(desc.record.text.as_str().replace('\r', "\n"))),
            Some(Err(err)) => Err(err.to_string()),
            None => Ok(None),
        };
        let sprite = self
            .ship_sprite(id)
            .map_err(|err| err.to_string())
            .map(|sprite| sheet_info(sprite.image_id, &sprite.sheet));
        // A ship whose layers cannot be looked up has no sprite either, and
        // the sprite's error already says why.
        let (glow, lights) = self.ship_layers(id).map_or((None, None), |layers| {
            (layers.glow.map(layer_info), layers.lights.map(layer_info))
        });
        ShipEntry {
            id,
            name,
            stats,
            description,
            sprite,
            glow,
            lights,
            blink: ship_blink(self, id.0),
        }
    }
}

fn layer_info(layer: Result<LayerSprite<'_>, LayerError>) -> Result<SheetInfo, String> {
    layer
        .map_err(|err| err.to_string())
        .map(|layer| sheet_info(layer.image_id, &layer.sheet))
}

/// The ship's resource name; if that is empty, its shipyard name; if that
/// is empty too, its ID.
fn name(ship: &StoreEntry<'_, Ship>) -> String {
    [
        ship.name.unwrap_or_default(),
        ship.record.short_name.as_str(),
    ]
    .into_iter()
    .find(|name| !name.is_empty())
    .map_or_else(|| fallback_name(ShipId(ship.id)), str::to_owned)
}

fn fallback_name(id: ShipId) -> String {
    format!("shïp {}", id.0)
}

fn stats(ship: &Ship) -> ShipStats {
    ShipStats {
        cost: ship.cost,
        speed: ship.speed,
        armor: ship.armor,
        shield: ship.shield,
    }
}

/// The sheet's ID and frame count.
fn sheet_info(image_id: i16, sheet: &SpriteSheet) -> SheetInfo {
    // An `rlëD` header counts its frames in a u16, and the decoder rejects
    // a sheet with none (`GraphicsError::NoFrames`).
    let frames = NonZeroU16::new(sheet.frames().len() as u16)
        .expect("a decoded rlëD has at least one frame");
    SheetInfo { image_id, frames }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::num::NonZeroU16;
    use std::path::Path;

    use nova_data::Record;
    use nova_data::graphics::RLED;
    use nova_data::graphics::fixture::RledBuilder;
    use nova_data::records::desc::Desc;
    use nova_data::records::ship::Ship;
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};
    use nova_sim::Blink;

    use super::*;
    use crate::ships::catalog::{SheetInfo, ShipStats};

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

    /// A resource: type, ID, name and bytes.
    type Res = (ResType, i16, Option<&'static str>, Vec<u8>);

    fn store(resources: &[Res]) -> GameData {
        let fork = resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, name, data)| {
                fork.resource(*ty, *id, name.map(str::as_bytes), data)
            })
            .build()
            .bytes;
        let file = OneFile(fork);
        GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
    }

    fn put(bytes: &mut [u8], at: usize, value: &[u8]) {
        bytes[at..at + value.len()].copy_from_slice(value);
    }

    /// A `shïp` with these stats and shipyard name.
    fn ship(stats: ShipStats, short_name: &str) -> Vec<u8> {
        let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
        put(&mut bytes, 0x02, &stats.shield.to_be_bytes());
        put(&mut bytes, 0x06, &stats.speed.to_be_bytes());
        put(&mut bytes, 0x0E, &stats.armor.to_be_bytes());
        put(&mut bytes, 0x30, &stats.cost.to_be_bytes());
        put(&mut bytes, 0x5CE, short_name.as_bytes());
        bytes
    }

    /// A `shän` with base image `base` (`frames_per` frames, one set) and
    /// glow and lights images `glow` and `lights`.
    fn anim(base: i16, frames_per: i16, glow: i16, lights: i16) -> Vec<u8> {
        let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
        put(&mut bytes, 0x00, &base.to_be_bytes());
        put(&mut bytes, 0x04, &1_i16.to_be_bytes());
        put(&mut bytes, 0x16, &glow.to_be_bytes());
        put(&mut bytes, 0x1E, &lights.to_be_bytes());
        put(&mut bytes, 0x34, &frames_per.to_be_bytes());
        bytes
    }

    /// An `rlëD` of `frames` 2x2 frames.
    fn sheet(frames: u16) -> Vec<u8> {
        (0..frames)
            .fold(RledBuilder::new(2, 2), |b, _| {
                b.frame(|f| f.line().pixels(&[0x7C00, 0x7C00]))
            })
            .build()
    }

    /// A `dësc` holding `text`.
    fn desc(text: &[u8]) -> Vec<u8> {
        let mut bytes = text.to_vec();
        bytes.extend([0, 0xFF, 0xFF]);
        bytes.extend([0; 34]);
        bytes
    }

    const STATS: ShipStats = ShipStats {
        cost: 15_000,
        speed: 300,
        armor: 25,
        shield: -40,
    };

    fn info(image_id: i16, frames: u16) -> SheetInfo {
        SheetInfo {
            image_id,
            frames: NonZeroU16::new(frames).expect("non-zero"),
        }
    }

    #[test]
    fn a_full_ship_has_its_name_stats_description_sprite_and_layers() {
        let data = store(&[
            (
                Ship::TYPE,
                128,
                Some("Shuttle"),
                ship(STATS, "Shuttle Mk I"),
            ),
            (ShipAnim::TYPE, 128, None, anim(1000, 4, 1100, 1200)),
            (RLED, 1000, None, sheet(4)),
            (RLED, 1100, None, sheet(1)),
            (RLED, 1200, None, sheet(4)),
            (Desc::TYPE, 13000, Some("Shuttle"), desc(b"Small.")),
        ]);
        assert_eq!(
            data.ship(ShipId(128)),
            ShipEntry {
                id: ShipId(128),
                name: "Shuttle".to_owned(),
                stats: Ok(STATS),
                description: Ok(Some("Small.".to_owned())),
                sprite: Ok(info(1000, 4)),
                glow: Some(Ok(info(1100, 1))),
                lights: Some(Ok(info(1200, 4))),
                blink: Blink::STEADY,
            }
        );
    }

    #[test]
    fn a_ship_carries_its_shans_blink() {
        // The Shuttle's: mode 1, A=4 B=1 C=2 D=20.
        let mut shan = anim(1000, 4, 0, 1200);
        for (at, value) in [(0x36, 1_i16), (0x38, 4), (0x3A, 1), (0x3C, 2), (0x3E, 20)] {
            put(&mut shan, at, &value.to_be_bytes());
        }
        let data = store(&[
            (Ship::TYPE, 128, Some("Shuttle"), ship(STATS, "")),
            (ShipAnim::TYPE, 128, None, shan),
            (RLED, 1000, None, sheet(4)),
            (RLED, 1200, None, sheet(4)),
        ]);
        assert_eq!(
            data.ship(ShipId(128)).blink,
            Blink {
                mode: 1,
                a: 4,
                b: 1,
                c: 2,
                d: 20,
            }
        );
    }

    #[test]
    fn a_ship_without_a_shan_has_a_sprite_error_and_no_layers() {
        let data = store(&[(Ship::TYPE, 129, Some("Drone"), ship(STATS, ""))]);
        let entry = data.ship(ShipId(129));
        assert_eq!(entry.sprite, Err("no shän 129 for shïp 129".to_owned()));
        assert_eq!((entry.glow, entry.lights), (None, None));
        assert_eq!(entry.stats, Ok(STATS));
        assert_eq!(entry.description, Ok(None));
    }

    #[test]
    fn a_missing_layer_is_that_layers_error() {
        let data = store(&[
            (Ship::TYPE, 130, Some("Lit"), ship(STATS, "")),
            (ShipAnim::TYPE, 130, None, anim(1000, 4, 1101, -1)),
            (RLED, 1000, None, sheet(4)),
        ]);
        let entry = data.ship(ShipId(130));
        assert_eq!(entry.sprite, Ok(info(1000, 4)));
        assert_eq!(
            entry.glow,
            Some(Err("shïp 130: no rlëD 1101 for its glow image".to_owned()))
        );
        assert_eq!(entry.lights, None);
    }

    #[test]
    fn a_sheet_with_no_frames_is_the_decoders_error() {
        let data = store(&[
            (Ship::TYPE, 130, Some("Empty"), ship(STATS, "")),
            (ShipAnim::TYPE, 130, None, anim(1000, 4, 1100, -1)),
            (RLED, 1000, None, sheet(0)),
            (RLED, 1100, None, sheet(0)),
        ]);
        let entry = data.ship(ShipId(130));
        let none = "the rlëD sheet has no frames";
        assert_eq!(entry.sprite, Err(format!("rlëD 1000: {none}")));
        assert_eq!(entry.glow, Some(Err(format!("glow rlëD 1100: {none}"))));
    }

    #[test]
    fn a_description_has_its_line_breaks_as_newlines() {
        let data = store(&[
            (Ship::TYPE, 128, Some("Shuttle"), ship(STATS, "")),
            (
                Desc::TYPE,
                13000,
                None,
                desc(b"One.\rTwo {b1 \"x\" \"y\"}."),
            ),
        ]);
        let entry = data.ship(ShipId(128));
        assert_eq!(
            entry.description,
            Ok(Some("One.\nTwo {b1 \"x\" \"y\"}.".to_owned()))
        );
    }

    #[test]
    fn an_undecodable_description_is_an_error() {
        let data = store(&[
            (Ship::TYPE, 128, Some("Shuttle"), ship(STATS, "")),
            (Desc::TYPE, 13000, None, b"unterminated".to_vec()),
        ]);
        let entry = data.ship(ShipId(128));
        let Err(message) = entry.description else {
            panic!("an error: {:?}", entry.description)
        };
        assert!(message.contains("13000"), "{message}");
    }

    #[test]
    fn an_unnamed_ship_falls_back_to_its_shipyard_name_then_its_id() {
        let data = store(&[
            (Ship::TYPE, 128, None, ship(STATS, "Heavy Shuttle")),
            (Ship::TYPE, 129, Some(""), ship(STATS, "")),
        ]);
        assert_eq!(data.ship(ShipId(128)).name, "Heavy Shuttle");
        assert_eq!(data.ship(ShipId(129)).name, "shïp 129");
    }

    #[test]
    fn an_undecodable_ship_is_named_by_id_with_a_stats_error() {
        let mut short = ship(STATS, "Broken");
        short.pop();
        let data = store(&[(Ship::TYPE, 131, Some("Broken"), short)]);
        let entry = data.ship(ShipId(131));
        assert_eq!(entry.name, "shïp 131");
        let Err(message) = &entry.stats else {
            panic!("an error: {:?}", entry.stats)
        };
        assert!(message.contains("131"), "{message}");
        assert_eq!(entry.sprite, Err(message.clone()));
    }

    #[test]
    fn a_missing_ship_is_named_by_id_with_errors() {
        let data = store(&[]);
        let entry = data.ship(ShipId(140));
        assert_eq!(entry.name, "shïp 140");
        assert_eq!(entry.stats, Err("no shïp 140".to_owned()));
        assert_eq!(entry.sprite, Err("no shïp 140".to_owned()));
    }

    #[test]
    fn ship_ids_are_every_shïp_ascending() {
        let data = store(&[
            (Ship::TYPE, 200, None, ship(STATS, "")),
            (Ship::TYPE, 128, None, ship(STATS, "")),
            (ShipAnim::TYPE, 150, None, anim(1000, 4, 0, 0)),
        ]);
        assert_eq!(data.ship_ids(), [ShipId(128), ShipId(200)]);
        assert_eq!(store(&[]).ship_ids(), []);
    }
}
