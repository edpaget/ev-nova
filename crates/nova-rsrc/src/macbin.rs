//! The MacBinary container: a 128-byte header, then the data fork and the
//! resource fork, each padded to a multiple of 128 bytes.
//!
//! See the crate docs' `## MacBinary` section for what is accepted and why.

use std::ops::Range;

use crate::{Fork, ParseError};

/// Length of the MacBinary header, and the unit forks are padded to.
const HEADER_LEN: usize = 128;
/// The MacBinary III signature and where it sits in the header.
const SIGNATURE: &[u8; 4] = b"mBIN";
const SIGNATURE_FIELD: usize = 102;
const CRC_FIELD: usize = 124;

/// Where a MacBinary file's two forks lie, as ranges of the whole file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Container {
    /// The data fork.
    pub(crate) data: Range<usize>,
    /// The resource fork; empty when the header gives it no length.
    pub(crate) resource: Range<usize>,
}

/// Reads a MacBinary header, as Apple's `macbinary` tool (which the
/// original's launcher runs) accepts one.
///
/// `Ok(None)` means the bytes are not MacBinary. A header that is MacBinary
/// but places a fork past the end of the file, or a III header with a bad
/// CRC, is an error.
pub(crate) fn read(bytes: &[u8]) -> Result<Option<Container>, ParseError> {
    let Some(header) = bytes.get(..HEADER_LEN) else {
        return truncated(bytes);
    };
    if header[0] != 0 || header[74] != 0 || header[82] != 0 {
        return Ok(None);
    }
    let data_len = u32_at(header, 83);
    let resource_len = u32_at(header, 87);
    // Apple's tool judges a header with bytes 101 to 125 zero by the
    // version I rules even when its CRC (then 0) matches, so a zero-filled
    // file is not MacBinary.
    if header[101..=125].iter().all(|&b| b == 0) {
        if header[1] > 63 || (data_len, resource_len) == (0, 0) {
            return Ok(None);
        }
    } else {
        let stored = u16_at(header, CRC_FIELD);
        let computed = crc16_xmodem(&header[..CRC_FIELD]);
        if stored != computed {
            if signed(header) {
                return Err(ParseError::MacBinaryCrcMismatch { stored, computed });
            }
            return Ok(None);
        }
    }

    let secondary = u64::from(u16_at(header, 120));
    let data_start = HEADER_LEN as u64 + padded(secondary);
    let data = fork_range(Fork::Data, data_start, data_len, bytes.len())?;
    let resource = if resource_len == 0 {
        bytes.len()..bytes.len()
    } else {
        let start = data_start + padded(u64::from(data_len));
        fork_range(Fork::Resource, start, resource_len, bytes.len())?
    };
    Ok(Some(Container { data, resource }))
}

/// A buffer shorter than the header is MacBinary only if it holds the III
/// signature (and so is at least 106 bytes long) after a zero first byte.
fn truncated(bytes: &[u8]) -> Result<Option<Container>, ParseError> {
    if bytes.first() == Some(&0)
        && bytes.len() >= SIGNATURE_FIELD + SIGNATURE.len()
        && signed(bytes)
    {
        Err(ParseError::MacBinaryHeaderTruncated {
            file_len: bytes.len(),
        })
    } else {
        Ok(None)
    }
}

fn signed(header: &[u8]) -> bool {
    header[SIGNATURE_FIELD..SIGNATURE_FIELD + SIGNATURE.len()] == *SIGNATURE
}

/// The range `start..start + len`, or an error naming `fork` if it runs
/// past `file_len`.
fn fork_range(
    fork: Fork,
    start: u64,
    len: u32,
    file_len: usize,
) -> Result<Range<usize>, ParseError> {
    let end = start + u64::from(len);
    match (usize::try_from(start), usize::try_from(end)) {
        (Ok(start), Ok(end)) if end <= file_len => Ok(start..end),
        _ => Err(ParseError::MacBinaryForkOutOfBounds {
            fork,
            offset: start,
            len,
            file_len,
        }),
    }
}

/// `len` rounded up to a multiple of 128.
fn padded(len: u64) -> u64 {
    len.next_multiple_of(HEADER_LEN as u64)
}

fn u16_at(bytes: &[u8], pos: usize) -> u16 {
    u16::from_be_bytes([bytes[pos], bytes[pos + 1]])
}

fn u32_at(bytes: &[u8], pos: usize) -> u32 {
    u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
}

/// CRC-16/XMODEM (polynomial `0x1021`, initial value 0, no reflection), as
/// MacBinary II and III store over the header's first 124 bytes.
pub(crate) fn crc16_xmodem(bytes: &[u8]) -> u16 {
    bytes.iter().fold(0, |crc, &byte| {
        (0..8).fold(crc ^ (u16::from(byte) << 8), |crc, _| {
            if crc & 0x8000 == 0 {
                crc << 1
            } else {
                (crc << 1) ^ 0x1021
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{
        BuiltMacBinary, ForkBuilder, MACBINARY_CRC_FIELD, MACBINARY_DATA_LEN_FIELD,
        MACBINARY_NAME_LEN_FIELD, MACBINARY_RESOURCE_LEN_FIELD, MACBINARY_SECONDARY_HEADER_FIELD,
        MACBINARY_SIGNATURE, MACBINARY_SIGNATURE_FIELD, MacBinaryBuilder, MacBinaryVersion,
    };
    use crate::{Fork, ParseError, ResType};

    use MacBinaryVersion::{I, II, III};

    fn fork() -> Vec<u8> {
        ForkBuilder::new()
            .resource(ResType(*b"PICT"), 128, None, b"pixels")
            .build()
            .bytes
    }

    fn build(version: MacBinaryVersion) -> BuiltMacBinary {
        MacBinaryBuilder::new(fork()).version(version).build()
    }

    fn container(built: &BuiltMacBinary) -> Container {
        read(&built.bytes)
            .expect("no container error")
            .expect("is MacBinary")
    }

    fn not_macbinary(built: &BuiltMacBinary) -> bool {
        matches!(read(&built.bytes), Ok(None))
    }

    #[test]
    fn crc_matches_the_xmodem_check_value() {
        assert_eq!(crc16_xmodem(b"123456789"), 0x31C3);
        assert_eq!(crc16_xmodem(b""), 0);
        assert_eq!(crc16_xmodem(&[0x80]), 0x9188);
    }

    #[test]
    fn every_version_is_accepted_and_locates_the_forks() {
        for version in [I, II, III] {
            let built = MacBinaryBuilder::new(fork())
                .version(version)
                .data_fork(b"hello".to_vec())
                .build();
            let found = container(&built);
            assert_eq!(found.data, 128..133, "{version:?}");
            assert_eq!(
                found.resource,
                built.layout.resource_offset..built.layout.resource_end,
                "{version:?}"
            );
            assert_eq!(&built.bytes[found.resource], fork().as_slice());
        }
    }

    #[test]
    fn version_i_accepts_name_lengths_up_to_63() {
        for len in [0, 1, 63] {
            let mut built = build(I);
            built.put_u8(MACBINARY_NAME_LEN_FIELD, len);
            assert!(read(&built.bytes).expect("ok").is_some(), "{len}");
        }
        let mut built = build(I);
        built.put_u8(MACBINARY_NAME_LEN_FIELD, 64);
        assert!(not_macbinary(&built));
    }

    #[test]
    fn a_crc_checked_header_takes_any_name_length() {
        let mut built = build(II);
        built.put_u8(MACBINARY_NAME_LEN_FIELD, 200).recompute_crc();
        assert!(read(&built.bytes).expect("ok").is_some());
    }

    #[test]
    fn non_zero_bytes_0_74_or_82_are_not_macbinary() {
        for version in [I, II, III] {
            for pos in [0, 74, 82] {
                let mut built = build(version);
                built.put_u8(pos, 1);
                if version != I {
                    built.recompute_crc();
                }
                assert!(not_macbinary(&built), "{version:?} byte {pos}");
            }
        }
    }

    #[test]
    fn version_i_needs_bytes_101_to_125_zero() {
        for pos in [101, 125] {
            let mut built = build(I);
            built.put_u8(pos, 1);
            assert!(not_macbinary(&built), "byte {pos}");
        }
    }

    #[test]
    fn version_i_with_both_forks_empty_is_not_macbinary() {
        let built = MacBinaryBuilder::new(Vec::new()).version(I).build();
        assert!(not_macbinary(&built));
        let data_only = MacBinaryBuilder::new(Vec::new())
            .data_fork(b"x".to_vec())
            .version(I)
            .build();
        assert!(read(&data_only.bytes).expect("ok").is_some());
        let resource_only = MacBinaryBuilder::new(vec![1]).version(I).build();
        assert!(read(&resource_only.bytes).expect("ok").is_some());
    }

    /// A version I shaped header (bytes 101 to 125 zero) whose CRC still
    /// matches: bytes 99 and 100 take the CRC of bytes 0 to 98, which
    /// zeroes the CRC of bytes 0 to 100, and the zeros after keep it zero.
    fn crc_matched_version_i(name_len: u8, data_len: u32) -> Vec<u8> {
        let mut header = vec![0; 128];
        header[MACBINARY_NAME_LEN_FIELD] = name_len;
        header[MACBINARY_DATA_LEN_FIELD..MACBINARY_DATA_LEN_FIELD + 4]
            .copy_from_slice(&data_len.to_be_bytes());
        let fix = crc16_xmodem(&header[..99]).to_be_bytes();
        header[99..=100].copy_from_slice(&fix);
        assert_eq!(crc16_xmodem(&header[..MACBINARY_CRC_FIELD]), 0);
        header.resize(128 + data_len.next_multiple_of(128) as usize, 0);
        header
    }

    #[test]
    fn a_zero_filled_buffer_is_not_macbinary() {
        // Its CRC (0) matches the zero stored at byte 124, but Apple's tool
        // judges a header with bytes 101 to 125 zero by the version I rules,
        // which reject two empty forks.
        assert!(matches!(read(&[0; 200]), Ok(None)));
        assert!(matches!(read(&[0; 128]), Ok(None)));
    }

    #[test]
    fn a_crc_matched_version_i_header_keeps_the_version_i_rules() {
        assert!(read(&crc_matched_version_i(63, 1)).expect("ok").is_some());
        assert!(matches!(read(&crc_matched_version_i(64, 1)), Ok(None)));
        assert!(matches!(read(&crc_matched_version_i(1, 0)), Ok(None)));
    }

    #[test]
    fn a_crc_checked_header_may_have_both_forks_empty() {
        for version in [II, III] {
            let built = MacBinaryBuilder::new(Vec::new()).version(version).build();
            let found = container(&built);
            assert!(found.data.is_empty() && found.resource.is_empty());
        }
    }

    #[test]
    fn a_bad_crc_without_the_signature_is_not_macbinary() {
        // A non-zero byte in 101..=125 rules out version I as well.
        let mut built = build(II);
        let crc = u16::from_be_bytes([
            built.bytes[MACBINARY_CRC_FIELD],
            built.bytes[MACBINARY_CRC_FIELD + 1],
        ]);
        built.put_u16(MACBINARY_CRC_FIELD, crc ^ 1);
        assert!(not_macbinary(&built));
    }

    #[test]
    fn a_bad_crc_with_the_signature_is_a_crc_mismatch() {
        let mut built = build(III);
        let stored = u16::from_be_bytes([
            built.bytes[MACBINARY_CRC_FIELD],
            built.bytes[MACBINARY_CRC_FIELD + 1],
        ]);
        built.put_u16(MACBINARY_CRC_FIELD, stored ^ 1);
        assert_eq!(
            read(&built.bytes).err(),
            Some(ParseError::MacBinaryCrcMismatch {
                stored: stored ^ 1,
                computed: stored,
            })
        );
    }

    #[test]
    fn the_data_fork_is_padded_to_128_bytes() {
        for (data_len, resource_start) in [(0, 128), (1, 256), (5, 256), (128, 256), (129, 384)] {
            let built = MacBinaryBuilder::new(fork())
                .data_fork(vec![9; data_len])
                .build();
            let found = container(&built);
            assert_eq!(found.data, 128..128 + data_len);
            assert_eq!(found.resource.start, resource_start, "{data_len}");
        }
    }

    #[test]
    fn the_secondary_header_is_skipped_rounded_up_to_128() {
        for (len, data_start) in [(1, 256), (128, 256), (129, 384), (200, 384)] {
            let built = MacBinaryBuilder::new(fork()).secondary_header(len).build();
            assert_eq!(container(&built).data.start, data_start, "{len}");
        }
    }

    #[test]
    fn the_resource_fork_may_end_at_the_end_of_the_file() {
        let mut built = build(II);
        let end = built.layout.resource_end;
        built.truncate(end);
        assert_eq!(container(&built).resource.end, end);
    }

    #[test]
    fn a_data_fork_only_file_needs_no_padding() {
        let mut built = MacBinaryBuilder::new(Vec::new())
            .data_fork(b"data".to_vec())
            .build();
        built.truncate(132);
        let found = container(&built);
        assert_eq!(found.data, 128..132);
        assert!(found.resource.is_empty());
    }

    fn out_of_bounds(built: &BuiltMacBinary) -> Option<(Fork, u64, u32)> {
        match read(&built.bytes) {
            Err(ParseError::MacBinaryForkOutOfBounds {
                fork,
                offset,
                len,
                file_len,
            }) => {
                assert_eq!(file_len, built.bytes.len());
                Some((fork, offset, len))
            }
            _ => None,
        }
    }

    #[test]
    fn a_resource_fork_short_by_one_byte_is_out_of_bounds() {
        let mut built = build(II);
        let (start, end) = (built.layout.resource_offset, built.layout.resource_end);
        built.truncate(end - 1);
        let len = u32::try_from(end - start).expect("small");
        assert_eq!(
            out_of_bounds(&built),
            Some((Fork::Resource, start as u64, len))
        );
    }

    #[test]
    fn huge_fork_lengths_are_out_of_bounds_without_overflow() {
        let mut built = build(II);
        built
            .put_u32(MACBINARY_RESOURCE_LEN_FIELD, u32::MAX)
            .recompute_crc();
        assert_eq!(out_of_bounds(&built), Some((Fork::Resource, 128, u32::MAX)));

        let mut built = build(II);
        built
            .put_u32(MACBINARY_DATA_LEN_FIELD, u32::MAX)
            .recompute_crc();
        assert_eq!(out_of_bounds(&built), Some((Fork::Data, 128, u32::MAX)));

        let mut built = build(II);
        built
            .put_u32(MACBINARY_DATA_LEN_FIELD, u32::MAX - 3)
            .put_u32(MACBINARY_RESOURCE_LEN_FIELD, u32::MAX)
            .recompute_crc();
        assert_eq!(out_of_bounds(&built), Some((Fork::Data, 128, u32::MAX - 3)));
    }

    #[test]
    fn fork_lengths_read_all_four_bytes() {
        for (field, fork) in [
            (MACBINARY_DATA_LEN_FIELD, Fork::Data),
            (MACBINARY_RESOURCE_LEN_FIELD, Fork::Resource),
        ] {
            for len in [0x0100_0000, 0x0001_0000, 0x0000_0100] {
                let mut built = build(II);
                built.put_u32(field, len).recompute_crc();
                let (found, _, found_len) = out_of_bounds(&built).expect("out of bounds");
                assert_eq!((found, found_len), (fork, len));
            }
        }
    }

    #[test]
    fn a_data_fork_past_the_end_is_out_of_bounds() {
        let mut built = build(II);
        built
            .put_u32(MACBINARY_DATA_LEN_FIELD, 10_000)
            .recompute_crc();
        assert_eq!(out_of_bounds(&built), Some((Fork::Data, 128, 10_000)));
    }

    #[test]
    fn a_secondary_header_past_the_end_is_out_of_bounds() {
        let mut built = build(II);
        built
            .put_u16(MACBINARY_SECONDARY_HEADER_FIELD, 0xFFFF)
            .recompute_crc();
        assert_eq!(out_of_bounds(&built), Some((Fork::Data, 128 + 65_536, 0)));
    }

    #[test]
    fn a_short_buffer_with_the_signature_is_a_truncated_header() {
        let mut built = build(III);
        built.truncate(127);
        assert_eq!(
            read(&built.bytes).err(),
            Some(ParseError::MacBinaryHeaderTruncated { file_len: 127 })
        );
        built.truncate(106);
        assert_eq!(
            read(&built.bytes).err(),
            Some(ParseError::MacBinaryHeaderTruncated { file_len: 106 })
        );
    }

    #[test]
    fn other_short_buffers_are_not_macbinary() {
        let mut built = build(III);
        built.truncate(105);
        assert!(not_macbinary(&built), "too short for the signature");

        let mut built = build(III);
        built.put_u8(0, 1).truncate(127);
        assert!(not_macbinary(&built), "byte 0 non-zero");

        for pos in 0..4 {
            let mut built = build(III);
            built
                .put_u8(MACBINARY_SIGNATURE_FIELD + pos, b'x')
                .truncate(127);
            assert!(not_macbinary(&built), "signature byte {pos}");
        }

        let mut built = build(II);
        built.truncate(127);
        assert!(not_macbinary(&built), "no signature");
        assert!(matches!(read(&[]), Ok(None)));
        assert_eq!(&MACBINARY_SIGNATURE[..], b"mBIN");
    }
}
