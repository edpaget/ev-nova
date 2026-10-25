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

use crate::catalog::StellarId;
use crate::navigation::NavMode;
use crate::session::StellarPick;
use crate::sound::SimSound;

/// What the flight view draws from, filled by
/// [`Session::fill_frame`](crate::Session::fill_frame): every field is
/// written on each fill, so a frame kept from one fill to the next shows
/// nothing stale.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlightFrame {
    /// The player's nav mode and its selection
    /// ([`Session::nav_mode`](crate::Session::nav_mode)).
    pub nav_mode: NavMode,
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
