//! The port's closes that reset the session, through the flight: the
//! outfitter's and the hire screen's ([`Session::close_outfitter`],
//! [`Session::close_hire`]), and the offer `dësc` reloaded as the mission
//! info over an offer closes ([`Session::reload_offer_desc`]).

use nova_sim::catalog::{DescMovie, MissionTexts, TextMovies};
use nova_sim::{RuleSource, Rulebook};

use super::*;

/// The cargo pod: 10 tons more cargo space (`ModType` 2) for 100
/// credits, of no mass.
const POD: OutfitId = OutfitId(210);

/// [`trading`]'s Earth with an outfitter selling the [`POD`], the
/// `chär` holding 5000 credits.
fn pod_port() -> FakeCatalog {
    let trade = trading();
    let flags = trade.sites[0].flags | StellarFlags::OUTFITTER;
    let pod = OutfitRecord {
        id: POD,
        name: "Cargo Pod".to_owned(),
        short_name: "Cargo Pod".to_owned(),
        mass: 0,
        cost: 100,
        mods: [(2, 10), (0, 0), (0, 0), (0, 0)],
        lc_name: "cargo pod".to_owned(),
        lc_plural: "cargo pods".to_owned(),
        ..outfitting().outfits[0].clone()
    };
    FakeCatalog {
        character: Ok(CharacterStart {
            cash: 5000,
            ..trade.character.clone().expect("a chär")
        }),
        sites: vec![site(128, (0.0, 0.0), flags)],
        outfits: vec![pod],
        ..trade
    }
}

/// The food aboard `view`.
fn food(view: &View) -> u32 {
    view.pilot()
        .expect("flying")
        .cargo()
        .find(|&(good, _)| good == Good::Commodity(0))
        .map_or(0, |(_, tons)| tons)
}

/// Landed at [`pod_port`]'s Earth by the other reading of
/// [`RuleKey::OutfitSaleCargo`]: the pod bought (20 tons), 20 tons of
/// food bought, and the pod sold, which keeps the 20 tons in the 10.
fn past_the_holds() -> View {
    let rules = Rulebook::default().with_override(RuleKey::OutfitSaleCargo, RuleSource::Bible);
    let mut view = FlightView::new(pod_port()).with_rules(rules);
    land_now(&mut view);
    view.settle_script();
    let buy = OutfitOrder {
        outfit: POD,
        direction: Direction::Buy,
    };
    assert_eq!(view.outfit(buy), Ok(()));
    while view.trade(BUY_FOOD).is_ok() {}
    assert_eq!(food(&view), 20, "the pod's holds full");
    let sell = OutfitOrder {
        outfit: POD,
        direction: Direction::Sell,
    };
    assert_eq!(view.outfit(sell), Ok(()));
    assert_eq!(food(&view), 20, "kept past the holds");
    view
}

#[test]
fn the_outfitter_closing_trims_the_cargo_to_the_holds() {
    let mut view = past_the_holds();
    view.close_outfitter();
    assert_eq!(food(&view), 10);
}

#[test]
fn the_hire_screen_closing_trims_the_cargo_to_the_holds() {
    let mut view = past_the_holds();
    view.close_hire();
    assert_eq!(food(&view), 10);
}

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
