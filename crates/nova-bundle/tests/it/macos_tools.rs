//! The macOS tools the bundler drives, run for real through `StdProcess`
//! with the argument lists the core builds: `plutil` on the generated
//! plist, the `lipo -archs` arm64 check and `codesign` on a binary, and
//! `hdiutil` on the staging folder, and a read of the keychain search list.
//! Each works in a temporary directory it owns.

use std::fs;
use std::path::Path;

use nova_bundle::build_id::BuildId;
use nova_bundle::bundle::{BundleError, Host, Step, check_archs, plan};
use nova_bundle::info_plist::info_plist;
use nova_bundle::layout::Layout;
use nova_bundle::ports::{Invocation, Process};
use nova_bundle::signing::{self, Signing};
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
    fs::write(&plist, info_plist(&spec(dir.path()), &build(), "13.0")).expect("write");
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
