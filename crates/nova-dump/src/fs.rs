//! The std adapters: the game data and the output directory on disk.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use nova_data::store::{GameData, OpenError};

use crate::ports::{DataSource, OutputRoot, Sink};

/// Adapter: opens the game data from disk with [`GameData::open`].
#[derive(Clone, Copy, Debug, Default)]
pub struct StdDataSource;

impl DataSource for StdDataSource {
    fn open(&self, data: &Path, plugins: Option<&Path>) -> Result<GameData, OpenError> {
        GameData::open(data, plugins)
    }
}

/// Adapter: the output directory on disk.
#[derive(Clone, Copy, Debug, Default)]
pub struct StdOutputRoot;

impl OutputRoot for StdOutputRoot {
    type Sink = DirSink;

    fn create(&self, root: &Path) -> io::Result<()> {
        fs::create_dir_all(root)
    }

    fn is_empty(&self, root: &Path) -> io::Result<bool> {
        Ok(fs::read_dir(root)?.next().is_none())
    }

    fn sink(&self, root: &Path) -> DirSink {
        DirSink {
            root: root.to_path_buf(),
        }
    }
}

/// Adapter: writes new files under a directory, creating their parents and
/// refusing to replace an existing file.
#[derive(Clone, Debug)]
pub struct DirSink {
    root: PathBuf,
}

impl Sink for DirSink {
    fn write(&mut self, rel: &Path, bytes: &[u8]) -> io::Result<()> {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?
            .write_all(bytes)
    }
}

#[cfg(test)]
mod tests {
    use nova_data::Record;
    use nova_data::records::spin::Spin;
    use tempfile::TempDir;

    use super::*;
    use crate::testutil::{fork, record};

    #[test]
    fn the_data_source_opens_a_folder_of_flattened_forks() {
        let dir = TempDir::new().expect("temp dir");
        let data = dir.path().join("data");
        let plugins = dir.path().join("plugins");
        std::fs::create_dir_all(&data).expect("mkdir");
        std::fs::create_dir_all(&plugins).expect("mkdir");
        let spin = record(12, &[(8, 2)]);
        std::fs::write(
            data.join("Nova Data"),
            fork(&[(Spin::TYPE, 200, None, spin.clone())]),
        )
        .expect("write");
        std::fs::write(plugins.join("Plug"), fork(&[(Spin::TYPE, 201, None, spin)]))
            .expect("write");

        let game = StdDataSource.open(&data, Some(&plugins)).expect("opens");
        assert_eq!(game.ids(Spin::TYPE), [200, 201]);
        let game = StdDataSource.open(&data, None).expect("opens");
        assert_eq!(game.ids(Spin::TYPE), [200]);
        assert!(
            StdDataSource
                .open(&dir.path().join("missing"), None)
                .is_err()
        );
    }

    #[test]
    fn create_makes_a_missing_directory_and_its_parents() {
        let dir = TempDir::new().expect("temp dir");
        let out = dir.path().join("a/b/out");
        StdOutputRoot.create(&out).expect("creates");
        assert!(out.is_dir());
        assert!(StdOutputRoot.is_empty(&out).expect("lists"));
        // An existing directory is fine.
        StdOutputRoot.create(&out).expect("exists");
        StdOutputRoot.create(dir.path()).expect("exists");
    }

    #[test]
    fn create_fails_where_a_file_is() {
        let dir = TempDir::new().expect("temp dir");
        let file = dir.path().join("file");
        std::fs::write(&file, b"x").expect("write");
        assert!(StdOutputRoot.create(&file).is_err());
    }

    #[test]
    fn a_directory_with_any_entry_is_not_empty() {
        let dir = TempDir::new().expect("temp dir");
        assert!(StdOutputRoot.is_empty(dir.path()).expect("lists"));
        std::fs::write(dir.path().join(".hidden"), b"").expect("write");
        assert!(!StdOutputRoot.is_empty(dir.path()).expect("lists"));

        let sub = TempDir::new().expect("temp dir");
        std::fs::create_dir(sub.path().join("d")).expect("mkdir");
        assert!(!StdOutputRoot.is_empty(sub.path()).expect("lists"));
    }

    #[test]
    fn listing_a_missing_directory_is_an_error() {
        let dir = TempDir::new().expect("temp dir");
        let error = StdOutputRoot
            .is_empty(&dir.path().join("missing"))
            .expect_err("missing");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn the_sink_writes_files_under_the_root_creating_parents() {
        let dir = TempDir::new().expect("temp dir");
        let mut sink = StdOutputRoot.sink(dir.path());
        sink.write(Path::new("png/rlëD/1000 Shuttle.png"), b"png")
            .expect("writes");
        sink.write(Path::new("png/rlëD/1001.png"), b"more")
            .expect("writes");
        sink.write(Path::new("top.json"), b"[]").expect("writes");
        let read = |rel: &str| std::fs::read(dir.path().join(rel)).expect("written");
        assert_eq!(read("png/rlëD/1000 Shuttle.png"), b"png");
        assert_eq!(read("png/rlëD/1001.png"), b"more");
        assert_eq!(read("top.json"), b"[]");
    }

    #[test]
    fn the_sink_never_overwrites() {
        let dir = TempDir::new().expect("temp dir");
        std::fs::write(dir.path().join("taken.json"), b"old").expect("write");
        let mut sink = StdOutputRoot.sink(dir.path());
        let error = sink
            .write(Path::new("taken.json"), b"new")
            .expect_err("exists");
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            std::fs::read(dir.path().join("taken.json")).expect("read"),
            b"old"
        );
    }
}
