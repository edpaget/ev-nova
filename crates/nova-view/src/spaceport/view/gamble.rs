//! The spaceport's side of the bar's Gamble: the hand-offs between the
//! bar's Gamble and race and whoever holds the session, and the texts
//! they show over the spaceport (`_DoBarDialog` @0x49136-0x491ad,
//! `_DoRaceDialog` @0x489ca).
//!
//! - Gamble asks what it opens ([`SpaceportView::take_gamble_request`]);
//!   the answer ([`SpaceportView::open_gamble`]) opens the race over the
//!   bar, or shows `STR#` 2002 #361 ([`NO_CASH_TO_BET`]) in "Desc Dialog"
//!   over the spaceport. As that text closes, the status bar is redrawn
//!   (`_RefreshSAT` @0x49191, [`SpaceportView::take_status_refresh`]) and
//!   the bar with it, letting go of a clicked Gamble drawn pressed. Its
//!   after-movie, the last `dësc` loaded's, is not built: it waits on
//!   rdm:phase/movies/phase-4-desc-movies-outside-missions.
//! - A racer's bet is taken through the spaceport
//!   ([`SpaceportView::take_race_bet`]); its result
//!   ([`SpaceportView::show_race`]) queues the result text over everything
//!   with its movie before it. The race shows the winner beneath the text
//!   only once that movie is over, as the original's invalidated race
//!   redraws only once the text dialog's loop runs. Once the text has
//!   closed, the winnings are due ([`SpaceportView::take_winnings_due`]),
//!   as the original credits them only then (@0x48e03-0x48e30); once
//!   credited, the most an asked bet stakes is the cash credited's
//!   ([`SpaceportView::race_winnings_collected`]).
//! - Help's text ([`SpaceportView::take_race_help_request`],
//!   [`SpaceportView::show_race_help`]) shows over the race with no
//!   picture and no movie before it (`_ShowTextDialog(0, 0, 0)` @0x48bbf),
//!   but the after-movie the load left.
//!
//! What waits for the texts to close is settled after each input and
//! tick ([`SpaceportView::settle_gamble`]), so it is settled before the
//! next frame.

use nova_sim::DescMovie;
use nova_sim::gamble::{Bet, GambleOpening, NO_CASH_TO_BET};

use super::{Open, SpaceportView};
use crate::flight::RaceShown;
use crate::spaceport::bar::BarScreen;
use crate::ui::dialog::DialogTemplate;
use crate::ui::mission_dialogs::DescShown;

/// What waits for the texts over the spaceport to close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AfterTexts {
    /// #361 is up: its close redraws the status bar and the bar.
    NoCash,
    /// The race's result text is up: its close makes the winnings due.
    Winnings,
}

/// The race's templates and what Gamble's texts wait on.
#[derive(Clone, Debug)]
pub(super) struct Gambling {
    /// The race dialog's template, or why there is none.
    race: Result<DialogTemplate, String>,
    /// The quantity dialog's, the asked bet's.
    quantity: Option<DialogTemplate>,
    /// What waits for the texts to close.
    after: Option<AfterTexts>,
    /// Whether the race's result text has closed since this was last
    /// taken.
    winnings_due: bool,
}

impl Default for Gambling {
    fn default() -> Self {
        Self {
            race: Err(format!(
                "the race dialog (DLOG {}) is not given",
                crate::spaceport::race::RACE_DIALOG
            )),
            quantity: None,
            after: None,
            winnings_due: false,
        }
    }
}

impl Gambling {
    /// `bar` with the race's templates.
    pub(super) fn give_race(&self, bar: BarScreen) -> BarScreen {
        bar.with_race(self.race.clone(), self.quantity.clone())
    }
}

impl SpaceportView {
    /// The spaceport with its bar's race laid out by `template`, the race
    /// dialog (or saying why there is none), its asked bet in the quantity
    /// dialog laid out by `quantity` (or its fallback): for each bar opened
    /// after it.
    #[must_use]
    pub fn with_race(
        self,
        template: Result<DialogTemplate, String>,
        quantity: Option<DialogTemplate>,
    ) -> Self {
        Self {
            gambling: Gambling {
                race: template,
                quantity,
                ..self.gambling
            },
            ..self
        }
    }

    /// Whether the bar's Gamble has asked what it opens since this was
    /// last asked ([`BarScreen::take_gamble_request`]): whoever flies the
    /// ship answers with [`SpaceportView::open_gamble`].
    pub fn take_gamble_request(&mut self) -> bool {
        self.open_bar_mut()
            .is_some_and(BarScreen::take_gamble_request)
    }

    /// Opens what Gamble opens: the race over the bar, or #361 over the
    /// spaceport (see the module docs).
    pub fn open_gamble(&mut self, opening: GambleOpening) {
        match opening {
            GambleOpening::Race { asked_max } => {
                if let Some(bar) = self.open_bar_mut() {
                    bar.open_race(asked_max);
                }
            }
            GambleOpening::NoCash => {
                // No after-movie yet: `rdm` task `gamble-361-after-movie`
                // passes the last `dësc` loaded's.
                self.show_text(NO_CASH_TO_BET, None);
                self.gambling.after = Some(AfterTexts::NoCash);
                self.settle_gamble();
            }
        }
    }

    /// The racer and the bet the bar's race asked for since this was last
    /// taken, once.
    pub fn take_race_bet(&mut self) -> Option<(u8, Bet)> {
        self.open_bar_mut()?.take_race_bet()
    }

    /// Shows the race asked for: its result, or, with none run, lets its
    /// racer go (see the module docs).
    pub fn show_race(&mut self, shown: Option<RaceShown>) {
        let Some(bar) = self.open_bar_mut() else {
            return;
        };
        let Some(shown) = shown else {
            bar.race_no_race();
            return;
        };
        bar.show_race_result(shown.winner, shown.asked_max);
        if let Some(dialogs) = &mut self.dialogs {
            dialogs.show_descs(vec![shown.desc]);
        }
        self.gambling.after = Some(AfterTexts::Winnings);
        self.settle_gamble();
    }

    /// Whether the race's Help has asked for its text since this was last
    /// asked: whoever flies the ship answers with
    /// [`SpaceportView::show_race_help`].
    pub fn take_race_help_request(&mut self) -> bool {
        self.open_bar_mut()
            .is_some_and(BarScreen::take_race_help_request)
    }

    /// Shows the race's Help text, `text`, with the after-movie `after`
    /// the load left, if any.
    pub fn show_race_help(&mut self, text: &str, after: Option<DescMovie>) {
        if let Some(dialogs) = &mut self.dialogs {
            dialogs.show_descs(vec![DescShown {
                text: text.to_owned(),
                graphic: None,
                movie: after,
            }]);
        }
    }

    /// Whether the race's result text has closed since this was last
    /// asked, once: whoever flies the ship credits the winnings due
    /// (`nova_view::flight::FlightView::collect_winnings`).
    pub fn take_winnings_due(&mut self) -> bool {
        std::mem::take(&mut self.gambling.winnings_due)
    }

    /// The race's winnings were credited, and an asked bet can now stake
    /// at most `asked_max`, the cash credited's
    /// ([`BarScreen::race_winnings_collected`]).
    pub fn race_winnings_collected(&mut self, asked_max: i64) {
        if let Some(bar) = self.open_bar_mut() {
            bar.race_winnings_collected(asked_max);
        }
    }

    /// Settles what waits on the texts over the spaceport (see the module
    /// docs): after each input and tick.
    pub(super) fn settle_gamble(&mut self) {
        let playing = self.playing();
        if let Some(bar) = self.open_bar_mut()
            && !playing
        {
            bar.reveal_race();
        }
        if self.texts_queued() {
            return;
        }
        match self.gambling.after.take() {
            Some(AfterTexts::NoCash) => {
                self.status_refresh = true;
                if let Some(bar) = self.open_bar_mut() {
                    bar.release_gamble();
                }
            }
            Some(AfterTexts::Winnings) => self.gambling.winnings_due = true,
            None => {}
        }
    }

    /// The bar open, if it is, to change.
    fn open_bar_mut(&mut self) -> Option<&mut BarScreen> {
        match &mut self.open {
            Some(Open::Bar(bar)) => Some(bar),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::time::Duration;

    use super::*;
    use crate::geometry::{Bounds, Point};
    use crate::input::{Input, Key, MouseButton};
    use crate::movie::fixture::{Call, MockMoviePlayer};
    use crate::movie::{MoviePlayer, Movies};
    use crate::screen::Screen;
    use crate::spaceport::bar::{GAMBLE_ITEM, GAMBLE_KEYS};
    use crate::spaceport::race::{FIRST_RACER_ITEM, HELP_ITEM, RaceScreen, RacerLook};
    use crate::spaceport::view::tests::{click_item, key, venues};
    use crate::ui::dialog::{Dialog, ItemSpec, ItemTemplate, Placement};

    /// Stock `DITL` 1023's frame and items, enough to lay the race out.
    fn race_template() -> DialogTemplate {
        let item = |x, y, w, h| ItemTemplate {
            bounds: Bounds::at(Point::new(x, y), w, h),
            enabled: false,
            kind: ItemSpec::User,
        };
        DialogTemplate {
            bounds: Bounds::at(Point::new(0.0, 0.0), 470.0, 230.0),
            placement: Placement::Center,
            items: vec![
                item(243.0, 196.0, 99.0, 25.0),
                item(129.0, 196.0, 99.0, 25.0),
                item(13.0, 88.0, 100.0, 100.0),
                item(128.0, 88.0, 100.0, 100.0),
                item(243.0, 88.0, 100.0, 100.0),
                item(357.0, 88.0, 100.0, 100.0),
            ],
        }
    }

    fn click(view: &mut SpaceportView, at: Point) {
        for pressed in [true, false] {
            view.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    /// Every venue, its race given, in the bar.
    fn in_the_bar() -> SpaceportView {
        in_the_bar_of(venues())
    }

    fn in_the_bar_of(view: SpaceportView) -> SpaceportView {
        let mut view = view.with_race(Ok(race_template()), None);
        let _ = view.take_offer_request();
        click_item(&mut view, 11);
        assert!(view.open_bar().is_some(), "in the bar");
        view
    }

    fn bar_item(view: &SpaceportView, item: usize) -> Point {
        view.open_bar()
            .and_then(BarScreen::dialog)
            .and_then(|dialog| dialog.item_bounds(item))
            .expect("a bar item")
            .center()
    }

    fn race_item(view: &SpaceportView, item: usize) -> Point {
        view.open_bar()
            .and_then(BarScreen::race_screen)
            .and_then(RaceScreen::dialog)
            .and_then(|dialog| dialog.item_bounds(item))
            .expect("a race item")
            .center()
    }

    fn gamble_held(view: &SpaceportView) -> bool {
        view.open_bar()
            .and_then(BarScreen::dialog)
            .and_then(Dialog::held_pressed)
            == Some(GAMBLE_ITEM)
    }

    fn shown_text(view: &SpaceportView) -> Option<String> {
        view.mission_text().map(|dialog| dialog.lines().join(" "))
    }

    fn race_open(view: &SpaceportView) -> Option<&RaceScreen> {
        view.open_bar().and_then(BarScreen::race_screen)
    }

    /// A race won by racer 1, its text "Racer 2 wins." with `movie`.
    fn won(movie: Option<DescMovie>) -> RaceShown {
        RaceShown {
            winner: 1,
            asked_max: 4_000,
            desc: DescShown {
                text: "Racer 2 wins.".to_owned(),
                graphic: None,
                movie,
            },
        }
    }

    #[test]
    fn gamble_asks_through_the_spaceport_by_click_g_and_r() {
        let mut view = in_the_bar();
        assert!(!view.take_gamble_request());
        let gamble = bar_item(&view, GAMBLE_ITEM);
        click(&mut view, gamble);
        assert!(view.take_gamble_request());
        assert!(!view.take_gamble_request(), "once");
        for pressed in GAMBLE_KEYS {
            view.input(&key(pressed));
            assert!(view.take_gamble_request(), "{pressed:?}");
        }
    }

    #[test]
    fn with_no_cash_361_shows_over_the_bar_and_its_close_redraws_the_status_bar_and_the_bar() {
        let mut view = in_the_bar();
        let gamble = bar_item(&view, GAMBLE_ITEM);
        click(&mut view, gamble);
        assert!(view.take_gamble_request());
        view.open_gamble(GambleOpening::NoCash);
        assert_eq!(shown_text(&view).as_deref(), Some(NO_CASH_TO_BET));
        assert!(view.open_bar().is_some(), "over the bar");
        assert!(gamble_held(&view), "Gamble drawn pressed beneath it");
        assert!(!view.take_status_refresh(), "not while it is up");
        view.input(&key(Key::Enter));
        assert!(view.mission_text().is_none(), "closed");
        assert!(view.take_status_refresh(), "`_RefreshSAT` @0x49191");
        assert!(!gamble_held(&view), "the bar redrew");
        assert!(race_open(&view).is_none(), "no race");
        assert!(view.open_bar().is_some(), "back in the bar");
    }

    #[test]
    fn with_cash_the_race_opens_over_the_bar_and_a_bet_runs_through_the_spaceport() {
        let mut view = in_the_bar();
        view.open_gamble(GambleOpening::Race { asked_max: 5_000 });
        assert!(race_open(&view).is_some(), "the race over the bar");
        assert!(view.mission_text().is_none());
        let racer = race_item(&view, FIRST_RACER_ITEM + 1);
        click(&mut view, racer);
        assert_eq!(view.take_race_bet(), Some((1, Bet::House)));
        view.show_race(Some(won(None)));
        assert_eq!(shown_text(&view).as_deref(), Some("Racer 2 wins."));
        assert_eq!(
            race_open(&view).map(|race| race.racer_look(1)),
            Some(RacerLook::Winner),
            "with no movie the result shows beneath the text at once"
        );
        assert!(!view.take_winnings_due(), "not while the text is up");
        view.input(&key(Key::Enter));
        assert!(view.take_winnings_due(), "once the text closes");
        assert!(!view.take_winnings_due(), "once");
        view.input(&key(Key::Escape));
        assert!(race_open(&view).is_none(), "back in the bar");
        assert!(view.open_bar().is_some());
    }

    #[test]
    fn the_race_flushes_and_its_asked_bets_quantity_doesnt() {
        let mut view = in_the_bar();
        view.open_gamble(GambleOpening::Race { asked_max: 5_000 });
        // `_DoRaceDialog` @0x48ad0; `_DoQuantityDialog` has no flush.
        assert_eq!(view.flushing_layers(), 2);
        view.input(&key(Key::Alt));
        let racer = race_item(&view, FIRST_RACER_ITEM);
        click(&mut view, racer);
        assert!(
            race_open(&view).is_some_and(|race| race.quantity().is_some()),
            "asking"
        );
        assert_eq!(view.flushing_layers(), 2);
    }

    #[test]
    fn the_result_shows_beneath_its_text_only_once_its_movie_is_over() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let movies = Movies::new(Rc::clone(&player) as Rc<dyn MoviePlayer>, true);
        let mut view = in_the_bar_of(venues().with_movies(movies));
        view.open_gamble(GambleOpening::Race { asked_max: 5_000 });
        let racer = race_item(&view, FIRST_RACER_ITEM);
        click(&mut view, racer);
        let _ = view.take_race_bet();
        view.show_race(Some(won(DescMovie::from_desc("Race 2.mov", 0))));
        assert!(
            player
                .calls()
                .contains(&Call::Start("Race 2.mov".to_owned()))
        );
        assert_eq!(
            race_open(&view).map(|race| race.racer_look(0)),
            Some(RacerLook::Pressed),
            "the racer chosen pressed under the movie"
        );
        for _ in 0..4 {
            view.tick(Duration::from_millis(100));
        }
        assert_eq!(shown_text(&view).as_deref(), Some("Racer 2 wins."));
        assert_eq!(
            race_open(&view).map(|race| race.racer_look(0)),
            Some(RacerLook::Loser),
            "shown once the movie is over"
        );
    }

    #[test]
    fn the_winnings_collected_raise_the_races_asked_most() {
        let mut view = in_the_bar();
        view.open_gamble(GambleOpening::Race { asked_max: 1_000 });
        let racer = race_item(&view, FIRST_RACER_ITEM + 1);
        click(&mut view, racer);
        let _ = view.take_race_bet();
        view.show_race(Some(RaceShown {
            asked_max: 0,
            ..won(None)
        }));
        view.input(&key(Key::Enter));
        assert!(view.take_winnings_due());
        view.race_winnings_collected(4_000);
        click(&mut view, racer);
        view.input(&key(Key::Alt));
        click(&mut view, racer);
        let field = race_open(&view)
            .and_then(RaceScreen::quantity)
            .and_then(|ask| ask.dialog())
            .map(|dialog| dialog.field().text().to_owned());
        assert_eq!(field.as_deref(), Some("4000"));
    }

    #[test]
    fn a_race_run_none_of_lets_its_racer_go() {
        let mut view = in_the_bar();
        view.open_gamble(GambleOpening::Race { asked_max: 0 });
        let racer = race_item(&view, FIRST_RACER_ITEM + 2);
        click(&mut view, racer);
        assert_eq!(view.take_race_bet(), Some((2, Bet::House)));
        view.show_race(None);
        assert_eq!(
            race_open(&view).map(|race| race.racer_look(2)),
            Some(RacerLook::Normal)
        );
        assert!(view.mission_text().is_none());
        assert!(!view.take_winnings_due());
    }

    #[test]
    fn help_asks_through_the_spaceport_and_shows_over_the_race() {
        let mut view = in_the_bar();
        view.open_gamble(GambleOpening::Race { asked_max: 5_000 });
        let help = race_item(&view, HELP_ITEM);
        click(&mut view, help);
        assert!(view.take_race_help_request());
        assert!(!view.take_race_help_request(), "once");
        view.show_race_help("How to bet.", None);
        assert_eq!(shown_text(&view).as_deref(), Some("How to bet."));
        view.input(&key(Key::Enter));
        assert!(race_open(&view).is_some(), "the race stays");
        assert!(!view.take_winnings_due(), "no race ran");
    }

    #[test]
    fn the_races_asked_bet_is_laid_out_by_the_quantity_dialog_given() {
        let item = |x, y, w, h, kind| ItemTemplate {
            bounds: Bounds::at(Point::new(x, y), w, h),
            enabled: true,
            kind,
        };
        let quantity = DialogTemplate {
            bounds: Bounds::at(Point::new(10.0, 20.0), 172.0, 72.0),
            placement: Placement::Fixed,
            items: vec![
                item(92.0, 42.0, 70.0, 20.0, ItemSpec::Button("OK".into())),
                item(6.0, 8.0, 102.0, 16.0, ItemSpec::StaticText("^0".into())),
                item(112.0, 8.0, 51.0, 16.0, ItemSpec::EditText("1".into())),
                item(10.0, 42.0, 70.0, 20.0, ItemSpec::Button("Cancel".into())),
            ],
        };
        let mut view = venues().with_race(Ok(race_template()), Some(quantity));
        let _ = view.take_offer_request();
        click_item(&mut view, 11);
        view.open_gamble(GambleOpening::Race { asked_max: 5_000 });
        view.input(&key(Key::Alt));
        let racer = race_item(&view, FIRST_RACER_ITEM);
        click(&mut view, racer);
        let at = race_open(&view)
            .and_then(RaceScreen::quantity)
            .and_then(|ask| ask.dialog())
            .map(|dialog| dialog.dialog().bounds().min);
        assert_eq!(
            at,
            Some(Point::new(10.0, 20.0)),
            "its template, not the fallback"
        );
    }

    #[test]
    fn the_race_holds_a_leave() {
        let mut view = in_the_bar();
        assert!(!view.holds_leave());
        view.open_gamble(GambleOpening::Race { asked_max: 5_000 });
        assert!(
            view.holds_leave(),
            "`_RaceFilter` reads only Return, Enter and Escape"
        );
    }
}
