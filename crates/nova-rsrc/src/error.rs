//! Errors from parsing and loading resource forks.

use std::path::PathBuf;
use std::{fmt, io};

use crate::ResType;

/// A section of a fork named by its header.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Section {
    /// The data section, holding length-prefixed resource data.
    Data,
    /// The resource map.
    Map,
}

/// Why a byte buffer is not a valid resource fork.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// The buffer is shorter than the 16-byte header.
    HeaderTruncated,
    /// The header places a section outside the file.
    SectionOutOfBounds {
        /// Which section.
        section: Section,
        /// The section's offset, from the header.
        offset: u32,
        /// The section's length, from the header.
        len: u32,
        /// The length of the whole fork.
        file_len: usize,
    },
    /// The map is shorter than its 28-byte header plus the type count.
    MapTruncated,
    /// The type list does not fit inside the map.
    TypeListOutOfBounds,
    /// The name list starts past the end of the map.
    NameListOutOfBounds,
    /// A type's reference list does not fit inside the map.
    ReferenceListOutOfBounds {
        /// The type whose list is out of bounds.
        ty: ResType,
    },
    /// A type appears twice in the type list.
    DuplicateType {
        /// The repeated type.
        ty: ResType,
    },
    /// An ID appears twice within one type.
    DuplicateId {
        /// The resource's type.
        ty: ResType,
        /// The repeated ID.
        id: i16,
    },
    /// A resource's data entry (its length prefix) starts outside the data
    /// section.
    DataOffsetOutOfBounds {
        /// The resource's type.
        ty: ResType,
        /// The resource's ID.
        id: i16,
        /// The offset from the start of the data section.
        offset: u32,
    },
    /// A resource's data runs past the end of the data section.
    DataLengthOutOfBounds {
        /// The resource's type.
        ty: ResType,
        /// The resource's ID.
        id: i16,
        /// The declared data length.
        len: u32,
    },
    /// A resource's name starts outside the name list.
    NameOffsetOutOfBounds {
        /// The resource's type.
        ty: ResType,
        /// The resource's ID.
        id: i16,
        /// The offset from the start of the name list.
        offset: u16,
    },
    /// A resource's name runs past the end of the map.
    NameLengthOutOfBounds {
        /// The resource's type.
        ty: ResType,
        /// The resource's ID.
        id: i16,
    },
    /// A resource is compressed (`dcmp`), which is not supported.
    CompressedResource {
        /// The resource's type.
        ty: ResType,
        /// The resource's ID.
        id: i16,
    },
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Data => "data",
            Self::Map => "map",
        })
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderTruncated => {
                f.write_str("resource fork header is truncated (needs 16 bytes)")
            }
            Self::SectionOutOfBounds {
                section,
                offset,
                len,
                file_len,
            } => write!(
                f,
                "{section} section (offset {offset}, length {len}) extends past the end of the {file_len}-byte fork"
            ),
            Self::MapTruncated => f.write_str("resource map is shorter than its 30-byte header"),
            Self::TypeListOutOfBounds => {
                f.write_str("type list extends past the end of the resource map")
            }
            Self::NameListOutOfBounds => {
                f.write_str("name list starts past the end of the resource map")
            }
            Self::ReferenceListOutOfBounds { ty } => write!(
                f,
                "reference list for type '{ty}' extends past the end of the resource map"
            ),
            Self::DuplicateType { ty } => {
                write!(f, "type '{ty}' appears more than once in the type list")
            }
            Self::DuplicateId { ty, id } => {
                write!(f, "resource '{ty}' {id} appears more than once")
            }
            Self::DataOffsetOutOfBounds { ty, id, offset } => write!(
                f,
                "resource '{ty}' {id}: data offset {offset} is outside the data section"
            ),
            Self::DataLengthOutOfBounds { ty, id, len } => write!(
                f,
                "resource '{ty}' {id}: data length {len} runs past the end of the data section"
            ),
            Self::NameOffsetOutOfBounds { ty, id, offset } => write!(
                f,
                "resource '{ty}' {id}: name offset {offset} is outside the name list"
            ),
            Self::NameLengthOutOfBounds { ty, id } => write!(
                f,
                "resource '{ty}' {id}: name runs past the end of the resource map"
            ),
            Self::CompressedResource { ty, id } => write!(
                f,
                "resource '{ty}' {id} is compressed, which is not supported"
            ),
        }
    }
}

impl std::error::Error for ParseError {}

/// Why a resource file could not be loaded from disk.
#[derive(Debug)]
pub enum LoadError {
    /// Reading a fork failed.
    Io {
        /// The file being loaded.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// The fork's bytes are not a valid resource fork.
    Parse {
        /// The file being loaded.
        path: PathBuf,
        /// What is wrong with the fork.
        source: ParseError,
    },
    /// Neither the data fork nor the resource fork holds any bytes.
    NoResourceFork {
        /// The file being loaded.
        path: PathBuf,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "reading {}: {source}", path.display()),
            Self::Parse { path, source } => write!(f, "parsing {}: {source}", path.display()),
            Self::NoResourceFork { path } => write!(f, "{} has no resource fork", path.display()),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::NoResourceFork { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHIP: ResType = ResType([b's', b'h', 0x95, b'p']);

    #[test]
    fn parse_errors_describe_the_problem() {
        let cases = [
            (
                ParseError::HeaderTruncated,
                "resource fork header is truncated (needs 16 bytes)",
            ),
            (
                ParseError::SectionOutOfBounds {
                    section: Section::Data,
                    offset: 256,
                    len: 10,
                    file_len: 100,
                },
                "data section (offset 256, length 10) extends past the end of the 100-byte fork",
            ),
            (
                ParseError::SectionOutOfBounds {
                    section: Section::Map,
                    offset: 1,
                    len: 2,
                    file_len: 3,
                },
                "map section (offset 1, length 2) extends past the end of the 3-byte fork",
            ),
            (
                ParseError::MapTruncated,
                "resource map is shorter than its 30-byte header",
            ),
            (
                ParseError::TypeListOutOfBounds,
                "type list extends past the end of the resource map",
            ),
            (
                ParseError::NameListOutOfBounds,
                "name list starts past the end of the resource map",
            ),
            (
                ParseError::ReferenceListOutOfBounds { ty: SHIP },
                "reference list for type 'shïp' extends past the end of the resource map",
            ),
            (
                ParseError::DuplicateType { ty: SHIP },
                "type 'shïp' appears more than once in the type list",
            ),
            (
                ParseError::DuplicateId { ty: SHIP, id: -3 },
                "resource 'shïp' -3 appears more than once",
            ),
            (
                ParseError::DataOffsetOutOfBounds {
                    ty: SHIP,
                    id: 128,
                    offset: 77,
                },
                "resource 'shïp' 128: data offset 77 is outside the data section",
            ),
            (
                ParseError::DataLengthOutOfBounds {
                    ty: SHIP,
                    id: 128,
                    len: 99,
                },
                "resource 'shïp' 128: data length 99 runs past the end of the data section",
            ),
            (
                ParseError::NameOffsetOutOfBounds {
                    ty: SHIP,
                    id: 128,
                    offset: 5,
                },
                "resource 'shïp' 128: name offset 5 is outside the name list",
            ),
            (
                ParseError::NameLengthOutOfBounds { ty: SHIP, id: 128 },
                "resource 'shïp' 128: name runs past the end of the resource map",
            ),
            (
                ParseError::CompressedResource { ty: SHIP, id: 128 },
                "resource 'shïp' 128 is compressed, which is not supported",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn parse_error_is_a_std_error() {
        let error: Box<dyn std::error::Error> = Box::new(ParseError::MapTruncated);
        assert!(error.source().is_none());
    }

    #[test]
    fn load_errors_name_the_path_and_chain_their_source() {
        use std::error::Error as _;
        let path = std::path::PathBuf::from("Nova Files/Nova Data 1.ndat");

        let io = LoadError::Io {
            path: path.clone(),
            source: std::io::Error::other("denied"),
        };
        assert_eq!(
            io.to_string(),
            "reading Nova Files/Nova Data 1.ndat: denied"
        );
        assert_eq!(io.source().expect("has source").to_string(), "denied");

        let parse = LoadError::Parse {
            path: path.clone(),
            source: ParseError::MapTruncated,
        };
        assert_eq!(
            parse.to_string(),
            "parsing Nova Files/Nova Data 1.ndat: resource map is shorter than its 30-byte header"
        );
        let source = parse.source().expect("has source");
        assert_eq!(
            source.downcast_ref::<ParseError>(),
            Some(&ParseError::MapTruncated)
        );

        let none = LoadError::NoResourceFork { path };
        assert_eq!(
            none.to_string(),
            "Nova Files/Nova Data 1.ndat has no resource fork"
        );
        assert!(none.source().is_none());
    }
}
