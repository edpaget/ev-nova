//! The reading shared by both grammars: a cursor over the text, whitespace,
//! and the one operand form, a letter followed by its number.

use std::ops::RangeInclusive;

use super::{ParseError, ParseErrorKind};

/// How one operand letter reads: the range of its number (`None` for a
/// letter that takes none, such as the gender test `G`) and what it builds.
pub(super) struct Spec<T> {
    pub range: Option<RangeInclusive<u16>>,
    pub build: fn(u16) -> T,
}

/// A position in an expression's text.
pub(super) struct Cursor<'a> {
    src: &'a str,
    pos: usize,
}

/// Spaces and tabs separate the parts of an expression.
pub(super) fn is_space(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}

impl<'a> Cursor<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    /// The byte offset reached.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// The byte at the cursor, if any.
    pub fn peek(&self) -> Option<u8> {
        self.src.as_bytes().get(self.pos).copied()
    }

    /// Steps over one byte (always an ASCII one).
    pub fn bump(&mut self) {
        self.pos += 1;
    }

    /// Steps over spaces and tabs; whether there were any.
    pub fn skip_space(&mut self) -> bool {
        let start = self.pos;
        while self.peek().is_some_and(is_space) {
            self.bump();
        }
        self.pos > start
    }

    /// An error of `kind` at the cursor.
    pub fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError { at: self.pos, kind }
    }

    /// The character at the cursor, which must not be the end.
    pub fn char(&self) -> char {
        self.src[self.pos..].chars().next().expect("not at the end")
    }

    /// An [`ParseErrorKind::Unexpected`] error for the character at the
    /// cursor.
    pub fn unexpected(&self) -> ParseError {
        self.error(ParseErrorKind::Unexpected(self.char()))
    }

    /// Reads one operand: an ASCII letter of either case, looked up
    /// (upper-cased) in `table`, then the number its spec asks for. A letter
    /// not in the table, or any other character, is an error of the kind
    /// `unknown` makes; the end of the text is a missing operand.
    pub fn operand<T>(
        &mut self,
        table: fn(u8) -> Option<Spec<T>>,
        unknown: fn(char) -> ParseErrorKind,
    ) -> Result<T, ParseError> {
        if self.peek().is_none() {
            return Err(self.error(ParseErrorKind::MissingOperand));
        }
        let spec = self
            .peek()
            .filter(u8::is_ascii_alphabetic)
            .and_then(|letter| table(letter.to_ascii_uppercase()))
            .ok_or_else(|| self.error(unknown(self.char())))?;
        self.bump();
        match spec.range {
            None => Ok((spec.build)(0)),
            Some(range) => Ok((spec.build)(self.number(range)?)),
        }
    }

    /// Reads a decimal number in `range`.
    pub fn number(&mut self, range: RangeInclusive<u16>) -> Result<u16, ParseError> {
        let start = self.pos;
        let mut value: u32 = 0;
        while let Some(digit) = self.peek().filter(u8::is_ascii_digit) {
            value = value
                .saturating_mul(10)
                .saturating_add(u32::from(digit - b'0'));
            self.bump();
        }
        if self.pos == start {
            return Err(self.error(ParseErrorKind::MissingNumber));
        }
        u16::try_from(value)
            .ok()
            .filter(|n| range.contains(n))
            .ok_or(ParseError {
                at: start,
                kind: ParseErrorKind::NumberOutOfRange {
                    value,
                    min: *range.start(),
                    max: *range.end(),
                },
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(letter: u8) -> Option<Spec<(char, u16)>> {
        match letter {
            b'B' => Some(Spec {
                range: Some(0..=9999),
                build: |n| ('B', n),
            }),
            b'G' => Some(Spec {
                range: None,
                build: |_| ('G', 0),
            }),
            _ => None,
        }
    }

    fn read(src: &str) -> (Result<(char, u16), ParseError>, usize) {
        let mut cursor = Cursor::new(src);
        let result = cursor.operand(table, ParseErrorKind::UnknownOperand);
        (result, cursor.pos())
    }

    #[test]
    fn an_operand_is_a_letter_of_either_case_then_its_number() {
        assert_eq!(read("b12 rest"), (Ok(('B', 12)), 3));
        assert_eq!(read("B0012"), (Ok(('B', 12)), 5));
        assert_eq!(read("g5"), (Ok(('G', 0)), 1));
    }

    #[test]
    fn the_number_must_be_there_and_in_range() {
        let missing = ParseError {
            at: 1,
            kind: ParseErrorKind::MissingNumber,
        };
        assert_eq!(read("b x"), (Err(missing), 1));
        let past = ParseErrorKind::NumberOutOfRange {
            value: 10000,
            min: 0,
            max: 9999,
        };
        assert_eq!(read("b10000").0, Err(ParseError { at: 1, kind: past }));
        let huge = ParseErrorKind::NumberOutOfRange {
            value: u32::MAX,
            min: 0,
            max: 9999,
        };
        assert_eq!(
            read("b99999999999999").0,
            Err(ParseError { at: 1, kind: huge })
        );
    }

    #[test]
    fn a_number_below_its_range_is_out_of_it() {
        let mut cursor = Cursor::new("127");
        let low = ParseErrorKind::NumberOutOfRange {
            value: 127,
            min: 128,
            max: 639,
        };
        assert_eq!(
            cursor.number(128..=639),
            Err(ParseError { at: 0, kind: low })
        );
        assert_eq!(Cursor::new("128").number(128..=639), Ok(128));
        assert_eq!(Cursor::new("639").number(128..=639), Ok(639));
    }

    #[test]
    fn an_unknown_letter_or_non_letter_is_reported_by_the_caller_s_kind() {
        let unknown = |c| ParseError {
            at: 0,
            kind: ParseErrorKind::UnknownOperand(c),
        };
        assert_eq!(read("z1"), (Err(unknown('z')), 0));
        assert_eq!(read("é1"), (Err(unknown('é')), 0));
        assert_eq!(read("51"), (Err(unknown('5')), 0));
        let missing = ParseError {
            at: 0,
            kind: ParseErrorKind::MissingOperand,
        };
        assert_eq!(read(""), (Err(missing), 0));
    }

    #[test]
    fn spaces_and_tabs_are_skipped_and_reported() {
        let mut cursor = Cursor::new(" \t b");
        assert!(cursor.skip_space());
        assert_eq!(cursor.pos(), 3);
        assert!(!cursor.skip_space());
        assert_eq!(cursor.peek(), Some(b'b'));
    }
}
