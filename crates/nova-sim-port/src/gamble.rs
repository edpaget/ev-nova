//! The bar's Gamble: the Galactic Racing Network race, as `_DoBarDialog`'s
//! Gamble branch (@0x49136-0x491ad) and `_DoRaceDialog` (@0x489ca-0x48ef4)
//! in the `EV Nova` executable run it.
//!
//! - **The opening** ([`opening`]): with cash above 0 Gamble opens the race
//!   (@0x4913c-0x4914e); otherwise it shows `STR#` 2002 #361
//!   ([`NO_CASH_TO_BET`]) and opens nothing (@0x49155-0x4918c).
//! - **The bet**: a racer clicked bets the house bet, `min(cash, 1000)`
//!   ([`house_bet`], @0x48c26-0x48c4f); with Option held the player is
//!   asked for the amount, up to `min(cash, 10000)` ([`asked_max`],
//!   `_DoQuantityDialog` @0x48bf9-0x48c1f), and Cancel asks 0. A bet of 0
//!   or less runs no race ([`Bet::amount`]).
//! - **The winner** ([`draw_winner`]): `Rand(4)`; a roll equal to the last
//!   winner is drawn again until it is neither the last winner nor the
//!   racer chosen (@0x48c97-0x48ccd), so a fresh first roll stands, even
//!   on the racer chosen.
//! - **The payout**: a win pays [`PAYOUT`] times the bet, credited only
//!   once the result text closes (@0x48e03-0x48e30), so a win nets three
//!   times the bet.
//! - **The result text** ([`result_text`]): the race's `dësc`
//!   ([`FIRST_RACE_DESC`] + the winner), and on a win "\r\r", #370 "Your
//!   winnings", ":  ", the winnings with commas, " ", "credit" or
//!   "credits", and "." ([`winnings_line`], @0x48d2e-0x48de1).
//!
//! None of it plays a sound: neither function calls `PlaySound` or
//! `SysBeep`.

use nova_sim_base::chance::Chance;
use nova_sim_base::rulebook::RuleSource;
use nova_sim_story::text_tags::comma_number;

use crate::escort_dialog::{CREDIT, CREDITS};

/// `STR#` 2002 #361: Gamble's text with no cash to bet.
pub const NO_CASH_TO_BET: &str = "Sorry, you don't have enough credits to bet today.";
/// `STR#` 2002 #370: the winnings line's label.
pub const YOUR_WINNINGS: &str = "Your winnings";
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
/// What a win pays, times the bet (@0x48e03-0x48e30).
pub const PAYOUT: i64 = 4;

/// What Gamble opens ([`opening`]).
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

/// What Gamble opens with `cash`: the race above 0, else
/// [`GambleOpening::NoCash`] (`_DoBarDialog` @0x4913c-0x4914e, the cash
/// read signed).
#[must_use]
pub fn opening(cash: i64) -> GambleOpening {
    if cash > 0 {
        GambleOpening::Race {
            asked_max: asked_max(cash),
        }
    } else {
        GambleOpening::NoCash
    }
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

/// The winner drawn on `chance`, `last` the last winner of the bar visit
/// and `chosen` the racer bet on: `Rand(4)`, drawn again while it equals
/// `last` and until it is neither `last` nor `chosen` (@0x48c97-0x48ccd).
/// So the first roll stands unless it equals `last`.
#[must_use]
pub fn draw_winner(last: Option<u8>, chosen: u8, chance: &mut (impl Chance + ?Sized)) -> u8 {
    let mut roll = || u8::try_from(chance.roll(u16::from(RACERS))).unwrap_or(RACERS - 1);
    let mut winner = roll();
    if Some(winner) == last {
        while Some(winner) == last || winner == chosen {
            winner = roll();
        }
    }
    winner
}

/// What a win of `bet` pays, credited once its text closes: [`PAYOUT`]
/// times it.
#[must_use]
pub fn winnings(bet: i64) -> i64 {
    bet.saturating_mul(PAYOUT)
}

/// The line a win appends to the race's text: "\r\r", [`YOUR_WINNINGS`],
/// ":  ", `winnings` as `_StrcatCommaNumber` writes it (its millions as
/// `millions` reads [`RuleKey::PayMillions`](nova_sim_base::rulebook::RuleKey::PayMillions)),
/// " ", [`CREDIT`] at exactly 1 and [`CREDITS`] otherwise, then "."
/// (@0x48d2e-0x48de1).
#[must_use]
pub fn winnings_line(winnings: i64, millions: RuleSource) -> String {
    let unit = if winnings == 1 { CREDIT } else { CREDITS };
    format!(
        "\r\r{YOUR_WINNINGS}:  {} {unit}.",
        comma_number(winnings, millions)
    )
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
    /// The winnings line ([`winnings_line`]) on a win, the winnings due
    /// once the text closes; `None` on a loss.
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
    use nova_sim_base::fixture::Scripted;

    #[test]
    fn gamble_opens_the_race_only_with_cash_above_0() {
        assert_eq!(opening(0), GambleOpening::NoCash);
        assert_eq!(opening(-5), GambleOpening::NoCash);
        assert_eq!(opening(1), GambleOpening::Race { asked_max: 1 });
        assert_eq!(
            opening(50_000),
            GambleOpening::Race {
                asked_max: MAX_ASKED_BET
            }
        );
    }

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
    fn a_first_roll_unlike_the_last_winner_stands_even_on_the_racer_chosen() {
        let mut chance = Scripted::rolling(&[2]);
        assert_eq!(draw_winner(Some(1), 2, &mut chance), 2);
        assert_eq!(chance.sides_asked, [4]);
    }

    #[test]
    fn with_no_last_winner_any_roll_stands() {
        for roll in 0..4 {
            let mut chance = Scripted::rolling(&[roll]);
            assert_eq!(
                draw_winner(None, 0, &mut chance),
                u8::try_from(roll).expect("small")
            );
        }
    }

    #[test]
    fn a_roll_of_the_last_winner_is_drawn_again_past_the_last_and_the_chosen() {
        let mut chance = Scripted::rolling(&[1, 1, 3, 1, 0]);
        assert_eq!(draw_winner(Some(1), 3, &mut chance), 0);
        assert_eq!(chance.sides_asked, [4; 5]);
    }

    /// A chance whose rolls run past the sides asked.
    struct Wild;

    impl Chance for Wild {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, _n: u32) -> u32 {
            0
        }

        fn roll(&mut self, _sides: u16) -> u16 {
            300
        }
    }

    #[test]
    fn a_roll_past_a_racers_number_is_the_last_racer() {
        assert_eq!(draw_winner(None, 0, &mut Wild), RACERS - 1);
    }

    #[test]
    fn a_win_pays_four_times_the_bet() {
        assert_eq!(winnings(250), 1000);
        assert_eq!(PAYOUT, 4);
    }

    #[test]
    fn the_winnings_line_words_the_amount_with_its_commas_and_unit() {
        assert_eq!(
            winnings_line(4_000, RuleSource::Engine),
            "\r\rYour winnings:  4,000 credits."
        );
        assert_eq!(
            winnings_line(1, RuleSource::Engine),
            "\r\rYour winnings:  1 credit."
        );
        assert_eq!(
            winnings_line(2_000_000, RuleSource::Engine),
            "\r\rYour winnings:  2.00M credits."
        );
        assert_eq!(
            winnings_line(2_000_000, RuleSource::Bible),
            "\r\rYour winnings:  2,000,000 credits."
        );
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
