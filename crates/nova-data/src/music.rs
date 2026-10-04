//! The game's music: where it is, and reading it.
//!
//! EV Nova plays one soundtrack, `Nova Music.mp3`, a loose MP3 file inside
//! `Nova Files` (the store walk skips it as not game data). Reading it
//! means finding the player's own `<Nova Files>/Nova Music.mp3`
//! ([`music_path`]) and reading its bytes ([`load_music`]), which go to the
//! audio player unchanged. No format check happens here: the player's MP3
//! decoder is the judge, and the player reports a file it cannot decode.
//! The music is the player's copy and is never committed.

use std::io;
use std::path::{Path, PathBuf};

use nova_rsrc::{Fork, ForkReader, StdForkReader};

/// Why the music could not be read.
#[derive(Debug, thiserror::Error)]
pub enum MusicError {
    /// The file is not there.
    #[error("no music at {}", path.display())]
    Missing {
        /// Where it was looked for.
        path: PathBuf,
    },
    /// The file could not be read.
    #[error("reading the music {}: {source}", path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// The read error.
        source: io::Error,
    },
}

/// Where the music sits for the `Nova Files` directory `data_dir`:
/// `<data_dir>/Nova Music.mp3`.
#[must_use]
pub fn music_path(data_dir: &Path) -> PathBuf {
    data_dir.join("Nova Music.mp3")
}

/// Reads the music from disk for the `Nova Files` directory `data_dir`.
pub fn open_music(data_dir: &Path) -> Result<Vec<u8>, MusicError> {
    load_music(&StdForkReader, data_dir)
}

/// Reads the music's data fork through `forks`, returning its bytes
/// unchanged.
pub fn load_music(forks: &impl ForkReader, data_dir: &Path) -> Result<Vec<u8>, MusicError> {
    let path = music_path(data_dir);
    match forks.read_fork(&path, Fork::Data) {
        Ok(Some(bytes)) => Ok(bytes),
        Ok(None) => Err(MusicError::Missing { path }),
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            Err(MusicError::Missing { path })
        }
        Err(source) => Err(MusicError::Io { path, source }),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

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

    fn music() -> PathBuf {
        Path::new(DATA_DIR).join("Nova Music.mp3")
    }

    #[test]
    fn the_music_is_nova_music_mp3_in_nova_files() {
        assert_eq!(music_path(Path::new(DATA_DIR)), music());
    }

    #[test]
    fn the_music_is_read_from_its_data_fork_unchanged() {
        let forks = OneFile::new(|| Ok(Some(b"ID3 not checked".to_vec())));
        let bytes = load_music(&forks, Path::new(DATA_DIR)).expect("loads");
        assert_eq!(bytes, b"ID3 not checked");
        assert_eq!(forks.asked.into_inner(), [(music(), Fork::Data)]);
    }

    #[test]
    fn a_missing_file_is_missing() {
        let not_found = OneFile::new(|| Err(io::Error::from(io::ErrorKind::NotFound)));
        let err = load_music(&not_found, Path::new(DATA_DIR)).expect_err("fails");
        assert!(
            matches!(&err, MusicError::Missing { path } if *path == music()),
            "{err:?}"
        );
        assert_eq!(
            err.to_string(),
            format!("no music at {}", music().display())
        );
        let no_fork = OneFile::new(|| Ok(None));
        let err = load_music(&no_fork, Path::new(DATA_DIR)).expect_err("fails");
        assert!(
            matches!(&err, MusicError::Missing { path } if *path == music()),
            "{err:?}"
        );
    }

    #[test]
    fn an_unreadable_file_is_an_io_error() {
        let forks = OneFile::new(|| Err(io::Error::other("disk on fire")));
        let err = load_music(&forks, Path::new(DATA_DIR)).expect_err("fails");
        assert!(
            matches!(&err, MusicError::Io { path, .. } if *path == music()),
            "{err:?}"
        );
        assert_eq!(
            err.to_string(),
            format!("reading the music {}: disk on fire", music().display())
        );
    }

    #[test]
    fn opening_reads_the_real_file() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(dir.path().join("Nova Music.mp3"), b"mp3 bytes").expect("writes");
        assert_eq!(open_music(dir.path()).expect("opens"), b"mp3 bytes");
        let empty = tempfile::tempdir().expect("a temporary directory");
        let err = open_music(empty.path()).expect_err("no file");
        assert!(matches!(err, MusicError::Missing { .. }), "{err:?}");
    }
}
