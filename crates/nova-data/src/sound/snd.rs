//! The `snd ` resource header and command list, down to the sound header.

use super::bytes::{slice_at, u16_at, u32_at};
use super::header::decode_header;
use super::{Pcm, SoundError};

/// `soundCmd`: play the sound header the parameter points at.
const SOUND_CMD: u16 = 0x50;
/// `bufferCmd`: the same, as a buffered command.
const BUFFER_CMD: u16 = 0x51;
/// Set on a command whose `param2` is an offset into the resource rather
/// than a memory pointer.
const DATA_OFFSET_FLAG: u16 = 0x8000;
/// One command: `cmd` u16, `param1` i16, `param2` u32.
const COMMAND_LEN: usize = 8;
/// One format 1 data-format entry: data type u16, `initOptions` u32.
const DATA_FORMAT_LEN: usize = 6;

/// Decodes one `snd ` resource to PCM.
///
/// The resource may be format 1 or 2. Its first sound or buffer command
/// must carry the data-offset flag and point at a standard, extended or
/// compressed sound header; see the [module docs](super) for what each
/// supports. Bytes after the sample data are ignored.
pub fn decode_snd(data: &[u8]) -> Result<Pcm, SoundError> {
    decode_header(data, header_offset(data)?)
}

/// The offset of the sound header that the first sound or buffer command
/// points at.
pub(super) fn header_offset(data: &[u8]) -> Result<usize, SoundError> {
    let count_at = match u16_at(data, 0)? {
        1 => {
            let formats = usize::from(u16_at(data, 2)?);
            slice_at(data, 4, formats * DATA_FORMAT_LEN)?;
            4 + formats * DATA_FORMAT_LEN
        }
        // Format 2: a reference count at 2, which only matters in memory.
        2 => 4,
        format => return Err(SoundError::UnsupportedFormat { format }),
    };
    let count = usize::from(u16_at(data, count_at)?);
    let list_at = count_at + 2;
    // The whole list is in the data, so the loop is bounded by its length.
    slice_at(data, list_at, count * COMMAND_LEN)?;
    for at in (list_at..list_at + count * COMMAND_LEN).step_by(COMMAND_LEN) {
        let cmd = u16_at(data, at)?;
        if !matches!(cmd & !DATA_OFFSET_FLAG, SOUND_CMD | BUFFER_CMD) {
            continue;
        }
        if cmd & DATA_OFFSET_FLAG == 0 {
            return Err(SoundError::UnsupportedCommand {
                command: cmd,
                offset: at,
            });
        }
        let offset = u32_at(data, at + 4)? as usize;
        if offset >= data.len() {
            return Err(SoundError::UnexpectedEnd { offset });
        }
        return Ok(offset);
    }
    Err(SoundError::NoSoundCommand)
}

#[cfg(test)]
mod tests {
    use super::super::fixture::{Header, SndBuilder, SndFormat};
    use super::*;

    fn standard() -> Header {
        Header::Standard {
            rate: 0x2B77_0000,
            loop_points: (0, 0),
            base_note: 60,
            samples: vec![0x80; 3],
        }
    }

    fn located(snd: &SndBuilder) -> Result<usize, SoundError> {
        header_offset(&snd.bytes())
    }

    #[test]
    fn both_formats_lead_to_the_header() {
        let one = SndBuilder::new(SndFormat::One, standard());
        assert_eq!(located(&one), Ok(20));
        let two = SndBuilder::new(SndFormat::Two, standard());
        assert_eq!(located(&two), Ok(14));
    }

    #[test]
    fn format_1_skips_every_data_format_entry() {
        for count in [0, 2] {
            let snd = SndBuilder::new(SndFormat::One, standard()).data_formats(count);
            let expected = 4 + 6 * usize::from(count) + 2 + 8;
            assert_eq!(snd.header_offset(), expected);
            assert_eq!(located(&snd), Ok(expected), "{count} entries");
        }
    }

    #[test]
    fn other_formats_are_unsupported() {
        let mut bytes = SndBuilder::new(SndFormat::Two, standard()).bytes();
        for format in [0, 3, 0x0102] {
            bytes[..2].copy_from_slice(&u16::to_be_bytes(format));
            assert_eq!(
                header_offset(&bytes),
                Err(SoundError::UnsupportedFormat { format })
            );
        }
    }

    #[test]
    fn other_commands_before_the_sound_command_are_skipped() {
        // Header after 3 commands: 4 + 2 + 24 = 30, plus a gap of 2.
        let snd = SndBuilder::new(SndFormat::Two, standard())
            .commands(vec![(0x002B, 0, 0), (0x8003, 1, 2), (0x8051, 0, 32)])
            .gap(2);
        assert_eq!(snd.header_offset(), 32);
        assert_eq!(located(&snd), Ok(32));
    }

    #[test]
    fn the_first_sound_or_buffer_command_wins() {
        let sound = SndBuilder::new(SndFormat::Two, standard())
            .commands(vec![(0x8050, 0, 22), (0x8051, 0, 99)]);
        assert_eq!(located(&sound), Ok(22));
        let buffer = SndBuilder::new(SndFormat::Two, standard()).commands(vec![
            (0x8051, 7, 22),
            (0x8050, 0, 99),
            (0x0050, 0, 0),
        ]);
        assert_eq!(located(&buffer), Ok(22));
    }

    #[test]
    fn a_command_without_the_offset_flag_is_unsupported() {
        for command in [0x0050, 0x0051] {
            let snd = SndBuilder::new(SndFormat::One, standard())
                .commands(vec![(0x8003, 0, 0), (command, 0, 0x1234)]);
            assert_eq!(
                located(&snd),
                Err(SoundError::UnsupportedCommand {
                    command,
                    offset: 20
                })
            );
        }
    }

    #[test]
    fn commands_only_match_on_their_low_15_bits() {
        // 0x4051 and 0xC051 are not bufferCmd; 0x0003 is not either.
        let snd = SndBuilder::new(SndFormat::Two, standard()).commands(vec![
            (0x4051, 0, 0),
            (0xC051, 0, 0),
            (0x0003, 0, 0),
        ]);
        assert_eq!(located(&snd), Err(SoundError::NoSoundCommand));
    }

    #[test]
    fn a_list_without_a_sound_command_has_none() {
        let empty = SndBuilder::new(SndFormat::One, standard()).commands(vec![]);
        assert_eq!(located(&empty), Err(SoundError::NoSoundCommand));
        let soundless = SndBuilder::new(SndFormat::Two, standard()).commands(vec![(0x002B, 0, 14)]);
        assert_eq!(located(&soundless), Err(SoundError::NoSoundCommand));
    }

    #[test]
    fn truncated_lists_end_where_their_read_began() {
        let bytes = SndBuilder::new(SndFormat::One, standard())
            .data_formats(2)
            .bytes();
        let end = |len: usize| header_offset(&bytes[..len]).unwrap_err();
        assert_eq!(end(1), SoundError::UnexpectedEnd { offset: 0 });
        assert_eq!(end(3), SoundError::UnexpectedEnd { offset: 2 });
        assert_eq!(end(15), SoundError::UnexpectedEnd { offset: 4 });
        assert_eq!(end(17), SoundError::UnexpectedEnd { offset: 16 });
        assert_eq!(end(25), SoundError::UnexpectedEnd { offset: 18 });
        let two = SndBuilder::new(SndFormat::Two, standard()).bytes();
        assert_eq!(
            header_offset(&two[..5]),
            Err(SoundError::UnexpectedEnd { offset: 4 })
        );
    }

    #[test]
    fn a_header_offset_outside_the_data_is_an_unexpected_end() {
        let snd =
            SndBuilder::new(SndFormat::Two, standard()).commands(vec![(0x8051, 0, 0xFFFF_FFFF)]);
        assert_eq!(
            located(&snd),
            Err(SoundError::UnexpectedEnd {
                offset: 0xFFFF_FFFF
            })
        );
        let len = snd.bytes().len();
        let past =
            SndBuilder::new(SndFormat::Two, standard()).commands(vec![(0x8051, 0, len as u32)]);
        assert_eq!(
            located(&past),
            Err(SoundError::UnexpectedEnd { offset: len })
        );
        let last =
            SndBuilder::new(SndFormat::Two, standard()).commands(vec![(0x8051, 0, len as u32 - 1)]);
        assert_eq!(located(&last), Ok(len - 1));
    }
}
