//! A choice in the haggle dialog ([`Haggle`]), from
//! `nova_sim_ai::hail::deal`, which re-exports it and keeps the
//! conversation, its mood and its price.

/// A choice in the haggle dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Haggle {
    /// "Accept Price".
    Accept,
    /// "Lower Price".
    LowerPrice,
}
