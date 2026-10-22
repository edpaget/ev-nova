//! The main screen's button tracking, as `_TrackMainScreenButtons`
//! (@0x195b9) tracks a press that `_MainScreenMouse`'s content part
//! (@0x1ae3a) hands it.
//!
//! It differs from the dialogs' [`ButtonTracker`](crate::ui::ButtonTracker),
//! so it is the main screen's own:
//!
//! - A press that hits no button is a miss: silent, nothing tracked and
//!   no choice (the tracker returns -1 @0x19650, and choice -1 only
//!   redraws).
//! - A press on a button plays snd 600 (@0x19687) and draws it pressed.
//! - While the press is held, each change of what is under the pointer
//!   plays a sound: onto a button 600 (@0x19974), straight from one button
//!   onto another included; off every button 601 (@0x19ac4).
//! - Letting go over a button plays 601 (@0x19c38) and chooses it;
//!   letting go off every button is silent and chooses nothing.
//!
//! The original polls the pointer's place before it reads the button
//! (@0x1985b-0x19942, then `Button` @0x19c01), so a release away from
//! where the pointer last moved first counts as a move there.

use crate::sound::UiSound;

/// What one tracking step did: the sound it raised and the button it
/// chose, either or both, if any.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tracked {
    /// The sounds raised, in order.
    pub sounds: [Option<UiSound>; 2],
    /// The button chosen, by index.
    pub chosen: Option<usize>,
}

impl Tracked {
    fn sound(sound: Option<UiSound>) -> Self {
        Self {
            sounds: [sound, None],
            chosen: None,
        }
    }

    /// The sounds raised, in order.
    pub fn sounds(&self) -> impl Iterator<Item = UiSound> + '_ {
        self.sounds.iter().flatten().copied()
    }
}

/// A press on the main screen's buttons, tracked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MainScreenTracker {
    /// Whether a press is tracked.
    held: bool,
    /// While it is, the button under the pointer, if any.
    under: Option<usize>,
}

impl MainScreenTracker {
    /// A press with `hit` under the pointer: a button starts a tracked
    /// press with 600; none is a silent miss.
    pub fn press(&mut self, hit: Option<usize>) -> Tracked {
        self.held = hit.is_some();
        self.under = hit;
        Tracked::sound(hit.map(|_| UiSound::ButtonDown))
    }

    /// The pointer moved, with `hit` under it: while a press is tracked,
    /// 600 onto a button and 601 off every one, each time what is under
    /// it changes.
    pub fn moved(&mut self, hit: Option<usize>) -> Tracked {
        if !self.held || self.under == hit {
            return Tracked::default();
        }
        self.under = hit;
        Tracked::sound(Some(if hit.is_some() {
            UiSound::ButtonDown
        } else {
            UiSound::ButtonUp
        }))
    }

    /// The press let go of with `hit` under the pointer, which first
    /// counts as a move there: over a button, 601 and its choice; off
    /// every button, silence.
    pub fn release(&mut self, hit: Option<usize>) -> Tracked {
        let moved = self.moved(hit);
        self.held = false;
        let Some(chosen) = self.under.take() else {
            return moved;
        };
        Tracked {
            sounds: [moved.sounds[0], Some(UiSound::ButtonUp)],
            chosen: Some(chosen),
        }
    }

    /// Abandons the press silently.
    pub fn cancel(&mut self) {
        self.held = false;
        self.under = None;
    }

    /// Whether a press is tracked, on a button or off them.
    #[must_use]
    pub fn tracking(&self) -> bool {
        self.held
    }

    /// The button drawn pressed: the one under the pointer while a press
    /// is tracked.
    #[must_use]
    pub fn pressed(&self) -> Option<usize> {
        self.under
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sounds(tracked: Tracked) -> Vec<UiSound> {
        tracked.sounds().collect()
    }

    const DOWN: UiSound = UiSound::ButtonDown;
    const UP: UiSound = UiSound::ButtonUp;

    #[test]
    fn a_press_on_nothing_is_a_silent_miss_that_chooses_nothing() {
        let mut tracker = MainScreenTracker::default();
        assert_eq!(tracker.press(None), Tracked::default());
        assert!(!tracker.tracking());
        assert_eq!(tracker.pressed(), None);
        assert_eq!(tracker.moved(Some(2)), Tracked::default(), "not tracked");
        assert_eq!(tracker.release(Some(2)), Tracked::default());
    }

    #[test]
    fn a_press_on_a_button_plays_600_and_draws_it_pressed() {
        let mut tracker = MainScreenTracker::default();
        assert_eq!(sounds(tracker.press(Some(3))), [DOWN]);
        assert!(tracker.tracking());
        assert_eq!(tracker.pressed(), Some(3));
    }

    #[test]
    fn dragging_off_plays_601_and_back_on_600() {
        let mut tracker = MainScreenTracker::default();
        tracker.press(Some(1));
        assert_eq!(sounds(tracker.moved(Some(1))), [], "no change");
        assert_eq!(sounds(tracker.moved(None)), [UP]);
        assert!(tracker.tracking(), "still held");
        assert_eq!(tracker.pressed(), None);
        assert_eq!(sounds(tracker.moved(None)), [], "no change");
        assert_eq!(sounds(tracker.moved(Some(1))), [DOWN]);
        assert_eq!(tracker.pressed(), Some(1));
    }

    #[test]
    fn from_one_button_straight_onto_another_plays_600_alone() {
        let mut tracker = MainScreenTracker::default();
        tracker.press(Some(0));
        assert_eq!(sounds(tracker.moved(Some(4))), [DOWN]);
        assert_eq!(tracker.pressed(), Some(4));
    }

    #[test]
    fn letting_go_over_a_button_plays_601_and_chooses_it() {
        let mut tracker = MainScreenTracker::default();
        tracker.press(Some(2));
        let released = tracker.release(Some(2));
        assert_eq!(sounds(released), [UP]);
        assert_eq!(released.chosen, Some(2));
        assert!(!tracker.tracking());
        assert_eq!(tracker.pressed(), None);
    }

    #[test]
    fn letting_go_off_every_button_is_silent_and_chooses_nothing() {
        let mut tracker = MainScreenTracker::default();
        tracker.press(Some(2));
        tracker.moved(None);
        assert_eq!(tracker.release(None), Tracked::default());
        assert!(!tracker.tracking());
    }

    #[test]
    fn a_release_away_from_the_last_move_first_moves_there() {
        let mut tracker = MainScreenTracker::default();
        tracker.press(Some(0));
        let released = tracker.release(Some(5));
        assert_eq!(sounds(released), [DOWN, UP]);
        assert_eq!(released.chosen, Some(5));
        let mut tracker = MainScreenTracker::default();
        tracker.press(Some(0));
        let released = tracker.release(None);
        assert_eq!(sounds(released), [UP], "dragged off, then let go");
        assert_eq!(released.chosen, None);
    }

    #[test]
    fn a_cancel_abandons_the_press_silently() {
        let mut tracker = MainScreenTracker::default();
        tracker.press(Some(1));
        tracker.cancel();
        assert!(!tracker.tracking());
        assert_eq!(tracker.pressed(), None);
        assert_eq!(tracker.release(Some(1)), Tracked::default());
    }
}
