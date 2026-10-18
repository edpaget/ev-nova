//! Fullscreen: borderless, or exclusive in a display mode the player
//! picked. The platform's shortcut toggles it, and the window's size
//! before it is given back on leaving.
//!
//! [`Fullscreen`] holds the decisions; the window adapter only carries
//! them out. The shortcut is the platform's usual one: Control+Command+F
//! on macOS, and Alt+Enter or F11 elsewhere. The system can change the
//! state without it (macOS's green button, its Window menu), and reports
//! the change through [`Fullscreen::observed`]; either way, leaving gives
//! the window back the size it had before entering.
//!
//! # Kinds
//!
//! Entering enters the chosen kind ([`Fullscreen::kind`]), the saved one
//! unless the player picks another. An exclusive kind is entered only
//! when the window's monitor offers its mode (listed by
//! [`listed_modes`]) and, when both are named, is the monitor it was
//! chosen on. Otherwise the game enters borderless with a warning naming
//! the mode, and stays borderless for the session, until the player
//! chooses a kind; it never guesses at the nearest mode. The chosen kind
//! is kept through the fallback, so the Preferences dialog shows it, and
//! picking borderless there is a change, saved.
//!
//! # Focus and exit
//!
//! Losing focus while exclusive leaves the mode, so the desktop gets its
//! own back, and suspends: the system's report that the window left
//! fullscreen is then expected, and changes nothing. Gaining focus enters
//! the same mode again. Exiting leaves an exclusive mode too, and after
//! it the system's reports are ignored, so the state saved is the one
//! the player quit in.

use nova_view::{FullscreenKind, Key, VideoMode, listed_modes};

use crate::app::WindowPort;
use crate::config::Os;
use crate::settings::FullscreenSetting;

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

/// Whether and how the game is fullscreen, the kind to enter, the
/// window's size to give back on leaving, and what the shortcut needs to
/// know of the keys held.
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)] // Independent flags, each its own rule.
pub(crate) struct Fullscreen {
    os: Os,
    /// How the game is fullscreen now, as asked or as the system reported;
    /// `None` when windowed.
    current: Option<FullscreenKind>,
    /// The kind the player chose, which entering enters unless the
    /// session fell back from it.
    kind: FullscreenSetting,
    /// Whether the window's monitor did not offer the chosen kind's mode,
    /// and the session runs borderless instead, warned once.
    fell_back: bool,
    /// Whether an exclusive mode was left for a focus loss, to be entered
    /// again when focus comes back.
    suspended: bool,
    /// Whether the app is exiting.
    exited: bool,
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
    /// Windowed, in a window of `size_px`, reading `os`'s shortcut, its
    /// kind borderless.
    pub(crate) fn new(os: Os, size_px: (u32, u32)) -> Self {
        Self {
            os,
            current: None,
            kind: FullscreenSetting::Borderless,
            fell_back: false,
            suspended: false,
            exited: false,
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

    /// Enters `kind` from now on, without changing the state now.
    pub(crate) fn set_kind(&mut self, kind: FullscreenSetting) {
        self.kind = kind;
        self.fell_back = false;
    }

    /// The kind entering enters now: borderless after a fallback, and the
    /// chosen kind otherwise.
    pub(crate) fn live_kind(&self) -> FullscreenKind {
        if self.fell_back {
            FullscreenKind::Borderless
        } else {
            self.kind.kind()
        }
    }

    /// The kind the player chose, kept through a fallback.
    pub(crate) fn kind(&self) -> &FullscreenSetting {
        &self.kind
    }

    /// Whether the game is fullscreen, an exclusive mode suspended for a
    /// focus loss included.
    pub(crate) fn is_on(&self) -> bool {
        self.current.is_some()
    }

    /// Whether the game is in an exclusive mode, not suspended, and not
    /// exiting.
    fn exclusive(&self) -> bool {
        matches!(self.current, Some(FullscreenKind::Exclusive(_)))
            && !self.suspended
            && !self.exited
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

    /// Forgets the keys held, since their releases will not arrive, and,
    /// while exclusive, leaves the mode until focus comes back.
    pub(crate) fn focus_lost(&mut self, window: &mut impl WindowPort) {
        self.held_modifiers = Modifiers::default();
        self.held = None;
        self.drop_text = false;
        if self.exclusive() {
            window.set_fullscreen(None);
            self.suspended = true;
        }
    }

    /// Enters again the exclusive mode a focus loss left.
    pub(crate) fn focus_gained(&mut self, window: &mut impl WindowPort) {
        if self.suspended && !self.exited {
            self.suspended = false;
            window.set_fullscreen(self.current);
        }
    }

    /// The app is exiting: leaves an exclusive mode, so the display gets
    /// its own back, and ignores the system's reports from now on.
    pub(crate) fn exiting(&mut self, window: &mut impl WindowPort) {
        if self.exclusive() {
            window.set_fullscreen(None);
        }
        self.exited = true;
    }

    /// Enters the chosen kind, remembering `window`'s size to give back, or
    /// leaves fullscreen, giving that size back. A warning when the chosen
    /// kind's mode is first found not offered and borderless is entered
    /// instead.
    pub(crate) fn toggle(&mut self, window: &mut impl WindowPort) -> Option<String> {
        if self.current.is_some() {
            window.set_fullscreen(None);
            window.request_inner_size(self.windowed_size_px);
            self.current = None;
            self.suspended = false;
            None
        } else {
            self.windowed_size_px = window.size_px();
            self.enter(window)
        }
    }

    /// Makes `kind` the chosen kind and enters it, afresh after a
    /// fallback: from a window, remembering its size; from fullscreen,
    /// switching to it alone. A warning as [`Fullscreen::toggle`] gives one.
    pub(crate) fn choose(
        &mut self,
        kind: FullscreenSetting,
        window: &mut impl WindowPort,
    ) -> Option<String> {
        self.set_kind(kind);
        if self.current.is_none() {
            self.windowed_size_px = window.size_px();
        }
        self.enter(window)
    }

    /// Asks `window` for the chosen kind, falling back to borderless, for
    /// the rest of the session, when its monitor does not offer the
    /// kind's mode.
    fn enter(&mut self, window: &mut impl WindowPort) -> Option<String> {
        let warning = match &self.kind {
            FullscreenSetting::Exclusive { mode, monitor }
                if !self.fell_back && !offered(window, mode, monitor.as_deref()) =>
            {
                let on = monitor
                    .as_ref()
                    .map(|name| format!(" ({name})"))
                    .unwrap_or_default();
                let warning = format!(
                    "nova: this monitor does not offer the saved display mode {}{on}; \
                     opening borderless fullscreen",
                    mode.label()
                );
                self.fell_back = true;
                Some(warning)
            }
            _ => None,
        };
        let kind = self.live_kind();
        window.set_fullscreen(Some(kind));
        self.current = Some(kind);
        self.suspended = false;
        warning
    }

    /// Takes the system's report of whether the window is `fullscreen`,
    /// and says whether that changed the state. Entering asks nothing of
    /// the window, which is already fullscreen, and counts as borderless;
    /// leaving gives back the size it had, however the player left, and
    /// keeps the chosen kind. A window left for a focus loss, or on the
    /// way out, is expected, and changes nothing.
    pub(crate) fn observed(&mut self, fullscreen: bool, window: &mut impl WindowPort) -> bool {
        if fullscreen == self.current.is_some() || self.suspended || self.exited {
            return false;
        }
        if fullscreen {
            self.current = Some(FullscreenKind::Borderless);
        } else {
            window.request_inner_size(self.windowed_size_px);
            self.current = None;
        }
        true
    }

    /// Takes the window's new size: while windowed, it is the size to give
    /// back on leaving. While fullscreen, or suspended from it, it is not
    /// the window's own, and ignored.
    pub(crate) fn resized(&mut self, size_px: (u32, u32)) {
        if self.current.is_none() {
            self.windowed_size_px = size_px;
        }
    }
}

/// Whether `window`'s monitor offers `mode`: it lists it, and is the
/// monitor named `monitor`, when both it and the saved kind have a name.
fn offered(window: &impl WindowPort, mode: &VideoMode, monitor: Option<&str>) -> bool {
    let same_monitor = match (monitor, window.monitor_name()) {
        (Some(saved), Some(now)) => saved == now,
        _ => true,
    };
    same_monitor && listed_modes(&window.video_modes()).contains(mode)
}

#[cfg(test)]
mod tests {
    use nova_view::{FullscreenKind, MonitorMode, VideoMode};

    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Call {
        SetFullscreen(Option<FullscreenKind>),
        RequestSize((u32, u32)),
    }

    const BORDERLESS: Call = Call::SetFullscreen(Some(FullscreenKind::Borderless));
    const WINDOWED: Call = Call::SetFullscreen(None);

    /// A window of settable size, on a monitor of settable modes and name,
    /// that records each fullscreen and size request, in order.
    struct RecordingWindow {
        size_px: (u32, u32),
        modes: Vec<MonitorMode>,
        monitor: Option<String>,
        calls: Vec<Call>,
    }

    impl RecordingWindow {
        fn new(size_px: (u32, u32)) -> Self {
            Self {
                size_px,
                modes: [fhd(), qhd()]
                    .map(|mode| MonitorMode {
                        mode,
                        bit_depth: 32,
                    })
                    .to_vec(),
                monitor: Some(MONITOR.to_owned()),
                calls: Vec::new(),
            }
        }
    }

    const MONITOR: &str = "DELL U2720Q";

    fn fhd() -> VideoMode {
        VideoMode {
            width: 1920,
            height: 1080,
            refresh_millihertz: 60_000,
        }
    }

    fn qhd() -> VideoMode {
        VideoMode {
            width: 2560,
            height: 1440,
            refresh_millihertz: 144_000,
        }
    }

    fn exclusive(mode: VideoMode, monitor: Option<&str>) -> FullscreenSetting {
        FullscreenSetting::Exclusive {
            mode,
            monitor: monitor.map(str::to_owned),
        }
    }

    fn entered(mode: VideoMode) -> Call {
        Call::SetFullscreen(Some(FullscreenKind::Exclusive(mode)))
    }

    /// Windowed in a 1024x768 window, its chosen kind `kind`.
    fn set_to(kind: FullscreenSetting) -> Fullscreen {
        let mut fullscreen = Fullscreen::new(Os::MacOs, (1024, 768));
        fullscreen.set_kind(kind);
        fullscreen
    }

    fn not_offered(mode: VideoMode, monitor: &str) -> String {
        format!(
            "nova: this monitor does not offer the saved display mode {} ({monitor}); \
             opening borderless fullscreen",
            mode.label()
        )
    }

    impl WindowPort for RecordingWindow {
        fn size_px(&self) -> (u32, u32) {
            self.size_px
        }

        fn scale_factor(&self) -> f64 {
            2.0
        }

        fn request_redraw(&mut self) {}

        fn set_fullscreen(&mut self, fullscreen: Option<FullscreenKind>) {
            self.calls.push(Call::SetFullscreen(fullscreen));
        }

        fn video_modes(&self) -> Vec<MonitorMode> {
            self.modes.clone()
        }

        fn monitor_name(&self) -> Option<String> {
            self.monitor.clone()
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
        let mut window = RecordingWindow::new((1024, 768));
        fullscreen.focus_lost(&mut window);
        assert_eq!(press(&mut fullscreen, Key::Char('f')), Shortcut::None);
        let mut fullscreen = Fullscreen::new(Os::Windows, (1024, 768));
        press(&mut fullscreen, Key::Alt);
        fullscreen.focus_lost(&mut window);
        assert_eq!(window.calls, [], "windowed");
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
        assert_eq!(fullscreen.toggle(&mut window), None);
        assert!(fullscreen.is_on());
        assert_eq!(window.calls, [BORDERLESS]);
        window.size_px = (3024, 1964);
        assert_eq!(fullscreen.toggle(&mut window), None);
        assert!(!fullscreen.is_on());
        assert_eq!(
            window.calls,
            [BORDERLESS, WINDOWED, Call::RequestSize((2048, 1536))]
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
        assert_eq!(window.calls, [WINDOWED, Call::RequestSize((1600, 1200))]);
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

    #[test]
    fn toggling_an_offered_exclusive_kind_enters_its_mode_and_leaves_to_the_window() {
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = set_to(exclusive(fhd(), Some(MONITOR)));
        assert_eq!(fullscreen.toggle(&mut window), None);
        assert!(fullscreen.is_on());
        assert_eq!(window.calls, [entered(fhd())]);
        assert_eq!(fullscreen.toggle(&mut window), None);
        assert!(!fullscreen.is_on());
        assert_eq!(
            window.calls,
            [entered(fhd()), WINDOWED, Call::RequestSize((1024, 768))]
        );
        assert_eq!(*fullscreen.kind(), exclusive(fhd(), Some(MONITOR)), "kept");
    }

    #[test]
    fn a_kind_the_monitor_does_not_offer_falls_back_to_borderless_once_with_a_warning() {
        let gone = VideoMode {
            refresh_millihertz: 59_940,
            ..fhd()
        };
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = set_to(exclusive(gone, Some(MONITOR)));
        assert_eq!(
            fullscreen.toggle(&mut window),
            Some(not_offered(gone, MONITOR)),
            "no such rate"
        );
        assert_eq!(window.calls, [BORDERLESS]);
        assert_eq!(
            *fullscreen.kind(),
            exclusive(gone, Some(MONITOR)),
            "the chosen kind is kept"
        );
        assert_eq!(fullscreen.live_kind(), FullscreenKind::Borderless, "runs");
        assert_eq!(
            set_to(exclusive(gone, Some(MONITOR))).live_kind(),
            FullscreenKind::Exclusive(gone),
            "not tried yet"
        );
        fullscreen.toggle(&mut window);
        assert_eq!(fullscreen.toggle(&mut window), None, "warned once");
        assert_eq!(window.calls[3], BORDERLESS);
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = set_to(exclusive(fhd(), Some("LG HDR 4K")));
        assert_eq!(
            fullscreen.toggle(&mut window),
            Some(not_offered(fhd(), "LG HDR 4K")),
            "another monitor"
        );
        assert_eq!(window.calls, [BORDERLESS]);
    }

    #[test]
    fn choosing_after_a_fallback_tries_the_new_kind_afresh() {
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = set_to(exclusive(fhd(), Some("LG HDR 4K")));
        assert!(fullscreen.toggle(&mut window).is_some());
        assert_eq!(
            fullscreen.choose(FullscreenSetting::Borderless, &mut window),
            None
        );
        assert_eq!(*fullscreen.kind(), FullscreenSetting::Borderless);
        assert_eq!(
            fullscreen.choose(exclusive(qhd(), Some(MONITOR)), &mut window),
            None
        );
        assert_eq!(window.calls, [BORDERLESS, BORDERLESS, entered(qhd())]);
        let mut fell_back = set_to(exclusive(fhd(), Some("LG HDR 4K")));
        fell_back.toggle(&mut window);
        fell_back.set_kind(exclusive(fhd(), Some(MONITOR)));
        fell_back.toggle(&mut window);
        assert_eq!(fell_back.toggle(&mut window), None, "a new kind, afresh");
        assert_eq!(window.calls[6..], [entered(fhd())]);
    }

    #[test]
    fn a_saved_kind_without_a_monitor_warns_without_naming_one() {
        let mut window = RecordingWindow::new((1024, 768));
        window.modes.clear();
        let mut fullscreen = set_to(exclusive(fhd(), None));
        assert_eq!(
            fullscreen.toggle(&mut window),
            Some(format!(
                "nova: this monitor does not offer the saved display mode {}; \
                 opening borderless fullscreen",
                fhd().label()
            ))
        );
    }

    #[test]
    fn a_mode_too_small_to_list_is_not_offered() {
        let small = VideoMode {
            width: 800,
            height: 600,
            refresh_millihertz: 60_000,
        };
        let mut window = RecordingWindow::new((1024, 768));
        window.modes.push(MonitorMode {
            mode: small,
            bit_depth: 32,
        });
        let mut fullscreen = set_to(exclusive(small, Some(MONITOR)));
        assert!(fullscreen.toggle(&mut window).is_some());
        assert_eq!(window.calls, [BORDERLESS]);
    }

    #[test]
    fn with_no_name_on_either_side_the_mode_alone_decides() {
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = set_to(exclusive(qhd(), None));
        assert_eq!(fullscreen.toggle(&mut window), None);
        assert_eq!(window.calls, [entered(qhd())]);
        let mut window = RecordingWindow::new((1024, 768));
        window.monitor = None;
        let mut fullscreen = set_to(exclusive(qhd(), Some("LG HDR 4K")));
        assert_eq!(fullscreen.toggle(&mut window), None);
        assert_eq!(window.calls, [entered(qhd())]);
    }

    #[test]
    fn choosing_a_kind_while_windowed_enters_it_remembering_the_size() {
        let mut window = RecordingWindow::new((1600, 1200));
        let mut fullscreen = Fullscreen::new(Os::MacOs, (1600, 1200));
        assert_eq!(
            fullscreen.choose(exclusive(qhd(), Some(MONITOR)), &mut window),
            None
        );
        assert!(fullscreen.is_on());
        assert_eq!(*fullscreen.kind(), exclusive(qhd(), Some(MONITOR)));
        fullscreen.toggle(&mut window);
        assert_eq!(
            window.calls,
            [entered(qhd()), WINDOWED, Call::RequestSize((1600, 1200))]
        );
    }

    #[test]
    fn choosing_another_kind_while_fullscreen_switches_to_it_alone() {
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = Fullscreen::new(Os::MacOs, (1024, 768));
        fullscreen.toggle(&mut window);
        fullscreen.resized((3024, 1964));
        window.calls.clear();
        assert_eq!(
            fullscreen.choose(exclusive(fhd(), Some(MONITOR)), &mut window),
            None
        );
        assert_eq!(window.calls, [entered(fhd())]);
        fullscreen.choose(FullscreenSetting::Borderless, &mut window);
        assert_eq!(window.calls, [entered(fhd()), BORDERLESS]);
        assert!(fullscreen.is_on());
        let gone = exclusive(fhd(), Some("LG HDR 4K"));
        assert_eq!(
            fullscreen.choose(gone, &mut window),
            Some(not_offered(fhd(), "LG HDR 4K"))
        );
        fullscreen.toggle(&mut window);
        assert_eq!(
            window.calls[2..],
            [BORDERLESS, WINDOWED, Call::RequestSize((1024, 768))]
        );
    }

    #[test]
    fn leaving_exclusive_through_the_system_clears_only_the_state() {
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = set_to(exclusive(fhd(), Some(MONITOR)));
        fullscreen.toggle(&mut window);
        assert!(fullscreen.observed(false, &mut window));
        assert!(!fullscreen.is_on());
        assert_eq!(*fullscreen.kind(), exclusive(fhd(), Some(MONITOR)));
        fullscreen.toggle(&mut window);
        assert_eq!(window.calls[2..], [entered(fhd())], "the kind again");
    }

    #[test]
    fn losing_focus_while_exclusive_leaves_its_mode_until_focus_comes_back() {
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = set_to(exclusive(fhd(), Some(MONITOR)));
        fullscreen.toggle(&mut window);
        window.calls.clear();
        fullscreen.focus_lost(&mut window);
        assert_eq!(window.calls, [WINDOWED]);
        assert!(!fullscreen.observed(false, &mut window), "expected");
        fullscreen.resized((1024, 768));
        fullscreen.resized((1280, 960));
        assert!(fullscreen.is_on(), "suspended, not left");
        assert_eq!(window.calls, [WINDOWED], "no size asked");
        fullscreen.focus_lost(&mut window);
        assert_eq!(window.calls, [WINDOWED], "once");
        fullscreen.focus_gained(&mut window);
        assert_eq!(window.calls, [WINDOWED, entered(fhd())]);
        fullscreen.focus_gained(&mut window);
        assert_eq!(window.calls.len(), 2, "resumed once");
        fullscreen.toggle(&mut window);
        assert_eq!(
            window.calls[2..],
            [WINDOWED, Call::RequestSize((1024, 768))],
            "the size from before it entered"
        );
    }

    #[test]
    fn focus_changes_while_windowed_or_borderless_ask_nothing() {
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = set_to(exclusive(fhd(), Some(MONITOR)));
        fullscreen.focus_lost(&mut window);
        fullscreen.focus_gained(&mut window);
        let mut borderless = Fullscreen::new(Os::MacOs, (1024, 768));
        borderless.toggle(&mut window);
        borderless.focus_lost(&mut window);
        borderless.focus_gained(&mut window);
        assert_eq!(window.calls, [BORDERLESS]);
        assert!(borderless.observed(false, &mut window), "a real exit");
    }

    #[test]
    fn exiting_leaves_an_exclusive_mode_and_then_ignores_the_system() {
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = set_to(exclusive(fhd(), Some(MONITOR)));
        fullscreen.toggle(&mut window);
        window.calls.clear();
        fullscreen.exiting(&mut window);
        assert_eq!(window.calls, [WINDOWED]);
        assert!(!fullscreen.observed(false, &mut window));
        assert!(fullscreen.is_on(), "kept as it was, for the save");
        assert_eq!(window.calls, [WINDOWED], "no size asked");
    }

    #[test]
    fn exiting_otherwise_asks_nothing() {
        let mut window = RecordingWindow::new((1024, 768));
        set_to(exclusive(fhd(), Some(MONITOR))).exiting(&mut window);
        let mut borderless = Fullscreen::new(Os::MacOs, (1024, 768));
        borderless.toggle(&mut window);
        borderless.exiting(&mut window);
        let mut suspended = set_to(exclusive(fhd(), Some(MONITOR)));
        suspended.toggle(&mut window);
        suspended.focus_lost(&mut window);
        suspended.exiting(&mut window);
        assert_eq!(window.calls, [BORDERLESS, entered(fhd()), WINDOWED]);
        suspended.focus_gained(&mut window);
        assert_eq!(window.calls.len(), 3, "no coming back after exiting");
    }

    #[test]
    fn the_system_entering_is_borderless_and_the_live_kind_is_kept() {
        let mut window = RecordingWindow::new((1024, 768));
        let mut fullscreen = set_to(exclusive(fhd(), Some(MONITOR)));
        assert!(fullscreen.observed(true, &mut window));
        fullscreen.focus_lost(&mut window);
        assert_eq!(window.calls, [], "not exclusive");
        assert_eq!(*fullscreen.kind(), exclusive(fhd(), Some(MONITOR)));
    }
}
