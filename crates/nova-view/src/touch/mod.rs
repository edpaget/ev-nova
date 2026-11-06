//! The touch router: where each finger goes.
//!
//! The app hands every [`Input::Touch`] to one [`TouchRouter`], which
//! decides what the screens see of it. This is the only place a touch
//! becomes pointer input, so nothing sees a touch twice.
//!
//! - **A claimed finger.** As a touch starts, the router asks a
//!   [`TouchClaimant`] whether the point falls on its controls. A claimed
//!   finger is the claimant's until it ends or is cancelled, even after it
//!   slides off them, and goes out as [`Routed::Claimed`], never as the
//!   pointer. The app's claimant is the screen
//!   ([`Screen::claims_touch`](crate::Screen::claims_touch)), which
//!   claims for flight's on-screen touch controls
//!   ([`crate::flight::touch_controls`]) while flight shows with nothing
//!   over it, and nothing elsewhere.
//! - **The primary finger.** The first *unclaimed* finger down while no
//!   other unclaimed finger is down is the UI's pointer
//!   ([`Routed::Pointer`]): a move to its point and a press, its moves,
//!   and a release where it lifts, so every button, list and drag works
//!   under it unchanged. A cancelled primary lets go without firing
//!   ([`PointerStep::Cancelled`]). Claimed fingers don't count, so a thumb
//!   on a claimant's control doesn't keep a tap elsewhere from pressing.
//! - **Other fingers** press nothing, and no finger is ever promoted to
//!   primary: once the primary lifts while another unclaimed finger is
//!   down, no unclaimed finger presses until every one has lifted.
//! - **A pinch.** A second unclaimed finger landing while the primary is
//!   the only one down begins a [`Pinch`]; the two fingers' spread makes
//!   the map's zoom steps ([`pinch`]), and either lifting ends it. The
//!   primary stays the pointer through it: the galaxy map cancels its own
//!   press as the pinch begins (`galaxy::touch`), and every other screen
//!   ignores the pinch.
//! - **Unknown fingers.** A move, end or cancel of a finger that isn't
//!   down routes nothing: winit's leave after every lift arrives as a
//!   cancel, and a finger that started in a letterbox bar never reached
//!   the router.
//!
//! # Where each addition is pinned
//!
//! The router's rules are pinned in `router.rs`'s tests and the pinch's
//! steps in `pinch.rs`'s. Each screen's behaviour under a finger (a tap
//! is a click, a cancel fires nothing, a second finger presses nothing)
//! is pinned over every screen in `ui::pad_focus`'s `every_screen` table,
//! and the map's pinch in `galaxy::touch`. The app's wiring (pixels to
//! logical points, the letterbox bars, the dev overlay, the device used
//! last) is `nova`'s, pinned in its `app` tests.

use crate::geometry::Point;
use crate::input::{Input, MouseButton, Pinch};

pub mod pinch;
mod router;
#[cfg(test)]
pub(crate) mod testkit;

pub use pinch::{PINCH_STEP, PinchSteps};
pub use router::TouchRouter;

/// What a touch claims, asked as a touch starts at `at` (logical): whether
/// it falls on the claimant's controls.
pub trait TouchClaimant {
    /// Whether a touch starting at `at` is the claimant's.
    fn claims(&self, at: Point) -> bool;
}

impl<F: Fn(Point) -> bool> TouchClaimant for F {
    fn claims(&self, at: Point) -> bool {
        self(at)
    }
}

/// Where the router sends one step of a touch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Routed {
    /// To the claimant's screen, as it is.
    Claimed(Input),
    /// To the UI's pointer.
    Pointer(PointerStep),
    /// To the screen, as [`Input::Pinch`].
    Pinch(Pinch),
}

/// One step of the primary finger as the UI's pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerStep {
    /// The pointer moved to a point.
    Moved(Point),
    /// The primary button went down at a point.
    Pressed(Point),
    /// The primary button went up at a point.
    Released(Point),
    /// The press was cancelled: let go without firing
    /// ([`Screen::cancel_pointer`](crate::Screen::cancel_pointer)).
    Cancelled,
}

impl PointerStep {
    /// The pointer input the step is, or `None` for a cancel, which the
    /// deliverer makes a
    /// [`Screen::cancel_pointer`](crate::Screen::cancel_pointer).
    #[must_use]
    pub const fn input(self) -> Option<Input> {
        match self {
            Self::Moved(at) => Some(Input::PointerMoved(at)),
            Self::Pressed(at) => Some(left(true, at)),
            Self::Released(at) => Some(left(false, at)),
            Self::Cancelled => None,
        }
    }
}

/// The primary button going down or up at `at`.
const fn left(pressed: bool, at: Point) -> Input {
    Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pointer_step_is_its_pointer_input() {
        let at = Point::new(12.0, 34.0);
        assert_eq!(
            PointerStep::Moved(at).input(),
            Some(Input::PointerMoved(at))
        );
        assert_eq!(PointerStep::Pressed(at).input(), Some(left(true, at)));
        assert_eq!(PointerStep::Released(at).input(), Some(left(false, at)));
        assert_eq!(PointerStep::Cancelled.input(), None);
    }

    #[test]
    fn a_closure_is_a_claimant() {
        let inside = |at: Point| at.x < 10.0;
        assert!(inside.claims(Point::new(5.0, 0.0)));
        assert!(!inside.claims(Point::new(15.0, 0.0)));
    }
}
