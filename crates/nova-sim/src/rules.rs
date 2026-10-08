//! The rules core: an ordered list of checks over any context, each check
//! passing or refusing with a reason, and each one described by a
//! [`Descriptor`] tools can list without running it.

use crate::chance::Chance;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// What every context a rule reads offers the core: the rule set its
/// disputed rules choose their reading by.
pub trait Context {
    /// The rule set in force.
    fn rulebook(&self) -> &Rulebook;
}

/// What a rule is, told without running it: its name, the original's
/// address or routine it reproduces, and the disputed rule it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    name: &'static str,
    reproduces: &'static str,
    reads: Option<RuleKey>,
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
            },
            how: CheckHow::Disputed { key, engine, other },
        }
    }

    /// What the check is.
    #[must_use]
    pub fn about(&self) -> &Descriptor {
        &self.about
    }
}

impl<C: Context, R> Check<C, R> {
    /// Runs the check on `facts`, in the reading the rule set chooses.
    fn decide(&self, facts: &C) -> Result<(), R> {
        match self.how {
            CheckHow::Plain(f) => f(facts),
            CheckHow::Disputed { key, engine, other } => {
                reading(facts.rulebook(), key, engine, other)(facts)
            }
        }
    }
}

/// An ordered list of checks over a context `C`, refusing with reasons
/// `R`.
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
}

impl<C, R> Checks<C, R> {
    /// Each check's descriptor, in order. No check is run.
    pub fn descriptors(&self) -> impl Iterator<Item = &Descriptor> {
        self.0.iter().map(Check::about)
    }
}

impl<C: Context, R> Checks<C, R> {
    /// The first refusal, in the list's order, or `Ok` when every check
    /// passes. The checks after a refusal are not run.
    pub fn first_refusal(&self, facts: &C, _chance: &mut dyn Chance) -> Result<(), R> {
        self.0.iter().try_for_each(|check| check.decide(facts))
    }

    /// Every check that refuses, in the list's order, each with its
    /// descriptor. The checks that pass are left out.
    pub fn all_refusals(&self, facts: &C) -> Vec<(&Descriptor, Verdict<R>)> {
        self.0
            .iter()
            .filter_map(|check| match check.decide(facts) {
                Ok(()) => None,
                Err(reason) => Some((check.about(), Verdict::Refuses(reason))),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;

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
    }

    /// A toy mission offer: what it asks of the pilot.
    struct MissionOffer {
        min_combat: u16,
    }

    /// What the toy offer reads about the pilot.
    struct PilotRecord {
        combat: u16,
    }

    /// The stellar the toy offer would be made at.
    struct Stellar {
        has_bar: bool,
    }

    /// A non-shop context: a mission offer, the pilot, the stellar and
    /// the rule set.
    struct Offer {
        mission: MissionOffer,
        pilot: PilotRecord,
        stellar: Stellar,
        rules: Rulebook,
    }

    impl Context for Offer {
        fn rulebook(&self) -> &Rulebook {
            &self.rules
        }
    }

    /// An offer asking a combat rating of 10 of a pilot rated 10, at a
    /// stellar with a bar, under the default rule set.
    fn offer() -> Offer {
        Offer {
            mission: MissionOffer { min_combat: 10 },
            pilot: PilotRecord { combat: 10 },
            stellar: Stellar { has_bar: true },
            rules: Rulebook::default(),
        }
    }

    /// The toy offer's checks: a bar, then the combat rating (disputed:
    /// the engine refuses below the offer's minimum, the other reading at
    /// or below it; `CrimeGains` is only a convenient key).
    fn offer_checks() -> Checks<Offer, OfferRefusal> {
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
        ]
        .into()
    }

    #[test]
    fn a_mission_offer_refuses_without_a_bar() {
        let facts = Offer {
            stellar: Stellar { has_bar: false },
            pilot: PilotRecord { combat: 0 },
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
                checks.first_refusal(&facts, &mut NeverFires),
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
        assert_eq!(checks.first_refusal(&engine, &mut NeverFires), Ok(()));
        let bible = Offer {
            rules: Rulebook::new(RuleSource::Bible)
                .with_override(RuleKey::TradeDebt, RuleSource::Engine),
            ..offer()
        };
        assert_eq!(
            checks.first_refusal(&bible, &mut NeverFires),
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
            [("bar", None), ("rating", Some(RuleKey::CrimeGains))]
        );
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
        ]
        .into()
    }

    #[test]
    fn descriptors_list_without_running_any_rule() {
        let checks = panicking();
        let listed: Vec<_> = checks
            .descriptors()
            .map(|about| (about.name(), about.reproduces(), about.reads()))
            .collect();
        assert_eq!(
            listed,
            [
                ("first", "@0x1000", None),
                ("second", "a routine", None),
                ("third", "@0x2000", Some(RuleKey::TradeDebt)),
            ]
        );
    }
}
