//! Media the asset manifest and the sounds name that a checkout hasn't
//! fetched: Git LFS pointers in place of the files.
//!
//! The engine and the `galaxy` tool both need to tell a pointer from the
//! file, so it lives here, beside the manifest ([`crate::asset`]) and the
//! sounds ([`crate::sound`]).

use crate::id::MediaPath;
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

/// The export media paths `galaxy`'s asset manifest and sounds name
/// ([`Galaxy::media_paths`] that are [`MediaPath::is_export`]) whose bytes in
/// `source` are LFS pointers, sorted.
///
/// It asks for each file's head ([`Source::head`]), as many bytes as a
/// pointer starts with, and reads no file whole. A file that is absent,
/// or can't be read, isn't a pointer: the `galaxy` tool's checks report
/// those. Files under `media/source/` are what artists edit, which
/// nothing loads, so they aren't read.
#[must_use]
pub fn lfs_pointers<'a>(galaxy: &'a Galaxy, source: &impl Source) -> Vec<&'a str> {
    galaxy
        .media()
        .into_iter()
        .filter(|path| path.is_export())
        .map(MediaPath::as_str)
        .filter(|path| {
            source
                .head(path, LFS_POINTER.len())
                .ok()
                .flatten()
                .is_some_and(|head| is_lfs_pointer(&head))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;
    use crate::load::load;
    use crate::source::{CountingSource, MemSource};

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
    fn the_check_asks_each_export_path_s_head_once_and_reads_no_file_whole() {
        let source = with_media(&[
            ("media/export/pack/surface.png", POINTER),
            ("media/export/pack/hull.glb", b"glTF\x02"),
            ("media/source/spire.blend", POINTER),
        ])
        .with(
            "assets/spire.md",
            "---\nkind: model\nplaceholder: true\nsources: [media/source/spire.blend]\n---\n",
        );
        let (galaxy, _) = load(&source);
        let counting = CountingSource::new(&source);
        assert_eq!(
            lfs_pointers(&galaxy, &counting),
            ["media/export/pack/surface.png"]
        );
        let export: Vec<&str> = galaxy
            .media()
            .into_iter()
            .filter(|path| path.is_export())
            .map(MediaPath::as_str)
            .collect();
        assert!(export.len() >= 3, "{export:?}");
        for path in export {
            assert_eq!(counting.heads_asked(path), 1, "{path}");
        }
        assert_eq!(counting.heads_asked("media/source/spire.blend"), 0);
        assert_eq!(counting.all_bytes_asked(), [], "no file whole");
    }

    #[test]
    fn a_file_of_exactly_a_pointer_s_start_is_a_pointer_and_one_a_byte_short_is_not() {
        // The head asked for is the whole of what a pointer starts with.
        let exactly = with_media(&[("media/export/pack/hull.glb", LFS_POINTER)]);
        assert_eq!(pointers(&exactly), ["media/export/pack/hull.glb"]);
        let short = &LFS_POINTER[..LFS_POINTER.len() - 1];
        let nearly = with_media(&[("media/export/pack/hull.glb", short)]);
        assert_eq!(pointers(&nearly), Vec::<String>::new());
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
    fn lfs_pointers_uses_the_manifest_s_export_rule() {
        let source = with_media(&[
            ("media/export/pack/hull.glb", POINTER),
            ("media/source/spire.blend", POINTER),
        ])
        .with(
            "assets/spire.md",
            "---\nkind: model\nplaceholder: true\nsources: [media/source/spire.blend]\n---\n",
        );
        let listed = pointers(&source);
        assert_eq!(listed, ["media/export/pack/hull.glb"]);
        for path in listed {
            let media = MediaPath::try_from(path.clone()).expect("a media path");
            assert!(media.is_export(), "{path}");
        }
    }

    #[test]
    fn a_pointer_the_manifest_does_not_name_is_not_listed() {
        let source = with_media(&[("media/export/other/stray.glb", POINTER)]);
        assert_eq!(pointers(&source), Vec::<String>::new());
    }

    #[test]
    fn a_sound_s_clip_that_is_a_pointer_is_listed() {
        let source = with_media(&[
            ("media/export/sounds/click.wav", POINTER),
            ("media/export/sounds/hum.wav", b"RIFF"),
        ])
        .with(
            "sounds/click.md",
            "---\nplaceholder: true\nfile: media/export/sounds/click.wav\n---\n",
        )
        .with(
            "sounds/hum.md",
            "---\nplaceholder: true\nfile: media/export/sounds/hum.wav\n---\n",
        );
        assert_eq!(pointers(&source), ["media/export/sounds/click.wav"]);
    }
}
