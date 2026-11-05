//! A store built from records decoded elsewhere: the new game's data,
//! which arrives as records, not as resource bytes.
//!
//! Every record is attributed to one [`SourceFile`] whose path is the
//! folder the records were read from, from the data directory
//! ([`Origin::Data`]), holding an empty [`ResourceFile`]. Each slot starts
//! with its decode filled in, so [`GameData::get`], [`GameData::get_any`]
//! and [`GameData::records`] read the records as they would decoded ones.
//!
//! Such a store lists IDs that have no raw bytes: [`GameData::resource`]
//! is `None` for every one. The only raw readers are media (`snd `,
//! `rlëD`, `PICT`) and `nova-dump`'s export, and the new game has none of
//! those yet.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use nova_rsrc::{ResType, ResourceFile};

use super::{GameData, Origin, Slot, SourceFile, TypeIndex};
use crate::decode::Entry;
use crate::registry::AnyRecord;

impl GameData {
    /// The store of `records`, decoded elsewhere and all read from `dir`,
    /// with no stand-ins ([`GameData::with_stand_ins`] adds them).
    ///
    /// Each record's type is its [`AnyRecord::res_type`]. A later record
    /// with the same type and ID replaces an earlier one. The store has one
    /// file (`dir`, [`Origin::Data`], with no resources), nothing failed or
    /// ignored, and no raw bytes: [`GameData::resource`] is `None` for
    /// every record (see the module docs).
    #[must_use]
    pub fn from_decoded(dir: PathBuf, records: Vec<Entry<AnyRecord>>) -> Self {
        let mut by_type: BTreeMap<ResType, BTreeMap<i16, Entry<AnyRecord>>> = BTreeMap::new();
        for entry in records {
            by_type
                .entry(entry.record.res_type())
                .or_default()
                .insert(entry.id, entry);
        }
        let index = by_type
            .into_iter()
            .map(|(ty, by_id)| {
                let (ids, slots) = by_id
                    .into_iter()
                    .map(|(id, entry)| (id, slot(entry)))
                    .unzip();
                (ty, TypeIndex { ids, slots })
            })
            .collect();
        Self {
            files: vec![SourceFile {
                path: dir,
                origin: Origin::Data,
                resources: ResourceFile::empty(),
            }],
            failed: Vec::new(),
            ignored: Vec::new(),
            index,
            tests: Mutex::default(),
            sets: Mutex::default(),
            stand_ins: BTreeMap::new(),
        }
    }
}

/// The slot of `entry`, defined by the store's one file and decoded
/// already, with no warning.
fn slot(entry: Entry<AnyRecord>) -> Slot {
    Slot {
        definers: vec![0],
        decoded: OnceLock::from(Some(Ok((entry, None)))),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::decode::{Record, decode_bytes};
    use crate::records::stellar::Stellar;
    use crate::records::system::System;
    use crate::store::Provenance;

    fn system(x_pos: i16) -> System {
        let mut system = decode_bytes::<System>(&[0; 428]).expect("decodes").record;
        system.x_pos = x_pos;
        system
    }

    fn stellar(x_pos: i16) -> Stellar {
        let mut stellar = decode_bytes::<Stellar>(&[0; 1118]).expect("decodes").record;
        stellar.x_pos = x_pos;
        stellar
    }

    fn entry(id: i16, name: &str, record: AnyRecord) -> Entry<AnyRecord> {
        Entry {
            id,
            name: Some(name.to_owned()),
            record,
        }
    }

    fn dir() -> PathBuf {
        Path::new("games").join("sample")
    }

    /// A system and a stellar in it.
    fn sample() -> GameData {
        GameData::from_decoded(
            dir(),
            vec![
                entry(128, "Alpha", AnyRecord::System(Box::new(system(5)))),
                entry(129, "Alpha Yard", AnyRecord::Stellar(Box::new(stellar(7)))),
            ],
        )
    }

    #[test]
    fn lists_the_ids_of_each_type() {
        let data = sample();
        assert_eq!(data.ids(System::TYPE), [128]);
        assert_eq!(data.ids(Stellar::TYPE), [129]);
        assert_eq!(
            data.types().collect::<Vec<_>>(),
            [Stellar::TYPE, System::TYPE]
        );
    }

    #[test]
    fn gets_a_record_with_its_name_from_the_folder() {
        let data = sample();
        let alpha = data.get::<System>(128).expect("present").expect("decoded");
        assert_eq!(alpha.id, 128);
        assert_eq!(alpha.name, Some("Alpha"));
        assert_eq!(alpha.record.x_pos, 5);
        assert_eq!(alpha.warning, None);
        assert_eq!(alpha.source.path, dir());
        assert_eq!(alpha.source.origin, Origin::Data);
        assert!(data.get::<System>(129).is_none(), "another type's id");
        assert!(data.get::<Stellar>(128).is_none());
    }

    #[test]
    fn iterates_the_records_of_a_type() {
        let data = sample();
        let stellars: Vec<_> = data
            .records::<Stellar>()
            .map(|(id, entry)| (id, entry.expect("decoded").record.x_pos))
            .collect();
        assert_eq!(stellars, [(129, 7)]);
    }

    #[test]
    fn names_the_one_file_as_provenance() {
        let data = sample();
        let Provenance { winner, shadowed } = data.provenance(Stellar::TYPE, 129).expect("present");
        assert_eq!(winner.path, dir());
        assert!(shadowed.is_empty());
        let [file] = data.files() else {
            panic!("one file: {:?}", data.files());
        };
        assert_eq!(file.path, dir());
        assert_eq!(file.origin, Origin::Data);
        assert!(file.resources.is_empty());
    }

    #[test]
    fn nothing_failed_or_was_ignored() {
        let data = sample();
        assert!(data.failed().is_empty());
        assert!(data.ignored().is_empty());
    }

    #[test]
    fn a_record_has_no_raw_bytes() {
        let data = sample();
        assert!(data.resource(System::TYPE, 128).is_none());
        assert!(data.resource(Stellar::TYPE, 129).is_none());
    }

    #[test]
    fn a_later_record_with_the_same_id_replaces_an_earlier_one() {
        let data = GameData::from_decoded(
            dir(),
            vec![
                entry(128, "First", AnyRecord::System(Box::new(system(1)))),
                entry(128, "Second", AnyRecord::System(Box::new(system(2)))),
            ],
        );
        assert_eq!(data.ids(System::TYPE), [128]);
        let entry = data.get::<System>(128).expect("present").expect("decoded");
        assert_eq!((entry.name, entry.record.x_pos), (Some("Second"), 2));
    }

    #[test]
    fn ids_are_ascending_whatever_the_order_given() {
        let data = GameData::from_decoded(
            dir(),
            vec![
                entry(130, "C", AnyRecord::System(Box::new(system(3)))),
                entry(128, "A", AnyRecord::System(Box::new(system(1)))),
                entry(129, "B", AnyRecord::System(Box::new(system(2)))),
            ],
        );
        assert_eq!(data.ids(System::TYPE), [128, 129, 130]);
        let names: Vec<_> = data
            .records::<System>()
            .map(|(_, entry)| entry.expect("decoded").name)
            .collect();
        assert_eq!(names, [Some("A"), Some("B"), Some("C")]);
    }

    #[test]
    fn no_records_is_the_empty_store_with_its_one_file() {
        let data = GameData::from_decoded(dir(), Vec::new());
        let empty = GameData::default();
        assert_eq!(data.types().count(), empty.types().count());
        assert_eq!(data.ids(System::TYPE), empty.ids(System::TYPE));
        assert!(data.get::<System>(128).is_none());
        assert!(data.failed().is_empty() && data.ignored().is_empty());
        assert_eq!(data.files().len(), 1, "the folder");
        assert!(empty.files().is_empty());
    }
}
