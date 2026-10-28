//! [`Session::apply`]'s asks ([`FlightCommand::is_ask`]): the hover, the
//! coming traffic, a description's load and a plain text's fill, each matched to the session
//! method that answers it, as [`Session::apply_ask`] says. None changes
//! anything a frame shows, which the tests here pin, so the view keeps
//! its frame across them.

use super::ApplyCollaborators;
use crate::flight_frame::{Answer, FlightCommand};
use crate::session::Session;

impl Session {
    /// Answers the ask `command`, handed `collab`:
    ///
    /// - [`FlightCommand::Hover`] runs [`Session::hover`]:
    ///   [`Answer::Hovered`], `None` over nothing.
    /// - [`FlightCommand::ComingTraffic`] runs
    ///   [`Session::coming_traffic_ships`] over the catalog:
    ///   [`Answer::ComingTraffic`]; refused without a catalog, as the
    ///   spawn table it resolves while traffic is due reads one.
    /// - [`FlightCommand::LoadDescription`] runs
    ///   [`Session::load_description`] on `collab`'s chance:
    ///   [`Answer::Description`], empty for a missing `dësc`.
    /// - [`FlightCommand::PlainText`] runs [`Session::plain_text`]:
    ///   [`Answer::PlainText`].
    #[allow(clippy::needless_pass_by_value)] // the commands come by value, as `Session::apply` takes them
    pub(super) fn apply_ask(
        &mut self,
        command: FlightCommand,
        collab: &mut ApplyCollaborators<'_>,
    ) -> Answer {
        match command {
            FlightCommand::Hover { at, drawn } => Answer::Hovered(self.hover(at, &drawn)),
            FlightCommand::ComingTraffic => match collab.catalog {
                Some(catalog) => Answer::ComingTraffic(self.coming_traffic_ships(catalog)),
                None => Answer::Refused,
            },
            FlightCommand::LoadDescription { raw, movie } => Answer::Description(
                self.load_description(raw.as_deref(), movie.as_ref(), collab.chance),
            ),
            FlightCommand::PlainText(text) => Answer::PlainText(self.plain_text(&text)),
            _ => Answer::Refused,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{DescMovie, StellarId};
    use crate::chance::NeverFires;
    use crate::flight_frame::{DrawnBoxes, FlightFrame};
    use crate::geometry::Vec2;
    use crate::hail::HailOptions;
    use crate::hover::{DrawnBox, Hover};
    use crate::session::apply::tests::apply;
    use crate::session::frame::tests::filled;
    use crate::stats::ShipStats;
    use crate::testkit::{FakePilotCatalog, catalog, npc};

    /// Runs `command` on `session`, handed `catalog`, on a chance that
    /// never fires.
    fn apply_with(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        command: FlightCommand,
    ) -> Answer {
        session.apply(
            command,
            &mut ApplyCollaborators {
                catalog: Some(catalog),
                chance: &mut NeverFires,
                law: &crate::legal::NovaLaw::default(),
                boarding_rule: &crate::board::NovaBoarding::default(),
                hail_options: &HailOptions::default(),
            },
        )
    }

    /// Planet 128, at (30, -40), 100 square.
    const OVER_128: Vec2 = Vec2::new(30.0, -40.0);

    /// A 60-pixel square at `at`.
    fn square(at: Vec2) -> DrawnBox {
        DrawnBox {
            centre: at,
            width: 60,
            height: 60,
        }
    }

    /// The hover at `at` with `drawn`, as the command carries it.
    fn hover(at: Vec2, drawn: &DrawnBoxes) -> FlightCommand {
        FlightCommand::Hover {
            at,
            drawn: drawn.clone(),
        }
    }

    #[test]
    fn the_hover_answers_what_the_session_hovers_a_ship_a_stellar_or_nothing() {
        let mut session = Session::start(&catalog()).expect("starts");
        let id = session.space.traffic.add_npc(npc(0, ShipStats::default()));
        let over_the_ship = DrawnBoxes {
            player: None,
            npcs: vec![(id, square(OVER_128))],
        };
        let none = DrawnBoxes::default();
        for (at, drawn, expected) in [
            (OVER_128, &over_the_ship, Some(Hover::Ship(id))),
            (OVER_128, &none, Some(Hover::Stellar(StellarId(128)))),
            (Vec2::new(500.0, 500.0), &none, None),
        ] {
            assert_eq!(session.hover(at, drawn), expected, "the session's");
            assert_eq!(
                apply(&mut session, hover(at, drawn)),
                Answer::Hovered(expected)
            );
        }
    }

    #[test]
    fn the_coming_traffic_answers_the_ships_due_and_then_those_the_system_was_filled_with() {
        let trafficked = crate::session::tests::trafficked(130, 2, 1);
        let mut session = Session::start(&trafficked).expect("starts");
        let due = session.coming_traffic_ships(&trafficked);
        assert!(!due.is_empty(), "precondition: the table's ships");
        assert_ne!(due, session.traffic_ships(), "precondition: not yet filled");
        assert_eq!(
            apply_with(&mut session, &trafficked, FlightCommand::ComingTraffic),
            Answer::ComingTraffic(due.clone())
        );
        session.populate(&trafficked, &mut NeverFires);
        assert_eq!(
            apply_with(&mut session, &catalog(), FlightCommand::ComingTraffic),
            Answer::ComingTraffic(session.traffic_ships()),
            "once filled, the system's own"
        );
    }

    #[test]
    fn the_coming_traffic_is_refused_without_a_catalog() {
        let mut session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            apply(&mut session, FlightCommand::ComingTraffic),
            Answer::Refused
        );
    }

    #[test]
    fn a_description_loads_as_the_session_loads_it_and_a_missing_one_is_empty() {
        let mut session = Session::start(&catalog()).expect("starts");
        let raw = "{b5 \"set\" \"clear\"}, sir.";
        let movie = DescMovie::from_desc("Port.mov", 0x0001);
        let mut by_hand = session.clone();
        let expected = by_hand.load_description(Some(raw), movie.as_ref(), &mut NeverFires);
        assert_eq!(
            apply(
                &mut session,
                FlightCommand::LoadDescription {
                    raw: Some(raw.to_owned()),
                    movie: movie.clone(),
                }
            ),
            Answer::Description(expected)
        );
        assert_eq!(session, by_hand, "left as the session's own load leaves it");
        assert_eq!(
            session.loaded_after_movie(),
            movie,
            "its movie the last loaded"
        );
        assert_eq!(
            apply(
                &mut session,
                FlightCommand::LoadDescription {
                    raw: None,
                    movie: None,
                }
            ),
            Answer::Description(String::new())
        );
    }

    #[test]
    fn a_plain_text_is_filled_as_the_session_fills_it_for_the_pilot() {
        let mut session = Session::start(&catalog()).expect("starts");
        session.pilot.ship_name = Some("Kestrel".to_owned());
        let raw = "Hello, <PSN>.";
        let expected = session.plain_text(raw);
        assert_eq!(expected, "Hello, Kestrel.", "precondition: a tag filled");
        assert_eq!(
            apply(&mut session, FlightCommand::PlainText(raw.to_owned())),
            Answer::PlainText(expected)
        );
    }

    #[test]
    fn an_ask_changes_nothing_a_frame_shows() {
        let trafficked = crate::session::tests::trafficked(130, 2, 1);
        let mut session = Session::start(&trafficked).expect("starts");
        session.select_stellar(crate::session::StellarPick::Slot(0));
        let asks = [
            hover(OVER_128, &DrawnBoxes::default()),
            FlightCommand::ComingTraffic,
            FlightCommand::LoadDescription {
                raw: Some("{b5 \"set\" \"clear\"}, sir.".to_owned()),
                movie: DescMovie::from_desc("Port.mov", 0x0001),
            },
            FlightCommand::LoadDescription {
                raw: None,
                movie: None,
            },
            FlightCommand::PlainText("Hello, <PSN>.".to_owned()),
        ];
        for ask in asks {
            let before: FlightFrame = filled(&session);
            let sent = format!("{ask:?}");
            assert!(ask.is_ask(), "{sent}");
            let answer = apply_with(&mut session, &trafficked, ask);
            assert!(!answer.is_refused(), "{sent}");
            assert_eq!(filled(&session), before, "{sent}");
        }
    }

    #[test]
    fn only_the_hover_the_coming_traffic_a_descriptions_load_and_a_plain_text_are_asks() {
        for command in [
            hover(Vec2::ZERO, &DrawnBoxes::default()),
            FlightCommand::ComingTraffic,
            FlightCommand::LoadDescription {
                raw: None,
                movie: None,
            },
            FlightCommand::PlainText(String::new()),
        ] {
            assert!(command.is_ask(), "{command:?}");
            assert_eq!(command.gate(), None, "{command:?}: no key gates it");
        }
        for command in [
            FlightCommand::ClearRoute,
            FlightCommand::SetWindowWidth(1024),
            FlightCommand::ClickInSpace {
                at: Vec2::ZERO,
                drawn: DrawnBoxes::default(),
            },
            FlightCommand::ReloadOfferDesc(crate::MissionId(128)),
        ] {
            assert!(!command.is_ask(), "{command:?}");
        }
    }
}
