//! `nova-dump --pin-ids` over the real adapters, on a temporary copy of
//! nova-data's sample fixture (`new_game::fixture::sample_folder`).

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use nova_data::new_game::{self, pins};
use nova_dump::app::run;
use nova_dump::fs::{StdDataSource, StdGameFolder, StdOutputRoot};

/// Copies the folder `from` into `to`, recursively.
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("creates");
    for entry in fs::read_dir(from).expect("lists") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a type").is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copies");
        }
    }
}

/// Runs `nova-dump --pin-ids dir`: its exit code and standard output.
fn pin(dir: &Path) -> (u8, String) {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let args: Vec<OsString> = vec!["--pin-ids".into(), dir.into()];
    let code = run(
        args,
        &StdDataSource,
        &StdOutputRoot,
        &StdGameFolder,
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(String::from_utf8_lossy(&stderr), "");
    (code, String::from_utf8(stdout).expect("UTF-8"))
}

#[test]
fn pinning_the_sample_writes_its_ids_and_a_second_run_changes_nothing() {
    let temp = tempfile::tempdir().expect("a temporary directory");
    let dir = temp.path().join("sample");
    copy(&new_game::fixture::sample_folder(), &dir);
    let table = dir.join(pins::FILE);
    if table.exists() {
        fs::remove_file(&table).expect("removes the checked-in table");
    }
    let loaded = new_game::open(&dir).expect("lists").ids;

    let (code, stdout) = pin(&dir);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.starts_with("pinned "), "{stdout}");
    let written = fs::read(&table).expect("written");
    assert_eq!(pins::parse(&written).expect("parses").pins, loaded);

    let (code, stdout) = pin(&dir);
    assert_eq!(code, 0);
    assert_eq!(stdout, format!("{} is up to date\n", table.display()));
    assert_eq!(fs::read(&table).expect("still there"), written);
    let reopened = new_game::open(&dir).expect("lists");
    assert_eq!(reopened.ids, loaded);
    assert_eq!(reopened.problems.len(), 0, "{:?}", reopened.problems);
}
