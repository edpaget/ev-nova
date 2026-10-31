//! The movie decoder over FFmpeg: a static build of the source pinned in
//! `third_party/ffmpeg-source.toml`, configured with only what EV Nova's
//! movies need ([`configuration`]).
//!
//! [`MovieFile`] reads one movie's video stream: it demuxes packets, decodes
//! them (single-threaded, so no frame is held back and the result is the
//! same every run), and converts each frame to RGBA with swscale. The
//! decoder reorders B-frames, so frames come out in display order, each
//! timed by its best-effort timestamp from the stream's start, never by
//! counting frames.

use std::path::Path;
use std::sync::Once;
use std::time::Duration;

use ffmpeg::codec::{self, threading};
use ffmpeg::format::{self, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{self, Flags};
use ffmpeg::util::error::EAGAIN;
use ffmpeg::util::frame;
use ffmpeg::{Error, Packet, Rational};
use ffmpeg_next as ffmpeg;

use crate::source::{DecodeError, Frame, FrameSource, Open};

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

/// `ts` ticks of `base` as a [`Duration`]; zero when it is negative.
fn to_duration(ts: i64, base: Rational) -> Duration {
    let (num, den) = (i128::from(base.numerator()), i128::from(base.denominator()));
    if den == 0 {
        return Duration::ZERO;
    }
    let nanos = i128::from(ts) * num * 1_000_000_000 / den;
    Duration::from_nanos(u64::try_from(nanos.max(0)).unwrap_or(u64::MAX))
}

/// A stream or container length, in ticks, unless FFmpeg doesn't know it
/// (`AV_NOPTS_VALUE`, which is negative) or it is zero.
fn known(ticks: i64) -> Option<i64> {
    (ticks > 0).then_some(ticks)
}

/// The microseconds FFmpeg times a container in (`AV_TIME_BASE`).
const MICROS: Rational = Rational(1, 1_000_000);

/// One movie file's video, decoded frame by frame.
pub struct MovieFile {
    input: format::context::Input,
    stream: usize,
    time_base: Rational,
    origin: i64,
    duration: Option<Duration>,
    frames: Option<u64>,
    size: (u32, u32),
    decoder: codec::decoder::Video,
    scaler: Option<scaling::Context>,
}

impl MovieFile {
    /// The movie at `path`, or `None` when it doesn't open, holds no video
    /// stream, or the stream has no decoder here.
    #[must_use]
    pub fn open(path: &Path) -> Option<Self> {
        init();
        let input = format::input(path).ok()?;
        let stream = input.streams().best(Type::Video)?;
        let mut context = codec::context::Context::from_parameters(stream.parameters()).ok()?;
        context.set_threading(threading::Config::count(1));
        let decoder = context.decoder().video().ok()?;
        let time_base = stream.time_base();
        let origin = known(stream.start_time()).unwrap_or(0);
        let duration = known(stream.duration())
            .map(|ticks| to_duration(ticks, time_base))
            .or_else(|| known(input.duration()).map(|micros| to_duration(micros, MICROS)));
        let frames = known(stream.frames()).and_then(|count| u64::try_from(count).ok());
        let size = (decoder.width(), decoder.height());
        let stream = stream.index();
        Some(Self {
            input,
            stream,
            time_base,
            origin,
            duration,
            frames,
            size,
            decoder,
            scaler: None,
        })
    }

    /// How many frames the video stream says it holds, when it says.
    #[must_use]
    pub fn frames(&self) -> Option<u64> {
        self.frames
    }

    /// Hands the decoder the video stream's next packet, or the end of the
    /// stream once there are none.
    fn feed(&mut self) -> Result<(), Error> {
        loop {
            let mut packet = Packet::empty();
            match packet.read(&mut self.input) {
                Ok(()) if packet.stream() == self.stream => {
                    return self.decoder.send_packet(&packet);
                }
                Ok(()) => {}
                Err(Error::Eof) => {
                    return self.decoder.send_eof();
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// `decoded` as a frame: RGBA at its own size, timed from the stream's
    /// start.
    fn convert(&mut self, decoded: &frame::Video) -> Result<Frame, Error> {
        let (format, width, height) = (decoded.format(), decoded.width(), decoded.height());
        let stale = self.scaler.as_ref().is_none_or(|scaler| {
            let input = scaler.input();
            (input.format, input.width, input.height) != (format, width, height)
        });
        if stale {
            self.scaler = Some(scaling::Context::get(
                format,
                width,
                height,
                Pixel::RGBA,
                width,
                height,
                Flags::BILINEAR,
            )?);
        }
        let mut rgba_frame = frame::Video::empty();
        self.scaler
            .as_mut()
            .unwrap_or_else(|| unreachable!("made above"))
            .run(decoded, &mut rgba_frame)?;
        let row = 4 * width as usize;
        let stride = rgba_frame.stride(0);
        let rgba = rgba_frame
            .data(0)
            .chunks(stride)
            .take(height as usize)
            .flat_map(|line| &line[..row])
            .copied()
            .collect();
        let at = decoded.timestamp().map_or(Duration::ZERO, |ts| {
            to_duration(ts - self.origin, self.time_base)
        });
        let duration =
            known(decoded.packet().duration).map(|ticks| to_duration(ticks, self.time_base));
        Ok(Frame {
            at,
            duration,
            width,
            height,
            rgba,
        })
    }
}

impl FrameSource for MovieFile {
    fn next_frame(&mut self) -> Result<Option<Frame>, DecodeError> {
        let fail = |e: Error| DecodeError(e.to_string());
        loop {
            let mut decoded = frame::Video::empty();
            match self.decoder.receive_frame(&mut decoded) {
                Ok(()) => return self.convert(&decoded).map(Some).map_err(fail),
                // The decoder wants input. Once the end of the stream is
                // sent, it never does: it hands out what it holds, then
                // `Eof`.
                Err(Error::Other { errno: EAGAIN }) => {
                    self.feed().map_err(fail)?;
                }
                Err(Error::Eof) => return Ok(None),
                Err(e) => return Err(fail(e)),
            }
        }
    }

    fn duration(&self) -> Option<Duration> {
        self.duration
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }
}

/// Opens movie files with FFmpeg ([`MovieFile::open`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct FfmpegOpen;

impl Open for FfmpegOpen {
    type Source = MovieFile;

    fn open(&self, path: &Path) -> Option<MovieFile> {
        MovieFile::open(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_positive_length_is_known() {
        assert_eq!(known(5), Some(5));
        assert_eq!(known(0), None);
        assert_eq!(known(i64::MIN), None, "AV_NOPTS_VALUE");
    }

    #[test]
    fn ticks_become_a_duration_through_the_time_base() {
        let ms = Duration::from_millis(1);
        assert_eq!(to_duration(1024, Rational(1, 10_240)), ms * 100);
        assert_eq!(
            to_duration(3, Rational(1001, 30_000)),
            Duration::from_micros(100_100)
        );
        assert_eq!(to_duration(-5, Rational(1, 10)), Duration::ZERO, "clamped");
        assert_eq!(
            to_duration(5, Rational(1, 0)),
            Duration::ZERO,
            "no time base"
        );
    }
}
