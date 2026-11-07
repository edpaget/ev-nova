//! The key names port's adapters: what the player's keyboard layout
//! prints on a key, asked again each time a key is named, so a layout
//! change shows at the next draw, as in the original's Keys window
//! (rdm:phase/key-bindings/phase-4-keys-screen's findings).
//!
//! - **macOS**: `nova_appkit::key_label`, by the key's virtual key code:
//!   Text Input Sources' current layout through `UCKeyTranslate`, with the
//!   original's arguments. It answers only on the main thread, where the
//!   game names keys (the keys screen's `refresh`, inside `App::handle`);
//!   off it, every key reads its US legend.
//! - **Windows**: `nova_win32::key_label`, by the key's extended
//!   scancode: the thread's layout through `MapVirtualKeyExW` and
//!   `ToUnicodeEx`.
//! - **Linux** (and any other target):
//!   [`NoKeyNames`](nova_view::NoKeyNames), so every key reads its US
//!   legend. winit 0.30 keeps its xkb keymap to itself: on Wayland only
//!   the seat's keyboard owner (winit) receives the keymap, and on X11
//!   reading it needs a second X connection and `xkbcommon-x11`.
//!
//! The native code each key asks about comes from winit's `to_scancode`,
//! over `key_code`, [`map_key`](super::map_key) the other way, so neither
//! platform crate keeps a key table.

use std::rc::Rc;

use nova_view::KeyNames;
#[cfg(not(any(target_os = "macos", windows)))]
use nova_view::NoKeyNames;

/// Key names from the platform's keyboard layout, through `label`: the
/// platform's layout lookup by native key code (the macOS virtual key
/// code, or the Windows extended scancode), which winit's `to_scancode`
/// gives for each key. Its one decision is which native code to ask
/// about ([`key_code`](super::key_code)); it caches nothing, so each
/// name asks the layout again.
#[cfg(any(target_os = "macos", windows))]
pub(crate) struct LayoutKeyNames<F> {
    label: F,
}

#[cfg(any(target_os = "macos", windows))]
impl<F: Fn(u32) -> Option<String>> LayoutKeyNames<F> {
    /// Names asked of `label`.
    pub(crate) fn new(label: F) -> Self {
        Self { label }
    }
}

#[cfg(any(target_os = "macos", windows))]
impl<F> std::fmt::Debug for LayoutKeyNames<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LayoutKeyNames")
    }
}

#[cfg(any(target_os = "macos", windows))]
impl<F: Fn(u32) -> Option<String>> KeyNames for LayoutKeyNames<F> {
    fn name(&self, key: nova_view::Key) -> Option<String> {
        use winit::platform::scancode::PhysicalKeyExtScancode;
        super::key_code(key)?.to_scancode().and_then(&self.label)
    }
}

/// The running platform's key names: the layout's on macOS and Windows
/// (`LayoutKeyNames` over `nova_appkit::key_label` or
/// `nova_win32::key_label`), and [`NoKeyNames`](nova_view::NoKeyNames)
/// elsewhere.
#[must_use]
pub fn system_key_names() -> Rc<dyn KeyNames> {
    #[cfg(target_os = "macos")]
    {
        Rc::new(LayoutKeyNames::new(|code| {
            nova_appkit::key_label(u16::try_from(code).ok()?)
        }))
    }
    #[cfg(windows)]
    {
        Rc::new(LayoutKeyNames::new(nova_win32::key_label))
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        Rc::new(NoKeyNames)
    }
}

#[cfg(test)]
mod tests {
    use nova_view::key_names::fixture::check_contract;

    use super::*;

    #[cfg(any(target_os = "macos", windows))]
    mod layout {
        use std::cell::RefCell;

        use nova_view::Key;
        use nova_view::key_names::fixture::check_contract;
        use winit::keyboard::KeyCode;
        use winit::platform::scancode::PhysicalKeyExtScancode;

        use super::super::LayoutKeyNames;
        use nova_view::KeyNames;

        #[test]
        fn each_name_asks_the_layout_again_with_the_key_s_native_code() {
            let a = KeyCode::KeyA.to_scancode().expect("A has a native code");
            let asked = RefCell::new(Vec::new());
            let names = LayoutKeyNames::new(|code| {
                asked.borrow_mut().push(code);
                (code == a).then(|| "q".to_owned())
            });
            assert_eq!(names.name(Key::KeyA), Some("q".to_owned()));
            assert_eq!(names.name(Key::KeyA), Some("q".to_owned()));
            assert_eq!(*asked.borrow(), vec![a, a]);
        }

        #[test]
        fn the_keypad_s_minus_asks_about_the_keypad_s_key() {
            let pad = KeyCode::NumpadSubtract.to_scancode();
            let asked = RefCell::new(Vec::new());
            let names = LayoutKeyNames::new(|code| {
                asked.borrow_mut().push(code);
                None
            });
            assert_eq!(names.name(Key::NumpadSubtract), None);
            assert_eq!(asked.borrow().first().copied(), pad);
            assert_ne!(pad, KeyCode::Minus.to_scancode());
        }

        #[test]
        fn a_key_with_no_native_code_never_asks() {
            let asked = RefCell::new(0);
            let names = LayoutKeyNames::new(|_| {
                *asked.borrow_mut() += 1;
                Some("x".to_owned())
            });
            assert_eq!(names.name(Key::Unidentified), None);
            assert_eq!(*asked.borrow(), 0);
        }

        #[test]
        fn the_layout_adapter_meets_the_port_s_contract() {
            let every = LayoutKeyNames::new(|code| Some(format!("{code:x}")));
            assert_eq!(check_contract(&every), Ok(()));
            let none = LayoutKeyNames::new(|_| None);
            assert_eq!(check_contract(&none), Ok(()));
        }

        #[test]
        fn its_debug_names_the_adapter() {
            let names = LayoutKeyNames::new(|_| None);
            assert_eq!(format!("{names:?}"), "LayoutKeyNames");
        }
    }

    #[test]
    fn the_platform_s_key_names_meet_the_port_s_contract() {
        // Real answers on Windows; on macOS the test thread isn't the main
        // thread, so `nova_appkit` answers nothing (its own main-thread
        // tests read the real layout); on Linux, `NoKeyNames`.
        assert_eq!(check_contract(&*system_key_names()), Ok(()));
    }

    #[test]
    fn the_platform_wires_its_layout_adapter_where_it_has_one() {
        let wired = format!("{:?}", system_key_names());
        let expected = if cfg!(any(target_os = "macos", windows)) {
            "LayoutKeyNames"
        } else {
            "NoKeyNames"
        };
        assert_eq!(wired, expected);
    }

    #[cfg(windows)]
    #[test]
    fn the_windows_adapter_asks_win32_about_each_key_s_own_scancode() {
        use nova_view::Key;
        use winit::keyboard::KeyCode;
        use winit::platform::scancode::PhysicalKeyExtScancode;
        let names = system_key_names();
        assert!(names.name(Key::KeyA).is_some(), "every layout prints there");
        for (key, code) in [
            (Key::KeyA, KeyCode::KeyA),
            (Key::Semicolon, KeyCode::Semicolon),
        ] {
            let scancode = code.to_scancode().expect("a scancode");
            assert_eq!(names.name(key), nova_win32::key_label(scancode), "{key:?}");
        }
    }

    #[cfg(not(any(target_os = "macos", windows)))]
    #[test]
    fn elsewhere_no_key_is_named_by_the_layout() {
        assert_eq!(system_key_names().name(nova_view::Key::KeyW), None);
    }
}
