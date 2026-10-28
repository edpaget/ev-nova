//! The macOS tools the bundler drives, run for real through `StdProcess`
//! with the argument lists the core builds: `plutil` on the generated
//! plist, the `lipo -archs` arm64 check and `codesign` on a binary, and
//! `hdiutil` on the staging folder, Sparkle's `shasum` check, `tar` unpack
//! and `ditto` copy, and a read of the keychain search list. Each works in
//! a temporary directory it owns.

use std::fs;
use std::path::Path;

use nova_bundle::build_id::BuildId;
use nova_bundle::bundle::{BundleError, Host, Step, check_archs, plan};
use nova_bundle::digest::{DigestTool, Pin, check_sha256};
use nova_bundle::info_plist::info_plist;
use nova_bundle::layout::Layout;
use nova_bundle::ports::{Invocation, Process};
use nova_bundle::signing::{self, Signing};
use nova_bundle::sparkle;
use nova_bundle::spec::BundleSpec;
use nova_bundle::system::StdProcess;

fn build() -> BuildId {
    BuildId::parse("20261028123456").expect("an ID")
}

fn spec(root: &Path) -> BundleSpec {
    BundleSpec {
        workspace_root: root.to_owned(),
        target_dir: root.join("target"),
        version: "0.1.0".into(),
        license: "GPL-3.0-or-later".into(),
        identifier: "io.github.edpaget.ev-nova".into(),
        name: "EV Nova".into(),
        category: "public.app-category.role-playing-games".into(),
        summary: "Play EV Nova from your own copy of its data".into(),
        description: "A new engine for EV Nova.".into(),
        homepage: "https://github.com/edpaget/ev-nova".into(),
        developer: "ev-nova contributors".into(),
        update_public_key: None,
    }
}

/// The core's layout under `root`, and the programs its steps run.
fn core_runs(root: &Path) -> (Layout, Vec<Invocation>) {
    let spec = spec(root);
    let layout = Layout::new(&spec.target_dir, Some(root.join("out"))).expect("layout");
    let host = Host {
        cargo: "cargo".into(),
        min_system: "13.0".into(),
        out: Some(layout.out.clone()),
        signing: Signing::AdHoc,
        channel: None,
    };
    let runs = plan(&spec, &layout, &host, &build())
        .steps
        .into_iter()
        .filter_map(|step| match step {
            Step::Run(invocation) => Some(invocation),
            _ => None,
        })
        .collect();
    (layout, runs)
}

/// The core's run of `program` with `first` as its first argument, on
/// `path`.
fn find(runs: &[Invocation], program: &str, first: &str, path: &Path) -> Invocation {
    runs.iter()
        .find(|i| {
            i.program == program
                && i.args.first().is_some_and(|a| a == first)
                && i.args.iter().any(|a| Path::new(a) == path)
        })
        .cloned()
        .unwrap_or_else(|| panic!("a `{program} {first}` run on {}", path.display()))
}

#[test]
fn plutil_accepts_the_generated_plist() {
    let dir = tempfile::tempdir().expect("temp dir");
    let plist = dir.path().join("Info.plist");
    fs::write(
        &plist,
        info_plist(&spec(dir.path()), &build(), "13.0", None),
    )
    .expect("write");
    StdProcess
        .run(&Invocation::new("plutil").arg("-lint").arg(&plist))
        .expect("plutil -lint passes");
}

#[test]
fn the_arm64_check_and_codesign_work_on_a_copy_of_a_binary() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (layout, runs) = core_runs(dir.path());
    fs::create_dir_all(&layout.staging).expect("staging");
    let binary = layout.dump();
    fs::copy(std::env::current_exe().expect("this test"), &binary).expect("copy");

    // This test binary is built for the host alone: arm64 on Apple
    // silicon, which the check passes, and x86_64 elsewhere, which it
    // refuses.
    let checked = check_archs(&mut StdProcess, &binary);
    if cfg!(target_arch = "aarch64") {
        checked.expect("arm64 alone");
    } else {
        assert!(
            matches!(checked, Err(BundleError::NotArm64 { .. })),
            "{checked:?}"
        );
    }

    StdProcess
        .run(&find(&runs, "codesign", "--force", &binary))
        .expect("ad-hoc signs");
    StdProcess
        .run(&find(&runs, "codesign", "--verify", &binary))
        .expect("verifies");
}

#[test]
fn hdiutil_makes_a_dmg_of_the_staging_folder() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (layout, runs) = core_runs(dir.path());
    fs::create_dir_all(&layout.staging).expect("staging");
    fs::create_dir_all(&layout.out).expect("out");
    fs::write(layout.staging.join("README.txt"), b"hello").expect("file");
    StdProcess
        .run(&find(&runs, "hdiutil", "create", &layout.dmg()))
        .expect("hdiutil create");
    assert!(layout.dmg().is_file());
}

#[test]
fn shasum_digests_a_file_as_the_check_reads_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("abc");
    fs::write(&file, b"abc").expect("seed");
    let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let pin = Pin {
        what: "abc",
        sha256: abc,
    };
    check_sha256(&mut StdProcess, &file, pin, DigestTool::Shasum).expect("the digest of abc");
    let error = check_sha256(&mut StdProcess, &file, sparkle::PIN, DigestTool::Shasum)
        .expect_err("another digest");
    assert!(
        matches!(&error, BundleError::Checksum { found, .. } if found == abc),
        "{error:?}"
    );
}

/// A `.tar.xz` shaped as Sparkle's release: `./`-prefixed entries, the
/// framework with a `Versions/Current` link, the licence, and a `bin/`
/// the bundler doesn't want, written to `archive`.
fn fixture_archive(scratch: &Path, archive: &Path) {
    let source = scratch.join("source");
    let b = source.join("Sparkle.framework").join("Versions").join("B");
    fs::create_dir_all(&b).expect("seed");
    fs::write(b.join("x"), b"code").expect("seed");
    std::os::unix::fs::symlink("B", b.with_file_name("Current")).expect("seed");
    fs::write(source.join("LICENSE"), b"licence").expect("seed");
    fs::create_dir_all(source.join("bin")).expect("seed");
    fs::write(source.join("bin").join("y"), b"tool").expect("seed");
    fs::create_dir_all(archive.parent().expect("a folder")).expect("seed");
    StdProcess
        .run(
            &Invocation::new("tar")
                .arg("-cJf")
                .arg(archive)
                .arg("-C")
                .arg(&source)
                .args(["./Sparkle.framework", "./LICENSE", "./bin"]),
        )
        .expect("tar -cJf");
}

#[test]
fn tar_unpacks_the_framework_and_licence_keeping_links() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (layout, runs) = core_runs(dir.path());
    fixture_archive(dir.path(), &layout.sparkle_archive());
    StdProcess
        .run(&find(&runs, "tar", "-xJf", &layout.sparkle_archive()))
        .expect("tar -xJf");
    let unpacked = layout.downloads().join("Sparkle.framework");
    let current = unpacked.join("Versions").join("Current");
    assert!(
        fs::symlink_metadata(&current)
            .expect("Current")
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read(current.join("x")).expect("through the link"),
        b"code"
    );
    assert_eq!(
        fs::read(layout.downloads().join("LICENSE")).expect("licence"),
        b"licence"
    );
    assert!(!layout.downloads().join("bin").exists(), "only what ships");
}

#[test]
fn ditto_copies_the_framework_keeping_links() {
    let dir = tempfile::tempdir().expect("temp dir");
    let (layout, runs) = core_runs(dir.path());
    fixture_archive(dir.path(), &layout.sparkle_archive());
    StdProcess
        .run(&find(&runs, "tar", "-xJf", &layout.sparkle_archive()))
        .expect("tar -xJf");
    fs::create_dir_all(layout.frameworks()).expect("Frameworks");
    let unpacked = layout.downloads().join("Sparkle.framework");
    let first = unpacked.to_str().expect("a UTF-8 path");
    StdProcess
        .run(&find(&runs, "ditto", first, &layout.sparkle_framework()))
        .expect("ditto");
    let current = layout.sparkle_framework().join("Versions").join("Current");
    assert!(
        fs::symlink_metadata(&current)
            .expect("Current")
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read(current.join("x")).expect("through the link"),
        b"code"
    );
}

/// Read-only: the core's `list-keychains` run, parsed as the core parses
/// it. Nothing here creates, imports or deletes a keychain, which would
/// change the developer's search list.
#[test]
fn the_user_s_keychain_search_list_parses_into_paths() {
    let listed = StdProcess
        .run(&signing::list_keychains())
        .expect("security list-keychains");
    let paths = signing::search_list(&listed);
    assert!(!paths.is_empty(), "{}", String::from_utf8_lossy(&listed));
    for path in paths {
        assert!(path.is_absolute(), "{}", path.display());
    }
}
