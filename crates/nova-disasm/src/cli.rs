//! The command line, parsed without touching the environment.

use std::ffi::OsString;
use std::path::PathBuf;

/// How to run `nova-disasm`, shown by `--help` and after a usage error.
pub const USAGE: &str = "\
usage: nova-disasm [--binary <PATH>] <COMMAND> <ARG>
       nova-disasm --help|-h

Answers questions about EV Nova's disassembly, from otool and nm output
cached under target/disasm/ by the binary's SHA-256.

commands:
  fn <SYMBOL|ADDRESS>       one function's instructions, with call targets,
                            strings and globals named
  callers <SYMBOL>          the functions that call SYMBOL, with call sites
  callees <SYMBOL>          what SYMBOL calls, with call counts
  find <SUBSTRING|ADDRESS>  matching symbols, their ranges and sizes

  --binary <PATH>  the EV Nova binary (default: $NOVA_BINARY, else
                   EV Nova.app/Contents/MacOS/EV Nova in the primary checkout)
  -h, --help       show this help
";

/// One question about the disassembly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Query {
    /// `fn`: a symbol or an address in the function.
    Function(String),
    /// `callers`: a symbol.
    Callers(String),
    /// `callees`: a symbol.
    Callees(String),
    /// `find`: a name substring or an address.
    Find(String),
}

/// What the command line asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Show the usage text.
    Help,
    /// Answer a query.
    Query {
        /// `--binary`, if given.
        binary: Option<PathBuf>,
        /// The question.
        query: Query,
    },
}

/// A command line that does not parse.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UsageError {
    /// No command was given.
    #[error("missing a command")]
    MissingCommand,
    /// A command that doesn't exist.
    #[error("unknown command {0:?}")]
    UnknownCommand(OsString),
    /// The command's argument was not given.
    #[error("{0} needs an argument")]
    MissingArgument(&'static str),
    /// An argument after the command's.
    #[error("unexpected argument {0:?}")]
    Extra(OsString),
    /// An option this tool does not have.
    #[error("unknown option {0:?}")]
    UnknownFlag(OsString),
    /// `--binary` came last.
    #[error("--binary needs a path")]
    MissingBinary,
    /// A command's argument is not UTF-8.
    #[error("argument {0:?} is not UTF-8")]
    NotUtf8(OsString),
}

/// Parses the arguments, without the program name.
///
/// Only `--binary`, `--help`/`-h` and `--` are options; an Objective-C
/// method name (`-[Foo bar]`, `+[Foo baz]`) is an argument.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, UsageError> {
    let mut binary = None;
    let mut positional = Vec::new();
    let mut args = args.into_iter();
    let mut options = true;
    while let Some(arg) = args.next() {
        let text = arg.to_str();
        match text {
            Some("--") if options => options = false,
            Some("--help" | "-h") if options => return Ok(Command::Help),
            Some("--binary") if options => {
                binary = Some(PathBuf::from(args.next().ok_or(UsageError::MissingBinary)?));
            }
            Some(t) if options && t.starts_with('-') && !t.starts_with("-[") => {
                return Err(UsageError::UnknownFlag(arg));
            }
            _ => positional.push(arg),
        }
    }
    let mut positional = positional.into_iter();
    let command = positional.next().ok_or(UsageError::MissingCommand)?;
    let (name, make): (&'static str, fn(String) -> Query) = match command.to_str() {
        Some("fn") => ("fn", Query::Function),
        Some("callers") => ("callers", Query::Callers),
        Some("callees") => ("callees", Query::Callees),
        Some("find") => ("find", Query::Find),
        _ => return Err(UsageError::UnknownCommand(command)),
    };
    let arg = positional.next().ok_or(UsageError::MissingArgument(name))?;
    if let Some(extra) = positional.next() {
        return Err(UsageError::Extra(extra));
    }
    let arg = arg.into_string().map_err(UsageError::NotUtf8)?;
    Ok(Command::Query {
        binary,
        query: make(arg),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Command, UsageError> {
        parse(args.iter().map(OsString::from))
    }

    fn q(query: Query) -> Command {
        Command::Query {
            binary: None,
            query,
        }
    }

    #[test]
    fn parses_each_command() {
        assert_eq!(
            p(&["fn", "_HandlePlayer"]),
            Ok(q(Query::Function("_HandlePlayer".into())))
        );
        assert_eq!(p(&["callers", "_A"]), Ok(q(Query::Callers("_A".into()))));
        assert_eq!(p(&["callees", "_A"]), Ok(q(Query::Callees("_A".into()))));
        assert_eq!(p(&["find", "player"]), Ok(q(Query::Find("player".into()))));
    }

    #[test]
    fn takes_the_binary_flag_anywhere() {
        let want = Ok(Command::Query {
            binary: Some(PathBuf::from("bin")),
            query: Query::Find("x".into()),
        });
        assert_eq!(p(&["--binary", "bin", "find", "x"]), want);
        assert_eq!(p(&["find", "x", "--binary", "bin"]), want);
    }

    #[test]
    fn help() {
        assert_eq!(p(&["--help"]), Ok(Command::Help));
        assert_eq!(p(&["fn", "-h"]), Ok(Command::Help));
    }

    #[test]
    fn usage_errors() {
        assert_eq!(p(&[]), Err(UsageError::MissingCommand));
        assert_eq!(p(&["dump"]), Err(UsageError::UnknownCommand("dump".into())));
        assert_eq!(p(&["fn"]), Err(UsageError::MissingArgument("fn")));
        assert_eq!(p(&["fn", "a", "b"]), Err(UsageError::Extra("b".into())));
        assert_eq!(
            p(&["--all", "fn", "a"]),
            Err(UsageError::UnknownFlag("--all".into()))
        );
        assert_eq!(p(&["fn", "a", "--binary"]), Err(UsageError::MissingBinary));
    }

    #[test]
    fn an_objective_c_method_is_an_argument() {
        assert_eq!(
            p(&["fn", "-[Foo bar:]"]),
            Ok(q(Query::Function("-[Foo bar:]".into())))
        );
        assert_eq!(
            p(&["callers", "+[A b]"]),
            Ok(q(Query::Callers("+[A b]".into())))
        );
    }

    #[test]
    fn a_double_dash_ends_the_options() {
        assert_eq!(
            p(&["find", "--", "-[Foo"]),
            Ok(q(Query::Find("-[Foo".into())))
        );
    }

    #[test]
    fn after_a_double_dash_option_names_are_arguments() {
        assert_eq!(p(&["--", "fn", "--"]), Ok(q(Query::Function("--".into()))));
        assert_eq!(
            p(&["--", "fn", "--help"]),
            Ok(q(Query::Function("--help".into())))
        );
        assert_eq!(
            p(&["--", "fn", "--binary"]),
            Ok(q(Query::Function("--binary".into())))
        );
    }
}
