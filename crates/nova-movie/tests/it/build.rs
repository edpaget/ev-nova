//! The FFmpeg linked in is the stripped-down static build: read from the
//! library's own report of how it was configured, and from the decoders
//! it holds.

use ffmpeg_next::codec::{self, Id};

#[test]
fn the_static_ffmpeg_is_the_stripped_down_build() {
    let configuration = nova_movie::ffmpeg::configuration();
    for part in [
        "--disable-everything",
        "--disable-autodetect",
        "--enable-static",
        "--disable-shared",
        "--enable-demuxer=mov",
        "--enable-protocol=file",
        "--disable-x86asm",
    ] {
        assert!(
            configuration.contains(part),
            "{part} is missing from {configuration}"
        );
    }
    assert!(
        !configuration.contains("--enable-gpl"),
        "every component is LGPL: {configuration}"
    );
}

#[test]
fn it_decodes_h264_and_the_quicktime_codecs_and_nothing_else() {
    nova_movie::ffmpeg::init();
    for id in [
        Id::H264,
        Id::CINEPAK,
        Id::SVQ1,
        Id::SVQ3,
        Id::QTRLE,
        Id::RPZA,
        Id::SMC,
    ] {
        assert!(codec::decoder::find(id).is_some(), "{id:?} has no decoder");
    }
    for id in [Id::MPEG4, Id::HEVC, Id::AAC, Id::PNG] {
        assert!(codec::decoder::find(id).is_none(), "{id:?} is built in");
    }
}
