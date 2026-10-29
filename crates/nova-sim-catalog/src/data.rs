//! The pilot, traffic, combat, asteroid and comm catalogs over the game
//! data: a thin mapping from `GameData`'s `chär`, `shïp`, `shän`, `oütf`,
//! `wëap`, `sÿst`, `spöb`, `jünk`, `öops`, `düde`, `flët`, `gövt`,
//! `mïsn` and `röid` records, its string lists and its stellar and
//! asteroid sprites; and a `shän`'s blink fields as a [`Blink`].

use std::collections::{BTreeMap, BTreeSet};

use nova_data::records::boom::Boom;
use nova_data::records::character::Character;
use nova_data::records::cron::Cron;
use nova_data::records::desc::Desc;
use nova_data::records::disaster::Disaster;
use nova_data::records::dude::Dude;
use nova_data::records::fleet::Fleet;
use nova_data::records::govt::Govt;
use nova_data::records::junk::Junk;
use nova_data::records::mission::Mission;
use nova_data::records::nebula::Nebula;
use nova_data::records::outfit::Outfit;
use nova_data::records::person::Person;
use nova_data::records::rank::Rank;
use nova_data::records::roid::Roid;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::string::StrResource;
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;
use nova_data::records::weapon::Weapon;
use nova_data::sound::decode_snd;
use nova_data::{GameData, Record, shot_spin_id, stellar_spin_id};
use nova_rsrc::ResType;

use crate::blink::Blink;
use crate::buoy::BUOY_STRINGS;
use crate::catalog::{
    AsteroidCatalog, AsteroidFieldRecord, AsteroidTypeRecord, AuxShip, BOX_GRAPHICS, BoomId,
    CharacterStart, CombatCatalog, CommCatalog, CommodityStrings, CronId, CronRecord, DateAffixes,
    DescMovie, DisasterId, DisasterRecord, DudeId, DudeRecord, EscortClass, EscortRecord,
    Explosion, FIRST_NEBULA, FleetId, FleetRecord, FrameSize, GateSite, GovtId, GovtRecord,
    HullRecord, JunkRecord, KINDS, LAST_NEBULA, LandingSite, ListedStellar, MissionId,
    MissionRecord, MissionShip, MissionTexts, NebulaId, NebulaRecord, OutfitId, OutfitRecord,
    Penalties, PersonId, PersonRecord, PersonWeapon, PilotCatalog, PlaceNames, RankId, RankRecord,
    ShipDisplay, ShipExits, ShipId, ShipRecord, SiteAnimation, SoundId, StarSystem, StartDate,
    StartError, StellarId, StellarLife, StellarMark, StellarScripts, StockWeapon, StringPatch,
    SystemId, SystemReinforcements, SystemTraffic, SystemVisibility, TextGraphics, TextMovies,
    TrafficCatalog, WeaponId, WeaponRecord, explosion_sound, loaded_mod_val,
};
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::resource_name::trim_resource_name;
use crate::voice::Voices;
use crate::warp::SoundLength;

/// The `spïn` of asteroid type 0's master sprite: type `n`'s is 800 +
/// `n` (`_LoadSprites` @0x20bb7).
pub const FIRST_ASTEROID_SPIN: i16 = 800;

/// The `spïn` of box graphic 0's master sprite: graphic `n`'s is 500 +
/// `n`, the box masters `_InitSprites` loads (500-504).
pub const FIRST_BOX_SPIN: i16 = 500;

/// The first `röid`, asteroid type 0's.
const FIRST_ROID: i16 = 128;

/// The `snd ` resource type.
const SND: ResType = ResType::new(*b"snd ");
/// The warp-up sound, `snd ` 128, whose length the warp's clock reads
/// (`_LoadSounds` @0x1c0b5-0x1c12d).
const WARP_UP_SOUND: i16 = 128;
/// The double-time warp-up sound, `snd ` 129 "Warp up x2", whose length
/// the warp's clock reads in double time (`_LoadSounds`
/// @0x1c0b5-0x1c12d, `_WarpSoundDuration` @0x8210-0x8219).
const WARP_UP_DOUBLE_SOUND: i16 = 129;
/// The warp-out sound, `snd ` 130, which an arrival plays (`_LoadSounds`
/// @0x1c152, `_HandlePlayer` @0x6bc08 and @0x6c706).
const WARP_OUT_SOUND: i16 = 130;
/// The first and last `snd ` of the escorts' voices: set 0's first
/// acknowledgement and set 7's last victory (see the
/// [`voice`](crate::voice) module).
const FIRST_VOICE_CLIP: i16 = 1000;
const LAST_VOICE_CLIP: i16 = 1799;

/// The length of `data`'s `snd ` `id` as `_ParseSndHeader` gives it: its
/// frames and the whole part of its rate; none when it is missing or
/// cannot be decoded.
fn sound_length(data: &GameData, id: i16) -> Option<SoundLength> {
    let snd = data.resource(SND, id)?;
    let pcm = decode_snd(snd.resource.data()).ok()?;
    Some(SoundLength {
        frames: u32::try_from(pcm.frames()).unwrap_or(u32::MAX),
        rate: pcm.sample_rate().fixed() >> 16,
    })
}

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
                    default_slots: default_slots(record),
                    strength: record.strength,
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
                    flags: record.flags.bits(),
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
                    skill_var: record.skill_var,
                    subtitle: record.subtitle.as_str().to_owned(),
                    upgrade_to: record.upgrade_to.filter(|ship| ship.0 >= 128),
                    esc_upgrd_cost: i64::from(record.esc_upgrd_cost.max(0)),
                    esc_sell_value: escort_sell_value(record.esc_sell_value, record.cost),
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
                    scan_mask: record.scan_mask.bits(),
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
                    tech_level: stellar.tech_level,
                    special_tech,
                    govt: stellar.govt,
                    flags2: stellar.flags2.bits(),
                    destroyed: false,
                    dominated_at_run_time: false,
                    graphic: stellar.graphic_type,
                    dead_graphic: dead_frame_size.map(|_| stellar.dead_type),
                    dead_frame_size,
                    type_counts_frames: counts_frames(self, stellar.graphic_type),
                    dead_counts_frames: dead_frame_size
                        .is_none_or(|_| counts_frames(self, stellar.dead_type)),
                    exit_angle: stellar.cust_snd_id,
                    tribute: stellar.tribute,
                    gravity: stellar.gravity,
                    fee: stellar.fee,
                    animation: SiteAnimation {
                        delay: stellar.anim_delay,
                        frame0_bias: stellar.frame0_bias,
                        open_frame: stellar.cust_pic_id,
                        type_frames: frame_count(self, stellar.graphic_type),
                        dead_frames: dead_frame_size
                            .map_or(0, |_| frame_count(self, stellar.dead_type)),
                    },
                    defense_dude: stellar.defense_dude.filter(|dude| dude.0 >= 128),
                    def_count: stellar.def_count,
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
                    links: con_slots(system.con),
                    govt: system.govt,
                    interference: system.interference,
                    murk: system.murk,
                    message: system.message,
                    background: system.bkgnd_color,
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

    fn string_patch(&self, id: i16) -> StringPatch {
        string_patch(self, id)
    }

    fn buoy_strings(&self) -> Vec<String> {
        strings(self, BUOY_STRINGS)
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
                    abbrev: record.abbrev.as_str().to_owned(),
                    base_price: record.base_price,
                    sold_at: stellars(&record.sold_at),
                    bought_at: stellars(&record.bought_at),
                    buy_on: self.test_expr(record.buy_on.as_str()).into(),
                    sell_on: self.test_expr(record.sell_on.as_str()).into(),
                    flags: record.flags.bits(),
                    scan_mask: record.scan_mask.bits(),
                    lc_name: record.lc_name.as_str().to_owned(),
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
        let mut listed: BTreeSet<StellarId> = BTreeSet::new();
        for (_, system) in self.records::<System>() {
            let Ok(system) = system else {
                continue;
            };
            listed.extend(system.record.nav_def.into_iter().flatten());
        }
        listed
            .into_iter()
            .filter_map(|id| {
                let stellar = self.get::<Stellar>(id.0)?.ok()?.record;
                Some(GateSite {
                    id,
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

    /// No upper bound is checked here: a `Weapon` past the last `wëap`
    /// finds no record, and the session gives that stellar no gun.
    fn stellar_weapons(&self) -> Vec<(StellarId, WeaponId)> {
        self.records::<Stellar>()
            .filter_map(|(id, stellar)| {
                let weapon = stellar.ok()?.record.weapon?;
                (weapon.0 >= 128).then_some((StellarId(id), weapon))
            })
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

    fn stellar_scripts(&self) -> Vec<StellarScripts> {
        self.records::<Stellar>()
            .filter_map(|(id, stellar)| {
                let stellar = stellar.ok()?.record;
                Some(StellarScripts {
                    id: StellarId(id),
                    on_dominate: self.set_expr(stellar.on_dominate.as_str()).into(),
                    on_release: self.set_expr(stellar.on_release.as_str()).into(),
                })
            })
            .collect()
    }

    fn fallback_frame_size(&self) -> Option<(u32, u32)> {
        let sheet = self.spin_sheet(stellar_spin_id(0)?).ok()?.sheet;
        Some((sheet.frame_width(), sheet.frame_height()))
    }

    fn fallback_counts_frames(&self) -> bool {
        counts_frames(self, 0)
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

    /// One pass over every system's `nav_def`, reading each stellar but
    /// not its sprites, as [`GameData::landing_sites`] would for every
    /// system.
    fn first_stellars(&self) -> Vec<(SystemId, StellarId)> {
        self.records::<System>()
            .filter_map(|(id, system)| {
                let first = system
                    .ok()?
                    .record
                    .nav_def
                    .into_iter()
                    .flatten()
                    .find(|stellar| matches!(self.get::<Stellar>(stellar.0), Some(Ok(_))))?;
                Some((SystemId(id), first))
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
                    min_status: stellar.min_status,
                    tribute: stellar.tribute,
                    tech_level: stellar.tech_level,
                    def_count: stellar.def_count,
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
    /// `_LoadObjectData` trims it (@0x772ab-0x77308): up to its last ';',
    /// trailing spaces and ';'s dropped, empty when it has none. The name
    /// is kept whole: the original's 30-character field (@0x77310-0x7732a)
    /// is the core's to apply (`nova_sim::mission::StellarMarks`),
    /// by [`StellarIdentityRecord`](crate::RuleKey::StellarIdentityRecord).
    /// A `spöb` that can't be decoded is left out, as one that didn't load.
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
    /// Each text's `Graphic` beside it, none for -1 too, and the movie its
    /// `MovieFile` names, played as its `Flags` say ([`DescMovie::from_desc`]).
    fn mission_texts(&self) -> Vec<MissionTexts> {
        let text = |id: Option<i16>| -> Option<String> {
            let desc = self.get::<Desc>(id?)?.ok()?;
            Some(desc.record.text.as_str().to_owned())
        };
        let graphic = |id: Option<i16>| -> Option<i16> {
            let desc = self.get::<Desc>(id?)?.ok()?;
            Some(desc.record.graphic?.0)
        };
        let movie = |id: Option<i16>| -> Option<DescMovie> {
            let desc = self.get::<Desc>(id?)?.ok()?;
            DescMovie::from_desc(desc.record.movie_file.as_str(), desc.record.flags.0)
        };
        self.records::<Mission>()
            .filter_map(|(id, mission)| {
                let mission = mission.ok()?;
                let record = mission.record;
                let raw = |desc: Option<nova_data::DescId>| text(desc.map(|desc| desc.0));
                let pict = |desc: Option<nova_data::DescId>| graphic(desc.map(|desc| desc.0));
                let film = |desc: Option<nova_data::DescId>| movie(desc.map(|desc| desc.0));
                let offer = id
                    .checked_sub(FIRST_MISSION)
                    .and_then(|index| FIRST_OFFER_DESC.checked_add(index));
                Some(MissionTexts {
                    id: MissionId(id),
                    name: shown_resource_name(mission.name.unwrap_or_default()).to_owned(),
                    offer: text(offer),
                    brief: raw(record.brief_text),
                    quick_brief: raw(record.quick_brief),
                    comp: raw(record.comp_text),
                    fail: raw(record.fail_text),
                    refuse: raw(record.refuse_text),
                    refuse_named: record.refuse_text.is_some(),
                    load_cargo: raw(record.load_carg_text),
                    dump_cargo: raw(record.dump_cargo_text),
                    ship_done: raw(record.ship_done_text),
                    accept_button: record.accept_button.as_str().to_owned(),
                    refuse_button: record.refuse_button.as_str().to_owned(),
                    graphics: TextGraphics {
                        offer: graphic(offer),
                        brief: pict(record.brief_text),
                        comp: pict(record.comp_text),
                        fail: pict(record.fail_text),
                        refuse: pict(record.refuse_text),
                        load_cargo: pict(record.load_carg_text),
                        dump_cargo: pict(record.dump_cargo_text),
                        ship_done: pict(record.ship_done_text),
                    },
                    movies: TextMovies {
                        offer: movie(offer),
                        brief: film(record.brief_text),
                        quick_brief: film(record.quick_brief),
                        comp: film(record.comp_text),
                        fail: film(record.fail_text),
                        refuse: film(record.refuse_text),
                        load_cargo: film(record.load_carg_text),
                        dump_cargo: film(record.dump_cargo_text),
                        ship_done: film(record.ship_done_text),
                    },
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

    /// Only IDs 128-159 have a slot (@0x7cf44-0x7d025; see the
    /// `nova_sim::nebula` module).
    fn nebulae(&self) -> Vec<NebulaRecord> {
        self.records::<Nebula>()
            .filter(|(id, _)| (FIRST_NEBULA..=LAST_NEBULA).contains(id))
            .filter_map(|(id, record)| {
                let record = record.ok()?.record;
                Some(NebulaRecord {
                    id: NebulaId(id),
                    left: record.x_pos,
                    top: record.y_pos,
                    width: record.x_size,
                    height: record.y_size,
                    active_on: self.test_expr(record.active_on.as_str()).into(),
                    on_explore: self.set_expr(record.on_explore.as_str()).into(),
                })
            })
            .collect()
    }

    fn mission_dude(&self, id: DudeId) -> Option<DudeRecord> {
        TrafficCatalog::dude(self, id)
    }

    fn warp_sound(&self) -> Option<SoundLength> {
        sound_length(self, WARP_UP_SOUND)
    }

    fn warp_sound_double_time(&self) -> Option<SoundLength> {
        sound_length(self, WARP_UP_DOUBLE_SOUND)
    }

    fn warp_out_sound(&self) -> Option<SoundLength> {
        sound_length(self, WARP_OUT_SOUND)
    }

    fn boom_sounds(&self) -> Vec<(BoomId, SoundId)> {
        self.records::<Boom>()
            .filter_map(|(id, boom)| {
                let sound = explosion_sound(boom.ok()?.record.sound_index)?;
                Some((BoomId(id), sound))
            })
            .collect()
    }

    /// Every `snd ` 1000-1799 that exists, with its length when it can be
    /// decoded, as `_LoadSounds` finds them (`_ResourceExists`,
    /// @0x1c03f-0x1c0b3), and every readable `gövt`'s `VoiceType`.
    fn voices(&self) -> Voices {
        let sounds: Vec<(SoundId, Option<SoundLength>)> = (FIRST_VOICE_CLIP..=LAST_VOICE_CLIP)
            .filter(|&id| self.resource(SND, id).is_some())
            .map(|id| (SoundId(id), sound_length(self, id)))
            .collect();
        let govts: Vec<(GovtId, i16)> = self
            .records::<Govt>()
            .filter_map(|(id, govt)| Some((GovtId(id), govt.ok()?.record.voice_type)))
            .collect();
        Voices::read(&sounds, &govts)
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
    tiles(data, graphic).is_some_and(|tiles| tiles > 0)
}

/// Graphic type `graphic`'s frame count in `data`, its `spïn`'s `xTiles` ×
/// `yTiles` as [`counts_frames`] reckons it: 0 when it does not read or is
/// not above 0, and at most `u16::MAX`.
fn frame_count(data: &GameData, graphic: i16) -> u16 {
    stellar_spin_id(graphic).map_or(0, |spin_id| spin_frame_count(data, spin_id))
}

/// `spïn` `spin_id`'s frame count in `data`, its `xTiles` × `yTiles` in
/// 32 bits: 0 when it does not read or is not above 0, and at most
/// `u16::MAX`.
fn spin_frame_count(data: &GameData, spin_id: i16) -> u16 {
    spin_tiles(data, spin_id).map_or(0, |tiles| u16::try_from(tiles.max(0)).unwrap_or(u16::MAX))
}

/// Graphic type `graphic`'s `spïn`'s `xTiles` × `yTiles` in 32 bits, when
/// it reads.
fn tiles(data: &GameData, graphic: i16) -> Option<i32> {
    spin_tiles(data, stellar_spin_id(graphic)?)
}

/// `spïn` `spin_id`'s `xTiles` × `yTiles` in 32 bits, when it reads.
fn spin_tiles(data: &GameData, spin_id: i16) -> Option<i32> {
    let spin = data.get::<Spin>(spin_id)?.ok()?;
    Some(i32::from(spin.record.x_tiles) * i32::from(spin.record.y_tiles))
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

    fn system_reinforcements(&self, id: SystemId) -> Option<SystemReinforcements> {
        let system = self.get::<System>(id.0)?.ok()?.record;
        // A `ReinfFleet` of -1 names none, as any of 127 or less does.
        SystemReinforcements::normalised(
            system.reinf_fleet.map_or(0, |fleet| fleet.0),
            system.reinf_time,
            system.reinf_intrval,
        )
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
                    quote: fleet.quote.map(|id| id.0).filter(|&quote| quote > 0),
                    flags: fleet.flags.0,
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

/// A resource name as the game shows a person's or a mission's: cut as
/// [`trim_resource_name`] cuts it (`_TrimResourceName` @0x717ac). A
/// mission's is kept whole, as the original keeps 255 characters.
fn shown_resource_name(name: &str) -> &str {
    trim_resource_name(name)
}

/// The most characters of a government's name the engine keeps (it
/// copies 31 bytes, a Pascal string's length byte among them, @0x7be51).
const GOVT_NAME_CHARS: usize = 30;

/// A `gövt` resource's name as the game shows it: cut as
/// [`shown_resource_name`] cuts it (`_LoadObjectData` @0x7bde9-0x7be2a),
/// and at most [`GOVT_NAME_CHARS`] characters.
fn govt_name(name: &str) -> String {
    shown_resource_name(name)
        .chars()
        .take(GOVT_NAME_CHARS)
        .collect()
}

/// The frame size of the sprite a shot of `Graphic` `graphic` is drawn
/// with in `data`, its `spïn` as the original clones it
/// ([`shot_spin_id`]): none for a `Graphic` below 0, a missing `spïn` or a
/// sheet that cannot be read.
fn shot_frame(data: &GameData, graphic: i16) -> Option<FrameSize> {
    let sheet = data.spin_sheet(shot_spin_id(graphic)?).ok()?.sheet;
    Some(FrameSize {
        width: sheet.frame_width(),
        height: sheet.frame_height(),
    })
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
                    graphic: record.graphic,
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
                    beam_width: record.beam_width,
                    falloff: record.falloff,
                    li_density: record.li_density,
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
                    recoil: record.recoil,
                    exit_type: record.exit_type,
                    ionization: record.ionization,
                    ionize_color: record.ionize_color,
                    jam_vuln: record.jam_vuln,
                    frame: shot_frame(self, record.graphic),
                    hit_particles: record.hit_particles,
                    hit_part_life: record.hit_part_life,
                    hit_part_vel: record.hit_part_vel,
                    hit_part_color: record.hit_part_color,
                })
            })
            .collect()
    }

    fn hulls(&self) -> Vec<HullRecord> {
        self.records::<Ship>()
            .filter_map(|(id, ship)| {
                let record = ship.ok()?.record;
                let (size, height, exits, display) = match self.get::<ShipAnim>(id) {
                    Some(Ok(anim)) => (
                        Some(anim.record.base_x_size),
                        Some(anim.record.base_y_size),
                        Some(ship_exits(anim.record)),
                        Some(ship_display(anim.record)),
                    ),
                    _ => (None, None, None, None),
                };
                Some(HullRecord {
                    id: ShipId(id),
                    flags: record.flags.bits(),
                    flags2: record.flags2.bits(),
                    flags3: record.flags3.bits(),
                    death_delay: record.death_delay,
                    pod_count: record.pod_count,
                    explode1: record.explode1,
                    explode2: record.explode2,
                    mass: record.mass,
                    weapons: stock_weapons(record),
                    size,
                    height,
                    strength: record.strength,
                    key_carried: record.key_carried,
                    cost: record.cost,
                    speed: record.speed,
                    exits,
                    display,
                    escort_class: EscortClass::of(
                        record.escort_type,
                        record.inherent_ai,
                        record.mass,
                    ),
                })
            })
            .collect()
    }

    fn governments(&self) -> Vec<GovtRecord> {
        self.records::<Govt>()
            .filter_map(|(id, govt)| {
                let govt = govt.ok()?;
                let record = govt.record;
                Some(GovtRecord {
                    id: GovtId(id),
                    name: govt_name(govt.name.unwrap_or_default()),
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
                    medium_name: record.medium_name.as_str().to_owned(),
                    scan_mask: record.scan_mask.bits(),
                    scan_fine: record.scan_fine,
                    news_pic: record.news_pic,
                    require: record.require.bits(),
                    inh_jam: record.inh_jam,
                    skill_mult: record.skill_mult,
                })
            })
            .collect()
    }
}

/// The `l33t` resource type, whose presence alone turns on `_leetMode`.
const LEET: ResType = ResType::new(*b"l33t");
/// Reads the records afresh on every call; a session asks once, when it
/// starts.
impl AsteroidCatalog for GameData {
    fn asteroid_types(&self) -> Vec<AsteroidTypeRecord> {
        (0..KINDS as u8)
            .filter_map(|kind| {
                let roid = self.get::<Roid>(FIRST_ROID + i16::from(kind))?.ok()?.record;
                let spin_id = FIRST_ASTEROID_SPIN + i16::from(kind);
                let frame_size = self
                    .spin_sheet(spin_id)
                    .ok()
                    .map(|spin| (spin.sheet.frame_width(), spin.sheet.frame_height()));
                Some(AsteroidTypeRecord {
                    kind,
                    strength: roid.strength,
                    spin_rate: roid.spin_rate,
                    frames: spin_frame_count(self, spin_id),
                    frame_size,
                    yield_type: roid.yield_type,
                    yield_qty: roid.yield_qty,
                    part_count: roid.part_count,
                    part_color: roid.part_color,
                    frag_types: [roid.frag_type1, roid.frag_type2]
                        .map(|frag| frag.map_or(-1, |id| id.0)),
                    frag_count: roid.frag_count,
                    explode_type: roid.explode_type,
                    mass: roid.mass,
                })
            })
            .collect()
    }

    fn box_frame_sizes(&self) -> [Option<(u32, u32)>; BOX_GRAPHICS] {
        std::array::from_fn(|graphic| {
            // In 0..5.
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let spin_id = FIRST_BOX_SPIN + graphic as i16;
            self.spin_sheet(spin_id)
                .ok()
                .map(|spin| (spin.sheet.frame_width(), spin.sheet.frame_height()))
        })
    }

    fn asteroid_fields(&self) -> Vec<AsteroidFieldRecord> {
        self.records::<System>()
            .filter_map(|(id, system)| {
                let record = system.ok()?.record;
                Some(AsteroidFieldRecord {
                    system: SystemId(id),
                    count: record.asteroids,
                    types: record.ast_types.bits(),
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

    /// Whether any `l33t` resource exists, whatever it holds
    /// (`CountResources('l33t') > 0`, @0x7e28c-0x7e2a2).
    fn leet_mode(&self) -> bool {
        !self.ids(LEET).is_empty()
    }

    fn plugin_string(&self, id: i16) -> StringPatch {
        string_patch(self, id)
    }
}

/// The most a `shïp`'s `InherentGovt` adds to a `gövt` ID to say it
/// takes on that government's attributes only.
const ATTRIBUTES_ONLY: i16 = 1000;

/// What a `shïp`'s `InherentGovt` adds to a `gövt` ID to say it fights as
/// that government only.
const COMBAT_ONLY: i16 = 2000;

/// A `sÿst`'s `Con1-Con16` as [`StarSystem::links`] keeps them: each in
/// its slot, an empty one (-1) as -1, as `_LoadObjectData` keeps all 16
/// in place (@0x7806d-0x780b8), so a link's index is its slot number. The
/// empty slots after the last link are dropped, as they number nothing.
fn con_slots(con: [Option<SystemId>; 16]) -> Vec<SystemId> {
    let used = con
        .iter()
        .rposition(Option::is_some)
        .map_or(0, |last| last + 1);
    con[..used]
        .iter()
        .map(|slot| slot.unwrap_or(SystemId(-1)))
        .collect()
}

/// A `shïp`'s `EscSellValue` as `_LoadObjectData` reads it
/// (@0x7a42e-0x7a442): as it is when above 0, else its `Cost` x 0.1 (the
/// double at 0xdd110), truncated. The double 0.1 is a hair above a tenth,
/// so the truncated product is the integer tenth for every `Cost`.
fn escort_sell_value(raw: i32, cost: i32) -> i64 {
    if raw > 0 {
        i64::from(raw)
    } else {
        i64::from(cost) / 10
    }
}

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

/// A `shän`'s weapon exit points, compression and `FramesPer`, raw.
fn ship_exits(anim: &ShipAnim) -> ShipExits {
    let pairs = |xs: [i16; 4], ys: [i16; 4]| std::array::from_fn(|i| (xs[i], ys[i]));
    ShipExits {
        points: [
            pairs(anim.gun_pos_x, anim.gun_pos_y),
            pairs(anim.turret_pos_x, anim.turret_pos_y),
            pairs(anim.guided_pos_x, anim.guided_pos_y),
            pairs(anim.beam_pos_x, anim.beam_pos_y),
        ],
        z: [
            anim.gun_pos_z,
            anim.turret_pos_z,
            anim.guided_pos_z,
            anim.beam_pos_z,
        ],
        up_compress: (anim.up_compress_x, anim.up_compress_y),
        down_compress: (anim.dn_compress_x, anim.dn_compress_y),
        frames: anim.frames_per,
    }
}

/// A `shän`'s display fields, raw.
fn ship_display(anim: &ShipAnim) -> ShipDisplay {
    ShipDisplay {
        flags: anim.flags.bits(),
        base_sets: anim.base_set_count,
        anim_delay: anim.anim_delay,
        alt_sets: anim.alt_set_count,
        alt_image: anim.alt_image_id > 0,
        blink: blink_of(anim),
        lights_image: anim.light_image_id > 0,
    }
}

/// A `shän`'s `BlinkMode` and `BlinkValA`–`D`, raw.
fn blink_of(anim: &ShipAnim) -> Blink {
    Blink {
        mode: anim.blink_mode,
        a: anim.blink_val_a,
        b: anim.blink_val_b,
        c: anim.blink_val_c,
        d: anim.blink_val_d,
    }
}

/// Ship `id`'s blink: its `shän`'s blink fields, or steady when the `shän`
/// is missing or cannot be read.
#[must_use]
pub fn ship_blink(data: &GameData, id: i16) -> Blink {
    match data.get::<ShipAnim>(id) {
        Some(Ok(anim)) => blink_of(anim.record),
        _ => Blink::STEADY,
    }
}

/// A `shïp`'s handling, reserve, cargo and mass fields, and its flags.
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
        flags: ship.flags.bits(),
        flags2: ship.flags2.bits(),
        ionize_max: ship.ionize_max,
        deionize: ship.deionize,
    }
}

/// A `shïp`'s `DefaultItems`, each item with its count in slot order.
/// `ship`'s `DefaultItems` slot by slot, 1-8, each with its raw count;
/// none where a slot names no item.
fn default_slots(ship: &Ship) -> [Option<(OutfitId, i16)>; 8] {
    let items = ship
        .default_items1_4
        .into_iter()
        .chain(ship.default_items5_8);
    let counts = ship.item_count1_4.into_iter().chain(ship.item_count5_8);
    let mut slots = [None; 8];
    for (slot, (item, count)) in slots.iter_mut().zip(items.zip(counts)) {
        *slot = item.map(|item| (item, count));
    }
    slots
}

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

/// `data`'s six `'STR '` patches from `first` up, raw, each as
/// [`string_patch`] reads it.
fn patches(data: &GameData, first: i16) -> [StringPatch; 6] {
    std::array::from_fn(|n| string_patch(data, first + i16::try_from(n).expect("one of six")))
}

/// `data`'s `'STR '` `id`, raw, from whichever file holds it. A zero-byte
/// one is [`StringPatch::Empty`]; any other that cannot be read is
/// [`StringPatch::Unreadable`]. Bytes past a readable string are ignored,
/// as the original reads only as many as its length byte says.
fn string_patch(data: &GameData, id: i16) -> StringPatch {
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_data::graphics::RLED;
    use nova_data::graphics::fixture::RledBuilder;
    use nova_data::records::disaster::Disaster;
    use nova_data::records::fixture::ShipAnimBuilder;
    use nova_data::records::junk::Junk;
    use nova_data::records::roid::Roid;
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::records::spin::Spin;
    use nova_data::records::string_list::StrList;
    use nova_data::records::weapon::Weapon;
    use nova_data::store::fixture::{PLUGINS_DIR, one_file};
    use nova_rsrc::ResType;
    use nova_rsrc::fixture::ForkBuilder;

    use super::*;
    use crate::catalog::{
        AsteroidCatalog, AsteroidFieldRecord, AsteroidTypeRecord, DisasterId, FrameSize, GateSite,
        GovtId, GovtRecord, HullRecord, JunkId, Penalties, ShipDisplay, ShipExits, StarSystem,
        StartDate, StellarId, StockWeapon, SystemVisibility, WeaponId, WeaponRecord,
    };
    use crate::control::{Script, Test, TestExpr};
    use crate::test_store::{put_i16s, store, store_named};

    /// Whether two tests are the one parse, shared.
    fn shared(a: &Test, b: &Test) -> bool {
        std::sync::Arc::ptr_eq(&a.0, &b.0)
    }

    /// Whether two scripts are the one parse, shared.
    fn shared_script(a: &Script, b: &Script) -> bool {
        std::sync::Arc::ptr_eq(&a.0, &b.0)
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
    fn a_ships_fields_include_its_flags() {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x4A, &[0x0018]);
        let data = store(&[(Ship::TYPE, 128, bytes)]);
        assert_eq!(data.ship_fields(ShipId(128)).map(|f| f.flags), Ok(0x0018));
    }

    #[test]
    fn a_ships_fields_include_its_flags2() {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x62, &[0x4021]);
        let data = store(&[(Ship::TYPE, 128, bytes)]);
        assert_eq!(data.ship_fields(ShipId(128)).map(|f| f.flags2), Ok(0x4021));
    }

    #[test]
    fn a_ships_fields_include_its_deionize_and_ionize_max() {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x36A, &[-5, 400]);
        let data = store(&[(Ship::TYPE, 128, bytes)]);
        let fields = data.ship_fields(ShipId(128)).expect("decodes");
        assert_eq!((fields.deionize, fields.ionize_max), (-5, 400));
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
        put_i16s(&mut bytes, 0x46, &[-7]);
        put_i16s(&mut bytes, 0x48, &[1129]);
        bytes[0x4A..0x4C].copy_from_slice(&0x8012_u16.to_be_bytes());
        bytes[0x60E..0x61D].copy_from_slice(b"heavy shuttle\0\0");
        put_i16s(&mut bytes, 0x732, &[2]);
        put_i16s(&mut bytes, 0x60, &[15]);
        bytes[0x26A..0x26E].copy_from_slice(b"b300");
        bytes[0x3D0..0x3D5].copy_from_slice(b"^b301");
        bytes[0x4CF..0x4D3].copy_from_slice(b"!b30");
        bytes[0x6E6..0x6F5].copy_from_slice(b"Light Freighter");
        put_i16s(&mut bytes, 0x728, &[129]);
        bytes[0x72A..0x72E].copy_from_slice(&6_000_i32.to_be_bytes());
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
                flags: 0x8012,
                ..ShipFields::default()
            },
            defaults: vec![(OutfitId(200), 2), (OutfitId(201), 0), (OutfitId(202), 1)],
            default_slots: [
                Some((OutfitId(200), 2)),
                Some((OutfitId(201), -1)),
                None,
                None,
                Some((OutfitId(202), 1)),
                None,
                None,
                None,
            ],
            strength: -7,
            cost: 17_500,
            tech_level: 6,
            buy_random: 45,
            hire_random: 40,
            require: 0x0000_0002_0000_0001,
            availability: Test::parse("b422"),
            appear_on: Test::parse("!b17"),
            flags: 0x8012,
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
            skill_var: 15,
            subtitle: "Light Freighter".to_owned(),
            upgrade_to: Some(ShipId(129)),
            esc_upgrd_cost: 6_000,
            esc_sell_value: 1_750,
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
    fn a_shïps_escort_upgrade_and_sale_read_as_load_object_data_reads_them() {
        let escort = |upgrade_to: i16, cost: i32, sell: i32, price: i32| {
            let mut bytes = for_sale();
            bytes[0x30..0x34].copy_from_slice(&price.to_be_bytes());
            put_i16s(&mut bytes, 0x728, &[upgrade_to]);
            bytes[0x72A..0x72E].copy_from_slice(&cost.to_be_bytes());
            bytes[0x72E..0x732].copy_from_slice(&sell.to_be_bytes());
            let ship = store(&[(Ship::TYPE, 128, bytes)]).ships().remove(0);
            (ship.upgrade_to, ship.esc_upgrd_cost, ship.esc_sell_value)
        };
        assert_eq!(
            escort(127, 10, 750, 4_995),
            (None, 10, 750),
            "an UpgradeTo below 128 is none"
        );
        assert_eq!(
            escort(128, -5, 0, 4_995),
            (Some(ShipId(128)), 0, 499),
            "a negative cost is 0; a sale of 0 is the Cost x 0.1, truncated"
        );
        assert_eq!(escort(-1, 0, -1, 4_999), (None, 0, 499), "UpgradeTo -1");
        assert_eq!(escort(0, 0, 1, 4_999), (None, 0, 1), "UpgradeTo 0");
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
            scan_mask: 0,
            on_purchase: Script::parse("b400"),
            on_sell: Script::parse("!b400"),
        };
        assert_eq!(
            data.outfits(),
            [
                record(200, "Big Gun", [(1, 128), (0, 0), (0, 0), (0, 0)]),
                record(201, "big gun", [(2, 5), (9, -2), (0, 0), (18, 4)]),
            ],
            "a resource without a name goes by its LCName; its Abbrev is kept; an undecodable one is skipped"
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
    fn an_oütfs_scan_mask_is_read_raw() {
        let mut bytes = outfit([(0, 0); 4]);
        put_i16s(&mut bytes, 0x3EE, &[0x8004_u16.cast_signed()]);
        let data = store(&[(Outfit::TYPE, 128, bytes)]);
        assert_eq!(data.outfits()[0].scan_mask, 0x8004);
    }

    /// `snd ` 128 of `samples` 8-bit mono samples at 22254.54 Hz.
    fn warp_up(samples: usize) -> (ResType, i16, Vec<u8>) {
        let (ty, _, bytes) = warp_up_as(129, samples);
        (ty, 128, bytes)
    }

    /// `snd ` `id` of `samples` 8-bit mono samples at 22254.54 Hz.
    fn warp_up_as(id: i16, samples: usize) -> (ResType, i16, Vec<u8>) {
        use nova_data::sound::fixture::{Header, SndBuilder, SndFormat};
        let snd = SndBuilder::new(
            SndFormat::One,
            Header::Standard {
                rate: 0x56EE_8BA3,
                loop_points: (0, 0),
                base_note: 60,
                samples: vec![0x80; samples],
            },
        );
        (ResType::new(*b"snd "), id, snd.bytes())
    }

    #[test]
    fn the_warp_sound_is_snd_128s_frames_and_the_whole_part_of_its_rate() {
        let data = store(&[warp_up(44_508)]);
        assert_eq!(
            data.warp_sound(),
            Some(SoundLength {
                frames: 44_508,
                rate: 22_254
            })
        );
        assert_eq!(
            crate::warp::sound_duration(data.warp_sound()),
            120,
            "two seconds at 22254 Hz, where 22255 would give 119"
        );
    }

    #[test]
    fn without_a_readable_snd_128_there_is_no_warp_sound() {
        assert_eq!(store(&[]).warp_sound(), None);
        let (ty, _, bytes) = warp_up(44_508);
        assert_eq!(
            store(&[(ty, 129, bytes.clone())]).warp_sound(),
            None,
            "129 is not it"
        );
        let broken = bytes[..bytes.len() - 10].to_vec();
        assert_eq!(
            store(&[(ty, 128, broken)]).warp_sound(),
            None,
            "undecodable"
        );
    }

    #[test]
    fn the_double_time_warp_sound_is_snd_129s() {
        let data = store(&[warp_up(1000), warp_up_as(129, 44_508)]);
        assert_eq!(
            data.warp_sound_double_time(),
            Some(SoundLength {
                frames: 44_508,
                rate: 22_254
            })
        );
        assert_eq!(
            data.warp_sound().map(|sound| sound.frames),
            Some(1000),
            "128 apart"
        );
    }

    #[test]
    fn the_warp_out_sound_is_snd_130s() {
        let data = store(&[warp_up(1000), warp_up_as(130, 44_508)]);
        assert_eq!(
            data.warp_out_sound(),
            Some(SoundLength {
                frames: 44_508,
                rate: 22_254
            })
        );
        assert_eq!(
            data.warp_sound().map(|sound| sound.frames),
            Some(1000),
            "128 apart"
        );
    }

    #[test]
    fn without_a_readable_snd_130_there_is_no_warp_out_sound() {
        assert_eq!(store(&[]).warp_out_sound(), None);
        assert_eq!(
            store(&[warp_up(44_508), warp_up_as(129, 44_508)]).warp_out_sound(),
            None,
            "128 and 129 are not it"
        );
        let (ty, id, bytes) = warp_up_as(130, 44_508);
        let broken = bytes[..bytes.len() - 10].to_vec();
        assert_eq!(
            store(&[(ty, id, broken)]).warp_out_sound(),
            None,
            "undecodable"
        );
    }

    #[test]
    fn without_a_readable_snd_129_there_is_no_double_time_warp_sound() {
        assert_eq!(store(&[]).warp_sound_double_time(), None);
        assert_eq!(
            store(&[warp_up(44_508)]).warp_sound_double_time(),
            None,
            "128 is not it"
        );
        let (ty, id, bytes) = warp_up_as(129, 44_508);
        let broken = bytes[..bytes.len() - 10].to_vec();
        assert_eq!(
            store(&[(ty, id, broken)]).warp_sound_double_time(),
            None,
            "undecodable"
        );
    }

    #[test]
    fn the_voices_are_each_govts_voice_type_and_the_clips_from_snd_1000() {
        use crate::voice::{GovtVoice, VoiceMessage, VoiceVariant};
        let mut federation = govt_bytes();
        put_i16s(&mut federation, 0x00, &[1003]);
        let mut silent = govt_bytes();
        put_i16s(&mut silent, 0x00, &[-1]);
        let (ty, _, bytes) = warp_up_as(1001, 44_508);
        let broken = (ty, 1001, bytes[..bytes.len() - 10].to_vec());
        let data = store(&[
            (Govt::TYPE, 128, federation),
            (Govt::TYPE, 129, silent),
            warp_up_as(1000, 44_508),
            broken,
            warp_up_as(1002, 22_254),
            warp_up_as(1799, 22_254),
            warp_up_as(1800, 22_254),
        ]);
        let voices = data.voices();
        assert_eq!(
            voices.govts.get(&GovtId(128)),
            Some(&GovtVoice {
                set: Some(3),
                forced: Some(VoiceVariant::Odd)
            })
        );
        assert_eq!(voices.govts.get(&GovtId(129)), Some(&GovtVoice::default()));
        assert_eq!(voices.table.count(0, VoiceMessage::Acknowledge), 3);
        assert_eq!(
            voices.table.count(7, VoiceMessage::Victory),
            0,
            "1799 alone, not from its base"
        );
        let clip = |draw| {
            voices
                .table
                .pick(
                    0,
                    VoiceMessage::Acknowledge,
                    None,
                    &mut nova_sim_base::fixture::Draws::of(&[draw]),
                )
                .map(|clip| (clip.sound, clip.length))
        };
        assert_eq!(clip(0), Some((SoundId(1000), 120)), "two seconds");
        assert_eq!(clip(1), Some((SoundId(1001), 0)), "undecodable");
        assert_eq!(clip(2), Some((SoundId(1002), 60)));
    }

    #[test]
    fn a_catalog_of_no_snd_1000_has_no_voice_clips() {
        use crate::voice::VoiceMessage;
        let voices = store(&[warp_up(44_508)]).voices();
        assert_eq!(voices.table.count(0, VoiceMessage::Acknowledge), 0);
        assert!(voices.govts.is_empty());
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

    /// `bytes`, a `spöb`, with this `Tribute`.
    fn paying(mut bytes: Vec<u8>, tribute: i16) -> Vec<u8> {
        put_i16s(&mut bytes, 0x0A, &[tribute]);
        bytes
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
    fn the_gate_sites_are_every_listed_stellar_by_id() {
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
                    position: Vec2::new(5.0, 6.0),
                    flags2: 0,
                    // Its zeroed links, kept raw.
                    links: [Some(StellarId(0)); 8],
                    exit_angle: 0,
                },
                GateSite {
                    id: StellarId(129),
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
        let mut governed = stellar(5, 6, 0, 0x21, 25);
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
                    min_status: 25,
                    tribute: 0,
                    def_count: 0,
                    tech_level: 0,
                },
                ListedStellar {
                    id: StellarId(129),
                    systems: vec![SystemId(130), SystemId(131)],
                    flags: 0x21,
                    flags2: 0x2000,
                    govt: None,
                    min_status: 0,
                    tribute: 0,
                    def_count: 0,
                    tech_level: 0,
                },
            ],
            "by ID, each system once by ID; unlisted 130, missing 999, and 133 listed only \
             by an undecodable system left out"
        );
        assert_eq!(store(&[]).listed_stellars(), []);
    }

    #[test]
    fn each_systems_first_stellar_is_the_first_of_its_landing_sites() {
        let data = store(&[
            (System::TYPE, 130, system_with(&[999, 129, 128])),
            (System::TYPE, 131, system_with(&[])),
            (System::TYPE, 132, system_with(&[129, 128, 130])),
            (System::TYPE, 133, short(system_with(&[128]))),
            (Stellar::TYPE, 128, stellar(0, 0, 0, 1, 0)),
            (Stellar::TYPE, 129, short(stellar(0, 0, 0, 1, 0))),
            (Stellar::TYPE, 130, stellar(0, 0, 0, 1, 0)),
        ]);
        let first = data.first_stellars();
        assert_eq!(
            first,
            [
                (SystemId(130), StellarId(128)),
                (SystemId(132), StellarId(128))
            ],
            "missing 999 and undecodable 129 passed over; 131 lists none and \
             undecodable 133 is none"
        );
        let landing: Vec<_> = data
            .star_map()
            .iter()
            .filter_map(|system| Some((system.id, data.landing_sites(system.id).first()?.id)))
            .collect();
        assert_eq!(first, landing, "as the landing sites give it");
        assert_eq!(store(&[]).first_stellars(), []);
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
    fn stellar_marks_keep_a_name_over_30_characters_whole() {
        let name = "A Stellar Named Well Past Thirty Chars";
        let data = store_named(&[(Stellar::TYPE, 128, Some(name), stellar(1, 2, 0, 1, 0))]);
        assert_eq!(
            data.stellar_marks(),
            [StellarMark {
                id: StellarId(128),
                place: (1, 2),
                name: name.to_owned(),
            }]
        );
    }

    #[test]
    fn a_systems_landing_sites_are_its_readable_stellars_in_nav_order() {
        let data = store(&[
            (System::TYPE, 130, system_with(&[129, 999, 128, 131])),
            (
                Stellar::TYPE,
                128,
                paying(stellar(-300, 450, 7, 0x0000_0013, 25), 250),
            ),
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
                    tech_level: 0,
                    special_tech: [0; 8],
                    govt: None,
                    flags2: 0x1200,
                    destroyed: false,
                    dominated_at_run_time: false,
                    graphic: 0,
                    dead_graphic: Some(0),
                    dead_frame_size: Some((12, 30)),
                    type_counts_frames: true,
                    dead_counts_frames: true,
                    exit_angle: 0,
                    tribute: 0,
                    gravity: 0,
                    fee: 0,
                    animation: SiteAnimation {
                        type_frames: 1,
                        dead_frames: 1,
                        ..SiteAnimation::default()
                    },
                    defense_dude: None,
                    def_count: 0,
                },
                LandingSite {
                    id: StellarId(128),
                    position: Vec2::new(-300.0, 450.0),
                    frame_size: None,
                    flags: 0x13,
                    min_status: 25,
                    tech_level: 0,
                    special_tech: [0; 8],
                    govt: None,
                    flags2: 0,
                    destroyed: false,
                    dominated_at_run_time: false,
                    graphic: 7,
                    dead_graphic: Some(0),
                    dead_frame_size: Some((12, 30)),
                    type_counts_frames: false,
                    dead_counts_frames: true,
                    exit_angle: 0,
                    tribute: 250,
                    gravity: 0,
                    fee: 0,
                    animation: SiteAnimation {
                        dead_frames: 1,
                        ..SiteAnimation::default()
                    },
                    defense_dude: None,
                    def_count: 0,
                },
            ]
        );
    }

    /// `bytes`, a `spöb`, with this `DefenseDude` and `DefCount`.
    fn defended(mut bytes: Vec<u8>, dude: i16, count: i16) -> Vec<u8> {
        put_i16s(&mut bytes, 0x1C, &[dude, count]);
        bytes
    }

    #[test]
    fn a_listed_stellar_reads_its_tribute_tech_level_and_def_count_raw() {
        let mut paid = paying(defended(stellar(0, 0, 0, 1, 0), 140, 7006), 250);
        put_i16s(&mut paid, 0x0C, &[4]);
        let data = store(&[
            (System::TYPE, 130, system_with(&[128, 129])),
            (Stellar::TYPE, 128, paid),
            (Stellar::TYPE, 129, stellar(0, 0, 0, 1, 0)),
        ]);
        assert_eq!(
            data.listed_stellars()
                .iter()
                .map(|stellar| (
                    stellar.id.0,
                    stellar.tribute,
                    stellar.tech_level,
                    stellar.def_count
                ))
                .collect::<Vec<_>>(),
            [(128, 250, 4, 7006), (129, 0, 0, 0)]
        );
    }

    #[test]
    fn a_landing_site_reads_its_defence_dude_and_its_raw_def_count() {
        let data = store(&[
            (System::TYPE, 130, system_with(&[128, 129, 130])),
            (
                Stellar::TYPE,
                128,
                defended(stellar(0, 0, 0, 1, 0), 140, 1082),
            ),
            (Stellar::TYPE, 129, defended(stellar(0, 0, 0, 1, 0), -1, 5)),
            (
                Stellar::TYPE,
                130,
                defended(stellar(0, 0, 0, 1, 0), 127, -3),
            ),
        ]);
        assert_eq!(
            data.landing_sites(SystemId(130))
                .iter()
                .map(|site| (site.id.0, site.defense_dude, site.def_count))
                .collect::<Vec<_>>(),
            [
                (128, Some(DudeId(140)), 1082),
                (129, None, 5),
                (130, None, -3),
            ],
            "a `DefenseDude` below 128 is none, as `_LoadObjectData` keeps -1 \
             (@0x7770c-0x77725); `DefCount` raw"
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
    fn a_landing_site_counts_dead_frames_when_its_dead_types_spins_tiles_multiply_above_0() {
        for (x_tiles, y_tiles, counts) in [
            (1, 1, true),
            (-2, -3, true),
            (0, 5, false),
            (-1, 4, false),
            (300, 300, true),
            (300, -300, false),
        ] {
            let data = store(&[
                (System::TYPE, 130, system_with(&[128])),
                // `Type` 4 has no `spïn`; `DeadType` 0 is `spïn` 1000.
                (Stellar::TYPE, 128, dead_as(stellar(0, 0, 4, 1, 0), 0)),
                (Spin::TYPE, 1000, tiled(x_tiles, y_tiles)),
                (RLED, 1000, sheet(12, 30)),
            ]);
            let site = data.landing_sites(SystemId(130))[0];
            assert_eq!(site.dead_graphic, Some(0));
            assert_eq!(site.dead_counts_frames, counts, "{x_tiles} x {y_tiles}");
        }
        let no_dead_sheet = store(&[
            (System::TYPE, 130, system_with(&[128])),
            (Stellar::TYPE, 128, dead_as(stellar(0, 0, 4, 1, 0), 5)),
        ]);
        let site = no_dead_sheet.landing_sites(SystemId(130))[0];
        assert_eq!(site.dead_graphic, None);
        assert!(site.dead_counts_frames, "true with no `DeadType` sheet");
    }

    /// A `röid` of this `Strength` and `SpinRate`, with no yield, no
    /// fragments and no explosion (-1 each) and nothing else.
    fn roid(strength: i16, spin_rate: i16) -> Vec<u8> {
        let mut bytes = vec![0; Roid::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &[strength, spin_rate, -1]);
        put_i16s(&mut bytes, 0x0E, &[-1, -1, 0, -1]);
        bytes
    }

    /// A `spïn` naming `rlëD` `image`, `x_tiles` across and `y_tiles` down.
    fn spin_tiled(image: i16, x_tiles: i16, y_tiles: i16) -> Vec<u8> {
        let mut bytes = spin(image);
        put_i16s(&mut bytes, 0x08, &[x_tiles, y_tiles]);
        bytes
    }

    #[test]
    fn the_asteroid_types_are_each_roid_with_its_spins_frames_and_size() {
        let data = store(&[
            (Roid::TYPE, 128, roid(0, 120)),
            (Roid::TYPE, 132, roid(250, -40)),
            // Past the 16 types.
            (Roid::TYPE, 144, roid(9, 9)),
            (Spin::TYPE, 800, spin_tiled(1800, 6, 6)),
            (RLED, 1800, sheet(48, 40)),
            // Frames, but no sheet to size them.
            (Spin::TYPE, 804, spin_tiled(1804, 2, 3)),
        ]);
        assert_eq!(
            data.asteroid_types(),
            [
                AsteroidTypeRecord {
                    kind: 0,
                    strength: 0,
                    spin_rate: 120,
                    frames: 36,
                    frame_size: Some((48, 40)),
                    ..AsteroidTypeRecord::default()
                },
                AsteroidTypeRecord {
                    kind: 4,
                    strength: 250,
                    spin_rate: -40,
                    frames: 6,
                    frame_size: None,
                    ..AsteroidTypeRecord::default()
                },
            ]
        );
    }

    #[test]
    fn the_box_frame_sizes_are_spins_500_to_504s_where_a_sheet_reads() {
        let data = store(&[
            (Spin::TYPE, 501, spin_tiled(1501, 6, 6)),
            (RLED, 1501, sheet(22, 18)),
            // Frames, but no sheet to size them.
            (Spin::TYPE, 503, spin_tiled(1503, 6, 6)),
            // Past the five box graphics.
            (Spin::TYPE, 505, spin_tiled(1505, 6, 6)),
            (RLED, 1505, sheet(30, 30)),
        ]);
        assert_eq!(
            data.box_frame_sizes(),
            [None, Some((22, 18)), None, None, None]
        );
    }

    #[test]
    fn an_asteroid_type_with_no_spin_has_no_frames_and_no_size() {
        let data = store(&[(Roid::TYPE, 143, roid(5, 100))]);
        assert_eq!(
            data.asteroid_types(),
            [AsteroidTypeRecord {
                kind: 15,
                strength: 5,
                spin_rate: 100,
                frames: 0,
                frame_size: None,
                ..AsteroidTypeRecord::default()
            }]
        );
    }

    #[test]
    fn an_asteroid_type_carries_its_yield_particles_fragments_explosion_and_mass_raw() {
        let mut bytes = roid(40, 150);
        put_i16s(&mut bytes, 0x04, &[1003, 6, 9]);
        bytes[0x0A..0x0E].copy_from_slice(&0x00AB_CDEF_u32.to_be_bytes());
        put_i16s(&mut bytes, 0x0E, &[129, -1, 3, 1002, 75]);
        let data = store(&[(Roid::TYPE, 128, bytes)]);
        assert_eq!(
            data.asteroid_types(),
            [AsteroidTypeRecord {
                kind: 0,
                strength: 40,
                spin_rate: 150,
                frames: 0,
                frame_size: None,
                yield_type: 1003,
                yield_qty: 6,
                part_count: 9,
                part_color: 0x00AB_CDEF,
                frag_types: [129, -1],
                frag_count: 3,
                explode_type: 1002,
                mass: 75,
            }]
        );
    }

    #[test]
    fn the_asteroid_fields_are_each_systems_asteroids_and_ast_types() {
        let mut rocky = system();
        put_i16s(&mut rocky, 0x6A, &[12]);
        rocky[0x94..0x96].copy_from_slice(&0x8011_u16.to_be_bytes());
        let data = store(&[
            (System::TYPE, 130, system()),
            (System::TYPE, 131, rocky),
            (System::TYPE, 132, short(system())),
        ]);
        assert_eq!(
            data.asteroid_fields(),
            [
                AsteroidFieldRecord {
                    system: SystemId(130),
                    count: 0,
                    types: 0,
                },
                AsteroidFieldRecord {
                    system: SystemId(131),
                    count: 12,
                    types: 0x8011,
                },
            ]
        );
    }

    #[test]
    fn a_landing_site_reads_its_animation_fields_and_its_sheets_frame_counts() {
        let mut gate = dead_as(stellar(0, 0, 3, 1, 0), 6);
        put_i16s(&mut gate, 0x18, &[17]);
        put_i16s(&mut gate, 0x22, &[-1, 4]);
        let tiles = |image, x: i16, y: i16| {
            let mut bytes = spin(image);
            put_i16s(&mut bytes, 0x08, &[x, y]);
            bytes
        };
        let data = store(&[
            (System::TYPE, 130, system_with(&[128, 129, 130])),
            (Stellar::TYPE, 128, gate),
            // A DeadType sheet that is not there counts no frames.
            (Stellar::TYPE, 129, dead_as(stellar(0, 0, 4, 1, 0), 5)),
            // Tiles that multiply to 0 or less, or past a u16.
            (Stellar::TYPE, 130, dead_as(stellar(0, 0, 7, 1, 0), 8)),
            (Spin::TYPE, 1003, tiles(1003, 6, 7)),
            (RLED, 1003, sheet(12, 30)),
            (Spin::TYPE, 1004, tiles(1004, 2, 3)),
            (RLED, 1004, sheet(12, 30)),
            (Spin::TYPE, 1005, tiles(1005, 1, 9)),
            (Spin::TYPE, 1006, tiles(1006, 4, 2)),
            (RLED, 1006, sheet(50, 20)),
            (Spin::TYPE, 1007, tiles(1007, -2, 3)),
            (RLED, 1007, sheet(12, 30)),
            (Spin::TYPE, 1008, tiles(1008, 300, 300)),
            (RLED, 1008, sheet(12, 30)),
        ]);
        let animations: Vec<SiteAnimation> = data
            .landing_sites(SystemId(130))
            .iter()
            .map(|site| site.animation)
            .collect();
        assert_eq!(
            animations,
            [
                SiteAnimation {
                    delay: -1,
                    frame0_bias: 4,
                    open_frame: 17,
                    type_frames: 42,
                    dead_frames: 8,
                },
                SiteAnimation {
                    type_frames: 6,
                    ..SiteAnimation::default()
                },
                SiteAnimation {
                    type_frames: 0,
                    dead_frames: u16::MAX,
                    ..SiteAnimation::default()
                },
            ]
        );
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
    fn graphic_type_0_counts_frames_when_spin_1000s_tiles_multiply_above_0() {
        for (x_tiles, y_tiles, counts) in [
            (1, 1, true),
            (-2, -3, true),
            (0, 5, false),
            (-1, 4, false),
            (300, 300, true),
            (300, -300, false),
        ] {
            let data = store(&[
                (Spin::TYPE, 1000, tiled(x_tiles, y_tiles)),
                (RLED, 1000, sheet(12, 30)),
            ]);
            assert_eq!(
                data.fallback_counts_frames(),
                counts,
                "{x_tiles} x {y_tiles}"
            );
            let data = std::rc::Rc::new(data);
            assert_eq!(PilotCatalog::fallback_counts_frames(&data), counts, "Rc");
            assert_eq!(PilotCatalog::fallback_counts_frames(&&*data), counts, "&");
        }
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
    fn the_star_map_is_every_readable_system_by_id_with_its_position_links_govt_interference_murk_and_buoy()
     {
        let data = store(&[
            (
                System::TYPE,
                131,
                murky(
                    buoyed(
                        interfering(governed(linked(600, -75, &[130, 999, 131]), 140), 45),
                        7,
                    ),
                    5,
                ),
            ),
            (System::TYPE, 129, short(linked(0, 0, &[130]))),
            (System::TYPE, 130, buoyed(linked(-20, 40, &[]), -1)),
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
                    murk: 0,
                    message: -1,
                    background: 0,
                },
                StarSystem {
                    id: SystemId(131),
                    position: Vec2::new(600.0, -75.0),
                    links: vec![SystemId(130), SystemId(999), SystemId(131)],
                    govt: Some(GovtId(140)),
                    stellars: vec![None; 16],
                    interference: 45,
                    murk: 5,
                    message: 7,
                    background: 0,
                },
            ]
        );
        assert_eq!(store(&[]).star_map(), []);
    }

    #[test]
    fn a_systems_murk_is_its_sÿsts_raw() {
        // `_LoadObjectData` stores `Murk` (`sÿst` +0x92) at runtime +0xbc
        // (@0x77eb7), which the map panel reads.
        let mut murky = linked(0, 0, &[]);
        put_i16s(&mut murky, 0x92, &[45]);
        let data = store(&[
            (System::TYPE, 130, murky),
            (System::TYPE, 131, linked(0, 0, &[])),
        ]);
        let murk: Vec<i16> = data.star_map().iter().map(|system| system.murk).collect();
        assert_eq!(murk, [45, 0]);
    }

    #[test]
    fn a_systems_background_is_its_sÿsts_bkgnd_color_raw() {
        // `BkgndColor` is `sÿst` +0x8e, `00RRGGBB`; `_LoadObjectData`
        // builds its `RGBColor` from it (@0x77d85-0x77dc5).
        let mut coloured = linked(0, 0, &[]);
        coloured[0x8E..0x92].copy_from_slice(&0x0012_3456_u32.to_be_bytes());
        let data = store(&[
            (System::TYPE, 130, coloured),
            (System::TYPE, 131, linked(0, 0, &[])),
        ]);
        let backgrounds: Vec<u32> = data
            .star_map()
            .iter()
            .map(|system| system.background)
            .collect();
        assert_eq!(backgrounds, [0x0012_3456, 0]);
    }

    #[test]
    fn an_empty_con_slot_keeps_its_place_on_the_star_map_as_minus_1() {
        // `_LoadObjectData` writes all 16 Con slots in place, an empty one
        // as -1 (@0x7806d-0x780b8), so a link's index is its slot number.
        let data = store(&[(System::TYPE, 130, linked(0, 0, &[-1, 131, -1, -1, 132]))]);
        assert_eq!(
            data.star_map()[0].links,
            [-1, 131, -1, -1, 132].map(SystemId),
            "the empty slots after the last link are dropped"
        );
    }

    /// `bytes`, a `sÿst`, with this `Message` (offset 0x68).
    fn buoyed(mut bytes: Vec<u8>, message: i16) -> Vec<u8> {
        put_i16s(&mut bytes, 0x68, &[message]);
        bytes
    }

    /// `bytes`, a `sÿst`, with this `Murk` (offset 0x92), raw.
    fn murky(mut bytes: Vec<u8>, murk: i16) -> Vec<u8> {
        put_i16s(&mut bytes, 0x92, &[murk]);
        bytes
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
    fn a_landing_sites_gravity_is_its_spöbs_raw() {
        // `_LoadObjectData` stores `Gravity` (`spöb` +0x238) at runtime
        // +0x46c (@0x773de-0x773e5), which the map panel reads.
        let gravities = [0, 7, -3];
        let mut resources = vec![(System::TYPE, 130, system_with(&[128, 129, 130]))];
        for (id, gravity) in (128..).zip(gravities) {
            let mut bytes = stellar(0, 0, 0, 1, 0);
            put_i16s(&mut bytes, 0x238, &[gravity]);
            resources.push((Stellar::TYPE, id, bytes));
        }
        let data = store(&resources);
        let found: Vec<i16> = data
            .landing_sites(SystemId(130))
            .iter()
            .map(|site| site.gravity)
            .collect();
        assert_eq!(found, gravities);
    }

    #[test]
    fn a_landing_sites_fee_is_its_spöbs_raw() {
        // `Fee` (`spöb` +0x234, i32) goes through raw, whatever it is:
        // `nova_sim::landing::fee_due` decides what a value means.
        let fees: [i32; 3] = [0, 1000, -5];
        let mut resources = vec![(System::TYPE, 130, system_with(&[128, 129, 130]))];
        for (id, fee) in (128..).zip(fees) {
            let mut bytes = stellar(0, 0, 0, 1, 0);
            bytes[0x234..0x238].copy_from_slice(&fee.to_be_bytes());
            resources.push((Stellar::TYPE, id, bytes));
        }
        let data = store(&resources);
        let found: Vec<i32> = data
            .landing_sites(SystemId(130))
            .iter()
            .map(|site| site.fee)
            .collect();
        assert_eq!(found, fees);
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
    fn a_plugin_string_is_its_str_resource_raw() {
        let data = store(&[
            (StrResource::TYPE, 9100, str_resource("grain")),
            (StrResource::TYPE, 9101, Vec::new()),
            (StrResource::TYPE, 9102, b"\x05ab".to_vec()),
            (StrResource::TYPE, 4999, b"\x02Hixyz".to_vec()),
        ]);
        assert_eq!(data.plugin_string(9100), StringPatch::Text("grain".into()));
        assert_eq!(data.plugin_string(9101), StringPatch::Empty);
        assert_eq!(data.plugin_string(9102), StringPatch::Unreadable);
        assert_eq!(data.plugin_string(4999), StringPatch::Text("Hi".into()));
        assert_eq!(data.plugin_string(9103), StringPatch::Absent);
        assert_eq!(
            CommCatalog::plugin_string(&&data, 9100),
            StringPatch::Text("grain".into()),
            "through a borrow"
        );
        let shared: std::rc::Rc<dyn CommCatalog> = std::rc::Rc::new(data);
        assert_eq!(shared.plugin_string(9101), StringPatch::Empty, "shared");
    }

    #[test]
    fn any_l33t_resource_turns_the_leet_mode_on() {
        assert!(!store(&[]).leet_mode());
        assert!(!CommCatalog::leet_mode(&&store(&[])), "a borrow asks");
        assert!(
            !store(&[(StrList::TYPE, 4001, str_list(&["food"]))]).leet_mode(),
            "nothing else does"
        );
        let empty = store(&[(LEET, 128, Vec::new())]);
        assert!(empty.leet_mode(), "even empty");
        assert!(CommCatalog::leet_mode(&&empty), "through a borrow");
        let shared: std::rc::Rc<dyn CommCatalog> = std::rc::Rc::new(store(&[(LEET, 200, vec![1])]));
        assert!(shared.leet_mode(), "through the shared catalog");
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

    fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> Vec<u8> {
        resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
                fork.resource(*ty, *id, None, data)
            })
            .build()
            .bytes
    }

    /// The store of a data file, `/data/Nova Data`, and a plug-in,
    /// `/plugins/Price Patch.npif`, holding these resources.
    fn layered(data: &[(ResType, i16, Vec<u8>)], plugin: &[(ResType, i16, Vec<u8>)]) -> GameData {
        one_file(fork(data))
            .file(
                Path::new(PLUGINS_DIR).join("Price Patch.npif"),
                fork(plugin),
            )
            .load()
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

    #[test]
    fn a_str_arrives_raw_from_any_file_by_its_id() {
        let data = layered(
            &[
                (StrResource::TYPE, 1001, str_resource("base")),
                (StrResource::TYPE, 1002, str_resource("base")),
            ],
            &[
                (StrResource::TYPE, 1002, str_resource("plug-in")),
                (StrResource::TYPE, 1003, Vec::new()),
                (StrResource::TYPE, 1004, b"\x05ab".to_vec()),
                (StrResource::TYPE, 1005, b"\x03Buoxyz".to_vec()),
                (StrList::TYPE, 1006, str_list(&["a list"])),
            ],
        );
        let cases = [
            (1000, StringPatch::Absent),
            (1001, StringPatch::Text("base".into())),
            (1002, StringPatch::Text("plug-in".into())),
            (1003, StringPatch::Empty),
            (1004, StringPatch::Unreadable),
            (1005, StringPatch::Text("Buo".into())),
            (1006, StringPatch::Absent),
        ];
        for (id, patch) in cases {
            assert_eq!(data.string_patch(id), patch, "{id}");
        }
    }

    #[test]
    fn the_buoy_strings_are_str_1000_whole_and_none_when_missing() {
        let data = layered(
            &[
                (StrList::TYPE, 1000, str_list(&["one", "two"])),
                (StrList::TYPE, 1001, str_list(&["other"])),
            ],
            &[],
        );
        assert_eq!(data.buoy_strings(), ["one", "two"]);
        assert_eq!(store(&[]).buoy_strings(), Vec::<String>::new());
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
        opals[0x66..0x69].copy_from_slice(b"Opl");
        opals[0xA6..0xA9].copy_from_slice(b"b43");
        opals[0x1A5..0x1A9].copy_from_slice(b"!b80");
        put_i16s(&mut opals, 0x22, &[0x0003, 0x8004_u16.cast_signed()]);
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
                    abbrev: String::new(),
                    base_price: 300,
                    sold_at: vec![StellarId(160)],
                    bought_at: Vec::new(),
                    buy_on: Test::default(),
                    sell_on: Test::default(),
                    flags: 0,
                    scan_mask: 0,
                    lc_name: "water".to_owned(),
                },
                JunkRecord {
                    id: JunkId(146),
                    name: "Opals".to_owned(),
                    abbrev: "Opl".to_owned(),
                    base_price: 1200,
                    sold_at: vec![StellarId(189), StellarId(165)],
                    bought_at: vec![StellarId(185), StellarId(199)],
                    buy_on: Test::parse("b43"),
                    sell_on: Test::parse("!b80"),
                    flags: 0x0003,
                    scan_mask: 0x8004,
                    lc_name: "opals".to_owned(),
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

    /// A `bööm` of `sound_index`, its frame advance 100 and its graphic 0.
    fn boom(sound_index: i16) -> Vec<u8> {
        [100_i16, sound_index, 0]
            .iter()
            .flat_map(|value| value.to_be_bytes())
            .collect()
    }

    #[test]
    fn boom_sounds_list_each_readable_bööm_by_id_with_a_sound_of_0_to_63() {
        use nova_data::records::boom::Boom;
        let data = store(&[
            (Boom::TYPE, 130, boom(63)),
            (Boom::TYPE, 128, boom(2)),
            (Boom::TYPE, 129, boom(-1)),
            (Boom::TYPE, 131, boom(64)),
        ]);
        assert_eq!(
            data.boom_sounds(),
            [(BoomId(128), SoundId(302)), (BoomId(130), SoundId(363))]
        );
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
    fn stellar_scripts_list_every_readable_spöbs_on_dominate_and_on_release() {
        let mut ruled = stellar(0, 0, 0, 0, 0);
        ruled[0x36..0x3A].copy_from_slice(b"b100");
        ruled[0x135..0x13A].copy_from_slice(b"!b100");
        let data = store(&[
            (Stellar::TYPE, 130, stellar(0, 0, 0, 0, 0)),
            (Stellar::TYPE, 128, ruled),
            (Stellar::TYPE, 129, short(stellar(0, 0, 0, 1, 0))),
        ]);
        let scripts = data.stellar_scripts();
        assert_eq!(
            scripts.iter().map(|script| script.id).collect::<Vec<_>>(),
            [StellarId(128), StellarId(130)],
            "the unreadable 129 left out"
        );
        assert_eq!(
            scripts[0].on_dominate.tree().map(|expr| expr.ops.len()),
            Some(1),
            "b100"
        );
        assert_eq!(
            scripts[0].on_release.tree().map(|expr| expr.ops.len()),
            Some(1),
            "!b100"
        );
        assert_eq!(scripts[1].on_dominate, Script::default(), "blank");
        assert_eq!(scripts[1].on_release, Script::default(), "blank");
        assert!(
            shared_script(
                &scripts[0].on_dominate,
                &data.stellar_scripts()[0].on_dominate
            ),
            "parsed once"
        );
        assert_eq!(store(&[]).stellar_scripts(), []);
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

    /// A `spöb` whose `Weapon` (0x23A) is `weapon`.
    fn gunned(weapon: i16) -> Vec<u8> {
        let mut bytes = stellar(0, 0, 0, 0, 0);
        put_i16s(&mut bytes, 0x23A, &[weapon]);
        bytes
    }

    #[test]
    fn stellar_weapons_list_every_readable_spöb_whose_weapon_is_128_or_more_by_id() {
        let data = store(&[
            (Stellar::TYPE, 133, gunned(199)),
            (Stellar::TYPE, 128, gunned(128)),
            (Stellar::TYPE, 129, short(gunned(196))),
            (Stellar::TYPE, 130, gunned(0)),
            (Stellar::TYPE, 131, gunned(-1)),
            (Stellar::TYPE, 132, gunned(127)),
        ]);
        assert_eq!(
            data.stellar_weapons(),
            [
                (StellarId(128), WeaponId(128)),
                (StellarId(133), WeaponId(199)),
            ],
            "the unreadable 129 left out; 0, -1 and 127 are none (@0x7779f-0x777b2)"
        );
        assert_eq!(store(&[]).stellar_weapons(), []);
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
        put_i16s(&mut bytes, 0x30, &[300, 2, 9]);
        bytes[0x48..0x4A].copy_from_slice(&0x8200_u16.to_be_bytes());
        put_i16s(&mut bytes, 0x56, &[12, 2]);
        put_i16s(&mut bytes, 0x5A, &[60, 30]);
        put_i16s(&mut bytes, 0x5E, &[40, 0, 7, 0]);
        bytes[0x66..0x68].copy_from_slice(&0x0003_u16.to_be_bytes());
        put_i16s(&mut bytes, 0x3E, &[5, 148, -10, 2, 8]);
        put_i16s(&mut bytes, 0x68, &[4, 70, 6, 3]);
        put_i16s(&mut bytes, 0x4A, &[-40]);
        bytes[0x72..0x76].copy_from_slice(&0x0034_c2ff_u32.to_be_bytes());
        put_i16s(&mut bytes, 0x4C, &[12, 45, 300]);
        bytes[0x52..0x56].copy_from_slice(&0x0012_3456_u32.to_be_bytes());
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
            graphic: 7,
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
            beam_width: 2,
            falloff: 9,
            li_density: 3,
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
            recoil: 12,
            exit_type: 2,
            ionization: -40,
            ionize_color: 0x0034_c2ff,
            jam_vuln: [40, 0, 7, 0],
            frame: None,
            hit_particles: 12,
            hit_part_life: 45,
            hit_part_vel: 300,
            hit_part_color: 0x0012_3456,
        };
        assert_eq!(
            data.weapons(),
            [record(128), record(140)],
            "by ID, the undecodable one left out"
        );
        assert_eq!(store(&[]).weapons(), []);
    }

    /// [`weapon_bytes`] of `Graphic` `graphic`.
    fn drawn_as(graphic: i16) -> Vec<u8> {
        let mut bytes = weapon_bytes();
        put_i16s(&mut bytes, 0x0E, &[graphic]);
        bytes
    }

    #[test]
    fn a_wëaps_shots_carry_the_frame_size_of_spin_3000_plus_its_graphic() {
        let data = store(&[
            (Weapon::TYPE, 128, drawn_as(3)),
            (Weapon::TYPE, 129, drawn_as(-1)),
            (Weapon::TYPE, 130, drawn_as(4)),
            (Weapon::TYPE, 131, drawn_as(5)),
            (Spin::TYPE, 2999, spin(2000)),
            (Spin::TYPE, 3003, spin(2000)),
            (Spin::TYPE, 3005, spin(2001)),
            (RLED, 2000, sheet(10, 30)),
        ]);
        let frames: Vec<Option<FrameSize>> =
            data.weapons().iter().map(|record| record.frame).collect();
        assert_eq!(
            frames,
            [
                Some(FrameSize {
                    width: 10,
                    height: 30
                }),
                None,
                None,
                None
            ],
            "spïn 3003's; none for Graphic -1, no spïn 3004, and spïn 3005's missing sheet"
        );
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
        put_i16s(&mut bytes, 0x4C, &[3]);
        bytes[0x726..0x728].copy_from_slice(&0x0040_u16.to_be_bytes());
        put_i16s(&mut bytes, 0x6CE, &[-1, 138, -1, -1]);
        put_i16s(&mut bytes, 0x6D6, &[0, 3, 0, 0]);
        put_i16s(&mut bytes, 0x6DE, &[0, 20, 0, 0]);
        put_i16s(&mut bytes, 0x36E, &[140]);
        bytes[0x30..0x34].copy_from_slice(&(-1_i32).to_be_bytes());
        bytes
    }

    /// A `shän` whose `BaseXSize` is `size`.
    fn anim(width: i16, height: i16) -> Vec<u8> {
        ShipAnimBuilder::new()
            .base_x_size(width)
            .base_y_size(height)
            .build()
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
        let record = |id, size, height, exits| HullRecord {
            id: ShipId(id),
            flags: 0x0130,
            flags2: 0,
            flags3: 0x0040,
            death_delay: 60,
            pod_count: 3,
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
            key_carried: Some(ShipId(140)),
            cost: -1,
            speed: 2,
            exits,
            display: exits.map(|_| ShipDisplay::default()),
            escort_class: EscortClass::Fighter,
        };
        assert_eq!(
            data.hulls(),
            [
                record(128, Some(24), Some(40), Some(ShipExits::default())),
                record(130, None, None, None),
                record(131, None, None, None)
            ],
            "by ID, the undecodable shïp left out; without a shän that can be read, no size"
        );
        assert_eq!(store(&[]).hulls(), []);
    }

    #[test]
    fn a_hull_carries_its_shäns_exit_points_compression_and_frames() {
        let mut bytes = anim(24, 24);
        put_i16s(&mut bytes, 0x34, &[64]);
        put_i16s(&mut bytes, 0x48, &[3, -3, 4, -4, 10, 11, 12, 13]);
        put_i16s(&mut bytes, 0x58, &[1, 2, 3, 4, 5, 6, 7, 8]);
        put_i16s(&mut bytes, 0x68, &[-1, -2, -3, -4, -5, -6, -7, -8]);
        put_i16s(&mut bytes, 0x78, &[20, 0, -20, 0, 0, 30, 0, -30]);
        put_i16s(&mut bytes, 0x88, &[100, 71, 0, -5]);
        put_i16s(&mut bytes, 0x90, &[-2, -2, -2, -2, 0, 1, 2, 3]);
        put_i16s(&mut bytes, 0xA0, &[4, 5, 6, 7, 25, 0, -25, 0]);
        let data = store(&[(Ship::TYPE, 128, armed()), (ShipAnim::TYPE, 128, bytes)]);
        assert_eq!(
            data.hulls()[0].exits,
            Some(ShipExits {
                points: [
                    [(3, 10), (-3, 11), (4, 12), (-4, 13)],
                    [(1, 5), (2, 6), (3, 7), (4, 8)],
                    [(-1, -5), (-2, -6), (-3, -7), (-4, -8)],
                    [(20, 0), (0, 30), (-20, 0), (0, -30)],
                ],
                z: [[-2; 4], [0, 1, 2, 3], [4, 5, 6, 7], [25, 0, -25, 0]],
                up_compress: (100, 71),
                down_compress: (0, -5),
                frames: 64,
            })
        );
    }

    #[test]
    fn a_hull_carries_its_shäns_display_fields() {
        // shän 379's: `Flags` 0xc2, six base sets, `AnimDelay` 5, no alt.
        let mut bytes = anim(24, 24);
        put_i16s(&mut bytes, 0x04, &[6]);
        put_i16s(&mut bytes, 0x0C, &[-1, -1, 0]);
        bytes[0x2E..0x30].copy_from_slice(&0x00C2_u16.to_be_bytes());
        put_i16s(&mut bytes, 0x30, &[5]);
        let data = store(&[(Ship::TYPE, 128, armed()), (ShipAnim::TYPE, 128, bytes)]);
        assert_eq!(
            data.hulls()[0].display,
            Some(ShipDisplay {
                flags: 0x00C2,
                base_sets: 6,
                anim_delay: 5,
                alt_sets: 0,
                alt_image: false,
                blink: Blink::STEADY,
                lights_image: false,
            })
        );
        // shän 380's: `Flags` 0x58, one base set, six alt sets of `rlëD`
        // 1330.
        let mut bytes = anim(24, 24);
        put_i16s(&mut bytes, 0x04, &[1]);
        put_i16s(&mut bytes, 0x0C, &[1330, 1331, 6]);
        bytes[0x2E..0x30].copy_from_slice(&0x0058_u16.to_be_bytes());
        put_i16s(&mut bytes, 0x30, &[5]);
        let data = store(&[(Ship::TYPE, 128, armed()), (ShipAnim::TYPE, 128, bytes)]);
        let display = data.hulls()[0].display.expect("a shän");
        assert_eq!((display.flags, display.base_sets), (0x0058, 1));
        assert_eq!((display.alt_sets, display.alt_image), (6, true));
        assert!(!display.lights_image);
        // A lights image (`LightImageID` 0x1E) blinking the stock triangle.
        let mut bytes = anim(24, 24);
        put_i16s(&mut bytes, 0x1E, &[1400]);
        put_i16s(&mut bytes, 0x36, &[2, 10, 75, 32, 75]);
        let data = store(&[(Ship::TYPE, 128, armed()), (ShipAnim::TYPE, 128, bytes)]);
        let display = data.hulls()[0].display.expect("a shän");
        assert!(display.lights_image);
        assert_eq!(
            display.blink,
            Blink {
                mode: 2,
                a: 10,
                b: 75,
                c: 32,
                d: 75
            }
        );
        let data = store(&[(Ship::TYPE, 128, armed())]);
        assert_eq!(data.hulls()[0].display, None, "no shän");
    }

    #[test]
    fn a_hull_carries_its_shïps_flags2() {
        let flagged = |flags2: u16| {
            let mut bytes = ship(1, 2, 3);
            bytes[0x62..0x64].copy_from_slice(&flags2.to_be_bytes());
            store(&[(Ship::TYPE, 128, bytes)]).hulls()[0].flags2
        };
        assert_eq!(flagged(0x0008), 0x0008);
        assert_eq!(flagged(0x0080), 0x0080);
        assert_eq!(flagged(0x8801), 0x8801);
    }

    /// The hull of a `shïp` of `EscortType` `escort_type`, `InherentAI`
    /// `inherent_ai` and `Mass` `mass`, as the combat catalog reads it.
    fn classed(escort_type: i16, inherent_ai: i16, mass: i16) -> HullRecord {
        let mut bytes = ship(1, 2, 3);
        put_i16s(&mut bytes, 0x3E, &[mass]);
        put_i16s(&mut bytes, 0x42, &[inherent_ai]);
        put_i16s(&mut bytes, 0x732, &[escort_type]);
        let mut hulls = store(&[(Ship::TYPE, 128, bytes)]).hulls();
        hulls.remove(0)
    }

    /// The hull record's escort class, which combat's `HullSpec` turns
    /// into its miss class (`nova_sim::combat::hull`'s tests cover that
    /// mapping).
    #[test]
    fn a_hulls_miss_class_comes_from_its_escort_type_or_one_worked_out() {
        let class =
            |escort_type, inherent_ai, mass| classed(escort_type, inherent_ai, mass).escort_class;
        // Worked out from the mass for an `EscortType` outside 0-3.
        for (mass, expected) in [
            (49, EscortClass::Fighter),
            (50, EscortClass::Medium),
            (199, EscortClass::Medium),
            (200, EscortClass::Warship),
        ] {
            assert_eq!(class(-1, 3, mass), expected, "mass {mass}");
        }
        assert_eq!(
            class(-1, 2, 10),
            EscortClass::Freighter,
            "a freighter by its InherentAI"
        );
        // Named directly, whatever the mass.
        for (escort_type, expected) in [
            (0, EscortClass::Fighter),
            (1, EscortClass::Medium),
            (2, EscortClass::Warship),
            (3, EscortClass::Freighter),
        ] {
            assert_eq!(class(escort_type, 3, 500), expected, "{escort_type}");
            assert_eq!(class(escort_type, 1, 10), expected, "{escort_type}");
        }
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
        put_i16s(&mut bytes, 0x30, &[150]);
        bytes[0x32..0x34].copy_from_slice(&0x8004_u16.to_be_bytes());
        bytes[0x34..0x3F].copy_from_slice(b"Federation\0");
        bytes[0x64..0x73].copy_from_slice(b"the Federation\0");
        put_i16s(&mut bytes, 0xAE, &[9001]);
        bytes[0x54..0x5C].copy_from_slice(&0x8000_0000_0000_0021_u64.to_be_bytes());
        put_i16s(&mut bytes, 0x5C, &[25, 0, -3, 0]);
        bytes
    }

    #[test]
    fn a_govt_records_name_is_its_resource_name_cut_at_its_last_semicolon() {
        // `_LoadObjectData` cuts the name before its last ';', dropping
        // the spaces and ';'s before it (@0x7bde9-0x7be2a), and copies
        // 31 bytes of it, the length among them (@0x7be51).
        let long = "A Government Named At Great Length";
        let data = store_named(&[
            (
                Govt::TYPE,
                128,
                Some("Federation;hates Temmin Shard"),
                govt_bytes(),
            ),
            (Govt::TYPE, 129, Some("Auroran ; ;notes;more"), govt_bytes()),
            (Govt::TYPE, 130, Some("Polaris"), govt_bytes()),
            (Govt::TYPE, 131, None, govt_bytes()),
            (Govt::TYPE, 132, Some(long), govt_bytes()),
        ]);
        let names: Vec<String> = data
            .governments()
            .into_iter()
            .map(|record| record.name)
            .collect();
        assert_eq!(
            names,
            ["Federation", "Auroran ; ;notes", "Polaris", "", &long[..30]]
        );
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
            name: String::new(),
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
            medium_name: "the Federation".to_owned(),
            scan_mask: 0x8004,
            scan_fine: -5,
            news_pic: 9001,
            require: 0x8000_0000_0000_0021,
            inh_jam: [25, 0, -3, 0],
            skill_mult: 150,
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
    fn a_missions_dude_is_read_as_the_traffic_reads_it() {
        let data = store(&[(
            Dude::TYPE,
            128,
            dude(3, 129, &[(140, 60), (-1, 30), (141, -10)]),
        )]);
        assert_eq!(
            PilotCatalog::mission_dude(&data, DudeId(128)),
            Some(DudeRecord {
                ai_type: 3,
                govt: Some(GovtId(129)),
                ships: vec![(ShipId(140), 60), (ShipId(141), -10)],
                booty: 0,
                info_types: 0,
            })
        );
        assert_eq!(PilotCatalog::mission_dude(&data, DudeId(129)), None);
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
                    quote: None,
                    flags: 0,
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
                    quote: None,
                    flags: 0,
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

    #[test]
    fn a_fleets_quote_above_0_is_carried() {
        let quoting = |quote: i16| {
            let mut bytes = fleet(140, &[], 129, -1);
            put_i16s(&mut bytes, 0x11E, &[quote]);
            store(&[(Fleet::TYPE, 128, bytes)])
                .fleets()
                .first()
                .and_then(|record| record.quote)
        };
        assert_eq!(quoting(7600), Some(7600));
        assert_eq!(quoting(0), None);
        assert_eq!(quoting(-1), None);
    }

    #[test]
    fn a_fleets_flags_are_carried() {
        let flagged = |flags: i16| {
            let mut bytes = fleet(140, &[], 129, -1);
            put_i16s(&mut bytes, 0x120, &[flags]);
            store(&[(Fleet::TYPE, 128, bytes)])
                .fleets()
                .first()
                .map(|record| record.flags)
        };
        assert_eq!(flagged(0x0001), Some(0x0001));
        assert_eq!(flagged(0), Some(0));
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

    /// A `nëbu` over `rect` (left, top, width, height), active by
    /// `active_on` and running `on_explore`, each NUL-terminated.
    fn nebu(rect: [i16; 4], active_on: &[u8], on_explore: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0; Nebula::SIZE.expect("fixed")];
        put_i16s(&mut bytes, 0x00, &rect);
        bytes[0x08..0x08 + active_on.len()].copy_from_slice(active_on);
        bytes[0x107..0x107 + on_explore.len()].copy_from_slice(on_explore);
        bytes
    }

    #[test]
    fn nebulae_read_each_nebu_128_to_159_raw() {
        let data = store_named(&[
            (Nebula::TYPE, 160, None, nebu([1, 2, 3, 4], b"\0", b"\0")),
            (
                Nebula::TYPE,
                159,
                None,
                nebu([-400, 120, 300, 200], b"b5\0", b"b6 !b7\0"),
            ),
            (Nebula::TYPE, 127, None, nebu([1, 2, 3, 4], b"\0", b"\0")),
            (
                Nebula::TYPE,
                128,
                None,
                nebu([10, -20, 0, 40], b"\0", b"\0"),
            ),
        ]);
        let nebulae = data.nebulae();
        assert_eq!(
            nebulae,
            [
                NebulaRecord {
                    id: NebulaId(128),
                    left: 10,
                    top: -20,
                    width: 0,
                    height: 40,
                    active_on: Test::default(),
                    on_explore: Script::default(),
                },
                NebulaRecord {
                    id: NebulaId(159),
                    left: -400,
                    top: 120,
                    width: 300,
                    height: 200,
                    active_on: Test::parse("b5"),
                    on_explore: Script::parse("b6 !b7"),
                },
            ],
            "only 128-159, by ID"
        );
        assert_eq!(
            nebulae[0].active_on.tree(),
            Some(&crate::control::TestExpr::Always),
            "a blank ActiveOn holds"
        );
        assert_eq!(store(&[]).nebulae(), []);
        for (a, b) in data.nebulae().iter().zip(&data.nebulae()) {
            assert!(shared(&a.active_on, &b.active_on), "parsed once");
            assert!(shared_script(&a.on_explore, &b.on_explore), "parsed once");
        }
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
        pictured(text, -1)
    }

    /// A `dësc` of `text` whose `Graphic` is `graphic`.
    fn pictured(text: &str, graphic: i16) -> Vec<u8> {
        let mut bytes = text.as_bytes().to_vec();
        bytes.push(0);
        bytes.extend(graphic.to_be_bytes());
        bytes.extend([0; 34]);
        bytes
    }

    /// A `dësc` of `text` naming the movie `file`, played as `flags` say.
    fn filmed(text: &str, file: &str, flags: u16) -> Vec<u8> {
        let mut bytes = pictured(text, -1);
        let tail = bytes.len() - 34;
        bytes[tail..tail + file.len()].copy_from_slice(file.as_bytes());
        bytes[tail + 32..].copy_from_slice(&flags.to_be_bytes());
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
            (Desc::TYPE, 4002, None, pictured("Offer <DST>.", 5008)),
            (Desc::TYPE, 200, None, pictured("Brief.", 300)),
            (Desc::TYPE, 201, None, pictured("Quick <DL>.", 301)),
            (Desc::TYPE, 202, None, pictured("Comp.", 302)),
            (Desc::TYPE, 203, None, pictured("Fail.", 303)),
            (Desc::TYPE, 204, None, pictured("Refuse.", 304)),
            (Desc::TYPE, 206, None, pictured("Load <CQ>.", 306)),
            (Desc::TYPE, 207, None, desc("Dump.")),
            (Desc::TYPE, 209, None, pictured("Done <SN>.", 309)),
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
                    refuse_named: true,
                    load_cargo: Some("Load <CQ>.".to_owned()),
                    dump_cargo: Some("Dump.".to_owned()),
                    ship_done: Some("Done <SN>.".to_owned()),
                    accept_button: "I'm in".to_owned(),
                    refuse_button: "No way".to_owned(),
                    graphics: TextGraphics {
                        offer: Some(5008),
                        brief: Some(300),
                        comp: Some(302),
                        fail: Some(303),
                        refuse: Some(304),
                        load_cargo: Some(306),
                        dump_cargo: None,
                        ship_done: Some(309),
                    },
                    movies: TextMovies::default(),
                },
                MissionTexts::named(MissionId(131), "Plain run  "),
                MissionTexts {
                    accept_button: " ok".to_owned(),
                    refuse_named: true,
                    ..MissionTexts::named(MissionId(132), "")
                },
            ],
            "each text carries its own dësc's graphic, none for a graphic of -1; -1 and a missing dësc (0 among them) are no text and no graphic, a RefuseText naming a missing one still named; the undecodable mïsn is skipped"
        );
        assert_eq!(store(&[]).mission_texts(), []);
    }

    #[test]
    fn each_text_carries_the_movie_its_own_desc_names() {
        use nova_data::records::mission::Mission;
        let data = store_named(&[
            (
                Mission::TYPE,
                130,
                Some("Filmed"),
                texted([200, -1, 206, 207, 202, 203, 204, 209], b"\0", b"\0"),
            ),
            (
                Desc::TYPE,
                4002,
                None,
                filmed("Offer.", "Offer.mov", 0x0001),
            ),
            (Desc::TYPE, 200, None, filmed("Brief.", "Brief.mov", 0x0006)),
            (Desc::TYPE, 202, None, filmed("Comp.", "Comp.mov", 0)),
            (Desc::TYPE, 203, None, filmed("Fail.", "Fail.mov", 0)),
            (Desc::TYPE, 204, None, filmed("Refuse.", "Refuse.mov", 0)),
            (Desc::TYPE, 206, None, filmed("Load.", "Load.mov", 0)),
            (Desc::TYPE, 207, None, desc("Dump.")),
            (Desc::TYPE, 209, None, filmed("Done.", "Done.mov", 0)),
        ]);
        let movie = |file: &str| DescMovie::from_desc(file, 0);
        let texts = data.mission_texts();
        assert_eq!(
            texts[0].movies,
            TextMovies {
                offer: DescMovie::from_desc("Offer.mov", 0x0001),
                brief: DescMovie::from_desc("Brief.mov", 0x0006),
                quick_brief: None,
                comp: movie("Comp.mov"),
                fail: movie("Fail.mov"),
                refuse: movie("Refuse.mov"),
                load_cargo: movie("Load.mov"),
                dump_cargo: None,
                ship_done: movie("Done.mov"),
            },
            "a dësc with no MovieFile, or a missing one, has none"
        );
        assert_eq!(
            texts[0].movies.offer.as_ref().map(|movie| movie.after),
            Some(true)
        );
    }

    #[test]
    fn the_quick_briefs_movie_is_decoded() {
        use nova_data::records::mission::Mission;
        let data = store_named(&[
            (
                Mission::TYPE,
                130,
                Some("Filmed"),
                texted([-1, 201, -1, -1, -1, -1, -1, -1], b"\0", b"\0"),
            ),
            (Desc::TYPE, 201, None, filmed("Quick.", "Quick.mov", 0x0001)),
        ]);
        assert_eq!(
            data.mission_texts()[0].movies.quick_brief,
            DescMovie::from_desc("Quick.mov", 0x0001),
            "the info screen loads it (@0x9d89f)"
        );
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
    fn a_resource_name_is_shown_up_to_its_last_semicolon_spaces_and_semicolons_before_it_dropped() {
        assert_eq!(
            shown_resource_name("Delivery to <DST>; Vellos1 "),
            "Delivery to <DST>"
        );
        assert_eq!(shown_resource_name("No cut  "), "No cut  ");
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

    /// A `sÿst` with this `ReinfFleet`, `ReinfTime` and `ReinfIntrval`.
    fn reinforced(fleet: i16, time: i16, interval: i16) -> Vec<u8> {
        let mut bytes = system();
        put_i16s(&mut bytes, 0x196, &[fleet, time, interval]);
        bytes
    }

    #[test]
    fn a_systems_reinforcements_are_its_fleet_above_127_its_time_and_an_interval_of_at_least_1() {
        let data = store(&[
            (System::TYPE, 128, reinforced(129, 900, 2)),
            (System::TYPE, 129, reinforced(128, -5, 0)),
            (System::TYPE, 130, reinforced(127, 900, 2)),
            (System::TYPE, 131, reinforced(0, 900, 2)),
            (System::TYPE, 132, reinforced(-1, 900, 2)),
            (System::TYPE, 133, reinforced(140, 30, -3)),
            (System::TYPE, 134, short(reinforced(129, 900, 2))),
        ]);
        let reinforcements = |id| data.system_reinforcements(SystemId(id));
        assert_eq!(
            reinforcements(128),
            Some(SystemReinforcements {
                fleet: FleetId(129),
                time: 900,
                interval: 2,
            })
        );
        assert_eq!(
            reinforcements(129),
            Some(SystemReinforcements {
                fleet: FleetId(128),
                time: -5,
                interval: 1,
            }),
            "the time raw, an interval of 0 read as 1 (@0x782df-0x782e9)"
        );
        assert_eq!(
            reinforcements(133).map(|it| it.interval),
            Some(1),
            "a negative interval read as 1"
        );
        for none in [130, 131, 132] {
            assert_eq!(
                reinforcements(none),
                None,
                "127 or below names none (@0x782cb)"
            );
        }
        assert_eq!(reinforcements(134), None, "undecodable");
        assert_eq!(reinforcements(135), None, "missing");
    }

    /// A `shän` with this `BlinkMode` and `BlinkValA` to `BlinkValD`.
    fn blinking(mode: i16, a: i16, b: i16, c: i16, d: i16) -> Vec<u8> {
        ShipAnimBuilder::new()
            .blink_mode(mode)
            .blink_val_a(a)
            .blink_val_b(b)
            .blink_val_c(c)
            .blink_val_d(d)
            .build()
    }

    #[test]
    fn a_blink_is_its_shäns_blink_fields() {
        let bytes = blinking(2, -3, 75, 31, 85);
        let shan = nova_data::decode_bytes::<ShipAnim>(&bytes)
            .expect("decodes")
            .record;
        assert_eq!(
            blink_of(&shan),
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
        let shan = blinking(1, 4, 1, 2, 20);
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
