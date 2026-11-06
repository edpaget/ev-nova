//! `otool -v -s __TEXT __cstring`: each string and its address.
//!
//! otool prints a string's bytes raw, escaping control bytes as `\n`, `\t`
//! and the like or as three octal digits (`\001`). The std adapter reads
//! the output one char per byte, so a string's length in bytes is the
//! count of its chars with each escape counted once.

/// One C string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CString {
    /// Its address.
    pub addr: u32,
    /// Its text, as otool printed it (escapes kept).
    pub text: String,
    /// Its length in bytes, without the NUL.
    pub len: u32,
}

/// What an address inside the string section points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StringRef<'a> {
    /// Inside a listed string (or on its NUL), `offset` bytes in.
    Listed {
        /// The string.
        string: &'a CString,
        /// Bytes from its start.
        offset: u32,
    },
    /// Inside the section, but in no listed string.
    Unlisted,
}

/// The listed strings, sorted by address.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CStrings {
    strings: Vec<CString>,
}

impl CStrings {
    /// The strings, sorted by address.
    #[must_use]
    pub fn strings(&self) -> &[CString] {
        &self.strings
    }

    /// What `addr` points at, or `None` outside the section (before the
    /// first string or past the last one's NUL).
    ///
    /// A string covers `addr .. addr + len + 1`: its text and its NUL. An
    /// address in the section that no string covers (alignment padding,
    /// a string otool didn't list) is [`StringRef::Unlisted`].
    #[must_use]
    pub fn string_at(&self, addr: u32) -> Option<StringRef<'_>> {
        let first = self.strings.first()?;
        let last = self.strings.last()?;
        if addr < first.addr || u64::from(addr) >= end(last) {
            return None;
        }
        let at = self.strings.partition_point(|s| s.addr <= addr);
        let string = &self.strings[at - 1];
        if u64::from(addr) < end(string) {
            Some(StringRef::Listed {
                string,
                offset: addr - string.addr,
            })
        } else {
            Some(StringRef::Unlisted)
        }
    }
}

/// One past a string's NUL.
fn end(s: &CString) -> u64 {
    u64::from(s.addr) + u64::from(s.len) + 1
}

/// Parses a `__cstring` listing.
#[must_use]
pub fn parse(listing: &str) -> CStrings {
    let mut strings: Vec<CString> = listing
        .lines()
        .filter_map(|line| {
            let addr = super::hex(line.get(..8)?)?;
            let rest = &line[8..];
            let text = rest
                .strip_prefix("  ")
                .or_else(|| rest.strip_prefix(' '))
                .unwrap_or(rest);
            Some(CString {
                addr,
                text: text.to_owned(),
                len: byte_len(text),
            })
        })
        .collect();
    strings.sort_by_key(|s| s.addr);
    CStrings { strings }
}

/// The length in bytes of a string otool printed with escapes.
fn byte_len(text: &str) -> u32 {
    let mut chars = text.chars().peekable();
    let mut len = 0;
    while let Some(c) = chars.next() {
        if c == '\\' {
            if chars.peek().is_some_and(|d| d.is_digit(8)) {
                // Three octal digits.
                for _ in 0..3 {
                    chars.next_if(|d| d.is_digit(8));
                }
            } else {
                chars.next();
            }
        }
        len += 1;
    }
    len
}

/// The value of the first byte, however otool printed it (a three-digit
/// octal escape, a named escape or a raw char), and the text after it.
fn first_byte(text: &str) -> Option<(u32, &str)> {
    let mut chars = text.chars();
    let c = chars.next()?;
    if c != '\\' {
        return Some((u32::from(c), chars.as_str()));
    }
    let escaped = chars.as_str();
    if let Some(digits) = escaped.get(..3)
        && digits.chars().all(|d| d.is_digit(8))
    {
        return Some((u32::from_str_radix(digits, 8).ok()?, &escaped[3..]));
    }
    let value = match escaped.chars().next()? {
        'a' => 0x07,
        'b' => 0x08,
        't' => 0x09,
        'n' => 0x0a,
        'v' => 0x0b,
        'f' => 0x0c,
        'r' => 0x0d,
        '\\' => u32::from(b'\\'),
        _ => return None,
    };
    Some((value, &escaped[1..]))
}

impl CString {
    /// The string's text after a Pascal length byte, when its first byte
    /// (an octal escape, a named escape or a raw char) equals the length
    /// of the rest.
    #[must_use]
    pub fn pascal(&self) -> Option<&str> {
        let (n, rest) = first_byte(&self.text)?;
        (n == byte_len(rest)).then_some(rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::fixture::CSTRINGS;

    fn listed<'a>(r: Option<&StringRef<'a>>) -> Option<(&'a str, u32)> {
        match r {
            Some(StringRef::Listed { string, offset }) => Some((string.text.as_str(), *offset)),
            _ => None,
        }
    }

    #[test]
    fn reads_each_string_after_the_header() {
        let s = parse(CSTRINGS);
        let got: Vec<_> = s
            .strings()
            .iter()
            .map(|c| (c.addr, c.text.as_str(), c.len))
            .collect();
        assert_eq!(
            got,
            [
                (0xd8200, "Hello world", 11),
                (0xd820c, "\\005Pilot", 6),
                (0xd8220, "Line\\n", 5),
                (0xd8230, "\\aCommand", 8),
                (0xd8240, "!Thirty-three bytes after the bang", 34)
            ]
        );
    }

    #[test]
    fn keeps_trailing_spaces_and_reads_empty_strings() {
        let s = parse("/x:\nContents of (__TEXT,__cstring) section\n000d8160  \n000d8161  , \n");
        let got: Vec<_> = s
            .strings()
            .iter()
            .map(|c| (c.text.as_str(), c.len))
            .collect();
        assert_eq!(got, [("", 0), (", ", 2)]);
    }

    #[test]
    fn counts_each_escape_as_one_byte() {
        assert_eq!(byte_len("\\0010"), 2);
        assert_eq!(byte_len("a\\tb\\\\"), 4);
        assert_eq!(byte_len("plain"), 5);
    }

    #[test]
    fn an_address_at_or_inside_a_string_resolves_to_it() {
        let s = parse(CSTRINGS);
        assert_eq!(
            listed(s.string_at(0xd8200).as_ref()),
            Some(("Hello world", 0))
        );
        assert_eq!(
            listed(s.string_at(0xd8203).as_ref()),
            Some(("Hello world", 3))
        );
    }

    #[test]
    fn a_string_covers_its_nul_and_no_further() {
        let s = parse(CSTRINGS);
        // "Line\n" is five bytes at 0xd8220: its NUL is at 0xd8225.
        assert_eq!(listed(s.string_at(0xd8225).as_ref()), Some(("Line\\n", 5)));
        assert_eq!(s.string_at(0xd8226), Some(StringRef::Unlisted));
        // The last string, 34 bytes at 0xd8240, has its NUL at 0xd8262.
        assert!(listed(s.string_at(0xd8262).as_ref()).is_some());
        assert_eq!(s.string_at(0xd8263), None);
    }

    fn pascal(text: &str) -> Option<&str> {
        let line = format!("000d8000  {text}\n");
        let s = parse(&line);
        let rest = s.strings()[0].pascal()?.len();
        Some(&text[text.len() - rest..])
    }

    #[test]
    fn a_length_byte_printed_as_an_octal_escape_is_pascal() {
        assert_eq!(pascal("\\005Pilot"), Some("Pilot"));
        assert_eq!(pascal("\\004Pilot"), None);
    }

    #[test]
    fn a_length_byte_printed_as_a_named_escape_is_pascal() {
        for (text, rest) in [
            ("\\aCommand", "Command"),
            ("\\bCommands", "Commands"),
            ("\\tCommanded", "Commanded"),
            ("\\n0123456789", "0123456789"),
            ("\\v0123456789a", "0123456789a"),
            ("\\f0123456789ab", "0123456789ab"),
            ("\\r0123456789abc", "0123456789abc"),
        ] {
            assert_eq!(pascal(text), Some(rest), "{text}");
        }
        assert_eq!(pascal("\\aPilot"), None);
        // A backslash byte (92) before 92 bytes.
        let long = "x".repeat(92);
        assert_eq!(pascal(&format!("\\\\{long}")), Some(long.as_str()));
    }

    #[test]
    fn a_length_byte_printed_raw_is_pascal() {
        assert_eq!(
            pascal("!Thirty-three bytes after the bang"),
            Some("Thirty-three bytes after the bang")
        );
        assert_eq!(pascal("Hello world"), None);
        assert_eq!(pascal(""), None);
    }

    #[test]
    fn the_fixture_s_pascal_strings_read_without_their_length_byte() {
        let s = parse(CSTRINGS);
        let got: Vec<_> = s.strings().iter().map(CString::pascal).collect();
        assert_eq!(
            got,
            [
                None,
                Some("Pilot"),
                None,
                Some("Command"),
                Some("Thirty-three bytes after the bang")
            ]
        );
    }

    #[test]
    fn an_address_in_a_gap_after_a_string_ends_is_unlisted() {
        let s = parse(CSTRINGS);
        // "\005Pilot" (six bytes at 0xd820c) ends with its NUL at 0xd8212;
        // the next string starts at 0xd8220.
        assert_eq!(
            listed(s.string_at(0xd8212).as_ref()),
            Some(("\\005Pilot", 6))
        );
        assert_eq!(s.string_at(0xd8213), Some(StringRef::Unlisted));
        assert_eq!(s.string_at(0xd8216), Some(StringRef::Unlisted));
    }

    #[test]
    fn an_address_before_the_first_string_is_outside() {
        let s = parse(CSTRINGS);
        assert_eq!(s.string_at(0xd81ff), None);
        assert_eq!(parse("").string_at(0), None);
    }
}
