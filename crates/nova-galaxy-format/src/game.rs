//! `game.md`: the one content file about the game as a whole, at the
//! data folder's root rather than in a kind's folder.
//!
//! Its front matter names the game, and its Markdown body is the game's
//! About text. A data folder without one is fine: the game then has no
//! About text. One that can't be read or doesn't parse is reported
//! against `game.md`, marked unreadable, and left out.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::frontmatter;
use crate::load::{Doc, Problem};
use crate::source::Source;

/// The game as a whole (`game.md`, at the data folder's root). Its body
/// is the game's About text.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Game {
    /// The game's name (a placeholder).
    pub name: String,
}

/// `game.md`'s path in the data folder.
pub const PATH: &str = "game.md";

/// Reads `game.md` from `source`: `None` when there is none, or when it
/// can't be read or doesn't parse, which is reported in `problems` and
/// marks it in `unreadable`.
pub(crate) fn read(
    source: &impl Source,
    problems: &mut Vec<Problem>,
    unreadable: &mut BTreeSet<String>,
) -> Option<Doc<Game>> {
    let loaded = match source.bytes(PATH) {
        Ok(None) => return None,
        Ok(Some(bytes)) => String::from_utf8(bytes)
            .map_err(|e| format!("couldn't be read: {e}"))
            .and_then(|text| {
                frontmatter::parse::<Game>(&text).map(|(data, body)| (data, body.to_string()))
            }),
        Err(e) => Err(format!("couldn't be read: {e}")),
    };
    match loaded {
        Ok((data, body)) => Some(Doc {
            path: PATH.to_string(),
            data,
            body,
        }),
        Err(message) => {
            unreadable.insert(PATH.to_string());
            problems.push(Problem::new(PATH, message));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;
    use crate::source::{MemSource, SourceFile};

    fn read_from(source: &impl Source) -> (Option<Doc<Game>>, Vec<Problem>, BTreeSet<String>) {
        let mut problems = Vec::new();
        let mut unreadable = BTreeSet::new();
        let game = read(source, &mut problems, &mut unreadable);
        (game, problems, unreadable)
    }

    #[test]
    fn reads_the_name_and_the_body() {
        let source = MemSource::default().with(
            "game.md",
            "---\nname: Hearthlight\n---\nA game about haulers.\n",
        );
        let (game, problems, unreadable) = read_from(&source);
        assert!(problems.is_empty(), "{problems:?}");
        assert!(unreadable.is_empty(), "{unreadable:?}");
        let game = game.expect("game.md");
        assert_eq!(game.path, "game.md");
        assert_eq!(game.data.name, "Hearthlight");
        assert_eq!(game.body, "A game about haulers.\n");
    }

    #[test]
    fn no_game_file_is_no_game_and_no_problem() {
        let (game, problems, unreadable) = read_from(&MemSource::default());
        assert!(game.is_none());
        assert!(problems.is_empty(), "{problems:?}");
        assert!(unreadable.is_empty(), "{unreadable:?}");
    }

    /// Asserts `source`'s `game.md` gives one problem against it, starting
    /// with `start`, and no game.
    fn assert_one_problem(source: &impl Source, start: &str) {
        let (game, problems, unreadable) = read_from(source);
        assert!(game.is_none());
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].file, "game.md");
        assert!(problems[0].message.starts_with(start), "{}", problems[0]);
        assert!(unreadable.contains("game.md"), "{unreadable:?}");
    }

    #[test]
    fn front_matter_that_does_not_parse_is_a_problem() {
        let source = MemSource::default().with("game.md", "---\nname: A\ntitle: B\n---\n");
        assert_one_problem(&source, "line 3");
    }

    #[test]
    fn text_that_is_not_utf8_is_a_problem() {
        let source = MemSource::default().with_bytes("game.md", b"\xff");
        assert_one_problem(&source, "couldn't be read: ");
    }

    /// A source whose every read fails.
    struct Broken;
    impl Source for Broken {
        fn files(&self, _: &str, _: &str) -> io::Result<Vec<SourceFile>> {
            Ok(Vec::new())
        }
        fn bytes(&self, _: &str) -> io::Result<Option<Vec<u8>>> {
            Err(io::Error::other("the disk is unreadable"))
        }
    }

    #[test]
    fn a_read_error_is_a_problem() {
        assert_one_problem(&Broken, "couldn't be read: the disk is unreadable");
    }
}
