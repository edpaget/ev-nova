//! The pilot catalog over the game data: a thin mapping from `GameData`'s
//! `chär`, `shïp`, `oütf`, `sÿst`, `spöb`, `jünk` and `öops` records, its
//! commodity string lists and its stellar sprites.

use nova_data::GameData;
use nova_data::records::character::Character;
use nova_data::records::disaster::Disaster;
use nova_data::records::junk::Junk;
use nova_data::records::outfit::Outfit;
use nova_data::records::ship::Ship;
use nova_data::records::stellar::Stellar;
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;

use crate::catalog::{
    CharacterStart, CommodityStrings, DisasterId, DisasterRecord, JunkRecord, LandingSite,
    OutfitId, OutfitRecord, PilotCatalog, ShipId, ShipRecord, SoundId, StarSystem, StartDate,
    StartError, SystemId,
};
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
            cash: character.record.cash,
            legal: std::array::from_fn(|slot| {
                character.record.govt[slot].map(|govt| (govt, character.record.status[slot]))
            }),
        })
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        match self.get::<Ship>(id.0) {
            Some(Ok(ship)) => Ok(ship_fields(ship.record)),
            Some(Err(err)) => Err(err.to_string()),
            None => Err(format!("no shïp {}", id.0)),
        }
    }

    fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
        let Some(Ok(ship)) = self.get::<Ship>(id.0) else {
            return Vec::new();
        };
        default_items(ship.record)
    }

    fn ships(&self) -> Vec<ShipRecord> {
        self.records::<Ship>()
            .filter_map(|(id, ship)| {
                let ship = ship.ok()?;
                let record = ship.record;
                let short_name = record.short_name.as_str().to_owned();
                Some(ShipRecord {
                    id: ShipId(id),
                    name: ship.name.map_or_else(|| short_name.clone(), str::to_owned),
                    short_name,
                    long_name: record.long_name.as_str().to_owned(),
                    fields: ship_fields(record),
                    defaults: default_items(record),
                    cost: record.cost,
                    tech_level: record.tech_level,
                    buy_random: record.buy_random,
                    require: record.require.bits(),
                    availability: record.availability.as_str().to_owned(),
                    flags3: record.flags3.bits(),
                    disp_weight: record.disp_weight,
                    max_gun: record.max_gun,
                    max_tur: record.max_tur,
                    length: record.length,
                    crew: record.crew,
                })
            })
            .collect()
    }

    fn outfits(&self) -> Vec<OutfitRecord> {
        self.records::<Outfit>()
            .filter_map(|(id, outfit)| {
                let outfit = outfit.ok()?;
                let record = outfit.record;
                Some(OutfitRecord {
                    id: OutfitId(id),
                    name: outfit
                        .name
                        .map_or_else(|| record.lc_name.as_str().to_owned(), str::to_owned),
                    short_name: record.short_name.as_str().to_owned(),
                    disp_weight: record.disp_weight,
                    mass: record.mass,
                    tech_level: record.tech_level,
                    max: record.max,
                    flags: record.flags.bits(),
                    cost: record.cost,
                    mods: [
                        (record.mod_type, record.mod_val),
                        (record.mod_type2, record.mod_val2),
                        (record.mod_type3, record.mod_val3),
                        (record.mod_type4, record.mod_val4),
                    ],
                    contribute: record.contribute.bits(),
                    require: record.require.bits(),
                    require_govt: record.require_govt,
                    availability: record.availability.as_str().to_owned(),
                })
            })
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
                let mut special_tech = [0; 8];
                special_tech[..3].copy_from_slice(&stellar.special_tech1_3);
                special_tech[3..].copy_from_slice(&stellar.special_tech4_8);
                Some(LandingSite {
                    id,
                    position: Vec2::new(f32::from(stellar.x_pos), f32::from(stellar.y_pos)),
                    frame_size,
                    flags: stellar.flags.bits(),
                    min_status: stellar.min_status,
                    landing_sound: landing_sound(stellar.cust_snd_id),
                    tech_level: stellar.tech_level,
                    special_tech,
                    govt: stellar.govt,
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

    fn commodity_strings(&self) -> CommodityStrings {
        CommodityStrings {
            names: strings(self, COMMODITY_NAMES),
            base_prices: strings(self, BASE_PRICES),
        }
    }

    fn junk(&self) -> Vec<JunkRecord> {
        self.records::<Junk>()
            .filter_map(|(id, junk)| {
                let junk = junk.ok()?;
                let record = junk.record;
                let stellars = |slots: &[Option<_>]| slots.iter().flatten().copied().collect();
                Some(JunkRecord {
                    id: nova_data::JunkId(id),
                    name: junk
                        .name
                        .map_or_else(|| record.lc_name.as_str().to_owned(), str::to_owned),
                    base_price: record.base_price,
                    sold_at: stellars(&record.sold_at),
                    bought_at: stellars(&record.bought_at),
                    buy_on: record.buy_on.as_str().to_owned(),
                    sell_on: record.sell_on.as_str().to_owned(),
                })
            })
            .collect()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        self.records::<Disaster>()
            .filter_map(|(id, disaster)| {
                let disaster = disaster.ok()?;
                let record = disaster.record;
                Some(DisasterRecord {
                    id: DisasterId(id),
                    name: disaster.name.unwrap_or_default().to_owned(),
                    // The decoder reads -1 (any stellar) as none.
                    stellar: record.stellar.map_or(-1, |stellar| stellar.0),
                    commodity: record.commodity,
                    price_delta: record.price_delta,
                    duration: record.duration,
                    freq: record.freq,
                    activate_on: record.activate_on.as_str().to_owned(),
                })
            })
            .collect()
    }
}

/// A `shïp`'s handling, reserve, cargo and mass fields.
fn ship_fields(ship: &Ship) -> ShipFields {
    ShipFields {
        speed: ship.speed,
        accel: ship.accel,
        maneuver: ship.maneuver,
        shield: ship.shield,
        armor: ship.armor,
        fuel: ship.fuel,
        fuel_regen: ship.fuel_regen,
        holds: ship.holds,
        mass: ship.mass,
        free_mass: ship.free_mass,
        contribute: ship.contribute.bits(),
    }
}

/// A `shïp`'s `DefaultItems`, each item with its count in slot order.
fn default_items(ship: &Ship) -> Vec<(OutfitId, u16)> {
    let items = ship
        .default_items1_4
        .into_iter()
        .chain(ship.default_items5_8);
    let counts = ship.item_count1_4.into_iter().chain(ship.item_count5_8);
    items
        .zip(counts)
        // A negative count carries none.
        .filter_map(|(item, count)| Some((item?, u16::try_from(count).unwrap_or(0))))
        .collect()
}

/// The `STR#` naming the standard commodities, "All Cargo".
const COMMODITY_NAMES: i16 = 4000;

/// The `STR#` pricing them, "Base Prices".
const BASE_PRICES: i16 = 4004;

/// Every string of `data`'s `STR#` `id`; none when it is missing or
/// cannot be read.
fn strings(data: &GameData, id: i16) -> Vec<String> {
    let Some(Ok(list)) = data.get::<StrList>(id) else {
        return Vec::new();
    };
    list.record
        .strings
        .iter()
        .map(|string| string.as_str().to_owned())
        .collect()
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
    use nova_data::records::disaster::Disaster;
    use nova_data::records::junk::Junk;
    use nova_data::records::spin::Spin;
    use nova_data::records::string_list::StrList;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};

    use super::*;
    use crate::catalog::{DisasterId, GovtId, JunkId, StarSystem, StartDate, StellarId};

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
        let named: Vec<_> = resources
            .iter()
            .map(|(ty, id, data)| (*ty, *id, None, data.clone()))
            .collect();
        store_named(&named)
    }

    /// The store of these resources, each with its name, if any.
    fn store_named(resources: &[(ResType, i16, Option<&str>, Vec<u8>)]) -> GameData {
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
            bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
        }
    }

    /// A `chär` starting in `ship` in these systems, with no cash and no
    /// legal records.
    fn character(ship: i16, systems: [i16; 4]) -> Vec<u8> {
        let mut bytes = vec![0; Character::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x04, &[ship]);
        put_i16s(&mut bytes, 0x06, &systems);
        put_i16s(&mut bytes, 0x0E, &[-1; 4]);
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
                cash: 0,
                legal: [None; 4],
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
    fn the_start_carries_the_chärs_cash_and_legal_records() {
        let mut bytes = character(128, [130, -1, -1, -1]);
        bytes[0x00..0x04].copy_from_slice(&(-7_000_i32).to_be_bytes());
        put_i16s(&mut bytes, 0x0E, &[129, -1, 131, 140]);
        put_i16s(&mut bytes, 0x16, &[50, 99, -20, 0]);
        let data = store(&[(Character::TYPE, 128, bytes)]);
        let start = data.first_character().expect("decodes");
        assert_eq!(start.cash, -7_000);
        assert_eq!(
            start.legal,
            [
                Some((GovtId(129), 50)),
                None,
                Some((GovtId(131), -20)),
                Some((GovtId(140), 0)),
            ]
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
    fn a_ships_fields_include_its_holds() {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x00, &[-10]);
        let data = store(&[(Ship::TYPE, 128, bytes)]);
        assert_eq!(data.ship_fields(ShipId(128)).map(|f| f.holds), Ok(-10));
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

    #[test]
    fn a_ships_fields_include_its_mass_free_mass_and_contribute() {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x0C, &[8]);
        put_i16s(&mut bytes, 0x3E, &[15]);
        bytes[0x64..0x6C].copy_from_slice(&0x0000_0008_0000_0001_u64.to_be_bytes());
        let data = store(&[(Ship::TYPE, 128, bytes)]);
        let fields = data.ship_fields(ShipId(128)).expect("decodes");
        assert_eq!(
            (fields.mass, fields.free_mass, fields.contribute),
            (15, 8, 0x0000_0008_0000_0001)
        );
    }

    #[test]
    fn a_ships_default_outfits_are_each_item_with_its_count_in_slot_order() {
        let data = store(&[(
            Ship::TYPE,
            128,
            outfitted(&[(200, 2), (999, 1), (202, 1)], &[(201, 3), (200, -4)]),
        )]);
        assert_eq!(
            data.default_outfits(ShipId(128)),
            [
                (OutfitId(200), 2),
                (OutfitId(999), 1),
                (OutfitId(202), 1),
                (OutfitId(201), 3),
                (OutfitId(200), 0),
            ],
            "repeats kept, a negative count carries none, and the oütf need not exist"
        );
    }

    #[test]
    fn a_missing_undecodable_or_unoutfitted_ship_has_no_default_outfits() {
        let data = store(&[
            (Ship::TYPE, 128, outfitted(&[], &[])),
            (Ship::TYPE, 129, short(outfitted(&[(200, 1)], &[]))),
        ]);
        assert_eq!(data.default_outfits(ShipId(128)), []);
        assert_eq!(data.default_outfits(ShipId(129)), []);
        assert_eq!(data.default_outfits(ShipId(130)), []);
    }

    /// A `shïp` sold in the shipyard: every field the shipyard reads set to
    /// something of its own, with two default items.
    fn for_sale() -> Vec<u8> {
        let mut bytes = outfitted(&[(200, 2), (201, -1)], &[(202, 1)]);
        put_i16s(&mut bytes, 0x00, &[-15]);
        put_i16s(&mut bytes, 0x0C, &[12]);
        put_i16s(&mut bytes, 0x2A, &[4, 2, 6]);
        bytes[0x30..0x34].copy_from_slice(&17_500_i32.to_be_bytes());
        put_i16s(&mut bytes, 0x3C, &[25, 30, 41]);
        put_i16s(&mut bytes, 0x44, &[3]);
        bytes[0x64..0x6C].copy_from_slice(&0x10_u64.to_be_bytes());
        bytes[0x6C..0x70].copy_from_slice(b"b422");
        bytes[0x380..0x388].copy_from_slice(&0x0000_0002_0000_0001_u64.to_be_bytes());
        put_i16s(&mut bytes, 0x388, &[45]);
        bytes[0x5CE..0x5DD].copy_from_slice(b"Heavy\\nShuttle!");
        bytes[0x62E..0x63D].copy_from_slice(b"A Heavy Shuttle");
        bytes[0x726..0x728].copy_from_slice(&0x4100_u16.to_be_bytes());
        bytes
    }

    #[test]
    fn each_readable_shïp_is_a_ship_record_by_id() {
        let data = store_named(&[
            (Ship::TYPE, 130, None, for_sale()),
            (Ship::TYPE, 129, Some("Heavy Shuttle"), for_sale()),
            (Ship::TYPE, 131, Some("Short"), short(for_sale())),
        ]);
        let record = |id: i16, name: &str| ShipRecord {
            id: ShipId(id),
            name: name.to_owned(),
            short_name: "Heavy\\nShuttle!".to_owned(),
            long_name: "A Heavy Shuttle".to_owned(),
            fields: ShipFields {
                holds: -15,
                accel: 1,
                speed: 2,
                maneuver: 3,
                free_mass: 12,
                mass: 30,
                contribute: 0x10,
                ..ShipFields::default()
            },
            defaults: vec![(OutfitId(200), 2), (OutfitId(201), 0), (OutfitId(202), 1)],
            cost: 17_500,
            tech_level: 6,
            buy_random: 45,
            require: 0x0000_0002_0000_0001,
            availability: "b422".to_owned(),
            flags3: 0x4100,
            disp_weight: 25,
            max_gun: 4,
            max_tur: 2,
            length: 41,
            crew: 3,
        };
        assert_eq!(
            data.ships(),
            [
                record(129, "Heavy Shuttle"),
                record(130, "Heavy\\nShuttle!"),
            ],
            "a resource without a name goes by its ShortName; an undecodable one is skipped"
        );
        assert_eq!(
            data.ships()[0].fields,
            data.ship_fields(ShipId(129)).expect("decodes"),
            "the same fields a session flies with"
        );
        assert_eq!(
            data.ships()[0].defaults,
            data.default_outfits(ShipId(129)),
            "the same default items"
        );
        assert_eq!(store(&[]).ships(), []);
    }

    /// An `oütf` with these four `ModType` and `ModVal` pairs, every other
    /// field set to something of its own.
    fn outfit(mods: [(i16, i16); 4]) -> Vec<u8> {
        let mut bytes = vec![0; Outfit::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[50, -5, 3]);
        put_i16s(&mut bytes, 0x0A, &[8]);
        bytes[0x0C..0x0E].copy_from_slice(&0x4100_u16.to_be_bytes());
        bytes[0x0E..0x12].copy_from_slice(&150_000_i32.to_be_bytes());
        for (at, (mod_type, mod_val)) in [0x06, 0x12, 0x16, 0x1A].into_iter().zip(mods) {
            put_i16s(&mut bytes, at, &[mod_type, mod_val]);
        }
        bytes[0x1E..0x26].copy_from_slice(&0x10_u64.to_be_bytes());
        bytes[0x26..0x2E].copy_from_slice(&0x0000_0008_0000_0001_u64.to_be_bytes());
        bytes[0x2E..0x31].copy_from_slice(b"b12");
        bytes[0x32B..0x334].copy_from_slice(b"Big\\nGun!");
        bytes[0x36B..0x372].copy_from_slice(b"big gun");
        put_i16s(&mut bytes, 0x3F2, &[1128]);
        bytes
    }

    #[test]
    fn each_readable_oütf_is_an_outfit_record_by_id() {
        let data = store_named(&[
            (
                Outfit::TYPE,
                201,
                None,
                outfit([(2, 5), (9, -2), (0, 0), (18, 4)]),
            ),
            (
                Outfit::TYPE,
                200,
                Some("Big Gun"),
                outfit([(1, 128), (0, 0), (0, 0), (0, 0)]),
            ),
            (Outfit::TYPE, 202, Some("Short"), short(outfit([(0, 0); 4]))),
        ]);
        let record = |id: i16, name: &str, mods| OutfitRecord {
            id: OutfitId(id),
            name: name.to_owned(),
            short_name: "Big\\nGun!".to_owned(),
            disp_weight: 50,
            mass: -5,
            tech_level: 3,
            max: 8,
            flags: 0x4100,
            cost: 150_000,
            mods,
            contribute: 0x10,
            require: 0x0000_0008_0000_0001,
            require_govt: 1128,
            availability: "b12".to_owned(),
        };
        assert_eq!(
            data.outfits(),
            [
                record(200, "Big Gun", [(1, 128), (0, 0), (0, 0), (0, 0)]),
                record(201, "big gun", [(2, 5), (9, -2), (0, 0), (18, 4)]),
            ],
            "a resource without a name goes by its LCName; an undecodable one is skipped"
        );
        assert_eq!(store(&[]).outfits(), []);
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
    /// `Flags` and `MinStatus`, independent and of no tech level.
    fn stellar(x: i16, y: i16, graphic_type: i16, flags: u32, min_status: i16) -> Vec<u8> {
        let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[x, y, graphic_type]);
        bytes[0x06..0x0A].copy_from_slice(&flags.to_be_bytes());
        put_i16s(&mut bytes, 0x14, &[-1, min_status]);
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
                    tech_level: 0,
                    special_tech: [0; 8],
                    govt: None,
                },
                LandingSite {
                    id: StellarId(128),
                    position: Vec2::new(-300.0, 450.0),
                    frame_size: None,
                    flags: 0x13,
                    min_status: 25,
                    landing_sound: None,
                    tech_level: 0,
                    special_tech: [0; 8],
                    govt: None,
                },
            ]
        );
    }

    #[test]
    fn a_landing_sites_tech_levels_and_government_are_its_spöbs() {
        let mut port = stellar(0, 0, 0, 0x05, 0);
        put_i16s(&mut port, 0x0C, &[4, 6, 55, 57, 128]);
        put_i16s(&mut port, 0x444, &[58, 81, 0, -1, 9]);
        let data = store(&[
            (System::TYPE, 130, system_with(&[128, 129])),
            (Stellar::TYPE, 128, port),
            (Stellar::TYPE, 129, stellar(5, 5, 0, 1, 0)),
        ]);
        let sites = data.landing_sites(SystemId(130));
        assert_eq!(
            sites
                .iter()
                .map(|site| (site.tech_level, site.special_tech, site.govt))
                .collect::<Vec<_>>(),
            [
                (4, [6, 55, 57, 58, 81, 0, -1, 9], Some(GovtId(128))),
                (0, [0; 8], None),
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

    /// A `STR#` of `strings`.
    fn str_list(strings: &[&str]) -> Vec<u8> {
        let mut bytes = u16::try_from(strings.len())
            .expect("few")
            .to_be_bytes()
            .to_vec();
        for string in strings {
            bytes.push(u8::try_from(string.len()).expect("short"));
            bytes.extend(string.as_bytes());
        }
        bytes
    }

    #[test]
    fn the_commodity_strings_are_str_4000_and_4004_whole() {
        let data = store(&[
            (
                StrList::TYPE,
                4000,
                str_list(&["Food", "Industrial", "*Cargo"]),
            ),
            (StrList::TYPE, 4004, str_list(&["75", "lots"])),
            (StrList::TYPE, 4001, str_list(&["other"])),
        ]);
        assert_eq!(
            data.commodity_strings(),
            CommodityStrings {
                names: vec!["Food".into(), "Industrial".into(), "*Cargo".into()],
                base_prices: vec!["75".into(), "lots".into()],
            }
        );
    }

    #[test]
    fn missing_or_undecodable_commodity_strings_are_none() {
        assert_eq!(store(&[]).commodity_strings(), CommodityStrings::default());
        let data = store(&[
            (StrList::TYPE, 4000, short(str_list(&["Food"]))),
            (StrList::TYPE, 4004, str_list(&["75"])),
        ]);
        assert_eq!(
            data.commodity_strings(),
            CommodityStrings {
                names: Vec::new(),
                base_prices: vec!["75".into()],
            }
        );
    }

    /// A `jünk` sold at `sold`, bought at `bought` (every other slot -1),
    /// at `price`, named `lc_name` in lower case.
    fn junk(sold: &[i16], bought: &[i16], price: i16, lc_name: &str) -> Vec<u8> {
        let mut bytes = vec![0; Junk::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[-1; 16]);
        put_i16s(&mut bytes, 0x00, sold);
        put_i16s(&mut bytes, 0x10, bought);
        put_i16s(&mut bytes, 0x20, &[price]);
        bytes[0x26..0x26 + lc_name.len()].copy_from_slice(lc_name.as_bytes());
        bytes
    }

    #[test]
    fn each_readable_jünk_is_a_special_good_by_id() {
        let mut opals = junk(&[189, 165], &[185, -1, 199], 1200, "opals");
        opals[0xA6..0xA9].copy_from_slice(b"b43");
        opals[0x1A5..0x1A9].copy_from_slice(b"!b80");
        let data = store_named(&[
            (Junk::TYPE, 146, Some("Opals"), opals),
            (Junk::TYPE, 134, None, junk(&[160], &[], 300, "water")),
            (
                Junk::TYPE,
                140,
                Some("Broken"),
                short(junk(&[], &[], 1, "x")),
            ),
        ]);
        assert_eq!(
            data.junk(),
            [
                JunkRecord {
                    id: JunkId(134),
                    name: "water".to_owned(),
                    base_price: 300,
                    sold_at: vec![StellarId(160)],
                    bought_at: Vec::new(),
                    buy_on: String::new(),
                    sell_on: String::new(),
                },
                JunkRecord {
                    id: JunkId(146),
                    name: "Opals".to_owned(),
                    base_price: 1200,
                    sold_at: vec![StellarId(189), StellarId(165)],
                    bought_at: vec![StellarId(185), StellarId(199)],
                    buy_on: "b43".to_owned(),
                    sell_on: "!b80".to_owned(),
                },
            ],
            "a resource without a name goes by its LCName; an undecodable one is skipped"
        );
        assert_eq!(store(&[]).junk(), []);
    }

    /// An `öops` at `stellar` moving `commodity` by `delta` for `duration`
    /// days, `freq` % a day.
    fn disaster(stellar: i16, commodity: i16, delta: i16, duration: i16, freq: i16) -> Vec<u8> {
        let mut bytes = vec![0; Disaster::SIZE.expect("fixed")];
        put_i16s(
            &mut bytes,
            0x00,
            &[stellar, commodity, delta, duration, freq],
        );
        bytes
    }

    #[test]
    fn each_readable_öops_is_an_event_by_id() {
        let mut gated = disaster(-2, 3, 40, 100, 25);
        gated[0x0A..0x0E].copy_from_slice(b"!b80");
        let data = store_named(&[
            (Disaster::TYPE, 129, None, gated),
            (
                Disaster::TYPE,
                128,
                Some("An enormous food surplus"),
                disaster(137, 0, -15, 30, 35),
            ),
            (
                Disaster::TYPE,
                130,
                Some("x"),
                short(disaster(1, 1, 1, 1, 1)),
            ),
            (
                Disaster::TYPE,
                131,
                Some("Anywhere"),
                disaster(-1, 1, 1, 1, 1),
            ),
        ]);
        assert_eq!(
            data.disasters(),
            [
                DisasterRecord {
                    id: DisasterId(128),
                    name: "An enormous food surplus".to_owned(),
                    stellar: 137,
                    commodity: 0,
                    price_delta: -15,
                    duration: 30,
                    freq: 35,
                    activate_on: String::new(),
                },
                DisasterRecord {
                    id: DisasterId(129),
                    name: String::new(),
                    stellar: -2,
                    commodity: 3,
                    price_delta: 40,
                    duration: 100,
                    freq: 25,
                    activate_on: "!b80".to_owned(),
                },
                DisasterRecord {
                    id: DisasterId(131),
                    name: "Anywhere".to_owned(),
                    stellar: -1,
                    commodity: 1,
                    price_delta: 1,
                    duration: 1,
                    freq: 1,
                    activate_on: String::new(),
                },
            ]
        );
        assert_eq!(store(&[]).disasters(), []);
    }
}
