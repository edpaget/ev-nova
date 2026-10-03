//! The overlay model: whether the developer tools show, where input goes,
//! and the frame times measured from injected elapsed time.

use std::time::Duration;

use super::frame_time::FrameTimes;
use crate::input::Key;

/// The key that shows and hides the overlay: backquote, the key under
/// Escape on a US keyboard.
pub const TOGGLE_KEY: Key = Key::Char('`');

/// Where one input event goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Routing {
    /// To the game's screen: the overlay is hidden.
    Game,
    /// To the overlay alone: it shows, and takes every input.
    Overlay,
    /// Nowhere: it is the toggle key's own press, repeat or release.
    Toggle,
}

/// The developer overlay: whether it shows, and the frame times.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DevOverlay {
    visible: bool,
    frame_times: FrameTimes,
    previous: Option<Duration>,
}

impl DevOverlay {
    /// Hidden, with no frame samples.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the overlay shows.
    #[must_use]
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// The intervals between recent redraws.
    #[must_use]
    pub fn frame_times(&self) -> &FrameTimes {
        &self.frame_times
    }

    /// Routes a key event. A press of [`TOGGLE_KEY`] that is not a repeat
    /// shows or hides the overlay; every event of that key is
    /// [`Routing::Toggle`]. Any other key goes to the overlay while it
    /// shows and to the game otherwise.
    pub fn key(&mut self, key: Key, pressed: bool, repeat: bool) -> Routing {
        if key == TOGGLE_KEY {
            if pressed && !repeat {
                self.visible = !self.visible;
            }
            Routing::Toggle
        } else {
            self.pointer()
        }
    }

    /// Routes a pointer event: to the overlay while it shows, to the game
    /// otherwise.
    #[must_use]
    pub fn pointer(&self) -> Routing {
        if self.visible {
            Routing::Overlay
        } else {
            Routing::Game
        }
    }

    /// Records a redraw at `elapsed`, the time since the app started. The
    /// first sets a baseline; each later one records the time since the
    /// previous. A clock that seems to run backwards records zero and
    /// keeps the later baseline.
    pub fn frame(&mut self, elapsed: Duration) {
        if let Some(previous) = self.previous {
            self.frame_times.record(elapsed.saturating_sub(previous));
        }
        self.previous = Some(self.previous.map_or(elapsed, |p| p.max(elapsed)));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn it_starts_hidden_with_no_frame_samples() {
        let overlay = DevOverlay::new();
        assert!(!overlay.visible());
        assert_eq!(overlay.frame_times().count(), 0);
        assert_eq!(DevOverlay::default(), overlay);
    }

    #[test]
    fn the_toggle_key_shows_and_hides_it() {
        let mut overlay = DevOverlay::new();
        assert_eq!(overlay.key(TOGGLE_KEY, true, false), Routing::Toggle);
        assert!(overlay.visible());
        assert_eq!(overlay.key(TOGGLE_KEY, true, false), Routing::Toggle);
        assert!(!overlay.visible());
    }

    #[test]
    fn the_toggle_key_is_backquote() {
        assert_eq!(TOGGLE_KEY, Key::Char('`'));
    }

    #[test]
    fn a_repeat_of_the_toggle_key_toggles_nothing() {
        let mut overlay = DevOverlay::new();
        assert_eq!(overlay.key(TOGGLE_KEY, true, true), Routing::Toggle);
        assert!(!overlay.visible());
        overlay.key(TOGGLE_KEY, true, false);
        assert_eq!(overlay.key(TOGGLE_KEY, true, true), Routing::Toggle);
        assert!(overlay.visible());
    }

    #[test]
    fn releasing_the_toggle_key_toggles_nothing() {
        let mut overlay = DevOverlay::new();
        assert_eq!(overlay.key(TOGGLE_KEY, false, false), Routing::Toggle);
        assert!(!overlay.visible());
        overlay.key(TOGGLE_KEY, true, false);
        assert_eq!(overlay.key(TOGGLE_KEY, false, false), Routing::Toggle);
        assert!(overlay.visible());
    }

    #[test]
    fn other_keys_go_to_the_game_when_hidden_and_the_overlay_when_visible() {
        let keys = [
            Key::Escape,
            Key::Tab,
            Key::Enter,
            Key::Space,
            Key::Left,
            Key::Right,
            Key::Up,
            Key::Down,
            Key::Char('d'),
        ];
        let mut overlay = DevOverlay::new();
        for key in keys {
            for (pressed, repeat) in [(true, false), (true, true), (false, false)] {
                assert_eq!(overlay.key(key, pressed, repeat), Routing::Game, "{key:?}");
            }
        }
        assert!(!overlay.visible());
        overlay.key(TOGGLE_KEY, true, false);
        for key in keys {
            for (pressed, repeat) in [(true, false), (true, true), (false, false)] {
                assert_eq!(
                    overlay.key(key, pressed, repeat),
                    Routing::Overlay,
                    "{key:?}"
                );
            }
        }
        assert!(overlay.visible());
    }

    #[test]
    fn the_pointer_goes_to_the_game_when_hidden_and_the_overlay_when_visible() {
        let mut overlay = DevOverlay::new();
        assert_eq!(overlay.pointer(), Routing::Game);
        overlay.key(TOGGLE_KEY, true, false);
        assert_eq!(overlay.pointer(), Routing::Overlay);
    }

    #[test]
    fn frames_record_the_time_between_redraws() {
        let mut overlay = DevOverlay::new();
        overlay.frame(ms(0));
        assert_eq!(
            overlay.frame_times().count(),
            0,
            "the first sets a baseline"
        );
        overlay.frame(ms(16));
        overlay.frame(ms(33));
        assert_eq!(overlay.frame_times().count(), 2);
        assert_eq!(overlay.frame_times().last(), Some(ms(17)));
        assert_eq!(
            overlay.frame_times().mean(),
            Some(Duration::from_micros(16_500))
        );
    }

    #[test]
    fn the_first_frame_is_a_baseline_even_late_in_the_run() {
        let mut overlay = DevOverlay::new();
        overlay.frame(ms(1000));
        overlay.frame(ms(1016));
        assert_eq!(overlay.frame_times().count(), 1);
        assert_eq!(overlay.frame_times().last(), Some(ms(16)));
    }

    #[test]
    fn a_clock_that_goes_back_records_zero_and_keeps_its_baseline() {
        let mut overlay = DevOverlay::new();
        overlay.frame(ms(100));
        overlay.frame(ms(90));
        assert_eq!(overlay.frame_times().last(), Some(Duration::ZERO));
        overlay.frame(ms(120));
        assert_eq!(overlay.frame_times().last(), Some(ms(20)));
        assert_eq!(overlay.frame_times().count(), 2);
    }
}
