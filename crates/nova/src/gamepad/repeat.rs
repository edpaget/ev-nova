//! Repeating a held d-pad or stick direction, as a held key repeats:
//! [`Repeater`] follows what [`PadInputs`](super::inputs::PadInputs) tells
//! the screens and, given the time each frame takes, makes the repeats.
//!
//! The delay and the rate are the macOS keyboard's defaults
//! ([`REPEAT_DELAY`], [`REPEAT_INTERVAL`]), so a held stick walks a list
//! as fast as a held arrow key on an untouched Mac. The original has no
//! pad, so they follow no reading of it; the values and their source are
//! recorded in rdm:phase/key-bindings/phase-18-repeat-held-pad-directions's
//! findings.
//!
//! The rules, as the keyboard's where one applies:
//!
//! - A press of a button that repeats ([`PadButton::repeats`]) starts it
//!   repeating [`REPEAT_DELAY`] from then; only the direction pressed last
//!   repeats, as only the last key down does.
//! - A press of any other button stops it, as does the release of the
//!   repeating button or of any button that doesn't repeat (Start or
//!   Select, the only layers, let go mid-hold would otherwise change what
//!   the next repeat means). A release of another direction doesn't, so a
//!   stick rolled from down to down-right keeps repeating right.
//! - [`Repeater::advance`] makes one repeat for each deadline the time
//!   reaches, so a long frame gets every repeat it spans, as the OS queues
//!   key repeats.

use std::time::Duration;

use nova_view::{Input, PadButton};

/// How long a direction is held before it first repeats: the macOS
/// keyboard's default initial key-repeat delay, 500 ms.
pub const REPEAT_DELAY: Duration = Duration::from_millis(500);

/// How long between repeats after the first: the macOS keyboard's default
/// key-repeat interval, 83.33 ms (12 a second).
pub const REPEAT_INTERVAL: Duration = Duration::from_nanos(83_333_333);

/// The direction repeating, if any, and the time left until its next
/// repeat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Repeater {
    /// The button repeating and the time left until it next repeats.
    held: Option<(PadButton, Duration)>,
}

impl Repeater {
    /// Follows `input`, as the screens were told it: a pad button's press
    /// or release starts or stops the repeat by the rules above; a repeat,
    /// or any other input, changes nothing.
    pub fn follow(&mut self, input: &Input) {
        let Input::PadButton {
            button,
            pressed,
            repeat: false,
        } = *input
        else {
            return;
        };
        if pressed {
            self.held = button.repeats().then_some((button, REPEAT_DELAY));
        } else if !button.repeats() || self.held.is_some_and(|(held, _)| held == button) {
            self.held = None;
        }
    }

    /// Lets `dt` pass, appending to `out` a repeat of the direction held
    /// for each deadline it reaches.
    pub fn advance(&mut self, dt: Duration, out: &mut Vec<Input>) {
        let Some((button, left)) = &mut self.held else {
            return;
        };
        let mut dt = dt;
        while dt >= *left {
            dt -= *left;
            *left = REPEAT_INTERVAL;
            out.push(Input::PadButton {
                button: *button,
                pressed: true,
                repeat: true,
            });
        }
        *left -= dt;
    }

    /// Stops any repeat: the direction held repeats again only once it is
    /// pressed afresh.
    pub const fn stop(&mut self) {
        self.held = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    fn press(button: PadButton) -> Input {
        Input::PadButton {
            button,
            pressed: true,
            repeat: false,
        }
    }

    fn release(button: PadButton) -> Input {
        Input::PadButton {
            button,
            pressed: false,
            repeat: false,
        }
    }

    fn repeat(button: PadButton) -> Input {
        Input::PadButton {
            button,
            pressed: true,
            repeat: true,
        }
    }

    /// What `repeater` makes over `dt`.
    fn after(repeater: &mut Repeater, dt: Duration) -> Vec<Input> {
        let mut out = Vec::new();
        repeater.advance(dt, &mut out);
        out
    }

    /// A repeater holding `button`, pressed just now.
    fn holding(button: PadButton) -> Repeater {
        let mut repeater = Repeater::default();
        repeater.follow(&press(button));
        repeater
    }

    #[test]
    fn a_held_direction_repeats_once_the_delay_is_reached_then_at_the_rate() {
        let mut repeater = holding(PadButton::DPadDown);
        assert_eq!(after(&mut repeater, REPEAT_DELAY.saturating_sub(MS)), []);
        assert_eq!(after(&mut repeater, MS), [repeat(PadButton::DPadDown)]);
        assert_eq!(after(&mut repeater, REPEAT_INTERVAL.saturating_sub(MS)), []);
        assert_eq!(after(&mut repeater, MS), [repeat(PadButton::DPadDown)]);
        assert_eq!(
            after(&mut repeater, REPEAT_INTERVAL),
            [repeat(PadButton::DPadDown)]
        );
    }

    #[test]
    fn a_long_frame_sends_every_repeat_it_spans() {
        let mut repeater = holding(PadButton::LeftStickUp);
        let spanned = after(&mut repeater, REPEAT_DELAY + REPEAT_INTERVAL * 3 + MS);
        assert_eq!(spanned, [repeat(PadButton::LeftStickUp); 4]);
        assert_eq!(
            after(&mut repeater, REPEAT_INTERVAL.saturating_sub(MS)),
            [repeat(PadButton::LeftStickUp)],
            "the next deadline keeps the rate"
        );
    }

    #[test]
    fn a_release_stops_the_repeat() {
        let mut repeater = holding(PadButton::DPadLeft);
        assert_eq!(after(&mut repeater, REPEAT_DELAY).len(), 1);
        repeater.follow(&release(PadButton::DPadLeft));
        assert_eq!(after(&mut repeater, REPEAT_DELAY * 4), []);
    }

    #[test]
    fn only_the_direction_pressed_last_repeats() {
        let mut repeater = holding(PadButton::DPadUp);
        assert_eq!(after(&mut repeater, REPEAT_DELAY.saturating_sub(MS)), []);
        repeater.follow(&press(PadButton::DPadRight));
        assert_eq!(
            after(&mut repeater, REPEAT_DELAY.saturating_sub(MS)),
            [],
            "a fresh delay"
        );
        assert_eq!(after(&mut repeater, MS), [repeat(PadButton::DPadRight)]);
    }

    #[test]
    fn another_buttons_press_stops_the_repeat() {
        let mut repeater = holding(PadButton::DPadUp);
        repeater.follow(&press(PadButton::South));
        assert_eq!(after(&mut repeater, REPEAT_DELAY * 4), []);
    }

    #[test]
    fn a_non_repeating_buttons_release_stops_it_and_another_directions_does_not() {
        let mut repeater = holding(PadButton::LeftStickRight);
        repeater.follow(&release(PadButton::LeftStickDown));
        assert_eq!(
            after(&mut repeater, REPEAT_DELAY),
            [repeat(PadButton::LeftStickRight)],
            "a stick rolled on from down"
        );
        repeater.follow(&release(PadButton::Select));
        assert_eq!(
            after(&mut repeater, REPEAT_DELAY * 4),
            [],
            "the layer let go"
        );
    }

    #[test]
    fn a_repeat_followed_back_keeps_the_rate() {
        let mut repeater = holding(PadButton::DPadDown);
        let made = after(&mut repeater, REPEAT_DELAY);
        for input in &made {
            repeater.follow(input);
        }
        assert_eq!(
            after(&mut repeater, REPEAT_INTERVAL),
            [repeat(PadButton::DPadDown)],
            "its own repeat restarts no delay"
        );
    }

    #[test]
    fn buttons_that_dont_repeat_never_repeat() {
        for button in PadButton::ALL.into_iter().filter(|b| !b.repeats()) {
            let mut repeater = holding(button);
            assert_eq!(
                after(&mut repeater, Duration::from_secs(2)),
                [],
                "{button:?}"
            );
        }
    }

    #[test]
    fn stop_forgets_the_direction_held() {
        let mut repeater = holding(PadButton::RightStickDown);
        repeater.stop();
        assert_eq!(after(&mut repeater, REPEAT_DELAY * 4), []);
    }

    #[test]
    fn other_inputs_change_nothing() {
        let mut repeater = holding(PadButton::DPadUp);
        repeater.follow(&Input::PointerLeft);
        assert_eq!(
            after(&mut repeater, REPEAT_DELAY),
            [repeat(PadButton::DPadUp)]
        );
    }
}
