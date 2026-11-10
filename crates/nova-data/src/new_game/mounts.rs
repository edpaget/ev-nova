//! Where each hardpoint's outfit stands on its hull, what it shows there,
//! and each weapon's muzzle reach ([`mounted`]), kept on the store
//! ([`GameData::with_mounts`](crate::store::GameData::with_mounts),
//! [`GameData::with_muzzle_reaches`](crate::store::GameData::with_muzzle_reaches)).
//!
//! The geometry is `nova_galaxy_media::mount`'s, asset decision 2 of
//! game-bible's format: a ray cast straight down at the hardpoint onto
//! the hull's model, scaled to its `size`'s length as the renderer scales
//! it, the mount standing on the highest surface it hits. This module
//! only chooses what to cast and keeps what comes back.
//!
//! - **A ship class**, a hull or a variant, mounts its `base` chain's
//!   hardpoints (a variant's replacing its base's of the same `id`) on
//!   its nearest `model`, at its nearest `size`'s length. One with no
//!   model that reads (a placeholder, an LFS pointer, a file that doesn't
//!   decode, or no `model` at all), or no `size`, mounts them on the
//!   plane.
//! - **A mod kit** mounts its hardpoints on every ship class, since where
//!   they stand depends on the hull it is fitted to. Kits and classes are
//!   few, so every pair is cast.
//! - **A mount shows** its hardpoint's `outfit`'s `model`, the ship as
//!   built, when that model reads; it is lifted by half the model's
//!   height, at a metre a model unit (`nova_galaxy_media::mount::fitting`).
//!   A mount with no outfit, or whose outfit has no model that reads,
//!   shows nothing, though it still fires. A `turret` mount's outfit turns
//!   on it. A `bay` mount is kept, though it has no exit point.
//! - **A weapon's muzzle reach** is its outfit's model's half length,
//!   in whole pixels at [`PX_PER_METRE`] through the conversion a
//!   hardpoint's place shares (`super::fleet::pixels`); none with no
//!   model that reads, or a reach under half a pixel.
//!
//! A weapon the player buys loose sits on no hardpoint, since nothing in
//! the format assigns it one, so it is drawn nowhere.
//!
//! The models are read through the stand-ins' cache ([`Models`]), so each
//! is read once and one that doesn't read is reported once.
//!
//! # Problems
//!
//! Each against the file that names the thing at fault, and the mount
//! still stands:
//!
//! - a hardpoint over no surface of its hull's model, against the file
//!   that adds it (a variant's inherited one against its base, as the
//!   stand-ins report an inherited model), naming the hardpoint and the
//!   model's asset: its outfit sits on the plane. A kit's hardpoint is
//!   reported for the classes its `fits` names or that carry it as built,
//!   naming the class too;
//! - an outfit's `model` that isn't in `assets/`, against the outfit: it
//!   shows nothing.
//!
//! [`PX_PER_METRE`]: super::fleet::PX_PER_METRE

use std::collections::{BTreeMap, BTreeSet};

use nova_galaxy_format::asset::Front;
use nova_galaxy_format::fleet::{Hardpoint, Mount as Kind, resolve};
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Galaxy, Problem};
use nova_galaxy_format::source::Source;
use nova_galaxy_media::Model;
use nova_galaxy_media::mount::{Fitting, Footing, fitting, mount_point};

use super::fleet::{chain_len, pixels};
use super::ids::IdTable;
use super::looks::{Models, asset};
use crate::store::{Mount, Mounts, Shown};
use crate::wire::id::{OutfitId, ShipId, WeaponId};

/// What [`mounted`] works out, for the store.
#[derive(Debug, Default)]
pub struct Mounted {
    /// Every ship class's own mounts, and every kit's on every class.
    pub mounts: Mounts,
    /// Each weapon's muzzle reach in whole pixels, by its `wëap`.
    pub muzzle_reaches: BTreeMap<WeaponId, i16>,
}

/// Every mount and muzzle reach of `galaxy`'s records in `ids`, reading
/// the models through `models` (see the module docs). What can't stand as
/// written is added to `problems`.
pub fn mounted<'a, S: Source>(
    galaxy: &'a Galaxy,
    ids: &IdTable,
    models: &mut Models<'a, S>,
    problems: &mut Vec<Problem>,
) -> Mounted {
    let fitted = outfit_fittings(galaxy, ids, models, problems);
    let muzzle_reaches = ids
        .weapons
        .iter()
        .filter_map(|(outfit, &weapon)| {
            let reach = pixels(fitted.get(outfit)?.1.reach);
            (reach > 0).then_some((WeaponId(weapon), reach))
        })
        .collect();
    let mut mounts = Mounts::default();
    // Off mounts, each once, after the models' own problems.
    let mut offs = BTreeSet::new();
    let mut report = |file: &str, why: String| {
        offs.insert((file.to_owned(), why));
    };
    for (id, &nova) in &ids.ships {
        let Ok(ship) = resolve(galaxy, id) else {
            continue;
        };
        // A hull's model not in `assets/` is the stand-ins' to report.
        let hull = ship.model.and_then(|model| {
            let (model, doc) = galaxy.assets.get_key_value(model)?;
            Some((model, models.read(model, doc, problems)?))
        });
        let on = hull.map(|(asset, (model, front))| (asset, (model, *front)));
        let length = ship.size.map_or(f64::NAN, |size| size.length);
        let stand =
            |hardpoint: &Hardpoint| stand(on.map(|(_, hull)| hull), length, hardpoint, &fitted);
        let chain = &ship.hardpoints[..chain_len(galaxy, &ship)];
        let own: Vec<Mount> = chain
            .iter()
            .map(|placed| {
                let mount = stand(placed.hardpoint);
                if let (Footing::Off, Some((asset, _))) = (mount.footing, on) {
                    report(
                        &galaxy.ships[placed.from].path,
                        off(&mount.hardpoint, asset, None),
                    );
                }
                mount
            })
            .collect();
        if !own.is_empty() {
            mounts.hulls.insert(ShipId(nova), own);
        }
        for (kit, &kit_nova) in &ids.kits {
            let doc = &galaxy.mods[kit];
            let carried = ship.mods.contains(kit);
            let fits = ship.role.is_some_and(|role| doc.data.fits.contains(&role));
            let placed: Vec<Mount> = doc
                .data
                .hardpoints
                .iter()
                .map(|hardpoint| {
                    let mount = stand(hardpoint);
                    if let (Footing::Off, Some((asset, _)), true) =
                        (mount.footing, on, carried || fits)
                    {
                        report(&doc.path, off(&mount.hardpoint, asset, Some(id)));
                    }
                    mount
                })
                .collect();
            if !placed.is_empty() {
                mounts
                    .kits
                    .insert((ShipId(nova), OutfitId(kit_nova)), placed);
            }
        }
    }
    problems.extend(offs.into_iter().map(|(file, why)| Problem::new(file, why)));
    Mounted {
        mounts,
        muzzle_reaches,
    }
}

/// Why a mount of `hardpoint` stands off the model `asset` (on the class
/// `hull`, for a kit's).
fn off(hardpoint: &str, asset: &Id, hull: Option<&Id>) -> String {
    let on = hull.map_or(String::new(), |hull| format!(" on `{hull}`"));
    format!(
        "its hardpoint `{hardpoint}` is over no surface of the model `{asset}`{on}; \
         its outfit sits on the plane"
    )
}

/// The mount of `hardpoint` on a hull `length` metres long whose model is
/// `hull`, showing its outfit as `fitted` says.
fn stand(
    hull: Option<(&Model, Front)>,
    length: f64,
    hardpoint: &Hardpoint,
    fitted: &BTreeMap<&Id, (&Id, Fitting)>,
) -> Mount {
    let standing = mount_point(hull, length, hardpoint.position);
    let shows = hardpoint
        .outfit
        .as_ref()
        .and_then(|outfit| fitted.get(outfit))
        .map(|(asset, fitting)| Shown {
            asset: asset.as_str().to_owned(),
            lift: fitting.lift,
        });
    Mount {
        hardpoint: hardpoint.id.as_str().to_owned(),
        at: standing.at,
        footing: standing.footing,
        turns: hardpoint.mount == Kind::Turret,
        shows,
    }
}

/// How each outfit in `ids` whose `model` reads sits on its mount, with
/// that model's asset id, by the outfit's id; an outfit's `model` that
/// isn't in `assets/` reported against the outfit.
fn outfit_fittings<'a, S: Source>(
    galaxy: &'a Galaxy,
    ids: &IdTable,
    models: &mut Models<'a, S>,
    problems: &mut Vec<Problem>,
) -> BTreeMap<&'a Id, (&'a Id, Fitting)> {
    let mut fitted = BTreeMap::new();
    for outfit in ids.outfits.keys() {
        let Some((outfit, doc)) = galaxy.outfits.get_key_value(outfit) else {
            continue;
        };
        let Some(model) = &doc.data.model else {
            continue;
        };
        let found = asset(
            galaxy,
            model,
            &doc.path,
            "model",
            "nothing on its hardpoints",
            problems,
        );
        if let Some((model, asset)) = found
            && let Some((read, front)) = models.read(model, asset, problems)
        {
            fitted.insert(outfit, (model, fitting(read, *front)));
        }
    }
    fitted
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::path::Path;

    use nova_galaxy_format::fixture::CC0;
    use nova_galaxy_format::source::MemSource;
    use nova_galaxy_media::fixture::glb;

    use super::*;
    use crate::new_game::tests::galaxy;
    use crate::new_game::{NewGame, load};

    const HULL_GLB: &str = "media/export/pack/hull.glb";
    const TURRET_GLB: &str = "media/export/pack/turret.glb";
    const BARGE_GLB: &str = "media/export/pack/barge.glb";

    /// The skiff's model, 2 wide, 1 tall and 4 long, facing +z.
    const HULL: [f32; 3] = [2.0, 1.0, 4.0];
    /// The turret's, 0.4 wide, 0.3 tall and 3 long, facing −z.
    const TURRET: [f32; 3] = [0.4, 0.3, 3.0];

    fn loaded(source: &MemSource) -> NewGame {
        load(source, Path::new("sample"))
    }

    /// The format's galaxy with the skiff's model in place.
    fn with_hull() -> MemSource {
        galaxy().with_bytes(HULL_GLB, &glb(HULL, [0.0; 3]))
    }

    /// `source` with a `turret` asset holding the turret's model, and the
    /// blaster showing it.
    fn with_turret(source: MemSource) -> MemSource {
        source
            .with(
                "assets/turret.md",
                &format!(
                    "---\nkind: model\nplaceholder: false\nfront: \"-z\"\n\
                     files: [{TURRET_GLB}]\n{CC0}---\n"
                ),
            )
            .with_bytes(TURRET_GLB, &glb(TURRET, [0.0; 3]))
            .with(
                "outfits/blaster.md",
                "---\nname: Blaster\nmount: fixed\ndamage: 5\nrange: 300\nreload: 0.5\n\
                 price: 100\nmodel: turret\n---\n",
            )
    }

    fn id(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    fn ship(game: &NewGame, name: &str) -> ShipId {
        ShipId(game.ids.ships[&id(name)])
    }

    fn kit(game: &NewGame, name: &str) -> OutfitId {
        OutfitId(game.ids.kits[&id(name)])
    }

    fn weapon(game: &NewGame, name: &str) -> WeaponId {
        WeaponId(game.ids.weapons[&id(name)])
    }

    fn model_of(bytes: &[u8]) -> Model {
        nova_galaxy_media::model("m.glb", bytes, |_| None).unwrap()
    }

    /// Where the core stands a hardpoint at `position` on the `size` box
    /// facing `front`, `length` metres long.
    fn cast(size: [f32; 3], front: Front, length: f64, position: [f64; 2]) -> [f64; 3] {
        let model = model_of(&glb(size, [0.0; 3]));
        mount_point(Some((&model, front)), length, position).at
    }

    fn problems(game: &NewGame) -> Vec<String> {
        game.problems.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_hull_s_own_mount_stands_where_the_core_casts_it() {
        let game = loaded(&with_hull());
        let mounts = game.data.mounts(ship(&game, "skiff"), &[]);
        assert_eq!(mounts.len(), 1);
        let nose = &mounts[0];
        assert_eq!(nose.hardpoint, "nose");
        assert_eq!(nose.footing, Footing::Surface);
        assert_eq!(nose.at, cast(HULL, Front::PlusZ, 8.0, [0.0, 3.0]));
        assert!(nose.at[2] > 0.0, "on the deck, above the middle");
        assert!(!nose.turns, "a fixed mount");
        assert_eq!(nose.shows, None, "the blaster has no model");
        assert_eq!(problems(&game), Vec::<String>::new());
    }

    #[test]
    fn a_variant_inherits_its_base_s_mounts_and_its_own_of_an_id_replaces_one() {
        let game = loaded(&with_hull());
        let armed = game.data.mounts(ship(&game, "armed-skiff"), &[]);
        assert_eq!(armed, game.data.mounts(ship(&game, "skiff"), &[]));
        let game = loaded(&with_hull().with(
            "ships/armed-skiff.md",
            "---\nname: Armed skiff\nbase: skiff\nmods: [gun-mount]\n\
             hardpoints:\n  - id: nose\n    mount: turret\n    position: [0.5, -1]\n---\n",
        ));
        let armed = game.data.mounts(ship(&game, "armed-skiff"), &[]);
        assert_eq!(armed.len(), 1, "{armed:?}");
        assert_eq!(armed[0].at, cast(HULL, Front::PlusZ, 8.0, [0.5, -1.0]));
        assert!(armed[0].turns);
    }

    /// `source` with a second shuttle hull, the barge, three times as
    /// tall as the skiff on its own model and 12 m long.
    fn with_barge(source: MemSource) -> MemSource {
        source
            .with(
                "assets/barge-hull.md",
                &format!("---\nkind: model\nplaceholder: false\nfiles: [{BARGE_GLB}]\n{CC0}---\n"),
            )
            .with_bytes(BARGE_GLB, &glb([2.0, 3.0, 4.0], [0.0; 3]))
            .with(
                "ships/barge.md",
                "---\nname: Barge\nstyle: civilian\nrole: shuttle\nmodel: barge-hull\n\
                 size: {length: 12, beam: 6, collision_radius: 6}\n---\n",
            )
    }

    #[test]
    fn a_kit_s_mount_stands_on_each_hull_it_is_fitted_to() {
        let game = loaded(&with_barge(with_hull()));
        let gun_mount = kit(&game, "gun-mount");
        let skiff = game.data.mounts(ship(&game, "skiff"), &[gun_mount]);
        let barge = game.data.mounts(ship(&game, "barge"), &[gun_mount]);
        assert_eq!(skiff.len(), 2, "the nose, then the kit's bed");
        let (on_skiff, on_barge) = (&skiff[1], &barge[0]);
        assert_eq!(on_skiff.hardpoint, "bed");
        assert_eq!(on_barge.hardpoint, "bed");
        assert_eq!(on_skiff.at, cast(HULL, Front::PlusZ, 8.0, [0.0, -1.0]));
        assert_eq!(
            on_barge.at,
            cast([2.0, 3.0, 4.0], Front::PlusZ, 12.0, [0.0, -1.0])
        );
        assert!(on_barge.at[2] > on_skiff.at[2], "the barge is taller");
        assert!(on_skiff.turns, "the bed is a turret");
        // The armed skiff carries it as built.
        let armed = game.data.mounts(ship(&game, "armed-skiff"), &[gun_mount]);
        assert_eq!(armed[1], *on_skiff);
    }

    #[test]
    fn a_placeholder_hull_s_mounts_stand_on_the_plane_with_no_problem() {
        let game = loaded(&galaxy().with(
            "assets/hull.md",
            "---\nkind: model\nplaceholder: true\n---\n",
        ));
        let gun_mount = kit(&game, "gun-mount");
        let mounts = game.data.mounts(ship(&game, "skiff"), &[gun_mount]);
        assert_eq!(
            mounts.iter().map(|m| (m.at, m.footing)).collect::<Vec<_>>(),
            [
                ([0.0, 3.0, 0.0], Footing::Plane),
                ([0.0, -1.0, 0.0], Footing::Plane)
            ]
        );
        assert_eq!(problems(&game), Vec::<String>::new());
    }

    #[test]
    fn a_hull_with_no_size_or_no_model_stands_its_mounts_on_the_plane() {
        let no_size = with_hull().with(
            "ships/skiff.md",
            "---\nname: Skiff\nstyle: civilian\nrole: shuttle\nmodel: hull\n\
             hardpoints:\n  - id: nose\n    mount: fixed\n    position: [0, 3]\n---\n",
        );
        let no_model = with_hull().with(
            "ships/skiff.md",
            "---\nname: Skiff\nstyle: civilian\nrole: shuttle\n\
             size: {length: 8, beam: 4, collision_radius: 4}\n\
             hardpoints:\n  - id: nose\n    mount: fixed\n    position: [0, 3]\n---\n",
        );
        for source in [no_size, no_model] {
            let game = loaded(&source);
            let mounts = game.data.mounts(ship(&game, "skiff"), &[]);
            assert_eq!(mounts[0].at, [0.0, 3.0, 0.0]);
            assert_eq!(mounts[0].footing, Footing::Plane);
        }
    }

    #[test]
    fn a_mount_off_its_hull_stands_on_the_plane_and_is_reported_once_against_its_file() {
        // The nose 10 m ahead of the 8 m skiff's middle; the armed skiff
        // inherits it.
        let game = loaded(&with_hull().with(
            "ships/skiff.md",
            "---\nname: Skiff\nstyle: civilian\nrole: shuttle\nmodel: hull\n\
             size: {length: 8, beam: 4, collision_radius: 4}\n\
             hardpoints:\n  - id: nose\n    mount: fixed\n    position: [0, 10]\n---\n",
        ));
        for name in ["skiff", "armed-skiff"] {
            let mounts = game.data.mounts(ship(&game, name), &[]);
            assert_eq!(mounts[0].at, [0.0, 10.0, 0.0], "{name}");
            assert_eq!(mounts[0].footing, Footing::Off, "{name}");
        }
        assert_eq!(
            problems(&game),
            [
                "ships/skiff.md: its hardpoint `nose` is over no surface of the model `hull`; \
              its outfit sits on the plane"
            ]
        );
    }

    #[test]
    fn a_kit_s_mount_off_a_hull_it_fits_is_reported_against_the_kit_naming_the_hull() {
        // The bed far aft; the kit fits shuttles, which the skiff is.
        let game = loaded(&with_hull().with(
            "mods/gun-mount.md",
            "---\nname: Gun mount\nfits: [shuttle]\nfitted_at: lawless\nprice: 100\n\
             hardpoints:\n  - id: bed\n    mount: turret\n    position: [0, -9]\n---\n",
        ));
        let mounts = game
            .data
            .mounts(ship(&game, "skiff"), &[kit(&game, "gun-mount")]);
        assert_eq!(mounts[1].footing, Footing::Off);
        assert_eq!(mounts[1].at[2], 0.0);
        let mut lines = problems(&game);
        lines.sort();
        assert_eq!(
            lines,
            [
                "mods/gun-mount.md: its hardpoint `bed` is over no surface of the model \
                 `hull` on `armed-skiff`; its outfit sits on the plane",
                "mods/gun-mount.md: its hardpoint `bed` is over no surface of the model \
                 `hull` on `skiff`; its outfit sits on the plane",
            ]
        );
    }

    #[test]
    fn a_kit_s_mount_off_a_hull_it_neither_fits_nor_is_built_with_is_not_reported() {
        let game = loaded(&with_hull().with(
            "mods/gun-mount.md",
            "---\nname: Gun mount\nfits: [freighter]\nfitted_at: lawless\nprice: 100\n\
             hardpoints:\n  - id: bed\n    mount: turret\n    position: [0, -9]\n---\n",
        ));
        let lines = problems(&game);
        assert_eq!(
            lines,
            [
                "mods/gun-mount.md: its hardpoint `bed` is over no surface of the model \
              `hull` on `armed-skiff`; its outfit sits on the plane"
            ],
            "only the armed skiff, which carries it as built"
        );
        let mounts = game
            .data
            .mounts(ship(&game, "skiff"), &[kit(&game, "gun-mount")]);
        assert_eq!(mounts[1].footing, Footing::Off, "still kept");
    }

    #[test]
    fn a_bay_mount_is_kept_for_drawing_but_fires_from_nowhere() {
        let game = loaded(&with_hull().with(
            "ships/skiff.md",
            "---\nname: Skiff\nstyle: civilian\nrole: shuttle\nmodel: hull\n\
             size: {length: 8, beam: 4, collision_radius: 4}\n\
             hardpoints:\n  - id: hangar\n    mount: bay\n    position: [0, 1]\n---\n",
        ));
        let skiff = ship(&game, "skiff");
        let mounts = game.data.mounts(skiff, &[]);
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].hardpoint, "hangar");
        assert!(!mounts[0].turns);
        assert_eq!(game.data.fittings().hulls.get(&skiff), None, "no exit");
    }

    #[test]
    fn a_mount_shows_its_outfit_s_model_lifted_by_its_fitting() {
        let game = loaded(&with_turret(with_hull()));
        let nose = &game.data.mounts(ship(&game, "skiff"), &[])[0];
        let turret = model_of(&glb(TURRET, [0.0; 3]));
        assert_eq!(
            nose.shows,
            Some(Shown {
                asset: "turret".into(),
                lift: fitting(&turret, Front::MinusZ).lift,
            })
        );
        assert_eq!(problems(&game), Vec::<String>::new());
    }

    #[test]
    fn a_mount_whose_outfit_has_no_model_that_reads_shows_nothing() {
        let placeholder = with_turret(with_hull()).with(
            "assets/turret.md",
            "---\nkind: model\nplaceholder: true\n---\n",
        );
        let pointer = with_turret(with_hull()).with_bytes(
            TURRET_GLB,
            &[
                nova_galaxy_format::media::LFS_POINTER,
                b"\noid sha256:ab\nsize 12\n",
            ]
            .concat(),
        );
        for source in [with_hull(), placeholder, pointer] {
            let game = loaded(&source);
            let nose = &game.data.mounts(ship(&game, "skiff"), &[])[0];
            assert_eq!(nose.shows, None);
            assert_eq!(problems(&game), Vec::<String>::new());
            assert_eq!(game.data.muzzle_reach(weapon(&game, "blaster")), None);
        }
    }

    #[test]
    fn an_outfit_s_model_not_in_assets_is_reported_against_the_outfit() {
        let game = loaded(&with_hull().with(
            "outfits/blaster.md",
            "---\nname: Blaster\nmount: fixed\ndamage: 5\nrange: 300\nreload: 0.5\n\
             price: 100\nmodel: gone\n---\n",
        ));
        assert_eq!(game.data.mounts(ship(&game, "skiff"), &[])[0].shows, None);
        assert_eq!(
            problems(&game),
            ["outfits/blaster.md: its model `gone` is not in assets/; \
              it draws as nothing on its hardpoints"]
        );
    }

    #[test]
    fn an_unreadable_model_a_hull_and_an_outfit_share_is_reported_once() {
        // The blaster shows the skiff's own hull asset, which doesn't decode.
        let source = galaxy().with_bytes(HULL_GLB, b"glTF nonsense").with(
            "outfits/blaster.md",
            "---\nname: Blaster\nmount: fixed\ndamage: 5\nrange: 300\nreload: 0.5\n\
             price: 100\nmodel: hull\n---\n",
        );
        let game = loaded(&source);
        let nose = &game.data.mounts(ship(&game, "skiff"), &[])[0];
        assert_eq!(nose.shows, None);
        assert_eq!(nose.footing, Footing::Plane);
        let lines = problems(&game);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with("assets/hull.md: its model doesn't read"));
    }

    #[test]
    fn an_unreadable_model_of_a_hull_with_no_stand_in_is_reported_by_its_mounts() {
        // With no size the skiff has no stand-in, so only its mounts read
        // its model.
        let source = galaxy().with_bytes(HULL_GLB, b"glTF nonsense").with(
            "ships/skiff.md",
            "---\nname: Skiff\nstyle: civilian\nrole: shuttle\nmodel: hull\n\
             hardpoints:\n  - id: nose\n    mount: fixed\n    position: [0, 3]\n---\n",
        );
        let lines = problems(&loaded(&source));
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with("assets/hull.md: its model doesn't read"));
    }

    #[test]
    fn an_unreadable_outfit_model_alone_is_reported_once() {
        let source = with_turret(with_hull()).with_bytes(TURRET_GLB, b"glTF nonsense");
        let game = loaded(&source);
        let lines = problems(&game);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with("assets/turret.md: its model doesn't read"));
    }

    #[test]
    fn a_weapon_s_muzzle_reach_is_its_fitting_s_reach_in_whole_pixels() {
        let game = loaded(&with_turret(with_hull()));
        let turret = model_of(&glb(TURRET, [0.0; 3]));
        let reach = pixels(fitting(&turret, Front::MinusZ).reach);
        assert!(reach > 0);
        assert_eq!(
            game.data.muzzle_reach(weapon(&game, "blaster")),
            Some(reach)
        );
    }

    #[test]
    fn a_reach_under_half_a_pixel_is_none_and_a_non_weapon_has_none() {
        // A model 0.6 long reaches 0.3 m: no whole pixel. A harmless
        // outfit with the turret's model is no weapon.
        let source = with_turret(with_hull())
            .with_bytes(TURRET_GLB, &glb([0.4, 0.3, 0.6], [0.0; 3]))
            .with(
                "outfits/lamp.md",
                "---\nname: Lamp\nmount: fixed\ndamage: 0\nrange: 0\nreload: 1\n\
                 price: 10\nmodel: turret\n---\n",
            );
        let game = loaded(&source);
        assert_eq!(game.data.muzzle_reach(weapon(&game, "blaster")), None);
        assert!(!game.ids.weapons.contains_key(&id("lamp")));
        assert!(game.ids.outfits.contains_key(&id("lamp")));
    }
}
