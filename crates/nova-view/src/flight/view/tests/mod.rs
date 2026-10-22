use std::cell::RefCell;
use std::num::NonZeroU16;

use crate::system::camera::{VIEW_CENTER, VIEW_SIZE};
use nova_sim::flight::player_step;
use nova_sim::landing::StellarFlags;
use nova_sim::{
    CharacterStart, CommodityStrings, DateAffixes, DisasterRecord, FlightRules, GateSite, Handling,
    JunkRecord, LandingSite, OutfitId, OutfitRecord, Reserves, ShipFields, ShipId, ShipRecord,
    ShipStats, SimDiagnostic, SimSound, StarSystem, StartDate, StartError, SystemId, TICK, Vec2,
};

use super::*;
use crate::devtools::{PilotDesk, PilotEdit};
use crate::draw::crossed_box;
use crate::draw::fill_command;
use crate::draw::lights_tint;
use crate::flight::cadence::{NAV_PERIOD_TICKS, TARGET_PERIOD_TICKS};
use crate::flight::catalog::{
    Blink, BoomLook, GovtId, LayerSheet, StarSheet, StatusBarLayout, TargetCard, WeaponLook,
};
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::escorts::{EscortMenuLooks, GROUP_KEYS, MENU_KEY, NO_ESCORTS};
use crate::flight::hud::{self, HudState, NavDisplay, StatusBar};
use crate::flight::jump::JumpPhase;
use crate::flight::radar::RadarStellar;
use crate::flight::radar::{self, IRect, RadarColor, RadarMark};
use crate::flight::sprite::rotation_frame;
use crate::flight::target::Tag;
use crate::flight::weapons;
use crate::galaxy::GalaxyCatalog;
use crate::galaxy::{Galaxy, MapMode, SystemEntry};
use crate::sound::Sound;
use crate::spaceport::PortDialog;
use crate::system::scene::PLACEHOLDER;
use crate::system::{scene, starfield};
use crate::time::sixtieths;
use crate::ui::PlunderShown;
use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::{DrawList, ImageKey, Input, MouseButton, Screen, ScreenAction};
use nova_sim::CommCatalog;
use nova_sim::ShipIff;
use nova_sim::mission::screens::NO_ACTIVE_MISSIONS;
use nova_sim::ship_layer::SlotSprites;
use nova_sim::{
    ALERT_CICN, AbortError, AcceptError, Assigned, Assignment, AsteroidCatalog, CombatCatalog,
    ComputerRefusal, Controls, FighterNote, MissionId, MissionShown, NotOffered, Pilot,
    PilotCatalog, PlayerCommand, RechargeRefusal, Rulebook, Rules, StellarIff, StellarPick, Take,
    Taken, TrafficCatalog, Turn, VenueOffer, lights_level,
};

use crate::system::catalog::{
    AnimationData, StellarContents, StellarId, StellarSheet, SystemContents,
};
use crate::{Blend, DrawCommand, Font};
use nova_sim::hyperspace::MIN_JUMP_DISTANCE;
use nova_sim::{BlinkChance, GLOW_CRUISE, HashedRolls, glow_level};
use nova_sim::{BoardRefusal, DockKind, MissionBoardLine};
use nova_sim::{RuleKey, RuleSource};

mod anchor;
mod boarding;
mod cloak;
mod combat;
mod comm_line;
mod controls;
mod dialog_map;
mod docked;
mod escorts;
mod flying;
mod hyperspace;
mod landing;
mod layers;
mod map_borders;
mod map_find;
mod maps;
mod missions;
mod nav;
mod pilot;
mod self_destruct;
mod ship_drawing;
mod shots;
mod stars;
mod stellars;
mod widescreen;

/// The first `chär` flies ship 128 (an average ship that turns 3° a
/// tick, with a 36-rotation, 40 x 40 sheet, `rlëD` 2000) from system
/// 130, Sol: Earth at (0, -600) and Moon at (300, -200), which animates
/// a frame a tick. On the map Sol is at (0, 0), linked both ways to Alpha
/// Centauri (131) at (600, 0), which holds Proxima at its centre;
/// Barnard (132) at (0, 600) is linked to nothing, unless `onward`.
/// Records the systems read.
// Each flag adds a system or a link of its own, read on its own.
#[allow(clippy::struct_excessive_bools)]
struct FakeCatalog {
    character: Result<CharacterStart, StartError>,
    fields: ShipFields,
    sheet: Result<ShipSheet, String>,
    /// `ïntf` 128: stock-like, or why it cannot be read.
    bar: Result<StatusBarLayout, String>,
    /// `cicn` 18000's size: 24 x 24, by default.
    alert_icon: Option<(u32, u32)>,
    /// Each `gövt`'s `Interface`: none, by default.
    interfaces: Vec<(i16, i16)>,
    /// The `ïntf`s past 128: none, by default.
    other_bars: Vec<(i16, StatusBarLayout)>,
    /// The `ïntf`s asked for.
    bars_asked: RefCell<Vec<i16>>,
    /// System 130's landing sites: Earth and Moon, by default.
    sites: Vec<LandingSite>,
    /// System 130's `NavDefs` slots on the map: none, by default.
    sol_navs: Vec<Option<(u32, u16)>>,
    /// System 130's `Interference`: none, by default.
    sol_interference: i16,
    /// System 130's `Murk`: none, by default.
    sol_murk: i16,
    /// System 130's `BkgndColor`: black, by default.
    sol_background: u32,
    /// System 131's `Message`: no buoy (−1), by default.
    alpha_buoy: i16,
    /// The escorts' voices: none, by default.
    voices: nova_sim::voice::Voices,
    /// The stellars Sol lists after Earth and the Moon: none, by
    /// default.
    sol_more: Vec<StellarContents>,
    systems_read: RefCell<Vec<SystemId>>,
    /// The `dësc`s: none, by default.
    descriptions: Vec<(i16, String)>,
    /// The movies the `dësc`s name: none, by default.
    desc_movies: Vec<(i16, nova_sim::DescMovie)>,
    /// The commodities: none, by default.
    commodities: CommodityStrings,
    /// The planetary events: none, by default.
    disasters: Vec<DisasterRecord>,
    /// The outfits: none, by default.
    outfits: Vec<OutfitRecord>,
    /// The `jünk`s: none, by default.
    junk: Vec<JunkRecord>,
    /// The ship classes the shipyard reads: none, by default.
    ships: Vec<ShipRecord>,
    /// The ships whose sheets were asked for.
    sheets_asked: RefCell<Vec<ShipId>>,
    /// Each system's traffic: none, by default.
    traffic: Vec<(SystemId, nova_sim::SystemTraffic)>,
    /// The düdes: none, by default.
    dudes: Vec<(nova_sim::DudeId, nova_sim::DudeRecord)>,
    /// The weapons: none, by default.
    weapons: Vec<nova_sim::WeaponRecord>,
    /// The ship types' combat fields: none, by default.
    hulls: Vec<nova_sim::HullRecord>,
    /// The governments: none, by default.
    govts: Vec<nova_sim::GovtRecord>,
    /// The ranks: none, by default.
    ranks: Vec<nova_sim::catalog::RankRecord>,
    /// The weapons' looks: none, by default.
    looks: Vec<(i16, Result<WeaponLook, String>)>,
    /// The explosions' looks: none, by default.
    booms: Vec<(i16, Result<BoomLook, String>)>,
    /// The ship types' target cards: none, by default.
    cards: Vec<(i16, TargetCard)>,
    /// The governments' target codes: none, by default.
    codes: Vec<(i16, String)>,
    /// The jettisoned cargo box's sheet: `rlëD` 500 of 36 frames, by
    /// default.
    box_sheet: Result<EffectSheet, String>,
    /// The mouse cursor's sheet: `rlëD` 651 of 8 frames, by default.
    cursor_sheet: Result<EffectSheet, String>,
    /// The stars' sheet: none, by default, so no star is drawn
    /// ([`star_sheet`] draws them).
    star_sheet: Result<StarSheet, String>,
    /// The asteroid types: none, by default.
    asteroid_types: Vec<nova_sim::AsteroidTypeRecord>,
    /// Each system's asteroid field: none, by default.
    asteroid_fields: Vec<nova_sim::AsteroidFieldRecord>,
    /// The string lists: none, by default.
    strings: Vec<(i16, Vec<String>)>,
    /// The persons: none, by default.
    persons: Vec<nova_sim::PersonRecord>,
    /// Ship 128's default items: none, by default.
    defaults: Vec<(OutfitId, u16)>,
    /// Ship 129's sheet in place of the one [`FakeCatalog::ship_sheet`]
    /// gives: none, by default.
    npc_sheet: Option<ShipSheet>,
    /// Whether Alpha Centauri links on to Barnard, which has no
    /// stellars: not by default.
    onward: bool,
    /// Whether Sol lists Barnard too, after Alpha Centauri, so that
    /// Hyper Select has two systems to cycle through: not by default.
    fan: bool,
    /// Whether Proxima Near (134) exists at (40, 30), close enough to
    /// Sol to show on the floating map, listed by Sol after its other
    /// links and linking back, with no stellars: not by default.
    near: bool,
    /// Every stellar a gate may lead to: none, by default.
    gates: Vec<GateSite>,
    /// The gates each system lists beyond its stellars: none, by
    /// default.
    gate_listers: Vec<(SystemId, StellarId)>,
    /// Whether Alpha Reborn (133) exists, at Alpha Centauri's place,
    /// with no links and Proxima Reborn (141) at its centre: not by
    /// default.
    reborn: bool,
    /// Whether system 128 exists, the escape pod's rescue's fallback,
    /// with no links and no stellars: not by default.
    home: bool,
    /// Whether the map's catalog has nebula 128, (−20, −20) 60 × 60 over
    /// Sol, with a 30 × 30 picture in slot 0 (`PICT` 9500) and a
    /// 120 × 120 one in slot 2 (9502): not by default. The session's
    /// catalog has no `nëbu`, so it is active, and unexplored until a
    /// test explores it.
    nebula: bool,
    /// The stellars that can be destroyed and regenerated: none, by
    /// default.
    lives: Vec<nova_sim::StellarLife>,
    /// The missions: none, by default.
    missions: Vec<nova_sim::catalog::MissionRecord>,
    /// What the mission screens show of each: none, by default.
    mission_texts: Vec<nova_sim::catalog::MissionTexts>,
    /// Each system's `Visibility`: none, by default, so every system
    /// is visible.
    visibility: Vec<nova_sim::catalog::SystemVisibility>,
    /// The session's `spöb` and `sÿst` names: none, by default.
    place_names: nova_sim::catalog::PlaceNames,
    /// The `crön`s: none, by default.
    crons: Vec<nova_sim::catalog::CronRecord>,
    /// Each `bööm`'s sound, as the session reads it: none, by default.
    boom_sounds: Vec<(nova_sim::BoomId, nova_sim::SoundId)>,
    /// Each system's government, in the session's map and the course
    /// map's catalog: none, by default.
    governed: Vec<(i16, i16)>,
    /// The systems besides Sol with a lived-in `NavDefs` slot on the
    /// map: none, by default.
    lived: Vec<i16>,
}

/// No strings unless a test sets them.
impl nova_sim::CommCatalog for FakeCatalog {
    fn string_list(&self, id: i16) -> Vec<String> {
        self.strings
            .iter()
            .find(|(list, _)| *list == id)
            .map_or_else(Vec::new, |(_, strings)| strings.clone())
    }
}

impl FakeCatalog {
    /// System `id`'s government, as [`FakeCatalog::governed`] says.
    fn govt_of(&self, id: i16) -> Option<GovtId> {
        self.governed
            .iter()
            .find(|&&(system, _)| system == id)
            .map(|&(_, govt)| GovtId(govt))
    }

    /// Sol's links: Alpha Centauri, then Barnard with `fan`, then
    /// Proxima Near with `near`.
    fn sol_links(&self) -> Vec<i16> {
        let mut links = vec![131];
        if self.fan {
            links.push(132);
        }
        if self.near {
            links.push(134);
        }
        links
    }
}

type View = FlightView<FakeCatalog>;

const FIELDS: ShipFields = ShipFields {
    speed: 300,
    accel: 300,
    maneuver: 30,
    shield: 40,
    armor: 60,
    fuel: 250,
    fuel_regen: 0,
    holds: 0,
    mass: 15,
    free_mass: 8,
    contribute: 1,
    shield_rech: 0,
    armor_rech: 0,
    flags: 0,
    flags2: 0,
    ionize_max: 0,
    deionize: 0,
};

fn sheet() -> ShipSheet {
    ShipSheet {
        weapon: None,
        weap_decay: 0,
        image_id: 2000,
        rotations: NonZeroU16::new(36).expect("non-zero"),
        frame_width: 40,
        frame_height: 40,
        glow: None,
        lights: None,
        alt: None,
        shield: None,
        base_transp: 0,
        solid: false,
    }
}

fn catalog() -> FakeCatalog {
    FakeCatalog {
        character: Ok(CharacterStart {
            ship: Some(ShipId(128)),
            systems: [None, Some(SystemId(130)), None, None],
            start: StartDate {
                day: 23,
                month: 6,
                year: 1177,
            },
            ..CharacterStart::default()
        }),
        fields: FIELDS,
        sheet: Ok(sheet()),
        bar: Ok(layout()),
        alert_icon: Some((24, 24)),
        interfaces: Vec::new(),
        other_bars: Vec::new(),
        bars_asked: RefCell::default(),
        sites: vec![
            site(128, (0.0, -600.0), StellarFlags::CAN_LAND),
            site(129, (300.0, -200.0), StellarFlags::CAN_LAND),
        ],
        systems_read: RefCell::default(),
        descriptions: Vec::new(),
        desc_movies: Vec::new(),
        commodities: CommodityStrings::default(),
        disasters: Vec::new(),
        outfits: Vec::new(),
        junk: Vec::new(),
        ships: Vec::new(),
        sheets_asked: RefCell::default(),
        traffic: Vec::new(),
        dudes: Vec::new(),
        weapons: Vec::new(),
        hulls: vec![hull_of(128, &[]), hull_of(129, &[]), hull_of(130, &[])],
        govts: Vec::new(),
        ranks: Vec::new(),
        looks: Vec::new(),
        booms: Vec::new(),
        cards: Vec::new(),
        codes: Vec::new(),
        box_sheet: Ok(EffectSheet {
            frame_width: 0,
            image_id: 500,
            frames: NonZeroU16::new(36).expect("non-zero"),
        }),
        cursor_sheet: Ok(EffectSheet {
            frame_width: 0,
            image_id: 651,
            frames: NonZeroU16::new(8).expect("non-zero"),
        }),
        star_sheet: Err("no spïn 700".to_owned()),
        asteroid_types: Vec::new(),
        asteroid_fields: Vec::new(),
        strings: Vec::new(),
        persons: Vec::new(),
        defaults: Vec::new(),
        onward: false,
        npc_sheet: None,
        fan: false,
        near: false,
        reborn: false,
        home: false,
        nebula: false,
        sol_navs: Vec::new(),
        sol_interference: 0,
        sol_murk: 0,
        sol_background: 0,
        alpha_buoy: -1,
        voices: nova_sim::voice::Voices::default(),
        sol_more: Vec::new(),
        gates: Vec::new(),
        gate_listers: Vec::new(),
        lives: Vec::new(),
        missions: Vec::new(),
        mission_texts: Vec::new(),
        visibility: Vec::new(),
        place_names: nova_sim::catalog::PlaceNames::default(),
        crons: Vec::new(),
        boom_sounds: Vec::new(),
        governed: Vec::new(),
        lived: Vec::new(),
    }
}

/// Stellar `id` at `(x, y)`, 20 x 20 (radius 10), with `flags`.
fn site(id: i16, (x, y): (f32, f32), flags: u32) -> LandingSite {
    LandingSite {
        id: StellarId(id),
        position: Vec2::new(x, y),
        frame_size: Some((20, 20)),
        flags,
        min_status: 0,
        tech_level: 1,
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
        gravity: 0,
        animation: nova_sim::catalog::SiteAnimation::default(),
    }
}

/// The start's date, with stock's affixes.
const DATE: &str = "June 23, 1177 NC";

/// Stock `ïntf` 128's areas, with background `PICT` 700.
fn layout() -> StatusBarLayout {
    let rect = |left, top, right, bottom| crate::geometry::Bounds {
        min: at(left, top),
        max: at(right, bottom),
    };
    StatusBarLayout {
        radar: rect(8.0, 8.0, 184.0, 184.0),
        shield: rect(35.0, 199.0, 184.0, 206.0),
        armor: rect(35.0, 216.0, 184.0, 223.0),
        fuel: rect(35.0, 234.0, 184.0, 241.0),
        nav: rect(8.0, 254.0, 184.0, 286.0),
        weap: rect(8.0, 300.0, 184.0, 315.0),
        targ: rect(8.0, 330.0, 184.0, 442.0),
        cargo: rect(8.0, 458.0, 184.0, 552.0),
        bright_text: Color::WHITE,
        dim_text: Color::DIM,
        bright_radar: Color::rgba(0, 255, 0, 255),
        dim_radar: Color::rgba(0, 128, 0, 255),
        shield_color: Color::rgba(0, 0, 255, 255),
        armor_color: Color::rgba(255, 0, 0, 255),
        fuel_full: Color::rgba(255, 255, 0, 255),
        fuel_partial: Color::rgba(128, 128, 0, 255),
        font: Font::Geneva,
        font_size: 12.0,
        subtitle_size: 10.0,
        status_bkgnd: 700,
    }
}

impl PilotCatalog for FakeCatalog {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        self.character.clone()
    }

    fn voices(&self) -> nova_sim::voice::Voices {
        self.voices.clone()
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        assert_eq!(id, ShipId(128));
        Ok(self.fields)
    }

    fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
        assert_eq!(id, ShipId(128));
        self.defaults.clone()
    }

    fn outfits(&self) -> Vec<OutfitRecord> {
        self.outfits.clone()
    }

    fn ships(&self) -> Vec<ShipRecord> {
        self.ships.clone()
    }

    /// Sol and Alpha Centauri; Barnard, which has no stellars, only
    /// when Alpha Centauri links on to it.
    fn system_exists(&self, id: SystemId) -> bool {
        matches!(id.0, 130 | 131)
            || (id.0 == 132 && self.onward)
            || (id.0 == 133 && self.reborn)
            || (id.0 == 128 && self.home)
            || (id.0 == 134 && self.near)
    }

    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
        match system.0 {
            130 => self.sites.clone(),
            131 => vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)],
            132 if self.onward || self.fan => Vec::new(),
            133 if self.reborn => vec![site(141, (0.0, 0.0), StellarFlags::CAN_LAND)],
            128 if self.home => Vec::new(),
            134 if self.near => Vec::new(),
            other => panic!("asked for sÿst {other}'s sites"),
        }
    }

    fn star_map(&self) -> Vec<StarSystem> {
        let star = |id, (x, y), links: &[i16]| StarSystem {
            id: SystemId(id),
            position: Vec2::new(x, y),
            links: links.iter().copied().map(SystemId).collect(),
            govt: None,
            stellars: Vec::new(),
            interference: 0,
            murk: 0,
            message: -1,
            background: 0,
        };
        let onward: &[i16] = if self.onward { &[130, 132] } else { &[130] };
        let sol = self.sol_links();
        let mut systems = vec![
            StarSystem {
                stellars: self.sol_navs.clone(),
                interference: self.sol_interference,
                murk: self.sol_murk,
                background: self.sol_background,
                ..star(130, (0.0, 0.0), &sol)
            },
            StarSystem {
                message: self.alpha_buoy,
                ..star(131, (600.0, 0.0), onward)
            },
            star(132, (0.0, 600.0), &[]),
        ];
        if self.reborn {
            systems.push(star(133, (600.0, 0.0), &[]));
        }
        if self.home {
            systems.push(star(128, (-600.0, 0.0), &[]));
        }
        if self.near {
            systems.push(star(134, (40.0, 30.0), &[130]));
        }
        for system in &mut systems {
            system.govt = self.govt_of(system.id.0);
            if self.lived.contains(&system.id.0) {
                system.stellars = vec![Some((StellarFlags::CAN_LAND, 0))];
            }
        }
        systems
    }

    fn commodity_strings(&self) -> CommodityStrings {
        self.commodities.clone()
    }

    fn string_patch(&self, _id: i16) -> nova_sim::catalog::StringPatch {
        nova_sim::catalog::StringPatch::Absent
    }

    /// [`FakeCatalog::strings`]' `STR#` 1000.
    fn buoy_strings(&self) -> Vec<String> {
        self.string_list(nova_sim::buoy::BUOY_STRINGS)
    }

    fn junk(&self) -> Vec<JunkRecord> {
        self.junk.clone()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        self.disasters.clone()
    }

    fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
        Vec::new()
    }

    /// Stock's: no prefix, and " NC".
    fn date_affixes(&self) -> DateAffixes {
        DateAffixes {
            prefix: String::new(),
            suffix: " NC".to_owned(),
        }
    }

    fn gate_sites(&self) -> Vec<GateSite> {
        self.gates.clone()
    }

    fn stellar_lives(&self) -> Vec<nova_sim::StellarLife> {
        self.lives.clone()
    }

    fn fallback_frame_size(&self) -> Option<(u32, u32)> {
        Some((100, 100))
    }

    fn missions(&self) -> Vec<nova_sim::catalog::MissionRecord> {
        self.missions.clone()
    }

    fn mission_texts(&self) -> Vec<nova_sim::catalog::MissionTexts> {
        self.mission_texts.clone()
    }

    /// Every stellar of each system's [`landing_sites`], with every
    /// system listing it, and the gates of `gate_listers`.
    fn listed_stellars(&self) -> Vec<nova_sim::catalog::ListedStellar> {
        let mut systems = vec![SystemId(130), SystemId(131)];
        if self.onward {
            systems.push(SystemId(132));
        }
        if self.reborn {
            systems.push(SystemId(133));
        }
        if self.near {
            systems.push(SystemId(134));
        }
        let sites: Vec<(SystemId, Vec<LandingSite>)> = systems
            .into_iter()
            .map(|system| (system, self.landing_sites(system)))
            .collect();
        let mut listed = nova_sim::fixture::listed_stellars(
            sites
                .iter()
                .map(|(system, sites)| (*system, sites.as_slice())),
        );
        nova_sim::fixture::list_also(&mut listed, self.gate_listers.iter().copied());
        listed
    }

    fn system_visibility(&self) -> Vec<nova_sim::catalog::SystemVisibility> {
        self.visibility.clone()
    }

    fn place_names(&self) -> nova_sim::catalog::PlaceNames {
        self.place_names.clone()
    }

    fn stellar_marks(&self) -> Vec<nova_sim::catalog::StellarMark> {
        Vec::new()
    }

    fn crons(&self) -> Vec<nova_sim::catalog::CronRecord> {
        self.crons.clone()
    }

    fn boom_sounds(&self) -> Vec<(nova_sim::BoomId, nova_sim::SoundId)> {
        self.boom_sounds.clone()
    }

    fn ranks(&self) -> Vec<nova_sim::catalog::RankRecord> {
        self.ranks.clone()
    }

    fn nebulae(&self) -> Vec<nova_sim::catalog::NebulaRecord> {
        Vec::new()
    }
}

/// The weapons, ship types' combat fields and governments given.
impl CombatCatalog for FakeCatalog {
    fn weapons(&self) -> Vec<nova_sim::WeaponRecord> {
        self.weapons.clone()
    }

    fn hulls(&self) -> Vec<nova_sim::HullRecord> {
        self.hulls.clone()
    }

    fn governments(&self) -> Vec<nova_sim::GovtRecord> {
        self.govts.clone()
    }
}

/// The asteroid types and fields given.
impl AsteroidCatalog for FakeCatalog {
    fn asteroid_types(&self) -> Vec<nova_sim::AsteroidTypeRecord> {
        self.asteroid_types.clone()
    }

    fn asteroid_fields(&self) -> Vec<nova_sim::AsteroidFieldRecord> {
        self.asteroid_fields.clone()
    }

    fn box_frame_sizes(&self) -> [Option<(u32, u32)>; nova_sim::BOX_GRAPHICS] {
        [None; nova_sim::BOX_GRAPHICS]
    }
}

/// The traffic, düdes and persons given; no fleets.
impl TrafficCatalog for FakeCatalog {
    fn system_traffic(&self, id: SystemId) -> Option<nova_sim::SystemTraffic> {
        self.traffic
            .iter()
            .find(|(system, _)| *system == id)
            .map(|(_, traffic)| *traffic)
    }

    fn dude(&self, id: nova_sim::DudeId) -> Option<nova_sim::DudeRecord> {
        self.dudes
            .iter()
            .find(|(dude, _)| *dude == id)
            .map(|(_, record)| record.clone())
    }

    fn fleets(&self) -> Vec<nova_sim::FleetRecord> {
        Vec::new()
    }

    fn persons(&self) -> Vec<nova_sim::PersonRecord> {
        self.persons.clone()
    }
}

fn stellar(id: i16, name: &str, (x, y): (i16, i16), frames: u16) -> StellarContents {
    StellarContents {
        id: StellarId(id),
        name: name.to_owned(),
        x,
        y,
        sprite: Ok(StellarSheet {
            image_id: 1000 + id,
            frames: NonZeroU16::new(frames).expect("non-zero"),
            frame_width: 20,
            frame_height: 20,
        }),
        dead_sprite: None,
        animation: AnimationData {
            delay: 1,
            frame0_bias: 0,
            only_when_destroyed: false,
        },
        radar_square: false,
    }
}

/// Graphic type 0's sheet (`spïn` 1000) in every system: one 40 x 40
/// frame.
const TYPE_ZERO: i16 = 3000;

/// `stellar`, drawing `DeadType` sheet 2000 + `id` (one 30 x 30 frame)
/// while destroyed.
fn dying(id: i16, name: &str, at: (i16, i16), frames: u16) -> StellarContents {
    StellarContents {
        dead_sprite: Some(Ok(StellarSheet {
            image_id: 2000 + id,
            frames: NonZeroU16::MIN,
            frame_width: 30,
            frame_height: 30,
        })),
        ..stellar(id, name, at, frames)
    }
}

impl SystemCatalog for FakeCatalog {
    fn system(&self, id: SystemId) -> SystemContents {
        self.systems_read.borrow_mut().push(id);
        let (name, stellars) = match id.0 {
            130 => (
                "Sol",
                [
                    dying(128, "Earth", (0, -600), 1),
                    stellar(129, "Moon", (300, -200), 4),
                ]
                .into_iter()
                .chain(self.sol_more.iter().cloned())
                .collect(),
            ),
            131 => ("Alpha Centauri", vec![dying(140, "Proxima", (0, 0), 1)]),
            132 if self.onward => ("Barnard", Vec::new()),
            133 if self.reborn => (
                "Alpha Reborn",
                vec![stellar(141, "Proxima Reborn", (0, 0), 1)],
            ),
            128 if self.home => ("Home", Vec::new()),
            134 if self.near => ("Proxima Near", Vec::new()),
            other => panic!("asked for sÿst {other}"),
        };
        SystemContents {
            id,
            name: name.to_owned(),
            stellars,
            type_zero_sprite: Ok(StellarSheet {
                image_id: TYPE_ZERO,
                frames: NonZeroU16::MIN,
                frame_width: 40,
                frame_height: 40,
            }),
            problems: Vec::new(),
        }
    }
}

impl GalaxyCatalog for FakeCatalog {
    fn galaxy(&self) -> Galaxy {
        let entry = |id, name: &str, (x, y), links: &[i16]| SystemEntry {
            id: SystemId(id),
            name: name.to_owned(),
            x,
            y,
            links: links.iter().copied().map(SystemId).collect(),
            govt: None,
            stellars: Vec::new(),
        };
        let onward: &[i16] = if self.onward { &[130, 132] } else { &[130] };
        let sol = self.sol_links();
        let mut systems = vec![
            entry(130, "Sol", (0, 0), &sol),
            entry(131, "Alpha Centauri", (600, 0), onward),
            entry(132, "Barnard", (0, 600), &[]),
        ];
        if self.reborn {
            systems.push(entry(133, "Alpha Reborn", (600, 0), &[]));
        }
        if self.near {
            systems.push(entry(134, "Proxima Near", (40, 30), &[130]));
        }
        if self.home {
            systems.push(entry(128, "Home", (-300, 200), &[]));
        }
        let picture = |slot: u8, side| crate::galaxy::NebulaPicture {
            id: 9500 + i16::from(slot),
            slot,
            width: side,
            height: side,
        };
        let nebulae = if self.nebula {
            vec![crate::galaxy::NebulaEntry {
                id: nova_sim::NebulaId(128),
                name: "Sol Nebula".to_owned(),
                left: -20,
                top: -20,
                width: 60,
                height: 60,
                pictures: vec![picture(0, 30), picture(2, 120)],
            }]
        } else {
            Vec::new()
        };
        for system in &mut systems {
            system.govt = self.govt_of(system.id.0);
        }
        Galaxy {
            systems,
            nebulae,
            ..Galaxy::default()
        }
    }
}

impl ShipSprites for FakeCatalog {
    /// Ship 128's sheet is [`FakeCatalog::sheet`]; ship 129's is
    /// `rlëD` 2001's, with 72 rotations, glow `rlëD` 2101, lights
    /// `rlëD` 2201 and weapon glow `rlëD` 2301 of `WeapDecay` 3; ship 130's cannot be read, nor can the escape
    /// pod's, 895.
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
        self.sheets_asked.borrow_mut().push(id);
        match id.0 {
            128 => self.sheet.clone(),
            895 => Err("no shän 895".to_owned()),
            129 if self.npc_sheet.is_some() => self.npc_sheet.ok_or_else(String::new),
            129 => Ok(ShipSheet {
                image_id: 2001,
                rotations: NonZeroU16::new(72).expect("non-zero"),
                glow: Some(layer(2101, 72)),
                lights: Some(layer(2201, 72)),
                weapon: Some(layer(2301, 72)),
                weap_decay: 3,
                ..sheet()
            }),
            130 => Err("no shän 130".to_owned()),
            131 => Ok(ShipSheet {
                image_id: 2002,
                frame_width: 100,
                frame_height: 100,
                ..sheet()
            }),
            other => panic!("asked for shïp {other}'s sheet"),
        }
    }
}

/// The looks, cards and codes given; any other weapon cannot be read,
/// and there is no other explosion type. Smoke sets 0 and 1 have their
/// sprites.
impl CombatLooks for FakeCatalog {
    fn weapon_look(&self, id: nova_sim::WeaponId) -> Result<WeaponLook, String> {
        self.looks
            .iter()
            .find(|(weapon, _)| *weapon == id.0)
            .map_or_else(
                || Err(format!("no wëap {}", id.0)),
                |(_, look)| look.clone(),
            )
    }

    fn boom_look(&self, id: nova_sim::BoomId) -> Option<Result<BoomLook, String>> {
        self.booms
            .iter()
            .find(|(boom, _)| *boom == id.0)
            .map(|(_, look)| look.clone())
    }

    fn target_card(&self, ship: ShipId) -> TargetCard {
        self.cards
            .iter()
            .find(|(id, _)| *id == ship.0)
            .map(|(_, card)| card.clone())
            .unwrap_or_default()
    }

    fn target_code(&self, govt: GovtId) -> Option<String> {
        self.codes
            .iter()
            .find(|(id, _)| *id == govt.0)
            .map(|(_, code)| code.clone())
    }

    /// Graphic 0's sheet as set; graphics 1-4, an asteroid's boxes',
    /// `rlëD` 501-504 of 36 frames.
    fn box_sheet(&self, graphic: u16) -> Result<EffectSheet, String> {
        match graphic {
            0 => self.box_sheet.clone(),
            1..=4 => Ok(EffectSheet {
                frame_width: 0,
                image_id: 500 + graphic.cast_signed(),
                frames: NonZeroU16::new(36).expect("non-zero"),
            }),
            other => Err(format!("no spïn {}", 500 + other)),
        }
    }

    /// Type `kind`'s sheet: `rlëD` 1800 + `kind`, of 36 frames.
    fn asteroid_sheet(&self, kind: u8) -> Result<EffectSheet, String> {
        Ok(EffectSheet {
            frame_width: 0,
            image_id: 1800 + i16::from(kind),
            frames: NonZeroU16::new(36).expect("non-zero"),
        })
    }

    fn cursor_sheet(&self) -> Result<EffectSheet, String> {
        self.cursor_sheet.clone()
    }

    /// The escape pods' sheet: `rlëD` 1095, of 36 rotations.
    fn pod_sheet(&self) -> Result<EffectSheet, String> {
        Ok(EffectSheet {
            image_id: 1095,
            frames: NonZeroU16::new(36).expect("non-zero"),
            frame_width: 0,
        })
    }

    fn smoke_sheet(&self, set: u8) -> Option<crate::flight::catalog::SmokeSheet> {
        (set < 2).then(|| crate::flight::catalog::SmokeSheet {
            first_icon: 1000 + 8 * i16::from(set),
        })
    }

    fn star_sheet(&self) -> Result<StarSheet, String> {
        self.star_sheet.clone()
    }
}

/// The escort menu in its own colours, so tests can tell them.
impl EscortMenuLooks for FakeCatalog {
    fn escort_menu_colors(&self) -> EscortMenuColors {
        MENU_COLORS
    }
}

const MENU_COLORS: EscortMenuColors = EscortMenuColors {
    border: Color::from_rgb24(0x0012_3456),
    hilite: Color::from_rgb24(0x0065_4321),
};

/// `ïntf` 128 and the others listed; every government listed has an
/// `Interface`, and the others have no record. `PICT` 700 (128's
/// background), 705 and 128 are 194 x 767, and there are no others.
impl StatusBars for FakeCatalog {
    fn government_interface(&self, id: GovtId) -> Result<Option<i16>, String> {
        Ok(self
            .interfaces
            .iter()
            .find(|(govt, _)| *govt == id.0)
            .map(|&(_, interface)| interface))
    }

    fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
        self.bars_asked.borrow_mut().push(id);
        if id == 128 {
            return self.bar.clone();
        }
        self.other_bars
            .iter()
            .find(|(bar, _)| *bar == id)
            .map(|(_, layout)| layout.clone())
            .ok_or_else(|| format!("no ïntf {id}"))
    }

    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        [700, 705, 128].contains(&id).then_some((194, 767))
    }

    fn icon_size(&self, id: i16) -> Option<(u32, u32)> {
        (id == ALERT_CICN).then_some(self.alert_icon).flatten()
    }
}

fn flight() -> View {
    FlightView::new(catalog())
}

/// Where flight draws the player's ship in Nova's own 1024 x 768 by the
/// original's reading of [`RuleKey::FlightAnchor`]: the play area's
/// centre, `_screenCenter`, (trunc((1024 − 194) / 2), 384).
const PLAYER_AT: Point = Point::new(415.0, 384.0);

fn at(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

fn key(key: Key, pressed: bool) -> Input {
    Input::Key {
        key,
        pressed,
        repeat: false,
    }
}

fn held(key: Key) -> Input {
    Input::Key {
        key,
        pressed: true,
        repeat: true,
    }
}

fn player(view: &View) -> ShipState {
    *view.session().expect("flying").player()
}

fn handling() -> Handling {
    ShipStats::new(FIELDS, &[]).handling
}

/// The ship as it starts: at rest at the centre, facing up.
fn start() -> ShipState {
    ShipState::default()
}

fn reserves(view: &View) -> Reserves {
    view.session().expect("flying").reserves()
}

/// `state` after `ticks` of the player's steps under `controls`, capped
/// at the top speed of a pilot without Strict Play (the speed a jump
/// arrives at).
fn stepped(mut state: ShipState, controls: Controls, ticks: u32) -> ShipState {
    let top = handling().max_speed * nova_sim::top_speed::NON_STRICT_SPEED;
    for _ in 0..ticks {
        player_step(
            &mut state,
            &handling(),
            controls,
            None,
            nova_sim::flight::Caps::top(top),
            FlightRules::default(),
        );
    }
    state
}

/// The ship after `ticks` of the player's steps under `controls` from a
/// new session's start: the first held at rest by the session's box of
/// none (see `nova_sim::afterburner::MaxVel::START`), the rest as
/// [`stepped`].
fn launched(controls: Controls, ticks: u32) -> ShipState {
    let mut state = start();
    let rest = nova_sim::flight::Caps {
        thrust_top: handling().max_speed,
        max_vel: 0.0,
    };
    player_step(
        &mut state,
        &handling(),
        controls,
        None,
        rest,
        FlightRules::default(),
    );
    stepped(state, controls, ticks.saturating_sub(1))
}

/// The second half of a tick: `TICK / 2` rounds down, so this is a
/// nanosecond longer.
const REST_OF_TICK: Duration = Duration::from_nanos(16_666_667);

const THRUST: Controls = Controls {
    thrust: true,
    turn: Turn::None,
    reverse: false,
    face: nova_sim::flight::Face::Off,
    afterburner: false,
};

fn ticks(view: &mut View, n: u32) {
    for _ in 0..n {
        view.tick(TICK);
    }
}

impl DescriptionSource for FakeCatalog {
    fn description(&self, id: i16) -> Result<String, String> {
        self.descriptions
            .iter()
            .find(|(found, _)| *found == id)
            .map(|(_, text)| text.clone())
            .ok_or_else(|| format!("no dësc {id}"))
    }

    fn button_style(&self) -> crate::ui::ButtonStyle {
        crate::ui::ButtonStyle::STOCK
    }
}

impl DescriptionGraphics for FakeCatalog {
    fn description_graphic(&self, _: i16) -> Option<i16> {
        None
    }

    fn description_movie(&self, id: i16) -> Option<nova_sim::DescMovie> {
        self.desc_movies
            .iter()
            .find(|(found, _)| *found == id)
            .map(|(_, movie)| movie.clone())
    }
}
fn drawn(view: &View) -> DrawList {
    let mut list = DrawList::new();
    view.draw(&mut list);
    list
}

fn overlay(text: &str, origin: Point, size: f32, color: Color) -> DrawCommand {
    DrawCommand::Text {
        text: text.to_owned(),
        font: Font::Geneva,
        origin,
        size,
        wrap_width: None,
        color,
    }
}

/// The stars' sheet, `rlëD` 700 of 16 frames of 5 × 5, as stock's.
fn star_sheet() -> StarSheet {
    StarSheet {
        image_id: 700,
        frames: NonZeroU16::new(16).expect("non-zero"),
        size: (5, 5),
    }
}

/// [`catalog`] with the stars' sheet, so flight draws its stars.
fn starry() -> FakeCatalog {
    FakeCatalog {
        star_sheet: Ok(star_sheet()),
        ..catalog()
    }
}

/// The stars `view` draws first in its world: its starfield's, `alpha` of
/// the way between its last two snapshots, by its rules' reading of
/// [`RuleKey::FlightStarfield`].
fn drawn_stars(view: &View) -> Vec<DrawCommand> {
    let source = view.session().map_or(RuleSource::Engine, |session| {
        session.rules().source_for(RuleKey::FlightStarfield)
    });
    let mut list = DrawList::new();
    let stars = (&view.stars, &view.looks);
    super::stars::draw(&mut list, stars, source, &view.camera(), view.alpha);
    list.iter().cloned().collect()
}

/// Whether `command` is one of the stars [`star_sheet`] draws.
fn is_star(command: &DrawCommand) -> bool {
    matches!(
        command,
        DrawCommand::Sprite {
            image,
            blend: Blend::Or,
            ..
        } if image.id == 700
    )
}

/// The sprites drawn, the stars left out.
fn sprites(list: &DrawList) -> Vec<(ImageKey, Point)> {
    list.iter()
        .filter(|command| !is_star(command))
        .filter_map(|command| match command {
            DrawCommand::Sprite { image, center, .. } => Some((*image, *center)),
            _ => None,
        })
        .collect()
}

fn lines(list: &DrawList) -> Vec<DrawCommand> {
    list.iter()
        .filter(|c| matches!(c, DrawCommand::Line { .. }))
        .cloned()
        .collect()
}

/// The stock radar, with the bar at (830, 0): centred on (926, 96).
const STOCK_RADAR: IRect = IRect::new(838, 8, 1014, 184);

/// What the radar shows this frame, as marks on the stock radar.
fn radar_marks(view: &View) -> Vec<RadarMark> {
    radar::marks(
        STOCK_RADAR,
        &view.radar_shown(),
        view.hud_cadence.blink_on(),
    )
}

/// The ships' blips on the radar this frame: its pixels but the
/// player's, the last.
fn blips(view: &View) -> Vec<RadarMark> {
    let mut marks: Vec<RadarMark> = radar_marks(view)
        .into_iter()
        .filter(|mark| matches!(mark, RadarMark::Pixel { .. }))
        .collect();
    marks.pop();
    marks
}

const PLAYER_PIXEL: RadarMark = RadarMark::Pixel {
    x: 926,
    y: 96,
    color: RadarColor::Bright,
};

fn texts(list: &DrawList) -> Vec<String> {
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The port's title, and its help line, on.
const TITLED: FlightOverlays = FlightOverlays {
    title: true,
    help: true,
    cheats: false,
};
fn layer(image_id: i16, frames: u16) -> LayerSheet {
    LayerSheet {
        image_id,
        frames: NonZeroU16::new(frames).expect("non-zero"),
        solid: false,
    }
}

/// The ship's sprites drawn: those from `rlëD`s 2000 to 2299.
fn ship_sprites(view: &View) -> Vec<(ImageKey, Point)> {
    sprites(&drawn(view))
        .into_iter()
        .filter(|(image, _)| (2000..2300).contains(&image.id))
        .collect()
}

/// [`layered`], with the lights blinking by `blink`.
fn blinking(blink: Blink) -> FakeCatalog {
    FakeCatalog {
        sheet: Ok(ShipSheet {
            glow: Some(layer(2100, 36)),
            lights: Some(layer(2200, 12)),
            ..sheet()
        }),
        hulls: vec![lit_hull(hull_of(128, &[]), blink)],
        ..catalog()
    }
}

/// `hull` with a lights sprite blinking `blink`, as its `shän` names one,
/// which the session's display steps (`nova_sim::combat::ship_lights`).
fn lit_hull(hull: nova_sim::HullRecord, blink: Blink) -> nova_sim::HullRecord {
    let display = hull.display.unwrap_or(nova_sim::catalog::ShipDisplay {
        base_sets: 1,
        ..nova_sim::catalog::ShipDisplay::default()
    });
    nova_sim::HullRecord {
        display: Some(nova_sim::catalog::ShipDisplay {
            blink,
            lights_image: true,
            ..display
        }),
        ..hull
    }
}

/// `view`'s flight with [`RuleKey::HudCadence`] read the Bible's way:
/// the HUD every frame.
fn every_frame(view: View) -> View {
    view.with_rules(Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Bible))
}

/// `ïntf` 129: [`layout`] in another shield colour, over `PICT` 705.
fn bar_129() -> StatusBarLayout {
    StatusBarLayout {
        shield_color: Color::rgba(0, 255, 255, 255),
        status_bkgnd: 705,
        ..layout()
    }
}

/// `catalog` where ship type `ship` (given a record if it has none)
/// belongs to gövt 150, whose `Interface` is `interface`, and with
/// `ïntf` 129 ([`bar_129`]).
fn governed(mut catalog: FakeCatalog, ship: i16, interface: i16) -> FakeCatalog {
    if !catalog.ships.iter().any(|record| record.id == ShipId(ship)) {
        let mut record = shipbuying().ships[0].clone();
        record.id = ShipId(ship);
        catalog.ships.push(record);
    }
    for record in &mut catalog.ships {
        if record.id == ShipId(ship) {
            record.inherent_govt = Some(GovtId(150));
        }
    }
    catalog.interfaces = vec![(150, interface)];
    catalog.other_bars = vec![(129, bar_129())];
    catalog
}
/// The fake catalog with system 130 holding just `sites`.
fn flight_among(sites: Vec<LandingSite>) -> View {
    FlightView::new(FakeCatalog { sites, ..catalog() })
}

const LAND: Key = Key::Char('l');

/// The message drawn at its place, if any.
fn message(view: &View) -> Option<DrawCommand> {
    drawn(view)
        .iter()
        .find(|c| matches!(c, DrawCommand::Text { origin, .. } if *origin == message_at()))
        .cloned()
}

/// Where the message's first line is drawn: the message box's
/// top-left.
fn message_at() -> Point {
    hud::message_box(VIEW_SIZE).min
}

/// The message `text` as a flight without metrics draws it.
fn message_drawn(text: &str) -> DrawCommand {
    DrawCommand::Text {
        text: text.to_owned(),
        font: hud::MESSAGE_FONT,
        origin: at(25.0, 724.0),
        size: 12.0,
        wrap_width: Some(755.0),
        color: Color::WHITE,
    }
}

/// Asserts the message line says `text` now, for `n` ticks of flight
/// in all, and not after.
#[track_caller]
fn assert_lasts(view: &mut View, text: &str, n: u16) {
    assert_eq!(view.message(), Some(text));
    ticks(view, u32::from(n) - 1);
    assert_eq!(
        view.message(),
        Some(text),
        "still shown after {} ticks",
        n - 1
    );
    ticks(view, 1);
    assert_eq!(view.message(), None, "gone after {n} ticks");
}

/// L pressed, a tick for the dock approach, and L pressed again:
/// clearance, then landing.
fn land_now(view: &mut View) {
    tap(view, LAND);
    ticks(view, 1);
    tap(view, LAND);
}

/// The name of the ship `view` flies, as traffic control says it.
fn ship_name(view: &View) -> String {
    view.session()
        .expect("flying")
        .pilot()
        .ship_name()
        .unwrap_or_default()
        .to_owned()
}
const MAP: Key = Key::Char('m');
const JUMP: Key = Key::Char('j');

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// Presses and releases `k`.
fn tap(view: &mut View, k: Key) {
    view.input(&key(k, true));
    view.input(&key(k, false));
}

/// Taps `k`, then runs the step whose secondary block takes it (see
/// `nova_sim::session`'s `secondary_select`).
fn tap_and_step(view: &mut View, k: Key) {
    tap(view, k);
    ticks(view, 1);
}

/// Where system `id` is on the course map, brought into item 3 first: the
/// map opens at 0.5625 on the current system and shows little of the
/// galaxy.
fn on_map(view: &mut View, id: i16) -> Point {
    view.map.centre_on_system(SystemId(id));
    let map = view.course_map();
    let system = map.model().system(SystemId(id)).expect("on the map");
    map.transform().world_to_screen(system.position())
}

/// Where system `id` is drawn on the course map as it looks now.
fn on_map_now(view: &View, id: i16) -> Point {
    let map = view.course_map();
    let system = map.model().system(SystemId(id)).expect("on the map");
    map.transform().world_to_screen(system.position())
}

/// Clicks the left button at `at`.
fn click(view: &mut View, at: Point) {
    for pressed in [true, false] {
        view.input(&Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed,
            at,
        });
    }
}

/// Opens the map, Shift-clicks each hop of the shortest route to system
/// `id` and closes the map again, as `_DoSystemMap` builds a course a hop
/// at a time ([`nova_sim::RuleKey::MapPlotting`]), the close cueing its
/// first hop. The course must come out as the route: a hop past an
/// unexplored system to an unknown one is refused, so a flight that
/// plots one reads the planner ([`planner`]), where each click plots the
/// route to its system.
fn plot(view: &mut View, id: i16) {
    let session = view.session().expect("flying");
    let route = session
        .star_map()
        .route(
            session.system(),
            SystemId(id),
            nova_sim::HyperlinkRule::Engine,
        )
        .expect("a route");
    tap(view, MAP);
    view.input(&key(Key::Shift, true));
    for &hop in &route {
        let at = on_map(view, hop.0);
        click(view, at);
    }
    view.input(&key(Key::Shift, false));
    assert_eq!(view.session().expect("flying").course(), route, "plotted");
    tap(view, MAP);
    assert!(!view.map_open());
}

/// `view` reading [`nova_sim::RuleKey::MapPlotting`]'s other reading, the
/// planner: a click on the map plots the shortest course to its system
/// and cues its first hop at once.
fn planner(view: View) -> View {
    let rules = view
        .session()
        .expect("flying")
        .rules()
        .with_override(nova_sim::RuleKey::MapPlotting, nova_sim::RuleSource::Bible);
    view.with_rules(rules)
}

/// Marks system `id` known, as a map or a visit would.
fn know(view: &mut View, id: i16) {
    view.session
        .as_mut()
        .expect("flying")
        .mark_known([SystemId(id)]);
}

/// Whether the open map draws Clear Route enabled
/// (`showClearRouteButton`).
fn clear_route_enabled(view: &View) -> bool {
    view.course_map().clear_route_enabled()
}

/// Presses Shift, clicks at `at` and lets Shift go.
fn shift_click(view: &mut View, at: Point) {
    view.input(&key(Key::Shift, true));
    click(view, at);
    view.input(&key(Key::Shift, false));
}

/// Thrusts straight up until the ship is at least the minimum jump
/// distance from the centre, then lets go of Up.
fn fly_out(view: &mut View) {
    view.input(&key(Key::Up, true));
    for _ in 0..2000 {
        if player(view).position.length() >= MIN_JUMP_DISTANCE {
            break;
        }
        view.tick(TICK);
    }
    view.input(&key(Key::Up, false));
    assert!(
        player(view).position.length() >= MIN_JUMP_DISTANCE,
        "{:?}",
        player(view)
    );
}

/// The mode-3 link the open course map draws, as (from, to) on the
/// map; none while it is shut.
fn hyper_selected_links(view: &View) -> Vec<(Point, Point)> {
    if !view.map_open() {
        return Vec::new();
    }
    view.course_map()
        .picture()
        .and_then(|picture| picture.mode3_link)
        .map(|(from, to)| (on_map_now(view, from.0), on_map_now(view, to.0)))
        .into_iter()
        .collect()
}

const REFUSED: Sound = Sound::Sim(SimSound::Refused);

/// `view` begun as a new pilot's, ticked once past the land hint, its
/// message and sounds let go.
fn new_pilot(mut view: View) -> View {
    view.begin();
    ticks(&mut view, 1);
    view.message = None;
    view.take_sounds();
    view
}

/// The fade quad's place in the list and its colour, if drawn.
fn fade(list: &DrawList) -> Option<(usize, Color)> {
    list.iter().enumerate().find_map(|(at, c)| match *c {
        DrawCommand::Line {
            from, width, color, ..
        } if from == Point::new(0.0, 384.0) && width == 768.0 => Some((at, color)),
        _ => None,
    })
}

/// Where the HUD starts: its status bar picture.
fn hud_at(list: &DrawList) -> usize {
    list.iter()
        .position(
            |c| matches!(c, DrawCommand::Picture { image, .. } if *image == ImageKey::picture(700)),
        )
        .expect("the HUD")
}

/// Ship 128 carrying the stock Multi-Jumping Organ (`oütf` 275,
/// multi-jump 10), with Alpha Centauri linked on to Barnard.
fn multi_jumping() -> FakeCatalog {
    FakeCatalog {
        outfits: vec![OutfitRecord {
            id: OutfitId(275),
            name: "Multi-Jumping Organ".to_owned(),
            short_name: "Multi-Jumping Organ".to_owned(),
            disp_weight: 0,
            mass: 0,
            tech_level: 1,
            max: 1,
            flags: 0,
            cost: 0,
            mods: [(32, 10), (0, 0), (0, 0), (0, 0)],
            contribute: 0,
            require: 0,
            require_govt: -1,
            buy_random: 100,
            availability: nova_sim::Test::default(),
            item_class: 0,
            lc_name: "multi-jumping organ".to_owned(),
            lc_plural: "multi-jumping organs".to_owned(),
            scan_mask: 0,
            on_purchase: nova_sim::Script::default(),
            on_sell: nova_sim::Script::default(),
        }],
        defaults: vec![(OutfitId(275), 1)],
        onward: true,
        ..catalog()
    }
}

/// Taps J and ticks until the jump's effect begins, as the session says the
/// jump has begun.
fn jump_now(view: &mut View) {
    tap(view, JUMP);
    for _ in 0..1000 {
        if view.jump_effect().is_some() {
            return;
        }
        view.tick(TICK);
    }
    panic!("the jump never began: {:?}", player(view));
}

/// Ticks until the ship has arrived in another system and the jump's
/// effect is over.
fn arrive_now(view: &mut View) {
    let leaving = view.session().expect("flying").system();
    for _ in 0..1000 {
        let arrived = view.session().expect("flying").system() != leaving;
        if arrived && view.jump_effect().is_none() {
            return;
        }
        view.tick(TICK);
    }
    panic!("never arrived: {:?}", player(view));
}

/// Two of `build`'s flights, each with its course to Alpha Centauri
/// plotted and flown out: the first braking and turning for the jump
/// on a tap of J, the second 300 ms into the jump's warp. Boxed: two flights
/// are too large for the stack together.
fn through_a_jump(build: impl Fn() -> View) -> [Box<View>; 2] {
    let mut braking = Box::new(build());
    plot(&mut braking, 131);
    fly_out(&mut braking);
    tap(&mut braking, JUMP);
    assert!(braking.preparing_jump(), "braking");
    let mut warping = Box::new(build());
    plot(&mut warping, 131);
    fly_out(&mut warping);
    jump_now(&mut warping);
    warping.tick(ms(300));
    assert!(warping.jump_effect().is_some(), "warping");
    [braking, warping]
}

/// Which stage of the jump `view` is in, for a failing assertion.
fn stage(view: &View) -> &'static str {
    if view.preparing_jump() {
        "braking"
    } else {
        "warping"
    }
}

/// Ticks `view` until its session's `_hyperGamma` is above 0, and gives
/// how many ticks that took.
fn tick_until_lit(view: &mut View) -> u32 {
    for n in 1..2000 {
        view.tick(TICK);
        if view.session().expect("flying").hyper_gamma() > 0.0 {
            return n;
        }
    }
    panic!("never lit: {:?}", player(view));
}

/// Ticks until the ship has arrived in another system, the jump's
/// fade-in yet to play.
fn tick_to_arrival(view: &mut View) {
    let leaving = view.session().expect("flying").system();
    for _ in 0..1000 {
        if view.session().expect("flying").system() != leaving {
            return;
        }
        view.tick(TICK);
    }
    panic!("never arrived: {:?}", player(view));
}

/// Jumps from Sol to Alpha Centauri, on a course on to Barnard, and
/// stops 50 ms into the arrival's fade-in.
fn fading_in_on_course_to_barnard() -> View {
    let mut view = planner(FlightView::new(FakeCatalog {
        onward: true,
        ..catalog()
    }));
    plot(&mut view, 132);
    fly_out(&mut view);
    jump_now(&mut view);
    tick_to_arrival(&mut view);
    view.tick(ms(50));
    assert_eq!(system_of(&view), SystemId(131));
    assert!(matches!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeIn(_))
    ));
    view
}
/// A pilot named `name` docked at Earth: landed over a planet 128 at
/// the centre, then flown in the stock catalog, where Earth is at
/// (0, -600).
fn docked_pilot(name: &str) -> Pilot {
    let centred = FakeCatalog {
        sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
        ..catalog()
    };
    let pilot = Pilot::new(&centred, name).expect("starts");
    let mut view = FlightView::with_pilot(centred, pilot);
    land_now(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    view.pilot().expect("flying").clone()
}
/// A view of Ada resumed over Earth and landed there, the landing,
/// its sounds and its message taken, with the systems read so far
/// forgotten.
fn landed_view() -> View {
    let mut view = FlightView::with_pilot(catalog(), docked_pilot("Ada"));
    land_now(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    view.take_sounds();
    view.message = None;
    view.catalog().systems_read.borrow_mut().clear();
    view
}

fn move_to(view: &mut View, system: i16, stellar: i16) {
    let mut desk = view.pilot_desk().expect("flying");
    desk.edit(PilotEdit::MoveTo {
        system: SystemId(system),
        stellar: StellarId(stellar),
    })
    .expect("moves");
}
use nova_sim::{Chance, Direction, DisasterId, Good, Lot, Order, TradeRefusal};

/// Earth at the centre, a trade center trading food at 75, the first
/// `chär` holding 1000 credits, and its ship 10 tons.
fn trading() -> FakeCatalog {
    let flags = StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | 2 << 28;
    FakeCatalog {
        character: Ok(CharacterStart {
            cash: 1000,
            ..catalog().character.expect("a chär")
        }),
        fields: ShipFields {
            holds: 10,
            ..FIELDS
        },
        sites: vec![site(128, (0.0, 0.0), flags)],
        commodities: CommodityStrings {
            names: vec!["Food".to_owned()],
            name_patches: Default::default(),
            base_prices: vec!["75".to_owned()],
            price_patches: Default::default(),
        },
        ..catalog()
    }
}

const BUY_FOOD: Order = Order {
    row: 0,
    good: Good::Commodity(0),
    direction: Direction::Buy,
    lot: Lot::Click,
};

use nova_sim::order_loop::{OrderRun, Stopped};
use nova_sim::{OutfitOrder, OutfitRefusal, OutfitRun, Reselection};

/// Earth at the centre, an outfitter selling a fuel tank (+100 fuel,
/// a ton, 1000 credits), the first `chär` holding 1000 credits.
fn outfitting() -> FakeCatalog {
    FakeCatalog {
        character: Ok(CharacterStart {
            cash: 1000,
            ..catalog().character.expect("a chär")
        }),
        sites: vec![site(
            128,
            (0.0, 0.0),
            StellarFlags::CAN_LAND | StellarFlags::OUTFITTER,
        )],
        outfits: vec![OutfitRecord {
            id: OutfitId(200),
            name: "Fuel Tank".to_owned(),
            short_name: "Fuel Tank".to_owned(),
            disp_weight: 0,
            mass: 1,
            tech_level: 1,
            max: 5,
            flags: 0,
            cost: 1000,
            mods: [(12, 100), (0, 0), (0, 0), (0, 0)],
            contribute: 0,
            require: 0,
            require_govt: -1,
            buy_random: 100,
            availability: nova_sim::Test::default(),
            item_class: 0,
            lc_name: "fuel tank".to_owned(),
            lc_plural: "fuel tanks".to_owned(),
            scan_mask: 0,
            on_purchase: nova_sim::Script::default(),
            on_sell: nova_sim::Script::default(),
        }],
        ..catalog()
    }
}

const BUY_TANK: OutfitOrder = OutfitOrder {
    outfit: OutfitId(200),
    direction: Direction::Buy,
};

/// Fires every time, and records each percent it is asked; each draw
/// is 0, and records its `n`.
#[derive(Default)]
struct Always {
    asked: Vec<u8>,
    drawn: Vec<u32>,
}

impl Chance for Always {
    fn fires(&mut self, percent: u8) -> bool {
        self.asked.push(percent);
        true
    }

    fn below(&mut self, n: u32) -> u32 {
        self.drawn.push(n);
        0
    }

    /// The last of the outcomes.
    fn roll(&mut self, sides: u16) -> u16 {
        sides.saturating_sub(1)
    }
}

/// Jumps to Alpha Centauri and arrives.
fn jump_to_alpha(view: &mut View) {
    plot(view, 131);
    fly_out(view);
    view.input(&key(JUMP, true));
    arrive_now(view);
    assert_eq!(view.session().expect("flying").system(), SystemId(131));
}

use nova_sim::ShipRefusal;

/// [`outfitting`], where Earth is a shipyard too, selling ship 129 for
/// 900 credits: twice as fast, with a fuel tank.
fn shipbuying() -> FakeCatalog {
    let mut earth = site(
        128,
        (0.0, 0.0),
        StellarFlags::CAN_LAND | StellarFlags::OUTFITTER | StellarFlags::SHIPYARD,
    );
    earth.tech_level = 1;
    FakeCatalog {
        sites: vec![earth],
        ships: vec![ShipRecord {
            id: ShipId(129),
            name: "Fast".to_owned(),
            short_name: "Fast".to_owned(),
            long_name: String::new(),
            fields: ShipFields {
                speed: 600,
                ..FIELDS
            },
            defaults: vec![(OutfitId(200), 1)],
            default_slots: [None; 8],
            strength: 0,
            cost: 900,
            tech_level: 1,
            buy_random: 100,
            hire_random: 0,
            require: 0,
            availability: nova_sim::Test::default(),
            appear_on: nova_sim::Test::default(),
            flags: 0,
            flags3: 0,
            disp_weight: 0,
            max_gun: 0,
            max_tur: 0,
            length: 0,
            crew: 0,
            inherent_ai: 1,
            comm_name: String::new(),
            inherent_govt: None,
            combat_govt: None,
            escort_type: -1,
            skill_var: 0,
            on_capture: nova_sim::Script::default(),
            on_purchase: nova_sim::Script::default(),
            on_retire: nova_sim::Script::default(),
        }],
        ..outfitting()
    }
}
/// Rolls the second outcome of every roll, so `R(a b)` takes `a`.
#[derive(Debug)]
struct RollsOne;

impl Chance for RollsOne {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n.min(1)
    }
}

fn rolling_one() -> SharedChance {
    SharedChance::new(Rc::new(RefCell::new(RollsOne)))
}

fn bit_set(view: &View, n: u16) -> bool {
    let bit = nova_sim::Bit::new(n).expect("a bit");
    view.pilot().is_some_and(|pilot| pilot.control_bit(bit))
}
/// The texts drawn in the nav area of the port's status bar
/// ([`FlightView::draw_status_bar`]), as [`nav`] reads flight's.
fn port_nav(view: &View) -> Vec<String> {
    let area = layout().nav.offset(at(830.0, 0.0));
    let mut bar = DrawList::new();
    view.draw_status_bar(&mut bar);
    bar.iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, origin, .. } if area.contains(*origin) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The nav area naming `name` in mode 2.
fn naming(name: &str) -> Vec<String> {
    vec![hud::NAV_STELLAR.to_owned(), name.to_owned()]
}

/// [`shipbuying`]'s Earth, a trade center too selling food, the ship
/// holding 10 tons; ship 130 under gövt 150, whose `Interface` is 129
/// ([`governed`]); the fuel tank's `OnPurchase` an `H130`, and outfit
/// 201 a tank with none; missions 130 and 131, errands at the mission
/// computer, 131 with a `BriefText`; Barnard (132), with no stellar;
/// and Proxima (140), Alpha Centauri's first stellar, named by the
/// session alone, as the flight scene is Sol's while docked.
fn port() -> FakeCatalog {
    let mut catalog = governed(shipbuying(), 130, 129);
    catalog.sites[0].flags |= trading().sites[0].flags;
    catalog.fields.holds = 10;
    catalog.commodities = trading().commodities;
    let plain = OutfitRecord {
        id: OutfitId(201),
        ..catalog.outfits[0].clone()
    };
    catalog.outfits[0].on_purchase = nova_sim::Script::parse("H130");
    catalog.outfits.push(plain);
    catalog.missions = vec![errand(130), errand(131)];
    catalog.mission_texts = vec![MissionTexts {
        brief: Some("Go.".to_owned()),
        ..MissionTexts::named(MissionId(131), "Walk")
    }];
    catalog.onward = true;
    catalog.place_names.stellars = BTreeMap::from([(StellarId(140), "Proxima".to_owned())]);
    catalog
}

/// [`port`] with a bar on Earth, a view of it landed there by the
/// engine's reading (as [`docked_at_earth`]), with `course` plotted
/// in flight first if given, and the bar opened.
fn in_the_bar(course: Option<i16>) -> View {
    in_the_bar_of(port(), course)
}

/// [`in_the_bar`] over `catalog`, a [`port`] changed.
fn in_the_bar_of(mut catalog: FakeCatalog, course: Option<i16>) -> View {
    catalog.sites[0].flags |= StellarFlags::BAR;
    let rules = Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Engine);
    let mut view = FlightView::new(catalog).with_rules(rules);
    if let Some(to) = course {
        view.session
            .as_mut()
            .expect("flying")
            .plot_course(SystemId(to))
            .expect("a route");
    }
    land_now(&mut view);
    view.settle_script();
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(128)))),
        "the land key put mode 2 over any cue"
    );
    assert_eq!(port_nav(&view), naming("Earth"));
    assert!(view.open_bar());
    view
}

/// Clicks system `id` on the map open in port.
fn port_click(view: &mut View, id: i16) {
    let at = on_map(view, id);
    for pressed in [true, false] {
        view.port_map_input(&Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed,
            at,
        });
    }
}

/// The nav area showing Alpha Centauri (131), unexplored, in mode 3.
fn cueing_alpha() -> Vec<String> {
    vec![
        hud::NAV_HYPERSPACE.to_owned(),
        hud::NAV_UNEXPLORED.to_owned(),
    ]
}
use nova_sim::mission::screens::SPACEPORT;

/// How an accept is made: from the mission computer, from the offer
/// dialog with a refuse button, or at once with no dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Accepted {
    Computer,
    Offer,
    AtOnce,
}

/// Which of the bar's parts show as they are now: the gauges, the
/// nav area, the target panel and the radar.
fn retaken(view: &View) -> [bool; 4] {
    [
        view.reserves_shown() == view.reserves(),
        view.nav_shown() == view.nav_now(),
        *view.target_shown() == view.target_now(),
        view.radar_scene == Some(view.radar_now()),
    ]
}
/// The nav area naming `name` in mode 2, as flight shows it.
fn flashing(name: &str) -> NavDisplay {
    NavDisplay::Stellar(Some(name.to_owned()))
}

/// The first `chär`'s `OnStart` is `on_start`.
fn starting_with(on_start: &str) -> FakeCatalog {
    let base = catalog();
    FakeCatalog {
        character: Ok(CharacterStart {
            on_start: nova_sim::Script::parse(on_start),
            ..base.character.clone().expect("a chär")
        }),
        ..base
    }
}
use std::collections::VecDeque;

use nova_sim::{Goal, NpcId};

/// Draws its script, then the last outcome, so no roll fires;
/// records each `n` asked.
#[derive(Default)]
struct Script {
    draws: VecDeque<u32>,
    asked: Vec<u32>,
}

impl Chance for Script {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.asked.push(n);
        self.draws.pop_front().unwrap_or(n - 1)
    }
}

fn scripted(draws: &[u32]) -> (Rc<RefCell<Script>>, SharedChance) {
    let script = Rc::new(RefCell::new(Script {
        draws: draws.iter().copied().collect(),
        asked: Vec::new(),
    }));
    let shared: Rc<RefCell<dyn Chance>> = script.clone();
    (script, SharedChance::new(shared))
}

/// One setup pass placing the düde's ship at (`x` - 750, `y` - 750)
/// facing `heading`, of aggression 0.
fn placed(x: u32, y: u32, heading: u32) -> [u32; 8] {
    [6, 6, 0, 0, x, y, heading, 2]
}

/// Decides nothing: every NPC idles.
#[derive(Debug)]
struct Still;

impl Behaviour for Still {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::Idle
    }
}

/// [`catalog`] with `avg` ships on average in each of `systems`, all of
/// düde 128's ship `ship` with AI `ai_type`; ships 129 and 130 are
/// records the traffic can fly.
fn trafficked(systems: &[i16], avg: i16, ship: i16, ai_type: i16) -> FakeCatalog {
    let mut dude_types = [(-1, 0); 8];
    dude_types[0] = (128, 100);
    let record = |id| ShipRecord {
        id: ShipId(id),
        name: format!("Ship {id}"),
        short_name: String::new(),
        long_name: String::new(),
        fields: ShipFields {
            speed: 600,
            ..FIELDS
        },
        defaults: Vec::new(),
        default_slots: [None; 8],
        strength: 0,
        cost: 1,
        tech_level: 1,
        buy_random: 100,
        hire_random: 0,
        require: 0,
        availability: nova_sim::Test::default(),
        appear_on: nova_sim::Test::default(),
        flags: 0,
        flags3: 0,
        disp_weight: 0,
        max_gun: 0,
        max_tur: 0,
        length: 0,
        crew: 0,
        inherent_ai: 1,
        comm_name: String::new(),
        inherent_govt: None,
        combat_govt: None,
        escort_type: -1,
        skill_var: 0,
        on_capture: nova_sim::Script::default(),
        on_purchase: nova_sim::Script::default(),
        on_retire: nova_sim::Script::default(),
    };
    FakeCatalog {
        traffic: systems
            .iter()
            .map(|&id| {
                (
                    SystemId(id),
                    nova_sim::SystemTraffic {
                        dude_types,
                        avg_ships: avg,
                        persons: Default::default(),
                    },
                )
            })
            .collect(),
        dudes: vec![(
            nova_sim::DudeId(128),
            nova_sim::DudeRecord {
                ai_type,
                govt: None,
                ships: vec![(ShipId(ship), 1)],
                booty: 0,
                info_types: 0,
            },
        )],
        ships: vec![record(129), record(130)],
        ..catalog()
    }
}

fn npc_ids(view: &View) -> Vec<NpcId> {
    let session = view.session().expect("flying");
    session.npcs().iter().map(|npc| npc.id).collect()
}

/// A full cloak.
const CLOAKED: nova_sim::cloak::Cloak = nova_sim::cloak::Cloak {
    fade: nova_sim::cloak::FADE_FULL,
    direction: 0,
};

/// Turns east at rest, then drifts out slowly that way until far
/// enough to jump: facing Alpha Centauri and slow enough, J streaks at
/// once.
fn ready_to_jump(view: &mut View) {
    view.input(&key(Key::Right, true));
    ticks(view, 30);
    view.input(&key(Key::Right, false));
    view.input(&key(Key::Up, true));
    ticks(view, 10);
    view.input(&key(Key::Up, false));
    for _ in 0..2000 {
        if player(view).position.length() >= MIN_JUMP_DISTANCE {
            break;
        }
        view.tick(TICK);
    }
    assert_eq!(player(view).heading, 90.0);
}
/// Says no ship is disabled, counting the times it is asked.
#[derive(Debug, Default)]
struct Counting {
    asked: std::cell::Cell<usize>,
}

impl DisableRule for Counting {
    fn disabled(&self, _armor: nova_sim::Gauge, _hull: &nova_sim::HullSpec) -> bool {
        self.asked.set(self.asked.get() + 1);
        false
    }
}

/// Every NPC idles and holds its trigger.
#[derive(Debug)]
struct Firing;

impl Behaviour for Firing {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::Idle
    }

    fn trigger(&self, _npc: &nova_sim::Npc, _around: &nova_sim::Surroundings) -> nova_sim::Trigger {
        nova_sim::Trigger {
            primary: true,
            secondary: None,
            only: None,
            bays: false,
            bays_after: false,
            point_defence: false,
            keep: None,
        }
    }
}
use std::num::NonZeroU16 as Frames;

use nova_sim::{BoomId, Condition, PlayerFate, WeaponId};

use crate::flight::brackets::{self, Standing};
use crate::flight::catalog::EffectSheet;
use crate::flight::effects::{DEBRIS_COLOR, Effects};
use crate::flight::target::{self, NO_SECONDARY, NO_TARGET};
use crate::flight::weapons::{BEAM_UNDER_SHIPS, translucent};
use crate::sound::CombatSound;

const BLASTER: WeaponId = WeaponId(128);
const ROCKET: WeaponId = WeaponId(140);
const MISSILE: WeaponId = WeaponId(141);
const TORCH: WeaponId = WeaponId(142);
const UNDER: WeaponId = WeaponId(146);
const OVER: WeaponId = WeaponId(147);
const UNSEEN: WeaponId = WeaponId(150);

/// `wëap` `id` firing every tick, 20 pixels a tick for 30 ticks, doing
/// no damage, with `flags`, `guidance` and `explod_type`.
fn gun(id: WeaponId, flags: u16, guidance: i16, explod_type: i16) -> nova_sim::WeaponRecord {
    nova_sim::WeaponRecord {
        id,
        reload: 0,
        count: 30,
        mass_dmg: 0,
        energy_dmg: 0,
        guidance,
        speed: 2000,
        ammo_type: -1,
        graphic: 0,
        inaccuracy: 0,
        impact: 0,
        explod_type,
        prox_radius: 0,
        blast_radius: 0,
        flags,
        seeker: 0,
        flags2: 0,
        flags3: 0,
        decay: 0,
        beam_length: 50,
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
    }
}

/// Ship `id` carrying one of each of `weapons`, gone at once in
/// `bööm` 128.
fn hull_of(id: i16, weapons: &[WeaponId]) -> nova_sim::HullRecord {
    nova_sim::HullRecord {
        id: ShipId(id),
        flags: 0,
        flags2: 0,
        flags3: 0,
        death_delay: 0,
        pod_count: 0,
        explode1: -1,
        explode2: 0,
        mass: 0,
        weapons: weapons
            .iter()
            .map(|&weapon| nova_sim::StockWeapon {
                weapon,
                count: 1,
                ammo: 0,
            })
            .collect(),
        size: None,
        height: None,
        strength: 0,
        key_carried: None,
        cost: 0,
        speed: 0,
        exits: None,
        // Steady lights, drawn where its sheet has a lights layer.
        display: Some(nova_sim::catalog::ShipDisplay {
            base_sets: 1,
            lights_image: true,
            ..nova_sim::catalog::ShipDisplay::default()
        }),
        escort_class: nova_sim::EscortClass::Freighter,
    }
}

/// `oütf` `id`, "Gun `id`", holding a `weapon`: a ton, 1000 credits.
fn holding(id: i16, weapon: WeaponId) -> OutfitRecord {
    OutfitRecord {
        id: OutfitId(id),
        name: format!("Gun {id}"),
        short_name: format!("Gun {id}"),
        disp_weight: 0,
        mass: 1,
        tech_level: 1,
        max: 10,
        flags: 0,
        cost: 1000,
        mods: [
            (nova_sim::combat::armament::MOD_WEAPON, weapon.0),
            (0, 0),
            (0, 0),
            (0, 0),
        ],
        contribute: 0,
        require: 0,
        require_govt: -1,
        buy_random: 100,
        availability: nova_sim::Test::default(),
        item_class: 0,
        lc_name: format!("gun {id}"),
        lc_plural: format!("guns {id}"),
        scan_mask: 0,
        on_purchase: nova_sim::Script::default(),
        on_sell: nova_sim::Script::default(),
    }
}

fn effect_sheet(image_id: i16, frames: u16) -> EffectSheet {
    EffectSheet {
        frame_width: 0,
        image_id,
        frames: Frames::new(frames).expect("non-zero"),
    }
}

/// A look named `name` sounding `snd ` `sound`, its shots on `rlëD`
/// `image` of 36 frames, if any.
fn look(name: &str, sound: i16, image: Option<i16>) -> WeaponLook {
    WeaponLook {
        name: name.to_owned(),
        sheet: image.map(|image| Ok(effect_sheet(image, 36))),
        sound: Some(nova_sim::SoundId(sound)),
        ..WeaponLook::default()
    }
}

/// [`trafficked`] (ships of type 129, warships) where the player's
/// ship carries `weapons`, held by outfits 250 on, in that order; the
/// blaster, rockets, missiles and torch have looks, and `bööm` 128 shows `rlëD` 400's 3 frames at a frame
/// a step, sounding `snd ` 302. Ship 129 is a "Light Transport" with
/// picture 3001, and govt 140's code is "Fed.".
fn armed(avg: i16, weapons: &[WeaponId]) -> FakeCatalog {
    let mut catalog = trafficked(&[130], avg, 129, 3);
    catalog.dudes[0].1.govt = Some(GovtId(140));
    FakeCatalog {
        weapons: vec![
            gun(BLASTER, 0, -1, 0),
            gun(ROCKET, 0x0002, -1, -1),
            gun(MISSILE, 0x0002, -1, -1),
            gun(TORCH, 0x0002, -1, -1),
            gun(UNDER, 0, 0, -1),
            gun(OVER, 0, 0, -1),
            gun(UNSEEN, 0, -1, -1),
        ],
        hulls: vec![hull_of(128, weapons), hull_of(129, &[])],
        outfits: (250..)
            .zip(weapons)
            .map(|(id, &weapon)| holding(id, weapon))
            .collect(),
        looks: vec![
            (128, Ok(look("Blaster", 208, Some(3500)))),
            (140, Ok(look("Rocket", 209, Some(3501)))),
            (141, Ok(look("Missile", 210, None))),
            (142, Ok(look("Torch", 211, None))),
            (
                146,
                Ok(WeaponLook {
                    flags2: BEAM_UNDER_SHIPS,
                    beam_width: 1,
                    beam_color: 0x0000_00FF,
                    ..WeaponLook::default()
                }),
            ),
            (
                147,
                Ok(WeaponLook {
                    beam_width: 1,
                    beam_color: 0x0000_FF00,
                    ..WeaponLook::default()
                }),
            ),
        ],
        booms: vec![(
            128,
            Ok(BoomLook {
                sheet: Ok(effect_sheet(400, 3)),
                advance: 1.0,
                sound: Some(nova_sim::SoundId(302)),
            }),
        )],
        cards: vec![(
            129,
            TargetCard {
                subtitle: "Light Transport".to_owned(),
                picture: Some(3001),
            },
        )],
        codes: vec![(140, "Fed.".to_owned())],
        ..catalog
    }
}

/// `catalog`'s flight with its NPCs idling, placed by `draws`, after
/// the step that populates the system.
fn fighting(catalog: FakeCatalog, draws: &[u32]) -> View {
    let (_, chance) = scripted(draws);
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    view
}

/// NPC 0 at (100, -100) and NPC 1 at (-50, 0), nearer the player.
fn two_npcs() -> Vec<u32> {
    [placed(850, 650, 90), placed(700, 750, 0)].concat()
}

fn target_of(view: &View) -> Option<NpcId> {
    view.session().expect("flying").target().map(|npc| npc.id)
}

/// Taps R while holding Alt (Option).
fn alt_tap(view: &mut View, k: Key) {
    view.input(&key(Key::Alt, true));
    tap(view, k);
    view.input(&key(Key::Alt, false));
}

/// NPC 0 attacks the player; the rest idle.
#[derive(Debug)]
struct Threatening;

impl Behaviour for Threatening {
    fn decide(
        &self,
        npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        if npc.id == NpcId(0) {
            Goal::Attack(ShipRef::Player)
        } else {
            Goal::Idle
        }
    }
}

/// How many of the shots in flight are of `weapon`.
fn shots_of(view: &View, weapon: WeaponId) -> usize {
    let session = view.session().expect("flying");
    session
        .shots()
        .iter()
        .filter(|shot| shot.weapon.id == weapon)
        .count()
}

fn secondary_of(view: &View) -> Option<WeaponId> {
    view.session().expect("flying").secondary()
}

/// The first command matching `wanted`'s position in `list`.
fn first(list: &DrawList, wanted: impl Fn(&DrawCommand) -> bool) -> usize {
    list.iter()
        .position(wanted)
        .unwrap_or_else(|| panic!("not drawn: {list:?}"))
}

fn sprite_of(id: i16) -> impl Fn(&DrawCommand) -> bool {
    move |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == id)
}

/// Whether a command draws a `cïcn` in `ids`.
fn icon_in(ids: std::ops::RangeInclusive<i16>) -> impl Fn(&DrawCommand) -> bool {
    move |command| {
        matches!(
            command,
            DrawCommand::Picture { image, .. }
                if image.kind == crate::ImageKind::Icon && ids.contains(&image.id)
        )
    }
}

/// The bracket `cïcn`s drawn (10000-10023), in order.
fn bracket_icons(list: &DrawList) -> Vec<i16> {
    list.iter()
        .filter(|command| icon_in(10000..=10023)(command))
        .map(|command| match command {
            DrawCommand::Picture { image, .. } => image.id,
            _ => unreachable!("a picture"),
        })
        .collect()
}

/// [`armed`] where the NPC, straight above the player and facing it,
/// fires a blaster that kills at a hit, and the player's ship breaks
/// up in `death_delay` ticks, then explodes as `bööm` 128 (`rlëD` 400).
fn killed_in(death_delay: i16) -> FakeCatalog {
    let mut catalog = armed(1, &[]);
    catalog.weapons[0].mass_dmg = 1000;
    catalog.weapons[0].flags = 0x0020;
    catalog.weapons[0].explod_type = -1;
    catalog.hulls[0].death_delay = death_delay;
    catalog.hulls[1] = hull_of(129, &[BLASTER]);
    catalog
}

/// [`killed_in`] (the player's `DeathDelay` `death_delay`) where the
/// player starts with each of `outfits` (188, `ModType` 11, the escape
/// pod; 187, `ModType` 20, auto-eject), with records of its ship and
/// of the pod.
fn ejecting_in(death_delay: i16, outfits: &[i16]) -> FakeCatalog {
    let mut catalog = killed_in(death_delay);
    let fitted = |id, mod_type| OutfitRecord {
        mods: [(mod_type, 0), (0, 0), (0, 0), (0, 0)],
        ..holding(id, WeaponId(128))
    };
    catalog.outfits.extend([fitted(188, 11), fitted(187, 20)]);
    catalog.defaults = outfits.iter().map(|&id| (OutfitId(id), 1)).collect();
    catalog.ships.extend([ship_class(128), ship_class(895)]);
    catalog
}

/// `shïp` `id`'s record, of [`FIELDS`].
fn ship_class(id: i16) -> ShipRecord {
    ShipRecord {
        id: ShipId(id),
        name: format!("Ship {id}"),
        short_name: format!("Ship {id}"),
        long_name: String::new(),
        fields: FIELDS,
        defaults: Vec::new(),
        default_slots: [None; 8],
        strength: 0,
        cost: 900,
        tech_level: 1,
        buy_random: 100,
        hire_random: 0,
        require: 0,
        availability: nova_sim::Test::default(),
        appear_on: nova_sim::Test::default(),
        flags: 0,
        flags3: 0,
        disp_weight: 0,
        max_gun: 0,
        max_tur: 0,
        length: 0,
        crew: 0,
        inherent_ai: 1,
        comm_name: String::new(),
        inherent_govt: None,
        combat_govt: None,
        escort_type: -1,
        skill_var: 0,
        on_capture: nova_sim::Script::default(),
        on_purchase: nova_sim::Script::default(),
        on_retire: nova_sim::Script::default(),
    }
}

/// `catalog`'s view under its NPC's fire, holding each of `keys`.
fn ejecting_view(catalog: FakeCatalog, keys: &[Key]) -> View {
    let mut view = fighting(catalog, &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
    for &held in keys {
        view.input(&key(held, true));
    }
    view
}

/// Ticks `view` until the player is in the escape pod, or for 60
/// ticks: whether it got there, and whether it was seen breaking up with
/// its countdown begun. The frame of the hit that breaks it up shows it
/// dying with the timer unset; the next frame's eject check (@0x6f22c)
/// comes before its display sets the timer (@0x43e29).
fn until_ejected(view: &mut View) -> (bool, bool) {
    let mut broke_up = false;
    for _ in 0..60 {
        view.tick(TICK);
        match view.session().expect("flying").player_fate() {
            PlayerFate::Ejected { .. } => return (true, broke_up),
            PlayerFate::BreakingUp { ticks_left, .. } if ticks_left > 0 => broke_up = true,
            _ => {}
        }
    }
    (false, broke_up)
}

/// Steps `view` until the player's ship explodes, calling
/// `breaking_up` on each frame before that while it breaks up, and
/// returns with the view on the explosion's tick.
fn until_the_player_explodes(view: &mut View, breaking_up: impl Fn(&View)) {
    let mut broke_up = false;
    for _ in 0..60 {
        view.tick(TICK);
        match view.session().expect("flying").player_fate() {
            PlayerFate::BreakingUp { .. } => {
                breaking_up(view);
                broke_up = true;
            }
            PlayerFate::Exploded => {
                assert!(broke_up, "it broke up first");
                return;
            }
            PlayerFate::Flying | PlayerFate::Disabled => {}
            fate @ (PlayerFate::Holding { .. }
            | PlayerFate::Fading
            | PlayerFate::GameOver
            | PlayerFate::Ejected { .. }) => {
                panic!("past the explosion's tick unseen: {fate:?}")
            }
        }
    }
    panic!("the player's ship never exploded");
}

/// A flight under fire from [`killed_in`]`(30)`'s NPC, stepped until
/// the player's ship breaks up and its display has set the timer, on the
/// frame after the hit (@0x43e29): a breakup of 90 ticks, so it is still
/// breaking up for the test.
fn breaking_up() -> View {
    let mut view = fighting(killed_in(30), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
    for _ in 0..60 {
        view.tick(TICK);
        if matches!(fate_of(&view), PlayerFate::BreakingUp { ticks_left, .. } if ticks_left > 0) {
            return view;
        }
    }
    panic!("the player's ship never broke up");
}

fn fate_of(view: &View) -> PlayerFate {
    view.session().expect("flying").player_fate()
}

/// A flight under fire from [`killed_in`]`(1)`'s NPC, stepped to the
/// player's explosion and one tick past it.
fn exploded() -> View {
    let mut view = fighting(killed_in(1), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
    until_the_player_explodes(&mut view, |_| {});
    view.tick(TICK);
    view
}

/// [`armed`] with a blaster whose shots detonate after a tick.
fn detonating() -> FakeCatalog {
    let mut catalog = armed(0, &[BLASTER]);
    catalog.weapons[0].flags = 0x8000;
    catalog.weapons[0].count = 1;
    catalog
}

/// Fires one detonating shot, which explodes as its end of life comes,
/// the tick after its one move.
fn detonate(view: &mut View) {
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    view.tick(TICK);
    assert_eq!(view.effects().explosions().len(), 1);
}

/// What the target panel's status line shows this frame, if there is
/// a target.
fn status_shown(view: &View) -> Option<nova_sim::TargetStatus> {
    view.target_shown()
        .as_ref()
        .as_ref()
        .map(|panel| panel.status)
}

/// What the target panel's status line would show of the target now.
fn status_now(view: &View) -> Option<nova_sim::TargetStatus> {
    view.target_now().map(|panel| panel.status)
}

/// Disables every ship.
#[derive(Debug)]
struct Disabling;

impl DisableRule for Disabling {
    fn disabled(&self, _armor: nova_sim::Gauge, _hull: &nova_sim::HullSpec) -> bool {
        true
    }
}

const QUAD: WeaponId = WeaponId(133);
const IR: WeaponId = WeaponId(134);

/// [`armed`] with the player, over a planet at the centre, carrying a
/// point-defence turret (held by outfit 250), and its traffic a
/// missile it fires once, 5 pixels a tick.
fn defending() -> FakeCatalog {
    let mut catalog = armed(1, &[QUAD]);
    catalog.sites = vec![site(
        140,
        (0.0, 0.0),
        StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
    )];
    let sprite = |side| {
        Some(nova_sim::FrameSize {
            width: side,
            height: side,
        })
    };
    catalog.weapons.push(nova_sim::WeaponRecord {
        reload: 5,
        count: 12,
        mass_dmg: 1,
        energy_dmg: 4,
        frame: sprite(35),
        ..gun(QUAD, 0, 9, -1)
    });
    catalog.weapons.push(nova_sim::WeaponRecord {
        reload: 1000,
        count: 200,
        speed: 500,
        frame: sprite(18),
        ..gun(IR, 0, 1, -1)
    });
    catalog.hulls = vec![hull_of(128, &[QUAD]), hull_of(129, &[IR])];
    catalog
}
use nova_sim::HullSpec;
use nova_sim::reserves::Gauge;

/// Disables every ship of some strength: the NPCs (ship 129, of
/// strength 5), never the player (ship 128, of none).
#[derive(Debug)]
struct DisablesTheStrong;

impl DisableRule for DisablesTheStrong {
    fn disabled(&self, _armor: Gauge, hull: &HullSpec) -> bool {
        hull.strength > 0.0
    }
}

/// Never repels, always gives odds of 75 and always captures; a person
/// carries its `Credits`, and grants nothing.
#[derive(Debug)]
struct Sure;

impl BoardingRule for Sure {
    fn repels(&self, _booty: u16) -> bool {
        false
    }

    fn capture_odds(&self, _crew: &nova_sim::board::CaptureCrew, _chance: &mut dyn Chance) -> u8 {
        75
    }

    fn captures(&self, _odds: u8, _chance: &mut dyn Chance) -> bool {
        true
    }

    fn person_credits(&self, credits: i32, _chance: &mut dyn Chance) -> i64 {
        i64::from(credits)
    }

    fn grant(
        &self,
        _grant: &nova_sim::grant::PersonGrant,
        _stock: &[nova_sim::grant::GrantStock],
        _free_mass: i64,
        _grant_max: nova_sim::RuleSource,
        _chance: &mut dyn Chance,
    ) -> Option<nova_sim::grant::Granted> {
        None
    }
}

/// [`trafficked`] with its ship 129 (of strength 5, a crew of 3 and a
/// `Cost` of 150,000, and no speed, so it starts at rest) carrying
/// money (`Booty` 0x0040), and the player's ship 128 a crew of 10.
fn boardable() -> FakeCatalog {
    let mut catalog = trafficked(&[130], 1, 129, 1);
    catalog.dudes[0].1.booty = 0x0040;
    for record in &mut catalog.ships {
        if record.id == ShipId(129) {
            record.crew = 3;
            record.cost = 150_000;
        }
    }
    let mut player = catalog.ships[0].clone();
    player.id = ShipId(128);
    player.crew = 10;
    catalog.ships[0].fields.speed = 0;
    catalog.ships.push(player);
    catalog.hulls = vec![
        hull_of(128, &[]),
        nova_sim::HullRecord {
            strength: 5,
            ..hull_of(129, &[])
        },
    ];
    catalog
}

/// [`boardable`]'s flight with its one NPC placed at (`x` - 750,
/// `y` - 750) facing `heading`, disabled by [`DisablesTheStrong`] on
/// the first tick unless `intact`, and targeted.
fn beside(x: u32, y: u32, heading: u32, intact: bool) -> View {
    let (_, chance) = scripted(&placed(x, y, heading));
    let mut view = FlightView::new(boardable())
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    if !intact {
        view = view.with_disable_rule(Rc::new(DisablesTheStrong));
    }
    view.tick(TICK);
    tap(&mut view, TARGET_KEY);
    let session = view.session().expect("flying");
    assert_eq!(session.target().map(|npc| npc.id), Some(NpcId(0)));
    view
}

/// [`beside`], boarded by [`Sure`] and captured, awaiting its
/// assignment.
fn captured() -> View {
    captured_in(boardable())
}

/// [`captured`], from `catalog`, a [`boardable`] one.
fn captured_in(catalog: FakeCatalog) -> View {
    let (_, chance) = scripted(&placed(750, 750, 0));
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still))
        .with_disable_rule(Rc::new(DisablesTheStrong))
        .with_boarding_rule(Rc::new(Sure));
    view.tick(TICK);
    tap(&mut view, TARGET_KEY);
    tap(&mut view, BOARD_KEY);
    let boarding = view.take_boarding().expect("boarded").view;
    assert_eq!(boarding.odds, 75, "by the rule given");
    assert_eq!(view.plunder(Take::Capture), Taken::Captured);
    assert_eq!(view.message(), None, "the assignment dialog says it");
    view
}
use nova_sim::{Haggle, HailOptions, HailRefusal, Help};

/// [`boardable`]'s ship comm strings: "c<n>" for each, with "Channel
/// open." in every variant of the first group.
fn comm_strings() -> Vec<String> {
    (1..=200)
        .map(|n| {
            if n <= 5 {
                "Channel open.".to_owned()
            } else {
                format!("c{n}")
            }
        })
        .collect()
}

/// [`beside`] with the ship comm strings.
fn hailable(intact: bool) -> View {
    let mut view = beside(750, 750, 0, intact);
    view.catalog.strings = vec![(3000, comm_strings())];
    view
}

/// An option that asks a price, saying nothing.
#[derive(Debug)]
struct Sell;

impl nova_sim::HailOption for Sell {
    fn label(&self) -> String {
        "Sell".to_owned()
    }

    fn applies(&self, _hail: &nova_sim::Hail) -> bool {
        true
    }

    fn press(&self, _hail: &nova_sim::Hail, _chance: &mut dyn Chance) -> nova_sim::Answer {
        let line = nova_sim::Reply::Line { list: 1, index: 1 };
        nova_sim::Answer {
            ask: Some(nova_sim::Ask {
                paid: (line, None),
                declined: (line, None),
                short: line,
            }),
            ..nova_sim::Answer::default()
        }
    }
}

/// An option the ship answers by flying over to refuel the player.
#[derive(Debug)]
struct Refuel;

impl nova_sim::HailOption for Refuel {
    fn label(&self) -> String {
        "Refuel".to_owned()
    }

    fn applies(&self, _hail: &nova_sim::Hail) -> bool {
        true
    }

    fn press(&self, _hail: &nova_sim::Hail, _chance: &mut dyn Chance) -> nova_sim::Answer {
        nova_sim::Answer {
            deed: Some(nova_sim::Deed::Help(Help::Refuel)),
            ..nova_sim::Answer::default()
        }
    }
}
use super::super::escorts::{ABSENT_ROW, MENU_AT, MENU_HEIGHT, MENU_WIDTH};
use nova_sim::{EscortClass, EscortGroup, EscortOrder};

/// `pilot`, saved and read back with a fleet of `ships`, each full.
fn with_escorts(pilot: &Pilot, ships: &[i16]) -> Pilot {
    with_fleet(pilot, ships, false)
}

/// `pilot`, saved and read back with a fleet of `ships`, each full,
/// and fighters out of a bay when `carried`.
fn with_fleet(pilot: &Pilot, ships: &[i16], carried: bool) -> Pilot {
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(pilot)).expect("JSON");
    let gauge = |max: f32| serde_json::json!({"now": max, "max": max});
    save["escorts"] = ships
        .iter()
        .map(|ship| {
            serde_json::json!({
                "ship": ship,
                "reserves": {"shield": gauge(40.0), "armor": gauge(60.0), "fuel": gauge(250.0)},
                "order": null,
                "carried": carried,
                "wage": null,
                "person": null
            })
        })
        .collect();
    nova_sim::save::decode(&save.to_string()).expect("a pilot")
}

/// [`boardable`] with ship 130 a fighter (`EscortType` 0) holding
/// no fuel, and two of them out of the player's bays; its traffic
/// ship (NPC 0) placed 100 above the player. A tick has placed the
/// fighters (NPCs 1 and 2).
fn fighters_out() -> View {
    fighters_out_of(boardable())
}

/// [`fighters_out`] from `catalog`, a [`boardable`] one.
fn fighters_out_of(mut catalog: FakeCatalog) -> View {
    for record in &mut catalog.ships {
        if record.id == ShipId(130) {
            record.escort_type = 0;
            record.fields.fuel = 0;
        }
    }
    let pilot = with_fleet(
        &Pilot::new(&catalog, "Carrier").expect("starts"),
        &[130, 130],
        true,
    );
    let (_, chance) = scripted(&placed(750, 650, 0));
    let mut view = FlightView::with_pilot(catalog, pilot)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    assert!(view.session().expect("flying").is_escort(NpcId(2)));
    view
}

/// Taps `key` with Alt (Option) held.
fn option(view: &mut View, key_: Key) {
    view.input(&key(Key::Alt, true));
    tap(view, key_);
    view.input(&key(Key::Alt, false));
}

const HELLO: &str = "Message Buoy: hello.";

/// [`boardable`] with ship 130, a warship escort (`EscortType` 2,
/// `InherentAI` 3), whose sheet cannot be read, and the pilot's fleet
/// one of them; its traffic ship (NPC 0) placed 100 above the player;
/// all idle. A tick has placed the escort (NPC 1).
fn escorted() -> View {
    escorted_with_traffic(750, 650)
}

/// [`escorted`] with its traffic ship (NPC 0) placed as
/// [`placed`]`(x, y, 0)` places it.
fn escorted_with_traffic(x: u32, y: u32) -> View {
    let mut catalog = boardable();
    for record in &mut catalog.ships {
        if record.id == ShipId(130) {
            record.escort_type = 2;
            record.inherent_ai = 3;
        }
    }
    let pilot = with_escorts(&Pilot::new(&catalog, "Escorted").expect("starts"), &[130]);
    let (_, chance) = scripted(&placed(x, y, 0));
    let mut view = FlightView::with_pilot(catalog, pilot)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    let session = view.session().expect("flying");
    assert!(session.is_escort(NpcId(1)), "{:?}", session.npcs());
    view
}

fn orders(view: &View) -> Vec<Option<EscortOrder>> {
    view.pilot()
        .expect("flying")
        .escorts()
        .iter()
        .map(|escort| escort.order)
        .collect()
}

/// The interface sounds `view` holds, taken.
fn ui_sounds(view: &mut View) -> Vec<UiSound> {
    view.take_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Ui(sound) => Some(sound),
            _ => None,
        })
        .collect()
}
use nova_sim::hire::DEFECTED_ONE;
use nova_sim::{ControlBits, HireRefusal, HireTerms};

/// [`shipbuying`] with its stellar a bar too, and ship 129 (`Cost`
/// 900) for hire half the days.
fn hiring() -> FakeCatalog {
    let mut catalog = shipbuying();
    catalog.sites[0].flags |= StellarFlags::BAR;
    for record in &mut catalog.ships {
        if record.id == ShipId(129) {
            record.hire_random = 50;
        }
    }
    catalog
}
use nova_sim::MissionTextKind;
use nova_sim::catalog::{MissionRecord, MissionShip, MissionTexts};
use nova_sim::control::{Script as SetScript, Test};

/// Mission `id`, offered at the mission computer of any stellar on
/// every visit, to anyone, going nowhere.
fn errand(id: i16) -> MissionRecord {
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
        on_accept: SetScript::default(),
        on_refuse: SetScript::default(),
        on_success: SetScript::default(),
        on_failure: SetScript::default(),
        on_abort: SetScript::default(),
        on_ship_done: SetScript::default(),
        can_abort: 1,
        comp_govt: None,
        comp_reward: 0,
        date_post_inc: 0,
        ships: MissionShip::NONE,
        aux: nova_sim::catalog::AuxShip::NONE,
    }
}

/// Holds Command (and Alt with `option`), taps `k`, and lets go.
fn command_tap(view: &mut View, k: Key, option: bool) {
    view.input(&key(Key::Command, true));
    if option {
        view.input(&key(Key::Alt, true));
    }
    tap(view, k);
    if option {
        view.input(&key(Key::Alt, false));
    }
    view.input(&key(Key::Command, false));
}

fn cargo_aboard(view: &View) -> i64 {
    view.pilot().expect("flying").total_cargo()
}

/// Inspects the player until its flag is set; then it has looked the
/// player over, and goes on to its goal.
#[derive(Debug)]
struct InspectingUntil(Rc<std::cell::Cell<bool>>, Goal);

impl Behaviour for InspectingUntil {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        if self.0.get() {
            self.1
        } else {
            Goal::Inspect(ShipRef::Player)
        }
    }
}

/// Control bits where nothing holds.
#[derive(Debug)]
struct Nothing;

impl ControlBits for Nothing {
    fn allows(&self, _test: &nova_sim::TestExpr, _pilot: &dyn nova_sim::PilotFacts) -> bool {
        false
    }
}
/// [`trafficked`] with no traffic of its own but a Person slot at
/// 100 %, "Ace" (`përs` 600, subtitle "Top Gun"), flying ship 129 and
/// saying `STR#` 7101 #1, "<OSN>: Prepare to die, <PN>!", now and
/// then.
fn peopled() -> FakeCatalog {
    let mut catalog = trafficked(&[130], 0, 129, 3);
    catalog.traffic[0].1.persons[0] = (Some(nova_sim::PersonId(600)), 100);
    catalog.persons = vec![nova_sim::PersonRecord {
        id: nova_sim::PersonId(600),
        name: "Ace".to_owned(),
        link_syst: -2,
        govt: None,
        ai_type: 3,
        aggress: 2,
        coward: 0,
        ship: Some(ShipId(129)),
        weapons: Vec::new(),
        credits: 0,
        shield_mod: 0,
        hail_pict: None,
        comm_quote: -1,
        hail_quote: 1,
        link_mission: None,
        flags: 0,
        active_on: nova_sim::Test::default(),
        subtitle: "Top Gun".to_owned(),
        flags2: 0,
        grant_class: 0,
        grant_count: 0,
        grant_prob: 0,
        mission_ship: None,
    }];
    catalog.strings = vec![(7101, vec!["<OSN>: Prepare to die, <PN>!".to_owned()])];
    catalog
}

/// Runs set expression `text` on `view`'s session.
fn run_set(view: &mut View, text: &str) {
    let expr = nova_sim::SetExpr::parse(text).expect("parses");
    let session = view.session.as_mut().expect("flying");
    session.run_set(&expr, &mut NeverFires);
}

/// `view` with `STR#` 25048 holding one message.
fn told(view: View) -> View {
    let strings = FakeCatalog {
        strings: vec![(25048, vec!["Off you go, <PSN>.".to_owned()])],
        ..catalog()
    };
    view.with_strings(Rc::new(strings))
}
/// The ships whose sheets `view` has asked for.
fn sheets(view: &View) -> Vec<ShipId> {
    view.catalog().sheets_asked.borrow().clone()
}

fn systems_read(view: &View) -> Vec<SystemId> {
    view.catalog().systems_read.borrow().clone()
}

/// Settles `view`'s session by itself, not through the view.
fn settle_session(view: &mut View) {
    let session = view.session.as_mut().expect("flying");
    let _ = session.settle_script(&view.catalog, &mut NeverFires);
}

/// The background of the status bar `view` shows.
fn bar_shown(view: &View) -> Option<i16> {
    view.status_bar().ok().map(|bar| bar.layout.status_bkgnd)
}
/// Changes `view`'s galaxy by itself, not through the view.
fn change_galaxy(view: &mut View, change: impl FnOnce(&mut nova_sim::story::GalaxyOverlay)) {
    let session = view.session.as_mut().expect("flying");
    assert!(session.change_galaxy(&view.catalog, change));
}

/// Hides system `id`.
fn hiding(id: i16) -> impl FnOnce(&mut nova_sim::story::GalaxyOverlay) {
    move |overlay| {
        overlay.set_visible(SystemId(id), Some(false));
    }
}
/// Stellar `id` with `Strength` 100 and `DeadTime` 5, so that `Y`
/// destroys it, and `Flags2` `flags2`.
fn life(id: i16, flags2: u16) -> nova_sim::StellarLife {
    nova_sim::StellarLife {
        id: StellarId(id),
        strength: 100,
        dead_time: 5,
        flags2,
        explosion: None,
        on_destroy: nova_sim::Script::default(),
        on_regen: nova_sim::Script::default(),
    }
}

/// A flight in Sol whose Earth (128, `DeadType` sheet 2128), Moon
/// (129, no different graphic) and Proxima (140, `DeadType` sheet
/// 2140) can be destroyed.
fn destructible() -> View {
    FlightView::new(FakeCatalog {
        lives: vec![life(128, 0), life(129, 0), life(140, 0)],
        ..catalog()
    })
}

/// Whether stellar `id` is drawn with its `Type` sheet (1000 + `id`)
/// and whether with its `DeadType` sheet (2000 + `id`), at any of the
/// first 4 frames.
fn sheets_drawn(view: &View, id: i16) -> (bool, bool) {
    let images: Vec<ImageKey> = sprites(&drawn(view))
        .into_iter()
        .map(|(image, _)| image)
        .collect();
    let sheet = |image_id| (0..4).any(|f| images.contains(&ImageKey::sprite(image_id, f)));
    (sheet(1000 + id), sheet(2000 + id))
}

const TYPE: (bool, bool) = (true, false);
/// Whether graphic type 0's sheet is drawn, at any of the first 4
/// frames: Moon (129) is the only stellar that can draw it.
fn type_zero_drawn(view: &View) -> bool {
    sprites(&drawn(view))
        .into_iter()
        .any(|(image, _)| (0..4).any(|f| image == ImageKey::sprite(TYPE_ZERO, f)))
}

/// Moon (129) is drawn with its `Type` sheet, or graphic type 0's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Moon {
    Type,
    TypeZero,
}

fn moon(view: &View) -> Moon {
    match (sheets_drawn(view, 129), type_zero_drawn(view)) {
        (TYPE, false) => Moon::Type,
        ((false, false), true) => Moon::TypeZero,
        other => panic!("the Moon drawn as {other:?}"),
    }
}

/// [`destructible`] with `DeadTypeFallback` read as `reading`.
fn reading(reading: RuleSource) -> View {
    destructible().with_rules(Rulebook::default().with_override(RuleKey::DeadTypeFallback, reading))
}

/// A pilot loaded with the Moon destroyed, flown by `rules`.
fn loaded(rules: Rulebook) -> View {
    let catalog = FakeCatalog {
        lives: vec![life(129, 0)],
        ..catalog()
    };
    let mut session = Session::start(&catalog).expect("starts");
    let destroy = nova_sim::SetExpr::parse("Y129").expect("parses");
    session.run_set(&destroy, &mut NeverFires);
    FlightView::with_pilot(catalog, session.pilot().clone()).with_rules(rules)
}
/// Grants a person's `GrantCount` of the first outfit it may.
#[derive(Debug)]
struct Grants;

impl BoardingRule for Grants {
    fn repels(&self, _booty: u16) -> bool {
        false
    }

    fn capture_odds(&self, _crew: &nova_sim::board::CaptureCrew, _chance: &mut dyn Chance) -> u8 {
        75
    }

    fn captures(&self, _odds: u8, _chance: &mut dyn Chance) -> bool {
        true
    }

    fn person_credits(&self, credits: i32, _chance: &mut dyn Chance) -> i64 {
        i64::from(credits)
    }

    fn grant(
        &self,
        grant: &nova_sim::grant::PersonGrant,
        stock: &[nova_sim::grant::GrantStock],
        _free_mass: i64,
        _grant_max: nova_sim::RuleSource,
        _chance: &mut dyn Chance,
    ) -> Option<nova_sim::grant::Granted> {
        let first = nova_sim::grant::candidates(grant.class, stock)
            .first()
            .copied()?;
        Some(nova_sim::grant::Granted {
            outfit: first.outfit,
            count: grant.count,
        })
    }
}

/// [`peopled`] with Ace a derelict (government 140), so disabled,
/// granting `count` spare parts (`oütf` 200, class 7), of no hail
/// quote; its ship 129 has a crew of 3 and a board reach.
fn granting(count: i16) -> FakeCatalog {
    let mut catalog = peopled();
    let ace = &mut catalog.persons[0];
    ace.govt = Some(GovtId(140));
    ace.hail_quote = -1;
    ace.grant_class = 7;
    ace.grant_prob = 100;
    ace.grant_count = count;
    catalog.govts = vec![nova_sim::GovtRecord {
        id: GovtId(140),
        name: String::new(),
        flags: 0x0800,
        flags2: 0,
        crime_tol: 0,
        initial_rec: 0,
        penalties: nova_sim::Penalties::default(),
        max_odds: 100,
        classes: [-1; 4],
        allies: [-1; 4],
        enemies: [-1; 4],
        comm_name: String::new(),
        medium_name: String::new(),
        scan_mask: 0,
        scan_fine: 0,
        news_pic: -1,
        require: 0,
        inh_jam: [0; 4],
        skill_mult: 0,
    }];
    for record in &mut catalog.ships {
        record.crew = 3;
    }
    catalog.hulls = vec![hull_of(129, &[])];
    catalog.outfits = vec![OutfitRecord {
        id: OutfitId(200),
        name: "Spare Part".to_owned(),
        short_name: "Spare Part".to_owned(),
        disp_weight: 0,
        mass: 0,
        tech_level: 1,
        max: 10,
        flags: 0,
        cost: 100,
        mods: [(0, 0); 4],
        contribute: 0,
        require: 0,
        require_govt: -1,
        buy_random: 100,
        availability: nova_sim::Test::default(),
        item_class: 7,
        lc_name: "spare part".to_owned(),
        lc_plural: "spare parts".to_owned(),
        scan_mask: 0,
        on_purchase: nova_sim::Script::default(),
        on_sell: nova_sim::Script::default(),
    }];
    catalog
}

/// A flight under fire from an NPC whose every hit disables, stepped
/// until the player's ship is disabled.
fn disabled_player() -> View {
    let mut catalog = armed(1, &[]);
    catalog.hulls[1] = hull_of(129, &[BLASTER]);
    let mut view = fighting(catalog, &placed(750, 650, 180))
        .with_behaviour(Rc::new(Firing))
        .with_disable_rule(Rc::new(Disabling));
    for _ in 0..40 {
        if fate_of(&view) == PlayerFate::Disabled {
            return view;
        }
        view.tick(TICK);
    }
    panic!("the player's ship was never disabled");
}

/// Where `view` draws the system point `(x, y)`.
fn on_screen(view: &View, (x, y): (f32, f32)) -> Point {
    view.camera().world_to_screen(at(x, y))
}

fn nav_mode(view: &View) -> NavMode {
    view.session().expect("flying").nav_mode()
}

/// The four corners from `cïcn` `first` round `centre`, half-size
/// `half`: top-left, top-right, bottom-right, bottom-left.
fn corners_at(first: i16, centre: Point, half: f32) -> Vec<(i16, Point)> {
    let (near, far) = (-half - 16.0, half);
    vec![
        (first, at(centre.x + near, centre.y + near)),
        (first + 1, at(centre.x + far, centre.y + near)),
        (first + 2, at(centre.x + far, centre.y + far)),
        (first + 3, at(centre.x + near, centre.y + far)),
    ]
}

fn nav_target(view: &View) -> Option<StellarId> {
    view.session().expect("flying").nav_target()
}

/// The texts drawn in the HUD's nav area, the stock bar's at (830, 0).
fn nav(view: &View) -> Vec<String> {
    let area = layout().nav.offset(at(830.0, 0.0));
    drawn(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, origin, .. } if area.contains(*origin) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The texts drawn in the cargo area, at the stock bar's (830, 0),
/// with where each starts.
fn cargo(view: &View) -> Vec<(String, Point)> {
    let area = layout().cargo.offset(at(830.0, 0.0));
    drawn(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, origin, .. } if area.contains(*origin) => {
                Some((text.clone(), *origin))
            }
            _ => None,
        })
        .collect()
}

/// The engine tick `view` has reached.
fn now_tick(view: &View) -> u64 {
    u64::try_from(crate::time::ticks(view.elapsed)).expect("in range")
}
const HYPER_SELECT: Key = Key::Char('\\');

fn course(view: &View) -> Vec<SystemId> {
    view.session().expect("flying").course().to_vec()
}
use crate::flight::jump::{ARRIVAL_FLASH_FOR, FADE_IN_FOR};
use nova_sim::GateKind;

/// Stellar `id` at (`x`, `y`), with `flags2`, `links` and
/// `exit_angle`.
fn gate_site(id: i16, (x, y): (f32, f32), flags2: u16, links: &[i16], exit_angle: i16) -> GateSite {
    let mut slots = [None; 8];
    for (slot, &link) in slots.iter_mut().zip(links) {
        *slot = Some(StellarId(link));
    }
    GateSite {
        id: StellarId(id),
        position: Vec2::new(x, y),
        flags2,
        links: slots,
        exit_angle,
    }
}

/// Sol holds hypergate 300 (a station) at its centre, over the ship,
/// linked to 310 in Alpha Centauri at (100, 200), heading ships out on
/// 90°.
fn gated() -> FakeCatalog {
    FakeCatalog {
        sites: vec![LandingSite {
            flags2: 0x1200,
            ..site(
                300,
                (0.0, 0.0),
                StellarFlags::CAN_LAND | StellarFlags::STATION,
            )
        }],
        gates: vec![
            gate_site(300, (0.0, 0.0), 0x1200, &[310], 120),
            gate_site(310, (100.0, 200.0), 0x1000, &[300], 90),
        ],
        gate_listers: vec![(SystemId(131), StellarId(310))],
        ..catalog()
    }
}

/// Sol holds unlinked wormhole 400 at its centre, over the ship, and
/// Alpha Centauri unlinked wormhole 410 at (-300, 40).
fn holed() -> FakeCatalog {
    FakeCatalog {
        sites: vec![LandingSite {
            flags2: 0x2200,
            ..site(400, (0.0, 0.0), StellarFlags::CAN_LAND)
        }],
        gates: vec![
            gate_site(400, (0.0, 0.0), 0x2200, &[], 120),
            gate_site(410, (-300.0, 40.0), 0x2000, &[], 0),
        ],
        gate_listers: vec![(SystemId(131), StellarId(410))],
        ..catalog()
    }
}

fn system_of(view: &View) -> SystemId {
    view.session().expect("flying").system()
}

/// The course map's sounds `view` holds, taken with the rest.
fn map_sounds(view: &mut View) -> Vec<UiSound> {
    ui_sounds(view)
        .into_iter()
        .filter(|sound| matches!(sound, UiSound::MapOpened | UiSound::MapClosed))
        .collect()
}

/// Where on screen `view` draws NPC `id`.
fn npc_on_screen(view: &View, id: NpcId) -> Point {
    let session = view.session().expect("flying");
    let npc = session
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("in the system");
    view.camera().world_to_screen(view.shown_npc(npc).0)
}

/// The index in `list` of the first sprite centred on `at`, or of a
/// crossed box's diagonal centred on it.
fn drawn_at(list: &DrawList, at: Point) -> usize {
    let near = |p: Point| (p.x - at.x).abs() < 0.01 && (p.y - at.y).abs() < 0.01;
    list.iter()
        .position(|command| match *command {
            DrawCommand::Sprite { center, .. } => near(center),
            DrawCommand::Line { from, to, .. } => near(Point::new(
                f32::midpoint(from.x, to.x),
                f32::midpoint(from.y, to.y),
            )),
            _ => false,
        })
        .unwrap_or_else(|| panic!("nothing drawn at {at:?}"))
}
/// IFF, a density scanner and anti-interference 30 as outfits 300-302.
fn radar_outfits() -> Vec<OutfitRecord> {
    let outfit = |id: i16, mod_type: i16, mod_val: i16| {
        let mut mods = [(0, 0); 4];
        mods[1] = (mod_type, mod_val);
        OutfitRecord {
            id: OutfitId(id),
            name: format!("Outfit {id}"),
            short_name: String::new(),
            disp_weight: 0,
            mass: 0,
            tech_level: 1,
            max: 10,
            flags: 0,
            cost: 1,
            mods,
            contribute: 0,
            require: 0,
            require_govt: -1,
            buy_random: 100,
            availability: nova_sim::Test::default(),
            on_purchase: nova_sim::Script::default(),
            on_sell: nova_sim::Script::default(),
            item_class: 0,
            lc_name: String::new(),
            lc_plural: String::new(),
            scan_mask: 0,
        }
    };
    vec![
        outfit(300, nova_sim::iff::MOD_IFF, 0),
        outfit(301, nova_sim::iff::MOD_DENSITY_SCANNER, 0),
        outfit(302, nova_sim::iff::MOD_ANTI_INTERFERENCE, 30),
    ]
}

/// Whether the radar shown has IFF's black fill.
fn radar_filled(view: &View) -> bool {
    radar_marks(view).first()
        == Some(&RadarMark::Fill {
            rect: STOCK_RADAR,
            color: RadarColor::Black,
        })
}

/// Thrusts straight up until the ship is more than 2700 from the
/// centre, then lets go of Up.
fn fly_far(view: &mut View) {
    view.input(&key(Key::Up, true));
    for _ in 0..5000 {
        if player(view).position.length() > 2700.0 {
            break;
        }
        view.tick(TICK);
    }
    view.input(&key(Key::Up, false));
    assert!(player(view).position.length() > 2700.0);
}
/// Quiet sound preferences: effects at level 1.
const QUIET: SoundPrefs = SoundPrefs {
    sound: true,
    music: true,
    effects_level: 1,
    music_level: 7,
};

/// Two NPCs, NPC 0 turning on the player at the first tick after the
/// flight's first (frame 0), sound `prefs`: the Red Alert's check at
/// frame 60 is the 60th tick from here.
fn threatened(prefs: SoundPrefs) -> View {
    fighting(armed(2, &[]), &two_npcs())
        .with_behaviour(Rc::new(Threatening))
        .with_sound_prefs(prefs)
}
use crate::flight::floating;
use crate::galaxy::map::{
    BRIGHT_GREEN, CLEARED, CYAN, DARK_GREY, DENIED, DIM_GREEN, HOSTILE, LESS_DARK_GREY, LIGHT_GREY,
};
use crate::geometry::Bounds;
use nova_sim::mission::screens::mac_ticks;

/// [`catalog`] with Proxima Near ([`FakeCatalog::near`]).
fn near() -> View {
    FlightView::new(FakeCatalog {
        near: true,
        ..catalog()
    })
}

/// The floating map's border lines in `list`: those in the
/// `FloatingMap` colour on the frame's edges, whatever their alpha.
fn floating_border(list: &DrawList) -> Vec<DrawCommand> {
    let border = MENU_COLORS.border;
    list.iter()
        .filter(|command| {
            matches!(command, DrawCommand::Line { from, color, .. }
                if (color.r, color.g, color.b) == (border.r, border.g, border.b)
                    && from.x >= 558.0)
        })
        .cloned()
        .collect()
}

/// The alpha the floating map's border is drawn at, if it is drawn.
fn floating_alpha(view: &View) -> Option<u8> {
    floating_border(&drawn(view))
        .first()
        .map(|command| match command {
            DrawCommand::Line { color, .. } => color.a,
            _ => unreachable!(),
        })
}

fn sims(view: &mut View) -> Vec<SimSound> {
    view.take_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Sim(sound) => Some(sound),
            _ => None,
        })
        .collect()
}
/// The times `view` raised `sound` since its sounds were last taken.
fn count(view: &mut View, sound: SimSound) -> usize {
    sims(view).into_iter().filter(|&s| s == sound).count()
}

/// Sets the player's condition in `view` to `to`.
fn condition(view: &mut View, to: Condition) {
    let Ok(session) = &mut view.session else {
        panic!("flying");
    };
    nova_sim::fixture::set_player_condition(session, to);
}
