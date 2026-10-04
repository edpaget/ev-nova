//! The pilot catalog over the game data: a thin mapping from `GameData`'s
//! `chär`, `shïp` and `sÿst` records.

use nova_data::GameData;
use nova_data::records::character::Character;
use nova_data::records::ship::Ship;
use nova_data::records::system::System;

use crate::catalog::{CharacterStart, PilotCatalog, ShipId, StartError, SystemId};
use crate::handling::ShipFields;

/// Reads the records afresh on every call; a session asks once, when it
/// starts.
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
            }),
            Some(Err(err)) => Err(err.to_string()),
            None => Err(format!("no shïp {}", id.0)),
        }
    }

    fn system_exists(&self, id: SystemId) -> bool {
        matches!(self.get::<System>(id.0), Some(Ok(_)))
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;

    use nova_data::Record;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};

    use super::*;

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
            })
        );
        let shipless = store(&[(Character::TYPE, 128, character(-1, [130, -1, -1, -1]))]);
        assert_eq!(shipless.first_character().map(|c| c.ship), Ok(None));
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
}
