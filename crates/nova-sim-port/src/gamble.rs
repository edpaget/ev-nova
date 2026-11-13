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

pub use nova_sim_state::shown::gamble::{
    AMOUNT_TO_BET, BET, Bet, FIRST_RACE_DESC, GambleOpening, HOUSE_BET, MAX_ASKED_BET,
    NO_CASH_TO_BET, RACE_HELP_DESC, RACERS, RaceRun, asked_max, house_bet, result_text,
};

/// `STR#` 2002 #370: the winnings line's label.
pub const YOUR_WINNINGS: &str = "Your winnings";
/// What a win pays, times the bet (@0x48e03-0x48e30).
pub const PAYOUT: i64 = 4;

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
}
