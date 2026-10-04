//! An NPC ship: what it is, how it performs, where it is and what it is
//! doing.

use crate::ai::Goal;
use crate::catalog::{GovtId, ShipId};
use crate::flight::ShipState;
use crate::stats::ShipStats;

/// An NPC's number in its system, unique while the player stays there:
/// the order it appeared in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NpcId(pub u32);

/// How an NPC behaves: the `düde` `AIType`, or a `shïp`'s `InherentAI`
/// (the Bible: 1 wimpy trader, 2 brave trader, 3 warship, 4 interceptor).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiType {
    /// 1: a trader that flees.
    WimpyTrader,
    /// 2: a trader that fights back.
    BraveTrader,
    /// 3: a warship.
    Warship,
    /// 4: an interceptor.
    Interceptor,
}

impl AiType {
    /// What a raw AI type other than 1-4 means: a wimpy trader, the most
    /// harmless, which lands rather than lingers. Stock ships all have an
    /// `InherentAI` of 1-4; this covers plug-in data that leaves it 0 or
    /// -1, where the original's behaviour has not been traced.
    pub const FALLBACK: Self = Self::WimpyTrader;

    /// The AI type `raw` means: 1-4 as the Bible says, anything else
    /// [`AiType::FALLBACK`].
    #[must_use]
    pub fn from_raw(raw: i16) -> Self {
        match raw {
            2 => Self::BraveTrader,
            3 => Self::Warship,
            4 => Self::Interceptor,
            _ => Self::FALLBACK,
        }
    }

    /// Whether it is a trader, wimpy or brave.
    #[must_use]
    pub fn trades(self) -> bool {
        matches!(self, Self::WimpyTrader | Self::BraveTrader)
    }
}

/// Whether an NPC is still coming out of hyperspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Gliding in from hyperspace, for this many more ticks: no steering
    /// and no decisions.
    JumpingIn {
        /// The ticks of the glide left.
        ticks_left: u32,
    },
    /// Flying under its own controls.
    Flying,
}

/// An NPC ship in the player's system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Npc {
    /// Its number.
    pub id: NpcId,
    /// Its ship type.
    pub ship: ShipId,
    /// Its government, or `None` for independent.
    pub govt: Option<GovtId>,
    /// How it behaves.
    pub ai_type: AiType,
    /// The fleet lead it escorts, if it is an escort.
    pub leader: Option<NpcId>,
    /// How it performs, its default items included.
    pub stats: ShipStats,
    /// Its fuel; 100 is one jump.
    pub fuel: f32,
    /// Where it is and how it moves.
    pub state: ShipState,
    /// Whether it is still jumping in.
    pub mode: Mode,
    /// What it is doing.
    pub goal: Goal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_ai_types_1_to_4_are_the_bibles_and_any_other_is_the_fallback() {
        assert_eq!(
            [1, 2, 3, 4].map(AiType::from_raw),
            [
                AiType::WimpyTrader,
                AiType::BraveTrader,
                AiType::Warship,
                AiType::Interceptor
            ]
        );
        for raw in [0, -1, 5, i16::MIN, i16::MAX] {
            assert_eq!(AiType::from_raw(raw), AiType::FALLBACK, "{raw}");
        }
        assert_eq!(AiType::FALLBACK, AiType::WimpyTrader);
    }

    #[test]
    fn only_the_traders_trade() {
        assert!(AiType::WimpyTrader.trades());
        assert!(AiType::BraveTrader.trades());
        assert!(!AiType::Warship.trades());
        assert!(!AiType::Interceptor.trades());
    }
}
