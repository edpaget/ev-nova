//! Direct writes of session state that no public setter makes, for
//! [`SessionBuilder`](crate::fixture::SessionBuilder): each puts the
//! session into a state play reaches some other way, without the rules on
//! that way (a load's, a landing's, a take-off's). Each is named for the
//! state it writes, and writes only that state and what the session
//! derives from it at once.

use super::Session;
use crate::catalog::{ShipId, StellarId, SystemId};
use crate::fleet::Escort;
use crate::geometry::Vec2;
use crate::mission::ActiveMission;
use crate::ship_change::OutfitCarry;

impl Session {
    /// Makes `hops` the course, started at the system the ship is in, or
    /// clears it, its start included, when `hops` is empty: the course a
    /// kept load leaves, not cued, the nav selection untouched.
    pub(crate) fn set_course(&mut self, hops: Vec<SystemId>) {
        self.pilot.course_start = (!hops.is_empty()).then_some(self.pilot.system);
        self.pilot.course = hops;
    }

    /// Puts the ship in flight over `stellar`, one of the system's shown
    /// stellars, as a load resumes a pilot last landed there: at its
    /// centre, at rest, it the stellar last landed on and where the ship
    /// last touched down. Nothing lands. Gives whether the system shows
    /// `stellar`; nothing changes when it doesn't.
    pub(crate) fn resume_over(&mut self, stellar: StellarId) -> bool {
        let Some(site) = self.chart.sites.iter().find(|site| site.id == stellar) else {
            return false;
        };
        let at = site.position;
        self.ship.player.position = at;
        self.ship.player.velocity = Vec2::ZERO;
        self.port.touchdown = at;
        self.pilot.stellar = Some(stellar);
        // The load's rebuild places the stellars' sprites from where the
        // ship resumes.
        self.place_stellar_sprites();
        true
    }

    /// Makes `escorts` the fleet, each restocked as a load restocks the
    /// fleet it reads: its shield and armour full, its fuel held to its
    /// class's most. The traffic places them as the system is next
    /// populated.
    pub(crate) fn set_escorts(&mut self, escorts: Vec<Escort>) {
        self.pilot.escorts = escorts;
        self.space
            .restock_fleet(&mut self.pilot, &self.statics, &self.setup);
    }

    /// Puts `mission` into the free `slot` and registers its hooks, as a
    /// load registers each slot it reads. Gives whether it was put;
    /// nothing changes for a slot taken or past the last.
    pub(crate) fn put_mission(&mut self, slot: u8, mission: ActiveMission) -> bool {
        if self.pilot.missions.get(slot).is_some() || !self.pilot.missions.put(slot, mission) {
            return false;
        }
        self.run.register_slot(&self.pilot, slot);
        true
    }

    /// Names the ship `name`.
    pub(crate) fn set_ship_name(&mut self, name: String) {
        self.pilot.ship_name = Some(name);
    }

    /// Makes the pilot play Strict Play when `strict`, or not.
    pub(crate) fn set_strict_play(&mut self, strict: bool) {
        self.pilot.strict_play = strict;
    }

    /// Makes the ship class `ship`, as `C` changes it: the outfits kept,
    /// none clamped. Gives whether the ship is now `ship`; nothing changes
    /// when the session has no record of it.
    pub(crate) fn become_ship(&mut self, ship: ShipId) -> bool {
        self.change_ship(ship, OutfitCarry::Keep, false);
        self.pilot.ship == ship
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::NavMode;
    use crate::pilot::Pilot;
    use crate::testkit::{catalog, land_now};

    #[test]
    fn setting_the_course_starts_it_here_and_cues_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.set_course(vec![SystemId(131), SystemId(132)]);
        assert_eq!(session.course(), [SystemId(131), SystemId(132)]);
        assert_eq!(session.pilot().course_start(), Some(SystemId(130)));
        assert_eq!(session.nav_mode(), NavMode::Off);
        session.set_course(Vec::new());
        assert_eq!(session.course(), []);
        assert_eq!(session.pilot().course_start(), None, "the start cleared");
    }

    #[test]
    fn resuming_over_a_stellar_leaves_the_session_as_a_load_resumed_over_it() {
        let catalog = catalog();
        let mut session =
            Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies");
        assert!(session.resume_over(StellarId(128)));
        assert_eq!(session.player().position, Vec2::new(30.0, -40.0));
        assert_eq!(session.player().velocity, Vec2::ZERO);
        assert_eq!(session.pilot().stellar(), Some(StellarId(128)));
        assert_eq!(session.landed(), None, "in flight");
        assert_eq!(session.port.touchdown, Vec2::new(30.0, -40.0));
        let mut saved = Pilot::new(&catalog, "Ada").expect("starts");
        saved.stellar = Some(StellarId(128));
        let resumed = Session::fly(&catalog, saved).expect("flies");
        // Two sessions built apart hold distinct shared strategies, which
        // compare by pointer, so their state is compared as it prints.
        assert_eq!(
            format!("{session:?}"),
            format!("{resumed:?}"),
            "as the load resumes it"
        );
    }

    #[test]
    fn resuming_over_a_stellar_not_shown_changes_nothing() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let before = session.clone();
        assert!(!session.resume_over(StellarId(140)), "Alpha Centauri's");
        assert!(!session.resume_over(StellarId(999)));
        assert_eq!(session, before);
    }

    #[test]
    fn a_ship_resumed_over_a_stellar_lands_on_it_at_once() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert!(session.resume_over(StellarId(128)));
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
    }

    #[test]
    fn setting_the_escorts_restocks_them_and_the_traffic_places_them() {
        use crate::catalog::ShipId;
        use crate::chance::NeverFires;
        use crate::reserves::Reserves;
        use crate::testkit::{FAST, FakePilotCatalog, ship};
        let catalog = FakePilotCatalog {
            ship_records: vec![ship(129, FAST)],
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        let escort = Escort {
            ship: ShipId(129),
            reserves: Reserves::full(1.0, 2.0, 5000.0),
            order: None,
            carried: false,
            wage: Some(100),
            person: None,
            mark: None,
        };
        session.set_escorts(vec![escort]);
        let held = session.pilot().escorts();
        assert_eq!(held.len(), 1);
        assert_eq!((held[0].ship, held[0].wage), (ShipId(129), Some(100)));
        let full = Reserves::full(30.0, 45.0, 300.0);
        assert_eq!(held[0].reserves, full, "restocked to its class");
        session.populate(&catalog, &mut NeverFires);
        assert!(
            session.npcs().iter().any(|npc| session.is_escort(npc.id)),
            "{:?}",
            session.npcs()
        );
    }

    #[test]
    fn a_mission_put_into_a_free_slot_is_registered_as_a_load_registers_it() {
        use crate::testkit::carrying;
        let catalog = catalog();
        let mission = ActiveMission {
            days_left: Some(5),
            ..carrying(200, 3)
        };
        let mut session =
            Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies");
        assert!(session.put_mission(2, mission.clone()));
        assert_eq!(session.pilot().missions().get(2), Some(&mission));
        let mut saved = Pilot::new(&catalog, "Ada").expect("starts");
        assert!(saved.missions.put(2, mission.clone()));
        let loaded = Session::fly(&catalog, saved).expect("flies");
        // As in the resume's test: compared as they print.
        assert_eq!(format!("{session:?}"), format!("{loaded:?}"));
        let before = format!("{session:?}");
        assert!(!session.put_mission(2, carrying(201, 1)), "taken");
        assert!(!session.put_mission(16, carrying(201, 1)), "past the last");
        assert_eq!(format!("{session:?}"), before, "nothing changes");
    }

    #[test]
    fn the_ship_name_and_strict_play_are_set_as_given() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.set_ship_name("Kestrel".to_owned());
        assert_eq!(session.pilot().ship_name(), Some("Kestrel"));
        session.set_strict_play(true);
        assert!(session.pilot().strict_play());
        session.set_strict_play(false);
        assert!(!session.pilot().strict_play());
    }

    #[test]
    fn becoming_a_ship_flies_its_class() {
        use crate::testkit::{FAST, FakePilotCatalog, ship};
        let catalog = FakePilotCatalog {
            ship_records: vec![ship(129, FAST)],
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        assert!(session.become_ship(ShipId(129)));
        assert_eq!(session.ship(), ShipId(129));
        assert!(!session.become_ship(ShipId(999)), "no record");
        assert_eq!(session.ship(), ShipId(129));
    }
}
