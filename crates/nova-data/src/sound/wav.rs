//! The WAV decoder: a RIFF/WAVE file of integer PCM in, the same [`Pcm`]
//! [`decode_snd`](super::decode_snd) gives out.
//!
//! A new game's sounds are `.wav` files (`nova_galaxy_format::sound`);
//! the stock game's are `snd ` resources. Both reach the audio side as one
//! [`Pcm`], so it plays either the same way.
//!
//! # Supported
//!
//! - A `RIFF` file of form `WAVE`, its chunks read in order: the first
//!   `fmt ` and the first `data`, in either order. Every other chunk is
//!   skipped, an odd-sized one with its pad byte. The `RIFF` size is not
//!   checked; bytes after the two chunks are ignored.
//! - A `fmt ` chunk of at least 16 bytes, format tag 1 (integer PCM),
//!   1 or 2 channels, a sample rate of 1 to 65535 Hz (a [`SampleRate`] is
//!   16.16), and 8-bit or 16-bit samples. Its byte rate and block align are
//!   not checked: the frame is the channels times the sample size.
//! - 8-bit samples are unsigned with silence at `0x80`, and become
//!   `(byte - 128) << 8`, as `decode_snd` reads 8-bit; 16-bit samples are
//!   little-endian two's complement and are kept as they are.
//!
//! Everything else is a [`WavError`] naming it: a file that isn't
//! `RIFF`/`WAVE`, one with no `fmt ` or no `data`, a short `fmt `, another
//! format tag (IEEE float, A-law, mu-law, `WAVE_FORMAT_EXTENSIBLE`), another
//! sample size or channel count, a rate out of range, a chunk claiming more
//! bytes than remain, or data that ends part-way through a frame.
//!
//! # Bounded work
//!
//! A chunk's size is checked against the bytes left before it is read, so
//! output is at most one sample per input byte and no input can hang the
//! decoder.
//!
//! # Sources
//!
//! The RIFF chunk layout, the `WAVE` form and the `fmt ` chunk's
//! `WAVEFORMAT`/`PCMWAVEFORMAT` fields: Microsoft and IBM, *Multimedia
//! Programming Interface and Data Specifications 1.0* (1991), "Waveform
//! Audio File Format". No code was copied.

use super::pcm::{Pcm, SampleRate};

/// Why a WAV file failed to decode. Offsets are bytes from the start of
/// the file.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WavError {
    /// It doesn't start with `RIFF` and the form `WAVE`.
    #[error("it isn't a RIFF WAVE file")]
    NotWave,
    /// A chunk whose header or body runs past the end of the file.
    #[error("the chunk at byte {offset:#x} needs {needed} bytes, only {available} remain")]
    Truncated {
        /// Where the chunk starts.
        offset: usize,
        /// The bytes its header and its size need.
        needed: u64,
        /// The bytes left from `offset`.
        available: usize,
    },
    /// No `fmt ` chunk.
    #[error("it has no `fmt ` chunk")]
    NoFormat,
    /// No `data` chunk.
    #[error("it has no `data` chunk")]
    NoData,
    /// A `fmt ` chunk too short to hold the PCM fields.
    #[error("its `fmt ` chunk is {len} bytes, under the 16 PCM needs")]
    ShortFormat {
        /// The chunk's size.
        len: usize,
    },
    /// A format tag other than 1, integer PCM.
    #[error("unsupported WAV format tag {tag:#06x}: only integer PCM (1) is read")]
    UnsupportedFormat {
        /// The `fmt ` chunk's format tag.
        tag: u16,
    },
    /// A channel count other than 1 or 2.
    #[error("unsupported channel count {channels}")]
    UnsupportedChannels {
        /// The `fmt ` chunk's channel count.
        channels: u16,
    },
    /// A sample size other than 8 or 16 bits.
    #[error("unsupported sample size of {bits} bits")]
    UnsupportedSampleSize {
        /// The `fmt ` chunk's bits per sample.
        bits: u16,
    },
    /// A sample rate of 0, or above 65535 Hz.
    #[error("the sample rate {rate} Hz is outside 1 to 65535")]
    BadSampleRate {
        /// The `fmt ` chunk's samples per second.
        rate: u32,
    },
    /// Sample data that ends part-way through a frame.
    #[error("the {len} bytes of sample data aren't a whole number of {frame}-byte frames")]
    PartialFrame {
        /// The `data` chunk's size.
        len: usize,
        /// The bytes in one frame.
        frame: usize,
    },
}

/// The format tag of integer PCM.
const PCM: u16 = 1;

/// The bytes of a chunk's header: its id and its little-endian size.
const CHUNK_HEADER: usize = 8;

/// The bytes before the first chunk: `RIFF`, the file's size and `WAVE`.
const RIFF_HEADER: usize = 12;

/// Decodes one WAV file's bytes (see the module docs).
///
/// # Errors
///
/// [`WavError`] naming what it doesn't read.
pub fn decode_wav(data: &[u8]) -> Result<Pcm, WavError> {
    if data.len() < RIFF_HEADER || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Err(WavError::NotWave);
    }
    let (mut format, mut samples) = (None, None);
    let mut at = RIFF_HEADER;
    while at < data.len() && (format.is_none() || samples.is_none()) {
        let (id, body) = chunk(data, at)?;
        match id {
            b"fmt " if format.is_none() => format = Some(body),
            b"data" if samples.is_none() => samples = Some(body),
            _ => {}
        }
        // A chunk's body is padded to an even length; a missing pad byte
        // at the very end is no matter.
        at += CHUNK_HEADER + body.len() + body.len() % 2;
    }
    let format = Format::read(format.ok_or(WavError::NoFormat)?)?;
    let samples = samples.ok_or(WavError::NoData)?;
    let frame = usize::from(format.channels) * usize::from(format.bits / 8);
    if !samples.len().is_multiple_of(frame) {
        return Err(WavError::PartialFrame {
            len: samples.len(),
            frame,
        });
    }
    let samples = match format.bits {
        8 => samples
            .iter()
            .map(|&byte| (i16::from(byte) - 128) << 8)
            .collect(),
        _ => samples
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&pair| i16::from_le_bytes(pair))
            .collect(),
    };
    Ok(Pcm::new(
        SampleRate::from_fixed(format.rate << 16),
        format.channels,
        samples,
        (0, 0),
        60,
    ))
}

/// The id and body of the chunk at `at`.
fn chunk(data: &[u8], at: usize) -> Result<(&[u8; 4], &[u8]), WavError> {
    let available = data.len() - at;
    let truncated = |needed: u64| WavError::Truncated {
        offset: at,
        needed,
        available,
    };
    let Some(header) = data.get(at..at + CHUNK_HEADER) else {
        return Err(truncated(CHUNK_HEADER as u64));
    };
    let (id, size) = header.split_at(4);
    let id: &[u8; 4] = id.try_into().expect("four bytes");
    let size = u32::from_le_bytes(size.try_into().expect("four bytes"));
    let needed = CHUNK_HEADER as u64 + u64::from(size);
    let start = at + CHUNK_HEADER;
    let body = usize::try_from(size)
        .ok()
        .and_then(|size| data.get(start..start.checked_add(size)?))
        .ok_or_else(|| truncated(needed))?;
    Ok((id, body))
}

/// What a `fmt ` chunk says, once checked.
struct Format {
    channels: u16,
    rate: u32,
    bits: u16,
}

impl Format {
    /// Reads and checks the `fmt ` chunk `body`.
    fn read(body: &[u8]) -> Result<Self, WavError> {
        let Some(fields) = body.get(..16) else {
            return Err(WavError::ShortFormat { len: body.len() });
        };
        let word = |at: usize| u16::from_le_bytes([fields[at], fields[at + 1]]);
        let tag = word(0);
        if tag != PCM {
            return Err(WavError::UnsupportedFormat { tag });
        }
        let channels = word(2);
        if !matches!(channels, 1 | 2) {
            return Err(WavError::UnsupportedChannels { channels });
        }
        let rate = u32::from_le_bytes([fields[4], fields[5], fields[6], fields[7]]);
        if !(1..=0xFFFF).contains(&rate) {
            return Err(WavError::BadSampleRate { rate });
        }
        let bits = word(14);
        if !matches!(bits, 8 | 16) {
            return Err(WavError::UnsupportedSampleSize { bits });
        }
        Ok(Self {
            channels,
            rate,
            bits,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixture::wav;
    use super::*;

    /// A `fmt ` chunk's body: `tag`, `channels`, `rate`, `bits`, with a
    /// byte rate and block align worked from them.
    fn fmt_body(tag: u16, channels: u16, rate: u32, bits: u16) -> Vec<u8> {
        let align = channels * (bits / 8);
        let mut body = Vec::new();
        body.extend(tag.to_le_bytes());
        body.extend(channels.to_le_bytes());
        body.extend(rate.to_le_bytes());
        body.extend((rate * u32::from(align)).to_le_bytes());
        body.extend(align.to_le_bytes());
        body.extend(bits.to_le_bytes());
        body
    }

    /// A chunk of `id` holding `body`, padded to an even length.
    fn chunk_of(id: &[u8], body: &[u8]) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend(u32::try_from(body.len()).unwrap().to_le_bytes());
        out.extend(body);
        if body.len() % 2 == 1 {
            out.push(0);
        }
        out
    }

    /// A RIFF WAVE file of `chunks`.
    fn riff(chunks: &[Vec<u8>]) -> Vec<u8> {
        let body: Vec<u8> = chunks.concat();
        let mut out = b"RIFF".to_vec();
        out.extend(u32::try_from(body.len() + 4).unwrap().to_le_bytes());
        out.extend(b"WAVE");
        out.extend(body);
        out
    }

    fn le16(samples: &[i16]) -> Vec<u8> {
        samples.iter().flat_map(|s| s.to_le_bytes()).collect()
    }

    #[test]
    fn sixteen_bit_mono_keeps_its_samples_and_rate() {
        let file = riff(&[
            chunk_of(b"fmt ", &fmt_body(1, 1, 22_050, 16)),
            chunk_of(b"data", &le16(&[0, i16::MAX, i16::MIN, -2])),
        ]);
        let pcm = decode_wav(&file).expect("decodes");
        assert_eq!(pcm.channels(), 1);
        assert_eq!(pcm.sample_rate().nearest_hz(), 22_050);
        assert_eq!(pcm.sample_rate().fixed(), 22_050 << 16);
        assert_eq!(pcm.samples(), [0, i16::MAX, i16::MIN, -2]);
        assert_eq!(pcm.frames(), 4);
        assert_eq!(pcm.loop_points(), None);
        assert_eq!(pcm.base_note(), 60);
    }

    #[test]
    fn sixteen_bit_stereo_stays_interleaved() {
        let file = riff(&[
            chunk_of(b"fmt ", &fmt_body(1, 2, 44_100, 16)),
            chunk_of(b"data", &le16(&[1, -1, 300, -300])),
        ]);
        let pcm = decode_wav(&file).expect("decodes");
        assert_eq!(pcm.channels(), 2);
        assert_eq!(pcm.sample_rate().nearest_hz(), 44_100);
        assert_eq!(pcm.samples(), [1, -1, 300, -300]);
        assert_eq!(pcm.frames(), 2);
    }

    #[test]
    fn eight_bit_samples_are_offset_from_0x80_as_a_snd_s_are() {
        let file = riff(&[
            chunk_of(b"fmt ", &fmt_body(1, 1, 11_025, 8)),
            chunk_of(b"data", &[0x80, 0x00, 0xFF, 0xC0]),
        ]);
        let pcm = decode_wav(&file).expect("decodes");
        assert_eq!(pcm.samples(), [0, -32768, 32512, 16384]);
    }

    #[test]
    fn other_chunks_are_skipped_an_odd_one_with_its_pad_byte() {
        let file = riff(&[
            chunk_of(b"LIST", b"odd"),
            chunk_of(b"fmt ", &fmt_body(1, 1, 8000, 16)),
            chunk_of(b"fact", &[1, 0, 0, 0]),
            chunk_of(b"data", &le16(&[7, 8])),
            chunk_of(b"data", &le16(&[9])),
        ]);
        assert_eq!(file.len() % 2, 0, "padded");
        let pcm = decode_wav(&file).expect("decodes");
        assert_eq!(pcm.samples(), [7, 8], "the first data chunk");
    }

    #[test]
    fn the_data_may_come_before_the_format() {
        let file = riff(&[
            chunk_of(b"data", &le16(&[5])),
            chunk_of(b"fmt ", &fmt_body(1, 1, 8000, 16)),
        ]);
        assert_eq!(decode_wav(&file).expect("decodes").samples(), [5]);
    }

    #[test]
    fn bytes_after_both_chunks_are_ignored() {
        let mut file = riff(&[
            chunk_of(b"fmt ", &fmt_body(1, 1, 8000, 16)),
            chunk_of(b"data", &le16(&[5])),
        ]);
        file.extend(b"junk");
        assert_eq!(decode_wav(&file).expect("decodes").samples(), [5]);
    }

    #[test]
    fn an_odd_last_chunk_may_lack_its_pad_byte() {
        let mut file = riff(&[
            chunk_of(b"fmt ", &fmt_body(1, 1, 8000, 8)),
            chunk_of(b"data", &[0x80, 0x81, 0x82]),
        ]);
        file.pop();
        assert_eq!(decode_wav(&file).expect("decodes").frames(), 3);
    }

    #[test]
    fn an_empty_data_chunk_is_no_samples() {
        let file = riff(&[
            chunk_of(b"fmt ", &fmt_body(1, 2, 8000, 16)),
            chunk_of(b"data", &[]),
        ]);
        assert_eq!(decode_wav(&file).expect("decodes").frames(), 0);
    }

    #[test]
    fn a_file_that_is_not_riff_wave_is_an_error() {
        let good = riff(&[
            chunk_of(b"fmt ", &fmt_body(1, 1, 8000, 16)),
            chunk_of(b"data", &[]),
        ]);
        let mut riff_avi = good.clone();
        riff_avi[8..12].copy_from_slice(b"AVI ");
        let mut rifx = good.clone();
        rifx[0..4].copy_from_slice(b"RIFX");
        for bad in [&b""[..], b"RIFF", b"RIFF\0\0\0\0WAV", &riff_avi, &rifx] {
            assert_eq!(decode_wav(bad), Err(WavError::NotWave), "{bad:?}");
        }
        let snd = super::super::fixture::SndBuilder::new(
            super::super::fixture::SndFormat::Two,
            super::super::fixture::Header::Standard {
                rate: 0x2B77_0000,
                loop_points: (0, 0),
                base_note: 60,
                samples: vec![0x80],
            },
        )
        .bytes();
        assert_eq!(decode_wav(&snd), Err(WavError::NotWave), "a snd");
    }

    #[test]
    fn a_missing_chunk_is_named() {
        let no_data = riff(&[chunk_of(b"fmt ", &fmt_body(1, 1, 8000, 16))]);
        assert_eq!(decode_wav(&no_data), Err(WavError::NoData));
        let no_format = riff(&[chunk_of(b"data", &le16(&[1]))]);
        assert_eq!(decode_wav(&no_format), Err(WavError::NoFormat));
        assert_eq!(decode_wav(&riff(&[])), Err(WavError::NoFormat));
    }

    #[test]
    fn a_chunk_past_the_end_is_truncated_at_its_offset() {
        let mut file = riff(&[
            chunk_of(b"fmt ", &fmt_body(1, 1, 8000, 16)),
            chunk_of(b"data", &le16(&[1, 2, 3])),
        ]);
        file.truncate(file.len() - 1);
        let data_at = RIFF_HEADER + CHUNK_HEADER + 16;
        assert_eq!(
            decode_wav(&file),
            Err(WavError::Truncated {
                offset: data_at,
                needed: 14,
                available: 13,
            })
        );
        let mut header_cut = riff(&[chunk_of(b"fmt ", &fmt_body(1, 1, 8000, 16))]);
        header_cut.extend(b"dat");
        assert_eq!(
            decode_wav(&header_cut),
            Err(WavError::Truncated {
                offset: data_at,
                needed: 8,
                available: 3,
            })
        );
        let mut huge = riff(&[chunk_of(b"fmt ", &fmt_body(1, 1, 8000, 16))]);
        huge.extend(b"data");
        huge.extend(u32::MAX.to_le_bytes());
        assert_eq!(
            decode_wav(&huge),
            Err(WavError::Truncated {
                offset: data_at,
                needed: 8 + u64::from(u32::MAX),
                available: 8,
            })
        );
    }

    #[test]
    fn each_unsupported_format_is_named() {
        let with = |format: Vec<u8>| {
            decode_wav(&riff(&[
                chunk_of(b"fmt ", &format),
                chunk_of(b"data", &[0; 4]),
            ]))
        };
        assert_eq!(
            with(fmt_body(3, 1, 8000, 32)),
            Err(WavError::UnsupportedFormat { tag: 3 }),
            "IEEE float"
        );
        assert_eq!(
            with(fmt_body(0xFFFE, 1, 8000, 16)),
            Err(WavError::UnsupportedFormat { tag: 0xFFFE }),
            "extensible"
        );
        assert_eq!(
            with(fmt_body(1, 3, 8000, 16)),
            Err(WavError::UnsupportedChannels { channels: 3 })
        );
        assert_eq!(
            with(fmt_body(1, 0, 8000, 16)),
            Err(WavError::UnsupportedChannels { channels: 0 })
        );
        assert_eq!(
            with(fmt_body(1, 1, 8000, 24)),
            Err(WavError::UnsupportedSampleSize { bits: 24 })
        );
        assert_eq!(
            with(fmt_body(1, 1, 0, 16)),
            Err(WavError::BadSampleRate { rate: 0 })
        );
        assert_eq!(
            with(fmt_body(1, 1, 65_536, 16)),
            Err(WavError::BadSampleRate { rate: 65_536 })
        );
        assert!(with(fmt_body(1, 1, 65_535, 16)).is_ok(), "the highest");
        assert!(with(fmt_body(1, 1, 1, 16)).is_ok(), "the lowest");
        assert_eq!(
            with(fmt_body(1, 1, 8000, 16)[..14].to_vec()),
            Err(WavError::ShortFormat { len: 14 })
        );
    }

    #[test]
    fn data_ending_part_way_through_a_frame_is_an_error() {
        let file = riff(&[
            chunk_of(b"fmt ", &fmt_body(1, 2, 8000, 16)),
            chunk_of(b"data", &[0; 6]),
        ]);
        assert_eq!(
            decode_wav(&file),
            Err(WavError::PartialFrame { len: 6, frame: 4 })
        );
        let mono = riff(&[
            chunk_of(b"fmt ", &fmt_body(1, 1, 8000, 16)),
            chunk_of(b"data", &[0; 3]),
        ]);
        assert_eq!(
            decode_wav(&mono),
            Err(WavError::PartialFrame { len: 3, frame: 2 })
        );
    }

    #[test]
    fn each_error_says_what_is_wrong() {
        assert_eq!(WavError::NotWave.to_string(), "it isn't a RIFF WAVE file");
        assert_eq!(
            WavError::UnsupportedFormat { tag: 3 }.to_string(),
            "unsupported WAV format tag 0x0003: only integer PCM (1) is read"
        );
        assert_eq!(
            WavError::Truncated {
                offset: 36,
                needed: 14,
                available: 13
            }
            .to_string(),
            "the chunk at byte 0x24 needs 14 bytes, only 13 remain"
        );
    }

    #[test]
    fn the_fixture_builder_round_trips() {
        let mono = decode_wav(&wav(1, 22_050, 16, &[1, -2, 300])).expect("decodes");
        assert_eq!(
            (mono.channels(), mono.sample_rate().nearest_hz()),
            (1, 22_050)
        );
        assert_eq!(mono.samples(), [1, -2, 300]);
        let stereo = decode_wav(&wav(2, 8000, 16, &[1, 2, 3, 4])).expect("decodes");
        assert_eq!((stereo.channels(), stereo.frames()), (2, 2));
        let eight = decode_wav(&wav(1, 11_025, 8, &[0, -32768, 16384, 32767])).expect("decodes");
        assert_eq!(eight.samples(), [0, -32768, 16384, 32512], "the high byte");
        let odd = wav(1, 8000, 8, &[0, 0, 0]);
        assert_eq!(odd.len() % 2, 0, "padded");
        assert_eq!(decode_wav(&odd).expect("decodes").frames(), 3);
    }
}
