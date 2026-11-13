//! The new game's music reaches the audio side: over the checked-in
//! fixture (`nova_data::new_game::fixture::folder`), opened as the app
//! opens it (`DiskData::open_new_game`, which asserts the load reports
//! no problem), start-up's music (`nova::audio::game_music`) is what
//! `main` hands `KiraAudio::open` unchanged.
//!
//! - The fixture's `game.md` names `music`: start-up gives that sound's
//!   file's bytes, with no warning.
//! - A copy of the fixture whose `game.md` names none: the load reports
//!   nothing, and start-up gives no music and no warning.
//!
//! The track's name is read from the fixture's `game.md` and its bytes
//! from the file its `sounds/` entry names, none retyped. How `game.md`
//! maps is nova-data's and how a track streams nova-audio's, each pinned
//! there; this pins that they meet.

use std::fs;

use nova::audio::game_music;
use nova::ruleset::Ruleset;
use nova_data::new_game::fixture::folder;

use crate::new_game_conversation::opened_from;
use crate::new_game_saved_ids::copy;
use crate::new_game_sounds::clip_bytes;
use crate::new_game_travel::fixture;

#[test]
fn start_up_hands_the_audio_side_the_track_game_md_names() {
    let game = fixture();
    let music = (game.galaxy.game.as_ref().expect("game.md").data.music)
        .as_ref()
        .expect("the fixture names its music");
    let data = opened_from(&folder());
    assert_eq!(
        game_music(Ruleset::NewGame, &data, &folder()),
        (Some(clip_bytes(&game, music.as_str())), None)
    );
}

#[test]
fn a_new_game_naming_no_music_plays_none_and_warns_of_nothing() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let copied = dir.path().join("new-game");
    copy(&folder(), &copied);
    let game_md = copied.join("game.md");
    let text = fs::read_to_string(&game_md).expect("reads game.md");
    let without: String = text
        .split_inclusive('\n')
        .filter(|line| !line.starts_with("music:"))
        .collect();
    assert_eq!(
        text.lines().count(),
        without.lines().count() + 1,
        "the fixture names its music on one line"
    );
    fs::write(&game_md, &without).expect("writes game.md");
    // Asserts the load reports no problem, so nothing about music.
    let data = opened_from(&copied);
    assert_eq!(game_music(Ruleset::NewGame, &data, &copied), (None, None));
}
