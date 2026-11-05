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
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::{Record, StellarId};
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
fn every_system_stellar_and_faction_has_a_record() {
    let Some((game, galaxy)) = sample() else {
        return;
    };
    assert_eq!(game.data.ids(System::TYPE).len(), galaxy.systems.len());
    assert_eq!(game.data.ids(Stellar::TYPE).len(), galaxy.stellars.len());
    assert_eq!(game.data.ids(Govt::TYPE).len(), galaxy.factions.len());
}

#[test]
fn each_system_s_nav_def_holds_exactly_its_stellars() {
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
