//! Which own-ship path a board of the player's own disabled escort or
//! fighter takes ([`Recovery`]), from `nova_sim_port::board_repair`,
//! which re-exports it and keeps the choice among the paths.

use super::message::SimMessage;

/// Which own-ship path a board takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    /// "Escort repaired." (`STR#` 2002 #127, @0x65cff): the ship rejoins
    /// the fleet.
    EscortRepaired,
    /// "Fighter repaired." (#128, @0x65af6, @0x65e29): the player's own
    /// fighter docks in its bay.
    FighterRepaired,
    /// "Fighter captured." (#129, @0x65e29): a ship of a type the player's
    /// bays carry docks in its bay.
    FighterCaptured,
}

impl Recovery {
    /// What the player is told.
    #[must_use]
    pub fn message(self) -> SimMessage {
        match self {
            Self::EscortRepaired => SimMessage::EscortRepaired,
            Self::FighterRepaired => SimMessage::FighterRepaired,
            Self::FighterCaptured => SimMessage::FighterCaptured,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_recovery_says_its_line() {
        assert_eq!(
            Recovery::EscortRepaired.message(),
            SimMessage::EscortRepaired
        );
        assert_eq!(
            Recovery::FighterRepaired.message(),
            SimMessage::FighterRepaired
        );
        assert_eq!(
            Recovery::FighterCaptured.message(),
            SimMessage::FighterCaptured
        );
    }
}
