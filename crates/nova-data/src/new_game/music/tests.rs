//! The music, over the format's test galaxy (`super::super::tests::galaxy`),
//! with a `game.md` naming it and its file written into the source.

use std::path::Path;

use nova_galaxy_format::source::{MemSource, SourceFile};

use super::super::tests::galaxy;
use super::super::{NewGame, load};

fn loaded(source: &impl Source) -> NewGame {
    load(source, Path::new("games"))
}

fn problems(game: &NewGame) -> Vec<String> {
    game.problems.iter().map(ToString::to_string).collect()
}

use super::*;

/// A track's bytes: what they hold isn't checked by the mapping.
const TRACK: &[u8] = b"\xFF\xFB\x90\x64 a track";

/// A `game.md` naming `music`.
fn game_md(music: &str) -> String {
    format!("---\nname: Test\nmusic: {music}\n---\nAbout.\n")
}

/// The galaxy with `game.md` naming `theme`, and `sounds/theme.md` naming
/// `file`, whose bytes are `bytes` when given.
fn theme_at(file: &str, bytes: Option<&[u8]>) -> MemSource {
    let source = galaxy().with("game.md", &game_md("theme")).with(
        "sounds/theme.md",
        &format!("---\nplaceholder: true\nfile: {file}\n---\n"),
    );
    match bytes {
        Some(bytes) => source.with_bytes(file, bytes),
        None => source,
    }
}

/// The one problem `theme_at(file, bytes)` gives, and that it plays no
/// music.
fn music_problem(file: &str, bytes: Option<&[u8]>) -> Vec<String> {
    let game = loaded(&theme_at(file, bytes));
    assert_eq!(game.data.soundtrack(), None, "no music");
    problems(&game)
}

#[test]
fn the_music_s_mp3_is_the_store_s_soundtrack() {
    let game = loaded(&theme_at("media/export/music/theme.mp3", Some(TRACK)));
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(game.data.soundtrack(), Some(TRACK));
}

#[test]
fn an_upper_case_extension_is_an_mp3_all_the_same() {
    let game = loaded(&theme_at("media/export/music/theme.MP3", Some(TRACK)));
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(game.data.soundtrack(), Some(TRACK));
}

#[test]
fn naming_no_music_is_no_soundtrack_and_no_problem() {
    let with_sound = galaxy()
        .with("game.md", "---\nname: Test\n---\n")
        .with(
            "sounds/theme.md",
            "---\nplaceholder: true\nfile: media/export/music/theme.mp3\n---\n",
        )
        .with_bytes("media/export/music/theme.mp3", TRACK);
    for source in [galaxy(), with_sound] {
        let game = loaded(&source);
        assert_eq!(problems(&game), Vec::<String>::new());
        assert_eq!(game.data.soundtrack(), None);
    }
}

#[test]
fn music_not_in_sounds_is_reported_against_game_md_once() {
    let game = loaded(&galaxy().with("game.md", &game_md("theme")));
    assert_eq!(
        problems(&game),
        ["game.md: its music `theme` is not in sounds/; the game plays no music"]
    );
    assert_eq!(game.data.soundtrack(), None);
}

#[test]
fn music_whose_sound_failed_to_load_is_not_reported_again() {
    let source = galaxy()
        .with("game.md", &game_md("theme"))
        .with("sounds/theme.md", "---\nplaceholder: true\n---\n");
    let game = loaded(&source);
    let ours: Vec<_> = problems(&game)
        .into_iter()
        .filter(|p| p.starts_with("game.md"))
        .collect();
    assert_eq!(ours, Vec::<String>::new());
    assert_eq!(problems(&game).len(), 1, "the loader's own");
    assert_eq!(game.data.soundtrack(), None);
}

#[test]
fn music_that_is_not_an_mp3_is_reported_against_the_sound() {
    assert_eq!(
        music_problem("media/export/sounds/theme.wav", Some(TRACK)),
        [
            "sounds/theme.md: its file `media/export/sounds/theme.wav` isn't an MP3 file; \
             the game plays no music"
        ]
    );
    assert_eq!(
        music_problem("media/export/sounds/theme", Some(TRACK)),
        [
            "sounds/theme.md: its file `media/export/sounds/theme` isn't an MP3 file; \
             the game plays no music"
        ]
    );
}

#[test]
fn music_outside_the_export_media_is_reported_against_the_sound() {
    assert_eq!(
        music_problem("media/source/music/theme.mp3", Some(TRACK)),
        [
            "sounds/theme.md: its file `media/source/music/theme.mp3` isn't under \
             media/export/; the game plays no music"
        ]
    );
}

#[test]
fn music_that_is_not_there_is_reported_against_the_sound() {
    assert_eq!(
        music_problem("media/export/music/theme.mp3", None),
        [
            "sounds/theme.md: its file `media/export/music/theme.mp3` isn't there; \
             the game plays no music"
        ]
    );
}

#[test]
fn music_that_is_an_lfs_pointer_plays_none_and_is_listed_among_the_pointers() {
    let pointer = b"version https://git-lfs.github.com/spec/v1\noid sha256:00\nsize 1\n";
    let game = loaded(&theme_at("media/export/music/theme.mp3", Some(pointer)));
    assert_eq!(problems(&game), Vec::<String>::new());
    assert_eq!(game.lfs_pointers, ["media/export/music/theme.mp3"]);
    assert_eq!(game.data.soundtrack(), None);
}

/// A source whose reads of `media/export/music/` fail.
struct Unreadable(MemSource);

impl Source for Unreadable {
    fn files(&self, dir: &str, ext: &str) -> std::io::Result<Vec<SourceFile>> {
        self.0.files(dir, ext)
    }

    fn bytes(&self, path: &str) -> std::io::Result<Option<Vec<u8>>> {
        if path.starts_with("media/export/music/") {
            return Err(std::io::Error::other("the disk is unreadable"));
        }
        self.0.bytes(path)
    }
}

#[test]
fn music_that_cannot_be_read_is_reported_against_the_sound() {
    let source = Unreadable(theme_at("media/export/music/theme.mp3", Some(TRACK)));
    let game = loaded(&source);
    assert_eq!(
        problems(&game),
        [
            "sounds/theme.md: its file `media/export/music/theme.mp3` couldn't be read (the \
             disk is unreadable); the game plays no music"
        ]
    );
    assert_eq!(game.data.soundtrack(), None);
}

#[test]
fn a_track_a_role_also_names_is_the_soundtrack_and_its_cue_is_reported_once() {
    // The same entry as both: the role's WAV check fails it, the music
    // keeps it. Each use reports only its own consequence.
    let source = theme_at("media/export/music/theme.mp3", Some(TRACK)).with(
        "game.md",
        "---\nname: Test\nmusic: theme\nsounds: {landing: theme}\n---\n",
    );
    let game = loaded(&source);
    assert_eq!(game.data.soundtrack(), Some(TRACK));
    let problems = problems(&game);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with(
            "sounds/theme.md: its file `media/export/music/theme.mp3` doesn't decode as a WAV"
        ),
        "{problems:?}"
    );
}
