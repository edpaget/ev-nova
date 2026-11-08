//! The program: parse the command line, load the listings, answer.
//!
//! Exit codes: 0 answered, 1 nothing matched (or a tool failed), 2 a
//! usage error or a missing binary.

use std::ffi::OsString;
use std::io::Write;

use crate::cache::{self, LoadError};
use crate::cli::{self, Command, Query, USAGE};
use crate::index::Index;
use crate::locate;
use crate::ports::{Binary, Cache, Env, Tools, Workspace};
use crate::render;

/// How many `find` matches a miss suggests.
const SUGGESTIONS: usize = 10;

/// The adapters [`run`] works through.
pub struct Ports<'a> {
    /// Reads the binary.
    pub binary: &'a dyn Binary,
    /// Lists it.
    pub tools: &'a dyn Tools,
    /// Keeps the listings.
    pub cache: &'a dyn Cache,
    /// The environment (`NOVA_BINARY`).
    pub env: &'a dyn Env,
    /// Where the checkouts are.
    pub workspace: &'a dyn Workspace,
}

/// Runs the program over `args` (without the program name) and returns
/// its exit code.
pub fn run(
    args: impl IntoIterator<Item = OsString>,
    ports: &Ports<'_>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> u8 {
    let (binary, query) = match cli::parse(args) {
        Ok(Command::Help) => {
            let _ = stdout.write_all(USAGE.as_bytes());
            return 0;
        }
        Ok(Command::Query { binary, query }) => (binary, query),
        Err(err) => {
            let _ = write!(stderr, "error: {err}\n{USAGE}");
            return 2;
        }
    };
    let path = locate::binary_path(binary.as_deref(), ports.env, ports.workspace);
    let listings = match cache::load(&path, ports.binary, ports.tools, ports.cache, stderr) {
        Ok(listings) => listings,
        Err(err) => {
            let _ = writeln!(stderr, "error: {err}");
            return match err {
                LoadError::BinaryMissing { .. } => 2,
                _ => 1,
            };
        }
    };
    let index = Index::build(&listings);
    let answer = answer(&index, &query);
    let _ = stdout.write_all(answer.out.as_bytes());
    let _ = stderr.write_all(answer.note.as_bytes());
    answer.code
}

/// What a query prints and how it exits.
struct Answer {
    code: u8,
    /// The answer itself, on stdout.
    out: String,
    /// A miss or an empty answer, explained on stderr.
    note: String,
}

impl Answer {
    fn found(out: String) -> Answer {
        Answer {
            code: 0,
            out,
            note: String::new(),
        }
    }

    fn note(code: u8, note: String) -> Answer {
        Answer {
            code,
            out: String::new(),
            note,
        }
    }
}

fn answer(index: &Index, query: &Query) -> Answer {
    match query {
        Query::Function(q) => match index.lookup(q) {
            Some(f) => Answer::found(render::function(index, f)),
            None => Answer::note(1, miss(index, q)),
        },
        Query::Callees(q) => match index.lookup(q) {
            Some(f) => Answer::found(render::callees(&index.callees(f))),
            None => Answer::note(1, miss(index, q)),
        },
        Query::Callers(q) => {
            let callers = index.callers(q);
            if callers.is_empty() {
                // A function nobody calls is an answer; an unknown name a miss.
                let code = u8::from(index.function_named(q).is_none());
                Answer::note(code, format!("no calls to {q:?}\n"))
            } else {
                Answer::found(render::callers(&callers))
            }
        }
        Query::Find(q) => {
            let found = index.find(q);
            if found.is_empty() {
                Answer::note(1, format!("nothing matches {q:?}\n"))
            } else {
                Answer::found(render::found(&found))
            }
        }
    }
}

/// A function that isn't there, with up to [`SUGGESTIONS`] `find` matches.
fn miss(index: &Index, q: &str) -> String {
    let mut found = index.find(q);
    found.retain(|f| !f.data);
    found.truncate(SUGGESTIONS);
    if found.is_empty() {
        format!("no function {q:?}\n")
    } else {
        format!(
            "no function {q:?}; did you mean:\n{}",
            render::found(&found)
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::fakes::{FakeBinary, FakeEnv, FakeTools, FakeWorkspace, MemCache};
    use crate::parse::fixture;

    struct Run {
        code: u8,
        out: String,
        err: String,
    }

    fn run_with(binary: &FakeBinary, args: &[&str]) -> Run {
        let tools = FakeTools::with(fixture::listings());
        let cache = MemCache::default();
        let env = FakeEnv::default();
        let workspace = FakeWorkspace {
            primary: Some(PathBuf::from("primary")),
            root: PathBuf::from("root"),
        };
        let ports = Ports {
            binary,
            tools: &tools,
            cache: &cache,
            env: &env,
            workspace: &workspace,
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(args.iter().map(OsString::from), &ports, &mut out, &mut err);
        Run {
            code,
            out: String::from_utf8(out).unwrap(),
            err: String::from_utf8(err).unwrap(),
        }
    }

    fn go(args: &[&str]) -> Run {
        run_with(&FakeBinary::with(b"nova"), args)
    }

    #[test]
    fn help_prints_the_usage() {
        let r = go(&["--help"]);
        assert_eq!((r.code, r.out.as_str()), (0, USAGE));
    }

    #[test]
    fn a_usage_error_exits_2_with_the_usage() {
        let r = go(&["dump"]);
        assert_eq!(r.code, 2);
        assert!(
            r.err.starts_with("error: unknown command \"dump\"\n"),
            "{}",
            r.err
        );
        assert!(r.err.ends_with(USAGE));
    }

    #[test]
    fn a_missing_binary_exits_2_naming_the_expected_path() {
        let r = run_with(&FakeBinary::missing(), &["fn", "_Beta"]);
        assert_eq!(r.code, 2);
        let expected = locate::app_binary(Path::new("primary"));
        assert!(
            r.err.contains(&format!(
                "EV Nova binary not found at {}",
                expected.display()
            )),
            "{}",
            r.err
        );
        assert!(r.out.is_empty());
    }

    #[test]
    fn the_binary_flag_names_the_path_read() {
        let binary = FakeBinary::missing();
        let r = run_with(&binary, &["--binary", "elsewhere", "fn", "_Beta"]);
        assert_eq!(r.code, 2);
        assert_eq!(*binary.reads.borrow(), [PathBuf::from("elsewhere")]);
    }

    #[test]
    fn fn_prints_the_function() {
        let r = go(&["fn", "Gamma"]);
        assert_eq!(r.code, 0);
        assert!(
            r.out.starts_with("_Gamma 0x3080..0x30a0 (32 bytes)\n"),
            "{}",
            r.out
        );
        assert!(!r.out.contains("_Delta"));
    }

    #[test]
    fn fn_takes_an_address_inside_the_function() {
        let r = go(&["fn", "0x3058"]);
        assert!(r.out.starts_with("_Beta 0x3050..0x3080"), "{}", r.out);
    }

    #[test]
    fn a_fn_miss_suggests_matches_and_exits_1() {
        let r = go(&["fn", "amm"]);
        assert_eq!(r.code, 1);
        assert!(r.out.is_empty());
        assert_eq!(
            r.err,
            "no function \"amm\"; did you mean:\n_Gamma  0x3080..0x30a0  32\n"
        );
    }

    #[test]
    fn a_fn_miss_with_nothing_similar() {
        let r = go(&["fn", "0x10"]);
        assert_eq!((r.code, r.err.as_str()), (1, "no function \"0x10\"\n"));
    }

    #[test]
    fn a_miss_suggests_at_most_ten() {
        let mut listings = fixture::listings();
        listings.text = std::iter::once("/x:\n(__TEXT,__text) section\n".to_owned())
            .chain((0..12).map(|i| format!("_Many{i}:\n{:08x}\tret\n", 0x1000 + i)))
            .collect();
        let tools = FakeTools::with(listings);
        let (cache, env) = (MemCache::default(), FakeEnv::default());
        let workspace = FakeWorkspace {
            primary: None,
            root: PathBuf::from("root"),
        };
        let binary = FakeBinary::with(b"x");
        let ports = Ports {
            binary: &binary,
            tools: &tools,
            cache: &cache,
            env: &env,
            workspace: &workspace,
        };
        let mut err = Vec::new();
        run(
            ["fn", "many"].map(OsString::from),
            &ports,
            &mut Vec::new(),
            &mut err,
        );
        assert_eq!(
            String::from_utf8(err).unwrap().lines().count(),
            1 + SUGGESTIONS
        );
    }

    #[test]
    fn callers_and_callees() {
        let r = go(&["callers", "_Gamma"]);
        assert_eq!(
            (r.code, r.out.as_str()),
            (0, "_Alpha  0x3008\n_Beta  0x3051\n")
        );
        let r = go(&["callees", "_Beta"]);
        assert_eq!(
            r.out,
            "_Gamma  x1\n_Delta  x1 (1 tail)\n_SysBeep  x1 (1 tail)\n"
        );
    }

    #[test]
    fn a_function_nobody_calls() {
        let r = go(&["callers", "_Alpha"]);
        assert_eq!(
            (r.code, r.out.as_str(), r.err.as_str()),
            (0, "", "no calls to \"_Alpha\"\n")
        );
    }

    #[test]
    fn callers_of_an_unknown_name_exit_1() {
        let r = go(&["callers", "_Nobody"]);
        assert_eq!((r.code, r.err.as_str()), (1, "no calls to \"_Nobody\"\n"));
    }

    #[test]
    fn callees_of_an_unknown_function_exit_1() {
        let r = go(&["callees", "_Nobody"]);
        assert_eq!((r.code, r.err.as_str()), (1, "no function \"_Nobody\"\n"));
    }

    #[test]
    fn find_prints_matches_or_exits_1() {
        let r = go(&["find", "BETA"]);
        assert_eq!((r.code, r.out.as_str()), (0, "_Beta  0x3050..0x3080  48\n"));
        let r = go(&["find", "zzz"]);
        assert_eq!((r.code, r.err.as_str()), (1, "nothing matches \"zzz\"\n"));
    }

    #[test]
    fn a_missing_tool_exits_1() {
        let tools = FakeTools {
            missing: true,
            ..FakeTools::named()
        };
        let (cache, env) = (MemCache::default(), FakeEnv::default());
        let workspace = FakeWorkspace {
            primary: None,
            root: PathBuf::from("root"),
        };
        let binary = FakeBinary::with(b"x");
        let ports = Ports {
            binary: &binary,
            tools: &tools,
            cache: &cache,
            env: &env,
            workspace: &workspace,
        };
        let mut err = Vec::new();
        let code = run(
            ["find", "x"].map(OsString::from),
            &ports,
            &mut Vec::new(),
            &mut err,
        );
        assert_eq!(code, 1);
        assert!(
            String::from_utf8(err)
                .unwrap()
                .starts_with("error: otool not found")
        );
    }
}
