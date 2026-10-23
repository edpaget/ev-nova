//! The story dispatcher wired through a session, through the public API
//! alone: a subscription registered, a landing, and the settling that
//! runs its script; and a stellar shot down, whose `OnDestroy` a later
//! dispatch sees.

use nova_sim::catalog::{DudeId, DudeRecord, FleetRecord, PersonRecord, SystemTraffic};
use nova_sim::combat::Rules;
use nova_sim::combat::armament::{MOD_WEAPON, Trigger};
use nova_sim::landing::StellarFlags;
use nova_sim::story::{Consumer, EventFilter, Subscription, SubscriptionId};
use nova_sim::{
    Bit, CharacterStart, CombatCatalog, CommodityStrings, DateAffixes, DisasterRecord, GovtRecord,
    HullRecord, JunkRecord, LandOutcome, LandPress, LandingSite, NeverFires, OutfitId,
    OutfitRecord, PilotCatalog, Script, Session, ShipFields, ShipId, StarSystem, StartDate,
    StartError, StellarId, StellarLife, SystemId, Test, TrafficCatalog, Vec2, WeaponId,
    WeaponRecord,
};

/// One `chär` flying an average ship from system 130, which holds one
/// landable stellar, 128, at the centre, and no traffic. The ship is
/// unarmed unless it is `armed` with outfit 250, a planet-type gun, and
/// the stellars live as `lives` say.
#[derive(Default)]
struct Home {
    armed: bool,
    lives: Vec<StellarLife>,
}

/// No asteroids anywhere.
impl nova_sim::AsteroidCatalog for Home {
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

/// Ungoverned; its one weapon, `wëap` 128, a planet-type gun firing
/// every tick and doing 60 damage.
impl CombatCatalog for Home {
    fn weapons(&self) -> Vec<WeaponRecord> {
        vec![WeaponRecord {
            id: WeaponId(128),
            reload: 0,
            count: 30,
            mass_dmg: 40,
            energy_dmg: 20,
            guidance: -1,
            speed: 1500,
            ammo_type: -1,
            graphic: 0,
            inaccuracy: 0,
            impact: 0,
            explod_type: -1,
            prox_radius: 0,
            blast_radius: 0,
            flags: 0,
            seeker: 0,
            flags2: 0x0400,
            flags3: 0,
            decay: 0,
            beam_length: 0,
            beam_width: 0,
            falloff: 0,
            li_density: 0,
            burst_count: 0,
            burst_reload: 0,
            guided_turn: 0,
            durability: 0,
            sub_count: 0,
            sub_type: None,
            sub_theta: 0,
            sub_limit: 0,
            max_ammo: 0,
            prox_safety: 0,
            recoil: 0,
            exit_type: -1,
            ionization: 0,
            ionize_color: 0,
            jam_vuln: [0; 4],
            frame: None,
            hit_particles: 0,
            hit_part_life: 0,
            hit_part_vel: 0,
            hit_part_color: 0,
        }]
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
        if self.armed {
            vec![(OutfitId(250), 1)]
        } else {
            Vec::new()
        }
    }

    fn outfits(&self) -> Vec<OutfitRecord> {
        vec![OutfitRecord {
            id: OutfitId(250),
            name: "Gun".to_owned(),
            short_name: "Gun".to_owned(),
            disp_weight: 0,
            mass: 1,
            tech_level: 1,
            max: 10,
            flags: 0,
            cost: 1000,
            mods: [(MOD_WEAPON, 128), (0, 0), (0, 0), (0, 0)],
            contribute: 0,
            require: 0,
            require_govt: -1,
            buy_random: 100,
            availability: Test::default(),
            item_class: 0,
            lc_name: "gun".to_owned(),
            lc_plural: "guns".to_owned(),
            scan_mask: 0,
            on_purchase: Script::default(),
            on_sell: Script::default(),
        }]
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
            tech_level: 0,
            special_tech: [0; 8],
            govt: None,
            flags2: 0,
            destroyed: false,
            graphic: 0,
            dead_graphic: None,
            dead_frame_size: None,
            type_counts_frames: true,
            dead_counts_frames: true,
            exit_angle: -1,
            tribute: 0,
            gravity: 0,
            fee: 0,
            animation: nova_sim::catalog::SiteAnimation::default(),
        }]
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

    fn stellar_weapons(&self) -> Vec<(StellarId, WeaponId)> {
        Vec::new()
    }

    fn date_affixes(&self) -> DateAffixes {
        DateAffixes::default()
    }

    fn gate_sites(&self) -> Vec<nova_sim::GateSite> {
        Vec::new()
    }

    fn stellar_lives(&self) -> Vec<StellarLife> {
        self.lives.clone()
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

    /// System 130's stellars, as it lists them.
    fn listed_stellars(&self) -> Vec<nova_sim::catalog::ListedStellar> {
        let sites = self.landing_sites(SystemId(130));
        nova_sim::fixture::listed_stellars([(SystemId(130), sites.as_slice())])
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
    let mut session = Session::start(&Home::default()).expect("starts");
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
    session.tick(nova_sim::Controls::default());
    assert_eq!(
        session.land(),
        Ok(LandPress::Outcome(LandOutcome::Landed(StellarId(128))))
    );
    assert!(!session.control_bit(bit), "not before the settling");
    let _ = session.settle_script(&Home::default(), &mut NeverFires);
    assert!(session.control_bit(bit));
}

#[test]
fn a_stellar_shot_down_runs_its_on_destroy_whose_bit_a_later_dispatch_sees() {
    let home = Home {
        armed: true,
        lives: vec![StellarLife {
            id: StellarId(128),
            strength: 100,
            dead_time: 3,
            flags2: 0,
            explosion: None,
            on_destroy: Script::parse("b7 M130"),
            on_regen: Script::default(),
        }],
    };
    let mut session = Session::start(&home).expect("starts");
    session
        .register_story(Subscription {
            guard: Test::parse("b7"),
            ..Subscription::new(
                SubscriptionId {
                    consumer: Consumer::Fixture,
                    record: 1,
                    hook: 0,
                },
                EventFilter::EnteredSystem(SystemId(130)),
                Script::parse("b8"),
            )
        })
        .expect("registers");
    session.hold_trigger(Trigger {
        primary: true,
        secondary: None,
        only: None,
        bays: false,
        bays_after: false,
        point_defence: false,
        keep: None,
    });
    for _ in 0..2 {
        session.tick_combat(Rules::default(), &mut NeverFires);
    }
    // The second shot's first move is tested in the next tick's hit pass.
    session.hold_trigger(Trigger::default());
    session.tick_combat(Rules::default(), &mut NeverFires);
    assert!(
        session.galaxy().destroyed(StellarId(128)),
        "60 twice: shot down"
    );
    let (b7, b8) = (Bit::new(7).expect("a bit"), Bit::new(8).expect("a bit"));
    assert!(session.control_bit(b7), "its OnDestroy ran");
    assert!(!session.control_bit(b8), "its move waits for the settling");
    let _ = session.settle_script(&home, &mut NeverFires);
    assert!(session.control_bit(b8), "the arrival's dispatch saw b7");
}
