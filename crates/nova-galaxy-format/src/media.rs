//! Media the asset manifest names that a checkout hasn't fetched: Git LFS
//! pointers in place of the files.
//!
//! The engine and the `galaxy` tool both need to tell a pointer from the
//! file, so it lives here, beside the manifest ([`crate::asset`]).

use crate::load::Galaxy;
use crate::source::Source;

/// What every Git LFS pointer file starts with. A checkout that hasn't
/// run `git lfs pull` has these in place of the media.
pub const LFS_POINTER: &[u8] = b"version https://git-lfs.github.com/spec/v1";

/// Whether `bytes` is a Git LFS pointer rather than the file itself.
#[must_use]
pub fn is_lfs_pointer(bytes: &[u8]) -> bool {
    bytes.starts_with(LFS_POINTER)
}

/// The export media paths `galaxy`'s asset manifest names
/// ([`Galaxy::media_paths`] under `media/export/`) whose bytes in
/// `source` are LFS pointers, sorted.
///
/// A file that is absent, or can't be read, isn't a pointer: the `galaxy`
/// tool's checks report those. Files under `media/source/` are what
/// artists edit, which nothing loads, so they aren't read.
#[must_use]
pub fn lfs_pointers<'a>(galaxy: &'a Galaxy, source: &impl Source) -> Vec<&'a str> {
    galaxy
        .media_paths()
        .into_iter()
        .filter(|path| path.starts_with(EXPORT))
        .filter(|path| {
            source
                .bytes(path)
                .ok()
                .flatten()
                .is_some_and(|bytes| is_lfs_pointer(&bytes))
        })
        .collect()
}

/// Where the media the engine loads sit, as [`MediaPath::is_export`]
/// reads it.
///
/// [`MediaPath::is_export`]: crate::id::MediaPath::is_export
const EXPORT: &str = "media/export/";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;
    use crate::load::load;
    use crate::source::MemSource;

    /// A pointer as `git lfs` writes one.
    const POINTER: &[u8] = b"version https://git-lfs.github.com/spec/v1\n\
        oid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\n\
        size 12345\n";

    #[test]
    fn a_pointer_is_recognised() {
        assert!(is_lfs_pointer(POINTER));
    }

    #[test]
    fn real_bytes_are_not_a_pointer() {
        assert!(!is_lfs_pointer(b"glTF\x02\0\0\0"));
        assert!(!is_lfs_pointer(b""));
        assert!(!is_lfs_pointer(
            b"version https://git-lfs.github.com/spec/v"
        ));
    }

    /// The fixture galaxy, with `media` written at the manifest's export
    /// paths.
    fn with_media(media: &[(&str, &[u8])]) -> MemSource {
        media
            .iter()
            .fold(fixture::galaxy(), |source, (path, bytes)| {
                source.with_bytes(path, bytes)
            })
    }

    fn pointers(source: &MemSource) -> Vec<String> {
        let (galaxy, _) = load(source);
        lfs_pointers(&galaxy, source)
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn lists_exactly_the_export_paths_holding_pointers_sorted() {
        let source = with_media(&[
            ("media/export/pack/surface.png", POINTER),
            ("media/export/pack/hull.glb", POINTER),
            ("media/export/pack/LICENSE.txt", b"CC0"),
        ]);
        assert_eq!(
            pointers(&source),
            [
                "media/export/pack/hull.glb",
                "media/export/pack/surface.png"
            ]
        );
    }

    #[test]
    fn real_media_lists_nothing() {
        let source = with_media(&[
            ("media/export/pack/surface.png", b"\x89PNG"),
            ("media/export/pack/hull.glb", b"glTF\x02"),
            ("media/export/pack/LICENSE.txt", b"CC0"),
        ]);
        assert_eq!(pointers(&source), Vec::<String>::new());
    }

    #[test]
    fn an_absent_file_is_not_a_pointer() {
        let source = with_media(&[("media/export/pack/hull.glb", POINTER)]);
        assert_eq!(pointers(&source), ["media/export/pack/hull.glb"]);
    }

    #[test]
    fn a_source_path_is_skipped() {
        let source = with_media(&[("media/source/spire.blend", POINTER)]).with(
            "assets/spire.md",
            "---\nkind: model\nplaceholder: true\nsources: [media/source/spire.blend]\n---\n",
        );
        assert_eq!(pointers(&source), Vec::<String>::new());
    }

    #[test]
    fn a_pointer_the_manifest_does_not_name_is_not_listed() {
        let source = with_media(&[("media/export/other/stray.glb", POINTER)]);
        assert_eq!(pointers(&source), Vec::<String>::new());
    }
}
