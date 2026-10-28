//! The FFI: the Apple event handlers, registered from [`APPLE_EVENTS`]
//! with `NSAppleEventManager`, and the target they call.
//!
//! Every `unsafe` block here carries a `SAFETY` comment. The class and ID
//! of an event (`AEEventClass`, `AEEventID`) are `FourCharCode`s, a C
//! `UInt32` (encoding `I`): they are passed and read as `u32` with
//! `msg_send!`, so the crate needs no `CoreServices` bindings.

#![allow(unsafe_code)]

use std::fmt;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventManager, NSObject};

use crate::apple_events::{APPLE_EVENTS, command_for, four_char_code};
use crate::bar::InstallError;
use crate::items::Command;

/// The target's state: what it forwards each event's command to.
struct TargetIvars {
    on_command: Box<dyn Fn(Command)>,
}

define_class!(
    // SAFETY:
    // - NSObject has no subclassing requirements.
    // - `AppleEventTarget` does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "NovaAppleEventTarget"]
    #[ivars = TargetIvars]
    struct AppleEventTarget;

    impl AppleEventTarget {
        /// An Apple event arrived: its class and ID say which command, and
        /// an event the game doesn't handle is ignored. The reply is left
        /// empty, which is success, as the original's handlers return
        /// `noErr`. The closure only forwards, so `AppKit`'s callback never
        /// runs the game.
        // SAFETY: `handleAppleEvent:withReplyEvent:` is the selector this
        // crate registers; the manager calls it with the event and its
        // reply, two `NSAppleEventDescriptor`s, and wants nothing back.
        #[unsafe(method(handleAppleEvent:withReplyEvent:))]
        fn handle_apple_event(
            &self,
            event: &NSAppleEventDescriptor,
            _reply: &NSAppleEventDescriptor,
        ) {
            // SAFETY: `eventClass` and `eventID` take nothing and return
            // an `AEEventClass` and an `AEEventID`, both `UInt32`.
            let (class, id): (u32, u32) =
                unsafe { (msg_send![event, eventClass], msg_send![event, eventID]) };
            if let Some(command) = command_for(class, id) {
                (self.ivars().on_command)(command);
            }
        }
    }
);

impl AppleEventTarget {
    fn new(mtm: MainThreadMarker, on_command: Box<dyn Fn(Command)>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(TargetIvars { on_command });
        // SAFETY: NSObject's `init` on a freshly allocated instance whose
        // ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

/// The installed Apple event handlers, one for each of [`APPLE_EVENTS`].
///
/// The quit event (the Dock's Quit, logout, restart, shutdown,
/// `osascript`'s `quit app`) sends the game's quit, [`Command::Quit`],
/// never `AppKit`'s `terminate:`, which would end the process with no
/// chance to save.
///
/// `NSAppleEventManager` holds a handler only weakly, so this keeps the
/// target: keep it for the whole run. Dropping it removes the handlers,
/// and the events then go unhandled (`AppKit`'s own handlers aren't put
/// back).
pub struct AppleEvents {
    _target: Retained<AppleEventTarget>,
}

impl fmt::Debug for AppleEvents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppleEvents").finish_non_exhaustive()
    }
}

impl AppleEvents {
    /// Registers a handler for each of [`APPLE_EVENTS`], calling
    /// `on_command` with each event's command.
    ///
    /// It needs the main thread. For the whole run, install it after
    /// `NSApplication` has finished launching (winit's `resumed`):
    /// `finishLaunching` installs `AppKit`'s own handlers, the quit
    /// event's calling `terminate:`, so one registered before then is
    /// replaced. Before launch there are no handlers at all, so one
    /// installed for a while there (around the startup's open panel) works
    /// until it is dropped, which should be before the app launches. Off
    /// the main thread it registers nothing.
    ///
    /// `on_command` runs inside `AppKit`'s event dispatch: it should only
    /// pass the command on (to an event loop proxy, say), never run the
    /// game.
    pub fn install(on_command: impl Fn(Command) + 'static) -> Result<Self, InstallError> {
        let mtm = MainThreadMarker::new().ok_or(InstallError::NotMainThread)?;
        let target = AppleEventTarget::new(mtm, Box::new(on_command));
        let handler: &AnyObject = &target;
        let manager = NSAppleEventManager::sharedAppleEventManager();
        for event in &APPLE_EVENTS {
            let (class, id) = (four_char_code(event.class), four_char_code(event.id));
            // SAFETY: the target responds to
            // `handleAppleEvent:withReplyEvent:`, the selector given, and
            // `AppleEvents` keeps it alive while it is registered (its drop
            // removes it before the target goes). The class and ID are
            // `UInt32`s.
            unsafe {
                let () = msg_send![
                    &*manager,
                    setEventHandler: handler,
                    andSelector: sel!(handleAppleEvent:withReplyEvent:),
                    forEventClass: class,
                    andEventID: id,
                ];
            }
        }
        Ok(Self { _target: target })
    }
}

impl Drop for AppleEvents {
    /// Removes the handlers before the target goes, so a later event calls
    /// no one.
    fn drop(&mut self) {
        let manager = NSAppleEventManager::sharedAppleEventManager();
        for event in &APPLE_EVENTS {
            let (class, id) = (four_char_code(event.class), four_char_code(event.id));
            // SAFETY: the class and ID are `UInt32`s; removing a handler
            // is always valid.
            unsafe {
                let () = msg_send![
                    &*manager,
                    removeEventHandlerForEventClass: class,
                    andEventID: id,
                ];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_the_main_thread_nothing_is_installed() {
        let result = std::thread::spawn(|| AppleEvents::install(|_| {}).map(|_| ()))
            .join()
            .expect("the thread ends");
        assert_eq!(result, Err(InstallError::NotMainThread));
    }
}
