//! What the new game's hulls, stellars and gates draw in place of sprites
//! ([`stand_ins`]): the bible has models, not Nova's sprites, so each is
//! drawn as a [`StandIn`] the store keeps
//! ([`GameData::with_stand_ins`](crate::store::GameData::with_stand_ins)).
//!
//! - **A hull** with a `size` is its model seen from above, placed at its
//!   length, its painted materials in its livery's colours, in pixels at
//!   [`PX_PER_METRE`] (`nova_galaxy_media::top_down`). Without a model
//!   that reads (a placeholder asset, an LFS pointer, a file that doesn't
//!   decode), it is its size box in its livery's primary, or bare grey
//!   ([`BARE`]) without a livery. A hull with no `size` has none, as it
//!   has no `shän`, and draws as the view's crossed box.
//! - **A station or outpost** whose `visual` is a model that reads is the
//!   model seen from above in its own colours and units: it has no size,
//!   so the view scales it.
//! - **Every other stellar** is a disc: in its generated look's first
//!   palette colour, else its owner's faction colour, else bare grey.
//! - **A jump point's gate** is a ring in bare grey.
//!
//! A model draws whenever one of its asset's files reads as a model,
//! whatever its `placeholder` flag, as `galaxy`'s contact sheet draws it
//! (`nova_galaxy_media::read_model`).
//!
//! # Problems
//!
//! Each against the file that names the thing, and the record still
//! draws:
//!
//! - a hull's `livery` that isn't in `liveries/`: it draws bare;
//! - a hull's `model` or a stellar's `visual` asset that isn't in
//!   `assets/`: it draws as its box or its disc;
//! - an asset whose model file doesn't decode, against the asset, once:
//!   whatever shows it draws as its box or its disc.
//!
//! An LFS pointer isn't reported again: [`NewGame::lfs_pointers`] lists
//! them. Nor is a reference to a file that failed to load. A model file
//! that isn't there falls back unreported, as other missing media do:
//! `galaxy check` reports it.
//!
//! [`NewGame::lfs_pointers`]: super::NewGame::lfs_pointers

use std::collections::BTreeMap;

use nova_galaxy_format::asset::{Asset, Front};
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Doc, Galaxy, Problem};
use nova_galaxy_format::model::{Stellar as BibleStellar, StellarKind, Visual};
use nova_galaxy_format::source::Source;
use nova_galaxy_media::{BARE, Model, NoModel, Placement, read_model, rgb, silhouette, top_down};

use super::fleet::PX_PER_METRE;
use super::ids::IdTable;
use super::map::absent;
use crate::store::{StandIn, StandInOf};
use crate::wire::id::{ShipId, StellarId};

/// The stand-in of each hull, stellar and gate of `galaxy` that has a
/// record in `ids`, by its record, reading the models through `source`
/// (see the module docs). What can't be drawn as written is added to
/// `problems`.
pub fn stand_ins(
    galaxy: &Galaxy,
    ids: &IdTable,
    source: &impl Source,
    problems: &mut Vec<Problem>,
) -> BTreeMap<StandInOf, StandIn> {
    let mut models = Models::new(source);
    let mut stand_ins = BTreeMap::new();
    for (id, &nova) in &ids.ships {
        let doc = &galaxy.ships[id];
        let ship = &doc.data;
        let Some(size) = ship.size else { continue };
        let livery = ship.livery.as_ref().and_then(|livery| {
            let found = galaxy.liveries.get(livery).map(|l| &l.data.palette);
            if found.is_none() && absent(galaxy, &galaxy.liveries, "liveries", livery) {
                problems.push(Problem::new(
                    &doc.path,
                    format!("its livery `{livery}` is not in liveries/; it draws bare"),
                ));
            }
            found
        });
        let asset = ship
            .model
            .as_ref()
            .and_then(|model| asset(galaxy, model, &doc.path, "model", "its size box", problems));
        let model = asset.and_then(|(id, asset)| models.read(id, asset, problems));
        let paint = asset.and_then(|(_, asset)| asset.data.paint.as_ref());
        let outline = top_down(model.map(|(m, f)| (m, *f)), size, paint, livery);
        stand_ins.insert(
            StandInOf::Ship(ShipId(nova)),
            StandIn::Outline(outline.scaled(PX_PER_METRE)),
        );
    }
    for (id, &nova) in &ids.stellars {
        let doc = &galaxy.stellars[id];
        let stellar = &doc.data;
        let asset = match &stellar.visual {
            Some(Visual::Asset(visual)) => {
                asset(galaxy, visual, &doc.path, "visual", "a disc", problems)
            }
            _ => None,
        };
        let model = asset
            .filter(|_| matches!(stellar.kind, StellarKind::Station | StellarKind::Outpost))
            .and_then(|(id, asset)| models.read(id, asset, problems));
        let stand_in = model.map_or_else(
            || disc(galaxy, stellar),
            |(model, front)| {
                StandIn::Outline(silhouette(
                    model,
                    &Placement::unscaled(model, *front),
                    &BTreeMap::new(),
                ))
            },
        );
        stand_ins.insert(StandInOf::Stellar(StellarId(nova)), stand_in);
    }
    for &nova in ids.gates.values() {
        stand_ins.insert(
            StandInOf::Stellar(StellarId(nova)),
            StandIn::Ring { color: BARE },
        );
    }
    stand_ins
}

/// The disc `stellar` draws: in its generated look's first palette colour,
/// else its owner's colour, else bare grey.
fn disc(galaxy: &Galaxy, stellar: &BibleStellar) -> StandIn {
    let palette = match &stellar.visual {
        Some(Visual::Generated(look)) => look.palette.first(),
        _ => None,
    };
    let owner = stellar
        .owner
        .as_ref()
        .and_then(|owner| galaxy.factions.get(owner))
        .map(|faction| &faction.data.color);
    StandIn::Disc {
        color: palette.or(owner).map_or(BARE, rgb),
    }
}

/// The asset `id`, named as `what` in `file`; none, reported (it draws as
/// `instead`), when it isn't in `assets/`.
fn asset<'a>(
    galaxy: &'a Galaxy,
    id: &'a Id,
    file: &str,
    what: &str,
    instead: &str,
    problems: &mut Vec<Problem>,
) -> Option<(&'a Id, &'a Doc<Asset>)> {
    let found = galaxy.assets.get(id);
    if found.is_none() && absent(galaxy, &galaxy.assets, "assets", id) {
        problems.push(Problem::new(
            file,
            format!("its {what} `{id}` is not in assets/; it draws as {instead}"),
        ));
    }
    found.map(|asset| (id, asset))
}

/// Each asset's model, read once.
struct Models<'a, S> {
    source: &'a S,
    read: BTreeMap<&'a Id, Result<(Model, Front), NoModel>>,
}

impl<'a, S: Source> Models<'a, S> {
    fn new(source: &'a S) -> Self {
        Self {
            source,
            read: BTreeMap::new(),
        }
    }

    /// The model of the asset `id`, read the first time it is asked for,
    /// when that reports why a file that should be a model isn't one.
    fn read(
        &mut self,
        id: &'a Id,
        asset: &'a Doc<Asset>,
        problems: &mut Vec<Problem>,
    ) -> Option<&(Model, Front)> {
        let source = self.source;
        self.read
            .entry(id)
            .or_insert_with(|| {
                let read = read_model(source, &asset.data);
                if let Err(NoModel::Unreadable(why)) = &read {
                    problems.push(Problem::new(
                        &asset.path,
                        format!("its model doesn't read: {why}; what shows it draws as a stand-in"),
                    ));
                }
                read
            })
            .as_ref()
            .ok()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::path::Path;

    use nova_galaxy_format::fixture::{self, CC0};
    use nova_galaxy_format::fleet::Size;
    use nova_galaxy_format::media::LFS_POINTER;
    use nova_galaxy_format::source::MemSource;
    use nova_galaxy_media::{Outline, size_box};

    use super::*;
    use crate::new_game::{NewGame, load};

    const HULL_GLB: &str = "media/export/pack/hull.glb";

    fn loaded(source: &MemSource) -> NewGame {
        load(source, Path::new("sample"))
    }

    /// The format's galaxy with the skiff's model, a 2 by 1 by 4 box in
    /// one material, `hull`.
    fn with_hull() -> MemSource {
        fixture::galaxy().with_bytes(
            HULL_GLB,
            &nova_galaxy_media::fixture::glb([2.0, 1.0, 4.0], [0.0; 3]),
        )
    }

    /// `source` with the skiff's asset a placeholder, with no files.
    fn placeholder_hull(source: MemSource) -> MemSource {
        source.with(
            "assets/hull.md",
            "---\nkind: model\nplaceholder: true\npaint: {primary: [hull]}\n---\n",
        )
    }

    fn ship(game: &NewGame, id: &str) -> Option<StandIn> {
        let nova = game.ids.ships[&id_of(id)];
        game.data.stand_in(StandInOf::Ship(ShipId(nova))).cloned()
    }

    fn stellar(game: &NewGame, id: &str) -> Option<StandIn> {
        let nova = game.ids.stellars[&id_of(id)];
        game.data
            .stand_in(StandInOf::Stellar(StellarId(nova)))
            .cloned()
    }

    fn id_of(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    fn outline(stand_in: Option<StandIn>) -> Outline {
        match stand_in {
            Some(StandIn::Outline(outline)) => outline,
            other => panic!("an outline: {other:?}"),
        }
    }

    /// The skiff's size: 8 m long, 4 m across.
    fn skiff_size() -> Size {
        Size {
            length: 8.0,
            beam: 4.0,
            collision_radius: 4.0,
        }
    }

    /// `league-paint`'s primary, `#ddccaa`.
    const LEAGUE_PRIMARY: [u8; 3] = [0xdd, 0xcc, 0xaa];

    fn problems(game: &NewGame) -> Vec<String> {
        game.problems.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_hull_whose_model_reads_is_its_painted_silhouette_at_its_length() {
        let game = loaded(&with_hull());
        let outline = outline(ship(&game, "skiff"));
        let ys: Vec<f64> = outline
            .triangles
            .iter()
            .flat_map(|t| t.corners.map(|c| c[1]))
            .collect();
        let length = ys.iter().copied().fold(f64::MIN, f64::max)
            - ys.iter().copied().fold(f64::MAX, f64::min);
        assert_eq!(length, 8.0 * PX_PER_METRE);
        assert_eq!(outline.triangles.len(), 4, "the box's top and bottom");
        assert!(outline.triangles.iter().all(|t| t.color == LEAGUE_PRIMARY));
        assert_eq!(problems(&game), Vec::<String>::new());
    }

    #[test]
    fn a_hull_whose_asset_is_a_placeholder_is_its_size_box_in_the_primary() {
        let game = loaded(&placeholder_hull(fixture::galaxy()));
        assert_eq!(
            ship(&game, "skiff"),
            Some(StandIn::Outline(
                size_box(skiff_size(), LEAGUE_PRIMARY).scaled(PX_PER_METRE)
            ))
        );
        assert_eq!(problems(&game), Vec::<String>::new());
    }

    #[test]
    fn a_hull_whose_model_is_an_lfs_pointer_is_its_box_with_no_problem() {
        let pointer = [LFS_POINTER, b"\noid sha256:ab\nsize 12\n"].concat();
        let game = loaded(&fixture::galaxy().with_bytes(HULL_GLB, &pointer));
        assert_eq!(
            ship(&game, "skiff"),
            Some(StandIn::Outline(
                size_box(skiff_size(), LEAGUE_PRIMARY).scaled(PX_PER_METRE)
            ))
        );
        assert_eq!(problems(&game), Vec::<String>::new());
        assert_eq!(game.lfs_pointers, [HULL_GLB]);
    }

    #[test]
    fn a_hull_whose_model_does_not_decode_is_its_box_and_the_asset_is_reported() {
        let game = loaded(&fixture::galaxy().with_bytes(HULL_GLB, b"glTF nonsense"));
        assert_eq!(
            ship(&game, "skiff"),
            Some(StandIn::Outline(
                size_box(skiff_size(), LEAGUE_PRIMARY).scaled(PX_PER_METRE)
            ))
        );
        let problems = problems(&game);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].starts_with(&format!(
                "assets/hull.md: its model doesn't read: `{HULL_GLB}`: not a readable glTF model"
            )),
            "{problems:?}"
        );
    }

    #[test]
    fn an_undecodable_model_is_reported_once_for_its_asset() {
        // Two hulls on one asset: one problem.
        let source = fixture::galaxy()
            .with_bytes(HULL_GLB, b"glTF nonsense")
            .with(
                "ships/barge.md",
                "---\nname: Barge\nmodel: hull\nsize: {length: 20, beam: 8, collision_radius: 8}\n---\n",
            );
        let game = loaded(&source);
        assert_eq!(problems(&game).len(), 1, "{:?}", problems(&game));
        assert!(matches!(ship(&game, "barge"), Some(StandIn::Outline(_))));
    }

    #[test]
    fn a_missing_model_file_is_its_box_and_left_to_the_galaxy_tool() {
        // The format's galaxy holds no media: `hull.glb` isn't there.
        let game = loaded(&fixture::galaxy());
        assert_eq!(
            ship(&game, "skiff"),
            Some(StandIn::Outline(
                size_box(skiff_size(), LEAGUE_PRIMARY).scaled(PX_PER_METRE)
            ))
        );
        assert_eq!(problems(&game), Vec::<String>::new());
    }

    #[test]
    fn a_hull_with_no_livery_is_bare() {
        let source = placeholder_hull(fixture::galaxy()).with(
            "ships/skiff.md",
            "---\nname: Skiff\nmodel: hull\nsize: {length: 8, beam: 4, collision_radius: 4}\n---\n",
        );
        let game = loaded(&source);
        assert_eq!(
            ship(&game, "skiff"),
            Some(StandIn::Outline(
                size_box(skiff_size(), BARE).scaled(PX_PER_METRE)
            ))
        );
    }

    #[test]
    fn a_livery_not_in_liveries_is_reported_and_the_hull_draws_bare() {
        let source = placeholder_hull(fixture::galaxy()).with(
            "ships/skiff.md",
            "---\nname: Skiff\nmodel: hull\nlivery: gone\n\
             size: {length: 8, beam: 4, collision_radius: 4}\n---\n",
        );
        let game = loaded(&source);
        assert_eq!(
            ship(&game, "skiff"),
            Some(StandIn::Outline(
                size_box(skiff_size(), BARE).scaled(PX_PER_METRE)
            ))
        );
        assert_eq!(
            problems(&game),
            ["ships/skiff.md: its livery `gone` is not in liveries/; it draws bare"]
        );
    }

    #[test]
    fn a_livery_that_failed_to_load_is_not_reported_again() {
        let source = placeholder_hull(fixture::galaxy())
            .with("liveries/league-paint.md", "---\nname: [\n---\n");
        let game = loaded(&source);
        assert_eq!(
            ship(&game, "skiff"),
            Some(StandIn::Outline(
                size_box(skiff_size(), BARE).scaled(PX_PER_METRE)
            ))
        );
        assert!(
            problems(&game)
                .iter()
                .all(|p| p.starts_with("liveries/league-paint.md")),
            "{:?}",
            problems(&game)
        );
    }

    #[test]
    fn a_model_not_in_assets_is_reported_and_the_hull_is_its_box() {
        let source = fixture::galaxy().with(
            "ships/skiff.md",
            "---\nname: Skiff\nmodel: gone\nlivery: league-paint\n\
             size: {length: 8, beam: 4, collision_radius: 4}\n---\n",
        );
        let game = loaded(&source);
        assert_eq!(
            ship(&game, "skiff"),
            Some(StandIn::Outline(
                size_box(skiff_size(), LEAGUE_PRIMARY).scaled(PX_PER_METRE)
            ))
        );
        assert_eq!(
            problems(&game),
            ["ships/skiff.md: its model `gone` is not in assets/; it draws as its size box"]
        );
    }

    #[test]
    fn an_asset_that_failed_to_load_is_not_reported_again() {
        let game = loaded(&fixture::galaxy().with("assets/hull.md", "---\nkind: [\n---\n"));
        assert_eq!(
            ship(&game, "skiff"),
            Some(StandIn::Outline(
                size_box(skiff_size(), LEAGUE_PRIMARY).scaled(PX_PER_METRE)
            ))
        );
        assert!(
            problems(&game)
                .iter()
                .all(|p| p.starts_with("assets/hull.md")),
            "{:?}",
            problems(&game)
        );
    }

    #[test]
    fn a_hull_with_no_size_has_no_stand_in() {
        let source = with_hull().with(
            "ships/skiff.md",
            "---\nname: Skiff\nmodel: hull\nlivery: league-paint\n---\n",
        );
        let game = loaded(&source);
        assert_eq!(ship(&game, "skiff"), None);
    }

    #[test]
    fn only_hulls_have_stand_ins() {
        // The armed skiff is a variant: no `shïp`, so no stand-in.
        let game = loaded(&with_hull());
        let ships = game.ids.ships.len();
        assert_eq!(ships, 1, "the skiff alone");
        let stand_ins: Vec<_> = (128..=200)
            .filter(|&nova| game.data.stand_in(StandInOf::Ship(ShipId(nova))).is_some())
            .collect();
        assert_eq!(stand_ins, [game.ids.ships[&id_of("skiff")]]);
    }

    /// The format's galaxy with `alpha-yard` shown by `yard.glb`, a 4 by 1
    /// by 2 box in white.
    fn yard_on_a_model() -> MemSource {
        fixture::galaxy()
            .with(
                "assets/yard.md",
                &format!(
                    "---\nkind: model\nplaceholder: false\nfiles: [media/export/pack/yard.glb]\n{CC0}---\n"
                ),
            )
            .with_bytes(
                "media/export/pack/yard.glb",
                &nova_galaxy_media::fixture::glb([4.0, 1.0, 2.0], [0.0; 3]),
            )
            .with(
                "stellars/alpha-yard.md",
                "---\nname: Alpha Yard\nkind: station\nsystem: alpha\nposition: [0, 0]\nowner: league\n\
                 visual: {asset: yard}\n---\n",
            )
    }

    #[test]
    fn a_station_whose_model_reads_is_its_silhouette_in_its_own_units() {
        let game = loaded(&yard_on_a_model());
        let outline = outline(stellar(&game, "alpha-yard"));
        assert_eq!(outline.triangles.len(), 4);
        assert_eq!(
            outline.reach(),
            2.0_f64.hypot(1.0),
            "4 by 2 units, a metre each"
        );
        assert!(
            outline.triangles.iter().all(|t| t.color == [255; 3]),
            "its own white"
        );
    }

    #[test]
    fn a_planet_whose_visual_is_a_model_is_still_a_disc() {
        let source = yard_on_a_model().with(
            "stellars/alpha-yard.md",
            "---\nname: Alpha Yard\nkind: planet\nsystem: alpha\nposition: [0, 0]\n\
             visual: {asset: yard}\n---\n",
        );
        let game = loaded(&source);
        assert_eq!(
            stellar(&game, "alpha-yard"),
            Some(StandIn::Disc { color: BARE })
        );
    }

    #[test]
    fn a_station_on_a_placeholder_is_a_disc_in_its_owner_s_colour() {
        // `alpha-yard` shows `spire`, a placeholder; `league` is #aa8800.
        let game = loaded(&with_hull());
        assert_eq!(
            stellar(&game, "alpha-yard"),
            Some(StandIn::Disc {
                color: [0xaa, 0x88, 0x00]
            })
        );
    }

    #[test]
    fn a_generated_planet_is_a_disc_in_its_first_palette_colour() {
        let source = with_hull().with(
            "stellars/alpha-prime.md",
            "---\nname: Alpha Prime\nkind: planet\nsystem: alpha\nposition: [50, 0]\nowner: league\n\
             visual: {generated: {type: rocky, palette: [\"#123456\", \"#654321\"], seed: 1}}\n---\n",
        );
        let game = loaded(&source);
        assert_eq!(
            stellar(&game, "alpha-prime"),
            Some(StandIn::Disc {
                color: [0x12, 0x34, 0x56]
            })
        );
    }

    #[test]
    fn a_texture_planet_with_no_owner_is_a_grey_disc() {
        // `alpha-prime` shows `surface`, a texture.
        let game = loaded(&with_hull());
        assert_eq!(
            stellar(&game, "alpha-prime"),
            Some(StandIn::Disc { color: BARE })
        );
        assert_eq!(problems(&game), Vec::<String>::new());
    }

    #[test]
    fn a_visual_not_in_assets_is_reported_and_the_stellar_is_a_disc() {
        let source = with_hull().with(
            "stellars/alpha-prime.md",
            "---\nname: Alpha Prime\nkind: planet\nsystem: alpha\nposition: [50, 0]\n\
             visual: {asset: gone}\n---\n",
        );
        let game = loaded(&source);
        assert_eq!(
            stellar(&game, "alpha-prime"),
            Some(StandIn::Disc { color: BARE })
        );
        assert_eq!(
            problems(&game),
            ["stellars/alpha-prime.md: its visual `gone` is not in assets/; it draws as a disc"]
        );
    }

    #[test]
    fn every_gate_is_a_grey_ring_under_its_stellar_id() {
        let game = loaded(&with_hull());
        assert_eq!(game.ids.gates.len(), 2);
        for &nova in game.ids.gates.values() {
            assert_eq!(
                game.data.stand_in(StandInOf::Stellar(StellarId(nova))),
                Some(&StandIn::Ring { color: BARE })
            );
        }
    }
}
