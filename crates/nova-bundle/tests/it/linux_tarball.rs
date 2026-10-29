//! The built Linux tarball, unpacked and checked (Linux only, opt-in).
//!
//! It skips, passing, unless `NOVA_TARBALL` names a built `.tar.gz`, so
//! neither `mise run ci` nor a developer's test run needs one. CI's Linux
//! `build` leg (the `ubuntu:22.04` container) packs it and sets it.
//!
//! It unpacks the tarball with `tar xzf` into a directory the test owns
//! and checks its one top-level directory: executable `nova` and
//! `nova-dump`, `README.txt` and `LICENSE`, and the `share/` tree, each
//! file as the workspace's own spec renders it for the build ID the
//! bundler resolves (`NOVA_BUILD_ID`, which CI's `build` job sets for
//! every step, else HEAD's committer time). Then it runs `nova` through
//! the `objdump -p` check of the glibc floor and the allowed libraries
//! ([`check_links`]).

use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nova_bundle::archive::LINUX;
use nova_bundle::build_id::resolve;
use nova_bundle::bundle::metadata;
use nova_bundle::linux::links::check_links;
use nova_bundle::linux::share;
use nova_bundle::ports::Process;
use nova_bundle::spec::BundleSpec;
use nova_bundle::system::{StdEnv, StdProcess};

const VARIABLE: &str = "NOVA_TARBALL";

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
fn the_tarball_holds_the_game_and_a_binary_within_the_floor() {
    let Some(tarball) = env::var_os(VARIABLE)
        .filter(|v| !v.is_empty())
        .map(|v| std::path::absolute(PathBuf::from(v)).expect("an absolute path"))
    else {
        eprintln!("{VARIABLE} is unset: skipping the tarball test");
        return;
    };
    let tmp = tempfile::tempdir().expect("temp dir");
    let status = Command::new("tar")
        .arg("xzf")
        .arg(&tarball)
        .current_dir(tmp.path())
        .stdin(Stdio::null())
        .status()
        .unwrap_or_else(|e| panic!("tar xzf {}: {e}", tarball.display()));
    assert!(status.success(), "tar xzf: {status}");
    let root = tmp.path().join(LINUX.dir);

    for binary in ["nova", "nova-dump"] {
        let mode = fs::metadata(root.join(binary))
            .unwrap_or_else(|e| panic!("{binary}: {e}"))
            .permissions()
            .mode();
        assert_ne!(mode & 0o111, 0, "{binary} is executable: {mode:o}");
    }
    for file in ["README.txt", "LICENSE"] {
        assert!(root.join(file).is_file(), "{file} is in the tarball");
    }

    let spec = spec();
    let id = resolve(&StdEnv, &mut StdProcess, &spec.workspace_root).expect("a build ID");
    for (path, bytes) in share::files(&spec, &id, &root) {
        assert_eq!(read(&path), bytes, "{}", path.display());
    }

    check_links(&mut StdProcess, &root.join("nova")).unwrap_or_else(|e| panic!("{e}"));
}
