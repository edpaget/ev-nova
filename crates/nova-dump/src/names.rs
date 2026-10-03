//! Filesystem-safe directory and file names for exported resources.

use nova_rsrc::ResType;

/// The longest resource name kept in a file name, in characters.
const MAX_NAME_CHARS: usize = 64;

/// The directory or file stem for resource type `ty`: its code read as Mac
/// Roman (`shïp`, `rlëD`, `STR#`), with every character other than a
/// letter, a digit or `#` replaced by `_` (`snd ` becomes `snd_`).
///
/// Every type `nova-dump` exports gets a distinct name this way; the tests
/// check it.
#[must_use]
pub fn type_component(ty: ResType) -> String {
    ty.to_string()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '#' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The file stem for resource `id` named `name`: `128`, or `128 Shuttle`.
///
/// The name loses what a file name cannot safely hold: `/`, `\`, `:` and
/// control characters become `_`, as does a leading `.`; it is cut to 64
/// characters and trailing spaces and dots are trimmed. The ID keeps stems
/// unique, so the name need not be.
#[must_use]
pub fn file_stem(id: i16, name: Option<&str>) -> String {
    let name = name.map(sanitise).unwrap_or_default();
    if name.is_empty() {
        id.to_string()
    } else {
        format!("{id} {name}")
    }
}

fn sanitise(name: &str) -> String {
    let cut: String = name
        .chars()
        .take(MAX_NAME_CHARS)
        .enumerate()
        .map(|(at, c)| {
            let unsafe_char = matches!(c, '/' | '\\' | ':') || c.is_control();
            if unsafe_char || (at == 0 && c == '.') {
                '_'
            } else {
                c
            }
        })
        .collect();
    cut.trim_end_matches([' ', '.']).to_owned()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use nova_data::graphics::RLED;
    use nova_data::{OUT_OF_SCOPE, TYPES};
    use nova_rsrc::ResType;

    use super::*;

    /// Whether `name` is one plain path component on every common file
    /// system: non-empty, no separators or control characters, not hidden,
    /// and no trailing space or dot.
    fn is_safe(name: &str) -> bool {
        !name.is_empty()
            && !name.starts_with('.')
            && !name.ends_with([' ', '.'])
            && !name
                .chars()
                .any(|c| matches!(c, '/' | '\\' | ':') || c.is_control())
    }

    #[test]
    fn every_exported_type_has_a_distinct_safe_name() {
        let exported: Vec<ResType> = TYPES.iter().chain(OUT_OF_SCOPE).copied().collect();
        assert!(exported.contains(&RLED));
        let names: BTreeSet<String> = exported.iter().map(|&ty| type_component(ty)).collect();
        assert_eq!(names.len(), exported.len(), "{names:?}");
        for name in &names {
            assert!(is_safe(name), "{name:?}");
        }
    }

    #[test]
    fn type_names_read_as_their_mac_roman_code() {
        let name = |code| type_component(ResType::from_mac_roman(code).expect("Mac Roman"));
        assert_eq!(name("shïp"), "shïp");
        assert_eq!(name("rlëD"), "rlëD");
        assert_eq!(name("STR#"), "STR#");
        assert_eq!(name("PICT"), "PICT");
        assert_eq!(name("STR "), "STR_");
        assert_eq!(name("snd "), "snd_");
    }

    #[test]
    fn a_file_stem_is_the_id_then_the_name() {
        assert_eq!(file_stem(128, None), "128");
        assert_eq!(file_stem(-5, None), "-5");
        assert_eq!(file_stem(128, Some("Shuttle")), "128 Shuttle");
        assert_eq!(file_stem(128, Some("")), "128");
    }

    #[test]
    fn names_lose_what_a_file_name_cannot_hold() {
        assert_eq!(file_stem(1, Some("a/b\\c:d")), "1 a_b_c_d");
        assert_eq!(file_stem(1, Some("tab\there\n")), "1 tab_here_");
        assert_eq!(file_stem(1, Some(".hidden")), "1 _hidden");
        assert_eq!(file_stem(1, Some("ends. . ")), "1 ends");
        assert_eq!(file_stem(1, Some(" . ")), "1");
        assert_eq!(file_stem(1, Some("Ünïcode ok")), "1 Ünïcode ok");
    }

    #[test]
    fn long_names_are_cut_at_64_characters() {
        let long = "é".repeat(70);
        let stem = file_stem(7, Some(&long));
        assert_eq!(stem, format!("7 {}", "é".repeat(64)));
        let exact = "x".repeat(64);
        assert_eq!(file_stem(7, Some(&exact)), format!("7 {exact}"));
        // Trailing dots are trimmed after cutting, so none survive.
        let dotted = format!("{}.{}", "x".repeat(63), "y");
        assert_eq!(file_stem(7, Some(&dotted)), format!("7 {}", "x".repeat(63)));
    }

    #[test]
    fn every_stem_is_safe() {
        for name in ["a/b", ".x", "x. ", "\u{7f}", " ", "...", "ok"] {
            let stem = file_stem(-1, Some(name));
            assert!(is_safe(&stem), "{name:?} -> {stem:?}");
        }
    }
}
