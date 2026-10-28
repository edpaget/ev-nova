//! The real Apple event handlers, installed on the main thread and sent
//! events by this process, which the Apple Event Manager hands straight
//! to the handler. Building and sending an event takes `msg_send!`, so
//! this module allows unsafe code, every block with a `SAFETY` comment.

#![allow(unsafe_code)]

use std::cell::RefCell;
use std::rc::Rc;

use nova_appkit::{AppleEvents, Command, four_char_code};
use objc2::MainThreadMarker;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyClass;
use objc2_foundation::NSAppleEventDescriptor;

/// `kAutoGenerateReturnID`.
const AUTO_GENERATE_RETURN_ID: i16 = -1;
/// `kAnyTransactionID`.
const ANY_TRANSACTION_ID: i32 = 0;
/// `NSAppleEventSendWaitForReply | NSAppleEventSendNeverInteract`.
const WAIT_NEVER_INTERACT: usize = 3 | 16;

/// Sends this process the Apple event `class`/`id` and waits for the
/// handler, which runs before this returns.
fn send(class: [u8; 4], id: [u8; 4]) {
    let descriptor = AnyClass::get(c"NSAppleEventDescriptor").expect("the class");
    let me = NSAppleEventDescriptor::currentProcessDescriptor();
    // SAFETY: the class method takes two `UInt32`s, a descriptor, an
    // `SInt16` and an `SInt32`, and returns an autoreleased descriptor.
    let event: Retained<NSAppleEventDescriptor> = unsafe {
        msg_send![
            descriptor,
            appleEventWithEventClass: four_char_code(class),
            eventID: four_char_code(id),
            targetDescriptor: &*me,
            returnID: AUTO_GENERATE_RETURN_ID,
            transactionID: ANY_TRANSACTION_ID,
        ]
    };
    // SAFETY: the options are an `NSUInteger`, the timeout an
    // `NSTimeInterval`, and the error out-pointer may be null. The reply
    // (none when the event went unhandled) isn't needed.
    let _reply: Option<Retained<NSAppleEventDescriptor>> = unsafe {
        msg_send![
            &*event,
            sendEventWithOptions: WAIT_NEVER_INTERACT,
            timeout: 5.0_f64,
            error: std::ptr::null_mut::<*mut objc2::runtime::AnyObject>(),
        ]
    };
}

pub fn the_installed_handlers_send_the_games_commands_until_dropped() {
    MainThreadMarker::new().expect("the test binary's main thread");
    let got = Rc::new(RefCell::new(Vec::new()));
    let events = {
        let got = Rc::clone(&got);
        AppleEvents::install(move |command| got.borrow_mut().push(command))
            .expect("on the main thread")
    };
    assert_eq!(format!("{events:?}"), "AppleEvents { .. }");

    // Preferences first: had the install registered nothing, a quit event
    // could reach `AppKit`'s `terminate:` and end this binary.
    send(*b"aevt", *b"pref");
    assert_eq!(*got.borrow(), [Command::Preferences], "the handler runs");
    send(*b"aevt", *b"quit");
    send(*b"aevt", *b"odoc");
    assert_eq!(
        *got.borrow(),
        [Command::Preferences, Command::Quit],
        "the quit event is the game's quit; an unhandled event sends nothing"
    );

    // Dropped, the handlers are removed: an event calls no one.
    drop(events);
    send(*b"aevt", *b"pref");
    assert_eq!(got.borrow().len(), 2, "nothing more");
}
