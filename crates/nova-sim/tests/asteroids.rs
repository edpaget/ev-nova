//! An asteroid field through a flight session, headless: a system holding
//! asteroids, entered through the real session construction, fills from
//! the field's edge one asteroid a pass, each of a type its `AstTypes`
//! sets, and the player's gun breaks them into boxes and fragments.

use nova_sim::combat::Rules;
use nova_sim::combat::armament::{MOD_WEAPON, Trigger};
use nova_sim::{
    AsteroidCatalog, AsteroidFieldRecord, AsteroidTypeRecord, CharacterStart, CombatCatalog,
    CombatEvent, CommodityStrings, Controls, DateAffixes, DisasterRecord, GovtRecord, HullRecord,
    JunkRecord, LandingSite, NeverFires, OutfitId, OutfitRecord, PilotCatalog, Script, Session,
    ShipFields, ShipId, StarSystem, StartDate, StartError, StellarId, SystemId, Test, Turn,
    WeaponId, WeaponRecord,
};

/// One `chär` flying an average ship from system 130, which has no
/// stellars and holds 4 asteroids of type 0, `röid` 128; armed with
/// outfit 250's gun when `armed`.
struct Pilot {
    armed: bool,
}

/// Ungoverned; its one weapon, `wëap` 128, a gun 20 pixels a tick for 40
/// ticks, every 5, doing 100 mass damage.
impl CombatCatalog for Pilot {
    fn weapons(&self) -> Vec<WeaponRecord> {
        vec![WeaponRecord {
            id: WeaponId(128),
            reload: 5,
            count: 40,
            mass_dmg: 100,
            energy_dmg: 0,
            guidance: -1,
            speed: 2000,
            ammo_type: -1,
            inaccuracy: 0,
            impact: 0,
            explod_type: -1,
            prox_radius: 0,
            blast_radius: 0,
            flags: 0,
            seeker: 0,
            flags2: 0,
            flags3: 0,
            decay: 0,
            beam_length: 0,
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

/// `röid` 128, of `Strength` 40 and `SpinRate` 150, its sprite 36 frames
/// of 96 x 96, yielding 3 boxes of commodity 2 on average, breaking into
/// 2 of `röid` 129 on average and exploding as `bööm` 128; `röid` 129;
/// system 130's field, 4 of type 0.
impl AsteroidCatalog for Pilot {
    fn asteroid_types(&self) -> Vec<AsteroidTypeRecord> {
        vec![
            AsteroidTypeRecord {
                kind: 0,
                strength: 40,
                spin_rate: 150,
                frames: 36,
                frame_size: Some((96, 96)),
                yield_type: 2,
                yield_qty: 3,
                frag_types: [129, -1],
                frag_count: 2,
                explode_type: 0,
                ..AsteroidTypeRecord::default()
            },
            AsteroidTypeRecord {
                kind: 1,
                strength: 10,
                frames: 36,
                frame_size: Some((24, 24)),
                ..AsteroidTypeRecord::default()
            },
        ]
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
    let mut session = Session::start(&Pilot { armed: false }).expect("starts");
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

#[test]
fn the_players_gun_breaks_an_asteroid_into_scoopable_boxes_a_fragment_and_its_explosion() {
    let mut session = Session::start(&Pilot { armed: true }).expect("starts");
    // An edge spawn drawing nothing lies 0.7 × 431 out on both axes,
    // down and to the right, at rest: turn to face it, within the 6
    // degrees its 48-pixel radius spans 426 pixels off.
    let turning = |turn| Controls {
        turn,
        ..Controls::default()
    };
    for _ in 0..200 {
        let heading = session.player().heading;
        if heading < 132.0 {
            session.tick(turning(Turn::Right));
        } else if heading > 138.0 {
            session.tick(turning(Turn::Left));
        } else {
            break;
        }
    }
    let heading = session.player().heading;
    assert!((132.0..=138.0).contains(&heading), "{heading}");
    session.hold_trigger(Trigger {
        primary: true,
        secondary: None,
        only: None,
        turrets_only: false,
        bays: false,
        point_defence: false,
    });
    let mut events = Vec::new();
    for _ in 0..40 {
        session.tick_combat(Rules::default(), &mut NeverFires);
        events.extend(session.take_combat_events());
        if !session.cargo_boxes().is_empty() {
            break;
        }
    }
    let boxes = session.cargo_boxes();
    assert!(!boxes.is_empty(), "an asteroid broke: {events:?}");
    assert!(
        boxes
            .iter()
            .all(|made| made.scoopable && made.contents == Some(2) && made.graphic == 1),
        "{boxes:?}"
    );
    assert!(
        events.iter().any(|event| matches!(
            event,
            CombatEvent::Exploded { explosion, .. } if explosion.boom == nova_sim::BoomId(128)
        )),
        "{events:?}"
    );
    assert!(
        session.asteroids().any(|asteroid| asteroid.kind == 1),
        "a fragment of röid 129"
    );
}
