//! The command line, parsed without touching the environment.
//!
//! [`parse`] is a pure function over the arguments (without the program
//! name), kept as [`OsString`]s so paths that are not UTF-8 survive.

use std::ffi::OsString;
use std::path::PathBuf;

/// How to run `nova-dump`, shown by `--help` and after a usage error.
pub const USAGE: &str = "\
usage: nova-dump <DATA_DIR> <OUT_DIR> [--plugins <DIR>] [--verbose|-v]
       nova-dump --help|-h

Writes every record as JSON, every PICT, cicn, ppat and rl\u{eb}D as PNG and
every snd as WAV under OUT_DIR, which must be missing or empty.

  --plugins <DIR>  also load the plug-ins under DIR
  -v, --verbose    list ignored files, and show the field path and bytes
                   around each failure
  -h, --help       show this help
  --               treat every later argument as a directory
";

/// What the command line asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Show the usage text.
    Help,
    /// Dump the game data.
    Dump(Options),
}

/// The options of a dump.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// The data directory (`Nova Files`).
    pub data: PathBuf,
    /// The output directory.
    pub out: PathBuf,
    /// The plug-ins directory, if any.
    pub plugins: Option<PathBuf>,
    /// Whether to add details to the summary.
    pub verbose: bool,
}

/// A command line that does not parse.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UsageError {
    /// A required directory was not given.
    #[error("missing the {0} directory")]
    MissingDirectory(&'static str),
    /// A positional argument after both directories.
    #[error("unexpected argument {0:?}")]
    Extra(OsString),
    /// An option this tool does not have.
    #[error("unknown option {0:?}")]
    UnknownFlag(OsString),
    /// An option that needs a value came last.
    #[error("{0} needs a directory")]
    MissingValue(&'static str),
    /// An option that may be given once was given again.
    #[error("{0} given more than once")]
    Repeated(&'static str),
}

const PLUGINS: &str = "--plugins";

/// Parses the arguments after the program name. `--help` anywhere among the
/// options wins over everything else, errors included.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, UsageError> {
    let mut args = args.into_iter();
    let mut positionals = Vec::new();
    let mut plugins = None;
    let mut verbose = false;
    let mut error = None;
    let mut flags_done = false;
    while let Some(arg) = args.next() {
        let is_flag = !flags_done && arg.len() > 1 && arg.as_encoded_bytes().starts_with(b"-");
        if !is_flag {
            positionals.push(arg);
            continue;
        }
        match arg.to_str() {
            Some("--help" | "-h") => return Ok(Command::Help),
            Some("--") => flags_done = true,
            Some("--verbose" | "-v") => verbose = true,
            Some(PLUGINS) => match args.next() {
                None => keep_first(&mut error, UsageError::MissingValue(PLUGINS)),
                Some(_) if plugins.is_some() => {
                    keep_first(&mut error, UsageError::Repeated(PLUGINS));
                }
                Some(dir) => plugins = Some(PathBuf::from(dir)),
            },
            _ => keep_first(&mut error, UsageError::UnknownFlag(arg)),
        }
    }
    if let Some(error) = error {
        return Err(error);
    }
    let mut positionals = positionals.into_iter();
    let data = positionals
        .next()
        .ok_or(UsageError::MissingDirectory("data"))?;
    let out = positionals
        .next()
        .ok_or(UsageError::MissingDirectory("output"))?;
    if let Some(extra) = positionals.next() {
        return Err(UsageError::Extra(extra));
    }
    Ok(Command::Dump(Options {
        data: data.into(),
        out: out.into(),
        plugins,
        verbose,
    }))
}

/// Records `new` unless an earlier error is already recorded.
fn keep_first(error: &mut Option<UsageError>, new: UsageError) {
    error.get_or_insert(new);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(args: &[&str]) -> Result<Command, UsageError> {
        parse(args.iter().map(OsString::from))
    }

    fn dump(data: &str, out: &str, plugins: Option<&str>, verbose: bool) -> Command {
        Command::Dump(Options {
            data: PathBuf::from(data),
            out: PathBuf::from(out),
            plugins: plugins.map(PathBuf::from),
            verbose,
        })
    }

    #[test]
    fn two_positionals_are_the_data_and_output_directories() {
        assert_eq!(
            parse_strs(&["data", "out"]),
            Ok(dump("data", "out", None, false))
        );
    }

    #[test]
    fn plugins_takes_the_next_argument_anywhere() {
        let expected = Ok(dump("data", "out", Some("plug"), false));
        assert_eq!(parse_strs(&["--plugins", "plug", "data", "out"]), expected);
        assert_eq!(parse_strs(&["data", "--plugins", "plug", "out"]), expected);
        assert_eq!(parse_strs(&["data", "out", "--plugins", "plug"]), expected);
    }

    #[test]
    fn verbose_has_a_long_and_a_short_form() {
        let expected = Ok(dump("data", "out", None, true));
        assert_eq!(parse_strs(&["--verbose", "data", "out"]), expected);
        assert_eq!(parse_strs(&["data", "out", "-v"]), expected);
        assert_eq!(parse_strs(&["-v", "data", "-v", "out"]), expected);
    }

    #[test]
    fn help_has_a_long_and_a_short_form_and_wins() {
        assert_eq!(parse_strs(&["--help"]), Ok(Command::Help));
        assert_eq!(parse_strs(&["-h"]), Ok(Command::Help));
        assert_eq!(parse_strs(&["data", "--bogus", "-h"]), Ok(Command::Help));
    }

    #[test]
    fn a_double_dash_ends_the_flags() {
        assert_eq!(
            parse_strs(&["-v", "--", "-data", "--plugins"]),
            Ok(dump("-data", "--plugins", None, true))
        );
        assert_eq!(
            parse_strs(&["--", "-h", "out"]),
            Ok(dump("-h", "out", None, false))
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_paths_survive() {
        use std::os::unix::ffi::OsStringExt;
        let odd = OsString::from_vec(vec![b'd', 0xFF]);
        let parsed = parse([odd.clone(), OsString::from("out")]);
        let Ok(Command::Dump(options)) = parsed else {
            panic!("{parsed:?}")
        };
        assert_eq!(options.data, PathBuf::from(odd));
    }

    #[test]
    fn usage_errors_name_the_problem() {
        let cases: [(&[&str], UsageError); 7] = [
            (&[], UsageError::MissingDirectory("data")),
            (&["data"], UsageError::MissingDirectory("output")),
            (&["a", "b", "c"], UsageError::Extra("c".into())),
            (
                &["a", "b", "--frob"],
                UsageError::UnknownFlag("--frob".into()),
            ),
            (&["-x", "a", "b"], UsageError::UnknownFlag("-x".into())),
            (
                &["a", "b", "--plugins"],
                UsageError::MissingValue("--plugins"),
            ),
            (
                &["--plugins", "p", "a", "b", "--plugins", "q"],
                UsageError::Repeated("--plugins"),
            ),
        ];
        for (args, error) in cases {
            assert_eq!(parse_strs(args), Err(error), "{args:?}");
        }
    }

    #[test]
    fn usage_error_messages() {
        let cases = [
            (
                UsageError::MissingDirectory("data"),
                "missing the data directory",
            ),
            (UsageError::Extra("c".into()), "unexpected argument \"c\""),
            (
                UsageError::UnknownFlag("--frob".into()),
                "unknown option \"--frob\"",
            ),
            (
                UsageError::MissingValue("--plugins"),
                "--plugins needs a directory",
            ),
            (
                UsageError::Repeated("--plugins"),
                "--plugins given more than once",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn the_first_usage_error_is_reported() {
        assert_eq!(
            parse_strs(&["--frob", "a", "b", "--plugins"]),
            Err(UsageError::UnknownFlag("--frob".into()))
        );
    }

    #[test]
    fn a_lone_dash_is_a_positional() {
        assert_eq!(parse_strs(&["-", "out"]), Ok(dump("-", "out", None, false)));
    }

    #[test]
    fn the_usage_text_names_every_option() {
        for option in [
            "<DATA_DIR>",
            "<OUT_DIR>",
            "--plugins",
            "--verbose",
            "-v",
            "--help",
            "-h",
        ] {
            assert!(USAGE.contains(option), "{option}");
        }
    }
}
