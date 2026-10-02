//! Sound headers: standard, extended and compressed, to PCM.

use super::SoundError;
use super::bytes::{u8_at, u32_at};
use super::pcm::{Pcm, SampleRate};

/// `encode` of a standard header: 8-bit mono samples.
const STANDARD: u8 = 0x00;
/// Bytes of a standard header before its samples.
const STANDARD_LEN: usize = 22;

/// Decodes the sound header at `at` and the samples that follow it.
pub(super) fn decode_header(data: &[u8], at: usize) -> Result<Pcm, SoundError> {
    // samplePtr (+0) is ignored: in a resource the samples follow the header.
    let count = u32_at(data, at + 4)?;
    let rate = u32_at(data, at + 8)?;
    let loop_points = (u32_at(data, at + 12)?, u32_at(data, at + 16)?);
    let encode = u8_at(data, at + 20)?;
    let base_note = u8_at(data, at + 21)?;
    let (channels, samples) = match encode {
        STANDARD => {
            let samples = sample_data(data, at + STANDARD_LEN, u64::from(count))?;
            (1, samples.iter().map(|&b| offset_binary(b)).collect())
        }
        _ => return Err(SoundError::UnsupportedHeader { encode, offset: at }),
    };
    if rate == 0 {
        return Err(SoundError::BadSampleRate);
    }
    let rate = SampleRate::from_fixed(rate);
    Ok(Pcm::new(rate, channels, samples, loop_points, base_note))
}

/// The `needed` bytes of sample data at `offset`. The length is checked
/// before anything is allocated for it.
fn sample_data(data: &[u8], offset: usize, needed: u64) -> Result<&[u8], SoundError> {
    let available = data.len().saturating_sub(offset);
    usize::try_from(needed)
        .ok()
        .filter(|&len| len <= available)
        .and_then(|len| data.get(offset..offset + len))
        .ok_or(SoundError::SamplesTruncated {
            offset,
            needed,
            available,
        })
}

/// An unsigned 8-bit sample (silence at `0x80`) as a 16-bit one.
fn offset_binary(byte: u8) -> i16 {
    (i16::from(byte) - 128) << 8
}

#[cfg(test)]
mod tests {
    use super::super::decode_snd;
    use super::super::fixture::{Header, SndBuilder, SndFormat};
    use super::*;

    const RATE: u32 = 0x2B77_45D1;

    fn standard(samples: Vec<u8>) -> Header {
        Header::Standard {
            rate: RATE,
            loop_points: (0, 0),
            base_note: 60,
            samples,
        }
    }

    fn decode(header: Header) -> Result<Pcm, SoundError> {
        decode_snd(&SndBuilder::new(SndFormat::One, header).bytes())
    }

    #[test]
    fn standard_samples_map_from_offset_binary_to_16_bits() {
        let pcm = decode(standard(vec![0x00, 0x80, 0xFF, 0x7F, 0x81])).unwrap();
        assert_eq!(pcm.samples(), [-32768, 0, 32512, -256, 256]);
        assert_eq!(pcm.channels(), 1);
        assert_eq!(pcm.frames(), 5);
        assert_eq!(pcm.sample_rate(), SampleRate::from_fixed(RATE));
        assert_eq!(pcm.loop_points(), None);
        assert_eq!(pcm.base_note(), 60);
    }

    #[test]
    fn standard_loop_points_and_base_note_are_kept_raw() {
        // Loop points past the last sample are kept, as in stock sounds.
        let pcm = decode(Header::Standard {
            rate: RATE,
            loop_points: (1742, 1743),
            base_note: 72,
            samples: vec![0x80; 4],
        })
        .unwrap();
        assert_eq!(pcm.loop_points(), Some((1742, 1743)));
        assert_eq!(pcm.base_note(), 72);
    }

    #[test]
    fn format_2_decodes_the_same_samples() {
        let header = standard(vec![0x10, 0x90]);
        let one = decode(header.clone()).unwrap();
        let two = decode_snd(&SndBuilder::new(SndFormat::Two, header).bytes()).unwrap();
        assert_eq!(one, two);
        assert_eq!(two.samples(), [-28672, 4096]);
    }

    #[test]
    fn an_empty_standard_sound_decodes_to_no_samples() {
        assert_eq!(decode(standard(vec![])).unwrap().samples(), [0i16; 0]);
    }

    #[test]
    fn trailing_bytes_are_ignored() {
        let mut bytes = SndBuilder::new(SndFormat::One, standard(vec![0x80, 0x81])).bytes();
        bytes.extend([0xFF; 9]);
        assert_eq!(decode_snd(&bytes).unwrap().samples(), [0, 256]);
    }

    #[test]
    fn missing_standard_samples_are_truncated() {
        let bytes = SndBuilder::new(SndFormat::One, standard(vec![0x80; 5])).bytes();
        assert_eq!(bytes.len(), 20 + 22 + 5);
        assert_eq!(
            decode_snd(&bytes[..45]),
            Err(SoundError::SamplesTruncated {
                offset: 42,
                needed: 5,
                available: 3
            })
        );
        assert_eq!(
            decode_snd(&bytes[..42]),
            Err(SoundError::SamplesTruncated {
                offset: 42,
                needed: 5,
                available: 0
            })
        );
    }

    #[test]
    fn a_truncated_standard_header_ends_at_its_field() {
        let bytes = SndBuilder::new(SndFormat::One, standard(vec![])).bytes();
        let end = |len: usize| decode_snd(&bytes[..len]).unwrap_err();
        // samplePtr (20..24) is never read.
        assert_eq!(end(21), SoundError::UnexpectedEnd { offset: 24 });
        assert_eq!(end(27), SoundError::UnexpectedEnd { offset: 24 });
        assert_eq!(end(31), SoundError::UnexpectedEnd { offset: 28 });
        assert_eq!(end(35), SoundError::UnexpectedEnd { offset: 32 });
        assert_eq!(end(39), SoundError::UnexpectedEnd { offset: 36 });
        assert_eq!(end(40), SoundError::UnexpectedEnd { offset: 40 });
        assert_eq!(end(41), SoundError::UnexpectedEnd { offset: 41 });
    }

    #[test]
    fn a_zero_rate_is_bad() {
        let header = Header::Standard {
            rate: 0,
            loop_points: (0, 0),
            base_note: 60,
            samples: vec![0x80],
        };
        assert_eq!(decode(header), Err(SoundError::BadSampleRate));
        let tiny = Header::Standard {
            rate: 1,
            loop_points: (0, 0),
            base_note: 60,
            samples: vec![0x80],
        };
        assert_eq!(decode(tiny).unwrap().sample_rate().fixed(), 1);
    }

    #[test]
    fn an_unknown_encoding_names_itself_and_the_header() {
        let mut bytes = SndBuilder::new(SndFormat::One, standard(vec![0x80])).bytes();
        for encode in [0x01, 0x02, 0xFD] {
            bytes[20 + 20] = encode;
            assert_eq!(
                decode_snd(&bytes),
                Err(SoundError::UnsupportedHeader { encode, offset: 20 })
            );
        }
    }
}
