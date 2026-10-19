//! The command line: which `Nova Files` directory to load, if the player
//! named one.

use std::ffi::OsString;
use std::path::PathBuf;

/// The command line was not understood.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("usage: nova [NOVA_FILES_DIR] (or set NOVA_DATA to the Nova Files directory)")]
pub struct Usage;

/// The `Nova Files` directory the player named: the one positional
/// argument if given (the arguments exclude the program name), else
/// `nova_data`, the value of `NOVA_DATA`; `None` when neither is, and the
/// lookup goes on (`crate::startup::locate`).
///
/// # Errors
///
/// [`Usage`] for more than one argument.
pub fn data_dir(args: &[OsString], nova_data: Option<OsString>) -> Result<Option<PathBuf>, Usage> {
    match args {
        [dir] => Ok(Some(PathBuf::from(dir))),
        [] => Ok(nova_data.map(PathBuf::from)),
        _ => Err(Usage),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn the_positional_argument_wins() {
        assert_eq!(
            data_dir(&args(&["/games/Nova Files"]), Some("/env".into())),
            Ok(Some(PathBuf::from("/games/Nova Files")))
        );
    }

    #[test]
    fn nova_data_is_the_fallback() {
        assert_eq!(
            data_dir(&args(&[]), Some("/env/Nova Files".into())),
            Ok(Some(PathBuf::from("/env/Nova Files")))
        );
    }

    #[test]
    fn neither_leaves_the_lookup_to_go_on() {
        assert_eq!(data_dir(&args(&[]), None), Ok(None));
    }

    #[test]
    fn the_usage_line_names_the_argument_and_the_variable() {
        assert_eq!(
            Usage.to_string(),
            "usage: nova [NOVA_FILES_DIR] (or set NOVA_DATA to the Nova Files directory)"
        );
    }

    #[test]
    fn more_than_one_argument_is_a_usage_error() {
        assert_eq!(data_dir(&args(&["/a", "/b"]), None), Err(Usage));
    }
}
