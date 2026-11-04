//! The winit adapter: translates winit's events into the app's, puts a
//! [`WindowPort`] over a real window and runs the event loop. Beside it,
//! [`dialogs`] holds the startup dialogs: the open panel and the alerts.
//!
//! Translation is pure and unit-tested; the window and the runner only
//! forward to the app. A key press that types a character becomes two
//! events: the key ([`key_event`]), then its text ([`text_event`]).
//!
//! The system can make the window fullscreen, or a window again, without
//! the app asking, and winit sends no event for it. So [`window_events`]
//! reports the window's state, [`WindowEvent::FullscreenChanged`], ahead
//! of every resize, occlusion and redraw, and the app acts only on a
//! change.

pub mod console;
pub mod dialogs;
mod keyboard_access;
#[cfg(target_os = "macos")]
mod menu_bar;
mod runner;
#[cfg(any(target_os = "macos", windows))]
mod updater;
mod window;

pub use dialogs::Dialogs;
pub use keyboard_access::system_keyboard_access;
pub use runner::{Runner, event_loop};
#[cfg(target_os = "macos")]
pub use updater::SparkleSource;
#[cfg(windows)]
pub use updater::{DllLoader, winsparkle_source};
pub use window::WinitWindow;

/// The game's app id: `[package.metadata.bundle] identifier` in nova's
/// `Cargo.toml`, which `build.rs` reads. It names the Linux desktop file
/// (`<APP_ID>.desktop`, which nova-bundle writes from the same key), and
/// the window takes it as its Wayland app id and X11 `WM_CLASS`, so docks
/// and task switchers match the window to the desktop file's icon and
/// name.
pub const APP_ID: &str = env!("NOVA_APP_ID");

use std::time::Duration;

use winit::event::{ElementState, MouseScrollDelta, WindowEvent as WinitEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::app::{WheelDelta, WindowEvent, WindowPort};
use nova_view::{Key, MouseButton};

/// The app's event for winit's `event`, or `None` for events the app does
/// not take. Redraws are `None`: the runner reads the clock and sends them
/// itself. Sizes come from the event, or from `window` when the event does
/// not carry one.
pub fn translate(event: &WinitEvent, window: &impl WindowPort) -> Option<WindowEvent> {
    Some(match event {
        WinitEvent::Resized(size) => WindowEvent::Resized {
            size_px: (size.width, size.height),
            scale_factor: window.scale_factor(),
        },
        WinitEvent::ScaleFactorChanged { scale_factor, .. } => WindowEvent::Resized {
            size_px: window.size_px(),
            scale_factor: *scale_factor,
        },
        WinitEvent::KeyboardInput { event, .. } => {
            key_event(event.physical_key, event.state, event.repeat)
        }
        WinitEvent::CursorMoved { position, .. } => WindowEvent::PointerMoved {
            px: (position.x, position.y),
        },
        WinitEvent::CursorLeft { .. } => WindowEvent::PointerLeft,
        WinitEvent::MouseInput { state, button, .. } => WindowEvent::PointerButton {
            button: map_button(*button),
            pressed: state.is_pressed(),
        },
        WinitEvent::MouseWheel { delta, .. } => WindowEvent::Scroll {
            delta: wheel_delta(*delta)?,
        },
        WinitEvent::Focused(false) => WindowEvent::FocusLost,
        WinitEvent::Focused(true) => WindowEvent::FocusGained,
        WinitEvent::CloseRequested => WindowEvent::CloseRequested,
        _ => return None,
    })
}

/// The vertical part of winit's wheel `delta`, toward the text's end for
/// positive, or `None` for one only across. winit's positive y scrolls
/// toward the top (the wheel turned away from the user, or a trackpad
/// swiped down with natural scrolling), so the sign flips.
fn wheel_delta(delta: MouseScrollDelta) -> Option<WheelDelta> {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => (y != 0.0).then(|| WheelDelta::Lines(-y)),
        MouseScrollDelta::PixelDelta(at) => (at.y != 0.0).then(|| WheelDelta::Px(-at.y)),
    }
}

/// The app's events for winit's `event`, in the order the app takes them,
/// with a redraw at `elapsed`, the time since the app started.
///
/// A resize, an occlusion and a redraw are led by the window's fullscreen
/// state as the system now reports it: the app must know it is fullscreen
/// before it takes the display's size, or it would keep that as the
/// window's own. The redraw's report is there because macOS clears the
/// state only once its exit animation ends, after that animation's
/// resizes; every frame reports it, so an exit is always seen.
pub fn window_events(
    event: &WinitEvent,
    window: &impl WindowPort,
    elapsed: Duration,
) -> impl Iterator<Item = WindowEvent> {
    let fullscreen = matches!(
        event,
        WinitEvent::Resized(_) | WinitEvent::Occluded(_) | WinitEvent::RedrawRequested
    )
    .then(|| WindowEvent::FullscreenChanged {
        fullscreen: window.is_fullscreen(),
    });
    let event = match event {
        WinitEvent::RedrawRequested => Some(WindowEvent::Redraw { elapsed }),
        other => translate(other, window),
    };
    fullscreen.into_iter().chain(event)
}

/// A key going down or up; `repeat` is winit's flag for a press the OS
/// sends again while the key is held.
#[must_use]
pub fn key_event(key: PhysicalKey, state: ElementState, repeat: bool) -> WindowEvent {
    WindowEvent::Key {
        key: map_key(key),
        pressed: state.is_pressed(),
        repeat,
    }
}

/// The text a key press types, from winit's `text`: a printable
/// character, or `None` for a release, a press that types nothing, a
/// control character (Return, Backspace, Escape, Tab, Delete) or more than
/// one character (a dead key's composition, which a name field does not
/// need).
#[must_use]
pub fn text_event(text: Option<&str>, state: ElementState) -> Option<WindowEvent> {
    if !state.is_pressed() {
        return None;
    }
    let mut chars = text?.chars();
    let c = chars.next()?;
    (chars.next().is_none() && !c.is_control()).then_some(WindowEvent::Text(c))
}

/// The game's key for a physical key, by its position: each key winit
/// names is the `Key` of the same name, so every letter, digit,
/// punctuation, function, keypad, navigation and editing key reaches the
/// game. Only the pairs below are folded, the second key arriving as the
/// first; a key winit can't identify, and winit's legacy `Meta`, `Hyper`
/// and `Turbo` (which `Key` has only as deprecated names), are
/// [`Key::Unidentified`].
///
/// Which screen reads which key, where it isn't plain:
/// - Page Up and Page Down (Mac key codes 0x74 and 0x79, fn+Up and fn+Down
///   on a Mac laptop) are the shop grids'; Home and End (0x73 and 0x77,
///   fn+Left and fn+Right) a text dialog's.
/// - Backspace deletes the character before a text field's caret: Delete
///   on a Mac keyboard.
/// - `=` is the unshifted `+` key on a US keyboard: with the keypad's `+`,
///   the map's zoom in. The letter keys are read by position, as the
///   original's `_PortFilter` and dialog filters read the Mac key code:
///   W, A, S and D move the system view whatever letters another layout
///   puts there.
/// - `\` is Hyper Select, the original's default, and the key under
///   Escape (`` ` ``) the developer tools' toggle, or flight's Nav Off
///   without them.
/// - Caps Lock toggles double time on each press, an accepted difference
///   from the original's following the lock state. winit 0.30.13 sends no
///   Caps Lock key on macOS (its `key_to_modifier` maps Alt, Control,
///   Super and Shift alone), so there it never arrives.
/// - The right Shift is its own key: flight and the map's click poll the
///   left Shift's key code alone, where the trade, the player info and
///   the map's Tab take either.
/// - The right Option (Alt) is its own key: flight and the race poll the
///   left Option's key code alone, where the exchange's and the
///   outfitter's counts take either, as does Alt-Enter.
/// - The right Control is its own key: flight polls the left Control's
///   key code alone (its secondary fire key).
/// - The keypad's Enter is its own key: the intro and flight's `ackComm`
///   poll Return's key code alone, where the dialogs take both.
/// - F11 is the fullscreen shortcut off a Mac. F12 is flight's frame-rate
///   key: macOS keeps it for itself unless "Use F1, F2, etc. keys as
///   standard function keys" is set, and fn-F12 reaches the game
///   otherwise.
/// - Command is winit's Super (the Windows key off a Mac), so it arrives
///   as `Key::MetaLeft` or `Key::MetaRight`. Flight polls the left
///   Command's key code alone, where the menus' chords take either.
#[must_use]
pub fn map_key(key: PhysicalKey) -> Key {
    /// The arms for keys winit and `Key` name alike.
    macro_rules! same_name {
        ($key:expr; $($name:ident),* $(,)?) => {
            match $key {
                $(PhysicalKey::Code(KeyCode::$name) => Key::$name,)*
                // Command, Super off a Mac.
                PhysicalKey::Code(KeyCode::SuperLeft) => Key::MetaLeft,
                PhysicalKey::Code(KeyCode::SuperRight) => Key::MetaRight,
                // `-` or the keypad's: the map's zoom out.
                PhysicalKey::Code(KeyCode::NumpadSubtract) => Key::Minus,
                // winit's `KeyCode` is `#[non_exhaustive]`; its legacy
                // `Meta`, `Hyper` and `Turbo` land here too.
                _ => Key::Unidentified,
            }
        };
    }
    same_name!(key;
        Backquote, Backslash, BracketLeft, BracketRight, Comma, Digit0, Digit1, Digit2, Digit3,
        Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Equal, IntlBackslash, IntlRo, IntlYen,
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN,
        KeyO, KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Minus, Period,
        Quote, Semicolon, Slash, AltLeft, AltRight, Backspace, CapsLock, ContextMenu, ControlLeft, ControlRight, Enter,
        ShiftLeft, ShiftRight, Space, Tab, Convert, KanaMode, Lang1, Lang2, Lang3, Lang4, Lang5,
        NonConvert, Delete, End, Help, Home, Insert, PageDown, PageUp, ArrowDown, ArrowLeft,
        ArrowRight, ArrowUp, NumLock, Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5,
        Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd, NumpadBackspace, NumpadClear,
        NumpadClearEntry, NumpadComma, NumpadDecimal, NumpadDivide, NumpadEnter, NumpadEqual, NumpadHash,
        NumpadMemoryAdd, NumpadMemoryClear, NumpadMemoryRecall, NumpadMemoryStore,
        NumpadMemorySubtract, NumpadMultiply, NumpadParenLeft, NumpadParenRight, NumpadStar,
        Escape, Fn, FnLock, PrintScreen, ScrollLock, Pause, BrowserBack, BrowserFavorites,
        BrowserForward, BrowserHome, BrowserRefresh, BrowserSearch, BrowserStop, Eject,
        LaunchApp1, LaunchApp2, LaunchMail, MediaPlayPause, MediaSelect, MediaStop,
        MediaTrackNext, MediaTrackPrevious, Power, Sleep, AudioVolumeDown, AudioVolumeMute,
        AudioVolumeUp, WakeUp, Abort, Resume, Suspend, Again, Copy, Cut, Find, Open, Paste,
        Props, Select, Undo, Hiragana, Katakana, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11,
        F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24, F25, F26, F27, F28,
        F29, F30, F31, F32, F33, F34, F35
    )
}

/// The game's mouse button for winit's.
#[must_use]
pub fn map_button(button: winit::event::MouseButton) -> MouseButton {
    match button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        _ => MouseButton::Other,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use winit::dpi::{PhysicalPosition, PhysicalSize};

    use winit::event::{DeviceId, MouseButton as WinitButton, TouchPhase};

    use super::*;

    /// An 800x600-pixel window at two pixels a point, fullscreen or not.
    struct Window {
        fullscreen: bool,
    }

    const WINDOWED: Window = Window { fullscreen: false };

    impl WindowPort for Window {
        fn size_px(&self) -> (u32, u32) {
            (800, 600)
        }

        fn scale_factor(&self) -> f64 {
            2.0
        }

        fn request_redraw(&mut self) {}

        fn is_fullscreen(&self) -> bool {
            self.fullscreen
        }
    }

    #[test]
    fn the_keys_the_game_uses_map_to_its_keys() {
        let cases = [
            (KeyCode::ArrowLeft, Key::ArrowLeft),
            (KeyCode::ArrowRight, Key::ArrowRight),
            (KeyCode::ArrowUp, Key::ArrowUp),
            (KeyCode::ArrowDown, Key::ArrowDown),
            (KeyCode::PageUp, Key::PageUp),
            (KeyCode::PageDown, Key::PageDown),
            (KeyCode::Home, Key::Home),
            (KeyCode::End, Key::End),
            (KeyCode::Enter, Key::Enter),
            (KeyCode::NumpadEnter, Key::NumpadEnter),
            (KeyCode::Escape, Key::Escape),
            (KeyCode::Space, Key::Space),
            (KeyCode::Tab, Key::Tab),
            (KeyCode::Backspace, Key::Backspace),
            (KeyCode::Equal, Key::Equal),
            (KeyCode::NumpadAdd, Key::NumpadAdd),
            (KeyCode::Minus, Key::Minus),
            (KeyCode::NumpadSubtract, Key::Minus),
            (KeyCode::KeyW, Key::KeyW),
            (KeyCode::KeyA, Key::KeyA),
            (KeyCode::KeyS, Key::KeyS),
            (KeyCode::KeyD, Key::KeyD),
            (KeyCode::KeyF, Key::KeyF),
            (KeyCode::KeyI, Key::KeyI),
            (KeyCode::KeyL, Key::KeyL),
            (KeyCode::KeyM, Key::KeyM),
            (KeyCode::KeyJ, Key::KeyJ),
            (KeyCode::KeyH, Key::KeyH),
            (KeyCode::KeyP, Key::KeyP),
            (KeyCode::KeyB, Key::KeyB),
            (KeyCode::AltLeft, Key::AltLeft),
            (KeyCode::AltRight, Key::AltRight),
            (KeyCode::ControlLeft, Key::ControlLeft),
            (KeyCode::ControlRight, Key::ControlRight),
            (KeyCode::ShiftLeft, Key::ShiftLeft),
            (KeyCode::ShiftRight, Key::ShiftRight),
            (KeyCode::KeyR, Key::KeyR),
            (KeyCode::KeyK, Key::KeyK),
            (KeyCode::KeyX, Key::KeyX),
            (KeyCode::KeyY, Key::KeyY),
            (KeyCode::KeyU, Key::KeyU),
            (KeyCode::KeyV, Key::KeyV),
            (KeyCode::KeyZ, Key::KeyZ),
            (KeyCode::KeyE, Key::KeyE),
            (KeyCode::KeyC, Key::KeyC),
            (KeyCode::KeyT, Key::KeyT),
            (KeyCode::KeyO, Key::KeyO),
            (KeyCode::KeyN, Key::KeyN),
            (KeyCode::KeyG, Key::KeyG),
            (KeyCode::KeyQ, Key::KeyQ),
            (KeyCode::SuperLeft, Key::MetaLeft),
            (KeyCode::SuperRight, Key::MetaRight),
            (KeyCode::CapsLock, Key::CapsLock),
            (KeyCode::Backquote, Key::Backquote),
            (KeyCode::Backslash, Key::Backslash),
            (KeyCode::Digit1, Key::Digit1),
            (KeyCode::Digit2, Key::Digit2),
            (KeyCode::Digit3, Key::Digit3),
            (KeyCode::Digit4, Key::Digit4),
            (KeyCode::Digit5, Key::Digit5),
            (KeyCode::F1, Key::F1),
            (KeyCode::F2, Key::F2),
            (KeyCode::F3, Key::F3),
            (KeyCode::F4, Key::F4),
            (KeyCode::F5, Key::F5),
            (KeyCode::F11, Key::F11),
            (KeyCode::F12, Key::F12),
        ];
        for (code, key) in cases {
            assert_eq!(map_key(PhysicalKey::Code(code)), key, "{code:?}");
        }
    }

    #[test]
    fn a_key_winit_cannot_identify_or_name_is_unidentified() {
        let unknown = PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified);
        assert_eq!(map_key(unknown), Key::Unidentified);
        let mac = PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::MacOS(0x7f));
        assert_eq!(map_key(mac), Key::Unidentified);
        for legacy in [KeyCode::Meta, KeyCode::Hyper, KeyCode::Turbo] {
            assert_eq!(
                map_key(PhysicalKey::Code(legacy)),
                Key::Unidentified,
                "{legacy:?}"
            );
        }
    }

    /// Every letter key winit defines, and the key it is.
    const LETTER_CODES: [(KeyCode, Key); 26] = [
        (KeyCode::KeyA, Key::KeyA),
        (KeyCode::KeyB, Key::KeyB),
        (KeyCode::KeyC, Key::KeyC),
        (KeyCode::KeyD, Key::KeyD),
        (KeyCode::KeyE, Key::KeyE),
        (KeyCode::KeyF, Key::KeyF),
        (KeyCode::KeyG, Key::KeyG),
        (KeyCode::KeyH, Key::KeyH),
        (KeyCode::KeyI, Key::KeyI),
        (KeyCode::KeyJ, Key::KeyJ),
        (KeyCode::KeyK, Key::KeyK),
        (KeyCode::KeyL, Key::KeyL),
        (KeyCode::KeyM, Key::KeyM),
        (KeyCode::KeyN, Key::KeyN),
        (KeyCode::KeyO, Key::KeyO),
        (KeyCode::KeyP, Key::KeyP),
        (KeyCode::KeyQ, Key::KeyQ),
        (KeyCode::KeyR, Key::KeyR),
        (KeyCode::KeyS, Key::KeyS),
        (KeyCode::KeyT, Key::KeyT),
        (KeyCode::KeyU, Key::KeyU),
        (KeyCode::KeyV, Key::KeyV),
        (KeyCode::KeyW, Key::KeyW),
        (KeyCode::KeyX, Key::KeyX),
        (KeyCode::KeyY, Key::KeyY),
        (KeyCode::KeyZ, Key::KeyZ),
    ];

    /// Every top-row digit key winit defines, and the key it is.
    const DIGIT_CODES: [(KeyCode, Key); 10] = [
        (KeyCode::Digit0, Key::Digit0),
        (KeyCode::Digit1, Key::Digit1),
        (KeyCode::Digit2, Key::Digit2),
        (KeyCode::Digit3, Key::Digit3),
        (KeyCode::Digit4, Key::Digit4),
        (KeyCode::Digit5, Key::Digit5),
        (KeyCode::Digit6, Key::Digit6),
        (KeyCode::Digit7, Key::Digit7),
        (KeyCode::Digit8, Key::Digit8),
        (KeyCode::Digit9, Key::Digit9),
    ];

    /// Every punctuation key winit defines, and the key it is.
    const PUNCTUATION_CODES: [(KeyCode, Key); 14] = [
        (KeyCode::Backquote, Key::Backquote),
        (KeyCode::Backslash, Key::Backslash),
        (KeyCode::BracketLeft, Key::BracketLeft),
        (KeyCode::BracketRight, Key::BracketRight),
        (KeyCode::Comma, Key::Comma),
        (KeyCode::Equal, Key::Equal),
        (KeyCode::IntlBackslash, Key::IntlBackslash),
        (KeyCode::IntlRo, Key::IntlRo),
        (KeyCode::IntlYen, Key::IntlYen),
        (KeyCode::Minus, Key::Minus),
        (KeyCode::Period, Key::Period),
        (KeyCode::Quote, Key::Quote),
        (KeyCode::Semicolon, Key::Semicolon),
        (KeyCode::Slash, Key::Slash),
    ];

    /// Every function key winit defines, F1 to F35, and the key it is.
    const FUNCTION_CODES: [(KeyCode, Key); 35] = [
        (KeyCode::F1, Key::F1),
        (KeyCode::F2, Key::F2),
        (KeyCode::F3, Key::F3),
        (KeyCode::F4, Key::F4),
        (KeyCode::F5, Key::F5),
        (KeyCode::F6, Key::F6),
        (KeyCode::F7, Key::F7),
        (KeyCode::F8, Key::F8),
        (KeyCode::F9, Key::F9),
        (KeyCode::F10, Key::F10),
        (KeyCode::F11, Key::F11),
        (KeyCode::F12, Key::F12),
        (KeyCode::F13, Key::F13),
        (KeyCode::F14, Key::F14),
        (KeyCode::F15, Key::F15),
        (KeyCode::F16, Key::F16),
        (KeyCode::F17, Key::F17),
        (KeyCode::F18, Key::F18),
        (KeyCode::F19, Key::F19),
        (KeyCode::F20, Key::F20),
        (KeyCode::F21, Key::F21),
        (KeyCode::F22, Key::F22),
        (KeyCode::F23, Key::F23),
        (KeyCode::F24, Key::F24),
        (KeyCode::F25, Key::F25),
        (KeyCode::F26, Key::F26),
        (KeyCode::F27, Key::F27),
        (KeyCode::F28, Key::F28),
        (KeyCode::F29, Key::F29),
        (KeyCode::F30, Key::F30),
        (KeyCode::F31, Key::F31),
        (KeyCode::F32, Key::F32),
        (KeyCode::F33, Key::F33),
        (KeyCode::F34, Key::F34),
        (KeyCode::F35, Key::F35),
    ];

    /// Every keypad key winit defines, and the key it is.
    const KEYPAD_CODES: [(KeyCode, Key); 30] = [
        (KeyCode::Numpad0, Key::Numpad0),
        (KeyCode::Numpad1, Key::Numpad1),
        (KeyCode::Numpad2, Key::Numpad2),
        (KeyCode::Numpad3, Key::Numpad3),
        (KeyCode::Numpad4, Key::Numpad4),
        (KeyCode::Numpad5, Key::Numpad5),
        (KeyCode::Numpad6, Key::Numpad6),
        (KeyCode::Numpad7, Key::Numpad7),
        (KeyCode::Numpad8, Key::Numpad8),
        (KeyCode::Numpad9, Key::Numpad9),
        (KeyCode::NumpadAdd, Key::NumpadAdd),
        (KeyCode::NumpadBackspace, Key::NumpadBackspace),
        (KeyCode::NumpadClear, Key::NumpadClear),
        (KeyCode::NumpadClearEntry, Key::NumpadClearEntry),
        (KeyCode::NumpadComma, Key::NumpadComma),
        (KeyCode::NumpadDecimal, Key::NumpadDecimal),
        (KeyCode::NumpadDivide, Key::NumpadDivide),
        (KeyCode::NumpadEnter, Key::NumpadEnter),
        (KeyCode::NumpadEqual, Key::NumpadEqual),
        (KeyCode::NumpadHash, Key::NumpadHash),
        (KeyCode::NumpadMemoryAdd, Key::NumpadMemoryAdd),
        (KeyCode::NumpadMemoryClear, Key::NumpadMemoryClear),
        (KeyCode::NumpadMemoryRecall, Key::NumpadMemoryRecall),
        (KeyCode::NumpadMemoryStore, Key::NumpadMemoryStore),
        (KeyCode::NumpadMemorySubtract, Key::NumpadMemorySubtract),
        (KeyCode::NumpadMultiply, Key::NumpadMultiply),
        (KeyCode::NumpadParenLeft, Key::NumpadParenLeft),
        (KeyCode::NumpadParenRight, Key::NumpadParenRight),
        (KeyCode::NumpadStar, Key::NumpadStar),
        (KeyCode::NumpadSubtract, Key::Minus),
    ];

    /// Every navigation and editing key winit defines, and the key it is.
    const NAVIGATION_CODES: [(KeyCode, Key); 11] = [
        (KeyCode::ArrowDown, Key::ArrowDown),
        (KeyCode::ArrowLeft, Key::ArrowLeft),
        (KeyCode::ArrowRight, Key::ArrowRight),
        (KeyCode::ArrowUp, Key::ArrowUp),
        (KeyCode::End, Key::End),
        (KeyCode::Home, Key::Home),
        (KeyCode::PageDown, Key::PageDown),
        (KeyCode::PageUp, Key::PageUp),
        (KeyCode::Delete, Key::Delete),
        (KeyCode::Insert, Key::Insert),
        (KeyCode::Help, Key::Help),
    ];

    /// Asserts that each of `codes` maps to its key, never to
    /// `Key::Unidentified`.
    fn assert_maps(codes: &[(KeyCode, Key)]) {
        for &(code, key) in codes {
            assert_ne!(key, Key::Unidentified, "{code:?}");
            assert_eq!(map_key(PhysicalKey::Code(code)), key, "{code:?}");
        }
    }

    #[test]
    fn every_letter_key_maps_to_its_key() {
        assert_maps(&LETTER_CODES);
    }

    #[test]
    fn every_digit_key_maps_to_its_key() {
        assert_maps(&DIGIT_CODES);
    }

    #[test]
    fn every_punctuation_key_maps_to_its_key() {
        assert_maps(&PUNCTUATION_CODES);
    }

    #[test]
    fn every_function_key_maps_to_its_key() {
        assert_maps(&FUNCTION_CODES);
    }

    #[test]
    fn every_keypad_key_maps_to_its_key() {
        assert_maps(&KEYPAD_CODES);
    }

    #[test]
    fn every_navigation_and_editing_key_maps_to_its_key() {
        assert_maps(&NAVIGATION_CODES);
    }

    /// The pairs `map_key` folds: the second key arrives as the first's.
    const FOLDS: [(KeyCode, KeyCode); 1] = [(KeyCode::NumpadSubtract, KeyCode::Minus)];

    #[test]
    fn two_keys_are_one_key_only_where_map_key_folds_them() {
        let modifiers = [
            KeyCode::ShiftLeft,
            KeyCode::ShiftRight,
            KeyCode::AltLeft,
            KeyCode::AltRight,
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ];
        let others = [
            KeyCode::Enter,
            KeyCode::Escape,
            KeyCode::Space,
            KeyCode::Tab,
            KeyCode::Backspace,
            KeyCode::CapsLock,
        ];
        let codes: Vec<KeyCode> = [
            &LETTER_CODES[..],
            &DIGIT_CODES,
            &PUNCTUATION_CODES,
            &FUNCTION_CODES,
            &KEYPAD_CODES,
            &NAVIGATION_CODES,
        ]
        .concat()
        .into_iter()
        .map(|(code, _)| code)
        .chain(modifiers)
        .chain(others)
        .collect();
        for (i, &a) in codes.iter().enumerate() {
            for &b in &codes[i + 1..] {
                let folded = FOLDS.contains(&(a, b)) || FOLDS.contains(&(b, a));
                assert_eq!(
                    map_key(PhysicalKey::Code(a)) == map_key(PhysicalKey::Code(b)),
                    folded,
                    "{a:?} and {b:?}"
                );
            }
        }
        for (folded, into) in FOLDS {
            assert_eq!(
                map_key(PhysicalKey::Code(folded)),
                map_key(PhysicalKey::Code(into)),
                "{folded:?}"
            );
        }
    }

    /// Character keys that screens other than flight react to, from the
    /// constants the screens match on where they name one. It is kept by
    /// hand and isn't all of them: flight's come from `every_key`, the
    /// comm, planet comm, escort and offer dialogs' from the tests below,
    /// and a screen's other keys may be missing. A screen test that sends
    /// one of these directly never crosses `map_key`, so this is what keeps
    /// each one reachable from a real keyboard.
    const GAME_CHARACTER_KEYS: [Key; 30] = [
        Key::KeyF,
        Key::KeyI,
        Key::KeyP,
        nova_view::ui::player_info::PLAYER_INFO_KEY,
        crate::app::screen::PREFERENCES_KEY,
        Key::KeyW,
        Key::KeyA,
        Key::KeyS,
        Key::KeyD,
        Key::Equal,
        Key::NumpadAdd,
        Key::Minus,
        nova_view::devtools::TOGGLE_KEY,
        nova_view::spaceport::trade::BUY_KEY,
        nova_view::spaceport::trade::SELL_KEY,
        nova_view::spaceport::outfitter::BUY_KEY,
        nova_view::spaceport::outfitter::SELL_KEY,
        nova_view::spaceport::shipyard::BUY_KEY,
        nova_view::ui::mission_info::MISSION_INFO_KEY,
        nova_view::spaceport::bar::NEWS_KEYS[0],
        nova_view::spaceport::bar::GAMBLE_KEYS[0],
        nova_view::spaceport::bar::GAMBLE_KEYS[1],
        nova_view::spaceport::PORT_KEYS[0].0,
        nova_view::spaceport::PORT_KEYS[1].0,
        nova_view::spaceport::PORT_KEYS[2].0,
        nova_view::spaceport::PORT_KEYS[3].0,
        nova_view::spaceport::PORT_KEYS[4].0,
        nova_view::spaceport::PORT_KEYS[5].0,
        nova_view::spaceport::PORT_KEYS[6].0,
        nova_view::spaceport::PORT_KEYS[7].0,
    ];

    #[test]
    fn the_listed_character_keys_come_from_their_physical_keys() {
        for key in GAME_CHARACTER_KEYS {
            assert_eq!(map_key(PhysicalKey::Code(us_key(key))), key, "{key:?}");
        }
    }

    /// The physical key that sends `key` on a US-layout keyboard.
    fn us_key(key: Key) -> KeyCode {
        match key {
            Key::KeyA => KeyCode::KeyA,
            Key::KeyB => KeyCode::KeyB,
            Key::KeyC => KeyCode::KeyC,
            Key::KeyD => KeyCode::KeyD,
            Key::KeyE => KeyCode::KeyE,
            Key::KeyF => KeyCode::KeyF,
            Key::KeyG => KeyCode::KeyG,
            Key::KeyH => KeyCode::KeyH,
            Key::KeyI => KeyCode::KeyI,
            Key::KeyJ => KeyCode::KeyJ,
            Key::KeyK => KeyCode::KeyK,
            Key::KeyL => KeyCode::KeyL,
            Key::KeyM => KeyCode::KeyM,
            Key::KeyN => KeyCode::KeyN,
            Key::KeyO => KeyCode::KeyO,
            Key::KeyP => KeyCode::KeyP,
            Key::KeyQ => KeyCode::KeyQ,
            Key::KeyR => KeyCode::KeyR,
            Key::KeyS => KeyCode::KeyS,
            Key::KeyT => KeyCode::KeyT,
            Key::KeyU => KeyCode::KeyU,
            Key::KeyV => KeyCode::KeyV,
            Key::KeyW => KeyCode::KeyW,
            Key::KeyX => KeyCode::KeyX,
            Key::KeyY => KeyCode::KeyY,
            Key::KeyZ => KeyCode::KeyZ,
            Key::Digit1 => KeyCode::Digit1,
            Key::Digit2 => KeyCode::Digit2,
            Key::Digit3 => KeyCode::Digit3,
            Key::Digit4 => KeyCode::Digit4,
            Key::Digit5 => KeyCode::Digit5,
            Key::Equal => KeyCode::Equal,
            Key::NumpadAdd => KeyCode::NumpadAdd,
            Key::Minus => KeyCode::Minus,
            Key::Backquote => KeyCode::Backquote,
            Key::Backslash => KeyCode::Backslash,
            Key::Comma => KeyCode::Comma,
            Key::ArrowLeft => KeyCode::ArrowLeft,
            Key::ArrowRight => KeyCode::ArrowRight,
            Key::ArrowUp => KeyCode::ArrowUp,
            Key::ArrowDown => KeyCode::ArrowDown,
            Key::Enter => KeyCode::Enter,
            Key::Space => KeyCode::Space,
            Key::Tab => KeyCode::Tab,
            Key::AltLeft => KeyCode::AltLeft,
            Key::ControlLeft => KeyCode::ControlLeft,
            Key::ShiftLeft => KeyCode::ShiftLeft,
            Key::MetaLeft => KeyCode::SuperLeft,
            Key::F1 => KeyCode::F1,
            Key::F2 => KeyCode::F2,
            Key::F3 => KeyCode::F3,
            Key::F4 => KeyCode::F4,
            Key::F5 => KeyCode::F5,
            Key::F12 => KeyCode::F12,
            other => panic!("no physical key listed for {other:?}"),
        }
    }

    #[test]
    fn every_key_flight_reads_comes_from_its_physical_key() {
        for key in nova_view::flight::view::every_key() {
            assert_eq!(map_key(PhysicalKey::Code(us_key(key))), key, "{key:?}");
        }
    }

    /// Asserts that `letter`'s physical key sends its key. The dialogs take
    /// either case, and one key sends both.
    fn assert_physical(letter: char) {
        let key = nova_view::letter_key(letter).expect("a letter key");
        assert_eq!(map_key(PhysicalKey::Code(us_key(key))), key, "{letter:?}");
    }

    #[test]
    fn every_letter_the_comm_dialog_and_its_haggle_read_comes_from_its_physical_key() {
        use nova_view::ui::comm::{ACCEPT_KEY, CLOSE_KEY, LOWER_KEY};
        let options = nova_sim::HailOptions::nova(&nova_sim::Rulebook::default());
        for letter in options.keys().chain([CLOSE_KEY, ACCEPT_KEY, LOWER_KEY]) {
            assert_physical(letter);
        }
    }

    /// Every character the planet comm and escort dialogs take, with each
    /// button shown: both match on their letters, so asking them is what
    /// keeps this list from being a second one.
    #[test]
    fn every_letter_the_planet_comm_and_escort_dialogs_read_comes_from_its_physical_key() {
        use nova_sim::planet_comm::{PlanetCommButton, PlanetCommView};
        use nova_view::ui::{escort_dialog, planet_comm};
        let views = [
            PlanetCommButton::Greetings,
            PlanetCommButton::OfferBribe,
            PlanetCommButton::DemandTribute,
            PlanetCommButton::Release,
        ]
        .map(|button| PlanetCommView {
            stellar: nova_sim::StellarId(128),
            name: String::new(),
            type_label: String::new(),
            reply: String::new(),
            second: button,
            third: Some(button),
            status: None,
            asking: None,
        });
        for c in ' '..='~' {
            let Some(key) = nova_view::letter_key(c) else {
                continue;
            };
            let planet = views
                .iter()
                .any(|view| planet_comm::key_press(key, view).is_some());
            if planet || escort_dialog::key_press(key).is_some() {
                assert_physical(c);
            }
        }
    }

    /// An offer button's key is its label's first letter, so every
    /// character a label can start with is a key: `offer_labels` keeps a
    /// mission's own label only when it starts with a letter. P, I and M
    /// ask for their screens before an offer reads the labels' letters.
    #[test]
    fn every_letter_an_offer_label_can_start_with_comes_from_its_physical_key() {
        use nova_sim::mission::offer_labels;
        for c in ' '..='~' {
            let label = offer_labels(&c.to_string(), "", false).accept;
            if label.starts_with(c) {
                assert_physical(c);
            }
        }
    }

    /// Every key other than a character's that a screen reacts to, where
    /// the screen names it with a constant, and the physical key that
    /// sends it.
    const GAME_NAMED_KEYS: [(Key, KeyCode); 12] = [
        (nova_view::ui::text_field::DELETE_KEY, KeyCode::Backspace),
        (nova_view::spaceport::shipyard::INFO_KEY, KeyCode::Space),
        (
            nova_view::spaceport::trade::MAX_LOT_KEYS[0],
            KeyCode::AltLeft,
        ),
        (
            nova_view::spaceport::trade::MAX_LOT_KEYS[1],
            KeyCode::AltRight,
        ),
        // The outfitter's grid selection.
        (Key::ArrowLeft, KeyCode::ArrowLeft),
        (Key::ArrowRight, KeyCode::ArrowRight),
        (Key::ArrowUp, KeyCode::ArrowUp),
        (Key::ArrowDown, KeyCode::ArrowDown),
        // Flight's jettison key's modifier, and the menus' Command.
        (Key::MetaLeft, KeyCode::SuperLeft),
        // The menus' Command.
        (Key::MetaRight, KeyCode::SuperRight),
        // The player info's Tab goes back with Shift held.
        (
            nova_view::ui::player_info::BACKWARDS_KEYS[0],
            KeyCode::ShiftLeft,
        ),
        (
            nova_view::ui::player_info::BACKWARDS_KEYS[1],
            KeyCode::ShiftRight,
        ),
    ];

    #[test]
    fn every_named_key_a_screen_reacts_to_comes_from_its_physical_key() {
        for (key, code) in GAME_NAMED_KEYS {
            assert_eq!(map_key(PhysicalKey::Code(code)), key, "{code:?}");
        }
    }

    #[test]
    fn a_key_event_carries_its_key_state_and_repeat_flag() {
        assert_eq!(
            key_event(
                PhysicalKey::Code(KeyCode::ArrowUp),
                ElementState::Pressed,
                false
            ),
            WindowEvent::Key {
                key: Key::ArrowUp,
                pressed: true,
                repeat: false
            }
        );
        assert_eq!(
            key_event(
                PhysicalKey::Code(KeyCode::Escape),
                ElementState::Released,
                false
            ),
            WindowEvent::Key {
                key: Key::Escape,
                pressed: false,
                repeat: false
            }
        );
    }

    #[test]
    fn a_key_held_past_the_repeat_delay_is_a_repeated_press() {
        assert_eq!(
            key_event(PhysicalKey::Code(KeyCode::Tab), ElementState::Pressed, true),
            WindowEvent::Key {
                key: Key::Tab,
                pressed: true,
                repeat: true
            }
        );
    }

    #[test]
    fn a_press_that_types_one_printable_character_is_text() {
        let pressed = ElementState::Pressed;
        assert_eq!(text_event(Some("a"), pressed), Some(WindowEvent::Text('a')));
        assert_eq!(text_event(Some("A"), pressed), Some(WindowEvent::Text('A')));
        assert_eq!(text_event(Some("é"), pressed), Some(WindowEvent::Text('é')));
        assert_eq!(text_event(Some(" "), pressed), Some(WindowEvent::Text(' ')));
        assert_eq!(text_event(Some("/"), pressed), Some(WindowEvent::Text('/')));
    }

    #[test]
    fn no_text_control_characters_several_characters_and_releases_are_not() {
        let pressed = ElementState::Pressed;
        for typed in [
            None,
            Some(""),
            Some("\r"),
            Some("\u{8}"),
            Some("\u{7f}"),
            Some("\u{1b}"),
            Some("\t"),
            Some("ab"),
        ] {
            assert_eq!(text_event(typed, pressed), None, "{typed:?}");
        }
        assert_eq!(text_event(Some("a"), ElementState::Released), None);
    }

    #[test]
    fn mouse_buttons_map_to_the_games() {
        assert_eq!(map_button(WinitButton::Left), MouseButton::Left);
        assert_eq!(map_button(WinitButton::Right), MouseButton::Right);
        assert_eq!(map_button(WinitButton::Middle), MouseButton::Middle);
        assert_eq!(map_button(WinitButton::Back), MouseButton::Other);
        assert_eq!(map_button(WinitButton::Other(7)), MouseButton::Other);
    }

    #[test]
    fn a_resize_carries_the_windows_scale_factor() {
        let event = WinitEvent::Resized(PhysicalSize::new(1280, 960));
        assert_eq!(
            translate(&event, &WINDOWED),
            Some(WindowEvent::Resized {
                size_px: (1280, 960),
                scale_factor: 2.0
            })
        );
    }

    #[test]
    fn pointer_events_are_in_physical_pixels() {
        let moved = WinitEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: PhysicalPosition::new(12.5, 40.0),
        };
        assert_eq!(
            translate(&moved, &WINDOWED),
            Some(WindowEvent::PointerMoved { px: (12.5, 40.0) })
        );
        let left = WinitEvent::CursorLeft {
            device_id: DeviceId::dummy(),
        };
        assert_eq!(translate(&left, &WINDOWED), Some(WindowEvent::PointerLeft));
        let pressed = WinitEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Pressed,
            button: WinitButton::Right,
        };
        assert_eq!(
            translate(&pressed, &WINDOWED),
            Some(WindowEvent::PointerButton {
                button: MouseButton::Right,
                pressed: true
            })
        );
        let released = WinitEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Released,
            button: WinitButton::Left,
        };
        assert_eq!(
            translate(&released, &WINDOWED),
            Some(WindowEvent::PointerButton {
                button: MouseButton::Left,
                pressed: false
            })
        );
    }

    fn wheel(delta: MouseScrollDelta) -> WinitEvent {
        WinitEvent::MouseWheel {
            device_id: DeviceId::dummy(),
            delta,
            phase: TouchPhase::Moved,
        }
    }

    #[test]
    fn a_wheel_is_a_scroll_toward_the_end_for_a_turn_toward_the_user() {
        for (y, lines) in [(1.0, -1.0), (-2.5, 2.5)] {
            assert_eq!(
                translate(&wheel(MouseScrollDelta::LineDelta(0.0, y)), &WINDOWED),
                Some(WindowEvent::Scroll {
                    delta: WheelDelta::Lines(lines)
                }),
                "{y}"
            );
        }
        assert_eq!(
            translate(
                &wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                    0.0, 24.0
                ))),
                &WINDOWED
            ),
            Some(WindowEvent::Scroll {
                delta: WheelDelta::Px(-24.0)
            })
        );
    }

    #[test]
    fn a_scroll_only_across_is_not_translated() {
        assert_eq!(
            translate(&wheel(MouseScrollDelta::LineDelta(3.0, 0.0)), &WINDOWED),
            None
        );
        assert_eq!(
            translate(
                &wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                    5.0, 0.0
                ))),
                &WINDOWED
            ),
            None
        );
    }

    #[test]
    fn closing_is_close_requested() {
        assert_eq!(
            translate(&WinitEvent::CloseRequested, &WINDOWED),
            Some(WindowEvent::CloseRequested)
        );
    }

    #[test]
    fn losing_focus_is_focus_lost_and_gaining_it_focus_gained() {
        assert_eq!(
            translate(&WinitEvent::Focused(false), &WINDOWED),
            Some(WindowEvent::FocusLost)
        );
        assert_eq!(
            translate(&WinitEvent::Focused(true), &WINDOWED),
            Some(WindowEvent::FocusGained)
        );
    }

    #[test]
    fn redraws_and_other_events_are_not_translated() {
        // The runner adds the elapsed time to redraws itself.
        assert_eq!(translate(&WinitEvent::RedrawRequested, &WINDOWED), None);
        assert_eq!(translate(&WinitEvent::Occluded(true), &WINDOWED), None);
    }

    const AT: Duration = Duration::from_millis(250);

    fn events(raw: &WinitEvent, fullscreen: bool) -> Vec<WindowEvent> {
        window_events(raw, &Window { fullscreen }, AT).collect()
    }

    #[test]
    fn a_resize_reports_the_windows_fullscreen_state_before_the_new_size() {
        let raw = WinitEvent::Resized(PhysicalSize::new(3024, 1964));
        for fullscreen in [true, false] {
            assert_eq!(
                events(&raw, fullscreen),
                [
                    WindowEvent::FullscreenChanged { fullscreen },
                    WindowEvent::Resized {
                        size_px: (3024, 1964),
                        scale_factor: 2.0
                    }
                ]
            );
        }
    }

    #[test]
    fn an_occlusion_reports_the_windows_fullscreen_state_alone() {
        for fullscreen in [true, false] {
            for occluded in [true, false] {
                assert_eq!(
                    events(&WinitEvent::Occluded(occluded), fullscreen),
                    [WindowEvent::FullscreenChanged { fullscreen }]
                );
            }
        }
    }

    /// macOS clears the state only once its exit animation ends, after
    /// that animation's resizes; every frame reports it, so the exit is
    /// seen.
    #[test]
    fn every_redraw_reports_the_fullscreen_state_first_then_the_time() {
        for fullscreen in [true, false] {
            assert_eq!(
                events(&WinitEvent::RedrawRequested, fullscreen),
                [
                    WindowEvent::FullscreenChanged { fullscreen },
                    WindowEvent::Redraw { elapsed: AT }
                ]
            );
        }
    }

    #[test]
    fn other_events_report_no_fullscreen_state() {
        let left = WinitEvent::CursorLeft {
            device_id: DeviceId::dummy(),
        };
        assert_eq!(events(&left, true), [WindowEvent::PointerLeft]);
        assert_eq!(
            events(&WinitEvent::Focused(false), true),
            [WindowEvent::FocusLost]
        );
        assert_eq!(
            events(&WinitEvent::Focused(true), true),
            [WindowEvent::FocusGained]
        );
        assert_eq!(
            events(&WinitEvent::CloseRequested, true),
            [WindowEvent::CloseRequested]
        );
        assert_eq!(
            events(&wheel(MouseScrollDelta::LineDelta(0.0, 1.0)), true),
            [WindowEvent::Scroll {
                delta: WheelDelta::Lines(-1.0)
            }]
        );
    }
}
