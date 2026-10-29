//! Which rules follow the original engine and which the Nova Bible: the
//! [`Rulebook`] the settings file chooses.
//!
//! The settings file (the one that keeps the sound settings; see
//! [`config`](crate::config)) holds the choice in three top-level fields:
//!
//! ```json
//! {
//!   "rules": "bible",
//!   "rule_overrides": { "crime_gains": "engine" },
//!   "rule_values": { "thrust_divisor": 3000 }
//! }
//! ```
//!
//! - [`RULES`], `"engine"` or `"bible"`: the source every disputed rule
//!   follows. Without it, each rule's own default: the engine's, but for
//!   `cron_zero_duration_holdoff` and `cron_post_holdoff`, which default
//!   to the fix (see [`nova_sim::rulebook`]).
//! - [`RULE_OVERRIDES`], an object: for any rule named by its
//!   [`RuleKey::key`], the source it follows whatever `"rules"` says.
//! - [`RULE_VALUES`], an object: for any numeric rule named by its
//!   [`RuleValue::key`], the whole number from 1 to `u32::MAX` it takes in
//!   place of its default (see [`nova_sim::rule_values`]). `"rules"`
//!   doesn't move these.
//!
//! The rules and their keys are listed once, in
//! [`nova_sim::rulebook`] and [`nova_sim::rule_values`]; a rule added
//! there is read here with no change. None of this is part of the pilot
//! save.

use core::num::NonZeroU32;
use nova_audio::SettingsStore;

use nova_data::graphics::EightBitReading;
use nova_sim::{RuleKey, RuleSource, RuleValue, Rulebook};
use serde_json::{Map, Value};

/// The settings file's field naming the source every rule follows.
pub const RULES: &str = "rules";
/// The settings file's field holding each rule's own source.
pub const RULE_OVERRIDES: &str = "rule_overrides";
/// The settings file's field holding each numeric rule's number.
pub const RULE_VALUES: &str = "rule_values";

/// The rulebook the settings saved in `store` choose, and the warnings to
/// print.
///
/// With no store, nothing saved, or settings that cannot be read or are
/// not a JSON object, the rulebook is the default (each rule its own
/// default), silently: the sound settings' keeper warns about settings
/// it cannot use. A source other than `"engine"` or `"bible"` is ignored,
/// as if it were absent, with a warning naming its key, so a typo never
/// brings back the engine's reading of a rule that defaults to the other;
/// a `"rule_overrides"` that is not an object, and an override naming no
/// rule, are ignored with a warning too. So are a `"rule_values"` that is
/// not an object, a value naming no numeric rule, and a number that is
/// not a JSON whole number from 1 to `u32::MAX`, which leaves that rule
/// at its default.
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
        let source = value.as_str().and_then(RuleSource::from_name);
        if source.is_none() {
            warnings.push(format!(
                "nova: the saved {key} in {location} is not \"engine\" or \"bible\"; \
                 ignoring it"
            ));
        }
        source
    };
    let mut rulebook = settings
        .get(RULES)
        .and_then(|value| source(value, &format!("\"{RULES}\"")))
        .map_or_else(Rulebook::default, Rulebook::new);
    let mut unknown = Vec::new();
    match settings.get(RULE_OVERRIDES) {
        None => {}
        Some(Value::Object(overrides)) => {
            for (key, value) in sorted(overrides) {
                match RuleKey::from_key(key) {
                    Some(rule) => {
                        let named = format!("\"{RULE_OVERRIDES}\".\"{key}\"");
                        if let Some(source) = source(value, &named) {
                            rulebook = rulebook.with_override(rule, source);
                        }
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
    match settings.get(RULE_VALUES) {
        None => {}
        Some(Value::Object(values)) => {
            for (key, value) in sorted(values) {
                match RuleValue::from_key(key) {
                    Some(rule) => match whole_number(value) {
                        Some(number) => rulebook = rulebook.with_value(rule, number),
                        None => warnings.push(format!(
                            "nova: the saved \"{RULE_VALUES}\".\"{key}\" in {location} is not \
                             a whole number from 1 to {}; ignoring it",
                            u32::MAX
                        )),
                    },
                    None => unknown.push(format!(
                        "nova: the saved \"{RULE_VALUES}\" in {location} name no value \
                         \"{key}\"; ignoring it"
                    )),
                }
            }
        }
        Some(_) => unknown.push(format!(
            "nova: the saved \"{RULE_VALUES}\" in {location} is not an object; ignoring it"
        )),
    }
    warnings.extend(unknown);
    (rulebook, warnings)
}

/// `value` as a numeric rule's number: a JSON whole number from 1 to
/// `u32::MAX`, written without a fraction.
fn whole_number(value: &Value) -> Option<NonZeroU32> {
    value
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .and_then(NonZeroU32::new)
}

/// `map`'s entries in key order, whatever order the map keeps.
fn sorted(map: &Map<String, Value>) -> Vec<(&String, &Value)> {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by_key(|&(key, _)| key);
    entries
}

/// How `rulebook` has the game's images decode an 8-bit `rlëD`
/// ([`RuleKey::EightBitSheet`]): as the original's 16-bit world draws it,
/// or through the Mac's standard palette.
#[must_use]
pub fn eight_bit_reading(rulebook: &Rulebook) -> EightBitReading {
    match rulebook.source_for(RuleKey::EightBitSheet) {
        RuleSource::Engine => EightBitReading::AsDrawn,
        RuleSource::Bible => EightBitReading::SystemPalette,
    }
}

#[cfg(test)]
mod tests {
    use nova_audio::recording::MemorySettings;

    use super::*;

    #[test]
    fn the_eight_bit_reading_follows_the_rulebook() {
        assert_eq!(
            eight_bit_reading(&Rulebook::default()),
            EightBitReading::AsDrawn
        );
        assert_eq!(
            eight_bit_reading(
                &Rulebook::default().with_override(RuleKey::EightBitSheet, RuleSource::Bible)
            ),
            EightBitReading::SystemPalette
        );
        assert_eq!(
            eight_bit_reading(
                &Rulebook::new(RuleSource::Bible)
                    .with_override(RuleKey::EightBitSheet, RuleSource::Engine)
            ),
            EightBitReading::AsDrawn
        );
    }

    fn rulebook_of(text: &str) -> (Rulebook, Vec<String>) {
        game_rulebook(Some(&mut MemorySettings::holding(text)))
    }

    fn engine() -> Rulebook {
        Rulebook::new(RuleSource::Engine)
    }

    fn by_default() -> Rulebook {
        Rulebook::default()
    }

    fn bible() -> Rulebook {
        Rulebook::new(RuleSource::Bible)
    }

    #[test]
    fn without_a_choice_each_rule_follows_its_default() {
        assert_eq!(game_rulebook(None), (by_default(), vec![]), "no store");
        assert_eq!(
            game_rulebook(Some(&mut MemorySettings::new())),
            (by_default(), vec![]),
            "nothing saved"
        );
        assert_eq!(rulebook_of("{}"), (by_default(), vec![]));
        assert_eq!(rulebook_of(r#"{"sound": false}"#), (by_default(), vec![]));
        assert_eq!((RULES, RULE_OVERRIDES), ("rules", "rule_overrides"));
    }

    #[test]
    fn an_explicit_engine_rules_sets_every_rule_the_cron_holdoffs_too() {
        let (rulebook, warnings) = rulebook_of(r#"{"rules": "engine", "rule_overrides": {}}"#);
        assert_eq!((rulebook, warnings), (engine(), vec![]));
        assert_eq!(
            rulebook.source_for(RuleKey::CronPostHoldoff),
            RuleSource::Engine
        );
        assert_ne!(rulebook, by_default());
    }

    #[test]
    fn an_engine_override_turns_the_dialog_focus_ring_off() {
        let (rulebook, warnings) =
            rulebook_of(r#"{"rule_overrides": {"dialog_focus_ring": "engine"}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook.source_for(RuleKey::DialogFocusRing),
            RuleSource::Engine
        );
        assert_eq!(
            by_default().source_for(RuleKey::DialogFocusRing),
            RuleSource::Bible
        );
    }

    #[test]
    fn an_override_sets_one_cron_holdoff_rule() {
        let (rulebook, warnings) =
            rulebook_of(r#"{"rule_overrides": {"cron_post_holdoff": "engine"}}"#);
        assert_eq!(
            (rulebook, warnings),
            (
                by_default().with_override(RuleKey::CronPostHoldoff, RuleSource::Engine),
                vec![]
            )
        );
        assert_eq!(
            rulebook.source_for(RuleKey::CronZeroDurationHoldoff),
            RuleSource::Bible
        );
        assert_eq!(rulebook.source_for(RuleKey::CrimeGains), RuleSource::Engine);
    }

    #[test]
    fn settings_that_cannot_be_used_are_the_defaults_leaving_the_warning_to_the_sound() {
        for text in ["not json", "", "[1, 2]", r#""bible""#] {
            assert_eq!(rulebook_of(text), (by_default(), vec![]), "{text}");
        }
        let mut store = MemorySettings::holding(r#"{"rules": "bible"}"#);
        store.fail_reads(true);
        assert_eq!(game_rulebook(Some(&mut store)), (by_default(), vec![]));
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
        assert_eq!(rulebook.global_source(), None);
        assert_eq!(rulebook.source_for(RuleKey::CrimeGains), RuleSource::Bible);
    }

    #[test]
    fn every_rule_is_overridden_by_its_settings_key() {
        for key in RuleKey::ALL {
            let name = key.key();
            let text = format!(r#"{{"rule_overrides": {{"{name}": "bible"}}}}"#);
            assert_eq!(
                rulebook_of(&text),
                (by_default().with_override(key, RuleSource::Bible), vec![]),
                "{name}"
            );
            let text = format!(r#"{{"rules": "bible", "rule_overrides": {{"{name}": "engine"}}}}"#);
            assert_eq!(
                rulebook_of(&text),
                (bible().with_override(key, RuleSource::Engine), vec![]),
                "{name}"
            );
        }
        let overrides: Map<String, Value> = RuleKey::ALL
            .into_iter()
            .map(|key| (key.key().to_owned(), Value::from("bible")))
            .collect();
        let text = serde_json::json!({ "rule_overrides": overrides }).to_string();
        let every = RuleKey::ALL.into_iter().fold(by_default(), |book, key| {
            book.with_override(key, RuleSource::Bible)
        });
        assert_eq!(rulebook_of(&text), (every, vec![]));
    }

    #[test]
    fn a_source_that_is_neither_is_ignored_with_a_warning_naming_its_key() {
        for value in [r#""Bible""#, r#""both""#, "1", "null", r#"{"bible": true}"#] {
            let text =
                format!(r#"{{"rules": {value}, "rule_overrides": {{"crime_gains": "bible"}}}}"#);
            assert_eq!(
                rulebook_of(&text),
                (
                    by_default().with_override(RuleKey::CrimeGains, RuleSource::Bible),
                    vec![
                        "nova: the saved \"rules\" in memory is not \"engine\" or \"bible\"; \
                         ignoring it"
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
                    bible(),
                    vec![
                        "nova: the saved \"rule_overrides\".\"crime_gains\" in memory is not \
                         \"engine\" or \"bible\"; ignoring it"
                            .to_owned()
                    ]
                ),
                "{text}"
            );
        }
    }

    fn n(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    #[test]
    fn every_value_is_set_by_its_settings_key() {
        assert_eq!(RULE_VALUES, "rule_values");
        for value in RuleValue::ALL {
            let name = value.key();
            for number in [1, 3000, 7919, u32::MAX] {
                let text = format!(r#"{{"rule_values": {{"{name}": {number}}}}}"#);
                assert_eq!(
                    rulebook_of(&text),
                    (by_default().with_value(value, n(number)), vec![]),
                    "{text}"
                );
            }
        }
        let (rulebook, warnings) = rulebook_of(r#"{"rule_values": {"thrust_divisor": 3000}}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(rulebook.value_for(RuleValue::ThrustDivisor), n(3000));
    }

    #[test]
    fn the_rules_leave_every_value_at_its_default_and_values_keep_the_sources() {
        let (rulebook, warnings) = rulebook_of(r#"{"rules": "bible"}"#);
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(rulebook.value_for(RuleValue::ThrustDivisor), n(5000));
        let (rulebook, warnings) = rulebook_of(
            r#"{"rules": "bible", "rule_overrides": {"crime_gains": "engine"}, "rule_values": {"thrust_divisor": 3000}}"#,
        );
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            rulebook,
            bible()
                .with_override(RuleKey::CrimeGains, RuleSource::Engine)
                .with_value(RuleValue::ThrustDivisor, n(3000))
        );
    }

    #[test]
    fn a_value_that_is_not_a_whole_number_from_1_is_ignored_with_a_warning_naming_its_key() {
        for value in [
            "0",
            "-1",
            "3000.5",
            "3000.0",
            r#""3000""#,
            "null",
            "4294967296",
            "[3000]",
        ] {
            let text = format!(
                r#"{{"rules": "bible", "rule_values": {{"thrust_divisor": {value}}}, "rule_overrides": {{"crime_gains": "engine"}}}}"#
            );
            assert_eq!(
                rulebook_of(&text),
                (
                    bible().with_override(RuleKey::CrimeGains, RuleSource::Engine),
                    vec![
                        "nova: the saved \"rule_values\".\"thrust_divisor\" in memory is not a \
                         whole number from 1 to 4294967295; ignoring it"
                            .to_owned()
                    ]
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn values_naming_no_value_or_not_an_object_are_ignored_with_a_warning() {
        let text = r#"{"rule_values": {"thrust": 3000, "thrust_divisor": 3000, "crime_gains": 1}}"#;
        assert_eq!(
            rulebook_of(text),
            (
                by_default().with_value(RuleValue::ThrustDivisor, n(3000)),
                vec![
                    "nova: the saved \"rule_values\" in memory name no value \"crime_gains\"; \
                     ignoring it"
                        .to_owned(),
                    "nova: the saved \"rule_values\" in memory name no value \"thrust\"; \
                     ignoring it"
                        .to_owned(),
                ]
            )
        );
        for value in [r#""3000""#, "[]", "null", "3"] {
            let text = format!(r#"{{"rules": "bible", "rule_values": {value}}}"#);
            assert_eq!(
                rulebook_of(&text),
                (
                    bible(),
                    vec![
                        "nova: the saved \"rule_values\" in memory is not an object; ignoring it"
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
