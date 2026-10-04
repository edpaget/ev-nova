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
//! the same place. A landed ship cannot jump, nor a jumping one land.
//!
//! The player plots a course to a system on the star map, read once when
//! the session starts: the fewest jumps along the hyperlinks. A jump to
//! the next system on it begins when the [`hyperspace`](crate::hyperspace)
//! rules allow, and while it lasts ticks move nothing. When the jump is
//! over ([`Session::arrive`]) the ship is in the next system, at its edge,
//! with a jump's fuel used and a day gone by, and the rest of the course
//! still ahead. In flight, fuel regenerates each tick at the rate the ship
//! and its default outfits give, read when the session starts.
//!
//! As it goes the session emits [`SimSound`] events (thrust starting and
//! stopping, landing, taking off, a jump beginning and ending), which the
//! audio side drains with [`Session::take_sounds`]. A refused landing or
//! jump emits nothing.

use crate::catalog::{GovtId, LandingSite, PilotCatalog, ShipId, StartError, StellarId, SystemId};
use crate::date::GameDate;
use crate::flight::{Controls, ShipState, step};
use crate::fuel::{fuel_regen_per_tick, regenerate};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::hyperspace::{
    DAYS_PER_JUMP, JUMP_FUEL, JumpRefusal, RouteError, StarMap, arrival, check_jump,
};
use crate::landing::{LandingRefusal, check_landing};
use crate::pilot::Pilot;
use crate::reserves::Reserves;
use crate::sound::SimSound;

/// The player's ship, flying in one system.
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    /// The pilot flying: everything a save keeps.
    pilot: Pilot,
    handling: Handling,
    player: ShipState,
    /// The system's stellars, read when the session starts.
    sites: Vec<LandingSite>,
    /// The stellar the ship is docked at, if it has landed.
    landed: Option<StellarId>,
    /// The star map, read when the session starts.
    star_map: StarMap,
    /// The system being jumped to, while a jump is under way.
    jumping: Option<SystemId>,
    /// The fuel gained each tick in flight, from the ship and its default
    /// outfits.
    fuel_regen: f32,
    /// Whether the ship is thrusting, as the last sounds told it.
    thrusting: bool,
    /// The sounds emitted since they were last taken.
    sounds: Vec<SimSound>,
}

impl Session {
    /// A new pilot's session, read from `catalog`: an unnamed
    /// [`Pilot::new`], flown.
    pub fn start(catalog: &impl PilotCatalog) -> Result<Self, StartError> {
        Self::fly(catalog, Pilot::new(catalog, "")?)
    }

    /// `pilot`'s session, its ship's handling, its system's stellars, the
    /// star map and its fuel regeneration read from `catalog`. The ship
    /// starts at rest at the system's centre, facing up.
    pub fn fly(catalog: &impl PilotCatalog, pilot: Pilot) -> Result<Self, StartError> {
        let ship = pilot.ship;
        let fields = catalog
            .ship_fields(ship)
            .map_err(|reason| StartError::Ship(ship, reason))?;
        Ok(Self {
            handling: Handling::from_fields(fields),
            player: ShipState::default(),
            sites: catalog.landing_sites(pilot.system),
            landed: None,
            star_map: StarMap::new(catalog.star_map()),
            jumping: None,
            // No outfits yet: outfitting will pass the ship's.
            fuel_regen: fuel_regen_per_tick(fields.fuel_regen, &catalog.default_outfits(ship)),
            thrusting: false,
            sounds: Vec::new(),
            pilot,
        })
    }

    /// Advances the session one tick under the player's `controls`, then
    /// regenerates fuel. A landed ship, or one jumping, does not move, and
    /// gains no fuel.
    pub fn tick(&mut self, controls: Controls) {
        if self.landed.is_none() && self.jumping.is_none() {
            if controls.thrust != self.thrusting {
                self.thrusting = controls.thrust;
                self.sounds.push(if controls.thrust {
                    SimSound::ThrustStarted
                } else {
                    SimSound::ThrustStopped
                });
            }
            step(&mut self.player, &self.handling, controls);
            regenerate(&mut self.pilot.reserves.fuel, self.fuel_regen);
        }
    }

    /// Stops the thrust, if the ship was thrusting, as it lands or jumps.
    fn stop_thrust(&mut self) {
        if self.thrusting {
            self.thrusting = false;
            self.sounds.push(SimSound::ThrustStopped);
        }
    }

    /// Plots a course from the system the ship is in to `to`, replacing any
    /// course, and gives it. When there is no route the course is cleared.
    pub fn plot_course(&mut self, to: SystemId) -> Result<&[SystemId], RouteError> {
        match self.star_map.route(self.pilot.system, to) {
            Ok(route) => {
                self.pilot.course = route;
                Ok(&self.pilot.course)
            }
            Err(error) => {
                self.pilot.course.clear();
                Err(error)
            }
        }
    }

    /// The systems still to jump to, in order, ending at the destination;
    /// none when no course is plotted or the destination has been reached.
    #[must_use]
    pub fn course(&self) -> &[SystemId] {
        &self.pilot.course
    }

    /// Begins a jump to the next system on the course, if the ship has not
    /// landed and the [`hyperspace`](crate::hyperspace) rules allow it, and
    /// gives that system; otherwise the refusal says why. Until it arrives,
    /// ticks move nothing.
    pub fn begin_jump(&mut self) -> Result<SystemId, JumpRefusal> {
        if self.landed.is_some() {
            return Err(JumpRefusal::Landed);
        }
        let next = check_jump(
            &self.player,
            self.pilot.reserves.fuel.now,
            self.pilot.course.first().copied(),
        )?;
        self.jumping = Some(next);
        self.stop_thrust();
        self.sounds.push(SimSound::JumpBegan);
        Ok(next)
    }

    /// The system being jumped to, while a jump is under way.
    #[must_use]
    pub fn jumping(&self) -> Option<SystemId> {
        self.jumping
    }

    /// Ends the jump under way, if any, and gives the system arrived in:
    /// the jump's fuel is used, the date advances, the system is taken off
    /// the course, and the ship is placed at its edge facing the system it
    /// came from (see [`arrival`]) with its reserves as they were. The new
    /// system's stellars are read from `catalog`. `None`, and nothing
    /// changes, when no jump is under way.
    pub fn arrive(&mut self, catalog: &impl PilotCatalog) -> Option<SystemId> {
        let next = self.jumping.take()?;
        let pilot = &mut self.pilot;
        pilot.reserves.fuel.now -= JUMP_FUEL;
        for _ in 0..DAYS_PER_JUMP {
            pilot.date = pilot.date.next_day();
        }
        if pilot.course.first() == Some(&next) {
            pilot.course.remove(0);
        }
        let map = |id| self.star_map.position(id).unwrap_or_default();
        self.player = arrival(map(pilot.system), map(next), &self.handling);
        pilot.system = next;
        self.sites = catalog.landing_sites(next);
        self.sounds.push(SimSound::Arrived);
        Some(next)
    }

    /// The sounds emitted since they were last taken, in order; taking
    /// them empties the list.
    pub fn take_sounds(&mut self) -> Vec<SimSound> {
        std::mem::take(&mut self.sounds)
    }

    /// The star map, as read when the session started.
    #[must_use]
    pub fn star_map(&self) -> &StarMap {
        &self.star_map
    }

    /// Today's date.
    #[must_use]
    pub fn date(&self) -> GameDate {
        self.pilot.date
    }

    /// The fuel the ship gains each tick in flight.
    #[must_use]
    pub fn fuel_regen_per_tick(&self) -> f32 {
        self.fuel_regen
    }

    /// Lands the ship on the stellar it is over, if it is not jumping and
    /// the [`landing`](crate::landing) rules allow it: it docks at the
    /// stellar's centre, at rest, its heading and reserves unchanged.
    /// Otherwise it flies on, and the refusal says why.
    pub fn land(&mut self) -> Result<StellarId, LandingRefusal> {
        if self.jumping.is_some() {
            return Err(LandingRefusal::Jumping);
        }
        let stellar = check_landing(&self.player, &self.sites, self.legal_record())?;
        let site = self.sites.iter().find(|site| site.id == stellar);
        if let Some(site) = site {
            self.player.position = site.position;
        }
        let stellar_sound = site.and_then(|site| site.landing_sound);
        self.player.velocity = Vec2::ZERO;
        self.landed = Some(stellar);
        self.stop_thrust();
        self.sounds.push(SimSound::Landed { stellar_sound });
        Ok(stellar)
    }

    /// Takes off from the stellar the ship is docked at, and gives it; the
    /// ship flies again from the stellar's centre, at rest. `None`, and
    /// nothing changes, when it has not landed.
    pub fn take_off(&mut self) -> Option<StellarId> {
        let stellar = self.landed.take()?;
        self.sounds.push(SimSound::TookOff);
        Some(stellar)
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

    /// The pilot flying.
    #[must_use]
    pub fn pilot(&self) -> &Pilot {
        &self.pilot
    }

    /// The ship's shield, armour and fuel.
    #[must_use]
    pub fn reserves(&self) -> Reserves {
        self.pilot.reserves
    }

    /// The system the player is in.
    #[must_use]
    pub fn system(&self) -> SystemId {
        self.pilot.system
    }

    /// The player's ship class.
    #[must_use]
    pub fn ship(&self) -> ShipId {
        self.pilot.ship
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
    use super::*;
    use crate::catalog::{CharacterStart, LandingSite, SoundId, StellarId};
    use crate::flight::Turn;
    use crate::fuel::{FUEL_SCOOP, OutfitMod};
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::hyperspace::{JumpRefusal, MIN_JUMP_DISTANCE, RouteError, StarMap};
    use crate::landing::LandingRefusal;
    use crate::reserves::{Gauge, Reserves};
    use crate::testkit::{FAST, FakePilotCatalog, START, catalog, planet, starting};

    #[test]
    fn a_session_flies_the_first_chärs_ship_with_its_handling() {
        let catalog = catalog();
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.ship(), ShipId(128));
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.handling(), Handling::from_fields(FAST));
        let asked = catalog.ships_asked.borrow();
        assert!(
            !asked.is_empty() && asked.iter().all(|&ship| ship == ShipId(128)),
            "{asked:?}"
        );
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
        assert_eq!(
            session.reserves(),
            Reserves {
                shield: Gauge::full(30.0),
                armor: Gauge::full(45.0),
                fuel: Gauge::full(300.0),
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
        let full = session.reserves();
        for _ in 0..30 {
            session.tick(Controls {
                thrust: true,
                ..Controls::default()
            });
        }
        assert_eq!(session.reserves(), full);
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
                start: START,
                ..CharacterStart::default()
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

    // Hyperspace.

    fn ids(route: &[i16]) -> Vec<SystemId> {
        route.iter().copied().map(SystemId).collect()
    }

    fn dmy(session: &Session) -> (u8, u8, i32) {
        let date = session.date();
        (date.day(), date.month(), date.year())
    }

    /// Flies the ship out from the centre until it is at least
    /// [`MIN_JUMP_DISTANCE`] away: it first turns to face away from the
    /// centre (Down faces against its motion), then thrusts.
    fn fly_out(session: &mut Session) {
        for _ in 0..3000 {
            let player = *session.player();
            if player.position.length() >= MIN_JUMP_DISTANCE {
                return;
            }
            let outward = if player.position.length() > 0.0 {
                crate::flight::heading_of(player.position)
            } else {
                0.0
            };
            let off = crate::flight::shortest_turn(player.heading, outward).abs();
            // Down lands exactly on the heading against the motion, which
            // is outward while the ship drifts in.
            let controls = if off < 1e-3 {
                THRUST
            } else if player.velocity.length() > crate::flight::AT_REST_SPEED {
                Controls {
                    reverse: true,
                    ..Controls::default()
                }
            } else {
                Controls {
                    turn: Turn::Right,
                    ..Controls::default()
                }
            };
            session.tick(controls);
        }
        panic!("never got out: {:?}", session.player());
    }

    /// Plots a course to `to`, flies out and jumps, and arrives.
    fn jump(session: &mut Session, catalog: &FakePilotCatalog, to: i16) -> Option<SystemId> {
        if session.course().last() != Some(&SystemId(to)) {
            session.plot_course(SystemId(to)).expect("a route");
        }
        fly_out(session);
        session.begin_jump().expect("jumps");
        session.arrive(catalog)
    }

    #[test]
    fn a_session_reads_the_star_map_and_the_date_once_when_it_starts() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(dmy(&session), (23, 6, 1177));
        assert_eq!(*catalog.star_map_reads.borrow(), 1);
        session.plot_course(SystemId(132)).expect("a route");
        jump(&mut session, &catalog, 132);
        jump(&mut session, &catalog, 132);
        assert_eq!(*catalog.star_map_reads.borrow(), 1);
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.course(), []);
        assert_eq!(session.jumping(), None);
    }

    #[test]
    fn the_star_map_is_the_catalogs() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(*session.star_map(), StarMap::new(catalog().star_map));
        assert_eq!(
            session.star_map().position(SystemId(132)),
            Some(Vec2::new(600.0, 600.0))
        );
    }

    #[test]
    fn plotting_a_course_keeps_the_route_and_a_failed_plot_clears_it() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.course(), []);
        assert_eq!(
            session.plot_course(SystemId(132)),
            Ok(&ids(&[131, 132])[..])
        );
        assert_eq!(session.course(), ids(&[131, 132]));
        assert_eq!(session.plot_course(SystemId(131)), Ok(&ids(&[131])[..]));
        assert_eq!(session.course(), ids(&[131]));
        assert_eq!(
            session.plot_course(SystemId(133)),
            Err(RouteError::Unreachable)
        );
        assert_eq!(session.course(), []);
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(
            session.plot_course(SystemId(130)),
            Err(RouteError::AlreadyThere)
        );
        assert_eq!(session.course(), []);
        session.plot_course(SystemId(132)).expect("a route");
        assert_eq!(session.plot_course(SystemId(999)), Err(RouteError::Unknown));
        assert_eq!(session.course(), []);
    }

    #[test]
    fn a_jump_is_refused_without_a_destination_too_close_or_without_fuel() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        session.plot_course(SystemId(131)).expect("a route");
        assert_eq!(
            session.begin_jump(),
            Err(JumpRefusal::TooClose { distance: 0.0 })
        );
        assert_eq!(session.jumping(), None);

        let empty = FakePilotCatalog {
            ships: vec![(ShipId(128), Ok(ShipFields { fuel: 99, ..FAST }))],
            ..catalog()
        };
        let mut session = Session::start(&empty).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        assert_eq!(
            session.begin_jump(),
            Err(JumpRefusal::NoFuel { fuel: 99.0 })
        );
        assert_eq!(session.jumping(), None);
    }

    #[test]
    fn while_jumping_ticks_move_nothing() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        assert_eq!(session.jumping(), Some(SystemId(131)));
        let leaving = *session.player();
        for _ in 0..10 {
            session.tick(THRUST);
        }
        assert_eq!(*session.player(), leaving);
        assert_eq!(session.system(), SystemId(130), "not there yet");
    }

    #[test]
    fn arriving_takes_a_jumps_fuel_and_a_day() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert_eq!(
            session.reserves().fuel,
            Gauge {
                now: 200.0,
                max: 300.0
            }
        );
        assert_eq!(dmy(&session), (24, 6, 1177));
        let shield = session.reserves().shield;
        assert_eq!(shield, Gauge::full(30.0), "the other reserves carry over");
    }

    #[test]
    fn arriving_puts_the_ship_at_the_edge_facing_the_system_it_came_from() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        fly_out(&mut session);
        session.begin_jump().expect("jumps");
        assert_eq!(session.arrive(&catalog), Some(SystemId(131)));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.course(), ids(&[132]));
        assert_eq!(session.jumping(), None);
        assert_eq!(
            *session.player(),
            crate::hyperspace::arrival(Vec2::ZERO, Vec2::new(600.0, 0.0), &session.handling())
        );
        assert_eq!(session.player().position, Vec2::new(-1000.0, 0.0));
    }

    #[test]
    fn arriving_reads_the_new_systems_stellars_and_landing_uses_them() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131);
        assert_eq!(
            *catalog.sites_asked.borrow(),
            [SystemId(130), SystemId(131)]
        );
        // Planet 140 at 131's centre: drift there and land.
        let refused = session.land();
        assert!(
            matches!(refused, Err(LandingRefusal::TooFar { nearest, .. }) if nearest == StellarId(140)),
            "{refused:?}"
        );
    }

    #[test]
    fn arriving_when_not_jumping_does_nothing() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        let before = session.clone();
        assert_eq!(session.arrive(&catalog), None);
        assert_eq!(session, before);
        assert_eq!(*catalog.sites_asked.borrow(), [SystemId(130)]);
    }

    #[test]
    fn a_second_jump_continues_the_route_to_the_destination() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        jump(&mut session, &catalog, 132);
        assert_eq!(session.system(), SystemId(131));
        session.tick(Controls::default());
        assert_eq!(
            session.begin_jump(),
            Err(JumpRefusal::TooClose { distance: 994.0 }),
            "drifting in from the edge"
        );
        assert_eq!(jump(&mut session, &catalog, 132), Some(SystemId(132)));
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.course(), []);
        assert_eq!(session.reserves().fuel.now, 100.0);
        assert_eq!(dmy(&session), (25, 6, 1177));
        // From 131, north of 132 on screen: it arrives at the top edge.
        assert_eq!(session.player().position, Vec2::new(0.0, -1000.0));
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    }

    /// The catalog with ship 128 regenerating a unit of fuel every
    /// `regen` ticks.
    fn regenerating(regen: i16) -> FakePilotCatalog {
        FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    fuel_regen: regen,
                    ..FAST
                }),
            )],
            ..catalog()
        }
    }

    #[test]
    fn fuel_regenerates_each_tick_at_the_ships_rate_up_to_full() {
        let catalog = regenerating(2);
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.fuel_regen_per_tick(), 0.5);
        jump(&mut session, &catalog, 131);
        assert_eq!(session.reserves().fuel.now, 200.0);
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, 205.0);
        for _ in 0..1000 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel, Gauge::full(300.0));
        let still = Session::start(&self::catalog()).expect("starts");
        assert_eq!(still.fuel_regen_per_tick(), 0.0);
    }

    #[test]
    fn the_ships_default_outfits_add_to_its_fuel_regeneration() {
        // A unit every 8 ticks from the ship, and two scoops each giving a
        // unit every 4 ticks; a mod of another type gives nothing.
        let scoops = OutfitMod {
            mod_type: FUEL_SCOOP,
            mod_val: 4,
            count: 2,
        };
        let other = OutfitMod {
            mod_type: 1,
            mod_val: 1,
            count: 1,
        };
        let catalog = FakePilotCatalog {
            outfits: vec![(ShipId(128), vec![scoops, other])],
            ..regenerating(8)
        };
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(*catalog.outfits_asked.borrow(), [ShipId(128)]);
        assert_eq!(session.fuel_regen_per_tick(), 0.125 + 0.5);
        jump(&mut session, &catalog, 131);
        for _ in 0..8 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, 205.0);
        assert_eq!(*catalog.outfits_asked.borrow(), [ShipId(128)], "once");
    }

    /// A slow ship, 1 pixel a tick at most, gaining a unit a tick: it
    /// arrives slow enough to land, over planet 140 at 131's edge.
    fn edge_lander() -> FakePilotCatalog {
        FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    speed: 100,
                    fuel_regen: 1,
                    ..FAST
                }),
            )],
            sites: vec![(SystemId(131), vec![planet(140, -1000.0, 0.0)])],
            ..catalog()
        }
    }

    /// A session landed on planet 140 at 131's edge, far enough out to
    /// jump, with fuel and 132 still to go.
    fn landed_at_the_edge(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        jump(&mut session, catalog, 132);
        assert_eq!(session.land(), Ok(StellarId(140)));
        session
    }

    #[test]
    fn a_jump_is_refused_while_landed() {
        let catalog = edge_lander();
        let mut session = landed_at_the_edge(&catalog);
        let docked = session.clone();
        assert_eq!(session.begin_jump(), Err(JumpRefusal::Landed));
        assert_eq!(session, docked, "nothing changes");
        session.take_off();
        assert_eq!(session.begin_jump(), Ok(SystemId(132)), "once off");
        assert_eq!(session.arrive(&catalog), Some(SystemId(132)));
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn landing_is_refused_while_jumping() {
        let catalog = edge_lander();
        let mut session = landed_at_the_edge(&catalog);
        session.take_off();
        session.begin_jump().expect("jumps from over planet 140");
        let jumping = session.clone();
        assert_eq!(session.land(), Err(LandingRefusal::Jumping));
        assert_eq!(session, jumping, "nothing changes");
        assert_eq!(session.arrive(&catalog), Some(SystemId(132)));
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn fuel_does_not_regenerate_while_landed_or_jumping() {
        let catalog = edge_lander();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(132)).expect("a route");
        jump(&mut session, &catalog, 132);
        assert_eq!(session.reserves().fuel.now, 200.0);
        assert_eq!(session.land(), Ok(StellarId(140)));
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, 200.0);
        session.take_off();
        session.tick(Controls::default());
        assert_eq!(session.reserves().fuel.now, 201.0);

        session.begin_jump().expect("jumps from the edge");
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.reserves().fuel.now, 201.0);
    }

    // Sounds.

    #[test]
    fn holding_thrust_starts_it_once_and_letting_go_stops_it_once() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(Controls::default());
        assert_eq!(session.take_sounds(), []);
        for _ in 0..5 {
            session.tick(THRUST);
        }
        assert_eq!(session.take_sounds(), [SimSound::ThrustStarted]);
        for _ in 0..5 {
            session.tick(Controls {
                turn: Turn::Left,
                ..Controls::default()
            });
        }
        assert_eq!(session.take_sounds(), [SimSound::ThrustStopped]);
        session.tick(THRUST);
        session.tick(Controls::default());
        session.tick(THRUST);
        assert_eq!(
            session.take_sounds(),
            [
                SimSound::ThrustStarted,
                SimSound::ThrustStopped,
                SimSound::ThrustStarted
            ]
        );
    }

    #[test]
    fn taking_the_sounds_empties_them() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(THRUST);
        assert_eq!(session.take_sounds(), [SimSound::ThrustStarted]);
        assert_eq!(session.take_sounds(), []);
    }

    /// The catalog with planet 128 playing `snd ` 10032 when landed on.
    fn sounding() -> FakePilotCatalog {
        FakePilotCatalog {
            sites: vec![(
                SystemId(130),
                vec![LandingSite {
                    landing_sound: Some(SoundId(10_032)),
                    ..planet(128, 30.0, -40.0)
                }],
            )],
            ..catalog()
        }
    }

    #[test]
    fn landing_emits_landed_with_the_stellars_sound() {
        let mut session = Session::start(&sounding()).expect("starts");
        session.land().expect("lands");
        assert_eq!(
            session.take_sounds(),
            [SimSound::Landed {
                stellar_sound: Some(SoundId(10_032))
            }]
        );
        let mut silent = Session::start(&catalog()).expect("starts");
        silent.land().expect("lands");
        assert_eq!(
            silent.take_sounds(),
            [SimSound::Landed {
                stellar_sound: None
            }]
        );
    }

    #[test]
    fn landing_while_thrusting_stops_the_thrust_first() {
        let mut session = Session::start(&sounding()).expect("starts");
        session.tick(Controls {
            turn: Turn::Right,
            ..THRUST
        });
        assert_eq!(session.take_sounds(), [SimSound::ThrustStarted]);
        session.land().expect("lands");
        assert_eq!(
            session.take_sounds(),
            [
                SimSound::ThrustStopped,
                SimSound::Landed {
                    stellar_sound: Some(SoundId(10_032))
                }
            ]
        );
        session.tick(THRUST);
        assert_eq!(session.take_sounds(), [], "docked, nothing thrusts");
        session.take_off();
        assert_eq!(session.take_sounds(), [SimSound::TookOff]);
        session.tick(THRUST);
        assert_eq!(
            session.take_sounds(),
            [SimSound::ThrustStarted],
            "off again"
        );
    }

    #[test]
    fn taking_off_emits_took_off_and_not_taking_off_emits_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.take_off();
        assert_eq!(session.take_sounds(), [], "not landed");
        session.land().expect("lands");
        session.take_sounds();
        session.take_off();
        assert_eq!(session.take_sounds(), [SimSound::TookOff]);
        session.take_off();
        assert_eq!(session.take_sounds(), [], "nor twice");
    }

    #[test]
    fn a_refused_landing_emits_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        for _ in 0..20 {
            session.tick(THRUST);
        }
        session.take_sounds();
        session.land().expect_err("refused");
        assert_eq!(session.take_sounds(), []);
        let mut jumping = Session::start(&edge_lander()).expect("starts");
        jumping.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut jumping);
        jumping.begin_jump().expect("jumps");
        jumping.take_sounds();
        assert_eq!(jumping.land(), Err(LandingRefusal::Jumping));
        assert_eq!(jumping.take_sounds(), []);
    }

    #[test]
    fn a_jump_emits_jump_began_then_arrived_stopping_the_thrust_first() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.tick(THRUST);
        let thrusting = session.take_sounds();
        assert_eq!(thrusting.last(), Some(&SimSound::ThrustStarted));
        session.begin_jump().expect("jumps");
        assert_eq!(
            session.take_sounds(),
            [SimSound::ThrustStopped, SimSound::JumpBegan]
        );
        session.tick(THRUST);
        assert_eq!(session.take_sounds(), [], "jumping, nothing thrusts");
        session.arrive(&catalog).expect("arrives");
        assert_eq!(session.take_sounds(), [SimSound::Arrived]);
        session.arrive(&catalog);
        assert_eq!(session.take_sounds(), [], "no jump under way");
    }

    #[test]
    fn a_jump_without_thrust_emits_only_jump_began() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        session.tick(Controls::default());
        session.take_sounds();
        session.begin_jump().expect("jumps");
        assert_eq!(session.take_sounds(), [SimSound::JumpBegan]);
    }

    #[test]
    fn a_refused_jump_emits_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.tick(THRUST);
        session.take_sounds();
        assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
        assert_eq!(session.take_sounds(), []);
        let catalog = edge_lander();
        let mut landed = landed_at_the_edge(&catalog);
        landed.take_sounds();
        assert_eq!(landed.begin_jump(), Err(JumpRefusal::Landed));
        assert_eq!(landed.take_sounds(), []);
    }
}
