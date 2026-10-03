//! The whole program, over its ports: parse, open, export, report.
//!
//! Exit codes: 0 when the dump finished with no failures; 1 when it
//! finished but some failures were listed (everything else was still
//! written); 2 for a usage error, data that cannot be opened, a data
//! directory that holds no resource file at all (checked before the output
//! directory is touched), an output directory that cannot be created, read
//! or is not empty, or a write error. `--help` exits 0.

use std::ffi::OsString;
use std::fmt::{Display, Write as _};
use std::io::Write;
use std::path::Path;

use nova_data::store::{GameData, Origin};

use crate::cli::{self, Command, Options, USAGE};
use crate::export::export;
use crate::ports::{DataSource, OutputRoot};
use crate::report;

/// The exit code for a run that could not finish.
const FATAL: u8 = 2;

/// Runs `nova-dump` with `args` (without the program name) and returns its
/// exit code. The summary goes to `stdout`; usage and fatal errors go to
/// `stderr`.
pub fn run(
    args: impl IntoIterator<Item = OsString>,
    data: &impl DataSource,
    root: &impl OutputRoot,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    match cli::parse(args) {
        Ok(Command::Help) => match stdout.write_all(USAGE.as_bytes()) {
            Ok(()) => 0,
            Err(error) => fatal(stderr, format_args!("writing the usage: {error}")),
        },
        Ok(Command::Dump(options)) => dump(&options, data, root, stdout, stderr),
        Err(error) => fatal(stderr, format_args!("{error}\n\n{}", USAGE.trim_end())),
    }
}

fn dump(
    options: &Options,
    data: &impl DataSource,
    root: &impl OutputRoot,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let game = match data.open(&options.data, options.plugins.as_deref()) {
        Ok(game) => game,
        Err(error) => return fatal(stderr, error),
    };
    if let Some(message) = no_game_data(&game, &options.data, options.verbose) {
        return fatal(stderr, message);
    }
    let out = options.out.display();
    if let Err(error) = root.create(&options.out) {
        return fatal(
            stderr,
            format_args!("creating the output directory {out}: {error}"),
        );
    }
    match root.is_empty(&options.out) {
        Ok(true) => {}
        Ok(false) => {
            return fatal(
                stderr,
                format_args!("the output directory {out} is not empty"),
            );
        }
        Err(error) => {
            return fatal(
                stderr,
                format_args!("reading the output directory {out}: {error}"),
            );
        }
    }
    let outcome = match export(&game, &mut root.sink(&options.out)) {
        Ok(outcome) => outcome,
        Err(error) => return fatal(stderr, error),
    };
    match report::render(&outcome, options.verbose, stdout) {
        Ok(()) => report::exit_code(&outcome),
        Err(error) => fatal(stderr, format_args!("writing the summary: {error}")),
    }
}

/// Why the run must stop when the data directory yielded no resource file
/// at all, or `None` when it did.
///
/// A file that was found but failed to load counts as data: it is a listed
/// failure (exit 1), not a wrong directory. Plug-ins never count: they
/// layer over the game data and cannot replace it. The message counts the
/// entries the data directory itself skipped, and lists them when
/// `verbose`.
fn no_game_data(game: &GameData, data: &Path, verbose: bool) -> Option<String> {
    let from_data = |origin: Origin| origin == Origin::Data;
    if game.files().iter().any(|file| from_data(file.origin))
        || game.failed().iter().any(|file| from_data(file.origin))
    {
        return None;
    }
    let ignored: Vec<_> = game
        .ignored()
        .iter()
        .filter(|entry| entry.path.parent() == Some(data))
        .collect();
    let mut message = format!(
        "no game data found in {}; pass the 'Nova Files' directory",
        data.display()
    );
    let count = match ignored.len() {
        0 => return Some(message),
        1 => "1 entry ignored".to_owned(),
        n => format!("{n} entries ignored"),
    };
    if verbose {
        let _ = write!(message, " ({count})");
        for entry in ignored {
            let reason = report::ignore_reason(&entry.reason);
            let _ = write!(message, "\n  {}: {reason}", entry.path.display());
        }
    } else {
        let _ = write!(message, " ({count}, use --verbose to list them)");
    }
    Some(message)
}

/// Prints `message` to `stderr` and returns the fatal exit code. A failure
/// to print is ignored: there is nowhere left to report it.
fn fatal(stderr: &mut impl Write, message: impl Display) -> u8 {
    let _ = writeln!(stderr, "nova-dump: {message}");
    FATAL
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::ffi::OsString;
    use std::io;
    use std::path::Path;
    use std::rc::Rc;

    use nova_data::Record;
    use nova_data::records::ship::Ship;
    use nova_data::records::spin::Spin;
    use nova_data::sound::fixture::{Header, SndBuilder, SndFormat};
    use nova_data::store::{GameData, OpenError};
    use nova_rsrc::ResType;

    use super::*;
    use crate::cli::USAGE;
    use crate::ports::{DataSource, OutputRoot};
    use crate::testutil::{MemFs, MemSink, fork, native, record};

    /// What the fake output directory holds before the run.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum RootState {
        Missing,
        Empty,
        NonEmpty,
        CannotCreate,
        CannotList,
    }

    /// An output directory in memory; records the calls made on it.
    struct FakeRoot {
        state: Cell<RootState>,
        calls: RefCell<Vec<String>>,
        sink: Rc<RefCell<MemSink>>,
    }

    impl FakeRoot {
        fn new(state: RootState) -> Self {
            Self {
                state: Cell::new(state),
                calls: RefCell::default(),
                sink: Rc::default(),
            }
        }

        fn failing_write(state: RootState, fail_at: usize) -> Self {
            let root = Self::new(state);
            root.sink.borrow_mut().fail_at = Some(fail_at);
            root
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }

        fn files(&self) -> Vec<String> {
            self.sink.borrow().paths()
        }
    }

    impl OutputRoot for FakeRoot {
        type Sink = Rc<RefCell<MemSink>>;

        fn create(&self, root: &Path) -> io::Result<()> {
            self.calls
                .borrow_mut()
                .push(format!("create {}", root.display()));
            match self.state.get() {
                RootState::CannotCreate => Err(io::Error::other("read-only volume")),
                RootState::Missing => {
                    self.state.set(RootState::Empty);
                    Ok(())
                }
                _ => Ok(()),
            }
        }

        fn is_empty(&self, root: &Path) -> io::Result<bool> {
            self.calls
                .borrow_mut()
                .push(format!("is_empty {}", root.display()));
            match self.state.get() {
                RootState::Missing => Err(io::Error::from(io::ErrorKind::NotFound)),
                RootState::CannotList => Err(io::Error::other("permission denied")),
                RootState::NonEmpty => Ok(false),
                RootState::Empty | RootState::CannotCreate => Ok(true),
            }
        }

        fn sink(&self, root: &Path) -> Self::Sink {
            self.calls
                .borrow_mut()
                .push(format!("sink {}", root.display()));
            Rc::clone(&self.sink)
        }
    }

    /// A data directory that cannot be listed.
    struct NoData;

    impl DataSource for NoData {
        fn open(&self, data: &Path, _plugins: Option<&Path>) -> Result<GameData, OpenError> {
            Err(OpenError::DataDir {
                path: data.to_path_buf(),
                source: io::Error::from(io::ErrorKind::NotFound),
            })
        }
    }

    /// A writer that always fails.
    struct FailingWriter;

    impl io::Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("broken pipe"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct Run {
        code: u8,
        stdout: String,
        stderr: String,
    }

    fn run_with(args: &[&str], data: &impl DataSource, root: &FakeRoot) -> Run {
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let code = run(
            args.iter().map(OsString::from),
            data,
            root,
            &mut stdout,
            &mut stderr,
        );
        Run {
            code,
            stdout: String::from_utf8(stdout).expect("UTF-8"),
            stderr: String::from_utf8(stderr).expect("UTF-8"),
        }
    }

    fn spin() -> Vec<u8> {
        record(12, &[(8, 2)])
    }

    /// A data directory with one good file.
    fn good_data() -> MemFs {
        MemFs::new().file("/data/Nova Data", fork(&[(Spin::TYPE, 200, None, spin())]))
    }

    /// Acceptance scenario: a plug-in with one truncated `shïp` in a valid
    /// fork, and a plug-in whose resource map lies past its end.
    fn broken_plugins() -> MemFs {
        let mut broken = fork(&[(Spin::TYPE, 201, None, spin())]);
        broken.truncate(20);
        good_data()
            .file(
                "/plugins/Bad Ship",
                fork(&[
                    (Ship::TYPE, 128, Some("Shuttle"), vec![0; 400]),
                    (Spin::TYPE, 202, None, spin()),
                ]),
            )
            .file("/plugins/Broken", broken)
    }

    const DUMP_WITH_PLUGINS: &[&str] = &["/data", "/out", "--plugins", "/plugins"];

    #[test]
    fn help_prints_the_usage_and_touches_nothing() {
        let root = FakeRoot::new(RootState::NonEmpty);
        let run = run_with(&["data", "-h"], &NoData, &root);
        assert_eq!(run.code, 0);
        assert_eq!(run.stdout, USAGE);
        assert_eq!(run.stderr, "");
        assert!(root.calls().is_empty());
    }

    #[test]
    fn a_usage_error_exits_2_with_the_usage_on_stderr() {
        let root = FakeRoot::new(RootState::Empty);
        let run = run_with(&["data"], &good_data(), &root);
        assert_eq!(run.code, 2);
        assert_eq!(run.stdout, "");
        assert_eq!(
            run.stderr,
            format!("nova-dump: missing the output directory\n\n{USAGE}")
        );
        assert!(root.calls().is_empty());
    }

    #[test]
    fn data_that_cannot_be_opened_exits_2_before_touching_the_output() {
        let root = FakeRoot::new(RootState::Missing);
        let run = run_with(&["/nowhere", "/out"], &NoData, &root);
        assert_eq!(run.code, 2);
        assert_eq!(run.stdout, "");
        assert_eq!(
            run.stderr,
            format!(
                "nova-dump: listing the data directory /nowhere: {}\n",
                io::Error::from(io::ErrorKind::NotFound)
            )
        );
        assert!(root.calls().is_empty());
    }

    /// The folder that contains `Nova Files`, passed in its place.
    fn parent_of_nova_files() -> MemFs {
        MemFs::new().file(
            "/data/Nova Files/Nova Data",
            fork(&[(Spin::TYPE, 200, None, spin())]),
        )
    }

    #[test]
    fn a_data_directory_with_no_game_data_exits_2_before_touching_the_output() {
        let root = FakeRoot::new(RootState::Missing);
        let run = run_with(&["/data", "/out"], &parent_of_nova_files(), &root);
        assert_eq!(run.code, 2);
        assert_eq!(run.stdout, "");
        assert_eq!(
            run.stderr,
            "nova-dump: no game data found in /data; pass the 'Nova Files' directory \
             (1 entry ignored, use --verbose to list them)\n"
        );
        assert!(root.calls().is_empty());
        assert!(root.files().is_empty());
    }

    #[test]
    fn an_empty_data_directory_has_no_game_data() {
        let root = FakeRoot::new(RootState::Empty);
        let run = run_with(&["/data", "/out"], &MemFs::new(), &root);
        assert_eq!(run.code, 2);
        assert_eq!(
            run.stderr,
            "nova-dump: no game data found in /data; pass the 'Nova Files' directory\n"
        );
        assert!(root.calls().is_empty());
    }

    #[test]
    fn verbose_lists_what_the_data_directory_ignored() {
        let data = parent_of_nova_files().file("/data/.DS_Store", vec![]);
        let root = FakeRoot::new(RootState::Empty);
        let run = run_with(&["/data", "/out", "-v"], &data, &root);
        assert_eq!(run.code, 2);
        assert_eq!(
            run.stderr,
            format!(
                "nova-dump: no game data found in /data; pass the 'Nova Files' directory \
                 (2 entries ignored)\n  \
                 {}: hidden\n  \
                 {}: folder inside the data directory\n",
                native("/data/.DS_Store"),
                native("/data/Nova Files"),
            )
        );
    }

    #[test]
    fn plug_ins_alone_are_not_game_data() {
        let data = parent_of_nova_files()
            .file("/plugins/Extra", fork(&[(Spin::TYPE, 300, None, spin())]))
            .file("/plugins/.DS_Store", vec![]);
        let root = FakeRoot::new(RootState::Empty);
        let run = run_with(DUMP_WITH_PLUGINS, &data, &root);
        assert_eq!(run.code, 2);
        // Only the data directory's own ignored entries are counted.
        assert_eq!(
            run.stderr,
            "nova-dump: no game data found in /data; pass the 'Nova Files' directory \
             (1 entry ignored, use --verbose to list them)\n"
        );
        assert!(root.calls().is_empty());
    }

    #[test]
    fn a_data_file_that_fails_to_load_is_a_failure_not_missing_data() {
        let mut broken = fork(&[(Spin::TYPE, 200, None, spin())]);
        broken.truncate(20);
        let data = MemFs::new().file("/data/Nova Data", broken);
        let root = FakeRoot::new(RootState::Empty);
        let run = run_with(&["/data", "/out"], &data, &root);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert_eq!(run.stderr, "");
        let lines: Vec<&str> = run.stdout.lines().collect();
        let at = lines
            .iter()
            .position(|l| *l == "Failures:")
            .expect("a list");
        let [file, total] = &lines[at + 1..] else {
            panic!("{}", run.stdout)
        };
        assert!(
            file.starts_with(&format!(
                "  file {}: not a valid resource fork: ",
                native("/data/Nova Data")
            )),
            "{file}"
        );
        assert_eq!(*total, "failures: 1");
        assert_eq!(root.calls(), ["create /out", "is_empty /out", "sink /out"]);
    }

    #[test]
    fn a_missing_or_empty_output_directory_gets_the_dump() {
        for state in [RootState::Missing, RootState::Empty] {
            let root = FakeRoot::new(state);
            let run = run_with(&["/data", "/out"], &good_data(), &root);
            assert_eq!(run.code, 0, "{state:?}: {}", run.stderr);
            assert_eq!(run.stderr, "");
            assert!(
                run.stdout.starts_with("records: 1 in 1 JSON files\n"),
                "{}",
                run.stdout
            );
            assert!(run.stdout.ends_with("\nfailures: 0\n"), "{}", run.stdout);
            assert_eq!(root.calls(), ["create /out", "is_empty /out", "sink /out"]);
            assert_eq!(root.files(), [native("json/spïn.json")]);
        }
    }

    #[test]
    fn a_non_empty_output_directory_is_refused() {
        let root = FakeRoot::new(RootState::NonEmpty);
        let run = run_with(&["/data", "/out"], &good_data(), &root);
        assert_eq!(run.code, 2);
        assert_eq!(run.stdout, "");
        assert_eq!(
            run.stderr,
            "nova-dump: the output directory /out is not empty\n"
        );
        assert_eq!(root.calls(), ["create /out", "is_empty /out"]);
        assert!(root.files().is_empty());
    }

    #[test]
    fn an_output_directory_that_cannot_be_prepared_exits_2() {
        let root = FakeRoot::new(RootState::CannotCreate);
        let run = run_with(&["/data", "/out"], &good_data(), &root);
        assert_eq!(run.code, 2);
        assert_eq!(
            run.stderr,
            "nova-dump: creating the output directory /out: read-only volume\n"
        );
        assert_eq!(root.calls(), ["create /out"]);

        let root = FakeRoot::new(RootState::CannotList);
        let run = run_with(&["/data", "/out"], &good_data(), &root);
        assert_eq!(run.code, 2);
        assert_eq!(
            run.stderr,
            "nova-dump: reading the output directory /out: permission denied\n"
        );
        assert_eq!(root.calls(), ["create /out", "is_empty /out"]);
        assert_eq!(run.stdout, "");
    }

    #[test]
    fn a_write_error_exits_2() {
        let root = FakeRoot::failing_write(RootState::Empty, 1);
        let run = run_with(&["/data", "/out"], &good_data(), &root);
        assert_eq!(run.code, 2);
        assert_eq!(run.stdout, "");
        assert_eq!(
            run.stderr,
            format!(
                "nova-dump: writing {}: disk full\n",
                native("json/spïn.json")
            )
        );
    }

    #[test]
    fn a_summary_that_cannot_be_printed_exits_2() {
        let root = FakeRoot::new(RootState::Empty);
        let mut stderr = Vec::new();
        let code = run(
            ["/data", "/out"].map(OsString::from),
            &good_data(),
            &root,
            &mut FailingWriter,
            &mut stderr,
        );
        assert_eq!(code, 2);
        assert_eq!(
            String::from_utf8(stderr).expect("UTF-8"),
            "nova-dump: writing the summary: broken pipe\n"
        );
    }

    #[test]
    fn a_usage_that_cannot_be_printed_exits_2() {
        let mut stderr = Vec::new();
        let root = FakeRoot::new(RootState::Empty);
        let code = run(
            ["--help"].map(OsString::from),
            &NoData,
            &root,
            &mut FailingWriter,
            &mut stderr,
        );
        assert_eq!(code, 2);
        assert_eq!(
            String::from_utf8(stderr).expect("UTF-8"),
            "nova-dump: writing the usage: broken pipe\n"
        );
    }

    #[test]
    fn a_corrupt_record_and_an_unreadable_plug_in_are_both_listed() {
        let root = FakeRoot::new(RootState::Empty);
        let run = run_with(DUMP_WITH_PLUGINS, &broken_plugins(), &root);
        assert_eq!(run.code, 1, "{}", run.stderr);
        assert_eq!(run.stderr, "");

        let lines: Vec<&str> = run.stdout.lines().collect();
        let at = lines
            .iter()
            .position(|l| *l == "Failures:")
            .expect("a list");
        let [ship, file, total] = &lines[at + 1..] else {
            panic!("{}", run.stdout)
        };
        assert!(
            ship.starts_with(&format!(
                "  shïp 128 \"Shuttle\" in {}: unexpected end of data at byte 0x",
                native("/plugins/Bad Ship")
            )),
            "{ship}"
        );
        assert!(
            ship.ends_with("(record is 400 bytes, layout needs 1860)"),
            "{ship}"
        );
        assert!(
            file.starts_with(&format!(
                "  file {}: not a valid resource fork: ",
                native("/plugins/Broken")
            )),
            "{file}"
        );
        assert_eq!(*total, "failures: 2");
        // The rest is still written: both good spïns and the (empty) shïp file.
        assert_eq!(
            root.files(),
            [native("json/shïp.json"), native("json/spïn.json")]
        );
        assert!(run.stdout.starts_with("records: 2 in 2 JSON files\n"));
    }

    #[test]
    fn a_plug_in_sound_too_slow_for_wav_is_listed_and_not_written() {
        let slow = SndBuilder::new(
            SndFormat::Two,
            Header::Standard {
                rate: 1,
                loop_points: (0, 0),
                base_note: 60,
                samples: vec![0x80, 0xFF],
            },
        )
        .bytes();
        let data = good_data().file(
            "/plugins/Slow",
            fork(&[(ResType::new(*b"snd "), 500, Some("drone"), slow)]),
        );
        let root = FakeRoot::new(RootState::Empty);
        let run = run_with(DUMP_WITH_PLUGINS, &data, &root);
        assert_eq!(run.code, 1, "{}", run.stdout);
        assert_eq!(run.stderr, "");
        let lines: Vec<&str> = run.stdout.lines().collect();
        let at = lines
            .iter()
            .position(|l| *l == "Failures:")
            .expect("a list");
        assert_eq!(
            lines[at + 1..],
            [
                format!(
                    "  snd  500 \"drone\" in {}: sample rate 0x00000001 (16.16) is below \
                     0.5 Hz, too low for a WAV file",
                    native("/plugins/Slow")
                ),
                "failures: 1".to_owned(),
            ],
            "{}",
            run.stdout
        );
        assert!(run.stdout.contains("sounds: 0 snd\n"), "{}", run.stdout);
        assert_eq!(root.files(), [native("json/spïn.json")]);
    }

    #[test]
    fn verbose_adds_the_field_path_and_a_hex_window() {
        let root = FakeRoot::new(RootState::Empty);
        let mut args = DUMP_WITH_PLUGINS.to_vec();
        args.push("--verbose");
        let run = run_with(&args, &broken_plugins(), &root);
        assert_eq!(run.code, 1);
        let lines: Vec<&str> = run.stdout.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.starts_with("  shïp 128"))
            .expect("the ship failure");
        assert!(
            lines[at + 1].starts_with("    field: Ship → "),
            "{}",
            run.stdout
        );
        assert!(
            lines[at + 2].starts_with("    offset: 0x"),
            "{}",
            run.stdout
        );
        assert!(lines[at + 3].starts_with("    0x0"), "{}", run.stdout);
        assert!(
            lines[at + 3..at + 7].iter().any(|row| row.contains('[')),
            "{}",
            run.stdout
        );
        assert!(
            lines[at + 3..]
                .iter()
                .any(|l| l.starts_with(&format!("  file {}: ", native("/plugins/Broken")))),
            "{}",
            run.stdout
        );
    }

    #[test]
    fn plug_ins_are_passed_to_the_data_source() {
        let root = FakeRoot::new(RootState::Empty);
        let data = good_data().file(
            "/elsewhere/Extra",
            fork(&[(ResType::new(*b"XYZW"), 1, None, vec![])]),
        );
        let run = run_with(&["/data", "/out", "--plugins", "/elsewhere"], &data, &root);
        assert!(
            run.stdout.contains("not exported: XYZW (1)"),
            "{}",
            run.stdout
        );
        let without = run_with(&["/data", "/out2"], &data, &FakeRoot::new(RootState::Empty));
        assert!(!without.stdout.contains("XYZW"), "{}", without.stdout);
    }
}
