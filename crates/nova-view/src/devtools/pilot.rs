//! The pilot desk port: what the pilot editor reads and edits of the pilot
//! flying, in the developer tools' own terms.
//!
//! The editor never sees the session: it reads a [`PilotSheet`], lists the
//! [`Place`]s it can move the pilot to, and sends one [`PilotEdit`] at a
//! time, which the desk carries out or refuses ([`EditRefusal`]). The
//! adapter over a live session is [`super::session`].

use std::fmt;

use nova_sim::catalog::{StellarId, SystemId};
use nova_sim::{Bit, GameDate, Gauge, Reserve};

/// The pilot flying, as the editor shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct PilotSheet {
    /// The pilot's name.
    pub name: String,
    /// Its cash.
    pub credits: i64,
    /// The ship's shield.
    pub shield: Gauge,
    /// The ship's armour.
    pub armor: Gauge,
    /// The ship's fuel.
    pub fuel: Gauge,
    /// The date.
    pub date: GameDate,
    /// The date as the game writes it.
    pub date_text: String,
    /// The system the pilot is in.
    pub system: Place<SystemId>,
    /// The stellar it is landed on; none in flight.
    pub landed: Option<Place<StellarId>>,
    /// The control bits set, ascending.
    pub bits: Vec<Bit>,
}

impl PilotSheet {
    /// The gauge of `reserve`.
    #[must_use]
    pub fn gauge(&self, reserve: Reserve) -> Gauge {
        match reserve {
            Reserve::Shield => self.shield,
            Reserve::Armor => self.armor,
            Reserve::Fuel => self.fuel,
        }
    }
}

/// A system or stellar, by ID and name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place<Id> {
    /// Its ID.
    pub id: Id,
    /// Its name.
    pub name: String,
}

impl Place<SystemId> {
    /// How a list shows it: its name, then its ID, e.g. "Sol (130)".
    #[must_use]
    pub fn label(&self) -> String {
        format!("{} ({})", self.name, self.id.0)
    }
}

impl Place<StellarId> {
    /// How a list shows it: its name, then its ID, e.g. "Earth (128)".
    #[must_use]
    pub fn label(&self) -> String {
        format!("{} ({})", self.name, self.id.0)
    }
}

/// One change to the pilot flying.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PilotEdit {
    /// Set the cash.
    Credits(i64),
    /// Set a reserve, which the desk holds within its gauge.
    Reserve(Reserve, f32),
    /// Set the date.
    Date(GameDate),
    /// Move the landed pilot to a stellar of a system, and dock there.
    MoveTo {
        /// The system.
        system: SystemId,
        /// The stellar.
        stellar: StellarId,
    },
    /// Set a control bit when `true`, clear it when `false`.
    Bit(Bit, bool),
}

/// Why the desk refused an edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditRefusal {
    /// The ship must be landed to move.
    InFlight,
    /// The system does not exist.
    NoSystem,
    /// The stellar is not one of the system's.
    NoStellar,
    /// The stellar is a gate, or cannot be landed on.
    NotLandable,
}

impl fmt::Display for EditRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InFlight => "the ship must be landed to move",
            Self::NoSystem => "no such system",
            Self::NoStellar => "no such stellar in that system",
            Self::NotLandable => "that stellar cannot be landed on",
        })
    }
}

/// The pilot flying, to read and edit.
pub trait PilotDesk {
    /// The pilot flying; `None` when there is none.
    fn sheet(&self) -> Option<PilotSheet>;

    /// Every system the pilot can be moved to, in ID order.
    fn systems(&self) -> Vec<Place<SystemId>>;

    /// The stellars of `system`; none for a system that is not there.
    fn stellars(&self, system: SystemId) -> Vec<Place<StellarId>>;

    /// Carries out `edit`.
    ///
    /// # Errors
    ///
    /// Why the edit was refused, when it was; then nothing changed.
    fn edit(&mut self, edit: PilotEdit) -> Result<(), EditRefusal>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_refusal_says_why() {
        assert_eq!(
            EditRefusal::InFlight.to_string(),
            "the ship must be landed to move"
        );
        assert_eq!(EditRefusal::NoSystem.to_string(), "no such system");
        assert_eq!(
            EditRefusal::NoStellar.to_string(),
            "no such stellar in that system"
        );
        assert_eq!(
            EditRefusal::NotLandable.to_string(),
            "that stellar cannot be landed on"
        );
    }

    #[test]
    fn a_place_is_labelled_by_name_and_id() {
        let system = Place {
            id: SystemId(130),
            name: "Sol".to_owned(),
        };
        assert_eq!(system.label(), "Sol (130)");
        let stellar = Place {
            id: StellarId(128),
            name: "Earth".to_owned(),
        };
        assert_eq!(stellar.label(), "Earth (128)");
    }

    #[test]
    fn a_sheets_gauge_is_its_reserves() {
        let gauge = |now| Gauge { now, max: 100.0 };
        let sheet = PilotSheet {
            name: String::new(),
            credits: 0,
            shield: gauge(1.0),
            armor: gauge(2.0),
            fuel: gauge(3.0),
            date: GameDate::new(1177, 6, 23).expect("a date"),
            date_text: String::new(),
            system: Place {
                id: SystemId(130),
                name: String::new(),
            },
            landed: None,
            bits: Vec::new(),
        };
        assert_eq!(sheet.gauge(Reserve::Shield), gauge(1.0));
        assert_eq!(sheet.gauge(Reserve::Armor), gauge(2.0));
        assert_eq!(sheet.gauge(Reserve::Fuel), gauge(3.0));
    }
}
