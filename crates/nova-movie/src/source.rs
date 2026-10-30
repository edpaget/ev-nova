//! The decoded frames the [`Player`](crate::Player) times: the port its
//! decisions run on, so they are tested on a hand-written source, and the
//! FFmpeg adapter ([`crate::ffmpeg`]) is the one real implementation.

use std::path::Path;
use std::time::Duration;

/// One decoded frame, in display order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// When it is shown, from the movie's start.
    pub at: Duration,
    /// How long it is shown, when the movie says (FFmpeg's frame
    /// duration). A movie whose length isn't known ends as its last frame
    /// does: at its `at` plus this, or at its `at` when this isn't known.
    pub duration: Option<Duration>,
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
    /// Its pixels: straight RGBA8, row-major, top row first,
    /// `4 × width × height` bytes.
    pub rgba: Vec<u8>,
}

/// Why a movie stopped decoding part way.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the movie couldn't be decoded: {0}")]
pub struct DecodeError(pub String);

/// An opened movie's frames.
pub trait FrameSource {
    /// The next frame in display order, `None` once there are no more,
    /// or why decoding failed.
    fn next_frame(&mut self) -> Result<Option<Frame>, DecodeError>;
    /// The movie's length, when it says.
    fn duration(&self) -> Option<Duration>;
    /// Its natural size in pixels, width then height.
    fn size(&self) -> (u32, u32);
}

/// Opens a movie file.
pub trait Open {
    /// The frames of a movie it opens.
    type Source: FrameSource;
    /// The movie at `path`, or `None` when it can't be opened or holds no
    /// video it can decode.
    fn open(&self, path: &Path) -> Option<Self::Source>;
}
