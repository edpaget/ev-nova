//! The sample with a sky block (`new_game::fixture::sky_sample`), loaded:
//! it loads with no problem, its start system, Luyten Crossing, reads the
//! block as written, and the other two systems have none.

use std::path::Path;

use nova_data::new_game::fixture::{SKY_SAMPLE_BLOCK, sky_sample};
use nova_data::new_game::{NewGame, load};
use nova_galaxy_format::id::{Color, Fraction, Id};
use nova_galaxy_format::sky::StarDensity;

fn id(text: &str) -> Id {
    Id::try_from(text.to_owned()).expect("an id")
}

fn loaded() -> NewGame {
    load(&sky_sample(), Path::new("sample"))
}

#[test]
fn the_sky_sample_loads_with_no_problem() {
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
fn the_start_system_reads_its_sky_as_written_and_the_others_have_none() {
    let game = loaded();
    let start = &game.galaxy.starts.values().next().expect("a start").data;
    assert_eq!(start.system.as_str(), "luyten-crossing");
    let sky = (game.galaxy.systems[&id("luyten-crossing")]
        .data
        .sky
        .as_ref())
    .expect("Luyten Crossing's sky");
    assert_eq!(sky.stars.map(StarDensity::get), Some(0.0));
    assert_eq!(sky.tint.as_ref().map(Color::rgb), Some([0xff, 0xb0, 0x70]));
    let backdrop = sky.backdrop.as_ref().expect("a backdrop");
    assert_eq!(backdrop.color.as_ref().map(Color::rgb), Some([0xff, 0, 0]));
    assert_eq!(backdrop.amount.map(Fraction::get), Some(1.0));
    for other in ["brightwater", "marrow"] {
        assert!(
            game.galaxy.systems[&id(other)].data.sky.is_none(),
            "{other}"
        );
    }
    assert!(SKY_SAMPLE_BLOCK.starts_with("sky:\n"), "{SKY_SAMPLE_BLOCK}");
}
