//! Test helpers that drive a real screen with fingers, through a
//! [`TouchRouter`], delivering what it routes as the app does: a claimed
//! touch or a pinch to [`Screen::input`], a pointer step as its pointer
//! input, and a cancel as [`Screen::cancel_pointer`].

use super::{PointerStep, Routed, TouchClaimant, TouchRouter};
use crate::geometry::Point;
use crate::input::{FingerId, Input, Touch, TouchPhase};
use crate::screen::{Screen, ScreenAction};

/// A router, a claimant and the screen they drive.
pub(crate) struct Fingers<'a, C: TouchClaimant> {
    router: TouchRouter,
    claimant: C,
    screen: &'a mut dyn Screen,
    /// Every action the screen answered with so far.
    actions: Vec<ScreenAction>,
}

impl<'a> Fingers<'a, fn(Point) -> bool> {
    /// Fingers on `screen`, which claims none of them.
    pub(crate) fn on(screen: &'a mut dyn Screen) -> Self {
        Self::claimed_by(screen, |_| false)
    }
}

impl<'a, C: TouchClaimant> Fingers<'a, C> {
    /// Fingers on `screen`, which `claimant` claims from.
    pub(crate) fn claimed_by(screen: &'a mut dyn Screen, claimant: C) -> Self {
        Self {
            router: TouchRouter::default(),
            claimant,
            screen,
            actions: Vec::new(),
        }
    }

    /// Every action the screen answered with since this was last taken.
    pub(crate) fn take_actions(&mut self) -> Vec<ScreenAction> {
        std::mem::take(&mut self.actions)
    }

    /// Finger `id` does `phase`, and the screen gets what is routed; the
    /// actions it answers with, which are also kept for
    /// [`Self::take_actions`].
    pub(crate) fn go(&mut self, id: usize, phase: TouchPhase) -> Vec<ScreenAction> {
        let input = Input::Touch(Touch {
            finger: FingerId::new(id),
            phase,
        });
        let routed = self.router.route(&input, &self.claimant);
        let mut actions = Vec::new();
        for step in routed {
            match step {
                Routed::Claimed(input) => actions.push(self.screen.input(&input)),
                Routed::Pinch(pinch) => actions.push(self.screen.input(&Input::Pinch(pinch))),
                Routed::Pointer(PointerStep::Cancelled) => self.screen.cancel_pointer(),
                Routed::Pointer(step) => {
                    if let Some(input) = step.input() {
                        actions.push(self.screen.input(&input));
                    }
                }
            }
        }
        self.actions.extend(&actions);
        actions
    }

    /// Finger `id` touches down at `at`.
    pub(crate) fn down(&mut self, id: usize, at: Point) -> Vec<ScreenAction> {
        self.go(id, TouchPhase::Started(at))
    }

    /// Finger `id` moves to `at`.
    pub(crate) fn move_to(&mut self, id: usize, at: Point) -> Vec<ScreenAction> {
        self.go(id, TouchPhase::Moved(at))
    }

    /// Finger `id` lifts at `at`.
    pub(crate) fn up(&mut self, id: usize, at: Point) -> Vec<ScreenAction> {
        self.go(id, TouchPhase::Ended(at))
    }

    /// The system cancels finger `id`.
    pub(crate) fn cancel(&mut self, id: usize) -> Vec<ScreenAction> {
        self.go(id, TouchPhase::Cancelled)
    }
}

/// One finger taps `screen` at `at`; the actions it answers with.
pub(crate) fn tap(screen: &mut dyn Screen, at: Point) -> Vec<ScreenAction> {
    let mut fingers = Fingers::on(screen);
    let mut actions = fingers.down(1, at);
    actions.extend(fingers.up(1, at));
    actions
}
