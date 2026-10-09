//! The story dispatcher wired through a session, through the public API
//! alone: a subscription registered, a landing, and the settling that
//! runs its script.

use nova_sim::catalog::{DudeId, DudeRecord, FleetRecord, PersonRecord, SystemTraffic};
use nova_sim::landing::StellarFlags;
use nova_sim::story::{Consumer, EventFilter, Subscription, SubscriptionId};
use nova_sim::{
    Bit, CharacterStart, CombatCatalog, CommodityStrings, DateAffixes, DisasterRecord, GovtRecord,
    HullRecord, JunkRecord, LandOutcome, LandPress, LandingSite, NeverFires, OutfitId,
    OutfitRecord, PilotCatalog, Script, Session, ShipFields, ShipId, StarSystem, StartDate,
    StartError, StellarId, SystemId, TrafficCatalog, Vec2, WeaponRecord,
};

/// One `chär` flying an average ship from system 130, which holds one
/// landable stellar, 128, at the centre, and no traffic.
struct Home;

/// Unarmed and ungoverned.
impl CombatCatalog for Home {
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

impl PilotCatalog for Home {
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
        vec![LandingSite {
            id: StellarId(128),
            position: Vec2::ZERO,
            frame_size: Some((200, 200)),
            flags: StellarFlags::CAN_LAND,
            min_status: 0,
            landing_sound: None,
            tech_level: 0,
            special_tech: [0; 8],
            govt: None,
            flags2: 0,
            destroyed: false,
        }]
    }

    fn star_map(&self) -> Vec<StarSystem> {
        Vec::new()
    }

    fn commodity_strings(&self) -> CommodityStrings {
        CommodityStrings::default()
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
}

/// No traffic anywhere.
impl TrafficCatalog for Home {
    fn system_traffic(&self, _id: SystemId) -> Option<SystemTraffic> {
        None
    }

    fn dude(&self, _id: DudeId) -> Option<DudeRecord> {
        None
    }

    fn fleets(&self) -> Vec<FleetRecord> {
        Vec::new()
    }

    fn persons(&self) -> Vec<PersonRecord> {
        Vec::new()
    }
}

#[test]
fn a_registered_subscription_runs_its_script_as_the_landing_settles() {
    let mut session = Session::start(&Home).expect("starts");
    let on_landing = Subscription::new(
        SubscriptionId {
            consumer: Consumer::Fixture,
            record: 1,
            hook: 0,
        },
        EventFilter::Landed(StellarId(128)),
        Script::parse("b42"),
    );
    session.register_story(on_landing).expect("registers");
    let bit = Bit::new(42).expect("a bit");
    session.land().expect("selects");
    assert_eq!(
        session.land(),
        Ok(LandPress::Outcome(LandOutcome::Landed(StellarId(128))))
    );
    assert!(!session.control_bit(bit), "not before the settling");
    let _ = session.settle_script(&Home, &mut NeverFires);
    assert!(session.control_bit(bit));
}
