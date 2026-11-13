//! The prompt for renaming a captured ship the player takes over
//! ([`CaptureNaming`]), from `nova_sim_port::capture_naming`, which
//! re-exports it and builds it from the strings and the captured class.

/// The prompt for renaming a captured ship the player takes over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureNaming {
    /// The prompt's text: `STR#` 2002 #119 alone (empty when missing).
    pub prompt: String,
    /// The name the prompt opens with: the captured class's `Name`, a
    /// space and three digits from 1 to 9.
    pub default: String,
}
