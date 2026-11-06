//! `nm -n`: every defined symbol, sorted by address.

/// One defined symbol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    /// Its address.
    pub addr: u32,
    /// nm's type letter (`t`, `T`, `s`, `d`, `b`, ...).
    pub kind: char,
    /// Its name.
    pub name: String,
}

impl Symbol {
    /// Whether it is code (`t`/`T`); the disassembly's labels already
    /// cover those.
    #[must_use]
    pub fn is_text(&self) -> bool {
        matches!(self.kind, 't' | 'T')
    }

    /// Whether it is data: neither code nor an absolute value (`A`, such
    /// as `__mh_execute_header`, which is a load address, not storage).
    #[must_use]
    pub fn is_data(&self) -> bool {
        !self.is_text() && !matches!(self.kind, 'A' | 'a')
    }
}

/// Parses an `nm -n` listing; undefined (`U`) symbols are left out.
#[must_use]
pub fn parse(listing: &str) -> Vec<Symbol> {
    let mut symbols: Vec<Symbol> = listing
        .lines()
        .filter_map(|line| {
            let (addr, rest) = line.split_once(' ')?;
            let addr = super::hex(addr)?;
            let mut chars = rest.chars();
            let kind = chars.next()?;
            let name = chars.as_str().strip_prefix(' ')?;
            (kind != 'U' && !name.is_empty()).then(|| Symbol {
                addr,
                kind,
                name: name.to_owned(),
            })
        })
        .collect();
    symbols.sort_by_key(|s| s.addr);
    symbols
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::fixture::SYMBOLS;

    #[test]
    fn reads_defined_symbols_in_address_order() {
        let symbols = parse(SYMBOLS);
        assert_eq!(symbols.len(), 10);
        assert_eq!(
            symbols[5],
            Symbol {
                addr: 0xdd060,
                kind: 's',
                name: "_kScale".into()
            }
        );
        assert!(symbols[4].is_text() && !symbols[5].is_text());
    }

    #[test]
    fn a_name_may_hold_spaces() {
        let symbols = parse("000cf268 t -[Foo bar:baz:]\n00001000 t +[A b]\n");
        assert_eq!(symbols[0].name, "+[A b]");
        assert_eq!(symbols[1].name, "-[Foo bar:baz:]");
    }
}
