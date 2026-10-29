//! [`Session::apply`]'s spaceport arms: the exchange, the recharge, the
//! outfitter, the shipyard, the bar, its hire screen and its race, the jettison,
//! the assignment of a ship captured and the intro's mark, each matched
//! to the session method that runs it and answered as
//! [`Session::apply_port`] says. None of them runs in `_DoPlayGameWork`'s
//! fixed step, so none is the step's ([`Session::step`]): the view sends
//! each as the player acts on its screen, and orders the bar's redraws
//! around it.

use super::ApplyCollaborators;
use crate::flight_frame::{Answer, FlightCommand};
use crate::session::Session;

impl Session {
    /// Runs the spaceport `command`, drawing on `collab`'s chance where
    /// its method does, and answers it:
    ///
    /// - [`FlightCommand::Trade`] runs [`Session::trade`]: [`Answer::Traded`],
    ///   its refusal among them.
    /// - [`FlightCommand::Recharge`] runs [`Session::recharge`]:
    ///   [`Answer::Recharged`].
    /// - [`FlightCommand::Outfit`] and [`FlightCommand::OutfitCounted`] run
    ///   [`Session::outfit_order`] and [`Session::outfit_counted_run`]:
    ///   [`Answer::Outfitted`], the whole run.
    /// - [`FlightCommand::BuyShip`] runs [`Session::buy_ship`]:
    ///   [`Answer::ShipBought`].
    /// - [`FlightCommand::Hire`] runs [`Session::hire`]: [`Answer::Hired`].
    /// - [`FlightCommand::OpenBar`] runs [`Session::open_bar`]: done, or
    ///   refused with no bar to open, changing nothing.
    /// - [`FlightCommand::Race`] runs [`Session::race`]: [`Answer::Raced`],
    ///   or refused when no race runs, changing nothing.
    /// - [`FlightCommand::CollectWinnings`] runs
    ///   [`Session::collect_winnings`]: done, or refused with none due.
    /// - [`FlightCommand::Assign`] runs [`Session::assign`]:
    ///   [`Answer::Assigned`], or refused with no capture awaiting its
    ///   assignment, changing nothing.
    /// - [`FlightCommand::OpenOutfitter`], [`FlightCommand::CloseOutfitter`],
    ///   [`FlightCommand::CloseHire`], [`FlightCommand::DeclineShip`],
    ///   [`FlightCommand::Jettison`] and [`FlightCommand::MarkIntroSeen`] run
    ///   [`Session::open_outfitter`], [`Session::close_outfitter`],
    ///   [`Session::close_hire`], [`Session::decline_ship`],
    ///   [`Session::jettison_cargo`] and [`Session::mark_intro_seen`]: done.
    ///
    /// None reads the catalog, so none is refused for want of one.
    pub(super) fn apply_port(
        &mut self,
        command: FlightCommand,
        collab: &mut ApplyCollaborators<'_>,
    ) -> Answer {
        let chance = &mut *collab.chance;
        match command {
            FlightCommand::Trade(order) => Answer::Traded(self.trade(order)),
            FlightCommand::Recharge => Answer::Recharged(self.recharge()),
            FlightCommand::OpenOutfitter => {
                self.open_outfitter();
                Answer::Done
            }
            FlightCommand::CloseOutfitter => {
                self.close_outfitter();
                Answer::Done
            }
            FlightCommand::Outfit { order, count } => {
                Answer::Outfitted(self.outfit_order(order, count, chance))
            }
            FlightCommand::OutfitCounted { order, count } => {
                Answer::Outfitted(self.outfit_counted_run(order, count, chance))
            }
            FlightCommand::BuyShip { ship, name } => {
                Answer::ShipBought(self.buy_ship(ship, &name, chance))
            }
            FlightCommand::DeclineShip(ship) => {
                self.decline_ship(ship);
                Answer::Done
            }
            FlightCommand::OpenBar => {
                if self.open_bar(chance) {
                    Answer::Done
                } else {
                    Answer::Refused
                }
            }
            FlightCommand::Hire(ship) => Answer::Hired(self.hire(ship, chance)),
            FlightCommand::CloseHire => {
                self.close_hire();
                Answer::Done
            }
            FlightCommand::Race { racer, bet } => self
                .race(racer, bet, chance)
                .map_or(Answer::Refused, Answer::Raced),
            FlightCommand::CollectWinnings => super::done_if(self.collect_winnings()),
            FlightCommand::Jettison { all } => {
                self.jettison_cargo(all, chance);
                Answer::Done
            }
            FlightCommand::Assign(choice) => self
                .assign(choice, chance)
                .map_or(Answer::Refused, Answer::Assigned),
            FlightCommand::MarkIntroSeen => {
                self.mark_intro_seen();
                Answer::Done
            }
            _ => Answer::Refused,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Assignment;
    use crate::catalog::{OutfitId, ShipId};
    use crate::chance::NeverFires;
    use crate::gamble::Bet;
    use crate::landing::StellarFlags;
    use crate::market::{Direction, Good, Lot, Order};
    use crate::outfitter::OutfitOrder;
    use crate::session::apply::tests::apply;
    use crate::session::tests::{boardable, captured, shipbuying};
    use crate::testkit::{FakePilotCatalog, land_now};

    /// [`shipbuying`]'s Earth, a bar too: an exchange, an outfitter and a
    /// shipyard.
    fn port() -> FakePilotCatalog {
        let mut catalog = shipbuying();
        catalog.sites[0].1[0].flags |= StellarFlags::BAR;
        catalog
    }

    /// A session of `catalog`'s landed on its planet 128.
    fn docked(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session.take_save_due();
        session
    }

    /// Checks that `command` answers `session` as `run` answers a copy of
    /// it, drawing on a chance that never fires, and leaves the session as
    /// `run` leaves the copy: the arm runs the method `run` calls.
    fn runs(
        session: &mut Session,
        command: FlightCommand,
        run: impl FnOnce(&mut Session) -> Answer,
    ) {
        let mut direct = session.clone();
        let expected = run(&mut direct);
        let sent = format!("{command:?}");
        assert_eq!(apply(session, command), expected, "{sent}");
        assert_eq!(*session, direct, "{sent}: as the method leaves it");
    }

    /// An order for a ton of food in the exchange's first row.
    const FOOD: Order = Order {
        row: 0,
        good: Good::Commodity(0),
        direction: Direction::Buy,
        lot: Lot::Click,
    };

    /// The speed booster, bought.
    const BOOSTER: OutfitOrder = OutfitOrder {
        outfit: OutfitId(300),
        direction: Direction::Buy,
    };

    #[test]
    fn a_trade_answers_what_the_exchange_traded_or_its_refusal() {
        let mut session = docked(&port());
        runs(&mut session, FlightCommand::Trade(FOOD), |session| {
            Answer::Traded(session.trade(FOOD))
        });
        assert!(matches!(
            apply(&mut session, FlightCommand::Trade(FOOD)),
            Answer::Traded(Ok(_))
        ));
        let mut flying = Session::start(&port()).expect("starts");
        assert_eq!(
            apply(&mut flying, FlightCommand::Trade(FOOD)),
            Answer::Traded(Err(crate::market::TradeRefusal::NoMarket)),
            "no exchange in flight"
        );
    }

    #[test]
    fn a_recharge_answers_its_price_or_its_refusal() {
        let mut session = docked(&port());
        session.pilot.reserves.fuel.now = 1.0;
        runs(&mut session, FlightCommand::Recharge, |session| {
            Answer::Recharged(session.recharge())
        });
        let mut flying = Session::start(&port()).expect("starts");
        assert!(matches!(
            apply(&mut flying, FlightCommand::Recharge),
            Answer::Recharged(Err(_))
        ));
    }

    #[test]
    fn the_outfitters_orders_answer_their_runs() {
        let mut session = docked(&port());
        runs(&mut session, FlightCommand::OpenOutfitter, |session| {
            session.open_outfitter();
            Answer::Done
        });
        runs(
            &mut session,
            FlightCommand::Outfit {
                order: BOOSTER,
                count: 1,
            },
            |session| Answer::Outfitted(session.outfit_order(BOOSTER, 1, &mut NeverFires)),
        );
        runs(
            &mut session,
            FlightCommand::OutfitCounted {
                order: BOOSTER,
                count: 0,
            },
            |session| Answer::Outfitted(session.outfit_counted_run(BOOSTER, 0, &mut NeverFires)),
        );
        assert!(
            session.port.stay.outfit_traded,
            "a counted Cancel counts the outfit day"
        );
        runs(&mut session, FlightCommand::CloseOutfitter, |session| {
            session.close_outfitter();
            Answer::Done
        });
    }

    #[test]
    fn the_shipyards_purchase_and_decline_answer_as_the_shipyard_does() {
        let mut session = docked(&port());
        runs(
            &mut session,
            FlightCommand::DeclineShip(ShipId(129)),
            |session| {
                session.decline_ship(ShipId(129));
                Answer::Done
            },
        );
        let buy = FlightCommand::BuyShip {
            ship: ShipId(129),
            name: "Kestrel".to_owned(),
        };
        runs(&mut session, buy.clone(), |session| {
            Answer::ShipBought(session.buy_ship(ShipId(129), "Kestrel", &mut NeverFires))
        });
        let mut flying = Session::start(&port()).expect("starts");
        assert!(matches!(
            apply(&mut flying, buy),
            Answer::ShipBought(Err(_))
        ));
    }

    #[test]
    fn the_bar_opens_hires_and_closes_and_with_no_bar_is_refused() {
        let mut session = docked(&port());
        runs(&mut session, FlightCommand::OpenBar, |session| {
            assert!(session.open_bar(&mut NeverFires));
            Answer::Done
        });
        runs(&mut session, FlightCommand::Hire(ShipId(129)), |session| {
            Answer::Hired(session.hire(ShipId(129), &mut NeverFires))
        });
        runs(&mut session, FlightCommand::CloseHire, |session| {
            session.close_hire();
            Answer::Done
        });
        let mut flying = Session::start(&port()).expect("starts");
        let before = flying.clone();
        assert_eq!(apply(&mut flying, FlightCommand::OpenBar), Answer::Refused);
        assert_eq!(flying, before, "no bar: nothing changed");
    }

    #[test]
    fn a_race_answers_its_run_and_the_collect_credits_and_no_race_is_refused() {
        let mut session = docked(&port());
        assert!(session.open_bar(&mut NeverFires));
        session.pilot.cash = 5_000;
        let race = FlightCommand::Race {
            racer: 0,
            bet: Bet::House,
        };
        runs(&mut session, race.clone(), |session| {
            Answer::Raced(
                session
                    .race(0, Bet::House, &mut NeverFires)
                    .expect("a race"),
            )
        });
        runs(&mut session, FlightCommand::CollectWinnings, |session| {
            assert!(session.collect_winnings(), "racer 0 won on a roll of 0");
            Answer::Done
        });
        let before = session.clone();
        assert_eq!(
            apply(&mut session, FlightCommand::CollectWinnings),
            Answer::Refused,
            "none due"
        );
        let nothing = FlightCommand::Race {
            racer: 1,
            bet: Bet::Asked(0),
        };
        assert_eq!(apply(&mut session, nothing), Answer::Refused, "no stake");
        assert_eq!(session, before, "nothing changed");
    }

    #[test]
    fn a_jettison_and_the_intros_mark_are_done() {
        let mut session = docked(&port());
        runs(
            &mut session,
            FlightCommand::Jettison { all: true },
            |session| {
                session.jettison_cargo(true, &mut NeverFires);
                Answer::Done
            },
        );
        runs(
            &mut session,
            FlightCommand::Jettison { all: false },
            |session| {
                session.jettison_cargo(false, &mut NeverFires);
                Answer::Done
            },
        );
        runs(&mut session, FlightCommand::MarkIntroSeen, |session| {
            session.mark_intro_seen();
            Answer::Done
        });
        assert!(session.seen_intro());
    }

    #[test]
    fn an_assignment_answers_what_it_did_and_with_no_capture_is_refused() {
        let catalog = boardable();
        let mut session = captured(&catalog);
        runs(
            &mut session,
            FlightCommand::Assign(Assignment::Escort),
            |session| {
                Answer::Assigned(
                    session
                        .assign(Assignment::Escort, &mut NeverFires)
                        .expect("awaits"),
                )
            },
        );
        assert_eq!(session.pilot().escorts().len(), 1);
        let before = session.clone();
        assert_eq!(
            apply(&mut session, FlightCommand::Assign(Assignment::MyShip)),
            Answer::Refused,
            "nothing awaits"
        );
        assert_eq!(session, before);
    }
}
