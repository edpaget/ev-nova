//! Control bits: the 10,000 flags missions and storylines script with.
//!
//! The pilot holds them as a [`ControlBitSet`], every bit clear on a new
//! pilot; [`nova_data::expr`] parses the expressions that read and write
//! them.
//!
//! A catalog record holds each control-bit test it names as a [`Test`]:
//! the tree parsed once (see [`GameData::test_expr`](nova_data::GameData::test_expr)),
//! or why it did not parse. A test that did not parse is never satisfied
//! ([`Test::holds`]): the safe direction, where a ship with a broken
//! `Availability` is not sold rather than sold out of turn.

use std::sync::Arc;

pub use nova_data::{Bit, BitWrite, ParsedTest, TestExpr};

/// How many `u64` words hold the bits: 10,000 bits, rounded up.
const WORDS: usize = (Bit::MAX as usize + 1).div_ceil(64);

/// The 10,000 control bits, 0-9999, each set or clear.
#[derive(Clone, PartialEq, Eq)]
pub struct ControlBitSet {
    words: [u64; WORDS],
}

impl Default for ControlBitSet {
    fn default() -> Self {
        Self { words: [0; WORDS] }
    }
}

impl std::fmt::Debug for ControlBitSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter().map(Bit::get)).finish()
    }
}

impl ControlBitSet {
    /// Every bit clear.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether `bit` is set.
    #[must_use]
    pub fn get(&self, bit: Bit) -> bool {
        let (word, mask) = Self::locate(bit);
        self.words[word] & mask != 0
    }

    /// Sets `bit`.
    pub fn set(&mut self, bit: Bit) {
        let (word, mask) = Self::locate(bit);
        self.words[word] |= mask;
    }

    /// Clears `bit`.
    pub fn clear(&mut self, bit: Bit) {
        let (word, mask) = Self::locate(bit);
        self.words[word] &= !mask;
    }

    /// Flips `bit`.
    pub fn toggle(&mut self, bit: Bit) {
        let (word, mask) = Self::locate(bit);
        self.words[word] ^= mask;
    }

    /// Writes `bit` as `write` says.
    pub fn write(&mut self, bit: Bit, write: BitWrite) {
        match write {
            BitWrite::Set => self.set(bit),
            BitWrite::Clear => self.clear(bit),
            BitWrite::Toggle => self.toggle(bit),
        }
    }

    /// The set bits, ascending.
    pub fn iter(&self) -> impl Iterator<Item = Bit> + '_ {
        (0..=Bit::MAX)
            .filter_map(Bit::new)
            .filter(|&bit| self.get(bit))
    }

    /// The word holding `bit` and its mask there.
    fn locate(bit: Bit) -> (usize, u64) {
        let n = usize::from(bit.get());
        (n / 64, 1 << (n % 64))
    }
}

impl FromIterator<Bit> for ControlBitSet {
    fn from_iter<I: IntoIterator<Item = Bit>>(iter: I) -> Self {
        let mut bits = Self::new();
        for bit in iter {
            bits.set(bit);
        }
        bits
    }
}

/// A record's control-bit test: its tree, parsed once and shared, or why
/// it did not parse. The default is a blank test, which always holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Test(pub(crate) ParsedTest);

impl Test {
    /// `text` parsed.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        Self(Arc::new(TestExpr::parse(text)))
    }

    /// The tree, or `None` when the test did not parse.
    #[must_use]
    pub fn tree(&self) -> Option<&TestExpr> {
        self.0.as_ref().as_ref().ok()
    }

    /// Whether the test holds by `check`: never when it did not parse.
    pub fn holds(&self, check: impl FnOnce(&TestExpr) -> bool) -> bool {
        self.tree().is_some_and(check)
    }
}

impl Default for Test {
    fn default() -> Self {
        Self(Arc::new(Ok(TestExpr::Always)))
    }
}

impl From<ParsedTest> for Test {
    fn from(parsed: ParsedTest) -> Self {
        Self(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_default_test_is_blank_and_always_holds() {
        assert_eq!(Test::default().tree(), Some(&TestExpr::Always));
        assert_eq!(Test::default(), Test::parse(""));
        assert!(Test::default().holds(|tree| *tree == TestExpr::Always));
    }

    #[test]
    fn a_test_holds_as_its_check_says_on_its_tree() {
        let test = Test::parse("b7");
        assert_eq!(test.tree(), TestExpr::parse("b7").ok().as_ref());
        assert!(test.holds(|tree| tree.reads().contains(&bit(7))));
        assert!(!test.holds(|_| false));
    }

    #[test]
    fn a_test_that_did_not_parse_never_holds() {
        let test = Test::parse("b1 &");
        assert_eq!(test.tree(), None);
        let mut asked = false;
        assert!(!test.holds(|_| {
            asked = true;
            true
        }));
        assert!(!asked, "the check is never asked");
    }

    #[test]
    fn a_test_from_a_parse_shares_it() {
        let parsed: ParsedTest = Arc::new(TestExpr::parse("b2"));
        let test = Test::from(Arc::clone(&parsed));
        assert!(Arc::ptr_eq(&test.0, &parsed));
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    #[test]
    fn a_new_set_has_every_bit_clear() {
        let bits = ControlBitSet::new();
        assert!((0..=Bit::MAX).all(|n| !bits.get(bit(n))));
        assert_eq!(bits.iter().count(), 0);
        assert_eq!(bits, ControlBitSet::default());
    }

    #[test]
    fn the_first_and_last_bits_set_clear_and_toggle_on_their_own() {
        let mut bits = ControlBitSet::new();
        bits.set(bit(0));
        assert!(bits.get(bit(0)));
        assert!(!bits.get(bit(9999)));
        assert!(!bits.get(bit(1)), "a neighbour stays clear");
        bits.set(bit(9999));
        assert!(bits.get(bit(9999)));
        assert!(!bits.get(bit(9998)));
        bits.clear(bit(0));
        assert!(!bits.get(bit(0)));
        assert!(bits.get(bit(9999)), "clearing one leaves the other");
        bits.toggle(bit(0));
        assert!(bits.get(bit(0)));
        bits.toggle(bit(9999));
        assert!(!bits.get(bit(9999)));
        assert!(bits.get(bit(0)), "toggling one leaves the other");
    }

    #[test]
    fn bits_in_one_word_are_kept_apart() {
        let mut bits = ControlBitSet::new();
        bits.set(bit(64));
        bits.set(bit(127));
        bits.clear(bit(64));
        assert!(!bits.get(bit(64)));
        assert!(bits.get(bit(127)));
        bits.set(bit(63));
        assert!(!bits.get(bit(64)), "a bit in the word before");
    }

    #[test]
    fn the_set_bits_iterate_in_ascending_order() {
        let mut bits = ControlBitSet::new();
        for n in [9999, 3, 64, 0, 512] {
            bits.set(bit(n));
        }
        let set: Vec<u16> = bits.iter().map(Bit::get).collect();
        assert_eq!(set, [0, 3, 64, 512, 9999]);
    }

    #[test]
    fn collecting_bits_sets_them_and_round_trips() {
        let bits: ControlBitSet = [bit(5), bit(700), bit(5)].into_iter().collect();
        assert_eq!(bits.iter().collect::<Vec<_>>(), [bit(5), bit(700)]);
        assert_eq!(bits.iter().collect::<ControlBitSet>(), bits);
    }

    #[test]
    fn a_write_follows_its_bit_write() {
        let mut bits = ControlBitSet::new();
        bits.write(bit(8), BitWrite::Set);
        assert!(bits.get(bit(8)));
        bits.write(bit(8), BitWrite::Set);
        assert!(bits.get(bit(8)), "setting a set bit keeps it");
        bits.write(bit(8), BitWrite::Toggle);
        assert!(!bits.get(bit(8)));
        bits.write(bit(8), BitWrite::Toggle);
        assert!(bits.get(bit(8)));
        bits.write(bit(8), BitWrite::Clear);
        assert!(!bits.get(bit(8)));
        bits.write(bit(8), BitWrite::Clear);
        assert!(!bits.get(bit(8)), "clearing a clear bit keeps it");
    }

    #[test]
    fn the_debug_form_lists_the_set_bits() {
        let bits: ControlBitSet = [bit(2), bit(9)].into_iter().collect();
        assert_eq!(format!("{bits:?}"), "{2, 9}");
    }
}
