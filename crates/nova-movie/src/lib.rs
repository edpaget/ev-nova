//! EV Nova's QuickTime movies, decoded with FFmpeg.
//!
//! [`FfmpegPlayer`] is the engine's [`MoviePlayer`]: it finds a movie by its
//! bare file name in the folders it is given ([`lookup`]), decodes it with
//! FFmpeg ([`ffmpeg`]) and shows its frames as the movie's clock reaches
//! them ([`Player`]).
//!
//! FFmpeg is built from source and linked statically by the vendored
//! `ffmpeg-sys-next` (`third_party/ffmpeg-sys-next`, whose README lists
//! its changes), from the release tarball `third_party/ffmpeg-source.toml`
//! pins. Only what the movies need is configured in: the mov demuxer, the
//! file protocol, the H.264 decoder the stock movies use and the older
//! QuickTime decoders a plug-in's movies may use (Cinepak, Sorenson Video
//! 1 and 3, Animation, Road Pizza, Graphics), and swscale
//! ([`ffmpeg::configuration`] reports the line). Every part is LGPL.
//!
//! Building it needs a C compiler and `make`, `curl` and `tar` (the first
//! build downloads the 12 MB source once per machine, into
//! `$CARGO_HOME/nova-ffmpeg`, and checks its SHA-256;
//! `NOVA_FFMPEG_TARBALL` names a local copy instead), and libclang for
//! bindgen. On Windows, MSVC builds it inside MSYS2 (`make` and
//! `diffutils`) with LLVM's libclang.
//!
//! The decoder's tests play small checked-in movies
//! (`tests/fixtures/README.md` says how each was made), never stock data;
//! the stock test plays `Race 1.mov` to `Race 4.mov` when `NOVA_DATA` is
//! set.
//!
//! [`MoviePlayer`]: nova_view::movie::MoviePlayer

pub mod ffmpeg;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod lookup;
mod player;
pub mod source;

use std::path::PathBuf;

pub use player::Player;

use crate::ffmpeg::FfmpegOpen;
use crate::lookup::StdFolders;

/// The engine's movie player: FFmpeg's decoder over the real folders.
pub type FfmpegPlayer = Player<FfmpegOpen, StdFolders>;

impl FfmpegPlayer {
    /// A player finding movies in `dirs`, the first folder first.
    #[must_use]
    pub fn new(dirs: Vec<PathBuf>) -> Self {
        Self::with(FfmpegOpen, StdFolders, dirs)
    }
}

#[cfg(test)]
mod tests {
    use nova_view::movie::MoviePlayer;

    use super::*;

    #[test]
    fn the_fixture_starts_from_a_folder_it_is_written_into() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join(fixture::NAME), fixture::MOVIE).expect("write");
        let player = FfmpegPlayer::new(vec![dir.path().to_path_buf()]);
        assert_eq!(player.start(fixture::NAME), Some(fixture::SIZE));
    }
}
