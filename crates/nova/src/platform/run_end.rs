//! The run's end, done once, whichever of its two triggers comes first.
//!
//! When the run is over the runner tells the app it is quitting
//! (`App::exiting`, which saves the pilot flying and leaves an exclusive
//! mode) and prints what the app reports. Two things can say the run is
//! over:
//!
//! - the runner's drop, after the event loop returns: every run whose
//!   loop returns ends there, the game's own quits included;
//! - on macOS, `NSApplicationWillTerminateNotification`, for a
//!   `terminate:` that bypassed the game (sent by code in the process
//!   itself, or a quit Apple event `AppKit` answers because the game's
//!   handlers couldn't be installed). `AppKit` calls `exit()` straight
//!   after it,
//!   so the loop never returns and nothing is dropped: winit-appkit 0.31's
//!   `applicationWillTerminate:` drops only its boxed `&mut` borrow of the
//!   runner (`EventHandler::terminate`), and `run_app` would drop the
//!   runner itself only after `run_app_on_demand` returned. The observer
//!   (`nova_appkit::WillTerminate`) calls [`Run::ender`]'s callback.
//!
//! [`Run`] holds the running state, and ends it ([`Ending::end`]) on the
//! first of the two; the other then does nothing. The state is kept, not
//! dropped, when the observer ends it, as it holds the observer whose
//! callback is running.

use std::cell::{RefCell, RefMut};
use std::rc::Rc;

/// What a run does once it is over.
pub(super) trait Ending {
    /// The run is over: does what ends it. [`Run`] calls it once.
    fn end(&mut self);
}

/// Where a run is: its state once started, and whether it has ended.
/// An ended run keeps its state until it is dropped.
struct State<T> {
    running: Option<T>,
    ended: bool,
}

/// The running state, shared with the callback [`Run::ender`] gives, and
/// ended once: by that callback or by the run's drop, whichever comes
/// first.
pub(super) struct Run<T: Ending> {
    state: Rc<RefCell<State<T>>>,
}

impl<T: Ending> Run<T> {
    /// A run not yet started.
    pub(super) fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(State {
                running: None,
                ended: false,
            })),
        }
    }

    /// Starts the run with `running`. A run starts once: given again,
    /// `running` is let go of.
    pub(super) fn start(&mut self, running: T) {
        let mut state = self.state.borrow_mut();
        if state.running.is_none() && !state.ended {
            state.running = Some(running);
        }
    }

    /// The running state, while the run is running: none before it starts
    /// or once it has ended.
    pub(super) fn get(&mut self) -> Option<RefMut<'_, T>> {
        RefMut::filter_map(self.state.borrow_mut(), |state| {
            if state.ended {
                None
            } else {
                state.running.as_mut()
            }
        })
        .ok()
    }

    /// Ends the run, if it is running: [`Ending::end`], once.
    pub(super) fn end(&self) {
        end(&self.state);
    }

    /// A callback that ends the run as [`Run::end`] does, for the
    /// platform's will-terminate notice. It holds the run weakly (the
    /// running state may hold the observer that holds the callback), so
    /// once the run is dropped it does nothing.
    // Only macOS has the notice; the tests run it everywhere.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub(super) fn ender(&self) -> impl Fn() + 'static
    where
        T: 'static,
    {
        let state = Rc::downgrade(&self.state);
        move || {
            if let Some(state) = state.upgrade() {
                end(&state);
            }
        }
    }
}

/// Ends the run in `state` if it is running, keeping its state. A run in
/// use is left as it is, to end on its drop: that is the notice arriving
/// inside one of the runner's own callbacks (in a modal it runs), where
/// winit-appkit has already panicked on its own borrow of the runner.
fn end<T: Ending>(state: &RefCell<State<T>>) {
    let Ok(mut state) = state.try_borrow_mut() else {
        return;
    };
    let state = &mut *state;
    if state.ended {
        return;
    }
    if let Some(running) = &mut state.running {
        running.end();
        state.ended = true;
    }
}

/// The run's drop is the trigger every run whose loop returns ends by.
impl<T: Ending> Drop for Run<T> {
    fn drop(&mut self) {
        self.end();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    /// Counts its ends and its drops.
    struct Counted {
        ends: Rc<Cell<u32>>,
        drops: Rc<Cell<u32>>,
    }

    impl Ending for Counted {
        fn end(&mut self) {
            self.ends.set(self.ends.get() + 1);
        }
    }

    impl Drop for Counted {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    /// A started run, with its ends and drops counted.
    fn started() -> (Run<Counted>, Rc<Cell<u32>>, Rc<Cell<u32>>) {
        let (ends, drops) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let mut run = Run::new();
        run.start(Counted {
            ends: Rc::clone(&ends),
            drops: Rc::clone(&drops),
        });
        (run, ends, drops)
    }

    #[test]
    fn a_run_never_started_has_nothing_to_end() {
        let mut run = Run::<Counted>::new();
        assert!(run.get().is_none());
        run.ender()();
        run.end();
        assert!(run.get().is_none(), "still nothing");
    }

    #[test]
    fn the_running_state_is_reachable_while_running() {
        let (mut run, ends, _) = started();
        run.get().expect("running").end();
        assert_eq!(ends.get(), 1, "the state itself");
    }

    #[test]
    fn dropping_the_run_ends_it_once() {
        let (run, ends, drops) = started();
        assert_eq!(ends.get(), 0);
        drop(run);
        assert_eq!((ends.get(), drops.get()), (1, 1));
    }

    #[test]
    fn the_ender_ends_it_once_and_the_drop_then_does_nothing() {
        let (mut run, ends, drops) = started();
        let ender = run.ender();
        ender();
        assert_eq!(ends.get(), 1);
        assert_eq!(drops.get(), 0, "kept: it holds the observer calling");
        assert!(run.get().is_none(), "no longer running");
        ender();
        drop(run);
        assert_eq!((ends.get(), drops.get()), (1, 1));
    }

    #[test]
    fn ending_then_the_ender_ends_it_once() {
        let (run, ends, _) = started();
        run.end();
        run.end();
        run.ender()();
        drop(run);
        assert_eq!(ends.get(), 1);
    }

    #[test]
    fn the_ender_does_nothing_once_the_run_is_gone() {
        let (run, ends, drops) = started();
        let ender = run.ender();
        drop(run);
        ender();
        assert_eq!((ends.get(), drops.get()), (1, 1));
    }

    #[test]
    fn the_ender_does_nothing_while_the_run_is_in_use_and_the_drop_ends_it() {
        let (mut run, ends, _) = started();
        let ender = run.ender();
        {
            let _in_use = run.get().expect("running");
            ender();
            assert_eq!(ends.get(), 0, "no panic, and nothing ended");
        }
        assert!(run.get().is_some(), "still running");
        drop(run);
        assert_eq!(ends.get(), 1);
    }

    #[test]
    fn a_second_start_is_ignored() {
        let (mut run, ends, drops) = started();
        let (other_ends, other_drops) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        run.start(Counted {
            ends: Rc::clone(&other_ends),
            drops: Rc::clone(&other_drops),
        });
        assert_eq!(other_drops.get(), 1, "let go at once");
        drop(run);
        assert_eq!((ends.get(), drops.get()), (1, 1), "the first ends");
        assert_eq!(other_ends.get(), 0);
    }
}
