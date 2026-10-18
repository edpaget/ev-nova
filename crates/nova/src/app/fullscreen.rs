//! Borderless fullscreen: the platform's shortcut toggles it, and the
//! window's size before it is given back on leaving.
//!
//! [`Fullscreen`] holds the decisions; the window adapter only carries
//! them out. The shortcut is the platform's usual one: Control+Command+F
//! on macOS, and Alt+Enter or F11 elsewhere. The system can change the
//! state without it (macOS's green button, its Window menu), and reports
//! the change through [`Fullscreen::observed`]; either way, leaving gives
//! the window back the size it had before entering.

use nova_view::Key;

use crate::app::WindowPort;
use crate::config::Os;

/// What a key event means to fullscreen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shortcut {
    /// Nothing: the key goes on to the game.
    None,
    /// The shortcut was pressed: toggle, and keep the key from the game.
    Toggle,
    /// A held shortcut's repeat, or its key's release: keep it from the
    /// game, but do nothing.
    Swallowed,
}

/// The modifiers the shortcuts use, as held down.
#[derive(Clone, Copy, Debug, Default)]
struct Modifiers {
    control: bool,
    command: bool,
    alt: bool,
}

/// Whether the game is fullscreen, the window's size to give back on
/// leaving, and what the shortcut needs to know of the keys held.
#[derive(Clone, Debug)]
pub(crate) struct Fullscreen {
    os: Os,
    on: bool,
    /// The window's inner size in physical pixels, the last time it was
    /// seen windowed.
    windowed_size_px: (u32, u32),
    held_modifiers: Modifiers,
    /// The shortcut's key while it is held down: its repeats and its
    /// release are the shortcut's, not the game's.
    held: Option<Key>,
    /// Whether the last key event was a shortcut press, whose text (some
    /// platforms type one for a chord) is dropped.
    drop_text: bool,
}

impl Fullscreen {
    /// Windowed, in a window of `size_px`, reading `os`'s shortcut.
    pub(crate) fn new(os: Os, size_px: (u32, u32)) -> Self {
        Self {
            os,
            on: false,
            windowed_size_px: size_px,
            held_modifiers: Modifiers::default(),
            held: None,
            drop_text: false,
        }
    }

    /// Reads `os`'s shortcut from now on.
    pub(crate) fn set_os(&mut self, os: Os) {
        self.os = os;
    }

    /// Whether the game is fullscreen.
    pub(crate) fn is_on(&self) -> bool {
        self.on
    }

    /// Takes a key event: tracks the modifiers held, and says whether it
    /// is the shortcut. A repeat never toggles, so holding the keys does
    /// not flicker, but it is swallowed with the press it repeats.
    pub(crate) fn key(&mut self, key: Key, pressed: bool, repeat: bool) -> Shortcut {
        self.drop_text = false;
        match key {
            Key::Control => self.held_modifiers.control = pressed,
            Key::Command => self.held_modifiers.command = pressed,
            Key::Alt => self.held_modifiers.alt = pressed,
            _ => {}
        }
        if self.held == Some(key) {
            if !pressed {
                self.held = None;
            }
            self.drop_text = pressed;
            return Shortcut::Swallowed;
        }
        if pressed && !repeat && self.is_shortcut(key) {
            self.held = Some(key);
            self.drop_text = true;
            return Shortcut::Toggle;
        }
        Shortcut::None
    }

    /// Whether pressing `key` with the modifiers held is the shortcut.
    fn is_shortcut(&self, key: Key) -> bool {
        let held = self.held_modifiers;
        match self.os {
            Os::MacOs => key == Key::Char('f') && held.control && held.command,
            Os::Windows | Os::Other => (key == Key::Enter && held.alt) || key == Key::Function(11),
        }
    }

    /// Whether the text typed now is to be dropped: it is when it follows
    /// a shortcut press directly. Asking clears it.
    pub(crate) fn drops_text(&mut self) -> bool {
        std::mem::take(&mut self.drop_text)
    }

    /// Forgets the keys held, since their releases will not arrive.
    pub(crate) fn focus_lost(&mut self) {
        self.held_modifiers = Modifiers::default();
        self.held = None;
        self.drop_text = false;
    }

    /// Enters fullscreen, remembering `window`'s size to give back, or
    /// leaves it, giving that size back. Says whether it is now on.
    pub(crate) fn toggle(&mut self, window: &mut impl WindowPort) -> bool {
        if self.on {
            self.leave(window);
        } else {
            self.windowed_size_px = window.size_px();
            window.set_fullscreen(true);
            self.on = true;
        }
        self.on
    }

    fn leave(&mut self, window: &mut impl WindowPort) {
        window.set_fullscreen(false);
        window.request_inner_size(self.windowed_size_px);
        self.on = false;
    }

    /// Takes the system's report of whether the window is `fullscreen`,
    /// and says whether that changed the state. Entering asks nothing of
    /// the window, which is already fullscreen; leaving gives back the
    /// size it had, however the player left.
    pub(crate) fn observed(&mut self, fullscreen: bool, window: &mut impl WindowPort) -> bool {
        if fullscreen == self.on {
            return false;
        }
        if fullscreen {
            self.on = true;
        } else {
            window.request_inner_size(self.windowed_size_px);
            self.on = false;
        }
        true
    }

    /// Takes the window's new size: while windowed, it is the size to give
    /// back on leaving. While fullscreen it is the display's, and ignored.
    pub(crate) fn resized(&mut self, size_px: (u32, u32)) {
        if !self.on {
            self.windowed_size_px = size_px;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Call {
        SetFullscreen(bool),
        RequestSize((u32, u32)),
    }

    /// A window of settable size that records each fullscreen and size
    /// request, in order.
    struct RecordingWindow {
        size_px: (u32, u32),
        calls: Vec<Call>,
    }

    impl RecordingWindow {
        fn new(size_px: (u32, u32)) -> Self {
            Self {
                size_px,
                calls: Vec::new(),
            }
        }
    }

    impl WindowPort for RecordingWindow {
        fn size_px(&self) -> (u32, u32) {
            self.size_px
        }

        fn scale_factor(&self) -> f64 {
            2.0
        }

        fn request_redraw(&mut self) {}

        fn set_fullscreen(&mut self, on: bool) {
            self.calls.push(Call::SetFullscreen(on));
        }

        fn request_inner_size(&mut self, size_px: (u32, u32)) {
            self.calls.push(Call::RequestSize(size_px));
        }
    }

    fn press(fullscreen: &mut Fullscreen, key: Key) -> Shortcut {
        fullscreen.key(key, true, false)
    }

    fn release(fullscreen: &mut Fullscreen, key: Key) -> Shortcut {
        fullscreen.key(key, false, false)
    }

    /// Whether pressing `chord`'s keys in order makes its last press the
    /// toggle, on `os`.
    fn toggles(os: Os, chord: &[Key]) -> bool {
        let mut fullscreen = Fullscreen::new(os, (1024, 768));
        let (last, held) = chord.split_last().expect("a key");
        for key in held {
            assert_eq!(press(&mut fullscreen, *key), Shortcut::None, "{key:?}");
        }
        press(&mut fullscreen, *last) == Shortcut::Toggle
    }

    #[test]
    fn on_macos_control_command_f_is_the_shortcut() {
        let mac = Os::MacOs;
        assert!(toggles(mac, &[Key::Control, Key::Command, Key::Char('f')]));
        assert!(toggles(mac, &[Key::Command, Key::Control, Key::Char('f')]));
        assert!(!toggles(mac, &[Key::Char('f')]));
        assert!(!toggles(mac, &[Key::Control, Key::Char('f')]));
        assert!(!toggles(mac, &[Key::Command, Key::Char('f')]));
        assert!(!toggles(mac, &[Key::Control, Key::Command, Key::Char('g')]));
        assert!(!toggles(mac, &[Key::Alt, Key::Enter]));
        assert!(!toggles(mac, &[Key::Function(11)]));
    }

    #[test]
    fn elsewhere_alt_enter_and_f11_are_the_shortcut() {
        for os in [Os::Windows, Os::Other] {
            assert!(toggles(os, &[Key::Alt, Key::Enter]), "{os:?}");
            assert!(toggles(os, &[Key::Function(11)]), "{os:?}");
            assert!(!toggles(os, &[Key::Enter]), "{os:?}");
            assert!(!toggles(os, &[Key::Function(10)]), "{os:?}");
            assert!(!toggles(os, &[Key::Alt, Key::Space]), "{os:?}");
            assert!(
                !toggles(os, &[Key::Control, Key::Command, Key::Char('f')]),
                "{os:?}"
            );
        }
    }

    #[test]
    fn a_released_modifier_stops_counting() {
        let mut fullscreen = Fullscreen::new(Os::MacOs, (1024, 768));
        press(&mut fullscreen, Key::Control);
        press(&mut fullscreen, Key::Command);
        assert_eq!(release(&mut fullscreen, Key::Control), Shortcut::None);
        assert_eq!(press(&mut fullscreen, Key::Char('f')), Shortcut::None);
        let mut fullscreen = Fullscreen::new(Os::MacOs, (1024, 768));
        press(&mut fullscreen, Key::Control);
        press(&mut fullscreen, Key::Command);
        release(&mut fullscreen, Key::Command);
        assert_eq!(press(&mut fullscreen, Key::Char('f')), Shortcut::None);
        let mut fullscreen = Fullscreen::new(Os::Windows, (1024, 768));
        press(&mut fullscreen, Key::Alt);
        release(&mut fullscreen, Key::Alt);
        assert_eq!(press(&mut fullscreen, Key::Enter), Shortcut::None);
    }

    #[test]
    fn losing_focus_forgets_the_held_modifiers() {
        let mut fullscreen = Fullscreen::new(Os::MacOs, (1024, 768));
        press(&mut fullscreen, Key::Control);
        press(&mut fullscreen, Key::Command);
        fullscreen.focus_lost();
        assert_eq!(press(&mut fullscreen, Key::Char('f')), Shortcut::None);
        let mut fullscreen = Fullscreen::new(Os::Windows, (1024, 768));
        press(&mut fullscreen, Key::Alt);
        fullscreen.focus_lost();
        assert_eq!(press(&mut fullscreen, Key::Enter), Shortcut::None);
    }

    #[test]
    fn a_held_shortcut_is_swallowed_without_toggling_again_and_so_is_its_release() {
        let mut fullscreen = Fullscreen::new(Os::Windows, (1024, 768));
        assert_eq!(press(&mut fullscreen, Key::Function(11)), Shortcut::Toggle);
        assert_eq!(
            fullscreen.key(Key::Function(11), true, true),
            Shortcut::Swallowed
        );
        assert_eq!(
            release(&mut fullscreen, Key::Function(11)),
            Shortcut::Swallowed
        );
        assert_eq!(
            release(&mut fullscreen, Key::Function(11)),
            Shortcut::None,
            "only the one release"
        );
        assert_eq!(
            fullscreen.key(Key::Function(11), true, true),
            Shortcut::None,
            "a repeat of a key not taken as the shortcut"
        );
    }

    #[test]
    fn the_shortcuts_key_released_after_its_modifier_is_still_swallowed() {
        let mut fullscreen = Fullscreen::new(Os::Windows, (1024, 768));
        press(&mut fullscreen, Key::Alt);
        assert_eq!(press(&mut fullscreen, Key::Enter), Shortcut::Toggle);
        assert_eq!(release(&mut fullscreen, Key::Alt), Shortcut::None);
        assert_eq!(release(&mut fullscreen, Key::Enter), Shortcut::Swallowed);
        assert_eq!(release(&mut fullscreen, Key::Enter), Shortcut::None);
    }

    #[test]
    fn only_the_text_right_after_a_shortcut_press_is_dropped() {
        let mut fullscreen = Fullscreen::new(Os::MacOs, (1024, 768));
        assert!(!fullscreen.drops_text(), "no shortcut yet");
        press(&mut fullscreen, Key::Control);
        press(&mut fullscreen, Key::Command);
        press(&mut fullscreen, Key::Char('f'));
        assert!(fullscreen.drops_text());
        assert!(!fullscreen.drops_text(), "the one text only");
        press(&mut fullscreen, Key::Char('f'));
        release(&mut fullscreen, Key::Char('f'));
        assert!(!fullscreen.drops_text(), "another key came between");
    }

    #[test]
    fn toggling_enters_then_gives_back_the_windows_size() {
        let mut window = RecordingWindow::new((2048, 1536));
        let mut fullscreen = Fullscreen::new(Os::MacOs, (1024, 768));
        assert!(!fullscreen.is_on());
        assert!(fullscreen.toggle(&mut window));
        assert!(fullscreen.is_on());
        assert_eq!(window.calls, [Call::SetFullscreen(true)]);
        window.size_px = (3024, 1964);
        assert!(!fullscreen.toggle(&mut window));
        assert!(!fullscreen.is_on());
        assert_eq!(
            window.calls,
            [
                Call::SetFullscreen(true),
                Call::SetFullscreen(false),
                Call::RequestSize((2048, 1536))
            ]
        );
    }

    #[test]
    fn a_resize_while_windowed_is_the_size_given_back_and_one_while_fullscreen_is_not() {
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = Fullscreen::new(Os::MacOs, (1024, 768));
        fullscreen.resized((1600, 1200));
        fullscreen.observed(true, &mut window);
        fullscreen.resized((3024, 1964));
        fullscreen.toggle(&mut window);
        assert_eq!(
            window.calls,
            [Call::SetFullscreen(false), Call::RequestSize((1600, 1200))]
        );
    }

    #[test]
    fn the_systems_report_of_a_change_is_followed() {
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = Fullscreen::new(Os::MacOs, (1280, 960));
        assert!(fullscreen.observed(true, &mut window), "changed");
        assert!(fullscreen.is_on());
        assert_eq!(window.calls, [], "the system already did it");
        assert!(fullscreen.observed(false, &mut window), "changed");
        assert!(!fullscreen.is_on());
        assert_eq!(window.calls, [Call::RequestSize((1280, 960))]);
    }

    #[test]
    fn a_report_of_the_state_held_does_nothing() {
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = Fullscreen::new(Os::MacOs, (1024, 768));
        assert!(!fullscreen.observed(false, &mut window));
        assert!(!fullscreen.is_on());
        fullscreen.toggle(&mut window);
        window.calls.clear();
        assert!(!fullscreen.observed(true, &mut window));
        assert!(fullscreen.is_on());
        assert_eq!(window.calls, []);
    }
}
