//! `game.md`: the one content file about the game as a whole, at the
//! data folder's root rather than in a kind's folder.
//!
//! Its front matter names the game, its music ([`Game::music`], a sound
//! like any other) and the sounds its engine plays for its own events
//! ([`EngineSounds`]), and its Markdown body is the game's About text. A
//! data folder without one is fine: the game then has no About text, no
//! music, and its engine plays no sound of its own. One that can't be
//! read or doesn't parse (an unknown role, or one named twice, among
//! them) is reported against `game.md`, marked unreadable, and left out
//! whole, its About text and its sounds with it.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::frontmatter;
use crate::id::Id;
use crate::load::{Doc, Problem};
use crate::source::Source;

/// The game as a whole (`game.md`, at the data folder's root). Its body
/// is the game's About text.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Game {
    /// The game's name (a placeholder).
    pub name: String,
    /// The game's music: a sound's id (`sounds/<id>.md`) whose file is an
    /// `.mp3`, played where the stock game plays its own (at the main
    /// menu, until a pilot enters the game). Left out, the game plays no
    /// music.
    #[serde(default)]
    pub music: Option<Id>,
    /// The sound the engine plays for each of its own events, by role
    /// (`sounds/<id>.md`). Left out, it plays none of them.
    #[serde(default)]
    pub sounds: EngineSounds,
}

/// The sounds the engine plays for its own events, each a sound's id
/// (`sounds/<id>.md`). A role names one sound, and every event that
/// shares the role plays it. A role left out is silent.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EngineSounds {
    /// A button going down: a click on an interface button, and the first
    /// half of a button pressed by its key.
    #[serde(default)]
    pub button_down: Option<Id>,
    /// A button coming up: the end of a click on an interface button, and
    /// the second half of a button pressed by its key.
    #[serde(default)]
    pub button_up: Option<Id>,
    /// The landing: a traffic controller's word (a clearance asked for,
    /// granted or refused) and the touchdown, and every other event that
    /// shares their beep, such as the player's ship disabled, the news
    /// opening and a comm button.
    #[serde(default)]
    pub landing: Option<Id>,
    /// A refusal: a key or a button that can't act (no fuel for a jump or
    /// a gate, nothing to buy, no game to enter), and every other event
    /// that shares its beep, such as a system the map can't find and the
    /// self-destruct's start.
    #[serde(default)]
    pub refused: Option<Id>,
    /// A ship arriving: out of a gate passage, and out of every jump.
    #[serde(default)]
    pub gate_passage: Option<Id>,
    /// An explosion: a ship's wreck exploding, small or large, and an
    /// explosion on board the player's ship.
    #[serde(default)]
    pub explosion: Option<Id>,
}

impl EngineSounds {
    /// Each role and the sound the data names for it.
    #[must_use]
    pub fn roles(&self) -> [(SoundRole, Option<&Id>); 6] {
        [
            (SoundRole::ButtonDown, self.button_down.as_ref()),
            (SoundRole::ButtonUp, self.button_up.as_ref()),
            (SoundRole::Landing, self.landing.as_ref()),
            (SoundRole::Refused, self.refused.as_ref()),
            (SoundRole::GatePassage, self.gate_passage.as_ref()),
            (SoundRole::Explosion, self.explosion.as_ref()),
        ]
    }
}

/// One of the engine's sound roles: a field of [`EngineSounds`], as
/// [`EngineSounds::roles`] gives it. A reader that must fill every role
/// matches on it exhaustively, so a role added here fails to compile there
/// rather than going unfilled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SoundRole {
    /// [`EngineSounds::button_down`].
    ButtonDown,
    /// [`EngineSounds::button_up`].
    ButtonUp,
    /// [`EngineSounds::landing`].
    Landing,
    /// [`EngineSounds::refused`].
    Refused,
    /// [`EngineSounds::gate_passage`].
    GatePassage,
    /// [`EngineSounds::explosion`].
    Explosion,
}

impl SoundRole {
    /// The role's name: the key `game.md`'s `sounds` writes it as.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            SoundRole::ButtonDown => "button_down",
            SoundRole::ButtonUp => "button_up",
            SoundRole::Landing => "landing",
            SoundRole::Refused => "refused",
            SoundRole::GatePassage => "gate_passage",
            SoundRole::Explosion => "explosion",
        }
    }
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
    fn reads_each_role_s_sound() {
        let source = MemSource::default().with(
            "game.md",
            "---\nname: A\nsounds:\n  button_down: click-down\n  button_up: click-up\n\
             \x20 landing: chime\n  refused: buzz\n  gate_passage: hum\n  explosion: blast\n---\n",
        );
        let (game, problems, _) = read_from(&source);
        assert!(problems.is_empty(), "{problems:?}");
        let sounds = game.expect("game.md").data.sounds;
        let named: Vec<_> = sounds
            .roles()
            .into_iter()
            .map(|(role, sound)| (role, role.name(), sound.map(Id::as_str)))
            .collect();
        assert_eq!(
            named,
            [
                (SoundRole::ButtonDown, "button_down", Some("click-down")),
                (SoundRole::ButtonUp, "button_up", Some("click-up")),
                (SoundRole::Landing, "landing", Some("chime")),
                (SoundRole::Refused, "refused", Some("buzz")),
                (SoundRole::GatePassage, "gate_passage", Some("hum")),
                (SoundRole::Explosion, "explosion", Some("blast")),
            ]
        );
    }

    #[test]
    fn each_role_is_read_from_the_key_its_name_gives_and_fills_only_itself() {
        for (role, _) in EngineSounds::default().roles() {
            let source = MemSource::default().with(
                "game.md",
                &format!("---\nname: A\nsounds: {{{}: chime}}\n---\n", role.name()),
            );
            let (game, problems, _) = read_from(&source);
            assert!(problems.is_empty(), "{role:?}: {problems:?}");
            let filled: Vec<_> = (game.expect("game.md").data.sounds.roles().into_iter())
                .filter_map(|(filled, sound)| sound.map(|s| (filled, s.as_str().to_owned())))
                .collect();
            assert_eq!(filled, [(role, "chime".to_owned())], "{role:?}");
        }
    }

    #[test]
    fn a_game_naming_no_sounds_or_some_leaves_the_rest_silent() {
        let source = MemSource::default().with("game.md", "---\nname: A\n---\n");
        let (game, _, _) = read_from(&source);
        assert_eq!(game.expect("game.md").data.sounds, EngineSounds::default());
        assert!(
            EngineSounds::default()
                .roles()
                .iter()
                .all(|(_, s)| s.is_none())
        );
        let source =
            MemSource::default().with("game.md", "---\nname: A\nsounds: {landing: chime}\n---\n");
        let (game, problems, _) = read_from(&source);
        assert!(problems.is_empty(), "{problems:?}");
        let sounds = game.expect("game.md").data.sounds;
        assert_eq!(sounds.landing.as_ref().map(Id::as_str), Some("chime"));
        assert!(sounds.refused.is_none() && sounds.button_down.is_none());
    }

    #[test]
    fn reads_the_music_the_game_names_beside_its_sounds() {
        let source = MemSource::default().with(
            "game.md",
            "---\nname: A\nmusic: theme\nsounds: {landing: chime}\n---\n",
        );
        let (game, problems, _) = read_from(&source);
        assert!(problems.is_empty(), "{problems:?}");
        let game = game.expect("game.md").data;
        assert_eq!(game.music.as_ref().map(Id::as_str), Some("theme"));
        assert_eq!(game.sounds.landing.as_ref().map(Id::as_str), Some("chime"));
    }

    #[test]
    fn a_game_naming_no_music_has_none_and_its_sounds_are_unaffected() {
        let source =
            MemSource::default().with("game.md", "---\nname: A\nsounds: {landing: chime}\n---\n");
        let (game, problems, _) = read_from(&source);
        assert!(problems.is_empty(), "{problems:?}");
        let game = game.expect("game.md").data;
        assert!(game.music.is_none());
        assert_eq!(game.sounds.landing.as_ref().map(Id::as_str), Some("chime"));
    }

    #[test]
    fn music_naming_what_is_not_an_id_is_a_problem() {
        let source = MemSource::default().with("game.md", "---\nname: A\nmusic: Theme\n---\n");
        assert_one_problem(&source, "line 3");
    }

    #[test]
    fn an_unknown_role_is_a_problem_and_leaves_the_file_out() {
        let source = MemSource::default().with(
            "game.md",
            "---\nname: A\nsounds:\n  landing: chime\n  launch: whoosh\n---\nAbout.\n",
        );
        assert_one_problem(&source, "line 5");
        let (_, problems, _) = read_from(&source);
        assert!(problems[0].message.contains("launch"), "{}", problems[0]);
    }

    #[test]
    fn a_role_named_twice_is_a_problem_and_leaves_the_file_out() {
        let source = MemSource::default().with(
            "game.md",
            "---\nname: A\nsounds:\n  landing: chime\n  landing: hum\n---\nAbout.\n",
        );
        assert_one_problem(&source, "line 5");
    }

    #[test]
    fn a_role_naming_what_is_not_an_id_is_a_problem() {
        let source =
            MemSource::default().with("game.md", "---\nname: A\nsounds: {landing: Chime}\n---\n");
        assert_one_problem(&source, "line 3");
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
