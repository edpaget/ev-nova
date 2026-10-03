//! The game's own font, Charcoal: where it is, and whether it is usable.
//!
//! # Where Charcoal comes from
//!
//! EV Nova draws its interface in Geneva and Charcoal. The Mac OS X release
//! does not keep Charcoal in a classic font suitcase (`FOND`/`NFNT`/`sfnt`
//! resources): it ships it as a loose TrueType file,
//! `EV Nova.app/Contents/Resources/Fonts/Charcoal.ttf`, beside the
//! `Nova Files` directory. No stock file holds any font resource
//! (`Nova-DF.rsrc`, the `.ndat` files, `Nova.rez` and the Windows `.rez`
//! files were all checked), and the Windows Community Edition ships no
//! Charcoal at all.
//!
//! So "extracting" Charcoal means finding the player's own
//! `<Nova Files>/../Fonts/Charcoal.ttf` ([`charcoal_path`]), reading it
//! ([`load_charcoal`]) and checking it is a usable outline font
//! ([`check_sfnt`]); its bytes go to the renderer unchanged. There is no
//! suitcase parser or bitmap-font converter, because no stock data has a
//! suitcase to feed one. When the file is missing or fails the check, the
//! renderer draws Charcoal text in its bundled substitute font instead.
//! The font is the player's copy and is never committed.
//!
//! The `fixture` feature exposes the `fixture` module, synthetic font
//! builders, to other crates' tests.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use nova_rsrc::{Fork, ForkReader, StdForkReader};

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

/// A four-byte sfnt tag, shown as text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tag(pub [u8; 4]);

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.escape_ascii())
    }
}

/// Why bytes are not a usable outline font.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SfntError {
    /// The header or table directory is cut short.
    #[error("the font's header or table directory is truncated")]
    Truncated,
    /// A TrueType collection (`ttcf`), not a single font.
    #[error("a font collection, not a single font")]
    Collection,
    /// The scaler tag is not TrueType or OpenType.
    #[error("not a TrueType or OpenType font (scaler tag {0})")]
    UnknownScaler(Tag),
    /// The table directory is empty.
    #[error("the font has no tables")]
    NoTables,
    /// A table runs past the end of the file.
    #[error("table {0} runs past the end of the font")]
    TableOutOfBounds(Tag),
    /// A required table is missing.
    #[error("the font has no {0} table")]
    MissingTable(Tag),
    /// Neither `glyf`/`loca` nor `CFF `/`CFF2` outlines: a bitmap-only font.
    #[error("the font has no outlines")]
    NoOutlines,
}

/// Why Charcoal could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum FontError {
    /// The file is not there (as with the Windows data).
    #[error("no font at {}", path.display())]
    Missing {
        /// Where it was looked for.
        path: PathBuf,
    },
    /// The file could not be read.
    #[error("reading the font {}: {source}", path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// The read error.
        source: io::Error,
    },
    /// The file is not a usable outline font.
    #[error("the font {} is unusable: {source}", path.display())]
    Invalid {
        /// The file.
        path: PathBuf,
        /// What is wrong with it.
        source: SfntError,
    },
}

/// Where Charcoal sits for the `Nova Files` directory `data_dir`:
/// `<data_dir>/../Fonts/Charcoal.ttf`.
#[must_use]
pub fn charcoal_path(data_dir: &Path) -> PathBuf {
    data_dir.join("../Fonts/Charcoal.ttf")
}

/// Reads Charcoal from disk for the `Nova Files` directory `data_dir`.
pub fn open_charcoal(data_dir: &Path) -> Result<Vec<u8>, FontError> {
    load_charcoal(&StdForkReader, data_dir)
}

/// Reads Charcoal's data fork through `forks` and checks it, returning
/// its bytes unchanged.
pub fn load_charcoal(forks: &impl ForkReader, data_dir: &Path) -> Result<Vec<u8>, FontError> {
    let path = charcoal_path(data_dir);
    let bytes = match forks.read_fork(&path, Fork::Data) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Err(FontError::Missing { path }),
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Err(FontError::Missing { path });
        }
        Err(source) => return Err(FontError::Io { path, source }),
    };
    match check_sfnt(&bytes) {
        Ok(()) => Ok(bytes),
        Err(source) => Err(FontError::Invalid { path, source }),
    }
}

/// Checks `bytes` is a single TrueType or OpenType font with outlines and
/// every table a renderer needs, each inside the file.
pub fn check_sfnt(bytes: &[u8]) -> Result<(), SfntError> {
    let scaler: [u8; 4] = take(bytes, 0)?;
    match &scaler {
        [0, 1, 0, 0] | b"true" | b"OTTO" => {}
        b"ttcf" => return Err(SfntError::Collection),
        _ => return Err(SfntError::UnknownScaler(Tag(scaler))),
    }
    let count = usize::from(u16::from_be_bytes(take(bytes, 4)?));
    if count == 0 {
        return Err(SfntError::NoTables);
    }
    let directory = bytes.get(12..12 + 16 * count).ok_or(SfntError::Truncated)?;
    let mut tags = Vec::with_capacity(count);
    for record in directory.as_chunks::<16>().0 {
        let tag: [u8; 4] = take(record, 0)?;
        let offset = u64::from(u32::from_be_bytes(take(record, 8)?));
        let length = u64::from(u32::from_be_bytes(take(record, 12)?));
        if offset + length > bytes.len() as u64 {
            return Err(SfntError::TableOutOfBounds(Tag(tag)));
        }
        tags.push(tag);
    }
    let has = |tag: &[u8; 4]| tags.contains(tag);
    if let Some(missing) = REQUIRED_TABLES.into_iter().find(|tag| !has(tag)) {
        return Err(SfntError::MissingTable(Tag(*missing)));
    }
    if !((has(b"glyf") && has(b"loca")) || has(b"CFF ") || has(b"CFF2")) {
        return Err(SfntError::NoOutlines);
    }
    Ok(())
}

/// The tables every font must have to be shaped and drawn.
pub const REQUIRED_TABLES: [&[u8; 4]; 6] = [b"cmap", b"head", b"hhea", b"hmtx", b"maxp", b"name"];

/// The `N` bytes of `bytes` at `at`, or [`SfntError::Truncated`].
fn take<const N: usize>(bytes: &[u8], at: usize) -> Result<[u8; N], SfntError> {
    bytes
        .get(at..at + N)
        .and_then(|slice| slice.try_into().ok())
        .ok_or(SfntError::Truncated)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::fixture::{REQUIRED, SfntBuilder, block_font};
    use super::*;

    #[test]
    fn truetype_apple_truetype_and_opentype_scalers_are_accepted() {
        let truetype = SfntBuilder::truetype();
        assert_eq!(check_sfnt(&truetype.build()), Ok(()));
        let mut apple = truetype.build();
        apple[..4].copy_from_slice(b"true");
        assert_eq!(check_sfnt(&apple), Ok(()));
        let opentype = REQUIRED
            .into_iter()
            .fold(SfntBuilder::new(b"OTTO"), |sfnt, tag| {
                sfnt.table(tag, [0; 4])
            })
            .table(b"CFF ", [0; 4]);
        assert_eq!(check_sfnt(&opentype.build()), Ok(()));
        let cff2 = opentype.without(b"CFF ").table(b"CFF2", [0; 4]);
        assert_eq!(check_sfnt(&cff2.build()), Ok(()));
    }

    #[test]
    fn a_collection_is_rejected() {
        let mut bytes = SfntBuilder::truetype().build();
        bytes[..4].copy_from_slice(b"ttcf");
        assert_eq!(check_sfnt(&bytes), Err(SfntError::Collection));
    }

    #[test]
    fn an_unknown_scaler_is_rejected() {
        let bytes = SfntBuilder::truetype().build();
        for scaler in [*b"typ1", [0, 1, 0, 1], [1, 1, 0, 0]] {
            let mut bad = bytes.clone();
            bad[..4].copy_from_slice(&scaler);
            assert_eq!(
                check_sfnt(&bad),
                Err(SfntError::UnknownScaler(Tag(scaler))),
                "{scaler:?}"
            );
        }
        assert_eq!(
            check_sfnt(b"not a font at all"),
            Err(SfntError::UnknownScaler(Tag(*b"not "))),
        );
    }

    #[test]
    fn a_short_header_is_truncated() {
        let bytes = SfntBuilder::truetype().build();
        for len in [0, 3, 4, 11] {
            assert_eq!(
                check_sfnt(&bytes[..len]),
                Err(SfntError::Truncated),
                "{len} bytes"
            );
        }
    }

    #[test]
    fn a_short_table_directory_is_truncated() {
        let bytes = SfntBuilder::truetype().build();
        // Eight tables: the directory ends at byte 12 + 8 * 16 = 140.
        assert_eq!(check_sfnt(&bytes[..139]), Err(SfntError::Truncated));
        assert_eq!(check_sfnt(&bytes[..12]), Err(SfntError::Truncated));
    }

    #[test]
    fn a_font_with_no_tables_is_rejected() {
        let bytes = SfntBuilder::new(&[0, 1, 0, 0]).build();
        assert_eq!(bytes.len(), 12);
        assert_eq!(check_sfnt(&bytes), Err(SfntError::NoTables));
    }

    /// Sets the length of the directory's `index`th table record.
    fn set_length(bytes: &mut [u8], index: usize, length: u32) {
        let at = 12 + 16 * index + 12;
        bytes[at..at + 4].copy_from_slice(&length.to_be_bytes());
    }

    #[test]
    fn a_table_past_the_end_is_rejected() {
        let mut bytes = SfntBuilder::truetype().build();
        // Sorted by tag, the last table is `name`, ending flush with the file.
        let len = bytes.len() as u32;
        let last = 7;
        assert_eq!(&bytes[12 + 16 * last..][..4], b"name");
        set_length(&mut bytes, last, 4);
        assert_eq!(check_sfnt(&bytes), Ok(()), "flush with the end");
        set_length(&mut bytes, last, 5);
        assert_eq!(
            check_sfnt(&bytes),
            Err(SfntError::TableOutOfBounds(Tag(*b"name")))
        );
        // An offset and length that overflow `u32` still fail cleanly.
        set_length(&mut bytes, 0, u32::MAX);
        assert_eq!(
            check_sfnt(&bytes),
            Err(SfntError::TableOutOfBounds(Tag(*b"cmap")))
        );
        set_length(&mut bytes, 0, len);
        assert!(check_sfnt(&bytes).is_err());
    }

    #[test]
    fn each_required_table_must_be_present() {
        for tag in REQUIRED {
            let bytes = SfntBuilder::truetype().without(tag).build();
            assert_eq!(
                check_sfnt(&bytes),
                Err(SfntError::MissingTable(Tag(*tag))),
                "{}",
                Tag(*tag)
            );
        }
    }

    #[test]
    fn a_bitmap_only_font_is_rejected() {
        let bitmap = SfntBuilder::truetype()
            .without(b"glyf")
            .without(b"loca")
            .table(b"EBDT", [0; 4])
            .table(b"EBLC", [0; 4]);
        assert_eq!(check_sfnt(&bitmap.build()), Err(SfntError::NoOutlines));
        let glyf_only = SfntBuilder::truetype().without(b"loca");
        assert_eq!(check_sfnt(&glyf_only.build()), Err(SfntError::NoOutlines));
        let loca_only = SfntBuilder::truetype().without(b"glyf");
        assert_eq!(check_sfnt(&loca_only.build()), Err(SfntError::NoOutlines));
    }

    #[test]
    fn the_block_font_passes() {
        assert_eq!(check_sfnt(&block_font("Charcoal")), Ok(()));
    }

    #[test]
    fn tags_show_as_text_with_odd_bytes_escaped() {
        assert_eq!(Tag(*b"CFF ").to_string(), "CFF ");
        assert_eq!(Tag([0, 1, b'a', 0xFF]).to_string(), "\\x00\\x01a\\xff");
    }

    /// A fork reader with one canned answer for every data fork, recording
    /// the paths it is asked for.
    struct OneFile {
        answer: fn() -> io::Result<Option<Vec<u8>>>,
        asked: RefCell<Vec<(PathBuf, Fork)>>,
    }

    impl OneFile {
        fn new(answer: fn() -> io::Result<Option<Vec<u8>>>) -> Self {
            Self {
                answer,
                asked: RefCell::new(Vec::new()),
            }
        }
    }

    impl ForkReader for OneFile {
        fn read_fork(&self, path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            self.asked.borrow_mut().push((path.to_path_buf(), fork));
            (self.answer)()
        }
    }

    const DATA_DIR: &str = "/games/EV Nova/Nova Files";
    const CHARCOAL: &str = "/games/EV Nova/Nova Files/../Fonts/Charcoal.ttf";

    #[test]
    fn charcoal_is_in_the_fonts_folder_beside_nova_files() {
        assert_eq!(charcoal_path(Path::new(DATA_DIR)), PathBuf::from(CHARCOAL));
    }

    #[test]
    fn a_valid_font_is_read_from_its_data_fork_unchanged() {
        let forks = OneFile::new(|| Ok(Some(block_font("Charcoal"))));
        let bytes = load_charcoal(&forks, Path::new(DATA_DIR)).expect("loads");
        assert_eq!(bytes, block_font("Charcoal"));
        assert_eq!(
            forks.asked.into_inner(),
            [(PathBuf::from(CHARCOAL), Fork::Data)]
        );
    }

    #[test]
    fn a_missing_file_is_missing() {
        let not_found = OneFile::new(|| Err(io::Error::from(io::ErrorKind::NotFound)));
        let err = load_charcoal(&not_found, Path::new(DATA_DIR)).expect_err("fails");
        assert!(
            matches!(&err, FontError::Missing { path } if path == Path::new(CHARCOAL)),
            "{err:?}"
        );
        assert_eq!(err.to_string(), format!("no font at {CHARCOAL}"));
        let no_fork = OneFile::new(|| Ok(None));
        let err = load_charcoal(&no_fork, Path::new(DATA_DIR)).expect_err("fails");
        assert!(matches!(err, FontError::Missing { .. }), "{err:?}");
    }

    #[test]
    fn an_unreadable_file_is_an_io_error() {
        let forks = OneFile::new(|| Err(io::Error::other("disk on fire")));
        let err = load_charcoal(&forks, Path::new(DATA_DIR)).expect_err("fails");
        assert!(
            matches!(&err, FontError::Io { path, .. } if path == Path::new(CHARCOAL)),
            "{err:?}"
        );
        assert_eq!(
            err.to_string(),
            format!("reading the font {CHARCOAL}: disk on fire")
        );
    }

    #[test]
    fn invalid_bytes_are_invalid() {
        let forks = OneFile::new(|| Ok(Some(b"ttcf and then nothing".to_vec())));
        let err = load_charcoal(&forks, Path::new(DATA_DIR)).expect_err("fails");
        assert!(
            matches!(
                &err,
                FontError::Invalid { path, source: SfntError::Collection }
                    if path == Path::new(CHARCOAL)
            ),
            "{err:?}"
        );
        assert_eq!(
            err.to_string(),
            format!("the font {CHARCOAL} is unusable: a font collection, not a single font")
        );
    }
}
