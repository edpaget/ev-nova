//! Typed text values: ids, flags and references.
//!
//! Each one checks its spelling when it is deserialized, so a typo fails at
//! load time with the offending text in the message. Each also describes
//! itself to the JSON Schema with the same pattern.

use std::borrow::Cow;
use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Deserialize;

const KEBAB: &str = "[a-z0-9]+(-[a-z0-9]+)*";
const INK_NAME: &str = "[A-Za-z_][A-Za-z0-9_]*";

/// Whether `s` is lowercase kebab-case: `far-gate`, `the-first-failure`.
#[must_use]
pub fn is_kebab(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// Whether `s` is lowercase `snake_case` starting with a letter: `met_organizer`.
#[must_use]
pub fn is_snake(s: &str) -> bool {
    s.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && s.split('_').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// Whether `s` is a valid Ink knot, stitch or variable name.
#[must_use]
pub fn is_ink_name(s: &str) -> bool {
    s.bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Declares a string newtype that checks its spelling on the way in.
macro_rules! checked_text {
    ($(#[$doc:meta])* $name:ident, $what:literal, $pattern:expr, $check:expr) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
        #[serde(try_from = "String")]
        pub struct $name(String);

        impl TryFrom<String> for $name {
            type Error = String;
            fn try_from(s: String) -> Result<Self, String> {
                if $check(&s) {
                    Ok(Self(s))
                } else {
                    Err(format!("`{s}` is not {}", $what))
                }
            }
        }

        impl $name {
            /// The text as written.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> {
                stringify!($name).into()
            }
            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                json_schema!({ "type": "string", "pattern": $pattern, "description": $what })
            }
        }
    };
}

checked_text!(
    /// A kebab-case id. A file's id is its name without `.md`; a jump
    /// point's id is unique within its system.
    Id,
    "a kebab-case id (lowercase letters, digits and single hyphens)",
    format!("^{KEBAB}$"),
    is_kebab
);

checked_text!(
    /// A world-state flag: a named on/off fact that missions set and test.
    /// Snake-case so the same name works as an Ink variable.
    Flag,
    "a snake_case flag (lowercase letters, digits and single underscores, starting with a letter)",
    "^[a-z][a-z0-9]*(_[a-z0-9]+)*$",
    is_snake
);

checked_text!(
    /// A colour as `#rrggbb`.
    Color,
    "a colour written #rrggbb",
    "^#[0-9a-fA-F]{6}$",
    |s: &str| s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
);

checked_text!(
    /// A jump point anywhere in the galaxy, written `system/point`.
    PointRef,
    "a jump point written system-id/point-id",
    format!("^{KEBAB}/{KEBAB}$"),
    |s: &str| s.split_once('/').is_some_and(|(a, b)| is_kebab(a) && is_kebab(b))
);

impl PointRef {
    /// The system's id.
    #[must_use]
    pub fn system(&self) -> Id {
        Id(self.0.split_once('/').expect("checked").0.to_string())
    }

    /// The point's id within its system.
    #[must_use]
    pub fn point(&self) -> Id {
        Id(self.0.split_once('/').expect("checked").1.to_string())
    }

    /// The reference to `point` in `system`.
    #[must_use]
    pub fn new(system: &Id, point: &Id) -> Self {
        Self(format!("{system}/{point}"))
    }
}

fn dialog_parts(s: &str) -> Option<(&str, &str, Option<&str>)> {
    let (file, target) = s.split_once('#')?;
    let stem = file.strip_prefix("dialog/")?.strip_suffix(".ink")?;
    let (knot, stitch) = match target.split_once('.') {
        Some((k, st)) => (k, Some(st)),
        None => (target, None),
    };
    (is_kebab(stem) && is_ink_name(knot) && stitch.is_none_or(is_ink_name))
        .then_some((file, knot, stitch))
}

checked_text!(
    /// A place in an Ink file, written `dialog/<file>.ink#knot` or
    /// `dialog/<file>.ink#knot.stitch`.
    DialogRef,
    "a dialog reference written dialog/<file>.ink#knot or dialog/<file>.ink#knot.stitch",
    format!("^dialog/{KEBAB}\\.ink#{INK_NAME}(\\.{INK_NAME})?$"),
    |s: &str| dialog_parts(s).is_some()
);

impl DialogRef {
    /// The Ink file, relative to the data folder.
    #[must_use]
    pub fn file(&self) -> &str {
        dialog_parts(&self.0).expect("checked").0
    }

    /// The knot.
    #[must_use]
    pub fn knot(&self) -> &str {
        dialog_parts(&self.0).expect("checked").1
    }

    /// The stitch within the knot, if named.
    #[must_use]
    pub fn stitch(&self) -> Option<&str> {
        dialog_parts(&self.0).expect("checked").2
    }
}

/// How harsh a place is, from 1 (mild) to 5 (brutal). See the harshness
/// gradient in `story/tone.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "u8")]
pub struct Harshness(u8);

impl TryFrom<u8> for Harshness {
    type Error = String;
    fn try_from(n: u8) -> Result<Self, String> {
        if (1..=5).contains(&n) {
            Ok(Self(n))
        } else {
            Err(format!("harshness {n} is outside 1 to 5"))
        }
    }
}

impl Harshness {
    /// The value, 1 to 5.
    #[must_use]
    pub fn get(self) -> u8 {
        self.0
    }
}

impl JsonSchema for Harshness {
    fn schema_name() -> Cow<'static, str> {
        "Harshness".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "integer", "minimum": 1, "maximum": 5,
            "description": "How harsh a place is, from 1 (mild) to 5 (brutal)"
        })
    }
}

fn is_date(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    let number = |part: &str, len: usize, range: std::ops::RangeInclusive<u32>| {
        part.len() == len
            && part.bytes().all(|b| b.is_ascii_digit())
            && part.parse().is_ok_and(|n| range.contains(&n))
    };
    matches!(parts.as_slice(), [y, m, d]
        if number(y, 4, 1900..=2999) && number(m, 2, 1..=12) && number(d, 2, 1..=31))
}

checked_text!(
    /// A calendar date written `YYYY-MM-DD`.
    Date,
    "a date written YYYY-MM-DD",
    "^[0-9]{4}-[0-9]{2}-[0-9]{2}$",
    is_date
);

checked_text!(
    /// A web address starting `https://` or `http://`.
    Url,
    "a web address starting https:// or http://",
    "^https?://[^\\s]+$",
    |s: &str| {
        let rest = s.strip_prefix("https://").or_else(|| s.strip_prefix("http://"));
        rest.is_some_and(|r| !r.is_empty() && !r.contains(char::is_whitespace))
    }
);

/// Whether `part` is a safe file or folder name on every platform:
/// letters, digits, `.`, `_` and `-`, not `.` or `..`.
fn is_file_name(part: &str) -> bool {
    !part.is_empty()
        && part != "."
        && part != ".."
        && part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

checked_text!(
    /// A media file in the data folder, under `media/export/` (what the
    /// engine loads) or `media/source/` (what artists edit).
    MediaPath,
    "a media file path under media/export/ or media/source/ (letters, digits, `.`, `_` and `-`)",
    "^media/(export|source)(/[A-Za-z0-9._-]+)+$",
    |s: &str| {
        let rest = s.strip_prefix("media/export/").or_else(|| s.strip_prefix("media/source/"));
        rest.is_some_and(|r| r.split('/').all(is_file_name))
    }
);

impl MediaPath {
    /// The file's extension in lowercase, or empty.
    #[must_use]
    pub fn extension(&self) -> String {
        let name = self.0.rsplit('/').next().unwrap_or_default();
        name.rsplit_once('.')
            .map(|(_, ext)| ext.to_ascii_lowercase())
            .unwrap_or_default()
    }

    /// Whether it is under `media/export/`.
    #[must_use]
    pub fn is_export(&self) -> bool {
        self.0.starts_with("media/export/")
    }
}

/// A number from 0 to 1: a coverage, a thickness.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Deserialize)]
#[serde(try_from = "f64")]
pub struct Fraction(f64);

impl TryFrom<f64> for Fraction {
    type Error = String;
    fn try_from(n: f64) -> Result<Self, String> {
        if (0.0..=1.0).contains(&n) {
            Ok(Self(n))
        } else {
            Err(format!("{n} is outside 0 to 1"))
        }
    }
}

impl Fraction {
    /// The value, 0 to 1.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

impl JsonSchema for Fraction {
    fn schema_name() -> Cow<'static, str> {
        "Fraction".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "number", "minimum": 0, "maximum": 1,
            "description": "A number from 0 to 1"
        })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn an_ink_name_may_start_with_an_underscore() {
        assert!(is_ink_name("_hidden"));
        assert!(is_ink_name("knot_2"));
        assert!(!is_ink_name("2knot"));
        assert!(!is_ink_name("-knot"));
    }

    #[test]
    fn a_source_path_is_not_export() {
        let source = MediaPath::try_from("media/source/spire.blend".to_string()).unwrap();
        assert!(!source.is_export());
    }

    /// `T`'s schema as the generator writes it: its name and its JSON.
    fn schema_of<T: JsonSchema>() -> (String, serde_json::Value) {
        let mut generator = SchemaGenerator::default();
        let schema = T::json_schema(&mut generator);
        (T::schema_name().into_owned(), schema.to_value())
    }

    #[test]
    fn a_harshness_schema_is_an_integer_from_1_to_5() {
        let (name, schema) = schema_of::<Harshness>();
        assert_eq!(name, "Harshness");
        assert_eq!(schema["type"], "integer");
        assert_eq!(
            (schema["minimum"].as_u64(), schema["maximum"].as_u64()),
            (Some(1), Some(5))
        );
    }

    #[test]
    fn a_fraction_schema_is_a_number_from_0_to_1() {
        let (name, schema) = schema_of::<Fraction>();
        assert_eq!(name, "Fraction");
        assert_eq!(schema["type"], "number");
        assert_eq!(
            (schema["minimum"].as_u64(), schema["maximum"].as_u64()),
            (Some(0), Some(1))
        );
    }

    #[test]
    fn an_id_is_kebab_case() {
        assert_eq!(
            Id::try_from("far-gate".to_string()).unwrap().as_str(),
            "far-gate"
        );
        assert!(Id::try_from("Far-Gate".to_string()).is_err());
        assert!(Id::try_from("far_gate".to_string()).is_err());
        assert!(Id::try_from("-gate".to_string()).is_err());
        assert!(Id::try_from("far--gate".to_string()).is_err());
        assert!(Id::try_from(String::new()).is_err());
    }

    #[test]
    fn a_flag_is_snake_case_like_an_ink_variable() {
        assert!(Flag::try_from("met_organizer".to_string()).is_ok());
        assert!(Flag::try_from("met-organizer".to_string()).is_err());
        assert!(Flag::try_from("9lives".to_string()).is_err());
    }

    #[test]
    fn a_point_ref_names_a_system_and_one_of_its_points() {
        let r = PointRef::try_from("marrow/far-gate".to_string()).unwrap();
        assert_eq!(
            (r.system().as_str(), r.point().as_str()),
            ("marrow", "far-gate")
        );
        assert_eq!(r.to_string(), "marrow/far-gate");
        assert!(PointRef::try_from("marrow".to_string()).is_err());
        assert!(PointRef::try_from("marrow/far/gate".to_string()).is_err());
    }

    #[test]
    fn a_dialog_ref_names_an_ink_file_and_knot() {
        let r = DialogRef::try_from("dialog/holding-the-point.ink#crossroads_offer".to_string())
            .unwrap();
        assert_eq!(r.file(), "dialog/holding-the-point.ink");
        assert_eq!(r.knot(), "crossroads_offer");
        assert_eq!(r.stitch(), None);
        let s = DialogRef::try_from("dialog/a.ink#knot.stitch".to_string()).unwrap();
        assert_eq!(s.stitch(), Some("stitch"));
        assert!(DialogRef::try_from("dialog/a.ink".to_string()).is_err());
        assert!(DialogRef::try_from("a.ink#knot".to_string()).is_err());
        assert!(DialogRef::try_from("dialog/a.txt#knot".to_string()).is_err());
        assert!(DialogRef::try_from("dialog/a.ink#bad-knot".to_string()).is_err());
    }

    #[test]
    fn a_color_is_six_hex_digits() {
        assert!(Color::try_from("#c08a2e".to_string()).is_ok());
        assert!(Color::try_from("c08a2e".to_string()).is_err());
        assert!(Color::try_from("#c08a2".to_string()).is_err());
        assert!(Color::try_from("#c08a2g".to_string()).is_err());
    }

    #[test]
    fn harshness_runs_from_one_to_five() {
        assert_eq!(Harshness::try_from(1).unwrap().get(), 1);
        assert_eq!(Harshness::try_from(3).unwrap().get(), 3);
        assert!(Harshness::try_from(5).is_ok());
        assert!(Harshness::try_from(0).is_err());
        assert!(Harshness::try_from(6).is_err());
    }
    #[test]
    fn a_date_is_year_month_day() {
        assert!(Date::try_from("2026-10-30".to_string()).is_ok());
        assert!(Date::try_from("2026-13-01".to_string()).is_err());
        assert!(Date::try_from("2026-1-01".to_string()).is_err());
        assert!(Date::try_from("30/10/2026".to_string()).is_err());
    }

    #[test]
    fn a_url_is_http_or_https() {
        assert!(Url::try_from("https://opengameart.org/content/x".to_string()).is_ok());
        assert!(Url::try_from("http://kenney.nl".to_string()).is_ok());
        assert!(Url::try_from("opengameart.org".to_string()).is_err());
        assert!(Url::try_from("https://a b".to_string()).is_err());
    }

    #[test]
    fn a_media_path_stays_inside_media() {
        let path =
            MediaPath::try_from("media/export/kenney-space-kit/craft_cargoB.glb".to_string())
                .unwrap();
        assert_eq!(path.extension(), "glb");
        assert!(path.is_export());
        assert!(MediaPath::try_from("media/source/x/hull.blend".to_string()).is_ok());
        for bad in [
            "assets/x.glb",
            "media/export/../../x.glb",
            "media/export/a b.glb",
            "media/export/a\\b.glb",
            "media/export//x.glb",
            "media/export/x.glb/",
            "media/x.glb",
        ] {
            assert!(MediaPath::try_from(bad.to_string()).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_fraction_runs_from_zero_to_one() {
        assert_eq!(Fraction::try_from(0.25).unwrap().get(), 0.25);
        assert!(Fraction::try_from(0.0).is_ok());
        assert!(Fraction::try_from(1.0).is_ok());
        assert!(Fraction::try_from(1.5).is_err());
        assert!(Fraction::try_from(-0.1).is_err());
        assert!(Fraction::try_from(f64::NAN).is_err());
    }
}
