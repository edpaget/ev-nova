//! The usual places on Linux, and on any other system that keeps the XDG
//! base directories (`Os::Other`): the Windows release in a Wine prefix,
//! or a copied `Nova Files` folder.
//!
//! Where these come from (Wine's default prefix, Lutris's install script,
//! the launchers left out and why) is recorded in the Findings of
//! rdm:phase/linux-platform/phase-2-find-nova-files-on-linux.

use std::ffi::OsString;
use std::path::PathBuf;

use super::{installs_in, nova_files_in};
use crate::config::xdg_dir;

/// The Lutris prefix's folder under `~/Games`: its install script's game
/// slug, which Lutris names the prefix after by default.
const LUTRIS_PREFIX: &str = "ev-nova";

/// Linux's usual places, from `env`, in the order they are tried:
///
/// 1. The `Nova Files` of the Windows release in each Wine prefix:
///    `$WINEPREFIX` (when set, non-empty and rooted), then Wine's default
///    `$HOME/.wine`, then Lutris's `$HOME/Games/ev-nova`. In each, an
///    `EV Nova` folder in `drive_c/Program Files`, then
///    `drive_c/Program Files (x86)`, then under `Ambrosia Software` in
///    each, as on Windows. A place named twice (`WINEPREFIX` is
///    `~/.wine`) is tried once.
/// 2. `$XDG_DATA_HOME/nova/Nova Files`, or
///    `$HOME/.local/share/nova/Nova Files` without an absolute
///    `XDG_DATA_HOME`: a copy kept with the game's other data.
/// 3. `$HOME/Games/EV Nova/Nova Files`: a copy kept with other games.
///
/// Without `HOME`, only the places from absolute variables remain.
/// Bottles' and Proton's prefixes are named by the player (a bottle) or
/// generated (an appid), so they aren't looked for: the open panel asks
/// once, and the choice is remembered.
pub(super) fn linux_places(env: impl Fn(&str) -> Option<OsString>) -> Vec<PathBuf> {
    let var = |name: &str| {
        env(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let home = var("HOME");
    let prefixes = var("WINEPREFIX")
        .filter(|prefix| prefix.has_root())
        .into_iter()
        .chain(
            home.iter()
                .flat_map(|home| [home.join(".wine"), home.join("Games").join(LUTRIS_PREFIX)]),
        );
    let installs = prefixes.flat_map(|prefix| {
        let drive_c = prefix.join("drive_c");
        installs_in(&[
            drive_c.join("Program Files"),
            drive_c.join("Program Files (x86)"),
        ])
    });
    let mut places = nova_files_in(installs);
    places.extend(
        xdg_dir(&env, "XDG_DATA_HOME", &[".local", "share"])
            .map(|data| data.join("nova").join("Nova Files")),
    );
    places.extend(home.map(|home| {
        let mut dir = home;
        dir.extend(["Games", "EV Nova", "Nova Files"]);
        dir
    }));
    places
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::super::tests::{Picks, ce_install, data_file, env, rez_file};
    use super::super::{DiskData, Remember, Via, locate};
    use super::*;

    /// The four `Nova Files` the Windows release may have in the Wine
    /// prefix at `prefix`, in the order they are tried.
    fn wine(prefix: &Path) -> Vec<PathBuf> {
        let drive_c = prefix.join("drive_c");
        let mut places = Vec::new();
        for vendor in [&[][..], &["Ambrosia Software"][..]] {
            for root in ["Program Files", "Program Files (x86)"] {
                let mut dir = drive_c.join(root);
                dir.extend(vendor);
                dir.extend(["EV Nova", "Nova Files"]);
                places.push(dir);
            }
        }
        places
    }

    fn under(base: &str, parts: &[&str]) -> PathBuf {
        let mut dir = PathBuf::from(base);
        dir.extend(parts);
        dir
    }

    /// The places that follow the Wine prefixes, for a home of `home` and
    /// no `XDG_DATA_HOME`.
    fn copies(home: &str) -> Vec<PathBuf> {
        vec![
            under(home, &[".local", "share", "nova", "Nova Files"]),
            under(home, &["Games", "EV Nova", "Nova Files"]),
        ]
    }

    #[test]
    fn the_wine_prefixes_come_first_then_the_copied_folders() {
        let expected: Vec<PathBuf> = [
            wine(Path::new("/prefix")),
            wine(&under("/home/p", &[".wine"])),
            wine(&under("/home/p", &["Games", "ev-nova"])),
            copies("/home/p"),
        ]
        .concat();
        assert_eq!(
            linux_places(env(&[("HOME", "/home/p"), ("WINEPREFIX", "/prefix")])),
            expected
        );
    }

    #[test]
    fn a_wine_prefix_that_is_the_default_one_is_tried_once() {
        let expected: Vec<PathBuf> = [
            wine(&under("/home/p", &[".wine"])),
            wine(&under("/home/p", &["Games", "ev-nova"])),
            copies("/home/p"),
        ]
        .concat();
        let default = under("/home/p", &[".wine"]);
        let default = default.to_str().expect("UTF-8");
        assert_eq!(
            linux_places(env(&[("HOME", "/home/p"), ("WINEPREFIX", default)])),
            expected
        );
    }

    #[test]
    fn an_empty_or_relative_wine_prefix_is_not_tried() {
        let expected: Vec<PathBuf> = [
            wine(&under("/home/p", &[".wine"])),
            wine(&under("/home/p", &["Games", "ev-nova"])),
            copies("/home/p"),
        ]
        .concat();
        for prefix in ["", "prefix"] {
            assert_eq!(
                linux_places(env(&[("HOME", "/home/p"), ("WINEPREFIX", prefix)])),
                expected,
                "{prefix:?}"
            );
        }
    }

    #[test]
    fn an_absolute_xdg_data_home_holds_the_copy() {
        let places = linux_places(env(&[("HOME", "/home/p"), ("XDG_DATA_HOME", "/data")]));
        assert_eq!(
            places[places.len() - 2..],
            [
                under("/data", &["nova", "Nova Files"]),
                under("/home/p", &["Games", "EV Nova", "Nova Files"]),
            ]
        );
    }

    #[test]
    fn an_empty_or_relative_xdg_data_home_falls_back_to_the_home() {
        for data in ["", "data"] {
            let places = linux_places(env(&[("HOME", "/home/p"), ("XDG_DATA_HOME", data)]));
            assert_eq!(places[places.len() - 2..], copies("/home/p"), "{data:?}");
        }
    }

    #[test]
    fn without_a_home_only_the_absolute_variables_places_remain() {
        let expected: Vec<PathBuf> = [
            wine(Path::new("/prefix")),
            vec![under("/data", &["nova", "Nova Files"])],
        ]
        .concat();
        for home in [None, Some("")] {
            let mut vars = vec![("WINEPREFIX", "/prefix"), ("XDG_DATA_HOME", "/data")];
            vars.extend(home.map(|home| ("HOME", home)));
            assert_eq!(linux_places(env(&vars)), expected, "{home:?}");
        }
    }

    #[test]
    fn with_nothing_set_there_are_no_places() {
        assert_eq!(linux_places(env(&[])), Vec::<PathBuf>::new());
    }

    /// A fake Linux environment whose home is `tmp/home`, with a Wine
    /// prefix at `tmp/prefix` and the XDG data folder at `tmp/data`.
    fn linux_env(tmp: &Path) -> impl Fn(&str) -> Option<OsString> + use<> {
        let vars = [
            ("HOME", tmp.join("home")),
            ("WINEPREFIX", tmp.join("prefix")),
            ("XDG_DATA_HOME", tmp.join("data")),
        ];
        move |name| {
            vars.iter()
                .find(|(var, _)| *var == name)
                .map(|(_, dir)| dir.clone().into_os_string())
        }
    }

    #[test]
    fn a_ce_install_in_the_default_wine_prefix_is_found_on_disk_without_asking() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let mut program_files_x86 = tmp.path().join("home");
        program_files_x86.extend([".wine", "drive_c", "Program Files (x86)"]);
        let nova_files = ce_install(&program_files_x86, &[("Nova Data 1.rez", rez_file())]);
        let usual = linux_places(linux_env(tmp.path()));
        let mut dialog = Picks::default();
        let located = locate(None, None, &usual, &DiskData, &mut dialog);
        let found = located.outcome.expect("found");
        assert_eq!((found.dir, found.via), (nova_files, Via::Usual));
        assert_eq!(found.data.files().len(), 1);
        assert_eq!(located.remember, Remember::Keep);
        assert_eq!(dialog.prompts, Vec::<String>::new());
    }

    #[test]
    fn an_ndat_copy_in_the_xdg_data_folder_is_found_on_disk() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let mut nova_files = tmp.path().join("data");
        nova_files.extend(["nova", "Nova Files"]);
        fs::create_dir_all(&nova_files).expect("makes the folders");
        fs::write(nova_files.join("Nova Data 1.ndat"), data_file()).expect("writes");
        let usual = linux_places(linux_env(tmp.path()));
        let mut dialog = Picks::default();
        let located = locate(None, None, &usual, &DiskData, &mut dialog);
        let found = located.outcome.expect("found");
        assert_eq!((found.dir, found.via), (nova_files, Via::Usual));
        assert_eq!(dialog.prompts, Vec::<String>::new());
    }

    #[test]
    fn a_prefix_whose_nova_files_hold_only_music_and_movies_is_skipped() {
        let tmp = tempfile::tempdir().expect("a temporary directory");
        let mut prefix_root = tmp.path().join("prefix");
        prefix_root.extend(["drive_c", "Program Files"]);
        ce_install(
            &prefix_root,
            &[
                ("Nova Music.mp3", b"ID3\x04\0\0\0\0\0\0".to_vec()),
                ("Race 1.mov", b"\0\0\0\x14ftypqt  ".to_vec()),
            ],
        );
        let mut lutris = tmp.path().join("home");
        lutris.extend(["Games", "ev-nova", "drive_c", "Program Files"]);
        let nova_files = ce_install(&lutris, &[("Nova Data 1.rez", rez_file())]);
        let usual = linux_places(linux_env(tmp.path()));
        let mut dialog = Picks::default();
        let located = locate(None, None, &usual, &DiskData, &mut dialog);
        let found = located.outcome.expect("found");
        assert_eq!((found.dir, found.via), (nova_files, Via::Usual));
        assert_eq!(dialog.prompts, Vec::<String>::new());
    }
}
