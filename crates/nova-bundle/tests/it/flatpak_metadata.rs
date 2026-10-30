//! What `flatpak info --show-permissions` lists for an app finished with
//! given `finish-args`, and a reader for its output. Pure, so it is
//! tested on every host; the `NOVA_FLATPAK` test (`linux_flatpak`, Linux
//! only) compares the two.

use std::collections::{BTreeMap, BTreeSet};

/// A flatpak's major and minor version.
pub type Version = (u32, u32);

/// The flatpak version `flatpak --version` prints (`Flatpak 1.16.6`).
pub fn flatpak_version(text: &str) -> Version {
    let number = text
        .split_whitespace()
        .nth(1)
        .unwrap_or_else(|| panic!("no version in {text:?}"));
    let mut parts = number.split('.').map(|part| {
        part.parse()
            .unwrap_or_else(|e| panic!("{number:?} in {text:?}: {e}"))
    });
    let mut part = || {
        parts
            .next()
            .unwrap_or_else(|| panic!("no minor version in {text:?}"))
    };
    (part(), part())
}

/// The first flatpak that stores `--socket=fallback-x11` alone.
///
/// Before it, flatpak stores `fallback-x11` together with `x11`, for
/// flatpaks older than `fallback-x11` itself (commit f33fac31, 2018):
/// reading the metadata, flatpak skips a socket name it doesn't know
/// (`flatpak_context_load_metadata` logs "Unknown socket type"), so those
/// still get X11. 1.16.6's `common/flatpak-context.c`,
/// `option_socket_cb`:
///
/// ```c
/// if (socket == FLATPAK_CONTEXT_SOCKET_FALLBACK_X11)
///   socket |= FLATPAK_CONTEXT_SOCKET_X11;
/// ```
///
/// and `flatpak-context.h` says of the flag "For backwards compat, also
/// set `SOCKET_X11`". The stored `x11` grants nothing more on a flatpak
/// that knows `fallback-x11`: 1.16.6's `common/flatpak-run-sockets.c`
/// decides X11 by `fallback-x11` alone when it is set
/// (`allow_x11 = !has_wayland`), so the sandbox gets X11 only where
/// there is no Wayland socket. 1.17.0 (commit c38e481f, "Handle
/// x11-fallback by converting to a conditional") stores each socket
/// name as given, so `fallback-x11` alone, and 1.18 keeps that.
///
/// The rule applies where `--socket` is parsed, in `flatpak
/// build-finish`, which writes the metadata; loading it applies no rule,
/// so `flatpak info` lists the metadata as stored. So the version that matters is the one that
/// finished the build: the host's, which `flatpak-builder` runs.
pub const FALLBACK_X11_ALONE: Version = (1, 17);

/// The permissions `args` (`finish-args`, each `--option=value`) store,
/// as `flatpak info --show-permissions` lists them on `flatpak`: each
/// `[Context]` key with its values, plus the `x11` socket that
/// `fallback-x11` implies before [`FALLBACK_X11_ALONE`].
pub fn expected_permissions(args: &[&str], flatpak: Version) -> BTreeMap<String, BTreeSet<String>> {
    let mut expected: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for arg in args {
        let (option, value) = arg
            .strip_prefix("--")
            .and_then(|a| a.split_once('='))
            .unwrap_or_else(|| panic!("{arg} is --option=value"));
        let key = match option {
            "socket" => "sockets",
            "share" => "shared",
            "device" => "devices",
            "filesystem" => "filesystems",
            "persist" => "persistent",
            other => panic!("no [Context] key for --{other}"),
        };
        let values = expected.entry(key.to_owned()).or_default();
        values.insert(value.to_owned());
        if key == "sockets" && value == "fallback-x11" && flatpak < FALLBACK_X11_ALONE {
            values.insert("x11".to_owned());
        }
    }
    expected
}

/// `flatpak info --show-permissions`'s key file, read as each group's
/// keys with their `;`-separated values.
pub fn permissions(text: &str) -> BTreeMap<String, BTreeMap<String, BTreeSet<String>>> {
    let mut groups: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    let mut group = String::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            name.clone_into(&mut group);
            groups.entry(group.clone()).or_default();
        } else if let Some((key, values)) = line.split_once('=') {
            let values = values
                .split(';')
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect();
            groups
                .entry(group.clone())
                .or_default()
                .insert(key.to_owned(), values);
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use nova_bundle::linux::flatpak::FINISH_ARGS;

    use super::*;

    /// What flatpak 1.16.6 (Ubuntu's, on CI's `ubuntu-24.04`) listed for
    /// the installed bundle.
    const SHOWN_BY_1_16: &str = "[Context]
shared=ipc;
sockets=x11;wayland;pulseaudio;fallback-x11;
devices=dri;
filesystems=~/.wine:ro;~/Games/EV Nova:ro;~/Games/ev-nova:ro;
persistent=.local/state;
";

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|&v| v.to_owned()).collect()
    }

    #[test]
    fn reads_the_version_flatpak_prints() {
        assert_eq!(flatpak_version("Flatpak 1.16.6\n"), (1, 16));
        assert_eq!(flatpak_version("Flatpak 1.17.0"), (1, 17));
        assert_eq!(flatpak_version("Flatpak 2.0"), (2, 0));
    }

    #[test]
    fn the_finish_args_are_what_flatpak_1_16_lists_for_the_installed_bundle() {
        assert_eq!(
            permissions(SHOWN_BY_1_16).get("Context"),
            Some(&expected_permissions(&FINISH_ARGS, (1, 16)))
        );
    }

    #[test]
    fn before_1_17_fallback_x11_is_stored_with_x11() {
        for flatpak in [(0, 11), (1, 12), (1, 16)] {
            assert_eq!(
                expected_permissions(&["--socket=wayland", "--socket=fallback-x11"], flatpak)["sockets"],
                set(&["wayland", "fallback-x11", "x11"]),
                "{flatpak:?}"
            );
        }
    }

    #[test]
    fn from_1_17_fallback_x11_is_stored_alone() {
        for flatpak in [(1, 17), (1, 18), (2, 0)] {
            assert_eq!(
                expected_permissions(&["--socket=wayland", "--socket=fallback-x11"], flatpak)["sockets"],
                set(&["wayland", "fallback-x11"]),
                "{flatpak:?}"
            );
        }
    }

    #[test]
    fn other_sockets_imply_nothing() {
        assert_eq!(
            expected_permissions(&["--socket=wayland", "--socket=pulseaudio"], (1, 16)),
            BTreeMap::from([("sockets".to_owned(), set(&["wayland", "pulseaudio"]))])
        );
    }

    #[test]
    fn maps_each_option_to_its_context_key() {
        assert_eq!(
            expected_permissions(
                &[
                    "--share=ipc",
                    "--device=dri",
                    "--filesystem=~/Games/EV Nova:ro",
                    "--persist=.local/state",
                ],
                (1, 16)
            ),
            BTreeMap::from([
                ("shared".to_owned(), set(&["ipc"])),
                ("devices".to_owned(), set(&["dri"])),
                ("filesystems".to_owned(), set(&["~/Games/EV Nova:ro"])),
                ("persistent".to_owned(), set(&[".local/state"])),
            ])
        );
    }

    #[test]
    fn reads_each_group_and_its_values() {
        let groups =
            permissions("[Context]\nsockets=x11;wayland;\n\n[Session Bus Policy]\norg.a=talk\n");
        assert_eq!(groups["Context"]["sockets"], set(&["x11", "wayland"]));
        assert_eq!(groups["Session Bus Policy"]["org.a"], set(&["talk"]));
    }
}
