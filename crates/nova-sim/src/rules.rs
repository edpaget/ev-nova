//! The rules core: the decisions a shop or an offer makes, written as
//! rules over a context of facts, generic over the context so that it
//! knows nothing of sessions, rows or shops.
//!
//! # Rules
//!
//! A rule is one decision with a [`Descriptor`]: its name, the original's
//! address or routine it reproduces, the disputed rule it reads (if any)
//! and whether it draws on the chance. Its constructor sets the last two,
//! so a descriptor cannot say otherwise than the rule does. Tools list a
//! list's descriptors ([`Checks::descriptors`]) without running any rule.
//!
//! There are two kinds, kept apart:
//!
//! - A [`Check`] answers "can I?": it passes, or refuses with a reason
//!   `R`. A [`Checks`] list holds checks in order. Gameplay asks its
//!   [`first_refusal`](Checks::first_refusal); tools ask its
//!   [`all_refusals`](Checks::all_refusals), every reason with its
//!   descriptor ("why isn't this offered here?").
//! - A [`Value`] answers "what does it show?": it gives data `T`, such as a
//!   price, a count, a maximum or a pay. No list takes one, and nothing
//!   turns one into a check or back.
//!
//! Rules are `fn` pointers over `&C`, and lists are built by functions
//! rather than kept in `static`s, so that a context may borrow (a
//! `fn(&C<'static>)` would take no shorter-lived context).
//!
//! # Readings
//!
//! Where the Nova Bible and the original engine disagree, a rule is
//! disputed ([`Check::disputed`], [`Value::disputed`]): it holds the
//! engine's reading and the other, and runs the one the context's rule
//! set gives, by [`Rulebook::source_for`] its [`RuleKey`]. The context
//! hands the rule set over through [`Context::rulebook`], the one thing
//! the core asks of it. The choice is part of the rule, not its caller's.
//!
//! # Draws
//!
//! A drawing check ([`Check::drawing`]) draws on the [`Chance`] handed to
//! [`first_refusal`](Checks::first_refusal) beside the context, not kept
//! in it. Its draws come in the list's order, and stop at the first
//! refusal, as the original's do when a later check is never reached.
//! [`all_refusals`](Checks::all_refusals) is handed no chance, so it can
//! never draw on the game's: it reports each drawing check as
//! [`Verdict::Draws`] without running it. (A caller wanting the outcome
//! runs `first_refusal` on a chance of its own.)
//!
//! A draw that the original makes while building what a shop shows, such
//! as the outfitter's day rolls (`wares::DayRolls`),
//! belongs in the facts: the caller draws it, keeps it, and hands it in.
//! A drawing check is for a draw made in check order, such as a mission
//! offer's `Rand`.
//!
//! # State
//!
//! The core holds none. Every rule reads the context through `&C`, and
//! state that lasts a visit or a day is the caller's, borrowed through
//! the context; writing it back is the caller's job too. So the same
//! context gives the same result each time it is evaluated: with any
//! chance for a list that does not draw, always for `all_refusals`, and
//! with chances scripted alike for a list that draws.
//!
//! # Out of scope
//!
//! Event-driven dispatch ("on this event, if this test holds, run this set
//! expression, once") belongs to the story runtime. At most, a
//! dispatcher's guard is a rule.

use crate::chance::Chance;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// What every context a rule reads offers the core: the rule set its
/// disputed rules choose their reading by.
pub trait Context {
    /// The rule set in force.
    fn rulebook(&self) -> &Rulebook;
}

/// What a rule is, told without running it: its name, the original's
/// address or routine it reproduces, the disputed rule it reads, and
/// whether it draws on the chance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    name: &'static str,
    reproduces: &'static str,
    reads: Option<RuleKey>,
    draws: bool,
}

impl Descriptor {
    /// The rule's name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The original's address or routine the rule reproduces, as free
    /// text.
    #[must_use]
    pub fn reproduces(&self) -> &'static str {
        self.reproduces
    }

    /// The disputed rule whose reading the rule follows, if any: set by
    /// the constructor that built a disputed rule, and by no other.
    #[must_use]
    pub fn reads(&self) -> Option<RuleKey> {
        self.reads
    }

    /// Whether the rule draws on the chance: set by the constructor that
    /// built a drawing rule, and by no other.
    #[must_use]
    pub fn draws(&self) -> bool {
        self.draws
    }
}

/// `engine` or `other`, as the rule set says `key` follows the engine or
/// the Bible.
fn reading<F>(rules: &Rulebook, key: RuleKey, engine: F, other: F) -> F {
    match rules.source_for(key) {
        RuleSource::Engine => engine,
        RuleSource::Bible => other,
    }
}

/// One "can I?" rule over a context `C`: it passes, or refuses with a
/// reason `R`.
pub struct Check<C, R> {
    about: Descriptor,
    how: CheckHow<C, R>,
}

/// How a check decides.
enum CheckHow<C, R> {
    /// One reading, whatever the rule set.
    Plain(fn(&C) -> Result<(), R>),
    /// The engine's reading and the other, chosen by the rule set's
    /// source for `key`.
    Disputed {
        key: RuleKey,
        engine: fn(&C) -> Result<(), R>,
        other: fn(&C) -> Result<(), R>,
    },
    /// One reading, drawing on the chance it is handed.
    Draws(fn(&C, &mut dyn Chance) -> Result<(), R>),
}

impl<C, R> Check<C, R> {
    /// A check with one reading, `f`.
    #[must_use]
    pub const fn plain(
        name: &'static str,
        reproduces: &'static str,
        f: fn(&C) -> Result<(), R>,
    ) -> Self {
        Self {
            about: Descriptor {
                name,
                reproduces,
                reads: None,
                draws: false,
            },
            how: CheckHow::Plain(f),
        }
    }

    /// A check with the engine's reading, `engine`, and the other,
    /// `other`, following the rule set's source for `key`.
    #[must_use]
    pub const fn disputed(
        name: &'static str,
        reproduces: &'static str,
        key: RuleKey,
        engine: fn(&C) -> Result<(), R>,
        other: fn(&C) -> Result<(), R>,
    ) -> Self {
        Self {
            about: Descriptor {
                name,
                reproduces,
                reads: Some(key),
                draws: false,
            },
            how: CheckHow::Disputed { key, engine, other },
        }
    }

    /// A check with one reading, `f`, that draws on the chance it is
    /// handed. [`Checks::all_refusals`] reports it without running it.
    #[must_use]
    pub const fn drawing(
        name: &'static str,
        reproduces: &'static str,
        f: fn(&C, &mut dyn Chance) -> Result<(), R>,
    ) -> Self {
        Self {
            about: Descriptor {
                name,
                reproduces,
                reads: None,
                draws: true,
            },
            how: CheckHow::Draws(f),
        }
    }

    /// What the check is.
    #[must_use]
    pub fn about(&self) -> &Descriptor {
        &self.about
    }
}

impl<C: Context, R> Check<C, R> {
    /// Runs the check on `facts`, in the reading the rule set chooses,
    /// drawing on `chance` if it is a drawing check.
    fn run(&self, facts: &C, chance: &mut dyn Chance) -> Result<(), R> {
        match self.how {
            CheckHow::Plain(f) => f(facts),
            CheckHow::Disputed { key, engine, other } => {
                reading(facts.rulebook(), key, engine, other)(facts)
            }
            CheckHow::Draws(f) => f(facts, chance),
        }
    }

    /// What the check makes of `facts` without drawing: a drawing check
    /// is reported as such and not run, and a check that passes gives
    /// `None`.
    fn verdict(&self, facts: &C) -> Option<Verdict<R>> {
        let decided = match self.how {
            CheckHow::Plain(f) => f(facts),
            CheckHow::Disputed { key, engine, other } => {
                reading(facts.rulebook(), key, engine, other)(facts)
            }
            CheckHow::Draws(_) => return Some(Verdict::Draws),
        };
        decided.err().map(Verdict::Refuses)
    }
}

/// An ordered list of checks over a context `C`, refusing with reasons
/// `R`.
///
/// It holds checks only: "can I?" is a checks list, and "what does it
/// show?" is a [`Value`], which no list takes. A list builds from checks:
///
/// ```
/// use nova_sim::rulebook::Rulebook;
/// use nova_sim::rules::{Check, Checks, Context};
///
/// struct Facts(Rulebook);
/// impl Context for Facts {
///     fn rulebook(&self) -> &Rulebook {
///         &self.0
///     }
/// }
///
/// let can: Check<Facts, ()> = Check::plain("can", "toy", |_| Ok(()));
/// let _: Checks<Facts, ()> = [can].into();
/// ```
///
/// and not from a value:
///
/// ```compile_fail
/// use nova_sim::rulebook::Rulebook;
/// use nova_sim::rules::{Checks, Context, Value};
///
/// struct Facts(Rulebook);
/// impl Context for Facts {
///     fn rulebook(&self) -> &Rulebook {
///         &self.0
///     }
/// }
///
/// let words: Value<Facts, String> = Value::plain("words", "toy", |_| String::new());
/// let _: Checks<Facts, ()> = [words].into();
/// ```
pub struct Checks<C, R>(Vec<Check<C, R>>);

impl<C, R, const N: usize> From<[Check<C, R>; N]> for Checks<C, R> {
    fn from(checks: [Check<C, R>; N]) -> Self {
        Self(checks.into())
    }
}

impl<C, R> FromIterator<Check<C, R>> for Checks<C, R> {
    fn from_iter<I: IntoIterator<Item = Check<C, R>>>(checks: I) -> Self {
        Self(checks.into_iter().collect())
    }
}

/// What a check made of the facts, as [`Checks::all_refusals`] reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict<R> {
    /// It refused, with this reason.
    Refuses(R),
    /// It draws on the chance, so it was not run.
    Draws,
}

impl<C, R> Checks<C, R> {
    /// Each check's descriptor, in order. No check is run.
    pub fn descriptors(&self) -> impl Iterator<Item = &Descriptor> {
        self.0.iter().map(Check::about)
    }
}

impl<C: Context, R> Checks<C, R> {
    /// The first refusal, in the list's order, or `Ok` when every check
    /// passes. The checks after a refusal are not run, so the drawing
    /// checks draw on `chance` in the list's order, and stop drawing at
    /// the first refusal.
    pub fn first_refusal(&self, facts: &C, chance: &mut dyn Chance) -> Result<(), R> {
        self.0.iter().try_for_each(|check| check.run(facts, chance))
    }

    /// Every check that refuses, in the list's order, each with its
    /// descriptor. The checks that pass are left out. It is handed no
    /// chance, so it draws on none: each drawing check is reported as
    /// [`Verdict::Draws`] and not run.
    pub fn all_refusals(&self, facts: &C) -> Vec<(&Descriptor, Verdict<R>)> {
        self.0
            .iter()
            .filter_map(|check| Some((check.about(), check.verdict(facts)?)))
            .collect()
    }
}

/// One "what does it show?" rule over a context `C`: it gives data `T`,
/// such as a price, a count, a maximum or a pay. It is no check, and no
/// [`Checks`] list takes it.
pub struct Value<C, T> {
    about: Descriptor,
    how: ValueHow<C, T>,
}

/// How a value is worked out.
enum ValueHow<C, T> {
    /// One reading, whatever the rule set.
    Plain(fn(&C) -> T),
    /// The engine's reading and the other, chosen by the rule set's
    /// source for `key`.
    Disputed {
        key: RuleKey,
        engine: fn(&C) -> T,
        other: fn(&C) -> T,
    },
}

impl<C, T> Value<C, T> {
    /// A value with one reading, `f`.
    #[must_use]
    pub const fn plain(name: &'static str, reproduces: &'static str, f: fn(&C) -> T) -> Self {
        Self {
            about: Descriptor {
                name,
                reproduces,
                reads: None,
                draws: false,
            },
            how: ValueHow::Plain(f),
        }
    }

    /// A value with the engine's reading, `engine`, and the other,
    /// `other`, following the rule set's source for `key`.
    #[must_use]
    pub const fn disputed(
        name: &'static str,
        reproduces: &'static str,
        key: RuleKey,
        engine: fn(&C) -> T,
        other: fn(&C) -> T,
    ) -> Self {
        Self {
            about: Descriptor {
                name,
                reproduces,
                reads: Some(key),
                draws: false,
            },
            how: ValueHow::Disputed { key, engine, other },
        }
    }

    /// What the value is.
    #[must_use]
    pub fn about(&self) -> &Descriptor {
        &self.about
    }
}

impl<C: Context, T> Value<C, T> {
    /// The value for `facts`, in the reading the rule set chooses.
    pub fn of(&self, facts: &C) -> T {
        match self.how {
            ValueHow::Plain(f) => f(facts),
            ValueHow::Disputed { key, engine, other } => {
                reading(facts.rulebook(), key, engine, other)(facts)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    use crate::chance::NeverFires;
    use crate::control::{ControlBits, FreshPilot, Gate, PilotFacts, Test, TestExpr};
    use crate::testkit::Scripted;

    /// The outfitter's Sell flag's local copy of `OutfitFlags::CANNOT_SELL`,
    /// so this module depends on no shop.
    const CANNOT_SELL: u16 = 0x0008;

    /// Why the toy Sell flag is off.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum SellRefusal {
        NoneOwned,
        CannotSell,
    }

    /// One small shop rule's facts: how many of an outfit the player owns,
    /// and the outfit's flags.
    struct SellFacts {
        owned: u16,
        flags: u16,
        rules: Rulebook,
    }

    impl Context for SellFacts {
        fn rulebook(&self) -> &Rulebook {
            &self.rules
        }
    }

    impl SellFacts {
        fn new(owned: u16, flags: u16) -> Self {
            Self {
                owned,
                flags,
                rules: Rulebook::default(),
            }
        }
    }

    /// The outfitter's Sell flag (@0x5c563-0x5c595): owned, and not
    /// flagged unsellable.
    fn sell_flag() -> Checks<SellFacts, SellRefusal> {
        [
            Check::plain("owned", "@0x5c563", |facts: &SellFacts| {
                if facts.owned == 0 {
                    Err(SellRefusal::NoneOwned)
                } else {
                    Ok(())
                }
            }),
            Check::plain("sellable", "@0x5c595", |facts: &SellFacts| {
                if facts.flags & CANNOT_SELL == 0 {
                    Ok(())
                } else {
                    Err(SellRefusal::CannotSell)
                }
            }),
        ]
        .into()
    }

    #[test]
    fn sell_flag_refuses_none_owned_before_cannot_sell() {
        let facts = SellFacts::new(0, CANNOT_SELL);
        assert_eq!(
            sell_flag().first_refusal(&facts, &mut NeverFires),
            Err(SellRefusal::NoneOwned)
        );
    }

    #[test]
    fn sell_flag_refuses_cannot_sell_when_owned() {
        let facts = SellFacts::new(1, CANNOT_SELL | 0x0010);
        assert_eq!(
            sell_flag().first_refusal(&facts, &mut NeverFires),
            Err(SellRefusal::CannotSell)
        );
    }

    #[test]
    fn sell_flag_passes_when_owned_and_sellable() {
        let facts = SellFacts::new(2, !CANNOT_SELL);
        assert_eq!(sell_flag().first_refusal(&facts, &mut NeverFires), Ok(()));
    }

    #[test]
    fn all_refusals_lists_every_reason_with_its_descriptor() {
        let checks = sell_flag();
        let refusals = checks.all_refusals(&SellFacts::new(0, CANNOT_SELL));
        let listed: Vec<_> = refusals
            .iter()
            .map(|(about, verdict)| (about.name(), about.reproduces(), *verdict))
            .collect();
        assert_eq!(
            listed,
            [
                (
                    "owned",
                    "@0x5c563",
                    Verdict::Refuses(SellRefusal::NoneOwned)
                ),
                (
                    "sellable",
                    "@0x5c595",
                    Verdict::Refuses(SellRefusal::CannotSell)
                ),
            ]
        );
    }

    #[test]
    fn all_refusals_omits_passing_checks() {
        let checks = sell_flag();
        let refusals = checks.all_refusals(&SellFacts::new(3, CANNOT_SELL));
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].0.name(), "sellable");
        assert_eq!(refusals[0].1, Verdict::Refuses(SellRefusal::CannotSell));
        assert!(checks.all_refusals(&SellFacts::new(3, 0)).is_empty());
    }

    #[test]
    fn a_list_can_be_collected_from_its_checks() {
        let checks: Checks<SellFacts, SellRefusal> = sell_flag()
            .0
            .into_iter()
            .filter(|check| check.about().name() == "sellable")
            .collect();
        let facts = SellFacts::new(0, 0);
        assert_eq!(checks.first_refusal(&facts, &mut NeverFires), Ok(()));
        assert_eq!(checks.descriptors().count(), 1);
    }

    #[test]
    fn the_same_facts_give_the_same_result_twice() {
        let checks = sell_flag();
        for facts in [
            SellFacts::new(0, CANNOT_SELL),
            SellFacts::new(1, CANNOT_SELL),
            SellFacts::new(1, 0),
        ] {
            assert_eq!(
                checks.first_refusal(&facts, &mut NeverFires),
                checks.first_refusal(&facts, &mut NeverFires)
            );
            assert_eq!(checks.all_refusals(&facts), checks.all_refusals(&facts));
        }
    }

    /// Why the toy mission offer is not offered.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum OfferRefusal {
        NoBar,
        TooGreen,
        Unavailable,
        LostTheRoll,
        LostTheSecondRoll,
    }

    /// A toy mission offer: what it asks of the pilot.
    struct MissionOffer {
        min_combat: u16,
        availability: Test,
        odds: u8,
        second_odds: u8,
    }

    /// What the toy offer reads about the pilot.
    struct PilotRecord {
        combat: u16,
        cash: i64,
    }

    /// The stellar the toy offer would be made at.
    struct Stellar {
        has_bar: bool,
    }

    /// A non-shop context: a mission offer, the pilot, the stellar, the
    /// gate the offer's control-bit test is asked through, and the rule
    /// set.
    struct Offer<'a> {
        mission: MissionOffer,
        pilot: PilotRecord,
        stellar: Stellar,
        gate: Gate<'a>,
        rules: Rulebook,
    }

    impl Context for Offer<'_> {
        fn rulebook(&self) -> &Rulebook {
            &self.rules
        }
    }

    /// A chance that fires on its first two questions.
    fn lucky() -> Scripted {
        Scripted::answering(&[true, true])
    }

    /// An offer asking a combat rating of 10 of a pilot rated 10, at a
    /// stellar with a bar, available to a fresh pilot, under the default
    /// rule set.
    fn offer() -> Offer<'static> {
        Offer {
            mission: MissionOffer {
                min_combat: 10,
                availability: Test::default(),
                odds: 40,
                second_odds: 60,
            },
            pilot: PilotRecord {
                combat: 10,
                cash: 1000,
            },
            stellar: Stellar { has_bar: true },
            gate: Gate::FRESH,
            rules: Rulebook::default(),
        }
    }

    /// The toy offer's checks: a bar, then the combat rating (disputed:
    /// the engine refuses below the offer's minimum, the other reading at
    /// or below it; `CrimeGains` is only a convenient key), then the
    /// offer's control-bit test through the gate, then two rolls on the
    /// chance, at the offer's odds and then its second odds.
    fn offer_checks<'a>() -> Checks<Offer<'a>, OfferRefusal> {
        [
            Check::plain("bar", "toy: the stellar's bar", |facts: &Offer| {
                if facts.stellar.has_bar {
                    Ok(())
                } else {
                    Err(OfferRefusal::NoBar)
                }
            }),
            Check::disputed(
                "rating",
                "toy: the offer's combat rating",
                RuleKey::CrimeGains,
                |facts: &Offer| {
                    if facts.pilot.combat < facts.mission.min_combat {
                        Err(OfferRefusal::TooGreen)
                    } else {
                        Ok(())
                    }
                },
                |facts: &Offer| {
                    if facts.pilot.combat <= facts.mission.min_combat {
                        Err(OfferRefusal::TooGreen)
                    } else {
                        Ok(())
                    }
                },
            ),
            Check::plain(
                "available",
                "toy: the offer's availability",
                |facts: &Offer| {
                    if facts.gate.allows(&facts.mission.availability) {
                        Ok(())
                    } else {
                        Err(OfferRefusal::Unavailable)
                    }
                },
            ),
            Check::drawing(
                "roll",
                "toy: the offer's Rand draw",
                |facts: &Offer, chance: &mut dyn Chance| {
                    if chance.fires(facts.mission.odds) {
                        Ok(())
                    } else {
                        Err(OfferRefusal::LostTheRoll)
                    }
                },
            ),
            Check::drawing(
                "second roll",
                "toy: a second Rand draw",
                |facts: &Offer, chance: &mut dyn Chance| {
                    if chance.fires(facts.mission.second_odds) {
                        Ok(())
                    } else {
                        Err(OfferRefusal::LostTheSecondRoll)
                    }
                },
            ),
        ]
        .into()
    }

    /// A hand-written control-bit port with a canned answer, recording
    /// each test it is asked.
    #[derive(Debug)]
    struct MockBits {
        answer: bool,
        asked: RefCell<Vec<TestExpr>>,
    }

    impl MockBits {
        fn answering(answer: bool) -> Self {
            Self {
                answer,
                asked: RefCell::default(),
            }
        }
    }

    impl ControlBits for MockBits {
        fn allows(&self, test: &TestExpr, _pilot: &dyn PilotFacts) -> bool {
            self.asked.borrow_mut().push(test.clone());
            self.answer
        }
    }

    /// The toy offer, available as `b5` says, asked through `bits`.
    fn gated(bits: &MockBits) -> Offer<'_> {
        Offer {
            mission: MissionOffer {
                availability: Test::parse("b5"),
                ..offer().mission
            },
            gate: Gate {
                control_bits: bits,
                pilot: &FreshPilot,
            },
            ..offer()
        }
    }

    #[test]
    fn a_check_reads_control_bits_through_the_gate() {
        let bits = MockBits::answering(false);
        let mut chance = lucky();
        assert_eq!(
            offer_checks().first_refusal(&gated(&bits), &mut chance),
            Err(OfferRefusal::Unavailable)
        );
        assert_eq!(
            *bits.asked.borrow(),
            [TestExpr::parse("b5").expect("parses")]
        );
        assert!(chance.asked.is_empty(), "no roll after the refusal");
    }

    #[test]
    fn a_check_passes_when_the_gate_allows() {
        let bits = MockBits::answering(true);
        assert_eq!(
            offer_checks().first_refusal(&gated(&bits), &mut lucky()),
            Ok(())
        );
        assert_eq!(bits.asked.borrow().len(), 1);
    }

    #[test]
    fn a_mission_offer_passes_when_every_check_passes() {
        let mut chance = lucky();
        let facts = offer();
        assert_eq!(offer_checks().first_refusal(&facts, &mut chance), Ok(()));
        assert_eq!(chance.asked, [40, 60]);
        assert_eq!(reward_words().of(&facts), "Pays 100 credits");
    }

    #[test]
    fn a_mission_offer_refuses_without_a_bar() {
        let facts = Offer {
            stellar: Stellar { has_bar: false },
            pilot: PilotRecord { combat: 0, cash: 0 },
            ..offer()
        };
        assert_eq!(
            offer_checks().first_refusal(&facts, &mut NeverFires),
            Err(OfferRefusal::NoBar)
        );
    }

    #[test]
    fn a_disputed_check_follows_the_rule_set() {
        let checks = offer_checks();
        for source in RuleSource::ALL {
            let facts = Offer {
                rules: Rulebook::default().with_override(RuleKey::CrimeGains, source),
                ..offer()
            };
            let expected = match source {
                RuleSource::Engine => Ok(()),
                RuleSource::Bible => Err(OfferRefusal::TooGreen),
            };
            assert_eq!(
                checks.first_refusal(&facts, &mut lucky()),
                expected,
                "{source:?}"
            );
            let all: Vec<_> = checks
                .all_refusals(&facts)
                .into_iter()
                .map(|(about, verdict)| (about.name(), verdict))
                .collect();
            let expected: Vec<_> = expected
                .err()
                .map(|reason| ("rating", Verdict::Refuses(reason)))
                .into_iter()
                .chain([("roll", Verdict::Draws), ("second roll", Verdict::Draws)])
                .collect();
            assert_eq!(all, expected, "{source:?}");
        }
    }

    #[test]
    fn a_disputed_check_ignores_other_keys_overrides() {
        let checks = offer_checks();
        let engine = Offer {
            rules: Rulebook::default().with_override(RuleKey::TradeDebt, RuleSource::Bible),
            ..offer()
        };
        assert_eq!(checks.first_refusal(&engine, &mut lucky()), Ok(()));
        let bible = Offer {
            rules: Rulebook::new(RuleSource::Bible)
                .with_override(RuleKey::TradeDebt, RuleSource::Engine),
            ..offer()
        };
        assert_eq!(
            checks.first_refusal(&bible, &mut lucky()),
            Err(OfferRefusal::TooGreen)
        );
    }

    #[test]
    fn a_disputed_rules_descriptor_names_its_key() {
        let checks = offer_checks();
        let reads: Vec<_> = checks
            .descriptors()
            .map(|about| (about.name(), about.reads()))
            .collect();
        assert_eq!(
            reads,
            [
                ("bar", None),
                ("rating", Some(RuleKey::CrimeGains)),
                ("available", None),
                ("roll", None),
                ("second roll", None),
            ]
        );
    }

    #[test]
    fn draws_happen_in_rule_order() {
        let mut chance = Scripted::answering(&[true, false]);
        assert_eq!(
            offer_checks().first_refusal(&offer(), &mut chance),
            Err(OfferRefusal::LostTheSecondRoll)
        );
        assert_eq!(chance.asked, [40, 60]);
    }

    #[test]
    fn draws_stop_at_the_first_refusal() {
        let checks = offer_checks();
        let no_bar = Offer {
            stellar: Stellar { has_bar: false },
            ..offer()
        };
        let mut chance = lucky();
        assert_eq!(
            checks.first_refusal(&no_bar, &mut chance),
            Err(OfferRefusal::NoBar)
        );
        assert!(chance.asked.is_empty());
        let mut chance = Scripted::answering(&[false, true]);
        assert_eq!(
            checks.first_refusal(&offer(), &mut chance),
            Err(OfferRefusal::LostTheRoll)
        );
        assert_eq!(chance.asked, [40]);
    }

    #[test]
    fn all_refusals_reports_a_drawing_check_without_drawing() {
        let checks = offer_checks();
        let mut chance = Scripted::answering(&[false]);
        let all: Vec<_> = checks
            .all_refusals(&offer())
            .into_iter()
            .map(|(about, verdict)| (about.name(), about.draws(), verdict))
            .collect();
        assert_eq!(
            all,
            [
                ("roll", true, Verdict::Draws),
                ("second roll", true, Verdict::Draws),
            ]
        );
        assert!(chance.asked.is_empty());
        assert_eq!(
            checks.first_refusal(&offer(), &mut chance),
            Err(OfferRefusal::LostTheRoll),
            "the first scripted answer is still the first given"
        );
    }

    #[test]
    fn a_drawing_list_is_repeatable_on_chances_scripted_alike() {
        let checks = offer_checks();
        let facts = offer();
        let mut first = Scripted::answering(&[true, false]);
        let mut second = Scripted::answering(&[true, false]);
        assert_eq!(
            checks.first_refusal(&facts, &mut first),
            checks.first_refusal(&facts, &mut second)
        );
        assert_eq!(first.asked, second.asked);
    }

    /// The toy offer's pay: a tenth of the pilot's cash by the engine, a
    /// twentieth by the other reading.
    fn pay<'a>() -> Value<Offer<'a>, i64> {
        Value::disputed(
            "pay",
            "toy: the offer's pay",
            RuleKey::CrimeGains,
            |facts: &Offer| facts.pilot.cash / 10,
            |facts: &Offer| facts.pilot.cash / 20,
        )
    }

    /// What the toy offer says it pays.
    fn reward_words<'a>() -> Value<Offer<'a>, String> {
        Value::plain("reward words", "toy: the offer's text", |facts: &Offer| {
            format!("Pays {} credits", pay().of(facts))
        })
    }

    #[test]
    fn a_value_gives_data() {
        let facts = offer();
        assert_eq!(reward_words().of(&facts), "Pays 100 credits");
        assert_eq!(pay().of(&facts), 100);
        assert_eq!(reward_words().about().name(), "reward words");
        assert_eq!(reward_words().about().reproduces(), "toy: the offer's text");
        assert_eq!(reward_words().about().reads(), None);
    }

    #[test]
    fn a_disputed_value_follows_the_rule_set() {
        for (source, expected) in [(RuleSource::Engine, 100), (RuleSource::Bible, 50)] {
            let facts = Offer {
                rules: Rulebook::default().with_override(RuleKey::CrimeGains, source),
                ..offer()
            };
            assert_eq!(pay().of(&facts), expected, "{source:?}");
        }
        let override_beats_default = Offer {
            rules: Rulebook::new(RuleSource::Bible)
                .with_override(RuleKey::CrimeGains, RuleSource::Engine),
            ..offer()
        };
        assert_eq!(pay().of(&override_beats_default), 100);
    }

    #[test]
    fn a_disputed_value_ignores_other_keys_overrides() {
        let engine = Offer {
            rules: Rulebook::default().with_override(RuleKey::TradeDebt, RuleSource::Bible),
            ..offer()
        };
        assert_eq!(pay().of(&engine), 100);
        let bible = Offer {
            rules: Rulebook::new(RuleSource::Bible)
                .with_override(RuleKey::TradeDebt, RuleSource::Engine),
            ..offer()
        };
        assert_eq!(pay().of(&bible), 50);
    }

    #[test]
    fn a_disputed_values_descriptor_names_its_key() {
        let pay = pay();
        assert_eq!(pay.about().name(), "pay");
        assert_eq!(pay.about().reproduces(), "toy: the offer's pay");
        assert_eq!(pay.about().reads(), Some(RuleKey::CrimeGains));
    }

    /// A context no facts can be built for, whose every rule panics if
    /// run.
    enum Panics {}

    fn panicking() -> Checks<Panics, ()> {
        [
            Check::plain("first", "@0x1000", |_: &Panics| panic!("first ran")),
            Check::plain("second", "a routine", |_: &Panics| panic!("second ran")),
            Check::disputed(
                "third",
                "@0x2000",
                RuleKey::TradeDebt,
                |_: &Panics| panic!("third's engine reading ran"),
                |_: &Panics| panic!("third's other reading ran"),
            ),
            Check::drawing("fourth", "@0x3000", |_: &Panics, _: &mut dyn Chance| {
                panic!("fourth ran")
            }),
        ]
        .into()
    }

    #[test]
    fn descriptors_list_without_running_any_rule() {
        let checks = panicking();
        let listed: Vec<_> = checks
            .descriptors()
            .map(|about| {
                (
                    about.name(),
                    about.reproduces(),
                    about.reads(),
                    about.draws(),
                )
            })
            .collect();
        assert_eq!(
            listed,
            [
                ("first", "@0x1000", None, false),
                ("second", "a routine", None, false),
                ("third", "@0x2000", Some(RuleKey::TradeDebt), false),
                ("fourth", "@0x3000", None, true),
            ]
        );
    }
}
