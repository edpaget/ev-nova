//! Decoding the checked-in fixtures (`tests/fixtures/README.md`).

use std::path::PathBuf;
use std::time::Duration;

use ffmpeg_next::codec::{self, Profile, profile::H264};
use ffmpeg_next::format;
use ffmpeg_next::media::Type;
use nova_movie::ffmpeg::MovieFile;
use nova_movie::source::{Frame, FrameSource};

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

/// Every frame of the fixture `name`, decoded.
fn decode_all(name: &str) -> (MovieFile, Vec<Frame>) {
    let mut movie = MovieFile::open(&fixtures().join(name)).expect("opens");
    let mut frames = Vec::new();
    while let Some(frame) = movie.next_frame().expect("decodes") {
        frames.push(frame);
    }
    (movie, frames)
}

#[test]
fn the_fixture_decodes_ten_64x32_frames_in_display_order() {
    let (movie, frames) = decode_all("main-bframes.mov");
    assert_eq!(movie.size(), (64, 32));
    assert_eq!(movie.duration(), Some(Duration::from_secs(1)));
    assert_eq!(frames.len(), 10);
    let ms = Duration::from_millis(1);
    for (i, frame) in frames.iter().enumerate() {
        assert_eq!(frame.at, ms * 100 * i as u32, "frame {i}");
        assert_eq!(frame.duration, Some(ms * 100), "frame {i}");
        assert_eq!((frame.width, frame.height), (64, 32));
        assert_eq!(frame.rgba.len(), 4 * 64 * 32);
        assert!(frame.rgba.chunks(4).all(|pixel| pixel[3] == 255), "opaque");
    }
    // Each frame is flat grey, brighter than the one before: shown in
    // the order they were made, not the order they were stored.
    let greys: Vec<u8> = frames.iter().map(|frame| frame.rgba[4 * 1000]).collect();
    assert!(
        greys.windows(2).all(|pair| pair[0] + 10 < pair[1]),
        "{greys:?}"
    );
    for frame in &frames {
        let pixel = &frame.rgba[4 * 1000..4 * 1000 + 3];
        assert!(
            pixel.iter().all(|&c| c.abs_diff(pixel[0]) <= 2),
            "grey: {pixel:?}"
        );
    }
}

#[test]
fn each_plug_in_codec_fixture_decodes() {
    for name in [
        "cinepak.mov",
        "qtrle.mov",
        "rpza.mov",
        "smc.mov",
        "svq1.mov",
    ] {
        let (movie, frames) = decode_all(name);
        assert_eq!(movie.size(), (64, 32), "{name}");
        assert_eq!(frames.len(), 3, "{name}");
        let ms = Duration::from_millis(1);
        for (i, frame) in frames.iter().enumerate() {
            assert_eq!(frame.at, ms * 100 * i as u32, "{name} frame {i}");
            assert_eq!(frame.rgba.len(), 4 * 64 * 32, "{name}");
        }
        // Coloured, and brighter frame by frame.
        let first = &frames[0].rgba[4 * 1000..4 * 1000 + 4];
        let last = &frames[2].rgba[4 * 1000..4 * 1000 + 4];
        assert_eq!(first[3], 255, "{name} is opaque");
        assert!(first[0] > first[2], "{name} is reddish: {first:?}");
        assert!(
            u32::from(last[0]) + u32::from(last[1]) + u32::from(last[2])
                > u32::from(first[0]) + u32::from(first[1]) + u32::from(first[2]),
            "{name} brightens: {first:?} then {last:?}"
        );
    }
}

#[test]
fn a_file_that_isn_t_a_movie_doesn_t_open() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("Race 1.mov");
    std::fs::write(&path, b"not a movie, just text").expect("write");
    assert!(MovieFile::open(&path).is_none());
    assert!(MovieFile::open(&dir.path().join("missing.mov")).is_none());
}
