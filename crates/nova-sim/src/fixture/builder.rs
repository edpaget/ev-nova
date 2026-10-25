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

use crate::catalog::{AsteroidCatalog, CombatCatalog, PilotCatalog, SystemId};
use crate::pilot::Pilot;
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
    fn with_rules_replaces_the_session_rules() {
        let catalog = catalog();
        let rules = Rulebook::default().with_override(RuleKey::LoadedCourse, RuleSource::Bible);
        let session = SessionBuilder::flying(&catalog).with_rules(rules).build();
        assert_eq!(session.rules(), &rules);
        let plain = SessionBuilder::flying(&catalog).build();
        assert_eq!(plain.rules(), &Rulebook::default());
    }
}
