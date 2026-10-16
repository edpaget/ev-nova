//! A fleet's arrival quote: the line an announced fleet's arrival shows
//! (`_HyperSpawnFleet` @0x42607-0x426f4 in the `EV Nova` executable).
//!
//! - **The string.** `_GetRandomIndString(buf, Quote)` (@0x42642) picks
//!   one string of the `flët`'s `Quote` `STR#`; the session picks it
//!   with its one port of that routine (`Session::random_string`), a
//!   missing or empty list giving an empty line.
//! - **The digits** (@0x42647-0x42696), filled here ([`fill_digits`]).
//!   Each '#' becomes a digit: the first '#' of each run of them
//!   `Rand(9) + '1'`, 1 to 9, and each '#' after it in the run
//!   `Rand(10) + '0'`, 0 to 9. Any other character ends the run.
//!
//! The session shows the line and beeps for it (see
//! [`SimMessage::FleetArrivalQuote`](crate::SimMessage::FleetArrivalQuote)).

use crate::chance::Chance;

/// `Rand(9) + '1'`: the first digit of a run of '#'s, 1 to 9.
const LEAD_DIGITS: u32 = 9;
/// `Rand(10) + '0'`: each later digit of the run, 0 to 9.
const DIGITS: u32 = 10;

/// `line` with each of its '#'s filled with a digit drawn on `chance`
/// (see the module docs); its other characters as they are.
pub fn fill_digits(line: &str, chance: &mut (impl Chance + ?Sized)) -> String {
    let mut in_run = false;
    line.chars()
        .map(|c| {
            if c != '#' {
                in_run = false;
                return c;
            }
            let digit = if in_run {
                chance.below(DIGITS)
            } else {
                chance.below(LEAD_DIGITS) + 1
            };
            in_run = true;
            char::from_digit(digit, DIGITS).unwrap_or('#')
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Draws;

    #[test]
    fn a_line_without_a_hash_is_kept_and_draws_nothing() {
        let mut chance = Draws::of(&[]);
        assert_eq!(fill_digits("Hail, pilot", &mut chance), "Hail, pilot");
        assert_eq!(fill_digits("", &mut chance), "");
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn the_first_hash_of_each_run_is_1_to_9_and_the_rest_0_to_9() {
        let line = "Call #-### or #";
        let mut lowest = Draws::of(&[0, 0, 0, 0, 0]);
        assert_eq!(fill_digits(line, &mut lowest), "Call 1-100 or 1");
        let mut highest = crate::NeverFires;
        assert_eq!(fill_digits(line, &mut highest), "Call 9-999 or 9");
        assert_eq!(lowest.asked, [9, 9, 10, 10, 9], "a lead digit a run");
    }
}
