//! The ship sprite catalog over the game data: a thin mapping from
//! `GameData`'s ship sprite lookup to [`ShipSheet`].

use std::num::NonZeroU16;

use nova_data::GameData;

use super::catalog::{ShipId, ShipSheet, ShipSprites};

/// Decodes the sheet on every call; the flight screen asks once, when it
/// opens.
impl ShipSprites for GameData {
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
        let sprite = self.ship_sprite(id).map_err(|err| err.to_string())?;
        let sheet = &sprite.sheet;
        // The sheet's layout is the `shän`'s frames per rotation as columns.
        let rotations = NonZeroU16::new(sheet.layout().columns())
            .expect("a sheet layout has at least one column");
        Ok(ShipSheet {
            image_id: sprite.image_id,
            rotations,
            frame_width: sheet.frame_width(),
            frame_height: sheet.frame_height(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;

    use nova_data::Record;
    use nova_data::graphics::RLED;
    use nova_data::graphics::fixture::RledBuilder;
    use nova_data::records::ship::Ship;
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};

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
    /// frames.
    fn anim(base: i16, sets: i16, frames_per: i16) -> Vec<u8> {
        let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
        put_i16(&mut bytes, 0x00, base);
        put_i16(&mut bytes, 0x04, sets);
        put_i16(&mut bytes, 0x34, frames_per);
        bytes
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
            })
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
}
