//! Touch aids: small controls drawn over a screen only while touch was the
//! device used last, so a touch player can do what a key or a modifier
//! gives the keyboard and mouse player
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps).

pub mod look;
pub mod place;
pub mod press;
pub mod toggle;

pub use toggle::ModifierToggle;
#[cfg(test)]
pub(crate) mod testkit;
