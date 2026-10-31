//! The movie player the game plays `dësc` movies through.
//!
//! `main` wires [`movie_player`]: `nova-movie`'s FFmpeg decoder, finding
//! each movie by its bare name in [`movie_folders`], the Plug-Ins folder
//! then the Nova Files, as the original's `_PlayQTMovie` does (@0x559d2,
//! @0x559ff). [`NoMoviePlayer`], which cannot start any movie, stays the
//! router's default, so tests that build the router play nothing: each
//! movie ends at once, as the original's `_PlayQTMovie` returns when its
//! file is not found or does not open (@0x559d2-0x55a5f).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use nova_movie::FfmpegPlayer;
use nova_view::DrawList;
use nova_view::geometry::Bounds;
use nova_view::movie::MoviePlayer;

use crate::config::{Os, plug_ins_dir};

/// The folders movies are looked for in on `os`, in order: the Plug-Ins
/// folder ([`plug_ins_dir`]) when there is one, then `nova_files`.
pub fn movie_folders(
    os: Os,
    env: impl Fn(&str) -> Option<OsString>,
    nova_files: &Path,
) -> Vec<PathBuf> {
    plug_ins_dir(os, env, nova_files)
        .into_iter()
        .chain([nova_files.to_path_buf()])
        .collect()
}

/// The game's movie player: FFmpeg's, finding movies in `folders`, the
/// first folder first.
#[must_use]
pub fn movie_player(folders: Vec<PathBuf>) -> Rc<dyn MoviePlayer> {
    Rc::new(FfmpegPlayer::new(folders))
}

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
    use std::path::Path;

    use super::*;
    use nova_view::geometry::Point;
    use nova_view::movie::fixture::check_contract;

    fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<std::ffi::OsString> {
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        move |name| {
            vars.iter()
                .find(|(var, _)| var == name)
                .map(|(_, value)| value.into())
        }
    }

    #[test]
    fn movies_are_looked_for_in_plug_ins_then_nova_files() {
        let install = Path::new("/games").join("EV Nova");
        let nova_files = install.join("Nova Files");
        assert_eq!(
            movie_folders(Os::Windows, env(&[]), &nova_files),
            [install.join("Plug-ins"), nova_files.clone()]
        );
        let mut plug_ins = Path::new("/Users/p").to_path_buf();
        plug_ins.extend(["Library", "Application Support", "EV Nova", "Plug-Ins"]);
        assert_eq!(
            movie_folders(Os::MacOs, env(&[("HOME", "/Users/p")]), &nova_files),
            [plug_ins, nova_files.clone()]
        );
    }

    #[test]
    fn without_a_plug_ins_folder_movies_are_looked_for_in_nova_files_alone() {
        let nova_files = Path::new("/games").join("Nova Files");
        assert_eq!(
            movie_folders(Os::MacOs, env(&[]), &nova_files),
            std::slice::from_ref(&nova_files)
        );
        assert_eq!(
            movie_folders(Os::Other, env(&[]), Path::new("/")),
            [Path::new("/").to_path_buf()]
        );
    }

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
