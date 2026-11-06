//! What the pads' events mean for the screens: [`PadInputs`] turns each
//! poll's [`PadEvent`]s into `nova_view` [`Input`]s.
//!
//! - A button passes through as [`Input::PadButton`].
//! - A stick axis passes its value through as [`Input::PadAxis`] (flight
//!   reads its analog amounts from it), and presses its direction's
//!   button ([`PadAxis::direction`]) once its value reaches
//!   [`PRESS_THRESHOLD`](nova_view::keymap::gamepad::PRESS_THRESHOLD) that
//!   way, letting go once it drops under
//!   [`RELEASE_THRESHOLD`](nova_view::keymap::gamepad::RELEASE_THRESHOLD). Between the two it stays as it was, so a
//!   stick resting at the threshold doesn't chatter.
//! - A trigger's value presses and lets go of its button the same way.
//! - Pads merge: a button (a trigger's among them) is down while any pad
//!   holds it, and reaches the screens only as the first pad presses it
//!   and the last lets go. A stick axis routes the value of the pad
//!   pushing it furthest from the centre, ties going to the lower
//!   [`PadId`], so an idle pad's jitter (a second controller, or Steam's
//!   virtual pad beside the real one) never overrides the pad in use.
//!   Its directions press off that merged value, so the directions held
//!   always agree with the value routed.
//! - A lost pad lets go of every button it held that no other pad holds,
//!   and routes each stick axis again over the pads left, none when no
//!   other pad pushes it, as the router's `release_keys` lets go on a
//!   focus loss.
//! - A held d-pad or stick direction repeats, as a held key does
//!   ([`super::repeat`]): [`PadInputs::advance`] makes the repeats for the
//!   time a frame takes, and every button input `feed` makes starts or
//!   stops them. The app advances before it feeds the frame's events, so
//!   time held before this poll yields its repeats and a release in it
//!   then stops them; a press in it starts its delay from now. When in a
//!   frame a repeat falls is invisible to the player: it reaches the
//!   screen before the frame is drawn. [`PadInputs::drain`] (the window
//!   in the background) stops them, so only a fresh push repeats again.
//!
//! The thresholds and the rule between them ([`crossing`]) are
//! `nova_view`'s (`nova_view::keymap::gamepad`), which flight's touch stick
//! reads too; their reasons are recorded in
//! rdm:phase/key-bindings/phase-7-gamepad-input's findings.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use nova_view::keymap::gamepad::crossing;
use nova_view::{Input, PadAxis, PadButton};

use super::repeat::Repeater;
use super::{PadEvent, PadId};

/// One pad's buttons down and the last value of each of its stick axes.
#[derive(Clone, Debug, Default, PartialEq)]
struct Pad {
    /// Every button down on it, its triggers' among them. Its stick
    /// directions aren't: they press off the merged sticks.
    down: BTreeSet<PadButton>,
    /// The last value it reported for each stick axis.
    axes: BTreeMap<PadAxis, f32>,
}

/// The pads' state, merged into one set of buttons down and one value a
/// stick axis.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PadInputs {
    /// Each connected pad.
    pads: BTreeMap<PadId, Pad>,
    /// The stick directions the merged sticks press.
    directions: BTreeSet<PadButton>,
    /// The direction repeating, following the button inputs `feed` makes.
    repeater: Repeater,
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
            let from = out.len();
            self.follow(event, out);
            for input in &out[from..] {
                self.repeater.follow(input);
            }
        }
    }

    /// Lets `dt` pass, appending to `out` the repeats of the direction
    /// held that it reaches ([`Repeater::advance`]).
    pub fn advance(&mut self, dt: Duration, out: &mut Vec<Input>) {
        self.repeater.advance(dt, out);
    }

    /// Follows `events` as [`PadInputs::feed`] does, telling no screen,
    /// and stops any repeat: a direction held across, or pressed in, the
    /// time the events were queued repeats only once pushed afresh.
    pub fn drain(&mut self, events: &[PadEvent]) {
        self.feed(events, &mut Vec::new());
        self.stop_repeating();
    }

    /// Stops any repeat ([`Repeater::stop`]).
    pub const fn stop_repeating(&mut self) {
        self.repeater.stop();
    }

    /// Whether any pad holds `button` down, or the merged sticks press it.
    #[must_use]
    pub fn is_down(&self, button: PadButton) -> bool {
        self.directions.contains(&button)
            || self.pads.values().any(|pad| pad.down.contains(&button))
    }

    /// Follows one event.
    fn follow(&mut self, event: &PadEvent, out: &mut Vec<Input>) {
        match *event {
            PadEvent::Connected { pad, .. } => {
                self.pads.entry(pad).or_default();
            }
            PadEvent::Disconnected(pad) => self.lose(pad, out),
            PadEvent::Button {
                pad,
                button,
                pressed,
            } => self.set(pad, button, pressed, out),
            PadEvent::Axis { pad, axis, value } => {
                self.pads.entry(pad).or_default().axes.insert(axis, value);
                self.route(axis, out);
            }
            PadEvent::Trigger { pad, button, value } => {
                if let Some(pressed) = crossing(value) {
                    self.set(pad, button, pressed, out);
                }
            }
        }
    }

    /// The value of `axis` on the pad pushing it furthest, the lower
    /// [`PadId`] on a tie; none when no pad pushes it.
    fn merged(&self, axis: PadAxis) -> f32 {
        self.pads
            .values()
            .filter_map(|pad| pad.axes.get(&axis).copied())
            .fold(0.0, |furthest, value| {
                if value.abs() > furthest.abs() {
                    value
                } else {
                    furthest
                }
            })
    }

    /// Tells the screens `axis`'s merged value, and presses or lets go of
    /// its two directions by it.
    fn route(&mut self, axis: PadAxis, out: &mut Vec<Input>) {
        let value = self.merged(axis);
        out.push(Input::PadAxis { axis, value });
        for positive in [true, false] {
            let along = if positive { value } else { -value };
            if let Some(pressed) = crossing(along) {
                let button = axis.direction(positive);
                let was = self.is_down(button);
                if pressed {
                    self.directions.insert(button);
                } else {
                    self.directions.remove(&button);
                }
                self.tell(button, was, out);
            }
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
        self.tell(button, was, out);
    }

    /// Tells the screens of `button` if it is no longer down or up as it
    /// `was`.
    fn tell(&self, button: PadButton, was: bool, out: &mut Vec<Input>) {
        let pressed = self.is_down(button);
        if pressed != was {
            out.push(Input::PadButton {
                button,
                pressed,
                repeat: false,
            });
        }
    }

    /// Forgets `pad`: lets go of each button it held that no other pad
    /// holds, and routes each stick axis over the pads left.
    fn lose(&mut self, pad: PadId, out: &mut Vec<Input>) {
        let Some(lost) = self.pads.remove(&pad) else {
            return;
        };
        for button in lost.down {
            if !self.is_down(button) {
                out.push(Input::PadButton {
                    button,
                    pressed: false,
                    repeat: false,
                });
            }
        }
        for axis in PadAxis::ALL {
            self.route(axis, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::super::Gamepads;
    use super::super::fixture::{FakeGamepads, is_trigger};
    use super::super::repeat::{REPEAT_DELAY, REPEAT_INTERVAL};
    use super::*;

    const MS: Duration = Duration::from_millis(1);

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
        Input::PadButton {
            button,
            pressed,
            repeat: false,
        }
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
    fn an_axis_past_the_threshold_presses_its_stick_direction() {
        let fake = FakeGamepads::with([
            vec![PadEvent::connected(ONE), axis(ONE, PadAxis::LeftY, -0.49)],
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
            vec![PadEvent::connected(ONE), axis(ONE, PadAxis::LeftX, 0.7)],
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
            vec![PadEvent::connected(ONE), axis(ONE, PadAxis::RightY, 0.45)],
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
            vec![PadEvent::connected(ONE), axis(ONE, PadAxis::LeftX, -1.0)],
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
            vec![PadEvent::connected(ONE), trigger(0.3)],
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
            PadEvent::connected(ONE),
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
            PadEvent::connected(ONE),
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
                PadEvent::connected(ONE),
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
            vec![PadEvent::connected(TWO), axis(TWO, PadAxis::RightX, 0.5)],
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
                PadEvent::connected(ONE),
                PadEvent::connected(TWO),
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
    fn two_pads_route_the_stick_pushed_furthest() {
        let fake = FakeGamepads::with([
            vec![
                PadEvent::connected(ONE),
                PadEvent::connected(TWO),
                axis(ONE, PadAxis::RightX, 0.6),
            ],
            vec![axis(TWO, PadAxis::RightX, 0.01)],
            vec![axis(TWO, PadAxis::RightX, -0.9)],
            vec![axis(TWO, PadAxis::RightX, -0.2)],
        ]);
        let mut inputs = PadInputs::new();
        let polls = polled(&fake, &mut inputs, 4);
        assert_eq!(
            polls[0],
            [
                value(PadAxis::RightX, 0.6),
                pressed(PadButton::RightStickRight, true)
            ]
        );
        assert_eq!(
            polls[1],
            [value(PadAxis::RightX, 0.6)],
            "the idle pad's jitter"
        );
        assert_eq!(
            polls[2],
            [
                value(PadAxis::RightX, -0.9),
                pressed(PadButton::RightStickRight, false),
                pressed(PadButton::RightStickLeft, true)
            ],
            "the other pad pushed further the other way"
        );
        assert_eq!(
            polls[3],
            [
                value(PadAxis::RightX, 0.6),
                pressed(PadButton::RightStickRight, true),
                pressed(PadButton::RightStickLeft, false)
            ],
            "and let back"
        );
    }

    #[test]
    fn two_pads_pushed_equally_far_route_the_lower_ids() {
        let fake = FakeGamepads::with([vec![
            PadEvent::connected(ONE),
            PadEvent::connected(TWO),
            axis(TWO, PadAxis::LeftY, -0.7),
            axis(ONE, PadAxis::LeftY, 0.7),
        ]]);
        let mut inputs = PadInputs::new();
        let polls = polled(&fake, &mut inputs, 1);
        let routed: Vec<Input> = polls[0]
            .iter()
            .copied()
            .filter(|input| matches!(input, Input::PadAxis { .. }))
            .collect();
        assert_eq!(routed.last(), Some(&value(PadAxis::LeftY, 0.7)));
        assert!(inputs.is_down(PadButton::LeftStickUp));
        assert!(!inputs.is_down(PadButton::LeftStickDown));
    }

    #[test]
    fn losing_one_of_two_pads_routes_the_others_stick() {
        let fake = FakeGamepads::with([
            vec![
                PadEvent::connected(ONE),
                PadEvent::connected(TWO),
                axis(ONE, PadAxis::RightX, 0.6),
                axis(TWO, PadAxis::RightX, 0.01),
                axis(TWO, PadAxis::LeftY, 0.3),
            ],
            vec![PadEvent::Disconnected(TWO)],
            vec![PadEvent::Disconnected(ONE)],
        ]);
        let mut inputs = PadInputs::new();
        let polls = polled(&fake, &mut inputs, 3);
        assert_eq!(buttons(&polls[1]), [], "the stick the other pad holds");
        assert!(polls[1].contains(&value(PadAxis::RightX, 0.6)));
        assert!(polls[1].contains(&value(PadAxis::LeftY, 0.0)));
        assert_eq!(
            buttons(&polls[2]),
            [pressed(PadButton::RightStickRight, false)]
        );
        assert!(polls[2].contains(&value(PadAxis::RightX, 0.0)));
    }

    /// What `inputs` makes over `dt` of hold.
    fn advanced(inputs: &mut PadInputs, dt: Duration) -> Vec<Input> {
        let mut out = Vec::new();
        inputs.advance(dt, &mut out);
        out
    }

    fn repeat(button: PadButton) -> Input {
        Input::PadButton {
            button,
            pressed: true,
            repeat: true,
        }
    }

    #[test]
    fn a_stick_pushed_past_the_threshold_repeats_its_direction() {
        let mut inputs = PadInputs::new();
        inputs.feed(
            &[PadEvent::connected(ONE), axis(ONE, PadAxis::LeftY, -0.9)],
            &mut Vec::new(),
        );
        assert_eq!(advanced(&mut inputs, REPEAT_DELAY.saturating_sub(MS)), []);
        assert_eq!(
            advanced(&mut inputs, MS + REPEAT_INTERVAL),
            [repeat(PadButton::LeftStickDown); 2]
        );
        let mut out = Vec::new();
        inputs.feed(&[axis(ONE, PadAxis::LeftY, 0.0)], &mut out);
        assert_eq!(buttons(&out), [pressed(PadButton::LeftStickDown, false)]);
        assert_eq!(advanced(&mut inputs, REPEAT_DELAY * 4), [], "let go");
    }

    #[test]
    fn a_dpad_direction_held_repeats_and_its_release_stops_it() {
        let dpad = |pressed| PadEvent::Button {
            pad: ONE,
            button: PadButton::DPadUp,
            pressed,
        };
        let mut inputs = PadInputs::new();
        inputs.feed(&[PadEvent::connected(ONE), dpad(true)], &mut Vec::new());
        assert_eq!(
            advanced(&mut inputs, REPEAT_DELAY),
            [repeat(PadButton::DPadUp)]
        );
        inputs.feed(&[dpad(false)], &mut Vec::new());
        assert_eq!(advanced(&mut inputs, REPEAT_DELAY * 4), []);
    }

    #[test]
    fn a_held_trigger_never_repeats() {
        let pulled: Vec<PadButton> = PadButton::ALL
            .into_iter()
            .filter(|&button| is_trigger(button) && !button.repeats())
            .collect();
        assert!(
            !pulled.is_empty(),
            "a value-crossing button that doesn't repeat"
        );
        for button in pulled {
            let mut inputs = PadInputs::new();
            let mut out = Vec::new();
            inputs.feed(
                &[
                    PadEvent::connected(ONE),
                    PadEvent::Trigger {
                        pad: ONE,
                        button,
                        value: 1.0,
                    },
                ],
                &mut out,
            );
            assert_eq!(out, [pressed(button, true)], "{button:?}");
            assert_eq!(
                advanced(&mut inputs, Duration::from_secs(2)),
                [],
                "{button:?}"
            );
        }
    }

    #[test]
    fn a_lost_pad_stops_its_repeat() {
        let mut inputs = PadInputs::new();
        inputs.feed(
            &[PadEvent::connected(ONE), axis(ONE, PadAxis::RightX, 0.9)],
            &mut Vec::new(),
        );
        inputs.feed(&[PadEvent::Disconnected(ONE)], &mut Vec::new());
        assert_eq!(advanced(&mut inputs, REPEAT_DELAY * 4), []);
    }

    #[test]
    fn draining_stops_the_repeat_and_follows_the_events() {
        let mut inputs = PadInputs::new();
        inputs.feed(
            &[PadEvent::connected(ONE), axis(ONE, PadAxis::LeftX, 0.9)],
            &mut Vec::new(),
        );
        inputs.drain(&[PadEvent::Button {
            pad: ONE,
            button: PadButton::DPadDown,
            pressed: true,
        }]);
        assert!(inputs.is_down(PadButton::DPadDown), "followed, unrouted");
        assert_eq!(
            advanced(&mut inputs, REPEAT_DELAY * 4),
            [],
            "neither the stick held across nor the press drained repeats"
        );
    }

    #[test]
    fn losing_an_unknown_pad_changes_nothing() {
        let mut inputs = PadInputs::new();
        let mut out = Vec::new();
        inputs.feed(&[PadEvent::Disconnected(ONE)], &mut out);
        assert_eq!(out, []);
    }
}
