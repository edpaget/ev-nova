//! What an escort is marked for at the next shipyard ([`EscortMark`]) and
//! the standing order it follows ([`EscortOrder`]), from
//! `nova_sim_combat::fleet`, which re-exports them and keeps the fleet's
//! records and the order word an NPC holds.

/// What an escort is marked for at the next shipyard (`_DoEscortLand`
/// @0x4031f settles it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscortMark {
    /// Upgraded to its class's `UpgradeTo` for its `EscUpgrdCost`.
    Upgrade,
    /// Sold for its class's `EscSellValue`.
    Sell,
}

/// A standing order an escort follows. An escort with none keeps
/// formation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscortOrder {
    /// 1: it defends the player.
    Defend,
    /// 2: it attacks the player's target.
    Attack,
    /// 4: it holds position.
    Hold,
    /// 3: Return to Hangar: a carried fighter flies back and docks with
    /// its carrier (see `nova_sim_combat::bay`); any other escort keeps
    /// formation.
    Dock,
}
