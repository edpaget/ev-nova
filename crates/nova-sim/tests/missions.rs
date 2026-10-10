//! A mission flown through a session, through the public API alone:
//! accepted at one stellar, carried through a jump, and completed on
//! landing at its destination, which pays it and frees its slot.

use nova_sim::catalog::{ListedStellar, MissionId, MissionRecord, MissionShip};
use nova_sim::landing::StellarFlags;
use nova_sim::mission::Venue;
use nova_sim::{
    Bit, CharacterStart, CombatCatalog, CommodityStrings, Controls, DateAffixes, DisasterRecord,
    GovtRecord, HullRecord, JunkRecord, LandOutcome, LandPress, LandingSite, NeverFires, OutfitId,
    OutfitRecord, Pilot, PilotCatalog, Script, Session, ShipFields, ShipId, StarSystem, StartDate,
    StartError, StellarId, SystemId, Test, Vec2, WeaponRecord,
};

/// One `chär` flying a slow ship, a pixel a tick at most, from system 130,
/// linked with 131 to its east. Stellar 128 is in 130, out past the
/// no-jump zone; 140 is in 131, where a jump from 130 arrives. Mission
/// 140, offered at 128, goes to 140 and pays 900; its `OnSuccess` sets
/// bit 31.
struct Courier;

/// Unarmed and ungoverned.
impl CombatCatalog for Courier {
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

/// Landable stellar `id` at (`x`, 0), 100 wide.
fn planet(id: i16, x: f32) -> LandingSite {
    LandingSite {
        id: StellarId(id),
        position: Vec2::new(x, 0.0),
        frame_size: Some((100, 100)),
        flags: StellarFlags::CAN_LAND,
        min_status: 0,
        landing_sound: None,
        tech_level: 0,
        special_tech: [0; 8],
        govt: None,
        flags2: 0,
        destroyed: false,
        graphic: 0,
        dead_graphic: None,
        dead_frame_size: None,
        type_counts_frames: true,
    }
}

/// The system of stellar `id`: 128's is 130, 140's 131.
fn system_of(id: StellarId) -> SystemId {
    if id == StellarId(128) {
        SystemId(130)
    } else {
        SystemId(131)
    }
}

impl PilotCatalog for Courier {
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
            speed: 100,
            accel: 900,
            maneuver: 30,
            fuel: 300,
            fuel_regen: 1,
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
        matches!(id, SystemId(130 | 131))
    }

    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
        match system {
            SystemId(130) => vec![planet(128, 1100.0)],
            SystemId(131) => vec![planet(140, -1000.0)],
            _ => Vec::new(),
        }
    }

    fn star_map(&self) -> Vec<StarSystem> {
        let star = |id, x, link| StarSystem {
            id: SystemId(id),
            position: Vec2::new(x, 0.0),
            links: vec![SystemId(link)],
            govt: None,
            stellars: Vec::new(),
        };
        vec![star(130, 0.0, 131), star(131, 600.0, 130)]
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

    fn fallback_frame_size(&self) -> Option<(u32, u32)> {
        Some((100, 100))
    }

    fn missions(&self) -> Vec<MissionRecord> {
        vec![MissionRecord {
            id: MissionId(140),
            avail_stel: 128,
            avail_loc: 0,
            avail_record: 0,
            avail_rating: 0,
            avail_random: 100,
            avail_ship_type: 0,
            avail_bits: Test::default(),
            require: 0,
            flags: 0,
            flags2: 0,
            travel_stel: 140,
            return_stel: -1,
            cargo_type: -1,
            cargo_qty: 0,
            pickup_mode: -1,
            drop_off_mode: -1,
            scan_mask: 0,
            pay_val: 900,
            time_limit: 0,
            disp_weight: 0,
            on_accept: Script::default(),
            on_refuse: Script::default(),
            on_success: Script::parse("b31"),
            on_failure: Script::default(),
            on_abort: Script::default(),
            can_abort: 1,
            comp_govt: None,
            comp_reward: 0,
            date_post_inc: 0,
            ships: MissionShip { count: 0, goal: -1 },
        }]
    }

    fn mission_texts(&self) -> Vec<nova_sim::catalog::MissionTexts> {
        Vec::new()
    }

    fn listed_stellars(&self) -> Vec<ListedStellar> {
        [StellarId(128), StellarId(140)]
            .into_iter()
            .map(|id| ListedStellar {
                id,
                systems: vec![system_of(id)],
                flags: StellarFlags::CAN_LAND,
                flags2: 0,
                govt: None,
            })
            .collect()
    }

    fn place_names(&self) -> nova_sim::catalog::PlaceNames {
        nova_sim::catalog::PlaceNames::default()
    }
}

/// No traffic anywhere.
impl nova_sim::TrafficCatalog for Courier {
    fn system_traffic(&self, _id: SystemId) -> Option<nova_sim::catalog::SystemTraffic> {
        None
    }

    fn dude(&self, _id: nova_sim::catalog::DudeId) -> Option<nova_sim::catalog::DudeRecord> {
        None
    }

    fn fleets(&self) -> Vec<nova_sim::catalog::FleetRecord> {
        Vec::new()
    }

    fn persons(&self) -> Vec<nova_sim::catalog::PersonRecord> {
        Vec::new()
    }
}

/// A new pilot, docked at 128 through a save that says so, flown and
/// settled.
fn docked_at_128() -> Session {
    let pilot = Pilot::new(&Courier, "Courier").expect("starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(128);
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(&Courier, docked).expect("flies");
    assert_eq!(session.landed(), Some(StellarId(128)));
    let _ = session.settle_script(&Courier, &mut NeverFires);
    session
}

/// Presses L until it lands, the first press selecting a stellar.
fn land(session: &mut Session) -> StellarId {
    for _ in 0..2 {
        if let Ok(LandPress::Outcome(LandOutcome::Landed(stellar))) = session.land() {
            return stellar;
        }
    }
    panic!("never landed: {:?}", session.player());
}

#[test]
fn a_mission_accepted_carried_through_a_jump_completes_on_landing_there() {
    let mut session = docked_at_128();
    let offers = session.mission_offers(Venue::Computer);
    assert_eq!(offers.len(), 1);
    assert_eq!(
        offers[0]
            .destination
            .map(|stop| (stop.stellar, stop.system)),
        Some((StellarId(140), Some(SystemId(131))))
    );
    let cash = session.pilot().cash();
    assert_eq!(
        session.accept_mission(MissionId(140), &mut NeverFires),
        Ok(0)
    );
    assert_eq!(
        session
            .pilot()
            .missions()
            .get(0)
            .map(|active| active.mission),
        Some(MissionId(140))
    );

    session.take_off(&mut NeverFires).expect("takes off");
    let _ = session.settle_script(&Courier, &mut NeverFires);
    session.plot_course(SystemId(131)).expect("a route");
    session.begin_jump().expect("out past the no-jump zone");
    for _ in 0..1000 {
        if session.jumping().is_some() {
            break;
        }
        session.tick(Controls::default());
    }
    assert_eq!(
        session.arrive(&Courier, &mut NeverFires),
        Some(SystemId(131))
    );
    let _ = session.settle_script(&Courier, &mut NeverFires);
    assert_eq!(land(&mut session), StellarId(140));
    let _ = session.settle_script(&Courier, &mut NeverFires);

    assert!(
        session.control_bit(Bit::new(31).expect("a bit")),
        "OnSuccess"
    );
    assert_eq!(session.pilot().cash(), cash + 900);
    assert_eq!(session.pilot().missions().lowest_free(), Some(0));
    assert_eq!(session.pilot().missions().iter().count(), 0);
}
