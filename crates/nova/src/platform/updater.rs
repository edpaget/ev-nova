//! The macOS updater's adapter: `nova_appkit`'s `SparkleUpdater` as the
//! core's [`Updater`], and [`SparkleSource`], what macOS offers to update
//! with, as its [`UpdaterSource`]. Each method forwards to one call; the
//! choice of updater is the core's ([`choose`](crate::app::choose)).
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
