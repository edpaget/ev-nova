//! A canned [`PilotCatalog`], [`TrafficCatalog`], [`CombatCatalog`],
//! [`AsteroidCatalog`] and [`CommCatalog`] for the crate's own tests.

use std::cell::RefCell;

/// The default NPC, from the traffic's test support.
pub(crate) use crate::traffic::fixture::npc;
/// The [`Chance`] mocks and the fast ship class, from the foundation
/// crate's test support.
pub(crate) use nova_sim_base::fixture::{Draws, FAST, Ranged, Scripted};
/// The catalog's record builders and control-bit mocks, from the catalog
/// crate's test support.
pub(crate) use nova_sim_catalog::fixture::{
    AllowAll, RefuseBits, cron, govt, hull, mission, nebula, outfit, person, planet, rank, ship,
    star, weapon,
};

use crate::catalog::{
    AsteroidCatalog, AsteroidFieldRecord, AsteroidTypeRecord, BOX_GRAPHICS, CharacterStart,
    CombatCatalog, CommCatalog, CommodityStrings, CronRecord, DateAffixes, DisasterRecord, DudeId,
    DudeRecord, FleetRecord, GateSite, GovtRecord, HullRecord, JunkRecord, LandingSite,
    ListedStellar, MissionId, MissionRecord, MissionTexts, NebulaRecord, OutfitId, OutfitRecord,
    PersonRecord, PilotCatalog, PlaceNames, RankRecord, ShipId, ShipRecord, StarSystem, StartDate,
    StartError, StellarId, StellarLife, StellarMark, StringPatch, SystemId, SystemTraffic,
    SystemVisibility, TrafficCatalog, WeaponRecord,
};
use crate::chance::{Chance, NeverFires};
use crate::control::Test;
use crate::flight::{Controls, Turn};
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::hyperspace::{JumpRefusal, MIN_JUMP_DISTANCE};
use crate::landing::{LandOutcome, LandingRefusal};
use crate::session::LandPress;
use crate::session::Session;
use crate::traffic::npc::NpcId;
use crate::warp::SoundLength;

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
    /// Each `'STR '` patch; any other is [`StringPatch::Absent`].
    pub(crate) string_patches: Vec<(i16, StringPatch)>,
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
    /// What `stellar_marks` returns for the stellars it names: none
    /// unless a test sets them. Every other stellar a system lists as a
    /// landing site is marked at 0,0 with a name of its own, `#<id>`, so
    /// no two read as one stop, unless it is `unloaded`.
    pub(crate) marks: Vec<StellarMark>,
    /// The listed stellars `stellar_marks` leaves unmarked, as `spöb`s
    /// that didn't load: none, by default.
    pub(crate) unloaded: Vec<StellarId>,
    /// Each system's `Visibility`: none, by default, which leaves every
    /// system visible.
    pub(crate) visibility: Vec<SystemVisibility>,
    /// Every `crön`: none, by default.
    pub(crate) crons: Vec<CronRecord>,
    /// Every `ränk`: none, by default.
    pub(crate) ranks: Vec<RankRecord>,
    /// Every `nëbu`: none, by default.
    pub(crate) nebulae: Vec<NebulaRecord>,
    /// The warp-up sound's length: none, by default, so the warp's clock
    /// takes 350 Mac ticks.
    pub(crate) warp_sound: Option<SoundLength>,
    /// The double-time warp-up sound's length: none, by default, so the
    /// warp's clock takes 350 Mac ticks in double time too.
    pub(crate) warp_sound_double_time: Option<SoundLength>,
    /// Every `röid`, with its sprite's frames and size: none, by default.
    pub(crate) asteroid_types: Vec<AsteroidTypeRecord>,
    /// Every system's asteroid field: none, by default.
    pub(crate) asteroid_fields: Vec<AsteroidFieldRecord>,
    /// Each box graphic's frame size: none, by default.
    pub(crate) box_frames: [Option<(u32, u32)>; BOX_GRAPHICS],
    /// The warp-out sound's length: none, by default, so an arrival's
    /// warp-out never plays on and every jump arrival plays it.
    pub(crate) warp_out_sound: Option<SoundLength>,
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
        string_patches: Vec::new(),
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
        unloaded: Vec::new(),
        visibility: Vec::new(),
        crons: Vec::new(),
        ranks: Vec::new(),
        nebulae: Vec::new(),
        warp_sound: None,
        warp_sound_double_time: None,
        asteroid_types: Vec::new(),
        asteroid_fields: Vec::new(),
        box_frames: [None; BOX_GRAPHICS],
        warp_out_sound: None,
    }
}

/// `catalog` with system 130's planet 129 at (-2000, 0): as far out as
/// at (2000, 0), but on the side the 999 bound on a destination does not
/// reach (see [`destination`](crate::traffic::destination)), so an idle
/// NPC drawing the last candidate heads for it, far from 128.
pub(crate) fn far_129_reachable(mut catalog: FakePilotCatalog) -> FakePilotCatalog {
    for (_, sites) in &mut catalog.sites {
        for site in sites.iter_mut().filter(|site| site.id == StellarId(129)) {
            site.position = Vec2::new(-2000.0, 0.0);
        }
    }
    catalog
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

/// `catalog` with a system 134 at 131's map place, (600, 0), listing a
/// stellar 141 at the centre, as 131 lists 140: 131 shows while b50 is
/// clear, 134 once it is set. With `marked`, 140 and 141 are marked with
/// one place and name, so b50 swaps a stellar for its variant (Tri'niphu
/// 212 for 657 after b1502, in miniature); without, they differ by name.
pub(crate) fn swapped(mut catalog: FakePilotCatalog, marked: bool) -> FakePilotCatalog {
    catalog.systems.push(SystemId(134));
    catalog.star_map.push(star(134, (600.0, 0.0), &[]));
    catalog
        .sites
        .push((SystemId(134), vec![planet(141, 0.0, 0.0)]));
    let mark = |id, name: &str| StellarMark {
        id: StellarId(id),
        place: (0, 0),
        name: name.to_owned(),
    };
    let other = if marked { "Tre'ar Zalom" } else { "Elsewhere" };
    catalog
        .marks
        .extend([mark(140, "Tre'ar Zalom"), mark(141, other)]);
    catalog.visibility.extend([
        SystemVisibility {
            id: SystemId(131),
            visibility: Test::parse("!b50"),
        },
        SystemVisibility {
            id: SystemId(134),
            visibility: Test::parse("b50"),
        },
    ]);
    catalog
}

pub(crate) const START: StartDate = StartDate {
    day: 23,
    month: 6,
    year: 1177,
};

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

    fn string_patch(&self, id: i16) -> StringPatch {
        self.string_patches
            .iter()
            .find(|(patch, _)| *patch == id)
            .map_or(StringPatch::Absent, |(_, patch)| patch.clone())
    }

    /// [`FakePilotCatalog::strings`]' `STR#` 1000.
    fn buoy_strings(&self) -> Vec<String> {
        self.string_list(crate::buoy::BUOY_STRINGS)
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
    /// and the gates of [`FakePilotCatalog::gate_listers`]; the sites of
    /// systems outside [`FakePilotCatalog::systems`] are left out.
    fn listed_stellars(&self) -> Vec<ListedStellar> {
        // Only the systems that can be read list, as the port says.
        let mut listed = crate::fixture::listed_stellars(
            self.sites
                .iter()
                .filter(|(system, _)| self.systems.contains(system))
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
        let given = |id: StellarId| {
            self.marks.iter().any(|mark| mark.id == id) || self.unloaded.contains(&id)
        };
        let listed = self
            .sites
            .iter()
            .flat_map(|(_, sites)| sites.iter().map(|site| site.id))
            .filter(|&id| !given(id))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|id| StellarMark {
                id,
                place: (0, 0),
                name: format!("#{}", id.0),
            });
        self.marks.iter().cloned().chain(listed).collect()
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

    fn warp_sound(&self) -> Option<SoundLength> {
        self.warp_sound
    }

    fn warp_sound_double_time(&self) -> Option<SoundLength> {
        self.warp_sound_double_time
    }

    fn warp_out_sound(&self) -> Option<SoundLength> {
        self.warp_out_sound
    }

    fn mission_dude(&self, id: DudeId) -> Option<DudeRecord> {
        TrafficCatalog::dude(self, id)
    }

    /// The first of each existing system's [`FakePilotCatalog::sites`], as
    /// [`PilotCatalog::landing_sites`] gives them, read without counting
    /// as a read of the star map or the sites.
    fn first_stellars(&self) -> Vec<(SystemId, StellarId)> {
        let mut first = std::collections::BTreeMap::new();
        for (system, sites) in &self.sites {
            first
                .entry(*system)
                .or_insert_with(|| sites.first().map(|site| site.id));
        }
        first
            .into_iter()
            .filter(|(system, _)| self.systems.contains(system))
            .filter_map(|(system, stellar)| Some((system, stellar?)))
            .collect()
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

/// No asteroids unless a test sets them.
impl AsteroidCatalog for FakePilotCatalog {
    fn asteroid_types(&self) -> Vec<AsteroidTypeRecord> {
        self.asteroid_types.clone()
    }

    fn asteroid_fields(&self) -> Vec<AsteroidFieldRecord> {
        self.asteroid_fields.clone()
    }

    fn box_frame_sizes(&self) -> [Option<(u32, u32)>; BOX_GRAPHICS] {
        self.box_frames
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

    /// [`FakePilotCatalog::string_patches`]' `'STR '` `id`, as
    /// [`PilotCatalog::string_patch`] reads it.
    fn plugin_string(&self, id: i16) -> StringPatch {
        self.string_patch(id)
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

/// Presses L until it lands or is refused, the dock approach alone run
/// between presses (not a whole tick, whose motion, fuel and timers the
/// tests calling this do not expect): a press that requests is followed
/// by the approach and another press, up to three. The dock request's
/// own messages and sounds raised on the way (its line, "No response.",
/// the greeting and traffic control's sound) are forgotten, as the
/// `clearance` module's tests pin them. Gives the stellar landed on, or
/// the refusal.
pub(crate) fn land_now(session: &mut Session) -> Result<StellarId, LandingRefusal> {
    let before = session.chatter_marks();
    let landed = (|| {
        for press in 0..3 {
            if press > 0 {
                session.dock_approach();
            }
            if let LandPress::Outcome(LandOutcome::Landed(stellar)) = session.land()? {
                return Ok(stellar);
            }
        }
        panic!(
            "L requested three times and never landed: {:?}",
            session.nav_target()
        );
    })();
    session.forget_dock_chatter(before);
    landed
}

/// The stellar a session just flown from a save resumed on: asserts it
/// flies, at rest, nothing about a landing run, and gives the stellar the
/// pilot last landed on, which the load keeps (see [`Session::load`]).
pub(crate) fn resumed_on(session: &Session) -> Option<StellarId> {
    assert_eq!(session.landed(), None, "resumed in flight");
    assert_eq!(session.player().velocity, Vec2::ZERO, "at rest");
    session.pilot().stellar()
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

/// A jump's arrival's draws: the roll table's
/// [`ROLLED`](crate::mission::ROLLED), each its last outcome, then
/// `draws`.
pub(crate) fn arriving(draws: &[u32]) -> Draws {
    let table = std::iter::repeat_n(99, crate::mission::ROLLED);
    Draws::of(&table.chain(draws.iter().copied()).collect::<Vec<_>>())
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

/// No ship drawn anywhere, so a click in space falls through to the
/// stellars.
pub(crate) struct NoShips;

impl crate::hover::DrawnShips for NoShips {
    fn npc(&self, _id: NpcId) -> Option<crate::hover::DrawnBox> {
        None
    }

    fn player(&self) -> Option<crate::hover::DrawnBox> {
        None
    }
}

/// The stellar a click at `at` in space, with no ship drawn, selects.
pub(crate) fn click_stellar(session: &mut Session, at: Vec2) -> Option<StellarId> {
    match session.click_in_space(at, &NoShips) {
        Some(crate::hover::Hover::Stellar(id)) => Some(id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_lists_a_stellar_only_under_the_systems_it_holds() {
        let fake = FakePilotCatalog {
            systems: vec![SystemId(131)],
            sites: vec![
                (SystemId(130), vec![planet(140, 0.0, 0.0)]),
                (SystemId(131), vec![planet(140, 0.0, 0.0)]),
            ],
            ..catalog()
        };
        let listed = fake.listed_stellars();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, StellarId(140));
        assert_eq!(listed[0].systems, [SystemId(131)]);
    }

    #[test]
    fn the_fake_lists_no_stellar_whose_every_lister_is_gone() {
        let fake = FakePilotCatalog {
            systems: vec![SystemId(131)],
            sites: vec![(SystemId(130), vec![planet(140, 0.0, 0.0)])],
            ..catalog()
        };
        assert_eq!(fake.listed_stellars(), []);
    }
}
