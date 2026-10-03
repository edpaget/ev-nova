//! A flight session: the player's ship in its starting system, flown one
//! tick at a time.
//!
//! A session starts as a new pilot does, from the first `chär` by
//! ascending ID: its ship, in the first of its starting systems that
//! exists. The ship starts at rest at the system's centre, facing up.

use crate::catalog::{PilotCatalog, ShipId, StartError, SystemId};
use crate::flight::{Controls, ShipState, step};
use crate::handling::Handling;

/// The player's ship, flying in one system.
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    ship: ShipId,
    system: SystemId,
    handling: Handling,
    player: ShipState,
}

impl Session {
    /// A new pilot's session, read from `catalog`.
    pub fn start(catalog: &impl PilotCatalog) -> Result<Self, StartError> {
        let character = catalog.first_character()?;
        let ship = character.ship.ok_or(StartError::NoShip)?;
        let fields = catalog
            .ship_fields(ship)
            .map_err(|reason| StartError::Ship(ship, reason))?;
        let system = character
            .systems
            .into_iter()
            .flatten()
            .find(|&id| catalog.system_exists(id))
            .ok_or(StartError::NoStartingSystem(character.systems))?;
        Ok(Self {
            ship,
            system,
            handling: Handling::from_fields(fields),
            player: ShipState::default(),
        })
    }

    /// Advances the session one tick under the player's `controls`.
    pub fn tick(&mut self, controls: Controls) {
        step(&mut self.player, &self.handling, controls);
    }

    /// The player's ship as it flies.
    #[must_use]
    pub fn player(&self) -> &ShipState {
        &self.player
    }

    /// The system the player is in.
    #[must_use]
    pub fn system(&self) -> SystemId {
        self.system
    }

    /// The player's ship class.
    #[must_use]
    pub fn ship(&self) -> ShipId {
        self.ship
    }

    /// How the player's ship flies.
    #[must_use]
    pub fn handling(&self) -> Handling {
        self.handling
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::catalog::CharacterStart;
    use crate::flight::Turn;
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;

    /// A canned first `chär`, ships and systems; records the ships asked
    /// for.
    struct FakePilotCatalog {
        character: Result<CharacterStart, StartError>,
        ships: Vec<(ShipId, Result<ShipFields, String>)>,
        systems: Vec<SystemId>,
        ships_asked: RefCell<Vec<ShipId>>,
    }

    const FAST: ShipFields = ShipFields {
        speed: 600,
        accel: 900,
        maneuver: 30,
    };

    /// The first `chär` flies ship 128 from system 130; ship 128 is fast,
    /// and systems 130 and 131 exist.
    fn catalog() -> FakePilotCatalog {
        FakePilotCatalog {
            character: Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [Some(SystemId(130)), None, None, None],
            }),
            ships: vec![(ShipId(128), Ok(FAST))],
            systems: vec![SystemId(130), SystemId(131)],
            ships_asked: RefCell::default(),
        }
    }

    fn starting(systems: [Option<i16>; 4]) -> FakePilotCatalog {
        FakePilotCatalog {
            character: Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: systems.map(|slot| slot.map(SystemId)),
            }),
            ..catalog()
        }
    }

    impl PilotCatalog for FakePilotCatalog {
        fn first_character(&self) -> Result<CharacterStart, StartError> {
            self.character.clone()
        }

        fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
            self.ships_asked.borrow_mut().push(id);
            self.ships
                .iter()
                .find(|(ship, _)| *ship == id)
                .map_or_else(|| Err(format!("no shïp {}", id.0)), |(_, f)| f.clone())
        }

        fn system_exists(&self, id: SystemId) -> bool {
            self.systems.contains(&id)
        }
    }

    #[test]
    fn a_session_flies_the_first_chärs_ship_with_its_handling() {
        let catalog = catalog();
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.ship(), ShipId(128));
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.handling(), Handling::from_fields(FAST));
        assert_eq!(*catalog.ships_asked.borrow(), [ShipId(128)]);
    }

    #[test]
    fn the_ship_starts_at_rest_at_the_centre_facing_up() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            *session.player(),
            ShipState {
                position: Vec2::ZERO,
                velocity: Vec2::ZERO,
                heading: 0.0,
            }
        );
    }

    #[test]
    fn the_first_system_that_exists_is_the_start() {
        let pick = |slots| Session::start(&starting(slots)).map(|s| s.system());
        assert_eq!(
            pick([None, Some(999), Some(131), Some(130)]),
            Ok(SystemId(131))
        );
        assert_eq!(pick([Some(131), Some(130), None, None]), Ok(SystemId(131)));
        assert_eq!(pick([None, None, None, Some(130)]), Ok(SystemId(130)));
    }

    #[test]
    fn no_system_that_exists_is_an_error_naming_the_slots() {
        let slots = [None, Some(999), None, Some(-5)];
        assert_eq!(
            Session::start(&starting(slots)),
            Err(StartError::NoStartingSystem(slots.map(|s| s.map(SystemId))))
        );
        assert_eq!(
            Session::start(&starting([None; 4])),
            Err(StartError::NoStartingSystem([None; 4]))
        );
    }

    #[test]
    fn a_missing_or_broken_chär_is_its_error() {
        for error in [
            StartError::NoCharacter,
            StartError::Character("chär 128: too short".to_owned()),
        ] {
            let catalog = FakePilotCatalog {
                character: Err(error.clone()),
                ..catalog()
            };
            assert_eq!(Session::start(&catalog), Err(error));
            assert_eq!(*catalog.ships_asked.borrow(), []);
        }
    }

    #[test]
    fn a_chär_without_a_ship_is_an_error() {
        let catalog = FakePilotCatalog {
            character: Ok(CharacterStart {
                ship: None,
                systems: [Some(SystemId(130)), None, None, None],
            }),
            ..catalog()
        };
        assert_eq!(Session::start(&catalog), Err(StartError::NoShip));
    }

    #[test]
    fn an_unreadable_ship_is_an_error_with_its_reason() {
        let catalog = FakePilotCatalog {
            ships: Vec::new(),
            ..catalog()
        };
        assert_eq!(
            Session::start(&catalog),
            Err(StartError::Ship(ShipId(128), "no shïp 128".to_owned()))
        );
    }

    #[test]
    fn a_tick_steps_the_player_under_the_controls() {
        let mut session = Session::start(&catalog()).expect("starts");
        let controls = Controls {
            thrust: true,
            turn: Turn::Right,
            reverse: false,
        };
        let mut expected = *session.player();
        for _ in 0..5 {
            session.tick(controls);
            step(&mut expected, &session.handling(), controls);
        }
        assert_eq!(*session.player(), expected);
        assert_ne!(expected, ShipState::default());
    }
}
