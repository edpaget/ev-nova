//! The galaxy map's hyperlinks and the simulation's star map are built
//! from the same `sÿst` `Con` links by two owners, `nova_view`'s
//! `GalaxyModel` and `nova_sim`'s `StarMap`, each normalising them on its
//! own. Over the same synthetic game data, both keep the same undirected
//! links: none to itself, none to a system that is missing or does not
//! decode, each once whichever of its systems lists it, however often.

use std::collections::BTreeSet;
use std::io;
use std::path::Path;

use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_sim::{PilotCatalog, StarMap};
use nova_view::galaxy::{GalaxyCatalog, GalaxyModel, SystemId};

/// One data file, `/data/Nova Data`, holding a fork.
struct OneFile(Vec<u8>);

impl DirLister for OneFile {
    fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
        Ok(vec![Listing {
            name: "Nova Data".into(),
            kind: EntryKind::File,
        }])
    }
}

impl ForkReader for OneFile {
    fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        Ok((fork == Fork::Data).then(|| self.0.clone()))
    }
}

fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
    }
}

/// An independent `sÿst` at map (`x`, 0) with these `Con` links, every
/// other slot -1, and no stellars.
fn system(x: i16, links: &[i16]) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[x, 0]);
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x04, links);
    put_i16s(&mut bytes, 0x66, &[-1]);
    bytes
}

/// Systems 128 to 134, with links listed only by the higher ID
/// (129-128), by both (129-130), repeated (130-131), to itself (131), to a
/// missing system (999) and to one that does not decode (135); 132-133
/// listed by both, twice each; 134 linked to nothing.
fn data() -> GameData {
    let mut short = system(0, &[128]);
    short.pop();
    let fork = [
        (128, system(0, &[999])),
        (129, system(10, &[128, 130])),
        (130, system(20, &[129, 131, 131, 135])),
        (131, system(30, &[131, 130])),
        (132, system(40, &[133, 133])),
        (133, system(50, &[132, 132, 999])),
        (134, system(60, &[])),
        (135, short),
    ]
    .iter()
    .fold(ForkBuilder::new(), |fork, (id, bytes)| {
        fork.resource(System::TYPE, *id, None, bytes)
    })
    .build()
    .bytes;
    let file = OneFile(fork);
    GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
}

/// The star map's links, as (lower ID, higher ID): each pair of systems
/// on it one jump apart.
fn star_map_links(map: &StarMap, systems: &[SystemId]) -> BTreeSet<(SystemId, SystemId)> {
    let mut links = BTreeSet::new();
    for &from in systems {
        for &to in systems {
            if from < to && map.route(from, to) == Ok(vec![to]) {
                links.insert((from, to));
            }
        }
    }
    links
}

#[test]
fn the_galaxy_map_and_the_star_map_keep_the_same_hyperlinks() {
    let data = data();
    let model = GalaxyModel::new(data.galaxy());
    let map = StarMap::new(data.star_map());
    let systems: Vec<SystemId> = model.systems().iter().map(|s| s.entry.id).collect();
    assert_eq!(systems, (128..=134).map(SystemId).collect::<Vec<_>>());
    let drawn: BTreeSet<_> = model.links().iter().copied().collect();
    assert_eq!(star_map_links(&map, &systems), drawn);
    let pair = |a, b| (SystemId(a), SystemId(b));
    assert_eq!(
        drawn,
        BTreeSet::from([
            pair(128, 129),
            pair(129, 130),
            pair(130, 131),
            pair(132, 133)
        ])
    );
}
