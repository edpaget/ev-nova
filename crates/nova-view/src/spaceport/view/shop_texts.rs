//! The spaceport's `STR#` texts, and the movie each plays after it.
//!
//! The shops' texts and the escorts' report are `_ShowTextDialog(0, 0, 0)`
//! calls that load no `dësc` of their own: the outfitter's #206, nothing
//! listed (@0x5bbf2), and its sale alerts, #207 and the #208-#212
//! sentences (@0x5c7a0, @0x5ca3f, @0x5ce5a); the shipyard's #223, nothing
//! listed, and #224, nothing for hire (@0x5e8da); and the report of the
//! escorts sold and upgraded (`_DoEscortLand` @0x407e8). The after-check
//! (@0x5b40b-0x5b44f) runs whatever the third argument says, so each plays
//! the after-movie (`Flags` 0x0001) of the last `dësc` loaded, which
//! whoever flies the ship reads as it answers
//! ([`SessionFlight::loaded_after_movie`](crate::flight::SessionFlight::loaded_after_movie),
//! under the `desc_movie_globals` rule) and hands in as data: `after` on
//! [`SpaceportView::show_text`], [`SpaceportView::set_opened_outfitter`],
//! [`SpaceportView::set_opened_shipyard`], [`SpaceportView::open_hire`]
//! and [`SpaceportView::alert_outfit`]. A movie handed in without its
//! after flag never plays: only an after-movie is read there.
//!
//! Each text goes through the spaceport's mission dialogs, so the movie
//! plays once its Done closes it, and what waits on the text waits through
//! the movie: a leave it holds ([`SpaceportView::holds_leave`]). The sale
//! alert is the outfitter's own dialog ([`OutfitterScreen::alert`]): its
//! movie is queued on the mission dialogs, over the outfitter, as the
//! alert closes, and an outfitter that the order left with nothing listed
//! closes once the movie has ended, as the original leaves its loop
//! (@0x5da33) only after `_ShowTextDialog` returns.

use nova_sim::{DescMovie, MissionShown};

use super::{Open, SpaceportView};
#[cfg(doc)]
use crate::spaceport::outfitter::OutfitterScreen;

/// `after`, if it is an after-movie: the only kind a `STR#` text plays.
pub(super) fn after_movie(after: Option<DescMovie>) -> Option<DescMovie> {
    after.filter(|movie| movie.after)
}

impl SpaceportView {
    /// Queues the sale alert's after-movie on the mission dialogs once the
    /// alert has closed.
    pub(super) fn play_alert_after(&mut self) {
        let alert_open = matches!(
            &self.open,
            Some(Open::Outfitter(screen)) if screen.alert_dialog().is_some()
        );
        if alert_open {
            return;
        }
        if let Some(movie) = self.alert_after.take()
            && let Some(dialogs) = &mut self.dialogs
        {
            dialogs.show_texts([MissionShown::Movie(movie)]);
        }
    }

    /// Closes the outfitter its order left with nothing listed, once its
    /// alert's after-movie has ended.
    pub(super) fn close_alerted_outfitter(&mut self) {
        if !self.texts_queued()
            && matches!(&self.open, Some(Open::Outfitter(screen)) if screen.closed())
        {
            self.close_open();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use nova_sim::{DescMovie, Reselection};

    use super::super::tests::{
        alert, click_item, done, drinking_with_info, hirelings, key, movies_of, ms, serve_openings,
        shopping, shown_text, started, tanks, venues, yard,
    };
    use super::*;
    use crate::input::Key;
    use crate::movie::fixture::MockMoviePlayer;
    use crate::movie::{MoviePlayer, Movies};
    use crate::screen::Screen;
    use crate::spaceport::outfitter::NO_ITEMS;
    use crate::spaceport::shipyard::NO_SHIPS;
    use nova_sim::hire::NONE_FOR_HIRE;
    use nova_sim::{Outfitter, Shipyard};

    fn after(file: &str) -> Option<DescMovie> {
        DescMovie::from_desc(file, 0x0001)
    }

    /// The view `make` builds, playing movies on a player whose movies
    /// last two ticks of 100 ms, or with movies off.
    fn playing(make: fn() -> SpaceportView, on: bool) -> (SpaceportView, Rc<MockMoviePlayer>) {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let movies = if on {
            movies_of(&player)
        } else {
            Movies::new(Rc::clone(&player) as Rc<dyn MoviePlayer>, false)
        };
        (make().with_movies(movies), player)
    }

    /// Plays `view`'s movie out: two ticks.
    fn play_out(view: &mut SpaceportView) {
        view.tick(ms(100));
        view.tick(ms(100));
    }

    /// Opens an outfitter listing nothing over `view`: #206 shows,
    /// playing `movie` after.
    fn empty_outfitter(view: &mut SpaceportView, movie: Option<DescMovie>) {
        click_item(view, 8);
        serve_openings(view);
        view.set_opened_outfitter(
            Outfitter {
                rows: Vec::new(),
                ..tanks(0)
            },
            movie,
        );
    }

    #[test]
    fn an_empty_outfitters_text_plays_the_after_movie_given_once_done_closes_it() {
        let (mut view, player) = playing(shopping, true);
        empty_outfitter(&mut view, after("After.mov"));
        assert_eq!(shown_text(&view).as_deref(), Some(NO_ITEMS));
        assert!(started(&player).is_empty(), "not before the text closes");
        done(&mut view);
        assert_eq!(started(&player), ["After.mov"]);
        assert!(view.playing());
        assert!(view.holds_leave(), "a leave waits through it");
        play_out(&mut view);
        assert!(!view.playing());
        assert!(!view.holds_leave());
    }

    #[test]
    fn a_text_given_no_movie_a_movie_not_flagged_after_or_movies_off_plays_nothing() {
        for (movie, on) in [
            (None, true),
            (DescMovie::from_desc("Before.mov", 0), true),
            (after("After.mov"), false),
        ] {
            let (mut view, player) = playing(shopping, on);
            empty_outfitter(&mut view, movie.clone());
            done(&mut view);
            assert!(started(&player).is_empty(), "{movie:?} {on}");
            assert!(!view.holds_leave(), "{movie:?} {on}");
        }
    }

    #[test]
    fn an_empty_shipyards_text_plays_the_after_movie_once_it_closes() {
        let (mut view, player) = playing(shopping, true);
        click_item(&mut view, 9);
        serve_openings(&mut view);
        view.set_opened_shipyard(
            Shipyard {
                rows: Vec::new(),
                ..yard(17_500)
            },
            after("After.mov"),
        );
        assert_eq!(shown_text(&view).as_deref(), Some(NO_SHIPS));
        assert!(started(&player).is_empty());
        done(&mut view);
        assert_eq!(started(&player), ["After.mov"]);
    }

    #[test]
    fn nothing_for_hires_text_plays_the_after_movie_once_it_closes() {
        let (mut view, player) = playing(drinking_with_info, true);
        click_item(&mut view, 11);
        view.input(&key(Key::Char('h')));
        assert!(view.take_hire_request());
        view.open_hire(hirelings(0, true), after("After.mov"));
        assert_eq!(shown_text(&view).as_deref(), Some(NONE_FOR_HIRE));
        assert!(started(&player).is_empty());
        done(&mut view);
        assert_eq!(started(&player), ["After.mov"]);
        assert!(view.open_bar().is_some(), "over the bar");
    }

    #[test]
    fn the_escorts_report_holds_the_leave_through_its_after_movie() {
        let (mut view, player) = playing(venues, true);
        view.show_text("Sold.", after("After.mov"));
        assert!(view.holds_leave());
        done(&mut view);
        assert_eq!(started(&player), ["After.mov"]);
        assert!(view.holds_leave(), "through the movie");
        play_out(&mut view);
        assert!(!view.holds_leave());
    }

    /// [`shopping`] in its outfitter, listing tanks, with a sale alert
    /// open over it playing `movie` after.
    fn alerted(on: bool, movie: Option<DescMovie>) -> (SpaceportView, Rc<MockMoviePlayer>) {
        let (mut view, player) = playing(shopping, on);
        click_item(&mut view, 8);
        serve_openings(&mut view);
        view.set_opened_outfitter(tanks(1), None);
        view.alert_outfit(&alert(), movie);
        assert!(
            view.open_outfitter()
                .is_some_and(|open| open.alert_dialog().is_some())
        );
        (view, player)
    }

    #[test]
    fn a_sale_alert_plays_the_after_movie_over_the_outfitter_once_it_closes() {
        let (mut view, player) = alerted(true, after("After.mov"));
        assert!(started(&player).is_empty(), "not under the alert");
        view.input(&key(Key::Escape));
        assert_eq!(started(&player), ["After.mov"]);
        assert!(view.playing());
        assert!(view.open_outfitter().is_some(), "over the outfitter");
        play_out(&mut view);
        assert!(!view.playing());
        assert!(view.open_outfitter().is_some(), "still open");
        assert!(!view.take_outfitter_closed());
        let (mut view, player) = alerted(true, None);
        view.input(&key(Key::Escape));
        assert!(started(&player).is_empty(), "none given");
        let (mut view, player) = alerted(false, after("After.mov"));
        view.input(&key(Key::Escape));
        assert!(started(&player).is_empty(), "movies off");
        assert!(!view.playing());
    }

    #[test]
    fn an_outfitter_left_with_nothing_listed_closes_once_its_alerts_movie_has_ended() {
        let (mut view, player) = alerted(true, after("After.mov"));
        view.outfit_ordered(Reselection::Leave);
        view.input(&key(Key::Escape));
        assert_eq!(started(&player), ["After.mov"]);
        assert!(
            view.open_outfitter().is_some(),
            "the movie first (@0x5da33)"
        );
        assert!(!view.take_outfitter_closed());
        play_out(&mut view);
        assert!(view.open_outfitter().is_none(), "closed as the movie ends");
        assert!(view.take_outfitter_closed());
        let (mut view, _) = alerted(true, None);
        view.outfit_ordered(Reselection::Leave);
        view.input(&key(Key::Escape));
        assert!(view.open_outfitter().is_none(), "with no movie, at once");
        assert!(view.take_outfitter_closed());
    }

    #[test]
    fn an_alert_that_cannot_open_keeps_no_movie() {
        let (mut view, player) = playing(shopping, true);
        view.alert_outfit(&alert(), after("After.mov"));
        click_item(&mut view, 8);
        serve_openings(&mut view);
        view.set_opened_outfitter(tanks(1), None);
        view.input(&key(Key::Escape));
        assert!(started(&player).is_empty(), "no outfitter, no alert");
    }
}
