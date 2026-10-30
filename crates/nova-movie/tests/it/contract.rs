//! The FFmpeg player meets `nova-view`'s movie player contract on the
//! fixture, found by name in the folders it is given.

use nova_movie::FfmpegPlayer;
use nova_view::movie::MoviePlayer;
use nova_view::movie::fixture::check_contract;

use crate::decode::fixtures;

#[test]
fn the_fixture_meets_the_movie_player_contract() {
    let player = FfmpegPlayer::new(vec![fixtures()]);
    assert_eq!(check_contract(&player, "main-bframes.mov"), Ok(()));
}

#[test]
fn the_fixture_starts_at_its_natural_size() {
    let player = FfmpegPlayer::new(vec![fixtures()]);
    assert_eq!(player.start("main-bframes.mov"), Some((64, 32)));
    player.stop();
}

#[test]
fn a_name_in_neither_folder_doesn_t_start_and_meets_the_contract() {
    let empty = tempfile::tempdir().expect("temp dir");
    let player = FfmpegPlayer::new(vec![empty.path().to_path_buf(), fixtures()]);
    assert_eq!(player.start("Race 1.mov"), None);
    assert_eq!(check_contract(&player, "Race 1.mov"), Ok(()));
}

#[test]
fn a_file_that_isn_t_a_movie_doesn_t_start() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join("Race 1.mov"), b"not a movie").expect("write");
    let player = FfmpegPlayer::new(vec![dir.path().to_path_buf()]);
    assert_eq!(player.start("Race 1.mov"), None);
    assert_eq!(check_contract(&player, "Race 1.mov"), Ok(()));
}
