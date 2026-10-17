//! The player's fleet as data: each ship in it ([`Escort`]), what a save
//! keeps of it, and the standing order it follows ([`EscortOrder`]).
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
