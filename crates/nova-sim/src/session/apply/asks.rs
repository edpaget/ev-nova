//! [`Session::apply`]'s asks ([`FlightCommand::is_ask`]): the hover, a
//! description's load and the port's own description's, each matched to the session method that answers
//! it, as [`Session::apply_ask`] says. None changes anything a frame
//! shows, which the tests here pin, so the view keeps its frame across
//! them. The coming traffic and a plain text's fill, asked once by a
//! screen, are screen asks (`session/screen.rs`).

use super::ApplyCollaborators;
use crate::flight_frame::{Answer, FlightCommand};
use crate::session::Session;

impl Session {
    /// Answers the ask `command`, handed `collab`:
    ///
    /// - [`FlightCommand::Hover`] runs [`Session::hover`]:
    ///   [`Answer::Hovered`], `None` over nothing.
    /// - [`FlightCommand::LoadDescription`] runs
    ///   [`Session::load_description`] on `collab`'s chance:
    ///   [`Answer::Description`], empty for a missing `dësc`.
    /// - [`FlightCommand::LoadPortDescription`] runs
    ///   [`Session::port_description`] on `collab`'s catalog and chance:
    ///   [`Answer::Description`], empty for a missing `dësc`, or refused in
    ///   flight or with no catalog shared.
    #[allow(clippy::needless_pass_by_value)] // the commands come by value, as `Session::apply` takes them
    pub(super) fn apply_ask(
        &mut self,
        command: FlightCommand,
        collab: &mut ApplyCollaborators<'_>,
    ) -> Answer {
        match command {
            FlightCommand::Hover { at, drawn } => Answer::Hovered(self.hover(at, &drawn)),
            FlightCommand::LoadDescription { raw, movie } => Answer::Description(
                self.load_description(raw.as_deref(), movie.as_ref(), collab.chance),
            ),
            FlightCommand::LoadPortDescription => collab
                .catalog
                .and_then(|catalog| self.port_description(catalog, collab.chance))
                .map_or(Answer::Refused, Answer::Description),
            _ => Answer::Refused,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{DescMovie, DescText, StellarId};
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
        DrawnBox::at(crate::navigation::SpriteRect::centred(at, (60, 60)))
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
    fn the_ports_description_loads_as_the_session_loads_it_landed_and_is_refused_otherwise() {
        let mut catalog = catalog();
        catalog.descs = vec![(
            128,
            DescText {
                text: "{b5 \"set\" \"clear\"}, sir.".to_owned(),
                movie: DescMovie::from_desc("Port.mov", 0x0001),
            },
        )];
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(
            apply_with(&mut session, &catalog, FlightCommand::LoadPortDescription),
            Answer::Refused,
            "in flight"
        );
        assert_eq!(crate::testkit::land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(
            apply(&mut session, FlightCommand::LoadPortDescription),
            Answer::Refused,
            "no catalog shared"
        );
        let mut by_hand = session.clone();
        let expected = by_hand.port_description(&catalog, &mut NeverFires);
        let before: FlightFrame = filled(&session);
        assert_eq!(
            apply_with(&mut session, &catalog, FlightCommand::LoadPortDescription),
            Answer::Description("clear, sir.".to_owned())
        );
        assert_eq!(expected.as_deref(), Some("clear, sir."));
        assert_eq!(session, by_hand, "left as the session's own load leaves it");
        assert_eq!(
            filled(&session),
            before,
            "an ask changes nothing a frame shows"
        );
    }

    #[test]
    fn an_ask_changes_nothing_a_frame_shows() {
        let trafficked = crate::session::tests::trafficked(130, 2, 1);
        let mut session = Session::start(&trafficked).expect("starts");
        session.select_stellar(crate::session::StellarPick::Slot(0));
        let asks = [
            hover(OVER_128, &DrawnBoxes::default()),
            FlightCommand::LoadDescription {
                raw: Some("{b5 \"set\" \"clear\"}, sir.".to_owned()),
                movie: DescMovie::from_desc("Port.mov", 0x0001),
            },
            FlightCommand::LoadDescription {
                raw: None,
                movie: None,
            },
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
    fn only_the_hover_and_a_descriptions_load_are_asks() {
        for command in [
            hover(Vec2::ZERO, &DrawnBoxes::default()),
            FlightCommand::LoadDescription {
                raw: None,
                movie: None,
            },
            FlightCommand::LoadPortDescription,
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
