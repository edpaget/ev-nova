//! A hex dump of the bytes around a failing offset.

use std::fmt::Write as _;

/// Bytes per row.
const ROW: usize = 16;

/// Rows of 16 bytes, aligned to 16, around `offset` in `data`: two rows
/// before the failing row, the failing row and one after, clamped to the
/// data. The failing byte is bracketed and each row ends with an ASCII
/// gutter:
///
/// ```
/// let data: Vec<u8> = (0x40..0x60).collect();
/// assert_eq!(
///     nova_dump::hex::window(&data, 0x12),
///     [
///         "0x0000: 40 41 42 43 44 45 46 47 48 49 4a 4b 4c 4d 4e 4f  |@ABCDEFGHIJKLMNO|",
///         "0x0010: 50 51[52]53 54 55 56 57 58 59 5a 5b 5c 5d 5e 5f  |PQRSTUVWXYZ[\\]^_|",
///     ]
/// );
/// ```
///
/// An offset at or past the end shows the last three rows and a line saying
/// so; empty data shows only a line saying so.
#[must_use]
pub fn window(data: &[u8], offset: usize) -> Vec<String> {
    let Some(last_row) = data.len().checked_sub(1).map(|last| last / ROW) else {
        return vec!["(resource is empty)".to_owned()];
    };
    let inside = offset < data.len();
    let anchor_row = if inside { offset / ROW } else { last_row };
    let first = anchor_row.saturating_sub(2);
    let last = (anchor_row + 1).min(last_row);
    let mut rows: Vec<String> = (first..=last)
        .map(|row| hex_row(data, row * ROW, offset))
        .collect();
    if !inside {
        rows.push(format!(
            "(offset {offset:#x} is at or past the end of the {}-byte resource)",
            data.len()
        ));
    }
    rows
}

/// One row starting at `start`, bracketing the byte at `offset` if it is
/// in the row.
fn hex_row(data: &[u8], start: usize, offset: usize) -> String {
    let bytes = &data[start..data.len().min(start + ROW)];
    let mut row = format!("{start:#06x}:");
    for at in start..=start + ROW {
        let in_row = at < start + ROW;
        let separator = if at == offset && in_row {
            '['
        } else if at == offset.wrapping_add(1) && at > start {
            ']'
        } else {
            ' '
        };
        row.push(separator);
        match data.get(at) {
            Some(byte) if in_row => write!(row, "{byte:02x}").expect("String"),
            _ if in_row => row.push_str("  "),
            _ => {}
        }
    }
    row.push_str(" |");
    row.extend(bytes.iter().map(|&b| {
        if b == b' ' || b.is_ascii_graphic() {
            char::from(b)
        } else {
            '.'
        }
    }));
    row.push('|');
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `len` bytes counting up from 0x40 (`@ABC...`).
    fn data(len: usize) -> Vec<u8> {
        (0..len).map(|i| 0x40 + i as u8).collect()
    }

    const ROW_0: &str =
        "0x0000: 40 41 42 43 44 45 46 47 48 49 4a 4b 4c 4d 4e 4f  |@ABCDEFGHIJKLMNO|";
    const ROW_1: &str =
        "0x0010: 50 51 52 53 54 55 56 57 58 59 5a 5b 5c 5d 5e 5f  |PQRSTUVWXYZ[\\]^_|";
    const ROW_2: &str =
        "0x0020: 60 61 62 63 64 65 66 67 68 69 6a 6b 6c 6d 6e 6f  |`abcdefghijklmno|";
    const ROW_3: &str =
        "0x0030: 70 71 72 73 74 75 76 77 78 79 7a 7b 7c 7d 7e 7f  |pqrstuvwxyz{|}~.|";
    const ROW_4: &str =
        "0x0040: 80 81 82 83 84 85 86 87 88 89 8a 8b 8c 8d 8e 8f  |................|";

    #[test]
    fn the_window_is_two_rows_before_the_failing_row_and_one_after() {
        assert_eq!(
            window(&data(80), 0x32),
            [
                ROW_1,
                ROW_2,
                "0x0030: 70 71[72]73 74 75 76 77 78 79 7a 7b 7c 7d 7e 7f  |pqrstuvwxyz{|}~.|",
                ROW_4,
            ]
        );
    }

    #[test]
    fn the_window_is_clamped_at_the_start() {
        assert_eq!(
            window(&data(80), 0),
            [
                "0x0000:[40]41 42 43 44 45 46 47 48 49 4a 4b 4c 4d 4e 4f  |@ABCDEFGHIJKLMNO|",
                ROW_1,
            ]
        );
        assert_eq!(
            window(&data(80), 0x1f),
            [
                ROW_0,
                "0x0010: 50 51 52 53 54 55 56 57 58 59 5a 5b 5c 5d 5e[5f] |PQRSTUVWXYZ[\\]^_|",
                ROW_2,
            ]
        );
    }

    #[test]
    fn the_window_is_clamped_at_the_end() {
        assert_eq!(
            window(&data(64), 0x3f),
            [
                ROW_1,
                ROW_2,
                "0x0030: 70 71 72 73 74 75 76 77 78 79 7a 7b 7c 7d 7e[7f] |pqrstuvwxyz{|}~.|",
            ]
        );
    }

    #[test]
    fn a_partial_last_row_keeps_the_gutter_aligned() {
        assert_eq!(
            window(&data(20), 0x12),
            [
                ROW_0,
                "0x0010: 50 51[52]53                                      |PQRS|",
            ]
        );
        assert_eq!(
            window(&data(20), 0x13),
            [
                ROW_0,
                "0x0010: 50 51 52[53]                                     |PQRS|",
            ]
        );
    }

    #[test]
    fn an_offset_past_the_end_shows_the_last_rows_and_says_so() {
        assert_eq!(
            window(&data(64), 64),
            [
                ROW_1,
                ROW_2,
                ROW_3,
                "(offset 0x40 is at or past the end of the 64-byte resource)",
            ]
        );
        assert_eq!(
            window(&data(3), 0x1a2),
            [
                "0x0000: 40 41 42                                         |@AB|",
                "(offset 0x1a2 is at or past the end of the 3-byte resource)",
            ]
        );
    }

    #[test]
    fn empty_data_says_so() {
        assert_eq!(window(&[], 0), ["(resource is empty)"]);
        assert_eq!(window(&[], 9), ["(resource is empty)"]);
    }

    #[test]
    fn printable_ascii_is_shown_and_the_rest_is_a_dot() {
        let bytes = [0x1f, 0x20, 0x7e, 0x7f, 0xff];
        assert_eq!(
            window(&bytes, 4),
            ["0x0000: 1f 20 7e 7f[ff]                                  |. ~..|"]
        );
    }

    #[test]
    fn wide_offsets_widen_the_address() {
        let bytes = vec![0; 0x10010];
        let rows = window(&bytes, 0x10005);
        assert!(rows[2].starts_with("0x10000:"), "{rows:?}");
        assert!(rows[0].starts_with("0xffe0:"), "{rows:?}");
    }
}
