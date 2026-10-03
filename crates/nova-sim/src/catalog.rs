//! The pilot catalog port: what a flight session starts from, in the
//! simulation's own terms.

use std::rc::Rc;

pub use nova_data::{ShipId, SystemId};

use crate::handling::ShipFields;

/// A new pilot's start, from the first `chär`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterStart {
    /// The starting `shïp`, if it names one.
    pub ship: Option<ShipId>,
    /// The starting `sÿst`s, in order; any may be unused.
    pub systems: [Option<SystemId>; 4],
}

/// Why a flight session could not start. Each message is ready to display.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StartError {
    /// There is no `chär` at all.
    #[error("no chär to start from")]
    NoCharacter,
    /// The first `chär` does not decode; the decoder's message.
    #[error("{0}")]
    Character(String),
    /// The first `chär` names no ship.
    #[error("the first chär has no ship")]
    NoShip,
    /// The starting ship cannot be read; why.
    #[error("{1}")]
    Ship(ShipId, String),
    /// None of the first `chär`'s starting systems exists.
    #[error("none of the first chär's starting systems ({}) exists", slots(.0))]
    NoStartingSystem([Option<SystemId>; 4]),
}

/// The starting system slots as text: each ID, or "none".
fn slots(systems: &[Option<SystemId>; 4]) -> String {
    systems
        .iter()
        .map(|slot| slot.map_or_else(|| "none".to_owned(), |id| id.0.to_string()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The game data a flight session starts from.
pub trait PilotCatalog {
    /// The first `chär` by ascending ID: its ship and starting systems.
    fn first_character(&self) -> Result<CharacterStart, StartError>;
    /// Ship `id`'s handling fields, or why they cannot be read.
    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String>;
    /// Whether system `id` exists and can be read.
    fn system_exists(&self, id: SystemId) -> bool;
}

/// A borrowed catalog is a catalog.
impl<T: PilotCatalog + ?Sized> PilotCatalog for &T {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        (**self).first_character()
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        (**self).ship_fields(id)
    }

    fn system_exists(&self, id: SystemId) -> bool {
        (**self).system_exists(id)
    }
}

/// A shared catalog is a catalog, so the sim and the views can read the
/// same game data.
impl<T: PilotCatalog + ?Sized> PilotCatalog for Rc<T> {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        (**self).first_character()
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        (**self).ship_fields(id)
    }

    fn system_exists(&self, id: SystemId) -> bool {
        (**self).system_exists(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ship 128 is average; system 130 alone exists.
    struct One;

    impl PilotCatalog for One {
        fn first_character(&self) -> Result<CharacterStart, StartError> {
            Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [Some(SystemId(130)), None, None, None],
            })
        }

        fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
            (id == ShipId(128))
                .then_some(ShipFields {
                    speed: 300,
                    accel: 300,
                    maneuver: 10,
                })
                .ok_or_else(|| format!("no shïp {}", id.0))
        }

        fn system_exists(&self, id: SystemId) -> bool {
            id == SystemId(130)
        }
    }

    /// Everything `catalog` says about ships 128 and 129 and systems 130
    /// and 131.
    fn reads(catalog: impl PilotCatalog) -> Vec<String> {
        vec![
            format!("{:?}", catalog.first_character()),
            format!("{:?}", catalog.ship_fields(ShipId(128))),
            format!("{:?}", catalog.ship_fields(ShipId(129))),
            format!("{}", catalog.system_exists(SystemId(130))),
            format!("{}", catalog.system_exists(SystemId(131))),
        ]
    }

    #[test]
    fn borrowed_and_shared_catalogs_are_catalogs() {
        let direct = reads(One);
        assert!(direct[1].contains("speed: 300"), "{direct:?}");
        assert_eq!(direct[2], r#"Err("no shïp 129")"#);
        assert_eq!(direct[3..], ["true", "false"]);
        assert_eq!(reads(&One), direct);
        assert_eq!(reads(Rc::new(One)), direct);
    }

    #[test]
    fn each_error_reads_as_a_sentence() {
        assert_eq!(StartError::NoCharacter.to_string(), "no chär to start from");
        assert_eq!(
            StartError::Character("chär 128: too short".to_owned()).to_string(),
            "chär 128: too short"
        );
        assert_eq!(StartError::NoShip.to_string(), "the first chär has no ship");
        assert_eq!(
            StartError::Ship(ShipId(128), "no shïp 128".to_owned()).to_string(),
            "no shïp 128"
        );
        assert_eq!(
            StartError::NoStartingSystem([None, Some(SystemId(999)), None, Some(SystemId(5))])
                .to_string(),
            "none of the first chär's starting systems (none, 999, none, 5) exists"
        );
    }
}
