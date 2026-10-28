//! Sparkle's updater loaded for real, on the main thread: a folder with no
//! framework, a framework that won't load, the test binary's bundle (none,
//! so no feed), and, opt-in, the real framework a bundle ships.
//!
//! The last skips, passing, unless `NOVA_SPARKLE_FRAMEWORKS` names a
//! `Contents/Frameworks` holding `Sparkle.framework`; CI's macOS `build`
//! leg sets it to the bundle it just staged. It makes the controller but
//! never starts it or checks, so it reaches no network and shows nothing.

use std::fs;
use std::path::PathBuf;

use nova_appkit::{FRAMEWORK, SparkleUpdater, UpdaterError, feed_configured};

pub fn a_folder_without_the_framework_loads_no_updater() {
    let dir = tempfile::tempdir().expect("temp dir");
    let error = SparkleUpdater::load_from(dir.path()).expect_err("no framework");
    assert_eq!(error, UpdaterError::NoFramework(dir.path().join(FRAMEWORK)));
}

pub fn a_framework_that_wont_load_is_a_load_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let resources = dir.path().join(FRAMEWORK).join("Resources");
    fs::create_dir_all(&resources).expect("seed");
    fs::write(
        resources.join("Info.plist"),
        br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>io.example.not-sparkle</string>
<key>CFBundleExecutable</key><string>Missing</string>
<key>CFBundlePackageType</key><string>FMWK</string>
</dict></plist>
"#,
    )
    .expect("seed");
    let error = SparkleUpdater::load_from(dir.path()).expect_err("won't load");
    assert!(matches!(error, UpdaterError::Load(_)), "{error:?}");
    assert!(
        error
            .to_string()
            .starts_with("Sparkle.framework would not load: ")
    );
}

pub fn the_test_binary_s_bundle_has_no_feed() {
    assert!(!feed_configured());
}

pub fn the_real_framework_makes_a_controller_that_isn_t_started() {
    let Some(frameworks) = std::env::var_os("NOVA_SPARKLE_FRAMEWORKS").filter(|v| !v.is_empty())
    else {
        eprintln!("NOVA_SPARKLE_FRAMEWORKS is unset: skipping the real framework's load");
        return;
    };
    let updater = SparkleUpdater::load_from(&PathBuf::from(frameworks))
        .expect("the controller, from the class and initialiser Sparkle names");
    assert_eq!(format!("{updater:?}"), "SparkleUpdater { .. }");
}
