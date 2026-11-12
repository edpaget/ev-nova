//! The new game's sky looks at the program's edge ([`sky_looks`]): each
//! system's own look from its data's `sky` block
//! (`nova_galaxy_format::sky::Sky`), keyed by its sky's seed, for the app
//! to hand the renderer ([`Renderer::set_sky_looks`](crate::Renderer::set_sky_looks))
//! with the world models ([`WorldAssets::skies`](crate::WorldAssets::skies)).
//!
//! It is pure and needs no GPU. A system with a `sky` block and a Nova ID
//! gives one entry, keyed by the flight's seed rule
//! (`nova_view::flight::sky::sky_seed`), each colour's bytes taken to 0
//! to 1 and each field left out left `None`, so the renderer's defaults
//! fill it ([`crate::sky::SkyLook`]). A system with none has no entry and
//! draws the default look. It reads no hazard: the `nebula` hazard is
//! murk, which the pose's level carries, and never sets a backdrop. Stock
//! data is never a new game, so it has none of this.

use nova_data::new_game::NewGame;
use nova_data::wire::id::SystemId;
use nova_galaxy_format::id::Color;
use nova_view::flight::sky::sky_seed;

use crate::sky::{BackdropLook, SkyLook, SkyLooks};

/// The looks of `game`'s systems that set one (see the module docs).
#[must_use]
pub fn sky_looks(game: &NewGame) -> SkyLooks {
    let mut looks = SkyLooks::new();
    for (id, &nova) in &game.ids.systems {
        let Some(sky) = game
            .galaxy
            .systems
            .get(id)
            .and_then(|system| system.data.sky.as_ref())
        else {
            continue;
        };
        let backdrop = sky.backdrop.as_ref();
        #[allow(clippy::cast_possible_truncation, reason = "0 to 2, and 0 to 1")]
        let look = SkyLook {
            stars: sky.stars.map_or(1.0, |stars| stars.get() as f32),
            tint: sky.tint.as_ref().map(rgb),
            backdrop: BackdropLook {
                color: backdrop.and_then(|b| b.color.as_ref()).map(rgb),
                amount: backdrop
                    .and_then(|b| b.amount)
                    .map(|amount| amount.get() as f32),
            },
        };
        looks.insert(sky_seed(SystemId(nova)), look);
    }
    looks
}

/// `color`'s red, green and blue, 0 to 1.
fn rgb(color: &Color) -> [f32; 3] {
    color.rgb().map(|c| f32::from(c) / 255.0)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::path::Path;

    use nova_data::new_game::fixture::{modelled_sample, sky_sample};
    use nova_data::new_game::load;
    use nova_galaxy_format::source::MemSource;

    use super::*;

    fn loaded(source: &MemSource) -> NewGame {
        let game = load(source, Path::new("sample"));
        assert_eq!(
            game.problems
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            Vec::<String>::new()
        );
        game
    }

    /// The seed the flight gives system `name`'s sky.
    fn seed_of(game: &NewGame, name: &str) -> u64 {
        let id = game
            .ids
            .systems
            .iter()
            .find(|(id, _)| id.as_str() == name)
            .map(|(_, &nova)| nova)
            .expect("a pinned system");
        sky_seed(SystemId(id))
    }

    #[test]
    fn a_sample_with_no_sky_block_gives_no_looks() {
        let game = loaded(&modelled_sample());
        assert!(sky_looks(&game).is_empty());
        // Marrow, in its nebula, draws the default too: no hazard sets a
        // look.
        assert_eq!(
            sky_looks(&game).look(seed_of(&game, "marrow")),
            SkyLook::default()
        );
    }

    #[test]
    fn the_sky_sample_gives_its_start_systems_look_by_its_seed() {
        let game = loaded(&sky_sample());
        let looks = sky_looks(&game);
        assert_eq!(looks.len(), 1);
        assert_eq!(
            looks.look(seed_of(&game, "luyten-crossing")),
            SkyLook {
                stars: 0.0,
                tint: Some([1.0, 176.0 / 255.0, 112.0 / 255.0]),
                backdrop: BackdropLook {
                    color: Some([1.0, 0.0, 0.0]),
                    amount: Some(1.0),
                },
            }
        );
        for other in ["brightwater", "marrow"] {
            assert_eq!(
                looks.look(seed_of(&game, other)),
                SkyLook::default(),
                "{other}"
            );
        }
    }

    /// The sample with `block` on Luyten Crossing.
    fn with_sky(block: &str) -> MemSource {
        let file = "systems/luyten-crossing.md";
        let text =
            std::fs::read_to_string(nova_data::new_game::fixture::sample_folder().join(file))
                .expect("a sample file");
        let (front, body) = text
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---\n"))
            .expect("front matter");
        modelled_sample().with(file, &format!("---\n{front}\n{block}---\n{body}"))
    }

    #[test]
    fn a_partial_block_leaves_the_rest_to_the_defaults() {
        let game = loaded(&with_sky("sky: {stars: 1.5, backdrop: {amount: 0.25}}\n"));
        assert_eq!(
            sky_looks(&game).look(seed_of(&game, "luyten-crossing")),
            SkyLook {
                stars: 1.5,
                tint: None,
                backdrop: BackdropLook {
                    color: None,
                    amount: Some(0.25),
                },
            }
        );
        let game = loaded(&with_sky("sky: {tint: \"#000000\"}\n"));
        assert_eq!(
            sky_looks(&game).look(seed_of(&game, "luyten-crossing")),
            SkyLook {
                tint: Some([0.0; 3]),
                ..SkyLook::default()
            }
        );
        // An empty block is an entry of defaults.
        let game = loaded(&with_sky("sky: {}\n"));
        let looks = sky_looks(&game);
        assert_eq!(looks.len(), 1);
        assert_eq!(
            looks.look(seed_of(&game, "luyten-crossing")),
            SkyLook::default()
        );
    }

    #[test]
    fn a_colours_bytes_are_taken_to_0_to_1() {
        let color = |text: &str| Color::try_from(text.to_owned()).expect("a colour");
        assert_eq!(rgb(&color("#ff0000")), [1.0, 0.0, 0.0]);
        assert_eq!(rgb(&color("#336699")), [0.2, 0.4, 0.6]);
    }

    #[test]
    fn the_world_assets_carry_the_looks() {
        let source = sky_sample();
        let game = loaded(&source);
        assert_eq!(crate::world_assets(&game, &source).skies, sky_looks(&game));
        assert_eq!(crate::world_assets(&game, &source).skies.len(), 1);
    }
}
