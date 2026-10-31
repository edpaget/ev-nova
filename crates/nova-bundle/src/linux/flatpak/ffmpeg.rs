//! The FFmpeg source the movie decoder builds, read from the workspace's
//! one record of it (`third_party/ffmpeg-source.toml`), so the offline
//! Flatpak build is handed the same tarball the vendored ffmpeg-sys-next's
//! build script checks.

use serde::Deserialize;

/// The record, as checked in.
const RECORD: &str = include_str!("../../../../../third_party/ffmpeg-source.toml");

/// One FFmpeg release tarball, pinned by its SHA-256.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FfmpegSource {
    /// FFmpeg's version, such as `9.0.2`.
    pub version: String,
    /// Where the tarball is downloaded from.
    pub url: String,
    /// Its SHA-256, as lowercase hex.
    pub sha256: String,
}

impl FfmpegSource {
    /// The workspace's record.
    #[must_use]
    pub fn record() -> Self {
        toml::from_str(RECORD)
            .unwrap_or_else(|e| unreachable!("third_party/ffmpeg-source.toml is checked in: {e}"))
    }

    /// The tarball's file name: the last part of its URL.
    #[must_use]
    pub fn file_name(&self) -> &str {
        self.url.rsplit('/').next().unwrap_or(&self.url)
    }
}
