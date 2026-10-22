//! [`FakePilotCatalog`]: a canned [`PilotCatalog`], [`TrafficCatalog`],
//! [`CombatCatalog`], [`AsteroidCatalog`] and [`CommCatalog`], and the
//! [`catalog`] it starts from, for the tests of the crates above.

use std::cell::RefCell;

use nova_sim_base::fixture::FAST;

use super::{planet, star};
use crate::catalog::{
    AsteroidCatalog, AsteroidFieldRecord, AsteroidTypeRecord, BOX_GRAPHICS, BoomId, CharacterStart,
    CombatCatalog, CommCatalog, CommodityStrings, CronRecord, DateAffixes, DisasterRecord, DudeId,
    DudeRecord, FleetRecord, GateSite, GovtRecord, HullRecord, JunkRecord, LandingSite,
    ListedStellar, MissionRecord, MissionTexts, NebulaRecord, OutfitId, OutfitRecord, PersonRecord,
    PilotCatalog, PlaceNames, RankRecord, ShipId, ShipRecord, SoundId, StarSystem, StartDate,
    StartError, StellarId, StellarLife, StellarMark, StringPatch, SystemId, SystemReinforcements,
    SystemTraffic, SystemVisibility, TrafficCatalog, WeaponRecord,
};
use crate::handling::ShipFields;
use crate::voice::Voices;
use crate::warp::SoundLength;

/// A canned first `chär`, ships and systems; records the ships asked
/// for.
pub struct FakePilotCatalog {
    pub character: Result<CharacterStart, StartError>,
    pub ships: Vec<(ShipId, Result<ShipFields, String>)>,
    pub systems: Vec<SystemId>,
    pub ships_asked: RefCell<Vec<ShipId>>,
    /// Each system's landing sites; any other has none.
    pub sites: Vec<(SystemId, Vec<LandingSite>)>,
    pub sites_asked: RefCell<Vec<SystemId>>,
    pub star_map: Vec<StarSystem>,
    pub star_map_reads: RefCell<usize>,
    /// Each ship's default items; any other ship has none.
    pub defaults: Vec<(ShipId, Vec<(OutfitId, u16)>)>,
    pub defaults_asked: RefCell<Vec<ShipId>>,
    /// Every `oütf`.
    pub outfits: Vec<OutfitRecord>,
    /// How many times the outfits were read.
    pub outfit_reads: RefCell<usize>,
    /// Every `shïp`, as the shipyard reads them.
    pub ship_records: Vec<ShipRecord>,
    /// How many times the ship records were read.
    pub ship_record_reads: RefCell<usize>,
    pub commodities: CommodityStrings,
    pub junk: Vec<JunkRecord>,
    pub disasters: Vec<DisasterRecord>,
    /// Every `spöb`'s `Flags`, read with the goods.
    pub stellars: Vec<(StellarId, u32)>,
    /// How many times the goods (commodities, `jünk`, `öops` and the
    /// stellars' flags) were read.
    pub goods_reads: RefCell<usize>,
    /// Each system's traffic; any other has none.
    pub traffic: Vec<(SystemId, SystemTraffic)>,
    /// Each system's reinforcement fleet; any other has none.
    pub reinforcements: Vec<(SystemId, SystemReinforcements)>,
    /// Every `düde`.
    pub dudes: Vec<(DudeId, DudeRecord)>,
    /// Every `flët`.
    pub fleets: Vec<FleetRecord>,
    /// Every `përs`.
    pub persons: Vec<PersonRecord>,
    /// Every `wëap`.
    pub weapons: Vec<WeaponRecord>,
    /// Every `shïp`'s combat fields.
    pub hulls: Vec<HullRecord>,
    /// Every `gövt`.
    pub govts: Vec<GovtRecord>,
    /// Each `STR#`; any other is missing.
    pub strings: Vec<(i16, Vec<String>)>,
    /// Each `'STR '` patch; any other is [`StringPatch::Absent`].
    pub string_patches: Vec<(i16, StringPatch)>,
    /// The first `chär`'s date prefix and suffix.
    pub date_affixes: DateAffixes,
    /// How many times the date affixes were read.
    pub date_affix_reads: RefCell<usize>,
    /// Every stellar a gate may lead to: none, by default.
    pub gates: Vec<GateSite>,
    /// How many times the gate sites were read.
    pub gate_reads: RefCell<usize>,
    /// Each system's `NavDefs` entries beyond its landing sites: the
    /// gates it lists that the ship never flies among, which the gate
    /// rules resolve to it. None, by default.
    pub gate_listers: Vec<(SystemId, StellarId)>,
    /// Every stellar's life: none, by default ([`FakePilotCatalog::living`]).
    pub lives: Vec<StellarLife>,
    /// How many times the stellars' lives were read.
    pub life_reads: RefCell<usize>,
    /// Graphic type 0's sheet's frame size, or `None` when it does not
    /// read: 100 square, by default.
    pub fallback_frame: Option<(u32, u32)>,
    /// Every `mïsn`: none, by default.
    pub missions: Vec<MissionRecord>,
    /// What the screens show of each `mïsn`: none, by default.
    pub mission_texts: Vec<MissionTexts>,
    /// The stellars' and systems' names: none, by default.
    pub place_names: PlaceNames,
    /// How many times the names were read.
    pub place_name_reads: RefCell<usize>,
    /// What `stellar_marks` returns for the stellars it names: none
    /// unless a test sets them. Every other stellar a system lists as a
    /// landing site is marked at 0,0 with a name of its own, `#<id>`, so
    /// no two read as one stop, unless it is `unloaded`.
    pub marks: Vec<StellarMark>,
    /// The listed stellars `stellar_marks` leaves unmarked, as `spöb`s
    /// that didn't load: none, by default.
    pub unloaded: Vec<StellarId>,
    /// Each system's `Visibility`: none, by default, which leaves every
    /// system visible.
    pub visibility: Vec<SystemVisibility>,
    /// Every `crön`: none, by default.
    pub crons: Vec<CronRecord>,
    /// Every `ränk`: none, by default.
    pub ranks: Vec<RankRecord>,
    /// Every `nëbu`: none, by default.
    pub nebulae: Vec<NebulaRecord>,
    /// The warp-up sound's length: none, by default, so the warp's clock
    /// takes 350 Mac ticks.
    pub warp_sound: Option<SoundLength>,
    /// The double-time warp-up sound's length: none, by default, so the
    /// warp's clock takes 350 Mac ticks in double time too.
    pub warp_sound_double_time: Option<SoundLength>,
    /// Every `röid`, with its sprite's frames and size: none, by default.
    pub asteroid_types: Vec<AsteroidTypeRecord>,
    /// Every system's asteroid field: none, by default.
    pub asteroid_fields: Vec<AsteroidFieldRecord>,
    /// Each box graphic's frame size: none, by default.
    pub box_frames: [Option<(u32, u32)>; BOX_GRAPHICS],
    /// The warp-out sound's length: none, by default, so an arrival's
    /// warp-out never plays on and every jump arrival plays it.
    pub warp_out_sound: Option<SoundLength>,
    /// Each `bööm` with the `snd ` its explosion plays.
    pub boom_sounds: Vec<(BoomId, SoundId)>,
    /// The escorts' voices: none, by default, so no escort speaks.
    pub voices: Voices,
}

impl FakePilotCatalog {
    /// This catalog with `lives` among its stellars' lives.
    #[must_use]
    pub fn living(mut self, lives: impl IntoIterator<Item = StellarLife>) -> Self {
        self.lives.extend(lives);
        self
    }
}

/// The first `chär` flies ship 128 from system 130 on 23 June 1177;
/// ship 128 is fast, and systems 130 and 131 exist. System 130 holds a
/// planet, 128, at (30, -40), which the ship starts over, and another,
/// 129, far away; system 131 holds one at the centre. On the map, 130
/// at (0, 0), 131 at (600, 0) and 132 at (600, 600) are linked in a
/// line, and 133 is linked to none.
#[must_use]
pub fn catalog() -> FakePilotCatalog {
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
        reinforcements: Vec::new(),
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
        boom_sounds: Vec::new(),
        voices: Voices::default(),
    }
}

/// The first `chär`'s start date in [`catalog`]: 23 June 1177.
pub const START: StartDate = StartDate {
    day: 23,
    month: 6,
    year: 1177,
};

/// [`catalog`] with the first `chär` starting in `systems`, each slot
/// a system ID or none.
#[must_use]
pub fn starting(systems: [Option<i16>; 4]) -> FakePilotCatalog {
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
    /// listing it ([`listed_stellars`](super::listed_stellars)),
    /// and the gates of [`FakePilotCatalog::gate_listers`]; the sites of
    /// systems outside [`FakePilotCatalog::systems`] are left out.
    fn listed_stellars(&self) -> Vec<ListedStellar> {
        // Only the systems that can be read list, as the port says.
        let mut listed = super::listed_stellars(
            self.sites
                .iter()
                .filter(|(system, _)| self.systems.contains(system))
                .map(|(system, sites)| (*system, sites.as_slice())),
        );
        super::list_also(&mut listed, self.gate_listers.iter().copied());
        listed
    }

    fn system_visibility(&self) -> Vec<SystemVisibility> {
        self.visibility.clone()
    }

    fn place_names(&self) -> PlaceNames {
        *self.place_name_reads.borrow_mut() += 1;
        self.place_names.clone()
    }

    /// Every stellar given a mark, then each other one listed, at its
    /// first site's position, as a `spöb`'s `xPos` and `yPos` place it.
    fn stellar_marks(&self) -> Vec<StellarMark> {
        let given = |id: StellarId| {
            self.marks.iter().any(|mark| mark.id == id) || self.unloaded.contains(&id)
        };
        let mut listed = std::collections::BTreeMap::new();
        for site in self.sites.iter().flat_map(|(_, sites)| sites) {
            if !given(site.id) {
                listed.entry(site.id).or_insert(site.position);
            }
        }
        let listed = listed.into_iter().map(|(id, position)| StellarMark {
            id,
            #[allow(clippy::cast_possible_truncation)] // a fixture's small positions
            place: (position.x as i16, position.y as i16),
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

    fn boom_sounds(&self) -> Vec<(BoomId, SoundId)> {
        self.boom_sounds.clone()
    }

    fn voices(&self) -> Voices {
        self.voices.clone()
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

    fn system_reinforcements(&self, id: SystemId) -> Option<SystemReinforcements> {
        self.reinforcements
            .iter()
            .find(|(system, _)| *system == id)
            .map(|(_, kept)| *kept)
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

    #[test]
    fn the_catalog_starts_ship_128_in_system_130_on_the_start_date() {
        let fake = catalog();
        assert_eq!(
            fake.first_character(),
            Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [Some(SystemId(130)), None, None, None],
                start: START,
                ..CharacterStart::default()
            })
        );
        assert_eq!(
            (START.day, START.month, START.year),
            (23, 6, 1177),
            "23 June 1177"
        );
        let elsewhere = starting([None, Some(131), None, Some(132)]);
        assert_eq!(
            elsewhere.first_character(),
            Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [None, Some(SystemId(131)), None, Some(SystemId(132))],
                start: START,
                ..CharacterStart::default()
            })
        );
        assert_eq!(
            elsewhere.star_map, fake.star_map,
            "the rest is the catalog's"
        );
    }

    #[test]
    fn living_adds_its_lives_to_the_catalogs() {
        let life = |id| StellarLife {
            id: StellarId(id),
            strength: 100,
            dead_time: 3,
            flags2: 0,
            explosion: None,
            on_destroy: crate::control::Script::default(),
            on_regen: crate::control::Script::default(),
        };
        let fake = FakePilotCatalog {
            lives: vec![life(128)],
            ..catalog()
        }
        .living([life(129), life(140)]);
        assert_eq!(fake.stellar_lives(), [life(128), life(129), life(140)]);
        assert_eq!(*fake.life_reads.borrow(), 1);
        assert_eq!(catalog().stellar_lives(), [], "none, by default");
    }

    #[test]
    fn the_fake_gives_each_ships_fields_and_default_outfits_and_records_the_asks() {
        let slow = ShipFields { speed: 1, ..FAST };
        let fake = FakePilotCatalog {
            ships: vec![
                (ShipId(128), Ok(FAST)),
                (ShipId(129), Ok(slow)),
                (ShipId(130), Err("broken".to_owned())),
            ],
            defaults: vec![
                (ShipId(128), vec![(OutfitId(128), 2)]),
                (ShipId(129), vec![(OutfitId(130), 1)]),
            ],
            ..catalog()
        };
        assert_eq!(fake.ship_fields(ShipId(129)), Ok(slow));
        assert_eq!(fake.ship_fields(ShipId(128)), Ok(FAST));
        assert_eq!(fake.ship_fields(ShipId(130)), Err("broken".to_owned()));
        assert_eq!(fake.ship_fields(ShipId(131)), Err("no shïp 131".to_owned()));
        assert_eq!(
            *fake.ships_asked.borrow(),
            [ShipId(129), ShipId(128), ShipId(130), ShipId(131)]
        );
        assert_eq!(fake.default_outfits(ShipId(129)), [(OutfitId(130), 1)]);
        assert_eq!(fake.default_outfits(ShipId(128)), [(OutfitId(128), 2)]);
        assert_eq!(fake.default_outfits(ShipId(131)), []);
        assert_eq!(
            *fake.defaults_asked.borrow(),
            [ShipId(129), ShipId(128), ShipId(131)]
        );
    }

    #[test]
    fn the_fake_gives_its_records_and_counts_each_read() {
        use super::super::{outfit, ship};
        let junk = JunkRecord {
            id: crate::catalog::JunkId(128),
            name: "Tribbles".to_owned(),
            abbrev: "Trb".to_owned(),
            base_price: 10,
            sold_at: Vec::new(),
            bought_at: Vec::new(),
            buy_on: crate::control::Test::parse(""),
            sell_on: crate::control::Test::parse(""),
            flags: 0,
            scan_mask: 0,
            lc_name: "tribbles".to_owned(),
        };
        let disaster = DisasterRecord {
            duration: 5,
            ..DisasterRecord::default()
        };
        let gate = GateSite {
            id: StellarId(150),
            position: crate::geometry::Vec2::new(1.0, 2.0),
            flags2: 0,
            links: [None; 8],
            exit_angle: 90,
        };
        let commodities = CommodityStrings {
            names: vec!["Food".to_owned()],
            ..CommodityStrings::default()
        };
        let place_names = PlaceNames {
            systems: [(SystemId(130), "Sol".to_owned())].into(),
            ..PlaceNames::default()
        };
        let fake = FakePilotCatalog {
            outfits: vec![outfit(128, &[])],
            ship_records: vec![ship(128, FAST)],
            commodities: commodities.clone(),
            junk: vec![junk.clone()],
            disasters: vec![disaster.clone()],
            stellars: vec![(StellarId(128), 4)],
            gates: vec![gate],
            place_names: place_names.clone(),
            ..catalog()
        };
        let counts = |fake: &FakePilotCatalog| {
            [
                *fake.outfit_reads.borrow(),
                *fake.ship_record_reads.borrow(),
                *fake.star_map_reads.borrow(),
                *fake.goods_reads.borrow(),
                *fake.date_affix_reads.borrow(),
                *fake.gate_reads.borrow(),
                *fake.place_name_reads.borrow(),
            ]
        };
        assert_eq!(counts(&fake), [0; 7]);
        for reads in 1..=2 {
            assert_eq!(fake.outfits(), [outfit(128, &[])]);
            assert_eq!(fake.ships(), [ship(128, FAST)]);
            assert_eq!(fake.star_map(), catalog().star_map);
            assert_eq!(fake.date_affixes().suffix, " NC");
            assert_eq!(fake.gate_sites(), [gate]);
            assert_eq!(fake.place_names(), place_names);
            assert_eq!(counts(&fake), [reads, reads, reads, 0, reads, reads, reads]);
        }
        assert_eq!(fake.commodity_strings(), commodities);
        assert_eq!(*fake.goods_reads.borrow(), 1);
        assert_eq!(fake.junk(), [junk]);
        assert_eq!(*fake.goods_reads.borrow(), 2);
        assert_eq!(fake.disasters(), [disaster]);
        assert_eq!(*fake.goods_reads.borrow(), 3);
        assert_eq!(fake.stellar_flags(), [(StellarId(128), 4)]);
        assert_eq!(*fake.goods_reads.borrow(), 4, "every good counts as one");
    }

    #[test]
    fn the_fake_gives_its_sites_systems_and_records_the_systems_asked() {
        let fake = catalog();
        assert!(fake.system_exists(SystemId(130)));
        assert!(fake.system_exists(SystemId(131)));
        assert!(!fake.system_exists(SystemId(132)), "on the map, unreadable");
        assert_eq!(fake.landing_sites(SystemId(131)), [planet(140, 0.0, 0.0)]);
        assert_eq!(
            fake.landing_sites(SystemId(130)),
            [planet(128, 30.0, -40.0), planet(129, 2000.0, 0.0)]
        );
        assert_eq!(fake.landing_sites(SystemId(133)), []);
        assert_eq!(
            *fake.sites_asked.borrow(),
            [SystemId(131), SystemId(130), SystemId(133)]
        );
        assert_eq!(
            fake.first_stellars(),
            [
                (SystemId(130), StellarId(128)),
                (SystemId(131), StellarId(140))
            ]
        );
        assert_eq!(fake.fallback_frame_size(), Some((100, 100)));
        let unread = FakePilotCatalog {
            fallback_frame: None,
            ..catalog()
        };
        assert_eq!(unread.fallback_frame_size(), None);
    }

    #[test]
    fn the_fake_marks_each_listed_stellar_it_isnt_given_by_its_id() {
        let given = StellarMark {
            id: StellarId(128),
            place: (5, 6),
            name: "Earth".to_owned(),
        };
        let fake = FakePilotCatalog {
            marks: vec![given.clone()],
            unloaded: vec![StellarId(129)],
            ..catalog()
        };
        assert_eq!(
            fake.stellar_marks(),
            [
                given,
                StellarMark {
                    id: StellarId(140),
                    place: (0, 0),
                    name: "#140".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn the_fake_gives_its_story_records_and_sounds() {
        use super::super::{cron, mission, nebula, rank};
        let texts = MissionTexts {
            id: crate::catalog::MissionId(128),
            name: "Deliver".to_owned(),
            offer: Some("Take it".to_owned()),
            brief: None,
            quick_brief: None,
            comp: None,
            fail: None,
            refuse: None,
            refuse_named: false,
            load_cargo: None,
            dump_cargo: None,
            ship_done: None,
            accept_button: String::new(),
            refuse_button: String::new(),
            graphics: crate::catalog::TextGraphics::default(),
            movies: crate::catalog::TextMovies::default(),
        };
        let visibility = SystemVisibility {
            id: SystemId(131),
            visibility: crate::control::Test::parse("b1"),
        };
        let sound = |frames| SoundLength { frames, rate: 22 };
        let dude = DudeRecord {
            ai_type: 3,
            ..DudeRecord::default()
        };
        let fake = FakePilotCatalog {
            missions: vec![mission(128)],
            mission_texts: vec![texts.clone()],
            visibility: vec![visibility.clone()],
            crons: vec![cron(130)],
            ranks: vec![rank(131)],
            nebulae: vec![nebula(132, (0, 0, 10, 10), "", "")],
            warp_sound: Some(sound(1)),
            warp_sound_double_time: Some(sound(2)),
            warp_out_sound: Some(sound(3)),
            boom_sounds: vec![(BoomId(130), SoundId(302))],
            dudes: vec![(DudeId(140), dude.clone())],
            voices: Voices::read(&[], &[(crate::catalog::GovtId(128), 4)]),
            ..catalog()
        };
        assert_eq!(fake.missions(), [mission(128)]);
        assert_eq!(fake.mission_texts(), [texts]);
        assert_eq!(fake.system_visibility(), [visibility]);
        assert_eq!(fake.crons(), [cron(130)]);
        assert_eq!(fake.ranks(), [rank(131)]);
        assert_eq!(fake.nebulae(), [nebula(132, (0, 0, 10, 10), "", "")]);
        assert_eq!(fake.warp_sound(), Some(sound(1)));
        assert_eq!(fake.warp_sound_double_time(), Some(sound(2)));
        assert_eq!(fake.warp_out_sound(), Some(sound(3)));
        assert_eq!(fake.boom_sounds(), [(BoomId(130), SoundId(302))]);
        assert_eq!(
            fake.voices().set_of(Some(crate::catalog::GovtId(128))),
            Some(4)
        );
        assert_eq!(fake.mission_dude(DudeId(140)), Some(dude));
        assert_eq!(fake.mission_dude(DudeId(141)), None);
    }

    #[test]
    fn the_fake_gives_its_traffic_by_system_and_its_dudes_by_id() {
        use super::super::person;
        let traffic = |avg_ships| SystemTraffic {
            avg_ships,
            ..SystemTraffic::default()
        };
        let dude = |ai_type| DudeRecord {
            ai_type,
            ..DudeRecord::default()
        };
        let fleet = FleetRecord {
            id: crate::catalog::FleetId(128),
            lead: Some(ShipId(128)),
            escorts: Vec::new(),
            govt: None,
            link_syst: -1,
            appear_on: crate::control::Test::parse(""),
            quote: None,
            flags: 0,
        };
        let fake = FakePilotCatalog {
            traffic: vec![(SystemId(130), traffic(4)), (SystemId(131), traffic(9))],
            dudes: vec![(DudeId(128), dude(1)), (DudeId(129), dude(2))],
            fleets: vec![fleet.clone()],
            persons: vec![person(128, 128)],
            ..catalog()
        };
        assert_eq!(fake.system_traffic(SystemId(131)), Some(traffic(9)));
        assert_eq!(fake.system_traffic(SystemId(130)), Some(traffic(4)));
        assert_eq!(fake.system_traffic(SystemId(132)), None);
        assert_eq!(fake.dude(DudeId(129)), Some(dude(2)));
        assert_eq!(fake.dude(DudeId(128)), Some(dude(1)));
        assert_eq!(fake.dude(DudeId(130)), None);
        assert_eq!(fake.fleets(), [fleet]);
        assert_eq!(fake.persons(), [person(128, 128)]);
    }

    #[test]
    fn the_fake_gives_its_reinforcements_by_system() {
        let kept = crate::catalog::SystemReinforcements {
            fleet: crate::catalog::FleetId(129),
            time: 900,
            interval: 2,
        };
        let fake = FakePilotCatalog {
            reinforcements: vec![(SystemId(130), kept)],
            ..catalog()
        };
        assert_eq!(fake.system_reinforcements(SystemId(130)), Some(kept));
        assert_eq!(fake.system_reinforcements(SystemId(131)), None);
        assert_eq!(catalog().system_reinforcements(SystemId(130)), None);
    }

    #[test]
    fn the_fake_gives_its_combat_and_asteroid_records() {
        use super::super::{govt, hull, weapon};
        let rock = AsteroidTypeRecord {
            kind: 2,
            strength: 30,
            spin_rate: 100,
            frames: 36,
            frame_size: Some((40, 40)),
            yield_type: 3,
            yield_qty: 2,
            part_count: 5,
            part_color: 0x00ff_8800,
            frag_types: [-1, -1],
            frag_count: 0,
            explode_type: -1,
            mass: 50,
        };
        let field = AsteroidFieldRecord {
            system: SystemId(130),
            count: 6,
            types: 0b100,
        };
        let mut box_frames = [None; BOX_GRAPHICS];
        box_frames[1] = Some((16, 12));
        let fake = FakePilotCatalog {
            weapons: vec![weapon(128)],
            hulls: vec![hull(129)],
            govts: vec![govt(130)],
            asteroid_types: vec![rock],
            asteroid_fields: vec![field],
            box_frames,
            ..catalog()
        };
        assert_eq!(fake.weapons(), [weapon(128)]);
        assert_eq!(fake.hulls(), [hull(129)]);
        assert_eq!(fake.governments(), [govt(130)]);
        assert_eq!(fake.asteroid_types(), [rock]);
        assert_eq!(fake.asteroid_fields(), [field]);
        assert_eq!(fake.box_frame_sizes(), box_frames);
        assert_eq!(catalog().box_frame_sizes(), [None; BOX_GRAPHICS]);
    }

    #[test]
    fn the_fake_gives_its_strings_and_patches_by_id() {
        let fake = FakePilotCatalog {
            strings: vec![
                (7101, vec!["Hello".to_owned(), "Bye".to_owned()]),
                (crate::buoy::BUOY_STRINGS, vec!["Buoy".to_owned()]),
            ],
            string_patches: vec![
                (5000, StringPatch::Text("Patched".to_owned())),
                (5001, StringPatch::Empty),
            ],
            ..catalog()
        };
        assert_eq!(fake.string_list(7101), ["Hello", "Bye"]);
        assert_eq!(fake.string_list(7102), Vec::<String>::new());
        assert_eq!(fake.buoy_strings(), ["Buoy"]);
        assert_eq!(fake.string_patch(5001), StringPatch::Empty);
        assert_eq!(
            fake.string_patch(5000),
            StringPatch::Text("Patched".to_owned())
        );
        assert_eq!(fake.string_patch(5002), StringPatch::Absent);
        assert_eq!(
            fake.plugin_string(5000),
            StringPatch::Text("Patched".to_owned())
        );
        assert_eq!(fake.plugin_string(5002), StringPatch::Absent);
    }
}
