//! The sounds screens make, as events: what happened, not what to play.
//! The audio side decides which `snd ` each one plays, if any.
//!
//! - [`UiSound`]: the interface's own, a button pressed and released.
//! - [`SimSound`]: the flight session's, re-exported from `nova_sim`.
//! - [`Sound`]: either, as a screen reports it through
//!   [`Screen::take_sounds`](crate::Screen::take_sounds).

pub use nova_sim::SimSound;

/// A sound the interface makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiSound {
    /// A button was pressed: the pointer went down on it.
    ButtonDown,
    /// A pressed button popped back up: the pointer was let go, on the
    /// button or off it.
    ButtonUp,
}

/// A sound a screen reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// From the flight session.
    Sim(SimSound),
    /// From the interface.
    Ui(UiSound),
}
