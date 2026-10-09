//! The session's side of the story runtime: the daily step's participants
//! run over the session's own state (see the `story` module's "Daily
//! step").

use super::Session;
use crate::chance::Chance;
use crate::market;
use crate::story::day::{self, Day, Participant};

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
            // Each slot is filled by its own phase: the crons by
            // crons-news-ranks phase 1, the missions' days left by missions
            // phase 2, the tribute and the stellars' defence by rdm task
            // planet-domination, the stellars' regeneration by
            // story-runtime phase 5, the reinforcements once they are
            // built, the ranks by crons-news-ranks phase 3, and `DayPassed`
            // by story-runtime phase 3. The day rolls are still drawn
            // afresh at each landing.
            Participant::Crons
            | Participant::MissionDays
            | Participant::Tribute
            | Participant::Stellars
            | Participant::Reinforcements
            | Participant::Ranks
            | Participant::DayRolls
            | Participant::DayPassed => {}
        }
    }
}

impl Session {
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
    use crate::catalog::{DisasterId, OutfitId, ShipId, StellarId};
    use crate::chance::NeverFires;
    use crate::date::GameDate;
    use crate::market::ActiveEvent;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::tests::{NEW, buy, outfitted, sell, shipbuying, surplus};
    use crate::testkit::{FakePilotCatalog, Scripted, jump, land_now};

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
}
