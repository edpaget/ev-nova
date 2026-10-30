//! EV Nova's QuickTime movies, decoded with FFmpeg.
//!
//! FFmpeg is built from source and linked statically by the vendored
//! `ffmpeg-sys-next` (`third_party/ffmpeg-sys-next`, whose README lists
//! its changes), from the release tarball `third_party/ffmpeg-source.toml`
//! pins. Only what the movies need is configured in: the mov demuxer, the
//! file protocol, the H.264 decoder the stock movies use and the older
//! QuickTime decoders a plug-in's movies may use (Cinepak, Sorenson Video
//! 1 and 3, Animation, Road Pizza, Graphics), and swscale
//! ([`ffmpeg::configuration`] reports the line).
//!
//! Building it needs a C compiler and `make`, `curl` and `tar` (the first
//! build downloads the 12 MB source once per machine, into
//! `$CARGO_HOME/nova-ffmpeg`, and checks its SHA-256;
//! `NOVA_FFMPEG_TARBALL` names a local copy instead), and libclang for
//! bindgen. On Windows, MSVC builds it inside MSYS2 (`make` and
//! `diffutils`) with LLVM's libclang.

pub mod ffmpeg;
