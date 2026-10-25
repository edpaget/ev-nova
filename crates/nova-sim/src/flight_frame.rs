//! The values the flight view's `FlightSim` port (`nova_view`'s
//! `flight::sim_port`) carries across the seam between the view and the
//! session: what a frame shows ([`FlightFrame`]), what the player asks
//! ([`FlightCommand`]) and what the session answers ([`Answer`]), what it
//! raised on the way ([`FlightEvent`]), and a fixed step's input and
//! report ([`StepInput`], [`StepReport`]).
//!
//! They live here, not in the view, so the session can build them:
//! [`Session::fill_frame`](crate::Session::fill_frame) fills a frame and
//! [`Session::apply`](crate::Session::apply) answers a command. Each
//! starts with what the first slice moved across the seam needs, the nav
//! mode and the stellar keys, and grows as later slices move more.

use std::collections::BTreeSet;

use crate::catalog::{StellarId, SystemId};
use crate::hyperspace::JumpReadiness;
use crate::navigation::NavMode;
use crate::session::StellarPick;
use crate::sound::SimSound;

/// What the flight view draws from, filled by
/// [`Session::fill_frame`](crate::Session::fill_frame): every field is
/// written on each fill, so a frame kept from one fill to the next shows
/// nothing stale. The default is what a session that failed to start
/// shows: nav off, no course, in flight, no jump.
#[derive(Clone, Debug, PartialEq)]
pub struct FlightFrame {
    /// The player's nav mode and its selection
    /// ([`Session::nav_mode`](crate::Session::nav_mode)).
    pub nav_mode: NavMode,
    /// The system the player is in
    /// ([`Session::system`](crate::Session::system)).
    pub system: SystemId,
    /// The course as it is flown
    /// ([`Session::course_shown`](crate::Session::course_shown)).
    pub course_shown: Vec<SystemId>,
    /// Whether a course is plotted, flown or not
    /// ([`Session::course`](crate::Session::course) has a hop).
    pub course_plotted: bool,
    /// Whether the player is docked
    /// ([`Session::landed`](crate::Session::landed) is some).
    pub docked: bool,
    /// Whether a jump is under way, the pre-jump stage included: the
    /// original's +0x50 > 0, which its key blocks test live
    /// ([`Session::jumping`](crate::Session::jumping) or
    /// [`Session::preparing_jump`](crate::Session::preparing_jump)).
    pub jump_under_way: bool,
    /// Whether the ship can jump to the system selected, as the nav area
    /// draws it ([`Session::jump_readiness`](crate::Session::jump_readiness)).
    pub jump_readiness: JumpReadiness,
    /// How many mission texts the session has queued, never reset
    /// ([`Session::texts_queued`](crate::Session::texts_queued)).
    pub texts_queued: u64,
    /// What the course map's drawing is taken from ([`MapKey`]).
    pub map_key: MapKey,
}

impl Default for FlightFrame {
    fn default() -> Self {
        Self {
            nav_mode: NavMode::Off,
            system: SystemId(0),
            course_shown: Vec::new(),
            course_plotted: false,
            docked: false,
            jump_under_way: false,
            jump_readiness: JumpReadiness::Blocked,
            texts_queued: 0,
            map_key: MapKey::default(),
        }
    }
}

/// What the course map's drawing
/// ([`Session::map_drawing`](crate::Session::map_drawing)) is taken from:
/// the current system, the systems explored and charted, the systems
/// known, the course saved and the nav. Two keys are equal exactly when
/// all six are, so a view that keeps the key it last drew from takes the
/// drawing again when it changes. The view compares keys; it reads
/// nothing in them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapKey {
    /// The current system ([`Session::system`](crate::Session::system)).
    pub system: SystemId,
    /// The systems the pilot has explored, in the pilot's order.
    pub explored: Vec<SystemId>,
    /// The systems the pilot has charted, in the pilot's order.
    pub charted: Vec<SystemId>,
    /// The systems known ([`Session::known`](crate::Session::known)).
    pub known: BTreeSet<SystemId>,
    /// The course saved ([`Session::course`](crate::Session::course)).
    pub course: Vec<SystemId>,
    /// The nav mode ([`Session::nav_mode`](crate::Session::nav_mode)).
    pub nav: NavMode,
}

impl Default for MapKey {
    /// A failed session's: system 0, nothing explored, known or plotted,
    /// nav off.
    fn default() -> Self {
        Self {
            system: SystemId(0),
            explored: Vec::new(),
            charted: Vec::new(),
            known: BTreeSet::new(),
            course: Vec::new(),
            nav: NavMode::Off,
        }
    }
}

/// Something the player asks of the flight, which
/// [`Session::apply`](crate::Session::apply) answers.
#[derive(Clone, Debug, PartialEq)]
pub enum FlightCommand {
    /// A stellar key: make the stellar `pick` finds the nav target
    /// ([`Session::select_stellar`](crate::Session::select_stellar)).
    SelectStellar(StellarPick),
}

/// What [`Session::apply`](crate::Session::apply) answered a
/// [`FlightCommand`].
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    /// The command did nothing: nothing changed, and no sound was raised.
    Refused,
    /// The stellar a [`FlightCommand::SelectStellar`] selected.
    Stellar(StellarId),
}

impl Answer {
    /// Whether the command was refused.
    #[must_use]
    pub fn is_refused(&self) -> bool {
        matches!(self, Self::Refused)
    }
}

/// Something the flight raised for the view to act on once: drained from
/// the port after each step or command.
#[derive(Clone, Debug, PartialEq)]
pub enum FlightEvent {
    /// A sound to play.
    Sound(SimSound),
}

/// What one fixed step of the flight is given. Empty until the step moves
/// across the seam.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepInput {}

/// What one fixed step of the flight reports. Empty until the step moves
/// across the seam.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepReport {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_refusal_is_refused() {
        assert!(Answer::Refused.is_refused());
        assert!(!Answer::Stellar(StellarId(128)).is_refused());
    }
}
