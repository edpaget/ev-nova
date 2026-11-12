//! The bar's Gamble as it is shown: what Gamble opens
//! ([`GambleOpening`]), a racer's bet ([`Bet`]), a race run
//! ([`RaceRun`]), the result text ([`result_text`]) and the dialog's
//! strings, from `nova_sim_port::gamble`, which re-exports them and keeps
//! the opening, the winner's draw and the winnings.

/// `STR#` 2002 #361: Gamble's text with no cash to bet.
pub const NO_CASH_TO_BET: &str = "Sorry, you don't have enough credits to bet today.";

/// `STR#` 2002 #372: the asked bet's prompt (`_DoQuantityDialog` mode 2,
/// @0x56e7c-0x56e9f).
pub const AMOUNT_TO_BET: &str = "Amount to bet:";

/// `STR#` 150 #59: the asked bet's OK button.
pub const BET: &str = "Bet";

/// The first race result's `dësc`, 32760 (0x7ff8, @0x48d05): racer `w`
/// winning shows `dësc` 32760 + `w`.
pub const FIRST_RACE_DESC: i16 = 32_760;

/// The race's Help `dësc`, 32764 (0x7ffc, @0x48b89).
pub const RACE_HELP_DESC: i16 = 32_764;

/// How many racers run: `Rand(4)` (@0x48c97), `DITL` 1023's items 3-6.
pub const RACERS: u8 = 4;

/// The most the house bet stakes (@0x48c26-0x48c4f).
pub const HOUSE_BET: i64 = 1000;

/// The most the asked bet can stake (@0x48bf9-0x48c1f).
pub const MAX_ASKED_BET: i64 = 10_000;

/// What Gamble opens (`nova_sim_port::gamble::opening`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GambleOpening {
    /// The race, the most an asked bet can stake given.
    Race {
        /// [`asked_max`] of the cash now.
        asked_max: i64,
    },
    /// No race: [`NO_CASH_TO_BET`] shows.
    NoCash,
}

/// The house bet with `cash`: `min(cash, 1000)` ([`HOUSE_BET`]).
#[must_use]
pub fn house_bet(cash: i64) -> i64 {
    cash.min(HOUSE_BET)
}

/// The most an asked bet can stake with `cash`: `min(cash, 10000)`
/// ([`MAX_ASKED_BET`]), where the quantity dialog opens.
#[must_use]
pub fn asked_max(cash: i64) -> i64 {
    cash.min(MAX_ASKED_BET)
}

/// What a racer's click bets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bet {
    /// The house bet ([`house_bet`]): no Option held.
    House,
    /// The amount the quantity dialog gave, Option held: 0 for its
    /// Cancel.
    Asked(i64),
}

impl Bet {
    /// What it stakes with `cash`: the house bet, or the amount asked,
    /// held to [`asked_max`] as the quantity dialog holds it. A stake of 0
    /// or less runs no race.
    #[must_use]
    pub fn amount(self, cash: i64) -> i64 {
        match self {
            Self::House => house_bet(cash),
            Self::Asked(asked) => asked.min(asked_max(cash)),
        }
    }
}

/// The race's result text: `desc`, the winner's `dësc` loaded, with
/// `line`, the winnings line on a win, after it.
#[must_use]
pub fn result_text(desc: &str, line: Option<&str>) -> String {
    let mut text = desc.to_owned();
    text.push_str(line.unwrap_or_default());
    text
}

/// What a race run did ([`Bet`] placed on a racer), as the session
/// answers it: the view loads the winner's `dësc` and appends the
/// winnings line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RaceRun {
    /// The racer bet on, 0 to 3.
    pub chosen: u8,
    /// The racer that won, 0 to 3.
    pub winner: u8,
    /// What was staked, already taken from the cash.
    pub bet: i64,
    /// The winnings line (`nova_sim_port::gamble::winnings_line`) on a win, the
    /// winnings due once the text closes; `None` on a loss.
    pub winnings_line: Option<String>,
    /// The most the next asked bet can stake, with the cash after the
    /// bet ([`asked_max`]).
    pub asked_max: i64,
}

impl RaceRun {
    /// Whether the racer bet on won.
    #[must_use]
    pub fn won(&self) -> bool {
        self.winner == self.chosen
    }

    /// The `dësc` the result shows: [`FIRST_RACE_DESC`] + the winner.
    #[must_use]
    pub fn desc(&self) -> i16 {
        FIRST_RACE_DESC + i16::from(self.winner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_house_bet_is_the_cash_up_to_1000() {
        assert_eq!(house_bet(999), 999);
        assert_eq!(house_bet(1000), 1000);
        assert_eq!(house_bet(1001), 1000);
        assert_eq!(HOUSE_BET, 1000);
    }

    #[test]
    fn the_asked_bet_is_held_to_the_cash_up_to_10000() {
        assert_eq!(asked_max(9_999), 9_999);
        assert_eq!(asked_max(10_000), 10_000);
        assert_eq!(asked_max(10_001), 10_000);
        assert_eq!(MAX_ASKED_BET, 10_000);
    }

    #[test]
    fn a_bet_stakes_the_house_bet_or_the_amount_asked_held_to_its_max() {
        assert_eq!(Bet::House.amount(5_000), 1000);
        assert_eq!(Bet::Asked(700).amount(5_000), 700);
        assert_eq!(Bet::Asked(9_000).amount(5_000), 5_000);
        assert_eq!(Bet::Asked(0).amount(5_000), 0);
        assert_eq!(Bet::Asked(-3).amount(5_000), -3);
    }

    #[test]
    fn the_result_is_the_race_text_with_the_winnings_line_on_a_win() {
        assert_eq!(result_text("Racer 2 wins.", None), "Racer 2 wins.");
        assert_eq!(
            result_text("Racer 2 wins.", Some("\r\rYour winnings:  8 credits.")),
            "Racer 2 wins.\r\rYour winnings:  8 credits."
        );
    }

    #[test]
    fn a_race_run_won_when_its_winner_is_the_racer_chosen_showing_its_desc() {
        let run = RaceRun {
            chosen: 2,
            winner: 2,
            bet: 10,
            winnings_line: Some(String::new()),
            asked_max: 0,
        };
        assert!(run.won());
        assert_eq!(run.desc(), 32_762);
        let lost = RaceRun { winner: 3, ..run };
        assert!(!lost.won());
        assert_eq!(lost.desc(), 32_763);
        assert_eq!(
            (FIRST_RACE_DESC, RACE_HELP_DESC, RACERS),
            (32_760, 32_764, 4)
        );
    }
}
