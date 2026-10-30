//! Updates: the [`Updater`] port the app drives, and the choice of updater
//! at launch.
//!
//! The app takes one updater ([`App::with_updater`]) and starts its
//! background checks as it does; the menu bar's Check for Updates…
//! ([`WindowEvent::CheckForUpdates`]) asks it to check now. The item is
//! offered only when the updater can check ([`Updater::can_check`]), which
//! the menu bar learns as it learns the screen's commands
//! ([`MenuPick::CheckForUpdates`]).
//!
//! What the platform offers is an [`UpdaterSource`]: whether the build
//! names a feed, and the platform's updater, loaded. [`choose`] is the
//! decision. On macOS the source is Sparkle's (`nova::platform`'s
//! `SparkleSource`); elsewhere there is none, and the app keeps
//! [`NoUpdater`].
//!
//! [`App::with_updater`]: super::App::with_updater
//! [`WindowEvent::CheckForUpdates`]: super::WindowEvent::CheckForUpdates
//! [`MenuPick::CheckForUpdates`]: super::MenuPick::CheckForUpdates

/// Port: the platform's updater.
pub trait Updater {
    /// Begins the background checks: called once, at launch.
    fn start(&mut self);

    /// Checks now: the player asked.
    fn check_now(&mut self);

    /// Whether it can check at all, so Check for Updates… is offered.
    /// `true` by default; only [`NoUpdater`] can't.
    fn can_check(&self) -> bool {
        true
    }
}

/// The updater of a build that has none: it does nothing, and Check for
/// Updates… is greyed.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoUpdater;

impl Updater for NoUpdater {
    fn start(&mut self) {}

    fn check_now(&mut self) {}

    fn can_check(&self) -> bool {
        false
    }
}

/// Port: what the platform offers to update with.
pub trait UpdaterSource {
    /// Whether the build names a feed to update from: a bundle built for
    /// a channel, with the key its updates are checked with.
    fn feed_configured(&self) -> bool;

    /// The platform's updater, loaded and not started, or why it couldn't
    /// be.
    fn load(&self) -> Result<Box<dyn Updater>, String>;
}

/// The updater chosen at launch, and the warning to report about it, if
/// any.
pub struct Chosen {
    /// The updater the app takes.
    pub updater: Box<dyn Updater>,
    /// Why a build that names a feed runs without its updater.
    pub warning: Option<String>,
}

/// The updater `source` gives: [`NoUpdater`], quietly, when the build names
/// no feed (`cargo run`, a development bundle), without loading anything;
/// the loaded updater when it names one; and [`NoUpdater`] with a warning
/// when it names one but the updater won't load, which is a broken bundle.
pub fn choose(source: &dyn UpdaterSource) -> Chosen {
    if !source.feed_configured() {
        return Chosen {
            updater: Box::new(NoUpdater),
            warning: None,
        };
    }
    match source.load() {
        Ok(updater) => Chosen {
            updater,
            warning: None,
        },
        Err(error) => Chosen {
            updater: Box::new(NoUpdater),
            warning: Some(format!("nova: updates are off: {error}")),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::*;

    /// What a recording updater was asked.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Call {
        Start,
        CheckNow,
    }

    /// Records each call into a log the test keeps.
    struct RecordingUpdater {
        calls: Rc<RefCell<Vec<Call>>>,
        can_check: bool,
    }

    impl Updater for RecordingUpdater {
        fn start(&mut self) {
            self.calls.borrow_mut().push(Call::Start);
        }

        fn check_now(&mut self) {
            self.calls.borrow_mut().push(Call::CheckNow);
        }

        fn can_check(&self) -> bool {
            self.can_check
        }
    }

    /// Names a feed or not, and loads a recording updater or fails;
    /// counts its loads.
    struct FakeSource {
        feed: bool,
        load: Result<Rc<RefCell<Vec<Call>>>, String>,
        loads: Cell<usize>,
    }

    impl FakeSource {
        fn new(feed: bool, load: Result<Rc<RefCell<Vec<Call>>>, String>) -> Self {
            Self {
                feed,
                load,
                loads: Cell::new(0),
            }
        }
    }

    impl UpdaterSource for FakeSource {
        fn feed_configured(&self) -> bool {
            self.feed
        }

        fn load(&self) -> Result<Box<dyn Updater>, String> {
            self.loads.set(self.loads.get() + 1);
            self.load.clone().map(|calls| {
                Box::new(RecordingUpdater {
                    calls,
                    can_check: true,
                }) as Box<dyn Updater>
            })
        }
    }

    /// An updater that keeps the default `can_check`.
    struct Plain;

    impl Updater for Plain {
        fn start(&mut self) {}

        fn check_now(&mut self) {}
    }

    #[test]
    fn no_feed_is_the_no_op_updater_without_a_warning_and_never_loads() {
        let source = FakeSource::new(false, Ok(Rc::default()));
        let chosen = choose(&source);
        assert!(!chosen.updater.can_check(), "the no-op updater");
        assert_eq!(chosen.warning, None);
        assert_eq!(source.loads.get(), 0);
    }

    #[test]
    fn a_feed_and_a_loaded_framework_is_the_loaded_updater() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let source = FakeSource::new(true, Ok(Rc::clone(&calls)));
        let mut chosen = choose(&source);
        assert_eq!(chosen.warning, None);
        assert_eq!(source.loads.get(), 1);
        assert!(chosen.updater.can_check());
        chosen.updater.start();
        chosen.updater.check_now();
        assert_eq!(*calls.borrow(), [Call::Start, Call::CheckNow]);
    }

    #[test]
    fn a_feed_without_the_framework_is_the_no_op_updater_with_a_warning() {
        let source = FakeSource::new(true, Err("there is no Sparkle.framework".into()));
        let chosen = choose(&source);
        assert!(!chosen.updater.can_check());
        assert_eq!(
            chosen.warning.as_deref(),
            Some("nova: updates are off: there is no Sparkle.framework")
        );
        assert_eq!(source.loads.get(), 1);
    }

    #[test]
    fn the_no_op_updater_does_nothing_and_cannot_check() {
        let mut updater = NoUpdater;
        updater.start();
        updater.check_now();
        assert!(!updater.can_check());
        assert!(Plain.can_check(), "an updater can check by default");
    }
}
