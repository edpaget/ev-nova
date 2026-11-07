//! The FFI: an observer of `NSApplicationWillTerminateNotification`, and
//! the target the notification center calls.
//!
//! Every `unsafe` block here carries a `SAFETY` comment.

#![allow(unsafe_code)]

use std::fmt;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::NSApplicationWillTerminateNotification;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObject};

use crate::bar::InstallError;

/// The target's state: what it calls on the notice.
struct TargetIvars {
    on_terminate: Box<dyn Fn()>,
}

define_class!(
    // SAFETY:
    // - NSObject has no subclassing requirements.
    // - `WillTerminateTarget` does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NovaWillTerminateTarget"]
    #[ivars = TargetIvars]
    struct WillTerminateTarget;

    impl WillTerminateTarget {
        /// The application is about to terminate: calls the callback.
        // SAFETY: `willTerminate:` is the selector this crate registers;
        // the notification center calls it with the `NSNotification` and
        // wants nothing back.
        #[unsafe(method(willTerminate:))]
        fn will_terminate(&self, _notification: &NSNotification) {
            (self.ivars().on_terminate)();
        }
    }
);

impl WillTerminateTarget {
    fn new(mtm: MainThreadMarker, on_terminate: Box<dyn Fn()>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TargetIvars { on_terminate });
        // SAFETY: NSObject's `init` on a freshly allocated instance whose
        // ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

/// The installed observer of `NSApplicationWillTerminateNotification`,
/// which `NSApplication` posts from `terminate:` just before it calls
/// `exit()`.
///
/// It is the game's last word on a `terminate:` that bypassed it, sent by
/// code in the process itself, or a quit Apple event `AppKit` answers
/// because [`AppleEvents`] couldn't be installed: the process
/// ends right after the callback returns, with no event loop returning
/// and nothing dropped. Unlike the menu bar's and the Apple events'
/// callbacks, which only pass a command on, this one must do its work
/// there and then.
///
/// The notification center holds the observer only weakly, so this keeps
/// the target: keep it for the whole run. Dropping it stops the
/// observing.
///
/// [`AppleEvents`]: crate::AppleEvents
pub struct WillTerminate {
    target: Retained<WillTerminateTarget>,
}

impl fmt::Debug for WillTerminate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WillTerminate").finish_non_exhaustive()
    }
}

impl WillTerminate {
    /// Observes `NSApplicationWillTerminateNotification` from any poster,
    /// calling `on_terminate` each time it is posted, inside the post.
    ///
    /// It needs the main thread, where `NSApplication` posts it. Off the
    /// main thread it observes nothing.
    pub fn install(on_terminate: impl Fn() + 'static) -> Result<Self, InstallError> {
        let mtm = MainThreadMarker::new().ok_or(InstallError::NotMainThread)?;
        let target = WillTerminateTarget::new(mtm, Box::new(on_terminate));
        let observer: &AnyObject = &target;
        // SAFETY: the target responds to `willTerminate:`, the selector
        // given, which takes the notification; `WillTerminate` keeps it
        // alive while it is registered (its drop removes it before the
        // target goes). The name is an `NSString` constant `AppKit`
        // defines, and no object means every poster.
        unsafe {
            NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
                observer,
                sel!(willTerminate:),
                Some(NSApplicationWillTerminateNotification),
                None,
            );
        }
        Ok(Self { target })
    }
}

impl Drop for WillTerminate {
    /// Removes the observer before the target goes, so a later notice
    /// calls no one.
    fn drop(&mut self) {
        let observer: &AnyObject = &self.target;
        // SAFETY: removing an observer, registered or not, is always valid.
        unsafe {
            NSNotificationCenter::defaultCenter().removeObserver(observer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_the_main_thread_nothing_is_observed() {
        let result = std::thread::spawn(|| WillTerminate::install(|| {}).map(|_| ()))
            .join()
            .expect("the thread ends");
        assert_eq!(result, Err(InstallError::NotMainThread));
    }
}
