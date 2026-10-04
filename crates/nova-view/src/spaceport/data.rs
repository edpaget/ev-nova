//! The spaceport's ports over the game data: a thin mapping from
//! `GameData`'s `spöb` records, `PICT`s and `shän` base images.

use nova_data::GameData;
use nova_data::graphics::PICT;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::stellar::Stellar;
use nova_sim::ShipId;

use super::catalog::{PortRecord, SpaceportCatalog, StellarId};
use super::shipyard::ShipBaseImages;

/// Reads afresh on every call; the spaceport asks once, when it opens.
impl SpaceportCatalog for GameData {
    fn stellar_port(&self, id: StellarId) -> Result<PortRecord, String> {
        let entry = match self.get::<Stellar>(id.0) {
            None => return Err(format!("no spöb {}", id.0)),
            Some(Err(err)) => return Err(err.to_string()),
            Some(Ok(entry)) => entry,
        };
        let stellar = entry.record;
        Ok(PortRecord {
            name: entry.name.unwrap_or_default().to_owned(),
            flags: stellar.flags.bits(),
            cust_pic_id: stellar.cust_pic_id,
            graphic_type: stellar.graphic_type,
        })
    }

    fn picture_exists(&self, id: i16) -> bool {
        self.resource(PICT, id).is_some()
    }
}

/// Reads every `shän` afresh; the shipyard asks once, when it opens.
impl ShipBaseImages for GameData {
    fn ship_base_images(&self) -> Vec<(ShipId, i16)> {
        self.records::<ShipAnim>()
            .filter_map(|(id, anim)| Some((ShipId(id), anim.ok()?.record.base_image_id)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;

    use nova_data::Record;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader};

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

    /// A `spöb` of graphic type `graphic_type`, with these `Flags` and
    /// `CustPicID`.
    fn stellar(graphic_type: i16, flags: u32, cust_pic_id: i16) -> Vec<u8> {
        let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
        bytes[0x04..0x06].copy_from_slice(&graphic_type.to_be_bytes());
        bytes[0x06..0x0A].copy_from_slice(&flags.to_be_bytes());
        bytes[0x18..0x1A].copy_from_slice(&cust_pic_id.to_be_bytes());
        bytes
    }

    /// Earth (128), a nameless stellar (129), a short one (130), and
    /// `PICT` 10003, which need not decode.
    fn data() -> GameData {
        let mut short = stellar(0, 0, 0);
        short.pop();
        let fork = ForkBuilder::new()
            .resource(Stellar::TYPE, 128, Some(b"Earth"), &stellar(3, 0x43, 9000))
            .resource(Stellar::TYPE, 129, None, &stellar(255, 0x21, -1))
            .resource(Stellar::TYPE, 130, Some(b"Short"), &short)
            .resource(PICT, 10_003, None, &[0; 4])
            .build()
            .bytes;
        let file = OneFile(fork);
        GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
    }

    #[test]
    fn a_stellars_port_is_its_name_flags_and_pictures() {
        let data = data();
        assert_eq!(
            data.stellar_port(StellarId(128)),
            Ok(PortRecord {
                name: "Earth".to_owned(),
                flags: 0x43,
                cust_pic_id: 9000,
                graphic_type: 3,
            })
        );
        assert_eq!(
            data.stellar_port(StellarId(129)),
            Ok(PortRecord {
                name: String::new(),
                flags: 0x21,
                cust_pic_id: -1,
                graphic_type: 255,
            })
        );
    }

    #[test]
    fn a_missing_or_undecodable_stellar_says_why() {
        let data = data();
        assert_eq!(
            data.stellar_port(StellarId(140)),
            Err("no spöb 140".to_owned())
        );
        let Err(reason) = data.stellar_port(StellarId(130)) else {
            panic!("an error")
        };
        assert!(reason.contains("130"), "{reason}");
    }

    #[test]
    fn a_picture_exists_when_the_data_has_it() {
        let data = data();
        assert!(data.picture_exists(10_003));
        assert!(!data.picture_exists(10_000));
    }

    /// A `shän` whose base image is `rlëD` `base`.
    fn ship_anim(base: i16) -> Vec<u8> {
        let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
        bytes[0x00..0x02].copy_from_slice(&base.to_be_bytes());
        bytes
    }

    #[test]
    fn the_ships_base_images_are_each_readable_shäns_by_id() {
        let mut short = ship_anim(1002);
        short.pop();
        let fork = ForkBuilder::new()
            .resource(ShipAnim::TYPE, 361, None, &ship_anim(1000))
            .resource(ShipAnim::TYPE, 128, None, &ship_anim(1000))
            .resource(ShipAnim::TYPE, 129, Some(b"Heavy"), &ship_anim(1001))
            .resource(ShipAnim::TYPE, 130, None, &short)
            .build()
            .bytes;
        let file = OneFile(fork);
        let data = GameData::load(&file, &file, Path::new("/data"), None).expect("opens");
        assert_eq!(
            data.ship_base_images(),
            [
                (ShipId(128), 1000),
                (ShipId(129), 1001),
                (ShipId(361), 1000)
            ],
            "an undecodable one is left out"
        );
        assert_eq!(self::data().ship_base_images(), []);
    }
}
