//! How a target command picks ([`TargetPick`]), from
//! `nova_sim_ai::targeting`, which re-exports it and keeps the searches.

/// How a target command picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetPick {
    /// The candidate nearest the player.
    Nearest,
    /// The candidate nearest the player that threatens it or a ship it
    /// leads (`_IsThreatToPlayer` @0x7f572-0x7f590,
    /// `nova_sim_ai::traffic::npc::Npc::threatens_player_or_led`).
    NearestThreat,
    /// The next candidate after the target (`nova_sim_ai::targeting::next`): Tab.
    Next,
    /// The next ship of the escort class after the target (the player's
    /// hired escorts and the unmarked ships they lead, such as their
    /// fighters; `nova_sim_ai::targeting::next_escort`), cloaked or not: Option-Tab.
    NextEscort,
    /// The candidate before the target (`nova_sim_ai::targeting::previous`): Shift-Tab.
    Previous,
    /// The ship of the escort class before the target
    /// (`nova_sim_ai::targeting::previous_escort`), cloaked or not: Shift-Option-Tab.
    PreviousEscort,
}
