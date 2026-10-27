//! Where the player's settings and pilots are saved (and where earlier
//! versions saved the pilots), and where the startup log is written: pure
//! functions of the target operating system and the environment, which
//! the caller reads.

use std::ffi::OsString;
use std::path::PathBuf;

/// The operating systems whose settings go in different places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    /// Windows: under `%APPDATA%`.
    Windows,
    /// macOS: under `~/Library/Application Support`.
    MacOs,
    /// Anything else: the XDG base directories, settings under
    /// `$XDG_CONFIG_HOME`, pilots under `$XDG_DATA_HOME` and the log
    /// under `$XDG_STATE_HOME`.
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

/// The pilots directory's name, in the `nova` directory: the original's
/// folder name (`STR#` 130 #3).
pub const PILOTS_DIR: &str = "Pilots";

/// The settings file on `os`, from the environment variables `env` looks
/// up, or `None` when the variables it needs are unset (or empty):
///
/// - Windows: `%APPDATA%\nova\settings.json`.
/// - macOS: `$HOME/Library/Application Support/nova/settings.json`.
/// - Otherwise: `$XDG_CONFIG_HOME/nova/settings.json`, or
///   `$HOME/.config/nova/settings.json` without it.
pub fn settings_path(os: Os, env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    Some(nova_dir(os, env)?.join(SETTINGS_FILE))
}

/// The directory pilots are saved in on `os`, or `None` when the
/// variables it needs are unset (or empty):
///
/// - Windows: `%APPDATA%\nova\Pilots`, beside the settings file (see
///   [`settings_path`]).
/// - macOS: `$HOME/Library/Application Support/nova/Pilots`, beside the
///   settings file.
/// - Otherwise: `$XDG_DATA_HOME/nova/Pilots`, or
///   `$HOME/.local/share/nova/Pilots` without an absolute one, as pilots
///   are the player's data, not configuration. They were beside the
///   settings before; see [`legacy_pilots_dir`].
pub fn pilots_dir(os: Os, env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let base = match os {
        Os::Windows | Os::MacOs => nova_dir(os, env)?,
        Os::Other => xdg_dir(&env, "XDG_DATA_HOME", &[".local", "share"])?.join("nova"),
    };
    Some(base.join(PILOTS_DIR))
}

/// Where earlier versions saved pilots on `os`, which launch moves them
/// out of into [`pilots_dir`], or `None` where they were never anywhere
/// else (macOS and Windows), when the variables it needs are unset (or
/// empty), or when it is the same directory as [`pilots_dir`] (say
/// `XDG_DATA_HOME` is `XDG_CONFIG_HOME`).
///
/// Otherwise it is [`PILOTS_DIR`] beside the settings file:
/// `$XDG_CONFIG_HOME/nova/Pilots`, or `$HOME/.config/nova/Pilots`.
pub fn legacy_pilots_dir(os: Os, env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    if os != Os::Other {
        return None;
    }
    let old = nova_dir(os, &env)?.join(PILOTS_DIR);
    (pilots_dir(os, &env).as_ref() != Some(&old)).then_some(old)
}

/// The startup log's name, in the `nova` logs directory.
pub const LOG_FILE: &str = "nova.log";

/// The file each launch writes its startup errors and warnings to on `os`,
/// or `None` where there is none yet, or when the variables it needs are
/// unset (or empty):
///
/// - macOS: `$HOME/Library/Logs/nova/nova.log`, where Console.app finds it.
/// - Windows: `%LOCALAPPDATA%\nova\logs\nova.log`: the machine's own,
///   not the roaming `%APPDATA%` the settings are in, as a log need not
///   follow the player to another machine.
/// - Otherwise: `$XDG_STATE_HOME/nova/nova.log`, or
///   `$HOME/.local/state/nova/nova.log` without an absolute
///   `XDG_STATE_HOME`: the XDG home for logs, which the player keeps but
///   need not back up.
pub fn log_path(os: Os, env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let var = |name: &str| env(name).filter(|value| !value.is_empty());
    let (base, parts): (_, &[&str]) = match os {
        Os::MacOs => (PathBuf::from(var("HOME")?), &["Library", "Logs", "nova"]),
        Os::Windows => (PathBuf::from(var("LOCALAPPDATA")?), &["nova", "logs"]),
        Os::Other => (
            xdg_dir(&env, "XDG_STATE_HOME", &[".local", "state"])?,
            &["nova"],
        ),
    };
    let mut path = base;
    path.extend(parts);
    path.push(LOG_FILE);
    Some(path)
}

/// The XDG base directory `name` names (`XDG_DATA_HOME`, say), or
/// `$HOME` joined with `fallback` when it is unset, empty or relative: the
/// XDG Base Directory spec says to ignore a relative one.
///
/// Relative here means without a root (`Path::has_root`), not
/// `!is_absolute()`: on a Windows host `/home/p` has a root but isn't
/// absolute, and the `Os::Other` paths are tested on every host.
pub(crate) fn xdg_dir(
    env: impl Fn(&str) -> Option<OsString>,
    name: &str,
    fallback: &[&str],
) -> Option<PathBuf> {
    let var = |name: &str| {
        env(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    if let Some(dir) = var(name).filter(|dir| dir.has_root()) {
        return Some(dir);
    }
    let mut home = var("HOME")?;
    home.extend(fallback);
    Some(home)
}

/// The `nova` directory under the platform's configuration directory.
fn nova_dir(os: Os, env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
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
    Some(base.join("nova"))
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

    fn pilots(base: &str, parts: &[&str]) -> PathBuf {
        let mut path = Path::new(base).to_path_buf();
        path.extend(parts);
        path.extend(["nova", "Pilots"]);
        path
    }

    #[test]
    fn mac_and_windows_pilots_are_saved_beside_the_settings() {
        let vars = env(&[("HOME", "/Users/p"), ("XDG_CONFIG_HOME", "/xdg")]);
        assert_eq!(
            pilots_dir(Os::MacOs, vars),
            Some(pilots("/Users/p", &["Library", "Application Support"]))
        );
        let vars = env(&[("APPDATA", "C:/Roaming"), ("HOME", "/home/p")]);
        assert_eq!(
            pilots_dir(Os::Windows, vars),
            Some(pilots("C:/Roaming", &[]))
        );
        for os in [Os::MacOs, Os::Windows, Os::Other] {
            assert_eq!(pilots_dir(os, env(&[])), None, "{os:?}");
        }
        assert_eq!(PILOTS_DIR, "Pilots");
    }

    #[test]
    fn elsewhere_pilots_are_saved_under_the_xdg_data_home() {
        let vars = env(&[
            ("HOME", "/home/p"),
            ("XDG_CONFIG_HOME", "/xdg-config"),
            ("XDG_DATA_HOME", "/xdg-data"),
        ]);
        assert_eq!(pilots_dir(Os::Other, vars), Some(pilots("/xdg-data", &[])));
    }

    #[test]
    fn without_an_absolute_xdg_data_home_pilots_are_under_dot_local_share() {
        for vars in [
            env(&[("HOME", "/home/p"), ("XDG_CONFIG_HOME", "/xdg")]),
            env(&[("HOME", "/home/p"), ("XDG_DATA_HOME", "")]),
            env(&[("HOME", "/home/p"), ("XDG_DATA_HOME", "rel/dir")]),
        ] {
            assert_eq!(
                pilots_dir(Os::Other, vars),
                Some(pilots("/home/p", &[".local", "share"]))
            );
        }
        let vars = env(&[("XDG_DATA_HOME", "rel/dir")]);
        assert_eq!(pilots_dir(Os::Other, vars), None);
        let vars = env(&[("HOME", ""), ("XDG_DATA_HOME", "")]);
        assert_eq!(pilots_dir(Os::Other, vars), None);
    }

    #[test]
    fn an_absolute_xdg_data_home_needs_no_home() {
        let vars = env(&[("XDG_DATA_HOME", "/xdg-data")]);
        assert_eq!(pilots_dir(Os::Other, vars), Some(pilots("/xdg-data", &[])));
    }

    #[test]
    fn pilots_used_to_be_saved_beside_the_settings_elsewhere() {
        let vars = env(&[
            ("HOME", "/home/p"),
            ("XDG_CONFIG_HOME", "/xdg-config"),
            ("XDG_DATA_HOME", "/xdg-data"),
        ]);
        assert_eq!(
            legacy_pilots_dir(Os::Other, vars),
            Some(pilots("/xdg-config", &[]))
        );
        let vars = env(&[("HOME", "/home/p")]);
        assert_eq!(
            legacy_pilots_dir(Os::Other, vars),
            Some(pilots("/home/p", &[".config"]))
        );
        assert_eq!(legacy_pilots_dir(Os::Other, env(&[])), None);
    }

    #[test]
    fn mac_and_windows_pilots_have_no_old_place() {
        let vars = env(&[
            ("HOME", "/Users/p"),
            ("APPDATA", "C:/Roaming"),
            ("XDG_CONFIG_HOME", "/xdg"),
        ]);
        assert_eq!(legacy_pilots_dir(Os::MacOs, &vars), None);
        assert_eq!(legacy_pilots_dir(Os::Windows, &vars), None);
    }

    #[test]
    fn pilots_have_no_old_place_when_it_is_the_new_one() {
        let vars = env(&[
            ("HOME", "/home/p"),
            ("XDG_CONFIG_HOME", "/xdg"),
            ("XDG_DATA_HOME", "/xdg"),
        ]);
        assert_eq!(pilots_dir(Os::Other, &vars), Some(pilots("/xdg", &[])));
        assert_eq!(legacy_pilots_dir(Os::Other, &vars), None);
    }

    #[test]
    fn the_mac_startup_log_is_under_library_logs() {
        let mut expected = PathBuf::from("/Users/p");
        expected.extend(["Library", "Logs", "nova", "nova.log"]);
        let vars = env(&[("HOME", "/Users/p"), ("XDG_CONFIG_HOME", "/xdg")]);
        assert_eq!(log_path(Os::MacOs, vars), Some(expected));
        assert_eq!(log_path(Os::MacOs, env(&[])), None);
        assert_eq!(log_path(Os::MacOs, env(&[("HOME", "")])), None);
    }

    #[test]
    fn the_windows_startup_log_is_under_localappdata_logs() {
        let mut expected = PathBuf::from("C:/Users/p/AppData/Local");
        expected.extend(["nova", "logs", "nova.log"]);
        let vars = env(&[
            ("LOCALAPPDATA", "C:/Users/p/AppData/Local"),
            ("APPDATA", "C:/Users/p/AppData/Roaming"),
            ("HOME", "/home/p"),
        ]);
        assert_eq!(log_path(Os::Windows, vars), Some(expected));
    }

    #[test]
    fn without_a_localappdata_there_is_no_windows_log() {
        assert_eq!(log_path(Os::Windows, env(&[])), None);
        assert_eq!(log_path(Os::Windows, env(&[("LOCALAPPDATA", "")])), None);
        let roaming = env(&[("APPDATA", "C:/Roaming"), ("HOME", "/home/p")]);
        assert_eq!(log_path(Os::Windows, roaming), None);
    }

    fn state_log(base: &str, parts: &[&str]) -> PathBuf {
        let mut path = PathBuf::from(base);
        path.extend(parts);
        path.extend(["nova", "nova.log"]);
        path
    }

    #[test]
    fn elsewhere_the_startup_log_is_under_the_xdg_state_home() {
        let vars = env(&[
            ("HOME", "/home/p"),
            ("APPDATA", "C:/Roaming"),
            ("LOCALAPPDATA", "C:/Local"),
            ("XDG_CONFIG_HOME", "/xdg"),
            ("XDG_DATA_HOME", "/xdg-data"),
            ("XDG_STATE_HOME", "/xdg-state"),
        ]);
        assert_eq!(
            log_path(Os::Other, vars),
            Some(state_log("/xdg-state", &[]))
        );
        let vars = env(&[("XDG_STATE_HOME", "/xdg-state")]);
        assert_eq!(
            log_path(Os::Other, vars),
            Some(state_log("/xdg-state", &[]))
        );
    }

    #[test]
    fn without_an_absolute_xdg_state_home_the_log_is_under_dot_local_state() {
        for vars in [
            env(&[("HOME", "/home/p"), ("XDG_DATA_HOME", "/xdg-data")]),
            env(&[("HOME", "/home/p"), ("XDG_STATE_HOME", "")]),
            env(&[("HOME", "/home/p"), ("XDG_STATE_HOME", "rel/dir")]),
        ] {
            assert_eq!(
                log_path(Os::Other, vars),
                Some(state_log("/home/p", &[".local", "state"]))
            );
        }
        assert_eq!(log_path(Os::Other, env(&[])), None);
        assert_eq!(log_path(Os::Other, env(&[("HOME", "")])), None);
        let vars = env(&[("XDG_STATE_HOME", "rel/dir")]);
        assert_eq!(log_path(Os::Other, vars), None);
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
