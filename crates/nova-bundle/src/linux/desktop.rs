//! The desktop file, `<identifier>.desktop`: what launchers, docks and
//! menus list the game as, and how they start it (the freedesktop.org
//! Desktop Entry Specification).

use super::{CATEGORIES, EXECUTABLE};
use crate::spec::BundleSpec;

/// The desktop file's name: the identifier, so the window's Wayland app
/// id and X11 `WM_CLASS`, which are the identifier too, match it.
#[must_use]
pub fn desktop_file_name(spec: &BundleSpec) -> String {
    format!("{}.desktop", spec.identifier)
}

/// Renders the desktop file's `[Desktop Entry]` group: an application
/// named for the spec, started as [`EXECUTABLE`] (which the player puts
/// on `PATH`), its icon the identifier's hicolor icons, and its
/// `StartupWMClass` the identifier, which GNOME on X11 matches the window
/// by.
#[must_use]
pub fn desktop_entry(spec: &BundleSpec) -> String {
    let entries = [
        ("Type", "Application".to_owned()),
        ("Name", escape(&spec.name)),
        ("Comment", escape(&spec.summary)),
        ("Exec", EXECUTABLE.to_owned()),
        ("Icon", escape(&spec.identifier)),
        ("Terminal", "false".to_owned()),
        ("Categories", CATEGORIES.to_owned()),
        ("StartupWMClass", escape(&spec.identifier)),
    ];
    let mut file = String::from("[Desktop Entry]\n");
    for (key, value) in entries {
        file.push_str(key);
        file.push('=');
        file.push_str(&value);
        file.push('\n');
    }
    file
}

/// `text` escaped as a Desktop Entry string value: a backslash, newline,
/// tab or carriage return becomes `\\`, `\n`, `\t` or `\r`.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            '\r' => escaped.push_str("\\r"),
            c => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::spec::tests::sample_metadata;

    /// The `[Desktop Entry]` group's keys and values, as written (still
    /// escaped), in order. Fails on any other group, on a line that is
    /// not `Key=Value`, a comment or blank, and on a key given twice.
    pub(crate) fn entries(file: &str) -> Vec<(String, String)> {
        let mut lines = file.lines();
        assert_eq!(lines.next(), Some("[Desktop Entry]"), "the group first");
        let mut entries: Vec<(String, String)> = Vec::new();
        for line in lines {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            assert!(!line.starts_with('['), "another group: {line}");
            let (key, value) = line.split_once('=').expect("Key=Value");
            assert!(
                entries.iter().all(|(seen, _)| seen != key),
                "{key} given twice"
            );
            entries.push((key.to_owned(), value.to_owned()));
        }
        entries
    }

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    fn value<'a>(entries: &'a [(String, String)], key: &str) -> &'a str {
        entries
            .iter()
            .find(|(k, _)| k == key)
            .map_or_else(|| panic!("no {key}"), |(_, v)| v.as_str())
    }

    #[test]
    fn the_file_is_named_for_the_identifier() {
        assert_eq!(desktop_file_name(&spec()), "io.example.nova.desktop");
    }

    #[test]
    fn holds_the_launcher_keys_from_the_spec() {
        let entries = entries(&desktop_entry(&spec()));
        let expected = [
            ("Type", "Application"),
            ("Name", "EV Nova"),
            ("Comment", "Fly the stars"),
            ("Exec", "nova"),
            ("Icon", "io.example.nova"),
            ("Terminal", "false"),
            ("Categories", "Game;RolePlaying;"),
            ("StartupWMClass", "io.example.nova"),
        ];
        for (key, want) in expected {
            assert_eq!(value(&entries, key), want, "{key}");
        }
        assert_eq!(entries.len(), expected.len(), "no other keys");
    }

    #[test]
    fn the_command_is_the_game_s_binary() {
        assert_eq!(EXECUTABLE, "nova");
    }

    #[test]
    fn escapes_the_string_values() {
        let mut spec = spec();
        spec.name = "EV\\Nova\nTwo\tTabs\rBack".into();
        spec.summary = "a\\b".into();
        let entries = entries(&desktop_entry(&spec));
        assert_eq!(value(&entries, "Name"), "EV\\\\Nova\\nTwo\\tTabs\\rBack");
        assert_eq!(value(&entries, "Comment"), "a\\\\b");
    }

    #[test]
    fn ends_with_a_newline() {
        assert!(desktop_entry(&spec()).ends_with('\n'));
    }
}
