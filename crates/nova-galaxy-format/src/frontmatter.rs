//! Splits a content file into its YAML front matter and Markdown body, and
//! reads the front matter strictly.

use serde::de::DeserializeOwned;
use serde_saphyr::{DuplicateKeyPolicy, MergeKeyPolicy, Options};

/// Reads `text`'s front matter as a `T` and returns it with the body.
///
/// The front matter sits between a first line `---` and the next `---`
/// line. It is read as strict YAML 1.2: unknown fields, duplicate keys,
/// merge keys and unknown tags are errors, only `true` and `false` are
/// booleans, and an unquoted number or boolean is never taken as text.
/// Errors start with the line number in the file.
pub fn parse<T: DeserializeOwned>(text: &str) -> Result<(T, &str), String> {
    let (yaml, body) = split(text)?;
    // A blank first line stands in for the opening `---`, so the YAML's
    // line numbers are the file's.
    let padded = format!("\n{yaml}");
    let value = serde_saphyr::from_str_with_options(&padded, strict()).map_err(|e| {
        let msg = first_line(&e.to_string());
        match e.location() {
            Some(at) => {
                let suffix = format!(" at line {}, column {}", at.line(), at.column());
                let msg = msg.strip_suffix(&suffix).unwrap_or(&msg);
                format!("line {}, column {}: {msg}", at.line(), at.column())
            }
            None => msg,
        }
    })?;
    Ok((value, body))
}

fn strict() -> Options {
    let mut options = Options::default();
    options.strict_booleans = true;
    options.no_schema = true;
    options.duplicate_keys = DuplicateKeyPolicy::Error;
    options.merge_keys = MergeKeyPolicy::Error;
    options.reject_unsupported_tags = true;
    options.with_snippet = false;
    options
}

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or_default().to_string()
}

/// Splits `text` into its front matter and its body.
fn split(text: &str) -> Result<(&str, &str), String> {
    let mut lines = text.split_inclusive('\n');
    let first = lines.next().unwrap_or_default();
    if first.trim_end() != "---" {
        return Err("line 1: a content file must start with a `---` line".into());
    }
    let start = first.len();
    let mut at = start;
    for line in lines {
        if line.trim_end() == "---" {
            return Ok((&text[start..at], &text[at + line.len()..]));
        }
        at += line.len();
    }
    Err("the front matter has no closing `---` line".into())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Thing {
        name: String,
        #[serde(default)]
        one_way: bool,
    }

    const GOOD: &str = "---\n# a comment\nname: Marrow\n---\nProse here.\n";

    #[test]
    fn reads_the_front_matter_and_returns_the_body() {
        let (thing, body) = parse::<Thing>(GOOD).unwrap();
        assert_eq!(
            thing,
            Thing {
                name: "Marrow".into(),
                one_way: false
            }
        );
        assert_eq!(body, "Prose here.\n");
    }

    #[test]
    fn accepts_windows_line_endings() {
        let (thing, body) = parse::<Thing>("---\r\nname: Marrow\r\n---\r\nProse.\r\n").unwrap();
        assert_eq!(thing.name, "Marrow");
        assert_eq!(body, "Prose.\r\n");
    }

    #[test]
    fn a_file_without_front_matter_is_an_error() {
        let err = parse::<Thing>("name: Marrow\n").unwrap_err();
        assert!(err.contains("must start with a `---` line"), "{err}");
    }

    #[test]
    fn unclosed_front_matter_is_an_error() {
        let err = parse::<Thing>("---\nname: Marrow\n").unwrap_err();
        assert!(err.contains("no closing `---`"), "{err}");
    }

    #[test]
    fn an_unknown_field_is_an_error_on_its_file_line() {
        let err = parse::<Thing>("---\nname: Marrow\nnmae: typo\n---\n").unwrap_err();
        assert!(
            err.starts_with("line 3, column 1: unknown field `nmae`"),
            "{err}"
        );
        assert!(
            !err.contains(" at line "),
            "the location is given once: {err}"
        );
    }

    #[test]
    fn yes_is_not_a_boolean() {
        let err = parse::<Thing>("---\nname: Marrow\none_way: yes\n---\n").unwrap_err();
        assert!(err.starts_with("line 3"), "{err}");
    }

    #[test]
    fn an_unquoted_number_or_boolean_is_not_text() {
        assert!(parse::<Thing>("---\nname: 1984\n---\n").is_err());
        assert!(parse::<Thing>("---\nname: true\n---\n").is_err());
        assert_eq!(
            parse::<Thing>("---\nname: \"1984\"\n---\n").unwrap().0.name,
            "1984"
        );
    }

    #[test]
    fn no_is_text_not_false() {
        assert_eq!(parse::<Thing>("---\nname: no\n---\n").unwrap().0.name, "no");
    }

    #[test]
    fn a_duplicate_key_is_an_error() {
        assert!(parse::<Thing>("---\nname: A\nname: B\n---\n").is_err());
    }
}
