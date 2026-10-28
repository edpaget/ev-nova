//! Where startup errors and warnings go: always the startup log and
//! stderr and, when stderr is not a terminal (a double-clicked app, where
//! nobody sees it), an alert as well.
//!
//! [`Report`] decides, over three ports: [`Terminal`] (is stderr one?),
//! [`StartupLog`] (the log file) and [`FatalAlert`] (the platform's error
//! alert). The warnings for a window are kept and given once, as one alert
//! body, for the runner to show over the game window when it opens
//! ([`Report::take_alert`]). [`Fatal`] maps each fatal startup error to its
//! exit code and message.

use std::fs::{self, File};
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use crate::build_id::{BUILD_ID, describe};
use crate::cli::Usage;
use crate::exit::{Exit, OpenFailure};

use super::locate::{NotFound, NotLocated};

/// Says whether stderr is a terminal: the port the report decides through.
pub trait Terminal {
    /// Whether stderr is a terminal.
    fn stderr_is_terminal(&self) -> bool;
}

/// The process's real stderr.
#[derive(Clone, Copy, Debug, Default)]
pub struct StdTerminal;

impl Terminal for StdTerminal {
    fn stderr_is_terminal(&self) -> bool {
        io::stderr().is_terminal()
    }
}

/// The startup log: the port each startup line is written through.
pub trait StartupLog {
    /// Writes `line` as one line.
    ///
    /// # Errors
    ///
    /// When it could not be written.
    fn write_line(&mut self, line: &str) -> io::Result<()>;
}

/// Shows a fatal startup error in an alert: the platform's port.
pub trait FatalAlert {
    /// Shows `message`, and returns once the player has dismissed it.
    fn show_fatal(&mut self, message: &str);
}

/// Where the player sees the startup lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// On the terminal stderr is: nothing more is needed.
    Terminal,
    /// Nowhere, unless the game shows them in a window.
    Window,
}

/// The startup log's first line: the version and the build.
pub(crate) fn header(version: &str, build: Option<&str>) -> String {
    format!("nova {version} ({}) starting", describe(build))
}

/// The startup report: writes each line to the log and stderr, and for a
/// window keeps the warnings for one alert and shows a fatal error in
/// another. Writing never fails the game: a log that cannot be written is
/// said once, and dropped.
pub struct Report {
    shown: Shown,
    log: Option<Box<dyn StartupLog>>,
    stderr: Box<dyn Write>,
    kept: Vec<String>,
    taken: bool,
}

impl Report {
    /// A report that reads `terminal` once, writes to `log` (if any) and
    /// `stderr`, and starts the log with a header naming this version and
    /// build.
    #[must_use]
    pub fn new(
        terminal: &dyn Terminal,
        log: Option<Box<dyn StartupLog>>,
        stderr: Box<dyn Write>,
    ) -> Self {
        let shown = if terminal.stderr_is_terminal() {
            Shown::Terminal
        } else {
            Shown::Window
        };
        let mut report = Self {
            shown,
            log,
            stderr,
            kept: Vec::new(),
            taken: false,
        };
        report.log(&header(env!("CARGO_PKG_VERSION"), BUILD_ID));
        report
    }

    /// Where the player sees the lines.
    #[must_use]
    pub fn shown(&self) -> Shown {
        self.shown
    }

    /// Reports the warning `line`: to the log and stderr, and kept for the
    /// warnings alert when there is no terminal.
    pub fn warn(&mut self, line: &str) {
        self.log(line);
        self.say(line);
    }

    /// Reports the fatal error `message`: to the log and stderr and, when
    /// there is no terminal, in `alert`.
    pub fn fatal(&mut self, message: &str, alert: &mut dyn FatalAlert) {
        self.log(message);
        self.write_stderr(message);
        if self.shown == Shown::Window {
            alert.show_fatal(message);
        }
    }

    /// Reports `fatal` ([`Report::fatal`]) and gives the exit code it
    /// ends the program with. One that shows no alert
    /// ([`Fatal::shows_alert`]) still goes to the log and stderr.
    pub fn fail(&mut self, fatal: &Fatal, alert: &mut dyn FatalAlert) -> u8 {
        let exit = fatal.exit();
        if let Some(message) = &exit.message {
            if fatal.shows_alert() {
                self.fatal(message, alert);
            } else {
                self.log(message);
                self.write_stderr(message);
            }
        }
        exit.code
    }

    /// The warnings kept for the alert, one per line, the first time it is
    /// asked; `None` when there are none, from a terminal, and on every
    /// later call.
    pub fn take_alert(&mut self) -> Option<String> {
        if std::mem::replace(&mut self.taken, true) || self.kept.is_empty() {
            return None;
        }
        Some(self.kept.join("\n"))
    }

    /// Writes `line` to the log; one that fails is said, and the log is
    /// dropped.
    fn log(&mut self, line: &str) {
        let Some(log) = &mut self.log else {
            return;
        };
        if let Err(error) = log.write_line(line) {
            self.log = None;
            self.say(&format!("nova: cannot write the log: {error}"));
        }
    }

    /// Writes `line` to stderr, and keeps it for the alert when there is
    /// no terminal.
    fn say(&mut self, line: &str) {
        self.write_stderr(line);
        if self.shown == Shown::Window {
            self.kept.push(line.to_owned());
        }
    }

    /// Writes `line` to stderr, which nobody may be reading: an error is
    /// ignored.
    fn write_stderr(&mut self, line: &str) {
        let _ = writeln!(self.stderr, "{line}");
    }
}

/// The startup log on disk. The file is opened (its directory created,
/// and the last launch's lines truncated) when the first line is written,
/// so the log always describes one launch and never grows without bound.
#[derive(Debug)]
pub struct FileLog {
    path: PathBuf,
    file: Option<File>,
}

impl FileLog {
    /// The log at `path`, not yet opened.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path, file: None }
    }
}

impl StartupLog for FileLog {
    fn write_line(&mut self, line: &str) -> io::Result<()> {
        let file = if let Some(file) = &mut self.file {
            file
        } else {
            if let Some(dir) = self.path.parent() {
                fs::create_dir_all(dir)?;
            }
            self.file.insert(File::create(&self.path)?)
        };
        writeln!(file, "{line}")
    }
}

/// A startup error the game cannot go on from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fatal {
    /// The command line was not understood.
    Usage,
    /// No `Nova Files` were found or picked.
    NotFound(NotFound),
    /// None were found, and a quit arrived while the player was asked.
    Quit(NotFound),
    /// The `Nova Files` named could not be listed: why.
    Unreadable(String),
    /// The event loop could not be made, or failed: why.
    EventLoop(String),
    /// The window could not be opened.
    Open(OpenFailure),
}

impl From<NotLocated> for Fatal {
    fn from(not: NotLocated) -> Self {
        match not {
            NotLocated::NotFound(not_found) => Self::NotFound(not_found),
            NotLocated::Quit(not_found) => Self::Quit(not_found),
            NotLocated::Unreadable { error, .. } => Self::Unreadable(error),
        }
    }
}

impl Fatal {
    /// The fatal error that ends a run, if any: the event loop's `result`,
    /// else the runner's open `failure`.
    #[must_use]
    pub fn ending(result: Result<(), String>, failure: Option<&OpenFailure>) -> Option<Self> {
        match result {
            Err(error) => Some(Self::EventLoop(error)),
            Ok(()) => failure.cloned().map(Self::Open),
        }
    }

    /// Whether, with no terminal, the error is shown in an alert: every
    /// one but a quit. The player asked to quit, and a second modal would
    /// hold up the logout or Dock Quit that sent the quit, with no handler
    /// left to dismiss it.
    #[must_use]
    pub fn shows_alert(&self) -> bool {
        !matches!(self, Self::Quit(_))
    }

    /// The exit code and message: 2 for the usage error and for no data
    /// found (cancelled or quit while asking), as a usage error always
    /// was; 1 for the rest.
    #[must_use]
    pub fn exit(&self) -> Exit {
        let (code, message) = match self {
            Self::Usage => (2, Usage.to_string()),
            Self::NotFound(not_found) | Self::Quit(not_found) => (2, not_found.to_string()),
            Self::Unreadable(error) | Self::EventLoop(error) => (1, format!("nova: {error}")),
            Self::Open(failure) => return crate::exit::exit(Some(failure)),
        };
        Exit {
            code,
            message: Some(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::Path;
    use std::rc::Rc;

    use super::*;

    struct Says(bool);

    impl Terminal for Says {
        fn stderr_is_terminal(&self) -> bool {
            self.0
        }
    }

    /// A log that records its lines, or fails every write.
    #[derive(Clone, Default)]
    struct Lines {
        lines: Rc<RefCell<Vec<String>>>,
        fail: bool,
    }

    impl StartupLog for Lines {
        fn write_line(&mut self, line: &str) -> io::Result<()> {
            if self.fail {
                return Err(io::Error::other("the disk is full"));
            }
            self.lines.borrow_mut().push(line.to_owned());
            Ok(())
        }
    }

    /// A stderr whose bytes the test can read.
    #[derive(Clone, Default)]
    struct Stderr(Rc<RefCell<Vec<u8>>>);

    impl Stderr {
        fn text(&self) -> String {
            String::from_utf8(self.0.borrow().clone()).expect("UTF-8")
        }
    }

    impl Write for Stderr {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct Alerts(Vec<String>);

    impl FatalAlert for Alerts {
        fn show_fatal(&mut self, message: &str) {
            self.0.push(message.to_owned());
        }
    }

    fn header() -> String {
        super::header(env!("CARGO_PKG_VERSION"), BUILD_ID)
    }

    #[test]
    fn the_header_names_the_build_id() {
        assert_eq!(
            super::header("0.1.0", Some("20261028123456")),
            "nova 0.1.0 (build 20261028123456) starting"
        );
    }

    #[test]
    fn a_build_without_an_id_is_a_development_build() {
        assert_eq!(
            super::header("0.1.0", None),
            "nova 0.1.0 (development build) starting"
        );
    }

    fn report(terminal: bool, log: &Lines, stderr: &Stderr) -> Report {
        Report::new(
            &Says(terminal),
            Some(Box::new(log.clone())),
            Box::new(stderr.clone()),
        )
    }

    #[test]
    fn the_log_starts_with_a_header_naming_the_version_and_build() {
        let log = Lines::default();
        let stderr = Stderr::default();
        let mut report = report(true, &log, &stderr);
        report.warn("nova: one");
        assert_eq!(*log.lines.borrow(), [header(), "nova: one".to_owned()]);
        assert_eq!(stderr.text(), "nova: one\n", "the header is the log's only");
    }

    #[test]
    fn from_a_terminal_everything_stays_on_stderr_and_in_the_log() {
        let log = Lines::default();
        let stderr = Stderr::default();
        let mut report = report(true, &log, &stderr);
        assert_eq!(report.shown(), Shown::Terminal);
        let mut alerts = Alerts::default();
        report.warn("nova: a warning");
        report.fatal("nova: it failed", &mut alerts);
        assert_eq!(stderr.text(), "nova: a warning\nnova: it failed\n");
        assert_eq!(
            *log.lines.borrow(),
            [
                header(),
                "nova: a warning".to_owned(),
                "nova: it failed".to_owned()
            ]
        );
        assert_eq!(report.take_alert(), None);
        assert_eq!(alerts.0, Vec::<String>::new());
    }

    #[test]
    fn without_a_terminal_the_warnings_are_one_alert_given_once() {
        let log = Lines::default();
        let stderr = Stderr::default();
        let mut report = report(false, &log, &stderr);
        assert_eq!(report.shown(), Shown::Window);
        report.warn("nova: one");
        report.warn("nova: two");
        assert_eq!(stderr.text(), "nova: one\nnova: two\n");
        assert_eq!(
            *log.lines.borrow(),
            [header(), "nova: one".to_owned(), "nova: two".to_owned()]
        );
        assert_eq!(report.take_alert(), Some("nova: one\nnova: two".to_owned()));
        assert_eq!(report.take_alert(), None);
    }

    #[test]
    fn without_warnings_there_is_no_alert() {
        let mut report = report(false, &Lines::default(), &Stderr::default());
        assert_eq!(report.take_alert(), None);
    }

    #[test]
    fn the_alert_is_given_once_even_when_first_asked_with_nothing_kept() {
        let mut report = report(false, &Lines::default(), &Stderr::default());
        assert_eq!(report.take_alert(), None);
        report.warn("nova: late");
        assert_eq!(report.take_alert(), None);
    }

    #[test]
    fn without_a_terminal_a_fatal_error_is_shown_in_the_alert() {
        let log = Lines::default();
        let stderr = Stderr::default();
        let mut report = report(false, &log, &stderr);
        let mut alerts = Alerts::default();
        report.fatal("nova: it failed", &mut alerts);
        assert_eq!(alerts.0, ["nova: it failed"]);
        assert_eq!(stderr.text(), "nova: it failed\n");
        assert_eq!(
            *log.lines.borrow(),
            [header(), "nova: it failed".to_owned()]
        );
        assert_eq!(
            report.take_alert(),
            None,
            "the fatal error is not a warning"
        );
    }

    #[test]
    fn a_log_that_cannot_be_written_is_said_once_and_dropped() {
        let log = Lines {
            fail: true,
            ..Lines::default()
        };
        let stderr = Stderr::default();
        let mut report = report(false, &log, &stderr);
        report.warn("nova: one");
        report.fatal("nova: it failed", &mut Alerts::default());
        assert_eq!(
            stderr.text(),
            "nova: cannot write the log: the disk is full\nnova: one\nnova: it failed\n"
        );
        assert_eq!(
            report.take_alert(),
            Some("nova: cannot write the log: the disk is full\nnova: one".to_owned())
        );
    }

    #[test]
    fn with_no_log_the_lines_still_reach_stderr() {
        let stderr = Stderr::default();
        let mut report = Report::new(&Says(true), None, Box::new(stderr.clone()));
        report.warn("nova: one");
        assert_eq!(stderr.text(), "nova: one\n");
    }

    #[test]
    fn failing_reports_the_message_and_gives_the_exit_code() {
        let log = Lines::default();
        let stderr = Stderr::default();
        let mut report = report(false, &log, &stderr);
        let mut alerts = Alerts::default();
        assert_eq!(report.fail(&Fatal::Usage, &mut alerts), 2);
        assert_eq!(alerts.0, [Usage.to_string()]);
        assert_eq!(stderr.text(), format!("{Usage}\n"));
    }

    #[test]
    fn each_fatal_error_has_its_exit_code_and_message() {
        let not_found = NotFound {
            places: vec![PathBuf::from("/usual")],
        };
        let cases = [
            (Fatal::Usage, 2, Usage.to_string()),
            (Fatal::NotFound(not_found.clone()), 2, not_found.to_string()),
            (Fatal::Quit(not_found.clone()), 2, not_found.to_string()),
            (
                Fatal::Unreadable("listing the data directory /x: gone".to_owned()),
                1,
                "nova: listing the data directory /x: gone".to_owned(),
            ),
            (
                Fatal::EventLoop("no display".to_owned()),
                1,
                "nova: no display".to_owned(),
            ),
        ];
        for (fatal, code, message) in cases {
            assert_eq!(
                fatal.exit(),
                Exit {
                    code,
                    message: Some(message)
                },
                "{fatal:?}"
            );
        }
        let failure = OpenFailure::Window("no display".to_owned());
        assert_eq!(
            Fatal::Open(failure.clone()).exit(),
            crate::exit::exit(Some(&failure))
        );
    }

    #[test]
    fn a_lookup_that_found_nothing_is_fatal() {
        let not_found = NotFound { places: Vec::new() };
        assert_eq!(
            Fatal::from(NotLocated::NotFound(not_found.clone())),
            Fatal::NotFound(not_found)
        );
        assert_eq!(
            Fatal::from(NotLocated::Unreadable {
                dir: PathBuf::from("/x"),
                error: "gone".to_owned()
            }),
            Fatal::Unreadable("gone".to_owned())
        );
    }

    #[test]
    fn a_quit_while_asking_maps_from_not_located() {
        let not_found = NotFound {
            places: vec![PathBuf::from("/usual")],
        };
        assert_eq!(
            Fatal::from(NotLocated::Quit(not_found.clone())),
            Fatal::Quit(not_found)
        );
    }

    #[test]
    fn a_quit_while_asking_is_logged_and_written_but_never_alerted_even_without_a_terminal() {
        let log = Lines::default();
        let stderr = Stderr::default();
        let mut report = report(false, &log, &stderr);
        let mut alerts = Alerts::default();
        let not_found = NotFound {
            places: vec![PathBuf::from("/usual")],
        };
        let message = not_found.to_string();
        assert_eq!(report.fail(&Fatal::Quit(not_found), &mut alerts), 2);
        assert!(alerts.0.is_empty(), "{:?}", alerts.0);
        assert_eq!(stderr.text(), format!("{message}\n"));
        assert_eq!(*log.lines.borrow(), [header(), message]);
    }

    #[test]
    fn every_fatal_error_but_a_quit_shows_an_alert() {
        let not_found = NotFound { places: Vec::new() };
        assert!(Fatal::Usage.shows_alert());
        assert!(Fatal::NotFound(not_found.clone()).shows_alert());
        assert!(Fatal::Unreadable("gone".to_owned()).shows_alert());
        assert!(Fatal::EventLoop("lost".to_owned()).shows_alert());
        assert!(Fatal::Open(OpenFailure::NoGpu("none".to_owned())).shows_alert());
        assert!(!Fatal::Quit(not_found).shows_alert());
    }

    #[test]
    fn a_run_ends_fatally_on_a_loop_error_or_a_window_that_did_not_open() {
        let failure = OpenFailure::NoGpu("no adapter".to_owned());
        assert_eq!(Fatal::ending(Ok(()), None), None);
        assert_eq!(
            Fatal::ending(Ok(()), Some(&failure)),
            Some(Fatal::Open(failure.clone()))
        );
        assert_eq!(
            Fatal::ending(Err("lost".to_owned()), Some(&failure)),
            Some(Fatal::EventLoop("lost".to_owned()))
        );
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).expect("written")
    }

    #[test]
    fn the_file_log_creates_its_directory_and_writes_a_line_each() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("Logs").join("nova").join("nova.log");
        let mut log = FileLog::new(path.clone());
        log.write_line("one").expect("writes");
        log.write_line("two").expect("writes");
        assert_eq!(read(&path), "one\ntwo\n");
    }

    #[test]
    fn each_launch_truncates_the_last_launch_s_log() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("nova.log");
        let mut first = FileLog::new(path.clone());
        first
            .write_line("first launch, a long line")
            .expect("writes");
        drop(first);
        let mut second = FileLog::new(path.clone());
        second.write_line("second").expect("writes");
        assert_eq!(read(&path), "second\n");
    }

    #[test]
    fn a_file_log_that_cannot_be_created_fails_its_write() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let blocker = dir.path().join("Logs");
        fs::write(&blocker, "a file where the directory goes").expect("writes");
        let mut log = FileLog::new(blocker.join("nova.log"));
        assert!(log.write_line("one").is_err());
    }
}
