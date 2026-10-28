//! The spaceport's, the comm's and the pilot's screens' answers to
//! [`Session::screen`]: each the session's own reads put together. They
//! coordinate the port, the chart, the mode, the space and the pilot, so
//! they are the session's.
//!
//! The shops' and the bar's asks ([`ScreenAsk::Shops`],
//! [`ScreenAsk::Outfitter`], [`ScreenAsk::HireList`]) draw, on the flight's
//! chance, only the day's rolls not drawn yet, and keep them in the port
//! (`Port::outfit_rolls`, `ship_rolls`, `hire_rolls`), as the original
//! keeps each day's (`_IncrementGameTime` @0xb7c4-0xb833 draws them all):
//! asked again, each answers from the rolls kept. The shops' shipyard is
//! built afresh, applying the redraws of the classes bought or declined
//! since its last build, at the points it was built before. A ship's name
//! prompt ([`ScreenAsk::ShipNaming`]) draws its default's digits afresh
//! at each ask and keeps nothing ([`ScreenAsk::draws_afresh`]).
//!
//! [`ScreenAsk::Shops`]: crate::flight_frame::ScreenAsk::Shops
//! [`ScreenAsk::Outfitter`]: crate::flight_frame::ScreenAsk::Outfitter
//! [`ScreenAsk::HireList`]: crate::flight_frame::ScreenAsk::HireList
//! [`ScreenAsk::ShipNaming`]: crate::flight_frame::ScreenAsk::ShipNaming
//! [`ScreenAsk::draws_afresh`]: crate::flight_frame::ScreenAsk::draws_afresh

use super::super::Session;
use crate::catalog::ShipId;
use crate::chance::Chance;
use crate::flight_frame::{BarNews, PortShops, ScreenAnswer};
use crate::hail::HailOptions;
use crate::{MissionId, SimCatalog};

impl Session {
    /// The spaceport's shops: the exchange ([`Session::market`]), the
    /// outfitter ([`Session::outfitter`]) and the shipyard
    /// ([`Session::shipyard`]), read in that order on `chance`, and the
    /// room to hire ([`Session::can_hire_escorts`]).
    pub(super) fn shops_screen(&mut self, chance: &mut dyn Chance) -> ScreenAnswer {
        let market = self.market();
        let outfitter = self.outfitter(chance);
        let shipyard = self.shipyard(chance);
        ScreenAnswer::Shops(Box::new(PortShops {
            market,
            outfitter,
            shipyard,
            can_hire_escorts: self.can_hire_escorts(),
        }))
    }

    /// The outfitter alone ([`Session::outfitter`]) on `chance`; nothing
    /// where there is none.
    pub(super) fn outfitter_screen(&mut self, chance: &mut dyn Chance) -> ScreenAnswer {
        self.outfitter(chance)
            .map_or(ScreenAnswer::Nothing, |outfitter| {
                ScreenAnswer::Outfitter(Box::new(outfitter))
            })
    }

    /// The ships for hire ([`Session::escorts_for_hire`]) on `chance`;
    /// nothing where there is no bar.
    pub(super) fn hire_list_screen(&mut self, chance: &mut dyn Chance) -> ScreenAnswer {
        self.escorts_for_hire(chance)
            .map_or(ScreenAnswer::Nothing, |list| {
                ScreenAnswer::HireList(Box::new(list))
            })
    }

    /// The name prompt for class `ship` ([`Session::ship_naming`]), worded
    /// from `catalog` and drawn on `chance`; nothing without a catalog.
    pub(super) fn ship_naming_screen(
        &mut self,
        ship: ShipId,
        catalog: Option<&dyn SimCatalog>,
        chance: &mut dyn Chance,
    ) -> ScreenAnswer {
        catalog.map_or(ScreenAnswer::Nothing, |catalog| {
            ScreenAnswer::ShipNaming(self.ship_naming(ship, catalog, chance))
        })
    }

    /// The bar visit's news as drawn ([`Session::news`]) over the
    /// picture of the stellar docked at ([`Session::news_picture`]);
    /// nothing unless both are some.
    pub(super) fn news_screen(&self) -> ScreenAnswer {
        match (self.news(), self.news_picture()) {
            (Some(news), Some(picture)) => ScreenAnswer::News(BarNews {
                news: news.clone(),
                picture,
            }),
            _ => ScreenAnswer::Nothing,
        }
    }

    /// What the mission info screen lists ([`Session::mission_info`]).
    pub(super) fn mission_info_screen(&self) -> ScreenAnswer {
        ScreenAnswer::MissionInfo(self.mission_info())
    }

    /// Whether `mission`'s offer dialog has a refuse button
    /// ([`Session::refusable_offer_dialog`]).
    pub(super) fn refusable_offer_screen(&self, mission: MissionId) -> ScreenAnswer {
        ScreenAnswer::RefusableOffer(self.refusable_offer_dialog(mission))
    }

    /// `raw` filled for `mission`'s offer ([`Session::offer_text`]);
    /// nothing when it is not offered.
    pub(super) fn offer_text_screen(&self, mission: MissionId, raw: &str) -> ScreenAnswer {
        self.offer_text(mission, raw)
            .map_or(ScreenAnswer::Nothing, ScreenAnswer::OfferText)
    }

    /// The comm dialog's contents ([`Session::hailing`]), worded from
    /// `catalog` and listing `options`; nothing with no hail under way or
    /// without a catalog.
    pub(super) fn hail_screen(
        &self,
        catalog: Option<&dyn SimCatalog>,
        options: &HailOptions,
    ) -> ScreenAnswer {
        catalog
            .and_then(|catalog| self.hailing(catalog, options))
            .map_or(ScreenAnswer::Nothing, |hail| {
                ScreenAnswer::Hail(Box::new(hail))
            })
    }

    /// What is on board the ship being boarded ([`Session::boarding`]);
    /// nothing with no boarding under way.
    pub(super) fn boarding_screen(&self) -> ScreenAnswer {
        self.boarding()
            .map_or(ScreenAnswer::Nothing, ScreenAnswer::Boarding)
    }

    /// The pilot panel ([`Session::pilot_panel`]) over `catalog`; nothing
    /// without one.
    pub(super) fn pilot_panel_screen(&self, catalog: Option<&dyn SimCatalog>) -> ScreenAnswer {
        catalog.map_or(ScreenAnswer::Nothing, |catalog| {
            ScreenAnswer::PilotPanel(Box::new(self.pilot_panel(catalog)))
        })
    }
}

#[cfg(test)]
pub(in crate::session) mod tests {
    use super::*;
    use crate::ShipRefusal;
    use crate::board::NovaBoarding;
    use crate::catalog::{MissionRecord, MissionTexts, SystemId};
    use crate::chance::NeverFires;
    use crate::flight_frame::{FlightFrame, ScreenAsk};
    use crate::landing::StellarFlags;
    use crate::legal::NovaLaw;
    use crate::mission::screens::BAR;
    use crate::session::apply::ApplyCollaborators;
    use crate::session::frame::tests::filled;
    use crate::session::hail::tests::{hail, hailable, targeting};
    use crate::session::mission_map::tests::{holding, landed};
    use crate::session::tests::{aboard, boardable, shipbuying};
    use crate::testkit::{FakePilotCatalog, mission, planet};

    /// A chance whose first draw goes one way and every later draw the
    /// other: the first percentage fires and no later one does; the
    /// first range draws its last outcome, every later one its first. So
    /// a roll drawn a second time decides otherwise than it did first,
    /// however many rolls an ask draws.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub(in crate::session) struct FirstFires {
        /// How many draws it has made.
        pub(in crate::session) draws: u32,
    }

    impl Chance for FirstFires {
        fn fires(&mut self, _percent: u8) -> bool {
            self.draws += 1;
            self.draws == 1
        }

        fn below(&mut self, n: u32) -> u32 {
            self.draws += 1;
            if self.draws == 1 { n - 1 } else { 0 }
        }
    }

    /// `session`'s answer to `ask`, handed `catalog` if any, on `chance`.
    pub(in crate::session) fn asked(
        session: &mut Session,
        catalog: Option<&FakePilotCatalog>,
        chance: &mut dyn Chance,
        ask: ScreenAsk,
    ) -> ScreenAnswer {
        session.screen(
            ask,
            &mut ApplyCollaborators {
                catalog: catalog.map(|catalog| catalog as &dyn SimCatalog),
                chance,
                law: &NovaLaw::default(),
                boarding_rule: &NovaBoarding::default(),
                hail_options: &HailOptions::default(),
            },
        )
    }

    /// Ship 129, for sale `BuyRandom` 50 and for hire `HireRandom` 50 in
    /// [`port`].
    const ROLLED: ShipId = ShipId(129);
    /// Ship 130, ship 129's like, always for sale in [`port`].
    const FOR_SALE: ShipId = ShipId(130);

    /// [`shipbuying`]'s planet 128, an exchange, an outfitter and a
    /// shipyard, a bar too: its speed booster on sale `BuyRandom` 50, ship
    /// 129 on `BuyRandom` 50 and for hire `HireRandom` 50, and ship 130,
    /// 129's like, always for sale; system 132 with
    /// stellar 150; mission 300 at the computer, to 140 and back at 150,
    /// and mission 141 at the bar, offering "Come along.".
    pub(in crate::session) fn port() -> FakePilotCatalog {
        let mut catalog = shipbuying();
        let like = |ships: &[(ShipId, _)]| ships.iter().find(|(id, _)| *id == ROLLED).cloned();
        let (_, fields) = like(&catalog.ships).expect("ship 129");
        catalog.ships.push((FOR_SALE, fields));
        let record = catalog
            .ship_records
            .iter()
            .find(|record| record.id == ROLLED)
            .cloned()
            .expect("ship 129's record");
        catalog.ship_records.push(crate::catalog::ShipRecord {
            id: FOR_SALE,
            ..record
        });
        catalog.sites[0].1[0].flags |= StellarFlags::BAR;
        catalog.outfits[0].buy_random = 50;
        for record in &mut catalog.ship_records {
            if record.id == ROLLED {
                record.buy_random = 50;
                record.hire_random = 50;
            }
        }
        catalog.systems.push(SystemId(132));
        catalog
            .sites
            .push((SystemId(132), vec![planet(150, 0.0, 0.0)]));
        catalog.missions = vec![
            MissionRecord {
                travel_stel: 140,
                return_stel: 150,
                ..mission(300)
            },
            MissionRecord {
                avail_loc: BAR,
                travel_stel: 140,
                ..mission(141)
            },
        ];
        catalog.mission_texts = vec![MissionTexts {
            offer: Some("Come along.".to_owned()),
            ..MissionTexts::named(MissionId(141), "Mission 141")
        }];
        catalog
    }

    /// [`port`]'s session landed on 128, its offers rolled, holding
    /// mission 300, with none of the day's rolls drawn yet.
    pub(in crate::session) fn docked(catalog: &FakePilotCatalog) -> Session {
        let mut session = landed(catalog);
        holding(&mut session, 300, Some(140), Some(150));
        session
    }

    #[test]
    fn the_shops_are_the_sessions_exchange_outfitter_shipyard_and_room_to_hire() {
        let catalog = port();
        let mut session = docked(&catalog);
        let mut by_hand = session.clone();
        let mut chance = FirstFires::default();
        let expected = PortShops {
            market: by_hand.market(),
            outfitter: by_hand.outfitter(&mut chance),
            shipyard: by_hand.shipyard(&mut chance),
            can_hire_escorts: by_hand.can_hire_escorts(),
        };
        assert!(chance.draws > 0, "precondition: the day's rolls drawn");
        assert!(
            expected.market.is_some()
                && expected.outfitter.is_some()
                && expected.shipyard.is_some(),
            "precondition: every shop"
        );
        assert!(expected.can_hire_escorts, "precondition: room to hire");
        let mut own = FirstFires::default();
        assert_eq!(
            asked(&mut session, None, &mut own, ScreenAsk::Shops),
            ScreenAnswer::Shops(Box::new(expected))
        );
        assert_eq!(own, chance, "the same draws");
        assert_eq!(session, by_hand, "the same rolls kept");
    }

    #[test]
    fn in_flight_the_shops_list_nothing() {
        let catalog = port();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(
            asked(&mut session, None, &mut NeverFires, ScreenAsk::Shops),
            ScreenAnswer::Shops(Box::new(PortShops {
                can_hire_escorts: session.can_hire_escorts(),
                ..PortShops::default()
            }))
        );
    }

    #[test]
    fn the_outfitter_alone_is_the_sessions_listing() {
        let catalog = port();
        let mut session = docked(&catalog);
        let mut by_hand = session.clone();
        let mut chance = FirstFires::default();
        let expected = by_hand.outfitter(&mut chance).expect("an outfitter");
        let mut own = FirstFires::default();
        assert_eq!(
            asked(&mut session, None, &mut own, ScreenAsk::Outfitter),
            ScreenAnswer::Outfitter(Box::new(expected))
        );
        assert_eq!((own, &session), (chance, &by_hand));
        let mut flying = Session::start(&catalog).expect("starts");
        assert_eq!(
            asked(&mut flying, None, &mut NeverFires, ScreenAsk::Outfitter),
            ScreenAnswer::Nothing,
            "none in flight"
        );
    }

    #[test]
    fn the_hire_list_is_the_bars_and_nothing_without_one() {
        let catalog = port();
        let mut session = docked(&catalog);
        let mut by_hand = session.clone();
        let mut chance = FirstFires::default();
        let expected = by_hand.escorts_for_hire(&mut chance).expect("a bar");
        assert!(chance.draws > 0, "precondition: the hire roll drawn");
        let mut own = FirstFires::default();
        assert_eq!(
            asked(&mut session, None, &mut own, ScreenAsk::HireList),
            ScreenAnswer::HireList(Box::new(expected))
        );
        assert_eq!((own, &session), (chance, &by_hand));
        let mut flying = Session::start(&catalog).expect("starts");
        assert_eq!(
            asked(&mut flying, None, &mut NeverFires, ScreenAsk::HireList),
            ScreenAnswer::Nothing,
            "no bar in flight"
        );
    }

    #[test]
    fn a_ship_naming_is_the_sessions_prompt_or_refusal() {
        let catalog = port();
        let mut session = docked(&catalog);
        for ship in [FOR_SALE, ShipId(128)] {
            let mut by_hand = session.clone();
            let mut chance = FirstFires::default();
            let expected = by_hand.ship_naming(ship, &catalog, &mut chance);
            let mut own = FirstFires::default();
            assert_eq!(
                asked(
                    &mut session,
                    Some(&catalog),
                    &mut own,
                    ScreenAsk::ShipNaming(ship)
                ),
                ScreenAnswer::ShipNaming(expected),
                "{ship:?}"
            );
            assert_eq!((own, &session), (chance, &by_hand), "{ship:?}");
        }
        assert!(
            matches!(
                asked(
                    &mut session,
                    Some(&catalog),
                    &mut NeverFires,
                    ScreenAsk::ShipNaming(FOR_SALE)
                ),
                ScreenAnswer::ShipNaming(Ok(_))
            ),
            "precondition: a prompt"
        );
        assert!(
            matches!(
                asked(
                    &mut session,
                    Some(&catalog),
                    &mut NeverFires,
                    ScreenAsk::ShipNaming(ShipId(128))
                ),
                ScreenAnswer::ShipNaming(Err(ShipRefusal::AlreadyFlown))
            ),
            "precondition: a refusal, of the class flown"
        );
    }

    #[test]
    fn the_news_is_the_visits_drawn_news_over_the_docked_stellars_picture() {
        let catalog = port();
        let mut session = docked(&catalog);
        assert_eq!(
            asked(&mut session, None, &mut NeverFires, ScreenAsk::News),
            ScreenAnswer::Nothing,
            "before the bar opens"
        );
        assert!(session.open_bar(&mut NeverFires), "a bar");
        let expected = BarNews {
            news: session.news().expect("drawn").clone(),
            picture: session.news_picture().expect("a bar's picture"),
        };
        assert_eq!(
            asked(&mut session, None, &mut NeverFires, ScreenAsk::News),
            ScreenAnswer::News(expected)
        );
    }

    #[test]
    fn the_mission_info_is_the_sessions() {
        let catalog = port();
        let mut session = docked(&catalog);
        let expected = session.mission_info();
        assert!(!expected.rows.is_empty(), "precondition: a mission held");
        assert_eq!(
            asked(&mut session, None, &mut NeverFires, ScreenAsk::MissionInfo),
            ScreenAnswer::MissionInfo(expected)
        );
    }

    #[test]
    fn the_offer_dialogs_refuse_button_is_the_sessions() {
        let catalog = port();
        let mut session = docked(&catalog);
        for (mission, refusable) in [(MissionId(141), true), (MissionId(999), false)] {
            assert_eq!(
                session.refusable_offer_dialog(mission),
                refusable,
                "precondition: {mission:?}"
            );
            assert_eq!(
                asked(
                    &mut session,
                    None,
                    &mut NeverFires,
                    ScreenAsk::RefusableOffer(mission)
                ),
                ScreenAnswer::RefusableOffer(refusable),
                "{mission:?}"
            );
        }
    }

    #[test]
    fn an_offer_text_is_filled_as_the_session_fills_it_and_nothing_once_not_offered() {
        let catalog = port();
        let mut session = docked(&catalog);
        let raw = "Run <PAY> for <PN>.";
        let expected = session
            .offer_text(MissionId(300), raw)
            .expect("precondition: offered");
        assert_ne!(expected, raw, "precondition: a tag filled");
        let offer = |mission| ScreenAsk::OfferText {
            mission,
            raw: raw.to_owned(),
        };
        assert_eq!(
            asked(&mut session, None, &mut NeverFires, offer(MissionId(300))),
            ScreenAnswer::OfferText(expected)
        );
        assert_eq!(
            asked(&mut session, None, &mut NeverFires, offer(MissionId(999))),
            ScreenAnswer::Nothing,
            "not offered"
        );
    }

    #[test]
    fn the_hail_is_the_sessions_and_nothing_with_none_under_way() {
        let catalog = hailable();
        let mut session = targeting(&catalog);
        assert_eq!(
            asked(
                &mut session,
                Some(&catalog),
                &mut NeverFires,
                ScreenAsk::Hail
            ),
            ScreenAnswer::Nothing,
            "no hail under way"
        );
        hail(&mut session, &catalog);
        let expected = session
            .hailing(&catalog, &HailOptions::default())
            .expect("hailing");
        assert_eq!(
            asked(
                &mut session,
                Some(&catalog),
                &mut NeverFires,
                ScreenAsk::Hail
            ),
            ScreenAnswer::Hail(Box::new(expected))
        );
        assert_eq!(
            asked(&mut session, None, &mut NeverFires, ScreenAsk::Hail),
            ScreenAnswer::Nothing,
            "no catalog"
        );
    }

    #[test]
    fn the_boarding_is_the_sessions_and_nothing_with_none_under_way() {
        let catalog = boardable();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(
            asked(&mut session, None, &mut NeverFires, ScreenAsk::Boarding),
            ScreenAnswer::Nothing,
            "no boarding under way"
        );
        let mut session = aboard(&catalog);
        let expected = session.boarding().expect("boarding");
        assert_eq!(
            asked(&mut session, None, &mut NeverFires, ScreenAsk::Boarding),
            ScreenAnswer::Boarding(expected)
        );
    }

    #[test]
    fn the_pilot_panel_is_the_sessions_and_nothing_without_a_catalog() {
        let catalog = port();
        let mut session = docked(&catalog);
        let expected = session.pilot_panel(&catalog);
        assert_eq!(
            asked(
                &mut session,
                Some(&catalog),
                &mut NeverFires,
                ScreenAsk::PilotPanel
            ),
            ScreenAnswer::PilotPanel(Box::new(expected))
        );
        assert_eq!(
            asked(&mut session, None, &mut NeverFires, ScreenAsk::PilotPanel),
            ScreenAnswer::Nothing
        );
    }

    /// Every port, comm and pilot ask, each with the scene where it
    /// answers something: the port's with the day's rolls not drawn yet,
    /// a class declined (its redraw pending) and the bar's news drawn; the
    /// hail's and the pilot panel's flying with a hail under way; the
    /// boarding's with one under way.
    fn scenes() -> Vec<(FakePilotCatalog, Session, Vec<ScreenAsk>)> {
        let port = port();
        let mut docked = docked(&port);
        assert!(docked.open_bar(&mut NeverFires), "a bar");
        docked.decline_ship(ROLLED);
        let hailable = hailable();
        let mut hailing = targeting(&hailable);
        hail(&mut hailing, &hailable);
        let boardable = boardable();
        let boarding = aboard(&boardable);
        vec![
            (
                port,
                docked,
                vec![
                    ScreenAsk::Shops,
                    ScreenAsk::Outfitter,
                    ScreenAsk::HireList,
                    ScreenAsk::ShipNaming(FOR_SALE),
                    ScreenAsk::News,
                    ScreenAsk::MissionInfo,
                    ScreenAsk::RefusableOffer(MissionId(141)),
                    ScreenAsk::OfferText {
                        mission: MissionId(300),
                        raw: "Run <PAY>.".to_owned(),
                    },
                    ScreenAsk::PilotPanel,
                ],
            ),
            (
                hailable,
                hailing,
                vec![ScreenAsk::Hail, ScreenAsk::PilotPanel],
            ),
            (boardable, boarding, vec![ScreenAsk::Boarding]),
        ]
    }

    #[test]
    fn a_port_screen_ask_changes_nothing_a_frame_shows() {
        for (catalog, mut session, asks) in scenes() {
            let rolls = asks.contains(&ScreenAsk::Shops);
            let mut chance = FirstFires::default();
            for ask in asks {
                let before: FlightFrame = filled(&session);
                let sent = format!("{ask:?}");
                let answer = asked(&mut session, Some(&catalog), &mut chance, ask.clone());
                assert!(answer.answers(&ask), "{sent}: {answer:?}");
                assert_ne!(answer, ScreenAnswer::Nothing, "{sent}");
                assert_eq!(filled(&session), before, "{sent}");
            }
            assert!(
                !rolls || chance.draws > 0,
                "the port's asks drew the day's rolls"
            );
        }
    }

    #[test]
    fn a_rolling_ask_asked_again_answers_the_same() {
        let catalog = port();
        for ask in [ScreenAsk::Shops, ScreenAsk::Outfitter, ScreenAsk::HireList] {
            let mut session = docked(&catalog);
            let mut chance = FirstFires::default();
            let first = asked(&mut session, None, &mut chance, ask.clone());
            assert!(chance.draws > 0, "precondition: {ask:?} draws");
            assert_eq!(
                asked(&mut session, None, &mut chance, ask.clone()),
                first,
                "{ask:?}"
            );
        }
        let mut session = docked(&catalog);
        let mut chance = FirstFires::default();
        let _ = asked(&mut session, None, &mut chance, ScreenAsk::Shops);
        session.decline_ship(ROLLED);
        let drawn = chance.draws;
        let first = asked(&mut session, None, &mut chance, ScreenAsk::Shops);
        assert!(chance.draws > drawn, "precondition: the decline's redraw");
        assert_eq!(
            asked(&mut session, None, &mut chance, ScreenAsk::Shops),
            first,
            "the redraw applied once"
        );
    }

    #[test]
    fn a_ship_namings_draw_keeps_nothing_the_shops_read() {
        let catalog = port();
        let mut session = docked(&catalog);
        let mut chance = FirstFires::default();
        let before = asked(&mut session, None, &mut chance, ScreenAsk::Shops);
        let drawn = chance.draws;
        let _ = asked(
            &mut session,
            Some(&catalog),
            &mut chance,
            ScreenAsk::ShipNaming(FOR_SALE),
        );
        assert!(
            chance.draws > drawn,
            "precondition: the name's digits drawn"
        );
        assert_eq!(
            asked(&mut session, None, &mut chance, ScreenAsk::Shops),
            before
        );
    }
}
