//! A finger's touch and the two-finger pinch, as [`Input`](super::Input)
//! carries them.
//!
//! The touch router ([`crate::touch`]) decides what a screen sees of a
//! finger: the primary finger only as the ordinary pointer inputs, a
//! finger a claimant claimed as [`Input::Touch`](super::Input::Touch), and
//! two free fingers as [`Input::Pinch`](super::Input::Pinch).

use crate::geometry::Point;

/// A finger, opaque. It is valid only until its touch ends or is
/// cancelled: the system reuses ids, so a later touch with the same id is
/// a new finger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FingerId(usize);

impl FingerId {
    /// The finger the platform numbers `raw`.
    #[must_use]
    pub const fn new(raw: usize) -> Self {
        Self(raw)
    }
}

/// One step of a finger's touch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Touch {
    /// The finger.
    pub finger: FingerId,
    /// What it did.
    pub phase: TouchPhase,
}

/// What a finger did, at a point in the logical space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TouchPhase {
    /// It touched down.
    Started(Point),
    /// It moved.
    Moved(Point),
    /// It lifted.
    Ended(Point),
    /// The system stopped following it (a system gesture took over, or the
    /// window lost focus): whatever it pressed is let go without firing.
    Cancelled,
}

impl TouchPhase {
    /// The phase with its point moved by `by`.
    #[must_use]
    pub fn translated(self, by: Point) -> Self {
        let moved = |p: Point| Point::new(p.x + by.x, p.y + by.y);
        match self {
            Self::Started(at) => Self::Started(moved(at)),
            Self::Moved(at) => Self::Moved(moved(at)),
            Self::Ended(at) => Self::Ended(moved(at)),
            Self::Cancelled => Self::Cancelled,
        }
    }
}

/// Two free fingers spreading or pinching together.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pinch {
    /// A second free finger landed while the primary was down: the pinch
    /// begins, with both fingers' points.
    Began {
        /// The primary finger's point.
        primary: Point,
        /// The second finger's point.
        second: Point,
    },
    /// The fingers spread by one zoom step: zoom in.
    ZoomIn,
    /// The fingers came together by one zoom step: zoom out.
    ZoomOut,
    /// One of the two lifted or was cancelled.
    Ended,
}

impl Pinch {
    /// The pinch with its points moved by `by`.
    #[must_use]
    pub fn translated(self, by: Point) -> Self {
        let moved = |p: Point| Point::new(p.x + by.x, p.y + by.y);
        match self {
            Self::Began { primary, second } => Self::Began {
                primary: moved(primary),
                second: moved(second),
            },
            other => other,
        }
    }
}
