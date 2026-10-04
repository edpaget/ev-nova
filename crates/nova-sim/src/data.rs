//! The pilot catalog over the game data: a thin mapping from `GameData`'s
//! `chär`, `shïp`, `oütf`, `sÿst` and `spöb` records, and its stellar
//! sprites.

use nova_data::GameData;
use nova_data::records::character::Character;
use nova_data::records::outfit::Outfit;
use nova_data::records::ship::Ship;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;

use crate::catalog::{
    CharacterStart, LandingSite, PilotCatalog, ShipId, SoundId, StarSystem, StartDate, StartError,
    SystemId,
};
use crate::fuel::OutfitMod;
use crate::geometry::Vec2;
use crate::handling::ShipFields;

/// Reads the records afresh on every call; a session asks once, when it
/// starts.
///
/// The starting date's day and month are the `chär`'s two words at 0x134
/// and 0x136, which the Bible leaves undocumented and the ResForge template
/// labels the starting day and month (stock: 23 and 6, with `StartYear`
/// 1177).
impl PilotCatalog for GameData {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        let (_, first) = self
            .records::<Character>()
            .next()
            .ok_or(StartError::NoCharacter)?;
        let character = first.map_err(|err| StartError::Character(err.to_string()))?;
        Ok(CharacterStart {
            ship: character.record.ship_type,
            systems: character.record.system,
            start: StartDate {
                day: character.record.unknown_0x134,
                month: character.record.unknown_0x136,
                year: character.record.start_year,
            },
        })
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        match self.get::<Ship>(id.0) {
            Some(Ok(ship)) => Ok(ShipFields {
                speed: ship.record.speed,
                accel: ship.record.accel,
                maneuver: ship.record.maneuver,
                shield: ship.record.shield,
                armor: ship.record.armor,
                fuel: ship.record.fuel,
                fuel_regen: ship.record.fuel_regen,
            }),
            Some(Err(err)) => Err(err.to_string()),
            None => Err(format!("no shïp {}", id.0)),
        }
    }

    fn default_outfits(&self, id: ShipId) -> Vec<OutfitMod> {
        let Some(Ok(ship)) = self.get::<Ship>(id.0) else {
            return Vec::new();
        };
        let ship = ship.record;
        let items = ship
            .default_items1_4
            .into_iter()
            .chain(ship.default_items5_8);
        let counts = ship.item_count1_4.into_iter().chain(ship.item_count5_8);
        items
            .zip(counts)
            .filter_map(|(item, count)| {
                let outfit = self.get::<Outfit>(item?.0)?.ok()?.record;
                // A negative count carries none.
                let count = u16::try_from(count).unwrap_or(0);
                let mods = [
                    (outfit.mod_type, outfit.mod_val),
                    (outfit.mod_type2, outfit.mod_val2),
                    (outfit.mod_type3, outfit.mod_val3),
                    (outfit.mod_type4, outfit.mod_val4),
                ];
                Some(mods.map(|(mod_type, mod_val)| OutfitMod {
                    mod_type,
                    mod_val,
                    count,
                }))
            })
            .flatten()
            .collect()
    }

    fn system_exists(&self, id: SystemId) -> bool {
        matches!(self.get::<System>(id.0), Some(Ok(_)))
    }

    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
        let Some(Ok(system)) = self.get::<System>(system.0) else {
            return Vec::new();
        };
        system
            .record
            .nav_def
            .into_iter()
            .flatten()
            .filter_map(|id| {
                let stellar = self.get::<Stellar>(id.0)?.ok()?.record;
                let frame_size = self
                    .stellar_sprite(id)
                    .ok()
                    .map(|sprite| (sprite.sheet.frame_width(), sprite.sheet.frame_height()));
                Some(LandingSite {
                    id,
                    position: Vec2::new(f32::from(stellar.x_pos), f32::from(stellar.y_pos)),
                    frame_size,
                    flags: stellar.flags.bits(),
                    min_status: stellar.min_status,
                    landing_sound: landing_sound(stellar.cust_snd_id),
                })
            })
            .collect()
    }

    fn star_map(&self) -> Vec<StarSystem> {
        self.records::<System>()
            .filter_map(|(id, system)| {
                let system = system.ok()?.record;
                Some(StarSystem {
                    id: SystemId(id),
                    position: Vec2::new(f32::from(system.x_pos), f32::from(system.y_pos)),
                    links: system.con.into_iter().flatten().collect(),
                })
            })
            .collect()
    }
}

/// The first stellar landing sound: the community *EV Nova Resource ID
/// Guide* gives `snd ` 10000 and up to "custom stellar landing sounds".
const FIRST_LANDING_SOUND: i16 = 10_000;

/// A `spöb`'s `CustSndID` as its landing sound: from
/// [`FIRST_LANDING_SOUND`] up. Anything below is none: -1, 0, and the
/// angle hypergates and wormholes keep in the field (stock 120).
fn landing_sound(cust_snd_id: i16) -> Option<SoundId> {
    (cust_snd_id >= FIRST_LANDING_SOUND).then_some(SoundId(cust_snd_id))
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;

    use nova_data::Record;
    use nova_data::graphics::RLED;
    use nova_data::graphics::fixture::RledBuilder;
    use nova_data::records::spin::Spin;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};

    use super::*;
    use crate::catalog::{StarSystem, StartDate, StellarId};

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

    fn store(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
        let fork = resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
                fork.resource(*ty, *id, None, data)
            })
            .build()
            .bytes;
        let file = OneFile(fork);
        GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
    }

    fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
        for (i, value) in values.iter().enumerate() {
            bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
        }
    }

    /// A `chär` starting in `ship` in these systems.
    fn character(ship: i16, systems: [i16; 4]) -> Vec<u8> {
        let mut bytes = vec![0; Character::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x04, &[ship]);
        put_i16s(&mut bytes, 0x06, &systems);
        bytes
    }

    /// A `shïp` with this `Accel`, `Speed` and `Maneuver`.
    fn ship(accel: i16, speed: i16, maneuver: i16) -> Vec<u8> {
        let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x04, &[accel, speed, maneuver]);
        bytes
    }

    /// A `shïp` with this `Shield`, `Fuel` and `Armor`, and no handling.
    fn reserves(shield: i16, fuel: i16, armor: i16) -> Vec<u8> {
        let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x02, &[shield]);
        put_i16s(&mut bytes, 0x0A, &[fuel]);
        put_i16s(&mut bytes, 0x0E, &[armor]);
        bytes
    }

    fn system() -> Vec<u8> {
        vec![0; System::SIZE.expect("fixed")]
    }

    fn short(mut bytes: Vec<u8>) -> Vec<u8> {
        bytes.pop();
        bytes
    }

    #[test]
    fn the_first_chär_by_id_gives_its_ship_and_systems() {
        let data = store(&[
            (Character::TYPE, 129, character(200, [131, -1, -1, -1])),
            (Character::TYPE, 128, character(128, [-1, 999, 130, -1])),
        ]);
        assert_eq!(
            data.first_character(),
            Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [None, Some(SystemId(999)), Some(SystemId(130)), None],
                start: StartDate::default(),
            })
        );
        let shipless = store(&[(Character::TYPE, 128, character(-1, [130, -1, -1, -1]))]);
        assert_eq!(shipless.first_character().map(|c| c.ship), Ok(None));
    }

    #[test]
    fn the_start_date_is_the_chärs_day_month_and_year() {
        let mut bytes = character(128, [130, -1, -1, -1]);
        put_i16s(&mut bytes, 0x134, &[23, 6, 1177]);
        let data = store(&[(Character::TYPE, 128, bytes)]);
        assert_eq!(
            data.first_character().map(|c| c.start),
            Ok(StartDate {
                day: 23,
                month: 6,
                year: 1177
            })
        );
    }

    #[test]
    fn no_chär_is_no_character() {
        assert_eq!(store(&[]).first_character(), Err(StartError::NoCharacter));
    }

    #[test]
    fn an_undecodable_first_chär_is_its_decode_error() {
        let data = store(&[
            (
                Character::TYPE,
                128,
                short(character(128, [130, -1, -1, -1])),
            ),
            (Character::TYPE, 129, character(128, [130, -1, -1, -1])),
        ]);
        let Err(StartError::Character(message)) = data.first_character() else {
            panic!("a decode error: {:?}", data.first_character())
        };
        assert!(message.contains("128"), "{message}");
    }

    #[test]
    fn a_ships_fields_are_its_speed_accel_and_maneuver() {
        let data = store(&[(Ship::TYPE, 128, ship(250, 400, 15))]);
        assert_eq!(
            data.ship_fields(ShipId(128)),
            Ok(ShipFields {
                speed: 400,
                accel: 250,
                maneuver: 15,
                ..ShipFields::default()
            })
        );
    }

    #[test]
    fn a_ships_fields_include_its_shield_armour_and_fuel() {
        let data = store(&[(Ship::TYPE, 128, reserves(30, 300, -45))]);
        assert_eq!(
            data.ship_fields(ShipId(128)),
            Ok(ShipFields {
                shield: 30,
                armor: -45,
                fuel: 300,
                ..ShipFields::default()
            })
        );
    }

    #[test]
    fn a_ships_fields_include_its_fuel_regeneration() {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x5E, &[8]);
        let data = store(&[(Ship::TYPE, 128, bytes)]);
        assert_eq!(data.ship_fields(ShipId(128)).map(|f| f.fuel_regen), Ok(8));
    }

    /// A `shïp` carrying `items` in its `DefaultItems` 1-4 and `more` in
    /// 5-8, each an `oütf` ID and a count; every other slot -1.
    fn outfitted(items: &[(i16, i16)], more: &[(i16, i16)]) -> Vec<u8> {
        let mut bytes = ship(1, 2, 3);
        for (ids_at, counts_at, slots) in [(0x4E, 0x56, items), (0x370, 0x378, more)] {
            put_i16s(&mut bytes, ids_at, &[-1; 4]);
            for (i, &(id, count)) in slots.iter().enumerate() {
                put_i16s(&mut bytes, ids_at + 2 * i, &[id]);
                put_i16s(&mut bytes, counts_at + 2 * i, &[count]);
            }
        }
        bytes
    }

    /// An `oütf` with these four `ModType` and `ModVal` pairs.
    fn outfit(mods: [(i16, i16); 4]) -> Vec<u8> {
        let mut bytes = vec![0; Outfit::SIZE.expect("fixed")];
        for (at, (mod_type, mod_val)) in [0x06, 0x12, 0x16, 0x1A].into_iter().zip(mods) {
            put_i16s(&mut bytes, at, &[mod_type, mod_val]);
        }
        bytes
    }

    fn outfit_mod(mod_type: i16, mod_val: i16, count: u16) -> OutfitMod {
        OutfitMod {
            mod_type,
            mod_val,
            count,
        }
    }

    #[test]
    fn a_ships_default_outfits_are_every_mod_of_each_readable_item_with_its_count() {
        let data = store(&[
            (
                Ship::TYPE,
                128,
                outfitted(&[(200, 2), (999, 1), (202, 1)], &[(201, 3), (200, -4)]),
            ),
            (
                Outfit::TYPE,
                200,
                outfit([(18, 8), (1, 5), (0, 0), (-1, 7)]),
            ),
            (
                Outfit::TYPE,
                201,
                outfit([(18, -20), (2, 3), (4, 5), (6, 7)]),
            ),
            (Outfit::TYPE, 202, short(outfit([(18, 1); 4]))),
        ]);
        assert_eq!(
            data.default_outfits(ShipId(128)),
            [
                outfit_mod(18, 8, 2),
                outfit_mod(1, 5, 2),
                outfit_mod(0, 0, 2),
                outfit_mod(-1, 7, 2),
                outfit_mod(18, -20, 3),
                outfit_mod(2, 3, 3),
                outfit_mod(4, 5, 3),
                outfit_mod(6, 7, 3),
                outfit_mod(18, 8, 0),
                outfit_mod(1, 5, 0),
                outfit_mod(0, 0, 0),
                outfit_mod(-1, 7, 0),
            ],
            "a missing or undecodable oütf is skipped, a negative count carries none"
        );
    }

    #[test]
    fn a_missing_undecodable_or_unoutfitted_ship_has_no_default_outfits() {
        let data = store(&[
            (Ship::TYPE, 128, outfitted(&[], &[])),
            (Ship::TYPE, 129, short(outfitted(&[(200, 1)], &[]))),
            (Outfit::TYPE, 200, outfit([(18, 8); 4])),
        ]);
        assert_eq!(data.default_outfits(ShipId(128)), []);
        assert_eq!(data.default_outfits(ShipId(129)), []);
        assert_eq!(data.default_outfits(ShipId(130)), []);
    }

    #[test]
    fn a_missing_or_undecodable_ship_says_why() {
        let data = store(&[(Ship::TYPE, 129, short(ship(1, 2, 3)))]);
        assert_eq!(data.ship_fields(ShipId(140)), Err("no shïp 140".to_owned()));
        let Err(message) = data.ship_fields(ShipId(129)) else {
            panic!("an error")
        };
        assert!(message.contains("129"), "{message}");
    }

    #[test]
    fn a_system_exists_when_it_is_there_and_decodes() {
        let data = store(&[
            (System::TYPE, 130, system()),
            (System::TYPE, 131, short(system())),
        ]);
        assert!(data.system_exists(SystemId(130)));
        assert!(!data.system_exists(SystemId(131)), "undecodable");
        assert!(!data.system_exists(SystemId(999)), "missing");
    }

    /// A `sÿst` whose `nav_def` holds `stellars`, every other slot -1.
    fn system_with(stellars: &[i16]) -> Vec<u8> {
        let mut bytes = system();
        put_i16s(&mut bytes, 0x24, &[-1; 16]);
        put_i16s(&mut bytes, 0x24, stellars);
        bytes
    }

    /// A `spöb` at (`x`, `y`) of graphic type `graphic_type`, with these
    /// `Flags` and `MinStatus`.
    fn stellar(x: i16, y: i16, graphic_type: i16, flags: u32, min_status: i16) -> Vec<u8> {
        let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[x, y, graphic_type]);
        bytes[0x06..0x0A].copy_from_slice(&flags.to_be_bytes());
        put_i16s(&mut bytes, 0x16, &[min_status]);
        bytes
    }

    /// A `spïn` naming `rlëD` `image`, one frame across.
    fn spin(image: i16) -> Vec<u8> {
        let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[image, -1, 0, 0, 1, 1]);
        bytes
    }

    /// One `width` x `height` frame.
    fn sheet(width: u16, height: u16) -> Vec<u8> {
        RledBuilder::new(width, height)
            .frame(|f| (0..height).fold(f, |f, _| f.line().pixels(&vec![0x7C00; width.into()])))
            .build()
    }

    #[test]
    fn a_systems_landing_sites_are_its_readable_stellars_in_nav_order() {
        let data = store(&[
            (System::TYPE, 130, system_with(&[129, 999, 128, 131])),
            (Stellar::TYPE, 128, stellar(-300, 450, 7, 0x0000_0013, 25)),
            (Stellar::TYPE, 129, stellar(10, -20, 0, 0x2001, -32767)),
            (Stellar::TYPE, 131, short(stellar(0, 0, 0, 1, 0))),
            (Spin::TYPE, 1000, spin(1000)),
            (RLED, 1000, sheet(12, 30)),
        ]);
        assert_eq!(
            data.landing_sites(SystemId(130)),
            [
                LandingSite {
                    id: StellarId(129),
                    position: Vec2::new(10.0, -20.0),
                    frame_size: Some((12, 30)),
                    flags: 0x2001,
                    min_status: -32767,
                    landing_sound: None,
                },
                LandingSite {
                    id: StellarId(128),
                    position: Vec2::new(-300.0, 450.0),
                    frame_size: None,
                    flags: 0x13,
                    min_status: 25,
                    landing_sound: None,
                },
            ]
        );
    }

    /// A `sÿst` at map (`x`, `y`) with these hyperlinks, every other slot
    /// -1.
    fn linked(x: i16, y: i16, links: &[i16]) -> Vec<u8> {
        let mut bytes = system();
        put_i16s(&mut bytes, 0x00, &[x, y]);
        put_i16s(&mut bytes, 0x04, &[-1; 16]);
        put_i16s(&mut bytes, 0x04, links);
        bytes
    }

    #[test]
    fn the_star_map_is_every_readable_system_by_id_with_its_position_and_links() {
        let data = store(&[
            (System::TYPE, 131, linked(600, -75, &[130, 999, 131])),
            (System::TYPE, 129, short(linked(0, 0, &[130]))),
            (System::TYPE, 130, linked(-20, 40, &[])),
        ]);
        assert_eq!(
            data.star_map(),
            [
                StarSystem {
                    id: SystemId(130),
                    position: Vec2::new(-20.0, 40.0),
                    links: Vec::new(),
                },
                StarSystem {
                    id: SystemId(131),
                    position: Vec2::new(600.0, -75.0),
                    links: vec![SystemId(130), SystemId(999), SystemId(131)],
                },
            ]
        );
        assert_eq!(store(&[]).star_map(), []);
    }

    /// A landable `spöb` at the centre with this `CustSndID`.
    fn sounding(cust_snd_id: i16) -> Vec<u8> {
        let mut bytes = stellar(0, 0, 0, 1, 0);
        put_i16s(&mut bytes, 0x1A, &[cust_snd_id]);
        bytes
    }

    #[test]
    fn a_landing_sites_sound_is_its_custom_sound_from_10000_up() {
        // Port Kane's 10032; none (-1), 0, the 120 a hypergate keeps there
        // as an angle, and 9999, are no landing sound.
        let sounds = [10_032, -1, 0, 120, 9_999, 10_000];
        let mut resources = vec![(
            System::TYPE,
            130,
            system_with(&[128, 129, 130, 131, 132, 133]),
        )];
        for (id, sound) in (128..).zip(sounds) {
            resources.push((Stellar::TYPE, id, sounding(sound)));
        }
        let data = store(&resources);
        let found: Vec<_> = data
            .landing_sites(SystemId(130))
            .iter()
            .map(|site| site.landing_sound)
            .collect();
        assert_eq!(
            found,
            [
                Some(SoundId(10_032)),
                None,
                None,
                None,
                None,
                Some(SoundId(10_000)),
            ]
        );
    }

    #[test]
    fn a_missing_or_undecodable_system_has_no_landing_sites() {
        let data = store(&[
            (System::TYPE, 131, short(system_with(&[128]))),
            (Stellar::TYPE, 128, stellar(0, 0, 0, 1, 0)),
        ]);
        assert_eq!(data.landing_sites(SystemId(131)), []);
        assert_eq!(data.landing_sites(SystemId(130)), []);
    }
}
