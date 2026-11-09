//! `--pin-ids <GAME_DATA>`: writes the new game's ID table, over the
//! [`GameFolder`] port.
//!
//! It loads the folder, merges the load's IDs into the table it holds
//! (`nova_data::new_game::pins::pin`) and replaces the table with the
//! result, so every record keeps its ID and each one added since gets
//! one. It is the only writer of the table: loading the game never writes
//! it.
//!
//! Exit codes: 0 when the table was written or was already up to date; 2
//! when the folder can't be listed, the table can't be read or written,
//! the table is one the loader ignores (it doesn't parse, or names an
//! unknown kind), or it holds entries the loader ignores (a duplicate ID,
//! a malformed key, an ID out of range). It refuses to write over either
//! kind of table, writing nothing: the first would lose every pin, and the
//! second would move each ignored entry's record to a new ID, so a pilot
//! saved at the old one would find something else. Refusing, rather than
//! writing with a warning, keeps the table's rule that no pin is ever
//! dropped or reused. It names each ignored entry and why; the author
//! fixes the table by hand (or moves it aside) and runs again.

use std::fmt::Display;
use std::io::Write;
use std::path::Path;

use nova_data::new_game::pins::{self, FILE};

use crate::ports::GameFolder;

/// The exit code for a run that could not finish.
const FATAL: u8 = 2;

/// Pins the IDs of the new game in `dir`, reporting to `stdout`, or to
/// `stderr` when it can't. Returns the exit code.
pub fn run(
    dir: &Path,
    folder: &impl GameFolder,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let path = dir.join(FILE);
    let path = path.display();
    let loaded = match folder.load(dir) {
        Ok(loaded) => loaded,
        Err(error) => return fatal(stderr, error),
    };
    let previous = match folder.table(dir) {
        Ok(previous) => previous,
        Err(error) => return fatal(stderr, format_args!("reading {path}: {error}")),
    };
    let pinned = match pins::pin(previous.as_deref(), &loaded) {
        Ok(pinned) => pinned,
        Err(error) => {
            return fatal(
                stderr,
                format_args!(
                    "refusing to replace {path}: {error}; fix it or move it aside, then run again"
                ),
            );
        }
    };
    if !pinned.dropped.is_empty() {
        return refuse_to_drop(stderr, &path, &pinned.dropped);
    }
    if previous.as_deref() == Some(pinned.text.as_bytes()) {
        return report(stdout, stderr, format_args!("{path} is up to date"));
    }
    if let Err(error) = folder.replace_table(dir, pinned.text.as_bytes()) {
        return fatal(stderr, format_args!("writing {path}: {error}"));
    }
    let records = if pinned.added == 1 {
        "record"
    } else {
        "records"
    };
    report(
        stdout,
        stderr,
        format_args!("pinned {} new {records} in {path}", pinned.added),
    )
}

/// Refuses to write a merge that drops the previous table's `dropped`
/// entries, naming each on `stderr`: their records would move, and a pilot
/// saved at an old ID would find something else there. Returns the fatal
/// exit code.
fn refuse_to_drop(stderr: &mut impl Write, path: &impl Display, dropped: &[String]) -> u8 {
    let (entries, records) = if dropped.len() == 1 {
        ("entry", "its record")
    } else {
        ("entries", "their records")
    };
    let _ = writeln!(
        stderr,
        "nova-dump: refusing to replace {path}: writing it would drop {} {entries} the loader \
         ignores, moving {records} to a new ID:",
        dropped.len()
    );
    for line in dropped {
        let _ = writeln!(stderr, "nova-dump:   {line}");
    }
    fatal(
        stderr,
        "fix the table by hand, or move it aside, then run again",
    )
}

/// Prints `message` to `stdout` and returns 0, or the fatal exit code when
/// it can't be printed.
fn report(stdout: &mut impl Write, stderr: &mut impl Write, message: impl Display) -> u8 {
    match writeln!(stdout, "{message}") {
        Ok(()) => 0,
        Err(error) => fatal(stderr, format_args!("writing the summary: {error}")),
    }
}

/// Prints `message` to `stderr` and returns the fatal exit code. A failure
/// to print is ignored: there is nowhere left to report it.
fn fatal(stderr: &mut impl Write, message: impl Display) -> u8 {
    let _ = writeln!(stderr, "nova-dump: {message}");
    FATAL
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use nova_data::new_game::IdTable;
    use nova_data::new_game::pins::{self, FILE};

    use super::*;
    use crate::testutil::FakeFolder;

    fn dir() -> PathBuf {
        Path::new("/game").to_path_buf()
    }

    /// The table's path as the messages show it.
    fn table_path() -> String {
        dir().join(FILE).display().to_string()
    }

    /// The table `json` holds.
    fn table(json: &str) -> IdTable {
        let table = pins::parse(json.as_bytes()).expect("parses");
        assert_eq!(table.problems, Vec::<String>::new());
        table.pins
    }

    /// Two systems.
    fn ids() -> IdTable {
        table(r#"{"systems": {"alpha": 128, "beta": 129}}"#)
    }

    struct Ran {
        code: u8,
        stdout: String,
        stderr: String,
    }

    fn pin_with(folder: &FakeFolder) -> Ran {
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let code = run(&dir(), folder, &mut stdout, &mut stderr);
        Ran {
            code,
            stdout: String::from_utf8(stdout).expect("UTF-8"),
            stderr: String::from_utf8(stderr).expect("UTF-8"),
        }
    }

    #[test]
    fn a_folder_with_no_table_gets_the_load_s_ids() {
        let folder = FakeFolder::default().with(&dir(), ids());
        let ran = pin_with(&folder);
        assert_eq!(ran.code, 0, "{}", ran.stderr);
        assert_eq!(ran.stderr, "");
        assert_eq!(
            ran.stdout,
            format!("pinned 2 new records in {}\n", table_path())
        );
        assert_eq!(folder.held(&dir()), Some(pins::text(&ids()).into_bytes()));
    }

    #[test]
    fn a_second_run_writes_nothing() {
        let folder = FakeFolder::default().with(&dir(), ids());
        pin_with(&folder);
        let ran = pin_with(&folder);
        assert_eq!(ran.code, 0);
        assert_eq!(ran.stdout, format!("{} is up to date\n", table_path()));
        assert_eq!(folder.writes(), [dir()], "the first run alone wrote");
    }

    #[test]
    fn a_record_added_since_is_pinned_and_a_retired_one_kept() {
        let previous = table(r#"{"systems": {"alpha": 128, "beta": 129, "gone": 130}}"#);
        let now = table(r#"{"systems": {"aaa-new": 131, "alpha": 128, "beta": 129}}"#);
        let folder = FakeFolder::default()
            .with(&dir(), now.clone())
            .table_of(&dir(), pins::text(&previous).as_bytes());
        let ran = pin_with(&folder);
        assert_eq!(
            ran.stdout,
            format!("pinned 1 new record in {}\n", table_path())
        );
        let held = folder.held(&dir()).expect("written");
        assert_eq!(
            pins::parse(&held).expect("parses").pins,
            table(r#"{"systems": {"aaa-new": 131, "alpha": 128, "beta": 129, "gone": 130}}"#)
        );
    }

    #[test]
    fn a_table_the_loader_ignores_is_never_written_over() {
        for (bytes, why) in [
            (&b"{not json"[..], "it doesn't parse as a table of IDs ("),
            (&br#"{"moons": {}}"#[..], "it names the kind `moons`"),
        ] {
            let folder = FakeFolder::default()
                .with(&dir(), ids())
                .table_of(&dir(), bytes);
            let ran = pin_with(&folder);
            assert_eq!(ran.code, 2);
            assert_eq!(ran.stdout, "");
            let prefix = format!("nova-dump: refusing to replace {}: {why}", table_path());
            assert!(ran.stderr.starts_with(&prefix), "{}", ran.stderr);
            assert!(
                ran.stderr
                    .ends_with("; fix it or move it aside, then run again\n"),
                "{}",
                ran.stderr
            );
            assert!(folder.writes().is_empty());
            assert_eq!(folder.held(&dir()), Some(bytes.to_vec()), "untouched");
        }
    }

    #[test]
    fn a_table_with_entries_the_loader_ignores_is_never_written_over() {
        // `beta` duplicates `alpha`'s ID, so the load numbered it afresh.
        let bytes = br#"{"systems": {"alpha": 128, "beta": 128}}"#;
        let loaded = table(r#"{"systems": {"alpha": 128, "beta": 129}}"#);
        let folder = FakeFolder::default()
            .with(&dir(), loaded)
            .table_of(&dir(), bytes);
        let ran = pin_with(&folder);
        assert_eq!(ran.code, 2);
        assert_eq!(ran.stdout, "");
        assert_eq!(
            ran.stderr,
            format!(
                "nova-dump: refusing to replace {}: writing it would drop 1 entry the loader \
                 ignores, moving its record to a new ID:\n\
                 nova-dump:   its systems entry `beta` is 128, which `systems/alpha` holds; \
                 it is ignored\n\
                 nova-dump: fix the table by hand, or move it aside, then run again\n",
                table_path()
            )
        );
        assert!(folder.writes().is_empty());
        assert_eq!(folder.held(&dir()), Some(bytes.to_vec()), "untouched");
    }

    #[test]
    fn every_dropped_entry_is_named() {
        let bytes = br#"{"systems": {"alpha": 128, "beta": 128, "gamma": 5000}}"#;
        let folder = FakeFolder::default()
            .with(&dir(), ids())
            .table_of(&dir(), bytes);
        let ran = pin_with(&folder);
        assert_eq!(ran.code, 2);
        assert!(
            ran.stderr
                .contains("would drop 2 entries the loader ignores, moving their records"),
            "{}",
            ran.stderr
        );
        for line in [
            "its systems entry `beta` is 128, which `systems/alpha` holds; it is ignored",
            "its systems entry `gamma` is 5000, outside 128 to 2175; it is ignored",
        ] {
            assert!(
                ran.stderr.contains(&format!("nova-dump:   {line}\n")),
                "{line}: {}",
                ran.stderr
            );
        }
        assert!(folder.writes().is_empty());
    }

    #[test]
    fn a_folder_that_doesn_t_list_exits_2() {
        let folder = FakeFolder::default();
        let ran = pin_with(&folder);
        assert_eq!(ran.code, 2);
        assert!(
            ran.stderr
                .starts_with(&format!("nova-dump: listing {}: ", dir().display())),
            "{}",
            ran.stderr
        );
        assert!(folder.writes().is_empty());
    }

    #[test]
    fn a_table_that_can_t_be_read_or_written_exits_2() {
        let folder = FakeFolder::default().with(&dir(), ids()).unreadable();
        let ran = pin_with(&folder);
        assert_eq!(ran.code, 2);
        assert_eq!(
            ran.stderr,
            format!("nova-dump: reading {}: permission denied\n", table_path())
        );
        assert!(folder.writes().is_empty());

        let folder = FakeFolder::default().with(&dir(), ids()).unwritable();
        let ran = pin_with(&folder);
        assert_eq!(ran.code, 2);
        assert_eq!(ran.stdout, "");
        assert_eq!(
            ran.stderr,
            format!("nova-dump: writing {}: read-only volume\n", table_path())
        );
    }

    /// A writer that always fails.
    struct FailingWriter;

    impl std::io::Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken pipe"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_summary_that_can_t_be_printed_exits_2_after_writing() {
        let folder = FakeFolder::default().with(&dir(), ids());
        let mut stderr = Vec::new();
        let code = run(&dir(), &folder, &mut FailingWriter, &mut stderr);
        assert_eq!(code, 2);
        assert_eq!(
            String::from_utf8(stderr).expect("UTF-8"),
            "nova-dump: writing the summary: broken pipe\n"
        );
        assert_eq!(folder.writes(), [dir()]);
    }
}
