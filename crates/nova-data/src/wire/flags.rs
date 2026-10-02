//! Flag words.
//!
//! Flag fields keep every bit as stored, including bits the Bible does not
//! name: nothing is dropped or validated. Each serializes as a plain number.

use binrw::BinRead;
use serde::{Deserialize, Serialize};

macro_rules! flag_words {
    ($($(#[doc = $doc:literal])* $name:ident($bits:ty);)*) => {$(
        $(#[doc = $doc])*
        #[derive(BinRead, Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub $bits);

        impl $name {
            /// Every bit, as stored.
            #[must_use]
            pub fn bits(self) -> $bits {
                self.0
            }

            /// Whether every bit in `mask` is set.
            #[must_use]
            pub fn contains(self, mask: $bits) -> bool {
                self.0 & mask == mask
            }
        }
    )*};
}

flag_words! {
    /// A 16-bit flag word.
    Flags16(u16);
    /// A 32-bit flag word.
    Flags32(u32);
    /// A 64-bit flag word, such as the `Contribute` and `Require` pairs
    /// (two longs that the Bible treats as one 64-bit set).
    Flags64(u64);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[derive(BinRead, Debug, PartialEq)]
    #[br(big)]
    struct Words {
        a: Flags16,
        b: Flags32,
        c: Flags64,
    }

    #[test]
    fn reads_big_endian_and_keeps_every_bit() {
        let bytes = [
            0x80, 0x01, // a
            0xDE, 0xAD, 0xBE, 0xEF, // b
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, // c
        ];
        let words = Words::read_be(&mut Cursor::new(bytes)).expect("decodes");
        assert_eq!(words.a.bits(), 0x8001);
        assert_eq!(words.b.bits(), 0xDEAD_BEEF);
        assert_eq!(words.c.bits(), 0x0123_4567_89AB_CDEF);
    }

    #[test]
    fn contains_needs_every_bit_of_the_mask() {
        let flags = Flags16(0b0110);
        assert!(flags.contains(0b0100));
        assert!(flags.contains(0b0110));
        assert!(!flags.contains(0b0001));
        assert!(!flags.contains(0b0101), "one of two bits set");
        assert!(Flags32(0x8000_0000).contains(0x8000_0000));
        assert!(!Flags64(1).contains(2));
        assert!(Flags64(3).contains(0));
    }

    #[test]
    fn serializes_as_a_number() {
        let json = serde_json::to_string(&Flags32(0x8000_0001)).expect("serializes");
        assert_eq!(json, "2147483649");
        let back: Flags32 = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, Flags32(0x8000_0001));
    }
}
