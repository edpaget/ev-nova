//! Nova control-bit (NCB) expressions, parsed into trees.
//!
//! Records keep each expression as the text they store; this module parses
//! that text. A test expression ([`TestExpr`]) decides whether something is
//! available (a mission offered, a ship sold); a set expression
//! ([`SetExpr`]) lists what happens when the player does something (bits
//! set, missions started, outfits granted). [`check_expressions`] parses
//! every expression field of a [`GameData`](crate::GameData) and reports the
//! ones that do not parse; [`EXPR_FIELDS`] lists those fields.
//!
//! A malformed expression is always an error: nothing is skipped, so a gap
//! in the grammar shows up in the stock test rather than as a quietly wrong
//! tree. Whoever evaluates the trees treats an expression that failed to
//! parse as never satisfied (a test) or as doing nothing (a set): the safe
//! direction, where a broken mission is not offered rather than offered out
//! of order.
//!
//! # Sources
//!
//! The Nova Bible ("A quick word about control bits and scripting")
//! describes both grammars. The original's evaluator, in the 1.0.10 Mac OS X
//! binary (`EV Nova.app/Contents/MacOS/EV Nova`, i386), settles what the
//! Bible leaves open, and where the two disagree this module follows the
//! binary. The addresses below are in that binary.
//!
//! # Test expressions
//!
//! `_EvalMissionBitTestString` @0x9959e returns true for a blank field. A
//! field whose first character is not one of `b ( ! p g o e` (either case)
//! is false outright, without being evaluated, so a leading space or `[`
//! makes the whole test false; this module rejects such a test
//! ([`ParseErrorKind::BadStart`]). It then copies the text, putting a space
//! between `((`, and calls `_EvalTestExp` @0x15cd4, which reads tokens with
//! `_GetToken` @0x147e9 and is true only if the result is exactly 1.
//!
//! The operands, each a letter in either case followed at once by its
//! number:
//!
//! - `Bxxx`: bit xxx, 0-9999. A larger number is false (0x149fb).
//! - `Pxxx`: true when the game is registered, or unregistered for fewer
//!   than xxx days (0x14911).
//! - `G`: true when the player is male (gender 1; 0x14962). It takes no
//!   number.
//! - `Oxxx`: true when the player has outfit xxx (128-639), counting
//!   deployed fighters (0x14a83).
//! - `Exxx`: true when the player has explored system xxx (128-2175;
//!   0x14b25).
//!
//! There is no random test: `_GetToken` knows only these five letters, and
//! nothing in `_EvalTestExp` calls `_Rand`. Randomness lives in the set
//! grammar's `R(...)`.
//!
//! The operators are `!`, `&`, `|` and parentheses. A run of `&` or of `|`
//! reads as one operator (`&&` is `&`; 0x14879). Spaces are skipped
//! (0x15d9c); any other character `_GetToken` does not know, a tab among
//! them, reads as `?` and is ignored. This module accepts spaces and tabs
//! and rejects every other unknown character.
//!
//! `!` negates the next operand or parenthesised group: it sets a flag the
//! next operand or group clears (0x15f58, 0x15e06), so `!!b1` is `!b1`, and
//! this module rejects the doubled form.
//!
//! **`&` and `|` have no precedence and do not chain.** `_EvalTestExp` keeps
//! the result so far and the value of the last operand; each `&` or `|` token
//! *replaces* the result with that last operand's value (0x15f5e, 0x15f6f)
//! before the next operand folds in. So in one group only the last operator
//! and its two operands count: `b1 & b2 & b3` evaluates as `b2 & b3`, and
//! `b1 & b2 | b3` as `b2 | b3`. This is the Bible's "unpredictable"
//! warning, made exact. No stock expression has more than one operator in a
//! group (every chain is parenthesised), so this module allows at most one
//! binary operator per group and rejects a second
//! ([`ParseErrorKind::ChainedOperators`]); [`TestExpr::And`] and
//! [`TestExpr::Or`] are binary.
//!
//! A bare number reads as a `#` token (0x14b5d), which only a comparison
//! (below) uses; anywhere else `_EvalTestExp` skips it. As the right
//! operand of `&` or `|` that leaves the left side standing: the stock
//! `mïsn` 428 has `(b50 | 467)`, a slip for `b467`, which evaluates as
//! `b50`. This module accepts a bare number there, and only there, and
//! drops it and its operator from the tree. (As a left operand it would
//! read as false, not skipped; this module rejects it.)
//!
//! The binary also has an undocumented count comparison. `[ ... ]` is a
//! group that returns how many of its operands are true (0x15e5c), and
//! `<`, `>` or `=` followed by a number compares the value before it with
//! that number (0x15f05). It works only when the count is the whole of a
//! group: anywhere else the comparison reads the wrong value, and at the
//! start of a field the leading `[` makes the field false. And because the
//! end of a group is found by counting brackets from its first character, a
//! group opening `([` ends at the `]`; the binary's `((` fix-up does not
//! cover `([`. So the one spelling that works is `( [b1 b2 b3] > 1)`, with
//! a space after the `(`, and that is the only one this module accepts
//! ([`TestExpr::Count`]). No stock expression uses it.
//!
//! # Set expressions
//!
//! `_EvalMissionBitSetString` @0x99dc5 does nothing for a blank field and
//! otherwise calls `_EvalSetExp` @0x150fc. That walks the text one character
//! at a time, upper-cased, through a jump table at 0xdd204: a letter
//! records a pending operator and clears the number, digits build the
//! number, and any other character (a space, a tab, a parenthesis, the end
//! of the text) runs the pending operator through a second table at
//! 0xdd2fc. The operators, which match the Bible's list exactly:
//!
//! - `bxxx` set, `!bxxx` clear, `^bxxx` toggle bit xxx (0-9999; 0x1524a,
//!   0x15271, 0x15298).
//! - `Axxx` abort, `Fxxx` fail, `Sxxx` start mission xxx (128-1127; 0x152c1,
//!   0x15353, 0x153b9).
//! - `Gxxx` grant, `Dxxx` remove one of outfit xxx (128-639; 0x1541d,
//!   0x1544b).
//! - `Cxxx`, `Exxx`, `Hxxx` change the player's ship to xxx (128-895),
//!   keeping the outfits, adding the new ship's defaults, or dropping the
//!   outfits that are not persistent (0x15493).
//! - `Mxxx` move to system xxx at its first stellar, `Nxxx` keeping the
//!   ship's position (128-2175; 0x1570e, 0x158d9).
//! - `Kxxx` activate, `Lxxx` deactivate rank xxx (128-255; 0x159b2,
//!   0x159e8).
//! - `Pxxx` play sound xxx, any ID (0x15a24).
//! - `Yxxx` destroy, `Uxxx` regenerate stellar xxx (128-2175; 0x15a38,
//!   0x15aeb).
//! - `Txxx` rename the player's ship from `STR#` xxx; `Qxxx` make the player
//!   leave the stellar with a message from `STR#` xxx (any ID; 0x15b25,
//!   0x15bee).
//! - `Xxxx` mark system xxx explored (128-2175; 0x15c71).
//!
//! The original ignores an operator whose number is out of its range; this
//! module rejects it, as it rejects a missing number (which the original
//! reads as 0).
//!
//! `!` and `^` record their own pending operator, which a following `b`
//! keeps, so they mean something only before `b`; this module rejects them
//! before anything else ([`ParseErrorKind::NegatedNonBit`]).
//!
//! `R(<op> <op>)` (0x1519a) draws 0 or 1 with `_Rand(2)`, and the separator
//! check at 0x151f7 skips whichever operator would run as number 0 or 1
//! after it. Skipping the first works by stepping over the character after
//! the `(`, so the spelling is fragile, and this module accepts only the
//! spelling where it works: `R(` with nothing between, then two operators
//! that are not `R`, separated by exactly one space or tab, the first a
//! single letter with its number (a leading `!` or `^` would be stepped
//! over alone, turning a clear into a set), then `)`. A third operator would
//! always run, and a second `R` would overwrite the first's draw.
//!
//! An operator written straight after another's number, with no separator,
//! replaces it: the letter overwrites the pending operator before it runs.
//! The stock `mïsn` 868 has `S862S863`, which starts only mission 863. This
//! module accepts that spelling outside `R(...)` for every letter but `b`
//! and `R` (a `b` there keeps the pending operator and only resets its
//! number), and drops the replaced operator from the tree, as the original
//! does.
//!
//! The Bible says "No parentheses are supported for set expressions";
//! outside `R(...)` this module rejects them.
//!
//! # Extending the grammars
//!
//! Each grammar maps its letters in one table (`test::operand_spec`,
//! `set::letter_spec`), so a new operator is one entry and one variant, and
//! the form of an operand (a letter, then its number) is read in one place
//! (`token::Cursor::operand`), so a longer spelling, such as a named flag,
//! can be tried before it without touching the letters. An extension must
//! use a spelling no stock expression can contain, so that it never changes
//! what an existing expression means.

mod check;
mod set;
mod test;
mod token;

use std::fmt;

use serde::{Deserialize, Serialize};

pub use check::{EXPR_FIELDS, ExprError, ExprField, ExprKind, check_expressions};
pub use set::{BitWrite, SetExpr, SetOp, SetOpKind};
pub use test::{Comparison, CountTerm, TestExpr, TestOperand};

/// A control bit, 0-9999.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct Bit(u16);

impl Bit {
    /// The highest bit, 9999.
    pub const MAX: u16 = 9999;

    /// Bit `n`, or `None` past [`Bit::MAX`].
    #[must_use]
    pub fn new(n: u16) -> Option<Self> {
        (n <= Self::MAX).then_some(Self(n))
    }

    /// The bit's number.
    #[must_use]
    pub fn get(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for Bit {
    type Error = String;

    fn try_from(n: u16) -> Result<Self, String> {
        Self::new(n).ok_or_else(|| format!("bit {n} is past {}", Self::MAX))
    }
}

impl From<Bit> for u16 {
    fn from(bit: Bit) -> u16 {
        bit.0
    }
}

/// Why an expression did not parse, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The byte offset into the expression's text.
    pub at: usize,
    /// What was wrong there.
    pub kind: ParseErrorKind,
}

/// What was wrong with an expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// A letter that is not a test operand.
    UnknownOperand(char),
    /// A letter that is not a set operator.
    UnknownOperator(char),
    /// A character the grammar does not allow here.
    Unexpected(char),
    /// An operand or operator letter with no number after it.
    MissingNumber,
    /// A number outside its range (`value` saturates at `u32::MAX`).
    NumberOutOfRange {
        /// The number as written.
        value: u32,
        /// The lowest number allowed.
        min: u16,
        /// The highest number allowed.
        max: u16,
    },
    /// An opening `(` or `[` that is never closed.
    Unclosed(char),
    /// A closing `)` or `]` with nothing open.
    UnexpectedClose(char),
    /// Nothing where an operand belongs.
    MissingOperand,
    /// Two operands with no `&` or `|` between them.
    MissingOperator,
    /// An `&` or `|` with no operand after it.
    DanglingOperator,
    /// A second `&` or `|` in one group.
    ChainedOperators,
    /// A test whose first character makes the original treat it as false.
    BadStart(char),
    /// A count `[...]` that is not the whole of a group opened by `( `.
    CountOutsideGroup,
    /// A count with no `<`, `>` or `=` and number after it.
    MissingComparison,
    /// An `R` not written `R(<op> <op>)`.
    BadRandom,
    /// A `!` or `^` before something other than `b`.
    NegatedNonBit,
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOperand(c) => write!(f, "unknown test operand `{c}`"),
            Self::UnknownOperator(c) => write!(f, "unknown set operator `{c}`"),
            Self::Unexpected(c) => write!(f, "unexpected `{c}`"),
            Self::MissingNumber => f.write_str("missing number"),
            Self::NumberOutOfRange { value, min, max } => {
                write!(f, "number {value} out of range {min}-{max}")
            }
            Self::Unclosed(c) => write!(f, "unclosed `{c}`"),
            Self::UnexpectedClose(c) => write!(f, "unmatched `{c}`"),
            Self::MissingOperand => f.write_str("missing operand"),
            Self::MissingOperator => f.write_str("missing `&` or `|` between operands"),
            Self::DanglingOperator => f.write_str("operator with no operand after it"),
            Self::ChainedOperators => f.write_str(
                "second `&` or `|` in one group (the original evaluates only the last; \
                 add parentheses)",
            ),
            Self::BadStart(c) => write!(f, "a test starting with `{c}` is always false"),
            Self::CountOutsideGroup => {
                f.write_str("a count `[...]` must be the whole of a group opened by `( `")
            }
            Self::MissingComparison => {
                f.write_str("a count needs `<`, `>` or `=` and a number after it")
            }
            Self::BadRandom => f.write_str("R must be written `R(<op> <op>)`"),
            Self::NegatedNonBit => f.write_str("`!` and `^` apply only to `b`"),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.kind, self.at)
    }
}

impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_run_from_0_to_9999() {
        assert_eq!(Bit::new(0).map(Bit::get), Some(0));
        assert_eq!(Bit::new(9999).map(Bit::get), Some(9999));
        assert_eq!(Bit::new(10000), None);
    }

    #[test]
    fn a_bit_serializes_as_its_number_and_rejects_one_past_the_range() {
        let bit = Bit::new(42).expect("in range");
        assert_eq!(serde_json::to_string(&bit).expect("serializes"), "42");
        assert_eq!(serde_json::from_str::<Bit>("42").expect("parses"), bit);
        assert!(serde_json::from_str::<Bit>("10000").is_err());
    }

    #[test]
    fn a_parse_error_names_what_and_where() {
        let err = ParseError {
            at: 4,
            kind: ParseErrorKind::UnknownOperator('Z'),
        };
        assert_eq!(err.to_string(), "unknown set operator `Z` at byte 4");
        let range = ParseErrorKind::NumberOutOfRange {
            value: 10000,
            min: 0,
            max: 9999,
        };
        assert_eq!(range.to_string(), "number 10000 out of range 0-9999");
    }
}
