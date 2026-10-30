//! Decoding the checked-in fixtures (`tests/fixtures/README.md`).

use std::path::PathBuf;

use ffmpeg_next::codec::{self, Profile, profile::H264};
use ffmpeg_next::format;
use ffmpeg_next::media::Type;

/// The fixtures' folder.
pub fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn the_fixture_is_h264_main_with_b_frames() {
    nova_movie::ffmpeg::init();
    let input = format::input(&fixtures().join("main-bframes.mov")).expect("opens");
    let stream = input.streams().best(Type::Video).expect("a video stream");
    let decoder = codec::context::Context::from_parameters(stream.parameters())
        .expect("parameters")
        .decoder()
        .video()
        .expect("a video decoder");
    assert_eq!(decoder.id(), codec::Id::H264);
    assert_eq!(decoder.profile(), Profile::H264(H264::Main));
    assert!(decoder.has_b_frames(), "the stream reorders frames");
}
