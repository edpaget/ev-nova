//! Sound decoder: `snd ` resources in, 16-bit PCM out.
//!
//! [`decode_snd`] is pure: it turns one `snd ` resource's bytes into a
//! [`Pcm`] and does no resource lookup or I/O. A [`Pcm`] holds the header's
//! exact sample rate ([`SampleRate`], 16.16 fixed point), the channel count,
//! and signed 16-bit linear samples interleaved by frame:
//!
//! - 8-bit samples (standard headers, and extended headers with
//!   `sampleSize` 8) are unsigned offset binary with silence at `0x80`, and
//!   become `(byte - 128) << 8`: `0x00` is -32768, `0x80` is 0, `0xFF` is
//!   32512.
//! - 16-bit samples (extended headers with `sampleSize` 16) are big-endian
//!   two's complement and are kept as they are.
//! - IMA4 decodes directly to 16-bit samples.
//!
//! # Example
//!
//! ```
//! use nova_data::sound::decode_snd;
//!
//! #[rustfmt::skip]
//! let snd = [
//!     0, 2, 0, 0,                          // format 2, refCount 0
//!     0, 1, 0x80, 0x51, 0, 0, 0, 0, 0, 14, // bufferCmd, header at 14
//!     0, 0, 0, 0,                          // samplePtr
//!     0, 0, 0, 3,                          // numBytes
//!     0x2B, 0x77, 0, 0,                    // sampleRate 11127.0
//!     0, 0, 0, 0, 0, 0, 0, 0,              // loopStart, loopEnd
//!     0x00, 60,                            // encode (standard), baseFrequency
//!     0x80, 0xFF, 0x00,                    // samples
//! ];
//! let pcm = decode_snd(&snd)?;
//! assert_eq!(pcm.sample_rate().nearest_hz(), 11_127);
//! assert_eq!(pcm.channels(), 1);
//! assert_eq!(pcm.samples(), [0, 32512, -32768]);
//!
//! // One sample short: the error names where the samples start.
//! let err = decode_snd(&snd[..snd.len() - 1]).unwrap_err();
//! assert_eq!(err.to_string(), "sample data at byte 0x24 needs 3 bytes, only 2 remain");
//! # Ok::<(), nova_data::sound::SoundError>(())
//! ```
//!
//! # Supported
//!
//! - `snd ` formats 1 and 2. Format 1's data-format list is skipped;
//!   format 2's reference count is ignored.
//! - The first `soundCmd` or `bufferCmd` in the command list, which must
//!   carry the data-offset flag (`0x8000`). Other commands are ignored.
//! - Standard headers: 8-bit mono.
//! - Extended headers: 8 or 16-bit, mono or stereo.
//! - Compressed headers: `ima4` with `compressionID` -1 or -2, mono or
//!   stereo (packets alternate left and right).
//!
//! Everything else is an error naming it ([`SoundError`]): other formats,
//! commands without the offset flag, other header encodings, sample sizes
//! and channel counts, and every other compression, MACE 3:1 and 6:1
//! included. Loop points are kept exactly as stored and never checked;
//! bytes after the sample data are ignored.
//!
//! # Sources
//!
//! - `snd ` formats 1 and 2, the command list, `soundCmd`/`bufferCmd` and
//!   the data-offset flag, the `SoundHeader`, `ExtSoundHeader` and
//!   `CmpSoundHeader` layouts and the compression IDs: Apple, *Inside
//!   Macintosh: Sound* (1994), ch. 2 "Sound Manager".
//! - IMA4 packets (34 bytes per channel: a 9-bit predictor and 7-bit step
//!   index, then 64 four-bit codes, low nibble first): Apple Technical Note
//!   TN1081 and the Sound Manager 3.2 IMA 4:1 description.
//! - The IMA ADPCM step and index tables and the decoding steps: the IMA
//!   Digital Audio Focus and Technical Working Group, "Recommended
//!   Practices for Enhancing Digital Audio Compatibility in Multimedia
//!   Systems" (1992). The IMA4 unit-test vectors are worked by hand from
//!   that algorithm.
//!
//! No code was copied.
//!
//! # Stock data
//!
//! All 227 stock `snd ` resources are in `Nova Sounds.ndat`, and the stock
//! test checks them:
//!
//! - 213 format 1 (one sampledSynth data format each) and 14 format 2.
//!   Each has one command, `0x8051`, pointing at the header right after
//!   the command list.
//! - 49 standard headers: 8-bit mono at about 11127 Hz (16.16 values
//!   `0x2B770000`, `0x2B7745D1` and `0x2B7745D0`). Nine have loop points,
//!   seven of them past the last sample.
//! - 178 compressed headers: `ima4`, `compressionID` -1, mono, 22050 Hz
//!   (one at 44100 Hz). No packet's step index exceeds 88.
//! - No extended headers, no other compressions, and no bytes after the
//!   sample data.
//!
//! # Bounded work
//!
//! The graphics decoders need a pixel budget because one small header can
//! claim a huge image. A sound does not: before allocating, every header's
//! claimed data length (`numBytes`; `numFrames * channels * sampleSize / 8`;
//! `numFrames * channels * 34`) is computed in 64-bit arithmetic and taken
//! from the input, so a claim larger than the data is
//! [`SoundError::SamplesTruncated`]. Output is then at most about 1.9
//! samples per input byte (IMA4's 64 per 34), which replaces a separate
//! budget. Every loop runs over an already-sliced input, so no input can
//! hang the decoder.
//!
//! # Dependencies
//!
//! This module depends only on `std` and `thiserror`: not on [`graphics`]
//! and not on the record side of the crate (`records`, `wire`, `decode`,
//! `error`, `file`).
//!
//! [`graphics`]: crate::graphics
//!
//! The `fixture` feature exposes the `fixture` module, builders for
//! synthetic `snd ` resources, to other crates' tests.

mod bytes;
mod error;
mod header;
mod ima4;
mod pcm;
mod snd;
#[cfg(test)]
mod sweep;

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

pub use error::SoundError;
pub use pcm::{Pcm, SampleRate};
pub use snd::decode_snd;
