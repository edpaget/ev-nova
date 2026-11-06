//! Text entry: the [`TextEntry`] port the app tells to start and stop the
//! platform's own entry, in the core's terms
//! (rdm:phase/key-bindings/phase-17-text-entry-port).
//!
//! - [`TextEntry`] is the port: start entry for a field at a rect of the
//!   window, stop it, and whether this platform shows its own keyboard on
//!   screen ([`TextEntry::soft_keyboard`], data the adapter reports). The
//!   real adapter is `platform::text_entry` (winit's IME requests, which
//!   also show the iPad's and Android's soft keyboards); a console port
//!   supplies its own over the system keyboard.
//! - [`NoTextEntry`] is the app's until one is given
//!   (`App::with_text_entry`): no soft keyboard, and nothing to start.
//! - `fixture` (the `fixture` feature, and this crate's tests) publishes
//!   the fake (`FakeTextEntry`) and the contract check (`check_contract`)
//!   every adapter runs.
//!
//! What a screen asks for is `nova_view`'s (`EntryAsk`, a field of the
//! frame), and when to start, restart or stop is the app's
//! (`app::text_entry`).
//!
//! Where an addition goes: a new thing to tell the platform is a port
//! method only if it is an action the OS takes; what the app decides from
//! the screens' asks stays in `app::text_entry`, pinned by its tests.

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

use crate::app::PxRect;

/// The platform's text entry, which the app starts at a focused field and
/// stops when no field asks.
pub trait TextEntry {
    /// Whether this platform shows its own keyboard on screen when entry
    /// starts (iOS, Android): the same answer on every call.
    fn soft_keyboard(&self) -> bool;

    /// Starts text entry for a field at `field` (physical pixels from the
    /// window's top-left): IME composition, with the candidate window by
    /// the field, and the soft keyboard where there is one. Called again
    /// while started, it starts afresh at `field`, and the soft keyboard
    /// shows again.
    fn start(&mut self, field: PxRect);

    /// Stops text entry: the IME off, and the soft keyboard hidden.
    /// Stopping when stopped does nothing.
    fn stop(&mut self);
}

/// No text entry: no soft keyboard, and nothing to start or stop. The
/// app's until the platform gives one; typed keys still type.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoTextEntry;

impl TextEntry for NoTextEntry {
    fn soft_keyboard(&self) -> bool {
        false
    }

    fn start(&mut self, _field: PxRect) {}

    fn stop(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_text_entry_meets_the_contract() {
        assert_eq!(fixture::check_contract(&mut NoTextEntry), Ok(()));
        assert!(!NoTextEntry.soft_keyboard());
    }
}
