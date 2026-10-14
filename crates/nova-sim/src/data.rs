//! The pilot, traffic, combat and comm catalogs over the game data: a
//! thin mapping from `GameData`'s `chär`, `shïp`, `shän`, `oütf`, `wëap`,
//! `sÿst`, `spöb`, `jünk`, `öops`, `düde`, `flët`, `gövt` and `mïsn` records, its
//! string lists and its stellar sprites; and a `shän`'s blink fields as a
//! [`Blink`].

use std::collections::BTreeMap;

use nova_data::records::character::Character;
use nova_data::records::cron::Cron;
use nova_data::records::desc::Desc;
use nova_data::records::disaster::Disaster;
use nova_data::records::dude::Dude;
use nova_data::records::fleet::Fleet;
use nova_data::records::govt::Govt;
use nova_data::records::junk::Junk;
use nova_data::records::mission::Mission;
use nova_data::records::outfit::Outfit;
use nova_data::records::person::Person;
use nova_data::records::rank::Rank;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::string::StrResource;
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;
use nova_data::records::weapon::Weapon;
use nova_data::{GameData, Record, stellar_spin_id};

use crate::blink::Blink;
use crate::catalog::{
    AuxShip, CharacterStart, CombatCatalog, CommCatalog, CommodityStrings, CronId, CronRecord,
    DateAffixes, DisasterId, DisasterRecord, DudeId, DudeRecord, EscortRecord, FleetId,
    FleetRecord, GateSite, GovtId, GovtRecord, HullRecord, JunkRecord, LandingSite, ListedStellar,
    MissionId, MissionRecord, MissionShip, MissionTexts, OutfitId, OutfitRecord, Penalties,
    PersonId, PersonRecord, PersonWeapon, PilotCatalog, PlaceNames, RankId, RankRecord, ShipId,
    ShipRecord, SoundId, StarSystem, StartDate, StartError, StellarId, StellarLife, StellarMark,
    StockWeapon, StringPatch, SystemId, SystemTraffic, SystemVisibility, TrafficCatalog, WeaponId,
    WeaponRecord, loaded_mod_val,
};
use crate::combat::weapon::Explosion;
use crate::geometry::Vec2;
use crate::handling::ShipFields;

/// Reads the records afresh on every call; a session asks once, when it
/// starts.
///
/// The starting date's day and month are the `chär`'s two words at 0x134
/// and 0x136, which the Bible leaves undocumented and the ResForge template
/// labels the starting day and month (stock: 23 and 6, with `StartYear`
/// 1177).
impl PilotCatalog for GameData {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        let (_, first) = self
            .records::<Character>()
            .next()
            .ok_or(StartError::NoCharacter)?;
        let character = first.map_err(|err| StartError::Character(err.to_string()))?;
        Ok(CharacterStart {
            ship: character.record.ship_type,
            systems: character.record.system,
            start: StartDate {
                day: character.record.unknown_0x134,
                month: character.record.unknown_0x136,
                year: character.record.start_year,
            },
            cash: character.record.cash,
            legal: std::array::from_fn(|slot| {
                character.record.govt[slot].map(|govt| (govt, character.record.status[slot]))
            }),
            kills: character.record.kills,
            on_start: self.set_expr(character.record.on_start.as_str()).into(),
        })
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        match self.get::<Ship>(id.0) {
            Some(Ok(ship)) => Ok(ship_fields(ship.record)),
            Some(Err(err)) => Err(err.to_string()),
            None => Err(format!("no shïp {}", id.0)),
        }
    }

    fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
        let Some(Ok(ship)) = self.get::<Ship>(id.0) else {
            return Vec::new();
        };
        default_items(ship.record)
    }

    fn ships(&self) -> Vec<ShipRecord> {
        self.records::<Ship>()
            .filter_map(|(id, ship)| {
                let ship = ship.ok()?;
                let record = ship.record;
                let short_name = record.short_name.as_str().to_owned();
                Some(ShipRecord {
                    id: ShipId(id),
                    name: ship
                        .name
                        .map_or_else(|| short_name.replace("\\n", " "), shown_name),
                    short_name,
                    long_name: record.long_name.as_str().to_owned(),
                    fields: ship_fields(record),
                    defaults: default_items(record),
                    cost: record.cost,
                    tech_level: record.tech_level,
                    buy_random: record.buy_random,
                    hire_random: record.hire_random,
                    require: record.require.bits(),
                    availability: self.test_expr(record.availability.as_str()).into(),
                    appear_on: self.test_expr(record.appear_on.as_str()).into(),
                    on_purchase: self.set_expr(record.on_purchase.as_str()).into(),
                    on_capture: self.set_expr(record.on_capture.as_str()).into(),
                    on_retire: self.set_expr(record.on_retire.as_str()).into(),
                    flags3: record.flags3.bits(),
                    disp_weight: record.disp_weight,
                    max_gun: record.max_gun,
                    max_tur: record.max_tur,
                    length: record.length,
                    crew: record.crew,
                    inherent_ai: record.inherent_ai,
                    comm_name: record.comm_name.as_str().to_owned(),
                    inherent_govt: inherent_govt(record.inherent_govt),
                    combat_govt: combat_govt(record.inherent_govt),
                    escort_type: record.escort_type,
                })
            })
            .collect()
    }

    fn outfits(&self) -> Vec<OutfitRecord> {
        self.records::<Outfit>()
            .filter_map(|(id, outfit)| {
                let outfit = outfit.ok()?;
                let record = outfit.record;
                Some(OutfitRecord {
                    id: OutfitId(id),
                    name: outfit
                        .name
                        .map_or_else(|| record.lc_name.as_str().to_owned(), str::to_owned),
                    short_name: record.short_name.as_str().to_owned(),
                    disp_weight: record.disp_weight,
                    mass: record.mass,
                    tech_level: record.tech_level,
                    max: record.max,
                    flags: record.flags.bits(),
                    cost: record.cost,
                    mods: [
                        (record.mod_type, record.mod_val),
                        (record.mod_type2, record.mod_val2),
                        (record.mod_type3, record.mod_val3),
                        (record.mod_type4, record.mod_val4),
                    ]
                    .map(|(mod_type, mod_val)| (mod_type, loaded_mod_val(mod_type, mod_val))),
                    contribute: record.contribute.bits(),
                    require: record.require.bits(),
                    require_govt: record.require_govt,
                    buy_random: record.buy_random,
                    availability: self.test_expr(record.availability.as_str()).into(),
                    on_purchase: self.set_expr(record.on_purchase.as_str()).into(),
                    on_sell: self.set_expr(record.on_sell.as_str()).into(),
                    item_class: record.item_class,
                    lc_name: record.lc_name.as_str().to_owned(),
                    lc_plural: record.lc_plural.as_str().to_owned(),
                })
            })
            .collect()
    }

    fn system_exists(&self, id: SystemId) -> bool {
        matches!(self.get::<System>(id.0), Some(Ok(_)))
    }

    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
        let Some(Ok(system)) = self.get::<System>(system.0) else {
            return Vec::new();
        };
        system
            .record
            .nav_def
            .into_iter()
            .flatten()
            .filter_map(|id| {
                let stellar = self.get::<Stellar>(id.0)?.ok()?.record;
                let frame_size = self
                    .stellar_sprite(id)
                    .ok()
                    .map(|sprite| (sprite.sheet.frame_width(), sprite.sheet.frame_height()));
                let dead_frame_size = match self.stellar_dead_sprite(id) {
                    Ok(Some(sprite)) => {
                        Some((sprite.sheet.frame_width(), sprite.sheet.frame_height()))
                    }
                    _ => None,
                };
                let mut special_tech = [0; 8];
                special_tech[..3].copy_from_slice(&stellar.special_tech1_3);
                special_tech[3..].copy_from_slice(&stellar.special_tech4_8);
                Some(LandingSite {
                    id,
                    position: Vec2::new(f32::from(stellar.x_pos), f32::from(stellar.y_pos)),
                    frame_size,
                    flags: stellar.flags.bits(),
                    min_status: stellar.min_status,
                    landing_sound: landing_sound(stellar.cust_snd_id),
                    tech_level: stellar.tech_level,
                    special_tech,
                    govt: stellar.govt,
                    flags2: stellar.flags2.bits(),
                    destroyed: false,
                    graphic: stellar.graphic_type,
                    dead_graphic: dead_frame_size.map(|_| stellar.dead_type),
                    dead_frame_size,
                    type_counts_frames: counts_frames(self, stellar.graphic_type),
                    exit_angle: stellar.cust_snd_id,
                })
            })
            .collect()
    }

    fn star_map(&self) -> Vec<StarSystem> {
        self.records::<System>()
            .filter_map(|(id, system)| {
                let system = system.ok()?.record;
                Some(StarSystem {
                    id: SystemId(id),
                    position: Vec2::new(f32::from(system.x_pos), f32::from(system.y_pos)),
                    links: system.con.into_iter().flatten().collect(),
                    govt: system.govt,
                    interference: system.interference,
                    stellars: system
                        .nav_def
                        .iter()
                        .map(|slot| {
                            let stellar = self.get::<Stellar>(slot.as_ref()?.0)?.ok()?.record;
                            Some((stellar.flags.bits(), stellar.flags2.bits()))
                        })
                        .collect(),
                })
            })
            .collect()
    }

    fn commodity_strings(&self) -> CommodityStrings {
        CommodityStrings {
            names: strings(self, COMMODITY_NAMES),
            name_patches: patches(self, NAME_PATCHES),
            base_prices: strings(self, BASE_PRICES),
            price_patches: patches(self, PRICE_PATCHES),
        }
    }

    fn junk(&self) -> Vec<JunkRecord> {
        self.records::<Junk>()
            .filter_map(|(id, junk)| {
                let junk = junk.ok()?;
                let record = junk.record;
                let stellars = |slots: &[Option<_>]| slots.iter().flatten().copied().collect();
                Some(JunkRecord {
                    id: nova_data::JunkId(id),
                    name: junk
                        .name
                        .map_or_else(|| record.lc_name.as_str().to_owned(), str::to_owned),
                    base_price: record.base_price,
                    sold_at: stellars(&record.sold_at),
                    bought_at: stellars(&record.bought_at),
                    buy_on: self.test_expr(record.buy_on.as_str()).into(),
                    sell_on: self.test_expr(record.sell_on.as_str()).into(),
                    flags: record.flags.bits(),
                })
            })
            .collect()
    }

    fn date_affixes(&self) -> DateAffixes {
        match self.records::<Character>().next() {
            Some((_, Ok(character))) => DateAffixes {
                prefix: character.record.date_prefix.as_str().to_owned(),
                suffix: character.record.date_suffix.as_str().to_owned(),
            },
            _ => DateAffixes::default(),
        }
    }

    fn gate_sites(&self) -> Vec<GateSite> {
        // Systems come by ascending ID, so the first to list a stellar is
        // the lowest.
        let mut listed: BTreeMap<StellarId, SystemId> = BTreeMap::new();
        for (id, system) in self.records::<System>() {
            let Ok(system) = system else {
                continue;
            };
            for stellar in system.record.nav_def.into_iter().flatten() {
                listed.entry(stellar).or_insert(SystemId(id));
            }
        }
        listed
            .into_iter()
            .filter_map(|(id, system)| {
                let stellar = self.get::<Stellar>(id.0)?.ok()?.record;
                Some(GateSite {
                    id,
                    system,
                    position: Vec2::new(f32::from(stellar.x_pos), f32::from(stellar.y_pos)),
                    flags2: stellar.flags2.bits(),
                    links: stellar.hyper_link,
                    exit_angle: stellar.cust_snd_id,
                })
            })
            .collect()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        self.records::<Disaster>()
            .filter_map(|(id, disaster)| {
                let disaster = disaster.ok()?;
                let record = disaster.record;
                Some(DisasterRecord {
                    id: DisasterId(id),
                    // The loader keeps it up to its last ';'
                    // (_LoadObjectData @0x7cdd3-0x7ce4d).
                    name: shown_resource_name(disaster.name.unwrap_or_default()).to_owned(),
                    // The decoder reads -1 (any stellar) as none.
                    stellar: record.stellar.map_or(-1, |stellar| stellar.0),
                    commodity: record.commodity,
                    price_delta: record.price_delta,
                    duration: record.duration,
                    freq: record.freq,
                    activate_on: self.test_expr(record.activate_on.as_str()).into(),
                })
            })
            .collect()
    }

    fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
        self.records::<Stellar>()
            .filter_map(|(id, stellar)| Some((StellarId(id), stellar.ok()?.record.flags.bits())))
            .collect()
    }

    fn stellar_lives(&self) -> Vec<StellarLife> {
        self.records::<Stellar>()
            .filter_map(|(id, stellar)| {
                let stellar = stellar.ok()?.record;
                Some(StellarLife {
                    id: StellarId(id),
                    strength: stellar.strength,
                    dead_time: stellar.dead_time,
                    flags2: stellar.flags2.bits(),
                    explosion: Explosion::decode(stellar.explod_type),
                    on_destroy: self.set_expr(stellar.on_destroy.as_str()).into(),
                    on_regen: self.set_expr(stellar.on_regen.as_str()).into(),
                })
            })
            .collect()
    }

    fn fallback_frame_size(&self) -> Option<(u32, u32)> {
        let sheet = self.spin_sheet(stellar_spin_id(0)?).ok()?.sheet;
        Some((sheet.frame_width(), sheet.frame_height()))
    }

    /// Each `Visibility` parsed once and shared
    /// ([`GameData::test_expr`]); a blank one always holds.
    fn system_visibility(&self) -> Vec<SystemVisibility> {
        self.records::<System>()
            .filter_map(|(id, system)| {
                let system = system.ok()?.record;
                Some(SystemVisibility {
                    id: SystemId(id),
                    visibility: self.test_expr(system.visibility.as_str()).into(),
                })
            })
            .collect()
    }

    fn listed_stellars(&self) -> Vec<ListedStellar> {
        // Systems come by ascending ID, so each stellar's come in order.
        let mut listed: BTreeMap<StellarId, Vec<SystemId>> = BTreeMap::new();
        for (id, system) in self.records::<System>() {
            let Ok(system) = system else {
                continue;
            };
            for stellar in system.record.nav_def.into_iter().flatten() {
                let systems = listed.entry(stellar).or_default();
                if !systems.contains(&SystemId(id)) {
                    systems.push(SystemId(id));
                }
            }
        }
        listed
            .into_iter()
            .filter_map(|(id, systems)| {
                let stellar = self.get::<Stellar>(id.0)?.ok()?.record;
                Some(ListedStellar {
                    id,
                    systems,
                    flags: stellar.flags.bits(),
                    flags2: stellar.flags2.bits(),
                    govt: stellar.govt,
                })
            })
            .collect()
    }

    /// Each name up to any ';', as the game shows it; a record with no
    /// name is named empty.
    fn place_names(&self) -> PlaceNames {
        PlaceNames {
            stellars: self
                .records::<Stellar>()
                .filter_map(|(id, stellar)| {
                    let name = stellar.ok()?.name.map(shown_name).unwrap_or_default();
                    Some((StellarId(id), name))
                })
                .collect(),
            systems: self
                .records::<System>()
                .filter_map(|(id, system)| {
                    let name = system.ok()?.name.map(shown_name).unwrap_or_default();
                    Some((SystemId(id), name))
                })
                .collect(),
        }
    }

    /// Each `spöb`'s `xPos`/`yPos` (@0x7733b-0x7734d) and its name as
    /// `_LoadObjectData` keeps it (@0x772ab-0x7732a): up to its last ';',
    /// trailing spaces and ';'s dropped, empty when it has none.
    fn stellar_marks(&self) -> Vec<StellarMark> {
        self.records::<Stellar>()
            .filter_map(|(id, stellar)| {
                let stellar = stellar.ok()?;
                Some(StellarMark {
                    id: StellarId(id),
                    place: (stellar.record.x_pos, stellar.record.y_pos),
                    name: stellar
                        .name
                        .map(shown_resource_name)
                        .unwrap_or_default()
                        .to_owned(),
                })
            })
            .collect()
    }

    /// Each text from its `dësc`, none for -1 or one that cannot be read;
    /// the offer's is `dësc` 4000 + the mission's index (@0xa3a85-0xa3aad).
    fn mission_texts(&self) -> Vec<MissionTexts> {
        let text = |id: Option<i16>| -> Option<String> {
            let desc = self.get::<Desc>(id?)?.ok()?;
            Some(desc.record.text.as_str().to_owned())
        };
        self.records::<Mission>()
            .filter_map(|(id, mission)| {
                let mission = mission.ok()?;
                let record = mission.record;
                let raw = |desc: Option<nova_data::DescId>| text(desc.map(|desc| desc.0));
                Some(MissionTexts {
                    id: MissionId(id),
                    name: shown_resource_name(mission.name.unwrap_or_default()).to_owned(),
                    offer: text(
                        id.checked_sub(FIRST_MISSION)
                            .and_then(|index| FIRST_OFFER_DESC.checked_add(index)),
                    ),
                    brief: raw(record.brief_text),
                    quick_brief: raw(record.quick_brief),
                    comp: raw(record.comp_text),
                    fail: raw(record.fail_text),
                    refuse: raw(record.refuse_text),
                    load_cargo: raw(record.load_carg_text),
                    dump_cargo: raw(record.dump_cargo_text),
                    ship_done: raw(record.ship_done_text),
                    accept_button: record.accept_button.as_str().to_owned(),
                    refuse_button: record.refuse_button.as_str().to_owned(),
                })
            })
            .collect()
    }

    fn missions(&self) -> Vec<MissionRecord> {
        self.records::<Mission>()
            .filter_map(|(id, mission)| {
                let record = mission.ok()?.record;
                let comp_govt = record
                    .comp_govt
                    .filter(|govt| (128..=383).contains(&govt.0));
                Some(MissionRecord {
                    id: MissionId(id),
                    avail_stel: record.avail_stel,
                    avail_loc: record.avail_loc,
                    avail_record: record.avail_record,
                    avail_rating: record.avail_rating,
                    avail_random: record.avail_random,
                    avail_ship_type: record.avail_ship_type,
                    avail_bits: self.test_expr(record.avail_bits.as_str()).into(),
                    require: record.require.bits(),
                    flags: record.flags.bits(),
                    flags2: record.flags2.bits(),
                    travel_stel: record.travel_stel,
                    return_stel: record.return_stel,
                    cargo_type: record.cargo_type,
                    cargo_qty: record.cargo_qty,
                    pickup_mode: record.pickup_mode,
                    drop_off_mode: record.drop_off_mode,
                    scan_mask: record.scan_mask.bits(),
                    pay_val: record.pay_val,
                    time_limit: record.time_limit,
                    disp_weight: record.disp_weight,
                    on_accept: self.set_expr(record.on_accept.as_str()).into(),
                    on_refuse: self.set_expr(record.on_refuse.as_str()).into(),
                    on_success: self.set_expr(record.on_success.as_str()).into(),
                    on_failure: self.set_expr(record.on_failure.as_str()).into(),
                    on_abort: self.set_expr(record.on_abort.as_str()).into(),
                    on_ship_done: self.set_expr(record.on_ship_done.as_str()).into(),
                    can_abort: record.can_abort,
                    comp_govt,
                    comp_reward: if comp_govt.is_some() {
                        record.comp_reward
                    } else {
                        0
                    },
                    date_post_inc: record.date_post_inc,
                    ships: mission_ship(record),
                    aux: AuxShip {
                        count: record.aux_ship_count,
                        dude: record.aux_ship_dude,
                        syst: record.aux_ship_syst,
                    },
                })
            })
            .collect()
    }

    fn crons(&self) -> Vec<CronRecord> {
        self.records::<Cron>()
            .filter_map(|(id, cron)| {
                let record = cron.ok()?.record;
                Some(CronRecord {
                    id: CronId(id),
                    first_day: record.first_day,
                    first_month: record.first_month,
                    first_year: record.first_year,
                    last_day: record.last_day,
                    last_month: record.last_month,
                    last_year: record.last_year,
                    random: record.random,
                    duration: record.duration,
                    pre_holdoff: record.pre_holdoff,
                    post_holdoff: record.post_holdoff,
                    flags: record.flags.bits(),
                    enable_on: self.test_expr(record.enable_on.as_str()).into(),
                    on_start: self.set_expr(record.on_start.as_str()).into(),
                    on_end: self.set_expr(record.on_end.as_str()).into(),
                    contribute: record.contribute.bits(),
                    require: record.require.bits(),
                    news_govt: record.news_govt.map(|id| id.map_or(-1, |id| id.0)),
                    govt_news: record.govt_news_str.map(|id| id.map_or(-1, |id| id.0)),
                    ind_news: record.ind_news_str.map_or(-1, |id| id.0),
                })
            })
            .collect()
    }

    /// Only IDs 128-255 have a slot (@0x7da37); `AffilGovt` up to 127
    /// loads as -1 (@0x7dab0-0x7dac3), and a `PriceMod` of 0 or below as
    /// 100 (@0x7db33-0x7db3a).
    fn ranks(&self) -> Vec<RankRecord> {
        self.records::<Rank>()
            .filter(|(id, _)| (FIRST_RANK..=LAST_RANK).contains(id))
            .filter_map(|(id, rank)| {
                let rank = rank.ok()?;
                let record = rank.record;
                Some(RankRecord {
                    id: RankId(id),
                    name: shown_resource_name(rank.name.unwrap_or_default()).to_owned(),
                    weight: record.weight,
                    affil_govt: record.affil_govt.filter(|govt| govt.0 > 127),
                    salary: record.salary.into(),
                    salary_cap: record.salary_cap.into(),
                    contribute: record.contribute.bits(),
                    flags: record.flags.bits(),
                    conv_name: record.conv_name.as_str().to_owned(),
                    short_name: record.short_name.as_str().to_owned(),
                    price_mod: if record.price_mod <= 0 {
                        100
                    } else {
                        record.price_mod
                    },
                })
            })
            .collect()
    }
}

/// The first `ränk` ID the original's table holds.
const FIRST_RANK: i16 = 128;
/// The last: the table holds 128 slots.
const LAST_RANK: i16 = 255;

/// Whether graphic type `graphic`'s `spïn` in `data` reads and gives a
/// frame count above 0, its `xTiles` × `yTiles` in 32 bits, as the
/// original's `_LoadMasterStellarSprite` multiplies them
/// (@0x1ed8c-0x1ed97).
fn counts_frames(data: &GameData, graphic: i16) -> bool {
    stellar_spin_id(graphic)
        .and_then(|spin| data.get::<Spin>(spin)?.ok())
        .is_some_and(|spin| i32::from(spin.record.x_tiles) * i32::from(spin.record.y_tiles) > 0)
}

/// Reads the records afresh on every call; a session asks each time it
/// populates a system.
impl TrafficCatalog for GameData {
    fn system_traffic(&self, id: SystemId) -> Option<SystemTraffic> {
        let system = self.get::<System>(id.0)?.ok()?.record;
        Some(SystemTraffic {
            dude_types: std::array::from_fn(|slot| (system.dude_types[slot], system.prob[slot])),
            avg_ships: system.avg_ships,
            persons: std::array::from_fn(|slot| {
                let person = system.person[slot].filter(|person| person.0 > LAST_UNUSED);
                (person, system.person_prob[slot])
            }),
        })
    }

    fn dude(&self, id: DudeId) -> Option<DudeRecord> {
        let dude = self.get::<Dude>(id.0)?.ok()?.record;
        Some(DudeRecord {
            ai_type: dude.ai_type,
            govt: dude.govt,
            ships: dude
                .ship_type
                .iter()
                .zip(dude.probability)
                .filter_map(|(ship, probability)| Some(((*ship)?, probability)))
                .collect(),
            booty: dude.booty.0,
            info_types: dude.info_types.0,
        })
    }

    fn fleets(&self) -> Vec<FleetRecord> {
        self.records::<Fleet>()
            .filter_map(|(id, fleet)| {
                let fleet = fleet.ok()?.record;
                Some(FleetRecord {
                    id: FleetId(id),
                    lead: fleet.lead_ship_type,
                    escorts: (0..fleet.escort_type.len())
                        .filter_map(|slot| {
                            Some(EscortRecord {
                                ship: fleet.escort_type[slot]?,
                                min: fleet.min[slot],
                                max: fleet.max[slot],
                            })
                        })
                        .collect(),
                    govt: fleet.govt,
                    link_syst: fleet.link_syst,
                    appear_on: self.test_expr(fleet.appear_on.as_str()).into(),
                })
            })
            .collect()
    }

    fn persons(&self) -> Vec<PersonRecord> {
        self.records::<Person>()
            .filter_map(|(id, person)| {
                let person = person.ok()?;
                let record = person.record;
                let weapons = (0..record.weap_type.len())
                    .filter_map(|slot| {
                        let weapon =
                            record.weap_type[slot].filter(|weapon| weapon.0 > LAST_UNUSED)?;
                        Some(PersonWeapon {
                            weapon,
                            count: record.weap_count[slot],
                            ammo: record.ammo_load[slot],
                        })
                    })
                    .collect();
                Some(PersonRecord {
                    id: PersonId(id),
                    name: person.name.map(person_name).unwrap_or_default(),
                    link_syst: record.link_syst,
                    govt: record.govt,
                    ai_type: record.ai_type,
                    aggress: record.aggress,
                    coward: record.coward,
                    ship: record.ship_type,
                    weapons,
                    credits: record.credits,
                    shield_mod: record.shield_mod,
                    hail_pict: record
                        .hail_pict
                        .map(|pict| pict.0)
                        .filter(|&pict| pict > LAST_UNUSED),
                    comm_quote: record.comm_quote,
                    hail_quote: record.hail_quote,
                    link_mission: record
                        .link_mission
                        .map(|mission| mission.0)
                        .filter(|&mission| mission > LAST_UNUSED),
                    flags: record.flags.bits(),
                    active_on: self.test_expr(record.active_on.as_str()).into(),
                    subtitle: record.subtitle.as_str().to_owned(),
                    flags2: record.flags2.bits(),
                    grant_class: record.grant_class,
                    grant_count: record.grant_count,
                    grant_prob: record.grant_prob,
                    mission_ship: record
                        .link_mission
                        .and_then(|mission| self.get::<Mission>(mission.0)?.ok())
                        .map(|mission| mission_ship(mission.record)),
                })
            })
            .collect()
    }
}

/// A `mïsn`'s special ships, raw.
fn mission_ship(record: &Mission) -> MissionShip {
    MissionShip {
        count: record.ship_count,
        goal: record.ship_goal,
        syst: record.ship_syst,
        dude: record.ship_dude,
        behav: record.ship_behav,
        start: record.ship_start,
        name_id: record.ship_name_id.map(|id| id.0),
        subtitle: record.ship_subtitle.map(|id| id.0),
    }
}

/// The first mission offer text's `dësc`, mission index 0's.
const FIRST_OFFER_DESC: i16 = 4000;
/// The lowest `mïsn` ID: mission index 0.
const FIRST_MISSION: i16 = 128;

/// The highest ID a person's weapon, `HailPict` or `LinkMission`, or a
/// system's Person slot, names none at (`_LoadObjectData` keeps only
/// those above it).
const LAST_UNUSED: i16 = 127;

/// The most characters of a person's name the engine keeps (it copies 30
/// bytes, a Pascal string's length byte among them).
const PERSON_NAME_CHARS: usize = 29;

/// A `përs` resource's name as the game shows it: cut as
/// [`shown_resource_name`] cuts it, and at most [`PERSON_NAME_CHARS`]
/// characters (`_LoadObjectData` @0x7c57d-0x7c5f3).
fn person_name(name: &str) -> String {
    shown_resource_name(name)
        .chars()
        .take(PERSON_NAME_CHARS)
        .collect()
}

/// A resource name as the game shows a person's or a mission's: up to its
/// last ';', trailing spaces and ';'s dropped (`_TrimResourceName`
/// @0x717ac). A mission's is kept whole, as the original keeps 255
/// characters.
fn shown_resource_name(name: &str) -> &str {
    let shown = name.rfind(';').map_or(name, |at| &name[..at]);
    shown.trim_end_matches([' ', ';'])
}

/// Reads the records afresh on every call; a session asks once, when it
/// starts.
impl CombatCatalog for GameData {
    fn weapons(&self) -> Vec<WeaponRecord> {
        self.records::<Weapon>()
            .filter_map(|(id, weapon)| {
                let record = weapon.ok()?.record;
                Some(WeaponRecord {
                    id: WeaponId(id),
                    reload: record.reload,
                    count: record.count,
                    mass_dmg: record.mass_dmg,
                    energy_dmg: record.energy_dmg,
                    guidance: record.guidance,
                    speed: record.speed,
                    ammo_type: record.ammo_type,
                    inaccuracy: record.inaccuracy,
                    impact: record.impact,
                    explod_type: record.explod_type,
                    prox_radius: record.prox_radius,
                    blast_radius: record.blast_radius,
                    flags: record.flags.bits(),
                    seeker: record.seeker.bits(),
                    flags2: record.flags2.bits(),
                    flags3: record.flags3.bits(),
                    decay: record.decay,
                    beam_length: record.beam_length,
                    burst_count: record.burst_count,
                    burst_reload: record.burst_reload,
                    guided_turn: record.guided_turn,
                    durability: record.durability,
                    sub_count: record.sub_count,
                    sub_type: record.sub_type,
                    sub_theta: record.sub_theta,
                    sub_limit: record.sub_limit,
                    max_ammo: record.max_ammo,
                    prox_safety: record.prox_safety,
                })
            })
            .collect()
    }

    fn hulls(&self) -> Vec<HullRecord> {
        self.records::<Ship>()
            .filter_map(|(id, ship)| {
                let record = ship.ok()?.record;
                let (size, height) = match self.get::<ShipAnim>(id) {
                    Some(Ok(anim)) => {
                        (Some(anim.record.base_x_size), Some(anim.record.base_y_size))
                    }
                    _ => (None, None),
                };
                Some(HullRecord {
                    id: ShipId(id),
                    flags: record.flags.bits(),
                    death_delay: record.death_delay,
                    explode1: record.explode1,
                    explode2: record.explode2,
                    mass: record.mass,
                    weapons: stock_weapons(record),
                    size,
                    height,
                    strength: record.strength,
                })
            })
            .collect()
    }

    fn governments(&self) -> Vec<GovtRecord> {
        self.records::<Govt>()
            .filter_map(|(id, govt)| {
                let record = govt.ok()?.record;
                Some(GovtRecord {
                    id: GovtId(id),
                    flags: record.flags.bits(),
                    flags2: record.flags2.bits(),
                    crime_tol: record.crime_tol,
                    initial_rec: record.initial_rec,
                    penalties: Penalties {
                        smuggle: record.smug_penalty,
                        disable: record.disab_penalty,
                        board: record.board_penalty,
                        kill: record.kill_penalty,
                        shoot: record.shoot_penalty,
                    },
                    max_odds: record.max_odds,
                    classes: record.class,
                    allies: record.ally,
                    enemies: record.enemy,
                    comm_name: record.comm_name.as_str().to_owned(),
                    scan_mask: record.scan_mask.bits(),
                    scan_fine: record.scan_fine,
                    news_pic: record.news_pic,
                    require: record.require.bits(),
                })
            })
            .collect()
    }
}

/// Reads the string list afresh on every call.
impl CommCatalog for GameData {
    fn string_list(&self, id: i16) -> Vec<String> {
        strings(self, id)
    }
}

/// The most a `shïp`'s `InherentGovt` adds to a `gövt` ID to say it
/// takes on that government's attributes only.
const ATTRIBUTES_ONLY: i16 = 1000;

/// What a `shïp`'s `InherentGovt` adds to a `gövt` ID to say it fights as
/// that government only.
const COMBAT_ONLY: i16 = 2000;

/// A `shïp`'s `InherentGovt` as the government whose attributes it takes
/// on: a `gövt` ID as it is, or less [`ATTRIBUTES_ONLY`] when it is in
/// that range; none for -1, or a government it fights as only (+2000),
/// whose attributes it does not take on.
fn inherent_govt(raw: i16) -> Option<GovtId> {
    match raw {
        0..ATTRIBUTES_ONLY => Some(GovtId(raw)),
        ATTRIBUTES_ONLY..2000 => Some(GovtId(raw - ATTRIBUTES_ONLY)),
        _ => None,
    }
}

/// A `shïp`'s `InherentGovt` as the government it fights as: a `gövt` ID
/// as it is, or less [`COMBAT_ONLY`] when it is in that range; none for -1,
/// or a government whose attributes only it takes on (+1000).
fn combat_govt(raw: i16) -> Option<GovtId> {
    match raw {
        0..ATTRIBUTES_ONLY => Some(GovtId(raw)),
        COMBAT_ONLY..3000 => Some(GovtId(raw - COMBAT_ONLY)),
        _ => None,
    }
}

/// A resource's name as the game shows it: up to any ';', after which
/// the designers kept their notes (stock `shïp` 361 is "Shuttle;Second-Hand
/// - poor").
fn shown_name(name: &str) -> String {
    name.split(';').next().unwrap_or_default().to_owned()
}

/// A `shïp`'s `WeapType` slots 1-8 that name a weapon, each with its
/// `WeapCount` and `AmmoLoad`, in slot order: an unused slot (-1) or 0
/// names none.
fn stock_weapons(ship: &Ship) -> Vec<StockWeapon> {
    let types = ship.weap_type1_4.into_iter().chain(ship.weap_type5_8);
    let counts = ship.weap_count1_4.into_iter().chain(ship.weap_count5_8);
    let ammo = ship.ammo_load1_4.into_iter().chain(ship.ammo_load5_8);
    types
        .zip(counts.zip(ammo))
        .filter_map(|(weapon, (count, ammo))| {
            let weapon = weapon.filter(|weapon| weapon.0 != 0)?;
            Some(StockWeapon {
                weapon,
                count,
                ammo,
            })
        })
        .collect()
}

/// A `shän`'s `BlinkMode` and `BlinkValA`–`D`, raw.
impl From<&ShipAnim> for Blink {
    fn from(anim: &ShipAnim) -> Self {
        Self {
            mode: anim.blink_mode,
            a: anim.blink_val_a,
            b: anim.blink_val_b,
            c: anim.blink_val_c,
            d: anim.blink_val_d,
        }
    }
}

/// Ship `id`'s blink: its `shän`'s blink fields, or steady when the `shän`
/// is missing or cannot be read.
#[must_use]
pub fn ship_blink(data: &GameData, id: i16) -> Blink {
    match data.get::<ShipAnim>(id) {
        Some(Ok(anim)) => Blink::from(anim.record),
        _ => Blink::STEADY,
    }
}

/// A `shïp`'s handling, reserve, cargo and mass fields.
fn ship_fields(ship: &Ship) -> ShipFields {
    ShipFields {
        speed: ship.speed,
        accel: ship.accel,
        maneuver: ship.maneuver,
        shield: ship.shield,
        armor: ship.armor,
        fuel: ship.fuel,
        fuel_regen: ship.fuel_regen,
        holds: ship.holds,
        mass: ship.mass,
        free_mass: ship.free_mass,
        contribute: ship.contribute.bits(),
        shield_rech: ship.shield_rech,
        armor_rech: ship.armor_rech,
        flags2: ship.flags2.bits(),
    }
}

/// A `shïp`'s `DefaultItems`, each item with its count in slot order.
fn default_items(ship: &Ship) -> Vec<(OutfitId, u16)> {
    let items = ship
        .default_items1_4
        .into_iter()
        .chain(ship.default_items5_8);
    let counts = ship.item_count1_4.into_iter().chain(ship.item_count5_8);
    items
        .zip(counts)
        // A negative count carries none.
        .filter_map(|(item, count)| Some((item?, u16::try_from(count).unwrap_or(0))))
        .collect()
}

/// The `STR#` naming the standard commodities, "All Cargo".
const COMMODITY_NAMES: i16 = 4000;

/// The first `'STR '` patching `STR#` 4000, the Bible's Appendix III:
/// 9000 + n patches commodity n's name.
const NAME_PATCHES: i16 = 9000;

/// The `STR#` pricing them, "Base Prices".
const BASE_PRICES: i16 = 4004;

/// The first `'STR '` patching `STR#` 4004, the Bible's Appendix III:
/// 9300 + n patches commodity n's base price.
const PRICE_PATCHES: i16 = 9300;

/// `data`'s six `'STR '` patches from `first` up, raw. A zero-byte one is
/// [`StringPatch::Empty`]; any other that cannot be read is
/// [`StringPatch::Unreadable`]. Bytes past a readable string are ignored,
/// as the original reads only as many as its length byte says.
fn patches(data: &GameData, first: i16) -> [StringPatch; 6] {
    std::array::from_fn(|n| {
        let id = first + i16::try_from(n).expect("one of six");
        match data.get::<StrResource>(id) {
            None => StringPatch::Absent,
            Some(Ok(entry)) => StringPatch::Text(entry.record.text.as_str().to_owned()),
            Some(Err(_)) => {
                let bytes = data.resource(StrResource::TYPE, id);
                if bytes.is_some_and(|bytes| bytes.resource.data().is_empty()) {
                    StringPatch::Empty
                } else {
                    StringPatch::Unreadable
                }
            }
        }
    })
}

/// Every string of `data`'s `STR#` `id`; none when it is missing or
/// cannot be read.
fn strings(data: &GameData, id: i16) -> Vec<String> {
    let Some(Ok(list)) = data.get::<StrList>(id) else {
        return Vec::new();
    };
    list.record
        .strings
        .iter()
        .map(|string| string.as_str().to_owned())
        .collect()
}

/// The first stellar landing sound: the community *EV Nova Resource ID
/// Guide* gives `snd ` 10000 and up to "custom stellar landing sounds".
const FIRST_LANDING_SOUND: i16 = 10_000;

/// A `spöb`'s `CustSndID` as its landing sound: from
/// [`FIRST_LANDING_SOUND`] up. Anything below is none: -1, 0, and the
/// angle hypergates and wormholes keep in the field (stock 120).
fn landing_sound(cust_snd_id: i16) -> Option<SoundId> {
    (cust_snd_id >= FIRST_LANDING_SOUND).then_some(SoundId(cust_snd_id))
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;

    use nova_data::graphics::RLED;
    use nova_data::graphics::fixture::RledBuilder;
    use nova_data::records::disaster::Disaster;
    use nova_data::records::junk::Junk;
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::records::spin::Spin;
    use nova_data::records::string_list::StrList;
    use nova_data::records::weapon::Weapon;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};

    use super::*;
    use crate::catalog::{
        DisasterId, GateSite, GovtId, GovtRecord, HullRecord, JunkId, Penalties, StarSystem,
        StartDate, StellarId, StockWeapon, SystemVisibility, WeaponId, WeaponRecord,
    };
    use crate::combat::weapon::Explosion;
    use crate::control::{Script, Test, TestExpr};

    /// One data file, `/data/Nova Data`, holding a fork.
    struct OneFile(Vec<u8>);

    impl DirLister for OneFile {
        fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
            Ok(vec![Listing {
                name: "Nova Data".into(),
                kind: EntryKind::File,
            }])
        }
    }

    impl ForkReader for OneFile {
        fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            Ok((fork == Fork::Data).then(|| self.0.clone()))
        }
    }

    /// Whether two tests are the one parse, shared.
    fn shared(a: &Test, b: &Test) -> bool {
        std::sync::Arc::ptr_eq(&a.0, &b.0)
    }

    /// Whether two scripts are the one parse, shared.
    fn shared_script(a: &Script, b: &Script) -> bool {
        std::sync::Arc::ptr_eq(&a.0, &b.0)
    }

    fn store(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
        let named: Vec<_> = resources
            .iter()
            .map(|(ty, id, data)| (*ty, *id, None, data.clone()))
            .collect();
        store_named(&named)
    }

    /// The store of these resources, each with its name, if any.
    fn store_named(resources: &[(ResType, i16, Option<&str>, Vec<u8>)]) -> GameData {
        let fork = resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, name, data)| {
                fork.resource(*ty, *id, name.map(str::as_bytes), data)
            })
            .build()
            .bytes;
        let file = OneFile(fork);
        GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
    }

    fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
        for (i, value) in values.iter().enumerate() {
            bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
        }
    }

    /// A `chär` starting in `ship` in these systems, with no cash and no
    /// legal records.
    fn character(ship: i16, systems: [i16; 4]) -> Vec<u8> {
        let mut bytes = vec![0; Character::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x04, &[ship]);
        put_i16s(&mut bytes, 0x06, &systems);
        put_i16s(&mut bytes, 0x0E, &[-1; 4]);
        bytes
    }

    /// A `shïp` with this `Accel`, `Speed` and `Maneuver`.
    fn ship(accel: i16, speed: i16, maneuver: i16) -> Vec<u8> {
        let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x04, &[accel, speed, maneuver]);
        bytes
    }

    /// A `shïp` with this `Shield`, `Fuel` and `Armor`, and no handling.
    fn reserves(shield: i16, fuel: i16, armor: i16) -> Vec<u8> {
        let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x02, &[shield]);
        put_i16s(&mut bytes, 0x0A, &[fuel]);
        put_i16s(&mut bytes, 0x0E, &[armor]);
        bytes
    }

    fn system() -> Vec<u8> {
        vec![0; System::SIZE.expect("fixed")]
    }

    fn short(mut bytes: Vec<u8>) -> Vec<u8> {
        bytes.pop();
        bytes
    }

    #[test]
    fn the_first_chär_by_id_gives_its_ship_and_systems() {
        let data = store(&[
            (Character::TYPE, 129, character(200, [131, -1, -1, -1])),
            (Character::TYPE, 128, character(128, [-1, 999, 130, -1])),
        ]);
        assert_eq!(
            data.first_character(),
            Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [None, Some(SystemId(999)), Some(SystemId(130)), None],
                start: StartDate::default(),
                cash: 0,
                legal: [None; 4],
                kills: 0,
                on_start: Script::default(),
            })
        );
        let shipless = store(&[(Character::TYPE, 128, character(-1, [130, -1, -1, -1]))]);
        assert_eq!(shipless.first_character().map(|c| c.ship), Ok(None));
    }

    #[test]
    fn the_start_carries_the_chärs_on_start_parsed() {
        let mut bytes = character(128, [130, -1, -1, -1]);
        bytes[0x32..0x38].copy_from_slice(b"b5 !b6");
        let data = store(&[(Character::TYPE, 128, bytes)]);
        assert_eq!(
            data.first_character().map(|c| c.on_start),
            Ok(Script::parse("b5 !b6"))
        );
        let first = data.first_character().expect("reads").on_start;
        let again = data.first_character().expect("reads").on_start;
        assert!(shared_script(&first, &again), "parsed once");
    }

    #[test]
    fn the_start_date_is_the_chärs_day_month_and_year() {
        let mut bytes = character(128, [130, -1, -1, -1]);
        put_i16s(&mut bytes, 0x134, &[23, 6, 1177]);
        let data = store(&[(Character::TYPE, 128, bytes)]);
        assert_eq!(
            data.first_character().map(|c| c.start),
            Ok(StartDate {
                day: 23,
                month: 6,
                year: 1177
            })
        );
    }

    /// A `chär` whose `DatePrefix` (0x13A) and `DateSuffix` (0x14A) are
    /// `prefix` and `suffix`.
    fn dated(prefix: &[u8], suffix: &[u8]) -> Vec<u8> {
        let mut bytes = character(128, [130, -1, -1, -1]);
        bytes[0x13A..0x13A + prefix.len()].copy_from_slice(prefix);
        bytes[0x14A..0x14A + suffix.len()].copy_from_slice(suffix);
        bytes
    }

    #[test]
    fn the_date_affixes_are_the_first_chärs_prefix_and_suffix() {
        let data = store(&[
            (Character::TYPE, 129, dated(b"Era \0", b" AD\0")),
            (Character::TYPE, 128, dated(b"Year \0", b" NC\0")),
        ]);
        assert_eq!(
            data.date_affixes(),
            DateAffixes {
                prefix: "Year ".to_owned(),
                suffix: " NC".to_owned(),
            }
        );
    }

    #[test]
    fn no_chär_or_an_undecodable_one_has_no_date_affixes() {
        assert_eq!(store(&[]).date_affixes(), DateAffixes::default());
        let data = store(&[
            (Character::TYPE, 128, short(dated(b"Year \0", b" NC\0"))),
            (Character::TYPE, 129, dated(b"Era \0", b" AD\0")),
        ]);
        assert_eq!(data.date_affixes(), DateAffixes::default());
    }

    #[test]
    fn the_start_carries_the_chärs_cash_and_legal_records() {
        let mut bytes = character(128, [130, -1, -1, -1]);
        bytes[0x00..0x04].copy_from_slice(&(-7_000_i32).to_be_bytes());
        put_i16s(&mut bytes, 0x0E, &[129, -1, 131, 140]);
        put_i16s(&mut bytes, 0x16, &[50, 99, -20, 0]);
        let data = store(&[(Character::TYPE, 128, bytes)]);
        let start = data.first_character().expect("decodes");
        assert_eq!(start.cash, -7_000);
        assert_eq!(
            start.legal,
            [
                Some((GovtId(129), 50)),
                None,
                Some((GovtId(131), -20)),
                Some((GovtId(140), 0)),
            ]
        );
    }

    #[test]
    fn the_start_carries_the_chärs_kills_raw() {
        for kills in [100, -3] {
            let mut bytes = character(128, [130, -1, -1, -1]);
            put_i16s(&mut bytes, 0x1E, &[kills]);
            let data = store(&[(Character::TYPE, 128, bytes)]);
            assert_eq!(data.first_character().map(|c| c.kills), Ok(kills));
        }
    }

    #[test]
    fn no_chär_is_no_character() {
        assert_eq!(store(&[]).first_character(), Err(StartError::NoCharacter));
    }

    #[test]
    fn an_undecodable_first_chär_is_its_decode_error() {
        let data = store(&[
            (
                Character::TYPE,
                128,
                short(character(128, [130, -1, -1, -1])),
            ),
            (Character::TYPE, 129, character(128, [130, -1, -1, -1])),
        ]);
        let Err(StartError::Character(message)) = data.first_character() else {
            panic!("a decode error: {:?}", data.first_character())
        };
        assert!(message.contains("128"), "{message}");
    }

    #[test]
    fn a_ships_fields_are_its_speed_accel_and_maneuver() {
        let data = store(&[(Ship::TYPE, 128, ship(250, 400, 15))]);
        assert_eq!(
            data.ship_fields(ShipId(128)),
            Ok(ShipFields {
                speed: 400,
                accel: 250,
                maneuver: 15,
                ..ShipFields::default()
            })
        );
    }

    #[test]
    fn a_ships_fields_include_its_shield_armour_and_fuel() {
        let data = store(&[(Ship::TYPE, 128, reserves(30, 300, -45))]);
        assert_eq!(
            data.ship_fields(ShipId(128)),
            Ok(ShipFields {
                shield: 30,
                armor: -45,
                fuel: 300,
                ..ShipFields::default()
            })
        );
    }

    #[test]
    fn a_ships_fields_include_its_holds() {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x00, &[-10]);
        let data = store(&[(Ship::TYPE, 128, bytes)]);
        assert_eq!(data.ship_fields(ShipId(128)).map(|f| f.holds), Ok(-10));
    }

    #[test]
    fn a_ships_fields_include_its_fuel_regeneration() {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x5E, &[8]);
        let data = store(&[(Ship::TYPE, 128, bytes)]);
        assert_eq!(data.ship_fields(ShipId(128)).map(|f| f.fuel_regen), Ok(8));
    }

    #[test]
    fn a_ships_fields_include_its_shield_and_armour_recharge() {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x10, &[125]);
        put_i16s(&mut bytes, 0x36, &[-20]);
        let data = store(&[(Ship::TYPE, 128, bytes)]);
        let fields = data.ship_fields(ShipId(128)).expect("decodes");
        assert_eq!((fields.shield_rech, fields.armor_rech), (125, -20));
    }

    #[test]
    fn a_ships_fields_include_its_flags2() {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x62, &[0x4021]);
        let data = store(&[(Ship::TYPE, 128, bytes)]);
        assert_eq!(data.ship_fields(ShipId(128)).map(|f| f.flags2), Ok(0x4021));
    }

    /// A `shïp` carrying `items` in its `DefaultItems` 1-4 and `more` in
    /// 5-8, each an `oütf` ID and a count; every other slot -1.
    fn outfitted(items: &[(i16, i16)], more: &[(i16, i16)]) -> Vec<u8> {
        let mut bytes = ship(1, 2, 3);
        for (ids_at, counts_at, slots) in [(0x4E, 0x56, items), (0x370, 0x378, more)] {
            put_i16s(&mut bytes, ids_at, &[-1; 4]);
            for (i, &(id, count)) in slots.iter().enumerate() {
                put_i16s(&mut bytes, ids_at + 2 * i, &[id]);
                put_i16s(&mut bytes, counts_at + 2 * i, &[count]);
            }
        }
        bytes
    }

    #[test]
    fn a_ships_fields_include_its_mass_free_mass_and_contribute() {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x0C, &[8]);
        put_i16s(&mut bytes, 0x3E, &[15]);
        bytes[0x64..0x6C].copy_from_slice(&0x0000_0008_0000_0001_u64.to_be_bytes());
        let data = store(&[(Ship::TYPE, 128, bytes)]);
        let fields = data.ship_fields(ShipId(128)).expect("decodes");
        assert_eq!(
            (fields.mass, fields.free_mass, fields.contribute),
            (15, 8, 0x0000_0008_0000_0001)
        );
    }

    #[test]
    fn a_ships_default_outfits_are_each_item_with_its_count_in_slot_order() {
        let data = store(&[(
            Ship::TYPE,
            128,
            outfitted(&[(200, 2), (999, 1), (202, 1)], &[(201, 3), (200, -4)]),
        )]);
        assert_eq!(
            data.default_outfits(ShipId(128)),
            [
                (OutfitId(200), 2),
                (OutfitId(999), 1),
                (OutfitId(202), 1),
                (OutfitId(201), 3),
                (OutfitId(200), 0),
            ],
            "repeats kept, a negative count carries none, and the oütf need not exist"
        );
    }

    #[test]
    fn a_missing_undecodable_or_unoutfitted_ship_has_no_default_outfits() {
        let data = store(&[
            (Ship::TYPE, 128, outfitted(&[], &[])),
            (Ship::TYPE, 129, short(outfitted(&[(200, 1)], &[]))),
        ]);
        assert_eq!(data.default_outfits(ShipId(128)), []);
        assert_eq!(data.default_outfits(ShipId(129)), []);
        assert_eq!(data.default_outfits(ShipId(130)), []);
    }

    /// A `shïp` sold in the shipyard: every field the shipyard reads set to
    /// something of its own, with two default items.
    fn for_sale() -> Vec<u8> {
        let mut bytes = outfitted(&[(200, 2), (201, -1)], &[(202, 1)]);
        put_i16s(&mut bytes, 0x00, &[-15]);
        put_i16s(&mut bytes, 0x0C, &[12]);
        put_i16s(&mut bytes, 0x2A, &[4, 2, 6]);
        bytes[0x30..0x34].copy_from_slice(&17_500_i32.to_be_bytes());
        put_i16s(&mut bytes, 0x3C, &[25, 30, 41, 2, 3]);
        bytes[0x64..0x6C].copy_from_slice(&0x10_u64.to_be_bytes());
        bytes[0x6C..0x70].copy_from_slice(b"b422");
        bytes[0x16B..0x16F].copy_from_slice(b"!b17");
        bytes[0x380..0x388].copy_from_slice(&0x0000_0002_0000_0001_u64.to_be_bytes());
        put_i16s(&mut bytes, 0x388, &[45, 40]);
        bytes[0x5CE..0x5DD].copy_from_slice(b"Heavy\\nShuttle!");
        bytes[0x62E..0x63D].copy_from_slice(b"A Heavy Shuttle");
        bytes[0x726..0x728].copy_from_slice(&0x4100_u16.to_be_bytes());
        put_i16s(&mut bytes, 0x48, &[1129]);
        bytes[0x60E..0x61D].copy_from_slice(b"heavy shuttle\0\0");
        put_i16s(&mut bytes, 0x732, &[2]);
        bytes[0x26A..0x26E].copy_from_slice(b"b300");
        bytes[0x3D0..0x3D5].copy_from_slice(b"^b301");
        bytes[0x4CF..0x4D3].copy_from_slice(b"!b30");
        bytes
    }

    #[test]
    fn each_readable_shïp_is_a_ship_record_by_id() {
        let data = store_named(&[
            (Ship::TYPE, 130, None, for_sale()),
            (Ship::TYPE, 129, Some("Heavy Shuttle"), for_sale()),
            (Ship::TYPE, 131, Some("Short"), short(for_sale())),
        ]);
        let record = |id: i16, name: &str| ShipRecord {
            id: ShipId(id),
            name: name.to_owned(),
            short_name: "Heavy\\nShuttle!".to_owned(),
            long_name: "A Heavy Shuttle".to_owned(),
            fields: ShipFields {
                holds: -15,
                accel: 1,
                speed: 2,
                maneuver: 3,
                free_mass: 12,
                mass: 30,
                contribute: 0x10,
                ..ShipFields::default()
            },
            defaults: vec![(OutfitId(200), 2), (OutfitId(201), 0), (OutfitId(202), 1)],
            cost: 17_500,
            tech_level: 6,
            buy_random: 45,
            hire_random: 40,
            require: 0x0000_0002_0000_0001,
            availability: Test::parse("b422"),
            appear_on: Test::parse("!b17"),
            flags3: 0x4100,
            disp_weight: 25,
            max_gun: 4,
            max_tur: 2,
            length: 41,
            crew: 3,
            inherent_ai: 2,
            comm_name: "heavy shuttle".to_owned(),
            inherent_govt: Some(GovtId(129)),
            combat_govt: None,
            escort_type: 2,
            on_capture: Script::parse("^b301"),
            on_purchase: Script::parse("b300"),
            on_retire: Script::parse("!b30"),
        };
        assert_eq!(
            data.ships(),
            [record(129, "Heavy Shuttle"), record(130, "Heavy Shuttle!"),],
            "a resource without a name goes by its ShortName on one line; an undecodable one \
             is skipped"
        );
        assert_eq!(
            data.ships()[0].fields,
            data.ship_fields(ShipId(129)).expect("decodes"),
            "the same fields a session flies with"
        );
        assert_eq!(
            data.ships()[0].defaults,
            data.default_outfits(ShipId(129)),
            "the same default items"
        );
        assert_eq!(store(&[]).ships(), []);
        for (a, b) in data.ships().iter().zip(&data.ships()) {
            assert!(
                shared(&a.availability, &b.availability),
                "parsed once: {:?}",
                a.id
            );
            assert!(
                shared(&a.appear_on, &b.appear_on),
                "AppearOn parsed once: {:?}",
                a.id
            );
            for (a, b) in [
                (&a.on_purchase, &b.on_purchase),
                (&a.on_capture, &b.on_capture),
                (&a.on_retire, &b.on_retire),
            ] {
                assert!(shared_script(a, b), "each hook parsed once: {a:?}");
            }
        }
    }

    #[test]
    fn a_shïps_inherent_government_is_its_gövt_attributes_or_none() {
        let inheriting = |raw: i16| {
            let mut bytes = for_sale();
            put_i16s(&mut bytes, 0x48, &[raw]);
            bytes
        };
        for (raw, govt) in [
            (-1, None),
            (128, Some(128)),
            (383, Some(383)),
            (1128, Some(128)),
            (1999, Some(999)),
            (2130, None),
            (0, Some(0)),
        ] {
            let data = store(&[(Ship::TYPE, 128, inheriting(raw))]);
            assert_eq!(
                data.ships()[0].inherent_govt,
                govt.map(GovtId),
                "InherentGovt {raw}"
            );
        }
    }

    #[test]
    fn a_shïps_combat_government_is_its_gövt_its_combat_only_one_or_none() {
        let inheriting = |raw: i16| {
            let mut bytes = for_sale();
            put_i16s(&mut bytes, 0x48, &[raw]);
            bytes
        };
        for (raw, govt) in [
            (-1, None),
            (128, Some(128)),
            (999, Some(999)),
            (1128, None),
            (2130, Some(130)),
            (2999, Some(999)),
            (3000, None),
            (0, Some(0)),
        ] {
            let data = store(&[(Ship::TYPE, 128, inheriting(raw))]);
            assert_eq!(
                data.ships()[0].combat_govt,
                govt.map(GovtId),
                "InherentGovt {raw}"
            );
        }
    }

    #[test]
    fn a_shïps_name_stops_at_its_designer_note() {
        let data = store_named(&[
            (
                Ship::TYPE,
                128,
                Some("Shuttle;Second-Hand - poor"),
                for_sale(),
            ),
            (Ship::TYPE, 129, Some("Lightning; Wild Geese"), for_sale()),
            (Ship::TYPE, 130, Some(";"), for_sale()),
        ]);
        let names: Vec<_> = data.ships().into_iter().map(|ship| ship.name).collect();
        assert_eq!(names, ["Shuttle", "Lightning", ""]);
    }

    /// An `oütf` with these four `ModType` and `ModVal` pairs, every other
    /// field set to something of its own.
    fn outfit(mods: [(i16, i16); 4]) -> Vec<u8> {
        let mut bytes = vec![0; Outfit::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[50, -5, 3]);
        put_i16s(&mut bytes, 0x0A, &[8]);
        bytes[0x0C..0x0E].copy_from_slice(&0x4100_u16.to_be_bytes());
        bytes[0x0E..0x12].copy_from_slice(&150_000_i32.to_be_bytes());
        for (at, (mod_type, mod_val)) in [0x06, 0x12, 0x16, 0x1A].into_iter().zip(mods) {
            put_i16s(&mut bytes, at, &[mod_type, mod_val]);
        }
        bytes[0x1E..0x26].copy_from_slice(&0x10_u64.to_be_bytes());
        bytes[0x26..0x2E].copy_from_slice(&0x0000_0008_0000_0001_u64.to_be_bytes());
        bytes[0x2E..0x31].copy_from_slice(b"b12");
        bytes[0x32B..0x334].copy_from_slice(b"Big\\nGun!");
        bytes[0x36B..0x372].copy_from_slice(b"big gun");
        put_i16s(&mut bytes, 0x3F0, &[75, 1128]);
        bytes[0x12D..0x131].copy_from_slice(b"b400");
        bytes[0x22C..0x231].copy_from_slice(b"!b400");
        bytes
    }

    #[test]
    fn each_readable_oütf_is_an_outfit_record_by_id() {
        let data = store_named(&[
            (
                Outfit::TYPE,
                201,
                None,
                outfit([(2, 5), (9, -2), (0, 0), (18, 4)]),
            ),
            (
                Outfit::TYPE,
                200,
                Some("Big Gun"),
                outfit([(1, 128), (0, 0), (0, 0), (0, 0)]),
            ),
            (Outfit::TYPE, 202, Some("Short"), short(outfit([(0, 0); 4]))),
        ]);
        let record = |id: i16, name: &str, mods| OutfitRecord {
            id: OutfitId(id),
            name: name.to_owned(),
            short_name: "Big\\nGun!".to_owned(),
            disp_weight: 50,
            mass: -5,
            tech_level: 3,
            max: 8,
            flags: 0x4100,
            cost: 150_000,
            mods,
            contribute: 0x10,
            require: 0x0000_0008_0000_0001,
            require_govt: 1128,
            buy_random: 75,
            availability: Test::parse("b12"),
            item_class: 0,
            lc_name: "big gun".to_owned(),
            lc_plural: String::new(),
            on_purchase: Script::parse("b400"),
            on_sell: Script::parse("!b400"),
        };
        assert_eq!(
            data.outfits(),
            [
                record(200, "Big Gun", [(1, 128), (0, 0), (0, 0), (0, 0)]),
                record(201, "big gun", [(2, 5), (9, -2), (0, 0), (18, 4)]),
            ],
            "a resource without a name goes by its LCName; an undecodable one is skipped"
        );
        assert_eq!(store(&[]).outfits(), []);
        for (a, b) in data.outfits().iter().zip(&data.outfits()) {
            assert!(
                shared(&a.availability, &b.availability),
                "parsed once: {:?}",
                a.id
            );
            assert!(shared_script(&a.on_purchase, &b.on_purchase), "{:?}", a.id);
            assert!(shared_script(&a.on_sell, &b.on_sell), "{:?}", a.id);
        }
    }

    #[test]
    fn an_oütfs_low_weapon_and_government_mod_vals_load_as_ids_in_every_slot() {
        let data = store(&[(
            Outfit::TYPE,
            128,
            outfit([(1, 5), (3, 127), (21, 0), (16, 5)]),
        )]);
        assert_eq!(
            data.outfits()[0].mods,
            [(1, 133), (3, 255), (21, 128), (16, 5)]
        );
    }

    #[test]
    fn an_oütfs_item_class_and_lower_case_names_are_read_raw() {
        let mut bytes = outfit([(0, 0); 4]);
        bytes[0x36B..0x36B + 11].copy_from_slice(b"spare part\0");
        bytes[0x3AB..0x3AB + 12].copy_from_slice(b"spare parts\0");
        put_i16s(&mut bytes, 0x3EC, &[7]);
        let data = store(&[(Outfit::TYPE, 128, bytes)]);
        let record = &data.outfits()[0];
        assert_eq!(record.item_class, 7);
        assert_eq!(record.lc_name, "spare part");
        assert_eq!(record.lc_plural, "spare parts");
    }

    #[test]
    fn a_missing_or_undecodable_ship_says_why() {
        let data = store(&[(Ship::TYPE, 129, short(ship(1, 2, 3)))]);
        assert_eq!(data.ship_fields(ShipId(140)), Err("no shïp 140".to_owned()));
        let Err(message) = data.ship_fields(ShipId(129)) else {
            panic!("an error")
        };
        assert!(message.contains("129"), "{message}");
    }

    #[test]
    fn a_system_exists_when_it_is_there_and_decodes() {
        let data = store(&[
            (System::TYPE, 130, system()),
            (System::TYPE, 131, short(system())),
        ]);
        assert!(data.system_exists(SystemId(130)));
        assert!(!data.system_exists(SystemId(131)), "undecodable");
        assert!(!data.system_exists(SystemId(999)), "missing");
    }

    /// A `sÿst` whose `nav_def` holds `stellars`, every other slot -1.
    fn system_with(stellars: &[i16]) -> Vec<u8> {
        let mut bytes = system();
        put_i16s(&mut bytes, 0x24, &[-1; 16]);
        put_i16s(&mut bytes, 0x24, stellars);
        bytes
    }

    /// A `spöb` at (`x`, `y`) of graphic type `graphic_type`, with these
    /// `Flags` and `MinStatus`, independent and of no tech level.
    fn stellar(x: i16, y: i16, graphic_type: i16, flags: u32, min_status: i16) -> Vec<u8> {
        let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[x, y, graphic_type]);
        bytes[0x06..0x0A].copy_from_slice(&flags.to_be_bytes());
        put_i16s(&mut bytes, 0x14, &[-1, min_status]);
        bytes
    }

    /// A `spïn` naming `rlëD` `image`, one frame across.
    fn spin(image: i16) -> Vec<u8> {
        let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[image, -1, 0, 0, 1, 1]);
        bytes
    }

    /// One `width` x `height` frame.
    fn sheet(width: u16, height: u16) -> Vec<u8> {
        RledBuilder::new(width, height)
            .frame(|f| (0..height).fold(f, |f, _| f.line().pixels(&vec![0x7C00; width.into()])))
            .build()
    }

    /// `bytes`, a `spöb`, with this `Flags2`.
    fn flagged2(mut bytes: Vec<u8>, flags2: u16) -> Vec<u8> {
        bytes[0x20..0x22].copy_from_slice(&flags2.to_be_bytes());
        bytes
    }

    /// A wormhole (`Flags2` 0x2000) at (`x`, `y`) heading out on
    /// `angle`, with these `HyperLink`s, every other slot -1.
    fn gate(x: i16, y: i16, angle: i16, links: &[i16]) -> Vec<u8> {
        let mut bytes = flagged2(stellar(x, y, 0, 0x21, 0), 0x2000);
        put_i16s(&mut bytes, 0x1A, &[angle]);
        put_i16s(&mut bytes, 0x26, &[-1; 8]);
        put_i16s(&mut bytes, 0x26, links);
        bytes
    }

    #[test]
    fn the_gate_sites_are_every_listed_stellar_by_id_in_the_lowest_system_listing_it() {
        let data = store(&[
            (System::TYPE, 131, system_with(&[129, 128])),
            (System::TYPE, 130, system_with(&[129, 999])),
            (System::TYPE, 132, short(system_with(&[133]))),
            (Stellar::TYPE, 128, stellar(5, 6, 0, 1, 0)),
            (Stellar::TYPE, 129, gate(-70, 250, 120, &[128, 0, 999])),
            (Stellar::TYPE, 130, gate(0, 0, 0, &[])),
            (Stellar::TYPE, 133, gate(0, 0, 0, &[])),
        ]);
        let links = |ids: &[i16]| {
            let mut slots = [None; 8];
            for (slot, &id) in slots.iter_mut().zip(ids) {
                *slot = Some(StellarId(id));
            }
            slots
        };
        assert_eq!(
            data.gate_sites(),
            [
                GateSite {
                    id: StellarId(128),
                    system: SystemId(131),
                    position: Vec2::new(5.0, 6.0),
                    flags2: 0,
                    // Its zeroed links, kept raw.
                    links: [Some(StellarId(0)); 8],
                    exit_angle: 0,
                },
                GateSite {
                    id: StellarId(129),
                    system: SystemId(130),
                    position: Vec2::new(-70.0, 250.0),
                    flags2: 0x2000,
                    links: links(&[128, 0, 999]),
                    exit_angle: 120,
                },
            ],
            "unlisted 130, missing 999, and 133 listed only by an undecodable system left out"
        );
        assert_eq!(store(&[]).gate_sites(), []);
    }

    #[test]
    fn each_readable_systems_visibility_is_its_test_parsed_a_blank_one_always() {
        let mut gated = system();
        gated[0x96..0x96 + 9].copy_from_slice(b"b5 & !b6\0");
        let data = store(&[
            (System::TYPE, 131, gated),
            (System::TYPE, 130, system()),
            (System::TYPE, 132, short(system())),
        ]);
        let tests = data.system_visibility();
        assert_eq!(
            tests,
            [
                SystemVisibility {
                    id: SystemId(130),
                    visibility: Test::default(),
                },
                SystemVisibility {
                    id: SystemId(131),
                    visibility: Test::parse("b5 & !b6"),
                },
            ],
            "by ID, the undecodable 132 left out"
        );
        assert_eq!(tests[0].visibility.tree(), Some(&TestExpr::Always));
        assert_eq!(store(&[]).system_visibility(), []);
    }

    #[test]
    fn the_listed_stellars_are_every_readable_one_some_system_lists_with_every_system_listing_it() {
        let mut governed = stellar(5, 6, 0, 0x21, 0);
        put_i16s(&mut governed, 0x14, &[140]);
        let data = store(&[
            (System::TYPE, 131, system_with(&[129, 128, 129])),
            (System::TYPE, 130, system_with(&[129, 999])),
            (System::TYPE, 132, short(system_with(&[133]))),
            (Stellar::TYPE, 128, governed),
            (Stellar::TYPE, 129, gate(-70, 250, 120, &[])),
            (Stellar::TYPE, 130, stellar(0, 0, 0, 1, 0)),
            (Stellar::TYPE, 133, stellar(0, 0, 0, 1, 0)),
        ]);
        assert_eq!(
            data.listed_stellars(),
            [
                ListedStellar {
                    id: StellarId(128),
                    systems: vec![SystemId(131)],
                    flags: 0x21,
                    flags2: 0,
                    govt: Some(GovtId(140)),
                },
                ListedStellar {
                    id: StellarId(129),
                    systems: vec![SystemId(130), SystemId(131)],
                    flags: 0x21,
                    flags2: 0x2000,
                    govt: None,
                },
            ],
            "by ID, each system once by ID; unlisted 130, missing 999, and 133 listed only \
             by an undecodable system left out"
        );
        assert_eq!(store(&[]).listed_stellars(), []);
    }

    #[test]
    fn the_place_names_are_every_readable_stellars_and_systems_as_shown() {
        let data = store_named(&[
            (Stellar::TYPE, 128, Some("Earth"), stellar(0, 0, 0, 1, 0)),
            (
                Stellar::TYPE,
                129,
                Some("Luna;moon"),
                stellar(0, 0, 0, 1, 0),
            ),
            (Stellar::TYPE, 130, None, stellar(0, 0, 0, 1, 0)),
            (
                Stellar::TYPE,
                131,
                Some("Broken"),
                short(stellar(0, 0, 0, 1, 0)),
            ),
            (System::TYPE, 130, Some("Sol"), system()),
            (System::TYPE, 131, Some("Gone"), short(system())),
        ]);
        let names = data.place_names();
        assert_eq!(
            names.stellars,
            BTreeMap::from([
                (StellarId(128), "Earth".to_owned()),
                (StellarId(129), "Luna".to_owned()),
                (StellarId(130), String::new()),
            ]),
            "up to any ';'; unnamed, empty; undecodable 131 left out"
        );
        assert_eq!(
            names.systems,
            BTreeMap::from([(SystemId(130), "Sol".to_owned())]),
            "undecodable 131 left out"
        );
        assert_eq!(store(&[]).place_names(), PlaceNames::default());
    }

    #[test]
    fn stellar_marks_read_each_spobs_place_and_name_up_to_its_last_semicolon() {
        let data = store_named(&[
            (
                Stellar::TYPE,
                128,
                Some("Tre'ar Zalom; variant ; "),
                stellar(-300, 450, 0, 1, 0),
            ),
            (Stellar::TYPE, 129, None, stellar(7, -9, 0, 1, 0)),
            (
                Stellar::TYPE,
                130,
                Some("Broken"),
                short(stellar(0, 0, 0, 1, 0)),
            ),
        ]);
        assert_eq!(
            data.stellar_marks(),
            [
                StellarMark {
                    id: StellarId(128),
                    place: (-300, 450),
                    name: "Tre'ar Zalom; variant".to_owned(),
                },
                StellarMark {
                    id: StellarId(129),
                    place: (7, -9),
                    name: String::new(),
                },
            ],
            "up to the last ';', trailing spaces and ';'s dropped; unnamed, empty; \
             undecodable 130 left out"
        );
    }

    #[test]
    fn a_systems_landing_sites_are_its_readable_stellars_in_nav_order() {
        let data = store(&[
            (System::TYPE, 130, system_with(&[129, 999, 128, 131])),
            (Stellar::TYPE, 128, stellar(-300, 450, 7, 0x0000_0013, 25)),
            (
                Stellar::TYPE,
                129,
                flagged2(stellar(10, -20, 0, 0x2001, -32767), 0x1200),
            ),
            (Stellar::TYPE, 131, short(stellar(0, 0, 0, 1, 0))),
            (Spin::TYPE, 1000, spin(1000)),
            (RLED, 1000, sheet(12, 30)),
        ]);
        assert_eq!(
            data.landing_sites(SystemId(130)),
            [
                LandingSite {
                    id: StellarId(129),
                    position: Vec2::new(10.0, -20.0),
                    frame_size: Some((12, 30)),
                    flags: 0x2001,
                    min_status: -32767,
                    landing_sound: None,
                    tech_level: 0,
                    special_tech: [0; 8],
                    govt: None,
                    flags2: 0x1200,
                    destroyed: false,
                    graphic: 0,
                    dead_graphic: Some(0),
                    dead_frame_size: Some((12, 30)),
                    type_counts_frames: true,
                    exit_angle: 0,
                },
                LandingSite {
                    id: StellarId(128),
                    position: Vec2::new(-300.0, 450.0),
                    frame_size: None,
                    flags: 0x13,
                    min_status: 25,
                    landing_sound: None,
                    tech_level: 0,
                    special_tech: [0; 8],
                    govt: None,
                    flags2: 0,
                    destroyed: false,
                    graphic: 7,
                    dead_graphic: Some(0),
                    dead_frame_size: Some((12, 30)),
                    type_counts_frames: false,
                    exit_angle: 0,
                },
            ]
        );
    }

    /// `bytes`, a `spöb`, with this `DeadType`.
    fn dead_as(mut bytes: Vec<u8>, dead_type: i16) -> Vec<u8> {
        put_i16s(&mut bytes, 0x240, &[dead_type]);
        bytes
    }

    #[test]
    fn a_landing_sites_dead_graphic_is_its_dead_type_when_that_sheet_reads() {
        let data = store(&[
            (System::TYPE, 130, system_with(&[128, 129, 130, 131, 132])),
            (Stellar::TYPE, 128, dead_as(stellar(0, 0, 4, 1, 0), 3)),
            (Stellar::TYPE, 129, dead_as(stellar(0, 0, 4, 1, 0), -1)),
            (Stellar::TYPE, 130, dead_as(stellar(0, 0, 4, 1, 0), 5)),
            (Stellar::TYPE, 131, dead_as(stellar(0, 0, 4, 1, 0), 300)),
            (Stellar::TYPE, 132, dead_as(stellar(0, 0, -2, 1, 0), 6)),
            (Spin::TYPE, 1003, spin(1003)),
            (RLED, 1003, sheet(12, 30)),
            // 1005 names a sheet that is not there.
            (Spin::TYPE, 1005, spin(1005)),
            (Spin::TYPE, 1006, spin(1006)),
            (RLED, 1006, sheet(4, 4)),
        ]);
        assert_eq!(
            data.landing_sites(SystemId(130))
                .iter()
                .map(|site| (site.id.0, site.graphic, site.dead_graphic))
                .collect::<Vec<_>>(),
            [
                (128, 4, Some(3)),
                (129, 4, None),
                (130, 4, None),
                (131, 4, None),
                (132, -2, Some(6)),
            ]
        );
    }

    #[test]
    fn a_landing_sites_dead_frame_size_is_its_dead_type_sheets_frame() {
        let data = store(&[
            (System::TYPE, 130, system_with(&[128, 129, 130, 131])),
            (Stellar::TYPE, 128, dead_as(stellar(0, 0, 4, 1, 0), 3)),
            (Stellar::TYPE, 129, dead_as(stellar(0, 0, 4, 1, 0), -1)),
            (Stellar::TYPE, 130, dead_as(stellar(0, 0, 4, 1, 0), 5)),
            (Stellar::TYPE, 131, dead_as(stellar(0, 0, 3, 1, 0), 6)),
            (Spin::TYPE, 1003, spin(1003)),
            (RLED, 1003, sheet(12, 30)),
            // 1005 names a sheet that is not there.
            (Spin::TYPE, 1005, spin(1005)),
            (Spin::TYPE, 1006, spin(1006)),
            (RLED, 1006, sheet(50, 20)),
        ]);
        assert_eq!(
            data.landing_sites(SystemId(130))
                .iter()
                .map(|site| (site.id.0, site.frame_size, site.dead_frame_size))
                .collect::<Vec<_>>(),
            [
                (128, None, Some((12, 30))),
                (129, None, None),
                (130, None, None),
                (131, Some((12, 30)), Some((50, 20))),
            ]
        );
    }

    /// A `spïn` naming `rlëD` 1000, `x_tiles` across and `y_tiles` down.
    fn tiled(x_tiles: i16, y_tiles: i16) -> Vec<u8> {
        let mut bytes = spin(1000);
        put_i16s(&mut bytes, 0x08, &[x_tiles, y_tiles]);
        bytes
    }

    #[test]
    fn a_landing_site_counts_frames_when_its_type_spins_tiles_multiply_above_0() {
        for (x_tiles, y_tiles, counts) in [
            (1, 1, true),
            (6, 6, true),
            (-2, -3, true),
            (0, 5, false),
            (5, 0, false),
            (-1, 4, false),
            (4, -1, false),
            // 32 bits, as the original multiplies: no word wrap.
            (300, 300, true),
            (300, -300, false),
        ] {
            let data = store(&[
                (System::TYPE, 130, system_with(&[128])),
                (Stellar::TYPE, 128, stellar(0, 0, 0, 1, 0)),
                (Spin::TYPE, 1000, tiled(x_tiles, y_tiles)),
                (RLED, 1000, sheet(12, 30)),
            ]);
            assert_eq!(
                data.landing_sites(SystemId(130))[0].type_counts_frames,
                counts,
                "{x_tiles} x {y_tiles}"
            );
        }
        let no_spin = store(&[
            (System::TYPE, 130, system_with(&[128])),
            (Stellar::TYPE, 128, stellar(0, 0, 0, 1, 0)),
        ]);
        assert!(!no_spin.landing_sites(SystemId(130))[0].type_counts_frames);
    }

    #[test]
    fn the_fallback_frame_size_is_spin_1000s_frame_when_it_reads() {
        assert_eq!(
            store(&[(Spin::TYPE, 1000, spin(1000)), (RLED, 1000, sheet(30, 12))])
                .fallback_frame_size(),
            Some((30, 12))
        );
        assert_eq!(
            store(&[(Spin::TYPE, 1000, spin(1000))]).fallback_frame_size(),
            None
        );
        assert_eq!(
            store(&[(RLED, 1000, sheet(4, 4))]).fallback_frame_size(),
            None
        );
        assert_eq!(
            store(&[(Spin::TYPE, 1001, spin(1000)), (RLED, 1000, sheet(4, 4))])
                .fallback_frame_size(),
            None
        );
    }

    #[test]
    fn a_borrowed_or_shared_game_data_forwards_the_fallback_frame_size() {
        for (resources, size) in [
            (
                vec![(Spin::TYPE, 1000, spin(1000)), (RLED, 1000, sheet(30, 12))],
                Some((30, 12)),
            ),
            (vec![(Spin::TYPE, 1000, spin(1000))], None),
        ] {
            let data = std::rc::Rc::new(store(&resources));
            assert_eq!(PilotCatalog::fallback_frame_size(&data), size, "Rc");
            assert_eq!(PilotCatalog::fallback_frame_size(&&*data), size, "&");
        }
    }

    #[test]
    fn a_landing_sites_tech_levels_and_government_are_its_spöbs() {
        let mut port = stellar(0, 0, 0, 0x05, 0);
        put_i16s(&mut port, 0x0C, &[4, 6, 55, 57, 128]);
        put_i16s(&mut port, 0x444, &[58, 81, 0, -1, 9]);
        let data = store(&[
            (System::TYPE, 130, system_with(&[128, 129])),
            (Stellar::TYPE, 128, port),
            (Stellar::TYPE, 129, stellar(5, 5, 0, 1, 0)),
        ]);
        let sites = data.landing_sites(SystemId(130));
        assert_eq!(
            sites
                .iter()
                .map(|site| (site.tech_level, site.special_tech, site.govt))
                .collect::<Vec<_>>(),
            [
                (4, [6, 55, 57, 58, 81, 0, -1, 9], Some(GovtId(128))),
                (0, [0; 8], None),
            ]
        );
    }

    /// An independent (`Govt` -1) `sÿst` at map (`x`, `y`) with these
    /// hyperlinks, every other slot -1.
    fn linked(x: i16, y: i16, links: &[i16]) -> Vec<u8> {
        let mut bytes = governed(system(), -1);
        put_i16s(&mut bytes, 0x00, &[x, y]);
        put_i16s(&mut bytes, 0x04, &[-1; 16]);
        put_i16s(&mut bytes, 0x04, links);
        bytes
    }

    /// `bytes`, a `sÿst`, owned by `gövt` `govt`.
    fn governed(mut bytes: Vec<u8>, govt: i16) -> Vec<u8> {
        put_i16s(&mut bytes, 0x66, &[govt]);
        bytes
    }

    #[test]
    fn the_star_map_is_every_readable_system_by_id_with_its_position_links_govt_and_interference() {
        let data = store(&[
            (
                System::TYPE,
                131,
                interfering(governed(linked(600, -75, &[130, 999, 131]), 140), 45),
            ),
            (System::TYPE, 129, short(linked(0, 0, &[130]))),
            (System::TYPE, 130, linked(-20, 40, &[])),
        ]);
        assert_eq!(
            data.star_map(),
            [
                StarSystem {
                    id: SystemId(130),
                    position: Vec2::new(-20.0, 40.0),
                    links: Vec::new(),
                    govt: None,
                    stellars: vec![None; 16],
                    interference: 0,
                },
                StarSystem {
                    id: SystemId(131),
                    position: Vec2::new(600.0, -75.0),
                    links: vec![SystemId(130), SystemId(999), SystemId(131)],
                    govt: Some(GovtId(140)),
                    stellars: vec![None; 16],
                    interference: 45,
                },
            ]
        );
        assert_eq!(store(&[]).star_map(), []);
    }

    /// `bytes`, a `sÿst`, with this `Interference` (offset 0x6C).
    fn interfering(mut bytes: Vec<u8>, interference: i16) -> Vec<u8> {
        put_i16s(&mut bytes, 0x6C, &[interference]);
        bytes
    }

    /// A landable `spöb` at the centre with this `CustSndID`.
    fn sounding(cust_snd_id: i16) -> Vec<u8> {
        let mut bytes = stellar(0, 0, 0, 1, 0);
        put_i16s(&mut bytes, 0x1A, &[cust_snd_id]);
        bytes
    }

    #[test]
    fn a_landing_sites_sound_is_its_custom_sound_from_10000_up() {
        // Port Kane's 10032; none (-1), 0, the 120 a hypergate keeps there
        // as an angle, and 9999, are no landing sound.
        let sounds = [10_032, -1, 0, 120, 9_999, 10_000];
        let mut resources = vec![(
            System::TYPE,
            130,
            system_with(&[128, 129, 130, 131, 132, 133]),
        )];
        for (id, sound) in (128..).zip(sounds) {
            resources.push((Stellar::TYPE, id, sounding(sound)));
        }
        let data = store(&resources);
        let found: Vec<_> = data
            .landing_sites(SystemId(130))
            .iter()
            .map(|site| site.landing_sound)
            .collect();
        assert_eq!(
            found,
            [
                Some(SoundId(10_032)),
                None,
                None,
                None,
                None,
                Some(SoundId(10_000)),
            ]
        );
    }

    #[test]
    fn a_landing_sites_exit_angle_is_its_custom_sound_raw() {
        // A gate keeps its exit heading in `CustSndID`; the value goes
        // through raw, whatever it is, for the gate rules to read.
        let angles = [120, -1, 0, 359, 360, 10_032];
        let mut resources = vec![(
            System::TYPE,
            130,
            system_with(&[128, 129, 130, 131, 132, 133]),
        )];
        for (id, angle) in (128..).zip(angles) {
            resources.push((Stellar::TYPE, id, sounding(angle)));
        }
        let data = store(&resources);
        let found: Vec<i16> = data
            .landing_sites(SystemId(130))
            .iter()
            .map(|site| site.exit_angle)
            .collect();
        assert_eq!(found, angles);
    }

    #[test]
    fn a_systems_stellars_on_the_star_map_are_each_nav_slots_flags_raw() {
        let mut bytes = system_with(&[129, 999, 128, 131]);
        put_i16s(&mut bytes, 0x04, &[-1; 16]);
        let data = store(&[
            (System::TYPE, 130, bytes),
            (Stellar::TYPE, 128, stellar(0, 0, 0, 0x0000_0021, 0)),
            (
                Stellar::TYPE,
                129,
                flagged2(stellar(0, 0, 0, 0x8001, 0), 0x3000),
            ),
            (Stellar::TYPE, 131, short(stellar(0, 0, 0, 1, 0))),
        ]);
        let map = data.star_map();
        let mut expected = vec![Some((0x8001, 0x3000)), None, Some((0x21, 0)), None];
        expected.resize(16, None);
        assert_eq!(map.len(), 1);
        assert_eq!(map[0].stellars, expected, "missing and unreadable are None");
    }

    #[test]
    fn a_missing_or_undecodable_system_has_no_landing_sites() {
        let data = store(&[
            (System::TYPE, 131, short(system_with(&[128]))),
            (Stellar::TYPE, 128, stellar(0, 0, 0, 1, 0)),
        ]);
        assert_eq!(data.landing_sites(SystemId(131)), []);
        assert_eq!(data.landing_sites(SystemId(130)), []);
    }

    /// A `STR#` of `strings`.
    fn str_list(strings: &[&str]) -> Vec<u8> {
        let mut bytes = u16::try_from(strings.len())
            .expect("few")
            .to_be_bytes()
            .to_vec();
        for string in strings {
            bytes.push(u8::try_from(string.len()).expect("short"));
            bytes.extend(string.as_bytes());
        }
        bytes
    }

    #[test]
    fn the_commodity_strings_are_str_4000_and_4004_whole() {
        let data = store(&[
            (
                StrList::TYPE,
                4000,
                str_list(&["Food", "Industrial", "*Cargo"]),
            ),
            (StrList::TYPE, 4004, str_list(&["75", "lots"])),
            (StrList::TYPE, 4001, str_list(&["other"])),
        ]);
        assert_eq!(
            data.commodity_strings(),
            CommodityStrings {
                names: vec!["Food".into(), "Industrial".into(), "*Cargo".into()],
                name_patches: Default::default(),
                base_prices: vec!["75".into(), "lots".into()],
                price_patches: Default::default(),
            }
        );
    }

    #[test]
    fn missing_or_undecodable_commodity_strings_are_none() {
        assert_eq!(store(&[]).commodity_strings(), CommodityStrings::default());
        let data = store(&[
            (StrList::TYPE, 4000, short(str_list(&["Food"]))),
            (StrList::TYPE, 4004, str_list(&["75"])),
        ]);
        assert_eq!(
            data.commodity_strings(),
            CommodityStrings {
                names: Vec::new(),
                name_patches: Default::default(),
                base_prices: vec!["75".into()],
                price_patches: Default::default(),
            }
        );
    }

    /// A data file, `/data/Nova Data`, and a plug-in,
    /// `/plugins/Price Patch.npif`, each holding a fork.
    struct Layered {
        data: Vec<u8>,
        plugin: Vec<u8>,
    }

    impl DirLister for Layered {
        fn list(&self, dir: &Path) -> io::Result<Vec<Listing>> {
            let name = if dir == Path::new("/data") {
                "Nova Data"
            } else {
                "Price Patch.npif"
            };
            Ok(vec![Listing {
                name: name.into(),
                kind: EntryKind::File,
            }])
        }
    }

    impl ForkReader for Layered {
        fn read_fork(&self, path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            let bytes = if path.starts_with("/data") {
                &self.data
            } else {
                &self.plugin
            };
            Ok((fork == Fork::Data).then(|| bytes.clone()))
        }
    }

    fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> Vec<u8> {
        resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
                fork.resource(*ty, *id, None, data)
            })
            .build()
            .bytes
    }

    /// The store of a data file and a plug-in holding these resources.
    fn layered(data: &[(ResType, i16, Vec<u8>)], plugin: &[(ResType, i16, Vec<u8>)]) -> GameData {
        let files = Layered {
            data: fork(data),
            plugin: fork(plugin),
        };
        GameData::load(
            &files,
            &files,
            Path::new("/data"),
            Some(Path::new("/plugins")),
        )
        .expect("opens")
    }

    /// A `'STR '` of `text`: its length byte, then the text.
    fn str_resource(text: &str) -> Vec<u8> {
        let mut bytes = vec![u8::try_from(text.len()).expect("short")];
        bytes.extend(text.as_bytes());
        bytes
    }

    const STOCK_PRICES: [&str; 6] = ["75", "350", "750", "900", "200", "550"];

    #[test]
    fn a_plugins_str_patches_arrive_raw_beside_str_4004() {
        let data = layered(
            &[
                (StrList::TYPE, 4000, str_list(&["Food"])),
                (StrList::TYPE, 4004, str_list(&STOCK_PRICES)),
            ],
            &[
                (StrResource::TYPE, 9301, str_resource("999")),
                (StrResource::TYPE, 9302, Vec::new()),
                (StrResource::TYPE, 9303, str_resource("")),
                (StrResource::TYPE, 9304, b"\x05ab".to_vec()),
                (StrResource::TYPE, 9305, b"\x03550xyz".to_vec()),
                (StrList::TYPE, 9300, str_list(&["1"])),
            ],
        );
        assert_eq!(
            data.commodity_strings(),
            CommodityStrings {
                names: vec!["Food".into()],
                name_patches: Default::default(),
                base_prices: STOCK_PRICES.map(str::to_owned).to_vec(),
                price_patches: [
                    StringPatch::Absent,
                    StringPatch::Text("999".into()),
                    StringPatch::Empty,
                    StringPatch::Text(String::new()),
                    StringPatch::Unreadable,
                    StringPatch::Text("550".into()),
                ],
            }
        );
    }

    #[test]
    fn a_str_patch_in_any_file_counts() {
        let data = layered(
            &[
                (StrResource::TYPE, 9300, str_resource("10")),
                (StrResource::TYPE, 9306, str_resource("70")),
                (StrResource::TYPE, 9299, str_resource("0")),
            ],
            &[],
        );
        let patches = data.commodity_strings().price_patches;
        assert_eq!(
            patches,
            [
                StringPatch::Text("10".into()),
                StringPatch::Absent,
                StringPatch::Absent,
                StringPatch::Absent,
                StringPatch::Absent,
                StringPatch::Absent,
            ],
            "only 9300-9305 patch prices"
        );
    }

    const STOCK_NAMES: [&str; 6] = [
        "Food",
        "Industrial",
        "Medical Supplies",
        "Luxury Goods",
        "Metal",
        "Equipment",
    ];

    #[test]
    fn a_plugins_str_name_patches_arrive_raw_beside_str_4000() {
        let data = layered(
            &[
                (StrList::TYPE, 4000, str_list(&STOCK_NAMES)),
                (StrList::TYPE, 4004, str_list(&STOCK_PRICES)),
            ],
            &[
                (StrResource::TYPE, 9000, str_resource("Grain")),
                (StrResource::TYPE, 9001, Vec::new()),
                (StrResource::TYPE, 9002, str_resource("")),
                (StrResource::TYPE, 9003, b"\x05ab".to_vec()),
                (StrResource::TYPE, 9005, b"\x04Gearxyz".to_vec()),
                (StrList::TYPE, 9004, str_list(&["X"])),
            ],
        );
        assert_eq!(
            data.commodity_strings(),
            CommodityStrings {
                names: STOCK_NAMES.map(str::to_owned).to_vec(),
                name_patches: [
                    StringPatch::Text("Grain".into()),
                    StringPatch::Empty,
                    StringPatch::Text(String::new()),
                    StringPatch::Unreadable,
                    StringPatch::Absent,
                    StringPatch::Text("Gear".into()),
                ],
                base_prices: STOCK_PRICES.map(str::to_owned).to_vec(),
                price_patches: Default::default(),
            }
        );
    }

    #[test]
    fn only_str_9000_to_9005_patch_names() {
        let outside = layered(
            &[
                (StrResource::TYPE, 8999, str_resource("Before")),
                (StrResource::TYPE, 9006, str_resource("After")),
                (StrResource::TYPE, 9300, str_resource("10")),
            ],
            &[],
        );
        assert_eq!(
            outside.commodity_strings().name_patches,
            <[StringPatch; 6]>::default()
        );
        let first = layered(&[(StrResource::TYPE, 9000, str_resource("Grain"))], &[]);
        assert_eq!(
            first.commodity_strings().name_patches[0],
            StringPatch::Text("Grain".into()),
            "a 'STR ' 9000 in the data file counts"
        );
    }

    /// A `jünk` sold at `sold`, bought at `bought` (every other slot -1),
    /// at `price`, named `lc_name` in lower case.
    fn junk(sold: &[i16], bought: &[i16], price: i16, lc_name: &str) -> Vec<u8> {
        let mut bytes = vec![0; Junk::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[-1; 16]);
        put_i16s(&mut bytes, 0x00, sold);
        put_i16s(&mut bytes, 0x10, bought);
        put_i16s(&mut bytes, 0x20, &[price]);
        bytes[0x26..0x26 + lc_name.len()].copy_from_slice(lc_name.as_bytes());
        bytes
    }

    #[test]
    fn each_readable_jünk_is_a_special_good_by_id() {
        let mut opals = junk(&[189, 165], &[185, -1, 199], 1200, "opals");
        opals[0xA6..0xA9].copy_from_slice(b"b43");
        opals[0x1A5..0x1A9].copy_from_slice(b"!b80");
        put_i16s(&mut opals, 0x22, &[0x0003]);
        let data = store_named(&[
            (Junk::TYPE, 146, Some("Opals"), opals),
            (Junk::TYPE, 134, None, junk(&[160], &[], 300, "water")),
            (
                Junk::TYPE,
                140,
                Some("Broken"),
                short(junk(&[], &[], 1, "x")),
            ),
        ]);
        assert_eq!(
            data.junk(),
            [
                JunkRecord {
                    id: JunkId(134),
                    name: "water".to_owned(),
                    base_price: 300,
                    sold_at: vec![StellarId(160)],
                    bought_at: Vec::new(),
                    buy_on: Test::default(),
                    sell_on: Test::default(),
                    flags: 0,
                },
                JunkRecord {
                    id: JunkId(146),
                    name: "Opals".to_owned(),
                    base_price: 1200,
                    sold_at: vec![StellarId(189), StellarId(165)],
                    bought_at: vec![StellarId(185), StellarId(199)],
                    buy_on: Test::parse("b43"),
                    sell_on: Test::parse("!b80"),
                    flags: 0x0003,
                },
            ],
            "a resource without a name goes by its LCName; an undecodable one is skipped"
        );
        assert_eq!(store(&[]).junk(), []);
        for (a, b) in data.junk().iter().zip(&data.junk()) {
            assert!(
                shared(&a.buy_on, &b.buy_on) && shared(&a.sell_on, &b.sell_on),
                "parsed once: {:?}",
                a.id
            );
        }
    }

    /// An `öops` at `stellar` moving `commodity` by `delta` for `duration`
    /// days, `freq` % a day.
    fn disaster(stellar: i16, commodity: i16, delta: i16, duration: i16, freq: i16) -> Vec<u8> {
        let mut bytes = vec![0; Disaster::SIZE.expect("fixed")];
        put_i16s(
            &mut bytes,
            0x00,
            &[stellar, commodity, delta, duration, freq],
        );
        bytes
    }

    #[test]
    fn a_disasters_name_is_its_resources_up_to_its_last_semicolon() {
        let data = store_named(&[
            (
                Disaster::TYPE,
                128,
                Some("A glut;  ;extra"),
                disaster(137, 0, -15, 30, 35),
            ),
            (
                Disaster::TYPE,
                129,
                Some(";only a note"),
                disaster(137, 0, -15, 30, 35),
            ),
        ]);
        let names: Vec<_> = data.disasters().into_iter().map(|d| d.name).collect();
        assert_eq!(
            names,
            ["A glut", ""],
            "cut at the last ';', trailing spaces and ';'s dropped (_LoadObjectData @0x7cdd3)"
        );
    }

    #[test]
    fn each_readable_öops_is_an_event_by_id() {
        let mut gated = disaster(-2, 3, 40, 100, 25);
        gated[0x0A..0x0E].copy_from_slice(b"!b80");
        let data = store_named(&[
            (Disaster::TYPE, 129, None, gated),
            (
                Disaster::TYPE,
                128,
                Some("An enormous food surplus"),
                disaster(137, 0, -15, 30, 35),
            ),
            (
                Disaster::TYPE,
                130,
                Some("x"),
                short(disaster(1, 1, 1, 1, 1)),
            ),
            (
                Disaster::TYPE,
                131,
                Some("Anywhere"),
                disaster(-1, 1, 1, 1, 1),
            ),
        ]);
        assert_eq!(
            data.disasters(),
            [
                DisasterRecord {
                    id: DisasterId(128),
                    name: "An enormous food surplus".to_owned(),
                    stellar: 137,
                    commodity: 0,
                    price_delta: -15,
                    duration: 30,
                    freq: 35,
                    activate_on: Test::default(),
                },
                DisasterRecord {
                    id: DisasterId(129),
                    name: String::new(),
                    stellar: -2,
                    commodity: 3,
                    price_delta: 40,
                    duration: 100,
                    freq: 25,
                    activate_on: Test::parse("!b80"),
                },
                DisasterRecord {
                    id: DisasterId(131),
                    name: "Anywhere".to_owned(),
                    stellar: -1,
                    commodity: 1,
                    price_delta: 1,
                    duration: 1,
                    freq: 1,
                    activate_on: Test::default(),
                },
            ]
        );
        assert_eq!(store(&[]).disasters(), []);
        for (a, b) in data.disasters().iter().zip(&data.disasters()) {
            assert!(
                shared(&a.activate_on, &b.activate_on),
                "parsed once: {:?}",
                a.id
            );
        }
    }

    #[test]
    fn stellar_lives_list_every_readable_spöb_by_id_with_its_strength_dead_time_flags2_explosion_and_scripts()
     {
        let mut whole = stellar(0, 0, 0, 0, 0);
        whole[0x23C..0x240].copy_from_slice(&70_000_i32.to_be_bytes());
        put_i16s(&mut whole, 0x242, &[-1, 1004]);
        whole[0x246..0x249].copy_from_slice(b"b12");
        whole[0x345..0x34A].copy_from_slice(b"b9 b8");
        let mut wreck = flagged2(stellar(0, 0, 0, 0, 0), 0x0040 | 0x0020);
        wreck[0x23C..0x240].copy_from_slice(&(-1_i32).to_be_bytes());
        put_i16s(&mut wreck, 0x242, &[3, -1]);
        let data = store(&[
            (Stellar::TYPE, 130, wreck),
            (Stellar::TYPE, 128, whole),
            (Stellar::TYPE, 129, short(stellar(0, 0, 0, 1, 0))),
        ]);
        let lives = data.stellar_lives();
        assert_eq!(
            lives
                .iter()
                .map(|life| (life.id, life.strength, life.dead_time, life.flags2))
                .collect::<Vec<_>>(),
            [
                (StellarId(128), 70_000, -1, 0),
                (StellarId(130), -1, 3, 0x0060),
            ],
            "the unreadable 129 left out"
        );
        assert_eq!(
            lives[0].on_regen.tree().map(|expr| expr.ops.len()),
            Some(2),
            "b9 b8"
        );
        assert_eq!(lives[1].on_regen, Script::default(), "blank");
        assert_eq!(
            lives.iter().map(|life| life.explosion).collect::<Vec<_>>(),
            [Explosion::decode(1004), None],
            "-1 for none"
        );
        assert!(Explosion::decode(1004).is_some_and(|boom| boom.extra));
        assert_eq!(
            lives[0].on_destroy.tree().map(|expr| expr.ops.len()),
            Some(1),
            "b12"
        );
        assert_eq!(lives[1].on_destroy, Script::default(), "blank");
        assert!(
            shared_script(&lives[0].on_regen, &data.stellar_lives()[0].on_regen),
            "parsed once"
        );
        assert_eq!(store(&[]).stellar_lives(), []);
    }

    #[test]
    fn stellar_flags_lists_every_readable_spöb_by_id_with_its_flags() {
        let data = store(&[
            (Stellar::TYPE, 130, stellar(0, 0, 0, 0x20, 0)),
            (Stellar::TYPE, 128, stellar(0, 0, 0, 0x2, 0)),
            (Stellar::TYPE, 129, short(stellar(0, 0, 0, 1, 0))),
        ]);
        assert_eq!(
            data.stellar_flags(),
            [(StellarId(128), 0x2), (StellarId(130), 0x20)],
            "the unreadable 129 left out"
        );
        assert_eq!(store(&[]).stellar_flags(), []);
    }

    /// A `wëap` with every field the combat catalog reads set to
    /// something of its own.
    fn weapon_bytes() -> Vec<u8> {
        let mut bytes = vec![0; Weapon::SIZE.expect("fixed")];
        put_i16s(
            &mut bytes,
            0x00,
            &[10, 13, 1, 4, -1, 1500, -1010, 7, 9, 3, 25, 1003, 5, 6],
        );
        bytes[0x1C..0x1E].copy_from_slice(&0x6102_u16.to_be_bytes());
        bytes[0x1E..0x20].copy_from_slice(&0x0021_u16.to_be_bytes());
        put_i16s(&mut bytes, 0x22, &[100]);
        put_i16s(&mut bytes, 0x30, &[300]);
        bytes[0x48..0x4A].copy_from_slice(&0x8200_u16.to_be_bytes());
        put_i16s(&mut bytes, 0x5A, &[60, 30]);
        bytes[0x66..0x68].copy_from_slice(&0x0003_u16.to_be_bytes());
        put_i16s(&mut bytes, 0x3E, &[5, 148, -10, 2, 8]);
        put_i16s(&mut bytes, 0x68, &[4, 70, 6]);
        bytes
    }

    #[test]
    fn each_readable_wëap_is_a_weapon_record_by_id() {
        let data = store(&[
            (Weapon::TYPE, 140, weapon_bytes()),
            (Weapon::TYPE, 128, weapon_bytes()),
            (Weapon::TYPE, 129, short(weapon_bytes())),
        ]);
        let record = |id| WeaponRecord {
            id: WeaponId(id),
            reload: 10,
            count: 13,
            mass_dmg: 1,
            energy_dmg: 4,
            guidance: -1,
            speed: 1500,
            ammo_type: -1010,
            inaccuracy: 9,
            impact: 25,
            explod_type: 1003,
            prox_radius: 5,
            blast_radius: 6,
            flags: 0x6102,
            seeker: 0x0021,
            flags2: 0x8200,
            flags3: 0x0003,
            decay: 100,
            beam_length: 300,
            burst_count: 60,
            burst_reload: 30,
            guided_turn: 70,
            durability: 4,
            sub_count: 5,
            sub_type: Some(WeaponId(148)),
            sub_theta: -10,
            sub_limit: 2,
            max_ammo: 6,
            prox_safety: 8,
        };
        assert_eq!(
            data.weapons(),
            [record(128), record(140)],
            "by ID, the undecodable one left out"
        );
        assert_eq!(store(&[]).weapons(), []);
    }

    #[test]
    fn a_wëap_without_a_sub_type_has_none() {
        let mut bytes = weapon_bytes();
        put_i16s(&mut bytes, 0x40, &[-1]);
        let data = store(&[(Weapon::TYPE, 128, bytes)]);
        assert_eq!(data.weapons()[0].sub_type, None);
        assert_eq!(data.weapons()[0].sub_count, 5);
    }

    /// A `shïp` with every combat field set to something of its own: two
    /// stock weapons in slots 1-4 (an unused and a 0 slot among them) and
    /// one in slots 5-8.
    fn armed() -> Vec<u8> {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x12, &[128, -1, 0, 130]);
        put_i16s(&mut bytes, 0x1A, &[2, 5, 6, 1]);
        put_i16s(&mut bytes, 0x22, &[0, 0, 0, 40]);
        put_i16s(&mut bytes, 0x34, &[60]);
        put_i16s(&mut bytes, 0x38, &[4, 1005]);
        put_i16s(&mut bytes, 0x3E, &[120]);
        put_i16s(&mut bytes, 0x46, &[325]);
        bytes[0x4A..0x4C].copy_from_slice(&0x0130_u16.to_be_bytes());
        put_i16s(&mut bytes, 0x6CE, &[-1, 138, -1, -1]);
        put_i16s(&mut bytes, 0x6D6, &[0, 3, 0, 0]);
        put_i16s(&mut bytes, 0x6DE, &[0, 20, 0, 0]);
        bytes
    }

    /// A `shän` whose `BaseXSize` is `size`.
    fn anim(width: i16, height: i16) -> Vec<u8> {
        let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x06, &[width, height]);
        bytes
    }

    #[test]
    fn each_readable_shïp_is_a_hull_record_by_id_with_its_shäns_size() {
        // Ship 128's sprite is 24 x 40.
        let data = store(&[
            (Ship::TYPE, 130, armed()),
            (Ship::TYPE, 128, armed()),
            (Ship::TYPE, 129, short(armed())),
            (Ship::TYPE, 131, armed()),
            (ShipAnim::TYPE, 128, anim(24, 40)),
            (ShipAnim::TYPE, 131, short(anim(30, 30))),
        ]);
        let record = |id, size, height| HullRecord {
            id: ShipId(id),
            flags: 0x0130,
            death_delay: 60,
            explode1: 4,
            explode2: 1005,
            mass: 120,
            weapons: vec![
                StockWeapon {
                    weapon: WeaponId(128),
                    count: 2,
                    ammo: 0,
                },
                StockWeapon {
                    weapon: WeaponId(130),
                    count: 1,
                    ammo: 40,
                },
                StockWeapon {
                    weapon: WeaponId(138),
                    count: 3,
                    ammo: 20,
                },
            ],
            size,
            height,
            strength: 325,
        };
        assert_eq!(
            data.hulls(),
            [
                record(128, Some(24), Some(40)),
                record(130, None, None),
                record(131, None, None)
            ],
            "by ID, the undecodable shïp left out; without a shän that can be read, no size"
        );
        assert_eq!(store(&[]).hulls(), []);
    }

    /// A `gövt` with every field the combat catalog reads set to something
    /// of its own.
    fn govt_bytes() -> Vec<u8> {
        let mut bytes = vec![0; Govt::SIZE.expect("fixed")];
        bytes[0x02..0x04].copy_from_slice(&0xE2B0_u16.to_be_bytes());
        bytes[0x04..0x06].copy_from_slice(&0x0012_u16.to_be_bytes());
        put_i16s(&mut bytes, 0x06, &[-5]);
        put_i16s(&mut bytes, 0x08, &[6, 1, 3, 5, 7, 2, -100, 200]);
        put_i16s(&mut bytes, 0x18, &[1, -1, 4, -1]);
        put_i16s(&mut bytes, 0x20, &[0, 1, 12, 13]);
        put_i16s(&mut bytes, 0x28, &[2, 10, 16, 9]);
        bytes[0x32..0x34].copy_from_slice(&0x8004_u16.to_be_bytes());
        bytes[0x34..0x3F].copy_from_slice(b"Federation\0");
        put_i16s(&mut bytes, 0xAE, &[9001]);
        bytes[0x54..0x5C].copy_from_slice(&0x8000_0000_0000_0021_u64.to_be_bytes());
        bytes
    }

    #[test]
    fn a_govt_record_carries_its_require() {
        let data = store(&[(Govt::TYPE, 128, govt_bytes())]);
        assert_eq!(
            data.governments()
                .iter()
                .map(|record| record.require)
                .collect::<Vec<_>>(),
            [0x8000_0000_0000_0021]
        );
    }

    #[test]
    fn each_readable_gövt_is_a_government_record_by_id() {
        let data = store(&[
            (Govt::TYPE, 140, govt_bytes()),
            (Govt::TYPE, 128, govt_bytes()),
            (Govt::TYPE, 129, short(govt_bytes())),
        ]);
        let record = |id| GovtRecord {
            id: GovtId(id),
            flags: 0xE2B0,
            flags2: 0x0012,
            crime_tol: 6,
            initial_rec: -100,
            penalties: Penalties {
                smuggle: 1,
                disable: 3,
                board: 5,
                kill: 7,
                shoot: 2,
            },
            max_odds: 200,
            classes: [1, -1, 4, -1],
            allies: [0, 1, 12, 13],
            enemies: [2, 10, 16, 9],
            comm_name: "Federation".to_owned(),
            scan_mask: 0x8004,
            scan_fine: -5,
            news_pic: 9001,
            require: 0x8000_0000_0000_0021,
        };
        assert_eq!(
            data.governments(),
            [record(128), record(140)],
            "by ID, the undecodable one left out"
        );
        assert_eq!(store(&[]).governments(), []);
    }

    /// A `sÿst` with these `DudeTypes` and `% Prob` and this `AvgShips`.
    fn trafficked(dude_types: [i16; 8], prob: [i16; 8], avg_ships: i16) -> Vec<u8> {
        let mut bytes = system();
        put_i16s(&mut bytes, 0x44, &dude_types);
        put_i16s(&mut bytes, 0x54, &prob);
        put_i16s(&mut bytes, 0x64, &[avg_ships]);
        bytes
    }

    #[test]
    fn a_systems_traffic_is_its_dude_types_with_their_odds_and_its_average() {
        let data = store(&[
            (
                System::TYPE,
                130,
                trafficked(
                    [128, -129, -1, 639, -383, 0, 640, 5],
                    [60, 20, 0, 1, 2, 3, 4, 5],
                    7,
                ),
            ),
            (System::TYPE, 131, short(system())),
        ]);
        assert_eq!(
            data.system_traffic(SystemId(130)),
            Some(SystemTraffic {
                dude_types: [
                    (128, 60),
                    (-129, 20),
                    (-1, 0),
                    (639, 1),
                    (-383, 2),
                    (0, 3),
                    (640, 4),
                    (5, 5)
                ],
                avg_ships: 7,
                persons: Default::default(),
            }),
            "every slot, raw"
        );
        assert_eq!(data.system_traffic(SystemId(131)), None, "undecodable");
        assert_eq!(data.system_traffic(SystemId(132)), None, "missing");
    }

    /// A `düde` of `ai_type` for `govt`, flying these ships with their
    /// probabilities in its first slots; the rest unused.
    fn dude(ai_type: i16, govt: i16, ships: &[(i16, i16)]) -> Vec<u8> {
        let mut bytes = vec![0; Dude::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[ai_type, govt]);
        put_i16s(&mut bytes, 0x08, &[-1; 16]);
        for (slot, (ship, probability)) in ships.iter().enumerate() {
            put_i16s(&mut bytes, 0x08 + 2 * slot, &[*ship]);
            put_i16s(&mut bytes, 0x28 + 2 * slot, &[*probability]);
        }
        bytes
    }

    #[test]
    fn a_dude_is_its_ai_type_government_and_the_slots_that_name_a_ship() {
        let data = store(&[
            (
                Dude::TYPE,
                128,
                dude(3, 129, &[(140, 60), (-1, 30), (141, 10)]),
            ),
            (Dude::TYPE, 129, dude(-1, -1, &[])),
            (Dude::TYPE, 130, short(dude(1, 128, &[(140, 100)]))),
        ]);
        assert_eq!(
            data.dude(DudeId(128)),
            Some(DudeRecord {
                ai_type: 3,
                govt: Some(GovtId(129)),
                ships: vec![(ShipId(140), 60), (ShipId(141), 10)],
                booty: 0,
                info_types: 0,
            }),
            "the unused slot is left out"
        );
        assert_eq!(
            data.dude(DudeId(129)),
            Some(DudeRecord {
                ai_type: -1,
                govt: None,
                ships: Vec::new(),
                booty: 0,
                info_types: 0,
            })
        );
        assert_eq!(data.dude(DudeId(130)), None, "undecodable");
        assert_eq!(data.dude(DudeId(131)), None, "missing");
    }

    #[test]
    fn a_dudes_booty_is_its_booty_flags() {
        let mut looted = dude(1, 128, &[(140, 100)]);
        put_i16s(&mut looted, 0x04, &[0x0041]);
        let mut every = dude(1, 128, &[(140, 100)]);
        put_i16s(&mut every, 0x04, &[-1]);
        let data = store(&[(Dude::TYPE, 128, looted), (Dude::TYPE, 129, every)]);
        assert_eq!(data.dude(DudeId(128)).map(|dude| dude.booty), Some(0x0041));
        assert_eq!(data.dude(DudeId(129)).map(|dude| dude.booty), Some(0xFFFF));
    }

    #[test]
    fn a_dudes_info_types_are_its_hail_information_bits() {
        let mut advising = dude(1, 128, &[(140, 100)]);
        put_i16s(&mut advising, 0x06, &[0x4005]);
        let mut every = dude(1, 128, &[(140, 100)]);
        put_i16s(&mut every, 0x06, &[-1]);
        let data = store(&[(Dude::TYPE, 128, advising), (Dude::TYPE, 129, every)]);
        assert_eq!(
            data.dude(DudeId(128)).map(|dude| dude.info_types),
            Some(0x4005)
        );
        assert_eq!(
            data.dude(DudeId(129)).map(|dude| dude.info_types),
            Some(0xFFFF)
        );
    }

    #[test]
    fn a_string_list_is_its_str_whole_and_none_when_missing_or_undecodable() {
        let data = store(&[
            (StrList::TYPE, 3000, str_list(&["Channel open.", "", "*Hi"])),
            (StrList::TYPE, 3001, short(str_list(&["Broken"]))),
        ]);
        assert_eq!(
            data.string_list(3000),
            ["Channel open.", "", "*Hi"],
            "every string, raw"
        );
        assert!(data.string_list(3001).is_empty(), "undecodable");
        assert!(data.string_list(3002).is_empty(), "missing");
    }

    /// A `flët` led by `lead`, with these escorts (type, min, max) in its
    /// first slots, the rest unused, for `govt`, linked to `link_syst`.
    fn fleet(lead: i16, escorts: &[(i16, i16, i16)], govt: i16, link_syst: i16) -> Vec<u8> {
        let mut bytes = vec![0; Fleet::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[lead]);
        put_i16s(&mut bytes, 0x02, &[-1; 4]);
        for (slot, (ship, min, max)) in escorts.iter().enumerate() {
            put_i16s(&mut bytes, 0x02 + 2 * slot, &[*ship]);
            put_i16s(&mut bytes, 0x0A + 2 * slot, &[*min]);
            put_i16s(&mut bytes, 0x12 + 2 * slot, &[*max]);
        }
        put_i16s(&mut bytes, 0x1A, &[govt, link_syst]);
        bytes
    }

    #[test]
    fn each_readable_flet_is_a_fleet_record_by_id() {
        let mut appearing = fleet(-1, &[(-1, 1, 1), (142, 0, 3)], -1, 10_000);
        appearing[0x1E..0x22].copy_from_slice(b"b42\0");
        let data = store(&[
            (Fleet::TYPE, 130, appearing),
            (Fleet::TYPE, 128, fleet(140, &[(141, 1, 2)], 129, -1)),
            (Fleet::TYPE, 129, short(fleet(140, &[], 128, -1))),
        ]);
        assert_eq!(
            data.fleets(),
            [
                FleetRecord {
                    id: FleetId(128),
                    lead: Some(ShipId(140)),
                    escorts: vec![EscortRecord {
                        ship: ShipId(141),
                        min: 1,
                        max: 2,
                    }],
                    govt: Some(GovtId(129)),
                    link_syst: -1,
                    appear_on: Test::default(),
                },
                FleetRecord {
                    id: FleetId(130),
                    lead: None,
                    escorts: vec![EscortRecord {
                        ship: ShipId(142),
                        min: 0,
                        max: 3,
                    }],
                    govt: None,
                    link_syst: 10_000,
                    appear_on: Test::parse("b42"),
                },
            ],
            "by ID, the undecodable one skipped and the unused escort slots left out"
        );
        assert_eq!(store(&[]).fleets(), []);
        for (a, b) in data.fleets().iter().zip(&data.fleets()) {
            assert!(
                shared(&a.appear_on, &b.appear_on),
                "parsed once: {:?}",
                a.id
            );
        }
    }

    /// A `përs` linked to `link_syst`, of `govt`, AI type 3, `Aggress` 2
    /// and `Coward` 25, flying `ship`, with these weapon slots (type,
    /// count, ammunition), 8000 credits, `ShieldMod` 250, `HailPict`
    /// `hail_pict`, `CommQuote` 24, `HailQuote` 8, no mission, `Flags`
    /// 0x0090, `ActiveOn` "b0 & !b8", subtitle "Top Gun" and `Flags2`
    /// 0x0001.
    fn person(
        link_syst: i16,
        govt: i16,
        ship: i16,
        weapons: [(i16, i16, i16); 4],
        hail_pict: i16,
    ) -> Vec<u8> {
        let mut bytes = vec![0; Person::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[link_syst, govt, 3, 2, 25, ship]);
        for (slot, (weapon, count, ammo)) in weapons.into_iter().enumerate() {
            put_i16s(&mut bytes, 0x0C + 2 * slot, &[weapon]);
            put_i16s(&mut bytes, 0x14 + 2 * slot, &[count]);
            put_i16s(&mut bytes, 0x1C + 2 * slot, &[ammo]);
        }
        bytes[0x24..0x28].copy_from_slice(&8000_i32.to_be_bytes());
        put_i16s(&mut bytes, 0x28, &[250, hail_pict, 24, 8, -1]);
        bytes[0x32..0x34].copy_from_slice(&0x0090_u16.to_be_bytes());
        bytes[0x34..0x34 + 9].copy_from_slice(b"b0 & !b8\0");
        bytes[0x13A..0x13A + 8].copy_from_slice(b"Top Gun\0");
        bytes[0x17E..0x180].copy_from_slice(&0x0001_u16.to_be_bytes());
        bytes
    }

    #[test]
    fn each_readable_përs_is_a_person_record_by_id() {
        let slots = [(132, 2, 0), (-1, 5, 5), (127, 1, 1), (135, 2, 50)];
        let data = store_named(&[
            (
                Person::TYPE,
                131,
                Some("Ace;designer note "),
                person(132, 128, 279, slots, 127),
            ),
            (
                Person::TYPE,
                130,
                None,
                person(-1, -1, 140, [(-1, 0, 0); 4], 7800),
            ),
            (
                Person::TYPE,
                132,
                Some("Broken"),
                short(person(-1, -1, 140, [(-1, 0, 0); 4], -1)),
            ),
        ]);
        let persons = data.persons();
        assert_eq!(
            persons.iter().map(|person| person.id).collect::<Vec<_>>(),
            [PersonId(130), PersonId(131)],
            "by ID, the undecodable one skipped"
        );
        let ace = &persons[1];
        assert_eq!(
            ace,
            &PersonRecord {
                id: PersonId(131),
                name: "Ace".to_owned(),
                link_syst: 132,
                govt: Some(GovtId(128)),
                ai_type: 3,
                aggress: 2,
                coward: 25,
                ship: Some(ShipId(279)),
                weapons: vec![
                    PersonWeapon {
                        weapon: WeaponId(132),
                        count: 2,
                        ammo: 0,
                    },
                    PersonWeapon {
                        weapon: WeaponId(135),
                        count: 2,
                        ammo: 50,
                    },
                ],
                credits: 8000,
                shield_mod: 250,
                hail_pict: None,
                comm_quote: 24,
                hail_quote: 8,
                link_mission: None,
                flags: 0x0090,
                active_on: Test::parse("b0 & !b8"),
                subtitle: "Top Gun".to_owned(),
                flags2: 0x0001,
                grant_class: 0,
                grant_count: 0,
                grant_prob: 0,
                mission_ship: None,
            },
            "the -1 and 127 weapon slots left out, HailPict 127 none"
        );
        let nameless = &persons[0];
        assert_eq!(nameless.name, "");
        assert_eq!(nameless.govt, None);
        assert_eq!(nameless.hail_pict, Some(7800));
        assert!(nameless.weapons.is_empty());
        assert_eq!(store(&[]).persons(), []);
        for (a, b) in data.persons().iter().zip(&data.persons()) {
            assert!(
                shared(&a.active_on, &b.active_on),
                "parsed once: {:?}",
                a.id
            );
        }
    }

    #[test]
    fn a_persons_name_is_its_resources_up_to_its_last_semicolon_trimmed_to_29() {
        for (resource, name) in [
            ("Ace;designer note ", "Ace"),
            ("Jack Folstam", "Jack Folstam"),
            ("Bounty Hunter  ;", "Bounty Hunter"),
            ("A;B;note", "A;B"),
            ("Ace;;note", "Ace"),
            (
                "A very long name of thirty-one",
                "A very long name of thirty-on",
            ),
        ] {
            assert_eq!(person_name(resource), name, "{resource}");
        }
        assert_eq!(PERSON_NAME_CHARS, 29);
    }

    #[test]
    fn a_persons_grant_is_its_grant_class_count_and_odds_raw() {
        let mut granting = person(-1, -1, 140, [(-1, 0, 0); 4], -1);
        put_i16s(&mut granting, 0x134, &[7, 3, 40]);
        let data = store(&[(Person::TYPE, 128, granting)]);
        let record = &data.persons()[0];
        assert_eq!(
            (record.grant_class, record.grant_count, record.grant_prob),
            (7, 3, 40)
        );
    }

    #[test]
    fn a_persons_mission_is_its_link_mission_above_127() {
        let mut linked = person(-1, -1, 140, [(-1, 0, 0); 4], -1);
        put_i16s(&mut linked, 0x30, &[400]);
        let mut low = person(-1, -1, 140, [(-1, 0, 0); 4], -1);
        put_i16s(&mut low, 0x30, &[127]);
        let data = store(&[(Person::TYPE, 128, linked), (Person::TYPE, 129, low)]);
        assert_eq!(data.persons()[0].link_mission, Some(400));
        assert_eq!(data.persons()[1].link_mission, None, "127 names none");
    }

    /// A `mïsn` of `ShipCount` `count` and `ShipGoal` `goal`.
    fn mission(count: i16, goal: i16) -> Vec<u8> {
        use nova_data::records::mission::Mission;
        let mut bytes = vec![0; Mission::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x20, &[count]);
        put_i16s(&mut bytes, 0x26, &[goal]);
        bytes
    }

    #[test]
    fn a_persons_mission_ship_is_its_link_missions_ship_count_and_goal() {
        use nova_data::records::mission::Mission;
        let linking = |mission: i16| {
            let mut bytes = person(-1, -1, 140, [(-1, 0, 0); 4], -1);
            put_i16s(&mut bytes, 0x30, &[mission]);
            bytes
        };
        let data = store(&[
            (Person::TYPE, 128, linking(132)),
            (Person::TYPE, 129, linking(-1)),
            (Person::TYPE, 130, linking(140)),
            (Person::TYPE, 131, linking(133)),
            (Mission::TYPE, 132, mission(1, 3)),
            (Mission::TYPE, 133, short(mission(1, 3))),
        ]);
        let persons = data.persons();
        assert_eq!(
            persons[0].mission_ship.map(|ship| (ship.count, ship.goal)),
            Some((1, 3))
        );
        assert_eq!(persons[1].mission_ship, None, "no LinkMission");
        assert_eq!(persons[2].mission_ship, None, "its mission missing");
        assert_eq!(persons[3].mission_ship, None, "its mission unreadable");
    }

    /// A `mïsn` offered as `avail_bits` says, with every field this phase
    /// reads set to a value of its own.
    fn offered(avail_bits: &[u8]) -> Vec<u8> {
        use nova_data::records::mission::Mission;
        let mut bytes = vec![0; Mission::SIZE.expect("fixed")];
        put_i16s(
            &mut bytes,
            0x00,
            &[10_005, 0, 0, -5, 40, 25, -2, -1, 1000, -10, 1, -1],
        );
        bytes[0x18..0x1A].copy_from_slice(&0x8020_u16.to_be_bytes());
        bytes[0x1C..0x20].copy_from_slice(&(-50_100_i32).to_be_bytes());
        put_i16s(&mut bytes, 0x20, &[2, -5, 150, 1, 2, 7000, -3]);
        put_i16s(&mut bytes, 0x2E, &[383, 7, 7001]);
        put_i16s(&mut bytes, 0x40, &[3, 0]);
        put_i16s(&mut bytes, 0x48, &[4, 151, -3]);
        put_i16s(&mut bytes, 0x50, &[0x2008, 0x0001]);
        put_i16s(&mut bytes, 0x5A, &[1129]);
        bytes[0x5C..0x5C + avail_bits.len()].copy_from_slice(avail_bits);
        for (offset, script) in [
            (0x15B, b"b10\0"),
            (0x25A, b"b11\0"),
            (0x359, b"b12\0"),
            (0x458, b"b13\0"),
            (0x557, b"b14\0"),
            (0x660, b"b15\0"),
        ] {
            bytes[offset..offset + script.len()].copy_from_slice(script);
        }
        bytes[0x656..0x65E].copy_from_slice(&0x0000_0004_0000_0001_u64.to_be_bytes());
        put_i16s(&mut bytes, 0x65E, &[4]);
        put_i16s(&mut bytes, 0x7A0, &[60]);
        bytes
    }

    /// [`offered`] with `CompGovt` `govt`.
    fn rewarding(govt: i16) -> Vec<u8> {
        let mut bytes = offered(b"\0");
        put_i16s(&mut bytes, 0x2E, &[govt]);
        bytes
    }

    /// A `crön` from 1 Sep 1177 to 30 Dec (any year), `Random` 100,
    /// `Duration` 105, holdoffs 2 and 3, `Flags` 0x0002, these expressions,
    /// and `Contribute` `0x4_0000_0001` and `Require` 8.
    fn cron(enable_on: &[u8], on_start: &[u8], on_end: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0; Cron::SIZE.expect("fixed")];
        put_i16s(
            &mut bytes,
            0x00,
            &[1, 9, 1177, 30, 12, -1, 100, 105, 2, 3, -1, 0x0002],
        );
        bytes[0x18..0x18 + enable_on.len()].copy_from_slice(enable_on);
        bytes[0x117..0x117 + on_start.len()].copy_from_slice(on_start);
        bytes[0x216..0x216 + on_end.len()].copy_from_slice(on_end);
        bytes[0x316..0x31E].copy_from_slice(&0x0000_0004_0000_0001_u64.to_be_bytes());
        bytes[0x31E..0x326].copy_from_slice(&8_u64.to_be_bytes());
        bytes
    }

    #[test]
    fn crons_read_each_crön_raw() {
        let data = store_named(&[
            (Cron::TYPE, 157, None, cron(b"\0", b"\0", b"\0")),
            (Cron::TYPE, 156, None, cron(b"!b42\0", b"b42\0", b"!b42\0")),
            (
                Cron::TYPE,
                158,
                None,
                cron(b"\0", b"\0", b"\0")[..800].to_vec(),
            ),
        ]);
        let crons = data.crons();
        assert_eq!(
            crons.iter().map(|cron| cron.id).collect::<Vec<_>>(),
            [CronId(156), CronId(157)],
            "by ID, the undecodable one skipped"
        );
        assert_eq!(
            crons[0],
            CronRecord {
                id: CronId(156),
                first_day: 1,
                first_month: 9,
                first_year: 1177,
                last_day: 30,
                last_month: 12,
                last_year: -1,
                random: 100,
                duration: 105,
                pre_holdoff: 2,
                post_holdoff: 3,
                flags: 0x0002,
                enable_on: Test::parse("!b42"),
                on_start: Script::parse("b42"),
                on_end: Script::parse("!b42"),
                contribute: 0x0000_0004_0000_0001,
                require: 8,
                news_govt: [0; 4],
                govt_news: [0; 4],
                ind_news: -1,
            }
        );
        assert_eq!(crons[1].enable_on, Test::default(), "a blank EnableOn");
        assert_eq!(
            crons[1].enable_on.tree(),
            Some(&crate::control::TestExpr::Always)
        );
        assert_eq!(crons[1].on_start, Script::default());
        assert_eq!(store(&[]).crons(), []);
        for (a, b) in data.crons().iter().zip(&data.crons()) {
            assert!(shared(&a.enable_on, &b.enable_on), "parsed once");
            assert!(shared_script(&a.on_start, &b.on_start), "parsed once");
            assert!(shared_script(&a.on_end, &b.on_end), "parsed once");
        }
    }

    #[test]
    fn crons_carry_their_news_fields_raw() {
        let mut bytes = cron(b"\0", b"\0", b"\0");
        put_i16s(&mut bytes, 0x14, &[0]);
        put_i16s(&mut bytes, 0x326, &[130, 0, -1, 0, 15_000, 0, -1, 0]);
        let data = store_named(&[(Cron::TYPE, 156, None, bytes)]);
        let record = &data.crons()[0];
        assert_eq!(record.news_govt, [130, 0, -1, 0], "-1 and 0 both kept");
        assert_eq!(record.govt_news, [15_000, 0, -1, 0]);
        assert_eq!(record.ind_news, 0);
        let crons = store_named(&[(Cron::TYPE, 157, None, cron(b"\0", b"\0", b"\0"))]).crons();
        assert_eq!(crons[0].ind_news, -1, "an IndNews of -1 stays -1");
    }

    /// A `ränk` of `Weight` -3 affiliated with `affil_govt`, `PriceMod`
    /// 85, `Salary` -200, `SalaryCap` 350000, `Contribute`
    /// `0x4_0000_0001`, `Flags` 0x0018, `ConvName` "Commander" and
    /// `ShortName` "Cmdr".
    fn rank(affil_govt: i16) -> Vec<u8> {
        use nova_data::records::rank::Rank;
        let mut bytes = vec![0; Rank::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[-3, affil_govt, 85]);
        bytes[0x06..0x0A].copy_from_slice(&(-200_i32).to_be_bytes());
        bytes[0x0A..0x0E].copy_from_slice(&350_000_i32.to_be_bytes());
        bytes[0x0E..0x16].copy_from_slice(&0x0000_0004_0000_0001_u64.to_be_bytes());
        put_i16s(&mut bytes, 0x16, &[0x0018]);
        bytes[0x18..0x18 + 9].copy_from_slice(b"Commander");
        bytes[0x58..0x58 + 4].copy_from_slice(b"Cmdr");
        bytes
    }

    #[test]
    fn ranks_read_each_rank_raw_with_its_name_up_to_its_last_semicolon() {
        use nova_data::records::rank::Rank;
        let data = store_named(&[
            (Rank::TYPE, 140, Some("; Rebel 1"), rank(130)),
            (Rank::TYPE, 128, Some("T5; Vell-os 1"), rank(129)),
            (Rank::TYPE, 129, None, rank(129)),
            (Rank::TYPE, 141, Some("Broken"), rank(129)[..100].to_vec()),
        ]);
        let ranks = data.ranks();
        assert_eq!(
            ranks.iter().map(|rank| rank.id).collect::<Vec<_>>(),
            [RankId(128), RankId(129), RankId(140)],
            "by ID, the undecodable one skipped"
        );
        assert_eq!(
            ranks[0],
            RankRecord {
                id: RankId(128),
                name: "T5".to_owned(),
                weight: -3,
                affil_govt: Some(GovtId(129)),
                salary: -200,
                salary_cap: 350_000,
                contribute: 0x0000_0004_0000_0001,
                flags: 0x0018,
                conv_name: "Commander".to_owned(),
                short_name: "Cmdr".to_owned(),
                price_mod: 85,
            }
        );
        assert_eq!(ranks[1].name, "", "no resource name");
        assert_eq!(ranks[2].name, "", "nothing before the last ';'");
        assert_eq!(ranks[2].affil_govt, Some(GovtId(130)));
        assert_eq!(store(&[]).ranks(), []);
    }

    #[test]
    fn a_rank_price_mod_of_0_or_below_loads_as_100() {
        use nova_data::records::rank::Rank;
        let price_mod = |raw: i16| {
            let mut bytes = rank(128);
            put_i16s(&mut bytes, 0x04, &[raw]);
            bytes
        };
        let data = store(&[
            (Rank::TYPE, 128, price_mod(0)),
            (Rank::TYPE, 129, price_mod(-5)),
            (Rank::TYPE, 130, price_mod(1)),
        ]);
        assert_eq!(
            data.ranks()
                .iter()
                .map(|rank| rank.price_mod)
                .collect::<Vec<_>>(),
            [100, 100, 1],
            "@0x7db33-0x7db3a"
        );
    }

    #[test]
    fn a_rank_affiliated_at_127_or_below_is_unaffiliated() {
        use nova_data::records::rank::Rank;
        let data = store(&[
            (Rank::TYPE, 128, rank(127)),
            (Rank::TYPE, 129, rank(-1)),
            (Rank::TYPE, 130, rank(128)),
            (Rank::TYPE, 131, rank(0)),
        ]);
        assert_eq!(
            data.ranks()
                .iter()
                .map(|rank| rank.affil_govt)
                .collect::<Vec<_>>(),
            [None, None, Some(GovtId(128)), None]
        );
    }

    #[test]
    fn only_ranks_128_to_255_are_read() {
        use nova_data::records::rank::Rank;
        let data = store(&[
            (Rank::TYPE, 127, rank(128)),
            (Rank::TYPE, 128, rank(128)),
            (Rank::TYPE, 255, rank(128)),
            (Rank::TYPE, 256, rank(128)),
        ]);
        assert_eq!(
            data.ranks().iter().map(|rank| rank.id).collect::<Vec<_>>(),
            [RankId(128), RankId(255)]
        );
    }

    #[test]
    fn each_readable_mïsn_is_a_mission_record_by_id() {
        use nova_data::records::mission::Mission;
        let data = store(&[
            (Mission::TYPE, 140, offered(b"b7 | !b8\0")),
            (Mission::TYPE, 129, offered(b"(b1\0")),
            (Mission::TYPE, 131, short(offered(b"\0"))),
        ]);
        let missions = data.missions();
        assert_eq!(
            missions
                .iter()
                .map(|mission| mission.id)
                .collect::<Vec<_>>(),
            [MissionId(129), MissionId(140)],
            "by ID, the undecodable one skipped"
        );
        assert_eq!(
            missions[1],
            MissionRecord {
                id: MissionId(140),
                avail_stel: 10_005,
                avail_loc: 0,
                avail_record: -5,
                avail_rating: 40,
                avail_random: 25,
                avail_ship_type: 1129,
                avail_bits: Test::parse("b7 | !b8"),
                require: 0x0000_0004_0000_0001,
                flags: 0x2008,
                flags2: 0x0001,
                travel_stel: -2,
                return_stel: -1,
                cargo_type: 1000,
                cargo_qty: -10,
                pickup_mode: 1,
                drop_off_mode: -1,
                scan_mask: 0x8020,
                pay_val: -50_100,
                time_limit: 3,
                disp_weight: 60,
                on_accept: Script::parse("b10"),
                on_refuse: Script::parse("b11"),
                on_success: Script::parse("b12"),
                on_failure: Script::parse("b13"),
                on_abort: Script::parse("b14"),
                on_ship_done: Script::parse("b15"),
                can_abort: 0,
                comp_govt: Some(GovtId(383)),
                comp_reward: 7,
                date_post_inc: 4,
                ships: MissionShip {
                    count: 2,
                    goal: 1,
                    syst: -5,
                    dude: Some(DudeId(150)),
                    behav: 2,
                    start: -3,
                    name_id: Some(7000),
                    subtitle: Some(7001),
                },
                aux: AuxShip {
                    count: 4,
                    dude: Some(DudeId(151)),
                    syst: -3,
                },
            }
        );
        assert!(
            !missions[0].avail_bits.holds(|_| true),
            "an AvailBits that does not parse never holds"
        );
        assert_eq!(store(&[]).missions(), []);
        for (a, b) in data.missions().iter().zip(&data.missions()) {
            assert!(
                shared(&a.avail_bits, &b.avail_bits),
                "parsed once: {:?}",
                a.id
            );
        }
    }

    /// A `dësc` of `text`, with no picture.
    fn desc(text: &str) -> Vec<u8> {
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(0);
        bytes.extend([0xFF; 2]);
        bytes.extend([0; 34]);
        bytes
    }

    /// A `mïsn` whose `BriefText`, `QuickBrief`, `LoadCargText`,
    /// `DumpCargoText`, `CompText`, `FailText`, `RefuseText` and
    /// `ShipDoneText` are `texts`, in that order, with these button
    /// labels.
    fn texted(texts: [i16; 8], accept: &[u8], refuse: &[u8]) -> Vec<u8> {
        use nova_data::records::mission::Mission;
        let mut bytes = vec![0; Mission::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x34, &texts[..6]);
        put_i16s(&mut bytes, 0x44, &texts[7..]);
        put_i16s(&mut bytes, 0x58, &texts[6..7]);
        bytes[0x75F..0x75F + accept.len()].copy_from_slice(accept);
        bytes[0x77F..0x77F + refuse.len()].copy_from_slice(refuse);
        bytes
    }

    #[test]
    fn each_readable_mïsns_texts_are_its_name_its_offer_and_its_dëscs() {
        use nova_data::records::mission::Mission;
        let data = store_named(&[
            (
                Mission::TYPE,
                130,
                Some("Delivery to <DST>; Vellos1 "),
                texted(
                    [200, 201, 206, 207, 202, 203, 204, 209],
                    b"I'm in\0",
                    b"No way\0",
                ),
            ),
            (
                Mission::TYPE,
                131,
                Some("Plain run  "),
                texted([-1; 8], b"\0", b"\0"),
            ),
            (
                Mission::TYPE,
                132,
                None,
                texted([205, 205, 205, 208, 205, 205, 205, 0], b" ok\0", b"\0"),
            ),
            (
                Mission::TYPE,
                133,
                Some("Short"),
                short(texted([-1; 8], b"\0", b"\0")),
            ),
            (Desc::TYPE, 4002, None, desc("Offer <DST>.")),
            (Desc::TYPE, 200, None, desc("Brief.")),
            (Desc::TYPE, 201, None, desc("Quick <DL>.")),
            (Desc::TYPE, 202, None, desc("Comp.")),
            (Desc::TYPE, 203, None, desc("Fail.")),
            (Desc::TYPE, 204, None, desc("Refuse.")),
            (Desc::TYPE, 206, None, desc("Load <CQ>.")),
            (Desc::TYPE, 207, None, desc("Dump.")),
            (Desc::TYPE, 209, None, desc("Done <SN>.")),
        ]);
        let texts = data.mission_texts();
        assert_eq!(
            texts,
            [
                MissionTexts {
                    id: MissionId(130),
                    name: "Delivery to <DST>".to_owned(),
                    offer: Some("Offer <DST>.".to_owned()),
                    brief: Some("Brief.".to_owned()),
                    quick_brief: Some("Quick <DL>.".to_owned()),
                    comp: Some("Comp.".to_owned()),
                    fail: Some("Fail.".to_owned()),
                    refuse: Some("Refuse.".to_owned()),
                    load_cargo: Some("Load <CQ>.".to_owned()),
                    dump_cargo: Some("Dump.".to_owned()),
                    ship_done: Some("Done <SN>.".to_owned()),
                    accept_button: "I'm in".to_owned(),
                    refuse_button: "No way".to_owned(),
                },
                MissionTexts::named(MissionId(131), "Plain run"),
                MissionTexts {
                    accept_button: " ok".to_owned(),
                    ..MissionTexts::named(MissionId(132), "")
                },
            ],
            "-1 and a missing dësc (0 among them) are none; the undecodable mïsn is skipped"
        );
        assert_eq!(store(&[]).mission_texts(), []);
    }

    #[test]
    fn a_borrowed_or_shared_game_data_forwards_the_mission_texts() {
        use nova_data::records::mission::Mission;
        let data = std::rc::Rc::new(store_named(&[(
            Mission::TYPE,
            128,
            Some("Run"),
            texted([-1; 8], b"\0", b"\0"),
        )]));
        let texts = vec![MissionTexts::named(MissionId(128), "Run")];
        assert_eq!(PilotCatalog::mission_texts(&data), texts, "Rc");
        assert_eq!(PilotCatalog::mission_texts(&&*data), texts, "&");
    }

    #[test]
    fn a_resource_name_is_shown_up_to_its_last_semicolon_trailing_spaces_and_semicolons_dropped() {
        assert_eq!(
            shown_resource_name("Delivery to <DST>; Vellos1 "),
            "Delivery to <DST>"
        );
        assert_eq!(shown_resource_name("No cut  "), "No cut");
        assert_eq!(shown_resource_name("a;b;c"), "a;b");
        assert_eq!(shown_resource_name("x;; ;"), "x");
    }

    #[test]
    fn a_comp_govt_outside_128_to_383_is_none_and_rewards_nothing() {
        use nova_data::records::mission::Mission;
        let data = store(&[
            (Mission::TYPE, 128, rewarding(128)),
            (Mission::TYPE, 129, rewarding(127)),
            (Mission::TYPE, 130, rewarding(384)),
            (Mission::TYPE, 131, rewarding(-1)),
        ]);
        let rewards: Vec<_> = data
            .missions()
            .iter()
            .map(|mission| (mission.comp_govt, mission.comp_reward))
            .collect();
        assert_eq!(
            rewards,
            [(Some(GovtId(128)), 7), (None, 0), (None, 0), (None, 0)],
            "an index of 0-255 (@0xa0c04-0xa0c6e)"
        );
    }

    #[test]
    fn a_systems_person_slots_are_each_person_with_its_chance() {
        let mut bytes = trafficked([-1; 8], [0; 8], 3);
        put_i16s(&mut bytes, 0x6E, &[510, -1, 600, -1, -1, -1, 127, -1]);
        put_i16s(&mut bytes, 0x7E, &[50, 0, 100, 0, 0, 0, 0, 7]);
        let data = store(&[(System::TYPE, 128, bytes)]);
        let traffic = data.system_traffic(SystemId(128)).expect("readable");
        assert_eq!(
            traffic.persons,
            [
                (Some(PersonId(510)), 50),
                (None, 0),
                (Some(PersonId(600)), 100),
                (None, 0),
                (None, 0),
                (None, 0),
                (None, 0),
                (None, 7),
            ],
            "-1 and 127 name none"
        );
    }

    #[test]
    fn a_blink_is_its_shäns_blink_fields() {
        let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x36, &[2, -3, 75, 31, 85]);
        let shan = nova_data::decode_bytes::<ShipAnim>(&bytes)
            .expect("decodes")
            .record;
        assert_eq!(
            Blink::from(&shan),
            Blink {
                mode: 2,
                a: -3,
                b: 75,
                c: 31,
                d: 85,
            }
        );
    }

    #[test]
    fn a_ships_blink_is_its_shäns_and_steady_without_a_readable_one() {
        let mut shan = vec![0; ShipAnim::SIZE.expect("fixed")];
        put_i16s(&mut shan, 0x36, &[1, 4, 1, 2, 20]);
        let data = store(&[
            (ShipAnim::TYPE, 128, shan.clone()),
            (ShipAnim::TYPE, 129, short(shan)),
        ]);
        assert_eq!(
            ship_blink(&data, 128),
            Blink {
                mode: 1,
                a: 4,
                b: 1,
                c: 2,
                d: 20,
            }
        );
        assert_eq!(ship_blink(&data, 129), Blink::STEADY, "undecodable");
        assert_eq!(ship_blink(&data, 130), Blink::STEADY, "no shän");
    }
}
