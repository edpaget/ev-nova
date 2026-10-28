//! The platforms' updater adapters. Each method forwards to one call; the
//! choice of updater is the core's ([`choose`](crate::app::choose)).
//!
//! # macOS
//!
//! `nova_appkit`'s `SparkleUpdater` as the core's
//! [`Updater`](crate::app::Updater), and `SparkleSource`, what macOS offers
//! to update with, as its [`UpdaterSource`](crate::app::UpdaterSource).
//!
//! When Sparkle installs an update it asks the app to quit with the quit
//! Apple event, as the Dock's Quit does, which the game's handlers take as
//! the menu's Quit: the pilot flying is saved, and Sparkle relaunches the
//! new build. A modal that holds the game's quit (the startup warnings
//! sheet, a file dialog) holds Sparkle's too, which then offers to retry.
//!
//! Starting and checking need the app bundle and Sparkle loaded, and
//! reach the network, so no unit test runs them: nova-appkit's `updater`
//! tests load the framework for real, and the phase's After-landing check
//! installs an update.
//!
//! # Windows
//!
//! `nova_win32::WinSparkleDll` as the core's `WinSparkleApi`, `DllLoader`
//! finding it beside `nova.exe`, and `winsparkle_source`, what Windows
//! offers to update with: the core's [`crate::winsparkle::WinSparkleSource`]
//! over the build's compiled-in feed, whose shutdown request sends
//! `LoopEvent::UpdaterQuit` through the event loop's proxy. winit hands it
//! to the app on the main thread, as the Quit command; a closed loop drops
//! it, since the game is already exiting.
//!
//! The DLL's calls start WinSparkle's checks and windows, so no unit test
//! makes them: nova-win32's tests load the real DLL (`NOVA_WINSPARKLE_DLL`),
//! and the phase's After-landing check installs an update.

#[cfg(target_os = "macos")]
pub use macos::SparkleSource;
#[cfg(windows)]
pub use windows::{DllLoader, winsparkle_source};

#[cfg(target_os = "macos")]
mod macos {
    use nova_appkit::SparkleUpdater;

    use crate::app::{Updater, UpdaterSource};

    impl Updater for SparkleUpdater {
        fn start(&mut self) {
            SparkleUpdater::start(self);
        }

        fn check_now(&mut self) {
            SparkleUpdater::check_now(self);
        }
    }

    /// Sparkle, from the `Sparkle.framework` the app bundle ships, when its
    /// `Info.plist` names a feed.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct SparkleSource;

    impl UpdaterSource for SparkleSource {
        fn feed_configured(&self) -> bool {
            nova_appkit::feed_configured()
        }

        fn load(&self) -> Result<Box<dyn Updater>, String> {
            SparkleUpdater::load()
                .map(|updater| Box::new(updater) as Box<dyn Updater>)
                .map_err(|error| error.to_string())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_test_binary_names_no_feed() {
            assert!(!SparkleSource.feed_configured(), "it has no bundle");
        }
    }
}

#[cfg(windows)]
mod windows {
    use std::path::PathBuf;

    use nova_win32::{WINSPARKLE_FILE, WinSparkleDll};
    use winit::event_loop::EventLoopProxy;

    use crate::app::LoopEvent;
    use crate::build_id::BUILD_ID;
    use crate::winsparkle::{
        FEED, PUBLIC_KEY, ShutdownRequest, WinSparkleApi, WinSparkleConfig, WinSparkleLoader,
        WinSparkleSource,
    };

    impl WinSparkleApi for WinSparkleDll {
        fn set_appcast_url(&mut self, url: &str) {
            WinSparkleDll::set_appcast_url(self, url);
        }

        fn set_eddsa_public_key(&mut self, key: &str) -> bool {
            WinSparkleDll::set_eddsa_public_key(self, key)
        }

        fn set_app_details(&mut self, company: &str, app: &str, version: &str) {
            WinSparkleDll::set_app_details(self, company, app, version);
        }

        fn set_app_build_version(&mut self, build: &str) {
            WinSparkleDll::set_app_build_version(self, build);
        }

        fn set_automatic_check_for_updates(&mut self, on: bool) {
            WinSparkleDll::set_automatic_check_for_updates(self, on);
        }

        fn set_shutdown_request_callback(&mut self, on_request: ShutdownRequest) {
            WinSparkleDll::set_shutdown_request_callback(self, on_request);
        }

        fn init(&mut self) {
            WinSparkleDll::init(self);
        }

        fn cleanup(&mut self) {
            WinSparkleDll::cleanup(self);
        }

        fn check_update_with_ui(&mut self) {
            WinSparkleDll::check_update_with_ui(self);
        }
    }

    /// `WinSparkle.dll` beside the running executable.
    #[derive(Clone, Debug)]
    pub struct DllLoader {
        path: Option<PathBuf>,
    }

    impl DllLoader {
        /// The loader of the DLL in `nova.exe`'s folder; none when the
        /// executable's path is unknown.
        #[must_use]
        pub fn beside_exe() -> Self {
            let path = std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|dir| dir.join(WINSPARKLE_FILE)));
            Self { path }
        }
    }

    impl WinSparkleLoader for DllLoader {
        fn present(&self) -> bool {
            self.path.as_ref().is_some_and(|path| path.is_file())
        }

        fn load(&self) -> Result<Box<dyn WinSparkleApi>, String> {
            let path = self.path.as_ref().ok_or("the game's folder is unknown")?;
            WinSparkleDll::load(path)
                .map(|dll| Box::new(dll) as Box<dyn WinSparkleApi>)
                .map_err(|error| error.to_string())
        }
    }

    /// WinSparkle beside `nova.exe`, over the feed this build compiled in,
    /// its shutdown request sent through `proxy`.
    #[must_use]
    pub fn winsparkle_source(proxy: EventLoopProxy<LoopEvent>) -> WinSparkleSource<DllLoader> {
        let config = WinSparkleConfig::new(FEED, PUBLIC_KEY, BUILD_ID, env!("CARGO_PKG_VERSION"));
        WinSparkleSource::new(
            config,
            DllLoader::beside_exe(),
            Box::new(move || {
                // A closed loop drops it: the game is already exiting.
                let _ = proxy.send_event(LoopEvent::UpdaterQuit);
            }),
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_test_binary_has_no_dll_beside_it() {
            let loader = DllLoader::beside_exe();
            assert!(loader.path.is_some(), "the test binary's folder");
            assert!(!loader.present(), "{:?}", loader.path);
        }
    }
}
