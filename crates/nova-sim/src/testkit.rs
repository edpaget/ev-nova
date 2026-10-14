//! A canned [`PilotCatalog`], [`TrafficCatalog`], [`CombatCatalog`] and
//! [`CommCatalog`] for the crate's own tests.

use std::cell::RefCell;

use crate::ai::Goal;
use crate::catalog::{
    AuxShip, CharacterStart, CombatCatalog, CommCatalog, CommodityStrings, CronId, CronRecord,
    DateAffixes, DisasterRecord, DudeId, DudeRecord, FleetRecord, GateSite, GovtId, GovtRecord,
    HullRecord, JunkRecord, LandingSite, ListedStellar, MissionId, MissionRecord, MissionShip,
    MissionTexts, NebulaId, NebulaRecord, OutfitId, OutfitRecord, Penalties, PersonId,
    PersonRecord, PilotCatalog, PlaceNames, RankId, RankRecord, ShipId, ShipRecord, StarSystem,
    StartDate, StartError, StellarId, StellarLife, StellarMark, SystemId, SystemTraffic,
    SystemVisibility, TrafficCatalog, WeaponId, WeaponRecord,
};
use crate::chance::{Chance, NeverFires};
use crate::combat::armament::{Armament, Trigger};
use crate::combat::hull::{Condition, HullSpec};
use crate::control::{ControlBits, PilotFacts, Script, Test, TestExpr};
use crate::flight::ShipState;
use crate::flight::{Controls, Turn};
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::hyperspace::{JumpRefusal, MIN_JUMP_DISTANCE};
use crate::landing::{LandOutcome, LandingRefusal, StellarFlags};
use crate::session::LandPress;
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
    /// Every `spöb`'s `Flags`, read with the goods.
    pub(crate) stellars: Vec<(StellarId, u32)>,
    /// How many times the goods (commodities, `jünk`, `öops` and the
    /// stellars' flags) were read.
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
    /// The first `chär`'s date prefix and suffix.
    pub(crate) date_affixes: DateAffixes,
    /// How many times the date affixes were read.
    pub(crate) date_affix_reads: RefCell<usize>,
    /// Every stellar a gate may lead to: none, by default.
    pub(crate) gates: Vec<GateSite>,
    /// How many times the gate sites were read.
    pub(crate) gate_reads: RefCell<usize>,
    /// Each system's `NavDefs` entries beyond its landing sites: the
    /// gates it lists that the ship never flies among, which the gate
    /// rules resolve to it. None, by default.
    pub(crate) gate_listers: Vec<(SystemId, StellarId)>,
    /// Every stellar's life: none, by default ([`FakePilotCatalog::living`]).
    pub(crate) lives: Vec<StellarLife>,
    /// How many times the stellars' lives were read.
    pub(crate) life_reads: RefCell<usize>,
    /// Graphic type 0's sheet's frame size, or `None` when it does not
    /// read: 100 square, by default.
    pub(crate) fallback_frame: Option<(u32, u32)>,
    /// Every `mïsn`: none, by default.
    pub(crate) missions: Vec<MissionRecord>,
    /// What the screens show of each `mïsn`: none, by default.
    pub(crate) mission_texts: Vec<MissionTexts>,
    /// The stellars' and systems' names: none, by default.
    pub(crate) place_names: PlaceNames,
    /// How many times the names were read.
    pub(crate) place_name_reads: RefCell<usize>,
    /// What `stellar_marks` returns: none unless a test sets them.
    pub(crate) marks: Vec<StellarMark>,
    /// Each system's `Visibility`: none, by default, which leaves every
    /// system visible.
    pub(crate) visibility: Vec<SystemVisibility>,
    /// Every `crön`: none, by default.
    pub(crate) crons: Vec<CronRecord>,
    /// Every `ränk`: none, by default.
    pub(crate) ranks: Vec<RankRecord>,
    /// Every `nëbu`: none, by default.
    pub(crate) nebulae: Vec<NebulaRecord>,
}

/// Cron `id`, open on every date, starting on any roll and asking
/// nothing, running 0 days with no holdoffs, and with no scripts and no
/// `Contribute`.
pub(crate) fn cron(id: i16) -> CronRecord {
    CronRecord {
        id: CronId(id),
        first_day: 0,
        first_month: 0,
        first_year: 0,
        last_day: 0,
        last_month: 0,
        last_year: 0,
        random: 100,
        duration: 0,
        pre_holdoff: 0,
        post_holdoff: 0,
        flags: 0,
        enable_on: Test::default(),
        on_start: Script::default(),
        on_end: Script::default(),
        contribute: 0,
        require: 0,
        news_govt: [-1; 4],
        govt_news: [-1; 4],
        ind_news: -1,
    }
}

/// Nebula `id` over `rect` (left, top, width, height), active by
/// `active_on` and running `on_explore` as it is explored.
pub(crate) fn nebula(
    id: i16,
    rect: (i16, i16, i16, i16),
    active_on: &str,
    on_explore: &str,
) -> NebulaRecord {
    NebulaRecord {
        id: NebulaId(id),
        left: rect.0,
        top: rect.1,
        width: rect.2,
        height: rect.3,
        active_on: Test::parse(active_on),
        on_explore: Script::parse(on_explore),
    }
}

/// Rank `id`: unaffiliated, of `Weight` 0, paying nothing, with no cap,
/// no `Contribute`, no flags, every name empty and a `PriceMod` of 100.
pub(crate) fn rank(id: i16) -> RankRecord {
    RankRecord {
        id: RankId(id),
        name: String::new(),
        weight: 0,
        affil_govt: None,
        salary: 0,
        salary_cap: 0,
        contribute: 0,
        flags: 0,
        conv_name: String::new(),
        short_name: String::new(),
        price_mod: 100,
    }
}

/// Stellar `id` of `Strength` 100 and `dead_time`, starting the game
/// whole, with no `OnRegen`.
pub(crate) fn life(id: i16, dead_time: i16) -> StellarLife {
    StellarLife {
        id: StellarId(id),
        strength: 100,
        dead_time,
        flags2: 0,
        explosion: None,
        on_destroy: crate::control::Script::default(),
        on_regen: crate::control::Script::default(),
    }
}

impl FakePilotCatalog {
    /// This catalog with `lives` among its stellars' lives.
    pub(crate) fn living(mut self, lives: impl IntoIterator<Item = StellarLife>) -> Self {
        self.lives.extend(lives);
        self
    }
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
    flags2: 0,
};

/// Mission `id`, offered at the mission computer of any stellar on every
/// visit, to anyone: going nowhere, carrying nothing, paying nothing and
/// with no deadline.
pub(crate) fn mission(id: i16) -> MissionRecord {
    MissionRecord {
        id: MissionId(id),
        avail_stel: -1,
        avail_loc: 0,
        avail_record: 0,
        avail_rating: 0,
        avail_random: 100,
        avail_ship_type: 0,
        avail_bits: Test::default(),
        require: 0,
        flags: 0,
        flags2: 0,
        travel_stel: -1,
        return_stel: -1,
        cargo_type: -1,
        cargo_qty: 0,
        pickup_mode: -1,
        drop_off_mode: -1,
        scan_mask: 0,
        pay_val: 0,
        time_limit: 0,
        disp_weight: 0,
        on_accept: Script::default(),
        on_refuse: Script::default(),
        on_success: Script::default(),
        on_failure: Script::default(),
        on_abort: Script::default(),
        on_ship_done: Script::default(),
        can_abort: 1,
        comp_govt: None,
        comp_reward: 0,
        date_post_inc: 0,
        ships: MissionShip::NONE,
        aux: AuxShip::NONE,
    }
}

/// Mission `id` active, going nowhere, with `tons` of cargo 0 (food)
/// aboard.
pub(crate) fn carrying(id: i16, tons: i64) -> crate::mission::ActiveMission {
    crate::mission::ActiveMission {
        mission: MissionId(id),
        destination: None,
        return_to: None,
        cargo_type: Some(0),
        cargo_tons: tons,
        pay: 0,
        days_left: None,
        deadline: None,
        reached: false,
        goals_met: true,
        aboard: true,
        failed: None,
        ships: crate::mission::SpecialShips::default(),
        aux: crate::mission::aux_ships::AuxShips::default(),
    }
}

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
        flags2: 0,
        destroyed: false,
        graphic: 0,
        dead_graphic: None,
        dead_frame_size: None,
        type_counts_frames: true,
        exit_angle: -1,
        tribute: 0,
    }
}

/// `oütf` `id` with these mods (the unused ones none): tech level 1, a
/// ton, 1000 credits, up to 10 owned, no flags, requiring nothing, for
/// sale every day (`BuyRandom` 100), of
/// `ItemClass` 0, its lower-case names "outfit `id`" and "outfits `id`".
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
        buy_random: 100,
        availability: Test::default(),
        item_class: 0,
        lc_name: format!("outfit {id}"),
        lc_plural: format!("outfits {id}"),
        scan_mask: 0,
        on_purchase: crate::control::Script::default(),
        on_sell: crate::control::Script::default(),
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
        availability: Test::default(),
        appear_on: Test::default(),
        flags3: 0,
        disp_weight: 0,
        max_gun: 2,
        max_tur: 1,
        length: 20,
        crew: 3,
        inherent_ai: 1,
        comm_name: format!("ship {id}"),
        inherent_govt: None,
        combat_govt: None,
        escort_type: -1,
        on_capture: crate::control::Script::default(),
        on_purchase: crate::control::Script::default(),
        on_retire: crate::control::Script::default(),
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
        prox_safety: 0,
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
        height: None,
        strength: 0,
        key_carried: None,
    }
}

/// `gövt` `id`: no flags, no classes, allies or enemies, no tolerance
/// for crime, no initial record, no penalties, and `MaxOdds` 100 (even odds), hailed as
/// "Govt `id`", scanning for nothing, fining nothing and requiring nothing.
pub(crate) fn govt(id: i16) -> GovtRecord {
    GovtRecord {
        id: GovtId(id),
        flags: 0,
        flags2: 0,
        crime_tol: 0,
        initial_rec: 0,
        penalties: Penalties::default(),
        max_odds: 100,
        classes: [-1; 4],
        allies: [-1; 4],
        enemies: [-1; 4],
        comm_name: format!("Govt {id}"),
        scan_mask: 0,
        scan_fine: 0,
        news_pic: -1,
        require: 0,
    }
}

/// `përs` `id`, "Person `id`", flying ship `ship`: a warship of no
/// government and `Aggress` 2 that may appear anywhere (`LinkSyst` -1),
/// with no `Coward`, extra weapons, credits, `ShieldMod`, quotes,
/// picture, mission, flags, `ActiveOn` or grant.
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
        active_on: Test::default(),
        subtitle: String::new(),
        flags2: 0,
        grant_class: 0,
        grant_count: 0,
        grant_prob: 0,
        mission_ship: None,
    }
}

/// NPC `id`, a wimpy trader of ship type 128 performing as `stats`, its
/// reserves full, at rest at the centre facing up, flying and idle,
/// intact and unarmed.
pub(crate) fn npc(id: u32, stats: ShipStats) -> Npc {
    Npc {
        id: NpcId(id),
        berth: 0,
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
        mission: None,
        aux: None,
        cloak: crate::cloak::Cloak::default(),
        cloak_gear: crate::cloak::CloakGear::default(),
        led_out: false,
        self_repair: false,
        former: None,
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
        stellars: Vec::new(),
        goods_reads: RefCell::default(),
        traffic: Vec::new(),
        dudes: Vec::new(),
        fleets: Vec::new(),
        persons: Vec::new(),
        weapons: Vec::new(),
        hulls: Vec::new(),
        govts: Vec::new(),
        strings: Vec::new(),
        date_affixes: DateAffixes {
            prefix: String::new(),
            suffix: " NC".to_owned(),
        },
        date_affix_reads: RefCell::default(),
        gates: Vec::new(),
        gate_reads: RefCell::default(),
        gate_listers: Vec::new(),
        lives: Vec::new(),
        life_reads: RefCell::default(),
        fallback_frame: Some((100, 100)),
        missions: Vec::new(),
        mission_texts: Vec::new(),
        place_names: PlaceNames::default(),
        place_name_reads: RefCell::default(),
        marks: Vec::new(),
        visibility: Vec::new(),
        crons: Vec::new(),
        ranks: Vec::new(),
        nebulae: Vec::new(),
    }
}

/// `catalog` with variants in system 130 marked as `_StellarsAreIdentical`
/// reads them: 150 at 128's place with its name "Home" (30,-40), 151 at
/// 129's with its name "Varia" (2000,0), and two that miss 129 by one:
/// 152 at its place named "Other", 153 named "Varia" at 2001,0.
pub(crate) fn with_variants(mut catalog: FakePilotCatalog) -> FakePilotCatalog {
    let sites = &mut catalog
        .sites
        .iter_mut()
        .find(|(system, _)| *system == SystemId(130))
        .expect("130 listed")
        .1;
    sites.extend([
        planet(150, -2000.0, 0.0),
        planet(151, 0.0, 2000.0),
        planet(152, 0.0, -2000.0),
        planet(153, -2000.0, -2000.0),
    ]);
    let mark = |id, place, name: &str| StellarMark {
        id: StellarId(id),
        place,
        name: name.to_owned(),
    };
    catalog.marks = vec![
        mark(128, (30, -40), "Home"),
        mark(129, (2000, 0), "Varia"),
        mark(150, (30, -40), "Home"),
        mark(151, (2000, 0), "Varia"),
        mark(152, (2000, 0), "Other"),
        mark(153, (2001, 0), "Varia"),
    ];
    catalog
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
        stellars: Vec::new(),
        interference: 0,
    }
}

/// The control bit that hides `system` in a [`hideable`] catalog: 9000
/// past its ID.
pub(crate) fn hiding_bit(system: i16) -> crate::control::Bit {
    u16::try_from(system)
        .ok()
        .and_then(|id| crate::control::Bit::new(9000 + id))
        .expect("a system of a hideable catalog has a hiding bit")
}

/// Makes `catalog` hideable: each system on its map that has no
/// `Visibility` yet is visible unless its [`hiding_bit`] is set. A test
/// hides it by setting the bit and refreshing the galaxy, as play does, so
/// every later refresh keeps it hidden.
pub(crate) fn hideable(catalog: &mut FakePilotCatalog) {
    for system in &catalog.star_map {
        if !catalog.visibility.iter().any(|test| test.id == system.id) {
            catalog.visibility.push(SystemVisibility {
                id: system.id,
                visibility: Test::parse(&format!("!b{}", hiding_bit(system.id.0).get())),
            });
        }
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

    fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
        *self.goods_reads.borrow_mut() += 1;
        self.stellars.clone()
    }

    fn date_affixes(&self) -> DateAffixes {
        *self.date_affix_reads.borrow_mut() += 1;
        self.date_affixes.clone()
    }

    fn gate_sites(&self) -> Vec<GateSite> {
        *self.gate_reads.borrow_mut() += 1;
        self.gates.clone()
    }

    fn stellar_lives(&self) -> Vec<StellarLife> {
        *self.life_reads.borrow_mut() += 1;
        self.lives.clone()
    }

    fn fallback_frame_size(&self) -> Option<(u32, u32)> {
        self.fallback_frame
    }

    fn missions(&self) -> Vec<MissionRecord> {
        self.missions.clone()
    }

    fn mission_texts(&self) -> Vec<MissionTexts> {
        self.mission_texts.clone()
    }

    /// Every stellar of [`FakePilotCatalog::sites`], with every system
    /// listing it ([`listed_stellars`](crate::fixture::listed_stellars)),
    /// and the gates of [`FakePilotCatalog::gate_listers`].
    fn listed_stellars(&self) -> Vec<ListedStellar> {
        let mut listed = crate::fixture::listed_stellars(
            self.sites
                .iter()
                .map(|(system, sites)| (*system, sites.as_slice())),
        );
        crate::fixture::list_also(&mut listed, self.gate_listers.iter().copied());
        listed
    }

    fn system_visibility(&self) -> Vec<SystemVisibility> {
        self.visibility.clone()
    }

    fn place_names(&self) -> PlaceNames {
        *self.place_name_reads.borrow_mut() += 1;
        self.place_names.clone()
    }

    fn stellar_marks(&self) -> Vec<StellarMark> {
        self.marks.clone()
    }

    fn crons(&self) -> Vec<CronRecord> {
        self.crons.clone()
    }

    fn ranks(&self) -> Vec<RankRecord> {
        self.ranks.clone()
    }

    fn nebulae(&self) -> Vec<NebulaRecord> {
        self.nebulae.clone()
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

/// Presses L until it lands or is refused: a first press that selects a
/// stellar is followed by a second. Gives the stellar landed on, or the
/// refusal.
pub(crate) fn land_now(session: &mut Session) -> Result<StellarId, LandingRefusal> {
    for _ in 0..2 {
        if let LandPress::Outcome(LandOutcome::Landed(stellar)) = session.land()? {
            return Ok(stellar);
        }
    }
    panic!(
        "L selected twice and never landed: {:?}",
        session.nav_target()
    );
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
    begin_jump_now(session).expect("jumps");
    session.arrive(catalog, chance)
}

/// Presses J, and if the jump is accepted, ticks with no keys held through
/// the pre-jump stage until the jump has begun. Gives what J gave.
pub(crate) fn begin_jump_now(session: &mut Session) -> Result<SystemId, JumpRefusal> {
    let next = session.begin_jump()?;
    for _ in 0..1000 {
        if session.jumping().is_some() {
            return Ok(next);
        }
        session.tick(Controls::default());
    }
    panic!("the jump never began: {:?}", session.player());
}

/// A [`Chance`] that answers and rolls from a script (no, and 0, once it
/// runs out) and records each percent and each number of sides it is
/// asked.
#[derive(Debug, Default)]
pub(crate) struct Scripted {
    answers: Vec<bool>,
    rolls: Vec<u16>,
    pub(crate) asked: Vec<u8>,
    pub(crate) sides_asked: Vec<u16>,
}

impl Scripted {
    /// Answers `answers`, in order, then no.
    pub(crate) fn answering(answers: &[bool]) -> Self {
        Self {
            answers: answers.iter().rev().copied().collect(),
            ..Self::default()
        }
    }

    /// Rolls `rolls`, in order, then 0.
    pub(crate) fn rolling(rolls: &[u16]) -> Self {
        Self {
            rolls: rolls.iter().rev().copied().collect(),
            ..Self::default()
        }
    }

    /// This script, rolling `rolls`, in order, then 0.
    pub(crate) fn and_rolling(self, rolls: &[u16]) -> Self {
        Self {
            rolls: rolls.iter().rev().copied().collect(),
            ..self
        }
    }
}

impl Chance for Scripted {
    fn fires(&mut self, percent: u8) -> bool {
        self.asked.push(percent);
        self.answers.pop().unwrap_or(false)
    }

    fn roll(&mut self, sides: u16) -> u16 {
        self.sides_asked.push(sides);
        self.rolls.pop().unwrap_or(0)
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

    /// A jump's arrival's draws: the roll table's
    /// [`ROLLED`](crate::mission::ROLLED), each its last outcome, then
    /// `draws`.
    pub(crate) fn arriving(draws: &[u32]) -> Self {
        let table = std::iter::repeat_n(99, crate::mission::ROLLED);
        Self {
            queue: table.chain(draws.iter().copied()).collect(),
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

/// Control bits that refuse every test reading one of these bits, and
/// hold every other.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RefuseBits(pub(crate) &'static [u16]);

impl ControlBits for RefuseBits {
    fn allows(&self, test: &TestExpr, _pilot: &dyn PilotFacts) -> bool {
        !test.reads().iter().any(|bit| self.0.contains(&bit.get()))
    }
}

/// Control bits that hold every test asked.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AllowAll;

impl ControlBits for AllowAll {
    fn allows(&self, _test: &TestExpr, _pilot: &dyn PilotFacts) -> bool {
        true
    }
}
