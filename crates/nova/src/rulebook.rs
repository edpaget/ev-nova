//! Which rules follow the original engine and which the Nova Bible: the
//! [`Rulebook`] the settings file chooses.
//!
//! The settings file (the one that keeps the sound settings; see
//! [`config`](crate::config)) holds the choice in two top-level fields:
//!
//! ```json
//! {
//!   "rules": "bible",
//!   "rule_overrides": { "crime_gains": "engine" }
//! }
//! ```
//!
//! - [`RULES`], `"engine"` or `"bible"`: the source every disputed rule
//!   follows. Without it, the engine's.
//! - [`RULE_OVERRIDES`], an object: for any rule named by its
//!   [`RuleKey::key`], the source it follows whatever `"rules"` says.
//!
//! The rules and their keys are listed once, in
//! [`nova_sim::rulebook`]; a rule added there is read here with no
//! change. None of this is part of the pilot save.

use nova_audio::SettingsStore;
use nova_sim::{RuleKey, RuleSource, Rulebook};
use serde_json::{Map, Value};

/// The settings file's field naming the source every rule follows.
pub const RULES: &str = "rules";
/// The settings file's field holding each rule's own source.
pub const RULE_OVERRIDES: &str = "rule_overrides";

/// The rulebook the settings saved in `store` choose, and the warnings to
/// print.
///
/// With no store, nothing saved, or settings that cannot be read or are
/// not a JSON object, the rulebook is the default (every rule the
/// engine's), silently: the sound settings' keeper warns about settings
/// it cannot use. A source other than `"engine"` or `"bible"` is the
/// engine's, with a warning naming its key; a `"rule_overrides"` that is
/// not an object, and an override naming no rule, are ignored with a
/// warning.
pub fn game_rulebook(store: Option<&mut (dyn SettingsStore + '_)>) -> (Rulebook, Vec<String>) {
    let Some(store) = store else {
        return (Rulebook::default(), Vec::new());
    };
    let Ok(Some(text)) = store.read() else {
        return (Rulebook::default(), Vec::new());
    };
    let Ok(Value::Object(settings)) = serde_json::from_str::<Value>(&text) else {
        return (Rulebook::default(), Vec::new());
    };
    let location = store.location();
    let mut warnings = Vec::new();
    let mut source = |value: &Value, key: &str| {
        value
            .as_str()
            .and_then(RuleSource::from_name)
            .unwrap_or_else(|| {
                warnings.push(format!(
                    "nova: the saved {key} in {location} is not \"engine\" or \"bible\"; \
                 following the engine's"
                ));
                RuleSource::Engine
            })
    };
    let mut rulebook = settings.get(RULES).map_or_else(Rulebook::default, |value| {
        Rulebook::new(source(value, &format!("\"{RULES}\"")))
    });
    let mut unknown = Vec::new();
    match settings.get(RULE_OVERRIDES) {
        None => {}
        Some(Value::Object(overrides)) => {
            for (key, value) in sorted(overrides) {
                match RuleKey::from_key(key) {
                    Some(rule) => {
                        let named = format!("\"{RULE_OVERRIDES}\".\"{key}\"");
                        rulebook = rulebook.with_override(rule, source(value, &named));
                    }
                    None => unknown.push(format!(
                        "nova: the saved \"{RULE_OVERRIDES}\" in {location} name no rule \
                         \"{key}\"; ignoring it"
                    )),
                }
            }
        }
        Some(_) => unknown.push(format!(
            "nova: the saved \"{RULE_OVERRIDES}\" in {location} is not an object; ignoring it"
        )),
    }
    warnings.extend(unknown);
    (rulebook, warnings)
}

/// `map`'s entries in key order, whatever order the map keeps.
fn sorted(map: &Map<String, Value>) -> Vec<(&String, &Value)> {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by_key(|&(key, _)| key);
    entries
}

#[cfg(test)]
mod tests {
    use nova_audio::recording::MemorySettings;

    use super::*;

    fn rulebook_of(text: &str) -> (Rulebook, Vec<String>) {
        game_rulebook(Some(&mut MemorySettings::holding(text)))
    }

    fn engine() -> Rulebook {
        Rulebook::new(RuleSource::Engine)
    }

    fn bible() -> Rulebook {
        Rulebook::new(RuleSource::Bible)
    }

    #[test]
    fn without_a_choice_every_rule_is_the_engines() {
        assert_eq!(game_rulebook(None), (engine(), vec![]), "no store");
        assert_eq!(
            game_rulebook(Some(&mut MemorySettings::new())),
            (engine(), vec![]),
            "nothing saved"
        );
        assert_eq!(rulebook_of("{}"), (engine(), vec![]));
        assert_eq!(rulebook_of(r#"{"sound": false}"#), (engine(), vec![]));
        assert_eq!(
            rulebook_of(r#"{"rules": "engine", "rule_overrides": {}}"#),
            (engine(), vec![])
        );
        assert_eq!((RULES, RULE_OVERRIDES), ("rules", "rule_overrides"));
    }

    #[test]
    fn settings_that_cannot_be_used_are_the_engines_leaving_the_warning_to_the_sound() {
        for text in ["not json", "", "[1, 2]", r#""bible""#] {
            assert_eq!(rulebook_of(text), (engine(), vec![]), "{text}");
        }
        let mut store = MemorySettings::holding(r#"{"rules": "bible"}"#);
        store.fail_reads(true);
        assert_eq!(game_rulebook(Some(&mut store)), (engine(), vec![]));
    }

    #[test]
    fn the_rules_choose_the_source_of_every_rule() {
        let (rulebook, warnings) = rulebook_of(r#"{"sound": false, "rules": "bible"}"#);
        assert_eq!((rulebook, warnings), (bible(), vec![]));
        assert_eq!(rulebook.source_for(RuleKey::CrimeGains), RuleSource::Bible);
    }

    #[test]
    fn an_override_beats_the_rules_both_ways() {
        let (rulebook, warnings) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"crime_gains": "engine"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            bible().with_override(RuleKey::CrimeGains, RuleSource::Engine)
        );
        assert_eq!(rulebook.source_for(RuleKey::CrimeGains), RuleSource::Engine);
        let (rulebook, warnings) = rulebook_of(r#"{"rule_overrides": {"crime_gains": "bible"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(rulebook.default_source(), RuleSource::Engine);
        assert_eq!(rulebook.source_for(RuleKey::CrimeGains), RuleSource::Bible);
    }

    #[test]
    fn the_hailing_rules_are_overridden_by_their_keys() {
        let text = r#"{"rule_overrides": {"quiet_hails": "bible", "long_advice": "bible"}}"#;
        let (rulebook, warnings) = rulebook_of(text);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine()
                .with_override(RuleKey::QuietHails, RuleSource::Bible)
                .with_override(RuleKey::LongAdvice, RuleSource::Bible)
        );
        let text = r#"{"rules": "bible", "rule_overrides": {"long_advice": "engine"}}"#;
        let (rulebook, _) = rulebook_of(text);
        assert_eq!(
            [RuleKey::QuietHails, RuleKey::LongAdvice].map(|key| rulebook.source_for(key)),
            [RuleSource::Bible, RuleSource::Engine]
        );
    }

    #[test]
    fn the_escort_rules_are_overridden_by_their_keys() {
        let text = r#"{"rule_overrides": {"escort_ai": "bible", "escort_orders": "bible"}}"#;
        let (rulebook, warnings) = rulebook_of(text);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine()
                .with_override(RuleKey::EscortAi, RuleSource::Bible)
                .with_override(RuleKey::EscortOrders, RuleSource::Bible)
        );
        let text = r#"{"rules": "bible", "rule_overrides": {"escort_orders": "engine"}}"#;
        let (rulebook, _) = rulebook_of(text);
        assert_eq!(
            [RuleKey::EscortAi, RuleKey::EscortOrders].map(|key| rulebook.source_for(key)),
            [RuleSource::Bible, RuleSource::Engine]
        );
    }

    #[test]
    fn the_fighter_rules_are_overridden_by_their_keys() {
        let text = r#"{"rule_overrides": {"fighter_launch": "bible", "fighter_recall": "bible"}}"#;
        let (rulebook, warnings) = rulebook_of(text);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine()
                .with_override(RuleKey::FighterLaunch, RuleSource::Bible)
                .with_override(RuleKey::FighterRecall, RuleSource::Bible)
        );
        let text = r#"{"rules": "bible", "rule_overrides": {"fighter_recall": "engine"}}"#;
        let (rulebook, _) = rulebook_of(text);
        assert_eq!(
            [RuleKey::FighterLaunch, RuleKey::FighterRecall].map(|key| rulebook.source_for(key)),
            [RuleSource::Bible, RuleSource::Engine]
        );
    }

    #[test]
    fn the_joining_and_grant_rules_are_overridden_by_their_keys() {
        let text = r#"{"rule_overrides": {"person_join": "bible", "grant_count": "bible", "grant_max": "bible"}}"#;
        let (rulebook, warnings) = rulebook_of(text);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine()
                .with_override(RuleKey::PersonJoin, RuleSource::Bible)
                .with_override(RuleKey::GrantCount, RuleSource::Bible)
                .with_override(RuleKey::GrantMax, RuleSource::Bible)
        );
    }

    #[test]
    fn the_persons_rules_are_overridden_by_their_keys() {
        let text = r#"{"rule_overrides": {"person_odds": "bible", "system_persons": "bible", "link_syst_slip": "bible", "shield_mod": "bible", "person_coward": "bible", "person_credits": "bible", "comm_quote": "bible"}}"#;
        let (rulebook, warnings) = rulebook_of(text);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine()
                .with_override(RuleKey::PersonOdds, RuleSource::Bible)
                .with_override(RuleKey::SystemPersons, RuleSource::Bible)
                .with_override(RuleKey::LinkSystSlip, RuleSource::Bible)
                .with_override(RuleKey::ShieldMod, RuleSource::Bible)
                .with_override(RuleKey::PersonCoward, RuleSource::Bible)
                .with_override(RuleKey::PersonCredits, RuleSource::Bible)
                .with_override(RuleKey::CommQuote, RuleSource::Bible)
        );
        let (rulebook, _) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"comm_quote": "engine"}}"#);
        assert_eq!(rulebook.source_for(RuleKey::CommQuote), RuleSource::Engine);
        assert_eq!(rulebook.source_for(RuleKey::PersonOdds), RuleSource::Bible);
    }

    #[test]
    fn the_hiring_rules_are_overridden_by_their_keys() {
        let text = r#"{"rule_overrides": {"hire_require": "bible", "take_off_pay": "bible", "hire_fee": "bible", "escort_wage": "bible"}}"#;
        let (rulebook, warnings) = rulebook_of(text);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine()
                .with_override(RuleKey::HireRequire, RuleSource::Bible)
                .with_override(RuleKey::TakeOffPay, RuleSource::Bible)
                .with_override(RuleKey::HireFee, RuleSource::Bible)
                .with_override(RuleKey::EscortWage, RuleSource::Bible)
        );
        assert_eq!(
            nova_sim::NovaHire::from_rulebook(&rulebook).hire_fee,
            RuleSource::Bible,
            "the hire_fee override reaches Nova's hire terms"
        );
        let text = r#"{"rules": "bible", "rule_overrides": {"take_off_pay": "engine"}}"#;
        let (rulebook, _) = rulebook_of(text);
        assert_eq!(
            [
                RuleKey::HireRequire,
                RuleKey::TakeOffPay,
                RuleKey::HireFee,
                RuleKey::EscortWage
            ]
            .map(|key| rulebook.source_for(key)),
            [
                RuleSource::Bible,
                RuleSource::Engine,
                RuleSource::Bible,
                RuleSource::Bible
            ]
        );
    }

    #[test]
    fn the_boarding_and_piracy_rules_are_overridden_by_their_keys() {
        let text = r#"{"rule_overrides": {"empty_booty": "bible", "crewless_capture": "bible", "piracy_police": "bible"}}"#;
        let (rulebook, warnings) = rulebook_of(text);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine()
                .with_override(RuleKey::EmptyBooty, RuleSource::Bible)
                .with_override(RuleKey::CrewlessCapture, RuleSource::Bible)
                .with_override(RuleKey::PiracyPolice, RuleSource::Bible)
        );
        let text = r#"{"rules": "bible", "rule_overrides": {"piracy_police": "engine"}}"#;
        let (rulebook, _) = rulebook_of(text);
        assert_eq!(
            [
                RuleKey::CrimeGains,
                RuleKey::EmptyBooty,
                RuleKey::CrewlessCapture,
                RuleKey::PiracyPolice
            ]
            .map(|key| rulebook.source_for(key)),
            [
                RuleSource::Bible,
                RuleSource::Bible,
                RuleSource::Bible,
                RuleSource::Engine
            ]
        );
    }

    #[test]
    fn the_shop_rules_are_overridden_by_their_keys() {
        let (rulebook, warnings) = rulebook_of(r#"{"rule_overrides": {"buy_random": "bible"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine().with_override(RuleKey::BuyRandom, RuleSource::Bible)
        );
        let (rulebook, _) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"buy_random": "engine"}}"#);
        assert_eq!(rulebook.source_for(RuleKey::BuyRandom), RuleSource::Engine);
        assert_eq!(rulebook.source_for(RuleKey::CrimeGains), RuleSource::Bible);
    }

    #[test]
    fn the_junk_flags_rule_is_overridden_by_its_key() {
        let (rulebook, warnings) = rulebook_of(r#"{"rule_overrides": {"junk_flags": "bible"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine().with_override(RuleKey::JunkFlags, RuleSource::Bible)
        );
        let (rulebook, _) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"junk_flags": "engine"}}"#);
        assert_eq!(rulebook.source_for(RuleKey::JunkFlags), RuleSource::Engine);
        assert_eq!(rulebook.source_for(RuleKey::BuyRandom), RuleSource::Bible);
    }

    #[test]
    fn the_event_price_rule_is_overridden_by_its_key() {
        let (rulebook, warnings) = rulebook_of(r#"{"rule_overrides": {"event_price": "bible"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine().with_override(RuleKey::EventPrice, RuleSource::Bible)
        );
        let (rulebook, _) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"event_price": "engine"}}"#);
        assert_eq!(rulebook.source_for(RuleKey::EventPrice), RuleSource::Engine);
        assert_eq!(rulebook.source_for(RuleKey::RaisedMax), RuleSource::Bible);
    }

    #[test]
    fn the_purchase_cargo_rule_is_overridden_by_its_key() {
        let (rulebook, warnings) =
            rulebook_of(r#"{"rule_overrides": {"purchase_cargo": "bible"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine().with_override(RuleKey::PurchaseCargo, RuleSource::Bible)
        );
        let (rulebook, _) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"purchase_cargo": "engine"}}"#);
        assert_eq!(
            rulebook.source_for(RuleKey::PurchaseCargo),
            RuleSource::Engine
        );
        assert_eq!(rulebook.source_for(RuleKey::EventPrice), RuleSource::Bible);
    }

    #[test]
    fn the_junk_trade_rule_is_overridden_by_its_key() {
        let (rulebook, warnings) = rulebook_of(r#"{"rule_overrides": {"junk_trade": "bible"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine().with_override(RuleKey::JunkTrade, RuleSource::Bible)
        );
        let (rulebook, _) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"junk_trade": "engine"}}"#);
        assert_eq!(rulebook.source_for(RuleKey::JunkTrade), RuleSource::Engine);
        assert_eq!(rulebook.source_for(RuleKey::JunkPrice), RuleSource::Bible);
    }

    #[test]
    fn the_trade_lot_rule_is_overridden_by_its_key() {
        let (rulebook, warnings) = rulebook_of(r#"{"rule_overrides": {"trade_lot": "bible"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine().with_override(RuleKey::TradeLot, RuleSource::Bible)
        );
        let (rulebook, _) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"trade_lot": "engine"}}"#);
        assert_eq!(rulebook.source_for(RuleKey::TradeLot), RuleSource::Engine);
        assert_eq!(rulebook.source_for(RuleKey::JunkTrade), RuleSource::Bible);
    }

    #[test]
    fn the_trade_quotient_rule_is_overridden_by_its_key() {
        let (rulebook, warnings) =
            rulebook_of(r#"{"rule_overrides": {"trade_quotient": "bible"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine().with_override(RuleKey::TradeQuotient, RuleSource::Bible)
        );
        let (rulebook, _) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"trade_quotient": "engine"}}"#);
        assert_eq!(
            rulebook.source_for(RuleKey::TradeQuotient),
            RuleSource::Engine
        );
        assert_eq!(rulebook.source_for(RuleKey::TradeLot), RuleSource::Bible);
    }

    #[test]
    fn the_outfit_limit_rule_is_overridden_by_its_key() {
        let (rulebook, warnings) = rulebook_of(r#"{"rule_overrides": {"outfit_limit": "bible"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine().with_override(RuleKey::OutfitLimit, RuleSource::Bible)
        );
        let (rulebook, _) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"outfit_limit": "engine"}}"#);
        assert_eq!(
            rulebook.source_for(RuleKey::OutfitLimit),
            RuleSource::Engine
        );
        assert_eq!(rulebook.source_for(RuleKey::JunkTrade), RuleSource::Bible);
    }

    #[test]
    fn the_junk_price_rule_is_overridden_by_its_key() {
        let (rulebook, warnings) = rulebook_of(r#"{"rule_overrides": {"junk_price": "bible"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine().with_override(RuleKey::JunkPrice, RuleSource::Bible)
        );
        let (rulebook, _) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"junk_price": "engine"}}"#);
        assert_eq!(rulebook.source_for(RuleKey::JunkPrice), RuleSource::Engine);
        assert_eq!(
            rulebook.source_for(RuleKey::PurchaseCargo),
            RuleSource::Bible
        );
    }

    #[test]
    fn the_raised_max_rule_is_overridden_by_its_key() {
        let (rulebook, warnings) = rulebook_of(r#"{"rule_overrides": {"raised_max": "bible"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine().with_override(RuleKey::RaisedMax, RuleSource::Bible)
        );
        let (rulebook, _) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"raised_max": "engine"}}"#);
        assert_eq!(rulebook.source_for(RuleKey::RaisedMax), RuleSource::Engine);
        assert_eq!(
            rulebook.source_for(RuleKey::LauncherSale),
            RuleSource::Bible
        );
    }

    #[test]
    fn the_launcher_sale_rule_is_overridden_by_its_key() {
        let (rulebook, warnings) = rulebook_of(r#"{"rule_overrides": {"launcher_sale": "bible"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            engine().with_override(RuleKey::LauncherSale, RuleSource::Bible)
        );
        let (rulebook, _) =
            rulebook_of(r#"{"rules": "bible", "rule_overrides": {"launcher_sale": "engine"}}"#);
        assert_eq!(
            rulebook.source_for(RuleKey::LauncherSale),
            RuleSource::Engine
        );
        assert_eq!(rulebook.source_for(RuleKey::JunkFlags), RuleSource::Bible);
    }

    #[test]
    fn a_source_that_is_neither_is_the_engines_with_a_warning_naming_its_key() {
        for value in [r#""Bible""#, r#""both""#, "1", "null", r#"{"bible": true}"#] {
            let text =
                format!(r#"{{"rules": {value}, "rule_overrides": {{"crime_gains": "bible"}}}}"#);
            assert_eq!(
                rulebook_of(&text),
                (
                    engine().with_override(RuleKey::CrimeGains, RuleSource::Bible),
                    vec![
                        "nova: the saved \"rules\" in memory is not \"engine\" or \"bible\"; \
                         following the engine's"
                            .to_owned()
                    ]
                ),
                "{text}"
            );
            let text =
                format!(r#"{{"rules": "bible", "rule_overrides": {{"crime_gains": {value}}}}}"#);
            assert_eq!(
                rulebook_of(&text),
                (
                    bible().with_override(RuleKey::CrimeGains, RuleSource::Engine),
                    vec![
                        "nova: the saved \"rule_overrides\".\"crime_gains\" in memory is not \
                         \"engine\" or \"bible\"; following the engine's"
                            .to_owned()
                    ]
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn overrides_naming_no_rule_or_not_an_object_are_ignored_with_a_warning() {
        let text = r#"{"rules": "bible", "rule_overrides": {"crime": "engine", "crime_gains": "engine", "": "bible"}}"#;
        assert_eq!(
            rulebook_of(text),
            (
                bible().with_override(RuleKey::CrimeGains, RuleSource::Engine),
                vec![
                    "nova: the saved \"rule_overrides\" in memory name no rule \"\"; ignoring it"
                        .to_owned(),
                    "nova: the saved \"rule_overrides\" in memory name no rule \"crime\"; \
                     ignoring it"
                        .to_owned(),
                ]
            )
        );
        for value in [r#""bible""#, "[]", "null", "3"] {
            let text = format!(r#"{{"rules": "bible", "rule_overrides": {value}}}"#);
            assert_eq!(
                rulebook_of(&text),
                (
                    bible(),
                    vec![
                        "nova: the saved \"rule_overrides\" in memory is not an object; \
                         ignoring it"
                            .to_owned()
                    ]
                ),
                "{text}"
            );
        }
    }
}
