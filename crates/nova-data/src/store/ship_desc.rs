//! Finding a ship's description.

use super::{GameData, StoreEntry};
use crate::error::DecodeError;
use crate::records::desc::Desc;
use crate::wire::id::ShipId;

/// The first ship ID and its description's `dësc` ID.
const FIRST_SHIP: i16 = 128;
const FIRST_SHIP_DESC: i16 = 13000;
/// The last ship ID the Bible allows.
const LAST_SHIP: i16 = 895;

/// The `dësc` ID describing `ship`: the Bible gives ship descriptions as
/// `dësc` 13000 to 13767 for `shïp` 128 to 895, so `13000 + (ID - 128)`.
/// `None` for an ID outside that range.
#[must_use]
pub const fn ship_desc_id(ship: ShipId) -> Option<i16> {
    match ship.0 {
        id @ FIRST_SHIP..=LAST_SHIP => Some(FIRST_SHIP_DESC + (id - FIRST_SHIP)),
        _ => None,
    }
}

impl GameData {
    /// The `dësc` describing `ship`, by the Bible's convention ([`ship_desc_id`]),
    /// in the same shape as [`GameData::get`]: `None` when the ship has no
    /// description (many variants have none), `Some(Err)` when it fails to
    /// decode.
    #[must_use]
    pub fn ship_description(
        &self,
        ship: ShipId,
    ) -> Option<Result<StoreEntry<'_, Desc>, &DecodeError>> {
        self.get::<Desc>(ship_desc_id(ship)?)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_rsrc::fixture::ForkBuilder;

    use super::*;
    use crate::decode::Record;
    use crate::store::fake::{FakeForks, FakeTree};
    use crate::store::fs::EntryKind::File;

    #[test]
    fn ship_ids_map_onto_dësc_13000_onwards() {
        assert_eq!(ship_desc_id(ShipId(128)), Some(13000));
        assert_eq!(ship_desc_id(ShipId(129)), Some(13001));
        assert_eq!(ship_desc_id(ShipId(895)), Some(13767));
    }

    #[test]
    fn ids_outside_the_ship_range_have_no_description_id() {
        for id in [127, 896, 0, -1, i16::MIN, i16::MAX] {
            assert_eq!(ship_desc_id(ShipId(id)), None, "{id}");
        }
    }

    /// A `dësc` holding `text`, with no graphic or movie.
    fn desc(text: &[u8]) -> Vec<u8> {
        let mut bytes = text.to_vec();
        bytes.extend([0, 0xFF, 0xFF]);
        bytes.extend([0; 34]);
        bytes
    }

    /// One data file holding `descs` (ID, bytes).
    fn store(descs: &[(i16, Vec<u8>)]) -> GameData {
        let fork = descs
            .iter()
            .fold(ForkBuilder::new(), |b, (id, data)| {
                b.resource(Desc::TYPE, *id, Some(b"Shuttle"), data)
            })
            .build()
            .bytes;
        let tree = FakeTree::new().dir("/d", &[("descs", File)]);
        let forks = FakeForks::new().file("/d/descs", fork);
        GameData::load(&tree, &forks, Path::new("/d"), None).expect("opens")
    }

    #[test]
    fn a_ships_description_is_its_dësc() {
        let data = store(&[(13000, desc(b"A shuttle.")), (13001, desc(b"Heavy."))]);
        let entry = data
            .ship_description(ShipId(128))
            .expect("present")
            .expect("decodes");
        assert_eq!(entry.id, 13000);
        assert_eq!(entry.name, Some("Shuttle"));
        assert_eq!(entry.record.text.as_str(), "A shuttle.");
        let entry = data
            .ship_description(ShipId(129))
            .expect("present")
            .expect("decodes");
        assert_eq!(entry.record.text.as_str(), "Heavy.");
    }

    #[test]
    fn a_ship_without_a_dësc_has_no_description() {
        let data = store(&[(13000, desc(b"A shuttle."))]);
        assert!(data.ship_description(ShipId(130)).is_none());
        assert!(data.ship_description(ShipId(127)).is_none());
    }

    #[test]
    fn an_undecodable_dësc_is_an_error() {
        let data = store(&[(13000, b"unterminated".to_vec())]);
        let err = data
            .ship_description(ShipId(128))
            .expect("present")
            .expect_err("fails to decode");
        assert_eq!((err.res_type, err.id), (Desc::TYPE, 13000));
    }
}
