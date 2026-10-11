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
//! next settling.
//!
//! The pilot keeps each crön running or holding off, its `days` and
//! `holdoff`, keyed by ID. A pilot flown from a save naming a crön the
//! data no longer has forgets it, so it is inactive and the next save
//! leaves it out, as the original's `_LoadPilotData` forces a slot whose
//! crön is gone to inactive (@0x75fbc-0x76025).
//!
//! **Contribute.** Every reader of the `Contribute` bits the player
//! holds sums them through one function, [`wares::contributed`]: the
//! mission offers, the bar's hires ([`Session::contributed`]), the
//! outfitter and the shipyard.

use std::rc::Rc;

use crate::catalog::{CronId, CronRecord};
use crate::chance::Chance;
use crate::control::Script;
use crate::cron::{self, CronDay, CronFacts, CronState};
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
        self.session.save_due = true;
    }

    fn eligible(&mut self, record: &CronRecord) -> bool {
        let session = &*self.session;
        let facts = CronFacts {
            record,
            today: session.pilot.date,
            contributed: session.contributed(),
            gate: session.gate(),
            rules: &session.rules,
        };
        cron::eligibility()
            .first_refusal(&facts, &mut &mut *self.chance)
            .is_ok()
    }

    /// Runs `script` as a participant's effect, its `M`, `N`, `Q` and `S`
    /// waiting for the next settling as any hook's do.
    fn run(&mut self, _id: SubscriptionId, script: &Script) {
        self.session.run_script(script, &mut *self.chance);
    }
}

impl Session {
    /// The daily step's second participant: one pass over the crons (see
    /// [`cron`](crate::cron)), drawing on `chance`.
    pub(super) fn crons_day(&mut self, chance: &mut (impl Chance + ?Sized)) {
        let crons = Rc::clone(&self.crons);
        cron::pass(
            &crons,
            &mut SessionCrons {
                session: self,
                chance,
            },
        );
    }

    /// The `Contribute` bits the player holds, as they stand
    /// (`_GetPlayerContributeBits` @0x76b2; see [`wares::contributed`]).
    pub(super) fn contributed(&self) -> u64 {
        let by = Contributors {
            ship: self.fields.contribute,
            outfits: &self.outfits,
            crons: &self.crons,
        };
        wares::contributed(&by, &self.pilot)
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{CronId, CronRecord, MissionId, MissionRecord, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::{Bit, Script, Test};
    use crate::cron::CronState;
    use crate::mission::Refusal;
    use crate::pilot::Pilot;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::rules::Verdict;
    use crate::save;
    use crate::session::Session;
    use crate::story::{Consumer, EventFilter, StoryEventKind, Subscription, SubscriptionId};
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, cron, land_now, mission};

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

    #[test]
    fn a_running_crons_contribute_unlocks_a_missions_require() {
        let catalog = contributing();
        let mut session = landed(&catalog);
        assert!(requires(&session));
        assert_eq!(session.contributed() & 0x4, 0);
        session.pilot.crons.insert(
            CronId(128),
            CronState {
                days: 5,
                holdoff: 0,
            },
        );
        assert!(!requires(&session));
        assert_eq!(session.contributed() & 0x4, 0x4);
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
    fn scripted(id: i16, duration: i16, on_start: &str, on_end: &str) -> CronRecord {
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
    fn a_crons_move_waits_for_the_settling() {
        let catalog = timing(vec![scripted(128, 2, "M131", "")]);
        let mut session = quiet(&catalog);
        session.let_days_pass(1, &mut NeverFires);
        assert_eq!(session.system(), SystemId(130), "not yet");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
    }

    #[test]
    fn a_cron_contributes_while_running_and_not_in_its_post_holdoff() {
        let mut held = scripted(128, 1, "", "");
        held.post_holdoff = 2;
        held.contribute = 0x4;
        let catalog = timing(vec![held]);
        let mut session = quiet(&catalog);
        assert_eq!(session.contributed() & 0x4, 0);
        session.let_days_pass(1, &mut NeverFires);
        assert_eq!(session.contributed() & 0x4, 0x4, "running");
        session.let_days_pass(1, &mut NeverFires);
        assert_eq!(
            session.pilot().crons().get(&CronId(128)),
            Some(&CronState {
                days: -1,
                holdoff: 2
            })
        );
        assert_eq!(session.contributed() & 0x4, 0, "in its post-holdoff");
        session.let_days_pass(2, &mut NeverFires);
        assert!(session.pilot().crons().is_empty());
        assert_eq!(session.contributed() & 0x4, 0);
    }

    #[test]
    fn a_crons_contribute_unlocks_a_missions_require_after_its_first_day() {
        let catalog = contributing();
        let mut session = landed(&catalog);
        assert!(requires(&session));
        session.let_days_pass(1, &mut NeverFires);
        assert!(!requires(&session));
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
}
