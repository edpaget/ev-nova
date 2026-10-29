//! The movie player the game plays `dësc` movies through.
//!
//! The engine has no QuickTime decoder, and the data store skips movie
//! files, so [`NoMoviePlayer`] cannot start any movie: each ends at once,
//! as the original's `_PlayQTMovie` returns when its file is not found or
//! does not open (@0x559d2-0x55a5f), and whatever comes after it shows.
//! rdm:phase/movies/phase-2-ffmpeg-decoder's `nova-movie` replaces this
//! player: it decodes each frame onto a [`nova_view::MovieScreen`] and
//! draws it through [`nova_view::DrawCommand::MovieFrame`].

use std::time::Duration;

use nova_view::DrawList;
use nova_view::geometry::Bounds;
use nova_view::movie::MoviePlayer;

/// A movie player that plays nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoMoviePlayer;

impl MoviePlayer for NoMoviePlayer {
    /// No movie can start.
    fn start(&self, _file: &str) -> Option<(u32, u32)> {
        None
    }

    /// Never reached: nothing started.
    fn advance(&self, _dt: Duration) -> bool {
        false
    }

    /// Never reached: nothing started.
    fn stop(&self) {}

    /// Never reached: nothing started.
    fn draw_frame(&self, _rect: Bounds, _list: &mut DrawList) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_view::geometry::Point;
    use nova_view::movie::fixture::check_contract;

    #[test]
    fn no_movie_player_meets_the_movie_player_contract() {
        assert_eq!(check_contract(&NoMoviePlayer, "Race 1.mov"), Ok(()));
    }

    #[test]
    fn no_movie_starts_and_nothing_is_drawn() {
        let player = NoMoviePlayer;
        assert_eq!(player.start("Race 1.mov"), None);
        assert_eq!(player.start("\u{1}"), None);
        assert!(!player.advance(Duration::from_millis(16)));
        player.stop();
        let mut list = DrawList::new();
        player.draw_frame(Bounds::at(Point::new(0.0, 0.0), 10.0, 10.0), &mut list);
        assert!(list.is_empty());
    }
}
