//! The help an NPC gives the player once asked ([`Help`]), from
//! `nova_sim_ai::ai`, which re-exports it and keeps the NPCs' goals and
//! the AI that flies them over to give it.

/// The help an NPC flies over to give the player, once asked (see
/// `nova_sim_ai::hail`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Help {
    /// It tops up the player's fuel.
    Refuel,
    /// It repairs the player's disabled ship.
    Repair,
}
