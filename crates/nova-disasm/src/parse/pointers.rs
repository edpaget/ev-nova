//! `otool -s __IMPORT __pointers`: the pointer slots' values.

use std::collections::BTreeMap;

/// Parses the hex dump into slot address -> the little-endian word there.
pub fn parse(listing: &str) -> BTreeMap<u32, u32> {
    let mut slots = BTreeMap::new();
    for line in listing.lines().skip(1) {
        let mut fields = line.split_whitespace();
        let Some(base) = fields.next().and_then(super::hex) else {
            continue;
        };
        let bytes: Vec<u8> = fields
            .map_while(|b| {
                (b.len() == 2)
                    .then(|| u8::from_str_radix(b, 16).ok())
                    .flatten()
            })
            .collect();
        for (i, word) in bytes.as_chunks::<4>().0.iter().enumerate() {
            let value = u32::from_le_bytes([word[0], word[1], word[2], word[3]]);
            slots.insert(base + 4 * i as u32, value);
        }
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::fixture::POINTERS;

    #[test]
    fn reads_each_word_little_endian_at_its_address() {
        let slots = parse(POINTERS);
        assert_eq!(slots.get(&0x3b_5000), Some(&0));
        assert_eq!(slots.get(&0x3b_500c), Some(&0x0018_6820));
        assert_eq!(slots.get(&0x3b_504c), Some(&0x0023_1790));
        assert_eq!(slots.len(), 8);
    }

    #[test]
    fn skips_the_header_and_a_partial_word() {
        let slots =
            parse("/x:\nContents of (__IMPORT,__pointers) section\n00001000\t01 00 00 00 02 00\n");
        assert_eq!(slots.into_iter().collect::<Vec<_>>(), [(0x1000, 1)]);
    }
}
