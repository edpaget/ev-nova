//! A flight session: the player's ship in its starting system, flown one
//! tick at a time.
//!
//! A session starts as a new pilot does, from the first `chär` by
//! ascending ID: its ship, in the first of its starting systems that
//! exists. The ship starts at rest at the system's centre, facing up, with
//! its shield, armour and fuel full.
//!
//! The ship lands on a stellar of its system when the
//! [`landing`](crate::landing) rules allow it: docked, it rests at the
//! stellar's centre and ticks move nothing until it takes off again, from
//! the same place.

use crate::catalog::{GovtId, LandingSite, PilotCatalog, ShipId, StartError, StellarId, SystemId};
use crate::flight::{Controls, ShipState, step};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::landing::{LandingRefusal, check_landing};
use crate::reserves::Reserves;

/// The player's ship, flying in one system.
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    ship: ShipId,
    system: SystemId,
    handling: Handling,
    player: ShipState,
    /// The system's stellars, read when the session starts.
    sites: Vec<LandingSite>,
    /// The stellar the ship is docked at, if it has landed.
    landed: Option<StellarId>,
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
            player: ShipState {
                reserves: Reserves::from_fields(fields),
                ..ShipState::default()
            },
            sites: catalog.landing_sites(system),
            landed: None,
        })
    }

    /// Advances the session one tick under the player's `controls`. A
    /// landed ship does not move.
    pub fn tick(&mut self, controls: Controls) {
        if self.landed.is_none() {
            step(&mut self.player, &self.handling, controls);
        }
    }

    /// Lands the ship on the stellar it is over, if the
    /// [`landing`](crate::landing) rules allow it: it docks at the
    /// stellar's centre, at rest, its heading and reserves unchanged.
    /// Otherwise it flies on, and the refusal says why.
    pub fn land(&mut self) -> Result<StellarId, LandingRefusal> {
        let stellar = check_landing(&self.player, &self.sites, self.legal_record())?;
        if let Some(site) = self.sites.iter().find(|site| site.id == stellar) {
            self.player.position = site.position;
        }
        self.player.velocity = Vec2::ZERO;
        self.landed = Some(stellar);
        Ok(stellar)
    }

    /// Takes off from the stellar the ship is docked at, and gives it; the
    /// ship flies again from the stellar's centre, at rest. `None`, and
    /// nothing changes, when it has not landed.
    pub fn take_off(&mut self) -> Option<StellarId> {
        self.landed.take()
    }

    /// The stellar the ship is docked at, if it has landed.
    #[must_use]
    pub fn landed(&self) -> Option<StellarId> {
        self.landed
    }

    /// The player's legal record in the system. Always 0: a new pilot's
    /// record is clean, and nothing changes it yet.
    #[must_use]
    pub fn legal_record(&self) -> i16 {
        0
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

    /// The government the player belongs to, if any. Always `None`: a new
    /// pilot belongs to no government. Nova grants membership later, through
    /// storyline play, and the first `chär` names none (its `Govt1-4` set
    /// starting legal records, not membership).
    #[must_use]
    pub fn government(&self) -> Option<GovtId> {
        None
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::catalog::{CharacterStart, LandingSite, StellarId};
    use crate::flight::Turn;
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::landing::{LandingRefusal, StellarFlags};
    use crate::reserves::{Gauge, Reserves};

    /// A canned first `chär`, ships and systems; records the ships asked
    /// for.
    struct FakePilotCatalog {
        character: Result<CharacterStart, StartError>,
        ships: Vec<(ShipId, Result<ShipFields, String>)>,
        systems: Vec<SystemId>,
        ships_asked: RefCell<Vec<ShipId>>,
        /// Each system's landing sites; any other has none.
        sites: Vec<(SystemId, Vec<LandingSite>)>,
        sites_asked: RefCell<Vec<SystemId>>,
    }

    const FAST: ShipFields = ShipFields {
        speed: 600,
        accel: 900,
        maneuver: 30,
        shield: 30,
        armor: 45,
        fuel: 300,
    };

    /// A landable planet at (`x`, `y`), 100 x 100 (radius 50).
    fn planet(id: i16, x: f32, y: f32) -> LandingSite {
        LandingSite {
            id: StellarId(id),
            position: Vec2::new(x, y),
            frame_size: Some((100, 100)),
            flags: StellarFlags::CAN_LAND,
            min_status: 0,
        }
    }

    /// The first `chär` flies ship 128 from system 130; ship 128 is fast,
    /// and systems 130 and 131 exist. System 130 holds a planet, 128, at
    /// (30, -40), which the ship starts over, and another, 129, far away;
    /// system 131 holds one at the centre.
    fn catalog() -> FakePilotCatalog {
        FakePilotCatalog {
            character: Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [Some(SystemId(130)), None, None, None],
            }),
            ships: vec![(ShipId(128), Ok(FAST))],
            systems: vec![SystemId(130), SystemId(131)],
            ships_asked: RefCell::default(),
            sites: vec![
                (
                    SystemId(130),
                    vec![planet(128, 30.0, -40.0), planet(129, 2000.0, 0.0)],
                ),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            sites_asked: RefCell::default(),
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

        fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
            self.sites_asked.borrow_mut().push(system);
            self.sites
                .iter()
                .find(|(id, _)| *id == system)
                .map_or_else(Vec::new, |(_, sites)| sites.clone())
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
                reserves: Reserves {
                    shield: Gauge::full(30.0),
                    armor: Gauge::full(45.0),
                    fuel: Gauge::full(300.0),
                },
            }
        );
    }

    #[test]
    fn a_new_pilot_belongs_to_no_government() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.government(), None);
    }

    #[test]
    fn a_tick_leaves_the_reserves_as_they_are() {
        let mut session = Session::start(&catalog()).expect("starts");
        let full = session.player().reserves;
        for _ in 0..30 {
            session.tick(Controls {
                thrust: true,
                ..Controls::default()
            });
        }
        assert_eq!(session.player().reserves, full);
        assert_ne!(session.player().position, Vec2::ZERO);
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

    // Landing.

    const THRUST: Controls = Controls {
        thrust: true,
        turn: Turn::None,
        reverse: false,
    };

    #[test]
    fn a_session_reads_its_systems_landing_sites_once() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(*catalog.sites_asked.borrow(), [SystemId(130)]);
        session.land().expect("lands");
        session.take_off();
        session.land().expect("lands again");
        assert_eq!(*catalog.sites_asked.borrow(), [SystemId(130)]);
        let other = starting([Some(131), None, None, None]);
        let mut session = Session::start(&other).expect("starts");
        assert_eq!(*other.sites_asked.borrow(), [SystemId(131)]);
        assert_eq!(session.land(), Ok(StellarId(140)));
    }

    #[test]
    fn a_new_pilots_record_is_clean() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.legal_record(), 0);
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn landing_docks_the_ship_at_the_stellar_at_rest() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(Controls {
            turn: Turn::Right,
            ..THRUST
        });
        let flying = *session.player();
        assert_ne!(flying.velocity, Vec2::ZERO);
        assert_eq!(session.land(), Ok(StellarId(128)));
        assert_eq!(session.landed(), Some(StellarId(128)));
        assert_eq!(
            *session.player(),
            ShipState {
                position: Vec2::new(30.0, -40.0),
                velocity: Vec2::ZERO,
                ..flying
            }
        );
    }

    #[test]
    fn a_tick_while_landed_moves_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.land().expect("lands");
        let docked = *session.player();
        for _ in 0..10 {
            session.tick(Controls {
                turn: Turn::Left,
                ..THRUST
            });
        }
        assert_eq!(*session.player(), docked);
    }

    #[test]
    fn taking_off_leaves_the_ship_at_the_stellar_at_rest_and_it_flies_again() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(Controls {
            turn: Turn::Right,
            ..THRUST
        });
        session.land().expect("lands");
        let docked = *session.player();
        assert_eq!(session.take_off(), Some(StellarId(128)));
        assert_eq!(session.landed(), None);
        assert_eq!(*session.player(), docked);
        assert_eq!(docked.position, Vec2::new(30.0, -40.0));
        assert_eq!(docked.velocity, Vec2::ZERO);
        session.tick(THRUST);
        let mut expected = docked;
        step(&mut expected, &session.handling(), THRUST);
        assert_eq!(*session.player(), expected);
        assert_ne!(expected, docked);
    }

    #[test]
    fn taking_off_without_landing_does_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(THRUST);
        let flying = *session.player();
        assert_eq!(session.take_off(), None);
        assert_eq!(*session.player(), flying);
        assert_eq!(session.take_off(), None, "nor twice");
    }

    #[test]
    fn a_refused_landing_is_its_refusal_and_the_ship_flies_on() {
        let mut session = Session::start(&catalog()).expect("starts");
        for _ in 0..20 {
            session.tick(THRUST);
        }
        let flying = *session.player();
        let refusal = session.land();
        assert_eq!(
            refusal,
            crate::landing::check_landing(
                &flying,
                &[planet(128, 30.0, -40.0), planet(129, 2000.0, 0.0)],
                0
            )
        );
        assert!(refusal.is_err(), "{refusal:?}");
        assert_eq!(session.landed(), None);
        assert_eq!(*session.player(), flying);
        session.tick(Controls::default());
        assert_ne!(*session.player(), flying, "still flying");

        let empty = FakePilotCatalog {
            sites: Vec::new(),
            ..catalog()
        };
        let mut session = Session::start(&empty).expect("starts");
        assert_eq!(session.land(), Err(LandingRefusal::NoStellars));
    }
}
