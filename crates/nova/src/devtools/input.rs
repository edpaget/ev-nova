//! The overlay's input: the game's own [`WindowEvent`]s, already
//! translated from the platform's, as egui's [`RawInput`].
//!
//! The overlay takes the events the game does, so it names no winit type,
//! and a winit bump never waits on egui. [`EguiInput`] follows
//! egui-winit 0.36.2's mapping, the glue it replaces:
//!
//! - Points are pixels over the scale factor. A button goes down and up at
//!   the pointer's last point, and with none known (before any move, or
//!   after it leaves) there is no event.
//! - Each key is its egui key by its physical position ([`egui_key`]),
//!   with the modifiers held. Command is Meta on a Mac and Control
//!   elsewhere. Command+X and +C are a cut and a copy, and +V nothing, as
//!   there is no clipboard.
//! - Typed text is egui's text, except while Control or Command is held.
//! - Losing focus lets go of every modifier, as their releases won't come.
//! - A finger is the pointer as egui-winit's `on_touch` emulates it: the
//!   first finger down while none is followed moves the pointer to its
//!   point and presses the primary button, its moves move it, its lift
//!   releases the button where it last was and the pointer goes, and its
//!   cancel makes the pointer go with no release. Other fingers, and the
//!   leave winit sends after a lift, give nothing; egui's own `Touch`
//!   events, which no panel reads, aren't sent.
//!
//! What the overlay no longer gets, none of it input the panels need:
//!
//! - Keys are physical, so on a layout other than QWERTY egui's shortcuts
//!   (Command+A, +Z, +X, +C) follow the key's position. Typed text comes
//!   from [`WindowEvent::Text`], so it is unaffected.
//! - Only the wheel's vertical axis is kept, as the game's
//!   [`WindowEvent::Scroll`] keeps it.
//! - The Back and Forward buttons are the game's `MouseButton::Other`, so
//!   they reach egui as nothing.
//! - There is no IME composition, and Windows' Shift+Delete and
//!   Ctrl+Insert are not a cut and a copy.
//! - The pointer stays an arrow: egui's cursor icon goes unused.

use std::time::Duration;

use egui::{
    Event, Key as EguiKey, Modifiers, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect,
    TouchPhase, ViewportId, vec2,
};
use nova_render::wgpu::MAX_TEXTURE_SIDE;
use nova_view::{FingerId, Key, MouseButton};

use crate::app::{WheelDelta, WindowEvent};

/// The modifier keys, left and right of each: Shift, Control, Alt and
/// Meta.
const MODIFIER_KEYS: [Key; 8] = [
    Key::ShiftLeft,
    Key::ShiftRight,
    Key::ControlLeft,
    Key::ControlRight,
    Key::AltLeft,
    Key::AltRight,
    Key::MetaLeft,
    Key::MetaRight,
];

/// The game's window events, queued as egui's input for the overlay's next
/// frame.
#[derive(Debug)]
pub struct EguiInput {
    /// Physical pixels per point.
    scale: f64,
    /// Whether Command is Meta (a Mac) or Control.
    mac: bool,
    /// Where the pointer last was, in points; none before it moves in, or
    /// once it has left.
    pointer: Option<Pos2>,
    /// The finger that is egui's pointer, while it is down.
    finger: Option<FingerId>,
    /// Which of [`MODIFIER_KEYS`] are held.
    held: [bool; 8],
    /// Whether the window takes keyboard input.
    focused: bool,
    /// The events since the last frame.
    events: Vec<Event>,
}

impl EguiInput {
    /// Input for a window at `scale_factor` physical pixels a point, on a
    /// Mac (`mac`, where Command is Meta) or not.
    #[must_use]
    pub fn new(scale_factor: f64, mac: bool) -> Self {
        Self {
            scale: scale_factor,
            mac,
            pointer: None,
            finger: None,
            held: [false; 8],
            focused: true,
            events: Vec::new(),
        }
    }

    /// Queues what `event` gives egui, if anything.
    pub fn on_event(&mut self, event: &WindowEvent) {
        match *event {
            WindowEvent::PointerMoved { px } => {
                let at = self.point(px);
                self.pointer = Some(at);
                self.events.push(Event::PointerMoved(at));
            }
            WindowEvent::PointerLeft => {
                self.pointer = None;
                self.events.push(Event::PointerGone);
            }
            WindowEvent::PointerButton { button, pressed } => {
                if let (Some(pos), Some(button)) = (self.pointer, pointer_button(button)) {
                    self.events.push(Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        modifiers: self.modifiers(),
                    });
                }
            }
            WindowEvent::Scroll { delta } => {
                // egui's positive scrolls toward the top, the game's
                // toward the text's end.
                let (unit, toward_top) = match delta {
                    WheelDelta::Lines(lines) => (MouseWheelUnit::Line, -lines),
                    WheelDelta::Px(px) => (MouseWheelUnit::Point, (-px / self.scale) as f32),
                };
                self.events.push(Event::MouseWheel {
                    unit,
                    delta: vec2(0.0, toward_top),
                    phase: TouchPhase::Move,
                    modifiers: self.modifiers(),
                });
            }
            WindowEvent::Key {
                key,
                pressed,
                repeat,
            } => self.key(key, pressed, repeat),
            WindowEvent::Text(c) => {
                let held = self.modifiers();
                // A command's character (Command-C's "c") is not typed.
                // (egui-winit also tests `mac_cmd`, which is held only
                // with `command`.)
                if !(held.ctrl || held.command) {
                    self.events.push(Event::Text(c.to_string()));
                }
            }
            WindowEvent::FocusLost => {
                self.held = [false; 8];
                self.events.push(Event::ModifiersChanged(self.modifiers()));
                self.events.push(Event::WindowFocused(false));
                self.focused = false;
            }
            WindowEvent::FocusGained => {
                self.events.push(Event::WindowFocused(true));
                self.focused = true;
            }
            WindowEvent::Resized { scale_factor, .. } => self.scale = scale_factor,
            WindowEvent::TouchStarted { finger, px } => {
                if self.finger.is_none() {
                    self.finger = Some(finger);
                    let at = self.point(px);
                    self.pointer = Some(at);
                    self.events.push(Event::PointerMoved(at));
                    self.events.push(Event::PointerButton {
                        pos: at,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: self.modifiers(),
                    });
                }
            }
            WindowEvent::TouchMoved { finger, px } => {
                if self.finger == Some(finger) {
                    let at = self.point(px);
                    self.pointer = Some(at);
                    self.events.push(Event::PointerMoved(at));
                }
            }
            WindowEvent::TouchEnded { finger, .. } => {
                if self.finger == Some(finger) {
                    if let Some(pos) = self.pointer {
                        self.events.push(Event::PointerButton {
                            pos,
                            button: PointerButton::Primary,
                            pressed: false,
                            modifiers: self.modifiers(),
                        });
                    }
                    self.finger_gone();
                }
            }
            WindowEvent::TouchCancelled { finger } => {
                if self.finger == Some(finger) {
                    self.finger_gone();
                }
            }
            WindowEvent::Redraw { .. }
            | WindowEvent::FullscreenChanged { .. }
            | WindowEvent::CloseRequested
            | WindowEvent::Menu(_)
            | WindowEvent::CheckForUpdates => {}
        }
    }

    /// egui's input for the next frame of a window `size_px` in physical
    /// pixels at `scale_factor`, `elapsed` since the app started: the
    /// screen in points (none for a window of no size, as a minimised one
    /// is on Windows) and every event queued since the last.
    pub fn take(&mut self, size_px: (u32, u32), scale_factor: f64, elapsed: Duration) -> RawInput {
        self.scale = scale_factor;
        let (width, height) = size_px;
        let points = vec2(width as f32, height as f32) / scale_factor as f32;
        // The root viewport, `RawInput`'s default, is the only one.
        let mut input = RawInput {
            screen_rect: (points.x > 0.0 && points.y > 0.0)
                .then(|| Rect::from_min_size(Pos2::ZERO, points)),
            max_texture_side: Some(MAX_TEXTURE_SIDE as usize),
            time: Some(elapsed.as_secs_f64()),
            events: std::mem::take(&mut self.events),
            focused: self.focused,
            ..RawInput::default()
        };
        input
            .viewports
            .entry(ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(scale_factor as f32);
        input
    }

    /// The pointer's finger lifted or was cancelled: the pointer vanishes,
    /// so nothing stays hovered, as egui-winit has it.
    fn finger_gone(&mut self) {
        self.finger = None;
        self.pointer = None;
        self.events.push(Event::PointerGone);
    }

    /// Window pixel `px` in points.
    fn point(&self, (x, y): (f64, f64)) -> Pos2 {
        Pos2::new((x / self.scale) as f32, (y / self.scale) as f32)
    }

    /// The modifiers held, as egui has them.
    fn modifiers(&self) -> Modifiers {
        let either = |left: usize| self.held[left] || self.held[left + 1];
        let (shift, ctrl, alt, meta) = (either(0), either(2), either(4), either(6));
        Modifiers {
            alt,
            ctrl,
            shift,
            mac_cmd: self.mac && meta,
            command: if self.mac { meta } else { ctrl },
        }
    }

    /// `key` going down or up: a modifier's change first, then the key, or
    /// the cut or copy it commands.
    fn key(&mut self, key: Key, pressed: bool, repeat: bool) {
        if let Some(index) = MODIFIER_KEYS.iter().position(|&modifier| modifier == key) {
            let before = self.modifiers();
            self.held[index] = pressed;
            let after = self.modifiers();
            if after != before {
                self.events.push(Event::ModifiersChanged(after));
            }
        }
        let Some(egui) = egui_key(key) else {
            return;
        };
        let modifiers = self.modifiers();
        if pressed {
            if egui == EguiKey::Cut || (modifiers.command && egui == EguiKey::X) {
                self.events.push(Event::Cut);
                return;
            }
            if egui == EguiKey::Copy || (modifiers.command && egui == EguiKey::C) {
                self.events.push(Event::Copy);
                return;
            }
            // There is no clipboard to paste from.
            if egui == EguiKey::Paste || (modifiers.command && egui == EguiKey::V) {
                return;
            }
        }
        self.events.push(Event::Key {
            key: egui,
            physical_key: Some(egui),
            pressed,
            repeat,
            modifiers,
        });
    }
}

/// egui's button for the game's, or `None` for the game's `Other`: Back,
/// Forward and the rest, which egui-winit gave egui as its extra buttons,
/// arrive merged and are dropped.
fn pointer_button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Primary),
        MouseButton::Right => Some(PointerButton::Secondary),
        MouseButton::Middle => Some(PointerButton::Middle),
        MouseButton::Other => None,
    }
}

/// egui's key for the game's: the rows of egui-winit 0.36.2's
/// `key_from_key_code`, over the same names (the game's [`Key`] is
/// keyboard-types' `Code`, as winit's `KeyCode` is). `None` for every
/// other key.
#[must_use]
#[allow(clippy::too_many_lines)] // one row for each key egui-winit names
pub fn egui_key(key: Key) -> Option<EguiKey> {
    Some(match key {
        Key::ArrowDown => EguiKey::ArrowDown,
        Key::ArrowLeft => EguiKey::ArrowLeft,
        Key::ArrowRight => EguiKey::ArrowRight,
        Key::ArrowUp => EguiKey::ArrowUp,
        Key::Escape => EguiKey::Escape,
        Key::Tab => EguiKey::Tab,
        Key::Backspace => EguiKey::Backspace,
        Key::Enter | Key::NumpadEnter => EguiKey::Enter,
        Key::Insert => EguiKey::Insert,
        Key::Delete => EguiKey::Delete,
        Key::Home => EguiKey::Home,
        Key::End => EguiKey::End,
        Key::PageUp => EguiKey::PageUp,
        Key::PageDown => EguiKey::PageDown,
        Key::Space => EguiKey::Space,
        Key::Comma => EguiKey::Comma,
        Key::Period => EguiKey::Period,
        Key::Semicolon => EguiKey::Semicolon,
        Key::Backslash => EguiKey::Backslash,
        Key::Slash | Key::NumpadDivide => EguiKey::Slash,
        Key::BracketLeft => EguiKey::OpenBracket,
        Key::BracketRight => EguiKey::CloseBracket,
        Key::Backquote => EguiKey::Backtick,
        Key::Quote => EguiKey::Quote,
        Key::Cut => EguiKey::Cut,
        Key::Copy => EguiKey::Copy,
        Key::Paste => EguiKey::Paste,
        Key::Minus | Key::NumpadSubtract => EguiKey::Minus,
        Key::NumpadAdd => EguiKey::Plus,
        Key::Equal => EguiKey::Equals,
        Key::Digit0 | Key::Numpad0 => EguiKey::Num0,
        Key::Digit1 | Key::Numpad1 => EguiKey::Num1,
        Key::Digit2 | Key::Numpad2 => EguiKey::Num2,
        Key::Digit3 | Key::Numpad3 => EguiKey::Num3,
        Key::Digit4 | Key::Numpad4 => EguiKey::Num4,
        Key::Digit5 | Key::Numpad5 => EguiKey::Num5,
        Key::Digit6 | Key::Numpad6 => EguiKey::Num6,
        Key::Digit7 | Key::Numpad7 => EguiKey::Num7,
        Key::Digit8 | Key::Numpad8 => EguiKey::Num8,
        Key::Digit9 | Key::Numpad9 => EguiKey::Num9,
        Key::KeyA => EguiKey::A,
        Key::KeyB => EguiKey::B,
        Key::KeyC => EguiKey::C,
        Key::KeyD => EguiKey::D,
        Key::KeyE => EguiKey::E,
        Key::KeyF => EguiKey::F,
        Key::KeyG => EguiKey::G,
        Key::KeyH => EguiKey::H,
        Key::KeyI => EguiKey::I,
        Key::KeyJ => EguiKey::J,
        Key::KeyK => EguiKey::K,
        Key::KeyL => EguiKey::L,
        Key::KeyM => EguiKey::M,
        Key::KeyN => EguiKey::N,
        Key::KeyO => EguiKey::O,
        Key::KeyP => EguiKey::P,
        Key::KeyQ => EguiKey::Q,
        Key::KeyR => EguiKey::R,
        Key::KeyS => EguiKey::S,
        Key::KeyT => EguiKey::T,
        Key::KeyU => EguiKey::U,
        Key::KeyV => EguiKey::V,
        Key::KeyW => EguiKey::W,
        Key::KeyX => EguiKey::X,
        Key::KeyY => EguiKey::Y,
        Key::KeyZ => EguiKey::Z,
        Key::F1 => EguiKey::F1,
        Key::F2 => EguiKey::F2,
        Key::F3 => EguiKey::F3,
        Key::F4 => EguiKey::F4,
        Key::F5 => EguiKey::F5,
        Key::F6 => EguiKey::F6,
        Key::F7 => EguiKey::F7,
        Key::F8 => EguiKey::F8,
        Key::F9 => EguiKey::F9,
        Key::F10 => EguiKey::F10,
        Key::F11 => EguiKey::F11,
        Key::F12 => EguiKey::F12,
        Key::F13 => EguiKey::F13,
        Key::F14 => EguiKey::F14,
        Key::F15 => EguiKey::F15,
        Key::F16 => EguiKey::F16,
        Key::F17 => EguiKey::F17,
        Key::F18 => EguiKey::F18,
        Key::F19 => EguiKey::F19,
        Key::F20 => EguiKey::F20,
        Key::F21 => EguiKey::F21,
        Key::F22 => EguiKey::F22,
        Key::F23 => EguiKey::F23,
        Key::F24 => EguiKey::F24,
        Key::F25 => EguiKey::F25,
        Key::F26 => EguiKey::F26,
        Key::F27 => EguiKey::F27,
        Key::F28 => EguiKey::F28,
        Key::F29 => EguiKey::F29,
        Key::F30 => EguiKey::F30,
        Key::F31 => EguiKey::F31,
        Key::F32 => EguiKey::F32,
        Key::F33 => EguiKey::F33,
        Key::F34 => EguiKey::F34,
        Key::F35 => EguiKey::F35,
        Key::ShiftLeft => EguiKey::ShiftLeft,
        Key::ShiftRight => EguiKey::ShiftRight,
        Key::ControlLeft => EguiKey::ControlLeft,
        Key::ControlRight => EguiKey::ControlRight,
        Key::AltLeft => EguiKey::AltLeft,
        Key::AltRight => EguiKey::AltRight,
        Key::MetaLeft => EguiKey::SuperLeft,
        Key::MetaRight => EguiKey::SuperRight,
        Key::IntlBackslash => EguiKey::IntlBackslash,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use egui::{Event, Key as EguiKey, Modifiers, MouseWheelUnit, PointerButton, Pos2, pos2, vec2};
    use nova_view::{Key, MouseButton};

    use super::*;
    use crate::app::{WheelDelta, WindowEvent};

    /// The egui events `events` give, through glue at `scale` on a Mac or
    /// not.
    fn egui_events(mac: bool, scale: f64, events: &[WindowEvent]) -> Vec<Event> {
        let mut input = EguiInput::new(scale, mac);
        for event in events {
            input.on_event(event);
        }
        input.take((800, 600), scale, Duration::ZERO).events
    }

    /// The egui events `events` give off a Mac at two pixels a point.
    fn at_2x(events: &[WindowEvent]) -> Vec<Event> {
        egui_events(false, 2.0, events)
    }

    fn moved(x: f64, y: f64) -> WindowEvent {
        WindowEvent::PointerMoved { px: (x, y) }
    }

    fn button(button: MouseButton, pressed: bool) -> WindowEvent {
        WindowEvent::PointerButton { button, pressed }
    }

    fn key(key: Key, pressed: bool) -> WindowEvent {
        WindowEvent::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    fn egui_button(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    fn egui_key(key: EguiKey, pressed: bool, repeat: bool, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat,
            modifiers,
        }
    }

    #[test]
    fn a_move_is_in_points() {
        assert_eq!(
            at_2x(&[moved(200.0, 100.0)]),
            [Event::PointerMoved(pos2(100.0, 50.0))]
        );
        let resized = WindowEvent::Resized {
            size_px: (800, 600),
            scale_factor: 1.0,
        };
        assert_eq!(
            at_2x(&[resized, moved(200.0, 100.0)]),
            [Event::PointerMoved(pos2(200.0, 100.0))]
        );
    }

    #[test]
    fn a_click_is_at_the_last_point() {
        let at = pos2(10.0, 20.0);
        assert_eq!(
            at_2x(&[
                moved(20.0, 40.0),
                button(MouseButton::Left, true),
                button(MouseButton::Left, false),
                button(MouseButton::Right, true),
                button(MouseButton::Middle, false),
                button(MouseButton::Other, true),
            ]),
            [
                Event::PointerMoved(at),
                egui_button(at, PointerButton::Primary, true),
                egui_button(at, PointerButton::Primary, false),
                egui_button(at, PointerButton::Secondary, true),
                egui_button(at, PointerButton::Middle, false),
            ]
        );
    }

    #[test]
    fn a_button_before_any_move_is_dropped() {
        assert_eq!(at_2x(&[button(MouseButton::Left, true)]), []);
    }

    #[test]
    fn leaving_forgets_the_point() {
        assert_eq!(
            at_2x(&[
                moved(20.0, 40.0),
                WindowEvent::PointerLeft,
                button(MouseButton::Left, true),
            ]),
            [Event::PointerMoved(pos2(10.0, 20.0)), Event::PointerGone]
        );
    }

    fn finger(f: usize) -> nova_view::FingerId {
        nova_view::FingerId::new(f)
    }

    fn touch_down(f: usize, x: f64, y: f64) -> WindowEvent {
        WindowEvent::TouchStarted {
            finger: finger(f),
            px: (x, y),
        }
    }

    fn touch_moved(f: usize, x: f64, y: f64) -> WindowEvent {
        WindowEvent::TouchMoved {
            finger: finger(f),
            px: (x, y),
        }
    }

    fn touch_up(f: usize, x: f64, y: f64) -> WindowEvent {
        WindowEvent::TouchEnded {
            finger: finger(f),
            px: (x, y),
        }
    }

    fn touch_cancelled(f: usize) -> WindowEvent {
        WindowEvent::TouchCancelled { finger: finger(f) }
    }

    #[test]
    fn a_first_finger_is_egui_s_pointer_pressing_and_releasing() {
        let at = pos2(10.0, 20.0);
        assert_eq!(
            at_2x(&[touch_down(1, 20.0, 40.0), touch_up(1, 20.0, 40.0)]),
            [
                Event::PointerMoved(at),
                egui_button(at, PointerButton::Primary, true),
                egui_button(at, PointerButton::Primary, false),
                Event::PointerGone,
            ]
        );
    }

    #[test]
    fn its_moves_move_the_pointer() {
        assert_eq!(
            at_2x(&[
                touch_down(1, 20.0, 40.0),
                touch_moved(1, 40.0, 60.0),
                touch_up(1, 40.0, 60.0),
            ]),
            [
                Event::PointerMoved(pos2(10.0, 20.0)),
                egui_button(pos2(10.0, 20.0), PointerButton::Primary, true),
                Event::PointerMoved(pos2(20.0, 30.0)),
                egui_button(pos2(20.0, 30.0), PointerButton::Primary, false),
                Event::PointerGone,
            ]
        );
    }

    #[test]
    fn a_second_finger_gives_nothing() {
        let first = at_2x(&[touch_down(1, 20.0, 40.0)]);
        assert_eq!(
            at_2x(&[
                touch_down(1, 20.0, 40.0),
                touch_down(2, 80.0, 80.0),
                touch_moved(2, 90.0, 90.0),
                touch_up(2, 90.0, 90.0),
                touch_cancelled(2),
            ]),
            first
        );
    }

    #[test]
    fn a_cancelled_touch_is_pointer_gone_without_a_release() {
        let at = pos2(10.0, 20.0);
        assert_eq!(
            at_2x(&[
                touch_down(1, 20.0, 40.0),
                touch_cancelled(1),
                button(MouseButton::Left, false),
            ]),
            [
                Event::PointerMoved(at),
                egui_button(at, PointerButton::Primary, true),
                Event::PointerGone,
            ],
            "and the point is forgotten"
        );
    }

    #[test]
    fn the_leave_after_a_lift_gives_nothing() {
        let lift = at_2x(&[touch_down(1, 20.0, 40.0), touch_up(1, 20.0, 40.0)]);
        assert_eq!(
            at_2x(&[
                touch_down(1, 20.0, 40.0),
                touch_up(1, 20.0, 40.0),
                touch_cancelled(1),
            ]),
            lift
        );
        assert_eq!(
            at_2x(&[touch_down(2, 2.0, 2.0)])[..1],
            [Event::PointerMoved(pos2(1.0, 1.0))],
            "the next finger is the pointer"
        );
    }

    #[test]
    fn the_wheel_scrolls_as_egui_expects() {
        let wheel = |unit, y| Event::MouseWheel {
            unit,
            delta: vec2(0.0, y),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        };
        assert_eq!(
            at_2x(&[
                WindowEvent::Scroll {
                    delta: WheelDelta::Lines(2.0)
                },
                WindowEvent::Scroll {
                    delta: WheelDelta::Px(24.0)
                },
                WindowEvent::Scroll {
                    delta: WheelDelta::Px(-6.0)
                },
            ]),
            [
                wheel(MouseWheelUnit::Line, -2.0),
                wheel(MouseWheelUnit::Point, -12.0),
                wheel(MouseWheelUnit::Point, 3.0),
            ]
        );
    }

    #[test]
    fn a_key_is_its_egui_key() {
        let cases = [
            (Key::KeyA, EguiKey::A),
            (Key::Enter, EguiKey::Enter),
            (Key::NumpadEnter, EguiKey::Enter),
            (Key::Backspace, EguiKey::Backspace),
            (Key::ArrowLeft, EguiKey::ArrowLeft),
            (Key::End, EguiKey::End),
            (Key::Digit6, EguiKey::Num6),
            (Key::F12, EguiKey::F12),
        ];
        for (nova, egui) in cases {
            let repeat = WindowEvent::Key {
                key: nova,
                pressed: true,
                repeat: true,
            };
            assert_eq!(
                at_2x(&[key(nova, true), repeat, key(nova, false)]),
                [
                    egui_key(egui, true, false, Modifiers::NONE),
                    egui_key(egui, true, true, Modifiers::NONE),
                    egui_key(egui, false, false, Modifiers::NONE),
                ],
                "{nova:?}"
            );
        }
    }

    #[test]
    fn a_key_egui_has_no_name_for_gives_nothing() {
        for unnamed in [Key::Unidentified, Key::CapsLock, Key::NumLock, Key::Fn] {
            assert_eq!(at_2x(&[key(unnamed, true)]), [], "{unnamed:?}");
        }
    }

    /// egui-winit 0.36.2's `key_from_key_code`, over the same names.
    const TABLE: &[(Key, EguiKey)] = &[
        (Key::ArrowDown, EguiKey::ArrowDown),
        (Key::ArrowLeft, EguiKey::ArrowLeft),
        (Key::ArrowRight, EguiKey::ArrowRight),
        (Key::ArrowUp, EguiKey::ArrowUp),
        (Key::Escape, EguiKey::Escape),
        (Key::Tab, EguiKey::Tab),
        (Key::Backspace, EguiKey::Backspace),
        (Key::Enter, EguiKey::Enter),
        (Key::NumpadEnter, EguiKey::Enter),
        (Key::Insert, EguiKey::Insert),
        (Key::Delete, EguiKey::Delete),
        (Key::Home, EguiKey::Home),
        (Key::End, EguiKey::End),
        (Key::PageUp, EguiKey::PageUp),
        (Key::PageDown, EguiKey::PageDown),
        (Key::Space, EguiKey::Space),
        (Key::Comma, EguiKey::Comma),
        (Key::Period, EguiKey::Period),
        (Key::Semicolon, EguiKey::Semicolon),
        (Key::Backslash, EguiKey::Backslash),
        (Key::Slash, EguiKey::Slash),
        (Key::NumpadDivide, EguiKey::Slash),
        (Key::BracketLeft, EguiKey::OpenBracket),
        (Key::BracketRight, EguiKey::CloseBracket),
        (Key::Backquote, EguiKey::Backtick),
        (Key::Quote, EguiKey::Quote),
        (Key::Cut, EguiKey::Cut),
        (Key::Copy, EguiKey::Copy),
        (Key::Paste, EguiKey::Paste),
        (Key::Minus, EguiKey::Minus),
        (Key::NumpadSubtract, EguiKey::Minus),
        (Key::NumpadAdd, EguiKey::Plus),
        (Key::Equal, EguiKey::Equals),
        (Key::Digit0, EguiKey::Num0),
        (Key::Digit1, EguiKey::Num1),
        (Key::Digit2, EguiKey::Num2),
        (Key::Digit3, EguiKey::Num3),
        (Key::Digit4, EguiKey::Num4),
        (Key::Digit5, EguiKey::Num5),
        (Key::Digit6, EguiKey::Num6),
        (Key::Digit7, EguiKey::Num7),
        (Key::Digit8, EguiKey::Num8),
        (Key::Digit9, EguiKey::Num9),
        (Key::Numpad0, EguiKey::Num0),
        (Key::Numpad1, EguiKey::Num1),
        (Key::Numpad2, EguiKey::Num2),
        (Key::Numpad3, EguiKey::Num3),
        (Key::Numpad4, EguiKey::Num4),
        (Key::Numpad5, EguiKey::Num5),
        (Key::Numpad6, EguiKey::Num6),
        (Key::Numpad7, EguiKey::Num7),
        (Key::Numpad8, EguiKey::Num8),
        (Key::Numpad9, EguiKey::Num9),
        (Key::KeyA, EguiKey::A),
        (Key::KeyB, EguiKey::B),
        (Key::KeyC, EguiKey::C),
        (Key::KeyD, EguiKey::D),
        (Key::KeyE, EguiKey::E),
        (Key::KeyF, EguiKey::F),
        (Key::KeyG, EguiKey::G),
        (Key::KeyH, EguiKey::H),
        (Key::KeyI, EguiKey::I),
        (Key::KeyJ, EguiKey::J),
        (Key::KeyK, EguiKey::K),
        (Key::KeyL, EguiKey::L),
        (Key::KeyM, EguiKey::M),
        (Key::KeyN, EguiKey::N),
        (Key::KeyO, EguiKey::O),
        (Key::KeyP, EguiKey::P),
        (Key::KeyQ, EguiKey::Q),
        (Key::KeyR, EguiKey::R),
        (Key::KeyS, EguiKey::S),
        (Key::KeyT, EguiKey::T),
        (Key::KeyU, EguiKey::U),
        (Key::KeyV, EguiKey::V),
        (Key::KeyW, EguiKey::W),
        (Key::KeyX, EguiKey::X),
        (Key::KeyY, EguiKey::Y),
        (Key::KeyZ, EguiKey::Z),
        (Key::F1, EguiKey::F1),
        (Key::F2, EguiKey::F2),
        (Key::F3, EguiKey::F3),
        (Key::F4, EguiKey::F4),
        (Key::F5, EguiKey::F5),
        (Key::F6, EguiKey::F6),
        (Key::F7, EguiKey::F7),
        (Key::F8, EguiKey::F8),
        (Key::F9, EguiKey::F9),
        (Key::F10, EguiKey::F10),
        (Key::F11, EguiKey::F11),
        (Key::F12, EguiKey::F12),
        (Key::F13, EguiKey::F13),
        (Key::F14, EguiKey::F14),
        (Key::F15, EguiKey::F15),
        (Key::F16, EguiKey::F16),
        (Key::F17, EguiKey::F17),
        (Key::F18, EguiKey::F18),
        (Key::F19, EguiKey::F19),
        (Key::F20, EguiKey::F20),
        (Key::F21, EguiKey::F21),
        (Key::F22, EguiKey::F22),
        (Key::F23, EguiKey::F23),
        (Key::F24, EguiKey::F24),
        (Key::F25, EguiKey::F25),
        (Key::F26, EguiKey::F26),
        (Key::F27, EguiKey::F27),
        (Key::F28, EguiKey::F28),
        (Key::F29, EguiKey::F29),
        (Key::F30, EguiKey::F30),
        (Key::F31, EguiKey::F31),
        (Key::F32, EguiKey::F32),
        (Key::F33, EguiKey::F33),
        (Key::F34, EguiKey::F34),
        (Key::F35, EguiKey::F35),
        (Key::ShiftLeft, EguiKey::ShiftLeft),
        (Key::ShiftRight, EguiKey::ShiftRight),
        (Key::ControlLeft, EguiKey::ControlLeft),
        (Key::ControlRight, EguiKey::ControlRight),
        (Key::AltLeft, EguiKey::AltLeft),
        (Key::AltRight, EguiKey::AltRight),
        (Key::MetaLeft, EguiKey::SuperLeft),
        (Key::MetaRight, EguiKey::SuperRight),
        (Key::IntlBackslash, EguiKey::IntlBackslash),
    ];

    #[test]
    fn every_table_row_maps() {
        for &(nova, egui) in TABLE {
            assert_eq!(super::egui_key(nova), Some(egui), "{nova:?}");
        }
    }

    #[test]
    fn typed_text_is_text() {
        assert_eq!(
            at_2x(&[WindowEvent::Text('é')]),
            [Event::Text("é".to_owned())]
        );
    }

    fn shift() -> Modifiers {
        Modifiers {
            shift: true,
            ..Modifiers::NONE
        }
    }

    #[test]
    fn shift_is_held_until_both_shifts_are_up() {
        assert_eq!(
            at_2x(&[
                key(Key::ShiftLeft, true),
                key(Key::ShiftRight, true),
                key(Key::ShiftLeft, false),
                key(Key::KeyA, true),
                key(Key::ShiftRight, false),
            ]),
            [
                Event::ModifiersChanged(shift()),
                egui_key(EguiKey::ShiftLeft, true, false, shift()),
                egui_key(EguiKey::ShiftRight, true, false, shift()),
                egui_key(EguiKey::ShiftLeft, false, false, shift()),
                egui_key(EguiKey::A, true, false, shift()),
                Event::ModifiersChanged(Modifiers::NONE),
                egui_key(EguiKey::ShiftRight, false, false, Modifiers::NONE),
            ]
        );
    }

    #[test]
    fn alt_and_control_are_their_keys() {
        let alt = Modifiers {
            alt: true,
            ..Modifiers::NONE
        };
        assert_eq!(
            at_2x(&[key(Key::AltRight, true)])[0],
            Event::ModifiersChanged(alt)
        );
        for mac in [true, false] {
            let events = egui_events(mac, 1.0, &[key(Key::ControlRight, true)]);
            let Event::ModifiersChanged(held) = events[0] else {
                panic!("{events:?}");
            };
            assert!(held.ctrl && !held.alt && !held.shift, "{held:?}");
        }
    }

    #[test]
    fn the_command_key_is_meta_on_a_mac_and_control_elsewhere() {
        let changed = |mac, held: Key| egui_events(mac, 1.0, &[key(held, true)])[0].clone();
        assert_eq!(
            changed(true, Key::MetaLeft),
            Event::ModifiersChanged(Modifiers {
                mac_cmd: true,
                command: true,
                ..Modifiers::NONE
            })
        );
        assert_eq!(
            changed(true, Key::ControlLeft),
            Event::ModifiersChanged(Modifiers {
                ctrl: true,
                ..Modifiers::NONE
            })
        );
        // Off a Mac, Meta (the Windows key) holds no egui modifier.
        assert_eq!(
            egui_events(false, 1.0, &[key(Key::MetaRight, true)]),
            [egui_key(EguiKey::SuperRight, true, false, Modifiers::NONE)]
        );
        assert_eq!(
            changed(false, Key::ControlLeft),
            Event::ModifiersChanged(Modifiers {
                ctrl: true,
                command: true,
                ..Modifiers::NONE
            })
        );
    }

    #[test]
    fn text_typed_with_command_or_control_is_dropped() {
        let typed = |mac, held| {
            egui_events(mac, 1.0, &[key(held, true), WindowEvent::Text('a')])
                .into_iter()
                .filter(|event| matches!(event, Event::Text(_)))
                .count()
        };
        assert_eq!(typed(true, Key::MetaLeft), 0);
        assert_eq!(typed(true, Key::ControlRight), 0);
        assert_eq!(typed(false, Key::ControlLeft), 0);
        assert_eq!(typed(false, Key::MetaLeft), 1, "the Windows key");
        assert_eq!(typed(true, Key::ShiftLeft), 1);
        assert_eq!(typed(false, Key::AltLeft), 1);
    }

    #[test]
    fn command_x_and_c_cut_and_copy() {
        for (mac, command) in [(true, Key::MetaLeft), (false, Key::ControlRight)] {
            let events = egui_events(
                mac,
                1.0,
                &[
                    key(command, true),
                    key(Key::KeyX, true),
                    key(Key::KeyX, false),
                    key(Key::KeyC, true),
                    key(Key::KeyV, true),
                    key(Key::KeyA, true),
                ],
            );
            let Event::ModifiersChanged(held) = events[0] else {
                panic!("{events:?}");
            };
            // The command's own key, then a cut, X's release, a copy,
            // nothing for a paste (there is no clipboard), and A.
            assert_eq!(
                events[2..],
                [
                    Event::Cut,
                    egui_key(EguiKey::X, false, false, held),
                    Event::Copy,
                    egui_key(EguiKey::A, true, false, held),
                ],
                "mac: {mac}"
            );
        }
        // Without the command key, X and C are keys.
        assert_eq!(
            at_2x(&[key(Key::KeyX, true), key(Key::KeyC, true)]),
            [
                egui_key(EguiKey::X, true, false, Modifiers::NONE),
                egui_key(EguiKey::C, true, false, Modifiers::NONE),
            ]
        );
        // The keyboard's own Cut and Copy keys.
        assert_eq!(
            at_2x(&[
                key(Key::Cut, true),
                key(Key::Copy, true),
                key(Key::Paste, true)
            ]),
            [Event::Cut, Event::Copy]
        );
    }

    #[test]
    fn the_pointer_and_the_wheel_carry_the_modifiers_held() {
        let at = pos2(5.0, 5.0);
        assert_eq!(
            at_2x(&[
                key(Key::ShiftLeft, true),
                moved(10.0, 10.0),
                button(MouseButton::Left, true),
                WindowEvent::Scroll {
                    delta: WheelDelta::Lines(1.0)
                },
            ])[2..],
            [
                Event::PointerMoved(at),
                Event::PointerButton {
                    pos: at,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: shift(),
                },
                Event::MouseWheel {
                    unit: MouseWheelUnit::Line,
                    delta: vec2(0.0, -1.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: shift(),
                },
            ]
        );
    }

    #[test]
    fn losing_focus_lets_go_of_the_modifiers() {
        let mut input = EguiInput::new(1.0, false);
        input.on_event(&key(Key::ShiftLeft, true));
        input.on_event(&WindowEvent::FocusLost);
        input.on_event(&key(Key::KeyA, true));
        let lost = input.take((800, 600), 1.0, Duration::ZERO);
        assert_eq!(
            lost.events,
            [
                Event::ModifiersChanged(shift()),
                egui_key(EguiKey::ShiftLeft, true, false, shift()),
                Event::ModifiersChanged(Modifiers::NONE),
                Event::WindowFocused(false),
                egui_key(EguiKey::A, true, false, Modifiers::NONE),
            ]
        );
        assert!(!lost.focused);
        input.on_event(&WindowEvent::FocusGained);
        let gained = input.take((800, 600), 1.0, Duration::ZERO);
        assert_eq!(gained.events, [Event::WindowFocused(true)]);
        assert!(gained.focused);
    }

    #[test]
    fn a_window_starts_focused() {
        assert!(
            EguiInput::new(1.0, true)
                .take((800, 600), 1.0, Duration::ZERO)
                .focused
        );
    }

    #[test]
    fn take_gives_the_screen_in_points_and_drains() {
        let mut input = EguiInput::new(1.0, false);
        input.on_event(&WindowEvent::Text('a'));
        input.on_event(&WindowEvent::Text('b'));
        let elapsed = Duration::from_millis(1500);
        let raw = input.take((2048, 1536), 2.0, elapsed);
        assert_eq!(
            raw.screen_rect,
            Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1024.0, 768.0)))
        );
        assert_eq!(
            raw.viewports[&egui::ViewportId::ROOT].native_pixels_per_point,
            Some(2.0)
        );
        assert_eq!(raw.viewport_id, egui::ViewportId::ROOT);
        assert_eq!(raw.max_texture_side, Some(8192));
        assert_eq!(raw.time, Some(1.5));
        assert_eq!(
            raw.events,
            [Event::Text("a".to_owned()), Event::Text("b".to_owned())]
        );
        assert_eq!(input.take((2048, 1536), 2.0, elapsed).events, []);
        // The scale `take` was given is the one later points are in.
        input.on_event(&moved(300.0, 30.0));
        assert_eq!(
            input.take((2048, 1536), 2.0, elapsed).events,
            [Event::PointerMoved(pos2(150.0, 15.0))]
        );
    }

    #[test]
    fn a_window_of_no_size_has_no_screen() {
        let mut input = EguiInput::new(1.0, false);
        for size in [(0, 0), (0, 768), (1024, 0)] {
            assert_eq!(
                input.take(size, 1.0, Duration::ZERO).screen_rect,
                None,
                "{size:?}"
            );
        }
    }

    #[test]
    fn redraws_and_other_events_give_nothing() {
        assert_eq!(
            at_2x(&[
                WindowEvent::Redraw {
                    elapsed: Duration::ZERO
                },
                WindowEvent::FullscreenChanged { fullscreen: true },
                WindowEvent::CloseRequested,
                WindowEvent::CheckForUpdates,
                WindowEvent::Resized {
                    size_px: (100, 100),
                    scale_factor: 2.0
                },
            ]),
            []
        );
    }
}
