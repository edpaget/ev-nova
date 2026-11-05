//! What the pads' events mean for the screens: [`PadInputs`] turns each
//! poll's [`PadEvent`]s into `nova_view` [`Input`]s.
//!
//! - A button passes through as [`Input::PadButton`].
//! - A stick axis passes its value through as [`Input::PadAxis`] (flight
//!   reads its analog amounts from it), and presses its direction's
//!   button ([`PadAxis::direction`]) once its value reaches
//!   [`PRESS_THRESHOLD`] that way, letting go once it drops under
//!   [`RELEASE_THRESHOLD`]. Between the two it stays as it was, so a
//!   stick resting at the threshold doesn't chatter.
//! - A trigger's value presses and lets go of its button the same way.
//! - Pads merge: a button is down while any pad holds it, and reaches
//!   the screens only as the first pad presses it and the last lets go.
//! - A lost pad lets go of everything it held and sends none for each
//!   stick axis, as the router's `release_keys` lets go on a focus loss.
//!
//! The thresholds and their reasons are recorded in
//! rdm:phase/key-bindings/phase-7-gamepad-input's findings.

use std::collections::{BTreeMap, BTreeSet};

use nova_view::{Input, PadAxis, PadButton};

use super::{PadEvent, PadId};

/// Where a stick direction or a trigger presses its button: halfway, the
/// common default.
pub const PRESS_THRESHOLD: f32 = 0.5;

/// Under which a pressed stick direction or trigger lets go: the 0.1 below
/// [`PRESS_THRESHOLD`] is the hysteresis.
pub const RELEASE_THRESHOLD: f32 = 0.4;

/// One pad's buttons down, those its values press among them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Pad {
    /// Every button down on it.
    down: BTreeSet<PadButton>,
}

/// The pads' state, merged into one set of buttons down.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PadInputs {
    /// Each connected pad.
    pads: BTreeMap<PadId, Pad>,
}

impl PadInputs {
    /// A state with no pad connected.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Follows `events`, in order, appending what each means for the
    /// screens to `out`.
    pub fn feed(&mut self, events: &[PadEvent], out: &mut Vec<Input>) {
        for event in events {
            self.follow(event, out);
        }
    }

    /// Whether any pad holds `button` down.
    #[must_use]
    pub fn is_down(&self, button: PadButton) -> bool {
        self.pads.values().any(|pad| pad.down.contains(&button))
    }

    /// Follows one event.
    fn follow(&mut self, event: &PadEvent, out: &mut Vec<Input>) {
        match *event {
            PadEvent::Connected(pad) => {
                self.pads.entry(pad).or_default();
            }
            PadEvent::Disconnected(pad) => self.lose(pad, out),
            PadEvent::Button {
                pad,
                button,
                pressed,
            } => self.set(pad, button, pressed, out),
            PadEvent::Axis { pad, axis, value } => {
                out.push(Input::PadAxis { axis, value });
                for positive in [true, false] {
                    let along = if positive { value } else { -value };
                    self.threshold(pad, axis.direction(positive), along, out);
                }
            }
            PadEvent::Trigger { pad, button, value } => {
                self.threshold(pad, button, value, out);
            }
        }
    }

    /// Presses `button` on `pad` at `value` past [`PRESS_THRESHOLD`], lets
    /// it go under [`RELEASE_THRESHOLD`], and leaves it between.
    fn threshold(&mut self, pad: PadId, button: PadButton, value: f32, out: &mut Vec<Input>) {
        if value >= PRESS_THRESHOLD {
            self.set(pad, button, true, out);
        } else if value < RELEASE_THRESHOLD {
            self.set(pad, button, false, out);
        }
    }

    /// Sets `button` down or up on `pad`, telling the screens only of a
    /// change to the merged set.
    fn set(&mut self, pad: PadId, button: PadButton, pressed: bool, out: &mut Vec<Input>) {
        let was = self.is_down(button);
        let held = &mut self.pads.entry(pad).or_default().down;
        if pressed {
            held.insert(button);
        } else {
            held.remove(&button);
        }
        if self.is_down(button) != was {
            out.push(Input::PadButton { button, pressed });
        }
    }

    /// Forgets `pad`: lets go of each button it held that no other pad
    /// holds, and sends none for each stick axis.
    fn lose(&mut self, pad: PadId, out: &mut Vec<Input>) {
        let Some(lost) = self.pads.remove(&pad) else {
            return;
        };
        for button in lost.down {
            if !self.is_down(button) {
                out.push(Input::PadButton {
                    button,
                    pressed: false,
                });
            }
        }
        for axis in PadAxis::ALL {
            out.push(Input::PadAxis { axis, value: 0.0 });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Gamepads;
    use super::super::fixture::FakeGamepads;
    use super::*;

    const ONE: PadId = PadId(0);
    const TWO: PadId = PadId(1);

    /// What each poll of `fake` means, one list a poll.
    fn polled(fake: &FakeGamepads, inputs: &mut PadInputs, polls: usize) -> Vec<Vec<Input>> {
        let mut port = fake.clone();
        (0..polls)
            .map(|_| {
                let mut events = Vec::new();
                port.poll(&mut events);
                let mut out = Vec::new();
                inputs.feed(&events, &mut out);
                out
            })
            .collect()
    }

    fn axis(pad: PadId, axis: PadAxis, value: f32) -> PadEvent {
        PadEvent::Axis { pad, axis, value }
    }

    fn pressed(button: PadButton, pressed: bool) -> Input {
        Input::PadButton { button, pressed }
    }

    fn value(axis: PadAxis, value: f32) -> Input {
        Input::PadAxis { axis, value }
    }

    /// The button inputs in `inputs`.
    fn buttons(inputs: &[Input]) -> Vec<Input> {
        inputs
            .iter()
            .copied()
            .filter(|input| matches!(input, Input::PadButton { .. }))
            .collect()
    }

    #[test]
    fn the_thresholds_are_the_findings() {
        assert!((PRESS_THRESHOLD - 0.5).abs() < f32::EPSILON);
        assert!((RELEASE_THRESHOLD - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn an_axis_past_the_threshold_presses_its_stick_direction() {
        let fake = FakeGamepads::with([
            vec![PadEvent::Connected(ONE), axis(ONE, PadAxis::LeftY, -0.49)],
            vec![axis(ONE, PadAxis::LeftY, -0.5)],
            vec![axis(ONE, PadAxis::RightX, 0.9)],
        ]);
        let mut inputs = PadInputs::new();
        let polls = polled(&fake, &mut inputs, 3);
        assert_eq!(buttons(&polls[0]), [], "short of the threshold");
        assert_eq!(
            buttons(&polls[1]),
            [pressed(PadButton::LeftStickDown, true)]
        );
        assert_eq!(
            buttons(&polls[2]),
            [pressed(PadButton::RightStickRight, true)]
        );
        assert!(inputs.is_down(PadButton::LeftStickDown));
    }

    #[test]
    fn back_under_the_release_threshold_releases_it() {
        let fake = FakeGamepads::with([
            vec![PadEvent::Connected(ONE), axis(ONE, PadAxis::LeftX, 0.7)],
            vec![axis(ONE, PadAxis::LeftX, 0.39)],
        ]);
        let mut inputs = PadInputs::new();
        let polls = polled(&fake, &mut inputs, 2);
        assert_eq!(
            buttons(&polls[0]),
            [pressed(PadButton::LeftStickRight, true)]
        );
        assert_eq!(
            buttons(&polls[1]),
            [pressed(PadButton::LeftStickRight, false)]
        );
    }

    #[test]
    fn between_the_two_thresholds_it_stays_as_it_was() {
        let fake = FakeGamepads::with([
            vec![PadEvent::Connected(ONE), axis(ONE, PadAxis::RightY, 0.45)],
            vec![axis(ONE, PadAxis::RightY, 0.5)],
            vec![axis(ONE, PadAxis::RightY, 0.45)],
            vec![axis(ONE, PadAxis::RightY, 0.4)],
            vec![axis(ONE, PadAxis::RightY, 0.5)],
            vec![axis(ONE, PadAxis::RightY, 0.399)],
        ]);
        let mut inputs = PadInputs::new();
        let polls = polled(&fake, &mut inputs, 6);
        let up = |down| vec![pressed(PadButton::RightStickUp, down)];
        assert_eq!(buttons(&polls[0]), [], "rising, not yet pressed");
        assert_eq!(buttons(&polls[1]), up(true));
        assert_eq!(buttons(&polls[2]), [], "falling, still pressed");
        assert_eq!(buttons(&polls[3]), [], "at the release threshold");
        assert_eq!(buttons(&polls[4]), [], "pressed already");
        assert_eq!(buttons(&polls[5]), up(false));
    }

    #[test]
    fn a_stick_swung_across_lets_go_of_one_way_and_presses_the_other() {
        let fake = FakeGamepads::with([
            vec![PadEvent::Connected(ONE), axis(ONE, PadAxis::LeftX, -1.0)],
            vec![axis(ONE, PadAxis::LeftX, 1.0)],
        ]);
        let mut inputs = PadInputs::new();
        let polls = polled(&fake, &mut inputs, 2);
        assert_eq!(
            buttons(&polls[0]),
            [pressed(PadButton::LeftStickLeft, true)]
        );
        assert_eq!(
            buttons(&polls[1]),
            [
                pressed(PadButton::LeftStickRight, true),
                pressed(PadButton::LeftStickLeft, false)
            ]
        );
    }

    #[test]
    fn a_trigger_past_the_threshold_presses_its_button() {
        let trigger = |value| PadEvent::Trigger {
            pad: ONE,
            button: PadButton::RightTrigger,
            value,
        };
        let fake = FakeGamepads::with([
            vec![PadEvent::Connected(ONE), trigger(0.3)],
            vec![trigger(0.5)],
            vec![trigger(0.45)],
            vec![trigger(0.1)],
        ]);
        let mut inputs = PadInputs::new();
        let polls = polled(&fake, &mut inputs, 4);
        assert_eq!(polls[0], []);
        assert_eq!(polls[1], [pressed(PadButton::RightTrigger, true)]);
        assert_eq!(polls[2], [], "the hysteresis");
        assert_eq!(polls[3], [pressed(PadButton::RightTrigger, false)]);
    }

    #[test]
    fn an_axis_emits_its_value_for_the_view() {
        let fake = FakeGamepads::with([vec![
            PadEvent::Connected(ONE),
            axis(ONE, PadAxis::RightX, 0.25),
            axis(ONE, PadAxis::LeftY, -0.1),
        ]]);
        let mut inputs = PadInputs::new();
        let polls = polled(&fake, &mut inputs, 1);
        assert_eq!(
            polls[0],
            [value(PadAxis::RightX, 0.25), value(PadAxis::LeftY, -0.1)]
        );
    }

    #[test]
    fn buttons_pass_through() {
        let fake = FakeGamepads::with([vec![
            PadEvent::Connected(ONE),
            PadEvent::Button {
                pad: ONE,
                button: PadButton::South,
                pressed: true,
            },
            PadEvent::Button {
                pad: ONE,
                button: PadButton::South,
                pressed: false,
            },
        ]]);
        let mut inputs = PadInputs::new();
        assert_eq!(
            polled(&fake, &mut inputs, 1)[0],
            [
                pressed(PadButton::South, true),
                pressed(PadButton::South, false)
            ]
        );
    }

    #[test]
    fn losing_a_pad_releases_every_button_and_direction_it_held_and_zeroes_its_axes() {
        let fake = FakeGamepads::with([
            vec![
                PadEvent::Connected(ONE),
                PadEvent::Button {
                    pad: ONE,
                    button: PadButton::DPadUp,
                    pressed: true,
                },
                axis(ONE, PadAxis::RightX, 0.8),
                PadEvent::Trigger {
                    pad: ONE,
                    button: PadButton::LeftTrigger,
                    value: 1.0,
                },
            ],
            vec![PadEvent::Disconnected(ONE)],
        ]);
        let mut inputs = PadInputs::new();
        let polls = polled(&fake, &mut inputs, 2);
        let mut released = buttons(&polls[1]);
        released.sort_by_key(|input| format!("{input:?}"));
        let mut expected = vec![
            pressed(PadButton::DPadUp, false),
            pressed(PadButton::RightStickRight, false),
            pressed(PadButton::LeftTrigger, false),
        ];
        expected.sort_by_key(|input| format!("{input:?}"));
        assert_eq!(released, expected);
        for each in PadAxis::ALL {
            assert!(polls[1].contains(&value(each, 0.0)), "{each:?}");
        }
        assert!(!inputs.is_down(PadButton::DPadUp));
    }

    #[test]
    fn a_pad_connecting_later_starts_working() {
        let fake = FakeGamepads::with([
            vec![],
            vec![],
            vec![PadEvent::Connected(TWO), axis(TWO, PadAxis::RightX, 0.5)],
        ]);
        let mut inputs = PadInputs::new();
        let polls = polled(&fake, &mut inputs, 3);
        assert_eq!(
            polls[2],
            [
                value(PadAxis::RightX, 0.5),
                pressed(PadButton::RightStickRight, true)
            ]
        );
    }

    #[test]
    fn two_pads_merge_a_button_down_while_either_holds_it() {
        let south = |pad, pressed| PadEvent::Button {
            pad,
            button: PadButton::South,
            pressed,
        };
        let fake = FakeGamepads::with([
            vec![
                PadEvent::Connected(ONE),
                PadEvent::Connected(TWO),
                south(ONE, true),
                south(TWO, true),
            ],
            vec![south(ONE, false)],
            vec![PadEvent::Disconnected(TWO)],
        ]);
        let mut inputs = PadInputs::new();
        let polls = polled(&fake, &mut inputs, 3);
        assert_eq!(polls[0], [pressed(PadButton::South, true)], "once");
        assert_eq!(polls[1], [], "the other still holds it");
        assert_eq!(buttons(&polls[2]), [pressed(PadButton::South, false)]);
    }

    #[test]
    fn losing_an_unknown_pad_changes_nothing() {
        let mut inputs = PadInputs::new();
        let mut out = Vec::new();
        inputs.feed(&[PadEvent::Disconnected(ONE)], &mut out);
        assert_eq!(out, []);
    }
}
