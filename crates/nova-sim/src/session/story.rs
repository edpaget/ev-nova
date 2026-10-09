//! The session's side of the story runtime: the story events raised from
//! their sources and waiting for the next settling, the ships watched for
//! their first sighting (see the `story` module's "Story events"), and the
//! daily step's participants run over the session's own state (its "Daily
//! step").

use std::collections::{BTreeSet, VecDeque};

use super::Session;
use crate::chance::Chance;
use crate::market;
use crate::story::day::{self, Day, Participant};
use crate::story::{Fate, StoryEvent};
use crate::traffic::npc::NpcId;

/// The session's story runtime: the events raised and not yet settled,
/// and the ships watched for their first sighting. None of it is saved.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct StoryRuntime {
    /// The events raised since the last settling, in the order raised.
    raised: VecDeque<StoryEvent>,
    /// The NPCs whose first sighting raises [`Fate::Observed`].
    pub(super) watched: BTreeSet<NpcId>,
}

/// One day of the session, its draws on `chance`.
struct SessionDay<'a, C: Chance + ?Sized> {
    session: &'a mut Session,
    chance: &'a mut C,
}

impl<C: Chance + ?Sized> Day for SessionDay<'_, C> {
    fn run(&mut self, participant: Participant) {
        let session = &mut *self.session;
        match participant {
            Participant::Date => session.pilot.date = session.pilot.date.next_day(),
            Participant::Events => {
                // The events step out of the pilot, so that their
                // `ActivateOn` can read the pilot through the session's
                // control bits: no test reads the events.
                let mut events = std::mem::take(&mut session.pilot.events);
                market::step_day(&session.goods, &mut events, session.gate(), self.chance);
                session.pilot.events = events;
            }
            Participant::DayPassed => session.raise(StoryEvent::DayPassed {
                date: session.pilot.date,
            }),
            // Each slot is filled by its own phase: the crons by
            // crons-news-ranks phase 1, the missions' days left by missions
            // phase 2, the tribute and the stellars' defence by rdm task
            // planet-domination, the stellars' regeneration by
            // story-runtime phase 5, the reinforcements once they are
            // built, and the ranks by crons-news-ranks phase 3. The day
            // rolls are still drawn afresh at each landing.
            Participant::Crons
            | Participant::MissionDays
            | Participant::Tribute
            | Participant::Stellars
            | Participant::Reinforcements
            | Participant::Ranks
            | Participant::DayRolls => {}
        }
    }
}

impl Session {
    /// Raises `event` from its one source: it waits, with the others
    /// raised, for the next settling ([`Session::settle_script`]). A
    /// source only raises; nothing here dispatches.
    pub(super) fn raise(&mut self, event: StoryEvent) {
        self.story.raised.push_back(event);
    }

    /// How many events have been raised and not yet settled: a mark to
    /// raise earlier ones at ([`Session::raise_at`]).
    pub(super) fn story_mark(&self) -> usize {
        self.story.raised.len()
    }

    /// Raises `event` at `mark`, before the events raised since: where a
    /// source learns of an event only after later ones.
    pub(super) fn raise_at(&mut self, mark: usize, event: StoryEvent) {
        self.story.raised.insert(mark, event);
    }

    /// Watches the NPC `ship`: the next flight tick of the traffic that
    /// finds it in the system raises its first sighting,
    /// [`Fate::Observed`], once, and forgets it. A ship no longer in the
    /// system is forgotten, and populating the system or resetting its
    /// traffic forgets every one, its ships replaced (see the `story`
    /// module's findings).
    pub fn watch_ship(&mut self, ship: NpcId) {
        self.story.watched.insert(ship);
    }

    /// Raises the first sighting of each watched ship in the system, and
    /// forgets every watched ship: seen, or no longer here. Every ship in
    /// the system is visible, as the engine has no cloak and no ship
    /// emerging from a gate (`_ShipVisibleToShip` @0x971c).
    pub(super) fn observe_watched(&mut self) {
        for ship in std::mem::take(&mut self.story.watched) {
            if self.traffic.npcs().iter().any(|npc| npc.id == ship) {
                self.raise(StoryEvent::ShipFate {
                    ship,
                    fate: Fate::Observed,
                });
            }
        }
    }

    /// Lets go of the events raised: the settling until the dispatcher
    /// takes them.
    pub(super) fn settle_story(&mut self) {
        self.story.raised.clear();
    }

    /// The events raised and not yet settled, in order.
    #[cfg(test)]
    pub(crate) fn story_raised(&self) -> Vec<StoryEvent> {
        self.story.raised.iter().copied().collect()
    }

    /// Lets `days` days go by through the daily step, each running its
    /// participants in the original's order, the events rolled on
    /// `chance`. No wage is paid here: the caller pays once after its
    /// days, as the original does.
    pub(super) fn pass_days(&mut self, days: u32, chance: &mut (impl Chance + ?Sized)) {
        day::pass_days(
            days,
            &mut SessionDay {
                session: self,
                chance,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{DisasterId, OutfitId, ShipId, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::date::GameDate;
    use crate::market::ActiveEvent;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Settled;
    use crate::session::tests::{NEW, buy, outfitted, sell, shipbuying, surplus};
    use crate::story::{Entrance, StoryEvent};
    use crate::testkit::{self, FakePilotCatalog, Scripted, jump, land_now};

    const SPEED: OutfitId = OutfitId(300);

    /// The date `days` days after `date`.
    fn after(date: GameDate, days: usize) -> GameDate {
        std::iter::successors(Some(date), |date| Some(date.next_day()))
            .nth(days)
            .expect("a date")
    }

    /// Takes off from `session`, landed, and gives how many days passed.
    fn days_at_take_off(session: &mut Session) -> usize {
        let landed = session.date();
        session.take_off(&mut NeverFires).expect("takes off");
        (0..10)
            .find(|&days| after(landed, days) == session.date())
            .expect("fewer than ten days")
    }

    /// [`shipbuying`]'s session, landed at its outfitter and shipyard.
    fn shopping() -> Session {
        outfitted(&shipbuying())
    }

    #[test]
    fn a_take_off_passes_a_day_and_rolls_its_events() {
        let catalog = surplus();
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131).expect("arrives");
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        let landed = session.date();
        let mut chance = Scripted::default();
        session.take_off(&mut chance).expect("takes off");
        assert_eq!(session.date(), after(landed, 1));
        assert_eq!((landed.day(), landed.month(), landed.year()), (24, 6, 1177));
        assert_eq!(chance.asked, [35], "one roll for the one day");
    }

    #[test]
    fn an_outfit_bought_or_sold_adds_a_day_at_take_off() {
        let mut session = shopping();
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        assert_eq!(days_at_take_off(&mut session), 2, "bought");

        let mut session = shopping();
        session.transact(|pilot| {
            pilot.outfits.insert(SPEED, 1);
        });
        assert_eq!(session.outfit(sell(SPEED), &mut NeverFires), Ok(()));
        assert_eq!(days_at_take_off(&mut session), 2, "sold");

        let mut session = shopping();
        assert!(session.outfit(sell(SPEED), &mut NeverFires).is_err());
        assert_eq!(days_at_take_off(&mut session), 1, "refused");
    }

    #[test]
    fn a_cancelled_count_of_an_allowed_order_still_adds_the_day() {
        let mut session = shopping();
        assert_eq!(
            session.outfit_counted(buy(SPEED), 0, &mut NeverFires),
            Ok(0)
        );
        assert_eq!(session.pilot().owned(SPEED), 0, "nothing moved");
        assert_eq!(days_at_take_off(&mut session), 2, "allowed");

        let mut session = shopping();
        assert_eq!(
            session.outfit_counted(sell(SPEED), 0, &mut NeverFires),
            Ok(0)
        );
        assert_eq!(days_at_take_off(&mut session), 1, "refused");
    }

    #[test]
    fn a_ship_bought_adds_four_days_at_take_off() {
        let mut session = shopping();
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("buys");
        assert_eq!(days_at_take_off(&mut session), 5);

        let mut session = shopping();
        assert!(
            session
                .buy_ship(ShipId(999), "Kestrel", &mut NeverFires)
                .is_err()
        );
        assert_eq!(days_at_take_off(&mut session), 1, "refused");
    }

    #[test]
    fn an_outfit_and_a_ship_add_five_days() {
        let mut session = shopping();
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("buys");
        assert_eq!(days_at_take_off(&mut session), 6);
    }

    #[test]
    fn escorts_sold_or_upgraded_pass_half_their_count() {
        let mut session = shopping();
        session.stay.escorts_changed = 4;
        assert_eq!(days_at_take_off(&mut session), 3);
    }

    #[test]
    fn a_new_landing_forgets_the_last_stay() {
        let mut session = shopping();
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        assert_eq!(days_at_take_off(&mut session), 2);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(days_at_take_off(&mut session), 1);
    }

    #[test]
    fn by_the_other_reading_a_stay_passes_no_day() {
        let rules = Rulebook::default().with_override(RuleKey::StayDays, RuleSource::Bible);
        assert_eq!(
            shopping().rules().source_for(RuleKey::StayDays),
            RuleSource::Engine,
            "by default"
        );
        let mut session = shopping().with_rules(rules);
        assert_eq!(
            session.rules().source_for(RuleKey::StayDays),
            RuleSource::Bible
        );
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("buys");
        session.stay.escorts_changed = 4;
        assert_eq!(days_at_take_off(&mut session), 0);

        let catalog = surplus();
        let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
        jump(&mut session, &catalog, 131).expect("arrives");
        land_now(&mut session).expect("lands");
        let mut chance = Scripted::default();
        session.take_off(&mut chance).expect("takes off");
        assert_eq!(chance.asked, Vec::<u8>::new(), "no roll");
    }

    #[test]
    fn the_date_saved_at_take_off_holds_its_days() {
        let mut session = shopping();
        let landed = session.date();
        session
            .buy_ship(NEW, "Kestrel", &mut NeverFires)
            .expect("buys");
        session.take_off(&mut NeverFires).expect("takes off");
        assert!(session.take_save_due());
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        assert_eq!(saved.date(), after(landed, 5));
    }

    /// [`surplus`]'s session, its event under way with 5 days left.
    fn eventful(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.pilot.events.insert(
            DisasterId(128),
            ActiveEvent {
                days: 5,
                stellar: None,
            },
        );
        session
    }

    #[test]
    fn set_date_runs_no_participant() {
        let catalog = surplus();
        let mut session = eventful(&catalog);
        let date = after(session.date(), 3);
        session.set_date(date);
        assert_eq!(session.date(), date);
        assert_eq!(
            session.pilot().events().collect::<Vec<_>>(),
            [(DisasterId(128), 5)]
        );
    }

    #[test]
    fn loading_a_save_passes_no_day() {
        let catalog = surplus();
        let session = eventful(&catalog);
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert_eq!(flown.date(), session.date());
        assert_eq!(
            flown.pilot().events().collect::<Vec<_>>(),
            [(DisasterId(128), 5)]
        );
    }

    const LANDED_128: StoryEvent = StoryEvent::Landed {
        stellar: StellarId(128),
    };
    const TOOK_OFF_128: StoryEvent = StoryEvent::TookOff {
        stellar: StellarId(128),
    };

    #[test]
    fn raised_events_queue_in_order_until_a_settle_takes_them() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.story_raised(), [], "nothing at first");
        session.raise(LANDED_128);
        session.raise(TOOK_OFF_128);
        assert_eq!(session.story_raised(), [LANDED_128, TOOK_OFF_128]);
        assert_eq!(session.take_events(), [], "never the view's outbox");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.story_raised(), [], "settled");
    }

    /// `session` with the events raised so far let go of, so a test sees
    /// only what follows.
    fn quiet(mut session: Session) -> Session {
        session.story.raised.clear();
        session
    }

    #[test]
    fn a_landing_raises_landed_once() {
        let catalog = testkit::catalog();
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        session.land().expect("selects");
        assert_eq!(session.story_raised(), [], "the first L only selects");
        session.land().expect("lands");
        assert_eq!(session.story_raised(), [LANDED_128]);
    }

    #[test]
    fn a_refused_landing_or_a_press_over_a_gate_raises_nothing() {
        let empty = FakePilotCatalog {
            sites: Vec::new(),
            ..testkit::catalog()
        };
        let mut session = quiet(Session::start(&empty).expect("starts"));
        assert!(land_now(&mut session).is_err());
        assert_eq!(session.story_raised(), [], "refused");

        let catalog = crate::session::tests::gated();
        let session = Session::start(&catalog).expect("starts");
        let raised_at_start = session.story_raised();
        let session = crate::session::tests::at_gate(&catalog);
        assert_eq!(session.story_raised(), raised_at_start, "over a gate");
    }

    /// `DayPassed` for each of the `days` days after `from`, in order.
    fn days_after(from: GameDate, days: usize) -> Vec<StoryEvent> {
        (1..=days)
            .map(|n| StoryEvent::DayPassed {
                date: after(from, n),
            })
            .collect()
    }

    const EXPLORED_130: StoryEvent = StoryEvent::Explored {
        system: SystemId(130),
    };

    #[test]
    fn a_take_off_raises_its_days_then_explored_then_took_off() {
        let mut session = quiet(shopping());
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        let landed = session.date();
        session.take_off(&mut NeverFires).expect("takes off");
        let mut expected = days_after(landed, 2);
        expected.extend([EXPLORED_130, TOOK_OFF_128]);
        assert_eq!(session.story_raised(), expected);
    }

    #[test]
    fn by_the_other_stay_reading_a_take_off_raises_only_explored_and_took_off() {
        let rules = Rulebook::default().with_override(RuleKey::StayDays, RuleSource::Bible);
        let mut session = quiet(shopping().with_rules(rules));
        assert_eq!(session.outfit(buy(SPEED), &mut NeverFires), Ok(()));
        session.take_off(&mut NeverFires).expect("takes off");
        assert_eq!(session.story_raised(), [EXPLORED_130, TOOK_OFF_128]);
    }

    #[test]
    fn a_take_off_unlanded_raises_nothing() {
        let catalog = testkit::catalog();
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        assert_eq!(session.take_off(&mut NeverFires), None);
        assert_eq!(session.story_raised(), []);
    }

    #[test]
    fn a_jump_raises_a_day_passed_for_each_of_its_days_dated_that_day() {
        let catalog = crate::session::tests::surplus();
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        session.stats.jump_days = 3;
        let left = session.date();
        jump(&mut session, &catalog, 131).expect("arrives");
        let days: Vec<StoryEvent> = session
            .story_raised()
            .into_iter()
            .filter(|event| matches!(event, StoryEvent::DayPassed { .. }))
            .collect();
        assert_eq!(days, days_after(left, 3));
    }

    #[test]
    fn set_date_and_a_load_raise_no_day() {
        let catalog = surplus();
        let mut session = quiet(eventful(&catalog));
        session.set_date(after(session.date(), 3));
        assert_eq!(session.story_raised(), []);
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let flown = Session::fly(&catalog, saved).expect("flies");
        assert!(
            !flown
                .story_raised()
                .iter()
                .any(|event| matches!(event, StoryEvent::DayPassed { .. })),
            "{:?}",
            flown.story_raised()
        );
    }

    fn entered(system: i16, how: Entrance) -> StoryEvent {
        StoryEvent::EnteredSystem {
            system: SystemId(system),
            how,
        }
    }

    fn explored(system: i16) -> StoryEvent {
        StoryEvent::Explored {
            system: SystemId(system),
        }
    }

    #[test]
    fn a_jump_raises_the_arrivals_explored_then_its_days_then_entered_system() {
        let catalog = testkit::catalog();
        let mut session = quiet(Session::start(&catalog).expect("starts"));
        session.stats.jump_days = 2;
        let left = session.date();
        jump(&mut session, &catalog, 131).expect("arrives");
        let mut expected = vec![explored(131)];
        expected.extend(days_after(left, 2));
        expected.push(entered(131, Entrance::Jump));
        assert_eq!(session.story_raised(), expected);
    }

    #[test]
    fn a_gate_raises_entered_system_then_explored_its_days_first_like_a_jump() {
        use crate::gate::GateArrivalRule;
        use crate::session::tests::{at_gate, gated};
        let catalog = gated();
        let mut session = quiet(at_gate(&catalog));
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
            .expect("enters");
        assert_eq!(
            session.story_raised(),
            [entered(131, Entrance::Gate), explored(131)],
            "by the engine"
        );

        let mut session = quiet(at_gate(&catalog).with_gate_arrival(GateArrivalRule::LikeJump));
        let left = session.date();
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
            .expect("enters");
        let mut expected = days_after(left, 1);
        expected.extend([entered(131, Entrance::Gate), explored(131)]);
        assert_eq!(session.story_raised(), expected, "like a jump");
    }

    /// `session` after running `M131` and settling its move.
    fn moved(mut session: Session, catalog: &FakePilotCatalog) -> Vec<StoryEvent> {
        let expr = SetExpr::parse("M131").expect("parses");
        session.run_set(&expr, &mut NeverFires);
        let mut session = quiet(session);
        session.settle_moves(catalog, &mut NeverFires, &mut Settled::default());
        session.story_raised()
    }

    #[test]
    fn a_move_raises_entered_system_and_as_an_arrival_explored() {
        let catalog = testkit::catalog();
        for (reading, landed) in [
            (RuleSource::Engine, true),
            (RuleSource::Engine, false),
            (RuleSource::Bible, true),
            (RuleSource::Bible, false),
        ] {
            let rules = Rulebook::default().with_override(RuleKey::MoveArrival, reading);
            let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
            if landed {
                land_now(&mut session).expect("lands");
            }
            let mut expected = vec![entered(131, Entrance::ScriptMove { landed })];
            if reading == RuleSource::Bible {
                expected.push(explored(131));
            }
            assert_eq!(moved(session, &catalog), expected, "{reading:?} {landed}");
        }
    }

    #[test]
    fn a_relocation_raises_entered_system_then_explored() {
        let catalog = testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        let mut session = quiet(session);
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("moves");
        assert_eq!(
            session.story_raised(),
            [entered(131, Entrance::Relocate), explored(131)]
        );
    }

    #[test]
    fn a_multi_jump_raises_the_passed_systems_explored_before_its_days() {
        use crate::hyperspace::MultiJumpRule;
        use crate::session::tests::{bound_for_134, chained};
        for (rule, mod_val, days) in [(MultiJumpRule::Engine, 2, 1), (MultiJumpRule::PerHop, 1, 2)]
        {
            let catalog = chained(testkit::catalog(), mod_val);
            let mut session = bound_for_134(&catalog, rule);
            testkit::begin_jump_now(&mut session).expect("jumps");
            let mut session = quiet(session);
            let left = session.date();
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(132))
            );
            let mut expected = vec![explored(131), explored(132)];
            expected.extend(days_after(left, days));
            expected.push(entered(132, Entrance::Jump));
            assert_eq!(session.story_raised(), expected, "{rule:?}");
        }
    }

    #[test]
    fn a_jump_off_its_course_raises_the_system_it_reaches() {
        use crate::hyperspace::MultiJumpRule;
        use crate::session::tests::{bound_for_134, chained};
        let catalog = chained(testkit::catalog(), 10);
        let mut session = bound_for_134(&catalog, MultiJumpRule::Engine);
        testkit::begin_jump_now(&mut session).expect("jumps");
        session.plot_course(SystemId(133)).expect("a route");
        let mut session = quiet(session);
        let left = session.date();
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        let mut expected = vec![explored(131)];
        expected.extend(days_after(left, 1));
        expected.push(entered(131, Entrance::Jump));
        assert_eq!(session.story_raised(), expected);
    }
}
