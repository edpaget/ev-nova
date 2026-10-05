//! The law the game judges the player's crimes by: [`NovaLaw`], following
//! the original engine's crime gains or the Bible's as the settings file
//! says.
//!
//! The choice is the `crime_gains` field of the settings file (the one
//! that keeps the sound settings; see [`config`](crate::config)):
//! `"engine"` (the default) or `"bible"`, as [`CrimeGains`] describes.
//! It is not part of the pilot save.

use nova_audio::SettingsStore;
use nova_sim::{CrimeGains, NovaLaw};

/// The settings file's field naming the [`CrimeGains`].
pub const CRIME_GAINS: &str = "crime_gains";

/// The law the settings saved in `store` choose, and a warning to print,
/// if any.
///
/// With no store, nothing saved, no `crime_gains` field, or settings that
/// cannot be read or are not a JSON object, the law is the default, the
/// engine's, silently: the sound settings' keeper warns about settings it
/// cannot use. A `crime_gains` other than `"engine"` or `"bible"` gives
/// the default too, with a warning naming the store's location.
pub fn game_law(store: Option<&mut (dyn SettingsStore + '_)>) -> (NovaLaw, Option<String>) {
    let Some(store) = store else {
        return (NovaLaw::default(), None);
    };
    let Ok(Some(text)) = store.read() else {
        return (NovaLaw::default(), None);
    };
    let Ok(serde_json::Value::Object(settings)) = serde_json::from_str(&text) else {
        return (NovaLaw::default(), None);
    };
    let gains = match settings.get(CRIME_GAINS).map(serde_json::Value::as_str) {
        None => CrimeGains::default(),
        Some(Some("engine")) => CrimeGains::Engine,
        Some(Some("bible")) => CrimeGains::Bible,
        Some(_) => {
            let warning = format!(
                "nova: the saved {CRIME_GAINS} in {} is not \"engine\" or \"bible\"; \
                 following the engine's",
                store.location()
            );
            return (NovaLaw::default(), Some(warning));
        }
    };
    (NovaLaw { gains }, None)
}

#[cfg(test)]
mod tests {
    use nova_audio::recording::MemorySettings;

    use super::*;

    const ENGINE: NovaLaw = NovaLaw {
        gains: CrimeGains::Engine,
    };
    const BIBLE: NovaLaw = NovaLaw {
        gains: CrimeGains::Bible,
    };

    fn law_of(text: &str) -> (NovaLaw, Option<String>) {
        game_law(Some(&mut MemorySettings::holding(text)))
    }

    #[test]
    fn the_settings_choose_the_engines_crime_gains_or_the_bibles() {
        assert_eq!(law_of(r#"{"crime_gains": "bible"}"#), (BIBLE, None));
        assert_eq!(
            law_of(r#"{"sound": false, "crime_gains": "engine"}"#),
            (ENGINE, None)
        );
        assert_eq!(CRIME_GAINS, "crime_gains");
    }

    #[test]
    fn without_a_choice_the_law_is_the_engines() {
        assert_eq!(game_law(None), (ENGINE, None), "no store");
        assert_eq!(
            game_law(Some(&mut MemorySettings::new())),
            (ENGINE, None),
            "nothing saved"
        );
        assert_eq!(law_of(r#"{"sound": false}"#), (ENGINE, None));
        assert_eq!(law_of("{}"), (ENGINE, None));
    }

    #[test]
    fn settings_that_cannot_be_used_give_the_engines_law_leaving_the_warning_to_the_sound() {
        for text in ["not json", "", "[1, 2]", r#""bible""#] {
            assert_eq!(law_of(text), (ENGINE, None), "{text}");
        }
        let store = MemorySettings::holding(r#"{"crime_gains": "bible"}"#);
        store.fail_reads(true);
        let mut store = store;
        assert_eq!(game_law(Some(&mut store)), (ENGINE, None));
    }

    #[test]
    fn a_crime_gains_that_is_neither_gives_the_engines_law_with_a_warning() {
        for text in [
            r#"{"crime_gains": "Bible"}"#,
            r#"{"crime_gains": "both"}"#,
            r#"{"crime_gains": 1}"#,
            r#"{"crime_gains": null}"#,
        ] {
            assert_eq!(
                law_of(text),
                (
                    ENGINE,
                    Some(
                        "nova: the saved crime_gains in memory is not \"engine\" or \"bible\"; \
                         following the engine's"
                            .to_owned()
                    )
                ),
                "{text}"
            );
        }
    }
}
