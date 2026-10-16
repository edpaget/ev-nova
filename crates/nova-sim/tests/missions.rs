//! A mission flown through a session, through the public API alone:
//! accepted at one stellar, carried through a jump, and completed on
//! landing at its destination, which pays it and frees its slot; and a
//! cargo mission, its cargo loaded as it is accepted and unloaded there.

use nova_sim::catalog::{AuxShip, ListedStellar, MissionId, MissionRecord, MissionShip};
use nova_sim::landing::StellarFlags;
use nova_sim::mission::Venue;
use nova_sim::{
    Bit, CharacterStart, CombatCatalog, CommodityStrings, Controls, DateAffixes, DisasterRecord,
    GovtRecord, HullRecord, JunkRecord, LandOutcome, LandPress, LandingSite, NeverFires, OutfitId,
    OutfitRecord, Pilot, PilotCatalog, Script, Session, ShipFields, ShipId, StarSystem, StartDate,
    StartError, StellarId, SystemId, Test, Vec2, WeaponRecord,
};

/// One `chär` flying a slow ship of 20 holds, a pixel a tick at most, from
/// system 130, linked with 131 to its east. Stellar 128 is in 130, out
/// past the no-jump zone; 140 is in 131, where a jump from 130 arrives.
/// Mission 140, offered at 128, goes to 140 and pays 900; its `OnSuccess`
/// sets bit 31. With a `DropOffMode`, it carries 5 tons of food, loaded as
/// it is accepted (`PickupMode` 0) and unloaded as that mode says.
struct Courier {
    drop_off: Option<i16>,
}

/// The courier carrying nothing.
const COURIER: Courier = Courier { drop_off: None };

/// No asteroids anywhere.
impl nova_sim::AsteroidCatalog for Courier {
    fn asteroid_types(&self) -> Vec<nova_sim::AsteroidTypeRecord> {
        Vec::new()
    }

    fn asteroid_fields(&self) -> Vec<nova_sim::AsteroidFieldRecord> {
        Vec::new()
    }

    fn box_frame_sizes(&self) -> [Option<(u32, u32)>; nova_sim::BOX_GRAPHICS] {
        [None; nova_sim::BOX_GRAPHICS]
    }
}

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
        tech_level: 0,
        special_tech: [0; 8],
        govt: None,
        flags2: 0,
        destroyed: false,
        graphic: 0,
        dead_graphic: None,
        dead_frame_size: None,
        type_counts_frames: true,
        exit_angle: -1,
        tribute: 0,
        animation: nova_sim::catalog::SiteAnimation::default(),
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
            holds: 20,
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
            interference: 0,
            message: -1,
        };
        vec![star(130, 0.0, 131), star(131, 600.0, 130)]
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
            cargo_type: if self.drop_off.is_some() { 0 } else { -1 },
            cargo_qty: if self.drop_off.is_some() { 5 } else { 0 },
            pickup_mode: if self.drop_off.is_some() { 0 } else { -1 },
            drop_off_mode: self.drop_off.unwrap_or(-1),
            scan_mask: 0,
            pay_val: 900,
            time_limit: 0,
            disp_weight: 0,
            on_accept: Script::default(),
            on_refuse: Script::default(),
            on_success: Script::parse("b31"),
            on_failure: Script::default(),
            on_abort: Script::default(),
            on_ship_done: Script::default(),
            can_abort: 1,
            comp_govt: None,
            comp_reward: 0,
            date_post_inc: 0,
            ships: MissionShip::NONE,
            aux: AuxShip::NONE,
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
                min_status: 0,
            })
            .collect()
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

/// A new pilot of `courier`'s, resumed on 128 through a save that says
/// so, flown, landed there and settled.
fn docked_at_128(courier: &Courier) -> Session {
    let pilot = Pilot::new(courier, "Courier").expect("starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(128);
    let saved = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(courier, saved).expect("flies");
    assert_eq!(session.landed(), None, "resumed in flight");
    assert_eq!(land(&mut session), StellarId(128));
    let _ = session.settle_script(courier, &mut NeverFires);
    session
}

/// Presses L until it lands, the first press requesting a stellar and a
/// tick between presses for the dock approach.
fn land(session: &mut Session) -> StellarId {
    for press in 0..3 {
        if press > 0 {
            session.tick(nova_sim::Controls::default());
        }
        if let Ok(LandPress::Outcome(LandOutcome::Landed(stellar))) = session.land() {
            return stellar;
        }
    }
    panic!("never landed: {:?}", session.player());
}

/// Takes `session` off from 128, jumps to 131 and lands on 140, settling
/// each step on `courier`.
fn fly_to_140(session: &mut Session, courier: &Courier) {
    session
        .take_off(courier, &mut NeverFires)
        .expect("takes off");
    let _ = session.settle_script(courier, &mut NeverFires);
    session.plot_course(SystemId(131)).expect("a route");
    session.begin_jump().expect("out past the no-jump zone");
    for _ in 0..1000 {
        if session.jumping().is_some() {
            break;
        }
        session.tick(Controls::default());
    }
    assert_eq!(
        session.arrive(courier, &mut NeverFires),
        Some(SystemId(131))
    );
    let _ = session.settle_script(courier, &mut NeverFires);
    assert_eq!(land(session), StellarId(140));
    let _ = session.settle_script(courier, &mut NeverFires);
}

#[test]
fn a_mission_accepted_carried_through_a_jump_completes_on_landing_there() {
    let mut session = docked_at_128(&COURIER);
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

    fly_to_140(&mut session, &COURIER);

    assert!(
        session.control_bit(Bit::new(31).expect("a bit")),
        "OnSuccess"
    );
    assert_eq!(session.pilot().cash(), cash + 900);
    assert_eq!(session.pilot().missions().lowest_free(), Some(0));
    assert_eq!(session.pilot().missions().iter().count(), 0);
}

#[test]
fn a_cargo_missions_cargo_loads_on_accept_and_unloads_at_its_destination() {
    for drop_off in [0, 1] {
        let courier = Courier {
            drop_off: Some(drop_off),
        };
        let mut session = docked_at_128(&courier);
        let cash = session.pilot().cash();
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        let active = session.pilot().missions().get(0).expect("accepted");
        assert!(active.aboard, "loaded at accept");
        assert_eq!(session.pilot().total_cargo(), 5, "{drop_off}");

        fly_to_140(&mut session, &courier);

        assert_eq!(session.pilot().total_cargo(), 0, "unloaded: {drop_off}");
        assert!(
            session.control_bit(Bit::new(31).expect("a bit")),
            "OnSuccess: {drop_off}"
        );
        assert_eq!(session.pilot().cash(), cash + 900, "{drop_off}");
        assert_eq!(session.pilot().missions().iter().count(), 0, "{drop_off}");
    }
}
