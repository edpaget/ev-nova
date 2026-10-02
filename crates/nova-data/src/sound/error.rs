//! Why a sound failed to decode.

use std::fmt;

/// A decoding failure. Offsets are bytes from the start of the resource.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SoundError {
    /// The data ended in the middle of a read that began at `offset`.
    #[error("unexpected end of data at byte {offset:#x}")]
    UnexpectedEnd {
        /// Where the failing read began.
        offset: usize,
    },
    /// A `snd ` resource format other than 1 or 2.
    #[error("unsupported snd format {format}")]
    UnsupportedFormat {
        /// The format word at the start of the resource.
        format: u16,
    },
    /// A command list with no sound command (`soundCmd` or `bufferCmd`).
    #[error("the snd resource has no sound command")]
    NoSoundCommand,
    /// A sound command whose parameter is a memory pointer rather than an
    /// offset into the resource (its data-offset flag, bit 15, is clear).
    #[error("unsupported sound command {command:#06x} at byte {offset:#x}: no data-offset flag")]
    UnsupportedCommand {
        /// The command word.
        command: u16,
        /// Where the command starts.
        offset: usize,
    },
    /// A sound header whose `encode` byte is not standard (`0x00`),
    /// extended (`0xFF`) or compressed (`0xFE`).
    #[error("unsupported sound header encoding {encode:#04x} at byte {offset:#x}")]
    UnsupportedHeader {
        /// The `encode` byte.
        encode: u8,
        /// Where the header starts.
        offset: usize,
    },
    /// A compressed sound header using anything but IMA4.
    #[error("unsupported sound compression {}", CompressionName(*format, *compression_id))]
    UnsupportedCompression {
        /// The header's `format`; zero when only the ID names it.
        format: [u8; 4],
        /// The header's `compressionID`.
        compression_id: i16,
    },
    /// An extended header with a sample size other than 8 or 16 bits.
    #[error("unsupported sample size of {bits} bits")]
    UnsupportedSampleSize {
        /// `sampleSize`.
        bits: u16,
    },
    /// An extended or compressed header with a channel count other than 1
    /// or 2.
    #[error("unsupported channel count {channels}")]
    UnsupportedChannels {
        /// `numChannels`.
        channels: u32,
    },
    /// A sample rate of zero.
    #[error("the sample rate is zero")]
    BadSampleRate,
    /// Less sample data than the header claims.
    #[error("sample data at byte {offset:#x} needs {needed} bytes, only {available} remain")]
    SamplesTruncated {
        /// Where the sample data starts.
        offset: usize,
        /// Bytes the header claims.
        needed: u64,
        /// Bytes left in the resource from `offset`.
        available: usize,
    },
}

/// Names a compression for [`SoundError::UnsupportedCompression`]: by its
/// `format` when that is set, otherwise by its well-known ID.
struct CompressionName([u8; 4], i16);

impl fmt::Display for CompressionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(format, id) = *self;
        if format != [0; 4] {
            write!(f, "'{}' ", format.escape_ascii())?;
        } else if id == 3 {
            f.write_str("MACE 3:1 ")?;
        } else if id == 4 {
            f.write_str("MACE 6:1 ")?;
        }
        write!(f, "(compressionID {id})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_problem_and_its_offset() {
        let cases = [
            (
                SoundError::UnexpectedEnd { offset: 0x2A },
                "unexpected end of data at byte 0x2a",
            ),
            (
                SoundError::UnsupportedFormat { format: 3 },
                "unsupported snd format 3",
            ),
            (
                SoundError::NoSoundCommand,
                "the snd resource has no sound command",
            ),
            (
                SoundError::UnsupportedCommand {
                    command: 0x51,
                    offset: 0xC,
                },
                "unsupported sound command 0x0051 at byte 0xc: no data-offset flag",
            ),
            (
                SoundError::UnsupportedHeader {
                    encode: 1,
                    offset: 0x14,
                },
                "unsupported sound header encoding 0x01 at byte 0x14",
            ),
            (
                SoundError::UnsupportedSampleSize { bits: 12 },
                "unsupported sample size of 12 bits",
            ),
            (
                SoundError::UnsupportedChannels { channels: 3 },
                "unsupported channel count 3",
            ),
            (SoundError::BadSampleRate, "the sample rate is zero"),
            (
                SoundError::SamplesTruncated {
                    offset: 0x16,
                    needed: 100,
                    available: 7,
                },
                "sample data at byte 0x16 needs 100 bytes, only 7 remain",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn unsupported_compression_names_the_format_or_the_well_known_id() {
        let message = |format: &[u8; 4], compression_id| {
            SoundError::UnsupportedCompression {
                format: *format,
                compression_id,
            }
            .to_string()
        };
        let none = &[0; 4];
        assert_eq!(
            message(none, 3),
            "unsupported sound compression MACE 3:1 (compressionID 3)"
        );
        assert_eq!(
            message(none, 4),
            "unsupported sound compression MACE 6:1 (compressionID 4)"
        );
        assert_eq!(
            message(none, 5),
            "unsupported sound compression (compressionID 5)"
        );
        assert_eq!(
            message(b"MAC3", -1),
            "unsupported sound compression 'MAC3' (compressionID -1)"
        );
        assert_eq!(
            message(b"raw ", 0),
            "unsupported sound compression 'raw ' (compressionID 0)"
        );
        assert_eq!(
            message(b"ima4", 3),
            "unsupported sound compression 'ima4' (compressionID 3)"
        );
        assert_eq!(
            message(&[0, 0, 0, 1], 0),
            "unsupported sound compression '\\x00\\x00\\x00\\x01' (compressionID 0)"
        );
    }
}
