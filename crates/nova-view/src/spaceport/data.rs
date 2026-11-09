//! The spaceport's ports over the game data: a thin mapping from
//! `GameData`'s `spöb` records and stellar stand-ins (the new game's
//! only, through [`outline_data`]), `PICT`s, each ship class's looks
//! (its `shän` base image, its `shïp` movie file and its stand-in) and
//! `cölr` 128's list and grid colours.

use std::collections::{BTreeMap, BTreeSet};

use nova_data::graphics::PICT;
use nova_data::records::colors::Colors;
use nova_data::records::ship::Ship;
use nova_data::records::stellar::Stellar;
use nova_data::{GameData, StandInOf};
use nova_sim::ShipId;

use super::catalog::{PortRecord, SpaceportCatalog, StellarId};
use super::grid::{GridColors, GridLooks};
use super::shipyard::{ShipLook, ShipLooks};
use super::trade::{ListColors, ListLooks};
use crate::color::Color;
use crate::outline_data;
use crate::target_picture::game_base_images;

/// The `cölr` the list and grid colours come from.
const COLORS_ID: i16 = 128;

/// Decodes on every call; the shops' screens ask once, as each opens.
impl GridLooks for GameData {
    fn grid_colors(&self) -> GridColors {
        let Some(Ok(colors)) = self.get::<Colors>(COLORS_ID) else {
            return GridColors::STOCK;
        };
        GridColors {
            dim: Color::from_rgb24(colors.record.grid_dim),
            bright: Color::from_rgb24(colors.record.grid_bright),
        }
    }
}

/// Decodes on every call; the trade center's screen asks once, as the
/// spaceport opens.
impl ListLooks for GameData {
    fn list_colors(&self) -> ListColors {
        let Some(Ok(colors)) = self.get::<Colors>(COLORS_ID) else {
            return ListColors::STOCK;
        };
        ListColors {
            text: Color::from_rgb24(colors.record.list_text),
            background: Color::from_rgb24(colors.record.list_bkgnd),
            hilite: Color::from_rgb24(colors.record.list_hilite),
        }
    }
}

/// Reads afresh on every call; the spaceport asks once, when it opens.
impl SpaceportCatalog for GameData {
    fn stellar_port(&self, id: StellarId) -> Result<PortRecord, String> {
        let entry = match self.get::<Stellar>(id.0) {
            None => return Err(format!("no spöb {}", id.0)),
            Some(Err(err)) => return Err(err.to_string()),
            Some(Ok(entry)) => entry,
        };
        let stellar = entry.record;
        Ok(PortRecord {
            name: entry.name.unwrap_or_default().to_owned(),
            flags: stellar.flags.bits(),
            cust_pic_id: stellar.cust_pic_id,
            graphic_type: stellar.graphic_type,
            cust_snd_id: stellar.cust_snd_id,
            flags2: stellar.flags2.bits(),
            stand_in: self
                .stand_in(StandInOf::Stellar(nova_data::StellarId(id.0)))
                .map(outline_data::stellar_stand_in),
        })
    }

    fn picture_exists(&self, id: i16) -> bool {
        self.resource(PICT, id).is_some()
    }
}

/// Reads every `shïp` and `shän` afresh; the shipyard and the hire screen
/// ask once, as each opens.
impl ShipLooks for GameData {
    fn ship_looks(&self) -> Vec<ShipLook> {
        let bases: BTreeMap<ShipId, i16> = game_base_images(self).into_iter().collect();
        let movies: BTreeMap<ShipId, Option<String>> = self
            .records::<Ship>()
            .filter_map(|(id, ship)| {
                let file = ship.ok()?.record.movie_file.as_str().to_owned();
                Some((ShipId(id), (!file.is_empty()).then_some(file)))
            })
            .collect();
        let ships: BTreeSet<ShipId> = bases.keys().chain(movies.keys()).copied().collect();
        ships
            .into_iter()
            .map(|ship| ShipLook {
                ship,
                base_image: bases.get(&ship).copied(),
                movie_file: movies.get(&ship).cloned().flatten(),
                stand_in: self
                    .stand_in(StandInOf::Ship(nova_data::ShipId(ship.0)))
                    .and_then(outline_data::ship_outline),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use nova_data::Record;
    use nova_data::StandIn;
    use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
    use nova_data::store::fixture::one_file;

    use nova_rsrc::fixture::ForkBuilder;

    use nova_data::records::ship_anim::ShipAnim;

    use super::*;
    use crate::outline::StellarStandIn;
    use crate::spaceport::shipyard::{base_images, movie_files};

    /// A `spöb` of graphic type `graphic_type`, with these `Flags`,
    /// `CustPicID`, `CustSndID` and `Flags2`.
    fn stellar(
        graphic_type: i16,
        flags: u32,
        cust_pic_id: i16,
        cust_snd_id: i16,
        flags2: u16,
    ) -> Vec<u8> {
        let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
        bytes[0x04..0x06].copy_from_slice(&graphic_type.to_be_bytes());
        bytes[0x06..0x0A].copy_from_slice(&flags.to_be_bytes());
        bytes[0x18..0x1A].copy_from_slice(&cust_pic_id.to_be_bytes());
        bytes[0x1A..0x1C].copy_from_slice(&cust_snd_id.to_be_bytes());
        bytes[0x20..0x22].copy_from_slice(&flags2.to_be_bytes());
        bytes
    }

    /// Earth (128), a nameless stellar (129), a short one (130), and
    /// `PICT` 10003, which need not decode.
    fn data() -> GameData {
        let mut short = stellar(0, 0, 0, 0, 0);
        short.pop();
        let fork = ForkBuilder::new()
            .resource(
                Stellar::TYPE,
                128,
                Some(b"Earth"),
                &stellar(3, 0x43, 9000, 10_032, 0x0011),
            )
            .resource(Stellar::TYPE, 129, None, &stellar(255, 0x21, -1, -1, 0))
            .resource(Stellar::TYPE, 130, Some(b"Short"), &short)
            .resource(PICT, 10_003, None, &[0; 4])
            .build()
            .bytes;
        one_file(fork).load()
    }

    #[test]
    fn a_stellars_port_is_its_name_flags_pictures_and_sound() {
        let data = data();
        assert_eq!(
            data.stellar_port(StellarId(128)),
            Ok(PortRecord {
                name: "Earth".to_owned(),
                flags: 0x43,
                cust_pic_id: 9000,
                graphic_type: 3,
                cust_snd_id: 10_032,
                flags2: 0x0011,
                stand_in: None,
            })
        );
        assert_eq!(
            data.stellar_port(StellarId(129)),
            Ok(PortRecord {
                name: String::new(),
                flags: 0x21,
                cust_pic_id: -1,
                graphic_type: 255,
                cust_snd_id: -1,
                flags2: 0,
                stand_in: None,
            })
        );
    }

    #[test]
    fn a_stellars_port_carries_its_stand_in_in_the_views_terms() {
        let outline = nova_data::Outline {
            triangles: vec![nova_data::OutlineTriangle {
                corners: [[0.0, 2.0], [1.0, -1.0], [-1.0, -1.0]],
                color: [10, 20, 30],
            }],
        };
        let data = data().with_stand_ins(BTreeMap::from([
            (
                StandInOf::Stellar(nova_data::StellarId(128)),
                StandIn::Disc { color: [1, 2, 3] },
            ),
            (
                StandInOf::Stellar(nova_data::StellarId(129)),
                StandIn::Outline(outline.clone()),
            ),
            (
                StandInOf::Ship(nova_data::ShipId(129)),
                StandIn::Disc { color: [9; 3] },
            ),
        ]));
        let stand_in = |id| data.stellar_port(StellarId(id)).expect("a record").stand_in;
        assert_eq!(
            stand_in(128),
            Some(StellarStandIn::Disc(Color::rgba(1, 2, 3, 255)))
        );
        assert_eq!(
            stand_in(129),
            Some(outline_data::stellar_stand_in(&StandIn::Outline(outline)))
        );
    }

    #[test]
    fn a_stock_stellars_port_has_no_stand_in() {
        assert_eq!(
            data().stellar_port(StellarId(128)).expect("Earth").stand_in,
            None
        );
    }

    #[test]
    fn a_missing_or_undecodable_stellar_says_why() {
        let data = data();
        assert_eq!(
            data.stellar_port(StellarId(140)),
            Err("no spöb 140".to_owned())
        );
        let Err(reason) = data.stellar_port(StellarId(130)) else {
            panic!("an error")
        };
        assert!(reason.contains("130"), "{reason}");
    }

    /// The data holding these resources, by type, ID and bytes.
    fn holding(resources: &[(nova_rsrc::ResType, i16, Vec<u8>)]) -> GameData {
        let mut fork = ForkBuilder::new();
        for (kind, id, bytes) in resources {
            fork = fork.resource(*kind, *id, None, bytes);
        }
        one_file(fork.build().bytes).load()
    }

    #[test]
    fn list_colors_come_from_colr_128_or_the_stock() {
        let mut bytes = vec![0; Colors::SIZE.expect("fixed")];
        bytes[0x8E..0x92].copy_from_slice(&0x0011_2233_u32.to_be_bytes());
        bytes[0x92..0x96].copy_from_slice(&0x0044_5566_u32.to_be_bytes());
        bytes[0x96..0x9A].copy_from_slice(&0x0077_8899_u32.to_be_bytes());
        assert_eq!(
            holding(&[(Colors::TYPE, 128, bytes.clone())]).list_colors(),
            ListColors {
                text: Color::rgba(0x11, 0x22, 0x33, 255),
                background: Color::rgba(0x44, 0x55, 0x66, 255),
                hilite: Color::rgba(0x77, 0x88, 0x99, 255),
            }
        );
        assert_eq!(holding(&[]).list_colors(), ListColors::STOCK);
        assert_eq!(
            holding(&[(Colors::TYPE, 129, bytes)]).list_colors(),
            ListColors::STOCK,
            "only 128"
        );
        assert_eq!(
            holding(&[(Colors::TYPE, 128, vec![0; 3])]).list_colors(),
            ListColors::STOCK,
            "undecodable"
        );
    }

    #[test]
    fn grid_colors_come_from_colr_128_or_the_stock() {
        let mut bytes = vec![0; Colors::SIZE.expect("fixed")];
        bytes[0x56..0x5A].copy_from_slice(&0x00AB_CDEF_u32.to_be_bytes());
        bytes[0x5A..0x5E].copy_from_slice(&0x0012_3456_u32.to_be_bytes());
        assert_eq!(
            holding(&[(Colors::TYPE, 128, bytes.clone())]).grid_colors(),
            GridColors {
                bright: Color::rgba(0xAB, 0xCD, 0xEF, 255),
                dim: Color::rgba(0x12, 0x34, 0x56, 255),
            }
        );
        assert_eq!(holding(&[]).grid_colors(), GridColors::STOCK);
        assert_eq!(
            holding(&[(Colors::TYPE, 129, bytes)]).grid_colors(),
            GridColors::STOCK,
            "only 128"
        );
        assert_eq!(
            holding(&[(Colors::TYPE, 128, vec![0; 3])]).grid_colors(),
            GridColors::STOCK,
            "undecodable"
        );
        assert_eq!(
            GridColors::STOCK,
            GridColors {
                bright: Color::rgba(0xFF, 0, 0, 255),
                dim: Color::rgba(0x40, 0x40, 0x40, 255),
            },
            "stock cölr 128's"
        );
    }

    #[test]
    fn a_picture_exists_when_the_data_has_it() {
        let data = data();
        assert!(data.picture_exists(10_003));
        assert!(!data.picture_exists(10_000));
    }

    /// A `shän` whose base image is `rlëD` `base`.
    fn ship_anim(base: i16) -> Vec<u8> {
        ShipAnimBuilder::new().base_image_id(base).build()
    }

    #[test]
    fn the_ships_base_images_are_each_readable_shäns_by_id() {
        let mut short = ship_anim(1002);
        short.pop();
        let fork = ForkBuilder::new()
            .resource(ShipAnim::TYPE, 361, None, &ship_anim(1000))
            .resource(ShipAnim::TYPE, 128, None, &ship_anim(1000))
            .resource(ShipAnim::TYPE, 129, Some(b"Heavy"), &ship_anim(1001))
            .resource(ShipAnim::TYPE, 130, None, &short)
            .build()
            .bytes;
        let data = one_file(fork).load();
        assert_eq!(
            base_images(&data.ship_looks()),
            [
                (ShipId(128), 1000),
                (ShipId(129), 1001),
                (ShipId(361), 1000)
            ],
            "an undecodable one is left out"
        );
        assert_eq!(self::data().ship_looks(), []);
    }

    /// A `shïp` whose `MovieFile` (offset 0x6AE) is `movie`.
    fn ship_naming(movie: &str) -> Vec<u8> {
        let mut bytes = ShipBuilder::new().build();
        bytes[0x6AE..0x6AE + movie.len()].copy_from_slice(movie.as_bytes());
        bytes
    }

    #[test]
    fn the_ships_movies_are_each_readable_ships_movie_file_not_empty() {
        let mut short = ship_naming("Short.mov");
        short.pop();
        let fork = ForkBuilder::new()
            .resource(Ship::TYPE, 200, None, &ship_naming("Late.mov"))
            .resource(Ship::TYPE, 128, Some(b"Filmed"), &ship_naming("Ship.mov"))
            .resource(Ship::TYPE, 129, None, &ship_naming(""))
            .resource(Ship::TYPE, 130, None, &short)
            .build()
            .bytes;
        let data = one_file(fork).load();
        assert_eq!(
            movie_files(&data.ship_looks()),
            [
                (ShipId(128), "Ship.mov".to_owned()),
                (ShipId(200), "Late.mov".to_owned())
            ],
            "an empty name names no movie (@0x60858-0x60866), and an \
             undecodable ship is left out"
        );
    }

    #[test]
    fn each_class_with_a_ship_or_a_shan_is_one_row_with_its_stand_in_outline() {
        let fork = ForkBuilder::new()
            .resource(Ship::TYPE, 128, None, &ship_naming("Ship.mov"))
            .resource(ShipAnim::TYPE, 128, None, &ship_anim(1000))
            .resource(Ship::TYPE, 129, None, &ship_naming(""))
            .resource(ShipAnim::TYPE, 130, None, &ship_anim(1001))
            .build()
            .bytes;
        let outline = nova_data::Outline {
            triangles: vec![nova_data::OutlineTriangle {
                corners: [[0.0, 2.0], [1.0, -1.0], [-1.0, -1.0]],
                color: [10, 20, 30],
            }],
        };
        let data = one_file(fork).load().with_stand_ins(BTreeMap::from([
            (
                StandInOf::Ship(nova_data::ShipId(129)),
                StandIn::Outline(outline.clone()),
            ),
            (
                StandInOf::Ship(nova_data::ShipId(130)),
                StandIn::Disc { color: [1, 2, 3] },
            ),
        ]));
        assert_eq!(
            data.ship_looks(),
            [
                ShipLook {
                    ship: ShipId(128),
                    base_image: Some(1000),
                    movie_file: Some("Ship.mov".to_owned()),
                    stand_in: None,
                },
                ShipLook {
                    ship: ShipId(129),
                    base_image: None,
                    movie_file: None,
                    stand_in: Some(outline_data::outline(&outline)),
                },
                ShipLook {
                    ship: ShipId(130),
                    base_image: Some(1001),
                    movie_file: None,
                    stand_in: None,
                },
            ],
            "a disc is no ship's stand-in"
        );
    }
}
