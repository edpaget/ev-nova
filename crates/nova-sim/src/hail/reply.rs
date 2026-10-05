//! What a hailed ship says: the [`Reply`] an option asks for, which the
//! session turns into words with the conversation's variant.
//!
//! The session says each reply ([`Reply::say`]), reading the strings
//! from a [`CommCatalog`]; a string that is missing, or of a list that
//! is missing, is none.
//!
//! **Ship comm strings** (`_LoadResponse` @0x90a0b in the `EV Nova`
//! executable). The replies come in groups of [`VARIANTS`], one group a
//! situation; a conversation says every reply in the same variant r (see
//! [`deal`](super::deal)). Group g, variant r is `STR#` 3000 string
//! 5g + r + 1 for g up to [`LAST_FIRST_LIST_GROUP`], and `STR#` 3001
//! string 5g + r - 189 beyond ([`Reply::locate`]). The groups used are
//! the constants below; the engine never loads group 27 ("Okay, I'll
//! leave you alone."), so nothing here does.
//!
//! **The opening line** ([`opening`], @0x95b83-0x95d49): "What is it you
//! want?" ([`WHAT_IS_IT`]) from a talkative ship that is hostile or does
//! not like the player, and "Channel open." ([`CHANNEL_OPEN`]) otherwise.
//!
//! **Advice lines** ([`Reply::Advice`]). Greetings to a friendly ship
//! reads one string of a list. A blank one, when the option says so,
//! reads "Greetings." (`STR#` 2002 #175, [`GREETINGS`]) instead. Then the
//! check at @0x91aa9-0x91ae2 ([`advice_fallback`]) replaces a line that
//! is blank or starts with `*` with group 9, "Nice to meet you."
//! ([`NICE_TO_MEET_YOU`]). The original holds the line as a Pascal string
//! but tests it as a C string, so it also replaces a line exactly
//! [`LONG_ADVICE`] characters long, whose length byte is a `*`. Stock has
//! three such lines (`STR#` 7009 #8, 7018 #4 and 7041 #1). The
//! [`RuleKey::LongAdvice`](crate::RuleKey::LongAdvice) entry chooses: by
//! the engine, those are replaced too; otherwise they are shown as
//! written.

use super::like::Attitude;
use crate::catalog::CommCatalog;
use crate::rulebook::RuleSource;

/// How many variants each group of ship comm strings has.
pub const VARIANTS: u8 = 5;
/// The ship comm strings' first list, `STR#` 3000.
pub const COMM_STRINGS: i16 = 3000;
/// The list the groups beyond [`LAST_FIRST_LIST_GROUP`] go on, `STR#`
/// 3001.
pub const MORE_COMM_STRINGS: i16 = 3001;
/// The last group on [`COMM_STRINGS`].
pub const LAST_FIRST_LIST_GROUP: u8 = 37;
/// How far the groups on [`MORE_COMM_STRINGS`] are numbered on from
/// those on the first: 5 x 38 strings less the first.
const MORE_COMM_OFFSET: u16 = 189;

/// The game's messages, `STR#` 2002.
pub const MESSAGES: i16 = 2002;
/// "Greetings.", what an advice line starts as (`STR#` 2002 #175).
pub const GREETINGS: (i16, u16) = (MESSAGES, 175);
/// The length of an advice line the engine mistakes for one starting
/// with `*` (its length byte, 42, is a `*`).
pub const LONG_ADVICE: usize = 42;
/// What an advice line to be passed over starts with.
const PASS_OVER: char = '*';

/// Group 0: "Channel open.": the opening line to a ship that likes the
/// player.
pub const CHANNEL_OPEN: u8 = 0;
/// Group 1: "No response.": an untalkative ship's Greetings.
pub const NO_RESPONSE: u8 = 1;
/// Group 2: "What is it you want?": the opening line to a ship that is
/// hostile or does not like the player.
pub const WHAT_IS_IT: u8 = 2;
/// Group 9: "Nice to meet you.": in place of an advice line passed over.
pub const NICE_TO_MEET_YOU: u8 = 9;
/// Group 12: "Yeah, come back when you actually have some money.": a
/// price the player cannot pay.
pub const NO_MONEY: u8 = 12;
/// Group 13: "Stop wasting my time.": Greetings to a ship that does not
/// like the player.
pub const WASTING_TIME: u8 = 13;
/// Group 14: "You're not in any trouble.": help the player does not
/// need.
pub const NOT_IN_TROUBLE: u8 = 14;
/// Group 16: "I'm busy.".
pub const BUSY: u8 = 16;
/// Group 17: "I'd rather not.": help while the player is under attack (a
/// placeholder for the battle help a follow-up adds).
pub const RATHER_NOT: u8 = 17;
/// Group 18: "You'll have to pay me first.": a bribe's price, in a
/// neutral mood.
pub const PAY_ME_FIRST: u8 = 18;
/// Group 19: "In your dreams, pal.": no deal.
pub const IN_YOUR_DREAMS: u8 = 19;
/// Group 20: "A pleasure doing business with you.": a bribe paid.
pub const PLEASURE: u8 = 20;
/// Group 23: "You're lucky - I'm in a good mood today.": a price, in a
/// good mood.
pub const GOOD_MOOD: u8 = 23;
/// Group 24: "I'm in a bad mood today, so it's going to cost you.": a
/// price, in a bad mood.
pub const BAD_MOOD: u8 = 24;
/// Group 28: "I'll help you out if you pay me.": help's price, in a
/// neutral mood.
pub const HELP_FOR_PAY: u8 = 28;
/// Group 29: "Okay, I'm on my way.": help granted.
pub const ON_MY_WAY: u8 = 29;
/// Group 30: "What? How dare you! Prepare to die!": a bribe declined.
pub const HOW_DARE_YOU: u8 = 30;
/// Group 31: "Ha ha ha. What a comedian.": help's price declined.
pub const COMEDIAN: u8 = 31;

/// The words an option asks a hailed ship to say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    /// A ship comm string group, said in the conversation's variant (see
    /// [`Reply::locate`]).
    Comm(u8),
    /// String `index` of `STR#` `list`, as it is: none when it is
    /// missing.
    Line {
        /// The `STR#`.
        list: i16,
        /// The string, from 1.
        index: u16,
    },
    /// An advice line: string `index` of `STR#` `list`, or "Greetings."
    /// ([`GREETINGS`]) when that is blank and `greet_if_blank`, then
    /// checked as [`advice_fallback`] says, by `long_advice`.
    Advice {
        /// The `STR#`.
        list: i16,
        /// The string, from 1.
        index: u16,
        /// Whether a blank line reads "Greetings." before the check.
        greet_if_blank: bool,
        /// Which reading of a line [`LONG_ADVICE`] long the check follows.
        long_advice: RuleSource,
    },
}

impl Reply {
    /// Where group `group`'s string for variant `variant` is: its `STR#`
    /// and its index, from 1 (see the module docs).
    #[must_use]
    pub fn locate(group: u8, variant: u8) -> (i16, u16) {
        let index = u16::from(VARIANTS) * u16::from(group) + u16::from(variant) + 1;
        if group <= LAST_FIRST_LIST_GROUP {
            (COMM_STRINGS, index)
        } else {
            (MORE_COMM_STRINGS, index - MORE_COMM_OFFSET - 1)
        }
    }

    /// The reply's words in `variant`, read from `catalog` (see the
    /// module docs): a string missing, or of a list missing, is none.
    #[must_use]
    pub fn say(self, variant: u8, catalog: &(impl CommCatalog + ?Sized)) -> String {
        let line = |(list, index): (i16, u16)| {
            let at = usize::from(index.checked_sub(1)?);
            catalog.string_list(list).into_iter().nth(at)
        };
        let line = |at| line(at).unwrap_or_default();
        match self {
            Self::Comm(group) => line(Self::locate(group, variant)),
            Self::Line { list, index } => line((list, index)),
            Self::Advice {
                list,
                index,
                greet_if_blank,
                long_advice,
            } => {
                let mut text = line((list, index));
                if text.is_empty() && greet_if_blank {
                    text = line(GREETINGS);
                }
                if advice_fallback(&text, long_advice) {
                    line(Self::locate(NICE_TO_MEET_YOU, variant))
                } else {
                    text
                }
            }
        }
    }
}

/// The group a ship opens with when hailed (see the module docs).
#[must_use]
pub fn opening(talkative: bool, attitude: Attitude) -> u8 {
    if talkative && attitude != Attitude::Friendly {
        WHAT_IS_IT
    } else {
        CHANNEL_OPEN
    }
}

/// Whether an advice line reads "Nice to meet you." instead: it is
/// blank, or starts with `*`; and by the engine (`long_advice`), it is
/// exactly [`LONG_ADVICE`] characters long (see the module docs).
#[must_use]
pub fn advice_fallback(line: &str, long_advice: RuleSource) -> bool {
    line.is_empty()
        || line.starts_with(PASS_OVER)
        || (long_advice == RuleSource::Engine && line.chars().count() == LONG_ADVICE)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `STR#` 3000 is "c<n>" for each string n up to 200, and 3001 "d<n>"
    /// up to 20; 2002 #175 is "Greetings."; 7000 holds `line` as #1, a
    /// blank #2 and "Hi" as #3; 7001 is empty.
    struct Strings {
        line: String,
    }

    impl CommCatalog for Strings {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                3000 => (1..=200).map(|n| format!("c{n}")).collect(),
                3001 => (1..=20).map(|n| format!("d{n}")).collect(),
                2002 => (1..=200)
                    .map(|n| {
                        if n == 175 {
                            "Greetings.".to_owned()
                        } else {
                            format!("m{n}")
                        }
                    })
                    .collect(),
                7000 => vec![self.line.clone(), String::new(), "Hi".to_owned()],
                _ => Vec::new(),
            }
        }
    }

    fn strings(line: &str) -> Strings {
        Strings {
            line: line.to_owned(),
        }
    }

    fn advice(list: i16, index: u16, greet_if_blank: bool, long_advice: RuleSource) -> Reply {
        Reply::Advice {
            list,
            index,
            greet_if_blank,
            long_advice,
        }
    }

    #[test]
    fn a_comm_reply_is_said_in_the_variant() {
        let catalog = strings("Fine.");
        assert_eq!(Reply::Comm(0).say(3, &catalog), "c4");
        assert_eq!(Reply::Comm(9).say(0, &catalog), "c46");
        assert_eq!(Reply::Comm(38).say(2, &catalog), "d3");
    }

    #[test]
    fn a_line_is_its_string_as_it_is_or_none() {
        let catalog = strings("*Hidden");
        let line = |list, index| Reply::Line { list, index }.say(4, &catalog);
        assert_eq!(line(7000, 1), "*Hidden", "no check");
        assert_eq!(line(7000, 2), "");
        assert_eq!(line(7000, 4), "", "past the end");
        assert_eq!(line(7000, 0), "", "there is no string 0");
        assert_eq!(line(7001, 1), "", "an empty list");
        assert_eq!(line(9999, 1), "", "a missing list");
    }

    #[test]
    fn an_advice_line_is_shown_as_written() {
        let catalog = strings("Welcome to Federation space.");
        assert_eq!(
            advice(7000, 1, false, RuleSource::Engine).say(2, &catalog),
            "Welcome to Federation space."
        );
        assert_eq!(
            advice(7000, 3, false, RuleSource::Engine).say(2, &catalog),
            "Hi",
            "a two-character line is shown"
        );
    }

    #[test]
    fn an_advice_line_passed_over_is_nice_to_meet_you_in_the_variant() {
        let long = "x".repeat(42);
        for (line, index) in [("*", 1), ("*Fed", 1), (long.as_str(), 1), ("", 2), ("", 4)] {
            let catalog = strings(line);
            for variant in [0, 4] {
                assert_eq!(
                    advice(7000, index, false, RuleSource::Engine).say(variant, &catalog),
                    format!("c{}", 46 + u16::from(variant)),
                    "{line:?} #{index}"
                );
            }
        }
        assert_eq!(
            advice(7001, 1, false, RuleSource::Engine).say(1, &strings("")),
            "c47",
            "an empty list"
        );
        assert_eq!(
            advice(9999, 1, true, RuleSource::Engine).say(1, &strings("")),
            "Greetings.",
            "a missing list, greeted"
        );
    }

    #[test]
    fn by_the_other_reading_a_long_advice_line_is_shown_as_written() {
        let long = "x".repeat(42);
        let catalog = strings(&long);
        assert_eq!(
            advice(7000, 1, false, RuleSource::Bible).say(0, &catalog),
            long
        );
        assert_eq!(
            advice(7000, 2, false, RuleSource::Bible).say(0, &catalog),
            "c46",
            "a blank line still is not"
        );
    }

    #[test]
    fn a_blank_advice_line_greets_when_asked_to_before_the_check() {
        let catalog = strings("");
        assert_eq!(
            advice(7000, 2, true, RuleSource::Engine).say(3, &catalog),
            "Greetings."
        );
        assert_eq!(
            advice(7000, 1, true, RuleSource::Engine).say(3, &catalog),
            "Greetings."
        );
        assert_eq!(
            advice(7000, 3, true, RuleSource::Engine).say(3, &catalog),
            "Hi",
            "only a blank one"
        );
        let starred = strings("*Secret");
        assert_eq!(
            advice(7000, 1, true, RuleSource::Engine).say(3, &starred),
            "c49",
            "a starred line is not blank"
        );
    }

    #[test]
    fn a_group_and_variant_locate_a_ship_comm_string() {
        assert_eq!(Reply::locate(0, 0), (3000, 1));
        assert_eq!(Reply::locate(9, 2), (3000, 48));
        assert_eq!(Reply::locate(37, 4), (3000, 190));
        assert_eq!(Reply::locate(38, 0), (3001, 1));
        assert_eq!(Reply::locate(39, 3), (3001, 9));
    }

    #[test]
    fn a_talkative_ship_that_is_not_friendly_asks_what_the_player_wants() {
        assert_eq!(opening(true, Attitude::Hostile), WHAT_IS_IT);
        assert_eq!(opening(true, Attitude::Unfriendly), WHAT_IS_IT);
        assert_eq!(opening(true, Attitude::Friendly), CHANNEL_OPEN);
        assert_eq!(opening(false, Attitude::Hostile), CHANNEL_OPEN);
        assert_eq!(opening(false, Attitude::Unfriendly), CHANNEL_OPEN);
    }

    #[test]
    fn the_groups_are_the_originals() {
        assert_eq!(
            [
                CHANNEL_OPEN,
                NO_RESPONSE,
                WHAT_IS_IT,
                NICE_TO_MEET_YOU,
                NO_MONEY,
                WASTING_TIME,
                NOT_IN_TROUBLE,
                BUSY,
                RATHER_NOT,
                PAY_ME_FIRST,
                IN_YOUR_DREAMS,
                PLEASURE,
                GOOD_MOOD,
                BAD_MOOD,
                HELP_FOR_PAY,
                ON_MY_WAY,
                HOW_DARE_YOU,
                COMEDIAN,
            ],
            [
                0, 1, 2, 9, 12, 13, 14, 16, 17, 18, 19, 20, 23, 24, 28, 29, 30, 31
            ]
        );
        assert_eq!(GREETINGS, (2002, 175));
    }

    #[test]
    fn a_blank_line_or_one_starting_with_a_star_is_passed_over_either_way() {
        for source in RuleSource::ALL {
            for line in ["", "*", "*Federation"] {
                assert!(advice_fallback(line, source), "{line:?} {source:?}");
            }
            for line in ["a*b", "Hi", "X", " *"] {
                assert!(!advice_fallback(line, source), "{line:?} {source:?}");
            }
        }
    }

    #[test]
    fn by_the_engine_a_line_of_exactly_42_characters_is_passed_over() {
        let long = |n| "x".repeat(n);
        assert!(advice_fallback(&long(42), RuleSource::Engine));
        assert!(!advice_fallback(&long(41), RuleSource::Engine));
        assert!(!advice_fallback(&long(43), RuleSource::Engine));
        let accented = format!("{}é", "x".repeat(41));
        assert!(
            advice_fallback(&accented, RuleSource::Engine),
            "characters, not bytes"
        );
    }

    #[test]
    fn otherwise_a_line_of_42_characters_is_shown_as_written() {
        assert!(!advice_fallback(&"x".repeat(42), RuleSource::Bible));
    }
}
