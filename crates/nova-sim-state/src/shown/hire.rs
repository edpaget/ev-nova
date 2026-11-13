//! The bar's hire list as the player sees it ([`HireList`],
//! [`HireRow`]), why a ship cannot be hired ([`HireRefusal`]), what a hire
//! did ([`Hired`]), what paying the escorts did ([`PayNote`]) and the
//! hire dialog's strings, from `nova_sim_port::hire`, which re-exports
//! them and keeps the fee, the wage and the list's building.

use nova_sim_base::rulebook::Rulebook;
use nova_sim_catalog::catalog::ShipId;

use super::shipyard::ShipSpecs;

/// `STR#` 150 #13: the bar's button, and the hire dialog's.
pub const HIRE_ESCORT: &str = "Hire Escort";

/// `STR#` 2002 #224: what the bar says with nothing for hire.
pub const NONE_FOR_HIRE: &str = "There are no ships available for hire.";

/// `STR#` 2002 #228.
pub const HIRING_PRICE: &str = "Hiring Price:";

/// `STR#` 2002 #166: a hired escort's status in the comm dialog.
pub const HIRED_ESCORT: &str = "Hired Escort";

/// `STR#` 2002 #168: any other escort's status in the comm dialog.
pub const ESCORT: &str = "Escort";

/// `STR#` 2002 #297.
pub const PAY_LABEL: &str = "Pay:";

/// `STR#` 2002 #267.
pub const PER_DAY: &str = "per day";

/// `STR#` 2002 #302: one escort defected.
pub const DEFECTED_ONE: &str = "Due to lack of pay, one of your escorts has defected.";

/// `STR#` 2002 #303: more than one defected.
pub const DEFECTED_SOME: &str = "Due to lack of pay, some of your escorts have defected.";

/// Why a ship cannot be hired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HireRefusal {
    /// There is no bar: the ship is not landed, or the stellar has none.
    NoBar,
    /// The bar does not list the ship today.
    NotListed,
    /// It is listed but not for hire: its `Availability` does not hold, or, by
    /// the other reading of
    /// [`RuleKey::HireRequire`](crate::RuleKey::HireRequire), its `Require` is
    /// not met.
    NotForHire,
    /// The cash does not cover its fee.
    CannotAfford,
    /// The fleet holds as many escorts as it can.
    FleetFull,
    /// By the other reading of
    /// [`RuleKey::HireNoSlot`](crate::RuleKey::HireNoSlot), the system has no
    /// ship-table slot for it below the spawn reserve.
    NoSlot,
}

/// One ship class for hire in the bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HireRow {
    /// The ship class.
    pub id: ShipId,
    /// Its name.
    pub name: String,
    /// Its `ShortName`, raw.
    pub short_name: String,
    /// The fee for hiring it, shown and checked.
    pub fee: i64,
    /// Its daily wage once hired.
    pub wage: i64,
    /// What the Info panel shows of it.
    pub specs: ShipSpecs,
    /// Whether it can be hired now, or why not.
    pub hire: Result<(), HireRefusal>,
}

/// The ships for hire in the bar, as the player sees them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HireList {
    /// The ships listed, in `DispWeight` order.
    pub rows: Vec<HireRow>,
    /// The player's credits.
    pub cash: i64,
    /// Whether the fleet has room for another escort.
    pub room: bool,
    /// How each disputed rule reads, the bar's
    /// (`nova_sim_port::hire::Bar::rules`): the hire screen reads its grid's
    /// ([`RuleKey::GridSelection`](crate::RuleKey::GridSelection)) from it.
    pub rules: Rulebook,
}

impl HireList {
    /// `ship`'s row, if it is listed.
    #[must_use]
    pub fn row(&self, ship: ShipId) -> Option<&HireRow> {
        self.rows.iter().find(|row| row.id == ship)
    }

    /// Whether `ship` can be hired now, or why not.
    pub fn check(&self, ship: ShipId) -> Result<(), HireRefusal> {
        self.row(ship).ok_or(HireRefusal::NotListed)?.hire
    }
}

/// What hiring a ship did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hired {
    /// The ship class hired.
    pub ship: ShipId,
    /// What the hire took from the cash.
    pub fee: i64,
    /// The daily wage it was hired at.
    pub wage: i64,
    /// Whether an escort joined the fleet: not when, by the engine reading of
    /// [`RuleKey::HireNoSlot`](crate::RuleKey::HireNoSlot), it found no
    /// ship-table slot, the fee taken all the same.
    pub joined: bool,
}

/// What paying the escorts did, for the flight's message line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayNote {
    /// So many hired escorts defected for want of pay.
    Defected(u32),
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_base::fixture::FAST;
    use nova_sim_base::rulebook::RuleSource;

    #[test]
    fn a_list_reads_its_rows_by_ship() {
        let row = |id| HireRow {
            id: ShipId(id),
            name: String::new(),
            short_name: String::new(),
            fee: 0,
            wage: 0,
            specs: ShipSpecs {
                fields: FAST,
                max_gun: 0,
                max_tur: 0,
                length: 0,
                crew: 0,
                space: 0,
                rounding: RuleSource::Engine,
                weapons: Vec::new(),
                long_name: String::new(),
            },
            hire: if id == 129 {
                Err(HireRefusal::CannotAfford)
            } else {
                Ok(())
            },
        };
        let list = HireList {
            rows: vec![row(128), row(129)],
            cash: 0,
            room: true,
            rules: Rulebook::default(),
        };
        assert_eq!(list.row(ShipId(129)).map(|row| row.id), Some(ShipId(129)));
        assert_eq!(list.row(ShipId(130)), None);
        assert_eq!(list.check(ShipId(128)), Ok(()));
        assert_eq!(list.check(ShipId(129)), Err(HireRefusal::CannotAfford));
        assert_eq!(list.check(ShipId(130)), Err(HireRefusal::NotListed));
    }
}
