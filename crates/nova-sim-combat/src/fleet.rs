//! The player's fleet as data: each ship in it ([`Escort`]), what a save
//! keeps of it, and the standing order it follows ([`EscortOrder`]); and
//! the order word a ship an NPC leads holds ([`OrderWord`]).
//!
//! How the escorts fly, take orders and keep formation is
//! `nova_sim::escort`'s; the pilot keeps the fleet (`nova_sim::Pilot`).

use crate::catalog::{PersonId, ShipId};
use crate::reserves::Reserves;

/// A ship in the player's fleet: its class, its shield, armour and fuel,
/// and the standing order it follows. In flight the session flies it as
/// an NPC beside the player (`nova_sim::Session`); the record keeps
/// what a save keeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Escort {
    /// The escort's ship class.
    pub ship: ShipId,
    /// Its shield, armour and fuel.
    pub reserves: Reserves,
    /// Its standing order; none keeps formation (see
    /// `nova_sim::escort`).
    pub order: Option<EscortOrder>,
    /// Whether it is a fighter launched from one of the player's bays,
    /// out of its bay (see [`bay`](crate::bay)), rather than an escort of
    /// its own.
    pub carried: bool,
    /// The daily wage it was hired at in the bar, which may be none; `None`
    /// for an escort that was not hired (a ship captured, or the old ship
    /// kept after "Use As My Ship"). Which wage a hired escort is paid
    /// each day follows `nova_sim::Session::with_rules` (see
    /// `nova_sim::hire`).
    pub wage: Option<i64>,
    /// The person (`përs`) this escort is, flying as itself, its name,
    /// ship and loadout its record's (see `nova_sim::person`);
    /// none for any other escort. One whose person no longer has a record
    /// flies as an ordinary escort of its ship class.
    pub person: Option<PersonId>,
}

impl Escort {
    /// Whether it was hired in the bar: it has a wage, even of none.
    #[must_use]
    pub fn hired(&self) -> bool {
        self.wage.is_some()
    }
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
    /// its carrier (see [`bay`](crate::bay)); any other escort keeps
    /// formation.
    Dock,
}

/// The order word a ship led by an NPC holds, the original's +0xc90a
/// (see `nova_sim_ai::ai::escort_orders`): -1 ([`OrderWord::NONE`]) at
/// every spawn, launch and placement, then whatever its leader's
/// `_AIIssueEscortOrders` last wrote (1 Defend, 2 Attack, 3 Return, 4
/// Hold, 0 formation). Kept raw, as a stray beam of `ExitType` 6 steps it
/// as a cursor and tells -1 from 0. Never saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrderWord(pub i16);

impl OrderWord {
    /// -1: no order yet, as every spawn writes.
    pub const NONE: Self = Self(-1);

    /// The word `order` is written as: 1 Defend, 2 Attack, 3 Return, 4
    /// Hold, and 0 for formation (none).
    #[must_use]
    pub fn of(order: Option<EscortOrder>) -> Self {
        Self(match order {
            None => 0,
            Some(EscortOrder::Defend) => 1,
            Some(EscortOrder::Attack) => 2,
            Some(EscortOrder::Dock) => 3,
            Some(EscortOrder::Hold) => 4,
        })
    }

    /// The order the word gives: 1 Defend, 2 Attack, 3 Return, 4 Hold;
    /// any other word formation (none).
    #[must_use]
    pub fn order(self) -> Option<EscortOrder> {
        match self.0 {
            1 => Some(EscortOrder::Defend),
            2 => Some(EscortOrder::Attack),
            3 => Some(EscortOrder::Dock),
            4 => Some(EscortOrder::Hold),
            _ => None,
        }
    }
}

impl Default for OrderWord {
    /// [`OrderWord::NONE`].
    fn default() -> Self {
        Self::NONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_order_word_gives_1_to_4_as_orders_and_anything_else_as_formation() {
        let order = |word| OrderWord(word).order();
        assert_eq!(order(1), Some(EscortOrder::Defend));
        assert_eq!(order(2), Some(EscortOrder::Attack));
        assert_eq!(order(3), Some(EscortOrder::Dock));
        assert_eq!(order(4), Some(EscortOrder::Hold));
        for word in [-1, 0, 5, i16::MIN, i16::MAX] {
            assert_eq!(order(word), None, "{word}");
        }
        assert_eq!(OrderWord::default(), OrderWord(-1));
        assert_eq!(OrderWord::NONE, OrderWord(-1));
    }

    #[test]
    fn an_order_is_written_as_its_word_and_formation_as_0() {
        assert_eq!(OrderWord::of(None), OrderWord(0));
        for (order, word) in [
            (EscortOrder::Defend, 1),
            (EscortOrder::Attack, 2),
            (EscortOrder::Dock, 3),
            (EscortOrder::Hold, 4),
        ] {
            assert_eq!(OrderWord::of(Some(order)), OrderWord(word));
            assert_eq!(OrderWord(word).order(), Some(order));
        }
    }
}
