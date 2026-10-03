//! The resource catalog over the game data: a thin mapping from
//! `GameData`'s index, provenance and decoded records to the port's owned
//! values.

use nova_data::{GameData, SourceFile};
use nova_rsrc::ResType;

use super::catalog::{RecordView, ResourceCatalog, ResourceDetail, ResourceSummary, SourceInfo};

/// Reads the store afresh on every call; the browser lists once and
/// inspects one resource per selection.
impl ResourceCatalog for GameData {
    fn resources(&self) -> Vec<ResourceSummary> {
        self.types()
            .flat_map(|ty| {
                self.ids(ty).iter().map(move |&id| ResourceSummary {
                    ty,
                    id,
                    name: self
                        .resource(ty, id)
                        .and_then(|found| found.resource.name())
                        .map(str::to_owned),
                })
            })
            .collect()
    }

    fn inspect(&self, ty: ResType, id: i16) -> Option<ResourceDetail> {
        let found = self.resource(ty, id)?;
        let provenance = self.provenance(ty, id)?;
        let record = match self.get_any(ty, id) {
            None => RecordView::NotARecord,
            Some(Ok(entry)) => RecordView::Json {
                text: format!("{:#}", entry.record.to_json()),
                warning: entry.warning.map(ToString::to_string),
            },
            Some(Err(error)) => RecordView::Error(error.to_string()),
        };
        Some(ResourceDetail {
            data: found.resource.data().to_vec(),
            source: source_info(provenance.winner),
            overridden: provenance.shadowed.into_iter().map(source_info).collect(),
            record,
        })
    }
}

fn source_info(file: &SourceFile) -> SourceInfo {
    SourceInfo {
        path: file.path.clone(),
        origin: file.origin,
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::{Path, PathBuf};

    use nova_data::records::spin::Spin;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_data::{GameData, Record};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};

    use crate::devtools::catalog::{
        Origin, RecordView, ResourceCatalog, ResourceSummary, SourceInfo,
    };

    const PICT: ResType = ResType::new(*b"PICT");
    const SND: ResType = ResType::new(*b"snd ");

    /// A resource: type, ID, name (Mac Roman) and bytes.
    type Res = (ResType, i16, Option<&'static [u8]>, Vec<u8>);

    /// Two files: `/data/Nova Data` and the plug-in `/plug/Over`, which
    /// loads after it.
    struct TwoFiles {
        data: Vec<u8>,
        plug: Vec<u8>,
    }

    impl DirLister for TwoFiles {
        fn list(&self, dir: &Path) -> io::Result<Vec<Listing>> {
            let name = if dir == Path::new("/data") {
                "Nova Data"
            } else {
                "Over"
            };
            Ok(vec![Listing {
                name: name.into(),
                kind: EntryKind::File,
            }])
        }
    }

    impl ForkReader for TwoFiles {
        fn read_fork(&self, path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            let bytes = if path.starts_with("/data") {
                &self.data
            } else {
                &self.plug
            };
            Ok((fork == Fork::Data).then(|| bytes.clone()))
        }
    }

    fn fork(resources: &[Res]) -> Vec<u8> {
        resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, name, data)| {
                fork.resource(*ty, *id, *name, data)
            })
            .build()
            .bytes
    }

    fn store(data: &[Res], plug: &[Res]) -> GameData {
        let files = TwoFiles {
            data: fork(data),
            plug: fork(plug),
        };
        GameData::load(&files, &files, Path::new("/data"), Some(Path::new("/plug"))).expect("opens")
    }

    /// A 12-byte `spïn`: sprites `sprites`, masks 1001, 48 × 48, 6 × 6.
    fn spin(sprites: i16) -> Vec<u8> {
        let mut bytes = vec![0, 0, 0x03, 0xE9, 0, 48, 0, 48, 0, 6, 0, 6];
        bytes[..2].copy_from_slice(&sprites.to_be_bytes());
        bytes
    }

    fn data_file() -> SourceInfo {
        SourceInfo {
            path: PathBuf::from("/data/Nova Data"),
            origin: Origin::Data,
        }
    }

    fn summary(ty: ResType, id: i16, name: Option<&str>) -> ResourceSummary {
        ResourceSummary {
            ty,
            id,
            name: name.map(str::to_owned),
        }
    }

    #[test]
    fn resources_list_every_type_in_byte_order_and_ids_ascending() {
        let data = store(
            &[
                (SND, 5, Some(b"Beep"), vec![1]),
                (Spin::TYPE, 130, None, spin(1000)),
                (PICT, 200, Some(b"Big"), vec![2]),
                (PICT, -3, None, vec![3]),
            ],
            // "Über" in Mac Roman.
            &[(Spin::TYPE, 128, Some(b"\x86ber"), spin(1001))],
        );
        assert_eq!(
            data.resources(),
            [
                summary(PICT, -3, None),
                summary(PICT, 200, Some("Big")),
                summary(SND, 5, Some("Beep")),
                summary(Spin::TYPE, 128, Some("Über")),
                summary(Spin::TYPE, 130, None),
            ]
        );
    }

    #[test]
    fn a_record_is_its_pretty_json() {
        let data = store(&[(Spin::TYPE, 128, None, spin(1000))], &[]);
        let detail = data.inspect(Spin::TYPE, 128).expect("present");
        let RecordView::Json { text, warning } = &detail.record else {
            panic!("not JSON: {:?}", detail.record);
        };
        let record = data
            .get_any(Spin::TYPE, 128)
            .expect("present")
            .expect("decodes");
        let parsed: serde_json::Value = serde_json::from_str(text).expect("parses");
        assert_eq!(parsed, record.record.to_json());
        assert_eq!(*text, format!("{:#}", record.record.to_json()));
        assert!(text.contains("\n  \"sprites_id\": 1000"), "{text}");
        assert_eq!(*warning, None);
        assert_eq!(detail.data, spin(1000));
        assert_eq!(detail.source, data_file());
        assert_eq!(detail.overridden, []);
    }

    #[test]
    fn a_record_that_does_not_decode_is_its_error() {
        let data = store(&[(Spin::TYPE, 128, Some(b"Short"), vec![0; 5])], &[]);
        let detail = data.inspect(Spin::TYPE, 128).expect("present");
        let error = data
            .get_any(Spin::TYPE, 128)
            .expect("present")
            .expect_err("too short");
        assert_eq!(detail.record, RecordView::Error(error.to_string()));
        assert_eq!(detail.data, [0; 5]);
    }

    #[test]
    fn a_long_record_carries_its_warning() {
        let mut long = spin(1000);
        long.extend([0, 0]);
        let data = store(&[(Spin::TYPE, 128, None, long)], &[]);
        let entry = data
            .get_any(Spin::TYPE, 128)
            .expect("present")
            .expect("decodes");
        let expected = entry.warning.expect("a warning").to_string();
        let RecordView::Json { warning, .. } =
            data.inspect(Spin::TYPE, 128).expect("present").record
        else {
            panic!("not JSON");
        };
        assert_eq!(warning, Some(expected));
    }

    #[test]
    fn a_picture_is_not_a_record_and_keeps_its_bytes() {
        let data = store(&[(PICT, 128, None, vec![9, 8, 7])], &[]);
        let detail = data.inspect(PICT, 128).expect("present");
        assert_eq!(detail.record, RecordView::NotARecord);
        assert_eq!(detail.data, [9, 8, 7]);
        assert_eq!(detail.source, data_file());
    }

    #[test]
    fn a_plug_in_override_names_the_plug_in_and_the_file_it_overrides() {
        let data = store(&[(PICT, 128, None, vec![1])], &[(PICT, 128, None, vec![2])]);
        let detail = data.inspect(PICT, 128).expect("present");
        assert_eq!(detail.data, [2]);
        assert_eq!(
            detail.source,
            SourceInfo {
                path: PathBuf::from("/plug/Over"),
                origin: Origin::PlugIn,
            }
        );
        assert_eq!(detail.overridden, [data_file()]);
    }

    #[test]
    fn a_missing_resource_is_none() {
        let data = store(&[(PICT, 128, None, vec![1])], &[]);
        assert_eq!(data.inspect(PICT, 129), None);
        assert_eq!(data.inspect(SND, 128), None);
    }
}
