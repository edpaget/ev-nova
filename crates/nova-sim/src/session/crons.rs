//! The crons on the session (see [`cron`](crate::cron)).
//!
//! The daily step's second participant runs `cron::pass` over the crons
//! the session read as it started, through `SessionCrons`, the session's
//! `CronDay`: each crön's state is the pilot's, and a change makes a save
//! due; an inactive crön's eligibility is asked now, its roll drawn on the
//! day's chance, its `Require` read from the bits contributed as they
//! stand and its `EnableOn` through the session's control bits; and each
//! `OnStart` and `OnEnd` runs at once through `Session::run_script`, as
//! a stellar's `OnRegen` does, its `M`, `N`, `Q` and `S` waiting for the
//! next settling. An iterative script's test before each run
//! (`cron::holds`) reads the bits contributed and the control bits as the
//! last run left them, and a loop stopped at 10001 runs reports
//! [`SimDiagnostic::CronRunsCapped`], once a session, through
//! [`Session::take_diagnostics`].
//!
//! The pilot keeps each crön running or holding off, its `days` and
//! `holdoff`, keyed by ID. A pilot flown from a save naming a crön the
//! data no longer has forgets it, so it is inactive and the next save
//! leaves it out, as the original's `_LoadPilotData` forces a slot whose
//! crön is gone to inactive (@0x75fbc-0x76025).
//!
//! **Contribute.** Every reader of the `Contribute` bits the player
//! holds sums them through one function, [`wares::contributed`]: the
//! mission offers, the bar's hires ([`PlayerShip::contributed`]), the
//! outfitter and the shipyard. It reads the ship flown, the outfits
//! owned, the crons running and the active ranks.

use super::{PlayerShip, Statics};
use crate::pilot::Pilot;
use std::rc::Rc;

use crate::catalog::{CronId, CronRecord};
use crate::chance::Chance;
use crate::combat::report::SimDiagnostic;
use crate::control::Script;
#[cfg(any(test, feature = "fixture"))]
use crate::cron::CronRefusal;
use crate::cron::{self, CronDay, CronFacts, CronHook, CronState};
#[cfg(any(test, feature = "fixture"))]
use crate::rules::Verdict;
use crate::session::Session;
use crate::story::SubscriptionId;
use crate::wares::{self, Contributors};

/// The session as the daily cron pass runs on it, its rolls drawn on
/// `chance`.
struct SessionCrons<'a, C: Chance + ?Sized> {
    session: &'a mut Session,
    chance: &'a mut C,
}

impl<C: Chance + ?Sized> CronDay for SessionCrons<'_, C> {
    fn state(&self, cron: CronId) -> Option<CronState> {
        self.session.pilot.crons.get(&cron).copied()
    }

    /// Writes the pilot's crons; a save is due.
    fn set_state(&mut self, cron: CronId, state: Option<CronState>) {
        let crons = &mut self.session.pilot.crons;
        match state {
            Some(state) => crons.insert(cron, state),
            None => crons.remove(&cron),
        };
        self.session.out.save_due = true;
    }

    fn eligible(&mut self, record: &CronRecord) -> bool {
        let facts = self.session.cron_facts(record);
        cron::eligibility()
            .first_refusal(&facts, &mut &mut *self.chance)
            .is_ok()
    }

    /// Runs `script` as a participant's effect, its `M`, `N`, `Q` and `S`
    /// waiting for the next settling as any hook's do.
    fn run(&mut self, _id: SubscriptionId, script: &Script) {
        self.session.run_script(script, &mut *self.chance);
    }

    fn holds(&mut self, record: &CronRecord) -> bool {
        let session = &*self.session;
        cron::holds(
            record,
            session.ship.contributed(&session.pilot, &session.statics),
            session.gate(),
        )
    }

    /// Reports it, once a session.
    fn capped(&mut self, cron: CronId, hook: CronHook) {
        self.session
            .out
            .reports
            .report(SimDiagnostic::CronRunsCapped { cron, hook });
    }
}

impl Session {
    /// The daily step's second participant: one pass over the crons (see
    /// [`cron`](crate::cron)), drawing on `chance`.
    pub(super) fn crons_day(&mut self, chance: &mut (impl Chance + ?Sized)) {
        let crons = Rc::clone(&self.statics.crons);
        let rules = self.setup.rules;
        cron::pass(
            &crons,
            &rules,
            &mut SessionCrons {
                session: self,
                chance,
            },
        );
    }

    /// Why crön `cron` would not start today, as the daily pass asks an
    /// inactive crön: every check of [`cron::eligibility`] that refuses,
    /// by name, in its order. Nothing is drawn ([`Checks::all_refusals`]),
    /// so the `Random` roll is always reported as [`Verdict::Draws`]; a
    /// crön would start when no check reports [`Verdict::Refuses`]. The
    /// crön's state is not read: this answers whether the pass would start
    /// it today were it inactive. `None` when the data has no such crön.
    ///
    /// For the tests and the fixture's walker: no production code calls it.
    ///
    /// [`Checks::all_refusals`]: crate::rules::Checks::all_refusals
    #[cfg(any(test, feature = "fixture"))]
    #[must_use]
    pub(crate) fn cron_refusals(
        &self,
        cron: CronId,
    ) -> Option<Vec<(&'static str, Verdict<CronRefusal>)>> {
        let record = self.statics.crons.iter().find(|record| record.id == cron)?;
        let facts = self.cron_facts(record);
        Some(
            cron::eligibility()
                .all_refusals(&facts)
                .into_iter()
                .map(|(descriptor, verdict)| (descriptor.name(), verdict))
                .collect(),
        )
    }

    /// What crön `record`'s eligibility reads, as the session stands:
    /// today, the `Contribute` bits held, the control bits and the rules.
    fn cron_facts<'a>(&'a self, record: &'a CronRecord) -> CronFacts<'a> {
        CronFacts {
            record,
            today: self.pilot.date,
            contributed: self.ship.contributed(&self.pilot, &self.statics),
            gate: self.gate(),
            rules: &self.setup.rules,
        }
    }
}

impl PlayerShip {
    /// The `Contribute` bits the player holds, as they stand
    /// (`_GetPlayerContributeBits` @0x76b2; see [`wares::contributed`]).
    pub(super) fn contributed(&self, pilot: &Pilot, statics: &Statics) -> u64 {
        let by = Contributors {
            ship: self.fit.fields.contribute,
            outfits: &statics.outfits,
            crons: &statics.crons,
            ranks: &statics.ranks,
        };
        wares::contributed(&by, pilot)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use crate::catalog::{
        CronId, CronRecord, MissionId, MissionRecord, OutfitId, OutfitRecord, ShipId, ShipRecord,
        StellarId, SystemId,
    };
    use crate::chance::NeverFires;
    use crate::combat::report::SimDiagnostic;
    use crate::control::{Bit, Script, Test};
    use crate::cron::{CronHook, CronRefusal, CronState, ITERATE_END, ITERATE_START};
    use crate::date::GameDate;
    use crate::mission::Refusal;
    use crate::outfitter::OutfitFlags;
    use crate::pilot::Pilot;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::rules::Verdict;
    use crate::save;
    use crate::session::Session;
    use crate::shipyard::ShipFlags3;
    use crate::story::{Consumer, EventFilter, StoryEventKind, Subscription, SubscriptionId};
    use crate::testkit::{
        FakePilotCatalog, Scripted, catalog, cron, land_now, mission, outfit, ship,
    };

    /// [`catalog`] with crön 128 contributing 0x4, and mission 130 at the
    /// computer requiring it.
    fn contributing() -> FakePilotCatalog {
        FakePilotCatalog {
            crons: vec![CronRecord {
                duration: 5,
                contribute: 0x4,
                ..cron(128)
            }],
            missions: vec![MissionRecord {
                require: 0x4,
                ..mission(130)
            }],
            ..catalog()
        }
    }

    /// A session of `catalog` landed on 128.
    fn landed(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(catalog, &mut NeverFires);
        session
    }

    /// Whether `session` refuses mission 130 for its `Require`.
    fn requires(session: &Session) -> bool {
        session
            .offer_refusals(MissionId(130))
            .expect("docked")
            .contains(&("Require", Verdict::Refuses(Refusal::Require)))
    }

    /// Why the session would not start crön 128 today.
    fn cron_refusals(session: &Session) -> Vec<(&'static str, Verdict<CronRefusal>)> {
        session.cron_refusals(CronId(128)).expect("a crön")
    }

    #[test]
    fn the_refusals_of_an_unknown_cron_are_none() {
        let catalog = timing(vec![cron(128)]);
        let session = quiet(&catalog);
        assert_eq!(session.cron_refusals(CronId(129)), None);
    }

    #[test]
    fn a_crons_refusals_name_each_check_that_refuses_its_random_drawing() {
        let mut gated = scripted(128, 2, "", "");
        gated.enable_on = Test::parse("b20");
        let catalog = timing(vec![gated]);
        let mut session = quiet(&catalog);
        assert_eq!(
            cron_refusals(&session),
            [
                ("Random", Verdict::Draws),
                ("EnableOn", Verdict::Refuses(CronRefusal::EnableOn)),
            ]
        );
        session.set_control_bit(bit(20), true);
        assert_eq!(cron_refusals(&session), [("Random", Verdict::Draws)]);
    }

    #[test]
    fn a_crons_refusals_list_its_duration_dates_and_require_in_order() {
        let mut shut = scripted(128, -1, "", "");
        shut.require = 0x4;
        shut.enable_on = Test::parse("b20");
        shut.first_month = 9;
        shut.first_day = 1;
        let catalog = timing(vec![shut]);
        let mut session = quiet(&catalog);
        let year = session.date().year();
        session.set_date(GameDate::new(year, 8, 30).expect("a date"));
        assert_eq!(
            cron_refusals(&session),
            [
                ("Duration", Verdict::Refuses(CronRefusal::Disabled)),
                ("Random", Verdict::Draws),
                ("date range", Verdict::Refuses(CronRefusal::Dates)),
                ("Require", Verdict::Refuses(CronRefusal::Require)),
                ("EnableOn", Verdict::Refuses(CronRefusal::EnableOn)),
            ]
        );
    }

    #[test]
    fn a_crons_require_is_met_in_its_refusals_by_a_running_crons_contribute() {
        let catalog = requiring(vec![CronRecord {
            contribute: 0x4,
            ..scripted(128, 5, "", "")
        }]);
        let mut session = quiet(&catalog);
        let refusals = |session: &Session| session.cron_refusals(CronId(129)).expect("a crön");
        assert!(refusals(&session).contains(&("Require", Verdict::Refuses(CronRefusal::Require))));
        session.pilot.crons.insert(
            CronId(128),
            CronState {
                days: 5,
                holdoff: 0,
            },
        );
        assert_eq!(refusals(&session), [("Random", Verdict::Draws)]);
    }

    #[test]
    fn a_running_crons_contribute_unlocks_a_missions_require() {
        let catalog = contributing();
        let mut session = landed(&catalog);
        assert!(requires(&session));
        assert_eq!(
            session.ship.contributed(&session.pilot, &session.statics) & 0x4,
            0
        );
        session.pilot.crons.insert(
            CronId(128),
            CronState {
                days: 5,
                holdoff: 0,
            },
        );
        assert!(!requires(&session));
        assert_eq!(
            session.ship.contributed(&session.pilot, &session.statics) & 0x4,
            0x4
        );
        session.pilot.crons.insert(
            CronId(128),
            CronState {
                days: 5,
                holdoff: 2,
            },
        );
        assert!(requires(&session), "in its pre-holdoff");
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    /// Crön `id` running `duration` days, its `OnStart` `on_start` and its
    /// `OnEnd` `on_end`.
    pub(in crate::session) fn scripted(
        id: i16,
        duration: i16,
        on_start: &str,
        on_end: &str,
    ) -> CronRecord {
        CronRecord {
            duration,
            on_start: Script::parse(on_start),
            on_end: Script::parse(on_end),
            ..cron(id)
        }
    }

    /// [`catalog`] with `crons`.
    fn timing(crons: Vec<CronRecord>) -> FakePilotCatalog {
        FakePilotCatalog { crons, ..catalog() }
    }

    /// A session of `catalog` in flight, its events and save taken.
    fn quiet(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.take_events();
        session.take_save_due();
        session
    }

    #[test]
    fn a_cron_starts_on_the_next_days_pass_running_its_on_start() {
        let catalog = timing(vec![scripted(128, 2, "b10", "!b10")]);
        let mut session = quiet(&catalog);
        assert!(!session.control_bit(bit(10)));
        session.let_days_pass(1, &mut NeverFires);
        assert!(session.control_bit(bit(10)), "its OnStart");
        assert!(session.take_save_due(), "its state changed");
        assert_eq!(
            session.pilot().crons().get(&CronId(128)),
            Some(&CronState {
                days: 2,
                holdoff: 0
            })
        );
        session.let_days_pass(1, &mut NeverFires);
        assert!(session.control_bit(bit(10)));
        session.let_days_pass(1, &mut NeverFires);
        assert!(!session.control_bit(bit(10)), "its OnEnd");
        assert!(session.pilot().crons().is_empty());
    }

    #[test]
    fn a_cron_not_eligible_does_not_start() {
        let mut gated = scripted(128, 2, "b10", "");
        gated.enable_on = Test::parse("b5");
        let catalog = timing(vec![gated]);
        let mut session = quiet(&catalog);
        session.let_days_pass(3, &mut NeverFires);
        assert!(!session.control_bit(bit(10)));
        assert!(session.pilot().crons().is_empty());
        assert!(!session.take_save_due(), "nothing changed");
        session.set_control_bit(bit(5), true);
        session.let_days_pass(1, &mut NeverFires);
        assert!(session.control_bit(bit(10)), "eligible now");
    }

    #[test]
    fn a_crons_roll_is_drawn_on_the_days_chance() {
        let mut rare = scripted(128, 2, "b10", "");
        rare.random = 0;
        let catalog = timing(vec![rare]);
        let mut session = quiet(&catalog);
        let mut chance = Scripted::rolling(&[1]);
        session.let_days_pass(1, &mut chance);
        assert!(
            chance.sides_asked.contains(&101),
            "{:?}",
            chance.sides_asked
        );
        assert!(!session.control_bit(bit(10)), "rolled 1, above Random 0");
        session.let_days_pass(1, &mut Scripted::rolling(&[0]));
        assert!(session.control_bit(bit(10)), "rolled 0");
    }

    #[test]
    fn the_crons_read_the_sessions_rules() {
        let mut rare = scripted(128, 2, "b10", "");
        rare.random = 0;
        let catalog = timing(vec![rare]);
        let mut session = quiet(&catalog).with_rules(
            Rulebook::default().with_override(RuleKey::CronRandomScale, RuleSource::Bible),
        );
        session.let_days_pass(1, &mut NeverFires);
        assert!(
            !session.control_bit(bit(10)),
            "Random 0 never, by the Bible"
        );
    }

    #[test]
    fn a_crons_script_runs_before_a_later_crons_enable_on_is_asked() {
        let mut later = scripted(129, 2, "b12", "");
        later.enable_on = Test::parse("b11");
        let catalog = timing(vec![later, scripted(128, 2, "b11", "")]);
        let mut session = quiet(&catalog);
        session.let_days_pass(1, &mut NeverFires);
        assert!(session.control_bit(bit(11)));
        assert!(session.control_bit(bit(12)), "both started the same day");
    }

    #[test]
    fn a_crons_on_start_runs_before_the_days_day_passed_subscriptions() {
        let catalog = timing(vec![scripted(128, 2, "b10", "")]);
        let mut session = quiet(&catalog);
        session
            .register_story(Subscription {
                guard: Test::parse("b10"),
                ..Subscription::new(
                    SubscriptionId {
                        consumer: Consumer::Fixture,
                        record: 1,
                        hook: 0,
                    },
                    EventFilter::Any(StoryEventKind::DayPassed),
                    Script::parse("b20"),
                )
            })
            .expect("registers");
        session.let_days_pass(1, &mut NeverFires);
        assert!(!session.control_bit(bit(20)), "not settled yet");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(session.control_bit(bit(20)), "the day's DayPassed saw b10");
    }

    #[test]
    fn a_crons_move_writes_the_system_at_once_and_enters_it_at_the_settling() {
        let catalog = timing(vec![scripted(128, 2, "M131", "")]);
        let mut session = quiet(&catalog);
        session.let_days_pass(1, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131), "at the operator");
        assert_eq!(
            session.stellars_shown().collect::<Vec<_>>(),
            [StellarId(128), StellarId(129)],
            "130's until the entry"
        );
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(
            session.stellars_shown().collect::<Vec<_>>(),
            [StellarId(140)]
        );
    }

    #[test]
    fn a_cron_contributes_while_running_and_not_in_its_post_holdoff() {
        let mut held = scripted(128, 1, "", "");
        held.post_holdoff = 2;
        held.contribute = 0x4;
        let catalog = timing(vec![held]);
        let mut session = quiet(&catalog);
        assert_eq!(
            session.ship.contributed(&session.pilot, &session.statics) & 0x4,
            0
        );
        session.let_days_pass(1, &mut NeverFires);
        assert_eq!(
            session.ship.contributed(&session.pilot, &session.statics) & 0x4,
            0x4,
            "running"
        );
        session.let_days_pass(1, &mut NeverFires);
        assert_eq!(
            session.pilot().crons().get(&CronId(128)),
            Some(&CronState {
                days: -1,
                holdoff: 2
            })
        );
        assert_eq!(
            session.ship.contributed(&session.pilot, &session.statics) & 0x4,
            0,
            "in its post-holdoff"
        );
        session.let_days_pass(2, &mut NeverFires);
        assert!(session.pilot().crons().is_empty());
        assert_eq!(
            session.ship.contributed(&session.pilot, &session.statics) & 0x4,
            0
        );
    }

    #[test]
    fn a_crons_contribute_unlocks_a_missions_require_after_its_first_day() {
        let catalog = contributing();
        let mut session = landed(&catalog);
        assert!(requires(&session));
        session.let_days_pass(1, &mut NeverFires);
        assert!(!requires(&session));
    }

    /// [`shipbuying`](crate::session::tests::shipbuying) with crön 128
    /// contributing 0x4, and ship 130 and outfit 310 hidden until 0x4 is
    /// contributed.
    fn cron_gated_wares() -> FakePilotCatalog {
        let mut catalog = crate::session::tests::shipbuying();
        catalog.crons = contributing().crons;
        catalog.ship_records.push(ShipRecord {
            require: 0x4,
            flags3: ShipFlags3::HIDE_UNLESS_REQUIRED,
            ..ship(130, catalog.ship_records[0].fields)
        });
        catalog.outfits.push(OutfitRecord {
            flags: OutfitFlags::HIDE_UNLESS_REQUIRED,
            require: 0x4,
            require_govt: -1,
            tech_level: 1,
            ..outfit(310, &[])
        });
        catalog
    }

    #[test]
    fn a_running_crons_contribute_lists_a_required_ship_and_outfit() {
        let catalog = cron_gated_wares();
        let mut session = crate::session::tests::outfitted(&catalog);
        let listed = |session: &mut Session| {
            let yard = session.shipyard(&mut NeverFires).expect("a shipyard");
            let shop = session.outfitter(&mut NeverFires).expect("an outfitter");
            (
                yard.row(ShipId(130)).is_some(),
                shop.row(OutfitId(310)).is_some(),
            )
        };
        assert_eq!(listed(&mut session), (false, false), "not running");
        session.pilot.crons.insert(
            CronId(128),
            CronState {
                days: 5,
                holdoff: 0,
            },
        );
        assert_eq!(listed(&mut session), (true, true), "running");
        session.pilot.crons.insert(
            CronId(128),
            CronState {
                days: 5,
                holdoff: 2,
            },
        );
        assert_eq!(listed(&mut session), (false, false), "in its pre-holdoff");
    }

    #[test]
    fn a_crons_dates_are_read_on_the_sessions_date() {
        let mut autumn = scripted(128, 2, "b10", "");
        autumn.first_month = 9;
        autumn.first_day = 1;
        let catalog = timing(vec![autumn]);
        let mut session = quiet(&catalog);
        let year = session.date().year();
        let date = |month, day| GameDate::new(year, month, day).expect("a date");
        session.set_date(date(8, 30));
        session.let_days_pass(1, &mut NeverFires);
        assert_eq!(session.date(), date(8, 31));
        assert!(!session.control_bit(bit(10)), "31 Aug is before its window");
        session.let_days_pass(1, &mut NeverFires);
        assert_eq!(session.date(), date(9, 1));
        assert!(session.control_bit(bit(10)), "its OnStart on 1 Sep");
    }

    /// Crön 129 requiring 0x4, its `OnStart` setting b12, and `others`.
    fn requiring(others: Vec<CronRecord>) -> FakePilotCatalog {
        let mut needy = scripted(129, 2, "b12", "");
        needy.require = 0x4;
        let mut crons = others;
        crons.push(needy);
        timing(crons)
    }

    #[test]
    fn a_crons_require_is_met_by_another_running_crons_contribute() {
        let mut giver = scripted(128, 5, "b11", "");
        giver.contribute = 0x4;
        giver.enable_on = Test::parse("b5");
        let catalog = requiring(vec![giver]);
        let mut session = quiet(&catalog);
        session.let_days_pass(2, &mut NeverFires);
        assert!(!session.control_bit(bit(12)), "nothing contributes 0x4");
        session.set_control_bit(bit(5), true);
        session.let_days_pass(1, &mut NeverFires);
        assert!(session.control_bit(bit(11)));
        assert!(session.control_bit(bit(12)), "crön 128 contributes 0x4");
    }

    #[test]
    fn a_crons_require_is_met_by_an_outfit_owned() {
        let mut catalog = requiring(Vec::new());
        catalog.outfits = vec![OutfitRecord {
            contribute: 0x4,
            ..outfit(300, &[])
        }];
        let mut session = quiet(&catalog);
        session.let_days_pass(2, &mut NeverFires);
        assert!(!session.control_bit(bit(12)), "nothing contributes 0x4");
        session.pilot.outfits.insert(OutfitId(300), 1);
        session.let_days_pass(1, &mut NeverFires);
        assert!(session.control_bit(bit(12)), "outfit 300 contributes 0x4");
    }

    #[test]
    fn a_running_cron_carries_on_through_a_save_to_its_end() {
        let catalog = timing(vec![scripted(128, 3, "b10", "!b10")]);
        let mut session = quiet(&catalog);
        session.let_days_pass(2, &mut NeverFires);
        let pilot = save::decode(&save::encode(session.pilot())).expect("loads");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().crons().get(&CronId(128)),
            Some(&CronState {
                days: 2,
                holdoff: 0
            })
        );
        session.let_days_pass(1, &mut NeverFires);
        assert!(session.control_bit(bit(10)));
        session.let_days_pass(1, &mut NeverFires);
        assert!(!session.control_bit(bit(10)), "its OnEnd on the fourth day");
    }

    #[test]
    fn a_crons_end_reads_the_sessions_rules() {
        let ending = CronRecord {
            post_holdoff: 5,
            ..scripted(128, 1, "b10", "!b10")
        };
        let catalog = timing(vec![ending]);
        let engine =
            Rulebook::default().with_override(RuleKey::CronPostHoldoff, RuleSource::Engine);
        let mut session = quiet(&catalog).with_rules(engine);
        session.let_days_pass(2, &mut NeverFires);
        assert!(!session.control_bit(bit(10)), "its OnEnd");
        session.set_control_bit(bit(10), true);
        session.let_days_pass(1, &mut NeverFires);
        assert!(!session.control_bit(bit(10)), "its OnEnd again");
        let held = CronState {
            days: -1,
            holdoff: 0,
        };
        assert_eq!(session.pilot().crons().get(&CronId(128)), Some(&held));
        let pilot = save::decode(&save::encode(session.pilot())).expect("loads");
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().crons().get(&CronId(128)),
            Some(&held),
            "kept through a save"
        );

        let mut session = quiet(&catalog);
        session.let_days_pass(2, &mut NeverFires);
        session.set_control_bit(bit(10), true);
        session.let_days_pass(1, &mut NeverFires);
        assert!(session.control_bit(bit(10)), "in its post-holdoff");
    }

    #[test]
    fn a_save_naming_a_missing_cron_flies_with_it_inactive() {
        let catalog = FakePilotCatalog {
            crons: vec![cron(128)],
            ..catalog()
        };
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        let running = CronState {
            days: 3,
            holdoff: 0,
        };
        pilot.crons.insert(CronId(128), running);
        pilot.crons.insert(CronId(300), running);
        let pilot = save::decode(&save::encode(&pilot)).expect("loads");
        assert_eq!(pilot.crons().len(), 2, "the save keeps it");
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().crons().iter().collect::<Vec<_>>(),
            [(&CronId(128), &running)]
        );
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        assert_eq!(saved.crons().keys().collect::<Vec<_>>(), [&CronId(128)]);
    }

    /// Crön 128 with `flags`, running `duration` days, its `OnStart`
    /// `on_start`, its `OnEnd` `on_end` and its `EnableOn` `enable_on`.
    fn flagged(
        flags: u16,
        duration: i16,
        on_start: &str,
        on_end: &str,
        enable_on: &str,
    ) -> CronRecord {
        CronRecord {
            flags,
            enable_on: Test::parse(enable_on),
            ..scripted(128, duration, on_start, on_end)
        }
    }

    fn capped(hook: CronHook) -> SimDiagnostic {
        SimDiagnostic::CronRunsCapped {
            cron: CronId(128),
            hook,
        }
    }

    #[test]
    fn an_iterative_on_start_runs_until_its_enable_on_fails() {
        let catalog = timing(vec![flagged(
            ITERATE_START,
            3,
            "b11 ^b10 ^b12",
            "",
            "!b11 | b10",
        )]);
        let mut session = quiet(&catalog);
        session.let_days_pass(1, &mut NeverFires);
        assert!(session.control_bit(bit(11)));
        assert!(!session.control_bit(bit(10)), "an even number of runs");
        assert!(!session.control_bit(bit(12)), "two runs, not three");
        assert_eq!(session.take_diagnostics(), []);
    }

    #[test]
    fn an_iterative_on_start_runs_zero_times_after_a_pre_holdoff_that_turned_its_enable_on_false() {
        for (flags, runs) in [(ITERATE_START, false), (0, true)] {
            let mut held = flagged(flags, 3, "b10", "", "!b5");
            held.pre_holdoff = 1;
            let catalog = timing(vec![held]);
            let mut session = quiet(&catalog);
            session.let_days_pass(1, &mut NeverFires);
            assert_eq!(session.pilot().crons().len(), 1, "started");
            session.set_control_bit(bit(5), true);
            session.let_days_pass(1, &mut NeverFires);
            assert_eq!(session.control_bit(bit(10)), runs, "flags {flags}");
        }
    }

    #[test]
    fn an_iterative_on_start_is_capped_at_10001_runs_with_a_diagnostic() {
        let catalog = timing(vec![flagged(ITERATE_START, 3, "^b10", "", "")]);
        let mut session = quiet(&catalog);
        session.let_days_pass(1, &mut NeverFires);
        assert!(session.control_bit(bit(10)), "10001 runs, an odd number");
        assert_eq!(session.take_diagnostics(), [capped(CronHook::OnStart)]);
        assert_eq!(session.take_diagnostics(), []);
    }

    #[test]
    fn a_non_iterative_cron_runs_once() {
        let catalog = timing(vec![flagged(0, 3, "^b10", "", "")]);
        let mut session = quiet(&catalog);
        session.let_days_pass(1, &mut NeverFires);
        assert!(session.control_bit(bit(10)));
        assert_eq!(session.take_diagnostics(), []);
    }

    #[test]
    fn an_iterative_on_end_does_not_run_when_its_enable_on_is_false_at_the_end() {
        for (flags, runs) in [(ITERATE_END, false), (0, true)] {
            let catalog = timing(vec![flagged(flags, 1, "b20", "b21", "!b20")]);
            let mut session = quiet(&catalog);
            session.let_days_pass(2, &mut NeverFires);
            assert_eq!(session.control_bit(bit(21)), runs, "flags {flags}");
            assert!(session.pilot().crons().is_empty(), "ended");
        }
    }

    /// Crön 128 requiring 0x4 with `flags`, its `OnStart` selling the
    /// owned outfit 300 that contributes 0x4, then toggling b10; crön 128
    /// contributing `contribute`.
    fn selling(contribute: u64) -> Session {
        let mut seller = flagged(ITERATE_START, 3, "D300 ^b10", "", "");
        seller.require = 0x4;
        seller.contribute = contribute;
        let mut catalog = timing(vec![seller]);
        catalog.outfits = vec![OutfitRecord {
            contribute: 0x4,
            ..outfit(300, &[])
        }];
        let mut session = quiet(&catalog);
        session.pilot.outfits.insert(OutfitId(300), 1);
        session.let_days_pass(1, &mut NeverFires);
        session
    }

    #[test]
    fn an_iterative_loop_stops_when_a_run_makes_require_fail() {
        let mut session = selling(0);
        assert!(session.control_bit(bit(10)), "one run");
        assert!(
            session
                .pilot
                .outfits
                .get(&OutfitId(300))
                .is_none_or(|n| *n == 0)
        );
        assert_eq!(session.take_diagnostics(), []);
    }

    #[test]
    fn a_crons_own_contribute_keeps_its_own_require_met() {
        let mut session = selling(0x4);
        assert!(session.control_bit(bit(10)), "10001 runs");
        assert_eq!(session.take_diagnostics(), [capped(CronHook::OnStart)]);
    }
}
