//! The resource catalog port: every resource in the game data, and one
//! resource's bytes, files and record, in the developer tools' own terms.

use std::path::PathBuf;
use std::rc::Rc;

pub use nova_data::Origin;
pub use nova_rsrc::ResType;

/// One resource in the index: its type, ID and name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceSummary {
    /// Its type code.
    pub ty: ResType,
    /// Its ID.
    pub id: i16,
    /// Its name, if it has one.
    pub name: Option<String>,
}

impl ResourceSummary {
    /// The row text: the type code (trailing spaces trimmed), the ID and the
    /// name when there is one.
    #[must_use]
    pub fn label(&self) -> String {
        let ty = type_code(self.ty);
        match &self.name {
            Some(name) => format!("{ty} {} {name}", self.id),
            None => format!("{ty} {}", self.id),
        }
    }
}

/// A type code as text, in Mac Roman, without its trailing spaces: `snd `
/// is `snd`.
#[must_use]
pub fn type_code(ty: ResType) -> String {
    ty.to_string().trim_end_matches(' ').to_owned()
}

/// A file that defines a resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceInfo {
    /// The file's path.
    pub path: PathBuf,
    /// Whether it is in the data folder or the plug-ins.
    pub origin: Origin,
}

/// A resource's decoded record, as the developer tools show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordView {
    /// The type has no record layout (a picture, sprite sheet, sound or
    /// unknown type).
    NotARecord,
    /// The record as pretty JSON, with the decoder's warning if it raised
    /// one.
    Json {
        /// The pretty JSON.
        text: String,
        /// The warning's text.
        warning: Option<String>,
    },
    /// The record did not decode: the error's text.
    Error(String),
}

/// One resource inspected: its bytes, the file it came from, the files it
/// overrides, and its record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceDetail {
    /// Its bytes, from the winning file.
    pub data: Vec<u8>,
    /// The winning file.
    pub source: SourceInfo,
    /// Earlier files that also define it, in load order.
    pub overridden: Vec<SourceInfo>,
    /// Its record.
    pub record: RecordView,
}

/// The resources the developer tools can browse.
pub trait ResourceCatalog {
    /// Every resource: types in type-code byte order, IDs ascending.
    fn resources(&self) -> Vec<ResourceSummary>;
    /// One resource's bytes, files and record, or `None` if it does not
    /// exist.
    fn inspect(&self, ty: ResType, id: i16) -> Option<ResourceDetail>;
}

/// A borrowed catalog is a catalog.
impl<T: ResourceCatalog + ?Sized> ResourceCatalog for &T {
    fn resources(&self) -> Vec<ResourceSummary> {
        (**self).resources()
    }

    fn inspect(&self, ty: ResType, id: i16) -> Option<ResourceDetail> {
        (**self).inspect(ty, id)
    }
}

/// A shared catalog is a catalog, so the browser and the game can read the
/// same game data.
impl<T: ResourceCatalog + ?Sized> ResourceCatalog for Rc<T> {
    fn resources(&self) -> Vec<ResourceSummary> {
        (**self).resources()
    }

    fn inspect(&self, ty: ResType, id: i16) -> Option<ResourceDetail> {
        (**self).inspect(ty, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SND: ResType = ResType::new(*b"snd ");
    const RLED: ResType = ResType::new([b'r', b'l', 0x91, b'D']);

    fn summary(ty: ResType, id: i16, name: Option<&str>) -> ResourceSummary {
        ResourceSummary {
            ty,
            id,
            name: name.map(str::to_owned),
        }
    }

    /// One resource, (`snd `, 128), whose bytes are its ID.
    struct One;

    impl ResourceCatalog for One {
        fn resources(&self) -> Vec<ResourceSummary> {
            vec![summary(SND, 128, None)]
        }

        fn inspect(&self, ty: ResType, id: i16) -> Option<ResourceDetail> {
            (ty == SND && id == 128).then(|| ResourceDetail {
                data: id.to_be_bytes().to_vec(),
                source: SourceInfo {
                    path: PathBuf::from("/data/Nova Sounds"),
                    origin: Origin::Data,
                },
                overridden: Vec::new(),
                record: RecordView::NotARecord,
            })
        }
    }

    #[test]
    fn the_label_is_the_type_id_and_name() {
        assert_eq!(
            summary(RLED, 128, Some("Shuttle")).label(),
            "rlëD 128 Shuttle"
        );
        assert_eq!(summary(RLED, -1, None).label(), "rlëD -1");
    }

    #[test]
    fn the_label_trims_the_types_trailing_spaces() {
        assert_eq!(summary(SND, 200, Some("Beep")).label(), "snd 200 Beep");
        assert_eq!(type_code(SND), "snd");
        assert_eq!(type_code(ResType::new(*b"S  #")), "S  #");
    }

    fn read(catalog: impl ResourceCatalog) -> (usize, Option<Vec<u8>>, bool) {
        (
            catalog.resources().len(),
            catalog.inspect(SND, 128).map(|detail| detail.data),
            catalog.inspect(SND, 129).is_none(),
        )
    }

    #[test]
    fn borrowed_and_shared_catalogs_are_catalogs() {
        let expected = (1, Some(vec![0, 128]), true);
        assert_eq!(read(&One), expected);
        assert_eq!(read(Rc::new(One)), expected);
    }
}
