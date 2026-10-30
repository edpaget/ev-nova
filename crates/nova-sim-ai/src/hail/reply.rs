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
//! **The opening line** ([`opening`], @0x95b83-0x95d49). An untalkative
//! or disabled ship opens "Channel open." ([`CHANNEL_OPEN`]), and a hostile
//! one, a threat to the player, "What is it you want?" ([`WHAT_IS_IT`])
//! (@0x95cb5-0x95d0b). A ship the player leads ([`Led`]) then opens by
//! what it is: "What can I do for you, sir?" ([`FIGHTER_OPENING`]) from
//! the player's launched fighter, the original's AI type 5, and "What can
//! I do for you?" ([`ESCORT_OPENING`]) from an escort, AI type 6
//! (@0x95d0b-0x95d49). Any other ship says "Channel open." when it likes
//! the player and "What is it you want?" when it does not.
//!
//! A talkative mission ship ([`MissionHail`], its mark's slot active)
//! opens by its mission instead, whatever its attitude (@0x95b9c-0x95cb3),
//! each step overriding the one before: "Channel open."; "What is it you
//! want?" for `ShipGoal` 0, destroy them (@0x95bf8-0x95c09); "Glad to see
//! you, " ([`GLAD_TO_SEE_YOU`]) then the player's ship's name and "." for
//! `ShipGoal` 3, escort them (@0x95c10-0x95c6c, [`Reply::Named`]); and
//! "What is it you want?" for `ShipBehav` 0, attacking the player
//! (@0x95c71-0x95c97); then "What can I do for you?"
//! ([`ESCORT_OPENING`]) for `ShipBehav` 1, escorting the player
//! (@0x95ca0-0x95cb3, @0x95d22-0x95d49), whatever its goal. An
//! untalkative mission ship opens "Channel open." as any untalkative ship
//! does (@0x95cb5).
//!
//! **Advice lines** ([`Reply::Advice`]). Greetings to a friendly ship
//! reads one string of a list, or first a plug-in's `'STR '` that patches
//! it (a government's hail, see [`advice`](super::advice)): a patch's
//! text stands, an empty one leaves "Greetings." and an unreadable one
//! reads "". A blank string, when the option says so, reads "Greetings."
//! (`STR#` 2002 #175, [`GREETINGS`]) instead; a specific advice line that
//! is not blank is parsed, its conditional text resolved, as the press
//! picks it (`_LoadAdvice` @0x919ca, [`Reply::say_parsed`]). A good-price
//! or disaster line ([`Reply::Rolled`]) is rolled as the hail opens, and
//! the session keeps it. Then the check at @0x91aa9-0x91ae2
//! ([`advice_fallback`], [`checked`]) replaces a line that is blank or
//! starts with `*` with group 9, "Nice to meet you."
//! ([`NICE_TO_MEET_YOU`]). A person's comm quote ([`Reply::Quote`]) comes
//! after the check, so it is shown as written. The original holds the line as a Pascal string
//! but tests it as a C string, so it also replaces a line exactly
//! [`LONG_ADVICE`] characters long, whose length byte is a `*`. Stock has
//! three such lines (`STR#` 7009 #8, 7018 #4 and 7041 #1). The
//! [`RuleKey::LongAdvice`](crate::RuleKey::LongAdvice) entry chooses: by
//! the engine, those are replaced too; otherwise they are shown as
//! written.

use super::advice::comm_quote_patch;
use super::like::Attitude;
use crate::catalog::{CommCatalog, StringPatch};
use crate::rulebook::RuleSource;
use crate::traffic::COMM_QUOTES;
use crate::traffic::ShipBehav;
use crate::traffic::ShipGoal;

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
/// Group 8: "Glad to see you, ": a mission ship's opening line to the
/// player escorting it, the player's ship's name and "." following
/// ([`Reply::Named`]).
pub const GLAD_TO_SEE_YOU: u8 = 8;
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
/// Group 17: "I'd rather not.": help asked of a wimpy trader while an
/// enemy of the ship's threatens the player and it has no battle help
/// (`_DoCommDialog` @0x96399, @0x96459), or of a brave trader that won't
/// fight for a price (@0x9639f-0x963d9).
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
/// Group 4: "What can I do for you?": the opening line to the player's
/// escort.
pub const ESCORT_OPENING: u8 = 4;
/// Group 5: "What can I do for you, sir?": the opening line to the
/// player's launched fighter (`_DoCommDialog` @0x95d22-0x95d33).
pub const FIGHTER_OPENING: u8 = 5;
/// Group 22: "Sorry sir, I'm just an escort.": Request Assistance to a
/// busy ship of the original's AI type above 4 (a mission escort, an NPC
/// fleet escort or an NPC's carried fighter, `Hail::escort_ai`,
/// @0x962ee-0x96304).
pub const JUST_AN_ESCORT: u8 = 22;
/// Group 25: "Sorry, I can't do that.": Request Assistance's refuel or
/// repair asked of a ship of AI type above 4 (a mission escort, an NPC
/// fleet escort or an NPC's carried fighter, `Hail::escort_ai`), before
/// any price (`_DoCommDialog` @0x96564-0x9656c, @0x9663a).
pub const CANT_DO_THAT: u8 = 25;
/// Group 38: "Goodbye, captain.": an escort released.
pub const RELEASED: u8 = 38;

/// The words an option asks a hailed ship to say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply {
    /// A ship comm string group, said in the conversation's variant (see
    /// [`Reply::locate`]).
    Comm(u8),
    /// A ship comm string group, said in the conversation's variant, then
    /// the player's ship's name and "." (`strcat` @0x95c56, the `.`
    /// @0x95c63): a missing string leaves the name and ".".
    Named(u8),
    /// String `index` of `STR#` `list`, as it is: none when it is
    /// missing.
    Line {
        /// The `STR#`.
        list: i16,
        /// The string, from 1.
        index: u16,
    },
    /// An advice line: the `'STR '` `patch` when there is one, else string
    /// `index` of `STR#` `list`, or "Greetings." ([`GREETINGS`]) when that
    /// is blank and `greet_if_blank`, then checked as [`advice_fallback`]
    /// says, by `long_advice`.
    Advice {
        /// The `STR#`.
        list: i16,
        /// The string, from 1.
        index: u16,
        /// Whether a blank line reads "Greetings." before the check.
        greet_if_blank: bool,
        /// The `'STR '` read before the `STR#`, a government hail's
        /// ([`govt_hail_patch`](super::advice::govt_hail_patch)): its text
        /// stands, an empty one reads "Greetings." (the buffer kept) and an
        /// unreadable one "", each then checked (see the module docs).
        patch: Option<i16>,
        /// Which reading of a line [`LONG_ADVICE`] long the check follows.
        long_advice: RuleSource,
    },
    /// A person's comm quote `index`, as written, with no check: its
    /// `'STR '` ([`comm_quote_patch`](super::advice::comm_quote_patch))
    /// when there is one, else `STR#` [`COMM_QUOTES`] string `index`. An
    /// unreadable patch reads "", and an empty one reads "Greetings." when
    /// `empty_reads_greetings` (the advice buffer kept, as Greetings says
    /// it), else "".
    Quote {
        /// The `CommQuote`, from 1.
        index: u16,
        /// Whether an empty patch leaves "Greetings.": Greetings' quote
        /// does (@0x91ae7-0x91b55), the opening line of
        /// [`RuleKey::CommQuote`](crate::RuleKey::CommQuote)'s other
        /// reading, which has no buffer, does not.
        empty_reads_greetings: bool,
    },
    /// The good-price or disaster line the hail rolled as it opened, which
    /// the session keeps (see [`advice`](super::advice)), checked as
    /// [`checked`] says by `long_advice`. On its own, with nothing rolled,
    /// it reads "Greetings.".
    Rolled {
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

    /// The reply's words in `variant`, read from `catalog`, a
    /// [`Reply::Named`] naming the player's ship `ship_name` (see the
    /// module docs): a string missing, or of a list missing, is none. A
    /// specific advice line is read as it is, unparsed
    /// ([`Reply::say_parsed`]).
    #[must_use]
    pub fn say(
        self,
        variant: u8,
        catalog: &(impl CommCatalog + ?Sized),
        ship_name: &str,
    ) -> String {
        self.say_parsed(variant, catalog, ship_name, str::to_owned)
    }

    /// The reply's words as [`Reply::say`] gives them, but a specific
    /// advice line ([`Reply::Advice`] with `greet_if_blank`, `STR#` 7500
    /// on) that is not blank is first given to `parse`, as `_LoadAdvice`
    /// parses it (@0x919b0-0x919ca) before the "Nice to meet you." check
    /// (@0x91aa9). "Greetings." in a blank one's place is not parsed
    /// (@0x919e6), nor is any other line.
    #[must_use]
    pub fn say_parsed(
        self,
        variant: u8,
        catalog: &(impl CommCatalog + ?Sized),
        ship_name: &str,
        parse: impl FnOnce(&str) -> String,
    ) -> String {
        let line = |(list, index): (i16, u16)| {
            let at = usize::from(index.checked_sub(1)?);
            catalog.string_list(list).into_iter().nth(at)
        };
        let line = |at| line(at).unwrap_or_default();
        match self {
            Self::Comm(group) => line(Self::locate(group, variant)),
            Self::Named(group) => format!("{}{ship_name}.", line(Self::locate(group, variant))),
            Self::Line { list, index } => line((list, index)),
            Self::Advice {
                list,
                index,
                greet_if_blank,
                patch,
                long_advice,
            } => {
                let patch = patch.map_or(StringPatch::Absent, |id| catalog.plugin_string(id));
                let text = match patch {
                    StringPatch::Text(text) => text,
                    StringPatch::Empty => line(GREETINGS),
                    StringPatch::Unreadable => String::new(),
                    StringPatch::Absent => {
                        let text = line((list, index));
                        if text.is_empty() && greet_if_blank {
                            line(GREETINGS)
                        } else if greet_if_blank {
                            parse(&text)
                        } else {
                            text
                        }
                    }
                };
                checked(text, long_advice, variant, catalog)
            }
            Self::Quote {
                index,
                empty_reads_greetings,
            } => {
                let patch = comm_quote_patch(index)
                    .map_or(StringPatch::Absent, |id| catalog.plugin_string(id));
                match patch {
                    StringPatch::Text(text) => text,
                    StringPatch::Empty if empty_reads_greetings => line(GREETINGS),
                    StringPatch::Empty | StringPatch::Unreadable => String::new(),
                    StringPatch::Absent => line((COMM_QUOTES, index)),
                }
            }
            Self::Rolled { .. } => line(GREETINGS),
        }
    }
}

/// `line`, an advice line, as the check leaves it (@0x91aa9-0x91ae2):
/// "Nice to meet you." in `variant`, read from `catalog`, when
/// [`advice_fallback`] holds by `long_advice`, else `line`.
#[must_use]
pub fn checked(
    line: String,
    long_advice: RuleSource,
    variant: u8,
    catalog: &(impl CommCatalog + ?Sized),
) -> String {
    if advice_fallback(&line, long_advice) {
        Reply::Comm(NICE_TO_MEET_YOU).say(variant, catalog, "")
    } else {
        line
    }
}

/// A hailed mission ship's mission, as its opening line reads it: the
/// `ShipGoal` of its slot's mission and its mark's `ShipBehav`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissionHail {
    /// What the player must do to it.
    pub goal: ShipGoal,
    /// How it behaves.
    pub behav: ShipBehav,
}

/// What an unmarked ship the player leads is, as its opening line reads
/// it: the original's AI type 5 or 6 (`_DoCommDialog` @0x95d0b-0x95d49).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Led {
    /// The player's launched fighter, AI type 5 (`_LaunchFighter`
    /// @0x3d771).
    Fighter,
    /// The player's escort, AI type 6.
    Escort,
}

/// What a ship opens with when hailed: by its `mission`, when it is a
/// talkative mission ship; then by its attitude and, for a ship the player
/// leads, by what it is (`led`) (see the module docs).
#[must_use]
pub fn opening(
    talkative: bool,
    attitude: Attitude,
    mission: Option<MissionHail>,
    led: Option<Led>,
) -> Reply {
    match (mission, led) {
        (Some(mission), _) if talkative => mission_opening(mission),
        _ if !talkative => Reply::Comm(CHANNEL_OPEN),
        _ if attitude == Attitude::Hostile => Reply::Comm(WHAT_IS_IT),
        (_, Some(Led::Fighter)) => Reply::Comm(FIGHTER_OPENING),
        (_, Some(Led::Escort)) => Reply::Comm(ESCORT_OPENING),
        _ if attitude == Attitude::Friendly => Reply::Comm(CHANNEL_OPEN),
        _ => Reply::Comm(WHAT_IS_IT),
    }
}

/// A talkative mission ship's opening line (see the module docs).
fn mission_opening(mission: MissionHail) -> Reply {
    match mission.behav {
        ShipBehav::AttackPlayer => return Reply::Comm(WHAT_IS_IT),
        ShipBehav::EscortPlayer => return Reply::Comm(ESCORT_OPENING),
        _ => {}
    }
    match mission.goal {
        ShipGoal::Destroy => Reply::Comm(WHAT_IS_IT),
        ShipGoal::Escort => Reply::Named(GLAD_TO_SEE_YOU),
        _ => Reply::Comm(CHANNEL_OPEN),
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
    use crate::traffic::ShipBehav;
    use crate::traffic::ShipGoal;

    /// `STR#` 3000 is "c<n>" for each string n up to 200, and 3001 "d<n>"
    /// up to 20; 2002 #175 is "Greetings."; 7000 holds `line` as #1, a
    /// blank #2 and "Hi" as #3; 7001 is empty; 7100 is "q<n>" up to 30.
    /// Its `'STR '`s are `patches`.
    struct Strings {
        line: String,
        patches: Vec<(i16, StringPatch)>,
    }

    impl CommCatalog for Strings {
        fn plugin_string(&self, id: i16) -> StringPatch {
            self.patches
                .iter()
                .find(|(at, _)| *at == id)
                .map_or(StringPatch::Absent, |(_, patch)| patch.clone())
        }

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
                7100 => (1..=30).map(|n| format!("q{n}")).collect(),
                _ => Vec::new(),
            }
        }
    }

    fn strings(line: &str) -> Strings {
        Strings {
            line: line.to_owned(),
            patches: Vec::new(),
        }
    }

    /// [`strings`] with `'STR '` `id` `patch`.
    fn patched(line: &str, id: i16, patch: StringPatch) -> Strings {
        Strings {
            patches: vec![(id, patch)],
            ..strings(line)
        }
    }

    fn advice(list: i16, index: u16, greet_if_blank: bool, long_advice: RuleSource) -> Reply {
        Reply::Advice {
            list,
            index,
            greet_if_blank,
            patch: None,
            long_advice,
        }
    }

    /// A government's hail, `STR#` 7000 #1, patched by `'STR '` 10010.
    fn govt_hail(long_advice: RuleSource) -> Reply {
        Reply::Advice {
            list: 7000,
            index: 1,
            greet_if_blank: false,
            patch: Some(10_010),
            long_advice,
        }
    }

    fn text(line: &str) -> StringPatch {
        StringPatch::Text(line.to_owned())
    }

    #[test]
    fn a_patched_advice_line_reads_its_patch_and_is_then_checked() {
        let said =
            |patch| govt_hail(RuleSource::Engine).say(0, &patched("Hail.", 10_010, patch), "");
        assert_eq!(said(text("Welcome.")), "Welcome.", "the patch stands");
        assert_eq!(said(text("*Hidden")), "c46", "then the check");
        assert_eq!(said(text(&"x".repeat(42))), "c46", "the long line too");
        assert_eq!(said(StringPatch::Empty), "Greetings.", "the buffer kept");
        assert_eq!(
            said(StringPatch::Unreadable),
            "c46",
            "read empty, then checked"
        );
        assert_eq!(said(StringPatch::Absent), "Hail.", "the STR# line");
        let other_id =
            govt_hail(RuleSource::Engine).say(0, &patched("Hail.", 10_011, text("No.")), "");
        assert_eq!(other_id, "Hail.", "another 'STR ' patches nothing");
        let long = "x".repeat(42);
        assert_eq!(
            govt_hail(RuleSource::Bible).say(0, &patched("", 10_010, text(&long)), ""),
            long,
            "by the other reading"
        );
    }

    #[test]
    fn an_unpatched_advice_line_reads_no_str() {
        let catalog = patched("Hail.", 10_010, text("Welcome."));
        assert_eq!(
            advice(7000, 1, false, RuleSource::Engine).say(0, &catalog, ""),
            "Hail."
        );
    }

    fn quote(index: u16, empty_reads_greetings: bool) -> Reply {
        Reply::Quote {
            index,
            empty_reads_greetings,
        }
    }

    #[test]
    fn a_comm_quote_reads_its_patch_as_written() {
        for greets in [true, false] {
            let said = |patch| quote(24, greets).say(0, &patched("", 15_024, patch), "");
            assert_eq!(said(text("*Well met.")), "*Well met.", "no check");
            assert_eq!(said(StringPatch::Unreadable), "", "{greets}");
            assert_eq!(said(StringPatch::Absent), "q24", "STR# 7100 #24");
            assert_eq!(
                quote(24, greets).say(0, &patched("", 15_023, text("No.")), ""),
                "q24"
            );
            assert_eq!(
                quote(31, greets).say(0, &strings(""), ""),
                "",
                "past the end"
            );
        }
    }

    #[test]
    fn an_empty_comm_quote_patch_greets_only_from_greetings() {
        let catalog = patched("", 15_024, StringPatch::Empty);
        assert_eq!(quote(24, true).say(0, &catalog, ""), "Greetings.");
        assert_eq!(quote(24, false).say(0, &catalog, ""), "", "the opening's");
    }

    #[test]
    fn a_comm_quote_past_the_patch_ids_reads_its_str_list() {
        let catalog = patched("", 14_999, text("No."));
        assert_eq!(quote(u16::MAX, true).say(0, &catalog, ""), "");
    }

    #[test]
    fn a_rolled_line_on_its_own_greets() {
        for long_advice in RuleSource::ALL {
            assert_eq!(
                Reply::Rolled { long_advice }.say(2, &strings(""), ""),
                "Greetings."
            );
        }
    }

    #[test]
    fn a_rolled_line_is_checked_in_the_variant_by_its_reading() {
        let catalog = strings("");
        let long = "x".repeat(42);
        let checked = |line: &str, long_advice| checked(line.to_owned(), long_advice, 3, &catalog);
        assert_eq!(
            checked("Earth is fine.", RuleSource::Engine),
            "Earth is fine."
        );
        assert_eq!(checked("*Earth is fine.", RuleSource::Bible), "c49");
        assert_eq!(checked(&long, RuleSource::Engine), "c49");
        assert_eq!(checked(&long, RuleSource::Bible), long);
    }

    #[test]
    fn a_comm_reply_is_said_in_the_variant() {
        let catalog = strings("Fine.");
        assert_eq!(Reply::Comm(0).say(3, &catalog, ""), "c4");
        assert_eq!(Reply::Comm(9).say(0, &catalog, ""), "c46");
        assert_eq!(Reply::Comm(38).say(2, &catalog, ""), "d3");
    }

    #[test]
    fn a_line_is_its_string_as_it_is_or_none() {
        let catalog = strings("*Hidden");
        let line = |list, index| Reply::Line { list, index }.say(4, &catalog, "");
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
            advice(7000, 1, false, RuleSource::Engine).say(2, &catalog, ""),
            "Welcome to Federation space."
        );
        assert_eq!(
            advice(7000, 3, false, RuleSource::Engine).say(2, &catalog, ""),
            "Hi",
            "a two-character line is shown"
        );
    }

    #[test]
    fn only_a_specific_advice_line_not_blank_is_parsed() {
        let catalog = strings("Raw");
        let parse = |line: &str| format!("parsed {line}");
        let said = |index, greet_if_blank| {
            advice(7000, index, greet_if_blank, RuleSource::Engine)
                .say_parsed(0, &catalog, "", parse)
        };
        assert_eq!(said(1, true), "parsed Raw", "specific advice (@0x919ca)");
        assert_eq!(said(1, false), "Raw", "a government's line is not parsed");
        assert_eq!(
            said(2, true),
            "Greetings.",
            "nor Greetings in a blank one's place"
        );
        let empty = advice(7000, 1, true, RuleSource::Engine)
            .say_parsed(0, &catalog, "", |_| String::new());
        assert_eq!(empty, "c46", "emptied, then passed over (@0x91aa9)");
        assert_eq!(
            Reply::Comm(0).say_parsed(3, &catalog, "", parse),
            "c4",
            "nor any other reply"
        );
    }

    #[test]
    fn an_advice_line_passed_over_is_nice_to_meet_you_in_the_variant() {
        let long = "x".repeat(42);
        for (line, index) in [("*", 1), ("*Fed", 1), (long.as_str(), 1), ("", 2), ("", 4)] {
            let catalog = strings(line);
            for variant in [0, 4] {
                assert_eq!(
                    advice(7000, index, false, RuleSource::Engine).say(variant, &catalog, ""),
                    format!("c{}", 46 + u16::from(variant)),
                    "{line:?} #{index}"
                );
            }
        }
        assert_eq!(
            advice(7001, 1, false, RuleSource::Engine).say(1, &strings(""), ""),
            "c47",
            "an empty list"
        );
        assert_eq!(
            advice(9999, 1, true, RuleSource::Engine).say(1, &strings(""), ""),
            "Greetings.",
            "a missing list, greeted"
        );
    }

    #[test]
    fn by_the_other_reading_a_long_advice_line_is_shown_as_written() {
        let long = "x".repeat(42);
        let catalog = strings(&long);
        assert_eq!(
            advice(7000, 1, false, RuleSource::Bible).say(0, &catalog, ""),
            long
        );
        assert_eq!(
            advice(7000, 2, false, RuleSource::Bible).say(0, &catalog, ""),
            "c46",
            "a blank line still is not"
        );
    }

    #[test]
    fn a_blank_advice_line_greets_when_asked_to_before_the_check() {
        let catalog = strings("");
        assert_eq!(
            advice(7000, 2, true, RuleSource::Engine).say(3, &catalog, ""),
            "Greetings."
        );
        assert_eq!(
            advice(7000, 1, true, RuleSource::Engine).say(3, &catalog, ""),
            "Greetings."
        );
        assert_eq!(
            advice(7000, 3, true, RuleSource::Engine).say(3, &catalog, ""),
            "Hi",
            "only a blank one"
        );
        let starred = strings("*Secret");
        assert_eq!(
            advice(7000, 1, true, RuleSource::Engine).say(3, &starred, ""),
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
        let opening = |talkative, attitude| opening(talkative, attitude, None, None);
        assert_eq!(opening(true, Attitude::Hostile), Reply::Comm(WHAT_IS_IT));
        assert_eq!(opening(true, Attitude::Unfriendly), Reply::Comm(WHAT_IS_IT));
        assert_eq!(opening(true, Attitude::Friendly), Reply::Comm(CHANNEL_OPEN));
        assert_eq!(opening(false, Attitude::Hostile), Reply::Comm(CHANNEL_OPEN));
        assert_eq!(
            opening(false, Attitude::Unfriendly),
            Reply::Comm(CHANNEL_OPEN)
        );
    }

    #[test]
    fn a_ship_the_player_leads_opens_by_what_it_is_unless_untalkative_or_hostile() {
        for attitude in [Attitude::Friendly, Attitude::Unfriendly] {
            assert_eq!(
                opening(true, attitude, None, Some(Led::Fighter)),
                Reply::Comm(FIGHTER_OPENING),
                "{attitude:?}"
            );
            assert_eq!(
                opening(true, attitude, None, Some(Led::Escort)),
                Reply::Comm(ESCORT_OPENING),
                "{attitude:?}"
            );
        }
        for led in [Led::Fighter, Led::Escort] {
            for attitude in ATTITUDES {
                assert_eq!(
                    opening(false, attitude, None, Some(led)),
                    Reply::Comm(CHANNEL_OPEN),
                    "untalkative {led:?} {attitude:?}"
                );
            }
            assert_eq!(
                opening(true, Attitude::Hostile, None, Some(led)),
                Reply::Comm(WHAT_IS_IT),
                "hostile {led:?}"
            );
        }
    }

    const ATTITUDES: [Attitude; 3] = [Attitude::Friendly, Attitude::Unfriendly, Attitude::Hostile];
    const GOALS: [ShipGoal; 9] = [
        ShipGoal::None,
        ShipGoal::Destroy,
        ShipGoal::Disable,
        ShipGoal::Board,
        ShipGoal::Escort,
        ShipGoal::Observe,
        ShipGoal::Rescue,
        ShipGoal::ChaseOff,
        ShipGoal::Other,
    ];
    const BEHAVS: [ShipBehav; 4] = [
        ShipBehav::Own,
        ShipBehav::AttackPlayer,
        ShipBehav::EscortPlayer,
        ShipBehav::AttackStellars,
    ];

    /// The opening of a talkative mission ship of `goal` and `behav`, the
    /// same whatever its attitude.
    fn mission_opening(goal: ShipGoal, behav: ShipBehav) -> Reply {
        let mission = Some(MissionHail { goal, behav });
        let said = opening(true, Attitude::Friendly, mission, None);
        for attitude in ATTITUDES {
            assert_eq!(opening(true, attitude, mission, None), said, "{attitude:?}");
            for led in [Led::Fighter, Led::Escort] {
                assert_eq!(opening(true, attitude, mission, Some(led)), said, "{led:?}");
            }
        }
        said
    }

    #[test]
    fn a_talkative_mission_ship_to_destroy_or_attacking_the_player_asks_what_it_wants() {
        for behav in [ShipBehav::Own, ShipBehav::AttackStellars] {
            assert_eq!(
                mission_opening(ShipGoal::Destroy, behav),
                Reply::Comm(WHAT_IS_IT),
                "{behav:?}"
            );
        }
        for goal in GOALS {
            assert_eq!(
                mission_opening(goal, ShipBehav::AttackPlayer),
                Reply::Comm(WHAT_IS_IT),
                "{goal:?}, the escort goal included"
            );
        }
    }

    #[test]
    fn a_talkative_mission_ship_to_escort_is_glad_to_see_the_player() {
        for behav in [ShipBehav::Own, ShipBehav::AttackStellars] {
            assert_eq!(
                mission_opening(ShipGoal::Escort, behav),
                Reply::Named(GLAD_TO_SEE_YOU),
                "{behav:?}"
            );
        }
    }

    #[test]
    fn a_talkative_mission_ship_escorting_the_player_asks_what_it_can_do_whatever_its_goal() {
        for goal in GOALS {
            assert_eq!(
                mission_opening(goal, ShipBehav::EscortPlayer),
                Reply::Comm(ESCORT_OPENING),
                "{goal:?}, the escort goal included"
            );
        }
        assert_eq!(
            opening(
                false,
                Attitude::Friendly,
                Some(MissionHail {
                    goal: ShipGoal::Escort,
                    behav: ShipBehav::EscortPlayer
                }),
                None
            ),
            Reply::Comm(CHANNEL_OPEN),
            "untalkative"
        );
    }

    #[test]
    fn any_other_talkative_mission_ship_opens_the_channel_whatever_its_attitude() {
        for goal in GOALS {
            if matches!(goal, ShipGoal::Destroy | ShipGoal::Escort) {
                continue;
            }
            for behav in [ShipBehav::Own, ShipBehav::AttackStellars] {
                assert_eq!(
                    mission_opening(goal, behav),
                    Reply::Comm(CHANNEL_OPEN),
                    "{goal:?} {behav:?}"
                );
            }
        }
    }

    #[test]
    fn an_untalkative_mission_ship_opens_the_channel() {
        for goal in GOALS {
            for behav in BEHAVS {
                for attitude in ATTITUDES {
                    assert_eq!(
                        opening(false, attitude, Some(MissionHail { goal, behav }), None),
                        Reply::Comm(CHANNEL_OPEN),
                        "{goal:?} {behav:?} {attitude:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_named_reply_is_said_in_the_variant_then_the_ship_name_and_a_full_stop() {
        let catalog = strings("");
        assert_eq!(
            Reply::Named(GLAD_TO_SEE_YOU).say(0, &catalog, "Pathfinder"),
            "c41Pathfinder."
        );
        assert_eq!(
            Reply::Named(GLAD_TO_SEE_YOU).say(4, &catalog, "Pathfinder"),
            "c45Pathfinder."
        );
        assert_eq!(
            Reply::Named(GLAD_TO_SEE_YOU).say(2, &catalog, ""),
            "c43.",
            "a ship never named"
        );
    }

    #[test]
    fn a_named_reply_missing_its_string_is_the_name_and_a_full_stop() {
        struct Nothing;
        impl CommCatalog for Nothing {
            fn string_list(&self, _id: i16) -> Vec<String> {
                Vec::new()
            }
        }
        assert_eq!(
            Reply::Named(GLAD_TO_SEE_YOU).say(1, &Nothing, "Pathfinder"),
            "Pathfinder."
        );
    }

    #[test]
    fn only_a_named_reply_says_the_ship_name() {
        let catalog = strings("Fine.");
        assert_eq!(Reply::Comm(8).say(0, &catalog, "Pathfinder"), "c41");
        assert_eq!(
            Reply::Line {
                list: 7000,
                index: 1
            }
            .say(0, &catalog, "Pathfinder"),
            "Fine."
        );
    }

    #[test]
    fn the_groups_are_the_originals() {
        assert_eq!(GLAD_TO_SEE_YOU, 8);
        assert_eq!(Reply::locate(GLAD_TO_SEE_YOU, 0), (3000, 41));
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
                ESCORT_OPENING,
                JUST_AN_ESCORT,
                RELEASED,
                CANT_DO_THAT,
            ],
            [
                0, 1, 2, 9, 12, 13, 14, 16, 17, 18, 19, 20, 23, 24, 28, 29, 30, 31, 4, 22, 38, 25
            ]
        );
        assert_eq!(Reply::locate(CANT_DO_THAT, 0), (3000, 126));
        assert_eq!(Reply::locate(ESCORT_OPENING, 0), (3000, 21));
        assert_eq!(Reply::locate(RELEASED, 0), (3001, 1));
        assert_eq!(Reply::locate(RELEASED, 4), (3001, 5));
        assert_eq!(Reply::locate(JUST_AN_ESCORT, 3), (3000, 114));
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
