//! Load order: how file names sort.

use std::cmp::Ordering;
use std::ffi::OsStr;

/// Compares two file names the way the classic Mac Finder sorted them:
/// case-insensitively, character by character.
///
/// Each name is converted to text (lossily, for names that are not UTF-8)
/// and every character lower-cased with Unicode case folding
/// ([`char::to_lowercase`], so `É` matches `é`, not only ASCII); the folded
/// characters then compare by code point. Digits are not compared
/// numerically (`Plug 10` sorts before `Plug 2`, as the classic Finder
/// sorted), accents are not stripped and names are not normalized: they
/// compare as the operating system returns them. Names that fold to the same
/// text (`a` and `A` on a case-sensitive volume) are ordered by their raw
/// bytes, so the order is total and deterministic.
#[must_use]
pub fn finder_cmp(a: &OsStr, b: &OsStr) -> Ordering {
    let (text_a, text_b) = (a.to_string_lossy(), b.to_string_lossy());
    let folded_a = text_a.chars().flat_map(char::to_lowercase);
    let folded_b = text_b.chars().flat_map(char::to_lowercase);
    folded_a
        .cmp(folded_b)
        .then_with(|| a.as_encoded_bytes().cmp(b.as_encoded_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmp(a: &str, b: &str) -> Ordering {
        finder_cmp(OsStr::new(a), OsStr::new(b))
    }

    #[test]
    fn case_is_ignored() {
        assert_eq!(cmp("alpha", "Beta"), Ordering::Less);
        assert_eq!(cmp("Beta", "charlie"), Ordering::Less);
        assert_eq!(cmp("Beta", "alpha"), Ordering::Greater);
    }

    #[test]
    fn case_folding_covers_non_ascii_letters() {
        // Folded, these compare on their second characters: b > a. Without
        // folding, É (U+00C9) sorts before é (U+00E9).
        assert_eq!(cmp("Éb", "éa"), Ordering::Greater);
        assert_eq!(cmp("éa", "Éb"), Ordering::Less);
    }

    #[test]
    fn folded_characters_compare_by_code_point() {
        // é is U+00E9, above f.
        assert_eq!(cmp("éa", "fz"), Ordering::Greater);
    }

    #[test]
    fn numbers_are_not_compared_numerically() {
        assert_eq!(cmp("Plug 10", "Plug 2"), Ordering::Less);
    }

    #[test]
    fn names_differing_only_in_case_are_ordered_by_their_bytes() {
        assert_eq!(cmp("Éclair", "éclair"), Ordering::Less);
        assert_eq!(cmp("éclair", "Éclair"), Ordering::Greater);
        assert_eq!(cmp("A", "a"), Ordering::Less);
        assert_eq!(cmp("a", "A"), Ordering::Greater);
    }

    #[test]
    fn equal_names_are_equal() {
        assert_eq!(cmp("Nova Data 1.ndat", "Nova Data 1.ndat"), Ordering::Equal);
        assert_eq!(cmp("", ""), Ordering::Equal);
    }

    #[test]
    fn a_prefix_sorts_first() {
        assert_eq!(cmp("Nova", "nova data"), Ordering::Less);
        assert_eq!(cmp("nova data", "Nova"), Ordering::Greater);
    }

    #[test]
    fn sorting_a_shuffled_list_gives_finder_order() {
        let mut names = vec![
            "charlie", "Plug 2", "beta", "Éclair", "Alpha", "éclair", "Plug 10", "a", "B",
        ];
        names.sort_by(|a, b| cmp(a, b));
        assert_eq!(
            names,
            // Folded `é` (U+00E9) sorts after every ASCII letter.
            [
                "a", "Alpha", "B", "beta", "charlie", "Plug 10", "Plug 2", "Éclair", "éclair"
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_names_compare_by_their_lossy_text_then_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let low = OsStr::from_bytes(b"x\xFEz");
        let high = OsStr::from_bytes(b"x\xFFa");
        // Both read as `x\u{FFFD}…`, so the last character decides.
        assert_eq!(finder_cmp(low, high), Ordering::Greater);
        let same_text = OsStr::from_bytes(b"x\xFF");
        let other_bytes = OsStr::from_bytes(b"x\xFE");
        assert_eq!(finder_cmp(other_bytes, same_text), Ordering::Less);
    }
}
