//! A canned [`PilotCatalog`], [`TrafficCatalog`], [`CombatCatalog`] and
//! [`CommCatalog`] for the crate's own tests.

use std::cell::RefCell;

use crate::ai::Goal;
use crate::catalog::{
    CharacterStart, CombatCatalog, CommCatalog, CommodityStrings, DisasterRecord, DudeId,
    DudeRecord, FleetRecord, GovtId, GovtRecord, HullRecord, JunkRecord, LandingSite, OutfitId,
    OutfitRecord, Penalties, PersonId, PersonRecord, PilotCatalog, ShipId, ShipRecord, StarSystem,
    StartDate, StartError, StellarId, SystemId, SystemTraffic, TrafficCatalog, WeaponId,
    WeaponRecord,
};
use crate::chance::{Chance, NeverFires};
use crate::combat::armament::{Armament, Trigger};
use crate::combat::hull::{Condition, HullSpec};
use crate::flight::ShipState;
use crate::flight::{Controls, Turn};
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::hyperspace::MIN_JUMP_DISTANCE;
use crate::landing::StellarFlags;
use crate::session::Session;
use crate::stats::ShipStats;
use crate::traffic::npc::{AiType, Mode, Npc, NpcId};

/// A canned first `chär`, ships and systems; records the ships asked
/// for.
pub(crate) struct FakePilotCatalog {
    pub(crate) character: Result<CharacterStart, StartError>,
    pub(crate) ships: Vec<(ShipId, Result<ShipFields, String>)>,
    pub(crate) systems: Vec<SystemId>,
    pub(crate) ships_asked: RefCell<Vec<ShipId>>,
    /// Each system's landing sites; any other has none.
    pub(crate) sites: Vec<(SystemId, Vec<LandingSite>)>,
    pub(crate) sites_asked: RefCell<Vec<SystemId>>,
    pub(crate) star_map: Vec<StarSystem>,
    pub(crate) star_map_reads: RefCell<usize>,
    /// Each ship's default items; any other ship has none.
    pub(crate) defaults: Vec<(ShipId, Vec<(OutfitId, u16)>)>,
    pub(crate) defaults_asked: RefCell<Vec<ShipId>>,
    /// Every `oütf`.
    pub(crate) outfits: Vec<OutfitRecord>,
    /// How many times the outfits were read.
    pub(crate) outfit_reads: RefCell<usize>,
    /// Every `shïp`, as the shipyard reads them.
    pub(crate) ship_records: Vec<ShipRecord>,
    /// How many times the ship records were read.
    pub(crate) ship_record_reads: RefCell<usize>,
    pub(crate) commodities: CommodityStrings,
    pub(crate) junk: Vec<JunkRecord>,
    pub(crate) disasters: Vec<DisasterRecord>,
    /// How many times the goods (commodities, `jünk` and `öops`) were read.
    pub(crate) goods_reads: RefCell<usize>,
    /// Each system's traffic; any other has none.
    pub(crate) traffic: Vec<(SystemId, SystemTraffic)>,
    /// Every `düde`.
    pub(crate) dudes: Vec<(DudeId, DudeRecord)>,
    /// Every `flët`.
    pub(crate) fleets: Vec<FleetRecord>,
    /// Every `përs`.
    pub(crate) persons: Vec<PersonRecord>,
    /// Every `wëap`.
    pub(crate) weapons: Vec<WeaponRecord>,
    /// Every `shïp`'s combat fields.
    pub(crate) hulls: Vec<HullRecord>,
    /// Every `gövt`.
    pub(crate) govts: Vec<GovtRecord>,
    /// Each `STR#`; any other is missing.
    pub(crate) strings: Vec<(i16, Vec<String>)>,
}

pub(crate) const FAST: ShipFields = ShipFields {
    speed: 600,
    accel: 900,
    maneuver: 30,
    shield: 30,
    armor: 45,
    fuel: 300,
    fuel_regen: 0,
    holds: 20,
    mass: 40,
    free_mass: 30,
    contribute: 0x1,
    shield_rech: 0,
    armor_rech: 0,
};

/// A landable planet at (`x`, `y`), 100 x 100 (radius 50).
pub(crate) fn planet(id: i16, x: f32, y: f32) -> LandingSite {
    LandingSite {
        id: StellarId(id),
        position: Vec2::new(x, y),
        frame_size: Some((100, 100)),
        flags: StellarFlags::CAN_LAND,
        min_status: 0,
        landing_sound: None,
        tech_level: 0,
        special_tech: [0; 8],
        govt: None,
    }
}

/// `oütf` `id` with these mods (the unused ones none): tech level 1, a
/// ton, 1000 credits, up to 10 owned, no flags, requiring nothing.
pub(crate) fn outfit(id: i16, mods: &[(i16, i16)]) -> OutfitRecord {
    let mut pairs = [(0, 0); 4];
    pairs[..mods.len()].copy_from_slice(mods);
    OutfitRecord {
        id: OutfitId(id),
        name: format!("Outfit {id}"),
        short_name: format!("Outfit\\n{id}"),
        disp_weight: 0,
        mass: 1,
        tech_level: 1,
        max: 10,
        flags: 0,
        cost: 1000,
        mods: pairs,
        contribute: 0,
        require: 0,
        require_govt: -1,
        availability: String::new(),
    }
}

/// `shïp` `id` with `fields`: 10,000 credits, tech level 1, for sale
/// every day (`BuyRandom` 100), never for hire (`HireRandom` 0),
/// requiring nothing, with no default items, no `Flags3` and no inherent
/// government, hailed as "ship `id`", a wimpy trader whose escort class
/// is worked out (`EscortType` -1).
pub(crate) fn ship(id: i16, fields: ShipFields) -> ShipRecord {
    ShipRecord {
        id: ShipId(id),
        name: format!("Ship {id}"),
        short_name: format!("Ship\\n{id}"),
        long_name: format!("The Ship {id}"),
        fields,
        defaults: Vec::new(),
        cost: 10_000,
        tech_level: 1,
        buy_random: 100,
        hire_random: 0,
        require: 0,
        availability: String::new(),
        flags3: 0,
        disp_weight: 0,
        max_gun: 2,
        max_tur: 1,
        length: 20,
        crew: 3,
        inherent_ai: 1,
        comm_name: format!("ship {id}"),
        inherent_govt: None,
        escort_type: -1,
    }
}

/// `wëap` `id`: an unguided weapon with unlimited ammo that does
/// nothing, with every other field none or unused (-1).
pub(crate) fn weapon(id: i16) -> WeaponRecord {
    WeaponRecord {
        id: WeaponId(id),
        reload: 0,
        count: 0,
        mass_dmg: 0,
        energy_dmg: 0,
        guidance: -1,
        speed: 0,
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
    }
}

/// `shïp` `id`'s combat fields: no flags, no weapons, no `shän`, no
/// explosions, gone at once, massless and of no strength.
pub(crate) fn hull(id: i16) -> HullRecord {
    HullRecord {
        id: ShipId(id),
        flags: 0,
        death_delay: 0,
        explode1: -1,
        explode2: -1,
        mass: 0,
        weapons: Vec::new(),
        size: None,
        strength: 0,
    }
}

/// `gövt` `id`: no flags, no classes, allies or enemies, no tolerance
/// for crime, no penalties, and `MaxOdds` 100 (even odds), hailed as
/// "Govt `id`".
pub(crate) fn govt(id: i16) -> GovtRecord {
    GovtRecord {
        id: GovtId(id),
        flags: 0,
        flags2: 0,
        crime_tol: 0,
        penalties: Penalties::default(),
        max_odds: 100,
        classes: [-1; 4],
        allies: [-1; 4],
        enemies: [-1; 4],
        comm_name: format!("Govt {id}"),
    }
}

/// `përs` `id`, "Person `id`", flying ship `ship`: a warship of no
/// government and `Aggress` 2 that may appear anywhere (`LinkSyst` -1),
/// with no `Coward`, extra weapons, credits, `ShieldMod`, quotes,
/// picture, mission, flags or `ActiveOn`.
pub(crate) fn person(id: i16, ship: i16) -> PersonRecord {
    PersonRecord {
        id: PersonId(id),
        name: format!("Person {id}"),
        link_syst: -1,
        govt: None,
        ai_type: 3,
        aggress: 2,
        coward: 0,
        ship: Some(ShipId(ship)),
        weapons: Vec::new(),
        credits: 0,
        shield_mod: 0,
        hail_pict: None,
        comm_quote: -1,
        hail_quote: -1,
        link_mission: None,
        flags: 0,
        active_on: String::new(),
        subtitle: String::new(),
        flags2: 0,
    }
}

/// NPC `id`, a wimpy trader of ship type 128 performing as `stats`, its
/// reserves full, at rest at the centre facing up, flying and idle,
/// intact and unarmed.
pub(crate) fn npc(id: u32, stats: ShipStats) -> Npc {
    Npc {
        id: NpcId(id),
        ship: ShipId(128),
        govt: None,
        ai_type: AiType::WimpyTrader,
        leader: None,
        class: crate::escort::EscortClass::Freighter,
        escort: None,
        stats,
        reserves: stats.full(),
        state: ShipState::default(),
        mode: Mode::Flying,
        goal: Goal::Idle,
        condition: Condition::Intact,
        hull: HullSpec::default(),
        armament: Armament::default(),
        rounds: std::collections::BTreeMap::new(),
        trigger: Trigger::default(),
        target: None,
        provoked: 0.0,
        aggression: 0,
        inspected: None,
        booty: 0,
        boarded: false,
        info_types: 0,
        spared: false,
        assisting: 0,
        carrier: None,
        person: None,
    }
}

/// The first `chär` flies ship 128 from system 130 on 23 June 1177;
/// ship 128 is fast, and systems 130 and 131 exist. System 130 holds a
/// planet, 128, at (30, -40), which the ship starts over, and another,
/// 129, far away; system 131 holds one at the centre. On the map, 130
/// at (0, 0), 131 at (600, 0) and 132 at (600, 600) are linked in a
/// line, and 133 is linked to none.
pub(crate) fn catalog() -> FakePilotCatalog {
    FakePilotCatalog {
        character: Ok(CharacterStart {
            ship: Some(ShipId(128)),
            systems: [Some(SystemId(130)), None, None, None],
            start: START,
            ..CharacterStart::default()
        }),
        ships: vec![(ShipId(128), Ok(FAST))],
        systems: vec![SystemId(130), SystemId(131)],
        ships_asked: RefCell::default(),
        sites: vec![
            (
                SystemId(130),
                vec![planet(128, 30.0, -40.0), planet(129, 2000.0, 0.0)],
            ),
            (SystemId(131), vec![planet(140, 0.0, 0.0)]),
        ],
        sites_asked: RefCell::default(),
        star_map: vec![
            star(130, (0.0, 0.0), &[131]),
            star(131, (600.0, 0.0), &[132]),
            star(132, (600.0, 600.0), &[]),
            star(133, (-600.0, 0.0), &[]),
        ],
        star_map_reads: RefCell::default(),
        defaults: Vec::new(),
        defaults_asked: RefCell::default(),
        outfits: Vec::new(),
        outfit_reads: RefCell::default(),
        ship_records: Vec::new(),
        ship_record_reads: RefCell::default(),
        commodities: CommodityStrings::default(),
        junk: Vec::new(),
        disasters: Vec::new(),
        goods_reads: RefCell::default(),
        traffic: Vec::new(),
        dudes: Vec::new(),
        fleets: Vec::new(),
        persons: Vec::new(),
        weapons: Vec::new(),
        hulls: Vec::new(),
        govts: Vec::new(),
        strings: Vec::new(),
    }
}

pub(crate) const START: StartDate = StartDate {
    day: 23,
    month: 6,
    year: 1177,
};

pub(crate) fn star(id: i16, (x, y): (f32, f32), links: &[i16]) -> StarSystem {
    StarSystem {
        id: SystemId(id),
        position: Vec2::new(x, y),
        links: links.iter().copied().map(SystemId).collect(),
        govt: None,
    }
}

pub(crate) fn starting(systems: [Option<i16>; 4]) -> FakePilotCatalog {
    FakePilotCatalog {
        character: Ok(CharacterStart {
            ship: Some(ShipId(128)),
            systems: systems.map(|slot| slot.map(SystemId)),
            start: START,
            ..CharacterStart::default()
        }),
        ..catalog()
    }
}

impl PilotCatalog for FakePilotCatalog {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        self.character.clone()
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        self.ships_asked.borrow_mut().push(id);
        self.ships
            .iter()
            .find(|(ship, _)| *ship == id)
            .map_or_else(|| Err(format!("no shïp {}", id.0)), |(_, f)| f.clone())
    }

    fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
        self.defaults_asked.borrow_mut().push(id);
        self.defaults
            .iter()
            .find(|(ship, _)| *ship == id)
            .map_or_else(Vec::new, |(_, items)| items.clone())
    }

    fn outfits(&self) -> Vec<OutfitRecord> {
        *self.outfit_reads.borrow_mut() += 1;
        self.outfits.clone()
    }

    fn ships(&self) -> Vec<ShipRecord> {
        *self.ship_record_reads.borrow_mut() += 1;
        self.ship_records.clone()
    }

    fn system_exists(&self, id: SystemId) -> bool {
        self.systems.contains(&id)
    }

    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
        self.sites_asked.borrow_mut().push(system);
        self.sites
            .iter()
            .find(|(id, _)| *id == system)
            .map_or_else(Vec::new, |(_, sites)| sites.clone())
    }

    fn star_map(&self) -> Vec<StarSystem> {
        *self.star_map_reads.borrow_mut() += 1;
        self.star_map.clone()
    }

    fn commodity_strings(&self) -> CommodityStrings {
        *self.goods_reads.borrow_mut() += 1;
        self.commodities.clone()
    }

    fn junk(&self) -> Vec<JunkRecord> {
        *self.goods_reads.borrow_mut() += 1;
        self.junk.clone()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        *self.goods_reads.borrow_mut() += 1;
        self.disasters.clone()
    }
}

/// No traffic unless a test sets it.
impl TrafficCatalog for FakePilotCatalog {
    fn system_traffic(&self, id: SystemId) -> Option<SystemTraffic> {
        self.traffic
            .iter()
            .find(|(system, _)| *system == id)
            .map(|(_, traffic)| *traffic)
    }

    fn dude(&self, id: DudeId) -> Option<DudeRecord> {
        self.dudes
            .iter()
            .find(|(dude, _)| *dude == id)
            .map(|(_, record)| record.clone())
    }

    fn fleets(&self) -> Vec<FleetRecord> {
        self.fleets.clone()
    }

    fn persons(&self) -> Vec<PersonRecord> {
        self.persons.clone()
    }
}

/// Unarmed and ungoverned, unless a test sets weapons, hulls and
/// governments.
impl CombatCatalog for FakePilotCatalog {
    fn weapons(&self) -> Vec<WeaponRecord> {
        self.weapons.clone()
    }

    fn hulls(&self) -> Vec<HullRecord> {
        self.hulls.clone()
    }

    fn governments(&self) -> Vec<GovtRecord> {
        self.govts.clone()
    }
}

/// No strings unless a test sets them.
impl CommCatalog for FakePilotCatalog {
    fn string_list(&self, id: i16) -> Vec<String> {
        self.strings
            .iter()
            .find(|(list, _)| *list == id)
            .map_or_else(Vec::new, |(_, strings)| strings.clone())
    }
}

/// Flies the ship out from the centre until it is at least
/// [`MIN_JUMP_DISTANCE`] away: it first turns to face away from the
/// centre (Down faces against its motion), then thrusts.
pub(crate) fn fly_out(session: &mut Session) {
    const THRUST: Controls = Controls {
        thrust: true,
        turn: Turn::None,
        reverse: false,
    };
    for _ in 0..3000 {
        let player = *session.player();
        if player.position.length() >= MIN_JUMP_DISTANCE {
            return;
        }
        let outward = if player.position.length() > 0.0 {
            crate::flight::heading_of(player.position)
        } else {
            0.0
        };
        let off = crate::flight::shortest_turn(player.heading, outward).abs();
        // Down lands exactly on the heading against the motion, which
        // is outward while the ship drifts in.
        let controls = if off < 1e-3 {
            THRUST
        } else if player.velocity.length() > crate::flight::AT_REST_SPEED {
            Controls {
                reverse: true,
                ..Controls::default()
            }
        } else {
            Controls {
                turn: Turn::Right,
                ..Controls::default()
            }
        };
        session.tick(controls);
    }
    panic!("never got out: {:?}", session.player());
}

/// Plots a course to `to`, flies out and jumps, and arrives, no chance
/// firing on the way.
pub(crate) fn jump(session: &mut Session, catalog: &FakePilotCatalog, to: i16) -> Option<SystemId> {
    jump_with(session, catalog, to, &mut NeverFires)
}

/// Plots a course to `to`, flies out and jumps, and arrives, the day's
/// chances rolled on `chance`.
pub(crate) fn jump_with(
    session: &mut Session,
    catalog: &FakePilotCatalog,
    to: i16,
    chance: &mut impl Chance,
) -> Option<SystemId> {
    if session.course().last() != Some(&SystemId(to)) {
        session.plot_course(SystemId(to)).expect("a route");
    }
    fly_out(session);
    session.begin_jump().expect("jumps");
    session.arrive(catalog, chance)
}

/// A [`Chance`] that answers from a script (no once it runs out) and
/// records each percent it is asked.
#[derive(Debug, Default)]
pub(crate) struct Scripted {
    answers: Vec<bool>,
    pub(crate) asked: Vec<u8>,
}

impl Scripted {
    /// Answers `answers`, in order, then no.
    pub(crate) fn answering(answers: &[bool]) -> Self {
        Self {
            answers: answers.iter().rev().copied().collect(),
            asked: Vec::new(),
        }
    }
}

impl Chance for Scripted {
    fn fires(&mut self, percent: u8) -> bool {
        self.asked.push(percent);
        self.answers.pop().unwrap_or(false)
    }

    /// The last outcome, as [`NeverFires`] draws: no roll fires.
    fn below(&mut self, n: u32) -> u32 {
        NeverFires.below(n)
    }
}

/// A [`Chance`] whose draws come from a queue (the last outcome, `n - 1`,
/// once it runs out, so no roll fires) and that records each `n` it is
/// asked. It never fires a percentage.
#[derive(Debug, Default)]
pub(crate) struct Draws {
    queue: std::collections::VecDeque<u32>,
    pub(crate) asked: Vec<u32>,
}

impl Draws {
    /// Draws `draws`, in order, then the last outcome.
    pub(crate) fn of(draws: &[u32]) -> Self {
        Self {
            queue: draws.iter().copied().collect(),
            asked: Vec::new(),
        }
    }
}

impl Chance for Draws {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    /// The next draw queued; one outside `0..n` is a mistake in the test.
    fn below(&mut self, n: u32) -> u32 {
        self.asked.push(n);
        match self.queue.pop_front() {
            Some(draw) => {
                assert!(draw < n, "drew {draw} below {n}");
                draw
            }
            None => n.saturating_sub(1),
        }
    }
}

/// A slow ship, 1 pixel a tick at most, gaining a unit a tick: it
/// arrives slow enough to land, over planet 140 at 131's edge.
pub(crate) fn edge_lander() -> FakePilotCatalog {
    FakePilotCatalog {
        ships: vec![(
            ShipId(128),
            Ok(ShipFields {
                speed: 100,
                fuel_regen: 1,
                ..FAST
            }),
        )],
        sites: vec![(SystemId(131), vec![planet(140, -1000.0, 0.0)])],
        ..catalog()
    }
}
