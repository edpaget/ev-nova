//! The command line: which `Nova Files` directory to load.

use std::ffi::OsString;
use std::path::PathBuf;

/// The command line was not understood.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("usage: nova [NOVA_FILES_DIR] (or set NOVA_DATA to the Nova Files directory)")]
pub struct Usage;

/// The `Nova Files` directory: the one positional argument if given (the
/// arguments exclude the program name), else `nova_data`, the value of
/// `NOVA_DATA`.
pub fn data_dir(args: &[OsString], nova_data: Option<OsString>) -> Result<PathBuf, Usage> {
    match args {
        [dir] => Ok(PathBuf::from(dir)),
        [] => nova_data.map(PathBuf::from).ok_or(Usage),
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
            Ok(PathBuf::from("/games/Nova Files"))
        );
    }

    #[test]
    fn nova_data_is_the_fallback() {
        assert_eq!(
            data_dir(&args(&[]), Some("/env/Nova Files".into())),
            Ok(PathBuf::from("/env/Nova Files"))
        );
    }

    #[test]
    fn neither_is_a_usage_error() {
        assert_eq!(data_dir(&args(&[]), None), Err(Usage));
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
