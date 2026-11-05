//! The new game's sample galaxy (game-bible's `data` folder, named by
//! `GAME_DATA`) through the folder adapter. Skips, passing, when
//! `GAME_DATA` is unset.
//!
//! Media that are Git LFS pointers are printed, not failed: a model that
//! is one draws as its hull's size box or its stellar's disc. Whether
//! game-bible's committed schemas match the format's is game-bible's own
//! test, against its pinned revision.

use std::collections::BTreeSet;

use nova_data::new_game::fleet::PX_PER_METRE;
use nova_data::new_game::{self, NewGame};
use nova_data::records::govt::Govt;
use nova_data::records::ship::Ship;
use nova_data::records::stellar::{Stellar, flags2};
use nova_data::records::system::System;
use nova_data::{Record, ShipId, StandIn, StandInOf, StellarId};
use nova_galaxy_format::id::PointRef;
use nova_galaxy_format::load::Galaxy;
use nova_galaxy_format::model::{StellarKind, Visual};
use nova_galaxy_format::source::DirSource;

use crate::common::game_data;

/// The sample, mapped and as the format loads it.
fn sample() -> Option<(NewGame, Galaxy)> {
    let dir = game_data()?;
    let game = new_game::open(&dir).expect("GAME_DATA lists");
    let (galaxy, _) = nova_galaxy_format::load::load(&DirSource::new(&dir));
    if !game.lfs_pointers.is_empty() {
        eprintln!(
            "{} media are Git LFS pointers, starting with {}",
            game.lfs_pointers.len(),
            game.lfs_pointers[0]
        );
    }
    Some((game, galaxy))
}

#[test]
fn the_sample_loads_and_maps_without_problems() {
    let Some((game, _)) = sample() else { return };
    let problems: Vec<_> = game.problems.iter().map(ToString::to_string).collect();
    assert_eq!(problems, Vec::<String>::new());
}

#[test]
fn every_system_stellar_jump_point_and_faction_has_a_record() {
    let Some((game, galaxy)) = sample() else {
        return;
    };
    assert_eq!(game.data.ids(System::TYPE).len(), galaxy.systems.len());
    let points: usize = galaxy
        .systems
        .values()
        .map(|doc| doc.data.jump_points.len())
        .sum();
    assert_eq!(
        game.data.ids(Stellar::TYPE).len(),
        galaxy.stellars.len() + points,
        "a spöb for each stellar and each jump point's gate"
    );
    assert_eq!(game.data.ids(Govt::TYPE).len(), galaxy.factions.len());
}

#[test]
fn each_system_s_nav_def_holds_exactly_its_stellars_and_gates() {
    let Some((game, galaxy)) = sample() else {
        return;
    };
    for (id, &nova) in &game.ids.systems {
        let system = game
            .data
            .get::<System>(nova)
            .expect("present")
            .expect("decoded");
        let held: BTreeSet<StellarId> = system.record.nav_def.iter().flatten().copied().collect();
        let its: BTreeSet<StellarId> = galaxy
            .stellars
            .iter()
            .filter(|(_, doc)| doc.data.system == *id)
            .map(|(stellar, _)| StellarId(game.ids.stellars[stellar]))
            .chain(
                galaxy.systems[id]
                    .data
                    .jump_points
                    .iter()
                    .map(|point| StellarId(game.ids.gates[&PointRef::new(id, &point.id)])),
            )
            .collect();
        assert_eq!(held, its, "{id}");
    }
}

#[test]
fn no_system_has_a_hyperspace_link() {
    let Some((game, _)) = sample() else { return };
    for (id, system) in game.data.records::<System>() {
        assert_eq!(system.expect("decoded").record.con, [None; 16], "sÿst {id}");
    }
}

#[test]
fn every_jump_point_has_a_gate_linked_to_its_partner_s() {
    let Some((game, galaxy)) = sample() else {
        return;
    };
    for (system, doc) in &galaxy.systems {
        for point in &doc.data.jump_points {
            let at = PointRef::new(system, &point.id);
            let spob = game
                .data
                .get::<Stellar>(game.ids.gates[&at])
                .expect("present")
                .expect("decoded")
                .record;
            assert_eq!(spob.flags2.bits(), flags2::HYPERGATE, "{at}");
            let partner = game.ids.gates.get(&point.to).copied().map(StellarId);
            assert!(
                partner.is_some(),
                "{at}: its partner {} has a gate",
                point.to
            );
            assert_eq!(spob.hyper_link[0], partner, "{at}");
            assert_eq!(spob.hyper_link[1..], [None; 7], "{at}");
        }
    }
}

#[test]
fn every_hull_has_a_ship() {
    let Some((game, galaxy)) = sample() else {
        return;
    };
    let hulls: BTreeSet<_> = galaxy
        .ships
        .iter()
        .filter(|(_, doc)| doc.data.base.is_none() && doc.data.mods.is_empty())
        .map(|(id, _)| id.clone())
        .collect();
    assert!(!hulls.is_empty(), "the sample has hulls");
    assert_eq!(
        game.ids.ships.keys().cloned().collect::<BTreeSet<_>>(),
        hulls
    );
    assert_eq!(game.data.ids(Ship::TYPE).len(), hulls.len());
}

/// The stand-in of the hull `id`.
fn ship_stand_in<'a>(game: &'a NewGame, id: &nova_galaxy_format::id::Id) -> Option<&'a StandIn> {
    let nova = *game.ids.ships.get(id)?;
    game.data.stand_in(StandInOf::Ship(ShipId(nova)))
}

/// Whether the asset `id`'s model reads from the sample's folder: one of
/// its files is a model that isn't an LFS pointer and decodes.
fn model_reads(galaxy: &Galaxy, dir: &DirSource, id: &nova_galaxy_format::id::Id) -> bool {
    galaxy
        .assets
        .get(id)
        .is_some_and(|asset| nova_galaxy_media::read_model(dir, &asset.data).is_ok())
}

#[test]
fn every_hull_with_a_size_is_its_silhouette_when_its_model_reads_and_else_its_box() {
    let Some((game, galaxy)) = sample() else {
        return;
    };
    let dir = DirSource::new(game_data().expect("set"));
    let mut silhouettes = 0;
    for (id, &nova) in &game.ids.ships {
        let ship = &galaxy.ships[id].data;
        let stand_in = game.data.stand_in(StandInOf::Ship(ShipId(nova)));
        let Some(size) = ship.size else {
            assert_eq!(stand_in, None, "{id}: no size, no stand-in");
            continue;
        };
        let Some(StandIn::Outline(outline)) = stand_in else {
            panic!("{id}: an outline, not {stand_in:?}");
        };
        let livery = ship
            .livery
            .as_ref()
            .and_then(|livery| galaxy.liveries.get(livery))
            .map(|livery| nova_galaxy_media::rgb(&livery.data.palette.primary));
        let size_box = nova_galaxy_media::size_box(size, livery.unwrap_or(nova_galaxy_media::BARE))
            .scaled(PX_PER_METRE);
        if ship
            .model
            .as_ref()
            .is_some_and(|model| model_reads(&galaxy, &dir, model))
        {
            assert!(
                outline.triangles.len() > size_box.triangles.len(),
                "{id}: its silhouette"
            );
            silhouettes += 1;
        } else {
            assert_eq!(*outline, size_box, "{id}: its size box");
        }
    }
    if game.lfs_pointers.is_empty() {
        assert!(
            silhouettes > 0,
            "with the media fetched, some hull's model reads"
        );
    }
}

#[test]
fn the_cathedral_cruiser_is_its_size_box() {
    let Some((game, galaxy)) = sample() else {
        return;
    };
    let id = nova_galaxy_format::id::Id::try_from("cathedral-cruiser".to_owned()).expect("an id");
    let ship = &galaxy.ships[&id].data;
    let primary = galaxy.liveries[ship.livery.as_ref().expect("a livery")]
        .data
        .palette
        .primary
        .clone();
    let size_box =
        nova_galaxy_media::size_box(ship.size.expect("a size"), nova_galaxy_media::rgb(&primary))
            .scaled(PX_PER_METRE);
    assert_eq!(ship_stand_in(&game, &id), Some(&StandIn::Outline(size_box)));
}

#[test]
fn every_stellar_has_a_stand_in_and_a_station_whose_model_reads_its_silhouette() {
    let Some((game, galaxy)) = sample() else {
        return;
    };
    let dir = DirSource::new(game_data().expect("set"));
    let mut outlines = 0;
    for (id, &nova) in &game.ids.stellars {
        let stellar = &galaxy.stellars[id].data;
        let stand_in = game.data.stand_in(StandInOf::Stellar(StellarId(nova)));
        let model = match &stellar.visual {
            Some(Visual::Asset(asset))
                if matches!(stellar.kind, StellarKind::Station | StellarKind::Outpost) =>
            {
                model_reads(&galaxy, &dir, asset)
            }
            _ => false,
        };
        if model {
            assert!(
                matches!(stand_in, Some(StandIn::Outline(outline)) if !outline.triangles.is_empty()),
                "{id}: {stand_in:?}"
            );
            outlines += 1;
        } else {
            assert!(
                matches!(stand_in, Some(StandIn::Disc { .. })),
                "{id}: {stand_in:?}"
            );
        }
    }
    if game.lfs_pointers.is_empty() {
        assert!(
            outlines > 0,
            "with the media fetched, the station-hub stations are outlines"
        );
    }
}

#[test]
fn every_gate_is_a_ring() {
    let Some((game, _)) = sample() else {
        return;
    };
    assert!(!game.ids.gates.is_empty());
    for (at, &nova) in &game.ids.gates {
        assert!(
            matches!(
                game.data.stand_in(StandInOf::Stellar(StellarId(nova))),
                Some(StandIn::Ring { .. })
            ),
            "{at}"
        );
    }
}
