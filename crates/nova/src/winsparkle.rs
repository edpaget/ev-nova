//! WinSparkle, the Windows updater, in the core's terms: the DLL's calls
//! as a port ([`WinSparkleApi`]), the startup sequence ([`configure`]),
//! the app's [`Updater`] over it ([`WinSparkleUpdater`]), and what Windows
//! offers to update with ([`WinSparkleSource`]), which decides when there
//! is no updater.
//!
//! It is plain Rust, built and tested on every host. The DLL's adapter is
//! `nova_win32::WinSparkleDll`, loaded from beside `nova.exe`, and
//! `nova::platform` wires the two together on Windows, forwarding each
//! port method to one call.
//!
//! # What a build updates from
//!
//! A Windows build compiles in its channel's feed and the key its updates
//! are checked with ([`FEED`], [`PUBLIC_KEY`]): nova's build script works
//! them out from `NOVA_CHANNEL` and the committed `update_public_key`
//! (`nova_winres::windows_update_feed`). A build updates only with a feed,
//! a key and a build ID ([`WinSparkleConfig::new`]); a development build
//! has none of them and looks for nothing. A build that has them but no
//! `WinSparkle.dll` beside it (the portable zip, a `cargo run`) runs the
//! no-op updater quietly. A DLL that is there but won't load, lacks a call
//! or refuses the key is a broken install, and the game says so
//! ([`choose`](crate::app::choose)'s warning).
//!
//! # The quit
//!
//! When the player installs an update, WinSparkle runs the new `.msi` and
//! then asks the game to quit, from a thread of its own, so the installer
//! can replace `nova.exe`. Without the request it does nothing, and the
//! game would hold its files. The platform's closure sends the request
//! through the event loop as `LoopEvent::UpdaterQuit`, which the app takes
//! on the main thread as the Quit command: it quits at once and saves the
//! pilot flying. The installer then relaunches the game.

use std::cell::Cell;

use crate::app::{Updater, UpdaterSource};

/// The Windows appcast this build checks, compiled in by nova's build
/// script for a channel with a committed key; `None` otherwise, and
/// always off Windows, where Sparkle reads the bundle's `Info.plist`.
#[cfg(windows)]
pub const FEED: Option<&str> = option_env!("NOVA_UPDATE_FEED");
/// The Windows appcast this build checks; never compiled in off Windows.
#[cfg(not(windows))]
pub const FEED: Option<&str> = None;

/// The public key, standard base64, the feed's updates are checked with,
/// compiled in beside [`FEED`].
#[cfg(windows)]
pub const PUBLIC_KEY: Option<&str> = option_env!("NOVA_UPDATE_PUBLIC_KEY");
/// The public key the feed's updates are checked with; never compiled in
/// off Windows.
#[cfg(not(windows))]
pub const PUBLIC_KEY: Option<&str> = None;

/// The company WinSparkle names: the installer's Manufacturer. With
/// [`APP`] it names WinSparkle's registry key,
/// `HKCU\Software\<company>\<app>\WinSparkle`.
pub const COMPANY: &str = "ev-nova contributors";

/// The app WinSparkle names: the product's name.
pub const APP: &str = "EV Nova";

/// What runs when WinSparkle asks the game to quit. WinSparkle calls it
/// from a thread of its own, never the main one.
pub type ShutdownRequest = Box<dyn Fn() + Send>;

/// Port: WinSparkle's calls, in Rust terms. The adapter turns each into
/// one call of the DLL.
pub trait WinSparkleApi {
    /// `win_sparkle_set_appcast_url`: the feed to check.
    fn set_appcast_url(&mut self, url: &str);

    /// `win_sparkle_set_eddsa_public_key`: the key, standard base64.
    /// Whether WinSparkle took it as a valid key.
    fn set_eddsa_public_key(&mut self, key: &str) -> bool;

    /// `win_sparkle_set_app_details`: the company, the app and the
    /// version it displays.
    fn set_app_details(&mut self, company: &str, app: &str, version: &str);

    /// `win_sparkle_set_app_build_version`: the build ID, compared with
    /// the feed's `sparkle:version`.
    fn set_app_build_version(&mut self, build: &str);

    /// `win_sparkle_set_automatic_check_for_updates`: whether it checks in
    /// the background without asking the player first.
    fn set_automatic_check_for_updates(&mut self, on: bool);

    /// `win_sparkle_set_shutdown_request_callback`: what runs when it asks
    /// the game to quit.
    fn set_shutdown_request_callback(&mut self, on_request: ShutdownRequest);

    /// `win_sparkle_init`: starts its background checks. Every setter
    /// comes first.
    fn init(&mut self);

    /// `win_sparkle_cleanup`: stops it, at exit.
    fn cleanup(&mut self);

    /// `win_sparkle_check_update_with_ui`: checks now, showing its dialog.
    fn check_update_with_ui(&mut self);
}

/// What WinSparkle is configured with: the feed, its key, the build ID and
/// the version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinSparkleConfig {
    appcast_url: String,
    public_key: String,
    build: String,
    version: String,
}

impl WinSparkleConfig {
    /// The configuration of a build with `feed`, `key` and `build` (its
    /// build ID) at `version`: `None` unless the feed, the key and the
    /// build ID are all there and non-empty, and nothing holds a NUL,
    /// which the DLL's strings can't carry.
    #[must_use]
    pub fn new(
        feed: Option<&str>,
        key: Option<&str>,
        build: Option<&str>,
        version: &str,
    ) -> Option<Self> {
        fn given(value: Option<&str>) -> Option<&str> {
            value.filter(|value| !value.is_empty())
        }
        let (feed, key, build) = (given(feed)?, given(key)?, given(build)?);
        if [feed, key, build, version]
            .iter()
            .any(|value| value.contains('\0'))
        {
            return None;
        }
        Some(Self {
            appcast_url: feed.to_owned(),
            public_key: key.to_owned(),
            build: build.to_owned(),
            version: version.to_owned(),
        })
    }
}

/// Configures `api` with `config` and `on_shutdown`, in the order
/// WinSparkle needs, all before `init`: the feed, the key, the app's
/// details, the build ID, background checks on (as macOS's
/// `SUEnableAutomaticChecks`), and the shutdown request. It doesn't start
/// it: [`Updater::start`] does.
///
/// # Errors
///
/// When WinSparkle refuses the key; nothing more is set.
pub fn configure(
    mut api: Box<dyn WinSparkleApi>,
    config: &WinSparkleConfig,
    on_shutdown: ShutdownRequest,
) -> Result<WinSparkleUpdater, String> {
    api.set_appcast_url(&config.appcast_url);
    if !api.set_eddsa_public_key(&config.public_key) {
        return Err("WinSparkle refused the build's update public key".to_owned());
    }
    api.set_app_details(COMPANY, APP, &config.version);
    api.set_app_build_version(&config.build);
    api.set_automatic_check_for_updates(true);
    api.set_shutdown_request_callback(on_shutdown);
    Ok(WinSparkleUpdater {
        api,
        started: false,
    })
}

/// The app's updater over a configured WinSparkle: started once, checked
/// only once started, and cleaned up when dropped if it started. The app
/// drops it as `main` drops the runner, after the event loop ends.
pub struct WinSparkleUpdater {
    api: Box<dyn WinSparkleApi>,
    started: bool,
}

impl Updater for WinSparkleUpdater {
    fn start(&mut self) {
        if !self.started {
            self.started = true;
            self.api.init();
        }
    }

    fn check_now(&mut self) {
        if self.started {
            self.api.check_update_with_ui();
        }
    }
}

impl Drop for WinSparkleUpdater {
    fn drop(&mut self) {
        if self.started {
            self.api.cleanup();
        }
    }
}

/// Port: finds and loads WinSparkle.
pub trait WinSparkleLoader {
    /// Whether the DLL is there to load.
    fn present(&self) -> bool;

    /// The DLL, loaded with every call it needs, or why it couldn't be.
    fn load(&self) -> Result<Box<dyn WinSparkleApi>, String>;
}

/// What Windows offers to update with: WinSparkle, through `loader`, when
/// the build has a configuration and the DLL is there.
pub struct WinSparkleSource<L> {
    config: Option<WinSparkleConfig>,
    loader: L,
    on_shutdown: Cell<Option<ShutdownRequest>>,
}

impl<L: WinSparkleLoader> WinSparkleSource<L> {
    /// The source of a build configured with `config` (`None`: it doesn't
    /// update), loading through `loader`, whose WinSparkle runs
    /// `on_shutdown` when it asks the game to quit.
    #[must_use]
    pub fn new(config: Option<WinSparkleConfig>, loader: L, on_shutdown: ShutdownRequest) -> Self {
        Self {
            config,
            loader,
            on_shutdown: Cell::new(Some(on_shutdown)),
        }
    }
}

impl<L: WinSparkleLoader> UpdaterSource for WinSparkleSource<L> {
    /// A configuration and the DLL beside the game. Without a
    /// configuration the DLL is never looked for.
    fn feed_configured(&self) -> bool {
        self.config.is_some() && self.loader.present()
    }

    /// The DLL, loaded and configured, once.
    fn load(&self) -> Result<Box<dyn Updater>, String> {
        let config = self
            .config
            .as_ref()
            .ok_or("this build names no update feed")?;
        let on_shutdown = self
            .on_shutdown
            .take()
            .ok_or("WinSparkle is already loaded")?;
        let api = self.loader.load()?;
        configure(api, config, on_shutdown).map(|updater| Box::new(updater) as Box<dyn Updater>)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;

    use super::*;
    use crate::app::{LoopEvent, choose};

    /// One call the recording API took.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Call {
        AppcastUrl(String),
        EddsaKey(String),
        AppDetails(String, String, String),
        BuildVersion(String),
        AutomaticChecks(bool),
        ShutdownCallback,
        Init,
        Cleanup,
        CheckWithUi,
    }

    type Calls = Rc<RefCell<Vec<Call>>>;
    type Kept = Rc<RefCell<Option<ShutdownRequest>>>;

    /// Records each call into a log the test keeps, keeps the shutdown
    /// closure for the test to fire, and takes the key unless `refuses`.
    struct RecordingApi {
        calls: Calls,
        shutdown: Kept,
        refuses: bool,
    }

    impl RecordingApi {
        fn boxed(calls: &Calls, shutdown: &Kept, refuses: bool) -> Box<dyn WinSparkleApi> {
            Box::new(Self {
                calls: Rc::clone(calls),
                shutdown: Rc::clone(shutdown),
                refuses,
            })
        }

        fn push(&self, call: Call) {
            self.calls.borrow_mut().push(call);
        }
    }

    impl WinSparkleApi for RecordingApi {
        fn set_appcast_url(&mut self, url: &str) {
            self.push(Call::AppcastUrl(url.to_owned()));
        }

        fn set_eddsa_public_key(&mut self, key: &str) -> bool {
            self.push(Call::EddsaKey(key.to_owned()));
            !self.refuses
        }

        fn set_app_details(&mut self, company: &str, app: &str, version: &str) {
            self.push(Call::AppDetails(
                company.to_owned(),
                app.to_owned(),
                version.to_owned(),
            ));
        }

        fn set_app_build_version(&mut self, build: &str) {
            self.push(Call::BuildVersion(build.to_owned()));
        }

        fn set_automatic_check_for_updates(&mut self, on: bool) {
            self.push(Call::AutomaticChecks(on));
        }

        fn set_shutdown_request_callback(&mut self, on_request: ShutdownRequest) {
            self.push(Call::ShutdownCallback);
            *self.shutdown.borrow_mut() = Some(on_request);
        }

        fn init(&mut self) {
            self.push(Call::Init);
        }

        fn cleanup(&mut self) {
            self.push(Call::Cleanup);
        }

        fn check_update_with_ui(&mut self) {
            self.push(Call::CheckWithUi);
        }
    }

    /// Whether the DLL is there, and whether it loads (a recording API
    /// that refuses the key or not) or fails; counts its calls.
    struct FakeLoader {
        present: bool,
        load: Result<bool, String>,
        calls: Calls,
        shutdown: Kept,
        presents: Cell<usize>,
        loads: Cell<usize>,
    }

    impl FakeLoader {
        fn new(present: bool, load: Result<bool, String>) -> Self {
            Self {
                present,
                load,
                calls: Calls::default(),
                shutdown: Kept::default(),
                presents: Cell::new(0),
                loads: Cell::new(0),
            }
        }
    }

    impl WinSparkleLoader for &FakeLoader {
        fn present(&self) -> bool {
            self.presents.set(self.presents.get() + 1);
            self.present
        }

        fn load(&self) -> Result<Box<dyn WinSparkleApi>, String> {
            self.loads.set(self.loads.get() + 1);
            self.load
                .clone()
                .map(|refuses| RecordingApi::boxed(&self.calls, &self.shutdown, refuses))
        }
    }

    const FEED_URL: &str =
        "https://github.com/edpaget/ev-nova/releases/download/nightly/appcast-windows-nightly.xml";
    const KEY: &str = "11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=";
    const BUILD: &str = "20261028123456";

    fn config() -> WinSparkleConfig {
        WinSparkleConfig::new(Some(FEED_URL), Some(KEY), Some(BUILD), "0.1.0").expect("a config")
    }

    fn nothing() -> ShutdownRequest {
        Box::new(|| {})
    }

    /// The startup sequence, every setter, in order.
    fn sequence() -> Vec<Call> {
        vec![
            Call::AppcastUrl(FEED_URL.to_owned()),
            Call::EddsaKey(KEY.to_owned()),
            Call::AppDetails(
                "ev-nova contributors".to_owned(),
                "EV Nova".to_owned(),
                "0.1.0".to_owned(),
            ),
            Call::BuildVersion(BUILD.to_owned()),
            Call::AutomaticChecks(true),
            Call::ShutdownCallback,
        ]
    }

    /// A configured updater over a recording API, and its log.
    fn configured() -> (WinSparkleUpdater, Calls) {
        let calls = Calls::default();
        let api = RecordingApi::boxed(&calls, &Kept::default(), false);
        let updater = configure(api, &config(), nothing()).expect("configured");
        calls.borrow_mut().clear();
        (updater, calls)
    }

    #[test]
    fn a_config_needs_a_feed_a_key_and_a_build_id() {
        assert!(WinSparkleConfig::new(Some(FEED_URL), Some(KEY), Some(BUILD), "0.1.0").is_some());
        for (feed, key, build) in [
            (None, Some(KEY), Some(BUILD)),
            (Some(""), Some(KEY), Some(BUILD)),
            (Some(FEED_URL), None, Some(BUILD)),
            (Some(FEED_URL), Some(""), Some(BUILD)),
            // A development build: no build ID.
            (Some(FEED_URL), Some(KEY), None),
            (Some(FEED_URL), Some(KEY), Some("")),
            // A build without a channel, or without a committed key.
            (None, None, Some(BUILD)),
        ] {
            assert_eq!(
                WinSparkleConfig::new(feed, key, build, "0.1.0"),
                None,
                "{feed:?} {key:?} {build:?}"
            );
        }
    }

    #[test]
    fn a_value_with_a_nul_names_no_feed() {
        for (feed, key, build, version) in [
            ("a\0b", KEY, BUILD, "0.1.0"),
            (FEED_URL, "a\0b", BUILD, "0.1.0"),
            (FEED_URL, KEY, "2026\0", "0.1.0"),
            (FEED_URL, KEY, BUILD, "0.1\0"),
        ] {
            assert_eq!(
                WinSparkleConfig::new(Some(feed), Some(key), Some(build), version),
                None,
                "{feed:?} {key:?} {build:?} {version:?}"
            );
        }
    }

    #[test]
    fn configure_runs_the_startup_sequence_in_order() {
        let calls = Calls::default();
        let api = RecordingApi::boxed(&calls, &Kept::default(), false);
        let updater = configure(api, &config(), nothing()).expect("configured");
        assert_eq!(*calls.borrow(), sequence(), "and no init");
        drop(updater);
        assert_eq!(
            *calls.borrow(),
            sequence(),
            "an unstarted one isn't cleaned up"
        );
    }

    #[test]
    fn a_refused_key_is_an_error_and_nothing_more_is_set() {
        let calls = Calls::default();
        let api = RecordingApi::boxed(&calls, &Kept::default(), true);
        let error = configure(api, &config(), nothing()).err().expect("refused");
        assert_eq!(error, "WinSparkle refused the build's update public key");
        assert_eq!(*calls.borrow(), sequence()[..2]);
    }

    #[test]
    fn start_inits_once() {
        let (mut updater, calls) = configured();
        updater.start();
        updater.start();
        assert_eq!(*calls.borrow(), [Call::Init]);
    }

    #[test]
    fn check_now_shows_the_check_only_after_start() {
        let (mut updater, calls) = configured();
        updater.check_now();
        assert_eq!(*calls.borrow(), [], "not started");
        updater.start();
        updater.check_now();
        assert_eq!(*calls.borrow(), [Call::Init, Call::CheckWithUi]);
    }

    #[test]
    fn dropping_a_started_updater_cleans_up_and_an_unstarted_one_doesn_t() {
        let (updater, calls) = configured();
        drop(updater);
        assert_eq!(*calls.borrow(), []);
        let (mut updater, calls) = configured();
        updater.start();
        drop(updater);
        assert_eq!(*calls.borrow(), [Call::Init, Call::Cleanup]);
    }

    #[test]
    fn it_can_check() {
        let (updater, _) = configured();
        assert!(updater.can_check());
    }

    #[test]
    fn the_shutdown_request_runs_the_closure_given() {
        let (sender, receiver) = mpsc::channel();
        let calls = Calls::default();
        let kept = Kept::default();
        let api = RecordingApi::boxed(&calls, &kept, false);
        let on_shutdown: ShutdownRequest = Box::new(move || {
            let _ = sender.send(LoopEvent::UpdaterQuit);
        });
        let _updater = configure(api, &config(), on_shutdown).expect("configured");
        let request = kept.borrow_mut().take().expect("the closure, given");
        // WinSparkle asks from a thread of its own.
        std::thread::spawn(request).join().expect("asked");
        assert_eq!(receiver.try_recv(), Ok(LoopEvent::UpdaterQuit));
        assert!(receiver.try_recv().is_err(), "exactly once");
    }

    #[test]
    fn without_a_config_nothing_is_looked_for() {
        let loader = FakeLoader::new(true, Ok(false));
        let source = WinSparkleSource::new(None, &loader, nothing());
        let chosen = choose(&source);
        assert!(!chosen.updater.can_check(), "the no-op updater");
        assert_eq!(chosen.warning, None);
        assert_eq!((loader.presents.get(), loader.loads.get()), (0, 0));
        assert_eq!(
            source.load().err().as_deref(),
            Some("this build names no update feed")
        );
        assert_eq!(loader.loads.get(), 0);
    }

    #[test]
    fn a_config_without_the_dll_is_the_quiet_no_op() {
        let loader = FakeLoader::new(false, Ok(false));
        let chosen = choose(&WinSparkleSource::new(Some(config()), &loader, nothing()));
        assert!(!chosen.updater.can_check());
        assert_eq!(chosen.warning, None, "the portable zip says nothing");
        assert_eq!((loader.presents.get(), loader.loads.get()), (1, 0));
    }

    #[test]
    fn a_dll_that_wont_load_is_the_no_op_with_a_warning() {
        let loader = FakeLoader::new(true, Err("no win_sparkle_init".to_owned()));
        let chosen = choose(&WinSparkleSource::new(Some(config()), &loader, nothing()));
        assert!(!chosen.updater.can_check());
        assert_eq!(
            chosen.warning.as_deref(),
            Some("nova: updates are off: no win_sparkle_init")
        );
        assert_eq!(loader.loads.get(), 1);
    }

    #[test]
    fn a_dll_that_refuses_the_key_is_the_no_op_with_a_warning() {
        let loader = FakeLoader::new(true, Ok(true));
        let chosen = choose(&WinSparkleSource::new(Some(config()), &loader, nothing()));
        assert!(!chosen.updater.can_check());
        assert_eq!(
            chosen.warning.as_deref(),
            Some("nova: updates are off: WinSparkle refused the build's update public key")
        );
        assert!(
            !loader.calls.borrow().contains(&Call::Init),
            "never started"
        );
    }

    #[test]
    fn a_dll_that_loads_is_configured_and_started_by_the_app() {
        let loader = FakeLoader::new(true, Ok(false));
        let source = WinSparkleSource::new(Some(config()), &loader, nothing());
        let mut chosen = choose(&source);
        assert_eq!(chosen.warning, None);
        assert!(chosen.updater.can_check());
        assert_eq!(
            *loader.calls.borrow(),
            sequence(),
            "configured, not started"
        );
        chosen.updater.start();
        let mut started = sequence();
        started.push(Call::Init);
        assert_eq!(*loader.calls.borrow(), started);
        assert!(loader.shutdown.borrow().is_some(), "the closure, given");
        assert_eq!(
            source.load().err().as_deref(),
            Some("WinSparkle is already loaded"),
            "its closure went to the first"
        );
        assert_eq!(loader.loads.get(), 1);
    }
}
