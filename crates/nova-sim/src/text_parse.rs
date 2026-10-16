//! The conditional text: `_ParseTextDescription` (@0x14bdd), which the
//! original runs on every `dësc` it loads, on a person's hail quote and on
//! the specific advice a ship gives, before the tags are filled.
//!
//! **The original.** A state machine copies the text into a buffer and
//! copies the result back (@0x14bdd-0x14f2f); the rank scan
//! ([`RankTagGovts::parsed`]) then runs on the result (@0x14f34). Outside a
//! conditional every character is copied but `{`, which opens one. After
//! the `{`:
//!
//! - `G` (either case) picks the first string for a male pilot, the second
//!   for a female one (@0x14c79-0x14cd2).
//! - `P` and `B` (either case) start a number; its first non-digit ends it
//!   and is consumed (@0x14d39-0x14dfc). `Pnnn` picks the first string when
//!   the game counts as paid for, unregistered for fewer than nnn days
//!   ([`PilotFacts::paid`], @0x14d70-0x14d8c); `Bnnn` when control bit nnn
//!   is set, never past 9999 (@0x14d9b-0x14db7). No number reads 0.
//! - `!` negates (@0x14d2d), as [`RuleKey::TextNegation`] says how far.
//! - Any other character is dropped, as [`RuleKey::StrayTextBrace`] says.
//!
//! The chosen string is copied from its opening `"` to its closing one; a
//! `\` is dropped and makes the next `"` part of the string (@0x14e1c,
//! @0x14e6c-0x14e8f). After the first string, the text up to the next `}`
//! is skipped, so a `}` inside the second string ends the conditional
//! there. When the second string is chosen, the first is skipped, then a
//! `}` ends the conditional with nothing copied and a `"` opens the second.
//! A number ended by `"` opens its first string at once. A text that ends
//! inside a conditional just ends.
//!
//! [`RuleKey::TextNegation`]: crate::RuleKey::TextNegation
//! [`RuleKey::StrayTextBrace`]: crate::RuleKey::StrayTextBrace

use crate::control::{Bit, PilotFacts};
use crate::pilot::Gender;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::text_tags::RankTagGovts;

/// The disputed rules the conditional text follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ParseRules {
    /// How far a `!` reaches ([`RuleKey::TextNegation`]): by the engine
    /// every later conditional of the text, by the Bible its own.
    pub negation: RuleSource,
    /// What a `{` with no test does ([`RuleKey::StrayTextBrace`]): by the
    /// engine it drops the text up to the next test letter; by the other
    /// reading it is copied as written.
    pub stray_brace: RuleSource,
}

impl ParseRules {
    /// The rules `rulebook` chooses.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            negation: rulebook.source_for(RuleKey::TextNegation),
            stray_brace: rulebook.source_for(RuleKey::StrayTextBrace),
        }
    }
}

/// What a number after the `{` tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Test {
    /// `P`: whether the game counts as paid for.
    Paid,
    /// `B`: a control bit.
    Bit,
}

/// Where the state machine is (the original's state number in brackets).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// Copying (0).
    Copy,
    /// After a `{`, before its test (1).
    Open,
    /// Reading a test's number (2).
    Number(Test, u64),
    /// Waiting for the chosen string's `"` (3).
    AwaitChosen,
    /// Copying the chosen string (4).
    Chosen,
    /// Waiting for the first string's `"`, to skip it (5).
    AwaitSkipped,
    /// Skipping the first string (6).
    Skipped,
    /// After the skipped first string: `}` ends, `"` opens the second (7).
    AwaitSecond,
    /// After the chosen string: skipping to `}` (8).
    SkipRest,
}

/// `text` with its conditional text resolved for the pilot `facts` read,
/// as `_ParseTextDescription` resolves it (see the module docs).
#[must_use]
pub fn conditional_text(
    text: &str,
    facts: &(impl PilotFacts + ?Sized),
    rules: ParseRules,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut state = State::Copy;
    let mut negate = false;
    let mut escape = false;
    // What a stray brace's other reading copies back: the `{` and the
    // `!`s read since, and the negation before it.
    let mut opened = String::new();
    let mut negate_before = false;
    for c in text.chars() {
        state = match state {
            State::Copy if c == '{' => {
                opened = String::from('{');
                negate_before = negate;
                if rules.negation == RuleSource::Bible {
                    negate = false;
                }
                State::Open
            }
            State::Copy => {
                out.push(c);
                State::Copy
            }
            State::Open => match c.to_ascii_lowercase() {
                'g' => pick((facts.gender() == Gender::Male) != negate, false),
                'p' => State::Number(Test::Paid, 0),
                'b' => State::Number(Test::Bit, 0),
                '!' => {
                    negate = true;
                    opened.push(c);
                    State::Open
                }
                _ if rules.stray_brace == RuleSource::Bible => {
                    out.push_str(&opened);
                    out.push(c);
                    negate = negate_before;
                    State::Copy
                }
                _ => State::Open,
            },
            State::Number(kind, n) => match c.to_digit(10) {
                Some(digit) => {
                    State::Number(kind, n.saturating_mul(10).saturating_add(u64::from(digit)))
                }
                None => pick(holds(kind, n, facts) != negate, c == '"'),
            },
            State::AwaitChosen if c == '"' => State::Chosen,
            State::AwaitSkipped if c == '"' => State::Skipped,
            State::AwaitChosen | State::AwaitSkipped => state,
            State::Chosen | State::Skipped if c == '\\' => {
                escape = true;
                state
            }
            State::Chosen if c == '"' && !escape => State::SkipRest,
            State::Skipped if c == '"' && !escape => State::AwaitSecond,
            State::Chosen | State::Skipped => {
                if state == State::Chosen {
                    out.push(c);
                }
                escape = false;
                state
            }
            State::AwaitSecond => match c {
                '}' => State::Copy,
                '"' => State::Chosen,
                _ => State::AwaitSecond,
            },
            State::SkipRest if c == '}' => State::Copy,
            State::SkipRest => State::SkipRest,
        };
    }
    out
}

/// Whether `test` of number `n` holds for `facts`: `paid(n)`, the days
/// saturating; or bit `n`, never past 9999.
fn holds(test: Test, n: u64, facts: &(impl PilotFacts + ?Sized)) -> bool {
    match test {
        Test::Paid => facts.paid(u16::try_from(n).unwrap_or(u16::MAX)),
        Test::Bit => u16::try_from(n)
            .ok()
            .and_then(Bit::new)
            .is_some_and(|bit| facts.bit(bit)),
    }
}

/// The state after a test decided `first`: its first string, or the
/// second, `quoted` when the character that ended the test opens it.
fn pick(first: bool, quoted: bool) -> State {
    match (first, quoted) {
        (true, true) => State::Chosen,
        (true, false) => State::AwaitChosen,
        (false, true) => State::Skipped,
        (false, false) => State::AwaitSkipped,
    }
}

/// `text` parsed as `_ParseTextDescription` parses it: its conditional
/// text resolved ([`conditional_text`]), then the rank governments it
/// names scanned from the result over `remembered`
/// ([`RankTagGovts::parsed`]), so a `<PRKnnn>` in a string not chosen is
/// not remembered.
#[must_use]
pub fn parse_text_description(
    text: &str,
    facts: &(impl PilotFacts + ?Sized),
    rules: ParseRules,
    remembered: RankTagGovts,
) -> (String, RankTagGovts) {
    let conditioned = conditional_text(text, facts, rules);
    let govts = remembered.parsed(&conditioned);
    (conditioned, govts)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::GovtId;
    use crate::catalog::{OutfitId, SystemId};

    /// A pilot of `gender` with `bits` set, paid for unless unregistered
    /// for `unregistered` days or more; it records the days it is asked
    /// about.
    #[derive(Debug)]
    struct FakeFacts {
        bits: BTreeSet<u16>,
        gender: Gender,
        unregistered: u16,
        asked: RefCell<Vec<u16>>,
    }

    impl FakeFacts {
        fn new(gender: Gender) -> Self {
            Self {
                bits: BTreeSet::new(),
                gender,
                unregistered: 0,
                asked: RefCell::new(Vec::new()),
            }
        }

        fn with_bit(mut self, bit: u16) -> Self {
            self.bits.insert(bit);
            self
        }
    }

    impl PilotFacts for FakeFacts {
        fn bit(&self, bit: Bit) -> bool {
            self.bits.contains(&bit.get())
        }

        fn gender(&self) -> Gender {
            self.gender
        }

        fn paid(&self, days: u16) -> bool {
            self.asked.borrow_mut().push(days);
            self.unregistered < days
        }

        fn has_outfit(&self, _outfit: OutfitId) -> bool {
            false
        }

        fn explored(&self, _system: SystemId) -> bool {
            false
        }
    }

    fn male() -> FakeFacts {
        FakeFacts::new(Gender::Male)
    }

    fn female() -> FakeFacts {
        FakeFacts::new(Gender::Female)
    }

    fn engine(text: &str, facts: &FakeFacts) -> String {
        conditional_text(text, facts, ParseRules::default())
    }

    fn bible() -> ParseRules {
        ParseRules {
            negation: RuleSource::Bible,
            stray_brace: RuleSource::Bible,
        }
    }

    #[test]
    fn plain_text_is_copied_as_it_is() {
        assert_eq!(engine("Hello, <PN>.", &male()), "Hello, <PN>.");
        assert_eq!(engine("", &male()), "");
    }

    #[test]
    fn gender_picks_the_first_string_for_a_male_pilot() {
        let text = "the {G \"man\" \"woman\"} who";
        assert_eq!(engine(text, &male()), "the man who");
        assert_eq!(engine(text, &female()), "the woman who");
        assert_eq!(
            engine("{g \"he\" \"she\"}", &female()),
            "she",
            "either case"
        );
    }

    #[test]
    fn a_bit_test_reads_the_bit_and_never_past_9999() {
        let text = "{b5 \"set\" \"clear\"}";
        assert_eq!(engine(text, &male().with_bit(5)), "set");
        assert_eq!(engine(text, &male()), "clear");
        assert_eq!(
            engine("{B9999 \"set\" \"clear\"}", &male().with_bit(9999)),
            "set"
        );
        assert_eq!(engine("{b10000 \"set\" \"clear\"}", &male()), "clear");
        assert_eq!(
            engine("{b99999999999999999999999 \"set\" \"clear\"}", &male()),
            "clear",
            "a huge number saturates past 9999"
        );
        assert_eq!(
            engine("{b \"set\" \"clear\"}", &male().with_bit(0)),
            "set",
            "no number reads 0"
        );
    }

    #[test]
    fn a_paid_test_asks_whether_the_game_counts_as_paid_for() {
        let paid = male();
        assert_eq!(engine("{P30 \"paid\" \"unpaid\"}", &paid), "paid");
        assert_eq!(*paid.asked.borrow(), [30]);
        let unpaid = FakeFacts {
            unregistered: 40,
            ..male()
        };
        assert_eq!(engine("{p30 \"paid\" \"unpaid\"}", &unpaid), "unpaid");
        let none = male();
        assert_eq!(
            engine("{P \"a\" \"b\"}", &none),
            "b",
            "no day is fewer than 0"
        );
        assert_eq!(*none.asked.borrow(), [0], "no number asks 0");
        let huge = male();
        let _ = engine("{P70000 \"a\" \"b\"}", &huge);
        assert_eq!(*huge.asked.borrow(), [u16::MAX], "the days saturate");
    }

    #[test]
    fn a_number_ended_by_a_quote_opens_its_string_at_once() {
        let text = "Requires: {P30\"License\" \"License, REGISTER\"}";
        assert_eq!(engine(text, &male()), "Requires: License");
        let unpaid = FakeFacts {
            unregistered: 40,
            ..male()
        };
        assert_eq!(engine(text, &unpaid), "Requires: License, REGISTER");
    }

    #[test]
    fn a_missing_second_string_reads_nothing() {
        assert_eq!(engine("a{b1 \"x\"}b", &male()), "ab");
        assert_eq!(engine("a{b1 \"x\"}b", &male().with_bit(1)), "axb");
    }

    #[test]
    fn a_backslash_makes_a_quote_part_of_the_string() {
        let text = "{G \"say \\\"hi\\\"\" \"say \\\"bye\\\"\"} now";
        assert_eq!(engine(text, &male()), "say \"hi\" now");
        assert_eq!(engine(text, &female()), "say \"bye\" now");
        assert_eq!(
            engine("{G \"a\\nb\" \"c\"}", &male()),
            "anb",
            "the backslash is dropped"
        );
        assert_eq!(
            engine("{G \"a\\\\\"b\" \"c\"}", &male()),
            "a\"b",
            "a second backslash too"
        );
    }

    #[test]
    fn a_closing_brace_outside_a_conditional_is_kept() {
        assert_eq!(engine("a } b", &male()), "a } b");
    }

    #[test]
    fn a_brace_inside_the_second_string_ends_the_conditional_after_the_first() {
        assert_eq!(engine("{G \"a\" \"b}c\"}d", &male()), "ac\"}d");
    }

    #[test]
    fn a_text_ending_inside_a_conditional_is_cut() {
        assert_eq!(engine("x {G \"abc", &male()), "x abc");
        assert_eq!(engine("x {b12", &male()), "x ");
        assert_eq!(engine("x {G \"a\" \"b", &female()), "x b");
    }

    #[test]
    fn negation_flips_the_test() {
        assert_eq!(engine("{!G \"a\" \"b\"}", &male()), "b");
        assert_eq!(engine("{!G \"a\" \"b\"}", &female()), "a");
        assert_eq!(engine("{!b3 \"a\" \"b\"}", &male()), "a");
        assert_eq!(engine("{!P30 \"a\" \"b\"}", &male()), "b");
    }

    #[test]
    fn by_the_engine_a_negation_flips_every_later_conditional() {
        let text = "{!G \"a\" \"b\"} {G \"c\" \"d\"} {b1 \"e\" \"f\"}";
        assert_eq!(engine(text, &male()), "b d e");
        assert_eq!(conditional_text(text, &male(), bible()), "b c f");
    }

    #[test]
    fn by_the_engine_a_stray_brace_drops_text_up_to_the_next_test() {
        let text = "{x hello {G \"m\" \"f\"} end";
        assert_eq!(engine(text, &male()), "m end");
        assert_eq!(
            engine("{x big {G \"m\" \"f\"}", &male()),
            "f",
            "the b of big starts a bit test: bit 0, clear"
        );
    }

    #[test]
    fn by_the_other_reading_a_stray_brace_is_copied_as_written() {
        let text = "{x hello {G \"m\" \"f\"} end";
        assert_eq!(conditional_text(text, &male(), bible()), "{x hello m end");
        assert_eq!(
            conditional_text("{!x} {G \"m\" \"f\"}", &male(), bible()),
            "{!x} m",
            "its negation goes with it"
        );
        let engine_negation = ParseRules {
            stray_brace: RuleSource::Bible,
            ..ParseRules::default()
        };
        assert_eq!(
            conditional_text("{!x} {G \"m\" \"f\"}", &male(), engine_negation),
            "{!x} m",
            "even when negation would reach on"
        );
    }

    #[test]
    fn the_rules_come_from_the_rulebook() {
        assert_eq!(
            ParseRules::from_rulebook(&Rulebook::default()),
            ParseRules::default()
        );
        let rulebook = Rulebook::default()
            .with_override(RuleKey::TextNegation, RuleSource::Bible)
            .with_override(RuleKey::StrayTextBrace, RuleSource::Bible);
        assert_eq!(ParseRules::from_rulebook(&rulebook), bible());
        let negation = Rulebook::default().with_override(RuleKey::TextNegation, RuleSource::Bible);
        assert_eq!(
            ParseRules::from_rulebook(&negation),
            ParseRules {
                negation: RuleSource::Bible,
                stray_brace: RuleSource::Engine,
            }
        );
    }

    #[test]
    fn the_rank_scan_reads_only_the_chosen_string() {
        let text = "{b1 \"<PRK129>!\" \"<SRK130>!\"}";
        let (parsed, govts) =
            parse_text_description(text, &male(), ParseRules::default(), RankTagGovts::NONE);
        assert_eq!(parsed, "<SRK130>!");
        assert_eq!(
            govts,
            RankTagGovts {
                prk: None,
                srk: Some(GovtId(130)),
            }
        );
        let remembered = RankTagGovts {
            prk: Some(GovtId(140)),
            srk: None,
        };
        let (_, govts) =
            parse_text_description(text, &male().with_bit(1), ParseRules::default(), remembered);
        assert_eq!(
            govts,
            RankTagGovts {
                prk: Some(GovtId(129)),
                srk: None,
            }
        );
    }
}
