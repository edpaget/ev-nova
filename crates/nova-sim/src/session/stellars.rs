//! The stellars destroyed and regenerated on the session: `Y` and `U`,
//! handed to `Session::apply_op`, each read from the stellar's
//! [`StellarLife`](crate::StellarLife) the session read when it started.
//! Each writes the galaxy overlay through the writer's catalog-free entry,
//! `Session::set_stellar_state`, so every reader sees it at once (see the
//! [`story`](crate::story) module's "Galaxy overlay").
//!
//! - `Yxxx` destroys the stellar with its countdown, its `DeadTime` or 1
//!   when that is negative, whether it was destroyed or not, so a second
//!   `Y` restarts the countdown (@0x15ac8-0x15ade). By
//!   [`RuleKey::DestroyInvincible`]'s engine reading an invincible stellar
//!   (`Strength` 0 or -1) is left as it was. A stellar with no record, or
//!   outside the original's slots, 128 to 2175, is left alone. It runs no
//!   `OnDestroy`, and its explosion waits for the stellars destroyed by
//!   shots (story-runtime phase 8).
//! - `Uxxx` regenerates the stellar at once and runs no `OnRegen`, which
//!   runs only as it regenerates on its own (@0x15aeb-0x15b20).
//!
//! A stellar the player is docked at stays the one docked at when `Y`
//! destroys it: the spaceport stays open and the take-off works, as the
//! original's port never asks; only landing there again is refused.

use super::Session;
use crate::catalog::StellarId;
use crate::rulebook::RuleKey;
use crate::story::wreck;

impl Session {
    /// `Y`: destroys `stellar` (see the module docs).
    pub(super) fn destroy_stellar(&mut self, stellar: StellarId) {
        if !wreck::in_range(stellar) {
            return;
        }
        let Some(life) = self.lives.get(&stellar) else {
            return;
        };
        if wreck::destroys(life, self.rules.source_for(RuleKey::DestroyInvincible)) {
            let countdown = wreck::countdown_on_destroy(life);
            self.set_stellar_state(stellar, Some(countdown));
        }
    }

    /// `U`: regenerates `stellar` (see the module docs). Only `Y` and a
    /// new pilot's stellars destroy one, each within the original's
    /// slots, so any other is never destroyed.
    pub(super) fn regenerate_stellar(&mut self, stellar: StellarId) {
        self.set_stellar_state(stellar, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RelocateRefusal;
    use crate::catalog::{StellarLife, SystemId};
    use crate::control::{Bit, Script, SetExpr, SetOp};
    use crate::landing::{LandingRefusal, StellarFlags};
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::session::{SessionEvent, StellarPick};
    use crate::story::Countdown;
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, land_now, life};

    /// [`catalog`] with 128 back after 2 days, 129 after 5, and 140
    /// never.
    fn wrecking() -> FakePilotCatalog {
        catalog().living([life(128, 2), life(129, 5), life(140, -1)])
    }

    fn run(session: &mut Session, text: &str) {
        session.run_set(
            &SetExpr::parse(text).expect("parses"),
            &mut Scripted::default(),
        );
    }

    /// `stellar`'s countdown, while it is destroyed.
    fn destroyed(session: &Session, stellar: i16) -> Option<Countdown> {
        session
            .galaxy()
            .overlay()
            .stellar(StellarId(stellar))
            .destroyed
    }

    fn left(days_left: u16) -> Countdown {
        Countdown { days_left }
    }

    /// Whether the stellar shown `stellar` is destroyed.
    fn shown_destroyed(session: &Session, stellar: i16) -> bool {
        session
            .sites
            .iter()
            .find(|site| site.id == StellarId(stellar))
            .expect("shown")
            .destroyed
    }

    fn quiet(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.take_events();
        session.take_save_due();
        session
    }

    #[test]
    fn y_destroys_a_stellar_for_its_dead_time_as_every_reader_sees_it() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y129");
        assert_eq!(destroyed(&session, 129), Some(left(5)));
        assert_eq!(session.pilot().galaxy, *session.galaxy().overlay());
        assert!(shown_destroyed(&session, 129));
        assert!(!shown_destroyed(&session, 128), "the other");
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
        assert!(session.take_save_due());
        run(&mut session, "Y140");
        assert_eq!(destroyed(&session, 140), Some(left(1)), "never back: a day");
    }

    #[test]
    fn the_land_key_refuses_a_destroyed_stellar() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        assert_eq!(
            session.select_stellar(StellarPick::Slot(0)),
            Some(StellarId(128))
        );
        assert_eq!(
            session.land(),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(128),
                station: false,
            })
        );
        assert_eq!(session.landed(), None);
    }

    #[test]
    fn f5_skips_a_destroyed_stellar() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        assert_eq!(
            session.select_stellar(StellarPick::Nearest),
            Some(StellarId(128))
        );
        run(&mut session, "Y128");
        assert_eq!(
            session.select_stellar(StellarPick::Nearest),
            Some(StellarId(129))
        );
        run(&mut session, "U128");
        assert_eq!(
            session.select_stellar(StellarPick::Nearest),
            Some(StellarId(128))
        );
    }

    /// [`wrecking`] with 128 and 140 landable only once destroyed.
    fn ruins() -> FakePilotCatalog {
        let mut catalog = wrecking();
        for (_, sites) in &mut catalog.sites {
            for site in sites {
                if matches!(site.id.0, 128 | 140) {
                    site.flags |= StellarFlags::ONLY_WHEN_DESTROYED;
                }
            }
        }
        catalog
    }

    #[test]
    fn a_stellar_landable_only_once_destroyed_becomes_landable_and_dockable() {
        let catalog = ruins();
        let mut session = quiet(&catalog);
        session.select_stellar(StellarPick::Slot(0));
        assert!(matches!(
            session.land(),
            Err(LandingRefusal::NotLandable { .. })
        ));
        run(&mut session, "Y128");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(
            session.relocate(&catalog, SystemId(131), StellarId(140)),
            Err(RelocateRefusal::NotLandable)
        );
        run(&mut session, "Y140");
        assert_eq!(
            session.relocate(&catalog, SystemId(131), StellarId(140)),
            Ok(())
        );
        session.assert_docked_among_shown();
        assert_eq!(session.landed(), Some(StellarId(140)));
    }

    #[test]
    fn a_second_y_restarts_the_countdown() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        session.set_stellar_state(StellarId(128), Some(left(1)));
        session.take_events();
        run(&mut session, "Y128");
        assert_eq!(destroyed(&session, 128), Some(left(2)));
        assert_eq!(session.take_events(), [], "destroyed already");
        assert!(session.take_save_due(), "a new countdown");
    }

    /// [`wrecking`] with 128 of `strength`.
    fn invincible(strength: i32) -> FakePilotCatalog {
        catalog().living([StellarLife {
            strength,
            ..life(128, 2)
        }])
    }

    #[test]
    fn by_the_engine_y_leaves_an_invincible_stellar_as_it_was() {
        for strength in [0, -1] {
            let catalog = invincible(strength);
            let mut session = quiet(&catalog);
            run(&mut session, "Y128");
            assert_eq!(destroyed(&session, 128), None, "{strength}");
            assert_eq!(session.take_events(), [], "{strength}");
            assert!(!session.take_save_due(), "{strength}");
            assert_eq!(
                session.rules().source_for(RuleKey::DestroyInvincible),
                RuleSource::Engine,
                "by default"
            );
        }
    }

    #[test]
    fn by_the_bible_y_destroys_an_invincible_stellar() {
        let rules =
            Rulebook::default().with_override(RuleKey::DestroyInvincible, RuleSource::Bible);
        for strength in [0, -1] {
            let catalog = invincible(strength);
            let mut session = quiet(&catalog).with_rules(rules);
            run(&mut session, "Y128");
            assert_eq!(destroyed(&session, 128), Some(left(2)), "{strength}");
        }
    }

    #[test]
    fn y_on_a_stellar_with_no_record_or_outside_the_slots_does_nothing() {
        let catalog = catalog().living([life(127, 2), life(2176, 2), life(2175, 2)]);
        let mut session = quiet(&catalog);
        run(&mut session, "Y300");
        for id in [127, 2176] {
            session
                .apply_op(
                    &SetOp::DestroyStellar(StellarId(id)),
                    &mut Scripted::default(),
                )
                .expect("handled");
        }
        assert!(session.galaxy().overlay().is_empty());
        assert_eq!(session.take_events(), []);
        assert!(!session.take_save_due());
        run(&mut session, "Y2175");
        assert_eq!(destroyed(&session, 2175), Some(left(2)), "the last slot");
    }

    #[test]
    fn u_regenerates_a_stellar_and_runs_no_on_regen() {
        let catalog = catalog().living([StellarLife {
            on_regen: Script::parse("b5"),
            ..life(128, 2)
        }]);
        let mut session = quiet(&catalog);
        run(&mut session, "U128");
        assert_eq!(session.take_events(), [], "whole already");
        assert!(!session.take_save_due());
        run(&mut session, "Y128 U128");
        assert_eq!(destroyed(&session, 128), None);
        assert!(!shown_destroyed(&session, 128));
        assert!(
            !session.control_bit(Bit::new(5).expect("a bit")),
            "no OnRegen"
        );
        assert_eq!(
            session.take_events(),
            [SessionEvent::GalaxyChanged, SessionEvent::GalaxyChanged]
        );
        assert!(session.galaxy().overlay().is_empty());
    }

    #[test]
    fn y_on_the_stellar_docked_at_leaves_the_player_docked_until_the_take_off() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        run(&mut session, "Y128");
        assert_eq!(session.landed(), Some(StellarId(128)), "still docked");
        session.assert_docked_among_shown();
        assert_eq!(session.pilot().stellar, Some(StellarId(128)));
        assert_eq!(
            session.take_off(&mut Scripted::default()),
            Some(StellarId(128))
        );
        session.select_stellar(StellarPick::Slot(0));
        assert_eq!(
            session.land(),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(128),
                station: false,
            }),
            "not again"
        );
    }

    #[test]
    fn a_pilot_saved_docked_at_a_stellar_destroyed_since_resumes_docked_there() {
        let catalog = wrecking();
        let mut session = quiet(&catalog);
        land_now(&mut session).expect("lands");
        run(&mut session, "Y128");
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(flown.landed(), Some(StellarId(128)));
        assert!(shown_destroyed(&flown, 128));
        assert_eq!(destroyed(&flown, 128), Some(left(2)));
    }
}
