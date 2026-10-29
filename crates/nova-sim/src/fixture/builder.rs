//! [`SessionBuilder`]: a session in a given state, for tests outside the
//! crate, reached without the load's rules.
//!
//! A test that needs "a session in state X" asks the builder for it, and
//! this module's tests pin what each step produces. A change to how the
//! session reaches a state then shows in this crate's tests, which the
//! hook runs, not only in a downstream crate's.
//!
//! **The base.** A new pilot from the catalog's first `chär`, named as
//! [`SessionBuilder::named`] says, flown as [`Session::fly`] flies it: no
//! rule of the load changes a new pilot, as it has no course, no stellar
//! and a system that exists. Every later step is applied to that built
//! session, in the order it was asked for, through the session's own
//! setters. None goes through a pilot that is then loaded, and none takes
//! off.

use crate::catalog::{AsteroidCatalog, CombatCatalog, PilotCatalog, ShipId, StellarId, SystemId};
use crate::fleet::Escort;
use crate::mission::ActiveMission;
use crate::pilot::Pilot;
use crate::reserves::Reserve;
use crate::rulebook::Rulebook;
use crate::session::Session;

/// One step of a [`SessionBuilder`], applied to the built session.
type Step<'c, C> = Box<dyn FnOnce(&mut Session, &'c C) + 'c>;

/// A session in a given state, built step by step over `catalog` (see
/// the module docs).
pub struct SessionBuilder<'c, C> {
    catalog: &'c C,
    name: String,
    rules: Option<Rulebook>,
    steps: Vec<Step<'c, C>>,
}

impl<'c, C: PilotCatalog + CombatCatalog + AsteroidCatalog> SessionBuilder<'c, C> {
    /// A new, unnamed pilot of `catalog` flying at the centre of its
    /// starting system, at rest.
    #[must_use]
    pub fn flying(catalog: &'c C) -> Self {
        Self {
            catalog,
            name: String::new(),
            rules: None,
            steps: Vec::new(),
        }
    }

    /// The pilot named `name`.
    #[must_use]
    pub fn named(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }

    /// The session's rules `rules` ([`Session::with_rules`]), set on the
    /// base before any step, so every step reads them.
    #[must_use]
    pub fn with_rules(mut self, rules: Rulebook) -> Self {
        self.rules = Some(rules);
        self
    }

    /// A course of `hops` from the system the ship is in, the start that
    /// system: as a kept load leaves a saved course, not cued, the nav
    /// selection untouched. [`SessionBuilder::cued`] cues it.
    #[must_use]
    pub fn with_course(self, hops: impl IntoIterator<Item = SystemId>) -> Self {
        let hops: Vec<SystemId> = hops.into_iter().collect();
        self.then(move |session, _| session.set_course(hops))
    }

    /// The course cued ([`Session::cue_course`]): its first hop selected
    /// in nav mode 3, when the ship can jump to it.
    #[must_use]
    pub fn cued(self) -> Self {
        self.then(|session, _| session.cue_course())
    }

    /// The ship in flight over `stellar`, one of its system's stellars
    /// shown, as a load resumes a pilot last landed there: at its centre,
    /// at rest, `stellar` the one last landed on. Nothing lands.
    ///
    /// # Panics
    ///
    /// At [`SessionBuilder::build`], when the system shows no `stellar`.
    #[must_use]
    pub fn over(self, stellar: StellarId) -> Self {
        self.then(move |session, _| {
            assert!(
                session.resume_over(stellar),
                "{stellar:?} is not shown in {:?}",
                session.system()
            );
        })
    }

    /// Landed at `stellar` in `system`, as [`Session::land_at`] lands
    /// there: docked, the spaceport open, its `Landed` raised for the next
    /// settling.
    ///
    /// # Panics
    ///
    /// At [`SessionBuilder::build`], when the landing is refused.
    #[must_use]
    pub fn landed_at(self, system: SystemId, stellar: StellarId) -> Self {
        self.then(move |session, catalog| {
            if let Err(refusal) = session.land_at(catalog, system, stellar) {
                panic!("landing at {stellar:?} in {system:?}: {refusal}");
            }
        })
    }

    /// `escorts` the fleet, each restocked as a load restocks it; the
    /// traffic places them in the system as it is next populated (a
    /// view's first tick).
    #[must_use]
    pub fn with_escorts(self, escorts: Vec<Escort>) -> Self {
        self.then(move |session, _| session.set_escorts(escorts))
    }

    /// The pilot's cash `credits` ([`Session::set_credits`]).
    #[must_use]
    pub fn with_credits(self, credits: i64) -> Self {
        self.then(move |session, _| session.set_credits(credits))
    }

    /// `mission` active in `slot`, its hooks registered as a load
    /// registers them.
    ///
    /// # Panics
    ///
    /// At [`SessionBuilder::build`], when `slot` is taken or past the
    /// last.
    #[must_use]
    pub fn with_mission(self, slot: u8, mission: ActiveMission) -> Self {
        self.then(move |session, _| {
            assert!(
                session.put_mission(slot, mission),
                "slot {slot} is not free"
            );
        })
    }

    /// The ship named `name`.
    #[must_use]
    pub fn with_ship_name(self, name: &str) -> Self {
        let name = name.to_owned();
        self.then(move |session, _| session.set_ship_name(name))
    }

    /// The pilot playing Strict Play.
    #[must_use]
    pub fn strict_play(self) -> Self {
        self.then(|session, _| session.set_strict_play(true))
    }

    /// The ship's `reserve` at `amount`, held within its gauge
    /// ([`Session::set_reserve`]).
    #[must_use]
    pub fn with_reserve(self, reserve: Reserve, amount: f32) -> Self {
        self.then(move |session, _| {
            session.set_reserve(reserve, amount);
        })
    }

    /// The ship of class `ship`, as `C` changes it: the outfits kept, none
    /// clamped.
    ///
    /// # Panics
    ///
    /// At [`SessionBuilder::build`], when the catalog has no record of
    /// `ship`.
    #[must_use]
    pub fn in_ship(self, ship: ShipId) -> Self {
        self.then(move |session, _| {
            assert!(session.become_ship(ship), "no record of {ship:?}");
        })
    }

    /// Adds `step`, applied to the built session after those before it.
    fn then(mut self, step: impl FnOnce(&mut Session, &'c C) + 'c) -> Self {
        self.steps.push(Box::new(step));
        self
    }

    /// The session, every step applied, with nothing told and no save due:
    /// the steps are setup, not play, and a view reads the state as it
    /// builds, as it reads a loaded one.
    ///
    /// # Panics
    ///
    /// When the pilot cannot start or fly ([`crate::StartError`]), or a
    /// step cannot be applied: a test helper's failure.
    #[must_use]
    pub fn build(self) -> Session {
        let pilot = Pilot::new(self.catalog, &self.name).expect("the pilot starts");
        let mut session = Session::fly(self.catalog, pilot).expect("the pilot flies");
        if let Some(rules) = self.rules {
            session = session.with_rules(rules);
        }
        for step in self.steps {
            step(&mut session, self.catalog);
        }
        session.take_events();
        session.take_save_due();
        session
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // the reserves set are whole values
mod tests {
    use super::*;
    use crate::navigation::NavMode;
    use crate::rulebook::{RuleKey, RuleSource};
    use crate::testkit::catalog;

    #[test]
    fn a_flying_builder_flies_a_new_pilot_at_the_centre() {
        let catalog = catalog();
        let session = SessionBuilder::flying(&catalog).named("Ada").build();
        assert_eq!(session.landed(), None);
        assert_eq!(session.jumping(), None);
        assert_eq!(session.preparing_jump(), None);
        assert_eq!(session.course(), []);
        assert_eq!(session.pilot().stellar(), None);
        assert_eq!(session.pilot().name(), "Ada");
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.player().position, crate::geometry::Vec2::ZERO);
        assert_eq!(session.player().velocity, crate::geometry::Vec2::ZERO);
        assert!(!session.load_repaired());
    }

    #[test]
    fn a_built_session_has_nothing_told_and_no_save_due() {
        let catalog = catalog();
        let mut session = SessionBuilder::flying(&catalog)
            .then(|session, _| {
                session.set_credits(5);
            })
            .build();
        assert_eq!(session.pilot().cash(), 5);
        assert!(!session.take_save_due());
        assert_eq!(session.take_events(), []);
    }

    #[test]
    fn with_course_keeps_the_hops_and_start_uncued() {
        let catalog = catalog();
        let session = SessionBuilder::flying(&catalog)
            .with_course([SystemId(131), SystemId(132)])
            .build();
        assert_eq!(session.course(), [SystemId(131), SystemId(132)]);
        assert_eq!(session.pilot().course_start(), Some(SystemId(130)));
        assert_eq!(session.nav_mode(), NavMode::Off, "not cued");
    }

    #[test]
    fn with_course_holds_under_either_loaded_course_reading() {
        let catalog = catalog();
        for source in RuleSource::ALL {
            let rules = Rulebook::default().with_override(RuleKey::LoadedCourse, source);
            let session = SessionBuilder::flying(&catalog)
                .with_rules(rules)
                .with_course([SystemId(131)])
                .build();
            assert_eq!(session.course(), [SystemId(131)], "{source:?}");
        }
    }

    #[test]
    fn cued_selects_the_first_hop_in_nav_mode_3() {
        let catalog = catalog();
        let session = SessionBuilder::flying(&catalog)
            .with_course([SystemId(131)])
            .cued()
            .build();
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(131))));
        assert_eq!(session.course(), [SystemId(131)]);
    }

    #[test]
    fn over_rests_on_the_stellars_centre_with_it_last_landed() {
        let catalog = catalog();
        let session = SessionBuilder::flying(&catalog)
            .over(StellarId(128))
            .build();
        assert_eq!(session.landed(), None, "in flight");
        assert_eq!(
            session.player().position,
            crate::geometry::Vec2::new(30.0, -40.0)
        );
        assert_eq!(session.player().velocity, crate::geometry::Vec2::ZERO);
        assert_eq!(session.pilot().stellar(), Some(StellarId(128)));
        assert_eq!(session.system(), SystemId(130));
    }

    #[test]
    #[should_panic(expected = "is not shown")]
    fn over_a_stellar_the_system_does_not_show_panics() {
        let catalog = catalog();
        let _ = SessionBuilder::flying(&catalog)
            .over(StellarId(140))
            .build();
    }

    #[test]
    fn landed_at_docks_with_the_port_open() {
        let catalog = catalog();
        let session = SessionBuilder::flying(&catalog)
            .landed_at(SystemId(131), StellarId(140))
            .build();
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.landed(), Some(StellarId(140)));
        assert_eq!(session.pilot().stellar(), Some(StellarId(140)));
        let landed = crate::story::StoryEvent::Landed {
            stellar: StellarId(140),
        };
        assert!(
            session.story_raised().contains(&landed),
            "for the next settling"
        );
    }

    #[test]
    #[should_panic(expected = "landing at")]
    fn a_refused_landing_panics() {
        let catalog = catalog();
        let _ = SessionBuilder::flying(&catalog)
            .landed_at(SystemId(131), StellarId(128))
            .build();
    }

    #[test]
    fn with_escorts_holds_the_fleet_on_the_pilot_and_a_populating_places_it() {
        use crate::catalog::ShipId;
        use crate::chance::NeverFires;
        use crate::reserves::Reserves;
        use crate::testkit::{FAST, FakePilotCatalog, ship};
        let catalog = FakePilotCatalog {
            ship_records: vec![ship(129, FAST)],
            ..catalog()
        };
        let escort = |carried| Escort {
            ship: ShipId(129),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried,
            wage: None,
            person: None,
            mark: None,
        };
        let mut session = SessionBuilder::flying(&catalog)
            .with_escorts(vec![escort(false), escort(false)])
            .build();
        assert_eq!(session.pilot().escorts(), [escort(false), escort(false)]);
        session.populate(&catalog, &mut NeverFires);
        let placed = session
            .npcs()
            .iter()
            .filter(|npc| session.is_escort(npc.id))
            .count();
        assert_eq!(placed, 2);
    }

    #[test]
    fn with_credits_sets_the_cash() {
        let catalog = catalog();
        let session = SessionBuilder::flying(&catalog).with_credits(-7).build();
        assert_eq!(session.pilot().cash(), -7);
    }

    #[test]
    fn with_mission_holds_it_in_its_slot() {
        let mission = crate::testkit::carrying(200, 3);
        let catalog = catalog();
        let session = SessionBuilder::flying(&catalog)
            .with_mission(4, mission.clone())
            .build();
        assert_eq!(session.pilot().missions().get(4), Some(&mission));
        assert_eq!(session.pilot().missions().iter().count(), 1);
    }

    #[test]
    #[should_panic(expected = "slot 0 is not free")]
    fn a_mission_into_a_taken_slot_panics() {
        let catalog = catalog();
        let _ = SessionBuilder::flying(&catalog)
            .with_mission(0, crate::testkit::carrying(200, 3))
            .with_mission(0, crate::testkit::carrying(201, 3))
            .build();
    }

    #[test]
    fn with_ship_name_and_strict_play_set_the_pilot() {
        let catalog = catalog();
        let session = SessionBuilder::flying(&catalog)
            .with_ship_name("Kestrel")
            .strict_play()
            .build();
        assert_eq!(session.pilot().ship_name(), Some("Kestrel"));
        assert!(session.pilot().strict_play());
        let plain = SessionBuilder::flying(&catalog).build();
        assert!(!plain.pilot().strict_play());
    }

    #[test]
    fn with_reserve_sets_it_within_its_gauge() {
        let catalog = catalog();
        let session = SessionBuilder::flying(&catalog)
            .with_reserve(Reserve::Fuel, 95.0)
            .with_reserve(Reserve::Shield, 1000.0)
            .build();
        assert_eq!(session.reserves().fuel.now, 95.0);
        assert_eq!(session.reserves().shield.now, 30.0, "held to its most");
    }

    #[test]
    fn in_ship_flies_the_class() {
        use crate::testkit::{FAST, FakePilotCatalog, ship};
        let catalog = FakePilotCatalog {
            ship_records: vec![ship(129, FAST)],
            ..catalog()
        };
        let session = SessionBuilder::flying(&catalog)
            .in_ship(ShipId(129))
            .build();
        assert_eq!(session.ship(), ShipId(129));
    }

    #[test]
    #[should_panic(expected = "no record of")]
    fn in_a_ship_with_no_record_panics() {
        let catalog = catalog();
        let _ = SessionBuilder::flying(&catalog)
            .in_ship(ShipId(999))
            .build();
    }

    #[test]
    fn with_rules_replaces_the_session_rules() {
        let catalog = catalog();
        let rules = Rulebook::default().with_override(RuleKey::LoadedCourse, RuleSource::Bible);
        let session = SessionBuilder::flying(&catalog).with_rules(rules).build();
        assert_eq!(session.rules(), &rules);
        let plain = SessionBuilder::flying(&catalog).build();
        assert_eq!(plain.rules(), &Rulebook::default());
    }
}
