//! The FFI: Sparkle's standard updater, loaded at run time.
//!
//! Nothing links Sparkle at build time: there is no `#[link]`, and its
//! controller class is looked up only after `Sparkle.framework` loads, so
//! `cargo run` and every build without the framework run without it.
//! [`SparkleUpdater::load`] loads the framework the app bundle ships in
//! `Contents/Frameworks` (`NSBundle`'s `privateFrameworksPath`) and makes
//! an `SPUStandardUpdaterController` that isn't started: the game decides
//! when it starts ([`SparkleUpdater::start`]) and when it checks
//! ([`SparkleUpdater::check_now`]). [`feed_configured`] says whether the
//! app's `Info.plist` names a feed and its key, without which Sparkle
//! would show an error alert when started.
//!
//! When Sparkle installs an update it asks the app to quit with a quit
//! Apple event (`-[NSRunningApplication terminate]`), which [`AppleEvents`]
//! turns into the game's quit, as the Dock's Quit.
//!
//! Every `unsafe` block here carries a `SAFETY` comment. Loading needs the
//! main thread, so the crate's unit tests only check it refuses another;
//! `tests/it` loads for real, on its binary's main thread.
//!
//! [`AppleEvents`]: crate::AppleEvents

#![allow(unsafe_code)]

use std::ffi::CStr;
use std::fmt;
use std::path::{Path, PathBuf};

use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2::{MainThreadMarker, msg_send};
use objc2_foundation::{NSBundle, NSString};

/// The framework's directory name.
pub const FRAMEWORK: &str = "Sparkle.framework";
/// The controller's class.
const CONTROLLER: &CStr = c"SPUStandardUpdaterController";
/// The `Info.plist` keys a feed needs: its URL and the key its updates
/// are checked with.
pub const FEED_KEYS: [&str; 2] = ["SUFeedURL", "SUPublicEDKey"];

/// Why there is no updater.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdaterError {
    /// It was asked for off the main thread, where `AppKit` can't be used.
    NotMainThread,
    /// There is no framework at this path.
    NoFramework(PathBuf),
    /// The framework is there but would not load, for this reason.
    Load(String),
    /// The framework loaded but has no controller class, or it would not
    /// initialise.
    NoController,
}

impl fmt::Display for UpdaterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMainThread => f.write_str("Sparkle can only be loaded on the main thread"),
            Self::NoFramework(path) => write!(f, "there is no {}", path.display()),
            Self::Load(why) => write!(f, "{FRAMEWORK} would not load: {why}"),
            Self::NoController => write!(
                f,
                "{FRAMEWORK} has no working {}",
                CONTROLLER.to_string_lossy()
            ),
        }
    }
}

impl std::error::Error for UpdaterError {}

/// Whether the app's `Info.plist` names a feed: each of [`FEED_KEYS`] is a
/// string that isn't empty. A process without a bundle (`cargo run`, a
/// test binary) names none.
#[must_use]
pub fn feed_configured() -> bool {
    let bundle = NSBundle::mainBundle();
    FEED_KEYS.iter().all(|key| {
        bundle
            .objectForInfoDictionaryKey(&NSString::from_str(key))
            .and_then(|value| value.downcast::<NSString>().ok())
            .is_some_and(|value| value.length() > 0)
    })
}

/// Sparkle's standard updater: its controller, with Sparkle's own alerts
/// and windows, made but not started.
pub struct SparkleUpdater {
    controller: Retained<AnyObject>,
}

impl fmt::Debug for SparkleUpdater {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SparkleUpdater").finish_non_exhaustive()
    }
}

impl SparkleUpdater {
    /// The updater from the framework the app bundle ships in
    /// `Contents/Frameworks`.
    pub fn load() -> Result<Self, UpdaterError> {
        let mtm = MainThreadMarker::new().ok_or(UpdaterError::NotMainThread)?;
        match NSBundle::mainBundle().privateFrameworksPath() {
            Some(frameworks) => load_in(mtm, Path::new(&frameworks.to_string())),
            None => Err(UpdaterError::NoFramework(PathBuf::from(FRAMEWORK))),
        }
    }

    /// The updater from `frameworks/Sparkle.framework`.
    pub fn load_from(frameworks: &Path) -> Result<Self, UpdaterError> {
        let mtm = MainThreadMarker::new().ok_or(UpdaterError::NotMainThread)?;
        load_in(mtm, frameworks)
    }

    /// Starts the updater: it checks in the background, on Sparkle's
    /// schedule (`SUScheduledCheckInterval`, a day by default).
    pub fn start(&self) {
        // SAFETY: `startUpdater` takes nothing and returns void, as
        // Sparkle 2's `SPUStandardUpdaterController.h` declares, and the
        // controller is one, retained.
        unsafe { msg_send![&*self.controller, startUpdater] }
    }

    /// Checks for updates now, showing Sparkle's progress and its answer.
    pub fn check_now(&self) {
        // SAFETY: `checkForUpdates:` takes a nullable sender and returns
        // void (an `IBAction`), as the header declares; nil is allowed.
        unsafe { msg_send![&*self.controller, checkForUpdates: None::<&AnyObject>] }
    }
}

/// Loads `frameworks/Sparkle.framework` and makes its controller, not
/// started, on the main thread `_mtm` vouches for.
fn load_in(_mtm: MainThreadMarker, frameworks: &Path) -> Result<SparkleUpdater, UpdaterError> {
    let path = frameworks.join(FRAMEWORK);
    let bundle = NSBundle::bundleWithPath(&NSString::from_str(&path.to_string_lossy()))
        .ok_or_else(|| UpdaterError::NoFramework(path.clone()))?;
    // SAFETY: loading runs the framework's initialisers, which register
    // Sparkle's classes; it is the framework the bundle ships, signed with
    // the app.
    unsafe { bundle.loadAndReturnError() }
        .map_err(|error| UpdaterError::Load(error.localizedDescription().to_string()))?;
    let class = AnyClass::get(CONTROLLER).ok_or(UpdaterError::NoController)?;
    // SAFETY: `alloc` on the controller's class, then its initialiser
    // `initWithStartingUpdater:updaterDelegate:userDriverDelegate:`, which
    // takes a BOOL and two nullable delegates and returns `instancetype`,
    // as Sparkle 2's header declares. Both delegates may be nil.
    let controller: Option<Retained<AnyObject>> = unsafe {
        let allocated: Allocated<AnyObject> = msg_send![class, alloc];
        msg_send![
            allocated,
            initWithStartingUpdater: Bool::NO,
            updaterDelegate: None::<&AnyObject>,
            userDriverDelegate: None::<&AnyObject>
        ]
    };
    controller
        .map(|controller| SparkleUpdater { controller })
        .ok_or(UpdaterError::NoController)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_the_main_thread_nothing_is_loaded() {
        let (from, bundled) = std::thread::spawn(|| {
            (
                SparkleUpdater::load_from(Path::new("/nowhere")).map(drop),
                SparkleUpdater::load().map(drop),
            )
        })
        .join()
        .expect("the thread ends");
        assert_eq!(from, Err(UpdaterError::NotMainThread));
        assert_eq!(bundled, Err(UpdaterError::NotMainThread));
    }

    #[test]
    fn each_error_says_what_failed() {
        let path = Path::new("/a").join("Sparkle.framework");
        for (error, text) in [
            (
                UpdaterError::NotMainThread,
                "Sparkle can only be loaded on the main thread".to_owned(),
            ),
            (
                UpdaterError::NoFramework(path.clone()),
                format!("there is no {}", path.display()),
            ),
            (
                UpdaterError::Load("no executable".into()),
                "Sparkle.framework would not load: no executable".to_owned(),
            ),
            (
                UpdaterError::NoController,
                "Sparkle.framework has no working SPUStandardUpdaterController".to_owned(),
            ),
        ] {
            assert_eq!(error.to_string(), text);
        }
    }
}
