//! The developer settings: the port's own overlays on the flight screen,
//! which the original does not draw, as the settings file turns them on.
//!
//! The settings file (the one that keeps the sound settings and the
//! rules; see [`config`](crate::config)) holds them in one top-level
//! object:
//!
//! ```json
//! {
//!   "developer": { "flight_title": true, "help_line": true }
//! }
//! ```
//!
//! - [`FLIGHT_TITLE`]: the system's name and `sÿst` ID at the top left of
//!   flight.
//! - [`HELP_LINE`]: the line of keys at the bottom of flight.
//!
//! Both are off unless set to `true`. They are not [`Rulebook`] rules:
//! neither reading is the original's. None of this is part of the pilot
//! save.
//!
//! [`Rulebook`]: nova_sim::Rulebook

use nova_audio::SettingsStore;
use nova_view::flight::FlightOverlays;
use serde_json::{Map, Value};

/// The settings file's field holding the developer settings.
pub const DEVELOPER: &str = "developer";
/// The developer setting that draws the flight's title.
pub const FLIGHT_TITLE: &str = "flight_title";
/// The developer setting that draws the flight's help line.
pub const HELP_LINE: &str = "help_line";

/// The overlays the settings saved in `store` turn on, and the warnings
/// to print.
///
/// With no store, nothing saved, settings that cannot be read or are not
/// a JSON object, or no [`DEVELOPER`] object, both are off, silently: the
/// sound settings' keeper warns about settings it cannot use. A
/// [`DEVELOPER`] that is not an object, and a setting that is not a
/// bool, are off, with a warning naming its key.
#[must_use]
pub fn flight_overlays(
    store: Option<&mut (dyn SettingsStore + '_)>,
) -> (FlightOverlays, Vec<String>) {
    let Some(store) = store else {
        return (FlightOverlays::default(), Vec::new());
    };
    let Ok(Some(text)) = store.read() else {
        return (FlightOverlays::default(), Vec::new());
    };
    let Ok(Value::Object(settings)) = serde_json::from_str::<Value>(&text) else {
        return (FlightOverlays::default(), Vec::new());
    };
    let location = store.location();
    let mut warnings = Vec::new();
    let developer = match settings.get(DEVELOPER) {
        None => return (FlightOverlays::default(), warnings),
        Some(Value::Object(developer)) => developer,
        Some(_) => {
            warnings.push(format!(
                "nova: the saved \"{DEVELOPER}\" in {location} is not an object; ignoring it"
            ));
            return (FlightOverlays::default(), warnings);
        }
    };
    let mut on = |key: &str| setting(developer, key, &location, &mut warnings);
    let overlays = FlightOverlays {
        title: on(FLIGHT_TITLE),
        help: on(HELP_LINE),
    };
    (overlays, warnings)
}

/// Whether `developer` turns `key` on: a missing key is off; one that is
/// not a bool is off, with a warning.
fn setting(
    developer: &Map<String, Value>,
    key: &str,
    location: &str,
    warnings: &mut Vec<String>,
) -> bool {
    match developer.get(key) {
        None => false,
        Some(Value::Bool(on)) => *on,
        Some(_) => {
            warnings.push(format!(
                "nova: the saved \"{DEVELOPER}\".\"{key}\" in {location} is not true or false; \
                 leaving it off"
            ));
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_audio::recording::MemorySettings;

    use super::*;

    fn overlays_of(text: &str) -> (FlightOverlays, Vec<String>) {
        flight_overlays(Some(&mut MemorySettings::holding(text)))
    }

    const OFF: FlightOverlays = FlightOverlays {
        title: false,
        help: false,
    };

    #[test]
    fn without_a_setting_both_are_off() {
        assert_eq!(flight_overlays(None), (OFF, vec![]), "no store");
        assert_eq!(
            flight_overlays(Some(&mut MemorySettings::new())),
            (OFF, vec![]),
            "nothing saved"
        );
        assert_eq!(overlays_of("{}"), (OFF, vec![]));
        assert_eq!(overlays_of(r#"{"sound": false}"#), (OFF, vec![]));
        assert_eq!(overlays_of(r#"{"developer": {}}"#), (OFF, vec![]));
        assert_eq!(
            overlays_of(r#"{"developer": {"flight_title": false, "help_line": false}}"#),
            (OFF, vec![])
        );
        assert_eq!(
            (DEVELOPER, FLIGHT_TITLE, HELP_LINE),
            ("developer", "flight_title", "help_line")
        );
    }

    #[test]
    fn settings_that_cannot_be_used_leave_both_off_silently() {
        for text in ["not json", "", "[1, 2]", r#""developer""#] {
            assert_eq!(overlays_of(text), (OFF, vec![]), "{text}");
        }
        let mut store =
            MemorySettings::holding(r#"{"developer": {"flight_title": true, "help_line": true}}"#);
        store.fail_reads(true);
        assert_eq!(flight_overlays(Some(&mut store)), (OFF, vec![]));
    }

    #[test]
    fn each_setting_turns_its_overlay_on() {
        assert_eq!(
            overlays_of(r#"{"developer": {"flight_title": true}}"#),
            (
                FlightOverlays {
                    title: true,
                    help: false
                },
                vec![]
            )
        );
        assert_eq!(
            overlays_of(r#"{"developer": {"help_line": true}}"#),
            (
                FlightOverlays {
                    title: false,
                    help: true
                },
                vec![]
            )
        );
        assert_eq!(
            overlays_of(
                r#"{"rules": "bible", "developer": {"flight_title": true, "help_line": true}}"#
            ),
            (
                FlightOverlays {
                    title: true,
                    help: true
                },
                vec![]
            )
        );
    }

    #[test]
    fn a_setting_that_is_not_a_bool_is_off_with_a_warning_naming_its_key() {
        for value in [r#""true""#, "1", "null", "[]", "{}"] {
            let text =
                format!(r#"{{"developer": {{"flight_title": {value}, "help_line": true}}}}"#);
            assert_eq!(
                overlays_of(&text),
                (
                    FlightOverlays {
                        title: false,
                        help: true
                    },
                    vec![
                        "nova: the saved \"developer\".\"flight_title\" in memory is not true \
                         or false; leaving it off"
                            .to_owned()
                    ]
                ),
                "{text}"
            );
            let text =
                format!(r#"{{"developer": {{"flight_title": true, "help_line": {value}}}}}"#);
            assert_eq!(
                overlays_of(&text),
                (
                    FlightOverlays {
                        title: true,
                        help: false
                    },
                    vec![
                        "nova: the saved \"developer\".\"help_line\" in memory is not true or \
                         false; leaving it off"
                            .to_owned()
                    ]
                ),
                "{text}"
            );
        }
    }

    #[test]
    fn a_developer_that_is_not_an_object_is_ignored_with_a_warning() {
        for value in ["true", "[]", "null", r#""on""#] {
            let text = format!(r#"{{"developer": {value}}}"#);
            assert_eq!(
                overlays_of(&text),
                (
                    OFF,
                    vec![
                        "nova: the saved \"developer\" in memory is not an object; ignoring it"
                            .to_owned()
                    ]
                ),
                "{text}"
            );
        }
    }
}
