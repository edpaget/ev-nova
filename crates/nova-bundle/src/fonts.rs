//! The fonts `nova` compiles in, and the licences that travel with them.
//!
//! `nova` builds two fonts into its executable through `nova-render`:
//! Noto Sans Regular and Noto Serif Regular, both under the SIL Open Font
//! License 1.1. The OFL asks every copy of the fonts to carry the licence,
//! so every download ships both licence files, each under a name that says
//! which font it covers.

use std::path::{Path, PathBuf};

/// The fonts' directory, relative to the workspace root, as components.
pub const FONT_DIR: [&str; 3] = ["crates", "nova-render", "fonts"];

/// Each font licence's file name in [`FONT_DIR`] and the name it ships as.
pub const FONT_LICENCES: [(&str, &str); 2] = [
    ("OFL.txt", "NotoSans-OFL.txt"),
    ("NotoSerif-OFL.txt", "NotoSerif-OFL.txt"),
];

/// Each font licence's source under `workspace_root`, with its shipped name.
#[must_use]
pub fn font_licences(workspace_root: &Path) -> [(PathBuf, &'static str); 2] {
    let dir = FONT_DIR
        .iter()
        .fold(workspace_root.to_path_buf(), |dir, part| dir.join(part));
    FONT_LICENCES.map(|(source, shipped)| (dir.join(source), shipped))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_names_are_distinct_and_clash_with_nothing_else_shipped() {
        let [(_, sans), (_, serif)] = font_licences(Path::new("root"));
        assert_eq!([sans, serif], ["NotoSans-OFL.txt", "NotoSerif-OFL.txt"]);
        for name in [sans, serif] {
            assert_ne!(name, "LICENSE");
            assert_ne!(name, "README.txt");
        }
    }

    #[test]
    fn the_sources_lie_in_nova_render_fonts_under_the_root() {
        let root = Path::new("root");
        let fonts = root.join("crates").join("nova-render").join("fonts");
        let [(sans, _), (serif, _)] = font_licences(root);
        assert_eq!(sans, fonts.join("OFL.txt"));
        assert_eq!(serif, fonts.join("NotoSerif-OFL.txt"));
    }

    #[test]
    fn each_source_exists_in_the_workspace() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        for (source, _) in font_licences(&root) {
            assert!(source.is_file(), "{}", source.display());
        }
    }
}
