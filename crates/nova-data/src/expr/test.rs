//! Test expressions: when something is available.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::token::{Cursor, Spec, is_space};
use super::{Bit, ParseError, ParseErrorKind};
use crate::wire::id::{OutfitId, SystemId};

/// The largest plain number an expression may hold: the original reads
/// numbers into a signed 16-bit word.
pub(super) const MAX_NUMBER: u16 = i16::MAX as u16;

/// A parsed test expression. A test that failed to parse is never
/// satisfied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestExpr {
    /// A blank test, which is always true.
    Always,
    /// One operand.
    Operand(TestOperand),
    /// `!`: the negation of an operand or group.
    Not(Box<TestExpr>),
    /// `&`: both sides. A group holds at most one `&` or `|`.
    And(Box<TestExpr>, Box<TestExpr>),
    /// `|`: either side. A group holds at most one `&` or `|`.
    Or(Box<TestExpr>, Box<TestExpr>),
    /// `( [t1 t2 ...] < n)` (or `>` or `=`): how many of the terms are true,
    /// compared with `value`. The binary's undocumented count comparison.
    Count {
        /// The terms counted.
        terms: Vec<CountTerm>,
        /// How the count compares with `value`.
        cmp: Comparison,
        /// The number compared with.
        value: u16,
    },
}

/// A test operand.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestOperand {
    /// `Bxxx`: the bit is set.
    Bit(Bit),
    /// `Pxxx`: the game is registered, or unregistered for fewer than
    /// `days` days.
    Paid {
        /// The days an unregistered game counts as paid for.
        days: u16,
    },
    /// `G`: the player is male.
    Male,
    /// `Oxxx`: the player has at least one of the outfit.
    HasOutfit(OutfitId),
    /// `Exxx`: the player has explored the system.
    Explored(SystemId),
}

/// One term of a count: an operand, perhaps negated.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountTerm {
    /// Whether the term is written `!`.
    pub negated: bool,
    /// The operand.
    pub operand: TestOperand,
}

/// How a count compares with its number.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Comparison {
    /// `<`.
    Less,
    /// `>`.
    Greater,
    /// `=`.
    Equal,
}

impl TestExpr {
    /// Parses a test expression; blank is [`TestExpr::Always`].
    pub fn parse(src: &str) -> Result<Self, ParseError> {
        let Some(&first) = src.as_bytes().first() else {
            return Ok(Self::Always);
        };
        let mut cursor = Cursor::new(src);
        if !matches!(
            first.to_ascii_uppercase(),
            b'B' | b'P' | b'G' | b'O' | b'E' | b'!' | b'('
        ) {
            return Err(cursor.error(ParseErrorKind::BadStart(cursor.char())));
        }
        let expr = group(&mut cursor)?;
        match cursor.peek() {
            None => Ok(expr),
            Some(_) => Err(cursor.error(ParseErrorKind::UnexpectedClose(cursor.char()))),
        }
    }

    /// Every bit the expression tests, wherever it sits.
    #[must_use]
    pub fn reads(&self) -> BTreeSet<Bit> {
        let mut bits = BTreeSet::new();
        self.collect_reads(&mut bits);
        bits
    }

    fn collect_reads(&self, bits: &mut BTreeSet<Bit>) {
        match self {
            Self::Always => {}
            Self::Operand(operand) => bits.extend(operand.bit()),
            Self::Not(inner) => inner.collect_reads(bits),
            Self::And(a, b) | Self::Or(a, b) => {
                a.collect_reads(bits);
                b.collect_reads(bits);
            }
            Self::Count { terms, .. } => bits.extend(terms.iter().filter_map(|t| t.operand.bit())),
        }
    }
}

impl TestOperand {
    /// The bit this operand tests, if it is a bit test.
    fn bit(self) -> Option<Bit> {
        match self {
            Self::Bit(bit) => Some(bit),
            _ => None,
        }
    }
}

/// The test operand letters.
pub(super) fn operand_spec(letter: u8) -> Option<Spec<TestOperand>> {
    let (range, build): (_, fn(u16) -> TestOperand) = match letter {
        b'B' => (Some(0..=Bit::MAX), |n| TestOperand::Bit(Bit(n))),
        b'P' => (Some(0..=MAX_NUMBER), |days| TestOperand::Paid { days }),
        b'G' => (None, |_| TestOperand::Male),
        b'O' => (Some(128..=639), |n| {
            TestOperand::HasOutfit(OutfitId(n as i16))
        }),
        b'E' => (Some(128..=2175), |n| {
            TestOperand::Explored(SystemId(n as i16))
        }),
        _ => return None,
    };
    Some(Spec { range, build })
}

/// Reads one operand.
fn operand(cursor: &mut Cursor<'_>) -> Result<TestOperand, ParseError> {
    cursor.operand(operand_spec, ParseErrorKind::UnknownOperand)
}

/// Reads a group's contents, up to (not over) its closing bracket or the
/// end: one term, or two joined by `&` or `|`.
fn group(cursor: &mut Cursor<'_>) -> Result<TestExpr, ParseError> {
    cursor.skip_space();
    let left = term(cursor)?;
    cursor.skip_space();
    let Some(join) = operator(cursor) else {
        end_of_group(cursor)?;
        return Ok(left);
    };
    let at = cursor.pos();
    while cursor.peek() == Some(join) {
        cursor.bump();
    }
    cursor.skip_space();
    if matches!(cursor.peek(), None | Some(b')' | b']')) {
        return Err(ParseError {
            at,
            kind: ParseErrorKind::DanglingOperator,
        });
    }
    let right = if cursor.peek().is_some_and(|byte| byte.is_ascii_digit()) {
        // A bare number, which the original skips: the left side stands.
        cursor.number(0..=MAX_NUMBER)?;
        None
    } else {
        Some(term(cursor)?)
    };
    cursor.skip_space();
    if operator(cursor).is_some() {
        return Err(cursor.error(ParseErrorKind::ChainedOperators));
    }
    end_of_group(cursor)?;
    let Some(right) = right else {
        return Ok(left);
    };
    let (left, right) = (Box::new(left), Box::new(right));
    Ok(if join == b'&' {
        TestExpr::And(left, right)
    } else {
        TestExpr::Or(left, right)
    })
}

/// The `&` or `|` at the cursor, if there is one.
fn operator(cursor: &Cursor<'_>) -> Option<u8> {
    cursor.peek().filter(|&byte| byte == b'&' || byte == b'|')
}

/// Checks that the group ends here: at a closing bracket or the end.
fn end_of_group(cursor: &Cursor<'_>) -> Result<(), ParseError> {
    match cursor.peek() {
        None | Some(b')' | b']') => Ok(()),
        Some(_) => Err(after_operand(cursor)),
    }
}

/// The error for a character that cannot follow an operand.
fn after_operand(cursor: &Cursor<'_>) -> ParseError {
    match cursor.peek() {
        Some(b'(' | b'!') => cursor.error(ParseErrorKind::MissingOperator),
        Some(letter) if letter.is_ascii_alphabetic() => {
            if operand_spec(letter.to_ascii_uppercase()).is_some() {
                cursor.error(ParseErrorKind::MissingOperator)
            } else {
                cursor.error(ParseErrorKind::UnknownOperand(cursor.char()))
            }
        }
        _ => cursor.unexpected(),
    }
}

/// Reads one term: an operand or parenthesised group, perhaps negated.
fn term(cursor: &mut Cursor<'_>) -> Result<TestExpr, ParseError> {
    if cursor.peek() != Some(b'!') {
        return atom(cursor);
    }
    cursor.bump();
    cursor.skip_space();
    if cursor.peek() == Some(b'!') {
        return Err(cursor.unexpected());
    }
    Ok(TestExpr::Not(Box::new(atom(cursor)?)))
}

/// Reads an operand or a parenthesised group.
fn atom(cursor: &mut Cursor<'_>) -> Result<TestExpr, ParseError> {
    match cursor.peek() {
        Some(b'(') => paren(cursor),
        Some(b'[') => Err(cursor.error(ParseErrorKind::CountOutsideGroup)),
        None | Some(b')' | b']' | b'&' | b'|') => Err(cursor.error(ParseErrorKind::MissingOperand)),
        Some(_) => Ok(TestExpr::Operand(operand(cursor)?)),
    }
}

/// Reads a parenthesised group, from its `(` to its `)`: a group's
/// contents, or a count when the `(` is followed by a space.
fn paren(cursor: &mut Cursor<'_>) -> Result<TestExpr, ParseError> {
    let open = cursor.pos();
    cursor.bump();
    if cursor.peek() == Some(b'[') {
        return Err(cursor.error(ParseErrorKind::CountOutsideGroup));
    }
    cursor.skip_space();
    let inner = if cursor.peek() == Some(b'[') {
        let count = count(cursor)?;
        cursor.skip_space();
        if cursor.peek().is_some_and(|byte| byte != b')') {
            return Err(cursor.error(ParseErrorKind::CountOutsideGroup));
        }
        count
    } else {
        group(cursor)?
    };
    match cursor.peek() {
        Some(b')') => {
            cursor.bump();
            Ok(inner)
        }
        None => Err(ParseError {
            at: open,
            kind: ParseErrorKind::Unclosed('('),
        }),
        Some(_) => Err(cursor.error(ParseErrorKind::UnexpectedClose(cursor.char()))),
    }
}

/// Reads a count, `[t1 t2 ...]` then its comparison and number.
fn count(cursor: &mut Cursor<'_>) -> Result<TestExpr, ParseError> {
    let open = cursor.pos();
    cursor.bump();
    let mut terms = Vec::new();
    loop {
        cursor.skip_space();
        match cursor.peek() {
            Some(b']') => break,
            None => {
                return Err(ParseError {
                    at: open,
                    kind: ParseErrorKind::Unclosed('['),
                });
            }
            Some(b'(' | b'[' | b'&' | b'|') => return Err(cursor.unexpected()),
            Some(first) => {
                let negated = first == b'!';
                if negated {
                    cursor.bump();
                }
                terms.push(CountTerm {
                    negated,
                    operand: operand(cursor)?,
                });
                if !cursor
                    .peek()
                    .is_none_or(|byte| is_space(byte) || byte == b']')
                {
                    return Err(after_operand(cursor));
                }
            }
        }
    }
    if terms.is_empty() {
        return Err(cursor.error(ParseErrorKind::MissingOperand));
    }
    cursor.bump();
    cursor.skip_space();
    let cmp = match cursor.peek() {
        Some(b'<') => Comparison::Less,
        Some(b'>') => Comparison::Greater,
        Some(b'=') => Comparison::Equal,
        _ => return Err(cursor.error(ParseErrorKind::MissingComparison)),
    };
    cursor.bump();
    cursor.skip_space();
    let value = cursor.number(0..=MAX_NUMBER)?;
    Ok(TestExpr::Count { terms, cmp, value })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::ParseErrorKind as K;

    fn parse(src: &str) -> TestExpr {
        TestExpr::parse(src).unwrap_or_else(|err| panic!("{src:?}: {err}"))
    }

    fn fails(src: &str) -> ParseError {
        TestExpr::parse(src).expect_err(src)
    }

    fn err(at: usize, kind: ParseErrorKind) -> ParseError {
        ParseError { at, kind }
    }

    fn bit(n: u16) -> TestExpr {
        TestExpr::Operand(TestOperand::Bit(Bit::new(n).expect("in range")))
    }

    fn not(expr: TestExpr) -> TestExpr {
        TestExpr::Not(Box::new(expr))
    }

    fn and(a: TestExpr, b: TestExpr) -> TestExpr {
        TestExpr::And(Box::new(a), Box::new(b))
    }

    fn or(a: TestExpr, b: TestExpr) -> TestExpr {
        TestExpr::Or(Box::new(a), Box::new(b))
    }

    fn term(negated: bool, n: u16) -> CountTerm {
        CountTerm {
            negated,
            operand: TestOperand::Bit(Bit::new(n).expect("in range")),
        }
    }

    #[test]
    fn a_blank_test_is_always_true() {
        assert_eq!(parse(""), TestExpr::Always);
    }

    #[test]
    fn b_tests_a_bit_from_0_to_9999_in_either_case() {
        assert_eq!(parse("b12"), bit(12));
        assert_eq!(parse("B12"), bit(12));
        assert_eq!(parse("b0"), bit(0));
        assert_eq!(parse("b9999"), bit(9999));
        assert_eq!(parse("b007"), bit(7));
        let past = K::NumberOutOfRange {
            value: 10000,
            min: 0,
            max: 9999,
        };
        assert_eq!(fails("b10000"), err(1, past));
    }

    #[test]
    fn p_tests_the_days_paid_for() {
        assert_eq!(
            parse("P0"),
            TestExpr::Operand(TestOperand::Paid { days: 0 })
        );
        assert_eq!(
            parse("p30"),
            TestExpr::Operand(TestOperand::Paid { days: 30 })
        );
        assert_eq!(
            parse("p32767"),
            TestExpr::Operand(TestOperand::Paid { days: 32767 })
        );
        let past = K::NumberOutOfRange {
            value: 32768,
            min: 0,
            max: 32767,
        };
        assert_eq!(fails("p32768"), err(1, past));
    }

    #[test]
    fn g_tests_the_gender_and_takes_no_number() {
        assert_eq!(parse("G"), TestExpr::Operand(TestOperand::Male));
        assert_eq!(parse("g"), TestExpr::Operand(TestOperand::Male));
        assert_eq!(fails("g1"), err(1, K::Unexpected('1')));
    }

    #[test]
    fn o_tests_an_outfit_from_128_to_639() {
        let outfit = |n| TestExpr::Operand(TestOperand::HasOutfit(OutfitId(n)));
        assert_eq!(parse("o128"), outfit(128));
        assert_eq!(parse("O639"), outfit(639));
        let range = |value| K::NumberOutOfRange {
            value,
            min: 128,
            max: 639,
        };
        assert_eq!(fails("o127"), err(1, range(127)));
        assert_eq!(fails("o640"), err(1, range(640)));
    }

    #[test]
    fn e_tests_an_explored_system_from_128_to_2175() {
        let explored = |n| TestExpr::Operand(TestOperand::Explored(SystemId(n)));
        assert_eq!(parse("e128"), explored(128));
        assert_eq!(parse("E2175"), explored(2175));
        let range = |value| K::NumberOutOfRange {
            value,
            min: 128,
            max: 2175,
        };
        assert_eq!(fails("e127"), err(1, range(127)));
        assert_eq!(fails("e2176"), err(1, range(2176)));
    }

    #[test]
    fn not_negates_the_next_operand_or_group() {
        assert_eq!(parse("!b1"), not(bit(1)));
        assert_eq!(parse("! b1"), not(bit(1)));
        assert_eq!(parse("!(b1 | b2)"), not(or(bit(1), bit(2))));
        assert_eq!(parse("!b1 & b2"), and(not(bit(1)), bit(2)));
        assert_eq!(fails("!!b1"), err(1, K::Unexpected('!')));
    }

    #[test]
    fn and_and_or_join_two_operands_and_runs_of_them_read_as_one() {
        assert_eq!(parse("b1 & b2"), and(bit(1), bit(2)));
        assert_eq!(parse("b1|b2"), or(bit(1), bit(2)));
        assert_eq!(parse("b1 && b2"), and(bit(1), bit(2)));
        assert_eq!(parse("b1 || b2"), or(bit(1), bit(2)));
    }

    #[test]
    fn a_second_operator_in_one_group_is_rejected() {
        assert_eq!(fails("b1 & b2 | b3"), err(8, K::ChainedOperators));
        assert_eq!(fails("b1 & b2 & b3"), err(8, K::ChainedOperators));
        assert_eq!(fails("(b1 | b2 | b3)"), err(9, K::ChainedOperators));
        // Parenthesised, each group holds one operator.
        assert_eq!(parse("(b1 & b2) | b3"), or(and(bit(1), bit(2)), bit(3)));
        assert_eq!(parse("b1 & (b2 | b3)"), and(bit(1), or(bit(2), bit(3))));
    }

    #[test]
    fn parentheses_nest_and_leave_no_node() {
        assert_eq!(parse("(b1)"), bit(1));
        assert_eq!(parse("((b1))"), bit(1));
        assert_eq!(
            parse("((b1 & b2) | (b3 & !b4))"),
            or(and(bit(1), bit(2)), and(bit(3), not(bit(4))))
        );
    }

    #[test]
    fn the_bible_s_examples_parse() {
        assert_eq!(
            parse("b13 & (b15 | !b72)"),
            and(bit(13), or(bit(15), not(bit(72))))
        );
        assert_eq!(
            parse("!(B42 | B53) & b103"),
            and(not(or(bit(42), bit(53))), bit(103))
        );
    }

    #[test]
    fn spaces_and_tabs_separate_anywhere_but_the_start() {
        assert_eq!(parse("b1\t&\tb2"), and(bit(1), bit(2)));
        assert_eq!(parse("( b1 &  b2 ) \t"), and(bit(1), bit(2)));
        assert_eq!(parse("b1 "), bit(1));
        assert_eq!(fails(" b1"), err(0, K::BadStart(' ')));
        assert_eq!(fails("\tb1"), err(0, K::BadStart('\t')));
    }

    #[test]
    fn a_test_must_start_as_the_original_requires() {
        for good in [
            "b1", "B1", "p1", "P1", "g", "G", "o128", "O128", "e128", "E128", "!b1", "(b1)",
        ] {
            assert!(TestExpr::parse(good).is_ok(), "{good}");
        }
        assert_eq!(fails("&b1"), err(0, K::BadStart('&')));
        assert_eq!(fails("x1"), err(0, K::BadStart('x')));
        assert_eq!(fails("[b1] > 0"), err(0, K::BadStart('[')));
    }

    #[test]
    fn a_count_compares_how_many_terms_are_true() {
        let count = |terms, cmp, value| TestExpr::Count { terms, cmp, value };
        assert_eq!(
            parse("( [b1 b2 !b3] > 1)"),
            count(
                vec![term(false, 1), term(false, 2), term(true, 3)],
                Comparison::Greater,
                1
            )
        );
        assert_eq!(
            parse("( [ b1 ]<1 )"),
            count(vec![term(false, 1)], Comparison::Less, 1)
        );
        assert_eq!(
            parse("b9 & !( [b1\tb2] = 2)"),
            and(
                bit(9),
                not(count(
                    vec![term(false, 1), term(false, 2)],
                    Comparison::Equal,
                    2
                ))
            )
        );
    }

    #[test]
    fn a_count_anywhere_but_alone_in_a_spaced_group_is_rejected() {
        assert_eq!(fails("([b1] > 0)"), err(1, K::CountOutsideGroup));
        assert_eq!(fails("b1 & [b2] > 0"), err(5, K::CountOutsideGroup));
        assert_eq!(fails("( [b1] > 0 & b2)"), err(11, K::CountOutsideGroup));
        assert_eq!(fails("( [b1])"), err(6, K::MissingComparison));
        assert_eq!(fails("( [b1] >)"), err(8, K::MissingNumber));
        assert_eq!(fails("( [] > 0)"), err(3, K::MissingOperand));
        assert_eq!(fails("( [b1 (b2)] > 0)"), err(6, K::Unexpected('(')));
        assert_eq!(fails("( [b1&b2] > 0)"), err(5, K::Unexpected('&')));
        assert_eq!(fails("( [b1 b2"), err(2, K::Unclosed('[')));
        assert_eq!(fails("( [b1] > 0"), err(0, K::Unclosed('(')));
    }

    #[test]
    fn unknown_letters_and_stray_characters_are_rejected() {
        assert_eq!(fails("b1 & x2"), err(5, K::UnknownOperand('x')));
        assert_eq!(fails("(2 & b1)"), err(1, K::UnknownOperand('2')));
        assert_eq!(fails("b1 & !2"), err(6, K::UnknownOperand('2')));
        assert_eq!(fails("b1 # b2"), err(3, K::Unexpected('#')));
        assert_eq!(fails("b1x"), err(2, K::UnknownOperand('x')));
        assert_eq!(fails("b1 < 2"), err(3, K::Unexpected('<')));
    }

    #[test]
    fn a_bare_number_as_the_right_operand_is_dropped_with_its_operator() {
        // The stock mïsn 428 has `(b50 | 467)`, a slip for `b467`. The
        // original reads the number as a token only a comparison uses, so
        // the group evaluates as `b50`.
        assert_eq!(
            parse("!(b511 | b515) & !((b50 | 467) | b6666)"),
            and(not(or(bit(511), bit(515))), not(or(bit(50), bit(6666))))
        );
        assert_eq!(parse("b1 && 2 "), bit(1));
        assert_eq!(fails("b1 | 2 | b3"), err(7, K::ChainedOperators));
        assert_eq!(fails("b1 | 2x"), err(6, K::UnknownOperand('x')));
        let past = K::NumberOutOfRange {
            value: 32768,
            min: 0,
            max: 32767,
        };
        assert_eq!(fails("b1 | 32768"), err(5, past));
    }

    #[test]
    fn a_letter_without_its_number_is_rejected() {
        assert_eq!(fails("b"), err(1, K::MissingNumber));
        assert_eq!(fails("b1 & o"), err(6, K::MissingNumber));
        assert_eq!(fails("b 1"), err(1, K::MissingNumber));
    }

    #[test]
    fn unbalanced_parentheses_are_rejected() {
        assert_eq!(fails("(b1"), err(0, K::Unclosed('(')));
        assert_eq!(fails("b1 & ((b2 | b3)"), err(5, K::Unclosed('(')));
        assert_eq!(fails("b1)"), err(2, K::UnexpectedClose(')')));
        assert_eq!(fails("(b1 & b2))"), err(9, K::UnexpectedClose(')')));
        assert_eq!(fails("(b1]"), err(3, K::UnexpectedClose(']')));
        assert_eq!(fails("b1]"), err(2, K::UnexpectedClose(']')));
    }

    #[test]
    fn missing_operands_and_operators_are_rejected() {
        assert_eq!(fails("()"), err(1, K::MissingOperand));
        assert_eq!(fails("!"), err(1, K::MissingOperand));
        assert_eq!(fails("(b1 & )"), err(4, K::DanglingOperator));
        assert_eq!(fails("b1 &"), err(3, K::DanglingOperator));
        assert_eq!(fails("b1 | "), err(3, K::DanglingOperator));
        assert_eq!(fails("(& b1)"), err(1, K::MissingOperand));
        assert_eq!(fails("b1 b2"), err(3, K::MissingOperator));
        assert_eq!(fails("b1 (b2)"), err(3, K::MissingOperator));
        assert_eq!(fails("b1 !b2"), err(3, K::MissingOperator));
        assert_eq!(fails("b1b2"), err(2, K::MissingOperator));
    }

    #[test]
    fn reads_lists_every_bit_tested_wherever_it_sits() {
        let expr = parse("!(b42 | (b7 & !b9)) & (p30 | b42)");
        let bits: Vec<u16> = expr.reads().into_iter().map(Bit::get).collect();
        assert_eq!(bits, [7, 9, 42]);
        let count = parse("( [b3 !b1 g] > 1)");
        let bits: Vec<u16> = count.reads().into_iter().map(Bit::get).collect();
        assert_eq!(bits, [1, 3]);
        assert!(parse("").reads().is_empty());
        assert!(parse("o128 | e128").reads().is_empty());
    }
}
