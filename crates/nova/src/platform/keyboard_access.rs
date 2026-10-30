//! The keyboard access port's adapters: on macOS, `nova_appkit`'s read of
//! full keyboard access; elsewhere, no setting.

use std::rc::Rc;

use crate::app::KeyboardAccess;

/// The full keyboard access setting, read through `nova_appkit` each time
/// it is asked. Off the main thread it answers `None`, as a platform with
/// no setting does.
#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Default)]
pub struct AppKitKeyboardAccess;

#[cfg(target_os = "macos")]
impl KeyboardAccess for AppKitKeyboardAccess {
    fn full_keyboard_access(&self) -> Option<bool> {
        nova_appkit::full_keyboard_access()
    }
}

/// The running platform's keyboard access setting: `AppKit`'s on macOS,
/// and none ([`NoKeyboardAccessSetting`](crate::app::NoKeyboardAccessSetting))
/// elsewhere.
#[must_use]
pub fn system_keyboard_access() -> Rc<dyn KeyboardAccess> {
    #[cfg(target_os = "macos")]
    {
        Rc::new(AppKitKeyboardAccess)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Rc::new(crate::app::NoKeyboardAccessSetting)
    }
}
