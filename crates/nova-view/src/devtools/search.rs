//! Searching resources by type, ID or name.
//!
//! The query, type codes and names are all [`fold`]ed, so case and the
//! accents of Mac Roman's Latin letters never matter: `rlëD`, `rleD` and
//! `RLED` are the same type, and "federation" finds "Fédération". The query
//! splits on whitespace into tokens, and an entry matches when every token
//! does. A token matches *structurally* when it is the entry's ID or its
//! whole type code (trailing spaces trimmed, so `snd` is `snd `), and
//! otherwise *by name* when the entry's name contains it. An entry's rank
//! is how many of its tokens matched only by name, so "rled 128" puts
//! `rlëD` 128 first, while "Station 128" still finds a resource named
//! "Station 128" whatever its ID.

/// Lower-cases `text` and maps Mac Roman's accented Latin letters to their
/// base letter (`é` → `e`, `Ÿ` → `y`); every other character is kept.
#[must_use]
pub fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(base_letter)
        .collect()
}

fn base_letter(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'ç' => 'c',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'ÿ' => 'y',
        other => other,
    }
}

/// One folded word of a query, and the ID it spells, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    text: String,
    id: Option<i16>,
}

/// A parsed search query: its folded, whitespace-separated tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    tokens: Vec<Token>,
}

impl Query {
    /// Folds `query` and splits it on whitespace.
    #[must_use]
    pub fn parse(query: &str) -> Self {
        let tokens = fold(query)
            .split_whitespace()
            .map(|text| Token {
                text: text.to_owned(),
                id: text.parse().ok(),
            })
            .collect();
        Self { tokens }
    }

    /// The rank of the entry with folded type code `ty_folded`, ID `id` and
    /// folded name `name_folded` (empty when it has none): how many tokens
    /// matched only by name, or `None` if some token matches neither way.
    #[must_use]
    pub fn rank(&self, ty_folded: &str, id: i16, name_folded: &str) -> Option<u32> {
        let ty = ty_folded.trim_end_matches(' ');
        let mut rank = 0;
        for token in &self.tokens {
            if token.id == Some(id) || token.text == ty {
                continue;
            }
            if !name_folded.contains(&token.text) {
                return None;
            }
            rank += 1;
        }
        Some(rank)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folding_lower_cases_and_strips_mac_roman_accents() {
        let pairs = [
            ("àáâãäåÀÁÂÃÄÅ", "aaaaaaaaaaaa"),
            ("çÇ", "cc"),
            ("èéêëÈÉÊË", "eeeeeeee"),
            ("ìíîïÌÍÎÏ", "iiiiiiii"),
            ("ñÑ", "nn"),
            ("òóôõöøÒÓÔÕÖØ", "oooooooooooo"),
            ("ùúûüÙÚÛÜ", "uuuuuuuu"),
            ("ÿŸ", "yy"),
            ("rlëD", "rled"),
            ("SHÏP", "ship"),
            ("Ÿ", "y"),
        ];
        for (text, folded) in pairs {
            assert_eq!(fold(text), folded, "{text}");
        }
    }

    #[test]
    fn folding_keeps_other_characters() {
        assert_eq!(
            fold("abc XYZ 0123 ™ æ œ ß #-_."),
            "abc xyz 0123 ™ æ œ ß #-_."
        );
        assert_eq!(fold("ÆŒ"), "æœ");
    }

    fn rank(query: &str, ty: &str, id: i16, name: &str) -> Option<u32> {
        Query::parse(query).rank(&fold(ty), id, &fold(name))
    }

    #[test]
    fn the_empty_query_matches_everything_at_rank_zero() {
        assert_eq!(rank("", "PICT", 128, "Anything"), Some(0));
        assert_eq!(rank("   ", "snd ", -1, ""), Some(0));
    }

    #[test]
    fn type_and_id_match_at_rank_zero_however_the_type_is_spelled() {
        for query in ["rlëD 128", "rleD 128", "RLED 128", "128 rled", "rlëd  128"] {
            assert_eq!(rank(query, "rlëD", 128, "Shuttle"), Some(0), "{query}");
        }
        assert_eq!(rank("rled 129", "rlëD", 128, "Shuttle"), None);
        assert_eq!(rank("PICT 128", "rlëD", 128, "Shuttle"), None);
    }

    #[test]
    fn a_type_alone_matches_every_id() {
        assert_eq!(rank("shïp", "shïp", 128, ""), Some(0));
        assert_eq!(rank("SHIP", "shïp", 400, "Kestrel"), Some(0));
    }

    #[test]
    fn a_type_matches_without_its_trailing_spaces() {
        assert_eq!(rank("snd", "snd ", 200, ""), Some(0));
        assert_eq!(rank("str", "STR ", 128, ""), Some(0));
        assert_eq!(rank("str", "STR#", 128, ""), None);
        assert_eq!(rank("str#", "STR#", 128, ""), Some(0));
    }

    #[test]
    fn a_partial_type_does_not_match_structurally() {
        assert_eq!(rank("rle", "rlëD", 128, ""), None);
        assert_eq!(rank("rle", "rlëD", 128, "Burled Oak"), Some(1));
        assert_eq!(rank("pic", "PICT", 128, ""), None);
    }

    #[test]
    fn the_id_matches_exactly() {
        assert_eq!(rank("12", "PICT", 128, ""), None);
        assert_eq!(rank("128", "PICT", 128, ""), Some(0));
        assert_eq!(rank("-1", "PICT", -1, ""), Some(0));
        assert_eq!(rank("-1", "PICT", 1, ""), None);
        assert_eq!(rank("+128", "PICT", 128, ""), Some(0));
    }

    #[test]
    fn a_number_that_is_not_the_id_can_match_the_name() {
        assert_eq!(rank("12", "PICT", 128, "Station 128"), Some(1));
        assert_eq!(rank("station 128", "spöb", 400, "Station 128"), Some(2));
        assert_eq!(rank("station 128", "spöb", 128, "Station 128"), Some(1));
        assert_eq!(rank("99999", "PICT", 128, "Item 99999"), Some(1));
    }

    #[test]
    fn names_match_as_folded_substrings() {
        assert_eq!(rank("federation", "gövt", 128, "Fédération"), Some(1));
        assert_eq!(rank("ERAT", "gövt", 128, "Fédération"), Some(1));
        assert_eq!(rank("fed navy", "gövt", 128, "Federation Navy"), Some(2));
    }

    #[test]
    fn every_token_must_match() {
        assert_eq!(rank("fed klingon", "gövt", 128, "Federation"), None);
        assert_eq!(rank("pict fed", "gövt", 128, "Federation"), None);
        assert_eq!(rank("gövt 129", "gövt", 128, "Federation"), None);
    }

    #[test]
    fn a_token_matching_nothing_gives_none() {
        assert_eq!(rank("zzz", "PICT", 128, "Planet"), None);
    }

    #[test]
    fn a_type_token_matching_a_name_counts_as_structural() {
        // "junk" is jünk's type, so jünk entries rank 0 and others named
        // "junk" rank 1.
        assert_eq!(rank("junk", "jünk", 128, "Junk Food"), Some(0));
        assert_eq!(rank("junk", "dësc", 128, "Junk Food"), Some(1));
    }
}
