//! Synthetic `snd ` builders for tests.
//!
//! Available to this crate's tests and, through the `fixture` feature, to
//! other crates' tests. The builders write well-formed sound resources from
//! plain values, so tests never need game data.

/// The `snd ` resource format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SndFormat {
    /// Format 1: a data-format list, then the command list.
    One,
    /// Format 2: a reference count, then the command list.
    Two,
}

/// A sound header and its sample data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Header {
    /// A standard header (`encode` `0x00`): 8-bit mono samples.
    Standard {
        /// Fixed 16.16 sample rate.
        rate: u32,
        /// `loopStart` and `loopEnd`.
        loop_points: (u32, u32),
        /// `baseFrequency`.
        base_note: u8,
        /// The samples; `numBytes` is their count.
        samples: Vec<u8>,
    },
    /// An extended header (`encode` `0xFF`). `numFrames` is the data length
    /// divided by the frame size (0 when the frame size is 0).
    Extended {
        /// `numChannels`.
        channels: u32,
        /// Fixed 16.16 sample rate.
        rate: u32,
        /// `sampleSize` in bits.
        sample_size: u16,
        /// The sample data, written as given.
        data: Vec<u8>,
    },
    /// A compressed header (`encode` `0xFE`). `numFrames` is the packet
    /// count divided by `channels` (0 when `channels` is 0).
    Compressed {
        /// `numChannels`.
        channels: u32,
        /// Fixed 16.16 sample rate.
        rate: u32,
        /// `format`, such as `ima4`.
        format: [u8; 4],
        /// `compressionID`.
        compression_id: i16,
        /// The packets, channels interleaved (left, right, left, ...).
        packets: Vec<[u8; 34]>,
    },
}

impl Header {
    /// The header followed by its sample data.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = vec![0; 4]; // samplePtr
        match self {
            Self::Standard {
                rate,
                loop_points,
                base_note,
                samples,
            } => {
                common(&mut out, samples.len() as u32, *rate, *loop_points, 0x00);
                out.push(*base_note);
                out.extend(samples);
            }
            Self::Extended {
                channels,
                rate,
                sample_size,
                data,
            } => {
                let frame = channels * u32::from(sample_size.div_ceil(8));
                let frames = (data.len() as u32).checked_div(frame).unwrap_or(0);
                common(&mut out, *channels, *rate, (0, 0), 0xFF);
                out.push(60);
                out.extend(frames.to_be_bytes());
                out.extend([0; 22]); // AIFFSampleRate, marker, instrument, AES
                out.extend(sample_size.to_be_bytes());
                out.extend([0; 14]); // futureUse1 to futureUse4
                out.extend(data);
            }
            Self::Compressed {
                channels,
                rate,
                format,
                compression_id,
                packets,
            } => {
                let frames = (packets.len() as u32).checked_div(*channels).unwrap_or(0);
                common(&mut out, *channels, *rate, (0, 0), 0xFE);
                out.push(60);
                out.extend(frames.to_be_bytes());
                out.extend([0; 14]); // AIFFSampleRate, markerChunk
                out.extend(format);
                out.extend([0; 12]); // futureUse2, stateVars, leftOverSamples
                out.extend(compression_id.to_be_bytes());
                out.extend([0; 4]); // packetSize, snthID
                out.extend(16u16.to_be_bytes());
                for packet in packets {
                    out.extend(packet);
                }
            }
        }
        out
    }
}

/// `numChannels`/`numBytes`, `sampleRate`, `loopStart`, `loopEnd`, `encode`.
fn common(out: &mut Vec<u8>, count: u32, rate: u32, loops: (u32, u32), encode: u8) {
    for word in [count, rate, loops.0, loops.1] {
        out.extend(word.to_be_bytes());
    }
    out.push(encode);
}

/// Builds a `snd ` resource around a [`Header`].
///
/// By default the command list is one `bufferCmd` with the data-offset flag
/// (`0x8051`) pointing at the header, which directly follows the list, as
/// in every stock sound. [`SndBuilder::commands`] replaces the list and
/// [`SndBuilder::gap`] moves the header further out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SndBuilder {
    format: SndFormat,
    header: Header,
    data_formats: u16,
    commands: Option<Vec<(u16, i16, u32)>>,
    gap: usize,
}

impl SndBuilder {
    /// A resource of `format` holding `header`. Format 1 gets one
    /// data-format entry (sampledSynth, initOptions `0x80`).
    #[must_use]
    pub fn new(format: SndFormat, header: Header) -> Self {
        Self {
            format,
            header,
            data_formats: 1,
            commands: None,
            gap: 0,
        }
    }

    /// Sets how many data-format entries format 1 writes, each a copy of
    /// the default one.
    #[must_use]
    pub fn data_formats(mut self, count: u16) -> Self {
        self.data_formats = count;
        self
    }

    /// Replaces the command list with (`cmd`, `param1`, `param2`) entries.
    #[must_use]
    pub fn commands(mut self, commands: Vec<(u16, i16, u32)>) -> Self {
        self.commands = Some(commands);
        self
    }

    /// Writes `len` zero bytes between the command list and the header.
    #[must_use]
    pub fn gap(mut self, len: usize) -> Self {
        self.gap = len;
        self
    }

    /// Where the header starts.
    #[must_use]
    pub fn header_offset(&self) -> usize {
        let list = match self.format {
            SndFormat::One => 4 + 6 * usize::from(self.data_formats),
            SndFormat::Two => 4,
        };
        list + 2 + 8 * self.commands.as_ref().map_or(1, Vec::len) + self.gap
    }

    /// The resource's bytes.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self.format {
            SndFormat::One => {
                out.extend(1u16.to_be_bytes());
                out.extend(self.data_formats.to_be_bytes());
                for _ in 0..self.data_formats {
                    out.extend(5u16.to_be_bytes());
                    out.extend(0x80u32.to_be_bytes());
                }
            }
            SndFormat::Two => out.extend([0, 2, 0, 0]),
        }
        let default = [(0x8051, 0, self.header_offset() as u32)];
        let commands = self.commands.as_deref().unwrap_or(&default);
        out.extend((commands.len() as u16).to_be_bytes());
        for (cmd, param1, param2) in commands {
            out.extend(cmd.to_be_bytes());
            out.extend(param1.to_be_bytes());
            out.extend(param2.to_be_bytes());
        }
        out.resize(out.len() + self.gap, 0);
        out.extend(self.header.bytes());
        out
    }
}

/// One IMA4 packet: a header word holding the top 9 bits of `predictor`
/// and the low 7 bits of `index`, then the 64 four-bit `codes` packed two
/// to a byte, low nibble first.
#[must_use]
pub fn ima4_packet(predictor: i16, index: u8, codes: [u8; 64]) -> [u8; 34] {
    let mut packet = [0; 34];
    // The fields share no bits, so `+` combines them (and, unlike `|`,
    // cannot be swapped for `^` unnoticed).
    let header = (predictor as u16 & 0xFF80) + u16::from(index & 0x7F);
    packet[..2].copy_from_slice(&header.to_be_bytes());
    for (byte, pair) in packet[2..].iter_mut().zip(codes.as_chunks::<2>().0) {
        *byte = (pair[0] & 0x0F) + (pair[1] << 4);
    }
    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_format_1_standard_sound_has_the_stock_layout() {
        let snd = SndBuilder::new(
            SndFormat::One,
            Header::Standard {
                rate: 0x2B77_0000,
                loop_points: (1, 2),
                base_note: 61,
                samples: vec![0x80, 0xFF],
            },
        );
        #[rustfmt::skip]
        let expected = [
            0, 1, 0, 1, 0, 5, 0, 0, 0, 0x80, // format 1, one sampledSynth entry
            0, 1, 0x80, 0x51, 0, 0, 0, 0, 0, 20, // bufferCmd, header at 20
            0, 0, 0, 0, // samplePtr
            0, 0, 0, 2, // numBytes
            0x2B, 0x77, 0, 0, // sampleRate
            0, 0, 0, 1, 0, 0, 0, 2, // loopStart, loopEnd
            0x00, 61, // encode, baseFrequency
            0x80, 0xFF, // samples
        ];
        assert_eq!(snd.header_offset(), 20);
        assert_eq!(snd.bytes(), expected);
    }

    #[test]
    fn a_format_2_extended_sound_has_its_header_after_one_command() {
        let snd = SndBuilder::new(
            SndFormat::Two,
            Header::Extended {
                channels: 2,
                rate: 0xAC44_0000,
                sample_size: 16,
                data: vec![1, 2, 3, 4, 5, 6, 7, 8],
            },
        );
        let mut expected = vec![0, 2, 0, 0, 0, 1, 0x80, 0x51, 0, 0, 0, 0, 0, 14];
        expected.extend([0, 0, 0, 0, 0, 0, 0, 2, 0xAC, 0x44, 0, 0]);
        expected.extend([0; 8]); // loop points
        expected.extend([0xFF, 60, 0, 0, 0, 2]); // encode, base, numFrames
        expected.extend([0; 22]);
        expected.extend([0, 16]); // sampleSize
        expected.extend([0; 14]);
        expected.extend([1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(snd.header_offset(), 14);
        assert_eq!(snd.bytes(), expected);
    }

    #[test]
    fn a_compressed_sound_counts_frames_per_channel() {
        let packets = vec![[0xAA; 34], [0xBB; 34]];
        let header = Header::Compressed {
            channels: 2,
            rate: 0x5622_0000,
            format: *b"ima4",
            compression_id: -1,
            packets,
        };
        let mut expected = vec![0, 0, 0, 0, 0, 0, 0, 2, 0x56, 0x22, 0, 0];
        expected.extend([0; 8]);
        expected.extend([0xFE, 60, 0, 0, 0, 1]); // encode, base, numFrames
        expected.extend([0; 14]);
        expected.extend(b"ima4");
        expected.extend([0; 12]);
        expected.extend([0xFF, 0xFF, 0, 0, 0, 0, 0, 16]); // id -1, sampleSize
        expected.extend([0xAA; 34]);
        expected.extend([0xBB; 34]);
        assert_eq!(header.bytes(), expected);
    }

    #[test]
    fn frame_counts_tolerate_zero_sized_frames() {
        let extended = Header::Extended {
            channels: 0,
            rate: 1,
            sample_size: 8,
            data: vec![1, 2],
        };
        assert_eq!(&extended.bytes()[22..26], [0; 4]);
        let odd = Header::Extended {
            channels: 1,
            rate: 1,
            sample_size: 12,
            data: vec![1, 2, 3, 4],
        };
        assert_eq!(&odd.bytes()[22..26], [0, 0, 0, 2]);
        let compressed = Header::Compressed {
            channels: 0,
            rate: 1,
            format: *b"ima4",
            compression_id: -1,
            packets: vec![[0; 34]],
        };
        assert_eq!(&compressed.bytes()[22..26], [0; 4]);
    }

    #[test]
    fn commands_data_formats_and_gaps_move_the_header() {
        let header = Header::Standard {
            rate: 1,
            loop_points: (0, 0),
            base_note: 60,
            samples: vec![],
        };
        let snd = SndBuilder::new(SndFormat::One, header.clone())
            .data_formats(2)
            .commands(vec![(0x002B, -2, 7), (0x8050, 0, 40)])
            .gap(4);
        assert_eq!(snd.header_offset(), 4 + 12 + 2 + 16 + 4);
        let bytes = snd.bytes();
        assert_eq!(&bytes[..4], [0, 1, 0, 2]);
        assert_eq!(&bytes[4..16], [0, 5, 0, 0, 0, 0x80, 0, 5, 0, 0, 0, 0x80]);
        assert_eq!(&bytes[16..18], [0, 2]);
        assert_eq!(&bytes[18..26], [0, 0x2B, 0xFF, 0xFE, 0, 0, 0, 7]);
        assert_eq!(&bytes[26..34], [0x80, 0x50, 0, 0, 0, 0, 0, 40]);
        assert_eq!(&bytes[34..38], [0; 4]);
        assert_eq!(&bytes[38..], header.bytes());
    }

    #[test]
    fn ima4_packets_pack_the_header_and_codes_low_nibble_first() {
        let mut codes = [0; 64];
        codes[0] = 0x4;
        codes[1] = 0x7;
        codes[63] = 0xF;
        let packet = ima4_packet(-32768, 88, codes);
        assert_eq!(&packet[..4], [0x80, 0x58, 0x74, 0x00]);
        assert_eq!(packet[33], 0xF0);
        // The predictor keeps only its top 9 bits; the index its low 7.
        assert_eq!(&ima4_packet(0x7FFF, 0xFF, [0; 64])[..2], [0x7F, 0xFF]);
        assert_eq!(&ima4_packet(0x0040, 0x80, [0; 64])[..2], [0x00, 0x00]);
    }
}
