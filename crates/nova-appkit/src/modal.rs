//! Ending a modal that `AppKit` is running, such as the startup's open
//! panel, from outside it.

use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;

/// Aborts the modal `NSApplication` is running, if any: its `runModal`
/// (or modal session) returns `NSModalResponseAbort`, which an open
/// panel's caller reads as a cancel.
///
/// It is `abortModal`, not `stopModal`, because the call comes from
/// outside the modal's own event handling (an Apple event handler, say),
/// where Apple documents `stopModal` as having no effect until the next
/// event. Off the main thread, or with no modal running, it does nothing.
pub fn abort_modal() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    if app.modalWindow().is_some() {
        app.abortModal();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_the_main_thread_it_does_nothing() {
        std::thread::spawn(abort_modal)
            .join()
            .expect("returns without panicking");
    }
}
