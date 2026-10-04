//! The interface file: the game's dialogs and their pictures.
//!
//! [`InterfaceData`] opens one file holding the interface resources
//! (`DLOG`, `DITL`, `PICT`, ...) and decodes them on request. It is kept
//! apart from [`GameData`](super::GameData): the interface's `PICT` IDs
//! overlap the game data's, and the two must never shadow each other.
//!
//! The caller names the file, because it sits beside `Nova Files` rather
//! than inside it. Two files qualify: `Nova-DF.rsrc` (Mac OS X), whose data
//! fork is a flattened resource fork, and the Windows `Nova.rez`, which
//! holds the same resources. Any file [`ResourceFile::load`] can read works.
//!
//! [`load_interface`] (and [`open_interface`], on disk) finds it from the
//! `Nova Files` directory: `Nova-DF.rsrc` beside it, else `Nova.rez`
//! beside it ([`interface_path_candidates`]).

use std::path::{Path, PathBuf};

use nova_rsrc::{ForkReader, LoadError, ResType, ResourceFile, StdForkReader};

use crate::decode::{Entry, Record, decode};
use crate::error::{DecodeError, DecodeWarning};
use crate::graphics::{GraphicsError, Image, PICT, decode_pict};
use crate::records::dialog::Dlog;
use crate::records::dialog_items::Ditl;
use crate::wire::id::{DitlId, PictId};

/// A decoded interface record, or why it failed.
type DecodeResult<T> = Result<(Entry<T>, Option<DecodeWarning>), DecodeError>;

/// Where the interface file may be, given the `Nova Files` directory
/// `data_dir`, in the order they are tried: the Mac OS X `Nova-DF.rsrc`
/// beside it, then the Windows `Nova.rez` beside it.
#[must_use]
pub fn interface_path_candidates(data_dir: &Path) -> [PathBuf; 2] {
    [
        data_dir.join("../Nova-DF.rsrc"),
        data_dir.join("../Nova.rez"),
    ]
}

/// Neither interface file could be opened.
#[derive(Debug, thiserror::Error)]
#[error(
    "no interface file: {} ({}); {} ({})",
    tried[0].0.display(), tried[0].1, tried[1].0.display(), tried[1].1
)]
pub struct NoInterfaceFile {
    /// Each path tried, in order, with why it failed.
    pub tried: Box<[(PathBuf, LoadError); 2]>,
}

/// Opens the interface file for the `Nova Files` directory `data_dir`
/// through `forks`: the first of [`interface_path_candidates`] that loads.
pub fn load_interface(
    forks: &impl ForkReader,
    data_dir: &Path,
) -> Result<InterfaceData, NoInterfaceFile> {
    let [mac, windows] = interface_path_candidates(data_dir);
    let mac_error = match InterfaceData::load(forks, &mac) {
        Ok(ui) => return Ok(ui),
        Err(error) => error,
    };
    match InterfaceData::load(forks, &windows) {
        Ok(ui) => Ok(ui),
        Err(error) => Err(NoInterfaceFile {
            tried: Box::new([(mac, mac_error), (windows, error)]),
        }),
    }
}

/// [`load_interface`] from disk.
pub fn open_interface(data_dir: &Path) -> Result<InterfaceData, NoInterfaceFile> {
    load_interface(&StdForkReader, data_dir)
}

/// The interface file's resources. Read-only; every lookup decodes afresh.
#[derive(Debug)]
pub struct InterfaceData {
    path: PathBuf,
    resources: ResourceFile,
}

impl InterfaceData {
    /// Opens the interface file at `path` from disk.
    pub fn open(path: &Path) -> Result<Self, LoadError> {
        Self::load(&StdForkReader, path)
    }

    /// Opens the interface file at `path` through `forks`. Unlike a game
    /// data file, which the store skips, a file that cannot be loaded is an
    /// error.
    pub fn load(forks: &impl ForkReader, path: &Path) -> Result<Self, LoadError> {
        Ok(Self {
            path: path.to_path_buf(),
            resources: ResourceFile::load(forks, path)?,
        })
    }

    /// The file's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Every resource in the file, raw.
    #[must_use]
    pub fn resources(&self) -> &ResourceFile {
        &self.resources
    }

    /// Every ID of type `ty`, ascending.
    #[must_use]
    pub fn ids(&self, ty: ResType) -> Vec<i16> {
        let mut ids: Vec<i16> = self.resources.resources(ty).map(|r| r.id()).collect();
        ids.sort_unstable();
        ids
    }

    /// `DLOG` `id`, decoded; `None` if the file has none.
    #[must_use]
    pub fn dialog(&self, id: i16) -> Option<DecodeResult<Dlog>> {
        self.decode(id)
    }

    /// `DITL` `id`, decoded; `None` if the file has none.
    #[must_use]
    pub fn items(&self, id: DitlId) -> Option<DecodeResult<Ditl>> {
        self.decode(id.0)
    }

    /// `PICT` `id`, decoded; `None` if the file has none.
    #[must_use]
    pub fn picture(&self, id: PictId) -> Option<Result<Image, GraphicsError>> {
        let res = self.resources.get(PICT, id.0)?;
        Some(decode_pict(res.data()))
    }

    fn decode<T: Record>(&self, id: i16) -> Option<DecodeResult<T>> {
        let res = self.resources.get(T::TYPE, id)?;
        Some(decode::<T>(&res))
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use nova_rsrc::fixture::{ForkBuilder, RezBuilder};
    use nova_rsrc::{LoadError, ResType};

    use super::*;
    use crate::decode::Record;
    use crate::graphics::PICT;
    use crate::graphics::fixture::PictBuilder;
    use crate::records::dialog::Dlog;
    use crate::records::dialog_items::tests::{ditl, item};
    use crate::records::dialog_items::{Ditl, ItemKind};
    use crate::store::GameData;
    use crate::store::fake::{FakeForks, FakeTree};
    use crate::store::fs::EntryKind::File;
    use crate::wire::id::{DitlId, PictId};

    const PATH: &str = "/game/Nova-DF.rsrc";

    /// `DLOG` 128 naming `DITL` `items`, with no position word.
    fn dlog(items: i16) -> Vec<u8> {
        let mut bytes = vec![0, 0, 0, 0, 0, 100, 0, 200, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0];
        bytes.extend(items.to_be_bytes());
        bytes.push(0);
        bytes
    }

    /// An empty `width` x `height` picture.
    fn pict(width: i16, height: i16) -> Vec<u8> {
        PictBuilder::new([0, 0, height, width]).end().build()
    }

    /// The interface resources: `DLOG` 128, its `DITL` 128, a malformed
    /// `DITL` 129, and a 5x3 `PICT` 128.
    fn resources() -> Vec<(ResType, i16, Vec<u8>)> {
        vec![
            (Dlog::TYPE, 128, dlog(128)),
            (Ditl::TYPE, 129, ditl(&[item(64, &[0])])),
            (
                Ditl::TYPE,
                128,
                ditl(&[item(4, b"OK"), item(64, &[0, 128])]),
            ),
            (PICT, 128, pict(5, 3)),
        ]
    }

    fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> Vec<u8> {
        resources
            .iter()
            .fold(ForkBuilder::new(), |b, (ty, id, data)| {
                b.resource(*ty, *id, Some(b"name"), data)
            })
            .build()
            .bytes
    }

    fn rez(resources: &[(ResType, i16, Vec<u8>)]) -> Vec<u8> {
        resources
            .iter()
            .fold(RezBuilder::new(), |b, (ty, id, data)| {
                b.resource(*ty, *id, Some(b"name"), data)
            })
            .build()
            .bytes
    }

    fn load(bytes: Vec<u8>) -> InterfaceData {
        let forks = FakeForks::new().file(PATH, bytes);
        InterfaceData::load(&forks, Path::new(PATH)).expect("loads")
    }

    /// Everything the lookups return for the [`resources`] fixture.
    fn check_contents(ui: &InterfaceData) {
        check_contents_at(ui, PATH);
    }

    /// [`check_contents`] for a file at `path`.
    fn check_contents_at(ui: &InterfaceData, path: &str) {
        assert_eq!(ui.path(), Path::new(path));
        assert_eq!(ui.resources().len(), 4);
        assert_eq!(ui.ids(Ditl::TYPE), [128, 129]);
        assert_eq!(ui.ids(Dlog::TYPE), [128]);
        assert!(ui.ids(ResType::new(*b"ALRT")).is_empty());

        let (dialog, warning) = ui.dialog(128).expect("present").expect("decodes");
        assert_eq!((dialog.id, dialog.name.as_deref()), (128, Some("name")));
        assert_eq!(dialog.record.items_id, DitlId(128));
        assert_eq!(warning, None);

        let (items, _) = ui.items(DitlId(128)).expect("present").expect("decodes");
        let kinds: Vec<_> = items.record.items.into_iter().map(|i| i.kind).collect();
        assert_eq!(
            kinds,
            [
                ItemKind::Button { title: "OK".into() },
                ItemKind::Picture { pict: PictId(128) }
            ]
        );

        let picture = ui.picture(PictId(128)).expect("present").expect("decodes");
        assert_eq!((picture.width(), picture.height()), (5, 3));
    }

    #[test]
    fn a_flattened_fork_decodes_dialogs_items_and_pictures() {
        check_contents(&load(fork(&resources())));
    }

    #[test]
    fn a_rez_file_decodes_the_same() {
        check_contents(&load(rez(&resources())));
    }

    #[test]
    fn a_missing_id_is_none() {
        let ui = load(fork(&resources()));
        assert!(ui.dialog(129).is_none());
        assert!(ui.items(DitlId(130)).is_none());
        assert!(ui.picture(PictId(129)).is_none());
    }

    #[test]
    fn a_malformed_item_list_fails_alone() {
        let ui = load(fork(&resources()));
        let err = ui.items(DitlId(129)).expect("present").expect_err("bad");
        assert_eq!((err.res_type, err.id), (Ditl::TYPE, 129));
        assert!(ui.items(DitlId(128)).expect("present").is_ok());
    }

    #[test]
    fn a_bad_picture_is_a_graphics_error() {
        let ui = load(fork(&[(PICT, 130, vec![0; 4])]));
        assert!(ui.picture(PictId(130)).expect("present").is_err());
    }

    #[test]
    fn a_trailing_byte_is_a_warning() {
        let mut bytes = dlog(128);
        bytes.extend([0, 0xA8, 0x0A, 0xEE]);
        let ui = load(fork(&[(Dlog::TYPE, 128, bytes)]));
        let (dialog, warning) = ui.dialog(128).expect("present").expect("decodes");
        assert_eq!(dialog.record.position, Some(0xA80A));
        assert!(warning.is_some());
    }

    #[test]
    fn an_unreadable_or_missing_file_is_an_io_error() {
        let forks = FakeForks::new().unreadable(PATH);
        let err = InterfaceData::load(&forks, Path::new(PATH)).expect_err("fails");
        assert!(matches!(err, LoadError::Io { .. }), "{err:?}");
        let err = InterfaceData::load(&FakeForks::new(), Path::new(PATH)).expect_err("fails");
        assert!(matches!(err, LoadError::Io { .. }), "{err:?}");
    }

    #[test]
    fn garbage_is_a_parse_error_and_nothing_is_no_fork() {
        let forks = FakeForks::new().file(PATH, b"not a resource fork".to_vec());
        let err = InterfaceData::load(&forks, Path::new(PATH)).expect_err("fails");
        assert!(matches!(err, LoadError::Parse { .. }), "{err:?}");
        let forks = FakeForks::new().file(PATH, Vec::new());
        let err = InterfaceData::load(&forks, Path::new(PATH)).expect_err("fails");
        assert!(matches!(err, LoadError::NoResourceFork { .. }), "{err:?}");
    }

    #[test]
    fn interface_pictures_are_separate_from_the_game_data() {
        let tree = FakeTree::new().dir("/d", &[("Nova Graphics", File)]);
        let forks = FakeForks::new()
            .file(PATH, fork(&resources()))
            .file("/d/Nova Graphics", fork(&[(PICT, 128, pict(7, 2))]));
        let data = GameData::load(&tree, &forks, Path::new("/d"), None).expect("opens");
        let ui = InterfaceData::load(&forks, Path::new(PATH)).expect("loads");

        let game = data.resource(PICT, 128).expect("present").resource;
        let game = crate::graphics::decode_pict(game.data()).expect("decodes");
        assert_eq!((game.width(), game.height()), (7, 2));
        let picture = ui.picture(PictId(128)).expect("present").expect("decodes");
        assert_eq!((picture.width(), picture.height()), (5, 3));
    }

    #[test]
    fn the_candidates_are_the_mac_file_then_the_windows_file_beside_the_data() {
        assert_eq!(
            interface_path_candidates(Path::new("/game/Nova Files")),
            [
                PathBuf::from("/game/Nova Files/../Nova-DF.rsrc"),
                PathBuf::from("/game/Nova Files/../Nova.rez"),
            ]
        );
    }

    const MAC: &str = "/game/Nova Files/../Nova-DF.rsrc";
    const WINDOWS: &str = "/game/Nova Files/../Nova.rez";

    #[test]
    fn the_mac_file_is_preferred() {
        let forks = FakeForks::new()
            .file(MAC, fork(&resources()))
            .file(WINDOWS, rez(&[(PICT, 128, pict(7, 2))]));
        let ui = load_interface(&forks, Path::new("/game/Nova Files")).expect("loads");
        assert_eq!(ui.path(), Path::new(MAC));
        check_contents_at(&ui, MAC);
    }

    #[test]
    fn the_windows_file_is_the_fallback() {
        for forks in [
            FakeForks::new(),
            FakeForks::new().unreadable(MAC),
            FakeForks::new().file(MAC, b"garbage".to_vec()),
        ] {
            let forks = forks.file(WINDOWS, rez(&resources()));
            let ui = load_interface(&forks, Path::new("/game/Nova Files")).expect("loads");
            check_contents_at(&ui, WINDOWS);
        }
    }

    #[test]
    fn with_neither_file_the_error_names_both() {
        let forks = FakeForks::new().unreadable(WINDOWS);
        let err = load_interface(&forks, Path::new("/game/Nova Files")).expect_err("fails");
        assert_eq!(err.tried[0].0, PathBuf::from(MAC));
        assert_eq!(err.tried[1].0, PathBuf::from(WINDOWS));
        assert!(matches!(err.tried[0].1, LoadError::Io { .. }));
        let (mac, windows) = (err.tried[0].0.display(), err.tried[1].0.display());
        let message = err.to_string();
        assert!(
            message.starts_with(&format!("no interface file: {mac} (")),
            "{message}"
        );
        assert!(message.contains(&format!("); {windows} (")), "{message}");
        assert!(message.contains("disk on fire"), "{message}");
    }

    #[test]
    fn it_can_be_shared_across_threads() {
        fn shareable<T: Send + Sync>() {}
        shareable::<InterfaceData>();
    }
}
