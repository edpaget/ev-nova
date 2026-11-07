//! Touch aids: small controls drawn over a screen off flight only while
//! touch was the device used last, so a touch player can do what a key or
//! a modifier gives the keyboard and mouse player
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps). Flight's own are
//! its overlay ([`crate::flight::touch_controls`]).
//!
//! - **[`ModifierToggle`]** ([`toggle`]): a one-shot toggle for each of
//!   the original's modifier clicks
//!   ([`CLICK_MODIFIERS`](crate::keymap::CLICK_MODIFIERS)), hosted by the
//!   trade center, the outfitter, the race and the in-flight maps.
//! - **[`PlayKeys`]** ([`play_keys`]): Map, Pilot and Missions, hosted by
//!   the spaceport over whichever of its screens shows, so touch opens
//!   the galaxy map, player info and mission info wherever their keys do.
//!
//! # The rules every aid keeps
//!
//! - **Shown** only while the host was last told
//!   [`Device::Touch`](crate::input::Device::Touch) and the host is the
//!   screen in front: it says it is live, and whoever holds it doesn't
//!   cover it ([`Screen::set_covered`](crate::screen::Screen::set_covered),
//!   [`host::cover`]). Hidden, an aid passes every input through
//!   untouched, so a keyboard, mouse or pad player's screen is exactly
//!   what it was; covered, it lets go and a toggle disarms.
//! - **Reaches the screen as the touch device**: an aid raises
//!   [`Input::TouchControl`](crate::input::Input::TouchControl) presses
//!   and releases of the same command the key, or the pad, gives, through
//!   the host's own reader. No new reader method, and no `Rulebook` key:
//!   a touch affordance the original lacks is a new feature.
//! - **Acts on the release inside it** ([`press`]): a press that slides
//!   off and lifts does nothing, and while it is held every pointer input
//!   is the aid's.
//! - **Sized in points** inside the safe area of the host's canvas, in a
//!   corner clear of the host's dialog where one is ([`place`]), and
//!   **drawn** as flight's square controls are ([`look`]).
//! - **Silent**, as phase 15's overlay is.
//!
//! # Where each addition is pinned
//!
//! An aid's rules are pinned in its own module's tests, the placement in
//! [`place`]'s and the hosting in [`host`]'s. Each host pins what its aid
//! does there (a toggled tap does what the modifier click does, the aid
//! hides under the host's modals and for any other device) in its own
//! `touch` module. The table in `tests.rs` pairs every modifier click
//! with a host, a new one failing it until it has a toggle, and pins that
//! each host's toggle hides and disarms covered. Each holder pins what
//! covers the screens it holds: the spaceport's in its
//! `tests::covered_aids`, flight's in its `tests::map_covered`, and the
//! app's in its own tests. The
//! every-screen table (`ui::pad_focus::every_screen`) leaves an aid's own
//! drawing out of its drawing comparisons (`PadProbe::touch_aids_drawn`).
//!
//! The on-screen keyboard ([`crate::ui::entry_keyboard`]) reuses these
//! parts but is no aid: it shows for the pad as well as for touch.

pub mod host;
pub mod look;
pub mod place;
pub mod play_keys;
pub mod press;
#[cfg(test)]
pub(crate) mod testkit;
pub mod toggle;

pub use play_keys::PlayKeys;
pub use toggle::ModifierToggle;

#[cfg(test)]
pub(crate) mod tests;
