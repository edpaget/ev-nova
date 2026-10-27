//! The port's closes that reset the session, through the flight: the
//! outfitter's and the hire screen's ([`Session::close_outfitter`],
//! [`Session::close_hire`]), and the offer `dësc` reloaded as the mission
//! info over an offer closes ([`Session::reload_offer_desc`]).

use nova_sim::catalog::{DescMovie, MissionTexts, TextMovies};

use super::*;

#[test]
fn a_failed_flight_closes_nothing() {
    let mut view = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    view.close_outfitter();
    view.close_hire();
    view.reload_offer_desc(MissionId(142));
    assert!(view.session().is_err());
}

#[test]
fn reloading_an_offer_desc_loads_its_after_movie() {
    let movie = DescMovie::from_desc("A.mov", 0x0001);
    assert!(movie.is_some());
    let mut view = FlightView::new(FakeCatalog {
        mission_texts: vec![MissionTexts {
            offer: Some("Job 142.".to_owned()),
            movies: TextMovies {
                offer: movie.clone(),
                ..TextMovies::default()
            },
            ..MissionTexts::named(MissionId(142), "Job")
        }],
        ..catalog()
    });
    assert_eq!(view.loaded_after_movie(), None);
    view.reload_offer_desc(MissionId(142));
    assert_eq!(view.loaded_after_movie(), movie);
}
