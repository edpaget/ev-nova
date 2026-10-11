//! Control bits that make a test hold, for a test that sets a pilot up
//! directly: [`bits_allowing`].

use crate::control::{Bit, Test, TestExpr, TestOperand};

/// An assignment of control bits that makes `test` hold for a pilot whose
/// other facts are a new pilot's: each bit named, with whether to set it
/// (`true`) or clear it.
///
/// `&` asks both sides, `|` its first side (its second when the first
/// cannot hold), and `!` flips what is asked. A blank test and `P` (the
/// game counts as paid for) ask nothing.
///
/// `None` when bits alone cannot make it hold: a count, an outfit, an
/// explored system, the gender, an unpaid game, an assignment that would
/// both set and clear one bit, or a test that did not parse.
#[must_use]
pub fn bits_allowing(test: &Test) -> Option<Vec<(Bit, bool)>> {
    let bits = making(test.tree()?, true)?;
    let conflicts = bits
        .iter()
        .any(|&(bit, on)| bits.iter().any(|&(other, set)| other == bit && set != on));
    (!conflicts).then_some(bits)
}

/// The bits that make `expr` come out `want`, or `None` when bits alone
/// cannot.
fn making(expr: &TestExpr, want: bool) -> Option<Vec<(Bit, bool)>> {
    match expr {
        TestExpr::Always | TestExpr::Operand(TestOperand::Paid { .. }) => want.then(Vec::new),
        TestExpr::Operand(TestOperand::Bit(bit)) => Some(vec![(*bit, want)]),
        TestExpr::Operand(_) | TestExpr::Count { .. } => None,
        TestExpr::Not(inner) => making(inner, !want),
        // Both sides come out `want` for an `&` asked true or an `|`
        // asked false; one side does for the others.
        TestExpr::And(a, b) | TestExpr::Or(a, b) if want == matches!(expr, TestExpr::And(..)) => {
            let mut bits = making(a, want)?;
            bits.extend(making(b, want)?);
            Some(bits)
        }
        TestExpr::And(a, b) | TestExpr::Or(a, b) => making(a, want).or_else(|| making(b, want)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{OutfitId, SystemId};
    use crate::control::{ControlBitSet, Gate, PilotFacts};
    use crate::pilot::Gender;

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("a bit")
    }

    fn allowing(text: &str) -> Option<Vec<(Bit, bool)>> {
        bits_allowing(&Test::parse(text))
    }

    /// A new pilot's facts, its bits `bits`.
    #[derive(Debug)]
    struct Bits(ControlBitSet);

    impl PilotFacts for Bits {
        fn bit(&self, bit: Bit) -> bool {
            self.0.get(bit)
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

    /// Whether `text` holds once the bits [`bits_allowing`] gives are
    /// written over `start`.
    fn holds_after(written: &str, start: &[u16]) -> bool {
        let test = Test::parse(written);
        let mut bits: ControlBitSet = start.iter().map(|&n| bit(n)).collect();
        for (b, on) in bits_allowing(&test).expect("bits can make it hold") {
            if on {
                bits.set(b);
            } else {
                bits.clear(b);
            }
        }
        let pilot = Bits(bits);
        Gate {
            control_bits: &crate::control::NovaBits,
            pilot: &pilot,
        }
        .allows(&test)
    }

    #[test]
    fn a_blank_test_and_paid_ask_nothing() {
        assert_eq!(allowing(""), Some(vec![]));
        assert_eq!(allowing("P0"), Some(vec![]));
    }

    #[test]
    fn a_bit_is_set_and_a_negated_bit_cleared() {
        assert_eq!(allowing("b304"), Some(vec![(bit(304), true)]));
        assert_eq!(allowing("!b305"), Some(vec![(bit(305), false)]));
        assert_eq!(allowing("!(!b305)"), Some(vec![(bit(305), true)]));
    }

    #[test]
    fn and_asks_both_sides() {
        assert_eq!(
            allowing("b1 & b2"),
            Some(vec![(bit(1), true), (bit(2), true)])
        );
    }

    #[test]
    fn or_asks_its_first_side_or_else_its_second() {
        assert_eq!(allowing("b1 | b2"), Some(vec![(bit(1), true)]));
        assert_eq!(allowing("o128 | b2"), Some(vec![(bit(2), true)]));
        assert_eq!(allowing("o128 | e130"), None);
    }

    #[test]
    fn a_negated_or_clears_both_sides_and_a_negated_and_its_first() {
        assert_eq!(
            allowing("!(b1 | b2)"),
            Some(vec![(bit(1), false), (bit(2), false)])
        );
        assert_eq!(allowing("!(b1 & b2)"), Some(vec![(bit(1), false)]));
        assert_eq!(allowing("!(o128 & b2)"), Some(vec![(bit(2), false)]));
        assert_eq!(allowing("!(o128 & e130)"), None);
        assert_eq!(allowing("!(o128 | b2)"), None);
        assert_eq!(allowing("!(b2 | o128)"), None);
    }

    #[test]
    fn what_bits_cannot_decide_gives_none() {
        assert_eq!(allowing("o128"), None, "an outfit");
        assert_eq!(allowing("e130"), None, "an explored system");
        assert_eq!(allowing("g"), None, "the gender");
        assert_eq!(allowing("!p0"), None, "an unpaid game");
        assert_eq!(allowing("( [b1 b2] > 1)"), None, "a count");
        assert_eq!(allowing("b1 & o128"), None, "one side of an and");
        assert_eq!(allowing("o128 & b1"), None, "the other side");
        assert_eq!(allowing("b1 &"), None, "an unparsed test");
    }

    #[test]
    fn a_bit_both_set_and_cleared_gives_none() {
        assert_eq!(allowing("b1 & !b1"), None);
        assert_eq!(
            allowing("b1 & b1"),
            Some(vec![(bit(1), true), (bit(1), true)]),
            "asked twice the same way"
        );
    }

    #[test]
    fn the_bits_make_stock_shaped_tests_hold() {
        for (text, start) in [
            ("(P0 & b304) & !b305", &[305][..]),
            ("!(b352 | b289) & !(b192 | b206)", &[352, 289, 192, 206][..]),
            ("(b281 | b319) & !b5758", &[5758][..]),
            ("b1 | b2", &[][..]),
            ("!(b1 & b2)", &[1, 2][..]),
        ] {
            assert!(!holds_after_nothing(text, start), "{text} fails first");
            assert!(holds_after(text, start), "{text}");
        }
    }

    /// Whether `text` holds with `start` alone set.
    fn holds_after_nothing(text: &str, start: &[u16]) -> bool {
        let pilot = Bits(start.iter().map(|&n| bit(n)).collect());
        Gate {
            control_bits: &crate::control::NovaBits,
            pilot: &pilot,
        }
        .allows(&Test::parse(text))
    }
}
