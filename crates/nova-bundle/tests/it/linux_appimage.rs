//! The built AppImage, extracted and checked (Linux only, opt-in).
//!
//! It skips, passing, unless `NOVA_APPIMAGE` names a built AppImage, so
//! neither `mise run ci` nor a developer's test run needs one. CI's Linux
//! `build` leg (the `ubuntu:22.04` container) builds it and sets it.
//!
//! It extracts the AppImage with `--appimage-extract` into a directory the
//! test owns, which needs no FUSE, and checks the AppDir: `AppRun`, the
//! desktop file and icon at its root, an executable `usr/bin/nova`, and
//! phase 1's `usr/share` tree, each file as the workspace's own spec
//! renders it for the build ID the bundler resolves (`NOVA_BUILD_ID`,
//! which CI's `build` job sets for every step, else HEAD's committer
//! time). Then it runs the extracted `nova` through the same
//! `objdump -p` check the build ran ([`check_links`]).

use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nova_bundle::build_id::resolve;
use nova_bundle::bundle::metadata;
use nova_bundle::linux::EXECUTABLE;
use nova_bundle::linux::appimage::links::check_links;
use nova_bundle::linux::desktop::{desktop_entry, desktop_file_name};
use nova_bundle::linux::icons::ICON_256;
use nova_bundle::linux::share;
use nova_bundle::ports::Process;
use nova_bundle::spec::BundleSpec;
use nova_bundle::system::{StdEnv, StdProcess};

const VARIABLE: &str = "NOVA_APPIMAGE";

fn spec() -> BundleSpec {
    let json = StdProcess
        .run(&metadata(&OsString::from(env!("CARGO"))))
        .expect("cargo metadata");
    BundleSpec::from_metadata(&json).expect("spec")
}

fn read(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn the_appimage_holds_the_app_dir_and_a_binary_within_the_floor() {
    let Some(appimage) = env::var_os(VARIABLE)
        .filter(|v| !v.is_empty())
        .map(|v| std::path::absolute(PathBuf::from(v)).expect("an absolute path"))
    else {
        eprintln!("{VARIABLE} is unset: skipping the AppImage test");
        return;
    };
    let tmp = tempfile::tempdir().expect("temp dir");
    let status = Command::new(&appimage)
        .arg("--appimage-extract")
        .current_dir(tmp.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .unwrap_or_else(|e| panic!("{}: {e}", appimage.display()));
    assert!(status.success(), "--appimage-extract: {status}");
    let root = tmp.path().join("squashfs-root");
    let spec = spec();

    let app_run = fs::read_link(root.join("AppRun")).expect("AppRun is a link");
    assert_eq!(app_run, Path::new("usr").join("bin").join(EXECUTABLE));

    let nova = root.join("usr").join("bin").join(EXECUTABLE);
    let mode = fs::metadata(&nova)
        .expect("usr/bin/nova")
        .permissions()
        .mode();
    assert_ne!(mode & 0o111, 0, "usr/bin/nova is executable: {mode:o}");

    // appimagetool may add keys to the root desktop file; every line the
    // spec renders must still be there.
    let desktop = String::from_utf8(read(&root.join(desktop_file_name(&spec)))).expect("UTF-8");
    for line in desktop_entry(&spec).lines() {
        assert!(desktop.lines().any(|l| l == line), "{line} in\n{desktop}");
    }
    let icon = format!("{}.png", spec.identifier);
    assert_eq!(read(&root.join(&icon)), ICON_256.png, "the root icon");
    assert_eq!(
        fs::read_link(root.join(".DirIcon")).expect(".DirIcon is a link"),
        Path::new(&icon)
    );

    let id = resolve(&StdEnv, &mut StdProcess, &spec.workspace_root).expect("a build ID");
    for (path, bytes) in share::files(&spec, &id, &root.join("usr")) {
        assert_eq!(read(&path), bytes, "{}", path.display());
    }

    check_links(&mut StdProcess, &nova).unwrap_or_else(|e| panic!("{e}"));
}
