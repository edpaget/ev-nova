//! Whether the player has turned on full keyboard access, the system
//! setting under which `AppKit` puts every control in a window's Tab loop.

use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;

/// Whether keyboard navigation is on in System Settings > Keyboard
/// (`-[NSApplication isFullKeyboardAccessEnabled]`, the
/// `AppleKeyboardUIMode` default): `Some(true)` on, `Some(false)` off.
/// It is one message, cheap enough to send each time it is wanted, and
/// `AppKit` sends no notice when it changes. Off the main thread, where
/// `NSApplication` can't be reached, `None`.
#[must_use]
pub fn full_keyboard_access() -> Option<bool> {
    let mtm = MainThreadMarker::new()?;
    Some(NSApplication::sharedApplication(mtm).isFullKeyboardAccessEnabled())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No test reads the real setting; off the main thread nothing is
    /// read at all.
    #[test]
    fn off_the_main_thread_it_answers_nothing() {
        let answer = std::thread::spawn(full_keyboard_access)
            .join()
            .expect("returns without panicking");
        assert_eq!(answer, None);
    }
}
