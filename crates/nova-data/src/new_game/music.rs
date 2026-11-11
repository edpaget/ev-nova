//! The new game's music: the `sounds/` entry `game.md`'s `music` names,
//! read as an MP3 file's bytes for the store's soundtrack
//! ([`GameData::soundtrack`](crate::GameData::soundtrack)).
//!
//! The game has one track, played where the stock game plays
//! `Nova Music.mp3`: start-up hands its bytes to the audio side, which
//! streams them at the main menu until a pilot enters the game, once,
//! unlooped, at the music volume, as it streams the stock track. With no
//! `music` named the game plays none, and nothing is reported.
//!
//! Whether the bytes decode isn't checked here, as it isn't for the stock
//! track ([`crate::music`]): the player's MP3 decoder is the judge, and
//! the audio side warns once when it can't play them.
//!
//! # Problems
//!
//! Each is reported once, and the game plays no music:
//!
//! - `music` naming no `sounds/` entry, against `game.md` (an entry that
//!   failed to load isn't reported again);
//! - the sound's file that isn't an `.mp3`, against the sound: the music
//!   is streamed only as MP3;
//! - the sound's file that isn't under `media/export/`, isn't there or
//!   can't be read, against the sound.
//!
//! A file that is a Git LFS pointer plays no music and isn't reported:
//! the load lists it among the pointers ([`super::NewGame::lfs_pointers`]).

use nova_galaxy_format::load::{Galaxy, Problem};
use nova_galaxy_format::source::Source;

use super::map::absent;
use super::sounds::export_bytes;

/// What follows from a problem with the music.
const NO_MUSIC: &str = "the game plays no music";

/// The bytes of the MP3 file `galaxy`'s `game.md` names as its `music`,
/// read from `source`; none when it names none, or when they can't be
/// had, reported in `problems` (see the module docs).
pub(super) fn read(
    galaxy: &Galaxy,
    source: &impl Source,
    problems: &mut Vec<Problem>,
) -> Option<Vec<u8>> {
    let game = galaxy.game.as_ref()?;
    let music = game.data.music.as_ref()?;
    let Some(doc) = galaxy.sounds.get(music) else {
        if absent(galaxy, &galaxy.sounds, "sounds", music) {
            problems.push(Problem::new(
                &game.path,
                format!("its music `{music}` is not in sounds/; {NO_MUSIC}"),
            ));
        }
        return None;
    };
    let file = &doc.data.file;
    let mut none = |why: &str| {
        problems.push(Problem::new(
            &doc.path,
            format!("its file `{file}` {why}; {NO_MUSIC}"),
        ));
        None
    };
    if file.extension() != "mp3" {
        return none("isn't an MP3 file");
    }
    match export_bytes(file, source) {
        Ok(bytes) => bytes,
        Err(why) => none(&why),
    }
}

#[cfg(test)]
mod tests;
