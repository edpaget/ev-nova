//! The new game's sample galaxy (game-bible's `data` folder, named by
//! `GAME_DATA`) through the folder adapter. Skips, passing, when
//! `GAME_DATA` is unset.
//!
//! Media that are Git LFS pointers are printed, not failed: nothing reads
//! media yet. Whether game-bible's committed schemas match the format's is
//! game-bible's own test, against its pinned revision.

use std::collections::BTreeSet;

use nova_data::new_game::{self, NewGame};
use nova_data::records::govt::Govt;
use nova_data::records::ship::Ship;
use nova_data::records::stellar::{Stellar, flags2};
use nova_data::records::system::System;
use nova_data::{Record, StellarId};
use nova_galaxy_format::id::PointRef;
use nova_galaxy_format::load::Galaxy;
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
