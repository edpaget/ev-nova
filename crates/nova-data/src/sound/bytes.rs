//! Bounds-checked big-endian reads at absolute offsets.
//!
//! The `snd ` layout is offset-based (the command list points at the sound
//! header, and header fields sit at fixed offsets from its start), so these
//! read at an offset rather than through a cursor. Every read either returns
//! exactly what it asked for or fails with [`SoundError::UnexpectedEnd`]
//! naming the offset where it began; no offset or length can overflow.

use super::SoundError;

/// The `len` bytes at `offset`.
pub(super) fn slice_at(data: &[u8], offset: usize, len: usize) -> Result<&[u8], SoundError> {
    offset
        .checked_add(len)
        .and_then(|end| data.get(offset..end))
        .ok_or(SoundError::UnexpectedEnd { offset })
}

/// The `N` bytes at `offset`.
pub(super) fn array_at<const N: usize>(data: &[u8], offset: usize) -> Result<[u8; N], SoundError> {
    slice_at(data, offset, N)?
        .try_into()
        .map_err(|_| SoundError::UnexpectedEnd { offset })
}

pub(super) fn u8_at(data: &[u8], offset: usize) -> Result<u8, SoundError> {
    Ok(array_at::<1>(data, offset)?[0])
}

pub(super) fn u16_at(data: &[u8], offset: usize) -> Result<u16, SoundError> {
    Ok(u16::from_be_bytes(array_at(data, offset)?))
}

pub(super) fn i16_at(data: &[u8], offset: usize) -> Result<i16, SoundError> {
    Ok(i16::from_be_bytes(array_at(data, offset)?))
}

pub(super) fn u32_at(data: &[u8], offset: usize) -> Result<u32, SoundError> {
    Ok(u32::from_be_bytes(array_at(data, offset)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: [u8; 6] = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xFF];

    fn end(offset: usize) -> SoundError {
        SoundError::UnexpectedEnd { offset }
    }

    #[test]
    fn reads_are_big_endian_at_their_offset() {
        assert_eq!(u8_at(&DATA, 5), Ok(0xFF));
        assert_eq!(u16_at(&DATA, 1), Ok(0x3456));
        assert_eq!(i16_at(&DATA, 4), Ok(-0x6501));
        assert_eq!(i16_at(&DATA, 0), Ok(0x1234));
        assert_eq!(u32_at(&DATA, 2), Ok(0x5678_9AFF));
        assert_eq!(slice_at(&DATA, 1, 3), Ok(&DATA[1..4]));
        assert_eq!(slice_at(&DATA, 6, 0), Ok(&[][..]));
    }

    #[test]
    fn a_read_past_the_end_names_its_offset() {
        assert_eq!(u8_at(&DATA, 6), Err(end(6)));
        assert_eq!(u16_at(&DATA, 5), Err(end(5)));
        assert_eq!(i16_at(&DATA, 5), Err(end(5)));
        assert_eq!(u32_at(&DATA, 3), Err(end(3)));
        assert_eq!(slice_at(&DATA, 2, 5), Err(end(2)));
        assert_eq!(slice_at(&DATA, 7, 0), Err(end(7)));
    }

    #[test]
    fn overflowing_offsets_and_lengths_fail() {
        assert_eq!(slice_at(&DATA, usize::MAX, 1), Err(end(usize::MAX)));
        assert_eq!(slice_at(&DATA, 1, usize::MAX), Err(end(1)));
        assert_eq!(u32_at(&DATA, usize::MAX - 1), Err(end(usize::MAX - 1)));
    }
}
