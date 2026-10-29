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
#[cfg(target_os = "macos")]
mod menu_bar;
mod runner;
#[cfg(any(target_os = "macos", windows))]
mod updater;
mod window;

pub use dialogs::Dialogs;
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

/// The game's key for a physical key; keys it does not use are
/// [`Key::Other`].
#[must_use]
pub fn map_key(key: PhysicalKey) -> Key {
    match key {
        PhysicalKey::Code(KeyCode::ArrowLeft) => Key::Left,
        PhysicalKey::Code(KeyCode::ArrowRight) => Key::Right,
        PhysicalKey::Code(KeyCode::ArrowUp) => Key::Up,
        PhysicalKey::Code(KeyCode::ArrowDown) => Key::Down,
        // The shop grids' Page Up and Page Down, Mac key codes 0x74 and
        // 0x79, fn+Up and fn+Down on a Mac laptop.
        PhysicalKey::Code(KeyCode::PageUp) => Key::PageUp,
        PhysicalKey::Code(KeyCode::PageDown) => Key::PageDown,
        PhysicalKey::Code(KeyCode::Enter | KeyCode::NumpadEnter) => Key::Enter,
        PhysicalKey::Code(KeyCode::Escape) => Key::Escape,
        PhysicalKey::Code(KeyCode::Space) => Key::Space,
        PhysicalKey::Code(KeyCode::Tab) => Key::Tab,
        // Deletes the character before the caret in a text field: Delete on
        // a Mac keyboard.
        PhysicalKey::Code(KeyCode::Backspace) => Key::Backspace,
        // The map's zoom keys, by position: `=` is the unshifted `+` key on
        // US-layout keyboards.
        PhysicalKey::Code(KeyCode::Equal) => Key::Char('='),
        PhysicalKey::Code(KeyCode::NumpadAdd) => Key::Char('+'),
        PhysicalKey::Code(KeyCode::Minus | KeyCode::NumpadSubtract) => Key::Char('-'),
        // The system view's movement keys, by position: W, A, S and D on a
        // US-layout keyboard, wherever another layout puts those letters.
        // S is also the spaceport's Shipyard key.
        PhysicalKey::Code(KeyCode::KeyW) => Key::Char('w'),
        PhysicalKey::Code(KeyCode::KeyA) => Key::Char('a'),
        PhysicalKey::Code(KeyCode::KeyS) => Key::Char('s'),
        PhysicalKey::Code(KeyCode::KeyD) => Key::Char('d'),
        // E, by position: flight's escort menu key, the comm dialog's
        // close key, and the main menu's E, which waits for this key's
        // release as `_QuickKeyCheck(0x0e)` does.
        PhysicalKey::Code(KeyCode::KeyE) => Key::Char('e'),
        // Flight's key and the spaceport's other Recharge key, by
        // position: F on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyF) => Key::Char('f'),
        // The About text's key, by position: I on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyI) => Key::Char('i'),
        // Landing's key, by position: L on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyL) => Key::Char('l'),
        // The flight map's key, by position: M on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyM) => Key::Char('m'),
        // The jump key, by position: J on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyJ) => Key::Char('j'),
        // Flight's Hyperspace Mode key and the bar's hire key, by position:
        // H on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyH) => Key::Char('h'),
        // Hyper Select, by position: `\` on a US-layout keyboard; the
        // original's default.
        PhysicalKey::Code(KeyCode::Backslash) => Key::Char('\\'),
        // The player info key in flight and the spaceport, and the
        // Preferences dialog's elsewhere, by position: P on a US-layout
        // keyboard.
        PhysicalKey::Code(KeyCode::KeyP) => Key::Char('p'),
        // Command+comma opens the Preferences, by position: the comma on a
        // US-layout keyboard.
        PhysicalKey::Code(KeyCode::Comma) => Key::Char(','),
        // The Trade Center's buy key and the spaceport's Bar key, by
        // position: B on a US-layout keyboard. Its sell key is S, above.
        PhysicalKey::Code(KeyCode::KeyB) => Key::Char('b'),
        // Flight's nearest target key and the spaceport's Recharge key,
        // by position: R on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyR) => Key::Char('r'),
        // The spaceport's letter keys, by position, as `_PortFilter` reads
        // the Mac key code: C (also flight's escort recall key) and T the
        // Trade Center, O the Outfitter, and N the Mission BBS (also the
        // bar's news key and an offer's refuse key).
        PhysicalKey::Code(KeyCode::KeyC) => Key::Char('c'),
        PhysicalKey::Code(KeyCode::KeyT) => Key::Char('t'),
        PhysicalKey::Code(KeyCode::KeyO) => Key::Char('o'),
        PhysicalKey::Code(KeyCode::KeyN) => Key::Char('n'),
        // Either Alt (Option on a Mac), which makes a trade the most
        // possible, and flight's weapon select go back, while held.
        PhysicalKey::Code(KeyCode::AltLeft | KeyCode::AltRight) => Key::Alt,
        // Either Shift, which turns flight's target select key back while
        // held, and the player info's Tab to the previous tab.
        PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => Key::Shift,
        // Either Control: flight's secondary fire key.
        PhysicalKey::Code(KeyCode::ControlLeft | KeyCode::ControlRight) => Key::Control,
        // Either Command (Super, the Windows key, off a Mac): what flight's
        // jettison and eject keys need held.
        PhysicalKey::Code(KeyCode::SuperLeft | KeyCode::SuperRight) => Key::Command,
        // Flight's jettison key, by position: K on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyK) => Key::Char('k'),
        // Flight's eject key, by position: X on a US-layout keyboard, the
        // original's default (keycode 0x07).
        PhysicalKey::Code(KeyCode::KeyX) => Key::Char('x'),
        // Double time's key: each press toggles it, an accepted difference
        // from the original's following the lock state. winit 0.30.13
        // sends no Caps Lock key on macOS (its `key_to_modifier` maps Alt,
        // Control, Super and Shift alone), so there it never arrives.
        PhysicalKey::Code(KeyCode::CapsLock) => Key::CapsLock,
        // The developer tools' toggle, by position: the key under Escape on
        // a US-layout keyboard. Without the developer tools, it reaches
        // flight as Nav Off, the original's default.
        PhysicalKey::Code(KeyCode::Backquote) => Key::Char('`'),
        // Flight's stellar slot keys, by position: 1-4 on the top row (the
        // keypad's are not the original's).
        PhysicalKey::Code(KeyCode::Digit1) => Key::Char('1'),
        PhysicalKey::Code(KeyCode::Digit2) => Key::Char('2'),
        PhysicalKey::Code(KeyCode::Digit3) => Key::Char('3'),
        PhysicalKey::Code(KeyCode::Digit4) => Key::Char('4'),
        // The top-row 5: the escort menu's Freighters key, and with
        // Command flight's second nearest-stellar key. 6-0 stay unmapped.
        PhysicalKey::Code(KeyCode::Digit5) => Key::Char('5'),
        // F1-F4, flight's other stellar slot keys, and F5, its nearest
        // stellar key.
        PhysicalKey::Code(KeyCode::F1) => Key::Function(1),
        PhysicalKey::Code(KeyCode::F2) => Key::Function(2),
        PhysicalKey::Code(KeyCode::F3) => Key::Function(3),
        PhysicalKey::Code(KeyCode::F4) => Key::Function(4),
        PhysicalKey::Code(KeyCode::F5) => Key::Function(5),
        // The fullscreen shortcut off a Mac.
        PhysicalKey::Code(KeyCode::F11) => Key::Function(11),
        _ => Key::Other,
    }
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
            (KeyCode::ArrowLeft, Key::Left),
            (KeyCode::ArrowRight, Key::Right),
            (KeyCode::ArrowUp, Key::Up),
            (KeyCode::ArrowDown, Key::Down),
            (KeyCode::PageUp, Key::PageUp),
            (KeyCode::PageDown, Key::PageDown),
            (KeyCode::Enter, Key::Enter),
            (KeyCode::NumpadEnter, Key::Enter),
            (KeyCode::Escape, Key::Escape),
            (KeyCode::Space, Key::Space),
            (KeyCode::Tab, Key::Tab),
            (KeyCode::Backspace, Key::Backspace),
            (KeyCode::Equal, Key::Char('=')),
            (KeyCode::NumpadAdd, Key::Char('+')),
            (KeyCode::Minus, Key::Char('-')),
            (KeyCode::NumpadSubtract, Key::Char('-')),
            (KeyCode::KeyW, Key::Char('w')),
            (KeyCode::KeyA, Key::Char('a')),
            (KeyCode::KeyS, Key::Char('s')),
            (KeyCode::KeyD, Key::Char('d')),
            (KeyCode::KeyF, Key::Char('f')),
            (KeyCode::KeyI, Key::Char('i')),
            (KeyCode::KeyL, Key::Char('l')),
            (KeyCode::KeyM, Key::Char('m')),
            (KeyCode::KeyJ, Key::Char('j')),
            (KeyCode::KeyH, Key::Char('h')),
            (KeyCode::KeyP, Key::Char('p')),
            (KeyCode::KeyB, Key::Char('b')),
            (KeyCode::AltLeft, Key::Alt),
            (KeyCode::AltRight, Key::Alt),
            (KeyCode::ControlLeft, Key::Control),
            (KeyCode::ControlRight, Key::Control),
            (KeyCode::ShiftLeft, Key::Shift),
            (KeyCode::ShiftRight, Key::Shift),
            (KeyCode::KeyR, Key::Char('r')),
            (KeyCode::KeyK, Key::Char('k')),
            (KeyCode::KeyX, Key::Char('x')),
            (KeyCode::KeyE, Key::Char('e')),
            (KeyCode::KeyC, Key::Char('c')),
            (KeyCode::KeyT, Key::Char('t')),
            (KeyCode::KeyO, Key::Char('o')),
            (KeyCode::KeyN, Key::Char('n')),
            (KeyCode::SuperLeft, Key::Command),
            (KeyCode::SuperRight, Key::Command),
            (KeyCode::CapsLock, Key::CapsLock),
            (KeyCode::Backquote, Key::Char('`')),
            (KeyCode::Backslash, Key::Char('\\')),
            (KeyCode::Digit1, Key::Char('1')),
            (KeyCode::Digit2, Key::Char('2')),
            (KeyCode::Digit3, Key::Char('3')),
            (KeyCode::Digit4, Key::Char('4')),
            (KeyCode::Digit5, Key::Char('5')),
            (KeyCode::F1, Key::Function(1)),
            (KeyCode::F2, Key::Function(2)),
            (KeyCode::F3, Key::Function(3)),
            (KeyCode::F4, Key::Function(4)),
            (KeyCode::F5, Key::Function(5)),
            (KeyCode::F11, Key::Function(11)),
            (KeyCode::KeyQ, Key::Other),
            (KeyCode::Digit6, Key::Other),
            (KeyCode::F6, Key::Other),
        ];
        for (code, key) in cases {
            assert_eq!(map_key(PhysicalKey::Code(code)), key, "{code:?}");
        }
        let unknown = PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified);
        assert_eq!(map_key(unknown), Key::Other);
    }

    /// Every character key some screen reacts to, from the constants the
    /// screens match on where they name one. A screen test that sends one of
    /// these directly never crosses `map_key`, so this is what keeps each
    /// one reachable from a real keyboard.
    const GAME_CHARACTER_KEYS: [Key; 46] = [
        Key::Char('f'),
        Key::Char('i'),
        Key::Char('p'),
        nova_view::ui::player_info::PLAYER_INFO_KEY,
        crate::app::screen::PREFERENCES_KEY,
        nova_view::flight::view::LAND_KEY,
        nova_view::flight::view::MAP_KEY,
        nova_view::flight::view::JUMP_KEY,
        nova_view::flight::view::SELECT_KEY,
        nova_view::flight::view::DESELECT_KEY,
        nova_view::flight::view::NEAREST_KEY,
        nova_view::flight::view::HYPER_SELECT_KEY,
        nova_view::flight::view::HYPERSPACE_MODE_KEY,
        nova_view::flight::view::NAV_OFF_KEY,
        nova_view::flight::view::JETTISON_KEY,
        nova_view::flight::view::EJECT_KEY,
        nova_view::flight::view::STELLAR_SLOT_KEYS[0][0],
        nova_view::flight::view::STELLAR_SLOT_KEYS[1][0],
        nova_view::flight::view::STELLAR_SLOT_KEYS[2][0],
        nova_view::flight::view::STELLAR_SLOT_KEYS[3][0],
        Key::Char('w'),
        Key::Char('a'),
        Key::Char('s'),
        Key::Char('d'),
        Key::Char('='),
        Key::Char('+'),
        Key::Char('-'),
        nova_view::devtools::TOGGLE_KEY,
        nova_view::spaceport::trade::BUY_KEY,
        nova_view::spaceport::trade::SELL_KEY,
        nova_view::spaceport::outfitter::BUY_KEY,
        nova_view::spaceport::outfitter::SELL_KEY,
        nova_view::spaceport::shipyard::BUY_KEY,
        nova_view::ui::mission_info::MISSION_INFO_KEY,
        nova_view::flight::escorts::MENU_KEY,
        nova_view::flight::escorts::GROUP_KEYS[4],
        nova_view::flight::view::ESCORT_RECALL_KEY,
        nova_view::spaceport::bar::NEWS_KEYS[0],
        nova_view::spaceport::PORT_KEYS[0].0,
        nova_view::spaceport::PORT_KEYS[1].0,
        nova_view::spaceport::PORT_KEYS[2].0,
        nova_view::spaceport::PORT_KEYS[3].0,
        nova_view::spaceport::PORT_KEYS[4].0,
        nova_view::spaceport::PORT_KEYS[5].0,
        nova_view::spaceport::PORT_KEYS[6].0,
        nova_view::spaceport::PORT_KEYS[7].0,
    ];

    /// The physical key that types `c` on a US-layout keyboard.
    fn us_position(c: char) -> KeyCode {
        match c {
            'a' => KeyCode::KeyA,
            'b' => KeyCode::KeyB,
            'c' => KeyCode::KeyC,
            'd' => KeyCode::KeyD,
            'e' => KeyCode::KeyE,
            'f' => KeyCode::KeyF,
            'h' => KeyCode::KeyH,
            'i' => KeyCode::KeyI,
            'j' => KeyCode::KeyJ,
            'k' => KeyCode::KeyK,
            'l' => KeyCode::KeyL,
            'm' => KeyCode::KeyM,
            'n' => KeyCode::KeyN,
            'o' => KeyCode::KeyO,
            'p' => KeyCode::KeyP,
            'r' => KeyCode::KeyR,
            's' => KeyCode::KeyS,
            't' => KeyCode::KeyT,
            'w' => KeyCode::KeyW,
            'x' => KeyCode::KeyX,
            '1' => KeyCode::Digit1,
            '2' => KeyCode::Digit2,
            '3' => KeyCode::Digit3,
            '4' => KeyCode::Digit4,
            '5' => KeyCode::Digit5,
            '=' => KeyCode::Equal,
            '+' => KeyCode::NumpadAdd,
            '-' => KeyCode::Minus,
            '`' => KeyCode::Backquote,
            '\\' => KeyCode::Backslash,
            ',' => KeyCode::Comma,
            other => panic!("no physical key listed for {other:?}"),
        }
    }

    #[test]
    fn every_character_key_a_screen_reacts_to_comes_from_its_physical_key() {
        for key in GAME_CHARACTER_KEYS {
            let Key::Char(c) = key else {
                panic!("{key:?} is not a character key");
            };
            assert_eq!(map_key(PhysicalKey::Code(us_position(c))), key, "{c:?}");
        }
    }

    /// Every key other than a character's that a screen reacts to, where
    /// the screen names it with a constant, and the physical key that
    /// sends it.
    const GAME_NAMED_KEYS: [(Key, KeyCode); 17] = [
        (nova_view::ui::text_field::DELETE_KEY, KeyCode::Backspace),
        (nova_view::spaceport::shipyard::INFO_KEY, KeyCode::Space),
        (nova_view::spaceport::trade::MAX_LOT_KEY, KeyCode::AltLeft),
        (nova_view::spaceport::trade::MAX_LOT_KEY, KeyCode::AltRight),
        // The outfitter's grid selection.
        (Key::Left, KeyCode::ArrowLeft),
        (Key::Right, KeyCode::ArrowRight),
        (Key::Up, KeyCode::ArrowUp),
        (Key::Down, KeyCode::ArrowDown),
        // Flight's stellar slot and nearest stellar keys.
        (
            nova_view::flight::view::STELLAR_SLOT_KEYS[0][1],
            KeyCode::F1,
        ),
        (
            nova_view::flight::view::STELLAR_SLOT_KEYS[1][1],
            KeyCode::F2,
        ),
        (
            nova_view::flight::view::STELLAR_SLOT_KEYS[2][1],
            KeyCode::F3,
        ),
        (
            nova_view::flight::view::STELLAR_SLOT_KEYS[3][1],
            KeyCode::F4,
        ),
        (nova_view::flight::view::NEAREST_STELLAR_KEY, KeyCode::F5),
        // Flight's jettison key's modifier.
        (Key::Command, KeyCode::SuperLeft),
        (Key::Command, KeyCode::SuperRight),
        // The player info's Tab goes back with Shift held.
        (
            nova_view::ui::player_info::BACKWARDS_KEY,
            KeyCode::ShiftLeft,
        ),
        (
            nova_view::ui::player_info::BACKWARDS_KEY,
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
                key: Key::Up,
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
