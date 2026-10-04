//! The winit adapter: translates winit's events into the app's, puts a
//! [`WindowPort`] over a real window and runs the event loop.
//!
//! Translation is pure and unit-tested; the window and the runner only
//! forward to the app. A key press that types a character becomes two
//! events: the key ([`key_event`]), then its text ([`text_event`]).

mod runner;
mod window;

pub use runner::Runner;
pub use window::WinitWindow;

use winit::event::{ElementState, WindowEvent as WinitEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::app::{WindowEvent, WindowPort};
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
        WinitEvent::MouseInput { state, button, .. } => WindowEvent::PointerButton {
            button: map_button(*button),
            pressed: state.is_pressed(),
        },
        WinitEvent::Focused(false) => WindowEvent::FocusLost,
        WinitEvent::CloseRequested => WindowEvent::CloseRequested,
        _ => return None,
    })
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
        PhysicalKey::Code(KeyCode::KeyW) => Key::Char('w'),
        PhysicalKey::Code(KeyCode::KeyA) => Key::Char('a'),
        PhysicalKey::Code(KeyCode::KeyS) => Key::Char('s'),
        PhysicalKey::Code(KeyCode::KeyD) => Key::Char('d'),
        // Flight's key, by position: F on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyF) => Key::Char('f'),
        // The About text's key, by position: I on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyI) => Key::Char('i'),
        // Landing's key, by position: L on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyL) => Key::Char('l'),
        // The flight map's key, by position: M on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyM) => Key::Char('m'),
        // The jump key, by position: J on a US-layout keyboard.
        PhysicalKey::Code(KeyCode::KeyJ) => Key::Char('j'),
        // The Preferences dialog's key, by position: P on a US-layout
        // keyboard.
        PhysicalKey::Code(KeyCode::KeyP) => Key::Char('p'),
        // The Trade Center's buy key, by position: B on a US-layout
        // keyboard. Its sell key is S, above.
        PhysicalKey::Code(KeyCode::KeyB) => Key::Char('b'),
        // Either Alt (Option on a Mac), which makes a trade the most
        // possible while held.
        PhysicalKey::Code(KeyCode::AltLeft | KeyCode::AltRight) => Key::Alt,
        // The developer tools' toggle, by position: the key under Escape on
        // a US-layout keyboard. Without the developer tools, no screen uses
        // it.
        PhysicalKey::Code(KeyCode::Backquote) => Key::Char('`'),
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
    use winit::dpi::{PhysicalPosition, PhysicalSize};
    use winit::event::{DeviceId, MouseButton as WinitButton};

    use super::*;

    struct Window;

    impl WindowPort for Window {
        fn size_px(&self) -> (u32, u32) {
            (800, 600)
        }

        fn scale_factor(&self) -> f64 {
            2.0
        }

        fn request_redraw(&mut self) {}
    }

    #[test]
    fn the_keys_the_game_uses_map_to_its_keys() {
        let cases = [
            (KeyCode::ArrowLeft, Key::Left),
            (KeyCode::ArrowRight, Key::Right),
            (KeyCode::ArrowUp, Key::Up),
            (KeyCode::ArrowDown, Key::Down),
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
            (KeyCode::KeyP, Key::Char('p')),
            (KeyCode::KeyB, Key::Char('b')),
            (KeyCode::AltLeft, Key::Alt),
            (KeyCode::AltRight, Key::Alt),
            (KeyCode::Backquote, Key::Char('`')),
            (KeyCode::KeyQ, Key::Other),
            (KeyCode::F1, Key::Other),
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
    const GAME_CHARACTER_KEYS: [Key; 20] = [
        Key::Char('f'),
        Key::Char('i'),
        Key::Char('p'),
        nova_view::flight::view::LAND_KEY,
        nova_view::flight::view::MAP_KEY,
        nova_view::flight::view::JUMP_KEY,
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
        nova_view::spaceport::shipyard::INFO_KEY,
    ];

    /// The physical key that types `c` on a US-layout keyboard.
    fn us_position(c: char) -> KeyCode {
        match c {
            'a' => KeyCode::KeyA,
            'b' => KeyCode::KeyB,
            'd' => KeyCode::KeyD,
            'f' => KeyCode::KeyF,
            'i' => KeyCode::KeyI,
            'j' => KeyCode::KeyJ,
            'l' => KeyCode::KeyL,
            'm' => KeyCode::KeyM,
            'p' => KeyCode::KeyP,
            's' => KeyCode::KeyS,
            'w' => KeyCode::KeyW,
            '=' => KeyCode::Equal,
            '+' => KeyCode::NumpadAdd,
            '-' => KeyCode::Minus,
            '`' => KeyCode::Backquote,
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
    const GAME_NAMED_KEYS: [(Key, KeyCode); 7] = [
        (nova_view::ui::text_field::DELETE_KEY, KeyCode::Backspace),
        (nova_view::spaceport::trade::MAX_LOT_KEY, KeyCode::AltLeft),
        (nova_view::spaceport::trade::MAX_LOT_KEY, KeyCode::AltRight),
        // The outfitter's grid selection.
        (Key::Left, KeyCode::ArrowLeft),
        (Key::Right, KeyCode::ArrowRight),
        (Key::Up, KeyCode::ArrowUp),
        (Key::Down, KeyCode::ArrowDown),
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
            translate(&event, &Window),
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
            translate(&moved, &Window),
            Some(WindowEvent::PointerMoved { px: (12.5, 40.0) })
        );
        let pressed = WinitEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Pressed,
            button: WinitButton::Right,
        };
        assert_eq!(
            translate(&pressed, &Window),
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
            translate(&released, &Window),
            Some(WindowEvent::PointerButton {
                button: MouseButton::Left,
                pressed: false
            })
        );
    }

    #[test]
    fn closing_is_close_requested() {
        assert_eq!(
            translate(&WinitEvent::CloseRequested, &Window),
            Some(WindowEvent::CloseRequested)
        );
    }

    #[test]
    fn losing_focus_is_focus_lost_and_gaining_it_is_nothing() {
        assert_eq!(
            translate(&WinitEvent::Focused(false), &Window),
            Some(WindowEvent::FocusLost)
        );
        assert_eq!(translate(&WinitEvent::Focused(true), &Window), None);
    }

    #[test]
    fn redraws_and_other_events_are_not_translated() {
        // The runner adds the elapsed time to redraws itself.
        assert_eq!(translate(&WinitEvent::RedrawRequested, &Window), None);
        assert_eq!(translate(&WinitEvent::Focused(true), &Window), None);
    }
}
