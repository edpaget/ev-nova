//! The new game's sky decorations at the program's edge
//! ([`sky_decorations`]): each system's list from its data's `sky` block
//! (`nova_galaxy_format::sky::decoration::Decoration`), with its images
//! read, keyed by its sky's seed, for [`world_assets`](crate::world_assets)
//! to put beside the systems' looks
//! ([`SkyLooks::set_decorations`](crate::sky::SkyLooks::set_decorations)).
//!
//! It needs no GPU. A system whose data has a `decorations` key and a
//! Nova ID gives one entry, keyed by the flight's seed rule
//! (`nova_view::flight::sky::sky_seed`): an empty list for an empty key,
//! so its sky draws none. A system with no key gives no entry, and draws
//! the renderer's default for its seed ([`crate::sky`]).
//!
//! # A decoration
//!
//! Each field the data leaves out takes the renderer's default
//! ([`SkyDecoration::new`]); a place left out is the seed's for the
//! decoration's index in the data's list
//! ([`seeded_place`](crate::sky::seeded_place)). A colour's bytes are
//! taken to 0 to 1.
//!
//! # Its image
//!
//! The image is a texture asset's one `.png`, read through the format's
//! `Source` port as a picture of any shape
//! (`nova_galaxy_media::read_picture`), held to the default budget's
//! `texture` limits and to [`SKY_IMAGE_SIDE`] on its longer side whatever
//! the budget says, then premultiplied and mipped ([`SkyImage::new`]).
//! Each distinct asset is read and decoded once, here, at load, and
//! every decoration that names it shares the one image behind a pointer,
//! so the renderer uploads it once.
//!
//! # What fails
//!
//! A decoration whose image can't be had is left out, and the others
//! keep their places. A failure that is news is a problem, once for each
//! asset, naming the first system that lists it: an asset that is a
//! model, a `.png` that doesn't read, one over its art budget, and one
//! over [`SKY_IMAGE_SIDE`].
//!
//! An asset the manifest lacks is not news here, because the loader has
//! told it already: it reports the id against each system's file that
//! lists it ([`nova_data::new_game::skies`]), among the game's own
//! problems, which the start-up shows with these. As for a planet's
//! texture, a placeholder with no file, a missing file and a Git LFS
//! pointer are not news either: a placeholder is no fault, a missing
//! file is left to `galaxy check` as other missing media are
//! ([`nova_data::new_game::looks`]), and the loader lists the pointers
//! ([`NewGame::lfs_pointers`]). Stock data is never a new game, so it
//! has none of this.

use std::collections::BTreeMap;
use std::sync::Arc;

use nova_data::new_game::NewGame;
use nova_data::wire::id::SystemId;
use nova_galaxy_format::asset::AssetKind;
use nova_galaxy_format::id::Id;
use nova_galaxy_format::sky::decoration::{Blend, Decoration};
use nova_galaxy_format::source::Source;
use nova_galaxy_media::budget::limits;
use nova_galaxy_media::{NoTexture, read_picture};
use nova_view::Point;
use nova_view::flight::sky::sky_seed;

use crate::galaxy_models::overrun;
use crate::sky::{SKY_IMAGE_SIDE, SkyBlend, SkyDecoration, SkyImage, seeded_place};

/// The decorations of a game's systems, and what failed reading them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkyDecorations {
    /// Each listing system's decorations, by its sky's seed, in the
    /// data's order less those whose image couldn't be had.
    pub lists: BTreeMap<u64, Vec<SkyDecoration>>,
    /// What failed that is news, each a line ready to report.
    pub problems: Vec<String>,
}

/// The decorations of `game`'s systems that list them, their images read
/// through `source` (see the module docs).
pub fn sky_decorations(game: &NewGame, source: &impl Source) -> SkyDecorations {
    let mut built = SkyDecorations::default();
    // Each asset asked for, with its image or none: read once, reported
    // once.
    let mut images: BTreeMap<Id, Option<Arc<SkyImage>>> = BTreeMap::new();
    for (id, &nova) in &game.ids.systems {
        let Some(listed) = (game.galaxy.systems.get(id))
            .and_then(|system| system.data.sky.as_ref())
            .and_then(|sky| sky.decorations.as_ref())
        else {
            continue;
        };
        let seed = sky_seed(SystemId(nova));
        let mut list = Vec::new();
        for (index, decoration) in listed.as_slice().iter().enumerate() {
            let asset = &decoration.image;
            let image = images.entry(asset.clone()).or_insert_with(|| {
                read(game, source, asset)
                    .map_err(|why| {
                        built.problems.extend(why.map(|why| {
                            format!(
                                "system `{id}`'s sky decoration `{asset}` {why}; it is not drawn"
                            )
                        }));
                    })
                    .ok()
            });
            if let Some(image) = image {
                list.push(decorated(decoration, Arc::clone(image), seed, index));
            }
        }
        built.lists.insert(seed, list);
    }
    built
}

/// `decoration`, the `index`th of the list of the sky of seed `seed`, as
/// the renderer draws it with `image`: each field the data leaves out at
/// the renderer's default, and a place left out at the seed's.
fn decorated(
    decoration: &Decoration,
    image: Arc<SkyImage>,
    seed: u64,
    index: usize,
) -> SkyDecoration {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "each is within a small range the format holds it to, and a place is a stellar's"
    )]
    let at = decoration.at.map_or_else(
        || seeded_place(seed, index),
        |[x, y]| Point::new(x as f32, y as f32),
    );
    let plain = SkyDecoration::new(image, at);
    #[allow(
        clippy::cast_possible_truncation,
        reason = "each is within a small range the format holds it to"
    )]
    SkyDecoration {
        parallax: (decoration.parallax).map_or(plain.parallax, |p| p.get() as f32),
        size: decoration.size.map_or(plain.size, |s| s.get() as f32),
        turn: decoration.turn.map_or(plain.turn, |t| t.get() as f32),
        spin: decoration.spin.map_or(plain.spin, |s| s.get() as f32),
        tint: (decoration.tint.as_ref()).map(|c| c.rgb().map(|c| f32::from(c) / 255.0)),
        brightness: (decoration.brightness).map_or(plain.brightness, |b| b.get() as f32),
        // Each of the format's blends is one of the renderer's, so this
        // names them all: a new one has to be drawn before it is read.
        blend: match decoration.blend {
            Blend::Over => SkyBlend::Over,
            Blend::Glow => SkyBlend::Glow,
        },
        ..plain
    }
}

/// The image of texture asset `asset`, read through `source`; or why it
/// has none, as the end of a line to report, `None` for what is not
/// news (see the module docs).
fn read(game: &NewGame, source: &impl Source, asset: &Id) -> Result<Arc<SkyImage>, Option<String>> {
    // An id the manifest lacks is the loader's to report, and it has.
    let doc = &game.galaxy.assets.get(asset).ok_or(None)?.data;
    if doc.kind != AssetKind::Texture {
        return Err(Some("is a model, not a texture".to_owned()));
    }
    let limits = limits(&game.galaxy, asset);
    let picture = read_picture(source, doc, &limits, SKY_IMAGE_SIDE).map_err(no_picture)?;
    SkyImage::new(picture.width, picture.height, picture.rgba)
        .map(Arc::new)
        .map_err(|why| Some(format!("doesn't read: `{}`: {why}", picture.file)))
}

/// Why a picture can't be had, as [`read`] says it.
fn no_picture(why: NoTexture) -> Option<String> {
    match why {
        NoTexture::None | NoTexture::Pointer | NoTexture::Missing(_) => None,
        NoTexture::Unreadable(why) => Some(format!("doesn't read: {why}")),
        NoTexture::OverBudget(over) => {
            let over: Vec<String> = over.iter().map(|o| overrun(o).to_string()).collect();
            Some(format!("is over its art budget: {}", over.join("; ")))
        }
        NoTexture::TooLarge { file, side, most } => Some(format!(
            "file `{file}` is {side} pixels on its longer side, over the {most} pixel limit of \
             a sky image"
        )),
        // A picture is of any shape, so its reader never says this.
        NoTexture::Shape {
            file,
            width,
            height,
        } => Some(format!("file `{file}` is {width} by {height}")),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::path::Path;

    use nova_data::new_game::fixture::{
        DECORATED_SAMPLE_BLOCK, SKY_GALAXY, SKY_GALAXY_PNG, SKY_GALAXY_SIZE, SKY_HULK,
        SKY_HULK_PNG, SKY_HULK_SIZE, decorated_sample, modelled_sample, sample_folder,
        sky_galaxy_rgba, sky_hulk_rgba,
    };
    use nova_data::new_game::load;
    use nova_galaxy_format::source::{CountingSource, MemSource};
    use nova_galaxy_media::fixture::png;

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

    /// `source` with `block` in system `system`'s front matter, in place
    /// of any `sky` block [`decorated_sample`] gave it.
    fn with_sky(source: MemSource, system: &str, block: &str) -> MemSource {
        let file = format!("systems/{system}.md");
        let text = std::fs::read_to_string(sample_folder().join(&file)).expect("a sample file");
        let (front, body) = text
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---\n"))
            .expect("front matter");
        source.with(&file, &format!("---\n{front}\n{block}---\n{body}"))
    }

    /// A texture asset's entry naming `file`.
    fn texture(file: &str) -> String {
        format!("---\nkind: texture\nplaceholder: false\nfiles: [{file}]\n---\n")
    }

    #[test]
    fn the_decorated_sample_gives_its_start_systems_two_decorations_by_its_seed() {
        let source = decorated_sample();
        let game = loaded(&source);
        let built = sky_decorations(&game, &source);
        assert_eq!(built.problems, Vec::<String>::new());
        let seed = seed_of(&game, "luyten-crossing");
        assert_eq!(built.lists.keys().copied().collect::<Vec<_>>(), [seed]);
        let [hulk, galaxy] = &built.lists[&seed][..] else {
            panic!("two decorations: {:?}", built.lists[&seed]);
        };
        // The hulk: the data's values.
        let (width, height) = SKY_HULK_SIZE;
        assert_eq!(
            *hulk.image,
            SkyImage::new(width, height, sky_hulk_rgba()).expect("the hulk")
        );
        assert_eq!(hulk.at, Point::new(140.0, -90.0));
        assert_eq!(hulk.parallax, 0.2);
        assert_eq!(hulk.size, 96.0);
        assert_eq!(hulk.turn, 30.0);
        assert_eq!(hulk.spin, 4.0);
        assert_eq!(hulk.tint, Some([1.0, 176.0 / 255.0, 112.0 / 255.0]));
        assert_eq!(hulk.brightness, 0.75);
        assert_eq!(hulk.blend, SkyBlend::Over);
        // The galaxy: the engine's defaults, at its seed's place for the
        // second of the list, glowing.
        let (width, height) = SKY_GALAXY_SIZE;
        let image = SkyImage::new(width, height, sky_galaxy_rgba()).expect("the galaxy");
        let expected = SkyDecoration {
            blend: SkyBlend::Glow,
            ..SkyDecoration::new(Arc::new(image), seeded_place(seed, 1))
        };
        assert_eq!(*galaxy, expected);
        assert_ne!(seeded_place(seed, 1), seeded_place(seed, 0));
    }

    #[test]
    fn an_empty_list_is_an_entry_of_none_and_no_key_is_no_entry() {
        let source = with_sky(
            with_sky(
                decorated_sample(),
                "brightwater",
                "sky: {decorations: []}\n",
            ),
            "marrow",
            "sky: {stars: 0.5}\n",
        );
        let game = loaded(&source);
        let built = sky_decorations(&game, &source);
        assert_eq!(built.problems, Vec::<String>::new());
        assert_eq!(
            built.lists.get(&seed_of(&game, "brightwater")),
            Some(&Vec::new())
        );
        assert_eq!(built.lists.get(&seed_of(&game, "marrow")), None);
        assert_eq!(built.lists.len(), 2);
        // A sample with no sky block anywhere has no entry at all.
        let plain = modelled_sample();
        let game = loaded(&plain);
        assert_eq!(sky_decorations(&game, &plain), SkyDecorations::default());
    }

    #[test]
    fn systems_naming_one_asset_share_its_one_image_read_once() {
        let source = with_sky(
            decorated_sample(),
            "marrow",
            "sky:\n  decorations:\n    - image: far-hulk\n      size: 40\n    \
             - image: far-hulk\n      turn: 90\n",
        );
        let game = loaded(&source);
        let counting = CountingSource::new(&source);
        let built = sky_decorations(&game, &counting);
        assert_eq!(built.problems, Vec::<String>::new());
        let crossing = &built.lists[&seed_of(&game, "luyten-crossing")];
        let marrow = &built.lists[&seed_of(&game, "marrow")];
        assert_eq!((crossing.len(), marrow.len()), (2, 2));
        assert!(Arc::ptr_eq(&crossing[0].image, &marrow[0].image));
        assert!(Arc::ptr_eq(&marrow[0].image, &marrow[1].image));
        assert!(!Arc::ptr_eq(&crossing[0].image, &crossing[1].image));
        assert_eq!(counting.bytes_asked(SKY_HULK_PNG), 1);
        assert_eq!(counting.bytes_asked(SKY_GALAXY_PNG), 1);
        // Each keeps its own values.
        assert_eq!((marrow[0].size, marrow[1].size), (40.0, 128.0));
        assert_eq!((marrow[0].turn, marrow[1].turn), (0.0, 90.0));
        // And its own index's place.
        let seed = seed_of(&game, "marrow");
        assert_eq!(marrow[0].at, seeded_place(seed, 0));
        assert_eq!(marrow[1].at, seeded_place(seed, 1));
    }

    /// The decorated sample with Marrow listing the hulk too, so an asset
    /// that fails is listed by two systems.
    fn listed_twice() -> MemSource {
        with_sky(
            decorated_sample(),
            "marrow",
            "sky:\n  decorations:\n    - image: far-hulk\n    - image: far-galaxy\n",
        )
    }

    /// The problems of `source`, which loads with none of its own, and
    /// how many decorations each of its two listing systems keeps.
    fn problems(source: &MemSource) -> (Vec<String>, (usize, usize)) {
        problems_of(&loaded(source), source)
    }

    /// The problems reading the decorations of `game`, loaded from
    /// `source`, and how many each of its two listing systems keeps.
    fn problems_of(game: &NewGame, source: &MemSource) -> (Vec<String>, (usize, usize)) {
        let built = sky_decorations(game, source);
        let kept = |system: &str| built.lists[&seed_of(game, system)].len();
        (
            built.problems.clone(),
            (kept("luyten-crossing"), kept("marrow")),
        )
    }

    #[test]
    fn each_kind_of_news_is_one_problem_naming_the_system_and_the_asset() {
        // A model, not a texture.
        let model = listed_twice().with(
            &format!("assets/{SKY_HULK}.md"),
            "---\nkind: model\nplaceholder: true\n---\n",
        );
        assert_eq!(
            problems(&model),
            (
                vec![format!(
                    "system `luyten-crossing`'s sky decoration `{SKY_HULK}` is a model, not a \
                     texture; it is not drawn"
                )],
                (1, 1)
            )
        );
        // A PNG that doesn't read.
        let broken = listed_twice().with_bytes(SKY_HULK_PNG, b"GIF89a nonsense");
        let (lines, kept) = problems(&broken);
        assert_eq!(kept, (1, 1));
        let [line] = &lines[..] else {
            panic!("one problem: {lines:?}");
        };
        assert!(
            line.starts_with(&format!(
                "system `luyten-crossing`'s sky decoration `{SKY_HULK}` doesn't read: \
                 `{SKY_HULK_PNG}`: "
            )) && line.ends_with("; it is not drawn"),
            "{line}"
        );
        // Over its art budget, on both measures.
        let tight = listed_twice().with(
            "budgets/default.md",
            "---\nmodel: {triangles: 20000, kib: 4096}\ntexture: {kib: 4096, side: 12}\n---\n",
        );
        assert_eq!(
            problems(&tight),
            (
                vec![format!(
                    "system `luyten-crossing`'s sky decoration `{SKY_HULK}` is over its art \
                     budget: is 16 pixels on its longest side, over the 12 pixel budget in \
                     `budgets/default.md`; it is not drawn"
                )],
                (1, 1)
            )
        );
        // Over the engine's side, whatever the budget allows: a header
        // alone says so, before anything is decoded.
        let mut wide = png(1, 1, &[0; 4])[..33].to_vec();
        wide[16..20].copy_from_slice(&2048_u32.to_be_bytes());
        let huge = listed_twice().with_bytes(SKY_GALAXY_PNG, &wide);
        assert_eq!(
            problems(&huge),
            (
                vec![format!(
                    "system `luyten-crossing`'s sky decoration `{SKY_GALAXY}` file \
                     `{SKY_GALAXY_PNG}` is 2048 pixels on its longer side, over the 1024 pixel \
                     limit of a sky image; it is not drawn"
                )],
                (1, 1)
            )
        );
    }

    #[test]
    fn what_is_not_news_leaves_the_decoration_out_unreported() {
        let none = (Vec::<String>::new(), (1, 1));
        // An asset the manifest lacks: the loader's to report, against
        // the system's file, so its game loads with a problem of its own
        // and reading the decorations adds none.
        let unknown = with_sky(
            listed_twice(),
            "luyten-crossing",
            "sky:\n  decorations:\n    - image: no-such-thing\n    - image: far-galaxy\n",
        );
        let game = load(&unknown, Path::new("sample"));
        assert_eq!(problems_of(&game, &unknown), (Vec::new(), (1, 2)));
        // A placeholder with no file.
        let placeholder = listed_twice().with(
            &format!("assets/{SKY_HULK}.md"),
            "---\nkind: texture\nplaceholder: true\n---\n",
        );
        assert_eq!(problems(&placeholder), none);
        // A missing file.
        let missing = listed_twice().with(
            &format!("assets/{SKY_HULK}.md"),
            &texture("media/export/sky/gone.png"),
        );
        assert_eq!(problems(&missing), none);
        // A Git LFS pointer.
        let pointer = listed_twice().with_bytes(
            SKY_HULK_PNG,
            b"version https://git-lfs.github.com/spec/v1\noid sha256:ab\nsize 12\n",
        );
        assert_eq!(problems(&pointer), none);
    }

    #[test]
    fn a_decoration_left_out_leaves_the_others_their_own_places() {
        // The first of the list fails, and the second leaves its place
        // to the seed: it takes the place of the list's second, not the
        // first. The loader reports the id its manifest lacks, so the
        // game isn't held to loading with no problem.
        let source = with_sky(
            decorated_sample(),
            "luyten-crossing",
            "sky:\n  decorations:\n    - image: no-such-thing\n    - image: far-galaxy\n",
        );
        let game = load(&source, Path::new("sample"));
        let seed = seed_of(&game, "luyten-crossing");
        let built = sky_decorations(&game, &source);
        let [galaxy] = &built.lists[&seed][..] else {
            panic!("one decoration");
        };
        assert_eq!(galaxy.at, seeded_place(seed, 1));
    }

    #[test]
    fn the_world_assets_carry_the_decorations_and_their_problems() {
        let source = decorated_sample();
        let game = loaded(&source);
        let built = sky_decorations(&game, &source);
        let assets = crate::world_assets(&game, &source);
        let seed = seed_of(&game, "luyten-crossing");
        assert_eq!(
            assets.skies.decorations(seed),
            Some(&built.lists[&seed][..])
        );
        assert_eq!(assets.skies.decorations(seed).map(<[_]>::len), Some(2));
        assert_eq!(assets.skies.decorations(seed_of(&game, "marrow")), None);
        assert_eq!(assets.problems, Vec::<String>::new());
        // The modelled sample, which lists none, carries none.
        let plain = modelled_sample();
        let game = loaded(&plain);
        let assets = crate::world_assets(&game, &plain);
        for system in ["luyten-crossing", "brightwater", "marrow"] {
            assert_eq!(assets.skies.decorations(seed_of(&game, system)), None);
        }
        // A problem reading an image is one of the world's.
        let broken = decorated_sample().with_bytes(SKY_HULK_PNG, b"GIF89a nonsense");
        let game = loaded(&broken);
        let assets = crate::world_assets(&game, &broken);
        let [line] = &assets.problems[..] else {
            panic!("one problem: {:?}", assets.problems);
        };
        assert!(
            line.contains(SKY_HULK) && line.ends_with("it is not drawn"),
            "{line}"
        );
        assert_eq!(DECORATED_SAMPLE_BLOCK.matches("image:").count(), 2);
    }
}
