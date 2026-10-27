//! The resource script (`.rc`) nova's build script compiles: the icon,
//! the manifest and the version information, with no `#include`, so any
//! resource compiler (`rc.exe`, `llvm-rc`, `windres`) takes it without
//! the Windows SDK's headers.

use std::fmt::Write;
use std::path::Path;

use crate::version::{VersionError, version_quad};
use crate::{ICON_ID, MANIFEST_ID, VERSION_ID};

/// What the script names and says.
#[derive(Clone, Copy, Debug)]
pub struct Resources<'a> {
    /// The `.ico` file.
    pub icon: &'a Path,
    /// The application manifest.
    pub manifest: &'a Path,
    /// The product's name ("EV Nova").
    pub product_name: &'a str,
    /// What the file is (the package's description).
    pub description: &'a str,
    /// The package's semver version, the strings' and the numbers'.
    pub version: &'a str,
    /// The executable's file name (`nova.exe`).
    pub original_filename: &'a str,
    /// The copyright line.
    pub copyright: &'a str,
}

/// `text` as the inside of a resource-script string: each `"` doubled and
/// each `\` doubled, so a path or a value is read back as it is.
#[must_use]
pub fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\"\"")
}

/// The resource script for `resources`.
///
/// The ids are [`ICON_ID`], [`MANIFEST_ID`] and [`VERSION_ID`]; resource
/// type 24 is `RT_MANIFEST`. The version information is an application's
/// (`VFT_APP`) for 32-bit Windows (`VOS_NT_WINDOWS32`), marked
/// `VS_FF_PRERELEASE` when the version has a pre-release part, with its
/// strings in US English, Unicode (`040904B0`). The script is read as
/// UTF-8, so the strings may be any text.
///
/// # Errors
///
/// When `resources.version` is not a semver version ([`version_quad`]).
pub fn resource_script(resources: &Resources) -> Result<String, VersionError> {
    let [major, minor, patch, build] = version_quad(resources.version)?;
    let quad = format!("{major},{minor},{patch},{build}");
    let numbers = resources.version.split('+').next().unwrap_or_default();
    let flags = if numbers.contains('-') { "0x2" } else { "0x0" };
    let quoted = |file: &Path| escape(&file.to_string_lossy());
    let internal = resources
        .original_filename
        .strip_suffix(".exe")
        .unwrap_or(resources.original_filename);
    let strings = [
        ("FileDescription", resources.description),
        ("FileVersion", resources.version),
        ("InternalName", internal),
        ("LegalCopyright", resources.copyright),
        ("OriginalFilename", resources.original_filename),
        ("ProductName", resources.product_name),
        ("ProductVersion", resources.version),
    ];
    let values = strings
        .iter()
        .fold(String::new(), |mut values, (name, value)| {
            let _ = writeln!(values, "      VALUE \"{name}\", \"{}\"", escape(value));
            values
        });
    Ok(format!(
        "#pragma code_page(65001)\n\
         {ICON_ID} ICON \"{icon}\"\n\
         {MANIFEST_ID} 24 \"{manifest}\"\n\
         {VERSION_ID} VERSIONINFO\n\
         FILEVERSION {quad}\n\
         PRODUCTVERSION {quad}\n\
         FILEFLAGSMASK 0x3f\n\
         FILEFLAGS {flags}\n\
         FILEOS 0x40004\n\
         FILETYPE 0x1\n\
         FILESUBTYPE 0x0\n\
         BEGIN\n\
         \x20 BLOCK \"StringFileInfo\"\n\
         \x20 BEGIN\n\
         \x20   BLOCK \"040904B0\"\n\
         \x20   BEGIN\n\
         {values}\
         \x20   END\n\
         \x20 END\n\
         \x20 BLOCK \"VarFileInfo\"\n\
         \x20 BEGIN\n\
         \x20   VALUE \"Translation\", 0x409, 1200\n\
         \x20 END\n\
         END\n",
        icon = quoted(resources.icon),
        manifest = quoted(resources.manifest),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    fn resources<'a>(icon: &'a Path, manifest: &'a Path, version: &'a str) -> Resources<'a> {
        Resources {
            icon,
            manifest,
            product_name: "EV Nova",
            description: "EV Nova engine and player",
            version,
            original_filename: "nova.exe",
            copyright: "Licensed under GPL-3.0-or-later",
        }
    }

    /// The script's statements, one per line, trimmed.
    fn lines(script: &str) -> Vec<&str> {
        script
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect()
    }

    fn script(version: &str) -> String {
        let icon = PathBuf::from("out").join("nova.ico");
        let manifest = PathBuf::from("pkg").join("nova.exe.manifest");
        resource_script(&resources(&icon, &manifest, version)).expect("a script")
    }

    #[test]
    fn the_icon_and_the_manifest_are_resource_one_of_their_types() {
        let icon = Path::new(r"C:\out dir\nova.ico");
        let manifest = Path::new(r#"C:\a "b"\nova.exe.manifest"#);
        let script = resource_script(&resources(icon, manifest, "0.1.0")).expect("a script");
        let lines = lines(&script);
        assert!(
            lines.contains(&r#"1 ICON "C:\\out dir\\nova.ico""#),
            "{script}"
        );
        assert!(
            lines.contains(&r#"1 24 "C:\\a ""b""\\nova.exe.manifest""#),
            "{script}"
        );
    }

    #[test]
    fn the_version_numbers_are_the_versions_quad() {
        let pre = script("1.2.3-rc.1");
        let pre_lines = lines(&pre);
        assert!(pre_lines.contains(&"1 VERSIONINFO"), "{pre}");
        assert!(pre_lines.contains(&"FILEVERSION 1,2,3,0"), "{pre}");
        assert!(pre_lines.contains(&"PRODUCTVERSION 1,2,3,0"), "{pre}");
        assert!(pre_lines.contains(&"FILEFLAGS 0x2"), "a pre-release: {pre}");
        assert!(pre_lines.contains(&"FILEOS 0x40004"), "{pre}");
        assert!(pre_lines.contains(&"FILETYPE 0x1"), "an application: {pre}");
        assert!(!pre_lines.contains(&"FILEFLAGS 0x0"), "{pre}");
        let release = script("0.1.0");
        let release_lines = lines(&release);
        assert!(release_lines.contains(&"FILEFLAGS 0x0"), "{release}");
        assert!(release_lines.contains(&"FILEVERSION 0,1,0,0"), "{release}");
        let built = script("0.1.0+abc-def");
        assert!(
            lines(&built).contains(&"FILEFLAGS 0x0"),
            "a build part only: {built}"
        );
    }

    #[test]
    fn the_strings_are_in_us_english_unicode() {
        let script = script("1.2.3-rc.1");
        let lines = lines(&script);
        for expected in [
            r#"BLOCK "StringFileInfo""#,
            r#"BLOCK "040904B0""#,
            r#"VALUE "ProductName", "EV Nova""#,
            r#"VALUE "FileDescription", "EV Nova engine and player""#,
            r#"VALUE "FileVersion", "1.2.3-rc.1""#,
            r#"VALUE "ProductVersion", "1.2.3-rc.1""#,
            r#"VALUE "OriginalFilename", "nova.exe""#,
            r#"VALUE "InternalName", "nova""#,
            r#"VALUE "LegalCopyright", "Licensed under GPL-3.0-or-later""#,
            r#"BLOCK "VarFileInfo""#,
            r#"VALUE "Translation", 0x409, 1200"#,
        ] {
            assert!(lines.contains(&expected), "{expected} in {script}");
        }
        assert_eq!(
            lines.first(),
            Some(&"#pragma code_page(65001)"),
            "read as UTF-8"
        );
    }

    #[test]
    fn a_value_with_a_quote_is_escaped() {
        let icon = PathBuf::from("nova.ico");
        let manifest = PathBuf::from("nova.exe.manifest");
        let mut resources = resources(&icon, &manifest, "0.1.0");
        resources.product_name = r#"EV "Nova""#;
        let script = resource_script(&resources).expect("a script");
        assert!(
            lines(&script).contains(&r#"VALUE "ProductName", "EV ""Nova""""#),
            "{script}"
        );
    }

    #[test]
    fn a_version_that_is_not_semver_is_refused() {
        let icon = PathBuf::from("nova.ico");
        let manifest = PathBuf::from("nova.exe.manifest");
        assert_eq!(
            resource_script(&resources(&icon, &manifest, "one")),
            Err(VersionError("one".to_owned()))
        );
    }

    #[test]
    fn quotes_and_backslashes_are_doubled() {
        assert_eq!(escape(r"C:\Users\p\out"), r"C:\\Users\\p\\out");
        assert_eq!(escape(r#"say "hi""#), r#"say ""hi"""#);
        assert_eq!(escape("plain/path é"), "plain/path é");
    }
}
