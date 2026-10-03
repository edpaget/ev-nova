//! The galaxy catalog over the game data: a thin mapping from `GameData`'s
//! `sÿst`, `spöb`, `gövt`, `nëbu` and `PICT` resources to a [`Galaxy`].

use std::collections::BTreeMap;

use nova_data::graphics::decode_pict;
use nova_data::records::govt::Govt;
use nova_data::records::nebula::Nebula;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::{GameData, Record, StoreEntry};
use nova_rsrc::ResType;

use super::catalog::{
    Galaxy, GalaxyCatalog, GovtId, NebulaEntry, NebulaId, NebulaPicture, StellarEntry, SystemEntry,
    SystemId,
};

/// The `PICT` resource type.
const PICT: ResType = ResType::new(*b"PICT");
/// The nebulae that have pictures, by the Bible: `nëbu` 128 to 159.
const PICTURED_NEBULAE: std::ops::RangeInclusive<i16> = 128..=159;
/// The first nebula's first `PICT`. Nebula N's pictures are `PICT`
/// 9500 + (N - 128) x 7 + k, for k = 0 to 6.
const FIRST_NEBULA_PICT: i16 = 9500;
/// How many pictures a nebula can have.
const PICTURES_PER_NEBULA: i16 = 7;

/// Reads every record afresh; the map asks once, when it is built.
impl GalaxyCatalog for GameData {
    fn galaxy(&self) -> Galaxy {
        let mut problems = Vec::new();
        let mut systems = Vec::new();
        for (_, record) in self.records::<System>() {
            match record {
                Ok(entry) => systems.push(system(self, &entry, &mut problems)),
                Err(err) => problems.push(err.to_string()),
            }
        }
        let mut govt_colors = BTreeMap::new();
        for (id, record) in self.records::<Govt>() {
            match record {
                Ok(entry) => {
                    govt_colors.insert(GovtId(id), entry.record.color);
                }
                Err(err) => problems.push(err.to_string()),
            }
        }
        let mut nebulae = Vec::new();
        for (_, record) in self.records::<Nebula>() {
            match record {
                Ok(entry) => nebulae.push(nebula(self, &entry, &mut problems)),
                Err(err) => problems.push(err.to_string()),
            }
        }
        Galaxy {
            systems,
            govt_colors,
            nebulae,
            problems,
        }
    }
}

/// The system's entry, with each stellar's name looked up.
fn system(
    data: &GameData,
    entry: &StoreEntry<'_, System>,
    problems: &mut Vec<String>,
) -> SystemEntry {
    let record = entry.record;
    let stellars = record
        .nav_def
        .iter()
        .flatten()
        .map(|&id| {
            match data.get::<Stellar>(id.0) {
                Some(Ok(_)) => {}
                Some(Err(err)) => problems.push(format!("sÿst {}: {err}", entry.id)),
                None => problems.push(format!("sÿst {}: no spöb {}", entry.id, id.0)),
            }
            // An undecodable spöb still has the name in its resource map.
            let name = data
                .resource(Stellar::TYPE, id.0)
                .and_then(|stellar| stellar.resource.name());
            StellarEntry {
                id,
                name: display_name(name, "spöb", id.0),
            }
        })
        .collect();
    SystemEntry {
        id: SystemId(entry.id),
        name: display_name(entry.name, "sÿst", entry.id),
        x: record.x_pos,
        y: record.y_pos,
        links: record.con.iter().flatten().copied().collect(),
        govt: record.govt,
        stellars,
    }
}

/// The nebula's entry, with every picture that exists for it.
fn nebula(
    data: &GameData,
    entry: &StoreEntry<'_, Nebula>,
    problems: &mut Vec<String>,
) -> NebulaEntry {
    let record = entry.record;
    let mut pictures = Vec::new();
    if PICTURED_NEBULAE.contains(&entry.id) {
        // In range, the IDs run from 9500 to 9723, well inside an i16.
        let first = FIRST_NEBULA_PICT + (entry.id - PICTURED_NEBULAE.start()) * PICTURES_PER_NEBULA;
        for id in first..first + PICTURES_PER_NEBULA {
            let Some(resource) = data.resource(PICT, id) else {
                continue;
            };
            match decode_pict(resource.resource.data()) {
                Ok(image) => pictures.push(NebulaPicture {
                    id,
                    width: image.width(),
                    height: image.height(),
                }),
                Err(err) => problems.push(format!("nëbu {}: PICT {id}: {err}", entry.id)),
            }
        }
    }
    NebulaEntry {
        id: NebulaId(entry.id),
        name: display_name(entry.name, "nëbu", entry.id),
        left: record.x_pos,
        top: record.y_pos,
        width: record.x_size,
        height: record.y_size,
        pictures,
    }
}

/// A resource's name up to any designer's note after a `;`, trimmed; if
/// that leaves nothing, its type and ID.
fn display_name(name: Option<&str>, kind: &str, id: i16) -> String {
    let name = name.unwrap_or_default();
    let shown = name.split(';').next().unwrap_or_default().trim();
    if shown.is_empty() {
        format!("{kind} {id}")
    } else {
        shown.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io;
    use std::path::Path;

    use nova_data::graphics::fixture::{DirectBits, PictBuilder};
    use nova_data::records::govt::Govt;
    use nova_data::records::nebula::Nebula;
    use nova_data::records::stellar::Stellar;
    use nova_data::records::system::System;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_data::{GameData, Record};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};

    use super::*;
    use crate::galaxy::catalog::{
        GovtId, NebulaEntry, NebulaId, NebulaPicture, StellarEntry, StellarId, SystemEntry,
        SystemId,
    };

    const PICT: ResType = ResType::new(*b"PICT");

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

    /// A resource: type, ID, name and bytes.
    type Res = (ResType, i16, Option<&'static str>, Vec<u8>);

    fn store(resources: &[Res]) -> GameData {
        let fork = resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, name, data)| {
                fork.resource(*ty, *id, name.map(str::as_bytes), data)
            })
            .build()
            .bytes;
        let file = OneFile(fork);
        GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
    }

    fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
        for (i, value) in values.iter().enumerate() {
            let start = at + 2 * i;
            bytes[start..start + 2].copy_from_slice(&value.to_be_bytes());
        }
    }

    /// A `sÿst` at (`x`, `y`) owned by `govt` (-1 for none), with these
    /// hyperlinks and stellars; every other link and stellar slot is -1.
    fn system(x: i16, y: i16, govt: i16, links: &[i16], stellars: &[i16]) -> Vec<u8> {
        let mut bytes = vec![0; System::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[x, y]);
        put_i16s(&mut bytes, 0x04, &[-1; 16]);
        put_i16s(&mut bytes, 0x04, links);
        put_i16s(&mut bytes, 0x24, &[-1; 16]);
        put_i16s(&mut bytes, 0x24, stellars);
        put_i16s(&mut bytes, 0x66, &[govt]);
        bytes
    }

    /// A `gövt` whose map colour is `color`.
    fn govt(color: u32) -> Vec<u8> {
        let mut bytes = vec![0; Govt::SIZE.expect("fixed")];
        bytes[0xA4..0xA8].copy_from_slice(&color.to_be_bytes());
        bytes
    }

    fn stellar() -> Vec<u8> {
        vec![0; Stellar::SIZE.expect("fixed")]
    }

    /// A `nëbu` covering `width` x `height` from (`left`, `top`).
    fn nebula(left: i16, top: i16, width: i16, height: i16) -> Vec<u8> {
        let mut bytes = vec![0; Nebula::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0, &[left, top, width, height]);
        bytes
    }

    /// A grey `width` x `height` `PICT`.
    fn picture(width: i16, height: i16) -> Vec<u8> {
        let bounds = [0, 0, height, width];
        let pixels = vec![0x4210; (width * height) as usize];
        PictBuilder::new(bounds)
            .direct_bits(&DirectBits::rgb555(bounds, &pixels))
            .end()
            .build()
    }

    fn system_entry(id: i16, name: &str) -> SystemEntry {
        SystemEntry {
            id: SystemId(id),
            name: name.to_owned(),
            x: 0,
            y: 0,
            links: Vec::new(),
            govt: None,
            stellars: Vec::new(),
        }
    }

    #[test]
    fn a_full_system_has_every_field_and_systems_come_in_id_order() {
        let data = store(&[
            (
                System::TYPE,
                129,
                Some("Sol"),
                system(-358, 300, -1, &[], &[]),
            ),
            (
                System::TYPE,
                128,
                Some("Kania"),
                system(160, -20, 130, &[129, 129, 128, 4000], &[137, 1404]),
            ),
            (Stellar::TYPE, 137, Some("Port Kane"), stellar()),
            (Stellar::TYPE, 1404, Some("HG-Kania"), stellar()),
            (Govt::TYPE, 130, Some("Federation"), govt(0x002C_2CAF)),
        ]);
        let galaxy = data.galaxy();
        assert_eq!(galaxy.problems, Vec::<String>::new());
        assert_eq!(
            galaxy.systems,
            [
                SystemEntry {
                    id: SystemId(128),
                    name: "Kania".to_owned(),
                    x: 160,
                    y: -20,
                    links: vec![SystemId(129), SystemId(129), SystemId(128), SystemId(4000)],
                    govt: Some(GovtId(130)),
                    stellars: vec![
                        StellarEntry {
                            id: StellarId(137),
                            name: "Port Kane".to_owned()
                        },
                        StellarEntry {
                            id: StellarId(1404),
                            name: "HG-Kania".to_owned()
                        },
                    ],
                },
                SystemEntry {
                    x: -358,
                    y: 300,
                    ..system_entry(129, "Sol")
                },
            ]
        );
    }

    #[test]
    fn links_and_stellars_skip_empty_slots_anywhere_in_the_record() {
        let mut bytes = system(0, 0, -1, &[], &[]);
        put_i16s(&mut bytes, 0x04 + 2 * 15, &[130]);
        put_i16s(&mut bytes, 0x24 + 2 * 3, &[140]);
        let data = store(&[
            (System::TYPE, 128, Some("Edge"), bytes),
            (Stellar::TYPE, 140, Some("Far"), stellar()),
        ]);
        let galaxy = data.galaxy();
        assert_eq!(galaxy.systems[0].links, [SystemId(130)]);
        assert_eq!(
            galaxy.systems[0].stellars,
            [StellarEntry {
                id: StellarId(140),
                name: "Far".to_owned()
            }]
        );
    }

    #[test]
    fn a_designer_note_after_a_semicolon_is_not_part_of_the_name() {
        let data = store(&[
            (
                System::TYPE,
                128,
                Some("Koria;Rebs !assim"),
                system(0, 0, -1, &[], &[140]),
            ),
            (
                System::TYPE,
                129,
                Some("  Koria ; Rebs assim"),
                system(0, 0, -1, &[], &[]),
            ),
            (
                System::TYPE,
                130,
                Some(" ;note"),
                system(0, 0, -1, &[], &[]),
            ),
            (System::TYPE, 131, None, system(0, 0, -1, &[], &[])),
            (Stellar::TYPE, 140, Some("Koria Station;x"), stellar()),
        ]);
        let galaxy = data.galaxy();
        let names: Vec<&str> = galaxy.systems.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Koria", "Koria", "sÿst 130", "sÿst 131"]);
        assert_eq!(galaxy.systems[0].stellars[0].name, "Koria Station");
    }

    #[test]
    fn a_missing_or_unnamed_stellar_is_named_by_id_and_a_missing_one_is_a_problem() {
        let mut short = stellar();
        short.pop();
        let data = store(&[
            (
                System::TYPE,
                128,
                Some("Here"),
                system(0, 0, -1, &[], &[150, 151, 152]),
            ),
            (Stellar::TYPE, 151, None, stellar()),
            (Stellar::TYPE, 152, Some("Broken"), short),
        ]);
        let galaxy = data.galaxy();
        let names: Vec<&str> = galaxy.systems[0]
            .stellars
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(names, ["spöb 150", "spöb 151", "Broken"]);
        assert_eq!(galaxy.problems.len(), 2, "{:?}", galaxy.problems);
        assert_eq!(galaxy.problems[0], "sÿst 128: no spöb 150");
        assert!(
            galaxy.problems[1].starts_with("sÿst 128: spöb 152"),
            "{:?}",
            galaxy.problems
        );
    }

    #[test]
    fn an_undecodable_system_is_left_out_with_a_problem() {
        let mut short = system(0, 0, -1, &[], &[]);
        short.pop();
        let data = store(&[
            (System::TYPE, 128, Some("Short"), short),
            (System::TYPE, 129, Some("Fine"), system(0, 0, -1, &[], &[])),
        ]);
        let galaxy = data.galaxy();
        assert_eq!(galaxy.systems, [system_entry(129, "Fine")]);
        assert_eq!(galaxy.problems.len(), 1);
        assert!(
            galaxy.problems[0].starts_with("sÿst 128 \"Short\""),
            "{:?}",
            galaxy.problems
        );
    }

    #[test]
    fn every_government_that_decodes_has_its_colour() {
        let mut short = govt(0);
        short.pop();
        let data = store(&[
            (Govt::TYPE, 128, Some("Federation"), govt(0x002C_2CAF)),
            (Govt::TYPE, 129, Some("Broken"), short),
            (Govt::TYPE, 142, Some("Rimerta"), govt(0)),
        ]);
        let galaxy = data.galaxy();
        assert_eq!(
            galaxy.govt_colors,
            BTreeMap::from([(GovtId(128), 0x002C_2CAF), (GovtId(142), 0)])
        );
        assert_eq!(galaxy.problems.len(), 1);
        assert!(
            galaxy.problems[0].starts_with("gövt 129 \"Broken\""),
            "{:?}",
            galaxy.problems
        );
    }

    #[test]
    fn a_nebula_has_its_rectangle_and_the_pictures_that_exist() {
        let data = store(&[
            (
                Nebula::TYPE,
                128,
                Some("Holpa Nebula"),
                nebula(-20, -30, 60, 50),
            ),
            (PICT, 9500, None, picture(15, 12)),
            (PICT, 9502, None, picture(60, 50)),
            (PICT, 9506, None, picture(120, 100)),
            // Nebula 129's first picture, and the one before nebula 128's.
            (PICT, 9507, None, picture(2, 2)),
            (PICT, 9499, None, picture(2, 2)),
        ]);
        let galaxy = data.galaxy();
        assert_eq!(galaxy.problems, Vec::<String>::new());
        assert_eq!(
            galaxy.nebulae,
            [NebulaEntry {
                id: NebulaId(128),
                name: "Holpa Nebula".to_owned(),
                left: -20,
                top: -30,
                width: 60,
                height: 50,
                pictures: vec![
                    NebulaPicture {
                        id: 9500,
                        width: 15,
                        height: 12
                    },
                    NebulaPicture {
                        id: 9502,
                        width: 60,
                        height: 50
                    },
                    NebulaPicture {
                        id: 9506,
                        width: 120,
                        height: 100
                    },
                ],
            }]
        );
    }

    #[test]
    fn each_nebulas_pictures_start_seven_after_the_last_ones() {
        let data = store(&[
            (Nebula::TYPE, 129, None, nebula(0, 0, 10, 10)),
            (Nebula::TYPE, 159, None, nebula(0, 0, 10, 10)),
            (PICT, 9506, None, picture(2, 2)),
            (PICT, 9507, None, picture(3, 3)),
            (PICT, 9513, None, picture(4, 4)),
            (PICT, 9514, None, picture(2, 2)),
            (PICT, 9717, None, picture(5, 5)),
            (PICT, 9723, None, picture(6, 6)),
        ]);
        let galaxy = data.galaxy();
        let pictures: Vec<(&str, Vec<i16>)> = galaxy
            .nebulae
            .iter()
            .map(|n| (n.name.as_str(), n.pictures.iter().map(|p| p.id).collect()))
            .collect();
        assert_eq!(
            pictures,
            [
                ("nëbu 129", vec![9507, 9513]),
                ("nëbu 159", vec![9717, 9723])
            ]
        );
    }

    #[test]
    fn a_nebula_outside_the_pictured_range_has_no_pictures() {
        let data = store(&[
            (Nebula::TYPE, 127, None, nebula(0, 0, 10, 10)),
            (Nebula::TYPE, 160, None, nebula(0, 0, 10, 10)),
            (Nebula::TYPE, i16::MIN, None, nebula(0, 0, 10, 10)),
            (Nebula::TYPE, i16::MAX, None, nebula(0, 0, 10, 10)),
            // Where nebulae 127 and 160 would find pictures by the formula.
            (PICT, 9493, None, picture(2, 2)),
            (PICT, 9499, None, picture(2, 2)),
            (PICT, 9724, None, picture(2, 2)),
            (PICT, 9730, None, picture(2, 2)),
        ]);
        let galaxy = data.galaxy();
        assert_eq!(galaxy.nebulae.len(), 4);
        for nebula in &galaxy.nebulae {
            assert_eq!(nebula.pictures, [], "{nebula:?}");
        }
        assert_eq!(galaxy.problems, Vec::<String>::new());
    }

    #[test]
    fn an_undecodable_nebula_picture_is_skipped_with_a_problem() {
        let mut short = nebula(0, 0, 10, 10);
        short.pop();
        let data = store(&[
            (Nebula::TYPE, 128, None, nebula(0, 0, 10, 10)),
            (Nebula::TYPE, 129, Some("Short"), short),
            (PICT, 9500, None, b"not a picture".to_vec()),
            (PICT, 9501, None, picture(4, 4)),
        ]);
        let galaxy = data.galaxy();
        let ids: Vec<i16> = galaxy.nebulae[0].pictures.iter().map(|p| p.id).collect();
        assert_eq!(ids, [9501]);
        assert_eq!(galaxy.nebulae.len(), 1);
        assert_eq!(galaxy.problems.len(), 2, "{:?}", galaxy.problems);
        assert!(
            galaxy.problems[0].starts_with("nëbu 128: PICT 9500: "),
            "{:?}",
            galaxy.problems
        );
        assert!(
            galaxy.problems[1].starts_with("nëbu 129 \"Short\""),
            "{:?}",
            galaxy.problems
        );
    }

    #[test]
    fn no_game_data_is_an_empty_galaxy() {
        assert_eq!(store(&[]).galaxy(), Galaxy::default());
    }
}
