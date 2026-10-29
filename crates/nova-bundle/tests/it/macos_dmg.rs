//! The `.dmg` as built, mounted and read back (macOS only, opt-in): the
//! trimmed, thinned, signed Sparkle inside `Nova.app`, and the app's
//! `Info.plist` as the core renders it for this build and channel.
//!
//! It skips, passing, unless `NOVA_DMG` names a built `.dmg`, so neither
//! `mise run ci` nor a developer's test run mounts anything. CI's macOS
//! `build` leg bundles and sets it, with the `NOVA_CHANNEL` and
//! `NOVA_BUILD_ID` the bundle was built with. A guard detaches the volume
//! whatever happens.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use nova_bundle::appcast::Platform;
use nova_bundle::build_id;
use nova_bundle::bundle::{check_archs, metadata};
use nova_bundle::channel::{self, feed_status};
use nova_bundle::info_plist::info_plist;
use nova_bundle::layout::APP;
use nova_bundle::ports::{Invocation, Process};
use nova_bundle::sparkle;
use nova_bundle::spec::BundleSpec;
use nova_bundle::system::{StdEnv, StdProcess};

/// Detaches the mounted volume when dropped.
struct Mounted(PathBuf);

impl Drop for Mounted {
    fn drop(&mut self) {
        let detach = Invocation::new("hdiutil")
            .args(["detach", "-force"])
            .arg(&self.0);
        if let Err(e) = StdProcess.run(&detach) {
            eprintln!("cleanup: `{detach}` failed: {e}");
        }
    }
}

/// Mounts `dmg` read-only at `mount`, out of Finder's sight.
fn attach(dmg: &Path, mount: &Path) -> Mounted {
    fs::create_dir_all(mount).expect("the mount point");
    StdProcess
        .run(
            &Invocation::new("hdiutil")
                .arg("attach")
                .args(["-nobrowse", "-readonly", "-noautoopen", "-mountpoint"])
                .arg(mount)
                .arg(dmg),
        )
        .expect("hdiutil attach");
    Mounted(mount.to_owned())
}

/// The plist at `path` as a dictionary.
fn dictionary(path: &Path) -> plist::Dictionary {
    plist::Value::from_file(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .into_dictionary()
        .expect("a dictionary")
}

#[test]
fn the_dmg_holds_a_trimmed_signed_sparkle_and_the_channel_s_plist() {
    let Some(dmg) = env::var_os("NOVA_DMG").filter(|v| !v.is_empty()) else {
        eprintln!("NOVA_DMG is unset: skipping the dmg test");
        return;
    };
    let dir = tempfile::tempdir().expect("temp dir");
    let mount = dir.path().join("mnt");
    let _mounted = attach(Path::new(&dmg), &mount);
    let app = mount.join(APP);
    let contents = app.join("Contents");
    let framework = contents.join("Frameworks").join(sparkle::FRAMEWORK);
    let version = framework.join("Versions").join("B");

    let framework_plist = dictionary(&version.join("Resources").join("Info.plist"));
    assert_eq!(
        framework_plist
            .get("CFBundleShortVersionString")
            .and_then(plist::Value::as_string),
        Some(sparkle::VERSION)
    );
    for removed in [version.join("XPCServices"), framework.join("XPCServices")] {
        assert!(
            fs::symlink_metadata(&removed).is_err(),
            "{} is removed",
            removed.display()
        );
    }
    for binary in [
        version.join("Sparkle"),
        version.join("Autoupdate"),
        version
            .join("Updater.app")
            .join("Contents")
            .join("MacOS")
            .join("Updater"),
    ] {
        check_archs(&mut StdProcess, &binary).expect("arm64 alone");
    }
    assert!(
        contents
            .join("Resources")
            .join("Sparkle-LICENSE.txt")
            .is_file()
    );

    let json = StdProcess
        .run(&metadata(&OsString::from(env!("CARGO"))))
        .expect("cargo metadata");
    let spec = BundleSpec::from_metadata(&json).expect("spec");
    let build =
        build_id::resolve(&StdEnv, &mut StdProcess, &spec.workspace_root).expect("the build ID");
    let min_system = env::var("MACOSX_DEPLOYMENT_TARGET").expect("cargo's [env] sets it");
    let channel = channel::from_env(&StdEnv).expect("a known channel");
    let feed = feed_status(&spec, channel, Platform::Macos);
    assert_eq!(
        fs::read(contents.join("Info.plist")).expect("Info.plist"),
        info_plist(&spec, &build, &min_system, feed.feed()),
        "{}",
        feed.describe()
    );

    StdProcess
        .run(
            &Invocation::new("codesign")
                .args(["--verify", "--deep", "--strict", "--verbose=2"])
                .arg(&app),
        )
        .expect("the app's signature, Sparkle's within it, verifies");
}
