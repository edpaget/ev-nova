//! Sounds as WAV files.
//!
//! A canonical 44-byte header (`RIFF`/`WAVE`, a `fmt ` chunk for 16-bit
//! linear PCM, then the `data` chunk) followed by the samples, little-endian
//! and interleaved as [`Pcm`] holds them.
//!
//! WAV stores a whole number of samples per second, so the header gets the
//! sound's 16.16 rate rounded to the nearest hertz
//! ([`SampleRate::nearest_hz`](nova_data::sound::SampleRate::nearest_hz)):
//! the classic 22254.5454 Hz becomes 22255 Hz, a pitch error under 0.003%.
//! A rate below 0.5 Hz would round to 0 Hz, an unplayable file, so it is
//! an error instead.
//! Loop points and the base note are not written.

use nova_data::sound::Pcm;

/// Bytes before the samples.
const HEADER_LEN: usize = 44;

/// Sample data longer than a WAV file's 32-bit sizes can describe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0} bytes of samples is too long for a WAV file")]
pub struct TooLong(pub u64);

/// A sound that cannot be written as a WAV file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WavError {
    /// The samples are too long.
    #[error(transparent)]
    TooLong(#[from] TooLong),
    /// The rate rounds to 0 Hz.
    #[error("sample rate {fixed:#010x} (16.16) is below 0.5 Hz, too low for a WAV file")]
    RateTooLow {
        /// The header's 16.16 fixed-point rate.
        fixed: u32,
    },
}

/// The sound as a WAV file.
pub fn encode(pcm: &Pcm) -> Result<Vec<u8>, WavError> {
    let rate = pcm.sample_rate();
    let hz = rate.nearest_hz();
    if hz == 0 {
        return Err(WavError::RateTooLow {
            fixed: rate.fixed(),
        });
    }
    Ok(encode_samples(pcm.channels(), hz, pcm.samples())?)
}

/// `samples`, interleaved over `channels`, as a WAV file at `rate` Hz.
pub fn encode_samples(channels: u16, rate: u32, samples: &[i16]) -> Result<Vec<u8>, TooLong> {
    let data_len = 2 * samples.len() as u64;
    let mut out = Vec::with_capacity(HEADER_LEN + 2 * samples.len());
    out.extend(header(channels, rate, data_len)?);
    for sample in samples {
        out.extend(sample.to_le_bytes());
    }
    Ok(out)
}

/// The 44-byte header for `data_len` bytes of 16-bit samples.
fn header(channels: u16, rate: u32, data_len: u64) -> Result<[u8; HEADER_LEN], TooLong> {
    let too_long = TooLong(data_len);
    let data_size = u32::try_from(data_len).map_err(|_| too_long)?;
    let riff_size = data_size.checked_add(36).ok_or(too_long)?;
    let block_align = channels * 2;
    let byte_rate = rate * u32::from(block_align);
    let mut out = [0; HEADER_LEN];
    let fields: [&[u8]; 12] = [
        b"RIFF",
        &riff_size.to_le_bytes(),
        b"WAVEfmt ",
        &16_u32.to_le_bytes(),
        &1_u16.to_le_bytes(),
        &channels.to_le_bytes(),
        &rate.to_le_bytes(),
        &byte_rate.to_le_bytes(),
        &block_align.to_le_bytes(),
        &16_u16.to_le_bytes(),
        b"data",
        &data_size.to_le_bytes(),
    ];
    let mut at = 0;
    for field in fields {
        out[at..at + field.len()].copy_from_slice(field);
        at += field.len();
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use nova_data::sound::decode_snd;
    use nova_data::sound::fixture::{Header, SndBuilder, SndFormat};

    use super::*;

    fn u16_at(bytes: &[u8], at: usize) -> u16 {
        u16::from_le_bytes([bytes[at], bytes[at + 1]])
    }

    fn u32_at(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes"))
    }

    #[test]
    fn mono_samples_get_a_canonical_header() {
        let wav = encode_samples(1, 11_127, &[0, 1, -1]).expect("small");
        #[rustfmt::skip]
        let expected: [u8; 50] = [
            b'R', b'I', b'F', b'F', 42, 0, 0, 0, b'W', b'A', b'V', b'E',
            b'f', b'm', b't', b' ', 16, 0, 0, 0,
            1, 0,                   // PCM
            1, 0,                   // channels
            0x77, 0x2B, 0, 0,       // 11127 Hz
            0xEE, 0x56, 0, 0,       // 22254 bytes per second
            2, 0,                   // block align
            16, 0,                  // bits per sample
            b'd', b'a', b't', b'a', 6, 0, 0, 0,
            0, 0, 1, 0, 0xFF, 0xFF, // little-endian samples
        ];
        assert_eq!(wav, expected);
    }

    #[test]
    fn stereo_samples_double_the_block_and_byte_rate() {
        let wav = encode_samples(2, 22_050, &[1, 2, 3, 4]).expect("small");
        assert_eq!(wav.len(), 44 + 8);
        assert_eq!(u32_at(&wav, 4), 36 + 8);
        assert_eq!(u16_at(&wav, 22), 2);
        assert_eq!(u32_at(&wav, 24), 22_050);
        assert_eq!(u32_at(&wav, 28), 22_050 * 4);
        assert_eq!(u16_at(&wav, 32), 4);
        assert_eq!(u16_at(&wav, 34), 16);
        assert_eq!(u32_at(&wav, 40), 8);
        assert_eq!(&wav[44..], [1, 0, 2, 0, 3, 0, 4, 0]);
    }

    #[test]
    fn a_fractional_rate_is_written_to_the_nearest_hertz() {
        let snd = SndBuilder::new(
            SndFormat::Two,
            Header::Standard {
                rate: 0x56EE_8BA3, // 22254.5454 Hz
                loop_points: (0, 0),
                base_note: 60,
                samples: vec![0x80, 0xFF],
            },
        );
        let pcm = decode_snd(&snd.bytes()).expect("decodes");
        let wav = encode(&pcm).expect("small");
        assert_eq!(u32_at(&wav, 24), 22_255);
        assert_eq!(u32_at(&wav, 28), 22_255 * 2);
        assert_eq!(&wav[44..], [0, 0, 0, 0x7F]);
    }

    fn pcm_at(rate: u32) -> Pcm {
        let snd = SndBuilder::new(
            SndFormat::Two,
            Header::Standard {
                rate,
                loop_points: (0, 0),
                base_note: 60,
                samples: vec![0x80],
            },
        );
        decode_snd(&snd.bytes()).expect("decodes")
    }

    #[test]
    fn a_rate_that_rounds_to_zero_hertz_is_an_error() {
        for fixed in [1, 0x7FFF] {
            let error = encode(&pcm_at(fixed)).expect_err("0 Hz is unplayable");
            assert_eq!(error, WavError::RateTooLow { fixed });
            assert_eq!(
                error.to_string(),
                format!(
                    "sample rate {fixed:#010x} (16.16) is below 0.5 Hz, too low for a WAV file"
                )
            );
        }
        let slowest = encode(&pcm_at(0x8000)).expect("rounds to 1 Hz");
        assert_eq!(u32_at(&slowest, 24), 1);
        assert_eq!(u32_at(&slowest, 28), 2);
    }

    #[test]
    fn data_too_long_for_a_riff_size_is_an_error() {
        let largest = u64::from(u32::MAX) - 36;
        let fits = header(1, 8000, largest).expect("fits");
        assert_eq!(u32_at(&fits, 4), u32::MAX);
        assert_eq!(u32_at(&fits, 40), u32::MAX - 36);
        assert_eq!(header(1, 8000, largest + 1), Err(TooLong(largest + 1)));
        assert_eq!(
            TooLong(5).to_string(),
            "5 bytes of samples is too long for a WAV file"
        );
    }
}
