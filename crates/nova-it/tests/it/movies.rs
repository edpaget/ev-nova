//! The game's movie player is wired to the decoder and the folders: a
//! movie in the Plug-Ins folder or the Nova Files starts, and one in
//! neither doesn't. The real decoder runs here, the edge crate's wiring
//! test; how movies play is `nova-movie`'s and `nova-view`'s to test.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use nova::config::Os;
use nova::movie::{movie_folders, movie_player};
use nova_movie::fixture;

/// An environment holding `vars`, and nothing else.
fn env(vars: Vec<(&'static str, PathBuf)>) -> impl Fn(&str) -> Option<OsString> {
    move |name| {
        vars.iter()
            .find(|(var, _)| *var == name)
            .map(|(_, value)| value.clone().into_os_string())
    }
}

/// An installed game's folders under a temporary directory: its Nova
/// Files, and the Plug-ins folder beside them.
struct Install {
    _root: tempfile::TempDir,
    nova_files: PathBuf,
    plug_ins: PathBuf,
}

fn install() -> Install {
    let root = tempfile::tempdir().expect("temp dir");
    let nova_files = root.path().join("Nova Files");
    let plug_ins = root.path().join("Plug-ins");
    fs::create_dir(&nova_files).expect("Nova Files");
    fs::create_dir(&plug_ins).expect("Plug-ins");
    Install {
        _root: root,
        nova_files,
        plug_ins,
    }
}

fn write_movie(dir: &Path) {
    fs::write(dir.join(fixture::NAME), fixture::MOVIE).expect("write the movie");
}

fn starts(folders: Vec<PathBuf>) -> Option<(u32, u32)> {
    let player = movie_player(folders);
    let size = player.start(fixture::NAME);
    player.stop();
    size
}

#[test]
fn a_movie_in_the_plug_ins_folder_starts() {
    let game = install();
    write_movie(&game.plug_ins);
    let folders = movie_folders(Os::Windows, env(Vec::new()), &game.nova_files);
    assert_eq!(starts(folders), Some(fixture::SIZE));
}

#[test]
fn a_movie_in_the_nova_files_starts() {
    let game = install();
    write_movie(&game.nova_files);
    let folders = movie_folders(Os::Windows, env(Vec::new()), &game.nova_files);
    assert_eq!(starts(folders), Some(fixture::SIZE));
}

#[test]
fn a_movie_in_the_macos_plug_ins_folder_starts() {
    let game = install();
    let home = tempfile::tempdir().expect("home");
    let mut plug_ins = home.path().to_path_buf();
    plug_ins.extend(["Library", "Application Support", "EV Nova", "Plug-Ins"]);
    fs::create_dir_all(&plug_ins).expect("Plug-Ins");
    write_movie(&plug_ins);
    let vars = env(vec![("HOME", home.path().to_path_buf())]);
    let folders = movie_folders(Os::MacOs, vars, &game.nova_files);
    assert_eq!(starts(folders), Some(fixture::SIZE));
}

#[test]
fn a_movie_in_neither_folder_doesn_t_start() {
    let game = install();
    let folders = movie_folders(Os::Windows, env(Vec::new()), &game.nova_files);
    assert_eq!(starts(folders), None);
}
