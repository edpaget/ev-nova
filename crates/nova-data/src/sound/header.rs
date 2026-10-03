//! Sound headers: standard, extended and compressed, to PCM.

use super::SoundError;
use super::bytes::{array_at, i16_at, u8_at, u16_at, u32_at};
use super::ima4::{Channel, PACKET_LEN, PACKET_SAMPLES};
use super::pcm::{Pcm, SampleRate};

/// `encode` of a standard header: 8-bit mono samples.
const STANDARD: u8 = 0x00;
/// Bytes of a standard header before its samples.
const STANDARD_LEN: usize = 22;
/// `encode` of an extended header: 8 or 16-bit samples, 1 or 2 channels.
const EXTENDED: u8 = 0xFF;
/// `encode` of a compressed header.
const COMPRESSED: u8 = 0xFE;
/// `compressionID` values that mean "see the `format` field".
const FIXED_COMPRESSION: i16 = -1;
const VARIABLE_COMPRESSION: i16 = -2;
/// Bytes of an extended or compressed header before its samples.
const LONG_HEADER_LEN: usize = 64;

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
        EXTENDED => {
            let channels = channel_count(count)?;
            let frames = u32_at(data, at + 22)?;
            let bits = u16_at(data, at + 48)?;
            if bits != 8 && bits != 16 {
                return Err(SoundError::UnsupportedSampleSize { bits });
            }
            let needed = u64::from(frames) * u64::from(channels) * u64::from(bits / 8);
            let bytes = sample_data(data, at + LONG_HEADER_LEN, needed)?;
            let samples = if bits == 8 {
                bytes.iter().map(|&b| offset_binary(b)).collect()
            } else {
                let (pairs, _) = bytes.as_chunks::<2>();
                pairs.iter().map(|&pair| i16::from_be_bytes(pair)).collect()
            };
            (channels, samples)
        }
        COMPRESSED => {
            let packets = u32_at(data, at + 22)?;
            let format = array_at::<4>(data, at + 40)?;
            let compression_id = i16_at(data, at + 56)?;
            if &format != b"ima4"
                || !matches!(compression_id, FIXED_COMPRESSION | VARIABLE_COMPRESSION)
            {
                return Err(SoundError::UnsupportedCompression {
                    format,
                    compression_id,
                });
            }
            let channels = channel_count(count)?;
            let needed = u64::from(packets) * u64::from(channels) * PACKET_LEN as u64;
            let bytes = sample_data(data, at + LONG_HEADER_LEN, needed)?;
            (channels, decode_ima4(bytes, channels))
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

/// IMA4 packets, `channels` (1 or 2) per frame of 64, to interleaved
/// samples. Each channel's packets go through that channel's decoder.
fn decode_ima4(bytes: &[u8], channels: u16) -> Vec<i16> {
    let channels = usize::from(channels);
    let (packets, _) = bytes.as_chunks::<PACKET_LEN>();
    let mut samples = vec![0; packets.len() * PACKET_SAMPLES];
    let mut decoders = [Channel::default(); 2];
    let frames = packets
        .chunks_exact(channels)
        .zip(samples.chunks_exact_mut(PACKET_SAMPLES * channels));
    for (frame_packets, out) in frames {
        let channel_packets = frame_packets.iter().zip(&mut decoders).enumerate();
        for (channel, (packet, decoder)) in channel_packets {
            let decoded = decoder.decode(packet);
            for (slot, sample) in out.iter_mut().skip(channel).step_by(channels).zip(decoded) {
                *slot = sample;
            }
        }
    }
    samples
}

/// `numChannels` of an extended or compressed header, if 1 or 2.
fn channel_count(channels: u32) -> Result<u16, SoundError> {
    match channels {
        1 | 2 => Ok(channels as u16),
        _ => Err(SoundError::UnsupportedChannels { channels }),
    }
}

/// An unsigned 8-bit sample (silence at `0x80`) as a 16-bit one:
/// `(byte - 128) << 8`, written as flipping the sign bit into the high
/// byte. (`(byte + 128) << 8` wraps to the same `i16`, so the subtraction
/// form would hide a sign mistake from the tests.)
fn offset_binary(byte: u8) -> i16 {
    i16::from_be_bytes([byte ^ 0x80, 0])
}

#[cfg(test)]
mod standard_tests {
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

#[cfg(test)]
mod extended_tests {
    use super::super::decode_snd;
    use super::super::fixture::{Header, SndBuilder, SndFormat};
    use super::*;

    const RATE: u32 = 0xAC44_0000;

    fn extended(channels: u32, sample_size: u16, data: Vec<u8>) -> Header {
        Header::Extended {
            channels,
            rate: RATE,
            sample_size,
            data,
        }
    }

    fn bytes(header: Header) -> Vec<u8> {
        SndBuilder::new(SndFormat::Two, header).bytes()
    }

    fn decode(header: Header) -> Result<Pcm, SoundError> {
        decode_snd(&bytes(header))
    }

    #[test]
    fn eight_bit_extended_samples_are_offset_binary() {
        let pcm = decode(extended(1, 8, vec![0x00, 0x80, 0xFF])).unwrap();
        assert_eq!(pcm.samples(), [-32768, 0, 32512]);
        assert_eq!((pcm.channels(), pcm.frames()), (1, 3));
        assert_eq!(pcm.sample_rate(), SampleRate::from_fixed(RATE));
        assert_eq!(pcm.base_note(), 60);
        assert_eq!(pcm.loop_points(), None);
    }

    #[test]
    fn eight_bit_stereo_stays_interleaved() {
        let pcm = decode(extended(2, 8, vec![0x00, 0xFF, 0x80, 0x81])).unwrap();
        assert_eq!(pcm.samples(), [-32768, 32512, 0, 256]);
        assert_eq!((pcm.channels(), pcm.frames()), (2, 2));
    }

    #[test]
    fn sixteen_bit_samples_are_big_endian_twos_complement() {
        let pcm = decode(extended(1, 16, vec![0x12, 0x34, 0xFF, 0xFE, 0x80, 0x00])).unwrap();
        assert_eq!(pcm.samples(), [0x1234, -2, -32768]);
        assert_eq!((pcm.channels(), pcm.frames()), (1, 3));
        let stereo = decode(extended(2, 16, vec![0, 1, 0, 2, 0, 3, 0, 4])).unwrap();
        assert_eq!(stereo.samples(), [1, 2, 3, 4]);
        assert_eq!((stereo.channels(), stereo.frames()), (2, 2));
    }

    #[test]
    fn extended_loop_points_are_kept() {
        let mut bytes = bytes(extended(1, 8, vec![0x80; 4]));
        bytes[14 + 15] = 1;
        bytes[14 + 19] = 3;
        assert_eq!(decode_snd(&bytes).unwrap().loop_points(), Some((1, 3)));
    }

    #[test]
    fn other_sample_sizes_are_unsupported() {
        for bits in [0, 4, 12, 24, 32] {
            assert_eq!(
                decode(extended(1, bits, vec![0; 8])),
                Err(SoundError::UnsupportedSampleSize { bits }),
                "{bits}"
            );
        }
    }

    #[test]
    fn only_mono_and_stereo_are_supported() {
        for channels in [0, 3, 0x1_0001, u32::MAX] {
            assert_eq!(
                decode(extended(channels, 8, vec![0; 6])),
                Err(SoundError::UnsupportedChannels { channels }),
                "{channels}"
            );
        }
    }

    #[test]
    fn missing_extended_samples_are_truncated() {
        // Stereo 16-bit: 2 frames of 4 bytes. Claim 3 frames.
        let mut bytes = bytes(extended(2, 16, vec![0; 8]));
        bytes[14 + 25] = 3;
        assert_eq!(
            decode_snd(&bytes),
            Err(SoundError::SamplesTruncated {
                offset: 14 + 64,
                needed: 12,
                available: 8
            })
        );
        let mut eight = self::bytes(extended(2, 8, vec![0; 4]));
        eight[14 + 25] = 3;
        assert_eq!(
            decode_snd(&eight),
            Err(SoundError::SamplesTruncated {
                offset: 14 + 64,
                needed: 6,
                available: 4
            })
        );
    }

    #[test]
    fn extended_trailing_bytes_are_ignored() {
        let mut bytes = bytes(extended(1, 16, vec![0, 7]));
        bytes.extend([0xAB; 3]);
        assert_eq!(decode_snd(&bytes).unwrap().samples(), [7]);
    }

    #[test]
    fn a_zero_extended_rate_is_bad() {
        let header = Header::Extended {
            channels: 1,
            rate: 0,
            sample_size: 8,
            data: vec![0x80],
        };
        assert_eq!(decode(header), Err(SoundError::BadSampleRate));
    }
}

#[cfg(test)]
mod compressed_tests {
    use super::super::decode_snd;
    use super::super::fixture::{Header, SndBuilder, SndFormat, ima4_packet};
    use super::*;

    const RATE: u32 = 0x5622_0000;

    /// A packet whose every sample is `level` (a multiple of 128).
    fn flat(level: i16) -> [u8; 34] {
        ima4_packet(level, 0, [0; 64])
    }

    fn compressed(channels: u32, packets: Vec<[u8; 34]>) -> Header {
        coded(channels, *b"ima4", -1, packets)
    }

    fn coded(
        channels: u32,
        format: [u8; 4],
        compression_id: i16,
        packets: Vec<[u8; 34]>,
    ) -> Header {
        Header::Compressed {
            channels,
            rate: RATE,
            format,
            compression_id,
            packets,
        }
    }

    fn bytes(header: Header) -> Vec<u8> {
        SndBuilder::new(SndFormat::One, header).bytes()
    }

    fn decode(header: Header) -> Result<Pcm, SoundError> {
        decode_snd(&bytes(header))
    }

    #[test]
    fn mono_ima4_decodes_64_samples_per_packet() {
        let mut codes = [0; 64];
        codes[0] = 4;
        let packets = vec![flat(256), ima4_packet(-512, 0, codes), flat(0)];
        let pcm = decode(compressed(1, packets)).unwrap();
        assert_eq!((pcm.channels(), pcm.frames()), (1, 192));
        assert_eq!(pcm.sample_rate(), SampleRate::from_fixed(RATE));
        assert_eq!(pcm.base_note(), 60);
        assert_eq!(pcm.loop_points(), None);
        assert_eq!(pcm.samples()[..64], [256; 64]);
        assert_eq!(pcm.samples()[64..66], [-505, -504]);
        assert_eq!(pcm.samples()[128..], [0; 64]);
    }

    #[test]
    fn stereo_packets_alternate_left_and_right() {
        let packets = vec![flat(128), flat(-128), flat(256), flat(-256)];
        let pcm = decode(compressed(2, packets)).unwrap();
        assert_eq!((pcm.channels(), pcm.frames()), (2, 128));
        let left: Vec<i16> = pcm.samples().iter().step_by(2).copied().collect();
        let right: Vec<i16> = pcm.samples().iter().skip(1).step_by(2).copied().collect();
        assert_eq!(left[..64], [128; 64]);
        assert_eq!(left[64..], [256; 64]);
        assert_eq!(right[..64], [-128; 64]);
        assert_eq!(right[64..], [-256; 64]);
    }

    #[test]
    fn each_channel_carries_its_own_state_between_packets() {
        // The left channel ends its first packet at predictor 1, index 0,
        // so its second packet (header 128, index 0) carries on from 1. A
        // state shared with the right channel (at -1280) would restart it
        // at 128.
        let mut codes = [0; 64];
        codes[1] = 4;
        codes[2] = 0xC;
        let packets = vec![
            ima4_packet(0, 0, codes),
            flat(-1280),
            flat(128),
            flat(-1280),
        ];
        let pcm = decode(compressed(2, packets)).unwrap();
        let left: Vec<i16> = pcm.samples().iter().step_by(2).copied().collect();
        assert_eq!(left[63..], [1; 65]);
        assert_eq!(pcm.samples()[1..2], [-1280]);
        assert_eq!(pcm.samples()[255], -1280);
    }

    #[test]
    fn variable_compression_ima4_decodes_too() {
        let pcm = decode(coded(1, *b"ima4", -2, vec![flat(384)])).unwrap();
        assert_eq!(pcm.samples(), [384; 64]);
    }

    #[test]
    fn other_compressions_are_unsupported_and_named() {
        let cases: [([u8; 4], i16, &str); 7] = [
            ([0; 4], 3, "MACE 3:1"),
            ([0; 4], 4, "MACE 6:1"),
            (*b"MAC3", -1, "'MAC3'"),
            (*b"MAC6", 4, "'MAC6'"),
            (*b"raw ", 0, "'raw '"),
            (*b"twos", 0, "'twos'"),
            (*b"ima4", 3, "'ima4' (compressionID 3)"),
        ];
        for (format, compression_id, name) in cases {
            let error = decode(coded(1, format, compression_id, vec![flat(0)])).unwrap_err();
            assert_eq!(
                error,
                SoundError::UnsupportedCompression {
                    format,
                    compression_id
                }
            );
            assert!(error.to_string().contains(name), "{error}");
        }
        for compression_id in [0, 1, -3] {
            assert!(decode(coded(1, *b"ima4", compression_id, vec![flat(0)])).is_err());
        }
    }

    #[test]
    fn compressed_channels_must_be_mono_or_stereo() {
        for channels in [0, 3] {
            assert_eq!(
                decode(compressed(channels, vec![flat(0); 3])),
                Err(SoundError::UnsupportedChannels { channels })
            );
        }
    }

    #[test]
    fn missing_packets_are_truncated() {
        // Stereo, 1 frame (2 packets) present; claim 2 frames.
        let mut bytes = bytes(compressed(2, vec![flat(0); 2]));
        bytes[20 + 25] = 2;
        assert_eq!(
            decode_snd(&bytes),
            Err(SoundError::SamplesTruncated {
                offset: 20 + 64,
                needed: 136,
                available: 68
            })
        );
    }

    #[test]
    fn compressed_trailing_bytes_are_ignored() {
        let mut bytes = bytes(compressed(1, vec![flat(128)]));
        bytes.extend([0x12; 33]);
        assert_eq!(decode_snd(&bytes).unwrap().samples(), [128; 64]);
    }

    #[test]
    fn a_zero_compressed_rate_is_bad() {
        let header = Header::Compressed {
            channels: 1,
            rate: 0,
            format: *b"ima4",
            compression_id: -1,
            packets: vec![flat(0)],
        };
        assert_eq!(decode(header), Err(SoundError::BadSampleRate));
    }
}

#[cfg(test)]
mod bounded_tests {
    use super::super::decode_snd;
    use super::super::fixture::{Header, SndBuilder, SndFormat, ima4_packet};
    use super::*;
    use crate::sweep::assert_never_panics;

    /// A resource of about 100 bytes whose header field at `field` (from
    /// the header start) is overwritten with `claim`.
    fn claiming(header: Header, field: usize, claim: u32) -> Vec<u8> {
        let mut bytes = SndBuilder::new(SndFormat::Two, header).bytes();
        bytes.resize(100, 0);
        bytes[14 + field..14 + field + 4].copy_from_slice(&claim.to_be_bytes());
        bytes
    }

    #[test]
    fn huge_claims_fail_before_allocating() {
        let standard = Header::Standard {
            rate: 1,
            loop_points: (0, 0),
            base_note: 60,
            samples: vec![],
        };
        assert_eq!(
            decode_snd(&claiming(standard, 4, u32::MAX)),
            Err(SoundError::SamplesTruncated {
                offset: 36,
                needed: u64::from(u32::MAX),
                available: 64
            })
        );
        let extended = Header::Extended {
            channels: 2,
            rate: 1,
            sample_size: 16,
            data: vec![],
        };
        assert_eq!(
            decode_snd(&claiming(extended, 22, u32::MAX)),
            Err(SoundError::SamplesTruncated {
                offset: 78,
                needed: u64::from(u32::MAX) * 4,
                available: 22
            })
        );
        let compressed = Header::Compressed {
            channels: 2,
            rate: 1,
            format: *b"ima4",
            compression_id: -1,
            packets: vec![],
        };
        assert_eq!(
            decode_snd(&claiming(compressed, 22, u32::MAX)),
            Err(SoundError::SamplesTruncated {
                offset: 78,
                needed: u64::from(u32::MAX) * 68,
                available: 22
            })
        );
    }

    #[test]
    fn a_header_cut_before_its_samples_is_truncated() {
        let extended = Header::Extended {
            channels: 1,
            rate: 1,
            sample_size: 8,
            data: vec![],
        };
        let bytes = SndBuilder::new(SndFormat::Two, extended).bytes();
        assert_eq!(
            decode_snd(&bytes[..14 + 60]),
            Err(SoundError::SamplesTruncated {
                offset: 78,
                needed: 0,
                available: 0
            })
        );
        assert!(decode_snd(&bytes).unwrap().samples().is_empty());
    }

    fn sweep(format: SndFormat, header: Header) {
        assert_never_panics(&SndBuilder::new(format, header).bytes(), decode_snd);
    }

    #[test]
    fn corrupt_sounds_never_panic() {
        let mut codes = [3; 64];
        codes[5] = 0xC;
        let packet = ima4_packet(-3000, 40, codes);
        sweep(
            SndFormat::One,
            Header::Standard {
                rate: 0x2B77_0000,
                loop_points: (1, 2),
                base_note: 60,
                samples: vec![0x80, 0x00, 0xFF],
            },
        );
        sweep(
            SndFormat::Two,
            Header::Standard {
                rate: 0x2B77_45D1,
                loop_points: (0, 0),
                base_note: 60,
                samples: vec![0x7F],
            },
        );
        for (channels, sample_size) in [(1, 8), (2, 16)] {
            sweep(
                SndFormat::One,
                Header::Extended {
                    channels,
                    rate: 0xAC44_0000,
                    sample_size,
                    data: vec![0x81, 0x7E, 0x00, 0xFF],
                },
            );
        }
        for channels in [1, 2] {
            sweep(
                SndFormat::Two,
                Header::Compressed {
                    channels,
                    rate: 0x5622_0000,
                    format: *b"ima4",
                    compression_id: -1,
                    packets: vec![packet; 2],
                },
            );
        }
    }
}
