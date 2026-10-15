//! An asteroid field through a flight session, headless: a system holding
//! asteroids, entered through the real session construction, fills from
//! the field's edge one asteroid a pass, each of a type its `AstTypes`
//! sets.

use nova_sim::{
    AsteroidCatalog, AsteroidFieldRecord, AsteroidTypeRecord, CharacterStart, CombatCatalog,
    CommodityStrings, Controls, DateAffixes, DisasterRecord, GovtRecord, HullRecord, JunkRecord,
    LandingSite, NeverFires, OutfitId, OutfitRecord, PilotCatalog, Session, ShipFields, ShipId,
    StarSystem, StartDate, StartError, StellarId, SystemId, WeaponRecord,
};

/// One `chär` flying an average ship from system 130, which has no
/// stellars and holds 4 asteroids of type 0, `röid` 128.
struct Pilot;

/// Unarmed and ungoverned.
impl CombatCatalog for Pilot {
    fn weapons(&self) -> Vec<WeaponRecord> {
        Vec::new()
    }

    fn hulls(&self) -> Vec<HullRecord> {
        Vec::new()
    }

    fn governments(&self) -> Vec<GovtRecord> {
        Vec::new()
    }
}

/// `röid` 128, of `Strength` 40 and `SpinRate` 150, its sprite 36 frames
/// of 48 x 48; system 130's field, 4 of type 0.
impl AsteroidCatalog for Pilot {
    fn asteroid_types(&self) -> Vec<AsteroidTypeRecord> {
        vec![AsteroidTypeRecord {
            kind: 0,
            strength: 40,
            spin_rate: 150,
            frames: 36,
            frame_size: Some((48, 48)),
            ..AsteroidTypeRecord::default()
        }]
    }

    fn asteroid_fields(&self) -> Vec<AsteroidFieldRecord> {
        vec![AsteroidFieldRecord {
            system: SystemId(130),
            count: 4,
            types: 0x0001,
        }]
    }
}

impl PilotCatalog for Pilot {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        Ok(CharacterStart {
            ship: Some(ShipId(128)),
            systems: [Some(SystemId(130)), None, None, None],
            start: StartDate::default(),
            ..CharacterStart::default()
        })
    }

    fn ship_fields(&self, _id: ShipId) -> Result<ShipFields, String> {
        Ok(ShipFields {
            speed: 300,
            accel: 300,
            maneuver: 60,
            ..ShipFields::default()
        })
    }

    fn default_outfits(&self, _id: ShipId) -> Vec<(OutfitId, u16)> {
        Vec::new()
    }

    fn outfits(&self) -> Vec<OutfitRecord> {
        Vec::new()
    }

    fn ships(&self) -> Vec<nova_sim::ShipRecord> {
        Vec::new()
    }

    fn system_exists(&self, id: SystemId) -> bool {
        id == SystemId(130)
    }

    fn landing_sites(&self, _system: SystemId) -> Vec<LandingSite> {
        Vec::new()
    }

    fn star_map(&self) -> Vec<StarSystem> {
        Vec::new()
    }

    fn commodity_strings(&self) -> CommodityStrings {
        CommodityStrings::default()
    }

    fn string_patch(&self, _id: i16) -> nova_sim::catalog::StringPatch {
        nova_sim::catalog::StringPatch::Absent
    }

    fn buoy_strings(&self) -> Vec<String> {
        Vec::new()
    }

    fn junk(&self) -> Vec<JunkRecord> {
        Vec::new()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        Vec::new()
    }

    fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
        Vec::new()
    }

    fn date_affixes(&self) -> DateAffixes {
        DateAffixes::default()
    }

    fn gate_sites(&self) -> Vec<nova_sim::GateSite> {
        Vec::new()
    }

    fn stellar_lives(&self) -> Vec<nova_sim::StellarLife> {
        Vec::new()
    }

    fn fallback_frame_size(&self) -> Option<(u32, u32)> {
        Some((100, 100))
    }

    fn missions(&self) -> Vec<nova_sim::catalog::MissionRecord> {
        Vec::new()
    }

    fn mission_texts(&self) -> Vec<nova_sim::catalog::MissionTexts> {
        Vec::new()
    }

    fn listed_stellars(&self) -> Vec<nova_sim::catalog::ListedStellar> {
        Vec::new()
    }

    fn system_visibility(&self) -> Vec<nova_sim::catalog::SystemVisibility> {
        Vec::new()
    }

    fn place_names(&self) -> nova_sim::catalog::PlaceNames {
        nova_sim::catalog::PlaceNames::default()
    }

    fn stellar_marks(&self) -> Vec<nova_sim::catalog::StellarMark> {
        Vec::new()
    }

    fn crons(&self) -> Vec<nova_sim::catalog::CronRecord> {
        Vec::new()
    }

    fn ranks(&self) -> Vec<nova_sim::catalog::RankRecord> {
        Vec::new()
    }

    fn nebulae(&self) -> Vec<nova_sim::catalog::NebulaRecord> {
        Vec::new()
    }
}

#[test]
fn a_system_holding_asteroids_fills_one_a_pass_from_the_edge() {
    let mut session = Session::start(&Pilot).expect("starts");
    assert_eq!(session.landed(), None);
    assert_eq!(session.asteroids().count(), 0);
    let mut chance = NeverFires;
    session.tick(Controls::default());
    session.tick_asteroids(&mut chance);
    let made: Vec<_> = session.asteroids().copied().collect();
    assert_eq!(made.len(), 1);
    assert_eq!(made[0].kind, 0);
    assert_eq!(made[0].strength, 40);
    for _ in 0..10 {
        session.tick(Controls::default());
        session.tick_asteroids(&mut chance);
    }
    assert!(session.asteroids().count() <= 4);
    assert!(session.asteroids().all(|asteroid| asteroid.kind == 0));
}
