//! The movie decoder over FFmpeg: a static build of the source pinned in
//! `third_party/ffmpeg-source.toml`, configured with only what EV Nova's
//! movies need.

use std::sync::Once;

use ffmpeg_next as ffmpeg;

/// The configure line of the FFmpeg linked in, as the library reports it.
#[must_use]
pub fn configuration() -> &'static str {
    ffmpeg::util::configuration()
}

/// Readies FFmpeg, once per process, with its logging silenced: a movie
/// that won't open is the player's to report, not FFmpeg's to print.
pub fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // Only fails on builds with neither a demuxer nor a decoder.
        let _ = ffmpeg::init();
        ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Quiet);
    });
}
