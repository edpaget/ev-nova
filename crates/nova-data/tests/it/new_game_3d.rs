//! The sample modelled for the 3D flight screen
//! (`new_game::fixture::modelled_sample_3d`), loaded: its two stations show
//! the `station-hub` model, drawn from above as their stand-ins, and
//! Luyten Crossing holds the generated gas giant Kestrel Reach. The load
//! keeps the galaxy, which the renderer's edge reads the models from, and
//! reports no problem: every record it adds is pinned.

use std::path::Path;

use nova_data::new_game::fixture::{KESTREL_REACH_ID, STATION_HUB_GLB, modelled_sample_3d};
use nova_data::new_game::{NewGame, load};
use nova_data::store::{StandIn, StandInOf};
use nova_data::wire::id::StellarId;
use nova_galaxy_format::id::Id;
use nova_galaxy_format::model::{PlanetType, Visual};
use nova_galaxy_format::source::Source;

fn id(text: &str) -> Id {
    Id::try_from(text.to_owned()).expect("an id")
}

fn loaded() -> NewGame {
    load(&modelled_sample_3d(), Path::new("sample"))
}

#[test]
fn the_modelled_sample_loads_with_no_problem() {
    let game = loaded();
    assert_eq!(
        game.problems
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        Vec::<String>::new()
    );
}

#[test]
fn the_galaxy_is_kept_with_kestrel_reach_s_look_in_luyten_crossing() {
    let game = loaded();
    let reach = &game.galaxy.stellars[&id("kestrel-reach")].data;
    assert_eq!(reach.system.as_str(), "luyten-crossing");
    let Some(Visual::Generated(look)) = &reach.visual else {
        panic!("a generated look: {:?}", reach.visual);
    };
    assert_eq!(look.kind, PlanetType::GasGiant);
    assert_eq!(look.seed, 4417);
    assert_eq!(look.palette.len(), 3);
    assert!(look.rings.is_some() && look.atmosphere.is_some());
    assert_eq!(game.ids.stellars[&id("kestrel-reach")], KESTREL_REACH_ID);
}

#[test]
fn both_stations_show_the_station_hub_drawn_from_above() {
    let game = loaded();
    assert!(
        modelled_sample_3d()
            .bytes(STATION_HUB_GLB)
            .expect("a memory read")
            .is_some(),
        "the hub's file is there"
    );
    for station in ["luyten-exchange", "marrow-yard"] {
        let stellar = &game.galaxy.stellars[&id(station)].data;
        assert!(
            matches!(&stellar.visual, Some(Visual::Asset(asset)) if asset.as_str() == "station-hub"),
            "{station}: {:?}",
            stellar.visual
        );
        let nova = StellarId(game.ids.stellars[&id(station)]);
        let Some(StandIn::Outline(outline)) = game.data.stand_in(StandInOf::Stellar(nova)) else {
            panic!("{station}: drawn as its model from above");
        };
        assert!(!outline.triangles.is_empty(), "{station}");
    }
}
