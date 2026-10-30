//! The bar's Gamble: the session's side of [`gamble`](crate::gamble).
//!
//! **Opening.** Docked at a stellar with a bar, Gamble opens the race with
//! cash above 0, else shows `STR#` 2002 #361
//! ([`Session::gamble_opening`], `_DoBarDialog` @0x4913c-0x4918c).
//!
//! **A race** ([`Session::race`]) takes the bet from the cash at once and
//! draws the winner on the caller's chance, against the bar visit's last
//! winner, which the bar's opening resets ([`Session::open_bar`],
//! @0x48f61-0x48f6c) and every race sets (@0x48cd5). A win's
//! [`PAYOUT`](crate::gamble::PAYOUT) times the bet is held as winnings
//! due, and credited only by [`Session::collect_winnings`], which the view
//! sends once the result text has closed (@0x48e03-0x48e30), answering
//! the most the next asked bet can stake of the cash credited; so the status
//! bar shows the cash less the bet while the text is up, as the original's
//! `_DrawStatus` after the bet (@0x48c92) and after the text (@0x48e30)
//! show it. Neither change makes a save due: the original saves nothing
//! here.
//!
//! **The result text.** The session answers only the draw
//! ([`RaceRun`]): the winner, the bet and, on a win, the winnings line
//! worded by the rule set's
//! [`RuleKey::PayMillions`](crate::RuleKey::PayMillions). The view loads
//! the winner's `dësc` ([`RaceRun::desc`]) through the same
//! description load every other screen's `dësc` takes
//! (`FlightCommand::LoadDescription`, [`Session::load_description`]), which
//! writes the run's last movie as `_LoadDescription` does (@0x48d05-0x48d17),
//! and appends the line ([`result_text`](crate::gamble::result_text)). So
//! the catalog's `dësc`s stay read on the view's side, as they are for
//! every other screen.

use super::{Port, Session};
use crate::chance::Chance;
use crate::gamble::{self, Bet, GambleOpening, RACERS, RaceRun};
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource};

impl Session {
    /// What the bar's Gamble opens ([`gamble::opening`]) with the cash
    /// now: `None` unless the ship is docked at a stellar with a bar.
    #[must_use]
    pub fn gamble_opening(&self) -> Option<GambleOpening> {
        self.chart.bar_site(self.mode.landed())?;
        Some(gamble::opening(self.pilot.cash))
    }

    /// Runs a race on `chosen`, 0 to 3, staking `bet` ([`Bet::amount`])
    /// and drawing its winner on `chance` (see the module docs). `None`,
    /// and nothing changes, unless docked at a stellar with a bar, for a
    /// racer past the fourth, or for a stake of 0 or less.
    pub fn race(&mut self, chosen: u8, bet: Bet, chance: &mut dyn Chance) -> Option<RaceRun> {
        self.chart.bar_site(self.mode.landed())?;
        let amount = bet.amount(self.pilot.cash);
        if chosen >= RACERS || amount <= 0 {
            return None;
        }
        let millions = self.setup.rules.source_for(RuleKey::PayMillions);
        Some(
            self.port
                .race(&mut self.pilot, chosen, amount, millions, chance),
        )
    }

    /// Credits the race winnings due, once the result text has closed
    /// (see the module docs): the most the next asked bet can stake
    /// ([`gamble::asked_max`]) of the cash credited, as the original asks
    /// each Option click's quantity dialog of the cash then
    /// (@0x48bf9-0x48c1f); or `None`, changing nothing, when none are due.
    pub fn collect_winnings(&mut self) -> Option<i64> {
        self.port
            .collect_winnings(&mut self.pilot)
            .then(|| gamble::asked_max(self.pilot.cash))
    }
}

impl Port {
    /// Forgets the last race winner, as the bar's opening does
    /// (@0x48f61-0x48f6c).
    pub(super) fn reset_race(&mut self) {
        self.last_race_winner = None;
    }

    /// Runs a race on `chosen`, staking `amount` (above 0) of `pilot`'s
    /// cash: the winner drawn on `chance` against the last, kept as the
    /// last, and on a win the winnings held due and their line worded as
    /// `millions` reads [`RuleKey::PayMillions`].
    fn race(
        &mut self,
        pilot: &mut Pilot,
        chosen: u8,
        amount: i64,
        millions: RuleSource,
        chance: &mut dyn Chance,
    ) -> RaceRun {
        pilot.cash -= amount;
        let winner = gamble::draw_winner(self.last_race_winner, chosen, chance);
        self.last_race_winner = Some(winner);
        let winnings_line = (winner == chosen).then(|| {
            let winnings = gamble::winnings(amount);
            self.winnings_due += winnings;
            gamble::winnings_line(winnings, millions)
        });
        RaceRun {
            chosen,
            winner,
            bet: amount,
            winnings_line,
            asked_max: gamble::asked_max(pilot.cash),
        }
    }

    /// Credits the winnings due to `pilot`'s cash: whether any were.
    fn collect_winnings(&mut self, pilot: &mut Pilot) -> bool {
        let due = std::mem::take(&mut self.winnings_due);
        pilot.cash += due;
        due != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::gamble::{MAX_ASKED_BET, winnings_line};
    use crate::landing::StellarFlags;
    use crate::session::mission_map::tests::landed;
    use crate::session::tests::shipbuying;
    use crate::testkit::{FakePilotCatalog, Scripted};

    /// A catalog whose planet 128 has a bar.
    fn port() -> FakePilotCatalog {
        let mut catalog = shipbuying();
        catalog.sites[0].1[0].flags |= StellarFlags::BAR;
        catalog
    }

    /// [`port`]'s session docked at its bar, opened, holding `cash`.
    fn at_bar(cash: i64) -> Session {
        let mut session = landed(&port());
        assert!(session.open_bar(&mut NeverFires), "a bar");
        session.pilot.cash = cash;
        session
    }

    #[test]
    fn gamble_opens_the_race_with_cash_and_the_no_cash_text_without() {
        assert_eq!(
            at_bar(20_000).gamble_opening(),
            Some(GambleOpening::Race {
                asked_max: MAX_ASKED_BET
            })
        );
        assert_eq!(at_bar(0).gamble_opening(), Some(GambleOpening::NoCash));
    }

    #[test]
    fn away_from_a_bar_gamble_opens_nothing_and_no_race_runs() {
        let catalog = port();
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.cash = 5_000;
        assert_eq!(session.gamble_opening(), None);
        let before = session.clone();
        assert_eq!(session.race(0, Bet::House, &mut NeverFires), None);
        assert_eq!(session, before);
    }

    #[test]
    fn a_bet_is_taken_at_once_and_a_win_pays_four_times_it_only_once_collected() {
        let mut session = at_bar(5_000);
        let run = session
            .race(2, Bet::House, &mut Scripted::rolling(&[2]))
            .expect("a race");
        assert_eq!(
            run,
            RaceRun {
                chosen: 2,
                winner: 2,
                bet: 1000,
                winnings_line: Some(winnings_line(4000, RuleSource::Engine)),
                asked_max: 4000,
            }
        );
        assert_eq!(
            session.pilot.cash, 4000,
            "the bet taken while the text is up"
        );
        assert_eq!(
            session.collect_winnings(),
            Some(8000),
            "the asked most of the cash credited"
        );
        assert_eq!(session.pilot.cash, 8000, "4 x 1000 credited");
        assert_eq!(session.collect_winnings(), None, "only once");
        assert_eq!(session.pilot.cash, 8000);
    }

    #[test]
    fn a_loss_takes_the_bet_and_leaves_nothing_to_collect() {
        let mut session = at_bar(500);
        let run = session
            .race(0, Bet::Asked(300), &mut Scripted::rolling(&[3]))
            .expect("a race");
        assert_eq!((run.winner, run.bet, run.winnings_line), (3, 300, None));
        assert_eq!(session.pilot.cash, 200);
        assert_eq!(session.collect_winnings(), None);
        assert_eq!(session.pilot.cash, 200);
    }

    #[test]
    fn the_collect_answers_the_asked_most_of_the_cash_after_the_credit() {
        // All 1,000 staked: the race leaves 0, the credit 4,000.
        let mut session = at_bar(1_000);
        let run = session
            .race(0, Bet::House, &mut Scripted::rolling(&[0]))
            .expect("a race");
        assert_eq!(run.asked_max, 0, "the cash less the bet");
        assert_eq!(session.collect_winnings(), Some(4_000));
        // Past 10,000 the asked most holds there.
        let mut session = at_bar(20_000);
        session
            .race(0, Bet::House, &mut Scripted::rolling(&[0]))
            .expect("a race");
        assert_eq!(session.collect_winnings(), Some(MAX_ASKED_BET));
    }

    #[test]
    fn the_winnings_line_follows_the_rule_sets_pay_millions() {
        let mut session = at_bar(500_000);
        session.setup.rules = session
            .setup
            .rules
            .with_override(RuleKey::PayMillions, RuleSource::Bible);
        let run = session
            .race(1, Bet::Asked(MAX_ASKED_BET), &mut Scripted::rolling(&[1]))
            .expect("a race");
        assert_eq!(
            run.winnings_line,
            Some(winnings_line(40_000, RuleSource::Bible))
        );
    }

    #[test]
    fn a_stake_of_0_or_less_or_a_fifth_racer_runs_no_race() {
        let mut session = at_bar(5_000);
        let before = session.clone();
        for (racer, bet) in [
            (0, Bet::Asked(0)),
            (1, Bet::Asked(-5)),
            (RACERS, Bet::House),
        ] {
            assert_eq!(
                session.race(racer, bet, &mut Scripted::rolling(&[0])),
                None,
                "{racer} {bet:?}"
            );
        }
        assert_eq!(session, before, "nothing changes");
    }

    #[test]
    fn the_last_winner_carries_across_races_until_the_bar_opens_again() {
        let mut session = at_bar(50_000);
        let first = session
            .race(0, Bet::House, &mut Scripted::rolling(&[3]))
            .expect("a race");
        assert_eq!(first.winner, 3);
        // A first roll of the last winner, 3, is drawn again past it and
        // the racer chosen, 0.
        let second = session
            .race(0, Bet::House, &mut Scripted::rolling(&[3, 0, 1]))
            .expect("a race");
        assert_eq!(second.winner, 1);
        assert!(session.open_bar(&mut NeverFires));
        // Were 1 still the last, it would be drawn again, giving 2.
        let third = session
            .race(0, Bet::House, &mut Scripted::rolling(&[1, 2]))
            .expect("a race");
        assert_eq!(third.winner, 1, "the bar's opening forgot the last");
    }
}
