//! The [`Gamepads`] port's fake and its contract, for this crate's tests
//! and its integration tests (the `fixture` feature).

use std::cell::RefCell;
use std::collections::{BTreeSet, VecDeque};
use std::rc::Rc;

use nova_view::PadButton;

use super::{Gamepads, PadEvent, PadId};

/// What a [`FakeGamepads`] and its clones share.
#[derive(Debug, Default)]
struct Script {
    /// The batches still to report, one a poll.
    batches: VecDeque<Vec<PadEvent>>,
    /// How many times it was polled.
    polls: usize,
}

/// A scripted [`Gamepads`]: each poll reports the next batch pushed, or
/// nothing, and counts itself. Its clones share the script, so a test
/// that hands one to the app keeps another to script and count.
#[derive(Clone, Debug, Default)]
pub struct FakeGamepads {
    /// The shared script.
    script: Rc<RefCell<Script>>,
}

impl FakeGamepads {
    /// A fake with nothing scripted.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A fake reporting `batches`, one a poll.
    #[must_use]
    pub fn with(batches: impl IntoIterator<Item = Vec<PadEvent>>) -> Self {
        let fake = Self::new();
        for batch in batches {
            fake.push(batch);
        }
        fake
    }

    /// Queues `batch` for a later poll, after those queued already.
    pub fn push(&self, batch: Vec<PadEvent>) {
        self.script.borrow_mut().batches.push_back(batch);
    }

    /// How many times it was polled.
    #[must_use]
    pub fn polls(&self) -> usize {
        self.script.borrow().polls
    }
}

impl Gamepads for FakeGamepads {
    fn poll(&mut self, out: &mut Vec<PadEvent>) {
        let mut script = self.script.borrow_mut();
        script.polls += 1;
        if let Some(batch) = script.batches.pop_front() {
            out.extend(batch);
        }
    }
}

/// How many polls [`check_contract`] reads.
const POLLS: usize = 8;

/// Checks what `pads` reports over a few polls against the port's
/// contract:
///
/// - a poll only appends: what `out` held stays, in front;
/// - no event names a pad before its `Connected` or after its
///   `Disconnected`, and a pad connected is not reported connected again
///   until it is lost;
/// - an axis value lies in -1 to 1 and a trigger's in 0 to 1, each a
///   number;
/// - a trigger's value is `LeftTrigger`'s or `RightTrigger`'s, and a
///   button event is never a trigger or a stick direction (the core makes
///   those from the values).
///
/// # Errors
///
/// The first broken clause, described.
pub fn check_contract(pads: &mut impl Gamepads) -> Result<(), String> {
    let mut connected: BTreeSet<PadId> = BTreeSet::new();
    let sentinel = PadEvent::Connected(PadId(usize::MAX));
    for poll in 0..POLLS {
        let mut out = vec![sentinel];
        pads.poll(&mut out);
        if out.first() != Some(&sentinel) {
            return Err(format!("poll {poll} didn't leave what out held: {out:?}"));
        }
        for event in &out[1..] {
            check_event(event, &mut connected).map_err(|why| format!("poll {poll}: {why}"))?;
        }
    }
    Ok(())
}

/// Checks one event against the pads `connected` so far, and follows it.
fn check_event(event: &PadEvent, connected: &mut BTreeSet<PadId>) -> Result<(), String> {
    let pad = event.pad();
    match *event {
        PadEvent::Connected(_) => {
            if !connected.insert(pad) {
                return Err(format!("{pad:?} connected twice"));
            }
            return Ok(());
        }
        PadEvent::Disconnected(_) => {
            if !connected.remove(&pad) {
                return Err(format!("{pad:?} lost while not connected"));
            }
            return Ok(());
        }
        PadEvent::Button { button, .. } => {
            if is_trigger(button) || button.stick_direction().is_some() {
                return Err(format!("{button:?} reported as a button: {event:?}"));
            }
        }
        PadEvent::Axis { value, .. } => {
            if !(-1.0..=1.0).contains(&value) {
                return Err(format!("an axis value out of -1 to 1: {event:?}"));
            }
        }
        PadEvent::Trigger { button, value, .. } => {
            if !is_trigger(button) {
                return Err(format!("{button:?} reported as a trigger: {event:?}"));
            }
            if !(0.0..=1.0).contains(&value) {
                return Err(format!("a trigger value out of 0 to 1: {event:?}"));
            }
        }
    }
    if connected.contains(&pad) {
        Ok(())
    } else {
        Err(format!("{event:?} names a pad not connected"))
    }
}

/// Whether `button` is a trigger.
fn is_trigger(button: PadButton) -> bool {
    matches!(button, PadButton::LeftTrigger | PadButton::RightTrigger)
}

#[cfg(test)]
mod tests {
    use nova_view::PadAxis;

    use super::*;

    const ONE: PadId = PadId(1);

    fn button(button: PadButton, pressed: bool) -> PadEvent {
        PadEvent::Button {
            pad: ONE,
            button,
            pressed,
        }
    }

    #[test]
    fn the_fake_reports_each_batch_on_its_own_poll_and_counts_polls() {
        let fake = FakeGamepads::with([vec![PadEvent::Connected(ONE)], vec![]]);
        fake.push(vec![button(PadButton::South, true)]);
        let mut handed = fake.clone();
        let mut out = Vec::new();
        handed.poll(&mut out);
        assert_eq!(out, [PadEvent::Connected(ONE)]);
        handed.poll(&mut out);
        assert_eq!(out.len(), 1, "an empty batch");
        handed.poll(&mut out);
        assert_eq!(out[1], button(PadButton::South, true));
        handed.poll(&mut out);
        assert_eq!(out.len(), 2, "nothing left");
        assert_eq!(fake.polls(), 4, "counted through a clone");
    }

    #[test]
    fn the_fake_passes_the_gamepads_contract() {
        let mut fake = FakeGamepads::with([
            vec![PadEvent::Connected(ONE), button(PadButton::South, true)],
            vec![
                PadEvent::Axis {
                    pad: ONE,
                    axis: PadAxis::RightX,
                    value: -1.0,
                },
                PadEvent::Trigger {
                    pad: ONE,
                    button: PadButton::RightTrigger,
                    value: 1.0,
                },
            ],
            vec![PadEvent::Disconnected(ONE), PadEvent::Connected(ONE)],
        ]);
        assert_eq!(check_contract(&mut fake), Ok(()));
        assert_eq!(check_contract(&mut FakeGamepads::new()), Ok(()));
    }

    /// A port that clears what `out` held.
    struct Clearing;

    impl Gamepads for Clearing {
        fn poll(&mut self, out: &mut Vec<PadEvent>) {
            out.clear();
        }
    }

    fn breaks(batches: Vec<Vec<PadEvent>>, why: &str) {
        let error = check_contract(&mut FakeGamepads::with(batches)).expect_err(why);
        assert!(error.contains(why), "{error} for {why}");
    }

    #[test]
    fn a_broken_port_fails_each_clause() {
        assert!(
            check_contract(&mut Clearing)
                .expect_err("cleared")
                .contains("didn't leave")
        );
        breaks(vec![vec![button(PadButton::South, true)]], "not connected");
        breaks(
            vec![
                vec![PadEvent::Connected(ONE)],
                vec![PadEvent::Disconnected(ONE)],
                vec![button(PadButton::East, false)],
            ],
            "not connected",
        );
        breaks(
            vec![vec![PadEvent::Connected(ONE), PadEvent::Connected(ONE)]],
            "connected twice",
        );
        breaks(vec![vec![PadEvent::Disconnected(ONE)]], "lost while not");
        let axis = |value| PadEvent::Axis {
            pad: ONE,
            axis: PadAxis::LeftY,
            value,
        };
        for value in [1.5, -1.01, f32::NAN] {
            breaks(
                vec![vec![PadEvent::Connected(ONE), axis(value)]],
                "an axis value",
            );
        }
        let trigger = |button, value| PadEvent::Trigger {
            pad: ONE,
            button,
            value,
        };
        for value in [-0.1, 1.1, f32::NAN] {
            breaks(
                vec![vec![
                    PadEvent::Connected(ONE),
                    trigger(PadButton::LeftTrigger, value),
                ]],
                "a trigger value",
            );
        }
        breaks(
            vec![vec![
                PadEvent::Connected(ONE),
                trigger(PadButton::South, 0.5),
            ]],
            "reported as a trigger",
        );
        for wrong in [PadButton::LeftStickDown, PadButton::RightTrigger] {
            breaks(
                vec![vec![PadEvent::Connected(ONE), button(wrong, true)]],
                "reported as a button",
            );
        }
    }
}
