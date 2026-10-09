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
//!
//! # Evaluating a test
//!
//! [`holds`] evaluates a test's tree against a read-only view of the
//! pilot ([`PilotFacts`]), as the original's `_EvalTestExp` does (see
//! [`nova_data::expr`]): a blank test holds; `Bxxx` reads the bit, `G`
//! the gender (true when male), `Oxxx` whether the pilot has the outfit,
//! `Exxx` whether it has explored the system, and `Pxxx` whether the game
//! counts as paid for; `!`, `&` and `|` combine them as written, each
//! binary, since the parser accepts one operator a group; and a count
//! holds when as many of its terms hold as its comparison asks. Test
//! evaluation draws nothing at random: the test grammar has no random
//! operand.
//!
//! The rules that test a record's control bits ask the [`ControlBits`]
//! port, which the caller wires at the edge, for the player's pilot (a
//! [`Gate`]): bar hire and the shipyard (`shïp` `Availability`), a
//! person's `ActiveOn`, the outfitter (`oütf` `Availability`), the
//! exchange (`jünk` `BuyOn` and `SellOn`), the day's events (`öops`
//! `ActivateOn`), and the traffic (`flët` and `shïp` `AppearOn`).
//! [`NovaBits`] is the original's evaluation, [`holds`].
//!
//! # Running a set expression
//!
//! [`execute`] runs a set expression on a target holding control bits
//! ([`BitStore`]): it writes the bits itself, draws `R(...)` on the
//! caller's [`Chance`](crate::Chance), and hands every other operator to
//! the caller's dispatch, which applies it or gives back its kind as
//! unhandled, skipping and reporting one given back (see the `set`
//! submodule). The session's dispatch is `Session::apply_op`.

mod set;

use std::fmt::Debug;
use std::sync::Arc;

pub use nova_data::{
    Bit, BitWrite, Comparison, ParsedSet, ParsedTest, SetExpr, SetOp, SetOpKind, TestExpr,
    TestOperand,
};

pub use self::set::{BitStore, execute};
use crate::catalog::{OutfitId, SystemId};
use crate::pilot::Gender;

/// What a control-bit test reads about the pilot.
pub trait PilotFacts: Debug {
    /// Whether control bit `bit` is set.
    fn bit(&self, bit: Bit) -> bool;
    /// The player's gender.
    fn gender(&self) -> Gender;
    /// Whether the game counts as paid for, unregistered for fewer than
    /// `days` days.
    fn paid(&self, days: u16) -> bool;
    /// Whether the player has at least one of `outfit`.
    fn has_outfit(&self, outfit: OutfitId) -> bool;
    /// Whether the player has explored `system`.
    fn explored(&self, system: SystemId) -> bool;
}

/// Whether `test` holds for `pilot` (see the module docs).
#[must_use]
pub fn holds(test: &TestExpr, pilot: &(impl PilotFacts + ?Sized)) -> bool {
    match test {
        TestExpr::Always => true,
        TestExpr::Operand(operand) => operand_holds(*operand, pilot),
        TestExpr::Not(inner) => !holds(inner, pilot),
        TestExpr::And(a, b) => holds(a, pilot) && holds(b, pilot),
        TestExpr::Or(a, b) => holds(a, pilot) || holds(b, pilot),
        TestExpr::Count { terms, cmp, value } => {
            let count = terms
                .iter()
                .filter(|term| operand_holds(term.operand, pilot) != term.negated)
                .count();
            let value = usize::from(*value);
            match cmp {
                Comparison::Less => count < value,
                Comparison::Greater => count > value,
                Comparison::Equal => count == value,
            }
        }
    }
}

/// Whether a control-bit test holds for the pilot: the port the rules
/// that test a record's control bits ask.
pub trait ControlBits: Debug {
    /// Whether `test` holds for `pilot`.
    fn allows(&self, test: &TestExpr, pilot: &dyn PilotFacts) -> bool;
}

/// The original's control bits: a test holds as [`holds`] says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NovaBits;

impl ControlBits for NovaBits {
    fn allows(&self, test: &TestExpr, pilot: &dyn PilotFacts) -> bool {
        holds(test, pilot)
    }
}

/// The port a rule tests a record's control bits through, and the pilot
/// it tests them for.
#[derive(Clone, Copy, Debug)]
pub struct Gate<'a> {
    /// The control-bit test.
    pub control_bits: &'a dyn ControlBits,
    /// What the control-bit test reads about the player.
    pub pilot: &'a dyn PilotFacts,
}

impl Gate<'_> {
    /// Whether `test` holds: never when it did not parse, and then the
    /// port is not asked.
    #[must_use]
    pub fn allows(&self, test: &Test) -> bool {
        test.holds(|test| self.control_bits.allows(test, self.pilot))
    }
}

impl Gate<'static> {
    /// A new pilot (no bit set, male, paid for, owning nothing and having
    /// explored nowhere) tested by Nova's control bits.
    pub const FRESH: Self = Self {
        control_bits: &NovaBits,
        pilot: &FreshPilot,
    };
}

/// A new pilot, as a control-bit test reads one: no bit set, male, paid
/// for, owning nothing and having explored nowhere.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FreshPilot;

impl PilotFacts for FreshPilot {
    fn bit(&self, _bit: Bit) -> bool {
        false
    }

    fn gender(&self) -> Gender {
        Gender::Male
    }

    fn paid(&self, _days: u16) -> bool {
        true
    }

    fn has_outfit(&self, _outfit: OutfitId) -> bool {
        false
    }

    fn explored(&self, _system: SystemId) -> bool {
        false
    }
}

/// What running a set expression has to tell the player's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptNote {
    /// An operator of this kind was skipped: nothing handles it yet. Each
    /// kind is told once a session.
    Unhandled(SetOpKind),
}

/// Whether one operand holds for `pilot`.
fn operand_holds(operand: TestOperand, pilot: &(impl PilotFacts + ?Sized)) -> bool {
    match operand {
        TestOperand::Bit(bit) => pilot.bit(bit),
        TestOperand::Paid { days } => pilot.paid(days),
        TestOperand::Male => pilot.gender() == Gender::Male,
        TestOperand::HasOutfit(outfit) => pilot.has_outfit(outfit),
        TestOperand::Explored(system) => pilot.explored(system),
    }
}

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

/// A record's set-expression hook: its operators, parsed once and shared,
/// or why it did not parse. The default is a blank script, which has no
/// operators. A script that did not parse does nothing when run: the safe
/// direction, where a broken hook writes no bit rather than half its bits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Script(pub(crate) ParsedSet);

impl Script {
    /// `text` parsed.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        Self(Arc::new(SetExpr::parse(text)))
    }

    /// The operators, or `None` when the script did not parse.
    #[must_use]
    pub fn tree(&self) -> Option<&SetExpr> {
        self.0.as_ref().as_ref().ok()
    }
}

impl Default for Script {
    fn default() -> Self {
        Self(Arc::new(Ok(SetExpr::default())))
    }
}

impl From<ParsedSet> for Script {
    fn from(parsed: ParsedSet) -> Self {
        Self(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_default_script_is_blank() {
        assert_eq!(Script::default(), Script::parse(""));
        assert_eq!(Script::default().tree().map(|set| set.ops.len()), Some(0));
    }

    #[test]
    fn a_script_gives_its_operators() {
        let script = Script::parse("b7 !b8");
        assert_eq!(script.tree(), SetExpr::parse("b7 !b8").ok().as_ref());
        assert_eq!(script.tree().map(|set| set.ops.len()), Some(2));
    }

    #[test]
    fn a_script_that_did_not_parse_has_no_tree() {
        assert_eq!(Script::parse("b1&").tree(), None);
    }

    #[test]
    fn a_script_from_a_parse_shares_it() {
        let parsed: ParsedSet = Arc::new(SetExpr::parse("b2"));
        let script = Script::from(Arc::clone(&parsed));
        assert!(Arc::ptr_eq(&script.0, &parsed));
    }

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

    /// A pilot view holding these bits, outfits and explored systems, this
    /// gender, and paid as `paid` says.
    #[derive(Debug, Default)]
    struct FakePilot {
        bits: Vec<u16>,
        female: bool,
        outfits: Vec<i16>,
        explored: Vec<i16>,
        unpaid: bool,
    }

    impl PilotFacts for FakePilot {
        fn bit(&self, bit: Bit) -> bool {
            self.bits.contains(&bit.get())
        }

        fn gender(&self) -> Gender {
            if self.female {
                Gender::Female
            } else {
                Gender::Male
            }
        }

        fn paid(&self, _days: u16) -> bool {
            !self.unpaid
        }

        fn has_outfit(&self, outfit: OutfitId) -> bool {
            self.outfits.contains(&outfit.0)
        }

        fn explored(&self, system: SystemId) -> bool {
            self.explored.contains(&system.0)
        }
    }

    fn with_bits(bits: &[u16]) -> FakePilot {
        FakePilot {
            bits: bits.to_vec(),
            ..FakePilot::default()
        }
    }

    /// Whether `text` holds for `pilot`.
    fn eval(text: &str, pilot: &FakePilot) -> bool {
        holds(&TestExpr::parse(text).expect("parses"), pilot)
    }

    #[test]
    fn a_blank_test_always_holds() {
        assert!(holds(&TestExpr::Always, &FakePilot::default()));
        assert!(eval("", &with_bits(&[])));
    }

    #[test]
    fn a_bit_holds_while_it_is_set() {
        assert!(eval("b5", &with_bits(&[5])));
        assert!(!eval("b5", &with_bits(&[4, 6])));
        assert!(!eval("!b5", &with_bits(&[5])));
        assert!(eval("!b5", &with_bits(&[])));
    }

    #[test]
    fn g_holds_for_a_male_pilot() {
        assert!(eval("g", &FakePilot::default()));
        let female = FakePilot {
            female: true,
            ..FakePilot::default()
        };
        assert!(!eval("G", &female));
        assert!(eval("!g", &female));
    }

    #[test]
    fn an_outfit_holds_while_it_is_owned() {
        let owner = FakePilot {
            outfits: vec![130],
            ..FakePilot::default()
        };
        assert!(eval("o130", &owner));
        assert!(!eval("o131", &owner));
    }

    #[test]
    fn a_system_holds_once_explored() {
        let explorer = FakePilot {
            explored: vec![200],
            ..FakePilot::default()
        };
        assert!(eval("e200", &explorer));
        assert!(!eval("e201", &explorer));
    }

    #[test]
    fn paid_follows_the_pilot_view() {
        assert!(eval("p30", &FakePilot::default()));
        let unpaid = FakePilot {
            unpaid: true,
            ..FakePilot::default()
        };
        assert!(!eval("p30", &unpaid));
    }

    #[test]
    fn and_and_or_follow_their_truth_tables() {
        for (bits, and, or) in [
            (&[][..], false, false),
            (&[1][..], false, true),
            (&[2][..], false, true),
            (&[1, 2][..], true, true),
        ] {
            let pilot = with_bits(bits);
            assert_eq!(eval("(b1 & b2)", &pilot), and, "{bits:?}");
            assert_eq!(eval("(b1 | b2)", &pilot), or, "{bits:?}");
        }
    }

    #[test]
    fn negated_groups_nest() {
        let male = |bits: &[u16]| with_bits(bits);
        assert!(eval("!(b1 | b2) & g", &male(&[])));
        assert!(!eval("!(b1 | b2) & g", &male(&[2])));
        let female = FakePilot {
            female: true,
            ..FakePilot::default()
        };
        assert!(!eval("!(b1 | b2) & g", &female));
        assert!(eval("(b1 & (b2 | !b3)) | e128", &male(&[1, 2, 3])));
        assert!(!eval("(b1 & (b2 | !b3)) | e128", &male(&[1, 3])));
    }

    #[test]
    fn a_count_compares_how_many_terms_hold() {
        // Terms b1, b2 and !b3: none hold with bit 3 set alone.
        for (bits, count) in [(&[3][..], 0), (&[][..], 1), (&[1][..], 2), (&[1, 2][..], 3)] {
            let pilot = with_bits(bits);
            assert_eq!(eval("( [b1 b2 !b3] > 1)", &pilot), count > 1, "{bits:?}");
            assert_eq!(eval("( [b1 b2 !b3] < 1)", &pilot), count < 1, "{bits:?}");
            assert_eq!(eval("( [b1 b2 !b3] = 2)", &pilot), count == 2, "{bits:?}");
        }
    }

    #[test]
    fn novas_control_bits_hold_as_the_evaluator_says() {
        for (text, bits) in [
            ("b3", &[3][..]),
            ("b3", &[][..]),
            ("!b3 & g", &[][..]),
            ("", &[][..]),
        ] {
            let tree = TestExpr::parse(text).expect("parses");
            let pilot = with_bits(bits);
            assert_eq!(
                NovaBits.allows(&tree, &pilot),
                holds(&tree, &pilot),
                "{text} {bits:?}"
            );
        }
        assert!(NovaBits.allows(&TestExpr::parse("b3").expect("parses"), &with_bits(&[3])));
        assert!(!NovaBits.allows(&TestExpr::parse("b3").expect("parses"), &with_bits(&[])));
    }

    #[test]
    fn a_gate_asks_its_port_about_its_pilot() {
        /// Holds a test while its pilot holds bit 4, recording each ask.
        #[derive(Debug, Default)]
        struct Recording(std::cell::RefCell<Vec<TestExpr>>);

        impl ControlBits for Recording {
            fn allows(&self, test: &TestExpr, pilot: &dyn PilotFacts) -> bool {
                self.0.borrow_mut().push(test.clone());
                pilot.bit(bit(4))
            }
        }

        let port = Recording::default();
        let with_4 = with_bits(&[4]);
        let gate = Gate {
            control_bits: &port,
            pilot: &with_4,
        };
        assert!(gate.allows(&Test::parse("b9")));
        let without = with_bits(&[]);
        let gate = Gate {
            pilot: &without,
            ..gate
        };
        assert!(!gate.allows(&Test::parse("b9")));
        let asked = TestExpr::parse("b9").expect("parses");
        assert_eq!(*port.0.borrow(), [asked.clone(), asked]);
    }

    #[test]
    fn a_gate_refuses_a_malformed_test_without_asking_its_port() {
        let gate = Gate {
            control_bits: &crate::testkit::AllowAll,
            ..Gate::FRESH
        };
        assert!(gate.allows(&Test::parse("b1")));
        assert!(!gate.allows(&Test::parse("b1 &")));
    }

    #[test]
    fn the_fresh_gate_tests_a_new_pilot_by_novas_bits() {
        let fresh = Gate::FRESH;
        assert!(fresh.allows(&Test::default()), "blank");
        assert!(!fresh.allows(&Test::parse("b3")), "no bit is set");
        assert!(fresh.allows(&Test::parse("!b3 & g")), "male");
        assert!(fresh.allows(&Test::parse("p30")), "paid");
        assert!(!fresh.allows(&Test::parse("o130")), "owning nothing");
        assert!(!fresh.allows(&Test::parse("e128")), "explored nowhere");
    }

    #[test]
    fn the_refusing_fake_refuses_tests_that_read_its_bits() {
        let gate = Gate {
            control_bits: &crate::testkit::RefuseBits(&[7]),
            ..Gate::FRESH
        };
        assert!(!gate.allows(&Test::parse("b7")));
        assert!(!gate.allows(&Test::parse("!b7")), "it reads bit 7");
        assert!(gate.allows(&Test::parse("!b8")));
        assert!(gate.allows(&Test::default()));
    }

    #[test]
    fn a_bare_number_after_an_operator_leaves_the_left_side() {
        assert!(eval("(b50 | 467)", &with_bits(&[50])));
        assert!(!eval("(b50 | 467)", &with_bits(&[467])));
    }
}
