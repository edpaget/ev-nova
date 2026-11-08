//! The real will-terminate observer, installed on the main thread and
//! sent `NSApplicationWillTerminateNotification` by this test, which the
//! notification center hands straight to it. Posting takes calls that
//! `objc2-foundation` marks unsafe, so this module allows unsafe code,
//! every block with a `SAFETY` comment.

#![allow(unsafe_code)]

use std::cell::Cell;
use std::rc::Rc;

use nova_appkit::WillTerminate;
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplicationWillTerminateNotification;
use objc2_foundation::{NSNotificationCenter, NSObject, NSString};

/// Posts `name` from an object of this test's own, so only an observer
/// of every poster (as the game's is, of `NSApplication`'s post) hears
/// it, and `AppKit`'s own observers of `NSApplication` don't.
fn post(name: &NSString) {
    let poster = NSObject::new();
    // SAFETY: the name is an `NSString` and the object an `NSObject`, the
    // types the method takes.
    unsafe {
        NSNotificationCenter::defaultCenter().postNotificationName_object(name, Some(&poster));
    }
}

pub fn the_observer_calls_back_on_will_terminate_until_dropped() {
    MainThreadMarker::new().expect("the test binary's main thread");
    let calls = Rc::new(Cell::new(0));
    let observer = {
        let calls = Rc::clone(&calls);
        WillTerminate::install(move || calls.set(calls.get() + 1)).expect("on the main thread")
    };
    assert_eq!(format!("{observer:?}"), "WillTerminate { .. }");

    post(&NSString::from_str("NovaSomethingElse"));
    assert_eq!(calls.get(), 0, "another notification calls nothing");
    // SAFETY: an `NSString` constant `AppKit` defines.
    let will_terminate = unsafe { NSApplicationWillTerminateNotification };
    post(will_terminate);
    assert_eq!(calls.get(), 1, "the callback runs, during the post");
    post(will_terminate);
    assert_eq!(calls.get(), 2, "each time");

    // Dropped, it is no longer observing.
    drop(observer);
    post(will_terminate);
    assert_eq!(calls.get(), 2, "nothing more");
}
