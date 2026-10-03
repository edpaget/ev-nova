//! Load order: how names sort and which directory entries load.
//!
//! Both functions here are pure. [`classify`] takes the folder's
//! [`Origin`] so that one rule covers both roots: a folder is
//! [`Classified::Descend`] in the plug-ins tree and ignored in the data
//! directory. The walk applies [`MAX_PLUGIN_DEPTH`] itself, since only it
//! knows how deep it is.

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::path::Path;

use super::Origin;
use super::fs::EntryKind;

/// Why a directory entry was not loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IgnoreReason {
    /// Its name starts with `.` (`.DS_Store`, `AppleDouble` `._x` files).
    Hidden,
    /// A symbolic link; links are never followed.
    Symlink,
    /// Neither a regular file nor a directory.
    NotAFile,
    /// A folder inside the data directory; only the plug-ins directory is
    /// walked recursively.
    DataSubFolder,
    /// A plug-ins sub-folder nested deeper than [`MAX_PLUGIN_DEPTH`].
    TooDeep,
    /// A file whose extension marks it as something other than game data
    /// (music, movies, documents, images). Holds the lower-cased extension.
    NotGameData(String),
}

/// What to do with one directory entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Classified {
    /// A file to try loading as a resource file.
    Candidate,
    /// A plug-ins sub-folder to walk, at its position in the order.
    Descend,
    /// Skipped, for a reason that is reported.
    Ignored(IgnoreReason),
}

/// The deepest plug-ins sub-folder level that is walked. The plug-ins
/// directory itself is level 0; its sub-folders are level 1.
pub const MAX_PLUGIN_DEPTH: usize = 16;

/// Lower-cased extensions of files that are never game data.
const NOT_GAME_DATA: &[&str] = &[
    "mp3", "mov", "txt", "rtf", "md", "pdf", "htm", "html", "jpg", "jpeg", "png", "gif",
];

/// Decides what to do with the entry `name` of kind `kind` found in a
/// folder of `origin`.
///
/// Hidden names (starting with `.`), symbolic links and entries that are
/// neither files nor folders are ignored. A folder is walked in the plug-ins
/// tree and ignored in the data directory. A file is ignored when its
/// lower-cased extension is in the skip list (music, movies, documents,
/// images); every other file is a candidate, including `.ndat`, `.npif`,
/// Windows `.rez` files, files with no extension and names such as
/// `Foo v1.2`, because classic plug-ins often have no extension or a dotted
/// name.
#[must_use]
pub fn classify(name: &OsStr, kind: EntryKind, origin: Origin) -> Classified {
    if name.as_encoded_bytes().starts_with(b".") {
        return Classified::Ignored(IgnoreReason::Hidden);
    }
    match (kind, origin) {
        (EntryKind::Symlink, _) => Classified::Ignored(IgnoreReason::Symlink),
        (EntryKind::Other, _) => Classified::Ignored(IgnoreReason::NotAFile),
        (EntryKind::Dir, Origin::Data) => Classified::Ignored(IgnoreReason::DataSubFolder),
        (EntryKind::Dir, Origin::PlugIn) => Classified::Descend,
        (EntryKind::File, _) => classify_file(name),
    }
}

fn classify_file(name: &OsStr) -> Classified {
    let extension = Path::new(name)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase());
    match extension {
        Some(ext) if NOT_GAME_DATA.contains(&ext.as_str()) => {
            Classified::Ignored(IgnoreReason::NotGameData(ext))
        }
        _ => Classified::Candidate,
    }
}

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

    #[test]
    fn classify_decides_every_kind_of_entry() {
        use Classified::{Candidate, Descend, Ignored};
        use EntryKind::{Dir, File, Other, Symlink};
        use IgnoreReason as R;
        use Origin::{Data, PlugIn};

        let not_game_data = |ext: &str| Ignored(R::NotGameData(ext.to_owned()));
        let cases = [
            (".DS_Store", File, Data, Ignored(R::Hidden)),
            ("._Nova Data 1.ndat", File, PlugIn, Ignored(R::Hidden)),
            (".hidden folder", Dir, PlugIn, Ignored(R::Hidden)),
            ("Nova Music.mp3", File, Data, not_game_data("mp3")),
            ("Race 1.MOV", File, Data, not_game_data("mov")),
            ("readme.txt", File, PlugIn, not_game_data("txt")),
            ("Read Me.RTF", File, PlugIn, not_game_data("rtf")),
            ("notes.md", File, PlugIn, not_game_data("md")),
            ("Manual.pdf", File, PlugIn, not_game_data("pdf")),
            ("index.htm", File, PlugIn, not_game_data("htm")),
            ("index.html", File, PlugIn, not_game_data("html")),
            ("shot.jpg", File, PlugIn, not_game_data("jpg")),
            ("shot.jpeg", File, PlugIn, not_game_data("jpeg")),
            ("shot.png", File, PlugIn, not_game_data("png")),
            ("shot.gif", File, PlugIn, not_game_data("gif")),
            ("x.rez", File, PlugIn, Candidate),
            ("X.REZ", File, Data, Candidate),
            ("Link", Symlink, PlugIn, Ignored(R::Symlink)),
            ("Link.ndat", Symlink, Data, Ignored(R::Symlink)),
            ("fifo", Other, PlugIn, Ignored(R::NotAFile)),
            ("Extras", Dir, Data, Ignored(R::DataSubFolder)),
            ("Extras", Dir, PlugIn, Descend),
            ("Docs.txt", Dir, PlugIn, Descend),
            ("Nova Data 1.ndat", File, Data, Candidate),
            ("Foo.npif", File, PlugIn, Candidate),
            ("Foo", File, PlugIn, Candidate),
            ("Foo v1.2", File, PlugIn, Candidate),
            ("mp3", File, PlugIn, Candidate),
        ];
        for (name, kind, origin, expected) in cases {
            assert_eq!(
                classify(OsStr::new(name), kind, origin),
                expected,
                "{name} {kind:?} in {origin:?}"
            );
        }
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
