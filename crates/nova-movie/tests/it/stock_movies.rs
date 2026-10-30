//! The original's movies, `Race 1.mov` to `Race 4.mov` in `Nova Files`:
//! each decodes to its end, every frame at 768x384 in display order, and
//! the player meets the movie player contract on it. Skips, passing,
//! when `NOVA_DATA` is unset.

use std::time::Instant;

use nova_movie::FfmpegPlayer;
use nova_movie::ffmpeg::MovieFile;
use nova_movie::source::FrameSource;
use nova_view::movie::fixture::check_contract;

use crate::common::nova_data;

const MOVIES: [&str; 4] = ["Race 1.mov", "Race 2.mov", "Race 3.mov", "Race 4.mov"];

#[test]
fn every_frame_of_each_stock_movie_decodes_in_order() {
    let Some(dir) = nova_data() else {
        return;
    };
    for name in MOVIES {
        let started = Instant::now();
        let mut movie = MovieFile::open(&dir.join(name)).expect(name);
        assert_eq!(movie.size(), (768, 384), "{name}");
        let expected = movie.frames();
        let mut frames = 0;
        let mut last = None;
        while let Some(frame) = movie.next_frame().expect(name) {
            assert_eq!((frame.width, frame.height), (768, 384), "{name}");
            if let Some(last) = last {
                assert!(frame.at > last, "{name}: {:?} after {last:?}", frame.at);
            }
            last = Some(frame.at);
            frames += 1;
        }
        assert!(frames > 0, "{name}");
        assert_eq!(Some(frames), expected, "{name}: every frame decodes");
        eprintln!(
            "{name}: {frames} frames, {:?} of movie, decoded in {:?}",
            movie.duration(),
            started.elapsed()
        );
    }
}

#[test]
fn the_player_meets_the_contract_on_each_stock_movie() {
    let Some(dir) = nova_data() else {
        return;
    };
    let player = FfmpegPlayer::new(vec![dir]);
    for name in MOVIES {
        assert_eq!(check_contract(&player, name), Ok(()), "{name}");
    }
}
