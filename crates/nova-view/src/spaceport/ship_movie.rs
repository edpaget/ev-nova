//! The shipyard's and the hire screen's looping ship movie: the selected
//! ship's `MovieFile` played over and over in the picture item (item 8),
//! as `_StartLoopingMovie` (@0x60266), `_LoopingMovieIdle` (@0x558d9) and
//! `_ShipyardFilter` (@0x60449) play it, through the [`Movies`]' looping
//! player ([`Movies::looping`]).
//!
//! - **The files.** `_LoadObjectData` copies each `shïp`'s `MovieFile`
//!   (offset 0x6AE) into its 256-byte entry of `_shipMovieFilenames`
//!   (@0x7b0ce-0x7b136); an empty one names no movie (the Pascal length
//!   byte, @0x60858-0x60866, @0x60c77-0x60c85). They are read once, as a
//!   screen opens ([`ShipMovieFiles`](super::shipyard::ShipMovieFiles)).
//! - **Starting.** A key or a press that moves the selection to another
//!   ship stops the loop playing (@0x6082e-0x6083f, @0x60c4d-0x60c5e),
//!   then, for a ship whose entry isn't empty, starts its file in item 8
//!   (@0x6088a-0x608b0, @0x60c68-0x60cd2); a selection that keeps the same
//!   ship touches nothing, and an empty cell or a scroll that clears the
//!   selection (@0x5f5c2-0x5f5d3, @0x5f674-0x5f685) only stops it
//!   ([`ShipMovie::select`]). `_StartLoopingMovie` returns at once with
//!   QuickTime Movies off (`*0x3b501c + 7`, @0x60282-0x6028b), disposes
//!   any loop playing (@0x60291-0x602a2), finds the file by its bare name
//!   in the Plug-Ins folder, then the Nova Files folder (@0x602f5,
//!   @0x60322, the player's job), and plays it stretched to item 8's rect
//!   (`SetMovieBox` @0x603bc) from its start (@0x6040c-0x6042e). A file
//!   that can't be found or opened plays nothing (@0x60435), and item 8
//!   draws as without one.
//! - **Looping.** `hintsLoop` and `loopTimeBase` (@0x603d9-0x603ff) loop
//!   it; the port starts the file again as the player reports it ended
//!   ([`ShipMovie::idle`]).
//! - **Servicing.** `_LoopingMovieIdle`'s `MoviesTask` (@0x558f3) is
//!   called only from the top of `_ShipyardFilter` (@0x6046e), on each
//!   event the shipyard or hire dialog itself takes, with no 6-tick
//!   throttle ([`ShipMovie::idle`]). While a dialog covers it (the Info
//!   panel, the name prompt, an offer, a text), nothing services it and
//!   item 8 keeps the frame last drawn; its time base runs on with the
//!   clock, so the first service after shows the frame due by then
//!   ([`ShipMovie::covered`]). Nothing stops it there: Info
//!   (`_DoShipyardDescription`, @0x5f517-0x5f535) leaves it playing.
//! - **Ending.** The dialog's close stops it before
//!   `DisposeDialogNoFlash`, however it leaves (@0x5f73f-0x5f750): Done,
//!   a purchase or a hire, a pending leave ([`ShipMovie::end`]).
//!   `_EndLoopingMovie` (@0x55934) has the same body and no caller.
//! - **Drawing.** With a ship selected and a loop playing,
//!   `_ShipyardDialogUpdate` copies the movie's current frame into item 8
//!   and draws neither the picture nor "No Picture" / "Available"
//!   (@0x59042-0x590a3 → @0x59238, [`ShipMovie::draw`]).
//!
//! The movie code reads no `_shipBuyMode`, so the hire screen loops its
//! ships' movies exactly as the shipyard does. None of these routines
//! plays a sound; a movie's own sound track is
//! rdm:phase/movies-fidelity/phase-9-movie-sound-track's.
//!
//! Left out:
//!
//! - `_StartLoopingMovie` clears `_introMusicMovie` (@0x60271-0x6027c)
//!   without disposing it: the intro music has been disposed before any
//!   landing (`_StopIntroMusic` @0x61d49-0x61d56), so it changes nothing.
//! - A `MovieFile` filling all 32 bytes lets `strcpy` run on into
//!   `WeapType` (@0x7b0e2): the original's limit; nova-data stops at 32.
//! - The key path's move to no ship skips the stop (@0x607cc → @0x608b7),
//!   but the key loop never lands on an empty cell while the page shows a
//!   ship (@0x606f1-0x6072f, @0x6077f-0x607b7): unreachable.
//! - `_LoopingMovieIdle`'s 30-tick sleep in the background
//!   (@0x558f8-0x5592d) is rdm:phase/play-feel-5-followups/phase-20-background-pauses-the-game's.
//! - `loopTimeBase` wraps the time past the end into the next pass
//!   (@0x603e6-0x603ff); the port starts the file again from its start,
//!   which differs only after a cover longer than the rest of the movie
//!   (rdm:phase/movies-fidelity/phase-8-looping-movie-wrap, which also
//!   owes the loop the time an overlay the router opens covers it).

use std::rc::Rc;
use std::time::Duration;

use nova_sim::ShipId;

use crate::draw::DrawList;
use crate::geometry::Bounds;
use crate::movie::{MoviePlayer, Movies};

/// A loop playing: the player it started on and its file.
#[derive(Clone)]
struct Playing {
    player: Rc<dyn MoviePlayer>,
    file: String,
}

/// The selected ship's looping movie (see the [module docs](self)).
#[derive(Clone)]
pub struct ShipMovie {
    movies: Movies,
    /// Each ship naming a movie, and its file.
    files: Vec<(ShipId, String)>,
    /// The ship last selected, whether or not its movie could start.
    selected: Option<ShipId>,
    /// The loop playing, if any.
    playing: Option<Playing>,
    /// The time the loop's time base ran on while covered, not yet shown.
    owed: Duration,
}

impl std::fmt::Debug for ShipMovie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShipMovie")
            .field("movies", &self.movies)
            .field("files", &self.files)
            .field("selected", &self.selected)
            .field("playing", &self.playing())
            .field("owed", &self.owed)
            .finish()
    }
}

impl ShipMovie {
    /// No movie playing yet, for the ships naming `files`, until movies
    /// are given ([`ShipMovie::set_movies`]).
    #[must_use]
    pub fn new(files: Vec<(ShipId, String)>) -> Self {
        Self {
            movies: Movies::none(),
            files,
            selected: None,
            playing: None,
            owed: Duration::ZERO,
        }
    }

    /// Plays the loops through `movies`: any loop playing stops, and the
    /// ship selected starts its own, as if just selected.
    pub fn set_movies(&mut self, movies: Movies) {
        self.movies = movies;
        self.restart();
    }

    /// The selection is now `ship`: another ship stops the loop playing
    /// and starts its own, if it names one and movies are on; the same
    /// ship does nothing (`_ShipyardFilter` @0x6082e-0x608b0,
    /// @0x60c4d-0x60cd2).
    pub fn select(&mut self, ship: Option<ShipId>) {
        if ship == self.selected {
            return;
        }
        self.selected = ship;
        self.restart();
    }

    /// Stops the loop playing, if any, and starts the selected ship's.
    fn restart(&mut self) {
        self.end();
        let Some(file) = self.selected.and_then(|ship| self.file_of(ship)) else {
            return;
        };
        let Some(player) = self.movies.looping() else {
            return;
        };
        if player.start(&file).is_some() {
            self.playing = Some(Playing { player, file });
        }
    }

    /// The file `ship` names, if any.
    fn file_of(&self, ship: ShipId) -> Option<String> {
        self.files
            .iter()
            .find(|(named, _)| *named == ship)
            .map(|(_, file)| file.clone())
    }

    /// The dialog takes an event `dt` after the last: the loop moves on by
    /// that and by any time owed from a cover (`_LoopingMovieIdle`'s
    /// `MoviesTask` @0x558f3), and an ended loop starts again from its
    /// start, or, failing that, stops.
    pub fn idle(&mut self, dt: Duration) {
        let owed = std::mem::take(&mut self.owed);
        let Some(playing) = &self.playing else {
            return;
        };
        if playing.player.advance(owed.saturating_add(dt)) {
            return;
        }
        if playing.player.start(&playing.file).is_none() {
            self.playing = None;
        }
    }

    /// Something covers the dialog for `dt`: the loop isn't serviced, so
    /// its frame holds, but its time base runs on, owed to the next
    /// [`ShipMovie::idle`].
    pub fn covered(&mut self, dt: Duration) {
        if self.playing.is_some() {
            self.owed = self.owed.saturating_add(dt);
        }
    }

    /// The dialog closes: the loop playing stops (@0x5f73f-0x5f750).
    pub fn end(&mut self) {
        self.owed = Duration::ZERO;
        if let Some(playing) = self.playing.take() {
            playing.player.stop();
        }
    }

    /// The file of the loop playing, if any.
    #[must_use]
    pub fn playing(&self) -> Option<&str> {
        self.playing.as_ref().map(|playing| playing.file.as_str())
    }

    /// Draws the loop's current frame stretched into `rect`, item 8, and
    /// true; with none playing, nothing, and false, for the picture or
    /// "No Picture" to be drawn instead (`_ShipyardDialogUpdate`
    /// @0x59042-0x590a3).
    pub fn draw(&self, rect: Bounds, list: &mut DrawList) -> bool {
        let Some(playing) = &self.playing else {
            return false;
        };
        playing.player.draw_frame(rect, list);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::Point;
    use crate::movie::fixture::{Call, MockMoviePlayer};

    const TICK: Duration = Duration::from_millis(100);

    /// Ship `id` selected.
    #[allow(clippy::unnecessary_wraps)]
    fn ship(id: i16) -> Option<ShipId> {
        Some(ShipId(id))
    }

    /// Ships 128 ("Ship.mov") and 130 ("Other.mov") name movies; 129
    /// doesn't.
    fn files() -> Vec<(ShipId, String)> {
        vec![
            (ShipId(128), "Ship.mov".to_owned()),
            (ShipId(130), "Other.mov".to_owned()),
        ]
    }

    fn movies(main: &Rc<MockMoviePlayer>, reel: &Rc<MockMoviePlayer>, on: bool) -> Movies {
        Movies::new(Rc::clone(main) as Rc<dyn MoviePlayer>, on)
            .with_looping_player(Rc::clone(reel) as Rc<dyn MoviePlayer>)
    }

    /// A [`ShipMovie`] over [`files`] looping on `reel`, movies on, and
    /// the main player, which it must never touch.
    fn looping_on(reel: &Rc<MockMoviePlayer>) -> (ShipMovie, Rc<MockMoviePlayer>) {
        let main = MockMoviePlayer::playing((640, 480), 5);
        let mut movie = ShipMovie::new(files());
        movie.set_movies(movies(&main, reel, true));
        (movie, main)
    }

    fn start(file: &str) -> Call {
        Call::Start(file.to_owned())
    }

    fn rect() -> Bounds {
        Bounds::at(Point::new(10.0, 20.0), 200.0, 200.0)
    }

    /// What `movie` draws into [`rect`], and whether it drew.
    fn drawn(movie: &ShipMovie) -> (Vec<Bounds>, bool) {
        let mut list = DrawList::new();
        let drew = movie.draw(rect(), &mut list);
        let areas = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::MovieFrame { area, .. } => Some(*area),
                _ => None,
            })
            .collect();
        (areas, drew)
    }

    #[test]
    fn selecting_a_ship_naming_a_movie_starts_it_on_the_looping_player() {
        let reel = MockMoviePlayer::playing((32, 32), 3);
        let (mut movie, main) = looping_on(&reel);
        assert!(reel.calls().is_empty(), "nothing selected yet");
        movie.select(ship(128));
        assert_eq!(reel.calls(), [start("Ship.mov")]);
        assert_eq!(movie.playing(), Some("Ship.mov"));
        assert!(main.calls().is_empty(), "the main player is the texts'");
    }

    #[test]
    fn a_ship_without_a_file_movies_off_or_no_looping_player_starts_nothing() {
        let reel = MockMoviePlayer::playing((32, 32), 3);
        let (mut movie, _) = looping_on(&reel);
        movie.select(ship(129));
        assert_eq!(movie.playing(), None);
        let main = MockMoviePlayer::playing((640, 480), 5);
        let mut off = ShipMovie::new(files());
        off.set_movies(movies(&main, &reel, false));
        off.select(ship(128));
        assert_eq!(off.playing(), None, "QuickTime Movies off (@0x60282)");
        let mut alone = ShipMovie::new(files());
        alone.set_movies(Movies::new(Rc::clone(&main) as Rc<dyn MoviePlayer>, true));
        alone.select(ship(128));
        assert_eq!(alone.playing(), None);
        let mut unset = ShipMovie::new(files());
        unset.select(ship(128));
        assert_eq!(unset.playing(), None, "no movies given");
        assert!(reel.calls().is_empty());
        assert!(main.calls().is_empty());
    }

    #[test]
    fn a_player_that_cannot_start_leaves_nothing_playing_or_drawn() {
        let reel = MockMoviePlayer::refusing();
        let (mut movie, _) = looping_on(&reel);
        movie.select(ship(128));
        assert_eq!(reel.calls(), [start("Ship.mov")]);
        assert_eq!(movie.playing(), None);
        assert_eq!(drawn(&movie), (vec![], false));
        movie.idle(TICK);
        movie.end();
        assert_eq!(
            reel.calls(),
            [start("Ship.mov")],
            "nothing to advance or stop"
        );
    }

    #[test]
    fn the_same_ship_again_does_not_restart_it() {
        let reel = MockMoviePlayer::playing((32, 32), 3);
        let (mut movie, _) = looping_on(&reel);
        movie.select(ship(128));
        movie.select(ship(128));
        assert_eq!(reel.calls(), [start("Ship.mov")]);
        let refusing = MockMoviePlayer::refusing();
        let (mut refused, _) = looping_on(&refusing);
        refused.select(ship(128));
        refused.select(ship(128));
        assert_eq!(
            refusing.calls(),
            [start("Ship.mov")],
            "the same ship, though none could start"
        );
    }

    #[test]
    fn another_ship_stops_the_loop_then_starts_its_own_and_none_only_stops() {
        let reel = MockMoviePlayer::playing((32, 32), 3);
        let (mut movie, _) = looping_on(&reel);
        movie.select(ship(128));
        movie.select(ship(130));
        assert_eq!(
            reel.calls(),
            [start("Ship.mov"), Call::Stop, start("Other.mov")]
        );
        movie.select(ship(129));
        movie.select(None);
        assert_eq!(reel.calls()[3..], [Call::Stop], "129 names none");
        assert_eq!(movie.playing(), None);
        movie.select(ship(128));
        movie.select(None);
        assert_eq!(reel.calls()[4..], [start("Ship.mov"), Call::Stop]);
    }

    #[test]
    fn idle_advances_by_the_time_and_an_ended_loop_starts_again() {
        let reel = MockMoviePlayer::playing((32, 32), 2);
        let (mut movie, _) = looping_on(&reel);
        movie.select(ship(128));
        movie.idle(TICK);
        assert_eq!(movie.playing(), Some("Ship.mov"));
        movie.idle(TICK);
        assert_eq!(
            reel.calls(),
            [
                start("Ship.mov"),
                Call::Advance,
                Call::Advance,
                start("Ship.mov")
            ],
            "hintsLoop (@0x603d9): no stop between"
        );
        assert_eq!(reel.advanced_by(), [TICK, TICK]);
        assert_eq!(movie.playing(), Some("Ship.mov"));
        movie.idle(TICK);
        assert_eq!(reel.calls()[4..], [Call::Advance], "playing on");
    }

    #[test]
    fn an_ended_loop_that_cannot_start_again_stops_playing() {
        /// Plays its first start for one advance, then refuses.
        struct Once(Rc<MockMoviePlayer>, std::cell::Cell<bool>);
        impl MoviePlayer for Once {
            fn start(&self, file: &str) -> Option<(u32, u32)> {
                let size = self.0.start(file);
                (!self.1.replace(true)).then_some(size).flatten()
            }
            fn advance(&self, dt: Duration) -> bool {
                self.0.advance(dt)
            }
            fn stop(&self) {
                self.0.stop();
            }
            fn draw_frame(&self, rect: Bounds, list: &mut DrawList) {
                self.0.draw_frame(rect, list);
            }
        }
        let inner = MockMoviePlayer::playing((32, 32), 1);
        let once = Rc::new(Once(Rc::clone(&inner), std::cell::Cell::new(false)));
        let main = MockMoviePlayer::playing((640, 480), 5);
        let mut movie = ShipMovie::new(files());
        movie.set_movies(
            Movies::new(main as Rc<dyn MoviePlayer>, true)
                .with_looping_player(once as Rc<dyn MoviePlayer>),
        );
        movie.select(ship(128));
        movie.idle(TICK);
        assert_eq!(movie.playing(), None, "the file went (@0x60435)");
        assert_eq!(drawn(&movie), (vec![], false));
    }

    #[test]
    fn covered_holds_the_frame_and_the_next_idle_advances_by_the_sum() {
        let reel = MockMoviePlayer::playing((32, 32), 10);
        let (mut movie, _) = looping_on(&reel);
        movie.select(ship(128));
        movie.covered(TICK);
        movie.covered(TICK);
        assert_eq!(
            reel.calls(),
            [start("Ship.mov")],
            "neither advanced nor stopped"
        );
        assert_eq!(drawn(&movie), (vec![rect()], true), "its frame holds");
        movie.idle(Duration::from_millis(16));
        assert_eq!(reel.advanced_by(), [Duration::from_millis(216)]);
        movie.idle(TICK);
        assert_eq!(reel.advanced_by()[1..], [TICK], "owed once");
    }

    #[test]
    fn nothing_is_owed_without_a_loop_or_across_a_new_selection() {
        let reel = MockMoviePlayer::playing((32, 32), 10);
        let (mut movie, _) = looping_on(&reel);
        movie.covered(TICK);
        movie.select(ship(128));
        movie.idle(TICK);
        assert_eq!(reel.advanced_by(), [TICK], "nothing playing while covered");
        movie.covered(TICK);
        movie.select(ship(130));
        movie.idle(TICK);
        assert_eq!(reel.advanced_by()[1..], [TICK], "the new loop owes nothing");
    }

    #[test]
    fn end_stops_it_and_twice_is_harmless() {
        let reel = MockMoviePlayer::playing((32, 32), 3);
        let (mut movie, _) = looping_on(&reel);
        movie.select(ship(128));
        movie.end();
        movie.end();
        assert_eq!(reel.calls(), [start("Ship.mov"), Call::Stop]);
        assert_eq!(movie.playing(), None);
        assert_eq!(drawn(&movie), (vec![], false), "the picture comes back");
    }

    #[test]
    fn draw_passes_item_8s_rect_to_the_player() {
        let reel = MockMoviePlayer::playing((32, 32), 3);
        let (mut movie, _) = looping_on(&reel);
        assert_eq!(drawn(&movie), (vec![], false));
        movie.select(ship(128));
        assert_eq!(drawn(&movie), (vec![rect()], true));
        assert_eq!(reel.drawn(), [rect()]);
    }

    #[test]
    fn movies_given_late_start_the_selected_ships_loop() {
        let reel = MockMoviePlayer::playing((32, 32), 3);
        let main = MockMoviePlayer::playing((640, 480), 5);
        let mut movie = ShipMovie::new(files());
        movie.select(ship(128));
        assert_eq!(movie.playing(), None, "no movies yet");
        movie.set_movies(movies(&main, &reel, true));
        assert_eq!(reel.calls(), [start("Ship.mov")]);
        movie.set_movies(movies(&main, &reel, true));
        assert_eq!(
            reel.calls(),
            [start("Ship.mov"), Call::Stop, start("Ship.mov")],
            "given again, it starts afresh"
        );
    }

    #[test]
    fn its_debug_names_what_it_holds() {
        let reel = MockMoviePlayer::playing((32, 32), 3);
        let (mut movie, _) = looping_on(&reel);
        movie.select(ship(128));
        let debug = format!("{movie:?}");
        for part in [
            "ShipMovie",
            "Ship.mov",
            "Other.mov",
            "looping: true",
            "owed",
        ] {
            assert!(debug.contains(part), "{part} in {debug}");
        }
    }
}
