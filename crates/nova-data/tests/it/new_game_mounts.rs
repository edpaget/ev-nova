//! The sample's hardpoint mounts and muzzle reach, loaded: over the
//! modelled sample (`new_game::fixture::modelled_sample`) the gun mount's
//! `gun-bed` stands on the light freighter's deck at the height the core
//! casts and shows the mass driver's turret; over the checked-in sample,
//! whose light freighter is a placeholder and whose mass driver has no
//! model, it stands on the plane and the mass driver has no reach.

// Each expected point is the core's own, not retyped.
#![allow(clippy::float_cmp)]

use std::path::Path;

use nova_data::new_game::fixture::{
    LIGHT_FREIGHTER_GLB, TURRET_MOUNT_GLB, modelled_sample, sample_folder,
};
use nova_data::new_game::{NewGame, load, open};
use nova_data::store::{Footing, Shown};
use nova_data::wire::id::{OutfitId, ShipId, WeaponId};
use nova_galaxy_format::asset::Front;
use nova_galaxy_format::id::Id;
use nova_galaxy_format::source::{MemSource, Source};
use nova_galaxy_media::Model;
use nova_galaxy_media::mount::{fitting, mount_point};

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

/// The model the modelled sample's file at `path` holds.
fn model_at(source: &MemSource, path: &str) -> Model {
    let bytes = source
        .bytes(path)
        .expect("a memory read")
        .expect("the model's file");
    nova_galaxy_media::model(path, &bytes, |_| None).expect("the model reads")
}

#[test]
fn the_gun_bed_stands_on_the_light_freighter_s_deck_and_shows_the_turret() {
    let source = modelled_sample();
    let game = load(&source, Path::new("sample"));
    assert_eq!(
        game.problems
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        Vec::<String>::new()
    );
    let hull = model_at(&source, LIGHT_FREIGHTER_GLB);
    let turret = model_at(&source, TURRET_MOUNT_GLB);
    let deck = mount_point(Some((&hull, Front::MinusZ)), 40.0, [0.0, -2.0]);
    assert_eq!(deck.footing, Footing::Surface);
    assert!(deck.at[2] > 0.0, "the deck is above the hull's middle");
    let gun_mount = kit(&game, "gun-mount");
    // The Harrow gunship as built, and a light freighter carrying the kit.
    for name in ["harrow-gunship", "light-freighter"] {
        let mounts = game.data.mounts(ship(&game, name), &[gun_mount]);
        assert_eq!(mounts.len(), 1, "{name}: {mounts:?}");
        let bed = &mounts[0];
        assert_eq!(bed.hardpoint, "gun-bed", "{name}");
        assert_eq!(bed.at, deck.at, "{name}");
        assert_eq!(bed.footing, Footing::Surface, "{name}");
        assert!(bed.turns, "{name}");
        assert_eq!(
            bed.shows,
            Some(Shown {
                asset: "turret-mount".into(),
                lift: fitting(&turret, Front::MinusZ).lift,
            }),
            "{name}"
        );
    }
    let reach = game.data.muzzle_reach(weapon(&game, "mass-driver"));
    assert!(reach.is_some_and(|px| px > 0), "{reach:?}");
}

#[test]
fn over_the_checked_in_sample_the_gun_bed_is_on_the_plane_and_has_no_reach() {
    let game = open(&sample_folder()).expect("the sample opens");
    let mounts = game
        .data
        .mounts(ship(&game, "harrow-gunship"), &[kit(&game, "gun-mount")]);
    assert_eq!(mounts.len(), 1);
    assert_eq!(mounts[0].at, [0.0, -2.0, 0.0]);
    assert_eq!(mounts[0].footing, Footing::Plane);
    assert_eq!(mounts[0].shows, None, "the mass driver has no model");
    assert_eq!(game.data.muzzle_reach(weapon(&game, "mass-driver")), None);
}
