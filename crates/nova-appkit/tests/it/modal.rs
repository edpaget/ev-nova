//! `abort_modal` on the real `NSApplication`, on the main thread: a no-op
//! with no modal running, and the end of a modal session that is. Making
//! the window and running the session take `msg_send!`-level calls that
//! `objc2-app-kit` marks unsafe, so this module allows unsafe code, every
//! block with a `SAFETY` comment.

#![allow(unsafe_code)]

use nova_appkit::abort_modal;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSModalResponseAbort, NSModalResponseContinue, NSWindow};

pub fn with_no_modal_running_it_does_nothing() {
    let mtm = MainThreadMarker::new().expect("the test binary's main thread");
    let app = NSApplication::sharedApplication(mtm);
    assert!(app.modalWindow().is_none(), "no modal before");
    abort_modal();
    assert!(app.modalWindow().is_none(), "none after");
}

pub fn it_aborts_a_running_modal_session() {
    let mtm = MainThreadMarker::new().expect("the test binary's main thread");
    let app = NSApplication::sharedApplication(mtm);
    // SAFETY: `new` on the main thread makes a window, which this test
    // keeps until it has ended the session and ordered it out; it is
    // never closed, so `releasedWhenClosed` never applies.
    let window = unsafe { NSWindow::new(mtm) };
    let session = app.beginModalSessionForWindow(&window);
    assert_eq!(
        app.modalWindow().as_deref(),
        Some(&*window),
        "the session's window is the modal one"
    );
    // SAFETY: `session` is the live session just begun.
    let before = unsafe { app.runModalSession(session) };
    assert_eq!(before, NSModalResponseContinue, "the session runs on");

    abort_modal();

    // SAFETY: `session` is still the live session; ending it once, after
    // its last run, is how a session is closed.
    let after = unsafe { app.runModalSession(session) };
    unsafe { app.endModalSession(session) };
    window.orderOut(None);
    assert_eq!(after, NSModalResponseAbort, "aborted");
    assert!(app.modalWindow().is_none(), "no modal left");
}
