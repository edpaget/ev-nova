//! PackBits run-length decoding, over 1-byte or 2-byte units.
//!
//! Source: Apple Technical Note TN1023 "Understanding PackBits", and
//! *Inside Macintosh: Imaging With QuickDraw* (1994) ch. 4 for `packType` 3,
//! which runs the same scheme over 16-bit pixels instead of bytes.
//!
//! Each run starts with a flag byte `n`: 0..=127 copies the next `n + 1`
//! units literally, 129..=255 repeats the next unit `257 - n` times, and 128
//! does nothing.

/// Unpacks one packed row of `unit`-byte units (1 or 2) into exactly
/// `expected` bytes. `None` if the row overruns or underruns `expected`, or
/// ends in the middle of a run.
pub(crate) fn unpack(packed: &[u8], unit: usize, expected: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(expected);
    let mut pos = 0;
    // Every run consumes at least its flag byte, so after `packed.len()`
    // runs the next pass sees the end.
    for _ in 0..=packed.len() {
        let Some(&flag) = packed.get(pos) else {
            return (out.len() == expected).then_some(out);
        };
        pos += 1;
        match flag {
            0..=127 => {
                let len = (usize::from(flag) + 1) * unit;
                out.extend_from_slice(packed.get(pos..pos + len)?);
                pos += len;
            }
            128 => {}
            129..=255 => {
                let repeated = packed.get(pos..pos + unit)?;
                for _ in 0..257 - usize::from(flag) {
                    out.extend_from_slice(repeated);
                }
                pos += unit;
            }
        }
        if out.len() > expected {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::fixture::pack_bits;

    /// TN1023's worked example, packed and unpacked.
    const TN1023_PACKED: [u8; 15] = [
        0xFE, 0xAA, 0x02, 0x80, 0x00, 0x2A, 0xFD, 0xAA, 0x03, 0x80, 0x00, 0x2A, 0x22, 0xF7, 0xAA,
    ];
    const TN1023_UNPACKED: [u8; 24] = [
        0xAA, 0xAA, 0xAA, 0x80, 0x00, 0x2A, 0xAA, 0xAA, 0xAA, 0xAA, 0x80, 0x00, 0x2A, 0x22, 0xAA,
        0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA,
    ];

    #[test]
    fn decodes_the_tn1023_example() {
        assert_eq!(
            unpack(&TN1023_PACKED, 1, 24).as_deref(),
            Some(&TN1023_UNPACKED[..])
        );
    }

    #[test]
    fn literal_runs_copy_n_plus_one_units() {
        assert_eq!(unpack(&[0, 7], 1, 1), Some(vec![7]));
        assert_eq!(unpack(&[2, 1, 2, 3], 1, 3), Some(vec![1, 2, 3]));
        assert_eq!(unpack(&[1, 1, 2, 3, 4], 2, 4), Some(vec![1, 2, 3, 4]));
        let mut longest = vec![127];
        longest.extend(0..128);
        assert_eq!(unpack(&longest, 1, 128), Some((0..128).collect()));
    }

    #[test]
    fn repeat_runs_copy_one_unit_257_minus_n_times() {
        assert_eq!(unpack(&[0xFF, 9], 1, 2), Some(vec![9, 9]));
        assert_eq!(unpack(&[0x81, 9], 1, 128), Some(vec![9; 128]));
        assert_eq!(unpack(&[0xFE, 1, 2], 2, 6), Some(vec![1, 2, 1, 2, 1, 2]));
    }

    #[test]
    fn flag_128_is_a_no_op() {
        assert_eq!(unpack(&[0x80, 0, 5, 0x80], 1, 1), Some(vec![5]));
        assert_eq!(unpack(&[0x80], 1, 0), Some(Vec::new()));
    }

    #[test]
    fn overrun_underrun_and_truncated_runs_are_rejected() {
        // Unpacks to 3 bytes, one more than expected.
        assert_eq!(unpack(&[0xFE, 1], 1, 2), None);
        // Unpacks to 1 byte, one fewer than expected.
        assert_eq!(unpack(&[0, 1], 1, 2), None);
        // Leftover data after the row is complete.
        assert_eq!(unpack(&[0, 1, 0, 2], 1, 1), None);
        // A literal run cut short, and a repeat with no unit to repeat.
        assert_eq!(unpack(&[2, 1, 2], 1, 3), None);
        assert_eq!(unpack(&[0xFE], 1, 3), None);
        assert_eq!(unpack(&[0xFE, 1], 2, 6), None);
        assert_eq!(unpack(&[0, 1], 2, 2), None);
        assert_eq!(unpack(&[], 1, 1), None);
        assert_eq!(unpack(&[], 1, 0), Some(Vec::new()));
    }

    #[test]
    fn a_long_row_of_no_ops_finishes() {
        assert_eq!(unpack(&[0x80; 4096], 1, 0), Some(Vec::new()));
        assert_eq!(unpack(&[0x80; 4096], 1, 1), None);
    }

    #[test]
    fn the_fixture_encoder_reproduces_tn1023() {
        assert_eq!(pack_bits(&TN1023_UNPACKED, 1), TN1023_PACKED);
    }

    #[test]
    fn the_fixture_encoder_round_trips() {
        let mut rows: Vec<Vec<u8>> = vec![
            vec![5],
            vec![5; 2],
            vec![5; 127],
            vec![5; 128],
            vec![5; 129],
            vec![5; 300],
            (0..=255).collect(),
            (0..300).map(|i| (i * 7 % 251) as u8).collect(),
            (0..300).map(|i| (i / 3) as u8).collect(),
            vec![1, 2, 2, 3, 3, 3, 4, 5],
        ];
        rows.extend([1, 127, 128, 129, 300].map(|len| (0..len).map(|i| (i % 2) as u8).collect()));
        for row in &rows {
            assert_eq!(unpack(&pack_bits(row, 1), 1, row.len()).as_ref(), Some(row));
            if row.len() % 2 == 0 {
                assert_eq!(unpack(&pack_bits(row, 2), 2, row.len()).as_ref(), Some(row));
            }
        }
    }

    #[test]
    fn the_fixture_encoder_packs_runs_and_literals_of_up_to_128_units() {
        assert_eq!(pack_bits(&[7; 300], 1), [0x81, 7, 0x81, 7, 0xD5, 7]);
        assert_eq!(pack_bits(&[1, 2, 1, 2, 3, 4], 2), [0xFF, 1, 2, 0, 3, 4]);
        let distinct: Vec<u8> = (0..200).map(|i| i as u8).collect();
        let packed = pack_bits(&distinct, 1);
        assert_eq!(packed.len(), 202);
        assert_eq!((packed[0], packed[129]), (127, 71));
    }
}
