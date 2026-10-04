//! Where the player's settings are saved: a pure function of the target
//! operating system and the environment, which the caller reads.

use std::ffi::OsString;
use std::path::PathBuf;

/// The operating systems whose settings go in different places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    /// Windows: under `%APPDATA%`.
    Windows,
    /// macOS: under `~/Library/Application Support`.
    MacOs,
    /// Anything else: the XDG configuration directory.
    Other,
}

impl Os {
    /// The operating system this program was built for.
    #[must_use]
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Other
        }
    }
}

/// The settings file's name, in the `nova` directory.
pub const SETTINGS_FILE: &str = "settings.json";

/// The settings file on `os`, from the environment variables `env` looks
/// up, or `None` when the variables it needs are unset (or empty):
///
/// - Windows: `%APPDATA%\nova\settings.json`.
/// - macOS: `$HOME/Library/Application Support/nova/settings.json`.
/// - Otherwise: `$XDG_CONFIG_HOME/nova/settings.json`, or
///   `$HOME/.config/nova/settings.json` without it.
pub fn settings_path(os: Os, env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let var = |name: &str| {
        env(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let base = match os {
        Os::Windows => var("APPDATA")?,
        Os::MacOs => var("HOME")?.join("Library").join("Application Support"),
        Os::Other => match var("XDG_CONFIG_HOME") {
            Some(config) => config,
            None => var("HOME")?.join(".config"),
        },
    };
    Some(base.join("nova").join(SETTINGS_FILE))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// An environment holding `vars`, and nothing else.
    fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let vars: Vec<(String, OsString)> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        move |name| {
            vars.iter()
                .find(|(var, _)| var == name)
                .map(|(_, value)| value.clone())
        }
    }

    fn nova(base: &str, parts: &[&str]) -> PathBuf {
        let mut path = Path::new(base).to_path_buf();
        path.extend(parts);
        path.extend(["nova", "settings.json"]);
        path
    }

    #[test]
    fn windows_settings_are_under_appdata() {
        let vars = env(&[
            ("APPDATA", "C:/Users/p/AppData/Roaming"),
            ("HOME", "/home/p"),
        ]);
        assert_eq!(
            settings_path(Os::Windows, vars),
            Some(nova("C:/Users/p/AppData/Roaming", &[]))
        );
        assert_eq!(
            settings_path(Os::Windows, env(&[("HOME", "/home/p")])),
            None
        );
        assert_eq!(settings_path(Os::Windows, env(&[("APPDATA", "")])), None);
    }

    #[test]
    fn mac_settings_are_under_application_support() {
        let vars = env(&[("HOME", "/Users/p"), ("XDG_CONFIG_HOME", "/xdg")]);
        assert_eq!(
            settings_path(Os::MacOs, vars),
            Some(nova("/Users/p", &["Library", "Application Support"]))
        );
        assert_eq!(settings_path(Os::MacOs, env(&[])), None);
        assert_eq!(settings_path(Os::MacOs, env(&[("HOME", "")])), None);
    }

    #[test]
    fn elsewhere_settings_are_under_the_xdg_config_home() {
        let vars = env(&[("HOME", "/home/p"), ("XDG_CONFIG_HOME", "/xdg")]);
        assert_eq!(settings_path(Os::Other, vars), Some(nova("/xdg", &[])));
    }

    #[test]
    fn without_an_xdg_config_home_they_are_under_dot_config() {
        for vars in [
            env(&[("HOME", "/home/p")]),
            env(&[("HOME", "/home/p"), ("XDG_CONFIG_HOME", "")]),
        ] {
            assert_eq!(
                settings_path(Os::Other, vars),
                Some(nova("/home/p", &[".config"]))
            );
        }
        assert_eq!(settings_path(Os::Other, env(&[("APPDATA", "C:/x")])), None);
    }

    #[test]
    fn the_current_os_is_the_build_target() {
        let expected = if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Other
        };
        assert_eq!(Os::current(), expected);
        assert_eq!(SETTINGS_FILE, "settings.json");
    }
}
