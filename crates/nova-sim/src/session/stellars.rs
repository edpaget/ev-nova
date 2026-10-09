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
//! The daily step's sixth participant counts each destroyed stellar down
//! a day, by ascending ID (@0xb5c4-0xb71e): one of negative `DeadTime`
//! stays destroyed for ever; any other regenerates once its countdown
//! reaches 0, and runs its `OnRegen` there and then (@0xb6ee), so a
//! `DayPassed` subscription that day sees what it set. A whole stellar's
//! daily repair (@0xb700) has nothing to repair: stellars take no damage
//! until shots can destroy them (story-runtime phase 8).
//!
//! A new pilot ([`Session::begin`]) starts with every stellar whose
//! `Flags2` has 0x0040 destroyed, with the countdown `Y` gives it, before
//! its `OnStart` runs (`_DoNewPilot` @0x192d6-0x1931c); a pilot loaded has
//! its stellars as saved.
//!
//! A stellar the player is docked at stays the one docked at when `Y`
//! destroys it: the spaceport stays open and the take-off works, as the
//! original's port never asks; only landing there again is refused.

use super::Session;
use crate::catalog::StellarId;
use crate::chance::Chance;
use crate::rulebook::RuleKey;
use crate::story::Countdown;
use crate::story::wreck::{self, Tick};

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

    /// The daily step's stellars (see the module docs): each destroyed
    /// stellar in turn, by ascending ID, counts a day down, or regenerates
    /// and runs its `OnRegen` at once, through [`Session::run_set`],
    /// drawing on `chance`. The overlay is read again after each, so a
    /// stellar an `OnRegen` destroys is counted the same day when its ID is
    /// higher, as the original's pass by index does. A destroyed stellar
    /// with no record is left as it is.
    pub(super) fn stellars_day(&mut self, chance: &mut (impl Chance + ?Sized)) {
        let mut after = None;
        while let Some((stellar, countdown)) = self.next_destroyed(after) {
            after = Some(stellar);
            let Some(life) = self.lives.get(&stellar).cloned() else {
                continue;
            };
            match wreck::day(&life, countdown) {
                Tick::Stays(countdown) => {
                    self.set_stellar_state(stellar, Some(countdown));
                }
                Tick::Regenerates => {
                    self.set_stellar_state(stellar, None);
                    self.run_script(&life.on_regen, chance);
                }
            }
        }
    }

    /// Destroys a new pilot's stellars that start the game destroyed
    /// (`Flags2` 0x0040), within the original's slots, each with the
    /// countdown `Y` gives it, as `_DoNewPilot` does before its `OnStart`
    /// (@0x192d6-0x1931c); by [`RuleKey::DestroyInvincible`]'s engine
    /// reading an invincible one starts whole, as `_StellarIsDestroyed`
    /// holds it.
    pub(super) fn destroy_from_the_start(&mut self) {
        let reading = self.rules.source_for(RuleKey::DestroyInvincible);
        let ruined: Vec<_> = self
            .lives
            .values()
            .filter(|life| {
                life.starts_destroyed && wreck::in_range(life.id) && wreck::destroys(life, reading)
            })
            .map(|life| (life.id, wreck::countdown_on_destroy(life)))
            .collect();
        for (stellar, countdown) in ruined {
            self.set_stellar_state(stellar, Some(countdown));
        }
    }

    /// The destroyed stellar of lowest ID above `after` (any, with none),
    /// with its countdown.
    fn next_destroyed(&self, after: Option<StellarId>) -> Option<(StellarId, Countdown)> {
        self.pilot
            .galaxy
            .stellars()
            .filter(|&(id, _)| after.is_none_or(|after| id > after))
            .find_map(|(id, change)| Some((id, change.destroyed?)))
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

    // The daily step's stellars.

    /// [`catalog`] with these lives, 128 and 129's `OnRegen` as given.
    fn regenerating(lives: [(i16, i16, &str); 2]) -> FakePilotCatalog {
        catalog().living(lives.map(|(id, dead_time, on_regen)| StellarLife {
            on_regen: Script::parse(on_regen),
            ..life(id, dead_time)
        }))
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("a bit")
    }

    #[test]
    fn a_destroyed_stellar_counts_its_days_down_and_regenerates_running_its_on_regen() {
        use crate::story::{Consumer, EventFilter, StoryEventKind, Subscription, SubscriptionId};
        let catalog = regenerating([(129, 2, "b7"), (128, 0, "")]);
        let mut session = quiet(&catalog);
        let id = SubscriptionId {
            consumer: Consumer::Fixture,
            record: 1,
            hook: 0,
        };
        session
            .register_story(Subscription {
                guard: crate::control::Test::parse("b7"),
                ..Subscription::new(
                    id,
                    EventFilter::Any(StoryEventKind::DayPassed),
                    Script::parse("b8"),
                )
            })
            .expect("registers");
        land_now(&mut session).expect("lands");
        run(&mut session, "Y129");
        session
            .take_off(&mut Scripted::default())
            .expect("takes off");
        assert_eq!(destroyed(&session, 129), Some(left(1)), "a day down");
        assert!(!session.control_bit(bit(7)));
        land_now(&mut session).expect("lands");
        let _ = session.settle_script(&catalog, &mut Scripted::default());
        assert!(!session.control_bit(bit(8)), "not yet");
        session.take_events();
        session
            .take_off(&mut Scripted::default())
            .expect("takes off");
        assert_eq!(destroyed(&session, 129), None, "regenerated");
        assert!(!shown_destroyed(&session, 129));
        assert!(session.control_bit(bit(7)), "its OnRegen, in the day");
        assert!(
            session.take_events().contains(&SessionEvent::GalaxyChanged),
            "told"
        );
        let _ = session.settle_script(&catalog, &mut Scripted::default());
        assert!(session.control_bit(bit(8)), "the day's DayPassed saw it");
    }

    #[test]
    fn a_stellar_of_dead_time_0_or_1_regenerates_on_its_next_day() {
        for dead_time in [0, 1] {
            let catalog = regenerating([(129, dead_time, "b7"), (128, 0, "")]);
            let mut session = quiet(&catalog);
            run(&mut session, "Y129");
            session.pass_days(1, &mut Scripted::default());
            assert_eq!(destroyed(&session, 129), None, "{dead_time}");
            assert!(session.control_bit(bit(7)), "{dead_time}");
        }
    }

    #[test]
    fn days_passed_together_each_count_a_day() {
        let catalog = regenerating([(129, 5, ""), (128, 0, "")]);
        let mut session = quiet(&catalog);
        session.stats.jump_days = 3;
        run(&mut session, "Y129");
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        assert_eq!(destroyed(&session, 129), Some(left(2)));
    }

    #[test]
    fn a_stellar_of_negative_dead_time_stays_destroyed() {
        let catalog = regenerating([(140, -1, "b7"), (128, 0, "")]);
        let mut session = quiet(&catalog);
        run(&mut session, "Y140");
        session.pass_days(10, &mut Scripted::default());
        assert_eq!(destroyed(&session, 140), Some(left(1)));
        assert!(!session.control_bit(bit(7)));
    }

    #[test]
    fn an_on_regen_destroying_a_higher_stellar_has_it_counted_the_same_day() {
        let catalog = regenerating([(128, 1, "Y129"), (129, 1, "b9")]);
        let mut session = quiet(&catalog);
        run(&mut session, "Y128");
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), None);
        assert_eq!(destroyed(&session, 129), None, "counted the same day");
        assert!(session.control_bit(bit(9)));
    }

    #[test]
    fn an_on_regen_destroying_a_lower_stellar_has_it_counted_from_the_next_day() {
        let catalog = regenerating([(128, 1, "b9"), (129, 1, "Y128")]);
        let mut session = quiet(&catalog);
        run(&mut session, "Y129");
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(destroyed(&session, 129), None);
        assert_eq!(destroyed(&session, 128), Some(left(1)), "passed already");
        assert!(!session.control_bit(bit(9)));
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), None);
        assert!(session.control_bit(bit(9)));
    }

    #[test]
    fn a_destroyed_stellar_with_no_record_is_left_as_it_is() {
        let catalog = regenerating([(129, 1, ""), (128, 0, "")]);
        let mut session = quiet(&catalog);
        session.set_stellar_state(StellarId(300), Some(left(4)));
        run(&mut session, "Y129");
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(destroyed(&session, 300), Some(left(4)));
        assert_eq!(destroyed(&session, 129), None, "the next one counted");
    }

    #[test]
    fn by_the_bible_a_destroyed_invincible_stellar_counts_down_as_any() {
        let rules =
            Rulebook::default().with_override(RuleKey::DestroyInvincible, RuleSource::Bible);
        let catalog = catalog().living([StellarLife {
            strength: 0,
            ..life(128, 2)
        }]);
        let mut session = quiet(&catalog).with_rules(rules);
        run(&mut session, "Y128");
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), Some(left(1)));
        session.pass_days(1, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), None);
    }

    // A new pilot's stellars.

    /// [`catalog`] whose `chär` runs `on_start`, with 128 (back after 3
    /// days), 129 (never back) and 2176 starting the game destroyed, and
    /// 140 whole.
    fn ruined_from_the_start(on_start: &str) -> FakePilotCatalog {
        let mut catalog = catalog().living(
            [
                (128, 3, true),
                (129, -1, true),
                (140, 2, false),
                (2176, 2, true),
            ]
            .map(|(id, dead_time, starts_destroyed)| StellarLife {
                starts_destroyed,
                ..life(id, dead_time)
            }),
        );
        if let Ok(start) = &mut catalog.character {
            start.on_start = Script::parse(on_start);
        }
        catalog
    }

    #[test]
    fn a_new_pilot_begins_with_the_stellars_that_start_the_game_destroyed() {
        let catalog = ruined_from_the_start("");
        let mut session = quiet(&catalog);
        assert!(session.galaxy().overlay().is_empty(), "flying alone");
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), Some(left(3)));
        assert_eq!(destroyed(&session, 129), Some(left(1)));
        assert_eq!(destroyed(&session, 140), None);
        assert_eq!(destroyed(&session, 2176), None, "outside the slots");
        assert!(shown_destroyed(&session, 128));
        assert!(session.take_save_due());
    }

    #[test]
    fn a_new_pilots_on_start_sees_its_stellars_destroyed() {
        let catalog = ruined_from_the_start("U128");
        let mut session = quiet(&catalog);
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), None, "regenerated by OnStart");
        assert_eq!(destroyed(&session, 129), Some(left(1)));
    }

    #[test]
    fn by_the_engine_an_invincible_stellar_does_not_start_the_game_destroyed() {
        let mut catalog = ruined_from_the_start("");
        catalog.lives[0].strength = 0;
        let mut session = quiet(&catalog);
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), None);
        let rules =
            Rulebook::default().with_override(RuleKey::DestroyInvincible, RuleSource::Bible);
        let mut session = quiet(&catalog).with_rules(rules);
        session.begin(&catalog, &mut Scripted::default());
        assert_eq!(destroyed(&session, 128), Some(left(3)), "by the Bible");
    }

    #[test]
    fn a_loaded_pilot_starts_no_stellar_destroyed() {
        let catalog = ruined_from_the_start("");
        let mut session = quiet(&catalog);
        session.begin(&catalog, &mut Scripted::default());
        run(&mut session, "U128");
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(destroyed(&flown, 128), None, "as saved");
        assert_eq!(destroyed(&flown, 129), Some(left(1)), "as saved");
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
