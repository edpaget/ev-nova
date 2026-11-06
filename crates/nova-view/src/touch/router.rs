//! [`TouchRouter`]: the state machine that sends each finger to the
//! claimant, the pointer, a pinch or nowhere (the module docs of
//! [`crate::touch`] give the rules).

use super::pinch::PinchSteps;
use super::{PointerStep, Routed, TouchClaimant};
use crate::geometry::Point;
use crate::input::{FingerId, Input, Pinch, Touch, TouchPhase};

/// What a finger down is to the router.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// The claimant's.
    Claimed,
    /// The UI's pointer.
    Primary,
    /// Unclaimed and pressing nothing.
    Other,
}

/// A finger down.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Finger {
    /// Its id.
    id: FingerId,
    /// What it is to the router.
    role: Role,
    /// Where it was last.
    at: Point,
}

/// The pinch the primary and a second finger make.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pinching {
    /// The second finger.
    second: FingerId,
    /// Its steps so far.
    steps: PinchSteps,
}

/// Sends each finger's touch where it goes: see [`crate::touch`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TouchRouter {
    /// The fingers down, in the order they landed.
    fingers: Vec<Finger>,
    /// Whether the primary lifted while another unclaimed finger was down:
    /// no unclaimed finger presses until every one has lifted.
    spent: bool,
    /// The pinch under way, if any.
    pinch: Option<Pinching>,
}

impl TouchRouter {
    /// Where `input`'s touch goes, asking `claimant` as a touch starts.
    /// Any input but [`Input::Touch`] goes nowhere.
    pub fn route(&mut self, input: &Input, claimant: &impl TouchClaimant) -> Vec<Routed> {
        let Input::Touch(Touch { finger, phase }) = *input else {
            return Vec::new();
        };
        let mut out = Vec::new();
        match phase {
            TouchPhase::Started(at) => {
                if self.index(finger).is_some() {
                    self.lift(finger, None, &mut out);
                }
                self.start(finger, at, claimant, &mut out);
            }
            TouchPhase::Moved(at) => self.moved(finger, at, &mut out),
            TouchPhase::Ended(at) => self.lift(finger, Some(at), &mut out),
            TouchPhase::Cancelled => self.lift(finger, None, &mut out),
        }
        out
    }

    /// Cancels every finger, as when the window loses focus or the dev
    /// overlay opens, and forgets them.
    pub fn cancel_all(&mut self) -> Vec<Routed> {
        let mut out = Vec::new();
        while let Some(finger) = self.fingers.first() {
            self.lift(finger.id, None, &mut out);
        }
        self.spent = false;
        out
    }

    /// Whether any finger, claimed or not, is down.
    #[must_use]
    pub fn any_down(&self) -> bool {
        !self.fingers.is_empty()
    }

    /// Where `finger` is in [`Self::fingers`], if it is down.
    fn index(&self, finger: FingerId) -> Option<usize> {
        self.fingers.iter().position(|each| each.id == finger)
    }

    /// The primary finger, if it is down.
    fn primary(&self) -> Option<Finger> {
        self.fingers
            .iter()
            .copied()
            .find(|each| each.role == Role::Primary)
    }

    /// How many unclaimed fingers are down.
    fn unclaimed(&self) -> usize {
        self.fingers
            .iter()
            .filter(|each| each.role != Role::Claimed)
            .count()
    }

    /// `finger` touches down at `at`.
    fn start(
        &mut self,
        finger: FingerId,
        at: Point,
        claimant: &impl TouchClaimant,
        out: &mut Vec<Routed>,
    ) {
        let role = if claimant.claims(at) {
            out.push(Routed::Claimed(claimed(finger, TouchPhase::Started(at))));
            Role::Claimed
        } else if self.unclaimed() == 0 && !self.spent {
            out.push(Routed::Pointer(PointerStep::Moved(at)));
            out.push(Routed::Pointer(PointerStep::Pressed(at)));
            Role::Primary
        } else {
            if let Some(primary) = self.primary()
                && self.unclaimed() == 1
            {
                out.push(Routed::Pinch(Pinch::Began {
                    primary: primary.at,
                    second: at,
                }));
                self.pinch = Some(Pinching {
                    second: finger,
                    steps: PinchSteps::new(distance(primary.at, at)),
                });
            }
            Role::Other
        };
        self.fingers.push(Finger {
            id: finger,
            role,
            at,
        });
    }

    /// `finger` moves to `at`.
    fn moved(&mut self, finger: FingerId, at: Point, out: &mut Vec<Routed>) {
        let Some(index) = self.index(finger) else {
            return;
        };
        self.fingers[index].at = at;
        match self.fingers[index].role {
            Role::Claimed => out.push(Routed::Claimed(claimed(finger, TouchPhase::Moved(at)))),
            Role::Primary => out.push(Routed::Pointer(PointerStep::Moved(at))),
            Role::Other => {}
        }
        let role = self.fingers[index].role;
        if let Some(pinch) = &mut self.pinch
            && (role == Role::Primary || pinch.second == finger)
            && let Some(primary) = self.fingers.iter().find(|each| each.role == Role::Primary)
            && let Some(second) = self.fingers.iter().find(|each| each.id == pinch.second)
        {
            let steps = pinch.steps.spread(distance(primary.at, second.at));
            out.extend(steps.into_iter().map(Routed::Pinch));
        }
    }

    /// `finger` lifts at `at`, or is cancelled for `None`, and is
    /// forgotten.
    fn lift(&mut self, finger: FingerId, at: Option<Point>, out: &mut Vec<Routed>) {
        let Some(index) = self.index(finger) else {
            return;
        };
        let lifted = self.fingers.remove(index);
        let in_pinch = self
            .pinch
            .is_some_and(|pinch| lifted.role == Role::Primary || pinch.second == finger);
        if in_pinch {
            self.pinch = None;
            out.push(Routed::Pinch(Pinch::Ended));
        }
        match (lifted.role, at) {
            (Role::Claimed, Some(at)) => {
                out.push(Routed::Claimed(claimed(finger, TouchPhase::Ended(at))));
            }
            (Role::Claimed, None) => {
                out.push(Routed::Claimed(claimed(finger, TouchPhase::Cancelled)));
            }
            (Role::Primary, Some(at)) => {
                if at != lifted.at {
                    out.push(Routed::Pointer(PointerStep::Moved(at)));
                }
                out.push(Routed::Pointer(PointerStep::Released(at)));
            }
            (Role::Primary, None) => out.push(Routed::Pointer(PointerStep::Cancelled)),
            (Role::Other, _) => {}
        }
        let unclaimed = self.unclaimed();
        if lifted.role == Role::Primary && unclaimed > 0 {
            self.spent = true;
        }
        if unclaimed == 0 {
            self.spent = false;
        }
    }
}

/// A claimed finger's touch, as the claimant gets it.
const fn claimed(finger: FingerId, phase: TouchPhase) -> Input {
    Input::Touch(Touch { finger, phase })
}

/// How far apart `a` and `b` are.
fn distance(a: Point, b: Point) -> f32 {
    (b.x - a.x).hypot(b.y - a.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Bounds;
    use std::cell::Cell;

    fn p(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn touch(id: usize, phase: TouchPhase) -> Input {
        Input::Touch(Touch {
            finger: FingerId::new(id),
            phase,
        })
    }

    fn never(_: Point) -> bool {
        false
    }

    fn always(_: Point) -> bool {
        true
    }

    /// A claimant that claims inside `rect` and counts how often it is
    /// asked.
    struct Asked {
        rect: Bounds,
        asked: Cell<usize>,
    }

    impl Asked {
        fn new() -> Self {
            Self {
                rect: Bounds::at(Point::new(0.0, 0.0), 100.0, 100.0),
                asked: Cell::new(0),
            }
        }
    }

    impl TouchClaimant for Asked {
        fn claims(&self, at: Point) -> bool {
            self.asked.set(self.asked.get() + 1);
            self.rect.contains(at)
        }
    }

    /// A router and its claimant, driven by finger.
    struct Drive<C: TouchClaimant> {
        router: TouchRouter,
        claimant: C,
    }

    impl<C: TouchClaimant> Drive<C> {
        fn new(claimant: C) -> Self {
            Self {
                router: TouchRouter::default(),
                claimant,
            }
        }

        fn go(&mut self, id: usize, phase: TouchPhase) -> Vec<Routed> {
            self.router.route(&touch(id, phase), &self.claimant)
        }

        fn down(&mut self, id: usize, at: Point) -> Vec<Routed> {
            self.go(id, TouchPhase::Started(at))
        }

        fn to(&mut self, id: usize, at: Point) -> Vec<Routed> {
            self.go(id, TouchPhase::Moved(at))
        }

        fn up(&mut self, id: usize, at: Point) -> Vec<Routed> {
            self.go(id, TouchPhase::Ended(at))
        }

        fn cancel(&mut self, id: usize) -> Vec<Routed> {
            self.go(id, TouchPhase::Cancelled)
        }
    }

    use PointerStep::{Cancelled, Moved, Pressed, Released};
    use Routed::{Claimed, Pointer};

    fn pinch(step: Pinch) -> Routed {
        Routed::Pinch(step)
    }

    #[test]
    fn the_first_finger_down_moves_presses_moves_and_releases_as_the_pointer() {
        let mut fingers = Drive::new(never);
        assert_eq!(
            fingers.down(1, p(10.0, 20.0)),
            [
                Pointer(Moved(p(10.0, 20.0))),
                Pointer(Pressed(p(10.0, 20.0)))
            ]
        );
        assert_eq!(
            fingers.to(1, p(11.0, 21.0)),
            [Pointer(Moved(p(11.0, 21.0)))]
        );
        assert_eq!(
            fingers.up(1, p(12.0, 22.0)),
            [
                Pointer(Moved(p(12.0, 22.0))),
                Pointer(Released(p(12.0, 22.0)))
            ]
        );
        assert!(!fingers.router.any_down());
    }

    #[test]
    fn an_end_where_the_finger_last_was_sends_no_move_first() {
        let mut fingers = Drive::new(never);
        fingers.down(1, p(10.0, 20.0));
        fingers.to(1, p(11.0, 21.0));
        assert_eq!(
            fingers.up(1, p(11.0, 21.0)),
            [Pointer(Released(p(11.0, 21.0)))]
        );
        fingers.down(2, p(5.0, 5.0));
        assert_eq!(fingers.up(2, p(5.0, 5.0)), [Pointer(Released(p(5.0, 5.0)))]);
    }

    #[test]
    fn a_cancelled_primary_is_cancelled_without_a_release() {
        let mut fingers = Drive::new(never);
        fingers.down(1, p(10.0, 20.0));
        assert_eq!(fingers.cancel(1), [Pointer(Cancelled)]);
        assert!(!fingers.router.any_down());
        assert_eq!(
            fingers.down(2, p(1.0, 1.0)),
            [Pointer(Moved(p(1.0, 1.0))), Pointer(Pressed(p(1.0, 1.0)))],
            "the next finger is primary"
        );
    }

    #[test]
    fn a_claimed_finger_goes_to_the_claimant_and_never_to_the_pointer() {
        let mut fingers = Drive::new(always);
        for (id, phase) in [
            (1, TouchPhase::Started(p(1.0, 2.0))),
            (1, TouchPhase::Moved(p(3.0, 4.0))),
            (1, TouchPhase::Ended(p(5.0, 6.0))),
            (2, TouchPhase::Started(p(7.0, 8.0))),
            (2, TouchPhase::Moved(p(9.0, 9.0))),
            (2, TouchPhase::Cancelled),
        ] {
            assert_eq!(
                fingers.go(id, phase),
                [Claimed(touch(id, phase))],
                "{phase:?}"
            );
        }
        assert!(!fingers.router.any_down());
    }

    #[test]
    fn the_claimant_is_asked_only_as_a_touch_starts() {
        let mut fingers = Drive::new(Asked::new());
        let inside = p(50.0, 50.0);
        let outside = p(500.0, 500.0);
        assert_eq!(
            fingers.down(1, inside),
            [Claimed(touch(1, TouchPhase::Started(inside)))]
        );
        assert_eq!(
            fingers.to(1, outside),
            [Claimed(touch(1, TouchPhase::Moved(outside)))],
            "it stays claimed off the controls"
        );
        assert_eq!(fingers.claimant.asked.get(), 1);
        fingers.down(2, outside);
        assert_eq!(
            fingers.to(2, inside),
            [Pointer(Moved(inside))],
            "the pointer stays the pointer on them"
        );
        assert_eq!(fingers.up(2, inside), [Pointer(Released(inside))],);
        fingers.up(1, outside);
        assert_eq!(fingers.claimant.asked.get(), 2, "once per start");
    }

    #[test]
    fn a_claimed_finger_does_not_keep_the_next_from_being_primary() {
        let mut fingers = Drive::new(Asked::new());
        fingers.down(1, p(50.0, 50.0));
        let tap = p(500.0, 300.0);
        assert_eq!(
            fingers.down(2, tap),
            [Pointer(Moved(tap)), Pointer(Pressed(tap))]
        );
        assert_eq!(fingers.up(2, tap), [Pointer(Released(tap))]);
    }

    #[test]
    fn a_second_finger_presses_nothing() {
        let mut fingers = Drive::new(never);
        fingers.down(1, p(10.0, 10.0));
        let second = fingers.down(2, p(300.0, 300.0));
        assert!(
            second.iter().all(|routed| !matches!(routed, Pointer(_))),
            "{second:?}"
        );
        assert!(
            fingers
                .up(2, p(300.0, 300.0))
                .iter()
                .all(|r| !matches!(r, Pointer(_)))
        );
        assert_eq!(
            fingers.up(1, p(10.0, 10.0)),
            [Pointer(Released(p(10.0, 10.0)))]
        );
    }

    #[test]
    fn when_the_primary_lifts_the_others_press_nothing_until_every_finger_has_lifted() {
        let mut fingers = Drive::new(never);
        fingers.down(1, p(10.0, 10.0));
        fingers.down(2, p(300.0, 300.0));
        fingers.up(1, p(10.0, 10.0));
        assert_eq!(fingers.to(2, p(310.0, 300.0)), []);
        assert_eq!(fingers.down(3, p(50.0, 50.0)), [], "a new finger");
        assert_eq!(fingers.up(2, p(310.0, 300.0)), []);
        assert_eq!(fingers.to(3, p(51.0, 50.0)), []);
        assert_eq!(fingers.up(3, p(51.0, 50.0)), []);
        assert_eq!(
            fingers.down(4, p(20.0, 20.0)),
            [
                Pointer(Moved(p(20.0, 20.0))),
                Pointer(Pressed(p(20.0, 20.0)))
            ],
            "every finger lifted"
        );
    }

    #[test]
    fn a_finger_is_never_promoted_to_primary() {
        let mut fingers = Drive::new(never);
        fingers.down(1, p(10.0, 10.0));
        fingers.down(2, p(300.0, 300.0));
        fingers.down(3, p(400.0, 300.0));
        fingers.up(1, p(10.0, 10.0));
        for id in [2, 3] {
            assert_eq!(fingers.to(id, p(1.0, 1.0)), [], "{id}");
            assert_eq!(fingers.up(id, p(2.0, 2.0)), [], "{id}");
        }
    }

    #[test]
    fn a_cancelled_primary_with_another_down_leaves_the_router_spent() {
        let mut fingers = Drive::new(never);
        fingers.down(1, p(10.0, 10.0));
        fingers.down(2, p(300.0, 300.0));
        assert_eq!(fingers.cancel(1), [pinch(Pinch::Ended), Pointer(Cancelled)]);
        assert_eq!(fingers.down(3, p(50.0, 50.0)), []);
        fingers.cancel(2);
        fingers.cancel(3);
        assert_eq!(
            fingers.down(4, p(5.0, 5.0)),
            [Pointer(Moved(p(5.0, 5.0))), Pointer(Pressed(p(5.0, 5.0)))]
        );
    }

    #[test]
    fn a_claimed_finger_down_does_not_keep_the_router_spent() {
        let mut fingers = Drive::new(Asked::new());
        fingers.down(1, p(500.0, 500.0));
        fingers.down(2, p(50.0, 50.0));
        fingers.down(3, p(600.0, 500.0));
        fingers.up(1, p(500.0, 500.0));
        fingers.up(3, p(600.0, 500.0));
        assert_eq!(
            fingers.down(4, p(700.0, 500.0)),
            [
                Pointer(Moved(p(700.0, 500.0))),
                Pointer(Pressed(p(700.0, 500.0)))
            ],
            "only the claimed finger is down"
        );
    }

    #[test]
    fn events_of_a_finger_not_down_route_nothing() {
        let mut fingers = Drive::new(never);
        assert_eq!(fingers.to(7, p(1.0, 1.0)), []);
        assert_eq!(fingers.up(7, p(1.0, 1.0)), []);
        assert_eq!(fingers.cancel(7), []);
        fingers.down(1, p(10.0, 10.0));
        fingers.up(1, p(10.0, 10.0));
        assert_eq!(fingers.cancel(1), [], "winit's leave after the lift");
        let mut claimed = Drive::new(always);
        claimed.down(1, p(10.0, 10.0));
        claimed.up(1, p(10.0, 10.0));
        assert_eq!(claimed.cancel(1), [], "a claimed finger's leave too");
    }

    #[test]
    fn a_reused_finger_id_is_a_new_touch() {
        let mut fingers = Drive::new(Asked::new());
        fingers.down(1, p(50.0, 50.0));
        fingers.up(1, p(50.0, 50.0));
        assert_eq!(
            fingers.down(1, p(500.0, 500.0)),
            [
                Pointer(Moved(p(500.0, 500.0))),
                Pointer(Pressed(p(500.0, 500.0)))
            ],
            "asked again, and unclaimed"
        );
        assert_eq!(fingers.claimant.asked.get(), 2);
    }

    #[test]
    fn a_start_for_a_finger_already_down_cancels_the_old_touch_first() {
        let mut fingers = Drive::new(never);
        fingers.down(1, p(10.0, 10.0));
        assert_eq!(
            fingers.down(1, p(20.0, 20.0)),
            [
                Pointer(Cancelled),
                Pointer(Moved(p(20.0, 20.0))),
                Pointer(Pressed(p(20.0, 20.0)))
            ]
        );
        let mut claimed = Drive::new(always);
        claimed.down(1, p(10.0, 10.0));
        assert_eq!(
            claimed.down(1, p(20.0, 20.0)),
            [
                Claimed(touch(1, TouchPhase::Cancelled)),
                Claimed(touch(1, TouchPhase::Started(p(20.0, 20.0))))
            ]
        );
    }

    #[test]
    fn a_second_free_finger_begins_a_pinch_at_both_points() {
        let mut fingers = Drive::new(never);
        fingers.down(1, p(10.0, 10.0));
        fingers.to(1, p(100.0, 100.0));
        assert_eq!(
            fingers.down(2, p(190.0, 100.0)),
            [pinch(Pinch::Began {
                primary: p(100.0, 100.0),
                second: p(190.0, 100.0)
            })]
        );
    }

    #[test]
    fn spreading_the_pinch_steps_the_zoom_after_the_primary_moves() {
        let mut fingers = Drive::new(never);
        fingers.down(1, p(100.0, 100.0));
        fingers.down(2, p(190.0, 100.0));
        assert_eq!(
            fingers.to(1, p(70.0, 100.0)),
            [Pointer(Moved(p(70.0, 100.0))), pinch(Pinch::ZoomIn)],
            "90 apart to 120"
        );
        assert_eq!(fingers.to(2, p(220.0, 100.0)), [], "150 apart");
        assert_eq!(
            fingers.to(2, p(220.0, 200.0)),
            [pinch(Pinch::ZoomIn)],
            "the distance, not the width"
        );
        assert_eq!(
            fingers.to(2, p(130.0, 100.0)),
            [
                pinch(Pinch::ZoomOut),
                pinch(Pinch::ZoomOut),
                pinch(Pinch::ZoomOut)
            ],
            "60 apart"
        );
    }

    #[test]
    fn a_pinch_ends_when_either_finger_lifts_or_is_cancelled() {
        let mut fingers = Drive::new(never);
        fingers.down(1, p(100.0, 100.0));
        fingers.down(2, p(190.0, 100.0));
        assert_eq!(
            fingers.up(1, p(100.0, 100.0)),
            [pinch(Pinch::Ended), Pointer(Released(p(100.0, 100.0)))]
        );
        assert_eq!(fingers.to(2, p(500.0, 100.0)), [], "no longer pinching");
        fingers.up(2, p(500.0, 100.0));

        fingers.down(1, p(100.0, 100.0));
        fingers.down(2, p(190.0, 100.0));
        assert_eq!(fingers.up(2, p(190.0, 100.0)), [pinch(Pinch::Ended)]);
        assert_eq!(
            fingers.to(1, p(10.0, 100.0)),
            [Pointer(Moved(p(10.0, 100.0)))],
            "the primary is still the pointer"
        );
        fingers.down(3, p(100.0, 100.0));
        assert_eq!(fingers.cancel(3), [pinch(Pinch::Ended)], "a second pinch");
        assert_eq!(fingers.cancel(1), [Pointer(Cancelled)]);
    }

    #[test]
    fn a_third_finger_neither_joins_nor_ends_a_pinch() {
        let mut fingers = Drive::new(never);
        fingers.down(1, p(100.0, 100.0));
        fingers.down(2, p(190.0, 100.0));
        assert_eq!(fingers.down(3, p(500.0, 500.0)), []);
        assert_eq!(fingers.to(3, p(900.0, 900.0)), []);
        assert_eq!(fingers.up(3, p(900.0, 900.0)), []);
        assert_eq!(fingers.to(2, p(220.0, 100.0)), [pinch(Pinch::ZoomIn)]);
    }

    #[test]
    fn a_claimed_finger_never_pinches() {
        let mut fingers = Drive::new(Asked::new());
        fingers.down(1, p(500.0, 500.0));
        assert_eq!(
            fingers.down(2, p(50.0, 50.0)),
            [Claimed(touch(2, TouchPhase::Started(p(50.0, 50.0))))]
        );
        assert_eq!(
            fingers.to(2, p(10.0, 10.0)),
            [Claimed(touch(2, TouchPhase::Moved(p(10.0, 10.0))))]
        );
        assert_eq!(
            fingers.up(2, p(10.0, 10.0)),
            [Claimed(touch(2, TouchPhase::Ended(p(10.0, 10.0))))]
        );
    }

    #[test]
    fn cancel_all_cancels_the_claimed_ends_the_pinch_and_cancels_the_pointer() {
        let mut fingers = Drive::new(Asked::new());
        fingers.down(1, p(50.0, 50.0));
        fingers.down(2, p(500.0, 500.0));
        fingers.down(3, p(600.0, 500.0));
        let cancels = fingers.router.cancel_all();
        assert_eq!(
            cancels,
            [
                Claimed(touch(1, TouchPhase::Cancelled)),
                pinch(Pinch::Ended),
                Pointer(Cancelled)
            ]
        );
        assert!(!fingers.router.any_down());
        assert_eq!(fingers.to(2, p(1.0, 1.0)), [], "forgotten");
        assert_eq!(fingers.router.cancel_all(), []);
    }

    #[test]
    fn cancel_all_clears_spent() {
        let mut fingers = Drive::new(never);
        fingers.down(1, p(10.0, 10.0));
        fingers.down(2, p(300.0, 300.0));
        fingers.up(1, p(10.0, 10.0));
        fingers.router.cancel_all();
        assert_eq!(
            fingers.down(3, p(5.0, 5.0)),
            [Pointer(Moved(p(5.0, 5.0))), Pointer(Pressed(p(5.0, 5.0)))]
        );
    }

    #[test]
    fn inputs_that_are_not_touches_route_nothing() {
        let mut router = TouchRouter::default();
        for input in [
            Input::PointerMoved(p(1.0, 1.0)),
            Input::Pinch(Pinch::ZoomIn),
            Input::Text('a'),
        ] {
            assert_eq!(router.route(&input, &always), [], "{input:?}");
        }
        assert!(!router.any_down());
    }

    #[test]
    fn any_down_follows_every_finger_claimed_or_not() {
        let mut fingers = Drive::new(Asked::new());
        assert!(!fingers.router.any_down());
        fingers.down(1, p(50.0, 50.0));
        assert!(fingers.router.any_down(), "claimed");
        fingers.down(2, p(500.0, 50.0));
        fingers.up(1, p(50.0, 50.0));
        assert!(fingers.router.any_down(), "the pointer");
        fingers.down(3, p(600.0, 50.0));
        fingers.up(2, p(500.0, 50.0));
        assert!(fingers.router.any_down(), "another");
        fingers.cancel(3);
        assert!(!fingers.router.any_down());
    }
}
