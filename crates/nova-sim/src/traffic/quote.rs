//! A fleet's arrival quote: the line an announced fleet's arrival shows
//! (`_HyperSpawnFleet` @0x42607-0x426f4 in the `EV Nova` executable).
//!
//! - **The string.** `_GetRandomIndString(buf, Quote)` (@0x42642,
//!   @0x72ef7-0x72f8f) clears the line, then copies string
//!   `Rand(count) + 1` of the `flët`'s `Quote` `STR#`. A missing or empty
//!   list leaves the line empty, with no draw (@0x72f0f, @0x72f65).
//! - **The digits** (@0x42647-0x42696). Each '#' becomes a digit: the
//!   first '#' of each run of them `Rand(9) + '1'`, 1 to 9, and each
//!   '#' after it in the run `Rand(10) + '0'`, 0 to 9. Any other
//!   character ends the run.
//!
//! The session shows the line and beeps for it (see
//! [`SimMessage::FleetArrivalQuote`](crate::SimMessage::FleetArrivalQuote)).

use crate::chance::Chance;

/// `Rand(9) + '1'`: the first digit of a run of '#'s, 1 to 9.
const LEAD_DIGITS: u32 = 9;
/// `Rand(10) + '0'`: each later digit of the run, 0 to 9.
const DIGITS: u32 = 10;

/// The text of a fleet's arrival quote drawn from its `strings`: one of
/// them, picked on `chance`, its '#'s filled with digits drawn on it
/// (see the module docs); empty, with no draw, when there are none.
pub fn arrival_quote(strings: &[String], chance: &mut (impl Chance + ?Sized)) -> String {
    if strings.is_empty() {
        return String::new();
    }
    let count = u32::try_from(strings.len()).unwrap_or(u32::MAX);
    let picked = usize::try_from(chance.below(count)).unwrap_or(0);
    let Some(line) = strings.get(picked) else {
        return String::new();
    };
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

    fn list(strings: &[&str]) -> Vec<String> {
        strings.iter().map(|&s| s.to_owned()).collect()
    }

    #[test]
    fn a_quote_is_the_string_its_draw_picks() {
        let mut chance = Draws::of(&[1]);
        let quote = arrival_quote(&list(&["One", "Two", "Three"]), &mut chance);
        assert_eq!(quote, "Two");
        assert_eq!(chance.asked, [3]);
    }

    #[test]
    fn no_list_or_an_empty_one_gives_an_empty_quote_and_draws_nothing() {
        let mut chance = Draws::of(&[]);
        assert_eq!(arrival_quote(&[], &mut chance), "");
        assert_eq!(arrival_quote(&list(&[""]), &mut chance), "");
        assert_eq!(chance.asked, [1], "only the pick of the one string");
    }

    #[test]
    fn the_first_hash_of_each_run_is_1_to_9_and_the_rest_0_to_9() {
        let line = list(&["Call #-### or #"]);
        let mut lowest = Draws::of(&[0, 0, 0, 0, 0, 0]);
        assert_eq!(arrival_quote(&line, &mut lowest), "Call 1-100 or 1");
        let mut highest = crate::NeverFires;
        assert_eq!(arrival_quote(&line, &mut highest), "Call 9-999 or 9");
        let asked = &lowest.asked[1..];
        assert_eq!(asked.iter().filter(|&&n| n == 9).count(), 3, "one a run");
        assert_eq!(asked.iter().filter(|&&n| n == 10).count(), 2);
        assert_eq!(asked.len(), 5);
    }
}
